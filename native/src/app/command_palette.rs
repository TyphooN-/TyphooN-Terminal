use super::*;

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
                        self.queue_open_symbol_sync_all_timeframes(sym);
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
            "GROK" | "GROK_CLI" | "GROK-BUILD" | "GROK_BUILD" | "ASKGROK" | "ASK_GROK" => {
                match std::process::Command::new("which").arg("grok").output() {
                    Ok(out) if out.status.success() => {
                        self.show_grok_cli = true;
                        self.log
                            .push_back(LogEntry::info("Grok Build CLI detected — opening chat"));
                    }
                    _ => {
                        self.log.push_back(LogEntry::err(
                            "Grok Build CLI not found in PATH. Install/configure the grok binary first.",
                        ));
                    }
                }
            }
            // ── AI session resume + history browser ──
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
            "RESUMEGROK" | "RESUME_GROK" => {
                if let Some(ref cache) = self.cache {
                    match typhoon_engine::core::ai_sessions::latest_for_provider(cache, "grok") {
                        Ok(Some(rec)) => {
                            self.grok_cli_history = rec.turns.clone();
                            self.grok_cli_session_id = rec.session_id.clone();
                            self.show_grok_cli = true;
                            self.log.push_back(LogEntry::info(format!(
                                "Resumed Grok session {} ({} turns — transcript replayed as context)",
                                rec.session_id, rec.turns.len())));
                        }
                        Ok(None) => self
                            .log
                            .push_back(LogEntry::warn("No saved Grok session to resume")),
                        Err(e) => self
                            .log
                            .push_back(LogEntry::err(format!("RESUMEGROK: {e}"))),
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
            // ── cross-client AI response cache stats ──
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
            cmd if cmd.starts_with("ASKGROK ") || cmd.starts_with("ASK_GROK ") => {
                let args = cmd
                    .splitn(2, char::is_whitespace)
                    .nth(1)
                    .unwrap_or("")
                    .trim();
                let (syms, question) = Self::parse_ask_args(args);
                if syms.is_empty() {
                    self.show_grok_cli = true;
                    self.log.push_back(LogEntry::warn(
                        "Usage: ASKGROK SYM1[,SYM2] [optional question]",
                    ));
                    return;
                }
                match std::process::Command::new("which").arg("grok").output() {
                    Ok(out) if out.status.success() => {
                        let packet = self.investigate_symbols(&syms, &question);
                        self.grok_cli_packet = Some(packet.clone());
                        self.show_grok_cli = true;
                        let first_user_turn = if question.is_empty() {
                            format!(
                                "Give me an overall read on {} — combine the research packet above with live web search for recent news/sentiment.",
                                syms.join(", ")
                            )
                        } else {
                            question.clone()
                        };
                        self.grok_cli_history.push((
                            true,
                            format!(
                                "[Research packet loaded: {}] {}",
                                syms.join(", "),
                                first_user_turn
                            ),
                        ));
                        if self.grok_cli_rx.is_none() {
                            let model = self.grok_model.clone();
                            let effort = self.grok_effort.clone();
                            let full_prompt = Self::build_claude_prompt(
                                Some(&packet),
                                &self.grok_cli_history,
                                &first_user_turn,
                                "",
                            );
                            let (tx, rx) = std::sync::mpsc::channel();
                            self.grok_cli_rx = Some(rx);
                            Self::spawn_grok_exec(model, effort, full_prompt, tx);
                            self.log.push_back(LogEntry::info(format!(
                                "Grok Build investigation dispatched: {} ({} symbols, model {}, effort {})",
                                syms.join(", "),
                                syms.len(),
                                if self.grok_model.trim().is_empty() { "auto" } else { self.grok_model.as_str() },
                                Self::grok_effort_label(&self.grok_effort)
                            )));
                        }
                    }
                    _ => {
                        self.log
                            .push_back(LogEntry::err("Grok Build CLI not found in PATH."));
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
                                for tf in &missing_tfs {
                                    if self.queue_kraken_futures_fetch(&su, tf) {
                                        fetched_count += 1;
                                    }
                                }
                            } else {
                                skipped_count += 1;
                                continue;
                            }
                        } else if is_crypto {
                            // Crypto: use Kraken public market data.
                            // Normalize: remove slashes, uppercase (BTC/USD → BTCUSD)
                            let clean_sym = sym.replace('/', "").to_uppercase();
                            if self.kraken_spot_symbol_scrape_enabled(&clean_sym) {
                                for tf in &missing_tfs {
                                    if self.queue_kraken_fetch(&clean_sym, tf) {
                                        fetched_count += 1;
                                    }
                                }
                            }
                        } else if self.broker_connected {
                            // Stocks/Forex/CFDs: use Alpaca (AlpacaFetchBars, with MT5 priority + full-history first fetch)
                            for tf in &missing_tfs {
                                if self.queue_alpaca_fetch(&sym, tf) {
                                    fetched_count += 1;
                                }
                            }
                        }

                        // tastytrade: bars + option chain (if connected and not already cached)
                        if self.tt_connected {
                            for tf in &missing_tfs {
                                if self.queue_tastytrade_fetch(&sym, tf) {
                                    fetched_count += 1;
                                }
                            }
                            let _ = self.broker_tx.send(BrokerCmd::TastytradeOptionChain {
                                symbol: sym.clone(),
                            });
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
                if !self.kraken_enabled {
                    self.log
                        .push_back(LogEntry::warn("Kraken is disabled in Settings"));
                } else {
                    self.show_kraken_trade_history = true;
                    let _ = self.broker_tx.send(BrokerCmd::KrakenFetchTrades);
                    self.log
                        .push_back(LogEntry::info("Kraken: refreshing trade history"));
                }
            }
            "KRAKEN_ORDERS" | "KRAKENORDERS" | "KRAKEN_OPEN_ORDERS" => {
                if !self.kraken_enabled {
                    self.log
                        .push_back(LogEntry::warn("Kraken is disabled in Settings"));
                } else {
                    self.show_kraken_open_orders = true;
                    let _ = self.broker_tx.send(BrokerCmd::KrakenFetchOpenOrders);
                    self.log
                        .push_back(LogEntry::info("Kraken: refreshing open orders"));
                }
            }
            "KRAKEN_FUTURES" | "KRAKENFUTURES" => {
                if !self.kraken_enabled {
                    self.log
                        .push_back(LogEntry::warn("Kraken is disabled in Settings"));
                } else {
                    let _ = self.broker_tx.send(BrokerCmd::KrakenFuturesGetInstruments);
                    self.kraken_futures_requested = true;
                    self.log.push_back(LogEntry::info(
                        "Kraken Futures: loading public instrument universe",
                    ));
                }
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
