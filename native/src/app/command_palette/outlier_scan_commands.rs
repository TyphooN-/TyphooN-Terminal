use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_outlier_scan_command(&mut self, cmd_upper: &str) -> bool {
        match cmd_upper {
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
                            self.outliers = outliers;
                            self.sector_stats = stats;
                            self.multi_outliers = multi.clone();
                            self.show_outliers = true;
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
                    self.outliers = outliers;
                    self.sector_stats = stats;
                    self.multi_outliers = Vec::new();
                    self.show_outliers = true;
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
                    // Compute VaR/Ask ratio from bar cache. Tick-scale specs are no longer
                    // available (Darwinex removed) — default tick_scale to 1.0 per symbol.
                    let tick_specs: std::collections::HashMap<String, f64> =
                        std::collections::HashMap::new();
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
                            "Need 5+ symbols with D1 bar data for VaR (have {}, {} missing bars). Run BARDATA first.",
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
                        self.outliers = sector_outliers;
                        self.sector_stats = sector_stats;
                        self.multi_outliers = Vec::new();
                        self.show_outliers = true;
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
                            "Need 5+ symbols with D1 bar data (have {}, {} missing). Run BARDATA first.",
                            atr_data.len(), no_bars
                        )));
                    } else {
                        let (outliers, stats) = var::detect_outliers(&atr_data, 1.5);
                        self.log.push_back(LogEntry::info(format!(
                            "ATR/Price outlier scan [{}]: {} outliers from {} symbols across {} sectors",
                            scope_label, outliers.len(), atr_data.len(), stats.len()
                        )));
                        self.outliers = outliers;
                        self.sector_stats = stats;
                        self.multi_outliers = Vec::new();
                        self.show_outliers = true;
                        self.outlier_scroll_pending = true;
                    }
                }
            }
            _ => return false,
        }
        true
    }
}
