// Mobile-specific sync manager that uses mobile configuration
// This replaces the desktop sync.rs with mobile-only functionality

use crate::crypto;
use crate::mobile_config::MobileConfig;
use crate::sync_db::LocalDb;
use crate::sync_status;
use anyhow::{Context, Result};
use automerge::{
    transaction::Transactable as _, Automerge, Change, ObjType, ReadDoc, ScalarValue, Value,
};
use futures_util::{SinkExt, StreamExt};
use lst_proto;
use notify::Event;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::{timeout, Instant};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;
use base64::Engine;

fn detect_doc_type(path: &std::path::Path) -> &str {
    let s = path.to_string_lossy();
    if s.contains("/lists/") || s.contains("daily_lists") {
        "list"
    } else if s.contains("/notes/") {
        "note"
    } else {
        "note" // default to note
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

pub struct MobileSyncManager {
    config: MobileConfig,
    client: Option<reqwest::Client>,
    db: LocalDb,
    encryption_key: [u8; 32],
    last_sync: Instant,
    pending_changes: HashMap<String, Vec<Vec<u8>>>,
    initial_sync_done: bool,
}

impl MobileSyncManager {
    pub async fn new(config: MobileConfig) -> Result<Self> {
        // Validate that we have the necessary sync configuration
        let syncd = config.syncd.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No syncd configuration found"))?;
        
        let client = if syncd.url.is_some() {
            Some(reqwest::Client::new())
        } else {
            None
        };

        // Ensure CRDT storage directory exists
        let storage = config.get_storage();
        tokio::fs::create_dir_all(&storage.crdt_dir)
            .await
            .with_context(|| {
                format!(
                    "Failed to create CRDT directory: {}",
                    storage.crdt_dir.display()
                )
            })?;

        // Set up database path
        let db_path = syncd.database_path.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No database path in syncd config"))?;

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let db = LocalDb::new(db_path)?;

        // Set up encryption key
        let key_path = syncd.encryption_key_path.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No encryption key path in syncd config"))?;
        
        // Ensure parent directory exists
        if let Some(parent) = key_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Use secure credential-based key derivation 
        // Get stored credentials from mobile config
        let (email, auth_token) = config.sync.get_credentials();
        let encryption_key = if let (Some(_email), Some(_auth_token)) = (email, auth_token) {
            // Try to load the key that was saved during login
            match lst_core::crypto::load_key(key_path) {
                Ok(key) => {
                    println!("DEBUG: Mobile sync using encryption key from file (derived during login)");
                    key
                }
                Err(_) => {
                    // Key file doesn't exist yet - this means the user needs to login
                    eprintln!("ERROR: No encryption key file found");
                    eprintln!("       Please complete authentication in the mobile app to derive and save the key");
                    return Err(anyhow::anyhow!("Authentication required: no encryption key available"));
                }
            }
        } else {
            eprintln!("WARNING: No stored credentials found in mobile config");
            eprintln!("         Please complete authentication in the mobile app first");
            // Fall back to legacy key derivation for compatibility
            crypto::load_or_derive_key(key_path, auth_token)?
        };

        Ok(Self {
            config,
            client,
            db,
            encryption_key,
            last_sync: Instant::now(),
            pending_changes: HashMap::new(),
            initial_sync_done: false,
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

            // Generate doc_id using the same method as desktop for consistency
            let doc_id = uuid::Uuid::new_v5(
                &uuid::Uuid::NAMESPACE_OID,
                path.to_string_lossy().as_bytes(),
            )
            .to_string();
            
            println!("📊 Mobile sync: Generated doc_id '{}' for path: {}", doc_id, path.display());

            if matches!(event.kind, notify::EventKind::Remove(_)) {
                self.db.delete_document(&doc_id)?;
                self.pending_changes.remove(&doc_id);
                continue;
            }

            let data = tokio::fs::read(&path).await.unwrap_or_default();

            // Check file size limits
            if let Some(sync_settings) = &self.config.sync_settings {
                if data.len() as u64 > sync_settings.max_file_size {
                    continue;
                }
            }

            let mut hasher = Sha256::new();
            hasher.update(&data);
            let hash = hex::encode(hasher.finalize());

            let doc_type = detect_doc_type(&path);

            let owner = self.config.syncd.as_ref()
                .and_then(|s| s.device_id.as_ref())
                .map(String::as_str)
                .unwrap_or("mobile");

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
            let mut doc = Automerge::load(&state)?;

            let mut change_objs = Vec::new();
            for (i, raw) in changes.iter().enumerate() {
                match crypto::decrypt(raw, &self.encryption_key) {
                    Ok(decrypted) => {
                        match Change::from_bytes(decrypted) {
                            Ok(change) => change_objs.push(change),
                            Err(e) => {
                                println!("📱 WARNING: Failed to parse change {} for doc {}: {}", i, doc_id, e);
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        println!("📱 WARNING: Failed to decrypt change {} for doc {} - likely different encryption key: {}", i, doc_id, e);
                        println!("📱   This typically happens when different devices use different encryption keys");
                        println!("📱   Skipping this change to prevent crash");
                        continue;
                    }
                }
            }
            
            if change_objs.is_empty() {
                println!("📱 WARNING: No valid changes could be decrypted for doc {}, skipping", doc_id);
                return Ok(());
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
                                    }
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
                        }
                        Value::Scalar(s) => {
                            if let ScalarValue::Str(text) = s.as_ref() {
                                text.to_string()
                            } else {
                                String::new()
                            }
                        }
                    }
                } else {
                    String::new()
                }
            };

            // Ensure directory structure exists before writing
            let file_path_obj = std::path::Path::new(&file_path);
            if let Some(parent) = file_path_obj.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .with_context(|| format!("Failed to create directory structure: {}", parent.display()))?;
            }

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

            println!("📱 Mobile sync: Applied remote changes to {} ({})", doc_id, file_path);
        }

        Ok(())
    }

    /// Connect to the sync server and exchange changes
    async fn sync_with_server(&mut self, encrypted: HashMap<String, Vec<Vec<u8>>>) -> Result<()> {
        let syncd = self.config.syncd.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Sync not configured"))?;

        let url = syncd.url.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Server URL not configured"))?;
        
        let token = self.config.get_jwt()
            .ok_or_else(|| anyhow::anyhow!("No valid JWT token. Please authenticate first"))?;

        let device_id = syncd.device_id.clone()
            .unwrap_or_else(|| "mobile".to_string());

        // Connect to WebSocket with Authorization header
        let request = http::Request::builder()
            .method("GET")
            .uri(url.as_str())
            .header("Host", url.replace("ws://", "").replace("wss://", "").replace("/api/sync", ""))
            .header("Upgrade", "websocket")
            .header("Connection", "Upgrade")
            .header("Sec-WebSocket-Key", base64::engine::general_purpose::STANDARD.encode("mobile-key-1234567"))
            .header("Sec-WebSocket-Version", "13")
            .header("Authorization", format!("Bearer {}", token))
            .body(())?;
        
        let connection_result = timeout(Duration::from_secs(10), 
            connect_async(request)
        ).await;
        let (ws, _) = match connection_result {
            Ok(Ok(ws)) => ws,
            Ok(Err(e)) => return Err(anyhow::anyhow!("Failed to connect to server: {}", e)),
            Err(_) => return Err(anyhow::anyhow!("Connection timeout")),
        };

        let (mut write, mut read) = ws.split();
        println!("📱 Mobile WebSocket connection established!");

        // Request document list to discover new documents
        println!("📱 Requesting document list from server...");
        let request_msg = lst_proto::ClientMessage::RequestDocumentList;
        if let Err(e) = write.send(Message::Text(serde_json::to_string(&request_msg)?)).await {
            return Err(anyhow::anyhow!("Failed to request document list: {}", e));
        }

        // Send changes to server
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
            
            if let Err(e) = write.send(Message::Text(serde_json::to_string(&msg)?)).await {
                return Err(anyhow::anyhow!("Failed to send changes: {}", e));
            }
        }

        // Read any new changes from server
        let mut changes_received = 0;
        loop {
            match timeout(Duration::from_secs(2), read.next()).await {
                Ok(Some(Ok(Message::Text(txt)))) => {
                    if let Ok(server_msg) = serde_json::from_str::<lst_proto::ServerMessage>(&txt) {
                        match server_msg {
                            lst_proto::ServerMessage::NewChanges { doc_id, from_device_id, changes } => {
                                // Only apply changes if they're from a different device
                                if from_device_id != device_id {
                                    println!("📱 Mobile sync: Applying changes from device: {} for doc: {}", from_device_id, doc_id);
                                    self.apply_remote_changes(&doc_id.to_string(), changes).await?;
                                    changes_received += 1;
                                } else {
                                    println!("📱 Mobile sync: Ignoring changes from own device: {} for doc: {}", from_device_id, doc_id);
                                }
                            }
                            lst_proto::ServerMessage::DocumentList { documents } => {
                                println!("📱 Mobile sync: Received document list with {} documents", documents.len());
                                for doc_info in &documents {
                                    println!("📱 Mobile sync: Server document: {} (updated: {})", doc_info.doc_id, doc_info.updated_at);
                                }
                                // Document discovery: fetch snapshots for unknown server docs
                                let mut server_ids = std::collections::HashSet::new();
                                for doc_info in &documents {
                                    server_ids.insert(doc_info.doc_id.to_string());
                                }
                                for doc_info in &documents {
                                    let doc_id_str = doc_info.doc_id.to_string();
                                    if self.db.get_document(&doc_id_str).ok().flatten().is_none() {
                                        // Request snapshot for unknown doc
                                        let req = lst_proto::ClientMessage::RequestSnapshot { doc_id: doc_info.doc_id };
                                        if let Err(e) = write.send(Message::Text(serde_json::to_string(&req)?)).await {
                                            println!("📱 Mobile sync: Failed requesting snapshot for {}: {}", doc_id_str, e);
                                        }
                                    }
                                }
                                // Seed server with local docs that are missing there
                                if let Ok(local_docs) = self.db.list_all_documents() {
                                    for (doc_id, path, _typ, state, _owner, _w, _r) in local_docs {
                                        if !server_ids.contains(&doc_id) {
                                            if let Ok(uuid) = uuid::Uuid::parse_str(&doc_id) {
                                                // Extract relative path from mobile content directory to preserve structure
                                                let content_dir = dirs::home_dir()
                                                    .map(|h| h.join("Documents").join("lst"))
                                                    .unwrap_or_else(|| std::path::PathBuf::from("."));
                                                
                                                let relative_path = std::path::Path::new(&path)
                                                    .strip_prefix(&content_dir)
                                                    .unwrap_or(std::path::Path::new("unknown.md"))
                                                    .to_string_lossy()
                                                    .to_string();
                                                
                                                // Encrypt relative path before sending
                                                let encrypted_filename = crypto::encrypt(relative_path.as_bytes(), &self.encryption_key)?;
                                                let encoded_filename = base64::engine::general_purpose::STANDARD.encode(&encrypted_filename);
                                                
                                                println!("📱 Mobile sync: Encrypting relative path: {} for doc {}", relative_path, doc_id);
                                                
                                                let msg = lst_proto::ClientMessage::PushSnapshot { 
                                                    doc_id: uuid, 
                                                    filename: encoded_filename,
                                                    snapshot: state 
                                                };
                                                if let Err(e) = write.send(Message::Text(serde_json::to_string(&msg)?)).await {
                                                    println!("📱 Mobile sync: Failed pushing snapshot for {}: {}", doc_id, e);
                                                } else {
                                                    println!("📱 Mobile sync: Seeded server with local doc {} ({})", doc_id, relative_path);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            lst_proto::ServerMessage::Authenticated { success } => {
                                println!("📱 Mobile sync: Auth response: {}", success);
                            }
                            lst_proto::ServerMessage::Snapshot { doc_id, filename, snapshot } => {
                                // Decrypt filename
                                let decrypted_filename = if let Ok(encrypted_bytes) = base64::engine::general_purpose::STANDARD.decode(&filename) {
                                    if let Ok(decrypted_bytes) = crypto::decrypt(&encrypted_bytes, &self.encryption_key) {
                                        String::from_utf8(decrypted_bytes).unwrap_or_else(|_| format!("{}.md", &doc_id.to_string()[..8]))
                                    } else {
                                        format!("{}.md", &doc_id.to_string()[..8])
                                    }
                                } else {
                                    format!("{}.md", &doc_id.to_string()[..8])
                                };
                                
                                println!("📱 Mobile sync: Decrypted filename: {}", decrypted_filename);
                                
                                // Persist snapshot as baseline document
                                let doc_id_str = doc_id.to_string();
                                match self.db.get_document(&doc_id_str) {
                                    Ok(Some((_path, _typ, _hash, _state, owner, writers, readers))) => {
                                        // Already exists; update snapshot
                                        if let Err(e) = self.db.save_document_snapshot(&doc_id_str, &snapshot, Some(owner.as_str()), writers.as_deref(), readers.as_deref()) {
                                            println!("📱 Mobile sync: Failed to save snapshot for {}: {}", doc_id_str, e);
                                        }
                                    }
                                    _ => {
                                        // Use the enhanced method that preserves filename/directory structure
                                        if let Err(e) = self.db.insert_new_document_from_snapshot_with_filename(&doc_id_str, &decrypted_filename, &snapshot) {
                                            println!("📱 Mobile sync: Failed to insert new doc from snapshot {}: {}", doc_id_str, e);
                                        } else {
                                            println!("📱 Mobile sync: Created new document from snapshot: {} -> {}", doc_id_str, decrypted_filename);
                                        }
                                    }
                                }
                            }
                            _ => {} // Ignore other message types
                        }
                    }
                }
                Ok(Some(Ok(Message::Close(_)))) => {
                    println!("📱 Mobile sync: Server closed connection");
                    break;
                }
                Ok(Some(Ok(_))) => {} // Ignore other message types
                Ok(Some(Err(e))) => {
                    println!("📱 Mobile sync: WebSocket error: {}", e);
                    break;
                }
                Ok(None) => {
                    println!("📱 Mobile sync: Connection closed");
                    break;
                }
                Err(_) => break, // Timeout - normal exit
            }
        }

        // Close connection gracefully
        let _ = write.close().await;
        
        if changes_received > 0 {
            println!("📱 Mobile sync: Received {} changes from server", changes_received);
        }
        
        Ok(())
    }

    pub async fn periodic_sync(&mut self) -> Result<()> {
        let interval = self.config.sync_settings.as_ref()
            .map(|s| s.interval_seconds)
            .unwrap_or(30);

        println!("📱 Mobile periodic_sync: interval={}, elapsed={}s", interval, self.last_sync.elapsed().as_secs());

        if self.last_sync.elapsed().as_secs() < interval {
            println!("📱 Mobile sync: Skipping sync - not enough time elapsed");
            return Ok(());
        }

        // Update pending changes count in status
        let pending_count = self.pending_changes.values().map(|v| v.len()).sum::<usize>() as u32;
        sync_status::update_pending_changes(pending_count)?;
        println!("📱 Mobile sync: Current pending changes: {}", pending_count);

        if self.client.is_some() {
            println!("📱 Mobile sync: Client available, performing sync...");
            match self.perform_sync().await {
                Ok(()) => {
                    println!("📱 Mobile sync: ✅ perform_sync completed successfully");
                    sync_status::mark_sync_connected()?;
                }
                Err(e) => {
                    println!("📱 Mobile sync: ❌ perform_sync failed: {}", e);
                    sync_status::mark_sync_disconnected(e.to_string())?;
                    return Err(e);
                }
            }
        } else {
            println!("📱 Mobile sync: ⚠️ No client available - sync not configured");
            sync_status::mark_sync_disconnected("Sync not configured".to_string())?;
        }

        self.last_sync = Instant::now();
        Ok(())
    }

    /// Ensure existing documents are added to pending changes for initial sync
    async fn ensure_initial_sync(&mut self) -> Result<()> {
        println!("📱 Mobile sync: Starting ensure_initial_sync check...");
        match self.db.list_all_documents() {
            Ok(local_docs) => {
                println!("📱 Mobile sync: Found {} local documents for initial sync", local_docs.len());
                let mut added_docs = 0;
                for (doc_id, file_path, _doc_type, state, _, _, _) in local_docs {
                    println!("📱 Mobile sync: Processing doc: {} ({})", doc_id, file_path);
                    // Only add documents that don't already have pending changes
                    if !self.pending_changes.contains_key(&doc_id) {
                        println!("📱 Mobile sync: Adding existing document to pending changes: {} -> {}", doc_id, file_path);
                        // Load the document and get all its changes (full history)
                        match Automerge::load(&state) {
                            Ok(doc) => {
                                let changes = doc
                                    .get_changes(&[])
                                    .into_iter()
                                    .map(|c| c.raw_bytes().to_vec())
                                    .collect::<Vec<_>>();
                                
                                println!("📱 Mobile sync: Document {} has {} changes", doc_id, changes.len());
                                if !changes.is_empty() {
                                    self.pending_changes.insert(doc_id.clone(), changes);
                                    added_docs += 1;
                                } else {
                                    println!("📱 Mobile sync: Warning: Document {} has no changes", doc_id);
                                }
                            }
                            Err(e) => {
                                println!("📱 Mobile sync: Error loading document {}: {}", doc_id, e);
                            }
                        }
                    } else {
                        println!("📱 Mobile sync: Document {} already has pending changes, skipping", doc_id);
                    }
                }
                println!("📱 Mobile sync: Added {} documents to pending changes", added_docs);
            }
            Err(e) => {
                println!("📱 Mobile sync: Error listing documents: {}", e);
            }
        }
        Ok(())
    }

    async fn perform_sync(&mut self) -> Result<()> {
        // Only do initial sync once
        if !self.initial_sync_done {
            self.ensure_initial_sync().await?;
            self.initial_sync_done = true;
        }

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

            println!("📱 Mobile sync: Syncing {} encrypted changes", encrypted_total);
            self.sync_with_server(encrypted).await?;
        } else {
            // still poll server for new changes
            println!("📱 Mobile sync: No pending changes, polling for remote changes");
            self.sync_with_server(HashMap::new()).await?;
        }
        Ok(())
    }
}