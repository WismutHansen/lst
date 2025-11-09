use crate::config::Config;
use crate::database::LocalDb;
use anyhow::{Context, Result};
use automerge::{Automerge, Change};
use base64::engine::general_purpose;
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use lst_core::config::State;
use lst_core::crypto;
use lst_core::sync::{
    canonical_path_with_id, canonicalize_doc_path, extract_automerge_content, update_automerge_doc,
    CanonicalDocPath, DocumentKind,
};
use notify::Event;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::{timeout, Instant};
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, http::header::AUTHORIZATION};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

pub struct SyncManager {
    config: Config,
    state: State,
    client: Option<reqwest::Client>,
    db: LocalDb,
    encryption_key: [u8; 32],
    last_sync: Instant,
    pending_changes: HashMap<String, Vec<Vec<u8>>>,
    initial_sync_done: bool,
    /// Tracks files recently created by sync to avoid processing them as local changes
    recently_synced_files: HashSet<std::path::PathBuf>,
}

impl SyncManager {
    pub async fn new(config: Config) -> Result<Self> {
        let client = if config
            .sync
            .as_ref()
            .and_then(|s| s.server_url.as_ref())
            .is_some()
        {
            Some(reqwest::Client::new())
        } else {
            None
        };

        // Ensure CRDT storage directory exists
        if let Some(ref storage) = config.storage {
            tokio::fs::create_dir_all(&storage.crdt_dir)
                .await
                .with_context(|| {
                    format!(
                        "Failed to create CRDT directory: {}",
                        storage.crdt_dir.display()
                    )
                })?;
        }

        let state = State::load()?;
        let db_path = state
            .get_sync_database_path()
            .expect("sync database path must be set in state");

        let db = LocalDb::new(db_path)?;

        let key_path = config
            .sync
            .as_ref()
            .and_then(|s| s.encryption_key_ref.as_ref())
            .expect("encryption_key_ref must be set in sync config");

        // Use the key file that was saved during 'lst auth login'
        // The login command already derived the secure key using credentials and saved it
        let (email, auth_token) = state.get_credentials();
        let encryption_key = if let (Some(_email), Some(_auth_token)) = (email, auth_token) {
            // Try to load the key that was saved during login
            let resolved_key_path = crypto::resolve_key_path(key_path)?;
            match crypto::load_key(&resolved_key_path) {
                Ok(key) => {
                    println!(
                        "DEBUG: Sync daemon using encryption key from file (derived during login)"
                    );
                    key
                }
                Err(e) => {
                    eprintln!("ERROR: Failed to load encryption key: {}", e);
                    eprintln!("       Please run 'lst auth login <email> <auth-token>' to derive and save the key");
                    return Err(e);
                }
            }
        } else {
            eprintln!("ERROR: No authentication credentials found");
            eprintln!("       Please run 'lst auth register <email>' followed by 'lst auth login <email> <auth-token>'");
            return Err(anyhow::anyhow!(
                "Authentication required: no stored credentials found"
            ));
        };

        Ok(Self {
            config,
            state,
            client,
            db,
            encryption_key,
            last_sync: Instant::now(),
            pending_changes: HashMap::new(),
            initial_sync_done: false,
            recently_synced_files: HashSet::new(),
        })
    }

