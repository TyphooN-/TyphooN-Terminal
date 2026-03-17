//! TyphooN Terminal — High-performance indicator computation (Wasm).
//!
//! Ports core indicator math from JavaScript to Rust for 10-50x speedup
//! on optimizer grid search and multi-symbol scanner paths.
//!
//! Compiled to WebAssembly via wasm-bindgen. Loaded in frontend alongside
//! lightweight-charts. JS calc functions remain for chart rendering (fast enough);
//! Wasm versions used for batch computation (optimizer, scanner).

use wasm_bindgen::prelude::*;

#[allow(dead_code)]
/// Bar data as flat f64 arrays for zero-copy Wasm interop.
/// Layout: [open0, high0, low0, close0, vol0, open1, high1, ...]
const FIELDS_PER_BAR: usize = 5; // O, H, L, C, V

#[inline]
fn bar_open(data: &[f64], i: usize) -> f64 { data[i * FIELDS_PER_BAR] }
#[inline]
fn bar_high(data: &[f64], i: usize) -> f64 { data[i * FIELDS_PER_BAR + 1] }
#[inline]
fn bar_low(data: &[f64], i: usize) -> f64 { data[i * FIELDS_PER_BAR + 2] }
#[inline]
fn bar_close(data: &[f64], i: usize) -> f64 { data[i * FIELDS_PER_BAR + 3] }
#[inline]
fn bar_count(data: &[f64]) -> usize { data.len() / FIELDS_PER_BAR }

// ── SMA ─────────────────────────────────────────────────────────────

/// Compute SMA. Returns f64 array of SMA values (length = bars - period + 1).
#[wasm_bindgen]
pub fn wasm_sma(data: &[f64], period: usize) -> Vec<f64> {
    let n = bar_count(data);
    if n < period || period == 0 { return vec![]; }
    let mut result = Vec::with_capacity(n - period + 1);
    let mut sum = 0.0;
    for i in 0..period { sum += bar_close(data, i); }
    result.push(sum / period as f64);
    for i in period..n {
        sum += bar_close(data, i) - bar_close(data, i - period);
        result.push(sum / period as f64);
    }
    result
}

// ── EMA ─────────────────────────────────────────────────────────────

#[wasm_bindgen]
pub fn wasm_ema(data: &[f64], period: usize) -> Vec<f64> {
    let n = bar_count(data);
    if n == 0 || period == 0 { return vec![]; }
    let k = 2.0 / (period as f64 + 1.0);
    let mut ema = bar_close(data, 0);
    let mut result = Vec::with_capacity(n.saturating_sub(period) + 1);
    for i in 0..n {
        ema = bar_close(data, i) * k + ema * (1.0 - k);
        if i >= period - 1 { result.push(ema); }
    }
    result
}

// ── KAMA (Kaufman Adaptive Moving Average) ──────────────────────────

#[wasm_bindgen]
pub fn wasm_kama(data: &[f64], period: usize, fast_p: usize, slow_p: usize) -> Vec<f64> {
    let n = bar_count(data);
    if n < period + 1 { return vec![]; }
    let fast_sc = 2.0 / (fast_p as f64 + 1.0);
    let slow_sc = 2.0 / (slow_p as f64 + 1.0);
    let mut result = Vec::with_capacity(n - period);
    let mut kama = bar_close(data, period);
    for i in period..n {
        let signal = (bar_close(data, i) - bar_close(data, i.saturating_sub(period))).abs();
        let mut noise = 0.0;
        for j in (i.saturating_sub(period - 1))..=i {
            if j > 0 { noise += (bar_close(data, j) - bar_close(data, j - 1)).abs(); }
        }
        let er = if noise > 1e-10 { signal / noise } else { 0.0 };
        let sc = (er * (fast_sc - slow_sc) + slow_sc).powi(2);
        kama += sc * (bar_close(data, i) - kama);
        result.push(kama);
    }
    result
}

// ── RSI ─────────────────────────────────────────────────────────────

#[wasm_bindgen]
pub fn wasm_rsi(data: &[f64], period: usize) -> Vec<f64> {
    let n = bar_count(data);
    if n < period + 1 { return vec![]; }
    let mut gains = 0.0;
    let mut losses = 0.0;
    for i in 1..=period {
        let change = bar_close(data, i) - bar_close(data, i - 1);
        if change > 0.0 { gains += change; } else { losses -= change; }
    }
    let mut avg_gain = gains / period as f64;
    let mut avg_loss = losses / period as f64;
    let mut result = Vec::with_capacity(n - period);
    for i in period..n {
        if i > period {
            let change = bar_close(data, i) - bar_close(data, i - 1);
            avg_gain = (avg_gain * (period as f64 - 1.0) + if change > 0.0 { change } else { 0.0 }) / period as f64;
            avg_loss = (avg_loss * (period as f64 - 1.0) + if change < 0.0 { -change } else { 0.0 }) / period as f64;
        }
        let rs = if avg_loss == 0.0 { 100.0 } else { avg_gain / avg_loss };
        result.push(100.0 - 100.0 / (1.0 + rs));
    }
    result
}

