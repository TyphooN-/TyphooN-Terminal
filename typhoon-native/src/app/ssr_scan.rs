use super::*;

/// Watchlist cache-key prefixes the SSR rule applies to — US equities only.
/// Crypto (`kraken:`) and futures (`kraken-futures:`) are out of scope for
/// SEC Rule 201.
fn ssr_equity_cache_key(cache_key: &str) -> bool {
    ["alpaca:", "kraken-equities:", "merged:", "yahoo-chart:"]
        .iter()
        .any(|p| cache_key.starts_with(p))
}

impl TyphooNApp {
    /// Computed SSR (Short Sale Restriction, SEC Rule 201) state machine —
    /// ADR-120's "next free extension". Every ~30s wall clock:
    ///
    /// - once per ET date, purge alerts whose restriction window ended
    ///   (trigger day + next trading day, holiday-aware);
    /// - while a US extended session is possible, walk the watchlist's
    ///   US-equity rows and flag any symbol trading ≥10% below its prior
    ///   close with a `kind='ssr'` regulatory alert.
    ///
    /// Writes run on a blocking worker (never the render thread); the BG
    /// regulatory refresh re-reads the table each cycle, so chart-header and
    /// watchlist badges pick new flags up without extra plumbing.
    pub(super) fn tick_ssr_scan(&mut self) {
        let now_utc = chrono::Utc::now();
        let now_s = now_utc.timestamp();
        if now_s - self.last_ssr_scan_s < 30 {
            return;
        }
        self.last_ssr_scan_s = now_s;
        let Some(cache) = self.cache.clone() else {
            return;
        };
        let today = typhoon_engine::core::market_session::us_eastern_date(now_utc)
            .format("%Y-%m-%d")
            .to_string();
        let run_purge = self.ssr_purge_done_for != today;

        // Trigger detection only makes sense while US equities can trade.
        let session_possible =
            typhoon_engine::core::market_session::us_equities_extended_session_possible(now_utc);
        let mut triggered: Vec<(String, f64, f64)> = Vec::new();
        if session_possible {
            for row in &self.watchlist_rows {
                if !ssr_equity_cache_key(&row.cache_key) {
                    continue;
                }
                if !typhoon_engine::core::regulatory_alerts::ssr_triggered(row.last, row.prev_close)
                {
                    continue;
                }
                let symbol = typhoon_engine::core::regulatory_alerts::normalize_regulatory_symbol(
                    &row.symbol,
                );
                if symbol.is_empty() {
                    continue;
                }
                // Already flagged (any active SSR row) — the upsert would be a
                // harmless refresh, but skipping keeps the write path quiet.
                let already = self
                    .bg
                    .regulatory_alerts_by_symbol
                    .get(&symbol)
                    .is_some_and(|alerts| alerts.iter().any(|a| a.kind == "ssr"));
                if already {
                    continue;
                }
                let drop_pct = (row.last / row.prev_close - 1.0) * 100.0;
                triggered.push((symbol, drop_pct, row.prev_close));
            }
            triggered.sort_by(|a, b| a.0.cmp(&b.0));
            triggered.dedup_by(|a, b| a.0 == b.0);
        }

        if !run_purge && triggered.is_empty() {
            return;
        }
        if run_purge {
            self.ssr_purge_done_for = today.clone();
        }
        for (symbol, drop_pct, _) in &triggered {
            self.log.push_back(LogEntry::warn(format!(
                "SSR triggered: {} {:.1}% below prior close — short sale restriction through the next trading day",
                symbol, drop_pct
            )));
        }
        self.rt_handle.spawn_blocking(move || {
            let Ok(conn) = cache.connection() else {
                return;
            };
            if run_purge {
                let _ = typhoon_engine::core::regulatory_alerts::purge_expired_ssr_alerts(
                    &conn, &today,
                );
            }
            for (symbol, drop_pct, prev_close) in triggered {
                let _ = typhoon_engine::core::regulatory_alerts::upsert_ssr_alert(
                    &conn, &symbol, &today, drop_pct, prev_close,
                );
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssr_scope_is_us_equity_sources_only() {
        assert!(ssr_equity_cache_key("alpaca:AAPL:1Day"));
        assert!(ssr_equity_cache_key("kraken-equities:WOK:1Day"));
        assert!(ssr_equity_cache_key("merged:HUBC:1Day"));
        assert!(ssr_equity_cache_key("yahoo-chart:TSLA:1Day"));
        assert!(!ssr_equity_cache_key("kraken:BTCUSD:1Day"));
        assert!(!ssr_equity_cache_key("kraken-futures:PI_XBTUSD:1Day"));
    }
}
