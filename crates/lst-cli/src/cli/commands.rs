use anyhow::{bail, Context, Result};
use colored::{ColoredString, Colorize};
use fuzzy_matcher::FuzzyMatcher;
use serde_json;
use serde_yaml;
use std::io::{self, BufRead, IsTerminal};

use crate::cli::{DlCmd, SyncCommands};
use crate::config::{get_config, Config};
use crate::storage;
use crate::{models::ItemStatus, storage::notes::delete_note};
use chrono::{Local, Utc};
use lst_core::config::State;
use lst_core::models::Category;
use std::path::Path;
use std::process::{Command, Stdio};

/// Create a new list: initializes file and opens in editor
pub fn new_list(title: &str) -> Result<()> {
    let key = title.trim_end_matches(".md");
    let path = storage::markdown::create_list(key).context("Failed to create note")?;
    open_editor(&path)
}

/// Handle the 'ls' command to list all lists
pub fn list_lists(json: bool) -> Result<()> {
    let lists = storage::list_lists()?;

    if json {
        println!("{}", serde_json::to_string(&lists)?);
        return Ok(());
    }

    if lists.is_empty() {
        println!("No lists found. Create one with 'lst new <list>'");
        return Ok(());
    }

    // Check if output is going to a terminal or is being piped
    if std::io::stdout().is_terminal() {
        // Human-readable format with header and indentation
        println!("Available lists:");
        for list in lists {
            println!("  {}", list);
        }
    } else {
        // Machine-readable format for pipes (no header, no indentation)
        for list in lists {
            println!("{}", list);
        }
    }

    Ok(())
}
/// Handle daily list commands: create/display/add/done/undone for YYYYMMDD_daily_list
pub async fn daily_list(cmd: Option<&DlCmd>, json: bool) -> Result<()> {
    let date = Local::now().format("%Y%m%d").to_string();
    let list_name = format!("daily_lists/{}_daily_list", date);
    // No subcommand: ensure exists then display
    match cmd {
        Some(DlCmd::Add { item }) => {
            add_item(&list_name, item, None, json).await?;
        }
        Some(DlCmd::Done { item }) => {
            mark_done(&list_name, item, json).await?;
        }
        Some(DlCmd::Undone { item }) => {
            mark_undone(&list_name, item, json).await?;
        }
        Some(DlCmd::List) => {
            display_daily_list(json)?;
        }
        Some(DlCmd::Remove { item }) => {
            remove_item(&list_name, item, json).await?;
        }
        None => {
            // create if missing
            if storage::markdown::load_list(&list_name).is_err() {
                storage::markdown::create_list(&list_name)?;
            }
            display_list(&list_name, json, false)?;
        }
    }
    Ok(())
}
/// Handle daily note: create or open YYYYMMDD_daily_note.md
pub fn daily_note(_json: bool) -> Result<()> {
    let date = Local::now().format("%Y%m%d").to_string();
    let notes_dir = storage::get_notes_dir()?;
    let filename = format!("daily_notes/{}_daily_note.md", date);
    let path = notes_dir.join(&filename);

    // create if missing
    if !path.exists() {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .context(format!("Failed to create directory: {}", parent.display()))?;
            }
        }

        let now = Utc::now().to_rfc3339();
        let title = format!("{}_daily_note", date);
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

    // Check if output is going to a terminal or is being piped
    if std::io::stdout().is_terminal() {
        // Human-readable format with header and indentation
        println!("Available notes:");
        for note in notes {
            println!("  {}", note);
        }
    } else {
        // Machine-readable format for pipes (no header, no indentation)
        for note in notes {
            println!("{}", note);
        }
    }

    Ok(())
}

/// Create a new note: initializes file and opens in editor
pub async fn note_new(title: &str) -> Result<()> {
    // Resolve note name (handle special cases like 'dn')
    let key = resolve_note(title).unwrap_or_else(|_| title.trim_end_matches(".md").to_string());
    // Create the note file (with frontmatter)
    let path = storage::notes::create_note(&key).context("Failed to create note")?;

    // Notify desktop app that a note was updated
    #[cfg(feature = "gui")]
    {
        let _ = notify_note_updated(&key).await;
    }

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
pub async fn note_add(title: &str, text: &str) -> Result<()> {
    // Resolve note key for append (omit .md)
    let key = title.trim_end_matches(".md");
    let note = resolve_note(key).unwrap_or_else(|_| key.to_string());
    // Append to note, creating if missing
    let path = storage::notes::append_to_note(&note, text).context("Failed to append to note")?;

    // Notify desktop app that a note was updated
    #[cfg(feature = "gui")]
    {
        let _ = notify_note_updated(&note).await;
    }

    open_editor(&path)
}

/// Delete a note
pub async fn note_delete(title: &str, force: bool) -> Result<()> {
    // Determine the note file path
    // Resolve note to delete
    let key = title.trim_end_matches(".md");
    let note = resolve_note(key)?;
    
    // Check if confirmation is needed
    let config = get_config();
    let need_confirm = config.ui.confirm_delete && !force;
    
    if need_confirm {
        use dialoguer::Confirm;
        let prompt = format!("Delete note '{}.md'?", note);
        let proceed = Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()?;
        if !proceed {
            println!("Aborted");
            return Ok(());
        }
    }
    
    let result = delete_note(&note);

    // Notify desktop app that a note was updated (deleted)
    #[cfg(feature = "gui")]
    {
        let _ = notify_note_updated(&note).await;
    }

    result
}

/// Display note content with metadata
pub fn note_show(title: &str, json: bool) -> Result<()> {
    use uuid::Uuid;

    let key = title.trim_end_matches(".md");
    let note = resolve_note(key)?;
    let path = storage::notes::load_note(&note).context("Failed to load note")?;

    if !path.exists() {
        bail!("Note '{}' does not exist", title);
    }

    let content = std::fs::read_to_string(&path)
        .context(format!("Failed to read note: {}", path.display()))?;

    let mut frontmatter = NoteFrontmatter::default();
    let body: String;

    if content.starts_with("---") {
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() >= 3 {
            if let Ok(fm) = serde_yaml::from_str::<NoteFrontmatter>(parts[1]) {
                frontmatter = fm;
            }
            body = parts[2].trim_start_matches('\n').to_string();
        } else {
            body = content.clone();
        }
    } else {
        body = content.clone();
    }

    if json {
        let id = Uuid::new_v5(&Uuid::NAMESPACE_OID, path.to_string_lossy().as_bytes()).to_string();

        let mut metadata = serde_json::Map::new();
        if let Some(ref created) = frontmatter.created {
            metadata.insert(
                "created".to_string(),
                serde_json::json!(created.to_rfc3339()),
            );
        }
        if let Some(ref updated) = frontmatter.updated {
            metadata.insert(
                "updated".to_string(),
                serde_json::json!(updated.to_rfc3339()),
            );
        }
        if let Some(ref tags) = frontmatter.tags {
            metadata.insert("tags".to_string(), serde_json::json!(tags));
        }
        if let Some(ref title_val) = frontmatter.title {
            metadata.insert("title".to_string(), serde_json::json!(title_val));
        }

        let output = serde_json::json!({
            "id": id,
            "title": frontmatter.title.as_ref().unwrap_or(&note),
            "path": path.to_string_lossy(),
            "content": body,
            "metadata": metadata
        });

        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!(
            "Title: {}",
            frontmatter.title.as_ref().unwrap_or(&note).cyan()
        );
        println!("Path: {}", path.display());
        if let Some(created) = frontmatter.created {
            println!("Created: {}", created.format("%Y-%m-%d %H:%M:%S UTC"));
        }
        if let Some(updated) = frontmatter.updated {
            println!("Updated: {}", updated.format("%Y-%m-%d %H:%M:%S UTC"));
        }
        if let Some(tags) = frontmatter.tags {
            println!("Tags: {}", tags.join(", "));
        }
        println!("\n{}", body);
    }

    Ok(())
}

/// Search for pattern in notes using ripgrep
pub fn note_grep(pattern: &str, json: bool) -> Result<()> {
    let notes_dir = storage::get_notes_dir()?;

    let output = Command::new("rg")
        .arg("--line-number")
        .arg("--no-heading")
        .arg("--with-filename")
        .arg("--color=never")
        .arg(pattern)
        .arg(&notes_dir)
        .output()
        .context("Failed to execute ripgrep. Make sure 'rg' is installed.")?;

    if !output.stderr.is_empty() {
        let stderr_msg = String::from_utf8_lossy(&output.stderr);
        bail!("ripgrep error: {}", stderr_msg);
    }

    if output.stdout.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("No matches found for pattern: {}", pattern);
        }
        return Ok(());
    }

    let results = String::from_utf8_lossy(&output.stdout);

    if json {
        let mut matches = Vec::new();

        for line in results.lines() {
            let parts: Vec<&str> = line.splitn(3, ':').collect();
            if parts.len() == 3 {
                let file_path = parts[0];
                let line_num = parts[1];
                let content = parts[2];

                let relative_path = if let Ok(stripped) =
                    std::path::Path::new(file_path).strip_prefix(&notes_dir)
                {
                    stripped.to_string_lossy().to_string()
                } else {
                    file_path.to_string()
                };

                let note_name = relative_path.trim_end_matches(".md").to_string();

                matches.push(serde_json::json!({
                    "note": note_name,
                    "line": line_num.parse::<u32>().unwrap_or(0),
                    "content": content.trim()
                }));
            }
        }

        println!("{}", serde_json::to_string_pretty(&matches)?);
    } else {
        for line in results.lines() {
            let parts: Vec<&str> = line.splitn(3, ':').collect();
            if parts.len() == 3 {
                let file_path = parts[0];
                let line_num = parts[1];
                let content = parts[2];

                let relative_path = if let Ok(stripped) =
                    std::path::Path::new(file_path).strip_prefix(&notes_dir)
                {
                    stripped.to_string_lossy().to_string()
                } else {
                    file_path.to_string()
                };

                let note_name = relative_path.trim_end_matches(".md");

                println!(
                    "{}:{} {}",
                    note_name.cyan(),
                    line_num.yellow(),
                    content.trim()
                );
            }
        }
    }

    Ok(())
}

