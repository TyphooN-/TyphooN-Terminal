use super::*;

mod ai_commands;
mod chart_drawing;
mod darwinex_commands;
mod market_data_commands;
mod outlier_scan_commands;
mod trade_order_commands;

impl TyphooNApp {
    pub(super) fn handle_command(&mut self, cmd: &str, ctx: &egui::Context) {
        let cmd_upper = cmd.trim().trim_start_matches('/').to_uppercase();
        self.log
            .push_back(LogEntry::info(format!("CMD: {}", cmd_upper)));
        if let Some(symbol) = cmd_upper.strip_prefix("BOOKMAP ") {
            self.open_bookmap_window(Some(symbol.trim().to_string()));
            return;
        }
        if self.handle_research_window_command(&cmd_upper) {
            return;
        }
        if self.handle_chart_drawing_command(&cmd_upper, ctx) {
            return;
        }
        if self.handle_ai_command(&cmd_upper) {
            return;
        }
        if self.handle_market_data_command(&cmd_upper) {
            return;
        }
        if self.handle_trade_order_command(&cmd_upper) {
            return;
        }
        if self.handle_darwinex_command(&cmd_upper) {
            return;
        }
        if self.handle_outlier_scan_command(&cmd_upper) {
            return;
        }
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
            // ── web article ingestion + packet viewer ──
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
                    kraken_equity_symbols: self.kraken_equity_universe_symbols.clone(),
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
            "BOOKMAP" => self.open_bookmap_window(None),
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
                let sym = self.symbol_input.clone();
                self.queue_open_symbol_sync_all_timeframes(&sym);
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
                // Main chart: ATR_Projection + PreviousCandleLevels + MultiKAMA + MTF_MA + SupplyDemand
                self.show_atr_proj = true;
                self.show_prev_levels = true;
                self.show_kama = true;
                self.show_sma200 = true;
                self.show_supply_demand = true;
                self.show_auto_fib = false;
                // Sub-pane 1: EhlersFisherTransform | Sub-pane 2: BetterVolume
                self.show_fisher = true;
                self.show_better_volume = true;
                self.log.push_back(LogEntry::info(
                    "NNFX preset: ATR_Proj + PrevLevels + MultiKAMA + MTF_MA + S/D + Fisher + BVol",
                ));
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
