use super::*;

impl TyphooNApp {
    pub(super) fn handle_broker_quote(&mut self, symbol: String, bid: f64, ask: f64, last: f64) {
        self.log.push_back(LogEntry::info(format!(
            "{}: bid {} ask {} last {}",
            symbol,
            format_price(bid),
            format_price(ask),
            format_price(last)
        )));
        if last <= 0.0 || !last.is_finite() {
            return;
        }

        let wanted = bare_symbol_from_key(&symbol)
            .replace('/', "")
            .trim_end_matches(".EQ")
            .trim_end_matches(".eq")
            .to_ascii_uppercase();
        if let Some(idxs) = self.chart_by_bare.get(&wanted) {
            for &i in idxs {
                if let Some(chart) = self.charts.get_mut(i) {
                    chart.apply_forming_price_update(last);
                }
            }
        } else {
            // fallback fuzzy for legacy keys (rare)
            for chart in &mut self.charts {
                let chart_sym = chart.symbol.replace('/', "").to_ascii_uppercase();
                if chart_sym.contains(&wanted) || wanted.contains(&chart_sym) {
                    chart.apply_forming_price_update(last);
                }
            }
        }
    }

    /// Real-time Alpaca market-data L1 (rich with sizes from WS).
    /// Updates charts and watchlist with bid/ask (sizes logged for richer view).
    pub(super) fn handle_alpaca_quote(
        &mut self,
        q: typhoon_engine::broker::protocol::AlpacaQuoteData,
    ) {
        if (q.bid <= 0.0 && q.ask <= 0.0) || (!q.bid.is_finite() && !q.ask.is_finite()) {
            return;
        }
        let bid = if q.bid > 0.0 { q.bid } else { q.ask };
        let ask = if q.ask > 0.0 { q.ask } else { q.bid };
        let wanted = bare_symbol_from_key(&q.symbol)
            .replace('/', "")
            .trim_end_matches(".EQ")
            .to_ascii_uppercase();
        // O(1) via index
        if let Some(idxs) = self.chart_by_bare.get(&wanted) {
            for &i in idxs {
                if let Some(chart) = self.charts.get_mut(i) {
                    chart.apply_live_quote_update(bid, ask, q.bid_size, q.ask_size, false);
                    // Rich L1: could store sizes in chart state if extended
                }
            }
        }
        self.apply_live_quote_to_watchlist(&wanted, bid, ask, q.bid_size, q.ask_size);
        // Log richer info occasionally
        if q.bid_size > 0.0 || q.ask_size > 0.0 {
            self.log.push_back(LogEntry::info(format!(
                "Alpaca L1 {}: b{}@{} a{}@{}",
                wanted,
                format_price(q.bid_size),
                format_price(bid),
                format_price(q.ask_size),
                format_price(ask)
            )));
        }
    }

    pub(super) fn handle_kraken_book_quote_tick(
        &mut self,
        symbol: String,
        bid: f64,
        ask: f64,
        bid_size: f64,
        ask_size: f64,
    ) {
        if bid <= 0.0 || ask <= 0.0 {
            return;
        }
        let wanted = bare_symbol_from_key(&symbol);
        if let Some(idxs) = self.chart_by_bare.get(&wanted) {
            for &i in idxs {
                if let Some(chart) = self.charts.get_mut(i) {
                    chart.apply_live_quote_update(bid, ask, bid_size, ask_size, false);
                }
            }
        }
        self.apply_live_quote_to_watchlist(&wanted, bid, ask, bid_size, ask_size);
    }

