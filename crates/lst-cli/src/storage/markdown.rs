use crate::models::{generate_anchor, is_valid_anchor, ItemStatus, List, ListItem};
use anyhow::{Context, Result};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

/// Load a list from a markdown file (supports directory paths)
pub fn load_list(list_name: &str) -> Result<List> {
    let lists_dir = super::get_lists_dir()?;

    // Try exact path first (supports both simple names and directory paths)
    let filename = format!("{}.md", list_name);
    let path = lists_dir.join(filename);

    if path.exists() {
        return parse_list_from_file(&path);
    }

    // If exact path doesn't exist and input looks like a simple filename, try fuzzy search
    if !list_name.contains('/') && !list_name.contains('\\') {
        let entries = super::list_lists_with_info()?;

        // First try exact filename match
        for entry in &entries {
            if entry.name == list_name {
                return parse_list_from_file(&entry.full_path);
            }
        }

        // Then try fuzzy match by filename
        let matches: Vec<&super::FileEntry> = entries
            .iter()
            .filter(|entry| entry.name.contains(list_name))
            .collect();

        match matches.len() {
            0 => anyhow::bail!("List '{}' does not exist", list_name),
            1 => parse_list_from_file(&matches[0].full_path),
            _ => {
                let match_names: Vec<String> =
                    matches.iter().map(|e| e.relative_path.clone()).collect();
                anyhow::bail!("Multiple lists match '{}': {:?}", list_name, match_names);
            }
        }
    } else {
        anyhow::bail!("List '{}' does not exist", list_name);
    }
}

/// Save a list to a markdown file using the original list name path
pub fn save_list_with_path(list: &List, list_name: &str) -> Result<()> {
    let lists_dir = super::get_lists_dir()?;
    let filename = format!("{}.md", list_name);
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
        let list_name = path
            .file_stem()
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
            let anchor = captures
                .get(3)
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
    let frontmatter =
        serde_yaml::to_string(list).unwrap_or_else(|_| "title: Untitled List\n".to_string());

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

/// Create a new list (supports directory paths)
pub fn create_list(name: &str) -> Result<PathBuf> {
    let lists_dir = super::get_lists_dir()?;
    let filename = format!("{}.md", name);
    let path = lists_dir.join(&filename);

    if path.exists() {
        anyhow::bail!("List '{}' already exists", name);
    }

    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }
    }

    // Extract just the filename for the list title (not the full path)
    let list_title = if name.contains('/') || name.contains('\\') {
        Path::new(name)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(name)
            .to_string()
    } else {
        name.to_string()
    };

    let list = List::new(list_title);
    write_list_to_file(&list, &path)?;

    Ok(path)
}

/// Add an item to a list
pub fn add_item(list_name: &str, text: &str) -> Result<ListItem> {
    let mut list = load_list(list_name)?;
    let item = list.add_item(text.to_string());
    let item_clone = item.clone();

    // Save to the correct path by resolving the list name to its actual path
    save_list_with_path(&list, list_name)?;

    Ok(item_clone)
}

/// Mark an item as done
pub fn mark_done(list_name: &str, target: &str) -> Result<Vec<ListItem>> {
    let mut list = load_list(list_name)?;

    // If there are multiple comma-separated targets, handle each one
    if target.contains(',') {
        let targets: Vec<&str> = target.split(',').map(|s| s.trim()).collect();
        let mut marked_items = Vec::new();

        for target in targets {
            if let Ok(item) = mark_item_done(&mut list, target) {
                marked_items.push(item);
            }
        }

        if marked_items.is_empty() {
            anyhow::bail!("No matching items found in list '{}'", list_name);
        }

        save_list_with_path(&list, list_name)?;
        return Ok(marked_items);
    }

    // Handle single target
    if let Ok(item) = mark_item_done(&mut list, target) {
        save_list_with_path(&list, list_name)?;
        return Ok(vec![item]);
    }

    anyhow::bail!(
        "No item matching '{}' found in list '{}'",
        target,
        list_name
    )
}

