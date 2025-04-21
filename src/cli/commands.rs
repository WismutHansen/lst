use anyhow::Result;
use colored::{Colorize, ColoredString};
use std::io::{self, BufRead};
use serde_json;

use crate::storage;
use crate::models::ItemStatus;

/// Handle the 'ls' command to list all lists
pub fn list_lists(json: bool) -> Result<()> {
    let lists = storage::list_lists()?;
    
    if json {
        println!("{}", serde_json::to_string(&lists)?);
        return Ok(());
    }
    
    if lists.is_empty() {
        println!("No lists found. Create one with 'lst add <list> <text>'");
        return Ok(());
    }
    
    println!("Available lists:");
    for list in lists {
        println!("  {}", list);
    }
    
    Ok(())
}

/// Handle the 'add' command to add an item to a list
pub fn add_item(list: &str, text: &str, json: bool) -> Result<()> {
    // Try to load the list, create it if it doesn't exist
    let list_result = storage::markdown::load_list(list);
    if list_result.is_err() {
        storage::markdown::create_list(list)?;
    }
    
    let item = storage::markdown::add_item(list, text)?;
    
    if json {
        println!("{}", serde_json::to_string(&item)?);
        return Ok(());
    }
    
    println!("Added to {}: {}", list.cyan(), text);
    
    Ok(())
}

/// Handle the 'done' command to mark an item as done
pub fn mark_done(list: &str, target: &str, json: bool) -> Result<()> {
    let item = storage::markdown::mark_done(list, target)?;
    
    if json {
        println!("{}", serde_json::to_string(&item)?);
        return Ok(());
    }
    
    println!("Marked done in {}: {}", list.cyan(), item.text);
    
    Ok(())
}

/// Handle the 'pipe' command to read items from stdin
pub fn pipe(list: &str, json: bool) -> Result<()> {
    // Try to load the list, create it if it doesn't exist
    let list_result = storage::markdown::load_list(list);
    if list_result.is_err() {
        storage::markdown::create_list(list)?;
    }
    
    let stdin = io::stdin();
    let mut count = 0;
    
    for line in stdin.lock().lines() {
        let line = line?;
        if !line.trim().is_empty() {
            storage::markdown::add_item(list, &line)?;
            count += 1;
        }
    }
    
    if json {
        println!("{{\"added\": {}}}", count);
        return Ok(());
    }
    
    println!("Added {} items to {}", count, list.cyan());
    
    Ok(())
}

/// Handle displaying a list
pub fn display_list(list: &str, json: bool) -> Result<()> {
    let list = storage::markdown::load_list(list)?;
    
    if json {
        println!("{}", serde_json::to_string(&list)?);
        return Ok(());
    }
    
    println!("{}:", list.metadata.title.cyan().bold());
    
    if list.items.is_empty() {
        println!("  No items in list");
        return Ok(());
    }
    
    for (idx, item) in list.items.iter().enumerate() {
        let checkbox: ColoredString = match item.status {
            ItemStatus::Todo => "[ ]".into(),
            ItemStatus::Done => "[x]".green(),
        };
        
        println!("#{} {} {} {}", idx + 1, checkbox, item.text, item.anchor.dimmed());
    }
    
    Ok(())
}