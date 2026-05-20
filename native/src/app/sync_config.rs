//! Broker market-data sync budgets and small timeframe helpers.
//!
//! Kept out of `app.rs` so scheduler policy has a small, compile-checkable home
//! instead of adding more constants and helper code to the main application unit.

pub(super) const KRAKEN_PUBLIC_FETCH_PERMITS: usize = 16;
pub(super) const KRAKEN_SPOT_QUEUE_WINDOW: usize = 160;
pub(super) const KRAKEN_FUTURES_QUEUE_WINDOW: usize = 96;
pub(super) const ALPACA_BACKGROUND_SCAN_LIMIT: usize = 384;
pub(super) const KRAKEN_SPOT_BACKGROUND_SCAN_LIMIT: usize = 384;
pub(super) const KRAKEN_FUTURES_BACKGROUND_SCAN_LIMIT: usize = 192;
pub(super) const TASTYTRADE_BACKGROUND_SCAN_LIMIT: usize = 96;

/// Largest `MAX_BARS` value that can safely cross the MT5 demand.txt / MQL5
/// boundary. This is a provider-maximum sentinel, not a local history target:
/// the terminal asks the EA for everything the broker server can provide, then
/// the saturation memory suppresses repeat full requests once the count stops
/// growing for a symbol/timeframe.
pub(super) const MT5_PROVIDER_MAX_BARS: u32 = i32::MAX as u32;

/// Kraken Spot public OHLC is a provider-window API, not a traversal API. Kraken
/// documents the endpoint as returning the most recent ~720 candles per interval
/// (monthly is shorter in practice), so these values are external provider
/// windows rather than arbitrary shallow-cache caps.
pub(super) const KRAKEN_SPOT_PROVIDER_WINDOW_BARS: u32 = 720;
pub(super) const KRAKEN_SPOT_MONTH_PROVIDER_WINDOW_BARS: u32 = 24;

pub(super) fn tastytrade_earliest_history_ms() -> i64 {
    chrono::NaiveDate::from_ymd_opt(2000, 1, 1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|ndt| ndt.and_utc().timestamp_millis())
        .unwrap_or(0)
}

pub(super) fn tastytrade_initial_from_time_ms(_timeframe: &str, _now_ms: i64) -> i64 {
    tastytrade_earliest_history_ms()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tastytrade_initial_time_is_floor_bounded() {
        assert_eq!(
            tastytrade_initial_from_time_ms("UNKNOWN", 0),
            tastytrade_earliest_history_ms()
        );
        assert_eq!(
            tastytrade_initial_from_time_ms("D1", 0),
            tastytrade_earliest_history_ms()
        );
    }

    #[test]
    fn tastytrade_initial_time_keeps_headroom_when_recent() {
        let now_ms = chrono::NaiveDate::from_ymd_opt(2025, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        let start = tastytrade_initial_from_time_ms("D1", now_ms);
        assert!(start < now_ms);
        assert!(start >= tastytrade_earliest_history_ms());
    }
}
