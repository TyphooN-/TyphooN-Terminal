use super::*;

impl TyphooNApp {
    pub(super) fn handle_macro_alt_data_msg(&mut self, msg: BrokerMsg) {
        match msg {
            BrokerMsg::FredData(series, yields) => {
                self.fred_data = series;
                self.fred_yield_curve = yields;
                self.log.push_back(LogEntry::info(format!(
                    "FRED: {} series loaded",
                    self.fred_data.len()
                )));
                // ADR-094: Chart result card for first FRED series
                if let Some(first) = self.fred_data.first() {
                    if !first.observations.is_empty() {
                        let vals: Vec<f64> = first.observations.iter().map(|o| o.value).collect();
                        self.result_card = Some((
                            ResultCard::Chart {
                                title: format!("FRED: {}", first.title),
                                label: first.id.clone(),
                                values: vals,
                            },
                            std::time::Instant::now(),
                        ));
                    }
                }
            }
            BrokerMsg::EconCalendarData(events) => {
                self.econ_events = events;
                self.econ_last_fetch_ts = chrono::Utc::now().timestamp();
                self.log.push_back(LogEntry::info(format!(
                    "Economic calendar: {} events loaded",
                    self.econ_events.len()
                )));
            }
            BrokerMsg::CongressData(trades) => {
                self.congress_trades = trades;
                // PERF: normalize ticker to uppercase once so per-frame scope filter
                // skips the alloc on every render.
                for row in &mut self.congress_trades {
                    row.2.make_ascii_uppercase();
                }
                self.log.push_back(LogEntry::info(format!(
                    "Congressional trades: {} loaded",
                    self.congress_trades.len()
                )));
            }
            _ => {}
        }
    }
}
