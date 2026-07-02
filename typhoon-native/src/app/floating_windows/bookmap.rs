use super::*;

pub(super) fn bookmap_symbol_key(symbol: &str) -> String {
    typhoon_engine::core::kraken::normalize_pair_symbol(&normalize_market_data_symbol(symbol))
        .replace('/', "")
}

pub(super) fn orderbook_json_matches_symbol(orderbook_json: &str, symbol: &str) -> bool {
    if orderbook_json.trim().is_empty() || symbol.trim().is_empty() {
        return false;
    }
    serde_json::from_str::<serde_json::Value>(orderbook_json)
        .ok()
        .and_then(|v| v.get("symbol").and_then(|s| s.as_str()).map(str::to_string))
        .map(|book_symbol| {
            bookmap_symbol_key(&book_symbol).eq_ignore_ascii_case(&bookmap_symbol_key(symbol))
        })
        .unwrap_or(false)
}

pub(super) fn kraken_bookmap_stream_supported(
    symbol: &str,
    kraken_pairs: &[(String, String)],
) -> bool {
    let trimmed = symbol.trim();
    if trimmed.is_empty() || trimmed.contains(".EQ") {
        return false;
    }
    let normalized = bookmap_symbol_key(trimmed);
    if kraken_pairs.iter().any(|(pair, display)| {
        bookmap_symbol_key(pair).eq_ignore_ascii_case(&normalized)
            || bookmap_symbol_key(display).eq_ignore_ascii_case(&normalized)
    }) {
        return true;
    }
    kraken_pairs.is_empty() && typhoon_engine::core::kraken::to_kraken_pair_lossy(trimmed).is_some()
}

