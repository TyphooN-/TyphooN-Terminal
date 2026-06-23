use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_trend_channel_transform_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── Trend, channel, and transform palette aliases ──
            // Bare ICHIMOKU / SUPERTREND / KELTNER / FISHER are already bound to
            // chart-overlay toggles upstream; only disambiguated forms are used here.
            "ICHIMOKUFIT" | "ICHIMOKU_WIN" | "IKH" | "KUMO" | "TENKAN_KIJUN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.ichimoku_win_symbol = sym;
                }
                self.show_ichimoku_win = true;
                if self.ichimoku_win_snapshot.symbol.is_empty()
                    && !self.ichimoku_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ichimoku(
                                &conn,
                                &self.ichimoku_win_symbol,
                            ) {
                                self.ichimoku_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "SUPERTRENDFIT" | "SUPERTREND_WIN" | "ST_FIT" | "ATR_TRAIL" | "SUPERTREND_ATR" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.supertrend_win_symbol = sym;
                }
                self.show_supertrend_win = true;
                if self.supertrend_win_snapshot.symbol.is_empty()
                    && !self.supertrend_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_supertrend(
                                &conn,
                                &self.supertrend_win_symbol,
                            ) {
                                self.supertrend_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "KELTNERFIT" | "KELTNER_WIN" | "KC_FIT" | "KELTNERCHAN" | "KELCHAN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.keltner_win_symbol = sym;
                }
                self.show_keltner_win = true;
                if self.keltner_win_snapshot.symbol.is_empty()
                    && !self.keltner_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_keltner(
                                &conn,
                                &self.keltner_win_symbol,
                            ) {
                                self.keltner_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "FISHERFIT" | "FISHER_WIN" | "FISHER_TRANSFORM" | "EHLERS_FISHER" | "FT_EHLERS" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.fisher_win_symbol = sym;
                }
                self.show_fisher_win = true;
                if self.fisher_win_snapshot.symbol.is_empty() && !self.fisher_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_fisher(
                                &conn,
                                &self.fisher_win_symbol,
                            ) {
                                self.fisher_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "AROON" | "AROON_UP" | "AROON_DOWN" | "AROONFIT" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.aroon_win_symbol = sym;
                }
                self.show_aroon_win = true;
                if self.aroon_win_snapshot.symbol.is_empty() && !self.aroon_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_aroon(
                                &conn,
                                &self.aroon_win_symbol,
                            ) {
                                self.aroon_win_snapshot = snap;
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
