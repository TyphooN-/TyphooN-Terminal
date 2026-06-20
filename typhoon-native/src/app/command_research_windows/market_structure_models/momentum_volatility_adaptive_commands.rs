use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_momentum_volatility_adaptive_command(
        &mut self,
        cmd_upper: &String,
    ) -> bool {
        match cmd_upper.as_str() {
            // ── Momentum, volatility, and adaptive-average palette aliases ──
            "ALMA" | "ALMAFIT" | "ALMA_WIN" | "ARNAUD_LEGOUX" | "GAUSSIAN_MA" => {
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
                    self.alma_win_symbol = sym;
                }
                self.show_alma_win = true;
                if self.alma_win_snapshot.symbol.is_empty() && !self.alma_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_alma(
                                &conn,
                                &self.alma_win_symbol,
                            ) {
                                self.alma_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "ZLEMA" | "ZLEMAFIT" | "ZLEMA_WIN" | "ZERO_LAG_EMA" | "EHLERS_ZLEMA" => {
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
                    self.zlema_win_symbol = sym;
                }
                self.show_zlema_win = true;
                if self.zlema_win_snapshot.symbol.is_empty() && !self.zlema_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_zlema(
                                &conn,
                                &self.zlema_win_symbol,
                            ) {
                                self.zlema_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "ELDERRAY" | "ELDER_RAY" | "ELDERRAY_WIN" | "BULL_BEAR_POWER" | "ELDER_BULL_BEAR" => {
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
                    self.elderray_win_symbol = sym;
                }
                self.show_elderray_win = true;
                if self.elderray_win_snapshot.symbol.is_empty()
                    && !self.elderray_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_elderray(
                                &conn,
                                &self.elderray_win_symbol,
                            ) {
                                self.elderray_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "TSF" | "TSFFIT" | "TSF_WIN" | "TIME_SERIES_FORECAST" | "LINREG_FORECAST" => {
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
                    self.tsf_win_symbol = sym;
                }
                self.show_tsf_win = true;
                if self.tsf_win_snapshot.symbol.is_empty() && !self.tsf_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_tsf(&conn, &self.tsf_win_symbol)
                            {
                                self.tsf_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "RVI" | "RVIFIT" | "RVI_WIN" | "RELATIVE_VIGOR" | "VIGOR_INDEX" => {
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
                    self.rvi_win_symbol = sym;
                }
                self.show_rvi_win = true;
                if self.rvi_win_snapshot.symbol.is_empty() && !self.rvi_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_rvi(&conn, &self.rvi_win_symbol)
                            {
                                self.rvi_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "TRIMA" | "TRIMAFIT" | "TRIMA_WIN" | "TRIANGULAR_MA" | "TRIANGULAR_MOVING_AVERAGE" => {
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
                    self.trima_win_symbol = sym;
                }
                self.show_trima_win = true;
                if self.trima_win_snapshot.symbol.is_empty() && !self.trima_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_trima(
                                &conn,
                                &self.trima_win_symbol,
                            ) {
                                self.trima_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "T3" | "T3FIT" | "T3_WIN" | "TILLSON" | "TILLSON_T3" => {
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
                    self.t3_win_symbol = sym;
                }
                self.show_t3_win = true;
                if self.t3_win_snapshot.symbol.is_empty() && !self.t3_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_t3(&conn, &self.t3_win_symbol)
                            {
                                self.t3_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "VIDYA" | "VIDYAFIT" | "VIDYA_WIN" | "VARIABLE_INDEX_DYNAMIC" | "CHANDE_VIDYA" => {
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
                    self.vidya_win_symbol = sym;
                }
                self.show_vidya_win = true;
                if self.vidya_win_snapshot.symbol.is_empty() && !self.vidya_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_vidya(
                                &conn,
                                &self.vidya_win_symbol,
                            ) {
                                self.vidya_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "SMI" | "SMIFIT" | "SMI_WIN" | "STOCHASTIC_MOMENTUM" | "BLAU_SMI" => {
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
                    self.smi_win_symbol = sym;
                }
                self.show_smi_win = true;
                if self.smi_win_snapshot.symbol.is_empty() && !self.smi_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_smi(&conn, &self.smi_win_symbol)
                            {
                                self.smi_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "PVT" | "PVTFIT" | "PVT_WIN" | "PRICE_VOLUME_TREND" | "VOLUME_PRICE_TREND" => {
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
                    self.pvt_win_symbol = sym;
                }
                self.show_pvt_win = true;
                if self.pvt_win_snapshot.symbol.is_empty() && !self.pvt_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_pvt(&conn, &self.pvt_win_symbol)
                            {
                                self.pvt_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "AC" | "ACFIT" | "AC_WIN" | "ACCELERATOR" | "ACCEL_OSC" | "ACCELERATOR_OSCILLATOR" => {
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
                    self.ac_win_symbol = sym;
                }
                self.show_ac_win = true;
                if self.ac_win_snapshot.symbol.is_empty() && !self.ac_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_ac(&conn, &self.ac_win_symbol)
                            {
                                self.ac_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "CHVOL" | "CHVOLFIT" | "CHVOL_WIN" | "CHAIKIN_VOL" | "CHAIKIN_VOLATILITY" => {
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
                    self.chvol_win_symbol = sym;
                }
                self.show_chvol_win = true;
                if self.chvol_win_snapshot.symbol.is_empty() && !self.chvol_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_chvol(
                                &conn,
                                &self.chvol_win_symbol,
                            ) {
                                self.chvol_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "BBWFIT" | "BBW_WIN" | "BOLLINGER_WIDTH" | "BBW" | "BBWPCT" => {
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
                    self.bbwidth_win_symbol = sym;
                }
                self.show_bbwidth_win = true;
                if self.bbwidth_win_snapshot.symbol.is_empty()
                    && !self.bbwidth_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_bbwidth(
                                &conn,
                                &self.bbwidth_win_symbol,
                            ) {
                                self.bbwidth_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "ELDERIMP" | "ELDERIMPULSE" | "IMPULSE" | "IMPULSE_SYSTEM" | "ELDER_IMPULSE" => {
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
                    self.elderimp_win_symbol = sym;
                }
                self.show_elderimp_win = true;
                if self.elderimp_win_snapshot.symbol.is_empty()
                    && !self.elderimp_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_elderimp(
                                &conn,
                                &self.elderimp_win_symbol,
                            ) {
                                self.elderimp_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "RMI" | "RMIFIT" | "RMI_WIN" | "RELATIVE_MOMENTUM" | "RELATIVE_MOMENTUM_INDEX" => {
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
                    self.rmi_win_symbol = sym;
                }
                self.show_rmi_win = true;
                if self.rmi_win_snapshot.symbol.is_empty() && !self.rmi_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_rmi(&conn, &self.rmi_win_symbol)
                            {
                                self.rmi_win_snapshot = snap;
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