    /// Rich L1 from Kraken WS v2 ticker. Uses bid/ask/sizes/last + 24h for richer view.
    pub(super) fn handle_kraken_ws_ticker(
        &mut self,
        t: typhoon_engine::broker::kraken::KrakenWsTicker,
    ) {
        let bid = t.bid.unwrap_or(0.0);
        let ask = t.ask.unwrap_or(0.0);
        let last = t.last.unwrap_or((bid + ask) * 0.5);
        if last <= 0.0 || !last.is_finite() {
            return;
        }
        let wanted = bare_symbol_from_key(&t.symbol)
            .replace('/', "")
            .trim_end_matches(".EQ")
            .to_ascii_uppercase();
        if let Some(idxs) = self.chart_by_bare.get(&wanted) {
            for &i in idxs {
                if let Some(chart) = self.charts.get_mut(i) {
                    chart.apply_live_quote_update(
                        bid,
                        ask,
                        t.bid_qty.unwrap_or(0.0),
                        t.ask_qty.unwrap_or(0.0),
                        false,
                    );
                    if t.bid.is_none() && t.ask.is_none() && t.volume_24h.unwrap_or(0.0) > 0.0 {
                        // Trade-driven ticker emission (from Kraken public trades): accumulate real volume
                        let vol = t.volume_24h.unwrap_or(0.0);
                        let _ = chart.apply_forming_trade(last, vol, t.ts_ms);
                        chart.live_trade_price = last;
                        chart.live_trade_vol = vol;
                        // Use real side from public trades if present, else heuristic
                        chart.live_trade_is_buy = match &t.last_trade_side {
                            Some(s) if s.eq_ignore_ascii_case("buy") => true,
                            Some(s) if s.eq_ignore_ascii_case("sell") => false,
                            _ => last >= chart.bars.last().map(|b| b.open).unwrap_or(last),
                        };
                        chart.mark_view_changed(); // ensure MTF + single cells repaint promptly for live trade updates (priority for foreground/MTF)
                    } else {
                        chart.apply_forming_price_update(last);
                    }
                }
            }
        }

        // Stronger MTF/foreground sync priority for live trades: mark WS-fresh for *exactly the TFs that have open charts for this symbol*
        // (not just hardcoded M1/M5). This keeps focused MTF cells fresh in the scheduler when trades arrive, reducing unnecessary REST.
        if t.bid.is_none() && t.ask.is_none() && t.volume_24h.unwrap_or(0.0) > 0.0 {
            let now_ms = chrono::Utc::now().timestamp_millis();
            if let Some(idxs) = self.chart_by_bare.get(&wanted) {
                for &i in idxs {
                    if let Some(chart) = self.charts.get(i) {
                        let tf = chart.timeframe.label().to_string();
                        // Only boost live low-TF (M1/M5) freshness from public trades.
                        // High-TF-first tiered snapshots (bounded, staggered highest-first in ohlc_pipeline)
                        // for full-universe remain untouched. This keeps tiered high-TF priority while
                        // live low-TF MTF cells get WS-fresh skips + merged-bar ts.
                        if !matches!(tf.as_str(), "1Min" | "5Min" | "M1" | "M5") {
                            continue;
                        }
                        // Use the (possibly trade-advanced) forming bar ts as the freshness anchor.
                        let bar_ts = chart.bars.last().map(|b| b.ts_ms).unwrap_or(0);
                        let ts = t.ts_ms.unwrap_or(now_ms).max(bar_ts);
                        self.kraken_ws_fresh_until
                            .insert((wanted.clone(), tf.clone()), now_ms.max(ts));

                        // Also advance the Kraken sync cache state ts. This lets the candidate
                        // scorer (classify + focus/score) treat the live-updated low-TF MTF bar
                        // as current, giving it effective priority/boost without disturbing the
                        // high-TF-first ring for full-universe.
                        let sync_key = (wanted.clone(), tf);
                        let entry = self
                            .cached_kraken_sync_state
                            .entry(sync_key)
                            .or_insert_with(Default::default);
                        entry.last_bar_ts_s = (now_ms.max(ts) / 1000) as i64;
                        entry.write_ts_s = (now_ms / 1000) as i64;
                    }
                }
            }
        }

        self.apply_live_quote_to_watchlist(
            &wanted,
            bid,
            ask,
            t.bid_qty.unwrap_or(0.0),
            t.ask_qty.unwrap_or(0.0),
        );
        // Incremental trade volume from public Kraken trades to watchlist row (O(1))
        if t.volume_24h.unwrap_or(0.0) > 0.0 {
            if let Some(&idx) = self.watchlist_by_bare.get(&wanted) {
                if let Some(row) = self.watchlist_rows.get_mut(idx) {
                    row.volume += t.volume_24h.unwrap_or(0.0);
                }
            }
        }
        // Log richer L1 occasionally (volume etc.)
        if t.volume_24h.unwrap_or(0.0) > 0.0 {
            self.log.push_back(LogEntry::info(format!(
                "Kraken L1 ticker {}: last {} vol24h {}",
                wanted,
                format_price(last),
                format_price(t.volume_24h.unwrap_or(0.0))
            )));
        }
    }

