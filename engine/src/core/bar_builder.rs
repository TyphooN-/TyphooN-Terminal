//! WebSocket-driven bar construction: builds 1-minute OHLCV bars from trade stream.
//!
//! Trades arrive via WebSocket (StreamTrade). BarBuilder accumulates them into
//! 1-minute candles. When a new minute starts, the previous candle is "completed"
//! and available for the frontend to consume.
//!
//! This eliminates the 10-second polling loop for live bar updates — the frontend
//! polls completed bars from BarBuilder instead of hitting the API.

use std::collections::{HashMap, VecDeque};

/// Maximum completed bars to buffer before oldest are dropped (prevents unbounded growth).
const MAX_COMPLETED_BARS: usize = 10_000;

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

impl PartialBar {
    fn to_completed(&self, symbol: &str) -> CompletedBar {
        let dt = chrono::DateTime::from_timestamp(self.minute_epoch, 0).unwrap_or_default();
        CompletedBar {
            symbol: symbol.to_string(),
            timestamp: dt.to_rfc3339(),
            open: self.open,
            high: self.high,
            low: self.low,
            close: self.close,
            volume: self.volume,
            trade_count: self.trade_count,
        }
    }
}

/// Builds 1-minute OHLCV bars from incoming trade stream.
/// Thread-safe: wrap in Mutex for shared state access.
pub struct BarBuilder {
    /// Active (still forming) bars, one per symbol
    active: HashMap<String, PartialBar>,
    /// Completed bars ready for frontend consumption
    completed: VecDeque<CompletedBar>,
}

impl Default for BarBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BarBuilder {
    pub fn new() -> Self {
        Self {
            active: HashMap::new(),
            completed: VecDeque::new(),
        }
    }