/// Search for text in notes (simple text search, fixed string)
pub fn note_search(query: &str, json: bool) -> Result<()> {
    let notes_dir = storage::get_notes_dir()?;

    let output = Command::new("rg")
        .arg("--fixed-strings")
        .arg("--line-number")
        .arg("--no-heading")
        .arg("--with-filename")
        .arg("--color=never")
        .arg(query)
        .arg(&notes_dir)
        .output()
        .context("Failed to execute ripgrep. Make sure 'rg' is installed.")?;

    if output.stdout.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("No matches found for: {}", query);
        }
        return Ok(());
    }

    let results = String::from_utf8_lossy(&output.stdout);

    if json {
        let mut note_list: std::collections::HashSet<String> = std::collections::HashSet::new();

        for line in results.lines() {
            let parts: Vec<&str> = line.splitn(3, ':').collect();
            if parts.len() >= 2 {
                let file_path = parts[0];

                let relative_path = if let Ok(stripped) =
                    std::path::Path::new(file_path).strip_prefix(&notes_dir)
                {
                    stripped.to_string_lossy().to_string()
                } else {
                    file_path.to_string()
                };

                let note_name = relative_path.trim_end_matches(".md").to_string();
                note_list.insert(note_name);
            }
        }

        let mut notes: Vec<String> = note_list.into_iter().collect();
        notes.sort();

        println!("{}", serde_json::to_string_pretty(&notes)?);
    } else {
        let mut note_list: std::collections::HashSet<String> = std::collections::HashSet::new();

        for line in results.lines() {
            let parts: Vec<&str> = line.splitn(3, ':').collect();
            if parts.len() >= 2 {
                let file_path = parts[0];

                let relative_path = if let Ok(stripped) =
                    std::path::Path::new(file_path).strip_prefix(&notes_dir)
                {
                    stripped.to_string_lossy().to_string()
                } else {
                    file_path.to_string()
                };

                let note_name = relative_path.trim_end_matches(".md").to_string();
                note_list.insert(note_name);
            }
        }

        if note_list.is_empty() {
            println!("No notes found containing: {}", query);
        } else {
            println!("Notes containing '{}':", query);
            let mut notes: Vec<String> = note_list.into_iter().collect();
            notes.sort();
            for note in notes {
                println!("  {}", note.cyan());
            }
        }
    }

    Ok(())
}

/// Get note metadata without full content
pub fn note_metadata(title: &str, json: bool) -> Result<()> {
    let key = title.trim_end_matches(".md");
    let note = resolve_note(key)?;
    let path = storage::notes::load_note(&note).context("Failed to load note")?;

    if !path.exists() {
        bail!("Note '{}' does not exist", title);
    }

    let content = std::fs::read_to_string(&path)
        .context(format!("Failed to read note: {}", path.display()))?;

    let mut frontmatter = NoteFrontmatter::default();
    let body: String;

    if content.starts_with("---") {
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() >= 3 {
            if let Ok(fm) = serde_yaml::from_str::<NoteFrontmatter>(parts[1]) {
                frontmatter = fm;
            }
            body = parts[2].trim_start_matches('\n').to_string();
        } else {
            body = content.clone();
        }
    } else {
        body = content.clone();
    }

    let word_count = body.split_whitespace().count();
    let line_count = body.lines().count();

    if json {
        let mut output = serde_json::Map::new();

        output.insert(
            "title".to_string(),
            serde_json::json!(frontmatter.title.as_ref().unwrap_or(&note)),
        );

        if let Some(ref created) = frontmatter.created {
            output.insert(
                "created".to_string(),
                serde_json::json!(created.to_rfc3339()),
            );
        }

        if let Some(ref updated) = frontmatter.updated {
            output.insert(
                "updated".to_string(),
                serde_json::json!(updated.to_rfc3339()),
            );
        }

        output.insert("word_count".to_string(), serde_json::json!(word_count));
        output.insert("line_count".to_string(), serde_json::json!(line_count));

        if let Some(ref tags) = frontmatter.tags {
            output.insert("tags".to_string(), serde_json::json!(tags));
        }

        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Note Metadata:");
        println!(
            "  Title: {}",
            frontmatter.title.as_ref().unwrap_or(&note).cyan()
        );
        println!("  Path: {}", path.display());

        if let Some(created) = frontmatter.created {
            println!("  Created: {}", created.format("%Y-%m-%d %H:%M:%S UTC"));
        }

        if let Some(updated) = frontmatter.updated {
            println!("  Updated: {}", updated.format("%Y-%m-%d %H:%M:%S UTC"));
        }

        if let Some(tags) = frontmatter.tags {
            println!("  Tags: {}", tags.join(", "));
        }

        println!("  Word count: {}", word_count);
        println!("  Line count: {}", line_count);
    }

    Ok(())
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

    // Handle special case: "dl" resolves to today's daily list
    if key == "dl" {
        let date = chrono::Local::now().format("%Y%m%d").to_string();
        return Ok(format!("daily_lists/{}_daily_list", date));
    }

    // If it contains path separators, use as-is (directory path)
    if key.contains('/') || key.contains('\\') {
        return Ok(key.to_string());
    }

    // Otherwise try fuzzy matching
    let entries = storage::list_lists_with_info()?;

    // First try exact filename match
    for entry in &entries {
        if entry.name == key {
            return Ok(entry.relative_path.clone());
        }
    }

    // Then try fuzzy match by filename
    let config = crate::config::Config::load()?;
    let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();

    let mut fuzzy_matches: Vec<(&storage::FileEntry, i64)> = entries
        .iter()
        .filter_map(|entry| {
            matcher
                .fuzzy_match(&entry.name, key)
                .filter(|&score| score >= config.fuzzy.threshold)
                .map(|score| (entry, score))
        })
        .collect();

    // Sort by score (highest first)
    fuzzy_matches.sort_by(|a, b| b.1.cmp(&a.1));

    match fuzzy_matches.len() {
        0 => Ok(key.to_string()), // Allow new list creation
        1 => Ok(fuzzy_matches[0].0.relative_path.clone()),
        _ => {
            // Show top matches with scores
            let max_suggestions = config.fuzzy.max_suggestions as usize;
            let match_names: Vec<String> = fuzzy_matches
                .iter()
                .take(max_suggestions)
                .map(|(entry, score)| format!("{} (score: {})", entry.relative_path, score))
                .collect();
            bail!("Multiple lists match '{}': {}", key, match_names.join(", "));
        }
    }
}

/// Resolve a note identifier: strip .md and fuzzy-match to exactly one or error
fn resolve_note(input: &str) -> Result<String> {
    let key = input.trim_end_matches(".md");

    // Handle special case: "dn" resolves to today's daily note
    if key == "dn" {
        let date = chrono::Local::now().format("%Y%m%d").to_string();
        return Ok(format!("daily_notes/{}_daily_note", date));
    }

    // If it contains path separators, use as-is (directory path)
    if key.contains('/') || key.contains('\\') {
        return Ok(key.to_string());
    }

    // Otherwise try fuzzy matching
    let entries = storage::list_notes_with_info()?;

    // First try exact filename match
    for entry in &entries {
        if entry.name == key {
            return Ok(entry.relative_path.clone());
        }
    }

    // Then try fuzzy match by filename
    let config = crate::config::Config::load()?;
    let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();

    let mut fuzzy_matches: Vec<(&storage::FileEntry, i64)> = entries
        .iter()
        .filter_map(|entry| {
            matcher
                .fuzzy_match(&entry.name, key)
                .filter(|&score| score >= config.fuzzy.threshold)
                .map(|score| (entry, score))
        })
        .collect();

    // Sort by score (highest first)
    fuzzy_matches.sort_by(|a, b| b.1.cmp(&a.1));

    match fuzzy_matches.len() {
        0 => bail!("No note matching '{}' found", input),
        1 => Ok(fuzzy_matches[0].0.relative_path.clone()),
        _ => {
            // Show top matches with scores
            let max_suggestions = config.fuzzy.max_suggestions as usize;
            let match_names: Vec<String> = fuzzy_matches
                .iter()
                .take(max_suggestions)
                .map(|(entry, score)| format!("{} (score: {})", entry.relative_path, score))
                .collect();
            bail!(
                "Multiple notes match '{}': {}",
                input,
                match_names.join(", ")
            );
        }
    }
}

