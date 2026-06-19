//! TyphooN Terminal Engine — shared library crate.
//!
//! Core trading engine: broker, cache, risk, backtest, screener.
//! Used by the native GPU renderer (egui + wgpu) in `typhoon-native/`.

pub mod broker;
pub mod core;
pub mod notifications;
pub mod strategies;
