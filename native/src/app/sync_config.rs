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

pub(super) fn tastytrade_earliest_history_ms() -> i64 {
    chrono::NaiveDate::from_ymd_opt(2000, 1, 1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|ndt| ndt.and_utc().timestamp_millis())
        .unwrap_or(0)
}

pub(super) fn tastytrade_initial_from_time_ms(timeframe: &str, now_ms: i64) -> i64 {
    let floor_ms = tastytrade_earliest_history_ms();
    let Some(period_s) = super::sync_timeframe_period_secs(timeframe) else {
        return floor_ms;
    };
    let Some(target_bars) = super::tastytrade_sync_target_bars(timeframe) else {
        return floor_ms;
    };
    let target_bars = i64::from(target_bars);
    let headroom_bars = (target_bars / 20).max(50);
    let lookback_ms = period_s
        .saturating_mul(1000)
        .saturating_mul(target_bars.saturating_add(headroom_bars));
    now_ms.saturating_sub(lookback_ms).max(floor_ms)
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
