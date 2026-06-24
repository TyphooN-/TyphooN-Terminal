use super::*;

pub fn format_price(p: f64) -> String {
    if p == 0.0 {
        return "0".into();
    }
    let abs = p.abs();
    if abs >= 10_000.0 {
        format!("{:.2}", p)
    } else if abs >= 1.0 {
        format!("{:.4}", p)
    } else {
        format!("{:.4}", p)
    }
}

pub(super) fn format_axis_price_label(prefix: &str, price: f64) -> String {
    format!("{} {}", prefix, format_price(price))
}

pub(super) fn format_ext_hours_symbol_badge(
    close: f64,
    ext_last: f64,
    prev_close: Option<f64>,
) -> String {
    let delta = ext_last - close;
    let pct = if close.abs() > f64::EPSILON {
        delta / close * 100.0
    } else {
        0.0
    };
    let day_pct = prev_close
        .and_then(|prev| (prev.abs() > f64::EPSILON).then_some((ext_last / prev - 1.0) * 100.0));
    if let Some(day_pct) = day_pct {
        format!(
            "Daily Close {} ({:+.2}%)  EXT last {}  Δ/C {} ({:+.2}%)",
            format_price(close),
            day_pct,
            format_price(ext_last),
            format_signed_price(delta),
            pct
        )
    } else {
        format!(
            "Daily Close {}  EXT last {}  Δ/C {} ({:+.2}%)",
            format_price(close),
            format_price(ext_last),
            format_signed_price(delta),
            pct
        )
    }
}

pub(super) fn format_signed_price(p: f64) -> String {
    if p < 0.0 {
        format!("-{}", format_price(p.abs()))
    } else {
        format!("+{}", format_price(p))
    }
}

/// Buffer-reusing variant of format_price — writes into caller's String to avoid heap alloc per call.
pub fn format_price_buf(p: f64, buf: &mut String) {
    use std::fmt::Write;
    buf.clear();
    if p == 0.0 {
        buf.push('0');
        return;
    }
    let abs = p.abs();
    if abs >= 10_000.0 {
        write!(buf, "{:.2}", p).ok();
    } else if abs >= 1.0 {
        write!(buf, "{:.4}", p).ok();
    } else {
        write!(buf, "{:.6}", p).ok();
    }
}

pub fn format_ts(ts_ms: i64, tf: Timeframe) -> String {
    let mut buf = String::with_capacity(18);
    format_ts_buf(ts_ms, tf, &mut buf);
    buf
}

pub(super) fn now_unix_ms() -> Option<i64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())
}

pub(super) fn chart_symbol_looks_crypto(symbol: &str) -> bool {
    let bare = bare_symbol_from_key(symbol)
        .replace('/', "")
        .trim_end_matches(".P")
        .trim_end_matches(".PF")
        .to_ascii_uppercase();
    const CRYPTO_BASES: &[&str] = &[
        "BTC", "XBT", "ETH", "SOL", "DOGE", "XRP", "ADA", "LTC", "LINK", "AVAX", "DOT", "XMR",
        "ZEC", "DASH", "UNI", "AAVE", "MATIC", "SHIB", "ATOM", "ALGO", "FTM", "NEAR", "APE", "ARB",
        "OP", "MKR", "COMP", "SNX", "CRV", "SUSHI", "YFI", "BAT", "MANA",
    ];
    CRYPTO_BASES
        .iter()
        .any(|base| bare == *base || bare.starts_with(base) && bare.ends_with("USD"))
}

pub(super) fn chart_source_trades_weekends(primary_source: &str, symbol: &str) -> bool {
    matches!(primary_source, "kraken-futures") || chart_symbol_looks_crypto(symbol)
}

pub(super) fn nth_weekday_of_month_utc_day(
    year: i32,
    month: u32,
    weekday: chrono::Weekday,
    nth: u32,
) -> u32 {
    use chrono::{Datelike, NaiveDate};
    let first = NaiveDate::from_ymd_opt(year, month, 1).expect("valid month");
    let first_idx = first.weekday().num_days_from_sunday() as i32;
    let target_idx = weekday.num_days_from_sunday() as i32;
    let delta = (target_idx - first_idx).rem_euclid(7) as u32;
    1 + delta + (nth.saturating_sub(1) * 7)
}

