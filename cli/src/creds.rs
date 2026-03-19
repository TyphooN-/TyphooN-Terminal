//! Load encrypted credentials from the GUI terminal's SQLite database.
//!
//! The GUI stores credentials as AES-256-GCM encrypted JSON in SQLite.
//! Key derivation: PBKDF2-HMAC-SHA256(hostname+username, salt_file, 100K iterations).
//! This module replicates the decryption so CLI can share the same credentials.

use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead};
use aes_gcm::aead::generic_array::GenericArray;
use std::path::PathBuf;

const PBKDF2_ITERATIONS: u32 = 100_000;

/// Get the config directory path (~/.config/typhoon-terminal/)
fn config_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".config").join("typhoon-terminal")
}

/// Read the persistent salt file
fn read_salt() -> Option<[u8; 32]> {
    let salt_path = config_dir().join(".cred_salt");
    let bytes = std::fs::read(salt_path).ok()?;
    if bytes.len() != 32 { return None; }
    let mut salt = [0u8; 32];
    salt.copy_from_slice(&bytes);
    Some(salt)
}

/// Derive the same AES-256 key as the GUI terminal
fn derive_key() -> Option<[u8; 32]> {
    let salt = read_salt()?;
    let hostname = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .unwrap_or_else(|_| "typhoon".to_string());
    let username = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "default".to_string());

    let password = format!("{hostname}:{username}:typhoon-terminal");
    let mut key = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(password.as_bytes(), &salt, PBKDF2_ITERATIONS, &mut key);
    Some(key)
}

/// Decrypt AES-256-GCM ciphertext (nonce prepended to ciphertext, base64 encoded)
fn decrypt(encrypted_b64: &str, key: &[u8; 32]) -> Option<String> {
    let data = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encrypted_b64).ok()?;
    if data.len() < 12 { return None; } // nonce is 12 bytes

    let (nonce_bytes, ciphertext) = data.split_at(12);
    let nonce = GenericArray::from_slice(nonce_bytes);
    let cipher = Aes256Gcm::new(GenericArray::from_slice(key));

    let plaintext = cipher.decrypt(nonce, ciphertext).ok()?;
    String::from_utf8(plaintext).ok()
}

/// Load saved credentials from the GUI terminal's SQLite database.
/// Returns (api_key, secret_key, account_name) if found.
pub fn load_saved_credentials(paper: bool) -> Option<(String, String, String)> {
    let key = derive_key()?;
    let db_path = config_dir().join("cache").join("typhoon_cache.db");
    if !db_path.exists() { return None; }

    let conn = rusqlite::Connection::open(&db_path).ok()?;

    // Look for saved accounts in kv_cache table
    // Keys are like "credential:AccountName" with encrypted JSON value
    let mut stmt = conn.prepare(
        "SELECT key, value FROM kv_cache WHERE key LIKE 'credential:%'"
    ).ok()?;

    let rows: Vec<(String, String)> = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }).ok()?.filter_map(|r| r.ok()).collect();

    for (db_key, encrypted_value) in &rows {
        // Try to decrypt
        if let Some(json_str) = decrypt(encrypted_value, &key) {
            // Parse JSON: { "apiKey": "...", "secretKey": "...", "paper": true/false }
            if let Ok(cred) = serde_json::from_str::<serde_json::Value>(&json_str) {
                let is_paper = cred["paper"].as_bool().unwrap_or(true);
                if is_paper != paper { continue; } // skip wrong mode

                let api_key = cred["apiKey"].as_str().or_else(|| cred["api_key"].as_str())?;
                let secret_key = cred["secretKey"].as_str().or_else(|| cred["secret_key"].as_str())?;
                let name = db_key.strip_prefix("credential:").unwrap_or(db_key);
                return Some((api_key.to_string(), secret_key.to_string(), name.to_string()));
            }
        }

        // If decryption fails, the value might be unencrypted JSON (legacy)
        if let Ok(cred) = serde_json::from_str::<serde_json::Value>(&encrypted_value) {
            let is_paper = cred["paper"].as_bool().unwrap_or(true);
            if is_paper != paper { continue; }

            if let (Some(ak), Some(sk)) = (
                cred["apiKey"].as_str().or_else(|| cred["api_key"].as_str()),
                cred["secretKey"].as_str().or_else(|| cred["secret_key"].as_str()),
            ) {
                let name = db_key.strip_prefix("credential:").unwrap_or(db_key);
                return Some((ak.to_string(), sk.to_string(), name.to_string()));
            }
        }
    }

    None
}
