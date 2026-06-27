use super::app_runtime_support::kraken_xstocks_session_status_now;
use super::*;

#[allow(deprecated)]
impl TyphooNApp {
    pub(super) fn render_symbol_timeframe_toolbar(&mut self, ctx: &egui::Context) {
        // ── symbol + timeframe toolbar ───────────────────────────────────────
        egui::Panel::top("toolbar").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT).small());
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.symbol_input)
                                .desired_width(180.0)
                                .font(egui::FontId::monospace(13.0)),
                        );

                        // Update autocomplete suggestions when text changes
                        if resp.changed() {
                            let query = self.symbol_input.trim().to_uppercase();
                            if query.len() >= 1 {
                                self.symbol_ac_visible = true;
                                self.symbol_ac_selected = 0;
                                // Build suggestions from cache keys + fundamentals
                                let mut suggestions = Vec::new();
                                // From fundamentals (has company name + sector).
                                // f.symbol is guaranteed uppercase (parse_yahoo_data), so skip the alloc.
                                for f in &self.bg.all_fundamentals {
                                    if f.symbol.contains(&query)
                                        || f.company_name.to_uppercase().contains(&query)
                                    {
                                        suggestions.push((
                                            f.symbol.clone(),
                                            f.company_name.clone(),
                                            f.sector.clone(),
                                        ));
                                    }
                                }
                                // From already-known active/cached symbols. This avoids a synchronous
                                // SQLite all_keys() scan on every keystroke.
                                let query_norm = query.replace('/', "");
                                for sym in &self.cached_active_symbols {
                                    let sym_norm = sym.replace('/', "").to_uppercase();
                                    if sym_norm.contains(&query_norm)
                                        && !suggestions
                                            .iter()
                                            .any(|(s, _, _)| s.replace('/', "").to_uppercase() == sym_norm)
                                    {
                                        let class = if sym_norm.ends_with("USD")
                                            && !sym_norm.contains('.')
                                            && sym_norm.len() <= 10
                                        {
                                            "crypto".to_string()
                                        } else {
                                            String::new()
                                        };
                                        suggestions.push((sym_norm, String::new(), class));
                                    }
                                }

                                // From enabled broker universes, even before bars are cached.
                                // This is the light-mode path: symbol metadata resolves immediately,
                                // and opening the chart queues on-demand bar sync.
                                if self.kraken_enabled && self.kraken_scrape_xstocks {
                                    for sym in &self.kraken_equity_universe_symbols {
                                        if sym.contains(&query)
                                            && !suggestions
                                                .iter()
                                                .any(|(s, _, _)| s.eq_ignore_ascii_case(sym))
                                        {
                                            suggestions.push((
                                                sym.clone(),
                                                "Kraken Securities".to_string(),
                                                "Kraken xStock".to_string(),
                                            ));
                                        }
                                    }
                                }
                                // From Kraken tradeable pairs (if loaded)
                                for (pair_name, display_name) in &self.kraken_pairs {
                                    let pn = pair_name.to_uppercase();
                                    let dn = display_name.to_uppercase();
                                    if pn.contains(&query) || dn.contains(&query) {
                                        if !suggestions.iter().any(|(s, _, _)| s.to_uppercase() == pn) {
                                            suggestions.push((
                                                display_name.clone(),
                                                pair_name.clone(),
                                                kraken_pair_asset_class(pair_name, display_name)
                                                    .to_string(),
                                            ));
                                        }
                                    }
                                }
                                suggestions.sort_by(|a, b| {
                                    // Exact prefix match first, then alphabetical
                                    let a_starts = a.0.to_uppercase().starts_with(&query);
                                    let b_starts = b.0.to_uppercase().starts_with(&query);
                                    b_starts.cmp(&a_starts).then(a.0.cmp(&b.0))
                                });
                                suggestions.truncate(12);
                                self.symbol_suggestions = suggestions;
                                // If few local results and query >= 2 chars, also search Alpaca
                                if self.symbol_suggestions.len() < 5 && query.len() >= 2 {
                                    let _ = self.broker_tx.send(BrokerCmd::SearchSymbols {
                                        query: query.clone(),
                                    });
                                }
                            } else {
                                self.symbol_ac_visible = false;
                            }
                        }

                        // Handle Enter: load symbol or select from autocomplete
                        if resp.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                            if self.symbol_ac_visible
                                && self.symbol_ac_selected < self.symbol_suggestions.len()
                            {
                                self.symbol_input =
                                    self.symbol_suggestions[self.symbol_ac_selected].0.clone();
                            }
                            let sym = self.symbol_input.trim().to_string();
                            let tf = self
                                .charts
                                .get(self.active_tab)
                                .map(|c| c.timeframe)
                                .unwrap_or(Timeframe::H4);
                            self.reload_symbol(&sym, tf);
                            self.symbol_ac_visible = false;
                        }

                        // Arrow keys to navigate suggestions
                        if self.symbol_ac_visible && resp.has_focus() {
                            if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                                self.symbol_ac_selected = (self.symbol_ac_selected + 1)
                                    .min(self.symbol_suggestions.len().saturating_sub(1));
                            }
                            if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                                self.symbol_ac_selected = self.symbol_ac_selected.saturating_sub(1);
                            }
                            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                                self.symbol_ac_visible = false;
                            }
                        }

                        // Hide autocomplete when input loses focus
                        if !resp.has_focus() && !ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                            // Delay hide slightly so clicks on suggestions register
                            // (egui processes click after focus loss)
                        }

                        ui.separator();

                        // Timeframe dropdown (ComboBox — type to search, e.g. "H4")
                        let cur_tf = self
                            .charts
                            .get(self.active_tab)
                            .map(|c| c.timeframe)
                            .unwrap_or(Timeframe::H4);
                        let mut new_tf = cur_tf;
                        egui::ComboBox::from_id_salt("tf_combo")
                            .selected_text(
                                egui::RichText::new(cur_tf.label())
                                    .color(ACCENT)
                                    .strong()
                                    .small(),
                            )
                            .width(55.0)
                            .show_ui(ui, |ui| {
                                for &tf in Timeframe::all() {
                                    ui.selectable_value(&mut new_tf, tf, tf.label());
                                }
                            });
                        if new_tf != cur_tf {
                            let sym = self
                                .charts
                                .get(self.active_tab)
                                .map(|c| c.symbol.clone())
                                .unwrap_or_else(|| self.symbol_input.trim().to_string());
                            self.reload_symbol(&sym, new_tf);
                        }

                        let source_state = self
                            .charts
                            .get(self.active_tab)
                            .map(|c| (c.symbol.clone(), c.primary_source, c.source_override))
                            .unwrap_or_else(|| (self.symbol_input.trim().to_string(), "", ""));
                        let mut selected_source = source_state.2;
                        let source_label = if selected_source.is_empty() {
                            if source_state.1.is_empty() {
                                "Auto".to_string()
                            } else {
                                format!("Auto → {}", cache_source_label(source_state.1))
                            }
                        } else if selected_source == "merged" {
                            "Merged".to_string()
                        } else {
                            cache_source_label(selected_source).to_string()
                        };
                        egui::ComboBox::from_id_salt("chart_source_combo")
                            .selected_text(
                                egui::RichText::new(source_label)
                                    .color(AXIS_TEXT)
                                    .monospace()
                                    .small(),
                            )
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut selected_source, "", "Auto");
                                ui.separator();
                                ui.selectable_value(&mut selected_source, "merged", "Merged");
                                for (source, label) in CHART_SOURCE_ORDER {
                                    ui.selectable_value(&mut selected_source, source, label);
                                }
                            });
                        if selected_source != source_state.2 {
                            if let Some(chart) = self.charts.get_mut(self.active_tab) {
                                chart.source_override = selected_source;
                            }
                            self.reload_symbol_auto(&source_state.0, cur_tf);
                            self.log.push_back(LogEntry::info(format!(
                                "Chart source {} for {} {}",
                                if selected_source.is_empty() {
                                    "set to Auto".to_string()
                                } else {
                                    format!(
                                        "forced to {}",
                                        if selected_source == "merged" {
                                            "Merged"
                                        } else {
                                            cache_source_label(selected_source)
                                        }
                                    )
                                },
                                source_state.0,
                                cur_tf.label()
                            )));
                        }

                        let orderbook_symbol = bare_symbol_from_key(&source_state.0)
                            .trim_end_matches(".EQ")
                            .to_ascii_uppercase();
                        let orderbook_symbol_is_public_kraken =
                            typhoon_engine::core::kraken::to_kraken_pair_lossy(&orderbook_symbol).is_some();
                        let show_kraken_l2 = self.kraken_enabled
                            && !orderbook_symbol.is_empty()
                            && orderbook_symbol_is_public_kraken;
                        if show_kraken_l2 {
                            let is_streaming = self
                                .kraken_orderbook_ws_symbol
                                .eq_ignore_ascii_case(&orderbook_symbol);
                            let label = if is_streaming { "L2 LIVE" } else { "L2" };
                            let color = if is_streaming { ACCENT } else { AXIS_TEXT };
                            if ui
                                .small_button(egui::RichText::new(label).color(color).monospace())
                                .on_hover_text("Open Kraken WebSocket v2 Level 2 DOM for the active symbol; validates CRC32 book checksums and stops on drift")
                                .clicked()
                            {
                                self.show_orderbook_window = true;
                                if !is_streaming {
                                    self.orderbook_result.clear();
                                    self.kraken_orderbook_ws_symbol = orderbook_symbol.clone();
                                    let _ = self.broker_tx.send(BrokerCmd::KrakenStartOrderbookWs {
                                        symbol: orderbook_symbol.clone(),
                                        depth: 100,
                                        publish_dom: true,
                                    });
                                }
                            }
                        }

                        ui.separator();

                        // MTF toggle
                        let mtf_txt = if self.mtf_enabled {
                            egui::RichText::new("MTF ON").color(ACCENT).small().strong()
                        } else {
                            egui::RichText::new("MTF").color(AXIS_TEXT).small()
                        };
                        if ui.small_button(mtf_txt).clicked() {
                            self.mtf_enabled = !self.mtf_enabled;
                        }
                        // The per-tab grid-visibility toggles moved onto the tab strip
                        // itself (click a tab in MTF mode to include/exclude it), so the
                        // inline checkboxes that used to crowd the toolbar are gone.

                        ui.separator();

                        // Bar count + active-position entry price
                        if let Some(c) = self.charts.get(self.active_tab) {
                            ui.label(
                                egui::RichText::new(format!("{} bars", c.bars.len()))
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                            let active_symbol = bare_symbol_from_key(&c.symbol).to_ascii_uppercase();
                            let active_entry = self
                                .live_positions
                                .iter()
                                .find_map(|pos| {
                                    let pos_symbol = bare_symbol_from_key(&pos.symbol).to_ascii_uppercase();
                                    (pos_symbol == active_symbol
                                        && pos.avg_entry_price.is_finite()
                                        && pos.avg_entry_price > 0.0)
                                        .then_some(pos.avg_entry_price)
                                })
                                .or_else(|| {
                                    self.kr_positions.iter().find_map(|pos| {
                                        let pos_symbol =
                                            bare_symbol_from_key(&pos.symbol).to_ascii_uppercase();
                                        if pos_symbol != active_symbol {
                                            return None;
                                        }
                                        if pos.avg_entry_price.is_finite() && pos.avg_entry_price > 0.0 {
                                            Some(pos.avg_entry_price)
                                        } else {
                                            self.kraken_position_avg_price(&pos.symbol)
                                        }
                                    })
                                });
                            if let Some(entry) = active_entry {
                                ui.label(
                                    egui::RichText::new(format!("Entry {}", format_price(entry)))
                                        .color(AXIS_TEXT)
                                        .small(),
                                );
                            }
                        }

                        if self.cache.is_none() {
                            ui.label(
                                egui::RichText::new("NO CACHE")
                                    .color(egui::Color32::from_rgb(255, 80, 80))
                                    .small()
                                    .strong(),
                            );
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("~").color(AXIS_TEXT).small());
                            ui.separator();
                            // Determine if we have any data source (broker, API keys, cache)
                            let has_broker = self.broker_connected || self.kraken_connected;
                            let has_api = !self.finnhub_key.is_empty()
                                || !self.fred_key.is_empty()
                                || !self.fmp_key.is_empty()
                                || !self.marketaux_key.is_empty()
                                || !self.alpha_vantage_key.is_empty()
                                || !self.cryptopanic_key.is_empty();
                            let has_cache = self.cache.is_some()
                                && self
                                    .bg
                                    .cache_stats
                                    .map(|(bars, _, _)| bars > 0)
                                    .unwrap_or(false);
                            if has_broker || has_api || has_cache {
                                // Show every active data connection with its protocol(s). Alpaca's
                                // mode (Paper/Live) is on the account chip further left; here we name
                                // the data protocol instead so the two aren't redundant. Keyless
                                // sources (Yahoo quotes/charts, GDELT/CoinDesk news) are listed when
                                // online; keyed providers when their API key is configured.
                                let online = self.broker_connected || self.kraken_connected;
                                let mut sources: Vec<String> = Vec::new();
                                if self.broker_connected {
                                    sources.push("Alpaca (REST)".into());
                                }
                                if self.kraken_connected {
                                    sources.push("Kraken (REST + WS)".into());
                                }
                                if online {
                                    sources.push("Yahoo".into());
                                }
                                if !self.finnhub_key.is_empty() {
                                    sources.push("Finnhub".into());
                                }
                                if !self.fmp_key.is_empty() {
                                    sources.push("FMP".into());
                                }
                                if !self.marketaux_key.is_empty() {
                                    sources.push("Marketaux".into());
                                }
                                if !self.alpha_vantage_key.is_empty() {
                                    sources.push("Alpha Vantage".into());
                                }
                                if !self.cryptopanic_key.is_empty() {
                                    sources.push("CryptoPanic".into());
                                }
                                if !self.fred_key.is_empty() {
                                    sources.push("FRED".into());
                                }
                                if online {
                                    sources.push("GDELT".into());
                                    sources.push("CoinDesk".into());
                                }
                                // Any data source connected = Connected. OFFLINE only when nothing connected.
                                // Market hours per-symbol can be refined later using symbol specs.
                                let (status, color) = ("Connected", UP);
                                let status_text = if sources.is_empty() {
                                    format!("\u{25CF} {}", status)
                                } else {
                                    format!("\u{25CF} {} [{}]", status, sources.join(" + "))
                                };
                                ui.label(egui::RichText::new(status_text).color(color).small());
                                let active_session = self.charts.get(self.active_tab).and_then(|chart| {
                                    let chart_source = chart.primary_source;
                                    let symbol = chart.symbol.split(':').next_back().unwrap_or("");
                                    let normalized_symbol = normalize_market_data_symbol(symbol);
                                    let normalized_upper = normalized_symbol.to_ascii_uppercase();
                                    let bare_equity_symbol = normalized_upper
                                        .replace('/', "")
                                        .trim_end_matches(".EQ")
                                        .to_string();
                                    let kraken_equity_pair = self.kraken_scrape_xstocks
                                        && (chart_source == "kraken-equities"
                                            || normalized_upper.ends_with(".EQ")
                                            || self
                                                .kraken_equity_universe_symbols
                                                .iter()
                                                .any(|candidate| candidate.as_str() == bare_equity_symbol.as_str()));
                                    let kraken_crypto_pair = chart_source == "kraken"
                                        && !kraken_equity_pair
                                        && typhoon_engine::core::kraken::to_kraken_pair_lossy(
                                            &normalized_symbol,
                                        )
                                        .is_some();
                                    if self.kraken_connected && kraken_equity_pair {
                                        // Per-symbol session: symbols without overnight
                                        // support (catalog `overnight_trading_support`)
                                        // close 8 PM–4 AM ET instead of trading 24/5.
                                        let overnight_enabled = !self
                                            .kraken_equity_no_overnight
                                            .contains(&bare_equity_symbol);
                                        Some(kraken_xstocks_session_status_now(overnight_enabled))
                                    } else if self.kraken_connected && kraken_crypto_pair {
                                        Some("24/7".to_string())
                                    } else if self.broker_connected && !self.market_clock_status.is_empty()
                                    {
                                        Some(self.market_clock_status.clone())
                                    } else {
                                        None
                                    }
                                });
                                if let Some(session) = active_session {
                                    let session_upper = session.to_ascii_uppercase();
                                    let session_color = if session_upper.contains("CLOSED") {
                                        DOWN
                                    } else if session_upper.contains("OPEN")
                                        || session_upper.contains("KRAKEN XSTOCKS")
                                        || session == "24/7"
                                    {
                                        UP
                                    } else {
                                        AXIS_TEXT
                                    };
                                    ui.label(
                                        egui::RichText::new(format!("[{}]", session))
                                            .color(session_color)
                                            .small(),
                                    );
                                }
                                if self.kraken_connected {
                                    let kraken_balance = self.kraken_usd_equivalent_balance();
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "[Kraken (Live) ${:.0}]",
                                            kraken_balance
                                        ))
                                        .color(UP)
                                        .small(),
                                    );
                                }
                                if let Some(ref acct) = self.live_account {
                                    let mode = if self.broker_paper { "Paper" } else { "Live" };
                                    let color = if self.broker_paper {
                                        egui::Color32::WHITE
                                    } else {
                                        UP
                                    };
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "[Alpaca ({}) ${:.0}]",
                                            mode, acct.equity
                                        ))
                                        .color(color)
                                        .small(),
                                    );
                                }
                            } else {
                                let offline_sources: Vec<&str> = Vec::new();
                                let src = if offline_sources.is_empty() {
                                    "no sources".to_string()
                                } else {
                                    offline_sources.join(" + ")
                                };
                                ui.label(
                                    egui::RichText::new(format!("\u{25CB} OFFLINE [{}]", src))
                                        .color(AXIS_TEXT)
                                        .small(),
                                );
                            }
                        });
                    });
                });
    }

    pub(super) fn render_symbol_autocomplete_dropdown(&mut self, ctx: &egui::Context) {
        // ── Symbol autocomplete dropdown ─────────────────────────────────────
        if self.symbol_ac_visible && !self.symbol_suggestions.is_empty() {
            let ac_cyan = egui::Color32::from_rgb(26, 188, 156);
            let ac_dim = egui::Color32::from_rgb(100, 100, 120);
            let ac_bg = egui::Color32::from_rgb(20, 22, 35);
            let ac_sel = egui::Color32::from_rgb(30, 40, 65);

            egui::Area::new(egui::Id::new("symbol_autocomplete"))
                .fixed_pos(egui::pos2(80.0, 45.0)) // below symbol input
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::NONE
                        .fill(ac_bg)
                        .inner_margin(4.0)
                        .corner_radius(4.0)
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 70)))
                        .show(ui, |ui| {
                            // Fixed row width so the company name has a bound to
                            // truncate against — otherwise a long name (e.g.
                            // "Direxion Shares ETF Trust …") overruns and overprints
                            // the right-aligned exchange/class label.
                            const AC_ROW_W: f32 = 480.0;
                            ui.set_min_width(AC_ROW_W);
                            let suggestions: Vec<_> = self.symbol_suggestions.clone();
                            let mut clicked_sym: Option<String> = None;
                            for (idx, (sym, company, sector)) in suggestions.iter().enumerate() {
                                let selected = idx == self.symbol_ac_selected;
                                let bg = if selected { ac_sel } else { ac_bg };
                                egui::Frame::NONE.fill(bg).inner_margin(4.0).show(ui, |ui| {
                                    ui.set_min_width(AC_ROW_W);
                                    ui.set_max_width(AC_ROW_W);
                                    let resp = ui
                                        .horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(sym)
                                                    .strong()
                                                    .color(ac_cyan)
                                                    .monospace(),
                                            );
                                            // Pin the exchange/class to the far right,
                                            // then let the company name fill the gap and
                                            // truncate so the two never overlap.
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if !sector.is_empty() {
                                                        ui.label(
                                                            egui::RichText::new(sector)
                                                                .small()
                                                                .color(ac_dim),
                                                        );
                                                    }
                                                    if !company.is_empty() {
                                                        ui.add(
                                                            egui::Label::new(
                                                                egui::RichText::new(company)
                                                                    .small()
                                                                    .color(
                                                                        egui::Color32::from_rgb(
                                                                            180, 180, 190,
                                                                        ),
                                                                    ),
                                                            )
                                                            .truncate(),
                                                        );
                                                    }
                                                },
                                            );
                                        })
                                        .response;
                                    if resp.clicked() {
                                        clicked_sym = Some(sym.clone());
                                    }
                                    if resp.hovered() {
                                        self.symbol_ac_selected = idx;
                                    }
                                });
                            }
                            if let Some(sym) = clicked_sym {
                                self.symbol_input = sym.clone();
                                let tf = self
                                    .charts
                                    .get(self.active_tab)
                                    .map(|c| c.timeframe)
                                    .unwrap_or(Timeframe::H4);
                                self.reload_symbol(&sym, tf);
                                self.symbol_ac_visible = false;
                            }
                        });
                });
        }
    }
}
