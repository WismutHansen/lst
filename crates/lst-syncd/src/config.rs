use anyhow::{Context, Result};
pub use lst_cli::config::Config;
use lst_core::config::State;
use std::{fs, path::Path};

/// Load syncd configuration from the unified lst config
pub fn load_syncd_config(path: &Path) -> Result<Config> {
    let config = if !path.exists() {
        // Create default config with syncd enabled
        let mut default_config = Config::default();
        default_config.init_sync()?;

        // Ensure config directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        // Write default config
        default_config
            .save()
            .context("Failed to save default config with syncd settings")?;

        println!(
            "Created default config with sync daemon settings at: {}",
            path.display()
        );
        default_config
    } else {
        let mut config = Config::load_from(path)?;

        // Initialize sync if not present
        if config.sync.is_none() {
            config.init_sync()?;
            config
                .save()
                .context("Failed to save config with sync settings")?;
            println!("Added sync settings to existing config");
        }

        // Ensure state is initialized with required fields
        let mut state = State::load().unwrap_or_default();
        let mut state_updated = false;

        if state.device.device_id.is_none() {
            let device_id = uuid::Uuid::new_v4().to_string();
            state.device.device_id = Some(device_id.clone());
            println!("Generated new device_id: {}", device_id);
            state_updated = true;
        }

        if state.sync.database_path.is_none() {
            let db_path = dirs::home_dir()
                .context("Cannot determine home directory")?
                .join(".local")
                .join("share")
                .join("lst")
                .join("sync.db");
            state.sync.database_path = Some(db_path);
            state_updated = true;
        }

        if state_updated {
            state
                .save()
                .context("Failed to save state with device info")?;
        }

        // Ensure encryption key is configured in sync settings
        if let Some(sync) = &config.sync {
            if sync.encryption_key_ref.is_none() {
                let mut updated_config = config.clone();
                if let Some(ref mut sync_cfg) = updated_config.sync {
                    sync_cfg.encryption_key_ref = Some("lst-master-key".to_string());
                }
                updated_config
                    .save()
                    .context("Failed to save updated config")?;
                config = updated_config;
            }
        }

        config
    };

    Ok(config)
}
