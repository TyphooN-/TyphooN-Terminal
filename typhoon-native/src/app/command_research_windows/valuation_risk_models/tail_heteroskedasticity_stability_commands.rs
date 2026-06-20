use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_tail_heteroskedasticity_stability_command(
        &mut self,
        cmd_upper: &String,
    ) -> bool {
        match cmd_upper.as_str() {
            // ── Tail risk, heteroskedasticity, and structural-stability palette aliases ──
            "HILLTAIL" | "HILL" | "HILL_TAIL" | "TAIL_INDEX" | "TAILINDEX" | "HILLESTIMATOR"
            | "POWER_LAW_TAIL" => {
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
                    self.hilltail_symbol = sym;
                }
                self.show_hilltail = true;
                if self.hilltail_snapshot.symbol.is_empty() && !self.hilltail_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_hilltail(
                                &conn,
                                &self.hilltail_symbol,
                            ) {
                                self.hilltail_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "ARCHLM" | "ARCH_LM" | "ENGLE_ARCH" | "ARCH_TEST" | "HETEROSKEDASTIC"
            | "HETERO_TEST" | "VOLCLUSTER_TEST" => {
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
                    self.archlm_symbol = sym;
                }
                self.show_archlm = true;
                if self.archlm_snapshot.symbol.is_empty() && !self.archlm_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_archlm(
                                &conn,
                                &self.archlm_symbol,
                            ) {
                                self.archlm_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "PAINRATIO" | "PAIN_RATIO" | "PAIN_INDEX" | "PAININDEX" | "PAIN" | "ZEPHYR_PAIN" => {
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
                    self.painratio_symbol = sym;
                }
                self.show_painratio = true;
                if self.painratio_snapshot.symbol.is_empty() && !self.painratio_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_painratio(
                                &conn,
                                &self.painratio_symbol,
                            ) {
                                self.painratio_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "CUSUM" | "BDE_CUSUM" | "STRUCTURAL_BREAK" | "MEAN_BREAK" | "CUSUM_TEST"
            | "STABILITY_TEST" => {
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
                    self.cusum_symbol = sym;
                }
                self.show_cusum = true;
                if self.cusum_snapshot.symbol.is_empty() && !self.cusum_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cusum(&conn, &self.cusum_symbol)
                            {
                                self.cusum_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "CFVAR" | "CORNISH_FISHER" | "CORNISHFISHER" | "MODIFIED_VAR" | "MODIFIEDVAR"
            | "CF_VAR" | "SKEW_KURT_VAR" => {
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
                    self.cfvar_symbol = sym;
                }
                self.show_cfvar = true;
                if self.cfvar_snapshot.symbol.is_empty() && !self.cfvar_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cfvar(&conn, &self.cfvar_symbol)
                            {
                                self.cfvar_snapshot = snap;
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
