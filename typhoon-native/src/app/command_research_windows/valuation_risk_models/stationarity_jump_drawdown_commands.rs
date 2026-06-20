use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_stationarity_jump_drawdown_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── Sharpe probability, stationarity, and drawdown-duration palette aliases ──
            "PSR"
            | "PROB_SHARPE"
            | "PROBSHARPE"
            | "PROBABILISTIC_SHARPE"
            | "PROBABILISTICSHARPE" => {
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
                    self.psr_symbol = sym;
                }
                self.show_psr = true;
                if self.psr_snapshot.symbol.is_empty() && !self.psr_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_psr(&conn, &self.psr_symbol)
                            {
                                self.psr_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "ADF" | "DICKEY_FULLER" | "DICKEYFULLER" | "UNIT_ROOT" | "UNITROOT"
            | "STATIONARITY" => {
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
                    self.adf_symbol = sym;
                }
                self.show_adf = true;
                if self.adf_snapshot.symbol.is_empty() && !self.adf_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_adf(&conn, &self.adf_symbol)
                            {
                                self.adf_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "MNKENDALL" | "MANN_KENDALL" | "MANNKENDALL" | "KENDALL_TREND" | "TREND_TEST" => {
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
                    self.mnkendall_symbol = sym;
                }
                self.show_mnkendall = true;
                if self.mnkendall_snapshot.symbol.is_empty() && !self.mnkendall_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_mnkendall(
                                &conn,
                                &self.mnkendall_symbol,
                            ) {
                                self.mnkendall_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "BIPOWER" | "BPV" | "BIPOWER_VAR" | "BIPOWERVAR" | "JUMP_RATIO" | "JUMPRATIO"
            | "BN_JUMP" => {
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
                    self.bipower_symbol = sym;
                }
                self.show_bipower = true;
                if self.bipower_snapshot.symbol.is_empty() && !self.bipower_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_bipower(
                                &conn,
                                &self.bipower_symbol,
                            ) {
                                self.bipower_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "DDDUR" | "DD_DURATION" | "DRAWDOWN_DURATION" | "DDDURATION" | "UNDERWATER"
            | "DRAWDOWNDURATION" => {
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
                    self.dddur_symbol = sym;
                }
                self.show_dddur = true;
                if self.dddur_snapshot.symbol.is_empty() && !self.dddur_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_dddur(&conn, &self.dddur_symbol)
                            {
                                self.dddur_snapshot = snap;
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
