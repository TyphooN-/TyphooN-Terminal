//! Color constants for the research UI.
//!
//! These mirror the corresponding `typhoon-native` `app::common` values; they are
//! duplicated here (six small `Color32` literals) so the crate has no dependency
//! back on `typhoon-native`. Per ADR-125 a shared theme crate is deferred until a
//! concrete cycle forces it; until then the duplication is the honest cost of the
//! boundary. Keep these in sync with `app::common`.

use egui::Color32;

pub const UP: Color32 = Color32::from_rgb(0, 255, 0); // #00ff00
pub const DOWN: Color32 = Color32::from_rgb(255, 0, 0); // #ff0000
pub const AXIS_TEXT: Color32 = Color32::from_rgb(140, 140, 160); // #8c8ca0
pub const BTN_GREEN_TEXT: Color32 = Color32::from_rgb(136, 255, 136); // #8f8
pub const BTN_MG: Color32 = Color32::from_rgb(58, 58, 0); // #3a3a00
pub const BTN_RED_TEXT: Color32 = Color32::from_rgb(255, 136, 136); // #f88