/// Resolve a note identifier: strip .md and fuzzy-match to exactly one or error
fn resolve_list(input: &str) -> Result<String> {
    let key = input.trim_end_matches(".md");

    // Handle special case: "dl" resolves to today's daily list
    if key == "dl" {
        let date = chrono::Local::now().format("%Y%m%d").to_string();
        return Ok(format!("daily_lists/{}_daily_list", date));
    }

    // If it contains path separators, use as-is (directory path)
    if key.contains('/') || key.contains('\\') {
        return Ok(key.to_string());
    }

    // Otherwise try fuzzy matching
    let entries = storage::list_lists_with_info()?;

    // First try exact filename match
    for entry in &entries {
        if entry.name == key {
            return Ok(entry.relative_path.clone());
        }
    }

    // Then try fuzzy match by filename
    let config = crate::config::Config::load()?;
    let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();

    let mut fuzzy_matches: Vec<(&storage::FileEntry, i64)> = entries
        .iter()
        .filter_map(|entry| {
            matcher
                .fuzzy_match(&entry.name, key)
                .filter(|&score| score >= config.fuzzy.threshold)
                .map(|score| (entry, score))
        })
        .collect();

    // Sort by score (highest first)
    fuzzy_matches.sort_by(|a, b| b.1.cmp(&a.1));

    match fuzzy_matches.len() {
        0 => bail!("No list matching '{}' found", input),
        1 => Ok(fuzzy_matches[0].0.relative_path.clone()),
        _ => {
            // Show top matches with scores
            let max_suggestions = config.fuzzy.max_suggestions as usize;
            let match_names: Vec<String> = fuzzy_matches
                .iter()
                .take(max_suggestions)
                .map(|(entry, score)| format!("{} (score: {})", entry.relative_path, score))
                .collect();
            bail!(
                "Multiple lists match '{}': {}",
                input,
                match_names.join(", ")
            );
        }
    }
}
/// Handle the 'open' command to open a list
pub fn open_list(list: &str) -> Result<()> {
    // Resolve list name (omit .md, fuzzy match)
    let key = list.trim_end_matches(".md");
    let name = resolve_list(key)?;
    let list = storage::markdown::load_list(&name).context("Failed to load list")?;
    let path = list.file_path();
    open_editor(&path)
}
/// Parse item text with category prefix (##category item)
fn parse_item_with_category(input: &str) -> (Option<String>, String) {
    if let Some(stripped) = input.strip_prefix("##") {
        // Format: "##category item text"
        if let Some(space_pos) = stripped.find(' ') {
            let category = stripped[..space_pos].to_string();
            let item_text = stripped[space_pos + 1..].to_string();
            (Some(category), item_text)
        } else {
            // Just "##category" - treat as uncategorized with ## prefix
            (None, input.to_string())
        }
    } else {
        // No category prefix
        (None, input.to_string())
    }
}

/// Handle the 'add' command to add an item to a list
pub async fn add_item(list: &str, text: &str, category: Option<&str>, json: bool) -> Result<()> {
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
            let (inline_category, text) = parse_item_with_category(item_text);
            // Inline category (##category) takes precedence over flag category
            let final_category = inline_category.as_deref().or(category);
            let item = storage::markdown::add_item_to_category(&list_name, &text, final_category)?;
            added_items.push(item);
        }
    }

    if json {
        println!("{}", serde_json::to_string(&added_items)?);
        return Ok(());
    }

    if added_items.len() == 1 {
        let category_info = if let Some(cat) = parse_item_with_category(text).0 {
            format!(" ({})", cat.cyan())
        } else {
            String::new()
        };
        println!(
            "Added to {}{}: {}",
            list_name.cyan(),
            category_info,
            added_items[0].text
        );
    } else {
        println!("Added {} items to {}:", added_items.len(), list.cyan());
        for item in added_items {
            println!("  {}", item.text);
        }
    }

    Ok(())
}

/// Handle the 'done' command to mark an item as done
pub async fn mark_done(list: &str, target: &str, json: bool) -> Result<()> {
    let list_name = normalize_list(list)?;
    let config = crate::config::Config::load()?;
    let items = storage::markdown::mark_done(&list_name, target, config.fuzzy.threshold)?;

    if json {
        println!("{}", serde_json::to_string(&items)?);
        return Ok(());
    }

    if items.len() == 1 {
        println!("Marked done in {}: {}", list_name.cyan(), items[0].text);
    } else {
        println!(
            "Marked {} items as done in {}:",
            items.len(),
            list_name.cyan()
        );
        for item in &items {
            println!("  {}", item.text);
        }
    }

    // Notify desktop app that the list was updated
    #[cfg(feature = "gui")]
    {
        let _ = notify_list_updated(&list_name).await;
    }

    Ok(())
}

/// Handle the 'undone' command to mark a completed item as not done
pub async fn mark_undone(list: &str, target: &str, json: bool) -> Result<()> {
    let list_name = normalize_list(list)?;
    let config = crate::config::Config::load()?;
    let items = storage::markdown::mark_undone(&list_name, target, config.fuzzy.threshold)?;

    if json {
        println!("{}", serde_json::to_string(&items)?);
        return Ok(());
    }

    if items.len() == 1 {
        println!("Marked undone in {}: {}", list_name.cyan(), items[0].text);
    } else {
        println!(
            "Marked {} items as undone in {}:",
            items.len(),
            list_name.cyan()
        );
        for item in &items {
            println!("  {}", item.text);
        }
    }

    // Notify desktop app that the list was updated
    #[cfg(feature = "gui")]
    {
        let _ = notify_list_updated(&list_name).await;
    }

    Ok(())
}

/// Handle the 'reset' command to mark all items in a list as undone
pub async fn reset_list(list: &str, json: bool) -> Result<()> {
    let list_name = normalize_list(list)?;
    let items = storage::markdown::reset_list(&list_name)?;

    if json {
        println!("{}", serde_json::to_string(&items)?);
        return Ok(());
    }

    if items.is_empty() {
        println!("No completed items found in {}", list_name.cyan());
    } else if items.len() == 1 {
        println!("Reset 1 item in {}: {}", list_name.cyan(), items[0].text);
    } else {
        println!("Reset {} items in {}:", items.len(), list_name.cyan());
        for item in &items {
            println!("  {}", item.text);
        }
    }

    // Notify desktop app that the list was updated
    #[cfg(feature = "gui")]
    {
        let _ = notify_list_updated(&list_name).await;
    }

    Ok(())
}

/// Handle the 'rm' command to remove an item from a list
pub async fn remove_item(list: &str, target: &str, json: bool) -> Result<()> {
    let list_name = normalize_list(list)?;
    let config = crate::config::Config::load()?;

    // Use the storage layer implementation
    let removed = storage::markdown::delete_item(&list_name, target, config.fuzzy.threshold)
        .with_context(|| format!("Failed to delete '{}' from {}", target, list_name))?;

    if json {
        println!("{}", serde_json::to_string(&removed)?);
        return Ok(());
    }

    if removed.len() == 1 {
        println!("Deleted from {}: {}", list_name.cyan(), removed[0].text);
    } else {
        println!("Deleted {} items from {}:", removed.len(), list_name.cyan());
        for item in &removed {
            println!("  {}", item.text);
        }
    }
    Ok(())
}

/// Handle the 'wipe' command to delete all entries from a list
pub fn wipe_list(list: &str, force: bool, json: bool) -> Result<()> {
    let list_name = normalize_list(list)?;
    if !force {
        use dialoguer::Confirm;
        let prompt = format!("Delete all items from '{}'?", list_name);
        let proceed = Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()?;
        if !proceed {
            println!("Aborted");
            return Ok(());
        }
    }

    let removed = storage::markdown::wipe_list(&list_name)?;

    if json {
        println!("{{\"deleted\": {}}}", removed);
    } else {
        println!("Deleted {} item(s) from {}", removed, list_name.cyan());
    }

    Ok(())
}

