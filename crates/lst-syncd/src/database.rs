use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::path::Path;

/// Local SQLite database used by lst-syncd
pub struct LocalDb {
    conn: Connection,
}

impl LocalDb {
    /// Open the database at the given path and initialize tables if needed
    pub fn new(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create db directory: {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open database: {}", path.display()))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS documents (
                doc_id TEXT PRIMARY KEY,
                file_path TEXT NOT NULL UNIQUE,
                doc_type TEXT NOT NULL,
                last_sync_hash TEXT,
                automerge_state BLOB NOT NULL
            );",
        )?
        ;
        Ok(Self { conn })
    }

    /// Insert or update a document row
    pub fn upsert_document(&self, doc_id: &str, file_path: &str, doc_type: &str, last_sync_hash: &str, state: &[u8]) -> Result<()> {
        self.conn.execute(
            "INSERT INTO documents (doc_id, file_path, doc_type, last_sync_hash, automerge_state)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(doc_id) DO UPDATE SET
                file_path = excluded.file_path,
                doc_type = excluded.doc_type,
                last_sync_hash = excluded.last_sync_hash,
                automerge_state = excluded.automerge_state",
            params![doc_id, file_path, doc_type, last_sync_hash, state],
        )?;
        Ok(())
    }
}
