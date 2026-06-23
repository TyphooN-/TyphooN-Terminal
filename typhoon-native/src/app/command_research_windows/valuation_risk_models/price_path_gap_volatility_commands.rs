use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_price_path_gap_volatility_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── Price-path, gap, and volatility-cluster palette aliases ──
            "DRAWUP" | "DRAW_UP" | "RALLYHIST" | "RALLY_HISTORY" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.drawup_symbol = sym;
                }
                self.show_drawup = true;
                if self.drawup_snapshot.symbol.is_empty() && !self.drawup_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_drawup(
                                &conn,
                                &self.drawup_symbol,
                            ) {
                                self.drawup_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "GAPSTATS" | "GAP_STATS" | "GAP" | "OVERNIGHT_GAP" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.gapstats_symbol = sym;
                }
                self.show_gapstats = true;
                if self.gapstats_snapshot.symbol.is_empty() && !self.gapstats_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_gapstats(
                                &conn,
                                &self.gapstats_symbol,
                            ) {
                                self.gapstats_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "VOLCLUSTER" | "VOL_CLUSTER" | "ARCH" | "VOLATILITYCLUSTER" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.volcluster_symbol = sym;
                }
                self.show_volcluster = true;
                if self.volcluster_snapshot.symbol.is_empty() && !self.volcluster_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_volcluster(
                                &conn,
                                &self.volcluster_symbol,
                            ) {
                                self.volcluster_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "CLOSEPLC" | "CLOSE_PLC" | "CLOSEPLACEMENT" | "CLOSE_PLACEMENT" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.closeplc_symbol = sym;
                }
                self.show_closeplc = true;
                if self.closeplc_snapshot.symbol.is_empty() && !self.closeplc_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_closeplc(
                                &conn,
                                &self.closeplc_symbol,
                            ) {
                                self.closeplc_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "MRHL" | "HALF_LIFE" | "HALFLIFE" | "AR1" | "MEAN_REVERT_HL" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.mrhl_symbol = sym;
                }
                self.show_mrhl = true;
                if self.mrhl_snapshot.symbol.is_empty() && !self.mrhl_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_mrhl(&conn, &self.mrhl_symbol)
                            {
                                self.mrhl_snapshot = snap;
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