/// Handle the 'delete' command to delete a list file
pub fn delete_list(list: &str, force: bool, json: bool) -> Result<()> {
    let list_name = normalize_list(list)?;
    
    // Check if confirmation is needed
    let config = get_config();
    let need_confirm = config.ui.confirm_delete && !force;
    
    if need_confirm {
        use dialoguer::Confirm;
        let prompt = format!("Delete list file '{}.md'?", list_name);
        let proceed = Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()?;
        if !proceed {
            if json {
                println!("{{\"deleted\": false, \"message\": \"Aborted\"}}");
            } else {
                println!("Aborted");
            }
            return Ok(());
        }
    }

    storage::markdown::delete_list(&list_name)?;

    if json {
        println!("{{\"deleted\": true, \"list\": \"{}\"}}", list_name);
    } else {
        println!("Deleted list: {}", list_name.cyan());
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
pub fn display_list(list: &str, json: bool, clean: bool) -> Result<()> {
    let list_name = normalize_list(list)?;
    let list = storage::markdown::load_list(&list_name)?;

    if json {
        println!("{}", serde_json::to_string(&list)?);
        return Ok(());
    }

    println!("{}:", list.metadata.title.cyan().bold());

    // Check if list has any items at all
    let total_items = list.uncategorized_items.len()
        + list.categories.iter().map(|c| c.items.len()).sum::<usize>();
    if total_items == 0 {
        println!("  No items in list");
        return Ok(());
    }

    let mut item_counter = 1;

    // Display uncategorized items first
    for item in &list.uncategorized_items {
        let checkbox: ColoredString = match item.status {
            ItemStatus::Todo => "[ ]".into(),
            ItemStatus::Done => "[x]".green(),
        };

        let text = match item.status {
            ItemStatus::Todo => item.text.normal(),
            ItemStatus::Done => item.text.strikethrough(),
        };

        if clean {
            println!("#{} {} {}", item_counter, checkbox, text);
        } else {
            println!(
                "#{} {} {} {}",
                item_counter,
                checkbox,
                text,
                item.anchor.dimmed()
            );
        }
        item_counter += 1;
    }

    // Display categorized items
    for category in &list.categories {
        if !category.items.is_empty() {
            println!("\n{}:", category.name.cyan().bold());

            for item in &category.items {
                let checkbox: ColoredString = match item.status {
                    ItemStatus::Todo => "[ ]".into(),
                    ItemStatus::Done => "[x]".green(),
                };

                let text = match item.status {
                    ItemStatus::Todo => item.text.normal(),
                    ItemStatus::Done => item.text.strikethrough(),
                };

                if clean {
                    println!("#{} {} {}", item_counter, checkbox, text);
                } else {
                    println!(
                        "#{} {} {} {}",
                        item_counter,
                        checkbox,
                        text,
                        item.anchor.dimmed()
                    );
                }
                item_counter += 1;
            }
        }
    }

    Ok(())
}

/// Handle sync daemon commands
pub fn handle_sync_command(cmd: SyncCommands, json: bool) -> Result<()> {
    match cmd {
        SyncCommands::Setup { server } => sync_setup(server, json),
        SyncCommands::Start { foreground } => sync_start(foreground, json),
        SyncCommands::Stop => sync_stop(json),
        SyncCommands::Status => sync_status(json),
        SyncCommands::Logs { follow, lines } => sync_logs(follow, lines, json),
    }
}

/// Setup sync configuration (first login flow)
pub fn sync_setup(server: Option<String>, json: bool) -> Result<()> {
    use dialoguer::Input;

    let mut config = Config::load()?;
    config.init_sync()?;

    let server_url = if let Some(url) = server {
        url
    } else {
        Input::<String>::new()
            .with_prompt("Enter server URL (host:port format, e.g. 192.168.1.25:5673)")
            .allow_empty(true)
            .interact()?
    };

    // No auth_token needed - just set up the server URL
    // Authentication happens via lst auth request/verify flow

    if let Some(ref mut sync) = config.sync {
        sync.server_url = if server_url.is_empty() {
            None
        } else {
            // Store the URL in a normalized format for sync daemon
            if server_url.contains("://") {
                // Full URL provided, store as-is
                Some(server_url.clone())
            } else {
                // Host:port format, convert to WebSocket URL for sync
                let parts: Vec<&str> = server_url.split(':').collect();
                if parts.len() == 2 {
                    Some(format!("ws://{}:{}/api/sync", parts[0], parts[1]))
                } else {
                    Some(server_url.clone())
                }
            }
        };
    }

    config.save()?;

    if json {
        println!(
            "{{\"status\": \"configured\", \"server\": {:?}}}",
            server_url
        );
    } else {
        if server_url.is_empty() {
            println!("Configured for local-only mode");
        } else {
            println!("Configured to sync with: {}", server_url.cyan());
            println!("Next steps:");
            println!("  1. Run 'lst auth request <email>' to request authentication");
            println!("  2. Check your email for the verification token");
            println!("  3. Run 'lst auth verify <email> <token>' to complete setup");
            println!("  4. Run 'lst sync start' to start syncing");
        }
    }

    Ok(())
}

/// Start sync daemon
pub fn sync_start(foreground: bool, json: bool) -> Result<()> {
    // Check if syncd binary exists
    let syncd_path = find_syncd_binary()?;

    let mut cmd = Command::new(&syncd_path);
    if foreground {
        cmd.arg("--foreground");
    }
    cmd.arg("--verbose");

    if foreground {
        // Run in foreground
        let status = cmd.status()?;
        if !status.success() {
            bail!("lst-syncd exited with status: {}", status);
        }
    } else {
        // Start daemon in background
        cmd.stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null());

        let child = cmd.spawn()?;
        let pid = child.id();

        if json {
            println!("{{\"status\": \"started\", \"pid\": {}}}", pid);
        } else {
            println!("Sync daemon started (PID: {})", pid);
        }
    }

    Ok(())
}

/// Stop sync daemon
pub fn sync_stop(json: bool) -> Result<()> {
    // Find running lst-syncd process and stop it
    let output = Command::new("pkill").args(&["-f", "lst-syncd"]).output()?;

    if json {
        println!("{{\"status\": \"stopped\"}}");
    } else {
        if output.status.success() {
            println!("Sync daemon stopped");
        } else {
            println!("No sync daemon found running");
        }
    }

    Ok(())
}

/// Show sync daemon status
pub fn sync_status(json: bool) -> Result<()> {
    let config = get_config();

    // Check if syncd is configured
    let configured = config.sync.is_some();
    let server_url = config.sync.as_ref().and_then(|s| s.server_url.as_ref());
    // Auth token no longer used - authentication is JWT-only

    // Check if daemon is running
    let running = Command::new("pgrep")
        .args(&["-f", "lst-syncd"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if json {
        println!(
            "{{\"configured\": {}, \"running\": {}, \"server\": {:?}}}",
            configured, running, server_url
        );
    } else {
        println!("Sync Configuration:");
        println!(
            "  Configured: {}",
            if configured {
                "Yes".green()
            } else {
                "No".red()
            }
        );

        if let Some(url) = server_url {
            println!("  Server: {}", url.cyan());
        } else {
            println!("  Mode: {}", "Local-only".yellow());
        }

        // Auth token removed - authentication is via JWT only
        println!(
            "  Daemon: {}",
            if running {
                "Running".green()
            } else {
                "Stopped".red()
            }
        );

        if !configured {
            println!("\nRun 'lst sync setup' to configure sync settings");
        } else if !running {
            println!("\nRun 'lst sync start' to start the sync daemon");
        }
    }

    Ok(())
}

/// Show sync daemon logs
pub fn sync_logs(follow: bool, lines: usize, _json: bool) -> Result<()> {
    println!("Sync daemon logs (last {} lines):", lines);

    // For now, just indicate that logging isn't implemented yet
    println!("Log viewing not implemented yet - check system logs for lst-syncd");

    if follow {
        println!("Use 'lst sync start --foreground' to see live output");
    }

    Ok(())
}

/// Find the lst-syncd binary
fn find_syncd_binary() -> Result<String> {
    // Try common locations for lst-syncd
    let possible_paths = [
        "lst-syncd",                  // In PATH
        "./target/debug/lst-syncd",   // Local debug build
        "./target/release/lst-syncd", // Local release build
        &std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|parent| parent.join("lst-syncd")))
            .and_then(|p| p.to_str().map(String::from))
            .unwrap_or_default(),
    ];

    for path in possible_paths.iter() {
        if path.is_empty() {
            continue;
        }

        if Command::new(path).arg("--help").output().is_ok() {
            return Ok(path.to_string());
        }
    }

    bail!("lst-syncd binary not found. Make sure it's installed and in your PATH.");
}

/// List all daily lists
pub fn display_daily_list(json: bool) -> Result<()> {
    let date = Local::now().format("%Y%m%d").to_string();
    let list_name = format!("daily_lists/{}_daily_list", date);
    display_list(&list_name, json, false)
}

/// Share a document by updating writers and readers in the local sync database
pub fn share_document(doc: &str, writers: Option<&str>, readers: Option<&str>) -> Result<()> {
    use rusqlite::Connection;
    use uuid::Uuid;

    let state = State::load()?;
    let db_path = state
        .get_sync_database_path()
        .context("sync database path not configured")?;

    // Resolve document path (list or note)
    let key = doc.trim_end_matches(".md");
    let (path, kind) = match resolve_list(key) {
        Ok(p) => {
            let lists_dir = storage::get_lists_dir()?;
            (lists_dir.join(format!("{}.md", p)), "list")
        }
        Err(_) => {
            let note = resolve_note(key)?;
            let notes_dir = storage::get_notes_dir()?;
            (notes_dir.join(format!("{}.md", note)), "note")
        }
    };

    if !path.exists() {
        bail!("{} '{}' does not exist", kind, doc);
    }

    let doc_id = Uuid::new_v5(&Uuid::NAMESPACE_OID, path.to_string_lossy().as_bytes()).to_string();
    let conn = Connection::open(db_path)?;
    let affected = conn.execute(
        "UPDATE documents SET writers = ?2, readers = ?3 WHERE doc_id = ?1",
        rusqlite::params![doc_id, writers, readers],
    )?;

    if affected == 0 {
        bail!("Document not tracked in sync database: {}", doc);
    }

    println!("Updated share info for {}", doc);
    Ok(())
}

/// Remove sharing information from a document in the local sync database
pub fn unshare_document(doc: &str) -> Result<()> {
    share_document(doc, None, None)
}

pub async fn remote_switch_list(list_name: &str) -> Result<()> {
    let resolved_name = resolve_list(list_name)?;
    let client = reqwest::Client::new();
    let res = client
        .post(format!("http://localhost:33333/command/switch-list"))
        .body(resolved_name.clone())
        .send()
        .await?;

    if res.status().is_success() {
        println!("Switched list to {}", resolved_name);
    } else {
        bail!("Failed to switch list: {}", res.status());
    }

    Ok(())
}

pub async fn remote_show_message(message: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let res = client
        .post(format!("http://localhost:33333/command/show-message"))
        .body(message.to_string())
        .send()
        .await?;

    if res.status().is_success() {
        println!("Message sent to desktop app");
    } else {
        bail!("Failed to send message: {}", res.status());
    }

    Ok(())
}

/// Send notification to desktop app that a list was updated
#[cfg(feature = "gui")]
async fn notify_list_updated(list_name: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let res = client
        .post("http://localhost:33333/command/list-updated")
        .body(list_name.to_string())
        .send()
        .await;

    match res {
        Ok(response) if response.status().is_success() => {
            // Notification sent successfully, but don't print anything to avoid cluttering CLI output
        }
        _ => {
            // Silently ignore notification failures - the CLI should work even if GUI isn't running
        }
    }

    Ok(())
}

/// Send notification to desktop app that a note was updated
#[cfg(feature = "gui")]
async fn notify_note_updated(note_name: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let res = client
        .post("http://localhost:33333/command/note-updated")
        .body(note_name.to_string())
        .send()
        .await;

    match res {
        Ok(response) if response.status().is_success() => {
            // Notification sent successfully, but don't print anything to avoid cluttering CLI output
        }
        _ => {
            // Silently ignore notification failures - the CLI should work even if GUI isn't running
        }
    }

    Ok(())
}

/// Send notification to desktop app that a file was changed
#[cfg(feature = "gui")]
async fn notify_file_changed(file_path: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let res = client
        .post("http://localhost:33333/command/file-changed")
        .body(file_path.to_string())
        .send()
        .await;

    match res {
        Ok(response) if response.status().is_success() => {
            // Notification sent successfully, but don't print anything to avoid cluttering CLI output
        }
        _ => {
            // Silently ignore notification failures - the CLI should work even if GUI isn't running
        }
    }

    Ok(())
}

/// Send notification to desktop app that theme was changed
#[cfg(feature = "gui")]
async fn notify_theme_changed(theme_name: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let res = client
        .post("http://localhost:33333/command/theme-changed")
        .body(theme_name.to_string())
        .send()
        .await;

    match res {
        Ok(response) if response.status().is_success() => {
            // Notification sent successfully, but don't print anything to avoid cluttering CLI output
        }
        _ => {
            // Silently ignore notification failures - the CLI should work even if GUI isn't running
        }
    }

    Ok(())
}

/// Tidy all lists: ensure they have proper YAML frontmatter and formatting
pub fn tidy_lists(json: bool) -> Result<()> {
    let entries = storage::list_lists_with_info()?;
    let mut tidied_count = 0;
    let mut errors = Vec::new();

    for entry in entries {
        match tidy_single_list(&entry.relative_path) {
            Ok(was_modified) => {
                if was_modified {
                    tidied_count += 1;
                    if !json {
                        println!("Tidied: {}", entry.relative_path.cyan());
                    }
                }
            }
            Err(e) => {
                errors.push(format!("Error tidying '{}': {}", entry.relative_path, e));
            }
        }
    }

    if json {
        println!(
            "{{\"tidied\": {}, \"errors\": {}}}",
            tidied_count,
            errors.len()
        );
    } else {
        if tidied_count > 0 {
            println!("Tidied {} list(s)", tidied_count);
        } else {
            println!("All lists are already properly formatted");
        }

        if !errors.is_empty() {
            println!("\nErrors:");
            for error in errors {
                println!("  {}", error.red());
            }
        }
    }

    Ok(())
}

/// Tidy a single list file, returning whether it was modified
fn tidy_single_list(list_name: &str) -> Result<bool> {
    // Load the list (this will parse and normalize it)
    let mut list = storage::markdown::load_list(list_name)?;

    // Check if any items are missing proper anchors
    let mut was_modified = false;

    for item in &mut list.items {
        // Check if anchor is missing or invalid
        if item.anchor.is_empty() || !crate::models::is_valid_anchor(&item.anchor) {
            item.anchor = crate::models::generate_anchor();
            was_modified = true;
        }
    }

    // Always save to ensure proper formatting (frontmatter + item formatting)
    // The save operation will format everything properly
    let original_content = std::fs::read_to_string(get_list_file_path(list_name)?)?;
    storage::markdown::save_list_with_path(&list, list_name)?;
    let new_content = std::fs::read_to_string(get_list_file_path(list_name)?)?;

    // Check if the content actually changed
    if original_content != new_content {
        was_modified = true;
    }

    Ok(was_modified)
}

/// Helper to get the full file path for a list
fn get_list_file_path(list_name: &str) -> Result<std::path::PathBuf> {
    let lists_dir = storage::get_lists_dir()?;
    let filename = format!("{}.md", list_name);
    Ok(lists_dir.join(filename))
}

/// Structure of note frontmatter used for tidying
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct NoteFrontmatter {
    title: Option<String>,
    created: Option<chrono::DateTime<chrono::Utc>>,
    updated: Option<chrono::DateTime<chrono::Utc>>,
    tags: Option<Vec<String>>,
}

/// Tidy all notes: ensure they have proper YAML frontmatter
pub fn tidy_notes(json: bool) -> Result<()> {
    let entries = storage::list_notes_with_info()?;
    let mut tidied_count = 0;
    let mut errors = Vec::new();

    for entry in entries {
        match tidy_single_note(&entry.relative_path) {
            Ok(was_modified) => {
                if was_modified {
                    tidied_count += 1;
                    if !json {
                        println!("Tidied: {}", entry.relative_path.cyan());
                    }
                }
            }
            Err(e) => {
                errors.push(format!("Error tidying '{}': {}", entry.relative_path, e));
            }
        }
    }

    if json {
        println!(
            "{{\"tidied\": {}, \"errors\": {}}}",
            tidied_count,
            errors.len()
        );
    } else {
        if tidied_count > 0 {
            println!("Tidied {} note(s)", tidied_count);
        } else {
            println!("All notes are already properly formatted");
        }

        if !errors.is_empty() {
            println!("\nErrors:");
            for error in errors {
                println!("  {}", error.red());
            }
        }
    }

    Ok(())
}

/// Tidy a single note file, returning whether it was modified
fn tidy_single_note(note_name: &str) -> Result<bool> {
    let path = get_note_file_path(note_name)?;
    let original_content = std::fs::read_to_string(&path)?;

    let mut was_modified = false;
    let mut frontmatter: NoteFrontmatter = NoteFrontmatter::default();
    let body: String;

    if original_content.starts_with("---") {
        let parts: Vec<&str> = original_content.splitn(3, "---").collect();
        if parts.len() >= 3 {
            if let Ok(fm) = serde_yaml::from_str::<NoteFrontmatter>(parts[1]) {
                frontmatter = fm;
            } else {
                was_modified = true;
            }
            body = parts[2].to_string();
        } else {
            body = parts.last().unwrap_or(&"").to_string();
            was_modified = true;
        }
    } else {
        body = original_content.clone();
        was_modified = true;
    }

    if frontmatter.title.is_none() {
        let title = std::path::Path::new(note_name)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(note_name)
            .to_string();
        frontmatter.title = Some(title);
        was_modified = true;
    }
    if frontmatter.created.is_none() {
        frontmatter.created = Some(chrono::Utc::now());
        was_modified = true;
    }

    let fm_string = serde_yaml::to_string(&frontmatter)?;
    let new_content = format!("---\n{}---\n\n{}", fm_string, body.trim_start_matches('\n'));

    if new_content != original_content {
        std::fs::write(&path, new_content)?;
        was_modified = true;
    }

    Ok(was_modified)
}

/// Helper to get the full file path for a note
fn get_note_file_path(note_name: &str) -> Result<std::path::PathBuf> {
    let notes_dir = storage::get_notes_dir()?;
    let filename = format!("{}.md", note_name);
    Ok(notes_dir.join(filename))
}

// Authentication command implementations

/// Request authentication token from server
fn parse_server_config(server_url: &str) -> Result<(String, u16)> {
    // Handle different formats:
    // "192.168.1.25:5673" -> ("192.168.1.25", 5673)
    // "ws://192.168.1.25:5673/api/sync" -> ("192.168.1.25", 5673)
    // "http://example.com:8080" -> ("example.com", 8080)

    if let Ok(url) = url::Url::parse(server_url) {
        let host = url
            .host_str()
            .context("Invalid host in server URL")?
            .to_string();
        let port = url
            .port()
            .unwrap_or(if url.scheme() == "https" { 443 } else { 80 });
        Ok((host, port))
    } else if server_url.contains(':') {
        // Handle "host:port" format
        let parts: Vec<&str> = server_url.split(':').collect();
        if parts.len() == 2 {
            let host = parts[0].to_string();
            let port = parts[1].parse().context("Invalid port number")?;
            Ok((host, port))
        } else {
            Err(anyhow::anyhow!(
                "Invalid server URL format. Use 'host:port' or full URL"
            ))
        }
    } else {
        Err(anyhow::anyhow!(
            "Invalid server URL format. Use 'host:port' or full URL"
        ))
    }
}

fn build_http_url(host: &str, port: u16) -> String {
    format!("http://{}:{}", host, port)
}

fn build_websocket_url(host: &str, port: u16) -> String {
    format!("ws://{}:{}/api/sync", host, port)
}

/// Register new account with secure password handling (shows auth token)
pub async fn auth_register(email: &str, host: Option<&str>, json: bool) -> Result<()> {
    let config = get_config();
    let server_url = config
        .sync
        .as_ref()
        .and_then(|s| s.server_url.as_ref())
        .context("No server URL configured. Run 'lst sync setup' first.")?;

    let (host, port) = if let Some(h) = host {
        // If host override is provided, assume default port
        (h.to_string(), 5673)
    } else {
        parse_server_config(server_url)?
    };

    let http_base_url = build_http_url(&host, port);

    use argon2::password_hash::SaltString;
    use argon2::{Argon2, PasswordHasher};
    use dialoguer::Password;
    use std::hash::Hasher;

    // Securely prompt for password
    let password = Password::new()
        .with_prompt("Create account password")
        .with_confirmation("Confirm password", "Passwords don't match, try again")
        .interact()?;

    // Create deterministic salt from email for client-side hashing (same as existing code)
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write(email.as_bytes());
    hasher.write(b"lst-client-salt"); // Add app-specific salt component
    let email_hash = hasher.finish();

    // Convert hash to 16-byte array for salt
    let salt_bytes = email_hash.to_le_bytes();
    let mut full_salt = [0u8; 16];
    full_salt[..8].copy_from_slice(&salt_bytes);
    full_salt[8..].copy_from_slice(&salt_bytes); // Repeat to fill 16 bytes

    let salt = SaltString::encode_b64(&full_salt).expect("Failed to encode salt");
    let argon2 = Argon2::default(); // Use default params like mobile app
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("hashing failed")
        .to_string();

    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "email": email,
        "host": host,
        "password_hash": password_hash
    });

    let response = client
        .post(format!("{}/api/auth/request", http_base_url))
        .json(&payload)
        .send()
        .await?;

    if response.status().is_success() {
        let auth_response: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({"status":"ok"}));

        if json {
            println!("{}", serde_json::to_string_pretty(&auth_response)?);
        } else {
            println!("New account registered successfully for {}", email.green());
            println!("");
            println!("  Security Notice:");
            println!("  Your auth token is displayed on the SERVER CONSOLE for security reasons.");
            println!("  Check the server logs or scan the QR code displayed on the server.");
            println!("");
            println!("  IMPORTANT: Save your auth token safely!");
            println!("  You'll need it to login and access your encrypted data.");
            println!("  If you lose it, your encrypted data cannot be recovered.");
            println!("");
            println!("Once you have the auth token, complete login with:");
            println!("  lst auth login {} <auth-token>", email.cyan());
        }
    } else {
        let error_text = response.text().await?;
        bail!("Failed to register account: {}", error_text);
    }

    Ok(())
}

