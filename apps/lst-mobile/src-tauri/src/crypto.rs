use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use rand::RngCore;
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
    
    println!("📱 DEBUG: Derived SECURE encryption key using Argon2 (email: {}, password len: {}, token len: {})", 
             email, password.len(), auth_token.len());
    
    // Clear sensitive data from memory
    combined_input.fill(0);
    
    Ok(key)
}

/// Load or derive encryption key using secure credentials (email + password + auth_token)
pub fn load_or_derive_key_from_credentials(
    path: &Path, 
    email: Option<&str>, 
    password: Option<&str>, 
    auth_token: Option<&str>
) -> Result<[u8; 32]> {
    // All three components required for security
    if let (Some(email), Some(password), Some(token)) = (email, password, auth_token) {
        match derive_key_from_credentials(email, password, token) {
            Ok(derived_key) => {
                // Save the derived key to the file for consistency
                if let Err(e) = save_derived_key(path, &derived_key) {
                    println!("📱 Warning: Failed to save derived key to {}: {}", path.display(), e);
                }
                
                println!("📱 DEBUG: Using SECURE Argon2-derived encryption key from credentials");
                return Ok(derived_key);
            }
            Err(e) => {
                println!("📱 ERROR: Secure key derivation failed: {}", e);
                return Err(e);
            }
        }
    }
    
    // Missing required credentials
    println!("📱 ERROR: Email, password, and auth token all required for key derivation.");
    println!("📱        Please register/login with proper credentials");
    Err(anyhow!("Complete authentication required for encryption key"))
}

/// Legacy function - now requires all credentials for security
pub fn load_or_derive_key(path: &Path, auth_token: Option<&str>) -> Result<[u8; 32]> {
    println!("📱 WARNING: Using legacy key derivation function without email/password!");
    println!("📱          Please use load_or_derive_key_from_credentials for proper security");
    
    if auth_token.is_none() {
        return Err(anyhow!("Auth token required for legacy key derivation"));
    }
    
    // For now, fallback to file-based key to avoid breaking existing code
    // This should be removed once all callers are updated
    load_key(path)
}

/// Load or create the encryption key stored at `path` (DEPRECATED - use auth token derivation)
/// If the file does not exist, a new random key is generated and written in base64 form.
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
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        if let Some(parent) = expanded.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create key directory: {}", parent.display()))?;
        }
        fs::write(&expanded, general_purpose::STANDARD.encode(key))
            .with_context(|| format!("Failed to write key file: {}", expanded.display()))?;
        Ok(key)
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