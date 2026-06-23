use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_return_distribution_tail_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── Return distribution and tail metric palette aliases ──
            "RETSKEW" | "RET_SKEW" | "SKEWNESS" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.retskew_symbol = sym;
                }
                self.show_retskew = true;
                if self.retskew_snapshot.symbol.is_empty() && !self.retskew_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_retskew(
                                &conn,
                                &self.retskew_symbol,
                            ) {
                                self.retskew_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "RETKURT" | "RET_KURT" | "KURTOSIS" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.retkurt_symbol = sym;
                }
                self.show_retkurt = true;
                if self.retkurt_snapshot.symbol.is_empty() && !self.retkurt_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_retkurt(
                                &conn,
                                &self.retkurt_symbol,
                            ) {
                                self.retkurt_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "TAILR" | "TAIL_RATIO" | "TAILRATIO" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.tailr_symbol = sym;
                }
                self.show_tailr = true;
                if self.tailr_snapshot.symbol.is_empty() && !self.tailr_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_tailr(&conn, &self.tailr_symbol)
                            {
                                self.tailr_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "RUNLEN" | "RUN_LEN" | "RUN_LENGTH" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.runlen_symbol = sym;
                }
                self.show_runlen = true;
                if self.runlen_snapshot.symbol.is_empty() && !self.runlen_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_runlen(
                                &conn,
                                &self.runlen_symbol,
                            ) {
                                self.runlen_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "DAYRANGE" | "DAY_RANGE" | "RANGESTAT" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.dayrange_symbol = sym;
                }
                self.show_dayrange = true;
                if self.dayrange_snapshot.symbol.is_empty() && !self.dayrange_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_dayrange(
                                &conn,
                                &self.dayrange_symbol,
                            ) {
                                self.dayrange_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            // ── Serial-dependence and asymmetry metrics ──
            "AUTOCOR" | "AUTO_COR" | "ACF" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.autocor_symbol = sym;
                }
                self.show_autocor = true;
                if self.autocor_snapshot.symbol.is_empty() && !self.autocor_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_autocor(
                                &conn,
                                &self.autocor_symbol,
                            ) {
                                self.autocor_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "HURST" | "HURST_EXPONENT" | "RESCALED_RANGE" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.hurst_symbol = sym;
                }
                self.show_hurst = true;
                if self.hurst_snapshot.symbol.is_empty() && !self.hurst_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_hurst(&conn, &self.hurst_symbol)
                            {
                                self.hurst_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "HITRATE" | "HIT_RATE" | "WIN_RATE" | "WINRATE" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.hitrate_symbol = sym;
                }
                self.show_hitrate = true;
                if self.hitrate_snapshot.symbol.is_empty() && !self.hitrate_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_hitrate(
                                &conn,
                                &self.hitrate_symbol,
                            ) {
                                self.hitrate_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "GLASYM" | "GL_ASYM" | "GAIN_LOSS_ASYM" | "GAINLOSSASYM" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.glasym_symbol = sym;
                }
                self.show_glasym = true;
                if self.glasym_snapshot.symbol.is_empty() && !self.glasym_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_glasym(
                                &conn,
                                &self.glasym_symbol,
                            ) {
                                self.glasym_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "VOLRATIO" | "VOL_RATIO" | "VOLUMERATIO" | "VOLUME_RATIO" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.volratio_symbol = sym;
                }
                self.show_volratio = true;
                if self.volratio_snapshot.symbol.is_empty() && !self.volratio_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_volratio(
                                &conn,
                                &self.volratio_symbol,
                            ) {
                                self.volratio_snapshot = snap;
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
