use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub mod markdown;

/// Get the base content directory path
pub fn get_content_dir() -> Result<PathBuf> {
    // First check the config
    let config = crate::config::Config::load()?;
    
    // If content_dir is specified in config, use that
    if let Some(dir) = config.paths.content_dir {
        // Expand '~' to the user's home directory
        let dir_str = dir.to_string_lossy();
        let expanded = if let Some(home) = dirs::home_dir() {
            // Strip leading '~/' or '~'
            let without_tilde = if let Some(s) = dir_str.strip_prefix("~/") {
                s
            } else if let Some(s) = dir_str.strip_prefix('~') {
                s
            } else {
                &dir_str
            };
            // Remove any leading path separator
            let without_sep = without_tilde.trim_start_matches(std::path::MAIN_SEPARATOR);
            home.join(without_sep)
        } else {
            // Fallback: use the path as-is
            PathBuf::from(&*dir_str)
        };
        if !expanded.exists() {
            fs::create_dir_all(&expanded)
                .with_context(|| format!("Failed to create content directory: {}", expanded.display()))?;
        }
        return Ok(expanded);
    }
    
    // Default to content/ in current directory
    let current_dir = std::env::current_dir()
        .context("Failed to get current directory")?;
    
    let content_dir = current_dir.join("content");
    if !content_dir.exists() {
        fs::create_dir_all(&content_dir)
            .context("Failed to create content directory")?;
    }
    
    Ok(content_dir)
}

/// Get the lists directory path
pub fn get_lists_dir() -> Result<PathBuf> {
    let lists_dir = get_content_dir()?.join("lists");
    if !lists_dir.exists() {
        fs::create_dir_all(&lists_dir)
            .context("Failed to create lists directory")?;
    }
    
    Ok(lists_dir)
}

/// Get the notes directory path
pub fn get_notes_dir() -> Result<PathBuf> {
    let notes_dir = get_content_dir()?.join("notes");
    if !notes_dir.exists() {
        fs::create_dir_all(&notes_dir)
            .context("Failed to create notes directory")?;
    }
    
    Ok(notes_dir)
}

/// Get the posts directory path
pub fn get_posts_dir() -> Result<PathBuf> {
    let posts_dir = get_content_dir()?.join("posts");
    if !posts_dir.exists() {
        fs::create_dir_all(&posts_dir)
            .context("Failed to create posts directory")?;
    }
    
    Ok(posts_dir)
}

/// Get the media directory path
pub fn get_media_dir() -> Result<PathBuf> {
    // First check the config
    let config = crate::config::Config::load()?;
    
    // If media_dir is specified in config, use that
    if let Some(dir) = config.paths.media_dir {
        // Expand '~' to the user's home directory
        let dir_str = dir.to_string_lossy();
        let expanded = if let Some(home) = dirs::home_dir() {
            // Strip leading '~/' or '~'
            let without_tilde = if let Some(s) = dir_str.strip_prefix("~/") {
                s
            } else if let Some(s) = dir_str.strip_prefix('~') {
                s
            } else {
                &dir_str
            };
            // Remove any leading path separator
            let without_sep = without_tilde.trim_start_matches(std::path::MAIN_SEPARATOR);
            home.join(without_sep)
        } else {
            // Fallback: use the path as-is
            PathBuf::from(&*dir_str)
        };
        if !expanded.exists() {
            fs::create_dir_all(&expanded)
                .with_context(|| format!("Failed to create media directory: {}", expanded.display()))?;
        }
        return Ok(expanded);
    }
    
    // Default to media/ in content directory
    let media_dir = get_content_dir()?.join("media");
    if !media_dir.exists() {
        fs::create_dir_all(&media_dir)
            .context("Failed to create media directory")?;
    }
    
    Ok(media_dir)
}

/// List all files in a directory with a specific extension
pub fn list_files(dir: &Path, extension: &str) -> Result<Vec<PathBuf>> {
    let entries = fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory: {}", dir.display()))?;
        
    let files = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == extension) {
                Some(path)
            } else {
                None
            }
        })
        .collect();
        
    Ok(files)
}

/// List all available lists
pub fn list_lists() -> Result<Vec<String>> {
    let lists_dir = get_lists_dir()?;
    let files = list_files(&lists_dir, "md")?;
    
    let lists = files
        .iter()
        .filter_map(|path| {
            path.file_stem().map(|stem| stem.to_string_lossy().to_string())
        })
        .collect();
        
    Ok(lists)
}