use anyhow::{bail, Context, Result};
use colored::{ColoredString, Colorize};
use serde_json;
use std::io::{self, BufRead};

use crate::cli::{DlCmd, SyncCommands};
use crate::storage;
use crate::{models::ItemStatus, storage::notes::delete_note};
use chrono::{Local, Utc};
use std::path::Path;
use std::process::{Command, Stdio};
use crate::config::{Config, get_config};

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
/// Handle daily list commands: create/display/add/done/undone for YYYYMMDD_daily_list
pub fn daily_list(cmd: Option<&DlCmd>, json: bool) -> Result<()> {
    let date = Local::now().format("%Y%m%d").to_string();
    let list_name = format!("daily_lists/{}_daily_list", date);
    // No subcommand: ensure exists then display
    match cmd {
        Some(DlCmd::Add { item }) => {
            add_item(&list_name, item, json)?;
        }
        Some(DlCmd::Done { item }) => {
            mark_done(&list_name, item, json)?;
        }
        Some(DlCmd::Undone { item }) => {
            mark_undone(&list_name, item, json)?;
        }
        Some(DlCmd::List) => {
            list_daily_lists(json)?;
        }
        Some(DlCmd::Remove { item }) => {
            remove_item(&list_name, item, json)?;
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
    let matches: Vec<&storage::FileEntry> = entries
        .iter()
        .filter(|entry| entry.name.contains(key))
        .collect();
    
    match matches.len() {
        0 => Ok(key.to_string()), // Allow new list creation
        1 => Ok(matches[0].relative_path.clone()),
        _ => {
            let match_names: Vec<String> = matches.iter().map(|e| e.relative_path.clone()).collect();
            bail!("Multiple lists match '{}': {:?}", key, match_names);
        }
    }
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
    let matches: Vec<&storage::FileEntry> = entries
        .iter()
        .filter(|entry| entry.name.contains(key))
        .collect();
    
    match matches.len() {
        0 => bail!("No note matching '{}' found", input),
        1 => Ok(matches[0].relative_path.clone()),
        _ => {
            let match_names: Vec<String> = matches.iter().map(|e| e.relative_path.clone()).collect();
            bail!("Multiple notes match '{}': {:?}", input, match_names);
        }
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
    let items = storage::markdown::mark_done(&list_name, target)?;

    if json {
        println!("{}", serde_json::to_string(&items)?);
        return Ok(());
    }

    if items.len() == 1 {
        println!("Marked done in {}: {}", list_name.cyan(), items[0].text);
    } else {
        println!("Marked {} items as done in {}:", items.len(), list_name.cyan());
        for item in &items {
            println!("  {}", item.text);
        }
    }

    Ok(())
}

/// Handle the 'undone' command to mark a completed item as not done
pub fn mark_undone(list: &str, target: &str, json: bool) -> Result<()> {
    let list_name = normalize_list(list)?;
    let items = storage::markdown::mark_undone(&list_name, target)?;

    if json {
        println!("{}", serde_json::to_string(&items)?);
        return Ok(());
    }

    if items.len() == 1 {
        println!("Marked undone in {}: {}", list_name.cyan(), items[0].text);
    } else {
        println!("Marked {} items as undone in {}:", items.len(), list_name.cyan());
        for item in &items {
            println!("  {}", item.text);
        }
    }

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
        
        let text = match item.status {
            ItemStatus::Todo => item.text.normal(),
            ItemStatus::Done => item.text.strikethrough(),
        };

        println!(
            "#{} {} {} {}",
            idx + 1,
            checkbox,
            text,
            item.anchor.dimmed()
        );
    }

    Ok(())
}

/// Handle sync daemon commands
pub fn handle_sync_command(cmd: SyncCommands, json: bool) -> Result<()> {
    match cmd {
        SyncCommands::Setup { server, token } => sync_setup(server, token, json),
        SyncCommands::Start { foreground } => sync_start(foreground, json),
        SyncCommands::Stop => sync_stop(json),
        SyncCommands::Status => sync_status(json),
        SyncCommands::Logs { follow, lines } => sync_logs(follow, lines, json),
    }
}

/// Setup sync configuration (first login flow)
pub fn sync_setup(server: Option<String>, token: Option<String>, json: bool) -> Result<()> {
    use dialoguer::{Input, Confirm};
    
    let mut config = Config::load()?;
    config.init_syncd()?;
    
    let server_url = if let Some(url) = server {
        url
    } else {
        Input::<String>::new()
            .with_prompt("Enter server URL (leave empty for local-only mode)")
            .allow_empty(true)
            .interact()?
    };
    
    let auth_token = if server_url.is_empty() {
        None
    } else if let Some(token) = token {
        Some(token)
    } else {
        let token: String = Input::new()
            .with_prompt("Enter authentication token")
            .interact()?;
        if token.is_empty() { None } else { Some(token) }
    };
    
    if let Some(ref mut syncd) = config.syncd {
        syncd.url = if server_url.is_empty() { None } else { Some(server_url.clone()) };
        syncd.auth_token = auth_token.clone();
    }
    
    config.save()?;
    
    if json {
        println!("{{\"status\": \"configured\", \"server\": {:?}, \"has_token\": {}}}", 
            server_url, auth_token.is_some());
    } else {
        if server_url.is_empty() {
            println!("Configured for local-only mode");
        } else {
            println!("Configured to sync with: {}", server_url.cyan());
            if auth_token.is_some() {
                println!("Authentication token set");
            }
        }
        
        if Confirm::new()
            .with_prompt("Start sync daemon now?")
            .default(true)
            .interact()? 
        {
            sync_start(false, json)?;
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
    let output = Command::new("pkill")
        .args(&["-f", "lst-syncd"])
        .output()?;
    
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
    let configured = config.syncd.is_some();
    let server_url = config.syncd.as_ref().and_then(|s| s.url.as_ref());
    let has_token = config.syncd.as_ref().and_then(|s| s.auth_token.as_ref()).is_some();
    
    // Check if daemon is running
    let running = Command::new("pgrep")
        .args(&["-f", "lst-syncd"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);
    
    if json {
        println!("{{\"configured\": {}, \"running\": {}, \"server\": {:?}, \"has_token\": {}}}", 
            configured, running, server_url, has_token);
    } else {
        println!("Sync Configuration:");
        println!("  Configured: {}", if configured { "Yes".green() } else { "No".red() });
        
        if let Some(url) = server_url {
            println!("  Server: {}", url.cyan());
        } else {
            println!("  Mode: {}", "Local-only".yellow());
        }
        
        println!("  Auth token: {}", if has_token { "Set".green() } else { "Not set".red() });
        println!("  Daemon: {}", if running { "Running".green() } else { "Stopped".red() });
        
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
        "lst-syncd", // In PATH
        "./target/debug/lst-syncd", // Local debug build
        "./target/release/lst-syncd", // Local release build
        &std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|parent| parent.join("lst-syncd")))
            .and_then(|p| p.to_str().map(String::from))
            .unwrap_or_default(),
    ];
    
    for path in possible_paths.iter() {
        if path.is_empty() { continue; }
        
        if Command::new(path).arg("--help").output().is_ok() {
            return Ok(path.to_string());
        }
    }
    
    bail!("lst-syncd binary not found. Make sure it's installed and in your PATH.");
}

/// List all daily lists
pub fn list_daily_lists(json: bool) -> Result<()> {
    let entries = storage::list_lists_with_info()?;
    
    // Filter for daily lists (in daily_lists directory)
    let daily_lists: Vec<&storage::FileEntry> = entries
        .iter()
        .filter(|entry| entry.relative_path.starts_with("daily_lists/") && entry.name.ends_with("_daily_list"))
        .collect();
    
    if json {
        let list_names: Vec<String> = daily_lists.iter().map(|e| e.relative_path.clone()).collect();
        println!("{}", serde_json::to_string(&list_names)?);
        return Ok(());
    }
    
    if daily_lists.is_empty() {
        println!("No daily lists found. Create one with 'lst dl add <text>'");
        return Ok(());
    }
    
    println!("Daily lists:");
    for entry in daily_lists {
        // Extract date from filename for display
        let date_part = entry.name.trim_end_matches("_daily_list");
        if let Ok(parsed_date) = chrono::NaiveDate::parse_from_str(date_part, "%Y%m%d") {
            let formatted_date = parsed_date.format("%Y-%m-%d (%A)").to_string();
            println!("  {} ({})", formatted_date, entry.relative_path);
        } else {
            println!("  {}", entry.relative_path);
        }
    }
    
    Ok(())
}