pub(super) fn us_eastern_offset_seconds(utc: chrono::DateTime<chrono::Utc>) -> i64 {
    use chrono::{Datelike, Timelike};
    let year = utc.year();
    let dst_start_day = nth_weekday_of_month_utc_day(year, 3, chrono::Weekday::Sun, 2);
    let dst_end_day = nth_weekday_of_month_utc_day(year, 11, chrono::Weekday::Sun, 1);
    let ordinal = utc.ordinal();
    let start = chrono::NaiveDate::from_ymd_opt(year, 3, dst_start_day)
        .expect("valid DST start")
        .ordinal();
    let end = chrono::NaiveDate::from_ymd_opt(year, 11, dst_end_day)
        .expect("valid DST end")
        .ordinal();
    let hour = utc.hour();

    // US Eastern DST runs from 02:00 local standard time (07:00 UTC) on the
    // second Sunday in March until 02:00 local daylight time (06:00 UTC) on the
    // first Sunday in November.
    let in_dst = if ordinal < start || ordinal > end {
        false
    } else if ordinal > start && ordinal < end {
        true
    } else if ordinal == start {
        hour >= 7
    } else {
        hour < 6
    };

    if in_dst { -4 * 3600 } else { -5 * 3600 }
}

pub(super) fn us_equity_weekend_closed_at_ms(now_ms: i64) -> bool {
    use chrono::{Datelike, TimeZone, Timelike};
    let Some(now_utc) = chrono::Utc.timestamp_millis_opt(now_ms).single() else {
        return false;
    };
    let now_et =
        now_utc.naive_utc() + chrono::Duration::seconds(us_eastern_offset_seconds(now_utc));
    let minute_of_day = now_et.hour() as i64 * 60 + now_et.minute() as i64;
    const WEEKEND_CLOSE: i64 = 20 * 60;
    match now_et.weekday() {
        chrono::Weekday::Fri => minute_of_day >= WEEKEND_CLOSE,
        chrono::Weekday::Sat => true,
        chrono::Weekday::Sun => minute_of_day < WEEKEND_CLOSE,
        _ => false,
    }
}

pub(super) fn chart_candle_countdown_allowed_at(
    primary_source: &str,
    symbol: &str,
    now_ms: i64,
) -> bool {
    chart_source_trades_weekends(primary_source, symbol) || !us_equity_weekend_closed_at_ms(now_ms)
}

pub(super) fn next_candle_remaining_ms_at(
    last_bar_ts_ms: i64,
    tf: Timeframe,
    now_ms: i64,
) -> Option<i64> {
    let interval_ms = i64::from(tf.minutes()).checked_mul(60_000)?;
    if interval_ms <= 0 || last_bar_ts_ms <= 0 || now_ms <= 0 {
        return None;
    }

    // TradingView-style: show time until the *next* bar boundary, projecting
    // forward from the last known bar. Tolerate a bar that is slightly stale
    // (live feeds replace the forming bar a beat after each boundary), but once
    // more than a full extra interval has elapsed with no replacement the feed
    // has stalled or the session is closed — there is no live bar to count down.
    let elapsed = now_ms.saturating_sub(last_bar_ts_ms);
    if elapsed >= interval_ms.saturating_mul(2) {
        return None;
    }
    let remaining = interval_ms - (elapsed % interval_ms);

    Some(remaining.max(0))
}

pub(super) fn next_candle_countdown_label_for_market(
    last_bar_ts_ms: i64,
    tf: Timeframe,
    primary_source: &str,
    symbol: &str,
) -> Option<String> {
    let now_ms = now_unix_ms()?;
    if !chart_candle_countdown_allowed_at(primary_source, symbol, now_ms) {
        return None;
    }
    let remaining_ms = next_candle_remaining_ms_at(last_bar_ts_ms, tf, now_ms)?;
    Some(format_candle_countdown(remaining_ms))
}

