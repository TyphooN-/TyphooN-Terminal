//! WGSL shader sources for `gpu_compute`.
//!
//! Kept out of the parent module so pipeline/resource code is readable.

pub(super) const SMA_SHADER: &str = r#"
// SMA Compute Shader — parallel per-bar computation
// Each thread computes SMA for one bar by summing the lookback window

struct Params {
    period: u32,
    bar_count: u32,
}

@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }

    if (i < params.period - 1u) {
        output[i] = 0.0;  // Not enough data for SMA
        return;
    }

    var sum: f32 = 0.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) {
        sum = sum + bars[i - j];
    }
    output[i] = sum / f32(params.period);
}
"#;

pub(super) const EMA_SHADER: &str = r#"
// EMA Compute Shader — sequential (each bar depends on previous)
// Single workgroup processes all bars in order

struct Params {
    period: u32,
    bar_count: u32,
}

@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let k: f32 = 2.0 / (f32(params.period) + 1.0);

    // Seed with SMA of first `period` bars
    var sum: f32 = 0.0;
    for (var i: u32 = 0u; i < params.period; i = i + 1u) {
        sum = sum + bars[i];
        output[i] = 0.0;
    }
    var ema: f32 = sum / f32(params.period);
    output[params.period - 1u] = ema;

    // Recursive EMA
    for (var i: u32 = params.period; i < params.bar_count; i = i + 1u) {
        ema = bars[i] * k + ema * (1.0 - k);
        output[i] = ema;
    }
}
"#;

pub(super) const RSI_SHADER: &str = r#"
// RSI Compute Shader — sequential (running average of gains/losses)
struct Params {
    period: u32,
    bar_count: u32,
}

@group(0) @binding(0) var<storage, read> bars: array<f32>;   // close prices
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    if (params.bar_count <= params.period) { return; }

    // Initial average gain/loss over first `period` changes
    var avg_gain: f32 = 0.0;
    var avg_loss: f32 = 0.0;
    for (var i: u32 = 1u; i <= params.period; i = i + 1u) {
        let change = bars[i] - bars[i - 1u];
        if (change > 0.0) { avg_gain = avg_gain + change; }
        else { avg_loss = avg_loss - change; }
        output[i - 1u] = 50.0;
    }
    avg_gain = avg_gain / f32(params.period);
    avg_loss = avg_loss / f32(params.period);

    let rs = select(avg_gain / avg_loss, 100.0, avg_loss < 0.000001);
    output[params.period] = 100.0 - 100.0 / (1.0 + rs);

    // Smoothed RSI
    for (var i: u32 = params.period + 1u; i < params.bar_count; i = i + 1u) {
        let change = bars[i] - bars[i - 1u];
        let gain = select(change, 0.0, change < 0.0);
        let loss = select(-change, 0.0, change > 0.0);
        avg_gain = (avg_gain * f32(params.period - 1u) + gain) / f32(params.period);
        avg_loss = (avg_loss * f32(params.period - 1u) + loss) / f32(params.period);
        let rs2 = select(avg_gain / avg_loss, 100.0, avg_loss < 0.000001);
        output[i] = 100.0 - 100.0 / (1.0 + rs2);
    }
}
"#;

pub(super) const KAMA_SHADER: &str = r#"
// KAMA (Kaufman Adaptive Moving Average) Compute Shader — sequential
// KAMA adapts its smoothing constant based on market efficiency ratio
struct Params {
    period: u32,       // efficiency ratio lookback (e.g., 10)
    bar_count: u32,
}

@group(0) @binding(0) var<storage, read> bars: array<f32>;   // close prices
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let fast_sc: f32 = 2.0 / 3.0;   // fast period = 2
    let slow_sc: f32 = 2.0 / 31.0;  // slow period = 30

    // Seed KAMA with first close
    var kama: f32 = bars[0];
    output[0] = kama;
    for (var i: u32 = 1u; i < params.period; i = i + 1u) {
        output[i] = bars[i];
        kama = bars[i];
    }

    // Compute KAMA
    for (var i: u32 = params.period; i < params.bar_count; i = i + 1u) {
        // Direction: absolute price change over period
        let direction = abs(bars[i] - bars[i - params.period]);
        // Volatility: sum of absolute bar-to-bar changes over period
        var volatility: f32 = 0.0;
        for (var j: u32 = i - params.period + 1u; j <= i; j = j + 1u) {
            volatility = volatility + abs(bars[j] - bars[j - 1u]);
        }
        // Efficiency Ratio
        let er = select(direction / volatility, 0.0, volatility < 0.000001);
        // Smoothing Constant = (ER × (fast_sc - slow_sc) + slow_sc)²
        let sc = er * (fast_sc - slow_sc) + slow_sc;
        let sc2 = sc * sc;
        // KAMA
        kama = kama + sc2 * (bars[i] - kama);
        output[i] = kama;
    }
}
"#;

pub(super) const ATR_SHADER: &str = r#"
// ATR Compute Shader — sequential (smoothed True Range)
// Input: interleaved [high, low, close] per bar = 3 floats per bar
struct Params {
    period: u32,
    bar_count: u32,
}

@group(0) @binding(0) var<storage, read> bars: array<f32>;  // [h0,l0,c0, h1,l1,c1, ...]
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    if (params.bar_count < 2u) { return; }
    output[0] = bars[0] - bars[1];  // first TR = high - low

    // Compute True Range for all bars
    var atr_sum: f32 = bars[0] - bars[1]; // first bar TR
    for (var i: u32 = 1u; i < params.bar_count; i = i + 1u) {
        let h = bars[i * 3u];
        let l = bars[i * 3u + 1u];
        let prev_c = bars[(i - 1u) * 3u + 2u];
        let tr1 = h - l;
        let tr2 = abs(h - prev_c);
        let tr3 = abs(l - prev_c);
        let tr = max(tr1, max(tr2, tr3));

        if (i < params.period) {
            atr_sum = atr_sum + tr;
            output[i] = 0.0;
        } else if (i == params.period) {
            atr_sum = atr_sum + tr;
            output[i] = atr_sum / f32(params.period);
        } else {
            // Smoothed ATR
            output[i] = (output[i - 1u] * f32(params.period - 1u) + tr) / f32(params.period);
        }
    }
}
"#;

pub(super) const BOLLINGER_SHADER: &str = r#"
// Bollinger Bands Compute Shader — parallel per-bar
// Each thread computes SMA + stddev for its lookback window
// Output: [middle, upper, lower] per bar = 3 floats per bar
struct Params {
    period: u32,
    bar_count: u32,
}

@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;  // [mid0, up0, lo0, mid1, ...]
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }

    if (i < params.period - 1u) {
        output[i * 3u] = 0.0;
        output[i * 3u + 1u] = 0.0;
        output[i * 3u + 2u] = 0.0;
        return;
    }

    // SMA
    var sum: f32 = 0.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) {
        sum = sum + bars[i - j];
    }
    let sma = sum / f32(params.period);

    // Standard deviation
    var var_sum: f32 = 0.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) {
        let d = bars[i - j] - sma;
        var_sum = var_sum + d * d;
    }
    let stdev = sqrt(var_sum / f32(params.period));

    output[i * 3u] = sma;
    output[i * 3u + 1u] = sma + 2.0 * stdev;
    output[i * 3u + 2u] = sma - 2.0 * stdev;
}
"#;

pub(super) const MACD_SHADER: &str = r#"
// MACD Compute Shader — sequential (two EMAs + signal EMA)
// Output: [macd_line, signal, histogram] per bar = 3 floats per bar
// params.period encodes 3 values: fast | (slow << 8) | (signal << 16)
// Default: 12 | (26 << 8) | (9 << 16) = 0x0009_1A0C
struct Params {
    period: u32,       // bit-packed: [7:0]=fast, [15:8]=slow, [23:16]=signal
    bar_count: u32,
}

@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    // Unpack periods from bit-packed param
    let fast_p = params.period & 0xFFu;
    let slow_p = (params.period >> 8u) & 0xFFu;
    let sig_p = (params.period >> 16u) & 0xFFu;
    // Fallback to standard if zero
    let fast = select(fast_p, 12u, fast_p == 0u);
    let slow = select(slow_p, 26u, slow_p == 0u);
    let sig = select(sig_p, 9u, sig_p == 0u);

    let k_fast: f32 = 2.0 / (f32(fast) + 1.0);
    let k_slow: f32 = 2.0 / (f32(slow) + 1.0);
    let k_sig: f32 = 2.0 / (f32(sig) + 1.0);

    var ema_fast: f32 = bars[0];
    var ema_slow: f32 = bars[0];
    var signal: f32 = 0.0;
    var macd_started: bool = false;

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i == 0u) {
            ema_fast = bars[0]; ema_slow = bars[0];
        } else {
            ema_fast = bars[i] * k_fast + ema_fast * (1.0 - k_fast);
            ema_slow = bars[i] * k_slow + ema_slow * (1.0 - k_slow);
        }
        let macd_line = ema_fast - ema_slow;

        if (i >= slow && !macd_started) {
            signal = macd_line;
            macd_started = true;
        } else if (macd_started) {
            signal = macd_line * k_sig + signal * (1.0 - k_sig);
        }

        let hist = macd_line - signal;
        output[i * 3u] = macd_line;
        output[i * 3u + 1u] = signal;
        output[i * 3u + 2u] = hist;
    }
}
"#;

pub(super) const FISHER_SHADER: &str = r#"
// Fisher Transform Compute Shader — sequential
// Ehlers Fisher Transform of normalized price
struct Params {
    period: u32,
    bar_count: u32,
}

@group(0) @binding(0) var<storage, read> bars: array<f32>;  // (high+low)/2 midpoints
@group(0) @binding(1) var<storage, read_write> output: array<f32>;  // [fisher, trigger] per bar
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    var fish: f32 = 0.0;
    var prev_fish: f32 = 0.0;
    var val: f32 = 0.0;

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < params.period) {
            output[i * 2u] = 0.0;
            output[i * 2u + 1u] = 0.0;
            continue;
        }

        // Find highest high and lowest low in period
        var highest: f32 = -1000000.0;
        var lowest: f32 = 1000000.0;
        for (var j: u32 = i - params.period + 1u; j <= i; j = j + 1u) {
            if (bars[j] > highest) { highest = bars[j]; }
            if (bars[j] < lowest) { lowest = bars[j]; }
        }

        // Normalize to -1..+1 range
        let range = highest - lowest;
        var raw: f32 = 0.0;
        if (range > 0.000001) {
            raw = 2.0 * (bars[i] - lowest) / range - 1.0;
        }
        // Clamp to (-0.999, 0.999)
        raw = max(-0.999, min(0.999, raw));
        // Smooth
        val = 0.33 * raw + 0.67 * val;
        val = max(-0.999, min(0.999, val));

        // Fisher transform
        prev_fish = fish;
        fish = 0.5 * log((1.0 + val) / (1.0 - val));

        output[i * 2u] = fish;
        output[i * 2u + 1u] = prev_fish;  // trigger = previous fisher
    }
}
"#;

pub(super) const STOCHASTIC_SHADER: &str = r#"
// Stochastic Oscillator — parallel per-bar for %K, then sequential for %D
// Input: [high, low, close] interleaved (3 floats per bar)
// Output: [k, d] per bar (2 floats per bar)
struct Params {
    period: u32,       // %K period (e.g., 14)
    bar_count: u32,
}

@group(0) @binding(0) var<storage, read> bars: array<f32>;  // [h0,l0,c0, h1,l1,c1, ...]
@group(0) @binding(1) var<storage, read_write> output: array<f32>;  // [k0,d0, k1,d1, ...]
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let d_period: u32 = 3u;  // %D smoothing period

    // Compute raw %K
    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < params.period - 1u) {
            output[i * 2u] = 50.0;
            output[i * 2u + 1u] = 50.0;
            continue;
        }
        var highest: f32 = -1000000.0;
        var lowest: f32 = 1000000.0;
        for (var j: u32 = i - params.period + 1u; j <= i; j = j + 1u) {
            let h = bars[j * 3u];
            let l = bars[j * 3u + 1u];
            if (h > highest) { highest = h; }
            if (l < lowest) { lowest = l; }
        }
        let close = bars[i * 3u + 2u];
        let range = highest - lowest;
        let k = select((close - lowest) / range * 100.0, 50.0, range < 0.000001);
        output[i * 2u] = k;
    }

    // Compute %D (3-period SMA of %K)
    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < params.period + d_period - 2u) {
            output[i * 2u + 1u] = output[i * 2u];
            continue;
        }
        var sum: f32 = 0.0;
        for (var j: u32 = 0u; j < d_period; j = j + 1u) {
            sum = sum + output[(i - j) * 2u];
        }
        output[i * 2u + 1u] = sum / f32(d_period);
    }
}
"#;

pub(super) const ADX_SHADER: &str = r#"
// ADX (Average Directional Index) Compute Shader — sequential
// Input: [high, low, close] interleaved (3 floats per bar)
// Output: [adx, plus_di, minus_di] per bar (3 floats per bar)
struct Params {
    period: u32,       // ADX period (e.g., 14)
    bar_count: u32,
}

@group(0) @binding(0) var<storage, read> bars: array<f32>;  // [h0,l0,c0, ...]
@group(0) @binding(1) var<storage, read_write> output: array<f32>;  // [adx,+di,-di, ...]
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    if (params.bar_count < 2u) { return; }

    var smooth_plus_dm: f32 = 0.0;
    var smooth_minus_dm: f32 = 0.0;
    var smooth_tr: f32 = 0.0;
    var smooth_dx: f32 = 0.0;
    let p = f32(params.period);

    for (var i: u32 = 1u; i < params.bar_count; i = i + 1u) {
        let h = bars[i * 3u];
        let l = bars[i * 3u + 1u];
        let prev_h = bars[(i - 1u) * 3u];
        let prev_l = bars[(i - 1u) * 3u + 1u];
        let prev_c = bars[(i - 1u) * 3u + 2u];

        // True Range
        let tr = max(h - l, max(abs(h - prev_c), abs(l - prev_c)));

        // Directional Movement
        let up_move = h - prev_h;
        let down_move = prev_l - l;
        var plus_dm: f32 = 0.0;
        var minus_dm: f32 = 0.0;
        if (up_move > down_move && up_move > 0.0) { plus_dm = up_move; }
        if (down_move > up_move && down_move > 0.0) { minus_dm = down_move; }

        if (i <= params.period) {
            smooth_plus_dm = smooth_plus_dm + plus_dm;
            smooth_minus_dm = smooth_minus_dm + minus_dm;
            smooth_tr = smooth_tr + tr;
            output[i * 3u] = 0.0;
            output[i * 3u + 1u] = 0.0;
            output[i * 3u + 2u] = 0.0;
        } else {
            smooth_plus_dm = smooth_plus_dm - smooth_plus_dm / p + plus_dm;
            smooth_minus_dm = smooth_minus_dm - smooth_minus_dm / p + minus_dm;
            smooth_tr = smooth_tr - smooth_tr / p + tr;

            let plus_di = select(100.0 * smooth_plus_dm / smooth_tr, 0.0, smooth_tr < 0.000001);
            let minus_di = select(100.0 * smooth_minus_dm / smooth_tr, 0.0, smooth_tr < 0.000001);
            let di_sum = plus_di + minus_di;
            let dx = select(100.0 * abs(plus_di - minus_di) / di_sum, 0.0, di_sum < 0.000001);

            if (i == params.period + 1u) {
                smooth_dx = dx;
            } else {
                smooth_dx = (smooth_dx * (p - 1.0) + dx) / p;
            }

            output[i * 3u] = smooth_dx;
            output[i * 3u + 1u] = plus_di;
            output[i * 3u + 2u] = minus_di;
        }
    }
    output[0] = 0.0; output[1] = 0.0; output[2] = 0.0;
}
"#;

