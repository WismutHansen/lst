use crate::{mobile_config, mobile_sync::MobileSyncManager, Note};
use anyhow::Result;
use lst_cli::models::List;
use lst_core::storage;
use notify::{
    event::{CreateKind, DataChange, ModifyKind, RemoveKind},
    Event, EventKind,
};

/// Bridge between mobile SQLite database and sync system
/// Converts mobile operations into sync-compatible operations
pub struct SyncBridge {
    sync_manager: Option<MobileSyncManager>,
}

impl SyncBridge {
    pub async fn new() -> Result<Self> {
        let config = mobile_config::get_current_config();

        let sync_manager = if config.has_syncd() && config.is_jwt_valid() {
            match MobileSyncManager::new(config).await {
                Ok(manager) => {
                    println!("📱 SyncBridge: Successfully created mobile sync manager");
                    Some(manager)
                }
                Err(e) => {
                    println!("📱 SyncBridge: Failed to create mobile sync manager: {}", e);
                    None
                }
            }
        } else {
            println!("📱 SyncBridge: Sync not configured or JWT invalid");
            None
        };

        Ok(Self { sync_manager })
    }

    /// Bridge a list operation to the sync system
    pub async fn sync_list_operation(&mut self, operation: ListOperation<'_>) -> Result<()> {
        if let Some(ref mut manager) = self.sync_manager {
            match operation {
                ListOperation::Create { title, list } => {
                    Self::handle_list_change(
                        manager,
                        &title,
                        &list,
                        EventKind::Create(CreateKind::File),
                    )
                    .await?;
                }
                ListOperation::Update { title, list } => {
                    Self::handle_list_change(
                        manager,
                        &title,
                        &list,
                        EventKind::Modify(ModifyKind::Data(DataChange::Content)),
                    )
                    .await?;
                }
                ListOperation::Delete { title } => {
                    Self::handle_list_delete(manager, &title).await?;
                }
            }
        } else {
            println!("📱 Mobile sync: No sync manager available for list operation");
        }
        Ok(())
    }

    /// Bridge a note operation to the sync system
    pub async fn sync_note_operation(&mut self, operation: NoteOperation<'_>) -> Result<()> {
        if let Some(ref mut manager) = self.sync_manager {
            match operation {
                NoteOperation::Create { title, note } => {
                    Self::handle_note_change(
                        manager,
                        &title,
                        &note,
                        EventKind::Create(CreateKind::File),
                    )
                    .await?;
                }
                NoteOperation::Update { title, note } => {
                    Self::handle_note_change(
                        manager,
                        &title,
                        &note,
                        EventKind::Modify(ModifyKind::Data(DataChange::Content)),
                    )
                    .await?;
                }
                NoteOperation::Delete { title } => {
                    Self::handle_note_delete(manager, &title).await?;
                }
            }
        } else {
            println!("📱 Mobile sync: No sync manager available for note operation");
        }
        Ok(())
    }

    async fn handle_list_change(
        manager: &mut MobileSyncManager,
        title: &str,
        list: &List,
        kind: EventKind,
    ) -> Result<()> {
        let markdown_content = Self::list_to_markdown(list)?;
        let lists_dir = storage::get_lists_dir()?;
        let file_path = lists_dir.join(format!("{}.md", title));

        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&file_path, &markdown_content)?;

        let event = Event {
            kind,
            paths: vec![file_path.clone()],
            attrs: Default::default(),
        };

        println!(
            "📱 Mobile sync: Processing list '{}' with path {}",
            title,
            file_path.display()
        );

        manager.handle_file_event(event).await?;

        Ok(())
    }

    async fn handle_note_change(
        manager: &mut MobileSyncManager,
        title: &str,
        note: &Note,
        kind: EventKind,
    ) -> Result<()> {
        let content = &note.content;
        let notes_dir = storage::get_notes_dir()?;
        let file_path = notes_dir.join(format!("{}.md", title));

        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&file_path, content)?;

        let event = Event {
            kind,
            paths: vec![file_path.clone()],
            attrs: Default::default(),
        };

        println!(
            "📱 Mobile sync: Processing note '{}' with path {}",
            title,
            file_path.display()
        );

        manager.handle_file_event(event).await?;

        Ok(())
    }

    async fn handle_list_delete(manager: &mut MobileSyncManager, title: &str) -> Result<()> {
        let lists_dir = storage::get_lists_dir()?;
        let file_path = lists_dir.join(format!("{}.md", title));

        if file_path.exists() {
            let _ = std::fs::remove_file(&file_path);
        }

        let event = Event {
            kind: EventKind::Remove(RemoveKind::File),
            paths: vec![file_path],
            attrs: Default::default(),
        };

        manager.handle_file_event(event).await?;
        Ok(())
    }

    async fn handle_note_delete(manager: &mut MobileSyncManager, title: &str) -> Result<()> {
        let notes_dir = storage::get_notes_dir()?;
        let file_path = notes_dir.join(format!("{}.md", title));

        if file_path.exists() {
            let _ = std::fs::remove_file(&file_path);
        }

        let event = Event {
            kind: EventKind::Remove(RemoveKind::File),
            paths: vec![file_path],
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
        } else {
            println!("📱 Mobile sync: No sync manager available for full sync");
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
