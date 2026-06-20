use super::*;

impl TyphooNApp {
    pub(super) fn handle_volatility_force_commands(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── palette aliases ──
            // Note: "MASS_INDEX"/"DORSEY_MASS" already claimed by curvefit.
            "MASSINDEX" | "MI" | "MASS_INDEX_WIN" | "MINDEX" | "MASS_25" => {
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
                    self.mass_index_win_symbol = sym;
                }
                self.show_mass_index_win = true;
                if self.mass_index_win_snapshot.symbol.is_empty()
                    && !self.mass_index_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_mass_index(
                                &conn,
                                &self.mass_index_win_symbol,
                            ) {
                                self.mass_index_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "NATR" | "NORMALIZED_ATR" | "NATR_WIN" | "NORMALIZED_ATR_WIN" | "ATR_PCT" => {
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
                    self.natr_win_symbol = sym;
                }
                self.show_natr_win = true;
                if self.natr_win_snapshot.symbol.is_empty() && !self.natr_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_natr(
                                &conn,
                                &self.natr_win_symbol,
                            ) {
                                self.natr_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // Note: bare "SQUEEZE" is a chart toggle, not claimed here.
            "TTM_SQUEEZE" | "TTMSQUEEZE" | "TTM_SQUEEZE_WIN" | "CARTER_SQUEEZE" | "TTM" => {
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
                    self.ttm_squeeze_win_symbol = sym;
                }
                self.show_ttm_squeeze_win = true;
                if self.ttm_squeeze_win_snapshot.symbol.is_empty()
                    && !self.ttm_squeeze_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ttm_squeeze(
                                &conn,
                                &self.ttm_squeeze_win_symbol,
                            ) {
                                self.ttm_squeeze_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // Note: "FORCE_INDEX"/"ELDER_FORCE" already claimed by EFI curvefit.
            "FORCEINDEX" | "FORCE" | "FI" | "FORCE_INDEX_WIN" | "FORCE13" => {
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
                    self.force_index_win_symbol = sym;
                }
                self.show_force_index_win = true;
                if self.force_index_win_snapshot.symbol.is_empty()
                    && !self.force_index_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_force_index(
                                &conn,
                                &self.force_index_win_symbol,
                            ) {
                                self.force_index_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "TRANGE" | "TRUE_RANGE" | "TR" | "TRANGE_WIN" | "RAW_TRUE_RANGE" => {
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
                    self.trange_win_symbol = sym;
                }
                self.show_trange_win = true;
                if self.trange_win_snapshot.symbol.is_empty() && !self.trange_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_trange(
                                &conn,
                                &self.trange_win_symbol,
                            ) {
                                self.trange_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            _ => return false,
        }
        true
    }
}