/// Mark an item as undone (not completed)
pub fn mark_undone(list_name: &str, target: &str) -> Result<Vec<ListItem>> {
    let mut list = load_list(list_name)?;

    // If there are multiple comma-separated targets, handle each one
    if target.contains(',') {
        let targets: Vec<&str> = target.split(',').map(|s| s.trim()).collect();
        let mut marked_items = Vec::new();

        for target in targets {
            if let Ok(item) = mark_item_undone(&mut list, target) {
                marked_items.push(item);
            }
        }

        if marked_items.is_empty() {
            anyhow::bail!("No matching items found in list '{}'", list_name);
        }

        save_list_with_path(&list, list_name)?;
        return Ok(marked_items);
    }

    // Handle single target
    if let Ok(item) = mark_item_undone(&mut list, target) {
        save_list_with_path(&list, list_name)?;
        return Ok(vec![item]);
    }

    anyhow::bail!(
        "No item matching '{}' found in list '{}'",
        target,
        list_name
    )
}

/// Helper function to mark a single item as done
fn mark_item_done(list: &mut List, target: &str) -> Result<ListItem> {
    // Find item and set status
    find_and_set_item_status(list, target, ItemStatus::Done)
}

/// Helper function to mark a single item as undone
fn mark_item_undone(list: &mut List, target: &str) -> Result<ListItem> {
    // Find item and set status
    find_and_set_item_status(list, target, ItemStatus::Todo)
}

/// Helper function to find an item and set its status
fn find_and_set_item_status(list: &mut List, target: &str, status: ItemStatus) -> Result<ListItem> {
    // Try to find the item by anchor first
    if is_valid_anchor(target) {
        if let Some(idx) = list.find_by_anchor(target) {
            list.items[idx].status = status;
            return Ok(list.items[idx].clone());
        }
    }

    // Try to find by exact text match
    if let Some(idx) = list.find_by_text(target) {
        list.items[idx].status = status;
        return Ok(list.items[idx].clone());
    }

    // Check if it's an index reference (#N)
    if let Some(number_str) = target.strip_prefix('#') {
        if let Ok(idx) = number_str.parse::<usize>() {
            if let Some(item) = list.get_by_index(idx - 1) {
                // Convert to 0-based
                let item = item.clone();
                let idx = list
                    .find_by_anchor(&item.anchor)
                    .context("Internal error: anchor not found")?;
                list.items[idx].status = status;
                return Ok(item);
            }
        }
    }

    // Fallback to fuzzy matching (simple contains for now)
    let matches = crate::models::fuzzy_find(&list.items, target, 0.75);
    match matches.len() {
        0 => anyhow::bail!("No item matching '{}' found", target),
        1 => {
            let idx = matches[0];
            list.items[idx].status = status;
            Ok(list.items[idx].clone())
        }
        _ => anyhow::bail!(
            "Multiple items match '{}', please use a more specific query",
            target
        ),
    }
}

/// Delete an item from a list
pub fn delete_item(list_name: &str, target: &str) -> Result<Vec<ListItem>> {
    let mut list = load_list(list_name)?;

    // If there are multiple comma-separated targets, handle each one
    if target.contains(',') {
        let targets: Vec<&str> = target.split(',').map(|s| s.trim()).collect();
        let mut removed_items = Vec::new();

        // Handle each target - we need to process them from highest index to lowest
        // to avoid changing indices during removal
        let mut to_remove = Vec::new();

        for target in targets {
            if let Ok((idx, item)) = find_item_for_removal(&list, target) {
                to_remove.push((idx, item.clone()));
            }
        }

        // Sort in reverse order by index
        to_remove.sort_by(|a, b| b.0.cmp(&a.0));

        // Remove items in reverse index order
        for (idx, _) in &to_remove {
            let removed = list.items.remove(*idx);
            removed_items.push(removed);
        }

        if removed_items.is_empty() {
            anyhow::bail!("No matching items found in list '{}'", list_name);
        }

        // Reverse back to original order for consistent output
        removed_items.reverse();
        list.metadata.updated = chrono::Utc::now();
        save_list_with_path(&list, list_name)?;
        return Ok(removed_items);
    }

    // Handle single target
    if let Ok((idx, _)) = find_item_for_removal(&list, target) {
        let removed = list.items.remove(idx);
        list.metadata.updated = chrono::Utc::now();
        save_list_with_path(&list, list_name)?;
        return Ok(vec![removed]);
    }

    anyhow::bail!(
        "No item matching '{}' found in list '{}'",
        target,
        list_name
    )
}