// ── Ehlers Fisher Transform ─────────────────────────────────────────

#[wasm_bindgen]
pub fn wasm_fisher(data: &[f64], period: usize) -> Vec<f64> {
    let n = bar_count(data);
    if n < period + 1 { return vec![]; }
    let mut result = Vec::with_capacity(n - period);
    let mut prev_smoothed = 0.0;
    let mut prev_fisher = 0.0;
    for i in period..n {
        let mut max_h = f64::NEG_INFINITY;
        let mut min_l = f64::MAX;
        for j in (i - period)..i {
            let h = bar_high(data, j);
            let l = bar_low(data, j);
            if h > max_h { max_h = h; }
            if l < min_l { min_l = l; }
        }
        let price = (bar_high(data, i) + bar_low(data, i)) / 2.0;
        let range = max_h - min_l;
        let normalized = if range > 0.0 { (price - min_l) / range } else { 0.5 };
        let os = 2.0 * (normalized - 0.5);
        let mut smoothed = 0.5 * os + 0.5 * prev_smoothed;
        smoothed = smoothed.clamp(-0.999, 0.999);
        let ft = 0.25 * ((1.0 + smoothed) / (1.0 - smoothed)).ln() + 0.5 * prev_fisher;
        result.push(ft);
        prev_smoothed = smoothed;
        prev_fisher = ft;
    }
    result
}

// ── ATR ─────────────────────────────────────────────────────────────

#[wasm_bindgen]
pub fn wasm_atr(data: &[f64], period: usize) -> Vec<f64> {
    let n = bar_count(data);
    if n < period + 2 { return vec![]; }
    let mut trs = Vec::with_capacity(n - 1);
    for i in 1..n {
        let tr = (bar_high(data, i) - bar_low(data, i))
            .max((bar_high(data, i) - bar_close(data, i - 1)).abs())
            .max((bar_low(data, i) - bar_close(data, i - 1)).abs());
        trs.push(tr);
    }
    let mut atr = trs[..period].iter().sum::<f64>() / period as f64;
    let mut result = Vec::with_capacity(trs.len() - period);
    for i in period..trs.len() {
        atr = (atr * (period as f64 - 1.0) + trs[i]) / period as f64;
        result.push(atr);
    }
    result
}

// ── MACD ────────────────────────────────────────────────────────────

/// Returns [macd_line..., signal_line..., histogram...] concatenated.
/// First value is the count of macd values, then signal count, then histogram count.
#[wasm_bindgen]
pub fn wasm_macd(data: &[f64], fast_p: usize, slow_p: usize, signal_p: usize) -> Vec<f64> {
    let fast_ema = wasm_ema(data, fast_p);
    let slow_ema = wasm_ema(data, slow_p);
    if fast_ema.is_empty() || slow_ema.is_empty() { return vec![]; }
    let offset = fast_ema.len() - slow_ema.len();
    let mut macd_line: Vec<f64> = Vec::with_capacity(slow_ema.len());
    for i in 0..slow_ema.len() {
        macd_line.push(fast_ema[i + offset] - slow_ema[i]);
    }
    if macd_line.len() < signal_p { return vec![]; }
    let k = 2.0 / (signal_p as f64 + 1.0);
    let mut sig = macd_line[0];
    let mut signal: Vec<f64> = Vec::new();
    let mut histogram: Vec<f64> = Vec::new();
    for (i, &m) in macd_line.iter().enumerate() {
        sig = m * k + sig * (1.0 - k);
        if i >= signal_p - 1 {
            signal.push(sig);
            histogram.push(m - sig);
        }
    }
    // Pack as [macd_count, signal_count, hist_count, macd..., signal..., hist...]
    let mut result = Vec::with_capacity(3 + macd_line.len() + signal.len() + histogram.len());
    result.push(macd_line.len() as f64);
    result.push(signal.len() as f64);
    result.push(histogram.len() as f64);
    result.extend(macd_line);
    result.extend(signal);
    result.extend(histogram);
    result
}

// ── Bollinger Bands ─────────────────────────────────────────────────

