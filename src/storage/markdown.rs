use anyhow::{Context, Result};
use regex::Regex;
use std::fs;
use std::path::Path;
use crate::models::{List, ListItem, ItemStatus, is_valid_anchor, generate_anchor};

/// Load a list from a markdown file
pub fn load_list(list_name: &str) -> Result<List> {
    let lists_dir = super::get_lists_dir()?;
    let filename = format!("{}.md", list_name);
    let path = lists_dir.join(filename);
    
    if !path.exists() {
        anyhow::bail!("List '{}' does not exist", list_name);
    }
    
    parse_list_from_file(&path)
}

/// Save a list to a markdown file
pub fn save_list(list: &List) -> Result<()> {
    let lists_dir = super::get_lists_dir()?;
    let filename = list.file_name();
    let path = lists_dir.join(filename);
    
    write_list_to_file(list, &path)
}

/// Parse a list from a markdown file
fn parse_list_from_file(path: &Path) -> Result<List> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read list file: {}", path.display()))?;
    
    parse_list_from_string(&content, path)
}

/// Write a list to a markdown file
fn write_list_to_file(list: &List, path: &Path) -> Result<()> {
    let content = format_list_as_markdown(list);
    
    fs::write(path, content)
        .with_context(|| format!("Failed to write list file: {}", path.display()))?;
    
    Ok(())
}

/// Parse a list from a markdown string
fn parse_list_from_string(content: &str, path: &Path) -> Result<List> {
    // Split content into frontmatter and body
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    
    if parts.len() < 3 {
        // No frontmatter, create a new list with just the content
        let list_name = path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled List".to_string());
        
        let mut list = List::new(list_name);
        parse_items(&mut list, content);
        return Ok(list);
    }
    
    // Parse frontmatter
    let frontmatter = parts[1].trim();
    let list: List = serde_yaml::from_str(frontmatter)
        .with_context(|| format!("Failed to parse list frontmatter in {}", path.display()))?;
    
    // Parse items from the body
    let mut list = list;
    parse_items(&mut list, parts[2]);
    
    Ok(list)
}

/// Parse list items from markdown content
fn parse_items(list: &mut List, content: &str) {
    // Clear existing items
    list.items.clear();
    
    lazy_static::lazy_static! {
        // Match markdown todo items with optional anchors
        static ref ITEM_RE: Regex = Regex::new(
            r"^- \[([ xX])\] (.*?)(?:  \^([A-Za-z0-9-]{4,}))?$"
        ).unwrap();
    }
    
    for line in content.lines() {
        if let Some(captures) = ITEM_RE.captures(line) {
            let status = if captures[1].trim().is_empty() {
                ItemStatus::Todo
            } else {
                ItemStatus::Done
            };
            
            let text = captures[2].to_string();
            let anchor = captures.get(3)
                .map(|m| format!("^{}", m.as_str()))
                .unwrap_or_else(generate_anchor);
            
            list.items.push(ListItem {
                text,
                status,
                anchor,
            });
        }
    }
}

/// Format a list as markdown
fn format_list_as_markdown(list: &List) -> String {
    // Format frontmatter
    let frontmatter = serde_yaml::to_string(list)
        .unwrap_or_else(|_| "title: Untitled List\n".to_string());
    
    let mut content = format!("---\n{}---\n\n", frontmatter);
    
    // Format items
    for item in &list.items {
        let status = match item.status {
            ItemStatus::Todo => " ",
            ItemStatus::Done => "x",
        };
        
        content.push_str(&format!("- [{}] {}  {}\n", status, item.text, item.anchor));
    }
    
    content
}

/// Create a new list
pub fn create_list(name: &str) -> Result<List> {
    let lists_dir = super::get_lists_dir()?;
    let filename = format!("{}.md", name);
    let path = lists_dir.join(filename);
    
    if path.exists() {
        anyhow::bail!("List '{}' already exists", name);
    }
    
    let list = List::new(name.to_string());
    write_list_to_file(&list, &path)?;
    
    Ok(list)
}

