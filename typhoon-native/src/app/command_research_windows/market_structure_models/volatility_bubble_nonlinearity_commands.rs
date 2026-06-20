use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_volatility_bubble_nonlinearity_command(
        &mut self,
        cmd_upper: &String,
    ) -> bool {
        match cmd_upper.as_str() {
            // ── Volatility, bubble, and nonlinearity palette aliases ──
            "GARCH11" | "GARCH" | "GARCH_11" | "BOLLERSLEV" | "CONDVOL" | "CONDITIONAL_VOL" => {
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
                    self.garch11_symbol = sym;
                }
                self.show_garch11 = true;
                if self.garch11_snapshot.symbol.is_empty() && !self.garch11_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_garch11(
                                &conn,
                                &self.garch11_symbol,
                            ) {
                                self.garch11_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "SADF" | "SUP_ADF" | "SUPADF" | "BUBBLETEST" | "BUBBLE_TEST" | "PWY"
            | "PHILLIPS_WU_YU" => {
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
                    self.sadf_symbol = sym;
                }
                self.show_sadf = true;
                if self.sadf_snapshot.symbol.is_empty() && !self.sadf_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_sadf(&conn, &self.sadf_symbol)
                            {
                                self.sadf_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "CORDIM" | "CORR_DIM" | "CORRDIM" | "D2" | "GRASSBERGER" | "GRASSBERGER_PROCACCIA" => {
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
                    self.cordim_symbol = sym;
                }
                self.show_cordim = true;
                if self.cordim_snapshot.symbol.is_empty() && !self.cordim_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cordim(
                                &conn,
                                &self.cordim_symbol,
                            ) {
                                self.cordim_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "SKSPEC" | "SKEW_SPEC" | "ROLLING_SKEW" | "SKEWSPECTRUM" | "SKEWSTAB"
            | "SKEWSTABILITY" => {
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
                    self.skspec_symbol = sym;
                }
                self.show_skspec = true;
                if self.skspec_snapshot.symbol.is_empty() && !self.skspec_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_skspec(
                                &conn,
                                &self.skspec_symbol,
                            ) {
                                self.skspec_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "AUTOMI" | "AUTO_MI" | "MUTUALINFO" | "MUTUAL_INFORMATION" | "MI_ACF"
            | "INFOTHEOACF" => {
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
                    self.automi_symbol = sym;
                }
                self.show_automi = true;
                if self.automi_snapshot.symbol.is_empty() && !self.automi_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_automi(
                                &conn,
                                &self.automi_symbol,
                            ) {
                                self.automi_snapshot = snap;
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
