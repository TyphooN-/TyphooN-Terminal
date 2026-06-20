use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_fractal_tail_dependence_commands_command(
        &mut self,
        cmd_upper: &String,
    ) -> bool {
        match cmd_upper.as_str() {
            // ── Fractal and tail-dependence palette aliases ──
            "HIGUCHI" | "HIGUCHI_FD" | "FRACTAL_DIM" | "FRACTALDIM" | "HFD" => {
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
                    self.higuchi_symbol = sym;
                }
                self.show_higuchi = true;
                if self.higuchi_snapshot.symbol.is_empty() && !self.higuchi_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_higuchi(
                                &conn,
                                &self.higuchi_symbol,
                            ) {
                                self.higuchi_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "PICKANDS" | "PICKANDS_TAIL" | "TAIL_INDEX_P" | "PICKANDSTAIL" => {
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
                    self.pickands_symbol = sym;
                }
                self.show_pickands = true;
                if self.pickands_snapshot.symbol.is_empty() && !self.pickands_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_pickands(
                                &conn,
                                &self.pickands_symbol,
                            ) {
                                self.pickands_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "KAPPA3" | "KAPPA_3" | "KAPPA3RATIO" | "KAPPA3_RATIO" | "KAPLAN_KNOWLES" => {
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
                    self.kappa3_symbol = sym;
                }
                self.show_kappa3 = true;
                if self.kappa3_snapshot.symbol.is_empty() && !self.kappa3_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_kappa3(
                                &conn,
                                &self.kappa3_symbol,
                            ) {
                                self.kappa3_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "LYAPUNOV" | "LYAPUNOV_EXP" | "LAMBDA_MAX" | "LYAPUNOVEXPONENT" | "ROSENSTEIN" => {
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
                    self.lyapunov_symbol = sym;
                }
                self.show_lyapunov = true;
                if self.lyapunov_snapshot.symbol.is_empty() && !self.lyapunov_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_lyapunov(
                                &conn,
                                &self.lyapunov_symbol,
                            ) {
                                self.lyapunov_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "RANKAC" | "RANK_AUTOCORR" | "SPEARMAN_AC" | "RANKAUTOCORRELATION" | "SPEARMANLAGS" => {
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
                    self.rankac_symbol = sym;
                }
                self.show_rankac = true;
                if self.rankac_snapshot.symbol.is_empty() && !self.rankac_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_rankac(
                                &conn,
                                &self.rankac_symbol,
                            ) {
                                self.rankac_snapshot = snap;
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
