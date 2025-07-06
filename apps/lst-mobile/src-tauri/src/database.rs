use anyhow::{anyhow, Result};
use chrono::Utc;
use lst_cli::models::{fuzzy_find, is_valid_anchor, ItemStatus, List};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use tauri::Manager;

pub struct Database {
    pool: Pool<SqliteConnectionManager>,
}

impl Database {
    pub fn new(app: &tauri::AppHandle) -> Result<Self> {
        let resolver = app.path();
        let mut path = resolver.app_data_dir()?;
        std::fs::create_dir_all(&path)?;
        path.push("lst_mobile.db");

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
            "CREATE TABLE IF NOT EXISTS lists (title TEXT PRIMARY KEY, data TEXT NOT NULL);",
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
            l.add_item(item.to_string());
        }
        self.save_list(&l)?;
        Ok(l)
    }

    pub fn toggle_item(&self, list: &str, target: &str) -> Result<List> {
        let mut l = self.load_list(list)?;
        if let Some(idx) = find_item_index(&l, target) {
            l.items[idx].status = match l.items[idx].status {
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
        if let Some(idx) = find_item_index(&l, target) {
            l.items[idx].text = text.to_string();
            l.metadata.updated = Utc::now();
            self.save_list(&l)?;
            Ok(l)
        } else {
            Err(anyhow!("No item matching '{}'", target))
        }
    }

    pub fn remove_item(&self, list: &str, target: &str) -> Result<List> {
        let mut l = self.load_list(list)?;
        if let Some(idx) = find_item_index(&l, target) {
            l.items.remove(idx);
            l.metadata.updated = Utc::now();
            self.save_list(&l)?;
            Ok(l)
        } else {
            Err(anyhow!("No item matching '{}'", target))
        }
    }

    pub fn reorder_item(&self, list: &str, target: &str, new_index: usize) -> Result<List> {
        let mut l = self.load_list(list)?;
        if let Some(idx) = find_item_index(&l, target) {
            let item = l.items.remove(idx);
            let clamped = new_index.min(l.items.len());
            l.items.insert(clamped, item);
            l.metadata.updated = Utc::now();
            self.save_list(&l)?;
            Ok(l)
        } else {
            Err(anyhow!("No item matching '{}'", target))
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
    let matches = fuzzy_find(&list.items, target, 0.75);
    match matches.len() {
        1 => Some(matches[0]),
        _ => None,
    }
}
