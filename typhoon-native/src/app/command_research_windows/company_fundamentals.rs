use super::*;

mod company_events_commands;
mod dividend_estimates_ratings_commands;
mod earnings_peers_commands;
mod financials_management_cot_commands;
mod fundamental_ratios_commands;
mod insider_fundamental_commands;
mod market_overview_commands;
mod sentiment_transcripts_tape_commands;
mod splits_etf_index_commands;

impl TyphooNApp {
    pub(super) fn handle_company_fundamentals_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // Company events, sentiment, transcripts, commodities, and tape research
            _ if self.handle_company_events_commands(cmd_upper) => {}
            _ if self.handle_sentiment_transcripts_tape_commands(cmd_upper) => {}
            _ if self.handle_dividend_estimates_ratings_commands(cmd_upper) => {}
            _ if self.handle_financials_management_cot_commands(cmd_upper) => {}
            _ if self.handle_splits_etf_index_commands(cmd_upper) => {}
            _ if self.handle_insider_fundamental_commands(cmd_upper) => {}
            _ if self.handle_market_overview_commands(cmd_upper) => {}
            _ if self.handle_fundamental_ratios_commands(cmd_upper) => {}
            // ── palette entries ──
            "HRA" | "HISTORICAL_RETURNS" | "RETURN_ANALYSIS" | "RISK_ANALYSIS" => {
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
                    self.hra_symbol = sym;
                }
                self.show_hra = true;
                if self.hra_snapshot.symbol.is_empty() && !self.hra_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_hra(&conn, &self.hra_symbol)
                            {
                                self.hra_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DCF" | "DISCOUNTED_CASH_FLOW" | "FAIR_VALUE" => {
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
                    self.dcf_symbol = sym;
                }
                self.show_dcf = true;
                if self.dcf_snapshot.symbol.is_empty() && !self.dcf_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_dcf(&conn, &self.dcf_symbol)
                            {
                                self.dcf_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SVM" | "STOCK_VALUATION" | "VALUATION_MODEL" | "FAIR_VALUE_SYNTHESIS" => {
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
                    self.svm_symbol = sym;
                }
                self.show_svm = true;
                if self.svm_snapshot.symbol.is_empty() && !self.svm_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_svm(&conn, &self.svm_symbol)
                            {
                                self.svm_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // Note: "OPTIONS" is intentionally omitted to preserve the legacy options arm below.
            "OMON" | "OPTIONS_CHAIN" | "OPT_CHAIN" => {
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
                    self.omon_symbol = sym;
                }
                self.show_omon = true;
                if self.omon_snapshot.symbol.is_empty() && !self.omon_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_options_chain(
                                    &conn,
                                    &self.omon_symbol,
                                )
                            {
                                self.omon_snapshot = snap;
                            }
                        }
                    }
                }
                if !self.omon_symbol.is_empty() {
                    self.omon_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchOptionsChain {
                        symbol: self.omon_symbol.to_uppercase(),
                    });
                }
            }
            "IVOL" | "IMPLIED_VOL" | "IV_RANK" | "IV_PERCENTILE" => {
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
                    self.ivol_symbol = sym;
                }
                self.show_ivol = true;
                if self.ivol_snapshot.symbol.is_empty() && !self.ivol_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_ivol(&conn, &self.ivol_symbol)
                            {
                                self.ivol_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SEAG" | "SEASONALITY" | "SEASONAL_ANALYSIS" | "SEASONAL" => {
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
                    self.seag_symbol = sym;
                }
                self.show_seag = true;
                if self.seag_snapshot.symbol.is_empty() && !self.seag_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_seasonality(
                                &conn,
                                &self.seag_symbol,
                            ) {
                                self.seag_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "COR" | "CORRELATION_MATRIX" | "CORR_MATRIX" | "PEER_CORR" => {
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
                    self.cor_symbol = sym;
                }
                self.show_cor = true;
                if self.cor_snapshot.symbol.is_empty() && !self.cor_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_correlation(
                                &conn,
                                &self.cor_symbol,
                            ) {
                                self.cor_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "TRA" | "TOTAL_RETURN" | "TOTAL_RETURN_ANALYSIS" | "TRET" => {
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
                    self.tra_symbol = sym;
                }
                self.show_tra = true;
                if self.tra_snapshot.symbol.is_empty() && !self.tra_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_total_return(
                                &conn,
                                &self.tra_symbol,
                            ) {
                                self.tra_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "TECH" | "TECHNICALS" | "TECHNICAL_INDICATORS" | "TA" => {
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
                    self.tech_symbol = sym;
                }
                self.show_tech = true;
                if self.tech_snapshot.symbol.is_empty() && !self.tech_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_technicals(
                                &conn,
                                &self.tech_symbol,
                            ) {
                                self.tech_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SKEW" | "VOL_SKEW" | "VOLATILITY_SKEW" | "SMILE" | "IV_SKEW" => {
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
                    self.skew_symbol = sym;
                }
                self.show_skew = true;
                if self.skew_snapshot.symbol.is_empty() && !self.skew_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_vol_skew(
                                &conn,
                                &self.skew_symbol,
                            ) {
                                self.skew_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // Leverage, accruals, realized-volatility, cash-flow, and short-interest research
            "LEV" | "LEVERAGE" | "DEBT_LEVERAGE" | "SOLVENCY" => {
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
                    self.lev_symbol = sym;
                }
                self.show_lev = true;
                if self.lev_snapshot.symbol.is_empty() && !self.lev_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_leverage(
                                &conn,
                                &self.lev_symbol,
                            ) {
                                self.lev_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ACRL" | "ACCRUALS" | "EARNINGS_QUALITY" | "FCF_QUALITY" => {
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
                    self.acrl_symbol = sym;
                }
                self.show_acrl = true;
                if self.acrl_snapshot.symbol.is_empty() && !self.acrl_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_accruals(
                                &conn,
                                &self.acrl_symbol,
                            ) {
                                self.acrl_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RVOL" | "REALIZED_VOL" | "VOL_CONE" | "HV" => {
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
                    self.rvol_symbol = sym;
                }
                self.show_rvol = true;
                if self.rvol_snapshot.symbol.is_empty() && !self.rvol_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_realized_vol(
                                &conn,
                                &self.rvol_symbol,
                            ) {
                                self.rvol_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "FCFY" | "FCF_YIELD" | "PAYOUT" | "DIV_SUSTAINABILITY" => {
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
                    self.fcfy_symbol = sym;
                }
                self.show_fcfy = true;
                if self.fcfy_snapshot.symbol.is_empty() && !self.fcfy_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_fcf_yield(
                                &conn,
                                &self.fcfy_symbol,
                            ) {
                                self.fcfy_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SHRT" | "DTC" | "DAYS_TO_COVER" | "SHORT_FLOAT" => {
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
                    self.shrt_symbol = sym;
                }
                self.show_shrt = true;
                if self.shrt_snapshot.symbol.is_empty() && !self.shrt_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_short_interest(
                                    &conn,
                                    &self.shrt_symbol,
                                )
                            {
                                self.shrt_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // Solvency, quality, volatility-estimator, EPS-beat, and price-target research
            "ALTZ" | "ALTMAN" | "Z_SCORE" | "BANKRUPTCY_RISK" => {
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
                    self.altz_symbol = sym;
                }
                self.show_altz = true;
                if self.altz_snapshot.symbol.is_empty() && !self.altz_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_altman_z(
                                &conn,
                                &self.altz_symbol,
                            ) {
                                self.altz_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PTFS" | "PIOTROSKI" | "F_SCORE" | "QUALITY_SCORE" => {
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
                    self.ptfs_symbol = sym;
                }
                self.show_ptfs = true;
                if self.ptfs_snapshot.symbol.is_empty() && !self.ptfs_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_piotroski(
                                &conn,
                                &self.ptfs_symbol,
                            ) {
                                self.ptfs_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VOLE" | "OHLC_VOL" | "VOL_ESTIMATORS" | "YANG_ZHANG" => {
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
                    self.vole_symbol = sym;
                }
                self.show_vole = true;
                if self.vole_snapshot.symbol.is_empty() && !self.vole_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ohlc_vol(
                                &conn,
                                &self.vole_symbol,
                            ) {
                                self.vole_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "EPSB" | "EPS_BEAT" | "BEAT_STREAK" | "SURPRISE_HISTORY" => {
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
                    self.epsb_symbol = sym;
                }
                self.show_epsb = true;
                if self.epsb_snapshot.symbol.is_empty() && !self.epsb_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_eps_beat(
                                &conn,
                                &self.epsb_symbol,
                            ) {
                                self.epsb_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PTD" | "TARGET_DISPERSION" | "IMPLIED_RETURN" | "CONSENSUS_TARGET" => {
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
                    self.ptd_symbol = sym;
                }
                self.show_ptd = true;
                if self.ptd_snapshot.symbol.is_empty() && !self.ptd_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_price_target_dispersion(
                                    &conn,
                                    &self.ptd_symbol,
                                )
                            {
                                self.ptd_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MNGR" | "INSIDER_BIAS" | "INSIDER_ACTIVITY" | "INSIDER_SCORE" => {
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
                    self.mngr_symbol = sym;
                }
                self.show_mngr = true;
                if self.mngr_snapshot.symbol.is_empty() && !self.mngr_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_insider_activity(
                                    &conn,
                                    &self.mngr_symbol,
                                )
                            {
                                self.mngr_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DIVG" | "DIV_GROWTH" | "DIVIDEND_GROWTH" | "DIV_CAGR" => {
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
                    self.divg_symbol = sym;
                }
                self.show_divg = true;
                if self.divg_snapshot.symbol.is_empty() && !self.divg_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_divg(&conn, &self.divg_symbol)
                            {
                                self.divg_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "EARM" | "EARN_MOMENTUM" | "EARNINGS_MOMENTUM" | "REV_MOMENTUM" => {
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
                    self.earm_symbol = sym;
                }
                self.show_earm = true;
                if self.earm_snapshot.symbol.is_empty() && !self.earm_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_earm(&conn, &self.earm_symbol)
                            {
                                self.earm_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SECTR" | "SECT_ROT" | "SECTOR_STRENGTH" | "RS_SECTOR" => {
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
                    self.sectr_symbol = sym;
                }
                self.show_sectr = true;
                if self.sectr_snapshot.symbol.is_empty() && !self.sectr_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_sector_rotation(
                                    &conn,
                                    &self.sectr_symbol,
                                )
                            {
                                self.sectr_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "UPDM" | "UPGRADE_MOMENTUM" | "RATING_MOMENTUM" | "ANALYST_MOMENTUM" => {
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
                    self.updm_symbol = sym;
                }
                self.show_updm = true;
                if self.updm_snapshot.symbol.is_empty() && !self.updm_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_updm(&conn, &self.updm_symbol)
                            {
                                self.updm_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // Momentum, liquidity, breakout, cash-cycle, and credit research
            "MOM" | "MOMENTUM" | "MOM_SCORE" | "MOMENTUM_12_1" => {
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
                    self.mom_symbol = sym;
                }
                self.show_mom = true;
                if self.mom_snapshot.symbol.is_empty() && !self.mom_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_momentum(
                                &conn,
                                &self.mom_symbol,
                            ) {
                                self.mom_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "LIQ" | "LIQUIDITY" | "LIQUIDITY_PROFILE" => {
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
                    self.liq_symbol = sym;
                }
                self.show_liq = true;
                if self.liq_snapshot.symbol.is_empty() && !self.liq_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_liquidity(
                                &conn,
                                &self.liq_symbol,
                            ) {
                                self.liq_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "BREAK" | "BREAKOUT" | "BREAKOUT_PROXIMITY" | "BRK_PROX" => {
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
                    self.break_symbol = sym;
                }
                self.show_break = true;
                if self.break_snapshot.symbol.is_empty() && !self.break_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_breakout(
                                &conn,
                                &self.break_symbol,
                            ) {
                                self.break_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CCRL" | "CASH_CYCLE" | "CCC" | "WORKING_CAPITAL_CYCLE" => {
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
                    self.ccrl_symbol = sym;
                }
                self.show_ccrl = true;
                if self.ccrl_snapshot.symbol.is_empty() && !self.ccrl_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cash_cycle(
                                &conn,
                                &self.ccrl_symbol,
                            ) {
                                self.ccrl_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CREDIT" | "CREDIT_SCORE" | "LETTER_GRADE" | "COMPOSITE_CREDIT" => {
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
                    self.credit_symbol = sym;
                }
                self.show_credit = true;
                if self.credit_snapshot.symbol.is_empty() && !self.credit_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_credit(
                                &conn,
                                &self.credit_symbol,
                            ) {
                                self.credit_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "GROWM" | "GARP" | "GROWTH" => {
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
                    self.growm_symbol = sym;
                }
                self.show_growm = true;
                if self.growm_snapshot.symbol.is_empty() && !self.growm_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_growm(&conn, &self.growm_symbol)
                            {
                                self.growm_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "FLOW" | "SMART_MONEY" | "INSIDER_FLOW" => {
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
                    self.flow_symbol = sym;
                }
                self.show_flow = true;
                if self.flow_snapshot.symbol.is_empty() && !self.flow_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_flow(&conn, &self.flow_symbol)
                            {
                                self.flow_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "REGIME" | "MARKET_REGIME" | "REGIME_CLASSIFIER" => {
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
                    self.regime_symbol = sym;
                }
                self.show_regime = true;
                if self.regime_snapshot.symbol.is_empty() && !self.regime_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_regime(
                                &conn,
                                &self.regime_symbol,
                            ) {
                                self.regime_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RELVOL" | "REL_VOLUME" | "RELATIVE_VOLUME" | "RELVOLUME" => {
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
                    self.relvol_symbol = sym;
                }
                self.show_relvol = true;
                if self.relvol_snapshot.symbol.is_empty() && !self.relvol_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_relvol(
                                &conn,
                                &self.relvol_symbol,
                            ) {
                                self.relvol_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MARGINS" | "MARGIN_TRAJECTORY" | "MARGIN_TREND" | "MARGIN_HISTORY" => {
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
                    self.margins_symbol = sym;
                }
                self.show_margins = true;
                if self.margins_snapshot.symbol.is_empty() && !self.margins_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_margins(
                                &conn,
                                &self.margins_symbol,
                            ) {
                                self.margins_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VAL" | "VALUE_FACTOR" | "VALUE_COMPOSITE" => {
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
                    self.val_symbol = sym;
                }
                self.show_val = true;
                if self.val_snapshot.symbol.is_empty() && !self.val_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_val(&conn, &self.val_symbol)
                            {
                                self.val_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "QUAL" | "QUALITY_FACTOR" | "QUALITY_COMPOSITE" => {
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
                    self.qual_symbol = sym;
                }
                self.show_qual = true;
                if self.qual_snapshot.symbol.is_empty() && !self.qual_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_qual(&conn, &self.qual_symbol)
                            {
                                self.qual_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RISK" | "RISK_FACTOR" | "RISK_COMPOSITE" => {
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
                    self.risk_symbol = sym;
                }
                self.show_risk = true;
                if self.risk_snapshot.symbol.is_empty() && !self.risk_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_risk(&conn, &self.risk_symbol)
                            {
                                self.risk_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "INSSTRK" | "INSIDER_STREAK" | "INSIDER_STREAKS" => {
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
                    self.insstrk_symbol = sym;
                }
                self.show_insstrk = true;
                if self.insstrk_snapshot.symbol.is_empty() && !self.insstrk_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_insstrk(
                                &conn,
                                &self.insstrk_symbol,
                            ) {
                                self.insstrk_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "COVG" | "COVERAGE" | "ANALYST_COVERAGE" | "COVERAGE_BREADTH" => {
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
                    self.covg_symbol = sym;
                }
                self.show_covg = true;
                if self.covg_snapshot.symbol.is_empty() && !self.covg_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_covg(&conn, &self.covg_symbol)
                            {
                                self.covg_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VRK" | "VALUE_RANK" | "VAL_RANK" => {
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
                    self.vrk_symbol = sym;
                }
                self.show_vrk = true;
                if self.vrk_snapshot.symbol.is_empty() && !self.vrk_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_vrk(&conn, &self.vrk_symbol)
                            {
                                self.vrk_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "QRK" | "QUALITY_RANK" | "QUAL_RANK" => {
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
                    self.qrk_symbol = sym;
                }
                self.show_qrk = true;
                if self.qrk_snapshot.symbol.is_empty() && !self.qrk_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_qrk(&conn, &self.qrk_symbol)
                            {
                                self.qrk_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RRK" | "RISK_RANK" => {
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
                    self.rrk_symbol = sym;
                }
                self.show_rrk = true;
                if self.rrk_snapshot.symbol.is_empty() && !self.rrk_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_rrk(&conn, &self.rrk_symbol)
                            {
                                self.rrk_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RELEPSGR" | "REL_EPS_GROWTH" | "RELATIVE_EPS_GROWTH" | "EPSGR" => {
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
                    self.relepsgr_symbol = sym;
                }
                self.show_relepsgr = true;
                if self.relepsgr_snapshot.symbol.is_empty() && !self.relepsgr_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_relepsgr(
                                &conn,
                                &self.relepsgr_symbol,
                            ) {
                                self.relepsgr_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PEAD" | "EARNINGS_DRIFT" | "POST_EARNINGS_DRIFT" => {
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
                    self.pead_symbol = sym;
                }
                self.show_pead = true;
                if self.pead_snapshot.symbol.is_empty() && !self.pead_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_pead(&conn, &self.pead_symbol)
                            {
                                self.pead_snapshot = snap;
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