pub(super) fn render_live_orderbook_heatmap(
    ui: &mut egui::Ui,
    orderbook_json: &str,
    selected_order_id: &mut Option<String>,
    bid_color: egui::Color32,
    ask_color: egui::Color32,
    dim_color: egui::Color32,
) -> bool {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(orderbook_json) else {
        return false;
    };
    let bids = v["bids"].as_array().map(|a| a.as_slice()).unwrap_or(&[]);
    let asks = v["asks"].as_array().map(|a| a.as_slice()).unwrap_or(&[]);
    if bids.is_empty() && asks.is_empty() {
        return false;
    }

    // Support both L2 ("price"/"size") and L3 ("limit_price"/"order_qty", "order_id")
    let get_price = |level: &serde_json::Value| -> f64 {
        level["limit_price"]
            .as_f64()
            .or_else(|| level["price"].as_f64())
            .unwrap_or(0.0)
    };
    let get_size = |level: &serde_json::Value| -> f64 {
        level["order_qty"]
            .as_f64()
            .or_else(|| level["size"].as_f64())
            .unwrap_or(0.0)
    };

    // Order-age for L3 coloring/interactions: parse ts -> age secs (smaller = newer)
    let get_age_secs = |level: &serde_json::Value| -> f64 {
        level
            .get("timestamp")
            .and_then(|t| t.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .map(|ts| {
                (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs_f64()
                    - ts)
                    .max(0.0)
            })
            .unwrap_or(8.0)
    };

    let is_l3 = v.get("is_l3").and_then(|b| b.as_bool()).unwrap_or(false)
        || bids.iter().any(|b| b.get("order_id").is_some())
        || asks.iter().any(|a| a.get("order_id").is_some());
    let order_count_bid: usize = if is_l3 { bids.len() } else { 0 };
    let order_count_ask: usize = if is_l3 { asks.len() } else { 0 };

    let max_size = bids
        .iter()
        .chain(asks.iter())
        .map(get_size)
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let ts = v["timestamp"].as_str().unwrap_or("live");
    // Follow-up polish: staleness + top rich L1 sizes
    let top_bid_size = bids.first().and_then(|b| b["size"].as_f64()).unwrap_or(0.0);
    let top_ask_size = asks.first().and_then(|a| a["size"].as_f64()).unwrap_or(0.0);
    let header = if is_l3 {
        format!(
            "Live L3 per-order — {} ({} orders bid, {} ask)",
            ts, order_count_bid, order_count_ask
        )
    } else if top_bid_size > 0.0 || top_ask_size > 0.0 {
        format!(
            "Live L2 depth — {}  (top b{:.2} a{:.2})",
            ts, top_bid_size, top_ask_size
        )
    } else {
        format!("Live L2 depth — {}", ts)
    };
    ui.label(egui::RichText::new(header).color(dim_color).small());

    let width = ui.available_width().max(240.0).min(620.0);
    let height = 170.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(5, 5, 15));

    let mid_y = rect.center().y;
    painter.line_segment(
        [
            egui::pos2(rect.left(), mid_y),
            egui::pos2(rect.right(), mid_y),
        ],
        egui::Stroke::new(1.0, dim_color),
    );

    let max_levels = 24_usize;
    let row_h = (height * 0.5 / max_levels as f32).max(3.0);
    for (idx, ask) in asks.iter().take(max_levels).enumerate() {
        let size = get_size(ask);
        let price = get_price(ask);
        let frac = (size / max_size).clamp(0.0, 1.0) as f32;
        let alpha = (80.0 + 175.0 * frac as f64).clamp(60.0, 220.0) as u8; // better density scaling
        let y = mid_y - (idx as f32 + 1.0) * row_h;
        let bar = egui::Rect::from_min_size(
            egui::pos2(rect.right() - width * frac, y),
            egui::vec2(width * frac, row_h - 1.0),
        );
        let age = get_age_secs(ask);
        let age_f = (1.0 / (1.0 + age / 20.0)) as f32; // newer brighter
        let a2 = (alpha as f32 * (0.65 + 0.35 * age_f)).clamp(50.0, 255.0) as u8;
        painter.rect_filled(
            bar,
            0.0,
            egui::Color32::from_rgba_premultiplied(200, 40, 40, a2),
        );
        if idx < 6 {
            painter.text(
                egui::pos2(rect.left() + 4.0, y),
                egui::Align2::LEFT_TOP,
                format_price(price),
                egui::FontId::monospace(9.0),
                ask_color,
            );
        }
    }
    for (idx, bid) in bids.iter().take(max_levels).enumerate() {
        let size = get_size(bid);
        let price = get_price(bid);
        let frac = (size / max_size).clamp(0.0, 1.0) as f32;
        let alpha = (80.0 + 175.0 * frac as f64).clamp(60.0, 220.0) as u8;
        let y = mid_y + idx as f32 * row_h;
        let bar = egui::Rect::from_min_size(
            egui::pos2(rect.left(), y),
            egui::vec2(width * frac, row_h - 1.0),
        );
        let age = get_age_secs(bid);
        let age_f = (1.0 / (1.0 + age / 20.0)) as f32;
        let a2 = (alpha as f32 * (0.65 + 0.35 * age_f)).clamp(50.0, 255.0) as u8;
        painter.rect_filled(
            bar,
            0.0,
            egui::Color32::from_rgba_premultiplied(0, 180, 60, a2),
        );
        if idx < 6 {
            painter.text(
                egui::pos2(rect.right() - 4.0, y),
                egui::Align2::RIGHT_TOP,
                format_price(price),
                egui::FontId::monospace(9.0),
                bid_color,
            );
        }
        // Per-order individual markers for L3 (small vertical ticks/dots)
        if is_l3 {
            let order_id = bid.get("order_id").and_then(|o| o.as_str()).unwrap_or("");
            let mx = rect.left() + (width * frac * 0.5);
            let marker = egui::pos2(mx, y + row_h * 0.3);
            painter.circle_filled(marker, 1.5, egui::Color32::from_rgb(180, 255, 180));
            if idx < 4 && !order_id.is_empty() {
                painter.text(
                    egui::pos2(mx + 3.0, y),
                    egui::Align2::LEFT_TOP,
                    &order_id[..order_id.len().min(6)],
                    egui::FontId::monospace(6.0),
                    egui::Color32::from_rgb(150, 220, 150),
                );
            }
        }
    }

    // Hover tooltip with top levels + sizes (rich L2/L3)
    if resp.hovered() {
        let mut tip = if is_l3 {
            format!(
                "L3 per-order snapshot {}\nOrders bid: {} ask: {}\n",
                ts, order_count_bid, order_count_ask
            )
        } else {
            format!("L2 snapshot {}\n", ts)
        };
        if let Some(b) = bids.first() {
            let p = get_price(b);
            let s = get_size(b);
            let oid = b.get("order_id").and_then(|o| o.as_str()).unwrap_or("");
            tip.push_str(&format!(
                "Top Bid: {} x {} {}
",
                format_price(p),
                s,
                oid
            ));
        }
        if let Some(a) = asks.first() {
            let p = get_price(a);
            let s = get_size(a);
            let oid = a.get("order_id").and_then(|o| o.as_str()).unwrap_or("");
            tip.push_str(&format!(
                "Top Ask: {} x {} {}
",
                format_price(p),
                s,
                oid
            ));
        }
        tip.push_str(if is_l3 {
            "Per-order markers shown"
        } else {
            "Hover for live depth density"
        });
        resp.on_hover_text(tip);
    }

    // Richer Bookmap L3: order list pane with age coloring + interactions (click = copy + select note)
    if is_l3 {
        ui.separator();
        ui.label(egui::RichText::new("L3 orders (age-colored, top 5/side)").small());
        egui::ScrollArea::vertical()
            .max_height(100.0)
            .show(ui, |ui| {
                for (side, col, levs) in [("BID", bid_color, bids), ("ASK", ask_color, asks)] {
                    for lev in levs.iter().take(5) {
                        let oid = lev.get("order_id").and_then(|o| o.as_str()).unwrap_or("?");
                        let p = get_price(lev);
                        let q = get_size(lev);
                        let age = get_age_secs(lev);
                        let age_txt = if age < 5.0 {
                            "new"
                        } else if age < 30.0 {
                            "mid"
                        } else {
                            "old"
                        };
                        let txt = format!(
                            "{} {} @ {:.4} x {:.4} ({})",
                            side,
                            &oid[..oid.len().min(8)],
                            p,
                            q,
                            age_txt
                        );
                        let is_selected = selected_order_id.as_deref() == Some(oid);
                        let label = if is_selected {
                            egui::RichText::new(txt)
                                .strong()
                                .color(egui::Color32::YELLOW)
                        } else {
                            egui::RichText::new(txt)
                        };
                        let resp = ui.colored_label(col, label);
                        if resp.clicked() {
                            *selected_order_id = Some(oid.to_string());
                            ui.ctx().copy_text(oid.to_string());
                            ui.label(
                                egui::RichText::new(format!(
                                    "selected {} for chart (L3 order)",
                                    &oid[..oid.len().min(6)]
                                ))
                                .small()
                                .color(egui::Color32::YELLOW),
                            );
                        }
                        if ui.small_button("copy").clicked() {
                            ui.ctx().copy_text(oid.to_string());
                        }
                    }
                }
            });
    }

    true
}

