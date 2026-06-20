use super::*;

mod directional_moneyflow_sar_commands;
mod distribution_entropy_commands;
mod fractal_tail_dependence_commands;
mod jump_stationarity_tail_commands;
mod residual_cycle_memory_commands;
mod squeeze_channel_adaptive_commands;
mod trend_channel_transform_commands;
mod volatility_bubble_nonlinearity_commands;
mod volume_choppiness_moving_average_commands;

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
            }
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
            }
            // ── Linear regression and Hilbert-transform palette aliases ──
            // Bare EFI / EMV / NVI / PVI / COPPOCK are unbound upstream (verified) and kept as aliases.
            "EFI" | "EFIFIT" | "EFI_WIN" | "FORCE_INDEX" | "ELDER_FORCE" => {
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
                    self.efi_win_symbol = sym;
                }
                self.show_efi_win = true;
                if self.efi_win_snapshot.symbol.is_empty() && !self.efi_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_efi(&conn, &self.efi_win_symbol)
                            {
                                self.efi_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "EMV" | "EMVFIT" | "EMV_WIN" | "EASE_OF_MOVEMENT" | "ARMS_EMV" => {
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
                    self.emv_win_symbol = sym;
                }
                self.show_emv_win = true;
                if self.emv_win_snapshot.symbol.is_empty() && !self.emv_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_emv(&conn, &self.emv_win_symbol)
                            {
                                self.emv_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "NVI" | "NVIFIT" | "NVI_WIN" | "NEG_VOLUME_INDEX" | "NEGATIVE_VOLUME" => {
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
                    self.nvi_win_symbol = sym;
                }
                self.show_nvi_win = true;
                if self.nvi_win_snapshot.symbol.is_empty() && !self.nvi_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_nvi(&conn, &self.nvi_win_symbol)
                            {
                                self.nvi_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PVI" | "PVIFIT" | "PVI_WIN" | "POS_VOLUME_INDEX" | "POSITIVE_VOLUME" => {
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
                    self.pvi_win_symbol = sym;
                }
                self.show_pvi_win = true;
                if self.pvi_win_snapshot.symbol.is_empty() && !self.pvi_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_pvi(&conn, &self.pvi_win_symbol)
                            {
                                self.pvi_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "COPPOCK" | "COPPOCKFIT" | "COPPOCK_WIN" | "COPPOCK_CURVE" | "COPPOCK_GUIDE" => {
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
                    self.coppock_win_symbol = sym;
                }
                self.show_coppock_win = true;
                if self.coppock_win_snapshot.symbol.is_empty()
                    && !self.coppock_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_coppock(
                                &conn,
                                &self.coppock_win_symbol,
                            ) {
                                self.coppock_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CMO" | "CMOFIT" | "CMO_WIN" | "CHANDE_MOMENTUM" | "CHANDE_MO" => {
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
                    self.cmo_win_symbol = sym;
                }
                self.show_cmo_win = true;
                if self.cmo_win_snapshot.symbol.is_empty() && !self.cmo_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cmo(&conn, &self.cmo_win_symbol)
                            {
                                self.cmo_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "QSTICK" | "QSTICKFIT" | "QSTICK_WIN" | "Q_STICK" | "CHANDE_QSTICK" => {
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
                    self.qstick_win_symbol = sym;
                }
                self.show_qstick_win = true;
                if self.qstick_win_snapshot.symbol.is_empty() && !self.qstick_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_qstick(
                                &conn,
                                &self.qstick_win_symbol,
                            ) {
                                self.qstick_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DISPARITY" | "DISPARITYFIT" | "DISPARITY_WIN" | "DISPARITY_INDEX" | "DISP_INDEX" => {
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
                    self.disparity_win_symbol = sym;
                }
                self.show_disparity_win = true;
                if self.disparity_win_snapshot.symbol.is_empty()
                    && !self.disparity_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_disparity(
                                &conn,
                                &self.disparity_win_symbol,
                            ) {
                                self.disparity_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "BOP" | "BOPFIT" | "BOP_WIN" | "BALANCE_OF_POWER" | "LIVSHIN_BOP" => {
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
                    self.bop_win_symbol = sym;
                }
                self.show_bop_win = true;
                if self.bop_win_snapshot.symbol.is_empty() && !self.bop_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_bop(&conn, &self.bop_win_symbol)
                            {
                                self.bop_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SCHAFF" | "SCHAFFFIT" | "SCHAFF_WIN" | "STC" | "SCHAFF_TREND_CYCLE" => {
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
                    self.schaff_win_symbol = sym;
                }
                self.show_schaff_win = true;
                if self.schaff_win_snapshot.symbol.is_empty() && !self.schaff_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_schaff(
                                &conn,
                                &self.schaff_win_symbol,
                            ) {
                                self.schaff_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
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
