use anyhow::{bail, Context, Result};
use colored::{ColoredString, Colorize};
use serde_json;
use serde_yaml;
use std::io::{self, BufRead};

use crate::cli::{DlCmd, SyncCommands};
use crate::config::{get_config, Config};
use crate::storage;
use crate::{models::ItemStatus, storage::notes::delete_note};
use chrono::{Local, Utc};
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

    println!("Available lists:");
    for list in lists {
        println!("  {}", list);
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
            add_item(&list_name, item, json).await?;
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
pub async fn note_delete(title: &str) -> Result<()> {
    // Determine the note file path
    // Resolve note to delete
    let key = title.trim_end_matches(".md");
    let note = resolve_note(key)?;
    let result = delete_note(&note);
    
    // Notify desktop app that a note was updated (deleted)
    #[cfg(feature = "gui")]
    {
        let _ = notify_note_updated(&note).await;
    }
    
    result
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
    let matches: Vec<&storage::FileEntry> = entries
        .iter()
        .filter(|entry| entry.name.contains(key))
        .collect();

    match matches.len() {
        0 => Ok(key.to_string()), // Allow new list creation
        1 => Ok(matches[0].relative_path.clone()),
        _ => {
            let match_names: Vec<String> =
                matches.iter().map(|e| e.relative_path.clone()).collect();
            bail!("Multiple lists match '{}': {:?}", key, match_names);
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
    let matches: Vec<&storage::FileEntry> = entries
        .iter()
        .filter(|entry| entry.name.contains(key))
        .collect();

    match matches.len() {
        0 => bail!("No note matching '{}' found", input),
        1 => Ok(matches[0].relative_path.clone()),
        _ => {
            let match_names: Vec<String> =
                matches.iter().map(|e| e.relative_path.clone()).collect();
            bail!("Multiple notes match '{}': {:?}", input, match_names);
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
    let matches: Vec<&storage::FileEntry> = entries
        .iter()
        .filter(|entry| entry.name.contains(key))
        .collect();

    match matches.len() {
        0 => bail!("No list matching '{}' found", input),
        1 => Ok(matches[0].relative_path.clone()),
        _ => {
            let match_names: Vec<String> =
                matches.iter().map(|e| e.relative_path.clone()).collect();
            bail!("Multiple lists match '{}': {:?}", input, match_names);
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
/// Handle the 'add' command to add an item to a list
pub async fn add_item(list: &str, text: &str, json: bool) -> Result<()> {
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
pub async fn mark_done(list: &str, target: &str, json: bool) -> Result<()> {
    let list_name = normalize_list(list)?;
    let items = storage::markdown::mark_done(&list_name, target)?;

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
    let items = storage::markdown::mark_undone(&list_name, target)?;

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
    use dialoguer::{Confirm, Input};

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
        if token.is_empty() {
            None
        } else {
            Some(token)
        }
    };

    if let Some(ref mut syncd) = config.syncd {
        syncd.url = if server_url.is_empty() {
            None
        } else {
            Some(server_url.clone())
        };
        syncd.auth_token = auth_token.clone();
    }

    config.save()?;

    if json {
        println!(
            "{{\"status\": \"configured\", \"server\": {:?}, \"has_token\": {}}}",
            server_url,
            auth_token.is_some()
        );
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
    let configured = config.syncd.is_some();
    let server_url = config.syncd.as_ref().and_then(|s| s.url.as_ref());
    let has_token = config
        .syncd
        .as_ref()
        .and_then(|s| s.auth_token.as_ref())
        .is_some();

    // Check if daemon is running
    let running = Command::new("pgrep")
        .args(&["-f", "lst-syncd"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if json {
        println!(
            "{{\"configured\": {}, \"running\": {}, \"server\": {:?}, \"has_token\": {}}}",
            configured, running, server_url, has_token
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

        println!(
            "  Auth token: {}",
            if has_token {
                "Set".green()
            } else {
                "Not set".red()
            }
        );
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
    display_list(&list_name, json)
}

/// Share a document by updating writers and readers in the local sync database
pub fn share_document(doc: &str, writers: Option<&str>, readers: Option<&str>) -> Result<()> {
    use rusqlite::Connection;
    use uuid::Uuid;

    let config = get_config();
    let db_path = config
        .syncd
        .as_ref()
        .and_then(|s| s.database_path.as_ref())
        .context("syncd.database_path not configured")?;

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
pub async fn auth_request(email: &str, host: Option<&str>, json: bool) -> Result<()> {
    let config = get_config();
    let server_url = config
        .server
        .url
        .as_ref()
        .context("No server URL configured. Run 'lst sync setup' first.")?;

    let host = if let Some(h) = host {
        h.to_string()
    } else {
        url::Url::parse(server_url)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "localhost".to_string())
    };

    use dialoguer::Password;
    use argon2::{Argon2, Algorithm, Params, PasswordHasher, Version};
    use argon2::password_hash::SaltString;

    let password = Password::new().with_prompt("Account password").interact()?;

    let params = Params::new(128 * 1024, 3, 2, None).expect("invalid params");
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let salt = SaltString::encode_b64(b"clientstatic").expect("salt");
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
        .post(format!("{}/api/auth/request", server_url))
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

/// Verify authentication token and store JWT
pub async fn auth_verify(email: &str, token: &str, json: bool) -> Result<()> {
    let mut config = Config::load()?;
    let server_url = config
        .server
        .url
        .as_ref()
        .context("No server URL configured. Run 'lst sync setup' first.")?;

    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "email": email,
        "token": token
    });

    let response = client
        .post(format!("{}/api/auth/verify", server_url))
        .json(&payload)
        .send()
        .await?;

    if response.status().is_success() {
        let verify_response: serde_json::Value = response.json().await?;

        if let Some(jwt) = verify_response.get("jwt").and_then(|j| j.as_str()) {
            // Parse JWT to get expiration (basic extraction without validation)
            let expires_at = chrono::Utc::now() + chrono::Duration::hours(1); // Default 1 hour

            config.store_jwt(jwt.to_string(), expires_at);
            config.save()?;

            if json {
                println!("{}", serde_json::to_string_pretty(&verify_response)?);
            } else {
                println!("Successfully authenticated as {}", email.green());
                println!("JWT token stored and ready for use");
            }
        } else {
            bail!("Invalid response: missing JWT token");
        }
    } else {
        let error_text = response.text().await?;
        bail!("Failed to verify token: {}", error_text);
    }

    Ok(())
}

/// Show current authentication status
pub fn auth_status(json: bool) -> Result<()> {
    let config = get_config();

    let has_server_url = config.server.url.is_some();
    let has_jwt = config.server.jwt_token.is_some();
    let jwt_valid = config.is_jwt_valid();

    if json {
        println!(
            "{}",
            serde_json::json!({
                "server_configured": has_server_url,
                "jwt_token_present": has_jwt,
                "jwt_valid": jwt_valid,
                "jwt_expires_at": config.server.jwt_expires_at
            })
        );
    } else {
        println!("Authentication Status:");

        if !has_server_url {
            println!("  Server: {}", "Not configured".red());
            println!("  Run 'lst sync setup' to configure server URL");
        } else {
            println!("  Server: {}", config.server.url.as_ref().unwrap().cyan());
        }

        if !has_jwt {
            println!("  JWT Token: {}", "Not present".red());
            println!("  Run 'lst auth request <email>' to authenticate");
        } else if jwt_valid {
            println!("  JWT Token: {}", "Valid".green());
            if let Some(expires_at) = config.server.jwt_expires_at {
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
    let mut config = Config::load()?;
    config.clear_jwt();
    config.save()?;

    if json {
        println!("{}", serde_json::json!({"status": "logged_out"}));
    } else {
        println!("Successfully logged out. JWT token removed.");
    }

    Ok(())
}

/// Helper function to make authenticated requests to the server
pub async fn make_authenticated_request(
    method: reqwest::Method,
    endpoint: &str,
    body: Option<serde_json::Value>,
) -> Result<reqwest::Response> {
    let config = get_config();

    let server_url = config
        .server
        .url
        .as_ref()
        .context("No server URL configured")?;

    let jwt = config
        .get_jwt()
        .context("No valid JWT token. Run 'lst auth request <email>' to authenticate")?;

    let client = reqwest::Client::new();
    let mut request = client
        .request(
            method,
            format!("{}/{}", server_url, endpoint.trim_start_matches('/')),
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