    pub(super) fn handle_kraken_equity_quote(
        &mut self,
        ticker: typhoon_engine::broker::kraken::KrakenEquityTicker,
    ) {
        if !self.kraken_enabled {
            return;
        }
        let weekend_closed = super::app_runtime_support::kraken_xstocks_weekend_closed_now();
        let symbol = ticker.symbol.to_ascii_uppercase();
        let last = ticker.price;
        if last <= 0.0 || !last.is_finite() {
            return;
        }

        let received_at_ms = chrono::Utc::now().timestamp_millis();
        self.kraken_equity_quote_meta.insert(
            symbol.clone(),
            KrakenEquityQuoteMeta {
                received_at_ms,
                quote_time_ms: ticker.time_ms,
                delayed: ticker.delayed,
                price: last,
            },
        );

        let wanted = bare_symbol_from_key(&symbol)
            .replace('/', "")
            .trim_end_matches(".EQ")
            .to_ascii_uppercase();
        if let Some(idxs) = self.chart_by_bare.get(&wanted) {
            for &i in idxs {
                if let Some(chart) = self.charts.get_mut(i) {
                    let realtime_fresh = !chart.live_quote_delayed
                        && chart
                            .live_quote_at
                            .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(30));
                    if !weekend_closed && !(ticker.delayed && realtime_fresh) {
                        chart.apply_live_quote_update(
                            ticker.bid,
                            ticker.ask,
                            0.0,
                            0.0,
                            ticker.delayed,
                        );
                    }
                }
            }
        } else {
            // fallback (rare norm diff)
            for chart in &mut self.charts {
                let chart_bare = chart.symbol.replace('/', "").to_ascii_uppercase();
                if chart_bare.contains(&wanted) {
                    let realtime_fresh = !chart.live_quote_delayed
                        && chart
                            .live_quote_at
                            .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(30));
                    if !weekend_closed && !(ticker.delayed && realtime_fresh) {
                        chart.apply_live_quote_update(
                            ticker.bid,
                            ticker.ask,
                            0.0,
                            0.0,
                            ticker.delayed,
                        );
                    }
                }
            }
        }

        let quote_updates_position = self.kr_positions.iter().any(|pos| {
            let pos_symbol = pos
                .symbol
                .replace('/', "")
                .trim_end_matches(".EQ")
                .to_ascii_uppercase();
            pos_symbol == symbol || pos.asset_id.ends_with(&symbol)
        });
        if quote_updates_position {
            self.refresh_kraken_position_costs();
            self.positions_last_update_ts = chrono::Utc::now().timestamp();
        }

        // Push live mid to the watchlist for instant reactivity — but only from a
        // real-time quote. The iapi equities feed is always delayed=true; letting it
        // overwrite a watchlist row's Last with a stale mid is what made the
        // watchlist disagree with the (consolidated) chart price. Delayed symbols
        // keep the watchlist's own consolidated quote (handle_watchlist_quotes);
        // real-time WS L2 book ticks still flow via handle_kraken_book_quote_tick.
        if !weekend_closed && !ticker.delayed {
            self.apply_live_quote_to_watchlist(&symbol, ticker.bid, ticker.ask, 0.0, 0.0);
        }

        tracing::debug!(
            "Kraken equities: {} bid {} ask {} last {}{}",
            symbol,
            format_price(ticker.bid),
            format_price(ticker.ask),
            format_price(last),
            if ticker.delayed { " (delayed)" } else { "" }
        );
    }

    /// Inject fresh live bid/ask (+ optional sizes) into matching watchlist row (O(1) via index).
    /// Rich L1 polish: stores sizes for display in watchlist/tooltip when available.
    fn apply_live_quote_to_watchlist(
        &mut self,
        bare_symbol: &str,
        bid: f64,
        ask: f64,
        bid_size: f64,
        ask_size: f64,
    ) {
        if bid <= 0.0 || ask <= 0.0 {
            return;
        }
        let mid = (bid + ask) * 0.5;
        let now = std::time::Instant::now();

        if let Some(&idx) = self.watchlist_by_bare.get(bare_symbol) {
            if let Some(row) = self.watchlist_rows.get_mut(idx) {
                let row_sym = row
                    .symbol
                    .replace('/', "")
                    .trim_end_matches(".EQ")
                    .to_ascii_uppercase();
                if row_sym == bare_symbol
                    || row_sym.contains(bare_symbol)
                    || bare_symbol.contains(&row_sym)
                {
                    row.live_bid = bid;
                    row.live_ask = ask;
                    if bid_size > 0.0 {
                        row.live_bid_size = bid_size;
                    }
                    if ask_size > 0.0 {
                        row.live_ask_size = ask_size;
                    }
                    row.live_quote_at = Some(now);

                    if row.prev_close > 0.0 {
                        row.last = mid;
                        row.change = mid - row.prev_close;
                        row.change_pct = (row.change / row.prev_close) * 100.0;
                    } else {
                        row.last = mid;
                    }
                    self.watchlist_last_update_ts = chrono::Utc::now().timestamp();
                }
            }
        }
    }

    /// Rebuild O(1) indices after any mutation to charts or watchlist_rows.
    /// Called after user actions, session restore, watchlist load etc.
    /// Hot quote paths use these instead of linear scans.
    pub(super) fn rebuild_live_indices(&mut self) {
        self.chart_by_bare.clear();
        for (i, chart) in self.charts.iter().enumerate() {
            let bare = bare_symbol_from_key(&chart.symbol)
                .replace('/', "")
                .trim_end_matches(".EQ")
                .trim_end_matches(".eq")
                .to_ascii_uppercase();
            if !bare.is_empty() {
                self.chart_by_bare.entry(bare).or_default().push(i);
            }
        }

        self.watchlist_by_bare.clear();
        for (i, row) in self.watchlist_rows.iter().enumerate() {
            let bare = bare_symbol_from_key(&row.symbol)
                .replace('/', "")
                .trim_end_matches(".EQ")
                .trim_end_matches(".eq")
                .to_ascii_uppercase();
            if !bare.is_empty() {
                self.watchlist_by_bare.insert(bare, i);
            }
        }
    }
}
