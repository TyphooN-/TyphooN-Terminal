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

    let max_size = bids
        .iter()
        .chain(asks.iter())
        .filter_map(|level| level["size"].as_f64())
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let ts = v["timestamp"].as_str().unwrap_or("live");
    ui.label(
        egui::RichText::new(format!("Live L2 depth — {ts}"))
            .color(dim_color)
            .small(),
    );
    let width = ui.available_width().max(240.0).min(620.0);
    let height = 170.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
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
        let size = ask["size"].as_f64().unwrap_or(0.0);
        let price = ask["price"].as_f64().unwrap_or(0.0);
        let frac = (size / max_size).clamp(0.0, 1.0) as f32;
        let y = mid_y - (idx as f32 + 1.0) * row_h;
        let bar = egui::Rect::from_min_size(
            egui::pos2(rect.right() - width * frac, y),
            egui::vec2(width * frac, row_h - 1.0),
        );
        painter.rect_filled(
            bar,
            0.0,
            egui::Color32::from_rgba_premultiplied(200, 40, 40, 125),
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
        let size = bid["size"].as_f64().unwrap_or(0.0);
        let price = bid["price"].as_f64().unwrap_or(0.0);
        let frac = (size / max_size).clamp(0.0, 1.0) as f32;
        let y = mid_y + idx as f32 * row_h;
        let bar = egui::Rect::from_min_size(
            egui::pos2(rect.left(), y),
            egui::vec2(width * frac, row_h - 1.0),
        );
        painter.rect_filled(
            bar,
            0.0,
            egui::Color32::from_rgba_premultiplied(0, 180, 60, 125),
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
