use crate::config::Config;
use lst_core::config::State;
use crate::database::LocalDb;
use crate::crypto;
use anyhow::{bail, Context, Result};
use automerge::{
    transaction::Transactable as _,
    Automerge, Change, ObjType, ReadDoc, ScalarValue, Value,
};
use notify::Event;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tokio::time::{Instant, timeout};
use std::time::Duration;
use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

fn detect_doc_type(path: &std::path::Path) -> &str {
    let s = path.to_string_lossy();
    if s.contains("/lists/") || s.contains("daily_lists") {
        "list"
    } else {
        "note"
    }
}

fn update_note_doc(doc: &mut Automerge, content: &str) -> Result<()> {
    let mut tx = doc.transaction();
    tx.put(&automerge::ROOT, "content", "")?;
    tx.update_text(&automerge::ROOT, content)?;
    tx.commit();
    Ok(())
}

fn update_list_doc(doc: &mut Automerge, content: &str) -> Result<()> {
    let mut tx = doc.transaction();
    
    // Create or recreate the list
    tx.delete(&automerge::ROOT, "items").ok(); // Ignore error if doesn't exist
    let items_id = tx.put_object(&automerge::ROOT, "items", ObjType::List)?;

    for (idx, line) in content.lines().enumerate() {
        let line = line.trim();
        if !line.is_empty() {
            tx.insert(&items_id, idx, ScalarValue::Str(line.into()))?;
        }
    }

    tx.commit();
    Ok(())
}

pub struct SyncManager {
    config: Config,
    state: State,
    client: Option<reqwest::Client>,
    db: LocalDb,
    encryption_key: [u8; 32],
    last_sync: Instant,
    pending_changes: HashMap<String, Vec<Vec<u8>>>,
}

