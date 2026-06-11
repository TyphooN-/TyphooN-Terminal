use super::*;

impl TyphooNApp {
    pub(super) fn handle_kraken_ws_bars_committed(&mut self, fresh: Vec<(String, String, i64)>) {
        // Mark each (symbol, tf) WS-fresh so the REST scheduler skips refetch while
        // the WS feed is keeping the cache current. O(n) over the flush batch;
        // per-key insert is O(1).
        let now_ms = chrono::Utc::now().timestamp_millis();
        for (symbol, tf, last_bar_ts_ms) in fresh {
            self.kraken_ws_fresh_until
                .insert((symbol, tf), now_ms.max(last_bar_ts_ms));
        }
    }

    pub(super) fn handle_kraken_ws_ohlc_status(
        &mut self,
        interval_min: u32,
        kind: String,
        detail: String,
    ) {
        let tf = typhoon_engine::broker::kraken::kraken_ws_interval_to_tf_label(interval_min)
            .unwrap_or("?");
        let msg = if detail.is_empty() {
            format!("Kraken WS OHLC {tf}: {kind}")
        } else {
            format!("Kraken WS OHLC {tf}: {kind} — {detail}")
        };
        if matches!(
            kind.as_str(),
            "disconnected"
                | "subscribe_failed"
                | "snapshot_disconnected"
                | "snapshot_subscribe_failed"
        ) {
            self.log.push_back(LogEntry::warn(msg));
        } else {
            self.log.push_back(LogEntry::info(msg));
        }
    }

    pub(super) fn handle_kraken_ws_ohlc_snapshot_sweep_settled(
        &mut self,
        interval_min: u32,
        pair_count: usize,
        error: Option<String>,
    ) {
        self.kraken_ws_ohlc_snapshot_sweep_in_flight = false;
        let tf = typhoon_engine::broker::kraken::kraken_ws_interval_to_tf_label(interval_min)
            .unwrap_or("?");
        if let Some(error) = error {
            self.log.push_back(LogEntry::warn(format!(
                "Kraken WS OHLC snapshot sweep {tf} failed after {pair_count} pairs — {error}"
            )));
        } else {
            self.log.push_back(LogEntry::info(format!(
                "Kraken WS OHLC snapshot sweep {tf}: completed {pair_count} pairs"
            )));
        }
    }
}
