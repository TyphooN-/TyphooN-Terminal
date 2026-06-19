use super::*;

mod basic_reversal;
pub use basic_reversal::*;
mod multibar_reversal;
pub use multibar_reversal::*;
mod doji_shadow_star;
pub use doji_shadow_star::*;
mod body_line_shapes;
pub use body_line_shapes::*;
mod neck_line_reversal;
pub use neck_line_reversal::*;
mod crow_line_reversal;
pub use crow_line_reversal::*;
mod separating_sandwich_doji;
pub use separating_sandwich_doji::*;
mod rare_multibar_reversal;
pub use rare_multibar_reversal::*;
mod gap_breakaway_reversal;
pub use gap_breakaway_reversal::*;
mod continuation_gap_patterns;
pub use continuation_gap_patterns::*;

// Candlestick pattern storage/helpers

/// Candle metrics for a single bar: (body, range, upper_shadow,
/// lower_shadow, body_pct_range, is_bullish). body = |close - open|;
/// range = high - low; upper_shadow = high - max(open, close);
/// lower_shadow = min(open, close) - low. body_pct_range = 100 ·
/// body / range (0 when range == 0 to avoid div-by-zero).
#[allow(dead_code)]
fn candle_metrics(bar: &HistoricalPriceRow) -> (f64, f64, f64, f64, f64, bool) {
    let body = (bar.close - bar.open).abs();
    let range = (bar.high - bar.low).max(0.0);
    let top = bar.open.max(bar.close);
    let bot = bar.open.min(bar.close);
    let upper = (bar.high - top).max(0.0);
    let lower = (bot - bar.low).max(0.0);
    let body_pct = if range > 1e-12 {
        100.0 * body / range
    } else {
        0.0
    };
    let bullish = bar.close >= bar.open;
    (body, range, upper, lower, body_pct, bullish)
}

#[allow(dead_code)]
fn candle_body_bounds(bar: &HistoricalPriceRow) -> (f64, f64) {
    (bar.open.min(bar.close), bar.open.max(bar.close))
}

/// Shared helper: scan sorted bars back from end to find the most recent
/// bar matching a predicate, and return (last_bar_match, days_since_pattern,
/// pattern_value_on_last_bar, pattern_value_prev_bar).
#[allow(dead_code)]
fn cdl_scan<F>(sorted: &[&HistoricalPriceRow], min_i: usize, detector: F) -> (bool, usize, i32, i32)
where
    F: Fn(&[&HistoricalPriceRow], usize) -> i32,
{
    let n = sorted.len();
    let last_val = detector(sorted, n - 1);
    let prev_val = if n >= 2 { detector(sorted, n - 2) } else { 0 };
    let last_match = last_val != 0;
    let mut days_since: usize = 0;
    if !last_match {
        let mut idx = n - 1;
        while idx > min_i {
            if detector(sorted, idx) != 0 {
                days_since = (n - 1) - idx;
                break;
            }
            idx -= 1;
        }
        if days_since == 0 {
            days_since = (n - 1) - min_i;
        }
    }
    (last_match, days_since, last_val, prev_val)
}
