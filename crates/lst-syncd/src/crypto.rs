use anyhow::{anyhow, Context, Result};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, Key, XNonce};
use rand::RngCore;
use base64::{engine::general_purpose, Engine as _};
use std::fs;
use std::path::Path;

/// Load or create the encryption key stored at `path`.
/// If the file does not exist, a new random key is generated and written in
/// base64 form.
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

