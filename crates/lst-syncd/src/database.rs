use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;
use automerge::{Automerge, ObjType, ReadDoc};
use lst_core::storage;

/// Local SQLite database used by lst-syncd
pub struct LocalDb {
    pub(crate) conn: Connection,
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

    /// Generate a unique file path if the provided one already exists
    fn ensure_unique_file_path(&self, preferred_path: &str, doc_id: &str) -> Result<String> {
        // Check if path is already taken by a different document
        let mut stmt = self.conn.prepare("SELECT doc_id FROM documents WHERE file_path = ?")?;
        if let Ok(existing_doc_id) = stmt.query_row([preferred_path], |row| {
            let existing_id: String = row.get(0)?;
            Ok(existing_id)
        }) {
            // If same doc_id, we can reuse the path
            if existing_doc_id == doc_id {
                return Ok(preferred_path.to_string());
            }
            // Different doc_id, need to generate unique path
            let path = std::path::Path::new(preferred_path);
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let extension = path.extension().unwrap_or_default().to_string_lossy();
            let parent = path.parent().unwrap_or(std::path::Path::new(""));
            
            // Try with doc_id suffix
            let unique_name = if extension.is_empty() {
                format!("{}_{}", stem, &doc_id[..8])
            } else {
                format!("{}_{}.{}", stem, &doc_id[..8], extension)
            };
            let unique_path = parent.join(unique_name);
            Ok(unique_path.to_string_lossy().to_string())
        } else {
            // Path is not taken, can use as-is
            Ok(preferred_path.to_string())
        }
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
        // Ensure we have a unique file path
        let unique_file_path = self.ensure_unique_file_path(file_path, doc_id)?;
        
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
            params![doc_id, unique_file_path, doc_type, last_sync_hash, state, owner, writers, readers],
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

    /// Save/overwrite snapshot bytes for a doc, inserting if absent
    pub fn save_document_snapshot(
        &self,
        doc_id: &str,
        snapshot: &[u8],
        owner: Option<&str>,
        writers: Option<&str>,
        readers: Option<&str>,
    ) -> Result<()> {
        // Check if exists
        let exists: bool = {
            let mut stmt = self.conn.prepare("SELECT 1 FROM documents WHERE doc_id = ?1 LIMIT 1")?;
            let mut rows = stmt.query(params![doc_id])?;
            rows.next()?.is_some()
        };
        if exists {
            self.conn.execute(
                "UPDATE documents SET automerge_state = ?2, owner = COALESCE(?3, owner), writers = COALESCE(?4, writers), readers = COALESCE(?5, readers) WHERE doc_id = ?1",
                params![doc_id, snapshot, owner, writers, readers],
            )?;
        } else {
            self.conn.execute(
                "INSERT INTO documents (doc_id, file_path, doc_type, last_sync_hash, automerge_state, owner, writers, readers)
                 VALUES (?1, '', 'unknown', '', ?2, COALESCE(?3, ''), ?4, ?5)",
                params![doc_id, snapshot, owner, writers, readers],
            )?;
        }
        Ok(())
    }

    /// Insert new doc from snapshot if missing
    pub fn insert_new_document_from_snapshot(&self, doc_id: &str, snapshot: &[u8]) -> Result<()> {
        // Load the Automerge document from snapshot
        let doc = Automerge::load(snapshot)?;
        
        // Extract content from the document
        let content = self.extract_content_from_automerge(&doc)?;
        
        // Determine document type and generate file path
        let (doc_type, file_path) = self.generate_file_path_for_document(doc_id, &content)?;
        
        // Write content to file
        if let Err(e) = std::fs::write(&file_path, &content) {
            eprintln!("Failed to write file {}: {}", file_path, e);
            return Err(anyhow::anyhow!("Failed to write file: {}", e));
        }
        
        println!("DEBUG: Created file from snapshot: {} -> {}", doc_id, file_path);
        
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
        
        // Reconstruct the full path using the relative path
        let content_dir = storage::get_content_dir()?;
        let file_path = content_dir.join(relative_path);
        
        // Ensure parent directory exists (creating full nested structure)
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
            println!("DEBUG: Created directory structure: {}", parent.display());
        }
        
        // Write content to file
        if let Err(e) = std::fs::write(&file_path, &content) {
            eprintln!("Failed to write file {}: {}", file_path.display(), e);
            return Err(anyhow::anyhow!("Failed to write file: {}", e));
        }
        
        println!("DEBUG: Created file from snapshot with original path: {} -> {}", doc_id, file_path.display());
        
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
        let content_dir = storage::get_content_dir()?;
        
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

    /// List all docs
    pub fn list_all_documents(&self) -> Result<Vec<(String, String, String, Vec<u8>, String, Option<String>, Option<String>)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT doc_id, file_path, doc_type, automerge_state, owner, writers, readers FROM documents")?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ));
        }
        Ok(out)
    }
}