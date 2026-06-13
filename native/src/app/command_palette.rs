use super::*;

mod ai_commands;
mod chart_drawing;
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
            "SETTINGS" => self.show_settings = true,
            "INDICATORS" => self.show_indicators_panel = !self.show_indicators_panel,
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
            "CALC" => self.show_risk_calc = true,
            "COMPARE" => {
                let sym = self.symbol_input.clone();
                if !sym.is_empty() {
                    if let Some(chart) = self.charts.get_mut(self.active_tab) {
                        chart.compare_symbol = Some(sym.clone());
                        chart.compare_bars.clear();
                        if let Some(ref cache) = self.cache {
                            let tf_label = chart.timeframe.cache_suffix();
                            let keys = [
                                format!("kraken:{}:{}", sym, tf_label),
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
            "SIGNAL" => self.show_indicators_panel = true,
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
                // TyphooN NNFX preset (clean reset + enable)
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
            other => self.handle_template_workspace_command(other),
        }
    }

    // ── Chart template helpers ────────────────────────────────────────────────
}
