//! TyphooN Terminal Engine — shared library crate.
//!
//! This module re-exports core trading engine functionality for use by:
//! - `main.rs` (Tauri WebKit frontend — legacy)
//! - `native/` (egui + wgpu native GPU frontend)
//! - Future TUI/CLI interfaces
//!
//! All broker, cache, risk, and analytics modules are available without
//! Tauri IPC overhead when used directly from Rust.

pub mod broker;
pub mod core;
pub mod notifications;
pub mod strategies;
