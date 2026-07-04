use super::*;
use crate::app::app_runtime_support::{is_routine_market_data_status, yahoo_chart_429_backoff_secs};

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
            // Escalating, self-decaying backoff. The lane fires several concurrent
            // requests, so one rate-limit event arrives as a burst of 429s — only
            // escalate once per pause window (when not already paused) so the burst
            // counts once. An isolated 429 now recovers in ~45s instead of the old
            // flat 5m that pinned the lane dark; sustained limiting escalates toward
            // a 10m ceiling. The counter resets on the first successful Yahoo
            // response (see handle_bars_fetched).
            if self.yahoo_chart_sync_pause_until_ts <= now {
                self.yahoo_chart_consecutive_429 = self.yahoo_chart_consecutive_429.saturating_add(1);
                let pause = yahoo_chart_429_backoff_secs(self.yahoo_chart_consecutive_429);
                self.yahoo_chart_sync_pause_until_ts = now + pause;
                self.yahoo_chart_sync_pause_reason = e.clone();
                self.log.push_back(LogEntry::warn(format!(
                    "Yahoo Chart rate limited (x{}) — pausing fallback lane {}s",
                    self.yahoo_chart_consecutive_429, pause
                )));
            }
        } else if e.contains("401") || e.contains("Unauthorized") || e.contains("403") {
            let disconnect_msg = format!("{} — API disconnected (check API keys in Settings)", e);
            if self.broker_connected {
                self.broker_connected = false;
                self.log.push_back(LogEntry::err(disconnect_msg.clone()));
            }
            if e.to_ascii_lowercase().contains("kraken") {
                self.kraken_connected = false;
            }
            self.push_connection_toast(disconnect_msg, false);
            // Don't log repeated auth failures
        } else if is_routine_market_data_status(&e) {
            tracing::debug!("{}", e);
        } else {
            let lower = e.to_ascii_lowercase();
            if lower.contains("disconnect")
                || lower.contains("connection failed")
                || lower.contains("auth failed")
                || lower.contains("stream failed")
            {
                self.push_connection_toast(e.clone(), false);
            }
            self.log.push_back(LogEntry::err(e));
        }
    }
}
