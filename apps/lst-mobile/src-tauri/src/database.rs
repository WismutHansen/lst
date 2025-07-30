use anyhow::{anyhow, Result};
use chrono::Utc;
use lst_cli::models::{fuzzy_find, is_valid_anchor, ItemStatus, List, ListItem, Category};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use tauri::Manager;
use crate::Note;

pub struct Database {
    pool: Pool<SqliteConnectionManager>,
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

    pub fn create_list(&self, title: &str) -> Result<List> {
        let list = List::new(title.to_string());
        let json = serde_json::to_string(&list)?;
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO lists (title, data) VALUES (?1, ?2)",
            params![title, json],
        )?;
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

    pub fn save_list(&self, list: &List) -> Result<()> {
        let conn = self.conn()?;
        let json = serde_json::to_string(list)?;
        conn.execute(
            "INSERT INTO lists (title, data) VALUES (?1, ?2) \
            ON CONFLICT(title) DO UPDATE SET data=excluded.data",
            params![list.metadata.title, json],
        )?;
        Ok(())
    }

    pub fn add_item(&self, list: &str, text: &str) -> Result<List> {
        let mut l = self.load_list(list)?;
        for item in text.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            // Check for ##category inline syntax
            let (parsed_category, parsed_text) = parse_item_input(item);
            l.add_item_to_category(parsed_text.to_string(), parsed_category);
        }
        self.save_list(&l)?;
        Ok(l)
    }

    pub fn add_item_to_category(&self, list: &str, text: &str, category: Option<&str>) -> Result<List> {
        let mut l = self.load_list(list)?;
        for item in text.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            // Check for ##category inline syntax
            let (parsed_category, parsed_text) = parse_item_input(item);
            let final_category = parsed_category.or(category);
            l.add_item_to_category(parsed_text.to_string(), final_category);
        }
        self.save_list(&l)?;
        Ok(l)
    }

    pub fn toggle_item(&self, list: &str, target: &str) -> Result<List> {
        let mut l = self.load_list(list)?;
        if let Some(item) = l.find_item_mut_by_anchor(target) {
            item.status = match item.status {
                ItemStatus::Todo => ItemStatus::Done,
                ItemStatus::Done => ItemStatus::Todo,
            };
            l.metadata.updated = Utc::now();
            self.save_list(&l)?;
            Ok(l)
        } else {
            Err(anyhow!("No item matching '{}'", target))
        }
    }

    pub fn edit_item(&self, list: &str, target: &str, text: &str) -> Result<List> {
        if text.trim().is_empty() {
            return Err(anyhow!("New text cannot be empty"));
        }
        let mut l = self.load_list(list)?;
        if let Some(item) = l.find_item_mut_by_anchor(target) {
            item.text = text.to_string();
            l.metadata.updated = Utc::now();
            self.save_list(&l)?;
            Ok(l)
        } else {
            Err(anyhow!("No item matching '{}'", target))
        }
    }

    pub fn remove_item(&self, list: &str, target: &str) -> Result<List> {
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
            self.save_list(&l)?;
            Ok(l)
        } else {
            Err(anyhow!("No item matching '{}'", target))
        }
    }

    pub fn reorder_item(&self, list: &str, target: &str, new_index: usize) -> Result<List> {
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
            self.save_list(&l)?;
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

    pub fn create_note(&self, title: &str) -> Result<Note> {
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

    pub fn save_note(&self, note: &Note) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO notes (title, content, created, file_path) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(title) DO UPDATE SET 
                content=excluded.content, 
                created=excluded.created,
                file_path=excluded.file_path",
            params![note.title, note.content, note.created, note.file_path],
        )?;
        Ok(())
    }

    pub fn delete_note(&self, title: &str) -> Result<()> {
        let conn = self.conn()?;
        let changes = conn.execute("DELETE FROM notes WHERE title=?1", [title])?;
        if changes == 0 {
            return Err(anyhow!("Note '{}' not found", title));
        }
        Ok(())
    }

    // Category management methods
    pub fn create_category(&self, list_name: &str, category_name: &str) -> Result<List> {
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
        self.save_list(&list)?;
        Ok(list)
    }

    pub fn move_item_to_category(&self, list_name: &str, item_anchor: &str, category_name: Option<&str>) -> Result<List> {
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
        self.save_list(&list)?;
        Ok(list)
    }

    pub fn delete_category(&self, list_name: &str, category_name: &str) -> Result<List> {
        let mut list = self.load_list(list_name)?;
        
        // Find category and move its items to uncategorized
        if let Some(pos) = list.categories.iter().position(|c| c.name == category_name) {
            let category = list.categories.remove(pos);
            list.uncategorized_items.extend(category.items);
            
            list.metadata.updated = Utc::now();
            self.save_list(&list)?;
            Ok(list)
        } else {
            Err(anyhow!("Category '{}' not found", category_name))
        }
    }

    pub fn get_categories(&self, list_name: &str) -> Result<Vec<String>> {
        let list = self.load_list(list_name)?;
        Ok(list.categories.iter().map(|c| c.name.clone()).collect())
    }

    pub fn rename_category(&self, list_name: &str, old_name: &str, new_name: &str) -> Result<List> {
        let mut list = self.load_list(list_name)?;
        
        // Check if new name already exists
        if list.categories.iter().any(|c| c.name == new_name) {
            return Err(anyhow!("Category '{}' already exists", new_name));
        }
        
        // Find and rename category
        if let Some(category) = list.categories.iter_mut().find(|c| c.name == old_name) {
            category.name = new_name.to_string();
            list.metadata.updated = Utc::now();
            self.save_list(&list)?;
            Ok(list)
        } else {
            Err(anyhow!("Category '{}' not found", old_name))
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
    let matches = fuzzy_find(&all_items, target, 0.75);
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
