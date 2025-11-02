use anyhow::{Context, Result};
use automerge::{Automerge, ObjType, ReadDoc, Value};
use lst_core::sync::{
    extract_automerge_content, path_from_relative, path_from_server_filename, write_document,
    CanonicalDocPath, DocumentKind,
};
use rusqlite::{params, Connection};
use std::path::Path;

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

    /// Validate and fix file paths that might be incomplete
    fn fix_incomplete_file_path(path: &str, doc_type: &str) -> String {
        let path_obj = std::path::Path::new(path);

        // If it's just a directory name (no extension), add appropriate extension
        if path_obj.extension().is_none() {
            let filename = path_obj.file_name().unwrap_or_default().to_string_lossy();

            // Skip if it's just a directory like "lists" or "notes"
            if filename == "lists" || filename == "notes" || filename == "content" {
                eprintln!("WARNING: Skipping bare directory name: {}", filename);
                return format!("_invalid_/{}.md", filename); // Put in invalid folder
            }

            // Add appropriate extension based on doc type
            let extension = match doc_type {
                "list" => ".md",
                "note" => ".md",
                _ => ".md",
            };

            format!("{}{}", path, extension)
        } else {
            path.to_string()
        }
    }

    /// Validate that a path represents a file, not a directory
    fn validate_file_path(path: &str) -> Result<()> {
        let path_obj = std::path::Path::new(path);

        // Skip paths that start with _invalid_
        if path_obj.starts_with("_invalid_") {
            return Err(anyhow::anyhow!("Path '{}' is marked as invalid", path));
        }

        // Check if it's just a directory name (no extension and no parent with extension)
        if path_obj.extension().is_none() {
            // Allow if it has a parent that suggests it's a file (e.g., "notes/something")
            if let Some(parent) = path_obj.parent() {
                if parent.to_string_lossy().is_empty() {
                    return Err(anyhow::anyhow!(
                        "Path '{}' appears to be a directory, not a file",
                        path
                    ));
                }
            } else {
                return Err(anyhow::anyhow!(
                    "Path '{}' appears to be a directory, not a file",
                    path
                ));
            }
        }
        Ok(())
    }

    /// Generate a unique file path if the provided one already exists
    fn ensure_unique_file_path(&self, preferred_path: &str, doc_id: &str) -> Result<String> {
        // Check if path is already taken by a different document
        let mut stmt = self
            .conn
            .prepare("SELECT doc_id FROM documents WHERE file_path = ?")?;
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
        // Fix incomplete file paths
        let fixed_file_path = Self::fix_incomplete_file_path(file_path, doc_type);

        // Validate that this is actually a file path, not a directory
        if let Err(e) = Self::validate_file_path(&fixed_file_path) {
            eprintln!(
                "WARNING: Skipping invalid file path for doc {}: {}",
                doc_id, e
            );
            return Ok(()); // Skip this document rather than fail
        }

        // Ensure we have a unique file path
        let unique_file_path = self.ensure_unique_file_path(&fixed_file_path, doc_id)?;

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

    /// Look up a document id by full file path
    pub fn get_doc_id_by_file_path(&self, file_path: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT doc_id FROM documents WHERE file_path = ?1 LIMIT 1")?;
        let mut rows = stmt.query(params![file_path])?;
        if let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            Ok(Some(id))
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
            self.conn.execute(
                "INSERT INTO documents (doc_id, file_path, doc_type, last_sync_hash, automerge_state, owner, writers, readers)
                 VALUES (?1, '', 'unknown', '', ?2, COALESCE(?3, ''), ?4, ?5)",
                params![doc_id, snapshot, owner, writers, readers],
            )?;
        }
        Ok(())
    }

    /// Insert new doc from snapshot if missing
    #[allow(dead_code)]
    pub fn insert_new_document_from_snapshot(&self, doc_id: &str, snapshot: &[u8]) -> Result<()> {
        let doc = Automerge::load(snapshot)?;
        let doc_kind = Self::detect_kind_from_doc(&doc);
        let content = extract_automerge_content(&doc, doc_kind)?;
        let canonical = self.generate_file_path_for_document(doc_id, doc_kind, &content)?;

        write_document(&canonical, &content)?;
        println!(
            "DEBUG: Created file from snapshot: {} -> {}",
            doc_id,
            canonical.full_path.display()
        );

        self.conn.execute(
            "INSERT OR IGNORE INTO documents (doc_id, file_path, doc_type, last_sync_hash, automerge_state, owner, writers, readers)
             VALUES (?1, ?2, ?3, '', ?4, '', NULL, NULL)",
            params![
                doc_id,
                canonical.full_path.to_string_lossy().to_string(),
                doc_kind.as_str(),
                snapshot
            ],
        )?;
        Ok(())
    }

    /// Insert a new document from snapshot with the original filename/path
    pub fn insert_new_document_from_snapshot_with_filename(
        &self,
        doc_id: &str,
        relative_path: &str,
        snapshot: &[u8],
    ) -> Result<()> {
        let doc = Automerge::load(snapshot)?;
        let doc_kind = Self::detect_kind_from_doc(&doc);
        let content = extract_automerge_content(&doc, doc_kind)?;
        let canonical = path_from_server_filename(relative_path)?;

        write_document(&canonical, &content)?;
        println!(
            "DEBUG: Created file from snapshot with original path: {} -> {}",
            doc_id,
            canonical.full_path.display()
        );

        self.conn.execute(
            "INSERT OR IGNORE INTO documents (doc_id, file_path, doc_type, last_sync_hash, automerge_state, owner, writers, readers)
             VALUES (?1, ?2, ?3, '', ?4, '', NULL, NULL)",
            params![
                doc_id,
                canonical.full_path.to_string_lossy().to_string(),
                doc_kind.as_str(),
                snapshot
            ],
        )?;
        Ok(())
    }

    /// Extract content from an Automerge document
    #[allow(dead_code)]
    fn generate_file_path_for_document(
        &self,
        doc_id: &str,
        kind: DocumentKind,
        content: &str,
    ) -> Result<CanonicalDocPath> {
        let filename = self.extract_filename_from_content(content, doc_id);
        let subdir = match kind {
            DocumentKind::List => "lists",
            DocumentKind::Note => "notes",
        };
        let relative = format!("{}/{}", subdir, filename);
        path_from_relative(&relative)
    }

    fn detect_kind_from_doc(doc: &Automerge) -> DocumentKind {
        if let Ok(Some((value, _))) = doc.get(&automerge::ROOT, "items") {
            if let Value::Object(obj_type) = value {
                if obj_type == ObjType::List {
                    return DocumentKind::List;
                }
            }
        }
        DocumentKind::Note
    }

    /// Extract a meaningful filename from document content
    #[allow(dead_code)]
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
    pub fn list_all_documents(
        &self,
    ) -> Result<
        Vec<(
            String,
            String,
            String,
            Vec<u8>,
            String,
            Option<String>,
            Option<String>,
        )>,
    > {
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
