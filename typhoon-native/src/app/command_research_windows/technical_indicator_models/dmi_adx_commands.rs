use super::*;

impl TyphooNApp {
    pub(super) fn handle_dmi_adx_commands(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── DMI family ──
            "PLUS_DI" | "PDI" | "DI_PLUS" | "DIPOS" | "WILDER_PDI" => {
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
                    self.plus_di_win_symbol = sym;
                }
                self.show_plus_di_win = true;
                if self.plus_di_win_snapshot.symbol.is_empty()
                    && !self.plus_di_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_plus_di(
                                &conn,
                                &self.plus_di_win_symbol,
                            ) {
                                self.plus_di_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MINUS_DI" | "MDI" | "DI_MINUS" | "DINEG" | "WILDER_MDI" => {
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
                    self.minus_di_win_symbol = sym;
                }
                self.show_minus_di_win = true;
                if self.minus_di_win_snapshot.symbol.is_empty()
                    && !self.minus_di_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_minus_di(
                                &conn,
                                &self.minus_di_win_symbol,
                            ) {
                                self.minus_di_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PLUS_DM" | "PDM" | "DM_PLUS" | "DMPOS" | "WILDER_PDM" => {
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
                    self.plus_dm_win_symbol = sym;
                }
                self.show_plus_dm_win = true;
                if self.plus_dm_win_snapshot.symbol.is_empty()
                    && !self.plus_dm_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_plus_dm(
                                &conn,
                                &self.plus_dm_win_symbol,
                            ) {
                                self.plus_dm_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MINUS_DM" | "MDM" | "DM_MINUS" | "DMNEG" | "WILDER_MDM" => {
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
                    self.minus_dm_win_symbol = sym;
                }
                self.show_minus_dm_win = true;
                if self.minus_dm_win_snapshot.symbol.is_empty()
                    && !self.minus_dm_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_minus_dm(
                                &conn,
                                &self.minus_dm_win_symbol,
                            ) {
                                self.minus_dm_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DX" | "DX_WILDER" | "DXWIN" | "DIRIDX" | "WILDER_DX" => {
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
                    self.dx_win_symbol = sym;
                }
                self.show_dx_win = true;
                if self.dx_win_snapshot.symbol.is_empty() && !self.dx_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_dx(&conn, &self.dx_win_symbol)
                            {
                                self.dx_win_snapshot = snap;
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
