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
    if n < period { return vec![]; }
    let sma_seed: f64 = (0..period).map(|i| bar_close(data, i)).sum::<f64>() / period as f64;
    let mut ema = sma_seed;
    let mut result = Vec::with_capacity(n - period + 1);
    // First output value IS the SMA seed (at index period-1)
    result.push(ema);
    for i in period..n {
        ema = bar_close(data, i) * k + ema * (1.0 - k);
        result.push(ema);
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
    if n < period + 1 || period == 0 { return vec![]; }
    let mut result = Vec::with_capacity(n - period);
    let mut prev_smoothed = 0.0;
    let mut prev_fisher = 0.0;
    for i in period..n {
        let mut max_h = f64::NEG_INFINITY;
        let mut min_l = f64::MAX;
        for j in (i + 1 - period)..=i {
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
    let mut upper = Vec::with_capacity(count);
    let mut lower = Vec::with_capacity(count);
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
        upper.push(mean + 2.0 * std);
        lower.push(mean - 2.0 * std);
    }
    let mut result = Vec::with_capacity(count * 2);
    result.extend(upper);
    result.extend(lower);
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

// ── BetterVolume ───────────────────────────────────────────────

/// Estimate buy/sell volume for a bar.
fn estimate_buy_sell(o: f64, h: f64, l: f64, c: f64, vol: f64) -> (f64, f64) {
    let range = h - l;
    if range <= 0.0 { return (vol * 0.5, vol * 0.5); }
    let buy = if c > o {
        let denom = 2.0 * range + o - c;
        (range / if denom > 0.0 { denom } else { range }) * vol
    } else if c < o {
        let denom = 2.0 * range + c - o;
        ((range + c - o) / if denom > 0.0 { denom } else { range }) * vol
    } else {
        vol * 0.5
    };
    (buy, vol - buy)
}

/// BetterVolume classification. Returns flat array: [color_index_0, color_index_1, ...].
/// Color indices: 0=LowVol(yellow), 1=ClimaxUp(red), 2=ClimaxDn(white), 3=Churn(green), 4=ClimaxChurn(magenta), 5=Normal(steelblue)
#[wasm_bindgen]
pub fn wasm_better_volume(data: &[f64], lookback: usize) -> Vec<f64> {
    let n = bar_count(data);
    if n < lookback + 2 { return vec![]; }
    let mut result = Vec::with_capacity(n);
    // Pad first lookback+1 bars with Normal
    for _ in 0..(lookback + 1).min(n) { result.push(5.0); }

    for pos in (lookback + 1)..n {
        let o = bar_open(data, pos);
        let h = bar_high(data, pos);
        let l = bar_low(data, pos);
        let c = bar_close(data, pos);
        let vol = data[pos * FIELDS_PER_BAR + 4];
        let range = (h - l).max(1e-10);
        let (buy_vol, sell_vol) = estimate_buy_sell(o, h, l, c, vol);
        let buy_range = buy_vol * range;
        let sell_range = sell_vol * range;
        let vol_div_r = vol / range;
        let sell_div_r = sell_vol / range;
        let buy_div_r = buy_vol / range;

        let mut high_buy_range = 0.0_f64;
        let mut high_sell_range = 0.0_f64;
        let mut high_vol_div_r = 0.0_f64;
        let mut low_sell_div_r = f64::MAX;
        let mut low_buy_div_r = f64::MAX;
        let mut low_total_vol = f64::MAX;

        for i in 0..lookback {
            let b = pos - 1 - i;
            if b >= n { break; }
            let bo = bar_open(data, b);
            let bh = bar_high(data, b);
            let bl = bar_low(data, b);
            let bc = bar_close(data, b);
            let bv = data[b * FIELDS_PER_BAR + 4];
            let br = (bh - bl).max(1e-10);
            let (bbv, bsv) = estimate_buy_sell(bo, bh, bl, bc, bv);
            if bbv * br > high_buy_range { high_buy_range = bbv * br; }
            if bsv * br > high_sell_range { high_sell_range = bsv * br; }
            if bv / br > high_vol_div_r { high_vol_div_r = bv / br; }
            if bsv / br < low_sell_div_r { low_sell_div_r = bsv / br; }
            if bbv / br < low_buy_div_r { low_buy_div_r = bbv / br; }
            if bv < low_total_vol { low_total_vol = bv; }
        }

        let is_low_vol = vol <= low_total_vol;
        let is_climax_up = c > o && (buy_range >= high_buy_range || sell_div_r <= low_sell_div_r);
        let is_climax_dn = c < o && (sell_range >= high_sell_range || buy_div_r <= low_buy_div_r);
        let is_churn = vol_div_r >= high_vol_div_r;

        let color = if (is_climax_up || is_climax_dn) && is_churn { 4.0 }      // ClimaxChurn
            else if is_low_vol { 0.0 }                                           // LowVol
            else if is_climax_up { 1.0 }                                         // ClimaxUp
            else if is_climax_dn { 2.0 }                                         // ClimaxDn
            else if is_churn { 3.0 }                                             // Churn
            else { 5.0 };                                                        // Normal
        result.push(color);
    }
    result
}

// ── Supply/Demand Zones ────────────────────────────────────────

/// Detect supply/demand zones using fractal swing detection.
/// Returns flat array: [startIdx, high, low, type, strength, ...] per zone.
/// type: 0=demand(support), 1=supply(resistance). strength: 0=untested, 1=tested, 2=proven.
#[wasm_bindgen]
pub fn wasm_supply_demand(data: &[f64], fractal_lookback: usize) -> Vec<f64> {
    let n = bar_count(data);
    if n < fractal_lookback * 2 + 5 { return vec![]; }

    let mut zones: Vec<(usize, f64, f64, f64, f64)> = Vec::new(); // (startIdx, high, low, type, strength)

    // Find fractal swing highs and lows
    for i in fractal_lookback..(n - fractal_lookback) {
        let mut is_high = true;
        let mut is_low = true;
        for j in 1..=fractal_lookback {
            if bar_high(data, i.saturating_sub(j)) >= bar_high(data, i) || bar_high(data, (i + j).min(n - 1)) >= bar_high(data, i) { is_high = false; }
            if bar_low(data, i.saturating_sub(j)) <= bar_low(data, i) || bar_low(data, (i + j).min(n - 1)) <= bar_low(data, i) { is_low = false; }
        }
        if is_high {
            // Supply zone: from body top to high
            let body_top = bar_open(data, i).max(bar_close(data, i));
            zones.push((i, bar_high(data, i), body_top, 1.0, 0.0));
        }
        if is_low {
            // Demand zone: from low to body bottom
            let body_bot = bar_open(data, i).min(bar_close(data, i));
            zones.push((i, body_bot, bar_low(data, i), 0.0, 0.0));
        }
    }

    // Test zones: count how many times price returned to the zone
    // Early-exit after 3 touches (proven) since higher counts don't change the result.
    for zone in &mut zones {
        let mut touches = 0u32;
        for k in (zone.0 + 1)..n {
            let h = bar_high(data, k);
            let l = bar_low(data, k);
            if l <= zone.1 && h >= zone.2 {
                touches += 1;
                if touches >= 3 { break; }
            }
        }
        zone.4 = if touches >= 3 { 2.0 } else if touches >= 1 { 1.0 } else { 0.0 };
    }

    // Flatten
    let mut result = Vec::with_capacity(zones.len() * 5);
    for z in &zones {
        result.push(z.0 as f64);
        result.push(z.1);
        result.push(z.2);
        result.push(z.3);
        result.push(z.4);
    }
    result
}

// ── DEMA ───────────────────────────────────────────────────────

#[wasm_bindgen]
pub fn wasm_dema(data: &[f64], period: usize) -> Vec<f64> {
    let ema1 = wasm_ema(data, period);
    if ema1.len() < period { return vec![]; }
    // Build pseudo-data from ema1 values for second EMA pass
    let mut ema1_data = Vec::with_capacity(ema1.len() * FIELDS_PER_BAR);
    for &v in &ema1 {
        ema1_data.push(v); // open
        ema1_data.push(v); // high
        ema1_data.push(v); // low
        ema1_data.push(v); // close
        ema1_data.push(0.0); // vol
    }
    let ema2 = wasm_ema(&ema1_data, period);
    let offset = ema1.len() - ema2.len();
    ema2.iter().enumerate().map(|(i, &v)| 2.0 * ema1[i + offset] - v).collect()
}

// ── Stochastic ─────────────────────────────────────────────────

/// Returns [k_0, d_0, k_1, d_1, ...] interleaved.
#[wasm_bindgen]
pub fn wasm_stochastic(data: &[f64], k_period: usize, d_period: usize, slowing: usize) -> Vec<f64> {
    let n = bar_count(data);
    if n < k_period + slowing + d_period { return vec![]; }
    // Raw %K
    let mut raw_k = Vec::with_capacity(n);
    for i in (k_period - 1)..n {
        let mut hh = f64::NEG_INFINITY;
        let mut ll = f64::MAX;
        for j in (i + 1 - k_period)..=i {
            if bar_high(data, j) > hh { hh = bar_high(data, j); }
            if bar_low(data, j) < ll { ll = bar_low(data, j); }
        }
        let range = hh - ll;
        raw_k.push(if range > 0.0 { (bar_close(data, i) - ll) / range * 100.0 } else { 50.0 });
    }
    // Slow %K (SMA of raw %K)
    if raw_k.len() < slowing { return vec![]; }
    let mut slow_k = Vec::with_capacity(raw_k.len());
    for i in (slowing - 1)..raw_k.len() {
        let sum: f64 = raw_k[(i + 1 - slowing)..=i].iter().sum();
        slow_k.push(sum / slowing as f64);
    }
    // %D (SMA of slow %K)
    if slow_k.len() < d_period { return vec![]; }
    let mut result = Vec::new();
    let mut d_sum = 0.0;
    for i in 0..slow_k.len() {
        d_sum += slow_k[i];
        if i >= d_period { d_sum -= slow_k[i - d_period]; }
        if i >= d_period - 1 {
            result.push(slow_k[i]);
            result.push(d_sum / d_period as f64);
        }
    }
    result
}

// ── CCI (Commodity Channel Index) ──────────────────────────────

#[wasm_bindgen]
pub fn wasm_cci(data: &[f64], period: usize) -> Vec<f64> {
    let n = bar_count(data);
    if period == 0 || n < period { return vec![]; }
    let mut result = Vec::with_capacity(n - period + 1);
    let mut tps = Vec::with_capacity(period);
    for i in (period - 1)..n {
        let mut sum = 0.0;
        tps.clear();
        for j in (i + 1 - period)..=i {
            let tp = (bar_high(data, j) + bar_low(data, j) + bar_close(data, j)) / 3.0;
            tps.push(tp);
            sum += tp;
        }
        let mean = sum / period as f64;
        let mad: f64 = tps.iter().map(|&t| (t - mean).abs()).sum::<f64>() / period as f64;
        result.push(if mad > 0.0 { (tps.last().unwrap() - mean) / (0.015 * mad) } else { 0.0 });
    }
    result
}

// ── Williams %R ────────────────────────────────────────────────

#[wasm_bindgen]
pub fn wasm_williams_r(data: &[f64], period: usize) -> Vec<f64> {
    let n = bar_count(data);
    if n < period { return vec![]; }
    let mut result = Vec::with_capacity(n - period + 1);
    for i in (period - 1)..n {
        let mut hh = f64::NEG_INFINITY;
        let mut ll = f64::MAX;
        for j in (i + 1 - period)..=i {
            if bar_high(data, j) > hh { hh = bar_high(data, j); }
            if bar_low(data, j) < ll { ll = bar_low(data, j); }
        }
        let range = hh - ll;
        result.push(if range > 0.0 { (hh - bar_close(data, i)) / range * -100.0 } else { -50.0 });
    }
    result
}

// ── ADX (Average Directional Index) ────────────────────────────

/// Returns [adx_0, plus_di_0, minus_di_0, adx_1, plus_di_1, minus_di_1, ...].
#[wasm_bindgen]
pub fn wasm_adx(data: &[f64], period: usize) -> Vec<f64> {
    let n = bar_count(data);
    if n < period * 3 { return vec![]; }
    let mut plus_dm = Vec::with_capacity(n);
    let mut minus_dm = Vec::with_capacity(n);
    let mut tr_vals = Vec::with_capacity(n);
    for i in 1..n {
        let h = bar_high(data, i);
        let l = bar_low(data, i);
        let ph = bar_high(data, i - 1);
        let pl = bar_low(data, i - 1);
        let pc = bar_close(data, i - 1);
        let up = h - ph;
        let dn = pl - l;
        plus_dm.push(if up > dn && up > 0.0 { up } else { 0.0 });
        minus_dm.push(if dn > up && dn > 0.0 { dn } else { 0.0 });
        tr_vals.push((h - l).max((h - pc).abs()).max((l - pc).abs()));
    }
    if tr_vals.len() < period { return vec![]; }
    // Smoothed TR, +DM, -DM
    let mut atr = tr_vals[..period].iter().sum::<f64>();
    let mut a_pdm = plus_dm[..period].iter().sum::<f64>();
    let mut a_mdm = minus_dm[..period].iter().sum::<f64>();
    let mut dx_vals = Vec::new();
    let mut result = Vec::new();
    for i in period..tr_vals.len() {
        atr = atr - atr / period as f64 + tr_vals[i];
        a_pdm = a_pdm - a_pdm / period as f64 + plus_dm[i];
        a_mdm = a_mdm - a_mdm / period as f64 + minus_dm[i];
        let pdi = if atr > 0.0 { a_pdm / atr * 100.0 } else { 0.0 };
        let mdi = if atr > 0.0 { a_mdm / atr * 100.0 } else { 0.0 };
        let sum = pdi + mdi;
        let dx = if sum > 0.0 { (pdi - mdi).abs() / sum * 100.0 } else { 0.0 };
        dx_vals.push((dx, pdi, mdi));
    }
    if dx_vals.len() < period { return vec![]; }
    let mut adx = dx_vals[..period].iter().map(|d| d.0).sum::<f64>() / period as f64;
    for i in period..dx_vals.len() {
        adx = (adx * (period as f64 - 1.0) + dx_vals[i].0) / period as f64;
        result.push(adx);
        result.push(dx_vals[i].1);
        result.push(dx_vals[i].2);
    }
    result
}

// ── OBV (On-Balance Volume) ────────────────────────────────────

#[wasm_bindgen]
pub fn wasm_obv(data: &[f64]) -> Vec<f64> {
    let n = bar_count(data);
    if n < 2 { return vec![]; }
    let mut result = Vec::with_capacity(n);
    let mut obv = 0.0;
    result.push(obv);
    for i in 1..n {
        let c = bar_close(data, i);
        let pc = bar_close(data, i - 1);
        let vol = data[i * FIELDS_PER_BAR + 4];
        if c > pc { obv += vol; } else if c < pc { obv -= vol; }
        result.push(obv);
    }
    result
}

// ── Momentum ───────────────────────────────────────────────────

#[wasm_bindgen]
pub fn wasm_momentum(data: &[f64], period: usize) -> Vec<f64> {
    let n = bar_count(data);
    if n <= period { return vec![]; }
    (period..n).map(|i| bar_close(data, i) - bar_close(data, i - period)).collect()
}

// ── WMA (Weighted Moving Average) ──────────────────────────────

#[wasm_bindgen]
pub fn wasm_wma(data: &[f64], period: usize) -> Vec<f64> {
    let n = bar_count(data);
    if n < period { return vec![]; }
    let denom = (period * (period + 1)) as f64 / 2.0;
    let mut result = Vec::with_capacity(n - period + 1);
    for i in (period - 1)..n {
        let mut sum = 0.0;
        for j in 0..period {
            sum += bar_close(data, i - period + 1 + j) * (j + 1) as f64;
        }
        result.push(sum / denom);
    }
    result
}

// ── HMA (Hull Moving Average) ──────────────────────────────────

#[wasm_bindgen]
pub fn wasm_hma(data: &[f64], period: usize) -> Vec<f64> {
    let half = period / 2;
    let sqrt_p = (period as f64).sqrt().round() as usize;
    let wma_half = wasm_wma(data, half.max(1));
    let wma_full = wasm_wma(data, period);
    if wma_half.is_empty() || wma_full.is_empty() { return vec![]; }
    let offset = wma_half.len() - wma_full.len();
    // Diff series: 2 * WMA(half) - WMA(full)
    let mut diff_data = Vec::with_capacity(wma_full.len() * FIELDS_PER_BAR);
    for i in 0..wma_full.len() {
        let v = 2.0 * wma_half[i + offset] - wma_full[i];
        diff_data.push(v); diff_data.push(v); diff_data.push(v); diff_data.push(v); diff_data.push(0.0);
    }
    wasm_wma(&diff_data, sqrt_p.max(1))
}
