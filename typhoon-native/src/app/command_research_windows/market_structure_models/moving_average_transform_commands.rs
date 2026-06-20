use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_moving_average_transform_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── Moving-average and transform research palette aliases ──
            "DEMA" | "DEMAFIT" | "DEMA_WIN" | "DOUBLE_EMA" | "DOUBLE_EXPONENTIAL" => {
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
                    self.dema_win_symbol = sym;
                }
                self.show_dema_win = true;
                if self.dema_win_snapshot.symbol.is_empty() && !self.dema_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_dema(
                                &conn,
                                &self.dema_win_symbol,
                            ) {
                                self.dema_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "TEMA" | "TEMAFIT" | "TEMA_WIN" | "TRIPLE_EMA_WIN" | "TRIPLE_EXPONENTIAL" => {
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
                    self.tema_win_symbol = sym;
                }
                self.show_tema_win = true;
                if self.tema_win_snapshot.symbol.is_empty() && !self.tema_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_tema(
                                &conn,
                                &self.tema_win_symbol,
                            ) {
                                self.tema_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "LINREG" | "LINREGFIT" | "LINREG_WIN" | "LIN_REGRESSION" | "LINEAR_REGRESSION" => {
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
                    self.linreg_win_symbol = sym;
                }
                self.show_linreg_win = true;
                if self.linreg_win_snapshot.symbol.is_empty() && !self.linreg_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_linreg(
                                &conn,
                                &self.linreg_win_symbol,
                            ) {
                                self.linreg_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "PIVOTSFIT" | "PIVOTS_WIN" | "PIVOTS_SNAPSHOT" | "FLOOR_PIVOTS"
            | "PIVOT_POINTS_WIN" => {
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
                    self.pivots_win_symbol = sym;
                }
                self.show_pivots_win = true;
                if self.pivots_win_snapshot.symbol.is_empty() && !self.pivots_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_pivots(
                                &conn,
                                &self.pivots_win_symbol,
                            ) {
                                self.pivots_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "HEIKIN"
            | "HEIKIN_WIN"
            | "HEIKIN_SNAPSHOT"
            | "HEIKIN_ASHI_SNAPSHOT"
            | "HA_SNAPSHOT" => {
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
                    self.heikin_win_symbol = sym;
                }
                self.show_heikin_win = true;
                if self.heikin_win_snapshot.symbol.is_empty() && !self.heikin_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_heikin(
                                &conn,
                                &self.heikin_win_symbol,
                            ) {
                                self.heikin_win_snapshot = snap;
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
