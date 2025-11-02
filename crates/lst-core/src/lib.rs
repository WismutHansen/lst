pub mod commands;
pub mod config;
pub mod crypto;
pub mod models;
pub mod storage;
pub mod sync;
pub mod theme;

// Re-export commonly used types and functions
pub use config::{get_config, Config};
pub use models::{ItemStatus, List, ListItem};
pub use storage::{list_lists, list_notes, markdown, notes};
pub use theme::{Theme, ThemeInfo, ThemeLoader, ThemeSystem, ThemeVariant};