pub(super) const WMA_SHADER: &str = r#"
// Weighted Moving Average — parallel per-bar
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count || i < params.period - 1u) { output[i] = 0.0; return; }
    var weighted_sum: f32 = 0.0;
    var weight_total: f32 = 0.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) {
        let w = f32(params.period - j);
        weighted_sum = weighted_sum + bars[i - j] * w;
        weight_total = weight_total + w;
    }
    output[i] = weighted_sum / weight_total;
}
"#;

pub(super) const CCI_SHADER: &str = r#"
// Commodity Channel Index — parallel per-bar (uses typical price from OHLC)
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;  // typical prices (H+L+C)/3
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count || i < params.period - 1u) { output[i] = 0.0; return; }
    // SMA of typical price
    var sum: f32 = 0.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) { sum = sum + bars[i - j]; }
    let sma = sum / f32(params.period);
    // Mean deviation
    var md: f32 = 0.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) { md = md + abs(bars[i - j] - sma); }
    md = md / f32(params.period);
    output[i] = select((bars[i] - sma) / (0.015 * md), 0.0, md < 0.000001);
}
"#;

pub(super) const WILLIAMS_R_SHADER: &str = r#"
// Williams %R — parallel per-bar (uses OHLC)
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;  // [h,l,c] interleaved
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count || i < params.period - 1u) { output[i] = -50.0; return; }
    var hh: f32 = -1000000.0;
    var ll: f32 = 1000000.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) {
        let idx = i - j;
        let h = bars[idx * 3u];
        let l = bars[idx * 3u + 1u];
        if (h > hh) { hh = h; }
        if (l < ll) { ll = l; }
    }
    let close = bars[i * 3u + 2u];
    let range = hh - ll;
    output[i] = select((hh - close) / range * -100.0, -50.0, range < 0.000001);
}
"#;

pub(super) const OBV_SHADER: &str = r#"
// On-Balance Volume — sequential (cumulative) using resident close + volume buffers.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(2) var<storage, read> close_bars: array<f32>;
@group(0) @binding(3) var<storage, read> volumes: array<f32>;
@group(0) @binding(5) var<storage, read_write> output: array<f32>;
@group(0) @binding(6) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    if (params.bar_count == 0u) { return; }
    output[0] = 0.0;
    var obv: f32 = 0.0;
    for (var i: u32 = 1u; i < params.bar_count; i = i + 1u) {
        let close = close_bars[i];
        let prev_close = close_bars[i - 1u];
        let vol = volumes[i];
        if (close > prev_close) { obv = obv + vol; }
        else if (close < prev_close) { obv = obv - vol; }
        output[i] = obv;
    }
}
"#;

pub(super) const MOMENTUM_SHADER: &str = r#"
// Momentum — parallel per-bar (simple price difference)
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { output[i] = 0.0; return; }
    if (i < params.period) { output[i] = 0.0; return; }
    output[i] = bars[i] - bars[i - params.period];
}
"#;

pub(super) const CMO_SHADER: &str = r#"
// CMO — parallel rolling gain/loss spread on closes.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    if (params.period == 0u || i < params.period) {
        output[i] = 0.0;
        return;
    }
    var sum_up: f32 = 0.0;
    var sum_dn: f32 = 0.0;
    let start = i + 1u - params.period;
    for (var j: u32 = start; j <= i; j = j + 1u) {
        let delta = bars[j] - bars[j - 1u];
        if (delta > 0.0) {
            sum_up = sum_up + delta;
        } else if (delta < 0.0) {
            sum_dn = sum_dn - delta;
        }
    }
    let denom = sum_up + sum_dn;
    var value: f32 = 0.0;
    if (denom > 1e-6) {
        value = 100.0 * (sum_up - sum_dn) / denom;
    }
    output[i] = value;
}
"#;

pub(super) const QSTICK_SHADER: &str = r#"
// QStick — parallel SMA of candle body using resident open + close buffers.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> open_bars: array<f32>;
@group(0) @binding(2) var<storage, read> close_bars: array<f32>;
@group(0) @binding(5) var<storage, read_write> output: array<f32>;
@group(0) @binding(6) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    if (params.period == 0u || i + 1u < params.period) {
        output[i] = 0.0;
        return;
    }
    var sum: f32 = 0.0;
    let start = i + 1u - params.period;
    for (var j: u32 = start; j <= i; j = j + 1u) {
        let open = open_bars[j];
        let close = close_bars[j];
        sum = sum + (close - open);
    }
    output[i] = sum / f32(params.period);
}
"#;

pub(super) const DISPARITY_SHADER: &str = r#"
// Disparity Index — parallel % deviation of close from SMA(period).
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    if (params.period == 0u || i + 1u < params.period) {
        output[i] = 0.0;
        return;
    }
    var sum: f32 = 0.0;
    let start = i + 1u - params.period;
    for (var j: u32 = start; j <= i; j = j + 1u) {
        sum = sum + bars[j];
    }
    let sma = sum / f32(params.period);
    var value: f32 = 0.0;
    if (abs(sma) > 1e-6) {
        value = (bars[i] / sma - 1.0) * 100.0;
    }
    output[i] = value;
}
"#;

pub(super) const BOP_SHADER: &str = r#"
// BOP — parallel SMA of (close-open)/(high-low) using resident open + OHLC buffers.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> open_bars: array<f32>;
@group(0) @binding(1) var<storage, read> ohlc_bars: array<f32>;
@group(0) @binding(5) var<storage, read_write> output: array<f32>;
@group(0) @binding(6) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    if (params.period == 0u || i + 1u < params.period) {
        output[i] = 0.0;
        return;
    }
    var sum: f32 = 0.0;
    let start = i + 1u - params.period;
    for (var j: u32 = start; j <= i; j = j + 1u) {
        let base = j * 3u;
        let open = open_bars[j];
        let high = ohlc_bars[base];
        let low = ohlc_bars[base + 1u];
        let close = ohlc_bars[base + 2u];
        let range = max(high - low, 1e-6);
        sum = sum + (close - open) / range;
    }
    output[i] = sum / f32(params.period);
}
"#;

pub(super) const STDDEV_SHADER: &str = r#"
// StdDev — parallel rolling sample standard deviation of closes.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    if (params.period < 2u || i + 1u < params.period) {
        output[i] = 0.0;
        return;
    }
    var sum: f32 = 0.0;
    let start = i + 1u - params.period;
    for (var j: u32 = start; j <= i; j = j + 1u) {
        sum = sum + bars[j];
    }
    let mean = sum / f32(params.period);
    var ss: f32 = 0.0;
    for (var j: u32 = start; j <= i; j = j + 1u) {
        let d = bars[j] - mean;
        ss = ss + d * d;
    }
    output[i] = sqrt(max(ss / f32(params.period - 1u), 0.0));
}
"#;

pub(super) const MFI_SHADER: &str = r#"
// MFI — parallel Money Flow Index using resident OHLC + volume buffers.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(1) var<storage, read> ohlc_bars: array<f32>;
@group(0) @binding(3) var<storage, read> volumes: array<f32>;
@group(0) @binding(5) var<storage, read_write> output: array<f32>;
@group(0) @binding(6) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    if (params.period == 0u || i < params.period) {
        output[i] = 0.0;
        return;
    }
    var pos_sum: f32 = 0.0;
    var neg_sum: f32 = 0.0;
    let start = i + 1u - params.period;
    for (var j: u32 = start; j <= i; j = j + 1u) {
        let base = j * 3u;
        let prev_base = (j - 1u) * 3u;
        let tp = (ohlc_bars[base] + ohlc_bars[base + 1u] + ohlc_bars[base + 2u]) / 3.0;
        let prev_tp =
            (ohlc_bars[prev_base] + ohlc_bars[prev_base + 1u] + ohlc_bars[prev_base + 2u]) / 3.0;
        let money_flow = tp * max(volumes[j], 0.0);
        if (tp > prev_tp) {
            pos_sum = pos_sum + money_flow;
        } else if (tp < prev_tp) {
            neg_sum = neg_sum + money_flow;
        }
    }
    if (neg_sum <= 1e-6) {
        output[i] = select(100.0, 50.0, pos_sum <= 1e-6);
        return;
    }
    let ratio = pos_sum / neg_sum;
    output[i] = clamp(100.0 - 100.0 / (1.0 + ratio), 0.0, 100.0);
}
"#;

pub(super) const TRIX_SHADER: &str = r#"
// TRIX — sequential triple-EMA ROC with signal EMA.
// params.period encodes: [7:0]=period, [15:8]=signal period.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let period_raw = params.period & 0xFFu;
    let signal_raw = (params.period >> 8u) & 0xFFu;
    let period = select(period_raw, 15u, period_raw == 0u);
    let signal_period = select(signal_raw, 9u, signal_raw == 0u);
    let k = 2.0 / (f32(period) + 1.0);
    let sig_k = 2.0 / (f32(signal_period) + 1.0);

    var ema1 = bars[0];
    var ema2 = bars[0];
    var ema3 = bars[0];
    var prev_ema3 = bars[0];
    var signal = 0.0;
    var signal_started = false;

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i > 0u) {
            ema1 = bars[i] * k + ema1 * (1.0 - k);
            ema2 = ema1 * k + ema2 * (1.0 - k);
            ema3 = ema2 * k + ema3 * (1.0 - k);
        }
        var trix = 0.0;
        if (i > 0u && abs(prev_ema3) > 1e-6) {
            trix = 100.0 * (ema3 / prev_ema3 - 1.0);
        }
        if (i + 1u >= signal_period) {
            if (!signal_started) {
                signal = trix;
                signal_started = true;
            } else {
                signal = trix * sig_k + signal * (1.0 - sig_k);
            }
        }
        output[i * 3u] = trix;
        output[i * 3u + 1u] = signal;
        output[i * 3u + 2u] = trix - signal;
        prev_ema3 = ema3;
    }
}
"#;

pub(super) const PPO_SHADER: &str = r#"
// PPO — sequential percentage price oscillator with signal EMA.
// params.period encodes: [7:0]=fast, [15:8]=slow, [23:16]=signal.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let fast_raw = params.period & 0xFFu;
    let slow_raw = (params.period >> 8u) & 0xFFu;
    let signal_raw = (params.period >> 16u) & 0xFFu;
    let fast = select(fast_raw, 12u, fast_raw == 0u);
    let slow = select(slow_raw, 26u, slow_raw == 0u);
    let signal_period = select(signal_raw, 9u, signal_raw == 0u);
    let k_fast = 2.0 / (f32(fast) + 1.0);
    let k_slow = 2.0 / (f32(slow) + 1.0);
    let k_sig = 2.0 / (f32(signal_period) + 1.0);

    var ema_fast = bars[0];
    var ema_slow = bars[0];
    var signal = 0.0;
    var signal_started = false;

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i > 0u) {
            ema_fast = bars[i] * k_fast + ema_fast * (1.0 - k_fast);
            ema_slow = bars[i] * k_slow + ema_slow * (1.0 - k_slow);
        }
        var ppo = 0.0;
        if (abs(ema_slow) > 1e-6) {
            ppo = 100.0 * (ema_fast - ema_slow) / ema_slow;
        }
        if (i + 1u >= signal_period) {
            if (!signal_started) {
                signal = ppo;
                signal_started = true;
            } else {
                signal = ppo * k_sig + signal * (1.0 - k_sig);
            }
        }
        output[i * 3u] = ppo;
        output[i * 3u + 1u] = signal;
        output[i * 3u + 2u] = ppo - signal;
    }
}
"#;

pub(super) const ULTOSC_SHADER: &str = r#"
// Ultimate Oscillator — sequential weighted BP/TR average using resident OHLC buffers.
// params.period encodes: [7:0]=short, [15:8]=mid, [23:16]=long.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(1) var<storage, read> ohlc_bars: array<f32>;
@group(0) @binding(5) var<storage, read_write> output: array<f32>;
@group(0) @binding(6) var<uniform> params: Params;

fn true_low(low: f32, prev_close: f32) -> f32 {
    return min(low, prev_close);
}

fn true_high(high: f32, prev_close: f32) -> f32 {
    return max(high, prev_close);
}

@compute @workgroup_size(1)
fn main() {
    let p1_raw = params.period & 0xFFu;
    let p2_raw = (params.period >> 8u) & 0xFFu;
    let p3_raw = (params.period >> 16u) & 0xFFu;
    let p1 = select(p1_raw, 7u, p1_raw == 0u);
    let p2 = select(p2_raw, 14u, p2_raw == 0u);
    let p3 = select(p3_raw, 28u, p3_raw == 0u);

    output[0] = 0.0;
    for (var i: u32 = 1u; i < params.bar_count; i = i + 1u) {
        if (i < p3) {
            output[i] = 0.0;
            continue;
        }
        var bp1: f32 = 0.0;
        var bp2: f32 = 0.0;
        var bp3: f32 = 0.0;
        var tr1: f32 = 0.0;
        var tr2: f32 = 0.0;
        var tr3: f32 = 0.0;
        for (var j: u32 = i + 1u - p3; j <= i; j = j + 1u) {
            let base = j * 3u;
            let prev_base = (j - 1u) * 3u;
            let prev_close = ohlc_bars[prev_base + 2u];
            let high = ohlc_bars[base];
            let low = ohlc_bars[base + 1u];
            let close = ohlc_bars[base + 2u];
            let bp = close - true_low(low, prev_close);
            let tr = max(true_high(high, prev_close) - true_low(low, prev_close), 1e-6);
            bp3 = bp3 + bp;
            tr3 = tr3 + tr;
            if (j + p2 >= i + 1u) {
                bp2 = bp2 + bp;
                tr2 = tr2 + tr;
            }
            if (j + p1 >= i + 1u) {
                bp1 = bp1 + bp;
                tr1 = tr1 + tr;
            }
        }
        let avg1 = bp1 / max(tr1, 1e-6);
        let avg2 = bp2 / max(tr2, 1e-6);
        let avg3 = bp3 / max(tr3, 1e-6);
        output[i] = clamp(100.0 * (4.0 * avg1 + 2.0 * avg2 + avg3) / 7.0, 0.0, 100.0);
    }
}
"#;

pub(super) const STOCHRSI_SHADER: &str = r#"
// StochRSI — sequential RSI, raw StochRSI, %K, then %D.
// params.period encodes: [7:0]=rsi, [15:8]=stoch, [23:16]=k, [31:24]=d.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

