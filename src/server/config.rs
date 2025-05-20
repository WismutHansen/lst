use serde::Deserialize;
use anyhow::Context;
use std::{env, fs, path::Path};

/// Server configuration loaded from TOML file
#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub server: ServerSettings,
    /// SMTP/email settings; if absent, login links are logged to stdout
    pub email: Option<EmailSettings>,
    pub content: ContentSettings,
}

/// Network settings for the HTTP server
#[derive(Debug, Deserialize, Clone)]
pub struct ServerSettings {
    /// Host/interface to bind to, e.g. "127.0.0.1"
    pub host: String,
    /// Port to listen on, e.g. 3000
    pub port: u16,
}

/// SMTP relay settings for sending login emails
#[derive(Debug, Deserialize, Clone)]
pub struct EmailSettings {
    pub smtp_host: String,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub sender: String,
}

/// Content directory layout and kinds
#[derive(Debug, Deserialize, Clone)]
pub struct ContentSettings {
    /// Root path for content files
    pub root: String,
    /// Document kinds (e.g. ["lists", "notes", "posts"])
    pub kinds: Vec<String>,
    /// Subdirectory for media files under content root
    pub media_dir: String,
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