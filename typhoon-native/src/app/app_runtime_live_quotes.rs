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

        let sym_norm = symbol.replace('/', "").to_ascii_uppercase();
        for chart in &mut self.charts {
            let chart_sym = chart.symbol.replace('/', "").to_ascii_uppercase();
            let mut parts = chart_sym.rsplit(':');
            let last_part = parts.next().unwrap_or(chart_sym.as_str());
            let chart_bare = if matches!(
                last_part,
                "1MIN"
                    | "5MIN"
                    | "15MIN"
                    | "30MIN"
                    | "1HOUR"
                    | "4HOUR"
                    | "1DAY"
                    | "1WEEK"
                    | "1MONTH"
            ) {
                parts.next().unwrap_or(chart_sym.as_str())
            } else {
                chart_sym.as_str()
            };
            if chart_bare == sym_norm.as_str()
                || chart_bare.contains(sym_norm.as_str())
                || sym_norm.contains(chart_bare)
            {
                chart.apply_forming_price_update(last);
            }
        }
    }

    /// Real-time Alpaca market-data tick. Updates matching charts (so the focused
    /// chart's forming bar and its position P/L go live) and the watchlist —
    /// mirrors the Kraken book-quote path so equities Kraken doesn't cover (e.g.
    /// HKIT) still get live prices instead of delayed REST.
    pub(super) fn handle_alpaca_quote(&mut self, symbol: String, bid: f64, ask: f64) {
        if bid <= 0.0 || ask <= 0.0 || !bid.is_finite() || !ask.is_finite() {
            return;
        }
        let wanted = bare_symbol_from_key(&symbol)
            .replace('/', "")
            .trim_end_matches(".EQ")
            .to_ascii_uppercase();
        for chart in &mut self.charts {
            let chart_symbol = bare_symbol_from_key(&chart.symbol)
                .replace('/', "")
                .trim_end_matches(".EQ")
                .to_ascii_uppercase();
            if chart_symbol == wanted
                || chart_symbol.contains(&wanted)
                || wanted.contains(&chart_symbol)
            {
                chart.apply_live_quote_update(bid, ask, false);
            }
        }
        self.apply_live_quote_to_watchlist(&wanted, bid, ask);
    }

    pub(super) fn handle_kraken_book_quote_tick(&mut self, symbol: String, bid: f64, ask: f64) {
        let last = (bid + ask) * 0.5;
        if last <= 0.0 || !last.is_finite() {
            return;
        }
        let wanted = bare_symbol_from_key(&symbol)
            .replace('/', "")
            .trim_end_matches(".EQ")
            .to_ascii_uppercase();
        for chart in &mut self.charts {
            let chart_symbol = bare_symbol_from_key(&chart.symbol)
                .replace('/', "")
                .trim_end_matches(".EQ")
                .to_ascii_uppercase();
            if chart_symbol == wanted
                || chart_symbol.contains(&wanted)
                || wanted.contains(&chart_symbol)
            {
                chart.apply_live_quote_update(bid, ask, false);
            }
        }
        self.apply_live_quote_to_watchlist(&wanted, bid, ask);
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

        for chart in &mut self.charts {
            let chart_sym = chart.symbol.replace('/', "").to_ascii_uppercase();
            let chart_bare = chart_sym
                .rsplit(':')
                .nth(1)
                .or_else(|| chart_sym.rsplit(':').next())
                .unwrap_or("")
                .trim_end_matches(".EQ")
                .to_string();
            if chart_bare == symbol {
                let realtime_fresh = !chart.live_quote_delayed
                    && chart
                        .live_quote_at
                        .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(30));
                if !weekend_closed && !(ticker.delayed && realtime_fresh) {
                    chart.apply_live_quote_update(ticker.bid, ticker.ask, ticker.delayed);
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
            self.apply_live_quote_to_watchlist(&symbol, ticker.bid, ticker.ask);
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

    /// Inject fresh live bid/ask into any matching watchlist row.
    /// Stores full bid/ask + timestamp so the watchlist can render Ask/Bid
    /// the same way the chart price axis does. Uses live mid for the "Last"
    /// column so change calculations stay perfectly in sync.
    fn apply_live_quote_to_watchlist(&mut self, bare_symbol: &str, bid: f64, ask: f64) {
        if bid <= 0.0 || ask <= 0.0 {
            return;
        }
        let mid = (bid + ask) * 0.5;
        let now = std::time::Instant::now();

        for row in &mut self.watchlist_rows {
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
                row.live_quote_at = Some(now);

                // Prefer live mid for the displayed Last so change calculations stay live
                if row.prev_close > 0.0 {
                    row.last = mid;
                    row.change = mid - row.prev_close;
                    row.change_pct = (row.change / row.prev_close) * 100.0;
                } else {
                    row.last = mid;
                }
                self.watchlist_last_update_ts = chrono::Utc::now().timestamp();
                break;
            }
        }
    }
}