/// Add an item to a list
pub fn add_item(list_name: &str, text: &str) -> Result<ListItem> {
    let mut list = load_list(list_name)?;
    let item = list.add_item(text.to_string());
    let item_clone = item.clone();
    save_list(&list)?;
    
    Ok(item_clone)
}

/// Mark an item as done
pub fn mark_done(list_name: &str, target: &str) -> Result<ListItem> {
    let mut list = load_list(list_name)?;
    
    // Try to find the item by anchor first
    if is_valid_anchor(target) {
        if let Some(idx) = list.find_by_anchor(target) {
            list.items[idx].status = ItemStatus::Done;
            let item = list.items[idx].clone();
            save_list(&list)?;
            return Ok(item);
        }
    }
    
    // Try to find by exact text match
    if let Some(idx) = list.find_by_text(target) {
        list.items[idx].status = ItemStatus::Done;
        let item = list.items[idx].clone();
        save_list(&list)?;
        return Ok(item);
    }
    
    // Check if it's an index reference (#N)
    if let Some(number_str) = target.strip_prefix('#') {
        if let Ok(idx) = number_str.parse::<usize>() {
            if let Some(item) = list.get_by_index(idx - 1) { // Convert to 0-based
                let item = item.clone();
                let idx = list.find_by_anchor(&item.anchor)
                    .context("Internal error: anchor not found")?;
                list.items[idx].status = ItemStatus::Done;
                save_list(&list)?;
                return Ok(item);
            }
        }
    }
    
    // Fallback to fuzzy matching (simple contains for now)
    let matches = crate::models::fuzzy_find(&list.items, target, 0.75);
    match matches.len() {
        0 => anyhow::bail!("No item matching '{}' found in list '{}'", target, list_name),
        1 => {
            let idx = matches[0];
            list.items[idx].status = ItemStatus::Done;
            let item = list.items[idx].clone();
            save_list(&list)?;
            Ok(item)
        },
        _ => anyhow::bail!("Multiple items match '{}', please use a more specific query", target),
    }
}

/// Delete an item from a list
pub fn delete_item(list_name: &str, target: &str) -> Result<ListItem> {
    let mut list = load_list(list_name)?;
    
    // Try to find the item by anchor first
    if is_valid_anchor(target) {
        if let Some(idx) = list.find_by_anchor(target) {
            let item = list.items.remove(idx);
            list.metadata.updated = chrono::Utc::now();
            save_list(&list)?;
            return Ok(item);
        }
    }
    
    // Try to find by exact text match
    if let Some(idx) = list.find_by_text(target) {
        let item = list.items.remove(idx);
        list.metadata.updated = chrono::Utc::now();
        save_list(&list)?;
        return Ok(item);
    }
    
    // Check if it's an index reference (#N)
    if let Some(number_str) = target.strip_prefix('#') {
        if let Ok(idx) = number_str.parse::<usize>() {
            if let Some(item) = list.get_by_index(idx - 1) { // Convert to 0-based
                let item = item.clone();
                let idx = list.find_by_anchor(&item.anchor)
                    .context("Internal error: anchor not found")?;
                let removed = list.items.remove(idx);
                list.metadata.updated = chrono::Utc::now();
                save_list(&list)?;
                return Ok(removed);
            }
        }
    }
    
    // Fallback to fuzzy matching (simple contains for now)
    let matches = crate::models::fuzzy_find(&list.items, target, 0.75);
    match matches.len() {
        0 => anyhow::bail!("No item matching '{}' found in list '{}'", target, list_name),
        1 => {
            let idx = matches[0];
            let item = list.items.remove(idx);
            list.metadata.updated = chrono::Utc::now();
            save_list(&list)?;
            Ok(item)
        },
        _ => anyhow::bail!("Multiple items match '{}', please use a more specific query", target),
    }
}