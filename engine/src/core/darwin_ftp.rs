//! Darwinex FTP raw data parser — reads D-Score components, quote data, and RETURN series
//! from the local NAS mirror of the Darwinex FTP feed.
//!
//! Data format: flat files per DARWIN with one row per trading day.
//! Format: `timestamp_ms,d_score[,extra_data]`
//! Extra data uses Python literal syntax (lists, tuples, None).
//!
//! Reference: https://help.darwinex.com/raw-darwin-data-user-guide

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ── Data Types ──────────────────────────────────────────────────────

/// A single daily data point from any D-Score component file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DScorePoint {
    pub timestamp_ms: i64,
    pub score: f64,
    pub raw_extra: String,  // unparsed extra data (Python literal)
}

/// Parsed RETURN file entry with cumulative equity curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnPoint {
    pub timestamp_ms: i64,
    pub score: f64,
    pub cumulative_returns: Vec<f64>,  // equity multiplier series (1.0 = start)
}

/// Parsed POSITIONS file entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionPoint {
    pub timestamp_ms: i64,
    pub score: f64,
    pub positions: Vec<FtpPosition>,
    pub open_count: i32,
    pub closed_count: i32,
}

/// A single position from the POSITIONS file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FtpPosition {
    pub symbol: String,
    pub total_trades: i32,
    pub wins: i32,
    pub losses: i32,
    pub best_return: f64,
    pub worst_return: f64,
    pub min_hold_ms: i64,
    pub max_hold_ms: i64,
}

/// Parsed EXPERIENCE file entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperiencePoint {
    pub timestamp_ms: i64,
    pub score: f64,
    pub trade_count: i32,
    pub months: f64,
}

/// Quote tick from a gzipped quote CSV.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteTick {
    pub timestamp_ms: i64,
    pub quote: f64,
}

/// Summary of a DARWIN from its FTP data (computed from RETURN file).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarwinFtpSummary {
    pub ticker: String,
    pub trading_days: usize,
    pub total_return_pct: f64,
    pub max_drawdown_pct: f64,
    pub sharpe: f64,
    pub sortino: f64,
    pub daily_vol: f64,
    pub best_day_pct: f64,
    pub worst_day_pct: f64,
    pub last_quote: f64,
    pub has_dscore: bool,
    pub has_quotes: bool,
    pub has_former_var10: bool,
    pub experience_score: f64,
    pub risk_stability_score: f64,
    pub performance_score: f64,
}

/// Available data for a DARWIN on the FTP.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DarwinDataAvailability {
    pub ticker: String,
    pub has_return: bool,
    pub has_trades: bool,
    pub has_positions: bool,
    pub has_experience: bool,
    pub has_risk_stability: bool,
    pub has_performance: bool,
    pub has_scalability: bool,
    pub has_market_correlation: bool,
    pub has_badges: bool,
    pub has_quotes: bool,
    pub has_former_var10: bool,
    pub quote_months: Vec<String>,
    pub dscore_days: usize,
}

// ── File Reading ────────────────────────────────────────────────────

/// Validate that a ticker/component string is safe for path construction.
/// Only allows alphanumeric, dots, underscores, and hyphens — blocks path traversal.
fn validate_path_component(s: &str) -> Result<(), String> {
    if s.is_empty() {
        return Err("Empty path component".into());
    }
    if s.contains("..") || s.contains('/') || s.contains('\\') {
        return Err(format!("Path traversal blocked: {}", s));
    }
    if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-') {
        return Err(format!("Invalid characters in path component: {}", s));
    }
    Ok(())
}

/// Construct the path to a DARWIN's component file.
fn component_path(ftp_dir: &Path, ticker: &str, component: &str) -> Result<PathBuf, String> {
    validate_path_component(ticker)?;
    validate_path_component(component)?;
    Ok(ftp_dir.join(ticker).join(component))
}

