//! Broker command runtime crate shell.
//!
//! ADR-125 Target 3 is moving the broker command processor out of the native UI
//! crate. This crate intentionally starts as a narrow shell: lower-layer broker,
//! cache, and chart-key dependencies live here before the native processor tree is
//! physically moved behind a single spawn seam.

pub mod ai_chat;
pub mod alpaca_account_data;
pub mod alpaca_order_ops;
pub mod alpaca_ws_commands;
pub mod bar_fetch_commands;
pub mod broker_processor;
pub mod connection_commands;
pub mod external_feeds;
pub mod fundamentals_commands;
pub mod kraken_market_commands;
pub mod kraken_ohlc_pipeline;
pub mod kraken_order_ops;
pub mod kraken_ws_commands;
pub mod market_data_commands;
pub mod matrix_commands;
pub mod misc_commands;
pub mod news;
pub mod news_ingest;
pub mod prelude;
pub mod research_compute;
pub mod research_fetch;
pub mod resources;
pub mod storage;
pub mod symbol_search;
pub mod watchlist_quotes;
