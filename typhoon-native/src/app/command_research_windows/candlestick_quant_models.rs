use super::*;

mod candlestick_pattern_commands;
mod macd_oscillator_extrema_commands;

impl TyphooNApp {
    pub(super) fn handle_candlestick_quant_model_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            _ if self.handle_macd_oscillator_extrema_command(cmd_upper) => {}
            _ if self.handle_candlestick_pattern_command(cmd_upper) => {}
            "DAGOSTINO" | "K2TEST" | "K2_OMNIBUS" | "DAGOSTINOPEARSON" | "DAGOSTINOWIN" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.dagostino_win_symbol = sym;
                }
                self.show_dagostino_win = true;
                if self.dagostino_win_snapshot.symbol.is_empty()
                    && !self.dagostino_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_dagostino(
                                &conn,
                                &self.dagostino_win_symbol,
                            ) {
                                self.dagostino_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "BAIPERRON" | "SUPF" | "SUP_F" | "BAI_PERRON" | "BAIPERRONWIN" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.baiperron_win_symbol = sym;
                }
                self.show_baiperron_win = true;
                if self.baiperron_win_snapshot.symbol.is_empty()
                    && !self.baiperron_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_baiperron(
                                &conn,
                                &self.baiperron_win_symbol,
                            ) {
                                self.baiperron_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "KUPIECPOF" | "KUPIEC" | "VAR_BACKTEST" | "POFTEST" | "KUPIECPOFWIN" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.kupiecpof_win_symbol = sym;
                }
                self.show_kupiecpof_win = true;
                if self.kupiecpof_win_snapshot.symbol.is_empty()
                    && !self.kupiecpof_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_kupiecpof(
                                &conn,
                                &self.kupiecpof_win_symbol,
                            ) {
                                self.kupiecpof_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            _ => return false,
        }
        true
    }
}
