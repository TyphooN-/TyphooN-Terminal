use super::*;

impl TyphooNApp {
    pub(super) fn render_trading_tools_windows(&mut self, ctx: &egui::Context) {
        // Order Flow
        if self.show_order_flow {
            egui::Window::new("Order Flow")
                .open(&mut self.show_order_flow)
                .resizable(true)
                .default_size([500.0, 450.0])
                .show(ctx, |ui| {
                    let of_green = egui::Color32::from_rgb(0, 200, 80);
                    let of_red = egui::Color32::from_rgb(220, 50, 50);
                    let of_dim = egui::Color32::from_rgb(80, 80, 100);

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
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("Order Flow: {}", sym)).strong());
                        if ui.button("Fetch L2").clicked() && !sym.is_empty() {
                            let _ = self.broker_tx.send(BrokerCmd::GetOrderbook {
                                symbol: sym.clone(),
                            });
                        }
                        let stream_supported =
                            kraken_bookmap_stream_supported(&sym, &self.kraken_pairs);
                        let stream_button =
                            ui.add_enabled(stream_supported, egui::Button::new("Stream L2"));
                        if stream_button.clicked() && !sym.is_empty() {
                            let _ = self.broker_tx.send(BrokerCmd::KrakenStartOrderbookWs {
                                symbol: sym.clone(),
                                depth: 100,
                            });
                        }
                        if !stream_supported && !sym.is_empty() {
                            stream_button.on_hover_text(
                                "Live Kraken depth is only available for Kraken spot pairs.",
                            );
                        }
                    });
                    ui.separator();

