use anyhow::{bail, Context, Result};
use colored::{ColoredString, Colorize};
use serde_json;
use std::io::{self, BufRead};

use crate::cli::DlCmd;
use crate::storage;
use crate::{models::ItemStatus, storage::notes::delete_note};
use chrono::{Local, Utc};
use std::path::Path;
use std::process::Command;

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
/// Handle daily list commands: create/display/add/done for YYYYMMDD_daily_list
pub fn daily_list(cmd: Option<&DlCmd>, json: bool) -> Result<()> {
    let date = Local::now().format("%Y%m%d").to_string();
    let list_name = format!("{}_daily_list", date);
    // No subcommand: ensure exists then display
    match cmd {
        Some(DlCmd::Add { item }) => {
            add_item(&list_name, item, json)?;
        }
        Some(DlCmd::Done { item }) => {
            mark_done(&list_name, item, json)?;
        }
        None => {
            // create if missing
            if storage::markdown::load_list(&list_name).is_err() {
                storage::markdown::create_list(&list_name)?;
            }
            display_list(&list_name, json)?;
        }
    }
    Ok(())
}
/// Handle daily note: create or open YYYYMMDD_daily_note.md
pub fn daily_note(_json: bool) -> Result<()> {
    let date = Local::now().format("%Y%m%d").to_string();
    let notes_dir = storage::get_notes_dir()?;
    let filename = format!("{}_daily_note.md", date);
    let path = notes_dir.join(&filename);
    // create if missing
    if !path.exists() {
        let now = Utc::now().to_rfc3339();
        let title = filename.trim_end_matches(".md");
        let content = format!("---\ntitle: \"{}\"\ncreated: {}\n---\n\n", title, now);
        std::fs::write(&path, content)
            .context(format!("Failed to create daily note: {}", path.display()))?;
    }
    // open in editor
    open_editor(&path)
}

/// Handle the 'ls' command to list all lists
pub fn list_notes(json: bool) -> Result<()> {
    let notes = storage::list_notes()?;

    if json {
        println!("{}", serde_json::to_string(&notes)?);
        return Ok(());
    }

    if notes.is_empty() {
        println!("No notes found. Create one with 'lst note new <list>'");
        return Ok(());
    }

    println!("Available notes:");
    for note in notes {
        println!("  {}", note);
    }

    Ok(())
}

/// Create a new note: initializes file and opens in editor
pub fn note_new(title: &str) -> Result<()> {
    // Normalize title (omit .md)
    let key = title.trim_end_matches(".md");
    // Create the note file (with frontmatter)
    let path = storage::notes::create_note(key).context("Failed to create note")?;
    // Open in editor
    open_editor(&path)
}

/// Open an existing note in the editor
pub fn note_open(title: &str) -> Result<()> {
    // Resolve note (allow fuzzy and omit .md)
    let key = title.trim_end_matches(".md");
    let note = resolve_note(key)?;
    let path = storage::notes::load_note(&note).context("Failed to load note")?;
    open_editor(&path)
}
/// Append text to an existing note (or create one), then open in editor
pub fn note_add(title: &str, text: &str) -> Result<()> {
    // Resolve note key for append (omit .md)
    let key = title.trim_end_matches(".md");
    let note = resolve_note(key).unwrap_or_else(|_| key.to_string());
    // Append to note, creating if missing
    let path = storage::notes::append_to_note(&note, text).context("Failed to append to note")?;
    // Inform user of success
    println!(
        "Appended to note '{}' (file: {})",
        title.cyan(),
        path.display()
    );
    Ok(())
}

/// Delete a note
pub fn note_delete(title: &str) -> Result<()> {
    // Determine the note file path
    // Resolve note to delete
    let key = title.trim_end_matches(".md");
    let note = resolve_note(key)?;
    delete_note(&note)
}

