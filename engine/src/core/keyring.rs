//! System keyring integration for secure credential storage.
//!
//! Uses the OS-native credential store:
//! - Linux: libsecret (GNOME Keyring / KDE Wallet)
//! - macOS: Keychain
//! - Windows: Credential Manager
//!
//! Service name: "typhoon-terminal"
//! Credentials stored: broker API keys, Finnhub key, tastytrade creds

const SERVICE: &str = "typhoon-terminal";

/// Store a credential in the system keyring.
pub fn store(key: &str, value: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE, key)
        .map_err(|e| format!("Keyring entry error: {e}"))?;
    entry.set_password(value)
        .map_err(|e| format!("Keyring store error: {e}"))
}

/// Retrieve a credential from the system keyring.
pub fn load(key: &str) -> Result<Option<String>, String> {
    let entry = keyring::Entry::new(SERVICE, key)
        .map_err(|e| format!("Keyring entry error: {e}"))?;
    match entry.get_password() {
        Ok(val) => Ok(Some(val)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Keyring load error: {e}")),
    }
}

/// Delete a credential from the system keyring.
pub fn delete(key: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE, key)
        .map_err(|e| format!("Keyring entry error: {e}"))?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()), // already gone
        Err(e) => Err(format!("Keyring delete error: {e}")),
    }
}

/// Standard credential keys used by TyphooN Terminal.
pub mod keys {
    pub const ALPACA_API_KEY: &str = "alpaca_api_key";
    pub const ALPACA_SECRET: &str = "alpaca_secret";
    pub const FINNHUB_KEY: &str = "finnhub_api_key";
    pub const TT_USERNAME: &str = "tastytrade_username";
    pub const TT_PASSWORD: &str = "tastytrade_password";
    pub const FRED_KEY: &str = "fred_api_key";
    pub const LAN_SYNC_PASS: &str = "lan_sync_passphrase";
    pub const DISCORD_WEBHOOK: &str = "discord_webhook";
    pub const PUSHOVER_TOKEN: &str = "pushover_token";
    pub const PUSHOVER_USER: &str = "pushover_user";
    pub const NTFY_TOPIC: &str = "ntfy_topic";
    pub const ANTHROPIC_KEY: &str = "anthropic_api_key";
    pub const OPENAI_KEY: &str = "openai_api_key";
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Key constants ───────────────────────────────────────────────

    #[test]
    fn key_constants_non_empty() {
        assert!(!keys::ALPACA_API_KEY.is_empty());
        assert!(!keys::ALPACA_SECRET.is_empty());
        assert!(!keys::FINNHUB_KEY.is_empty());
        assert!(!keys::TT_USERNAME.is_empty());
        assert!(!keys::TT_PASSWORD.is_empty());
        assert!(!keys::FRED_KEY.is_empty());
        assert!(!keys::LAN_SYNC_PASS.is_empty());
    }

    #[test]
    fn service_name() {
        assert_eq!(SERVICE, "typhoon-terminal");
    }

    // ── Integration: store/load/delete cycle ────────────────────────
    // Uses a unique test key to avoid colliding with real credentials.

    #[test]
    #[ignore] // requires unlocked secret service (GNOME Keyring / KDE Wallet)
    fn store_load_delete_roundtrip() {
        let test_key = "typhoon_test_unit_keyring_roundtrip";
        let test_value = "s3cret_test_value_12345";

        // Store
        store(test_key, test_value).expect("store should succeed");

        // Load
        let loaded = load(test_key).expect("load should succeed");
        assert_eq!(loaded, Some(test_value.to_string()));

        // Delete
        delete(test_key).expect("delete should succeed");

        // Verify gone
        let after_delete = load(test_key).expect("load after delete should succeed");
        assert_eq!(after_delete, None);
    }

    #[test]
    fn load_nonexistent_returns_none() {
        let result = load("typhoon_test_nonexistent_key_xyz").expect("load should not error");
        assert_eq!(result, None);
    }

    #[test]
    fn delete_nonexistent_is_ok() {
        // Deleting a key that doesn't exist should succeed (idempotent)
        delete("typhoon_test_nonexistent_key_xyz").expect("delete nonexistent should be ok");
    }

    #[test]
    #[ignore] // requires unlocked secret service (GNOME Keyring / KDE Wallet)
    fn store_overwrites_existing() {
        let test_key = "typhoon_test_overwrite_key";

        store(test_key, "value_1").expect("first store");
        store(test_key, "value_2").expect("overwrite store");

        let loaded = load(test_key).expect("load").expect("should exist");
        assert_eq!(loaded, "value_2");

        // Cleanup
        delete(test_key).expect("cleanup");
    }
}
