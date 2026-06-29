use super::*;
use crate::app::app_runtime_support::is_routine_market_data_status;

impl TyphooNApp {
    pub(super) fn tick_positions_orders_refresh(&mut self, now_instant: std::time::Instant) {
        // Positions/orders are trading-critical UI, not five-minute background
        // metadata. Reconcile them periodically without tying the cadence to the
        // broad cache refresh loop; the dispatch timestamp prevents per-frame spam
        // if a broker response is slow.
        let positions_due = self
            .positions_auto_refresh_at
            .map(|t| now_instant.duration_since(t) >= std::time::Duration::from_secs(30))
            .unwrap_or(true);
        if positions_due {
            let mut requested = false;
            if self.alpaca_enabled && self.broker_connected {
                let _ = self.broker_tx.send(BrokerCmd::GetPositions);
                let _ = self.broker_tx.send(BrokerCmd::GetOrders);
                requested = true;
            }
            if self.kraken_enabled && self.kraken_connected {
                let _ = self.broker_tx.send(BrokerCmd::KrakenGetBalance);
                let _ = self.broker_tx.send(BrokerCmd::KrakenGetPositions);
                let _ = self.broker_tx.send(BrokerCmd::KrakenFetchOpenOrders);
                requested = true;
            }
            if requested {
                self.positions_auto_refresh_at = Some(now_instant);
            }
        }

        // Market clock: the US-equities session string ("PRE-MARKET · Core in 1h
        // 20m", "OPEN · closes in …", "CLOSED · opens in …") is formatted
        // broker-side with the countdown baked in at fetch time. Fetched only on
        // connect, it freezes — a status shown hours after connect counts down from
        // the connect-time snapshot (e.g. a stale "opens in 10h 30m" on a morning
        // when core open is ~1h away). Re-fetch on a 60s cadence so the minute-
        // granularity countdown stays within a minute of truth. Alpaca's /v2/clock
        // is a trivial endpoint, so 1 req/min is negligible.
        if self.alpaca_enabled && self.broker_connected {
            let clock_due = self
                .market_clock_refresh_at
                .map(|t| now_instant.duration_since(t) >= std::time::Duration::from_secs(60))
                .unwrap_or(true);
            if clock_due {
                let _ = self.broker_tx.send(BrokerCmd::GetMarketClock);
                self.market_clock_refresh_at = Some(now_instant);
            }
        }

        // Account snapshot: equity / buying power / margins were fetched only on
        // connect, so the headline Equity froze while the derived Open P/L kept
        // updating per-frame. Refresh on a 10s cadence so equity tracks fills and
        // mark-price moves. /v2/account is a trivial endpoint (~6 req/min).
        if self.alpaca_enabled && self.broker_connected {
            let account_due = self
                .account_refresh_at
                .map(|t| now_instant.duration_since(t) >= std::time::Duration::from_secs(10))
                .unwrap_or(true);
            if account_due {
                let _ = self.broker_tx.send(BrokerCmd::GetAccount);
                self.account_refresh_at = Some(now_instant);
            }
        }
    }

    pub(super) fn handle_order_result(&mut self, msg: String) {
        // Compact pass completion — manual or auto. Mark scheduler idle and
        // record the timestamp so the cadence gate counts this run.
        // The Compact handler also emits per-200-row progress lines starting
        // with "Compact: " — those are not completions.
        if msg.starts_with("Compact complete:") {
            self.auto_compact_in_progress = false;
            self.auto_compact_started_ms = 0;
            self.auto_compact_last_run_ms = chrono::Utc::now().timestamp_millis();
            self.sync_preferences_save();
        }

        // Only refresh positions after actual trade operations (not every log message).
        // OrderResult is used for many non-trade messages (backfill, etc.)
        // that would spam GetPositions → HTTP 429 Too Many Requests.
        let msg_lc = msg.to_ascii_lowercase();
        let is_trade = msg_lc.contains("filled")
            || msg_lc.contains("order")
            || msg_lc.contains("closed")
            || msg_lc.contains("cancelled");
        if is_trade && self.alpaca_enabled && self.broker_connected {
            let _ = self.broker_tx.send(BrokerCmd::GetPositions);
            let _ = self.broker_tx.send(BrokerCmd::GetOrders);
        }
        if is_trade && self.kraken_enabled && self.kraken_connected && msg_lc.contains("kraken") {
            let _ = self.broker_tx.send(BrokerCmd::KrakenGetPositions);
            let _ = self.broker_tx.send(BrokerCmd::KrakenGetBalance);
        }

        // Track BARDATA progress.
        if msg.starts_with("BARDATA:") {
            self.bardata_log.push_back(msg.clone());
            while self.bardata_log.len() > 200 {
                self.bardata_log.pop_front();
            }
            // Count any finished fetch (success, error, or empty) as completed.
            if msg.contains("bars stored")
                || msg.contains("complete")
                || msg.contains("failed")
                || msg.contains("no bars")
            {
                self.bardata_completed += 1;
            }
        }

        // ADR-094: Use Trade log level and toast for fills.
        if is_trade {
            self.log.push_back(LogEntry::trade(&msg));
            self.toasts.push(Toast {
                message: msg,
                color: egui::Color32::from_rgb(80, 220, 120),
                created: std::time::Instant::now(),
                duration: std::time::Duration::from_secs(5),
                dismissable: false,
                dismissed: false,
            });
        } else if is_routine_market_data_status(&msg) {
            tracing::debug!("{}", msg);
        } else {
            self.log.push_back(LogEntry::info(msg));
        }
    }
}
