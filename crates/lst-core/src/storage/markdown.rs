use crate::models::{generate_anchor, is_valid_anchor, ItemStatus, List, ListItem, Category};
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
    let mut list: List = serde_yaml::from_str(frontmatter)
        .with_context(|| format!("Failed to parse list frontmatter in {}", path.display()))?;

    // Handle backward compatibility: migrate old 'items' field to 'uncategorized_items'
    if !list.items.is_empty() {
        list.uncategorized_items = list.items.clone();
        list.items.clear();
    }

    // Parse items from the body
    parse_items(&mut list, parts[2]);

    Ok(list)
}

/// Parse list items from markdown content
fn parse_items(list: &mut List, content: &str) {
    // Clear existing items and categories
    list.uncategorized_items.clear();
    list.categories.clear();

    lazy_static::lazy_static! {
        // Match markdown todo items with optional anchors
        static ref ITEM_RE: Regex = Regex::new(
            r"^- \[([ xX])\] (.*?)(?:  \^([A-Za-z0-9-]{4,}))?$"
        ).unwrap();
        // Match category headlines
        static ref HEADLINE_RE: Regex = Regex::new(r"^## (.+)$").unwrap();
    }

    let mut current_category: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();
        
        // Check for category headline
        if let Some(captures) = HEADLINE_RE.captures(line) {
            let category_name = captures[1].trim().to_string();
            current_category = Some(category_name.clone());
            
            // Create category if it doesn't exist
            if !list.categories.iter().any(|c| c.name == category_name) {
                list.categories.push(Category {
                    name: category_name,
                    items: Vec::new(),
                });
            }
            continue;
        }

        // Check for list item
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

            let item = ListItem {
                text,
                status,
                anchor,
            };

            // Add to current category or uncategorized
            match &current_category {
                Some(cat_name) => {
                    if let Some(category) = list.categories.iter_mut().find(|c| c.name == *cat_name) {
                        category.items.push(item);
                    }
                }
                None => {
                    list.uncategorized_items.push(item);
                }
            }
        }
    }
}

