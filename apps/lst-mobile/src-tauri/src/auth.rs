use anyhow::{Context, Result};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use keyring::Entry;
use lst_cli::config::{get_config, Config};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref CONFIG_MUTEX: Mutex<Config> = Mutex::new(get_config().clone());
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

    let client = reqwest::Client::new();
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

    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/api/auth/verify", base_url))
        .json(&verify_request)
        .send()
        .await
        .context("Failed to send token verification request")?;

    if response.status().is_success() {
        let auth_response: AuthResponse = response
            .json()
            .await
            .context("Failed to parse authentication response")?;

        if auth_response.success {
            if let Some(jwt_token) = auth_response.jwt_token {
                // Store JWT token securely
                store_jwt_token(&email, &jwt_token)?;
                
                // Update config with JWT token (expires in 1 hour by default)
                let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);
                update_config_with_jwt(jwt_token.to_string(), expires_at)?;
                
                Ok("Authentication successful! Sync is now enabled.".to_string())
            } else {
                Err(anyhow::anyhow!("Server did not return JWT token"))
            }
        } else {
            Err(anyhow::anyhow!("Authentication failed: {}", auth_response.message))
        }
    } else {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        Err(anyhow::anyhow!("Token verification failed: {}", error_text))
    }
}

/// Update config with JWT token
fn update_config_with_jwt(jwt_token: String, expires_at: chrono::DateTime<chrono::Utc>) -> Result<()> {
    let mut config = CONFIG_MUTEX.lock().unwrap();
    config.store_jwt(jwt_token, expires_at);
    config.save().context("Failed to save config with JWT token")?;
    Ok(())
}

/// Update sync configuration
pub fn update_sync_config(
    server_url: String,
    _email: String,
    device_id: String,
    _sync_enabled: bool,
    sync_interval: u32,
) -> Result<()> {
    let mut config = CONFIG_MUTEX.lock().unwrap();
    
    // Initialize syncd config if not present
    if config.syncd.is_none() {
        drop(config);
        let mut temp_config = CONFIG_MUTEX.lock().unwrap();
        temp_config.init_syncd()?;
        config = temp_config;
    }
    
    // Update syncd configuration
    if let Some(ref mut syncd) = config.syncd {
        syncd.url = if server_url.is_empty() { None } else { Some(server_url) };
        syncd.device_id = Some(device_id);
    }
    
    // Update sync settings
    if let Some(ref mut sync) = config.sync {
        sync.interval_seconds = sync_interval as u64;
    }
    
    // Save configuration
    config.save().context("Failed to save sync configuration")?;
    
    Ok(())
}

/// Get current sync configuration
pub fn get_sync_config() -> Result<(String, String, String, bool, u32)> {
    let config = CONFIG_MUTEX.lock().unwrap();
    
    let server_url = config.syncd
        .as_ref()
        .and_then(|s| s.url.as_ref())
        .cloned()
        .unwrap_or_default();
    
    let device_id = config.syncd
        .as_ref()
        .and_then(|s| s.device_id.as_ref())
        .cloned()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    
    let sync_interval = config.sync
        .as_ref()
        .map(|s| s.interval_seconds as u32)
        .unwrap_or(30);
    
    let sync_enabled = !server_url.is_empty() && config.is_jwt_valid();
    
    // For email, we'll need to store it separately or extract from JWT
    // For now, return empty string
    let email = String::new();
    
    Ok((server_url, email, device_id, sync_enabled, sync_interval))
}