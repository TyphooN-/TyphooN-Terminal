//! Load encrypted credentials from the GUI terminal's SQLite database.
//!
//! Storage format: JSON → AES-256-GCM encrypt → base64 → zstd compress → SQLite BLOB.
//! Key derivation: PBKDF2-HMAC-SHA256("TyphooN-Terminal-v2-{host}-{user}-credential-key", salt_file, 100K).
//! DB key format: "cred:{account_name}" in kv_cache table.

use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead};
use std::path::PathBuf;

const PBKDF2_ITERATIONS: u32 = 100_000;

fn config_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".config").join("typhoon-terminal")
}

fn read_salt() -> Option<[u8; 32]> {
    let bytes = std::fs::read(config_dir().join(".cred_salt")).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let mut salt = [0u8; 32];
    salt.copy_from_slice(&bytes);
    Some(salt)
}

fn derive_key() -> Option<[u8; 32]> {
    let salt = read_salt()?;
    let hostname = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .unwrap_or_else(|_| "typhoon".to_string());
    let username = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "default".to_string());
    let password = format!("TyphooN-Terminal-v2-{hostname}-{username}-credential-key");
    let mut key = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(password.as_bytes(), &salt, PBKDF2_ITERATIONS, &mut key);
    Some(key)
}

fn decrypt_aes(encrypted_b64: &str, key: &[u8; 32]) -> Option<String> {
    let data =
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encrypted_b64).ok()?;
    if data.len() < 13 {
        return None;
    }
    let (nonce_bytes, ciphertext) = data.split_at(12);
    let cipher = Aes256Gcm::new(GenericArray::from_slice(key));
    let plaintext = cipher
        .decrypt(GenericArray::from_slice(nonce_bytes), ciphertext)
        .ok()?;
    String::from_utf8(plaintext).ok()
}

/// Load saved credentials from GUI terminal's SQLite kv_cache.
/// Returns (api_key, secret_key, account_name).
pub fn load_saved_credentials(_paper: bool) -> Option<(String, String, String)> {
    let key = derive_key()?;
    let db_path = config_dir().join("cache").join("typhoon_cache.db");
    if !db_path.exists() {
        return None;
    }

    let conn = rusqlite::Connection::open(&db_path).ok()?;

    // Query all credential entries (stored as zstd-compressed BLOBs)
    let mut stmt = conn
        .prepare("SELECT key, value FROM kv_cache WHERE key LIKE 'cred:%'")
        .ok()?;

    let rows: Vec<(String, Vec<u8>)> = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
        })
        .ok()?
        .filter_map(|r| r.ok())
        .collect();

    for (db_key, compressed_blob) in &rows {
        let name = db_key.strip_prefix("cred:").unwrap_or(db_key);

        // Step 1: zstd decompress the BLOB → base64 string
        let decompressed = match zstd::decode_all(compressed_blob.as_slice()) {
            Ok(d) => d,
            Err(_) => compressed_blob.clone(), // maybe not compressed
        };
        let b64_string = match String::from_utf8(decompressed) {
            Ok(s) => s,
            Err(_) => continue,
        };

        // Step 2: AES-256-GCM decrypt the base64 → JSON
        if let Some(json_str) = decrypt_aes(&b64_string, &key) {
            if let Ok(cred) = serde_json::from_str::<serde_json::Value>(&json_str) {
                if let (Some(ak), Some(sk)) = (cred["apiKey"].as_str(), cred["secretKey"].as_str())
                {
                    return Some((ak.to_string(), sk.to_string(), name.to_string()));
                }
            }
        }
    }

    None
}
