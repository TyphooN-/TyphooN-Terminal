use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_downside_efficiency_volatility_command(
        &mut self,
        cmd_upper: &String,
    ) -> bool {
        match cmd_upper.as_str() {
            // ── Downside risk, efficiency, and volatility palette aliases ──
            "DOWNVOL" | "DOWN_VOL" | "SEMIDEV" | "SORTINO" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.downvol_symbol = sym;
                }
                self.show_downvol = true;
                if self.downvol_snapshot.symbol.is_empty() && !self.downvol_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_downvol(
                                &conn,
                                &self.downvol_symbol,
                            ) {
                                self.downvol_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "SHARPR" | "SHARPE" | "SHARPE_RATIO" | "SHARPERATIO" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.sharpr_symbol = sym;
                }
                self.show_sharpr = true;
                if self.sharpr_snapshot.symbol.is_empty() && !self.sharpr_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_sharpr(
                                &conn,
                                &self.sharpr_symbol,
                            ) {
                                self.sharpr_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "EFFRATIO" | "EFF_RATIO" | "KAUFMAN" | "KAUFMAN_ER" | "KER" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.effratio_symbol = sym;
                }
                self.show_effratio = true;
                if self.effratio_snapshot.symbol.is_empty() && !self.effratio_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_effratio(
                                &conn,
                                &self.effratio_symbol,
                            ) {
                                self.effratio_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "WICKBIAS" | "WICK_BIAS" | "WICKS" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.wickbias_symbol = sym;
                }
                self.show_wickbias = true;
                if self.wickbias_snapshot.symbol.is_empty() && !self.wickbias_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_wickbias(
                                &conn,
                                &self.wickbias_symbol,
                            ) {
                                self.wickbias_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "VOLOFVOL" | "VOL_OF_VOL" | "VOV" | "VVOL" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.volofvol_symbol = sym;
                }
                self.show_volofvol = true;
                if self.volofvol_snapshot.symbol.is_empty() && !self.volofvol_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_volofvol(
                                &conn,
                                &self.volofvol_symbol,
                            ) {
                                self.volofvol_snapshot = snap;
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
