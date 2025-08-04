use anyhow::{Context, Result};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use keyring::Entry;
use lst_cli::config::Config;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::collections::HashMap;
use lazy_static::lazy_static;

// Mobile-specific in-memory storage for sync config
lazy_static! {
    static ref MOBILE_SYNC_CONFIG: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
    static ref CONFIG_MUTEX: Mutex<Config> = Mutex::new(Config::default());
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthRequest {
    email: String,
    host: String,
    password_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthVerifyRequest {
    email: String,
    token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthResponse {
    success: bool,
    message: String,
    jwt_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VerifyResponse {
    jwt: String,
    user: String,
}

/// Hash a password using Argon2id
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;
    Ok(password_hash.to_string())
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(hash).map_err(|e| anyhow::anyhow!("Invalid password hash: {}", e))?;
    let argon2 = Argon2::default();
    Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
}

/// Store JWT token securely in system keychain
pub fn store_jwt_token(email: &str, token: &str) -> Result<()> {
    let entry = Entry::new("lst-mobile", email).context("Failed to create keyring entry")?;
    entry.set_password(token).context("Failed to store JWT token")?;
    Ok(())
}

/// Retrieve JWT token from system keychain
pub fn get_jwt_token(email: &str) -> Result<Option<String>> {
    let entry = Entry::new("lst-mobile", email).context("Failed to create keyring entry")?;
    match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow::anyhow!("Failed to retrieve JWT token: {}", e)),
    }
}

/// Clear JWT token from system keychain
pub fn clear_jwt_token(email: &str) -> Result<()> {
    let entry = Entry::new("lst-mobile", email).context("Failed to create keyring entry")?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()), // Already cleared
        Err(e) => Err(anyhow::anyhow!("Failed to clear JWT token: {}", e)),
    }
}

/// Request authentication token from server
pub async fn request_auth_token(email: String, server_url: String, password: Option<String>) -> Result<String> {
    // Validate inputs
    if email.is_empty() || !email.contains('@') {
        return Err(anyhow::anyhow!("Invalid email address"));
    }
    
    if server_url.is_empty() {
        return Err(anyhow::anyhow!("Server URL is required"));
    }

    // Hash password if provided (for now, we'll use a default password)
    let password_hash = if let Some(pwd) = password {
        hash_password(&pwd)?
    } else {
        // For demo purposes, use a default password hash
        hash_password("default_password")?
    };

    // Prepare request
    let auth_request = AuthRequest {
        email: email.clone(),
        host: server_url.clone(),
        password_hash,
    };

    // Extract base URL from WebSocket URL
    let base_url = server_url
        .replace("ws://", "http://")
        .replace("wss://", "https://")
        .replace("/api/sync", "");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;
    let response = client
        .post(&format!("{}/api/auth/request", base_url))
        .json(&auth_request)
        .send()
        .await
        .context("Failed to send authentication request")?;

    if response.status().is_success() {
        Ok("Authentication token has been sent to your email. Please check your inbox.".to_string())
    } else {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        Err(anyhow::anyhow!("Authentication request failed: {}", error_text))
    }
}

/// Verify authentication token with server
pub async fn verify_auth_token(email: String, token: String, server_url: String) -> Result<String> {
    // Validate inputs
    if token.is_empty() || token.len() < 4 {
        return Err(anyhow::anyhow!("Invalid token format"));
    }

    // Prepare request
    let verify_request = AuthVerifyRequest {
        email: email.clone(),
        token: token.clone(),
    };

    // Extract base URL from WebSocket URL
    let base_url = server_url
        .replace("ws://", "http://")
        .replace("wss://", "https://")
        .replace("/api/sync", "");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;
    let response = client
        .post(&format!("{}/api/auth/verify", base_url))
        .json(&verify_request)
        .send()
        .await
        .context("Failed to send token verification request")?;

    if response.status().is_success() {
        // Parse the server's VerifyResponse format
        let verify_response: VerifyResponse = response
            .json()
            .await
            .context("Failed to parse authentication response")?;

        // Store JWT token securely
        store_jwt_token(&email, &verify_response.jwt)?;
        
        // Update config with JWT token (expires in 1 hour by default)
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);
        update_config_with_jwt(verify_response.jwt, expires_at)?;
        
        Ok("Authentication successful! Sync is now enabled.".to_string())
    } else {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        Err(anyhow::anyhow!("Token verification failed: {}", error_text))
    }
}

