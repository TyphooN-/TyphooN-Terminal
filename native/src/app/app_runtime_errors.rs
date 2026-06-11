use super::*;
use crate::app::app_runtime_support::is_routine_market_data_status;

impl TyphooNApp {
    pub(super) fn handle_broker_error(&mut self, e: String, now: i64) {
        // Compact pass failed — clear in_progress so the gate can retry next window.
        // Don't update last_run_ms; we want the cadence to keep trying.
        if e.starts_with("Compact failed:")
            || (self.auto_compact_in_progress && e.starts_with("Cannot open cache:"))
        {
            self.auto_compact_in_progress = false;
            self.auto_compact_started_ms = 0;
            self.auto_compact_last_skip = Some(format!("last attempt failed: {}", e));
        }
        if e.starts_with("Asset fetch failed:") {
            self.all_broker_assets_fetched = false;
        } else if e.starts_with("Kraken pairs:") {
            self.kraken_pairs_requested = false;
        } else if e.starts_with("Kraken futures instruments:") {
            self.kraken_futures_requested = false;
        } else if e.starts_with("Kraken equities universe failed:") {
            self.kraken_equity_universe_requested = false;
            let backoff = if e.contains("iapi temporarily rate-limited")
                || e.contains("1015")
                || e.contains("429")
                || e.to_ascii_lowercase().contains("rate limit")
            {
                300
            } else {
                60
            };
            self.kraken_equity_universe_retry_after_ts = now + backoff;
        } else if e.contains("Yahoo Chart HTTP 429") {
            let pause = 300; // 5 minutes backoff on Yahoo rate limit
            if now + pause > self.yahoo_chart_sync_pause_until_ts {
                self.yahoo_chart_sync_pause_until_ts = now + pause;
                self.yahoo_chart_sync_pause_reason = e.clone();
                self.log.push_back(LogEntry::warn(format!(
                    "Yahoo Chart rate limited — pausing fallback lane for 5m"
                )));
            }
        } else if e.contains("401") || e.contains("Unauthorized") || e.contains("403") {
            if self.broker_connected {
                self.broker_connected = false;
                self.log.push_back(LogEntry::err(format!(
                    "{} — disconnected (check API keys in Settings)",
                    e
                )));
            }
            // Don't log repeated auth failures
        } else if is_routine_market_data_status(&e) {
            tracing::debug!("{}", e);
        } else {
            self.log.push_back(LogEntry::err(e));
        }
    }
}
