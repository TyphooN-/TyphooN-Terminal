use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_trade_order_command(&mut self, cmd_upper: &str) -> bool {
        match cmd_upper {
            "OPEN_TRADE" => {
                self.submit_quick_trade();
            }
            "EXPORT_CALENDAR" => {
                if self.event_calendar_rows.is_empty() {
                    self.log
                        .push_back(LogEntry::warn("No events loaded — open CALENDAR first"));
                } else {
                    let ics = Self::build_events_ics(
                        &self.event_calendar_rows,
                        self.event_filter_source,
                        true,
                        true,
                        true,
                    );
                    let mut path = dirs_home();
                    path.push("export");
                    let _ = std::fs::create_dir_all(&path);
                    path.push("typhoon_events.ics");
                    match std::fs::write(&path, &ics) {
                        Ok(_) => self.log.push_back(LogEntry::info(format!(
                            "Calendar exported: {} ({} bytes)",
                            path.display(),
                            ics.len()
                        ))),
                        Err(e) => self
                            .log
                            .push_back(LogEntry::err(format!("ICS export failed: {e}"))),
                    }
                }
            }
            cmd if cmd.starts_with("BACKTEST_EXPAND") => {
                let rest = cmd.trim_start_matches("BACKTEST_EXPAND").trim();
                if rest.is_empty() {
                    if self.mt5_backtest_expand_symbols.is_empty() {
                        self.log.push_back(LogEntry::info(
                            "backtest_expand: empty. Usage: BACKTEST_EXPAND EURUSD [bars]  (compatibility override; provider-max MT5 sync is already the default)"));
                    } else {
                        let mut list: Vec<(String, u32)> = self
                            .mt5_backtest_expand_symbols
                            .iter()
                            .map(|(k, v)| (k.clone(), *v))
                            .collect();
                        list.sort_by(|a, b| a.0.cmp(&b.0));
                        let shown = list
                            .iter()
                            .map(|(s, n)| format!("{}={}", s, n))
                            .collect::<Vec<_>>()
                            .join(", ");
                        self.log
                            .push_back(LogEntry::info(format!("backtest_expand map: {}", shown)));
                    }
                } else {
                    let parts: Vec<&str> = rest.split_whitespace().collect();
                    let sym = parts[0].to_uppercase();
                    // MT5 sync already asks for provider-maximum history by default.
                    // Keep this command as a compatibility knob for old saved sessions/manual
                    // experiments, but never let it shrink below the provider-max sentinel.
                    let default_bars: u32 = MT5_PROVIDER_MAX_BARS;
                    let cap: u32 = MT5_PROVIDER_MAX_BARS;
                    let bars: u32 = if parts.len() >= 2 {
                        parts[1].parse::<u32>().unwrap_or(default_bars).min(cap)
                    } else {
                        default_bars
                    };
                    self.mt5_backtest_expand_symbols.insert(sym.clone(), bars);
                    self.log.push_back(LogEntry::info(format!(
                        "backtest_expand: {} → {} bars (overrides tiered default on gap-fill requests)",
                        sym, bars)));
                    self.detect_mt5_gaps();
                    self.flush_mt5_demand_txt(true);
                }
            }
            cmd if cmd.starts_with("BACKTEST_UNEXPAND") => {
                let rest = cmd.trim_start_matches("BACKTEST_UNEXPAND").trim();
                if rest.is_empty() {
                    self.log
                        .push_back(LogEntry::warn("Usage: BACKTEST_UNEXPAND EURUSD"));
                } else {
                    let sym = rest.to_uppercase();
                    if self.mt5_backtest_expand_symbols.remove(&sym).is_some() {
                        self.log.push_back(LogEntry::info(format!(
                            "backtest_expand: removed {} — provider-max MT5 sync remains the default",
                            sym
                        )));
                    } else {
                        self.log.push_back(LogEntry::info(format!(
                            "backtest_expand: {} not in set",
                            sym
                        )));
                    }
                }
            }
            cmd if cmd.starts_with("OCO ") => {
                // OCO SELL AAPL 10 200.00 180.00
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.len() >= 6 {
                    let side = parts[1].to_lowercase();
                    let symbol = parts[2].to_string();
                    let qty: f64 = parts[3].parse().unwrap_or(0.0);
                    let tp: f64 = parts[4].parse().unwrap_or(0.0);
                    let sl: f64 = parts[5].parse().unwrap_or(0.0);
                    if qty > 0.0 && tp > 0.0 && sl > 0.0 {
                        let _ = self.broker_tx.send(BrokerCmd::AlpacaOcoOrder {
                            symbol: symbol.clone(),
                            qty,
                            side: side.clone(),
                            tp_price: tp,
                            sl_price: sl,
                        });
                        self.log.push_back(LogEntry::info(format!(
                            "OCO {} {} {} TP:{} SL:{}",
                            side, qty, symbol, tp, sl
                        )));
                    } else {
                        self.log.push_back(LogEntry::warn(
                            "Invalid OCO params — need positive qty, TP, SL",
                        ));
                    }
                } else {
                    self.log
                        .push_back(LogEntry::warn("Usage: OCO SELL AAPL 10 200.00 180.00"));
                }
            }
            "CLOSE_ALL" => {
                self.close_all_selected_brokers();
            }
            "CLOSE_PARTIAL" => {
                self.close_partial_active_symbol();
            }
            "SET_SL" => {
                // Use last close price as initial SL, then user can drag
                if let Some(chart) = self.charts.get(self.active_tab) {
                    if let Some(last) = chart.bars.last() {
                        let sl = last.close * 0.98; // default: 2% below current price
                        self.sl_price = Some(sl);
                        self.sl_enabled = true;
                        self.sync_trade_line_inputs();
                        self.log.push_back(LogEntry::info(format!(
                            "SL set at {} — drag to adjust",
                            format_price(sl)
                        )));
                    }
                }
            }
            "SET_TP" => {
                if let Some(chart) = self.charts.get(self.active_tab) {
                    if let Some(last) = chart.bars.last() {
                        let tp = last.close * 1.04; // default: 4% above current price
                        self.tp_price = Some(tp);
                        self.tp_enabled = true;
                        self.sync_trade_line_inputs();
                        self.log.push_back(LogEntry::info(format!(
                            "TP set at {} — drag to adjust",
                            format_price(tp)
                        )));
                    }
                }
            }
            "OPEN_MG" => {
                if self.broker_connected {
                    self.log.push_back(LogEntry::info(
                        "Martingale: use chart SL/TP lines and the broker-backed Open MG flow",
                    ));
                } else {
                    self.log
                        .push_back(LogEntry::warn("Connect to broker first"));
                }
            }
            "BUY_LINES" | "SELL_LINES" => {
                let is_buy = cmd_upper == "BUY_LINES";
                match self.set_visible_range_trade_lines(is_buy) {
                    Ok((sl, tp)) => {
                        self.log.push_back(LogEntry::info(format!(
                            "{}: SL {} TP {} (drag to adjust)",
                            if is_buy { "Buy Lines" } else { "Sell Lines" },
                            format_price(sl),
                            format_price(tp)
                        )));
                    }
                    Err(e) => self.log.push_back(LogEntry::warn(e)),
                }
            }
            _ => return false,
        }
        true
    }
}
