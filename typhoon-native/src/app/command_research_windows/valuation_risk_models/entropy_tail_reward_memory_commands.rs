use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_entropy_tail_reward_memory_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── Entropy, tail reward, and serial-memory palette aliases ──
            "ENTROPY" | "SHANNON" | "SHANNON_ENTROPY" | "SHANNONENTROPY" | "RETURN_ENTROPY"
            | "RETURNENTROPY" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.entropy_symbol = sym;
                }
                self.show_entropy = true;
                if self.entropy_snapshot.symbol.is_empty() && !self.entropy_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_entropy(
                                &conn,
                                &self.entropy_symbol,
                            ) {
                                self.entropy_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "RACHEV"
            | "RACHEV_RATIO"
            | "RACHEVRATIO"
            | "ETL_RATIO"
            | "ETLRATIO"
            | "TAIL_EXPECTATION_RATIO" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.rachev_symbol = sym;
                }
                self.show_rachev = true;
                if self.rachev_snapshot.symbol.is_empty() && !self.rachev_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_rachev(
                                &conn,
                                &self.rachev_symbol,
                            ) {
                                self.rachev_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "GPR" | "GAIN_TO_PAIN" | "GAINTOPAIN" | "GAIN_PAIN" | "GAINPAIN" | "PROFIT_FACTOR"
            | "PROFITFACTOR" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.gpr_symbol = sym;
                }
                self.show_gpr = true;
                if self.gpr_snapshot.symbol.is_empty() && !self.gpr_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_gpr(&conn, &self.gpr_symbol)
                            {
                                self.gpr_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "PACF"
            | "PARTIAL_ACF"
            | "PARTIALACF"
            | "PARTIAL_AUTOCORRELATION"
            | "PARTIALAUTOCORRELATION"
            | "PACF_LAG" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.pacf_symbol = sym;
                }
                self.show_pacf = true;
                if self.pacf_snapshot.symbol.is_empty() && !self.pacf_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_pacf(&conn, &self.pacf_symbol)
                            {
                                self.pacf_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "APEN"
            | "APPROX_ENTROPY"
            | "APPROXENTROPY"
            | "APPROXIMATE_ENTROPY"
            | "APPROXIMATEENTROPY"
            | "PINCUS" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.apen_symbol = sym;
                }
                self.show_apen = true;
                if self.apen_snapshot.symbol.is_empty() && !self.apen_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_apen(&conn, &self.apen_symbol)
                            {
                                self.apen_snapshot = snap;
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
