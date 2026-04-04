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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_trade_creates_active_bar() {
        let mut bb = BarBuilder::new();
        bb.ingest_trade("AAPL", 150.0, 100.0, "2026-01-15T10:30:00Z");

        let bar = bb.get_active_bar("AAPL").expect("should have active bar");
        assert_eq!(bar.symbol, "AAPL");
        assert_eq!(bar.open, 150.0);
        assert_eq!(bar.high, 150.0);
        assert_eq!(bar.low, 150.0);
        assert_eq!(bar.close, 150.0);
        assert_eq!(bar.volume, 100.0);
        assert_eq!(bar.trade_count, 1);
        assert!(bar.timestamp.contains("2026-01-15T10:30:00"));
    }

    #[test]
    fn multiple_trades_same_minute_update_ohlcv() {
        let mut bb = BarBuilder::new();
        bb.ingest_trade("AAPL", 150.0, 100.0, "2026-01-15T10:30:00Z");
        bb.ingest_trade("AAPL", 155.0, 200.0, "2026-01-15T10:30:15Z");
        bb.ingest_trade("AAPL", 148.0, 50.0, "2026-01-15T10:30:30Z");
        bb.ingest_trade("AAPL", 152.0, 75.0, "2026-01-15T10:30:45Z");

        let bar = bb.get_active_bar("AAPL").unwrap();
        assert_eq!(bar.open, 150.0);
        assert_eq!(bar.high, 155.0);
        assert_eq!(bar.low, 148.0);
        assert_eq!(bar.close, 152.0);
        assert_eq!(bar.volume, 425.0);
        assert_eq!(bar.trade_count, 4);
    }

    #[test]
    fn minute_boundary_completes_bar_and_starts_new() {
        let mut bb = BarBuilder::new();
        bb.ingest_trade("AAPL", 150.0, 100.0, "2026-01-15T10:30:00Z");
        bb.ingest_trade("AAPL", 155.0, 200.0, "2026-01-15T10:30:30Z");

        // Cross into next minute
        bb.ingest_trade("AAPL", 160.0, 50.0, "2026-01-15T10:31:00Z");

        // Old bar should be completed
        let completed = bb.drain_completed();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].open, 150.0);
        assert_eq!(completed[0].high, 155.0);
        assert_eq!(completed[0].low, 150.0);
        assert_eq!(completed[0].close, 155.0);
        assert_eq!(completed[0].volume, 300.0);
        assert_eq!(completed[0].trade_count, 2);
        assert!(completed[0].timestamp.contains("2026-01-15T10:30:00"));

        // New bar should be active
        let active = bb.get_active_bar("AAPL").unwrap();
        assert_eq!(active.open, 160.0);
        assert_eq!(active.close, 160.0);
        assert_eq!(active.volume, 50.0);
        assert_eq!(active.trade_count, 1);
        assert!(active.timestamp.contains("2026-01-15T10:31:00"));
    }

    #[test]
    fn zero_and_negative_price_ignored() {
        let mut bb = BarBuilder::new();
        bb.ingest_trade("AAPL", 0.0, 100.0, "2026-01-15T10:30:00Z");
        bb.ingest_trade("AAPL", -5.0, 100.0, "2026-01-15T10:30:10Z");
        assert!(bb.get_active_bar("AAPL").is_none());

        // Valid trade still works after ignored ones
        bb.ingest_trade("AAPL", 150.0, 100.0, "2026-01-15T10:30:20Z");
        assert!(bb.get_active_bar("AAPL").is_some());

        // Zero price within existing bar is ignored
        bb.ingest_trade("AAPL", 0.0, 500.0, "2026-01-15T10:30:30Z");
        let bar = bb.get_active_bar("AAPL").unwrap();
        assert_eq!(bar.trade_count, 1);
        assert_eq!(bar.volume, 100.0);
    }

    #[test]
    fn drain_completed_clears_buffer() {
        let mut bb = BarBuilder::new();
        bb.ingest_trade("AAPL", 150.0, 100.0, "2026-01-15T10:30:00Z");
        bb.ingest_trade("AAPL", 160.0, 50.0, "2026-01-15T10:31:00Z");

        let first = bb.drain_completed();
        assert_eq!(first.len(), 1);

        let second = bb.drain_completed();
        assert!(second.is_empty());
    }

    #[test]
    fn multiple_symbols_tracked_independently() {
        let mut bb = BarBuilder::new();
        bb.ingest_trade("AAPL", 150.0, 100.0, "2026-01-15T10:30:00Z");
        bb.ingest_trade("MSFT", 300.0, 200.0, "2026-01-15T10:30:00Z");
        bb.ingest_trade("AAPL", 155.0, 50.0, "2026-01-15T10:30:30Z");
        bb.ingest_trade("MSFT", 295.0, 75.0, "2026-01-15T10:30:30Z");

        let aapl = bb.get_active_bar("AAPL").unwrap();
        assert_eq!(aapl.open, 150.0);
        assert_eq!(aapl.high, 155.0);
        assert_eq!(aapl.low, 150.0);
        assert_eq!(aapl.volume, 150.0);

        let msft = bb.get_active_bar("MSFT").unwrap();
        assert_eq!(msft.open, 300.0);
        assert_eq!(msft.high, 300.0);
        assert_eq!(msft.low, 295.0);
        assert_eq!(msft.volume, 275.0);

        let all = bb.get_all_active_bars();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn get_active_bar_returns_none_for_unknown_symbol() {
        let bb = BarBuilder::new();
        assert!(bb.get_active_bar("UNKNOWN").is_none());

        let mut bb2 = BarBuilder::new();
        bb2.ingest_trade("AAPL", 150.0, 100.0, "2026-01-15T10:30:00Z");
        assert!(bb2.get_active_bar("MSFT").is_none());
    }
}
