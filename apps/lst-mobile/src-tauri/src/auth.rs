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
    password_hash: String, // Client-side hashed password (email-based salt)
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

/// Hash a password using Argon2id with a deterministic salt based on email
/// This ensures the same password+email combination always produces the same hash
pub fn hash_password_with_email(password: &str, email: &str) -> Result<String> {
    // Create deterministic salt from email for client-side hashing
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    use std::hash::Hasher;
    hasher.write(email.as_bytes());
    hasher.write(b"lst-client-salt"); // Add app-specific salt component
    let email_hash = hasher.finish();
    
    // Convert hash to 16-byte array for salt
    let salt_bytes = email_hash.to_le_bytes();
    let mut full_salt = [0u8; 16];
    full_salt[..8].copy_from_slice(&salt_bytes);
    full_salt[8..].copy_from_slice(&salt_bytes); // Repeat to fill 16 bytes
    
    let salt = SaltString::encode_b64(&full_salt).map_err(|e| anyhow::anyhow!("Failed to encode salt: {}", e))?;
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;
    Ok(password_hash.to_string())
}

/// Hash a password using Argon2id with random salt (for server-side storage)
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

/// Store JWT token in database
pub fn store_jwt_token_to_db(db: &crate::database::Database, email: &str, token: &str) -> Result<()> {
    db.save_sync_config("jwt_token", token)?;
    db.save_sync_config("jwt_email", email)?;
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);
    db.save_sync_config("jwt_expires_at", &expires_at.to_rfc3339())?;
    Ok(())
}

/// Retrieve JWT token from database
pub fn get_jwt_token_from_db(db: &crate::database::Database) -> Result<Option<(String, String)>> {
    let token = match db.load_sync_config("jwt_token")? {
        Some(t) => t,
        None => return Ok(None),
    };
    
    let email = match db.load_sync_config("jwt_email")? {
        Some(e) => e,
        None => return Ok(None),
    };
    
    // Check if token is expired
    if let Some(expires_str) = db.load_sync_config("jwt_expires_at")? {
        if let Ok(expires_at) = chrono::DateTime::parse_from_rfc3339(&expires_str) {
            if chrono::Utc::now() > expires_at.with_timezone(&chrono::Utc) {
                // Token is expired, clear it
                clear_jwt_token_from_db(db)?;
                return Ok(None);
            }
        }
    }
    
    Ok(Some((token, email)))
}

/// Clear JWT token from database
pub fn clear_jwt_token_from_db(db: &crate::database::Database) -> Result<()> {
    // Delete JWT-related keys from sync_config table
    let conn = db.pool.get()?;
    conn.execute("DELETE FROM sync_config WHERE key IN ('jwt_token', 'jwt_email', 'jwt_expires_at')", [])?;
    Ok(())
}

/// Store JWT token securely in system keychain (fallback/legacy)
pub fn store_jwt_token(email: &str, token: &str) -> Result<()> {
    let entry = Entry::new("lst-mobile", email).context("Failed to create keyring entry")?;
    entry.set_password(token).context("Failed to store JWT token")?;
    Ok(())
}

/// Retrieve JWT token from system keychain (fallback/legacy)
pub fn get_jwt_token(email: &str) -> Result<Option<String>> {
    let entry = Entry::new("lst-mobile", email).context("Failed to create keyring entry")?;
    match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow::anyhow!("Failed to retrieve JWT token: {}", e)),
    }
}

/// Clear JWT token from system keychain (fallback/legacy)
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

    // Hash password with email-based salt for secure transmission
    let password_to_hash = password.unwrap_or_else(|| "default_password".to_string());
    let client_password_hash = hash_password_with_email(&password_to_hash, &email)?;

    // Prepare request
    let auth_request = AuthRequest {
        email: email.clone(),
        host: server_url.clone(),
        password_hash: client_password_hash, // Client-side hashed password
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
pub async fn verify_auth_token_with_db(email: String, token: String, server_url: String, db: &crate::database::Database) -> Result<String> {
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

        // Store JWT token in database
        store_jwt_token_to_db(db, &email, &verify_response.jwt)?;
        
        // Update config with JWT token (expires in 1 hour by default)
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);
        update_config_with_jwt(verify_response.jwt, expires_at, server_url, email)?;
        
        Ok("Authentication successful! Sync is now enabled.".to_string())
    } else {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        Err(anyhow::anyhow!("Token verification failed: {}", error_text))
    }
}

/// Verify authentication token with server (legacy version without db parameter)
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
        update_config_with_jwt(verify_response.jwt, expires_at, server_url, email)?;
        
        Ok("Authentication successful! Sync is now enabled.".to_string())
    } else {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        Err(anyhow::anyhow!("Token verification failed: {}", error_text))
    }
}