/// Update config with JWT token and sync settings
fn update_config_with_jwt(jwt_token: String, expires_at: chrono::DateTime<chrono::Utc>) -> Result<()> {
    let mut config = CONFIG_MUTEX.lock().unwrap();
    config.store_jwt(jwt_token, expires_at);
    
    // Also update the sync configuration from mobile storage
    let storage = MOBILE_SYNC_CONFIG.lock().unwrap();
    if let Some(server_url) = storage.get("server_url") {
        // Set up syncd configuration
        let mut syncd_config = lst_cli::config::SyncdConfig {
            url: Some(server_url.clone()),
            auth_token: None, // We use JWT instead
            device_id: storage.get("device_id").cloned(),
            database_path: None,
            encryption_key_ref: None,
        };
        
        // Set up basic paths for mobile
        let app_data_dir = std::env::temp_dir().join("lst-mobile");
        std::fs::create_dir_all(&app_data_dir).ok();
        
        syncd_config.database_path = Some(app_data_dir.join("sync.db"));
        
        // Create a simple encryption key if it doesn't exist
        let key_path = app_data_dir.join("sync.key");
        if !key_path.exists() {
            // Generate a simple 32-byte key for demo purposes
            let key = (0..32).map(|i| (i as u8).wrapping_mul(7).wrapping_add(42)).collect::<Vec<u8>>();
            std::fs::write(&key_path, key).ok();
        }
        syncd_config.encryption_key_ref = Some(key_path.to_string_lossy().to_string());
        
        config.syncd = Some(syncd_config);
        
        // Set up sync configuration
        let sync_config = lst_cli::config::SyncSettings {
            interval_seconds: storage.get("sync_interval")
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            max_file_size: 10 * 1024 * 1024, // 10MB default
            exclude_patterns: vec![],
        };
        config.sync = Some(sync_config);
    }
    
    config.save().context("Failed to save config with JWT token")?;
    Ok(())
}

/// Update sync configuration
pub fn update_sync_config(
    server_url: String,
    email: String,
    device_id: String,
    sync_enabled: bool,
    sync_interval: u32,
) -> Result<()> {
    // Store in mobile-specific in-memory storage
    let mut storage = MOBILE_SYNC_CONFIG.lock().unwrap();
    storage.insert("server_url".to_string(), server_url);
    storage.insert("email".to_string(), email);
    storage.insert("device_id".to_string(), device_id);
    storage.insert("sync_enabled".to_string(), sync_enabled.to_string());
    storage.insert("sync_interval".to_string(), sync_interval.to_string());
    
    Ok(())
}

/// Get current config for sync manager
pub fn get_current_config() -> Config {
    let config_guard = CONFIG_MUTEX.lock().unwrap();
    config_guard.clone()
}

/// Get current sync configuration
pub fn get_sync_config() -> Result<(String, String, String, bool, u32)> {
    let storage = MOBILE_SYNC_CONFIG.lock().unwrap();
    
    let server_url = storage.get("server_url").cloned().unwrap_or_default();
    let email = storage.get("email").cloned().unwrap_or_default();
    let device_id = storage.get("device_id").cloned().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    
    let sync_interval = storage.get("sync_interval")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(30);
    
    let sync_enabled = storage.get("sync_enabled")
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false);
    
    Ok((server_url, email, device_id, sync_enabled, sync_interval))
}