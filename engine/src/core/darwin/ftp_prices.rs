use serde::{Deserialize, Serialize};

// ── FTP Quote / Price Series ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarwinQuoteBar {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

/// Build a synthetic OHLC price series from a DARWIN's FTP RETURN file.
///
/// The RETURN file contains one line per day:
///   `timestamp,experience_score,[cumulative_return_values...]`
/// where return values are multipliers (1.0 = starting point).
///
/// We convert to a price series starting at 100.0.  Each day's intra-day
/// values become the high/low; first value = open, last value = close.
/// The `timeframe` parameter controls aggregation: "1Day", "1Week", or
/// "1Month".
pub fn get_darwin_price_series(
    ftp_path: &str,
    darwin_ticker: &str,
    timeframe: &str,
) -> Result<Vec<DarwinQuoteBar>, String> {
    let return_path = std::path::Path::new(ftp_path)
        .join(darwin_ticker)
        .join("RETURN");

    if !return_path.exists() {
        return Err(format!("RETURN file not found: {}", return_path.display()));
    }

    let content =
        std::fs::read_to_string(&return_path).map_err(|e| format!("Read RETURN failed: {e}"))?;

    let base_price = 100.0f64;

    // Parse each line into a daily bar
    let mut daily_bars: Vec<DarwinQuoteBar> = Vec::new();
    let mut prev_close = base_price;

    for line in content.lines() {
        let parts: Vec<&str> = line.splitn(3, ',').collect();
        if parts.len() < 3 {
            continue;
        }

        let timestamp = parts[0].trim();
        // Extract date portion (YYYY-MM-DD) from timestamp
        let date = if timestamp.len() >= 10 {
            &timestamp[..10]
        } else {
            timestamp
        };

        let vals_str = parts[2].trim_start_matches('[').trim_end_matches(']');

        let values: Vec<f64> = vals_str
            .split(',')
            .filter_map(|s| s.trim().parse::<f64>().ok())
            .collect();

        if values.is_empty() {
            continue;
        }

        let prices: Vec<f64> = values.iter().map(|v| v * base_price).collect();

        let open = prev_close;
        let close = *prices.last().unwrap_or(&100.0);
        let mut high = open.max(close);
        let mut low = open.min(close);
        for &p in &prices {
            if p > high {
                high = p;
            }
            if p < low {
                low = p;
            }
        }

        prev_close = close;

        daily_bars.push(DarwinQuoteBar {
            date: date.to_string(),
            open,
            high,
            low,
            close,
        });
    }

    // Aggregate by timeframe
    match timeframe {
        "1Day" => Ok(daily_bars),
        "1Week" => Ok(aggregate_bars(&daily_bars, |d| {
            // ISO week: group by YYYY-Www
            week_key(d)
        })),
        "1Month" => Ok(aggregate_bars(&daily_bars, |d| {
            if d.len() >= 7 {
                d[..7].to_string()
            } else {
                d.to_string()
            }
        })),
        _ => Err(format!(
            "Unsupported timeframe: {timeframe}. Use 1Day, 1Week, or 1Month."
        )),
    }
}

/// Aggregate daily bars into larger periods using a key function.
fn aggregate_bars<F>(bars: &[DarwinQuoteBar], key_fn: F) -> Vec<DarwinQuoteBar>
where
    F: Fn(&str) -> String,
{
    if bars.is_empty() {
        return Vec::new();
    }

    let mut result: Vec<DarwinQuoteBar> = Vec::new();
    let mut current_key = key_fn(&bars[0].date);
    let mut open = bars[0].open;
    let mut high = bars[0].high;
    let mut low = bars[0].low;
    let mut close = bars[0].close;
    let mut date = bars[0].date.clone();

    for bar in bars.iter().skip(1) {
        let k = key_fn(&bar.date);
        if k == current_key {
            if bar.high > high {
                high = bar.high;
            }
            if bar.low < low {
                low = bar.low;
            }
            close = bar.close;
        } else {
            result.push(DarwinQuoteBar {
                date: date.clone(),
                open,
                high,
                low,
                close,
            });
            current_key = k;
            date = bar.date.clone();
            open = bar.open;
            high = bar.high;
            low = bar.low;
            close = bar.close;
        }
    }
    result.push(DarwinQuoteBar {
        date,
        open,
        high,
        low,
        close,
    });
    result
}

/// Derive an ISO-week key "YYYY-Www" from a "YYYY-MM-DD" date string.
fn week_key(date: &str) -> String {
    if date.len() < 10 {
        return date.to_string();
    }
    // Parse year, month, day
    let y: i32 = date[..4].parse().unwrap_or(0);
    let m: u32 = date[5..7].parse().unwrap_or(1);
    let d: u32 = date[8..10].parse().unwrap_or(1);

    // Day-of-year
    let is_leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let mdays: [u32; 12] = [
        31,
        if is_leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let doy: u32 = mdays[..(m as usize - 1)].iter().sum::<u32>() + d;

    // Day of week (Mon=1 .. Sun=7) via Tomohiko Sakamoto
    let t = [0i32, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let yy = if m < 3 { y - 1 } else { y };
    let dow_sun0 = (yy + yy / 4 - yy / 100 + yy / 400 + t[(m - 1) as usize] + d as i32) % 7; // 0=Sun
    let dow_mon1 = if dow_sun0 == 0 { 7u32 } else { dow_sun0 as u32 }; // 1=Mon..7=Sun

    let week = (doy + 7 - dow_mon1) / 7;
    format!("{y}-W{week:02}")
}
