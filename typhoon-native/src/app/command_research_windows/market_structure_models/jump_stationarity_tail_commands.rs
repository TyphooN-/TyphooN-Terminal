use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_jump_stationarity_tail_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── Jump, stationarity, and tail palette aliases ──
            "BNSJUMP" | "BNS_JUMP" | "JUMPTEST" | "JUMP_TEST" | "BARNDORFF" | "BIPOWERJUMP" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.bnsjump_symbol = sym;
                }
                self.show_bnsjump = true;
                if self.bnsjump_snapshot.symbol.is_empty() && !self.bnsjump_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_bnsjump(
                                &conn,
                                &self.bnsjump_symbol,
                            ) {
                                self.bnsjump_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "PPROOT" | "PHILLIPS_PERRON" | "PHILLIPSPERRON" | "PP_TEST" | "PPTEST"
            | "UNITROOTPP" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.pproot_symbol = sym;
                }
                self.show_pproot = true;
                if self.pproot_snapshot.symbol.is_empty() && !self.pproot_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_pproot(
                                &conn,
                                &self.pproot_symbol,
                            ) {
                                self.pproot_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "MFDFA" | "MF_DFA" | "MULTIFRACTAL" | "MULTIFRACTALDFA" | "MFSPECTRUM" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.mfdfa_symbol = sym;
                }
                self.show_mfdfa = true;
                if self.mfdfa_snapshot.symbol.is_empty() && !self.mfdfa_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_mfdfa(&conn, &self.mfdfa_symbol)
                            {
                                self.mfdfa_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "HILLKS" | "HILL_KS" | "PARETO_KS" | "TAILFIT" | "HILLTAILFIT" | "HILLGOF" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.hillks_symbol = sym;
                }
                self.show_hillks = true;
                if self.hillks_snapshot.symbol.is_empty() && !self.hillks_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_hillks(
                                &conn,
                                &self.hillks_symbol,
                            ) {
                                self.hillks_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "TSI" | "TRUE_STRENGTH" | "TRUESTRENGTHINDEX" | "BLAU_TSI" | "BLAUINDEX"
            | "MOMENTUMTSI" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.tsi_symbol = sym;
                }
                self.show_tsi = true;
                if self.tsi_snapshot.symbol.is_empty() && !self.tsi_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_tsi(&conn, &self.tsi_symbol)
                            {
                                self.tsi_snapshot = snap;
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
