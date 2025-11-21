use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::theme::{Theme, ThemeLoader};

/// Configuration for the lst application
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "tauri", derive(Type))]
pub struct Config {
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub fuzzy: FuzzyConfig,
    #[serde(default)]
    pub paths: PathsConfig,
    #[serde(default)]
    pub server: ServerConfig,
    // New tinted theming system
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub theme: Option<Theme>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage: Option<StorageConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync: Option<SyncSettings>,
}

#[cfg(feature = "tauri")]
use specta::Type;

// Legacy theme config for backwards compatibility
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
#[cfg_attr(feature = "tauri", derive(Type))]
pub struct LegacyThemeConfig {
    #[serde(default)]
    pub vars: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "tauri", derive(Type))]
pub struct UiConfig {
    #[serde(default = "default_resolution_order")]
    pub resolution_order: Vec<String>,

    /// Enable Vim-like keybindings in the frontend
    #[serde(default)]
    pub vim_mode: bool,

    /// Leader key used for command sequences (defaults to space)
    #[serde(default = "default_leader_key")]
    pub leader_key: String,

    /// Ask for confirmation before deleting lists or notes
    #[serde(default = "default_confirm_delete")]
    pub confirm_delete: bool,

    // Legacy theme config for backwards compatibility
    #[serde(default)]
    pub theme: LegacyThemeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "tauri", derive(Type))]
pub struct FuzzyConfig {
    #[serde(default = "default_threshold")]
    pub threshold: i64,
    #[serde(default = "default_max_suggestions")]
    pub max_suggestions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "tauri", derive(Type))]
pub struct PathsConfig {
    pub content_dir: Option<PathBuf>,
    pub media_dir: Option<PathBuf>,
    pub kinds: Option<Vec<String>>,
    /// Directory containing theme files (defaults to ~/.config/themes)
    pub themes_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "tauri", derive(Type))]
pub struct ServerConfig {
    /// Host for lst-server daemon (only used when running lst-server)
    pub host: Option<String>,
    /// Port for lst-server daemon (only used when running lst-server)
    pub port: Option<u16>,
    /// Base directory for server databases (only used when running lst-server)
    pub data_dir: Option<PathBuf>,
    /// Tokens database filename (only used when running lst-server)
    pub tokens_db: Option<String>,
    /// Content database filename (only used when running lst-server)
    pub content_db: Option<String>,
    /// Sync database filename (only used when running lst-server)
    pub sync_db: Option<String>,
}

// SyncdConfig removed - consolidated into SyncSettings

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "tauri", derive(Type))]
pub struct StorageConfig {
    /// Directory for CRDT state storage
    pub crdt_dir: PathBuf,

    /// Maximum number of CRDT snapshots to keep
    #[serde(default = "default_max_snapshots")]
    pub max_snapshots: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "tauri", derive(Type))]
pub struct SyncSettings {
    /// Server URL (if None, runs in local-only mode)
    pub server_url: Option<String>,

    /// Reference to the encryption key
    pub encryption_key_ref: Option<String>,

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

/// Machine-specific state that should not be synced across devices
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "tauri", derive(Type))]
pub struct State {
    /// Authentication settings
    #[serde(default)]
    pub auth: AuthState,

    /// Device-specific settings
    #[serde(default)]
    pub device: DeviceState,

    /// Sync database settings
    #[serde(default)]
    pub sync: SyncState,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "tauri", derive(Type))]
pub struct AuthState {
    /// User email address for authentication
    pub email: Option<String>,

    /// Authentication token for server (used for refresh and encryption key derivation)
    pub auth_token: Option<String>,

    /// JWT token for authentication (stored after successful login)
    pub jwt_token: Option<String>,

    /// Expiration timestamp for the JWT token
    #[schemars(with = "String")]
    pub jwt_expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "tauri", derive(Type))]
pub struct DeviceState {
    /// Device identifier (auto-generated if missing)
    pub device_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "tauri", derive(Type))]
pub struct SyncState {
    /// Path to the local sync database
    pub database_path: Option<PathBuf>,
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
                vim_mode: false,
                leader_key: default_leader_key(),
                confirm_delete: default_confirm_delete(),
                theme: LegacyThemeConfig::default(),
            },
            fuzzy: FuzzyConfig {
                threshold: default_threshold(),
                max_suggestions: default_max_suggestions(),
            },
            paths: PathsConfig {
                content_dir: None,
                media_dir: None,
                kinds: None,
                themes_dir: None,
            },
            server: ServerConfig::default(),
            theme: None,
            storage: None,
            sync: None,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            resolution_order: default_resolution_order(),
            vim_mode: false,
            leader_key: default_leader_key(),
            confirm_delete: default_confirm_delete(),
            theme: LegacyThemeConfig::default(),
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
            themes_dir: None,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: None,
            port: None,
            data_dir: None,
            tokens_db: None,
            content_db: None,
            sync_db: None,
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            auth: AuthState::default(),
            device: DeviceState::default(),
            sync: SyncState::default(),
        }
    }
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            email: None,
            auth_token: None,
            jwt_token: None,
            jwt_expires_at: None,
        }
    }
}

