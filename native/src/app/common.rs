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
pub(crate) const BG: egui::Color32 = egui::Color32::from_rgb(0, 0, 0);
pub(crate) const GRID: egui::Color32 = egui::Color32::from_rgb(33, 33, 33); // #333 (WebKit dotted grid)
pub(crate) const UP: egui::Color32 = egui::Color32::from_rgb(0, 255, 0); // #00ff00 (MT5 bright green — solid fill)
pub(crate) const DOWN: egui::Color32 = egui::Color32::from_rgb(255, 0, 0); // #ff0000 (MT5 bright red — solid fill)
pub(crate) const SMA200_COL: egui::Color32 = egui::Color32::from_rgb(255, 255, 0); // #ffff00 yellow (MT5 match)
pub(crate) const SMA100_COL: egui::Color32 = egui::Color32::from_rgb(100, 180, 255); // #64b4ff blue
pub(crate) const KAMA_COL: egui::Color32 = egui::Color32::from_rgb(220, 220, 230); // soft white (MT5 KAMA)
pub(crate) const EMA_COL: egui::Color32 = egui::Color32::from_rgb(255, 130, 60);
pub(crate) const BB_COL: egui::Color32 = egui::Color32::from_rgb(80, 160, 200);
pub(crate) const BB_FILL: egui::Color32 = egui::Color32::from_rgba_premultiplied(80, 160, 200, 25);
pub(crate) const AXIS_TEXT: egui::Color32 = egui::Color32::from_rgb(140, 140, 160); // #8c8ca0
pub(crate) const ACCENT: egui::Color32 = egui::Color32::from_rgb(76, 175, 80);
pub(crate) const FISHER_POS: egui::Color32 = egui::Color32::from_rgb(0, 255, 0); // #00ff00 (MT5 bright green)
pub(crate) const FISHER_NEG: egui::Color32 = egui::Color32::from_rgb(255, 0, 0); // #ff0000 (MT5 bright red)
pub(crate) const FISHER_SIG: egui::Color32 = egui::Color32::from_rgb(169, 169, 169); // clrDarkGray (MT5 signal)
pub(crate) const RSI_LINE: egui::Color32 = egui::Color32::from_rgb(200, 180, 60); // #c8b43c (mustard yellow)
pub(crate) const MACD_LINE_COL: egui::Color32 = egui::Color32::from_rgb(100, 180, 255); // #64b4ff
pub(crate) const MACD_SIG_COL: egui::Color32 = egui::Color32::from_rgb(255, 130, 48); // #ff8230 (orange)

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