/// Login with email, auth token, and password (derives secure encryption key)
pub async fn auth_login(email: &str, auth_token: &str, json: bool) -> Result<()> {
    let config = get_config();
    let mut state = State::load()?;
    let server_url = config
        .sync
        .as_ref()
        .and_then(|s| s.server_url.as_ref())
        .context("No server URL configured. Run 'lst sync setup' first.")?;

    let (host, port) = parse_server_config(server_url)?;
    let http_base_url = build_http_url(&host, port);

    use dialoguer::Password;

    // Securely prompt for password (never print it)
    let password = Password::new().with_prompt("Account password").interact()?;

    // Derive secure encryption key using all three components
    let key_path = lst_core::crypto::get_master_key_path()?;
    match lst_core::crypto::derive_key_from_credentials(email, &password, auth_token) {
        Ok(derived_key) => {
            // Save the derived key
            if let Err(e) = lst_core::crypto::save_derived_key(&key_path, &derived_key) {
                eprintln!("Warning: Failed to save encryption key: {}", e);
            }

            // Store credentials for future use
            state.store_auth_credentials(email.to_string(), auth_token.to_string());

            // Get JWT token for server authentication
            let client = reqwest::Client::new();
            let payload = serde_json::json!({
                "email": email,
                "token": auth_token
            });

            let response = client
                .post(format!("{}/api/auth/verify", http_base_url))
                .json(&payload)
                .send()
                .await?;

            if response.status().is_success() {
                let verify_response: serde_json::Value = response.json().await?;

                if let Some(jwt) = verify_response.get("jwt").and_then(|j| j.as_str()) {
                    // Parse JWT to get expiration (basic extraction without validation)
                    let expires_at = chrono::Utc::now() + chrono::Duration::hours(1); // Default 1 hour

                    state.store_jwt(jwt.to_string(), expires_at);
                    state.save()?;

                    if json {
                        println!("{}", serde_json::to_string_pretty(&verify_response)?);
                    } else {
                        println!("Successfully logged in as {}", email.green());
                        println!("Secure encryption key derived and stored");
                        println!("JWT token stored and ready for sync");
                    }
                } else {
                    bail!("Invalid response: missing JWT token");
                }
            } else {
                let error_text = response.text().await?;
                bail!("Failed to verify auth token: {}", error_text);
            }
        }
        Err(e) => {
            bail!("Failed to derive encryption key: {}", e);
        }
    }

    Ok(())
}

