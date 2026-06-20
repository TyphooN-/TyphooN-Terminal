use super::*;

mod directional_moneyflow_sar_commands;
mod distribution_entropy_commands;
mod fractal_tail_dependence_commands;
mod jump_stationarity_tail_commands;
mod momentum_oscillator_commands;
mod price_transform_extrema_commands;
mod residual_cycle_memory_commands;
mod squeeze_channel_adaptive_commands;
mod trend_channel_transform_commands;
mod volatility_bubble_nonlinearity_commands;
mod volume_choppiness_moving_average_commands;
mod volume_momentum_trend_cycle_commands;

impl TyphooNApp {
    pub(super) fn handle_market_structure_model_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            _ if self.handle_distribution_entropy_command(cmd_upper) => {}
            _ if self.handle_fractal_tail_dependence_command(cmd_upper) => {}
            _ if self.handle_jump_stationarity_tail_command(cmd_upper) => {}
            _ if self.handle_volatility_bubble_nonlinearity_command(cmd_upper) => {}
            _ if self.handle_residual_cycle_memory_command(cmd_upper) => {}
            _ if self.handle_squeeze_channel_adaptive_command(cmd_upper) => {}
            _ if self.handle_trend_channel_transform_command(cmd_upper) => {}
            _ if self.handle_directional_moneyflow_sar_command(cmd_upper) => {}
            _ if self.handle_volume_choppiness_moving_average_command(cmd_upper) => {}
            _ if self.handle_momentum_oscillator_command(cmd_upper) => {}
            _ if self.handle_price_transform_extrema_command(cmd_upper) => {}
            _ if self.handle_volume_momentum_trend_cycle_command(cmd_upper) => {}
            // ── Research moving-average palette aliases ──
            "STOCH" | "STOCHFIT" | "STOCH_WIN" | "STOCHASTIC" | "LANE_STOCH" => {
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
                    self.stoch_win_symbol = sym;
                }
                self.show_stoch_win = true;
                if self.stoch_win_snapshot.symbol.is_empty() && !self.stoch_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_stoch(
                                &conn,
                                &self.stoch_win_symbol,
                            ) {
                                self.stoch_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MACD" | "MACDFIT" | "MACD_WIN" | "APPEL_MACD" | "MOVING_AVERAGE_CONVERGENCE" => {
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
                    self.macd_win_symbol = sym;
                }
                self.show_macd_win = true;
                if self.macd_win_snapshot.symbol.is_empty() && !self.macd_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_macd(
                                &conn,
                                &self.macd_win_symbol,
                            ) {
                                self.macd_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VWAPFIT" | "VWAP_WIN" | "VWAP_SNAPSHOT" | "VOLUME_WEIGHTED" | "VOL_WEIGHTED_AVG" => {
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
                    self.vwap_win_symbol = sym;
                }
                self.show_vwap_win = true;
                if self.vwap_win_snapshot.symbol.is_empty() && !self.vwap_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_vwap(
                                &conn,
                                &self.vwap_win_symbol,
                            ) {
                                self.vwap_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MCGD" | "MCGDFIT" | "MCGD_WIN" | "MCGINLEY_DYNAMIC" | "MCGINLEY" => {
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
                    self.mcgd_win_symbol = sym;
                }
                self.show_mcgd_win = true;
                if self.mcgd_win_snapshot.symbol.is_empty() && !self.mcgd_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_mcgd(
                                &conn,
                                &self.mcgd_win_symbol,
                            ) {
                                self.mcgd_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RWI" | "RWIFIT" | "RWI_WIN" | "RANDOM_WALK" | "POULOS_RWI" => {
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
                    self.rwi_win_symbol = sym;
                }
                self.show_rwi_win = true;
                if self.rwi_win_snapshot.symbol.is_empty() && !self.rwi_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_rwi(&conn, &self.rwi_win_symbol)
                            {
                                self.rwi_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Moving-average research palette aliases ──
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
            }
            // ── Momentum and volatility research palette aliases ──
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
            }
            _ => return false,
        }
        true
    }
}
