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
                        let l2_supported =
                            kraken_bookmap_stream_supported(&sym, &self.kraken_pairs);
                        if ui
                            .add_enabled(l2_supported, egui::Button::new("Fetch L2"))
                            .clicked()
                            && !sym.is_empty()
                        {
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
                                depth: self.dom_depth,
                                publish_dom: true,
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

                            // Footprint-style summary with +/- control
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Footprint Summary").small().strong());
                                if ui.small_button("−").clicked() {
                                    self.order_flow_footprint_bars =
                                        (self.order_flow_footprint_bars.saturating_sub(10)).max(10);
                                }
                                ui.label(format!("last {} bars", self.order_flow_footprint_bars));
                                if ui.small_button("+").clicked() {
                                    self.order_flow_footprint_bars =
                                        (self.order_flow_footprint_bars + 10).min(200);
                                }
                            });
                            let footprint_bars = self.order_flow_footprint_bars;
                            let last_n = &recent[recent.len().saturating_sub(footprint_bars)..];
                            let min_p = last_n.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                            let max_p = last_n.iter().map(|b| b.high).fold(f64::MIN, f64::max);
                            let range = max_p - min_p;
                            if range > 0.0 {
                                let levels = 15_usize;
                                let step = range / levels as f64;
                                let mut buy_vol = vec![0.0_f64; levels];
                                let mut sell_vol = vec![0.0_f64; levels];
                                for b in last_n {
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
            let mut selected_order_id = window.selected_order_id;
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
                                ui.label("Depth:");
                                let mut depth = self.dom_depth as i32;
                                ui.add(egui::Slider::new(&mut depth, 10..=250).step_by(10.0));
                                self.dom_depth = depth.clamp(10, 250) as usize;
                                let stream_button = ui.add_enabled(
                                    stream_supported,
                                    egui::Button::new("Stream Depth"),
                                );
                                if stream_button.clicked() && !sym.is_empty() {
                                    let _ = self.broker_tx.send(BrokerCmd::KrakenStartOrderbookWs {
                                        symbol: sym.clone(),
                                        depth: self.dom_depth,
                                        publish_dom: true,
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
                                    &mut selected_order_id,
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
                open_bookmaps.push(BookmapWindowState {
                    symbol: sym,
                    open,
                    selected_order_id,
                });
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

                            // Derive sym for buttons: prefer last result, fall back to active chart
                            let dom_sym = if let Ok(v) = serde_json::from_str::<serde_json::Value>(&self.orderbook_result) {
                                v["symbol"].as_str().unwrap_or("").to_string()
                            } else {
                                self.charts.get(self.active_tab)
                                    .map(|c| c.symbol.split(':').rev().nth(1).or_else(|| c.symbol.split(':').last()).unwrap_or("").to_string())
                                    .unwrap_or_default()
                            };

                            // Follow-up: depth preference slider (session-persisted for L2 DOM)
                            ui.horizontal(|ui| {
                                ui.label("Depth:");
                                let mut d = self.dom_depth as i32;
                                ui.add(egui::Slider::new(&mut d, 10..=250).step_by(10.0));
                                self.dom_depth = d.clamp(10, 250) as usize;
                                if ui.button("Apply to Stream").clicked() && !dom_sym.is_empty() {
                                    // Use preferred depth when (re)starting
                                    let _ = self.broker_tx.send(BrokerCmd::KrakenStartOrderbookWs {
                                        symbol: dom_sym.clone(),
                                        depth: self.dom_depth,
                                        publish_dom: true,
                                    });
                                }
                            });

                            // Rich L2 polish: refresh + stream + L3 trigger buttons
                            ui.horizontal(|ui| {
                                if ui.button("Refresh L2").clicked() && !dom_sym.is_empty() {
                                    let _ = self.broker_tx.send(BrokerCmd::GetOrderbook { symbol: dom_sym.clone() });
                                }
                                ui.label(egui::RichText::new("(snapshots; Kraken streams)").small().color(ob_dim));
                                if ui.add_enabled(
                                    kraken_bookmap_stream_supported(&dom_sym, &self.kraken_pairs),
                                    egui::Button::new("Start Stream").small()
                                ).clicked() && !dom_sym.is_empty() {
                                    let _ = self.broker_tx.send(BrokerCmd::KrakenStartOrderbookWs {
                                        symbol: dom_sym.clone(),
                                        depth: self.dom_depth,
                                        publish_dom: true,
                                    });
                                }
                                // L3 foundation trigger (polish 6): real L3 is opt-in and entitlement-gated.
                                let l3_supported =
                                    kraken_bookmap_stream_supported(&dom_sym, &self.kraken_pairs);
                                let l3_button = ui.add_enabled(
                                    l3_supported && !dom_sym.is_empty(),
                                    egui::Button::new("Start L3 (Kraken)").small(),
                                );
                                if l3_button.clicked() {
                                    self.kraken_l3_status = format!(
                                        "L3 requested for {} (awaiting auth entitlement + stream)",
                                        dom_sym
                                    );
                                    let _ = self.broker_tx.send(BrokerCmd::KrakenStartLevel3Ws {
                                        symbol: dom_sym.clone(),
                                    });
                                }
                                if !l3_supported && !dom_sym.is_empty() {
                                    l3_button.on_hover_text(
                                        "Real Kraken L3 is only available for supported Kraken spot pairs and still requires auth entitlements. Use L3 Demo to exercise the UI path.",
                                    );
                                } else {
                                    l3_button.on_hover_text(
                                        "Start Kraken authenticated L3 for this spot pair. Requires Kraken auth entitlements; L1/L2 remain preferred if unavailable.",
                                    );
                                }
                                if ui.button("L3 Demo (entitled sim)").clicked() && !dom_sym.is_empty() {
                                    self.kraken_l3_status = format!("L3 demo active for {} (simulated per-order depth — assume entitlements)", dom_sym);
                                    let now_s = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;
                                    // Sample L3-style data to exercise parser, Bookmap L3 viz, selected-order header details, and age coloring.
                                    self.orderbook_result = serde_json::json!({
                                        "symbol": dom_sym,
                                        "timestamp": "sim-l3",
                                        "checksum": 123456,
                                        "is_l3": true,
                                        "bids": [
                                            {"order_id":"O1","limit_price":123.45,"order_qty":1.2,"timestamp":format!("{now_s:.3}")},
                                            {"order_id":"O2","limit_price":123.40,"order_qty":0.8,"timestamp":format!("{:.3}", now_s - 12.0)}
                                        ],
                                        "asks": [
                                            {"order_id":"A1","limit_price":123.55,"order_qty":2.5,"timestamp":format!("{:.3}", now_s - 45.0)}
                                        ]
                                    }).to_string();
                                    // window already open via outer logic / user can open; avoid double-borrow on the open flag inside closure
                                }
                            });
                            // Deeper L3 UI: status label (foundation active)
                            if !self.kraken_l3_status.is_empty() {
                                ui.label(egui::RichText::new(&self.kraken_l3_status).small().color(ob_dim));
                            }
                            if self.orderbook_result.is_empty() {
                                ui.label(egui::RichText::new("No L2 data — click Fetch Depth in Bookmap or Fetch L2 in Order Flow.").color(ob_dim).small());
                            } else if let Ok(v) = serde_json::from_str::<serde_json::Value>(&self.orderbook_result) {
                                let sym = v["symbol"].as_str().unwrap_or("?");
                                let ts  = v["timestamp"].as_str().unwrap_or("");
                                let checksum_status = v["checksum_status"].as_str().unwrap_or("");
                                let checksum = v["checksum"].as_u64();
                                let header = if checksum_status.is_empty() {
                                    format!("{} — {}", sym, ts)
                                } else if let Some(checksum) = checksum {
                                    format!("{} — {} · checksum {} ({})", sym, ts, checksum, checksum_status)
                                } else {
                                    format!("{} — {} · checksum {}", sym, ts, checksum_status)
                                };
                                ui.label(egui::RichText::new(header).strong().small());

                                let bids: Vec<_> = v["bids"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).to_vec();
                                let asks: Vec<_> = v["asks"].as_array().map(|a| a.as_slice()).unwrap_or(&[]).to_vec();
                                let level_price = |level: &serde_json::Value| -> f64 {
                                    level["limit_price"]
                                        .as_f64()
                                        .or_else(|| level["price"].as_f64())
                                        .unwrap_or(0.0)
                                };
                                let level_size = |level: &serde_json::Value| -> f64 {
                                    level["order_qty"]
                                        .as_f64()
                                        .or_else(|| level["size"].as_f64())
                                        .unwrap_or(0.0)
                                };

                                // Polish 3/5: level counts + freshness note (after parse)
                                let bid_cnt = bids.len();
                                let ask_cnt = asks.len();
                                let age_note = if ts.is_empty() { " (live)".to_string() } else { format!(" age:{}", ts) };
                                let provider_note = if sym.contains("/") || sym.to_uppercase().ends_with("USD") { "Kraken" } else { "Alpaca snapshot" };
                                let l3_note = if v.get("is_l3").and_then(|b| b.as_bool()).unwrap_or(false) || bids.iter().any(|b| b.get("order_id").is_some()) {
                                    " · L3 per-order"
                                } else { "" };
                                ui.label(egui::RichText::new(format!("Levels: B{} A{} · {} {}{}", bid_cnt, ask_cnt, provider_note, age_note, l3_note)).small().color(ob_dim));

                                // Compute rich L2 metrics
                                let bid_vol: f64 = bids.iter().map(|e| level_size(e)).sum();
                                let ask_vol: f64 = asks.iter().map(|e| level_size(e)).sum();
                                let total_vol = (bid_vol + ask_vol).max(1e-9);
                                let imbalance = (bid_vol - ask_vol) / total_vol;
                                let imb_color = if imbalance > 0.05 { ob_bid } else if imbalance < -0.05 { ob_ask } else { ob_dim };
                                ui.label(egui::RichText::new(format!(
                                    "Bid vol: {:.2}  Ask vol: {:.2}  Imbalance: {:.1}% ",
                                    bid_vol, ask_vol, imbalance * 100.0
                                )).small().color(imb_color));

                                // Rich L2 polish: spread and mid
                                if let (Some(top_ask), Some(top_bid)) = (
                                    asks.first().map(|a| level_price(a)),
                                    bids.first().map(|b| level_price(b)),
                                ) {
                                    if top_ask > 0.0 && top_bid > 0.0 {
                                        let spread = top_ask - top_bid;
                                        let mid = (top_ask + top_bid) / 2.0;
                                        ui.label(egui::RichText::new(format!(
                                            "Spread: {:.4}  Mid: {:.4}",
                                            spread, mid
                                        )).small().color(ob_dim));
                                        }

                                        // Top L1 sizes from the same snapshot (richer view)
                                        if let (Some(top_bid), Some(top_ask)) = (
                                        bids.first().map(|b| level_size(b)),
                                        asks.first().map(|a| level_size(a)),
                                        ) {
                                        if top_bid > 0.0 || top_ask > 0.0 {
                                            ui.label(egui::RichText::new(format!(
                                                "Top sizes — Bid: {:.4}  Ask: {:.4}",
                                                top_bid, top_ask
                                            )).small().color(ob_dim));
                                        }
                                        }
                                        }

                                        // max size for bar scaling (use per level or global)
                                let max_sz = bids.iter().chain(asks.iter())
                                    .map(|e| level_size(e))
                                    .fold(0.0_f64, f64::max).max(1.0);
                                let avail_w = ui.available_width().min(320.0);

                                // cumulative for richer view
                                let mut cum_bid = 0.0f64;
                                let mut cum_ask = 0.0f64;

                                egui::ScrollArea::vertical().auto_shrink(false).max_height(340.0).show(ui, |ui| {
                                    ui.label(egui::RichText::new("Asks (sell side) — richer L2").color(ob_ask).small().strong());
                                    for ask in asks.iter().rev().take(25) {
                                        let price = level_price(ask);
                                        let size  = level_size(ask);
                                        cum_ask += size;
                                        let frac  = (size / max_sz) as f32;
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(format_price(price)).monospace().small().color(ob_ask));
                                            let (rect, _) = ui.allocate_exact_size(egui::vec2(avail_w - 140.0, 10.0), egui::Sense::hover());
                                            ui.painter_at(rect).rect_filled(
                                                egui::Rect::from_min_size(rect.min, egui::vec2(frac * rect.width(), 10.0)),
                                                0.0, egui::Color32::from_rgba_premultiplied(200, 40, 40, 120));
                                            ui.label(egui::RichText::new(format!("{:.4} c{:.2}", size, cum_ask)).monospace().small().color(ob_dim));
                                        });
                                    }
                                    ui.separator();
                                    ui.label(egui::RichText::new("Bids (buy side) — richer L2").color(ob_bid).small().strong());
                                    for bid in bids.iter().take(25) {
                                        let price = level_price(bid);
                                        let size  = level_size(bid);
                                        cum_bid += size;
                                        let frac  = (size / max_sz) as f32;
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(format_price(price)).monospace().small().color(ob_bid));
                                            let (rect, _) = ui.allocate_exact_size(egui::vec2(avail_w - 140.0, 10.0), egui::Sense::hover());
                                            ui.painter_at(rect).rect_filled(
                                                egui::Rect::from_min_size(rect.min, egui::vec2(frac * rect.width(), 10.0)),
                                                0.0, egui::Color32::from_rgba_premultiplied(0, 180, 60, 120));
                                            ui.label(egui::RichText::new(format!("{:.4} c{:.2}", size, cum_bid)).monospace().small().color(ob_dim));
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
                                0 => typhoon_transpiler::compile_mql5(&self.compiler_source),
                                1 => typhoon_transpiler::compile_mql4(&self.compiler_source),
                                2 => typhoon_transpiler::compile_pine(&self.compiler_source),
                                3 => typhoon_transpiler::compile_easylang(&self.compiler_source),
                                4 => typhoon_transpiler::compile_thinkscript(&self.compiler_source),
                                5 => typhoon_transpiler::compile_afl(&self.compiler_source),
                                6 => typhoon_transpiler::compile_probuilder(&self.compiler_source),
                                7 => typhoon_transpiler::compile_ninjascript(&self.compiler_source),
                                8 => typhoon_transpiler::compile_calgo(&self.compiler_source),
                                9 => typhoon_transpiler::compile_acsil(&self.compiler_source),
                                _ => typhoon_transpiler::compile_mql5(&self.compiler_source),
                            };
                            self.compiler_diagnostics.clear();
                            for d in &result.diagnostics {
                                self.compiler_diagnostics.push_back(format!(
                                    "{}:{}: {} — {}",
                                    d.line,
                                    d.col,
                                    match d.level {
                                        typhoon_transpiler::DiagLevel::Error => "ERROR",
                                        typhoon_transpiler::DiagLevel::Warning => "WARN",
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

                    // ── Cross-language transpile row ────────
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Transpile to:").small());
                        const TRANSPILE_TARGETS: &[(
                            &str,
                            typhoon_transpiler::transpile::TargetLanguage,
                        )] = &[
                            ("MQL5", typhoon_transpiler::transpile::TargetLanguage::Mql5),
                            ("MQL4", typhoon_transpiler::transpile::TargetLanguage::Mql4),
                            (
                                "PineScript v5",
                                typhoon_transpiler::transpile::TargetLanguage::PineScript,
                            ),
                            (
                                "EasyLanguage",
                                typhoon_transpiler::transpile::TargetLanguage::EasyLanguage,
                            ),
                            (
                                "thinkScript",
                                typhoon_transpiler::transpile::TargetLanguage::ThinkScript,
                            ),
                            (
                                "AFL (AmiBroker)",
                                typhoon_transpiler::transpile::TargetLanguage::Afl,
                            ),
                            (
                                "ProBuilder",
                                typhoon_transpiler::transpile::TargetLanguage::ProBuilder,
                            ),
                            (
                                "NinjaScript",
                                typhoon_transpiler::transpile::TargetLanguage::NinjaScript,
                            ),
                            (
                                "cAlgo (cTrader)",
                                typhoon_transpiler::transpile::TargetLanguage::Calgo,
                            ),
                            (
                                "ACSIL (Sierra Chart)",
                                typhoon_transpiler::transpile::TargetLanguage::Acsil,
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
                            use typhoon_transpiler::transpile::{SourceLanguage, transpile};
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
                                .unwrap_or(typhoon_transpiler::transpile::TargetLanguage::Mql5);
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
