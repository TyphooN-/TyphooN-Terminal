use super::*;
use crate::app::app_runtime_support::is_routine_market_data_status;

impl TyphooNApp {
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
        // OrderResult is used for many non-trade messages (LAN sync, backfill, etc.)
        // that would spam GetPositions → HTTP 429 Too Many Requests.
        let is_trade = msg.contains("filled")
            || msg.contains("order")
            || msg.contains("closed")
            || msg.contains("cancelled");
        if is_trade && self.alpaca_enabled && self.broker_connected {
            let _ = self.broker_tx.send(BrokerCmd::GetPositions);
            let _ = self.broker_tx.send(BrokerCmd::GetOrders);
        }
        if is_trade
            && self.kraken_enabled
            && self.kraken_connected
            && msg.to_ascii_lowercase().contains("kraken")
        {
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
