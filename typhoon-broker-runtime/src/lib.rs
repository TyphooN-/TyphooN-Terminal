//! Broker command runtime crate shell.
//!
//! ADR-125 Target 3 is moving the broker command processor out of the native UI
//! crate. This crate intentionally starts as a narrow shell: lower-layer broker,
//! cache, and chart-key dependencies live here before the native processor tree is
//! physically moved behind a single spawn seam.

pub mod alpaca_account_data;
pub mod external_feeds;
pub mod misc_commands;
pub mod prelude;
pub mod resources;
pub mod symbol_search;
