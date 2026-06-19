use super::*;

impl TyphooNApp {
    pub(super) fn tick_kraken_ws_scheduling(&mut self, now_instant: std::time::Instant) {
        // WS OHLC spawn is pair-discovery/settings driven. At startup the
        // settings loop can run before Kraken AssetPairs have landed, in
        // which case maybe_start_kraken_ws_ohlc defers without flipping
        // `started=true`. Retry every 15s so the full-universe streamers come
        // up once pair discovery completes, without forcing the user to toggle
        // the setting. Cheap idempotent no-op once `started=true`.
        if !self.kraken_ws_ohlc_started
            && self.kraken_ws_ohlc_enabled
            && self.kraken_enabled
            && now_instant.duration_since(self.kraken_ws_ohlc_last_spawn_retry)
                >= std::time::Duration::from_secs(15)
        {
            self.kraken_ws_ohlc_last_spawn_retry = now_instant;
            self.maybe_start_kraken_ws_ohlc();
        }

        // Chart bid/ask should prefer Kraken's WS v2 L2 top-of-book when the
        // active chart is a Kraken spot or xStock symbol. OHLC updates are bar
        // cadence; ticker/iapi can lag or be delayed. The book stream is the
        // freshest public best bid/ask feed we have and validates CRC32 before
        // publishing top-of-book ticks back into ChartState.
        if self.kraken_enabled
            && now_instant.duration_since(self.kraken_chart_l2_last_start_attempt)
                >= std::time::Duration::from_secs(5)
            && let Some(chart) = self.charts.get(self.active_tab)
        {
            let source = cache_source_from_key(&chart.symbol);
            let bare = bare_symbol_from_key(&chart.symbol)
                .trim_end_matches(".EQ")
                .to_ascii_uppercase();
            let kraken_chart = matches!(source, "kraken" | "kraken-equities")
                || {
                    let chart_symbol_upper = chart.symbol.to_ascii_uppercase();
                    chart_symbol_upper.contains("KRAKEN") || chart_symbol_upper.contains(".EQ")
                }
                || self
                    .kraken_equity_universe_symbols
                    .binary_search_by(|symbol| symbol.trim_end_matches(".EQ").cmp(&bare))
                    .is_ok();
            if kraken_chart
                && !bare.is_empty()
                && !self.kraken_chart_l2_ws_symbol.eq_ignore_ascii_case(&bare)
            {
                self.kraken_chart_l2_last_start_attempt = now_instant;
                self.kraken_chart_l2_ws_symbol = bare.clone();
                let _ = self.broker_tx.send(BrokerCmd::KrakenStartOrderbookWs {
                    symbol: bare,
                    depth: 10,
                    publish_dom: false,
                });
            }
        }
    }

    pub(super) fn handle_kraken_ws_status(&mut self, status: String, message: String) {
        let should_reconcile = status == "online" && message.contains("reconnected");
        let text = format!("Kraken WS {status}: {message}");
        if matches!(status.as_str(), "error" | "closed") {
            self.log.push_back(LogEntry::warn(text));
        } else {
            self.log.push_back(LogEntry::info(text));
        }
        if should_reconcile && self.kraken_enabled {
            // A reconnect means a delta gap may exist. Pull REST snapshots so
            // balances, cost basis, P/L, and open orders converge immediately.
            let _ = self.broker_tx.send(BrokerCmd::KrakenGetBalance);
            let _ = self.broker_tx.send(BrokerCmd::KrakenGetPositions);
            let _ = self.broker_tx.send(BrokerCmd::KrakenFetchTrades);
            let _ = self.broker_tx.send(BrokerCmd::KrakenFetchOpenOrders);
        }
    }

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
