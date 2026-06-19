//! Extracted from app.rs: platform helpers.

use super::*;

// ─── platform helper ─────────────────────────────────────────────────────────

pub(crate) fn dirs_home() -> PathBuf {
    let mut p = if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
    } else {
        PathBuf::from("/tmp")
    };
    p.push(".config");
    p.push("typhoon-terminal");
    p
}

/// Optional user-configured cache directory. Populated at startup from
/// `~/.config/typhoon-terminal/cache_location.txt` so `cache_dir()` and
/// `cache_db_path()` can redirect the heavy SQLite blob onto a NAS mount
/// or a faster/larger drive while session.json + keyring stay local.
/// `None` = the default `~/.config/typhoon-terminal/cache/` location.
pub(crate) static CUSTOM_CACHE_DIR: std::sync::OnceLock<Option<PathBuf>> =
    std::sync::OnceLock::new();

/// Where the custom-cache-dir setting is persisted. This file is read BEFORE
/// session.json is loaded (since the cache itself needs to be opened early),
/// so it lives in a fixed local location regardless of where the cache is.
pub fn cache_location_file() -> PathBuf {
    let mut p = dirs_home();
    p.push("cache_location.txt");
    p
}

/// Read the configured custom cache dir from `cache_location_file`. Returns
/// `None` when the file is missing, empty, or unparseable. Called once at
/// startup (main.rs) and used to seed `CUSTOM_CACHE_DIR`.
pub fn read_custom_cache_dir() -> Option<PathBuf> {
    let path = cache_location_file();
    let s = std::fs::read_to_string(&path).ok()?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    let pb = PathBuf::from(trimmed);
    if pb.is_absolute() { Some(pb) } else { None }
}

/// Persist the custom cache dir. `None` clears the override by deleting the
/// file — the next startup reverts to the default location.
pub fn write_custom_cache_dir(dir: Option<&std::path::Path>) -> std::io::Result<()> {
    let path = cache_location_file();
    match dir {
        Some(p) => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, p.display().to_string())
        }
        None => {
            if path.exists() {
                std::fs::remove_file(&path)
            } else {
                Ok(())
            }
        }
    }
}

/// Seed the process-wide cache-dir override. Idempotent after the first call
/// (OnceLock). Main.rs calls this before the cache is opened.
pub fn init_custom_cache_dir(dir: Option<PathBuf>) {
    let _ = CUSTOM_CACHE_DIR.set(dir);
}

/// Directory that holds `typhoon_cache.db`. Respects the custom override if
/// it was set at startup AND the target directory exists; otherwise falls
/// back to the default `dirs_home()/cache`. Callers that need to know which
/// is in effect can compare against `dirs_home().join("cache")`.
pub fn cache_dir() -> PathBuf {
    if let Some(Some(custom)) = CUSTOM_CACHE_DIR.get() {
        // Defensive: if the custom dir disappeared (unmounted NAS, removed
        // drive), fall back to the default so the app still starts instead
        // of hard-erroring. The UI shows a warning banner; `read_custom_cache_dir`
        // is still the source of truth for "what the user configured".
        if custom.is_dir() {
            return custom.clone();
        }
    }
    let mut p = dirs_home();
    p.push("cache");
    p
}

/// Full path to the SQLite cache database.
pub fn cache_db_path() -> PathBuf {
    cache_dir().join("typhoon_cache.db")
}
