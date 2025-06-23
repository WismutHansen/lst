use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use rand::RngCore;
use sodiumoxide::crypto::{box_, sealedbox};
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

/// Generate a device keypair using libsodium.
pub fn generate_keypair() -> (box_::PublicKey, box_::SecretKey) {
    box_::gen_keypair()
}

/// Load a keypair from the given base path ("{path}.pub" and "{path}.sec").
/// If the files are missing, a new pair is generated and stored as base64.
pub fn load_or_create_keypair(base: &Path) -> Result<(box_::PublicKey, box_::SecretKey)> {
    let pub_path = base.with_extension("pub");
    let sec_path = base.with_extension("sec");

    if pub_path.exists() && sec_path.exists() {
        let pub_bytes = general_purpose::STANDARD.decode(fs::read(&pub_path)?)?;
        let sec_bytes = general_purpose::STANDARD.decode(fs::read(&sec_path)?)?;
        return Ok((
            box_::PublicKey::from_slice(&pub_bytes).ok_or_else(|| anyhow!("Invalid public key"))?,
            box_::SecretKey::from_slice(&sec_bytes).ok_or_else(|| anyhow!("Invalid secret key"))?,
        ));
    }

    let (pk, sk) = generate_keypair();
    if let Some(parent) = pub_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&pub_path, general_purpose::STANDARD.encode(pk.as_ref()))?;
    fs::write(&sec_path, general_purpose::STANDARD.encode(sk.as_ref()))?;
    Ok((pk, sk))
}

/// Encrypt a message with the recipient's public key using sealed boxes.
pub fn seal_for(pk: &box_::PublicKey, msg: &[u8]) -> Vec<u8> {
    sealedbox::seal(msg, pk)
}

/// Decrypt a sealed box message with our keypair.
pub fn open_sealed(pk: &box_::PublicKey, sk: &box_::SecretKey, data: &[u8]) -> Result<Vec<u8>> {
    sealedbox::open(data, pk, sk).map_err(|_| anyhow!("Failed to decrypt sealed box"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn encrypt_roundtrip() {
        let key = [42u8; 32];
        let msg = b"hello";
        let ct = encrypt(msg, &key).unwrap();
        let pt = decrypt(&ct, &key).unwrap();
        assert_eq!(pt, msg);
    }

    #[test]
    fn sealed_box_roundtrip() {
        let (pk, sk) = generate_keypair();
        let msg = b"secret";
        let sealed = seal_for(&pk, msg);
        let opened = open_sealed(&pk, &sk, &sealed).unwrap();
        assert_eq!(opened, msg);
    }

    #[test]
    fn load_or_create_keypair_roundtrip() {
        let dir = tempdir().unwrap();
        let base = dir.path().join("device_key");
        let (pk1, sk1) = load_or_create_keypair(&base).unwrap();
        assert!(base.with_extension("pub").exists());
        let (pk2, sk2) = load_or_create_keypair(&base).unwrap();
        assert_eq!(pk1.as_ref(), pk2.as_ref());
        assert_eq!(sk1.as_ref(), sk2.as_ref());
    }
}