impl SyncManager {
    pub async fn new(config: Config) -> Result<Self> {
        let client = if config.sync.as_ref().and_then(|s| s.server_url.as_ref()).is_some() {
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
        let encryption_key = crypto::load_key(std::path::Path::new(key_path))?;

        Ok(Self {
            config,
            state,
            client,
            db,
            encryption_key,
            last_sync: Instant::now(),
            pending_changes: HashMap::new(),
        })
    }

    pub async fn handle_file_event(&mut self, event: Event) -> Result<()> {
        for path in event.paths {
            if let Some(filename) = path.file_name() {
                if let Some(filename_str) = filename.to_str() {
                    if filename_str.starts_with('.')
                        || filename_str.ends_with(".tmp")
                        || filename_str.ends_with(".swp")
                    {
                        continue;
                    }
                }
            }

            let doc_id = uuid::Uuid::new_v5(
                &uuid::Uuid::NAMESPACE_OID,
                path.to_string_lossy().as_bytes(),
            )
            .to_string();

            if matches!(event.kind, notify::EventKind::Remove(_)) {
                self.db.delete_document(&doc_id)?;
                self.pending_changes.remove(&doc_id);
                continue;
            }

            let data = tokio::fs::read(&path).await.unwrap_or_default();

            if let Some(sync) = &self.config.sync {
                if data.len() as u64 > sync.max_file_size {
                    continue;
                }
            }

            let mut hasher = Sha256::new();
            hasher.update(&data);
            let hash = hex::encode(hasher.finalize());

            let doc_type = detect_doc_type(&path);

            let owner = self
                .state
                .device
                .device_id
                .as_ref()
                .map(String::as_str)
                .unwrap_or("local");

            let new_content = String::from_utf8_lossy(&data);

            if let Some((_, _, last_hash, state, existing_owner, writers, readers)) =
                self.db.get_document(&doc_id)?
            {
                if last_hash == hash {
                    continue;
                }

                let mut doc = Automerge::load(&state)?;
                let old_heads = doc.get_heads().into_iter().collect::<Vec<_>>();

                if doc_type == "list" {
                    update_list_doc(&mut doc, &new_content)?;
                } else {
                    update_note_doc(&mut doc, &new_content)?;
                }

                let new_state = doc.save();

                self.db.upsert_document(
                    &doc_id,
                    &path.to_string_lossy(),
                    doc_type,
                    &hash,
                    &new_state,
                    &existing_owner,
                    writers.as_deref(),
                    readers.as_deref(),
                )?;

                let new_doc = Automerge::load(&new_state)?;
                let changes = new_doc.get_changes(&old_heads)
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

                if doc_type == "list" {
                    update_list_doc(&mut doc, &new_content)?;
                } else {
                    update_note_doc(&mut doc, &new_content)?;
                }

                let new_state = doc.save();

                self.db.upsert_document(
                    &doc_id,
                    &path.to_string_lossy(),
                    doc_type,
                    &hash,
                    &new_state,
                    owner,
                    None,
                    None,
                )?;

                let new_doc = Automerge::load(&new_state)?;
                let changes = new_doc.get_changes(&old_heads)
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
            let mut doc = Automerge::load(&state)?;

            let mut change_objs = Vec::new();
            for raw in changes {
                let decrypted = crypto::decrypt(&raw, &self.encryption_key)?;
                let change = Change::from_bytes(decrypted)?;
                change_objs.push(change);
            }

            doc.apply_changes(change_objs)?;

            let new_state = doc.save();

            let content = if doc_type == "list" {
                if let Some((items_val, items_id)) = doc.get(automerge::ROOT, "items")? {
                    if let Value::Object(_) = items_val {
                        let mut lines = Vec::new();
                        let len = doc.length(&items_id);
                        for i in 0..len {
                            if let Some((val, _)) = doc.get(&items_id, i)? {
                                match val {
                                    Value::Scalar(s) => {
                                        if let ScalarValue::Str(text) = s.as_ref() {
                                            lines.push(text.to_string());
                                        }
                                    },
                                    _ => {}
                                }
                            }
                        }
                        lines.join("\n")
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else {
                // For text content, try to get it as text object first, then as scalar
                if let Some((content_val, content_id)) = doc.get(automerge::ROOT, "content")? {
                    match content_val {
                        Value::Object(_) => {
                            // Try to get as text first
                            doc.text(&content_id).unwrap_or_default()
                        },
                        Value::Scalar(s) => {
                            if let ScalarValue::Str(text) = s.as_ref() {
                                text.to_string()
                            } else {
                                String::new()
                            }
                        },
                        _ => String::new(),
                    }
                } else {
                    String::new()
                }
            };

            tokio::fs::write(&file_path, &content)
                .await
                .with_context(|| format!("Failed to write updated file: {}", file_path))?;

            let mut hasher = Sha256::new();
            hasher.update(content.as_bytes());
            let new_hash = hex::encode(hasher.finalize());

            self.db.upsert_document(
                doc_id,
                &file_path,
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
        let server_url = self.config
            .sync
            .as_ref()
            .and_then(|s| s.server_url.as_ref())
            .context("No server URL configured")?;

        let auth_token = self.state
            .get_auth_token()
            .context("No auth token stored for refresh")?;

        // Parse server URL to get host and port
        let url_parts: Vec<&str> = server_url.split("://").collect();
        let host_port = if url_parts.len() > 1 { url_parts[1] } else { url_parts[0] };
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
                "password_hash": auth_token
            });

            let response = client
                .post(format!("{}/api/auth/refresh", http_base_url))
                .json(&payload)
                .send()
                .await?;

            if response.status().is_success() {
                let refresh_response: serde_json::Value = response.json().await?;

                if let Some(jwt) = refresh_response.get("jwt").and_then(|j| j.as_str()) {
                    // Parse JWT to get expiration (basic extraction without validation)
                    let expires_at = chrono::Utc::now() + chrono::Duration::hours(1); // Default 1 hour

                    self.state.store_jwt(jwt.to_string(), expires_at);
                    self.state.save()?;
                    
                    println!("DEBUG: JWT token refreshed successfully");
                    Ok(())
                } else {
                    return Err(anyhow::anyhow!("Invalid refresh response: missing JWT token"));
                }
            } else {
                let error_text = response.text().await?;
                return Err(anyhow::anyhow!("Failed to refresh JWT token: {}", error_text));
            }
        } else {
            return Err(anyhow::anyhow!("No HTTP client available for JWT refresh"));
        }
    }

    /// Connect to the sync server and exchange changes
    async fn sync_with_server(&mut self, encrypted: HashMap<String, Vec<Vec<u8>>>) -> Result<()> {
        println!("DEBUG: sync_with_server called");
        
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
            },
            None => {
                println!("DEBUG: No sync config found");
                return Ok(())
            },
        };

        let url = match &sync.server_url {
            Some(u) => {
                println!("DEBUG: Found server URL: {}", u);
                u
            },
            None => {
                println!("DEBUG: No server URL found");
                return Ok(())
            },
        };

        // Debug: Check what JWT token we have
        if let Some(ref jwt) = self.state.auth.jwt_token {
            let preview_len = std::cmp::min(20, jwt.len());
            println!("DEBUG: Found JWT token: {}...", &jwt[..preview_len]);
        } else {
            println!("DEBUG: No JWT token found in state");
        }
        
        let token = self.state
            .auth
            .jwt_token
            .as_ref()
            .context("No valid JWT token after refresh attempt")?
            .to_string();

        let device_id = self.state
            .device
            .device_id
            .clone()
            .unwrap_or_else(|| "unknown".to_string());

        // Connect to WebSocket with Authorization header (like mobile app)
        use tokio_tungstenite::tungstenite::http::Request;
        use base64::Engine;
        let ws_request = Request::builder()
            .method("GET")
            .uri(url.as_str())
            .header("Host", "192.168.1.25:5673")
            .header("Upgrade", "websocket")
            .header("Connection", "Upgrade")
            .header("Sec-WebSocket-Key", base64::engine::general_purpose::STANDARD.encode("desktop-key-12345678"))
            .header("Sec-WebSocket-Version", "13")
            .header("Authorization", format!("Bearer {}", token))
            .body(())?;

        let (ws, _) = connect_async(ws_request).await?;
        let (mut write, mut read) = ws.split();
        println!("WebSocket connection established with HTTP header auth");

        // 1) Discover server docs
        let request_list = lst_proto::ClientMessage::RequestDocumentList;
        write.send(Message::Text(serde_json::to_string(&request_list)?)).await?;

        // 2) Push local pending changes
        for (doc_id, changes) in encrypted {
            if changes.is_empty() {
                continue;
            }
            let uuid = Uuid::parse_str(&doc_id)?;
            let msg = lst_proto::ClientMessage::PushChanges {
                doc_id: uuid,
                device_id: device_id.clone(),
                changes,
            };
            write.send(Message::Text(serde_json::to_string(&msg)?)).await?;
        }

        // 3) After receiving server list, request snapshots for unknown docs
        //    Also, if we have local docs unknown to server, push snapshots to seed them.
        //    We handle this inside the read loop when DocumentList arrives.

        // read messages until small timeout
        loop {
            match timeout(Duration::from_secs(2), read.next()).await {
                Ok(Some(Ok(Message::Text(txt)))) => {
                    if let Ok(server_msg) = serde_json::from_str::<lst_proto::ServerMessage>(&txt) {
                        match server_msg {
                            lst_proto::ServerMessage::NewChanges { doc_id, changes, .. } => {
                                self.apply_remote_changes(&doc_id.to_string(), changes).await?;
                            }
                            lst_proto::ServerMessage::DocumentList { documents } => {
                                // Build a set of known local docs
                                let mut local_ids = std::collections::HashSet::new();
                                for (doc_id, _path, _typ, _state, _owner, _w, _r) in self.db.list_all_documents()? {
                                    local_ids.insert(doc_id);
                                }
                                // Request snapshots for unknown server docs
                                for info in &documents {
                                    let id_str = info.doc_id.to_string();
                                    if !local_ids.contains(&id_str) {
                                        let req = lst_proto::ClientMessage::RequestSnapshot { doc_id: info.doc_id };
                                        let _ = write.send(Message::Text(serde_json::to_string(&req)?)).await;
                                    }
                                }
                                // Push snapshots for local docs missing on server
                                use std::collections::HashSet;
                                let server_ids: HashSet<String> = documents.into_iter().map(|d| d.doc_id.to_string()).collect();
                                for (doc_id, _path, _typ, state, _owner, _w, _r) in self.db.list_all_documents()? {
                                    if !server_ids.contains(&doc_id) {
                                        if let Ok(uuid) = Uuid::parse_str(&doc_id) {
                                            let msg = lst_proto::ClientMessage::PushSnapshot { doc_id: uuid, snapshot: state };
                                            let _ = write.send(Message::Text(serde_json::to_string(&msg)?)).await;
                                        }
                                    }
                                }
                            }
                            lst_proto::ServerMessage::Snapshot { doc_id, snapshot } => {
                                // Persist snapshot as baseline
                                let id_str = doc_id.to_string();
                                match self.db.get_document(&id_str)? {
                                    Some((_path, _typ, _hash, _state, owner, writers, readers)) => {
                                        let _ = self.db.save_document_snapshot(&id_str, &snapshot, Some(owner.as_str()), writers.as_deref(), readers.as_deref());
                                    }
                                    None => {
                                        let _ = self.db.insert_new_document_from_snapshot(&id_str, &snapshot);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Some(Ok(Message::Close(_)))) => break,
                Ok(Some(Ok(_))) => {},
                Ok(Some(Err(_e))) => break,
                Ok(None) => break,
                Err(_) => break,
            }
        }

        // ignore errors closing
        let _ = write.close().await;
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

        if self.client.is_some() {
            if !self.pending_changes.is_empty() {
                let mut encrypted_total = 0;
                let mut encrypted: HashMap<String, Vec<Vec<u8>>> = HashMap::new();
                for (doc, changes) in self.pending_changes.drain() {
                    let mut enc = Vec::new();
                    for c in changes {
                        let e = crypto::encrypt(&c, &self.encryption_key)?;
                        encrypted_total += 1;
                        enc.push(e);
                    }
                    encrypted.insert(doc, enc);
                }

                println!("Syncing {encrypted_total} encrypted changes");
                self.sync_with_server(encrypted).await?;
            } else {
                // still poll server for new changes
                self.sync_with_server(HashMap::new()).await?;
            }
        }

        self.last_sync = Instant::now();

        Ok(())
    }
}
