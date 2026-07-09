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
