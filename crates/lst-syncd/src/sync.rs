use crate::config::Config;
use crate::database::LocalDb;
use crate::crypto;
use anyhow::{Context, Result};
use automerge::{
    transaction::Transactable as _,
    Automerge, Change, ObjType, ReadDoc, ScalarValue, Value,
};
use notify::Event;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tokio::time::Instant;

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
    tx.put(automerge::ROOT, "content", "").ok();
    tx.update_text(automerge::ROOT, "content", content)?;
    tx.commit();
    Ok(())
}

fn update_list_doc(doc: &mut Automerge, content: &str) -> Result<()> {
    let mut tx = doc.transaction();
    let items = match doc.get(automerge::ROOT, "items")? {
        Some((id, _)) => id,
        None => tx.put_object(automerge::ROOT, "items", ObjType::List)?,
    };

    let len = doc.length(items);
    for idx in (0..len).rev() {
        tx.delete(items, idx)?;
    }

    for (idx, line) in content.lines().enumerate() {
        let line = line.trim();
        if !line.is_empty() {
            tx.insert(items, idx, ScalarValue::Str(line.into()))?;
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
        let encryption_key = crypto::load_key(key_path)?;

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
                let old_heads = doc.get_heads();

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

                let changes = doc
                    .get_changes_added(&old_heads)
                    .into_iter()
                    .map(|c| c.raw_bytes().to_vec())
                    .collect::<Vec<_>>();

                self.pending_changes
                    .entry(doc_id)
                    .or_insert_with(Vec::new)
                    .extend(changes);
            } else {
                let mut doc = Automerge::new();
                let old_heads = doc.get_heads();

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

                let changes = doc
                    .get_changes_added(&old_heads)
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
                let change = Change::from_bytes(&decrypted)?;
                change_objs.push(change);
            }

            doc.apply_changes(change_objs)?;

            let new_state = doc.save();

            let content = if doc_type == "list" {
                if let Some((items, _)) = doc.get(automerge::ROOT, "items")? {
                    let mut lines = Vec::new();
                    for i in 0..doc.length(items) {
                        if let Some((_id, val)) = doc.get(items, i)? {
                            match val {
                                Value::Text(t) => lines.push(t.to_string()),
                                Value::Scalar(ScalarValue::Str(s)) => lines.push(s.to_string()),
                                _ => {}
                            }
                        }
                    }
                    lines.join("\n")
                } else {
                    String::new()
                }
            } else if let Some((_id, value)) = doc.get(automerge::ROOT, "content")? {
                match value {
                    Value::Text(t) => t.to_string(),
                    _ => String::new(),
                }
            } else {
                String::new()
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

        if let Some(ref client) = self.client {
            if let Some(ref syncd) = self.config.syncd {
                if let Some(ref server_url) = syncd.url {
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
                        println!(
                            "Would sync {encrypted_total} encrypted changes to {server_url}"
                        );

                        // Placeholder for actual networking
                        let _ = (client, encrypted);
                    }
                }
            }
        }

        self.last_sync = Instant::now();

        Ok(())
    }
}
