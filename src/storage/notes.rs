use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use std::path::PathBuf;
use std::fs;

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

/// Delete a note with the given title (`slug.md`).
pub fn delete_note(title: &str) -> Result<()> {
    let path = get_note_path(title).context("building note path failed")?;

    if !path.exists() {
        // Return a structured error instead of silently creating a new file.
        anyhow::bail!("note `{}` does not exist", title);
    }

    fs::remove_file(&path).with_context(|| format!("could not delete {}", path.display()))?;

    Ok(())
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
/// Append text to a note (with a newline between old and new text).
/// Creates the note if it does not exist.
pub fn append_to_note(title: &str, text: &str) -> Result<PathBuf> {
    let path = get_note_path(title)?;
    if !path.exists() {
        // Create a new note with frontmatter
        create_note(title)?;
    }
    // Append text with preceding blank line
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("Failed to open note file for append: {}", path.display()))?;
    // Write a blank line, the text, and a newline
    use std::io::Write;
    writeln!(file, "\n{}", text)
        .with_context(|| format!("Failed to write to note file: {}", path.display()))?;
    Ok(path)
}
