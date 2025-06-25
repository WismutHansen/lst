use anyhow::{Context, Result};
use rusqlite::{params, Connection};
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
                automerge_state BLOB NOT NULL,
                owner TEXT NOT NULL,
                writers TEXT,
                readers TEXT
            );",
        )?;
        Ok(Self { conn })
    }

    /// Insert or update a document row
    pub fn upsert_document(
        &self,
        doc_id: &str,
        file_path: &str,
        doc_type: &str,
        last_sync_hash: &str,
        state: &[u8],
        owner: &str,
        writers: Option<&str>,
        readers: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO documents (doc_id, file_path, doc_type, last_sync_hash, automerge_state, owner, writers, readers)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(doc_id) DO UPDATE SET
                file_path = excluded.file_path,
                doc_type = excluded.doc_type,
                last_sync_hash = excluded.last_sync_hash,
                automerge_state = excluded.automerge_state,
                owner = excluded.owner,
                writers = excluded.writers,
                readers = excluded.readers",
            params![doc_id, file_path, doc_type, last_sync_hash, state, owner, writers, readers],
        )?;
        Ok(())
    }

    /// Fetch a document row by doc_id
    pub fn get_document(
        &self,
        doc_id: &str,
    ) -> Result<Option<(String, String, String, Vec<u8>, String, Option<String>, Option<String>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT file_path, doc_type, last_sync_hash, automerge_state, owner, writers, readers FROM documents WHERE doc_id = ?1",
        )?;
        let mut rows = stmt.query(params![doc_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            )))
        } else {
            Ok(None)
        }
    }

    /// Delete a document by id
    pub fn delete_document(&self, doc_id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM documents WHERE doc_id = ?1", params![doc_id])?;
        Ok(())
    }

    
}
