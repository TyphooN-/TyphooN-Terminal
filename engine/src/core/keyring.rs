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
}
