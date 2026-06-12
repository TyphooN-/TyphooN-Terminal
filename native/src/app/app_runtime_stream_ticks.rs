use super::*;

impl TyphooNApp {
    pub(super) fn handle_stream_tick_msg(&mut self, msg: BrokerMsg) {
        match msg {
            BrokerMsg::UnusualVolumeResults(results) => {
                self.log.push_back(LogEntry::info(format!(
                    "Unusual volume: {} symbols flagged",
                    results.len()
                )));
                self.unusual_volume_results = results;
            }
            BrokerMsg::MarketClock(msg) => {
                self.market_clock_status = msg.clone();
                self.log.push_back(LogEntry::info(msg));
            }
            BrokerMsg::StreamTick {
                symbol,
                price,
                size,
                timestamp,
            } => self.handle_stream_trade_tick(symbol, price, size, timestamp),
            BrokerMsg::StreamQuoteTick { symbol, bid, ask } => {
                self.handle_stream_quote_tick(symbol, bid, ask);
            }
            _ => {}
        }
    }

    fn handle_stream_trade_tick(
        &mut self,
        symbol: String,
        price: f64,
        size: f64,
        timestamp: String,
    ) {
        // TAS tape — keep up to 500 most-recent trades for the current TAS subscription.
        if self.show_tas
            && !self.tas_paused
            && !self.tas_symbol.is_empty()
            && (symbol.eq_ignore_ascii_case(&self.tas_symbol)
                || self.tas_symbol.contains(&symbol)
                || symbol.contains(&self.tas_symbol))
        {
            // Infer side from previous-tick comparison on the same symbol.
            let side = if let Some((_, prev_px, _, _, _)) = self.tas_rows.front() {
                if price > *prev_px {
                    "buy"
                } else if price < *prev_px {
                    "sell"
                } else {
                    "flat"
                }
            } else {
                "flat"
            };
            self.tas_rows.push_front((
                symbol.clone(),
                price,
                size,
                side.to_string(),
                timestamp.clone(),
            ));
            while self.tas_rows.len() > 500 {
                self.tas_rows.pop_back();
            }
        }

        // Feed into BarBuilder for real-time bar construction.
        if let Ok(mut bb) = self.bar_builder.lock() {
            bb.ingest_trade(&symbol, price, size, &timestamp);
            // Drain completed bars and append to matching charts.
            let completed = bb.drain_completed();
            for bar in completed {
                for chart in &mut self.charts {
                    if chart_matches_stream_bar(&chart.symbol, &bar.symbol) {
                        chart.bars.push(Bar {
                            ts_ms: chrono::DateTime::parse_from_rfc3339(&bar.timestamp)
                                .map(|dt| dt.timestamp_millis())
                                .unwrap_or(0),
                            open: bar.open,
                            high: bar.high,
                            low: bar.low,
                            close: bar.close,
                            volume: bar.volume,
                        });
                        // Advance view offset if following latest.
                        if self.follow_latest
                            && !chart.manual_view_override
                            && chart.view_offset >= chart.bars.len().saturating_sub(2)
                        {
                            chart.view_offset = chart.bars.len().saturating_sub(1) + 20;
                        }
                    }
                }
            }
        }
    }

    fn handle_stream_quote_tick(&mut self, symbol: String, bid: f64, ask: f64) {
        // Update forming bar close price + live bid/ask on matching charts.
        let last = (bid + ask) / 2.0;
        if last > 0.0 {
            // Live quotes stored in-memory only (chart.live_bid/ask).
            // Removed per-tick KV writes: 851 symbols × zstd compress + SQLite INSERT
            // was burning hundreds of SSD writes/sec during market hours.
            for chart in &mut self.charts {
                if chart.symbol.contains(&symbol) {
                    chart.apply_live_quote_update(bid, ask, false);
                }
            }
        }
    }
}

fn chart_matches_stream_bar(chart_symbol: &str, bar_symbol: &str) -> bool {
    if chart_symbol.contains(bar_symbol) {
        return true;
    }

    // Chart symbols often look like "provider:SYMBOL:TF". The old hot path
    // split the string twice per completed bar/chart. One rsplit gives the
    // timeframe and symbol candidate without allocations or a second scan.
    let mut parts = chart_symbol.rsplit(':');
    let last = parts.next().unwrap_or(chart_symbol);
    let candidate = parts.next().unwrap_or(last);
    !candidate.is_empty() && bar_symbol.contains(candidate)
}
