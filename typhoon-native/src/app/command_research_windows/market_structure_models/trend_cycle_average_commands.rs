use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_trend_cycle_average_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── Trend-cycle and average palette aliases ──
            "STOCH" | "STOCHFIT" | "STOCH_WIN" | "STOCHASTIC" | "LANE_STOCH" => {
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
                    self.stoch_win_symbol = sym;
                }
                self.show_stoch_win = true;
                if self.stoch_win_snapshot.symbol.is_empty() && !self.stoch_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_stoch(
                                &conn,
                                &self.stoch_win_symbol,
                            ) {
                                self.stoch_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "MACD" | "MACDFIT" | "MACD_WIN" | "APPEL_MACD" | "MOVING_AVERAGE_CONVERGENCE" => {
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
                    self.macd_win_symbol = sym;
                }
                self.show_macd_win = true;
                if self.macd_win_snapshot.symbol.is_empty() && !self.macd_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_macd(
                                &conn,
                                &self.macd_win_symbol,
                            ) {
                                self.macd_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "VWAPFIT" | "VWAP_WIN" | "VWAP_SNAPSHOT" | "VOLUME_WEIGHTED" | "VOL_WEIGHTED_AVG" => {
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
                    self.vwap_win_symbol = sym;
                }
                self.show_vwap_win = true;
                if self.vwap_win_snapshot.symbol.is_empty() && !self.vwap_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_vwap(
                                &conn,
                                &self.vwap_win_symbol,
                            ) {
                                self.vwap_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "MCGD" | "MCGDFIT" | "MCGD_WIN" | "MCGINLEY_DYNAMIC" | "MCGINLEY" => {
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
                    self.mcgd_win_symbol = sym;
                }
                self.show_mcgd_win = true;
                if self.mcgd_win_snapshot.symbol.is_empty() && !self.mcgd_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_mcgd(
                                &conn,
                                &self.mcgd_win_symbol,
                            ) {
                                self.mcgd_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "RWI" | "RWIFIT" | "RWI_WIN" | "RANDOM_WALK" | "POULOS_RWI" => {
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
                    self.rwi_win_symbol = sym;
                }
                self.show_rwi_win = true;
                if self.rwi_win_snapshot.symbol.is_empty() && !self.rwi_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_rwi(&conn, &self.rwi_win_symbol)
                            {
                                self.rwi_win_snapshot = snap;
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