/// Read a raw D-Score component file. Returns (timestamp_ms, score, extra_data) tuples.
pub fn read_component_file(ftp_dir: &Path, ticker: &str, component: &str) -> Result<Vec<DScorePoint>, String> {
    let path = component_path(ftp_dir, ticker, component)?;
    if !path.is_file() {
        return Err(format!("{}/{} not found", ticker, component));
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Read {}/{} failed: {e}", ticker, component))?;

    let mut points = Vec::new();
    for line in content.lines() {
        if line.is_empty() { continue; }
        let (ts, score, extra) = parse_dscore_line(line);
        if ts > 0 {
            points.push(DScorePoint { timestamp_ms: ts, score, raw_extra: extra });
        }
    }
    Ok(points)
}

/// Parse a single D-Score line: "timestamp,score[,extra...]"
fn parse_dscore_line(line: &str) -> (i64, f64, String) {
    // Split on first two commas only — extra data may contain commas
    let mut parts = line.splitn(3, ',');
    let ts: i64 = parts.next().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    let score: f64 = parts.next().and_then(|s| s.trim().parse().ok()).unwrap_or(0.0);
    let extra = parts.next().unwrap_or("").to_string();
    (ts, score, extra)
}

// ── RETURN Parsing ──────────────────────────────────────────────────

/// Read and parse the RETURN file — extracts daily returns + cumulative equity curve.
pub fn read_return_file(ftp_dir: &Path, ticker: &str) -> Result<Vec<ReturnPoint>, String> {
    let points = read_component_file(ftp_dir, ticker, "RETURN")?;
    let mut results = Vec::new();

    for p in &points {
        let returns = parse_float_array(&p.raw_extra);
        results.push(ReturnPoint {
            timestamp_ms: p.timestamp_ms,
            score: p.score,
            cumulative_returns: returns,
        });
    }
    Ok(results)
}

/// Extract daily percentage returns from cumulative RETURN data.
/// The RETURN file stores cumulative equity multipliers. To get daily returns:
/// daily_return[i] = (last_value[i] / last_value[i-1]) - 1
pub fn compute_daily_returns_from_ftp(returns: &[ReturnPoint]) -> Vec<f64> {
    let mut daily = Vec::new();
    let mut prev_equity = 1.0;

    for r in returns {
        if let Some(&last_val) = r.cumulative_returns.last() {
            if prev_equity > 0.0 && last_val > 0.0 {
                let day_ret = (last_val / prev_equity) - 1.0;
                daily.push(day_ret);
            }
            prev_equity = last_val;
        }
    }
    daily
}

/// Compute summary statistics from RETURN file data.
pub fn compute_return_summary(ticker: &str, returns: &[ReturnPoint]) -> DarwinFtpSummary {
    let daily_rets = compute_daily_returns_from_ftp(returns);
    let n = daily_rets.len();

    let last_cumulative = returns.last()
        .and_then(|r| r.cumulative_returns.last().copied())
        .unwrap_or(1.0);
    let total_return = (last_cumulative - 1.0) * 100.0;

    // Max drawdown from cumulative returns
    let mut peak = 1.0_f64;
    let mut max_dd = 0.0_f64;
    for r in returns {
        if let Some(&val) = r.cumulative_returns.last() {
            if val > peak { peak = val; }
            if peak > 0.0 {
                let dd = (peak - val) / peak * 100.0;
                if dd > max_dd { max_dd = dd; }
            }
        }
    }

    // Daily stats
    let mean = if n > 0 { daily_rets.iter().sum::<f64>() / n as f64 } else { 0.0 };
    let var = if n > 1 {
        daily_rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1) as f64
    } else { 0.0 };
    let daily_vol = var.sqrt();
    let ann_vol = daily_vol * (252.0_f64).sqrt();

    let sharpe = if ann_vol > 0.0 { (mean * 252.0) / ann_vol } else { 0.0 };

    let downside_var = if n > 1 {
        daily_rets.iter().filter(|&&r| r < 0.0).map(|r| r.powi(2)).sum::<f64>()
            / (n - 1) as f64
    } else { 0.0 };
    let sortino = if downside_var > 0.0 { (mean * 252.0) / (downside_var.sqrt() * (252.0_f64).sqrt()) } else { 0.0 };

    let best = daily_rets.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let worst = daily_rets.iter().cloned().fold(f64::INFINITY, f64::min);

    DarwinFtpSummary {
        ticker: ticker.to_string(),
        trading_days: n,
        total_return_pct: total_return,
        max_drawdown_pct: max_dd,
        sharpe,
        sortino,
        daily_vol,
        best_day_pct: if best.is_finite() { best * 100.0 } else { 0.0 },
        worst_day_pct: if worst.is_finite() { worst * 100.0 } else { 0.0 },
        last_quote: last_cumulative * 100.0,  // DARWIN price (starts at 100)
        has_dscore: true,
        has_quotes: false,  // caller fills this
        has_former_var10: false,
        experience_score: returns.last().map(|r| r.score).unwrap_or(0.0),
        risk_stability_score: 0.0,
        performance_score: 0.0,
    }
}

