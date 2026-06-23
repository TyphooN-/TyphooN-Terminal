use super::*;

impl TyphooNApp {
    pub(super) fn handle_linearreg_slope_commands(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── palette aliases ──
            "LINEARREG_SLOPE" | "LINREG_SLOPE" | "LINREGSLOPE" | "LRSLOPE" | "SLOPE" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.linearreg_slope_win_symbol = sym;
                }
                self.show_linearreg_slope_win = true;
                if self.linearreg_slope_win_snapshot.symbol.is_empty()
                    && !self.linearreg_slope_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_linearreg_slope(
                                    &conn,
                                    &self.linearreg_slope_win_symbol,
                                )
                            {
                                self.linearreg_slope_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HT_DCPERIOD" | "HTDCPERIOD" | "DCPERIOD" | "HILBERT_PERIOD" | "CYCLE_PERIOD" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.ht_dcperiod_win_symbol = sym;
                }
                self.show_ht_dcperiod_win = true;
                if self.ht_dcperiod_win_snapshot.symbol.is_empty()
                    && !self.ht_dcperiod_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_dcperiod(
                                &conn,
                                &self.ht_dcperiod_win_symbol,
                            ) {
                                self.ht_dcperiod_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HT_TRENDMODE" | "HTTRENDMODE" | "TRENDMODE" | "HILBERT_TRENDMODE"
            | "CYCLE_TRENDMODE" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.ht_trendmode_win_symbol = sym;
                }
                self.show_ht_trendmode_win = true;
                if self.ht_trendmode_win_snapshot.symbol.is_empty()
                    && !self.ht_trendmode_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_trendmode(
                                &conn,
                                &self.ht_trendmode_win_symbol,
                            ) {
                                self.ht_trendmode_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ACCBANDS" | "ACCELERATION_BANDS" | "ACCBAND" | "HEADLEY" | "ACC_BANDS" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.accbands_win_symbol = sym;
                }
                self.show_accbands_win = true;
                if self.accbands_win_snapshot.symbol.is_empty()
                    && !self.accbands_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_accbands(
                                &conn,
                                &self.accbands_win_symbol,
                            ) {
                                self.accbands_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "STOCHF" | "STOCHFAST" | "FAST_STOCH" | "FASTSTOCH" | "STOCH_FAST" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.stochf_win_symbol = sym;
                }
                self.show_stochf_win = true;
                if self.stochf_win_snapshot.symbol.is_empty() && !self.stochf_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_stochf(
                                &conn,
                                &self.stochf_win_symbol,
                            ) {
                                self.stochf_win_snapshot = snap;
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
