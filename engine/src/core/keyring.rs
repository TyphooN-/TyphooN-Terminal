//! System keyring integration for secure credential storage.
//!
//! Uses the OS-native credential store:
//! - Linux: Secret Service (GNOME Keyring / KDE Wallet) for compatibility with
//!   TyphooN's pre-keyring-4 credential locations.
//! - macOS: Keychain.
//! - Windows: Credential Manager.
//!
//! Service name: "typhoon-terminal"
//! Credentials stored: broker API keys, Finnhub key.

use std::sync::OnceLock;

use keyring_core::{Entry, Error};

const SERVICE: &str = "typhoon-terminal";

static KEYRING_INIT: OnceLock<Result<(), String>> = OnceLock::new();

fn ensure_keyring_store() -> Result<(), String> {
    if keyring_core::get_default_store().is_some() {
        return Ok(());
    }
    KEYRING_INIT
        .get_or_init(|| {
            #[cfg(any(target_os = "linux", target_os = "freebsd"))]
            {
                // keyring 4.x split the public Entry API into keyring-core and
                // backend crates. Use Secret Service directly to preserve the
                // credential namespace used by TyphooN's previous keyring 3.x
                // integration instead of silently switching Linux users to
                // kernel keyutils.
                dbus_secret_service_keyring_store::Store::new()
                    .map(|store| {
                        let store: std::sync::Arc<keyring_core::CredentialStore> = store;
                        keyring_core::set_default_store(store);
                    })
                    .map_err(|e| format!("Keyring Secret Service init error: {e}"))
            }
            #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
            {
                Err("Keyring 4.x backend is not configured for this platform".to_string())
            }
        })
        .clone()
}

fn entry_for(key: &str) -> Result<Entry, String> {
    ensure_keyring_store()?;
    Entry::new(SERVICE, key).map_err(|e| format!("Keyring entry error: {e}"))
}

/// Store a credential in the system keyring.
pub fn store(key: &str, value: &str) -> Result<(), String> {
    entry_for(key)?
        .set_password(value)
        .map_err(|e| format!("Keyring store error: {e}"))
}

/// Retrieve a credential from the system keyring.
pub fn load(key: &str) -> Result<Option<String>, String> {
    match entry_for(key)?.get_password() {
        Ok(val) => Ok(Some(val)),
        Err(Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Keyring load error: {e}")),
    }
}

/// Delete a credential from the system keyring.
pub fn delete(key: &str) -> Result<(), String> {
    match entry_for(key)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(Error::NoEntry) => Ok(()), // already gone
        Err(e) => Err(format!("Keyring delete error: {e}")),
    }
}

/// Standard credential keys used by TyphooN Terminal.
pub mod keys {
    pub const ALPACA_API_KEY: &str = "alpaca_api_key";
    pub const ALPACA_SECRET: &str = "alpaca_secret";
    pub const FINNHUB_KEY: &str = "finnhub_api_key";
    pub const FRED_KEY: &str = "fred_api_key";
    pub const DISCORD_WEBHOOK: &str = "discord_webhook";
    pub const PUSHOVER_TOKEN: &str = "pushover_token";
    pub const PUSHOVER_USER: &str = "pushover_user";
    pub const NTFY_TOPIC: &str = "ntfy_topic";
    pub const ANTHROPIC_KEY: &str = "anthropic_api_key";
    pub const OPENAI_KEY: &str = "openai_api_key";
    pub const KRAKEN_API_KEY: &str = "kraken_api_key";
    pub const KRAKEN_API_SECRET: &str = "kraken_api_secret";
    pub const KRAKEN_WS_API_KEY: &str = "kraken_ws_api_key";
    pub const KRAKEN_WS_API_SECRET: &str = "kraken_ws_api_secret";
    pub const GEMINI_KEY: &str = "gemini_api_key";
    pub const XAI_KEY: &str = "xai_api_key";
    pub const MISTRAL_KEY: &str = "mistral_api_key";
    pub const PERPLEXITY_KEY: &str = "perplexity_api_key";
    pub const MATRIX_ACCESS_TOKEN: &str = "matrix_access_token";
    pub const MATRIX_USER_ID: &str = "matrix_user_id";
    // News & research APIs (free tier)
    pub const FMP_KEY: &str = "fmp_api_key";
    pub const MARKETAUX_KEY: &str = "marketaux_api_key";
    pub const ALPHA_VANTAGE_KEY: &str = "alpha_vantage_api_key";
    pub const CRYPTOPANIC_KEY: &str = "cryptopanic_api_key";
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
        assert!(!keys::FRED_KEY.is_empty());
    }

    #[test]
    fn service_name() {
        assert_eq!(SERVICE, "typhoon-terminal");
    }

    // ── Integration: store/load/delete cycle ────────────────────────
    // Uses a unique test key to avoid colliding with real credentials.

    #[test]
    fn keyring4_sample_store_roundtrip_and_idempotent_delete() {
        let sample_store = keyring_core::sample::Store::new_with_configuration(
            &std::collections::HashMap::from([("persist", "false")]),
        )
        .expect("sample store should initialize");
        keyring_core::set_default_store(sample_store);
        let test_key = "typhoon_test_keyring4_sample_roundtrip";

        delete(test_key).expect("pre-clean delete should be idempotent");
        assert_eq!(load(test_key).expect("missing load should not error"), None);

        store(test_key, "value_1").expect("initial store should succeed");
        assert_eq!(
            load(test_key).expect("load after first store"),
            Some("value_1".to_string())
        );

        store(test_key, "value_2").expect("overwrite store should succeed");
        assert_eq!(
            load(test_key).expect("load after overwrite"),
            Some("value_2".to_string())
        );

        delete(test_key).expect("delete should succeed");
        assert_eq!(load(test_key).expect("load after delete"), None);
    }

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
    #[ignore] // requires a configured keyring backend under keyring 4.x
    fn load_nonexistent_returns_none() {
        let result = load("typhoon_test_nonexistent_key_xyz").expect("load should not error");
        assert_eq!(result, None);
    }

    #[test]
    #[ignore] // requires a configured keyring backend under keyring 4.x
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
