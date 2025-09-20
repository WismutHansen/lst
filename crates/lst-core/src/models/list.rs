use crate::storage::get_lists_dir;
use chrono::{DateTime, Utc};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use rand::distributions::{Alphanumeric, DistString};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[cfg(feature = "tauri")]
use specta::Type;

use std::path::PathBuf;
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
#[cfg_attr(feature = "tauri", derive(Type))]
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
#[cfg_attr(feature = "tauri", derive(Type))]
pub enum ItemStatus {
    Todo,
    Done,
}

/// Represents a single item in a list
#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "tauri", derive(Type))]
pub struct ListItem {
    /// The text content of the item
    pub text: String,

    /// The status of the item (todo or done)
    pub status: ItemStatus,

    /// Unique anchor identifier for the item
    pub anchor: String,
}

/// Represents a category containing list items
#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "tauri", derive(Type))]
pub struct Category {
    /// The name of the category
    pub name: String,

    /// Items in this category
    pub items: Vec<ListItem>,
}

/// Represents a complete list with metadata and items
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "tauri", derive(Type))]
pub struct List {
    /// Metadata for the list
    #[serde(flatten)]
    pub metadata: ListMetadata,

    /// Items without category (before first headline)
    #[serde(default)]
    pub uncategorized_items: Vec<ListItem>,

    /// Categorized items
    #[serde(default)]
    pub categories: Vec<Category>,

    /// Legacy field for backward compatibility - will be migrated to uncategorized_items
    #[serde(default, skip_serializing)]
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
            uncategorized_items: vec![],
            categories: vec![],
            items: vec![],
        }
    }

    /// Add a new item to the list (uncategorized)
    pub fn add_item(&mut self, text: String) -> &ListItem {
        let anchor = generate_anchor();
        let item = ListItem {
            text,
            status: ItemStatus::Todo,
            anchor,
        };
        self.uncategorized_items.push(item);
        self.metadata.updated = Utc::now();
        self.uncategorized_items.last().unwrap()
    }

    /// Add a new item to a specific category
    pub fn add_item_to_category(&mut self, text: String, category: Option<&str>) -> ListItem {
        let anchor = generate_anchor();
        let item = ListItem {
            text,
            status: ItemStatus::Todo,
            anchor,
        };

        self.metadata.updated = Utc::now();

        match category {
            Some(cat_name) => {
                if let Some(cat) = self.categories.iter_mut().find(|c| c.name == cat_name) {
                    cat.items.push(item.clone());
                    item
                } else {
                    // Create new category
                    let new_cat = Category {
                        name: cat_name.to_string(),
                        items: vec![item.clone()],
                    };
                    self.categories.push(new_cat);
                    item
                }
            }
            None => {
                self.uncategorized_items.push(item.clone());
                item
            }
        }
    }

    /// Get all items across all categories
    pub fn all_items(&self) -> impl Iterator<Item = &ListItem> {
        self.uncategorized_items.iter()
            .chain(self.categories.iter().flat_map(|c| c.items.iter()))
    }

    /// Get all items across all categories (mutable)
    pub fn all_items_mut(&mut self) -> impl Iterator<Item = &mut ListItem> {
        self.uncategorized_items.iter_mut()
            .chain(self.categories.iter_mut().flat_map(|c| c.items.iter_mut()))
    }

    /// Find an item by its anchor (returns global index across all items)
    pub fn find_by_anchor(&self, anchor: &str) -> Option<usize> {
        self.all_items().position(|item| item.anchor == anchor)
    }

    /// Find an item by exact text match (returns global index across all items)
    pub fn find_by_text(&self, text: &str) -> Option<usize> {
        self.all_items()
            .position(|item| item.text.to_lowercase() == text.to_lowercase())
    }

    /// Find an item by index (0-based, across all items)
    pub fn get_by_index(&self, index: usize) -> Option<&ListItem> {
        self.all_items().nth(index)
    }

    /// Find an item by anchor and return mutable reference with location info
    pub fn find_item_mut_by_anchor(&mut self, anchor: &str) -> Option<&mut ListItem> {
        // Check uncategorized items first
        if let Some(item) = self.uncategorized_items.iter_mut().find(|item| item.anchor == anchor) {
            return Some(item);
        }
        
        // Check categorized items
        for category in &mut self.categories {
            if let Some(item) = category.items.iter_mut().find(|item| item.anchor == anchor) {
                return Some(item);
            }
        }
        
        None
    }

    /// Get the file name for this list
    pub fn file_name(&self) -> String {
        format!(
            "{}.md",
            self.metadata.title.to_lowercase().replace(' ', "-")
        )
    }
    /// Get the file path (currently just returns the file name; prepend a dir if needed)
    pub fn file_path(&self) -> PathBuf {
        let lists_dir = get_lists_dir().unwrap();
        lists_dir.join(self.file_name())
    }
}

/// Check if an anchor is valid
pub fn is_valid_anchor(anchor: &str) -> bool {
    lazy_static::lazy_static! {
        static ref ANCHOR_RE: Regex = Regex::new(r"^\^[A-Za-z0-9-]{4,}$").unwrap();
    }
    ANCHOR_RE.is_match(anchor)
}

/// Find items by fuzzy matching text with scoring and ranking
/// Returns a vector of matching indices sorted by relevance score
pub fn fuzzy_find(items: &[ListItem], query: &str, threshold: i64) -> Vec<usize> {
    if query.is_empty() {
        return Vec::new();
    }

    let matcher = SkimMatcherV2::default();
    let mut matches_with_scores: Vec<(usize, i64)> = Vec::new();

    for (index, item) in items.iter().enumerate() {
        // Try fuzzy matching on the item text
        if let Some(score) = matcher.fuzzy_match(&item.text, query) {
            if score >= threshold {
                matches_with_scores.push((index, score));
            }
        }

        // Also try substring matching as fallback for very short queries
        if query.len() <= 3 && item.text.to_lowercase().contains(&query.to_lowercase()) {
            // Give substring matches a lower score boost
            let substring_score = (query.len() * 50) as i64;
            if !matches_with_scores.iter().any(|(idx, _)| *idx == index) {
                matches_with_scores.push((index, substring_score));
            }
        }
    }

    // Sort by score (highest first) and return indices
    matches_with_scores.sort_by(|a, b| b.1.cmp(&a.1));
    matches_with_scores.into_iter().map(|(idx, _)| idx).collect()
}


