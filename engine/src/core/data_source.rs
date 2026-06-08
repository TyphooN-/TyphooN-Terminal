//! Pluggable data source hierarchy (ADR-038 Phase 2).
//!
//! Formalizes the cache key prefix routing
//! (mt5: > kraken: > kraken-futures: > cryptocompare: > tastytrade: > alpaca:)
//! into a trait-based system with per-source health tracking, per-symbol overrides,
//! and configurable priority ordering.

use serde::{Deserialize, Serialize};

/// Unique identifier for a data source.
pub type SourceId = String;

/// A registered data source with metadata and health state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataSourceEntry {
    /// Unique identifier (e.g. "mt5-darwinex", "alpaca-paper", "kraken")
    pub id: SourceId,
    /// Cache key prefix (e.g. "mt5", "alpaca", "cryptocompare", "kraken")
    pub cache_prefix: String,
    /// Human-readable label (e.g. "MT5 (Darwinex)", "Alpaca (Paper)")
    pub label: String,
    /// Priority (lower = higher priority). Used for default ordering.
    pub priority: u32,
    /// Whether this source is currently healthy (connected, syncing).
    #[serde(skip)]
    pub healthy: bool,
    /// Timestamp of last successful data delivery (epoch seconds).
    #[serde(skip)]
    pub last_success_ts: i64,
    /// Asset classes this source supports (empty = all).
    /// Examples: "forex", "equity", "crypto", "cfd"
    pub asset_classes: Vec<String>,
}

/// Per-symbol routing override.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SymbolOverride {
    /// Symbol pattern (exact match or prefix with `*` wildcard).
    pub pattern: String,
    /// Ordered source IDs to try for this symbol.
    pub sources: Vec<SourceId>,
}

/// Manages data source priority, health, and per-symbol routing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataSourceManager {
    /// Registered data sources in default priority order.
    pub sources: Vec<DataSourceEntry>,
    /// Per-symbol routing overrides (checked before default priority).
    pub overrides: Vec<SymbolOverride>,
    /// Health timeout: source marked unhealthy after this many seconds without data.
    pub health_timeout_secs: i64,
}

impl Default for DataSourceManager {
    fn default() -> Self {
        Self {
            sources: vec![
                DataSourceEntry {
                    id: "mt5-darwinex".into(),
                    cache_prefix: "mt5".into(),
                    label: "MT5 (Darwinex)".into(),
                    priority: 1,
                    healthy: true,
                    last_success_ts: 0,
                    asset_classes: vec!["forex".into(), "cfd".into()],
                },
                DataSourceEntry {
                    id: "kraken".into(),
                    cache_prefix: "kraken".into(),
                    label: "Kraken".into(),
                    priority: 2,
                    healthy: true,
                    last_success_ts: 0,
                    asset_classes: vec!["crypto".into(), "tokenized_equity".into(), "etf".into()],
                },
                DataSourceEntry {
                    id: "kraken-futures".into(),
                    cache_prefix: "kraken-futures".into(),
                    label: "Kraken Futures".into(),
                    priority: 3,
                    healthy: true,
                    last_success_ts: 0,
                    asset_classes: vec!["crypto_futures".into(), "futures".into()],
                },
                DataSourceEntry {
                    id: "cryptocompare".into(),
                    cache_prefix: "cryptocompare".into(),
                    label: "CryptoCompare".into(),
                    priority: 4,
                    healthy: true,
                    last_success_ts: 0,
                    asset_classes: vec!["crypto".into()],
                },
                DataSourceEntry {
                    id: "alpaca".into(),
                    cache_prefix: "alpaca".into(),
                    label: "Alpaca (delayed fallback)".into(),
                    priority: 6,
                    healthy: true,
                    last_success_ts: 0,
                    asset_classes: vec!["equity".into()],
                },
            ],
            overrides: Vec::new(),
            health_timeout_secs: 900, // 15 minutes
        }
    }
}

impl DataSourceManager {
    /// Record a successful data delivery from a source.
    pub fn mark_success(&mut self, source_id: &str) {
        let now = chrono::Utc::now().timestamp();
        if let Some(s) = self.sources.iter_mut().find(|s| s.id == source_id) {
            s.healthy = true;
            s.last_success_ts = now;
        }
    }

    /// Record a failed data delivery from a source.
    pub fn mark_failure(&mut self, source_id: &str) {
        if let Some(s) = self.sources.iter_mut().find(|s| s.id == source_id) {
            s.healthy = false;
        }
    }

    /// Update health status based on timeouts. Call periodically (e.g. every 60s).
    pub fn update_health(&mut self) {
        let now = chrono::Utc::now().timestamp();
        let timeout = self.health_timeout_secs;
        for s in &mut self.sources {
            if s.last_success_ts > 0 && now - s.last_success_ts > timeout {
                s.healthy = false;
            }
        }
    }