pub(super) fn format_candle_countdown(remaining_ms: i64) -> String {
    let total_secs = (remaining_ms.max(0) + 999) / 1000;
    let days = total_secs / 86_400;
    let hours = (total_secs % 86_400) / 3_600;
    let minutes = (total_secs % 3_600) / 60;
    let seconds = total_secs % 60;

    if days > 0 {
        format!("{days}d {hours:02}:{minutes:02}")
    } else if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

/// Buffer-reusing variant of format_ts — writes into caller's String to avoid heap alloc per call.
pub fn format_ts_buf(ts_ms: i64, tf: Timeframe, buf: &mut String) {
    use chrono::TimeZone;
    buf.clear();
    let dt = chrono::Utc
        .timestamp_millis_opt(ts_ms)
        .single()
        .unwrap_or_default();
    use std::fmt::Write;
    match tf {
        Timeframe::MN1 => {
            write!(buf, "{}", dt.format("%b %Y")).ok();
        }
        Timeframe::W1 | Timeframe::D1 => {
            write!(buf, "{}", dt.format("%d %b'%y")).ok();
        }
        _ => {
            write!(buf, "{}", dt.format("%d %b'%y %H:%M")).ok();
        }
    };
}

/// Smallest "nice" axis-label stride (in minutes) whose on-screen spacing is at
/// least `min_gap_px`, drawn from a human ladder of clock/calendar boundaries
/// (…15m, 30m, 1h, 2h, 3h, 4h, 6h, 12h, 1d, 2d, 3d, 1w…). Aligning labels to
/// these boundaries — instead of "every N bars" — is what makes intraday times
/// land on round values (14:00, 18:00) rather than whatever bar the step hit.
pub(super) fn intraday_axis_stride_minutes(tf_minutes: u32, bar_w: f32, min_gap_px: f32) -> i64 {
    const LADDER: &[i64] = &[
        1, 2, 5, 15, 30, // sub-hour
        60, 120, 180, 240, 360, 720, // 1h … 12h
        1440, 2880, 4320, 10080, 20160, 43200, // 1d … ~1mo
    ];
    let tf = (tf_minutes.max(1)) as i64;
    for &stride in LADDER {
        if stride < tf {
            continue;
        }
        // bars per stride × pixels per bar = on-screen gap between labels
        let gap_px = (stride as f32 / tf as f32) * bar_w;
        if gap_px >= min_gap_px {
            return stride;
        }
    }
    *LADDER.last().unwrap()
}

/// Hierarchical, boundary-aligned time axis for intraday timeframes.
///
/// The old scheme labelled every Nth bar with the full `%d %b'%y %H:%M` string;
/// on lower timeframes each ~76px label sat ~80px from its neighbour and the
/// repeated date made them collide into an unreadable smear. Here labels are
/// placed only where a chosen clock boundary is crossed (so they fall on round
/// times), never closer than `MIN_GAP_PX`, and each shows only what changed:
/// the date ("08 May") when the day rolls over, otherwise the time ("14:00").
pub(super) fn draw_intraday_time_axis(
    painter: &egui::Painter,
    bars: &[Bar],
    data_left: f32,
    bar_w: f32,
    chart_rect: egui::Rect,
    tf_minutes: u32,
    grid_stroke: egui::Stroke,
    label_buf: &mut String,
) {
    use chrono::{Datelike, TimeZone, Timelike};
    use std::fmt::Write;
    const MIN_GAP_PX: f32 = 64.0;

    if bars.is_empty() {
        return;
    }
    let stride_ms = intraday_axis_stride_minutes(tf_minutes, bar_w, MIN_GAP_PX) * 60_000;
    if stride_ms <= 0 {
        return;
    }
    let font = egui::FontId::monospace(9.0);
    let mut last_label_x = f32::NEG_INFINITY;
    let mut last_label_date: Option<chrono::NaiveDate> = None;
    let mut prev_bucket: Option<i64> = None;

    for (rel_idx, bar) in bars.iter().enumerate() {
        // A label is a candidate only on the bar that first crosses each stride
        // boundary (and always the first visible bar, for left-edge context).
        let bucket = bar.ts_ms.div_euclid(stride_ms);
        let is_boundary = prev_bucket != Some(bucket);
        prev_bucket = Some(bucket);
        if !is_boundary {
            continue;
        }
        let x = data_left + (rel_idx as f32 + 0.5) * bar_w;
        // Time gaps (weekend/overnight) can place two boundaries on adjacent
        // bars; the pixel-gap guard keeps them from overprinting.
        if x - last_label_x < MIN_GAP_PX {
            continue;
        }
        let dt = chrono::Utc
            .timestamp_millis_opt(bar.ts_ms)
            .single()
            .unwrap_or_default();
        let date = dt.date_naive();
        let first_label = last_label_date.is_none();
        let new_day = last_label_date != Some(date);
        let new_year = last_label_date
            .map(|d| d.year() != date.year())
            .unwrap_or(true);

        label_buf.clear();
        if first_label || new_year {
            // First tick / year rollover: anchor with the year for context.
            let _ = write!(label_buf, "{}", dt.format("%d %b'%y"));
        } else if new_day {
            let _ = write!(label_buf, "{}", dt.format("%d %b"));
        } else {
            let _ = write!(label_buf, "{:02}:{:02}", dt.hour(), dt.minute());
        }

        painter.line_segment(
            [
                egui::pos2(x, chart_rect.top()),
                egui::pos2(x, chart_rect.bottom()),
            ],
            grid_stroke,
        );
        painter.text(
            egui::pos2(x, chart_rect.bottom() + 2.0),
            egui::Align2::CENTER_TOP,
            label_buf.as_str(),
            font.clone(),
            AXIS_TEXT,
        );
        last_label_x = x;
        last_label_date = Some(date);
    }
}
