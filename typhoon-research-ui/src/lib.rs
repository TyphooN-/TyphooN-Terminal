//! Research-UI view + interaction layer, extracted from `typhoon-native` (ADR-125
//! Phase 2 — first crate). Owns the per-window snapshot-display renderers
//! (`render`) and the compute-window interaction shell (`window_shell`), as free
//! functions over `egui` + `typhoon_engine` DTOs — no `TyphooNApp` dependency.
//!
//! `typhoon-native` remains the binary/app shell: it owns the dispatchers, command
//! handlers, and window state, and calls into this crate. This crate must never
//! depend on `typhoon-native`.

pub mod render;
pub mod theme;
pub mod window_shell;
