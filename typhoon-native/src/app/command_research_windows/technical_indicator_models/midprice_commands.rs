use super::*;

impl TyphooNApp {
    pub(super) fn handle_midprice_commands(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── palette aliases ──
            "MIDPRICE" | "MID_PRICE" | "MIDBAR" | "MIDBARPRICE" | "HLMIDPRICE" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
            _ => return false,
        }
        true
    }
}
