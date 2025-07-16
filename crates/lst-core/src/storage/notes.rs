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

/// Return the path for a note with given title (supports directory paths)
pub fn get_note_path(title: &str) -> Result<PathBuf> {
    let notes_dir = super::get_notes_dir()?;
    
    // If title contains path separators, treat as directory path
    if title.contains('/') || title.contains('\\') {
        let filename = format!("{}.md", title);
        Ok(notes_dir.join(filename))
    } else {
        // Single filename - try to find via fuzzy search first
        if let Ok(resolved_path) = resolve_note_path(title) {
            return Ok(resolved_path);
        }
        
        // Fallback to creating in root notes directory
        let filename = format!("{}.md", slugify(title));
        Ok(notes_dir.join(filename))
    }
}

/// Resolve a note by title using fuzzy search (filename only)
pub fn resolve_note_path(title: &str) -> Result<PathBuf> {
    let entries = super::list_notes_with_info()?;
    
    // First try exact filename match
    for entry in &entries {
        if entry.name == title {
            return Ok(entry.full_path.clone());
        }
    }
    
    // Then try fuzzy match by filename
    let matches: Vec<&super::FileEntry> = entries
        .iter()
        .filter(|entry| entry.name.contains(title))
        .collect();
    
    match matches.len() {
        0 => anyhow::bail!("Note '{}' does not exist", title),
        1 => Ok(matches[0].full_path.clone()),
        _ => {
            let match_names: Vec<String> = matches.iter().map(|e| e.relative_path.clone()).collect();
            anyhow::bail!("Multiple notes match '{}': {:?}", title, match_names);
        }
    }
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
    let notes_dir = super::get_notes_dir()?;
    let filename = format!("{}.md", title);
    let path = notes_dir.join(&filename);
    
    if path.exists() {
        return Err(anyhow!("Note '{}' already exists", title));
    }
    
    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }
    }
    
    // Extract just the filename for the note title (not the full path)
    let note_title = if title.contains('/') || title.contains('\\') {
        std::path::Path::new(title)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(title)
            .to_string()
    } else {
        title.to_string()
    };
    
    // Build frontmatter
    let now = Utc::now().to_rfc3339();
    let content = format!("---\ntitle: \"{}\"\ncreated: {}\n---\n\n", note_title, now);
    fs::write(&path, content)
        .with_context(|| format!("Failed to create note file: {}", path.display()))?;
    Ok(path)
}

/// Ensure note exists and return its path
pub fn load_note(title: &str) -> Result<PathBuf> {
    // Try direct path resolution first
    if title.contains('/') || title.contains('\\') {
        let notes_dir = super::get_notes_dir()?;
        let filename = format!("{}.md", title);
        let path = notes_dir.join(filename);
        if path.exists() {
            return Ok(path);
        }
        return Err(anyhow!("Note '{}' does not exist", title));
    }
    
    // Use fuzzy resolution for simple names
    resolve_note_path(title)
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
