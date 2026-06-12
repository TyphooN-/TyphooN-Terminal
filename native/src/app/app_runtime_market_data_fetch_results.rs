use super::*;

impl TyphooNApp {
    pub(super) fn handle_market_data_fetch_result_msg(&mut self, msg: BrokerMsg) -> bool {
        match msg {
            BrokerMsg::BarsFetched {
                source,
                symbol,
                timeframe,
                count,
            } => self.handle_bars_fetched(source, symbol, timeframe, count),
            BrokerMsg::AlpacaFetchSettled {
                symbol,
                timeframe,
                success,
            } => self.handle_alpaca_fetch_settled(symbol, timeframe, success),
            BrokerMsg::KrakenFetchSettled { symbol, timeframe } => {
                self.settle_market_data_fetch("kraken", &symbol, &timeframe);
                true
            }
            BrokerMsg::KrakenBackfillComplete {
                symbol,
                timeframe,
                bar_count,
                target_bars,
            } => {
                self.handle_kraken_backfill_complete(symbol, timeframe, bar_count, target_bars);
                false
            }
            BrokerMsg::KrakenFuturesFetchSettled { symbol, timeframe } => {
                self.settle_market_data_fetch("kraken-futures", &symbol, &timeframe);
                true
            }
            BrokerMsg::KrakenFuturesBackfillComplete {
                symbol,
                timeframe,
                bar_count,
                target_bars,
            } => {
                self.handle_kraken_futures_backfill_complete(
                    symbol,
                    timeframe,
                    bar_count,
                    target_bars,
                );
                false
            }
            _ => false,
        }
    }

    fn handle_bars_fetched(
        &mut self,
        source: String,
        symbol: String,
        timeframe: String,
        count: usize,
    ) -> bool {
        let should_reload = self
            .charts
            .get(self.active_tab)
            .map(|c| c.should_reload_for_bar_fetch(&symbol, &timeframe, &source))
            .unwrap_or(false);
        let source_label = match source.as_str() {
            "alpaca" => "Alpaca",
            "kraken" => "Kraken",
            "kraken-futures" => "Kraken Futures",
            "yahoo-chart" => "Yahoo Chart",
            _ => source.as_str(),
        };
        if should_reload {
            self.log.push_back(LogEntry::info(format!(
                "{} fetched {} bars for {} {} — queued active chart reload",
                source_label, count, symbol, timeframe
            )));
        } else {
            tracing::debug!(
                "{} fetched {} bars for {} {}",
                source_label,
                count,
                symbol,
                timeframe
            );
        }
        let source_has_terminal_settlement =
            matches!(source.as_str(), "alpaca" | "kraken" | "kraken-futures");
        if !source_has_terminal_settlement {
            self.settle_market_data_fetch(&source, &symbol, &timeframe);
        }
        if source_has_terminal_settlement {
            self.note_cached_sync_success(&source, &symbol, &timeframe, count);
        }
        if source == "alpaca" {
            // Any newly-written bars supersede prior no-data tombstones.
            self.alpaca_no_data_drain(&symbol, &timeframe);
            // Avoid a synchronous full SQLite storage-stat scan for every
            // automated bar write. `note_cached_sync_success` keeps the
            // scheduler O(1)-fresh; refresh the heavy Storage view only
            // when a storage window is visible.
            if self.show_storage || self.show_cache_stats {
                self.refresh_storage_snapshot_after_action("alpaca_bars");
            }
        }

        if should_reload {
            self.queue_chart_reload(self.active_tab);
        }
        !source_has_terminal_settlement && matches!(source.as_str(), "kraken" | "kraken-futures")
    }

    fn handle_alpaca_fetch_settled(
        &mut self,
        symbol: String,
        timeframe: String,
        success: bool,
    ) -> bool {
        self.settle_market_data_fetch("alpaca", &symbol, &timeframe);
        if success {
            self.alpaca_retry_drain(&symbol, &timeframe);
            return true;
        }
        false
    }

    fn handle_kraken_backfill_complete(
        &mut self,
        symbol: String,
        timeframe: String,
        bar_count: usize,
        target_bars: usize,
    ) {
        let changed =
            self.kraken_backfill_complete_mark(&symbol, &timeframe, bar_count, target_bars);
        if changed {
            let marker_count = self.kraken_backfill_complete_pairs.len();
            // First-time saturation per pair is one-shot, but across ~12 k
            // tradable symbols this floods the user log during initial sweep.
            // Detailed line goes to debug; a milestone rollup at every 100th
            // new marker keeps progress visible without spam.
            tracing::debug!(
                "Kraken {} {}: provider window saturated at {}/{} bars ({} marked)",
                symbol,
                timeframe,
                bar_count,
                target_bars,
                marker_count
            );
            if marker_count.is_multiple_of(100) {
                self.log.push_back(LogEntry::info(format!(
                    "Kraken backfill milestone: {} pairs at provider-window saturation",
                    marker_count
                )));
            }
        }
    }

    fn handle_kraken_futures_backfill_complete(
        &mut self,
        symbol: String,
        timeframe: String,
        bar_count: usize,
        target_bars: usize,
    ) {
        let changed =
            self.kraken_futures_backfill_complete_mark(&symbol, &timeframe, bar_count, target_bars);
        if changed {
            let marker_count = self.kraken_futures_backfill_complete_pairs.len();
            self.log.push_back(LogEntry::info(format!(
                "Kraken Futures {} {}: marked backfill-complete at {}/{} bars — full history exhausted; automated sync will keep it current ({} marked)",
                symbol, timeframe, bar_count, target_bars, marker_count
            )));
        }
    }
}
