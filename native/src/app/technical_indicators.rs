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
    pub(super) name: &'static str,
    pub(super) x: (usize, f64), // bar index, price
    pub(super) a: (usize, f64),
    pub(super) b: (usize, f64),
    pub(super) c: (usize, f64),
    pub(super) d: (usize, f64), // completion / entry point
    pub(super) tp1: f64,        // target 1 (0.382 AD)
    pub(super) tp2: f64,        // target 2 (0.618 AD)
    pub(super) sl: f64,         // stop loss (beyond X)
    pub(super) bullish: bool,
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

type HiLo = (Option<f64>, Option<f64>);

/// From accumulated per-period (hi, lo) groups, return the previous (second-to-
/// last, i.e. last *closed*) and current (last, i.e. *forming*) period high/low.
/// Each is `(None, None)` when that period is absent.
fn prev_and_current_group(groups: &[(f64, f64)]) -> (HiLo, HiLo) {
    let current = groups
        .last()
        .map_or((None, None), |g| (Some(g.0), Some(g.1)));
    let prev = if groups.len() >= 2 {
        let p = &groups[groups.len() - 2];
        (Some(p.0), Some(p.1))
    } else {
        (None, None)
    };
    (prev, current)
}

/// Group bars by a fixed period (ms) and return the previous (last closed) and
/// current (forming) period high/low — `((prev_hi, prev_lo), (cur_hi, cur_lo))`.
fn group_period_levels(bars: &[Bar], period_ms: i64) -> (HiLo, HiLo) {
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
    prev_and_current_group(&groups)
}

/// Monthly variant of [`group_period_levels`] — groups by calendar year-month.
fn group_month_levels(bars: &[Bar]) -> (HiLo, HiLo) {
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
    prev_and_current_group(&groups)
}

/// Previous candle levels for multiple timeframes — matches PreviousCandleLevels.mqh.
/// Returns (H1, H4, D1, W1, MN1) previous (last closed) candle high/low.
#[allow(clippy::type_complexity)]
pub(super) fn compute_prev_candle_levels(bars: &[Bar]) -> (HiLo, HiLo, HiLo, HiLo, HiLo) {
    if bars.len() < 2 {
        return ((None, None), (None, None), (None, None), (None, None), (None, None));
    }
    let h1 = group_period_levels(bars, 3_600_000).0; // 1 hour
    let h4 = group_period_levels(bars, 14_400_000).0; // 4 hours
    let d1 = group_period_levels(bars, 86_400_000).0; // 1 day
    let w1 = group_period_levels(bars, 7 * 86_400_000).0; // 1 week
    let mn1 = group_month_levels(bars).0;
    (h1, h4, d1, w1, mn1)
}

/// Current ("Judas") candle levels — the *forming* D1/W1/MN1 period high/low.
/// PreviousCandleLevels.mqh draws these (magenta) alongside the previous levels.
/// Returns (D1, W1, MN1) current candle high/low.
#[allow(clippy::type_complexity)]
pub(super) fn compute_current_candle_levels(bars: &[Bar]) -> (HiLo, HiLo, HiLo) {
    if bars.is_empty() {
        return ((None, None), (None, None), (None, None));
    }
    let d1 = group_period_levels(bars, 86_400_000).1; // 1 day
    let w1 = group_period_levels(bars, 7 * 86_400_000).1; // 1 week
    let mn1 = group_month_levels(bars).1;
    (d1, w1, mn1)
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
    pub(crate) const VAR_Z95: f64 = 1.644_853_626_951_472_2;
    pub(crate) const VAR_EPS: f64 = 1e-9;

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
    pub(crate) const FRACTAL_LOOKBACK: usize = 5;
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
    pub(crate) const BACK_LIMIT: usize = 1000;
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
    pub(crate) const FRACTAL_LOOKBACK: usize = 5;
    pub(crate) const BACK_LIMIT: usize = 1000;

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
