use super::*;

pub(super) fn compute_sma(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < period {
        return out;
    }
    let mut sum: f64 = bars[..period].iter().map(|b| b.close).sum();
    out[period - 1] = Some(sum / period as f64);
    for i in period..n {
        sum += bars[i].close - bars[i - period].close;
        out[i] = Some(sum / period as f64);
    }
    out
}

pub(super) fn compute_ema(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < period {
        return out;
    }
    let k = 2.0 / (period as f64 + 1.0);
    // Seed with SMA
    let seed: f64 = bars[..period].iter().map(|b| b.close).sum::<f64>() / period as f64;
    out[period - 1] = Some(seed);
    let mut ema = seed;
    for i in period..n {
        ema = bars[i].close * k + ema * (1.0 - k);
        out[i] = Some(ema);
    }
    out
}

/// Kaufman Adaptive Moving Average — O(n) with rolling volatility sum.
pub(super) fn compute_kama(
    bars: &[Bar],
    er_period: usize,
    fast: usize,
    slow: usize,
) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n <= er_period {
        return out;
    }

    let fast_sc = 2.0 / (fast as f64 + 1.0);
    let slow_sc = 2.0 / (slow as f64 + 1.0);

    let mut kama = bars[er_period].close;
    out[er_period] = Some(kama);

    // Pre-compute absolute price changes
    let mut changes = vec![0.0_f64; n];
    for i in 1..n {
        changes[i] = (bars[i].close - bars[i - 1].close).abs();
    }

    // Initial volatility sum for first KAMA bar
    let mut vol_sum: f64 = changes[1..=er_period].iter().sum();

    for i in (er_period + 1)..n {
        // Rolling volatility: add newest change, remove oldest
        vol_sum += changes[i] - changes[i - er_period];
        let direction = (bars[i].close - bars[i - er_period].close).abs();
        let er = if vol_sum < f64::EPSILON {
            0.0
        } else {
            (direction / vol_sum).clamp(0.0, 1.0)
        };
        let sc = (er * (fast_sc - slow_sc) + slow_sc).powi(2);
        kama += sc * (bars[i].close - kama);
        out[i] = Some(kama);
    }
    out
}

pub(super) fn compute_bollinger(
    bars: &[Bar],
    period: usize,
    mult: f64,
) -> (Vec<Option<f64>>, Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = bars.len();
    let mut mid = vec![None; n];
    let mut upper = vec![None; n];
    let mut lower = vec![None; n];
    if n < period {
        return (mid, upper, lower);
    }

    // Rolling sum/sum_sq — O(n) instead of O(n×period)
    let pf = period as f64;
    let mut sum: f64 = bars[..period].iter().map(|b| b.close).sum();
    let mut sum_sq: f64 = bars[..period].iter().map(|b| b.close * b.close).sum();

    let mean = sum / pf;
    let variance = (sum_sq / pf - mean * mean).max(0.0);
    let std_dev = variance.sqrt();
    mid[period - 1] = Some(mean);
    upper[period - 1] = Some(mean + mult * std_dev);
    lower[period - 1] = Some(mean - mult * std_dev);

    for i in period..n {
        let old = bars[i - period].close;
        let new = bars[i].close;
        sum += new - old;
        sum_sq += new * new - old * old;
        let mean = sum / pf;
        let variance = (sum_sq / pf - mean * mean).max(0.0);
        let std_dev = variance.sqrt();
        mid[i] = Some(mean);
        upper[i] = Some(mean + mult * std_dev);
        lower[i] = Some(mean - mult * std_dev);
    }
    (mid, upper, lower)
}

/// VWAP with standard deviation bands, anchored at each new UTC day.
pub(super) fn compute_vwap(
    bars: &[Bar],
) -> (
    Vec<Option<f64>>,
    Vec<Option<f64>>,
    Vec<Option<f64>>,
    Vec<Option<f64>>,
    Vec<Option<f64>>,
    Vec<Option<f64>>,
    Vec<Option<f64>>,
) {
    let n = bars.len();
    let mut vwap = vec![None; n];
    let mut u1 = vec![None; n];
    let mut u2 = vec![None; n];
    let mut u3 = vec![None; n];
    let mut l1 = vec![None; n];
    let mut l2 = vec![None; n];
    let mut l3 = vec![None; n];
    if n == 0 {
        return (vwap, u1, u2, u3, l1, l2, l3);
    }

    let mut cum_vol = 0.0_f64;
    let mut cum_tp_vol = 0.0_f64;
    let mut cum_tp2_vol = 0.0_f64;
    let mut prev_day = -1i64;

    for i in 0..n {
        let bar = &bars[i];
        let day = bar.ts_ms / 1000 / 86400;
        // Reset on new day
        if day != prev_day {
            cum_vol = 0.0;
            cum_tp_vol = 0.0;
            cum_tp2_vol = 0.0;
            prev_day = day;
        }

        let tp = (bar.high + bar.low + bar.close) / 3.0;
        let vol = bar.volume.max(1.0); // avoid div-by-zero for zero-volume bars
        cum_vol += vol;
        cum_tp_vol += tp * vol;
        cum_tp2_vol += tp * tp * vol;

        let vw = cum_tp_vol / cum_vol;
        let variance = (cum_tp2_vol / cum_vol - vw * vw).max(0.0);
        let sd = variance.sqrt();

        vwap[i] = Some(vw);
        u1[i] = Some(vw + sd);
        u2[i] = Some(vw + 2.0 * sd);
        u3[i] = Some(vw + 3.0 * sd);
        l1[i] = Some(vw - sd);
        l2[i] = Some(vw - 2.0 * sd);
        l3[i] = Some(vw - 3.0 * sd);
    }
    (vwap, u1, u2, u3, l1, l2, l3)
}

/// Supertrend indicator: ATR-based trend following with direction flip.
pub(super) fn compute_supertrend(
    bars: &[Bar],
    atr: &[Option<f64>],
    period: usize,
    multiplier: f64,
) -> (Vec<Option<f64>>, Vec<bool>) {
    let n = bars.len();
    let mut st = vec![None; n];
    let mut bull = vec![true; n];
    if n <= period {
        return (st, bull);
    }

    let mut upper_band;
    let mut lower_band;
    let mut prev_upper = 0.0_f64;
    let mut prev_lower = 0.0_f64;
    let mut prev_st = 0.0_f64;
    #[allow(unused_assignments)]
    let mut is_bull = true;

    for i in period..n {
        let atr_val = atr[i].unwrap_or(0.0);
        let hl2 = (bars[i].high + bars[i].low) / 2.0;
        let basic_upper = hl2 + multiplier * atr_val;
        let basic_lower = hl2 - multiplier * atr_val;

        // Final bands: use previous final band if it's more favorable
        upper_band = if basic_upper < prev_upper || bars[i - 1].close > prev_upper {
            basic_upper
        } else {
            prev_upper
        };
        lower_band = if basic_lower > prev_lower || bars[i - 1].close < prev_lower {
            basic_lower
        } else {
            prev_lower
        };

        // Direction
        if prev_st == prev_upper {
            is_bull = bars[i].close > upper_band;
        } else {
            is_bull = bars[i].close >= lower_band;
        }

        let val = if is_bull { lower_band } else { upper_band };
        st[i] = Some(val);
        bull[i] = is_bull;
        prev_upper = upper_band;
        prev_lower = lower_band;
        prev_st = val;
    }
    (st, bull)
}

/// Donchian Channels: highest high / lowest low over N bars. O(n) with deques.
pub(super) fn compute_donchian(
    bars: &[Bar],
    period: usize,
) -> (Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = bars.len();
    let mut upper = vec![None; n];
    let mut lower = vec![None; n];
    if n < period {
        return (upper, lower);
    }

    let mut max_dq: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    let mut min_dq: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    for i in 0..n {
        while max_dq
            .back()
            .map_or(false, |&j| bars[j].high <= bars[i].high)
        {
            max_dq.pop_back();
        }
        max_dq.push_back(i);
        while max_dq.front().map_or(false, |&j| j + period <= i) {
            max_dq.pop_front();
        }
        while min_dq.back().map_or(false, |&j| bars[j].low >= bars[i].low) {
            min_dq.pop_back();
        }
        min_dq.push_back(i);
        while min_dq.front().map_or(false, |&j| j + period <= i) {
            min_dq.pop_front();
        }
        if i >= period - 1 {
            upper[i] = Some(bars[*max_dq.front().unwrap_or(&0)].high);
            lower[i] = Some(bars[*min_dq.front().unwrap_or(&0)].low);
        }
    }
    (upper, lower)
}

/// Keltner Channels: EMA(20) ± ATR(10) × multiplier.
pub(super) fn compute_keltner(
    bars: &[Bar],
    ema_period: usize,
    atr_period: usize,
    mult: f64,
) -> (Vec<Option<f64>>, Vec<Option<f64>>, Vec<Option<f64>>) {
    let ema = compute_ema(bars, ema_period);
    let atr = compute_atr(bars, atr_period);
    let n = bars.len();
    let mut mid = vec![None; n];
    let mut upper = vec![None; n];
    let mut lower = vec![None; n];
    for i in 0..n {
        if let (Some(e), Some(a)) = (ema[i], atr[i]) {
            mid[i] = Some(e);
            upper[i] = Some(e + mult * a);
            lower[i] = Some(e - mult * a);
        }
    }
    (mid, upper, lower)
}

/// Linear Regression Channel: rolling regression ± 2σ standard error bands.
/// Linear Regression Channel — O(n) with rolling sums for Σy, Σxy, Σy².
pub(super) fn compute_regression_channel(
    bars: &[Bar],
    period: usize,
) -> (Vec<Option<f64>>, Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = bars.len();
    let mut mid = vec![None; n];
    let mut upper = vec![None; n];
    let mut lower = vec![None; n];
    if n < period {
        return (mid, upper, lower);
    }
    let pf = period as f64;

    // Constants: Σx, Σx² for x = 0..period-1
    let sx: f64 = (0..period).map(|x| x as f64).sum();
    let sx2: f64 = (0..period).map(|x| (x * x) as f64).sum();
    let denom = pf * sx2 - sx * sx;
    if denom.abs() < f64::EPSILON {
        return (mid, upper, lower);
    }

    // Initial sums for first window
    let mut sy: f64 = 0.0;
    let mut sxy: f64 = 0.0;
    let mut sy2: f64 = 0.0;
    for k in 0..period {
        let y = bars[k].close;
        sy += y;
        sxy += k as f64 * y;
        sy2 += y * y;
    }

    for i in (period - 1)..n {
        if i > period - 1 {
            // Rolling update: window slides right by 1
            let old_y = bars[i - period].close;
            let new_y = bars[i].close;
            // When window slides: each existing term's x decreases by 1
            // sxy_new = sxy_old - (sy_old - old_y) + (period-1) * new_y
            sxy = sxy - sy + old_y + (period - 1) as f64 * new_y;
            sy = sy - old_y + new_y;
            sy2 = sy2 - old_y * old_y + new_y * new_y;
        }

        let slope = (pf * sxy - sx * sy) / denom;
        let intercept = (sy - slope * sx) / pf;
        let reg_val = intercept + slope * (period - 1) as f64;

        // SSE from algebra: Σ(y-ŷ)² = Σy² - b0*Σy - b1*Σxy
        let sse = (sy2 - intercept * sy - slope * sxy).max(0.0);
        let se = (sse / (pf - 2.0).max(1.0)).sqrt();

        mid[i] = Some(reg_val);
        upper[i] = Some(reg_val + 2.0 * se);
        lower[i] = Some(reg_val - 2.0 * se);
    }
    (mid, upper, lower)
}

/// Squeeze Momentum: detect BB inside KC (squeeze), momentum from linear regression.
pub(super) fn compute_squeeze_momentum(
    bb_upper: &[Option<f64>],
    bb_lower: &[Option<f64>],
    kc_upper: &[Option<f64>],
    kc_lower: &[Option<f64>],
    bars: &[Bar],
    period: usize,
) -> (Vec<Option<f64>>, Vec<bool>) {
    let n = bars.len();
    let mut mom = vec![None; n];
    let mut squeeze = vec![false; n];

    // Squeeze: BB inside KC
    for i in 0..n {
        if let (Some(bbu), Some(bbl), Some(kcu), Some(kcl)) = (
            bb_upper.get(i).and_then(|v| *v),
            bb_lower.get(i).and_then(|v| *v),
            kc_upper.get(i).and_then(|v| *v),
            kc_lower.get(i).and_then(|v| *v),
        ) {
            squeeze[i] = bbu < kcu && bbl > kcl;
        }
    }

    // Momentum: linear regression of (close - HL2), O(n) rolling sums
    if n >= period {
        let pf = period as f64;
        let sx: f64 = (0..period).map(|x| x as f64).sum();
        let sx2: f64 = (0..period).map(|x| (x * x) as f64).sum();
        let denom = pf * sx2 - sx * sx;
        if denom.abs() > f64::EPSILON {
            // Pre-compute deviation values
            let vals: Vec<f64> = bars
                .iter()
                .map(|b| b.close - (b.high + b.low) / 2.0)
                .collect();

            let mut sy: f64 = vals[..period].iter().sum();
            let mut sxy: f64 = vals[..period]
                .iter()
                .enumerate()
                .map(|(k, &v)| k as f64 * v)
                .sum();

            for i in (period - 1)..n {
                if i > period - 1 {
                    let old = vals[i - period];
                    let new = vals[i];
                    sxy = sxy - sy + old + (period - 1) as f64 * new;
                    sy = sy - old + new;
                }
                let slope = (pf * sxy - sx * sy) / denom;
                let intercept = (sy - slope * sx) / pf;
                mom[i] = Some(intercept + slope * (period - 1) as f64);
            }
        }
    }
    (mom, squeeze)
}

pub(super) fn compute_rsi(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n <= period {
        return out;
    }

    let mut avg_gain = 0.0_f64;
    let mut avg_loss = 0.0_f64;

    // Initial averages
    for i in 1..=period {
        let delta = bars[i].close - bars[i - 1].close;
        if delta > 0.0 {
            avg_gain += delta;
        } else {
            avg_loss -= delta;
        }
    }
    avg_gain /= period as f64;
    avg_loss /= period as f64;

    let rs = if avg_loss < f64::EPSILON {
        100.0
    } else {
        avg_gain / avg_loss
    };
    out[period] = Some(100.0 - 100.0 / (1.0 + rs));

    for i in (period + 1)..n {
        let delta = bars[i].close - bars[i - 1].close;
        let (gain, loss) = if delta > 0.0 {
            (delta, 0.0)
        } else {
            (0.0, -delta)
        };
        avg_gain = (avg_gain * (period as f64 - 1.0) + gain) / period as f64;
        avg_loss = (avg_loss * (period as f64 - 1.0) + loss) / period as f64;
        let rs = if avg_loss < f64::EPSILON {
            100.0
        } else {
            avg_gain / avg_loss
        };
        out[i] = Some(100.0 - 100.0 / (1.0 + rs));
    }
    out
}

/// Ehlers Fisher Transform — matches EhlersFisherTransform.mqh exactly.
/// Smoothing: 0.5 * oscillator + 0.5 * prev_work (NOT 0.33/0.67)
/// Fisher: 0.25 * ln((1+w)/(1-w)) + 0.5 * prev_fisher (recursive)
/// Ehlers Fisher Transform — O(n) with monotonic deque for sliding min/max.
pub(super) fn compute_fisher(bars: &[Bar], period: usize) -> (Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = bars.len();
    let mut fisher = vec![None; n];
    let mut signal = vec![None; n];
    if n <= period {
        return (fisher, signal);
    }

    let mut work = 0.0_f64;
    let mut prev_fisher = 0.0_f64;

    // Monotonic deques for O(1) amortized sliding window max/min
    let mut max_dq: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    let mut min_dq: std::collections::VecDeque<usize> = std::collections::VecDeque::new();

    for i in 0..n {
        // Maintain max deque (bar.high)
        while max_dq
            .back()
            .map_or(false, |&j| bars[j].high <= bars[i].high)
        {
            max_dq.pop_back();
        }
        max_dq.push_back(i);
        while max_dq.front().map_or(false, |&j| j + period <= i) {
            max_dq.pop_front();
        }

        // Maintain min deque (bar.low)
        while min_dq.back().map_or(false, |&j| bars[j].low >= bars[i].low) {
            min_dq.pop_back();
        }
        min_dq.push_back(i);
        while min_dq.front().map_or(false, |&j| j + period <= i) {
            min_dq.pop_front();
        }

        if i < period {
            continue;
        }

        let hi = bars[*max_dq.front().unwrap_or(&0)].high;
        let lo = bars[*min_dq.front().unwrap_or(&0)].low;
        let mid = (bars[i].high + bars[i].low) / 2.0;

        let range = hi - lo;
        let raw = if range < f64::EPSILON {
            0.0
        } else {
            2.0 * ((mid - lo) / range - 0.5)
        };
        work = (0.5 * raw + 0.5 * work).clamp(-0.999, 0.999);
        let f = 0.25 * ((1.0 + work) / (1.0 - work)).ln() + 0.5 * prev_fisher;

        signal[i] = Some(prev_fisher);
        fisher[i] = Some(f);
        prev_fisher = f;
    }
    (fisher, signal)
}

pub(super) fn compute_atr(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n <= period {
        return out;
    }

    let mut sum = 0.0_f64;
    for i in 1..=period {
        let tr = true_range(&bars[i], &bars[i - 1]);
        sum += tr;
    }
    let mut atr = sum / period as f64;
    out[period] = Some(atr);

    for i in (period + 1)..n {
        let tr = true_range(&bars[i], &bars[i - 1]);
        atr = (atr * (period as f64 - 1.0) + tr) / period as f64;
        out[i] = Some(atr);
    }
    out
}

pub(super) fn true_range(bar: &Bar, prev: &Bar) -> f64 {
    let hl = bar.high - bar.low;
    let hc = (bar.high - prev.close).abs();
    let lc = (bar.low - prev.close).abs();
    hl.max(hc).max(lc)
}

pub(super) fn compute_macd(
    bars: &[Bar],
    fast: usize,
    slow: usize,
    signal_period: usize,
) -> (Vec<Option<f64>>, Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = bars.len();
    let ema_fast = compute_ema(bars, fast);
    let ema_slow = compute_ema(bars, slow);

    let mut macd_line = vec![None; n];
    for i in 0..n {
        if let (Some(f), Some(s)) = (ema_fast[i], ema_slow[i]) {
            macd_line[i] = Some(f - s);
        }
    }

    // Signal line = EMA of MACD line
    let mut signal = vec![None; n];
    let mut hist = vec![None; n];
    let k = 2.0 / (signal_period as f64 + 1.0);

    // Find first valid MACD value to seed signal
    let first_valid = macd_line.iter().position(|v| v.is_some());
    if let Some(start) = first_valid {
        // Seed with SMA of first signal_period MACD values
        let mut count = 0;
        let mut sum = 0.0;
        let mut seed_idx = start;
        for i in start..n {
            if let Some(v) = macd_line[i] {
                sum += v;
                count += 1;
                if count == signal_period {
                    seed_idx = i;
                    break;
                }
            }
        }
        if count == signal_period {
            let mut sig = sum / signal_period as f64;
            signal[seed_idx] = Some(sig);
            hist[seed_idx] = macd_line[seed_idx].map(|m| m - sig);

            for i in (seed_idx + 1)..n {
                if let Some(m) = macd_line[i] {
                    sig = m * k + sig * (1.0 - k);
                    signal[i] = Some(sig);
                    hist[i] = Some(m - sig);
                }
            }
        }
    }
    (macd_line, signal, hist)
}

/// Convert regular bars to Heikin-Ashi bars for rendering.
pub(super) fn heikin_ashi(bars: &[Bar]) -> Vec<Bar> {
    if bars.is_empty() {
        return Vec::new();
    }
    let mut ha = Vec::with_capacity(bars.len());

    let first = &bars[0];
    let ha_close = (first.open + first.high + first.low + first.close) / 4.0;
    let ha_open = (first.open + first.close) / 2.0;
    ha.push(Bar {
        ts_ms: first.ts_ms,
        open: ha_open,
        high: first.high.max(ha_open).max(ha_close),
        low: first.low.min(ha_open).min(ha_close),
        close: ha_close,
        volume: first.volume,
    });

    for i in 1..bars.len() {
        let b = &bars[i];
        let prev = &ha[i - 1];
        let c = (b.open + b.high + b.low + b.close) / 4.0;
        let o = (prev.open + prev.close) / 2.0;
        ha.push(Bar {
            ts_ms: b.ts_ms,
            open: o,
            high: b.high.max(o).max(c),
            low: b.low.min(o).min(c),
            close: c,
            volume: b.volume,
        });
    }
    ha
}

/// Convert bars to Renko bricks. Brick size = ATR(14) of the input data.
pub(super) fn renko_bricks(bars: &[Bar]) -> Vec<Bar> {
    if bars.len() < 15 {
        return bars.to_vec();
    }
    // Compute brick size from ATR(14)
    let atr_vals = compute_atr(bars, 14);
    let brick_size = atr_vals
        .iter()
        .rev()
        .flatten()
        .next()
        .copied()
        .unwrap_or(1.0);
    if brick_size < f64::EPSILON {
        return bars.to_vec();
    }

    let mut bricks: Vec<Bar> = Vec::new();
    let mut current = bars[0].close;

    for bar in bars {
        while bar.close >= current + brick_size {
            let open = current;
            current += brick_size;
            bricks.push(Bar {
                ts_ms: bar.ts_ms,
                open,
                high: current,
                low: open,
                close: current,
                volume: bar.volume,
            });
        }
        while bar.close <= current - brick_size {
            let open = current;
            current -= brick_size;
            bricks.push(Bar {
                ts_ms: bar.ts_ms,
                open,
                high: open,
                low: current,
                close: current,
                volume: bar.volume,
            });
        }
    }
    if bricks.is_empty() {
        bars.to_vec()
    } else {
        bricks
    }
}

// ─── harmonic pattern detection (Scott Carney) ───────────────────────────────

#[derive(Clone, Debug)]
pub(super) struct HarmonicPattern {
    name: &'static str,
    x: (usize, f64), // bar index, price
    a: (usize, f64),
    b: (usize, f64),
    c: (usize, f64),
    d: (usize, f64), // completion / entry point
    tp1: f64,        // target 1 (0.382 AD)
    tp2: f64,        // target 2 (0.618 AD)
    sl: f64,         // stop loss (beyond X)
    bullish: bool,
}

pub(super) fn detect_harmonic_patterns(
    bars: &[Bar],
    fractals_up: &[bool],
    fractals_down: &[bool],
) -> Vec<HarmonicPattern> {
    let n = bars.len();
    if n < 20 {
        return Vec::new();
    }
    let mut patterns: Vec<HarmonicPattern> = Vec::new();

    // Collect swing points from fractals
    let mut swings: Vec<(usize, f64, bool)> = Vec::new(); // (index, price, is_high)
    for i in 0..n {
        if i < fractals_up.len() && fractals_up[i] {
            swings.push((i, bars[i].high, true));
        }
        if i < fractals_down.len() && fractals_down[i] {
            swings.push((i, bars[i].low, false));
        }
    }

    // Need at least 5 swing points for XABCD
    if swings.len() < 5 {
        return patterns;
    }

    // Check the most recent swing combinations (limit to last 12 swings for performance)
    // C(20,5)=15504 vs C(12,5)=792 — 20× fewer pattern checks
    let start = swings.len().saturating_sub(12);
    let recent = &swings[start..];

    for i in 0..recent.len().saturating_sub(4) {
        for j in (i + 1)..recent.len().saturating_sub(3) {
            for k in (j + 1)..recent.len().saturating_sub(2) {
                for l in (k + 1)..recent.len().saturating_sub(1) {
                    for m in (l + 1)..recent.len() {
                        let x = recent[i];
                        let a = recent[j];
                        let b = recent[k];
                        let c = recent[l];
                        let d = recent[m];

                        // Must alternate: high-low-high-low-high or low-high-low-high-low
                        if x.2 == a.2 || a.2 == b.2 || b.2 == c.2 || c.2 == d.2 {
                            continue;
                        }

                        let xa = (a.1 - x.1).abs();
                        if xa < f64::EPSILON {
                            continue;
                        }
                        let ab = (b.1 - a.1).abs();
                        let bc = (c.1 - b.1).abs();
                        let cd = (d.1 - c.1).abs();
                        let xd = (d.1 - x.1).abs();
                        let ad = (d.1 - a.1).abs();

                        let ab_xa = ab / xa;
                        let bc_ab = if ab > f64::EPSILON { bc / ab } else { continue };
                        let _cd_bc = if bc > f64::EPSILON { cd / bc } else { continue };
                        let xd_xa = xd / xa;

                        let bullish = x.1 < a.1; // X is low, A is high = bullish

                        // Gartley: AB=0.618 XA, BC=0.382-0.886 AB, CD=1.27-1.618 BC, XD=0.786 XA
                        if in_range(ab_xa, 0.55, 0.68)
                            && in_range(bc_ab, 0.35, 0.92)
                            && in_range(xd_xa, 0.72, 0.84)
                        {
                            let tp1 = if bullish {
                                d.1 + ad * 0.382
                            } else {
                                d.1 - ad * 0.382
                            };
                            let tp2 = if bullish {
                                d.1 + ad * 0.618
                            } else {
                                d.1 - ad * 0.618
                            };
                            let sl = if bullish {
                                x.1 - xa * 0.1
                            } else {
                                x.1 + xa * 0.1
                            };
                            patterns.push(HarmonicPattern {
                                name: "Gartley",
                                x: (x.0, x.1),
                                a: (a.0, a.1),
                                b: (b.0, b.1),
                                c: (c.0, c.1),
                                d: (d.0, d.1),
                                tp1,
                                tp2,
                                sl,
                                bullish,
                            });
                        }
                        // Butterfly: AB=0.786 XA, BC=0.382-0.886 AB, XD=1.27 XA
                        else if in_range(ab_xa, 0.72, 0.84)
                            && in_range(bc_ab, 0.35, 0.92)
                            && in_range(xd_xa, 1.20, 1.35)
                        {
                            let tp1 = if bullish {
                                d.1 + ad * 0.382
                            } else {
                                d.1 - ad * 0.382
                            };
                            let tp2 = if bullish {
                                d.1 + ad * 0.618
                            } else {
                                d.1 - ad * 0.618
                            };
                            let sl = if bullish {
                                d.1 - xa * 0.15
                            } else {
                                d.1 + xa * 0.15
                            };
                            patterns.push(HarmonicPattern {
                                name: "Butterfly",
                                x: (x.0, x.1),
                                a: (a.0, a.1),
                                b: (b.0, b.1),
                                c: (c.0, c.1),
                                d: (d.0, d.1),
                                tp1,
                                tp2,
                                sl,
                                bullish,
                            });
                        }
                        // Bat: AB=0.382-0.50 XA, BC=0.382-0.886 AB, XD=0.886 XA
                        else if in_range(ab_xa, 0.35, 0.55)
                            && in_range(bc_ab, 0.35, 0.92)
                            && in_range(xd_xa, 0.82, 0.92)
                        {
                            let tp1 = if bullish {
                                d.1 + ad * 0.382
                            } else {
                                d.1 - ad * 0.382
                            };
                            let tp2 = if bullish {
                                d.1 + ad * 0.618
                            } else {
                                d.1 - ad * 0.618
                            };
                            let sl = if bullish {
                                x.1 - xa * 0.1
                            } else {
                                x.1 + xa * 0.1
                            };
                            patterns.push(HarmonicPattern {
                                name: "Bat",
                                x: (x.0, x.1),
                                a: (a.0, a.1),
                                b: (b.0, b.1),
                                c: (c.0, c.1),
                                d: (d.0, d.1),
                                tp1,
                                tp2,
                                sl,
                                bullish,
                            });
                        }
                        // Crab: AB=0.382-0.618 XA, BC=0.382-0.886 AB, XD=1.618 XA
                        else if in_range(ab_xa, 0.35, 0.65)
                            && in_range(bc_ab, 0.35, 0.92)
                            && in_range(xd_xa, 1.55, 1.72)
                        {
                            let tp1 = if bullish {
                                d.1 + ad * 0.382
                            } else {
                                d.1 - ad * 0.382
                            };
                            let tp2 = if bullish {
                                d.1 + ad * 0.618
                            } else {
                                d.1 - ad * 0.618
                            };
                            let sl = if bullish {
                                d.1 - xa * 0.1
                            } else {
                                d.1 + xa * 0.1
                            };
                            patterns.push(HarmonicPattern {
                                name: "Crab",
                                x: (x.0, x.1),
                                a: (a.0, a.1),
                                b: (b.0, b.1),
                                c: (c.0, c.1),
                                d: (d.0, d.1),
                                tp1,
                                tp2,
                                sl,
                                bullish,
                            });
                        }
                        // Shark: AB=1.13-1.618 XA, BC=1.618-2.24 AB, XD=0.886 XA
                        else if in_range(ab_xa, 1.10, 1.65) && in_range(xd_xa, 0.82, 0.92) {
                            let tp1 = if bullish {
                                d.1 + ad * 0.382
                            } else {
                                d.1 - ad * 0.382
                            };
                            let tp2 = if bullish {
                                d.1 + ad * 0.618
                            } else {
                                d.1 - ad * 0.618
                            };
                            let sl = if bullish {
                                x.1 - xa * 0.1
                            } else {
                                x.1 + xa * 0.1
                            };
                            patterns.push(HarmonicPattern {
                                name: "Shark",
                                x: (x.0, x.1),
                                a: (a.0, a.1),
                                b: (b.0, b.1),
                                c: (c.0, c.1),
                                d: (d.0, d.1),
                                tp1,
                                tp2,
                                sl,
                                bullish,
                            });
                        }
                        // Cypher: AB=0.382-0.618 XA, BC=1.13-1.414 AB, XD=0.786 XA
                        else if in_range(ab_xa, 0.35, 0.65)
                            && in_range(bc_ab, 1.10, 1.45)
                            && in_range(xd_xa, 0.72, 0.84)
                        {
                            let tp1 = if bullish {
                                d.1 + ad * 0.382
                            } else {
                                d.1 - ad * 0.382
                            };
                            let tp2 = if bullish {
                                d.1 + ad * 0.618
                            } else {
                                d.1 - ad * 0.618
                            };
                            let sl = if bullish {
                                x.1 - xa * 0.1
                            } else {
                                x.1 + xa * 0.1
                            };
                            patterns.push(HarmonicPattern {
                                name: "Cypher",
                                x: (x.0, x.1),
                                a: (a.0, a.1),
                                b: (b.0, b.1),
                                c: (c.0, c.1),
                                d: (d.0, d.1),
                                tp1,
                                tp2,
                                sl,
                                bullish,
                            });
                        }
                        // Alt Bat (Carney): AB=0.382 XA, BC=0.382-0.886 AB, XD=1.13 XA
                        else if in_range(ab_xa, 0.33, 0.43)
                            && in_range(bc_ab, 0.35, 0.92)
                            && in_range(xd_xa, 1.10, 1.16)
                        {
                            let tp1 = if bullish {
                                d.1 + ad * 0.382
                            } else {
                                d.1 - ad * 0.382
                            };
                            let tp2 = if bullish {
                                d.1 + ad * 0.618
                            } else {
                                d.1 - ad * 0.618
                            };
                            let sl = if bullish {
                                d.1 - xa * 0.15
                            } else {
                                d.1 + xa * 0.15
                            };
                            patterns.push(HarmonicPattern {
                                name: "Alt Bat",
                                x: (x.0, x.1),
                                a: (a.0, a.1),
                                b: (b.0, b.1),
                                c: (c.0, c.1),
                                d: (d.0, d.1),
                                tp1,
                                tp2,
                                sl,
                                bullish,
                            });
                        }
                        // Deep Crab (Carney): AB=0.886 XA, BC=0.382-0.886 AB, XD=1.618 XA
                        else if in_range(ab_xa, 0.84, 0.93)
                            && in_range(bc_ab, 0.35, 0.92)
                            && in_range(xd_xa, 1.55, 1.70)
                        {
                            let tp1 = if bullish {
                                d.1 + ad * 0.382
                            } else {
                                d.1 - ad * 0.382
                            };
                            let tp2 = if bullish {
                                d.1 + ad * 0.618
                            } else {
                                d.1 - ad * 0.618
                            };
                            let sl = if bullish {
                                d.1 - xa * 0.15
                            } else {
                                d.1 + xa * 0.15
                            };
                            patterns.push(HarmonicPattern {
                                name: "Deep Crab",
                                x: (x.0, x.1),
                                a: (a.0, a.1),
                                b: (b.0, b.1),
                                c: (c.0, c.1),
                                d: (d.0, d.1),
                                tp1,
                                tp2,
                                sl,
                                bullish,
                            });
                        }
                        // Three Drives: AB=0.618-0.786 XA, CD=0.618-0.786 BC, BC=1.272-1.618 AB
                        else if in_range(ab_xa, 0.58, 0.82)
                            && in_range(bc_ab, 1.22, 1.66)
                            && in_range(xd_xa, 0.90, 1.10)
                        {
                            // Verify the CD retracement is also in range
                            let cd = (d.1 - c.1).abs();
                            let cd_bc = if bc > f64::EPSILON { cd / bc } else { 0.0 };
                            if in_range(cd_bc, 0.58, 0.82) {
                                let tp1 = if bullish {
                                    d.1 + ad * 0.382
                                } else {
                                    d.1 - ad * 0.382
                                };
                                let tp2 = if bullish {
                                    d.1 + ad * 0.618
                                } else {
                                    d.1 - ad * 0.618
                                };
                                let sl = if bullish {
                                    d.1 - xa * 0.15
                                } else {
                                    d.1 + xa * 0.15
                                };
                                patterns.push(HarmonicPattern {
                                    name: "3 Drives",
                                    x: (x.0, x.1),
                                    a: (a.0, a.1),
                                    b: (b.0, b.1),
                                    c: (c.0, c.1),
                                    d: (d.0, d.1),
                                    tp1,
                                    tp2,
                                    sl,
                                    bullish,
                                });
                            }
                        }
                        // 5-0: AB=1.13-1.618 XA, BC=1.618-2.24 AB, XD=0.50 BC
                        else if in_range(ab_xa, 1.10, 1.65) && in_range(bc_ab, 1.55, 2.30) {
                            let bc_val = (d.1 - c.1).abs();
                            let xd_bc = if bc > f64::EPSILON { bc_val / bc } else { 0.0 };
                            if in_range(xd_bc, 0.45, 0.55) {
                                let tp1 = if bullish {
                                    d.1 + ad * 0.382
                                } else {
                                    d.1 - ad * 0.382
                                };
                                let tp2 = if bullish {
                                    d.1 + ad * 0.618
                                } else {
                                    d.1 - ad * 0.618
                                };
                                let sl = if bullish {
                                    d.1 - xa * 0.15
                                } else {
                                    d.1 + xa * 0.15
                                };
                                patterns.push(HarmonicPattern {
                                    name: "5-0",
                                    x: (x.0, x.1),
                                    a: (a.0, a.1),
                                    b: (b.0, b.1),
                                    c: (c.0, c.1),
                                    d: (d.0, d.1),
                                    tp1,
                                    tp2,
                                    sl,
                                    bullish,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    // Deduplicate (keep most recent pattern of each type)
    patterns.sort_by(|a, b| b.d.0.cmp(&a.d.0));
    patterns.truncate(10); // keep max 10 most recent
    patterns
}

pub(super) fn in_range(v: f64, lo: f64, hi: f64) -> bool {
    v >= lo && v <= hi
}

pub(super) fn compute_fractals_up(bars: &[Bar]) -> Vec<bool> {
    let n = bars.len();
    let mut out = vec![false; n];
    if n < 5 {
        return out;
    }
    for i in 2..(n - 2) {
        if bars[i].high > bars[i - 1].high
            && bars[i].high > bars[i - 2].high
            && bars[i].high > bars[i + 1].high
            && bars[i].high > bars[i + 2].high
        {
            out[i] = true;
        }
    }
    out
}

pub(super) fn compute_fractals_down(bars: &[Bar]) -> Vec<bool> {
    let n = bars.len();
    let mut out = vec![false; n];
    if n < 5 {
        return out;
    }
    for i in 2..(n - 2) {
        if bars[i].low < bars[i - 1].low
            && bars[i].low < bars[i - 2].low
            && bars[i].low < bars[i + 1].low
            && bars[i].low < bars[i + 2].low
        {
            out[i] = true;
        }
    }
    out
}

/// Previous candle levels for multiple timeframes — matches PreviousCandleLevels.mqh.
/// Returns (H1, H4, D1, W1, MN1) previous candle high/low.
#[allow(clippy::type_complexity)]
pub(super) fn compute_prev_candle_levels(
    bars: &[Bar],
) -> (
    (Option<f64>, Option<f64>), // H1
    (Option<f64>, Option<f64>), // H4
    (Option<f64>, Option<f64>), // D1
    (Option<f64>, Option<f64>), // W1
    (Option<f64>, Option<f64>), // MN1
) {
    if bars.len() < 2 {
        return (
            (None, None),
            (None, None),
            (None, None),
            (None, None),
            (None, None),
        );
    }

    fn group_prev(bars: &[Bar], period_ms: i64) -> (Option<f64>, Option<f64>) {
        let mut groups: Vec<(f64, f64)> = Vec::new();
        let mut current_period = -1_i64;
        let mut hi = f64::MIN;
        let mut lo = f64::MAX;
        for bar in bars {
            let p = bar.ts_ms / period_ms;
            if p != current_period {
                if current_period >= 0 {
                    groups.push((hi, lo));
                }
                current_period = p;
                hi = bar.high;
                lo = bar.low;
            } else {
                hi = hi.max(bar.high);
                lo = lo.min(bar.low);
            }
        }
        if current_period >= 0 {
            groups.push((hi, lo));
        }
        if groups.len() >= 2 {
            let prev = &groups[groups.len() - 2];
            (Some(prev.0), Some(prev.1))
        } else {
            (None, None)
        }
    }

    // Monthly: group by year-month
    fn group_prev_monthly(bars: &[Bar]) -> (Option<f64>, Option<f64>) {
        let mut groups: Vec<(f64, f64)> = Vec::new();
        let mut cur_ym = 0i32;
        let mut hi = f64::MIN;
        let mut lo = f64::MAX;
        for bar in bars {
            let dt = chrono::DateTime::from_timestamp(bar.ts_ms / 1000, 0).unwrap_or_default();
            use chrono::Datelike;
            let ym = dt.year() * 100 + dt.month() as i32;
            if ym != cur_ym {
                if cur_ym > 0 {
                    groups.push((hi, lo));
                }
                cur_ym = ym;
                hi = bar.high;
                lo = bar.low;
            } else {
                hi = hi.max(bar.high);
                lo = lo.min(bar.low);
            }
        }
        if cur_ym > 0 {
            groups.push((hi, lo));
        }
        if groups.len() >= 2 {
            let prev = &groups[groups.len() - 2];
            (Some(prev.0), Some(prev.1))
        } else {
            (None, None)
        }
    }

    let h1 = group_prev(bars, 3_600_000); // 1 hour
    let h4 = group_prev(bars, 14_400_000); // 4 hours
    let d1 = group_prev(bars, 86_400_000); // 1 day
    let w1 = group_prev(bars, 7 * 86_400_000); // 1 week
    let mn1 = group_prev_monthly(bars);

    (h1, h4, d1, w1, mn1)
}

/// Stochastic Oscillator — O(n) with monotonic deque for sliding min/max.
pub(super) fn compute_stochastic(
    bars: &[Bar],
    k_period: usize,
    k_smooth: usize,
    d_smooth: usize,
) -> (Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = bars.len();
    let mut raw_k = vec![None; n];
    if n < k_period {
        return (raw_k.clone(), raw_k);
    }

    // Raw %K with monotonic deque for O(1) sliding window max/min
    let mut max_dq: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    let mut min_dq: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    for i in 0..n {
        while max_dq
            .back()
            .map_or(false, |&j| bars[j].high <= bars[i].high)
        {
            max_dq.pop_back();
        }
        max_dq.push_back(i);
        while max_dq.front().map_or(false, |&j| j + k_period <= i) {
            max_dq.pop_front();
        }

        while min_dq.back().map_or(false, |&j| bars[j].low >= bars[i].low) {
            min_dq.pop_back();
        }
        min_dq.push_back(i);
        while min_dq.front().map_or(false, |&j| j + k_period <= i) {
            min_dq.pop_front();
        }

        if i >= k_period - 1 {
            let hi = bars[*max_dq.front().unwrap_or(&0)].high;
            let lo = bars[*min_dq.front().unwrap_or(&0)].low;
            let range = hi - lo;
            raw_k[i] = Some(if range < f64::EPSILON {
                50.0
            } else {
                (bars[i].close - lo) / range * 100.0
            });
        }
    }

    // Smooth %K (SMA of raw_k)
    let stoch_k = sma_of_option(&raw_k, k_smooth);
    // %D = SMA of %K
    let stoch_d = sma_of_option(&stoch_k, d_smooth);
    (stoch_k, stoch_d)
}

/// SMA over Option<f64> series (skipping None values, rolling window over valid values only).
pub(super) fn sma_of_option(data: &[Option<f64>], period: usize) -> Vec<Option<f64>> {
    let n = data.len();
    let mut out = vec![None; n];
    let vals: Vec<(usize, f64)> = data
        .iter()
        .enumerate()
        .filter_map(|(i, v)| v.map(|x| (i, x)))
        .collect();
    if vals.len() >= period {
        let mut s: f64 = vals[..period].iter().map(|(_, v)| v).sum();
        out[vals[period - 1].0] = Some(s / period as f64);
        for j in period..vals.len() {
            s += vals[j].1 - vals[j - period].1;
            out[vals[j].0] = Some(s / period as f64);
        }
    }
    out
}

pub(super) fn compute_adx(
    bars: &[Bar],
    period: usize,
) -> (Vec<Option<f64>>, Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = bars.len();
    let mut adx = vec![None; n];
    let mut di_plus = vec![None; n];
    let mut di_minus = vec![None; n];
    if n <= period + 1 {
        return (adx, di_plus, di_minus);
    }

    // Compute +DM, -DM, TR
    let mut plus_dm = vec![0.0_f64; n];
    let mut minus_dm = vec![0.0_f64; n];
    let mut tr_vals = vec![0.0_f64; n];

    for i in 1..n {
        let up = bars[i].high - bars[i - 1].high;
        let down = bars[i - 1].low - bars[i].low;
        plus_dm[i] = if up > down && up > 0.0 { up } else { 0.0 };
        minus_dm[i] = if down > up && down > 0.0 { down } else { 0.0 };
        tr_vals[i] = true_range(&bars[i], &bars[i - 1]);
    }

    // Smoothed sums (Wilder's smoothing)
    let mut sm_pdm: f64 = plus_dm[1..=period].iter().sum();
    let mut sm_mdm: f64 = minus_dm[1..=period].iter().sum();
    let mut sm_tr: f64 = tr_vals[1..=period].iter().sum();

    let calc_di = |dm: f64, tr: f64| -> f64 {
        if tr < f64::EPSILON {
            0.0
        } else {
            100.0 * dm / tr
        }
    };

    di_plus[period] = Some(calc_di(sm_pdm, sm_tr));
    di_minus[period] = Some(calc_di(sm_mdm, sm_tr));

    let mut dx_sum = 0.0_f64;
    let di_p = calc_di(sm_pdm, sm_tr);
    let di_m = calc_di(sm_mdm, sm_tr);
    let dx0 = if (di_p + di_m) < f64::EPSILON {
        0.0
    } else {
        100.0 * (di_p - di_m).abs() / (di_p + di_m)
    };
    dx_sum += dx0;

    for i in (period + 1)..n {
        sm_pdm = sm_pdm - sm_pdm / period as f64 + plus_dm[i];
        sm_mdm = sm_mdm - sm_mdm / period as f64 + minus_dm[i];
        sm_tr = sm_tr - sm_tr / period as f64 + tr_vals[i];

        let dip = calc_di(sm_pdm, sm_tr);
        let dim = calc_di(sm_mdm, sm_tr);
        di_plus[i] = Some(dip);
        di_minus[i] = Some(dim);

        let dx = if (dip + dim) < f64::EPSILON {
            0.0
        } else {
            100.0 * (dip - dim).abs() / (dip + dim)
        };

        if i < period * 2 {
            dx_sum += dx;
            if i == period * 2 - 1 {
                adx[i] = Some(dx_sum / period as f64);
            }
        } else if let Some(prev_adx) = adx[i - 1] {
            adx[i] = Some((prev_adx * (period as f64 - 1.0) + dx) / period as f64);
        }
    }
    (adx, di_plus, di_minus)
}

pub(super) fn compute_ichimoku(
    bars: &[Bar],
    tenkan: usize,
    kijun: usize,
    senkou_b: usize,
) -> (
    Vec<Option<f64>>,
    Vec<Option<f64>>,
    Vec<Option<f64>>,
    Vec<Option<f64>>,
) {
    let n = bars.len();
    let mut tk = vec![None; n];
    let mut kj = vec![None; n];
    // Span A and B are shifted forward by kijun periods, but we store them at current index
    // (the chart renderer will need to handle the offset, or we store pre-shifted)
    let mut span_a = vec![None; n];
    let mut span_b = vec![None; n];

    let midpoint = |slice: &[Bar]| -> f64 {
        let hi = slice.iter().map(|b| b.high).fold(f64::MIN, f64::max);
        let lo = slice.iter().map(|b| b.low).fold(f64::MAX, f64::min);
        (hi + lo) / 2.0
    };

    for i in 0..n {
        if i >= tenkan - 1 {
            tk[i] = Some(midpoint(&bars[(i + 1 - tenkan)..=i]));
        }
        if i >= kijun - 1 {
            kj[i] = Some(midpoint(&bars[(i + 1 - kijun)..=i]));
        }
        // Span A = (Tenkan + Kijun) / 2, shifted forward by kijun
        if let (Some(t), Some(k)) = (tk[i], kj[i]) {
            let target = i + kijun;
            if target < n {
                span_a[target] = Some((t + k) / 2.0);
            }
        }
        // Span B = midpoint of senkou_b period, shifted forward by kijun
        if i >= senkou_b - 1 {
            let val = midpoint(&bars[(i + 1 - senkou_b)..=i]);
            let target = i + kijun;
            if target < n {
                span_b[target] = Some(val);
            }
        }
    }
    (tk, kj, span_a, span_b)
}

pub(super) fn compute_wma(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < period {
        return out;
    }
    let denom = (period * (period + 1)) as f64 / 2.0;
    for i in (period - 1)..n {
        let mut sum = 0.0;
        for j in 0..period {
            sum += bars[i + 1 - period + j].close * (j + 1) as f64;
        }
        out[i] = Some(sum / denom);
    }
    out
}

pub(super) fn compute_hma(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    // HMA = WMA(2*WMA(n/2) - WMA(n), sqrt(n))
    let n = bars.len();
    let half = period / 2;
    let sqrt_p = (period as f64).sqrt() as usize;
    let wma_half = compute_wma(bars, half.max(1));
    let wma_full = compute_wma(bars, period);
    // Build diff series
    let mut diff_bars: Vec<Bar> = Vec::with_capacity(n);
    for i in 0..n {
        let close = match (wma_half[i], wma_full[i]) {
            (Some(h), Some(f)) => 2.0 * h - f,
            _ => bars[i].close,
        };
        diff_bars.push(Bar {
            ts_ms: bars[i].ts_ms,
            open: close,
            high: close,
            low: close,
            close,
            volume: 0.0,
        });
    }
    compute_wma(&diff_bars, sqrt_p.max(1))
}

/// CCI — O(n×period) for mean deviation (inherent), but with rolling TP sum for mean.
pub(super) fn compute_cci(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < period {
        return out;
    }
    let pf = period as f64;

    // Pre-compute typical prices
    let tp: Vec<f64> = bars
        .iter()
        .map(|b| (b.high + b.low + b.close) / 3.0)
        .collect();

    // Rolling TP sum for SMA
    let mut tp_sum: f64 = tp[..period].iter().sum();

    for i in (period - 1)..n {
        if i > period - 1 {
            tp_sum += tp[i] - tp[i - period];
        }
        let mean = tp_sum / pf;
        // Mean deviation must be computed fresh (depends on mean)
        let md: f64 = (0..period)
            .map(|k| (tp[i + 1 - period + k] - mean).abs())
            .sum::<f64>()
            / pf;
        out[i] = if md < f64::EPSILON {
            Some(0.0)
        } else {
            Some((tp[i] - mean) / (0.015 * md))
        };
    }
    out
}

/// Williams %R — O(n) with monotonic deque for sliding min/max.
pub(super) fn compute_williams_r(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < period {
        return out;
    }
    let mut max_dq: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    let mut min_dq: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    for i in 0..n {
        while max_dq
            .back()
            .map_or(false, |&j| bars[j].high <= bars[i].high)
        {
            max_dq.pop_back();
        }
        max_dq.push_back(i);
        while max_dq.front().map_or(false, |&j| j + period <= i) {
            max_dq.pop_front();
        }
        while min_dq.back().map_or(false, |&j| bars[j].low >= bars[i].low) {
            min_dq.pop_back();
        }
        min_dq.push_back(i);
        while min_dq.front().map_or(false, |&j| j + period <= i) {
            min_dq.pop_front();
        }
        if i >= period - 1 {
            let hi = bars[*max_dq.front().unwrap_or(&0)].high;
            let lo = bars[*min_dq.front().unwrap_or(&0)].low;
            let range = hi - lo;
            out[i] = if range < f64::EPSILON {
                Some(-50.0)
            } else {
                Some(-100.0 * (hi - bars[i].close) / range)
            };
        }
    }
    out
}

pub(super) fn compute_obv(bars: &[Bar]) -> Vec<Option<f64>> {
    let n = bars.len();
    if n == 0 {
        return vec![];
    }
    let mut out = vec![None; n];
    let mut obv = 0.0_f64;
    out[0] = Some(0.0);
    for i in 1..n {
        if bars[i].close > bars[i - 1].close {
            obv += bars[i].volume;
        } else if bars[i].close < bars[i - 1].close {
            obv -= bars[i].volume;
        }
        out[i] = Some(obv);
    }
    out
}

pub(super) fn compute_momentum(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    for i in period..n {
        out[i] = Some(bars[i].close - bars[i - period].close);
    }
    out
}

pub(super) fn compute_cmo(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if period == 0 || n <= period {
        return out;
    }
    for (i, slot) in out.iter_mut().enumerate().skip(period) {
        let mut sum_up = 0.0;
        let mut sum_dn = 0.0;
        for j in (i + 1 - period)..=i {
            let delta = bars[j].close - bars[j - 1].close;
            if delta > 0.0 {
                sum_up += delta;
            } else if delta < 0.0 {
                sum_dn += -delta;
            }
        }
        let denom = sum_up + sum_dn;
        *slot = Some(if denom > f64::EPSILON {
            100.0 * (sum_up - sum_dn) / denom
        } else {
            0.0
        });
    }
    out
}

pub(super) fn compute_qstick(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if period == 0 || n < period {
        return out;
    }
    let mut body_sum = 0.0;
    for i in 0..n {
        body_sum += bars[i].close - bars[i].open;
        if i >= period {
            body_sum -= bars[i - period].close - bars[i - period].open;
        }
        if i + 1 >= period {
            out[i] = Some(body_sum / period as f64);
        }
    }
    out
}

pub(super) fn compute_disparity(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if period == 0 || n < period {
        return out;
    }
    let mut close_sum = 0.0;
    for i in 0..n {
        close_sum += bars[i].close;
        if i >= period {
            close_sum -= bars[i - period].close;
        }
        if i + 1 >= period {
            let sma = close_sum / period as f64;
            out[i] = Some(if sma.abs() > f64::EPSILON {
                (bars[i].close / sma - 1.0) * 100.0
            } else {
                0.0
            });
        }
    }
    out
}

pub(super) fn compute_bop(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if period == 0 || n < period {
        return out;
    }
    let mut raw = vec![0.0; n];
    for (i, bar) in bars.iter().enumerate() {
        let range = (bar.high - bar.low).max(1e-9);
        raw[i] = (bar.close - bar.open) / range;
    }
    let mut sum = 0.0;
    for i in 0..n {
        sum += raw[i];
        if i >= period {
            sum -= raw[i - period];
        }
        if i + 1 >= period {
            out[i] = Some(sum / period as f64);
        }
    }
    out
}

pub(super) fn compute_stddev(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if period < 2 || n < period {
        return out;
    }
    for (i, slot) in out.iter_mut().enumerate().skip(period - 1) {
        let start = i + 1 - period;
        let mean = bars[start..=i].iter().map(|b| b.close).sum::<f64>() / period as f64;
        let variance = bars[start..=i]
            .iter()
            .map(|b| {
                let d = b.close - mean;
                d * d
            })
            .sum::<f64>()
            / (period - 1) as f64;
        *slot = Some(variance.max(0.0).sqrt());
    }
    out
}

pub(super) fn ema_of_values(values: &[f64], period: usize) -> Vec<Option<f64>> {
    let n = values.len();
    let mut out = vec![None; n];
    if period == 0 || n < period {
        return out;
    }
    let alpha = 2.0 / (period as f64 + 1.0);
    let mut ema = values[..period].iter().sum::<f64>() / period as f64;
    out[period - 1] = Some(ema);
    for i in period..n {
        ema = alpha * values[i] + (1.0 - alpha) * ema;
        out[i] = Some(ema);
    }
    out
}

pub(super) fn ema_of_option_series(values: &[Option<f64>], period: usize) -> Vec<Option<f64>> {
    let n = values.len();
    let mut out = vec![None; n];
    if period == 0 {
        return out;
    }
    let valid: Vec<(usize, f64)> = values
        .iter()
        .enumerate()
        .filter_map(|(i, v)| v.map(|x| (i, x)))
        .collect();
    if valid.len() < period {
        return out;
    }
    let alpha = 2.0 / (period as f64 + 1.0);
    let mut ema = valid[..period].iter().map(|(_, v)| *v).sum::<f64>() / period as f64;
    out[valid[period - 1].0] = Some(ema);
    for &(idx, value) in valid.iter().skip(period) {
        ema = alpha * value + (1.0 - alpha) * ema;
        out[idx] = Some(ema);
    }
    out
}

pub(super) fn compute_mfi(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if period == 0 || n <= period {
        return out;
    }
    let typical: Vec<f64> = bars
        .iter()
        .map(|b| (b.high + b.low + b.close) / 3.0)
        .collect();
    let mut pos_flow = vec![0.0_f64; n];
    let mut neg_flow = vec![0.0_f64; n];
    for i in 1..n {
        let money_flow = typical[i] * bars[i].volume.max(0.0);
        if typical[i] > typical[i - 1] {
            pos_flow[i] = money_flow;
        } else if typical[i] < typical[i - 1] {
            neg_flow[i] = money_flow;
        }
    }
    let mut pos_sum = pos_flow[1..=period].iter().sum::<f64>();
    let mut neg_sum = neg_flow[1..=period].iter().sum::<f64>();
    out[period] = Some(if neg_sum <= f64::EPSILON {
        if pos_sum <= f64::EPSILON { 50.0 } else { 100.0 }
    } else {
        let ratio = pos_sum / neg_sum;
        100.0 - 100.0 / (1.0 + ratio)
    });
    for i in (period + 1)..n {
        pos_sum += pos_flow[i] - pos_flow[i - period];
        neg_sum += neg_flow[i] - neg_flow[i - period];
        out[i] = Some(if neg_sum <= f64::EPSILON {
            if pos_sum <= f64::EPSILON { 50.0 } else { 100.0 }
        } else {
            let ratio = pos_sum / neg_sum;
            100.0 - 100.0 / (1.0 + ratio)
        });
    }
    out
}

pub(super) fn compute_trix(
    bars: &[Bar],
    period: usize,
    signal_period: usize,
) -> (Vec<Option<f64>>, Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = bars.len();
    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let ema1 = ema_of_values(&closes, period);
    let ema2 = ema_of_option_series(&ema1, period);
    let ema3 = ema_of_option_series(&ema2, period);
    let mut trix = vec![None; n];
    for i in 1..n {
        if let (Some(prev), Some(curr)) = (ema3[i - 1], ema3[i]) {
            trix[i] = Some(if prev.abs() > f64::EPSILON {
                100.0 * (curr / prev - 1.0)
            } else {
                0.0
            });
        }
    }
    let signal = ema_of_option_series(&trix, signal_period);
    let hist = trix
        .iter()
        .zip(signal.iter())
        .map(|(line, sig)| match (line, sig) {
            (Some(line), Some(sig)) => Some(line - sig),
            _ => None,
        })
        .collect();
    (trix, signal, hist)
}

pub(super) fn compute_ppo(
    bars: &[Bar],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
) -> (Vec<Option<f64>>, Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = bars.len();
    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let ema_fast = ema_of_values(&closes, fast_period);
    let ema_slow = ema_of_values(&closes, slow_period);
    let mut ppo = vec![None; n];
    for i in 0..n {
        if let (Some(fast), Some(slow)) = (ema_fast[i], ema_slow[i]) {
            ppo[i] = Some(if slow.abs() > f64::EPSILON {
                100.0 * (fast - slow) / slow
            } else {
                0.0
            });
        }
    }
    let signal = ema_of_option_series(&ppo, signal_period);
    let hist = ppo
        .iter()
        .zip(signal.iter())
        .map(|(line, sig)| match (line, sig) {
            (Some(line), Some(sig)) => Some(line - sig),
            _ => None,
        })
        .collect();
    (ppo, signal, hist)
}

pub(super) fn compute_ultosc(bars: &[Bar]) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n <= 28 {
        return out;
    }
    let mut bp = vec![0.0_f64; n];
    let mut tr = vec![0.0_f64; n];
    for i in 1..n {
        let prev_close = bars[i - 1].close;
        let true_low = bars[i].low.min(prev_close);
        let true_high = bars[i].high.max(prev_close);
        bp[i] = bars[i].close - true_low;
        tr[i] = (true_high - true_low).max(1e-9);
    }
    let mut bp_prefix = vec![0.0_f64; n + 1];
    let mut tr_prefix = vec![0.0_f64; n + 1];
    for i in 0..n {
        bp_prefix[i + 1] = bp_prefix[i] + bp[i];
        tr_prefix[i + 1] = tr_prefix[i] + tr[i];
    }
    for i in 28..n {
        let sum7_bp = bp_prefix[i + 1] - bp_prefix[i + 1 - 7];
        let sum14_bp = bp_prefix[i + 1] - bp_prefix[i + 1 - 14];
        let sum28_bp = bp_prefix[i + 1] - bp_prefix[i + 1 - 28];
        let sum7_tr = (tr_prefix[i + 1] - tr_prefix[i + 1 - 7]).max(1e-9);
        let sum14_tr = (tr_prefix[i + 1] - tr_prefix[i + 1 - 14]).max(1e-9);
        let sum28_tr = (tr_prefix[i + 1] - tr_prefix[i + 1 - 28]).max(1e-9);
        let avg7 = sum7_bp / sum7_tr;
        let avg14 = sum14_bp / sum14_tr;
        let avg28 = sum28_bp / sum28_tr;
        out[i] = Some(100.0 * (4.0 * avg7 + 2.0 * avg14 + avg28) / 7.0);
    }
    out
}

pub(super) fn compute_stochrsi(
    bars: &[Bar],
    rsi_period: usize,
    stoch_period: usize,
    k_smooth: usize,
    d_smooth: usize,
) -> (Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = bars.len();
    let rsi = compute_rsi(bars, rsi_period);
    let mut raw = vec![None; n];
    if stoch_period == 0 {
        return (raw.clone(), raw);
    }
    for i in 0..n {
        if i + 1 < stoch_period {
            continue;
        }
        let start = i + 1 - stoch_period;
        let window = &rsi[start..=i];
        if window.iter().any(|v| v.is_none()) {
            continue;
        }
        let mut min_rsi = f64::MAX;
        let mut max_rsi = f64::MIN;
        for value in window.iter().flatten() {
            min_rsi = min_rsi.min(*value);
            max_rsi = max_rsi.max(*value);
        }
        if let Some(curr) = rsi[i] {
            let range = max_rsi - min_rsi;
            raw[i] = Some(if range.abs() > f64::EPSILON {
                ((curr - min_rsi) / range * 100.0).clamp(0.0, 100.0)
            } else {
                50.0
            });
        }
    }
    let k: Vec<Option<f64>> = sma_of_option(&raw, k_smooth)
        .into_iter()
        .map(|v| v.map(|x| x.clamp(0.0, 100.0)))
        .collect();
    let d: Vec<Option<f64>> = sma_of_option(&k, d_smooth)
        .into_iter()
        .map(|v| v.map(|x| x.clamp(0.0, 100.0)))
        .collect();
    (k, d)
}

pub(super) fn compute_var_oscillator(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    const VAR_Z95: f64 = 1.644_853_626_951_472_2;
    const VAR_EPS: f64 = 1e-9;

    let n = bars.len();
    let mut out = vec![None; n];
    if period == 0 || n <= period {
        return out;
    }

    let mut returns = vec![0.0_f64; n];
    for i in 1..n {
        let prev = bars[i - 1].close.max(VAR_EPS);
        let close = bars[i].close.max(VAR_EPS);
        returns[i] = (close / prev).ln();
    }

    let period_f = period as f64;
    let mut sum = 0.0_f64;
    let mut sum_sq = 0.0_f64;
    for ret in &returns[1..=period] {
        sum += *ret;
        sum_sq += *ret * *ret;
    }

    for i in period..n {
        let mean = sum / period_f;
        let variance = (sum_sq / period_f - mean * mean).max(0.0);
        let sigma = variance.sqrt();
        let var95 = (VAR_Z95 * sigma - mean).max(VAR_EPS);
        out[i] = Some(-100.0 * returns[i] / var95);

        if i + 1 < n {
            let removed = returns[i + 1 - period];
            let added = returns[i + 1];
            sum += added - removed;
            sum_sq += added * added - removed * removed;
        }
    }

    out
}

pub(super) fn compute_parabolic_sar(bars: &[Bar], af_step: f64, af_max: f64) -> Vec<Option<f64>> {
    let n = bars.len();
    if n < 2 {
        return vec![None; n];
    }
    let mut out = vec![None; n];
    let mut is_long = bars[1].close > bars[0].close;
    let mut sar = if is_long { bars[0].low } else { bars[0].high };
    let mut ep = if is_long { bars[1].high } else { bars[1].low };
    let mut af = af_step;
    out[1] = Some(sar);

    for i in 2..n {
        sar += af * (ep - sar);
        if is_long {
            sar = sar.min(bars[i - 1].low).min(bars[i - 2].low);
            if bars[i].low < sar {
                is_long = false;
                sar = ep;
                ep = bars[i].low;
                af = af_step;
            } else {
                if bars[i].high > ep {
                    ep = bars[i].high;
                    af = (af + af_step).min(af_max);
                }
            }
        } else {
            sar = sar.max(bars[i - 1].high).max(bars[i - 2].high);
            if bars[i].high > sar {
                is_long = true;
                sar = ep;
                ep = bars[i].high;
                af = af_step;
            } else {
                if bars[i].low < ep {
                    ep = bars[i].low;
                    af = (af + af_step).min(af_max);
                }
            }
        }
        out[i] = Some(sar);
    }
    out
}

/// ATR Projection — matches ATR_Projection.mqh behavior.
/// For each higher timeframe (D1, W1, MN1, H4, H1, M15):
///   find the current HTF period's open price and compute ATR(14) on HTF bars,
///   then project lines at open ± ATR.
/// Returns Vec of (label, open, atr_value, start_bar_idx) for each HTF level.
pub(super) fn compute_atr_projection_levels(
    bars: &[Bar],
    chart_tf_minutes: u32,
) -> Vec<(&'static str, f64, f64, usize)> {
    if bars.is_empty() {
        return Vec::new();
    }
    let mut levels = Vec::new();
    // HTF definitions: (label, period_minutes, lookback_bars_for_line_start, max_chart_tf)
    // max_chart_tf: only show if chart timeframe <= this (matching MT5 _Period checks)
    let htfs: &[(&str, u32, usize, u32)] = &[
        ("M15", 15, 7, 60),          // show on M1..H1
        ("H1", 60, 12, 240),         // show on M1..H4
        ("H4", 240, 11, 1440),       // show on M1..D1
        ("D1", 1440, 7, 10080),      // show on M1..W1
        ("W1", 10080, 4, u32::MAX),  // show always
        ("MN1", 43200, 2, u32::MAX), // show always
    ];

    for &(label, htf_minutes, lookback, max_tf) in htfs {
        if chart_tf_minutes > max_tf {
            continue;
        }
        // Can only project HTFs >= chart timeframe
        if htf_minutes < chart_tf_minutes {
            continue;
        }

        // Find the current HTF period open: the open of the first bar that starts the current HTF candle
        // Walk backward from the last bar to find where the HTF period boundary is
        let last_ts = match bars.last() {
            Some(b) => b.ts_ms,
            None => return Vec::new(),
        };
        let htf_secs = htf_minutes as i64 * 60;
        // For monthly, align to start of month; for weekly, align to Monday 00:00 UTC
        let current_htf_start = if htf_minutes >= 43200 {
            // Monthly: start of current month
            let dt = chrono::DateTime::from_timestamp(last_ts / 1000, 0).unwrap_or_default();
            use chrono::Datelike;
            chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month(), 1)
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|ndt| ndt.and_utc().timestamp() * 1000)
                .unwrap_or(last_ts)
        } else if htf_minutes >= 10080 {
            // Weekly: start of current week (Monday 00:00 UTC)
            let dt = chrono::DateTime::from_timestamp(last_ts / 1000, 0).unwrap_or_default();
            use chrono::Datelike;
            let days_since_mon = dt.weekday().num_days_from_monday() as i64;
            let mon = dt.date_naive() - chrono::Duration::days(days_since_mon);
            mon.and_hms_opt(0, 0, 0)
                .map(|ndt| ndt.and_utc().timestamp() * 1000)
                .unwrap_or(last_ts)
        } else {
            // Standard: floor to HTF period
            let ts_sec = last_ts / 1000;
            (ts_sec / (htf_secs)) * htf_secs * 1000
        };

        // Find the bar at or after current_htf_start → that's the HTF candle open
        let htf_open_bar = bars.iter().position(|b| b.ts_ms >= current_htf_start);
        let htf_open = match htf_open_bar {
            Some(idx) => bars[idx].open,
            None => continue,
        };

        // Compute ATR(14) on aggregated HTF bars
        // Aggregate chart bars into HTF OHLC candles, then compute ATR
        let htf_bars = aggregate_bars_to_htf(bars, htf_minutes);
        let atr_period = 14;
        if htf_bars.len() < atr_period + 1 {
            continue;
        }
        // Compute ATR on the HTF bars (last value)
        let mut tr_vals = Vec::with_capacity(htf_bars.len());
        for i in 0..htf_bars.len() {
            let tr = if i == 0 {
                htf_bars[i].high - htf_bars[i].low
            } else {
                let hl = htf_bars[i].high - htf_bars[i].low;
                let hc = (htf_bars[i].high - htf_bars[i - 1].close).abs();
                let lc = (htf_bars[i].low - htf_bars[i - 1].close).abs();
                hl.max(hc).max(lc)
            };
            tr_vals.push(tr);
        }
        // Simple ATR: average of last `atr_period` TR values
        let start = tr_vals.len().saturating_sub(atr_period);
        let atr_val: f64 = tr_vals[start..].iter().sum::<f64>() / atr_period as f64;
        if atr_val <= 0.0 {
            continue;
        }

        // Start bar index for drawing the line (lookback HTF bars → find corresponding chart bar)
        let start_idx = if lookback < htf_bars.len() {
            let htf_start_bar = &htf_bars[htf_bars.len() - lookback - 1];
            bars.iter()
                .position(|b| b.ts_ms >= htf_start_bar.ts_ms)
                .unwrap_or(0)
        } else {
            0
        };

        levels.push((label, htf_open, atr_val, start_idx));
    }
    levels
}

/// Aggregate chart-timeframe bars into higher-timeframe OHLC bars.
pub(super) fn aggregate_bars_to_htf(bars: &[Bar], htf_minutes: u32) -> Vec<Bar> {
    if bars.is_empty() {
        return Vec::new();
    }
    let htf_ms = htf_minutes as i64 * 60 * 1000;
    let mut result = Vec::new();
    let mut current = Bar {
        ts_ms: 0,
        open: 0.0,
        high: f64::NEG_INFINITY,
        low: f64::INFINITY,
        close: 0.0,
        volume: 0.0,
    };
    let mut period_start = 0i64;

    for bar in bars {
        let bar_period = if htf_minutes >= 43200 {
            // Monthly alignment
            let dt = chrono::DateTime::from_timestamp(bar.ts_ms / 1000, 0).unwrap_or_default();
            use chrono::Datelike;
            chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month(), 1)
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|ndt| ndt.and_utc().timestamp() * 1000)
                .unwrap_or(bar.ts_ms)
        } else if htf_minutes >= 10080 {
            // Weekly alignment (Monday 00:00)
            let dt = chrono::DateTime::from_timestamp(bar.ts_ms / 1000, 0).unwrap_or_default();
            use chrono::Datelike;
            let days_since_mon = dt.weekday().num_days_from_monday() as i64;
            let mon = dt.date_naive() - chrono::Duration::days(days_since_mon);
            mon.and_hms_opt(0, 0, 0)
                .map(|ndt| ndt.and_utc().timestamp() * 1000)
                .unwrap_or(bar.ts_ms)
        } else {
            (bar.ts_ms / htf_ms) * htf_ms
        };

        if bar_period != period_start {
            if current.ts_ms > 0 {
                result.push(current.clone());
            }
            period_start = bar_period;
            current = Bar {
                ts_ms: bar_period,
                open: bar.open,
                high: bar.high,
                low: bar.low,
                close: bar.close,
                volume: bar.volume,
            };
        } else {
            if bar.high > current.high {
                current.high = bar.high;
            }
            if bar.low < current.low {
                current.low = bar.low;
            }
            current.close = bar.close;
            current.volume += bar.volume;
        }
    }
    if current.ts_ms > 0 {
        result.push(current);
    }
    result
}

/// Legacy per-bar ATR projection (kept for GPU compute path compatibility).
pub(super) fn compute_atr_projection(
    bars: &[Bar],
    atr: &[Option<f64>],
) -> (Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = bars.len();
    let mut upper = vec![None; n];
    let mut lower = vec![None; n];
    for i in 0..n {
        if let Some(a) = atr[i] {
            upper[i] = Some(bars[i].open + a);
            lower[i] = Some(bars[i].open - a);
        }
    }
    (upper, lower)
}

/// BetterVolume — 1:1 port of BetterVolume.mqh (Emini-Watch algorithm).
/// Classifies each bar using buy/sell pressure estimation and lookback extremes.
/// Returns: 0=low_vol(yellow), 1=climax_up(red), 2=climax_dn(white), 3=churn(green),
///          4=climax_churn(magenta), 5=normal(steelblue)
pub(super) fn compute_better_volume(bars: &[Bar]) -> Vec<u8> {
    let n = bars.len();
    let lookback = 20usize;
    if n < lookback + 2 {
        return vec![5; n];
    } // all normal if too few bars

    // Estimate buy/sell volumes from candle structure (matching MQL5 EstimateBuySell)
    let estimate_buy_sell = |b: &Bar| -> (f64, f64) {
        let total = b.volume;
        let range = b.high - b.low;
        if range <= 0.0 {
            return (total * 0.5, total * 0.5);
        }
        let (o, c) = (b.open, b.close);
        let buy = if c > o {
            let denom = 2.0 * range + o - c;
            let denom = if denom <= 0.0 { range } else { denom };
            (range / denom) * total
        } else if c < o {
            let denom = 2.0 * range + c - o;
            let denom = if denom <= 0.0 { range } else { denom };
            ((range + c - o) / denom) * total
        } else {
            total * 0.5
        };
        (buy, total - buy)
    };

    let mut out = vec![5u8; n]; // default: normal
    let min_range = 1e-10_f64;

    for i in 0..n {
        if i + lookback >= n {
            continue;
        } // MQL5 series: bar i needs lookback bars AFTER it
        // Wait, in our chronological order: bar i needs lookback bars BEFORE it.
        // MQL5 series: bar 0 = newest, lookback scans bar+1..bar+lookback (older bars).
        // In chronological: bar i is our current bar. Lookback = previous bars i-1..i-lookback.
        if i < lookback {
            continue;
        }

        let b = &bars[i];
        let vol = b.volume;
        let range = (b.high - b.low).max(min_range);
        let (buy_vol, sell_vol) = estimate_buy_sell(b);

        let buy_range = buy_vol * range;
        let sell_range = sell_vol * range;
        let vol_div_r = vol / range;
        let sell_div_r = sell_vol / range;
        let buy_div_r = buy_vol / range;

        // Find lookback extremes (previous `lookback` bars)
        let mut high_buy_range = 0.0_f64;
        let mut high_sell_range = 0.0_f64;
        let mut high_vol_div_r = 0.0_f64;
        let mut low_sell_div_r = f64::MAX;
        let mut low_buy_div_r = f64::MAX;
        let mut low_total_vol = f64::MAX;

        for j in 1..=lookback {
            let bi = i - j;
            let bj = &bars[bi];
            let (bv, sv) = estimate_buy_sell(bj);
            let r = (bj.high - bj.low).max(min_range);
            let v = bj.volume;

            let br = bv * r;
            let sr = sv * r;
            let vr = v / r;
            let sdr = sv / r;
            let bdr = bv / r;

            if br > high_buy_range {
                high_buy_range = br;
            }
            if sr > high_sell_range {
                high_sell_range = sr;
            }
            if vr > high_vol_div_r {
                high_vol_div_r = vr;
            }
            if sdr < low_sell_div_r {
                low_sell_div_r = sdr;
            }
            if bdr < low_buy_div_r {
                low_buy_div_r = bdr;
            }
            if v < low_total_vol {
                low_total_vol = v;
            }
        }

        // 1-bar classification flags
        let mut is_climax_up = false;
        let mut is_climax_dn = false;
        let mut is_churn = false;
        let mut is_low_vol = false;

        // Low Volume: volume <= lowest in lookback
        if vol <= low_total_vol {
            is_low_vol = true;
        }

        // Climax Up: (buyVol*range == highest) OR (sellVol/range == lowest), C > O
        if b.close > b.open {
            if buy_range >= high_buy_range || sell_div_r <= low_sell_div_r {
                is_climax_up = true;
            }
        }

        // Climax Down: (sellVol*range == highest) OR (buyVol/range == lowest), C < O
        if b.close < b.open {
            if sell_range >= high_sell_range || buy_div_r <= low_buy_div_r {
                is_climax_dn = true;
            }
        }

        // Churn: totalVol/range == highest in lookback
        if vol_div_r >= high_vol_div_r {
            is_churn = true;
        }

        // 2-bar analysis (matching MQL5 InpUse2Bars=true)
        if i >= lookback + 1 {
            let b2 = &bars[i - 1];
            let (bv2, sv2) = estimate_buy_sell(b2);
            let total_buy = buy_vol + bv2;
            let total_sell = sell_vol + sv2;
            let total_vol2 = vol + b2.volume;
            let range2 = (b.high.max(b2.high) - b.low.min(b2.low)).max(min_range);

            let buy_range2 = total_buy * range2;
            let sell_range2 = total_sell * range2;
            let vol_div_r2 = total_vol2 / range2;
            let sell_div_r2 = total_sell / range2;
            let buy_div_r2 = total_buy / range2;

            // 2-bar lookback extremes
            let mut h_br2 = 0.0_f64;
            let mut h_sr2 = 0.0_f64;
            let mut h_vr2 = 0.0_f64;
            let mut l_sdr2 = f64::MAX;
            let mut l_bdr2 = f64::MAX;
            let mut l_vol2 = f64::MAX;

            for j in 1..=lookback {
                let b1i = i - j;
                if b1i == 0 {
                    break;
                }
                let b2i = b1i - 1;
                let bj1 = &bars[b1i];
                let bj2 = &bars[b2i];
                let (bva, sva) = estimate_buy_sell(bj1);
                let (bvb, svb) = estimate_buy_sell(bj2);
                let tb = bva + bvb;
                let ts = sva + svb;
                let tv = bj1.volume + bj2.volume;
                let r2 = (bj1.high.max(bj2.high) - bj1.low.min(bj2.low)).max(min_range);

                if tb * r2 > h_br2 {
                    h_br2 = tb * r2;
                }
                if ts * r2 > h_sr2 {
                    h_sr2 = ts * r2;
                }
                if tv / r2 > h_vr2 {
                    h_vr2 = tv / r2;
                }
                if ts / r2 < l_sdr2 {
                    l_sdr2 = ts / r2;
                }
                if tb / r2 < l_bdr2 {
                    l_bdr2 = tb / r2;
                }
                if tv < l_vol2 {
                    l_vol2 = tv;
                }
            }

            if total_vol2 <= l_vol2 {
                is_low_vol = true;
            }
            if b.close > b.open && (buy_range2 >= h_br2 || sell_div_r2 <= l_sdr2) {
                is_climax_up = true;
            }
            if b.close < b.open && (sell_range2 >= h_sr2 || buy_div_r2 <= l_bdr2) {
                is_climax_dn = true;
            }
            if vol_div_r2 >= h_vr2 {
                is_churn = true;
            }
        }

        // Priority: ClimaxChurn > LowVol > ClimaxUp > ClimaxDown > Churn > Normal
        out[i] = if (is_climax_up || is_climax_dn) && is_churn {
            4
        }
        // climax+churn (magenta)
        else if is_low_vol {
            0
        }
        // low volume (yellow)
        else if is_climax_up {
            1
        }
        // climax up (red)
        else if is_climax_dn {
            2
        }
        // climax down (white)
        else if is_churn {
            3
        }
        // churn (green)
        else {
            5
        }; // normal (steelblue)
    }
    out
}

/// Supply/Demand zones from GPU fractal detection output.
/// GPU Phase 1 outputs [zone_type, zone_high, zone_low] per bar.
/// CPU Phase 2: refine boundaries with actual open prices, test zones, merge, purge broken.
pub(super) fn compute_supply_demand_zones_from_gpu(
    gpu_data: &[f32],
    bars: &[Bar],
) -> (Vec<(usize, f64, f64, u8)>, Vec<(usize, f64, f64, u8)>) {
    const FRACTAL_LOOKBACK: usize = 5;
    let n = bars.len();
    if n < FRACTAL_LOOKBACK * 2 + 1 {
        return (Vec::new(), Vec::new());
    }

    struct Zone {
        idx: usize,
        hi: f64,
        lo: f64,
        touches: u32,
        is_supply: bool,
        broken: bool,
    }
    let mut zones: Vec<Zone> = Vec::new();

    let min_height = bars
        .iter()
        .filter_map(|b| {
            let r = b.high - b.low;
            if r > 0.0 { Some(r) } else { None }
        })
        .fold(f64::MAX, f64::min)
        * 0.01;

    // Apply BACK_LIMIT (matching MQL5 InpBackLimit=1000) to limit scan depth
    const BACK_LIMIT: usize = 1000;
    let scan_start = if n > BACK_LIMIT + FRACTAL_LOOKBACK {
        n - BACK_LIMIT
    } else {
        0
    };

    // Extract fractals from GPU output, refine boundaries with actual open prices
    for i in scan_start..n {
        let zone_type = gpu_data.get(i * 3).copied().unwrap_or(0.0);
        if zone_type < -0.5 {
            // Supply fractal: hi = high, lo = min(close, open)
            let hi = bars[i].high;
            let lo = bars[i].close.min(bars[i].open);
            let lo = if hi - lo < min_height {
                hi - min_height
            } else {
                lo
            };
            zones.push(Zone {
                idx: i,
                hi,
                lo,
                touches: 0,
                is_supply: true,
                broken: false,
            });
        } else if zone_type > 0.5 {
            // Demand fractal: hi = max(close, open), lo = low
            let lo = bars[i].low;
            let hi = bars[i].close.max(bars[i].open);
            let hi = if hi - lo < min_height {
                lo + min_height
            } else {
                hi
            };
            zones.push(Zone {
                idx: i,
                hi,
                lo,
                touches: 0,
                is_supply: false,
                broken: false,
            });
        }
    }

    // Test zones, merge, purge — identical to CPU-only path
    for z in &mut zones {
        let scan_from = z.idx + FRACTAL_LOOKBACK + 1;
        for b in scan_from..n {
            if bars[b].high >= z.lo && bars[b].low <= z.hi {
                if z.is_supply && bars[b].close > z.hi {
                    z.broken = true;
                    break;
                }
                if !z.is_supply && bars[b].close < z.lo {
                    z.broken = true;
                    break;
                }
                z.touches += 1;
            }
        }
    }
    zones.retain(|z| !z.broken);

    if zones.len() >= 2 {
        zones.sort_by(|a, b| {
            let type_a = if a.is_supply { 0u8 } else { 1 };
            let type_b = if b.is_supply { 0u8 } else { 1 };
            type_a
                .cmp(&type_b)
                .then(a.lo.partial_cmp(&b.lo).unwrap_or(std::cmp::Ordering::Equal))
        });
        let mut write = 0;
        for i in 1..zones.len() {
            let same_type = zones[i].is_supply == zones[write].is_supply;
            let overlapping = zones[i].lo <= zones[write].hi;
            if same_type && overlapping {
                zones[write].hi = zones[write].hi.max(zones[i].hi);
                zones[write].lo = zones[write].lo.min(zones[i].lo);
                zones[write].touches += zones[i].touches;
                if zones[i].idx < zones[write].idx {
                    zones[write].idx = zones[i].idx;
                }
            } else {
                write += 1;
                if write != i {
                    zones[write] = Zone {
                        idx: zones[i].idx,
                        hi: zones[i].hi,
                        lo: zones[i].lo,
                        touches: zones[i].touches,
                        is_supply: zones[i].is_supply,
                        broken: zones[i].broken,
                    };
                }
            }
        }
        zones.truncate(write + 1);
    }

    let mut supply = Vec::new();
    let mut demand = Vec::new();
    for z in &zones {
        let strength: u8 = if z.touches == 0 {
            0
        } else if z.touches <= 2 {
            1
        } else {
            2
        };
        if z.is_supply {
            supply.push((z.idx, z.hi, z.lo, strength));
        } else {
            demand.push((z.idx, z.hi, z.lo, strength));
        }
    }
    (supply, demand)
}

/// Supply/Demand zone detection — 1:1 port of SupplyDemand.mqh fractal-based algorithm.
/// Returns (supply_zones, demand_zones) each as Vec<(bar_idx, zone_high, zone_low, strength)>.
/// Strength: 0=untested, 1=tested (1-2 touches), 2=proven (3+ touches).
/// Broken zones are purged (not returned).
pub(super) fn compute_supply_demand_zones(
    bars: &[Bar],
) -> (Vec<(usize, f64, f64, u8)>, Vec<(usize, f64, f64, u8)>) {
    const FRACTAL_LOOKBACK: usize = 5;
    const BACK_LIMIT: usize = 1000;

    let n = bars.len();
    if n < FRACTAL_LOOKBACK * 2 + 1 {
        return (Vec::new(), Vec::new());
    }

    let limit = BACK_LIMIT.min(n.saturating_sub(FRACTAL_LOOKBACK + 1));

    // Zone: (bar_idx, hi, lo, touch_count, is_supply, is_broken)
    struct Zone {
        idx: usize,
        hi: f64,
        lo: f64,
        touches: u32,
        is_supply: bool,
        broken: bool,
    }

    let mut zones: Vec<Zone> = Vec::new();

    // Minimum zone height: smallest price increment we can detect
    let min_height = bars
        .iter()
        .filter_map(|b| {
            let r = b.high - b.low;
            if r > 0.0 { Some(r) } else { None }
        })
        .fold(f64::MAX, f64::min)
        * 0.01;

    // ── Find fractal zones (matching MQL5 IsFractalHigh/IsFractalLow) ────
    // Bars are in chronological order (0=oldest). MQL5 uses series mode (0=newest).
    // MQL5 scans i from InpFractalLookback..limit-InpFractalLookback with series arrays.
    // In chronological: scan from (n-1-limit+FRACTAL_LOOKBACK) to (n-1-FRACTAL_LOOKBACK).
    let scan_start = if n > limit + FRACTAL_LOOKBACK {
        n - 1 - limit + FRACTAL_LOOKBACK
    } else {
        FRACTAL_LOOKBACK
    };
    let scan_end = n - 1 - FRACTAL_LOOKBACK;

    for i in scan_start..=scan_end {
        // Fractal high: bar's high must be STRICTLY greater than ALL lookback bars.
        // MQL5 uses >= for rejection: if(high[bar-i] >= val) return false;
        // This means equal highs REJECT the fractal (matching MT5 1:1).
        let is_fractal_high = (1..=FRACTAL_LOOKBACK).all(|k| {
            i >= k
                && i + k < n
                && bars[i].high > bars[i - k].high
                && bars[i].high > bars[i + k].high
        });
        if is_fractal_high {
            let hi = bars[i].high;
            let lo = bars[i].close.min(bars[i].open);
            let lo = if hi - lo < min_height {
                hi - min_height
            } else {
                lo
            };
            zones.push(Zone {
                idx: i,
                hi,
                lo,
                touches: 0,
                is_supply: true,
                broken: false,
            });
        }

        // Fractal low: bar's low must be STRICTLY less than ALL lookback bars.
        // MQL5 uses <= for rejection: if(low[bar-i] <= val) return false;
        let is_fractal_low = (1..=FRACTAL_LOOKBACK).all(|k| {
            i >= k && i + k < n && bars[i].low < bars[i - k].low && bars[i].low < bars[i + k].low
        });
        if is_fractal_low {
            let lo = bars[i].low;
            let hi = bars[i].close.max(bars[i].open);
            let hi = if hi - lo < min_height {
                lo + min_height
            } else {
                hi
            };
            zones.push(Zone {
                idx: i,
                hi,
                lo,
                touches: 0,
                is_supply: false,
                broken: false,
            });
        }
    }

    // ── Test zones against subsequent price action (matching MQL5 TestZones) ────
    // MQL5 scans from fractalBar - lookback - 1 down to 0 (series = toward newest).
    // In chronological: scan from fractal_idx + lookback + 1 toward n-1.
    for z in &mut zones {
        let scan_from = z.idx + FRACTAL_LOOKBACK + 1;
        for b in scan_from..n {
            // Does bar's range overlap the zone?
            if bars[b].high >= z.lo && bars[b].low <= z.hi {
                // Check for break: close pierces beyond zone boundary
                if z.is_supply && bars[b].close > z.hi {
                    z.broken = true;
                    break;
                }
                if !z.is_supply && bars[b].close < z.lo {
                    z.broken = true;
                    break;
                }
                z.touches += 1;
            }
        }
    }

    // Purge broken zones
    zones.retain(|z| !z.broken);

    // ── Merge overlapping same-type zones (matching MQL5 MergeZones sort-and-sweep) ──
    if zones.len() >= 2 {
        // Sort by is_supply (supply first = false < true? MQL5: ZONE_SUPPLY=0 first)
        // MQL5 sorts by type ascending (SUPPLY=0 first), then lo ascending
        zones.sort_by(|a, b| {
            let type_a = if a.is_supply { 0u8 } else { 1 };
            let type_b = if b.is_supply { 0u8 } else { 1 };
            type_a
                .cmp(&type_b)
                .then(a.lo.partial_cmp(&b.lo).unwrap_or(std::cmp::Ordering::Equal))
        });

        let mut write = 0;
        for i in 1..zones.len() {
            let same_type = zones[i].is_supply == zones[write].is_supply;
            let overlapping = zones[i].lo <= zones[write].hi;
            if same_type && overlapping {
                // Merge into write
                zones[write].hi = zones[write].hi.max(zones[i].hi);
                zones[write].lo = zones[write].lo.min(zones[i].lo);
                zones[write].touches += zones[i].touches;
                if zones[i].idx < zones[write].idx {
                    zones[write].idx = zones[i].idx;
                }
            } else {
                write += 1;
                if write != i {
                    let z = Zone {
                        idx: zones[i].idx,
                        hi: zones[i].hi,
                        lo: zones[i].lo,
                        touches: zones[i].touches,
                        is_supply: zones[i].is_supply,
                        broken: zones[i].broken,
                    };
                    zones[write] = z;
                }
            }
        }
        zones.truncate(write + 1);
    }

    // ── Assign strength and split into supply/demand ──
    let mut supply: Vec<(usize, f64, f64, u8)> = Vec::new();
    let mut demand: Vec<(usize, f64, f64, u8)> = Vec::new();

    for z in &zones {
        let strength: u8 = if z.touches == 0 {
            0
        } else if z.touches <= 2 {
            1
        } else {
            2
        };
        if z.is_supply {
            supply.push((z.idx, z.hi, z.lo, strength));
        } else {
            demand.push((z.idx, z.hi, z.lo, strength));
        }
    }

    (supply, demand)
}

// ─── Ehlers indicators ───────────────────────────────────────────────────────

pub(super) fn ehlers_super_smoother(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < 3 {
        return out;
    }
    let a = (-1.414 * std::f64::consts::PI / period as f64).exp();
    let b = 2.0 * a * (1.414 * std::f64::consts::PI / period as f64).cos();
    let c2 = b;
    let c3 = -a * a;
    let c1 = 1.0 - c2 - c3;
    out[0] = Some(bars[0].close);
    out[1] = Some(bars[1].close);
    for i in 2..n {
        let prev1 = out[i - 1].unwrap_or(bars[i - 1].close);
        let prev2 = out[i - 2].unwrap_or(bars[i - 2].close);
        let val = c1 * (bars[i].close + bars[i - 1].close) / 2.0 + c2 * prev1 + c3 * prev2;
        out[i] = Some(val);
    }
    out
}

pub(super) fn ehlers_decycler(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    // Decycler = price - highpass(price, period)
    let n = bars.len();
    let mut out = vec![None; n];
    if n < 3 {
        return out;
    }
    let alpha = (2.0 * std::f64::consts::PI / (period as f64 * 1.414)).cos();
    let a1 = (alpha + (alpha * alpha - 1.0).max(0.0).sqrt())
        .max(0.001)
        .recip();
    // 2-pole highpass
    let mut hp = vec![0.0_f64; n];
    for i in 2..n {
        hp[i] = (1.0 - a1 / 2.0)
            * (1.0 - a1 / 2.0)
            * (bars[i].close - 2.0 * bars[i - 1].close + bars[i - 2].close)
            + 2.0 * (1.0 - a1) * hp[i - 1]
            - (1.0 - a1) * (1.0 - a1) * hp[i - 2];
    }
    for i in 0..n {
        out[i] = Some(bars[i].close - hp[i]);
    }
    out
}

pub(super) fn ehlers_instantaneous_trendline(bars: &[Bar]) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < 7 {
        return out;
    }
    let mut itl = vec![0.0_f64; n];
    for i in 0..7.min(n) {
        itl[i] = bars[i].close;
    }
    for i in 7..n {
        itl[i] = (bars[i].close + 2.0 * bars[i - 1].close + bars[i - 2].close) / 4.0 * 0.5
            + itl[i - 1] * 0.5;
        // Simplified Ehlers ITL: 2-bar WMA smoothed recursively
        itl[i] = (2.0 * itl[i] + itl[i - 1] + itl[i - 2] + itl[i - 3]) / 5.0;
    }
    for i in 0..n {
        out[i] = Some(itl[i]);
    }
    out
}

pub(super) fn ehlers_mama_fama(
    bars: &[Bar],
    fast_limit: f64,
    slow_limit: f64,
) -> (Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = bars.len();
    let mut mama = vec![None; n];
    let mut fama = vec![None; n];
    if n < 6 {
        return (mama, fama);
    }

    let mut smooth = vec![0.0_f64; n];
    let mut phase = vec![0.0_f64; n];
    let mut mama_v = vec![0.0_f64; n];
    let mut fama_v = vec![0.0_f64; n];
    let mut i1 = vec![0.0_f64; n];
    let mut q1 = vec![0.0_f64; n];

    for i in 3..n {
        smooth[i] = (4.0 * bars[i].close
            + 3.0 * bars[i - 1].close
            + 2.0 * bars[i - 2].close
            + bars[i - 3].close)
            / 10.0;
    }

    for i in 6..n {
        let det = 0.0962 * smooth[i] + 0.5769 * smooth[i - 2]
            - 0.5769 * smooth[i - 4]
            - 0.0962 * smooth[i - 6];
        q1[i] = det;
        i1[i] = smooth[i - 3];

        // Phase
        if i1[i].abs() > 0.0 {
            phase[i] = (q1[i] / i1[i]).atan().to_degrees();
        }
        let delta_phase = (phase[i - 1] - phase[i]).max(1.0);
        let alpha = (fast_limit / delta_phase).max(slow_limit);

        if i < 7 {
            mama_v[i] = bars[i].close;
            fama_v[i] = bars[i].close;
        } else {
            mama_v[i] = alpha * smooth[i] + (1.0 - alpha) * mama_v[i - 1];
            fama_v[i] = 0.5 * alpha * mama_v[i] + (1.0 - 0.5 * alpha) * fama_v[i - 1];
        }
        mama[i] = Some(mama_v[i]);
        fama[i] = Some(fama_v[i]);
    }
    (mama, fama)
}

pub(super) fn ehlers_even_better_sinewave(bars: &[Bar], duration: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < 5 {
        return out;
    }
    // Highpass then super smooth, then compute sinewave
    let mut hp = vec![0.0_f64; n];
    let alpha1 = (2.0 * std::f64::consts::PI / (duration as f64 * 1.414)).cos();
    let a1_coeff = if alpha1.abs() > f64::EPSILON {
        (alpha1 + (alpha1 * alpha1 - 1.0).max(0.0).sqrt())
            .max(0.001)
            .recip()
    } else {
        0.5
    };

    for i in 2..n {
        hp[i] = (1.0 - a1_coeff / 2.0).powi(2)
            * (bars[i].close - 2.0 * bars[i - 1].close + bars[i - 2].close)
            + 2.0 * (1.0 - a1_coeff) * hp[i - 1]
            - (1.0 - a1_coeff).powi(2) * hp[i - 2];
    }
    // Super smoother on HP
    let period = duration / 4;
    let a = (-1.414 * std::f64::consts::PI / period.max(1) as f64).exp();
    let b = 2.0 * a * (1.414 * std::f64::consts::PI / period.max(1) as f64).cos();
    let c1 = 1.0 - b + a * a;
    let mut filt = vec![0.0_f64; n];
    for i in 2..n {
        filt[i] = c1 * (hp[i] + hp[i - 1]) / 2.0 + b * filt[i - 1] - a * a * filt[i - 2];
    }
    // Wave = atan(filt[i] / filt[i-1]) if filt[i-1] != 0
    for i in 1..n {
        if filt[i - 1].abs() > f64::EPSILON {
            let wave = (filt[i] / filt[i - 1]).atan() / std::f64::consts::FRAC_PI_2;
            out[i] = Some(wave.clamp(-1.0, 1.0));
        } else {
            out[i] = Some(0.0);
        }
    }
    out
}

pub(super) fn ehlers_cyber_cycle(bars: &[Bar]) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < 5 {
        return out;
    }
    let mut smooth = vec![0.0_f64; n];
    let mut cycle = vec![0.0_f64; n];
    for i in 3..n {
        smooth[i] = (bars[i].close + 2.0 * bars[i - 1].close + bars[i - 2].close) / 4.0;
    }
    let alpha = 0.07; // 2/(period+1) with period~27
    for i in 4..n {
        let c1: f64 = 1.0 - 0.5 * alpha;
        let c2: f64 = 1.0 - alpha;
        cycle[i] = c1 * c1 * (smooth[i] - 2.0 * smooth[i - 1] + smooth[i - 2])
            + 2.0 * c2 * cycle[i - 1]
            - c2 * c2 * cycle[i - 2];
        out[i] = Some(cycle[i]);
    }
    out
}

pub(super) fn ehlers_cg_oscillator(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < period {
        return out;
    }
    for i in (period - 1)..n {
        let mut num = 0.0_f64;
        let mut den = 0.0_f64;
        for j in 0..period {
            let p = bars[i - j].close;
            num += (j as f64 + 1.0) * p;
            den += p;
        }
        out[i] = if den.abs() > f64::EPSILON {
            Some(-num / den + (period as f64 + 1.0) / 2.0)
        } else {
            Some(0.0)
        };
    }
    out
}

pub(super) fn ehlers_roofing_filter(
    bars: &[Bar],
    lp_period: usize,
    hp_period: usize,
) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < 3 {
        return out;
    }
    // Highpass
    let alpha1 = (2.0 * std::f64::consts::PI / hp_period as f64).cos();
    let a1 = if alpha1.abs() > f64::EPSILON {
        (alpha1 + (alpha1 * alpha1 - 1.0).max(0.0).sqrt())
            .max(0.001)
            .recip()
    } else {
        0.5
    };
    let mut hp = vec![0.0_f64; n];
    for i in 2..n {
        hp[i] = (1.0 - a1 / 2.0).powi(2)
            * (bars[i].close - 2.0 * bars[i - 1].close + bars[i - 2].close)
            + 2.0 * (1.0 - a1) * hp[i - 1]
            - (1.0 - a1).powi(2) * hp[i - 2];
    }
    // Super smoother on HP output
    let a = (-1.414 * std::f64::consts::PI / lp_period as f64).exp();
    let b = 2.0 * a * (1.414 * std::f64::consts::PI / lp_period as f64).cos();
    let c1 = 1.0 - b + a * a;
    let mut filt = vec![0.0_f64; n];
    for i in 2..n {
        filt[i] = c1 * (hp[i] + hp[i - 1]) / 2.0 + b * filt[i - 1] - a * a * filt[i - 2];
        out[i] = Some(filt[i]);
    }
    out
}

// ─── chart rendering ─────────────────────────────────────────────────────────

/// Draw a single chart viewport into `rect` using `painter`.
pub(super) fn draw_chart(
    painter: &egui::Painter,
    chart: &ChartState,
    rect: egui::Rect,
    crosshair: Option<egui::Pos2>,
    flags: &IndicatorFlags,
    show_rsi: bool,
    show_fisher: bool,
    show_macd: bool,
    show_volume_pane: bool,
    show_stochastic: bool,
    show_adx: bool,
    show_cci: bool,
    show_williams_r: bool,
    show_obv: bool,
    show_momentum: bool,
    show_cmo: bool,
    show_qstick: bool,
    show_disparity: bool,
    show_bop: bool,
    show_stddev: bool,
    show_mfi: bool,
    show_trix: bool,
    show_ppo: bool,
    show_ultosc: bool,
    show_stochrsi: bool,
    show_var_oscillator: bool,
    show_better_volume: bool,
    show_ehlers_ebsw: bool,
    show_ehlers_cyber: bool,
    show_ehlers_cg: bool,
    show_ehlers_roof: bool,
    show_squeeze: bool,
    sl_price: Option<f64>,
    tp_price: Option<f64>,
    trade_overlay: &TradeOverlay,
    alerts: &[(f64, String)],
    draw_mode: &DrawMode,
) {
    // ── Performance early-out for live Kraken WS updates ───────────────────
    // Fast path: if nothing changed since last render, skip everything.
    if !chart.forming_bar_dirty
        && chart.visible_bars_gen == chart.last_rendered_gen
        && chart.last_visible_bar_ts == chart.last_rendered_bar_ts
        && chart.visible_bars_gen > 0
    {
        return;
    }
    // Update the "last rendered" snapshot for next frame
    // (we mutate through &mut via interior mutability or by accepting &mut ChartState
    // in a real caller; for now we just document the intent).
    // In practice the render loop should call chart.last_rendered_gen = chart.visible_bars_gen etc after draw.

    // ── background ──────────────────────────────────────────────────────────
    painter.rect_filled(rect, 0.0, BG);

    let (start_idx, end_idx) = chart.visible_range();
    let bars = &chart.bars[start_idx..end_idx];

    if bars.is_empty() {
        // Show the live bar-fetch path instead of pointing users at MT5-only tooling.
        let sym = chart.symbol.as_str();
        let line1 = format!("No data for {}", sym);
        let line2 = "Fetching bars from Kraken when available; chart refreshes after cache update"
            .to_string();
        painter.text(
            rect.center() - egui::vec2(0.0, 12.0),
            egui::Align2::CENTER_CENTER,
            line1,
            egui::FontId::proportional(16.0),
            egui::Color32::from_rgb(180, 180, 200),
        );
        painter.text(
            rect.center() + egui::vec2(0.0, 10.0),
            egui::Align2::CENTER_CENTER,
            line2,
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(110, 110, 130),
        );
        return;
    }

    // Allocate sub-pane space at bottom
    let sub_pane_count = show_rsi as u8
        + show_fisher as u8
        + show_macd as u8
        + show_volume_pane as u8
        + show_stochastic as u8
        + show_adx as u8
        + show_cci as u8
        + show_williams_r as u8
        + show_obv as u8
        + show_momentum as u8
        + show_cmo as u8
        + show_qstick as u8
        + show_disparity as u8
        + show_bop as u8
        + show_stddev as u8
        + show_mfi as u8
        + show_trix as u8
        + show_ppo as u8
        + show_ultosc as u8
        + show_stochrsi as u8
        + show_var_oscillator as u8
        + show_better_volume as u8
        + show_ehlers_ebsw as u8
        + show_ehlers_cyber as u8
        + show_ehlers_cg as u8
        + show_ehlers_roof as u8
        + show_squeeze as u8;
    const SUB_PANE_H: f32 = 80.0; // Height per indicator sub-pane (RSI, Fisher, MACD, Volume)
    const MIN_MAIN_CHART_H: f32 = 140.0;
    // When user is interacting, some expensive sub-pane rendering can be skipped in future passes
    let sub_pane_height = if sub_pane_count > 0 {
        // Keep the main price chart valid even when many sub-panes are enabled
        // or the window is temporarily tiny during startup/layout restore. A
        // negative-height chart rect makes later f32::clamp calls panic
        // (`min > max`). The sub-panes may overflow/clipped below, but the app
        // must never crash because indicator height exceeded available space.
        (SUB_PANE_H * sub_pane_count as f32).min((rect.height() - MIN_MAIN_CHART_H).max(0.0))
    } else {
        0.0
    };
    let main_rect = egui::Rect::from_min_max(
        rect.min,
        egui::pos2(rect.right(), rect.bottom() - sub_pane_height),
    );

    // Price axis margins
    let price_axis_w = 70.0_f32;
    let time_axis_h = 22.0_f32;
    let chart_rect = egui::Rect::from_min_max(
        main_rect.min,
        egui::pos2(
            main_rect.right() - price_axis_w,
            main_rect.bottom() - time_axis_h,
        ),
    );

    // Price axis background (subtle — indicates it's interactive like TradingView)
    let price_axis_bg = egui::Rect::from_min_max(
        egui::pos2(chart_rect.right(), chart_rect.top()),
        egui::pos2(rect.right(), chart_rect.bottom()),
    );
    painter.rect_filled(price_axis_bg, 0.0, egui::Color32::from_rgb(6, 6, 10));
    // Thin separator line between chart and price axis
    painter.line_segment(
        [
            egui::pos2(chart_rect.right(), chart_rect.top()),
            egui::pos2(chart_rect.right(), chart_rect.bottom()),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(25, 30, 45)),
    );
    // Subtle drag handle indicator (3 horizontal lines at center of price axis)
    if let Some(cross) = crosshair {
        if cross.x > chart_rect.right() && cross.x < rect.right() {
            let cx = chart_rect.right() + price_axis_w * 0.5;
            let cy = price_axis_bg.center().y;
            for dy in [-4.0_f32, 0.0, 4.0] {
                painter.line_segment(
                    [egui::pos2(cx - 6.0, cy + dy), egui::pos2(cx + 6.0, cy + dy)],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 70, 90)),
                );
            }
        }
    }

    // ── price range ─────────────────────────────────────────────────────────
    let mut price_min = bars.iter().map(|b| b.low).fold(f64::MAX, f64::min);
    let mut price_max = bars.iter().map(|b| b.high).fold(f64::MIN, f64::max);

    // Also account for indicator values in visible range
    for i in start_idx..end_idx {
        if flags.sma200 {
            if let Some(v) = chart.sma200[i] {
                price_min = price_min.min(v);
                price_max = price_max.max(v);
            }
        }
        if flags.sma100 {
            if let Some(v) = chart.sma100[i] {
                price_min = price_min.min(v);
                price_max = price_max.max(v);
            }
        }
        if flags.kama {
            if let Some(v) = chart.kama[i] {
                price_min = price_min.min(v);
                price_max = price_max.max(v);
            }
        }
        if flags.ema21 {
            if let Some(v) = chart.ema21[i] {
                price_min = price_min.min(v);
                price_max = price_max.max(v);
            }
        }
        if flags.bollinger {
            if let Some(v) = chart.bb_upper[i] {
                price_max = price_max.max(v);
            }
            if let Some(v) = chart.bb_lower[i] {
                price_min = price_min.min(v);
            }
        }
        if flags.ichimoku {
            if let Some(v) = chart.ichi_span_a[i] {
                price_min = price_min.min(v);
                price_max = price_max.max(v);
            }
            if let Some(v) = chart.ichi_span_b[i] {
                price_min = price_min.min(v);
                price_max = price_max.max(v);
            }
        }
    }

    let padding = (price_max - price_min) * 0.05;
    price_min -= padding;
    price_max += padding;

    // Vertical pan + zoom
    let range = price_max - price_min;
    let centre = (price_max + price_min) * 0.5 + chart.price_pan;
    let half = range * 0.5 / chart.price_zoom;
    price_min = centre - half;
    price_max = centre + half;

    if (price_max - price_min).abs() < f64::EPSILON {
        return;
    }

    let use_log = chart.log_scale && price_min > 0.0; // log scale requires positive prices
    // Precompute the log-axis constants once. price_to_y is called once per visible
    // bar per indicator (~hundreds per frame), so hoisting the two `.ln()` calls out
    // of the closure turns a per-call cost into a per-frame cost.
    let log_max = if use_log { price_max.ln() } else { 0.0 };
    let log_min = if use_log { price_min.ln() } else { 0.0 };
    let log_range = log_max - log_min;
    let log_range_degenerate = use_log && log_range.abs() < f64::EPSILON;
    let linear_range = price_max - price_min;
    let chart_top = chart_rect.top();
    let chart_h = chart_rect.height();
    let price_to_y = |p: f64| -> f32 {
        let frac = if use_log {
            if log_range_degenerate {
                0.5
            } else {
                (log_max - p.max(0.001).ln()) / log_range
            }
        } else {
            (price_max - p) / linear_range
        };
        chart_top + frac as f32 * chart_h
    };

    // ── bar width ────────────────────────────────────────────────────────────
    let n_bars = bars.len() as f32;
    let bar_w = (chart_rect.width() / n_bars).max(1.0);
    let candle_w = (bar_w * 0.7).max(1.0);
    let half_body = candle_w * 0.5;
    let render_step = chart_render_sample_step(bars.len(), chart_rect.width());
    let fill_half_w = (bar_w * render_step as f32 * 0.5).max(bar_w * 0.5);

    // ── session highlighting (Asian / London / New York) ────────────────────
    // Batched: find contiguous session blocks and draw one rect per block (not per bar).
    if flags.sessions {
        let session_asian = egui::Color32::from_rgba_premultiplied(40, 60, 120, 18);
        let session_london = egui::Color32::from_rgba_premultiplied(60, 120, 60, 18);
        let session_ny = egui::Color32::from_rgba_premultiplied(120, 60, 40, 18);
        let tf_minutes = chart.timeframe.minutes();
        if tf_minutes < 240 {
            // For each session, find contiguous blocks and draw one rect per block
            let sessions: &[(u32, u32, egui::Color32)] = &[
                (0, 540, session_asian),
                (420, 960, session_london),
                (810, 1200, session_ny),
            ];
            for &(start_hm, end_hm, color) in sessions {
                let mut block_start: Option<usize> = None;
                for i in 0..=bars.len() {
                    let in_session = if i < bars.len() {
                        let secs = bars[i].ts_ms / 1000;
                        let day_secs = ((secs % 86400) + 86400) % 86400;
                        let hm = (day_secs / 60) as u32;
                        hm >= start_hm && hm < end_hm
                    } else {
                        false
                    };
                    if in_session && block_start.is_none() {
                        block_start = Some(i);
                    } else if !in_session {
                        let bs = match block_start {
                            Some(v) => v,
                            None => continue,
                        };
                        let x1 = chart_rect.left() + bs as f32 * bar_w;
                        let x2 = (chart_rect.left() + i as f32 * bar_w).min(chart_rect.right());
                        painter.rect_filled(
                            egui::Rect::from_min_max(
                                egui::pos2(x1, chart_rect.top()),
                                egui::pos2(x2, chart_rect.bottom()),
                            ),
                            0.0,
                            color,
                        );
                        block_start = None;
                    }
                }
            }
        }
    }

    // ── grid lines (price) ──────────────────────────────────────────────────
    // Use one faint line per grid level. The old dotted grid emitted hundreds to
    // thousands of tiny line-segment shapes every frame on large charts, which is
    // pure UI overhead during drag/zoom. Solid low-alpha grid lines keep the same
    // spatial reference with a tiny, fixed primitive count.
    let grid_steps = 8;
    let grid_col = egui::Color32::from_rgb(33, 33, 33);
    let grid_stroke = egui::Stroke::new(0.5, grid_col);
    let mut label_buf = String::with_capacity(16); // reuse buffer across grid labels (avoids heap alloc per label per frame)
    for i in 0..=grid_steps {
        let p = price_min + (price_max - price_min) * (i as f64 / grid_steps as f64);
        let y = price_to_y(p);
        painter.line_segment(
            [
                egui::pos2(chart_rect.left(), y),
                egui::pos2(chart_rect.right(), y),
            ],
            grid_stroke,
        );
        format_price_buf(p, &mut label_buf);
        painter.text(
            egui::pos2(chart_rect.right() + 4.0, y),
            egui::Align2::LEFT_CENTER,
            &label_buf,
            egui::FontId::monospace(10.0),
            AXIS_TEXT,
        );
    }

    // ── grid lines (time) ────────────────────────────────────────────────────
    let time_step = ((80.0 / bar_w) as usize).max(1);
    for (rel_idx, bar) in bars.iter().enumerate().step_by(time_step) {
        let x = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
        painter.line_segment(
            [
                egui::pos2(x, chart_rect.top()),
                egui::pos2(x, chart_rect.bottom()),
            ],
            grid_stroke,
        );
        format_ts_buf(bar.ts_ms, chart.timeframe, &mut label_buf);
        painter.text(
            egui::pos2(x, chart_rect.bottom() + 2.0),
            egui::Align2::CENTER_TOP,
            &label_buf,
            egui::FontId::monospace(9.0),
            AXIS_TEXT,
        );
    }

    // ── MA ribbon fill (KAMA vs SMA200) — only when single-TF lines are visible ──
    if flags.sma200 && flags.kama && chart.mtf_sma.is_empty() && chart.multi_kama.is_empty() {
        for (rel_idx, _) in bars.iter().enumerate().step_by(render_step) {
            let abs_idx = start_idx + rel_idx;
            if abs_idx >= chart.sma200.len() || abs_idx >= chart.kama.len() {
                continue;
            }
            if let (Some(sma_v), Some(kama_v)) = (chart.sma200[abs_idx], chart.kama[abs_idx]) {
                let x = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
                let y_sma = price_to_y(sma_v);
                let y_kama = price_to_y(kama_v);
                let (top, bot) = if y_sma < y_kama {
                    (y_sma, y_kama)
                } else {
                    (y_kama, y_sma)
                };
                if top <= chart_rect.bottom() && bot >= chart_rect.top() {
                    let fill = if kama_v > sma_v {
                        egui::Color32::from_rgba_premultiplied(0, 180, 60, 18) // bullish green
                    } else {
                        egui::Color32::from_rgba_premultiplied(180, 40, 0, 18) // bearish red
                    };
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(x - fill_half_w, top.max(chart_rect.top())),
                            egui::pos2(x + fill_half_w, bot.min(chart_rect.bottom())),
                        ),
                        0.0,
                        fill,
                    );
                }
            }
        }
    }

    // ── Bollinger Band fill ──────────────────────────────────────────────────
    if flags.bollinger {
        // Build polygon directly: upper points forward, lower points reversed — no clone needed.
        // Dense views use the same pixel-aware decimation as line/candle rendering.
        let mut fill_points_upper: Vec<egui::Pos2> =
            Vec::with_capacity(bars.len() / render_step + 1);
        let mut fill_points_lower: Vec<egui::Pos2> =
            Vec::with_capacity(bars.len() / render_step + 1);
        for (rel_idx, _) in bars.iter().enumerate().step_by(render_step) {
            let abs_idx = start_idx + rel_idx;
            if abs_idx >= chart.bb_upper.len() {
                continue;
            }
            if let (Some(u), Some(l)) = (chart.bb_upper[abs_idx], chart.bb_lower[abs_idx]) {
                let x = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
                let yu = price_to_y(u);
                let yl = price_to_y(l);
                if yu >= chart_rect.top() && yl <= chart_rect.bottom() {
                    fill_points_upper.push(egui::pos2(x, yu));
                    fill_points_lower.push(egui::pos2(x, yl));
                }
            }
        }
        if fill_points_upper.len() > 1 {
            let mut poly = Vec::with_capacity(fill_points_upper.len() + fill_points_lower.len());
            poly.extend_from_slice(&fill_points_upper);
            poly.extend(fill_points_lower.iter().rev());
            painter.add(egui::Shape::convex_polygon(
                poly,
                BB_FILL,
                egui::Stroke::NONE,
            ));
        }
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.bb_upper,
            start_idx,
            bar_w,
            &price_to_y,
            BB_COL,
            1.0,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.bb_lower,
            start_idx,
            bar_w,
            &price_to_y,
            BB_COL,
            1.0,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.bb_mid,
            start_idx,
            bar_w,
            &price_to_y,
            BB_COL,
            0.5,
        );
    }

    // ── VWAP with deviation bands ───────────────────────────────────────────
    if flags.vwap {
        let vwap_col = egui::Color32::from_rgb(255, 215, 0); // gold
        let band_col1 = egui::Color32::from_rgba_premultiplied(100, 149, 237, 50); // cornflower blue
        let band_col2 = egui::Color32::from_rgba_premultiplied(100, 149, 237, 30);
        let band_col3 = egui::Color32::from_rgba_premultiplied(100, 149, 237, 15);
        // Fill bands (3σ first, then 2σ, then 1σ so inner is on top)
        for (upper, lower, fill_col) in [
            (&chart.vwap_upper3, &chart.vwap_lower3, band_col3),
            (&chart.vwap_upper2, &chart.vwap_lower2, band_col2),
            (&chart.vwap_upper1, &chart.vwap_lower1, band_col1),
        ] {
            for (rel_idx, _) in bars.iter().enumerate().step_by(render_step) {
                let abs_idx = start_idx + rel_idx;
                if abs_idx >= upper.len() {
                    continue;
                }
                if let (Some(u), Some(l)) = (upper[abs_idx], lower[abs_idx]) {
                    let x = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
                    let yu = price_to_y(u);
                    let yl = price_to_y(l);
                    let (top, bot) = if yu < yl { (yu, yl) } else { (yl, yu) };
                    if top <= chart_rect.bottom() && bot >= chart_rect.top() {
                        painter.rect_filled(
                            egui::Rect::from_min_max(
                                egui::pos2(x - fill_half_w, top.max(chart_rect.top())),
                                egui::pos2(x + fill_half_w, bot.min(chart_rect.bottom())),
                            ),
                            0.0,
                            fill_col,
                        );
                    }
                }
            }
        }
        // VWAP line
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.vwap,
            start_idx,
            bar_w,
            &price_to_y,
            vwap_col,
            2.0,
        );
        // Band edge lines
        let band_line = egui::Color32::from_rgba_premultiplied(100, 149, 237, 80);
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.vwap_upper1,
            start_idx,
            bar_w,
            &price_to_y,
            band_line,
            0.5,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.vwap_lower1,
            start_idx,
            bar_w,
            &price_to_y,
            band_line,
            0.5,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.vwap_upper2,
            start_idx,
            bar_w,
            &price_to_y,
            band_line,
            0.5,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.vwap_lower2,
            start_idx,
            bar_w,
            &price_to_y,
            band_line,
            0.5,
        );
    }

    // ── Supertrend ─────────────────────────────────────────────────────────
    if flags.supertrend {
        let st_bull_col = egui::Color32::from_rgb(0, 200, 100);
        let st_bear_col = egui::Color32::from_rgb(220, 50, 50);
        // Draw as colored segments — bull=green, bear=red
        let mut points: Vec<egui::Pos2> = Vec::with_capacity(bars.len() / render_step + 1);
        let mut prev_bull = true;
        for (rel_idx, _) in bars.iter().enumerate().step_by(render_step) {
            let abs_idx = start_idx + rel_idx;
            if abs_idx >= chart.supertrend.len() {
                continue;
            }
            if let Some(v) = chart.supertrend[abs_idx] {
                let x = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
                let y = price_to_y(v);
                let is_bull = chart.supertrend_bull.get(abs_idx).copied().unwrap_or(true);
                if is_bull != prev_bull && points.len() > 1 {
                    let col = if prev_bull { st_bull_col } else { st_bear_col };
                    painter.add(egui::Shape::line(
                        std::mem::take(&mut points),
                        egui::Stroke::new(2.0, col),
                    ));
                }
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    points.push(egui::pos2(x, y));
                }
                prev_bull = is_bull;
            }
        }
        if points.len() > 1 {
            let col = if prev_bull { st_bull_col } else { st_bear_col };
            painter.add(egui::Shape::line(points, egui::Stroke::new(2.0, col)));
        }
    }

    // ── Donchian Channels ────────────────────────────────────────────────
    if flags.donchian {
        let dc_col = egui::Color32::from_rgb(0, 180, 255);
        let dc_fill = egui::Color32::from_rgba_premultiplied(0, 180, 255, 15);
        // Fill between upper and lower
        for (rel_idx, _) in bars.iter().enumerate().step_by(render_step) {
            let abs_idx = start_idx + rel_idx;
            if abs_idx >= chart.donchian_upper.len() {
                continue;
            }
            if let (Some(u), Some(l)) =
                (chart.donchian_upper[abs_idx], chart.donchian_lower[abs_idx])
            {
                let x = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
                let yu = price_to_y(u);
                let yl = price_to_y(l);
                if yu <= chart_rect.bottom() && yl >= chart_rect.top() {
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(x - fill_half_w, yu.max(chart_rect.top())),
                            egui::pos2(x + fill_half_w, yl.min(chart_rect.bottom())),
                        ),
                        0.0,
                        dc_fill,
                    );
                }
            }
        }
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.donchian_upper,
            start_idx,
            bar_w,
            &price_to_y,
            dc_col,
            1.0,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.donchian_lower,
            start_idx,
            bar_w,
            &price_to_y,
            dc_col,
            1.0,
        );
    }

    // ── Keltner Channels ─────────────────────────────────────────────────
    if flags.keltner {
        let kc_col = egui::Color32::from_rgb(255, 165, 0); // orange
        let kc_fill = egui::Color32::from_rgba_premultiplied(255, 165, 0, 15);
        for (rel_idx, _) in bars.iter().enumerate().step_by(render_step) {
            let abs_idx = start_idx + rel_idx;
            if abs_idx >= chart.keltner_upper.len() {
                continue;
            }
            if let (Some(u), Some(l)) = (chart.keltner_upper[abs_idx], chart.keltner_lower[abs_idx])
            {
                let x = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
                let yu = price_to_y(u);
                let yl = price_to_y(l);
                if yu <= chart_rect.bottom() && yl >= chart_rect.top() {
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(x - fill_half_w, yu.max(chart_rect.top())),
                            egui::pos2(x + fill_half_w, yl.min(chart_rect.bottom())),
                        ),
                        0.0,
                        kc_fill,
                    );
                }
            }
        }
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.keltner_upper,
            start_idx,
            bar_w,
            &price_to_y,
            kc_col,
            1.0,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.keltner_lower,
            start_idx,
            bar_w,
            &price_to_y,
            kc_col,
            1.0,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.keltner_mid,
            start_idx,
            bar_w,
            &price_to_y,
            kc_col,
            0.5,
        );
    }

    // ── Regression Channel ─────────────────────────────────────────────────
    if flags.regression {
        let rc_col = egui::Color32::from_rgb(180, 130, 255); // light purple
        let rc_fill = egui::Color32::from_rgba_premultiplied(180, 130, 255, 15);
        for (rel_idx, _) in bars.iter().enumerate().step_by(render_step) {
            let abs_idx = start_idx + rel_idx;
            if abs_idx >= chart.regression_upper.len() {
                continue;
            }
            if let (Some(u), Some(l)) = (
                chart.regression_upper[abs_idx],
                chart.regression_lower[abs_idx],
            ) {
                let x = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
                let yu = price_to_y(u);
                let yl = price_to_y(l);
                if yu <= chart_rect.bottom() && yl >= chart_rect.top() {
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(x - fill_half_w, yu.max(chart_rect.top())),
                            egui::pos2(x + fill_half_w, yl.min(chart_rect.bottom())),
                        ),
                        0.0,
                        rc_fill,
                    );
                }
            }
        }
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.regression_upper,
            start_idx,
            bar_w,
            &price_to_y,
            rc_col,
            0.8,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.regression_lower,
            start_idx,
            bar_w,
            &price_to_y,
            rc_col,
            0.8,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.regression_mid,
            start_idx,
            bar_w,
            &price_to_y,
            rc_col,
            1.5,
        );
    }

    // ── Ichimoku cloud ─────────────────────────────────────────────────────
    if flags.ichimoku {
        // Cloud fill between Span A and Span B
        for (rel_idx, _) in bars.iter().enumerate().step_by(render_step) {
            let abs_idx = start_idx + rel_idx;
            if abs_idx >= chart.ichi_span_a.len() {
                continue;
            }
            if let (Some(a), Some(b)) = (chart.ichi_span_a[abs_idx], chart.ichi_span_b[abs_idx]) {
                let x = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
                let ya = price_to_y(a);
                let yb = price_to_y(b);
                let color = if a >= b {
                    ICHI_CLOUD_BULL
                } else {
                    ICHI_CLOUD_BEAR
                };
                let (top, bot) = if ya < yb { (ya, yb) } else { (yb, ya) };
                if top <= chart_rect.bottom() && bot >= chart_rect.top() {
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(x - fill_half_w, top.max(chart_rect.top())),
                            egui::pos2(x + fill_half_w, bot.min(chart_rect.bottom())),
                        ),
                        0.0,
                        color,
                    );
                }
            }
        }
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.ichi_tenkan,
            start_idx,
            bar_w,
            &price_to_y,
            ICHI_TENKAN,
            1.0,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.ichi_kijun,
            start_idx,
            bar_w,
            &price_to_y,
            ICHI_KIJUN,
            1.0,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.ichi_span_a,
            start_idx,
            bar_w,
            &price_to_y,
            ICHI_SPAN_A,
            0.8,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.ichi_span_b,
            start_idx,
            bar_w,
            &price_to_y,
            ICHI_SPAN_B,
            0.8,
        );
    }

    // ── indicator lines ──────────────────────────────────────────────────────
    // Current-TF SMA200: only show if NO MTF SMA data exists (MTF replaces it in NNFX mode)
    if flags.sma200 && chart.mtf_sma.is_empty() {
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.sma200,
            start_idx,
            bar_w,
            &price_to_y,
            SMA200_COL,
            1.5,
        );
    }
    if flags.sma100 {
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.sma100,
            start_idx,
            bar_w,
            &price_to_y,
            SMA100_COL,
            1.5,
        );
    }
    // Current-TF KAMA: only show if NO MultiKAMA HTF data exists
    if flags.kama && chart.multi_kama.is_empty() {
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.kama,
            start_idx,
            bar_w,
            &price_to_y,
            KAMA_COL,
            1.5,
        );
    }
    // MultiKAMA: higher TF KAMAs (MT5: clrWhite for KAMA, but visually distinguished)
    // MTF SMA lines (matching MTF_MA.mqh: H1/200, H4/200, D1/200, W1/200, W1/100, MN1/100)
    if flags.sma200 && !chart.mtf_sma.is_empty() {
        // Colors matching MTF_MA.mqh SetIndexStyle (lines 226-231)
        for (label, projected) in &chart.mtf_sma {
            let color = match label.as_str() {
                "H1 200" => egui::Color32::from_rgb(255, 99, 71), // clrTomato
                _ => egui::Color32::from_rgb(255, 0, 255),        // clrMagenta (all others)
            };
            let mut prev: Option<egui::Pos2> = None;
            for &(bar_idx, sma_val) in projected {
                if bar_idx >= start_idx && bar_idx < end_idx {
                    let rel = bar_idx - start_idx;
                    let x = chart_rect.left() + (rel as f32 + 0.5) * bar_w;
                    let y = price_to_y(sma_val);
                    if y >= chart_rect.top() && y <= chart_rect.bottom() {
                        let pt = egui::pos2(x, y);
                        if let Some(p) = prev {
                            painter.line_segment([p, pt], egui::Stroke::new(2.0, color));
                        }
                        prev = Some(pt);
                    } else {
                        prev = None;
                    }
                }
            }
        }
    }

    // MQL4 mode uses white for all; MTF_MA overlay uses magenta for higher TFs
    if flags.kama && !chart.multi_kama.is_empty() {
        // MultiKAMA: ALL WHITE (matching MT5 MultiKAMA.mqh SetIndexStyle lines 59-63)
        let htf_colors = [
            egui::Color32::from_rgb(255, 255, 255), // H1 — white (clrWhite)
            egui::Color32::from_rgb(255, 255, 255), // H4 — white (clrWhite)
            egui::Color32::from_rgb(255, 255, 255), // D1 — white (clrWhite)
            egui::Color32::from_rgb(255, 255, 255), // W1 — white (clrWhite)
            egui::Color32::from_rgb(255, 255, 255), // MN1 — white (clrWhite)
        ];
        for (tf_idx, (_tf_label, projected)) in chart.multi_kama.iter().enumerate() {
            let color = htf_colors
                .get(tf_idx)
                .copied()
                .unwrap_or(egui::Color32::from_rgb(255, 0, 255));
            let mut prev: Option<egui::Pos2> = None;
            for &(bar_idx, kama_val) in projected {
                if bar_idx >= start_idx && bar_idx < end_idx {
                    let rel = bar_idx - start_idx;
                    let x = chart_rect.left() + (rel as f32 + 0.5) * bar_w;
                    let y = price_to_y(kama_val);
                    if y >= chart_rect.top() && y <= chart_rect.bottom() {
                        let pt = egui::pos2(x, y);
                        if let Some(p) = prev {
                            painter.line_segment([p, pt], egui::Stroke::new(2.0, color));
                        }
                        prev = Some(pt);
                    } else {
                        prev = None;
                    }
                }
            }
        }
    }
    if flags.ema21 {
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.ema21,
            start_idx,
            bar_w,
            &price_to_y,
            EMA_COL,
            1.5,
        );
    }
    if flags.wma {
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.wma,
            start_idx,
            bar_w,
            &price_to_y,
            WMA_COL,
            1.0,
        );
    }
    if flags.hma {
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.hma,
            start_idx,
            bar_w,
            &price_to_y,
            HMA_COL,
            1.5,
        );
    }

    // ATR Projection — multi-timeframe horizontal levels (matching ATR_Projection.mqh).
    // Draw one clipped line primitive per level; dotted per-pixel segments were pure
    // tessellation/GPU-upload pressure and did not add price accuracy.
    if flags.atr_proj {
        let atr_yellow = egui::Color32::from_rgb(255, 255, 0); // clrYellow
        for &(label, htf_open, atr_val, line_start_idx) in &chart.atr_proj_levels {
            let upper_price = htf_open + atr_val;
            let lower_price = htf_open - atr_val;
            let x_start = if line_start_idx >= start_idx {
                chart_rect.left() + ((line_start_idx - start_idx) as f32) * bar_w
            } else {
                chart_rect.left()
            }
            .clamp(chart_rect.left(), chart_rect.right());
            let x_end = chart_rect.right();
            for (price, suffix) in [(upper_price, "Hi"), (lower_price, "Lo")] {
                let y = price_to_y(price);
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    painter.line_segment(
                        [egui::pos2(x_start, y), egui::pos2(x_end, y)],
                        egui::Stroke::new(1.5, atr_yellow),
                    );
                    // Label: "ATR D1 Hi 1.2345"
                    painter.text(
                        egui::pos2(x_start + 4.0, y - 10.0),
                        egui::Align2::LEFT_BOTTOM,
                        &format!("ATR {} {} {}", label, suffix, format_price(price)),
                        egui::FontId::monospace(8.0),
                        atr_yellow,
                    );
                }
            }
        }
    }

    // Parabolic SAR dots. Dense zoomed-out views cannot distinguish one dot per
    // historical bar, so sample at viewport density like candles/indicator lines.
    if flags.psar {
        for (rel_idx, _) in bars.iter().enumerate().step_by(render_step) {
            let abs_idx = start_idx + rel_idx;
            if abs_idx >= chart.psar.len() {
                continue;
            }
            if let Some(sar) = chart.psar[abs_idx] {
                let x = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
                let y = price_to_y(sar);
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    painter.circle_filled(egui::pos2(x, y), 2.0, SAR_COL);
                }
            }
        }
    }

    // ── Ehlers overlay indicators ───────────────────────────────────────────
    if flags.ehlers_ss {
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.ehlers_ss,
            start_idx,
            bar_w,
            &price_to_y,
            EHLERS_SS_COL,
            1.5,
        );
    }
    if flags.ehlers_decycler {
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.ehlers_decycler,
            start_idx,
            bar_w,
            &price_to_y,
            EHLERS_DEC_COL,
            1.5,
        );
    }
    if flags.ehlers_itl {
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.ehlers_itl,
            start_idx,
            bar_w,
            &price_to_y,
            EHLERS_ITL_COL,
            1.5,
        );
    }
    if flags.ehlers_mama {
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.ehlers_mama,
            start_idx,
            bar_w,
            &price_to_y,
            EHLERS_MAMA_COL,
            1.5,
        );
        draw_indicator_line(
            painter,
            chart_rect,
            bars,
            &chart.ehlers_fama,
            start_idx,
            bar_w,
            &price_to_y,
            EHLERS_FAMA_COL,
            1.0,
        );
    }

    // ── previous candle levels ─────────────────────────────────────────────
    if flags.prev_levels {
        // Matching PreviousCandleLevels.mqh — White for previous, per-TF colors
        let level_pairs = [
            (
                chart.prev_h1_high,
                "H1 Hi",
                egui::Color32::from_rgb(180, 180, 180),
            ),
            (
                chart.prev_h1_low,
                "H1 Lo",
                egui::Color32::from_rgb(180, 180, 180),
            ),
            (
                chart.prev_h4_high,
                "H4 Hi",
                egui::Color32::from_rgb(200, 200, 200),
            ),
            (
                chart.prev_h4_low,
                "H4 Lo",
                egui::Color32::from_rgb(200, 200, 200),
            ),
            (chart.prev_daily_high, "D Hi", egui::Color32::WHITE),
            (chart.prev_daily_low, "D Lo", egui::Color32::WHITE),
            (
                chart.prev_weekly_high,
                "W Hi",
                egui::Color32::from_rgb(255, 0, 255),
            ), // Magenta
            (
                chart.prev_weekly_low,
                "W Lo",
                egui::Color32::from_rgb(255, 0, 255),
            ),
            (
                chart.prev_monthly_high,
                "MN Hi",
                egui::Color32::from_rgb(255, 0, 255),
            ),
            (
                chart.prev_monthly_low,
                "MN Lo",
                egui::Color32::from_rgb(255, 0, 255),
            ),
        ];
        for (price_opt, label, color) in &level_pairs {
            if let Some(p) = price_opt {
                let y = price_to_y(*p);
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    painter.line_segment(
                        [
                            egui::pos2(chart_rect.left(), y),
                            egui::pos2(chart_rect.right(), y),
                        ],
                        egui::Stroke::new(0.5, *color),
                    );
                    painter.text(
                        egui::pos2(chart_rect.right() - 40.0, y - 10.0),
                        egui::Align2::LEFT_TOP,
                        label,
                        egui::FontId::monospace(8.0),
                        *color,
                    );
                }
            }
        }
    }

    // ── pivot points ──────────────────────────────────────────────────────
    if flags.pivots {
        let pivot_levels = [
            (chart.pivot_p, "P", egui::Color32::from_rgb(200, 200, 200)),
            (chart.pivot_r1, "R1", egui::Color32::from_rgb(200, 80, 80)),
            (chart.pivot_r2, "R2", egui::Color32::from_rgb(255, 40, 40)),
            (chart.pivot_s1, "S1", egui::Color32::from_rgb(80, 200, 80)),
            (chart.pivot_s2, "S2", egui::Color32::from_rgb(40, 255, 40)),
        ];
        for (price_opt, label, color) in &pivot_levels {
            if let Some(p) = price_opt {
                let y = price_to_y(*p);
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    painter.line_segment(
                        [
                            egui::pos2(chart_rect.left(), y),
                            egui::pos2(chart_rect.right(), y),
                        ],
                        egui::Stroke::new(0.7, *color),
                    );
                    painter.text(
                        egui::pos2(chart_rect.left() + 2.0, y - 10.0),
                        egui::Align2::LEFT_TOP,
                        label,
                        egui::FontId::monospace(8.0),
                        *color,
                    );
                }
            }
        }
    }

    // ── fractals ─────────────────────────────────────────────────────────
    if flags.fractals {
        // Market structure: track prev swing high/low to label HH/HL/LH/LL
        let mut prev_swing_high: Option<f64> = None;
        let mut prev_swing_low: Option<f64> = None;
        // Scan all bars up to visible end to get accurate structure context
        let scan_start = start_idx.saturating_sub(50); // look back for prior swings
        for si in scan_start..start_idx {
            if si < chart.fractal_up.len() && chart.fractal_up[si] {
                prev_swing_high = Some(chart.bars[si].high);
            }
            if si < chart.fractal_down.len() && chart.fractal_down[si] {
                prev_swing_low = Some(chart.bars[si].low);
            }
        }
        let ms_font = egui::FontId::monospace(8.0);
        let fractal_font = egui::FontId::proportional(10.0);
        let min_structure_label_gap = if render_step > 1 { 12.0 } else { 0.0 };
        let mut last_high_label_x = f32::NEG_INFINITY;
        let mut last_low_label_x = f32::NEG_INFINITY;
        for (rel_idx, bar) in bars.iter().enumerate() {
            let abs_idx = start_idx + rel_idx;
            let x = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
            if abs_idx < chart.fractal_up.len() && chart.fractal_up[abs_idx] {
                let y = price_to_y(bar.high) - 8.0;
                if y >= chart_rect.top() && x - last_high_label_x >= min_structure_label_gap {
                    painter.text(
                        egui::pos2(x, y),
                        egui::Align2::CENTER_BOTTOM,
                        "▲",
                        fractal_font.clone(),
                        UP,
                    );
                    // Market structure label
                    if let Some(prev_h) = prev_swing_high {
                        let (label, col) = if bar.high > prev_h {
                            ("HH", UP)
                        } else {
                            ("LH", DOWN)
                        };
                        painter.text(
                            egui::pos2(x, y - 10.0),
                            egui::Align2::CENTER_BOTTOM,
                            label,
                            ms_font.clone(),
                            col,
                        );
                    }
                    last_high_label_x = x;
                }
                prev_swing_high = Some(bar.high);
            }
            if abs_idx < chart.fractal_down.len() && chart.fractal_down[abs_idx] {
                let y = price_to_y(bar.low) + 2.0;
                if y <= chart_rect.bottom() && x - last_low_label_x >= min_structure_label_gap {
                    painter.text(
                        egui::pos2(x, y),
                        egui::Align2::CENTER_TOP,
                        "▼",
                        fractal_font.clone(),
                        DOWN,
                    );
                    if let Some(prev_l) = prev_swing_low {
                        let (label, col) = if bar.low > prev_l {
                            ("HL", UP)
                        } else {
                            ("LL", DOWN)
                        };
                        painter.text(
                            egui::pos2(x, y + 10.0),
                            egui::Align2::CENTER_TOP,
                            label,
                            ms_font.clone(),
                            col,
                        );
                    }
                    last_low_label_x = x;
                }
                prev_swing_low = Some(bar.low);
            }
        }
    }

    // ── harmonic patterns (Scott Carney XABCD) ────────────────────────────
    if flags.harmonics {
        let pattern_col = egui::Color32::from_rgb(0, 200, 255);
        let tp_col = egui::Color32::from_rgb(0, 200, 80);
        let sl_col = egui::Color32::from_rgb(220, 40, 40);
        for pat in &chart.harmonics {
            let pts = [pat.x, pat.a, pat.b, pat.c, pat.d];
            let screen_pts = pts.map(|(idx, price)| {
                if idx >= start_idx && idx < end_idx {
                    Some(egui::pos2(
                        chart_rect.left() + ((idx - start_idx) as f32 + 0.5) * bar_w,
                        price_to_y(price),
                    ))
                } else {
                    None
                }
            });
            // XABCD lines
            for w in screen_pts.windows(2) {
                if let (Some(p1), Some(p2)) = (w[0], w[1]) {
                    painter.line_segment([p1, p2], egui::Stroke::new(1.5, pattern_col));
                }
            }
            // Labels
            let labels = ["X", "A", "B", "C", "D"];
            for (i, sp) in screen_pts.iter().enumerate() {
                if let Some(p) = sp {
                    painter.text(
                        egui::pos2(p.x, p.y + if i % 2 == 0 { -12.0 } else { 4.0 }),
                        egui::Align2::CENTER_TOP,
                        labels[i],
                        egui::FontId::monospace(10.0),
                        pattern_col,
                    );
                }
            }
            // Pattern name
            if let Some(d_pt) = screen_pts[4] {
                let dir = if pat.bullish { "BULL" } else { "BEAR" };
                let col = if pat.bullish { UP } else { DOWN };
                painter.text(
                    egui::pos2(d_pt.x + 5.0, d_pt.y - 20.0),
                    egui::Align2::LEFT_TOP,
                    &format!("{} {}", pat.name, dir),
                    egui::FontId::monospace(9.0),
                    col,
                );
                // TP/SL from D
                for (price, label, c) in [
                    (pat.tp1, "TP1", tp_col),
                    (pat.tp2, "TP2", tp_col),
                    (pat.sl, "SL", sl_col),
                ] {
                    let y = price_to_y(price);
                    if y >= chart_rect.top() && y <= chart_rect.bottom() {
                        painter.line_segment(
                            [egui::pos2(d_pt.x, y), egui::pos2(chart_rect.right(), y)],
                            egui::Stroke::new(0.7, c),
                        );
                        painter.text(
                            egui::pos2(d_pt.x + 2.0, y - 9.0),
                            egui::Align2::LEFT_TOP,
                            &format!("{} {}", label, format_price(price)),
                            egui::FontId::monospace(8.0),
                            c,
                        );
                    }
                }
            }
        }
    }

    // ── supply/demand zones ─────────────────────────────────────────────────
    if flags.supply_demand {
        let status_label = |s: u8| -> &str {
            match s {
                0 => "Untested",
                1 => "Tested",
                2 => "Proven",
                _ => "",
            }
        };
        // Zones extend from their creation bar to the chart right edge (matching MT5).
        // Show any zone whose creation bar is <= end_idx (it extends into or past the view).
        // Demand zones — MT5 colors: DarkSeaGreen/MediumSeaGreen/SeaGreen
        for &(idx, zh, zl, status) in &chart.demand_zones {
            if idx < end_idx {
                let x_start = if idx >= start_idx {
                    chart_rect.left() + ((idx - start_idx) as f32) * bar_w
                } else {
                    chart_rect.left()
                };
                let y_top = price_to_y(zh);
                let y_bot = price_to_y(zl);
                if y_bot >= chart_rect.top() && y_top <= chart_rect.bottom() {
                    let (fill_col, label_col) = match status {
                        0 => (
                            egui::Color32::from_rgba_premultiplied(143, 188, 143, 50),
                            egui::Color32::from_rgb(143, 188, 143),
                        ),
                        1 => (
                            egui::Color32::from_rgba_premultiplied(60, 179, 113, 60),
                            egui::Color32::from_rgb(60, 179, 113),
                        ),
                        _ => (
                            egui::Color32::from_rgba_premultiplied(46, 139, 87, 70),
                            egui::Color32::from_rgb(46, 139, 87),
                        ),
                    };
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(x_start, y_top.max(chart_rect.top())),
                            egui::pos2(chart_rect.right(), y_bot.min(chart_rect.bottom())),
                        ),
                        0.0,
                        fill_col,
                    );
                    painter.text(
                        egui::pos2(
                            chart_rect.right() - 4.0,
                            y_bot.min(chart_rect.bottom()) - 12.0,
                        ),
                        egui::Align2::RIGHT_TOP,
                        &format!("Demand [{}]", status_label(status)),
                        egui::FontId::monospace(9.0),
                        label_col,
                    );
                }
            }
        }
        // Supply zones — MT5 colors: SkyBlue/DeepSkyBlue/DodgerBlue
        for &(idx, zh, zl, status) in &chart.supply_zones {
            if idx < end_idx {
                let x_start = if idx >= start_idx {
                    chart_rect.left() + ((idx - start_idx) as f32) * bar_w
                } else {
                    chart_rect.left()
                };
                let y_top = price_to_y(zh);
                let y_bot = price_to_y(zl);
                if y_bot >= chart_rect.top() && y_top <= chart_rect.bottom() {
                    let (fill_col, label_col) = match status {
                        0 => (
                            egui::Color32::from_rgba_premultiplied(135, 206, 235, 50),
                            egui::Color32::from_rgb(135, 206, 235),
                        ),
                        1 => (
                            egui::Color32::from_rgba_premultiplied(0, 191, 255, 60),
                            egui::Color32::from_rgb(0, 191, 255),
                        ),
                        _ => (
                            egui::Color32::from_rgba_premultiplied(30, 144, 255, 70),
                            egui::Color32::from_rgb(30, 144, 255),
                        ),
                    };
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(x_start, y_top.max(chart_rect.top())),
                            egui::pos2(chart_rect.right(), y_bot.min(chart_rect.bottom())),
                        ),
                        0.0,
                        fill_col,
                    );
                    painter.text(
                        egui::pos2(chart_rect.right() - 4.0, y_top.max(chart_rect.top()) + 2.0),
                        egui::Align2::RIGHT_TOP,
                        &format!("Supply [{}]", status_label(status)),
                        egui::FontId::monospace(9.0),
                        label_col,
                    );
                }
            }
        }
    }

    // ── Fair Value Gaps (3-bar imbalance zones) ────────────────────────────
    if flags.fvg && bars.len() >= 3 {
        let fvg_bull = egui::Color32::from_rgba_premultiplied(0, 180, 80, 30);
        let fvg_bear = egui::Color32::from_rgba_premultiplied(220, 50, 50, 30);
        let fvg_bull_edge = egui::Color32::from_rgba_premultiplied(0, 180, 80, 80);
        let fvg_bear_edge = egui::Color32::from_rgba_premultiplied(220, 50, 50, 80);
        // Suffix arrays make the "has this gap been filled?" lookup O(1).
        // future_min_low[k] = min(bars[k..].low); future_max_high[k] = max(bars[k..].high).
        // The previous code scanned bars[i+2..] for each FVG candidate (O(n²) per frame
        // — pricey on dense charts and unnecessary when only the suffix extremes matter).
        let n = bars.len();
        let mut future_min_low: Vec<f64> = vec![f64::INFINITY; n + 1];
        let mut future_max_high: Vec<f64> = vec![f64::NEG_INFINITY; n + 1];
        for k in (0..n).rev() {
            future_min_low[k] = future_min_low[k + 1].min(bars[k].low);
            future_max_high[k] = future_max_high[k + 1].max(bars[k].high);
        }
        for i in 1..n.saturating_sub(1) {
            let prev = &bars[i - 1];
            let next = &bars[i + 1];
            let x_start = chart_rect.left() + ((i + 1) as f32 + 0.5) * bar_w;
            let x_end = chart_rect.right();
            let scan_start = (i + 2).min(n);
            // Bullish FVG: bar[i+1].low > bar[i-1].high (gap up)
            if next.low > prev.high {
                let gap_top = price_to_y(next.low);
                let gap_bot = price_to_y(prev.high);
                if gap_top <= chart_rect.bottom() && gap_bot >= chart_rect.top() {
                    let filled = future_min_low[scan_start] <= prev.high;
                    if !filled {
                        painter.rect_filled(
                            egui::Rect::from_min_max(
                                egui::pos2(x_start, gap_top.max(chart_rect.top())),
                                egui::pos2(x_end, gap_bot.min(chart_rect.bottom())),
                            ),
                            0.0,
                            fvg_bull,
                        );
                        painter.line_segment(
                            [egui::pos2(x_start, gap_top), egui::pos2(x_end, gap_top)],
                            egui::Stroke::new(0.5, fvg_bull_edge),
                        );
                        painter.line_segment(
                            [egui::pos2(x_start, gap_bot), egui::pos2(x_end, gap_bot)],
                            egui::Stroke::new(0.5, fvg_bull_edge),
                        );
                    }
                }
            }
            // Bearish FVG: bar[i+1].high < bar[i-1].low (gap down)
            if next.high < prev.low {
                let gap_top = price_to_y(prev.low);
                let gap_bot = price_to_y(next.high);
                if gap_top <= chart_rect.bottom() && gap_bot >= chart_rect.top() {
                    let filled = future_max_high[scan_start] >= prev.low;
                    if !filled {
                        painter.rect_filled(
                            egui::Rect::from_min_max(
                                egui::pos2(x_start, gap_top.max(chart_rect.top())),
                                egui::pos2(x_end, gap_bot.min(chart_rect.bottom())),
                            ),
                            0.0,
                            fvg_bear,
                        );
                        painter.line_segment(
                            [egui::pos2(x_start, gap_top), egui::pos2(x_end, gap_top)],
                            egui::Stroke::new(0.5, fvg_bear_edge),
                        );
                        painter.line_segment(
                            [egui::pos2(x_start, gap_bot), egui::pos2(x_end, gap_bot)],
                            egui::Stroke::new(0.5, fvg_bear_edge),
                        );
                    }
                }
            }
        }
    }

    // ── Order Blocks (ICT/Smart Money) ──────────────────────────────────────
    // Bullish OB: last bearish candle before a strong bullish move (next close > current high + 1 ATR)
    // Bearish OB: last bullish candle before a strong bearish move (next close < current low - 1 ATR)
    if flags.order_blocks && bars.len() >= 3 {
        let ob_bull_fill = egui::Color32::from_rgba_premultiplied(0, 180, 160, 25);
        let ob_bull_edge = egui::Color32::from_rgba_premultiplied(0, 180, 160, 100);
        let ob_bear_fill = egui::Color32::from_rgba_premultiplied(220, 50, 50, 25);
        let ob_bear_edge = egui::Color32::from_rgba_premultiplied(220, 50, 50, 100);
        let ob_label_col = egui::Color32::from_rgba_premultiplied(200, 200, 200, 180);

        // Compute rolling ATR(14) for impulsive move threshold. Keep the early-bar
        // behavior unchanged, but avoid recomputing the 14-bar true-range window
        // for every bar on provider-maximum histories.
        let atr_period = 14usize;
        let mut true_ranges: Vec<f64> = Vec::with_capacity(bars.len());
        let mut local_atr: Vec<f64> = Vec::with_capacity(bars.len());
        let mut rolling_sum = 0.0;
        for i in 0..bars.len() {
            let bar = &bars[i];
            let tr = if i == 0 {
                bar.high - bar.low
            } else {
                let prev_close = bars[i - 1].close;
                let hl = bar.high - bar.low;
                let hc = (bar.high - prev_close).abs();
                let lc = (bar.low - prev_close).abs();
                hl.max(hc).max(lc)
            };
            true_ranges.push(tr);
            rolling_sum += tr;
            if i >= atr_period {
                rolling_sum -= true_ranges[i - atr_period];
                local_atr.push(rolling_sum / atr_period as f64);
            } else {
                local_atr.push(bar.high - bar.low);
            }
        }

        // Collect order blocks (limit to most recent 20)
        struct OBZone {
            high: f64,
            low: f64,
            bar_idx: usize,
            is_bull: bool,
            end_idx: usize,
        }
        let mut zones: Vec<OBZone> = Vec::with_capacity(20);

        // Walk newest-to-oldest and stop once the render cap is full. The old path
        // scanned every bar, built every historical OB, then drained the front just
        // to keep the last 20. On provider-maximum histories that did wasted work
        // proportional to the full cache depth on every chart render.
        for i in (0..bars.len().saturating_sub(1)).rev() {
            let cur = &bars[i];
            let nxt = &bars[i + 1];
            let atr = local_atr[i];
            if atr <= 0.0 {
                continue;
            }

            // Bullish OB: bearish candle, then next close breaks above current high by >= 1 ATR
            if cur.close < cur.open && nxt.close > cur.high + atr {
                let mut end = bars.len();
                for j in (i + 2)..bars.len() {
                    if bars[j].low <= cur.high {
                        end = j;
                        break;
                    }
                }
                zones.push(OBZone {
                    high: cur.high,
                    low: cur.low,
                    bar_idx: i,
                    is_bull: true,
                    end_idx: end,
                });
            }

            // Bearish OB: bullish candle, then next close breaks below current low by >= 1 ATR
            if cur.close > cur.open && nxt.close < cur.low - atr {
                let mut end = bars.len();
                for j in (i + 2)..bars.len() {
                    if bars[j].high >= cur.low {
                        end = j;
                        break;
                    }
                }
                zones.push(OBZone {
                    high: cur.high,
                    low: cur.low,
                    bar_idx: i,
                    is_bull: false,
                    end_idx: end,
                });
            }

            if zones.len() >= 20 {
                break;
            }
        }
        zones.reverse();

        for ob in &zones {
            let x_start = chart_rect.left() + (ob.bar_idx as f32 + 0.5) * bar_w;
            let x_end = if ob.end_idx >= bars.len() {
                chart_rect.right()
            } else {
                chart_rect.left() + (ob.end_idx as f32 + 0.5) * bar_w
            };
            if x_end < chart_rect.left() || x_start > chart_rect.right() {
                continue;
            }

            let y_top = price_to_y(ob.high);
            let y_bot = price_to_y(ob.low);
            if y_top > chart_rect.bottom() || y_bot < chart_rect.top() {
                continue;
            }

            let (fill, edge) = if ob.is_bull {
                (ob_bull_fill, ob_bull_edge)
            } else {
                (ob_bear_fill, ob_bear_edge)
            };
            let ct = y_top.max(chart_rect.top());
            let cb = y_bot.min(chart_rect.bottom());
            let cl = x_start.max(chart_rect.left());
            let cr = x_end.min(chart_rect.right());

            painter.rect_filled(
                egui::Rect::from_min_max(egui::pos2(cl, ct), egui::pos2(cr, cb)),
                0.0,
                fill,
            );
            painter.line_segment(
                [egui::pos2(cl, ct), egui::pos2(cr, ct)],
                egui::Stroke::new(0.7, edge),
            );
            painter.line_segment(
                [egui::pos2(cl, cb), egui::pos2(cr, cb)],
                egui::Stroke::new(0.7, edge),
            );
            // "OB" label
            if cr - cl > 20.0 {
                painter.text(
                    egui::pos2(cl + 3.0, ct + 1.0),
                    egui::Align2::LEFT_TOP,
                    if ob.is_bull { "OB+" } else { "OB-" },
                    egui::FontId::monospace(9.0),
                    ob_label_col,
                );
            }
        }
    }

    // ── Auto Fibonacci levels (matching AutoFibonacci.mqh) ─────────────────
    if flags.auto_fib && !chart.auto_fib_levels.is_empty() {
        for (price, label, is_ext) in &chart.auto_fib_levels {
            let y = price_to_y(*price);
            if y >= chart_rect.top() && y <= chart_rect.bottom() {
                // One clipped line per level. Dotted Fib levels used to emit a
                // per-pixel segment loop, which is bad for dense adaptive-sync
                // repaint. Keep the exact level, drop the decorative primitive spam.
                let color = if *is_ext {
                    egui::Color32::from_rgb(30, 144, 255) // clrDodgerBlue
                } else {
                    egui::Color32::from_rgb(255, 215, 0) // clrGold
                };
                painter.line_segment(
                    [
                        egui::pos2(chart_rect.left(), y),
                        egui::pos2(chart_rect.right(), y),
                    ],
                    egui::Stroke::new(1.0, color),
                );
                // Label on right
                let mut fib_label = String::with_capacity(label.len() + 24);
                fib_label.push_str(label);
                fib_label.push(' ');
                fib_label.push_str(&format_price(*price));
                painter.text(
                    egui::pos2(chart_rect.right() - 4.0, y - 1.0),
                    egui::Align2::RIGHT_BOTTOM,
                    fib_label,
                    egui::FontId::monospace(8.0),
                    color,
                );
            }
        }
        // Draw swing line
        if let Some((_high, _low, hi_idx, lo_idx)) = chart.auto_fib_swing {
            if hi_idx >= start_idx && hi_idx < end_idx && lo_idx >= start_idx && lo_idx < end_idx {
                let x1 = chart_rect.left() + ((hi_idx - start_idx) as f32 + 0.5) * bar_w;
                let y1 = price_to_y(_high);
                let x2 = chart_rect.left() + ((lo_idx - start_idx) as f32 + 0.5) * bar_w;
                let y2 = price_to_y(_low);
                painter.line_segment(
                    [egui::pos2(x1, y1), egui::pos2(x2, y2)],
                    egui::Stroke::new(1.0, egui::Color32::WHITE),
                );
            }
        }
    }

    // ── price data (possibly Heikin-Ashi transformed) ──────────────────────
    let ha_bars;
    let renko_bars;
    let render_bars: &[Bar] = match chart.chart_type {
        ChartType::HeikinAshi => {
            ha_bars = heikin_ashi(bars);
            &ha_bars
        }
        ChartType::Renko => {
            renko_bars = renko_bricks(bars);
            &renko_bars
        }
        _ => bars,
    };

    // ── draw bars (candle/HA/line/OHLC) ──────────────────────────────────
    match chart.chart_type {
        ChartType::Line => {
            // Line chart: polyline through close prices. Downsample when the view
            // contains more bars than horizontal pixels can distinguish; drawing
            // tens of thousands of sub-pixel vertices only adds tessellation work.
            let mut points: Vec<egui::Pos2> = Vec::with_capacity(bars.len() / render_step + 1);
            for (rel_idx, bar) in bars.iter().enumerate().step_by(render_step) {
                let x = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
                let y = price_to_y(bar.close);
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    points.push(egui::pos2(x, y));
                }
            }
            if points.len() > 1 {
                painter.add(egui::Shape::line(points, egui::Stroke::new(1.5, ACCENT)));
            }
        }
        ChartType::OhlcBars => {
            // OHLC Bars: vertical wick + left tick (open) + right tick (close)
            for (rel_idx, bar) in bars.iter().enumerate().step_by(render_step) {
                let cx = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
                let y_open = price_to_y(bar.open);
                let y_high = price_to_y(bar.high);
                let y_low = price_to_y(bar.low);
                let y_close = price_to_y(bar.close);
                let is_wknd = chart.gap_fill_timestamps.contains(&bar.ts_ms);
                let color = if is_wknd {
                    if bar.close >= bar.open {
                        egui::Color32::from_rgb(255, 0, 255)
                    } else {
                        egui::Color32::from_rgb(180, 0, 180)
                    }
                } else {
                    if bar.close >= bar.open { UP } else { DOWN }
                };
                let tick = half_body.max(2.0);

                // Vertical line
                painter.line_segment(
                    [egui::pos2(cx, y_high), egui::pos2(cx, y_low)],
                    egui::Stroke::new(1.0, color),
                );
                // Open tick (left)
                painter.line_segment(
                    [egui::pos2(cx - tick, y_open), egui::pos2(cx, y_open)],
                    egui::Stroke::new(1.0, color),
                );
                // Close tick (right)
                painter.line_segment(
                    [egui::pos2(cx, y_close), egui::pos2(cx + tick, y_close)],
                    egui::Stroke::new(1.0, color),
                );
            }
        }
        ChartType::Candle | ChartType::HeikinAshi | ChartType::Renko => {
            let weekend_up = egui::Color32::from_rgb(255, 0, 255); // magenta bull (gap-fill/weekend)
            let weekend_dn = egui::Color32::from_rgb(180, 0, 180); // dark magenta bear (weekend gap-fill)
            // Volume heatmap uses pre-computed vol_avg_20 from ChartState (no per-frame alloc)
            let vol_avg = &chart.vol_avg_20;
            for (rel_idx, bar) in render_bars.iter().enumerate().step_by(render_step) {
                let cx = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
                let y_open = price_to_y(bar.open);
                let y_high = price_to_y(bar.high);
                let y_low = price_to_y(bar.low);
                let y_close = price_to_y(bar.close);
                // Gap-fill bars (Kraken) get magenta color.
                // Use explicit timestamp tracking rather than day-of-week:
                // MT5 uses UTC+2 so Saturday 00:00 MT5 = Friday 22:00 UTC — day-of-week is unreliable.
                let is_weekend = chart.gap_fill_timestamps.contains(&bar.ts_ms);
                let color = if flags.vol_heatmap && !vol_avg.is_empty() {
                    // Volume heatmap: blue (low) → green → yellow → red (high)
                    let abs_idx = start_idx + rel_idx;
                    let avg = if abs_idx < vol_avg.len() && vol_avg[abs_idx] > 0.0 {
                        vol_avg[abs_idx]
                    } else {
                        1.0
                    };
                    let ratio = (bar.volume / avg).min(3.0) / 3.0; // 0..1, capped at 3x avg
                    if ratio < 0.33 {
                        // Blue to green
                        let t = ratio / 0.33;
                        let r = (40.0 * (1.0 - t)) as u8;
                        let g = (80.0 + 140.0 * t) as u8;
                        let b = (200.0 * (1.0 - t)) as u8;
                        egui::Color32::from_rgb(r, g, b)
                    } else if ratio < 0.66 {
                        // Green to yellow
                        let t = (ratio - 0.33) / 0.33;
                        let r = (220.0 * t) as u8;
                        let g = (220.0 - 30.0 * t) as u8;
                        egui::Color32::from_rgb(r, g, 0)
                    } else {
                        // Yellow to red
                        let t = (ratio - 0.66) / 0.34;
                        let g = (190.0 * (1.0 - t)) as u8;
                        egui::Color32::from_rgb(230, g, 0)
                    }
                } else if is_weekend {
                    if bar.close >= bar.open {
                        weekend_up
                    } else {
                        weekend_dn
                    }
                } else if chart.primary_first_ts > 0 && bar.ts_ms < chart.primary_first_ts {
                    // Backfill data (older than primary source) — same magenta as weekend
                    if bar.close >= bar.open {
                        weekend_up
                    } else {
                        weekend_dn
                    }
                } else {
                    if bar.close >= bar.open { UP } else { DOWN }
                };

                // Wick
                painter.line_segment(
                    [egui::pos2(cx, y_high), egui::pos2(cx, y_low)],
                    egui::Stroke::new(1.0, color),
                );

                // Body
                let body_top = y_open.min(y_close);
                let body_bottom = y_open.max(y_close);
                let body_height = (body_bottom - body_top).max(1.0);
                let body_rect = egui::Rect::from_min_size(
                    egui::pos2(cx - half_body, body_top),
                    egui::vec2(candle_w, body_height),
                );

                if body_height > 2.0 {
                    // Solid filled candles (TradingView/lightweight-charts style)
                    painter.rect_filled(body_rect, 0.0, color);
                } else {
                    // Doji: single line
                    painter.line_segment(
                        [
                            egui::pos2(cx - half_body, body_top),
                            egui::pos2(cx + half_body, body_top),
                        ],
                        egui::Stroke::new(1.0, color),
                    );
                }
            }
        }
    }

    // ── Extended Hours Candle (magenta, TradingView-style) ─────────────
    // When pre/post market data is available, draw a real ext hours candle.
    // Otherwise, draw a ghost placeholder.
    // Position flush to the right edge of the chart (no reserved slot in bar_w math,
    // so we clamp into chart_rect instead of reserving whitespace and shifting all bars).
    if let Some(last) = bars.last() {
        let next_x = (chart_rect.right() - half_body - 2.0).max(chart_rect.left() + bar_w);
        if next_x > chart_rect.left() + bar_w {
            if chart.ext_active && chart.ext_high > 0.0 {
                // Real extended hours candle (magenta)
                let ext_col = egui::Color32::from_rgb(200, 50, 200); // Magenta
                let y_open = price_to_y(chart.ext_open);
                let y_high = price_to_y(chart.ext_high);
                let y_low = price_to_y(chart.ext_low);
                let y_close = price_to_y(chart.ext_close);
                // Wick
                painter.line_segment(
                    [egui::pos2(next_x, y_high), egui::pos2(next_x, y_low)],
                    egui::Stroke::new(1.0, ext_col),
                );
                // Body
                let body_top = y_open.min(y_close);
                let body_h = (y_open - y_close).abs().max(1.0);
                let body_rect = egui::Rect::from_min_size(
                    egui::pos2(next_x - half_body, body_top),
                    egui::vec2(candle_w, body_h),
                );
                if body_h > 2.0 {
                    painter.rect_filled(body_rect, 0.0, ext_col);
                } else {
                    painter.line_segment(
                        [
                            egui::pos2(next_x - half_body, body_top),
                            egui::pos2(next_x + half_body, body_top),
                        ],
                        egui::Stroke::new(1.0, ext_col),
                    );
                }
            } else {
                // Ghost candle (no ext data — regular hours)
                let ghost_col = egui::Color32::from_rgba_premultiplied(100, 100, 120, 80);
                let ghost_close = last.close;
                let ghost_open = last.close;
                let ghost_high = last.close + (last.high - last.low) * 0.3;
                let ghost_low = last.close - (last.high - last.low) * 0.3;
                let y_open = price_to_y(ghost_open);
                let y_high = price_to_y(ghost_high);
                let y_low = price_to_y(ghost_low);
                let y_close = price_to_y(ghost_close);
                painter.line_segment(
                    [egui::pos2(next_x, y_high), egui::pos2(next_x, y_low)],
                    egui::Stroke::new(1.0, ghost_col),
                );
                let body_top = y_open.min(y_close);
                let body_h = (y_open - y_close).abs().max(2.0);
                let body_rect = egui::Rect::from_min_size(
                    egui::pos2(next_x - half_body, body_top),
                    egui::vec2(candle_w, body_h),
                );
                painter.rect_stroke(
                    body_rect,
                    0.0,
                    egui::Stroke::new(1.0, ghost_col),
                    egui::StrokeKind::Outside,
                );
            }
        }
    }

    // ── last price line ──────────────────────────────────────────────────────
    if let Some(last) = bars.last() {
        let y = price_to_y(last.close);
        if y >= chart_rect.top() && y <= chart_rect.bottom() {
            let color = if last.close >= last.open { UP } else { DOWN };
            // Dashed line
            let dash_len = 6.0_f32;
            let mut x = chart_rect.left();
            while x < chart_rect.right() {
                let end = (x + dash_len).min(chart_rect.right());
                painter.line_segment(
                    [egui::pos2(x, y), egui::pos2(end, y)],
                    egui::Stroke::new(1.0, color),
                );
                x += dash_len * 2.0;
            }
            // Price label background
            let label = format_price(last.close);
            let lbl_rect = egui::Rect::from_min_size(
                egui::pos2(chart_rect.right() + 2.0, y - 8.0),
                egui::vec2(price_axis_w - 4.0, 16.0),
            );
            painter.rect_filled(lbl_rect, 2.0, color);
            painter.text(
                egui::pos2(chart_rect.right() + 4.0, y),
                egui::Align2::LEFT_CENTER,
                &label,
                egui::FontId::monospace(10.0),
                egui::Color32::BLACK,
            );
        }
    }

    // ── Extended hours price line (magenta dashed) ─────────────────────────
    if chart.ext_active && chart.ext_close > 0.0 {
        let y = price_to_y(chart.ext_close);
        if y >= chart_rect.top() && y <= chart_rect.bottom() {
            let ext_col = egui::Color32::from_rgb(200, 50, 200);
            let dash_len = 4.0_f32;
            let mut x = chart_rect.left();
            while x < chart_rect.right() {
                let end = (x + dash_len).min(chart_rect.right());
                painter.line_segment(
                    [egui::pos2(x, y), egui::pos2(end, y)],
                    egui::Stroke::new(1.0, ext_col),
                );
                x += dash_len * 2.0;
            }
            // Price label
            let label = format_price(chart.ext_close);
            let lbl_rect = egui::Rect::from_min_size(
                egui::pos2(chart_rect.right() + 2.0, y - 8.0),
                egui::vec2(price_axis_w - 4.0, 16.0),
            );
            painter.rect_filled(lbl_rect, 2.0, ext_col);
            painter.text(
                egui::pos2(chart_rect.right() + 4.0, y),
                egui::Align2::LEFT_CENTER,
                &label,
                egui::FontId::monospace(10.0),
                egui::Color32::BLACK,
            );
        }
    }

    // ── Bid/Ask spread lines (live streaming quotes) ──────────────────────
    if chart.live_bid > 0.0 && chart.live_ask > 0.0 {
        let bid_y = price_to_y(chart.live_bid);
        let ask_y = price_to_y(chart.live_ask);
        let bid_col = egui::Color32::from_rgba_premultiplied(0, 200, 80, 120);
        let ask_col = egui::Color32::from_rgba_premultiplied(220, 50, 50, 120);
        if bid_y >= chart_rect.top() && bid_y <= chart_rect.bottom() {
            painter.line_segment(
                [
                    egui::pos2(chart_rect.left(), bid_y),
                    egui::pos2(chart_rect.right(), bid_y),
                ],
                egui::Stroke::new(0.5, bid_col),
            );
        }
        if ask_y >= chart_rect.top() && ask_y <= chart_rect.bottom() {
            painter.line_segment(
                [
                    egui::pos2(chart_rect.left(), ask_y),
                    egui::pos2(chart_rect.right(), ask_y),
                ],
                egui::Stroke::new(0.5, ask_col),
            );
        }
    }

    // ── Volume Profile overlay (volume-at-price with POC + Value Area) ─────
    if flags.price_histogram {
        let num_buckets = (chart_rect.height() / 4.0).max(10.0) as usize;
        let bucket_h = chart_rect.height() / num_buckets as f32;
        let mut buckets = vec![0.0_f64; num_buckets];
        let mut buy_vol = vec![0.0_f64; num_buckets]; // close > open = buying pressure
        let mut max_vol = 0.0_f64;

        for bar in bars {
            let y_high_frac = ((price_max - bar.high) / (price_max - price_min)).clamp(0.0, 1.0);
            let y_low_frac = ((price_max - bar.low) / (price_max - price_min)).clamp(0.0, 1.0);
            let b_top = (y_high_frac * num_buckets as f64) as usize;
            let b_bot = ((y_low_frac * num_buckets as f64) as usize).min(num_buckets - 1);
            let span = (b_bot - b_top).max(1) as f64;
            let vol_per_level = bar.volume / span;
            let is_buy = bar.close >= bar.open;
            for b in b_top..=b_bot {
                if b < num_buckets {
                    buckets[b] += vol_per_level;
                    if is_buy {
                        buy_vol[b] += vol_per_level;
                    }
                    max_vol = max_vol.max(buckets[b]);
                }
            }
        }

        // POC = highest volume bucket
        let poc_idx = buckets
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Value Area: expand from POC until 70% of total volume
        let total_vol: f64 = buckets.iter().sum();
        let va_target = total_vol * 0.7;
        let mut va_vol = buckets[poc_idx];
        let mut va_lo = poc_idx;
        let mut va_hi = poc_idx;
        while va_vol < va_target && (va_lo > 0 || va_hi < num_buckets - 1) {
            let expand_lo = if va_lo > 0 { buckets[va_lo - 1] } else { 0.0 };
            let expand_hi = if va_hi < num_buckets - 1 {
                buckets[va_hi + 1]
            } else {
                0.0
            };
            if expand_lo >= expand_hi && va_lo > 0 {
                va_lo -= 1;
                va_vol += buckets[va_lo];
            } else if va_hi < num_buckets - 1 {
                va_hi += 1;
                va_vol += buckets[va_hi];
            } else {
                break;
            }
        }

        // Draw horizontal bars: buy (teal) left, sell (red) right, POC highlighted
        let max_bar_w = chart_rect.width() * 0.18;
        let poc_col = egui::Color32::from_rgba_premultiplied(255, 215, 0, 120); // gold
        let va_buy = egui::Color32::from_rgba_premultiplied(38, 166, 154, 60); // teal
        let va_sell = egui::Color32::from_rgba_premultiplied(239, 83, 80, 60); // red
        let out_buy = egui::Color32::from_rgba_premultiplied(38, 166, 154, 30);
        let out_sell = egui::Color32::from_rgba_premultiplied(239, 83, 80, 30);
        let edge_col = egui::Color32::from_rgba_premultiplied(100, 140, 255, 80);
        for (i, &vol) in buckets.iter().enumerate() {
            if vol <= 0.0 {
                continue;
            }
            let frac = (vol / max_vol) as f32;
            let total_w = frac * max_bar_w;
            let buy_frac = if vol > 0.0 {
                (buy_vol[i] / vol) as f32
            } else {
                0.5
            };
            let buy_w = total_w * buy_frac;
            let sell_w = total_w - buy_w;
            let y_top = chart_rect.top() + i as f32 * bucket_h;
            let y_bot = y_top + bucket_h;
            let is_va = i >= va_lo && i <= va_hi;

            if i == poc_idx {
                // POC: full-width gold highlight line
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(chart_rect.right() - total_w, y_top),
                        egui::pos2(chart_rect.right(), y_bot),
                    ),
                    0.0,
                    poc_col,
                );
            } else {
                // Buy volume (right-aligned, teal)
                let (bc, sc) = if is_va {
                    (va_buy, va_sell)
                } else {
                    (out_buy, out_sell)
                };
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(chart_rect.right() - total_w, y_top),
                        egui::pos2(chart_rect.right() - sell_w, y_bot),
                    ),
                    0.0,
                    bc,
                );
                // Sell volume
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(chart_rect.right() - sell_w, y_top),
                        egui::pos2(chart_rect.right(), y_bot),
                    ),
                    0.0,
                    sc,
                );
            }
            // Left edge
            painter.line_segment(
                [
                    egui::pos2(chart_rect.right() - total_w, y_top),
                    egui::pos2(chart_rect.right() - total_w, y_bot),
                ],
                egui::Stroke::new(0.5, edge_col),
            );
        }
        // POC dashed line across chart
        {
            let poc_y = chart_rect.top() + (poc_idx as f32 + 0.5) * bucket_h;
            let mut px = chart_rect.left();
            while px < chart_rect.right() {
                let end = (px + 4.0).min(chart_rect.right());
                painter.line_segment(
                    [egui::pos2(px, poc_y), egui::pos2(end, poc_y)],
                    egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(255, 215, 0, 80)),
                );
                px += 8.0;
            }
        }
    }

    // ── crosshair ────────────────────────────────────────────────────────────
    if let Some(pos) = crosshair {
        if chart_rect.contains(pos) {
            let ch_color = egui::Color32::from_rgba_premultiplied(180, 180, 200, 100);
            painter.line_segment(
                [
                    egui::pos2(pos.x, chart_rect.top()),
                    egui::pos2(pos.x, chart_rect.bottom()),
                ],
                egui::Stroke::new(0.5, ch_color),
            );
            painter.line_segment(
                [
                    egui::pos2(chart_rect.left(), pos.y),
                    egui::pos2(chart_rect.right(), pos.y),
                ],
                egui::Stroke::new(0.5, ch_color),
            );

            // Price label on right axis
            let frac = (pos.y - chart_rect.top()) / chart_rect.height();
            let price = price_max - frac as f64 * (price_max - price_min);
            let label = format_price(price);
            let lbl_rect = egui::Rect::from_min_size(
                egui::pos2(chart_rect.right() + 2.0, pos.y - 8.0),
                egui::vec2(price_axis_w - 4.0, 16.0),
            );
            painter.rect_filled(lbl_rect, 2.0, egui::Color32::from_rgb(50, 50, 80));
            painter.text(
                egui::pos2(chart_rect.right() + 4.0, pos.y),
                egui::Align2::LEFT_CENTER,
                &label,
                egui::FontId::monospace(10.0),
                egui::Color32::WHITE,
            );

            // OHLCV + indicator values data window (WebKit: .data-window — #000000ee bg)
            let rel_x = pos.x - chart_rect.left();
            let bar_idx = ((rel_x / bar_w) as usize).min(bars.len().saturating_sub(1));
            if bar_idx < bars.len() {
                let b = &bars[bar_idx];
                let abs_idx = start_idx + bar_idx;
                let tooltip = format!(
                    "O:{} H:{} L:{} C:{} V:{:.0}",
                    format_price(b.open),
                    format_price(b.high),
                    format_price(b.low),
                    format_price(b.close),
                    b.volume
                );
                // Semi-transparent background behind data text (WebKit: background #000000ee)
                let data_bg = egui::Rect::from_min_size(
                    egui::pos2(chart_rect.left() + 2.0, chart_rect.top() + 2.0),
                    egui::vec2(tooltip.len() as f32 * 6.5 + 8.0, 30.0),
                );
                painter.rect_filled(
                    data_bg,
                    2.0,
                    egui::Color32::from_rgba_premultiplied(0, 0, 0, 238),
                );
                painter.text(
                    egui::pos2(chart_rect.left() + 6.0, chart_rect.top() + 4.0),
                    egui::Align2::LEFT_TOP,
                    &tooltip,
                    egui::FontId::monospace(10.0),
                    egui::Color32::from_rgb(220, 220, 255),
                );

                // Indicator values on second line
                let mut ind_parts: Vec<String> = Vec::new();
                if flags.sma200 {
                    if let Some(Some(v)) = chart.sma200.get(abs_idx) {
                        ind_parts.push(format!("SMA200:{}", format_price(*v)));
                    }
                }
                if flags.sma100 {
                    if let Some(Some(v)) = chart.sma100.get(abs_idx) {
                        ind_parts.push(format!("SMA100:{}", format_price(*v)));
                    }
                }
                if flags.kama {
                    if let Some(Some(v)) = chart.kama.get(abs_idx) {
                        ind_parts.push(format!("KAMA:{}", format_price(*v)));
                    }
                }
                if flags.ema21 {
                    if let Some(Some(v)) = chart.ema21.get(abs_idx) {
                        ind_parts.push(format!("EMA21:{}", format_price(*v)));
                    }
                }
                if show_rsi {
                    if let Some(Some(v)) = chart.rsi.get(abs_idx) {
                        ind_parts.push(format!("RSI:{:.1}", v));
                    }
                }
                if show_cmo {
                    if let Some(Some(v)) = chart.cmo.get(abs_idx) {
                        ind_parts.push(format!("CMO:{:+.1}", v));
                    }
                }
                if show_qstick {
                    if let Some(Some(v)) = chart.qstick.get(abs_idx) {
                        ind_parts.push(format!("QStick:{:+.3}", v));
                    }
                }
                if show_disparity {
                    if let Some(Some(v)) = chart.disparity.get(abs_idx) {
                        ind_parts.push(format!("Disp:{:+.2}%", v));
                    }
                }
                if show_bop {
                    if let Some(Some(v)) = chart.bop.get(abs_idx) {
                        ind_parts.push(format!("BOP:{:+.3}", v));
                    }
                }
                if show_stddev {
                    if let Some(Some(v)) = chart.stddev.get(abs_idx) {
                        ind_parts.push(format!("StdDev:{:.3}", v));
                    }
                }
                if show_mfi {
                    if let Some(Some(v)) = chart.mfi.get(abs_idx) {
                        ind_parts.push(format!("MFI:{:.1}", v));
                    }
                }
                if show_trix {
                    if let Some(Some(v)) = chart.trix_line.get(abs_idx) {
                        ind_parts.push(format!("TRIX:{:+.3}", v));
                    }
                }
                if show_ppo {
                    if let Some(Some(v)) = chart.ppo_line.get(abs_idx) {
                        ind_parts.push(format!("PPO:{:+.2}", v));
                    }
                }
                if show_ultosc {
                    if let Some(Some(v)) = chart.ultosc.get(abs_idx) {
                        ind_parts.push(format!("ULT:{:.1}", v));
                    }
                }
                if show_stochrsi {
                    if let (Some(Some(k)), Some(Some(d))) =
                        (chart.stochrsi_k.get(abs_idx), chart.stochrsi_d.get(abs_idx))
                    {
                        ind_parts.push(format!("StochRSI:{:.1}/{:.1}", k, d));
                    }
                }
                if show_var_oscillator {
                    if let Some(Some(v)) = chart.var_oscillator.get(abs_idx) {
                        ind_parts.push(format!("VaR:{:.1}", v));
                    }
                }
                if let Some(Some(v)) = chart.atr.get(abs_idx) {
                    ind_parts.push(format!("ATR:{}", format_price(*v)));
                }
                if !ind_parts.is_empty() {
                    let ind_text = ind_parts.join("  ");
                    painter.text(
                        egui::pos2(chart_rect.left() + 6.0, chart_rect.top() + 18.0),
                        egui::Align2::LEFT_TOP,
                        &ind_text,
                        egui::FontId::monospace(10.0),
                        egui::Color32::from_rgb(180, 180, 200),
                    );
                }
            }
        }
    }

    // ── symbol / tf label (WebKit: .mtf-cell-label — #8cf, 11px bold, text-shadow)
    let sym_label = format!("{} [{}]", chart.symbol, chart.timeframe.label());
    // Shadow for readability over candles
    painter.text(
        egui::pos2(chart_rect.left() + 9.0, chart_rect.top() + 7.0),
        egui::Align2::LEFT_TOP,
        &sym_label,
        egui::FontId::monospace(11.0),
        egui::Color32::from_rgb(0, 0, 0),
    );
    painter.text(
        egui::pos2(chart_rect.left() + 8.0, chart_rect.top() + 6.0),
        egui::Align2::LEFT_TOP,
        &sym_label,
        egui::FontId::monospace(11.0),
        egui::Color32::WHITE,
    );

    // ── indicator legend ─────────────────────────────────────────────────────
    let ly = chart_rect.top() + 34.0;
    let mut lx = chart_rect.left() + 8.0;
    if flags.sma200 {
        painter.text(
            egui::pos2(lx, ly),
            egui::Align2::LEFT_TOP,
            "SMA200",
            egui::FontId::monospace(10.0),
            SMA200_COL,
        );
        lx += 57.0;
    }
    if flags.sma100 {
        painter.text(
            egui::pos2(lx, ly),
            egui::Align2::LEFT_TOP,
            "SMA100",
            egui::FontId::monospace(10.0),
            SMA100_COL,
        );
        lx += 57.0;
    }
    if flags.kama {
        painter.text(
            egui::pos2(lx, ly),
            egui::Align2::LEFT_TOP,
            "KAMA(10,2,30)",
            egui::FontId::monospace(10.0),
            KAMA_COL,
        );
        lx += 110.0;
    }
    if flags.ema21 {
        painter.text(
            egui::pos2(lx, ly),
            egui::Align2::LEFT_TOP,
            "EMA21",
            egui::FontId::monospace(10.0),
            EMA_COL,
        );
        lx += 50.0;
    }
    if flags.bollinger {
        painter.text(
            egui::pos2(lx, ly),
            egui::Align2::LEFT_TOP,
            "BB(20,2)",
            egui::FontId::monospace(10.0),
            BB_COL,
        );
    }

    // Chart overlay removed — info shown in crosshair tooltip + right panel instead

    // ── sub-panes (RSI, Fisher) ──────────────────────────────────────────────
    let mut sub_y = main_rect.bottom();

    if show_rsi {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.rsi,
            start_idx,
            bar_w,
            "RSI(14)",
            RSI_LINE,
            0.0,
            100.0,
            Some(70.0),
            Some(30.0),
        );
        sub_y += 80.0;
    }

    if show_fisher {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_fisher_pane(
            painter,
            pane_rect,
            bars,
            &chart.fisher,
            &chart.fisher_signal,
            start_idx,
            bar_w,
        );
        sub_y += 80.0;
    }

    if show_macd {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_macd_pane(
            painter,
            pane_rect,
            bars,
            &chart.macd_line,
            &chart.macd_signal,
            &chart.macd_hist,
            start_idx,
            bar_w,
            "MACD(12,26,9)",
            MACD_LINE_COL,
            MACD_SIG_COL,
        );
        sub_y += 80.0;
    }

    if show_volume_pane {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_volume_pane(painter, pane_rect, bars, start_idx, bar_w);
        sub_y += 80.0;
    }

    if show_stochastic {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_stoch_pane(
            painter,
            pane_rect,
            bars,
            &chart.stoch_k,
            &chart.stoch_d,
            start_idx,
            bar_w,
            "Stoch(14,3,3)",
        );
        sub_y += 80.0;
    }

    if show_adx {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_adx_pane(
            painter,
            pane_rect,
            bars,
            &chart.adx,
            &chart.di_plus,
            &chart.di_minus,
            start_idx,
            bar_w,
        );
        sub_y += 80.0;
    }

    if show_cci {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.cci,
            start_idx,
            bar_w,
            "CCI(20)",
            CCI_COL,
            -200.0,
            200.0,
            Some(100.0),
            Some(-100.0),
        );
        sub_y += 80.0;
    }

    if show_williams_r {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.williams_r,
            start_idx,
            bar_w,
            "Williams %R(14)",
            WILLR_COL,
            -100.0,
            0.0,
            Some(-20.0),
            Some(-80.0),
        );
        sub_y += 80.0;
    }

    if show_obv {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        // OBV auto-scales
        let mut ob_min = f64::MAX;
        let mut ob_max = f64::MIN;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.obv.get(start_idx + ri) {
                ob_min = ob_min.min(*v);
                ob_max = ob_max.max(*v);
            }
        }
        let pad = (ob_max - ob_min) * 0.1;
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.obv,
            start_idx,
            bar_w,
            "OBV",
            OBV_COL,
            ob_min - pad,
            ob_max + pad,
            None,
            None,
        );
        sub_y += 80.0;
    }

    if show_momentum {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut m_min = f64::MAX;
        let mut m_max = f64::MIN;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.momentum.get(start_idx + ri) {
                m_min = m_min.min(*v);
                m_max = m_max.max(*v);
            }
        }
        let pad = (m_max - m_min).max(0.001) * 0.1;
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.momentum,
            start_idx,
            bar_w,
            "Momentum(10)",
            egui::Color32::from_rgb(200, 150, 100),
            m_min - pad,
            m_max + pad,
            None,
            None,
        );
        sub_y += 80.0;
    }

    if show_cmo {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.cmo,
            start_idx,
            bar_w,
            "CMO(9)",
            egui::Color32::from_rgb(120, 220, 200),
            -100.0,
            100.0,
            Some(50.0),
            Some(-50.0),
        );
        sub_y += 80.0;
    }

    if show_qstick {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut bound = 0.0_f64;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.qstick.get(start_idx + ri) {
                bound = bound.max(v.abs());
            }
        }
        let bound = bound.max(0.001);
        let pad = bound * 0.1;
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.qstick,
            start_idx,
            bar_w,
            "QStick(14)",
            egui::Color32::from_rgb(190, 140, 255),
            -(bound + pad),
            bound + pad,
            None,
            None,
        );
        sub_y += 80.0;
    }

    if show_disparity {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut bound = 3.0_f64;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.disparity.get(start_idx + ri) {
                bound = bound.max(v.abs());
            }
        }
        let pad = bound * 0.1;
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.disparity,
            start_idx,
            bar_w,
            "Disparity(14)",
            egui::Color32::from_rgb(255, 210, 90),
            -(bound + pad),
            bound + pad,
            Some(3.0),
            Some(-3.0),
        );
        sub_y += 80.0;
    }

    if show_bop {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.bop,
            start_idx,
            bar_w,
            "BOP(14)",
            egui::Color32::from_rgb(255, 120, 120),
            -1.0,
            1.0,
            Some(0.5),
            Some(-0.5),
        );
        sub_y += 80.0;
    }

    if show_stddev {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut s_max = 0.0_f64;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.stddev.get(start_idx + ri) {
                s_max = s_max.max(*v);
            }
        }
        let s_max = s_max.max(1.0);
        let pad = s_max * 0.1;
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.stddev,
            start_idx,
            bar_w,
            "StdDev(20)",
            egui::Color32::from_rgb(120, 180, 255),
            0.0,
            s_max + pad,
            None,
            None,
        );
        sub_y += 80.0;
    }

    if show_mfi {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.mfi,
            start_idx,
            bar_w,
            "MFI(14)",
            MFI_COL,
            0.0,
            100.0,
            Some(80.0),
            Some(20.0),
        );
        sub_y += 80.0;
    }

    if show_trix {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_macd_pane(
            painter,
            pane_rect,
            bars,
            &chart.trix_line,
            &chart.trix_signal,
            &chart.trix_hist,
            start_idx,
            bar_w,
            "TRIX(15,9)",
            TRIX_LINE_COL,
            TRIX_SIG_COL,
        );
        sub_y += 80.0;
    }

    if show_ppo {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_macd_pane(
            painter,
            pane_rect,
            bars,
            &chart.ppo_line,
            &chart.ppo_signal,
            &chart.ppo_hist,
            start_idx,
            bar_w,
            "PPO(12,26,9)",
            PPO_LINE_COL,
            PPO_SIG_COL,
        );
        sub_y += 80.0;
    }

    if show_ultosc {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.ultosc,
            start_idx,
            bar_w,
            "ULTOSC(7,14,28)",
            ULTOSC_COL,
            0.0,
            100.0,
            Some(70.0),
            Some(30.0),
        );
        sub_y += 80.0;
    }

    if show_stochrsi {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_stoch_pane(
            painter,
            pane_rect,
            bars,
            &chart.stochrsi_k,
            &chart.stochrsi_d,
            start_idx,
            bar_w,
            "StochRSI(14,14,3,3)",
        );
        sub_y += 80.0;
    }

    if show_var_oscillator {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut bound = 100.0_f64;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.var_oscillator.get(start_idx + ri) {
                bound = bound.max(v.abs());
            }
        }
        let bound = bound.max(100.0);
        let pad = bound * 0.1;
        draw_oscillator_pane(
            painter,
            pane_rect,
            bars,
            &chart.var_oscillator,
            start_idx,
            bar_w,
            "VaR Osc(20,95%)",
            egui::Color32::from_rgb(255, 170, 80),
            -(bound + pad),
            bound + pad,
            Some(100.0),
            Some(-100.0),
        );
        sub_y += 80.0;
    }

    if show_better_volume {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_better_volume_pane(
            painter,
            pane_rect,
            bars,
            &chart.better_vol_type,
            start_idx,
            bar_w,
        );
        sub_y += 80.0;
    }

    // Ehlers sub-panes
    if show_ehlers_ebsw {
        let pr = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(
            painter,
            pr,
            bars,
            &chart.ehlers_ebsw,
            start_idx,
            bar_w,
            "EBSW",
            EHLERS_EBSW_COL,
            -1.0,
            1.0,
            None,
            None,
        );
        sub_y += 80.0;
    }
    if show_ehlers_cyber {
        let pr = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut cmin = f64::MAX;
        let mut cmax = f64::MIN;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.ehlers_cyber.get(start_idx + ri) {
                cmin = cmin.min(*v);
                cmax = cmax.max(*v);
            }
        }
        let pad = (cmax - cmin).max(0.001) * 0.1;
        draw_oscillator_pane(
            painter,
            pr,
            bars,
            &chart.ehlers_cyber,
            start_idx,
            bar_w,
            "Cyber Cycle",
            EHLERS_CYBER_COL,
            cmin - pad,
            cmax + pad,
            None,
            None,
        );
        sub_y += 80.0;
    }
    if show_ehlers_cg {
        let pr = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut cmin = f64::MAX;
        let mut cmax = f64::MIN;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.ehlers_cg.get(start_idx + ri) {
                cmin = cmin.min(*v);
                cmax = cmax.max(*v);
            }
        }
        let pad = (cmax - cmin).max(0.001) * 0.1;
        draw_oscillator_pane(
            painter,
            pr,
            bars,
            &chart.ehlers_cg,
            start_idx,
            bar_w,
            "CG Oscillator",
            EHLERS_CG_COL,
            cmin - pad,
            cmax + pad,
            None,
            None,
        );
        sub_y += 80.0;
    }
    if show_ehlers_roof {
        let pr = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        let mut cmin = f64::MAX;
        let mut cmax = f64::MIN;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.ehlers_roof.get(start_idx + ri) {
                cmin = cmin.min(*v);
                cmax = cmax.max(*v);
            }
        }
        let pad = (cmax - cmin).max(0.001) * 0.1;
        draw_oscillator_pane(
            painter,
            pr,
            bars,
            &chart.ehlers_roof,
            start_idx,
            bar_w,
            "Roofing Filter",
            EHLERS_ROOF_COL,
            cmin - pad,
            cmax + pad,
            None,
            None,
        );
    }

    // ── Squeeze Momentum sub-pane ──────────────────────────────────────────
    if show_squeeze {
        let sr = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        #[allow(unused_assignments)]
        {
            sub_y += 80.0;
        } // last sub-pane
        painter.rect_filled(sr, 0.0, egui::Color32::from_rgb(0, 0, 0));
        painter.line_segment(
            [
                egui::pos2(sr.left(), sr.top()),
                egui::pos2(sr.right(), sr.top()),
            ],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 68, 68)),
        );
        // Find momentum range
        let mut mom_min = f64::MAX;
        let mut mom_max = f64::MIN;
        for (ri, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = chart.squeeze_mom.get(start_idx + ri) {
                mom_min = mom_min.min(*v);
                mom_max = mom_max.max(*v);
            }
        }
        if mom_min >= mom_max {
            mom_min = -1.0;
            mom_max = 1.0;
        }
        let pad = (mom_max - mom_min) * 0.1;
        mom_min -= pad;
        mom_max += pad;
        let val_to_y = |v: f64| -> f32 {
            sr.top() + ((mom_max - v) / (mom_max - mom_min)) as f32 * sr.height()
        };
        let zero_y = val_to_y(0.0);
        // Zero line
        painter.line_segment(
            [
                egui::pos2(sr.left(), zero_y),
                egui::pos2(sr.right(), zero_y),
            ],
            egui::Stroke::new(0.5, egui::Color32::from_rgb(60, 60, 60)),
        );
        // Histogram bars
        for (ri, _) in bars.iter().enumerate() {
            let abs_idx = start_idx + ri;
            if let Some(Some(v)) = chart.squeeze_mom.get(abs_idx) {
                let x = sr.left() + (ri as f32 + 0.5) * bar_w;
                let y = val_to_y(*v);
                let is_squeeze = chart.squeeze_on.get(abs_idx).copied().unwrap_or(false);
                // Color: squeeze=gray, released: positive=cyan, negative=red
                // Momentum direction: increasing=brighter, decreasing=dimmer
                let prev_v = if abs_idx > 0 {
                    chart
                        .squeeze_mom
                        .get(abs_idx - 1)
                        .and_then(|v| *v)
                        .unwrap_or(0.0)
                } else {
                    0.0
                };
                let color = if is_squeeze {
                    egui::Color32::from_rgb(100, 100, 100) // gray = squeeze active
                } else if *v > 0.0 {
                    if *v > prev_v {
                        egui::Color32::from_rgb(0, 220, 200)
                    } else {
                        egui::Color32::from_rgb(0, 120, 100)
                    }
                } else {
                    if *v < prev_v {
                        egui::Color32::from_rgb(220, 50, 50)
                    } else {
                        egui::Color32::from_rgb(120, 30, 30)
                    }
                };
                let half = (bar_w * 0.35).max(0.5);
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(x - half, y.min(zero_y)),
                        egui::pos2(x + half, y.max(zero_y)),
                    ),
                    0.0,
                    color,
                );
            }
        }
        // Label
        painter.text(
            egui::pos2(sr.left() + 4.0, sr.top() + 2.0),
            egui::Align2::LEFT_TOP,
            "Squeeze",
            egui::FontId::monospace(9.0),
            AXIS_TEXT,
        );
    }

    // ── SL/TP planning lines ───────────────────────────────────────────────
    for (price_opt, label, color) in [
        (&sl_price, "SL", egui::Color32::from_rgb(220, 40, 40)),
        (&tp_price, "TP", egui::Color32::from_rgb(0, 200, 80)),
    ] {
        if let Some(p) = price_opt {
            let y = price_to_y(*p);
            if y >= chart_rect.top() && y <= chart_rect.bottom() {
                let shadow = egui::Color32::from_rgba_premultiplied(0, 0, 0, 190);
                let band =
                    egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 36);
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(chart_rect.left(), y - 5.0),
                        egui::pos2(chart_rect.right(), y + 5.0),
                    ),
                    0.0,
                    band,
                );
                painter.line_segment(
                    [
                        egui::pos2(chart_rect.left(), y),
                        egui::pos2(chart_rect.right(), y),
                    ],
                    egui::Stroke::new(6.0, shadow),
                );
                painter.line_segment(
                    [
                        egui::pos2(chart_rect.left(), y),
                        egui::pos2(chart_rect.right(), y),
                    ],
                    egui::Stroke::new(3.0, color),
                );

                let pad_x = 6.0_f32;
                let pad_y = 3.0_f32;
                let price_text = format!("{} {}", label, format_price(*p));
                let price_galley = painter.layout_no_wrap(
                    price_text,
                    egui::FontId::monospace(11.0),
                    egui::Color32::BLACK,
                );
                let price_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        chart_rect.left() + 8.0,
                        y - price_galley.rect.height() * 0.5 - pad_y,
                    ),
                    egui::vec2(
                        price_galley.rect.width() + pad_x * 2.0,
                        price_galley.rect.height() + pad_y * 2.0,
                    ),
                );
                painter.rect_filled(price_rect, 3.0, color);
                painter.rect_stroke(
                    price_rect,
                    3.0,
                    egui::Stroke::new(1.0, shadow),
                    egui::StrokeKind::Outside,
                );
                painter.galley(
                    egui::pos2(
                        price_rect.left() + pad_x,
                        price_rect.center().y - price_galley.rect.height() * 0.5,
                    ),
                    price_galley,
                    egui::Color32::BLACK,
                );

                // P&L from last price
                if let Some(last) = bars.last() {
                    let dist = *p - last.close;
                    let dist_label = if dist > 0.0 {
                        format!("+{}", format_price(dist.abs()))
                    } else if dist < 0.0 {
                        format!("-{}", format_price(dist.abs()))
                    } else {
                        format!("±{}", format_price(0.0))
                    };
                    let dist_galley = painter.layout_no_wrap(
                        dist_label,
                        egui::FontId::monospace(10.0),
                        egui::Color32::BLACK,
                    );
                    let dist_rect = egui::Rect::from_min_size(
                        egui::pos2(
                            chart_rect.right() - dist_galley.rect.width() - 26.0,
                            y - dist_galley.rect.height() * 0.5 - pad_y,
                        ),
                        egui::vec2(
                            dist_galley.rect.width() + pad_x * 2.0,
                            dist_galley.rect.height() + pad_y * 2.0,
                        ),
                    );
                    painter.rect_filled(
                        dist_rect,
                        3.0,
                        egui::Color32::from_rgba_premultiplied(
                            color.r(),
                            color.g(),
                            color.b(),
                            220,
                        ),
                    );
                    painter.rect_stroke(
                        dist_rect,
                        3.0,
                        egui::Stroke::new(1.0, shadow),
                        egui::StrokeKind::Outside,
                    );
                    painter.galley(
                        egui::pos2(
                            dist_rect.left() + pad_x,
                            dist_rect.center().y - dist_galley.rect.height() * 0.5,
                        ),
                        dist_galley,
                        egui::Color32::BLACK,
                    );
                }
            }
        }
    }

    // ── Drawing control points (drag handles when selected) ────────────────
    if let Some(sel) = chart.selected_drawing {
        if let Some(drawing) = chart.drawings.get(sel) {
            let cp_size = 4.0_f32; // half-size of control point square
            let cp_fill = egui::Color32::from_rgb(0, 200, 220);
            let cp_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
            // Collect control points as (bar_idx, price)
            let mut cps: Vec<(usize, f64)> = Vec::new();
            match drawing {
                Drawing::HLine { price, .. } => {
                    cps.push((start_idx, *price));
                    cps.push((end_idx.saturating_sub(1), *price));
                }
                Drawing::VLine { bar_idx, .. } => {
                    cps.push((*bar_idx, price_max));
                    cps.push((*bar_idx, price_min));
                }
                Drawing::TrendLine { p1, p2, .. }
                | Drawing::ExtendedLine { p1, p2, .. }
                | Drawing::ArrowLine { p1, p2, .. }
                | Drawing::InfoLine { p1, p2, .. }
                | Drawing::TrendAngle { p1, p2, .. }
                | Drawing::Rectangle { p1, p2, .. }
                | Drawing::Highlighter { p1, p2, .. }
                | Drawing::Ruler { p1, p2, .. }
                | Drawing::MeasureTool { p1, p2, .. }
                | Drawing::Forecast { p1, p2, .. }
                | Drawing::Ellipse { p1, p2, .. }
                | Drawing::SineWave { p1, p2, .. } => {
                    cps.push(*p1);
                    cps.push(*p2);
                }
                Drawing::Pitchfork { pivot, p2, p3, .. }
                | Drawing::SchiffPitchfork { pivot, p2, p3, .. }
                | Drawing::ModSchiffPitchfork { pivot, p2, p3, .. }
                | Drawing::InsidePitchfork { pivot, p2, p3, .. } => {
                    cps.push(*pivot);
                    cps.push(*p2);
                    cps.push(*p3);
                }
                Drawing::FiboExtension { p1, p2, p3, .. }
                | Drawing::FibChannel { p1, p2, p3, .. }
                | Drawing::TrendChannel { p1, p2, p3, .. }
                | Drawing::ArcDraw { p1, p2, p3, .. }
                | Drawing::Triangle { p1, p2, p3, .. }
                | Drawing::RotatedRectangle { p1, p2, p3, .. } => {
                    cps.push(*p1);
                    cps.push(*p2);
                    cps.push(*p3);
                }
                Drawing::Polyline { points, .. }
                | Drawing::ElliottWave { points, .. }
                | Drawing::AbcCorrection { points, .. }
                | Drawing::HeadShoulders { points, .. }
                | Drawing::XabcdPattern { points, .. }
                | Drawing::PathDraw { points, .. } => {
                    for pt in points {
                        cps.push(*pt);
                    }
                }
                _ => {} // single-point tools: no resize handles needed
            }
            for (bi, pr) in &cps {
                if *bi >= start_idx && *bi < end_idx {
                    let x = chart_rect.left() + ((*bi - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*pr);
                    if y >= chart_rect.top() && y <= chart_rect.bottom() {
                        let r = egui::Rect::from_center_size(
                            egui::pos2(x, y),
                            egui::vec2(cp_size * 2.0, cp_size * 2.0),
                        );
                        painter.rect_filled(r, 0.0, cp_fill);
                        painter.rect_stroke(r, 0.0, cp_stroke, egui::StrokeKind::Outside);
                    }
                }
            }
        }
    }

    // ── Compare symbol overlay (% change line) ──────────────────────────
    if let Some(ref _cmp_sym) = chart.compare_symbol {
        if !chart.compare_bars.is_empty() && bars.len() > 1 {
            let cmp = &chart.compare_bars;
            let (start_idx, _end_idx) = chart.visible_range();
            let base_close = chart.bars.get(start_idx).map(|b| b.close).unwrap_or(1.0);
            let cmp_base = cmp
                .get(start_idx.min(cmp.len().saturating_sub(1)))
                .map(|b| b.close)
                .unwrap_or(1.0);
            if base_close > 0.0 && cmp_base > 0.0 {
                let cmp_col = egui::Color32::from_rgb(200, 100, 255); // purple overlay
                let mut prev_pt: Option<egui::Pos2> = None;
                for rel_idx in 0..bars.len() {
                    let abs_idx = start_idx + rel_idx;
                    if abs_idx >= cmp.len() {
                        break;
                    }
                    let cmp_pct = (cmp[abs_idx].close - cmp_base) / cmp_base;
                    let mapped_price = base_close * (1.0 + cmp_pct);
                    let x = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
                    let y = price_to_y(mapped_price);
                    let pt = egui::pos2(x, y.clamp(chart_rect.top(), chart_rect.bottom()));
                    if let Some(pp) = prev_pt {
                        painter.line_segment([pp, pt], egui::Stroke::new(1.5, cmp_col));
                    }
                    prev_pt = Some(pt);
                }
            }
        }
    }

    // ── DARWIN/broker trade markers (buy/sell arrows + position lines) ────────
    // Position entry/SL/TP lines
    for pl in &trade_overlay.position_lines {
        let y = price_to_y(pl.price);
        if y >= chart_rect.top() && y <= chart_rect.bottom() {
            let (color, label_prefix) = match pl.line_type {
                0 => (
                    if pl.is_buy {
                        egui::Color32::from_rgb(0, 150, 255)
                    } else {
                        egui::Color32::from_rgb(255, 100, 50)
                    },
                    if pl.is_buy { "BUY" } else { "SELL" },
                ),
                1 => (egui::Color32::from_rgb(255, 60, 60), "SL"),
                _ => (egui::Color32::from_rgb(60, 200, 60), "TP"),
            };
            // Dashed line across chart
            let dash_len = 6.0_f32;
            let gap_len = 4.0_f32;
            let mut fx = chart_rect.left();
            while fx < chart_rect.right() {
                let end = (fx + dash_len).min(chart_rect.right());
                painter.line_segment(
                    [egui::pos2(fx, y), egui::pos2(end, y)],
                    egui::Stroke::new(1.0, color),
                );
                fx += dash_len + gap_len;
            }
            // Label with volume
            let label = format!("{} {:.2}", label_prefix, pl.volume);
            painter.text(
                egui::pos2(chart_rect.left() + 4.0, y - 10.0),
                egui::Align2::LEFT_TOP,
                &label,
                egui::FontId::monospace(9.0),
                color,
            );
        }
    }
    // Trade arrows (buy = green up-arrow, sell = red down-arrow).
    // PERF: markers are sorted by bar_idx (see build_trade_overlay). Binary-search
    // for the first in-range marker so we skip off-screen history in O(log N) instead
    // of scanning the full Vec every frame.
    // Arrows render per-deal (small triangles — not noisy). Labels are deferred
    // and collapsed by screen-pixel clustering so dense DARWIN mirror activity
    // (many accounts, slightly different fill prices on the same bar) doesn't
    // bury the candles under overlapping text blocks. Previously each deal
    // rendered its own "HAKR 1.00"/"MFS0 2.00" label and the chart became
    // unreadable at high trade density.
    struct PendingLabel {
        x: f32,
        y: f32,
        is_buy: bool,
        volume: f64,
        price: f64,
        ticker: String,
        count: u32,
    }
    let mut pending_labels: Vec<PendingLabel> = Vec::new();
    let marker_start = trade_overlay
        .markers
        .partition_point(|m| m.bar_idx < start_idx);
    for tm in trade_overlay.markers[marker_start..]
        .iter()
        .take_while(|m| m.bar_idx < end_idx)
    {
        let rel = tm.bar_idx - start_idx;
        let x = chart_rect.left() + (rel as f32 + 0.5) * bar_w;
        let y = price_to_y(tm.price);
        if y < chart_rect.top() || y > chart_rect.bottom() {
            continue;
        }
        let (color, arrow_dir) = if tm.is_buy {
            (egui::Color32::from_rgb(76, 175, 80), 1.0_f32) // green, points up (below bar)
        } else {
            (egui::Color32::from_rgb(244, 67, 54), -1.0_f32) // red, points down (above bar)
        };
        let arrow_size = 6.0_f32;
        let y_offset = arrow_size * 2.0 * arrow_dir;
        let tip_y = y + y_offset;
        let base_y = tip_y + arrow_size * arrow_dir;
        let points = vec![
            egui::pos2(x, tip_y),
            egui::pos2(x - arrow_size * 0.6, base_y),
            egui::pos2(x + arrow_size * 0.6, base_y),
        ];
        painter.add(egui::Shape::convex_polygon(
            points,
            color,
            egui::Stroke::NONE,
        ));
        let label_y = if tm.is_buy {
            base_y + 2.0
        } else {
            base_y - 10.0
        };
        pending_labels.push(PendingLabel {
            x,
            y: label_y,
            is_buy: tm.is_buy,
            volume: tm.volume,
            price: tm.price,
            ticker: tm.ticker.clone(),
            count: tm.count,
        });
    }

    // Greedy pixel-proximity clustering per side. CLUSTER_X/Y roughly match the
    // bounding box of an 8pt monospace label so only markers that would
    // actually overlap get merged.
    const CLUSTER_X: f32 = 44.0;
    const CLUSTER_Y: f32 = 12.0;
    struct LabelCluster {
        x_sum: f32,
        y_sum: f32,
        n: u32,
        is_buy: bool,
        volume: f64,
        price_w_sum: f64,
        weight_sum: f64,
        tickers: Vec<String>,
        deals: u32,
    }
    pending_labels.sort_by(|a, b| {
        a.is_buy
            .cmp(&b.is_buy)
            .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
            .then(a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
    });
    let mut clusters: Vec<LabelCluster> = Vec::new();
    'outer: for lbl in pending_labels {
        for c in clusters.iter_mut() {
            if c.is_buy != lbl.is_buy {
                continue;
            }
            let cx = c.x_sum / c.n as f32;
            let cy = c.y_sum / c.n as f32;
            if (cx - lbl.x).abs() < CLUSTER_X && (cy - lbl.y).abs() < CLUSTER_Y {
                let w = lbl.volume.max(1e-6);
                c.x_sum += lbl.x;
                c.y_sum += lbl.y;
                c.n += 1;
                c.volume += lbl.volume;
                c.price_w_sum += lbl.price * w;
                c.weight_sum += w;
                c.deals += lbl.count;
                for t in lbl.ticker.split(", ").filter(|t| !t.is_empty()) {
                    if !c.tickers.iter().any(|x| x == t) {
                        c.tickers.push(t.to_string());
                    }
                }
                continue 'outer;
            }
        }
        let w = lbl.volume.max(1e-6);
        let mut tickers: Vec<String> = Vec::new();
        for t in lbl.ticker.split(", ").filter(|t| !t.is_empty()) {
            if !tickers.iter().any(|x: &String| x == t) {
                tickers.push(t.to_string());
            }
        }
        clusters.push(LabelCluster {
            x_sum: lbl.x,
            y_sum: lbl.y,
            n: 1,
            is_buy: lbl.is_buy,
            volume: lbl.volume,
            price_w_sum: lbl.price * w,
            weight_sum: w,
            tickers,
            deals: lbl.count,
        });
    }
    for c in &clusters {
        let color = if c.is_buy {
            egui::Color32::from_rgb(76, 175, 80)
        } else {
            egui::Color32::from_rgb(244, 67, 54)
        };
        let x = c.x_sum / c.n as f32;
        let y = c.y_sum / c.n as f32;
        let avg_price = if c.weight_sum > 0.0 {
            c.price_w_sum / c.weight_sum
        } else {
            0.0
        };
        let label = if c.tickers.is_empty() {
            format!("{:.2}", c.volume)
        } else if c.n == 1 && c.tickers.len() == 1 {
            if c.deals > 1 || c.volume >= 0.1 {
                format!("{} {:.2}", c.tickers[0], c.volume)
            } else {
                c.tickers[0].clone()
            }
        } else {
            let head = if c.tickers.len() <= 3 {
                c.tickers.join(",")
            } else {
                format!("{}+{}", c.tickers[..2].join(","), c.tickers.len() - 2)
            };
            format!("[{}] @{:.2} {:.2}", head, avg_price, c.volume)
        };
        painter.text(
            egui::pos2(x, y),
            egui::Align2::CENTER_TOP,
            &label,
            egui::FontId::monospace(8.0),
            color,
        );
    }

    // Total signal-account volume across currently visible markers. Sits in the
    // bottom-right corner out of the way of the ATR HUD (top-right) and the
    // position-line labels (top-left).
    let buy_total: f64 = clusters.iter().filter(|c| c.is_buy).map(|c| c.volume).sum();
    let sell_total: f64 = clusters
        .iter()
        .filter(|c| !c.is_buy)
        .map(|c| c.volume)
        .sum();
    if buy_total > 0.0 || sell_total > 0.0 {
        let hud = format!("DARWINS  BUY {:.2}  SELL {:.2}", buy_total, sell_total);
        painter.text(
            egui::pos2(chart_rect.right() - 4.0, chart_rect.bottom() - 4.0),
            egui::Align2::RIGHT_BOTTOM,
            &hud,
            egui::FontId::monospace(9.0),
            egui::Color32::from_gray(210),
        );
    }

    // ── alert price lines ─────────────────────────────────────────────────────
    if !alerts.is_empty() {
        let alert_col = egui::Color32::from_rgb(255, 165, 0); // orange
        let alert_bg = egui::Color32::from_rgba_premultiplied(255, 165, 0, 30);
        for (price, label) in alerts {
            let y = price_to_y(*price);
            if y >= chart_rect.top() && y <= chart_rect.bottom() {
                // Dotted line across chart
                let mut ax = chart_rect.left();
                while ax < chart_rect.right() {
                    let end = (ax + 4.0).min(chart_rect.right());
                    painter.line_segment(
                        [egui::pos2(ax, y), egui::pos2(end, y)],
                        egui::Stroke::new(1.0, alert_col),
                    );
                    ax += 8.0;
                }
                // Label with bell icon
                let lbl = if label.is_empty() {
                    format!("\u{1F514} {}", format_price(*price))
                } else {
                    format!("\u{1F514} {} {}", label, format_price(*price))
                };
                let text_rect = egui::Rect::from_min_size(
                    egui::pos2(chart_rect.left() + 2.0, y - 9.0),
                    egui::vec2(lbl.len() as f32 * 6.5 + 6.0, 16.0),
                );
                painter.rect_filled(text_rect, 2.0, alert_bg);
                painter.text(
                    egui::pos2(chart_rect.left() + 5.0, y),
                    egui::Align2::LEFT_CENTER,
                    &lbl,
                    egui::FontId::monospace(9.0),
                    alert_col,
                );
            }
        }
    }

    // ── drawing annotations ──────────────────────────────────────────────────
    // Helper: draw a line segment respecting the per-drawing LineStyle.
    let draw_line = |painter: &egui::Painter,
                     p1: egui::Pos2,
                     p2: egui::Pos2,
                     stroke: egui::Stroke,
                     style: LineStyle| {
        match style {
            LineStyle::Solid => {
                painter.line_segment([p1, p2], stroke);
            }
            LineStyle::Dashed => {
                let dx = p2.x - p1.x;
                let dy = p2.y - p1.y;
                let len = (dx * dx + dy * dy).sqrt();
                if len < 0.1 {
                    return;
                }
                let (nx, ny) = (dx / len, dy / len);
                let dash = 8.0f32;
                let gap = 5.0f32;
                let mut t = 0.0f32;
                while t < len {
                    let t1 = (t + dash).min(len);
                    painter.line_segment(
                        [
                            egui::pos2(p1.x + nx * t, p1.y + ny * t),
                            egui::pos2(p1.x + nx * t1, p1.y + ny * t1),
                        ],
                        stroke,
                    );
                    t += dash + gap;
                }
            }
            LineStyle::Dotted => {
                let dx = p2.x - p1.x;
                let dy = p2.y - p1.y;
                let len = (dx * dx + dy * dy).sqrt();
                if len < 0.1 {
                    return;
                }
                let (nx, ny) = (dx / len, dy / len);
                let dot = stroke.width.max(2.0);
                let gap = 4.0f32;
                let mut t = 0.0f32;
                while t < len {
                    let t1 = (t + dot).min(len);
                    painter.line_segment(
                        [
                            egui::pos2(p1.x + nx * t, p1.y + ny * t),
                            egui::pos2(p1.x + nx * t1, p1.y + ny * t1),
                        ],
                        stroke,
                    );
                    t += dot + gap;
                }
            }
        }
    };

    for (draw_idx, drawing) in chart.drawings.iter().enumerate() {
        // Per-drawing style: line width + style (with fallback defaults)
        let (d_width, d_style) = chart
            .drawing_styles
            .get(draw_idx)
            .copied()
            .unwrap_or((1.5, LineStyle::Solid));
        let is_selected = chart.selected_drawing == Some(draw_idx);
        // Selection: boost width and tint color slightly cyan
        let sel_boost = if is_selected { 1.5 } else { 0.0 };
        let effective_width = d_width + sel_boost;
        // Tint helper: if selected, blend color toward cyan for visibility
        let sel_tint = |c: egui::Color32| -> egui::Color32 {
            if !is_selected {
                return c;
            }
            egui::Color32::from_rgb(
                c.r().saturating_add(30),
                c.g().saturating_add(50),
                c.b().saturating_add(80),
            )
        };
        match drawing {
            Drawing::HLine { price, color } => {
                let y = price_to_y(*price);
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    draw_line(
                        &painter,
                        egui::pos2(chart_rect.left(), y),
                        egui::pos2(chart_rect.right(), y),
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                        d_style,
                    );
                    painter.text(
                        egui::pos2(chart_rect.right() - 60.0, y - 10.0),
                        egui::Align2::LEFT_TOP,
                        &format_price(*price),
                        egui::FontId::monospace(9.0),
                        *color,
                    );
                }
            }
            Drawing::TrendLine { p1, p2, color } => {
                // Map bar indices to x positions
                let x1 = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(chart_rect.left() + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(chart_rect.left() + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    draw_line(
                        &painter,
                        egui::pos2(x1, y1),
                        egui::pos2(x2, y2),
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                        d_style,
                    );
                }
            }
            Drawing::FiboRetrace {
                high,
                low,
                bar_start,
                bar_end,
            } => {
                let x_start = if *bar_start >= start_idx && *bar_start < end_idx {
                    chart_rect.left() + ((*bar_start - start_idx) as f32 + 0.5) * bar_w
                } else {
                    chart_rect.left()
                };
                let x_end = if *bar_end >= start_idx && *bar_end < end_idx {
                    chart_rect.left() + ((*bar_end - start_idx) as f32 + 0.5) * bar_w
                } else {
                    chart_rect.right()
                };
                let levels = [0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0];
                let range = high - low;
                for &level in &levels {
                    let price = high - range * level;
                    let y = price_to_y(price);
                    if y >= chart_rect.top() && y <= chart_rect.bottom() {
                        painter.line_segment(
                            [egui::pos2(x_start, y), egui::pos2(x_end, y)],
                            egui::Stroke::new(0.8, FIBO_COL),
                        );
                        painter.text(
                            egui::pos2(x_end + 2.0, y - 8.0),
                            egui::Align2::LEFT_TOP,
                            &format!("{:.1}% {}", level * 100.0, format_price(price)),
                            egui::FontId::monospace(8.0),
                            FIBO_COL,
                        );
                    }
                }
            }
            Drawing::VLine { bar_idx, color } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = chart_rect.left() + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    draw_line(
                        &painter,
                        egui::pos2(x, chart_rect.top()),
                        egui::pos2(x, chart_rect.bottom()),
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                        d_style,
                    );
                }
            }
            Drawing::Rectangle { p1, p2, color } => {
                let x1 = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(chart_rect.left() + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(chart_rect.left() + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    let r = egui::Rect::from_two_pos(egui::pos2(x1, y1), egui::pos2(x2, y2));
                    painter.rect_filled(r, 0.0, *color);
                    painter.rect_stroke(
                        r,
                        0.0,
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                        egui::StrokeKind::Outside,
                    );
                }
            }
            Drawing::Ray {
                origin,
                slope,
                color,
            } => {
                if origin.0 >= start_idx && origin.0 < end_idx {
                    let x1 = chart_rect.left() + ((origin.0 - start_idx) as f32 + 0.5) * bar_w;
                    let y1 = price_to_y(origin.1);
                    let bars_to_edge = ((chart_rect.right() - x1) / bar_w) as f64;
                    let end_price = origin.1 + slope * bars_to_edge;
                    let y2 = price_to_y(end_price);
                    draw_line(
                        &painter,
                        egui::pos2(x1, y1),
                        egui::pos2(chart_rect.right(), y2),
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                        d_style,
                    );
                }
            }
            Drawing::Channel {
                p1,
                p2,
                width,
                color,
            } => {
                let x1 = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(chart_rect.left() + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(chart_rect.left() + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    let y1b = price_to_y(p1.1 + width);
                    let y2b = price_to_y(p2.1 + width);
                    let sc = sel_tint(*color);
                    draw_line(
                        &painter,
                        egui::pos2(x1, y1),
                        egui::pos2(x2, y2),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    draw_line(
                        &painter,
                        egui::pos2(x1, y1b),
                        egui::pos2(x2, y2b),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    let fill =
                        egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 20);
                    let poly = vec![
                        egui::pos2(x1, y1),
                        egui::pos2(x2, y2),
                        egui::pos2(x2, y2b),
                        egui::pos2(x1, y1b),
                    ];
                    painter.add(egui::Shape::convex_polygon(poly, fill, egui::Stroke::NONE));
                }
            }
            Drawing::ExtendedLine { p1, p2, color } => {
                // Extend line infinitely in both directions across visible chart
                if p1.0 != p2.0 {
                    let slope = (p2.1 - p1.1) / (p2.0 as f64 - p1.0 as f64);
                    let price_at_start = p1.1 + slope * (start_idx as f64 - p1.0 as f64);
                    let price_at_end = p1.1 + slope * (end_idx as f64 - p1.0 as f64);
                    let y1 = price_to_y(price_at_start);
                    let y2 = price_to_y(price_at_end);
                    draw_line(
                        &painter,
                        egui::pos2(chart_rect.left(), y1),
                        egui::pos2(chart_rect.right(), y2),
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                        d_style,
                    );
                }
            }
            Drawing::HRay {
                bar_idx,
                price,
                color,
            } => {
                let y = price_to_y(*price);
                let x_start = if *bar_idx >= start_idx && *bar_idx < end_idx {
                    chart_rect.left() + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w
                } else {
                    chart_rect.left()
                }; // bar left of view — draw full width
                draw_line(
                    &painter,
                    egui::pos2(x_start, y),
                    egui::pos2(chart_rect.right(), y),
                    egui::Stroke::new(effective_width, sel_tint(*color)),
                    d_style,
                );
            }
            Drawing::CrossLine {
                bar_idx,
                price,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = chart_rect.left() + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    let sc = sel_tint(*color);
                    let sw = egui::Stroke::new(effective_width, sc);
                    draw_line(
                        &painter,
                        egui::pos2(x, chart_rect.top()),
                        egui::pos2(x, chart_rect.bottom()),
                        sw,
                        d_style,
                    );
                    draw_line(
                        &painter,
                        egui::pos2(chart_rect.left(), y),
                        egui::pos2(chart_rect.right(), y),
                        sw,
                        d_style,
                    );
                }
            }
            Drawing::ArrowLine { p1, p2, color } => {
                let x1 = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(chart_rect.left() + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(chart_rect.left() + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    let sc = sel_tint(*color);
                    draw_line(
                        &painter,
                        egui::pos2(x1, y1),
                        egui::pos2(x2, y2),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    // Arrowhead at p2
                    let dx = x2 - x1;
                    let dy = y2 - y1;
                    let len = (dx * dx + dy * dy).sqrt().max(1.0);
                    let ux = dx / len;
                    let uy = dy / len;
                    let sz = 8.0_f32;
                    let ax = x2 - ux * sz + uy * sz * 0.4;
                    let ay = y2 - uy * sz - ux * sz * 0.4;
                    let bx = x2 - ux * sz - uy * sz * 0.4;
                    let by = y2 - uy * sz + ux * sz * 0.4;
                    painter.add(egui::Shape::convex_polygon(
                        vec![egui::pos2(x2, y2), egui::pos2(ax, ay), egui::pos2(bx, by)],
                        sc,
                        egui::Stroke::NONE,
                    ));
                }
            }
            Drawing::InfoLine { p1, p2, color } => {
                let x1 = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(chart_rect.left() + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(chart_rect.left() + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    draw_line(
                        &painter,
                        egui::pos2(x1, y1),
                        egui::pos2(x2, y2),
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                        d_style,
                    );
                    // Info label: distance, percent, bars
                    let dist = p2.1 - p1.1;
                    let pct = if p1.1.abs() > f64::EPSILON {
                        dist / p1.1 * 100.0
                    } else {
                        0.0
                    };
                    let bar_count = if p2.0 > p1.0 {
                        p2.0 - p1.0
                    } else {
                        p1.0 - p2.0
                    };
                    let label = format!("{:.2} ({:+.2}%) {} bars", dist, pct, bar_count);
                    let mid_x = (x1 + x2) / 2.0;
                    let mid_y = (y1 + y2) / 2.0 - 12.0;
                    painter.text(
                        egui::pos2(mid_x, mid_y),
                        egui::Align2::CENTER_BOTTOM,
                        &label,
                        egui::FontId::monospace(10.0),
                        *color,
                    );
                }
            }
            Drawing::Pitchfork {
                pivot,
                p2,
                p3,
                color,
            } => {
                // Andrews Pitchfork: median line from pivot to midpoint(p2,p3), parallel upper/lower
                let to_x = |idx: usize| -> Option<f32> {
                    if idx >= start_idx && idx < end_idx {
                        Some(chart_rect.left() + ((idx - start_idx) as f32 + 0.5) * bar_w)
                    } else {
                        None
                    }
                };
                if let (Some(xp), Some(x2), Some(x3)) = (to_x(pivot.0), to_x(p2.0), to_x(p3.0)) {
                    let yp = price_to_y(pivot.1);
                    let y2 = price_to_y(p2.1);
                    let y3 = price_to_y(p3.1);
                    let mid_x = (x2 + x3) / 2.0;
                    let mid_y = (y2 + y3) / 2.0;
                    // Median line (extended to chart edge)
                    let dx = mid_x - xp;
                    let dy = mid_y - yp;
                    let ext = if dx.abs() > 0.1 {
                        (chart_rect.right() - xp) / dx
                    } else {
                        1.0
                    };
                    let end_x = xp + dx * ext;
                    let end_y = yp + dy * ext;
                    let sc = sel_tint(*color);
                    let sw = egui::Stroke::new(effective_width, sc);
                    draw_line(
                        &painter,
                        egui::pos2(xp, yp),
                        egui::pos2(end_x, end_y),
                        sw,
                        d_style,
                    );
                    // Upper line (through p2, parallel to median)
                    let ux = x2 + dx * ext;
                    let uy = y2 + dy * ext;
                    draw_line(
                        &painter,
                        egui::pos2(x2, y2),
                        egui::pos2(ux.min(chart_rect.right()), uy),
                        sw,
                        d_style,
                    );
                    // Lower line (through p3, parallel to median)
                    let lx = x3 + dx * ext;
                    let ly = y3 + dy * ext;
                    draw_line(
                        &painter,
                        egui::pos2(x3, y3),
                        egui::pos2(lx.min(chart_rect.right()), ly),
                        sw,
                        d_style,
                    );
                }
            }
            Drawing::FiboExtension { p1, p2, p3, color } => {
                let to_x = |idx: usize| -> Option<f32> {
                    if idx >= start_idx && idx < end_idx {
                        Some(chart_rect.left() + ((idx - start_idx) as f32 + 0.5) * bar_w)
                    } else {
                        None
                    }
                };
                if let Some(x3) = to_x(p3.0) {
                    let range = (p2.1 - p1.1).abs();
                    let base = p3.1;
                    let dir = if p2.1 > p1.1 { 1.0 } else { -1.0 };
                    let levels = [0.0, 0.618, 1.0, 1.272, 1.618, 2.0, 2.618];
                    let names = ["0%", "61.8%", "100%", "127.2%", "161.8%", "200%", "261.8%"];
                    let sc = sel_tint(*color);
                    for (i, &lvl) in levels.iter().enumerate() {
                        let price = base + dir * range * lvl;
                        let y = price_to_y(price);
                        if y >= chart_rect.top() && y <= chart_rect.bottom() {
                            let alpha = if lvl == 1.0 || lvl == 1.618 { 180 } else { 100 };
                            let c = egui::Color32::from_rgba_premultiplied(
                                sc.r(),
                                sc.g(),
                                sc.b(),
                                alpha,
                            );
                            let lw = if lvl == 1.0 || lvl == 1.618 {
                                effective_width
                            } else {
                                effective_width * 0.65
                            };
                            draw_line(
                                &painter,
                                egui::pos2(x3, y),
                                egui::pos2(chart_rect.right(), y),
                                egui::Stroke::new(lw, c),
                                d_style,
                            );
                            painter.text(
                                egui::pos2(chart_rect.right() - 60.0, y - 10.0),
                                egui::Align2::LEFT_BOTTOM,
                                names[i],
                                egui::FontId::monospace(9.0),
                                c,
                            );
                        }
                    }
                }
            }
            Drawing::GannFan {
                origin,
                scale,
                color,
            } => {
                if origin.0 >= start_idx && origin.0 < end_idx {
                    let ox = chart_rect.left() + ((origin.0 - start_idx) as f32 + 0.5) * bar_w;
                    let oy = price_to_y(origin.1);
                    // Gann angles: 1×8, 1×4, 1×3, 1×2, 1×1, 2×1, 3×1, 4×1, 8×1
                    let ratios: &[(f64, &str)] = &[
                        (0.125, "1×8"),
                        (0.25, "1×4"),
                        (0.333, "1×3"),
                        (0.5, "1×2"),
                        (1.0, "1×1"),
                        (2.0, "2×1"),
                        (3.0, "3×1"),
                        (4.0, "4×1"),
                        (8.0, "8×1"),
                    ];
                    let sc = sel_tint(*color);
                    for &(ratio, label) in ratios {
                        let bars_to_edge = ((chart_rect.right() - ox) / bar_w) as f64;
                        let end_price = origin.1 + scale * ratio * bars_to_edge;
                        let end_y = price_to_y(end_price);
                        let alpha = if ratio == 1.0 { 200 } else { 100 };
                        let c =
                            egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), alpha);
                        let w = if ratio == 1.0 {
                            effective_width
                        } else {
                            effective_width * 0.55
                        };
                        draw_line(
                            &painter,
                            egui::pos2(ox, oy),
                            egui::pos2(chart_rect.right(), end_y),
                            egui::Stroke::new(w, c),
                            d_style,
                        );
                        painter.text(
                            egui::pos2(chart_rect.right() - 2.0, end_y),
                            egui::Align2::RIGHT_CENTER,
                            label,
                            egui::FontId::monospace(8.0),
                            c,
                        );
                        // Downward mirror
                        let dn_price = origin.1 - scale * ratio * bars_to_edge;
                        let dn_y = price_to_y(dn_price);
                        draw_line(
                            &painter,
                            egui::pos2(ox, oy),
                            egui::pos2(chart_rect.right(), dn_y),
                            egui::Stroke::new(w, c),
                            d_style,
                        );
                    }
                }
            }
            Drawing::LongPosition {
                entry,
                stop,
                target,
            } => {
                if entry.0 >= start_idx && entry.0 < end_idx {
                    let x = chart_rect.left() + ((entry.0 - start_idx) as f32 + 0.5) * bar_w;
                    let ye = price_to_y(entry.1);
                    let ys = price_to_y(*stop);
                    let yt = price_to_y(*target);
                    let w = (chart_rect.right() - x).min(200.0);
                    // Stop zone (red)
                    painter.rect_filled(
                        egui::Rect::from_min_max(egui::pos2(x, ye), egui::pos2(x + w, ys)),
                        0.0,
                        egui::Color32::from_rgba_premultiplied(220, 40, 40, 30),
                    );
                    // Target zone (green)
                    painter.rect_filled(
                        egui::Rect::from_min_max(egui::pos2(x, yt), egui::pos2(x + w, ye)),
                        0.0,
                        egui::Color32::from_rgba_premultiplied(0, 200, 80, 30),
                    );
                    // Entry line
                    painter.line_segment(
                        [egui::pos2(x, ye), egui::pos2(x + w, ye)],
                        egui::Stroke::new(1.5, egui::Color32::WHITE),
                    );
                    // R:R label
                    let risk = (entry.1 - stop).abs();
                    let reward = (target - entry.1).abs();
                    let rr = if risk > f64::EPSILON {
                        reward / risk
                    } else {
                        0.0
                    };
                    painter.text(
                        egui::pos2(x + w + 4.0, ye),
                        egui::Align2::LEFT_CENTER,
                        &format!("R:R {:.1}", rr),
                        egui::FontId::monospace(10.0),
                        egui::Color32::WHITE,
                    );
                }
            }
            Drawing::ShortPosition {
                entry,
                stop,
                target,
            } => {
                if entry.0 >= start_idx && entry.0 < end_idx {
                    let x = chart_rect.left() + ((entry.0 - start_idx) as f32 + 0.5) * bar_w;
                    let ye = price_to_y(entry.1);
                    let ys = price_to_y(*stop);
                    let yt = price_to_y(*target);
                    let w = (chart_rect.right() - x).min(200.0);
                    // Stop zone (red, above entry for short)
                    painter.rect_filled(
                        egui::Rect::from_min_max(egui::pos2(x, ys), egui::pos2(x + w, ye)),
                        0.0,
                        egui::Color32::from_rgba_premultiplied(220, 40, 40, 30),
                    );
                    // Target zone (green, below entry for short)
                    painter.rect_filled(
                        egui::Rect::from_min_max(egui::pos2(x, ye), egui::pos2(x + w, yt)),
                        0.0,
                        egui::Color32::from_rgba_premultiplied(0, 200, 80, 30),
                    );
                    painter.line_segment(
                        [egui::pos2(x, ye), egui::pos2(x + w, ye)],
                        egui::Stroke::new(1.5, egui::Color32::WHITE),
                    );
                    let risk = (stop - entry.1).abs();
                    let reward = (entry.1 - target).abs();
                    let rr = if risk > f64::EPSILON {
                        reward / risk
                    } else {
                        0.0
                    };
                    painter.text(
                        egui::pos2(x + w + 4.0, ye),
                        egui::Align2::LEFT_CENTER,
                        &format!("R:R {:.1}", rr),
                        egui::FontId::monospace(10.0),
                        egui::Color32::WHITE,
                    );
                }
            }
            Drawing::PriceRange { p1, p2 } => {
                let x1 = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(chart_rect.left() + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(chart_rect.left() + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    let fill = egui::Color32::from_rgba_premultiplied(100, 150, 255, 20);
                    painter.rect_filled(
                        egui::Rect::from_two_pos(egui::pos2(x1, y1), egui::pos2(x2, y2)),
                        0.0,
                        fill,
                    );
                    let dist = p2.1 - p1.1;
                    let pct = if p1.1.abs() > f64::EPSILON {
                        dist / p1.1 * 100.0
                    } else {
                        0.0
                    };
                    let bars = if p2.0 > p1.0 {
                        p2.0 - p1.0
                    } else {
                        p1.0 - p2.0
                    };
                    let label = format!("{:.2} ({:+.2}%) {} bars", dist, pct, bars);
                    painter.text(
                        egui::pos2((x1 + x2) / 2.0, y1.min(y2) - 4.0),
                        egui::Align2::CENTER_BOTTOM,
                        &label,
                        egui::FontId::monospace(10.0),
                        egui::Color32::from_rgb(100, 150, 255),
                    );
                }
            }
            Drawing::TextLabel {
                bar_idx,
                price,
                text,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = chart_rect.left() + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    painter.text(
                        egui::pos2(x, y),
                        egui::Align2::CENTER_CENTER,
                        text,
                        egui::FontId::monospace(11.0),
                        *color,
                    );
                }
            }
            Drawing::ArrowMarker {
                bar_idx,
                price,
                is_up,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = chart_rect.left() + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    let sz = 8.0_f32;
                    if *is_up {
                        let pts = vec![
                            egui::pos2(x, y - sz),
                            egui::pos2(x - sz * 0.6, y + sz * 0.3),
                            egui::pos2(x + sz * 0.6, y + sz * 0.3),
                        ];
                        painter.add(egui::Shape::convex_polygon(pts, *color, egui::Stroke::NONE));
                    } else {
                        let pts = vec![
                            egui::pos2(x, y + sz),
                            egui::pos2(x - sz * 0.6, y - sz * 0.3),
                            egui::pos2(x + sz * 0.6, y - sz * 0.3),
                        ];
                        painter.add(egui::Shape::convex_polygon(pts, *color, egui::Stroke::NONE));
                    }
                }
            }
            Drawing::Ellipse { p1, p2, color } => {
                let x1 = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(chart_rect.left() + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(chart_rect.left() + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    let cx = (x1 + x2) / 2.0;
                    let cy = (y1 + y2) / 2.0;
                    let rx = (x2 - x1).abs() / 2.0;
                    let ry = (y2 - y1).abs() / 2.0;
                    let n_pts = 48;
                    let pts: Vec<egui::Pos2> = (0..n_pts)
                        .map(|i| {
                            let a = 2.0 * std::f32::consts::PI * i as f32 / n_pts as f32;
                            egui::pos2(cx + rx * a.cos(), cy + ry * a.sin())
                        })
                        .collect();
                    let sc = sel_tint(*color);
                    let fill = egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), 20);
                    painter.add(egui::Shape::convex_polygon(
                        pts,
                        fill,
                        egui::Stroke::new(effective_width, sc),
                    ));
                }
            }
            Drawing::Triangle { p1, p2, p3, color } => {
                let to_pt = |idx: usize, price: f64| -> Option<egui::Pos2> {
                    if idx >= start_idx && idx < end_idx {
                        let x = chart_rect.left() + ((idx - start_idx) as f32 + 0.5) * bar_w;
                        Some(egui::pos2(x, price_to_y(price)))
                    } else {
                        None
                    }
                };
                if let (Some(a), Some(b), Some(c)) =
                    (to_pt(p1.0, p1.1), to_pt(p2.0, p2.1), to_pt(p3.0, p3.1))
                {
                    let sc = sel_tint(*color);
                    let fill = egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), 20);
                    painter.add(egui::Shape::convex_polygon(
                        vec![a, b, c],
                        fill,
                        egui::Stroke::new(effective_width, sc),
                    ));
                }
            }
            Drawing::TrendAngle { p1, p2, color } => {
                let x1 = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(chart_rect.left() + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(chart_rect.left() + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    draw_line(
                        &painter,
                        egui::pos2(x1, y1),
                        egui::pos2(x2, y2),
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                        d_style,
                    );
                    // Angle display
                    let dx = x2 - x1;
                    let dy = y2 - y1;
                    let angle_deg = (dy / dx).atan().to_degrees();
                    painter.text(
                        egui::pos2((x1 + x2) / 2.0, (y1 + y2) / 2.0 - 12.0),
                        egui::Align2::CENTER_BOTTOM,
                        &format!("{:.1}°", angle_deg),
                        egui::FontId::monospace(10.0),
                        sel_tint(*color),
                    );
                }
            }
            Drawing::ParallelChannel {
                p1,
                p2,
                offset,
                color,
            } => {
                let x1 = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(chart_rect.left() + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(chart_rect.left() + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    let y1u = price_to_y(p1.1 + offset);
                    let y2u = price_to_y(p2.1 + offset);
                    let y1d = price_to_y(p1.1 - offset);
                    let y2d = price_to_y(p2.1 - offset);
                    let sc = sel_tint(*color);
                    // Center line (dashed-style: thinner)
                    draw_line(
                        &painter,
                        egui::pos2(x1, y1),
                        egui::pos2(x2, y2),
                        egui::Stroke::new(effective_width * 0.5, sc),
                        d_style,
                    );
                    // Upper boundary
                    draw_line(
                        &painter,
                        egui::pos2(x1, y1u),
                        egui::pos2(x2, y2u),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    // Lower boundary
                    draw_line(
                        &painter,
                        egui::pos2(x1, y1d),
                        egui::pos2(x2, y2d),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    // Fill between upper and lower
                    let fill =
                        egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 15);
                    let poly = vec![
                        egui::pos2(x1, y1u),
                        egui::pos2(x2, y2u),
                        egui::pos2(x2, y2d),
                        egui::pos2(x1, y1d),
                    ];
                    painter.add(egui::Shape::convex_polygon(poly, fill, egui::Stroke::NONE));
                }
            }
            Drawing::FibChannel { p1, p2, p3, color } => {
                let to_x = |idx: usize| -> Option<f32> {
                    if idx >= start_idx && idx < end_idx {
                        Some(chart_rect.left() + ((idx - start_idx) as f32 + 0.5) * bar_w)
                    } else {
                        None
                    }
                };
                if let (Some(x1), Some(x2)) = (to_x(p1.0), to_x(p2.0)) {
                    // Channel width from p3 offset perpendicular to the trendline
                    let ch_offset = p3.1 - p1.1; // price offset defining full channel width
                    let levels = [0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0];
                    let names = ["0%", "23.6%", "38.2%", "50%", "61.8%", "78.6%", "100%"];
                    let sc = sel_tint(*color);
                    for (i, &lvl) in levels.iter().enumerate() {
                        let off = ch_offset * lvl;
                        let ly1 = price_to_y(p1.1 + off);
                        let ly2 = price_to_y(p2.1 + off);
                        let alpha = if lvl == 0.0 || lvl == 0.5 || lvl == 1.0 {
                            180
                        } else {
                            100
                        };
                        let c =
                            egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), alpha);
                        let w = if lvl == 0.0 || lvl == 1.0 {
                            effective_width
                        } else {
                            effective_width * 0.55
                        };
                        draw_line(
                            &painter,
                            egui::pos2(x1, ly1),
                            egui::pos2(x2, ly2),
                            egui::Stroke::new(w, c),
                            d_style,
                        );
                        painter.text(
                            egui::pos2(x2 + 4.0, ly2),
                            egui::Align2::LEFT_CENTER,
                            names[i],
                            egui::FontId::monospace(8.0),
                            c,
                        );
                    }
                    // Fill 0-100%
                    let fill = egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), 10);
                    let poly = vec![
                        egui::pos2(x1, price_to_y(p1.1)),
                        egui::pos2(x2, price_to_y(p2.1)),
                        egui::pos2(x2, price_to_y(p2.1 + ch_offset)),
                        egui::pos2(x1, price_to_y(p1.1 + ch_offset)),
                    ];
                    painter.add(egui::Shape::convex_polygon(poly, fill, egui::Stroke::NONE));
                }
            }
            Drawing::FibTimeZones { bar_idx, color } => {
                // Draw vertical lines at Fibonacci intervals: 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144
                let fibs = [1usize, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233];
                let mut cumulative = 0usize;
                for &f in &fibs {
                    cumulative += f;
                    let idx = bar_idx + cumulative;
                    if idx >= start_idx && idx < end_idx {
                        let x = chart_rect.left() + ((idx - start_idx) as f32 + 0.5) * bar_w;
                        let alpha = if f <= 3 { 120 } else { 80 };
                        let sc = sel_tint(*color);
                        let c =
                            egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), alpha);
                        draw_line(
                            &painter,
                            egui::pos2(x, chart_rect.top()),
                            egui::pos2(x, chart_rect.bottom()),
                            egui::Stroke::new(effective_width * 0.65, c),
                            d_style,
                        );
                        painter.text(
                            egui::pos2(x + 2.0, chart_rect.top() + 2.0),
                            egui::Align2::LEFT_TOP,
                            &format!("{}", cumulative),
                            egui::FontId::monospace(8.0),
                            c,
                        );
                    }
                }
            }
            Drawing::PriceLabel {
                bar_idx,
                price,
                color,
            } => {
                let y = price_to_y(*price);
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    // Horizontal line from bar to right edge
                    let x_start = if *bar_idx >= start_idx && *bar_idx < end_idx {
                        chart_rect.left() + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w
                    } else if *bar_idx < start_idx {
                        chart_rect.left()
                    } else {
                        return; // bar beyond visible range
                    };
                    let sc = sel_tint(*color);
                    draw_line(
                        &painter,
                        egui::pos2(x_start, y),
                        egui::pos2(chart_rect.right(), y),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    // Price badge on the right
                    let label = format!("{:.5}", price);
                    let badge_w = 65.0_f32;
                    let badge_h = 14.0_f32;
                    let badge_rect = egui::Rect::from_min_size(
                        egui::pos2(chart_rect.right() - badge_w, y - badge_h / 2.0),
                        egui::vec2(badge_w, badge_h),
                    );
                    painter.rect_filled(badge_rect, 2.0, *color);
                    let text_col = if (color.r() as u16 + color.g() as u16 + color.b() as u16) > 384
                    {
                        egui::Color32::BLACK
                    } else {
                        egui::Color32::WHITE
                    };
                    painter.text(
                        badge_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        &label,
                        egui::FontId::monospace(9.0),
                        text_col,
                    );
                }
            }
            Drawing::Callout {
                anchor,
                label_pos,
                text,
                color,
            } => {
                let to_x = |idx: usize| -> Option<f32> {
                    if idx >= start_idx && idx < end_idx {
                        Some(chart_rect.left() + ((idx - start_idx) as f32 + 0.5) * bar_w)
                    } else {
                        None
                    }
                };
                if let (Some(ax), Some(lx)) = (to_x(anchor.0), to_x(label_pos.0)) {
                    let ay = price_to_y(anchor.1);
                    let ly = price_to_y(label_pos.1);
                    // Arrow line from label to anchor
                    painter.line_segment(
                        [egui::pos2(lx, ly), egui::pos2(ax, ay)],
                        egui::Stroke::new(1.0, *color),
                    );
                    // Arrowhead at anchor
                    let dx = ax - lx;
                    let dy = ay - ly;
                    let len = (dx * dx + dy * dy).sqrt().max(1.0);
                    let ux = dx / len;
                    let uy = dy / len;
                    let sz = 6.0_f32;
                    let a1 = egui::pos2(ax - ux * sz + uy * sz * 0.4, ay - uy * sz - ux * sz * 0.4);
                    let a2 = egui::pos2(ax - ux * sz - uy * sz * 0.4, ay - uy * sz + ux * sz * 0.4);
                    painter.add(egui::Shape::convex_polygon(
                        vec![egui::pos2(ax, ay), a1, a2],
                        *color,
                        egui::Stroke::NONE,
                    ));
                    // Text box at label_pos
                    let pad = 4.0_f32;
                    let galley =
                        painter.layout_no_wrap(text.clone(), egui::FontId::monospace(10.0), *color);
                    let tw = galley.rect.width();
                    let th = galley.rect.height();
                    let box_rect = egui::Rect::from_min_size(
                        egui::pos2(lx - tw / 2.0 - pad, ly - th / 2.0 - pad),
                        egui::vec2(tw + pad * 2.0, th + pad * 2.0),
                    );
                    let bg = egui::Color32::from_rgba_premultiplied(20, 20, 30, 220);
                    painter.rect_filled(box_rect, 3.0, bg);
                    painter.rect_stroke(
                        box_rect,
                        3.0,
                        egui::Stroke::new(1.0, *color),
                        egui::StrokeKind::Outside,
                    );
                    painter.galley(egui::pos2(lx - tw / 2.0, ly - th / 2.0), galley, *color);
                }
            }
            Drawing::Highlighter { p1, p2, color } => {
                let x1 = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(chart_rect.left() + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(chart_rect.left() + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    let sc = sel_tint(*color);
                    let fill = egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), 40);
                    painter.rect_filled(
                        egui::Rect::from_two_pos(egui::pos2(x1, y1), egui::pos2(x2, y2)),
                        0.0,
                        fill,
                    );
                    // Border
                    painter.rect_stroke(
                        egui::Rect::from_two_pos(egui::pos2(x1, y1), egui::pos2(x2, y2)),
                        0.0,
                        egui::Stroke::new(effective_width, sc),
                        egui::StrokeKind::Outside,
                    );
                }
            }
            Drawing::CrossMarker {
                bar_idx,
                price,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = chart_rect.left() + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    let sz = 6.0_f32;
                    let sc = sel_tint(*color);
                    let sw = egui::Stroke::new(effective_width, sc);
                    // + shape
                    draw_line(
                        &painter,
                        egui::pos2(x - sz, y),
                        egui::pos2(x + sz, y),
                        sw,
                        d_style,
                    );
                    draw_line(
                        &painter,
                        egui::pos2(x, y - sz),
                        egui::pos2(x, y + sz),
                        sw,
                        d_style,
                    );
                }
            }
            Drawing::Polyline { points, color } => {
                let mut screen_pts: Vec<egui::Pos2> = Vec::with_capacity(points.len());
                for &(idx, price) in points.iter() {
                    if idx >= start_idx && idx < end_idx {
                        let x = chart_rect.left() + ((idx - start_idx) as f32 + 0.5) * bar_w;
                        screen_pts.push(egui::pos2(x, price_to_y(price)));
                    }
                }
                if screen_pts.len() > 1 {
                    painter.add(egui::Shape::line(
                        screen_pts,
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                    ));
                }
            }
            Drawing::AnchorNote {
                bar_idx,
                price,
                text,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = chart_rect.left() + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    let pad = 4.0_f32;
                    let galley =
                        painter.layout_no_wrap(text.clone(), egui::FontId::monospace(10.0), *color);
                    let tw = galley.rect.width();
                    let th = galley.rect.height();
                    let box_rect = egui::Rect::from_min_size(
                        egui::pos2(x - pad, y - th - pad * 2.0),
                        egui::vec2(tw + pad * 2.0, th + pad * 2.0),
                    );
                    let bg = egui::Color32::from_rgba_premultiplied(15, 15, 25, 230);
                    painter.rect_filled(box_rect, 3.0, bg);
                    painter.rect_stroke(
                        box_rect,
                        3.0,
                        egui::Stroke::new(1.0, *color),
                        egui::StrokeKind::Outside,
                    );
                    painter.galley(egui::pos2(x, y - th - pad), galley, *color);
                    // Small triangle pointer down to the anchor point
                    let tri = vec![
                        egui::pos2(x + 4.0, y - pad),
                        egui::pos2(x + 10.0, y - pad),
                        egui::pos2(x + 7.0, y),
                    ];
                    painter.add(egui::Shape::convex_polygon(tri, *color, egui::Stroke::NONE));
                }
            }
            Drawing::RegressionChannel { p1, p2, color } => {
                // Linear regression of close prices between p1 and p2 bars
                let b1 = p1.0.min(p2.0);
                let b2 = p1.0.max(p2.0);
                if b2 > b1 && b1 < end_idx && b2 >= start_idx {
                    // Compute regression from bar data
                    let n = (b2 - b1 + 1) as f64;
                    let mut sum_x = 0.0_f64;
                    let mut sum_y = 0.0_f64;
                    let mut sum_xy = 0.0_f64;
                    let mut sum_xx = 0.0_f64;
                    let mut count = 0u32;
                    for idx in b1..=b2 {
                        if idx < bars.len() {
                            let xi = (idx - b1) as f64;
                            let yi = bars[idx].close;
                            sum_x += xi;
                            sum_y += yi;
                            sum_xy += xi * yi;
                            sum_xx += xi * xi;
                            count += 1;
                        }
                    }
                    if count > 1 {
                        let cn = count as f64;
                        let slope = (cn * sum_xy - sum_x * sum_y) / (cn * sum_xx - sum_x * sum_x);
                        let intercept = (sum_y - slope * sum_x) / cn;
                        // Standard deviation from regression line
                        let mut sum_sq = 0.0_f64;
                        for idx in b1..=b2 {
                            if idx < bars.len() {
                                let xi = (idx - b1) as f64;
                                let predicted = intercept + slope * xi;
                                let diff = bars[idx].close - predicted;
                                sum_sq += diff * diff;
                            }
                        }
                        let std_dev = (sum_sq / cn).sqrt();
                        // Draw regression line + 1 StdDev bands
                        let x_start = if b1 >= start_idx && b1 < end_idx {
                            chart_rect.left() + ((b1 - start_idx) as f32 + 0.5) * bar_w
                        } else {
                            chart_rect.left()
                        };
                        let x_end = if b2 >= start_idx && b2 < end_idx {
                            chart_rect.left() + ((b2 - start_idx) as f32 + 0.5) * bar_w
                        } else {
                            chart_rect.right()
                        };
                        let reg_y1 = price_to_y(intercept);
                        let reg_y2 = price_to_y(intercept + slope * n);
                        let sc = sel_tint(*color);
                        // Center line
                        draw_line(
                            &painter,
                            egui::pos2(x_start, reg_y1),
                            egui::pos2(x_end, reg_y2),
                            egui::Stroke::new(effective_width, sc),
                            d_style,
                        );
                        // Upper band (+1 StdDev)
                        let uy1 = price_to_y(intercept + std_dev);
                        let uy2 = price_to_y(intercept + slope * n + std_dev);
                        draw_line(
                            &painter,
                            egui::pos2(x_start, uy1),
                            egui::pos2(x_end, uy2),
                            egui::Stroke::new(effective_width * 0.55, sc),
                            d_style,
                        );
                        // Lower band (-1 StdDev)
                        let dy1 = price_to_y(intercept - std_dev);
                        let dy2 = price_to_y(intercept + slope * n - std_dev);
                        draw_line(
                            &painter,
                            egui::pos2(x_start, dy1),
                            egui::pos2(x_end, dy2),
                            egui::Stroke::new(effective_width * 0.55, sc),
                            d_style,
                        );
                        // Fill between bands
                        let fill = egui::Color32::from_rgba_premultiplied(
                            color.r(),
                            color.g(),
                            color.b(),
                            15,
                        );
                        let poly = vec![
                            egui::pos2(x_start, uy1),
                            egui::pos2(x_end, uy2),
                            egui::pos2(x_end, dy2),
                            egui::pos2(x_start, dy1),
                        ];
                        painter.add(egui::Shape::convex_polygon(poly, fill, egui::Stroke::NONE));
                    }
                }
            }
            Drawing::GannBox { p1, p2, color } => {
                let x1o = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(chart_rect.left() + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2o = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(chart_rect.left() + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1o, x2o) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    let rect_d = egui::Rect::from_two_pos(egui::pos2(x1, y1), egui::pos2(x2, y2));
                    let fill =
                        egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 12);
                    painter.rect_filled(rect_d, 0.0, fill);
                    painter.rect_stroke(
                        rect_d,
                        0.0,
                        egui::Stroke::new(1.0, *color),
                        egui::StrokeKind::Outside,
                    );
                    // Gann grid: horizontal levels at Gann ratios
                    let gann_h: &[f64] = &[0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875, 1.0];
                    for &ratio in gann_h {
                        let p = p1.1 + (p2.1 - p1.1) * ratio;
                        let yy = price_to_y(p);
                        let alpha = if ratio == 0.5 { 120 } else { 50 };
                        let c = egui::Color32::from_rgba_premultiplied(
                            color.r(),
                            color.g(),
                            color.b(),
                            alpha,
                        );
                        painter.line_segment(
                            [egui::pos2(x1, yy), egui::pos2(x2, yy)],
                            egui::Stroke::new(0.5, c),
                        );
                    }
                    // Vertical grid at same ratios
                    for &ratio in gann_h {
                        let xx = x1 + (x2 - x1) * ratio as f32;
                        let alpha = if ratio == 0.5 { 120 } else { 50 };
                        let c = egui::Color32::from_rgba_premultiplied(
                            color.r(),
                            color.g(),
                            color.b(),
                            alpha,
                        );
                        painter.line_segment(
                            [egui::pos2(xx, y1), egui::pos2(xx, y2)],
                            egui::Stroke::new(0.5, c),
                        );
                    }
                    // Diagonal 1×1 from corners
                    let c_diag =
                        egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 80);
                    painter.line_segment(
                        [egui::pos2(x1, y1), egui::pos2(x2, y2)],
                        egui::Stroke::new(0.8, c_diag),
                    );
                    painter.line_segment(
                        [egui::pos2(x2, y1), egui::pos2(x1, y2)],
                        egui::Stroke::new(0.8, c_diag),
                    );
                }
            }
            Drawing::ElliottWave { points, color } => {
                let mut screen_pts: Vec<(f32, f32)> = Vec::new();
                for &(bi, pr) in points.iter() {
                    if bi >= start_idx && bi < end_idx {
                        let x = chart_rect.left() + ((bi - start_idx) as f32 + 0.5) * bar_w;
                        let y = price_to_y(pr);
                        screen_pts.push((x, y));
                    }
                }
                let labels = ["1", "2", "3", "4", "5"];
                let sc = sel_tint(*color);
                for i in 0..screen_pts.len() {
                    if i + 1 < screen_pts.len() {
                        draw_line(
                            &painter,
                            egui::pos2(screen_pts[i].0, screen_pts[i].1),
                            egui::pos2(screen_pts[i + 1].0, screen_pts[i + 1].1),
                            egui::Stroke::new(effective_width, sc),
                            d_style,
                        );
                    }
                    if i < labels.len() {
                        painter.text(
                            egui::pos2(screen_pts[i].0, screen_pts[i].1 - 10.0),
                            egui::Align2::CENTER_BOTTOM,
                            labels[i],
                            egui::FontId::monospace(11.0),
                            sc,
                        );
                    }
                }
            }
            Drawing::AbcCorrection { points, color } => {
                let mut screen_pts: Vec<(f32, f32)> = Vec::new();
                for &(bi, pr) in points.iter() {
                    if bi >= start_idx && bi < end_idx {
                        let x = chart_rect.left() + ((bi - start_idx) as f32 + 0.5) * bar_w;
                        let y = price_to_y(pr);
                        screen_pts.push((x, y));
                    }
                }
                let labels = ["A", "B", "C"];
                let sc = sel_tint(*color);
                for i in 0..screen_pts.len() {
                    if i + 1 < screen_pts.len() {
                        draw_line(
                            &painter,
                            egui::pos2(screen_pts[i].0, screen_pts[i].1),
                            egui::pos2(screen_pts[i + 1].0, screen_pts[i + 1].1),
                            egui::Stroke::new(effective_width, sc),
                            d_style,
                        );
                    }
                    if i < labels.len() {
                        painter.text(
                            egui::pos2(screen_pts[i].0, screen_pts[i].1 - 10.0),
                            egui::Align2::CENTER_BOTTOM,
                            labels[i],
                            egui::FontId::monospace(11.0),
                            sc,
                        );
                    }
                }
            }
            Drawing::DateRange { p1, p2 } => {
                let x1o = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(chart_rect.left() + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2o = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(chart_rect.left() + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1o, x2o) {
                    let mid_y = (price_to_y(p1.1) + price_to_y(p2.1)) / 2.0;
                    let col = egui::Color32::from_rgb(100, 200, 255);
                    // Vertical markers
                    painter.line_segment(
                        [egui::pos2(x1, mid_y - 12.0), egui::pos2(x1, mid_y + 12.0)],
                        egui::Stroke::new(1.0, col),
                    );
                    painter.line_segment(
                        [egui::pos2(x2, mid_y - 12.0), egui::pos2(x2, mid_y + 12.0)],
                        egui::Stroke::new(1.0, col),
                    );
                    // Connecting line
                    painter.line_segment(
                        [egui::pos2(x1, mid_y), egui::pos2(x2, mid_y)],
                        egui::Stroke::new(1.0, col),
                    );
                    let bar_count = if p2.0 > p1.0 {
                        p2.0 - p1.0
                    } else {
                        p1.0 - p2.0
                    };
                    let label = format!("{} bars", bar_count);
                    painter.text(
                        egui::pos2((x1 + x2) / 2.0, mid_y - 6.0),
                        egui::Align2::CENTER_BOTTOM,
                        &label,
                        egui::FontId::monospace(10.0),
                        col,
                    );
                }
            }
            Drawing::DatePriceRange { p1, p2 } => {
                let x1o = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(chart_rect.left() + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2o = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(chart_rect.left() + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1o, x2o) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    let fill = egui::Color32::from_rgba_premultiplied(100, 200, 150, 15);
                    painter.rect_filled(
                        egui::Rect::from_two_pos(egui::pos2(x1, y1), egui::pos2(x2, y2)),
                        0.0,
                        fill,
                    );
                    painter.rect_stroke(
                        egui::Rect::from_two_pos(egui::pos2(x1, y1), egui::pos2(x2, y2)),
                        0.0,
                        egui::Stroke::new(0.8, egui::Color32::from_rgb(100, 200, 150)),
                        egui::StrokeKind::Outside,
                    );
                    let bars = if p2.0 > p1.0 {
                        p2.0 - p1.0
                    } else {
                        p1.0 - p2.0
                    };
                    let dist = p2.1 - p1.1;
                    let pct = if p1.1.abs() > f64::EPSILON {
                        dist / p1.1 * 100.0
                    } else {
                        0.0
                    };
                    let label = format!("{} bars | {:.2} ({:+.2}%)", bars, dist, pct);
                    let col = egui::Color32::from_rgb(100, 200, 150);
                    painter.text(
                        egui::pos2((x1 + x2) / 2.0, y1.min(y2) - 4.0),
                        egui::Align2::CENTER_BOTTOM,
                        &label,
                        egui::FontId::monospace(10.0),
                        col,
                    );
                }
            }
            Drawing::HeadShoulders { points, color } => {
                // 5 points: 0=LS bottom, 1=LS top, 2=Head top, 3=RS top, 4=RS bottom
                // Connect all in order, draw neckline between 0 and 4
                let mut screen_pts: Vec<(f32, f32)> = Vec::new();
                for &(bi, pr) in points.iter() {
                    if bi >= start_idx && bi < end_idx {
                        let x = chart_rect.left() + ((bi - start_idx) as f32 + 0.5) * bar_w;
                        let y = price_to_y(pr);
                        screen_pts.push((x, y));
                    }
                }
                let labels = ["LS", "L", "H", "R", "RS"];
                let sc = sel_tint(*color);
                for i in 0..screen_pts.len() {
                    if i + 1 < screen_pts.len() {
                        draw_line(
                            &painter,
                            egui::pos2(screen_pts[i].0, screen_pts[i].1),
                            egui::pos2(screen_pts[i + 1].0, screen_pts[i + 1].1),
                            egui::Stroke::new(effective_width, sc),
                            d_style,
                        );
                    }
                    if i < labels.len() {
                        painter.text(
                            egui::pos2(screen_pts[i].0, screen_pts[i].1 - 10.0),
                            egui::Align2::CENTER_BOTTOM,
                            labels[i],
                            egui::FontId::monospace(9.0),
                            sc,
                        );
                    }
                }
                // Neckline: dashed line between point 0 and point 4
                if screen_pts.len() >= 5 {
                    let nk_col =
                        egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), 150);
                    draw_line(
                        &painter,
                        egui::pos2(screen_pts[0].0, screen_pts[0].1),
                        egui::pos2(screen_pts[4].0, screen_pts[4].1),
                        egui::Stroke::new(effective_width, nk_col),
                        LineStyle::Dashed,
                    );
                    painter.text(
                        egui::pos2(
                            (screen_pts[0].0 + screen_pts[4].0) / 2.0,
                            (screen_pts[0].1 + screen_pts[4].1) / 2.0 + 12.0,
                        ),
                        egui::Align2::CENTER_TOP,
                        "Neckline",
                        egui::FontId::monospace(9.0),
                        nk_col,
                    );
                }
            }
            Drawing::XabcdPattern { points, color } => {
                let mut screen_pts: Vec<(f32, f32)> = Vec::new();
                for &(bi, pr) in points.iter() {
                    if bi >= start_idx && bi < end_idx {
                        let x = chart_rect.left() + ((bi - start_idx) as f32 + 0.5) * bar_w;
                        let y = price_to_y(pr);
                        screen_pts.push((x, y));
                    }
                }
                let labels = ["X", "A", "B", "C", "D"];
                let sc = sel_tint(*color);
                for i in 0..screen_pts.len() {
                    if i + 1 < screen_pts.len() {
                        draw_line(
                            &painter,
                            egui::pos2(screen_pts[i].0, screen_pts[i].1),
                            egui::pos2(screen_pts[i + 1].0, screen_pts[i + 1].1),
                            egui::Stroke::new(effective_width, sc),
                            d_style,
                        );
                    }
                    if i < labels.len() {
                        painter.text(
                            egui::pos2(screen_pts[i].0, screen_pts[i].1 - 10.0),
                            egui::Align2::CENTER_BOTTOM,
                            labels[i],
                            egui::FontId::monospace(11.0),
                            sc,
                        );
                    }
                }
                // XA→BD dashed line (harmonic diagonal)
                if screen_pts.len() >= 5 {
                    let diag = egui::Color32::from_rgba_premultiplied(sc.r(), sc.g(), sc.b(), 80);
                    draw_line(
                        &painter,
                        egui::pos2(screen_pts[0].0, screen_pts[0].1),
                        egui::pos2(screen_pts[3].0, screen_pts[3].1),
                        egui::Stroke::new(0.6, diag),
                        LineStyle::Dashed,
                    );
                    draw_line(
                        &painter,
                        egui::pos2(screen_pts[1].0, screen_pts[1].1),
                        egui::pos2(screen_pts[4].0, screen_pts[4].1),
                        egui::Stroke::new(0.6, diag),
                        LineStyle::Dashed,
                    );
                }
            }
            Drawing::Brush { points, color } => {
                for &(bi, pr) in points.iter() {
                    if bi >= start_idx && bi < end_idx {
                        let x = chart_rect.left() + ((bi - start_idx) as f32 + 0.5) * bar_w;
                        let y = price_to_y(pr);
                        painter.circle_filled(egui::pos2(x, y), 2.0, *color);
                    }
                }
            }
            Drawing::SchiffPitchfork {
                pivot,
                p2,
                p3,
                color,
            }
            | Drawing::ModSchiffPitchfork {
                pivot,
                p2,
                p3,
                color,
            } => {
                // Schiff: shifted pivot = midpoint(pivot, p2) on bar-axis, midpoint(pivot, p2) on price
                // Modified Schiff: shifted pivot = (mid(pivot.bar, p2.bar), mid(pivot.price, p3.price))
                let is_mod = matches!(drawing, Drawing::ModSchiffPitchfork { .. });
                let shifted_bar = if is_mod {
                    ((pivot.0 as f64 + p2.0 as f64) / 2.0) as usize
                } else {
                    ((pivot.0 as f64 + p2.0 as f64) / 2.0) as usize
                };
                let shifted_price = if is_mod {
                    (pivot.1 + p2.1) / 2.0 * 0.5 + (pivot.1 + p3.1) / 2.0 * 0.5
                } else {
                    (pivot.1 + p2.1) / 2.0
                };
                let mid_bar = ((p2.0 as f64 + p3.0 as f64) / 2.0) as usize;
                let mid_price = (p2.1 + p3.1) / 2.0;
                let bar_to_x = |b: usize| -> Option<f32> {
                    if b >= start_idx && b < end_idx {
                        Some(chart_rect.left() + ((b - start_idx) as f32 + 0.5) * bar_w)
                    } else {
                        None
                    }
                };
                let sc = sel_tint(*color);
                // Median line: shifted pivot → midpoint of p2,p3
                if let (Some(sx), Some(mx)) = (bar_to_x(shifted_bar), bar_to_x(mid_bar)) {
                    draw_line(
                        &painter,
                        egui::pos2(sx, price_to_y(shifted_price)),
                        egui::pos2(mx, price_to_y(mid_price)),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                }
                // Parallel lines through p2 and p3
                if let (Some(sx), Some(mx), Some(x2), Some(x3)) = (
                    bar_to_x(shifted_bar),
                    bar_to_x(mid_bar),
                    bar_to_x(p2.0),
                    bar_to_x(p3.0),
                ) {
                    let dx = mx - sx;
                    let dy = price_to_y(mid_price) - price_to_y(shifted_price);
                    let y2 = price_to_y(p2.1);
                    let y3 = price_to_y(p3.1);
                    draw_line(
                        &painter,
                        egui::pos2(x2, y2),
                        egui::pos2(x2 + dx, y2 + dy),
                        egui::Stroke::new(effective_width * 0.7, sc),
                        d_style,
                    );
                    draw_line(
                        &painter,
                        egui::pos2(x3, y3),
                        egui::pos2(x3 + dx, y3 + dy),
                        egui::Stroke::new(effective_width * 0.7, sc),
                        d_style,
                    );
                }
            }
            Drawing::CyclicLines {
                bar_start,
                bar_end,
                color,
            } => {
                let interval = if *bar_end > *bar_start {
                    bar_end - bar_start
                } else {
                    1
                };
                let mut b = *bar_start;
                while b < start_idx + (end_idx - start_idx) + interval * 20 {
                    if b >= start_idx && b < end_idx {
                        let x = chart_rect.left() + ((b - start_idx) as f32 + 0.5) * bar_w;
                        draw_line(
                            &painter,
                            egui::pos2(x, chart_rect.top()),
                            egui::pos2(x, chart_rect.bottom()),
                            egui::Stroke::new(effective_width * 0.5, sel_tint(*color)),
                            d_style,
                        );
                    }
                    b += interval;
                }
            }
            Drawing::SineWave { p1, p2, color } => {
                let bar_to_x = |b: usize| -> f32 {
                    chart_rect.left() + ((b as f32 - start_idx as f32) + 0.5) * bar_w
                };
                let period = ((p2.0 as f64 - p1.0 as f64).abs()).max(1.0);
                let amplitude = (p2.1 - p1.1).abs() / 2.0;
                let mid_price = (p1.1 + p2.1) / 2.0;
                let start_bar = p1.0;
                let mut prev: Option<egui::Pos2> = None;
                for b in start_idx..end_idx {
                    let phase = (b as f64 - start_bar as f64) / period * 2.0 * std::f64::consts::PI;
                    let price_val = mid_price + amplitude * phase.sin();
                    let x = bar_to_x(b);
                    let y = price_to_y(price_val);
                    let pt = egui::pos2(x, y);
                    if let Some(p) = prev {
                        painter.line_segment(
                            [p, pt],
                            egui::Stroke::new(effective_width, sel_tint(*color)),
                        );
                    }
                    prev = Some(pt);
                }
            }
            Drawing::Emoji {
                bar_idx,
                price,
                emoji,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = chart_rect.left() + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    painter.text(
                        egui::pos2(x, y),
                        egui::Align2::CENTER_CENTER,
                        emoji,
                        egui::FontId::proportional(16.0),
                        egui::Color32::WHITE,
                    );
                }
            }
            Drawing::Flag {
                bar_idx,
                price,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = chart_rect.left() + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    let sc = sel_tint(*color);
                    // Pole
                    draw_line(
                        &painter,
                        egui::pos2(x, y),
                        egui::pos2(x, y - 20.0),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    // Flag triangle
                    let tri = vec![
                        egui::pos2(x, y - 20.0),
                        egui::pos2(x + 12.0, y - 15.0),
                        egui::pos2(x, y - 10.0),
                    ];
                    painter.add(egui::Shape::convex_polygon(tri, sc, egui::Stroke::NONE));
                }
            }
            Drawing::Balloon {
                anchor,
                label_pos,
                text,
                color,
            } => {
                let bar_to_x = |b: usize| -> Option<f32> {
                    if b >= start_idx && b < end_idx {
                        Some(chart_rect.left() + ((b - start_idx) as f32 + 0.5) * bar_w)
                    } else {
                        None
                    }
                };
                if let (Some(ax), Some(lx)) = (bar_to_x(anchor.0), bar_to_x(label_pos.0)) {
                    let ay = price_to_y(anchor.1);
                    let ly = price_to_y(label_pos.1);
                    // Line from anchor to label
                    draw_line(
                        &painter,
                        egui::pos2(ax, ay),
                        egui::pos2(lx, ly),
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                        d_style,
                    );
                    // Bubble background
                    let text_rect =
                        egui::Rect::from_center_size(egui::pos2(lx, ly), egui::vec2(80.0, 24.0));
                    painter.rect_filled(
                        text_rect,
                        6.0,
                        egui::Color32::from_rgba_premultiplied(40, 40, 60, 200),
                    );
                    let sc = sel_tint(*color);
                    painter.rect_stroke(
                        text_rect,
                        6.0,
                        egui::Stroke::new(effective_width, sc),
                        egui::StrokeKind::Outside,
                    );
                    painter.text(
                        egui::pos2(lx, ly),
                        egui::Align2::CENTER_CENTER,
                        text,
                        egui::FontId::monospace(10.0),
                        sc,
                    );
                }
            }
            Drawing::SessionBreak { bar_idx, color } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = chart_rect.left() + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let sc = sel_tint(*color);
                    // Dashed vertical line — delegate to draw_line for style support
                    draw_line(
                        &painter,
                        egui::pos2(x, chart_rect.top()),
                        egui::pos2(x, chart_rect.bottom()),
                        egui::Stroke::new(effective_width, sc),
                        LineStyle::Dashed,
                    );
                    painter.text(
                        egui::pos2(x + 4.0, chart_rect.top() + 2.0),
                        egui::Align2::LEFT_TOP,
                        "Session",
                        egui::FontId::monospace(8.0),
                        sc,
                    );
                }
            }
            Drawing::MagnetLevel { price, color } => {
                let y = price_to_y(*price);
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    // Check if last bar's close is within 0.5% of this level
                    let glow = if end_idx > start_idx {
                        let last_close =
                            chart.bars.get(end_idx - 1).map(|b| b.close).unwrap_or(0.0);
                        (last_close - price).abs() / price.abs().max(0.0001) < 0.005
                    } else {
                        false
                    };
                    let base_col = if glow {
                        egui::Color32::from_rgb(255, 255, 100)
                    } else {
                        sel_tint(*color)
                    };
                    let stroke_w = if glow {
                        effective_width.max(2.5)
                    } else {
                        effective_width
                    };
                    let draw_color = base_col;
                    draw_line(
                        &painter,
                        egui::pos2(chart_rect.left(), y),
                        egui::pos2(chart_rect.right(), y),
                        egui::Stroke::new(stroke_w, draw_color),
                        d_style,
                    );
                    if glow {
                        // Glow effect: semi-transparent wider line
                        let glow_col = egui::Color32::from_rgba_premultiplied(255, 255, 100, 40);
                        painter.line_segment(
                            [
                                egui::pos2(chart_rect.left(), y),
                                egui::pos2(chart_rect.right(), y),
                            ],
                            egui::Stroke::new(6.0, glow_col),
                        );
                    }
                    painter.text(
                        egui::pos2(chart_rect.right() - 80.0, y - 10.0),
                        egui::Align2::LEFT_TOP,
                        &format!("M {}", &format_price(*price)),
                        egui::FontId::monospace(9.0),
                        base_col,
                    );
                }
            }
            Drawing::RiskRewardBox {
                entry,
                stop,
                target,
            } => {
                let bar_to_x = |b: usize| -> f32 {
                    chart_rect.left() + ((b as f32 - start_idx as f32) + 0.5) * bar_w
                };
                let entry_x = bar_to_x(entry.0);
                let entry_y = price_to_y(entry.1);
                let stop_y = price_to_y(*stop);
                let target_y = price_to_y(*target);
                let box_width = bar_w * 20.0;
                let right_x = entry_x + box_width;
                // Risk zone (entry to stop) — red
                let risk_rect = egui::Rect::from_two_pos(
                    egui::pos2(entry_x, entry_y),
                    egui::pos2(right_x, stop_y),
                );
                painter.rect_filled(
                    risk_rect,
                    0.0,
                    egui::Color32::from_rgba_premultiplied(220, 40, 40, 30),
                );
                painter.rect_stroke(
                    risk_rect,
                    0.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(220, 40, 40)),
                    egui::StrokeKind::Outside,
                );
                // Reward zone (entry to target) — green
                let reward_rect = egui::Rect::from_two_pos(
                    egui::pos2(entry_x, entry_y),
                    egui::pos2(right_x, target_y),
                );
                painter.rect_filled(
                    reward_rect,
                    0.0,
                    egui::Color32::from_rgba_premultiplied(0, 200, 80, 30),
                );
                painter.rect_stroke(
                    reward_rect,
                    0.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 200, 80)),
                    egui::StrokeKind::Outside,
                );
                // Entry line
                painter.line_segment(
                    [egui::pos2(entry_x, entry_y), egui::pos2(right_x, entry_y)],
                    egui::Stroke::new(1.5, egui::Color32::WHITE),
                );
                // R:R ratio
                let risk = (entry.1 - stop).abs();
                let reward = (target - entry.1).abs();
                let rr = if risk > 0.0 { reward / risk } else { 0.0 };
                painter.text(
                    egui::pos2(right_x + 4.0, entry_y),
                    egui::Align2::LEFT_CENTER,
                    &format!("R:R {:.1}", rr),
                    egui::FontId::monospace(10.0),
                    egui::Color32::WHITE,
                );
            }
            Drawing::FibCircle {
                center,
                radius_pt,
                color,
            } => {
                let cx = chart_rect.left() + ((center.0 as f32 - start_idx as f32) + 0.5) * bar_w;
                let cy = price_to_y(center.1);
                let rx =
                    chart_rect.left() + ((radius_pt.0 as f32 - start_idx as f32) + 0.5) * bar_w;
                let ry = price_to_y(radius_pt.1);
                let base_r = ((rx - cx).powi(2) + (ry - cy).powi(2)).sqrt();
                let fib_ratios = [0.236, 0.382, 0.5, 0.618, 0.786, 1.0];
                for ratio in &fib_ratios {
                    let r = base_r * (*ratio as f32);
                    let segments = 64;
                    let mut pts = Vec::with_capacity(segments + 1);
                    for i in 0..=segments {
                        let angle = (i as f32 / segments as f32) * 2.0 * std::f32::consts::PI;
                        pts.push(egui::pos2(cx + r * angle.cos(), cy + r * angle.sin()));
                    }
                    let sc = sel_tint(*color);
                    for w in pts.windows(2) {
                        painter.line_segment([w[0], w[1]], egui::Stroke::new(effective_width, sc));
                    }
                    painter.text(
                        egui::pos2(cx + r + 2.0, cy),
                        egui::Align2::LEFT_CENTER,
                        &format!("{:.3}", ratio),
                        egui::FontId::monospace(8.0),
                        *color,
                    );
                }
            }
            Drawing::ArcDraw { p1, p2, p3, color } => {
                let bar_to_x = |b: usize| -> f32 {
                    chart_rect.left() + ((b as f32 - start_idx as f32) + 0.5) * bar_w
                };
                let x1 = bar_to_x(p1.0);
                let y1 = price_to_y(p1.1);
                let x2 = bar_to_x(p2.0);
                let y2 = price_to_y(p2.1);
                let x3 = bar_to_x(p3.0);
                let y3 = price_to_y(p3.1);
                // Quadratic bezier through 3 points: control point derived from midpoint
                let ctrl_x = 2.0 * x2 - 0.5 * x1 - 0.5 * x3;
                let ctrl_y = 2.0 * y2 - 0.5 * y1 - 0.5 * y3;
                let segments = 48;
                let mut prev = egui::pos2(x1, y1);
                for i in 1..=segments {
                    let t = i as f32 / segments as f32;
                    let it = 1.0 - t;
                    let px = it * it * x1 + 2.0 * it * t * ctrl_x + t * t * x3;
                    let py = it * it * y1 + 2.0 * it * t * ctrl_y + t * t * y3;
                    let pt = egui::pos2(px, py);
                    painter.line_segment(
                        [prev, pt],
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                    );
                    prev = pt;
                }
            }
            Drawing::CurveDraw {
                p1,
                ctrl1,
                ctrl2,
                p2,
                color,
            } => {
                let bar_to_x = |b: usize| -> f32 {
                    chart_rect.left() + ((b as f32 - start_idx as f32) + 0.5) * bar_w
                };
                let x0 = bar_to_x(p1.0);
                let y0 = price_to_y(p1.1);
                let cx1 = bar_to_x(ctrl1.0);
                let cy1 = price_to_y(ctrl1.1);
                let cx2 = bar_to_x(ctrl2.0);
                let cy2 = price_to_y(ctrl2.1);
                let x3 = bar_to_x(p2.0);
                let y3 = price_to_y(p2.1);
                let segments = 64;
                let mut prev = egui::pos2(x0, y0);
                for i in 1..=segments {
                    let t = i as f32 / segments as f32;
                    let it = 1.0 - t;
                    let px = it.powi(3) * x0
                        + 3.0 * it.powi(2) * t * cx1
                        + 3.0 * it * t.powi(2) * cx2
                        + t.powi(3) * x3;
                    let py = it.powi(3) * y0
                        + 3.0 * it.powi(2) * t * cy1
                        + 3.0 * it * t.powi(2) * cy2
                        + t.powi(3) * y3;
                    let pt = egui::pos2(px, py);
                    painter.line_segment(
                        [prev, pt],
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                    );
                    prev = pt;
                }
                // Draw control point markers
                painter.circle_stroke(egui::pos2(cx1, cy1), 3.0, egui::Stroke::new(1.0, *color));
                painter.circle_stroke(egui::pos2(cx2, cy2), 3.0, egui::Stroke::new(1.0, *color));
            }
            Drawing::PathDraw { points, color } => {
                if points.len() >= 2 {
                    let bar_to_x = |b: usize| -> f32 {
                        chart_rect.left() + ((b as f32 - start_idx as f32) + 0.5) * bar_w
                    };
                    let screen_pts: Vec<egui::Pos2> = points
                        .iter()
                        .map(|(b, p)| egui::pos2(bar_to_x(*b), price_to_y(*p)))
                        .collect();
                    // Catmull-Rom interpolation between each segment
                    for seg in 0..screen_pts.len() - 1 {
                        let p0 = if seg > 0 {
                            screen_pts[seg - 1]
                        } else {
                            screen_pts[seg]
                        };
                        let pa = screen_pts[seg];
                        let pb = screen_pts[seg + 1];
                        let p3 = if seg + 2 < screen_pts.len() {
                            screen_pts[seg + 2]
                        } else {
                            screen_pts[seg + 1]
                        };
                        let steps = 24;
                        let mut prev = pa;
                        for i in 1..=steps {
                            let t = i as f32 / steps as f32;
                            let t2 = t * t;
                            let t3 = t2 * t;
                            let px = 0.5
                                * ((2.0 * pa.x)
                                    + (-p0.x + pb.x) * t
                                    + (2.0 * p0.x - 5.0 * pa.x + 4.0 * pb.x - p3.x) * t2
                                    + (-p0.x + 3.0 * pa.x - 3.0 * pb.x + p3.x) * t3);
                            let py = 0.5
                                * ((2.0 * pa.y)
                                    + (-p0.y + pb.y) * t
                                    + (2.0 * p0.y - 5.0 * pa.y + 4.0 * pb.y - p3.y) * t2
                                    + (-p0.y + 3.0 * pa.y - 3.0 * pb.y + p3.y) * t3);
                            let pt = egui::pos2(px, py);
                            painter.line_segment(
                                [prev, pt],
                                egui::Stroke::new(effective_width, sel_tint(*color)),
                            );
                            prev = pt;
                        }
                    }
                }
            }
            Drawing::Forecast { p1, p2, color } => {
                let bar_to_x = |b: usize| -> f32 {
                    chart_rect.left() + ((b as f32 - start_idx as f32) + 0.5) * bar_w
                };
                let x1 = bar_to_x(p1.0);
                let y1 = price_to_y(p1.1);
                let x2 = bar_to_x(p2.0);
                let y2 = price_to_y(p2.1);
                let sc = sel_tint(*color);
                // Solid trend line
                draw_line(
                    &painter,
                    egui::pos2(x1, y1),
                    egui::pos2(x2, y2),
                    egui::Stroke::new(effective_width, sc),
                    d_style,
                );
                // Dashed projection forward (same slope, same length)
                let dx = x2 - x1;
                let dy = y2 - y1;
                let proj_x = x2 + dx;
                let proj_y = y2 + dy;
                draw_line(
                    &painter,
                    egui::pos2(x2, y2),
                    egui::pos2(proj_x, proj_y),
                    egui::Stroke::new(effective_width * 0.7, sc),
                    LineStyle::Dashed,
                );
                painter.text(
                    egui::pos2(proj_x + 4.0, proj_y),
                    egui::Align2::LEFT_CENTER,
                    "Forecast",
                    egui::FontId::monospace(9.0),
                    sc,
                );
            }
            Drawing::GhostFeed { p1, p2, color } => {
                let bar_to_x = |b: usize| -> f32 {
                    chart_rect.left() + ((b as f32 - start_idx as f32) + 0.5) * bar_w
                };
                // Mirror the bars from p1..p2 forward starting at p2
                let src_start = p1.0.min(p2.0);
                let src_end = p1.0.max(p2.0);
                let mirror_len = src_end - src_start;
                if mirror_len > 0 {
                    for i in 0..mirror_len {
                        let src_idx = src_start + i;
                        let dst_idx = src_end + i;
                        if src_idx < chart.bars.len()
                            && dst_idx < chart.bars.len() + CHART_RIGHT_MARGIN
                        {
                            let src_bar = chart.bars.get(src_idx);
                            if let Some(sb) = src_bar {
                                let x = bar_to_x(dst_idx);
                                let oy = price_to_y(sb.open);
                                let cy = price_to_y(sb.close);
                                let hy = price_to_y(sb.high);
                                let ly = price_to_y(sb.low);
                                let ghost_col = egui::Color32::from_rgba_premultiplied(
                                    color.r(),
                                    color.g(),
                                    color.b(),
                                    80,
                                );
                                painter.line_segment(
                                    [egui::pos2(x, hy), egui::pos2(x, ly)],
                                    egui::Stroke::new(0.5, ghost_col),
                                );
                                let top = oy.min(cy);
                                let bot = oy.max(cy);
                                let w = (bar_w * 0.6).max(1.0);
                                painter.rect_filled(
                                    egui::Rect::from_min_max(
                                        egui::pos2(x - w / 2.0, top),
                                        egui::pos2(x + w / 2.0, bot),
                                    ),
                                    0.0,
                                    ghost_col,
                                );
                            }
                        }
                    }
                }
            }
            Drawing::Signpost {
                bar_idx,
                price,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = chart_rect.left() + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    let sc = sel_tint(*color);
                    // Pole
                    draw_line(
                        &painter,
                        egui::pos2(x, y + 15.0),
                        egui::pos2(x, y - 15.0),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    // Arrow head (pointing right)
                    let arrow = vec![
                        egui::pos2(x, y - 12.0),
                        egui::pos2(x + 14.0, y - 6.0),
                        egui::pos2(x, y),
                    ];
                    painter.add(egui::Shape::convex_polygon(arrow, sc, egui::Stroke::NONE));
                    // Base
                    draw_line(
                        &painter,
                        egui::pos2(x - 5.0, y + 15.0),
                        egui::pos2(x + 5.0, y + 15.0),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                }
            }
            Drawing::Ruler { p1, p2, color } => {
                let bar_to_x = |b: usize| -> f32 {
                    chart_rect.left() + ((b as f32 - start_idx as f32) + 0.5) * bar_w
                };
                let x1 = bar_to_x(p1.0);
                let y1 = price_to_y(p1.1);
                let x2 = bar_to_x(p2.0);
                let y2 = price_to_y(p2.1);
                let sc = sel_tint(*color);
                draw_line(
                    &painter,
                    egui::pos2(x1, y1),
                    egui::pos2(x2, y2),
                    egui::Stroke::new(effective_width, sc),
                    d_style,
                );
                // Endpoints
                painter.circle_filled(egui::pos2(x1, y1), 3.0, sc);
                painter.circle_filled(egui::pos2(x2, y2), 3.0, sc);
                // Measurement label
                let price_diff = p2.1 - p1.1;
                let bars_diff = if p2.0 > p1.0 {
                    p2.0 - p1.0
                } else {
                    p1.0 - p2.0
                };
                let pct = if p1.1.abs() > 0.0001 {
                    (price_diff / p1.1) * 100.0
                } else {
                    0.0
                };
                let mid_x = (x1 + x2) / 2.0;
                let mid_y = (y1 + y2) / 2.0;
                let label = format!("{:.4} ({} bars, {:.2}%)", price_diff, bars_diff, pct);
                let bg_rect = egui::Rect::from_center_size(
                    egui::pos2(mid_x, mid_y - 12.0),
                    egui::vec2(label.len() as f32 * 6.5 + 8.0, 16.0),
                );
                painter.rect_filled(
                    bg_rect,
                    3.0,
                    egui::Color32::from_rgba_premultiplied(0, 0, 0, 200),
                );
                painter.text(
                    egui::pos2(mid_x, mid_y - 12.0),
                    egui::Align2::CENTER_CENTER,
                    &label,
                    egui::FontId::monospace(10.0),
                    sc,
                );
            }
            Drawing::TimeCycle {
                bar_start,
                bar_end,
                color,
            } => {
                let interval = if *bar_end > *bar_start {
                    bar_end - bar_start
                } else {
                    1
                };
                let mut b = *bar_start;
                while b < chart.bars.len() + CHART_RIGHT_MARGIN * 10 {
                    if b >= start_idx && b < end_idx {
                        let x = chart_rect.left() + ((b as f32 - start_idx as f32) + 0.5) * bar_w;
                        let sc = sel_tint(*color);
                        draw_line(
                            &painter,
                            egui::pos2(x, chart_rect.top()),
                            egui::pos2(x, chart_rect.bottom()),
                            egui::Stroke::new(effective_width, sc),
                            d_style,
                        );
                    }
                    // Draw semi-circle arc between this line and the next
                    let next_b = b + interval;
                    if b >= start_idx && next_b < end_idx {
                        let x1 = chart_rect.left() + ((b as f32 - start_idx as f32) + 0.5) * bar_w;
                        let x2 =
                            chart_rect.left() + ((next_b as f32 - start_idx as f32) + 0.5) * bar_w;
                        let cx = (x1 + x2) / 2.0;
                        let r = (x2 - x1) / 2.0;
                        let arc_y = chart_rect.bottom() - 2.0;
                        let segs = 24;
                        let sc = sel_tint(*color);
                        let mut prev_pt = egui::pos2(x1, arc_y);
                        for i in 1..=segs {
                            let angle = std::f32::consts::PI * (i as f32 / segs as f32);
                            let px = cx - r * angle.cos();
                            let py = arc_y - r * angle.sin() * 0.3; // squashed arc
                            let pt = egui::pos2(px, py);
                            painter.line_segment(
                                [prev_pt, pt],
                                egui::Stroke::new(effective_width * 0.55, sc),
                            );
                            prev_pt = pt;
                        }
                    }
                    b += interval;
                    if b > end_idx + interval * 2 {
                        break;
                    }
                }
            }
            Drawing::SpeedResistanceFan { p1, p2, p3, color } => {
                let bar_to_x = |b: usize| -> f32 {
                    chart_rect.left() + ((b as f32 - start_idx as f32) + 0.5) * bar_w
                };
                let x1 = bar_to_x(p1.0);
                let y1 = price_to_y(p1.1);
                let x2 = bar_to_x(p2.0);
                let y2 = price_to_y(p2.1);
                let x3 = bar_to_x(p3.0);
                let _ = x3;
                // Speed lines: 1/3 and 2/3 of the move
                let dy = y2 - y1;
                let dx = x2 - x1;
                let extend = chart_rect.right() - x1;
                let sc = sel_tint(*color);
                for frac in [1.0_f32 / 3.0, 2.0 / 3.0] {
                    let target_y = y1 + dy * frac;
                    let slope = if dx.abs() > 0.1 {
                        (target_y - y1) / dx
                    } else {
                        0.0
                    };
                    let end_x = x1 + extend;
                    let end_y = y1 + slope * extend;
                    draw_line(
                        &painter,
                        egui::pos2(x1, y1),
                        egui::pos2(end_x, end_y),
                        egui::Stroke::new(effective_width * 0.7, sc),
                        d_style,
                    );
                    painter.text(
                        egui::pos2(end_x - 30.0, end_y),
                        egui::Align2::LEFT_CENTER,
                        &format!("{:.0}%", frac * 100.0),
                        egui::FontId::monospace(8.0),
                        sc,
                    );
                }
                // Base line
                draw_line(
                    &painter,
                    egui::pos2(x1, y1),
                    egui::pos2(x2, y2),
                    egui::Stroke::new(effective_width, sc),
                    d_style,
                );
            }
            Drawing::SpeedResistanceArc { p1, p2, p3, color } => {
                let bar_to_x = |b: usize| -> f32 {
                    chart_rect.left() + ((b as f32 - start_idx as f32) + 0.5) * bar_w
                };
                let x1 = bar_to_x(p1.0);
                let y1 = price_to_y(p1.1);
                let x2 = bar_to_x(p2.0);
                let y2 = price_to_y(p2.1);
                let _ = bar_to_x(p3.0);
                let base_r = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
                let sc = sel_tint(*color);
                // Base line
                draw_line(
                    &painter,
                    egui::pos2(x1, y1),
                    egui::pos2(x2, y2),
                    egui::Stroke::new(effective_width, sc),
                    d_style,
                );
                // Arcs at 1/3 and 2/3
                for frac in [1.0_f32 / 3.0, 2.0 / 3.0] {
                    let r = base_r * frac;
                    let segs = 32;
                    let mut prev: Option<egui::Pos2> = None;
                    for i in 0..=segs {
                        let angle = std::f32::consts::PI * (i as f32 / segs as f32);
                        let px = x1 + r * angle.cos();
                        let py = y1 - r * angle.sin();
                        let pt = egui::pos2(px, py);
                        if let Some(p) = prev {
                            painter.line_segment(
                                [p, pt],
                                egui::Stroke::new(effective_width * 0.7, sc),
                            );
                        }
                        prev = Some(pt);
                    }
                }
            }
            Drawing::FibSpiral {
                center,
                radius_pt,
                color,
            } => {
                let cx = chart_rect.left() + ((center.0 as f32 - start_idx as f32) + 0.5) * bar_w;
                let cy = price_to_y(center.1);
                let rx =
                    chart_rect.left() + ((radius_pt.0 as f32 - start_idx as f32) + 0.5) * bar_w;
                let ry = price_to_y(radius_pt.1);
                let base_r = ((rx - cx).powi(2) + (ry - cy).powi(2)).sqrt().max(1.0);
                // Golden spiral: r = a * e^(b*theta) where b = ln(phi)/(PI/2)
                let phi: f32 = 1.618033988749895;
                let b_param = phi.ln() / (std::f32::consts::PI / 2.0);
                let a_param = base_r / (b_param * 6.0 * std::f32::consts::PI).exp();
                let total_angle = 6.0 * std::f32::consts::PI; // 3 full turns
                let steps = 200;
                let mut prev: Option<egui::Pos2> = None;
                for i in 0..=steps {
                    let theta = total_angle * (i as f32 / steps as f32);
                    let r = a_param * (b_param * theta).exp();
                    let px = cx + r * theta.cos();
                    let py = cy - r * theta.sin();
                    let pt = egui::pos2(px, py);
                    if let Some(p) = prev {
                        painter.line_segment(
                            [p, pt],
                            egui::Stroke::new(effective_width, sel_tint(*color)),
                        );
                    }
                    prev = Some(pt);
                }
            }
            Drawing::RotatedRectangle { p1, p2, p3, color } => {
                let bar_to_x = |b: usize| -> f32 {
                    chart_rect.left() + ((b as f32 - start_idx as f32) + 0.5) * bar_w
                };
                let x1 = bar_to_x(p1.0);
                let y1 = price_to_y(p1.1);
                let x2 = bar_to_x(p2.0);
                let y2 = price_to_y(p2.1);
                let x3 = bar_to_x(p3.0);
                let y3 = price_to_y(p3.1);
                // Baseline direction
                let bx = x2 - x1;
                let by = y2 - y1;
                let blen = (bx * bx + by * by).sqrt().max(0.001);
                let nx = -by / blen;
                let ny = bx / blen;
                // Project p3 onto the normal to get height
                let h = (x3 - x1) * nx + (y3 - y1) * ny;
                // Four corners
                let c1 = egui::pos2(x1, y1);
                let c2 = egui::pos2(x2, y2);
                let c3 = egui::pos2(x2 + nx * h, y2 + ny * h);
                let c4 = egui::pos2(x1 + nx * h, y1 + ny * h);
                let fill =
                    egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 25);
                painter.add(egui::Shape::convex_polygon(
                    vec![c1, c2, c3, c4],
                    fill,
                    egui::Stroke::new(effective_width, sel_tint(*color)),
                ));
            }
            Drawing::AnchoredVwapLine { bar_idx, color } => {
                if *bar_idx < chart.bars.len() {
                    let mut cum_vol_price = 0.0_f64;
                    let mut cum_vol = 0.0_f64;
                    let mut prev_pt: Option<egui::Pos2> = None;
                    for i in *bar_idx..chart.bars.len() {
                        let bar = &chart.bars[i];
                        let typical = (bar.high + bar.low + bar.close) / 3.0;
                        cum_vol_price += typical * bar.volume;
                        cum_vol += bar.volume;
                        let vwap = if cum_vol > 0.0 {
                            cum_vol_price / cum_vol
                        } else {
                            typical
                        };
                        if i >= start_idx && i < end_idx {
                            let x =
                                chart_rect.left() + ((i as f32 - start_idx as f32) + 0.5) * bar_w;
                            let y = price_to_y(vwap);
                            let pt = egui::pos2(x, y);
                            if let Some(p) = prev_pt {
                                painter.line_segment(
                                    [p, pt],
                                    egui::Stroke::new(effective_width, sel_tint(*color)),
                                );
                            }
                            prev_pt = Some(pt);
                        } else {
                            prev_pt = None;
                        }
                    }
                    // Label
                    if let Some(last) = prev_pt {
                        painter.text(
                            egui::pos2(last.x + 4.0, last.y),
                            egui::Align2::LEFT_CENTER,
                            "aVWAP",
                            egui::FontId::monospace(9.0),
                            *color,
                        );
                    }
                }
            }
            Drawing::TrendChannel { p1, p2, p3, color } => {
                let to_x = |idx: usize| -> Option<f32> {
                    if idx >= start_idx && idx < end_idx {
                        Some(chart_rect.left() + ((idx - start_idx) as f32 + 0.5) * bar_w)
                    } else {
                        None
                    }
                };
                if let (Some(x1), Some(x2)) = (to_x(p1.0), to_x(p2.0)) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    let ch_offset = p3.1 - p1.1;
                    let sc = sel_tint(*color);
                    // Main trendline
                    draw_line(
                        &painter,
                        egui::pos2(x1, y1),
                        egui::pos2(x2, y2),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    // Parallel line
                    let y1p = price_to_y(p1.1 + ch_offset);
                    let y2p = price_to_y(p2.1 + ch_offset);
                    draw_line(
                        &painter,
                        egui::pos2(x1, y1p),
                        egui::pos2(x2, y2p),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    // Mid line (dashed)
                    let y1m = price_to_y(p1.1 + ch_offset * 0.5);
                    let y2m = price_to_y(p2.1 + ch_offset * 0.5);
                    draw_line(
                        &painter,
                        egui::pos2(x1, y1m),
                        egui::pos2(x2, y2m),
                        egui::Stroke::new(effective_width * 0.35, sc),
                        LineStyle::Dashed,
                    );
                    // Fill
                    let fill =
                        egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 18);
                    let poly = vec![
                        egui::pos2(x1, y1),
                        egui::pos2(x2, y2),
                        egui::pos2(x2, y2p),
                        egui::pos2(x1, y1p),
                    ];
                    painter.add(egui::Shape::convex_polygon(poly, fill, egui::Stroke::NONE));
                }
            }
            Drawing::InsidePitchfork {
                pivot,
                p2,
                p3,
                color,
            } => {
                let to_pt = |idx: usize, price: f64| -> Option<egui::Pos2> {
                    if idx >= start_idx && idx < end_idx {
                        Some(egui::pos2(
                            chart_rect.left() + ((idx - start_idx) as f32 + 0.5) * bar_w,
                            price_to_y(price),
                        ))
                    } else {
                        None
                    }
                };
                if let (Some(pv), Some(a), Some(b)) = (
                    to_pt(pivot.0, pivot.1),
                    to_pt(p2.0, p2.1),
                    to_pt(p3.0, p3.1),
                ) {
                    let sc = sel_tint(*color);
                    // Inside pitchfork: median from midpoint of p2-p3 through pivot, extended
                    let mid = egui::pos2((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
                    // Median line from pivot through midpoint, extended 2x
                    let dx = mid.x - pv.x;
                    let dy = mid.y - pv.y;
                    let ext = egui::pos2(pv.x + dx * 2.5, pv.y + dy * 2.5);
                    draw_line(
                        &painter,
                        pv,
                        ext,
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    // Prongs from p2 and p3, parallel to median
                    let ext_a = egui::pos2(a.x + dx * 2.0, a.y + dy * 2.0);
                    let ext_b = egui::pos2(b.x + dx * 2.0, b.y + dy * 2.0);
                    draw_line(
                        &painter,
                        a,
                        ext_a,
                        egui::Stroke::new(effective_width * 0.7, sc),
                        d_style,
                    );
                    draw_line(
                        &painter,
                        b,
                        ext_b,
                        egui::Stroke::new(effective_width * 0.7, sc),
                        d_style,
                    );
                    // Connect pivot to p2 and p3
                    draw_line(
                        &painter,
                        pv,
                        a,
                        egui::Stroke::new(effective_width * 0.4, sc),
                        d_style,
                    );
                    draw_line(
                        &painter,
                        pv,
                        b,
                        egui::Stroke::new(effective_width * 0.4, sc),
                        d_style,
                    );
                }
            }
            Drawing::FibWedge { p1, p2, p3, color } => {
                let to_pt = |idx: usize, price: f64| -> Option<egui::Pos2> {
                    if idx >= start_idx && idx < end_idx {
                        Some(egui::pos2(
                            chart_rect.left() + ((idx - start_idx) as f32 + 0.5) * bar_w,
                            price_to_y(price),
                        ))
                    } else {
                        None
                    }
                };
                if let (Some(a), Some(b), Some(c)) =
                    (to_pt(p1.0, p1.1), to_pt(p2.0, p2.1), to_pt(p3.0, p3.1))
                {
                    let sc = sel_tint(*color);
                    // Two converging trendlines: p1->p2 and p1->p3
                    draw_line(
                        &painter,
                        a,
                        b,
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    draw_line(
                        &painter,
                        a,
                        c,
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    // Fib levels between the two lines
                    let levels = [0.236, 0.382, 0.5, 0.618, 0.786];
                    let names = ["23.6%", "38.2%", "50%", "61.8%", "78.6%"];
                    for (i, &lvl) in levels.iter().enumerate() {
                        let lb = egui::pos2(
                            a.x + (b.x - a.x) * lvl as f32,
                            a.y + (b.y - a.y) * lvl as f32,
                        );
                        let lc = egui::pos2(
                            a.x + (c.x - a.x) * lvl as f32,
                            a.y + (c.y - a.y) * lvl as f32,
                        );
                        let alpha = if lvl == 0.5 { 140 } else { 80 };
                        let lc2 = egui::Color32::from_rgba_premultiplied(
                            color.r(),
                            color.g(),
                            color.b(),
                            alpha,
                        );
                        painter.line_segment([lb, lc], egui::Stroke::new(0.7, lc2));
                        painter.text(
                            egui::pos2(lc.x + 3.0, lc.y),
                            egui::Align2::LEFT_CENTER,
                            names[i],
                            egui::FontId::monospace(8.0),
                            lc2,
                        );
                    }
                    // Fill between the two lines
                    let fill =
                        egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 12);
                    painter.add(egui::Shape::convex_polygon(
                        vec![a, b, c],
                        fill,
                        egui::Stroke::NONE,
                    ));
                }
            }
            Drawing::PriceNote { price, text, color } => {
                let y = price_to_y(*price);
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    // Dashed horizontal line
                    let alpha_line =
                        egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 80);
                    painter.line_segment(
                        [
                            egui::pos2(chart_rect.left(), y),
                            egui::pos2(chart_rect.right(), y),
                        ],
                        egui::Stroke::new(0.5, alpha_line),
                    );
                    // Text box
                    let pad = 4.0_f32;
                    let galley =
                        painter.layout_no_wrap(text.clone(), egui::FontId::monospace(10.0), *color);
                    let tw = galley.rect.width();
                    let th = galley.rect.height();
                    let box_rect = egui::Rect::from_min_size(
                        egui::pos2(chart_rect.left() + 10.0, y - th - pad * 2.0),
                        egui::vec2(tw + pad * 2.0, th + pad * 2.0),
                    );
                    let bg = egui::Color32::from_rgba_premultiplied(25, 20, 35, 230);
                    painter.rect_filled(box_rect, 3.0, bg);
                    painter.rect_stroke(
                        box_rect,
                        3.0,
                        egui::Stroke::new(1.0, *color),
                        egui::StrokeKind::Outside,
                    );
                    painter.galley(
                        egui::pos2(chart_rect.left() + 10.0 + pad, y - th - pad),
                        galley,
                        *color,
                    );
                    // Price badge
                    let label = format!("{:.5}", price);
                    painter.text(
                        egui::pos2(chart_rect.right() - 4.0, y - 2.0),
                        egui::Align2::RIGHT_BOTTOM,
                        &label,
                        egui::FontId::monospace(8.0),
                        *color,
                    );
                }
            }
            Drawing::MeasureTool { p1, p2, color } => {
                let x1o = if p1.0 >= start_idx && p1.0 < end_idx {
                    Some(chart_rect.left() + ((p1.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                let x2o = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(chart_rect.left() + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else {
                    None
                };
                if let (Some(x1), Some(x2)) = (x1o, x2o) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    // Connecting line
                    let sc = sel_tint(*color);
                    draw_line(
                        &painter,
                        egui::pos2(x1, y1),
                        egui::pos2(x2, y2),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    // Compute measurements
                    let bars_count = if p2.0 > p1.0 {
                        p2.0 - p1.0
                    } else {
                        p1.0 - p2.0
                    };
                    let price_diff = p2.1 - p1.1;
                    let pct = if p1.1.abs() > 1e-10 {
                        (price_diff / p1.1) * 100.0
                    } else {
                        0.0
                    };
                    let dx = x2 - x1;
                    let dy = y2 - y1;
                    let angle_deg = if dx.abs() > 0.01 {
                        (dy / dx).atan().to_degrees()
                    } else {
                        90.0
                    };
                    // R:R placeholder (1:1 without SL/TP context)
                    let info = format!(
                        "{} bars | {:.5} | {:.2}% | {:.1}° | R:R 1:1",
                        bars_count, price_diff, pct, angle_deg
                    );
                    // Background box
                    let mid_x = (x1 + x2) / 2.0;
                    let mid_y = (y1 + y2) / 2.0;
                    let pad = 4.0_f32;
                    let galley = painter.layout_no_wrap(info, egui::FontId::monospace(9.0), *color);
                    let tw = galley.rect.width();
                    let th = galley.rect.height();
                    let box_rect = egui::Rect::from_min_size(
                        egui::pos2(mid_x - tw / 2.0 - pad, mid_y - th - pad * 2.0),
                        egui::vec2(tw + pad * 2.0, th + pad * 2.0),
                    );
                    let bg = egui::Color32::from_rgba_premultiplied(15, 15, 25, 220);
                    painter.rect_filled(box_rect, 3.0, bg);
                    painter.rect_stroke(
                        box_rect,
                        3.0,
                        egui::Stroke::new(1.0, *color),
                        egui::StrokeKind::Outside,
                    );
                    painter.galley(
                        egui::pos2(mid_x - tw / 2.0, mid_y - th - pad),
                        galley,
                        *color,
                    );
                    // Endpoint markers
                    painter.circle_filled(egui::pos2(x1, y1), 3.0, *color);
                    painter.circle_filled(egui::pos2(x2, y2), 3.0, *color);
                }
            }
            Drawing::AnchoredText {
                bar_idx,
                price,
                text,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = chart_rect.left() + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    painter.text(
                        egui::pos2(x, y),
                        egui::Align2::LEFT_BOTTOM,
                        text,
                        egui::FontId::monospace(11.0),
                        sel_tint(*color),
                    );
                }
            }
            Drawing::Comment {
                bar_idx,
                price,
                text,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = chart_rect.left() + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    let sc = sel_tint(*color);
                    let galley =
                        painter.layout_no_wrap(text.clone(), egui::FontId::monospace(9.0), sc);
                    let tw = galley.rect.width();
                    let th = galley.rect.height();
                    let pad = 3.0_f32;
                    let br = egui::Rect::from_min_size(
                        egui::pos2(x - pad, y - th - pad * 2.0),
                        egui::vec2(tw + pad * 2.0, th + pad * 2.0),
                    );
                    painter.rect_filled(
                        br,
                        2.0,
                        egui::Color32::from_rgba_premultiplied(20, 20, 30, 200),
                    );
                    painter.rect_stroke(
                        br,
                        2.0,
                        egui::Stroke::new(1.0, sc),
                        egui::StrokeKind::Outside,
                    );
                    painter.galley(egui::pos2(x, y - th - pad), galley, sc);
                }
            }
            Drawing::ArrowMarkerLeft {
                bar_idx,
                price,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = chart_rect.left() + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    let sc = sel_tint(*color);
                    let sz = 8.0_f32;
                    painter.add(egui::Shape::convex_polygon(
                        vec![
                            egui::pos2(x - sz, y),
                            egui::pos2(x + sz * 0.5, y - sz * 0.7),
                            egui::pos2(x + sz * 0.5, y + sz * 0.7),
                        ],
                        sc,
                        egui::Stroke::NONE,
                    ));
                }
            }
            Drawing::ArrowMarkerRight {
                bar_idx,
                price,
                color,
            } => {
                if *bar_idx >= start_idx && *bar_idx < end_idx {
                    let x = chart_rect.left() + ((*bar_idx - start_idx) as f32 + 0.5) * bar_w;
                    let y = price_to_y(*price);
                    let sc = sel_tint(*color);
                    let sz = 8.0_f32;
                    painter.add(egui::Shape::convex_polygon(
                        vec![
                            egui::pos2(x + sz, y),
                            egui::pos2(x - sz * 0.5, y - sz * 0.7),
                            egui::pos2(x - sz * 0.5, y + sz * 0.7),
                        ],
                        sc,
                        egui::Stroke::NONE,
                    ));
                }
            }
            Drawing::Circle { p1, p2, color } => {
                if p1.0 >= start_idx && p1.0 < end_idx && p2.0 >= start_idx && p2.0 < end_idx {
                    let cx = chart_rect.left() + ((p1.0 - start_idx) as f32 + 0.5) * bar_w;
                    let cy = price_to_y(p1.1);
                    let rx = chart_rect.left() + ((p2.0 - start_idx) as f32 + 0.5) * bar_w;
                    let ry = price_to_y(p2.1);
                    let radius = ((rx - cx).powi(2) + (ry - cy).powi(2)).sqrt();
                    painter.circle_stroke(
                        egui::pos2(cx, cy),
                        radius,
                        egui::Stroke::new(effective_width, sel_tint(*color)),
                    );
                }
            }
            Drawing::PitchFan { p1, p2, color }
            | Drawing::TrendFibTime { p1, p2, color }
            | Drawing::GannSquare { p1, p2, color }
            | Drawing::GannSquareFixed { p1, p2, color }
            | Drawing::BarsPattern { p1, p2, color }
            | Drawing::Projection { p1, p2, color }
            | Drawing::DoubleCurve { p1, p2, color } => {
                if p1.0 >= start_idx && p1.0 < end_idx && p2.0 >= start_idx && p2.0 < end_idx {
                    let x1 = chart_rect.left() + ((p1.0 - start_idx) as f32 + 0.5) * bar_w;
                    let y1 = price_to_y(p1.1);
                    let x2 = chart_rect.left() + ((p2.0 - start_idx) as f32 + 0.5) * bar_w;
                    let y2 = price_to_y(p2.1);
                    let sc = sel_tint(*color);
                    draw_line(
                        &painter,
                        egui::pos2(x1, y1),
                        egui::pos2(x2, y2),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                    painter.circle_filled(egui::pos2(x1, y1), 3.0, sc);
                    painter.circle_filled(egui::pos2(x2, y2), 3.0, sc);
                }
            }
            Drawing::TrianglePattern { points, color }
            | Drawing::ThreeDrives { points, color }
            | Drawing::ElliottDouble { points, color }
            | Drawing::AbcdPattern { points, color }
            | Drawing::CypherPattern { points, color }
            | Drawing::ElliottTriangle { points, color }
            | Drawing::ElliottTripleCombo { points, color } => {
                let labels: &[&str] = match drawing {
                    Drawing::TrianglePattern { .. } => &["A", "B", "C"],
                    Drawing::ThreeDrives { .. } => &["1", "2", "3"],
                    Drawing::ElliottDouble { .. } => &["W", "X", "Y"],
                    Drawing::AbcdPattern { .. } => &["A", "B", "C", "D"],
                    Drawing::CypherPattern { .. } => &["X", "A", "B", "C", "D"],
                    Drawing::ElliottTriangle { .. } => &["A", "B", "C", "D", "E"],
                    Drawing::ElliottTripleCombo { .. } => &["W", "X", "Y", "X", "Z"],
                    _ => &[],
                };
                let screen_pts: Vec<(f32, f32)> = points
                    .iter()
                    .filter(|(bi, _)| *bi >= start_idx && *bi < end_idx)
                    .map(|(bi, pr)| {
                        (
                            chart_rect.left() + ((*bi - start_idx) as f32 + 0.5) * bar_w,
                            price_to_y(*pr),
                        )
                    })
                    .collect();
                let sc = sel_tint(*color);
                for w in screen_pts.windows(2) {
                    draw_line(
                        &painter,
                        egui::pos2(w[0].0, w[0].1),
                        egui::pos2(w[1].0, w[1].1),
                        egui::Stroke::new(effective_width, sc),
                        d_style,
                    );
                }
                for (i, &(x, y)) in screen_pts.iter().enumerate() {
                    painter.circle_filled(egui::pos2(x, y), 3.0, sc);
                    if i < labels.len() {
                        painter.text(
                            egui::pos2(x, y - 12.0),
                            egui::Align2::CENTER_BOTTOM,
                            labels[i],
                            egui::FontId::monospace(10.0),
                            sc,
                        );
                    }
                }
            }
        }
    }

    // ── Drawing Preview (ghost line during placement) ─────────────────────
    // When a drawing tool is active and the user has placed the first point,
    // render a semi-transparent preview line/shape from the first point to the
    // current mouse position. This gives immediate visual feedback — the user
    // sees exactly what the drawing will look like before committing.
    if let Some(cross) = crosshair {
        let preview_color = egui::Color32::from_rgba_premultiplied(200, 200, 255, 120);
        let preview_stroke = egui::Stroke::new(1.5, preview_color);
        // Convert crosshair to bar/price
        let mouse_rel = ((cross.x - chart_rect.left()) / bar_w).max(0.0) as usize;
        let _mouse_bar = start_idx + mouse_rel.min(end_idx.saturating_sub(start_idx + 1));
        let mouse_price = {
            let frac = (cross.y - chart_rect.top()) / chart_rect.height();
            price_max - frac as f64 * (price_max - price_min)
        };
        let _ = mouse_price;

        // Helper: convert (bar_idx, price) to screen pos
        let to_screen = |bar: usize, price: f64| -> Option<egui::Pos2> {
            if bar >= start_idx && bar < end_idx {
                let x = chart_rect.left() + ((bar - start_idx) as f32 + 0.5) * bar_w;
                let y = price_to_y(price);
                Some(egui::pos2(x, y))
            } else {
                None
            }
        };

        // Generic preview: extract first point from any P2 state, draw line to cursor.
        // Extract second point from any P3 state, draw P1→P2→cursor.
        // This covers all 70+ drawing types without naming every variant.
        let p1_data: Option<(usize, f64)> = {
            // Use debug format to extract bar1/price1 from any P2 variant
            let dm_str = format!("{:?}", draw_mode);
            if dm_str.contains("bar1:") && dm_str.contains("price1:") {
                // Parse bar1 and price1 from debug string
                let bar1 = dm_str
                    .split("bar1: ")
                    .nth(1)
                    .and_then(|s| s.split([',', ' ', '}']).next())
                    .and_then(|s| s.parse::<usize>().ok());
                let price1 = dm_str
                    .split("price1: ")
                    .nth(1)
                    .and_then(|s| s.split([',', ' ', '}']).next())
                    .and_then(|s| s.parse::<f64>().ok());
                bar1.zip(price1)
            } else {
                None
            }
        };
        let p2_data: Option<(usize, f64)> = {
            let dm_str = format!("{:?}", draw_mode);
            if dm_str.contains("bar2:") && dm_str.contains("price2:") {
                let bar2 = dm_str
                    .split("bar2: ")
                    .nth(1)
                    .and_then(|s| s.split([',', ' ', '}']).next())
                    .and_then(|s| s.parse::<usize>().ok());
                let price2 = dm_str
                    .split("price2: ")
                    .nth(1)
                    .and_then(|s| s.split([',', ' ', '}']).next())
                    .and_then(|s| s.parse::<f64>().ok());
                bar2.zip(price2)
            } else {
                None
            }
        };

        match draw_mode {
            DrawMode::PlacingHLine => {
                painter.line_segment(
                    [
                        egui::pos2(chart_rect.left(), cross.y),
                        egui::pos2(chart_rect.right(), cross.y),
                    ],
                    preview_stroke,
                );
            }
            DrawMode::PlacingVLine => {
                painter.line_segment(
                    [
                        egui::pos2(cross.x, chart_rect.top()),
                        egui::pos2(cross.x, chart_rect.bottom()),
                    ],
                    preview_stroke,
                );
            }
            DrawMode::PlacingHRay => {
                painter.line_segment(
                    [
                        egui::pos2(cross.x, cross.y),
                        egui::pos2(chart_rect.right(), cross.y),
                    ],
                    preview_stroke,
                );
            }
            DrawMode::PlacingCrossLine => {
                painter.line_segment(
                    [
                        egui::pos2(chart_rect.left(), cross.y),
                        egui::pos2(chart_rect.right(), cross.y),
                    ],
                    preview_stroke,
                );
                painter.line_segment(
                    [
                        egui::pos2(cross.x, chart_rect.top()),
                        egui::pos2(cross.x, chart_rect.bottom()),
                    ],
                    preview_stroke,
                );
            }
            DrawMode::None => {}
            _ => {
                // Generic preview for all P2 states (point 1 placed, drawing line to cursor)
                if let Some((bar1, price1)) = p1_data {
                    if let Some(p1) = to_screen(bar1, price1) {
                        if let Some((bar2, price2)) = p2_data {
                            // P3 state: show P1→P2 solid, P2→cursor ghost
                            if let Some(p2) = to_screen(bar2, price2) {
                                painter
                                    .line_segment([p1, p2], egui::Stroke::new(1.5, preview_color));
                                painter.line_segment(
                                    [p2, cross],
                                    egui::Stroke::new(
                                        1.0,
                                        egui::Color32::from_rgba_premultiplied(200, 200, 255, 80),
                                    ),
                                );
                                painter.circle_filled(p1, 4.0, preview_color);
                                painter.circle_filled(p2, 4.0, preview_color);
                                painter.circle_stroke(cross, 4.0, preview_stroke);
                            }
                        } else {
                            // P2 state: show P1→cursor ghost line
                            painter.line_segment([p1, cross], preview_stroke);
                            painter.circle_filled(p1, 4.0, preview_color);
                            painter.circle_stroke(cross, 4.0, preview_stroke);
                        }
                    }
                }
            }
        }
    }
}

/// Draw an oscillator sub-pane (RSI, etc.) with optional overbought/oversold levels.
pub(super) fn draw_oscillator_pane(
    painter: &egui::Painter,
    rect: egui::Rect,
    bars: &[Bar],
    series: &[Option<f64>],
    start_idx: usize,
    bar_w: f32,
    label: &str,
    color: egui::Color32,
    val_min: f64,
    val_max: f64,
    ob_level: Option<f64>,
    os_level: Option<f64>,
) {
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(0, 0, 0));
    // Sub-pane border-top separator (#444 matching old WebKit)
    painter.line_segment(
        [
            egui::pos2(rect.left(), rect.top()),
            egui::pos2(rect.right(), rect.top()),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 68, 68)),
    );

    let val_to_y = |v: f64| -> f32 {
        let frac = (val_max - v) / (val_max - val_min);
        rect.top() + frac as f32 * rect.height()
    };

    // OB/OS levels
    let level_color = egui::Color32::from_rgba_premultiplied(140, 140, 160, 60);
    if let Some(ob) = ob_level {
        let y = val_to_y(ob);
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(0.5, level_color),
        );
    }
    if let Some(os) = os_level {
        let y = val_to_y(os);
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(0.5, level_color),
        );
    }
    // Mid line
    let mid_y = val_to_y((val_max + val_min) / 2.0);
    painter.line_segment(
        [
            egui::pos2(rect.left(), mid_y),
            egui::pos2(rect.right(), mid_y),
        ],
        egui::Stroke::new(0.3, GRID),
    );

    // Data line. Sub-panes share the main chart's pixel-aware decimation so
    // dense views don't upload invisible sub-pixel oscillator vertices.
    let sample_step = chart_render_sample_step(bars.len(), rect.width());
    let mut points: Vec<egui::Pos2> = Vec::with_capacity(bars.len() / sample_step + 1);
    for (rel_idx, _) in bars.iter().enumerate().step_by(sample_step) {
        let abs_idx = start_idx + rel_idx;
        if abs_idx >= series.len() {
            continue;
        }
        if let Some(v) = series[abs_idx] {
            let x = rect.left() + (rel_idx as f32 + 0.5) * bar_w;
            let y = val_to_y(v).clamp(rect.top(), rect.bottom());
            points.push(egui::pos2(x, y));
        }
    }
    if points.len() > 1 {
        painter.add(egui::Shape::line(points, egui::Stroke::new(1.5, color)));
    }

    // Label
    painter.text(
        egui::pos2(rect.left() + 4.0, rect.top() + 2.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::FontId::monospace(9.0),
        egui::Color32::WHITE,
    );
}

/// Draw Fisher Transform sub-pane with color-coded histogram bars.
pub(super) fn draw_fisher_pane(
    painter: &egui::Painter,
    rect: egui::Rect,
    bars: &[Bar],
    fisher: &[Option<f64>],
    signal: &[Option<f64>],
    start_idx: usize,
    bar_w: f32,
) {
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(0, 0, 0));
    // Sub-pane border-top separator (#444 matching old WebKit)
    painter.line_segment(
        [
            egui::pos2(rect.left(), rect.top()),
            egui::pos2(rect.right(), rect.top()),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 68, 68)),
    );

    // Fisher typically ranges -3..3, auto-scale
    let mut f_min = -2.0_f64;
    let mut f_max = 2.0_f64;
    for (rel_idx, _) in bars.iter().enumerate() {
        let abs_idx = start_idx + rel_idx;
        if abs_idx >= fisher.len() {
            continue;
        }
        if let Some(v) = fisher[abs_idx] {
            f_min = f_min.min(v);
            f_max = f_max.max(v);
        }
    }
    let padding = (f_max - f_min) * 0.1;
    f_min -= padding;
    f_max += padding;

    let val_to_y = |v: f64| -> f32 {
        let frac = (f_max - v) / (f_max - f_min);
        rect.top() + frac as f32 * rect.height()
    };

    let sample_step = chart_render_sample_step(bars.len(), rect.width());

    // Zero line. Use one primitive instead of dotted per-pixel segment spam.
    let zero_y = val_to_y(0.0);
    painter.line_segment(
        [
            egui::pos2(rect.left(), zero_y),
            egui::pos2(rect.right(), zero_y),
        ],
        egui::Stroke::new(0.5, egui::Color32::from_rgb(60, 60, 60)),
    );

    // Signal line FIRST (behind Fisher — MT5: clrDarkGray/orange, width 1)
    let mut sig_points: Vec<egui::Pos2> = Vec::with_capacity(bars.len() / sample_step + 1);
    for (rel_idx, _) in bars.iter().enumerate().step_by(sample_step) {
        let abs_idx = start_idx + rel_idx;
        if abs_idx >= signal.len() {
            continue;
        }
        if let Some(v) = signal[abs_idx] {
            let x = rect.left() + (rel_idx as f32 + 0.5) * bar_w;
            let y = val_to_y(v).clamp(rect.top(), rect.bottom());
            sig_points.push(egui::pos2(x, y));
        }
    }
    if sig_points.len() > 1 {
        painter.add(egui::Shape::line(
            sig_points,
            egui::Stroke::new(1.0, FISHER_SIG),
        )); // clrDarkGray signal (MT5 buffer 3)
    }

    // Fisher line — colored segments per sampled bar (MT5 exact: green when Fisher > Signal, red when < Signal)
    // NO histogram bars — just the line (matching MT5 screenshot exactly)
    for (rel_idx, _) in bars.iter().enumerate().step_by(sample_step) {
        let abs_idx = start_idx + rel_idx;
        let next_rel_idx = (rel_idx + sample_step).min(bars.len().saturating_sub(1));
        let next_abs_idx = start_idx + next_rel_idx;
        if next_abs_idx >= fisher.len() || next_rel_idx == rel_idx {
            continue;
        }
        if let (Some(f0), Some(f1)) = (fisher[abs_idx], fisher[next_abs_idx]) {
            let sig = if abs_idx < signal.len() {
                signal[abs_idx]
            } else {
                None
            };
            // MT5: clrMediumSeaGreen when Fisher > Signal, clrOrangeRed when Fisher < Signal
            let color = match sig {
                Some(s) if f0 > s => FISHER_POS, // green
                Some(_) => FISHER_NEG,           // red
                None => {
                    if f0 >= 0.0 {
                        FISHER_POS
                    } else {
                        FISHER_NEG
                    }
                }
            };
            let x0 = rect.left() + (rel_idx as f32 + 0.5) * bar_w;
            let x1 = rect.left() + (next_rel_idx as f32 + 0.5) * bar_w;
            let y0 = val_to_y(f0).clamp(rect.top(), rect.bottom());
            let y1 = val_to_y(f1).clamp(rect.top(), rect.bottom());
            painter.line_segment(
                [egui::pos2(x0, y0), egui::pos2(x1, y1)],
                egui::Stroke::new(2.0, color),
            );
        }
    }

    // Label with current values (MT5 style: "Ehlers Fisher transform (32) -2.037 -2.068")
    let last_fisher = fisher.iter().rev().find_map(|v| *v);
    let last_signal = signal.iter().rev().find_map(|v| *v);
    let label = match (last_fisher, last_signal) {
        (Some(f), Some(s)) => format!("Ehlers Fisher transform (32) {:.3} {:.3}", f, s),
        (Some(f), None) => format!("Ehlers Fisher transform (32) {:.3}", f),
        _ => "Ehlers Fisher transform (32)".to_string(),
    };
    painter.text(
        egui::pos2(rect.left() + 4.0, rect.top() + 2.0),
        egui::Align2::LEFT_TOP,
        &label,
        egui::FontId::monospace(9.0),
        egui::Color32::WHITE,
    );
}

/// Draw MACD sub-pane with two lines + histogram.
pub(super) fn draw_macd_pane(
    painter: &egui::Painter,
    rect: egui::Rect,
    bars: &[Bar],
    macd_line: &[Option<f64>],
    macd_signal: &[Option<f64>],
    macd_hist: &[Option<f64>],
    start_idx: usize,
    bar_w: f32,
    label: &str,
    line_color: egui::Color32,
    signal_color: egui::Color32,
) {
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(0, 0, 0));
    // Sub-pane border-top separator (#444 matching old WebKit)
    painter.line_segment(
        [
            egui::pos2(rect.left(), rect.top()),
            egui::pos2(rect.right(), rect.top()),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 68, 68)),
    );

    // Auto-scale
    let mut v_min = 0.0_f64;
    let mut v_max = 0.0_f64;
    for (rel_idx, _) in bars.iter().enumerate() {
        let abs_idx = start_idx + rel_idx;
        if abs_idx >= macd_line.len() {
            continue;
        }
        for series in [macd_line, macd_signal, macd_hist] {
            if let Some(Some(v)) = series.get(abs_idx) {
                v_min = v_min.min(*v);
                v_max = v_max.max(*v);
            }
        }
    }
    let padding = (v_max - v_min).max(0.001) * 0.1;
    v_min -= padding;
    v_max += padding;

    let val_to_y = |v: f64| -> f32 {
        let frac = (v_max - v) / (v_max - v_min);
        rect.top() + frac as f32 * rect.height()
    };

    let sample_step = chart_render_sample_step(bars.len(), rect.width());

    // Zero line
    let zero_y = val_to_y(0.0);
    painter.line_segment(
        [
            egui::pos2(rect.left(), zero_y),
            egui::pos2(rect.right(), zero_y),
        ],
        egui::Stroke::new(0.3, GRID),
    );

    // Histogram. Preserve the strongest absolute bar in each sampled bucket so
    // dense rendering does not hide spikes while still emitting ~pixel-count rects.
    let hist_w = (bar_w * sample_step as f32 * 0.6).max(1.0);
    for rel_idx in (0..bars.len()).step_by(sample_step) {
        let bucket_end = (rel_idx + sample_step).min(bars.len());
        let mut selected: Option<(usize, f64)> = None;
        for bucket_rel in rel_idx..bucket_end {
            let abs_idx = start_idx + bucket_rel;
            if let Some(Some(v)) = macd_hist.get(abs_idx) {
                if selected.map_or(true, |(_, cur)| v.abs() > cur.abs()) {
                    selected = Some((bucket_rel, *v));
                }
            }
        }
        if let Some((bucket_rel, v)) = selected {
            let x = rect.left() + (bucket_rel as f32 + 0.5) * bar_w;
            let y = val_to_y(v);
            // MACD histogram: teal green positive, coral red negative (TradingView/MT5 style)
            let color = if v >= 0.0 {
                egui::Color32::from_rgb(38, 166, 154) // #26a69a (teal green)
            } else {
                egui::Color32::from_rgb(239, 83, 80) // #ef5350 (coral red)
            };
            let (top, bottom) = if v >= 0.0 { (y, zero_y) } else { (zero_y, y) };
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(x - hist_w / 2.0, top),
                    egui::pos2(x + hist_w / 2.0, bottom),
                ),
                0.0,
                color,
            );
        }
    }

    // MACD line
    let mut points: Vec<egui::Pos2> = Vec::with_capacity(bars.len() / sample_step + 1);
    for (rel_idx, _) in bars.iter().enumerate().step_by(sample_step) {
        let abs_idx = start_idx + rel_idx;
        if let Some(Some(v)) = macd_line.get(abs_idx) {
            points.push(egui::pos2(
                rect.left() + (rel_idx as f32 + 0.5) * bar_w,
                val_to_y(*v).clamp(rect.top(), rect.bottom()),
            ));
        }
    }
    if points.len() > 1 {
        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(1.5, line_color),
        ));
    }

    // Signal line
    let mut points: Vec<egui::Pos2> = Vec::with_capacity(bars.len() / sample_step + 1);
    for (rel_idx, _) in bars.iter().enumerate().step_by(sample_step) {
        let abs_idx = start_idx + rel_idx;
        if let Some(Some(v)) = macd_signal.get(abs_idx) {
            points.push(egui::pos2(
                rect.left() + (rel_idx as f32 + 0.5) * bar_w,
                val_to_y(*v).clamp(rect.top(), rect.bottom()),
            ));
        }
    }
    if points.len() > 1 {
        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(1.0, signal_color),
        ));
    }

    painter.text(
        egui::pos2(rect.left() + 4.0, rect.top() + 2.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::FontId::monospace(9.0),
        egui::Color32::WHITE,
    );
}

/// Draw volume bars sub-pane.
pub(super) fn draw_volume_pane(
    painter: &egui::Painter,
    rect: egui::Rect,
    bars: &[Bar],
    _start_idx: usize,
    bar_w: f32,
) {
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(0, 0, 0));
    // Sub-pane border-top separator (#444 matching old WebKit)
    painter.line_segment(
        [
            egui::pos2(rect.left(), rect.top()),
            egui::pos2(rect.right(), rect.top()),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 68, 68)),
    );

    if bars.is_empty() {
        return;
    }
    let max_vol = bars.iter().map(|b| b.volume).fold(0.0_f64, f64::max);
    if max_vol <= 0.0 {
        return;
    }

    let sample_step = chart_render_sample_step(bars.len(), rect.width());
    let hist_w = (bar_w * sample_step as f32 * 0.7).max(1.0);
    for rel_idx in (0..bars.len()).step_by(sample_step) {
        let bucket_end = (rel_idx + sample_step).min(bars.len());
        let Some((bucket_rel, b)) = bars[rel_idx..bucket_end]
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.volume.total_cmp(&b.volume))
            .map(|(offset, b)| (rel_idx + offset, b))
        else {
            continue;
        };
        let x = rect.left() + (bucket_rel as f32 + 0.5) * bar_w;
        let h = (b.volume / max_vol) as f32 * rect.height();
        let color = if b.close >= b.open {
            egui::Color32::from_rgba_premultiplied(UP.r(), UP.g(), UP.b(), 150)
        } else {
            egui::Color32::from_rgba_premultiplied(DOWN.r(), DOWN.g(), DOWN.b(), 150)
        };
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(x - hist_w / 2.0, rect.bottom() - h),
                egui::pos2(x + hist_w / 2.0, rect.bottom()),
            ),
            0.0,
            color,
        );
    }

    painter.text(
        egui::pos2(rect.left() + 4.0, rect.top() + 2.0),
        egui::Align2::LEFT_TOP,
        "Volume",
        egui::FontId::monospace(9.0),
        egui::Color32::WHITE,
    );
}

/// Draw Better Volume sub-pane (NNFX-style color-coded volume).
pub(super) fn draw_better_volume_pane(
    painter: &egui::Painter,
    rect: egui::Rect,
    bars: &[Bar],
    vol_type: &[u8],
    start_idx: usize,
    bar_w: f32,
) {
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(0, 0, 0));
    // Sub-pane border-top separator (#444 matching old WebKit)
    painter.line_segment(
        [
            egui::pos2(rect.left(), rect.top()),
            egui::pos2(rect.right(), rect.top()),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 68, 68)),
    );

    if bars.is_empty() {
        return;
    }
    let max_vol = bars.iter().map(|b| b.volume).fold(0.0_f64, f64::max);
    if max_vol <= 0.0 {
        return;
    }

    let sample_step = chart_render_sample_step(bars.len(), rect.width());
    let hist_w = (bar_w * sample_step as f32 * 0.7).max(1.0);
    for rel_idx in (0..bars.len()).step_by(sample_step) {
        let bucket_end = (rel_idx + sample_step).min(bars.len());
        let Some((bucket_rel, b)) = bars[rel_idx..bucket_end]
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.volume.total_cmp(&b.volume))
            .map(|(offset, b)| (rel_idx + offset, b))
        else {
            continue;
        };
        let abs_idx = start_idx + bucket_rel;
        let x = rect.left() + (bucket_rel as f32 + 0.5) * bar_w;
        let h = (b.volume / max_vol) as f32 * rect.height();
        let vt = vol_type.get(abs_idx).copied().unwrap_or(5);
        // MQL5 enum: 0=low(yellow), 1=climax_up(red), 2=climax_dn(white),
        //            3=churn(green), 4=climax_churn(magenta), 5=normal(steelblue)
        let color = match vt {
            0 => BVOL_LOW,       // Yellow — low volume
            1 => BVOL_CLIMAX_UP, // Red — climax up
            2 => BVOL_CLIMAX_DN, // White — climax down
            3 => BVOL_HIGH,      // Green — churn
            4 => BVOL_CHURN,     // Magenta — climax + churn
            _ => BVOL_NORMAL,    // SteelBlue — normal
        };
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(x - hist_w / 2.0, rect.bottom() - h),
                egui::pos2(x + hist_w / 2.0, rect.bottom()),
            ),
            0.0,
            color,
        );
    }
    // Label with current volume value (MT5 style: "BetterVol(20) 10748 0")
    let last_vol = bars.last().map(|b| b.volume as i64).unwrap_or(0);
    let label = format!("BetterVol(20) {} 0", last_vol);
    painter.text(
        egui::pos2(rect.left() + 4.0, rect.top() + 2.0),
        egui::Align2::LEFT_TOP,
        &label,
        egui::FontId::monospace(9.0),
        egui::Color32::WHITE,
    );
}

/// Draw Stochastic sub-pane.
pub(super) fn draw_stoch_pane(
    painter: &egui::Painter,
    rect: egui::Rect,
    bars: &[Bar],
    stoch_k: &[Option<f64>],
    stoch_d: &[Option<f64>],
    start_idx: usize,
    bar_w: f32,
    label: &str,
) {
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(0, 0, 0));
    // Sub-pane border-top separator (#444 matching old WebKit)
    painter.line_segment(
        [
            egui::pos2(rect.left(), rect.top()),
            egui::pos2(rect.right(), rect.top()),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 68, 68)),
    );

    let val_to_y = |v: f64| -> f32 {
        let frac = (100.0 - v) / 100.0;
        rect.top() + frac as f32 * rect.height()
    };

    // OB/OS levels
    let level_col = egui::Color32::from_rgba_premultiplied(140, 140, 160, 60);
    for &lvl in &[80.0, 20.0] {
        let y = val_to_y(lvl);
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(0.5, level_col),
        );
    }

    let sample_step = chart_render_sample_step(bars.len(), rect.width());

    // %K line
    let mut points: Vec<egui::Pos2> = Vec::with_capacity(bars.len() / sample_step + 1);
    for (rel_idx, _) in bars.iter().enumerate().step_by(sample_step) {
        if let Some(Some(v)) = stoch_k.get(start_idx + rel_idx) {
            points.push(egui::pos2(
                rect.left() + (rel_idx as f32 + 0.5) * bar_w,
                val_to_y(*v).clamp(rect.top(), rect.bottom()),
            ));
        }
    }
    if points.len() > 1 {
        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(1.5, STOCH_K_COL),
        ));
    }

    // %D line
    let mut points: Vec<egui::Pos2> = Vec::with_capacity(bars.len() / sample_step + 1);
    for (rel_idx, _) in bars.iter().enumerate().step_by(sample_step) {
        if let Some(Some(v)) = stoch_d.get(start_idx + rel_idx) {
            points.push(egui::pos2(
                rect.left() + (rel_idx as f32 + 0.5) * bar_w,
                val_to_y(*v).clamp(rect.top(), rect.bottom()),
            ));
        }
    }
    if points.len() > 1 {
        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(1.0, STOCH_D_COL),
        ));
    }

    painter.text(
        egui::pos2(rect.left() + 4.0, rect.top() + 2.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::FontId::monospace(9.0),
        egui::Color32::WHITE,
    );
}

/// Draw ADX + DI+/DI- sub-pane.
pub(super) fn draw_adx_pane(
    painter: &egui::Painter,
    rect: egui::Rect,
    bars: &[Bar],
    adx: &[Option<f64>],
    di_plus: &[Option<f64>],
    di_minus: &[Option<f64>],
    start_idx: usize,
    bar_w: f32,
) {
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(0, 0, 0));
    // Sub-pane border-top separator (#444 matching old WebKit)
    painter.line_segment(
        [
            egui::pos2(rect.left(), rect.top()),
            egui::pos2(rect.right(), rect.top()),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 68, 68)),
    );

    // Auto-scale 0-60
    let val_to_y = |v: f64| -> f32 {
        let frac = (60.0 - v) / 60.0;
        rect.top() + frac as f32 * rect.height()
    };

    // Reference line at 25
    let y25 = val_to_y(25.0);
    painter.line_segment(
        [egui::pos2(rect.left(), y25), egui::pos2(rect.right(), y25)],
        egui::Stroke::new(
            0.5,
            egui::Color32::from_rgba_premultiplied(140, 140, 160, 60),
        ),
    );

    let sample_step = chart_render_sample_step(bars.len(), rect.width());
    for (series, color, width) in [
        (adx, ADX_COL, 1.5_f32),
        (di_plus, DI_PLUS_COL, 1.0),
        (di_minus, DI_MINUS_COL, 1.0),
    ] {
        let mut points: Vec<egui::Pos2> = Vec::with_capacity(bars.len() / sample_step + 1);
        for (rel_idx, _) in bars.iter().enumerate().step_by(sample_step) {
            if let Some(Some(v)) = series.get(start_idx + rel_idx) {
                points.push(egui::pos2(
                    rect.left() + (rel_idx as f32 + 0.5) * bar_w,
                    val_to_y(*v).clamp(rect.top(), rect.bottom()),
                ));
            }
        }
        if points.len() > 1 {
            painter.add(egui::Shape::line(points, egui::Stroke::new(width, color)));
        }
    }

    painter.text(
        egui::pos2(rect.left() + 4.0, rect.top() + 2.0),
        egui::Align2::LEFT_TOP,
        "ADX(14)",
        egui::FontId::monospace(9.0),
        egui::Color32::WHITE,
    );
}

/// Render decimation for dense chart views.
///
/// Keep at most ~2 samples per horizontal pixel. More than that is visually
/// indistinguishable but expensive for egui tessellation and GPU upload.
pub(super) fn chart_render_sample_step(len: usize, width_px: f32) -> usize {
    if len <= 1 {
        return 1;
    }
    let max_samples = ((width_px.max(1.0).ceil() as usize).saturating_mul(2)).max(1);
    if len <= max_samples {
        1
    } else {
        len.saturating_add(max_samples - 1) / max_samples
    }
}

/// Render a single indicator series as a polyline.
pub(super) fn draw_indicator_line(
    painter: &egui::Painter,
    chart_rect: egui::Rect,
    bars: &[Bar],
    series: &[Option<f64>],
    start_idx: usize,
    bar_w: f32,
    price_to_y: &dyn Fn(f64) -> f32,
    color: egui::Color32,
    width: f32,
) {
    let sample_step = chart_render_sample_step(bars.len(), chart_rect.width());
    let mut points: Vec<egui::Pos2> = Vec::with_capacity(bars.len() / sample_step + 1);
    let stroke = egui::Stroke::new(width, color);
    for (rel_idx, _bar) in bars.iter().enumerate().step_by(sample_step) {
        let abs_idx = start_idx + rel_idx;
        if abs_idx >= series.len() {
            continue;
        }
        if let Some(v) = series[abs_idx] {
            let x = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
            let y = price_to_y(v);
            if y >= chart_rect.top() && y <= chart_rect.bottom() {
                points.push(egui::pos2(x, y));
            } else if points.len() > 1 {
                painter.add(egui::Shape::line(std::mem::take(&mut points), stroke));
            } else {
                points.clear();
            }
        } else if points.len() > 1 {
            painter.add(egui::Shape::line(std::mem::take(&mut points), stroke));
        } else {
            points.clear();
        }
    }
    if points.len() > 1 {
        painter.add(egui::Shape::line(points, stroke));
    }
}

// ─── helpers ─────────────────────────────────────────────────────────────────

pub(super) fn parse_range(s: &str, default_lo: usize, default_hi: usize) -> (usize, usize) {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() == 2 {
        let lo = parts[0].trim().parse().unwrap_or(default_lo);
        let hi = parts[1].trim().parse().unwrap_or(default_hi);
        (lo, hi)
    } else {
        (default_lo, default_hi)
    }
}

pub(super) fn parse_range_f32(s: &str, default_lo: f64, default_hi: f64) -> (f64, f64) {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() == 2 {
        let lo = parts[0].trim().parse().unwrap_or(default_lo);
        let hi = parts[1].trim().parse().unwrap_or(default_hi);
        (lo, hi)
    } else {
        (default_lo, default_hi)
    }
}

pub(super) fn format_price(p: f64) -> String {
    if p == 0.0 {
        return "0".into();
    }
    let abs = p.abs();
    if abs >= 10_000.0 {
        format!("{:.2}", p)
    } else if abs >= 1.0 {
        format!("{:.4}", p)
    } else {
        format!("{:.6}", p)
    }
}

/// Buffer-reusing variant of format_price — writes into caller's String to avoid heap alloc per call.
pub(super) fn format_price_buf(p: f64, buf: &mut String) {
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

pub(super) fn format_ts(ts_ms: i64, tf: Timeframe) -> String {
    let mut buf = String::with_capacity(12);
    format_ts_buf(ts_ms, tf, &mut buf);
    buf
}

/// Buffer-reusing variant of format_ts — writes into caller's String to avoid heap alloc per call.
pub(super) fn format_ts_buf(ts_ms: i64, tf: Timeframe, buf: &mut String) {
    use chrono::{TimeZone, Timelike};
    buf.clear();
    let dt = chrono::Utc
        .timestamp_millis_opt(ts_ms)
        .single()
        .unwrap_or_default();
    use std::fmt::Write;
    match tf {
        Timeframe::MN1 => {
            write!(buf, "{}", dt.format("%b'%y")).ok();
        }
        Timeframe::W1 | Timeframe::D1 => {
            write!(buf, "{}", dt.format("%d %b")).ok();
        }
        Timeframe::H4 | Timeframe::H1 => {
            if dt.hour() == 0 {
                write!(buf, "{}", dt.format("%d %b")).ok();
            } else {
                write!(buf, "{}", dt.format("%H:%M")).ok();
            }
        }
        _ => {
            write!(buf, "{}", dt.format("%H:%M")).ok();
        }
    };
}

// ─── command palette ─────────────────────────────────────────────────────────
