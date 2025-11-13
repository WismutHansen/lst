// Mobile-specific configuration management
// This file provides mobile-only config functions that don't touch desktop config

use anyhow::Result;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Mutex;
use lst_core::crypto;

lazy_static! {
    static ref MOBILE_CONFIG: Mutex<MobileConfig> = Mutex::new(MobileConfig::default());
}

/// Complete mobile app configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileConfig {
    pub sync: MobileSyncConfig,
    pub ui: MobileUiConfig,
    pub syncd: Option<MobileSyncdConfig>,
    pub sync_settings: Option<MobileSyncSettings>,
}

impl Default for MobileConfig {
    fn default() -> Self {
        MobileConfig {
            sync: MobileSyncConfig::new(),
            ui: MobileUiConfig::default(),
            syncd: None,
            sync_settings: Some(MobileSyncSettings::default()),
        }
    }
}

impl MobileConfig {
    /// Check if JWT token is valid
    pub fn is_jwt_valid(&self) -> bool {
        self.sync.is_jwt_valid()
    }

    /// Store JWT token
    pub fn store_jwt(&mut self, token: String, expires_at: chrono::DateTime<chrono::Utc>) {
        self.sync.store_jwt(token, expires_at);
    }
}

/// Mobile app sync configuration stored in SQLite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileSyncConfig {
    pub server_url: Option<String>,
    pub email: Option<String>,
    pub auth_token: Option<String>,
    pub jwt_token: Option<String>,
    pub jwt_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub device_id: Option<String>,
}

impl MobileSyncConfig {
    /// Create new config with default values
    pub fn new() -> Self {
        MobileSyncConfig {
            server_url: None,
            email: None,
            auth_token: None,
            jwt_token: None,
            device_id: Some(uuid::Uuid::new_v4().to_string()),
            jwt_expires_at: None,
        }
    }

    /// Load mobile sync config from database
    pub fn load_from_db(db: &crate::database::Database) -> Result<Self> {
        let server_url = db.load_sync_config("server_url")?;
        let email = db.load_sync_config("email")?;
        let auth_token = db.load_sync_config("auth_token")?;
        let jwt_token = db.load_sync_config("jwt_token")?;
        let device_id = db.load_sync_config("device_id")?;

        let jwt_expires_at = if let Some(expires_str) = db.load_sync_config("jwt_expires_at")? {
            chrono::DateTime::parse_from_rfc3339(&expires_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .ok()
        } else {
            None
        };

        Ok(Self {
            server_url,
            email,
            auth_token,
            jwt_token,
            jwt_expires_at,
            device_id,
        })
    }

    /// Save mobile sync config to database
    pub fn save_to_db(&self, db: &crate::database::Database) -> Result<()> {
        if let Some(ref url) = self.server_url {
            db.save_sync_config("server_url", url)?;
        }
        if let Some(ref email) = self.email {
            db.save_sync_config("email", email)?;
        }
        if let Some(ref auth_token) = self.auth_token {
            db.save_sync_config("auth_token", auth_token)?;
        }
        if let Some(ref token) = self.jwt_token {
            db.save_sync_config("jwt_token", token)?;
        }
        if let Some(ref device_id) = self.device_id {
            db.save_sync_config("device_id", device_id)?;
        }
        if let Some(expires_at) = self.jwt_expires_at {
            db.save_sync_config("jwt_expires_at", &expires_at.to_rfc3339())?;
        }
        Ok(())
    }

    /// Check if JWT token is valid and not expired
    pub fn is_jwt_valid(&self) -> bool {
        if let Some(ref jwt) = self.jwt_token {
            if let Some(expires_at) = self.jwt_expires_at {
                return !jwt.is_empty() && chrono::Utc::now() < expires_at;
            }
        }
        false
    }

    /// Check if sync is configured
    pub fn is_sync_configured(&self) -> bool {
        self.server_url.is_some() && self.device_id.is_some()
    }

    /// Update JWT token
    pub fn store_jwt(&mut self, token: String, expires_at: chrono::DateTime<chrono::Utc>) {
        self.jwt_token = Some(token);
        self.jwt_expires_at = Some(expires_at);
    }

    /// Store email and auth token for credential-based key derivation
    pub fn store_auth_credentials(&mut self, email: String, auth_token: String) {
        self.email = Some(email);
        self.auth_token = Some(auth_token);
    }

    /// Get stored credentials for key derivation
    pub fn get_credentials(&self) -> (Option<&str>, Option<&str>) {
        (self.email.as_deref(), self.auth_token.as_deref())
    }

    /// Get stored email address
    pub fn get_email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    /// Get stored auth token
    pub fn get_auth_token(&self) -> Option<&str> {
        self.auth_token.as_deref()
    }
}

/// Mobile-specific UI configuration with defaults
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MobileUiConfig {
    pub resolution_order: Option<Vec<String>>,
    pub keybind_mode: Option<String>,
    pub compact_mode: Option<bool>,
    pub leader_key: Option<String>,
    pub theme: Option<MobileThemeConfig>,
}

impl Default for MobileUiConfig {
    fn default() -> Self {
        MobileUiConfig {
            resolution_order: Some(vec!["mobile".to_string(), "default".to_string()]),
            keybind_mode: Some("mobile".to_string()),
            compact_mode: Some(true),
            leader_key: Some("Escape".to_string()),
            theme: Some(MobileThemeConfig::default()),
        }
    }
}

/// Mobile-specific theme configuration
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MobileThemeConfig {
    pub vars: Option<std::collections::HashMap<String, String>>,
}

impl Default for MobileThemeConfig {
    fn default() -> Self {
        MobileThemeConfig {
            vars: Some(std::collections::HashMap::new()),
        }
    }
}

/// Get current mobile config
pub fn get_current_config() -> MobileConfig {
    let config_guard = MOBILE_CONFIG.lock().unwrap();
    config_guard.clone()
}

/// Update mobile config
pub fn update_config<F>(updater: F)
where
    F: FnOnce(&mut MobileConfig),
{
    let mut config_guard = MOBILE_CONFIG.lock().unwrap();
    updater(&mut *config_guard);
}

/// Save mobile configuration to database
pub fn save_config_to_db(db: &crate::database::Database) -> Result<()> {
    let config = get_current_config();

    // Save sync configuration
    config.sync.save_to_db(db)?;

    // Save UI configuration as JSON
    if let Ok(ui_json) = serde_json::to_string(&config.ui) {
        db.save_sync_config("ui_config", &ui_json)?;
    }

    // Save syncd configuration if it exists
    if let Some(ref syncd) = config.syncd {
        if let Ok(syncd_json) = serde_json::to_string(syncd) {
            db.save_sync_config("syncd_config", &syncd_json)?;
        }
    }

    // Save sync settings if they exist
    if let Some(ref sync_settings) = config.sync_settings {
        if let Ok(settings_json) = serde_json::to_string(sync_settings) {
            db.save_sync_config("sync_settings", &settings_json)?;
        }
    }

    println!("📱 Config: Saved mobile configuration to database");
    Ok(())
}

/// Load mobile configuration from database
pub fn load_config_from_db(db: &crate::database::Database) -> Result<()> {
    update_config(|config| {
        // Load sync configuration
        if let Ok(sync_config) = MobileSyncConfig::load_from_db(db) {
            config.sync = sync_config;
        }

        // Load UI configuration
        if let Ok(Some(ui_json)) = db.load_sync_config("ui_config") {
            if let Ok(ui_config) = serde_json::from_str::<MobileUiConfig>(&ui_json) {
                config.ui = ui_config;
            }
        }

        // Load syncd configuration
        if let Ok(Some(syncd_json)) = db.load_sync_config("syncd_config") {
            if let Ok(syncd_config) = serde_json::from_str::<MobileSyncdConfig>(&syncd_json) {
                config.syncd = Some(syncd_config);
            }
        }

        // Load sync settings
        if let Ok(Some(settings_json)) = db.load_sync_config("sync_settings") {
            if let Ok(sync_settings) = serde_json::from_str::<MobileSyncSettings>(&settings_json) {
                config.sync_settings = Some(sync_settings);
            }
        }

        println!("📱 Config: Loaded mobile configuration from database");
    });

    Ok(())
}

/// Mobile-specific syncd configuration (equivalent to lst_cli::config::SyncdConfig)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileSyncdConfig {
    pub url: Option<String>,
    pub device_id: Option<String>,
    pub database_path: Option<std::path::PathBuf>,
    pub encryption_key_path: Option<std::path::PathBuf>,
}

