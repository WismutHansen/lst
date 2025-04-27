use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration for the lst application
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub fuzzy: FuzzyConfig,
    #[serde(default)]
    pub paths: PathsConfig,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ServerConfig {
    #[serde(default = "default_server_url")]
    pub url: String,
    pub auth_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_resolution_order")]
    pub resolution_order: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FuzzyConfig {
    #[serde(default = "default_threshold")]
    pub threshold: f32,
    #[serde(default = "default_max_suggestions")]
    pub max_suggestions: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PathsConfig {
    pub content_dir: Option<PathBuf>,
    pub media_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
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
            },
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
        }
    }
}

fn default_server_url() -> String {
    "http://localhost:3000/api".to_string()
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
}

