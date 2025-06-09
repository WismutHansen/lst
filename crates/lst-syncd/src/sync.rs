use crate::config::Config;
use crate::database::LocalDb;
use anyhow::{Context, Result};
use automerge::{transaction::Transactable as _, Automerge, Change, ReadDoc};
use notify::Event;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tokio::time::Instant;

pub struct SyncManager {
    config: Config,
    client: Option<reqwest::Client>,
    db: LocalDb,
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

        Ok(Self {
            config,
            client,
            db,
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

            let doc_type = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown");

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

                let mut tx = doc.transaction();
                tx.put(automerge::ROOT, "content", "").ok();
                tx.update_text(automerge::ROOT, "content", &new_content)?;
                tx.commit();

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
                let mut tx = doc.transaction();
                tx.put(automerge::ROOT, "content", "").ok();
                tx.update_text(automerge::ROOT, "content", &new_content)?;
                tx.commit();

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
                        println!(
                            "Would sync {} pending changes to {}",
                            self.pending_changes.len(),
                            server_url
                        );

                        // Placeholder for actual networking
                        let _ = client;
                    }
                }
            }
        }

        self.last_sync = Instant::now();

        Ok(())
    }
}
