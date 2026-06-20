use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_factor_growth_quality_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── Factor, growth, and quality palette aliases ──
            "SIZEF" | "SIZE_FACTOR" | "SIZE_RANK" => {
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
                    self.sizef_symbol = sym;
                }
                self.show_sizef = true;
                if self.sizef_snapshot.symbol.is_empty() && !self.sizef_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_sizef(&conn, &self.sizef_symbol)
                            {
                                self.sizef_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "MOMF" | "MOMENTUM_RANK" | "MOM_RANK" => {
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
                    self.momf_symbol = sym;
                }
                self.show_momf = true;
                if self.momf_snapshot.symbol.is_empty() && !self.momf_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_momf(&conn, &self.momf_symbol)
                            {
                                self.momf_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "PEADRANK" | "PEAD_RANK" => {
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
                    self.peadrank_symbol = sym;
                }
                self.show_peadrank = true;
                if self.peadrank_snapshot.symbol.is_empty() && !self.peadrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_peadrank(
                                &conn,
                                &self.peadrank_symbol,
                            ) {
                                self.peadrank_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "FQM" | "FUND_QUALITY" | "QUALITY_METER" => {
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
                    self.fqm_symbol = sym;
                }
                self.show_fqm = true;
                if self.fqm_snapshot.symbol.is_empty() && !self.fqm_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_fqm(&conn, &self.fqm_symbol)
                            {
                                self.fqm_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "REVRANK" | "REV_RANK" | "REVENUE_GROWTH_RANK" => {
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
                    self.revrank_symbol = sym;
                }
                self.show_revrank = true;
                if self.revrank_snapshot.symbol.is_empty() && !self.revrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_revrank(
                                &conn,
                                &self.revrank_symbol,
                            ) {
                                self.revrank_snapshot = snap;
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
