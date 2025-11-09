use crate::storage;
use anyhow::{anyhow, Context, Result};
use automerge::{transaction::Transactable as _, Automerge, ObjType, ReadDoc, ScalarValue, Value};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Categories supported by the sync layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentKind {
    List,
    Note,
}

impl DocumentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DocumentKind::List => "list",
            DocumentKind::Note => "note",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "list" => DocumentKind::List,
            _ => DocumentKind::Note,
        }
    }
}

/// Canonical representation of a document path.
#[derive(Debug, Clone)]
pub struct CanonicalDocPath {
    pub full_path: PathBuf,
    pub relative_path: String,
    pub kind: DocumentKind,
}

impl CanonicalDocPath {
    /// Generate a deterministic document id from this path.
    pub fn document_id(&self) -> String {
        uuid_from_relative_path(&self.relative_path)
    }
}

/// Resolve a path (absolute or relative) against the configured content directory
/// and derive its canonical metadata used for sync.
pub fn canonicalize_doc_path(path: &Path) -> Result<CanonicalDocPath> {
    let content_dir = storage::get_content_dir()?;

    let full_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        content_dir.join(path)
    };

    // Best-effort normalization; if the path lives outside the content dir we still
    // use the display form to keep IDs stable across platforms.
    let relative_path = match full_path.strip_prefix(&content_dir) {
        Ok(rel) => rel.to_path_buf(),
        Err(_) => {
            // Accept already relative inputs like "lists/foo.md"
            if path.is_relative() {
                PathBuf::from(path)
            } else {
                full_path.clone()
            }
        }
    };

    let relative_str = normalize_relative_path(&relative_path);
    let kind = detect_kind(&relative_str);

    Ok(CanonicalDocPath {
        full_path,
        relative_path: relative_str,
        kind,
    })
}

fn normalize_relative_path(path: &Path) -> String {
    // Convert to forward-slash separated string without leading separators.
    let raw = path.to_string_lossy().replace('\\', "/");
    raw.trim_start_matches('/').to_string()
}

fn detect_kind(relative: &str) -> DocumentKind {
    if relative.starts_with("lists/") || relative == "lists" {
        DocumentKind::List
    } else if relative.starts_with("notes/") || relative == "notes" {
        DocumentKind::Note
    } else {
        // Default to notes; this keeps backwards compatibility with older paths.
        DocumentKind::Note
    }
}

fn uuid_from_relative_path(relative: &str) -> String {
    let normalized = relative.replace('\\', "/");
    Uuid::new_v5(&Uuid::NAMESPACE_OID, normalized.as_bytes()).to_string()
}

/// Apply plain-text content into an Automerge document using the shared schema.
pub fn update_automerge_doc(doc: &mut Automerge, kind: DocumentKind, content: &str) -> Result<()> {
    match kind {
        DocumentKind::List => update_list_doc(doc, content),
        DocumentKind::Note => update_note_doc(doc, content),
    }
}

/// Extract a plain-text representation from an Automerge document using the shared schema.
pub fn extract_automerge_content(doc: &Automerge, kind: DocumentKind) -> Result<String> {
    match kind {
        DocumentKind::List => extract_list_content(doc),
        DocumentKind::Note => extract_note_content(doc),
    }
}

fn update_note_doc(doc: &mut Automerge, content: &str) -> Result<()> {
    let maybe_id = if let Some((Value::Object(_), id)) = doc.get(automerge::ROOT, "content")? {
        Some(id)
    } else {
        None
    };
    let mut tx = doc.transaction();
    let content_id = match maybe_id {
        Some(id) => id,
        None => tx.put_object(&automerge::ROOT, "content", ObjType::Text)?,
    };
    tx.update_text(&content_id, content)?;
    tx.commit();
    Ok(())
}

fn update_list_doc(doc: &mut Automerge, content: &str) -> Result<()> {
    let mut tx = doc.transaction();

    tx.delete(&automerge::ROOT, "items").ok();
    let items_id = tx.put_object(&automerge::ROOT, "items", ObjType::List)?;

    let mut insert_index = 0;
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            tx.insert(&items_id, insert_index, ScalarValue::Str(trimmed.into()))?;
            insert_index += 1;
        }
    }

    tx.commit();
    Ok(())
}

fn extract_note_content(doc: &Automerge) -> Result<String> {
    if let Some((content_val, content_id)) = doc.get(automerge::ROOT, "content")? {
        match content_val {
            Value::Object(_) => Ok(doc.text(&content_id).unwrap_or_default()),
            Value::Scalar(s) => {
                if let ScalarValue::Str(text) = s.as_ref() {
                    Ok(text.to_string())
                } else {
                    Ok(String::new())
                }
            }
        }
    } else {
        Ok(String::new())
    }
}

fn extract_list_content(doc: &Automerge) -> Result<String> {
    if let Some((items_val, items_id)) = doc.get(automerge::ROOT, "items")? {
        if let Value::Object(obj_type) = items_val {
            if obj_type == ObjType::List {
                let mut lines = Vec::new();
                let len = doc.length(&items_id);
                for i in 0..len {
                    if let Some((value, _)) = doc.get(&items_id, i)? {
                        if let Value::Scalar(scalar) = value {
                            if let ScalarValue::Str(text) = scalar.as_ref() {
                                lines.push(text.to_string());
                            }
                        }
                    }
                }
                return Ok(lines.join("\n"));
            }
        }
    }
    Ok(String::new())
}

/// Helper to read file content for a canonical path.
pub fn read_document_bytes(path: &CanonicalDocPath) -> Result<Vec<u8>> {
    std::fs::read(&path.full_path)
        .with_context(|| format!("Failed to read document {}", path.full_path.display()))
}

/// Ensure the parent directory for a canonical path exists.
pub fn ensure_parent_dir(path: &CanonicalDocPath) -> Result<()> {
    if let Some(parent) = path.full_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }
    Ok(())
}

/// Write content to disk for a canonical path.
pub fn write_document(path: &CanonicalDocPath, content: &str) -> Result<()> {
    ensure_parent_dir(path)?;
    std::fs::write(&path.full_path, content).with_context(|| {
        format!(
            "Failed to write document content to {}",
            path.full_path.display()
        )
    })?;
    Ok(())
}

/// Convenience to derive canonical metadata and document id in a single call.
pub fn canonical_path_with_id(path: &Path) -> Result<(CanonicalDocPath, String)> {
    let canonical = canonicalize_doc_path(path)?;
    let doc_id = canonical.document_id();
    Ok((canonical, doc_id))
}

/// Build a canonical path from a stored relative path.
pub fn path_from_relative(relative: &str) -> Result<CanonicalDocPath> {
    let content_dir = storage::get_content_dir()?;
    let full_path = content_dir.join(relative);
    let kind = detect_kind(relative);

    Ok(CanonicalDocPath {
        full_path,
        relative_path: relative.to_string(),
        kind,
    })
}

/// Resolve a canonical path from a filename supplied by the server.
pub fn path_from_server_filename(encoded_relative: &str) -> Result<CanonicalDocPath> {
    if encoded_relative.is_empty() {
        return Err(anyhow!("Empty relative path received from server"));
    }
    path_from_relative(encoded_relative)
}
