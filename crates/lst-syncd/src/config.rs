use serde::{Deserialize, Serialize};
use anyhow::{Context, Result};
use std::{fs, path::{Path, PathBuf}};

/// Configuration for the lst sync daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Content directory to watch for changes
    pub content_dir: PathBuf,
    
    /// Server configuration for remote sync
    pub server: ServerConfig,
    
    /// Local CRDT storage settings
    pub storage: StorageConfig,
    
    /// Sync behavior settings
    pub sync: SyncSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server URL (if None, runs in local-only mode)
    pub url: Option<String>,
    
    /// Authentication token for server
    pub auth_token: Option<String>,
    
    /// Device identifier
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Directory for CRDT state storage
    pub crdt_dir: PathBuf,
    
    /// Maximum number of CRDT snapshots to keep
    #[serde(default = "default_max_snapshots")]
    pub max_snapshots: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSettings {
    /// Sync interval in seconds
    #[serde(default = "default_sync_interval")]
    pub interval_seconds: u64,
    
    /// Maximum file size to sync (in bytes)
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,
    
    /// File patterns to exclude from sync
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
}

fn default_sync_interval() -> u64 {
    30 // 30 seconds
}

fn default_max_file_size() -> u64 {
    10 * 1024 * 1024 // 10MB
}

fn default_max_snapshots() -> usize {
    100
}

impl Default for SyncConfig {
    fn default() -> Self {
        let home_dir = dirs::home_dir().expect("Cannot determine home directory");
        let content_dir = home_dir.join("lst").join("content");
        let crdt_dir = dirs::config_dir()
            .expect("Cannot determine config directory")
            .join("lst")
            .join("crdt");
            
        Self {
            content_dir,
            server: ServerConfig {
                url: None,
                auth_token: None,
                device_id: uuid::Uuid::new_v4().to_string(),
            },
            storage: StorageConfig {
                crdt_dir,
                max_snapshots: default_max_snapshots(),
            },
            sync: SyncSettings {
                interval_seconds: default_sync_interval(),
                max_file_size: default_max_file_size(),
                exclude_patterns: vec![
                    ".*".to_string(),
                    "*.tmp".to_string(),
                    "*.swp".to_string(),
                ],
            },
        }
    }
}

impl SyncConfig {
    /// Load configuration from file, creating default if it doesn't exist
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            // Create default config
            let default_config = Self::default();
            
            // Ensure config directory exists
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
            }
            
            // Write default config
            let toml_str = toml::to_string_pretty(&default_config)
                .context("Failed to serialize default config")?;
            fs::write(path, toml_str)
                .with_context(|| format!("Failed to write default config to: {}", path.display()))?;
            
            println!("Created default sync daemon config at: {}", path.display());
            return Ok(default_config);
        }
        
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        
        let config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
        
        Ok(config)
    }
}