// ── POSITIONS Parsing ───────────────────────────────────────────────

/// Read and parse the POSITIONS file.
pub fn read_positions_file(ftp_dir: &Path, ticker: &str) -> Result<Vec<PositionPoint>, String> {
    let points = read_component_file(ftp_dir, ticker, "POSITIONS")?;
    let mut results = Vec::new();

    for p in &points {
        let (positions, open, closed) = parse_positions_extra(&p.raw_extra);
        results.push(PositionPoint {
            timestamp_ms: p.timestamp_ms,
            score: p.score,
            positions,
            open_count: open,
            closed_count: closed,
        });
    }
    Ok(results)
}

/// Parse the POSITIONS extra data.
/// Format: `[['SYM', 5, 2, 3, 1.02, 0.99, 22582957, 63028797]],5,4`
fn parse_positions_extra(extra: &str) -> (Vec<FtpPosition>, i32, i32) {
    let mut positions = Vec::new();
    if extra.is_empty() { return (positions, 0, 0); }

    // Find the outer array boundaries
    let arr_start = extra.find("[[");
    let arr_end = extra.find("]]");

    let (open, closed) = if let Some(end_idx) = arr_end {
        // After "]]" there should be ",open,closed"
        let tail = &extra[end_idx + 2..];
        let nums: Vec<i32> = tail.split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        (nums.first().copied().unwrap_or(0), nums.get(1).copied().unwrap_or(0))
    } else {
        // No position array — try to parse just the trailing numbers
        let nums: Vec<i32> = extra.split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        (nums.first().copied().unwrap_or(0), nums.get(1).copied().unwrap_or(0))
    };

    if let (Some(start), Some(end)) = (arr_start, arr_end) {
        let inner = &extra[start + 1..end + 1]; // includes inner brackets
        // Split by "], [" to get individual positions
        for pos_str in inner.split("'], ['").chain(inner.split("], [")) {
            let clean = pos_str.replace(['[', ']', '\''], "");
            let parts: Vec<&str> = clean.split(',').map(|s| s.trim()).collect();
            if parts.len() >= 8 {
                positions.push(FtpPosition {
                    symbol: parts[0].to_string(),
                    total_trades: parts[1].parse().unwrap_or(0),
                    wins: parts[2].parse().unwrap_or(0),
                    losses: parts[3].parse().unwrap_or(0),
                    best_return: parts[4].parse().unwrap_or(0.0),
                    worst_return: parts[5].parse().unwrap_or(0.0),
                    min_hold_ms: parts[6].parse().unwrap_or(0),
                    max_hold_ms: parts[7].parse().unwrap_or(0),
                });
            }
        }
    }

    (positions, open, closed)
}

// ── EXPERIENCE Parsing ──────────────────────────────────────────────

/// Read and parse the EXPERIENCE file.
pub fn read_experience_file(ftp_dir: &Path, ticker: &str) -> Result<Vec<ExperiencePoint>, String> {
    let points = read_component_file(ftp_dir, ticker, "EXPERIENCE")?;
    let mut results = Vec::new();

    for p in &points {
        let (count, months) = parse_experience_extra(&p.raw_extra);
        results.push(ExperiencePoint {
            timestamp_ms: p.timestamp_ms,
            score: p.score,
            trade_count: count,
            months,
        });
    }
    Ok(results)
}

fn parse_experience_extra(extra: &str) -> (i32, f64) {
    // Format: [trade_count, months]
    let clean = extra.replace(['[', ']'], "");
    let parts: Vec<&str> = clean.split(',').map(|s| s.trim()).collect();
    let count = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let months = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    (count, months)
}

// ── Quote Reading ───────────────────────────────────────────────────

