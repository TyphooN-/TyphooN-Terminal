//! Chart-UI view + chart-local state layer, extracted from `typhoon-native`
//! (ADR-125 Target 2 — second crate). Owns the chart-local data types (`types`:
//! `Bar` / `ChartType` / `Timeframe`) and, as later slices land, the chart state
//! (`ChartState` / `ChartCamera` / `IndicatorFlags`), indicator math, and the egui
//! rendering layer (price bars, overlays, axes, drawing tools).
//!
//! `typhoon-native` remains the binary/app shell: it owns the chart command/ops
//! dispatchers (`chart_ops`), the broker-coupled equity-merge pipeline, and the
//! `TyphooNApp` state graph, and calls into this crate. This crate must never depend
//! on `typhoon-native`.

pub mod auto_fibonacci;
pub mod camera_controls;
pub mod cache_keys;
pub mod drawing;
pub mod indicators;
pub mod models;
pub mod render;
pub mod state;
pub mod types;
