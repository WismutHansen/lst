use anyhow::{anyhow, Result};
use chrono::Utc;
use lst_cli::models::{fuzzy_find, is_valid_anchor, ItemStatus, List, ListItem, Category};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use tauri::Manager;
use crate::{Note, sync_bridge::{SyncBridge, ListOperation, NoteOperation}};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Database {
    pub pool: Pool<SqliteConnectionManager>,
}

impl Database {
    pub fn new(app: &tauri::AppHandle) -> Result<Self> {
        let resolver = app.path();
        let mut path = resolver.app_data_dir().or_else(|_| {
            // Fallback for iOS - use app local data directory
            resolver.app_local_data_dir()
        })?;
        
        // Ensure directory exists with proper error handling for iOS
        if let Err(e) = std::fs::create_dir_all(&path) {
            eprintln!("Failed to create app data directory: {}", e);
            // Try alternative path for iOS
            path = resolver.app_local_data_dir()?;
            std::fs::create_dir_all(&path)?;
        }
        
        path.push("lst_mobile.db");
        println!("Database path: {:?}", path);

        let manager = SqliteConnectionManager::file(&path);
        let pool = Pool::new(manager)?;

        let db = Self { pool };
        db.init()?;
        Ok(db)
    }

    fn conn(&self) -> Result<PooledConnection<SqliteConnectionManager>> {
        self.pool.get().map_err(|e| e.into())
    }

    fn init(&self) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS lists (title TEXT PRIMARY KEY, data TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS notes (
                 title TEXT PRIMARY KEY,
                 content TEXT NOT NULL,
                 created TEXT,
                 file_path TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS sync_config (
                 key TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );",
        )?;
        Ok(())
    }