/// Format a list as markdown
fn format_list_as_markdown(list: &List) -> String {
    // Format frontmatter - only serialize metadata, not items
    let frontmatter =
        serde_yaml::to_string(&list.metadata).unwrap_or_else(|_| "title: Untitled List\n".to_string());

    let mut content = format!("---\n{}---\n\n", frontmatter);

    // Format uncategorized items first (no headline)
    for item in &list.uncategorized_items {
        let status = match item.status {
            ItemStatus::Todo => " ",
            ItemStatus::Done => "x",
        };
        content.push_str(&format!("- [{}] {}  {}\n", status, item.text, item.anchor));
    }

    // Add blank line between uncategorized and categorized if both exist
    if !list.uncategorized_items.is_empty() && !list.categories.is_empty() {
        content.push('\n');
    }

    // Format categorized items with headlines
    for category in &list.categories {
        content.push_str(&format!("## {}\n", category.name));
        for item in &category.items {
            let status = match item.status {
                ItemStatus::Todo => " ",
                ItemStatus::Done => "x",
            };
            content.push_str(&format!("- [{}] {}  {}\n", status, item.text, item.anchor));
        }
        content.push('\n');
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

/// Add an item to a specific category in a list
pub fn add_item_to_category(list_name: &str, text: &str, category: Option<&str>) -> Result<ListItem> {
    let mut list = load_list(list_name)?;
    let item = list.add_item_to_category(text.to_string(), category);

    save_list_with_path(&list, list_name)?;

    Ok(item)
}

/// Mark an item as done
pub fn mark_done(list_name: &str, target: &str, threshold: i64) -> Result<Vec<ListItem>> {
    let mut list = load_list(list_name)?;

    // If there are multiple comma-separated targets, handle each one
    if target.contains(',') {
        let targets: Vec<&str> = target.split(',').map(|s| s.trim()).collect();
        let mut marked_items = Vec::new();

        for target in targets {
            if let Ok(item) = mark_item_done(&mut list, target, threshold) {
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
    if let Ok(item) = mark_item_done(&mut list, target, threshold) {
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
pub fn mark_undone(list_name: &str, target: &str, threshold: i64) -> Result<Vec<ListItem>> {
    let mut list = load_list(list_name)?;

    // If there are multiple comma-separated targets, handle each one
    if target.contains(',') {
        let targets: Vec<&str> = target.split(',').map(|s| s.trim()).collect();
        let mut marked_items = Vec::new();

        for target in targets {
            if let Ok(item) = mark_item_undone(&mut list, target, threshold) {
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
    if let Ok(item) = mark_item_undone(&mut list, target, threshold) {
        save_list_with_path(&list, list_name)?;
        return Ok(vec![item]);
    }

    anyhow::bail!(
        "No item matching '{}' found in list '{}'",
        target,
        list_name
    )
}

/// Reset all items in a list to undone status
pub fn reset_list(list_name: &str) -> Result<Vec<ListItem>> {
    let mut list = load_list(list_name)?;
    let mut reset_items = Vec::new();

    // Mark all items as undone
    for item in list.all_items_mut() {
        if item.status == ItemStatus::Done {
            item.status = ItemStatus::Todo;
            reset_items.push(item.clone());
        }
    }

    if reset_items.is_empty() {
        anyhow::bail!("No completed items found in list '{}'", list_name);
    }

    list.metadata.updated = chrono::Utc::now();
    save_list_with_path(&list, list_name)?;
    Ok(reset_items)
}

/// Helper function to mark a single item as done
fn mark_item_done(list: &mut List, target: &str, threshold: i64) -> Result<ListItem> {
    // Find item and set status
    find_and_set_item_status(list, target, ItemStatus::Done, threshold)
}

/// Helper function to mark a single item as undone
fn mark_item_undone(list: &mut List, target: &str, threshold: i64) -> Result<ListItem> {
    // Find item and set status
    find_and_set_item_status(list, target, ItemStatus::Todo, threshold)
}

/// Helper function to find an item and set its status
fn find_and_set_item_status(list: &mut List, target: &str, status: ItemStatus, threshold: i64) -> Result<ListItem> {
    // Try to find the item by anchor first
    if is_valid_anchor(target) {
        if let Some(item) = list.find_item_mut_by_anchor(target) {
            item.status = status;
            return Ok(item.clone());
        }
    }

    // Try to find by exact text match
    if let Some(item) = list.all_items_mut().find(|item| item.text.to_lowercase() == target.to_lowercase()) {
        item.status = status;
        return Ok(item.clone());
    }

    // Check if it's an index reference (#N)
    if let Some(number_str) = target.strip_prefix('#') {
        if let Ok(idx) = number_str.parse::<usize>() {
            if let Some(item) = list.all_items_mut().nth(idx - 1) {
                item.status = status;
                return Ok(item.clone());
            }
        }
    }

    // Fallback to fuzzy matching (simple contains for now)
    let all_items: Vec<ListItem> = list.all_items().cloned().collect();
    let matches = crate::models::fuzzy_find(&all_items, target, threshold);
    match matches.len() {
        0 => anyhow::bail!("No item matching '{}' found", target),
        1 => {
            let target_anchor = &all_items[matches[0]].anchor;
            if let Some(item) = list.find_item_mut_by_anchor(target_anchor) {
                item.status = status;
                Ok(item.clone())
            } else {
                anyhow::bail!("Internal error: anchor not found")
            }
        }
        _ => anyhow::bail!(
            "Multiple items match '{}', please use a more specific query",
            target
        ),
    }
}

/// Delete an item from a list
pub fn delete_item(list_name: &str, target: &str, threshold: i64) -> Result<Vec<ListItem>> {
    let mut list = load_list(list_name)?;

    // If there are multiple comma-separated targets, handle each one
    if target.contains(',') {
        let targets: Vec<&str> = target.split(',').map(|s| s.trim()).collect();
        let mut removed_items = Vec::new();

        // Handle each target - we need to process them carefully to avoid index issues
        for target in targets {
            if let Ok(location) = find_item_for_removal(&list, target, threshold) {
                let removed = remove_item_at_location(&mut list, location);
                removed_items.push(removed);
            }
        }

        if removed_items.is_empty() {
            anyhow::bail!("No matching items found in list '{}'", list_name);
        }

        list.metadata.updated = chrono::Utc::now();
        save_list_with_path(&list, list_name)?;
        return Ok(removed_items);
    }

    // Handle single target
    if let Ok(location) = find_item_for_removal(&list, target, threshold) {
        let removed = remove_item_at_location(&mut list, location);
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

/// Remove an item at the specified location
pub fn remove_item_at_location(list: &mut List, location: ItemLocation) -> ListItem {
    match location {
        ItemLocation::Uncategorized(idx) => list.uncategorized_items.remove(idx),
        ItemLocation::Categorized { category_index, item_index } => {
            list.categories[category_index].items.remove(item_index)
        }
    }
}

/// Edit the text of an item in a list
pub fn edit_item_text(list_name: &str, target: &str, new_text: &str) -> Result<()> {
    if new_text.trim().is_empty() {
        anyhow::bail!("New text cannot be empty");
    }

    let mut list = load_list(list_name)?;

    // Find the item by anchor (most reliable method)
    if is_valid_anchor(target) {
        if let Some(item) = list.find_item_mut_by_anchor(target) {
            item.text = new_text.to_string();
            list.metadata.updated = chrono::Utc::now();
            save_list_with_path(&list, list_name)?;
            return Ok(());
        }
    }

    // Try other methods - need to find first, then modify
    let target_lower = target.to_lowercase();
    let found_anchor = list.all_items()
        .find(|item| item.text.to_lowercase() == target_lower)
        .map(|item| item.anchor.clone());
    
    if let Some(anchor) = found_anchor {
        if let Some(item) = list.find_item_mut_by_anchor(&anchor) {
            item.text = new_text.to_string();
            list.metadata.updated = chrono::Utc::now();
            save_list_with_path(&list, list_name)?;
            Ok(())
        } else {
            anyhow::bail!("Internal error: anchor not found")
        }
    } else {
        anyhow::bail!(
            "No item matching '{}' found in list '{}'",
            target,
            list_name
        )
    }
}

/// Move an item to a new position within a list
pub fn reorder_item(list_name: &str, target: &str, new_index: usize, threshold: i64) -> Result<()> {
    let mut list = load_list(list_name)?;

    if let Ok(location) = find_item_for_removal(&list, target, threshold) {
        let item = remove_item_at_location(&mut list, location);
        
        // For now, reordering puts items in uncategorized section
        // TODO: Could be enhanced to support reordering within categories
        let clamped = new_index.min(list.uncategorized_items.len());
        list.uncategorized_items.insert(clamped, item);
        
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

/// Helper function to find an item for removal, returning location info
pub fn find_item_for_removal(list: &List, target: &str, threshold: i64) -> Result<ItemLocation> {
    // Try to find the item by anchor first
    if is_valid_anchor(target) {
        if let Some(location) = find_item_location_by_anchor(list, target) {
            return Ok(location);
        }
    }

    // Try to find by exact text match
    if let Some(location) = find_item_location_by_text(list, target) {
        return Ok(location);
    }

    // Check if it's an index reference (#N)
    if let Some(number_str) = target.strip_prefix('#') {
        if let Ok(idx) = number_str.parse::<usize>() {
            if let Some(location) = find_item_location_by_global_index(list, idx - 1) {
                return Ok(location);
            }
        }
    }

    // Fallback to fuzzy matching (simple contains for now)
    let all_items: Vec<ListItem> = list.all_items().cloned().collect();
    let matches = crate::models::fuzzy_find(&all_items, target, threshold);
    match matches.len() {
        0 => anyhow::bail!("No item matching '{}' found", target),
        1 => {
            let target_anchor = &all_items[matches[0]].anchor;
            if let Some(location) = find_item_location_by_anchor(list, target_anchor) {
                Ok(location)
            } else {
                anyhow::bail!("Internal error: anchor not found")
            }
        }
        _ => anyhow::bail!(
            "Multiple items match '{}', please use a more specific query",
            target
        ),
    }
}

/// Represents the location of an item within the list structure
#[derive(Debug)]
pub enum ItemLocation {
    Uncategorized(usize),
    Categorized { category_index: usize, item_index: usize },
}

/// Find item location by anchor
fn find_item_location_by_anchor(list: &List, anchor: &str) -> Option<ItemLocation> {
    // Check uncategorized items
    if let Some(idx) = list.uncategorized_items.iter().position(|item| item.anchor == anchor) {
        return Some(ItemLocation::Uncategorized(idx));
    }
    
    // Check categorized items
    for (cat_idx, category) in list.categories.iter().enumerate() {
        if let Some(item_idx) = category.items.iter().position(|item| item.anchor == anchor) {
            return Some(ItemLocation::Categorized {
                category_index: cat_idx,
                item_index: item_idx,
            });
        }
    }
    
    None
}

/// Find item location by text
fn find_item_location_by_text(list: &List, text: &str) -> Option<ItemLocation> {
    let text_lower = text.to_lowercase();
    
    // Check uncategorized items
    if let Some(idx) = list.uncategorized_items.iter().position(|item| item.text.to_lowercase() == text_lower) {
        return Some(ItemLocation::Uncategorized(idx));
    }
    
    // Check categorized items
    for (cat_idx, category) in list.categories.iter().enumerate() {
        if let Some(item_idx) = category.items.iter().position(|item| item.text.to_lowercase() == text_lower) {
            return Some(ItemLocation::Categorized {
                category_index: cat_idx,
                item_index: item_idx,
            });
        }
    }
    
    None
}

/// Find item location by global index
fn find_item_location_by_global_index(list: &List, global_index: usize) -> Option<ItemLocation> {
    let mut current_index = 0;
    
    // Check uncategorized items
    if global_index < list.uncategorized_items.len() {
        return Some(ItemLocation::Uncategorized(global_index));
    }
    current_index += list.uncategorized_items.len();
    
    // Check categorized items
    for (cat_idx, category) in list.categories.iter().enumerate() {
        if global_index < current_index + category.items.len() {
            let item_idx = global_index - current_index;
            return Some(ItemLocation::Categorized {
                category_index: cat_idx,
                item_index: item_idx,
            });
        }
        current_index += category.items.len();
    }
    
    None
}

/// Remove all items from a list, returning the number of removed entries
pub fn wipe_list(list_name: &str) -> Result<usize> {
    let mut list = load_list(list_name)?;
    let removed = list.uncategorized_items.len() + list.categories.iter().map(|c| c.items.len()).sum::<usize>();
    if removed == 0 {
        return Ok(0);
    }
    list.uncategorized_items.clear();
    list.categories.clear();
    list.metadata.updated = chrono::Utc::now();
    save_list_with_path(&list, list_name)?;
    Ok(removed)
}