/// Returns [upper..., lower...] concatenated. Each has (n - period + 1) values.
#[wasm_bindgen]
pub fn wasm_bollinger(data: &[f64], period: usize) -> Vec<f64> {
    let n = bar_count(data);
    if n < period { return vec![]; }
    let count = n - period + 1;
    let mut result = Vec::with_capacity(count * 2);
    for i in (period - 1)..n {
        let mut sum = 0.0;
        let mut sum_sq = 0.0;
        for j in (i + 1 - period)..=i {
            let c = bar_close(data, j);
            sum += c;
            sum_sq += c * c;
        }
        let mean = sum / period as f64;
        let std = (sum_sq / period as f64 - mean * mean).max(0.0).sqrt();
        result.push(mean + 2.0 * std); // upper
    }
    for i in (period - 1)..n {
        let mut sum = 0.0;
        let mut sum_sq = 0.0;
        for j in (i + 1 - period)..=i {
            let c = bar_close(data, j);
            sum += c;
            sum_sq += c * c;
        }
        let mean = sum / period as f64;
        let std = (sum_sq / period as f64 - mean * mean).max(0.0).sqrt();
        result.push(mean - 2.0 * std); // lower
    }
    result
}

// ── Batch Backtest (SMA Cross) ──────────────────────────────────────

/// Run SMA cross backtest on flat bar data. Returns [total_pnl, win_rate, profit_factor, trade_count].
#[wasm_bindgen]
pub fn wasm_backtest_sma(data: &[f64], fast_period: usize, slow_period: usize, equity: f64) -> Vec<f64> {
    let n = bar_count(data);
    if n < slow_period + 2 { return vec![0.0, 0.0, 0.0, 0.0]; }
    let fast_sma = wasm_sma(data, fast_period);
    let slow_sma = wasm_sma(data, slow_period);
    if fast_sma.is_empty() || slow_sma.is_empty() { return vec![0.0, 0.0, 0.0, 0.0]; }

    let offset = fast_sma.len() - slow_sma.len();
    let mut in_position = false;
    let mut is_long = true;
    let mut entry_price = 0.0;
    let mut total_pnl = 0.0;
    let mut gross_profit = 0.0;
    let mut gross_loss = 0.0;
    let mut wins = 0u32;
    let mut trades = 0u32;

    for i in 1..slow_sma.len() {
        let fast_now = fast_sma[i + offset];
        let fast_prev = fast_sma[i + offset - 1];
        let slow_now = slow_sma[i];
        let slow_prev = slow_sma[i - 1];
        let bar_idx = i + slow_period - 1;
        let price = bar_close(data, bar_idx);

        // Crossover: buy
        if fast_prev <= slow_prev && fast_now > slow_now {
            if in_position && !is_long {
                let pnl = (entry_price - price) * (equity / entry_price);
                total_pnl += pnl;
                if pnl > 0.0 { gross_profit += pnl; wins += 1; } else { gross_loss -= pnl; }
                trades += 1;
            }
            in_position = true;
            is_long = true;
            entry_price = price;
        }
        // Crossunder: sell
        if fast_prev >= slow_prev && fast_now < slow_now {
            if in_position && is_long {
                let pnl = (price - entry_price) * (equity / entry_price);
                total_pnl += pnl;
                if pnl > 0.0 { gross_profit += pnl; wins += 1; } else { gross_loss -= pnl; }
                trades += 1;
            }
            in_position = true;
            is_long = false;
            entry_price = price;
        }
    }

    let win_rate = if trades > 0 { wins as f64 / trades as f64 * 100.0 } else { 0.0 };
    let profit_factor = if gross_loss > 0.0 { gross_profit / gross_loss } else { 0.0 };
    vec![total_pnl, win_rate, profit_factor, trades as f64]
}

/// Grid-search SMA optimization. Returns flat array: [fast, slow, pnl, win_rate, pf, trades, ...].
/// 6 values per result, sorted by profit factor descending.
#[wasm_bindgen]
pub fn wasm_optimize_sma(data: &[f64], fast_min: usize, fast_max: usize, slow_min: usize, slow_max: usize, equity: f64, top_n: usize) -> Vec<f64> {
    let mut results: Vec<(f64, [f64; 6])> = Vec::new();
    for fast in fast_min..=fast_max {
        for slow in slow_min..=slow_max {
            if fast >= slow { continue; }
            let r = wasm_backtest_sma(data, fast, slow, equity);
            if r[3] > 0.0 { // has trades
                results.push((r[2], [fast as f64, slow as f64, r[0], r[1], r[2], r[3]]));
            }
        }
    }
    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(top_n);
    let mut flat = Vec::with_capacity(results.len() * 6);
    for (_, r) in results { flat.extend_from_slice(&r); }
    flat
}