pub async fn auth_request(email: &str, host: Option<&str>, json: bool) -> Result<()> {
    let config = get_config();
    let mut state = State::load()?;
    let server_url = config
        .sync
        .as_ref()
        .and_then(|s| s.server_url.as_ref())
        .context("No server URL configured. Run 'lst sync setup' first.")?;

    let (host, port) = if let Some(h) = host {
        // If host override is provided, assume default port
        (h.to_string(), 5673)
    } else {
        parse_server_config(server_url)?
    };

    let http_base_url = build_http_url(&host, port);

    use argon2::password_hash::SaltString;
    use argon2::{Argon2, PasswordHasher};
    use dialoguer::Password;
    use std::hash::Hasher;

    let password = Password::new().with_prompt("Account password").interact()?;

    // Create deterministic salt from email for client-side hashing (same as mobile)
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write(email.as_bytes());
    hasher.write(b"lst-client-salt"); // Add app-specific salt component
    let email_hash = hasher.finish();

    // Convert hash to 16-byte array for salt
    let salt_bytes = email_hash.to_le_bytes();
    let mut full_salt = [0u8; 16];
    full_salt[..8].copy_from_slice(&salt_bytes);
    full_salt[8..].copy_from_slice(&salt_bytes); // Repeat to fill 16 bytes

    let salt = SaltString::encode_b64(&full_salt).expect("Failed to encode salt");
    let argon2 = Argon2::default(); // Use default params like mobile app
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("hashing failed")
        .to_string();

    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "email": email,
        "host": host,
        "password_hash": password_hash
    });

    let response = client
        .post(format!("{}/api/auth/request", http_base_url))
        .json(&payload)
        .send()
        .await?;

    if response.status().is_success() {
        let auth_response: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({"status":"ok"}));

        // Store password hash for JWT refresh later
        state.store_auth_token(password_hash);
        state.save()?;

        if json {
            println!("{}", serde_json::to_string_pretty(&auth_response)?);
        } else {
            println!("Authentication token requested for {}", email.cyan());
            println!("Check your email or server logs for the token, then run:");
            println!("  lst auth verify {} <token>", email.cyan());
        }
    } else {
        let error_text = response.text().await?;
        bail!("Failed to request authentication token: {}", error_text);
    }

    Ok(())
}

/// Show current authentication status
pub fn auth_status(json: bool) -> Result<()> {
    let config = get_config();
    let state = State::load()?;

    let has_server_url = config
        .sync
        .as_ref()
        .and_then(|s| s.server_url.as_ref())
        .is_some();
    let has_jwt = state.auth.jwt_token.is_some();
    let jwt_valid = state.is_jwt_valid();

    if json {
        println!(
            "{}",
            serde_json::json!({
                "server_configured": has_server_url,
                "jwt_token_present": has_jwt,
                "jwt_valid": jwt_valid,
                "jwt_expires_at": state.auth.jwt_expires_at
            })
        );
    } else {
        println!("Authentication Status:");

        if !has_server_url {
            println!("  Server: {}", "Not configured".red());
            println!("  Run 'lst sync setup' to configure server URL");
        } else {
            println!(
                "  Server: {}",
                config
                    .sync
                    .as_ref()
                    .and_then(|s| s.server_url.as_ref())
                    .unwrap()
                    .cyan()
            );
        }

        if !has_jwt {
            println!("  JWT Token: {}", "Not present".red());
            println!("  Run 'lst auth request <email>' to authenticate");
        } else if jwt_valid {
            println!("  JWT Token: {}", "Valid".green());
            if let Some(expires_at) = state.auth.jwt_expires_at {
                println!("  Expires: {}", expires_at.format("%Y-%m-%d %H:%M:%S UTC"));
            }
        } else {
            println!("  JWT Token: {}", "Expired".yellow());
            println!("  Run 'lst auth request <email>' to re-authenticate");
        }
    }

    Ok(())
}

/// Remove stored authentication token
pub fn auth_logout(json: bool) -> Result<()> {
    let mut state = State::load()?;
    state.clear_jwt();
    state.save()?;

    if json {
        println!("{}", serde_json::json!({"status": "logged_out"}));
    } else {
        println!("Successfully logged out. JWT token removed.");
    }

    Ok(())
}

/// Refresh JWT token using stored auth token
pub async fn refresh_jwt_token(config: &Config, state: &mut State) -> Result<()> {
    let server_url = config
        .sync
        .as_ref()
        .and_then(|s| s.server_url.as_ref())
        .context("No server URL configured")?;

    let auth_token = state
        .get_auth_token()
        .context("No auth token stored. Run 'lst auth request <email>' to authenticate")?;

    let (host, port) = parse_server_config(server_url)?;
    let http_base_url = build_http_url(&host, port);

    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "password_hash": auth_token
    });

    let response = client
        .post(format!("{}/api/auth/refresh", http_base_url))
        .json(&payload)
        .send()
        .await?;

    if response.status().is_success() {
        let refresh_response: serde_json::Value = response.json().await?;

        if let Some(jwt) = refresh_response.get("jwt").and_then(|j| j.as_str()) {
            // Parse JWT to get expiration (basic extraction without validation)
            let expires_at = chrono::Utc::now() + chrono::Duration::hours(1); // Default 1 hour

            state.store_jwt(jwt.to_string(), expires_at);
            state.save()?;

            println!("JWT token refreshed successfully");
            Ok(())
        } else {
            bail!("Invalid refresh response: missing JWT token");
        }
    } else {
        let error_text = response.text().await?;
        bail!("Failed to refresh JWT token: {}. You may need to re-authenticate with 'lst auth request <email>'", error_text);
    }
}

