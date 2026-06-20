use super::*;

mod downside_efficiency_volatility_commands;
mod drawdown_seasonality_spread_commands;
mod entropy_tail_reward_memory_commands;
mod event_dividend_risk_rank_commands;
mod factor_growth_quality_commands;
mod leverage_quality_liquidity_rank_commands;
mod price_path_gap_volatility_commands;
mod range_volatility_calendar_tail_commands;
mod return_distribution_tail_commands;
mod reward_risk_serial_liquidity_commands;
mod risk_adjusted_liquidity_normality_commands;
mod stationarity_jump_drawdown_commands;
mod tail_heteroskedasticity_stability_commands;
mod upside_leverage_concentration_commands;

impl TyphooNApp {
    pub(super) fn handle_valuation_risk_model_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            _ if self.handle_factor_growth_quality_command(cmd_upper) => {}
            _ if self.handle_leverage_quality_liquidity_rank_command(cmd_upper) => {}
            _ if self.handle_event_dividend_risk_rank_command(cmd_upper) => {}
            _ if self.handle_return_distribution_tail_command(cmd_upper) => {}
            _ if self.handle_price_path_gap_volatility_command(cmd_upper) => {}
            _ if self.handle_downside_efficiency_volatility_command(cmd_upper) => {}
            _ if self.handle_risk_adjusted_liquidity_normality_command(cmd_upper) => {}
            _ if self.handle_drawdown_seasonality_spread_command(cmd_upper) => {}
            _ if self.handle_range_volatility_calendar_tail_command(cmd_upper) => {}
            _ if self.handle_reward_risk_serial_liquidity_command(cmd_upper) => {}
            _ if self.handle_stationarity_jump_drawdown_command(cmd_upper) => {}
            _ if self.handle_tail_heteroskedasticity_stability_command(cmd_upper) => {}
            _ if self.handle_entropy_tail_reward_memory_command(cmd_upper) => {}
            _ if self.handle_upside_leverage_concentration_command(cmd_upper) => {}
            // ── palette ──
            "SAMPEN" | "SAMPLE_ENTROPY" | "SAMPLEENTROPY" => {
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
                    self.sampen_symbol = sym;
                }
                self.show_sampen = true;
                if self.sampen_snapshot.symbol.is_empty() && !self.sampen_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_sampen(
                                &conn,
                                &self.sampen_symbol,
                            ) {
                                self.sampen_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PERMEN" | "PERMUTATION_ENTROPY" | "PERMENTROPY" => {
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
                    self.permen_symbol = sym;
                }
                self.show_permen = true;
                if self.permen_snapshot.symbol.is_empty() && !self.permen_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_permen(
                                &conn,
                                &self.permen_symbol,
                            ) {
                                self.permen_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RECFACT" | "RECOVERY_FACTOR" | "RECOVERYFACTOR" => {
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
                    self.recfact_symbol = sym;
                }
                self.show_recfact = true;
                if self.recfact_snapshot.symbol.is_empty() && !self.recfact_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_recfact(
                                &conn,
                                &self.recfact_symbol,
                            ) {
                                self.recfact_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "KPSS" | "KPSS_TEST" | "KPSSTEST" => {
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
                    self.kpss_symbol = sym;
                }
                self.show_kpss = true;
                if self.kpss_snapshot.symbol.is_empty() && !self.kpss_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_kpss(&conn, &self.kpss_symbol)
                            {
                                self.kpss_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SPECENT" | "SPECTRAL_ENTROPY" | "SPECTRALENTROPY" => {
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
                    self.specent_symbol = sym;
                }
                self.show_specent = true;
                if self.specent_snapshot.symbol.is_empty() && !self.specent_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_specent(
                                &conn,
                                &self.specent_symbol,
                            ) {
                                self.specent_snapshot = snap;
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