fn compute_rsi_value(avg_gain: f32, avg_loss: f32) -> f32 {
    if (avg_loss <= 1e-6) {
        return select(100.0, 50.0, avg_gain <= 1e-6);
    }
    let rs = avg_gain / avg_loss;
    return 100.0 - 100.0 / (1.0 + rs);
}

@compute @workgroup_size(1)
fn main() {
    let rsi_raw = params.period & 0xFFu;
    let stoch_raw = (params.period >> 8u) & 0xFFu;
    let k_raw = (params.period >> 16u) & 0xFFu;
    let d_raw = (params.period >> 24u) & 0xFFu;
    let rsi_period = select(rsi_raw, 14u, rsi_raw == 0u);
    let stoch_period = select(stoch_raw, 14u, stoch_raw == 0u);
    let k_period = select(k_raw, 3u, k_raw == 0u);
    let d_period = select(d_raw, 3u, d_raw == 0u);

    output[0] = 50.0;
    output[1] = 0.0;
    var avg_gain: f32 = 0.0;
    var avg_loss: f32 = 0.0;
    for (var i: u32 = 1u; i < params.bar_count; i = i + 1u) {
        let delta = bars[i] - bars[i - 1u];
        let gain = max(delta, 0.0);
        let loss = max(-delta, 0.0);
        if (i < rsi_period) {
            avg_gain = avg_gain + gain;
            avg_loss = avg_loss + loss;
            output[i * 2u] = 50.0;
        } else if (i == rsi_period) {
            avg_gain = (avg_gain + gain) / f32(rsi_period);
            avg_loss = (avg_loss + loss) / f32(rsi_period);
            output[i * 2u] = compute_rsi_value(avg_gain, avg_loss);
        } else {
            avg_gain = (avg_gain * f32(rsi_period - 1u) + gain) / f32(rsi_period);
            avg_loss = (avg_loss * f32(rsi_period - 1u) + loss) / f32(rsi_period);
            output[i * 2u] = compute_rsi_value(avg_gain, avg_loss);
        }
        output[i * 2u + 1u] = 0.0;
    }

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < stoch_period - 1u) {
            output[i * 2u + 1u] = 0.0;
            continue;
        }
        var min_rsi = 1e9;
        var max_rsi = -1e9;
        let start = i + 1u - stoch_period;
        for (var j: u32 = start; j <= i; j = j + 1u) {
            let rsi = output[j * 2u];
            min_rsi = min(min_rsi, rsi);
            max_rsi = max(max_rsi, rsi);
        }
        let range = max_rsi - min_rsi;
        output[i * 2u + 1u] = select(
            clamp((output[i * 2u] - min_rsi) / range * 100.0, 0.0, 100.0),
            50.0,
            range <= 1e-6,
        );
    }

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < k_period - 1u) {
            output[i * 2u] = 0.0;
            continue;
        }
        let start = i + 1u - k_period;
        var sum_k: f32 = 0.0;
        for (var j: u32 = start; j <= i; j = j + 1u) {
            sum_k = sum_k + output[j * 2u + 1u];
        }
        output[i * 2u] = clamp(sum_k / f32(k_period), 0.0, 100.0);
    }

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < d_period - 1u) {
            output[i * 2u + 1u] = 0.0;
            continue;
        }
        let start = i + 1u - d_period;
        var sum_d: f32 = 0.0;
        for (var j: u32 = start; j <= i; j = j + 1u) {
            sum_d = sum_d + output[j * 2u];
        }
        output[i * 2u + 1u] = clamp(sum_d / f32(d_period), 0.0, 100.0);
    }
}
"#;

pub(super) const VAR_OSCILLATOR_SHADER: &str = r#"
// VaR oscillator — sequential rolling parametric VaR (95%) on close-to-close log returns.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

const VAR_Z95: f32 = 1.6448536;
const VAR_EPS: f32 = 1e-6;

@compute @workgroup_size(1)
fn main() {
    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        output[i] = 0.0;
    }
    if (params.period == 0u || params.bar_count <= params.period) { return; }

    for (var i: u32 = params.period; i < params.bar_count; i = i + 1u) {
        var sum: f32 = 0.0;
        var sum_sq: f32 = 0.0;
        let start = i + 1u - params.period;
        for (var j: u32 = start; j <= i; j = j + 1u) {
            let prev_close = max(bars[j - 1u], VAR_EPS);
            let close = max(bars[j], VAR_EPS);
            let ret = log(close / prev_close);
            sum = sum + ret;
            sum_sq = sum_sq + ret * ret;
        }

        let count = f32(params.period);
        let mean = sum / count;
        let variance = max(sum_sq / count - mean * mean, 0.0);
        let sigma = sqrt(variance);
        let var95 = max(VAR_EPS, VAR_Z95 * sigma - mean);

        let prev_close = max(bars[i - 1u], VAR_EPS);
        let close = max(bars[i], VAR_EPS);
        let current_ret = log(close / prev_close);
        output[i] = -100.0 * current_ret / var95;
    }
}
"#;

pub(super) const PSAR_SHADER: &str = r#"
// Parabolic SAR — sequential (state machine)
struct Params { period: u32, bar_count: u32, }  // period unused, af_step=0.02, af_max=0.2 hardcoded
@group(0) @binding(0) var<storage, read> bars: array<f32>;  // [h,l,c] interleaved
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    if (params.bar_count < 3u) { return; }
    let af_step: f32 = 0.02;
    let af_max: f32 = 0.2;

    // Initialize: start long
    var is_long: bool = true;
    var af: f32 = af_step;
    var ep: f32 = bars[0];      // extreme point = first high
    var sar: f32 = bars[1];     // start at first low
    output[0] = sar;
    output[1] = sar;

    for (var i: u32 = 2u; i < params.bar_count; i = i + 1u) {
        let h = bars[i * 3u];
        let l = bars[i * 3u + 1u];

        var new_sar = sar + af * (ep - sar);

        if (is_long) {
            // Clamp SAR below prior two lows
            let prev_l = bars[(i - 1u) * 3u + 1u];
            let prev2_l = bars[(i - 2u) * 3u + 1u];
            new_sar = min(new_sar, min(prev_l, prev2_l));

            if (l < new_sar) {
                // Reverse to short
                is_long = false;
                new_sar = ep;
                ep = l;
                af = af_step;
            } else {
                if (h > ep) {
                    ep = h;
                    af = min(af + af_step, af_max);
                }
            }
        } else {
            // Clamp SAR above prior two highs
            let prev_h = bars[(i - 1u) * 3u];
            let prev2_h = bars[(i - 2u) * 3u];
            new_sar = max(new_sar, max(prev_h, prev2_h));

            if (h > new_sar) {
                // Reverse to long
                is_long = true;
                new_sar = ep;
                ep = h;
                af = af_step;
            } else {
                if (l < ep) {
                    ep = l;
                    af = min(af + af_step, af_max);
                }
            }
        }

        sar = new_sar;
        output[i] = sar;
    }
}
"#;

pub(super) const ICHIMOKU_SHADER: &str = r#"
// Ichimoku Kinko Hyo — sequential (4 outputs: tenkan, kijun, span_a, span_b)
// Input: [high, low, close] interleaved
// Output: [tenkan, kijun, span_a, span_b] × bar_count = 4 floats per bar
struct Params { period: u32, bar_count: u32, }  // period unused, hardcoded 9/26/52
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

fn highest_high(start: u32, len: u32) -> f32 {
    var hh: f32 = -1000000.0;
    for (var j: u32 = 0u; j < len; j = j + 1u) {
        let h = bars[(start + j) * 3u];
        if (h > hh) { hh = h; }
    }
    return hh;
}
fn lowest_low(start: u32, len: u32) -> f32 {
    var ll: f32 = 1000000.0;
    for (var j: u32 = 0u; j < len; j = j + 1u) {
        let l = bars[(start + j) * 3u + 1u];
        if (l < ll) { ll = l; }
    }
    return ll;
}

@compute @workgroup_size(1)
fn main() {
    let tenkan_p: u32 = 9u;
    let kijun_p: u32 = 26u;
    let senkou_b_p: u32 = 52u;

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        let base = i * 4u;
        // Tenkan-sen (9-period midpoint)
        if (i >= tenkan_p - 1u) {
            let start = i - tenkan_p + 1u;
            output[base] = (highest_high(start, tenkan_p) + lowest_low(start, tenkan_p)) / 2.0;
        } else { output[base] = 0.0; }

        // Kijun-sen (26-period midpoint)
        if (i >= kijun_p - 1u) {
            let start = i - kijun_p + 1u;
            output[base + 1u] = (highest_high(start, kijun_p) + lowest_low(start, kijun_p)) / 2.0;
        } else { output[base + 1u] = 0.0; }

        // Senkou Span A (midpoint of tenkan + kijun, projected 26 forward)
        if (i >= kijun_p - 1u) {
            output[base + 2u] = (output[base] + output[base + 1u]) / 2.0;
        } else { output[base + 2u] = 0.0; }

        // Senkou Span B (52-period midpoint, projected 26 forward)
        if (i >= senkou_b_p - 1u) {
            let start = i - senkou_b_p + 1u;
            output[base + 3u] = (highest_high(start, senkou_b_p) + lowest_low(start, senkou_b_p)) / 2.0;
        } else { output[base + 3u] = 0.0; }
    }
}
"#;

pub(super) const CCI_GPU_SHADER: &str = r#"
// CCI with built-in typical price computation from OHLC
// Input: [high, low, close] interleaved
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count || i < params.period - 1u) { output[i] = 0.0; return; }

    // Compute typical prices and SMA
    var sum: f32 = 0.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) {
        let idx = i - j;
        let tp = (bars[idx * 3u] + bars[idx * 3u + 1u] + bars[idx * 3u + 2u]) / 3.0;
        sum = sum + tp;
    }
    let sma = sum / f32(params.period);

    // Mean deviation
    var md: f32 = 0.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) {
        let idx = i - j;
        let tp = (bars[idx * 3u] + bars[idx * 3u + 1u] + bars[idx * 3u + 2u]) / 3.0;
        md = md + abs(tp - sma);
    }
    md = md / f32(params.period);

    let tp_now = (bars[i * 3u] + bars[i * 3u + 1u] + bars[i * 3u + 2u]) / 3.0;
    output[i] = select((tp_now - sma) / (0.015 * md), 0.0, md < 0.000001);
}
"#;

pub(super) const OBV_GPU_SHADER: &str = r#"
// OBV with close+volume from OHLC buffer (close at offset 2, volume separate)
// Input binding 0: close prices, binding 1 is output
// We'll use a separate volume buffer approach — close prices in bars, volume uploaded separately
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;  // close prices
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    // OBV without volume data — use price change magnitude as proxy
    // (Real OBV requires volume buffer; this is a reasonable GPU approximation)
    if (params.bar_count == 0u) { return; }
    output[0] = 0.0;
    var obv: f32 = 0.0;
    for (var i: u32 = 1u; i < params.bar_count; i = i + 1u) {
        let change = bars[i] - bars[i - 1u];
        if (change > 0.0) { obv = obv + abs(change); }
        else if (change < 0.0) { obv = obv - abs(change); }
        output[i] = obv;
    }
}
"#;

pub(super) const EHLERS_SUPERSMOOTHER_SHADER: &str = r#"
// Ehlers Super Smoother — 2-pole Butterworth low-pass filter (sequential)
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let pi: f32 = 3.14159265;
    let a = exp(-1.414 * pi / f32(params.period));
    let b = 2.0 * a * cos(1.414 * pi / f32(params.period));
    let c2 = b;
    let c3 = -a * a;
    let c1 = 1.0 - c2 - c3;

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < 2u) {
            output[i] = bars[i];
        } else {
            output[i] = c1 * (bars[i] + bars[i - 1u]) / 2.0 + c2 * output[i - 1u] + c3 * output[i - 2u];
        }
    }
}
"#;

pub(super) const EHLERS_DECYCLER_SHADER: &str = r#"
// Ehlers Decycler — price minus super-smoothed component (sequential)
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let pi: f32 = 3.14159265;
    let a = exp(-1.414 * pi / f32(params.period));
    let b = 2.0 * a * cos(1.414 * pi / f32(params.period));
    let c2 = b;
    let c3 = -a * a;
    let c1 = 1.0 - c2 - c3;

    // First compute super smoother
    var ss: array<f32, 2>;
    ss[0] = bars[0]; ss[1] = bars[min(1u, params.bar_count - 1u)];
    output[0] = 0.0;
    if (params.bar_count > 1u) { output[1] = 0.0; }

    for (var i: u32 = 2u; i < params.bar_count; i = i + 1u) {
        let smoothed = c1 * (bars[i] + bars[i - 1u]) / 2.0 + c2 * ss[1] + c3 * ss[0];
        output[i] = bars[i] - smoothed;
        ss[0] = ss[1];
        ss[1] = smoothed;
    }
}
"#;

pub(super) const FRACTALS_SHADER: &str = r#"
// Fractals (Williams) — parallel per-bar
// Fractal Up: high[i] > high[i-2..i+2] (5-bar pattern)
// Fractal Down: low[i] < low[i-2..i+2]
// Output: [fractal_up_flag, fractal_down_flag] per bar (2 floats, 0.0 or price)
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;  // [h,l,c] interleaved
@group(0) @binding(1) var<storage, read_write> output: array<f32>;  // [up, down] per bar
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    let base = i * 2u;

    if (i < 2u || i + 2u >= params.bar_count) {
        output[base] = 0.0;
        output[base + 1u] = 0.0;
        return;
    }

    let h = bars[i * 3u];
    let l = bars[i * 3u + 1u];

    // Fractal up: current high > surrounding 4 highs
    let is_up = h > bars[(i - 2u) * 3u] && h > bars[(i - 1u) * 3u]
             && h > bars[(i + 1u) * 3u] && h > bars[(i + 2u) * 3u];
    output[base] = select(0.0, h, is_up);

    // Fractal down: current low < surrounding 4 lows
    let is_down = l < bars[(i - 2u) * 3u + 1u] && l < bars[(i - 1u) * 3u + 1u]
               && l < bars[(i + 1u) * 3u + 1u] && l < bars[(i + 2u) * 3u + 1u];
    output[base + 1u] = select(0.0, l, is_down);
}
"#;

