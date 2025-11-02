use anyhow::Context;
use serde::Deserialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

/// Server configuration loaded from TOML file
#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    #[serde(default)]
    pub server: ServerSettings,
    /// SMTP/email settings; if absent, login links are logged to stdout
    pub email: Option<EmailSettings>,
    #[serde(default)]
    pub paths: PathsSettings,
    #[serde(default)]
    pub database: DatabaseSettings,
}

/// Network settings for the HTTP server
#[derive(Debug, Deserialize, Clone)]
pub struct ServerSettings {
    /// Host/interface to bind to, e.g. "127.0.0.1"
    #[serde(default = "default_host")]
    pub host: String,
    /// Port to listen on, e.g. 3000
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    5673
}

/// SMTP relay settings for sending login emails
#[derive(Debug, Deserialize, Clone)]
pub struct EmailSettings {
    pub smtp_host: String,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub sender: String,
}

/// Path settings shared with CLI
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct PathsSettings {
    /// Root path for content files
    pub content_dir: Option<String>,
    /// Document kinds (e.g. ["lists", "notes", "posts"])
    pub kinds: Option<Vec<String>>,
    /// Subdirectory for media files under content root
    pub media_dir: Option<String>,
}

/// Database configuration for the server
#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseSettings {
    /// Directory where server databases are stored
    #[serde(default = "default_database_dir")]
    pub data_dir: String,
    /// Path to tokens database file (relative to data_dir if not absolute)
    #[serde(default = "default_tokens_db")]
    pub tokens_db: String,
    /// Path to content database file (relative to data_dir if not absolute)
    #[serde(default = "default_content_db")]
    pub content_db: String,
    /// Path to sync database file (relative to data_dir if not absolute)
    #[serde(default = "default_sync_db")]
    pub sync_db: String,
}

fn default_database_dir() -> String {
    "~/.local/share/lst/lst_server_data".to_string()
}

fn default_tokens_db() -> String {
    "tokens.db".to_string()
}

fn default_content_db() -> String {
    "content.db".to_string()
}

fn default_sync_db() -> String {
    "sync.db".to_string()
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

impl Default for PathsSettings {
    fn default() -> Self {
        Self {
            content_dir: None,
            kinds: None,
            media_dir: None,
        }
    }
}

impl Default for DatabaseSettings {
    fn default() -> Self {
        Self {
            data_dir: default_database_dir(),
            tokens_db: default_tokens_db(),
            content_db: default_content_db(),
            sync_db: default_sync_db(),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server: ServerSettings::default(),
            email: None,
            paths: PathsSettings::default(),
            database: DatabaseSettings::default(),
        }
    }
}

impl Settings {
    /// Load and parse the configuration from the given TOML file path
    pub fn from_file(path: &Path) -> anyhow::Result<Self> {
        let data = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file {}", path.display()))?;
        let mut settings: Settings = toml::from_str(&data)
            .with_context(|| format!("failed to parse config file {}", path.display()))?;
        // Expand SMTP password from environment variable if in ${VAR} form
        if let Some(ref mut email) = settings.email {
            if email.smtp_pass.starts_with("${") && email.smtp_pass.ends_with('}') {
                let var = &email.smtp_pass[2..email.smtp_pass.len() - 1];
                email.smtp_pass = env::var(var)
                    .with_context(|| format!("missing environment var {} for smtp_pass", var))?;
            }
        }
        Ok(settings)
    }
}

impl DatabaseSettings {
    /// Resolve the data directory path, expanding ~ to home directory
    pub fn resolve_data_dir(&self) -> anyhow::Result<PathBuf> {
        if self.data_dir.starts_with("~/") {
            let home_dir = dirs::home_dir().context("Could not determine home directory")?;
            Ok(home_dir.join(&self.data_dir[2..]))
        } else {
            Ok(PathBuf::from(&self.data_dir))
        }
    }

    /// Get the full path to the tokens database
    pub fn tokens_db_path(&self) -> anyhow::Result<PathBuf> {
        let base_dir = self.resolve_data_dir()?;
        if Path::new(&self.tokens_db).is_absolute() {
            Ok(PathBuf::from(&self.tokens_db))
        } else {
            Ok(base_dir.join(&self.tokens_db))
        }
    }

    /// Get the full path to the content database
    pub fn content_db_path(&self) -> anyhow::Result<PathBuf> {
        let base_dir = self.resolve_data_dir()?;
        if Path::new(&self.content_db).is_absolute() {
            Ok(PathBuf::from(&self.content_db))
        } else {
            Ok(base_dir.join(&self.content_db))
        }
    }

    /// Get the full path to the sync database  
    pub fn sync_db_path(&self) -> anyhow::Result<PathBuf> {
        let base_dir = self.resolve_data_dir()?;
        if Path::new(&self.sync_db).is_absolute() {
            Ok(PathBuf::from(&self.sync_db))
        } else {
            Ok(base_dir.join(&self.sync_db))
        }
    }
}
