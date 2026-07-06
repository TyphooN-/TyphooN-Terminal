use super::*;

use typhoon_engine::core::screener::{FieldFilter, SavedScreen, ScreenerField};

impl TyphooNApp {
    /// Finviz-style fundamentals screen (ADR-116): registry filters over the
    /// cached fundamentals universe, with kv-persisted saved screens.
    fn render_fundamentals_screen_section(&mut self, ui: &mut egui::Ui) -> SymbolAction {
        let mut pending_action = SymbolAction::None;
        // Saved screens load once per session (single kv blob).
        if !self.saved_screens_loaded {
            self.saved_screens_loaded = true;
            if let Some(ref cache) = self.cache {
                if let Ok(Some(json)) = cache.get_kv("screener:saved_screens") {
                    self.saved_screens = serde_json::from_str(&json).unwrap_or_default();
                }
            }
        }
        egui::CollapsingHeader::new("Fundamentals Screen (Finviz-style)")
            .default_open(false)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{} symbols with cached fundamentals — every registry field is a \
                         range filter (ADR-116).",
                        self.bg.all_fundamentals.len()
                    ))
                    .color(AXIS_TEXT)
                    .small(),
                );
                // Filter composer.
                ui.horizontal(|ui| {
                    let current = ScreenerField::ALL
                        .get(self.fund_screen_field_idx)
                        .copied()
                        .unwrap_or(ScreenerField::MarketCap);
                    egui::ComboBox::from_id_salt("fund_screen_field")
                        .selected_text(current.label())
                        .show_ui(ui, |ui| {
                            for (idx, f) in ScreenerField::ALL.iter().enumerate() {
                                ui.selectable_value(
                                    &mut self.fund_screen_field_idx,
                                    idx,
                                    f.label(),
                                );
                            }
                        });
                    ui.label("min");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.fund_screen_min).desired_width(70.0),
                    );
                    ui.label("max");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.fund_screen_max).desired_width(70.0),
                    );
                    if ui.button("Add filter").clicked() {
                        let min = self.fund_screen_min.trim().parse::<f64>().ok();
                        let max = self.fund_screen_max.trim().parse::<f64>().ok();
                        if min.is_some() || max.is_some() {
                            self.fund_screen_filters.push(FieldFilter {
                                field: current,
                                min,
                                max,
                            });
                        }
                    }
                });
                // Active filters.
                let mut remove_idx: Option<usize> = None;
                for (idx, f) in self.fund_screen_filters.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} ∈ [{}, {}]",
                                f.field.label(),
                                f.min.map(|v| v.to_string()).unwrap_or_else(|| "-∞".into()),
                                f.max.map(|v| v.to_string()).unwrap_or_else(|| "+∞".into()),
                            ))
                            .monospace()
                            .small(),
                        );
                        if ui.small_button("✕").clicked() {
                            remove_idx = Some(idx);
                        }
                    });
                }
                if let Some(idx) = remove_idx {
                    self.fund_screen_filters.remove(idx);
                }
                ui.horizontal(|ui| {
                    if ui.button("Run Screen").clicked() {
                        self.fund_screen_results = self
                            .bg
                            .all_fundamentals
                            .iter()
                            .filter(|f| {
                                // Descriptive row: watchlist quote when the
                                // symbol is watched, else the fundamentals'
                                // own stored price.
                                let wl = self
                                    .watchlist_rows
                                    .iter()
                                    .find(|w| w.symbol.eq_ignore_ascii_case(&f.symbol));
                                let s = typhoon_engine::core::screener::ScreenerSymbol {
                                    symbol: f.symbol.clone(),
                                    name: f.company_name.clone(),
                                    asset_class: "stock".into(),
                                    price: wl.map(|w| w.last).or(f.stock_price).unwrap_or(0.0),
                                    volume: wl.map(|w| w.volume).unwrap_or(0.0),
                                    change_pct: wl.map(|w| w.change_pct).unwrap_or(0.0),
                                    tradable: true,
                                    shortable: false,
                                    fractionable: false,
                                    sector: Some(f.sector.clone()),
                                };
                                self.fund_screen_filters
                                    .iter()
                                    .all(|filter| filter.matches(&s, Some(f)))
                            })
                            .cloned()
                            .collect();
                    }
                    ui.label(
                        egui::RichText::new(format!(
                            "{} match(es)",
                            self.fund_screen_results.len()
                        ))
                        .color(AXIS_TEXT)
                        .small(),
                    );
                    ui.separator();
                    // Saved screens (single kv blob `screener:saved_screens`).
                    ui.add(
                        egui::TextEdit::singleline(&mut self.fund_screen_name)
                            .desired_width(110.0)
                            .hint_text("screen name"),
                    );
                    if ui.button("Save").clicked() && !self.fund_screen_name.trim().is_empty() {
                        let name = self.fund_screen_name.trim().to_string();
                        self.saved_screens.retain(|s| s.name != name);
                        self.saved_screens.push(SavedScreen {
                            name,
                            filter: Default::default(),
                            field_filters: self.fund_screen_filters.clone(),
                        });
                        if let Some(ref cache) = self.cache {
                            if let Ok(json) = serde_json::to_string(&self.saved_screens) {
                                let _ = cache.put_kv("screener:saved_screens", &json);
                            }
                        }
                    }
                });
                if !self.saved_screens.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new("Saved:").color(AXIS_TEXT).small());
                        let mut delete_name: Option<String> = None;
                        for screen in self.saved_screens.clone() {
                            if ui.small_button(&screen.name).clicked() {
                                self.fund_screen_filters = screen.field_filters.clone();
                                self.fund_screen_name = screen.name.clone();
                            }
                            if ui
                                .small_button(egui::RichText::new("🗑").small())
                                .on_hover_text(format!("Delete '{}'", screen.name))
                                .clicked()
                            {
                                delete_name = Some(screen.name.clone());
                            }
                        }
                        if let Some(name) = delete_name {
                            self.saved_screens.retain(|s| s.name != name);
                            if let Some(ref cache) = self.cache {
                                if let Ok(json) = serde_json::to_string(&self.saved_screens) {
                                    let _ = cache.put_kv("screener:saved_screens", &json);
                                }
                            }
                        }
                    });
                }
                if !self.fund_screen_results.is_empty() {
                    egui::ScrollArea::vertical()
                        .id_salt("fund_screen_results")
                        .max_height(180.0)
                        .show(ui, |ui| {
                            egui::Grid::new("fund_screen_grid")
                                .striped(true)
                                .show(ui, |ui| {
                                    for h in ["Symbol", "Sector", "MCap", "P/E", "ROE", "Div %", ""]
                                    {
                                        ui.strong(h);
                                    }
                                    ui.end_row();
                                    for f in self.fund_screen_results.iter().take(200) {
                                        ui.label(egui::RichText::new(&f.symbol).monospace());
                                        ui.label(egui::RichText::new(&f.sector).small());
                                        ui.label(
                                            f.market_cap
                                                .map(|m| format!("${:.1}B", m / 1e9))
                                                .unwrap_or_else(|| "—".into()),
                                        );
                                        ui.label(
                                            f.pe_ratio
                                                .map(|v| format!("{v:.1}"))
                                                .unwrap_or_else(|| "—".into()),
                                        );
                                        ui.label(
                                            f.roe
                                                .map(|v| format!("{v:.1}%"))
                                                .unwrap_or_else(|| "—".into()),
                                        );
                                        ui.label(
                                            f.dividend_yield
                                                .map(|v| format!("{v:.2}%"))
                                                .unwrap_or_else(|| "—".into()),
                                        );
                                        if ui
                                            .small_button("+")
                                            .on_hover_text("Open new chart")
                                            .clicked()
                                        {
                                            pending_action =
                                                SymbolAction::OpenChart(f.symbol.clone());
                                        }
                                        ui.end_row();
                                    }
                                });
                        });
                }
            });
        pending_action
    }

    pub(super) fn render_symbol_screener_window(&mut self, ctx: &egui::Context) {
        // Screener — uses cached symbol data
        if self.show_screener {
            let mut pending_action = SymbolAction::None;
            let mut fund_action = SymbolAction::None;
            let mut open = self.show_screener;
            egui::Window::new("Symbol Screener")
                .open(&mut open)
                .resizable(true)
                .default_size([700.0, 480.0])
                .show(ctx, |ui| {
                    fund_action = self.render_fundamentals_screen_section(ui);
                    ui.separator();
                    let details = &self.bg.detailed_stats;
                    ui.horizontal(|ui| {
                        ui.label(format!("{} cached entries", details.len()));
                        ui.add_space(10.0);
                        ui.label("Filter:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.screener_filter)
                                .desired_width(200.0)
                                .font(egui::TextStyle::Monospace)
                                .hint_text("symbol / TF / source…"),
                        );
                        if ui.small_button("✕").clicked() {
                            self.screener_filter.clear();
                        }
                    });
                    ui.separator();
                    let filt = self.screener_filter.to_lowercase();
                    // Hide cache metadata rows (__SYMBOLS__, __SPECS__,
                    // __SERVER__, __HEARTBEAT__) — their middle parts render as
                    // bogus "symbols" in the Source/Symbol/TF grid.
                    let mut entries: Vec<&(String, i64, i64)> = details
                        .iter()
                        .filter(|(key, _, _)| !key.contains(":__"))
                        .filter(|(key, _, _)| filt.is_empty() || key.to_lowercase().contains(&filt))
                        .collect();
                    entries.sort_by(|a, b| {
                        let parse = |key: &String| -> (String, String, String) {
                            let parts: Vec<&str> = key.splitn(3, ':').collect();
                            match parts.as_slice() {
                                [source, sym, tf] => {
                                    ((*source).into(), (*sym).into(), (*tf).into())
                                }
                                [sym, tf] => ("local".into(), (*sym).into(), (*tf).into()),
                                [sym] => ("local".into(), (*sym).into(), String::new()),
                                _ => (String::new(), key.clone(), String::new()),
                            }
                        };
                        let (a_src, a_sym, a_tf) = parse(&a.0);
                        let (b_src, b_sym, b_tf) = parse(&b.0);
                        let ord = match self.screener_sort_col {
                            0 => a_src.cmp(&b_src).then_with(|| a_sym.cmp(&b_sym)),
                            1 => a_sym.cmp(&b_sym).then_with(|| a_tf.cmp(&b_tf)),
                            2 => a_tf.cmp(&b_tf).then_with(|| a_sym.cmp(&b_sym)),
                            3 => a.1.cmp(&b.1).then_with(|| a_sym.cmp(&b_sym)),
                            _ => a_sym.cmp(&b_sym),
                        };
                        if self.screener_sort_asc {
                            ord
                        } else {
                            ord.reverse()
                        }
                    });
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .max_height(360.0)
                        .show(ui, |ui| {
                            egui::Grid::new("screener_grid")
                                .striped(true)
                                .num_columns(5)
                                .min_col_width(60.0)
                                .show(ui, |ui| {
                                    sortable_header(
                                        ui,
                                        "Source",
                                        0,
                                        &mut self.screener_sort_col,
                                        &mut self.screener_sort_asc,
                                    );
                                    sortable_header(
                                        ui,
                                        "Symbol",
                                        1,
                                        &mut self.screener_sort_col,
                                        &mut self.screener_sort_asc,
                                    );
                                    sortable_header(
                                        ui,
                                        "TF",
                                        2,
                                        &mut self.screener_sort_col,
                                        &mut self.screener_sort_asc,
                                    );
                                    sortable_header(
                                        ui,
                                        "Bars",
                                        3,
                                        &mut self.screener_sort_col,
                                        &mut self.screener_sort_asc,
                                    );
                                    ui.strong("");
                                    ui.end_row();
                                    let mut load_key: Option<String> = None;
                                    for (key, count, _size) in &entries {
                                        // Parse key: "source:SYMBOL:TF" or "SYMBOL:TF"
                                        let parts: Vec<&str> = key.splitn(3, ':').collect();
                                        let (source, sym, tf) = match parts.as_slice() {
                                            [s, sym, tf] => (*s, *sym, *tf),
                                            [sym, tf] => ("local", *sym, *tf),
                                            [sym] => ("local", *sym, ""),
                                            _ => ("", key.as_str(), ""),
                                        };
                                        let src_col = match source {
                                            "alpaca" => egui::Color32::from_rgb(100, 220, 100),
                                            "kraken" => egui::Color32::from_rgb(200, 100, 255),
                                            _ => egui::Color32::from_rgb(160, 160, 160),
                                        };
                                        ui.label(
                                            egui::RichText::new(source)
                                                .color(src_col)
                                                .monospace()
                                                .small(),
                                        );
                                        ui.label(egui::RichText::new(sym).monospace());
                                        ui.label(egui::RichText::new(tf).monospace().small());
                                        ui.label(
                                            egui::RichText::new(format!("{}", count))
                                                .monospace()
                                                .color(if *count > 5000 {
                                                    egui::Color32::from_rgb(100, 220, 100)
                                                } else {
                                                    egui::Color32::from_rgb(180, 180, 180)
                                                }),
                                        );
                                        ui.horizontal(|ui| {
                                            if ui
                                                .small_button(egui::RichText::new("+").small())
                                                .on_hover_text("Open new chart")
                                                .clicked()
                                            {
                                                pending_action =
                                                    SymbolAction::OpenChart(sym.to_string());
                                            }
                                            if ui.small_button("▶ Load").clicked() {
                                                load_key = Some(key.to_string());
                                            }
                                        });
                                        ui.end_row();
                                    }
                                    // Load symbol into active chart
                                    if let Some(key) = load_key {
                                        if let Some(ref cache_arc) = self.cache {
                                            if let Some(chart) =
                                                self.charts.get_mut(self.active_tab)
                                            {
                                                match cache_arc.get_bars_raw(&key) {
                                                    Ok(Some(raw)) => {
                                                        chart.bars = raw
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
                                                        chart.view_offset =
                                                            chart.bars.len().saturating_sub(1)
                                                                + CHART_RIGHT_MARGIN;
                                                        chart.switch_symbol(bare_symbol_from_key(
                                                            &key,
                                                        ));
                                                        chart.compute_indicators();
                                                        self.log.push_back(LogEntry::info(
                                                            format!(
                                                                "Loaded {} bars from {}",
                                                                chart.bars.len(),
                                                                key
                                                            ),
                                                        ));
                                                    }
                                                    Ok(None) => {
                                                        self.log.push_back(LogEntry::warn(
                                                            format!("No data for {}", key),
                                                        ));
                                                    }
                                                    Err(e) => {
                                                        self.log.push_back(LogEntry::err(format!(
                                                            "Load error: {}",
                                                            e
                                                        )));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                });
                        });
                });
            self.show_screener = open;
            self.apply_symbol_action(pending_action);
            self.apply_symbol_action(fund_action);
        }
    }
}
