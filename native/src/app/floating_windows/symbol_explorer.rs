use super::*;

impl TyphooNApp {
    pub(super) fn render_symbol_explorer_window(&mut self, ctx: &egui::Context) {
        // Symbols Explorer — all-encompassing symbol browser with broker hierarchy
        if self.show_symbols {
            let mut show_symbols = self.show_symbols;
            // Fetch broker symbol universes on first open. Alpaca exposes full
            // asset metadata; Kraken exposes tradable symbol catalogs.
            if !self.all_broker_assets_fetched && self.broker_connected && self.alpaca_enabled {
                let _ = self.broker_tx.send(BrokerCmd::GetAllAssets);
                self.all_broker_assets_fetched = true;
            }
            if self.kraken_enabled && self.kraken_pairs.is_empty() && !self.kraken_pairs_requested {
                let _ = self.broker_tx.send(BrokerCmd::KrakenGetPairs);
                self.kraken_pairs_requested = true;
            }
            if self.kraken_enabled
                && self.kraken_scrape_xstocks
                && self.kraken_equity_universe_symbols.is_empty()
                && (!self.kraken_equity_universe_requested
                    || chrono::Utc::now().timestamp() >= self.kraken_equity_universe_retry_after_ts)
            {
                let _ = self.broker_tx.send(BrokerCmd::KrakenFetchEquityUniverse);
                self.kraken_equity_universe_requested = true;
                self.kraken_equity_universe_retry_after_ts = chrono::Utc::now().timestamp() + 120;
            }
            if self.kraken_enabled
                && self.kraken_scrape_futures
                && self.kraken_futures_symbols.is_empty()
                && !self.kraken_futures_requested
            {
                let _ = self.broker_tx.send(BrokerCmd::KrakenFuturesGetInstruments);
                self.kraken_futures_requested = true;
            }
            egui::Window::new("Symbol Explorer")
                .open(&mut show_symbols)
                .resizable(true)
                .default_size([680.0, 650.0])
                .max_size([680.0, 640.0])
                .show(ctx, |ui| {
                    let sym_green = egui::Color32::from_rgb(46, 204, 113);
                    let sym_blue = egui::Color32::from_rgb(52, 152, 219);
                    let sym_orange = egui::Color32::from_rgb(255, 130, 60);
                    let sym_white = egui::Color32::from_rgb(220, 220, 220);
                    let sym_dim = egui::Color32::from_rgb(120, 120, 120);
                    let sym_cached = egui::Color32::from_rgb(80, 200, 120);

                    ui.horizontal(|ui| {
                        ui.label("Filter:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.symbols_filter)
                                .desired_width(250.0)
                                .font(egui::TextStyle::Monospace),
                        );
                        if ui.small_button("Clear").clicked() {
                            self.symbols_filter.clear();
                        }
                    });
                    ui.separator();

                    // Build set of cached symbols (normalized, no slash, uppercase)
                    let details = &self.bg.detailed_stats;
                    let filter_upper = self.symbols_filter.to_uppercase();
                    let mut broker_universe_assets: Vec<(String, String, String)> =
                        Vec::with_capacity(
                            self.all_broker_assets.len()
                                + self.kraken_pairs.len()
                                + self.kraken_equity_universe_symbols.len()
                                + self.kraken_futures_symbols.len(),
                        );
                    broker_universe_assets.extend(self.all_broker_assets.iter().cloned());
                    broker_universe_assets.extend(self.kraken_pairs.iter().map(
                        |(pair, display)| (display.clone(), pair.clone(), "crypto".to_string()),
                    ));
                    broker_universe_assets.extend(self.kraken_equity_universe_symbols.iter().map(
                        |sym| {
                            (
                                sym.clone(),
                                "Kraken Securities/xStock".to_string(),
                                "us_equity".to_string(),
                            )
                        },
                    ));
                    broker_universe_assets.extend(self.kraken_futures_symbols.iter().map(|sym| {
                        (
                            sym.clone(),
                            "Kraken Futures instrument".to_string(),
                            "future".to_string(),
                        )
                    }));

                    // PERF: return &str slices instead of a heap-allocated Vec<&str>.
                    // Called once per detailed_stats entry (~500/frame when explorer open).
                    fn parse_cache_key(key: &str) -> Option<(&str, &str, &str)> {
                        let count = key.bytes().filter(|&b| b == b':').count();
                        let mut parts = key.splitn(4, ':');
                        match count {
                            3 => {
                                let a = parts.next()?;
                                let _b = parts.next()?;
                                let c = parts.next()?;
                                let d = parts.next()?;
                                Some((a, c, d))
                            }
                            2 => {
                                let a = parts.next()?;
                                let b = parts.next()?;
                                let c = parts.next()?;
                                Some((a, b, c))
                            }
                            1 => {
                                let a = parts.next()?;
                                let b = parts.next()?;
                                Some(("local", a, b))
                            }
                            _ => None,
                        }
                    }

                    // Cached symbols: source → symbol → Vec<(tf, bars)>
                    let mut cached: std::collections::BTreeMap<
                        String,
                        std::collections::BTreeMap<String, Vec<(String, i64)>>,
                    > = std::collections::BTreeMap::new();
                    let mut cached_syms_set: std::collections::HashSet<String> =
                        std::collections::HashSet::new();
                    for (key, bars, _ts) in details {
                        // Skip cache metadata rows (`<prefix>:__NAME__:…`).
                        // parse_cache_key would otherwise surface "__HEARTBEAT__"
                        // et al as bogus symbols in the Symbol Explorer tree.
                        if key.contains(":__") {
                            continue;
                        }
                        if let Some((source, symbol, tf)) = parse_cache_key(key) {
                            // In-place uppercase instead of replace().to_uppercase() (two allocs → one).
                            let mut norm = symbol.replace('/', "");
                            norm.make_ascii_uppercase();
                            if !cached_syms_set.contains(&norm) {
                                cached_syms_set.insert(norm.clone());
                            }
                            if !filter_upper.is_empty() && !norm.contains(&filter_upper) {
                                continue;
                            }
                            cached
                                .entry(source.to_string())
                                .or_default()
                                .entry(symbol.to_string())
                                .or_default()
                                .push((tf.to_string(), *bars));
                        }
                    }

                    // Build fundamentals lookup (symbol → (sector, industry, name)).
                    // f.symbol is already uppercase (parse_yahoo_data), so we key on &str and
                    // borrow — zero String allocation per record per frame.
                    let fund_map: std::collections::HashMap<&str, (&str, &str, &str)> = self
                        .bg
                        .all_fundamentals
                        .iter()
                        .map(|f| {
                            (
                                f.symbol.as_str(),
                                (
                                    f.sector.as_str(),
                                    f.industry.as_str(),
                                    f.company_name.as_str(),
                                ),
                            )
                        })
                        .collect();

                    // Categorize a symbol
                    let categorize = |sym: &str, asset_class: &str| -> &'static str {
                        let s = sym.to_uppercase();
                        if asset_class == "crypto"
                            || s.contains('/')
                            || (s.ends_with("USD") && s.len() <= 10 && !s.contains('.'))
                            || s.ends_with("BTC")
                            || s.ends_with("ETH")
                        {
                            return "Crypto";
                        }
                        if s.len() == 6
                            && s.chars().all(|c| c.is_ascii_alphabetic())
                            && !s.ends_with("USD")
                        {
                            return "Forex";
                        }
                        if s.starts_with('.')
                            || s.contains("500")
                            || s.contains("DAX")
                            || s.contains("NAS")
                            || s.contains("JPN")
                            || s.contains("US30")
                            || s.contains("US100")
                            || s.contains("STOXX")
                        {
                            return "Indices";
                        }
                        if s.contains("XAU")
                            || s.contains("XAG")
                            || s.contains("XNG")
                            || s.contains("OIL")
                            || s.contains("BRENT")
                            || s.contains("NATGAS")
                            || s.contains("COCOA")
                            || s.contains("WHEAT")
                            || s.contains("CORN")
                            || s.contains("SUGAR")
                            || s.contains("COFFEE")
                        {
                            return "Commodities";
                        }
                        // Use fundamentals sector if available
                        if let Some((sector, _, _)) = fund_map.get(s.as_str()) {
                            if !sector.is_empty() {
                                return match *sector {
                                    s if s.contains("ETF") || s.contains("Fund") => "ETFs",
                                    _ => "Stocks",
                                };
                            }
                        }
                        if asset_class == "us_equity" {
                            return "Stocks";
                        }
                        "Other"
                    };

                    // Count cached + broker universe
                    let cached_count: usize = cached.values().map(|s| s.len()).sum();
                    let broker_count = broker_universe_assets.len();
                    let uncached_count = broker_universe_assets
                        .iter()
                        .filter(|(s, _, _)| {
                            !cached_syms_set.contains(&s.replace('/', "").to_uppercase())
                        })
                        .count();
                    let cache_label = "cached";
                    ui.label(
                        egui::RichText::new(format!(
                            "{} {} symbols | {} broker universe ({} not cached)",
                            cached_count, cache_label, broker_count, uncached_count
                        ))
                        .color(sym_dim),
                    );
                    ui.add_space(4.0);

                    let mut load_sym: Option<String> = None;
                    let mut add_wl: Option<String> = None;
                    // Macro for symbol row rendering
                    macro_rules! sym_row {
                        ($ui:expr, $sym:expr, $info:expr, $indent:expr, $load:expr, $wl:expr) => {
                            $ui.horizontal(|ui| {
                                ui.add_space($indent);
                                if ui
                                    .add(
                                        egui::Button::new(egui::RichText::new("\u{1F4C8}").small())
                                            .min_size(egui::vec2(22.0, 18.0)),
                                    )
                                    .on_hover_text("Load chart")
                                    .clicked()
                                {
                                    $load = Some($sym.to_string());
                                }
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("+WL").color(sym_blue).small(),
                                        )
                                        .min_size(egui::vec2(30.0, 18.0)),
                                    )
                                    .on_hover_text("Add to watchlist")
                                    .clicked()
                                {
                                    $wl = Some($sym.to_string());
                                }
                                if ui
                                    .add(
                                        egui::Label::new(
                                            egui::RichText::new($sym).monospace().color(sym_white),
                                        )
                                        .sense(egui::Sense::click()),
                                    )
                                    .clicked()
                                {
                                    $load = Some($sym.to_string());
                                }
                                ui.label(egui::RichText::new($info).color(sym_dim).small());
                            });
                        };
                    }

                    let avail = ui.available_height().max(300.0);
                    egui::ScrollArea::vertical()
                        .id_salt("symbols_scroll")
                        .min_scrolled_height(avail)
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            // ── Section 1: Cached Data (by source) ──
                            let source_labels: &[(&str, &str, egui::Color32)] = &[
                                ("alpaca", "Alpaca (cached)", sym_green),
                                ("kraken", "Kraken", sym_orange),
                                ("kraken-equities", "Kraken Equities (cached)", sym_orange),
                            ];

                            for (source_key, label, color) in source_labels {
                                let Some(syms) = cached.get(*source_key) else {
                                    continue;
                                };

                                let mut categories: std::collections::BTreeMap<
                                    &str,
                                    Vec<(&String, &Vec<(String, i64)>)>,
                                > = std::collections::BTreeMap::new();
                                for (sym, tfs) in syms {
                                    categories
                                        .entry(categorize(sym, ""))
                                        .or_default()
                                        .push((sym, tfs));
                                }

                                let section_id = source_key.to_string();
                                let expanded = self.symbols_expanded.contains(&section_id);
                                let arrow = if expanded { "\u{25BC}" } else { "\u{25B6}" };
                                if ui
                                    .add(
                                        egui::Label::new(
                                            egui::RichText::new(format!(
                                                "{} {} ({})",
                                                arrow,
                                                label,
                                                syms.len()
                                            ))
                                            .color(*color)
                                            .strong(),
                                        )
                                        .sense(egui::Sense::click()),
                                    )
                                    .clicked()
                                {
                                    if expanded {
                                        self.symbols_expanded.remove(&section_id);
                                    } else {
                                        self.symbols_expanded.insert(section_id.clone());
                                    }
                                }
                                if !expanded {
                                    continue;
                                }

                                for (cat, entries) in &categories {
                                    if categories.len() > 1 {
                                        let cat_id = format!("{}:{}", source_key, cat);
                                        let cat_exp = self.symbols_expanded.contains(&cat_id);
                                        let ca = if cat_exp { "\u{25BC}" } else { "\u{25B6}" };
                                        ui.horizontal(|ui| {
                                            ui.add_space(12.0);
                                            if ui
                                                .add(
                                                    egui::Label::new(
                                                        egui::RichText::new(format!(
                                                            "{} {} ({})",
                                                            ca,
                                                            cat,
                                                            entries.len()
                                                        ))
                                                        .color(sym_dim),
                                                    )
                                                    .sense(egui::Sense::click()),
                                                )
                                                .clicked()
                                            {
                                                if cat_exp {
                                                    self.symbols_expanded.remove(&cat_id);
                                                } else {
                                                    self.symbols_expanded.insert(cat_id.clone());
                                                }
                                            }
                                        });
                                        if !cat_exp {
                                            continue;
                                        }
                                    }
                                    for (sym, tfs) in entries {
                                        let total_bars: i64 = tfs.iter().map(|(_, b)| *b).sum();
                                        // sym is a cache-key fragment — upper-case in place so the &str key lookup works.
                                        let sym_upper = sym.to_uppercase();
                                        let name = fund_map
                                            .get(sym_upper.as_str())
                                            .map(|(_, _, n)| *n)
                                            .unwrap_or("");
                                        let info = if name.is_empty() {
                                            format!("{} TFs  {} bars", tfs.len(), total_bars)
                                        } else {
                                            format!(
                                                "{} TFs  {} bars  {}",
                                                tfs.len(),
                                                total_bars,
                                                name
                                            )
                                        };
                                        sym_row!(
                                            ui,
                                            sym.as_str(),
                                            info,
                                            24.0_f32,
                                            load_sym,
                                            add_wl
                                        );
                                    }
                                }
                            }

                            // ── Section 2: Broker Universe (uncached symbols) ──
                            if !broker_universe_assets.is_empty() {
                                // Group by category using fundamentals
                                let mut universe: std::collections::BTreeMap<
                                    &str,
                                    Vec<&(String, String, String)>,
                                > = std::collections::BTreeMap::new();
                                for asset in &broker_universe_assets {
                                    let sym_norm = asset.0.replace('/', "").to_uppercase();
                                    if !filter_upper.is_empty()
                                        && !sym_norm.contains(&filter_upper)
                                        && !asset.1.to_uppercase().contains(&filter_upper)
                                    {
                                        continue;
                                    }
                                    let cat = categorize(&asset.0, &asset.2);
                                    universe.entry(cat).or_default().push(asset);
                                }

                                let universe_total: usize =
                                    universe.values().map(|v| v.len()).sum();
                                let section_id = "broker_universe".to_string();
                                let expanded = self.symbols_expanded.contains(&section_id);
                                let arrow = if expanded { "\u{25BC}" } else { "\u{25B6}" };
                                ui.add_space(8.0);
                                if ui
                                    .add(
                                        egui::Label::new(
                                            egui::RichText::new(format!(
                                                "{} Broker Universe ({})",
                                                arrow, universe_total
                                            ))
                                            .color(sym_green)
                                            .strong(),
                                        )
                                        .sense(egui::Sense::click()),
                                    )
                                    .clicked()
                                {
                                    if expanded {
                                        self.symbols_expanded.remove(&section_id);
                                    } else {
                                        self.symbols_expanded.insert(section_id.clone());
                                    }
                                }

                                if expanded {
                                    for (cat, assets) in &universe {
                                        let cat_id = format!("universe:{}", cat);
                                        let cat_exp = self.symbols_expanded.contains(&cat_id);
                                        let ca = if cat_exp { "\u{25BC}" } else { "\u{25B6}" };
                                        ui.horizontal(|ui| {
                                            ui.add_space(12.0);
                                            if ui
                                                .add(
                                                    egui::Label::new(
                                                        egui::RichText::new(format!(
                                                            "{} {} ({})",
                                                            ca,
                                                            cat,
                                                            assets.len()
                                                        ))
                                                        .color(sym_dim),
                                                    )
                                                    .sense(egui::Sense::click()),
                                                )
                                                .clicked()
                                            {
                                                if cat_exp {
                                                    self.symbols_expanded.remove(&cat_id);
                                                } else {
                                                    self.symbols_expanded.insert(cat_id.clone());
                                                }
                                            }
                                        });
                                        if !cat_exp {
                                            continue;
                                        }

                                        for (sym, name, _class) in assets.iter().take(500) {
                                            let is_cached = cached_syms_set
                                                .contains(&sym.replace('/', "").to_uppercase());
                                            let info = if is_cached {
                                                format!("{}  [cached]", name)
                                            } else {
                                                name.to_string()
                                            };
                                            ui.horizontal(|ui| {
                                                ui.add_space(24.0);
                                                if ui
                                                    .add(
                                                        egui::Button::new(
                                                            egui::RichText::new("\u{1F4C8}")
                                                                .small(),
                                                        )
                                                        .min_size(egui::vec2(22.0, 18.0)),
                                                    )
                                                    .clicked()
                                                {
                                                    load_sym = Some(sym.clone());
                                                }
                                                if ui
                                                    .add(
                                                        egui::Button::new(
                                                            egui::RichText::new("+WL")
                                                                .color(sym_blue)
                                                                .small(),
                                                        )
                                                        .min_size(egui::vec2(30.0, 18.0)),
                                                    )
                                                    .clicked()
                                                {
                                                    add_wl = Some(sym.clone());
                                                }
                                                let sym_color =
                                                    if is_cached { sym_cached } else { sym_white };
                                                if ui
                                                    .add(
                                                        egui::Label::new(
                                                            egui::RichText::new(sym.as_str())
                                                                .monospace()
                                                                .color(sym_color),
                                                        )
                                                        .sense(egui::Sense::click()),
                                                    )
                                                    .clicked()
                                                {
                                                    load_sym = Some(sym.clone());
                                                }
                                                ui.label(
                                                    egui::RichText::new(&info)
                                                        .color(sym_dim)
                                                        .small(),
                                                );
                                            });
                                        }
                                        if assets.len() > 500 {
                                            ui.horizontal(|ui| {
                                                ui.add_space(24.0);
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "... {} more (use filter)",
                                                        assets.len() - 500
                                                    ))
                                                    .color(sym_dim)
                                                    .small(),
                                                );
                                            });
                                        }
                                    }
                                }
                            }
                        });

                    // Handle chart load
                    if let Some(symbol) = load_sym {
                        self.symbol_input = symbol.clone();
                        if let Some(chart) = self.charts.get_mut(self.active_tab) {
                            chart.switch_symbol(symbol.clone());
                            if let Some(ref cache_arc) = self.cache {
                                let mut gpu = self.gpu_indicators.take();
                                if !chart.try_load(
                                    Arc::as_ref(cache_arc),
                                    &mut self.log,
                                    gpu.as_mut(),
                                ) {
                                    self.queue_chart_reload(self.active_tab);
                                }
                                self.gpu_indicators = gpu;
                            }
                        }
                        self.log
                            .push_back(LogEntry::info(format!("Chart: {}", symbol)));
                    }

                    // Handle watchlist add
                    if let Some(sym) = add_wl {
                        let sym_upper = sym.to_uppercase();
                        if !self.user_watchlist_set.contains(&sym_upper) {
                            self.user_watchlist.push(sym_upper.clone());
                            self.user_watchlist_set.insert(sym_upper.clone());
                            self.watchlist_cache_tried = false;
                            if self.broker_connected {
                                let _ = self.broker_tx.send(BrokerCmd::GetWatchlistQuotes {
                                    symbols: self.user_watchlist.clone(),
                                });
                            }
                            self.log.push_back(LogEntry::info(format!(
                                "Added {} to watchlist",
                                sym_upper
                            )));
                        }
                    }
                });
            self.show_symbols = show_symbols;
        }
    }
}
