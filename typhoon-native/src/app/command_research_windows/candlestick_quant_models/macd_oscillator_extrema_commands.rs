use super::*;

impl TyphooNApp {
    pub(super) fn handle_macd_oscillator_extrema_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            "AROONOSC" | "AROONOSCWIN" | "AROON_OSC" | "AROONOSCILLATOR" | "AROON_DIFF" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.aroonosc_win_symbol = sym;
                }
                self.show_aroonosc_win = true;
                if self.aroonosc_win_snapshot.symbol.is_empty()
                    && !self.aroonosc_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_aroonosc(
                                &conn,
                                &self.aroonosc_win_symbol,
                            ) {
                                self.aroonosc_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MINMAXINDEX" | "MMIDXWIN" | "MINMAX_IDX" | "EXTREMA_IDX" | "HL_IDX" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.minmaxindex_win_symbol = sym;
                }
                self.show_minmaxindex_win = true;
                if self.minmaxindex_win_snapshot.symbol.is_empty()
                    && !self.minmaxindex_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_minmaxindex(
                                &conn,
                                &self.minmaxindex_win_symbol,
                            ) {
                                self.minmaxindex_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MACDEXT" | "MACDEXTWIN" | "MACD_EXT" | "MACD_CONFIG" | "MACD_FLEX" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.macdext_win_symbol = sym;
                }
                self.show_macdext_win = true;
                if self.macdext_win_snapshot.symbol.is_empty()
                    && !self.macdext_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_macdext(
                                &conn,
                                &self.macdext_win_symbol,
                            ) {
                                self.macdext_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MACDFIX" | "MACDFIXWIN" | "MACD_FIX" | "MACD_12_26" | "MACD_STD" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.macdfix_win_symbol = sym;
                }
                self.show_macdfix_win = true;
                if self.macdfix_win_snapshot.symbol.is_empty()
                    && !self.macdfix_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_macdfix(
                                &conn,
                                &self.macdfix_win_symbol,
                            ) {
                                self.macdfix_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MAVP" | "MAVPWIN" | "VAR_PERIOD_MA" | "MA_VARPERIOD" | "MA_DYNAMIC" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.mavp_win_symbol = sym;
                }
                self.show_mavp_win = true;
                if self.mavp_win_snapshot.symbol.is_empty() && !self.mavp_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_mavp(
                                &conn,
                                &self.mavp_win_symbol,
                            ) {
                                self.mavp_win_snapshot = snap;
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
