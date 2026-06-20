use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_momentum_oscillator_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── Momentum oscillator palette aliases ──
            // Bare PPO / DPO / KST / ULTOSC / WILLR are unbound upstream (verified) and kept as aliases.
            "PPO" | "PPOFIT" | "PPO_WIN" | "PCT_PRICE_OSC" | "PERCENT_PRICE_OSC" => {
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
                    self.ppo_win_symbol = sym;
                }
                self.show_ppo_win = true;
                if self.ppo_win_snapshot.symbol.is_empty() && !self.ppo_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_ppo(&conn, &self.ppo_win_symbol)
                            {
                                self.ppo_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "DPO" | "DPOFIT" | "DPO_WIN" | "DETRENDED_PRICE" | "DETRENDED_OSC" => {
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
                    self.dpo_win_symbol = sym;
                }
                self.show_dpo_win = true;
                if self.dpo_win_snapshot.symbol.is_empty() && !self.dpo_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_dpo(&conn, &self.dpo_win_symbol)
                            {
                                self.dpo_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "KST" | "KSTFIT" | "KST_WIN" | "KNOW_SURE_THING" | "PRING_KST" => {
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
                    self.kst_win_symbol = sym;
                }
                self.show_kst_win = true;
                if self.kst_win_snapshot.symbol.is_empty() && !self.kst_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_kst(&conn, &self.kst_win_symbol)
                            {
                                self.kst_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "ULTOSC"
            | "ULTOSCFIT"
            | "ULTOSC_WIN"
            | "ULTIMATE_OSC"
            | "ULTIMATE_OSCILLATOR"
            | "WILLIAMS_ULTOSC" => {
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
                    self.ultosc_win_symbol = sym;
                }
                self.show_ultosc_win = true;
                if self.ultosc_win_snapshot.symbol.is_empty() && !self.ultosc_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ultosc(
                                &conn,
                                &self.ultosc_win_symbol,
                            ) {
                                self.ultosc_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "WILLR" | "WILLRFIT" | "WILLR_WIN" | "WILLIAMS_R" | "WILLIAMS_PCT_R" | "PERCENT_R" => {
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
                    self.willr_win_symbol = sym;
                }
                self.show_willr_win = true;
                if self.willr_win_snapshot.symbol.is_empty() && !self.willr_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_willr(
                                &conn,
                                &self.willr_win_symbol,
                            ) {
                                self.willr_win_snapshot = snap;
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
