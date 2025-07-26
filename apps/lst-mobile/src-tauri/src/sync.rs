use crate::crypto;
use crate::sync_db::LocalDb;
use crate::sync_status;
use anyhow::{Context, Result};
use automerge::{
    transaction::Transactable as _, Automerge, Change, ObjType, ReadDoc, ScalarValue, Value,
};
use futures_util::{SinkExt, StreamExt};
use lst_cli::config::Config;
use notify::Event;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::{timeout, Instant};
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
    client: Option<reqwest::Client>,
    db: LocalDb,
    encryption_key: [u8; 32],
    last_sync: Instant,
    pending_changes: HashMap<String, Vec<Vec<u8>>>,
}

impl SyncManager {
    pub async fn new(config: Config) -> Result<Self> {
        let client = if config.syncd.as_ref().and_then(|s| s.url.as_ref()).is_some() {
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

        let db_path = config
            .syncd
            .as_ref()
            .and_then(|s| s.database_path.as_ref())
            .expect("database_path must be set in syncd config");

        let db = LocalDb::new(db_path)?;

        let key_path = config
            .syncd
            .as_ref()
            .and_then(|s| s.encryption_key_ref.as_ref())
            .expect("encryption_key_ref must be set in syncd config");
        let encryption_key = crypto::load_key(std::path::Path::new(key_path))?;

        Ok(Self {
            config,
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
                .config
                .syncd
                .as_ref()
                .and_then(|s| s.device_id.as_ref())
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

    /// Connect to the sync server and exchange changes
    async fn sync_with_server(&mut self, encrypted: HashMap<String, Vec<Vec<u8>>>) -> Result<()> {
        let syncd = match &self.config.syncd {
            Some(s) => s,
            None => return Err(anyhow::anyhow!("Sync not configured"));
        };

        let url = match &syncd.url {
            Some(u) => u,
            None => return Err(anyhow::anyhow!("Server URL not configured"));
        };
        
        let token = self
            .config
            .get_jwt()
            .context("No valid JWT token. Please authenticate first")?
            .to_string();

        let device_id = syncd
            .device_id
            .clone()
            .unwrap_or_else(|| "unknown".to_string());

        // Add connection timeout and retry logic
        let connection_result = timeout(Duration::from_secs(10), connect_async(url)).await;
        let (ws, _) = match connection_result {
            Ok(Ok(ws)) => ws,
            Ok(Err(e)) => return Err(anyhow::anyhow!("Failed to connect to server: {}", e)),
            Err(_) => return Err(anyhow::anyhow!("Connection timeout")),
        };

        let (mut write, mut read) = ws.split();

        // Authenticate with server
        let auth_msg = lst_proto::ClientMessage::Authenticate { jwt: token.clone() };
        if let Err(e) = write.send(Message::Text(serde_json::to_string(&auth_msg)?)).await {
            return Err(anyhow::anyhow!("Failed to send authentication: {}", e));
        }

        // Wait for authentication response
        match timeout(Duration::from_secs(10), read.next()).await {
            Ok(Some(Ok(Message::Text(txt)))) => {
                if let Ok(lst_proto::ServerMessage::Authenticated { success }) =
                    serde_json::from_str(&txt)
                {
                    if !success {
                        return Err(anyhow::anyhow!("Server authentication failed"));
                    }
                } else {
                    return Err(anyhow::anyhow!("Invalid authentication response"));
                }
            }
            Ok(Some(Ok(_))) => return Err(anyhow::anyhow!("Unexpected message type")),
            Ok(Some(Err(e))) => return Err(anyhow::anyhow!("WebSocket error: {}", e)),
            Ok(None) => return Err(anyhow::anyhow!("Connection closed during authentication")),
            Err(_) => return Err(anyhow::anyhow!("Authentication timeout")),
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
                            lst_proto::ServerMessage::NewChanges { doc_id, changes, .. } => {
                                self.apply_remote_changes(&doc_id.to_string(), changes).await?;
                                changes_received += 1;
                            }
                            _ => {} // Ignore other message types
                        }
                    }
                }
                Ok(Some(Ok(Message::Close(_)))) => {
                    println!("Server closed connection");
                    break;
                }
                Ok(Some(Ok(_))) => {} // Ignore other message types
                Ok(Some(Err(e))) => {
                    println!("WebSocket error: {}", e);
                    break;
                }
                Ok(None) => {
                    println!("Connection closed");
                    break;
                }
                Err(_) => break, // Timeout - normal exit
            }
        }

        // Close connection gracefully
        let _ = write.close().await;
        
        if changes_received > 0 {
            println!("Received {} changes from server", changes_received);
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

        // Update pending changes count in status
        let pending_count = self.pending_changes.values().map(|v| v.len()).sum::<usize>() as u32;
        sync_status::update_pending_changes(pending_count)?;

        if self.client.is_some() {
            match self.perform_sync().await {
                Ok(()) => {
                    sync_status::mark_sync_connected()?;
                }
                Err(e) => {
                    sync_status::mark_sync_disconnected(e.to_string())?;
                    return Err(e);
                }
            }
        } else {
            sync_status::mark_sync_disconnected("Sync not configured".to_string())?;
        }

        self.last_sync = Instant::now();
        Ok(())
    }

    async fn perform_sync(&mut self) -> Result<()> {
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
        Ok(())
    }
}