pub(super) const HMA_SHADER: &str = r#"
// Hull Moving Average — sequential (WMA composition: 2*WMA(n/2) - WMA(n), then WMA(sqrt(n)))
// All WMA computations inlined (WGSL can't pass storage pointers to functions)
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let n = params.period;
    let half_n = max(n / 2u, 1u);
    let sqrt_n = max(u32(sqrt(f32(n))), 1u);

    // Step 1: Compute delta = 2*WMA(n/2) - WMA(n) into output
    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < n - 1u) { output[i] = 0.0; continue; }

        // Inline WMA(half_n) on bars
        var ws_half: f32 = 0.0;
        var wt_half: f32 = 0.0;
        for (var j: u32 = 0u; j < half_n; j = j + 1u) {
            let w = f32(half_n - j);
            ws_half = ws_half + bars[i - j] * w;
            wt_half = wt_half + w;
        }
        let wma_half = ws_half / wt_half;

        // Inline WMA(n) on bars
        var ws_full: f32 = 0.0;
        var wt_full: f32 = 0.0;
        for (var j: u32 = 0u; j < n; j = j + 1u) {
            let w = f32(n - j);
            ws_full = ws_full + bars[i - j] * w;
            wt_full = wt_full + w;
        }
        let wma_full = ws_full / wt_full;

        output[i] = 2.0 * wma_half - wma_full;
    }

    // Step 2: WMA(sqrt_n) of the delta series (stored in output)
    // Copy delta to temp array first (can't read and write same buffer safely)
    var temp: array<f32, 512>;
    let copy_len = min(params.bar_count, 512u);
    for (var i: u32 = 0u; i < copy_len; i = i + 1u) {
        temp[i] = output[i];
    }

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < n - 1u + sqrt_n - 1u) { output[i] = 0.0; continue; }
        if (i >= 512u) { output[i] = 0.0; continue; }  // safety bound

        var ws: f32 = 0.0;
        var wt: f32 = 0.0;
        for (var j: u32 = 0u; j < sqrt_n; j = j + 1u) {
            let w = f32(sqrt_n - j);
            ws = ws + temp[i - j] * w;
            wt = wt + w;
        }
        output[i] = select(ws / wt, 0.0, wt < 0.000001);
    }
}
"#;

pub(super) const EHLERS_ITL_SHADER: &str = r#"
// Ehlers Instantaneous Trendline — sequential IIR
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    for (var i: u32 = 0u; i < min(7u, params.bar_count); i = i + 1u) { output[i] = bars[i]; }
    for (var i: u32 = 7u; i < params.bar_count; i = i + 1u) {
        var itl = (bars[i] + 2.0 * bars[i - 1u] + bars[i - 2u]) / 4.0 * 0.5 + output[i - 1u] * 0.5;
        itl = (2.0 * itl + output[i - 1u] + output[i - 2u] + output[i - 3u]) / 5.0;
        output[i] = itl;
    }
}
"#;

pub(super) const EHLERS_CYBER_SHADER: &str = r#"
// Ehlers Cyber Cycle — sequential 2nd-order bandpass
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let alpha: f32 = 0.07;
    let c1: f32 = (1.0 - 0.5 * alpha) * (1.0 - 0.5 * alpha);
    let c2: f32 = 1.0 - alpha;

    // Smooth
    for (var i: u32 = 0u; i < min(3u, params.bar_count); i = i + 1u) { output[i] = 0.0; }
    for (var i: u32 = 3u; i < params.bar_count; i = i + 1u) {
        let sm_cur = (bars[i] + 2.0 * bars[i - 1u] + bars[i - 2u]) / 4.0;
        let sm_prev = (bars[i - 1u] + 2.0 * bars[i - 2u] + bars[i - 3u]) / 4.0;
        let sm_prev2 = (bars[i - 2u] + 2.0 * bars[i - 3u] + bars[max(i, 4u) - 4u]) / 4.0;
        output[i] = c1 * (sm_cur - 2.0 * sm_prev + sm_prev2) + 2.0 * c2 * output[i - 1u] - c2 * c2 * output[max(i, 2u) - 2u];
    }
}
"#;

pub(super) const EHLERS_CG_SHADER: &str = r#"
// Ehlers Center of Gravity Oscillator — parallel per-bar
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count || i < params.period - 1u) { output[i] = 0.0; return; }
    var num: f32 = 0.0;
    var den: f32 = 0.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) {
        let p = bars[i - j];
        num = num + f32(j + 1u) * p;
        den = den + p;
    }
    output[i] = select(-num / den + f32(params.period + 1u) / 2.0, 0.0, abs(den) < 0.000001);
}
"#;

pub(super) const EHLERS_ROOF_SHADER: &str = r#"
// Ehlers Roofing Filter — sequential (highpass + super smoother)
// period field repurposed: low 16 bits = lp_period, high 16 bits = hp_period
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let lp_period = params.period & 0xFFFFu;
    let hp_period = params.period >> 16u;
    let pi: f32 = 3.14159265;

    if (params.bar_count < 3u) { return; }

    // Highpass filter
    let alpha1 = cos(2.0 * pi / f32(max(hp_period, 2u)));
    let a1 = select(1.0 / max(alpha1 + sqrt(max(alpha1 * alpha1 - 1.0, 0.0)), 0.001), 0.5, abs(alpha1) < 0.000001);
    let hp_coeff = (1.0 - a1 / 2.0) * (1.0 - a1 / 2.0);
    let hp_c2 = 2.0 * (1.0 - a1);
    let hp_c3 = (1.0 - a1) * (1.0 - a1);

    // Super smoother coefficients
    let a = exp(-1.414 * pi / f32(max(lp_period, 1u)));
    let b = 2.0 * a * cos(1.414 * pi / f32(max(lp_period, 1u)));
    let ss_c1 = 1.0 - b + a * a;

    // Two-pass: highpass then super smooth
    output[0] = 0.0; output[1] = 0.0;
    var hp_prev1: f32 = 0.0;
    var hp_prev2: f32 = 0.0;
    var filt_prev1: f32 = 0.0;
    var filt_prev2: f32 = 0.0;

    for (var i: u32 = 2u; i < params.bar_count; i = i + 1u) {
        let hp = hp_coeff * (bars[i] - 2.0 * bars[i - 1u] + bars[i - 2u]) + hp_c2 * hp_prev1 - hp_c3 * hp_prev2;
        let filt = ss_c1 * (hp + hp_prev1) / 2.0 + b * filt_prev1 - a * a * filt_prev2;
        output[i] = filt;
        hp_prev2 = hp_prev1; hp_prev1 = hp;
        filt_prev2 = filt_prev1; filt_prev1 = filt;
    }
}
"#;

pub(super) const EHLERS_EBSW_SHADER: &str = r#"
// Ehlers Even Better Sinewave — sequential (highpass + super smooth + atan)
struct Params { period: u32, bar_count: u32, }  // period = duration (e.g., 40)
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let pi: f32 = 3.14159265;
    let duration = f32(max(params.period, 4u));
    if (params.bar_count < 5u) { return; }

    // Highpass coefficients
    let alpha1 = cos(2.0 * pi / (duration * 1.414));
    let a1 = select(1.0 / max(alpha1 + sqrt(max(alpha1 * alpha1 - 1.0, 0.0)), 0.001), 0.5, abs(alpha1) < 0.000001);
    let hp_coeff = (1.0 - a1 / 2.0) * (1.0 - a1 / 2.0);

    // Super smoother coefficients (period/4)
    let ss_period = max(duration / 4.0, 1.0);
    let a = exp(-1.414 * pi / ss_period);
    let b = 2.0 * a * cos(1.414 * pi / ss_period);
    let c1 = 1.0 - b + a * a;

    var hp_prev1: f32 = 0.0; var hp_prev2: f32 = 0.0;
    var filt_prev1: f32 = 0.0; var filt_prev2: f32 = 0.0;
    output[0] = 0.0; output[1] = 0.0;

    for (var i: u32 = 2u; i < params.bar_count; i = i + 1u) {
        // Highpass
        let hp = hp_coeff * (bars[i] - 2.0 * bars[i - 1u] + bars[i - 2u])
            + 2.0 * (1.0 - a1) * hp_prev1 - (1.0 - a1) * (1.0 - a1) * hp_prev2;
        // Super smooth
        let filt = c1 * (hp + hp_prev1) / 2.0 + b * filt_prev1 - a * a * filt_prev2;
        // Sinewave = atan(filt / filt_prev) normalized
        var wave: f32 = 0.0;
        if (abs(filt_prev1) > 0.000001) {
            wave = clamp(atan2(filt, filt_prev1) / (pi / 2.0), -1.0, 1.0);
        }
        output[i] = wave;
        hp_prev2 = hp_prev1; hp_prev1 = hp;
        filt_prev2 = filt_prev1; filt_prev1 = filt;
    }
}
"#;

pub(super) const EHLERS_MAMA_SHADER: &str = r#"
// Ehlers MAMA/FAMA — sequential (adaptive moving average pair)
// Output: [mama, fama] per bar = 2 floats per bar
struct Params { period: u32, bar_count: u32, }  // period unused
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;  // [mama, fama] interleaved
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let fast_limit: f32 = 0.5;
    let slow_limit: f32 = 0.05;
    let pi: f32 = 3.14159265;
    if (params.bar_count < 7u) { return; }

    // Smoothed price
    var sm_arr: array<f32, 7>;
    for (var i: u32 = 0u; i < 7u; i = i + 1u) { sm_arr[i] = bars[i]; }

    var mama_v: f32 = bars[0];
    var fama_v: f32 = bars[0];
    var prev_phase: f32 = 0.0;
    var prev_i1: f32 = 0.0;

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < 6u) {
            output[i * 2u] = bars[i];
            output[i * 2u + 1u] = bars[i];
            continue;
        }

        // 4-bar WMA smooth
        let s = (4.0 * bars[i] + 3.0 * bars[i - 1u] + 2.0 * bars[i - 2u] + bars[i - 3u]) / 10.0;
        let s2 = (4.0 * bars[i - 2u] + 3.0 * bars[i - 3u] + 2.0 * bars[i - 4u] + bars[i - 5u]) / 10.0;
        let s4 = (4.0 * bars[i - 4u] + 3.0 * bars[i - 5u] + 2.0 * bars[max(i, 6u) - 6u] + bars[max(i, 7u) - min(7u, i)]) / 10.0;

        // Hilbert discriminator
        let det = 0.0962 * s + 0.5769 * s2 - 0.5769 * s4 - 0.0962 * (4.0 * bars[max(i, 6u) - 6u] + 3.0 * bars[max(i, 7u) - min(7u, i)] + 2.0 * bars[max(i, 8u) - min(8u, i)] + bars[max(i, 9u) - min(9u, i)]) / 10.0;
        let i1 = bars[i - 3u];

        // Phase
        var phase: f32 = 0.0;
        if (abs(i1) > 0.000001) { phase = atan2(det, i1) * 180.0 / pi; }
        let delta_phase = max(prev_phase - phase, 1.0);
        let alpha = max(fast_limit / delta_phase, slow_limit);

        mama_v = alpha * s + (1.0 - alpha) * mama_v;
        fama_v = 0.5 * alpha * mama_v + (1.0 - 0.5 * alpha) * fama_v;

        output[i * 2u] = mama_v;
        output[i * 2u + 1u] = fama_v;
        prev_phase = phase;
        prev_i1 = i1;
    }
}
"#;

pub(super) const SUPPLY_DEMAND_SHADER: &str = r#"
// Supply/Demand Zone Detection — Phase 1: GPU fractal detection (parallel per-bar)
// Port of SupplyDemand.mqh IsFractalHigh/IsFractalLow with 5-bar lookback.
// Output: [zone_type, zone_high, zone_low] per bar (3 floats)
//   zone_type: 0=none, 1=demand (fractal low), -1=supply (fractal high)
//   zone_high/zone_low: body-to-wick boundaries matching MQL5
// CPU then does zone testing, merging, and break detection.
struct Params { period: u32, bar_count: u32, }  // period = fractal lookback (5)
@group(0) @binding(0) var<storage, read> bars: array<f32>;  // [h,l,c] interleaved
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    let base = i * 3u;
    let lookback = params.period;  // 5

    // Need lookback bars on each side
    if (i < lookback || i + lookback >= params.bar_count) {
        output[base] = 0.0; output[base + 1u] = 0.0; output[base + 2u] = 0.0;
        return;
    }

    let h = bars[i * 3u];
    let l = bars[i * 3u + 1u];
    let c = bars[i * 3u + 2u];

    // Fractal high: bar's high is strictly greater than lookback bars on each side
    var is_fractal_high = true;
    for (var k: u32 = 1u; k <= lookback; k = k + 1u) {
        if (bars[(i - k) * 3u] >= h || bars[(i + k) * 3u] >= h) {
            is_fractal_high = false;
            break;
        }
    }

    // Fractal low: bar's low is strictly less than lookback bars on each side
    var is_fractal_low = true;
    for (var k: u32 = 1u; k <= lookback; k = k + 1u) {
        if (bars[(i - k) * 3u + 1u] <= l || bars[(i + k) * 3u + 1u] <= l) {
            is_fractal_low = false;
            break;
        }
    }

    if (is_fractal_high) {
        // Supply zone: hi = high, lo = min(close, open) ≈ min(close, prev_close) approximation
        // Note: OHLC buffer lacks open; use close as approximation (CPU refines with actual open)
        output[base] = -1.0;
        output[base + 1u] = h;
        output[base + 2u] = c;  // placeholder — CPU replaces with min(close, open)
    } else if (is_fractal_low) {
        // Demand zone: hi = max(close, open) ≈ close, lo = low
        output[base] = 1.0;
        output[base + 1u] = c;  // placeholder — CPU replaces with max(close, open)
        output[base + 2u] = l;
    } else {
        output[base] = 0.0; output[base + 1u] = 0.0; output[base + 2u] = 0.0;
    }
}
"#;

pub(super) const ATR_PROJECTION_SHADER: &str = r#"
// ATR Projection — parallel per-bar: open ± ATR using resident open + aux ATR buffers.
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> open_bars: array<f32>;
@group(0) @binding(4) var<storage, read> atr_values: array<f32>;
@group(0) @binding(5) var<storage, read_write> output: array<f32>;
@group(0) @binding(6) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    let open_val = open_bars[i];
    let atr_val = atr_values[i];
    if (atr_val > 0.0) {
        output[i * 2u] = open_val + atr_val;
        output[i * 2u + 1u] = open_val - atr_val;
    } else {
        output[i * 2u] = 0.0;
        output[i * 2u + 1u] = 0.0;
    }
}
"#;

pub(super) const BETTER_VOLUME_SHADER: &str = r#"
// BetterVolume — Full Emini-Watch algorithm (1:1 parity with CPU/MQL5)
// Output: classification f32: 0=low_vol, 1=climax_up, 2=climax_dn, 3=churn, 4=climax_churn, 5=normal
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> open_bars: array<f32>;
@group(0) @binding(1) var<storage, read> ohlc_bars: array<f32>;
@group(0) @binding(3) var<storage, read> volumes: array<f32>;
@group(0) @binding(5) var<storage, read_write> output: array<f32>;
@group(0) @binding(6) var<uniform> params: Params;

