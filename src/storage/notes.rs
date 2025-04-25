use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use std::fs;
use std::path::PathBuf;

/// Simple slugify: lowercase, replace non-alphanumeric with '-', trim hyphens
fn slugify(title: &str) -> String {
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else if c.is_whitespace() {
                '-'
            } else {
                '-'
            }
        })
        .collect();
    slug.trim_matches('-').to_string()
}

/// Return the path for a note with given title (slug.md)
pub fn get_note_path(title: &str) -> Result<PathBuf> {
    let notes_dir = super::get_notes_dir()?;
    let filename = format!("{}.md", slugify(title));
    Ok(notes_dir.join(filename))
}

/// Create a new note file with frontmatter and return its path
pub fn create_note(title: &str) -> Result<PathBuf> {
    let path = get_note_path(title)?;
    if path.exists() {
        return Err(anyhow!("Note '{}' already exists", title));
    }
    // Build frontmatter
    let now = Utc::now().to_rfc3339();
    let content = format!("---\ntitle: \"{}\"\ncreated: {}\n---\n\n", title, now);
    fs::write(&path, content)
        .with_context(|| format!("Failed to create note file: {}", path.display()))?;
    Ok(path)
}

/// Ensure note exists and return its path
pub fn load_note(title: &str) -> Result<PathBuf> {
    let path = get_note_path(title)?;
    if !path.exists() {
        return Err(anyhow!("Note '{}' does not exist", title));
    }
    Ok(path)
}