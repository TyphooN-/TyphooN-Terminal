//! Research-UI view + interaction layer, extracted from `typhoon-native` (ADR-125
//! Phase 2 — first crate). Owns the per-window snapshot-display renderers
//! (`render`), the compute-window interaction shell (`window_shell`), and the
//! symbol-investigation packet text formatters (`format`) — all free functions over
//! `egui` + `typhoon_engine` DTOs, with no `TyphooNApp` dependency. (`format` is
//! pure text and needs no egui.)
//!
//! `typhoon-native` remains the binary/app shell: it owns the dispatchers, command
//! handlers, and window state, and calls into this crate. This crate must never
//! depend on `typhoon-native`.

pub mod format;
pub mod packet;
pub mod render;
pub mod theme;
pub mod window_shell;
