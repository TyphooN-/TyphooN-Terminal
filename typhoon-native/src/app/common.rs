//! Extracted from app.rs: common helpers.

use super::*;

// === Navbar Typography Helpers (US Graphics level) ===
#[allow(dead_code)]
pub fn nav_primary(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(egui::RichText::new(text).strong().size(13.0));
}

#[allow(dead_code)]
pub fn nav_secondary(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(
        egui::RichText::new(text)
            .size(11.5)
            .color(egui::Color32::from_rgb(170, 170, 170)),
    );
}

pub fn nav_muted(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(egui::RichText::new(text).size(10.5).color(AXIS_TEXT));
}

// ─── colours ────────────────────────────────────────────────────────────────
// Base chart palette now lives in typhoon-chart-ui (ADR-125 Target 2, slice 7); re-exported
// so the ~66 native files using UP/DOWN/AXIS_TEXT/… and the app glob are unchanged.
pub(crate) use typhoon_chart_ui::models::{
    ACCENT, AXIS_TEXT, BB_COL, BG, DOWN, EMA_COL, FISHER_NEG, FISHER_POS, KAMA_COL, MACD_LINE_COL,
    RSI_LINE, SMA100_COL, SMA200_COL, UP,
};

// ─── right panel button colours (exact WebKit CSS values) ────────────────────
pub(crate) const BTN_GREEN: egui::Color32 = egui::Color32::from_rgb(10, 95, 56); // .btn-action: #0a5f38
pub(crate) const BTN_GREEN_TEXT: egui::Color32 = egui::Color32::from_rgb(136, 255, 136); // #8f8
pub(crate) const BTN_MG: egui::Color32 = egui::Color32::from_rgb(58, 58, 0); // .btn-mg: #3a3a00
pub(crate) const BTN_BLUE: egui::Color32 = egui::Color32::from_rgb(15, 52, 96); // .btn-lines: #0f3460
pub(crate) const BTN_BLUE_TEXT: egui::Color32 = egui::Color32::from_rgb(136, 204, 255); // #8cf
pub(crate) const BTN_RED: egui::Color32 = egui::Color32::from_rgb(90, 26, 26); // .btn-danger: #5a1a1a
pub(crate) const BTN_RED_TEXT: egui::Color32 = egui::Color32::from_rgb(255, 136, 136); // #f88
pub(crate) const BG_BUTTON: egui::Color32 = egui::Color32::from_rgb(26, 26, 46); // --bg-button: #1a1a2e
pub(crate) const QUAKE_CMD: egui::Color32 = egui::Color32::from_rgb(0, 220, 220); // used in status bar
// Watchlist symbol colours (rotating palette)
pub(crate) const WL_COLORS: [egui::Color32; 8] = [
    egui::Color32::from_rgb(0, 220, 80),    // green
    egui::Color32::from_rgb(255, 200, 50),  // yellow
    egui::Color32::from_rgb(180, 100, 255), // purple
    egui::Color32::from_rgb(220, 40, 40),   // red
    egui::Color32::from_rgb(255, 255, 255), // white
    egui::Color32::from_rgb(0, 180, 255),   // cyan
    egui::Color32::from_rgb(255, 130, 60),  // orange
    egui::Color32::from_rgb(200, 80, 200),  // pink
];

/// Broker-agnostic entry point: can `broker` deliver a **live streaming** L2
/// depth book for `symbol`? Gated first on the broker's declared L2 capability
/// (ADR-129 capability model: `OrderBroker::l2_support`), then on the
/// broker-specific symbol scope. Adding a broker to `OrderBroker` forces a new
/// arm here, so depth gating can never silently keep a stale single-broker
/// assumption. `kraken_pairs_normalized` is the loaded Kraken pair catalog
/// membership set consumed by the Kraken arm; it is ignored by brokers whose
/// depth scope is not Kraken's.
pub(crate) fn depth_stream_supported(
    broker: OrderBroker,
    symbol: &str,
    kraken_pairs_normalized: &std::collections::HashSet<String>,
    kraken_pairs_empty: bool,
) -> bool {
    // Only brokers that declare a *streaming* L2 book can start a live depth
    // stream. Alpaca (crypto REST snapshots) and any future snapshot-only
    // broker short-circuit to `false` here via the shared capability model.
    if !broker.l2_support().is_live() {
        return false;
    }
    match broker {
        OrderBroker::Kraken => {
            kraken_pair_streamable(symbol, kraken_pairs_normalized, kraken_pairs_empty)
        }
        // Unreachable today (Alpaca L2 is Snapshot, filtered above); listed so a
        // future streaming Alpaca depth feed must opt in explicitly rather than
        // inherit Kraken's pair logic.
        OrderBroker::Alpaca => false,
    }
}

/// Kraken-specific predicate: is `symbol` a Kraken spot/xStock pair whose v2
/// `book` we can stream? The Kraken arm of [`depth_stream_supported`].
fn kraken_pair_streamable(
    symbol: &str,
    kraken_pairs_normalized: &std::collections::HashSet<String>,
    kraken_pairs_empty: bool,
) -> bool {
    let trimmed = symbol.trim();
    if trimmed.is_empty() || trimmed.contains(".EQ") {
        return false;
    }

    let symbol_key =
        typhoon_engine::core::kraken::normalize_pair_symbol(&normalize_market_data_symbol(trimmed))
            .replace('/', "")
            .to_ascii_uppercase();

    kraken_pairs_normalized.contains(&symbol_key)
        || (kraken_pairs_empty
            && typhoon_engine::core::kraken::to_kraken_pair_lossy(trimmed).is_some())
}

/// Kraken depth-stream gate used across DOM/Bookmap/toolbar call sites. Thin
/// alias for the Kraken arm of the broker-agnostic [`depth_stream_supported`];
/// kept as the name the depth/Bookmap surfaces already call.
pub(crate) fn kraken_depth_stream_supported(
    symbol: &str,
    kraken_pairs_normalized: &std::collections::HashSet<String>,
    kraken_pairs_empty: bool,
) -> bool {
    depth_stream_supported(
        OrderBroker::Kraken,
        symbol,
        kraken_pairs_normalized,
        kraken_pairs_empty,
    )
}
