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
    }

    pub(super) fn handle_kraken_equity_quote(
        &mut self,
        ticker: typhoon_engine::broker::kraken::KrakenEquityTicker,
    ) {
        if !self.kraken_enabled {
            return;
        }
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

        // Do not write quote bars from the egui thread. During SEC/news sweeps
        // SQLite can be write-locked for seconds; a single KrakenEquityQuote then
        // blew the broker drain budget and froze free-look. History fetches still
        // persist quote/history bars on blocking workers; this path is just the
        // live UI overlay.
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
                // A delayed iapi equity quote (~15 min) must not clobber a fresh
                // real-time WS quote. Adopt it only when no recent real-time quote
                // exists (cold start or the WS feed went quiet); real-time quotes
                // (delayed=false) always win. Keeps the chart spread and last
                // matching the live tape during CORE.
                let realtime_fresh = !chart.live_quote_delayed
                    && chart
                        .live_quote_at
                        .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(30));
                if !(ticker.delayed && realtime_fresh) {
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
            // The position row's quantity/cost basis only changes on balance/trade
            // events, but its displayed current price and P/L are quote-driven.
            // If a held xStock quote updates the chart/watchlist, mark Positions
            // fresh too; otherwise the header says "3m" while the row is already
            // priced from this same tick.
            self.positions_last_update_ts = chrono::Utc::now().timestamp();
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
}