impl Default for DeviceState {
    fn default() -> Self {
        Self { device_id: None }
    }
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            database_path: None,
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

fn default_threshold() -> i64 {
    50
}

fn default_max_suggestions() -> usize {
    7
}

fn default_leader_key() -> String {
    " ".to_string()
}

fn default_confirm_delete() -> bool {
    true
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
        let config_path = config_dir.join("config.toml");
        if !config_path.exists() {
            // Create default config if it doesn't exist
            fs::create_dir_all(&config_dir).context("Failed to create config directory")?;
            let default_config = Self::default();
            let mut toml_str = toml::to_string_pretty(&default_config)
                .context("Failed to serialize default config")?;

            // Add schema reference header
            let header = r#"# LST Configuration File
# Schema: https://json-schema.org/draft-07/schema#
# LST Configuration Schema: ./lst-config-schema.json
# For LSP/editor validation, configure your editor to use the schema above

"#;
            toml_str = format!("{}{}", header, toml_str);

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
        let config_path = config_dir.join("config.toml");
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

    /// Initialize sync configuration with defaults
    pub fn init_sync(&mut self) -> Result<()> {
        if self.sync.is_none() {
            let crdt_dir = dirs::config_dir()
                .context("Cannot determine config directory")?
                .join("lst")
                .join("crdt");

            self.sync = Some(SyncSettings {
                server_url: None,
                encryption_key_ref: Some("lst-master-key".to_string()),
                interval_seconds: default_sync_interval(),
                max_file_size: default_max_file_size(),
                exclude_patterns: vec![".*".to_string(), "*.tmp".to_string(), "*.swp".to_string()],
            });

            self.storage = Some(StorageConfig {
                crdt_dir,
                max_snapshots: default_max_snapshots(),
            });
        }
        Ok(())
    }

    /// Get the current theme, loading default if none specified
    pub fn get_theme(&self) -> Result<Theme> {
        if let Some(ref theme) = self.theme {
            Ok(theme.clone())
        } else {
            // Load default theme
            let loader = ThemeLoader::with_config(self.paths.themes_dir.clone());
            loader.load_theme("base16-default-dark")
        }
    }

    /// Set the current theme
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = Some(theme);
    }

    /// Load theme by name
    pub fn load_theme_by_name(&self, name: &str) -> Result<Theme> {
        let loader = ThemeLoader::with_config(self.paths.themes_dir.clone());
        loader.load_theme(name)
    }

    /// Get theme loader
    pub fn get_theme_loader(&self) -> ThemeLoader {
        ThemeLoader::with_config(self.paths.themes_dir.clone())
    }

    /// Generate JSON schema for the configuration
    pub fn generate_schema() -> Result<String> {
        let schema = schemars::schema_for!(Config);
        let json_schema =
            serde_json::to_string_pretty(&schema).context("Failed to serialize schema to JSON")?;
        Ok(json_schema)
    }

    /// Generate default config with schema header for testing
    #[cfg(test)]
    pub fn generate_default_config_with_header() -> String {
        let default_config = Self::default();
        let toml_str =
            toml::to_string_pretty(&default_config).expect("Failed to serialize default config");

        let header = r#"# LST Configuration File
# Schema: https://json-schema.org/draft-07/schema#
# LST Configuration Schema: ./lst-config-schema.json
# For LSP/editor validation, configure your editor to use the schema above

"#;
        format!("{}{}", header, toml_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_includes_schema_header() {
        let config_with_header = Config::generate_default_config_with_header();

        assert!(config_with_header.starts_with("# LST Configuration File"));
        assert!(config_with_header.contains("# LST Configuration Schema: ./lst-config-schema.json"));
        assert!(config_with_header.contains("[fuzzy]"));
        assert!(config_with_header.contains("threshold = 50.0"));
    }
}

impl State {
    /// Load state from the default location
    pub fn load() -> Result<Self> {
        // Check if state path is specified via environment variable
        if let Ok(custom_path) = std::env::var("LST_STATE") {
            return Self::load_from(&PathBuf::from(custom_path));
        }

        let state_path = Self::get_state_path()?;
        if !state_path.exists() {
            // Create default state if it doesn't exist
            let state_dir = state_path.parent().unwrap();
            fs::create_dir_all(state_dir).context("Failed to create state directory")?;
            let default_state = Self::default();
            default_state.save()?;
            return Ok(default_state);
        }
        Self::load_from(&state_path)
    }

    /// Load state from a specific path
    pub fn load_from(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read state file: {}", path.display()))?;

        let state: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse state file: {}", path.display()))?;

        Ok(state)
    }

    /// Save state to the default location
    pub fn save(&self) -> Result<()> {
        let state_path = Self::get_state_path()?;
        let state_dir = state_path.parent().unwrap();
        fs::create_dir_all(state_dir).context("Failed to create state directory")?;
        let toml_str = toml::to_string_pretty(self).context("Failed to serialize state")?;
        fs::write(&state_path, toml_str).context("Failed to write state file")?;
        Ok(())
    }

    /// Get the state file path
    pub fn get_state_path() -> Result<PathBuf> {
        let home_dir = dirs::home_dir().context("Could not determine home directory")?;
        Ok(home_dir
            .join(".local")
            .join("share")
            .join("lst")
            .join("state.toml"))
    }

    /// Initialize state with defaults
    pub fn init(&mut self) -> Result<()> {
        // Generate device ID if not present
        if self.device.device_id.is_none() {
            self.device.device_id = Some(uuid::Uuid::new_v4().to_string());
        }

        // Set default sync database path if not present
        if self.sync.database_path.is_none() {
            let home_dir = dirs::home_dir().context("Could not determine home directory")?;
            let state_dir = home_dir.join(".local").join("share").join("lst");
            self.sync.database_path = Some(state_dir.join("sync.db"));
        }

        Ok(())
    }

    /// Check if JWT token is valid and not expired
    pub fn is_jwt_valid(&self) -> bool {
        if let Some(ref jwt) = self.auth.jwt_token {
            if let Some(expires_at) = self.auth.jwt_expires_at {
                return !jwt.is_empty() && chrono::Utc::now() < expires_at;
            }
        }
        false
    }

    /// Store JWT token with expiration
    pub fn store_jwt(&mut self, jwt: String, expires_at: chrono::DateTime<chrono::Utc>) {
        self.auth.jwt_token = Some(jwt);
        self.auth.jwt_expires_at = Some(expires_at);
    }

    /// Clear JWT token
    pub fn clear_jwt(&mut self) {
        self.auth.jwt_token = None;
        self.auth.jwt_expires_at = None;
    }

    /// Get valid JWT token if available
    pub fn get_jwt(&self) -> Option<&str> {
        if self.is_jwt_valid() {
            self.auth.jwt_token.as_deref()
        } else {
            None
        }
    }

    /// Store email and auth token for authentication
    pub fn store_auth_credentials(&mut self, email: String, auth_token: String) {
        self.auth.email = Some(email);
        self.auth.auth_token = Some(auth_token);
    }

    /// Store auth token for refresh
    pub fn store_auth_token(&mut self, auth_token: String) {
        self.auth.auth_token = Some(auth_token);
    }

    /// Get auth token for refresh
    pub fn get_auth_token(&self) -> Option<&str> {
        self.auth.auth_token.as_deref()
    }

    /// Get stored email address
    pub fn get_email(&self) -> Option<&str> {
        self.auth.email.as_deref()
    }

    /// Get stored credentials for key derivation
    pub fn get_credentials(&self) -> (Option<&str>, Option<&str>) {
        (self.auth.email.as_deref(), self.auth.auth_token.as_deref())
    }

    /// Get device ID, generating one if it doesn't exist
    pub fn get_device_id(&mut self) -> Result<String> {
        if let Some(ref device_id) = self.device.device_id {
            Ok(device_id.clone())
        } else {
            let device_id = uuid::Uuid::new_v4().to_string();
            self.device.device_id = Some(device_id.clone());
            self.save()?;
            Ok(device_id)
        }
    }

    /// Get sync database path
    pub fn get_sync_database_path(&self) -> Option<&PathBuf> {
        self.sync.database_path.as_ref()
    }

    /// Set sync database path
    pub fn set_sync_database_path(&mut self, path: PathBuf) {
        self.sync.database_path = Some(path);
    }

    /// Check if JWT is about to expire (within 5 minutes) and needs refresh
    pub fn needs_jwt_refresh(&self) -> bool {
        if let Some(expires_at) = self.auth.jwt_expires_at {
            let now = chrono::Utc::now();
            let time_until_expiry = expires_at - now;
            time_until_expiry.num_minutes() < 5 // Refresh if less than 5 minutes left
        } else {
            true // No expiration time means we should refresh
        }
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
