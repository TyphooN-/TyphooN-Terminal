use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_volume_choppiness_moving_average_command(
        &mut self,
        cmd_upper: &String,
    ) -> bool {
        match cmd_upper.as_str() {
            // ── Volume, choppiness, and moving-average palette aliases ──
            // Bare OBV and HMA collide with chart-overlay toggles upstream;
            // only disambiguated forms are used for OBV/HMA research windows.
            // Bare VORTEX, CHOP, TRIX are unbound and kept as aliases.
            "VORTEX" | "VORTEXFIT" | "VORTEX_WIN" | "VI" | "VI_14" | "BOTES_SIEPMAN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.vortex_win_symbol = sym;
                }
                self.show_vortex_win = true;
                if self.vortex_win_snapshot.symbol.is_empty() && !self.vortex_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_vortex(
                                &conn,
                                &self.vortex_win_symbol,
                            ) {
                                self.vortex_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "CHOP" | "CHOPFIT" | "CHOP_WIN" | "CHOPPINESS" | "CHOPPINESS_INDEX" | "DREISS" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.chop_win_symbol = sym;
                }
                self.show_chop_win = true;
                if self.chop_win_snapshot.symbol.is_empty() && !self.chop_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_chop(
                                &conn,
                                &self.chop_win_symbol,
                            ) {
                                self.chop_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "OBVFIT" | "OBV_WIN" | "OBVREG" | "GRANVILLE_OBV" | "ONBALANCE_VOLUME" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.obv_win_symbol = sym;
                }
                self.show_obv_win = true;
                if self.obv_win_snapshot.symbol.is_empty() && !self.obv_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_obv(&conn, &self.obv_win_symbol)
                            {
                                self.obv_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "TRIX" | "TRIXFIT" | "TRIX_WIN" | "TRIPLE_EMA" | "HUTSON_TRIX" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.trix_win_symbol = sym;
                }
                self.show_trix_win = true;
                if self.trix_win_snapshot.symbol.is_empty() && !self.trix_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_trix(
                                &conn,
                                &self.trix_win_symbol,
                            ) {
                                self.trix_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "HMAFIT" | "HMA_WIN" | "HMAREG" | "HULL_MA" | "HULL_MOVING_AVG" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.hma_win_symbol = sym;
                }
                self.show_hma_win = true;
                if self.hma_win_snapshot.symbol.is_empty() && !self.hma_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_hma(&conn, &self.hma_win_symbol)
                            {
                                self.hma_win_snapshot = snap;
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
