use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_reward_risk_serial_liquidity_command(
        &mut self,
        cmd_upper: &String,
    ) -> bool {
        match cmd_upper.as_str() {
            // ── Reward-risk, serial-dependence, and zero-return palette aliases ──
            "STERLING" | "STERLING_RATIO" | "STERLINGRATIO" => {
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
                    self.sterling_symbol = sym;
                }
                self.show_sterling = true;
                if self.sterling_snapshot.symbol.is_empty() && !self.sterling_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_sterling(
                                &conn,
                                &self.sterling_symbol,
                            ) {
                                self.sterling_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "KELLYF" | "KELLY" | "KELLY_FRACTION" | "KELLY_CRITERION" | "OPTIMAL_F" => {
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
                    self.kellyf_symbol = sym;
                }
                self.show_kellyf = true;
                if self.kellyf_snapshot.symbol.is_empty() && !self.kellyf_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_kellyf(
                                &conn,
                                &self.kellyf_symbol,
                            ) {
                                self.kellyf_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "LJUNGB" | "LJUNG_BOX" | "LJUNGBOX" | "PORTMANTEAU" | "QSTAT" | "Q_STAT" => {
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
                    self.ljungb_symbol = sym;
                }
                self.show_ljungb = true;
                if self.ljungb_snapshot.symbol.is_empty() && !self.ljungb_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ljungb(
                                &conn,
                                &self.ljungb_symbol,
                            ) {
                                self.ljungb_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "RUNSTEST" | "RUNS_TEST" | "WALD_WOLFOWITZ" | "WW_RUNS" | "SIGN_RUNS" => {
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
                    self.runstest_symbol = sym;
                }
                self.show_runstest = true;
                if self.runstest_snapshot.symbol.is_empty() && !self.runstest_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_runstest(
                                &conn,
                                &self.runstest_symbol,
                            ) {
                                self.runstest_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "ZERORET" | "ZERO_RETURN" | "LOT" | "LESMOND" | "ZERO_DAYS" | "ZERODAYS" => {
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
                    self.zeroret_symbol = sym;
                }
                self.show_zeroret = true;
                if self.zeroret_snapshot.symbol.is_empty() && !self.zeroret_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_zeroret(
                                &conn,
                                &self.zeroret_symbol,
                            ) {
                                self.zeroret_snapshot = snap;
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