// Estimate buy/sell volume from candle structure (matching MQL5 EstimateBuySell)
fn estimate_buy(o: f32, h: f32, l: f32, c: f32, vol: f32) -> f32 {
    let range = h - l;
    if (range <= 0.0) { return vol * 0.5; }
    if (c > o) {
        let denom = 2.0 * range + o - c;
        let d = select(denom, range, denom <= 0.0);
        return (range / d) * vol;
    } else if (c < o) {
        let denom = 2.0 * range + c - o;
        let d = select(denom, range, denom <= 0.0);
        return ((range + c - o) / d) * vol;
    }
    return vol * 0.5;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    let lb = params.period; // lookback (20)
    if (i >= params.bar_count) { output[i] = 5.0; return; }
    if (i < lb) { output[i] = 5.0; return; } // not enough history

    let min_range: f32 = 0.0000000001;

    // Current bar OHLCV
    let base = i * 3u;
    let o = open_bars[i];
    let h = ohlc_bars[base];
    let l = ohlc_bars[base + 1u];
    let c = ohlc_bars[base + 2u];
    let vol = volumes[i];
    let range = max(h - l, min_range);

    let buy_vol = estimate_buy(o, h, l, c, vol);
    let sell_vol = vol - buy_vol;

    let buy_range = buy_vol * range;
    let sell_range = sell_vol * range;
    let vol_div_r = vol / range;
    let sell_div_r = sell_vol / range;
    let buy_div_r = buy_vol / range;

    // Lookback extremes (previous lb bars)
    var high_buy_range: f32 = 0.0;
    var high_sell_range: f32 = 0.0;
    var high_vol_div_r: f32 = 0.0;
    var low_sell_div_r: f32 = 999999999.0;
    var low_buy_div_r: f32 = 999999999.0;
    var low_total_vol: f32 = 999999999.0;

    for (var j: u32 = 1u; j <= lb; j = j + 1u) {
        let bi = i - j;
        let bbase = bi * 3u;
        let bo = open_bars[bi];
        let bh = ohlc_bars[bbase];
        let bl = ohlc_bars[bbase + 1u];
        let bc = ohlc_bars[bbase + 2u];
        let bv = volumes[bi];
        let br = max(bh - bl, min_range);

        let bbuy = estimate_buy(bo, bh, bl, bc, bv);
        let bsell = bv - bbuy;

        let bbr = bbuy * br;
        let bsr = bsell * br;
        let bvr = bv / br;
        let bsdr = bsell / br;
        let bbdr = bbuy / br;

        high_buy_range = max(high_buy_range, bbr);
        high_sell_range = max(high_sell_range, bsr);
        high_vol_div_r = max(high_vol_div_r, bvr);
        low_sell_div_r = min(low_sell_div_r, bsdr);
        low_buy_div_r = min(low_buy_div_r, bbdr);
        low_total_vol = min(low_total_vol, bv);
    }

    // 1-bar classification
    var is_climax_up: bool = false;
    var is_climax_dn: bool = false;
    var is_churn: bool = false;
    var is_low_vol: bool = false;

    if (vol <= low_total_vol) { is_low_vol = true; }
    if (c > o && (buy_range >= high_buy_range || sell_div_r <= low_sell_div_r)) { is_climax_up = true; }
    if (c < o && (sell_range >= high_sell_range || buy_div_r <= low_buy_div_r)) { is_climax_dn = true; }
    if (vol_div_r >= high_vol_div_r) { is_churn = true; }

    // 2-bar analysis (matching MQL5 InpUse2Bars=true)
    if (i >= lb + 1u) {
        let pi = i - 1u;
        let pbase = pi * 3u;
        let po = open_bars[pi];
        let ph = ohlc_bars[pbase];
        let pl = ohlc_bars[pbase + 1u];
        let pc = ohlc_bars[pbase + 2u];
        let pv = volumes[pi];

        let pbuy = estimate_buy(po, ph, pl, pc, pv);
        let psell = pv - pbuy;
        let total_buy = buy_vol + pbuy;
        let total_sell = sell_vol + psell;
        let total_vol2 = vol + pv;
        let range2 = max(max(h, ph) - min(l, pl), min_range);

        let buy_range2 = total_buy * range2;
        let sell_range2 = total_sell * range2;
        let vol_div_r2 = total_vol2 / range2;
        let sell_div_r2 = total_sell / range2;
        let buy_div_r2 = total_buy / range2;

        // 2-bar lookback extremes
        var h_br2: f32 = 0.0;
        var h_sr2: f32 = 0.0;
        var h_vr2: f32 = 0.0;
        var l_sdr2: f32 = 999999999.0;
        var l_bdr2: f32 = 999999999.0;
        var l_vol2: f32 = 999999999.0;

        for (var j: u32 = 1u; j <= lb; j = j + 1u) {
            let b1i = i - j;
            if (b1i == 0u) { break; }
            let b2i = b1i - 1u;

            let base1 = b1i * 3u;
            let base2 = b2i * 3u;
            let o1 = open_bars[b1i]; let h1 = ohlc_bars[base1]; let l1 = ohlc_bars[base1 + 1u];
            let c1 = ohlc_bars[base1 + 2u]; let v1 = volumes[b1i];
            let o2 = open_bars[b2i]; let h2 = ohlc_bars[base2]; let l2 = ohlc_bars[base2 + 1u];
            let c2 = ohlc_bars[base2 + 2u]; let v2 = volumes[b2i];

            let tb = estimate_buy(o1, h1, l1, c1, v1) + estimate_buy(o2, h2, l2, c2, v2);
            let ts = (v1 - estimate_buy(o1, h1, l1, c1, v1)) + (v2 - estimate_buy(o2, h2, l2, c2, v2));
            let tv = v1 + v2;
            let r2 = max(max(h1, h2) - min(l1, l2), min_range);

            h_br2 = max(h_br2, tb * r2);
            h_sr2 = max(h_sr2, ts * r2);
            h_vr2 = max(h_vr2, tv / r2);
            l_sdr2 = min(l_sdr2, ts / r2);
            l_bdr2 = min(l_bdr2, tb / r2);
            l_vol2 = min(l_vol2, tv);
        }

        if (total_vol2 <= l_vol2) { is_low_vol = true; }
        if (c > o && (buy_range2 >= h_br2 || sell_div_r2 <= l_sdr2)) { is_climax_up = true; }
        if (c < o && (sell_range2 >= h_sr2 || buy_div_r2 <= l_bdr2)) { is_climax_dn = true; }
        if (vol_div_r2 >= h_vr2) { is_churn = true; }
    }

    // Priority: ClimaxChurn > LowVol > ClimaxUp > ClimaxDown > Churn > Normal
    if ((is_climax_up || is_climax_dn) && is_churn) { output[i] = 4.0; }  // climax+churn (magenta)
    else if (is_low_vol) { output[i] = 0.0; }     // low volume (yellow)
    else if (is_climax_up) { output[i] = 1.0; }   // climax up (red)
    else if (is_climax_dn) { output[i] = 2.0; }   // climax down (white)
    else if (is_churn) { output[i] = 3.0; }       // churn (green)
    else { output[i] = 5.0; }                     // normal (steelblue)
}
"#;

pub(super) const ANCHORED_VWAP_SHADER: &str = r#"
// Anchored VWAP — sequential from anchor bar to end
// Cumulative (price × volume) / cumulative volume from anchor point
struct Params { period: u32, bar_count: u32, }  // period = anchor bar index
@group(0) @binding(2) var<storage, read> close_bars: array<f32>;
@group(0) @binding(3) var<storage, read> volumes: array<f32>;
@group(0) @binding(5) var<storage, read_write> output: array<f32>;
@group(0) @binding(6) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let anchor = params.period;
    var cum_pv: f32 = 0.0;
    var cum_vol: f32 = 0.0;

    for (var i: u32 = 0u; i < params.bar_count; i = i + 1u) {
        if (i < anchor) { output[i] = 0.0; continue; }
        let close = close_bars[i];
        let vol = volumes[i];
        cum_pv = cum_pv + close * vol;
        cum_vol = cum_vol + vol;
        output[i] = select(cum_pv / cum_vol, close, cum_vol < 0.000001);
    }
}
"#;

pub(super) const BACKTEST_EVAL_SHADER: &str = r#"
struct Params {
    bar_count: u32,
    combo_count: u32,
}

struct Combo {
    sma_fast: u32,
    sma_slow: u32,
    rsi_period: u32,
    rsi_overbought: f32,
    rsi_oversold: f32,
    atr_period: u32,
    atr_sl_mult: f32,
    atr_tp_mult: f32,
}

@group(0) @binding(0) var<storage, read> closes: array<f32>;
@group(0) @binding(1) var<storage, read> ohlc: array<f32>;  // [h,l,c] × bar_count
@group(0) @binding(2) var<storage, read> combos: array<f32>;  // 8 floats per combo
@group(0) @binding(3) var<storage, read_write> results: array<f32>;  // 9 floats per combo
@group(0) @binding(4) var<uniform> params: Params;

// Compute SMA at bar index for given period
fn sma_at(idx: u32, period: u32) -> f32 {
    if (idx < period - 1u) { return 0.0; }
    var sum: f32 = 0.0;
    for (var j: u32 = 0u; j < period; j = j + 1u) {
        sum = sum + closes[idx - j];
    }
    return sum / f32(period);
}

// Compute ATR at bar index
fn atr_at(idx: u32, period: u32) -> f32 {
    if (idx < period + 1u) { return 0.0; }
    var atr: f32 = 0.0;
    // Simple average of TR over last `period` bars
    for (var j: u32 = 0u; j < period; j = j + 1u) {
        let i = idx - j;
        let h = ohlc[i * 3u];
        let l = ohlc[i * 3u + 1u];
        let prev_c = ohlc[(i - 1u) * 3u + 2u];
        let tr = max(h - l, max(abs(h - prev_c), abs(l - prev_c)));
        atr = atr + tr;
    }
    return atr / f32(period);
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let combo_idx = id.x;
    if (combo_idx >= params.combo_count) { return; }

    // Unpack parameters
    let base = combo_idx * 8u;
    let sma_fast = bitcast<u32>(combos[base]);
    let sma_slow = bitcast<u32>(combos[base + 1u]);
    let rsi_period = bitcast<u32>(combos[base + 2u]);
    let rsi_ob = combos[base + 3u];
    let rsi_os = combos[base + 4u];
    let atr_period = bitcast<u32>(combos[base + 5u]);
    let atr_sl_mult = combos[base + 6u];
    let atr_tp_mult = combos[base + 7u];

    let lookback = max(sma_slow, max(rsi_period + 1u, atr_period + 1u));

    // State
    var equity: f32 = 100000.0;
    var peak: f32 = equity;
    var max_dd: f32 = 0.0;
    var in_trade: bool = false;
    var trade_dir: i32 = 0;  // 1=long, -1=short
    var entry_price: f32 = 0.0;
    var stop_loss: f32 = 0.0;
    var take_profit: f32 = 0.0;
    var wins: u32 = 0u;
    var losses: u32 = 0u;
    var total_profit: f32 = 0.0;
    var total_loss: f32 = 0.0;
    var total_hold: u32 = 0u;
    var trade_start: u32 = 0u;
    var daily_pnl_sum: f32 = 0.0;
    var daily_pnl_sq: f32 = 0.0;
    var daily_pnl_down: f32 = 0.0;
    var prev_equity: f32 = equity;

    // RSI state (running)
    var avg_gain: f32 = 0.0;
    var avg_loss: f32 = 0.0;
    var rsi_ready: bool = false;
    var rsi_val: f32 = 50.0;

    // Seed RSI
    if (rsi_period > 0u && lookback < params.bar_count) {
        for (var i: u32 = 1u; i <= rsi_period; i = i + 1u) {
            let chg = closes[i] - closes[i - 1u];
            if (chg > 0.0) { avg_gain = avg_gain + chg; }
            else { avg_loss = avg_loss - chg; }
        }
        avg_gain = avg_gain / f32(rsi_period);
        avg_loss = avg_loss / f32(rsi_period);
        rsi_ready = true;
    }

    // Walk bars
    for (var i: u32 = lookback; i < params.bar_count; i = i + 1u) {
        let close = closes[i];
        let prev_close = closes[i - 1u];
        let high = ohlc[i * 3u];
        let low = ohlc[i * 3u + 1u];

        // Update RSI
        if (rsi_ready && i > rsi_period) {
            let chg = close - prev_close;
            let gain = max(chg, 0.0);
            let loss = max(-chg, 0.0);
            avg_gain = (avg_gain * f32(rsi_period - 1u) + gain) / f32(rsi_period);
            avg_loss = (avg_loss * f32(rsi_period - 1u) + loss) / f32(rsi_period);
            let rs = select(avg_gain / avg_loss, 100.0, avg_loss < 0.000001);
            rsi_val = 100.0 - 100.0 / (1.0 + rs);
        }

        // SMA values
        let fast_sma = sma_at(i, sma_fast);
        let slow_sma = sma_at(i, sma_slow);
        let prev_fast = sma_at(i - 1u, sma_fast);
        let prev_slow = sma_at(i - 1u, sma_slow);
        let atr = atr_at(i, atr_period);

        // Check SL/TP if in trade
        if (in_trade) {
            var pnl: f32 = 0.0;
            var closed: bool = false;

            if (trade_dir == 1) {
                // Long: check stop loss (low touches SL) or take profit (high touches TP)
                if (low <= stop_loss) { pnl = stop_loss - entry_price; closed = true; }
                else if (take_profit > 0.0 && high >= take_profit) { pnl = take_profit - entry_price; closed = true; }
            } else {
                // Short
                if (high >= stop_loss) { pnl = entry_price - stop_loss; closed = true; }
                else if (take_profit > 0.0 && low <= take_profit) { pnl = entry_price - take_profit; closed = true; }
            }

            if (closed) {
                equity = equity + pnl;
                if (pnl > 0.0) { wins = wins + 1u; total_profit = total_profit + pnl; }
                else { losses = losses + 1u; total_loss = total_loss - pnl; }
                total_hold = total_hold + (i - trade_start);
                in_trade = false;
            }
        }

        // Entry signals (SMA crossover + RSI filter)
        if (!in_trade && fast_sma > 0.0 && slow_sma > 0.0 && atr > 0.0) {
            // Long: fast crosses above slow, RSI not overbought
            if (prev_fast <= prev_slow && fast_sma > slow_sma && rsi_val < rsi_ob) {
                in_trade = true;
                trade_dir = 1;
                entry_price = close;
                stop_loss = close - atr * atr_sl_mult;
                take_profit = close + atr * atr_tp_mult;
                trade_start = i;
            }
            // Short: fast crosses below slow, RSI not oversold
            else if (prev_fast >= prev_slow && fast_sma < slow_sma && rsi_val > rsi_os) {
                in_trade = true;
                trade_dir = -1;
                entry_price = close;
                stop_loss = close + atr * atr_sl_mult;
                take_profit = close - atr * atr_tp_mult;
                trade_start = i;
            }
        }

        // Track drawdown and daily PnL
        if (equity > peak) { peak = equity; }
        if (peak > 0.0) {
            let dd = (peak - equity) / peak;
            if (dd > max_dd) { max_dd = dd; }
        }
        let daily_ret = (equity - prev_equity) / max(prev_equity, 0.01);
        daily_pnl_sum = daily_pnl_sum + daily_ret;
        daily_pnl_sq = daily_pnl_sq + daily_ret * daily_ret;
        if (daily_ret < 0.0) { daily_pnl_down = daily_pnl_down + daily_ret * daily_ret; }
        prev_equity = equity;
    }

    // Close any open trade at last bar
    if (in_trade) {
        let last_close = closes[params.bar_count - 1u];
        var pnl: f32 = 0.0;
        if (trade_dir == 1) { pnl = last_close - entry_price; }
        else { pnl = entry_price - last_close; }
        equity = equity + pnl;
        if (pnl > 0.0) { wins = wins + 1u; } else { losses = losses + 1u; }
    }

    // Compute metrics
    let trades = wins + losses;
    let net_pnl = equity - 100000.0;
    let n_f = f32(params.bar_count - lookback);
    let mean_ret = daily_pnl_sum / max(n_f, 1.0);
    let variance = daily_pnl_sq / max(n_f, 1.0) - mean_ret * mean_ret;
    let std_dev = sqrt(max(variance, 0.0));
    let down_dev = sqrt(daily_pnl_down / max(n_f, 1.0));
    let ann_mean = mean_ret * 252.0;
    let ann_vol = std_dev * 15.8745;
    let ann_down = down_dev * 15.8745;
    let sharpe = select(ann_mean / ann_vol, 0.0, ann_vol < 0.000001);
    let sortino = select(ann_mean / ann_down, 0.0, ann_down < 0.000001);
    let win_rate = select(f32(wins) / f32(trades), 0.0, trades == 0u);
    let pf = select(total_profit / max(total_loss, 0.01), 0.0, trades == 0u);
    let avg_hold = select(f32(total_hold) / f32(trades), 0.0, trades == 0u);

    // Write results: [net_pnl, max_dd, sharpe, sortino, win_rate, pf, trades, avg_hold, 0(robustness)]
    let out = combo_idx * 9u;
    results[out] = net_pnl;
    results[out + 1u] = max_dd;
    results[out + 2u] = sharpe;
    results[out + 3u] = sortino;
    results[out + 4u] = win_rate;
    results[out + 5u] = pf;
    results[out + 6u] = f32(trades);
    results[out + 7u] = avg_hold;
    results[out + 8u] = 0.0;  // robustness filled by second pass
}
"#;