/// Update config with JWT token and sync settings
fn update_config_with_jwt(jwt_token: String, expires_at: chrono::DateTime<chrono::Utc>, server_url: String, email: String) -> Result<()> {
    let mut config = CONFIG_MUTEX.lock().unwrap();
    config.store_jwt(jwt_token, expires_at);
    
    // Set up sync configuration using provided parameters instead of reading from storage
    if !server_url.is_empty() {
        // Generate a device ID if not already in storage
        let storage = MOBILE_SYNC_CONFIG.lock().unwrap();
        let device_id = storage.get("device_id").cloned()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        drop(storage); // Release the lock
        
        // Set up syncd configuration
        let mut syncd_config = lst_cli::config::SyncdConfig {
            url: Some(server_url.clone()),
            auth_token: None, // We use JWT instead
            device_id: Some(device_id.clone()),
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
        
        // Set up sync configuration with default values
        let sync_config = lst_cli::config::SyncSettings {
            interval_seconds: 30, // Default 30 seconds
            max_file_size: 10 * 1024 * 1024, // 10MB default
            exclude_patterns: vec![],
        };
        config.sync = Some(sync_config);
        
        // Also store the basic sync config in MOBILE_SYNC_CONFIG for later use
        let mut storage = MOBILE_SYNC_CONFIG.lock().unwrap();
        storage.insert("server_url".to_string(), server_url);
        storage.insert("email".to_string(), email);
        storage.insert("device_id".to_string(), device_id);
        storage.insert("sync_enabled".to_string(), "true".to_string());
        storage.insert("sync_interval".to_string(), "30".to_string());
    }
    
    // Skip saving config to file system on mobile - use in-memory storage instead
    // Mobile apps have restricted file system access and use SQLite database
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
    storage.insert("server_url".to_string(), server_url.clone());
    storage.insert("email".to_string(), email.clone());
    storage.insert("device_id".to_string(), device_id.clone());
    storage.insert("sync_enabled".to_string(), sync_enabled.to_string());
    storage.insert("sync_interval".to_string(), sync_interval.to_string());
    drop(storage); // Release the lock before updating CONFIG_MUTEX
    
    // Also update the CONFIG_MUTEX with sync configuration
    let mut config = CONFIG_MUTEX.lock().unwrap();
    
    // Set up syncd configuration
    let mut syncd_config = lst_cli::config::SyncdConfig {
        url: Some(server_url),
        auth_token: None, // We use JWT instead
        device_id: Some(device_id),
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
        interval_seconds: sync_interval as u64,
        max_file_size: 10 * 1024 * 1024, // 10MB default
        exclude_patterns: vec![],
    };
    config.sync = Some(sync_config);
    
    Ok(())
}

/// Get current config for sync manager
pub fn get_current_config() -> Config {
    let config_guard = CONFIG_MUTEX.lock().unwrap();
    config_guard.clone()
}

/// Initialize sync config from database on startup
pub fn initialize_sync_config_from_db(db: &crate::database::Database) -> Result<()> {
    let config_map = db.load_all_sync_config()?;
    
    let mut storage = MOBILE_SYNC_CONFIG.lock().unwrap();
    for (key, value) in config_map.iter() {
        storage.insert(key.clone(), value.clone());
    }
    
    // Load JWT token from database if available
    if let Ok(Some((jwt_token, jwt_email))) = get_jwt_token_from_db(db) {
        println!("Loading JWT token for user: {}", jwt_email);
        
        // Parse expiration time
        let expires_at = config_map.get("jwt_expires_at")
            .and_then(|expires_str| chrono::DateTime::parse_from_rfc3339(expires_str).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(1));
        
        // Update CONFIG_MUTEX with JWT token
        let mut config = CONFIG_MUTEX.lock().unwrap();
        config.store_jwt(jwt_token, expires_at);
        drop(config); // Release config lock
    } else {
        println!("No valid JWT token found in database");
    }
    
    // If we have sync config, also initialize CONFIG_MUTEX
    if let (Some(server_url), Some(device_id)) = (config_map.get("server_url"), config_map.get("device_id")) {
        drop(storage); // Release storage lock before acquiring CONFIG_MUTEX
        
        let mut config = CONFIG_MUTEX.lock().unwrap();
        
        // Set up syncd configuration
        let mut syncd_config = lst_cli::config::SyncdConfig {
            url: Some(server_url.clone()),
            auth_token: None, // We use JWT instead
            device_id: Some(device_id.clone()),
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
        let sync_interval = config_map.get("sync_interval")
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(30);
        
        let sync_config = lst_cli::config::SyncSettings {
            interval_seconds: sync_interval,
            max_file_size: 10 * 1024 * 1024, // 10MB default
            exclude_patterns: vec![],
        };
        config.sync = Some(sync_config);
    }
    
    Ok(())
}

/// Update sync configuration and persist to database
pub fn update_sync_config_with_db(
    db: &crate::database::Database,
    server_url: String,
    email: String,
    device_id: String,
    sync_enabled: bool,
    sync_interval: u32,
) -> Result<()> {
    // Store in database for persistence
    db.save_sync_config("server_url", &server_url)?;
    db.save_sync_config("email", &email)?;
    db.save_sync_config("device_id", &device_id)?;
    db.save_sync_config("sync_enabled", &sync_enabled.to_string())?;
    db.save_sync_config("sync_interval", &sync_interval.to_string())?;

    // Also store in mobile-specific in-memory storage for backwards compatibility
    let mut storage = MOBILE_SYNC_CONFIG.lock().unwrap();
    storage.insert("server_url".to_string(), server_url.clone());
    storage.insert("email".to_string(), email.clone());
    storage.insert("device_id".to_string(), device_id.clone());
    storage.insert("sync_enabled".to_string(), sync_enabled.to_string());
    storage.insert("sync_interval".to_string(), sync_interval.to_string());
    drop(storage); // Release the lock before updating CONFIG_MUTEX

    // Also update the CONFIG_MUTEX with sync configuration
    let mut config = CONFIG_MUTEX.lock().unwrap();
    
    // Set up syncd configuration
    let mut syncd_config = lst_cli::config::SyncdConfig {
        url: Some(server_url),
        auth_token: None, // We use JWT instead
        device_id: Some(device_id),
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
        interval_seconds: sync_interval as u64,
        max_file_size: 10 * 1024 * 1024, // 10MB default
        exclude_patterns: vec![],
    };
    config.sync = Some(sync_config);
    
    Ok(())
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