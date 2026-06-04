use super::*;

impl TyphooNApp {
    pub(super) fn render_symbol_screener_window(&mut self, ctx: &egui::Context) {
        // Screener — uses cached symbol data
        if self.show_screener {
            egui::Window::new("Symbol Screener")
                .open(&mut self.show_screener)
                .resizable(true)
                .default_size([700.0, 480.0])
                .show(ctx, |ui| {
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
                    // Hide BarCacheWriter metadata rows (__SYMBOLS__, __SPECS__,
                    // __SERVER__, __HEARTBEAT__) — their middle parts render as
                    // bogus "symbols" in the Source/Symbol/TF grid.
                    let mut entries: Vec<&(String, i64, i64)> = details
                        .iter()
                        .filter(|(key, _, _)| !key.starts_with("mt5:__"))
                        .filter(|(key, _, _)| filt.is_empty() || key.to_lowercase().contains(&filt))
                        .collect();
                    entries.sort_by(|a, b| {
                        let parse = |key: &String| -> (String, String, String) {
                            let parts: Vec<&str> = key.splitn(3, ':').collect();
                            match parts.as_slice() {
                                [source, sym, tf] => {
                                    ((*source).into(), (*sym).into(), (*tf).into())
                                }
                                [sym, tf] => ("mt5".into(), (*sym).into(), (*tf).into()),
                                [sym] => ("mt5".into(), (*sym).into(), String::new()),
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
                                            [sym, tf] => ("mt5", *sym, *tf),
                                            [sym] => ("mt5", *sym, ""),
                                            _ => ("", key.as_str(), ""),
                                        };
                                        let src_col = match source {
                                            "mt5" => egui::Color32::from_rgb(100, 180, 255),
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
                                        if ui.small_button("▶ Load").clicked() {
                                            load_key = Some(key.to_string());
                                        }
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
                                                        chart.symbol = bare_symbol_from_key(&key);
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
        }
    }
}