    pub async fn handle_file_event(&mut self, event: Event) -> Result<()> {
        for original_path in event.paths {
            let (canonical, derived_doc_id) = match canonical_path_with_id(&original_path) {
                Ok(result) => result,
                Err(e) => {
                    eprintln!(
                        "DEBUG: Skipping path {}: failed to canonicalize ({})",
                        original_path.display(),
                        e
                    );
                    continue;
                }
            };

            // Skip files we just created via sync
            if self.recently_synced_files.contains(&canonical.full_path) {
                println!(
                    "DEBUG: Skipping recently synced file: {}",
                    canonical.full_path.display()
                );
                self.recently_synced_files.remove(&canonical.full_path);
                continue;
            }

            let path_str = canonical.full_path.to_string_lossy();

            // Skip cloud storage directories and files
            if path_str.contains("OneDrive")
                || path_str.contains("GoogleDrive")
                || path_str.contains("Dropbox")
                || path_str.contains("iCloud")
                || path_str.contains(".cloud")
            {
                println!(
                    "DEBUG: Skipping cloud storage path: {}",
                    canonical.full_path.display()
                );
                continue;
            }

            // Skip directories - only process files
            if canonical.full_path.is_dir() {
                println!(
                    "DEBUG: Skipping directory: {}",
                    canonical.full_path.display()
                );
                continue;
            }

            if let Some(filename) = canonical.full_path.file_name() {
                if let Some(filename_str) = filename.to_str() {
                    if filename_str.starts_with('.')
                        || filename_str.ends_with(".tmp")
                        || filename_str.ends_with(".swp")
                    {
                        continue;
                    }
                }
            }

            // Skip files we just created via sync
            let file_path_str = canonical.full_path.to_string_lossy().to_string();

            // Prefer existing mapping by file_path (absolute)
            let existing_doc_id = self
                .db
                .get_doc_id_by_file_path(&file_path_str)
                .ok()
                .flatten()
                .or_else(|| {
                    self.db
                        .get_doc_id_by_file_path(&canonical.relative_path)
                        .ok()
                        .flatten()
                });

            let doc_id = if let Some(id) = existing_doc_id {
                id
            } else {
                derived_doc_id.clone()
            };

            println!(
                "DEBUG: Processing file {} -> doc_id: {}",
                canonical.full_path.display(),
                doc_id
            );

            if matches!(event.kind, notify::EventKind::Remove(_)) {
                self.db.delete_document(&doc_id)?;
                self.pending_changes.remove(&doc_id);
                continue;
            }

            let data = tokio::fs::read(&canonical.full_path)
                .await
                .unwrap_or_default();

            if let Some(sync) = &self.config.sync {
                if data.len() as u64 > sync.max_file_size {
                    continue;
                }
            }

            let mut hasher = Sha256::new();
            hasher.update(&data);
            let hash = hex::encode(hasher.finalize());

            let doc_kind = canonical.kind;
            let doc_type = doc_kind.as_str();

            let owner = self
                .state
                .device
                .device_id
                .as_ref()
                .map(String::as_str)
                .unwrap_or("local");

            let new_content = String::from_utf8_lossy(&data);

            if let Some((
                _,
                existing_doc_type,
                last_hash,
                state,
                existing_owner,
                writers,
                readers,
            )) = self.db.get_document(&doc_id)?
            {
                let existing_kind = DocumentKind::from_str(&existing_doc_type);
                if last_hash == hash {
                    continue;
                }

                let mut doc = Automerge::load(&state)?;
                let old_heads = doc.get_heads().into_iter().collect::<Vec<_>>();

                update_automerge_doc(&mut doc, existing_kind, &new_content)?;

                let new_state = doc.save();

                println!(
                    "DEBUG: Updating existing document {} with {} bytes",
                    doc_id,
                    new_state.len()
                );
                self.db.upsert_document(
                    &doc_id,
                    &file_path_str,
                    &existing_doc_type,
                    &hash,
                    &new_state,
                    &existing_owner,
                    writers.as_deref(),
                    readers.as_deref(),
                )?;

                let new_doc = Automerge::load(&new_state)?;
                let changes = new_doc
                    .get_changes(&old_heads)
                    .into_iter()
                    .map(|c| c.raw_bytes().to_vec())
                    .collect::<Vec<_>>();

                self.pending_changes
                    .entry(doc_id)
                    .or_insert_with(Vec::new)
                    .extend(changes);
            } else {
                let mut doc = Automerge::new();
                let old_heads = doc.get_heads().into_iter().collect::<Vec<_>>();

                update_automerge_doc(&mut doc, doc_kind, &new_content)?;

                let new_state = doc.save();

                println!(
                    "DEBUG: Creating new document {} with {} bytes",
                    doc_id,
                    new_state.len()
                );
                self.db.upsert_document(
                    &doc_id,
                    &file_path_str,
                    doc_type,
                    &hash,
                    &new_state,
                    owner,
                    None,
                    None,
                )?;

                let new_doc = Automerge::load(&new_state)?;
                let changes = new_doc
                    .get_changes(&old_heads)
                    .into_iter()
                    .map(|c| c.raw_bytes().to_vec())
                    .collect::<Vec<_>>();

                self.pending_changes
                    .entry(doc_id)
                    .or_insert_with(Vec::new)
                    .extend(changes);
            }
        }

        Ok(())
    }