    /// Ingest a trade. If the trade's minute differs from the active bar's minute,
    /// the active bar is completed and a new one starts.
    ///
    /// Validates: price > 0, size >= 0, price is finite, timestamp parses correctly.
    pub fn ingest_trade(&mut self, symbol: &str, price: f64, size: f64, timestamp_rfc3339: &str) {
        // Validate inputs
        if price <= 0.0 || !price.is_finite() {
            return;
        }
        if size < 0.0 || !size.is_finite() {
            return;
        }
        if symbol.is_empty() {
            return;
        }

        // Parse timestamp to epoch seconds, floor to minute boundary
        let trade_epoch = match chrono::DateTime::parse_from_rfc3339(timestamp_rfc3339) {
            Ok(dt) => dt.timestamp(),
            Err(_) => return, // reject unparseable timestamps instead of silently using epoch 0
        };
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
                let completed = bar.to_completed(symbol);
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
                self.push_completed(completed);
            }
        } else {
            // First trade for this symbol
            self.active.insert(
                symbol.to_string(),
                PartialBar {
                    minute_epoch: trade_minute,
                    open: price,
                    high: price,
                    low: price,
                    close: price,
                    volume: size,
                    trade_count: 1,
                },
            );
        }
    }

    fn push_completed(&mut self, bar: CompletedBar) {
        self.completed.push_back(bar);
        while self.completed.len() > MAX_COMPLETED_BARS {
            self.completed.pop_front();
        }
    }

    /// Flush any active bars older than `max_age_secs` into the completed buffer.
    /// Call periodically (e.g., every 60s) to ensure bars don't stay open indefinitely
    /// when no trades arrive for a symbol.
    pub fn flush_stale(&mut self, max_age_secs: i64) {
        let now = chrono::Utc::now().timestamp();
        let cutoff = now - max_age_secs;
        let stale_syms: Vec<String> = self
            .active
            .iter()
            .filter(|(_, bar)| bar.minute_epoch + 60 < cutoff)
            .map(|(sym, _)| sym.clone())
            .collect();
        for sym in stale_syms {
            if let Some(bar) = self.active.remove(&sym) {
                let completed = bar.to_completed(&sym);
                self.push_completed(completed);
            }
        }
    }

    /// Ingest a quote (bid/ask) to update the active bar's close/high/low.
    /// Useful for instruments where quotes arrive more frequently than trades.
    pub fn ingest_quote(&mut self, symbol: &str, bid: f64, ask: f64) {
        if bid <= 0.0 || ask <= 0.0 || !bid.is_finite() || !ask.is_finite() {
            return;
        }
        let mid = (bid + ask) / 2.0;
        if let Some(bar) = self.active.get_mut(symbol) {
            bar.close = mid;
            bar.high = bar.high.max(mid);
            bar.low = bar.low.min(mid);
        }
    }

    /// Drain all completed bars. Returns them and clears the internal buffer.
    pub fn drain_completed(&mut self) -> Vec<CompletedBar> {
        std::mem::take(&mut self.completed).into_iter().collect()
    }

    /// Get the current (still-forming) bar for a symbol, if any.
    /// Useful for real-time display of the live candle.
    pub fn get_active_bar(&self, symbol: &str) -> Option<CompletedBar> {
        self.active.get(symbol).map(|bar| bar.to_completed(symbol))
    }

    /// Get all active (forming) bars across all symbols.
    pub fn get_all_active_bars(&self) -> Vec<CompletedBar> {
        self.active
            .iter()
            .map(|(sym, bar)| bar.to_completed(sym))
            .collect()
    }

    /// Number of symbols currently being tracked.
    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    /// Number of completed bars waiting to be drained.
    pub fn pending_count(&self) -> usize {
        self.completed.len()
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

    // ── New robustness tests ──

    #[test]
    fn nan_and_infinity_price_rejected() {
        let mut bb = BarBuilder::new();
        bb.ingest_trade("AAPL", f64::NAN, 100.0, "2026-01-15T10:30:00Z");
        bb.ingest_trade("AAPL", f64::INFINITY, 100.0, "2026-01-15T10:30:10Z");
        bb.ingest_trade("AAPL", f64::NEG_INFINITY, 100.0, "2026-01-15T10:30:20Z");
        assert!(bb.get_active_bar("AAPL").is_none());
    }

    #[test]
    fn negative_size_rejected() {
        let mut bb = BarBuilder::new();
        bb.ingest_trade("AAPL", 150.0, -100.0, "2026-01-15T10:30:00Z");
        assert!(bb.get_active_bar("AAPL").is_none());
    }

    #[test]
    fn nan_size_rejected() {
        let mut bb = BarBuilder::new();
        bb.ingest_trade("AAPL", 150.0, f64::NAN, "2026-01-15T10:30:00Z");
        assert!(bb.get_active_bar("AAPL").is_none());
    }

    #[test]
    fn invalid_timestamp_rejected() {
        let mut bb = BarBuilder::new();
        bb.ingest_trade("AAPL", 150.0, 100.0, "not-a-timestamp");
        assert!(bb.get_active_bar("AAPL").is_none());
    }

    #[test]
    fn empty_symbol_rejected() {
        let mut bb = BarBuilder::new();
        bb.ingest_trade("", 150.0, 100.0, "2026-01-15T10:30:00Z");
        assert_eq!(bb.active_count(), 0);
    }

    #[test]
    fn zero_size_trade_accepted() {
        // Zero-size trades are valid (e.g., index ticks with no volume)
        let mut bb = BarBuilder::new();
        bb.ingest_trade("SPX", 5000.0, 0.0, "2026-01-15T10:30:00Z");
        let bar = bb.get_active_bar("SPX").unwrap();
        assert_eq!(bar.volume, 0.0);
        assert_eq!(bar.trade_count, 1);
    }

    #[test]
    fn ingest_quote_updates_active_bar() {
        let mut bb = BarBuilder::new();
        bb.ingest_trade("AAPL", 150.0, 100.0, "2026-01-15T10:30:00Z");

        bb.ingest_quote("AAPL", 152.0, 153.0); // mid = 152.5
        let bar = bb.get_active_bar("AAPL").unwrap();
        assert_eq!(bar.close, 152.5);
        assert_eq!(bar.high, 152.5); // higher than original 150
        assert_eq!(bar.low, 150.0); // original still lowest
        assert_eq!(bar.trade_count, 1); // quotes don't increment trade count
    }

    #[test]
    fn ingest_quote_no_active_bar_ignored() {
        let mut bb = BarBuilder::new();
        bb.ingest_quote("AAPL", 150.0, 151.0);
        assert!(bb.get_active_bar("AAPL").is_none()); // quote alone doesn't create a bar
    }

    #[test]
    fn ingest_quote_invalid_rejected() {
        let mut bb = BarBuilder::new();
        bb.ingest_trade("AAPL", 150.0, 100.0, "2026-01-15T10:30:00Z");
        bb.ingest_quote("AAPL", -1.0, 151.0); // invalid bid
        bb.ingest_quote("AAPL", 150.0, f64::NAN); // invalid ask
        let bar = bb.get_active_bar("AAPL").unwrap();
        assert_eq!(bar.close, 150.0); // unchanged
    }

    #[test]
    fn active_count_and_pending_count() {
        let mut bb = BarBuilder::new();
        assert_eq!(bb.active_count(), 0);
        assert_eq!(bb.pending_count(), 0);

        bb.ingest_trade("AAPL", 150.0, 100.0, "2026-01-15T10:30:00Z");
        bb.ingest_trade("MSFT", 300.0, 200.0, "2026-01-15T10:30:00Z");
        assert_eq!(bb.active_count(), 2);
        assert_eq!(bb.pending_count(), 0);

        bb.ingest_trade("AAPL", 160.0, 50.0, "2026-01-15T10:31:00Z");
        assert_eq!(bb.active_count(), 2);
        assert_eq!(bb.pending_count(), 1);
    }

    #[test]
    fn completed_buffer_is_bounded_as_fifo() {
        let mut bb = BarBuilder::new();
        let start = chrono::DateTime::parse_from_rfc3339("2026-01-15T09:00:00Z").unwrap();
        for minute in 0..=MAX_COMPLETED_BARS + 1 {
            let ts = (start + chrono::Duration::minutes(minute as i64)).to_rfc3339();
            bb.ingest_trade("AAPL", 100.0 + minute as f64, 1.0, &ts);
        }

        assert_eq!(bb.pending_count(), MAX_COMPLETED_BARS);
        let completed = bb.drain_completed();
        assert_eq!(completed.len(), MAX_COMPLETED_BARS);
        assert!(completed[0].timestamp.contains("2026-01-15T09:01:00"));
    }

    #[test]
    fn default_impl() {
        let bb = BarBuilder::default();
        assert_eq!(bb.active_count(), 0);
        assert_eq!(bb.pending_count(), 0);
    }
}
