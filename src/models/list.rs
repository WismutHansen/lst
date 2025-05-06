use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rand::distributions::{Alphanumeric, DistString};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

pub fn generate_anchor() -> String {
    // Use 5 random alphanumeric characters
    let anchor = format!(
        "^{}",
        Alphanumeric.sample_string(&mut rand::thread_rng(), 5)
    );
    anchor
}

/// Represents the metadata for a list
#[derive(Debug, Serialize, Deserialize)]
pub struct ListMetadata {
    /// Unique identifier for the list
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,

    /// Human-readable title of the list
    pub title: String,

    /// List of users who have access to the list
    #[serde(default)]
    pub sharing: Vec<String>,

    /// When the list was last updated
    #[serde(default = "Utc::now")]
    pub updated: DateTime<Utc>,
}

/// Represents the status of a list item (done or not)
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ItemStatus {
    Todo,
    Done,
}

/// Represents a single item in a list
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListItem {
    /// The text content of the item
    pub text: String,

    /// The status of the item (todo or done)
    pub status: ItemStatus,

    /// Unique anchor identifier for the item
    pub anchor: String,
}

/// Represents a complete list with metadata and items
#[derive(Debug, Serialize, Deserialize)]
pub struct List {
    /// Metadata for the list
    #[serde(flatten)]
    pub metadata: ListMetadata,

    /// List items
    #[serde(default)]
    pub items: Vec<ListItem>,
}

impl List {
    /// Create a new list with the given title
    pub fn new(title: String) -> Self {
        Self {
            metadata: ListMetadata {
                id: Uuid::new_v4(),
                title,
                sharing: vec![],
                updated: Utc::now(),
            },
            items: vec![],
        }
    }

    /// Add a new item to the list
    pub fn add_item(&mut self, text: String) -> &ListItem {
        let anchor = generate_anchor();
        let item = ListItem {
            text,
            status: ItemStatus::Todo,
            anchor,
        };
        self.items.push(item);
        self.metadata.updated = Utc::now();
        self.items.last().unwrap()
    }

    /// Mark an item as done
    pub fn mark_done(&mut self, anchor: &str) -> Result<&ListItem> {
        let idx = self
            .find_by_anchor(anchor)
            .with_context(|| format!("Item with anchor '{}' not found", anchor))?;

        self.items[idx].status = ItemStatus::Done;
        self.metadata.updated = Utc::now();
        Ok(&self.items[idx])
    }

    /// Find an item by its anchor
    pub fn find_by_anchor(&self, anchor: &str) -> Option<usize> {
        self.items.iter().position(|item| item.anchor == anchor)
    }

    /// Find an item by exact text match
    pub fn find_by_text(&self, text: &str) -> Option<usize> {
        self.items
            .iter()
            .position(|item| item.text.to_lowercase() == text.to_lowercase())
    }

    /// Find an item by index (0-based)
    pub fn get_by_index(&self, index: usize) -> Option<&ListItem> {
        self.items.get(index)
    }

    /// Get the file name for this list
    pub fn file_name(&self) -> String {
        format!(
            "{}.md",
            self.metadata.title.to_lowercase().replace(' ', "-")
        )
    }
}

/// Check if an anchor is valid
pub fn is_valid_anchor(anchor: &str) -> bool {
    lazy_static::lazy_static! {
        static ref ANCHOR_RE: Regex = Regex::new(r"^\^[A-Za-z0-9-]{4,}$").unwrap();
    }
    ANCHOR_RE.is_match(anchor)
}

/// Find an item by fuzzy matching text
/// Returns a vector of potential matching indices
pub fn fuzzy_find(items: &[ListItem], query: &str, _threshold: f32) -> Vec<usize> {
    // Simple contains matching for now, can be improved later with a fuzzy matching algorithm
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.text.to_lowercase().contains(&query.to_lowercase()))
        .map(|(i, _)| i)
        .collect()
}

/// Parse a list from a markdown file
pub fn parse_list_from_markdown(_path: &Path) -> Result<List> {
    // Placeholder implementation, to be expanded
    Err(anyhow::anyhow!("Not implemented yet"))
}

/// Save a list to a markdown file
pub fn save_list_to_markdown(_list: &List, _path: &Path) -> Result<()> {
    // Placeholder implementation, to be expanded
    Err(anyhow::anyhow!("Not implemented yet"))
}

