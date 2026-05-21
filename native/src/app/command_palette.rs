use super::*;

impl TyphooNApp {
    pub(super) fn handle_command(&mut self, cmd: &str, ctx: &egui::Context) {
        let cmd_upper = cmd.trim().to_uppercase();
        self.log
            .push_back(LogEntry::info(format!("CMD: {}", cmd_upper)));
        match cmd_upper.as_str() {
            "QUIT" => {
                self.save_session();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            "MTF" | "MTF_GRID" => {
                self.mtf_enabled = !self.mtf_enabled;
                // When enabling MTF grid, load any charts with empty bars
                if self.mtf_enabled {
                    if let Some(ref cache) = self.cache.clone() {
                        let mut retry_first_chart = false;
                        for chart in &mut self.charts {
                            if chart.bars.is_empty() {
                                {
                                    let mut gpu = self.gpu_indicators.take();
                                    if !chart.try_load(
                                        Arc::as_ref(cache),
                                        &mut self.log,
                                        gpu.as_mut(),
                                    ) {
                                        retry_first_chart = true;
                                    }
                                    self.gpu_indicators = gpu;
                                }
                            }
                        }
                        if retry_first_chart {
                            self.queue_chart_reload(0);
                        }
                    }
                }
                self.log
                    .push_back(LogEntry::info(format!("MTF grid: {}", self.mtf_enabled)));
            }
            "MTF_2X2" => {
                self.setup_mtf_grid(2, 4);
            }
            "MTF_3X3" => {
                self.setup_mtf_grid(3, 9);
            }
            "MTF_4X4" => {
                self.setup_mtf_grid(4, 16);
            }
            "MTF_4X3" => {
                self.setup_mtf_grid(4, 12);
            }
            "RELOAD" => {
                if let Some(ref cache) = self.cache.clone() {
                    let mut retry_first_chart = false;
                    for chart in &mut self.charts {
                        {
                            let mut gpu = self.gpu_indicators.take();
                            if !chart.try_load(Arc::as_ref(cache), &mut self.log, gpu.as_mut()) {
                                retry_first_chart = true;
                            }
                            self.gpu_indicators = gpu;
                        }
                    }
                    if retry_first_chart {
                        self.queue_chart_reload(0);
                    }
                }
            }
            "CONNECT" => self.show_connect = true,
            "SETTINGS" => self.show_settings = true,
            "INDICATORS" => self.show_indicators_panel = !self.show_indicators_panel,
            "DARWIN" => self.show_darwin_accounts = true,
            "PORTFOLIO" => self.show_darwin_portfolio = true,
            "OVERLAP" => self.show_symbol_overlap = true,
            "BACKTEST" => self.show_backtest = true,
            "SCREENER" => self.show_screener = true,
            "SYMBOLS" | "SYM" => self.show_symbols = true,
            "OPTIMIZER" => self.show_optimizer = true,
            "RISK_CALC" => self.show_risk_calc = true,
            "COMPOUND" | "COMPOUND_INTEREST" => self.show_compound_calc = true,
            "VAR" => self.show_var_mult = true,
            "MARGIN" => self.show_margin_monitor = true,
            "FRED" => {
                if self.fred_key.is_empty() {
                    self.log
                        .push_back(LogEntry::warn("Set FRED API Key in Settings first"));
                } else {
                    self.show_fred = true;
                    let key = self.fred_key.clone();
                    let _ = self.broker_tx.send(BrokerCmd::FredFetch { api_key: key });
                }
            }
            "NEWS" => self.show_news = true,
            // ── Godel parity research windows (ADR-107) ──
            "DES" | "DESCRIPTION" => {
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
                    self.desc_symbol = sym.clone();
                }
                self.show_company_desc = true;
                if !self.finnhub_key.is_empty() && !self.desc_symbol.is_empty() {
                    self.desc_loading = true;
                    let s = self.desc_symbol.to_uppercase();
                    let k = self.finnhub_key.clone();
                    let _ = self.broker_tx.send(BrokerCmd::FetchCompanyProfile {
                        symbol: s.clone(),
                        finnhub_key: k.clone(),
                    });
                    let _ = self.broker_tx.send(BrokerCmd::FetchStockPeers {
                        symbol: s.clone(),
                        finnhub_key: k.clone(),
                    });
                    let _ = self.broker_tx.send(BrokerCmd::FetchEarningsHistory {
                        symbol: s.clone(),
                        finnhub_key: k.clone(),
                    });
                    let _ = self.broker_tx.send(BrokerCmd::FetchPressReleases {
                        symbol: s,
                        finnhub_key: k,
                    });
                }
            }
            "IPO" => {
                self.show_ipo_calendar = true;
                if !self.finnhub_key.is_empty() {
                    self.ipo_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchIpoCalendar {
                        finnhub_key: self.finnhub_key.clone(),
                        days_ahead: 30,
                        days_back: 30,
                    });
                }
            }
            "ERN" | "EARNINGS_HISTORY" => {
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
                    self.earnings_history_symbol = sym;
                }
                self.show_earnings_history = true;
                if !self.finnhub_key.is_empty() && !self.earnings_history_symbol.is_empty() {
                    self.earnings_history_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchEarningsHistory {
                        symbol: self.earnings_history_symbol.to_uppercase(),
                        finnhub_key: self.finnhub_key.clone(),
                    });
                }
            }
            "PEERS" => {
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
                    self.peers_symbol = sym;
                }
                self.show_peers = true;
                if !self.finnhub_key.is_empty() && !self.peers_symbol.is_empty() {
                    self.peers_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchStockPeers {
                        symbol: self.peers_symbol.to_uppercase(),
                        finnhub_key: self.finnhub_key.clone(),
                    });
                }
            }
            "PRESS" => {
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
                    self.press_symbol = sym;
                }
                self.show_press_releases = true;
                if !self.finnhub_key.is_empty() && !self.press_symbol.is_empty() {
                    self.press_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchPressReleases {
                        symbol: self.press_symbol.to_uppercase(),
                        finnhub_key: self.finnhub_key.clone(),
                    });
                }
            }
            "SENTIMENT" | "SOCIAL" => {
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
                    self.sentiment_symbol = sym;
                }
                self.show_sentiment = true;
                if !self.finnhub_key.is_empty() && !self.sentiment_symbol.is_empty() {
                    self.sentiment_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchSocialSentiment {
                        symbol: self.sentiment_symbol.to_uppercase(),
                        finnhub_key: self.finnhub_key.clone(),
                    });
                }
            }
            "TRANSCRIPTS" | "CALLS" => {
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
                    self.transcripts_symbol = sym;
                }
                self.show_transcripts = true;
                if !self.fmp_key.is_empty() && !self.transcripts_symbol.is_empty() {
                    self.transcripts_loading_list = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchTranscriptList {
                        symbol: self.transcripts_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "GLCO" | "COMMODITIES" => {
                self.show_commodities = true;
                self.commodities_loading = true;
                let _ = self.broker_tx.send(BrokerCmd::FetchCommoditiesQuotes);
            }
            "RESEARCH_SCRAPE" | "RSCRAPE" => {
                let _ = self.broker_tx.send(BrokerCmd::ResearchScrape {
                    use_mt5: true,
                    use_alpaca: true,
                    use_tastytrade: true,
                    finnhub_key: self.finnhub_key.clone(),
                    fmp_key: self.fmp_key.clone(),
                });
                self.log.push_back(LogEntry::info(
                    "Research scrape started across MT5/Alpaca/TastyTrade universe",
                ));
            }
            "TAS" | "TIME_SALES" => {
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
                    self.tas_symbol = sym;
                }
                self.tas_rows.clear();
                self.tas_paused = false;
                self.show_tas = true;
            }
            // ── ADR-109 Godel Parity Round 2 ──
            "DVD" | "DIV_HISTORY" | "DIVIDEND_HISTORY" => {
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
                    self.dividend_history_symbol = sym.clone();
                }
                self.show_dividend_history = true;
                if !self.fmp_key.is_empty() && !self.dividend_history_symbol.is_empty() {
                    self.dividend_history_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchDividendHistory {
                        symbol: self.dividend_history_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "EEB" | "ESTIMATES" | "FORWARD_EARNINGS" => {
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
                    self.earnings_estimates_symbol = sym.clone();
                }
                self.show_earnings_estimates = true;
                if !self.fmp_key.is_empty() && !self.earnings_estimates_symbol.is_empty() {
                    self.earnings_estimates_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchEarningsEstimates {
                        symbol: self.earnings_estimates_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "UPDG" | "UPGRADES" | "DOWNGRADES" | "RATING_CHANGES" => {
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
                    self.rating_changes_symbol = sym.clone();
                }
                self.show_rating_changes = true;
                if !self.fmp_key.is_empty() && !self.rating_changes_symbol.is_empty() {
                    self.rating_changes_loading = true;
                    let _ = self.broker_tx.send(BrokerCmd::FetchRatingChanges {
                        symbol: self.rating_changes_symbol.to_uppercase(),
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            "GY" | "TREASURY" | "YIELD_CURVE" | "YIELDS" => {
                self.show_treasury_curve = true;
                self.treasury_yields_loading = true;
                let _ = self.broker_tx.send(BrokerCmd::FetchTreasuryYields);
            }
            // ── ADR-110 Godel Parity Round 3 ──
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
            // ── ADR-111 Round 4 palette entries ──
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
                if !self.fmp_key.is_empty() && !self.splits_symbol.is_empty() {
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
            // ── ADR-112 Round 5 palette entries ──
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
            // ── ADR-113 Round 6 palette entries ──
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
            // ── ADR-114 Round 7 palette entries ──
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
            // ── ADR-115 Round 8 palette entries ──
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
            // ── ADR-117 Godel Parity Round 10 ──
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
            // ── ADR-118 Godel Parity Round 11 ──
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
            // ── ADR-120 Godel Parity Round 13 ──
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
            // ── ADR-124 Round 17 ──
            "SIZEF" | "SIZE_FACTOR" | "SIZE_RANK" => {
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
                    self.sizef_symbol = sym;
                }
                self.show_sizef = true;
                if self.sizef_snapshot.symbol.is_empty() && !self.sizef_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_sizef(&conn, &self.sizef_symbol)
                            {
                                self.sizef_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MOMF" | "MOMENTUM_RANK" | "MOM_RANK" => {
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
                    self.momf_symbol = sym;
                }
                self.show_momf = true;
                if self.momf_snapshot.symbol.is_empty() && !self.momf_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_momf(&conn, &self.momf_symbol)
                            {
                                self.momf_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PEADRANK" | "PEAD_RANK" => {
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
                    self.peadrank_symbol = sym;
                }
                self.show_peadrank = true;
                if self.peadrank_snapshot.symbol.is_empty() && !self.peadrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_peadrank(
                                &conn,
                                &self.peadrank_symbol,
                            ) {
                                self.peadrank_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "FQM" | "FUND_QUALITY" | "QUALITY_METER" => {
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
                    self.fqm_symbol = sym;
                }
                self.show_fqm = true;
                if self.fqm_snapshot.symbol.is_empty() && !self.fqm_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_fqm(&conn, &self.fqm_symbol)
                            {
                                self.fqm_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "REVRANK" | "REV_RANK" | "REVENUE_GROWTH_RANK" => {
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
                    self.revrank_symbol = sym;
                }
                self.show_revrank = true;
                if self.revrank_snapshot.symbol.is_empty() && !self.revrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_revrank(
                                &conn,
                                &self.revrank_symbol,
                            ) {
                                self.revrank_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-125 Round 18 ──
            "LEVRANK" | "LEV_RANK" | "LEVERAGE_RANK" => {
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
                    self.levrank_symbol = sym;
                }
                self.show_levrank = true;
                if self.levrank_snapshot.symbol.is_empty() && !self.levrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_levrank(
                                &conn,
                                &self.levrank_symbol,
                            ) {
                                self.levrank_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "OPERANK" | "OPER_RANK" | "OP_QUALITY_RANK" => {
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
                    self.operank_symbol = sym;
                }
                self.show_operank = true;
                if self.operank_snapshot.symbol.is_empty() && !self.operank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_operank(
                                &conn,
                                &self.operank_symbol,
                            ) {
                                self.operank_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "FQMRANK" | "FQM_RANK" => {
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
                    self.fqmrank_symbol = sym;
                }
                self.show_fqmrank = true;
                if self.fqmrank_snapshot.symbol.is_empty() && !self.fqmrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_fqmrank(
                                &conn,
                                &self.fqmrank_symbol,
                            ) {
                                self.fqmrank_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "LIQRANK" | "LIQ_RANK" | "LIQUIDITY_RANK" => {
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
                    self.liqrank_symbol = sym;
                }
                self.show_liqrank = true;
                if self.liqrank_snapshot.symbol.is_empty() && !self.liqrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_liqrank(
                                &conn,
                                &self.liqrank_symbol,
                            ) {
                                self.liqrank_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "TLRANK" | "TL_RANK" | "LIQ30_RANK" | "ADV30_RANK" => {
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
                    self.tlrank_symbol = sym;
                }
                self.show_tlrank = true;
                if self.tlrank_snapshot.symbol.is_empty() && !self.tlrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_tlrank(
                                &conn,
                                &self.tlrank_symbol,
                            ) {
                                self.tlrank_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SURPSTK" | "EPS_STREAK" | "SURPRISE_STREAK" => {
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
                    self.surpstk_symbol = sym;
                }
                self.show_surpstk = true;
                if self.surpstk_snapshot.symbol.is_empty() && !self.surpstk_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_surpstk(
                                &conn,
                                &self.surpstk_symbol,
                            ) {
                                self.surpstk_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DVDRANK" | "DIVG_RANK" | "DIVIDEND_RANK" => {
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
                    self.dvdrank_symbol = sym;
                }
                self.show_dvdrank = true;
                if self.dvdrank_snapshot.symbol.is_empty() && !self.dvdrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_dvdrank(
                                &conn,
                                &self.dvdrank_symbol,
                            ) {
                                self.dvdrank_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "EARMRANK" | "EARM_RANK" | "EARNINGS_MOMENTUM_RANK" => {
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
                    self.earmrank_symbol = sym;
                }
                self.show_earmrank = true;
                if self.earmrank_snapshot.symbol.is_empty() && !self.earmrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_earmrank(
                                &conn,
                                &self.earmrank_symbol,
                            ) {
                                self.earmrank_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "UPDGRANK" | "UPDG_RANK" | "UPGRADE_RANK" => {
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
                    self.updgrank_symbol = sym;
                }
                self.show_updgrank = true;
                if self.updgrank_snapshot.symbol.is_empty() && !self.updgrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_updgrank(
                                &conn,
                                &self.updgrank_symbol,
                            ) {
                                self.updgrank_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "GY_STAT" | "GAP_YEARLY" | "GAPS" => {
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
                    self.gy_symbol = sym;
                }
                self.show_gy = true;
                if self.gy_snapshot.symbol.is_empty() && !self.gy_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_gy(&conn, &self.gy_symbol)
                            {
                                self.gy_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DES_STREAK" | "DAILY_STREAK" | "EVENT_STREAK" => {
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
                    self.des_symbol = sym;
                }
                self.show_des = true;
                if self.des_snapshot.symbol.is_empty() && !self.des_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_des(&conn, &self.des_symbol)
                            {
                                self.des_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DVDYIELDRANK" | "DVDY_RANK" | "DIVIDEND_YIELD_RANK" => {
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
                    self.dvdyieldrank_symbol = sym;
                }
                self.show_dvdyieldrank = true;
                if self.dvdyieldrank_snapshot.symbol.is_empty()
                    && !self.dvdyieldrank_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_dvdyieldrank(
                                &conn,
                                &self.dvdyieldrank_symbol,
                            ) {
                                self.dvdyieldrank_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SHRANK" | "SHORT_RANK" | "SHORT_INT_RANK" => {
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
                    self.shrank_symbol = sym;
                }
                self.show_shrank = true;
                if self.shrank_snapshot.symbol.is_empty() && !self.shrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_shrank(
                                &conn,
                                &self.shrank_symbol,
                            ) {
                                self.shrank_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SHORTRANK_DELTA" | "SHORT_DELTA_RANK" | "SHORTTREND_RANK" | "SHORTRANKD" => {
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
                    self.shortrank_delta_symbol = sym;
                }
                self.show_shortrank_delta = true;
                if self.shortrank_delta_snapshot.symbol.is_empty()
                    && !self.shortrank_delta_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_shortrank_delta(
                                    &conn,
                                    &self.shortrank_delta_symbol,
                                )
                            {
                                self.shortrank_delta_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "INSIDERCONC" | "INSIDER_CONC" | "INSIDER_OWNERSHIP_CONC" | "INSIDER_HOLD_RANK" => {
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
                    self.insiderconc_symbol = sym;
                }
                self.show_insiderconc = true;
                if self.insiderconc_snapshot.symbol.is_empty()
                    && !self.insiderconc_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_insiderconc(
                                &conn,
                                &self.insiderconc_symbol,
                            ) {
                                self.insiderconc_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ATRANN" | "ATR_ANN" | "ANNUALIZED_ATR" => {
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
                    self.atrann_symbol = sym;
                }
                self.show_atrann = true;
                if self.atrann_snapshot.symbol.is_empty() && !self.atrann_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_atrann(
                                &conn,
                                &self.atrann_symbol,
                            ) {
                                self.atrann_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DDHIST" | "DD_HIST" | "DRAWDOWN_HIST" => {
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
                    self.ddhist_symbol = sym;
                }
                self.show_ddhist = true;
                if self.ddhist_snapshot.symbol.is_empty() && !self.ddhist_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ddhist(
                                &conn,
                                &self.ddhist_symbol,
                            ) {
                                self.ddhist_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PRICEPERF" | "PRICE_PERF" | "MULTI_RETURN" => {
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
                    self.priceperf_symbol = sym;
                }
                self.show_priceperf = true;
                if self.priceperf_snapshot.symbol.is_empty() && !self.priceperf_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_priceperf(
                                &conn,
                                &self.priceperf_symbol,
                            ) {
                                self.priceperf_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MOMRANK_MULTI" | "MOMRANKM" | "SECTOR_MOM_RANK" | "PRICEPERF_RANK" => {
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
                    self.momrank_multi_symbol = sym;
                }
                self.show_momrank_multi = true;
                if self.momrank_multi_snapshot.symbol.is_empty()
                    && !self.momrank_multi_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_momrank_multi(
                                    &conn,
                                    &self.momrank_multi_symbol,
                                )
                            {
                                self.momrank_multi_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "BETARANK" | "BETA_RANK" | "BRK" => {
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
                    self.betarank_symbol = sym;
                }
                self.show_betarank = true;
                if self.betarank_snapshot.symbol.is_empty() && !self.betarank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_betarank(
                                &conn,
                                &self.betarank_symbol,
                            ) {
                                self.betarank_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PEGRANK" | "PEG_RANK" | "PEG_SCORE" => {
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
                    self.pegrank_symbol = sym;
                }
                self.show_pegrank = true;
                if self.pegrank_snapshot.symbol.is_empty() && !self.pegrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_pegrank(
                                &conn,
                                &self.pegrank_symbol,
                            ) {
                                self.pegrank_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "FHIGHLOW" | "FHL" | "52_WEEK" => {
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
                    self.fhighlow_symbol = sym;
                }
                self.show_fhighlow = true;
                if self.fhighlow_snapshot.symbol.is_empty() && !self.fhighlow_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_fhighlow(
                                &conn,
                                &self.fhighlow_symbol,
                            ) {
                                self.fhighlow_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RVCONE" | "RV_CONE" | "REAL_VOL_CONE" => {
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
                    self.rvcone_symbol = sym;
                }
                self.show_rvcone = true;
                if self.rvcone_snapshot.symbol.is_empty() && !self.rvcone_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_rvcone(
                                &conn,
                                &self.rvcone_symbol,
                            ) {
                                self.rvcone_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CALPB" | "CAL_PB" | "CAL_BREAK" => {
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
                    self.calpb_symbol = sym;
                }
                self.show_calpb = true;
                if self.calpb_snapshot.symbol.is_empty() && !self.calpb_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_calpb(&conn, &self.calpb_symbol)
                            {
                                self.calpb_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CORRSTK" | "CORR_STK" | "BENCH_CORR" | "SPY_CORR" | "SECTOR_CORR" => {
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
                    self.corrstk_symbol = sym;
                }
                self.show_corrstk = true;
                if self.corrstk_snapshot.symbol.is_empty() && !self.corrstk_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_corrstk(
                                &conn,
                                &self.corrstk_symbol,
                            ) {
                                self.corrstk_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CORRRANK" | "CORR_RANK" | "BENCH_RANK" | "CORR_LINK_RANK" => {
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
                    self.corrrank_symbol = sym;
                }
                self.show_corrrank = true;
                if self.corrrank_snapshot.symbol.is_empty() && !self.corrrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_corrrank(
                                &conn,
                                &self.corrrank_symbol,
                            ) {
                                self.corrrank_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "OPERANK_DELTA" | "OPERANKD" | "OP_MARGIN_DELTA_RANK" | "OPERATING_MARGIN_DELTA" => {
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
                    self.operank_delta_symbol = sym;
                }
                self.show_operank_delta = true;
                if self.operank_delta_snapshot.symbol.is_empty()
                    && !self.operank_delta_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_operank_delta(
                                    &conn,
                                    &self.operank_delta_symbol,
                                )
                            {
                                self.operank_delta_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DIVACC" | "DIV_ACCEL" | "DIVIDEND_ACCELERATION" => {
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
                    self.divacc_symbol = sym;
                }
                self.show_divacc = true;
                if self.divacc_snapshot.symbol.is_empty() && !self.divacc_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_divacc(
                                &conn,
                                &self.divacc_symbol,
                            ) {
                                self.divacc_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "EPSACC" | "EPS_ACCEL" | "EARNINGS_ACCELERATION" => {
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
                    self.epsacc_symbol = sym;
                }
                self.show_epsacc = true;
                if self.epsacc_snapshot.symbol.is_empty() && !self.epsacc_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_epsacc(
                                &conn,
                                &self.epsacc_symbol,
                            ) {
                                self.epsacc_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VRP" | "VOL_RISK_PREMIUM" | "IV_RV_RATIO" | "REALIZED_VS_IMPLIED_VOL_RATIO" => {
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
                    self.vrp_symbol = sym;
                }
                self.show_vrp = true;
                if self.vrp_snapshot.symbol.is_empty() && !self.vrp_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_vrp(&conn, &self.vrp_symbol)
                            {
                                self.vrp_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-129 Round 22 palette entries ──
            "RETSKEW" | "RET_SKEW" | "SKEWNESS" => {
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
                    self.retskew_symbol = sym;
                }
                self.show_retskew = true;
                if self.retskew_snapshot.symbol.is_empty() && !self.retskew_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_retskew(
                                &conn,
                                &self.retskew_symbol,
                            ) {
                                self.retskew_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RETKURT" | "RET_KURT" | "KURTOSIS" => {
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
                    self.retkurt_symbol = sym;
                }
                self.show_retkurt = true;
                if self.retkurt_snapshot.symbol.is_empty() && !self.retkurt_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_retkurt(
                                &conn,
                                &self.retkurt_symbol,
                            ) {
                                self.retkurt_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "TAILR" | "TAIL_RATIO" | "TAILRATIO" => {
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
                    self.tailr_symbol = sym;
                }
                self.show_tailr = true;
                if self.tailr_snapshot.symbol.is_empty() && !self.tailr_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_tailr(&conn, &self.tailr_symbol)
                            {
                                self.tailr_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RUNLEN" | "RUN_LEN" | "RUN_LENGTH" => {
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
                    self.runlen_symbol = sym;
                }
                self.show_runlen = true;
                if self.runlen_snapshot.symbol.is_empty() && !self.runlen_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_runlen(
                                &conn,
                                &self.runlen_symbol,
                            ) {
                                self.runlen_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DAYRANGE" | "DAY_RANGE" | "RANGESTAT" => {
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
                    self.dayrange_symbol = sym;
                }
                self.show_dayrange = true;
                if self.dayrange_snapshot.symbol.is_empty() && !self.dayrange_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_dayrange(
                                &conn,
                                &self.dayrange_symbol,
                            ) {
                                self.dayrange_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-131 Round 23 ──
            "AUTOCOR" | "AUTO_COR" | "ACF" => {
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
                    self.autocor_symbol = sym;
                }
                self.show_autocor = true;
                if self.autocor_snapshot.symbol.is_empty() && !self.autocor_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_autocor(
                                &conn,
                                &self.autocor_symbol,
                            ) {
                                self.autocor_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HURST" | "HURST_EXPONENT" | "RESCALED_RANGE" => {
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
                    self.hurst_symbol = sym;
                }
                self.show_hurst = true;
                if self.hurst_snapshot.symbol.is_empty() && !self.hurst_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_hurst(&conn, &self.hurst_symbol)
                            {
                                self.hurst_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HITRATE" | "HIT_RATE" | "WIN_RATE" | "WINRATE" => {
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
                    self.hitrate_symbol = sym;
                }
                self.show_hitrate = true;
                if self.hitrate_snapshot.symbol.is_empty() && !self.hitrate_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_hitrate(
                                &conn,
                                &self.hitrate_symbol,
                            ) {
                                self.hitrate_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "GLASYM" | "GL_ASYM" | "GAIN_LOSS_ASYM" | "GAINLOSSASYM" => {
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
                    self.glasym_symbol = sym;
                }
                self.show_glasym = true;
                if self.glasym_snapshot.symbol.is_empty() && !self.glasym_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_glasym(
                                &conn,
                                &self.glasym_symbol,
                            ) {
                                self.glasym_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VOLRATIO" | "VOL_RATIO" | "VOLUMERATIO" | "VOLUME_RATIO" => {
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
                    self.volratio_symbol = sym;
                }
                self.show_volratio = true;
                if self.volratio_snapshot.symbol.is_empty() && !self.volratio_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_volratio(
                                &conn,
                                &self.volratio_symbol,
                            ) {
                                self.volratio_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-132 Round 24 palette ──
            "DRAWUP" | "DRAW_UP" | "RALLYHIST" | "RALLY_HISTORY" => {
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
                    self.drawup_symbol = sym;
                }
                self.show_drawup = true;
                if self.drawup_snapshot.symbol.is_empty() && !self.drawup_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_drawup(
                                &conn,
                                &self.drawup_symbol,
                            ) {
                                self.drawup_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "GAPSTATS" | "GAP_STATS" | "GAP" | "OVERNIGHT_GAP" => {
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
                    self.gapstats_symbol = sym;
                }
                self.show_gapstats = true;
                if self.gapstats_snapshot.symbol.is_empty() && !self.gapstats_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_gapstats(
                                &conn,
                                &self.gapstats_symbol,
                            ) {
                                self.gapstats_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VOLCLUSTER" | "VOL_CLUSTER" | "ARCH" | "VOLATILITYCLUSTER" => {
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
                    self.volcluster_symbol = sym;
                }
                self.show_volcluster = true;
                if self.volcluster_snapshot.symbol.is_empty() && !self.volcluster_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_volcluster(
                                &conn,
                                &self.volcluster_symbol,
                            ) {
                                self.volcluster_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CLOSEPLC" | "CLOSE_PLC" | "CLOSEPLACEMENT" | "CLOSE_PLACEMENT" => {
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
                    self.closeplc_symbol = sym;
                }
                self.show_closeplc = true;
                if self.closeplc_snapshot.symbol.is_empty() && !self.closeplc_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_closeplc(
                                &conn,
                                &self.closeplc_symbol,
                            ) {
                                self.closeplc_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MRHL" | "HALF_LIFE" | "HALFLIFE" | "AR1" | "MEAN_REVERT_HL" => {
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
                    self.mrhl_symbol = sym;
                }
                self.show_mrhl = true;
                if self.mrhl_snapshot.symbol.is_empty() && !self.mrhl_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_mrhl(&conn, &self.mrhl_symbol)
                            {
                                self.mrhl_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-133 Round 25 palette ──
            "DOWNVOL" | "DOWN_VOL" | "SEMIDEV" | "SORTINO" => {
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
                    self.downvol_symbol = sym;
                }
                self.show_downvol = true;
                if self.downvol_snapshot.symbol.is_empty() && !self.downvol_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_downvol(
                                &conn,
                                &self.downvol_symbol,
                            ) {
                                self.downvol_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SHARPR" | "SHARPE" | "SHARPE_RATIO" | "SHARPERATIO" => {
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
                    self.sharpr_symbol = sym;
                }
                self.show_sharpr = true;
                if self.sharpr_snapshot.symbol.is_empty() && !self.sharpr_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_sharpr(
                                &conn,
                                &self.sharpr_symbol,
                            ) {
                                self.sharpr_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "EFFRATIO" | "EFF_RATIO" | "KAUFMAN" | "KAUFMAN_ER" | "KER" => {
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
                    self.effratio_symbol = sym;
                }
                self.show_effratio = true;
                if self.effratio_snapshot.symbol.is_empty() && !self.effratio_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_effratio(
                                &conn,
                                &self.effratio_symbol,
                            ) {
                                self.effratio_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "WICKBIAS" | "WICK_BIAS" | "WICKS" => {
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
                    self.wickbias_symbol = sym;
                }
                self.show_wickbias = true;
                if self.wickbias_snapshot.symbol.is_empty() && !self.wickbias_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_wickbias(
                                &conn,
                                &self.wickbias_symbol,
                            ) {
                                self.wickbias_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VOLOFVOL" | "VOL_OF_VOL" | "VOV" | "VVOL" => {
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
                    self.volofvol_symbol = sym;
                }
                self.show_volofvol = true;
                if self.volofvol_snapshot.symbol.is_empty() && !self.volofvol_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_volofvol(
                                &conn,
                                &self.volofvol_symbol,
                            ) {
                                self.volofvol_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-134 Round 26 palette ──
            "CALMAR" | "CALMAR_RATIO" | "CALMARRATIO" => {
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
                    self.calmar_symbol = sym;
                }
                self.show_calmar = true;
                if self.calmar_snapshot.symbol.is_empty() && !self.calmar_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_calmar(
                                &conn,
                                &self.calmar_symbol,
                            ) {
                                self.calmar_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ULCER" | "ULCER_INDEX" | "ULCERINDEX" | "MARTIN" | "UPI" => {
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
                    self.ulcer_symbol = sym;
                }
                self.show_ulcer = true;
                if self.ulcer_snapshot.symbol.is_empty() && !self.ulcer_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_ulcer(&conn, &self.ulcer_symbol)
                            {
                                self.ulcer_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VARRATIO" | "VAR_RATIO" | "VARIANCE_RATIO" | "LO_MACKINLAY" => {
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
                    self.varratio_symbol = sym;
                }
                self.show_varratio = true;
                if self.varratio_snapshot.symbol.is_empty() && !self.varratio_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_varratio(
                                &conn,
                                &self.varratio_symbol,
                            ) {
                                self.varratio_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "AMIHUD" | "AMIHUD_ILLIQ" | "ILLIQ" | "ILLIQUIDITY" => {
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
                    self.amihud_symbol = sym;
                }
                self.show_amihud = true;
                if self.amihud_snapshot.symbol.is_empty() && !self.amihud_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_amihud(
                                &conn,
                                &self.amihud_symbol,
                            ) {
                                self.amihud_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "JBNORM" | "JB" | "JARQUE_BERA" | "NORMALITY" => {
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
                    self.jbnorm_symbol = sym;
                }
                self.show_jbnorm = true;
                if self.jbnorm_snapshot.symbol.is_empty() && !self.jbnorm_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_jbnorm(
                                &conn,
                                &self.jbnorm_symbol,
                            ) {
                                self.jbnorm_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-135 Round 27 palette ──
            "OMEGA" | "OMEGA_RATIO" | "OMEGARATIO" => {
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
                    self.omega_symbol = sym;
                }
                self.show_omega = true;
                if self.omega_snapshot.symbol.is_empty() && !self.omega_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_omega(&conn, &self.omega_symbol)
                            {
                                self.omega_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DFA" | "DETRENDED_FLUCT" | "DETRENDED_FLUCTUATION" | "DFAALPHA" => {
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
                    self.dfa_symbol = sym;
                }
                self.show_dfa = true;
                if self.dfa_snapshot.symbol.is_empty() && !self.dfa_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_dfa(&conn, &self.dfa_symbol)
                            {
                                self.dfa_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "BURKE" | "BURKE_RATIO" | "BURKERATIO" => {
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
                    self.burke_symbol = sym;
                }
                self.show_burke = true;
                if self.burke_snapshot.symbol.is_empty() && !self.burke_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_burke(&conn, &self.burke_symbol)
                            {
                                self.burke_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MONTHSEAS" | "MONTHLY_SEASONALITY" | "MONTHLYSEASONALITY" | "SEAS" | "MONTH_SEAS" => {
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
                    self.monthseas_symbol = sym;
                }
                self.show_monthseas = true;
                if self.monthseas_snapshot.symbol.is_empty() && !self.monthseas_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_monthseas(
                                &conn,
                                &self.monthseas_symbol,
                            ) {
                                self.monthseas_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ROLLSPRD" | "ROLL_SPREAD" | "ROLLSPREAD" | "ROLL" | "EFFECTIVE_SPREAD" => {
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
                    self.rollsprd_symbol = sym;
                }
                self.show_rollsprd = true;
                if self.rollsprd_snapshot.symbol.is_empty() && !self.rollsprd_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_rollsprd(
                                &conn,
                                &self.rollsprd_symbol,
                            ) {
                                self.rollsprd_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-136 Round 28 palette ──
            "PARKINSON" | "PARKINSON_VOL" | "PARKVOL" | "HL_VOL" | "RANGE_VOL" => {
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
                    self.parkinson_symbol = sym;
                }
                self.show_parkinson = true;
                if self.parkinson_snapshot.symbol.is_empty() && !self.parkinson_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_parkinson(
                                &conn,
                                &self.parkinson_symbol,
                            ) {
                                self.parkinson_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "GKVOL" | "GARMAN_KLASS" | "GARMANKLASS" | "GK_VOL" | "GARMAN_KLASS_VOL" => {
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
                    self.gkvol_symbol = sym;
                }
                self.show_gkvol = true;
                if self.gkvol_snapshot.symbol.is_empty() && !self.gkvol_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_gkvol(&conn, &self.gkvol_symbol)
                            {
                                self.gkvol_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RSVOL" | "ROGERS_SATCHELL" | "ROGERSSATCHELL" | "RS_VOL" | "DRIFT_FREE_VOL" => {
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
                    self.rsvol_symbol = sym;
                }
                self.show_rsvol = true;
                if self.rsvol_snapshot.symbol.is_empty() && !self.rsvol_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_rsvol(&conn, &self.rsvol_symbol)
                            {
                                self.rsvol_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CVAR" | "EXPECTED_SHORTFALL" | "ES" | "CONDITIONAL_VAR" | "ES5" | "ES_5"
            | "TAIL_EXPECTED" => {
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
                    self.cvar_symbol = sym;
                }
                self.show_cvar = true;
                if self.cvar_snapshot.symbol.is_empty() && !self.cvar_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cvar(&conn, &self.cvar_symbol)
                            {
                                self.cvar_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DOWEFFECT" | "DOW_EFFECT" | "DOW" | "WEEKDAY_EFFECT" | "DAY_OF_WEEK" | "DAYOFWEEK" => {
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
                    self.doweffect_symbol = sym;
                }
                self.show_doweffect = true;
                if self.doweffect_snapshot.symbol.is_empty() && !self.doweffect_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_doweffect(
                                &conn,
                                &self.doweffect_symbol,
                            ) {
                                self.doweffect_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-137 Round 29 palette ──
            "STERLING" | "STERLING_RATIO" | "STERLINGRATIO" => {
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
                    self.sterling_symbol = sym;
                }
                self.show_sterling = true;
                if self.sterling_snapshot.symbol.is_empty() && !self.sterling_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_sterling(
                                &conn,
                                &self.sterling_symbol,
                            ) {
                                self.sterling_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "KELLYF" | "KELLY" | "KELLY_FRACTION" | "KELLY_CRITERION" | "OPTIMAL_F" => {
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
                    self.kellyf_symbol = sym;
                }
                self.show_kellyf = true;
                if self.kellyf_snapshot.symbol.is_empty() && !self.kellyf_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_kellyf(
                                &conn,
                                &self.kellyf_symbol,
                            ) {
                                self.kellyf_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "LJUNGB" | "LJUNG_BOX" | "LJUNGBOX" | "PORTMANTEAU" | "QSTAT" | "Q_STAT" => {
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
                    self.ljungb_symbol = sym;
                }
                self.show_ljungb = true;
                if self.ljungb_snapshot.symbol.is_empty() && !self.ljungb_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ljungb(
                                &conn,
                                &self.ljungb_symbol,
                            ) {
                                self.ljungb_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RUNSTEST" | "RUNS_TEST" | "WALD_WOLFOWITZ" | "WW_RUNS" | "SIGN_RUNS" => {
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
                    self.runstest_symbol = sym;
                }
                self.show_runstest = true;
                if self.runstest_snapshot.symbol.is_empty() && !self.runstest_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_runstest(
                                &conn,
                                &self.runstest_symbol,
                            ) {
                                self.runstest_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ZERORET" | "ZERO_RETURN" | "LOT" | "LESMOND" | "ZERO_DAYS" | "ZERODAYS" => {
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
                    self.zeroret_symbol = sym;
                }
                self.show_zeroret = true;
                if self.zeroret_snapshot.symbol.is_empty() && !self.zeroret_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_zeroret(
                                &conn,
                                &self.zeroret_symbol,
                            ) {
                                self.zeroret_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-138 Round 30 palette ──
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
            // ── ADR-139 Round 31 palette ──
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
            // ── ADR-140 Round 32 palette ──
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
            // ── ADR-141 Round 33 palette ──
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
            // ── ADR-142 Round 34 palette ──
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
            // ── ADR-143 Round 35 palette aliases ──
            "ROBVOL" | "ROBUST_VOL" | "ROBUSTVOL" => {
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
                    self.robvol_symbol = sym;
                }
                self.show_robvol = true;
                if self.robvol_snapshot.symbol.is_empty() && !self.robvol_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_robvol(
                                &conn,
                                &self.robvol_symbol,
                            ) {
                                self.robvol_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RENYIENT" | "RENYI_ENTROPY" | "RENYIENTROPY" => {
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
                    self.renyient_symbol = sym;
                }
                self.show_renyient = true;
                if self.renyient_snapshot.symbol.is_empty() && !self.renyient_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_renyient(
                                &conn,
                                &self.renyient_symbol,
                            ) {
                                self.renyient_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RETQUANT" | "RETURN_QUANTILES" | "RETURNQUANTILES" => {
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
                    self.retquant_symbol = sym;
                }
                self.show_retquant = true;
                if self.retquant_snapshot.symbol.is_empty() && !self.retquant_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_retquant(
                                &conn,
                                &self.retquant_symbol,
                            ) {
                                self.retquant_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MSENT" | "MULTISCALE_ENTROPY" | "MULTISCALEENTROPY" => {
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
                    self.msent_symbol = sym;
                }
                self.show_msent = true;
                if self.msent_snapshot.symbol.is_empty() && !self.msent_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_msent(&conn, &self.msent_symbol)
                            {
                                self.msent_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "EWMAVOL" | "EWMA_VOL" | "EWMAVOLATILITY" => {
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
                    self.ewmavol_symbol = sym;
                }
                self.show_ewmavol = true;
                if self.ewmavol_snapshot.symbol.is_empty() && !self.ewmavol_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ewmavol(
                                &conn,
                                &self.ewmavol_symbol,
                            ) {
                                self.ewmavol_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "KSNORM" | "KS_NORM" | "KS_TEST" | "KSTEST" => {
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
                    self.ksnorm_symbol = sym;
                }
                self.show_ksnorm = true;
                if self.ksnorm_snapshot.symbol.is_empty() && !self.ksnorm_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ksnorm(
                                &conn,
                                &self.ksnorm_symbol,
                            ) {
                                self.ksnorm_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ADTEST" | "AD_TEST" | "ANDERSON_DARLING" | "ANDERSONDARLING" => {
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
                    self.adtest_symbol = sym;
                }
                self.show_adtest = true;
                if self.adtest_snapshot.symbol.is_empty() && !self.adtest_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_adtest(
                                &conn,
                                &self.adtest_symbol,
                            ) {
                                self.adtest_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "LMOM" | "L_MOMENTS" | "LMOMENTS" | "HOSKING" => {
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
                    self.lmom_symbol = sym;
                }
                self.show_lmom = true;
                if self.lmom_snapshot.symbol.is_empty() && !self.lmom_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_lmom(&conn, &self.lmom_symbol)
                            {
                                self.lmom_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "KYLELAM" | "KYLE_LAMBDA" | "KYLELAMBDA" | "PRICE_IMPACT" | "PRICEIMPACT" => {
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
                    self.kylelam_symbol = sym;
                }
                self.show_kylelam = true;
                if self.kylelam_snapshot.symbol.is_empty() && !self.kylelam_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_kylelam(
                                &conn,
                                &self.kylelam_symbol,
                            ) {
                                self.kylelam_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PEAKOVER" | "PEAKS_OVER_THRESHOLD" | "POT" | "EVT_POT" | "EXCEEDANCES" => {
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
                    self.peakover_symbol = sym;
                }
                self.show_peakover = true;
                if self.peakover_snapshot.symbol.is_empty() && !self.peakover_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_peakover(
                                &conn,
                                &self.peakover_symbol,
                            ) {
                                self.peakover_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-145 Round 37 palette aliases ──
            "HIGUCHI" | "HIGUCHI_FD" | "FRACTAL_DIM" | "FRACTALDIM" | "HFD" => {
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
                    self.higuchi_symbol = sym;
                }
                self.show_higuchi = true;
                if self.higuchi_snapshot.symbol.is_empty() && !self.higuchi_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_higuchi(
                                &conn,
                                &self.higuchi_symbol,
                            ) {
                                self.higuchi_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PICKANDS" | "PICKANDS_TAIL" | "TAIL_INDEX_P" | "PICKANDSTAIL" => {
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
                    self.pickands_symbol = sym;
                }
                self.show_pickands = true;
                if self.pickands_snapshot.symbol.is_empty() && !self.pickands_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_pickands(
                                &conn,
                                &self.pickands_symbol,
                            ) {
                                self.pickands_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "KAPPA3" | "KAPPA_3" | "KAPPA3RATIO" | "KAPPA3_RATIO" | "KAPLAN_KNOWLES" => {
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
                    self.kappa3_symbol = sym;
                }
                self.show_kappa3 = true;
                if self.kappa3_snapshot.symbol.is_empty() && !self.kappa3_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_kappa3(
                                &conn,
                                &self.kappa3_symbol,
                            ) {
                                self.kappa3_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "LYAPUNOV" | "LYAPUNOV_EXP" | "LAMBDA_MAX" | "LYAPUNOVEXPONENT" | "ROSENSTEIN" => {
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
                    self.lyapunov_symbol = sym;
                }
                self.show_lyapunov = true;
                if self.lyapunov_snapshot.symbol.is_empty() && !self.lyapunov_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_lyapunov(
                                &conn,
                                &self.lyapunov_symbol,
                            ) {
                                self.lyapunov_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RANKAC" | "RANK_AUTOCORR" | "SPEARMAN_AC" | "RANKAUTOCORRELATION" | "SPEARMANLAGS" => {
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
                    self.rankac_symbol = sym;
                }
                self.show_rankac = true;
                if self.rankac_snapshot.symbol.is_empty() && !self.rankac_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_rankac(
                                &conn,
                                &self.rankac_symbol,
                            ) {
                                self.rankac_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-146 Round 38 palette aliases ──
            "BNSJUMP" | "BNS_JUMP" | "JUMPTEST" | "JUMP_TEST" | "BARNDORFF" | "BIPOWERJUMP" => {
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
                    self.bnsjump_symbol = sym;
                }
                self.show_bnsjump = true;
                if self.bnsjump_snapshot.symbol.is_empty() && !self.bnsjump_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_bnsjump(
                                &conn,
                                &self.bnsjump_symbol,
                            ) {
                                self.bnsjump_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PPROOT" | "PHILLIPS_PERRON" | "PHILLIPSPERRON" | "PP_TEST" | "PPTEST"
            | "UNITROOTPP" => {
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
                    self.pproot_symbol = sym;
                }
                self.show_pproot = true;
                if self.pproot_snapshot.symbol.is_empty() && !self.pproot_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_pproot(
                                &conn,
                                &self.pproot_symbol,
                            ) {
                                self.pproot_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MFDFA" | "MF_DFA" | "MULTIFRACTAL" | "MULTIFRACTALDFA" | "MFSPECTRUM" => {
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
                    self.mfdfa_symbol = sym;
                }
                self.show_mfdfa = true;
                if self.mfdfa_snapshot.symbol.is_empty() && !self.mfdfa_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_mfdfa(&conn, &self.mfdfa_symbol)
                            {
                                self.mfdfa_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HILLKS" | "HILL_KS" | "PARETO_KS" | "TAILFIT" | "HILLTAILFIT" | "HILLGOF" => {
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
                    self.hillks_symbol = sym;
                }
                self.show_hillks = true;
                if self.hillks_snapshot.symbol.is_empty() && !self.hillks_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_hillks(
                                &conn,
                                &self.hillks_symbol,
                            ) {
                                self.hillks_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "TSI" | "TRUE_STRENGTH" | "TRUESTRENGTHINDEX" | "BLAU_TSI" | "BLAUINDEX"
            | "MOMENTUMTSI" => {
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
                    self.tsi_symbol = sym;
                }
                self.show_tsi = true;
                if self.tsi_snapshot.symbol.is_empty() && !self.tsi_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_tsi(&conn, &self.tsi_symbol)
                            {
                                self.tsi_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-147 Round 39 palette aliases ──
            "GARCH11" | "GARCH" | "GARCH_11" | "BOLLERSLEV" | "CONDVOL" | "CONDITIONAL_VOL" => {
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
                    self.garch11_symbol = sym;
                }
                self.show_garch11 = true;
                if self.garch11_snapshot.symbol.is_empty() && !self.garch11_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_garch11(
                                &conn,
                                &self.garch11_symbol,
                            ) {
                                self.garch11_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SADF" | "SUP_ADF" | "SUPADF" | "BUBBLETEST" | "BUBBLE_TEST" | "PWY"
            | "PHILLIPS_WU_YU" => {
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
                    self.sadf_symbol = sym;
                }
                self.show_sadf = true;
                if self.sadf_snapshot.symbol.is_empty() && !self.sadf_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_sadf(&conn, &self.sadf_symbol)
                            {
                                self.sadf_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CORDIM" | "CORR_DIM" | "CORRDIM" | "D2" | "GRASSBERGER" | "GRASSBERGER_PROCACCIA" => {
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
                    self.cordim_symbol = sym;
                }
                self.show_cordim = true;
                if self.cordim_snapshot.symbol.is_empty() && !self.cordim_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cordim(
                                &conn,
                                &self.cordim_symbol,
                            ) {
                                self.cordim_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SKSPEC" | "SKEW_SPEC" | "ROLLING_SKEW" | "SKEWSPECTRUM" | "SKEWSTAB"
            | "SKEWSTABILITY" => {
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
                    self.skspec_symbol = sym;
                }
                self.show_skspec = true;
                if self.skspec_snapshot.symbol.is_empty() && !self.skspec_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_skspec(
                                &conn,
                                &self.skspec_symbol,
                            ) {
                                self.skspec_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "AUTOMI" | "AUTO_MI" | "MUTUALINFO" | "MUTUAL_INFORMATION" | "MI_ACF"
            | "INFOTHEOACF" => {
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
                    self.automi_symbol = sym;
                }
                self.show_automi = true;
                if self.automi_snapshot.symbol.is_empty() && !self.automi_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_automi(
                                &conn,
                                &self.automi_symbol,
                            ) {
                                self.automi_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-149 Round 40 palette aliases ──
            "DURBINWATSON" | "DURBIN_WATSON" | "DW" | "DWSTAT" | "DWTEST" | "RESIDAC" => {
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
                    self.durbinwatson_symbol = sym;
                }
                self.show_durbinwatson = true;
                if self.durbinwatson_snapshot.symbol.is_empty()
                    && !self.durbinwatson_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_durbinwatson(
                                &conn,
                                &self.durbinwatson_symbol,
                            ) {
                                self.durbinwatson_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "BDSTEST" | "BDS_TEST" | "BDS" | "BROCK_DECHERT" | "BROCKDECHERT" | "IIDTEST" => {
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
                    self.bdstest_symbol = sym;
                }
                self.show_bdstest = true;
                if self.bdstest_snapshot.symbol.is_empty() && !self.bdstest_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_bdstest(
                                &conn,
                                &self.bdstest_symbol,
                            ) {
                                self.bdstest_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "BREUSCHPAGAN" | "BREUSCH_PAGAN" | "BP" | "BPTEST" | "HETEROTEST" | "HETEROLMTEST" => {
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
                    self.breuschpagan_symbol = sym;
                }
                self.show_breuschpagan = true;
                if self.breuschpagan_snapshot.symbol.is_empty()
                    && !self.breuschpagan_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_breuschpagan(
                                &conn,
                                &self.breuschpagan_symbol,
                            ) {
                                self.breuschpagan_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "TURNPTS" | "TURN_PTS" | "TURNINGPOINTS" | "BARTELS" | "TURNINGTEST" | "TURNINGPTS" => {
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
                    self.turnpts_symbol = sym;
                }
                self.show_turnpts = true;
                if self.turnpts_snapshot.symbol.is_empty() && !self.turnpts_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_turnpts(
                                &conn,
                                &self.turnpts_symbol,
                            ) {
                                self.turnpts_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PERIODOGRAM" | "PERGRAM" | "DFTSPEC" | "SPECDENSITY" | "DOMINANTCYCLE"
            | "CYCLEFINDER" => {
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
                    self.periodogram_symbol = sym;
                }
                self.show_periodogram = true;
                if self.periodogram_snapshot.symbol.is_empty()
                    && !self.periodogram_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_periodogram(
                                &conn,
                                &self.periodogram_symbol,
                            ) {
                                self.periodogram_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MCLEODLI" | "MCLEOD" | "MLTEST" | "SQRETURNS" | "ARCHPORTMANTEAU" => {
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
                    self.mcleodli_symbol = sym;
                }
                self.show_mcleodli = true;
                if self.mcleodli_snapshot.symbol.is_empty() && !self.mcleodli_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_mcleodli(
                                &conn,
                                &self.mcleodli_symbol,
                            ) {
                                self.mcleodli_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "OUFIT" | "ORNSTEIN" | "OU" | "OUPROCESS" | "OU_FIT" | "MEANREVERTFIT" => {
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
                    self.oufit_symbol = sym;
                }
                self.show_oufit = true;
                if self.oufit_snapshot.symbol.is_empty() && !self.oufit_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_oufit(&conn, &self.oufit_symbol)
                            {
                                self.oufit_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "GPH" | "GEWEKE" | "GEWEKEPORTERHUDAK" | "LONGMEMORY" | "FRACTIONAL_D"
            | "LOGPERIODOGRAM" => {
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
                    self.gph_symbol = sym;
                }
                self.show_gph = true;
                if self.gph_snapshot.symbol.is_empty() && !self.gph_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_gph(&conn, &self.gph_symbol)
                            {
                                self.gph_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "BURGSPEC" | "BURG" | "MAXENTROPY" | "ARSPECTRUM" | "MESPEC" => {
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
                    self.burgspec_symbol = sym;
                }
                self.show_burgspec = true;
                if self.burgspec_snapshot.symbol.is_empty() && !self.burgspec_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_burgspec(
                                &conn,
                                &self.burgspec_symbol,
                            ) {
                                self.burgspec_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "KENDALLTAU" | "KTAU" | "RANKAUTOCORR" | "TAULAG1" | "KENDALLLAG" => {
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
                    self.kendalltau_symbol = sym;
                }
                self.show_kendalltau = true;
                if self.kendalltau_snapshot.symbol.is_empty() && !self.kendalltau_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_kendalltau(
                                &conn,
                                &self.kendalltau_symbol,
                            ) {
                                self.kendalltau_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-151 Round 42 palette aliases ──
            // NOTE: bare "SQUEEZE"/"DONCHIAN"/"KAMA"/"KAUFMAN" are already
            // bound to chart-overlay toggles — Round 42 research windows use
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
            }
            "SQUEEZEWATCHLIST"
            | "SQZWATCH"
            | "SHORT_SQUEEZE_WATCH"
            | "SQUEEZE_WATCH"
            | "SQUEEZELIST" => {
                self.show_squeeze_watchlist = true;
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
            }
            // ── ADR-152 Round 43 palette aliases ──
            // Bare ICHIMOKU / SUPERTREND / KELTNER / FISHER are already bound to
            // chart-overlay toggles upstream; only disambiguated forms are used here.
            "ICHIMOKUFIT" | "ICHIMOKU_WIN" | "IKH" | "KUMO" | "TENKAN_KIJUN" => {
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
            }
            "SUPERTRENDFIT" | "SUPERTREND_WIN" | "ST_FIT" | "ATR_TRAIL" | "SUPERTREND_ATR" => {
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
            }
            "KELTNERFIT" | "KELTNER_WIN" | "KC_FIT" | "KELTNERCHAN" | "KELCHAN" => {
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
            }
            "FISHERFIT" | "FISHER_WIN" | "FISHER_TRANSFORM" | "EHLERS_FISHER" | "FT_EHLERS" => {
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
            }
            "AROON" | "AROON_UP" | "AROON_DOWN" | "AROONFIT" => {
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
            }
            // ── ADR-153 Round 44 palette aliases ──
            // Bare ADX / CCI / PSAR are already bound to chart-overlay toggles upstream;
            // only disambiguated forms are used for ADX/CCI/PSAR research windows.
            // Bare CMF and MFI are unbound and kept as aliases.
            "ADXFIT" | "ADX_WIN" | "ADXREG" | "DIRECTIONAL_INDEX" | "WILDERADX" => {
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
                    self.adx_win_symbol = sym;
                }
                self.show_adx_win = true;
                if self.adx_win_snapshot.symbol.is_empty() && !self.adx_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_adx(&conn, &self.adx_win_symbol)
                            {
                                self.adx_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CCIFIT" | "CCI_WIN" | "CCIREG" | "COMMODITY_CHANNEL" | "LAMBERTCCI" => {
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
                    self.cci_win_symbol = sym;
                }
                self.show_cci_win = true;
                if self.cci_win_snapshot.symbol.is_empty() && !self.cci_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cci(&conn, &self.cci_win_symbol)
                            {
                                self.cci_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CMF" | "CMFFIT" | "CHAIKIN_MF" | "CHAIKIN_MONEY_FLOW" | "MONEYFLOW_CMF" => {
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
                    self.cmf_win_symbol = sym;
                }
                self.show_cmf_win = true;
                if self.cmf_win_snapshot.symbol.is_empty() && !self.cmf_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cmf(&conn, &self.cmf_win_symbol)
                            {
                                self.cmf_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MFI" | "MFIFIT" | "MONEY_FLOW_INDEX" | "MFIREG" | "MFI_14" => {
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
                    self.mfi_win_symbol = sym;
                }
                self.show_mfi_win = true;
                if self.mfi_win_snapshot.symbol.is_empty() && !self.mfi_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_mfi(&conn, &self.mfi_win_symbol)
                            {
                                self.mfi_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PSARFIT" | "PSAR_WIN" | "PARABOLIC_SAR" | "WILDER_SAR" | "SARFIT" => {
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
                    self.psar_win_symbol = sym;
                }
                self.show_psar_win = true;
                if self.psar_win_snapshot.symbol.is_empty() && !self.psar_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_psar(
                                &conn,
                                &self.psar_win_symbol,
                            ) {
                                self.psar_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-154 Round 45 palette aliases ──
            // Bare OBV and HMA collide with chart-overlay toggles upstream;
            // only disambiguated forms are used for OBV/HMA research windows.
            // Bare VORTEX, CHOP, TRIX are unbound and kept as aliases.
            "VORTEX" | "VORTEXFIT" | "VORTEX_WIN" | "VI" | "VI_14" | "BOTES_SIEPMAN" => {
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
                    self.vortex_win_symbol = sym;
                }
                self.show_vortex_win = true;
                if self.vortex_win_snapshot.symbol.is_empty() && !self.vortex_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_vortex(
                                &conn,
                                &self.vortex_win_symbol,
                            ) {
                                self.vortex_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CHOP" | "CHOPFIT" | "CHOP_WIN" | "CHOPPINESS" | "CHOPPINESS_INDEX" | "DREISS" => {
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
                    self.chop_win_symbol = sym;
                }
                self.show_chop_win = true;
                if self.chop_win_snapshot.symbol.is_empty() && !self.chop_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_chop(
                                &conn,
                                &self.chop_win_symbol,
                            ) {
                                self.chop_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "OBVFIT" | "OBV_WIN" | "OBVREG" | "GRANVILLE_OBV" | "ONBALANCE_VOLUME" => {
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
                    self.obv_win_symbol = sym;
                }
                self.show_obv_win = true;
                if self.obv_win_snapshot.symbol.is_empty() && !self.obv_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_obv(&conn, &self.obv_win_symbol)
                            {
                                self.obv_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "TRIX" | "TRIXFIT" | "TRIX_WIN" | "TRIPLE_EMA" | "HUTSON_TRIX" => {
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
                    self.trix_win_symbol = sym;
                }
                self.show_trix_win = true;
                if self.trix_win_snapshot.symbol.is_empty() && !self.trix_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_trix(
                                &conn,
                                &self.trix_win_symbol,
                            ) {
                                self.trix_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HMAFIT" | "HMA_WIN" | "HMAREG" | "HULL_MA" | "HULL_MOVING_AVG" => {
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
                    self.hma_win_symbol = sym;
                }
                self.show_hma_win = true;
                if self.hma_win_snapshot.symbol.is_empty() && !self.hma_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_hma(&conn, &self.hma_win_symbol)
                            {
                                self.hma_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-155 Round 46 palette aliases ──
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
            // ── ADR-156 Round 47 palette aliases ──
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
            // ── ADR-158 Round 48 palette aliases ──
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
            // ── ADR-160 Round 50 ──
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
            // ── ADR-161 Round 51 palette aliases ──
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
            // ── ADR-163 Round 52 palette aliases ──
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
            // ── ADR-166 Options Expiration Calendar ──
            "EXPCAL"
            | "OPTCAL"
            | "EXPIRY"
            | "EXPIRATIONS"
            | "OPTION_CALENDAR"
            | "OPTIONS_CALENDAR"
            | "OPTION_EXPIRATION_CALENDAR" => {
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
                    self.expcal_win_symbol = sym;
                }
                self.show_expcal_win = true;
                if self.expcal_win_calendar.is_empty() {
                    let today = chrono::Local::now().date_naive();
                    self.expcal_win_calendar =
                        typhoon_engine::core::research::compute_market_calendar(
                            today,
                            self.expcal_win_horizon_days,
                        );
                }
                if self.expcal_win_snapshot.symbol.is_empty() && !self.expcal_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_symbol_expirations(
                                    &conn,
                                    &self.expcal_win_symbol,
                                )
                            {
                                self.expcal_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-167 Round 55 ──
            "SMMA" | "SMMAFIT" | "SMMA_WIN" | "WILDER_MA" | "WILDER_SMMA" | "RMA"
            | "SMOOTHED_MA" => {
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
                    self.smma_win_symbol = sym;
                }
                self.show_smma_win = true;
                if self.smma_win_snapshot.symbol.is_empty() && !self.smma_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_smma(
                                &conn,
                                &self.smma_win_symbol,
                            ) {
                                self.smma_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ALLIGATOR"
            | "ALLIG"
            | "GATOR"
            | "ALLIGATOR_WIN"
            | "WILLIAMS_ALLIGATOR"
            | "BILL_WILLIAMS_ALLIGATOR" => {
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
                    self.alligator_win_symbol = sym;
                }
                self.show_alligator_win = true;
                if self.alligator_win_snapshot.symbol.is_empty()
                    && !self.alligator_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_alligator(
                                &conn,
                                &self.alligator_win_symbol,
                            ) {
                                self.alligator_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CRSI" | "CRSIFIT" | "CRSI_WIN" | "CONNORS_RSI" | "CONNORSRSI" => {
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
                    self.crsi_win_symbol = sym;
                }
                self.show_crsi_win = true;
                if self.crsi_win_snapshot.symbol.is_empty() && !self.crsi_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_crsi(
                                &conn,
                                &self.crsi_win_symbol,
                            ) {
                                self.crsi_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SEB" | "SEBFIT" | "SEB_WIN" | "STDERR_BANDS" | "STANDARD_ERROR_BANDS" | "SE_BANDS" => {
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
                    self.seb_win_symbol = sym;
                }
                self.show_seb_win = true;
                if self.seb_win_snapshot.symbol.is_empty() && !self.seb_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_seb(&conn, &self.seb_win_symbol)
                            {
                                self.seb_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "IMI" | "IMIFIT" | "IMI_WIN" | "INTRADAY_MOMENTUM_INDEX" | "CHANDE_IMI" => {
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
                    self.imi_win_symbol = sym;
                }
                self.show_imi_win = true;
                if self.imi_win_snapshot.symbol.is_empty() && !self.imi_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_imi(&conn, &self.imi_win_symbol)
                            {
                                self.imi_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "GMMA" | "GMMAFIT" | "GMMA_WIN" | "GUPPY" | "GUPPY_MMA" | "GUPPY_MULTIPLE_MA" => {
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
                    self.gmma_win_symbol = sym;
                }
                self.show_gmma_win = true;
                if self.gmma_win_snapshot.symbol.is_empty() && !self.gmma_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_gmma(
                                &conn,
                                &self.gmma_win_symbol,
                            ) {
                                self.gmma_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MAENV"
            | "MAENVFIT"
            | "MAENV_WIN"
            | "MA_ENVELOPE"
            | "MOVING_AVG_ENVELOPE"
            | "MA_ENV" => {
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
                    self.maenv_win_symbol = sym;
                }
                self.show_maenv_win = true;
                if self.maenv_win_snapshot.symbol.is_empty() && !self.maenv_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_maenv(
                                &conn,
                                &self.maenv_win_symbol,
                            ) {
                                self.maenv_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ADL"
            | "ADLFIT"
            | "ADL_WIN"
            | "ACCUM_DIST"
            | "ACCUMULATION_DISTRIBUTION"
            | "CHAIKIN_ADL"
            | "AD_LINE" => {
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
                    self.adl_win_symbol = sym;
                }
                self.show_adl_win = true;
                if self.adl_win_snapshot.symbol.is_empty() && !self.adl_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_adl(&conn, &self.adl_win_symbol)
                            {
                                self.adl_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VHF"
            | "VHFFIT"
            | "VHF_WIN"
            | "VERTHORZ"
            | "VERT_HORZ_FILTER"
            | "VERTICAL_HORIZONTAL_FILTER" => {
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
                    self.vhf_win_symbol = sym;
                }
                self.show_vhf_win = true;
                if self.vhf_win_snapshot.symbol.is_empty() && !self.vhf_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_vhf(&conn, &self.vhf_win_symbol)
                            {
                                self.vhf_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VROC"
            | "VROCFIT"
            | "VROC_WIN"
            | "VOLUME_ROC"
            | "VOL_ROC"
            | "VOLUME_RATE_OF_CHANGE" => {
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
                    self.vroc_win_symbol = sym;
                }
                self.show_vroc_win = true;
                if self.vroc_win_snapshot.symbol.is_empty() && !self.vroc_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_vroc(
                                &conn,
                                &self.vroc_win_symbol,
                            ) {
                                self.vroc_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "KDJ" | "KDJFIT" | "KDJ_WIN" | "K_D_J" | "KDJ_STOCH" | "STOCH_KDJ" => {
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
                    self.kdj_win_symbol = sym;
                }
                self.show_kdj_win = true;
                if self.kdj_win_snapshot.symbol.is_empty() && !self.kdj_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_kdj(&conn, &self.kdj_win_symbol)
                            {
                                self.kdj_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "QQE"
            | "QQEFIT"
            | "QQE_WIN"
            | "QQE_MOD"
            | "QUANT_QUAL_EST"
            | "QUANTITATIVE_QUALITATIVE" => {
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
                    self.qqe_win_symbol = sym;
                }
                self.show_qqe_win = true;
                if self.qqe_win_snapshot.symbol.is_empty() && !self.qqe_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_qqe(&conn, &self.qqe_win_symbol)
                            {
                                self.qqe_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PMO"
            | "PMOFIT"
            | "PMO_WIN"
            | "PRING_PMO"
            | "PRICE_MOMENTUM_OSC"
            | "PRICE_MOMENTUM_OSCILLATOR" => {
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
                    self.pmo_win_symbol = sym;
                }
                self.show_pmo_win = true;
                if self.pmo_win_snapshot.symbol.is_empty() && !self.pmo_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_pmo(&conn, &self.pmo_win_symbol)
                            {
                                self.pmo_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CFO"
            | "CFOFIT"
            | "CFO_WIN"
            | "FORECAST_OSC"
            | "CHANDE_FORECAST"
            | "FORECAST_OSCILLATOR" => {
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
                    self.cfo_win_symbol = sym;
                }
                self.show_cfo_win = true;
                if self.cfo_win_snapshot.symbol.is_empty() && !self.cfo_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cfo(&conn, &self.cfo_win_symbol)
                            {
                                self.cfo_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "TMF" | "TMFFIT" | "TMF_WIN" | "TWIGGS_MF" | "TWIGGS_MONEY_FLOW"
            | "TWIGGSMONEYFLOW" => {
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
                    self.tmf_win_symbol = sym;
                }
                self.show_tmf_win = true;
                if self.tmf_win_snapshot.symbol.is_empty() && !self.tmf_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_tmf(&conn, &self.tmf_win_symbol)
                            {
                                self.tmf_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "FRACTALS_WIN"
            | "FRACTAL_WIN"
            | "FRACTALS_RESEARCH"
            | "BILL_WILLIAMS_FRACTALS"
            | "BW_FRACTALS" => {
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
                    self.fractals_win_symbol = sym;
                }
                self.show_fractals_win = true;
                if self.fractals_win_snapshot.symbol.is_empty()
                    && !self.fractals_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_fractals(
                                &conn,
                                &self.fractals_win_symbol,
                            ) {
                                self.fractals_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "IFT_RSI" | "IFTRSI" | "INVERSE_FISHER_RSI" | "EHLERS_IFT_RSI" | "INVFISHER_RSI" => {
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
                    self.ift_rsi_win_symbol = sym;
                }
                self.show_ift_rsi_win = true;
                if self.ift_rsi_win_snapshot.symbol.is_empty()
                    && !self.ift_rsi_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ift_rsi(
                                &conn,
                                &self.ift_rsi_win_symbol,
                            ) {
                                self.ift_rsi_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MAMA" | "MAMA_WIN" | "MESA_ADAPTIVE_MA" | "MESA_AMA" | "EHLERS_MAMA" => {
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
                    self.mama_win_symbol = sym;
                }
                self.show_mama_win = true;
                if self.mama_win_snapshot.symbol.is_empty() && !self.mama_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_mama(
                                &conn,
                                &self.mama_win_symbol,
                            ) {
                                self.mama_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "COG" | "COG_WIN" | "CENTER_OF_GRAVITY" | "EHLERS_COG" | "COG_OSC" => {
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
                    self.cog_win_symbol = sym;
                }
                self.show_cog_win = true;
                if self.cog_win_snapshot.symbol.is_empty() && !self.cog_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cog(&conn, &self.cog_win_symbol)
                            {
                                self.cog_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DIDI" | "DIDI_INDEX" | "DIDI_NEEDLES" | "AGUIAR_DIDI" | "DIDI_WIN" => {
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
                    self.didi_win_symbol = sym;
                }
                self.show_didi_win = true;
                if self.didi_win_snapshot.symbol.is_empty() && !self.didi_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_didi(
                                &conn,
                                &self.didi_win_symbol,
                            ) {
                                self.didi_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DEMARKER" | "DEM" | "DEMARK" | "DEMARKER_WIN" | "DEMARKER_RESEARCH" => {
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
                    self.demarker_win_symbol = sym;
                }
                self.show_demarker_win = true;
                if self.demarker_win_snapshot.symbol.is_empty()
                    && !self.demarker_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_demarker(
                                &conn,
                                &self.demarker_win_symbol,
                            ) {
                                self.demarker_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "GATOR_OSC" | "GATOR_OSCILLATOR" | "GATOR_WIN" | "BW_GATOR" | "BILL_WILLIAMS_GATOR" => {
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
                    self.gator_win_symbol = sym;
                }
                self.show_gator_win = true;
                if self.gator_win_snapshot.symbol.is_empty() && !self.gator_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_gator(
                                &conn,
                                &self.gator_win_symbol,
                            ) {
                                self.gator_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "BW_MFI"
            | "BWMFI"
            | "MARKET_FACILITATION_INDEX"
            | "BILL_WILLIAMS_MFI"
            | "BWMFI_WIN" => {
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
                    self.bw_mfi_win_symbol = sym;
                }
                self.show_bw_mfi_win = true;
                if self.bw_mfi_win_snapshot.symbol.is_empty() && !self.bw_mfi_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_bw_mfi(
                                &conn,
                                &self.bw_mfi_win_symbol,
                            ) {
                                self.bw_mfi_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VWMA" | "VWMA_WIN" | "VOL_WEIGHTED_MA" | "VOLUME_WEIGHTED_MA" | "VWMA_RESEARCH" => {
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
                    self.vwma_win_symbol = sym;
                }
                self.show_vwma_win = true;
                if self.vwma_win_snapshot.symbol.is_empty() && !self.vwma_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_vwma(
                                &conn,
                                &self.vwma_win_symbol,
                            ) {
                                self.vwma_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "STDDEV" | "STD_DEV" | "STANDARD_DEVIATION" | "ROLLING_STDDEV" | "STDDEV_WIN" => {
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
                    self.stddev_win_symbol = sym;
                }
                self.show_stddev_win = true;
                if self.stddev_win_snapshot.symbol.is_empty() && !self.stddev_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_stddev(
                                &conn,
                                &self.stddev_win_symbol,
                            ) {
                                self.stddev_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-172 Round 60: WMA / RAINBOW / MESA_SINE / FRAMA / IBS ──
            "WMA" | "WEIGHTED_MA" | "WMA_WIN" | "LINEAR_WEIGHTED_MA" => {
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
            // ── ADR-174 Round 62 palette aliases ──
            // Note: "MASS_INDEX"/"DORSEY_MASS" already claimed by ADR-156 Round 47 curvefit.
            "MASSINDEX" | "MI" | "MASS_INDEX_WIN" | "MINDEX" | "MASS_25" => {
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
                    self.mass_index_win_symbol = sym;
                }
                self.show_mass_index_win = true;
                if self.mass_index_win_snapshot.symbol.is_empty()
                    && !self.mass_index_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_mass_index(
                                &conn,
                                &self.mass_index_win_symbol,
                            ) {
                                self.mass_index_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "NATR" | "NORMALIZED_ATR" | "NATR_WIN" | "NORMALIZED_ATR_WIN" | "ATR_PCT" => {
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
                    self.natr_win_symbol = sym;
                }
                self.show_natr_win = true;
                if self.natr_win_snapshot.symbol.is_empty() && !self.natr_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_natr(
                                &conn,
                                &self.natr_win_symbol,
                            ) {
                                self.natr_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // Note: bare "SQUEEZE" is a chart toggle, not claimed here.
            "TTM_SQUEEZE" | "TTMSQUEEZE" | "TTM_SQUEEZE_WIN" | "CARTER_SQUEEZE" | "TTM" => {
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
                    self.ttm_squeeze_win_symbol = sym;
                }
                self.show_ttm_squeeze_win = true;
                if self.ttm_squeeze_win_snapshot.symbol.is_empty()
                    && !self.ttm_squeeze_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ttm_squeeze(
                                &conn,
                                &self.ttm_squeeze_win_symbol,
                            ) {
                                self.ttm_squeeze_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // Note: "FORCE_INDEX"/"ELDER_FORCE" already claimed by ADR-158 Round 48 EFI curvefit.
            "FORCEINDEX" | "FORCE" | "FI" | "FORCE_INDEX_WIN" | "FORCE13" => {
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
                    self.force_index_win_symbol = sym;
                }
                self.show_force_index_win = true;
                if self.force_index_win_snapshot.symbol.is_empty()
                    && !self.force_index_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_force_index(
                                &conn,
                                &self.force_index_win_symbol,
                            ) {
                                self.force_index_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "TRANGE" | "TRUE_RANGE" | "TR" | "TRANGE_WIN" | "RAW_TRUE_RANGE" => {
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
                    self.trange_win_symbol = sym;
                }
                self.show_trange_win = true;
                if self.trange_win_snapshot.symbol.is_empty() && !self.trange_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_trange(
                                &conn,
                                &self.trange_win_symbol,
                            ) {
                                self.trange_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-175 Round 63 palette aliases ──
            "LINEARREG_SLOPE" | "LINREG_SLOPE" | "LINREGSLOPE" | "LRSLOPE" | "SLOPE" => {
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
                    self.linearreg_slope_win_symbol = sym;
                }
                self.show_linearreg_slope_win = true;
                if self.linearreg_slope_win_snapshot.symbol.is_empty()
                    && !self.linearreg_slope_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_linearreg_slope(
                                    &conn,
                                    &self.linearreg_slope_win_symbol,
                                )
                            {
                                self.linearreg_slope_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HT_DCPERIOD" | "HTDCPERIOD" | "DCPERIOD" | "HILBERT_PERIOD" | "CYCLE_PERIOD" => {
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
                    self.ht_dcperiod_win_symbol = sym;
                }
                self.show_ht_dcperiod_win = true;
                if self.ht_dcperiod_win_snapshot.symbol.is_empty()
                    && !self.ht_dcperiod_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_dcperiod(
                                &conn,
                                &self.ht_dcperiod_win_symbol,
                            ) {
                                self.ht_dcperiod_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HT_TRENDMODE" | "HTTRENDMODE" | "TRENDMODE" | "HILBERT_TRENDMODE"
            | "CYCLE_TRENDMODE" => {
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
                    self.ht_trendmode_win_symbol = sym;
                }
                self.show_ht_trendmode_win = true;
                if self.ht_trendmode_win_snapshot.symbol.is_empty()
                    && !self.ht_trendmode_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_trendmode(
                                &conn,
                                &self.ht_trendmode_win_symbol,
                            ) {
                                self.ht_trendmode_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ACCBANDS" | "ACCELERATION_BANDS" | "ACCBAND" | "HEADLEY" | "ACC_BANDS" => {
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
                    self.accbands_win_symbol = sym;
                }
                self.show_accbands_win = true;
                if self.accbands_win_snapshot.symbol.is_empty()
                    && !self.accbands_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_accbands(
                                &conn,
                                &self.accbands_win_symbol,
                            ) {
                                self.accbands_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "STOCHF" | "STOCHFAST" | "FAST_STOCH" | "FASTSTOCH" | "STOCH_FAST" => {
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
                    self.stochf_win_symbol = sym;
                }
                self.show_stochf_win = true;
                if self.stochf_win_snapshot.symbol.is_empty() && !self.stochf_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_stochf(
                                &conn,
                                &self.stochf_win_symbol,
                            ) {
                                self.stochf_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-176 Round 64 palette aliases ──
            "LINEARREG" | "LINEARREG_FIT" | "LINEAR_REG" | "LINEARREG_WIN" | "LINREG_FITTED" => {
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
                    self.linearreg_win_symbol = sym;
                }
                self.show_linearreg_win = true;
                if self.linearreg_win_snapshot.symbol.is_empty()
                    && !self.linearreg_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_linearreg(
                                &conn,
                                &self.linearreg_win_symbol,
                            ) {
                                self.linearreg_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "LINEARREG_ANGLE" | "LREGANGLE" | "LINEAR_REG_ANGLE" | "LINREGANGLE" | "LRANGLE" => {
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
                    self.linearreg_angle_win_symbol = sym;
                }
                self.show_linearreg_angle_win = true;
                if self.linearreg_angle_win_snapshot.symbol.is_empty()
                    && !self.linearreg_angle_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_linearreg_angle(
                                    &conn,
                                    &self.linearreg_angle_win_symbol,
                                )
                            {
                                self.linearreg_angle_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HT_DCPHASE" | "DCPHASE" | "HILBERT_DCPHASE" | "HTDCPHASE" | "CYCLE_PHASE" => {
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
                    self.ht_dcphase_win_symbol = sym;
                }
                self.show_ht_dcphase_win = true;
                if self.ht_dcphase_win_snapshot.symbol.is_empty()
                    && !self.ht_dcphase_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_dcphase(
                                &conn,
                                &self.ht_dcphase_win_symbol,
                            ) {
                                self.ht_dcphase_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HT_SINE" | "HTSINE" | "HILBERT_SINE" | "SINEWAVE" | "LEADSINE" => {
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
                    self.ht_sine_win_symbol = sym;
                }
                self.show_ht_sine_win = true;
                if self.ht_sine_win_snapshot.symbol.is_empty()
                    && !self.ht_sine_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_sine(
                                &conn,
                                &self.ht_sine_win_symbol,
                            ) {
                                self.ht_sine_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HT_PHASOR" | "HTPHASOR" | "HILBERT_PHASOR" | "PHASOR" | "IQ_COMP" => {
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
                    self.ht_phasor_win_symbol = sym;
                }
                self.show_ht_phasor_win = true;
                if self.ht_phasor_win_snapshot.symbol.is_empty()
                    && !self.ht_phasor_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_phasor(
                                &conn,
                                &self.ht_phasor_win_symbol,
                            ) {
                                self.ht_phasor_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-177 Round 65 palette aliases ──
            "MIDPRICE" | "MID_PRICE" | "MIDBAR" | "MIDBARPRICE" | "HLMIDPRICE" => {
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
                    self.midprice_win_symbol = sym;
                }
                self.show_midprice_win = true;
                if self.midprice_win_snapshot.symbol.is_empty()
                    && !self.midprice_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_midprice(
                                &conn,
                                &self.midprice_win_symbol,
                            ) {
                                self.midprice_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "APO" | "ABS_PRICE_OSC" | "ABSPRICEOSC" | "ABSPO" | "APOWIN" => {
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
                    self.apo_win_symbol = sym;
                }
                self.show_apo_win = true;
                if self.apo_win_snapshot.symbol.is_empty() && !self.apo_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_apo(&conn, &self.apo_win_symbol)
                            {
                                self.apo_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MOMRAW" | "MOMENTUM_RAW" | "MOM_TA" | "RAWMOM" | "TALIB_MOM" => {
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
                    self.mom_win_symbol = sym;
                }
                self.show_mom_win = true;
                if self.mom_win_snapshot.symbol.is_empty() && !self.mom_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_mom(&conn, &self.mom_win_symbol)
                            {
                                self.mom_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SAREXT" | "SAR_EXT" | "EXTENDED_SAR" | "SAREXTENDED" | "PSAR_EXT" => {
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
                    self.sarext_win_symbol = sym;
                }
                self.show_sarext_win = true;
                if self.sarext_win_snapshot.symbol.is_empty() && !self.sarext_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_sarext(
                                &conn,
                                &self.sarext_win_symbol,
                            ) {
                                self.sarext_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ADXR" | "ADX_RATING" | "ADX_R" | "ADXRATING" | "ADX_RANK" => {
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
                    self.adxr_win_symbol = sym;
                }
                self.show_adxr_win = true;
                if self.adxr_win_snapshot.symbol.is_empty() && !self.adxr_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_adxr(
                                &conn,
                                &self.adxr_win_symbol,
                            ) {
                                self.adxr_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "AVGPRICE" | "AVG_PRICE" | "OHLC_AVG" | "OHLCAVG" | "AVGOHLC" => {
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
                    self.avgprice_win_symbol = sym;
                }
                self.show_avgprice_win = true;
                if self.avgprice_win_snapshot.symbol.is_empty()
                    && !self.avgprice_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_avgprice(
                                &conn,
                                &self.avgprice_win_symbol,
                            ) {
                                self.avgprice_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MEDPRICE" | "MED_PRICE" | "HLMED" | "HLMEDIAN" | "RANGEMEDIAN" => {
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
                    self.medprice_win_symbol = sym;
                }
                self.show_medprice_win = true;
                if self.medprice_win_snapshot.symbol.is_empty()
                    && !self.medprice_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_medprice(
                                &conn,
                                &self.medprice_win_symbol,
                            ) {
                                self.medprice_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "TYPPRICE" | "TYP_PRICE" | "TYPICAL_PRICE" | "TYPICALPRICE" | "HLC3" => {
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
                    self.typprice_win_symbol = sym;
                }
                self.show_typprice_win = true;
                if self.typprice_win_snapshot.symbol.is_empty()
                    && !self.typprice_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_typprice(
                                &conn,
                                &self.typprice_win_symbol,
                            ) {
                                self.typprice_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "WCLPRICE" | "WCL_PRICE" | "WEIGHTED_CLOSE" | "WEIGHTEDCLOSE" | "HLCC4" => {
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
                    self.wclprice_win_symbol = sym;
                }
                self.show_wclprice_win = true;
                if self.wclprice_win_snapshot.symbol.is_empty()
                    && !self.wclprice_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_wclprice(
                                &conn,
                                &self.wclprice_win_symbol,
                            ) {
                                self.wclprice_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VARIANCE" | "VARIANCE_WIN" | "CLOSE_VARIANCE" | "CVARIANCE" | "VARWIN" => {
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
                    self.variance_win_symbol = sym;
                }
                self.show_variance_win = true;
                if self.variance_win_snapshot.symbol.is_empty()
                    && !self.variance_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_variance(
                                &conn,
                                &self.variance_win_symbol,
                            ) {
                                self.variance_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-179 Round 67: DMI family ──
            "PLUS_DI" | "PDI" | "DI_PLUS" | "DIPOS" | "WILDER_PDI" => {
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
                    self.plus_di_win_symbol = sym;
                }
                self.show_plus_di_win = true;
                if self.plus_di_win_snapshot.symbol.is_empty()
                    && !self.plus_di_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_plus_di(
                                &conn,
                                &self.plus_di_win_symbol,
                            ) {
                                self.plus_di_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MINUS_DI" | "MDI" | "DI_MINUS" | "DINEG" | "WILDER_MDI" => {
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
                    self.minus_di_win_symbol = sym;
                }
                self.show_minus_di_win = true;
                if self.minus_di_win_snapshot.symbol.is_empty()
                    && !self.minus_di_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_minus_di(
                                &conn,
                                &self.minus_di_win_symbol,
                            ) {
                                self.minus_di_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PLUS_DM" | "PDM" | "DM_PLUS" | "DMPOS" | "WILDER_PDM" => {
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
                    self.plus_dm_win_symbol = sym;
                }
                self.show_plus_dm_win = true;
                if self.plus_dm_win_snapshot.symbol.is_empty()
                    && !self.plus_dm_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_plus_dm(
                                &conn,
                                &self.plus_dm_win_symbol,
                            ) {
                                self.plus_dm_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MINUS_DM" | "MDM" | "DM_MINUS" | "DMNEG" | "WILDER_MDM" => {
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
                    self.minus_dm_win_symbol = sym;
                }
                self.show_minus_dm_win = true;
                if self.minus_dm_win_snapshot.symbol.is_empty()
                    && !self.minus_dm_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_minus_dm(
                                &conn,
                                &self.minus_dm_win_symbol,
                            ) {
                                self.minus_dm_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DX" | "DX_WILDER" | "DXWIN" | "DIRIDX" | "WILDER_DX" => {
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
                    self.dx_win_symbol = sym;
                }
                self.show_dx_win = true;
                if self.dx_win_snapshot.symbol.is_empty() && !self.dx_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_dx(&conn, &self.dx_win_symbol)
                            {
                                self.dx_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-180 Round 68 ──
            "ROC" | "ROC_WILDER" | "ROCWIN" | "ROCRATE" | "RATE_OF_CHANGE" => {
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
                    self.roc_win_symbol = sym;
                }
                self.show_roc_win = true;
                if self.roc_win_snapshot.symbol.is_empty() && !self.roc_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_roc(&conn, &self.roc_win_symbol)
                            {
                                self.roc_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ROCP" | "ROCP_WILDER" | "ROCPWIN" | "ROCPCT" | "ROC_PCT" => {
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
                    self.rocp_win_symbol = sym;
                }
                self.show_rocp_win = true;
                if self.rocp_win_snapshot.symbol.is_empty() && !self.rocp_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_rocp(
                                &conn,
                                &self.rocp_win_symbol,
                            ) {
                                self.rocp_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ROCR" | "ROCR_WILDER" | "ROCRWIN" | "ROCRATIO" | "ROC_RATIO" => {
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
                    self.rocr_win_symbol = sym;
                }
                self.show_rocr_win = true;
                if self.rocr_win_snapshot.symbol.is_empty() && !self.rocr_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_rocr(
                                &conn,
                                &self.rocr_win_symbol,
                            ) {
                                self.rocr_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ROCR100" | "ROCR100_WILDER" | "ROCR100WIN" | "ROCR100IDX" | "ROC_RATIO_100" => {
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
                    self.rocr100_win_symbol = sym;
                }
                self.show_rocr100_win = true;
                if self.rocr100_win_snapshot.symbol.is_empty()
                    && !self.rocr100_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_rocr100(
                                &conn,
                                &self.rocr100_win_symbol,
                            ) {
                                self.rocr100_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CORREL" | "CORRWIN" | "ROLLCORR" | "AUTOCORR" | "PEARSON_AUTO" => {
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
                    self.correl_win_symbol = sym;
                }
                self.show_correl_win = true;
                if self.correl_win_snapshot.symbol.is_empty() && !self.correl_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_correl(
                                &conn,
                                &self.correl_win_symbol,
                            ) {
                                self.correl_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MIN" | "MINWIN" | "MIN_CLOSE" | "LOW_BAND" | "ROLL_MIN" => {
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
                    self.min_win_symbol = sym;
                }
                self.show_min_win = true;
                if self.min_win_snapshot.symbol.is_empty() && !self.min_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_min(&conn, &self.min_win_symbol)
                            {
                                self.min_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MAX" | "MAXWIN" | "MAX_CLOSE" | "HIGH_BAND" | "ROLL_MAX" => {
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
                    self.max_win_symbol = sym;
                }
                self.show_max_win = true;
                if self.max_win_snapshot.symbol.is_empty() && !self.max_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_max(&conn, &self.max_win_symbol)
                            {
                                self.max_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MINMAX" | "MINMAXWIN" | "RANGE_BAND" | "HL_RANGE" | "EXTREMA" => {
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
                    self.minmax_win_symbol = sym;
                }
                self.show_minmax_win = true;
                if self.minmax_win_snapshot.symbol.is_empty() && !self.minmax_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_minmax(
                                &conn,
                                &self.minmax_win_symbol,
                            ) {
                                self.minmax_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MININDEX" | "MINIDXWIN" | "LOW_IDX" | "MIN_AGE" | "LOW_RECENCY" => {
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
                    self.minindex_win_symbol = sym;
                }
                self.show_minindex_win = true;
                if self.minindex_win_snapshot.symbol.is_empty()
                    && !self.minindex_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_minindex(
                                &conn,
                                &self.minindex_win_symbol,
                            ) {
                                self.minindex_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MAXINDEX" | "MAXIDXWIN" | "HIGH_IDX" | "MAX_AGE" | "HIGH_RECENCY" => {
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
                    self.maxindex_win_symbol = sym;
                }
                self.show_maxindex_win = true;
                if self.maxindex_win_snapshot.symbol.is_empty()
                    && !self.maxindex_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_maxindex(
                                &conn,
                                &self.maxindex_win_symbol,
                            ) {
                                self.maxindex_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "BBANDS" | "BBANDSWIN" | "BB_BANDS" | "BBAND" | "BOLL_BANDS" => {
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
                    self.bbands_win_symbol = sym;
                }
                self.show_bbands_win = true;
                if self.bbands_win_snapshot.symbol.is_empty() && !self.bbands_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_bbands(
                                &conn,
                                &self.bbands_win_symbol,
                            ) {
                                self.bbands_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "AD" | "AD_LINE_TALIB" | "AD_CHAIKIN" | "ADWIN" | "TALIB_AD" => {
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
                    self.ad_win_symbol = sym;
                }
                self.show_ad_win = true;
                if self.ad_win_snapshot.symbol.is_empty() && !self.ad_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_ad(&conn, &self.ad_win_symbol)
                            {
                                self.ad_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ADOSC" | "ADOSCWIN" | "TALIB_ADOSC" | "AD_OSCILLATOR" | "CHAIKIN_ADO" => {
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
                    self.adosc_win_symbol = sym;
                }
                self.show_adosc_win = true;
                if self.adosc_win_snapshot.symbol.is_empty() && !self.adosc_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_adosc(
                                &conn,
                                &self.adosc_win_symbol,
                            ) {
                                self.adosc_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SUM" | "SUMWIN" | "ROLLSUM" | "CLOSE_SUM" | "SUM_CLOSE" => {
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
                    self.sum_win_symbol = sym;
                }
                self.show_sum_win = true;
                if self.sum_win_snapshot.symbol.is_empty() && !self.sum_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_sum(&conn, &self.sum_win_symbol)
                            {
                                self.sum_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "LINEARREG_INTERCEPT"
            | "LINREG_INTERCEPT"
            | "LINTERCEPT"
            | "LRINTERCEPT"
            | "REG_INTERCEPT"
            | "LINEARREG_B" => {
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
                    self.linreg_intercept_win_symbol = sym;
                }
                self.show_linreg_intercept_win = true;
                if self.linreg_intercept_win_snapshot.symbol.is_empty()
                    && !self.linreg_intercept_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_linreg_intercept(
                                    &conn,
                                    &self.linreg_intercept_win_symbol,
                                )
                            {
                                self.linreg_intercept_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-183 Round 71 palette aliases ──
            "AROONOSC" | "AROONOSCWIN" | "AROON_OSC" | "AROONOSCILLATOR" | "AROON_DIFF" => {
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
                    self.aroonosc_win_symbol = sym;
                }
                self.show_aroonosc_win = true;
                if self.aroonosc_win_snapshot.symbol.is_empty()
                    && !self.aroonosc_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_aroonosc(
                                &conn,
                                &self.aroonosc_win_symbol,
                            ) {
                                self.aroonosc_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MINMAXINDEX" | "MMIDXWIN" | "MINMAX_IDX" | "EXTREMA_IDX" | "HL_IDX" => {
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
                    self.minmaxindex_win_symbol = sym;
                }
                self.show_minmaxindex_win = true;
                if self.minmaxindex_win_snapshot.symbol.is_empty()
                    && !self.minmaxindex_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_minmaxindex(
                                &conn,
                                &self.minmaxindex_win_symbol,
                            ) {
                                self.minmaxindex_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MACDEXT" | "MACDEXTWIN" | "MACD_EXT" | "MACD_CONFIG" | "MACD_FLEX" => {
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
                    self.macdext_win_symbol = sym;
                }
                self.show_macdext_win = true;
                if self.macdext_win_snapshot.symbol.is_empty()
                    && !self.macdext_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_macdext(
                                &conn,
                                &self.macdext_win_symbol,
                            ) {
                                self.macdext_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MACDFIX" | "MACDFIXWIN" | "MACD_FIX" | "MACD_12_26" | "MACD_STD" => {
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
                    self.macdfix_win_symbol = sym;
                }
                self.show_macdfix_win = true;
                if self.macdfix_win_snapshot.symbol.is_empty()
                    && !self.macdfix_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_macdfix(
                                &conn,
                                &self.macdfix_win_symbol,
                            ) {
                                self.macdfix_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MAVP" | "MAVPWIN" | "VAR_PERIOD_MA" | "MA_VARPERIOD" | "MA_DYNAMIC" => {
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
                    self.mavp_win_symbol = sym;
                }
                self.show_mavp_win = true;
                if self.mavp_win_snapshot.symbol.is_empty() && !self.mavp_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_mavp(
                                &conn,
                                &self.mavp_win_symbol,
                            ) {
                                self.mavp_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-184 Round 72 — CDL* candlestick patterns ──
            "CDLDOJI" | "CDLDOJIWIN" | "DOJI" | "DOJI_PATTERN" | "DOJI_CANDLE" => {
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
                    self.cdl_doji_win_symbol = sym;
                }
                self.show_cdl_doji_win = true;
                if self.cdl_doji_win_snapshot.symbol.is_empty()
                    && !self.cdl_doji_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_doji(
                                &conn,
                                &self.cdl_doji_win_symbol,
                            ) {
                                self.cdl_doji_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLHAMMER" | "CDLHAMMERWIN" | "HAMMER" | "HAMMER_PATTERN" | "HAMMER_CANDLE" => {
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
                    self.cdl_hammer_win_symbol = sym;
                }
                self.show_cdl_hammer_win = true;
                if self.cdl_hammer_win_snapshot.symbol.is_empty()
                    && !self.cdl_hammer_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_hammer(
                                &conn,
                                &self.cdl_hammer_win_symbol,
                            ) {
                                self.cdl_hammer_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLSHOOTINGSTAR"
            | "SHOOTINGSTAR"
            | "SHOOTING_STAR"
            | "CDLSHOOTINGSTARWIN"
            | "SHOOTING_STAR_PATTERN" => {
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
                    self.cdl_shooting_star_win_symbol = sym;
                }
                self.show_cdl_shooting_star_win = true;
                if self.cdl_shooting_star_win_snapshot.symbol.is_empty()
                    && !self.cdl_shooting_star_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_shooting_star(
                                    &conn,
                                    &self.cdl_shooting_star_win_symbol,
                                )
                            {
                                self.cdl_shooting_star_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLENGULFING" | "ENGULFING" | "CDLENGULFINGWIN" | "ENGULFING_PATTERN"
            | "ENGULFING_CANDLE" => {
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
                    self.cdl_engulfing_win_symbol = sym;
                }
                self.show_cdl_engulfing_win = true;
                if self.cdl_engulfing_win_snapshot.symbol.is_empty()
                    && !self.cdl_engulfing_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_engulfing(
                                    &conn,
                                    &self.cdl_engulfing_win_symbol,
                                )
                            {
                                self.cdl_engulfing_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLHARAMI" | "HARAMI" | "CDLHARAMIWIN" | "HARAMI_PATTERN" | "INSIDE_BAR" => {
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
                    self.cdl_harami_win_symbol = sym;
                }
                self.show_cdl_harami_win = true;
                if self.cdl_harami_win_snapshot.symbol.is_empty()
                    && !self.cdl_harami_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_harami(
                                &conn,
                                &self.cdl_harami_win_symbol,
                            ) {
                                self.cdl_harami_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-185 Round 73 — CDL* 3-bar / 2-bar patterns ──
            "CDLMORNINGSTAR"
            | "MORNINGSTAR"
            | "MORNING_STAR"
            | "CDLMORNINGSTARWIN"
            | "MORNING_STAR_PATTERN" => {
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
                    self.cdl_morning_star_win_symbol = sym;
                }
                self.show_cdl_morning_star_win = true;
                if self.cdl_morning_star_win_snapshot.symbol.is_empty()
                    && !self.cdl_morning_star_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_morning_star(
                                    &conn,
                                    &self.cdl_morning_star_win_symbol,
                                )
                            {
                                self.cdl_morning_star_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLEVENINGSTAR"
            | "EVENINGSTAR"
            | "EVENING_STAR"
            | "CDLEVENINGSTARWIN"
            | "EVENING_STAR_PATTERN" => {
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
                    self.cdl_evening_star_win_symbol = sym;
                }
                self.show_cdl_evening_star_win = true;
                if self.cdl_evening_star_win_snapshot.symbol.is_empty()
                    && !self.cdl_evening_star_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_evening_star(
                                    &conn,
                                    &self.cdl_evening_star_win_symbol,
                                )
                            {
                                self.cdl_evening_star_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDL3BLACKCROWS"
            | "THREEBLACKCROWS"
            | "THREE_BLACK_CROWS"
            | "BLACK_CROWS"
            | "CDLTHREEBLACKCROWSWIN" => {
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
                    self.cdl_three_black_crows_win_symbol = sym;
                }
                self.show_cdl_three_black_crows_win = true;
                if self.cdl_three_black_crows_win_snapshot.symbol.is_empty()
                    && !self.cdl_three_black_crows_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_three_black_crows(
                                    &conn,
                                    &self.cdl_three_black_crows_win_symbol,
                                )
                            {
                                self.cdl_three_black_crows_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDL3WHITESOLDIERS"
            | "THREEWHITESOLDIERS"
            | "THREE_WHITE_SOLDIERS"
            | "WHITE_SOLDIERS"
            | "CDLTHREEWHITESOLDIERSWIN" => {
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
                    self.cdl_three_white_soldiers_win_symbol = sym;
                }
                self.show_cdl_three_white_soldiers_win = true;
                if self.cdl_three_white_soldiers_win_snapshot.symbol.is_empty()
                    && !self.cdl_three_white_soldiers_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_three_white_soldiers(
                                    &conn,
                                    &self.cdl_three_white_soldiers_win_symbol,
                                )
                            {
                                self.cdl_three_white_soldiers_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLDARKCLOUDCOVER"
            | "DARKCLOUDCOVER"
            | "DARK_CLOUD_COVER"
            | "DARK_CLOUD"
            | "CDLDARKCLOUDCOVERWIN" => {
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
                    self.cdl_dark_cloud_cover_win_symbol = sym;
                }
                self.show_cdl_dark_cloud_cover_win = true;
                if self.cdl_dark_cloud_cover_win_snapshot.symbol.is_empty()
                    && !self.cdl_dark_cloud_cover_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_dark_cloud_cover(
                                    &conn,
                                    &self.cdl_dark_cloud_cover_win_symbol,
                                )
                            {
                                self.cdl_dark_cloud_cover_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-186 Round 74 — CDL* piercing / doji variants / hammer mirrors ──
            "CDLPIERCING" | "PIERCING" | "PIERCING_LINE" | "PIERCINGLINE" | "CDLPIERCINGWIN" => {
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
                    self.cdl_piercing_win_symbol = sym;
                }
                self.show_cdl_piercing_win = true;
                if self.cdl_piercing_win_snapshot.symbol.is_empty()
                    && !self.cdl_piercing_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_piercing(
                                &conn,
                                &self.cdl_piercing_win_symbol,
                            ) {
                                self.cdl_piercing_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLDRAGONFLYDOJI"
            | "DRAGONFLYDOJI"
            | "DRAGONFLY_DOJI"
            | "DRAGONFLY"
            | "CDLDRAGONFLYDOJIWIN" => {
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
                    self.cdl_dragonfly_doji_win_symbol = sym;
                }
                self.show_cdl_dragonfly_doji_win = true;
                if self.cdl_dragonfly_doji_win_snapshot.symbol.is_empty()
                    && !self.cdl_dragonfly_doji_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_dragonfly_doji(
                                    &conn,
                                    &self.cdl_dragonfly_doji_win_symbol,
                                )
                            {
                                self.cdl_dragonfly_doji_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLGRAVESTONEDOJI"
            | "GRAVESTONEDOJI"
            | "GRAVESTONE_DOJI"
            | "GRAVESTONE"
            | "CDLGRAVESTONEDOJIWIN" => {
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
                    self.cdl_gravestone_doji_win_symbol = sym;
                }
                self.show_cdl_gravestone_doji_win = true;
                if self.cdl_gravestone_doji_win_snapshot.symbol.is_empty()
                    && !self.cdl_gravestone_doji_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_gravestone_doji(
                                    &conn,
                                    &self.cdl_gravestone_doji_win_symbol,
                                )
                            {
                                self.cdl_gravestone_doji_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLHANGINGMAN" | "HANGINGMAN" | "HANGING_MAN" | "CDLHANGINGMANWIN" | "HANGMAN" => {
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
                    self.cdl_hanging_man_win_symbol = sym;
                }
                self.show_cdl_hanging_man_win = true;
                if self.cdl_hanging_man_win_snapshot.symbol.is_empty()
                    && !self.cdl_hanging_man_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_hanging_man(
                                    &conn,
                                    &self.cdl_hanging_man_win_symbol,
                                )
                            {
                                self.cdl_hanging_man_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLINVERTEDHAMMER"
            | "INVERTEDHAMMER"
            | "INVERTED_HAMMER"
            | "INVHAMMER"
            | "CDLINVERTEDHAMMERWIN" => {
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
                    self.cdl_inverted_hammer_win_symbol = sym;
                }
                self.show_cdl_inverted_hammer_win = true;
                if self.cdl_inverted_hammer_win_snapshot.symbol.is_empty()
                    && !self.cdl_inverted_hammer_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_inverted_hammer(
                                    &conn,
                                    &self.cdl_inverted_hammer_win_symbol,
                                )
                            {
                                self.cdl_inverted_hammer_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-187 Round 75 — CDL* harami cross / long-legged doji / marubozu / spinning top / tristar ──
            "CDLHARAMICROSS"
            | "HARAMICROSS"
            | "HARAMI_CROSS"
            | "CDLHARAMICROSSWIN"
            | "HARAMI_CROSS_PATTERN" => {
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
                    self.cdl_harami_cross_win_symbol = sym;
                }
                self.show_cdl_harami_cross_win = true;
                if self.cdl_harami_cross_win_snapshot.symbol.is_empty()
                    && !self.cdl_harami_cross_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_harami_cross(
                                    &conn,
                                    &self.cdl_harami_cross_win_symbol,
                                )
                            {
                                self.cdl_harami_cross_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLLONGLEGGEDDOJI"
            | "LONGLEGGEDDOJI"
            | "LONG_LEGGED_DOJI"
            | "LONGLEGGED"
            | "CDLLONGLEGGEDDOJIWIN" => {
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
                    self.cdl_long_legged_doji_win_symbol = sym;
                }
                self.show_cdl_long_legged_doji_win = true;
                if self.cdl_long_legged_doji_win_snapshot.symbol.is_empty()
                    && !self.cdl_long_legged_doji_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_long_legged_doji(
                                    &conn,
                                    &self.cdl_long_legged_doji_win_symbol,
                                )
                            {
                                self.cdl_long_legged_doji_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLMARUBOZU" | "MARUBOZU" | "MARUBOZU_CANDLE" | "MARUBOZU_PATTERN"
            | "CDLMARUBOZUWIN" => {
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
                    self.cdl_marubozu_win_symbol = sym;
                }
                self.show_cdl_marubozu_win = true;
                if self.cdl_marubozu_win_snapshot.symbol.is_empty()
                    && !self.cdl_marubozu_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_marubozu(
                                &conn,
                                &self.cdl_marubozu_win_symbol,
                            ) {
                                self.cdl_marubozu_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLSPINNINGTOP"
            | "SPINNINGTOP"
            | "SPINNING_TOP"
            | "SPINNING_TOP_PATTERN"
            | "CDLSPINNINGTOPWIN" => {
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
                    self.cdl_spinning_top_win_symbol = sym;
                }
                self.show_cdl_spinning_top_win = true;
                if self.cdl_spinning_top_win_snapshot.symbol.is_empty()
                    && !self.cdl_spinning_top_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_spinning_top(
                                    &conn,
                                    &self.cdl_spinning_top_win_symbol,
                                )
                            {
                                self.cdl_spinning_top_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLTRISTAR" | "TRISTAR" | "TRI_STAR" | "TRIPLE_DOJI" | "CDLTRISTARWIN" => {
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
                    self.cdl_tristar_win_symbol = sym;
                }
                self.show_cdl_tristar_win = true;
                if self.cdl_tristar_win_snapshot.symbol.is_empty()
                    && !self.cdl_tristar_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_tristar(
                                &conn,
                                &self.cdl_tristar_win_symbol,
                            ) {
                                self.cdl_tristar_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-191 Round 76 — CDL* doji star / morning doji star / evening doji star / abandoned baby / three inside ──
            "CDLDOJISTAR" | "DOJISTAR" | "DOJI_STAR" | "CDLDOJISTARWIN" | "DOJISTAR_PATTERN" => {
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
                    self.cdl_doji_star_win_symbol = sym;
                }
                self.show_cdl_doji_star_win = true;
                if self.cdl_doji_star_win_snapshot.symbol.is_empty()
                    && !self.cdl_doji_star_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_doji_star(
                                    &conn,
                                    &self.cdl_doji_star_win_symbol,
                                )
                            {
                                self.cdl_doji_star_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLMORNINGDOJISTAR"
            | "MORNINGDOJISTAR"
            | "MORNING_DOJI_STAR"
            | "CDLMORNINGDOJISTARWIN"
            | "MORNING_DOJI_STAR_PATTERN" => {
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
                    self.cdl_morning_doji_star_win_symbol = sym;
                }
                self.show_cdl_morning_doji_star_win = true;
                if self.cdl_morning_doji_star_win_snapshot.symbol.is_empty()
                    && !self.cdl_morning_doji_star_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_morning_doji_star(
                                    &conn,
                                    &self.cdl_morning_doji_star_win_symbol,
                                )
                            {
                                self.cdl_morning_doji_star_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLEVENINGDOJISTAR"
            | "EVENINGDOJISTAR"
            | "EVENING_DOJI_STAR"
            | "CDLEVENINGDOJISTARWIN"
            | "EVENING_DOJI_STAR_PATTERN" => {
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
                    self.cdl_evening_doji_star_win_symbol = sym;
                }
                self.show_cdl_evening_doji_star_win = true;
                if self.cdl_evening_doji_star_win_snapshot.symbol.is_empty()
                    && !self.cdl_evening_doji_star_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_evening_doji_star(
                                    &conn,
                                    &self.cdl_evening_doji_star_win_symbol,
                                )
                            {
                                self.cdl_evening_doji_star_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLABANDONEDBABY"
            | "ABANDONEDBABY"
            | "ABANDONED_BABY"
            | "CDLABANDONEDBABYWIN"
            | "ABANDONED_BABY_PATTERN" => {
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
                    self.cdl_abandoned_baby_win_symbol = sym;
                }
                self.show_cdl_abandoned_baby_win = true;
                if self.cdl_abandoned_baby_win_snapshot.symbol.is_empty()
                    && !self.cdl_abandoned_baby_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_abandoned_baby(
                                    &conn,
                                    &self.cdl_abandoned_baby_win_symbol,
                                )
                            {
                                self.cdl_abandoned_baby_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDL3INSIDE"
            | "THREEINSIDE"
            | "THREE_INSIDE"
            | "CDL3INSIDEWIN"
            | "THREE_INSIDE_PATTERN" => {
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
                    self.cdl_three_inside_win_symbol = sym;
                }
                self.show_cdl_three_inside_win = true;
                if self.cdl_three_inside_win_snapshot.symbol.is_empty()
                    && !self.cdl_three_inside_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_three_inside(
                                    &conn,
                                    &self.cdl_three_inside_win_symbol,
                                )
                            {
                                self.cdl_three_inside_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-192 Round 77 — CDL* belt hold / closing marubozu / high wave / long line / short line ──
            "CDLBELTHOLD" | "BELTHOLD" | "BELT_HOLD" | "CDLBELTHOLDWIN" | "BELT_HOLD_PATTERN" => {
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
                    self.cdl_belt_hold_win_symbol = sym;
                }
                self.show_cdl_belt_hold_win = true;
                if self.cdl_belt_hold_win_snapshot.symbol.is_empty()
                    && !self.cdl_belt_hold_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_belt_hold(
                                    &conn,
                                    &self.cdl_belt_hold_win_symbol,
                                )
                            {
                                self.cdl_belt_hold_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLCLOSINGMARUBOZU"
            | "CLOSINGMARUBOZU"
            | "CLOSING_MARUBOZU"
            | "CDLCLOSINGMARUBOZUWIN"
            | "CLOSING_MARUBOZU_PATTERN" => {
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
                    self.cdl_closing_marubozu_win_symbol = sym;
                }
                self.show_cdl_closing_marubozu_win = true;
                if self.cdl_closing_marubozu_win_snapshot.symbol.is_empty()
                    && !self.cdl_closing_marubozu_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_closing_marubozu(
                                    &conn,
                                    &self.cdl_closing_marubozu_win_symbol,
                                )
                            {
                                self.cdl_closing_marubozu_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLHIGHWAVE" | "HIGHWAVE" | "HIGH_WAVE" | "CDLHIGHWAVEWIN" | "HIGH_WAVE_PATTERN" => {
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
                    self.cdl_high_wave_win_symbol = sym;
                }
                self.show_cdl_high_wave_win = true;
                if self.cdl_high_wave_win_snapshot.symbol.is_empty()
                    && !self.cdl_high_wave_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_high_wave(
                                    &conn,
                                    &self.cdl_high_wave_win_symbol,
                                )
                            {
                                self.cdl_high_wave_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLLONGLINE" | "LONGLINE" | "LONG_LINE" | "CDLLONGLINEWIN" | "LONG_LINE_PATTERN" => {
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
                    self.cdl_long_line_win_symbol = sym;
                }
                self.show_cdl_long_line_win = true;
                if self.cdl_long_line_win_snapshot.symbol.is_empty()
                    && !self.cdl_long_line_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_long_line(
                                    &conn,
                                    &self.cdl_long_line_win_symbol,
                                )
                            {
                                self.cdl_long_line_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLSHORTLINE" | "SHORTLINE" | "SHORT_LINE" | "CDLSHORTLINEWIN"
            | "SHORT_LINE_PATTERN" => {
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
                    self.cdl_short_line_win_symbol = sym;
                }
                self.show_cdl_short_line_win = true;
                if self.cdl_short_line_win_snapshot.symbol.is_empty()
                    && !self.cdl_short_line_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_short_line(
                                    &conn,
                                    &self.cdl_short_line_win_symbol,
                                )
                            {
                                self.cdl_short_line_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-193 Round 78 — CDL* counterattack / homing pigeon / in-neck / on-neck / thrusting ──
            "CDLCOUNTERATTACK"
            | "COUNTERATTACK"
            | "COUNTER_ATTACK"
            | "CDLCOUNTERATTACKWIN"
            | "COUNTERATTACK_PATTERN" => {
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
                    self.cdl_counterattack_win_symbol = sym;
                }
                self.show_cdl_counterattack_win = true;
                if self.cdl_counterattack_win_snapshot.symbol.is_empty()
                    && !self.cdl_counterattack_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_counterattack(
                                    &conn,
                                    &self.cdl_counterattack_win_symbol,
                                )
                            {
                                self.cdl_counterattack_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLHOMINGPIGEON"
            | "HOMINGPIGEON"
            | "HOMING_PIGEON"
            | "CDLHOMINGPIGEONWIN"
            | "HOMING_PIGEON_PATTERN" => {
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
                    self.cdl_homing_pigeon_win_symbol = sym;
                }
                self.show_cdl_homing_pigeon_win = true;
                if self.cdl_homing_pigeon_win_snapshot.symbol.is_empty()
                    && !self.cdl_homing_pigeon_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_homing_pigeon(
                                    &conn,
                                    &self.cdl_homing_pigeon_win_symbol,
                                )
                            {
                                self.cdl_homing_pigeon_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLINNECK" | "INNECK" | "IN_NECK" | "CDLINNECKWIN" | "IN_NECK_PATTERN" => {
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
                    self.cdl_in_neck_win_symbol = sym;
                }
                self.show_cdl_in_neck_win = true;
                if self.cdl_in_neck_win_snapshot.symbol.is_empty()
                    && !self.cdl_in_neck_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_in_neck(
                                &conn,
                                &self.cdl_in_neck_win_symbol,
                            ) {
                                self.cdl_in_neck_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLONNECK" | "ONNECK" | "ON_NECK" | "CDLONNECKWIN" | "ON_NECK_PATTERN" => {
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
                    self.cdl_on_neck_win_symbol = sym;
                }
                self.show_cdl_on_neck_win = true;
                if self.cdl_on_neck_win_snapshot.symbol.is_empty()
                    && !self.cdl_on_neck_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_on_neck(
                                &conn,
                                &self.cdl_on_neck_win_symbol,
                            ) {
                                self.cdl_on_neck_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLTHRUSTING" | "THRUSTING" | "THRUST" | "CDLTHRUSTINGWIN" | "THRUSTING_PATTERN" => {
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
                    self.cdl_thrusting_win_symbol = sym;
                }
                self.show_cdl_thrusting_win = true;
                if self.cdl_thrusting_win_snapshot.symbol.is_empty()
                    && !self.cdl_thrusting_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_thrusting(
                                    &conn,
                                    &self.cdl_thrusting_win_symbol,
                                )
                            {
                                self.cdl_thrusting_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDL2CROWS" | "TWOCROWS" | "TWO_CROWS" | "CDL2CROWSWIN" | "TWO_CROWS_PATTERN" => {
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
                    self.cdl_two_crows_win_symbol = sym;
                }
                self.show_cdl_two_crows_win = true;
                if self.cdl_two_crows_win_snapshot.symbol.is_empty()
                    && !self.cdl_two_crows_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_two_crows(
                                    &conn,
                                    &self.cdl_two_crows_win_symbol,
                                )
                            {
                                self.cdl_two_crows_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDL3LINESTRIKE" | "THREELINESTRIKE" | "THREE_LINE_STRIKE" | "CDL3LINESTRIKEWIN"
            | "LINE_STRIKE" => {
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
                    self.cdl_three_line_strike_win_symbol = sym;
                }
                self.show_cdl_three_line_strike_win = true;
                if self.cdl_three_line_strike_win_snapshot.symbol.is_empty()
                    && !self.cdl_three_line_strike_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_three_line_strike(
                                    &conn,
                                    &self.cdl_three_line_strike_win_symbol,
                                )
                            {
                                self.cdl_three_line_strike_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDL3OUTSIDE" | "THREEOUTSIDE" | "THREE_OUTSIDE" | "CDL3OUTSIDEWIN" | "OUTSIDE3" => {
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
                    self.cdl_three_outside_win_symbol = sym;
                }
                self.show_cdl_three_outside_win = true;
                if self.cdl_three_outside_win_snapshot.symbol.is_empty()
                    && !self.cdl_three_outside_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_three_outside(
                                    &conn,
                                    &self.cdl_three_outside_win_symbol,
                                )
                            {
                                self.cdl_three_outside_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLMATCHINGLOW" | "MATCHINGLOW" | "MATCHING_LOW" | "CDLMATCHINGLOWWIN"
            | "MATCH_LOW" => {
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
                    self.cdl_matching_low_win_symbol = sym;
                }
                self.show_cdl_matching_low_win = true;
                if self.cdl_matching_low_win_snapshot.symbol.is_empty()
                    && !self.cdl_matching_low_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_matching_low(
                                    &conn,
                                    &self.cdl_matching_low_win_symbol,
                                )
                            {
                                self.cdl_matching_low_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLSEPARATINGLINES"
            | "SEPARATINGLINES"
            | "SEPARATING_LINES"
            | "CDLSEPARATINGLINESWIN"
            | "SEP_LINES" => {
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
                    self.cdl_separating_lines_win_symbol = sym;
                }
                self.show_cdl_separating_lines_win = true;
                if self.cdl_separating_lines_win_snapshot.symbol.is_empty()
                    && !self.cdl_separating_lines_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_separating_lines(
                                    &conn,
                                    &self.cdl_separating_lines_win_symbol,
                                )
                            {
                                self.cdl_separating_lines_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLSTICKSANDWICH"
            | "STICKSANDWICH"
            | "STICK_SANDWICH"
            | "CDLSTICKSANDWICHWIN"
            | "SANDWICH" => {
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
                    self.cdl_stick_sandwich_win_symbol = sym;
                }
                self.show_cdl_stick_sandwich_win = true;
                if self.cdl_stick_sandwich_win_snapshot.symbol.is_empty()
                    && !self.cdl_stick_sandwich_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_stick_sandwich(
                                    &conn,
                                    &self.cdl_stick_sandwich_win_symbol,
                                )
                            {
                                self.cdl_stick_sandwich_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLRICKSHAWMAN" | "RICKSHAWMAN" | "RICKSHAW_MAN" | "CDLRICKSHAWMANWIN"
            | "RICKSHAW" => {
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
                    self.cdl_rickshaw_man_win_symbol = sym;
                }
                self.show_cdl_rickshaw_man_win = true;
                if self.cdl_rickshaw_man_win_snapshot.symbol.is_empty()
                    && !self.cdl_rickshaw_man_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_rickshaw_man(
                                    &conn,
                                    &self.cdl_rickshaw_man_win_symbol,
                                )
                            {
                                self.cdl_rickshaw_man_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLTAKURI" | "TAKURI" | "CDLTAKURIWIN" | "TAKURI_CANDLE" => {
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
                    self.cdl_takuri_win_symbol = sym;
                }
                self.show_cdl_takuri_win = true;
                if self.cdl_takuri_win_snapshot.symbol.is_empty()
                    && !self.cdl_takuri_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_takuri(
                                &conn,
                                &self.cdl_takuri_win_symbol,
                            ) {
                                self.cdl_takuri_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDL3STARSINSOUTH"
            | "THREESTARSINSOUTH"
            | "THREE_STARS_IN_SOUTH"
            | "SOUTH_STARS"
            | "CDL3STARSINSOUTHWIN" => {
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
                    self.cdl_three_stars_in_south_win_symbol = sym;
                }
                self.show_cdl_three_stars_in_south_win = true;
                if self.cdl_three_stars_in_south_win_snapshot.symbol.is_empty()
                    && !self.cdl_three_stars_in_south_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_three_stars_in_south(
                                    &conn,
                                    &self.cdl_three_stars_in_south_win_symbol,
                                )
                            {
                                self.cdl_three_stars_in_south_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLIDENTICAL3CROWS"
            | "IDENTICAL3CROWS"
            | "IDENTICAL_THREE_CROWS"
            | "THREE_IDENTICAL_CROWS"
            | "CDLIDENTICAL3CROWSWIN" => {
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
                    self.cdl_identical_three_crows_win_symbol = sym;
                }
                self.show_cdl_identical_three_crows_win = true;
                if self
                    .cdl_identical_three_crows_win_snapshot
                    .symbol
                    .is_empty()
                    && !self.cdl_identical_three_crows_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_identical_three_crows(
                                    &conn,
                                    &self.cdl_identical_three_crows_win_symbol,
                                )
                            {
                                self.cdl_identical_three_crows_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLKICKING" | "KICKING" | "CDLKICKINGWIN" | "KICKING_CANDLE" => {
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
                    self.cdl_kicking_win_symbol = sym;
                }
                self.show_cdl_kicking_win = true;
                if self.cdl_kicking_win_snapshot.symbol.is_empty()
                    && !self.cdl_kicking_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_kicking(
                                &conn,
                                &self.cdl_kicking_win_symbol,
                            ) {
                                self.cdl_kicking_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLKICKINGBYLENGTH"
            | "KICKINGBYLENGTH"
            | "KICKING_BY_LENGTH"
            | "CDLKICKINGBYLENGTHWIN"
            | "KICKING_LENGTH" => {
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
                    self.cdl_kicking_by_length_win_symbol = sym;
                }
                self.show_cdl_kicking_by_length_win = true;
                if self.cdl_kicking_by_length_win_snapshot.symbol.is_empty()
                    && !self.cdl_kicking_by_length_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_kicking_by_length(
                                    &conn,
                                    &self.cdl_kicking_by_length_win_symbol,
                                )
                            {
                                self.cdl_kicking_by_length_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLLADDERBOTTOM" | "LADDERBOTTOM" | "LADDER_BOTTOM" | "BOTTOM_LADDER"
            | "CDLLADDERBOTTOMWIN" => {
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
                    self.cdl_ladder_bottom_win_symbol = sym;
                }
                self.show_cdl_ladder_bottom_win = true;
                if self.cdl_ladder_bottom_win_snapshot.symbol.is_empty()
                    && !self.cdl_ladder_bottom_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_ladder_bottom(
                                    &conn,
                                    &self.cdl_ladder_bottom_win_symbol,
                                )
                            {
                                self.cdl_ladder_bottom_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLUNIQUE3RIVER" | "UNIQUE3RIVER" | "UNIQUE_THREE_RIVER" | "THREE_RIVER"
            | "CDLUNIQUE3RIVERWIN" => {
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
                    self.cdl_unique_three_river_win_symbol = sym;
                }
                self.show_cdl_unique_three_river_win = true;
                if self.cdl_unique_three_river_win_snapshot.symbol.is_empty()
                    && !self.cdl_unique_three_river_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_unique_three_river(
                                    &conn,
                                    &self.cdl_unique_three_river_win_symbol,
                                )
                            {
                                self.cdl_unique_three_river_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLADVANCEBLOCK" | "ADVANCEBLOCK" | "ADVANCE_BLOCK" | "CDLADVANCEBLOCKWIN"
            | "ADV_BLOCK" => {
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
                    self.cdl_advance_block_win_symbol = sym;
                }
                self.show_cdl_advance_block_win = true;
                if self.cdl_advance_block_win_snapshot.symbol.is_empty()
                    && !self.cdl_advance_block_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_advance_block(
                                    &conn,
                                    &self.cdl_advance_block_win_symbol,
                                )
                            {
                                self.cdl_advance_block_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLBREAKAWAY" | "BREAKAWAY" | "CDLBREAKAWAYWIN" | "BREAK_AWAY" => {
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
                    self.cdl_breakaway_win_symbol = sym;
                }
                self.show_cdl_breakaway_win = true;
                if self.cdl_breakaway_win_snapshot.symbol.is_empty()
                    && !self.cdl_breakaway_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_breakaway(
                                    &conn,
                                    &self.cdl_breakaway_win_symbol,
                                )
                            {
                                self.cdl_breakaway_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLGAPSIDESIDEWHITE"
            | "GAPSIDESIDEWHITE"
            | "GAP_SIDE_SIDE_WHITE"
            | "CDLGAPSIDESIDEWHITEWIN"
            | "SIDE_BY_SIDE_WHITE" => {
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
                    self.cdl_gap_side_side_white_win_symbol = sym;
                }
                self.show_cdl_gap_side_side_white_win = true;
                if self.cdl_gap_side_side_white_win_snapshot.symbol.is_empty()
                    && !self.cdl_gap_side_side_white_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_gap_side_side_white(
                                    &conn,
                                    &self.cdl_gap_side_side_white_win_symbol,
                                )
                            {
                                self.cdl_gap_side_side_white_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLUPSIDEGAP2CROWS"
            | "UPSIDEGAP2CROWS"
            | "UPSIDE_GAP_TWO_CROWS"
            | "CDLUPSIDEGAP2CROWSWIN"
            | "GAP2CROWS" => {
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
                    self.cdl_upside_gap_two_crows_win_symbol = sym;
                }
                self.show_cdl_upside_gap_two_crows_win = true;
                if self.cdl_upside_gap_two_crows_win_snapshot.symbol.is_empty()
                    && !self.cdl_upside_gap_two_crows_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_upside_gap_two_crows(
                                    &conn,
                                    &self.cdl_upside_gap_two_crows_win_symbol,
                                )
                            {
                                self.cdl_upside_gap_two_crows_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLXSIDEGAP3METHODS"
            | "XSIDEGAP3METHODS"
            | "XSIDE_GAP_THREE_METHODS"
            | "CDLXSIDEGAP3METHODSWIN"
            | "GAP3METHODS" => {
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
                    self.cdl_xside_gap_three_methods_win_symbol = sym;
                }
                self.show_cdl_xside_gap_three_methods_win = true;
                if self
                    .cdl_xside_gap_three_methods_win_snapshot
                    .symbol
                    .is_empty()
                    && !self.cdl_xside_gap_three_methods_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_xside_gap_three_methods(
                                    &conn,
                                    &self.cdl_xside_gap_three_methods_win_symbol,
                                )
                            {
                                self.cdl_xside_gap_three_methods_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLCONCEALBABYSWALL"
            | "CONCEALBABYSWALL"
            | "CONCEAL_BABY_SWALLOW"
            | "CDLCONCEALBABYSWALLWIN"
            | "BABY_SWALLOW" => {
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
                    self.cdl_conceal_baby_swallow_win_symbol = sym;
                }
                self.show_cdl_conceal_baby_swallow_win = true;
                if self.cdl_conceal_baby_swallow_win_snapshot.symbol.is_empty()
                    && !self.cdl_conceal_baby_swallow_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_conceal_baby_swallow(
                                    &conn,
                                    &self.cdl_conceal_baby_swallow_win_symbol,
                                )
                            {
                                self.cdl_conceal_baby_swallow_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLHIKKAKE" | "HIKKAKE" | "HIKKAKEWIN" => {
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
                    self.cdl_hikkake_win_symbol = sym;
                }
                self.show_cdl_hikkake_win = true;
                if self.cdl_hikkake_win_snapshot.symbol.is_empty()
                    && !self.cdl_hikkake_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_hikkake(
                                &conn,
                                &self.cdl_hikkake_win_symbol,
                            ) {
                                self.cdl_hikkake_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLHIKKAKEMOD" | "HIKKAKEMOD" | "MODHIKKAKE" | "HIKKAKEMODWIN" => {
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
                    self.cdl_hikkake_mod_win_symbol = sym;
                }
                self.show_cdl_hikkake_mod_win = true;
                if self.cdl_hikkake_mod_win_snapshot.symbol.is_empty()
                    && !self.cdl_hikkake_mod_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_hikkake_mod(
                                    &conn,
                                    &self.cdl_hikkake_mod_win_symbol,
                                )
                            {
                                self.cdl_hikkake_mod_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLMATHOLD" | "MATHOLD" | "MAT_HOLD" | "MATHOLDWIN" => {
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
                    self.cdl_mat_hold_win_symbol = sym;
                }
                self.show_cdl_mat_hold_win = true;
                if self.cdl_mat_hold_win_snapshot.symbol.is_empty()
                    && !self.cdl_mat_hold_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_mat_hold(
                                &conn,
                                &self.cdl_mat_hold_win_symbol,
                            ) {
                                self.cdl_mat_hold_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLRISEFALL3METHODS"
            | "RISEFALL3METHODS"
            | "RISE_FALL_THREE_METHODS"
            | "CDLRISEFALL3METHODSWIN" => {
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
                    self.cdl_rise_fall_three_methods_win_symbol = sym;
                }
                self.show_cdl_rise_fall_three_methods_win = true;
                if self
                    .cdl_rise_fall_three_methods_win_snapshot
                    .symbol
                    .is_empty()
                    && !self.cdl_rise_fall_three_methods_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_rise_fall_three_methods(
                                    &conn,
                                    &self.cdl_rise_fall_three_methods_win_symbol,
                                )
                            {
                                self.cdl_rise_fall_three_methods_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLSTALLEDPATTERN"
            | "STALLEDPATTERN"
            | "STALLED_PATTERN"
            | "STALLPATTERN"
            | "CDLSTALLEDPATTERNWIN" => {
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
                    self.cdl_stalled_pattern_win_symbol = sym;
                }
                self.show_cdl_stalled_pattern_win = true;
                if self.cdl_stalled_pattern_win_snapshot.symbol.is_empty()
                    && !self.cdl_stalled_pattern_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_stalled_pattern(
                                    &conn,
                                    &self.cdl_stalled_pattern_win_symbol,
                                )
                            {
                                self.cdl_stalled_pattern_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLTASUKIGAP" | "TASUKIGAP" | "TASUKI_GAP" | "CDLTASUKIGAPWIN" => {
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
                    self.cdl_tasuki_gap_win_symbol = sym;
                }
                self.show_cdl_tasuki_gap_win = true;
                if self.cdl_tasuki_gap_win_snapshot.symbol.is_empty()
                    && !self.cdl_tasuki_gap_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_tasuki_gap(
                                    &conn,
                                    &self.cdl_tasuki_gap_win_symbol,
                                )
                            {
                                self.cdl_tasuki_gap_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-189 Round 76 Quant Stats aliases ──
            "MODSHARPE" | "ADJSHARPE" | "ADJUSTED_SHARPE" | "PEZIER_WHITE" | "MODSHARPEWIN" => {
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
                    self.modsharpe_win_symbol = sym;
                }
                self.show_modsharpe_win = true;
                if self.modsharpe_win_snapshot.symbol.is_empty()
                    && !self.modsharpe_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_modsharpe(
                                &conn,
                                &self.modsharpe_win_symbol,
                            ) {
                                self.modsharpe_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HSIEHTEST" | "HSIEH" | "HSIEH_NONLIN" | "NONLIN_3RDMOM" | "HSIEHTESTWIN" => {
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
                    self.hsiehtest_win_symbol = sym;
                }
                self.show_hsiehtest_win = true;
                if self.hsiehtest_win_snapshot.symbol.is_empty()
                    && !self.hsiehtest_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_hsiehtest(
                                &conn,
                                &self.hsiehtest_win_symbol,
                            ) {
                                self.hsiehtest_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CHOWBREAK" | "CHOW" | "CHOW_TEST" | "STRUCT_BREAK" | "CHOWBREAKWIN" => {
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
                    self.chowbreak_win_symbol = sym;
                }
                self.show_chowbreak_win = true;
                if self.chowbreak_win_snapshot.symbol.is_empty()
                    && !self.chowbreak_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_chowbreak(
                                &conn,
                                &self.chowbreak_win_symbol,
                            ) {
                                self.chowbreak_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DRIFTBURST" | "DRIFT_BURST" | "COR18" | "KERNEL_DRIFT" | "DRIFTBURSTWIN" => {
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
                    self.driftburst_win_symbol = sym;
                }
                self.show_driftburst_win = true;
                if self.driftburst_win_snapshot.symbol.is_empty()
                    && !self.driftburst_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_driftburst(
                                &conn,
                                &self.driftburst_win_symbol,
                            ) {
                                self.driftburst_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HLVCLUST" | "PARKINSON_CLUST" | "HL_CLUSTER" | "HL_VOLCLUST" | "HLVCLUSTWIN" => {
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
                    self.hlvclust_win_symbol = sym;
                }
                self.show_hlvclust_win = true;
                if self.hlvclust_win_snapshot.symbol.is_empty()
                    && !self.hlvclust_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_hlvclust(
                                &conn,
                                &self.hlvclust_win_symbol,
                            ) {
                                self.hlvclust_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-190 Round 77 palette aliases (Quant Stats) ──
            "YANGZHANG" | "YZ_VOL" | "YZVOL" | "YZ_RANGEVOL" | "YANGZHANGWIN" => {
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
                    self.yangzhang_win_symbol = sym;
                }
                self.show_yangzhang_win = true;
                if self.yangzhang_win_snapshot.symbol.is_empty()
                    && !self.yangzhang_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_yangzhang(
                                &conn,
                                &self.yangzhang_win_symbol,
                            ) {
                                self.yangzhang_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "KUIPER" | "KUIPERTEST" | "KUIPER_CDF" | "VSTAT" | "KUIPERWIN" => {
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
                    self.kuiper_win_symbol = sym;
                }
                self.show_kuiper_win = true;
                if self.kuiper_win_snapshot.symbol.is_empty() && !self.kuiper_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_kuiper(
                                &conn,
                                &self.kuiper_win_symbol,
                            ) {
                                self.kuiper_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DAGOSTINO" | "K2TEST" | "K2_OMNIBUS" | "DAGOSTINOPEARSON" | "DAGOSTINOWIN" => {
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
                    self.dagostino_win_symbol = sym;
                }
                self.show_dagostino_win = true;
                if self.dagostino_win_snapshot.symbol.is_empty()
                    && !self.dagostino_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_dagostino(
                                &conn,
                                &self.dagostino_win_symbol,
                            ) {
                                self.dagostino_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "BAIPERRON" | "SUPF" | "SUP_F" | "BAI_PERRON" | "BAIPERRONWIN" => {
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
                    self.baiperron_win_symbol = sym;
                }
                self.show_baiperron_win = true;
                if self.baiperron_win_snapshot.symbol.is_empty()
                    && !self.baiperron_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_baiperron(
                                &conn,
                                &self.baiperron_win_symbol,
                            ) {
                                self.baiperron_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "KUPIECPOF" | "KUPIEC" | "VAR_BACKTEST" | "POFTEST" | "KUPIECPOFWIN" => {
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
                    self.kupiecpof_win_symbol = sym;
                }
                self.show_kupiecpof_win = true;
                if self.kupiecpof_win_snapshot.symbol.is_empty()
                    && !self.kupiecpof_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_kupiecpof(
                                &conn,
                                &self.kupiecpof_win_symbol,
                            ) {
                                self.kupiecpof_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── ADR-130 web article ingestion + packet viewer ──
            "INGEST_RESEARCH" | "INGEST" | "RESEARCH_INGEST" | "INGESTRESEARCH" => {
                self.show_ingest_research = true;
            }
            "RESEARCH_PACKET"
            | "PACKET"
            | "PACKET_VIEW"
            | "VIEW_PACKET"
            | "RESEARCH_PACKET_VIEW" => {
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
                if !sym.is_empty() && self.packet_viewer_symbol.is_empty() {
                    self.packet_viewer_symbol = sym;
                }
                self.show_packet_viewer = true;
            }
            "CALENDAR" => {
                self.show_calendar = true;
                // Also show the econ calendar panel and fetch if empty (absorbed ECON_CALENDAR).
                self.show_econ_calendar = true;
                if self.econ_events.is_empty() {
                    let _ = self.broker_tx.send(BrokerCmd::FetchEconCalendar {
                        finnhub_key: self.finnhub_key.clone(),
                    });
                }
            }
            "SEC" => self.show_sec = true,
            "INSIDER" => self.show_insider = true,
            "FUNDAMENTALS" => self.show_fundamentals = true,
            "EV" => self.show_ev_scanner = true,
            "EARNINGS" => self.show_earnings_calendar = true,
            "DIVIDENDS" => self.show_dividend_calendar = true,
            s if s == "EVSCRAPE" || s == "EVSCRAPE FORCE" => {
                let force = s.ends_with("FORCE");
                let db_path = cache_db_path();
                // Broker scope override: narrow sources to just the scoped broker.
                // SCOPE ALL → use configured source toggles. SCOPE ALPACA → force use_alpaca only, etc.
                let (use_mt5, use_alpaca, use_tasty, use_kraken) = match self.broker_scope {
                    EventSource::All => (
                        self.fund_source_mt5,
                        self.fund_source_alpaca,
                        self.fund_source_tastytrade,
                        self.fund_source_kraken,
                    ),
                    EventSource::Alpaca => (false, true, false, false),
                    EventSource::Darwinex => (true, false, false, false),
                    EventSource::Tasty => (false, false, true, false),
                    EventSource::Kraken => (false, false, false, true),
                    EventSource::Positions => (
                        self.fund_source_mt5,
                        self.fund_source_alpaca,
                        self.fund_source_tastytrade,
                        self.fund_source_kraken,
                    ),
                };
                let _ = self.broker_tx.send(BrokerCmd::FundamentalsScrape {
                    db_path,
                    use_mt5,
                    use_alpaca,
                    use_tastytrade: use_tasty,
                    use_kraken,
                    force,
                });
                self.scrape_fund_running = true;
                self.scrape_fund_ok = 0;
                self.scrape_fund_fail = 0;
                self.scrape_fund_skipped = 0;
                let sources: Vec<&str> = [
                    ("MT5", use_mt5),
                    ("Alpaca", use_alpaca),
                    ("TastyTrade", use_tasty),
                    ("Kraken", use_kraken),
                ]
                .iter()
                .filter(|(_, on)| *on)
                .map(|(n, _)| *n)
                .collect();
                let force_label = if force {
                    " (FORCE — ignoring cache)"
                } else {
                    ""
                };
                self.log.push_back(LogEntry::info(format!(
                    "Fundamentals scrape started [{}] sources: {}{}...",
                    self.broker_scope_label(),
                    sources.join(", "),
                    force_label
                )));
            }
            "MT5SYNC" => {
                let paths: Vec<String> = self
                    .mt5_db_paths
                    .iter()
                    .filter(|p| !p.is_empty() && std::path::Path::new(p.as_str()).exists())
                    .cloned()
                    .collect();
                if paths.is_empty() {
                    self.log.push_back(LogEntry::warn(
                        "No valid MT5 database paths configured — set them in Settings",
                    ));
                } else {
                    let _ = self.broker_tx.send(BrokerCmd::Mt5Sync {
                        sources: paths.clone(),
                        enabled_timeframes: self.enabled_standard_sync_timeframes(),
                    });
                    self.log.push_back(LogEntry::info(format!(
                        "MT5 sync started ({} sources)...",
                        paths.len()
                    )));
                }
            }
            "ANALYST" => {
                self.show_analyst = true;
                // Also fetch price targets (absorbed PRICE_TARGET command).
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
                if !sym.is_empty() && !self.finnhub_key.is_empty() {
                    let _ = self.broker_tx.send(BrokerCmd::GetPriceTarget {
                        symbol: sym,
                        finnhub_key: self.finnhub_key.clone(),
                    });
                }
            }
            "SHORT_INTEREST" => {
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
                if !sym.is_empty() && !self.finnhub_key.is_empty() {
                    let _ = self.broker_tx.send(BrokerCmd::GetShortInterest {
                        symbol: sym,
                        finnhub_key: self.finnhub_key.clone(),
                    });
                }
            }
            "CORPORATE" => {
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
                    let _ = self
                        .broker_tx
                        .send(BrokerCmd::GetCorporateActions { symbol: sym });
                }
            }
            "MOST_ACTIVE" => {
                if self.broker_connected {
                    let _ = self.broker_tx.send(BrokerCmd::GetMostActive);
                    self.log
                        .push_back(LogEntry::info("Fetching most active symbols..."));
                } else {
                    self.log
                        .push_back(LogEntry::warn("Connect to broker first"));
                }
            }
            "PORTFOLIO_HIST" => {
                if self.broker_connected {
                    let _ = self.broker_tx.send(BrokerCmd::GetPortfolioHistory {
                        period: "1M".into(),
                    });
                    self.log
                        .push_back(LogEntry::info("Fetching portfolio equity history (1M)..."));
                } else {
                    self.log
                        .push_back(LogEntry::warn("Connect to broker first"));
                }
            }
            "WATCHLISTS" => {
                if self.broker_connected {
                    let _ = self.broker_tx.send(BrokerCmd::GetWatchlists);
                } else {
                    self.log
                        .push_back(LogEntry::warn("Connect to broker first"));
                }
            }
            "OPTIONS" => {
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
                    // Fetch from both brokers — Alpaca equity options + tastytrade
                    let expiry = chrono::Utc::now().format("%Y-%m-%d").to_string();
                    let _ = self.broker_tx.send(BrokerCmd::GetOptionsChain {
                        symbol: sym.clone(),
                        expiry,
                    });
                    self.option_chain_sym = sym.clone();
                    let _ = self
                        .broker_tx
                        .send(BrokerCmd::TastytradeOptionChain { symbol: sym });
                    self.show_option_chain = true;
                    self.log.push_back(LogEntry::info(format!(
                        "Fetching option chain for {}...",
                        self.option_chain_sym
                    )));
                }
            }
            "HOLDERS" => self.show_holders = true,
            "COMPILE" => self.show_indicator_compiler = true,
            "STREAM" => {
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
                if !sym.is_empty() && !self.stream_active {
                    let _ = self.broker_tx.send(BrokerCmd::StartStream {
                        trade_symbols: vec![sym.clone()],
                        quote_symbols: vec![sym.clone()],
                    });
                    self.stream_active = true;
                    self.log
                        .push_back(LogEntry::info(format!("Starting stream for {}", sym)));
                }
            }
            "DXLINK_STREAM" => {
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
                    let _ = self.broker_tx.send(BrokerCmd::StartDxLinkStream {
                        symbols: vec![sym.clone()],
                    });
                    self.log.push_back(LogEntry::info(format!(
                        "Starting DXLink stream for {}",
                        sym
                    )));
                }
            }
            "CORRELATION" => self.show_correlation = true,
            "SEASONALS" => self.show_seasonals = true,
            "MONTECARLO" => self.show_montecarlo = true,
            "STRESS_TEST" => self.show_stress_test = true,
            "VOLUME_PROFILE" => self.show_volume_profile = true,
            "HV_CONE" => self.show_hv_cone = true,
            "SECTOR_HEATMAP" => self.show_sector_heatmap = true,
            "DIVSCREEN" => self.show_dividends = true,
            s if s.starts_with("SCOPE") => {
                // SCOPE [ALL|ALPACA|DARWINEX|TASTY|KRAKEN] — global broker filter for fundamentals.
                let arg = s.trim_start_matches("SCOPE").trim();
                let (new_scope, label) = match arg {
                    "" => {
                        // No arg: open SCOPE popup window
                        self.show_scope_window = true;
                        return;
                    }
                    "ALL" => (EventSource::All, "ALL"),
                    "ALPACA" => (EventSource::Alpaca, "ALPACA"),
                    "DARWINEX" | "DARWIN" => (EventSource::Darwinex, "DARWINEX"),
                    "TASTY" | "TASTYTRADE" => (EventSource::Tasty, "TASTY"),
                    "KRAKEN" | "KR" => (EventSource::Kraken, "KRAKEN"),
                    "POSITIONS" | "POS" => (EventSource::Positions, "POSITIONS"),
                    other => {
                        self.log.push_back(LogEntry::err(format!(
                            "Unknown SCOPE '{other}'. Valid: ALL, ALPACA, DARWINEX, TASTY, KRAKEN, POSITIONS"
                        )));
                        return;
                    }
                };
                self.broker_scope = new_scope;
                // Sync fund_source toggles with scope
                match new_scope {
                    EventSource::All => {
                        self.fund_source_mt5 = true;
                        self.fund_source_alpaca = true;
                        self.fund_source_tastytrade = true;
                        self.fund_source_kraken = true;
                    }
                    EventSource::Alpaca => {
                        self.fund_source_mt5 = false;
                        self.fund_source_alpaca = true;
                        self.fund_source_tastytrade = false;
                        self.fund_source_kraken = false;
                    }
                    EventSource::Darwinex => {
                        self.fund_source_mt5 = true;
                        self.fund_source_alpaca = false;
                        self.fund_source_tastytrade = false;
                        self.fund_source_kraken = false;
                    }
                    EventSource::Tasty => {
                        self.fund_source_mt5 = false;
                        self.fund_source_alpaca = false;
                        self.fund_source_tastytrade = true;
                        self.fund_source_kraken = false;
                    }
                    EventSource::Kraken => {
                        self.fund_source_mt5 = false;
                        self.fund_source_alpaca = false;
                        self.fund_source_tastytrade = false;
                        self.fund_source_kraken = true;
                    }
                    EventSource::Positions => {
                        self.fund_source_mt5 = true;
                        self.fund_source_alpaca = true;
                        self.fund_source_tastytrade = true;
                        self.fund_source_kraken = true;
                    }
                }
                let n = self.scoped_fundamentals().len();
                self.log.push_back(LogEntry::info(format!(
                    "Broker scope → {label} ({} fundamentals in scope)",
                    n
                )));
            }
            "EVENTS" => {
                // Comprehensive upcoming events view for actively traded symbols.
                // Aggregates earnings / ex-dividend / dividend-payment dates from
                // fundamentals, tags each row by broker tradeability (Alpaca / Darwinex / Tasty).
                use chrono::NaiveDate;
                let today = chrono::Utc::now().date_naive();

                // Active symbol sets (bare tickers, uppercased).
                let alpaca_syms: std::collections::HashSet<String> = self
                    .live_positions
                    .iter()
                    .map(|p| p.symbol.replace('/', "").to_uppercase())
                    .collect();
                let tasty_syms: std::collections::HashSet<String> = self
                    .tt_positions
                    .iter()
                    .map(|p| p.symbol.replace('/', "").to_uppercase())
                    .collect();
                let kraken_syms = self.kraken_scope_symbols();
                // Darwinex: strip suffix (.US, .UK, .DE, etc.) from tradeable MT5 symbols.
                let darwinex_syms: std::collections::HashSet<String> = self
                    .darwinex_radar_data
                    .iter()
                    .filter(|(_, _, _, trade_mode, _, _, _, _, _)| *trade_mode != 0)
                    .map(|(sym, _, _, _, _, _, _, _, _)| {
                        sym.split('.').next().unwrap_or(sym.as_str()).to_uppercase()
                    })
                    .collect();

                let parse_date = |s: &str| -> Option<NaiveDate> {
                    NaiveDate::parse_from_str(s, "%Y-%m-%d")
                        .ok()
                        .or_else(|| NaiveDate::parse_from_str(s, "%Y/%m/%d").ok())
                };

                let mut rows: Vec<EventRow> = Vec::new();
                for f in &self.bg.all_fundamentals {
                    let sym_u = f.symbol.to_uppercase();
                    let in_alpaca = alpaca_syms.contains(&sym_u);
                    let in_tasty = tasty_syms.contains(&sym_u);
                    let in_darwinex = darwinex_syms.contains(&sym_u);
                    let in_kraken = kraken_syms.contains(&sym_u);
                    if !in_alpaca && !in_tasty && !in_darwinex && !in_kraken {
                        continue;
                    }

                    let mut push = |date_str: &str, kind: EventKind, detail: String| {
                        if let Some(d) = parse_date(date_str) {
                            let days = (d - today).num_days();
                            if days < 0 {
                                return;
                            } // only upcoming
                            rows.push(EventRow {
                                symbol: f.symbol.clone(),
                                company: f.company_name.clone(),
                                date: date_str.to_string(),
                                days_until: days,
                                kind,
                                detail,
                                in_alpaca,
                                in_darwinex,
                                in_tasty,
                                in_kraken,
                            });
                        }
                    };

                    if let Some(ref d) = f.next_earnings_date {
                        let detail = match f.pe_ratio {
                            Some(pe) => format!("P/E {:.1}", pe),
                            None => String::new(),
                        };
                        push(d, EventKind::Earnings, detail);
                    }
                    if let Some(ref d) = f.next_ex_dividend_date {
                        let detail = match f.dividend_yield {
                            Some(y) => format!("{:.2}% yield", y),
                            None => String::new(),
                        };
                        push(d, EventKind::ExDividend, detail);
                    }
                    if let Some(ref d) = f.next_dividend_payment_date {
                        let detail = match f.dividend_yield {
                            Some(y) => format!("{:.2}% yield", y),
                            None => String::new(),
                        };
                        push(d, EventKind::DividendPayment, detail);
                    }
                }

                // Sort by days_until ASC (most imminent first).
                rows.sort_by_key(|r| r.days_until);

                self.log.push_back(LogEntry::info(format!(
                    "Event Calendar: {} upcoming events | Alpaca {} • Darwinex {} • Tasty {}",
                    rows.len(),
                    alpaca_syms.len(),
                    darwinex_syms.len(),
                    tasty_syms.len()
                )));
                if rows.is_empty() {
                    self.log.push_back(LogEntry::warn("No events found. Run EVSCRAPE/FUNDAMENTALS first to populate earnings/dividend dates."));
                }
                self.event_calendar_rows = rows;
                self.show_event_calendar = true;
            }
            "CONFLUENCE" => self.show_confluence = true,
            "STAT_ARB" => self.show_stat_arb = true,
            "RISK_BUDGET" => self.show_risk_budget = true,
            "ORDER_FLOW" => self.show_order_flow = true,
            "BOOKMAP" => self.show_bookmap = true,
            "JOURNAL" => self.show_journal = true,
            "CACHE_STATS" => self.show_cache_stats = true,
            "SOURCES" => {
                // ADR-038 Phase 2: Show data source status
                self.data_sources.update_health();
                let summary = self.data_sources.status_summary();
                for (id, label, healthy, last_ts) in &summary {
                    let status = if *healthy { "HEALTHY" } else { "DOWN" };
                    let last = if *last_ts > 0 {
                        chrono::DateTime::from_timestamp(*last_ts, 0)
                            .map(|d| d.format("%H:%M:%S").to_string())
                            .unwrap_or_default()
                    } else {
                        "never".to_string()
                    };
                    let level = if *healthy {
                        LogLevel::Info
                    } else {
                        LogLevel::Warn
                    };
                    self.log.push_back(LogEntry::new(
                        level,
                        format!("[{}] {} — {} (last: {})", status, label, id, last),
                    ));
                }
                if !self.data_sources.overrides.is_empty() {
                    self.log.push_back(LogEntry::info(format!(
                        "Per-symbol overrides: {}",
                        self.data_sources.overrides.len()
                    )));
                    for ovr in &self.data_sources.overrides {
                        self.log.push_back(LogEntry::info(format!(
                            "  {} → {}",
                            ovr.pattern,
                            ovr.sources.join(", ")
                        )));
                    }
                }
                // Result card: Summary of sources
                let metrics: Vec<(String, String, egui::Color32)> = summary
                    .iter()
                    .map(|(_, label, healthy, _)| {
                        let status = if *healthy { "OK" } else { "DOWN" };
                        let color = if *healthy {
                            egui::Color32::from_rgb(80, 220, 120)
                        } else {
                            egui::Color32::from_rgb(255, 80, 80)
                        };
                        (label.clone(), status.to_string(), color)
                    })
                    .collect();
                self.result_card = Some((
                    ResultCard::Summary {
                        title: "Data Sources".to_string(),
                        metrics,
                    },
                    std::time::Instant::now(),
                ));
            }
            "STORAGE" => self.show_storage = true,
            "SYNC" | "SYNC_STATUS" | "SYNC_PCT" | "BARSYNC" | "BAR_SYNC" => {
                self.show_sync_status = true
            }
            "LAN_SYNC" => self.show_lan_sync = true,
            "UNUSUAL_VOLUME" => {
                self.show_unusual_volume = true;
                // Send scan to background thread (avoids blocking UI with DB reads)
                let keys: Vec<(String, i64)> = self
                    .bg
                    .detailed_stats
                    .iter()
                    .map(|(k, c, _)| (k.clone(), *c))
                    .collect();
                let _ = self.broker_tx.send(BrokerCmd::ScanUnusualVolume { keys });
                self.log
                    .push_back(LogEntry::info("Scanning unusual volume..."));
            }
            "SECTOR_ROTATION" => self.show_sector_rotation = true,
            "CONGRESS" => {
                self.show_congress = true;
                if self.congress_trades.is_empty() {
                    let _ = self.broker_tx.send(BrokerCmd::FetchCongressTrades);
                }
            }
            "HELP" => self.show_help = true,
            "FULLSCREEN" => ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true)),
            "CLOSE_WINDOWS" => self.close_all_windows(),
            "NEW_WINDOW" => {
                let exe = std::env::current_exe().unwrap_or_default();
                match std::process::Command::new(&exe).spawn() {
                    Ok(_) => self
                        .log
                        .push_back(LogEntry::info("New window launched (separate process)")),
                    Err(e) => self
                        .log
                        .push_back(LogEntry::err(format!("Failed to launch new window: {}", e))),
                }
            }
            // Chart types
            "CANDLE" => {
                if let Some(c) = self.charts.get_mut(self.active_tab) {
                    c.chart_type = ChartType::Candle;
                }
            }
            "HEIKINASHI" => {
                if let Some(c) = self.charts.get_mut(self.active_tab) {
                    c.chart_type = ChartType::HeikinAshi;
                }
            }
            "LINE" => {
                if let Some(c) = self.charts.get_mut(self.active_tab) {
                    c.chart_type = ChartType::Line;
                }
            }
            "OHLC" => {
                if let Some(c) = self.charts.get_mut(self.active_tab) {
                    c.chart_type = ChartType::OhlcBars;
                }
            }
            "RENKO" => {
                if let Some(c) = self.charts.get_mut(self.active_tab) {
                    c.chart_type = ChartType::Renko;
                }
            }
            "EXPORT_CSV" => {
                self.export_csv();
            }
            "SCREENSHOT" => {
                self.screenshot_requested = true;
                self.log.push_back(LogEntry::info(
                    "Screenshot requested — capturing next frame...",
                ));
            }
            "SHARE" | "SCREENSHOT_SHARE" => {
                if let Some(ref path) = self.last_screenshot_path {
                    if self.matrix_access_token.is_empty()
                        || self.matrix_access_token == "none"
                        || self.matrix_access_token == "pending"
                    {
                        self.log.push_back(LogEntry::warn(
                            "Matrix: no access token — set it in Settings first",
                        ));
                    } else if path.exists() {
                        let _ = self.broker_tx.send(BrokerCmd::MatrixSendImage {
                            room_id: self.matrix_room.clone(),
                            access_token: self.matrix_access_token.clone(),
                            file_path: path.clone(),
                        });
                        self.log.push_back(LogEntry::info(format!(
                            "Sharing screenshot to community chat: {}",
                            path.display()
                        )));
                    } else {
                        self.log.push_back(LogEntry::warn(
                            "Screenshot file not found — take a SCREENSHOT first",
                        ));
                    }
                } else {
                    self.log.push_back(LogEntry::warn(
                        "No screenshot taken yet — use SCREENSHOT first",
                    ));
                }
            }
            // Tabs
            "NEW_TAB" => {
                let tf = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| c.timeframe)
                    .unwrap_or(Timeframe::H4);
                let mut new_chart = ChartState::new(&self.symbol_input, tf);
                if let Some(ref cache) = self.cache.clone() {
                    {
                        let mut gpu = self.gpu_indicators.take();
                        new_chart.try_load(Arc::as_ref(cache), &mut self.log, gpu.as_mut());
                        self.gpu_indicators = gpu;
                    }
                }
                self.charts.push(new_chart);
                self.active_tab = self.charts.len() - 1;
            }
            "CLOSE_TAB" => {
                if self.charts.len() > 1 {
                    self.charts.remove(self.active_tab);
                    if self.active_tab >= self.charts.len() {
                        self.active_tab = self.charts.len() - 1;
                    }
                }
            }
            // DARWIN-specific
            "DARWINS" => self.show_darwin_portfolio = true,
            "DRAWDOWN" => {
                self.darwin_view = 0;
                self.show_darwin_portfolio = true;
            } // Portfolio Summary with per-DARWIN DD%
            "REBALANCE" => {
                self.darwin_view = 18;
                self.show_darwin_portfolio = true;
            } // Optimal Allocation view
            "DARWIN_TRADES" => {
                self.log.push_back(LogEntry::info(
                    "DARWIN trade markers: open DARWIN Accounts for deal history",
                ));
                self.show_darwin_accounts = true;
            }
            "POSITION_CHARTS" => {
                // Collect unique symbols from all enabled position sources
                let mut symbols: Vec<String> = Vec::new();
                let mut seen = std::collections::HashSet::new();
                if self.show_darwin_positions {
                    for pos in &self.bg.open_positions {
                        if seen.insert(pos.symbol.clone()) {
                            symbols.push(pos.symbol.clone());
                        }
                    }
                }
                if self.show_alpaca_positions {
                    for pos in &self.live_positions {
                        if seen.insert(pos.symbol.clone()) {
                            symbols.push(pos.symbol.clone());
                        }
                    }
                }
                if self.show_tt_positions {
                    for pos in &self.tt_positions {
                        if seen.insert(pos.symbol.clone()) {
                            symbols.push(pos.symbol.clone());
                        }
                    }
                }
                if self.show_kr_positions {
                    for pos in &self.kr_positions {
                        if seen.insert(pos.symbol.clone()) {
                            symbols.push(pos.symbol.clone());
                        }
                    }
                }
                if symbols.is_empty() {
                    self.log
                        .push_back(LogEntry::warn("No open positions to chart"));
                } else {
                    let count = symbols.len();
                    for sym in &symbols {
                        let mut chart = ChartState::new(sym, Timeframe::W1);
                        if let Some(ref cache) = self.cache {
                            let mut gpu = self.gpu_indicators.take();
                            chart.try_load(Arc::as_ref(cache), &mut self.log, gpu.as_mut());
                            self.gpu_indicators = gpu;
                        }
                        self.charts.push(chart);
                    }
                    self.active_tab = self.charts.len().saturating_sub(1);
                    self.symbol_input = symbols.last().cloned().unwrap_or_default();
                    self.log.push_back(LogEntry::info(format!(
                        "Opened {} W1 charts for open positions: {}",
                        count,
                        symbols.join(", ")
                    )));
                }
            }
            "DSCORE" => {
                self.show_var_mult = true;
            }
            "DARWIN_BROWSER" => {
                self.show_darwin_browser = true;
            }
            "SWAPHARVEST" => {
                if let Some(ref cache) = self.cache {
                    if let Some(conn) = cache.try_connection() {
                        match darwin::swap_harvest(&conn, 0.0) {
                            Ok(result) => {
                                self.log.push_back(LogEntry::info(format!(
                                    "SWAPHARVEST: {} symbols with positive swap ({} long, {} short, {} both) out of {} scanned",
                                    result.entries.len(), result.long_count, result.short_count, result.both_count, result.total_scanned
                                )));
                                self.swap_harvest_results = Some(result);
                                self.show_swap_harvest = true;
                            }
                            Err(e) => self
                                .log
                                .push_back(LogEntry::err(format!("SwapHarvest failed: {}", e))),
                        }
                    }
                }
            }
            "DARWINEXRADAR" => {
                if let Some(ref cache) = self.cache {
                    if let Some(conn) = cache.try_connection() {
                        // Load all specs for UI display
                        match darwin::load_all_specs_parsed(&conn) {
                            Ok(data) => {
                                self.log.push_back(LogEntry::info(format!(
                                    "DARWINEXRADAR: loaded {} symbols",
                                    data.len()
                                )));
                                self.darwinex_radar_data = data;
                                self.show_darwinex_radar = true;
                            }
                            Err(e) => self
                                .log
                                .push_back(LogEntry::err(format!("DARWINEXRADAR failed: {}", e))),
                        }
                        // Compute changelog (compares against previous snapshot)
                        match darwin::radar_changelog(&conn) {
                            Ok(changes) => {
                                if changes.is_empty() {
                                    self.log.push_back(LogEntry::info(
                                        "DARWINEXRADAR: no changes since last snapshot",
                                    ));
                                } else {
                                    self.log.push_back(LogEntry::info(format!(
                                        "DARWINEXRADAR: {} changes detected",
                                        changes.len()
                                    )));
                                }
                                self.darwinex_radar_changelog = changes;
                            }
                            Err(e) => self
                                .log
                                .push_back(LogEntry::warn(format!("Changelog: {}", e))),
                        }
                        // Export CSVs (terminal export dir + optional web-compatible radar dir)
                        let mut out = dirs_home();
                        out.push("export");
                        let _ = std::fs::create_dir_all(&out);
                        match darwin::export_radar_txt(&conn, &conn, &out.display().to_string()) {
                            Ok(msg) => self
                                .log
                                .push_back(LogEntry::info(format!("Radar exported: {}", msg))),
                            Err(e) => self
                                .log
                                .push_back(LogEntry::err(format!("Export failed: {}", e))),
                        }
                    }
                }
            }
            // ── Darwinex Zero Web Scraping (ADR-093) ────────────────
            "DWXLOGIN" => {
                use typhoon_engine::core::darwin_web;
                use typhoon_engine::core::keyring;
                let email = keyring::load(darwin_web::keys::DARWINEX_EMAIL)
                    .ok()
                    .flatten();
                let password = keyring::load(darwin_web::keys::DARWINEX_PASSWORD)
                    .ok()
                    .flatten();
                match (email, password) {
                    (Some(email), Some(password)) => {
                        if self.dwx_logged_in {
                            self.log.push_back(LogEntry::warn(
                                "Already logged in — use DWXLOGOUT first",
                            ));
                        } else {
                            self.log.push_back(LogEntry::info(
                                "Launching Chrome for Darwinex Zero login...",
                            ));
                            let (tx, rx) = std::sync::mpsc::channel();
                            self.dwx_rx = Some(rx);
                            let config = self.dwx_config.clone();
                            let cache_arc = self.cache.clone();
                            // Load cached cookies
                            let cached_cookies: Option<Vec<darwin_web::SerializableCookie>> =
                                cache_arc
                                    .as_ref()
                                    .and_then(|c| {
                                        c.get_kv(darwin_web::cache_keys::COOKIES).ok().flatten()
                                    })
                                    .and_then(|json| serde_json::from_str(&json).ok());
                            let rt = self.rt_handle.clone();
                            let _ = std::thread::Builder::new()
                                .name("typhoon-dwx-login".into())
                                .spawn(move || {
                                    rt.block_on(async {
                                        match darwin_web::launch_browser().await {
                                            Ok(driver) => {
                                                let cache_fn =
                                                    |key: &str, val: &str| -> Result<(), String> {
                                                        if let Some(ref c) = cache_arc {
                                                            c.put_kv(key, val)
                                                        } else {
                                                            Ok(())
                                                        }
                                                    };
                                                let result = darwin_web::login_and_scrape(
                                                    &driver,
                                                    &email,
                                                    &password,
                                                    cached_cookies.as_deref(),
                                                    &config,
                                                    cache_fn,
                                                )
                                                .await;
                                                // Save cookies for next time
                                                if let Ok(cookies) =
                                                    darwin_web::get_cookies(&driver).await
                                                {
                                                    if let Some(ref c) = cache_arc {
                                                        if let Ok(json) =
                                                            serde_json::to_string(&cookies)
                                                        {
                                                            let _ = c.put_kv(
                                                                darwin_web::cache_keys::COOKIES,
                                                                &json,
                                                            );
                                                        }
                                                    }
                                                }
                                                // Send result + driver handle back to UI thread
                                                let driver_arc = std::sync::Arc::new(
                                                    tokio::sync::Mutex::new(driver),
                                                );
                                                let _ = tx.send((result, Some(driver_arc)));
                                            }
                                            Err(e) => {
                                                let _ = tx.send((Err(e), None));
                                            }
                                        }
                                    });
                                });
                        }
                    }
                    _ => {
                        self.log.push_back(LogEntry::err(
                            "Darwinex credentials not set — use DWXSETCREDS first",
                        ));
                    }
                }
            }
            "DWXSYNC" => {
                if !self.dwx_logged_in {
                    self.log
                        .push_back(LogEntry::warn("Not logged in — use DWXLOGIN first"));
                } else {
                    self.log
                        .push_back(LogEntry::info("Manual DARWIN web scrape started..."));
                    // Trigger scrape via the existing driver session
                    if let Some(ref driver_arc) = self.dwx_driver {
                        let driver_arc = driver_arc.clone();
                        let config = self.dwx_config.clone();
                        let cache_arc = self.cache.clone();
                        let (tx, rx) = std::sync::mpsc::channel();
                        self.dwx_rx = Some(rx);
                        let rt = self.rt_handle.clone();
                        let _ = std::thread::Builder::new()
                            .name("typhoon-dwx-sync".into())
                            .spawn(move || {
                                rt.block_on(async {
                                    let driver = driver_arc.lock().await;
                                    let cache_fn = |key: &str, val: &str| -> Result<(), String> {
                                        if let Some(ref c) = cache_arc {
                                            c.put_kv(key, val)
                                        } else {
                                            Ok(())
                                        }
                                    };
                                    let result = typhoon_engine::core::darwin_web::scrape_all(
                                        &driver, &config, cache_fn,
                                    )
                                    .await;
                                    let _ = tx.send((result, None)); // No new driver, just result
                                });
                            });
                    }
                }
            }
            "DWXAUTO" => {
                self.dwx_config.auto_scrape = !self.dwx_config.auto_scrape;
                self.log.push_back(LogEntry::info(format!(
                    "Darwinex auto-scrape: {} (every hour at :{:02})",
                    if self.dwx_config.auto_scrape {
                        "ON"
                    } else {
                        "OFF"
                    },
                    self.dwx_config.scrape_minute
                )));
                // Persist config
                if let Some(ref cache) = self.cache {
                    if let Ok(json) = serde_json::to_string(&self.dwx_config) {
                        let _ = cache
                            .put_kv(typhoon_engine::core::darwin_web::cache_keys::CONFIG, &json);
                    }
                }
            }
            "DWXSTATUS" => {
                let status = if self.dwx_logged_in {
                    "logged in"
                } else {
                    "not logged in"
                };
                let auto = if self.dwx_config.auto_scrape {
                    format!("ON (every hour at :{:02})", self.dwx_config.scrape_minute)
                } else {
                    "OFF".to_string()
                };
                let darwins = if self.dwx_config.managed_darwins.is_empty() {
                    "none set".to_string()
                } else {
                    self.dwx_config.managed_darwins.join(", ")
                };
                let excluded = if self.dwx_config.excluded_darwins.is_empty() {
                    "none".to_string()
                } else {
                    self.dwx_config.excluded_darwins.join(", ")
                };
                let last = self
                    .dwx_last_update
                    .as_ref()
                    .map(|u| {
                        let dt = chrono::DateTime::from_timestamp_millis(u.timestamp_ms)
                            .unwrap_or_default();
                        format!(
                            "{} ({} DARWINs, {} corr, {} alerts, {} alloc{}{})",
                            dt.format("%Y-%m-%d %H:%M:%S"),
                            u.snapshots.len(),
                            u.correlations.len(),
                            u.correlation_alerts.len(),
                            u.allocations.len(),
                            if u.portfolio_performance.is_some() {
                                ", perf"
                            } else {
                                ""
                            },
                            if u.portfolio_risk.is_some() {
                                ", risk"
                            } else {
                                ""
                            },
                        )
                    })
                    .unwrap_or_else(|| "never".to_string());
                self.log.push_back(LogEntry::info(format!(
                    "DWX Status: {} | Auto: {} | DARWINs: {} | Excluded: {} | Last scrape: {}",
                    status, auto, darwins, excluded, last
                )));
            }
            "DWXLOGOUT" => {
                if let Some(driver_arc) = self.dwx_driver.take() {
                    let rt = self.rt_handle.clone();
                    let _ = std::thread::Builder::new()
                        .name("typhoon-dwx-logout".into())
                        .spawn(move || {
                            rt.block_on(async {
                                let driver = std::sync::Arc::try_unwrap(driver_arc)
                                    .map_err(|_| "Driver still in use".to_string());
                                if let Ok(mutex) = driver {
                                    let d = mutex.into_inner();
                                    let _ =
                                        typhoon_engine::core::darwin_web::close_browser(d).await;
                                }
                            });
                        });
                    self.dwx_logged_in = false;
                    self.log
                        .push_back(LogEntry::info("Darwinex browser closed"));
                } else {
                    self.log
                        .push_back(LogEntry::warn("No active Darwinex browser session"));
                }
                // Clear cached cookies
                if let Some(ref cache) = self.cache {
                    let _ =
                        cache.put_kv(typhoon_engine::core::darwin_web::cache_keys::COOKIES, "[]");
                }
            }
            "DWXSETCREDS" => {
                // For now, prompt user to set via keyring manually or add a UI dialog
                self.log
                    .push_back(LogEntry::info("Use system keyring to set credentials:"));
                self.log.push_back(LogEntry::info(
                    "  keyring set typhoon-terminal darwinex_email YOUR_EMAIL",
                ));
                self.log.push_back(LogEntry::info(
                    "  keyring set typhoon-terminal darwinex_password YOUR_PASSWORD",
                ));
            }
            cmd if cmd.starts_with("DWXDARWINS ") || cmd.starts_with("DWXDARWINS\t") => {
                let tickers: Vec<String> = cmd[11..]
                    .split_whitespace()
                    .map(|s| s.trim().to_uppercase())
                    .filter(|s| !s.is_empty())
                    .collect();
                if tickers.is_empty() {
                    self.log
                        .push_back(LogEntry::warn("Usage: DWXDARWINS TPN AJT XUQF ..."));
                } else {
                    self.dwx_config.managed_darwins = tickers.clone();
                    self.dwx_config.normalize();
                    self.log.push_back(LogEntry::info(format!(
                        "Managed DARWINs set: {}",
                        self.dwx_config.managed_darwins.join(", ")
                    )));
                    if let Some(ref cache) = self.cache {
                        if let Ok(json) = serde_json::to_string(&self.dwx_config) {
                            let _ = cache.put_kv(
                                typhoon_engine::core::darwin_web::cache_keys::CONFIG,
                                &json,
                            );
                        }
                    }
                }
            }
            cmd if cmd.starts_with("DWXEXCLUDE ") || cmd.starts_with("DWXEXCLUDE\t") => {
                let tickers: Vec<String> = cmd[11..]
                    .split_whitespace()
                    .map(|s| s.trim().to_uppercase())
                    .filter(|s| !s.is_empty())
                    .collect();
                if tickers.is_empty() {
                    self.log
                        .push_back(LogEntry::warn("Usage: DWXEXCLUDE MFSO ..."));
                } else {
                    self.dwx_config.excluded_darwins = tickers.clone();
                    self.dwx_config.normalize();
                    self.log.push_back(LogEntry::info(format!(
                        "Excluded DARWINs: {}",
                        self.dwx_config.excluded_darwins.join(", ")
                    )));
                    if let Some(ref cache) = self.cache {
                        if let Ok(json) = serde_json::to_string(&self.dwx_config) {
                            let _ = cache.put_kv(
                                typhoon_engine::core::darwin_web::cache_keys::CONFIG,
                                &json,
                            );
                        }
                    }
                }
            }
            "RISKRUIN" => self.show_risk_ruin = true,
            "SCRAPESTATUS" => {
                self.show_scrape_status = true;
                // ADR-094: Summary result card for scrape status
                self.result_card = Some((
                    ResultCard::Summary {
                        title: "Scrape Status".to_string(),
                        metrics: vec![
                            (
                                "Fundamentals".to_string(),
                                format!("{}/{}", self.scrape_fund_ok, self.scrape_fund_total),
                                if self.scrape_fund_running {
                                    egui::Color32::YELLOW
                                } else {
                                    egui::Color32::from_rgb(80, 220, 120)
                                },
                            ),
                            (
                                "SEC".to_string(),
                                if self.scrape_sec_running {
                                    "Running".to_string()
                                } else {
                                    "Idle".to_string()
                                },
                                if self.scrape_sec_running {
                                    egui::Color32::YELLOW
                                } else {
                                    egui::Color32::GRAY
                                },
                            ),
                        ],
                    },
                    std::time::Instant::now(),
                ));
            }
            "WEBSERVER" => {
                if !self.web_server_running {
                    if self.lan_sync_passphrase.is_empty() {
                        self.log.push_back(LogEntry::err(
                            "Set LAN sync passphrase in Settings before starting web server",
                        ));
                    } else {
                        // Generate ephemeral self-signed TLS cert (same as LAN sync)
                        match typhoon_engine::core::lan_sync::generate_self_signed_cert() {
                            Ok((cert_pem, key_pem, _fingerprint)) => {
                                let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel::<
                                    typhoon_web_protocol::WebCmd,
                                >();
                                let (msg_tx, _) = tokio::sync::broadcast::channel::<
                                    typhoon_web_protocol::WebMsg,
                                >(512);
                                let state = typhoon_web_server::WebServerState {
                                    cmd_tx,
                                    msg_tx: msg_tx.clone(),
                                    passphrase: self.lan_sync_passphrase.clone(),
                                };
                                let wasm_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                                    .join("../target/web-dist");
                                typhoon_web_server::start_web_server(
                                    &self.rt_handle,
                                    state,
                                    9848,
                                    wasm_dir,
                                    cert_pem,
                                    key_pem,
                                );
                                self.web_cmd_rx = Some(cmd_rx);
                                self.web_msg_tx = Some(msg_tx);
                                self.web_server_running = true;
                                self.log.push_back(LogEntry::info("Web server started on https://0.0.0.0:9848 (passphrase required)"));
                            }
                            Err(e) => {
                                self.log.push_back(LogEntry::err(format!(
                                    "Web server TLS cert failed: {e}"
                                )));
                            }
                        }
                    }
                } else {
                    self.log
                        .push_back(LogEntry::info("Web server already running on port 9848"));
                }
            }
            "REPLAY" => {
                self.replay_active = !self.replay_active;
                if self.replay_active {
                    self.replay_bar_idx = 50.min(
                        self.charts
                            .get(self.active_tab)
                            .map(|c| c.bars.len())
                            .unwrap_or(0),
                    );
                    self.replay_playing = false;
                    self.replay_timer = 0.0;
                    self.log.push_back(LogEntry::info(
                        "Replay ON — Space: play/pause, →: next bar, ←: prev bar, ↑/↓: speed"
                            .to_string(),
                    ));
                } else {
                    self.replay_bar_idx = 0;
                    self.replay_playing = false;
                    self.log.push_back(LogEntry::info("Replay OFF".to_string()));
                }
            }
            "ALERTS" => {
                self.show_alert_builder = true;
                self.alert_symbol = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| c.symbol.clone())
                    .unwrap_or_default();
            }
            "OUTLIERS" => {
                // Multi-dimensional outlier detection: VaR + EV + ATR + SEC + Volume
                // Uses the global broker scope (set via SCOPE command).
                let fund_owned = self.scoped_fundamentals_owned();
                if let Some(ref cache) = self.cache {
                    if let Some(_conn) = cache.try_connection() {
                        use typhoon_engine::core::var;
                        let fund = &fund_owned;
                        if fund.len() < 10 {
                            self.log.push_back(LogEntry::warn(
                                "Need 10+ symbols with fundamentals data. Run EVSCRAPE first.",
                            ));
                        } else {
                            // Build per-symbol data maps from all available sources
                            let mut symbols: Vec<(String, String, String)> = Vec::new();
                            let mut ev_map = std::collections::HashMap::new();
                            let mut var_map = std::collections::HashMap::new();
                            let mut atr_map = std::collections::HashMap::new();
                            let mut sec_map = std::collections::HashMap::new();

                            for f in fund {
                                let sector = if f.sector.is_empty() {
                                    "Unknown".to_string()
                                } else {
                                    f.sector.clone()
                                };
                                let industry = if f.industry.is_empty() {
                                    sector.clone()
                                } else {
                                    f.industry.clone()
                                };
                                // PERF: Clone symbol ONCE per row (was 4x — outliers, ev_map, var_map, atr_map)
                                let sym = f.symbol.clone();
                                symbols.push((sym.clone(), sector, industry));
                                // EV: MCap/EV ratio (valuation anomaly)
                                if let (Some(mc), Some(ev)) = (f.market_cap, f.enterprise_value) {
                                    if ev > 0.0 {
                                        ev_map.insert(sym.clone(), mc / ev * 100.0);
                                    }
                                }
                                // P/E as proxy for VaR (extreme P/E = risk)
                                if let Some(pe) = f.pe_ratio {
                                    if pe.abs() > 0.0 {
                                        var_map.insert(sym.clone(), pe.abs());
                                    }
                                }
                                // Short ratio as ATR proxy (high short = volatility risk)
                                if let Some(sr) = f.short_ratio {
                                    if sr > 0.0 {
                                        atr_map.insert(sym, sr);
                                    }
                                }
                            }
                            // SEC filings per symbol — initialize ALL symbols to 0 first
                            // so z-score sees full distribution (not just non-zero entries)
                            for (sym, _, _) in &symbols {
                                sec_map.entry(sym.clone()).or_insert(0);
                            }
                            for filing in &self.bg.sec_filings {
                                *sec_map.entry(filing.ticker.clone()).or_insert(0) += 1;
                            }
                            // Also count insider trades
                            for (ticker, trades) in &self.bg.insider_trades {
                                *sec_map.entry(ticker.clone()).or_insert(0) += trades.len() as i32;
                            }

                            // Run multi-dimensional outlier detection
                            let multi = var::detect_multi_outliers(
                                &symbols, &var_map, &ev_map, &atr_map, &sec_map, 1.5,
                            );
                            // Also run single-dimension (legacy) for sector stats
                            let data: Vec<(String, String, String, f64)> = fund
                                .iter()
                                .filter_map(|f| {
                                    f.market_cap.map(|mc| {
                                        let sector = if f.sector.is_empty() {
                                            "Unknown".to_string()
                                        } else {
                                            f.sector.clone()
                                        };
                                        let industry = if f.industry.is_empty() {
                                            sector.clone()
                                        } else {
                                            f.industry.clone()
                                        };
                                        (f.symbol.clone(), sector, industry, mc)
                                    })
                                })
                                .filter(|(_, _, _, mc)| *mc > 0.0)
                                .collect();
                            let (outliers, stats) = var::detect_outliers(&data, 1.5);

                            let extreme =
                                multi.iter().filter(|o| o.dimensions_flagged >= 3).count();
                            let high = multi.iter().filter(|o| o.dimensions_flagged == 2).count();
                            self.log.push_back(LogEntry::info(format!(
                                "Multi-outlier scan: {} total ({} EXTREME, {} HIGH) from {} symbols | VaR:{} EV:{} ATR:{} SEC:{}",
                                multi.len(), extreme, high, symbols.len(),
                                var_map.len(), ev_map.len(), atr_map.len(), sec_map.len()
                            )));
                            self.darwinex_outliers = outliers;
                            self.darwinex_sector_stats = stats;
                            self.darwinex_multi_outliers = multi.clone();
                            self.show_darwinex_outliers = true;
                            self.outlier_scroll_pending = true;

                            // ADR-094: Table result card for top outliers
                            if !multi.is_empty() {
                                let headers = vec![
                                    "Symbol".into(),
                                    "Score".into(),
                                    "Dims".into(),
                                    "Tier".into(),
                                ];
                                let rows: Vec<Vec<String>> = multi
                                    .iter()
                                    .take(20)
                                    .map(|o| {
                                        vec![
                                            o.symbol.clone(),
                                            format!("{:.1}", o.composite_score),
                                            format!("{}", o.dimensions_flagged),
                                            o.tier.clone(),
                                        ]
                                    })
                                    .collect();
                                self.result_card = Some((
                                    ResultCard::Table {
                                        title: "Multi-Dimensional Outliers".to_string(),
                                        headers,
                                        rows,
                                        sort_col: 1,
                                        sort_asc: false,
                                    },
                                    std::time::Instant::now(),
                                ));
                            }
                        }
                    }
                }
            }
            "DARWINVAR" | "DARWINVAROUTLIERS" | "VAROUTLIERS" => {
                // DARWIN VaR outlier scanner: IQR detection on per-DARWIN var_95 values,
                // plus flagging against Darwinex corridor (3.25% – 6.5% of equity).
                use typhoon_engine::core::var;
                if self.bg.per_darwin_var.len() < 4 {
                    self.log.push_back(LogEntry::warn(format!(
                        "Need 4+ DARWINs with VaR data (have {}). Load DARWIN daily returns first.",
                        self.bg.per_darwin_var.len()
                    )));
                } else {
                    // Flat distribution — all DARWINs in one "sector" since they're all strategies.
                    // Industry mirrors sector (no finer classification exists for DARWINs).
                    let data: Vec<(String, String, String, f64)> = self
                        .bg
                        .per_darwin_var
                        .iter()
                        .filter(|(_, vr)| vr.var_95 > 0.0)
                        .map(|(ticker, vr)| {
                            (
                                ticker.clone(),
                                "DARWIN".to_string(),
                                "DARWIN".to_string(),
                                vr.var_95,
                            )
                        })
                        .collect();
                    let (outliers, stats) = var::detect_outliers(&data, 1.5);

                    // Darwinex corridor: 3.25% - 6.5% of equity.
                    // Assumes var_95 is expressed as % of equity (typical for Darwinex VaR).
                    const CORRIDOR_LOW: f64 = 3.25;
                    const CORRIDOR_HIGH: f64 = 6.50;
                    let below: Vec<&str> = data
                        .iter()
                        .filter(|(_, _, _, v)| *v < CORRIDOR_LOW)
                        .map(|(s, _, _, _)| s.as_str())
                        .collect();
                    let above: Vec<&str> = data
                        .iter()
                        .filter(|(_, _, _, v)| *v > CORRIDOR_HIGH)
                        .map(|(s, _, _, _)| s.as_str())
                        .collect();

                    self.log.push_back(LogEntry::info(format!(
                        "DARWIN VaR outliers: {} IQR-flagged from {} DARWINs | Corridor violations: {} below {:.2}%, {} above {:.2}%",
                        outliers.len(), data.len(), below.len(), CORRIDOR_LOW, above.len(), CORRIDOR_HIGH
                    )));
                    if !below.is_empty() {
                        self.log.push_back(LogEntry::warn(format!(
                            "Below corridor: {}",
                            below.join(", ")
                        )));
                    }
                    if !above.is_empty() {
                        self.log.push_back(LogEntry::err(format!(
                            "Above corridor (rule violation): {}",
                            above.join(", ")
                        )));
                    }

                    self.darwinex_outliers = outliers;
                    self.darwinex_sector_stats = stats;
                    self.darwinex_multi_outliers = Vec::new();
                    self.show_darwinex_outliers = true;
                    self.outlier_scroll_pending = true;

                    // ADR-094: Show VaR corridor gauge as result card
                    let avg_var =
                        data.iter().map(|(_, _, _, v)| v).sum::<f64>() / data.len().max(1) as f64;
                    self.result_card = Some((
                        ResultCard::Gauge {
                            title: "DARWIN VaR Corridor".to_string(),
                            label: "Avg VaR95".to_string(),
                            value: avg_var,
                            min: 0.0,
                            max: 10.0,
                            danger_low: CORRIDOR_LOW,
                            danger_high: CORRIDOR_HIGH,
                        },
                        std::time::Instant::now(),
                    ));

                    // ADR-094: Toast for corridor violations
                    if !above.is_empty() {
                        self.toasts.push(Toast {
                            message: format!(
                                "VaR CORRIDOR BREACH: {} above 6.5%",
                                above.join(", ")
                            ),
                            color: egui::Color32::from_rgb(255, 80, 80),
                            created: std::time::Instant::now(),
                            duration: std::time::Duration::from_secs(30),
                            dismissable: true,
                            dismissed: false,
                        });
                    }
                }
            }
            "EVOUTLIERS" | "EV_OUTLIERS" => {
                // Enterprise value outlier scanner: IQR detection on EV, grouped by sector.
                // Respects the global broker_scope filter.
                use typhoon_engine::core::var;
                let fund_owned = self.scoped_fundamentals_owned();
                let fund = &fund_owned;
                let scope_label = self.broker_scope_label();
                let data: Vec<(String, String, String, f64)> = fund
                    .iter()
                    .filter_map(|f| {
                        f.enterprise_value.map(|ev| {
                            let sector = if f.sector.is_empty() {
                                "Unknown".to_string()
                            } else {
                                f.sector.clone()
                            };
                            let industry = if f.industry.is_empty() {
                                sector.clone()
                            } else {
                                f.industry.clone()
                            };
                            (f.symbol.clone(), sector, industry, ev)
                        })
                    })
                    .filter(|(_, _, _, ev)| *ev > 0.0)
                    .collect();
                if data.len() < 10 {
                    self.log.push_back(LogEntry::warn(format!(
                        "Need 10+ symbols with enterprise_value (have {}). Run EVSCRAPE first.",
                        data.len()
                    )));
                } else {
                    let (outliers, stats) = var::detect_outliers(&data, 1.5);
                    let extreme = outliers.iter().filter(|o| o.tier == "EXTREME").count();
                    let high = outliers.iter().filter(|o| o.tier == "HIGH").count();
                    self.log.push_back(LogEntry::info(format!(
                        "EV outliers [{}]: {} total ({} EXTREME, {} HIGH) from {} symbols across {} sectors",
                        scope_label, outliers.len(), extreme, high, data.len(), stats.len()
                    )));
                    self.darwinex_outliers = outliers;
                    self.darwinex_sector_stats = stats;
                    self.darwinex_multi_outliers = Vec::new();
                    self.show_darwinex_outliers = true;
                    self.outlier_scroll_pending = true;
                }
            }
            "VAROUTLIER" | "VAR_OUTLIER" | "VAR_OUTLIERS" => {
                // VaR/Ask ratio IQR analysis.
                // Computes VaR_1_Lot from daily returns (95% confidence) for each symbol,
                // then runs 3-level IQR detection: industry → aggregated sector → global.
                use typhoon_engine::core::var;
                let fund_owned = self.scoped_fundamentals_owned();
                let scope_label = self.broker_scope_label();

                if fund_owned.len() < 10 {
                    self.log.push_back(LogEntry::warn(format!(
                        "Need 10+ symbols with fundamentals data (have {}). Run EVSCRAPE first.",
                        fund_owned.len()
                    )));
                } else if let Some(ref cache) = self.cache {
                    // Compute VaR/Ask ratio from bar cache + tick specs (DWEX Portfolio Risk Man formula)
                    let tick_specs = if let Some(conn) = cache.try_connection() {
                        darwin::load_tick_specs(&conn).unwrap_or_default()
                    } else {
                        std::collections::HashMap::new()
                    };
                    let mut var_data: Vec<(String, String, String, f64)> = Vec::new();
                    let mut no_bars = 0usize;

                    for f in &fund_owned {
                        let sector = if f.sector.is_empty() {
                            "Unknown".to_string()
                        } else {
                            f.sector.clone()
                        };
                        let industry = if f.industry.is_empty() {
                            sector.clone()
                        } else {
                            f.industry.clone()
                        };
                        let keys = [
                            format!("mt5:{}:1Day", f.symbol),
                            format!("alpaca:{}:1Day", f.symbol),
                        ];
                        let mut closes: Vec<f64> = Vec::new();
                        for key in &keys {
                            if let Ok(Some(bars)) = cache.get_bars_raw(key) {
                                if bars.len() >= 30 {
                                    closes = bars.iter().map(|(_, _, _, _, c, _)| *c).collect();
                                    break;
                                }
                            }
                        }
                        if closes.len() < 30 {
                            no_bars += 1;
                            continue;
                        }
                        let sym_upper = f.symbol.to_uppercase();
                        let tick_scale = tick_specs.get(&sym_upper).copied().unwrap_or(1.0);
                        if let Some((_, ratio)) =
                            var::compute_var_from_closes_with_tick(&closes, 0.95, tick_scale)
                        {
                            var_data.push((f.symbol.clone(), sector, industry, ratio));
                        }
                    }

                    if var_data.len() < 5 {
                        self.log.push_back(LogEntry::warn(format!(
                            "Need 5+ symbols with D1 bar data for VaR (have {}, {} missing bars). Run MT5SYNC first.",
                            var_data.len(), no_bars
                        )));
                    } else {
                        // IQR analysis grouped by sector (industry carried as display column).
                        // Industry has too few peers per group (~2-5) for IQR to be statistically
                        // meaningful — sector (~10-30 peers) is the right granularity.
                        let (sector_outliers, sector_stats) = var::detect_outliers(&var_data, 1.5);

                        // Global statistics
                        let mut vals: Vec<f64> = var_data.iter().map(|(_, _, _, v)| *v).collect();
                        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                        let q1 = vals[vals.len() / 4];
                        let q3 = vals[3 * vals.len() / 4];
                        let iqr = q3 - q1;

                        self.log.push_back(LogEntry::info(format!(
                            "VaR/Ask outlier scan [{}]: {} symbols | {} outliers across {} sectors",
                            scope_label,
                            var_data.len(),
                            sector_outliers.len(),
                            sector_stats.len()
                        )));
                        self.log.push_back(LogEntry::info(format!(
                            "Global VaR/Ask: Q1={:.2}% Q3={:.2}% IQR={:.2}% Bounds=[{:.2}%, {:.2}%]",
                            q1, q3, iqr, q1 - 1.5 * iqr, q3 + 1.5 * iqr
                        )));

                        // Show sector-level outliers (primary view)
                        self.darwinex_outliers = sector_outliers;
                        self.darwinex_sector_stats = sector_stats;
                        self.darwinex_multi_outliers = Vec::new();
                        self.show_darwinex_outliers = true;
                        self.outlier_scroll_pending = true;
                    }
                }
            }
            "ATROUTLIER" | "ATR_OUTLIER" => {
                // ATR/Price ratio IQR analysis.
                // Computes ATR(14)/Close for each symbol, groups by sector, runs IQR detection.
                use typhoon_engine::core::var;
                let fund_owned = self.scoped_fundamentals_owned();
                let scope_label = self.broker_scope_label();

                if fund_owned.len() < 10 {
                    self.log.push_back(LogEntry::warn(format!(
                        "Need 10+ symbols with fundamentals data (have {}). Run EVSCRAPE first.",
                        fund_owned.len()
                    )));
                } else if let Some(ref cache) = self.cache {
                    let mut atr_data: Vec<(String, String, String, f64)> = Vec::new();
                    let mut no_bars = 0usize;

                    for f in &fund_owned {
                        let sector = if f.sector.is_empty() {
                            "Unknown".to_string()
                        } else {
                            f.sector.clone()
                        };
                        let industry = if f.industry.is_empty() {
                            sector.clone()
                        } else {
                            f.industry.clone()
                        };
                        let keys = [
                            format!("mt5:{}:1Day", f.symbol),
                            format!("alpaca:{}:1Day", f.symbol),
                        ];
                        let mut bars: Vec<(f64, f64, f64, f64)> = Vec::new(); // (o,h,l,c)
                        for key in &keys {
                            if let Ok(Some(raw)) = cache.get_bars_raw(key) {
                                if raw.len() >= 20 {
                                    bars = raw
                                        .iter()
                                        .map(|(_, o, h, l, c, _)| (*o, *h, *l, *c))
                                        .collect();
                                    break;
                                }
                            }
                        }
                        if bars.len() < 20 {
                            no_bars += 1;
                            continue;
                        }
                        // Compute ATR(14)
                        let period = 14;
                        let n = bars.len();
                        let mut atr = 0.0_f64;
                        for i in 1..n.min(period + 1) {
                            let tr = (bars[i].1 - bars[i].2)
                                .max((bars[i].1 - bars[i - 1].3).abs())
                                .max((bars[i].2 - bars[i - 1].3).abs());
                            atr += tr;
                        }
                        atr /= period as f64;
                        for i in (period + 1)..n {
                            let tr = (bars[i].1 - bars[i].2)
                                .max((bars[i].1 - bars[i - 1].3).abs())
                                .max((bars[i].2 - bars[i - 1].3).abs());
                            atr = (atr * (period as f64 - 1.0) + tr) / period as f64;
                        }
                        let close = bars.last().map(|b| b.3).unwrap_or(0.0);
                        if close > 0.0 && atr > 0.0 {
                            atr_data.push((
                                f.symbol.clone(),
                                sector,
                                industry,
                                atr / close * 100.0,
                            ));
                        }
                    }

                    if atr_data.len() < 5 {
                        self.log.push_back(LogEntry::warn(format!(
                            "Need 5+ symbols with D1 bar data (have {}, {} missing). Run MT5SYNC first.",
                            atr_data.len(), no_bars
                        )));
                    } else {
                        let (outliers, stats) = var::detect_outliers(&atr_data, 1.5);
                        self.log.push_back(LogEntry::info(format!(
                            "ATR/Price outlier scan [{}]: {} outliers from {} symbols across {} sectors",
                            scope_label, outliers.len(), atr_data.len(), stats.len()
                        )));
                        self.darwinex_outliers = outliers;
                        self.darwinex_sector_stats = stats;
                        self.darwinex_multi_outliers = Vec::new();
                        self.show_darwinex_outliers = true;
                        self.outlier_scroll_pending = true;
                    }
                }
            }
            "DARWINIA_SCAN" | "DARWIN_SCAN" | "GPU_SCAN" => {
                if self.darwin_ftp_dir.is_empty() {
                    self.log
                        .push_back(LogEntry::warn("Set Darwinex FTP Dir in Settings first"));
                } else if self.gpu_darwin.is_some() {
                    // GPU available — use GPU-accelerated scan
                    let _ = self.broker_tx.send(BrokerCmd::DarwinGpuScan {
                        ftp_dir: self.darwin_ftp_dir.clone(),
                        min_days: 90,
                    });
                    self.log.push_back(LogEntry::info(
                        "DarwinIA scan started (GPU, 50K DARWINs)...",
                    ));
                } else {
                    // CPU fallback
                    let _ = self.broker_tx.send(BrokerCmd::DarwinFtpScan {
                        ftp_dir: self.darwin_ftp_dir.clone(),
                        min_days: 90,
                    });
                    self.log.push_back(LogEntry::info(
                        "DarwinIA scan started (CPU fallback, no GPU)...",
                    ));
                }
            }
            // Drawing tools
            "SNAP" | "MAGNET" => {
                self.snap_enabled = !self.snap_enabled;
                self.log.push_back(LogEntry::info(format!(
                    "Magnet snap: {}",
                    if self.snap_enabled { "ON" } else { "OFF" }
                )));
            }
            "CROSS_TF" | "CROSS_TF_DRAWINGS" => {
                self.cross_tf_drawings = !self.cross_tf_drawings;
                self.log.push_back(LogEntry::info(format!(
                    "Cross-TF drawings: {}",
                    if self.cross_tf_drawings {
                        "ON — drawings shared across timeframes"
                    } else {
                        "OFF"
                    }
                )));
            }
            "FIT" | "FIT_ALL" | "AUTO_FIT" => {
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    chart.visible_bars = chart.bars.len().max(50);
                    chart.view_offset = chart.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
                    chart.price_zoom = 1.0;
                    chart.price_pan = 0.0;
                    self.log
                        .push_back(LogEntry::info("Auto-fit: showing all bars"));
                }
            }
            "LOG_SCALE" | "LOG" => {
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    chart.log_scale = !chart.log_scale;
                    self.log.push_back(LogEntry::info(format!(
                        "Price scale: {}",
                        if chart.log_scale {
                            "logarithmic"
                        } else {
                            "linear"
                        }
                    )));
                }
            }
            "FOLLOW" | "AUTO_SCROLL" => {
                self.follow_latest = !self.follow_latest;
                self.log.push_back(LogEntry::info(format!(
                    "Follow latest: {}",
                    if self.follow_latest {
                        "ON — chart auto-scrolls"
                    } else {
                        "OFF — locked position"
                    }
                )));
            }
            "DRAW_HLINE" => self.draw_mode = DrawMode::PlacingHLine,
            "DRAW_TRENDLINE" => self.draw_mode = DrawMode::PlacingTrendP1,
            "DRAW_FIBO" => self.draw_mode = DrawMode::PlacingFiboP1,
            "DRAW_VLINE" => self.draw_mode = DrawMode::PlacingVLine,
            "DRAW_RECT" => self.draw_mode = DrawMode::PlacingRectP1,
            "DRAW_RAY" => self.draw_mode = DrawMode::PlacingRayP1,
            "DRAW_CHANNEL" => self.draw_mode = DrawMode::PlacingChannelP1,
            "DRAW_PARALLEL_CH" => self.draw_mode = DrawMode::PlacingParallelChP1,
            "DRAW_FIB_CHANNEL" => self.draw_mode = DrawMode::PlacingFibChannelP1,
            "DRAW_FIB_TIME" => self.draw_mode = DrawMode::PlacingFibTimeZones,
            "DRAW_PRICE_LABEL" => self.draw_mode = DrawMode::PlacingPriceLabel,
            "DRAW_CALLOUT" => self.draw_mode = DrawMode::PlacingCalloutP1,
            "DRAW_HIGHLIGHTER" => self.draw_mode = DrawMode::PlacingHighlighterP1,
            "DRAW_CROSS_MARKER" => self.draw_mode = DrawMode::PlacingCrossMarker,
            "DRAW_POLYLINE" => {
                self.draw_mode = DrawMode::PlacingPolyline;
                self.polyline_points.clear();
            }
            "DRAW_ANCHOR_NOTE" => self.draw_mode = DrawMode::PlacingAnchorNote,
            "DRAW_REGRESSION" => self.draw_mode = DrawMode::PlacingRegressionChP1,
            "DRAW_GANN_BOX" => self.draw_mode = DrawMode::PlacingGannBoxP1,
            "DRAW_ELLIOTT" => {
                self.draw_mode = DrawMode::PlacingElliottWave;
                self.multi_click_points.clear();
            }
            "DRAW_ABC" => {
                self.draw_mode = DrawMode::PlacingAbcCorrection;
                self.multi_click_points.clear();
            }
            "DRAW_DATE_RANGE" => self.draw_mode = DrawMode::PlacingDateRangeP1,
            "DRAW_DATE_PRICE" => self.draw_mode = DrawMode::PlacingDatePriceRangeP1,
            "DRAW_HEAD_SHOULDERS" => {
                self.draw_mode = DrawMode::PlacingHeadShoulders;
                self.multi_click_points.clear();
            }
            "DRAW_XABCD" => {
                self.draw_mode = DrawMode::PlacingXabcdPattern;
                self.multi_click_points.clear();
            }
            "DRAW_BRUSH" => {
                self.draw_mode = DrawMode::PlacingBrush;
                self.brush_points.clear();
            }
            "DRAW_SCHIFF_FORK" => self.draw_mode = DrawMode::PlacingSchiffPitchforkP1,
            "DRAW_MOD_SCHIFF_FORK" => self.draw_mode = DrawMode::PlacingModSchiffPitchforkP1,
            "DRAW_CYCLIC_LINES" => self.draw_mode = DrawMode::PlacingCyclicLinesP1,
            "DRAW_SINE_WAVE" => self.draw_mode = DrawMode::PlacingSineWaveP1,
            "DRAW_EMOJI" => self.draw_mode = DrawMode::PlacingEmoji,
            "DRAW_FLAG" => self.draw_mode = DrawMode::PlacingFlag,
            "DRAW_BALLOON" => self.draw_mode = DrawMode::PlacingBalloonP1,
            "DRAW_SESSION_BREAK" => self.draw_mode = DrawMode::PlacingSessionBreak,
            "DRAW_MAGNET_LEVEL" => self.draw_mode = DrawMode::PlacingMagnetLevel,
            "DRAW_RISK_REWARD" => self.draw_mode = DrawMode::PlacingRiskRewardP1,
            "DRAW_FIB_CIRCLE" => self.draw_mode = DrawMode::PlacingFibCircleP1,
            "DRAW_ARC" => self.draw_mode = DrawMode::PlacingArcP1,
            "DRAW_CURVE" => self.draw_mode = DrawMode::PlacingCurveP1,
            "DRAW_PATH" => {
                self.draw_mode = DrawMode::PlacingPath;
                self.polyline_points.clear();
            }
            "DRAW_FORECAST" => self.draw_mode = DrawMode::PlacingForecastP1,
            "DRAW_GHOST_FEED" => self.draw_mode = DrawMode::PlacingGhostFeedP1,
            "DRAW_SIGNPOST" => self.draw_mode = DrawMode::PlacingSignpost,
            "DRAW_RULER" => self.draw_mode = DrawMode::PlacingRulerP1,
            "DRAW_TIME_CYCLE" => self.draw_mode = DrawMode::PlacingTimeCycleP1,
            "DRAW_SPEED_FAN" => self.draw_mode = DrawMode::PlacingSpeedFanP1,
            "DRAW_SPEED_ARC" => self.draw_mode = DrawMode::PlacingSpeedArcP1,
            "DRAW_FIB_SPIRAL" => self.draw_mode = DrawMode::PlacingFibSpiralP1,
            "DRAW_ROTATED_RECT" => self.draw_mode = DrawMode::PlacingRotatedRectP1,
            "DRAW_ANCHORED_VWAP" => self.draw_mode = DrawMode::PlacingAnchoredVwap,
            "DRAW_TREND_CHANNEL" => self.draw_mode = DrawMode::PlacingTrendChannelP1,
            "DRAW_INSIDE_PITCHFORK" => self.draw_mode = DrawMode::PlacingInsidePitchforkP1,
            "DRAW_FIB_WEDGE" => self.draw_mode = DrawMode::PlacingFibWedgeP1,
            "DRAW_PRICE_NOTE" => self.draw_mode = DrawMode::PlacingPriceNote,
            "DRAW_MEASURE_TOOL" => self.draw_mode = DrawMode::PlacingMeasureToolP1,
            "DRAW_ANCHORED_TEXT" => self.draw_mode = DrawMode::PlacingAnchoredText,
            "DRAW_COMMENT" => self.draw_mode = DrawMode::PlacingComment,
            "DRAW_ARROW_LEFT" => self.draw_mode = DrawMode::PlacingArrowMarkerLeft,
            "DRAW_ARROW_RIGHT" => self.draw_mode = DrawMode::PlacingArrowMarkerRight,
            "DRAW_CIRCLE" => self.draw_mode = DrawMode::PlacingCircleP1,
            "DRAW_PITCH_FAN" => self.draw_mode = DrawMode::PlacingPitchFanP1,
            "DRAW_TREND_FIB_TIME" => self.draw_mode = DrawMode::PlacingTrendFibTimeP1,
            "DRAW_GANN_SQUARE" => self.draw_mode = DrawMode::PlacingGannSquareP1,
            "DRAW_GANN_SQUARE_FIXED" => self.draw_mode = DrawMode::PlacingGannSquareFixedP1,
            "DRAW_BARS_PATTERN" => self.draw_mode = DrawMode::PlacingBarsPatternP1,
            "DRAW_PROJECTION" => self.draw_mode = DrawMode::PlacingProjectionP1,
            "DRAW_DOUBLE_CURVE" => self.draw_mode = DrawMode::PlacingDoubleCurveP1,
            "DRAW_TRIANGLE_PATTERN" => {
                self.draw_mode = DrawMode::PlacingTrianglePattern;
                self.multi_click_points.clear();
            }
            "DRAW_THREE_DRIVES" => {
                self.draw_mode = DrawMode::PlacingThreeDrives;
                self.multi_click_points.clear();
            }
            "DRAW_ELLIOTT_DOUBLE" => {
                self.draw_mode = DrawMode::PlacingElliottDouble;
                self.multi_click_points.clear();
            }
            "DRAW_ABCD" => {
                self.draw_mode = DrawMode::PlacingAbcdPattern;
                self.multi_click_points.clear();
            }
            "DRAW_CYPHER" => {
                self.draw_mode = DrawMode::PlacingCypherPattern;
                self.multi_click_points.clear();
            }
            "DRAW_ELLIOTT_TRIANGLE" => {
                self.draw_mode = DrawMode::PlacingElliottTriangle;
                self.multi_click_points.clear();
            }
            "DRAW_ELLIOTT_TRIPLE" => {
                self.draw_mode = DrawMode::PlacingElliottTripleCombo;
                self.multi_click_points.clear();
            }
            "DRAW_ERASER" => {
                self.draw_mode = DrawMode::Eraser;
            }
            "CLEAR_DRAWINGS" => {
                if let Some(c) = self.charts.get_mut(self.active_tab) {
                    c.drawings.clear();
                    c.drawing_styles.clear();
                }
            }
            "SESSIONS" => {
                self.show_sessions = !self.show_sessions;
                self.log.push_back(LogEntry::info(format!(
                    "Sessions: {}",
                    if self.show_sessions { "ON" } else { "OFF" }
                )));
            }
            "VOL_HEATMAP" => {
                self.show_vol_heatmap = !self.show_vol_heatmap;
                self.log.push_back(LogEntry::info(format!(
                    "Volume heatmap: {}",
                    if self.show_vol_heatmap { "ON" } else { "OFF" }
                )));
            }
            "VWAP" => {
                self.show_vwap = !self.show_vwap;
                self.log.push_back(LogEntry::info(format!(
                    "VWAP: {}",
                    if self.show_vwap { "ON" } else { "OFF" }
                )));
            }
            "PRICE_HIST" => {
                self.show_price_histogram = !self.show_price_histogram;
                self.log.push_back(LogEntry::info(format!(
                    "Price histogram: {}",
                    if self.show_price_histogram {
                        "ON"
                    } else {
                        "OFF"
                    }
                )));
            }
            "SUPERTREND" => {
                self.show_supertrend = !self.show_supertrend;
                self.log.push_back(LogEntry::info(format!(
                    "Supertrend: {}",
                    if self.show_supertrend { "ON" } else { "OFF" }
                )));
            }
            "DONCHIAN" => {
                self.show_donchian = !self.show_donchian;
                self.log.push_back(LogEntry::info(format!(
                    "Donchian: {}",
                    if self.show_donchian { "ON" } else { "OFF" }
                )));
            }
            "KELTNER" => {
                self.show_keltner = !self.show_keltner;
                self.log.push_back(LogEntry::info(format!(
                    "Keltner: {}",
                    if self.show_keltner { "ON" } else { "OFF" }
                )));
            }
            "REGRESSION" => {
                self.show_regression = !self.show_regression;
                self.log.push_back(LogEntry::info(format!(
                    "Regression: {}",
                    if self.show_regression { "ON" } else { "OFF" }
                )));
            }
            "SQUEEZE" => {
                self.show_squeeze = !self.show_squeeze;
                self.log.push_back(LogEntry::info(format!(
                    "Squeeze: {}",
                    if self.show_squeeze { "ON" } else { "OFF" }
                )));
            }
            "VAROSC" | "VAR_OSC" | "VAR_OSCILLATOR" => {
                self.show_var_oscillator = !self.show_var_oscillator;
                self.log.push_back(LogEntry::info(format!(
                    "VaR Oscillator: {}",
                    if self.show_var_oscillator {
                        "ON"
                    } else {
                        "OFF"
                    }
                )));
            }
            "CMO_CHART" | "SHOW_CMO" => {
                self.show_cmo = !self.show_cmo;
                self.log.push_back(LogEntry::info(format!(
                    "CMO chart pane: {}",
                    if self.show_cmo { "ON" } else { "OFF" }
                )));
            }
            "QSTICK_CHART" | "SHOW_QSTICK" => {
                self.show_qstick = !self.show_qstick;
                self.log.push_back(LogEntry::info(format!(
                    "QStick chart pane: {}",
                    if self.show_qstick { "ON" } else { "OFF" }
                )));
            }
            "DISPARITY_CHART" | "SHOW_DISPARITY" => {
                self.show_disparity = !self.show_disparity;
                self.log.push_back(LogEntry::info(format!(
                    "Disparity chart pane: {}",
                    if self.show_disparity { "ON" } else { "OFF" }
                )));
            }
            "BOP_CHART" | "SHOW_BOP" => {
                self.show_bop = !self.show_bop;
                self.log.push_back(LogEntry::info(format!(
                    "BOP chart pane: {}",
                    if self.show_bop { "ON" } else { "OFF" }
                )));
            }
            "STDDEV_CHART" | "SHOW_STDDEV" => {
                self.show_stddev = !self.show_stddev;
                self.log.push_back(LogEntry::info(format!(
                    "StdDev chart pane: {}",
                    if self.show_stddev { "ON" } else { "OFF" }
                )));
            }
            "MFI_CHART" | "SHOW_MFI" => {
                self.show_mfi = !self.show_mfi;
                self.log.push_back(LogEntry::info(format!(
                    "MFI chart pane: {}",
                    if self.show_mfi { "ON" } else { "OFF" }
                )));
            }
            "TRIX_CHART" | "SHOW_TRIX" => {
                self.show_trix = !self.show_trix;
                self.log.push_back(LogEntry::info(format!(
                    "TRIX chart pane: {}",
                    if self.show_trix { "ON" } else { "OFF" }
                )));
            }
            "PPO_CHART" | "SHOW_PPO" => {
                self.show_ppo = !self.show_ppo;
                self.log.push_back(LogEntry::info(format!(
                    "PPO chart pane: {}",
                    if self.show_ppo { "ON" } else { "OFF" }
                )));
            }
            "ULTOSC_CHART" | "SHOW_ULTOSC" => {
                self.show_ultosc = !self.show_ultosc;
                self.log.push_back(LogEntry::info(format!(
                    "ULTOSC chart pane: {}",
                    if self.show_ultosc { "ON" } else { "OFF" }
                )));
            }
            "STOCHRSI_CHART" | "SHOW_STOCHRSI" => {
                self.show_stochrsi = !self.show_stochrsi;
                self.log.push_back(LogEntry::info(format!(
                    "StochRSI chart pane: {}",
                    if self.show_stochrsi { "ON" } else { "OFF" }
                )));
            }
            "FVG" | "FAIR_VALUE_GAP" => {
                self.show_fvg = !self.show_fvg;
                self.log.push_back(LogEntry::info(format!(
                    "FVG: {}",
                    if self.show_fvg { "ON" } else { "OFF" }
                )));
            }
            "ORDER_BLOCKS" | "OB" => {
                self.show_order_blocks = !self.show_order_blocks;
                self.log.push_back(LogEntry::info(format!(
                    "Order Blocks: {}",
                    if self.show_order_blocks { "ON" } else { "OFF" }
                )));
            }
            "COPY_CHART" => {
                if let Some(chart) = self.charts.get(self.active_tab) {
                    let (vs, ve) = chart.visible_range();
                    let visible = &chart.bars[vs..ve];
                    if visible.is_empty() {
                        self.log
                            .push_back(LogEntry::warn("No visible bars to copy"));
                    } else {
                        let mut csv = String::from("Date,Open,High,Low,Close,Volume\n");
                        for bar in visible {
                            let dt = chrono::DateTime::from_timestamp_millis(bar.ts_ms)
                                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                                .unwrap_or_else(|| bar.ts_ms.to_string());
                            csv.push_str(&format!(
                                "{},{},{},{},{},{}\n",
                                dt, bar.open, bar.high, bar.low, bar.close, bar.volume
                            ));
                        }
                        ctx.copy_text(csv);
                        self.log.push_back(LogEntry::info(format!(
                            "Copied {} bars to clipboard as CSV",
                            visible.len()
                        )));
                    }
                }
            }
            "OBJECTS" | "OBJECT_LIST" => {
                self.show_object_list = !self.show_object_list;
            }
            // Timeframe shortcuts — any TF label works (M1, M2, H6, D3, Y1, etc.)
            _ if Timeframe::from_label(&cmd_upper).is_some() => {
                let tf = match Timeframe::from_label(&cmd_upper) {
                    Some(t) => t,
                    None => return,
                };
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| c.symbol.clone())
                    .unwrap_or_else(|| self.symbol_input.clone());
                self.reload_symbol(&sym, tf);
            }
            // Aliases
            "EQUITY" => self.show_darwin_portfolio = true,
            "CALC" => self.show_risk_calc = true,
            "TRADESTATS" => {
                self.darwin_view = 0;
                self.show_darwin_portfolio = true;
            } // Portfolio Summary
            "PERF" => {
                self.darwin_view = 14;
                self.show_darwin_portfolio = true;
            } // Seasonals
            "COMPARE" => {
                let sym = self.symbol_input.clone();
                if !sym.is_empty() {
                    if let Some(chart) = self.charts.get_mut(self.active_tab) {
                        chart.compare_symbol = Some(sym.clone());
                        chart.compare_bars.clear();
                        if let Some(ref cache) = self.cache {
                            let tf_label = chart.timeframe.cache_suffix();
                            let keys = [
                                format!("mt5:{}:{}", sym, tf_label),
                                format!("alpaca:{}:{}", sym, tf_label),
                            ];
                            for key in &keys {
                                if let Ok(Some(raw)) = cache.get_bars_raw(key) {
                                    chart.compare_bars = raw
                                        .into_iter()
                                        .map(|(ts, o, h, l, c, v)| Bar {
                                            ts_ms: ts,
                                            open: o,
                                            high: h,
                                            low: l,
                                            close: c,
                                            volume: v,
                                        })
                                        .collect();
                                    if !chart.compare_bars.is_empty() {
                                        self.log.push_back(LogEntry::info(format!(
                                            "Compare: {} loaded ({} bars)",
                                            sym,
                                            chart.compare_bars.len()
                                        )));
                                        break;
                                    }
                                }
                            }
                            if chart.compare_bars.is_empty() {
                                self.log.push_back(LogEntry::warn(format!(
                                    "Compare: no cached data for {}",
                                    sym
                                )));
                            }
                        }
                    }
                } else {
                    // Empty symbol clears the compare overlay
                    if let Some(chart) = self.charts.get_mut(self.active_tab) {
                        chart.compare_symbol = None;
                        chart.compare_bars.clear();
                        self.log
                            .push_back(LogEntry::info("Compare overlay cleared"));
                    }
                }
            }
            "SPREAD" => {
                self.darwin_view = 4;
                self.show_darwin_portfolio = true;
            } // Symbol Exposure
            "HEATMAP" => {
                self.darwin_view = 14;
                self.show_darwin_portfolio = true;
            } // Seasonals
            "PROFILE" => self.show_darwin_accounts = true,
            "SIGNAL" => self.show_indicators_panel = true,
            "DASHBOARD" => self.show_cache_stats = true,
            "STATUS" => self.show_cache_stats = true,
            "IMPORT_XLSX" => self.show_darwin_accounts = true,
            "WORKSPACE" => {
                self.save_session();
                self.log.push_back(LogEntry::info("Workspace saved"));
            }
            "BACKUP" => {
                self.save_session();
                self.log.push_back(LogEntry::info("Session backup saved"));
            }
            "QUOTE" => {
                let sym = self.symbol_input.trim().to_string();
                let _ = self.broker_tx.send(BrokerCmd::GetQuote { symbol: sym });
            }
            "CLOCK" => {
                let _ = self.broker_tx.send(BrokerCmd::GetMarketClock);
                if !self.broker_connected {
                    self.log
                        .push_back(LogEntry::warn("Broker not connected — clock may fail"));
                }
            }
            "FILLS" => {
                if self.broker_connected {
                    let _ = self.broker_tx.send(BrokerCmd::GetActivities { limit: 20 });
                } else {
                    self.log
                        .push_back(LogEntry::warn("Connect to broker first"));
                }
            }
            "MOVERS" => {
                if self.broker_connected {
                    let _ = self.broker_tx.send(BrokerCmd::GetTopMovers);
                } else {
                    self.log
                        .push_back(LogEntry::warn("Connect to broker first"));
                }
            }
            "SEARCH" => {
                let query = self.command_input.trim().to_string();
                if query.len() >= 2 {
                    let _ = self.broker_tx.send(BrokerCmd::SearchSymbols { query });
                } else {
                    self.log
                        .push_back(LogEntry::warn("Type at least 2 characters to search"));
                }
            }
            "HISTORY" => {
                if self.broker_connected {
                    let _ = self
                        .broker_tx
                        .send(BrokerCmd::GetOrderHistory { limit: 50 });
                } else {
                    self.log
                        .push_back(LogEntry::warn("Connect to broker first"));
                }
            }
            "PIVOTS" => self.show_pivots = !self.show_pivots,
            "SRLEVEL" => self.show_pivots = !self.show_pivots,
            "FRACTALS" => self.show_fractals = !self.show_fractals,
            "HARMONICS" => self.show_harmonics = !self.show_harmonics,
            "AUTO_FIB" => self.show_auto_fib = !self.show_auto_fib,
            "SUPPLY_DEMAND" => self.show_supply_demand = !self.show_supply_demand,
            "NNFX" => {
                // 1:1 MT5 TyphooN NNFX preset (clean reset + enable)
                // Reset all (skip sma200/kama since they get enabled below)
                self.show_sma100 = false;
                self.show_ema21 = false;
                self.show_bollinger = false;
                self.show_ichimoku = false;
                self.show_wma = false;
                self.show_hma = false;
                self.show_psar = false;
                self.show_rsi = false;
                self.show_macd = false;
                self.show_stochastic = false;
                self.show_adx = false;
                self.show_cci = false;
                self.show_williams_r = false;
                self.show_obv = false;
                self.show_momentum = false;
                self.show_cmo = false;
                self.show_qstick = false;
                self.show_disparity = false;
                self.show_bop = false;
                self.show_stddev = false;
                self.show_mfi = false;
                self.show_trix = false;
                self.show_ppo = false;
                self.show_ultosc = false;
                self.show_stochrsi = false;
                self.show_var_oscillator = false;
                self.show_volume_pane = false;
                self.show_fractals = false;
                self.show_harmonics = false;
                self.show_pivots = false;
                self.show_ehlers_ss = false;
                self.show_ehlers_decycler = false;
                self.show_ehlers_itl = false;
                self.show_ehlers_mama = false;
                self.show_ehlers_ebsw = false;
                self.show_ehlers_cyber = false;
                self.show_ehlers_cg = false;
                self.show_ehlers_roof = false;
                // Main chart: ATR_Projection + PreviousCandleLevels + MultiKAMA + MTF_MA + SupplyDemand + AutoFib
                self.show_atr_proj = true;
                self.show_prev_levels = true;
                self.show_kama = true;
                self.show_sma200 = true;
                self.show_supply_demand = true;
                self.show_auto_fib = true;
                // Sub-pane 1: EhlersFisherTransform | Sub-pane 2: BetterVolume
                self.show_fisher = true;
                self.show_better_volume = true;
                self.log.push_back(LogEntry::info("NNFX preset (1:1 MT5): ATR_Proj + PrevLevels + MultiKAMA + MTF_MA + S/D + AutoFib + Fisher + BVol"));
            }
            "RESET_IND" => {
                self.show_sma200 = false;
                self.show_sma100 = false;
                self.show_kama = false;
                self.show_ema21 = false;
                self.show_bollinger = false;
                self.show_ichimoku = false;
                self.show_wma = false;
                self.show_hma = false;
                self.show_psar = false;
                self.show_atr_proj = false;
                self.show_prev_levels = false;
                self.show_rsi = false;
                self.show_fisher = false;
                self.show_macd = false;
                self.show_stochastic = false;
                self.show_adx = false;
                self.show_cci = false;
                self.show_williams_r = false;
                self.show_obv = false;
                self.show_momentum = false;
                self.show_cmo = false;
                self.show_qstick = false;
                self.show_disparity = false;
                self.show_bop = false;
                self.show_stddev = false;
                self.show_mfi = false;
                self.show_trix = false;
                self.show_ppo = false;
                self.show_ultosc = false;
                self.show_stochrsi = false;
                self.show_var_oscillator = false;
                self.show_better_volume = false;
                self.show_volume_pane = false;
                self.show_fvg = false;
                self.show_order_blocks = false;
                self.log
                    .push_back(LogEntry::info("All indicators disabled"));
            }
            "DATA_WINDOW" => self.show_data_window = true,
            // "ALERTS" handled above (alert builder)
            "ORDER" => {
                self.submit_quick_trade();
            }
            "CRYPTO_FEAR_GREED" | "FEAR_GREED" | "FNG" => {
                self.show_fear_greed = true;
                if self.fear_greed_value == 0 {
                    let _ = self.broker_tx.send(BrokerCmd::FetchFearGreed);
                }
            }
            // Bare window-openers — kept as aliases so existing recent-commands history still works,
            // but removed from the palette in favour of the ASKAI / ASKCLAUDE / ASKGEMINI variants
            // which can also pre-load a research packet on the selected symbols.
            "AI" | "AI_CHAT" | "ASKAI" | "ASK_AI" | "INVESTIGATE" => self.show_ai_chat = true,
            "GEMINI" | "GEMINI_CLI" | "GEMINI-CLI" | "ASKGEMINI" | "ASK_GEMINI" => {
                match std::process::Command::new("which").arg("gemini").output() {
                    Ok(out) if out.status.success() => {
                        self.show_gemini_cli = true;
                        self.log
                            .push_back(LogEntry::info("Gemini CLI detected — opening chat"));
                    }
                    _ => {
                        self.log.push_back(LogEntry::err("Gemini CLI not found in PATH. Install: npm install -g @anthropic-ai/gemini-cli or pip install gemini-cli"));
                    }
                }
            }
            "CLAUDE" | "CLAUDE_CODE" | "CLAUDE-CODE" | "ASKCLAUDE" | "ASK_CLAUDE" => {
                // Check if claude binary exists
                match std::process::Command::new("which").arg("claude").output() {
                    Ok(out) if out.status.success() => {
                        self.show_claude_code = true;
                        self.log
                            .push_back(LogEntry::info("Claude Code CLI detected — opening chat"));
                    }
                    _ => {
                        self.log.push_back(LogEntry::err("Claude Code CLI not found in PATH. Install: npm install -g @anthropic-ai/claude-code"));
                    }
                }
            }
            "CODEX" | "CODEX_CLI" | "CODEX-CLI" | "ASKCODEX" | "ASK_CODEX" => {
                match std::process::Command::new("which").arg("codex").output() {
                    Ok(out) if out.status.success() => {
                        self.show_codex_cli = true;
                        self.log
                            .push_back(LogEntry::info("Codex CLI detected — opening chat"));
                    }
                    _ => {
                        self.log.push_back(LogEntry::err(
                            "Codex CLI not found in PATH. Install: npm install -g @openai/codex",
                        ));
                    }
                }
            }
            "HERMES" | "HERMES_CLI" | "HERMES-CLI" | "ASKHERMES" | "ASK_HERMES" => {
                match std::process::Command::new("which").arg("hermes").output() {
                    Ok(out) if out.status.success() => {
                        self.show_hermes_cli = true;
                        self.log
                            .push_back(LogEntry::info("Hermes Agent CLI detected — opening chat"));
                    }
                    _ => {
                        self.log.push_back(LogEntry::err(
                            "Hermes Agent CLI not found in PATH. Install/configure Hermes Agent first.",
                        ));
                    }
                }
            }
            // ── ADR-157 AI session resume + history browser ──
            "RESUMECLAUDE" | "RESUME_CLAUDE" => {
                if let Some(ref cache) = self.cache {
                    match typhoon_engine::core::ai_sessions::latest_for_provider(cache, "claude") {
                        Ok(Some(rec)) => {
                            self.claude_code_history = rec.turns.clone();
                            self.claude_code_session_id = if rec.cli_session_id.is_empty() {
                                Some(rec.session_id.clone())
                            } else {
                                Some(rec.cli_session_id.clone())
                            };
                            self.show_claude_code = true;
                            self.log.push_back(LogEntry::info(format!(
                                "Resumed Claude session {} ({} turns)",
                                rec.session_id,
                                rec.turns.len()
                            )));
                        }
                        Ok(None) => self
                            .log
                            .push_back(LogEntry::warn("No saved Claude session to resume")),
                        Err(e) => self
                            .log
                            .push_back(LogEntry::err(format!("RESUMECLAUDE: {e}"))),
                    }
                } else {
                    self.log
                        .push_back(LogEntry::warn("Cache not ready — wait a moment and retry"));
                }
            }
            "RESUMEGEMINI" | "RESUME_GEMINI" => {
                if let Some(ref cache) = self.cache {
                    match typhoon_engine::core::ai_sessions::latest_for_provider(cache, "gemini") {
                        Ok(Some(rec)) => {
                            self.gemini_cli_history = rec.turns.clone();
                            self.gemini_cli_session_id = rec.session_id.clone();
                            self.show_gemini_cli = true;
                            self.log.push_back(LogEntry::info(format!(
                                "Resumed Gemini session {} ({} turns — no native --resume, transcript replayed as context)",
                                rec.session_id, rec.turns.len())));
                        }
                        Ok(None) => self
                            .log
                            .push_back(LogEntry::warn("No saved Gemini session to resume")),
                        Err(e) => self
                            .log
                            .push_back(LogEntry::err(format!("RESUMEGEMINI: {e}"))),
                    }
                } else {
                    self.log
                        .push_back(LogEntry::warn("Cache not ready — wait a moment and retry"));
                }
            }
            "RESUMECODEX" | "RESUME_CODEX" => {
                if let Some(ref cache) = self.cache {
                    match typhoon_engine::core::ai_sessions::latest_for_provider(cache, "codex") {
                        Ok(Some(rec)) => {
                            self.codex_cli_history = rec.turns.clone();
                            self.codex_cli_session_id = rec.session_id.clone();
                            self.show_codex_cli = true;
                            self.log.push_back(LogEntry::info(format!(
                                "Resumed Codex session {} ({} turns — no native resume, transcript replayed as context)",
                                rec.session_id, rec.turns.len())));
                        }
                        Ok(None) => self
                            .log
                            .push_back(LogEntry::warn("No saved Codex session to resume")),
                        Err(e) => self
                            .log
                            .push_back(LogEntry::err(format!("RESUMECODEX: {e}"))),
                    }
                } else {
                    self.log
                        .push_back(LogEntry::warn("Cache not ready — wait a moment and retry"));
                }
            }
            "RESUMEHERMES" | "RESUME_HERMES" => {
                if let Some(ref cache) = self.cache {
                    match typhoon_engine::core::ai_sessions::latest_for_provider(cache, "hermes") {
                        Ok(Some(rec)) => {
                            self.hermes_cli_history = rec.turns.clone();
                            self.hermes_cli_session_id = rec.session_id.clone();
                            self.show_hermes_cli = true;
                            self.log.push_back(LogEntry::info(format!(
                                "Resumed Hermes session {} ({} turns — transcript replayed as context)",
                                rec.session_id, rec.turns.len())));
                        }
                        Ok(None) => self
                            .log
                            .push_back(LogEntry::warn("No saved Hermes session to resume")),
                        Err(e) => self
                            .log
                            .push_back(LogEntry::err(format!("RESUMEHERMES: {e}"))),
                    }
                } else {
                    self.log
                        .push_back(LogEntry::warn("Cache not ready — wait a moment and retry"));
                }
            }
            "RESUMEAI" | "RESUME_AI" => {
                if let Some(ref cache) = self.cache {
                    match typhoon_engine::core::ai_sessions::latest_for_provider(cache, "ai_chat") {
                        Ok(Some(rec)) => {
                            self.ai_chat_history = rec.turns.clone();
                            self.ai_chat_session_id = rec.session_id.clone();
                            self.show_ai_chat = true;
                            self.log.push_back(LogEntry::info(format!(
                                "Resumed AI chat session {} ({} turns)",
                                rec.session_id,
                                rec.turns.len()
                            )));
                        }
                        Ok(None) => self
                            .log
                            .push_back(LogEntry::warn("No saved AI chat session to resume")),
                        Err(e) => self.log.push_back(LogEntry::err(format!("RESUMEAI: {e}"))),
                    }
                } else {
                    self.log
                        .push_back(LogEntry::warn("Cache not ready — wait a moment and retry"));
                }
            }
            "AISESSIONS" | "AI_SESSIONS" => {
                self.show_ai_sessions = true;
                if let Some(ref cache) = self.cache {
                    self.ai_sessions_index =
                        typhoon_engine::core::ai_sessions::read_index(cache).unwrap_or_default();
                }
                self.ai_sessions_last_refresh = chrono::Utc::now().timestamp();
            }
            "SCREENSHOTS" | "GALLERY" => {
                self.show_screenshots_gallery = true;
                self.scan_screenshots();
            }
            // ── ADR-162 cross-client AI response cache stats ──
            "AICACHE" | "AI_CACHE" | "AI_RESPONSE_CACHE" | "RESPONSE_CACHE" => {
                self.show_ai_cache = true;
                if let Some(ref cache) = self.cache {
                    self.ai_cache_stats =
                        typhoon_engine::core::ai_response_cache::stats(cache).unwrap_or_default();
                    self.ai_cache_recent =
                        typhoon_engine::core::ai_response_cache::recent_entries(cache, 50)
                            .unwrap_or_default();
                }
                self.ai_cache_last_refresh = chrono::Utc::now().timestamp();
            }
            // Investigation variants — open the window AND pre-load a research packet for the given symbols.
            cmd if cmd.starts_with("ASKAI ")
                || cmd.starts_with("ASK_AI ")
                || cmd.starts_with("INVESTIGATE ") =>
            {
                let args = cmd
                    .splitn(2, char::is_whitespace)
                    .nth(1)
                    .unwrap_or("")
                    .trim();
                let (syms, question) = Self::parse_ask_args(args);
                if syms.is_empty() {
                    self.show_ai_chat = true;
                    self.log.push_back(LogEntry::warn(
                        "Usage: ASKAI SYM1[,SYM2] [optional question]",
                    ));
                } else {
                    let packet = self.investigate_symbols(&syms, &question);
                    // Persist the packet so follow-up Sends still see the fundamentals
                    // (not just a "[Research packet: …]" placeholder in the history).
                    self.ai_chat_packet = Some(packet.clone());
                    self.show_ai_chat = true;
                    self.ai_chat_history.push((
                        true,
                        format!(
                            "[Research packet loaded: {}] {}",
                            syms.join(", "),
                            if question.is_empty() {
                                "Give me an overall read on these tickers.".to_string()
                            } else {
                                question.clone()
                            }
                        ),
                    ));
                    let first_turn = if question.is_empty() {
                        "Give me an overall read on these tickers — combine the research packet with live web search for recent news/sentiment.".to_string()
                    } else {
                        question.clone()
                    };
                    let (provider, key) = match self.ai_provider {
                        0 => ("claude", self.anthropic_key.clone()),
                        1 => ("openai", self.openai_key.clone()),
                        2 => ("gemini", self.gemini_key.clone()),
                        3 => ("grok", self.xai_key.clone()),
                        4 => ("mistral", self.mistral_key.clone()),
                        5 => ("perplexity", self.perplexity_key.clone()),
                        6 => ("local", "http://localhost:11434".to_string()),
                        _ => ("openai", self.openai_key.clone()),
                    };
                    if key.is_empty() && self.ai_provider != 6 {
                        self.ai_chat_history
                            .push((false, "Set API key in Settings first.".into()));
                    } else {
                        let _ = self.broker_tx.send(BrokerCmd::AiChat {
                            provider: provider.into(),
                            api_key: key,
                            message: first_turn,
                            history: Vec::new(), // fresh chain — packet is in the system prompt
                            system: Some(packet),
                            model: Some(self.ai_model.clone()),
                        });
                        self.log.push_back(LogEntry::info(format!(
                            "AI investigation dispatched: {} ({} symbols, {} backend, {})",
                            syms.join(", "),
                            syms.len(),
                            provider,
                            self.ai_model
                        )));
                    }
                }
            }
            cmd if cmd.starts_with("ASKCLAUDE ") || cmd.starts_with("ASK_CLAUDE ") => {
                let args = cmd
                    .splitn(2, char::is_whitespace)
                    .nth(1)
                    .unwrap_or("")
                    .trim();
                let (syms, question) = Self::parse_ask_args(args);
                if syms.is_empty() {
                    self.show_claude_code = true;
                    self.log.push_back(LogEntry::warn(
                        "Usage: ASKCLAUDE SYM1[,SYM2] [optional question]",
                    ));
                    return;
                }
                match std::process::Command::new("which").arg("claude").output() {
                    Ok(out) if out.status.success() => {
                        let packet = self.investigate_symbols(&syms, &question);
                        // Store the packet so follow-ups in the Claude Code window still
                        // have access to the same research context. `build_claude_prompt`
                        // re-injects it on every Send.
                        self.claude_code_packet = Some(packet.clone());
                        self.show_claude_code = true;
                        let first_user_turn = if question.is_empty() {
                            format!(
                                "Give me an overall read on {} — combine the research packet above with a live web search for recent news/sentiment.",
                                syms.join(", ")
                            )
                        } else {
                            question.clone()
                        };
                        self.claude_code_history.push((
                            true,
                            format!(
                                "[Research packet loaded: {}] {}",
                                syms.join(", "),
                                first_user_turn
                            ),
                        ));
                        if self.claude_code_rx.is_none() {
                            // Fresh session UUID — subsequent Sends in the window will --resume.
                            let session_id = Self::new_uuid();
                            self.claude_code_session_id = Some(session_id.clone());
                            let model = self.claude_model.clone();
                            let full_prompt = Self::build_claude_prompt(
                                Some(&packet),
                                &self.claude_code_history,
                                &first_user_turn,
                                &self.claude_effort,
                            );
                            let (tx, rx) = std::sync::mpsc::channel();
                            self.claude_code_rx = Some(rx);
                            Self::spawn_claude_print(model, session_id, true, full_prompt, tx);
                            self.log.push_back(LogEntry::info(format!(
                                "Claude Code investigation dispatched: {} ({} symbols, {} model)",
                                syms.join(", "),
                                syms.len(),
                                self.claude_model
                            )));
                        }
                    }
                    _ => {
                        self.log
                            .push_back(LogEntry::err("Claude Code CLI not found in PATH."));
                    }
                }
            }
            cmd if cmd.starts_with("ASKGEMINI ") || cmd.starts_with("ASK_GEMINI ") => {
                let args = cmd
                    .splitn(2, char::is_whitespace)
                    .nth(1)
                    .unwrap_or("")
                    .trim();
                let (syms, question) = Self::parse_ask_args(args);
                if syms.is_empty() {
                    self.show_gemini_cli = true;
                    self.log.push_back(LogEntry::warn(
                        "Usage: ASKGEMINI SYM1[,SYM2] [optional question]",
                    ));
                    return;
                }
                match std::process::Command::new("which").arg("gemini").output() {
                    Ok(out) if out.status.success() => {
                        let packet = self.investigate_symbols(&syms, &question);
                        self.gemini_cli_packet = Some(packet.clone());
                        self.show_gemini_cli = true;
                        let first_user_turn = if question.is_empty() {
                            format!(
                                "Give me an overall read on {} — combine the research packet above with a live web search for recent news/sentiment.",
                                syms.join(", ")
                            )
                        } else {
                            question.clone()
                        };
                        self.gemini_cli_history.push((
                            true,
                            format!(
                                "[Research packet loaded: {}] {}",
                                syms.join(", "),
                                first_user_turn
                            ),
                        ));
                        if self.gemini_cli_rx.is_none() {
                            let model = self.gemini_model.clone();
                            let full_prompt = Self::build_claude_prompt(
                                Some(&packet),
                                &self.gemini_cli_history,
                                &first_user_turn,
                                "",
                            );
                            let (tx, rx) = std::sync::mpsc::channel();
                            self.gemini_cli_rx = Some(rx);
                            Self::spawn_gemini_prompt(model, full_prompt, tx);
                            self.log.push_back(LogEntry::info(format!(
                                "Gemini CLI investigation dispatched: {} ({} symbols, {})",
                                syms.join(", "),
                                syms.len(),
                                self.gemini_model
                            )));
                        }
                    }
                    _ => {
                        self.log
                            .push_back(LogEntry::err("Gemini CLI not found in PATH."));
                    }
                }
            }
            cmd if cmd.starts_with("ASKHERMES ") || cmd.starts_with("ASK_HERMES ") => {
                let args = cmd
                    .splitn(2, char::is_whitespace)
                    .nth(1)
                    .unwrap_or("")
                    .trim();
                let (syms, question) = Self::parse_ask_args(args);
                if syms.is_empty() {
                    self.show_hermes_cli = true;
                    self.log.push_back(LogEntry::warn(
                        "Usage: ASKHERMES SYM1[,SYM2] [optional question]",
                    ));
                    return;
                }
                match std::process::Command::new("which").arg("hermes").output() {
                    Ok(out) if out.status.success() => {
                        let packet = self.investigate_symbols(&syms, &question);
                        self.hermes_cli_packet = Some(packet.clone());
                        self.show_hermes_cli = true;
                        let first_user_turn = if question.is_empty() {
                            format!(
                                "Give me an overall read on {} — combine the research packet above with live web search for recent news/sentiment.",
                                syms.join(", ")
                            )
                        } else {
                            question.clone()
                        };
                        self.hermes_cli_history.push((
                            true,
                            format!(
                                "[Research packet loaded: {}] {}",
                                syms.join(", "),
                                first_user_turn
                            ),
                        ));
                        if self.hermes_cli_rx.is_none() {
                            let full_prompt = Self::build_claude_prompt(
                                Some(&packet),
                                &self.hermes_cli_history,
                                &first_user_turn,
                                "",
                            );
                            let model = self.hermes_model.clone();
                            let provider = self.hermes_provider.clone();
                            let (tx, rx) = std::sync::mpsc::channel();
                            self.hermes_cli_rx = Some(rx);
                            Self::spawn_hermes_exec(model, provider, full_prompt, tx);
                            self.log.push_back(LogEntry::info(format!(
                                "Hermes Agent investigation dispatched: {} ({} symbols{})",
                                syms.join(", "),
                                syms.len(),
                                if self.hermes_model.trim().is_empty() {
                                    "".to_string()
                                } else {
                                    format!(", {}", self.hermes_model)
                                }
                            )));
                        }
                    }
                    _ => {
                        self.log
                            .push_back(LogEntry::err("Hermes Agent CLI not found in PATH."));
                    }
                }
            }
            cmd if cmd.starts_with("ASKCODEX ") || cmd.starts_with("ASK_CODEX ") => {
                let args = cmd
                    .splitn(2, char::is_whitespace)
                    .nth(1)
                    .unwrap_or("")
                    .trim();
                let (syms, question) = Self::parse_ask_args(args);
                if syms.is_empty() {
                    self.show_codex_cli = true;
                    self.log.push_back(LogEntry::warn(
                        "Usage: ASKCODEX SYM1[,SYM2] [optional question]",
                    ));
                    return;
                }
                match std::process::Command::new("which").arg("codex").output() {
                    Ok(out) if out.status.success() => {
                        let packet = self.investigate_symbols(&syms, &question);
                        self.codex_cli_packet = Some(packet.clone());
                        self.show_codex_cli = true;
                        let first_user_turn = if question.is_empty() {
                            format!(
                                "Give me an overall read on {} — combine the research packet above with a live web search for recent news/sentiment.",
                                syms.join(", ")
                            )
                        } else {
                            question.clone()
                        };
                        self.codex_cli_history.push((
                            true,
                            format!(
                                "[Research packet loaded: {}] {}",
                                syms.join(", "),
                                first_user_turn
                            ),
                        ));
                        if self.codex_cli_rx.is_none() {
                            let model = self.codex_model.clone();
                            let reasoning_effort = self.codex_reasoning_effort.clone();
                            let full_prompt = Self::build_claude_prompt(
                                Some(&packet),
                                &self.codex_cli_history,
                                &first_user_turn,
                                "",
                            );
                            let (tx, rx) = std::sync::mpsc::channel();
                            self.codex_cli_rx = Some(rx);
                            Self::spawn_codex_exec(model, reasoning_effort, full_prompt, tx);
                            self.log.push_back(LogEntry::info(format!(
                                "Codex CLI investigation dispatched: {} ({} symbols, {}, {})",
                                syms.join(", "),
                                syms.len(),
                                self.codex_model,
                                Self::codex_reasoning_effort_label(&self.codex_reasoning_effort)
                            )));
                        }
                    }
                    _ => {
                        self.log
                            .push_back(LogEntry::err("Codex CLI not found in PATH."));
                    }
                }
            }
            "CHAT" | "MATRIX" => {
                self.show_matrix_chat = true;
                if self.matrix_access_token.is_empty() || self.matrix_access_token == "none" {
                    self.log.push_back(LogEntry::warn(
                        "Matrix: no access token — set it in Settings",
                    ));
                }
            }
            "WSB" | "REDDIT" | "WALLSTREETBETS" => {
                self.show_reddit = true;
                if self.reddit_posts.is_empty() {
                    let _ = self.broker_tx.send(BrokerCmd::FetchRedditWSB);
                }
            }
            "BARDATA" | "FETCH_ALL" | "FULL_HISTORY" => {
                // Download ALL available bars for ALL symbols from ALL connected brokers
                // Collects: chart tab symbols, watchlist symbols, DARWIN position symbols, Alpaca positions
                let all_tfs =
                    self.filtered_sync_timeframes(["1Day", "1Week", "1Hour", "4Hour", "1Month"]);
                let mut symbols: std::collections::HashSet<String> =
                    std::collections::HashSet::new();

                // Chart tab symbols
                for chart in &self.charts {
                    let bare = chart.symbol.split(':').last().unwrap_or("").to_string();
                    if !bare.is_empty() {
                        symbols.insert(bare);
                    }
                }
                // Watchlist symbols
                for sym in &self.user_watchlist {
                    if !sym.is_empty() {
                        symbols.insert(sym.clone());
                    }
                }
                // DARWIN/MT5 position symbols
                for pos in &self.bg.open_positions {
                    let sym = pos.symbol.replace('/', "");
                    if !sym.is_empty() {
                        symbols.insert(sym);
                    }
                }
                // Alpaca position symbols
                for pos in &self.live_positions {
                    if !pos.symbol.is_empty() {
                        symbols.insert(pos.symbol.clone());
                    }
                }
                // tastytrade position symbols
                for pos in &self.tt_positions {
                    if !pos.symbol.is_empty() {
                        symbols.insert(pos.symbol.clone());
                    }
                }
                // Full Alpaca broker universe (12K+ symbols)
                for (sym, _name, _class) in &self.all_broker_assets {
                    symbols.insert(sym.replace('/', "").to_uppercase());
                }
                // Kraken tradeable pairs
                for (pair, _name) in &self.kraken_pairs {
                    symbols.insert(pair.clone());
                }
                // Kraken Futures instruments
                for symbol in &self.kraken_futures_symbols {
                    symbols.insert(symbol.clone());
                }

                if symbols.is_empty() {
                    self.log.push_back(LogEntry::warn(
                        "BARDATA: no symbols to fetch — open charts or add to watchlist first",
                    ));
                } else {
                    let crypto_bases = [
                        "BTC", "ETH", "SOL", "DOGE", "XRP", "ADA", "LTC", "LINK", "AVAX", "DOT",
                        "XMR", "ZEC", "DASH", "UNI", "AAVE", "MATIC", "SHIB", "ATOM", "ALGO",
                        "FTM", "NEAR", "APE", "ARB",
                    ];

                    // Build set of already-cached symbol:TF combos to skip redundant fetches
                    let mut cached_keys: std::collections::HashSet<String> =
                        std::collections::HashSet::new();
                    for (key, bars, _ts) in &self.bg.detailed_stats {
                        if *bars > 0 {
                            // any cached data = don't re-download full history
                            // Normalize: extract bare symbol + TF from cache key
                            let parts: Vec<&str> = key.split(':').collect();
                            if parts.len() >= 2 {
                                let sym_part =
                                    parts[parts.len() - 2].replace('/', "").to_uppercase();
                                let tf_part = parts[parts.len() - 1];
                                cached_keys.insert(format!("{}:{}", sym_part, tf_part));
                            }
                        }
                    }

                    // Partition: uncached first, then partially cached
                    let mut uncached_syms = Vec::new();
                    let mut cached_syms = Vec::new();
                    for sym in &symbols {
                        let su = sym.to_uppercase();
                        let has_any = all_tfs
                            .iter()
                            .any(|tf| cached_keys.contains(&format!("{}:{}", su, tf)));
                        if has_any {
                            cached_syms.push(sym.clone());
                        } else {
                            uncached_syms.push(sym.clone());
                        }
                    }

                    let mut fetched_count = 0;
                    let mut skipped_count = 0;
                    let db_path = cache_db_path();

                    // Process uncached symbols first (highest priority)
                    for sym in uncached_syms.iter().chain(cached_syms.iter()) {
                        let su = sym.to_uppercase();
                        let is_crypto = crypto_bases
                            .iter()
                            .any(|b| su.starts_with(b) && su.ends_with("USD"));
                        let is_kraken_futures =
                            typhoon_engine::core::kraken_futures::is_futures_symbol(&su);

                        // Find which TFs are missing for this symbol
                        let missing_tfs: Vec<String> = all_tfs
                            .iter()
                            .filter(|tf| !cached_keys.contains(&format!("{}:{}", su, tf)))
                            .cloned()
                            .collect();

                        if missing_tfs.is_empty() {
                            skipped_count += 1;
                            continue; // fully cached, skip entirely
                        }

                        if is_kraken_futures {
                            if self.kraken_scrape_futures {
                                let _ = self.broker_tx.send(BrokerCmd::KrakenFuturesBackfill {
                                    symbol: su.clone(),
                                    timeframes: missing_tfs.clone(),
                                    db_path: db_path.clone(),
                                    backfill_complete: false,
                                });
                                fetched_count += missing_tfs.len();
                            } else {
                                skipped_count += 1;
                                continue;
                            }
                        } else if is_crypto {
                            // Crypto: use Kraken public market data.
                            // Normalize: remove slashes, uppercase (BTC/USD → BTCUSD)
                            let clean_sym = sym.replace('/', "").to_uppercase();
                            if self.kraken_spot_symbol_scrape_enabled(&clean_sym) {
                                let _ = self.broker_tx.send(BrokerCmd::KrakenBackfill {
                                    symbol: clean_sym.clone(),
                                    timeframes: missing_tfs.clone(),
                                    db_path: db_path.clone(),
                                    backfill_complete: false,
                                });
                                fetched_count += missing_tfs.len();
                            }
                        } else if self.broker_connected {
                            // Stocks/Forex/CFDs: use Alpaca (AlpacaFetchBars, with MT5 priority + full-history first fetch)
                            for tf in &missing_tfs {
                                self.queue_alpaca_fetch(&sym, tf);
                            }
                        }

                        // tastytrade: bars + option chain (if connected and not already cached)
                        if self.tt_connected {
                            for tf in &missing_tfs {
                                let _ = self.broker_tx.send(BrokerCmd::TastyTradeFetchBars {
                                    symbol: sym.clone(),
                                    timeframe: tf.clone(),
                                    backfill_complete: false,
                                });
                                fetched_count += 1;
                            }
                            let _ = self.broker_tx.send(BrokerCmd::TastytradeOptionChain {
                                symbol: sym.clone(),
                            });
                        }
                        // Count individual TF fetches (not symbols) to match completion counter
                        if !is_kraken_futures && !is_crypto && self.broker_connected {
                            fetched_count += missing_tfs.len(); // Alpaca FetchAllBars per TF
                        }
                    }

                    // Update progress tracking and open window
                    self.bardata_total = symbols.len();
                    self.bardata_queued = fetched_count;
                    self.bardata_skipped = skipped_count;
                    self.bardata_completed = 0;
                    self.bardata_log.clear();
                    for line in [
                        format!("BARDATA: total symbols: {}", symbols.len()),
                        format!("BARDATA: queued for download: {}", fetched_count),
                        format!("BARDATA: already cached (skipped): {}", skipped_count),
                        format!(
                            "BARDATA: uncached priority symbols: {}",
                            uncached_syms.len()
                        ),
                    ] {
                        self.bardata_log.push_back(line.clone());
                        self.log.push_back(LogEntry::info(line));
                    }
                    self.show_bardata = true;
                    self.bardata_active = true;
                }
            }
            "INDICES" | "WORLD_INDICES" => {
                self.show_world_indices = true;
                let symbols = vec![
                    "DIA", "SPY", "QQQ", "IWM", "EFA", "EEM", "VGK", "EWJ", "FXI", "EWZ", "GLD",
                    "SLV", "USO", "TLT", "UUP", "BTCUSD",
                ]
                .into_iter()
                .map(String::from)
                .collect();
                let _ = self
                    .broker_tx
                    .send(BrokerCmd::GetWatchlistQuotes { symbols });
                self.log
                    .push_back(LogEntry::info("Fetching world indices quotes..."));
            }
            "CRYPTO50" | "CRYPTO_TOP50" => {
                self.show_crypto_top50 = true;
                let _ = self.broker_tx.send(BrokerCmd::FetchCryptoTop50);
                self.log
                    .push_back(LogEntry::info("Fetching CoinGecko top 50..."));
            }
            "FOREX" | "FOREX_MATRIX" => {
                self.show_forex_matrix = true;
                let symbols = vec![
                    "EURUSD", "GBPUSD", "USDJPY", "USDCHF", "AUDUSD", "NZDUSD", "USDCAD", "EURGBP",
                    "EURJPY", "GBPJPY",
                ]
                .into_iter()
                .map(String::from)
                .collect();
                let _ = self
                    .broker_tx
                    .send(BrokerCmd::GetWatchlistQuotes { symbols });
                self.log
                    .push_back(LogEntry::info("Fetching forex pairs..."));
            }
            "KRAKEN" => {
                self.show_settings = true;
                self.log.push_back(LogEntry::info(
                    "Open Settings to configure Kraken API credentials",
                ));
            }
            "KRAKEN_TRADES" | "KRAKENTRADES" | "KRAKEN_HISTORY" => {
                self.show_kraken_trade_history = true;
                let _ = self.broker_tx.send(BrokerCmd::KrakenFetchTrades);
                self.log
                    .push_back(LogEntry::info("Kraken: refreshing trade history"));
            }
            "KRAKEN_ORDERS" | "KRAKENORDERS" | "KRAKEN_OPEN_ORDERS" => {
                self.show_kraken_open_orders = true;
                let _ = self.broker_tx.send(BrokerCmd::KrakenFetchOpenOrders);
                self.log
                    .push_back(LogEntry::info("Kraken: refreshing open orders"));
            }
            "KRAKEN_FUTURES" | "KRAKENFUTURES" => {
                let _ = self.broker_tx.send(BrokerCmd::KrakenFuturesGetInstruments);
                self.kraken_futures_requested = true;
                self.log.push_back(LogEntry::info(
                    "Kraken Futures: loading public instrument universe",
                ));
            }
            "PREV_LEVELS" => self.show_prev_levels = !self.show_prev_levels,
            // Trading
            "OPEN_TRADE" => {
                self.submit_quick_trade();
            }
            "EXPORT_CALENDAR" => {
                if self.event_calendar_rows.is_empty() {
                    self.log
                        .push_back(LogEntry::warn("No events loaded — open CALENDAR first"));
                } else {
                    let ics = Self::build_events_ics(
                        &self.event_calendar_rows,
                        self.event_filter_source,
                        true,
                        true,
                        true,
                    );
                    let mut path = dirs_home();
                    path.push("export");
                    let _ = std::fs::create_dir_all(&path);
                    path.push("typhoon_events.ics");
                    match std::fs::write(&path, &ics) {
                        Ok(_) => self.log.push_back(LogEntry::info(format!(
                            "Calendar exported: {} ({} bytes)",
                            path.display(),
                            ics.len()
                        ))),
                        Err(e) => self
                            .log
                            .push_back(LogEntry::err(format!("ICS export failed: {e}"))),
                    }
                }
            }
            cmd if cmd.starts_with("BACKTEST_EXPAND") => {
                let rest = cmd.trim_start_matches("BACKTEST_EXPAND").trim();
                if rest.is_empty() {
                    if self.mt5_backtest_expand_symbols.is_empty() {
                        self.log.push_back(LogEntry::info(
                            "backtest_expand: empty. Usage: BACKTEST_EXPAND EURUSD [bars]  (compatibility override; provider-max MT5 sync is already the default)"));
                    } else {
                        let mut list: Vec<(String, u32)> = self
                            .mt5_backtest_expand_symbols
                            .iter()
                            .map(|(k, v)| (k.clone(), *v))
                            .collect();
                        list.sort_by(|a, b| a.0.cmp(&b.0));
                        let shown = list
                            .iter()
                            .map(|(s, n)| format!("{}={}", s, n))
                            .collect::<Vec<_>>()
                            .join(", ");
                        self.log
                            .push_back(LogEntry::info(format!("backtest_expand map: {}", shown)));
                    }
                } else {
                    let parts: Vec<&str> = rest.split_whitespace().collect();
                    let sym = parts[0].to_uppercase();
                    // MT5 sync already asks for provider-maximum history by default.
                    // Keep this command as a compatibility knob for old saved sessions/manual
                    // experiments, but never let it shrink below the provider-max sentinel.
                    let default_bars: u32 = MT5_PROVIDER_MAX_BARS;
                    let cap: u32 = MT5_PROVIDER_MAX_BARS;
                    let bars: u32 = if parts.len() >= 2 {
                        parts[1].parse::<u32>().unwrap_or(default_bars).min(cap)
                    } else {
                        default_bars
                    };
                    self.mt5_backtest_expand_symbols.insert(sym.clone(), bars);
                    self.log.push_back(LogEntry::info(format!(
                        "backtest_expand: {} → {} bars (overrides tiered default on gap-fill requests)",
                        sym, bars)));
                    self.detect_mt5_gaps();
                    self.flush_mt5_demand_txt(true);
                }
            }
            cmd if cmd.starts_with("BACKTEST_UNEXPAND") => {
                let rest = cmd.trim_start_matches("BACKTEST_UNEXPAND").trim();
                if rest.is_empty() {
                    self.log
                        .push_back(LogEntry::warn("Usage: BACKTEST_UNEXPAND EURUSD"));
                } else {
                    let sym = rest.to_uppercase();
                    if self.mt5_backtest_expand_symbols.remove(&sym).is_some() {
                        self.log.push_back(LogEntry::info(format!(
                            "backtest_expand: removed {} — provider-max MT5 sync remains the default",
                            sym
                        )));
                    } else {
                        self.log.push_back(LogEntry::info(format!(
                            "backtest_expand: {} not in set",
                            sym
                        )));
                    }
                }
            }
            cmd if cmd.starts_with("OCO ") => {
                // OCO SELL AAPL 10 200.00 180.00
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.len() >= 6 {
                    let side = parts[1].to_lowercase();
                    let symbol = parts[2].to_string();
                    let qty: f64 = parts[3].parse().unwrap_or(0.0);
                    let tp: f64 = parts[4].parse().unwrap_or(0.0);
                    let sl: f64 = parts[5].parse().unwrap_or(0.0);
                    if qty > 0.0 && tp > 0.0 && sl > 0.0 {
                        let _ = self.broker_tx.send(BrokerCmd::AlpacaOcoOrder {
                            symbol: symbol.clone(),
                            qty,
                            side: side.clone(),
                            tp_price: tp,
                            sl_price: sl,
                        });
                        self.log.push_back(LogEntry::info(format!(
                            "OCO {} {} {} TP:{} SL:{}",
                            side, qty, symbol, tp, sl
                        )));
                    } else {
                        self.log.push_back(LogEntry::warn(
                            "Invalid OCO params — need positive qty, TP, SL",
                        ));
                    }
                } else {
                    self.log
                        .push_back(LogEntry::warn("Usage: OCO SELL AAPL 10 200.00 180.00"));
                }
            }
            "CLOSE_ALL" => {
                self.close_all_selected_brokers();
            }
            "CLOSE_PARTIAL" => {
                self.close_partial_active_symbol();
            }
            "SET_SL" => {
                // Use last close price as initial SL, then user can drag
                if let Some(chart) = self.charts.get(self.active_tab) {
                    if let Some(last) = chart.bars.last() {
                        let sl = last.close * 0.98; // default: 2% below current price
                        self.sl_price = Some(sl);
                        self.sl_enabled = true;
                        self.sync_trade_line_inputs();
                        self.log.push_back(LogEntry::info(format!(
                            "SL set at {} — drag to adjust",
                            format_price(sl)
                        )));
                    }
                }
            }
            "SET_TP" => {
                if let Some(chart) = self.charts.get(self.active_tab) {
                    if let Some(last) = chart.bars.last() {
                        let tp = last.close * 1.04; // default: 4% above current price
                        self.tp_price = Some(tp);
                        self.tp_enabled = true;
                        self.sync_trade_line_inputs();
                        self.log.push_back(LogEntry::info(format!(
                            "TP set at {} — drag to adjust",
                            format_price(tp)
                        )));
                    }
                }
            }
            "OPEN_MG" => {
                if self.broker_connected {
                    self.log.push_back(LogEntry::info(
                        "Martingale: use chart SL/TP lines and the broker-backed Open MG flow",
                    ));
                } else {
                    self.log
                        .push_back(LogEntry::warn("Connect to broker first"));
                }
            }
            "BUY_LINES" | "SELL_LINES" => {
                let is_buy = cmd == "BUY_LINES";
                match self.set_visible_range_trade_lines(is_buy) {
                    Ok((sl, tp)) => {
                        self.log.push_back(LogEntry::info(format!(
                            "{}: SL {} TP {} (drag to adjust)",
                            if is_buy { "Buy Lines" } else { "Sell Lines" },
                            format_price(sl),
                            format_price(tp)
                        )));
                    }
                    Err(e) => self.log.push_back(LogEntry::warn(e)),
                }
            }
            "TEMPLATES" | "LIST_TEMPLATES" => {
                let builtins = ["NNFX", "CLEAN", "FULL"];
                let mut names: Vec<String> = builtins
                    .iter()
                    .map(|s| format!("{} (built-in)", s))
                    .collect();
                for k in self.chart_templates.keys() {
                    if !builtins.contains(&k.as_str()) {
                        names.push(k.clone());
                    }
                }
                names.sort();
                self.log
                    .push_back(LogEntry::info(format!("Templates: {}", names.join(", "))));
            }
            // ADR-092: UX improvement commands
            "COMPACT" => {
                self.compact_mode = !self.compact_mode;
                if self.compact_mode {
                    self.show_rsi = false;
                    self.show_fisher = false;
                    self.show_macd = false;
                    self.show_stochastic = false;
                    self.show_adx = false;
                    self.show_volume_pane = false;
                    self.show_better_volume = false;
                    self.log
                        .push_back(LogEntry::info("Compact mode ON — sub-panes hidden"));
                } else {
                    self.show_fisher = true;
                    self.show_better_volume = true;
                    self.log.push_back(LogEntry::info(
                        "Compact mode OFF — default indicators restored",
                    ));
                }
            }
            "RULER" => {
                self.log.push_back(LogEntry::info(
                    "Ruler: use trendline (Alt+T) to measure price/time distance",
                ));
            }
            other => {
                // Commands with arguments
                if other.starts_with("DELETE_DARWIN ") {
                    let ticker = other
                        .splitn(2, ' ')
                        .nth(1)
                        .unwrap_or("")
                        .trim()
                        .to_uppercase();
                    if ticker.is_empty() {
                        self.log.push_back(LogEntry::warn(
                            "Usage: DELETE_DARWIN TICKER (e.g. DELETE_DARWIN CKUC)",
                        ));
                    } else {
                        // Immediately remove from in-memory UI state
                        self.bg.accounts.retain(|a| a.darwin_ticker != ticker);
                        self.bg.account_details.retain(|d| d.ticker != ticker);

                        // Write blacklist + update KV (fast)
                        if let Some(ref cache) = self.cache {
                            let _ = cache.put_kv(&format!("darwin:deleted:{}", ticker), "1");
                            let _ = cache.put_kv(
                                "darwin:account_details",
                                &serde_json::to_string(&self.bg.account_details)
                                    .unwrap_or_default(),
                            );

                            // Offload SQL DELETE through the app runtime's blocking pool.
                            let cache = cache.clone();
                            let ticker_clone = ticker.clone();
                            self.rt_handle.spawn_blocking(move || {
                                if let Ok(conn) = cache.connection() {
                                    let _ = typhoon_engine::core::darwin::delete_darwin_account(
                                        &conn,
                                        &ticker_clone,
                                    );
                                }
                            });
                        }
                        self.log.push_back(LogEntry::info(format!(
                            "Deleting DARWIN {} (background)...",
                            ticker
                        )));
                    }
                } else if other.starts_with("SAVE_TEMPLATE ") || other.starts_with("TEMPLATE_SAVE ")
                {
                    let name = other.splitn(2, ' ').nth(1).unwrap_or("").trim().to_string();
                    if name.is_empty() {
                        self.log
                            .push_back(LogEntry::warn("Usage: SAVE_TEMPLATE <name>"));
                    } else {
                        let snap = self.capture_indicator_snapshot();
                        self.chart_templates.insert(name.clone(), snap);
                        self.save_session();
                        self.log
                            .push_back(LogEntry::info(format!("Template '{}' saved", name)));
                    }
                } else if other.starts_with("LOAD_TEMPLATE ") || other.starts_with("TEMPLATE ") {
                    let name = other.splitn(2, ' ').nth(1).unwrap_or("").trim().to_string();
                    if name.is_empty() {
                        self.log
                            .push_back(LogEntry::warn("Usage: LOAD_TEMPLATE <name>"));
                    } else {
                        // Check built-in presets first
                        let template = match name.as_str() {
                            "NNFX" => Some(Self::builtin_template_nnfx()),
                            "CLEAN" => Some(Self::builtin_template_clean()),
                            "FULL" => Some(Self::builtin_template_full()),
                            _ => self.chart_templates.get(&name).cloned(),
                        };
                        if let Some(snap) = template {
                            self.apply_indicator_snapshot(&snap);
                            self.log
                                .push_back(LogEntry::info(format!("Template '{}' loaded", name)));
                        } else {
                            self.log.push_back(LogEntry::warn(format!(
                                "Template '{}' not found",
                                name
                            )));
                        }
                    }
                } else if other.starts_with("DELETE_TEMPLATE ") {
                    let name = other.splitn(2, ' ').nth(1).unwrap_or("").trim().to_string();
                    if self.chart_templates.remove(&name).is_some() {
                        self.save_session();
                        self.log
                            .push_back(LogEntry::info(format!("Template '{}' deleted", name)));
                    } else {
                        self.log
                            .push_back(LogEntry::warn(format!("Template '{}' not found", name)));
                    }
                } else if other.starts_with("WORKSPACE_SAVE ") {
                    let name = other.splitn(2, ' ').nth(1).unwrap_or("").trim().to_string();
                    if name.is_empty() {
                        self.log
                            .push_back(LogEntry::warn("Usage: WORKSPACE_SAVE <name>"));
                    } else {
                        let snap = self.capture_workspace_snapshot();
                        if let Ok(json) = serde_json::to_string(&snap) {
                            self.workspaces.insert(name.clone(), json);
                            self.save_session();
                            self.log
                                .push_back(LogEntry::info(format!("Workspace '{}' saved", name)));
                        }
                    }
                } else if other.starts_with("WORKSPACE_LOAD ") {
                    let name = other.splitn(2, ' ').nth(1).unwrap_or("").trim().to_string();
                    // Check user-saved first, then builtin presets
                    if let Some(json) = self.workspaces.get(&name).cloned() {
                        if let Ok(snap) = serde_json::from_str::<serde_json::Value>(&json) {
                            self.apply_workspace_snapshot(&snap);
                            self.log
                                .push_back(LogEntry::info(format!("Workspace '{}' loaded", name)));
                        }
                    } else if let Some(snap) = Self::builtin_workspace(&name) {
                        self.apply_workspace_snapshot(&snap);
                        self.log.push_back(LogEntry::info(format!(
                            "Built-in workspace '{}' loaded",
                            name.to_uppercase()
                        )));
                    } else {
                        self.log.push_back(LogEntry::warn(format!(
                            "Workspace '{}' not found (try TRADING/RESEARCH/DARWIN/COMPACT)",
                            name
                        )));
                    }
                } else if other == "WORKSPACES" {
                    let mut all: Vec<String> = self.workspaces.keys().cloned().collect();
                    all.extend([
                        "TRADING (built-in)".into(),
                        "RESEARCH (built-in)".into(),
                        "DARWIN (built-in)".into(),
                        "COMPACT (built-in)".into(),
                    ]);
                    self.log
                        .push_back(LogEntry::info(format!("Workspaces: {}", all.join(", "))));
                } else {
                    self.log
                        .push_back(LogEntry::warn(format!("Unknown command: {}", other)));
                }
            }
        }
    }

    // ── Chart template helpers ────────────────────────────────────────────────
}
