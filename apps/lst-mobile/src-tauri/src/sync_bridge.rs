use anyhow::Result;
use lst_cli::models::List;
use crate::{Note, sync::SyncManager};
use std::path::PathBuf;
use notify::{Event, EventKind, event::{CreateKind, ModifyKind, DataChange, RemoveKind}};

/// Bridge between mobile SQLite database and sync system
/// Converts mobile operations into sync-compatible operations
pub struct SyncBridge {
    sync_manager: Option<SyncManager>,
}

impl SyncBridge {
    pub async fn new(config: lst_cli::config::Config) -> Result<Self> {
        let sync_manager = if config.syncd.is_some() && config.is_jwt_valid() {
            Some(SyncManager::new(config).await?)
        } else {
            None
        };

        Ok(Self { sync_manager })
    }

    /// Bridge a list operation to the sync system
    pub async fn sync_list_operation(&mut self, operation: ListOperation<'_>) -> Result<()> {
        if let Some(ref mut manager) = self.sync_manager {
            match operation {
                ListOperation::Create { title, list } => {
                    Self::handle_list_change(manager, &title, &list, EventKind::Create(CreateKind::File)).await?;
                }
                ListOperation::Update { title, list } => {
                    Self::handle_list_change(manager, &title, &list, EventKind::Modify(ModifyKind::Data(DataChange::Content))).await?;
                }
                ListOperation::Delete { title } => {
                    Self::handle_list_delete(manager, &title).await?;
                }
            }
        }
        Ok(())
    }

    /// Bridge a note operation to the sync system
    pub async fn sync_note_operation(&mut self, operation: NoteOperation<'_>) -> Result<()> {
        if let Some(ref mut manager) = self.sync_manager {
            match operation {
                NoteOperation::Create { title, note } => {
                    Self::handle_note_change(manager, &title, &note, EventKind::Create(CreateKind::File)).await?;
                }
                NoteOperation::Update { title, note } => {
                    Self::handle_note_change(manager, &title, &note, EventKind::Modify(ModifyKind::Data(DataChange::Content))).await?;
                }
                NoteOperation::Delete { title } => {
                    Self::handle_note_delete(manager, &title).await?;
                }
            }
        }
        Ok(())
    }

    async fn handle_list_change(manager: &mut SyncManager, title: &str, list: &List, kind: EventKind) -> Result<()> {
        // Convert list to markdown format (same as desktop)
        let markdown_content = Self::list_to_markdown(list)?;
        
        // Create a virtual file path for the list
        let virtual_path = PathBuf::from(format!("lists/{}.md", title));
        
        // Create a synthetic file event
        let event = Event {
            kind,
            paths: vec![virtual_path],
            attrs: Default::default(),
        };

        // Simulate writing the file content to a temporary location for sync
        let temp_dir = std::env::temp_dir().join("lst-mobile-sync");
        std::fs::create_dir_all(&temp_dir)?;
        let temp_file = temp_dir.join(format!("{}.md", title));
        std::fs::write(&temp_file, &markdown_content)?;

        // Trigger sync with the file event
        manager.handle_file_event(event).await?;

        Ok(())
    }

    async fn handle_note_change(manager: &mut SyncManager, title: &str, note: &Note, kind: EventKind) -> Result<()> {
        // Notes are already in markdown format
        let content = &note.content;
        
        // Create a virtual file path for the note
        let virtual_path = PathBuf::from(format!("notes/{}.md", title));
        
        // Create a synthetic file event
        let event = Event {
            kind,
            paths: vec![virtual_path],
            attrs: Default::default(),
        };

        // Simulate writing the file content to a temporary location for sync
        let temp_dir = std::env::temp_dir().join("lst-mobile-sync");
        std::fs::create_dir_all(&temp_dir)?;
        let temp_file = temp_dir.join(format!("note_{}.md", title));
        std::fs::write(&temp_file, content)?;

        // Trigger sync with the file event
        manager.handle_file_event(event).await?;

        Ok(())
    }

    async fn handle_list_delete(manager: &mut SyncManager, title: &str) -> Result<()> {
        let virtual_path = PathBuf::from(format!("lists/{}.md", title));
        
        let event = Event {
            kind: EventKind::Remove(RemoveKind::File),
            paths: vec![virtual_path],
            attrs: Default::default(),
        };

        manager.handle_file_event(event).await?;
        Ok(())
    }

    async fn handle_note_delete(manager: &mut SyncManager, title: &str) -> Result<()> {
        let virtual_path = PathBuf::from(format!("notes/{}.md", title));
        
        let event = Event {
            kind: EventKind::Remove(RemoveKind::File),
            paths: vec![virtual_path],
            attrs: Default::default(),
        };

        manager.handle_file_event(event).await?;
        Ok(())
    }

    /// Convert a List struct to markdown format (compatible with CLI)
    fn list_to_markdown(list: &List) -> Result<String> {
        // Create YAML frontmatter with metadata
        let frontmatter = serde_yaml::to_string(&list.metadata)
            .unwrap_or_else(|_| format!("title: \"{}\"\n", list.metadata.title));
        
        let mut content = format!("---\n{}---\n\n", frontmatter);
        
        // Add uncategorized items first (no headline)
        for item in &list.uncategorized_items {
            let status = match item.status {
                lst_cli::models::ItemStatus::Todo => " ",
                lst_cli::models::ItemStatus::Done => "x",
            };
            content.push_str(&format!("- [{}] {}  {}\n", status, item.text, item.anchor));
        }
        
        // Add blank line between uncategorized and categorized if both exist
        if !list.uncategorized_items.is_empty() && !list.categories.is_empty() {
            content.push('\n');
        }
        
        // Add categorized items with headlines
        for category in &list.categories {
            content.push_str(&format!("## {}\n", category.name));
            for item in &category.items {
                let status = match item.status {
                    lst_cli::models::ItemStatus::Todo => " ",
                    lst_cli::models::ItemStatus::Done => "x", 
                };
                content.push_str(&format!("- [{}] {}  {}\n", status, item.text, item.anchor));
            }
            content.push('\n');
        }
        
        Ok(content)
    }

    /// Trigger a full sync
    pub async fn trigger_full_sync(&mut self) -> Result<()> {
        if let Some(ref mut manager) = self.sync_manager {
            manager.periodic_sync().await?;
        }
        Ok(())
    }
}

/// Operations that can be performed on lists
#[derive(Debug)]
pub enum ListOperation<'a> {
    Create { title: String, list: &'a List },
    Update { title: String, list: &'a List },
    Delete { title: String },
}

/// Operations that can be performed on notes  
#[derive(Debug)]
pub enum NoteOperation<'a> {
    Create { title: String, note: &'a Note },
    Update { title: String, note: &'a Note },
    Delete { title: String },
}