/// Edit the text of an item in a list
pub fn edit_item_text(list_name: &str, target: &str, new_text: &str) -> Result<()> {
    if new_text.trim().is_empty() {
        anyhow::bail!("New text cannot be empty");
    }

    let mut list = load_list(list_name)?;

    if let Ok((idx, _)) = find_item_for_removal(&list, target) {
        list.items[idx].text = new_text.to_string();
        list.metadata.updated = chrono::Utc::now();
        save_list_with_path(&list, list_name)?;
        Ok(())
    } else {
        anyhow::bail!(
            "No item matching '{}' found in list '{}'",
            target,
            list_name
        )
    }
}

/// Move an item to a new position within a list
pub fn reorder_item(list_name: &str, target: &str, new_index: usize) -> Result<()> {
    let mut list = load_list(list_name)?;

    if let Ok((idx, _)) = find_item_for_removal(&list, target) {
        let item = list.items.remove(idx);
        let clamped = new_index.min(list.items.len());
        list.items.insert(clamped, item);
        list.metadata.updated = chrono::Utc::now();
        save_list_with_path(&list, list_name)?;
        Ok(())
    } else {
        anyhow::bail!(
            "No item matching '{}' found in list '{}'",
            target,
            list_name
        )
    }
}

/// Save a list to a markdown file
pub fn save_list(list: &List) -> Result<()> {
    let lists_dir = super::get_lists_dir()?;
    let filename = list.file_name();
    let path = lists_dir.join(filename);

    write_list_to_file(list, &path)
}

/// Helper function to find an item for removal, returning (index, item)
pub fn find_item_for_removal<'a>(list: &'a List, target: &str) -> Result<(usize, &'a ListItem)> {
    // Try to find the item by anchor first
    if is_valid_anchor(target) {
        if let Some(idx) = list.find_by_anchor(target) {
            return Ok((idx, &list.items[idx]));
        }
    }

    // Try to find by exact text match
    if let Some(idx) = list.find_by_text(target) {
        return Ok((idx, &list.items[idx]));
    }

    // Check if it's an index reference (#N)
    if let Some(number_str) = target.strip_prefix('#') {
        if let Ok(idx) = number_str.parse::<usize>() {
            if let Some(item) = list.get_by_index(idx - 1) {
                // Convert to 0-based
                let idx = list
                    .find_by_anchor(&item.anchor)
                    .context("Internal error: anchor not found")?;
                return Ok((idx, &list.items[idx]));
            }
        }
    }

    // Fallback to fuzzy matching (simple contains for now)
    let matches = crate::models::fuzzy_find(&list.items, target, 0.75);
    match matches.len() {
        0 => anyhow::bail!("No item matching '{}' found", target),
        1 => {
            let idx = matches[0];
            Ok((idx, &list.items[idx]))
        }
        _ => anyhow::bail!(
            "Multiple items match '{}', please use a more specific query",
            target
        ),
    }
}

/// Remove all items from a list, returning the number of removed entries
pub fn wipe_list(list_name: &str) -> Result<usize> {
    let mut list = load_list(list_name)?;
    let removed = list.items.len();
    if removed == 0 {
        return Ok(0);
    }
    list.items.clear();
    list.metadata.updated = chrono::Utc::now();
    save_list_with_path(&list, list_name)?;
    Ok(removed)
}
