use crate::config::get_config;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub mod markdown;
/// Notes storage (creates and opens individual markdown files under notes/)
pub mod notes;

/// Get the base content directory path
/// Get the base content directory path, using the global cached configuration
pub fn get_content_dir() -> Result<PathBuf> {
    // First check the config (cached)
    let config = get_config();

    // If content_dir is specified in config, use that (supports absolute, relative, or '~' paths)
    if let Some(dir) = config.paths.content_dir.clone() {
        let dir_str = dir.to_string_lossy();
        // Only expand leading '~' to home directory; otherwise use as given
        let expanded: PathBuf = if dir_str.starts_with("~") {
            // Tilde expansion
            if let Some(home) = dirs::home_dir() {
                // Remove '~' and any leading separator, then join to home
                let without_tilde = dir_str
                    .trim_start_matches('~')
                    .trim_start_matches(std::path::MAIN_SEPARATOR);
                home.join(without_tilde)
            } else {
                // Fallback to literal path
                PathBuf::from(&*dir_str)
            }
        } else {
            // Use the path as-is (absolute or relative)
            dir
        };
        if !expanded.exists() {
            fs::create_dir_all(&expanded).with_context(|| {
                format!("Failed to create content directory: {}", expanded.display())
            })?;
        }
        return Ok(expanded);
    }

    // Default to content/ in current directory
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;

    let content_dir = current_dir.join("content");
    if !content_dir.exists() {
        fs::create_dir_all(&content_dir).context("Failed to create content directory")?;
    }

    Ok(content_dir)
}

/// Get the lists directory path
pub fn get_lists_dir() -> Result<PathBuf> {
    let lists_dir = get_content_dir()?.join("lists");
    if !lists_dir.exists() {
        fs::create_dir_all(&lists_dir).context("Failed to create lists directory")?;
    }

    Ok(lists_dir)
}

/// Get the notes directory path
pub fn get_notes_dir() -> Result<PathBuf> {
    let notes_dir = get_content_dir()?.join("notes");
    if !notes_dir.exists() {
        fs::create_dir_all(&notes_dir).context("Failed to create notes directory")?;
    }

    Ok(notes_dir)
}



/// Recursively list all files in a directory tree with a specific extension
pub fn list_files_recursive(dir: &Path, extension: &str) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    list_files_recursive_impl(dir, extension, &mut files)?;
    Ok(files)
}

fn list_files_recursive_impl(dir: &Path, extension: &str, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory: {}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() && path.extension().map_or(false, |ext| ext == extension) {
            files.push(path);
        } else if path.is_dir() {
            // Skip hidden directories (starting with .)
            if let Some(dir_name) = path.file_name() {
                if !dir_name.to_string_lossy().starts_with('.') {
                    list_files_recursive_impl(&path, extension, files)?;
                }
            }
        }
    }

    Ok(())
}

/// Represents a file with its filename and relative path from the base directory
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,      // filename without extension (e.g., "pharmacy")
    pub relative_path: String,  // relative path from base dir (e.g., "groceries/pharmacy")
    pub full_path: PathBuf,     // full filesystem path
}

/// List all available lists with directory structure support
pub fn list_lists() -> Result<Vec<String>> {
    let lists_dir = get_lists_dir()?;
    let files = list_files_recursive(&lists_dir, "md")?;

    let lists = files
        .iter()
        .filter_map(|path| {
            // Get relative path from lists directory
            if let Ok(relative) = path.strip_prefix(&lists_dir) {
                // Remove .md extension and convert to string
                let path_without_ext = relative.with_extension("");
                return Some(path_without_ext.to_string_lossy().to_string());
            }
            None
        })
        .collect();

    Ok(lists)
}

/// List all available lists with full file information
pub fn list_lists_with_info() -> Result<Vec<FileEntry>> {
    let lists_dir = get_lists_dir()?;
    let files = list_files_recursive(&lists_dir, "md")?;

    let lists = files
        .iter()
        .filter_map(|path| {
            // Get relative path from lists directory
            if let Ok(relative) = path.strip_prefix(&lists_dir) {
                // Get filename without extension
                let name = relative.file_stem()?.to_string_lossy().to_string();
                // Get relative path without extension
                let relative_path = relative.with_extension("").to_string_lossy().to_string();
                
                return Some(FileEntry {
                    name,
                    relative_path,
                    full_path: path.clone(),
                });
            }
            None
        })
        .collect();

    Ok(lists)
}

/// List all available notes with directory structure support
pub fn list_notes() -> Result<Vec<String>> {
    let notes_dir = get_notes_dir()?;
    let files = list_files_recursive(&notes_dir, "md")?;

    let notes = files
        .iter()
        .filter_map(|path| {
            // Get relative path from notes directory
            if let Ok(relative) = path.strip_prefix(&notes_dir) {
                // Remove .md extension and convert to string
                let path_without_ext = relative.with_extension("");
                return Some(path_without_ext.to_string_lossy().to_string());
            }
            None
        })
        .collect();

    Ok(notes)
}

/// List all available notes with full file information
pub fn list_notes_with_info() -> Result<Vec<FileEntry>> {
    let notes_dir = get_notes_dir()?;
    let files = list_files_recursive(&notes_dir, "md")?;

    let notes = files
        .iter()
        .filter_map(|path| {
            // Get relative path from notes directory
            if let Ok(relative) = path.strip_prefix(&notes_dir) {
                // Get filename without extension
                let name = relative.file_stem()?.to_string_lossy().to_string();
                // Get relative path without extension
                let relative_path = relative.with_extension("").to_string_lossy().to_string();
                
                return Some(FileEntry {
                    name,
                    relative_path,
                    full_path: path.clone(),
                });
            }
            None
        })
        .collect();

    Ok(notes)
}
