//! WebSocket-driven bar construction: builds 1-minute OHLCV bars from trade stream.
//!
//! Trades arrive via WebSocket (StreamTrade). BarBuilder accumulates them into
//! 1-minute candles. When a new minute starts, the previous candle is "completed"
//! and available for the frontend to consume.
//!
//! This eliminates the 10-second polling loop for live bar updates — the frontend
//! polls completed bars from BarBuilder instead of hitting the API.

use std::collections::HashMap;

/// A completed 1-minute bar.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CompletedBar {
    pub symbol: String,
    pub timestamp: String, // RFC3339, floored to minute
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub trade_count: u32,
}

/// Partial bar being constructed from live trades.
#[derive(Debug, Clone)]
struct PartialBar {
    minute_epoch: i64, // epoch seconds, floored to 60
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
    trade_count: u32,
}

/// Builds 1-minute OHLCV bars from incoming trade stream.
/// Thread-safe: wrap in Mutex for shared state access.
pub struct BarBuilder {
    /// Active (still forming) bars, one per symbol
    active: HashMap<String, PartialBar>,
    /// Completed bars ready for frontend consumption
    completed: Vec<CompletedBar>,
}

impl BarBuilder {
    pub fn new() -> Self {
        Self {
            active: HashMap::new(),
            completed: Vec::new(),
        }
    }

    /// Ingest a trade. If the trade's minute differs from the active bar's minute,
    /// the active bar is completed and a new one starts.
    pub fn ingest_trade(&mut self, symbol: &str, price: f64, size: f64, timestamp_rfc3339: &str) {
        if price <= 0.0 { return; }

        // Parse timestamp to epoch seconds, floor to minute boundary
        let trade_epoch = chrono::DateTime::parse_from_rfc3339(timestamp_rfc3339)
            .map(|dt| dt.timestamp())
            .unwrap_or(0);
        let trade_minute = trade_epoch - (trade_epoch % 60);

        if let Some(bar) = self.active.get_mut(symbol) {
            if bar.minute_epoch == trade_minute {
                // Same minute — update HLCV
                bar.high = bar.high.max(price);
                bar.low = bar.low.min(price);
                bar.close = price;
                bar.volume += size;
                bar.trade_count += 1;
            } else {
                // New minute — complete old bar, start new one
                let dt = chrono::DateTime::from_timestamp(bar.minute_epoch, 0).unwrap_or_default();
                self.completed.push(CompletedBar {
                    symbol: symbol.to_string(),
                    timestamp: dt.to_rfc3339(),
                    open: bar.open,
                    high: bar.high,
                    low: bar.low,
                    close: bar.close,
                    volume: bar.volume,
                    trade_count: bar.trade_count,
                });
                // Start new bar
                *bar = PartialBar {
                    minute_epoch: trade_minute,
                    open: price,
                    high: price,
                    low: price,
                    close: price,
                    volume: size,
                    trade_count: 1,
                };
            }
        } else {
            // First trade for this symbol
            self.active.insert(symbol.to_string(), PartialBar {
                minute_epoch: trade_minute,
                open: price,
                high: price,
                low: price,
                close: price,
                volume: size,
                trade_count: 1,
            });
        }
    }

    /// Drain all completed bars. Returns them and clears the internal buffer.
    pub fn drain_completed(&mut self) -> Vec<CompletedBar> {
        std::mem::take(&mut self.completed)
    }

    /// Get the current (still-forming) bar for a symbol, if any.
    /// Useful for real-time display of the live candle.
    pub fn get_active_bar(&self, symbol: &str) -> Option<CompletedBar> {
        self.active.get(symbol).map(|bar| {
            let dt = chrono::DateTime::from_timestamp(bar.minute_epoch, 0).unwrap_or_default();
            CompletedBar {
                symbol: symbol.to_string(),
                timestamp: dt.to_rfc3339(),
                open: bar.open,
                high: bar.high,
                low: bar.low,
                close: bar.close,
                volume: bar.volume,
                trade_count: bar.trade_count,
            }
        })
    }

    /// Get all active (forming) bars across all symbols.
    pub fn get_all_active_bars(&self) -> Vec<CompletedBar> {
        self.active.keys()
            .filter_map(|sym| self.get_active_bar(sym))
            .collect()
    }
}
