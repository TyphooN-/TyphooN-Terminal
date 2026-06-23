use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_drawdown_seasonality_spread_command(
        &mut self,
        cmd_upper: &String,
    ) -> bool {
        match cmd_upper.as_str() {
            // ── Drawdown, seasonality, and spread palette aliases ──
            "OMEGA" | "OMEGA_RATIO" | "OMEGARATIO" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.omega_symbol = sym;
                }
                self.show_omega = true;
                if self.omega_snapshot.symbol.is_empty() && !self.omega_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_omega(&conn, &self.omega_symbol)
                            {
                                self.omega_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "DFA" | "DETRENDED_FLUCT" | "DETRENDED_FLUCTUATION" | "DFAALPHA" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.dfa_symbol = sym;
                }
                self.show_dfa = true;
                if self.dfa_snapshot.symbol.is_empty() && !self.dfa_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_dfa(&conn, &self.dfa_symbol)
                            {
                                self.dfa_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "BURKE" | "BURKE_RATIO" | "BURKERATIO" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.burke_symbol = sym;
                }
                self.show_burke = true;
                if self.burke_snapshot.symbol.is_empty() && !self.burke_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_burke(&conn, &self.burke_symbol)
                            {
                                self.burke_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "MONTHSEAS" | "MONTHLY_SEASONALITY" | "MONTHLYSEASONALITY" | "SEAS" | "MONTH_SEAS" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.monthseas_symbol = sym;
                }
                self.show_monthseas = true;
                if self.monthseas_snapshot.symbol.is_empty() && !self.monthseas_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_monthseas(
                                &conn,
                                &self.monthseas_symbol,
                            ) {
                                self.monthseas_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "ROLLSPRD" | "ROLL_SPREAD" | "ROLLSPREAD" | "ROLL" | "EFFECTIVE_SPREAD" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.rollsprd_symbol = sym;
                }
                self.show_rollsprd = true;
                if self.rollsprd_snapshot.symbol.is_empty() && !self.rollsprd_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_rollsprd(
                                &conn,
                                &self.rollsprd_symbol,
                            ) {
                                self.rollsprd_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            _ => false,
        }
    }
}