    /// Apply remote Automerge changes to the local document and file
    pub async fn apply_remote_changes(
        &mut self,
        doc_id: &str,
        changes: Vec<Vec<u8>>,
    ) -> Result<()> {
        if let Some((file_path, doc_type, _last_hash, state, owner, writers, readers)) =
            self.db.get_document(doc_id)?
        {
            let doc_kind = DocumentKind::from_str(&doc_type);
            let canonical =
                canonicalize_doc_path(Path::new(&file_path)).unwrap_or_else(|_| CanonicalDocPath {
                    full_path: PathBuf::from(&file_path),
                    relative_path: file_path.clone(),
                    kind: doc_kind,
                });
            let mut doc = Automerge::load(&state)?;

            let mut change_objs = Vec::new();
            for (i, raw) in changes.iter().enumerate() {
                match crypto::decrypt(raw, &self.encryption_key) {
                    Ok(decrypted) => match Change::from_bytes(decrypted) {
                        Ok(change) => change_objs.push(change),
                        Err(e) => {
                            eprintln!(
                                "WARNING: Failed to parse change {} for doc {}: {}",
                                i, doc_id, e
                            );
                            continue;
                        }
                    },
                    Err(e) => {
                        eprintln!("WARNING: Failed to decrypt change {} for doc {} - likely different encryption key: {}", i, doc_id, e);
                        eprintln!("  This typically happens when different devices use different encryption keys");
                        eprintln!("  Skipping this change to prevent crash");
                        continue;
                    }
                }
            }

            if change_objs.is_empty() {
                eprintln!(
                    "WARNING: No valid changes could be decrypted for doc {}, skipping",
                    doc_id
                );
                return Ok(());
            }

            doc.apply_changes(change_objs)?;

            let new_state = doc.save();

            let content = extract_automerge_content(&doc, doc_kind)?;

            // Avoid feedback loop: mark as recently synced before writing
            self.recently_synced_files
                .insert(canonical.full_path.clone());

            tokio::fs::write(&canonical.full_path, &content)
                .await
                .with_context(|| {
                    format!(
                        "Failed to write updated file: {}",
                        canonical.full_path.display()
                    )
                })?;

            let mut hasher = Sha256::new();
            hasher.update(content.as_bytes());
            let new_hash = hex::encode(hasher.finalize());

            let canonical_path_str = canonical.full_path.to_string_lossy().to_string();

            self.db.upsert_document(
                doc_id,
                &canonical_path_str,
                &doc_type,
                &new_hash,
                &new_state,
                &owner,
                writers.as_deref(),
                readers.as_deref(),
            )?;
        }

        Ok(())
    }

