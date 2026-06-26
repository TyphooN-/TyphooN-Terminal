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