/// Helper function to make authenticated requests to the server
pub async fn make_authenticated_request(
    method: reqwest::Method,
    endpoint: &str,
    body: Option<serde_json::Value>,
) -> Result<reqwest::Response> {
    let config = get_config();
    let mut state = State::load()?;

    let server_url = config
        .sync
        .as_ref()
        .and_then(|s| s.server_url.as_ref())
        .context("No server URL configured")?;

    let (host, port) = parse_server_config(server_url)?;
    let http_base_url = build_http_url(&host, port);

    // Check if JWT needs refresh before making the request
    if !state.is_jwt_valid() || state.needs_jwt_refresh() {
        if state.get_auth_token().is_some() {
            println!("JWT token expired or about to expire, refreshing...");
            if let Err(e) = refresh_jwt_token(&config, &mut state).await {
                eprintln!("Failed to refresh JWT token: {}", e);
                bail!("JWT token expired and refresh failed. Run 'lst auth request <email>' to re-authenticate");
            }
        } else {
            bail!("No valid JWT token and no auth token for refresh. Run 'lst auth request <email>' to authenticate");
        }
    }

    let jwt = state
        .get_jwt()
        .context("No valid JWT token after refresh attempt")?;

    let client = reqwest::Client::new();
    let mut request = client
        .request(
            method,
            format!("{}/{}", http_base_url, endpoint.trim_start_matches('/')),
        )
        .header("Authorization", format!("Bearer {}", jwt));

    if let Some(body) = body {
        request = request.json(&body);
    }

    let response = request.send().await?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        bail!("Authentication failed. JWT token may be expired. Run 'lst auth request <email>' to re-authenticate");
    }

    Ok(response)
}

// Server content management commands

/// Create content on the server
pub async fn server_create(kind: &str, path: &str, content: &str, json: bool) -> Result<()> {
    let payload = serde_json::json!({
        "kind": kind,
        "path": path,
        "content": content
    });

    let response =
        make_authenticated_request(reqwest::Method::POST, "/api/content", Some(payload)).await?;

    if response.status().is_success() {
        let result: serde_json::Value = response.json().await?;

        if json {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!("Successfully created {}/{}", kind.cyan(), path.cyan());
        }
    } else {
        let error_text = response.text().await?;
        bail!("Failed to create content: {}", error_text);
    }

    Ok(())
}

/// Get content from the server
pub async fn server_get(kind: &str, path: &str, json: bool) -> Result<()> {
    let endpoint = format!("/api/content/{}/{}", kind, path);

    let response = make_authenticated_request(reqwest::Method::GET, &endpoint, None).await?;

    if response.status().is_success() {
        let content = response.text().await?;

        if json {
            println!(
                "{}",
                serde_json::json!({
                    "kind": kind,
                    "path": path,
                    "content": content
                })
            );
        } else {
            println!("Content from {}/{}:", kind.cyan(), path.cyan());
            println!("{}", content);
        }
    } else if response.status() == reqwest::StatusCode::NOT_FOUND {
        if json {
            println!("{}", serde_json::json!({"error": "Content not found"}));
        } else {
            println!("Content not found: {}/{}", kind, path);
        }
    } else {
        let error_text = response.text().await?;
        bail!("Failed to get content: {}", error_text);
    }

    Ok(())
}

/// Update content on the server
pub async fn server_update(kind: &str, path: &str, content: &str, json: bool) -> Result<()> {
    let endpoint = format!("/api/content/{}/{}", kind, path);
    let payload = serde_json::json!({
        "content": content
    });

    let response =
        make_authenticated_request(reqwest::Method::PUT, &endpoint, Some(payload)).await?;

    if response.status().is_success() {
        let result: serde_json::Value = response.json().await?;

        if json {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!("Successfully updated {}/{}", kind.cyan(), path.cyan());
        }
    } else if response.status() == reqwest::StatusCode::NOT_FOUND {
        if json {
            println!("{}", serde_json::json!({"error": "Content not found"}));
        } else {
            bail!("Content not found: {}/{}", kind, path);
        }
    } else {
        let error_text = response.text().await?;
        bail!("Failed to update content: {}", error_text);
    }

    Ok(())
}

/// Delete content from the server
pub async fn server_delete(kind: &str, path: &str, json: bool) -> Result<()> {
    let endpoint = format!("/api/content/{}/{}", kind, path);

    let response = make_authenticated_request(reqwest::Method::DELETE, &endpoint, None).await?;

    if response.status().is_success() {
        let result: serde_json::Value = response.json().await?;

        if json {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!("Successfully deleted {}/{}", kind.cyan(), path.cyan());
        }
    } else if response.status() == reqwest::StatusCode::NOT_FOUND {
        if json {
            println!("{}", serde_json::json!({"error": "Content not found"}));
        } else {
            bail!("Content not found: {}/{}", kind, path);
        }
    } else {
        let error_text = response.text().await?;
        bail!("Failed to delete content: {}", error_text);
    }

    Ok(())
}

// Category management commands

/// Create a new category in a list
pub async fn category_add(list: &str, name: &str, json: bool) -> Result<()> {
    let list_name = normalize_list(list)?;
    let mut list_obj = storage::markdown::load_list(&list_name)?;

    // Check if category already exists
    if list_obj.categories.iter().any(|c| c.name == name) {
        bail!("Category '{}' already exists in list '{}'", name, list_name);
    }

    // Add empty category
    list_obj.categories.push(Category {
        name: name.to_string(),
        items: Vec::new(),
    });

    list_obj.metadata.updated = chrono::Utc::now();
    storage::markdown::save_list_with_path(&list_obj, &list_name)?;

    if json {
        println!(
            "{}",
            serde_json::json!({"status": "success", "message": format!("Created category '{}'", name)})
        );
    } else {
        println!("Created category '{}' in {}", name.cyan(), list_name.cyan());
    }

    Ok(())
}

/// Move an item to a different category
pub async fn category_move(list: &str, item: &str, category: &str, json: bool) -> Result<()> {
    let list_name = normalize_list(list)?;
    let mut list_obj = storage::markdown::load_list(&list_name)?;
    let config = crate::config::Config::load()?;

    // Find and remove the item from its current location
    let location =
        storage::markdown::find_item_for_removal(&list_obj, item, config.fuzzy.threshold)?;
    let moved_item = storage::markdown::remove_item_at_location(&mut list_obj, location);

    // Add to target category (create if doesn't exist)
    if let Some(cat) = list_obj.categories.iter_mut().find(|c| c.name == category) {
        cat.items.push(moved_item.clone());
    } else {
        // Create new category
        list_obj.categories.push(Category {
            name: category.to_string(),
            items: vec![moved_item.clone()],
        });
    }

    list_obj.metadata.updated = chrono::Utc::now();
    storage::markdown::save_list_with_path(&list_obj, &list_name)?;

    if json {
        println!(
            "{}",
            serde_json::json!({"status": "success", "item": moved_item, "category": category})
        );
    } else {
        println!(
            "Moved '{}' to category '{}' in {}",
            moved_item.text,
            category.cyan(),
            list_name.cyan()
        );
    }

    Ok(())
}

/// List all categories in a list
pub async fn category_list(list: &str, json: bool) -> Result<()> {
    let list_name = normalize_list(list)?;
    let list_obj = storage::markdown::load_list(&list_name)?;

    if json {
        let categories: Vec<_> = list_obj.categories.iter().map(|c| &c.name).collect();
        println!("{}", serde_json::to_string(&categories)?);
        return Ok(());
    }

    if list_obj.categories.is_empty() {
        println!("No categories in {}", list_name.cyan());
        return Ok(());
    }

    println!("Categories in {}:", list_name.cyan());
    for category in &list_obj.categories {
        println!("  {} ({} items)", category.name, category.items.len());
    }

    if !list_obj.uncategorized_items.is_empty() {
        println!(
            "  {} ({} items)",
            "(uncategorized)".dimmed(),
            list_obj.uncategorized_items.len()
        );
    }

    Ok(())
}

/// Remove a category (moves items to uncategorized)
pub async fn category_remove(list: &str, name: &str, json: bool) -> Result<()> {
    let list_name = normalize_list(list)?;
    let mut list_obj = storage::markdown::load_list(&list_name)?;

    // Find and remove the category
    if let Some(pos) = list_obj.categories.iter().position(|c| c.name == name) {
        let removed_category = list_obj.categories.remove(pos);
        let item_count = removed_category.items.len();

        // Move items to uncategorized
        list_obj.uncategorized_items.extend(removed_category.items);

        list_obj.metadata.updated = chrono::Utc::now();
        storage::markdown::save_list_with_path(&list_obj, &list_name)?;

        if json {
            println!(
                "{}",
                serde_json::json!({"status": "success", "moved_items": item_count})
            );
        } else {
            println!(
                "Removed category '{}' from {} ({} items moved to uncategorized)",
                name.cyan(),
                list_name.cyan(),
                item_count
            );
        }
    } else {
        bail!("Category '{}' not found in list '{}'", name, list_name);
    }

    Ok(())
}

// ============================================================================
// Theme Management Commands
// ============================================================================

/// List all available themes
pub fn theme_list(verbose: bool, json: bool) -> Result<()> {
    let config = Config::load()?;
    let loader = config.get_theme_loader();
    let themes = loader.list_themes();

    if json {
        if verbose {
            let mut theme_infos = Vec::new();
            for theme_name in themes {
                if let Ok(info) = loader.get_theme_info(&theme_name) {
                    theme_infos.push(info);
                }
            }
            println!("{}", serde_json::to_string_pretty(&theme_infos)?);
        } else {
            println!("{}", serde_json::to_string(&themes)?);
        }
        return Ok(());
    }

    if themes.is_empty() {
        println!("No themes found.");
        return Ok(());
    }

    println!("Available themes:");

    if verbose {
        for theme_name in themes {
            if let Ok(info) = loader.get_theme_info(&theme_name) {
                println!(
                    "  {} - {}",
                    info.name.cyan(),
                    info.description
                        .unwrap_or_else(|| "No description".to_string())
                        .dimmed()
                );
                if let Some(author) = info.author {
                    println!("    Author: {}", author.dimmed());
                }
                println!(
                    "    System: {:?}, Variant: {:?}",
                    info.system,
                    info.variant
                        .unwrap_or_else(|| lst_core::theme::ThemeVariant::Dark)
                );
                println!();
            }
        }
    } else {
        for theme_name in themes {
            println!("  {}", theme_name);
        }
    }

    Ok(())
}

