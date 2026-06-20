use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_squeeze_channel_adaptive_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── Squeeze, channel, and adaptive-average palette aliases ──
            // NOTE: bare "SQUEEZE"/"DONCHIAN"/"KAMA"/"KAUFMAN" are already
            // bound to chart-overlay toggles — these research windows use
            // disambiguated aliases only.
            "SHORTSQUEEZE" | "SHORT_SQUEEZE" | "SQZCOMP" | "SQUEEZESCORE" | "SQZSCORE" => {
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
                    self.squeeze_win_symbol = sym;
                }
                self.show_squeeze_win = true;
                if self.squeeze_win_snapshot.symbol.is_empty()
                    && !self.squeeze_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_squeeze(
                                &conn,
                                &self.squeeze_win_symbol,
                            ) {
                                self.squeeze_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "SQUEEZERANK" | "SQZRANK" | "SQUEEZE_RANK" | "SQRANK" | "SHORTSQUEEZERANK" => {
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
                    self.squeezerank_symbol = sym;
                }
                self.show_squeezerank = true;
                if self.squeezerank_snapshot.symbol.is_empty()
                    && !self.squeezerank_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_squeezerank(
                                &conn,
                                &self.squeezerank_symbol,
                            ) {
                                self.squeezerank_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "SQUEEZEWATCHLIST"
            | "SQZWATCH"
            | "SHORT_SQUEEZE_WATCH"
            | "SQUEEZE_WATCH"
            | "SQUEEZELIST" => {
                self.show_squeeze_watchlist = true;
                true
            }
            "BBSQUEEZE" | "BB_SQUEEZE" | "BOLLINGERSQUEEZE" | "BBANDS_SQUEEZE" | "BBWIDTH" => {
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
                    self.bbsqueeze_symbol = sym;
                }
                self.show_bbsqueeze = true;
                if self.bbsqueeze_snapshot.symbol.is_empty() && !self.bbsqueeze_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_bbsqueeze(
                                &conn,
                                &self.bbsqueeze_symbol,
                            ) {
                                self.bbsqueeze_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "DONCHIANBREAK" | "DONCHIANCHANNEL" | "DONCHIAN_CHANNEL" | "DONBREAK" | "DCCHAN" => {
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
                    self.donchian_win_symbol = sym;
                }
                self.show_donchian_win = true;
                if self.donchian_win_snapshot.symbol.is_empty()
                    && !self.donchian_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_donchian(
                                &conn,
                                &self.donchian_win_symbol,
                            ) {
                                self.donchian_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "KAMAFIT" | "KAMA_ER" | "KAMA_ADAPTIVE" | "ADAPTIVEMA" | "KAUFMAN_AMA" => {
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
                    self.kama_win_symbol = sym;
                }
                self.show_kama_win = true;
                if self.kama_win_snapshot.symbol.is_empty() && !self.kama_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_kama(
                                &conn,
                                &self.kama_win_symbol,
                            ) {
                                self.kama_win_snapshot = snap;
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
