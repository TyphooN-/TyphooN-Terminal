use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_price_transform_extrema_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── Price-transform extrema palette aliases ──
            // Bare MASS / CHAIKOSC / KLINGER / STOCHRSI / AWESOME are unbound upstream (verified) and kept as aliases.
            "MASS" | "MASSFIT" | "MASS_WIN" | "MASS_INDEX" | "DORSEY_MASS" => {
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
                    self.mass_win_symbol = sym;
                }
                self.show_mass_win = true;
                if self.mass_win_snapshot.symbol.is_empty() && !self.mass_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_mass(
                                &conn,
                                &self.mass_win_symbol,
                            ) {
                                self.mass_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "CHAIKOSC" | "CHAIKOSCFIT" | "CHAIKOSC_WIN" | "CHAIKIN_OSC" | "CHAIKIN_OSCILLATOR"
            | "CHKOSC" => {
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
                    self.chaikosc_win_symbol = sym;
                }
                self.show_chaikosc_win = true;
                if self.chaikosc_win_snapshot.symbol.is_empty()
                    && !self.chaikosc_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_chaikosc(
                                &conn,
                                &self.chaikosc_win_symbol,
                            ) {
                                self.chaikosc_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "KLINGER" | "KLINGERFIT" | "KLINGER_WIN" | "KVO" | "KLINGER_OSC" | "KLINGER_VOLUME" => {
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
                    self.klinger_win_symbol = sym;
                }
                self.show_klinger_win = true;
                if self.klinger_win_snapshot.symbol.is_empty()
                    && !self.klinger_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_klinger(
                                &conn,
                                &self.klinger_win_symbol,
                            ) {
                                self.klinger_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "STOCHRSI" | "STOCHRSIFIT" | "STOCHRSI_WIN" | "STOCH_RSI" | "STOCHASTIC_RSI"
            | "SRSI" => {
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
                    self.stochrsi_win_symbol = sym;
                }
                self.show_stochrsi_win = true;
                if self.stochrsi_win_snapshot.symbol.is_empty()
                    && !self.stochrsi_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_stochrsi(
                                &conn,
                                &self.stochrsi_win_symbol,
                            ) {
                                self.stochrsi_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "AWESOME" | "AWESOMEFIT" | "AWESOME_WIN" | "AO" | "AWESOME_OSC"
            | "AWESOME_OSCILLATOR" | "BILL_WILLIAMS" => {
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
                    self.awesome_win_symbol = sym;
                }
                self.show_awesome_win = true;
                if self.awesome_win_snapshot.symbol.is_empty()
                    && !self.awesome_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_awesome(
                                &conn,
                                &self.awesome_win_symbol,
                            ) {
                                self.awesome_win_snapshot = snap;
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
