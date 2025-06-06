use serde::Deserialize;
use anyhow::Context;
use std::{env, fs, path::Path};

/// Server configuration loaded from TOML file
#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    #[serde(default)]
    pub lst_server: ServerSettings,
    /// SMTP/email settings; if absent, login links are logged to stdout
    pub email: Option<EmailSettings>,
    #[serde(default)]
    pub paths: PathsSettings,
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
    3000
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
pub struct PathsSettings {
    /// Root path for content files
    pub content_dir: Option<String>,
    /// Document kinds (e.g. ["lists", "notes", "posts"])
    pub kinds: Option<Vec<String>>,
    /// Subdirectory for media files under content root
    pub media_dir: Option<String>,
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

    /// Load configuration from the standard lst config location
    pub fn load() -> anyhow::Result<Self> {
        let home_dir = dirs::home_dir().context("Could not determine home directory")?;
        let config_path = home_dir.join(".config").join("lst").join("lst.toml");
        Self::from_file(&config_path)
    }
}