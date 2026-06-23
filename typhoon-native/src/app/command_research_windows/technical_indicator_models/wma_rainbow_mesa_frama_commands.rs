use super::*;

impl TyphooNApp {
    pub(super) fn handle_wma_rainbow_mesa_frama_commands(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── WMA / RAINBOW / MESA_SINE / FRAMA / IBS ──
            "WMA" | "WEIGHTED_MA" | "WMA_WIN" | "LINEAR_WEIGHTED_MA" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.wma_win_symbol = sym;
                }
                self.show_wma_win = true;
                if self.wma_win_snapshot.symbol.is_empty() && !self.wma_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_wma(&conn, &self.wma_win_symbol)
                            {
                                self.wma_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RAINBOW" | "RAINBOW_MA" | "RAINBOW_OSC" | "RAINBOW_WIN" | "WIDNER_RAINBOW" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.rainbow_win_symbol = sym;
                }
                self.show_rainbow_win = true;
                if self.rainbow_win_snapshot.symbol.is_empty()
                    && !self.rainbow_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_rainbow(
                                &conn,
                                &self.rainbow_win_symbol,
                            ) {
                                self.rainbow_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MESA_SINE" | "MESASINE" | "MESA_SINEWAVE" | "SINE_WAVE" | "EHLERS_SINE" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.mesa_sine_win_symbol = sym;
                }
                self.show_mesa_sine_win = true;
                if self.mesa_sine_win_snapshot.symbol.is_empty()
                    && !self.mesa_sine_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_mesa_sine(
                                &conn,
                                &self.mesa_sine_win_symbol,
                            ) {
                                self.mesa_sine_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "FRAMA" | "FRACTAL_ADAPTIVE_MA" | "FRAMA_WIN" | "EHLERS_FRAMA" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.frama_win_symbol = sym;
                }
                self.show_frama_win = true;
                if self.frama_win_snapshot.symbol.is_empty() && !self.frama_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_frama(
                                &conn,
                                &self.frama_win_symbol,
                            ) {
                                self.frama_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "IBS" | "INTERNAL_BAR_STRENGTH" | "IBS_WIN" | "BAR_STRENGTH" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.ibs_win_symbol = sym;
                }
                self.show_ibs_win = true;
                if self.ibs_win_snapshot.symbol.is_empty() && !self.ibs_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_ibs(&conn, &self.ibs_win_symbol)
                            {
                                self.ibs_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "LAGUERRE_RSI" | "LAGUERRERSI" | "LRSI" | "LAGUERRE_RSI_WIN" | "EHLERS_LAGUERRE" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.laguerre_rsi_win_symbol = sym;
                }
                self.show_laguerre_rsi_win = true;
                if self.laguerre_rsi_win_snapshot.symbol.is_empty()
                    && !self.laguerre_rsi_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_laguerre_rsi(
                                &conn,
                                &self.laguerre_rsi_win_symbol,
                            ) {
                                self.laguerre_rsi_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ZIGZAG" | "ZIG_ZAG" | "ZIGZAG_WIN" | "ZZ" | "PIVOT_REVERSAL" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.zigzag_win_symbol = sym;
                }
                self.show_zigzag_win = true;
                if self.zigzag_win_snapshot.symbol.is_empty() && !self.zigzag_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_zigzag(
                                &conn,
                                &self.zigzag_win_symbol,
                            ) {
                                self.zigzag_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PGO" | "PRETTY_GOOD_OSC" | "PRETTY_GOOD_OSCILLATOR" | "PGO_WIN" | "JOHNSON_PGO" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.pgo_win_symbol = sym;
                }
                self.show_pgo_win = true;
                if self.pgo_win_snapshot.symbol.is_empty() && !self.pgo_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_pgo(&conn, &self.pgo_win_symbol)
                            {
                                self.pgo_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HT_TRENDLINE" | "HTTRENDLINE" | "HT_TREND" | "HT_TRENDLINE_WIN"
            | "HILBERT_TRENDLINE" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.ht_trendline_win_symbol = sym;
                }
                self.show_ht_trendline_win = true;
                if self.ht_trendline_win_snapshot.symbol.is_empty()
                    && !self.ht_trendline_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_trendline(
                                &conn,
                                &self.ht_trendline_win_symbol,
                            ) {
                                self.ht_trendline_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MIDPOINT" | "MIDPOINT_WIN" | "HL_MIDPOINT" | "MIDPOINT_N" | "MIDPT" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.midpoint_win_symbol = sym;
                }
                self.show_midpoint_win = true;
                if self.midpoint_win_snapshot.symbol.is_empty()
                    && !self.midpoint_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_midpoint(
                                &conn,
                                &self.midpoint_win_symbol,
                            ) {
                                self.midpoint_win_snapshot = snap;
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