impl TyphooNApp {
    pub(in crate::app) fn open_bookmap_window(&mut self, symbol: Option<String>) {
        let resolved = symbol
            .map(|s| normalize_market_data_symbol(&s))
            .filter(|s| !s.is_empty())
            .or_else(|| self.active_trade_symbol())
            .unwrap_or_else(|| "UNKNOWN".to_string());

        if self
            .bookmap_windows
            .iter_mut()
            .any(|w| w.symbol.eq_ignore_ascii_case(&resolved))
        {
            self.log
                .push_back(LogEntry::info(format!("Bookmap already open: {resolved}")));
            return;
        }

        self.bookmap_windows.push(BookmapWindowState {
            symbol: resolved.clone(),
            open: true,
            selected_order_id: None,
        });
        self.log
            .push_back(LogEntry::info(format!("Bookmap opened: {resolved}")));
    }
}

#[cfg(test)]
mod tests {
    use super::{kraken_bookmap_stream_supported, orderbook_json_matches_symbol};

    #[test]
    fn orderbook_json_symbol_matching_normalizes_pairs() {
        let book = serde_json::json!({
            "symbol": "BTC/USD",
            "timestamp": "2026-05-26T00:00:00Z",
            "bids": [{ "price": 100.0, "size": 1.0 }],
            "asks": [{ "price": 101.0, "size": 2.0 }]
        })
        .to_string();

        assert!(orderbook_json_matches_symbol(&book, "BTCUSD"));
        assert!(orderbook_json_matches_symbol(&book, "btc/usd"));
        assert!(!orderbook_json_matches_symbol(&book, "ETHUSD"));
        assert!(!orderbook_json_matches_symbol("not-json", "BTCUSD"));
    }

    #[test]
    fn kraken_bookmap_stream_support_stays_spot_pair_scoped() {
        let known_pairs = vec![("XBTUSD".to_string(), "BTC/USD".to_string())];
        assert!(kraken_bookmap_stream_supported("BTC/USD", &known_pairs));
        assert!(kraken_bookmap_stream_supported("BTCUSD", &known_pairs));
        assert!(!kraken_bookmap_stream_supported("TNDM", &known_pairs));
        assert!(!kraken_bookmap_stream_supported("GDC", &known_pairs));
        assert!(!kraken_bookmap_stream_supported("GDC.EQ", &known_pairs));

        assert!(kraken_bookmap_stream_supported("ETHUSD", &[]));
    }
}
