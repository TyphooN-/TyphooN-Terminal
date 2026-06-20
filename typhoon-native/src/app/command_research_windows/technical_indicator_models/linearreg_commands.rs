use super::*;

impl TyphooNApp {
    pub(super) fn handle_linearreg_commands(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── palette aliases ──
            "LINEARREG" | "LINEARREG_FIT" | "LINEAR_REG" | "LINEARREG_WIN" | "LINREG_FITTED" => {
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
                    self.linearreg_win_symbol = sym;
                }
                self.show_linearreg_win = true;
                if self.linearreg_win_snapshot.symbol.is_empty()
                    && !self.linearreg_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_linearreg(
                                &conn,
                                &self.linearreg_win_symbol,
                            ) {
                                self.linearreg_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "LINEARREG_ANGLE" | "LREGANGLE" | "LINEAR_REG_ANGLE" | "LINREGANGLE" | "LRANGLE" => {
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
                    self.linearreg_angle_win_symbol = sym;
                }
                self.show_linearreg_angle_win = true;
                if self.linearreg_angle_win_snapshot.symbol.is_empty()
                    && !self.linearreg_angle_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_linearreg_angle(
                                    &conn,
                                    &self.linearreg_angle_win_symbol,
                                )
                            {
                                self.linearreg_angle_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HT_DCPHASE" | "DCPHASE" | "HILBERT_DCPHASE" | "HTDCPHASE" | "CYCLE_PHASE" => {
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
                    self.ht_dcphase_win_symbol = sym;
                }
                self.show_ht_dcphase_win = true;
                if self.ht_dcphase_win_snapshot.symbol.is_empty()
                    && !self.ht_dcphase_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_dcphase(
                                &conn,
                                &self.ht_dcphase_win_symbol,
                            ) {
                                self.ht_dcphase_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HT_SINE" | "HTSINE" | "HILBERT_SINE" | "SINEWAVE" | "LEADSINE" => {
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
                    self.ht_sine_win_symbol = sym;
                }
                self.show_ht_sine_win = true;
                if self.ht_sine_win_snapshot.symbol.is_empty()
                    && !self.ht_sine_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_sine(
                                &conn,
                                &self.ht_sine_win_symbol,
                            ) {
                                self.ht_sine_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HT_PHASOR" | "HTPHASOR" | "HILBERT_PHASOR" | "PHASOR" | "IQ_COMP" => {
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
                    self.ht_phasor_win_symbol = sym;
                }
                self.show_ht_phasor_win = true;
                if self.ht_phasor_win_snapshot.symbol.is_empty()
                    && !self.ht_phasor_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_phasor(
                                &conn,
                                &self.ht_phasor_win_symbol,
                            ) {
                                self.ht_phasor_win_snapshot = snap;
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
