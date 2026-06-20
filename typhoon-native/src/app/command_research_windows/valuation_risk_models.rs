use super::*;

mod downside_efficiency_volatility_commands;
mod drawdown_seasonality_spread_commands;
mod event_dividend_risk_rank_commands;
mod factor_growth_quality_commands;
mod leverage_quality_liquidity_rank_commands;
mod price_path_gap_volatility_commands;
mod range_volatility_calendar_tail_commands;
mod return_distribution_tail_commands;
mod reward_risk_serial_liquidity_commands;
mod risk_adjusted_liquidity_normality_commands;

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
            // ── palette ──
            "PSR"
            | "PROB_SHARPE"
            | "PROBSHARPE"
            | "PROBABILISTIC_SHARPE"
            | "PROBABILISTICSHARPE" => {
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
                    self.psr_symbol = sym;
                }
                self.show_psr = true;
                if self.psr_snapshot.symbol.is_empty() && !self.psr_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_psr(&conn, &self.psr_symbol)
                            {
                                self.psr_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ADF" | "DICKEY_FULLER" | "DICKEYFULLER" | "UNIT_ROOT" | "UNITROOT"
            | "STATIONARITY" => {
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
                    self.adf_symbol = sym;
                }
                self.show_adf = true;
                if self.adf_snapshot.symbol.is_empty() && !self.adf_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_adf(&conn, &self.adf_symbol)
                            {
                                self.adf_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MNKENDALL" | "MANN_KENDALL" | "MANNKENDALL" | "KENDALL_TREND" | "TREND_TEST" => {
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
                    self.mnkendall_symbol = sym;
                }
                self.show_mnkendall = true;
                if self.mnkendall_snapshot.symbol.is_empty() && !self.mnkendall_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_mnkendall(
                                &conn,
                                &self.mnkendall_symbol,
                            ) {
                                self.mnkendall_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "BIPOWER" | "BPV" | "BIPOWER_VAR" | "BIPOWERVAR" | "JUMP_RATIO" | "JUMPRATIO"
            | "BN_JUMP" => {
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
                    self.bipower_symbol = sym;
                }
                self.show_bipower = true;
                if self.bipower_snapshot.symbol.is_empty() && !self.bipower_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_bipower(
                                &conn,
                                &self.bipower_symbol,
                            ) {
                                self.bipower_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DDDUR" | "DD_DURATION" | "DRAWDOWN_DURATION" | "DDDURATION" | "UNDERWATER"
            | "DRAWDOWNDURATION" => {
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
                    self.dddur_symbol = sym;
                }
                self.show_dddur = true;
                if self.dddur_snapshot.symbol.is_empty() && !self.dddur_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_dddur(&conn, &self.dddur_symbol)
                            {
                                self.dddur_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── palette ──
            "HILLTAIL" | "HILL" | "HILL_TAIL" | "TAIL_INDEX" | "TAILINDEX" | "HILLESTIMATOR"
            | "POWER_LAW_TAIL" => {
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
                    self.hilltail_symbol = sym;
                }
                self.show_hilltail = true;
                if self.hilltail_snapshot.symbol.is_empty() && !self.hilltail_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_hilltail(
                                &conn,
                                &self.hilltail_symbol,
                            ) {
                                self.hilltail_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ARCHLM" | "ARCH_LM" | "ENGLE_ARCH" | "ARCH_TEST" | "HETEROSKEDASTIC"
            | "HETERO_TEST" | "VOLCLUSTER_TEST" => {
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
                    self.archlm_symbol = sym;
                }
                self.show_archlm = true;
                if self.archlm_snapshot.symbol.is_empty() && !self.archlm_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_archlm(
                                &conn,
                                &self.archlm_symbol,
                            ) {
                                self.archlm_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PAINRATIO" | "PAIN_RATIO" | "PAIN_INDEX" | "PAININDEX" | "PAIN" | "ZEPHYR_PAIN" => {
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
                    self.painratio_symbol = sym;
                }
                self.show_painratio = true;
                if self.painratio_snapshot.symbol.is_empty() && !self.painratio_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_painratio(
                                &conn,
                                &self.painratio_symbol,
                            ) {
                                self.painratio_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CUSUM" | "BDE_CUSUM" | "STRUCTURAL_BREAK" | "MEAN_BREAK" | "CUSUM_TEST"
            | "STABILITY_TEST" => {
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
                    self.cusum_symbol = sym;
                }
                self.show_cusum = true;
                if self.cusum_snapshot.symbol.is_empty() && !self.cusum_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cusum(&conn, &self.cusum_symbol)
                            {
                                self.cusum_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CFVAR" | "CORNISH_FISHER" | "CORNISHFISHER" | "MODIFIED_VAR" | "MODIFIEDVAR"
            | "CF_VAR" | "SKEW_KURT_VAR" => {
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
                    self.cfvar_symbol = sym;
                }
                self.show_cfvar = true;
                if self.cfvar_snapshot.symbol.is_empty() && !self.cfvar_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cfvar(&conn, &self.cfvar_symbol)
                            {
                                self.cfvar_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── palette ──
            "ENTROPY" | "SHANNON" | "SHANNON_ENTROPY" | "SHANNONENTROPY" | "RETURN_ENTROPY"
            | "RETURNENTROPY" => {
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
                    self.entropy_symbol = sym;
                }
                self.show_entropy = true;
                if self.entropy_snapshot.symbol.is_empty() && !self.entropy_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_entropy(
                                &conn,
                                &self.entropy_symbol,
                            ) {
                                self.entropy_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RACHEV"
            | "RACHEV_RATIO"
            | "RACHEVRATIO"
            | "ETL_RATIO"
            | "ETLRATIO"
            | "TAIL_EXPECTATION_RATIO" => {
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
                    self.rachev_symbol = sym;
                }
                self.show_rachev = true;
                if self.rachev_snapshot.symbol.is_empty() && !self.rachev_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_rachev(
                                &conn,
                                &self.rachev_symbol,
                            ) {
                                self.rachev_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "GPR" | "GAIN_TO_PAIN" | "GAINTOPAIN" | "GAIN_PAIN" | "GAINPAIN" | "PROFIT_FACTOR"
            | "PROFITFACTOR" => {
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
                    self.gpr_symbol = sym;
                }
                self.show_gpr = true;
                if self.gpr_snapshot.symbol.is_empty() && !self.gpr_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_gpr(&conn, &self.gpr_symbol)
                            {
                                self.gpr_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PACF"
            | "PARTIAL_ACF"
            | "PARTIALACF"
            | "PARTIAL_AUTOCORRELATION"
            | "PARTIALAUTOCORRELATION"
            | "PACF_LAG" => {
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
                    self.pacf_symbol = sym;
                }
                self.show_pacf = true;
                if self.pacf_snapshot.symbol.is_empty() && !self.pacf_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_pacf(&conn, &self.pacf_symbol)
                            {
                                self.pacf_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "APEN"
            | "APPROX_ENTROPY"
            | "APPROXENTROPY"
            | "APPROXIMATE_ENTROPY"
            | "APPROXIMATEENTROPY"
            | "PINCUS" => {
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
                    self.apen_symbol = sym;
                }
                self.show_apen = true;
                if self.apen_snapshot.symbol.is_empty() && !self.apen_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_apen(&conn, &self.apen_symbol)
                            {
                                self.apen_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── palette ──
            "UPR" | "UPSIDE_POTENTIAL" | "UPSIDEPOTENTIAL" | "UPSIDE_RATIO" | "UPSIDERATIO" => {
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
                    self.upr_symbol = sym;
                }
                self.show_upr = true;
                if self.upr_snapshot.symbol.is_empty() && !self.upr_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_upr(&conn, &self.upr_symbol)
                            {
                                self.upr_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "LEVEREFF" | "LEVERAGE_EFFECT" | "LEVERAGEEFFECT" | "LEVER_EFF" | "ASYM_VOL"
            | "ASYMVOL" => {
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
                    self.levereff_symbol = sym;
                }
                self.show_levereff = true;
                if self.levereff_snapshot.symbol.is_empty() && !self.levereff_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_levereff(
                                &conn,
                                &self.levereff_symbol,
                            ) {
                                self.levereff_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DRAWDAR" | "DRAWDOWN_AT_RISK" | "DRAWDOWNATRISK" | "DAR" | "CDAR"
            | "CONDITIONAL_DAR" => {
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
                    self.drawdar_symbol = sym;
                }
                self.show_drawdar = true;
                if self.drawdar_snapshot.symbol.is_empty() && !self.drawdar_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_drawdar(
                                &conn,
                                &self.drawdar_symbol,
                            ) {
                                self.drawdar_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VARHALF"
            | "VOL_HALFLIFE"
            | "VOLHALFLIFE"
            | "VOL_PERSIST"
            | "VOLPERSIST"
            | "VOLATILITY_HALFLIFE" => {
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
                    self.varhalf_symbol = sym;
                }
                self.show_varhalf = true;
                if self.varhalf_snapshot.symbol.is_empty() && !self.varhalf_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_varhalf(
                                &conn,
                                &self.varhalf_symbol,
                            ) {
                                self.varhalf_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "GINI" | "GINI_COEFF" | "GINICOEFF" | "GINI_COEFFICIENT" | "RETURN_CONCENTRATION" => {
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
                    self.gini_symbol = sym;
                }
                self.show_gini = true;
                if self.gini_snapshot.symbol.is_empty() && !self.gini_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_gini(&conn, &self.gini_symbol)
                            {
                                self.gini_snapshot = snap;
                            }
                        }
                    }
                }
            }
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