/// Spawn the user's editor (from $EDITOR or default 'vi') on the given path
fn open_editor(path: &Path) -> Result<()> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = Command::new(editor)
        .arg(path)
        .status()
        .context("Failed to launch editor")?;
    if !status.success() {
        anyhow::bail!("Editor exited with non-zero status");
    }
    Ok(())
}
/// Normalize a list identifier: strip .md and fuzzy-match existing, or allow new
fn normalize_list(input: &str) -> Result<String> {
    let key = input.trim_end_matches(".md");
    let lists = storage::list_lists()?;
    if lists.contains(&key.to_string()) {
        return Ok(key.to_string());
    }
    let matches: Vec<&String> = lists.iter().filter(|l| l.contains(key)).collect();
    if matches.len() == 1 {
        return Ok(matches[0].clone());
    }
    Ok(key.to_string())
}
/// Normalize a note identifier: strip .md and fuzzy-match existing, or allow new
fn normalize_note(input: &str) -> Result<String> {
    let key = input.trim_end_matches(".md");
    let notes = storage::list_notes()?;
    if notes.contains(&key.to_string()) {
        return Ok(key.to_string());
    }
    let matches: Vec<&String> = notes.iter().filter(|n| n.contains(key)).collect();
    if !matches.is_empty() {
        // Fuzzy match: take first matching note
        return Ok(matches[0].clone());
    }
    bail!("No note matching '{}' found", key)
}
/// Resolve a note identifier: strip .md and fuzzy-match to exactly one or error
fn resolve_note(input: &str) -> Result<String> {
    let key = input.trim_end_matches(".md");
    let notes = storage::list_notes()?;
    if notes.contains(&key.to_string()) {
        return Ok(key.to_string());
    }
    let matches: Vec<&String> = notes.iter().filter(|n| n.contains(key)).collect();
    match matches.len() {
        1 => Ok(matches[0].clone()),
        0 => bail!("No note matching '{}' found", input),
        _ => bail!("Multiple notes match '{}': {:?}", input, matches),
    }
}

/// Handle the 'add' command to add an item to a list
pub fn add_item(list: &str, text: &str, json: bool) -> Result<()> {
    // Try to load the list, create it if it doesn't exist
    // Resolve list name (omit .md, fuzzy match)
    let list_name = normalize_list(list)?;
    let list_result = storage::markdown::load_list(&list_name);
    if list_result.is_err() {
        storage::markdown::create_list(&list_name)?;
    }

    // Split by commas and trim whitespace
    let items: Vec<&str> = text.split(',').map(|s| s.trim()).collect();
    let mut added_items = Vec::new();

    for item_text in items {
        if !item_text.is_empty() {
            let item = storage::markdown::add_item(&list_name, item_text)?;
            added_items.push(item);
        }
    }

    if json {
        println!("{}", serde_json::to_string(&added_items)?);
        return Ok(());
    }

    if added_items.len() == 1 {
        println!("Added to {}: {}", list_name.cyan(), added_items[0].text);
    } else {
        println!("Added {} items to {}:", added_items.len(), list.cyan());
        for item in added_items {
            println!("  {}", item.text);
        }
    }

    Ok(())
}

/// Handle the 'done' command to mark an item as done
pub fn mark_done(list: &str, target: &str, json: bool) -> Result<()> {
    let list_name = normalize_list(list)?;
    let item = storage::markdown::mark_done(&list_name, target)?;

    if json {
        println!("{}", serde_json::to_string(&item)?);
        return Ok(());
    }

    println!("Marked done in {}: {}", list_name.cyan(), item.text);

    Ok(())
}
/// Handle the 'rm' command to remove an item from a list
pub fn remove_item(list: &str, target: &str, json: bool) -> Result<()> {
    let list_name = normalize_list(list)?;
    
    // Use the storage layer implementation
    let removed = storage::markdown::delete_item(&list_name, target)
        .with_context(|| format!("Failed to delete '{}' from {}", target, list_name))?;

    if json {
        println!("{}", serde_json::to_string(&removed)?);
    } else {
        println!("Deleted from {}: {}", list_name.cyan(), removed.text);
    }
    Ok(())
}

/// Handle the 'pipe' command to read items from stdin
pub fn pipe(list: &str, json: bool) -> Result<()> {
    // Try to load the list, create it if it doesn't exist
    let list_name = normalize_list(list)?;
    let list_result = storage::markdown::load_list(&list_name);
    if list_result.is_err() {
        storage::markdown::create_list(list)?;
    }

    let stdin = io::stdin();
    let mut count = 0;

    for line in stdin.lock().lines() {
        let line = line?;
        if !line.trim().is_empty() {
            storage::markdown::add_item(&list_name, &line)?;
            count += 1;
        }
    }

    if json {
        println!("{{\"added\": {}}}", count);
        return Ok(());
    }

    println!("Added {} items to {}", count, list_name.cyan());

    Ok(())
}

/// Handle displaying a list
pub fn display_list(list: &str, json: bool) -> Result<()> {
    let list_name = normalize_list(list)?;
    let list = storage::markdown::load_list(&list_name)?;

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

        println!(
            "#{} {} {} {}",
            idx + 1,
            checkbox,
            item.text,
            item.anchor.dimmed()
        );
    }

    Ok(())
}
