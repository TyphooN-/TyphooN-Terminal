use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_fundamental_event_command(&mut self, cmd_upper: &str) -> bool {
        match cmd_upper {
            "FUNDAMENTALS" => self.show_fundamentals = true,
            "EV" => self.show_ev_scanner = true,
            "EARNINGS" => self.show_earnings_calendar = true,
            "DIVIDENDS" => self.show_dividend_calendar = true,
            s if s == "EVSCRAPE" || s == "EVSCRAPE FORCE" => {
                let force = s.ends_with("FORCE");
                let db_path = cache_db_path();
                // Broker scope override: narrow sources to just the scoped broker.
                // SCOPE ALL → use configured source toggles. SCOPE ALPACA → force use_alpaca only, etc.
                let (use_alpaca, use_kraken) = match self.broker_scope {
                    EventSource::All => (self.fund_source_alpaca, self.fund_source_kraken),
                    EventSource::Alpaca => (true, false),
                    EventSource::Kraken => (false, true),
                    EventSource::Positions => (self.fund_source_alpaca, self.fund_source_kraken),
                };
                let _ = self.broker_tx.send(BrokerCmd::FundamentalsScrape {
                    db_path,
                    use_alpaca,
                    use_kraken,
                    kraken_equity_symbols: self.kraken_equity_universe_symbols.clone(),
                    force,
                });
                self.scrape_fund_running = true;
                self.scrape_fund_ok = 0;
                self.scrape_fund_fail = 0;
                self.scrape_fund_skipped = 0;
                let sources: Vec<&str> = [("Alpaca", use_alpaca), ("Kraken", use_kraken)]
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
                    self.show_option_chain = true;
                    self.log.push_back(LogEntry::info(format!(
                        "Fetching option chain for {}...",
                        self.option_chain_sym
                    )));
                }
            }
            "HOLDERS" => self.show_holders = true,
            "COMPILE" => self.show_indicator_compiler = true,
            "CORRELATION" => self.show_correlation = true,
            "SEASONALS" => self.show_seasonals = true,
            "MONTECARLO" => self.show_montecarlo = true,
            "STRESS_TEST" => self.show_stress_test = true,
            "VOLUME_PROFILE" => self.show_volume_profile = true,
            "HV_CONE" => self.show_hv_cone = true,
            "SECTOR_HEATMAP" => self.show_sector_heatmap = true,
            "DIVSCREEN" => self.show_dividends = true,
            "COMPANY" | "DES" | "PROFILE" => {
                let sym = self
                    .active_trade_symbol()
                    .unwrap_or_default()
                    .to_uppercase();
                if sym.is_empty() {
                    self.company_info_symbol.clear();
                    self.company_info_text = "No active chart symbol.".to_string();
                    self.show_company_info_window = true;
                    return true;
                }

                self.company_info_symbol = sym.clone();
                let cached_profile = self
                    .cache
                    .as_ref()
                    .and_then(|cache| cache.connection().ok())
                    .and_then(|conn| {
                        typhoon_engine::core::research::get_profile(&conn, &sym)
                            .ok()
                            .flatten()
                    });

                if let Some(profile) = cached_profile {
                    self.company_info_text =
                        typhoon_engine::core::research::get_company_summary(&profile);
                } else {
                    self.company_info_text = if self.finnhub_key.is_empty() {
                        format!(
                            "No cached profile for {sym}. Add a Finnhub key and run PROFILE again or use EVSCRAPE."
                        )
                    } else {
                        let _ = self.broker_tx.send(BrokerCmd::FetchCompanyProfile {
                            symbol: sym.clone(),
                            finnhub_key: self.finnhub_key.clone(),
                        });
                        format!(
                            "No cached profile for {sym}. Fetch requested; reopen PROFILE after it completes."
                        )
                    };
                }
                self.show_company_info_window = true;
            }
            s if s.starts_with("SCOPE") => {
                // SCOPE [ALL|ALPACA|KRAKEN] — global broker filter for fundamentals.
                let arg = s.trim_start_matches("SCOPE").trim();
                let (new_scope, label) = match arg {
                    "" => {
                        // No arg: open SCOPE popup window
                        self.show_scope_window = true;
                        return true;
                    }
                    "ALL" => (EventSource::All, "ALL"),
                    "ALPACA" => (EventSource::Alpaca, "ALPACA"),
                    "KRAKEN" | "KR" => (EventSource::Kraken, "KRAKEN"),
                    "POSITIONS" | "POS" => (EventSource::Positions, "POSITIONS"),
                    other => {
                        self.log.push_back(LogEntry::err(format!(
                            "Unknown SCOPE '{other}'. Valid: ALL, ALPACA, KRAKEN, POSITIONS"
                        )));
                        return true;
                    }
                };
                self.broker_scope = new_scope;
                // Sync fund_source toggles with scope
                match new_scope {
                    EventSource::All => {
                        self.fund_source_alpaca = true;
                        self.fund_source_kraken = true;
                    }
                    EventSource::Alpaca => {
                        self.fund_source_alpaca = true;
                        self.fund_source_kraken = false;
                    }
                    EventSource::Kraken => {
                        self.fund_source_alpaca = false;
                        self.fund_source_kraken = true;
                    }
                    EventSource::Positions => {
                        self.fund_source_alpaca = true;
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
                // fundamentals, tags each row by broker tradeability (Alpaca / Kraken).
                use chrono::NaiveDate;
                let today = chrono::Utc::now().date_naive();

                // Active symbol sets (bare tickers, uppercased).
                let alpaca_syms: std::collections::HashSet<String> = self
                    .live_positions
                    .iter()
                    .map(|p| p.symbol.replace('/', "").to_uppercase())
                    .collect();
                let kraken_syms = self.kraken_scope_symbols();

                let parse_date = |s: &str| -> Option<NaiveDate> {
                    NaiveDate::parse_from_str(s, "%Y-%m-%d")
                        .ok()
                        .or_else(|| NaiveDate::parse_from_str(s, "%Y/%m/%d").ok())
                };

                let mut rows: Vec<EventRow> = Vec::new();
                for f in &self.bg.all_fundamentals {
                    let sym_u = f.symbol.to_uppercase();
                    let in_alpaca = alpaca_syms.contains(&sym_u);
                    let in_kraken = kraken_syms.contains(&sym_u);
                    if !in_alpaca && !in_kraken {
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
                    "Event Calendar: {} upcoming events | Alpaca {} • Kraken {}",
                    rows.len(),
                    alpaca_syms.len(),
                    kraken_syms.len()
                )));
                if rows.is_empty() {
                    self.log.push_back(LogEntry::warn("No events found. Run EVSCRAPE/FUNDAMENTALS first to populate earnings/dividend dates."));
                }
                self.event_calendar_rows = rows;
                self.show_event_calendar = true;
            }
            _ => return false,
        }
        true
    }
}
