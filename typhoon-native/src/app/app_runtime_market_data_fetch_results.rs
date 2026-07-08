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
            if self.alpaca_consecutive_429 != 0 {
                self.alpaca_consecutive_429 = 0;
            }
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
        if count > 0 {
            self.unresolvable_drain(&source, &symbol, &timeframe);
        }
        if source == "yahoo-chart" && self.yahoo_chart_consecutive_429 != 0 {
            // A completed Yahoo response (any bar count) proves the lane isn't
            // 429-blocked right now — reset the escalating backoff so the next
            // isolated 429 recovers in ~45s instead of compounding toward 10m.
            self.yahoo_chart_consecutive_429 = 0;
        }
        if source == "yahoo-chart" && count > 0 {
            // Yahoo fetches always request full period1=0 history, so a
            // non-empty store saturates the provider window: suppress future
            // Backfill re-selection for this pair (Stale refresh still runs).
            self.yahoo_chart_backfill_complete_mark(&symbol, &timeframe, count);
        }

        if should_reload {
            self.queue_chart_reload(self.active_tab);
        }

        // MTF_MA / MultiKAMA overlays project a symbol's higher-timeframe series
        // (H1/H4/D1/W1/MN1) onto its lower-timeframe charts. `ensure_mql_mtf_overlays_for_render`
        // only (re)computes those lines while the overlay buffer is *empty*, so a
        // higher timeframe that lands in the cache AFTER a chart first rendered
        // never backfills — the "missing SMAs on chart" report. When such a
        // higher-timeframe series is freshly fetched, queue an overlay-refreshing
        // reload for the OTHER open charts of the same symbol (the active chart is
        // handled above). Gated on an overlay being enabled and suppressed during
        // full-universe sync; the deferred loader dedupes/paces the reloads.
        if (self.show_sma200 || self.show_kama)
            && !self.heavy_sync_in_progress
            && Self::is_mtf_overlay_source_timeframe(&timeframe)
        {
            self.queue_same_symbol_overlay_reloads(&symbol, self.active_tab);
        }
        !source_has_terminal_settlement && matches!(source.as_str(), "kraken" | "kraken-futures")
    }

    /// True when `timeframe` is one of the higher-timeframe series the MTF_MA /
    /// MultiKAMA overlays project onto lower-timeframe charts. Lower timeframes
    /// (M1–M30) are never overlay inputs, so a fetch on them can't backfill a
    /// missing overlay line and doesn't warrant a same-symbol reload sweep.
    fn is_mtf_overlay_source_timeframe(timeframe: &str) -> bool {
        matches!(timeframe, "1Hour" | "4Hour" | "1Day" | "1Week" | "1Month")
    }

    /// Refresh the MTF_MA / MultiKAMA overlay lines for every open chart of
    /// `symbol` except `skip_idx` (handled by the active-chart path), without
    /// reloading bars.
    ///
    /// Backfilling overlays does NOT require a full chart reload: the chart
    /// already has its candles, only the projected higher-timeframe lines are
    /// stale (they were computed before this HTF series landed in cache). The old
    /// implementation queued a full `queue_chart_reload`, which re-decompressed
    /// and re-merged every same-symbol chart's candles on the render thread — the
    /// ~140–790ms-per-chart "second pass" that fired once `heavy_sync` ended and
    /// doubled startup load time on a large grid. Recompute just the overlay lines
    /// from the now-cached HTF series instead; the shared HTF indicator memo makes
    /// the SMA200/KAMA computation O(1) per symbol/timeframe across hosts, so this
    /// is a small projection per chart rather than a fresh SQLite load. Charts with
    /// no bars yet are skipped (nothing to project onto — the deferred loader will
    /// load them normally).
    fn queue_same_symbol_overlay_reloads(&mut self, symbol: &str, skip_idx: usize) {
        let Some(cache) = self.cache.clone() else {
            return;
        };
        let targets: Vec<usize> = self
            .charts
            .iter()
            .enumerate()
            .filter(|(idx, c)| *idx != skip_idx && c.symbol_matches(symbol) && !c.bars.is_empty())
            .map(|(idx, _)| idx)
            .collect();
        for idx in targets {
            if let Some(chart) = self.charts.get_mut(idx) {
                chart.compute_mtf_sma(&cache);
                chart.compute_multi_kama(&cache);
            }
        }
    }

    fn handle_alpaca_fetch_settled(
        &mut self,
        symbol: String,
        timeframe: String,
        success: bool,
    ) -> bool {
        self.settle_market_data_fetch("alpaca", &symbol, &timeframe);
        if success {
            // Adaptive re-probe backoff: grow the empty-streak when this fetch
            // wrote nothing, reset it when it landed bars (the preceding
            // BarsFetched already advanced write_ts_s). A failed fetch is left to
            // the retry machinery and doesn't count as "caught up".
            self.note_alpaca_refetch_outcome(&symbol, &timeframe);
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
