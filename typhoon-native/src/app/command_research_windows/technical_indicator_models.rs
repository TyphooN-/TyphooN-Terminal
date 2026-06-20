use super::*;

mod adaptive_momentum_commands;
mod linearreg_slope_commands;
mod momentum_flow_commands;
mod volatility_force_commands;
mod wma_rainbow_mesa_frama_commands;

impl TyphooNApp {
    pub(super) fn handle_technical_indicator_model_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // (delegated early groups)
            _ if self.handle_adaptive_momentum_commands(cmd_upper) => {}
            _ if self.handle_momentum_flow_commands(cmd_upper) => {}
            _ if self.handle_wma_rainbow_mesa_frama_commands(cmd_upper) => {}
            _ if self.handle_volatility_force_commands(cmd_upper) => {}
            _ if self.handle_linearreg_slope_commands(cmd_upper) => {}
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
            // ── palette aliases ──
            "MIDPRICE" | "MID_PRICE" | "MIDBAR" | "MIDBARPRICE" | "HLMIDPRICE" => {
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
                    self.midprice_win_symbol = sym;
                }
                self.show_midprice_win = true;
                if self.midprice_win_snapshot.symbol.is_empty()
                    && !self.midprice_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_midprice(
                                &conn,
                                &self.midprice_win_symbol,
                            ) {
                                self.midprice_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "APO" | "ABS_PRICE_OSC" | "ABSPRICEOSC" | "ABSPO" | "APOWIN" => {
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
                    self.apo_win_symbol = sym;
                }
                self.show_apo_win = true;
                if self.apo_win_snapshot.symbol.is_empty() && !self.apo_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_apo(&conn, &self.apo_win_symbol)
                            {
                                self.apo_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MOMRAW" | "MOMENTUM_RAW" | "MOM_TA" | "RAWMOM" | "TALIB_MOM" => {
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
                    self.mom_win_symbol = sym;
                }
                self.show_mom_win = true;
                if self.mom_win_snapshot.symbol.is_empty() && !self.mom_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_mom(&conn, &self.mom_win_symbol)
                            {
                                self.mom_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SAREXT" | "SAR_EXT" | "EXTENDED_SAR" | "SAREXTENDED" | "PSAR_EXT" => {
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
                    self.sarext_win_symbol = sym;
                }
                self.show_sarext_win = true;
                if self.sarext_win_snapshot.symbol.is_empty() && !self.sarext_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_sarext(
                                &conn,
                                &self.sarext_win_symbol,
                            ) {
                                self.sarext_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ADXR" | "ADX_RATING" | "ADX_R" | "ADXRATING" | "ADX_RANK" => {
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
                    self.adxr_win_symbol = sym;
                }
                self.show_adxr_win = true;
                if self.adxr_win_snapshot.symbol.is_empty() && !self.adxr_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_adxr(
                                &conn,
                                &self.adxr_win_symbol,
                            ) {
                                self.adxr_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "AVGPRICE" | "AVG_PRICE" | "OHLC_AVG" | "OHLCAVG" | "AVGOHLC" => {
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
                    self.avgprice_win_symbol = sym;
                }
                self.show_avgprice_win = true;
                if self.avgprice_win_snapshot.symbol.is_empty()
                    && !self.avgprice_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_avgprice(
                                &conn,
                                &self.avgprice_win_symbol,
                            ) {
                                self.avgprice_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MEDPRICE" | "MED_PRICE" | "HLMED" | "HLMEDIAN" | "RANGEMEDIAN" => {
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
                    self.medprice_win_symbol = sym;
                }
                self.show_medprice_win = true;
                if self.medprice_win_snapshot.symbol.is_empty()
                    && !self.medprice_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_medprice(
                                &conn,
                                &self.medprice_win_symbol,
                            ) {
                                self.medprice_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "TYPPRICE" | "TYP_PRICE" | "TYPICAL_PRICE" | "TYPICALPRICE" | "HLC3" => {
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
                    self.typprice_win_symbol = sym;
                }
                self.show_typprice_win = true;
                if self.typprice_win_snapshot.symbol.is_empty()
                    && !self.typprice_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_typprice(
                                &conn,
                                &self.typprice_win_symbol,
                            ) {
                                self.typprice_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "WCLPRICE" | "WCL_PRICE" | "WEIGHTED_CLOSE" | "WEIGHTEDCLOSE" | "HLCC4" => {
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
                    self.wclprice_win_symbol = sym;
                }
                self.show_wclprice_win = true;
                if self.wclprice_win_snapshot.symbol.is_empty()
                    && !self.wclprice_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_wclprice(
                                &conn,
                                &self.wclprice_win_symbol,
                            ) {
                                self.wclprice_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VARIANCE" | "VARIANCE_WIN" | "CLOSE_VARIANCE" | "CVARIANCE" | "VARWIN" => {
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
                    self.variance_win_symbol = sym;
                }
                self.show_variance_win = true;
                if self.variance_win_snapshot.symbol.is_empty()
                    && !self.variance_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_variance(
                                &conn,
                                &self.variance_win_symbol,
                            ) {
                                self.variance_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
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
            // ── Research section ──
            "ROC" | "ROC_WILDER" | "ROCWIN" | "ROCRATE" | "RATE_OF_CHANGE" => {
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
                    self.roc_win_symbol = sym;
                }
                self.show_roc_win = true;
                if self.roc_win_snapshot.symbol.is_empty() && !self.roc_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_roc(&conn, &self.roc_win_symbol)
                            {
                                self.roc_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ROCP" | "ROCP_WILDER" | "ROCPWIN" | "ROCPCT" | "ROC_PCT" => {
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
                    self.rocp_win_symbol = sym;
                }
                self.show_rocp_win = true;
                if self.rocp_win_snapshot.symbol.is_empty() && !self.rocp_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_rocp(
                                &conn,
                                &self.rocp_win_symbol,
                            ) {
                                self.rocp_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ROCR" | "ROCR_WILDER" | "ROCRWIN" | "ROCRATIO" | "ROC_RATIO" => {
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
                    self.rocr_win_symbol = sym;
                }
                self.show_rocr_win = true;
                if self.rocr_win_snapshot.symbol.is_empty() && !self.rocr_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_rocr(
                                &conn,
                                &self.rocr_win_symbol,
                            ) {
                                self.rocr_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ROCR100" | "ROCR100_WILDER" | "ROCR100WIN" | "ROCR100IDX" | "ROC_RATIO_100" => {
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
                    self.rocr100_win_symbol = sym;
                }
                self.show_rocr100_win = true;
                if self.rocr100_win_snapshot.symbol.is_empty()
                    && !self.rocr100_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_rocr100(
                                &conn,
                                &self.rocr100_win_symbol,
                            ) {
                                self.rocr100_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CORREL" | "CORRWIN" | "ROLLCORR" | "AUTOCORR" | "PEARSON_AUTO" => {
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
                    self.correl_win_symbol = sym;
                }
                self.show_correl_win = true;
                if self.correl_win_snapshot.symbol.is_empty() && !self.correl_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_correl(
                                &conn,
                                &self.correl_win_symbol,
                            ) {
                                self.correl_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MIN" | "MINWIN" | "MIN_CLOSE" | "LOW_BAND" | "ROLL_MIN" => {
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
                    self.min_win_symbol = sym;
                }
                self.show_min_win = true;
                if self.min_win_snapshot.symbol.is_empty() && !self.min_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_min(&conn, &self.min_win_symbol)
                            {
                                self.min_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MAX" | "MAXWIN" | "MAX_CLOSE" | "HIGH_BAND" | "ROLL_MAX" => {
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
                    self.max_win_symbol = sym;
                }
                self.show_max_win = true;
                if self.max_win_snapshot.symbol.is_empty() && !self.max_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_max(&conn, &self.max_win_symbol)
                            {
                                self.max_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MINMAX" | "MINMAXWIN" | "RANGE_BAND" | "HL_RANGE" | "EXTREMA" => {
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
                    self.minmax_win_symbol = sym;
                }
                self.show_minmax_win = true;
                if self.minmax_win_snapshot.symbol.is_empty() && !self.minmax_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_minmax(
                                &conn,
                                &self.minmax_win_symbol,
                            ) {
                                self.minmax_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MININDEX" | "MINIDXWIN" | "LOW_IDX" | "MIN_AGE" | "LOW_RECENCY" => {
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
                    self.minindex_win_symbol = sym;
                }
                self.show_minindex_win = true;
                if self.minindex_win_snapshot.symbol.is_empty()
                    && !self.minindex_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_minindex(
                                &conn,
                                &self.minindex_win_symbol,
                            ) {
                                self.minindex_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MAXINDEX" | "MAXIDXWIN" | "HIGH_IDX" | "MAX_AGE" | "HIGH_RECENCY" => {
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
                    self.maxindex_win_symbol = sym;
                }
                self.show_maxindex_win = true;
                if self.maxindex_win_snapshot.symbol.is_empty()
                    && !self.maxindex_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_maxindex(
                                &conn,
                                &self.maxindex_win_symbol,
                            ) {
                                self.maxindex_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "BBANDS" | "BBANDSWIN" | "BB_BANDS" | "BBAND" | "BOLL_BANDS" => {
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
                    self.bbands_win_symbol = sym;
                }
                self.show_bbands_win = true;
                if self.bbands_win_snapshot.symbol.is_empty() && !self.bbands_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_bbands(
                                &conn,
                                &self.bbands_win_symbol,
                            ) {
                                self.bbands_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "AD" | "AD_LINE_TALIB" | "AD_CHAIKIN" | "ADWIN" | "TALIB_AD" => {
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
                    self.ad_win_symbol = sym;
                }
                self.show_ad_win = true;
                if self.ad_win_snapshot.symbol.is_empty() && !self.ad_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_ad(&conn, &self.ad_win_symbol)
                            {
                                self.ad_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ADOSC" | "ADOSCWIN" | "TALIB_ADOSC" | "AD_OSCILLATOR" | "CHAIKIN_ADO" => {
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
                    self.adosc_win_symbol = sym;
                }
                self.show_adosc_win = true;
                if self.adosc_win_snapshot.symbol.is_empty() && !self.adosc_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_adosc(
                                &conn,
                                &self.adosc_win_symbol,
                            ) {
                                self.adosc_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SUM" | "SUMWIN" | "ROLLSUM" | "CLOSE_SUM" | "SUM_CLOSE" => {
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
                    self.sum_win_symbol = sym;
                }
                self.show_sum_win = true;
                if self.sum_win_snapshot.symbol.is_empty() && !self.sum_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_sum(&conn, &self.sum_win_symbol)
                            {
                                self.sum_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "LINEARREG_INTERCEPT"
            | "LINREG_INTERCEPT"
            | "LINTERCEPT"
            | "LRINTERCEPT"
            | "REG_INTERCEPT"
            | "LINEARREG_B" => {
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
                    self.linreg_intercept_win_symbol = sym;
                }
                self.show_linreg_intercept_win = true;
                if self.linreg_intercept_win_snapshot.symbol.is_empty()
                    && !self.linreg_intercept_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_linreg_intercept(
                                    &conn,
                                    &self.linreg_intercept_win_symbol,
                                )
                            {
                                self.linreg_intercept_win_snapshot = snap;
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
