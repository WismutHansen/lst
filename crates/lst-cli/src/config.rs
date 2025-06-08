use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration for the lst application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub fuzzy: FuzzyConfig,
    #[serde(default)]
    pub paths: PathsConfig,
    #[serde(default)]
    pub server: ServerConfig,
    // Syncd-specific configuration (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub syncd: Option<SyncdConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage: Option<StorageConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync: Option<SyncSettings>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_resolution_order")]
    pub resolution_order: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzyConfig {
    #[serde(default = "default_threshold")]
    pub threshold: f32,
    #[serde(default = "default_max_suggestions")]
    pub max_suggestions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub content_dir: Option<PathBuf>,
    pub media_dir: Option<PathBuf>,
    pub kinds: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub url: Option<String>,
    pub auth_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncdConfig {
    /// Server URL (if None, runs in local-only mode)
    pub url: Option<String>,
    
    /// Authentication token for server
    pub auth_token: Option<String>,
    
    /// Device identifier (auto-generated if missing)
    pub device_id: Option<String>,

    /// Path to the local sync database
    pub database_path: Option<PathBuf>,

    /// Reference to the encryption key
    pub encryption_key_ref: Option<String>,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            ui: UiConfig {
                resolution_order: default_resolution_order(),
            },
            fuzzy: FuzzyConfig {
                threshold: default_threshold(),
                max_suggestions: default_max_suggestions(),
            },
            paths: PathsConfig {
                content_dir: None,
                media_dir: None,
                kinds: None,
            },
            server: ServerConfig::default(),
            syncd: None,
            storage: None,
            sync: None,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            resolution_order: default_resolution_order(),
        }
    }
}

impl Default for FuzzyConfig {
    fn default() -> Self {
        Self {
            threshold: default_threshold(),
            max_suggestions: default_max_suggestions(),
        }
    }
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            content_dir: None,
            media_dir: None,
            kinds: None,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            url: None,
            auth_token: None,
        }
    }
}


fn default_resolution_order() -> Vec<String> {
    vec![
        "anchor".to_string(),
        "exact".to_string(),
        "fuzzy".to_string(),
        "index".to_string(),
        "interactive".to_string(),
    ]
}

fn default_threshold() -> f32 {
    0.75
}

fn default_max_suggestions() -> usize {
    7
}

impl Config {
    /// Load configuration from the default location
    pub fn load() -> Result<Self> {
        // Check if config path is specified via environment variable
        if let Ok(custom_path) = std::env::var("LST_CONFIG") {
            return Self::load_from(&PathBuf::from(custom_path));
        }
        // Always use ~/.config/lst/ regardless of platform
        let home_dir = dirs::home_dir().context("Could not determine home directory")?;
        let config_dir = home_dir.join(".config").join("lst");
        let config_path = config_dir.join("lst.toml");
        if !config_path.exists() {
            // Create default config if it doesn't exist
            fs::create_dir_all(&config_dir).context("Failed to create config directory")?;
            let default_config = Self::default();
            let toml_str = toml::to_string_pretty(&default_config)
                .context("Failed to serialize default config")?;
            fs::write(&config_path, toml_str).context("Failed to write default config file")?;
            return Ok(default_config);
        }
        Self::load_from(&config_path)
    }

    /// Load configuration from a specific path
    pub fn load_from(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(config)
    }

    /// Save configuration to the default location
    pub fn save(&self) -> Result<()> {
        // Always use ~/.config/lst/ regardless of platform
        let home_dir = dirs::home_dir().context("Could not determine home directory")?;
        let config_dir = home_dir.join(".config").join("lst");
        fs::create_dir_all(&config_dir).context("Failed to create config directory")?;
        let config_path = config_dir.join("lst.toml");
        let toml_str = toml::to_string_pretty(self).context("Failed to serialize config")?;
        fs::write(&config_path, toml_str).context("Failed to write config file")?;
        Ok(())
    }

    /// Get the content directory, using default if not configured
    pub fn get_content_dir(&self) -> PathBuf {
        if let Some(ref content_dir) = self.paths.content_dir {
            content_dir.clone()
        } else {
            // Default content directory
            let home_dir = dirs::home_dir().expect("Cannot determine home directory");
            home_dir.join("lst").join("content")
        }
    }

    /// Initialize syncd configuration with defaults
    pub fn init_syncd(&mut self) -> Result<()> {
        if self.syncd.is_none() {
            let crdt_dir = dirs::config_dir()
                .context("Cannot determine config directory")?
                .join("lst")
                .join("crdt");
            
            let config_dir = dirs::config_dir()
                .context("Cannot determine config directory")?
                .join("lst");

            let db_path = config_dir.join("syncd.db");

            self.syncd = Some(SyncdConfig {
                url: None,
                auth_token: None,
                device_id: Some(uuid::Uuid::new_v4().to_string()),
                database_path: Some(db_path),
                encryption_key_ref: Some("lst-master-key".to_string()),
            });
            
            self.storage = Some(StorageConfig {
                crdt_dir,
                max_snapshots: default_max_snapshots(),
            });
            
            self.sync = Some(SyncSettings {
                interval_seconds: default_sync_interval(),
                max_file_size: default_max_file_size(),
                exclude_patterns: vec![
                    ".*".to_string(),
                    "*.tmp".to_string(),
                    "*.swp".to_string(),
                ],
            });
        }
        Ok(())
    }
}

// Global cached configuration: loaded once on first access
lazy_static::lazy_static! {
    static ref GLOBAL_CONFIG: Config = Config::load().expect("Failed to load config");
}

/// Get the global cached configuration
pub fn get_config() -> &'static Config {
    &GLOBAL_CONFIG
}