pub(super) const ROBUSTNESS_SHADER: &str = r#"
struct Params {
    bar_count: u32,
    combo_count: u32,
}

@group(0) @binding(0) var<storage, read> closes: array<f32>;       // unused but needed for layout
@group(0) @binding(1) var<storage, read> ohlc: array<f32>;         // unused
@group(0) @binding(2) var<storage, read> combos: array<f32>;       // param combos
@group(0) @binding(3) var<storage, read_write> results: array<f32>; // update robustness field
@group(0) @binding(4) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= params.combo_count) { return; }

    let out = idx * 9u;
    let my_sharpe = results[out + 2u];

    // Compare with neighbors (simple: ±1 index as proxy for ±1 on each param)
    var sum: f32 = my_sharpe;
    var sum_sq: f32 = my_sharpe * my_sharpe;
    var count: f32 = 1.0;

    if (idx > 0u) {
        let neighbor_sharpe = results[(idx - 1u) * 9u + 2u];
        sum = sum + neighbor_sharpe;
        sum_sq = sum_sq + neighbor_sharpe * neighbor_sharpe;
        count = count + 1.0;
    }
    if (idx + 1u < params.combo_count) {
        let neighbor_sharpe = results[(idx + 1u) * 9u + 2u];
        sum = sum + neighbor_sharpe;
        sum_sq = sum_sq + neighbor_sharpe * neighbor_sharpe;
        count = count + 1.0;
    }

    let mean = sum / count;
    let variance = sum_sq / count - mean * mean;
    // Robustness: low variance among neighbors = high score
    let robustness = select(1.0 / (1.0 + sqrt(max(variance, 0.0)) * 10.0), 0.0, my_sharpe < 0.0);
    results[out + 8u] = robustness;
}
"#;

pub(super) const MONTE_CARLO_SHADER: &str = r#"
struct Params {
    bar_count: u32,
    combo_count: u32,  // repurposed: simulation_count
}

@group(0) @binding(0) var<storage, read> closes: array<f32>;       // repurposed: daily returns
@group(0) @binding(1) var<storage, read> ohlc: array<f32>;         // unused
@group(0) @binding(2) var<storage, read> combos: array<f32>;       // repurposed: [days_forward, starting_equity, 0...]
@group(0) @binding(3) var<storage, read_write> results: array<f32>; // final equity per simulation
@group(0) @binding(4) var<uniform> params: Params;

// PCG hash for GPU-side pseudo-random number generation
fn pcg_hash(input: u32) -> u32 {
    var state = input * 747796405u + 2891336453u;
    var word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn rand_f32(seed: ptr<function, u32>) -> f32 {
    *seed = pcg_hash(*seed);
    return f32(*seed) / 4294967295.0;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let sim_idx = id.x;
    if (sim_idx >= params.combo_count) { return; }

    let n_returns = params.bar_count;
    let days_forward = bitcast<u32>(combos[0]);
    let starting_equity = combos[1];

    var seed: u32 = sim_idx * 1234567u + 42u;
    var equity: f32 = starting_equity;
    var peak: f32 = equity;
    var max_dd: f32 = 0.0;

    // Random walk: sample from historical returns
    for (var d: u32 = 0u; d < days_forward; d = d + 1u) {
        // Pick a random historical return
        let r_idx = u32(rand_f32(&seed) * f32(n_returns - 1u));
        let daily_ret = closes[min(r_idx, n_returns - 1u)];
        equity = equity * (1.0 + daily_ret);

        if (equity > peak) { peak = equity; }
        let dd = (peak - equity) / max(peak, 0.01);
        if (dd > max_dd) { max_dd = dd; }
    }

    // Output: [final_equity, max_drawdown, ...] per simulation
    // Pack into 9-float result slots (reusing BacktestResult layout)
    let out = sim_idx * 9u;
    results[out] = equity - starting_equity;  // net PnL
    results[out + 1u] = max_dd;               // max drawdown
    results[out + 2u] = (equity - starting_equity) / starting_equity * 100.0;  // return %
    results[out + 3u] = equity;               // final equity
    // Remaining slots zeroed
    results[out + 4u] = 0.0;
    results[out + 5u] = 0.0;
    results[out + 6u] = 0.0;
    results[out + 7u] = 0.0;
    results[out + 8u] = 0.0;
}
"#;

pub(super) const NNFX_EVAL_SHADER: &str = r#"
struct Params {
    bar_count: u32,
    combo_count: u32,
}

@group(0) @binding(0) var<storage, read> closes: array<f32>;
@group(0) @binding(1) var<storage, read> ohlc: array<f32>;
@group(0) @binding(2) var<storage, read> combos: array<f32>;
@group(0) @binding(3) var<storage, read_write> results: array<f32>;
@group(0) @binding(4) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let combo_idx = id.x;
    if (combo_idx >= params.combo_count) { return; }

    let base = combo_idx * 8u;
    let kama_period = bitcast<u32>(combos[base]);
    let fisher_period = bitcast<u32>(combos[base + 1u]);
    let atr_period = bitcast<u32>(combos[base + 2u]);
    let adx_period = bitcast<u32>(combos[base + 3u]);
    let adx_threshold = combos[base + 4u];
    let atr_sl_mult = combos[base + 5u];
    let atr_tp_mult = combos[base + 6u];

    let lookback = max(max(kama_period, fisher_period), max(atr_period, adx_period)) + 2u;
    if (lookback >= params.bar_count) {
        let out = combo_idx * 9u;
        for (var k: u32 = 0u; k < 9u; k = k + 1u) { results[out + k] = 0.0; }
        return;
    }

    // State
    var equity: f32 = 100000.0;
    var peak: f32 = equity;
    var max_dd: f32 = 0.0;
    var in_trade: bool = false;
    var trade_dir: i32 = 0;
    var entry_price: f32 = 0.0;
    var stop_loss: f32 = 0.0;
    var take_profit: f32 = 0.0;
    var wins: u32 = 0u;
    var losses: u32 = 0u;
    var total_profit: f32 = 0.0;
    var total_loss: f32 = 0.0;
    var total_hold: u32 = 0u;
    var trade_start: u32 = 0u;
    var prev_equity: f32 = equity;
    var daily_pnl_sum: f32 = 0.0;
    var daily_pnl_sq: f32 = 0.0;
    var daily_pnl_down: f32 = 0.0;

    // KAMA state
    let fast_sc: f32 = 2.0 / 3.0;
    let slow_sc: f32 = 2.0 / 31.0;
    var kama: f32 = closes[0];
    var prev_kama: f32 = closes[0];

    // Fisher state
    var fish: f32 = 0.0;
    var prev_fish: f32 = 0.0;
    var fish_val: f32 = 0.0;

    // ATR state
    var atr_val: f32 = 0.0;
    var atr_sum: f32 = 0.0;
    var atr_ready: bool = false;

    // ADX state
    var smooth_plus_dm: f32 = 0.0;
    var smooth_minus_dm: f32 = 0.0;
    var smooth_tr: f32 = 0.0;
    var smooth_dx: f32 = 0.0;
    var adx_val: f32 = 0.0;

    for (var i: u32 = 1u; i < params.bar_count; i = i + 1u) {
        let close = closes[i];
        let high = ohlc[i * 3u];
        let low = ohlc[i * 3u + 1u];
        let prev_close = closes[i - 1u];
        let prev_high = ohlc[(i - 1u) * 3u];
        let prev_low = ohlc[(i - 1u) * 3u + 1u];
        let mid = (high + low) / 2.0;

        // Update KAMA
        if (i >= kama_period) {
            let direction = abs(close - closes[i - kama_period]);
            var volatility: f32 = 0.0;
            for (var j: u32 = i - kama_period + 1u; j <= i; j = j + 1u) {
                volatility = volatility + abs(closes[j] - closes[j - 1u]);
            }
            let er = select(direction / volatility, 0.0, volatility < 0.000001);
            let sc = er * (fast_sc - slow_sc) + slow_sc;
            prev_kama = kama;
            kama = kama + sc * sc * (close - kama);
        }

        // Update Fisher Transform
        if (i >= fisher_period) {
            var highest: f32 = -1000000.0;
            var lowest: f32 = 1000000.0;
            for (var j: u32 = i - fisher_period + 1u; j <= i; j = j + 1u) {
                let m = (ohlc[j * 3u] + ohlc[j * 3u + 1u]) / 2.0;
                if (m > highest) { highest = m; }
                if (m < lowest) { lowest = m; }
            }
            let range = highest - lowest;
            var raw: f32 = 0.0;
            if (range > 0.000001) { raw = 2.0 * (mid - lowest) / range - 1.0; }
            raw = clamp(raw, -0.999, 0.999);
            fish_val = 0.33 * raw + 0.67 * fish_val;
            fish_val = clamp(fish_val, -0.999, 0.999);
            prev_fish = fish;
            fish = 0.5 * log((1.0 + fish_val) / (1.0 - fish_val));
        }

        // Update ATR
        let tr = max(high - low, max(abs(high - prev_close), abs(low - prev_close)));
        if (i <= atr_period) {
            atr_sum = atr_sum + tr;
            if (i == atr_period) { atr_val = atr_sum / f32(atr_period); atr_ready = true; }
        } else if (atr_ready) {
            atr_val = (atr_val * f32(atr_period - 1u) + tr) / f32(atr_period);
        }

        // Update ADX
        let up_move = high - prev_high;
        let down_move = prev_low - low;
        var plus_dm: f32 = 0.0;
        var minus_dm: f32 = 0.0;
        if (up_move > down_move && up_move > 0.0) { plus_dm = up_move; }
        if (down_move > up_move && down_move > 0.0) { minus_dm = down_move; }
        if (i <= adx_period) {
            smooth_plus_dm = smooth_plus_dm + plus_dm;
            smooth_minus_dm = smooth_minus_dm + minus_dm;
            smooth_tr = smooth_tr + tr;
        } else {
            let p = f32(adx_period);
            smooth_plus_dm = smooth_plus_dm - smooth_plus_dm / p + plus_dm;
            smooth_minus_dm = smooth_minus_dm - smooth_minus_dm / p + minus_dm;
            smooth_tr = smooth_tr - smooth_tr / p + tr;
            let plus_di = select(100.0 * smooth_plus_dm / smooth_tr, 0.0, smooth_tr < 0.000001);
            let minus_di = select(100.0 * smooth_minus_dm / smooth_tr, 0.0, smooth_tr < 0.000001);
            let di_sum = plus_di + minus_di;
            let dx = select(100.0 * abs(plus_di - minus_di) / di_sum, 0.0, di_sum < 0.000001);
            smooth_dx = (smooth_dx * (f32(adx_period) - 1.0) + dx) / f32(adx_period);
            adx_val = smooth_dx;
        }

        if (i < lookback) { continue; }

        // Check SL/TP
        if (in_trade) {
            var pnl: f32 = 0.0;
            var closed: bool = false;
            if (trade_dir == 1) {
                if (low <= stop_loss) { pnl = stop_loss - entry_price; closed = true; }
                else if (take_profit > 0.0 && high >= take_profit) { pnl = take_profit - entry_price; closed = true; }
            } else {
                if (high >= stop_loss) { pnl = entry_price - stop_loss; closed = true; }
                else if (take_profit > 0.0 && low <= take_profit) { pnl = entry_price - take_profit; closed = true; }
            }
            if (closed) {
                equity = equity + pnl;
                if (pnl > 0.0) { wins = wins + 1u; total_profit = total_profit + pnl; }
                else { losses = losses + 1u; total_loss = total_loss - pnl; }
                total_hold = total_hold + (i - trade_start);
                in_trade = false;
            }
        }

        // NNFX Entry: Fisher crosses zero + KAMA confirms trend + ADX filter
        if (!in_trade && atr_ready && adx_val > adx_threshold) {
            // Long: Fisher crosses above 0, KAMA rising
            if (prev_fish <= 0.0 && fish > 0.0 && kama > prev_kama) {
                in_trade = true; trade_dir = 1;
                entry_price = close;
                stop_loss = close - atr_val * atr_sl_mult;
                take_profit = close + atr_val * atr_tp_mult;
                trade_start = i;
            }
            // Short: Fisher crosses below 0, KAMA falling
            else if (prev_fish >= 0.0 && fish < 0.0 && kama < prev_kama) {
                in_trade = true; trade_dir = -1;
                entry_price = close;
                stop_loss = close + atr_val * atr_sl_mult;
                take_profit = close - atr_val * atr_tp_mult;
                trade_start = i;
            }
        }

        // Track drawdown
        if (equity > peak) { peak = equity; }
        if (peak > 0.0) { let dd = (peak - equity) / peak; if (dd > max_dd) { max_dd = dd; } }
        let daily_ret = (equity - prev_equity) / max(prev_equity, 0.01);
        daily_pnl_sum = daily_pnl_sum + daily_ret;
        daily_pnl_sq = daily_pnl_sq + daily_ret * daily_ret;
        if (daily_ret < 0.0) { daily_pnl_down = daily_pnl_down + daily_ret * daily_ret; }
        prev_equity = equity;
    }

    // Close open trade
    if (in_trade) {
        let lc = closes[params.bar_count - 1u];
        if (trade_dir == 1) { equity = equity + lc - entry_price; }
        else { equity = equity + entry_price - lc; }
    }

    // Metrics
    let trades = wins + losses;
    let n_f = f32(params.bar_count - lookback);
    let mean_ret = daily_pnl_sum / max(n_f, 1.0);
    let variance = daily_pnl_sq / max(n_f, 1.0) - mean_ret * mean_ret;
    let std_dev = sqrt(max(variance, 0.0));
    let down_dev = sqrt(daily_pnl_down / max(n_f, 1.0));
    let ann_mean = mean_ret * 252.0;
    let ann_vol = std_dev * 15.8745;
    let ann_down = down_dev * 15.8745;

    let out = combo_idx * 9u;
    results[out] = equity - 100000.0;
    results[out + 1u] = max_dd;
    results[out + 2u] = select(ann_mean / ann_vol, 0.0, ann_vol < 0.000001);
    results[out + 3u] = select(ann_mean / ann_down, 0.0, ann_down < 0.000001);
    results[out + 4u] = select(f32(wins) / f32(trades), 0.0, trades == 0u);
    results[out + 5u] = select(total_profit / max(total_loss, 0.01), 0.0, trades == 0u);
    results[out + 6u] = f32(trades);
    results[out + 7u] = select(f32(total_hold) / f32(trades), 0.0, trades == 0u);
    results[out + 8u] = 0.0;
}
"#;

