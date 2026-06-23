use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_risk_adjusted_liquidity_normality_command(
        &mut self,
        cmd_upper: &String,
    ) -> bool {
        match cmd_upper.as_str() {
            // ── Risk-adjusted, liquidity, and normality palette aliases ──
            "CALMAR" | "CALMAR_RATIO" | "CALMARRATIO" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.calmar_symbol = sym;
                }
                self.show_calmar = true;
                if self.calmar_snapshot.symbol.is_empty() && !self.calmar_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_calmar(
                                &conn,
                                &self.calmar_symbol,
                            ) {
                                self.calmar_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "ULCER" | "ULCER_INDEX" | "ULCERINDEX" | "MARTIN" | "UPI" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.ulcer_symbol = sym;
                }
                self.show_ulcer = true;
                if self.ulcer_snapshot.symbol.is_empty() && !self.ulcer_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_ulcer(&conn, &self.ulcer_symbol)
                            {
                                self.ulcer_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "VARRATIO" | "VAR_RATIO" | "VARIANCE_RATIO" | "LO_MACKINLAY" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.varratio_symbol = sym;
                }
                self.show_varratio = true;
                if self.varratio_snapshot.symbol.is_empty() && !self.varratio_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_varratio(
                                &conn,
                                &self.varratio_symbol,
                            ) {
                                self.varratio_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "AMIHUD" | "AMIHUD_ILLIQ" | "ILLIQ" | "ILLIQUIDITY" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.amihud_symbol = sym;
                }
                self.show_amihud = true;
                if self.amihud_snapshot.symbol.is_empty() && !self.amihud_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_amihud(
                                &conn,
                                &self.amihud_symbol,
                            ) {
                                self.amihud_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "JBNORM" | "JB" | "JARQUE_BERA" | "NORMALITY" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.jbnorm_symbol = sym;
                }
                self.show_jbnorm = true;
                if self.jbnorm_snapshot.symbol.is_empty() && !self.jbnorm_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_jbnorm(
                                &conn,
                                &self.jbnorm_symbol,
                            ) {
                                self.jbnorm_snapshot = snap;
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
