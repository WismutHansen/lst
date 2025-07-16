use anyhow::Result;
use chrono;
use crate::storage;

/// Add an item to a list
pub async fn add_item(list: &str, text: &str, _json: bool) -> Result<()> {
    // Resolve list name (omit .md, fuzzy match)
    let list_name = normalize_list(list)?;
    let list_result = storage::markdown::load_list(&list_name);
    if list_result.is_err() {
        storage::markdown::create_list(&list_name)?;
    }

    // Split by commas and trim whitespace
    let items: Vec<&str> = text.split(',').map(|s| s.trim()).collect();

    for item_text in items {
        if !item_text.is_empty() {
            storage::markdown::add_item(&list_name, item_text)?;
        }
    }

    Ok(())
}

/// Mark an item as done
pub async fn mark_done(list: &str, target: &str, _json: bool) -> Result<()> {
    let list_name = normalize_list(list)?;
    storage::markdown::mark_done(&list_name, target)?;
    Ok(())
}

/// Mark an item as undone
pub async fn mark_undone(list: &str, target: &str, _json: bool) -> Result<()> {
    let list_name = normalize_list(list)?;
    storage::markdown::mark_undone(&list_name, target)?;
    Ok(())
}

/// Remove an item from a list
pub async fn remove_item(list: &str, target: &str, _json: bool) -> Result<()> {
    let list_name = normalize_list(list)?;
    storage::markdown::delete_item(&list_name, target)?;
    Ok(())
}

// Helper function to normalize list names
fn normalize_list(list: &str) -> Result<String> {
    let key = list.trim_end_matches(".md");
    
    // Handle special case: "dl" resolves to today's daily list
    if key == "dl" {
        let date = chrono::Local::now().format("%Y%m%d").to_string();
        return Ok(format!("daily_lists/{}_daily_list", date));
    }
    
    // For other cases, return as-is (simplified version without fuzzy matching)
    Ok(key.to_string())
}

// Helper function to normalize note names
fn normalize_note(note: &str) -> Result<String> {
    let key = note.trim_end_matches(".md");
    
    // Handle special case: "dn" resolves to today's daily note
    if key == "dn" {
        let date = chrono::Local::now().format("%Y%m%d").to_string();
        return Ok(format!("daily_notes/{}_daily_note", date));
    }
    
    // For other cases, return as-is (simplified version without fuzzy matching)
    Ok(key.to_string())
}