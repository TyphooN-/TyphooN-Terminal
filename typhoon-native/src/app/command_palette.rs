use super::*;

mod ai_commands;
mod fundamental_event_commands;
mod market_data_commands;
mod outlier_scan_commands;
mod template_workspace_commands;
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
        if self.handle_ai_command(&cmd_upper) {
            return;
        }
        if self.handle_market_data_command(&cmd_upper) {
            return;
        }
        if self.handle_trade_order_command(&cmd_upper) {
            return;
        }
        if self.handle_outlier_scan_command(&cmd_upper) {
            return;
        }
        if self.handle_fundamental_event_command(&cmd_upper) {
            return;
        }
        match cmd_upper.as_str() {
            "QUIT" => {
                self.save_session();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            "TRADECOPY" | "TRADE_COPY" | "COPYTRADE" => {
                self.show_tradecopy = true;
            }
            "MARKET_MAP" | "MARKETMAP" | "MAP" | "TREEMAP" => {
                self.show_market_map = true;
            }
            "REG_SHO" | "REGSHO" => {
                self.show_reg_sho_window = true;
            }
            "HALTS" | "HALT" | "TRADE_HALTS" | "LULD" => {
                self.show_halts_window = true;
            }
            "SETTINGS" => self.show_settings = true,
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
            "SMA_INTELLIGENCE" => self.show_sma_intelligence = true,
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
            // Aliases
            "CALC" => self.show_risk_calc = true,
            "DASHBOARD" => self.show_cache_stats = true,
            "STATUS" => self.show_cache_stats = true,
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
                    self.show_kraken_trade_history = true;
                    let _ = self.broker_tx.send(BrokerCmd::KrakenFetchTrades);
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
            // but removed from the palette in favour of the ASKAI / ASKCLAUDE / ASKANTIGRAVITY variants
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
            other => self.handle_template_workspace_command(other),
        }
    }

    // ── Chart template helpers ────────────────────────────────────────────────
}