    /// Get ordered cache key candidates for a symbol + timeframe.
    /// Checks per-symbol overrides first, then falls back to default priority.
    /// Skips unhealthy sources (but includes them at the end as last resort).
    pub fn resolve_candidates(&self, symbol: &str, timeframe: &str) -> Vec<String> {
        let sym_upper = symbol.to_uppercase();

        // Check per-symbol overrides first
        let override_sources = self.overrides.iter().find(|o| {
            let pat = o.pattern.to_uppercase();
            if pat.ends_with('*') {
                sym_upper.starts_with(&pat[..pat.len() - 1])
            } else {
                sym_upper == pat
            }
        });

        let ordered: Vec<&DataSourceEntry> = if let Some(ovr) = override_sources {
            // Use override ordering
            ovr.sources
                .iter()
                .filter_map(|id| self.sources.iter().find(|s| s.id == *id))
                .collect()
        } else {
            // Default priority ordering
            let mut sorted: Vec<&DataSourceEntry> = self.sources.iter().collect();
            sorted.sort_by_key(|s| s.priority);
            sorted
        };

        // Healthy sources first, then unhealthy as fallback
        let mut healthy: Vec<String> = Vec::new();
        let mut unhealthy: Vec<String> = Vec::new();
        for s in &ordered {
            let key = format!("{}:{}:{}", s.cache_prefix, sym_upper, timeframe);
            if s.healthy {
                healthy.push(key);
            } else {
                unhealthy.push(key);
            }
        }
        // Also add bare key (legacy)
        healthy.push(format!("{}:{}", sym_upper, timeframe));
        healthy.extend(unhealthy);
        healthy
    }

    /// Find which source a cache key belongs to by prefix.
    pub fn source_for_key(&self, cache_key: &str) -> Option<&DataSourceEntry> {
        self.sources
            .iter()
            .find(|s| cache_key.starts_with(&format!("{}:", s.cache_prefix)))
    }

    /// Get a summary of all sources and their health status.
    pub fn status_summary(&self) -> Vec<(String, String, bool, i64)> {
        self.sources
            .iter()
            .map(|s| (s.id.clone(), s.label.clone(), s.healthy, s.last_success_ts))
            .collect()
    }

    /// Add a per-symbol routing override.
    pub fn add_override(&mut self, pattern: &str, sources: Vec<String>) {
        // Remove existing override for this pattern
        self.overrides
            .retain(|o| o.pattern.to_uppercase() != pattern.to_uppercase());
        self.overrides.push(SymbolOverride {
            pattern: pattern.to_uppercase(),
            sources,
        });
    }
}

// ── KV Cache Keys ──────────────────────────────────────────────────

pub mod cache_keys {
    pub const CONFIG: &str = "data_sources:config";
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_sources_ordered() {
        let mgr = DataSourceManager::default();
        assert_eq!(mgr.sources.len(), 6);
        assert_eq!(mgr.sources[0].id, "mt5-darwinex");
        assert_eq!(mgr.sources[0].priority, 1);
        assert_eq!(mgr.sources[1].id, "kraken");
        assert_eq!(mgr.sources[2].id, "kraken-futures");
        assert_eq!(mgr.sources[5].id, "alpaca");
    }

    #[test]
    fn resolve_candidates_default_order() {
        let mgr = DataSourceManager::default();
        let candidates = mgr.resolve_candidates("EURUSD", "1Hour");
        assert_eq!(candidates[0], "mt5:EURUSD:1Hour");
        assert_eq!(candidates[1], "kraken:EURUSD:1Hour");
        assert_eq!(candidates[2], "kraken-futures:EURUSD:1Hour");
        assert_eq!(candidates[3], "cryptocompare:EURUSD:1Hour");
        assert_eq!(candidates[4], "alpaca:EURUSD:1Hour");
        assert_eq!(candidates[5], "EURUSD:1Hour"); // bare key
    }

    #[test]
    fn resolve_candidates_skips_unhealthy() {
        let mut mgr = DataSourceManager::default();
        mgr.mark_failure("mt5-darwinex");
        let candidates = mgr.resolve_candidates("AAPL", "1Day");
        // Healthy first, then unhealthy at end
        assert_eq!(candidates[0], "kraken:AAPL:1Day");
        assert!(candidates.iter().any(|c| c == "mt5:AAPL:1Day")); // still present, just last
    }

    #[test]
    fn resolve_candidates_with_override() {
        let mut mgr = DataSourceManager::default();
        mgr.add_override("BTC*", vec!["kraken".into(), "cryptocompare".into()]);
        let candidates = mgr.resolve_candidates("BTCUSD", "1Hour");
        assert_eq!(candidates[0], "kraken:BTCUSD:1Hour");
        assert_eq!(candidates[1], "cryptocompare:BTCUSD:1Hour");
    }