    /// Refresh JWT token using stored auth token
    async fn refresh_jwt_token(&mut self) -> Result<()> {
        let server_url = self
            .config
            .sync
            .as_ref()
            .and_then(|s| s.server_url.as_ref())
            .context("No server URL configured")?;

        let (email, auth_token) = self.state.get_credentials();
        let email = email.context("No stored email for refresh")?;
        let auth_token = auth_token.context("No auth token stored for refresh")?;

        // Parse server URL to get host and port
        let url_parts: Vec<&str> = server_url.split("://").collect();
        let host_port = if url_parts.len() > 1 {
            url_parts[1]
        } else {
            url_parts[0]
        };
        let host_port_parts: Vec<&str> = host_port.split(':').collect();
        let host = host_port_parts[0];
        let port: u16 = if host_port_parts.len() > 1 {
            host_port_parts[1].parse().unwrap_or(5673)
        } else {
            5673
        };

        let http_base_url = format!("http://{}:{}", host, port);

        if let Some(client) = &self.client {
            let payload = serde_json::json!({
                "email": email,
                "token": auth_token
            });

            let response = client
                .post(format!("{}/api/auth/verify", http_base_url))
                .json(&payload)
                .send()
                .await?;

            if response.status().is_success() {
                let verify_response: serde_json::Value = response.json().await?;

                if let Some(jwt) = verify_response.get("jwt").and_then(|j| j.as_str()) {
                    // Default 1 hour
                    let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);

                    self.state.store_jwt(jwt.to_string(), expires_at);
                    self.state.save()?;

                    println!("DEBUG: JWT token refreshed successfully");
                    Ok(())
                } else {
                    return Err(anyhow::anyhow!(
                        "Invalid verify response: missing JWT token"
                    ));
                }
            } else {
                let error_text = response.text().await?;
                return Err(anyhow::anyhow!(
                    "Failed to refresh JWT token: {}",
                    error_text
                ));
            }
        } else {
            return Err(anyhow::anyhow!("No HTTP client available for JWT refresh"));
        }
    }

    /// Connect to the sync server and exchange changes
    /// Returns Ok(true) if sync succeeded, Ok(false) if connection failed (non-fatal)
    async fn sync_with_server(&mut self, encrypted: HashMap<String, Vec<Vec<u8>>>) -> Result<bool> {
        println!(
            "DEBUG: sync_with_server called with {} documents containing changes",
            encrypted.len()
        );
        for (doc_id, changes) in &encrypted {
            println!(
                "DEBUG: Document {} has {} pending changes",
                doc_id,
                changes.len()
            );
        }

        // Check if JWT needs refresh before using it
        if !self.state.is_jwt_valid() || self.state.needs_jwt_refresh() {
            if self.state.get_auth_token().is_some() {
                println!("DEBUG: JWT token expired or about to expire, refreshing...");
                if let Err(e) = self.refresh_jwt_token().await {
                    eprintln!("Failed to refresh JWT token: {}", e);
                    return Err(anyhow::anyhow!("JWT token expired and refresh failed. Run 'lst auth request <email>' to re-authenticate"));
                }
            } else {
                return Err(anyhow::anyhow!("No valid JWT token and no auth token for refresh. Run 'lst auth request <email>' to authenticate"));
            }
        }

        let sync = match &self.config.sync {
            Some(s) => {
                println!("DEBUG: Found sync config");
                s
            }
            None => {
                println!("DEBUG: No sync config found");
                return Ok(true);
            }
        };

        let url = match &sync.server_url {
            Some(u) => {
                println!("DEBUG: Found server URL: {}", u);
                // Convert HTTP URLs to WebSocket URLs and ensure /api/sync path
                let mut ws_url = u.replace("http://", "ws://").replace("https://", "wss://");

                // Ensure the URL ends with /api/sync
                if !ws_url.ends_with("/api/sync") {
                    if !ws_url.ends_with("/") {
                        ws_url.push('/');
                    }
                    ws_url.push_str("api/sync");
                }

                println!("DEBUG: Converted to WebSocket URL: {}", ws_url);
                ws_url
            }
            None => {
                println!("DEBUG: No server URL found");
                return Ok(true);
            }
        };

        // Debug: Check what JWT token we have
        if let Some(ref jwt) = self.state.auth.jwt_token {
            let preview_len = std::cmp::min(20, jwt.len());
            println!("DEBUG: Found JWT token: {}...", &jwt[..preview_len]);
        } else {
            println!("DEBUG: No JWT token found in state");
        }

        let token = self
            .state
            .auth
            .jwt_token
            .as_ref()
            .context("No valid JWT token after refresh attempt")?
            .to_string();

        let device_id = self
            .state
            .device
            .device_id
            .clone()
            .unwrap_or_else(|| "unknown".to_string());

        // Connect to WebSocket with Authorization header and timeout
        let mut ws_request = url.as_str().into_client_request()?;
        ws_request
            .headers_mut()
            .insert(AUTHORIZATION, format!("Bearer {}", token).parse()?);

        let connection_result = timeout(Duration::from_secs(10), connect_async(ws_request)).await;
        let (ws, _) = match connection_result {
            Ok(Ok(ws)) => ws,
            Ok(Err(e)) => {
                eprintln!("Failed to connect to sync server: {}", e);
                eprintln!("The server may be unreachable. Will retry on next sync interval.");
                return Ok(false); // Return false to indicate connection failure
            }
            Err(_) => {
                eprintln!("Connection to sync server timed out after 10 seconds");
                eprintln!("Will retry on next sync interval.");
                return Ok(false); // Return false to indicate connection failure
            }
        };
        let (mut write, mut read) = ws.split();
        println!("WebSocket connection established with HTTP header auth");

        // 1) Discover server docs
        let request_list = lst_proto::ClientMessage::RequestDocumentList;
        write
            .send(Message::Text(serde_json::to_string(&request_list)?))
            .await?;

        // 2) Push local pending changes
        println!(
            "DEBUG: Processing {} documents with changes",
            encrypted.len()
        );
        for (doc_id, changes) in encrypted {
            if changes.is_empty() {
                println!("DEBUG: Skipping doc {} - no changes", doc_id);
                continue;
            }
            println!(
                "DEBUG: Pushing {} changes for doc {}",
                changes.len(),
                doc_id
            );
            let uuid = Uuid::parse_str(&doc_id)?;
            let msg = lst_proto::ClientMessage::PushChanges {
                doc_id: uuid,
                device_id: device_id.clone(),
                changes,
            };
            write
                .send(Message::Text(serde_json::to_string(&msg)?))
                .await?;
            println!("DEBUG: Sent PushChanges message for doc {}", doc_id);
        }

        // 3) After receiving server list, request snapshots for unknown docs
        //    Also, if we have local docs unknown to server, push snapshots to seed them.
        //    We handle this inside the read loop when DocumentList arrives.

        // read messages until timeout (give server time to process changes and send snapshots)
        let mut expected_snapshots = 0;
        let mut received_snapshots = 0;
        let mut received_document_list = false;

        loop {
            match timeout(Duration::from_secs(60), read.next()).await {
                Ok(Some(Ok(Message::Text(txt)))) => {
                    if let Ok(server_msg) = serde_json::from_str::<lst_proto::ServerMessage>(&txt) {
                        match server_msg {
                            lst_proto::ServerMessage::NewChanges {
                                doc_id,
                                from_device_id,
                                changes,
                            } => {
                                // Filter out our own changes to avoid infinite loops
                                if from_device_id != device_id {
                                    println!("DEBUG: Applying {} remote changes for doc {} from device {}", changes.len(), doc_id, from_device_id);
                                    self.apply_remote_changes(&doc_id.to_string(), changes)
                                        .await?;
                                } else {
                                    println!(
                                        "DEBUG: Ignoring own changes for doc {} from device {}",
                                        doc_id, from_device_id
                                    );
                                }
                            }
                            lst_proto::ServerMessage::DocumentList { documents } => {
                                received_document_list = true;
                                println!(
                                    "DEBUG: ✅ RECEIVED DocumentList with {} documents from server",
                                    documents.len()
                                );

                                // Build a set of known local docs
                                let mut local_ids = std::collections::HashSet::new();
                                let local_docs = self.db.list_all_documents()?;
                                println!("DEBUG: Found {} local documents", local_docs.len());
                                for (doc_id, _path, _typ, _state, _owner, _w, _r) in local_docs {
                                    println!("DEBUG: Local doc: {}", doc_id);
                                    local_ids.insert(doc_id);
                                }

                                // Request snapshots for unknown server docs
                                for info in &documents {
                                    let id_str = info.doc_id.to_string();
                                    if !local_ids.contains(&id_str) {
                                        println!(
                                            "DEBUG: Requesting snapshot for missing doc: {}",
                                            id_str
                                        );
                                        let req = lst_proto::ClientMessage::RequestSnapshot {
                                            doc_id: info.doc_id,
                                        };
                                        let _ = write
                                            .send(Message::Text(serde_json::to_string(&req)?))
                                            .await;
                                        expected_snapshots += 1;
                                    } else {
                                        println!("DEBUG: Doc {} already exists locally, skipping snapshot request", id_str);
                                    }
                                }
                                println!("DEBUG: Finished processing {} server documents, expecting {} snapshots", documents.len(), expected_snapshots);

                                // Push snapshots for local docs missing on server
                                use std::collections::HashSet;
                                let server_ids: HashSet<String> = documents
                                    .into_iter()
                                    .map(|d| d.doc_id.to_string())
                                    .collect();
                                println!("DEBUG: Server has {} documents", server_ids.len());
                                let local_docs_for_push = self.db.list_all_documents()?;
                                let mut pushed_count = 0;
                                for (doc_id, path, _typ, state, _owner, _w, _r) in
                                    local_docs_for_push
                                {
                                    if !server_ids.contains(&doc_id) {
                                        println!("DEBUG: 📤 Pushing local doc {} to server (not on server)", doc_id);
                                        if let Ok(uuid) = Uuid::parse_str(&doc_id) {
                                            // Extract relative path from content directory to preserve structure
                                            let content_dir = lst_core::storage::get_content_dir()
                                                .unwrap_or_else(|_| std::path::PathBuf::from("."));
                                            let relative_path = std::path::Path::new(&path)
                                                .strip_prefix(&content_dir)
                                                .unwrap_or(std::path::Path::new("unknown.md"))
                                                .to_string_lossy()
                                                .to_string();

                                            // Encrypt relative path before sending
                                            let encrypted_filename = crypto::encrypt(
                                                relative_path.as_bytes(),
                                                &self.encryption_key,
                                            )?;
                                            let encoded_filename = general_purpose::STANDARD
                                                .encode(&encrypted_filename);

                                            println!(
                                                "DEBUG: 🔐 Encrypting relative path: {} for doc {}",
                                                relative_path, doc_id
                                            );

                                            let msg = lst_proto::ClientMessage::PushSnapshot {
                                                doc_id: uuid,
                                                filename: encoded_filename,
                                                snapshot: state,
                                            };
                                            if let Err(e) = write
                                                .send(Message::Text(serde_json::to_string(&msg)?))
                                                .await
                                            {
                                                println!("DEBUG: ❌ Failed to send PushSnapshot for {}: {}", doc_id, e);
                                            } else {
                                                pushed_count += 1;
                                                println!(
                                                    "DEBUG: ✅ Sent PushSnapshot for {}",
                                                    doc_id
                                                );
                                            }
                                        }
                                    } else {
                                        println!("DEBUG: Doc {} already exists on server", doc_id);
                                    }
                                }
                                println!(
                                    "DEBUG: 📤 Pushed {} local documents to server",
                                    pushed_count
                                );
                            }
                            lst_proto::ServerMessage::Snapshot {
                                doc_id,
                                filename,
                                snapshot,
                            } => {
                                received_snapshots += 1;
                                println!(
                                    "DEBUG: Received snapshot {}/{} for doc {} ({} bytes)",
                                    received_snapshots,
                                    expected_snapshots,
                                    doc_id,
                                    snapshot.len()
                                );

                                // Decrypt filename
                                let decrypted_filename = if let Ok(encrypted_bytes) =
                                    general_purpose::STANDARD.decode(&filename)
                                {
                                    if let Ok(decrypted_bytes) =
                                        crypto::decrypt(&encrypted_bytes, &self.encryption_key)
                                    {
                                        String::from_utf8(decrypted_bytes).unwrap_or_else(|_| {
                                            format!("{}.md", &doc_id.to_string()[..8])
                                        })
                                    } else {
                                        format!("{}.md", &doc_id.to_string()[..8])
                                    }
                                } else {
                                    format!("{}.md", &doc_id.to_string()[..8])
                                };

                                println!("DEBUG: Decrypted filename: {}", decrypted_filename);

                                // Persist snapshot as baseline
                                let id_str = doc_id.to_string();
                                match self.db.get_document(&id_str)? {
                                    Some((_path, _typ, _hash, _state, owner, writers, readers)) => {
                                        let _ = self.db.save_document_snapshot(
                                            &id_str,
                                            &snapshot,
                                            Some(owner.as_str()),
                                            writers.as_deref(),
                                            readers.as_deref(),
                                        );
                                    }
                                    None => {
                                        let _ = self
                                            .db
                                            .insert_new_document_from_snapshot_with_filename(
                                                &id_str,
                                                &decrypted_filename,
                                                &snapshot,
                                            );
                                    }
                                }

                                // Check if we've received all expected snapshots
                                if expected_snapshots > 0
                                    && received_snapshots >= expected_snapshots
                                {
                                    println!("DEBUG: Received all {} expected snapshots, closing connection", expected_snapshots);
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Some(Ok(Message::Close(_)))) => {
                    println!("DEBUG: Server closed WebSocket connection");
                    break;
                }
                Ok(Some(Ok(_))) => {}
                Ok(Some(Err(e))) => {
                    println!("DEBUG: WebSocket error: {}", e);
                    break;
                }
                Ok(None) => {
                    println!("DEBUG: WebSocket stream ended");
                    break;
                }
                Err(_) => {
                    println!("DEBUG: WebSocket read timeout after 60 seconds, closing connection");
                    println!("DEBUG: DocumentList received: {}, Received {}/{} expected snapshots before timeout", 
                             received_document_list, received_snapshots, expected_snapshots);
                    break;
                }
            }
        }

        // ignore errors closing
        let _ = write.close().await;
        Ok(true) // Sync succeeded
    }

    /// Scan all existing files in content directory and add them to sync
    async fn ensure_initial_sync(&mut self) -> Result<()> {
        println!("DEBUG: Starting initial file discovery...");
        let content_dir = lst_core::storage::get_content_dir()?;

        // Recursively scan content directory for .md files
        let mut files_found = 0;
        let mut files_added = 0;

        if let Ok(entries) = std::fs::read_dir(&content_dir) {
            for entry in entries.flatten() {
                if let Err(e) = self
                    .scan_directory_recursive(entry.path(), &mut files_found, &mut files_added)
                    .await
                {
                    eprintln!("Error scanning directory {}: {}", entry.path().display(), e);
                }
            }
        }

        println!(
            "DEBUG: Initial sync: Found {} files, added {} to sync",
            files_found, files_added
        );
        Ok(())
    }

    /// Recursively scan directory for markdown files
    async fn scan_directory_recursive(
        &mut self,
        dir_path: std::path::PathBuf,
        files_found: &mut usize,
        files_added: &mut usize,
    ) -> Result<()> {
        if dir_path.is_file() {
            if let Some(ext) = dir_path.extension() {
                if ext == "md" {
                    *files_found += 1;
                    if let Err(e) = self.process_existing_file(&dir_path, files_added).await {
                        eprintln!(
                            "Error processing existing file {}: {}",
                            dir_path.display(),
                            e
                        );
                    }
                }
            }
        } else if dir_path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&dir_path) {
                for entry in entries.flatten() {
                    Box::pin(self.scan_directory_recursive(entry.path(), files_found, files_added))
                        .await?;
                }
            }
        }
        Ok(())
    }

    /// Process an existing file and add it to sync if not already tracked
    async fn process_existing_file(
        &mut self,
        file_path: &std::path::Path,
        files_added: &mut usize,
    ) -> Result<()> {
        let (canonical, derived_doc_id) = canonical_path_with_id(file_path)?;
        let file_path_str = canonical.full_path.to_string_lossy().to_string();
        let existing_doc_id = self
            .db
            .get_doc_id_by_file_path(&file_path_str)
            .ok()
            .flatten()
            .or_else(|| {
                self.db
                    .get_doc_id_by_file_path(&canonical.relative_path)
                    .ok()
                    .flatten()
            });
        let doc_id = existing_doc_id.unwrap_or_else(|| derived_doc_id.clone());

        // Check if already in database
        if self.db.get_document(&doc_id)?.is_some() {
            return Ok(()); // Already tracked
        }

        println!(
            "DEBUG: Discovering new file: {} -> {}",
            file_path.display(),
            doc_id
        );

        // Read file content
        let data = match tokio::fs::read(file_path).await {
            Ok(data) => data,
            Err(_) => return Ok(()), // Skip unreadable files
        };

        // Check file size limits
        if let Some(sync) = &self.config.sync {
            if data.len() as u64 > sync.max_file_size {
                return Ok(());
            }
        }

        let mut hasher = sha2::Sha256::new();
        hasher.update(&data);
        let hash = hex::encode(hasher.finalize());

        let doc_kind = canonical.kind;
        let owner = self
            .state
            .device
            .device_id
            .as_ref()
            .map(String::as_str)
            .unwrap_or("local");
        let new_content = String::from_utf8_lossy(&data);

        // Create new Automerge document
        let mut doc = Automerge::new();
        let old_heads = doc.get_heads().into_iter().collect::<Vec<_>>();

        update_automerge_doc(&mut doc, doc_kind, &new_content)?;

        let new_state = doc.save();

        // Store in database
        self.db.upsert_document(
            &doc_id,
            &file_path_str,
            doc_kind.as_str(),
            &hash,
            &new_state,
            owner,
            None,
            None,
        )?;

        // Add to pending changes for sync
        let new_doc = Automerge::load(&new_state)?;
        let changes = new_doc
            .get_changes(&old_heads)
            .into_iter()
            .map(|c| c.raw_bytes().to_vec())
            .collect::<Vec<_>>();

        if !changes.is_empty() {
            self.pending_changes.insert(doc_id.clone(), changes);
            *files_added += 1;
            println!(
                "DEBUG: Added existing file to sync: {}",
                file_path.display()
            );
        }

        Ok(())
    }

    pub async fn periodic_sync(&mut self) -> Result<()> {
        let interval = self
            .config
            .sync
            .as_ref()
            .map(|s| s.interval_seconds)
            .unwrap_or(30);

        if self.last_sync.elapsed().as_secs() < interval {
            return Ok(());
        }

        // Do initial sync on first run
        if !self.initial_sync_done {
            self.ensure_initial_sync().await?;
            self.initial_sync_done = true;
        }

        if self.client.is_some() {
            if !self.pending_changes.is_empty() {
                let mut encrypted_total = 0;
                let mut encrypted: HashMap<String, Vec<Vec<u8>>> = HashMap::new();
                // Keep a backup of pending changes in case sync fails
                let pending_backup = self.pending_changes.clone();
                
                println!(
                    "DEBUG: Draining {} documents from pending_changes",
                    self.pending_changes.len()
                );
                for (doc, changes) in self.pending_changes.drain() {
                    println!(
                        "DEBUG: Processing {} changes for doc {} during drain",
                        changes.len(),
                        doc
                    );
                    let mut enc = Vec::new();
                    for c in changes {
                        let e = crypto::encrypt(&c, &self.encryption_key)?;
                        encrypted_total += 1;
                        enc.push(e);
                    }
                    let enc_len = enc.len();
                    encrypted.insert(doc.clone(), enc);
                    println!("DEBUG: Added {enc_len} encrypted changes for doc {doc}");
                }

                println!("Syncing {encrypted_total} encrypted changes");
                match self.sync_with_server(encrypted).await {
                    Ok(true) => {
                        // Sync succeeded, changes were sent
                        println!("DEBUG: Sync completed successfully");
                    }
                    Ok(false) => {
                        // Connection failed, restore pending changes for next sync
                        println!("DEBUG: Sync connection failed, restoring pending changes for retry");
                        self.pending_changes = pending_backup;
                    }
                    Err(e) => {
                        // Other error, restore pending changes and propagate error
                        self.pending_changes = pending_backup;
                        return Err(e);
                    }
                }
            } else {
                // still poll server for new changes
                let _ = self.sync_with_server(HashMap::new()).await;
            }
        }

        self.last_sync = Instant::now();

        Ok(())
    }
}