/// List available quote months for a DARWIN.
pub fn list_quote_months(ftp_dir: &Path, ticker: &str) -> Vec<String> {
    if validate_path_component(ticker).is_err() { return Vec::new(); }
    let quotes_dir = ftp_dir.join(ticker).join("quotes");
    if !quotes_dir.is_dir() { return Vec::new(); }

    let mut months: Vec<String> = std::fs::read_dir(&quotes_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    months.sort();
    months
}

/// Read quote ticks for a specific month. Decompresses all .csv.gz files in the month dir.
pub fn read_quotes_month(ftp_dir: &Path, ticker: &str, month: &str) -> Result<Vec<QuoteTick>, String> {
    validate_path_component(ticker)?;
    validate_path_component(month)?;
    let month_dir = ftp_dir.join(ticker).join("quotes").join(month);
    if !month_dir.is_dir() {
        return Err(format!("Quote month dir not found: {}/{}/quotes/{}", ftp_dir.display(), ticker, month));
    }

    let mut all_ticks = Vec::new();
    let entries: Vec<_> = std::fs::read_dir(&month_dir)
        .map_err(|e| format!("Read dir failed: {e}"))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "gz").unwrap_or(false))
        .collect();

    for entry in &entries {
        match read_gzipped_quotes(&entry.path()) {
            Ok(ticks) => all_ticks.extend(ticks),
            Err(_) => continue,
        }
    }

    all_ticks.sort_by_key(|t| t.timestamp_ms);
    Ok(all_ticks)
}

/// Read a single gzipped quote CSV file.
fn read_gzipped_quotes(path: &Path) -> Result<Vec<QuoteTick>, String> {
    use std::io::Read;
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Open failed: {e}"))?;
    let mut decoder = flate2::read::GzDecoder::new(file);
    let mut content = String::new();
    decoder.read_to_string(&mut content)
        .map_err(|e| format!("Decompress failed: {e}"))?;

    let mut ticks = Vec::new();
    for line in content.lines() {
        if line.starts_with("timestamp") { continue; } // skip header
        let parts: Vec<&str> = line.splitn(2, ',').collect();
        if parts.len() == 2 {
            if let (Ok(ts), Ok(q)) = (parts[0].parse::<i64>(), parts[1].parse::<f64>()) {
                ticks.push(QuoteTick { timestamp_ms: ts, quote: q });
            }
        }
    }
    Ok(ticks)
}

/// Build a daily OHLC series from tick quotes (for charting a DARWIN's price).
pub fn quotes_to_daily_ohlc(ticks: &[QuoteTick]) -> Vec<(i64, f64, f64, f64, f64)> {
    use std::collections::BTreeMap;
    let mut daily: BTreeMap<i64, (f64, f64, f64, f64)> = BTreeMap::new();

    for tick in ticks {
        let day = tick.timestamp_ms / 86_400_000 * 86_400_000; // floor to day
        let entry = daily.entry(day).or_insert((tick.quote, tick.quote, tick.quote, tick.quote));
        if tick.quote > entry.1 { entry.1 = tick.quote; } // high
        if tick.quote < entry.2 { entry.2 = tick.quote; } // low
        entry.3 = tick.quote; // close = last tick
    }

    daily.into_iter()
        .map(|(ts, (o, h, l, c))| (ts, o, h, l, c))
        .collect()
}

// ── Data Availability ───────────────────────────────────────────────

/// Check what data is available for a DARWIN on the FTP.
pub fn check_availability(ftp_dir: &Path, ticker: &str) -> DarwinDataAvailability {
    if validate_path_component(ticker).is_err() {
        return DarwinDataAvailability { ticker: ticker.to_string(), ..Default::default() };
    }
    let darwin_dir = ftp_dir.join(ticker);
    if !darwin_dir.is_dir() {
        return DarwinDataAvailability { ticker: ticker.to_string(), ..Default::default() };
    }

    let has_file = |name: &str| -> bool {
        let p = darwin_dir.join(name);
        p.is_file() && p.metadata().map(|m| m.len() > 0).unwrap_or(false)
    };

    let quote_months = list_quote_months(ftp_dir, ticker);
    let dscore_days = if has_file("RETURN") {
        std::fs::read_to_string(darwin_dir.join("RETURN"))
            .map(|c| c.lines().filter(|l| !l.is_empty()).count())
            .unwrap_or(0)
    } else { 0 };

    let former_dir = format!("_{}_former_var10", ticker);

    DarwinDataAvailability {
        ticker: ticker.to_string(),
        has_return: has_file("RETURN"),
        has_trades: has_file("TRADES"),
        has_positions: has_file("POSITIONS"),
        has_experience: has_file("EXPERIENCE"),
        has_risk_stability: has_file("RISK_STABILITY"),
        has_performance: has_file("PERFORMANCE"),
        has_scalability: has_file("SCALABILITY"),
        has_market_correlation: has_file("MARKET_CORRELATION"),
        has_badges: has_file("BADGES"),
        has_quotes: !quote_months.is_empty(),
        has_former_var10: darwin_dir.join(&former_dir).is_dir(),
        quote_months,
        dscore_days,
    }
}