                    if let Some(chart) = self.charts.get(self.active_tab) {
                        let bars = &chart.bars;
                        let n = bars.len();
                        if n > 10 {
                            let recent = &bars[n.saturating_sub(60)..];

                            // Cumulative Delta (buying vs selling pressure proxy)
                            ui.label(
                                egui::RichText::new("Cumulative Delta (volume × direction)")
                                    .small()
                                    .strong(),
                            );
                            let mut cum_delta = Vec::with_capacity(recent.len());
                            let mut running = 0.0_f64;
                            for b in recent {
                                let delta = if b.close >= b.open {
                                    b.volume
                                } else {
                                    -b.volume
                                };
                                running += delta;
                                cum_delta.push(running);
                            }
                            {
                                let pts: PlotPoints = PlotPoints::new(
                                    cum_delta
                                        .iter()
                                        .enumerate()
                                        .map(|(i, &d)| [i as f64, d])
                                        .collect(),
                                );
                                let c = if *cum_delta.last().unwrap_or(&0.0) >= 0.0 {
                                    of_green
                                } else {
                                    of_red
                                };
                                let line = Line::new("Cum Delta", pts).color(c).width(1.5);
                                Plot::new("cum_delta_plot")
                                    .height(100.0)
                                    .allow_drag(false)
                                    .allow_zoom(false)
                                    .allow_scroll(false)
                                    .show_axes([false, true])
                                    .show(ui, |plot_ui| {
                                        plot_ui.line(line);
                                    });
                            }

                            // Per-bar Delta bars
                            ui.label(egui::RichText::new("Per-Bar Delta").small().strong());
                            {
                                let bars_plot: Vec<PlotBar> = recent
                                    .iter()
                                    .enumerate()
                                    .map(|(i, b)| {
                                        let delta = if b.close >= b.open {
                                            b.volume
                                        } else {
                                            -b.volume
                                        };
                                        let c = if delta >= 0.0 { of_green } else { of_red };
                                        PlotBar::new(i as f64, delta).width(0.8).fill(c)
                                    })
                                    .collect();
                                let chart = BarChart::new("Delta", bars_plot);
                                Plot::new("delta_bars")
                                    .height(80.0)
                                    .allow_drag(false)
                                    .allow_zoom(false)
                                    .allow_scroll(false)
                                    .show_axes([false, true])
                                    .show(ui, |plot_ui| {
                                        plot_ui.bar_chart(chart);
                                    });
                            }

                            // Footprint-style summary (price levels with buy/sell volume)
                            ui.label(
                                egui::RichText::new("Footprint Summary (last 20 bars)")
                                    .small()
                                    .strong(),
                            );
                            let last20 = &recent[recent.len().saturating_sub(20)..];
                            let min_p = last20.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                            let max_p = last20.iter().map(|b| b.high).fold(f64::MIN, f64::max);
                            let range = max_p - min_p;
                            if range > 0.0 {
                                let levels = 15_usize;
                                let step = range / levels as f64;
                                let mut buy_vol = vec![0.0_f64; levels];
                                let mut sell_vol = vec![0.0_f64; levels];
                                for b in last20 {
                                    let mid_level =
                                        ((((b.high + b.low) / 2.0) - min_p) / step) as usize;
                                    let idx = mid_level.min(levels - 1);
                                    if b.close >= b.open {
                                        buy_vol[idx] += b.volume;
                                    } else {
                                        sell_vol[idx] += b.volume;
                                    }
                                }

                                let max_vol = buy_vol
                                    .iter()
                                    .chain(sell_vol.iter())
                                    .cloned()
                                    .fold(0.0_f64, f64::max);
                                let avail_w = ui.available_width();
                                for i in (0..levels).rev() {
                                    let price = min_p + (i as f64 + 0.5) * step;
                                    let bv = buy_vol[i];
                                    let sv = sell_vol[i];
                                    let b_frac = if max_vol > 0.0 {
                                        (bv / max_vol) as f32
                                    } else {
                                        0.0
                                    };
                                    let s_frac = if max_vol > 0.0 {
                                        (sv / max_vol) as f32
                                    } else {
                                        0.0
                                    };

                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(format_price(price))
                                                .monospace()
                                                .small()
                                                .color(of_dim),
                                        );
                                        let (rect, _) = ui.allocate_exact_size(
                                            egui::vec2(avail_w - 80.0, 12.0),
                                            egui::Sense::hover(),
                                        );
                                        let painter = ui.painter_at(rect);
                                        let mid_x = rect.left() + rect.width() / 2.0;
                                        // Buy bar (extends right from center)
                                        painter.rect_filled(
                                            egui::Rect::from_min_size(
                                                egui::pos2(mid_x, rect.top()),
                                                egui::vec2(b_frac * rect.width() / 2.0, 12.0),
                                            ),
                                            0.0,
                                            of_green,
                                        );
                                        // Sell bar (extends left from center)
                                        painter.rect_filled(
                                            egui::Rect::from_min_size(
                                                egui::pos2(
                                                    mid_x - s_frac * rect.width() / 2.0,
                                                    rect.top(),
                                                ),
                                                egui::vec2(s_frac * rect.width() / 2.0, 12.0),
                                            ),
                                            0.0,
                                            of_red,
                                        );
                                    });
                                }
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Sells ←").color(of_red).small());
                                    ui.label(egui::RichText::new("→ Buys").color(of_green).small());
                                });
                            }
                        } else {
                            ui.label(egui::RichText::new("Load chart data first.").color(of_dim));
                        }
                    }
                });
        }

        // Bookmap — one floating heatmap per requested symbol.
        if self.show_bookmap {
            self.open_bookmap_window(None);
            self.show_bookmap = false;
        }
        let mut open_bookmaps = Vec::with_capacity(self.bookmap_windows.len());
        for window in std::mem::take(&mut self.bookmap_windows) {
            let sym = window.symbol;
            let mut open = window.open;
            let title = format!("Bookmap Heatmap — {sym}");
            egui::Window::new(title)
                        .id(egui::Id::new(("bookmap_heatmap", sym.as_str())))
                        .open(&mut open)
                        .resizable(true)
                        .default_size([600.0, 450.0])
                        .show(ctx, |ui| {
                            let bm_green = egui::Color32::from_rgb(0, 180, 80);
                            let bm_red = egui::Color32::from_rgb(200, 50, 50);
                            let bm_dim = egui::Color32::from_rgb(80, 80, 100);

                            let stream_supported = kraken_bookmap_stream_supported(&sym, &self.kraken_pairs);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(format!("Depth: {sym}")).strong());
                                if ui.button("Fetch Depth").clicked() && !sym.is_empty() {
                                    let _ = self.broker_tx.send(BrokerCmd::GetOrderbook {
                                        symbol: sym.clone(),
                                    });
                                }
                                let stream_button = ui.add_enabled(
                                    stream_supported,
                                    egui::Button::new("Stream Depth"),
                                );
                                if stream_button.clicked() && !sym.is_empty() {
                                    let _ = self.broker_tx.send(BrokerCmd::KrakenStartOrderbookWs {
                                        symbol: sym.clone(),
                                        depth: 100,
                                    });
                                }
                                if !stream_supported && !sym.is_empty() {
                                    stream_button.on_hover_text("Live Kraken depth is only available for Kraken spot pairs, not equity symbols.");
                                }
                                ui.label(egui::RichText::new("L2 depth").color(bm_dim).small());
                            });
                            ui.separator();

                            if orderbook_json_matches_symbol(&self.orderbook_result, &sym)
                                && render_live_orderbook_heatmap(
                                    ui,
                                    &self.orderbook_result,
                                    bm_green,
                                    bm_red,
                                    bm_dim,
                                )
                            {
                                ui.separator();
                            }

                            // Render depth heatmap from the requested symbol's chart data.
                            let chart = self.charts.iter().find(|chart| {
                                normalize_market_data_symbol(&chart.symbol).eq_ignore_ascii_case(&sym)
                            });
                            if let Some(chart) = chart {
                                let bars = &chart.bars;
                                let n = bars.len();
                                if n > 20 {
                                    // Build a price × time volume heatmap from recent bars
                                    let recent = &bars[n.saturating_sub(100)..];
                                    let min_p = recent.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                                    let max_p = recent.iter().map(|b| b.high).fold(f64::MIN, f64::max);
                                    let price_range = max_p - min_p;
                                    if price_range > 0.0 {
                                        let rows = 40_usize; // price levels
                                        let cols = recent.len();

                                        // Allocate and paint heatmap
                                        let avail = ui.available_size();
                                        let w = avail.x.min(580.0);
                                        let h = 300.0_f32;
                                        let (rect, _) =
                                            ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
                                        let painter = ui.painter_at(rect);
                                        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(5, 5, 15));

                                        let cell_w = w / cols as f32;
                                        let cell_h = h / rows as f32;

                                        for (col, bar) in recent.iter().enumerate() {
                                            let x = rect.left() + col as f32 * cell_w;
                                            // Map bar's high-low range to row indices
                                            let row_lo =
                                                ((bar.low - min_p) / price_range * rows as f64) as usize;
                                            let row_hi =
                                                ((bar.high - min_p) / price_range * rows as f64) as usize;
                                            let vol_norm =
                                                (bar.volume.ln().max(0.0) / 15.0).min(1.0) as f32;

                                            for row in row_lo..=row_hi.min(rows - 1) {
                                                let y = rect.bottom() - (row as f32 + 1.0) * cell_h;
                                                let intensity = vol_norm * 0.8;
                                                let color = if bar.close >= bar.open {
                                                    egui::Color32::from_rgba_premultiplied(
                                                        0,
                                                        (intensity * 200.0) as u8,
                                                        (intensity * 80.0) as u8,
                                                        (intensity * 255.0) as u8,
                                                    )
                                                } else {
                                                    egui::Color32::from_rgba_premultiplied(
                                                        (intensity * 200.0) as u8,
                                                        (intensity * 50.0) as u8,
                                                        0,
                                                        (intensity * 255.0) as u8,
                                                    )
                                                };
                                                painter.rect_filled(
                                                    egui::Rect::from_min_size(
                                                        egui::pos2(x, y),
                                                        egui::vec2(cell_w, cell_h),
                                                    ),
                                                    0.0,
                                                    color,
                                                );
                                            }
                                        }

                                        // Price axis labels
                                        for i in 0..=4 {
                                            let frac = i as f64 / 4.0;
                                            let price = min_p + frac * price_range;
                                            let y = rect.bottom() - frac as f32 * h;
                                            painter.text(
                                                egui::pos2(rect.right() - 2.0, y),
                                                egui::Align2::RIGHT_CENTER,
                                                format_price(price),
                                                egui::FontId::monospace(9.0),
                                                bm_dim,
                                            );
                                        }

                                        // Legend
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("Bid Volume").color(bm_green).small(),
                                            );
                                            ui.label(
                                                egui::RichText::new("Ask Volume").color(bm_red).small(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{} bars × {} levels",
                                                    cols, rows
                                                ))
                                                .color(bm_dim)
                                                .small(),
                                            );
                                        });
                                    }
                                } else {
                                    ui.label(egui::RichText::new("Load chart data first.").color(bm_dim));
                                }
                            } else {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "No open chart data for {sym}. Open/load the symbol chart first."
                                    ))
                                    .color(bm_dim),
                                );
                            }
                        });
            if open {
                open_bookmaps.push(BookmapWindowState { symbol: sym, open });
            }
        }
        self.bookmap_windows = open_bookmaps;

        // Orderbook DOM — shows real L2 data from Fetch Depth/Fetch L2
        if self.show_orderbook_window {
            egui::Window::new("Orderbook DOM")
                        .open(&mut self.show_orderbook_window)
                        .resizable(true).default_size([360.0, 420.0])
                        .show(ctx, |ui| {
                            let ob_bid = egui::Color32::from_rgb(0, 200, 80);
                            let ob_ask = egui::Color32::from_rgb(220, 50, 50);
                            let ob_dim = egui::Color32::from_rgb(80, 80, 100);
                            if self.orderbook_result.is_empty() {
                                ui.label(egui::RichText::new("No L2 data — click Fetch Depth in Bookmap or Fetch L2 in Order Flow.").color(ob_dim).small());
                            } else if let Ok(v) = serde_json::from_str::<serde_json::Value>(&self.orderbook_result) {
                                let sym = v["symbol"].as_str().unwrap_or("?");
                                let ts  = v["timestamp"].as_str().unwrap_or("");
                                ui.label(egui::RichText::new(format!("{} — {}", sym, ts)).strong().small());
                                ui.separator();
                                let bids = v["bids"].as_array().map(|a| a.as_slice()).unwrap_or(&[]);
                                let asks = v["asks"].as_array().map(|a| a.as_slice()).unwrap_or(&[]);
                                // max size for bar scaling
                                let max_sz = bids.iter().chain(asks.iter())
                                    .filter_map(|e| e["size"].as_f64())
                                    .fold(0.0_f64, f64::max).max(1.0);
                                let avail_w = ui.available_width().min(320.0);
                                egui::ScrollArea::vertical().auto_shrink(false).max_height(340.0).show(ui, |ui| {
                                    ui.label(egui::RichText::new("Asks (sell side)").color(ob_ask).small().strong());
                                    for ask in asks.iter().rev().take(15) {
                                        let price = ask["price"].as_f64().unwrap_or(0.0);
                                        let size  = ask["size"].as_f64().unwrap_or(0.0);
                                        let frac  = (size / max_sz) as f32;
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(format_price(price)).monospace().small().color(ob_ask));
                                            let (rect, _) = ui.allocate_exact_size(egui::vec2(avail_w - 90.0, 10.0), egui::Sense::hover());
                                            ui.painter_at(rect).rect_filled(
                                                egui::Rect::from_min_size(rect.min, egui::vec2(frac * rect.width(), 10.0)),
                                                0.0, egui::Color32::from_rgba_premultiplied(200, 40, 40, 120));
                                            ui.label(egui::RichText::new(format!("{:.4}", size)).monospace().small().color(ob_dim));
                                        });
                                    }
                                    ui.separator();
                                    ui.label(egui::RichText::new("Bids (buy side)").color(ob_bid).small().strong());
                                    for bid in bids.iter().take(15) {
                                        let price = bid["price"].as_f64().unwrap_or(0.0);
                                        let size  = bid["size"].as_f64().unwrap_or(0.0);
                                        let frac  = (size / max_sz) as f32;
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(format_price(price)).monospace().small().color(ob_bid));
                                            let (rect, _) = ui.allocate_exact_size(egui::vec2(avail_w - 90.0, 10.0), egui::Sense::hover());
                                            ui.painter_at(rect).rect_filled(
                                                egui::Rect::from_min_size(rect.min, egui::vec2(frac * rect.width(), 10.0)),
                                                0.0, egui::Color32::from_rgba_premultiplied(0, 180, 60, 120));
                                            ui.label(egui::RichText::new(format!("{:.4}", size)).monospace().small().color(ob_dim));
                                        });
                                    }
                                });
                            } else {
                                ui.label(egui::RichText::new("Failed to parse orderbook data.").color(ob_ask).small());
                            }
                        });
        }

        // MQL5/PineScript Indicator Compiler
        if self.show_indicator_compiler {
            egui::Window::new("Indicator Compiler")
                .open(&mut self.show_indicator_compiler)
                .resizable(true)
                .default_size([650.0, 550.0])
                .max_size([650.0, 560.0])
                .show(ctx, |ui| {
                    let cc_green = egui::Color32::from_rgb(46, 204, 113);
                    let cc_red = egui::Color32::from_rgb(231, 76, 60);
                    let cc_dim = egui::Color32::from_rgb(100, 100, 120);
                    // Language table — kept adjacent to the match arms below so
                    // they stay in sync if we add another frontend.
                    const LANG_LABELS: &[&str] = &[
                        "MQL5",
                        "MQL4",
                        "PineScript",
                        "EasyLanguage",
                        "thinkScript",
                        "AFL (AmiBroker)",
                        "ProBuilder",
                        "NinjaScript",
                        "cAlgo (cTrader)",
                        "ACSIL (Sierra Chart)",
                    ];
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Language:").small());
                        egui::ComboBox::from_id_salt("compiler_lang")
                            .selected_text(
                                LANG_LABELS
                                    .get(self.compiler_language)
                                    .copied()
                                    .unwrap_or("MQL5"),
                            )
                            .width(180.0)
                            .show_ui(ui, |ui| {
                                for (i, label) in LANG_LABELS.iter().enumerate() {
                                    ui.selectable_value(&mut self.compiler_language, i, *label);
                                }
                            });
                        if ui.button("Load File...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter(
                                    "Indicator",
                                    &[
                                        "mq5", "mqh", // MQL5
                                        "mq4", "mqh",  // MQL4
                                        "pine", // PineScript
                                        "el", "els", // EasyLanguage
                                        "ts", "tos", // thinkScript
                                        "afl", // AFL
                                        "itf", // ProBuilder
                                        "cs",  // NinjaScript + cAlgo (C#)
                                        "cpp", "h", // ACSIL (Sierra Chart)
                                        "txt",
                                    ],
                                )
                                .pick_file()
                            {
                                if let Ok(contents) = std::fs::read_to_string(&path) {
                                    self.compiler_source = contents;
                                    // Auto-detect language by extension / content
                                    self.compiler_language = match path
                                        .extension()
                                        .and_then(|e| e.to_str())
                                    {
                                        Some("mq4") => 1,
                                        Some("pine") => 2,
                                        Some("el") | Some("els") => 3,
                                        Some("ts") | Some("tos") => 4,
                                        Some("afl") => 5,
                                        Some("itf") => 6,
                                        Some("cs") => {
                                            // Disambiguate NinjaScript vs cAlgo by content
                                            if self.compiler_source.contains("NinjaScriptProperty")
                                                || self.compiler_source.contains("NinjaTrader")
                                            {
                                                7
                                            } else {
                                                8
                                            }
                                        }
                                        Some("cpp") | Some("h") => {
                                            // Sierra Chart ACSIL if it contains SierraChart.h or SCSF
                                            if self.compiler_source.contains("SierraChart.h")
                                                || self.compiler_source.contains("SCSFExport")
                                                || self
                                                    .compiler_source
                                                    .contains("SCStudyInterfaceRef")
                                            {
                                                9
                                            } else {
                                                0
                                            }
                                        }
                                        _ => 0,
                                    };
                                    self.log.push_back(LogEntry::info(format!(
                                        "Loaded: {}",
                                        path.display()
                                    )));
                                }
                            }
                        }
                        let compile_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("Compile").color(egui::Color32::WHITE),
                            )
                            .fill(BTN_BLUE),
                        );
                        if compile_btn.clicked() && !self.compiler_source.is_empty() {
                            let result = match self.compiler_language {
                                0 => mql5_compiler::compile_mql5(&self.compiler_source),
                                1 => mql5_compiler::compile_mql4(&self.compiler_source),
                                2 => mql5_compiler::compile_pine(&self.compiler_source),
                                3 => mql5_compiler::compile_easylang(&self.compiler_source),
                                4 => mql5_compiler::compile_thinkscript(&self.compiler_source),
                                5 => mql5_compiler::compile_afl(&self.compiler_source),
                                6 => mql5_compiler::compile_probuilder(&self.compiler_source),
                                7 => mql5_compiler::compile_ninjascript(&self.compiler_source),
                                8 => mql5_compiler::compile_calgo(&self.compiler_source),
                                9 => mql5_compiler::compile_acsil(&self.compiler_source),
                                _ => mql5_compiler::compile_mql5(&self.compiler_source),
                            };
                            self.compiler_diagnostics.clear();
                            for d in &result.diagnostics {
                                self.compiler_diagnostics.push_back(format!(
                                    "{}:{}: {} — {}",
                                    d.line,
                                    d.col,
                                    match d.level {
                                        mql5_compiler::DiagLevel::Error => "ERROR",
                                        mql5_compiler::DiagLevel::Warning => "WARN",
                                        _ => "INFO",
                                    },
                                    d.message
                                ));
                            }
                            if result.wasm.is_some() {
                                let wasm_size = result.wasm.as_ref().map(|w| w.len()).unwrap_or(0);
                                let buffers =
                                    result.metadata.as_ref().map(|m| m.buffers).unwrap_or(0);
                                let inputs = result
                                    .metadata
                                    .as_ref()
                                    .map(|m| m.inputs.len())
                                    .unwrap_or(0);
                                self.compiler_diagnostics.push_front(format!(
                                    "OK: compiled to {} bytes WASM — {} buffers, {} inputs",
                                    wasm_size, buffers, inputs
                                ));
                                self.log.push_back(LogEntry::info(format!(
                                    "Compiled: {} bytes WASM, {} buffers",
                                    wasm_size, buffers
                                )));
                            }
                            self.compiler_metadata = Some(result);
                        }
                    });

                    // ── Cross-language transpile row (ADR-090) ────────
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Transpile to:").small());
                        const TRANSPILE_TARGETS: &[(
                            &str,
                            mql5_compiler::transpile::TargetLanguage,
                        )] = &[
                            ("MQL5", mql5_compiler::transpile::TargetLanguage::Mql5),
                            ("MQL4", mql5_compiler::transpile::TargetLanguage::Mql4),
                            (
                                "PineScript v5",
                                mql5_compiler::transpile::TargetLanguage::PineScript,
                            ),
                            (
                                "EasyLanguage",
                                mql5_compiler::transpile::TargetLanguage::EasyLanguage,
                            ),
                            (
                                "thinkScript",
                                mql5_compiler::transpile::TargetLanguage::ThinkScript,
                            ),
                            (
                                "AFL (AmiBroker)",
                                mql5_compiler::transpile::TargetLanguage::Afl,
                            ),
                            (
                                "ProBuilder",
                                mql5_compiler::transpile::TargetLanguage::ProBuilder,
                            ),
                            (
                                "NinjaScript",
                                mql5_compiler::transpile::TargetLanguage::NinjaScript,
                            ),
                            (
                                "cAlgo (cTrader)",
                                mql5_compiler::transpile::TargetLanguage::Calgo,
                            ),
                            (
                                "ACSIL (Sierra Chart)",
                                mql5_compiler::transpile::TargetLanguage::Acsil,
                            ),
                        ];
                        egui::ComboBox::from_id_salt("compiler_transpile_target")
                            .selected_text(
                                TRANSPILE_TARGETS
                                    .get(self.compiler_transpile_target)
                                    .map(|(l, _)| *l)
                                    .unwrap_or("MQL5"),
                            )
                            .width(180.0)
                            .show_ui(ui, |ui| {
                                for (i, (label, _)) in TRANSPILE_TARGETS.iter().enumerate() {
                                    ui.selectable_value(
                                        &mut self.compiler_transpile_target,
                                        i,
                                        *label,
                                    );
                                }
                            });
                        if ui.button("Transpile").clicked() && !self.compiler_source.is_empty() {
                            use mql5_compiler::transpile::{SourceLanguage, transpile};
                            let from = match self.compiler_language {
                                0 => SourceLanguage::Mql5,
                                1 => SourceLanguage::Mql4,
                                2 => SourceLanguage::PineScript,
                                3 => SourceLanguage::EasyLanguage,
                                4 => SourceLanguage::ThinkScript,
                                5 => SourceLanguage::Afl,
                                6 => SourceLanguage::ProBuilder,
                                7 => SourceLanguage::NinjaScript,
                                8 => SourceLanguage::Calgo,
                                9 => SourceLanguage::Acsil,
                                _ => SourceLanguage::Mql5,
                            };
                            let to = TRANSPILE_TARGETS
                                .get(self.compiler_transpile_target)
                                .map(|(_, t)| *t)
                                .unwrap_or(mql5_compiler::transpile::TargetLanguage::Mql5);
                            match transpile(&self.compiler_source, from, to) {
                                Ok(out) => {
                                    let line_count = out.lines().count();
                                    self.compiler_transpiled = Some(out);
                                    self.log.push_back(LogEntry::info(format!(
                                        "Transpiled {:?} → {:?}: {} lines",
                                        from, to, line_count
                                    )));
                                }
                                Err(e) => {
                                    self.compiler_transpiled = None;
                                    self.log
                                        .push_back(LogEntry::err(format!("Transpile failed: {e}")));
                                    self.compiler_diagnostics
                                        .push_front(format!("TRANSPILE ERROR: {e}"));
                                }
                            }
                        }
                        if self.compiler_transpiled.is_some()
                            && ui.button("Use as Source").clicked()
                        {
                            if let Some(ref out) = self.compiler_transpiled {
                                self.compiler_source = out.clone();
                                // Map transpile-target index → language dropdown index.
                                // Transpile targets: 0=MQL5 1=MQL4 2=Pine 3=EL 4=TS 5=AFL 6=PB 7=Ninja 8=cAlgo
                                // Language dropdown: 0=MQL5 1=MQL4 2=Pine 3=EL 4=TS 5=AFL 6=PB 7=Ninja 8=cAlgo
                                // They happen to line up 1:1 after Phase 2.
                                self.compiler_language = self.compiler_transpile_target;
                                self.compiler_transpiled = None;
                            }
                        }
                        if self.compiler_transpiled.is_some() && ui.button("Copy").clicked() {
                            if let Some(ref out) = self.compiler_transpiled {
                                ui.ctx().copy_text(out.clone());
                            }
                        }
                    });
                    ui.separator();

                    // Source code editor
                    ui.label(egui::RichText::new("Source Code").small().strong());
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .max_height(280.0)
                        .id_salt("compiler_src")
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.compiler_source)
                                    .code_editor()
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(16)
                                    .font(egui::TextStyle::Monospace),
                            );
                        });
                    ui.separator();

                    // Diagnostics
                    if !self.compiler_diagnostics.is_empty() {
                        ui.label(egui::RichText::new("Diagnostics").small().strong());
                        egui::ScrollArea::vertical()
                            .auto_shrink(false)
                            .max_height(120.0)
                            .id_salt("compiler_diag")
                            .show(ui, |ui| {
                                for d in &self.compiler_diagnostics {
                                    let c = if d.starts_with("OK:") {
                                        cc_green
                                    } else if d.contains("ERROR") {
                                        cc_red
                                    } else {
                                        cc_dim
                                    };
                                    ui.label(egui::RichText::new(d).monospace().small().color(c));
                                }
                            });
                    }

                    // Metadata summary
                    if let Some(ref result) = self.compiler_metadata {
                        if let Some(ref meta) = result.metadata {
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("Name: {}", meta.short_name))
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("Buffers: {}", meta.buffers))
                                        .color(cc_dim)
                                        .small(),
                                );
                                ui.label(
                                    egui::RichText::new(if meta.separate_window {
                                        "Separate Window"
                                    } else {
                                        "Chart Overlay"
                                    })
                                    .color(cc_dim)
                                    .small(),
                                );
                            });
                            if !meta.inputs.is_empty() {
                                ui.label(egui::RichText::new("Inputs:").small());
                                for inp in &meta.inputs {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "  {} ({}) = {}",
                                            inp.name, inp.param_type, inp.default_value
                                        ))
                                        .monospace()
                                        .small()
                                        .color(cc_dim),
                                    );
                                }
                            }
                            if !meta.plots.is_empty() {
                                ui.label(egui::RichText::new("Plots:").small());
                                for p in &meta.plots {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "  [{}] {} — {:?} color={}",
                                            p.index, p.label, p.draw_type, p.color
                                        ))
                                        .monospace()
                                        .small()
                                        .color(cc_dim),
                                    );
                                }
                            }
                        }
                    }

                    // Transpiled output panel
                    if let Some(ref transpiled) = self.compiler_transpiled {
                        ui.separator();
                        ui.label(
                            egui::RichText::new("Transpiled Output")
                                .small()
                                .strong()
                                .color(cc_green),
                        );
                        egui::ScrollArea::vertical()
                            .auto_shrink(false)
                            .max_height(200.0)
                            .id_salt("compiler_transpile_out")
                            .show(ui, |ui| {
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(transpiled).monospace().small(),
                                    )
                                    .wrap_mode(egui::TextWrapMode::Extend),
                                );
                            });
                    }
                });
        }
    }
}