    pub fn list_titles(&self) -> Result<Vec<String>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT title FROM lists ORDER BY title")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub async fn create_list(&self, title: &str, app: Option<&tauri::AppHandle>) -> Result<List> {
        let list = List::new(title.to_string());
        let json = serde_json::to_string(&list)?;
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO lists (title, data) VALUES (?1, ?2)",
            params![title, json],
        )?;

        // Trigger sync if app handle is provided
        if let Some(app_handle) = app {
            println!("📱 Triggering sync for list creation: {}", title);
            match self.trigger_list_sync(app_handle, ListOperation::Create { 
                title: title.to_string(), 
                list: &list 
            }).await {
                Ok(()) => println!("📱 ✅ Successfully triggered sync for new list: {}", title),
                Err(e) => {
                    eprintln!("📱 ❌ Failed to trigger list sync for new list '{}': {}", title, e);
                    // Don't fail the entire operation if sync fails - data is still saved locally
                }
            }
        }

        Ok(list)
    }

    pub fn load_list(&self, title: &str) -> Result<List> {
        let conn = self.conn()?;
        let json: String = conn
            .query_row("SELECT data FROM lists WHERE title=?1", [title], |row| {
                row.get(0)
            })
            .optional()? // optional to handle not found
            .ok_or_else(|| anyhow!("List '{}' not found", title))?;
        Ok(serde_json::from_str(&json)?)
    }

    pub async fn save_list(&self, list: &List, app: Option<&tauri::AppHandle>) -> Result<()> {
        let conn = self.conn()?;
        let json = serde_json::to_string(list)?;
        conn.execute(
            "INSERT INTO lists (title, data) VALUES (?1, ?2) \
            ON CONFLICT(title) DO UPDATE SET data=excluded.data",
            params![list.metadata.title, json],
        )?;

        // Trigger sync if app handle is provided
        if let Some(app_handle) = app {
            println!("📱 Triggering sync for list update: {}", list.metadata.title);
            match self.trigger_list_sync(app_handle, ListOperation::Update { 
                title: list.metadata.title.clone(), 
                list 
            }).await {
                Ok(()) => println!("📱 ✅ Successfully triggered sync for list: {}", list.metadata.title),
                Err(e) => {
                    eprintln!("📱 ❌ Failed to trigger list sync for '{}': {}", list.metadata.title, e);
                    // Don't fail the entire operation if sync fails - data is still saved locally
                }
            }
        }
        
        Ok(())
    }

    pub async fn add_item(&self, list: &str, text: &str, app: Option<&tauri::AppHandle>) -> Result<List> {
        let mut l = self.load_list(list)?;
        for item in text.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            // Check for ##category inline syntax
            let (parsed_category, parsed_text) = parse_item_input(item);
            l.add_item_to_category(parsed_text.to_string(), parsed_category);
        }
        self.save_list(&l, app).await?;
        Ok(l)
    }

    pub async fn add_item_to_category(&self, list: &str, text: &str, category: Option<&str>, app: Option<&tauri::AppHandle>) -> Result<List> {
        let mut l = self.load_list(list)?;
        for item in text.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            // Check for ##category inline syntax
            let (parsed_category, parsed_text) = parse_item_input(item);
            let final_category = parsed_category.or(category);
            l.add_item_to_category(parsed_text.to_string(), final_category);
        }
        self.save_list(&l, app).await?;
        Ok(l)
    }

    pub async fn toggle_item(&self, list: &str, target: &str, app: Option<&tauri::AppHandle>) -> Result<List> {
        let mut l = self.load_list(list)?;
        if let Some(item) = l.find_item_mut_by_anchor(target) {
            item.status = match item.status {
                ItemStatus::Todo => ItemStatus::Done,
                ItemStatus::Done => ItemStatus::Todo,
            };
            l.metadata.updated = Utc::now();
            self.save_list(&l, app).await?;
            Ok(l)
        } else {
            Err(anyhow!("No item matching '{}'", target))
        }
    }

    pub async fn edit_item(&self, list: &str, target: &str, text: &str, app: Option<&tauri::AppHandle>) -> Result<List> {
        if text.trim().is_empty() {
            return Err(anyhow!("New text cannot be empty"));
        }
        let mut l = self.load_list(list)?;
        if let Some(item) = l.find_item_mut_by_anchor(target) {
            item.text = text.to_string();
            l.metadata.updated = Utc::now();
            self.save_list(&l, app).await?;
            Ok(l)
        } else {
            Err(anyhow!("No item matching '{}'", target))
        }
    }

    pub async fn remove_item(&self, list: &str, target: &str, app: Option<&tauri::AppHandle>) -> Result<List> {
        let mut l = self.load_list(list)?;
        
        // Find and remove item from its location
        let mut found = false;
        
        // Check uncategorized items
        if let Some(pos) = l.uncategorized_items.iter().position(|item| 
            item.anchor == target || item.text.to_lowercase() == target.to_lowercase()) {
            l.uncategorized_items.remove(pos);
            found = true;
        } else {
            // Check categorized items
            for category in &mut l.categories {
                if let Some(pos) = category.items.iter().position(|item| 
                    item.anchor == target || item.text.to_lowercase() == target.to_lowercase()) {
                    category.items.remove(pos);
                    found = true;
                    break;
                }
            }
        }
        
        if found {
            l.metadata.updated = Utc::now();
            self.save_list(&l, app).await?;
            Ok(l)
        } else {
            Err(anyhow!("No item matching '{}'", target))
        }
    }

    pub async fn reorder_item(&self, list: &str, target: &str, new_index: usize, app: Option<&tauri::AppHandle>) -> Result<List> {
        let mut l = self.load_list(list)?;
        
        // Find and remove item from its current location
        let mut item_to_move = None;
        
        // Check uncategorized items
        if let Some(pos) = l.uncategorized_items.iter().position(|item| 
            item.anchor == target || item.text.to_lowercase() == target.to_lowercase()) {
            item_to_move = Some(l.uncategorized_items.remove(pos));
        } else {
            // Check categorized items
            for category in &mut l.categories {
                if let Some(pos) = category.items.iter().position(|item| 
                    item.anchor == target || item.text.to_lowercase() == target.to_lowercase()) {
                    item_to_move = Some(category.items.remove(pos));
                    break;
                }
            }
        }
        
        if let Some(item) = item_to_move {
            // For now, reordering puts items in uncategorized section
            let clamped = new_index.min(l.uncategorized_items.len());
            l.uncategorized_items.insert(clamped, item);
            l.metadata.updated = Utc::now();
            self.save_list(&l, app).await?;
            Ok(l)
        } else {
            Err(anyhow!("No item matching '{}'", target))
        }
    }

    // Note-related methods
    pub fn list_note_titles(&self) -> Result<Vec<String>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT title FROM notes ORDER BY title")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub async fn create_note(&self, title: &str, app: Option<&tauri::AppHandle>) -> Result<Note> {
        let created = Utc::now().to_rfc3339();
        let file_path = format!("notes/{}.md", title.replace(' ', "_").to_lowercase());
        let note = Note {
            title: title.to_string(),
            content: String::new(),
            created: Some(created.clone()),
            file_path,
        };
        
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO notes (title, content, created, file_path) VALUES (?1, ?2, ?3, ?4)",
            params![note.title, note.content, created, note.file_path],
        )?;

        // Trigger sync if app handle is provided
        if let Some(app_handle) = app {
            println!("Triggering sync for note creation: {}", title);
            if let Err(e) = self.trigger_note_sync(app_handle, NoteOperation::Create { 
                title: title.to_string(), 
                note: &note 
            }).await {
                eprintln!("Failed to trigger note sync for new note '{}': {}", title, e);
            } else {
                println!("Successfully triggered sync for new note: {}", title);
            }
        }

        Ok(note)
    }

    pub fn load_note(&self, title: &str) -> Result<Note> {
        let conn = self.conn()?;
        let result = conn.query_row(
            "SELECT title, content, created, file_path FROM notes WHERE title=?1",
            [title],
            |row| {
                Ok(Note {
                    title: row.get(0)?,
                    content: row.get(1)?,
                    created: row.get(2).ok(),
                    file_path: row.get(3)?,
                })
            },
        ).optional()?;
        
        result.ok_or_else(|| anyhow!("Note '{}' not found", title))
    }

    pub async fn save_note(&self, note: &Note, app: Option<&tauri::AppHandle>) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO notes (title, content, created, file_path) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(title) DO UPDATE SET 
                content=excluded.content, 
                created=excluded.created,
                file_path=excluded.file_path",
            params![note.title, note.content, note.created, note.file_path],
        )?;

        // Trigger sync if app handle is provided
        if let Some(app_handle) = app {
            println!("Triggering sync for note update: {}", note.title);
            if let Err(e) = self.trigger_note_sync(app_handle, NoteOperation::Update { 
                title: note.title.clone(), 
                note 
            }).await {
                eprintln!("Failed to trigger note sync for updated note '{}': {}", note.title, e);
            } else {
                println!("Successfully triggered sync for note update: {}", note.title);
            }
        }

        Ok(())
    }

    pub async fn delete_note(&self, title: &str, app: Option<&tauri::AppHandle>) -> Result<()> {
        let conn = self.conn()?;
        let changes = conn.execute("DELETE FROM notes WHERE title=?1", [title])?;
        if changes == 0 {
            return Err(anyhow!("Note '{}' not found", title));
        }

        // Trigger sync if app handle is provided
        if let Some(app_handle) = app {
            if let Err(e) = self.trigger_note_sync(app_handle, NoteOperation::Delete { 
                title: title.to_string()
            }).await {
                eprintln!("Failed to trigger note sync: {}", e);
            }
        }

        Ok(())
    }

    // Category management methods
    pub async fn create_category(&self, list_name: &str, category_name: &str, app: Option<&tauri::AppHandle>) -> Result<List> {
        let mut list = self.load_list(list_name)?;
        
        // Check if category already exists
        if list.categories.iter().any(|c| c.name == category_name) {
            return Err(anyhow!("Category '{}' already exists", category_name));
        }
        
        // Add new empty category
        list.categories.push(Category {
            name: category_name.to_string(),
            items: Vec::new(),
        });
        
        list.metadata.updated = Utc::now();
        self.save_list(&list, app).await?;
        Ok(list)
    }

    pub async fn move_item_to_category(&self, list_name: &str, item_anchor: &str, category_name: Option<&str>, app: Option<&tauri::AppHandle>) -> Result<List> {
        let mut list = self.load_list(list_name)?;
        
        // Find and remove the item from its current location
        let mut item_to_move = None;
        
        // Check uncategorized items
        if let Some(pos) = list.uncategorized_items.iter().position(|item| item.anchor == item_anchor) {
            item_to_move = Some(list.uncategorized_items.remove(pos));
        } else {
            // Check categorized items
            for category in &mut list.categories {
                if let Some(pos) = category.items.iter().position(|item| item.anchor == item_anchor) {
                    item_to_move = Some(category.items.remove(pos));
                    break;
                }
            }
        }
        
        let item = item_to_move.ok_or_else(|| anyhow!("Item with anchor '{}' not found", item_anchor))?;
        
        // Add item to new location
        match category_name {
            Some(cat_name) => {
                // Find or create category
                if let Some(category) = list.categories.iter_mut().find(|c| c.name == cat_name) {
                    category.items.push(item);
                } else {
                    // Create new category
                    list.categories.push(Category {
                        name: cat_name.to_string(),
                        items: vec![item],
                    });
                }
            }
            None => {
                // Move to uncategorized
                list.uncategorized_items.push(item);
            }
        }
        
        list.metadata.updated = Utc::now();
        self.save_list(&list, app).await?;
        Ok(list)
    }

    pub async fn delete_category(&self, list_name: &str, category_name: &str, app: Option<&tauri::AppHandle>) -> Result<List> {
        let mut list = self.load_list(list_name)?;
        
        // Find category and move its items to uncategorized
        if let Some(pos) = list.categories.iter().position(|c| c.name == category_name) {
            let category = list.categories.remove(pos);
            list.uncategorized_items.extend(category.items);
            
            list.metadata.updated = Utc::now();
            self.save_list(&list, app).await?;
            Ok(list)
        } else {
            Err(anyhow!("Category '{}' not found", category_name))
        }
    }

    pub fn get_categories(&self, list_name: &str) -> Result<Vec<String>> {
        let list = self.load_list(list_name)?;
        Ok(list.categories.iter().map(|c| c.name.clone()).collect())
    }

    pub async fn rename_category(&self, list_name: &str, old_name: &str, new_name: &str, app: Option<&tauri::AppHandle>) -> Result<List> {
        let mut list = self.load_list(list_name)?;
        
        // Check if new name already exists
        if list.categories.iter().any(|c| c.name == new_name) {
            return Err(anyhow!("Category '{}' already exists", new_name));
        }
        
        // Find and rename category
        if let Some(category) = list.categories.iter_mut().find(|c| c.name == old_name) {
            category.name = new_name.to_string();
            list.metadata.updated = Utc::now();
            self.save_list(&list, app).await?;
            Ok(list)
        } else {
            Err(anyhow!("Category '{}' not found", old_name))
        }
    }

    /// Save a sync configuration key-value pair
    pub fn save_sync_config(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO sync_config (key, value) VALUES (?1, ?2) 
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// Load a sync configuration value by key
    pub fn load_sync_config(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn()?;
        let result = conn
            .query_row("SELECT value FROM sync_config WHERE key = ?1", params![key], |row| {
                row.get::<_, String>(0)
            })
            .optional()?;
        Ok(result)
    }

    /// Load all sync configuration as a HashMap
    pub fn load_all_sync_config(&self) -> Result<std::collections::HashMap<String, String>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT key, value FROM sync_config")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        
        let mut config = std::collections::HashMap::new();
        for row in rows {
            let (key, value) = row?;
            config.insert(key, value);
        }
        Ok(config)
    }

    /// Helper method to trigger list sync operation
    async fn trigger_list_sync(&self, app: &tauri::AppHandle, operation: ListOperation<'_>) -> Result<()> {
        let bridge_state: tauri::State<Arc<Mutex<Option<SyncBridge>>>> = app.state();
        
        // Try to ensure sync bridge is initialized
        if let Err(e) = self.ensure_sync_bridge_initialized(app, &bridge_state).await {
            println!("Warning: Could not initialize sync bridge, sync disabled: {}", e);
            return Err(anyhow!("Failed to initialize sync bridge: {}", e));
        }
        
        let mut bridge_guard = bridge_state.lock().await;
        if let Some(ref mut bridge) = *bridge_guard {
            println!("Executing list sync operation...");
            match bridge.sync_list_operation(operation).await {
                Ok(()) => {
                    println!("List sync operation completed successfully");
                    Ok(())
                }
                Err(e) => {
                    eprintln!("List sync operation failed: {}", e);
                    Err(e)
                }
            }
        } else {
            println!("Warning: Sync bridge not available, skipping sync");
            Ok(()) // Don't treat this as an error, just skip sync
        }
    }

    /// Helper method to trigger note sync operation  
    async fn trigger_note_sync(&self, app: &tauri::AppHandle, operation: NoteOperation<'_>) -> Result<()> {
        let bridge_state: tauri::State<Arc<Mutex<Option<SyncBridge>>>> = app.state();
        
        // Try to ensure sync bridge is initialized
        if let Err(e) = self.ensure_sync_bridge_initialized(app, &bridge_state).await {
            println!("Warning: Could not initialize sync bridge, sync disabled: {}", e);
            return Err(anyhow!("Failed to initialize sync bridge: {}", e));
        }
        
        let mut bridge_guard = bridge_state.lock().await;
        if let Some(ref mut bridge) = *bridge_guard {
            println!("Executing note sync operation...");
            match bridge.sync_note_operation(operation).await {
                Ok(()) => {
                    println!("Note sync operation completed successfully");
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Note sync operation failed: {}", e);
                    Err(e)
                }
            }
        } else {
            println!("Warning: Sync bridge not available, skipping sync");
            Ok(()) // Don't treat this as an error, just skip sync
        }
    }

    /// Initialize sync bridge with current config
    pub async fn ensure_sync_bridge_initialized(
        &self,
        _app: &tauri::AppHandle, 
        bridge_state: &Arc<Mutex<Option<SyncBridge>>>
    ) -> Result<()> {
        let mut bridge_guard = bridge_state.lock().await;
        
        if bridge_guard.is_none() {
            println!("Initializing sync bridge...");
            let config = crate::mobile_config::get_current_config();
            
            // Check if sync is properly configured
            if !config.is_jwt_valid() {
                return Err(anyhow!("Sync not configured - JWT token invalid or missing"));
            }
            
            match SyncBridge::new().await {
                Ok(bridge) => {
                    println!("Sync bridge initialized successfully");
                    *bridge_guard = Some(bridge);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Failed to create sync bridge: {}", e);
                    Err(anyhow!("Failed to initialize sync bridge: {}", e))
                }
            }
        } else {
            println!("Sync bridge already initialized");
            Ok(())
        }
    }
}

fn find_item_index(list: &List, target: &str) -> Option<usize> {
    if is_valid_anchor(target) {
        if let Some(idx) = list.find_by_anchor(target) {
            return Some(idx);
        }
    }
    if let Some(idx) = list.find_by_text(target) {
        return Some(idx);
    }
    if let Some(number_str) = target.strip_prefix('#') {
        if let Ok(num) = number_str.parse::<usize>() {
            if let Some(item) = list.get_by_index(num - 1) {
                if let Some(idx) = list.find_by_anchor(&item.anchor) {
                    return Some(idx);
                }
            }
        }
    }
    let all_items: Vec<ListItem> = list.all_items().cloned().collect();
    let matches = fuzzy_find(&all_items, target, 75);
    match matches.len() {
        1 => Some(matches[0]),
        _ => None,
    }
}

fn parse_item_input(input: &str) -> (Option<&str>, &str) {
    if input.starts_with("##") {
        if let Some(space_index) = input.find(' ') {
            if space_index > 2 {
                let category = &input[2..space_index];
                let text = &input[space_index + 1..];
                return (Some(category), text);
            }
        }
    }
    (None, input)
}