// ── Universe Scanning ───────────────────────────────────────────────

/// List all DARWIN tickers in the FTP directory.
pub fn list_all_darwins(ftp_dir: &Path) -> Result<Vec<String>, String> {
    let entries = std::fs::read_dir(ftp_dir)
        .map_err(|e| format!("Read FTP dir failed: {e}"))?;

    let mut tickers: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|name| !name.starts_with('.') && name.chars().all(|c| c.is_ascii_uppercase()))
        .collect();

    tickers.sort();
    Ok(tickers)
}

/// Scan all DARWINs with RETURN data and compute summaries.
/// This is the "universe index" — reads ~50KB per DARWIN, sequential I/O.
/// On ZFS raidz2 with 50K DARWINs, takes ~5-10 seconds.
pub fn scan_universe(
    ftp_dir: &Path,
    min_days: usize,
    progress: Option<&dyn Fn(usize, usize)>,
) -> Result<Vec<DarwinFtpSummary>, String> {
    let tickers = list_all_darwins(ftp_dir)?;
    let total = tickers.len();
    let mut summaries = Vec::new();

    for (i, ticker) in tickers.iter().enumerate() {
        if let Some(ref cb) = progress {
            if i % 1000 == 0 { cb(i, total); }
        }

        // Direct path construction — avoid recursive find
        let return_path = ftp_dir.join(ticker).join("RETURN");
        if !return_path.is_file() { continue; }

        match read_return_file(ftp_dir, ticker) {
            Ok(returns) if returns.len() >= min_days => {
                let mut summary = compute_return_summary(ticker, &returns);
                summary.has_quotes = ftp_dir.join(ticker).join("quotes").is_dir();
                summary.has_former_var10 = ftp_dir.join(ticker).join(format!("_{}_former_var10", ticker)).is_dir();

                // Read latest D-Score component scores
                if let Ok(rs) = read_component_file(ftp_dir, ticker, "RISK_STABILITY") {
                    summary.risk_stability_score = rs.last().map(|p| p.score).unwrap_or(0.0);
                }
                if let Ok(perf) = read_component_file(ftp_dir, ticker, "PERFORMANCE") {
                    summary.performance_score = perf.last().map(|p| p.score).unwrap_or(0.0);
                }

                summaries.push(summary);
            }
            _ => continue,
        }
    }

    // Sort by Sharpe descending
    summaries.sort_by(|a, b| b.sharpe.partial_cmp(&a.sharpe).unwrap_or(std::cmp::Ordering::Equal));
    Ok(summaries)
}

/// Compute pairwise correlation between two DARWINs using their RETURN files.
pub fn compute_correlation(ftp_dir: &Path, ticker_a: &str, ticker_b: &str) -> Result<f64, String> {
    let returns_a = read_return_file(ftp_dir, ticker_a)?;
    let returns_b = read_return_file(ftp_dir, ticker_b)?;

    let daily_a = compute_daily_returns_from_ftp(&returns_a);
    let daily_b = compute_daily_returns_from_ftp(&returns_b);

    // Align by taking min length
    let n = daily_a.len().min(daily_b.len());
    if n < 30 { return Err("Need 30+ overlapping days for correlation".into()); }

    let a = &daily_a[daily_a.len() - n..];
    let b = &daily_b[daily_b.len() - n..];

    let mean_a = a.iter().sum::<f64>() / n as f64;
    let mean_b = b.iter().sum::<f64>() / n as f64;

    let mut cov = 0.0;
    let mut var_a = 0.0;
    let mut var_b = 0.0;
    for i in 0..n {
        let da = a[i] - mean_a;
        let db = b[i] - mean_b;
        cov += da * db;
        var_a += da * da;
        var_b += db * db;
    }

    let denom = (var_a * var_b).sqrt();
    if denom == 0.0 { return Ok(0.0); }
    Ok(cov / denom)
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Parse a Python-style float array: "[1.0, 1.002, 0.998]" → Vec<f64>
fn parse_float_array(s: &str) -> Vec<f64> {
    let clean = s.trim().trim_start_matches('[').trim_end_matches(']');
    if clean.is_empty() { return Vec::new(); }
    clean.split(',')
        .filter_map(|v| v.trim().parse::<f64>().ok())
        .collect()
}

/// Convert millisecond timestamp to YYYY-MM-DD string.
pub fn ms_to_date(ts_ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ts_ms)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}
