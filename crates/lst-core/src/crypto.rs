use anyhow::{anyhow, Context, Result};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, Key, XNonce};
use rand::RngCore;
use base64::{engine::general_purpose, Engine as _};
use std::fs;
use std::path::Path;
use argon2::{Argon2, password_hash::{PasswordHasher, SaltString}};
use sha2::{Sha256, Digest};

/// Derive secure encryption key from email + password + auth_token using Argon2
/// This ensures maximum security: user secret + identity + server token
/// Uses Argon2 for proper key derivation (secure, slow, memory-hard)
pub fn derive_key_from_credentials(email: &str, password: &str, auth_token: &str) -> Result<[u8; 32]> {
    // Create deterministic salt from email for consistency across devices
    let mut salt_hasher = Sha256::new();
    salt_hasher.update(b"lst-salt-v2:");
    salt_hasher.update(email.to_lowercase().as_bytes());
    let salt_hash = salt_hasher.finalize();
    
    // Use first 16 bytes as salt for Argon2
    let salt = SaltString::encode_b64(&salt_hash[..16])
        .map_err(|e| anyhow!("Failed to encode salt: {}", e))?;
    
    // Combine all three components for maximum security
    let mut combined_input = Vec::new();
    combined_input.extend_from_slice(password.as_bytes());
    combined_input.extend_from_slice(b":");
    combined_input.extend_from_slice(email.to_lowercase().as_bytes());
    combined_input.extend_from_slice(b":");
    combined_input.extend_from_slice(auth_token.as_bytes());
    
    // Use Argon2 to derive key from combined input
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(&combined_input, &salt)
        .map_err(|e| anyhow!("Argon2 key derivation failed: {}", e))?;
    
    // Extract 32 bytes for encryption key
    let hash_bytes = password_hash.hash.ok_or_else(|| anyhow!("No hash in password result"))?;
    if hash_bytes.len() < 32 {
        return Err(anyhow!("Derived hash too short"));
    }
    
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash_bytes.as_bytes()[..32]);
    
    println!("DEBUG: Derived SECURE encryption key using Argon2 (email: {}, password len: {}, token len: {})", 
             email, password.len(), auth_token.len());
    
    // Clear sensitive data from memory
    combined_input.fill(0);
    
    Ok(key)
}

/// Load a previously saved encryption key from disk
pub fn load_key(path: &Path) -> Result<[u8; 32]> {
    let expanded = if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            home.join(path.strip_prefix("~/").unwrap())
        } else {
            return Err(anyhow!("Cannot determine home directory"));
        }
    } else {
        path.to_path_buf()
    };

    if expanded.exists() {
        let data = fs::read(&expanded)
            .with_context(|| format!("Failed to read key file: {}", expanded.display()))?;
        let decoded = if data.len() == 32 {
            data
        } else {
            general_purpose::STANDARD.decode(&data)?
        };
        if decoded.len() != 32 {
            return Err(anyhow!("Invalid key length"));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&decoded);
        Ok(key)
    } else {
        return Err(anyhow!("No encryption key found at {}. Please run authentication first.", expanded.display()));
    }
}

/// Get the proper path for storing the master key based on platform
/// For desktop/CLI: ~/.local/share/lst/lst-master-key
/// For mobile: Use app data directory (platform-specific)
pub fn get_master_key_path() -> Result<std::path::PathBuf> {
    // For mobile platforms, we should use a different path
    // This function provides the default for desktop/CLI
    if let Some(data_dir) = dirs::data_dir() {
        let lst_data_dir = data_dir.join("lst");
        std::fs::create_dir_all(&lst_data_dir)
            .with_context(|| format!("Failed to create data directory: {}", lst_data_dir.display()))?;
        Ok(lst_data_dir.join("lst-master-key"))
    } else {
        // Fallback to home directory if data_dir is not available
        if let Some(home_dir) = dirs::home_dir() {
            let lst_data_dir = home_dir.join(".local").join("share").join("lst");
            std::fs::create_dir_all(&lst_data_dir)
                .with_context(|| format!("Failed to create data directory: {}", lst_data_dir.display()))?;
            Ok(lst_data_dir.join("lst-master-key"))
        } else {
            Err(anyhow!("Cannot determine data directory or home directory"))
        }
    }
}

/// Get the master key path for mobile apps using app-specific data directory
/// This should be used by mobile apps as they can't access ~/.local/share
pub fn get_mobile_master_key_path() -> Result<std::path::PathBuf> {
    if let Some(data_dir) = dirs::data_dir() {
        let app_data_dir = data_dir.join("lst-mobile");
        std::fs::create_dir_all(&app_data_dir)
            .with_context(|| format!("Failed to create app data directory: {}", app_data_dir.display()))?;
        Ok(app_data_dir.join("lst-master-key"))
    } else {
        // Fallback to temp if data_dir fails
        let app_data_dir = std::env::temp_dir().join("lst-mobile");
        std::fs::create_dir_all(&app_data_dir)
            .with_context(|| format!("Failed to create temp app data directory: {}", app_data_dir.display()))?;
        Ok(app_data_dir.join("lst-master-key"))
    }
}

/// Resolve the master key path from an encryption_key_ref string
/// If the ref is "lst-master-key", resolves to the proper platform-specific path
/// Otherwise, treats it as a literal path
pub fn resolve_key_path(encryption_key_ref: &str) -> Result<std::path::PathBuf> {
    if encryption_key_ref == "lst-master-key" {
        // Use platform-appropriate path
        get_master_key_path()
    } else {
        // Treat as literal path (supports existing behavior for custom paths)
        Ok(std::path::PathBuf::from(encryption_key_ref))
    }
}

/// Resolve the master key path for mobile apps from an encryption_key_ref string
pub fn resolve_mobile_key_path(encryption_key_ref: &str) -> Result<std::path::PathBuf> {
    if encryption_key_ref == "lst-master-key" {
        // Use mobile-specific path
        get_mobile_master_key_path()
    } else {
        // Treat as literal path (supports existing behavior for custom paths)
        Ok(std::path::PathBuf::from(encryption_key_ref))
    }
}

/// Save a derived key to the key file for consistency
pub fn save_derived_key(path: &Path, key: &[u8; 32]) -> Result<()> {
    let expanded = if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            home.join(path.strip_prefix("~/").unwrap())
        } else {
            return Err(anyhow!("Cannot determine home directory"));
        }
    } else {
        path.to_path_buf()
    };

    if let Some(parent) = expanded.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create key directory: {}", parent.display()))?;
    }
    fs::write(&expanded, general_purpose::STANDARD.encode(key))
        .with_context(|| format!("Failed to write key file: {}", expanded.display()))?;
    Ok(())
}

/// Encrypt data using XChaCha20-Poly1305.
/// The returned vector is nonce || ciphertext.
pub fn encrypt(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    let mut nonce = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut nonce);
    let ciphertext = cipher
        .encrypt(XNonce::from_slice(&nonce), data)
        .map_err(|e| anyhow!("Encryption failed: {e}"))?;
    let mut out = Vec::with_capacity(24 + ciphertext.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypt data previously encrypted with `encrypt`.
pub fn decrypt(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    if data.len() < 24 {
        return Err(anyhow!("Ciphertext too short"));
    }
    let (nonce, ciphertext) = data.split_at(24);
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    let plaintext = cipher
        .decrypt(XNonce::from_slice(nonce), ciphertext)
        .map_err(|e| anyhow!("Decryption failed: {e}"))?;
    Ok(plaintext)
}