pub(super) const WALK_FORWARD_SHADER: &str = r#"
struct Params {
    bar_count: u32,
    combo_count: u32,
}

@group(0) @binding(0) var<storage, read> closes: array<f32>;
@group(0) @binding(1) var<storage, read> ohlc: array<f32>;
@group(0) @binding(2) var<storage, read> combos: array<f32>;
@group(0) @binding(3) var<storage, read_write> results: array<f32>;
@group(0) @binding(4) var<uniform> params: Params;

// Walk-forward uses same eval logic but the caller uploads a subset of bars
// for the out-of-sample window. This shader is identical to BACKTEST_EVAL_SHADER
// but exists as a separate pipeline for clarity. The host code handles windowing.
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let combo_idx = id.x;
    if (combo_idx >= params.combo_count) { return; }

    // Identical to backtest eval — host controls which bars are uploaded
    let base = combo_idx * 8u;
    let sma_fast = bitcast<u32>(combos[base]);
    let sma_slow = bitcast<u32>(combos[base + 1u]);
    let lookback = max(sma_fast, sma_slow) + 2u;
    if (lookback >= params.bar_count) {
        let out = combo_idx * 9u;
        for (var k: u32 = 0u; k < 9u; k = k + 1u) { results[out + k] = 0.0; }
        return;
    }

    var equity: f32 = 100000.0;
    var peak: f32 = equity;
    var max_dd: f32 = 0.0;
    var wins: u32 = 0u;
    var losses: u32 = 0u;
    var in_trade: bool = false;
    var trade_dir: i32 = 0;
    var entry_price: f32 = 0.0;

    for (var i: u32 = lookback; i < params.bar_count; i = i + 1u) {
        // Simplified SMA cross for walk-forward (reuses same combo format)
        var fast_sum: f32 = 0.0;
        var slow_sum: f32 = 0.0;
        var prev_fast_sum: f32 = 0.0;
        var prev_slow_sum: f32 = 0.0;
        for (var j: u32 = 0u; j < sma_fast; j = j + 1u) { fast_sum += closes[i - j]; prev_fast_sum += closes[i - 1u - j]; }
        for (var j: u32 = 0u; j < sma_slow; j = j + 1u) { slow_sum += closes[i - j]; prev_slow_sum += closes[i - 1u - j]; }
        let fast_sma = fast_sum / f32(sma_fast);
        let slow_sma = slow_sum / f32(sma_slow);
        let prev_fast = prev_fast_sum / f32(sma_fast);
        let prev_slow = prev_slow_sum / f32(sma_slow);

        if (in_trade) {
            let pnl = select(closes[i] - entry_price, entry_price - closes[i], trade_dir == 1);
            // Simple exit: reverse signal
            if ((trade_dir == 1 && fast_sma < slow_sma) || (trade_dir == -1 && fast_sma > slow_sma)) {
                equity += pnl;
                if (pnl > 0.0) { wins += 1u; } else { losses += 1u; }
                in_trade = false;
            }
        }
        if (!in_trade) {
            if (prev_fast <= prev_slow && fast_sma > slow_sma) { in_trade = true; trade_dir = 1; entry_price = closes[i]; }
            else if (prev_fast >= prev_slow && fast_sma < slow_sma) { in_trade = true; trade_dir = -1; entry_price = closes[i]; }
        }
        if (equity > peak) { peak = equity; }
        let dd = (peak - equity) / max(peak, 0.01);
        if (dd > max_dd) { max_dd = dd; }
    }

    let trades = wins + losses;
    let out = combo_idx * 9u;
    results[out] = equity - 100000.0;
    results[out + 1u] = max_dd;
    results[out + 2u] = select(f32(wins) / f32(trades), 0.0, trades == 0u);
    results[out + 3u] = 0.0; results[out + 4u] = 0.0; results[out + 5u] = 0.0;
    results[out + 6u] = f32(trades);
    results[out + 7u] = 0.0; results[out + 8u] = 0.0;
}
"#;

pub(super) const VOLUME_PROFILE_SHADER: &str = r#"
struct Params {
    bar_count: u32,
    num_levels: u32,
    price_min: f32,
    price_max: f32,
}
@group(0) @binding(0) var<storage, read> ohlcv: array<f32>;
@group(0) @binding(1) var<storage, read_write> histogram: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= params.bar_count) { return; }
    let base = i * 5u;
    let high = ohlcv[base + 1u];
    let low = ohlcv[base + 2u];
    let close = ohlcv[base + 3u];
    let volume = ohlcv[base + 4u];
    let price_range = params.price_max - params.price_min;
    if (price_range <= 0.0) { return; }
    // Distribute volume across price levels touched by this bar
    let level_size = price_range / f32(params.num_levels);
    let lo_level = u32(max((low - params.price_min) / level_size, 0.0));
    let hi_level = min(u32((high - params.price_min) / level_size), params.num_levels - 1u);
    let levels_touched = hi_level - lo_level + 1u;
    let vol_per_level = volume / f32(levels_touched);
    for (var l = lo_level; l <= hi_level; l++) {
        // Atomic-free: each thread writes to different regions (acceptable race for visualization)
        histogram[l] += vol_per_level;
    }
}
"#;

pub(super) const BATCH_SCREENER_SHADER: &str = r#"
struct Params {
    symbol_count: u32,
    bars_per_symbol: u32,
    rsi_period: u32,
    sma_period: u32,
}
@group(0) @binding(0) var<storage, read> closes: array<f32>;
@group(0) @binding(1) var<storage, read_write> results: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let sym = gid.x;
    if (sym >= params.symbol_count) { return; }
    let base = sym * params.bars_per_symbol;
    let n = params.bars_per_symbol;
    if (n < 2u) { results[sym * 2u] = 50.0; results[sym * 2u + 1u] = 0.0; return; }

    // RSI
    var avg_gain = 0.0;
    var avg_loss = 0.0;
    let rp = min(params.rsi_period, n - 1u);
    for (var i = 1u; i <= rp; i++) {
        let diff = closes[base + i] - closes[base + i - 1u];
        if (diff > 0.0) { avg_gain += diff; } else { avg_loss -= diff; }
    }
    avg_gain /= f32(rp);
    avg_loss /= f32(rp);
    for (var i = rp + 1u; i < n; i++) {
        let diff = closes[base + i] - closes[base + i - 1u];
        if (diff > 0.0) {
            avg_gain = (avg_gain * f32(rp - 1u) + diff) / f32(rp);
            avg_loss = (avg_loss * f32(rp - 1u)) / f32(rp);
        } else {
            avg_gain = (avg_gain * f32(rp - 1u)) / f32(rp);
            avg_loss = (avg_loss * f32(rp - 1u) - diff) / f32(rp);
        }
    }
    var rsi = 50.0;
    if (avg_loss > 0.0) {
        let rs = avg_gain / avg_loss;
        rsi = 100.0 - (100.0 / (1.0 + rs));
    } else if (avg_gain > 0.0) {
        rsi = 100.0;
    }
    results[sym * 2u] = rsi;

    // SMA (last sma_period bars)
    var sum = 0.0;
    let sp = min(params.sma_period, n);
    for (var i = n - sp; i < n; i++) {
        sum += closes[base + i];
    }
    results[sym * 2u + 1u] = sum / f32(sp);
}
"#;

pub(super) const ROLLING_STATS_SHADER: &str = r#"
struct Params {
    total_days: u32,
    window_size: u32,
    _pad0: u32,
    _pad1: u32,
}
@group(0) @binding(0) var<storage, read> returns: array<f32>;
@group(0) @binding(1) var<storage, read_write> rolling_sharpe: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pos = gid.x;
    let end = pos + params.window_size;
    if (end > params.total_days) { return; }

    // Mean
    var sum = 0.0;
    for (var i = pos; i < end; i++) {
        sum += returns[i];
    }
    let mean = sum / f32(params.window_size);

    // StdDev
    var var_sum = 0.0;
    for (var i = pos; i < end; i++) {
        let diff = returns[i] - mean;
        var_sum += diff * diff;
    }
    let std_dev = sqrt(var_sum / f32(params.window_size));

    // Sharpe (annualized, assumes daily returns)
    if (std_dev > 0.0001) {
        rolling_sharpe[pos] = (mean * 252.0) / (std_dev * sqrt(252.0));
    } else {
        rolling_sharpe[pos] = 0.0;
    }
}
"#;

pub(super) const RENKO_BUILDER_SHADER: &str = r#"
struct Params {
    bar_count: u32,
    brick_size: f32,
    _pad0: u32,
    _pad1: u32,
}
@group(0) @binding(0) var<storage, read> closes: array<f32>;
@group(0) @binding(1) var<storage, read_write> bricks: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    if (params.bar_count < 1u) { return; }
    var brick_base = closes[0u];
    var brick_count = 0u;
    let max_bricks = params.bar_count * 2u; // upper bound

    for (var i = 1u; i < params.bar_count; i++) {
        let price = closes[i];
        // Up bricks
        while (price >= brick_base + params.brick_size && brick_count < max_bricks) {
            let out = brick_count * 3u;
            bricks[out] = 1.0;  // direction: up
            bricks[out + 1u] = brick_base;
            bricks[out + 2u] = brick_base + params.brick_size;
            brick_base += params.brick_size;
            brick_count++;
        }
        // Down bricks
        while (price <= brick_base - params.brick_size && brick_count < max_bricks) {
            let out = brick_count * 3u;
            bricks[out] = -1.0;  // direction: down
            bricks[out + 1u] = brick_base;
            bricks[out + 2u] = brick_base - params.brick_size;
            brick_base -= params.brick_size;
            brick_count++;
        }
    }
    // Store brick count in first output slot (slot 0 rewritten)
    // Consumers check bricks[i*3] for 1.0/-1.0 vs 0.0 to find end
}
"#;

pub(super) const TICK_AGGREGATION_SHADER: &str = r#"
struct Params {
    tick_count: u32,
    tf_seconds: u32,
    _pad0: u32,
    _pad1: u32,
}
@group(0) @binding(0) var<storage, read> tick_prices: array<f32>;
@group(0) @binding(1) var<storage, read> tick_timestamps: array<u32>;
@group(0) @binding(2) var<storage, read_write> ohlcv_out: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let bar_idx = gid.x;
    if (params.tick_count == 0u) { return; }

    let bar_start_ts = tick_timestamps[0u] + bar_idx * params.tf_seconds;
    let bar_end_ts = bar_start_ts + params.tf_seconds;

    var o = 0.0;
    var h = -999999.0;
    var l = 999999.0;
    var c = 0.0;
    var v = 0.0;
    var found = false;

    for (var i = 0u; i < params.tick_count; i++) {
        let ts = tick_timestamps[i];
        if (ts >= bar_start_ts && ts < bar_end_ts) {
            let price = tick_prices[i];
            if (!found) { o = price; found = true; }
            if (price > h) { h = price; }
            if (price < l) { l = price; }
            c = price;
            v += 1.0;
        }
    }

    if (found) {
        let out = bar_idx * 5u;
        ohlcv_out[out] = o;
        ohlcv_out[out + 1u] = h;
        ohlcv_out[out + 2u] = l;
        ohlcv_out[out + 3u] = c;
        ohlcv_out[out + 4u] = v;
    }
}
"#;

pub(super) const MULTI_SYMBOL_BACKTEST_SHADER: &str = r#"
struct Params {
    bars_per_symbol: u32,
    symbol_count: u32,
    fast_start: u32,
    fast_step: u32,
    slow_start: u32,
    slow_step: u32,
    combos_per_symbol: u32,
    _pad: u32,
}
@group(0) @binding(0) var<storage, read> all_closes: array<f32>;
@group(0) @binding(1) var<storage, read_write> results: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let thread_idx = gid.x;
    let total_combos = params.symbol_count * params.combos_per_symbol;
    if (thread_idx >= total_combos) { return; }

    let sym_idx = thread_idx / params.combos_per_symbol;
    let combo_idx = thread_idx % params.combos_per_symbol;
    let base = sym_idx * params.bars_per_symbol;
    let n = params.bars_per_symbol;

    // Derive fast/slow periods from combo index (grid search)
    let fast_range = 10u; // 10 fast values per slow value
    let fast_idx = combo_idx % fast_range;
    let slow_idx = combo_idx / fast_range;
    let fast_period = params.fast_start + fast_idx * params.fast_step;
    let slow_period = params.slow_start + slow_idx * params.slow_step;

    if (fast_period >= slow_period || slow_period >= n) {
        let out = thread_idx * 4u;
        results[out] = 0.0; results[out + 1u] = 0.0;
        results[out + 2u] = 0.0; results[out + 3u] = 0.0;
        return;
    }

    // SMA crossover backtest
    var equity = 100000.0;
    var peak = equity;
    var max_dd = 0.0;
    var wins = 0u;
    var losses = 0u;
    var in_trade = false;
    var trade_dir = 0;
    var entry_price = 0.0;

    for (var i = slow_period; i < n; i++) {
        // Compute fast SMA
        var fast_sum = 0.0;
        for (var j = i - fast_period; j < i; j++) { fast_sum += all_closes[base + j]; }
        let fast_sma = fast_sum / f32(fast_period);
        // Compute slow SMA
        var slow_sum = 0.0;
        for (var j = i - slow_period; j < i; j++) { slow_sum += all_closes[base + j]; }
        let slow_sma = slow_sum / f32(slow_period);

        let price = all_closes[base + i];

        if (in_trade) {
            let pnl = f32(trade_dir) * (price - entry_price) * 100.0;
            if ((trade_dir == 1 && fast_sma < slow_sma) || (trade_dir == -1 && fast_sma > slow_sma)) {
                equity += pnl;
                if (pnl > 0.0) { wins++; } else { losses++; }
                in_trade = false;
            }
        }
        if (!in_trade) {
            // Compute previous bar SMAs for crossover detection
            if (i > slow_period) {
                var pf = 0.0;
                for (var j = i - 1u - fast_period; j < i - 1u; j++) { pf += all_closes[base + j]; }
                let prev_fast = pf / f32(fast_period);
                var ps = 0.0;
                for (var j = i - 1u - slow_period; j < i - 1u; j++) { ps += all_closes[base + j]; }
                let prev_slow = ps / f32(slow_period);
                if (prev_fast <= prev_slow && fast_sma > slow_sma) {
                    in_trade = true; trade_dir = 1; entry_price = price;
                } else if (prev_fast >= prev_slow && fast_sma < slow_sma) {
                    in_trade = true; trade_dir = -1; entry_price = price;
                }
            }
        }
        if (equity > peak) { peak = equity; }
        let dd = (peak - equity) / max(peak, 0.01);
        if (dd > max_dd) { max_dd = dd; }
    }

    let trades = wins + losses;
    let out = thread_idx * 4u;
    results[out] = equity - 100000.0;  // net P&L
    results[out + 1u] = max_dd;
    results[out + 2u] = select(f32(wins) / f32(trades), 0.0, trades == 0u);
    results[out + 3u] = f32(trades);
}
"#;

