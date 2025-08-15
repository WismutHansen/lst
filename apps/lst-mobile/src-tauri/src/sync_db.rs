use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;
use automerge::{Automerge, ObjType, ReadDoc};
use std::path::PathBuf;

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
    ) -> Result<
        Option<(
            String,
            String,
            String,
            Vec<u8>,
            String,
            Option<String>,
            Option<String>,
        )>,
    > {
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

    /// Convenience alias used by mobile_sync code to check presence/state
    pub fn get_document_state(&self, doc_id: &str) -> Result<Option<Vec<u8>>> {
        let mut stmt = self
            .conn
            .prepare("SELECT automerge_state FROM documents WHERE doc_id = ?1")?;
        let mut rows = stmt.query(params![doc_id])?;
        if let Some(row) = rows.next()? {
            let state: Vec<u8> = row.get(0)?;
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }

    /// Save/overwrite the local snapshot (automerge_state) and optionally update ACL metadata.
    pub fn save_document_snapshot(
        &self,
        doc_id: &str,
        snapshot: &[u8],
        owner: Option<&str>,
        writers: Option<&str>,
        readers: Option<&str>,
    ) -> Result<()> {
        // Update existing row if present, otherwise insert a minimal one.
        let exists: bool = {
            let mut stmt = self
                .conn
                .prepare("SELECT 1 FROM documents WHERE doc_id = ?1 LIMIT 1")?;
            let mut rows = stmt.query(params![doc_id])?;
            rows.next()?.is_some()
        };

        if exists {
            self.conn.execute(
                "UPDATE documents SET automerge_state = ?2, owner = COALESCE(?3, owner), writers = COALESCE(?4, writers), readers = COALESCE(?5, readers) WHERE doc_id = ?1",
                params![doc_id, snapshot, owner, writers, readers],
            )?;
        } else {
            // Provide sane defaults for missing columns; caller can backfill later via upsert.
            self.conn.execute(
                "INSERT INTO documents (doc_id, file_path, doc_type, last_sync_hash, automerge_state, owner, writers, readers)
                 VALUES (?1, '', 'unknown', '', ?2, COALESCE(?3, ''), ?4, ?5)",
                params![doc_id, snapshot, owner, writers, readers],
            )?;
        }
        Ok(())
    }

    /// Insert a new document from a snapshot if it doesn't already exist; no-op if present.
    pub fn insert_new_document_from_snapshot(&self, doc_id: &str, snapshot: &[u8]) -> Result<()> {
        // Load the Automerge document from snapshot
        let doc = Automerge::load(snapshot)?;
        
        // Extract content from the document
        let content = self.extract_content_from_automerge(&doc)?;
        
        // Determine document type and generate file path
        let (doc_type, file_path) = self.generate_file_path_for_document(doc_id, &content)?;
        
        // Write content to file
        if let Err(e) = std::fs::write(&file_path, &content) {
            eprintln!("📱 Failed to write file {}: {}", file_path, e);
            return Err(anyhow::anyhow!("Failed to write file: {}", e));
        }
        
        println!("📱 Created file from snapshot: {} -> {}", doc_id, file_path);
        
        // Store in database
        self.conn.execute(
            "INSERT OR IGNORE INTO documents (doc_id, file_path, doc_type, last_sync_hash, automerge_state, owner, writers, readers)
             VALUES (?1, ?2, ?3, '', ?4, '', NULL, NULL)",
            params![doc_id, file_path, doc_type, snapshot],
        )?;
        Ok(())
    }

    /// Insert a new document from snapshot with the original filename/path
    pub fn insert_new_document_from_snapshot_with_filename(&self, doc_id: &str, relative_path: &str, snapshot: &[u8]) -> Result<()> {
        // Load the Automerge document from snapshot
        let doc = Automerge::load(snapshot)?;
        
        // Extract content from the document
        let content = self.extract_content_from_automerge(&doc)?;
        
        // Get mobile content directory (different from desktop storage)
        let content_dir = self.get_mobile_content_dir()?;
        let file_path = content_dir.join(relative_path);
        
        // Ensure parent directory exists (creating full nested structure)
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
            println!("📱 Created directory structure: {}", parent.display());
        }
        
        // Write content to file
        if let Err(e) = std::fs::write(&file_path, &content) {
            eprintln!("📱 Failed to write file {}: {}", file_path.display(), e);
            return Err(anyhow::anyhow!("Failed to write file: {}", e));
        }
        
        println!("📱 Created file from snapshot with original path: {} -> {}", doc_id, file_path.display());
        
        // Determine document type from path
        let doc_type = if relative_path.starts_with("lists/") || relative_path.contains("/lists/") {
            "list"
        } else {
            "note"
        };
        
        // Store in database with full path
        self.conn.execute(
            "INSERT OR IGNORE INTO documents (doc_id, file_path, doc_type, last_sync_hash, automerge_state, owner, writers, readers)
             VALUES (?1, ?2, ?3, '', ?4, '', NULL, NULL)",
            params![doc_id, file_path.to_string_lossy().to_string(), doc_type, snapshot],
        )?;
        Ok(())
    }

    /// Extract content from an Automerge document
    fn extract_content_from_automerge(&self, doc: &Automerge) -> Result<String> {
        // Check if this is a list document (has "items" array)
        if let Ok(Some((items_value, items_id))) = doc.get(&automerge::ROOT, "items") {
            if let automerge::Value::Object(obj_type) = items_value {
                if obj_type == ObjType::List {
                    // Extract list items
                    let mut content = String::new();
                    let length = doc.length(&items_id);
                    for i in 0..length {
                        if let Ok(Some((value, _))) = doc.get(&items_id, i) {
                            if let automerge::Value::Scalar(s) = value {
                                if let automerge::ScalarValue::Str(text) = s.as_ref() {
                                    content.push_str(text);
                                    content.push('\n');
                                }
                            }
                        }
                    }
                    return Ok(content);
                }
            }
        }
        
        // Check if this is a note document (has "content" field)
        if let Ok(Some((value, _))) = doc.get(&automerge::ROOT, "content") {
            if let automerge::Value::Scalar(s) = value {
                if let automerge::ScalarValue::Str(text) = s.as_ref() {
                    return Ok(text.to_string());
                }
            }
        }
        
        // Fallback: empty content
        Ok(String::new())
    }
    
    /// Generate appropriate file path for a document
    fn generate_file_path_for_document(&self, doc_id: &str, content: &str) -> Result<(String, String)> {
        let content_dir = self.get_mobile_content_dir()?;
        
        // Determine if this is a list or note based on content structure
        let doc_type = if content.lines().any(|line| !line.trim().is_empty()) && 
                          content.lines().all(|line| line.len() < 200) {
            "list"
        } else {
            "note"
        };
        
        // Extract meaningful filename from content
        let filename = self.extract_filename_from_content(content, doc_id);
        
        let subdir = if doc_type == "list" { "lists" } else { "notes" };
        let file_path = content_dir.join(subdir).join(&filename);
        
        // Ensure parent directory exists
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        Ok((doc_type.to_string(), file_path.to_string_lossy().to_string()))
    }
    
    /// Extract a meaningful filename from document content
    fn extract_filename_from_content(&self, content: &str, fallback_doc_id: &str) -> String {
        // Try to extract title from first non-empty line
        let first_line = content
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("")
            .trim();
        
        if !first_line.is_empty() {
            // Clean up the title to make it filename-safe
            let mut filename = first_line
                .chars()
                .take(50) // Limit length
                .map(|c| match c {
                    'a'..='z' | 'A'..='Z' | '0'..='9' => c,
                    ' ' | '-' | '_' => '-',
                    _ => '_',
                })
                .collect::<String>()
                .trim_matches('-')
                .trim_matches('_')
                .to_lowercase();
            
            // Remove multiple consecutive dashes/underscores
            while filename.contains("--") {
                filename = filename.replace("--", "-");
            }
            while filename.contains("__") {
                filename = filename.replace("__", "_");
            }
            
            // Ensure filename is not empty after cleaning
            if !filename.is_empty() {
                return format!("{}.md", filename);
            }
        }
        
        // Fallback to doc_id if no meaningful title found
        format!("{}.md", &fallback_doc_id[..8])
    }

    /// Get mobile content directory
    fn get_mobile_content_dir(&self) -> Result<PathBuf> {
        // For mobile, we'll use a simple documents directory structure
        // This should be configured per platform but for now we'll use a standard path
        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
        let content_dir = home_dir.join("Documents").join("lst");
        
        std::fs::create_dir_all(&content_dir)?;
        Ok(content_dir)
    }

    /// Delete a document by id
    #[allow(dead_code)]
    pub fn delete_document(&self, doc_id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM documents WHERE doc_id = ?1", params![doc_id])?;
        Ok(())
    }

    /// List all documents in the local database
    pub fn list_all_documents(&self) -> Result<Vec<(String, String, String, Vec<u8>, String, Option<String>, Option<String>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT doc_id, file_path, doc_type, automerge_state, owner, writers, readers FROM documents",
        )?;
        let mut rows = stmt.query([])?;
        let mut documents = Vec::new();
        while let Some(row) = rows.next()? {
            documents.push((
                row.get(0)?, // doc_id
                row.get(1)?, // file_path
                row.get(2)?, // doc_type
                row.get(3)?, // automerge_state
                row.get(4)?, // owner
                row.get(5)?, // writers
                row.get(6)?, // readers
            ));
        }
        Ok(documents)
    }
}