/// Show information about the current theme
pub fn theme_current(json: bool) -> Result<()> {
    let config = get_config();
    let current_theme = config.get_theme()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&current_theme)?);
        return Ok(());
    }

    println!("Current theme: {}", current_theme.scheme.cyan());
    if let Some(name) = &current_theme.name {
        println!("Name: {}", name);
    }
    if let Some(author) = &current_theme.author {
        println!("Author: {}", author.dimmed());
    }
    if let Some(description) = &current_theme.description {
        println!("Description: {}", description);
    }
    println!("System: {:?}", current_theme.system);
    if let Some(variant) = &current_theme.variant {
        println!("Variant: {:?}", variant);
    }

    // Show some key colors
    println!("\nKey colors:");
    if let Some(bg) = &current_theme.palette.base00 {
        println!("  Background: {}", bg.dimmed());
    }
    if let Some(fg) = &current_theme.palette.base05 {
        println!("  Foreground: {}", fg.dimmed());
    }
    if let Some(primary) = &current_theme.palette.base0d {
        println!("  Primary: {}", primary.dimmed());
    }

    Ok(())
}

/// Apply a theme
pub async fn theme_apply(theme_name: &str, json: bool) -> Result<()> {
    let mut config = Config::load()?;
    let theme = config
        .load_theme_by_name(theme_name)
        .with_context(|| format!("Failed to load theme '{}'", theme_name))?;

    config.set_theme(theme.clone());
    config.save()?;

    // Notify GUI applications about theme change
    #[cfg(feature = "gui")]
    {
        let _ = notify_theme_changed(theme_name).await;
    }

    if json {
        println!(
            "{}",
            serde_json::json!({
                "status": "success",
                "theme": theme_name,
                "message": format!("Applied theme '{}'", theme_name)
            })
        );
    } else {
        println!("Applied theme: {}", theme_name.cyan());
        if let Some(name) = &theme.name {
            println!("  {}", name.dimmed());
        }
    }

    Ok(())
}

/// Show detailed information about a theme
pub fn theme_info(theme_name: &str, json: bool) -> Result<()> {
    let config = Config::load()?;
    let loader = config.get_theme_loader();
    let theme = loader
        .load_theme(theme_name)
        .with_context(|| format!("Failed to load theme '{}'", theme_name))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&theme)?);
        return Ok(());
    }

    println!("Theme: {}", theme.scheme.cyan());
    if let Some(name) = &theme.name {
        println!("Name: {}", name);
    }
    if let Some(author) = &theme.author {
        println!("Author: {}", author.dimmed());
    }
    if let Some(description) = &theme.description {
        println!("Description: {}", description);
    }
    println!("System: {:?}", theme.system);
    if let Some(variant) = &theme.variant {
        println!("Variant: {:?}", variant);
    }

    if let Some(inherits) = &theme.inherits {
        println!("Inherits from: {}", inherits.dimmed());
    }

    println!("\nColor palette:");
    let palette_colors = [
        ("base00", &theme.palette.base00, "Default Background"),
        ("base01", &theme.palette.base01, "Lighter Background"),
        ("base02", &theme.palette.base02, "Selection Background"),
        ("base03", &theme.palette.base03, "Comments"),
        ("base04", &theme.palette.base04, "Dark Foreground"),
        ("base05", &theme.palette.base05, "Default Foreground"),
        ("base06", &theme.palette.base06, "Light Foreground"),
        ("base07", &theme.palette.base07, "Light Background"),
        ("base08", &theme.palette.base08, "Red"),
        ("base09", &theme.palette.base09, "Orange"),
        ("base0A", &theme.palette.base0a, "Yellow"),
        ("base0B", &theme.palette.base0b, "Green"),
        ("base0C", &theme.palette.base0c, "Cyan"),
        ("base0D", &theme.palette.base0d, "Blue"),
        ("base0E", &theme.palette.base0e, "Purple"),
        ("base0F", &theme.palette.base0f, "Brown"),
    ];

    for (name, color, description) in palette_colors {
        if let Some(color_value) = color {
            println!(
                "  {}: {} ({})",
                name.cyan(),
                color_value,
                description.dimmed()
            );
        }
    }

    println!("\nSemantic mappings:");
    println!("  background: {}", theme.semantic.background.cyan());
    println!("  foreground: {}", theme.semantic.foreground.cyan());
    println!("  primary: {}", theme.semantic.primary.cyan());
    println!("  accent: {}", theme.semantic.accent.cyan());
    println!("  success: {}", theme.semantic.success.cyan());
    println!("  warning: {}", theme.semantic.warning.cyan());
    println!("  error: {}", theme.semantic.error.cyan());

    Ok(())
}

/// Validate a theme file
pub fn theme_validate(file_path: &str, json: bool) -> Result<()> {
    let config = Config::load()?;
    let loader = config.get_theme_loader();
    let path = Path::new(file_path);

    if !path.exists() {
        bail!("Theme file not found: {}", file_path);
    }

    match loader.load_theme_from_file(path) {
        Ok(theme) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "valid",
                        "theme": theme.scheme,
                        "message": "Theme file is valid"
                    })
                );
            } else {
                println!("✓ Theme file is valid: {}", theme.scheme.cyan());
                if let Some(name) = &theme.name {
                    println!("  Name: {}", name);
                }
                println!("  System: {:?}", theme.system);
            }
        }
        Err(e) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "invalid",
                        "error": e.to_string()
                    })
                );
            } else {
                println!("✗ Theme file is invalid: {}", e.to_string().red());
            }
            return Err(e);
        }
    }

    Ok(())
}

/// Generate CSS from current theme (debug command)
pub fn theme_generate_css(json: bool) -> Result<()> {
    let config = get_config();
    let theme = config.get_theme()?;

    if json {
        println!(
            "{}",
            serde_json::json!({
                "css": theme.generate_css_theme()
            })
        );
    } else {
        println!("{}", theme.generate_css_theme());
    }

    Ok(())
}

// ============================================================================
// User Management Commands (requires lst-server binary)
// ============================================================================

/// Check if lst-server binary is available
fn check_server_binary() -> Result<()> {
    match std::process::Command::new("lst-server")
        .arg("--help")
        .output()
    {
        Ok(output) if output.status.success() => Ok(()),
        Ok(_) => bail!("lst-server binary found but returned error. Please ensure lst-server is properly installed."),
        Err(_) => bail!("lst-server binary not found. Please install lst-server to use user management commands."),
    }
}

/// List all users
pub async fn user_list(json: bool) -> Result<()> {
    check_server_binary()?;

    let mut cmd = std::process::Command::new("lst-server");
    cmd.arg("user").arg("list");
    if json {
        cmd.arg("--json");
    }

    let output = cmd
        .output()
        .context("Failed to execute lst-server user list")?;

    if output.status.success() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        bail!("lst-server user list failed: {}", error);
    }

    Ok(())
}

/// Create a new user
pub async fn user_create(email: &str, name: Option<&str>, json: bool) -> Result<()> {
    check_server_binary()?;

    let mut cmd = std::process::Command::new("lst-server");
    cmd.arg("user").arg("create").arg(email);
    if let Some(name) = name {
        cmd.arg("--name").arg(name);
    }
    if json {
        cmd.arg("--json");
    }

    let output = cmd
        .output()
        .context("Failed to execute lst-server user create")?;

    if output.status.success() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        bail!("lst-server user create failed: {}", error);
    }

    Ok(())
}

/// Delete a user
pub async fn user_delete(email: &str, force: bool, json: bool) -> Result<()> {
    check_server_binary()?;

    let mut cmd = std::process::Command::new("lst-server");
    cmd.arg("user").arg("delete").arg(email);
    if force {
        cmd.arg("--force");
    }
    if json {
        cmd.arg("--json");
    }

    let output = cmd
        .output()
        .context("Failed to execute lst-server user delete")?;

    if output.status.success() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        bail!("lst-server user delete failed: {}", error);
    }

    Ok(())
}

/// Update user information
pub async fn user_update(
    email: &str,
    name: Option<&str>,
    enabled: Option<bool>,
    json: bool,
) -> Result<()> {
    check_server_binary()?;

    let mut cmd = std::process::Command::new("lst-server");
    cmd.arg("user").arg("update").arg(email);
    if let Some(name) = name {
        cmd.arg("--name").arg(name);
    }
    if let Some(enabled) = enabled {
        cmd.arg("--enabled").arg(enabled.to_string());
    }
    if json {
        cmd.arg("--json");
    }

    let output = cmd
        .output()
        .context("Failed to execute lst-server user update")?;

    if output.status.success() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        bail!("lst-server user update failed: {}", error);
    }

    Ok(())
}

/// Show detailed information about a user
pub async fn user_info(email: &str, json: bool) -> Result<()> {
    check_server_binary()?;

    let mut cmd = std::process::Command::new("lst-server");
    cmd.arg("user").arg("info").arg(email);
    if json {
        cmd.arg("--json");
    }

    let output = cmd
        .output()
        .context("Failed to execute lst-server user info")?;

    if output.status.success() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        bail!("lst-server user info failed: {}", error);
    }

    Ok(())
}