impl MobileSyncdConfig {
    pub fn new(server_url: String, device_id: String) -> Self {
        // Set up mobile-specific paths - use persistent storage
        let app_data_dir = if let Some(data_dir) = dirs::data_dir() {
            data_dir.join("lst-mobile")
        } else {
            // Fallback to temp if data_dir fails
            std::env::temp_dir().join("lst-mobile")
        };
        let encryption_key_path = match crypto::get_mobile_master_key_path() {
            Ok(path) => path,
            Err(err) => {
                eprintln!(
                    "Warning: Falling back to legacy mobile key path after error resolving default: {}",
                    err
                );
                app_data_dir.join("sync.key")
            }
        };

        MobileSyncdConfig {
            url: Some(server_url),
            device_id: Some(device_id),
            database_path: Some(app_data_dir.join("sync.db")),
            encryption_key_path: Some(encryption_key_path),
        }
    }
}

/// Mobile-specific sync settings (equivalent to lst_cli::config::SyncSettings)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileSyncSettings {
    pub interval_seconds: u64,
    pub max_file_size: u64,
    pub exclude_patterns: Vec<String>,
}

impl Default for MobileSyncSettings {
    fn default() -> Self {
        MobileSyncSettings {
            interval_seconds: 30,            // 30 seconds for mobile
            max_file_size: 10 * 1024 * 1024, // 10MB
            exclude_patterns: vec![],
        }
    }
}

/// Mobile-specific storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileStorageConfig {
    pub crdt_dir: std::path::PathBuf,
}

impl Default for MobileStorageConfig {
    fn default() -> Self {
        // Use persistent app data directory instead of temp
        let app_data_dir = if let Some(data_dir) = dirs::data_dir() {
            data_dir.join("lst-mobile")
        } else {
            // Fallback to temp if data_dir fails
            std::env::temp_dir().join("lst-mobile")
        };

        MobileStorageConfig {
            crdt_dir: app_data_dir.join("crdt"),
        }
    }
}

impl MobileConfig {
    /// Get JWT token for sync operations
    pub fn get_jwt(&self) -> Option<&str> {
        self.sync.jwt_token.as_deref()
    }

    /// Set up sync configuration with server details
    pub fn setup_sync(&mut self, server_url: String, device_id: String) {
        self.syncd = Some(MobileSyncdConfig::new(server_url, device_id));

        // Ensure sync settings exist
        if self.sync_settings.is_none() {
            self.sync_settings = Some(MobileSyncSettings::default());
        }
    }

    /// Check if syncd is configured
    pub fn has_syncd(&self) -> bool {
        self.syncd.is_some()
    }

    /// Get storage configuration
    pub fn get_storage(&self) -> MobileStorageConfig {
        MobileStorageConfig::default()
    }
}
