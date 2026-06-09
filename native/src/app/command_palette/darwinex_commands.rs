use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_darwinex_command(&mut self, cmd_upper: &str) -> bool {
        match cmd_upper {
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
            _ => return false,
        }
        true
    }
}