    #[test]
    fn source_for_key() {
        let mgr = DataSourceManager::default();
        let source = mgr.source_for_key("mt5:EURUSD:1Hour");
        assert_eq!(source.map(|s| s.id.as_str()), Some("mt5-darwinex"));
        assert!(mgr.source_for_key("unknown:X:1D").is_none());
    }

    #[test]
    fn mark_success_updates_timestamp() {
        let mut mgr = DataSourceManager::default();
        assert_eq!(mgr.sources[0].last_success_ts, 0);
        mgr.mark_success("mt5-darwinex");
        assert!(mgr.sources[0].last_success_ts > 0);
        assert!(mgr.sources[0].healthy);
    }

    #[test]
    fn mark_failure_sets_unhealthy() {
        let mut mgr = DataSourceManager::default();
        mgr.mark_failure("alpaca");
        let alpaca = mgr.sources.iter().find(|s| s.id == "alpaca");
        assert!(!alpaca.map(|s| s.healthy).unwrap_or(true));
    }

    #[test]
    fn config_roundtrip() {
        let mgr = DataSourceManager::default();
        let json = serde_json::to_string(&mgr).unwrap();
        let back: DataSourceManager = serde_json::from_str(&json).unwrap();
        assert_eq!(back.sources.len(), 6);
        assert_eq!(back.health_timeout_secs, 900);
    }

    #[test]
    fn config_deny_unknown_fields() {
        let json = r#"{"sources":[],"overrides":[],"health_timeout_secs":900,"extra":1}"#;
        assert!(serde_json::from_str::<DataSourceManager>(json).is_err());
    }

    #[test]
    fn override_replaces_existing() {
        let mut mgr = DataSourceManager::default();
        mgr.add_override("BTC*", vec!["kraken".into()]);
        mgr.add_override("BTC*", vec!["cryptocompare".into()]);
        assert_eq!(mgr.overrides.len(), 1);
        assert_eq!(mgr.overrides[0].sources, vec!["cryptocompare"]);
    }

    #[test]
    fn status_summary() {
        let mgr = DataSourceManager::default();
        let summary = mgr.status_summary();
        assert_eq!(summary.len(), 6);
        assert!(summary[0].2); // mt5 healthy
    }

    #[test]
    fn resolve_candidates_case_insensitive() {
        let mgr = DataSourceManager::default();
        let upper = mgr.resolve_candidates("EURUSD", "1Hour");
        let lower = mgr.resolve_candidates("eurusd", "1Hour");
        // Both should produce uppercase keys
        assert_eq!(upper[0], "mt5:EURUSD:1Hour");
        assert_eq!(lower[0], "mt5:EURUSD:1Hour");
    }

    #[test]
    fn wildcard_override_prefix_match() {
        let mut mgr = DataSourceManager::default();
        mgr.add_override("SOL*", vec!["cryptocompare".into()]);
        let c1 = mgr.resolve_candidates("SOLUSD", "1Day");
        assert_eq!(c1[0], "cryptocompare:SOLUSD:1Day");
        // Non-matching symbol uses default order
        let c2 = mgr.resolve_candidates("AAPL", "1Day");
        assert_eq!(c2[0], "mt5:AAPL:1Day");
    }

    #[test]
    fn exact_override_match() {
        let mut mgr = DataSourceManager::default();
        mgr.add_override("XAUUSD", vec!["mt5-darwinex".into(), "alpaca".into()]);
        let c = mgr.resolve_candidates("XAUUSD", "1Hour");
        assert_eq!(c[0], "mt5:XAUUSD:1Hour");
        assert_eq!(c[1], "alpaca:XAUUSD:1Hour");
    }

    #[test]
    fn health_timeout_marks_stale_sources() {
        let mut mgr = DataSourceManager::default();
        mgr.health_timeout_secs = 1; // 1 second timeout for test
        mgr.sources[0].last_success_ts = chrono::Utc::now().timestamp() - 10; // 10s ago
        mgr.update_health();
        assert!(!mgr.sources[0].healthy); // should be marked unhealthy
    }

    #[test]
    fn mark_success_restores_health() {
        let mut mgr = DataSourceManager::default();
        mgr.mark_failure("mt5-darwinex");
        assert!(!mgr.sources[0].healthy);
        mgr.mark_success("mt5-darwinex");
        assert!(mgr.sources[0].healthy);
    }

    #[test]
    fn unknown_source_id_ignored() {
        let mut mgr = DataSourceManager::default();
        mgr.mark_success("nonexistent");
        mgr.mark_failure("nonexistent");
        // No panic, no change
        assert_eq!(mgr.sources.len(), 6);
    }
}
