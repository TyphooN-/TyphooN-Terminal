use super::*;

mod company_events_commands;
mod dividend_estimates_ratings_commands;
mod earnings_peers_commands;
mod sentiment_transcripts_tape_commands;

impl TyphooNApp {
    pub(super) fn handle_company_fundamentals_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // Company events, sentiment, transcripts, commodities, and tape research
            _ if self.handle_company_events_commands(cmd_upper) => {}
            _ if self.handle_sentiment_transcripts_tape_commands(cmd_upper) => {}
            _ if self.handle_dividend_estimates_ratings_commands(cmd_upper) => {}
            // Financial statements, management, and COT research
            "FA" | "FINANCIALS" | "INCOME" | "BALANCE" | "CASHFLOW" => {
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
                    self.financials_symbol = sym.clone();
                }
                self.show_financials = true;
                self.financials_view = match cmd_upper.as_str() {
                    "BALANCE" => FinancialsView::Balance,
                    "CASHFLOW" => FinancialsView::CashFlow,
                    _ => FinancialsView::Income,
                };
                if !self.fmp_key.is_empty() && !self.financials_symbol.is_empty() {
                    self.financials_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchFinancialStatements {
                        symbol: self.financials_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "MGMT" | "MANAGEMENT" | "OFFICERS" | "EXECUTIVES" => {
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
                    self.executives_symbol = sym.clone();
                }
                self.show_executives = true;
                if !self.finnhub_key.is_empty() && !self.executives_symbol.is_empty() {
                    self.executives_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchExecutives {
                        symbol: self.executives_symbol.to_uppercase(),
                        finnhub_key: self.finnhub_key.clone(),
                    });
                }
            }
            "COT" | "COMMITMENTS" | "POSITIONING" => {
                self.show_cot = true;
                self.cot_loading = true;
                let _ = self.broker_tx.send(BrokerCmd::FetchCotReports);
            }
            // ── palette entries ──
            "SPLT" | "SPLIT" | "SPLITS" | "STOCK_SPLIT" => {
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
                    self.splits_symbol = sym.clone();
                }
                self.show_splits = true;
                if !self.splits_symbol.is_empty() {
                    self.splits_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchStockSplits {
                        symbol: self.splits_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "ETF" | "HOLDINGS" | "ETF_HOLDINGS" | "FUND" => {
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
                    self.etf_symbol = sym.clone();
                }
                self.show_etf_holdings = true;
                if !self.fmp_key.is_empty() && !self.etf_symbol.is_empty() {
                    self.etf_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchEtfHoldings {
                        symbol: self.etf_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "ANR" | "ANALYST_RECS" | "RECOMMENDATIONS" | "PRICE_TARGET" | "PT" | "TARGET" => {
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
                    self.anr_symbol = sym.clone();
                }
                self.show_analyst_recs = true;
                if !self.finnhub_key.is_empty() && !self.anr_symbol.is_empty() {
                    self.anr_loading = true;
                    let sym_u = self.anr_symbol.to_uppercase();
                    let _ = self.broker_tx.send(BrokerCmd::FetchAnalystRecs {
                        symbol: sym_u.clone(),
                        finnhub_key: self.finnhub_key.clone(),
                    });
                    let _ = self.broker_tx.send(BrokerCmd::FetchPriceTarget {
                        symbol: sym_u,
                        finnhub_key: self.finnhub_key.clone(),
                    });
                }
            }
            "ESG" | "SUSTAINABILITY" => {
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
                    self.esg_symbol = sym.clone();
                }
                self.show_esg = true;
                if !self.fmp_key.is_empty() && !self.esg_symbol.is_empty() {
                    self.esg_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchEsgScores {
                        symbol: self.esg_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "MEMB" | "MEMBERS" | "CONSTITUENTS" | "SP500" | "NDX" | "DJIA" => {
                let requested = match cmd_upper.as_str() {
                    "NDX" => "NDX",
                    "DJIA" => "DJIA",
                    "SP500" => "SP500",
                    _ => {
                        if self.index_code.is_empty() {
                            "SP500"
                        } else {
                            self.index_code.as_str()
                        }
                    }
                };
                self.index_code = requested.to_string();
                self.show_index_members = true;
                if !self.fmp_key.is_empty() {
                    self.memb_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchIndexMembers {
                        index_code: self.index_code.clone(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            // ── palette entries ──
            "INS" | "INSIDER_TRADES" | "FORM4" => {
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
                    self.insider_symbol = sym.clone();
                }
                self.show_insider_trades = true;
                if !self.fmp_key.is_empty() && !self.insider_symbol.is_empty() {
                    self.insider_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchInsiderTrades {
                        symbol: self.insider_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "HDS" | "INST" | "INSTITUTIONAL" | "13F" => {
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
                    self.inst_holders_symbol = sym.clone();
                }
                self.show_inst_holders = true;
                if !self.fmp_key.is_empty() && !self.inst_holders_symbol.is_empty() {
                    self.inst_holders_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchInstitutionalHolders {
                        symbol: self.inst_holders_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "FLOAT" | "SHARES" | "OUTSTANDING" => {
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
                    self.float_symbol = sym.clone();
                }
                self.show_shares_float = true;
                if !self.fmp_key.is_empty() && !self.float_symbol.is_empty() {
                    self.float_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchSharesFloat {
                        symbol: self.float_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "HP" | "HIST" | "HISTORICAL" | "PRICE_HISTORY" => {
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
                    self.hp_symbol = sym.clone();
                }
                self.show_hist_price = true;
                if !self.fmp_key.is_empty() && !self.hp_symbol.is_empty() {
                    self.hp_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchHistoricalPrice {
                        symbol: self.hp_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                        limit: self.hp_limit.max(50),
                    });
                }
            }
            "EPS" | "SURPRISE" | "EARNINGS_SURPRISE" => {
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
                    self.eps_symbol = sym.clone();
                }
                self.show_eps_surprise = true;
                if !self.fmp_key.is_empty() && !self.eps_symbol.is_empty() {
                    self.eps_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchEarningsSurprises {
                        symbol: self.eps_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            // ── palette entries ──
            // (intentionally omits "INDICES" to preserve the legacy ETF dashboard below)
            "WEI" | "GLOBAL_INDICES" => {
                self.show_wei = true;
                // Load cached snapshot first for instant display.
                if self.wei_indices.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(rows)) =
                                typhoon_engine::core::research::get_world_indices(&conn)
                            {
                                self.wei_indices = rows;
                            }
                        }
                    }
                }
                // Then kick a fresh fetch (Yahoo, no API key).
                self.wei_loading = true;
                let _ = self.broker_tx.send(BrokerCmd::FetchWorldIndices);
            }
            // (intentionally omits "MOVERS" to preserve the legacy broker top-movers arm below)
            "MOV" | "GAINERS" | "LOSERS" | "ACTIVES" => {
                self.show_market_movers = true;
                if self.market_movers.gainers.is_empty()
                    && self.market_movers.losers.is_empty()
                    && self.market_movers.actives.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(mov)) =
                                typhoon_engine::core::research::get_market_movers(&conn)
                            {
                                self.market_movers = mov;
                            }
                        }
                    }
                }
                if !self.fmp_key.is_empty() {
                    self.mov_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchMarketMovers {
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "INDU" | "SECTOR" | "SECTORS" | "SECTOR_PERFORMANCE" => {
                self.show_sector_perf = true;
                if self.sector_perf.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(rows)) =
                                typhoon_engine::core::research::get_sector_performance(&conn)
                            {
                                self.sector_perf = rows;
                            }
                        }
                    }
                }
                if !self.fmp_key.is_empty() {
                    self.indu_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchSectorPerformance {
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "CACS" | "CORP_ACTIONS" | "CORPORATE_ACTIONS" | "ACTIONS" => {
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
                    self.cacs_symbol = sym;
                }
                self.show_cacs = true;
                // No fetcher — the window aggregates cached splits/dividends/earnings/IPOs.
            }
            "WACC" | "COST_OF_CAPITAL" | "CAPM" => {
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
                    self.wacc_symbol = sym;
                }
                self.show_wacc = true;
                if self.wacc_snapshot.symbol.is_empty() && !self.wacc_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_wacc(&conn, &self.wacc_symbol)
                            {
                                self.wacc_snapshot = snap;
                            }
                        }
                    }
                }
                if !self.fmp_key.is_empty() && !self.wacc_symbol.is_empty() {
                    self.wacc_loading = true;
                    // Pass the latest 10Y yield we know about (in-memory);
                    // fall back to 4.5 % if the GY window hasn't been fetched.
                    let rf = self
                        .treasury_yields
                        .iter()
                        .find(|y| y.tenor == "10Y")
                        .map(|y| y.yield_pct)
                        .unwrap_or(4.5);
                    let _ = self.broker_tx.send(BrokerCmd::FetchWaccSnapshot {
                        symbol: self.wacc_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                        risk_free_pct: rf,
                    });
                }
            }
            // ── palette entries ──
            "WCR" | "CURRENCY" | "CURRENCIES" | "FX_RATES" => {
                self.show_wcr = true;
                if self.wcr_rates.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(rows)) =
                                typhoon_engine::core::research::get_currency_rates(&conn)
                            {
                                self.wcr_rates = rows;
                            }
                        }
                    }
                }
                self.wcr_loading = true;
                let _ = self.broker_tx.send(BrokerCmd::FetchCurrencyRates);
            }
            "BETA" | "ROLLING_BETA" => {
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
                    self.beta_symbol = sym;
                }
                self.show_beta = true;
                if self.beta_snapshot.symbol.is_empty() && !self.beta_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_beta(&conn, &self.beta_symbol)
                            {
                                self.beta_snapshot = snap;
                            }
                        }
                    }
                }
                if !self.fmp_key.is_empty() && !self.beta_symbol.is_empty() {
                    self.beta_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchBetaSnapshot {
                        symbol: self.beta_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "DDM" | "GORDON_GROWTH" | "DIVIDEND_DISCOUNT" => {
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
                    self.ddm_symbol = sym;
                }
                self.show_ddm = true;
                if self.ddm_snapshot.symbol.is_empty() && !self.ddm_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_ddm(&conn, &self.ddm_symbol)
                            {
                                self.ddm_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RV" | "RELATIVE_VALUATION" | "PEER_VALUATION" => {
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
                    self.rv_symbol = sym;
                }
                self.show_rv = true;
                if self.rv_snapshot.symbol.is_empty() && !self.rv_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(rv)) =
                                typhoon_engine::core::research::get_relative_valuation(
                                    &conn,
                                    &self.rv_symbol,
                                )
                            {
                                self.rv_snapshot = rv;
                            }
                        }
                    }
                }
            }
            "FIGI" | "OPENFIGI" | "IDENTIFIERS" => {
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
                    self.figi_symbol = sym;
                }
                self.show_figi = true;
                if self.figi_snapshot.identifiers.is_empty() && !self.figi_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_figi(&conn, &self.figi_symbol)
                            {
                                self.figi_snapshot = snap;
                            }
                        }
                    }
                }
                if !self.figi_symbol.is_empty() {
                    self.figi_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchFigiIdentifiers {
                        symbol: self.figi_symbol.to_uppercase(),
                    });
                }
            }
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