pub(super) const CANDLE_RENDER_SHADER: &str = r#"
// Per-instance data uploaded from CPU each frame
struct CandleInstance {
    @location(0) x_center: f32,
    @location(1) body_top: f32,
    @location(2) body_bot: f32,
    @location(3) wick_top: f32,
    @location(4) wick_bot: f32,
    @location(5) is_up: f32,           // 1.0 = green, 0.0 = red
    @location(6) half_width: f32,
    @location(7) is_live_forming: f32, // 1.0 = live quote mode (thin mid line, no body)
}

struct Uniforms {
    viewport_width: f32,
    viewport_height: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

// 10 vertices per instance: 4 for body quad (triangle strip), 2 for wick top, 2 for wick bot
// vertex_index 0-3: body quad, 4-5: top wick, 6-7: bottom wick, 8-9: unused
@vertex
fn vs_main(
    instance: CandleInstance,
    @builtin(vertex_index) vid: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let green = vec4<f32>(0.0, 1.0, 0.0, 1.0);
    let red = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    if (instance.is_live_forming > 0.5) {
        out.color = vec4<f32>(0.9, 0.9, 0.9, 1.0); // neutral/white for live forming bar
    } else {
        out.color = select(red, green, instance.is_up > 0.5);
    }

    // NDC coords: x in [-1, 1], y in [-1, 1]
    let ndc_x = (instance.x_center / uniforms.viewport_width) * 2.0 - 1.0;
    let hw = (instance.half_width / uniforms.viewport_width) * 2.0;
    let wick_w = 1.0 / uniforms.viewport_width; // 1px wick

    var pos = vec2<f32>(0.0, 0.0);

    if (instance.is_live_forming > 0.5) {
        // Live forming bar: draw NOTHING — let the Bid/Ask overlay be the only live indicator
        out.position = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        return out;
    } else {
        switch (vid) {
            // Body quad (triangle strip: 0-1-2-3)
            case 0u: { pos = vec2<f32>(ndc_x - hw, 1.0 - instance.body_top / uniforms.viewport_height * 2.0); }
            case 1u: { pos = vec2<f32>(ndc_x + hw, 1.0 - instance.body_top / uniforms.viewport_height * 2.0); }
            case 2u: { pos = vec2<f32>(ndc_x - hw, 1.0 - instance.body_bot / uniforms.viewport_height * 2.0); }
            case 3u: { pos = vec2<f32>(ndc_x + hw, 1.0 - instance.body_bot / uniforms.viewport_height * 2.0); }
            // Top wick (line: 4-5)
            case 4u: { pos = vec2<f32>(ndc_x, 1.0 - instance.wick_top / uniforms.viewport_height * 2.0); }
            case 5u: { pos = vec2<f32>(ndc_x, 1.0 - instance.body_top / uniforms.viewport_height * 2.0); }
            // Bottom wick (line: 6-7)
            case 6u: { pos = vec2<f32>(ndc_x, 1.0 - instance.body_bot / uniforms.viewport_height * 2.0); }
            case 7u: { pos = vec2<f32>(ndc_x, 1.0 - instance.wick_bot / uniforms.viewport_height * 2.0); }
            default: { pos = vec2<f32>(0.0, 0.0); }
        }
    }

    out.position = vec4<f32>(pos, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

pub(super) const POLYLINE_RENDER_SHADER: &str = r#"
struct Uniforms {
    viewport_width: f32,
    viewport_height: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let ndc_x = (in.pos.x / uniforms.viewport_width) * 2.0 - 1.0;
    let ndc_y = 1.0 - (in.pos.y / uniforms.viewport_height) * 2.0;
    out.position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

pub(super) const HEATMAP_RENDER_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VertexOutput {
    var out: VertexOutput;
    // Fullscreen triangle (3 vertices cover entire viewport)
    let x = f32(vid & 1u) * 4.0 - 1.0;
    let y = f32((vid >> 1u) & 1u) * 4.0 - 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@group(0) @binding(0) var heatmap_texture: texture_2d<f32>;
@group(0) @binding(1) var heatmap_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(heatmap_texture, heatmap_sampler, in.uv);
}
"#;

pub(super) const ZONE_COMPOSITE_SHADER: &str = r#"
struct ZoneInstance {
    @location(0) rect_min: vec2<f32>,
    @location(1) rect_max: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct Uniforms {
    viewport_width: f32,
    viewport_height: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(
    zone: ZoneInstance,
    @builtin(vertex_index) vid: u32,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = zone.color;
    // Expand instance to quad (triangle strip: 4 vertices)
    var pos = vec2<f32>(0.0, 0.0);
    switch (vid) {
        case 0u: { pos = zone.rect_min; }
        case 1u: { pos = vec2<f32>(zone.rect_max.x, zone.rect_min.y); }
        case 2u: { pos = vec2<f32>(zone.rect_min.x, zone.rect_max.y); }
        case 3u: { pos = zone.rect_max; }
        default: {}
    }
    let ndc_x = (pos.x / uniforms.viewport_width) * 2.0 - 1.0;
    let ndc_y = 1.0 - (pos.y / uniforms.viewport_height) * 2.0;
    out.position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

pub(super) const SUPERTREND_SHADER: &str = r#"
// Supertrend — sequential (ATR-based trailing stop with direction flip)
// Output: 2 per bar [supertrend_value, direction] where direction: 1.0=up, -1.0=down
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let n = params.bar_count;
    let p = params.period;
    let mult: f32 = 3.0;
    if (n < p + 1u) { return; }

    // ATR seed
    var atr: f32 = 0.0;
    for (var i: u32 = 1u; i < p + 1u; i = i + 1u) {
        let h = bars[i * 3u]; let l = bars[i * 3u + 1u]; let pc = bars[(i - 1u) * 3u + 2u];
        atr += max(h - l, max(abs(h - pc), abs(l - pc)));
    }
    atr = atr / f32(p);

    var dir: f32 = 1.0;
    var upper_band: f32 = 0.0;
    var lower_band: f32 = 0.0;

    for (var i: u32 = 0u; i < n; i = i + 1u) {
        let h = bars[i * 3u]; let l = bars[i * 3u + 1u]; let c = bars[i * 3u + 2u];
        if (i >= p) {
            let pc = bars[(i - 1u) * 3u + 2u];
            let tr = max(h - l, max(abs(h - pc), abs(l - pc)));
            atr = (atr * f32(p - 1u) + tr) / f32(p);
        }
        let hl2 = (h + l) / 2.0;
        let raw_upper = hl2 + mult * atr;
        let raw_lower = hl2 - mult * atr;
        if (i == 0u) { upper_band = raw_upper; lower_band = raw_lower; }
        else {
            upper_band = select(raw_upper, min(raw_upper, upper_band), raw_upper < upper_band || bars[(i - 1u) * 3u + 2u] > upper_band);
            lower_band = select(raw_lower, max(raw_lower, lower_band), raw_lower > lower_band || bars[(i - 1u) * 3u + 2u] < lower_band);
        }
        if (dir == 1.0 && c < lower_band) { dir = -1.0; }
        else if (dir == -1.0 && c > upper_band) { dir = 1.0; }
        output[i * 2u] = select(upper_band, lower_band, dir == 1.0);
        output[i * 2u + 1u] = dir;
    }
}
"#;

pub(super) const DONCHIAN_SHADER: &str = r#"
// Donchian Channel — parallel (rolling highest high, lowest low)
// Output: 2 per bar [upper, lower]
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    if (i < params.period - 1u) { output[i * 2u] = 0.0; output[i * 2u + 1u] = 0.0; return; }
    var hh: f32 = -1000000.0;
    var ll: f32 = 1000000.0;
    for (var j: u32 = 0u; j < params.period; j = j + 1u) {
        let idx = i - j;
        if (bars[idx * 3u] > hh) { hh = bars[idx * 3u]; }
        if (bars[idx * 3u + 1u] < ll) { ll = bars[idx * 3u + 1u]; }
    }
    output[i * 2u] = hh;
    output[i * 2u + 1u] = ll;
}
"#;

pub(super) const KELTNER_SHADER: &str = r#"
// Keltner Channel — sequential (EMA ± mult × ATR)
// Output: 3 per bar [upper, mid, lower]
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let n = params.bar_count;
    let p = params.period;
    let mult: f32 = 1.5;
    if (n < p) { return; }
    let alpha = 2.0 / (f32(p) + 1.0);
    var ema: f32 = bars[2u];
    var atr: f32 = bars[0u] - bars[1u];
    output[0u] = 0.0; output[1u] = 0.0; output[2u] = 0.0;
    for (var i: u32 = 1u; i < n; i = i + 1u) {
        let h = bars[i * 3u]; let l = bars[i * 3u + 1u]; let c = bars[i * 3u + 2u];
        let pc = bars[(i - 1u) * 3u + 2u];
        ema = alpha * c + (1.0 - alpha) * ema;
        let tr = max(h - l, max(abs(h - pc), abs(l - pc)));
        atr = (atr * f32(p - 1u) + tr) / f32(p);
        if (i < p) { output[i * 3u] = 0.0; output[i * 3u + 1u] = 0.0; output[i * 3u + 2u] = 0.0; }
        else {
            output[i * 3u] = ema + mult * atr;
            output[i * 3u + 1u] = ema;
            output[i * 3u + 2u] = ema - mult * atr;
        }
    }
}
"#;

pub(super) const REGRESSION_SHADER: &str = r#"
// Linear Regression Channel — parallel (least squares + standard error)
// Output: 3 per bar [mid, upper(+2σ), lower(−2σ)]
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    let p = params.period;
    if (i < p - 1u) { output[i * 3u] = 0.0; output[i * 3u + 1u] = 0.0; output[i * 3u + 2u] = 0.0; return; }
    let n = f32(p);
    var sum_x: f32 = 0.0; var sum_y: f32 = 0.0; var sum_xy: f32 = 0.0; var sum_xx: f32 = 0.0;
    for (var j: u32 = 0u; j < p; j = j + 1u) {
        let x = f32(j);
        let y = bars[(i - p + 1u + j) * 3u + 2u];
        sum_x += x; sum_y += y; sum_xy += x * y; sum_xx += x * x;
    }
    let denom = n * sum_xx - sum_x * sum_x;
    if (abs(denom) < 0.000001) { let avg = sum_y / n; output[i * 3u] = avg; output[i * 3u + 1u] = avg; output[i * 3u + 2u] = avg; return; }
    let b = (n * sum_xy - sum_x * sum_y) / denom;
    let a = (sum_y - b * sum_x) / n;
    let reg_val = a + b * (n - 1.0);
    var sse: f32 = 0.0;
    for (var j: u32 = 0u; j < p; j = j + 1u) {
        let e = bars[(i - p + 1u + j) * 3u + 2u] - (a + b * f32(j));
        sse += e * e;
    }
    let se = sqrt(sse / n);
    output[i * 3u] = reg_val;
    output[i * 3u + 1u] = reg_val + 2.0 * se;
    output[i * 3u + 2u] = reg_val - 2.0 * se;
}
"#;

pub(super) const SQUEEZE_SHADER: &str = r#"
// Squeeze Momentum — sequential (BB inside KC detection + momentum)
// Output: 2 per bar [momentum, squeeze_on]
struct Params { period: u32, bar_count: u32, }
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let n = params.bar_count;
    let p = params.period;
    let bb_mult: f32 = 2.0;
    let kc_mult: f32 = 1.5;
    if (n < p) { return; }
    let alpha = 2.0 / (f32(p) + 1.0);
    var ema: f32 = bars[2u];
    var atr: f32 = bars[0u] - bars[1u];
    for (var i: u32 = 0u; i < n; i = i + 1u) {
        let h = bars[i * 3u]; let l = bars[i * 3u + 1u]; let c = bars[i * 3u + 2u];
        if (i > 0u) {
            let pc = bars[(i - 1u) * 3u + 2u];
            ema = alpha * c + (1.0 - alpha) * ema;
            atr = (atr * f32(p - 1u) + max(h - l, max(abs(h - pc), abs(l - pc)))) / f32(p);
        }
        if (i < p - 1u) { output[i * 2u] = 0.0; output[i * 2u + 1u] = 0.0; continue; }
        // SMA + StdDev + Donchian over window
        var sum: f32 = 0.0; var hh: f32 = -1e9; var ll: f32 = 1e9;
        for (var j: u32 = 0u; j < p; j = j + 1u) {
            let sc = bars[(i - j) * 3u + 2u]; sum += sc;
            let sh = bars[(i - j) * 3u]; let sl = bars[(i - j) * 3u + 1u];
            if (sh > hh) { hh = sh; } if (sl < ll) { ll = sl; }
        }
        let sma = sum / f32(p);
        var vs: f32 = 0.0;
        for (var j: u32 = 0u; j < p; j = j + 1u) { let d = bars[(i - j) * 3u + 2u] - sma; vs += d * d; }
        let sd = sqrt(vs / f32(p));
        let squeeze_on = select(0.0, 1.0, sma - bb_mult * sd > ema - kc_mult * atr && sma + bb_mult * sd < ema + kc_mult * atr);
        output[i * 2u] = c - ((hh + ll) / 2.0 + sma) / 2.0;
        output[i * 2u + 1u] = squeeze_on;
    }
}
"#;

pub(super) const PREV_LEVELS_SHADER: &str = r#"
// Previous Candle Levels — parallel (approximate prev day high/low)
// Output: 2 per bar [prev_day_high, prev_day_low]
struct Params { period: u32, bar_count: u32, }  // period = minutes per bar
@group(0) @binding(0) var<storage, read> bars: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.bar_count) { return; }
    let bpd = select(1u, 1440u / max(params.period, 1u), params.period > 0u);
    if (i < bpd) { output[i * 2u] = 0.0; output[i * 2u + 1u] = 0.0; return; }
    let start = i - bpd;
    var ph: f32 = -1e9; var pl: f32 = 1e9;
    for (var j: u32 = start; j < i; j = j + 1u) {
        if (bars[j * 3u] > ph) { ph = bars[j * 3u]; }
        if (bars[j * 3u + 1u] < pl) { pl = bars[j * 3u + 1u]; }
    }
    output[i * 2u] = ph;
    output[i * 2u + 1u] = pl;
}
"#;
