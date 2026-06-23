use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_upside_leverage_concentration_command(
        &mut self,
        cmd_upper: &String,
    ) -> bool {
        match cmd_upper.as_str() {
            // ── Upside, leverage asymmetry, drawdown-at-risk, and concentration palette aliases ──
            "UPR" | "UPSIDE_POTENTIAL" | "UPSIDEPOTENTIAL" | "UPSIDE_RATIO" | "UPSIDERATIO" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.upr_symbol = sym;
                }
                self.show_upr = true;
                if self.upr_snapshot.symbol.is_empty() && !self.upr_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_upr(&conn, &self.upr_symbol)
                            {
                                self.upr_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "LEVEREFF" | "LEVERAGE_EFFECT" | "LEVERAGEEFFECT" | "LEVER_EFF" | "ASYM_VOL"
            | "ASYMVOL" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.levereff_symbol = sym;
                }
                self.show_levereff = true;
                if self.levereff_snapshot.symbol.is_empty() && !self.levereff_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_levereff(
                                &conn,
                                &self.levereff_symbol,
                            ) {
                                self.levereff_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "DRAWDAR" | "DRAWDOWN_AT_RISK" | "DRAWDOWNATRISK" | "DAR" | "CDAR"
            | "CONDITIONAL_DAR" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.drawdar_symbol = sym;
                }
                self.show_drawdar = true;
                if self.drawdar_snapshot.symbol.is_empty() && !self.drawdar_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_drawdar(
                                &conn,
                                &self.drawdar_symbol,
                            ) {
                                self.drawdar_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "VARHALF"
            | "VOL_HALFLIFE"
            | "VOLHALFLIFE"
            | "VOL_PERSIST"
            | "VOLPERSIST"
            | "VOLATILITY_HALFLIFE" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.varhalf_symbol = sym;
                }
                self.show_varhalf = true;
                if self.varhalf_snapshot.symbol.is_empty() && !self.varhalf_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_varhalf(
                                &conn,
                                &self.varhalf_symbol,
                            ) {
                                self.varhalf_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "GINI" | "GINI_COEFF" | "GINICOEFF" | "GINI_COEFFICIENT" | "RETURN_CONCENTRATION" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.gini_symbol = sym;
                }
                self.show_gini = true;
                if self.gini_snapshot.symbol.is_empty() && !self.gini_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_gini(&conn, &self.gini_symbol)
                            {
                                self.gini_snapshot = snap;
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
