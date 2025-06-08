pub use lst_cli::config::Config;
use anyhow::{Context, Result};
use std::{fs, path::Path};

/// Load syncd configuration from the unified lst config
pub fn load_syncd_config(path: &Path) -> Result<Config> {
    let config = if !path.exists() {
        // Create default config with syncd enabled
        let mut default_config = Config::default();
        default_config.init_syncd()?;
        
        // Ensure config directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }
        
        // Write default config
        default_config.save()
            .context("Failed to save default config with syncd settings")?;
        
        println!("Created default config with sync daemon settings at: {}", path.display());
        default_config
    } else {
        let mut config = Config::load_from(path)?;
        
        // Initialize syncd if not present
        if config.syncd.is_none() {
            config.init_syncd()?;
            config.save()
                .context("Failed to save config with syncd settings")?;
            println!("Added sync daemon settings to existing config");
        }
        
        // Ensure required syncd fields are present
        if let Some(syncd) = &config.syncd {
            if syncd.device_id.is_none() || syncd.database_path.is_none() || syncd.encryption_key_ref.is_none() {
                let mut updated_config = config.clone();
                if let Some(ref mut syncd_cfg) = updated_config.syncd {
                    if syncd_cfg.device_id.is_none() {
                        let device_id = uuid::Uuid::new_v4().to_string();
                        syncd_cfg.device_id = Some(device_id.clone());
                        println!("Generated new device_id: {}", device_id);
                    }
                    if syncd_cfg.database_path.is_none() {
                        let db_path = dirs::config_dir()
                            .context("Cannot determine config directory")?
                            .join("lst")
                            .join("syncd.db");
                        syncd_cfg.database_path = Some(db_path);
                    }
                    if syncd_cfg.encryption_key_ref.is_none() {
                        syncd_cfg.encryption_key_ref = Some("lst-master-key".to_string());
                    }
                }
                updated_config.save()
                    .context("Failed to save updated syncd config")?;
                return Ok(updated_config);
            }
        }
        
        config
    };
    
    Ok(config)
}

