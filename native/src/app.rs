//! Main application state and egui rendering.
//!
//! Fully functional trading terminal:
//! - Real candlestick rendering from SQLiteCache via egui Painter
//! - SMA(200), SMA(100), KAMA(10,2,30), EMA(21), Bollinger(20,2) indicators
//! - RSI(14) and Fisher Transform sub-panes
//! - Zoom (scroll wheel), pan (click-drag)
//! - Price/time axes
//! - Command palette (~ tilde, Quake-style console)
//! - Symbol/timeframe selector
//! - MTF grid mode
//! - Floating windows: Settings, DARWIN, Risk, Backtest, Screener, News, etc.
//! - Right panel: positions, orders, risk
//! - Bottom panel: log messages

use eframe::egui;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use typhoon_engine::core::cache::SqliteCache;
use std::io::Write;
use serde_json;
use typhoon_engine::core::darwin;
use typhoon_engine::core::backtest;
use typhoon_engine::core::risk;
use typhoon_engine::core::margin;
use typhoon_engine::broker::alpaca::Bar as EngineBar;

// ─── colours ────────────────────────────────────────────────────────────────
const BG: egui::Color32 = egui::Color32::from_rgb(0, 0, 0);
const GRID: egui::Color32 = egui::Color32::from_rgb(26, 26, 42);
const UP: egui::Color32 = egui::Color32::from_rgb(0, 220, 80);
const DOWN: egui::Color32 = egui::Color32::from_rgb(220, 40, 40);
const SMA200_COL: egui::Color32 = egui::Color32::from_rgb(255, 200, 50);
const SMA100_COL: egui::Color32 = egui::Color32::from_rgb(100, 180, 255);
const KAMA_COL: egui::Color32 = egui::Color32::from_rgb(200, 100, 255);
const EMA_COL: egui::Color32 = egui::Color32::from_rgb(255, 130, 60);
const BB_COL: egui::Color32 = egui::Color32::from_rgb(80, 160, 200);
const BB_FILL: egui::Color32 = egui::Color32::from_rgba_premultiplied(80, 160, 200, 25);
const AXIS_TEXT: egui::Color32 = egui::Color32::from_rgb(140, 140, 160);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(76, 175, 80);
const FISHER_POS: egui::Color32 = egui::Color32::from_rgb(0, 200, 100);
const FISHER_NEG: egui::Color32 = egui::Color32::from_rgb(200, 50, 50);
const RSI_LINE: egui::Color32 = egui::Color32::from_rgb(200, 180, 60);
const MACD_LINE_COL: egui::Color32 = egui::Color32::from_rgb(100, 180, 255);
const MACD_SIG_COL: egui::Color32 = egui::Color32::from_rgb(255, 130, 60);

// ─── types ───────────────────────────────────────────────────────────────────

/// A single OHLCV bar.
#[derive(Clone, Debug)]
struct Bar {
    ts_ms: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

/// Chart rendering style.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ChartType {
    Candle,
    HeikinAshi,
    Line,
    OhlcBars,
    Renko,
}

impl ChartType {
    fn label(self) -> &'static str {
        match self {
            ChartType::Candle     => "Candle",
            ChartType::HeikinAshi => "Heikin-Ashi",
            ChartType::Line       => "Line",
            ChartType::OhlcBars   => "OHLC Bars",
            ChartType::Renko      => "Renko",
        }
    }
}

/// Available timeframes for the selector toolbar.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Timeframe {
    M1, M5, M15, M30, H1, H4, D1, W1, MN1,
}

impl Timeframe {
    fn label(self) -> &'static str {
        match self {
            Timeframe::M1  => "M1",
            Timeframe::M5  => "M5",
            Timeframe::M15 => "M15",
            Timeframe::M30 => "M30",
            Timeframe::H1  => "H1",
            Timeframe::H4  => "H4",
            Timeframe::D1  => "D1",
            Timeframe::W1  => "W1",
            Timeframe::MN1 => "MN1",
        }
    }

    /// Build the cache key suffix used by the MT5 XML import pipeline.
    fn cache_suffix(self) -> &'static str {
        match self {
            Timeframe::M1  => "1Min",
            Timeframe::M5  => "5Min",
            Timeframe::M15 => "15Min",
            Timeframe::M30 => "30Min",
            Timeframe::H1  => "1Hour",
            Timeframe::H4  => "4Hour",
            Timeframe::D1  => "1Day",
            Timeframe::W1  => "1Week",
            Timeframe::MN1 => "1Month",
        }
    }
}

/// Log severity level.
#[derive(Clone, Debug)]
enum LogLevel {
    Info,
    Warn,
    Error,
}

/// A single log entry displayed in the bottom panel.
#[derive(Clone, Debug)]
struct LogEntry {
    level: LogLevel,
    msg: String,
}

impl LogEntry {
    fn info(msg: impl Into<String>) -> Self { Self { level: LogLevel::Info,  msg: msg.into() } }
    fn warn(msg: impl Into<String>) -> Self { Self { level: LogLevel::Warn,  msg: msg.into() } }
    fn err (msg: impl Into<String>) -> Self { Self { level: LogLevel::Error, msg: msg.into() } }

    fn color(&self) -> egui::Color32 {
        match self.level {
            LogLevel::Info  => egui::Color32::from_rgb(160, 200, 160),
            LogLevel::Warn  => egui::Color32::from_rgb(255, 200, 50),
            LogLevel::Error => egui::Color32::from_rgb(255, 80, 80),
        }
    }

    fn prefix(&self) -> &'static str {
        match self.level {
            LogLevel::Info  => "[INFO] ",
            LogLevel::Warn  => "[WARN] ",
            LogLevel::Error => "[ERR]  ",
        }
    }
}

// ─── drawing tools ───────────────────────────────────────────────────────────

const HLINE_COL: egui::Color32 = egui::Color32::from_rgb(255, 200, 80);
const TRENDLINE_COL: egui::Color32 = egui::Color32::from_rgb(100, 200, 255);
const FIBO_COL: egui::Color32 = egui::Color32::from_rgb(200, 160, 100);

#[derive(Clone, Debug)]
enum Drawing {
    /// Horizontal price line.
    HLine { price: f64, color: egui::Color32 },
    /// Trendline between two (bar_index, price) points.
    TrendLine {
        p1: (usize, f64), // (absolute bar index, price)
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Fibonacci retracement between two price levels.
    FiboRetrace {
        high: f64,
        low: f64,
        bar_start: usize,
        bar_end: usize,
    },
    /// Vertical line at a bar index.
    #[allow(dead_code)]
    VLine { bar_idx: usize, color: egui::Color32 },
    /// Rectangle between two (bar, price) corners.
    #[allow(dead_code)]
    Rectangle {
        p1: (usize, f64),
        p2: (usize, f64),
        color: egui::Color32,
    },
    /// Ray from one point extending infinitely to the right.
    #[allow(dead_code)]
    Ray {
        origin: (usize, f64),
        slope: f64, // price per bar
        color: egui::Color32,
    },
    /// Parallel channel (trendline + offset).
    #[allow(dead_code)]
    Channel {
        p1: (usize, f64),
        p2: (usize, f64),
        width: f64, // price offset for parallel line
        color: egui::Color32,
    },
}

/// Drawing interaction mode.
#[derive(Clone, Copy, PartialEq, Debug)]
enum DrawMode {
    None,
    PlacingHLine,
    PlacingTrendP1,
    PlacingTrendP2 { bar1: usize, price1: f64 },
    PlacingFiboP1,
    PlacingFiboP2 { bar1: usize, price1: f64 },
    PlacingVLine,
    PlacingRectP1,
    PlacingRectP2 { bar1: usize, price1: f64 },
    PlacingRayP1,
    PlacingRayP2 { bar1: usize, price1: f64 },
    PlacingChannelP1,
    PlacingChannelP2 { bar1: usize, price1: f64 },
    PlacingChannelP3 { bar1: usize, price1: f64, bar2: usize, price2: f64 },
}

// ─── Ichimoku data ───────────────────────────────────────────────────────────

const ICHI_TENKAN: egui::Color32 = egui::Color32::from_rgb(0, 180, 230);
const ICHI_KIJUN: egui::Color32 = egui::Color32::from_rgb(200, 50, 50);
const ICHI_SPAN_A: egui::Color32 = egui::Color32::from_rgb(80, 200, 80);
const ICHI_SPAN_B: egui::Color32 = egui::Color32::from_rgb(200, 80, 80);
const ICHI_CLOUD_BULL: egui::Color32 = egui::Color32::from_rgba_premultiplied(80, 200, 80, 20);
const ICHI_CLOUD_BEAR: egui::Color32 = egui::Color32::from_rgba_premultiplied(200, 80, 80, 20);

const STOCH_K_COL: egui::Color32 = egui::Color32::from_rgb(100, 180, 255);
const STOCH_D_COL: egui::Color32 = egui::Color32::from_rgb(255, 130, 60);
const ADX_COL: egui::Color32 = egui::Color32::from_rgb(200, 180, 60);
const DI_PLUS_COL: egui::Color32 = egui::Color32::from_rgb(0, 200, 100);
const DI_MINUS_COL: egui::Color32 = egui::Color32::from_rgb(200, 50, 50);
const WMA_COL: egui::Color32 = egui::Color32::from_rgb(180, 100, 200);
const HMA_COL: egui::Color32 = egui::Color32::from_rgb(0, 200, 200);
const CCI_COL: egui::Color32 = egui::Color32::from_rgb(200, 140, 80);
const WILLR_COL: egui::Color32 = egui::Color32::from_rgb(180, 80, 200);
const OBV_COL: egui::Color32 = egui::Color32::from_rgb(100, 200, 160);
const SAR_COL: egui::Color32 = egui::Color32::from_rgb(255, 200, 0);
const ATR_PROJ_COL: egui::Color32 = egui::Color32::from_rgb(255, 200, 50);
const BVOL_CLIMAX_UP: egui::Color32 = egui::Color32::from_rgb(0, 200, 80);
const BVOL_CLIMAX_DN: egui::Color32 = egui::Color32::from_rgb(220, 40, 40);
const BVOL_HIGH: egui::Color32 = egui::Color32::from_rgb(0, 120, 255);
const BVOL_LOW: egui::Color32 = egui::Color32::from_rgb(255, 200, 50);
const BVOL_CHURN: egui::Color32 = egui::Color32::from_rgb(180, 180, 180);

/// Indicator visibility flags passed to draw_chart.
struct IndicatorFlags {
    sma200: bool,
    sma100: bool,
    kama: bool,
    ema21: bool,
    bollinger: bool,
    ichimoku: bool,
    wma: bool,
    hma: bool,
    psar: bool,
    atr_proj: bool,
}

/// All state for one chart viewport.
struct ChartState {
    /// The symbol string shown in the toolbar.
    symbol: String,
    /// Currently selected timeframe.
    timeframe: Timeframe,
    /// Chart rendering style.
    chart_type: ChartType,
    /// Raw bar data loaded from cache.
    bars: Vec<Bar>,
    /// Pre-computed SMA(200) — indexed parallel to `bars`.
    sma200: Vec<Option<f64>>,
    /// Pre-computed SMA(100) — indexed parallel to `bars`.
    sma100: Vec<Option<f64>>,
    /// Pre-computed KAMA(10,2,30) — indexed parallel to `bars`.
    kama: Vec<Option<f64>>,
    /// Pre-computed EMA(21).
    ema21: Vec<Option<f64>>,
    /// Bollinger Bands (middle, upper, lower).
    bb_mid: Vec<Option<f64>>,
    bb_upper: Vec<Option<f64>>,
    bb_lower: Vec<Option<f64>>,
    /// RSI(14) — 0..100 range.
    rsi: Vec<Option<f64>>,
    /// Fisher Transform.
    fisher: Vec<Option<f64>>,
    fisher_signal: Vec<Option<f64>>,
    /// ATR(14).
    atr: Vec<Option<f64>>,
    /// MACD(12,26,9).
    macd_line: Vec<Option<f64>>,
    macd_signal: Vec<Option<f64>>,
    macd_hist: Vec<Option<f64>>,
    /// Stochastic(14,3,3).
    stoch_k: Vec<Option<f64>>,
    stoch_d: Vec<Option<f64>>,
    /// ADX(14) + DI+/DI-.
    adx: Vec<Option<f64>>,
    di_plus: Vec<Option<f64>>,
    di_minus: Vec<Option<f64>>,
    /// Ichimoku(9,26,52).
    ichi_tenkan: Vec<Option<f64>>,
    ichi_kijun: Vec<Option<f64>>,
    ichi_span_a: Vec<Option<f64>>,
    ichi_span_b: Vec<Option<f64>>,
    /// WMA(20), HMA(20).
    wma: Vec<Option<f64>>,
    hma: Vec<Option<f64>>,
    /// CCI(20), Williams %R(14).
    cci: Vec<Option<f64>>,
    williams_r: Vec<Option<f64>>,
    /// OBV.
    obv: Vec<Option<f64>>,
    /// Momentum(10).
    momentum: Vec<Option<f64>>,
    /// Parabolic SAR(0.02, 0.2).
    psar: Vec<Option<f64>>,
    /// ATR Projection (open ± ATR bands).
    atr_proj_upper: Vec<Option<f64>>,
    atr_proj_lower: Vec<Option<f64>>,
    /// Better Volume classification.
    better_vol_type: Vec<u8>, // 0=normal, 1=climax_up, 2=climax_dn, 3=high, 4=low, 5=churn
    /// Drawing annotations.
    drawings: Vec<Drawing>,

    // ── view state ────────────────────────────────────────────────────────
    /// How many bars are visible horizontally (zoom level).
    visible_bars: usize,
    /// Index of the right-most visible bar (0 = oldest, len-1 = newest).
    view_offset: usize,
    /// Fractional price offset for vertical pan.
    price_pan: f64,
    /// Multiplier applied to the natural price range for vertical zoom.
    price_zoom: f64,

    // ── interaction helpers ───────────────────────────────────────────────
    is_dragging: bool,
    drag_start: Option<egui::Pos2>,
    drag_start_offset: usize,
    drag_start_ppan: f64,
}

impl ChartState {
    fn new(symbol: impl Into<String>, tf: Timeframe) -> Self {
        Self {
            symbol: symbol.into(),
            timeframe: tf,
            chart_type: ChartType::Candle,
            bars: Vec::new(),
            sma200: Vec::new(),
            sma100: Vec::new(),
            kama: Vec::new(),
            ema21: Vec::new(),
            bb_mid: Vec::new(),
            bb_upper: Vec::new(),
            bb_lower: Vec::new(),
            rsi: Vec::new(),
            fisher: Vec::new(),
            fisher_signal: Vec::new(),
            atr: Vec::new(),
            macd_line: Vec::new(),
            macd_signal: Vec::new(),
            macd_hist: Vec::new(),
            stoch_k: Vec::new(),
            stoch_d: Vec::new(),
            adx: Vec::new(),
            di_plus: Vec::new(),
            di_minus: Vec::new(),
            ichi_tenkan: Vec::new(),
            ichi_kijun: Vec::new(),
            ichi_span_a: Vec::new(),
            ichi_span_b: Vec::new(),
            wma: Vec::new(),
            hma: Vec::new(),
            cci: Vec::new(),
            williams_r: Vec::new(),
            obv: Vec::new(),
            momentum: Vec::new(),
            psar: Vec::new(),
            atr_proj_upper: Vec::new(),
            atr_proj_lower: Vec::new(),
            better_vol_type: Vec::new(),
            drawings: Vec::new(),
            visible_bars: 200,
            view_offset: 0,
            price_pan: 0.0,
            price_zoom: 1.0,
            is_dragging: false,
            drag_start: None,
            drag_start_offset: 0,
            drag_start_ppan: 0.0,
        }
    }

    /// Cache key for this symbol + timeframe.
    fn cache_key(&self) -> String {
        format!("mt5:CC:{}", self.timeframe.cache_suffix())
    }

    /// Load bars from the shared cache, re-compute indicators.
    fn load(&mut self, cache: &SqliteCache, log: &mut VecDeque<LogEntry>) {
        let key = self.cache_key();
        match cache.get_bars_raw(&key) {
            Ok(Some(raw)) => {
                self.bars = raw.into_iter().map(|(ts, o, h, l, c, v)| Bar {
                    ts_ms: ts, open: o, high: h, low: l, close: c, volume: v,
                }).collect();
                self.view_offset = self.bars.len().saturating_sub(1);
                self.compute_indicators();
                log.push_back(LogEntry::info(format!(
                    "Loaded {} bars for {} [{}]",
                    self.bars.len(), self.symbol, self.timeframe.label()
                )));
            }
            Ok(None) => {
                self.bars.clear();
                log.push_back(LogEntry::warn(format!(
                    "No data found for key '{}' — run the MT5 XML import pipeline first", key
                )));
            }
            Err(e) => {
                self.bars.clear();
                log.push_back(LogEntry::err(format!("Cache read error: {e}")));
            }
        }
        while log.len() > 500 { log.pop_front(); }
    }

    fn compute_indicators(&mut self) {
        self.sma200 = compute_sma(&self.bars, 200);
        self.sma100 = compute_sma(&self.bars, 100);
        self.kama   = compute_kama(&self.bars, 10, 2, 30);
        self.ema21  = compute_ema(&self.bars, 21);
        let (mid, upper, lower) = compute_bollinger(&self.bars, 20, 2.0);
        self.bb_mid = mid;
        self.bb_upper = upper;
        self.bb_lower = lower;
        self.rsi = compute_rsi(&self.bars, 14);
        let (f, fs) = compute_fisher(&self.bars, 10);
        self.fisher = f;
        self.fisher_signal = fs;
        self.atr = compute_atr(&self.bars, 14);
        let (ml, ms, mh) = compute_macd(&self.bars, 12, 26, 9);
        self.macd_line = ml;
        self.macd_signal = ms;
        self.macd_hist = mh;
        let (sk, sd) = compute_stochastic(&self.bars, 14, 3, 3);
        self.stoch_k = sk;
        self.stoch_d = sd;
        let (adx, dip, dim) = compute_adx(&self.bars, 14);
        self.adx = adx;
        self.di_plus = dip;
        self.di_minus = dim;
        let (tk, kj, sa, sb) = compute_ichimoku(&self.bars, 9, 26, 52);
        self.ichi_tenkan = tk;
        self.ichi_kijun = kj;
        self.ichi_span_a = sa;
        self.ichi_span_b = sb;
        self.wma = compute_wma(&self.bars, 20);
        self.hma = compute_hma(&self.bars, 20);
        self.cci = compute_cci(&self.bars, 20);
        self.williams_r = compute_williams_r(&self.bars, 14);
        self.obv = compute_obv(&self.bars);
        self.momentum = compute_momentum(&self.bars, 10);
        self.psar = compute_parabolic_sar(&self.bars, 0.02, 0.2);
        let (au, al) = compute_atr_projection(&self.bars, &self.atr);
        self.atr_proj_upper = au;
        self.atr_proj_lower = al;
        self.better_vol_type = compute_better_volume(&self.bars);
    }

    fn visible_range(&self) -> (usize, usize) {
        if self.bars.is_empty() { return (0, 0); }
        let end = (self.view_offset + 1).min(self.bars.len());
        let start = end.saturating_sub(self.visible_bars);
        (start, end)
    }
}

// ─── indicator computation ────────────────────────────────────────────────────

fn compute_sma(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < period { return out; }
    let mut sum: f64 = bars[..period].iter().map(|b| b.close).sum();
    out[period - 1] = Some(sum / period as f64);
    for i in period..n {
        sum += bars[i].close - bars[i - period].close;
        out[i] = Some(sum / period as f64);
    }
    out
}

fn compute_ema(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < period { return out; }
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

/// Kaufman Adaptive Moving Average.
fn compute_kama(bars: &[Bar], er_period: usize, fast: usize, slow: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n <= er_period { return out; }

    let fast_sc = 2.0 / (fast as f64 + 1.0);
    let slow_sc = 2.0 / (slow as f64 + 1.0);

    let mut kama = bars[er_period].close;
    out[er_period] = Some(kama);

    for i in (er_period + 1)..n {
        let direction = (bars[i].close - bars[i - er_period].close).abs();
        let volatility: f64 = (0..er_period)
            .map(|k| (bars[i - k].close - bars[i - k - 1].close).abs())
            .sum();
        let er = if volatility < f64::EPSILON { 0.0 } else { (direction / volatility).clamp(0.0, 1.0) };
        let sc = (er * (fast_sc - slow_sc) + slow_sc).powi(2);
        kama += sc * (bars[i].close - kama);
        out[i] = Some(kama);
    }
    out
}

fn compute_bollinger(bars: &[Bar], period: usize, mult: f64) -> (Vec<Option<f64>>, Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = bars.len();
    let mut mid = vec![None; n];
    let mut upper = vec![None; n];
    let mut lower = vec![None; n];
    if n < period { return (mid, upper, lower); }

    for i in (period - 1)..n {
        let slice = &bars[(i + 1 - period)..=i];
        let mean: f64 = slice.iter().map(|b| b.close).sum::<f64>() / period as f64;
        let variance: f64 = slice.iter().map(|b| (b.close - mean).powi(2)).sum::<f64>() / period as f64;
        let std_dev = variance.sqrt();
        mid[i] = Some(mean);
        upper[i] = Some(mean + mult * std_dev);
        lower[i] = Some(mean - mult * std_dev);
    }
    (mid, upper, lower)
}

fn compute_rsi(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n <= period { return out; }

    let mut avg_gain = 0.0_f64;
    let mut avg_loss = 0.0_f64;

    // Initial averages
    for i in 1..=period {
        let delta = bars[i].close - bars[i - 1].close;
        if delta > 0.0 { avg_gain += delta; } else { avg_loss -= delta; }
    }
    avg_gain /= period as f64;
    avg_loss /= period as f64;

    let rs = if avg_loss < f64::EPSILON { 100.0 } else { avg_gain / avg_loss };
    out[period] = Some(100.0 - 100.0 / (1.0 + rs));

    for i in (period + 1)..n {
        let delta = bars[i].close - bars[i - 1].close;
        let (gain, loss) = if delta > 0.0 { (delta, 0.0) } else { (0.0, -delta) };
        avg_gain = (avg_gain * (period as f64 - 1.0) + gain) / period as f64;
        avg_loss = (avg_loss * (period as f64 - 1.0) + loss) / period as f64;
        let rs = if avg_loss < f64::EPSILON { 100.0 } else { avg_gain / avg_loss };
        out[i] = Some(100.0 - 100.0 / (1.0 + rs));
    }
    out
}

fn compute_fisher(bars: &[Bar], period: usize) -> (Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = bars.len();
    let mut fisher = vec![None; n];
    let mut signal = vec![None; n];
    if n <= period { return (fisher, signal); }

    let mut val = 0.0_f64;
    let mut prev_fisher = 0.0_f64;

    for i in period..n {
        let slice = &bars[(i + 1 - period)..=i];
        let hi = slice.iter().map(|b| b.high).fold(f64::MIN, f64::max);
        let lo = slice.iter().map(|b| b.low).fold(f64::MAX, f64::min);
        let mid = (bars[i].high + bars[i].low) / 2.0;

        let range = hi - lo;
        let raw = if range < f64::EPSILON { 0.0 } else { 2.0 * ((mid - lo) / range - 0.5) };
        val = 0.33 * raw.clamp(-0.999, 0.999) + 0.67 * val;
        let clamped = val.clamp(-0.999, 0.999);
        let f = 0.5 * ((1.0 + clamped) / (1.0 - clamped)).ln();

        signal[i] = Some(prev_fisher);
        fisher[i] = Some(f);
        prev_fisher = f;
    }
    (fisher, signal)
}

fn compute_atr(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n <= period { return out; }

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

fn true_range(bar: &Bar, prev: &Bar) -> f64 {
    let hl = bar.high - bar.low;
    let hc = (bar.high - prev.close).abs();
    let lc = (bar.low - prev.close).abs();
    hl.max(hc).max(lc)
}

fn compute_macd(bars: &[Bar], fast: usize, slow: usize, signal_period: usize) -> (Vec<Option<f64>>, Vec<Option<f64>>, Vec<Option<f64>>) {
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
fn heikin_ashi(bars: &[Bar]) -> Vec<Bar> {
    if bars.is_empty() { return Vec::new(); }
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
fn renko_bricks(bars: &[Bar]) -> Vec<Bar> {
    if bars.len() < 15 { return bars.to_vec(); }
    // Compute brick size from ATR(14)
    let atr_vals = compute_atr(bars, 14);
    let brick_size = atr_vals.iter().rev().flatten().next().copied().unwrap_or(1.0);
    if brick_size < f64::EPSILON { return bars.to_vec(); }

    let mut bricks: Vec<Bar> = Vec::new();
    let mut current = bars[0].close;

    for bar in bars {
        while bar.close >= current + brick_size {
            let open = current;
            current += brick_size;
            bricks.push(Bar {
                ts_ms: bar.ts_ms, open, high: current, low: open, close: current, volume: bar.volume,
            });
        }
        while bar.close <= current - brick_size {
            let open = current;
            current -= brick_size;
            bricks.push(Bar {
                ts_ms: bar.ts_ms, open, high: open, low: current, close: current, volume: bar.volume,
            });
        }
    }
    if bricks.is_empty() { bars.to_vec() } else { bricks }
}

fn compute_stochastic(bars: &[Bar], k_period: usize, k_smooth: usize, d_smooth: usize) -> (Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = bars.len();
    let mut raw_k = vec![None; n];
    if n < k_period { return (raw_k.clone(), raw_k); }

    // Raw %K
    for i in (k_period - 1)..n {
        let slice = &bars[(i + 1 - k_period)..=i];
        let hi = slice.iter().map(|b| b.high).fold(f64::MIN, f64::max);
        let lo = slice.iter().map(|b| b.low).fold(f64::MAX, f64::min);
        let range = hi - lo;
        raw_k[i] = Some(if range < f64::EPSILON { 50.0 } else { (bars[i].close - lo) / range * 100.0 });
    }

    // Smooth %K (SMA of raw_k)
    let stoch_k = sma_of_option(&raw_k, k_smooth);
    // %D = SMA of %K
    let stoch_d = sma_of_option(&stoch_k, d_smooth);
    (stoch_k, stoch_d)
}

fn sma_of_option(data: &[Option<f64>], period: usize) -> Vec<Option<f64>> {
    let n = data.len();
    let mut out = vec![None; n];
    let mut sum = 0.0_f64;
    let mut count = 0_usize;
    for i in 0..n {
        if let Some(v) = data[i] {
            sum += v;
            count += 1;
            if count >= period {
                if count > period {
                    // Find the value to subtract (period steps back through valid values)
                    let mut back = 0;
                    let mut found = 0;
                    for j in (0..i).rev() {
                        if data[j].is_some() {
                            found += 1;
                            if found == period {
                                back = j;
                                break;
                            }
                        }
                    }
                    if let Some(old) = data[back] {
                        sum -= old;
                    }
                }
                out[i] = Some(sum / period as f64);
            }
        }
    }
    // Simpler approach: just running SMA over valid values
    let mut out2 = vec![None; n];
    let vals: Vec<(usize, f64)> = data.iter().enumerate().filter_map(|(i, v)| v.map(|x| (i, x))).collect();
    if vals.len() >= period {
        let mut s: f64 = vals[..period].iter().map(|(_, v)| v).sum();
        out2[vals[period - 1].0] = Some(s / period as f64);
        for j in period..vals.len() {
            s += vals[j].1 - vals[j - period].1;
            out2[vals[j].0] = Some(s / period as f64);
        }
    }
    out2
}

fn compute_adx(bars: &[Bar], period: usize) -> (Vec<Option<f64>>, Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = bars.len();
    let mut adx = vec![None; n];
    let mut di_plus = vec![None; n];
    let mut di_minus = vec![None; n];
    if n <= period + 1 { return (adx, di_plus, di_minus); }

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
        if tr < f64::EPSILON { 0.0 } else { 100.0 * dm / tr }
    };

    di_plus[period] = Some(calc_di(sm_pdm, sm_tr));
    di_minus[period] = Some(calc_di(sm_mdm, sm_tr));

    let mut dx_sum = 0.0_f64;
    let di_p = calc_di(sm_pdm, sm_tr);
    let di_m = calc_di(sm_mdm, sm_tr);
    let dx0 = if (di_p + di_m) < f64::EPSILON { 0.0 } else { 100.0 * (di_p - di_m).abs() / (di_p + di_m) };
    dx_sum += dx0;

    for i in (period + 1)..n {
        sm_pdm = sm_pdm - sm_pdm / period as f64 + plus_dm[i];
        sm_mdm = sm_mdm - sm_mdm / period as f64 + minus_dm[i];
        sm_tr = sm_tr - sm_tr / period as f64 + tr_vals[i];

        let dip = calc_di(sm_pdm, sm_tr);
        let dim = calc_di(sm_mdm, sm_tr);
        di_plus[i] = Some(dip);
        di_minus[i] = Some(dim);

        let dx = if (dip + dim) < f64::EPSILON { 0.0 } else { 100.0 * (dip - dim).abs() / (dip + dim) };

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

fn compute_ichimoku(bars: &[Bar], tenkan: usize, kijun: usize, senkou_b: usize) -> (Vec<Option<f64>>, Vec<Option<f64>>, Vec<Option<f64>>, Vec<Option<f64>>) {
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

fn compute_wma(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < period { return out; }
    let denom = (period * (period + 1)) as f64 / 2.0;
    for i in (period - 1)..n {
        let mut sum = 0.0;
        for j in 0..period {
            sum += bars[i - period + 1 + j].close * (j + 1) as f64;
        }
        out[i] = Some(sum / denom);
    }
    out
}

fn compute_hma(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
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
        diff_bars.push(Bar { ts_ms: bars[i].ts_ms, open: close, high: close, low: close, close, volume: 0.0 });
    }
    compute_wma(&diff_bars, sqrt_p.max(1))
}

fn compute_cci(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < period { return out; }
    for i in (period - 1)..n {
        let slice = &bars[(i + 1 - period)..=i];
        let tp: Vec<f64> = slice.iter().map(|b| (b.high + b.low + b.close) / 3.0).collect();
        let mean: f64 = tp.iter().sum::<f64>() / period as f64;
        let md: f64 = tp.iter().map(|v| (v - mean).abs()).sum::<f64>() / period as f64;
        let current_tp = (bars[i].high + bars[i].low + bars[i].close) / 3.0;
        out[i] = if md < f64::EPSILON { Some(0.0) } else { Some((current_tp - mean) / (0.015 * md)) };
    }
    out
}

fn compute_williams_r(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < period { return out; }
    for i in (period - 1)..n {
        let slice = &bars[(i + 1 - period)..=i];
        let hi = slice.iter().map(|b| b.high).fold(f64::MIN, f64::max);
        let lo = slice.iter().map(|b| b.low).fold(f64::MAX, f64::min);
        let range = hi - lo;
        out[i] = if range < f64::EPSILON { Some(-50.0) } else { Some(-100.0 * (hi - bars[i].close) / range) };
    }
    out
}

fn compute_obv(bars: &[Bar]) -> Vec<Option<f64>> {
    let n = bars.len();
    if n == 0 { return vec![]; }
    let mut out = vec![None; n];
    let mut obv = 0.0_f64;
    out[0] = Some(0.0);
    for i in 1..n {
        if bars[i].close > bars[i - 1].close { obv += bars[i].volume; }
        else if bars[i].close < bars[i - 1].close { obv -= bars[i].volume; }
        out[i] = Some(obv);
    }
    out
}

fn compute_momentum(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    for i in period..n {
        out[i] = Some(bars[i].close - bars[i - period].close);
    }
    out
}

fn compute_parabolic_sar(bars: &[Bar], af_step: f64, af_max: f64) -> Vec<Option<f64>> {
    let n = bars.len();
    if n < 2 { return vec![None; n]; }
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
                if bars[i].high > ep { ep = bars[i].high; af = (af + af_step).min(af_max); }
            }
        } else {
            sar = sar.max(bars[i - 1].high).max(bars[i - 2].high);
            if bars[i].high > sar {
                is_long = true;
                sar = ep;
                ep = bars[i].high;
                af = af_step;
            } else {
                if bars[i].low < ep { ep = bars[i].low; af = (af + af_step).min(af_max); }
            }
        }
        out[i] = Some(sar);
    }
    out
}

fn compute_atr_projection(bars: &[Bar], atr: &[Option<f64>]) -> (Vec<Option<f64>>, Vec<Option<f64>>) {
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

fn compute_better_volume(bars: &[Bar]) -> Vec<u8> {
    let n = bars.len();
    if n < 20 { return vec![0; n]; }
    let mut out = vec![0u8; n];
    for i in 20..n {
        let avg_vol: f64 = bars[(i - 20)..i].iter().map(|b| b.volume).sum::<f64>() / 20.0;
        let range = bars[i].high - bars[i].low;
        let avg_range: f64 = bars[(i - 20)..i].iter().map(|b| b.high - b.low).sum::<f64>() / 20.0;
        let vol_ratio = if avg_vol > 0.0 { bars[i].volume / avg_vol } else { 1.0 };
        let range_ratio = if avg_range > 0.0 { range / avg_range } else { 1.0 };
        let is_up = bars[i].close >= bars[i].open;

        if vol_ratio > 2.0 && range_ratio > 1.5 {
            out[i] = if is_up { 1 } else { 2 }; // climax up/down
        } else if vol_ratio > 1.5 && range_ratio < 0.7 {
            out[i] = 5; // churn (high vol, low range)
        } else if vol_ratio > 1.5 {
            out[i] = 3; // high volume
        } else if vol_ratio < 0.5 {
            out[i] = 4; // low volume
        }
    }
    out
}

// ─── chart rendering ─────────────────────────────────────────────────────────

/// Draw a single chart viewport into `rect` using `painter`.
fn draw_chart(
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
    show_better_volume: bool,
    sl_price: Option<f64>,
    tp_price: Option<f64>,
) {
    // ── background ──────────────────────────────────────────────────────────
    painter.rect_filled(rect, 0.0, BG);

    let (start_idx, end_idx) = chart.visible_range();
    let bars = &chart.bars[start_idx..end_idx];

    if bars.is_empty() {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No data — load a symbol",
            egui::FontId::proportional(16.0),
            egui::Color32::from_rgb(100, 100, 120),
        );
        return;
    }

    // Allocate sub-pane space at bottom
    let sub_pane_count = show_rsi as u8 + show_fisher as u8 + show_macd as u8 + show_volume_pane as u8
        + show_stochastic as u8 + show_adx as u8 + show_cci as u8 + show_williams_r as u8
        + show_obv as u8 + show_momentum as u8 + show_better_volume as u8;
    let sub_pane_height = if sub_pane_count > 0 { 80.0 * sub_pane_count as f32 } else { 0.0 };
    let main_rect = egui::Rect::from_min_max(
        rect.min,
        egui::pos2(rect.right(), rect.bottom() - sub_pane_height),
    );

    // Price axis margins
    let price_axis_w = 70.0_f32;
    let time_axis_h  = 22.0_f32;
    let chart_rect = egui::Rect::from_min_max(
        main_rect.min,
        egui::pos2(main_rect.right() - price_axis_w, main_rect.bottom() - time_axis_h),
    );

    // ── price range ─────────────────────────────────────────────────────────
    let mut price_min = bars.iter().map(|b| b.low).fold(f64::MAX, f64::min);
    let mut price_max = bars.iter().map(|b| b.high).fold(f64::MIN, f64::max);

    // Also account for indicator values in visible range
    for i in start_idx..end_idx {
        if flags.sma200 { if let Some(v) = chart.sma200[i] { price_min = price_min.min(v); price_max = price_max.max(v); } }
        if flags.sma100 { if let Some(v) = chart.sma100[i] { price_min = price_min.min(v); price_max = price_max.max(v); } }
        if flags.kama   { if let Some(v) = chart.kama[i]   { price_min = price_min.min(v); price_max = price_max.max(v); } }
        if flags.ema21  { if let Some(v) = chart.ema21[i]  { price_min = price_min.min(v); price_max = price_max.max(v); } }
        if flags.bollinger {
            if let Some(v) = chart.bb_upper[i] { price_max = price_max.max(v); }
            if let Some(v) = chart.bb_lower[i] { price_min = price_min.min(v); }
        }
        if flags.ichimoku {
            if let Some(v) = chart.ichi_span_a[i] { price_min = price_min.min(v); price_max = price_max.max(v); }
            if let Some(v) = chart.ichi_span_b[i] { price_min = price_min.min(v); price_max = price_max.max(v); }
        }
    }

    let padding = (price_max - price_min) * 0.05;
    price_min -= padding;
    price_max += padding;

    // Vertical pan + zoom
    let range = price_max - price_min;
    let centre = (price_max + price_min) * 0.5 + chart.price_pan;
    let half   = range * 0.5 / chart.price_zoom;
    price_min = centre - half;
    price_max = centre + half;

    if (price_max - price_min).abs() < f64::EPSILON { return; }

    let price_to_y = |p: f64| -> f32 {
        let frac = (price_max - p) / (price_max - price_min);
        chart_rect.top() + frac as f32 * chart_rect.height()
    };

    // ── bar width ────────────────────────────────────────────────────────────
    let n_bars    = bars.len() as f32;
    let bar_w     = (chart_rect.width() / n_bars).max(1.0);
    let candle_w  = (bar_w * 0.7).max(1.0);
    let half_body = candle_w * 0.5;

    // ── grid lines (price) ───────────────────────────────────────────────────
    let grid_steps = 8;
    for i in 0..=grid_steps {
        let p   = price_min + (price_max - price_min) * (i as f64 / grid_steps as f64);
        let y   = price_to_y(p);
        painter.line_segment(
            [egui::pos2(chart_rect.left(), y), egui::pos2(chart_rect.right(), y)],
            egui::Stroke::new(0.5, GRID),
        );
        let label = format_price(p);
        painter.text(
            egui::pos2(chart_rect.right() + 4.0, y),
            egui::Align2::LEFT_CENTER,
            &label,
            egui::FontId::monospace(10.0),
            AXIS_TEXT,
        );
    }

    // ── grid lines (time) ───────────────────────────────────────────────────
    let time_step = ((80.0 / bar_w) as usize).max(1);
    for (rel_idx, bar) in bars.iter().enumerate() {
        if rel_idx % time_step != 0 { continue; }
        let x = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
        painter.line_segment(
            [egui::pos2(x, chart_rect.top()), egui::pos2(x, chart_rect.bottom())],
            egui::Stroke::new(0.5, GRID),
        );
        let label = format_ts(bar.ts_ms, chart.timeframe);
        painter.text(
            egui::pos2(x, chart_rect.bottom() + 2.0),
            egui::Align2::CENTER_TOP,
            &label,
            egui::FontId::monospace(9.0),
            AXIS_TEXT,
        );
    }

    // ── Bollinger Band fill ──────────────────────────────────────────────────
    if flags.bollinger {
        let mut fill_points_upper: Vec<egui::Pos2> = Vec::new();
        let mut fill_points_lower: Vec<egui::Pos2> = Vec::new();
        for (rel_idx, _) in bars.iter().enumerate() {
            let abs_idx = start_idx + rel_idx;
            if abs_idx >= chart.bb_upper.len() { continue; }
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
        // Draw fill as a polygon: upper forward + lower reversed
        if fill_points_upper.len() > 1 {
            let mut poly = fill_points_upper.clone();
            poly.extend(fill_points_lower.iter().rev());
            painter.add(egui::Shape::convex_polygon(poly, BB_FILL, egui::Stroke::NONE));
        }
        draw_indicator_line(painter, chart_rect, bars, &chart.bb_upper, start_idx, bar_w, &price_to_y, BB_COL, 1.0);
        draw_indicator_line(painter, chart_rect, bars, &chart.bb_lower, start_idx, bar_w, &price_to_y, BB_COL, 1.0);
        draw_indicator_line(painter, chart_rect, bars, &chart.bb_mid,   start_idx, bar_w, &price_to_y, BB_COL, 0.5);
    }

    // ── Ichimoku cloud ─────────────────────────────────────────────────────
    if flags.ichimoku {
        // Cloud fill between Span A and Span B
        for (rel_idx, _) in bars.iter().enumerate() {
            let abs_idx = start_idx + rel_idx;
            if abs_idx >= chart.ichi_span_a.len() { continue; }
            if let (Some(a), Some(b)) = (chart.ichi_span_a[abs_idx], chart.ichi_span_b[abs_idx]) {
                let x = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
                let ya = price_to_y(a);
                let yb = price_to_y(b);
                let color = if a >= b { ICHI_CLOUD_BULL } else { ICHI_CLOUD_BEAR };
                let (top, bot) = if ya < yb { (ya, yb) } else { (yb, ya) };
                if top <= chart_rect.bottom() && bot >= chart_rect.top() {
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(x - bar_w * 0.5, top.max(chart_rect.top())),
                            egui::pos2(x + bar_w * 0.5, bot.min(chart_rect.bottom())),
                        ),
                        0.0, color,
                    );
                }
            }
        }
        draw_indicator_line(painter, chart_rect, bars, &chart.ichi_tenkan, start_idx, bar_w, &price_to_y, ICHI_TENKAN, 1.0);
        draw_indicator_line(painter, chart_rect, bars, &chart.ichi_kijun,  start_idx, bar_w, &price_to_y, ICHI_KIJUN,  1.0);
        draw_indicator_line(painter, chart_rect, bars, &chart.ichi_span_a, start_idx, bar_w, &price_to_y, ICHI_SPAN_A, 0.8);
        draw_indicator_line(painter, chart_rect, bars, &chart.ichi_span_b, start_idx, bar_w, &price_to_y, ICHI_SPAN_B, 0.8);
    }

    // ── indicator lines ──────────────────────────────────────────────────────
    if flags.sma200 { draw_indicator_line(painter, chart_rect, bars, &chart.sma200, start_idx, bar_w, &price_to_y, SMA200_COL, 1.5); }
    if flags.sma100 { draw_indicator_line(painter, chart_rect, bars, &chart.sma100, start_idx, bar_w, &price_to_y, SMA100_COL, 1.5); }
    if flags.kama   { draw_indicator_line(painter, chart_rect, bars, &chart.kama,   start_idx, bar_w, &price_to_y, KAMA_COL,   1.5); }
    if flags.ema21  { draw_indicator_line(painter, chart_rect, bars, &chart.ema21,  start_idx, bar_w, &price_to_y, EMA_COL,    1.5); }
    if flags.wma    { draw_indicator_line(painter, chart_rect, bars, &chart.wma,    start_idx, bar_w, &price_to_y, WMA_COL,    1.0); }
    if flags.hma    { draw_indicator_line(painter, chart_rect, bars, &chart.hma,    start_idx, bar_w, &price_to_y, HMA_COL,    1.5); }

    // ATR Projection bands
    if flags.atr_proj {
        draw_indicator_line(painter, chart_rect, bars, &chart.atr_proj_upper, start_idx, bar_w, &price_to_y, ATR_PROJ_COL, 1.0);
        draw_indicator_line(painter, chart_rect, bars, &chart.atr_proj_lower, start_idx, bar_w, &price_to_y, ATR_PROJ_COL, 1.0);
    }

    // Parabolic SAR dots
    if flags.psar {
        for (rel_idx, _) in bars.iter().enumerate() {
            let abs_idx = start_idx + rel_idx;
            if abs_idx >= chart.psar.len() { continue; }
            if let Some(sar) = chart.psar[abs_idx] {
                let x = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
                let y = price_to_y(sar);
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    painter.circle_filled(egui::pos2(x, y), 2.0, SAR_COL);
                }
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
            // Line chart: polyline through close prices
            let mut points: Vec<egui::Pos2> = Vec::with_capacity(bars.len());
            for (rel_idx, bar) in bars.iter().enumerate() {
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
            for (rel_idx, bar) in bars.iter().enumerate() {
                let cx = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
                let y_open  = price_to_y(bar.open);
                let y_high  = price_to_y(bar.high);
                let y_low   = price_to_y(bar.low);
                let y_close = price_to_y(bar.close);
                let color = if bar.close >= bar.open { UP } else { DOWN };
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
            for (rel_idx, bar) in render_bars.iter().enumerate() {
                let cx = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
                let y_open  = price_to_y(bar.open);
                let y_high  = price_to_y(bar.high);
                let y_low   = price_to_y(bar.low);
                let y_close = price_to_y(bar.close);
                let color = if bar.close >= bar.open { UP } else { DOWN };

                // Wick
                painter.line_segment(
                    [egui::pos2(cx, y_high), egui::pos2(cx, y_low)],
                    egui::Stroke::new(1.0, color),
                );

                // Body
                let body_top    = y_open.min(y_close);
                let body_bottom = y_open.max(y_close);
                let body_height = (body_bottom - body_top).max(1.0);
                let body_rect = egui::Rect::from_min_size(
                    egui::pos2(cx - half_body, body_top),
                    egui::vec2(candle_w, body_height),
                );

                if body_height > 2.0 {
                    let fill = egui::Color32::from_rgba_premultiplied(
                        color.r(), color.g(), color.b(), 220,
                    );
                    painter.rect_filled(body_rect, 0.0, fill);
                    painter.rect_stroke(body_rect, 0.0, egui::Stroke::new(0.5, color), egui::StrokeKind::Outside);
                } else {
                    painter.line_segment(
                        [egui::pos2(cx - half_body, body_top), egui::pos2(cx + half_body, body_top)],
                        egui::Stroke::new(1.0, color),
                    );
                }
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

    // ── crosshair ────────────────────────────────────────────────────────────
    if let Some(pos) = crosshair {
        if chart_rect.contains(pos) {
            let ch_color = egui::Color32::from_rgba_premultiplied(180, 180, 200, 100);
            painter.line_segment(
                [egui::pos2(pos.x, chart_rect.top()), egui::pos2(pos.x, chart_rect.bottom())],
                egui::Stroke::new(0.5, ch_color),
            );
            painter.line_segment(
                [egui::pos2(chart_rect.left(), pos.y), egui::pos2(chart_rect.right(), pos.y)],
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

            // OHLCV + indicator values tooltip for nearest bar
            let rel_x = pos.x - chart_rect.left();
            let bar_idx = ((rel_x / bar_w) as usize).min(bars.len().saturating_sub(1));
            if bar_idx < bars.len() {
                let b = &bars[bar_idx];
                let abs_idx = start_idx + bar_idx;
                let tooltip = format!(
                    "O:{} H:{} L:{} C:{} V:{:.0}",
                    format_price(b.open), format_price(b.high),
                    format_price(b.low),  format_price(b.close), b.volume
                );
                painter.text(
                    egui::pos2(chart_rect.left() + 6.0, chart_rect.top() + 4.0),
                    egui::Align2::LEFT_TOP,
                    &tooltip,
                    egui::FontId::monospace(11.0),
                    egui::Color32::from_rgb(220, 220, 255),
                );

                // Indicator values on second line
                let mut ind_parts: Vec<String> = Vec::new();
                if flags.sma200 { if let Some(Some(v)) = chart.sma200.get(abs_idx) { ind_parts.push(format!("SMA200:{}", format_price(*v))); } }
                if flags.sma100 { if let Some(Some(v)) = chart.sma100.get(abs_idx) { ind_parts.push(format!("SMA100:{}", format_price(*v))); } }
                if flags.kama   { if let Some(Some(v)) = chart.kama.get(abs_idx)   { ind_parts.push(format!("KAMA:{}", format_price(*v))); } }
                if flags.ema21  { if let Some(Some(v)) = chart.ema21.get(abs_idx)  { ind_parts.push(format!("EMA21:{}", format_price(*v))); } }
                if show_rsi     { if let Some(Some(v)) = chart.rsi.get(abs_idx)    { ind_parts.push(format!("RSI:{:.1}", v)); } }
                if let Some(Some(v)) = chart.atr.get(abs_idx)                      { ind_parts.push(format!("ATR:{}", format_price(*v))); }
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

    // ── symbol / tf label ────────────────────────────────────────────────────
    painter.text(
        egui::pos2(chart_rect.left() + 8.0, chart_rect.top() + 6.0),
        egui::Align2::LEFT_TOP,
        &format!("{} [{}]", chart.symbol, chart.timeframe.label()),
        egui::FontId::proportional(13.0),
        egui::Color32::from_rgb(200, 200, 220),
    );

    // ── indicator legend ─────────────────────────────────────────────────────
    let ly = chart_rect.top() + 34.0;
    let mut lx = chart_rect.left() + 8.0;
    if flags.sma200 {
        painter.text(egui::pos2(lx, ly), egui::Align2::LEFT_TOP, "SMA200", egui::FontId::monospace(10.0), SMA200_COL);
        lx += 57.0;
    }
    if flags.sma100 {
        painter.text(egui::pos2(lx, ly), egui::Align2::LEFT_TOP, "SMA100", egui::FontId::monospace(10.0), SMA100_COL);
        lx += 57.0;
    }
    if flags.kama {
        painter.text(egui::pos2(lx, ly), egui::Align2::LEFT_TOP, "KAMA(10,2,30)", egui::FontId::monospace(10.0), KAMA_COL);
        lx += 110.0;
    }
    if flags.ema21 {
        painter.text(egui::pos2(lx, ly), egui::Align2::LEFT_TOP, "EMA21", egui::FontId::monospace(10.0), EMA_COL);
        lx += 50.0;
    }
    if flags.bollinger {
        painter.text(egui::pos2(lx, ly), egui::Align2::LEFT_TOP, "BB(20,2)", egui::FontId::monospace(10.0), BB_COL);
    }

    // ── sub-panes (RSI, Fisher) ──────────────────────────────────────────────
    let mut sub_y = main_rect.bottom();

    if show_rsi {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(painter, pane_rect, bars, &chart.rsi, start_idx, bar_w, "RSI(14)", RSI_LINE, 0.0, 100.0, Some(70.0), Some(30.0));
        sub_y += 80.0;
    }

    if show_fisher {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_fisher_pane(painter, pane_rect, bars, &chart.fisher, &chart.fisher_signal, start_idx, bar_w);
        sub_y += 80.0;
    }

    if show_macd {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_macd_pane(painter, pane_rect, bars, &chart.macd_line, &chart.macd_signal, &chart.macd_hist, start_idx, bar_w);
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
        draw_stoch_pane(painter, pane_rect, bars, &chart.stoch_k, &chart.stoch_d, start_idx, bar_w);
        sub_y += 80.0;
    }

    if show_adx {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_adx_pane(painter, pane_rect, bars, &chart.adx, &chart.di_plus, &chart.di_minus, start_idx, bar_w);
        sub_y += 80.0;
    }

    if show_cci {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(painter, pane_rect, bars, &chart.cci, start_idx, bar_w, "CCI(20)", CCI_COL, -200.0, 200.0, Some(100.0), Some(-100.0));
        sub_y += 80.0;
    }

    if show_williams_r {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_oscillator_pane(painter, pane_rect, bars, &chart.williams_r, start_idx, bar_w, "Williams %R(14)", WILLR_COL, -100.0, 0.0, Some(-20.0), Some(-80.0));
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
            if let Some(Some(v)) = chart.obv.get(start_idx + ri) { ob_min = ob_min.min(*v); ob_max = ob_max.max(*v); }
        }
        let pad = (ob_max - ob_min) * 0.1;
        draw_oscillator_pane(painter, pane_rect, bars, &chart.obv, start_idx, bar_w, "OBV", OBV_COL, ob_min - pad, ob_max + pad, None, None);
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
            if let Some(Some(v)) = chart.momentum.get(start_idx + ri) { m_min = m_min.min(*v); m_max = m_max.max(*v); }
        }
        let pad = (m_max - m_min).max(0.001) * 0.1;
        draw_oscillator_pane(painter, pane_rect, bars, &chart.momentum, start_idx, bar_w, "Momentum(10)", egui::Color32::from_rgb(200, 150, 100), m_min - pad, m_max + pad, None, None);
        sub_y += 80.0;
    }

    if show_better_volume {
        let pane_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), sub_y),
            egui::pos2(rect.right() - price_axis_w, sub_y + 80.0),
        );
        draw_better_volume_pane(painter, pane_rect, bars, &chart.better_vol_type, start_idx, bar_w);
    }

    // ── SL/TP planning lines ───────────────────────────────────────────────
    for (price_opt, label, color) in [
        (&sl_price, "SL", egui::Color32::from_rgb(220, 40, 40)),
        (&tp_price, "TP", egui::Color32::from_rgb(0, 200, 80)),
    ] {
        if let Some(p) = price_opt {
            let y = price_to_y(*p);
            if y >= chart_rect.top() && y <= chart_rect.bottom() {
                // Dashed line
                let dash = 4.0_f32;
                let mut x = chart_rect.left();
                while x < chart_rect.right() {
                    let end = (x + dash).min(chart_rect.right());
                    painter.line_segment([egui::pos2(x, y), egui::pos2(end, y)], egui::Stroke::new(1.5, color));
                    x += dash * 2.0;
                }
                // Label
                let lbl = format!("{} {}", label, format_price(*p));
                painter.text(egui::pos2(chart_rect.left() + 4.0, y - 12.0), egui::Align2::LEFT_TOP, &lbl, egui::FontId::monospace(10.0), color);
                // P&L from last price
                if let Some(last) = bars.last() {
                    let dist = *p - last.close;
                    let pips = format_price(dist.abs());
                    let dir = if dist > 0.0 { "+" } else { "-" };
                    painter.text(
                        egui::pos2(chart_rect.right() - 100.0, y - 12.0),
                        egui::Align2::LEFT_TOP,
                        &format!("{}{}", dir, pips),
                        egui::FontId::monospace(9.0),
                        color,
                    );
                }
            }
        }
    }

    // ── drawing annotations ──────────────────────────────────────────────────
    for drawing in &chart.drawings {
        match drawing {
            Drawing::HLine { price, color } => {
                let y = price_to_y(*price);
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    painter.line_segment(
                        [egui::pos2(chart_rect.left(), y), egui::pos2(chart_rect.right(), y)],
                        egui::Stroke::new(1.0, *color),
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
                } else { None };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx {
                    Some(chart_rect.left() + ((p2.0 - start_idx) as f32 + 0.5) * bar_w)
                } else { None };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    painter.line_segment(
                        [egui::pos2(x1, y1), egui::pos2(x2, y2)],
                        egui::Stroke::new(1.5, *color),
                    );
                }
            }
            Drawing::FiboRetrace { high, low, bar_start, bar_end } => {
                let x_start = if *bar_start >= start_idx && *bar_start < end_idx {
                    chart_rect.left() + ((*bar_start - start_idx) as f32 + 0.5) * bar_w
                } else { chart_rect.left() };
                let x_end = if *bar_end >= start_idx && *bar_end < end_idx {
                    chart_rect.left() + ((*bar_end - start_idx) as f32 + 0.5) * bar_w
                } else { chart_rect.right() };
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
                    painter.line_segment(
                        [egui::pos2(x, chart_rect.top()), egui::pos2(x, chart_rect.bottom())],
                        egui::Stroke::new(1.0, *color),
                    );
                }
            }
            Drawing::Rectangle { p1, p2, color } => {
                let x1 = if p1.0 >= start_idx && p1.0 < end_idx { Some(chart_rect.left() + ((p1.0 - start_idx) as f32 + 0.5) * bar_w) } else { None };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx { Some(chart_rect.left() + ((p2.0 - start_idx) as f32 + 0.5) * bar_w) } else { None };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    let r = egui::Rect::from_two_pos(egui::pos2(x1, y1), egui::pos2(x2, y2));
                    painter.rect_filled(r, 0.0, *color);
                    painter.rect_stroke(r, 0.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(color.r(), color.g(), color.b())), egui::StrokeKind::Outside);
                }
            }
            Drawing::Ray { origin, slope, color } => {
                if origin.0 >= start_idx && origin.0 < end_idx {
                    let x1 = chart_rect.left() + ((origin.0 - start_idx) as f32 + 0.5) * bar_w;
                    let y1 = price_to_y(origin.1);
                    let bars_to_edge = ((chart_rect.right() - x1) / bar_w) as f64;
                    let end_price = origin.1 + slope * bars_to_edge;
                    let y2 = price_to_y(end_price);
                    painter.line_segment([egui::pos2(x1, y1), egui::pos2(chart_rect.right(), y2)], egui::Stroke::new(1.5, *color));
                }
            }
            Drawing::Channel { p1, p2, width, color } => {
                let x1 = if p1.0 >= start_idx && p1.0 < end_idx { Some(chart_rect.left() + ((p1.0 - start_idx) as f32 + 0.5) * bar_w) } else { None };
                let x2 = if p2.0 >= start_idx && p2.0 < end_idx { Some(chart_rect.left() + ((p2.0 - start_idx) as f32 + 0.5) * bar_w) } else { None };
                if let (Some(x1), Some(x2)) = (x1, x2) {
                    let y1 = price_to_y(p1.1);
                    let y2 = price_to_y(p2.1);
                    let y1b = price_to_y(p1.1 + width);
                    let y2b = price_to_y(p2.1 + width);
                    painter.line_segment([egui::pos2(x1, y1), egui::pos2(x2, y2)], egui::Stroke::new(1.5, *color));
                    painter.line_segment([egui::pos2(x1, y1b), egui::pos2(x2, y2b)], egui::Stroke::new(1.0, *color));
                    // Fill between
                    let fill = egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 20);
                    let poly = vec![egui::pos2(x1, y1), egui::pos2(x2, y2), egui::pos2(x2, y2b), egui::pos2(x1, y1b)];
                    painter.add(egui::Shape::convex_polygon(poly, fill, egui::Stroke::NONE));
                }
            }
        }
    }
}

/// Draw an oscillator sub-pane (RSI, etc.) with optional overbought/oversold levels.
fn draw_oscillator_pane(
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
    painter.rect_stroke(rect, 0.0, egui::Stroke::new(0.5, GRID), egui::StrokeKind::Outside);

    let val_to_y = |v: f64| -> f32 {
        let frac = (val_max - v) / (val_max - val_min);
        rect.top() + frac as f32 * rect.height()
    };

    // OB/OS levels
    let level_color = egui::Color32::from_rgba_premultiplied(140, 140, 160, 60);
    if let Some(ob) = ob_level {
        let y = val_to_y(ob);
        painter.line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)], egui::Stroke::new(0.5, level_color));
    }
    if let Some(os) = os_level {
        let y = val_to_y(os);
        painter.line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)], egui::Stroke::new(0.5, level_color));
    }
    // Mid line
    let mid_y = val_to_y((val_max + val_min) / 2.0);
    painter.line_segment([egui::pos2(rect.left(), mid_y), egui::pos2(rect.right(), mid_y)], egui::Stroke::new(0.3, GRID));

    // Data line
    let mut points: Vec<egui::Pos2> = Vec::with_capacity(bars.len());
    for (rel_idx, _) in bars.iter().enumerate() {
        let abs_idx = start_idx + rel_idx;
        if abs_idx >= series.len() { continue; }
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
    painter.text(egui::pos2(rect.left() + 4.0, rect.top() + 2.0), egui::Align2::LEFT_TOP, label, egui::FontId::monospace(9.0), color);
}

/// Draw Fisher Transform sub-pane with color-coded histogram bars.
fn draw_fisher_pane(
    painter: &egui::Painter,
    rect: egui::Rect,
    bars: &[Bar],
    fisher: &[Option<f64>],
    signal: &[Option<f64>],
    start_idx: usize,
    bar_w: f32,
) {
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(0, 0, 0));
    painter.rect_stroke(rect, 0.0, egui::Stroke::new(0.5, GRID), egui::StrokeKind::Outside);

    // Fisher typically ranges -3..3, auto-scale
    let mut f_min = -2.0_f64;
    let mut f_max = 2.0_f64;
    for (rel_idx, _) in bars.iter().enumerate() {
        let abs_idx = start_idx + rel_idx;
        if abs_idx >= fisher.len() { continue; }
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

    // Zero line
    let zero_y = val_to_y(0.0);
    painter.line_segment([egui::pos2(rect.left(), zero_y), egui::pos2(rect.right(), zero_y)], egui::Stroke::new(0.5, GRID));

    // Histogram bars
    let hist_w = (bar_w * 0.6).max(1.0);
    for (rel_idx, _) in bars.iter().enumerate() {
        let abs_idx = start_idx + rel_idx;
        if abs_idx >= fisher.len() { continue; }
        if let Some(v) = fisher[abs_idx] {
            let x = rect.left() + (rel_idx as f32 + 0.5) * bar_w;
            let y = val_to_y(v);
            let color = if v >= 0.0 { FISHER_POS } else { FISHER_NEG };
            let (top, bottom) = if v >= 0.0 { (y, zero_y) } else { (zero_y, y) };
            painter.rect_filled(
                egui::Rect::from_min_max(egui::pos2(x - hist_w / 2.0, top), egui::pos2(x + hist_w / 2.0, bottom)),
                0.0,
                egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 180),
            );
        }
    }

    // Signal line
    let mut points: Vec<egui::Pos2> = Vec::with_capacity(bars.len());
    for (rel_idx, _) in bars.iter().enumerate() {
        let abs_idx = start_idx + rel_idx;
        if abs_idx >= signal.len() { continue; }
        if let Some(v) = signal[abs_idx] {
            let x = rect.left() + (rel_idx as f32 + 0.5) * bar_w;
            let y = val_to_y(v).clamp(rect.top(), rect.bottom());
            points.push(egui::pos2(x, y));
        }
    }
    if points.len() > 1 {
        painter.add(egui::Shape::line(points, egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 255, 100))));
    }

    painter.text(egui::pos2(rect.left() + 4.0, rect.top() + 2.0), egui::Align2::LEFT_TOP, "Fisher(10)", egui::FontId::monospace(9.0), FISHER_POS);
}

/// Draw MACD sub-pane with two lines + histogram.
fn draw_macd_pane(
    painter: &egui::Painter,
    rect: egui::Rect,
    bars: &[Bar],
    macd_line: &[Option<f64>],
    macd_signal: &[Option<f64>],
    macd_hist: &[Option<f64>],
    start_idx: usize,
    bar_w: f32,
) {
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(0, 0, 0));
    painter.rect_stroke(rect, 0.0, egui::Stroke::new(0.5, GRID), egui::StrokeKind::Outside);

    // Auto-scale
    let mut v_min = 0.0_f64;
    let mut v_max = 0.0_f64;
    for (rel_idx, _) in bars.iter().enumerate() {
        let abs_idx = start_idx + rel_idx;
        if abs_idx >= macd_line.len() { continue; }
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

    // Zero line
    let zero_y = val_to_y(0.0);
    painter.line_segment([egui::pos2(rect.left(), zero_y), egui::pos2(rect.right(), zero_y)], egui::Stroke::new(0.3, GRID));

    // Histogram
    let hist_w = (bar_w * 0.6).max(1.0);
    for (rel_idx, _) in bars.iter().enumerate() {
        let abs_idx = start_idx + rel_idx;
        if abs_idx >= macd_hist.len() { continue; }
        if let Some(v) = macd_hist[abs_idx] {
            let x = rect.left() + (rel_idx as f32 + 0.5) * bar_w;
            let y = val_to_y(v);
            let color = if v >= 0.0 { FISHER_POS } else { FISHER_NEG };
            let (top, bottom) = if v >= 0.0 { (y, zero_y) } else { (zero_y, y) };
            painter.rect_filled(
                egui::Rect::from_min_max(egui::pos2(x - hist_w / 2.0, top), egui::pos2(x + hist_w / 2.0, bottom)),
                0.0,
                egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 120),
            );
        }
    }

    // MACD line
    let mut points: Vec<egui::Pos2> = Vec::new();
    for (rel_idx, _) in bars.iter().enumerate() {
        let abs_idx = start_idx + rel_idx;
        if let Some(Some(v)) = macd_line.get(abs_idx) {
            points.push(egui::pos2(rect.left() + (rel_idx as f32 + 0.5) * bar_w, val_to_y(*v).clamp(rect.top(), rect.bottom())));
        }
    }
    if points.len() > 1 { painter.add(egui::Shape::line(points, egui::Stroke::new(1.5, MACD_LINE_COL))); }

    // Signal line
    let mut points: Vec<egui::Pos2> = Vec::new();
    for (rel_idx, _) in bars.iter().enumerate() {
        let abs_idx = start_idx + rel_idx;
        if let Some(Some(v)) = macd_signal.get(abs_idx) {
            points.push(egui::pos2(rect.left() + (rel_idx as f32 + 0.5) * bar_w, val_to_y(*v).clamp(rect.top(), rect.bottom())));
        }
    }
    if points.len() > 1 { painter.add(egui::Shape::line(points, egui::Stroke::new(1.0, MACD_SIG_COL))); }

    painter.text(egui::pos2(rect.left() + 4.0, rect.top() + 2.0), egui::Align2::LEFT_TOP, "MACD(12,26,9)", egui::FontId::monospace(9.0), MACD_LINE_COL);
}

/// Draw volume bars sub-pane.
fn draw_volume_pane(
    painter: &egui::Painter,
    rect: egui::Rect,
    bars: &[Bar],
    _start_idx: usize,
    bar_w: f32,
) {
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(0, 0, 0));
    painter.rect_stroke(rect, 0.0, egui::Stroke::new(0.5, GRID), egui::StrokeKind::Outside);

    if bars.is_empty() { return; }
    let max_vol = bars.iter().map(|b| b.volume).fold(0.0_f64, f64::max);
    if max_vol <= 0.0 { return; }

    let hist_w = (bar_w * 0.7).max(1.0);
    for (rel_idx, b) in bars.iter().enumerate() {
        let x = rect.left() + (rel_idx as f32 + 0.5) * bar_w;
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

    painter.text(egui::pos2(rect.left() + 4.0, rect.top() + 2.0), egui::Align2::LEFT_TOP, "Volume", egui::FontId::monospace(9.0), AXIS_TEXT);
}

/// Draw Better Volume sub-pane (NNFX-style color-coded volume).
fn draw_better_volume_pane(
    painter: &egui::Painter,
    rect: egui::Rect,
    bars: &[Bar],
    vol_type: &[u8],
    start_idx: usize,
    bar_w: f32,
) {
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(0, 0, 0));
    painter.rect_stroke(rect, 0.0, egui::Stroke::new(0.5, GRID), egui::StrokeKind::Outside);

    if bars.is_empty() { return; }
    let max_vol = bars.iter().map(|b| b.volume).fold(0.0_f64, f64::max);
    if max_vol <= 0.0 { return; }

    let hist_w = (bar_w * 0.7).max(1.0);
    for (rel_idx, b) in bars.iter().enumerate() {
        let abs_idx = start_idx + rel_idx;
        let x = rect.left() + (rel_idx as f32 + 0.5) * bar_w;
        let h = (b.volume / max_vol) as f32 * rect.height();
        let vt = vol_type.get(abs_idx).copied().unwrap_or(0);
        let color = match vt {
            1 => BVOL_CLIMAX_UP,
            2 => BVOL_CLIMAX_DN,
            3 => BVOL_HIGH,
            4 => BVOL_LOW,
            5 => BVOL_CHURN,
            _ => if b.close >= b.open {
                egui::Color32::from_rgba_premultiplied(0, 150, 60, 140)
            } else {
                egui::Color32::from_rgba_premultiplied(150, 30, 30, 140)
            },
        };
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(x - hist_w / 2.0, rect.bottom() - h),
                egui::pos2(x + hist_w / 2.0, rect.bottom()),
            ),
            0.0, color,
        );
    }
    painter.text(egui::pos2(rect.left() + 4.0, rect.top() + 2.0), egui::Align2::LEFT_TOP, "BetterVolume", egui::FontId::monospace(9.0), BVOL_HIGH);
}

/// Draw Stochastic sub-pane.
fn draw_stoch_pane(
    painter: &egui::Painter,
    rect: egui::Rect,
    bars: &[Bar],
    stoch_k: &[Option<f64>],
    stoch_d: &[Option<f64>],
    start_idx: usize,
    bar_w: f32,
) {
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(0, 0, 0));
    painter.rect_stroke(rect, 0.0, egui::Stroke::new(0.5, GRID), egui::StrokeKind::Outside);

    let val_to_y = |v: f64| -> f32 {
        let frac = (100.0 - v) / 100.0;
        rect.top() + frac as f32 * rect.height()
    };

    // OB/OS levels
    let level_col = egui::Color32::from_rgba_premultiplied(140, 140, 160, 60);
    for &lvl in &[80.0, 20.0] {
        let y = val_to_y(lvl);
        painter.line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)], egui::Stroke::new(0.5, level_col));
    }

    // %K line
    let mut points: Vec<egui::Pos2> = Vec::new();
    for (rel_idx, _) in bars.iter().enumerate() {
        if let Some(Some(v)) = stoch_k.get(start_idx + rel_idx) {
            points.push(egui::pos2(rect.left() + (rel_idx as f32 + 0.5) * bar_w, val_to_y(*v).clamp(rect.top(), rect.bottom())));
        }
    }
    if points.len() > 1 { painter.add(egui::Shape::line(points, egui::Stroke::new(1.5, STOCH_K_COL))); }

    // %D line
    let mut points: Vec<egui::Pos2> = Vec::new();
    for (rel_idx, _) in bars.iter().enumerate() {
        if let Some(Some(v)) = stoch_d.get(start_idx + rel_idx) {
            points.push(egui::pos2(rect.left() + (rel_idx as f32 + 0.5) * bar_w, val_to_y(*v).clamp(rect.top(), rect.bottom())));
        }
    }
    if points.len() > 1 { painter.add(egui::Shape::line(points, egui::Stroke::new(1.0, STOCH_D_COL))); }

    painter.text(egui::pos2(rect.left() + 4.0, rect.top() + 2.0), egui::Align2::LEFT_TOP, "Stoch(14,3,3)", egui::FontId::monospace(9.0), STOCH_K_COL);
}

/// Draw ADX + DI+/DI- sub-pane.
fn draw_adx_pane(
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
    painter.rect_stroke(rect, 0.0, egui::Stroke::new(0.5, GRID), egui::StrokeKind::Outside);

    // Auto-scale 0-60
    let val_to_y = |v: f64| -> f32 {
        let frac = (60.0 - v) / 60.0;
        rect.top() + frac as f32 * rect.height()
    };

    // Reference line at 25
    let y25 = val_to_y(25.0);
    painter.line_segment([egui::pos2(rect.left(), y25), egui::pos2(rect.right(), y25)],
        egui::Stroke::new(0.5, egui::Color32::from_rgba_premultiplied(140, 140, 160, 60)));

    for (series, color, width) in [(adx, ADX_COL, 1.5_f32), (di_plus, DI_PLUS_COL, 1.0), (di_minus, DI_MINUS_COL, 1.0)] {
        let mut points: Vec<egui::Pos2> = Vec::new();
        for (rel_idx, _) in bars.iter().enumerate() {
            if let Some(Some(v)) = series.get(start_idx + rel_idx) {
                points.push(egui::pos2(rect.left() + (rel_idx as f32 + 0.5) * bar_w, val_to_y(*v).clamp(rect.top(), rect.bottom())));
            }
        }
        if points.len() > 1 { painter.add(egui::Shape::line(points, egui::Stroke::new(width, color))); }
    }

    painter.text(egui::pos2(rect.left() + 4.0, rect.top() + 2.0), egui::Align2::LEFT_TOP, "ADX(14)", egui::FontId::monospace(9.0), ADX_COL);
}

/// Render a single indicator series as a polyline.
fn draw_indicator_line(
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
    let mut points: Vec<egui::Pos2> = Vec::with_capacity(bars.len());
    for (rel_idx, _bar) in bars.iter().enumerate() {
        let abs_idx = start_idx + rel_idx;
        if abs_idx >= series.len() { continue; }
        if let Some(v) = series[abs_idx] {
            let x = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
            let y = price_to_y(v);
            if y >= chart_rect.top() && y <= chart_rect.bottom() {
                points.push(egui::pos2(x, y));
            } else if !points.is_empty() {
                if points.len() > 1 {
                    painter.add(egui::Shape::line(points.clone(), egui::Stroke::new(width, color)));
                }
                points.clear();
            }
        } else if !points.is_empty() {
            if points.len() > 1 {
                painter.add(egui::Shape::line(points.clone(), egui::Stroke::new(width, color)));
            }
            points.clear();
        }
    }
    if points.len() > 1 {
        painter.add(egui::Shape::line(points, egui::Stroke::new(width, color)));
    }
}

// ─── helpers ─────────────────────────────────────────────────────────────────

fn parse_range(s: &str, default_lo: usize, default_hi: usize) -> (usize, usize) {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() == 2 {
        let lo = parts[0].trim().parse().unwrap_or(default_lo);
        let hi = parts[1].trim().parse().unwrap_or(default_hi);
        (lo, hi)
    } else {
        (default_lo, default_hi)
    }
}

fn format_price(p: f64) -> String {
    if p == 0.0 { return "0".into(); }
    let abs = p.abs();
    if abs >= 10_000.0 { format!("{:.2}", p) }
    else if abs >= 1.0  { format!("{:.4}", p) }
    else                { format!("{:.6}", p) }
}

fn format_ts(ts_ms: i64, tf: Timeframe) -> String {
    use chrono::TimeZone;
    let dt = chrono::Utc.timestamp_millis_opt(ts_ms).single().unwrap_or_default();
    match tf {
        Timeframe::MN1 => dt.format("%b'%y").to_string(),
        Timeframe::W1  => dt.format("%d %b").to_string(),
        Timeframe::D1  => dt.format("%d %b").to_string(),
        Timeframe::H4 | Timeframe::H1 => {
            // Show date + time for H1/H4
            if dt.format("%H").to_string() == "00" {
                dt.format("%d %b").to_string()
            } else {
                dt.format("%H:%M").to_string()
            }
        }
        _              => dt.format("%H:%M").to_string(),
    }
}

// ─── command palette ─────────────────────────────────────────────────────────

struct Command {
    name: &'static str,
    desc: &'static str,
}

const COMMANDS: &[Command] = &[
    // Core
    Command { name: "CONNECT",       desc: "Connect to broker (Alpaca / MT5)" },
    Command { name: "SETTINGS",      desc: "Application settings" },
    Command { name: "RELOAD",        desc: "Reload bars from cache" },
    Command { name: "QUIT",          desc: "Exit the application" },
    // View
    Command { name: "MTF",           desc: "Toggle multi-timeframe grid" },
    Command { name: "INDICATORS",    desc: "Toggle indicator settings panel" },
    Command { name: "FULLSCREEN",    desc: "Toggle fullscreen mode" },
    // Trading
    Command { name: "OPEN_TRADE",    desc: "Open a new trade" },
    Command { name: "CLOSE_ALL",     desc: "Close all open positions" },
    Command { name: "CLOSE_PARTIAL", desc: "Close partial position" },
    Command { name: "SET_SL",        desc: "Set stop-loss on current position" },
    Command { name: "SET_TP",        desc: "Set take-profit on current position" },
    Command { name: "OPEN_MG",       desc: "Open Martingale hedge" },
    Command { name: "BUY_LINES",     desc: "Place buy reference lines on chart" },
    Command { name: "SELL_LINES",    desc: "Place sell reference lines on chart" },
    // Tools
    Command { name: "DARWIN",        desc: "DARWIN accounts overview" },
    Command { name: "PORTFOLIO",     desc: "DARWIN portfolio dashboard" },
    Command { name: "OVERLAP",       desc: "Symbol overlap / correlation" },
    Command { name: "BACKTEST",      desc: "Run backtest on current symbol" },
    Command { name: "SCREENER",      desc: "Symbol screener" },
    Command { name: "OPTIMIZER",     desc: "Strategy parameter optimizer" },
    Command { name: "RISK_CALC",     desc: "Position sizing / risk calculator" },
    Command { name: "VAR",           desc: "VaR multiplier estimator" },
    Command { name: "MARGIN",        desc: "Margin monitor" },
    // Research
    Command { name: "NEWS",          desc: "Market news & events" },
    Command { name: "CALENDAR",      desc: "Economic calendar" },
    Command { name: "SEC",           desc: "SEC filings (10-K, 10-Q, 8-K)" },
    Command { name: "INSIDER",       desc: "Insider trades (Form 4)" },
    Command { name: "FUNDAMENTALS",  desc: "Company fundamentals" },
    Command { name: "ANALYST",       desc: "Analyst ratings & targets" },
    Command { name: "HOLDERS",       desc: "Institutional holders" },
    // Analysis
    Command { name: "CORRELATION",   desc: "Correlation matrix" },
    Command { name: "SEASONALS",     desc: "Seasonal patterns" },
    Command { name: "MONTECARLO",    desc: "Monte Carlo VaR simulation" },
    Command { name: "STRESS_TEST",   desc: "Portfolio stress test" },
    Command { name: "VOLUME_PROFILE",desc: "Volume profile (POC + value area)" },
    Command { name: "ORDER_FLOW",    desc: "Order flow / delta analysis" },
    Command { name: "BOOKMAP",       desc: "Bookmap depth heatmap" },
    // Chart types
    Command { name: "CANDLE",        desc: "Switch to candlestick chart" },
    Command { name: "HEIKINASHI",    desc: "Switch to Heikin-Ashi chart" },
    Command { name: "LINE",          desc: "Switch to line chart" },
    Command { name: "OHLC",          desc: "Switch to OHLC bars chart" },
    Command { name: "RENKO",         desc: "Switch to Renko chart" },
    Command { name: "EXPORT_CSV",    desc: "Export chart data to CSV" },
    Command { name: "NEW_TAB",       desc: "Open new chart tab" },
    Command { name: "CLOSE_TAB",     desc: "Close current chart tab" },
    // DARWIN-specific
    Command { name: "DARWINS",       desc: "Combined DARWIN portfolio view" },
    Command { name: "DRAWDOWN",      desc: "Drawdown dashboard per DARWIN" },
    Command { name: "REBALANCE",     desc: "VaR reduction via decorrelation" },
    Command { name: "DARWIN_TRADES", desc: "Toggle deal history markers on chart" },
    Command { name: "DSCORE",        desc: "D-Score estimation components" },
    // Drawing tools
    Command { name: "DRAW_HLINE",    desc: "Draw horizontal line" },
    Command { name: "DRAW_TRENDLINE",desc: "Draw trendline (2 clicks)" },
    Command { name: "DRAW_FIBO",     desc: "Draw Fibonacci retracement" },
    Command { name: "CLEAR_DRAWINGS",desc: "Clear all drawings on chart" },
    // Timeframes (direct switch)
    Command { name: "M1",            desc: "Switch to 1-minute timeframe" },
    Command { name: "M5",            desc: "Switch to 5-minute timeframe" },
    Command { name: "M15",           desc: "Switch to 15-minute timeframe" },
    Command { name: "M30",           desc: "Switch to 30-minute timeframe" },
    Command { name: "H1",            desc: "Switch to 1-hour timeframe" },
    Command { name: "H4",            desc: "Switch to 4-hour timeframe" },
    Command { name: "D1",            desc: "Switch to daily timeframe" },
    Command { name: "W1",            desc: "Switch to weekly timeframe" },
    Command { name: "MN1",           desc: "Switch to monthly timeframe" },
    // Analytics (from old app)
    Command { name: "EQUITY",        desc: "Account equity curve" },
    Command { name: "CALC",          desc: "Position sizing calculator" },
    Command { name: "TRADESTATS",    desc: "Trade statistics (win rate, expectancy)" },
    Command { name: "PERF",          desc: "Symbol performance chart" },
    Command { name: "COMPARE",       desc: "Normalized multi-symbol overlay" },
    Command { name: "SPREAD",        desc: "Price ratio / spread chart" },
    Command { name: "PIVOTS",        desc: "Classic pivot points on chart" },
    Command { name: "SRLEVEL",       desc: "Auto support/resistance from fractals" },
    Command { name: "HEATMAP",       desc: "Daily P&L heatmap" },
    Command { name: "PROFILE",       desc: "Trading profile (best symbols, times)" },
    Command { name: "SIGNAL",        desc: "Composite 0-100 trading signal" },
    Command { name: "DASHBOARD",     desc: "System health dashboard" },
    Command { name: "STATUS",        desc: "Cache, memory, uptime status" },
    // Crypto-specific
    Command { name: "CRYPTO_BACKFILL",desc: "Kraken weekend gap-fill" },
    // Data management
    Command { name: "BACKUP",        desc: "Backup settings and cache" },
    Command { name: "IMPORT_XLSX",   desc: "Import DARWIN XLSX trade history" },
    Command { name: "WORKSPACE",     desc: "Save/restore workspace layout" },
    // Misc
    Command { name: "CACHE_STATS",   desc: "Show cache statistics" },
    Command { name: "CLOSE_WINDOWS", desc: "Close all floating windows" },
    Command { name: "HELP",          desc: "Keyboard shortcuts reference" },
];

fn fuzzy_match(query: &str, target: &str) -> bool {
    let q = query.to_lowercase();
    let t = target.to_lowercase();
    if q.is_empty() { return true; }
    let mut qi = q.chars().peekable();
    for c in t.chars() {
        if qi.peek() == Some(&c) { qi.next(); }
    }
    qi.peek().is_none()
}

// ─── application state ───────────────────────────────────────────────────────

/// Whether the bottom panel is showing log or volume.
#[derive(PartialEq)]
enum BottomTab {
    Log,
    Volume,
}

pub struct TyphooNApp {
    /// Shared cache handle — opened once at startup.
    cache: Option<Arc<SqliteCache>>,
    /// Cache open error (shown in log if set).
    cache_err: Option<String>,

    /// Symbol input text in the toolbar.
    symbol_input: String,

    /// Primary chart (or charts[0] in grid mode).
    charts: Vec<ChartState>,
    /// MTF grid: how many columns to show.
    mtf_cols: usize,
    /// MTF grid enabled flag.
    mtf_enabled: bool,

    /// Command palette open state.
    command_open: bool,
    /// Raw user input in the command palette.
    command_input: String,

    // ── indicator overlay toggles ────────────────────────────────────────
    show_sma200: bool,
    show_sma100: bool,
    show_kama: bool,
    show_ema21: bool,
    show_bollinger: bool,
    show_rsi: bool,
    show_fisher: bool,
    show_macd: bool,
    show_volume_pane: bool,
    show_stochastic: bool,
    show_adx: bool,
    show_ichimoku: bool,
    show_wma: bool,
    show_hma: bool,
    show_psar: bool,
    show_atr_proj: bool,
    show_cci: bool,
    show_williams_r: bool,
    show_obv: bool,
    show_momentum: bool,
    show_better_volume: bool,

    /// Drawing interaction mode.
    draw_mode: DrawMode,

    /// DARWIN XLSX import ticker input.
    darwin_import_ticker: String,

    /// Broker connection fields (Alpaca).
    broker_api_key: String,
    broker_secret: String,
    broker_paper: bool,

    /// Broker connection fields (tastytrade).
    tt_username: String,
    tt_password: String,
    tt_sandbox: bool,

    /// SL/TP planning lines (visual, pre-broker).
    sl_price: Option<f64>,
    tp_price: Option<f64>,

    // ── risk calculator state ────────────────────────────────────────────
    rc_equity: String,
    rc_risk_pct: String,
    rc_entry: String,
    rc_sl: String,
    rc_tp: String,
    rc_tick_value: String,
    rc_tick_size: String,
    rc_result: String,

    // ── backtest state ───────────────────────────────────────────────────
    bt_strategy: usize,
    bt_fast_period: String,
    bt_slow_period: String,
    bt_equity: String,
    bt_result: Option<backtest::TradeReport>,
    bt_trades: Vec<backtest::Trade>,

    // ── optimizer state ──────────────────────────────────────────────────
    opt_fast_range: String,
    opt_slow_range: String,
    opt_results: Vec<backtest::OptimizationResult>,

    // ── margin monitor state ─────────────────────────────────────────────
    mm_equity: String,
    mm_margin: String,
    mm_margin_per_lot: String,
    mm_trim_pct: String,
    mm_result: String,

    // ── tab bar ──────────────────────────────────────────────────────────
    /// Index of the active tab (into `charts`).
    active_tab: usize,

    // ── watchlist ────────────────────────────────────────────────────────
    /// Cached symbol keys from SQLite cache.
    watchlist_symbols: Vec<(String, i64)>,

    // ── floating window visibility ───────────────────────────────────────
    show_settings: bool,
    show_darwin_accounts: bool,
    show_darwin_portfolio: bool,
    show_risk_calc: bool,
    show_backtest: bool,
    show_screener: bool,
    show_optimizer: bool,
    show_news: bool,
    show_calendar: bool,
    show_sec: bool,
    show_insider: bool,
    show_fundamentals: bool,
    show_analyst: bool,
    show_holders: bool,
    show_symbol_overlap: bool,
    show_correlation: bool,
    show_seasonals: bool,
    show_montecarlo: bool,
    show_stress_test: bool,
    show_volume_profile: bool,
    show_order_flow: bool,
    show_bookmap: bool,
    show_var_mult: bool,
    show_margin_monitor: bool,
    show_cache_stats: bool,
    show_help: bool,
    show_connect: bool,
    show_indicators_panel: bool,
    show_data_window: bool,
    show_alerts: bool,

    /// Price alerts.
    alerts: Vec<(f64, String)>, // (price, label)
    alert_price_input: String,
    alert_label_input: String,

    /// Bottom panel tab.
    bottom_tab: BottomTab,

    /// Application log — max 500 entries, ring-buffer style.
    log: VecDeque<LogEntry>,

    /// Crosshair position in screen coordinates (updated each frame).
    crosshair: Option<egui::Pos2>,

    /// Counter to avoid calling ctx.request_repaint in a tight loop.
    frame_count: u64,
}

impl TyphooNApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut log: VecDeque<LogEntry> = VecDeque::new();

        // ── open SQLite cache ────────────────────────────────────────────────
        let db_path = {
            let mut p = dirs_home();
            p.push("cache");
            p.push("typhoon_cache.db");
            p
        };

        let (cache, cache_err) = match SqliteCache::open(&db_path) {
            Ok(c) => {
                log.push_back(LogEntry::info(format!("Cache opened: {}", db_path.display())));
                (Some(Arc::new(c)), None)
            }
            Err(e) => {
                let msg = format!("Cannot open cache at {}: {e}", db_path.display());
                log.push_back(LogEntry::err(&msg));
                (None, Some(msg))
            }
        };

        // ── build default chart set (H4 + D1 + H1 + W1 for MTF grid) ────────
        let default_tfs = [Timeframe::H4, Timeframe::D1, Timeframe::H1, Timeframe::W1];
        let mut charts: Vec<ChartState> = default_tfs
            .iter()
            .map(|&tf| ChartState::new("CC:BTCUSD", tf))
            .collect();

        // ── load bars into all charts ────────────────────────────────────────
        if let Some(ref c) = cache {
            for chart in &mut charts {
                chart.load(c, &mut log);
            }
        }

        // ── load watchlist from cache keys ───────────────────────────────────
        let watchlist_symbols = if let Some(ref c) = cache {
            match c.detailed_stats() {
                Ok(stats) => stats.into_iter().map(|(k, count, _)| (k, count)).collect(),
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        };

        let mut app = Self {
            cache,
            cache_err,
            symbol_input: "CC:BTCUSD".to_string(),
            charts,
            mtf_cols: 2,
            mtf_enabled: false,
            command_open: false,
            command_input: String::new(),
            show_sma200: true,
            show_sma100: true,
            show_kama: true,
            show_ema21: false,
            show_bollinger: false,
            show_rsi: false,
            show_fisher: false,
            show_macd: false,
            show_volume_pane: false,
            show_stochastic: false,
            show_adx: false,
            show_ichimoku: false,
            show_wma: false,
            show_hma: false,
            show_psar: false,
            show_atr_proj: false,
            show_cci: false,
            show_williams_r: false,
            show_obv: false,
            show_momentum: false,
            show_better_volume: false,
            draw_mode: DrawMode::None,
            darwin_import_ticker: String::new(),
            broker_api_key: String::new(),
            broker_secret: String::new(),
            broker_paper: true,
            tt_username: String::new(),
            tt_password: String::new(),
            tt_sandbox: true,
            sl_price: None,
            tp_price: None,
            rc_equity: "10000".to_string(),
            rc_risk_pct: "1.0".to_string(),
            rc_entry: String::new(),
            rc_sl: String::new(),
            rc_tp: String::new(),
            rc_tick_value: "1.0".to_string(),
            rc_tick_size: "0.01".to_string(),
            rc_result: String::new(),
            bt_strategy: 0,
            bt_fast_period: "10".to_string(),
            bt_slow_period: "50".to_string(),
            bt_equity: "10000".to_string(),
            bt_result: None,
            bt_trades: Vec::new(),
            opt_fast_range: "5-50".to_string(),
            opt_slow_range: "20-200".to_string(),
            opt_results: Vec::new(),
            mm_equity: "10000".to_string(),
            mm_margin: "0".to_string(),
            mm_margin_per_lot: "1000".to_string(),
            mm_trim_pct: "150".to_string(),
            mm_result: String::new(),
            active_tab: 0,
            watchlist_symbols,
            show_settings: false,
            show_darwin_accounts: false,
            show_darwin_portfolio: false,
            show_risk_calc: false,
            show_backtest: false,
            show_screener: false,
            show_optimizer: false,
            show_news: false,
            show_calendar: false,
            show_sec: false,
            show_insider: false,
            show_fundamentals: false,
            show_analyst: false,
            show_holders: false,
            show_symbol_overlap: false,
            show_correlation: false,
            show_seasonals: false,
            show_montecarlo: false,
            show_stress_test: false,
            show_volume_profile: false,
            show_order_flow: false,
            show_bookmap: false,
            show_var_mult: false,
            show_margin_monitor: false,
            show_cache_stats: false,
            show_help: false,
            show_connect: false,
            show_indicators_panel: false,
            show_data_window: false,
            show_alerts: false,
            alerts: Vec::new(),
            alert_price_input: String::new(),
            alert_label_input: String::new(),
            bottom_tab: BottomTab::Log,
            log,
            crosshair: None,
            frame_count: 0,
        };
        app.load_session();
        app
    }

    // ── helpers ──────────────────────────────────────────────────────────────

    fn dark_visuals() -> egui::Visuals {
        let mut v = egui::Visuals::dark();
        v.panel_fill                        = egui::Color32::from_rgb(0, 0, 0);
        v.window_fill                       = egui::Color32::from_rgb(5, 5, 8);
        v.extreme_bg_color                  = egui::Color32::from_rgb(0, 0, 0);
        v.widgets.noninteractive.bg_fill    = egui::Color32::from_rgb(8, 8, 12);
        v.widgets.inactive.bg_fill          = egui::Color32::from_rgb(12, 12, 18);
        v.widgets.hovered.bg_fill           = egui::Color32::from_rgb(20, 20, 35);
        v.widgets.active.bg_fill            = egui::Color32::from_rgb(20, 40, 100);
        v.selection.bg_fill                 = egui::Color32::from_rgb(20, 50, 120);
        v
    }

    fn reload_symbol(&mut self, symbol: &str, tf: Timeframe) {
        if let Some(ref cache) = self.cache {
            let chart_type = self.charts.get(self.active_tab).map(|c| c.chart_type).unwrap_or(ChartType::Candle);
            let mut chart = ChartState::new(symbol, tf);
            chart.chart_type = chart_type;
            let cache_ref = Arc::as_ref(cache);
            chart.load(cache_ref, &mut self.log);
            if let Some(target) = self.charts.get_mut(self.active_tab) {
                *target = chart;
            }
        } else {
            self.log.push_back(LogEntry::warn("Cache not available"));
        }
    }

    fn indicator_flags(&self) -> IndicatorFlags {
        IndicatorFlags {
            sma200: self.show_sma200,
            sma100: self.show_sma100,
            kama: self.show_kama,
            ema21: self.show_ema21,
            bollinger: self.show_bollinger,
            ichimoku: self.show_ichimoku,
            wma: self.show_wma,
            hma: self.show_hma,
            psar: self.show_psar,
            atr_proj: self.show_atr_proj,
        }
    }

    fn handle_command(&mut self, cmd: &str, ctx: &egui::Context) {
        match cmd.trim().to_uppercase().as_str() {
            "QUIT" => {
                self.save_session();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            "MTF" | "MTF_GRID" => {
                self.mtf_enabled = !self.mtf_enabled;
                self.log.push_back(LogEntry::info(format!("MTF grid: {}", self.mtf_enabled)));
            }
            "RELOAD" => {
                if let Some(ref cache) = self.cache.clone() {
                    for chart in &mut self.charts {
                        chart.load(Arc::as_ref(cache), &mut self.log);
                    }
                }
            }
            "CONNECT"       => self.show_connect = true,
            "SETTINGS"      => self.show_settings = true,
            "INDICATORS"    => self.show_indicators_panel = !self.show_indicators_panel,
            "DARWIN"        => self.show_darwin_accounts = true,
            "PORTFOLIO"     => self.show_darwin_portfolio = true,
            "OVERLAP"       => self.show_symbol_overlap = true,
            "BACKTEST"      => self.show_backtest = true,
            "SCREENER"      => self.show_screener = true,
            "OPTIMIZER"     => self.show_optimizer = true,
            "RISK_CALC"     => self.show_risk_calc = true,
            "VAR"           => self.show_var_mult = true,
            "MARGIN"        => self.show_margin_monitor = true,
            "NEWS"          => self.show_news = true,
            "CALENDAR"      => self.show_calendar = true,
            "SEC"           => self.show_sec = true,
            "INSIDER"       => self.show_insider = true,
            "FUNDAMENTALS"  => self.show_fundamentals = true,
            "ANALYST"       => self.show_analyst = true,
            "HOLDERS"       => self.show_holders = true,
            "CORRELATION"   => self.show_correlation = true,
            "SEASONALS"     => self.show_seasonals = true,
            "MONTECARLO"    => self.show_montecarlo = true,
            "STRESS_TEST"   => self.show_stress_test = true,
            "VOLUME_PROFILE"=> self.show_volume_profile = true,
            "ORDER_FLOW"    => self.show_order_flow = true,
            "BOOKMAP"       => self.show_bookmap = true,
            "CACHE_STATS"   => self.show_cache_stats = true,
            "HELP"          => self.show_help = true,
            "FULLSCREEN"    => ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true)),
            "CLOSE_WINDOWS" => self.close_all_windows(),
            // Chart types
            "CANDLE"     => { if let Some(c) = self.charts.get_mut(self.active_tab) { c.chart_type = ChartType::Candle; } }
            "HEIKINASHI" => { if let Some(c) = self.charts.get_mut(self.active_tab) { c.chart_type = ChartType::HeikinAshi; } }
            "LINE"       => { if let Some(c) = self.charts.get_mut(self.active_tab) { c.chart_type = ChartType::Line; } }
            "OHLC"       => { if let Some(c) = self.charts.get_mut(self.active_tab) { c.chart_type = ChartType::OhlcBars; } }
            "RENKO"      => { if let Some(c) = self.charts.get_mut(self.active_tab) { c.chart_type = ChartType::Renko; } }
            "EXPORT_CSV" => { self.export_csv(); }
            // Tabs
            "NEW_TAB" => {
                let tf = self.charts.get(self.active_tab).map(|c| c.timeframe).unwrap_or(Timeframe::H4);
                let mut new_chart = ChartState::new(&self.symbol_input, tf);
                if let Some(ref cache) = self.cache.clone() {
                    new_chart.load(Arc::as_ref(cache), &mut self.log);
                }
                self.charts.push(new_chart);
                self.active_tab = self.charts.len() - 1;
            }
            "CLOSE_TAB" => {
                if self.charts.len() > 1 {
                    self.charts.remove(self.active_tab);
                    if self.active_tab >= self.charts.len() {
                        self.active_tab = self.charts.len() - 1;
                    }
                }
            }
            // DARWIN-specific
            "DARWINS"       => self.show_darwin_portfolio = true,
            "DRAWDOWN"      => self.show_darwin_portfolio = true,
            "REBALANCE"     => self.show_symbol_overlap = true,
            "DARWIN_TRADES" => { self.log.push_back(LogEntry::info("DARWIN trade markers: open DARWIN Accounts for deal history")); self.show_darwin_accounts = true; }
            "DSCORE"        => { self.log.push_back(LogEntry::info("D-Score: open DARWIN Accounts for per-account analytics")); self.show_darwin_accounts = true; }
            // Drawing tools
            "DRAW_HLINE"     => self.draw_mode = DrawMode::PlacingHLine,
            "DRAW_TRENDLINE" => self.draw_mode = DrawMode::PlacingTrendP1,
            "DRAW_FIBO"      => self.draw_mode = DrawMode::PlacingFiboP1,
            "CLEAR_DRAWINGS" => { if let Some(c) = self.charts.get_mut(self.active_tab) { c.drawings.clear(); } }
            // Timeframe shortcuts
            "M1"  => { let sym = self.symbol_input.clone(); self.reload_symbol(&sym, Timeframe::M1); }
            "M5"  => { let sym = self.symbol_input.clone(); self.reload_symbol(&sym, Timeframe::M5); }
            "M15" => { let sym = self.symbol_input.clone(); self.reload_symbol(&sym, Timeframe::M15); }
            "M30" => { let sym = self.symbol_input.clone(); self.reload_symbol(&sym, Timeframe::M30); }
            "H1"  => { let sym = self.symbol_input.clone(); self.reload_symbol(&sym, Timeframe::H1); }
            "H4"  => { let sym = self.symbol_input.clone(); self.reload_symbol(&sym, Timeframe::H4); }
            "D1"  => { let sym = self.symbol_input.clone(); self.reload_symbol(&sym, Timeframe::D1); }
            "W1"  => { let sym = self.symbol_input.clone(); self.reload_symbol(&sym, Timeframe::W1); }
            "MN1" => { let sym = self.symbol_input.clone(); self.reload_symbol(&sym, Timeframe::MN1); }
            // Aliases
            "EQUITY"         => self.show_darwin_portfolio = true,
            "CALC"           => self.show_risk_calc = true,
            "TRADESTATS"     => self.show_darwin_accounts = true,
            "PERF"           => self.show_seasonals = true,
            "COMPARE"        => self.show_correlation = true,
            "SPREAD"         => self.show_symbol_overlap = true,
            "HEATMAP"        => self.show_seasonals = true,
            "PROFILE"        => self.show_darwin_accounts = true,
            "SIGNAL"         => self.show_indicators_panel = true,
            "DASHBOARD"      => self.show_cache_stats = true,
            "STATUS"         => self.show_cache_stats = true,
            "IMPORT_XLSX"    => self.show_darwin_accounts = true,
            "WORKSPACE"      => { self.save_session(); self.log.push_back(LogEntry::info("Workspace saved")); }
            "BACKUP"         => { self.save_session(); self.log.push_back(LogEntry::info("Session backup saved")); }
            "PIVOTS" | "SRLEVEL" => {
                self.log.push_back(LogEntry::info("Pivot/SR levels: use drawing tools to mark key levels"));
            }
            "CRYPTO_BACKFILL" => {
                self.log.push_back(LogEntry::info("Kraken backfill: requires async runtime integration"));
            }
            // Trading stubs — log the action
            "OPEN_TRADE" | "CLOSE_ALL" | "CLOSE_PARTIAL" |
            "SET_SL" | "SET_TP" | "OPEN_MG" | "BUY_LINES" | "SELL_LINES" => {
                self.log.push_back(LogEntry::info(format!("Trading: {} — connect to broker first", cmd)));
            }
            other => {
                self.log.push_back(LogEntry::warn(format!("Unknown command: {}", other)));
            }
        }
    }

    fn save_session(&self) {
        let session = serde_json::json!({
            "symbol": self.symbol_input,
            "active_tab": self.active_tab,
            "tabs": self.charts.iter().map(|c| serde_json::json!({
                "symbol": c.symbol,
                "timeframe": c.timeframe.label(),
                "chart_type": c.chart_type.label(),
            })).collect::<Vec<_>>(),
            "indicators": {
                "sma200": self.show_sma200,
                "sma100": self.show_sma100,
                "kama": self.show_kama,
                "ema21": self.show_ema21,
                "bollinger": self.show_bollinger,
                "ichimoku": self.show_ichimoku,
                "rsi": self.show_rsi,
                "fisher": self.show_fisher,
                "macd": self.show_macd,
                "stochastic": self.show_stochastic,
                "adx": self.show_adx,
                "volume_pane": self.show_volume_pane,
            },
            "mtf_enabled": self.mtf_enabled,
        });
        let mut path = dirs_home();
        path.push("session.json");
        if let Ok(json) = serde_json::to_string_pretty(&session) {
            let _ = std::fs::write(&path, json);
        }
    }

    fn load_session(&mut self) {
        let mut path = dirs_home();
        path.push("session.json");
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                if let Some(sym) = v["symbol"].as_str() { self.symbol_input = sym.to_string(); }
                if let Some(mtf) = v["mtf_enabled"].as_bool() { self.mtf_enabled = mtf; }
                if let Some(ind) = v.get("indicators") {
                    if let Some(b) = ind["sma200"].as_bool() { self.show_sma200 = b; }
                    if let Some(b) = ind["sma100"].as_bool() { self.show_sma100 = b; }
                    if let Some(b) = ind["kama"].as_bool() { self.show_kama = b; }
                    if let Some(b) = ind["ema21"].as_bool() { self.show_ema21 = b; }
                    if let Some(b) = ind["bollinger"].as_bool() { self.show_bollinger = b; }
                    if let Some(b) = ind["ichimoku"].as_bool() { self.show_ichimoku = b; }
                    if let Some(b) = ind["rsi"].as_bool() { self.show_rsi = b; }
                    if let Some(b) = ind["fisher"].as_bool() { self.show_fisher = b; }
                    if let Some(b) = ind["macd"].as_bool() { self.show_macd = b; }
                    if let Some(b) = ind["stochastic"].as_bool() { self.show_stochastic = b; }
                    if let Some(b) = ind["adx"].as_bool() { self.show_adx = b; }
                    if let Some(b) = ind["volume_pane"].as_bool() { self.show_volume_pane = b; }
                }
                self.log.push_back(LogEntry::info("Session restored"));
            }
        }
    }

    fn export_csv(&mut self) {
        if let Some(chart) = self.charts.get(self.active_tab) {
            if chart.bars.is_empty() {
                self.log.push_back(LogEntry::warn("No bars to export"));
                return;
            }
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("CSV", &["csv"])
                .set_file_name(&format!("{}_{}.csv", chart.symbol, chart.timeframe.label()))
                .set_title("Export Chart Data")
                .save_file()
            {
                match std::fs::File::create(&path) {
                    Ok(mut f) => {
                        let _ = writeln!(f, "timestamp,open,high,low,close,volume");
                        for bar in &chart.bars {
                            let _ = writeln!(f, "{},{},{},{},{},{}", bar.ts_ms, bar.open, bar.high, bar.low, bar.close, bar.volume);
                        }
                        self.log.push_back(LogEntry::info(format!("Exported {} bars to {}", chart.bars.len(), path.display())));
                    }
                    Err(e) => {
                        self.log.push_back(LogEntry::err(format!("Export failed: {}", e)));
                    }
                }
            }
        }
    }

    fn close_all_windows(&mut self) {
        self.show_settings = false;
        self.show_darwin_accounts = false;
        self.show_darwin_portfolio = false;
        self.show_risk_calc = false;
        self.show_backtest = false;
        self.show_screener = false;
        self.show_optimizer = false;
        self.show_news = false;
        self.show_calendar = false;
        self.show_sec = false;
        self.show_insider = false;
        self.show_fundamentals = false;
        self.show_analyst = false;
        self.show_holders = false;
        self.show_symbol_overlap = false;
        self.show_correlation = false;
        self.show_seasonals = false;
        self.show_montecarlo = false;
        self.show_stress_test = false;
        self.show_volume_profile = false;
        self.show_order_flow = false;
        self.show_bookmap = false;
        self.show_var_mult = false;
        self.show_margin_monitor = false;
        self.show_cache_stats = false;
        self.show_help = false;
        self.show_connect = false;
        self.show_indicators_panel = false;
        self.show_data_window = false;
        self.show_alerts = false;
    }

    // ── chart interaction (zoom / pan) ───────────────────────────────────────

    fn handle_zoom(chart: &mut ChartState, delta: f32) {
        let factor = if delta > 0.0 { 0.85_f32 } else { 1.0 / 0.85_f32 };
        let new_vis = ((chart.visible_bars as f32 * factor) as usize)
            .clamp(10, chart.bars.len().max(10));
        chart.visible_bars = new_vis;
    }

    fn handle_pan_h(chart: &mut ChartState, dx: f32, rect_width: f32) {
        if chart.bars.is_empty() { return; }
        let bar_w = rect_width / chart.visible_bars as f32;
        let delta_bars = (dx / bar_w) as isize;
        let new_offset = (chart.view_offset as isize - delta_bars)
            .clamp(0, chart.bars.len() as isize - 1) as usize;
        chart.view_offset = new_offset;
    }

    // ── floating window rendering ────────────────────────────────────────────

    fn draw_floating_windows(&mut self, ctx: &egui::Context) {
        // Settings
        if self.show_settings {
            egui::Window::new("Settings")
                .open(&mut self.show_settings)
                .default_size([400.0, 300.0])
                .show(ctx, |ui| {
                    ui.heading("General");
                    ui.separator();
                    ui.label("Theme: Dark (hardcoded)");
                    ui.label("Refresh rate: 250ms");
                    ui.add_space(10.0);
                    ui.heading("Data Sources");
                    ui.separator();
                    ui.label("SQLite cache: ~/.config/typhoon-terminal/cache/typhoon_cache.db");
                    if let Some(ref cache) = self.cache {
                        if let Ok((rows, kv, size)) = cache.stats() {
                            ui.label(format!("Bar entries: {}", rows));
                            ui.label(format!("KV entries: {}", kv));
                            ui.label(format!("DB size: {} KB", size / 1024));
                        }
                    }
                    ui.add_space(10.0);
                    ui.heading("Indicators");
                    ui.separator();
                    ui.checkbox(&mut self.show_sma200,    "SMA(200)");
                    ui.checkbox(&mut self.show_sma100,    "SMA(100)");
                    ui.checkbox(&mut self.show_kama,      "KAMA(10,2,30)");
                    ui.checkbox(&mut self.show_ema21,     "EMA(21)");
                    ui.checkbox(&mut self.show_bollinger, "Bollinger Bands(20,2)");
                    ui.checkbox(&mut self.show_rsi,          "RSI(14) sub-pane");
                    ui.checkbox(&mut self.show_fisher,       "Fisher Transform(10) sub-pane");
                    ui.checkbox(&mut self.show_macd,         "MACD(12,26,9) sub-pane");
                    ui.checkbox(&mut self.show_volume_pane,  "Volume sub-pane");
                });
        }

        // Connect to Broker
        if self.show_connect {
            egui::Window::new("Connect to Broker")
                .open(&mut self.show_connect)
                .default_size([450.0, 300.0])
                .show(ctx, |ui| {
                    ui.heading("Alpaca Markets");
                    ui.separator();
                    egui::Grid::new("broker_grid").num_columns(2).show(ui, |ui| {
                        ui.label("API Key:");
                        ui.add(egui::TextEdit::singleline(&mut self.broker_api_key).desired_width(250.0).password(true));
                        ui.end_row();
                        ui.label("Secret:");
                        ui.add(egui::TextEdit::singleline(&mut self.broker_secret).desired_width(250.0).password(true));
                        ui.end_row();
                        ui.label("Mode:");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.broker_paper, true, "Paper");
                            ui.radio_value(&mut self.broker_paper, false, "Live");
                        });
                        ui.end_row();
                    });
                    ui.add_space(5.0);
                    if ui.button("Connect").clicked() {
                        if self.broker_api_key.is_empty() || self.broker_secret.is_empty() {
                            self.log.push_back(LogEntry::warn("Enter API key and secret"));
                        } else {
                            self.log.push_back(LogEntry::info(format!(
                                "Alpaca {} broker — async connection requires tokio runtime integration (Phase 5)",
                                if self.broker_paper { "Paper" } else { "Live" }
                            )));
                        }
                    }
                    ui.add_space(10.0);
                    ui.heading("tastytrade");
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Username:");
                        ui.add(egui::TextEdit::singleline(&mut self.tt_username).desired_width(200.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Password:");
                        ui.add(egui::TextEdit::singleline(&mut self.tt_password).desired_width(200.0).password(true));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Mode:");
                        ui.radio_value(&mut self.tt_sandbox, true, "Sandbox");
                        ui.radio_value(&mut self.tt_sandbox, false, "Production");
                    });
                    if ui.button("Connect tastytrade").clicked() {
                        if self.tt_username.is_empty() || self.tt_password.is_empty() {
                            self.log.push_back(LogEntry::warn("Enter tastytrade username and password"));
                        } else {
                            self.log.push_back(LogEntry::info(format!(
                                "tastytrade {} — session auth requires async runtime integration (Phase 8)",
                                if self.tt_sandbox { "Sandbox" } else { "Production" }
                            )));
                        }
                    }
                    ui.add_space(10.0);
                    ui.heading("MT5 (view-only data source)");
                    ui.separator();
                    ui.label("MT5 bar data imported via BarCacheWriter EA → SQLite cache.");
                    ui.label("Trade management stays in MT5. DARWIN analytics via XLSX import.");
                });
        }

        // Indicator settings panel
        if self.show_indicators_panel {
            egui::Window::new("Indicators")
                .open(&mut self.show_indicators_panel)
                .default_size([250.0, 350.0])
                .show(ctx, |ui| {
                    ui.heading("Overlay Indicators");
                    ui.separator();
                    ui.checkbox(&mut self.show_sma200,    "SMA(200)");
                    ui.checkbox(&mut self.show_sma100,    "SMA(100)");
                    ui.checkbox(&mut self.show_kama,      "KAMA(10,2,30)");
                    ui.checkbox(&mut self.show_ema21,     "EMA(21)");
                    ui.checkbox(&mut self.show_bollinger, "Bollinger Bands(20,2)");
                    ui.checkbox(&mut self.show_ichimoku, "Ichimoku Cloud(9,26,52)");
                    ui.checkbox(&mut self.show_wma,      "WMA(20)");
                    ui.checkbox(&mut self.show_hma,      "HMA(20)");
                    ui.checkbox(&mut self.show_psar,     "Parabolic SAR(0.02,0.2)");
                    ui.checkbox(&mut self.show_atr_proj, "ATR Projection(14)");
                    ui.add_space(10.0);
                    ui.heading("Sub-Pane Indicators");
                    ui.separator();
                    ui.checkbox(&mut self.show_rsi,            "RSI(14)");
                    ui.checkbox(&mut self.show_fisher,         "Fisher Transform(10)");
                    ui.checkbox(&mut self.show_macd,           "MACD(12,26,9)");
                    ui.checkbox(&mut self.show_stochastic,     "Stochastic(14,3,3)");
                    ui.checkbox(&mut self.show_adx,            "ADX(14)");
                    ui.checkbox(&mut self.show_cci,            "CCI(20)");
                    ui.checkbox(&mut self.show_williams_r,     "Williams %R(14)");
                    ui.checkbox(&mut self.show_obv,            "OBV");
                    ui.checkbox(&mut self.show_momentum,       "Momentum(10)");
                    ui.checkbox(&mut self.show_better_volume,  "Better Volume");
                    ui.checkbox(&mut self.show_volume_pane,    "Volume");
                });
        }

        // DARWIN Accounts — wired to engine darwin.rs
        if self.show_darwin_accounts {
            egui::Window::new("DARWIN Accounts")
                .open(&mut self.show_darwin_accounts)
                .default_size([700.0, 500.0])
                .show(ctx, |ui| {
                    ui.heading("Account Overview");
                    ui.separator();
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            // Ensure tables exist
                            let _ = darwin::create_darwin_tables(&conn);
                            match darwin::list_darwin_accounts(&conn) {
                                Ok(accounts) if !accounts.is_empty() => {
                                    egui::Grid::new("darwin_acct_grid").striped(true).num_columns(6).show(ui, |ui| {
                                        ui.strong("DARWIN"); ui.strong("MT5"); ui.strong("Deals"); ui.strong("Positions"); ui.strong("Balance"); ui.strong("P&L");
                                        ui.end_row();
                                        for acct in &accounts {
                                            if let Ok(summary) = darwin::get_darwin_summary(&conn, &acct.darwin_ticker) {
                                                ui.label(&acct.darwin_ticker);
                                                ui.label(&acct.mt5_account);
                                                ui.label(format!("{}", summary.win_count + summary.loss_count));
                                                ui.label(format!("{}", acct.position_count));
                                                ui.label(format!("${:.2}", summary.final_balance));
                                                let pnl_color = if summary.total_profit >= 0.0 { UP } else { DOWN };
                                                ui.label(egui::RichText::new(format!("${:.2}", summary.total_profit)).color(pnl_color));
                                                ui.end_row();
                                            }
                                        }
                                    });
                                    ui.add_space(10.0);
                                    // Per-account details (expandable)
                                    for acct in &accounts {
                                        if let Ok(summary) = darwin::get_darwin_summary(&conn, &acct.darwin_ticker) {
                                            ui.collapsing(format!("{} — Details", acct.darwin_ticker), |ui| {
                                                egui::Grid::new(format!("det_{}", acct.darwin_ticker)).striped(true).num_columns(2).show(ui, |ui| {
                                                    ui.label("Win Rate:"); ui.label(format!("{:.1}%", summary.win_rate * 100.0));
                                                    ui.end_row();
                                                    ui.label("Profit Factor:"); ui.label(format!("{:.2}", summary.profit_factor));
                                                    ui.end_row();
                                                    ui.label("Max Drawdown:"); ui.label(format!("{:.2}%", summary.max_drawdown_pct));
                                                    ui.end_row();
                                                    ui.label("Total Commission:"); ui.label(format!("${:.2}", summary.total_commission));
                                                    ui.end_row();
                                                    ui.label("Total Swap:"); ui.label(format!("${:.2}", summary.total_swap));
                                                    ui.end_row();
                                                    ui.label("Symbols Traded:"); ui.label(format!("{}", summary.symbols_traded.len()));
                                                    ui.end_row();
                                                });
                                                // VaR stats
                                                if let Ok(daily) = darwin::get_daily_returns(&conn, &acct.darwin_ticker) {
                                                    if !daily.is_empty() {
                                                        let var_stats = darwin::compute_var(&daily);
                                                        ui.add_space(5.0);
                                                        ui.label(egui::RichText::new("Risk Metrics").strong());
                                                        egui::Grid::new(format!("var_{}", acct.darwin_ticker)).striped(true).num_columns(2).show(ui, |ui| {
                                                            ui.label("VaR 95%:"); ui.label(format!("${:.2}", var_stats.var_95));
                                                            ui.end_row();
                                                            ui.label("VaR 99%:"); ui.label(format!("${:.2}", var_stats.var_99));
                                                            ui.end_row();
                                                            ui.label("Sharpe:"); ui.label(format!("{:.3}", var_stats.sharpe));
                                                            ui.end_row();
                                                            ui.label("Sortino:"); ui.label(format!("{:.3}", var_stats.sortino));
                                                            ui.end_row();
                                                            ui.label("Daily Vol:"); ui.label(format!("{:.4}", var_stats.daily_vol));
                                                            ui.end_row();
                                                            ui.label("Best Day:"); ui.label(format!("${:.2}", var_stats.best_day));
                                                            ui.end_row();
                                                            ui.label("Worst Day:"); ui.label(format!("${:.2}", var_stats.worst_day));
                                                            ui.end_row();
                                                        });
                                                        // Monthly returns
                                                        let monthly = darwin::get_monthly_returns(&daily);
                                                        if !monthly.is_empty() {
                                                            ui.add_space(5.0);
                                                            ui.label(egui::RichText::new("Monthly Returns").strong());
                                                            egui::Grid::new(format!("mo_{}", acct.darwin_ticker)).striped(true).num_columns(3).show(ui, |ui| {
                                                                ui.strong("Period"); ui.strong("P&L"); ui.strong("Return");
                                                                ui.end_row();
                                                                for m in monthly.iter().rev().take(12) {
                                                                    ui.label(format!("{}-{:02}", m.year, m.month));
                                                                    let c = if m.pnl >= 0.0 { UP } else { DOWN };
                                                                    ui.label(egui::RichText::new(format!("${:.2}", m.pnl)).color(c));
                                                                    ui.label(egui::RichText::new(format!("{:.2}%", m.return_pct)).color(c));
                                                                    ui.end_row();
                                                                }
                                                            });
                                                        }
                                                    }
                                                }
                                                // Streak analysis
                                                if let Ok(streaks) = darwin::get_streak_analysis(&conn, &acct.darwin_ticker) {
                                                    ui.add_space(5.0);
                                                    ui.label(egui::RichText::new("Streaks").strong());
                                                    egui::Grid::new(format!("str_{}", acct.darwin_ticker)).striped(true).num_columns(2).show(ui, |ui| {
                                                        ui.label("Max Win Streak:"); ui.label(egui::RichText::new(format!("{}", streaks.max_win_streak)).color(UP));
                                                        ui.end_row();
                                                        ui.label("Max Loss Streak:"); ui.label(egui::RichText::new(format!("{}", streaks.max_loss_streak)).color(DOWN));
                                                        ui.end_row();
                                                        ui.label("Current Streak:");
                                                        let sc = if streaks.current_streak >= 0 { UP } else { DOWN };
                                                        ui.label(egui::RichText::new(format!("{}", streaks.current_streak)).color(sc));
                                                        ui.end_row();
                                                        ui.label("Avg Win Streak:"); ui.label(format!("{:.1}", streaks.avg_win_streak));
                                                        ui.end_row();
                                                        ui.label("Avg Loss Streak:"); ui.label(format!("{:.1}", streaks.avg_loss_streak));
                                                        ui.end_row();
                                                    });
                                                }
                                                // Hourly P&L
                                                if let Ok(hourly) = darwin::get_hourly_pnl(&conn, &acct.darwin_ticker) {
                                                    if !hourly.is_empty() {
                                                        ui.add_space(5.0);
                                                        ui.label(egui::RichText::new("Hourly P&L").strong());
                                                        egui::Grid::new(format!("hr_{}", acct.darwin_ticker)).striped(true).num_columns(4).show(ui, |ui| {
                                                            ui.strong("Hour"); ui.strong("P&L"); ui.strong("Trades"); ui.strong("Win%");
                                                            ui.end_row();
                                                            for h in &hourly {
                                                                ui.label(format!("{:02}:00", h.hour));
                                                                let c = if h.total_pnl >= 0.0 { UP } else { DOWN };
                                                                ui.label(egui::RichText::new(format!("${:.2}", h.total_pnl)).color(c));
                                                                ui.label(format!("{}", h.trade_count));
                                                                let wr = if h.trade_count > 0 { h.win_count as f64 / h.trade_count as f64 * 100.0 } else { 0.0 };
                                                                ui.label(format!("{:.0}%", wr));
                                                                ui.end_row();
                                                            }
                                                        });
                                                    }
                                                }
                                            });
                                        }
                                    }
                                }
                                Ok(_) => {
                                    ui.label(egui::RichText::new("No DARWIN accounts imported yet.").color(AXIS_TEXT));
                                    ui.label(egui::RichText::new("Export MT5 Trade History as XLSX, then import here.").color(AXIS_TEXT).small());
                                }
                                Err(e) => {
                                    ui.label(egui::RichText::new(format!("Error: {}", e)).color(egui::Color32::from_rgb(255, 80, 80)));
                                }
                            }
                            // ── XLSX Import section ──────────────────
                            ui.add_space(10.0);
                            ui.separator();
                            ui.heading("Import DARWIN XLSX");
                            ui.horizontal(|ui| {
                                ui.label("DARWIN Ticker:");
                                ui.add(egui::TextEdit::singleline(&mut self.darwin_import_ticker).desired_width(80.0));
                            });
                            if ui.button("Select XLSX File & Import…").clicked() {
                                let ticker = self.darwin_import_ticker.trim().to_string();
                                if ticker.is_empty() {
                                    self.log.push_back(LogEntry::warn("Enter a DARWIN ticker name first (e.g. THA, THB)"));
                                } else if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("Excel", &["xlsx"])
                                    .set_title("Select MT5 Trade History XLSX")
                                    .pick_file()
                                {
                                    match darwin::import_darwin_xlsx(&conn, &path.display().to_string(), &ticker) {
                                        Ok((name, deals, positions)) => {
                                            self.log.push_back(LogEntry::info(format!(
                                                "DARWIN {} imported: {} deals, {} positions from {}",
                                                name, deals, positions, path.display()
                                            )));
                                        }
                                        Err(e) => {
                                            self.log.push_back(LogEntry::err(format!("XLSX import failed: {}", e)));
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        ui.label(egui::RichText::new("Cache not available").color(egui::Color32::from_rgb(255, 80, 80)));
                    }
                });
        }

        // DARWIN Portfolio — wired to engine darwin.rs portfolio functions
        if self.show_darwin_portfolio {
            egui::Window::new("DARWIN Portfolio")
                .open(&mut self.show_darwin_portfolio)
                .default_size([700.0, 500.0])
                .show(ctx, |ui| {
                    ui.heading("Portfolio Dashboard");
                    ui.separator();
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = darwin::create_darwin_tables(&conn);
                            match darwin::get_portfolio_summary(&conn) {
                                Ok(portfolio) if !portfolio.accounts.is_empty() => {
                                    egui::Grid::new("port_summary").striped(true).num_columns(2).show(ui, |ui| {
                                        ui.label("Accounts:"); ui.label(format!("{}", portfolio.accounts.len()));
                                        ui.end_row();
                                        ui.label("Initial Balance:"); ui.label(format!("${:.2}", portfolio.total_initial_balance));
                                        ui.end_row();
                                        ui.label("Final Balance:"); ui.label(format!("${:.2}", portfolio.total_final_balance));
                                        ui.end_row();
                                        let pnl_c = if portfolio.total_net_pnl >= 0.0 { UP } else { DOWN };
                                        ui.label("Net P&L:"); ui.label(egui::RichText::new(format!("${:.2}", portfolio.total_net_pnl)).color(pnl_c));
                                        ui.end_row();
                                        ui.label("Total Commission:"); ui.label(format!("${:.2}", portfolio.total_commission));
                                        ui.end_row();
                                        ui.label("Max Drawdown:"); ui.label(format!("{:.2}%", portfolio.combined_max_drawdown_pct));
                                        ui.end_row();
                                        ui.label("Total Deals:"); ui.label(format!("{}", portfolio.total_deals));
                                        ui.end_row();
                                    });

                                    // Portfolio VaR
                                    if let Ok(daily) = darwin::get_portfolio_daily_returns(&conn) {
                                        if !daily.is_empty() {
                                            let var_stats = darwin::compute_var(&daily);
                                            ui.add_space(10.0);
                                            ui.heading("Portfolio Risk");
                                            ui.separator();
                                            egui::Grid::new("port_var").striped(true).num_columns(4).show(ui, |ui| {
                                                ui.label("VaR 95%:"); ui.label(format!("${:.2}", var_stats.var_95));
                                                ui.label("Sharpe:"); ui.label(format!("{:.3}", var_stats.sharpe));
                                                ui.end_row();
                                                ui.label("VaR 99%:"); ui.label(format!("${:.2}", var_stats.var_99));
                                                ui.label("Sortino:"); ui.label(format!("{:.3}", var_stats.sortino));
                                                ui.end_row();
                                                ui.label("Ann. Vol:"); ui.label(format!("{:.4}", var_stats.annualized_vol));
                                                ui.label("Calmar:"); ui.label(format!("{:.3}", var_stats.calmar));
                                                ui.end_row();
                                            });
                                        }
                                    }

                                    // Correlation matrix
                                    if let Ok(corrs) = darwin::get_darwin_correlations(&conn) {
                                        if !corrs.is_empty() {
                                            ui.add_space(10.0);
                                            ui.heading("Correlation Matrix");
                                            ui.separator();
                                            egui::Grid::new("corr_grid").striped(true).num_columns(3).show(ui, |ui| {
                                                ui.strong("DARWIN A"); ui.strong("DARWIN B"); ui.strong("Correlation");
                                                ui.end_row();
                                                for c in &corrs {
                                                    ui.label(&c.darwin_a);
                                                    ui.label(&c.darwin_b);
                                                    let color = if c.correlation.abs() > 0.95 { egui::Color32::from_rgb(255, 80, 80) }
                                                                else if c.correlation.abs() > 0.7 { egui::Color32::from_rgb(255, 200, 50) }
                                                                else { UP };
                                                    ui.label(egui::RichText::new(format!("{:.4}", c.correlation)).color(color));
                                                    ui.end_row();
                                                }
                                            });
                                        }
                                    }

                                    // Symbol exposure
                                    if let Ok(exposure) = darwin::get_portfolio_exposure(&conn) {
                                        if !exposure.is_empty() {
                                            ui.add_space(10.0);
                                            ui.heading("Symbol Exposure");
                                            ui.separator();
                                            egui::Grid::new("exp_grid").striped(true).num_columns(5).show(ui, |ui| {
                                                ui.strong("Symbol"); ui.strong("Long $"); ui.strong("Short $"); ui.strong("Net $"); ui.strong("DARWINs");
                                                ui.end_row();
                                                for e in &exposure {
                                                    ui.label(&e.symbol);
                                                    ui.label(format!("{:.0}", e.long_notional));
                                                    ui.label(format!("{:.0}", e.short_notional));
                                                    let net_c = if e.net_notional >= 0.0 { UP } else { DOWN };
                                                    ui.label(egui::RichText::new(format!("{:.0}", e.net_notional)).color(net_c));
                                                    ui.label(e.darwins.join(", "));
                                                    ui.end_row();
                                                }
                                            });
                                        }
                                    }
                                }
                                Ok(_) => {
                                    ui.label(egui::RichText::new("No DARWIN accounts imported.").color(AXIS_TEXT));
                                }
                                Err(e) => {
                                    ui.label(egui::RichText::new(format!("Error: {}", e)).color(egui::Color32::from_rgb(255, 80, 80)));
                                }
                            }
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new("VaR corridor: 3.25% – 6.5%  |  Correlation limit: 0.95 / 45d").color(AXIS_TEXT));
                        }
                    }
                });
        }

        // Risk Calculator — wired to engine risk.rs
        if self.show_risk_calc {
            egui::Window::new("Risk Calculator")
                .open(&mut self.show_risk_calc)
                .default_size([400.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("Position Sizing");
                    ui.separator();
                    egui::Grid::new("risk_calc_grid").num_columns(2).show(ui, |ui| {
                        ui.label("Account Equity:"); ui.add(egui::TextEdit::singleline(&mut self.rc_equity).desired_width(120.0));
                        ui.end_row();
                        ui.label("Risk %:"); ui.add(egui::TextEdit::singleline(&mut self.rc_risk_pct).desired_width(120.0));
                        ui.end_row();
                        ui.label("Entry Price:"); ui.add(egui::TextEdit::singleline(&mut self.rc_entry).desired_width(120.0));
                        ui.end_row();
                        ui.label("Stop Loss:"); ui.add(egui::TextEdit::singleline(&mut self.rc_sl).desired_width(120.0));
                        ui.end_row();
                        ui.label("Take Profit:"); ui.add(egui::TextEdit::singleline(&mut self.rc_tp).desired_width(120.0));
                        ui.end_row();
                        ui.label("Tick Value:"); ui.add(egui::TextEdit::singleline(&mut self.rc_tick_value).desired_width(120.0));
                        ui.end_row();
                        ui.label("Tick Size:"); ui.add(egui::TextEdit::singleline(&mut self.rc_tick_size).desired_width(120.0));
                        ui.end_row();
                    });
                    ui.add_space(10.0);
                    if ui.button("Calculate").clicked() {
                        let equity: f64 = self.rc_equity.replace(['$', ','], "").parse().unwrap_or(0.0);
                        let risk_pct: f64 = self.rc_risk_pct.parse().unwrap_or(1.0);
                        let entry: f64 = self.rc_entry.parse().unwrap_or(0.0);
                        let sl: f64 = self.rc_sl.parse().unwrap_or(0.0);
                        let tp: f64 = self.rc_tp.parse().unwrap_or(0.0);
                        let tick_val: f64 = self.rc_tick_value.parse().unwrap_or(1.0);
                        let tick_sz: f64 = self.rc_tick_size.parse().unwrap_or(0.01);

                        if equity > 0.0 && entry > 0.0 && sl > 0.0 {
                            let sl_distance = (entry - sl).abs();
                            let risk_amount = equity * risk_pct / 100.0;
                            let spec = risk::SymbolSpec {
                                symbol: self.symbol_input.clone(),
                                tick_size: tick_sz, tick_value: tick_val,
                                volume_min: 0.01, volume_max: 100.0, volume_step: 0.01,
                                contract_size: 1.0, margin_rate: 0.0,
                            };
                            let lots = risk::risk_lots(&spec, risk_amount, sl_distance);
                            let rr = if tp > 0.0 && sl_distance > 0.0 { (tp - entry).abs() / sl_distance } else { 0.0 };
                            self.rc_result = format!(
                                "Lots: {:.2}\nRisk: ${:.2} ({:.1}%)\nSL Distance: {}\nR:R = {:.2}",
                                lots, risk_amount, risk_pct, format_price(sl_distance), rr
                            );

                            // Margin check
                            let usable = margin::usable_margin(equity, 0.0, 10.0);
                            self.rc_result.push_str(&format!("\nUsable margin: ${:.2}", usable));
                        } else {
                            self.rc_result = "Enter equity, entry, and SL".to_string();
                        }
                    }
                    ui.separator();
                    if !self.rc_result.is_empty() {
                        ui.label(egui::RichText::new(&self.rc_result).monospace().color(egui::Color32::from_rgb(200, 220, 255)));
                    }
                });
        }

        // Backtest — wired to engine backtest.rs
        if self.show_backtest {
            egui::Window::new("Backtest Engine")
                .open(&mut self.show_backtest)
                .default_size([600.0, 500.0])
                .show(ctx, |ui| {
                    ui.heading("Strategy Backtest");
                    ui.separator();
                    let chart = self.charts.get(self.active_tab);
                    let n_bars = chart.map(|c| c.bars.len()).unwrap_or(0);
                    let tf = chart.map(|c| c.timeframe.label()).unwrap_or("—");
                    ui.horizontal(|ui| {
                        ui.label("Symbol:"); ui.label(egui::RichText::new(&self.symbol_input).strong());
                        ui.label("TF:"); ui.label(egui::RichText::new(tf).strong());
                        ui.label("Bars:"); ui.label(egui::RichText::new(format!("{}", n_bars)).strong());
                    });
                    ui.add_space(5.0);
                    ui.horizontal(|ui| {
                        ui.label("Strategy:");
                        ui.radio_value(&mut self.bt_strategy, 0, "SMA Cross");
                        ui.radio_value(&mut self.bt_strategy, 1, "NNFX");
                    });
                    ui.horizontal(|ui| {
                        ui.label("Fast Period:"); ui.add(egui::TextEdit::singleline(&mut self.bt_fast_period).desired_width(50.0));
                        ui.label("Slow Period:"); ui.add(egui::TextEdit::singleline(&mut self.bt_slow_period).desired_width(50.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Initial Equity:"); ui.add(egui::TextEdit::singleline(&mut self.bt_equity).desired_width(80.0));
                    });

                    ui.add_space(5.0);
                    if ui.button("Run Backtest").clicked() && n_bars > 0 {
                        if let Some(chart) = self.charts.get(self.active_tab) {
                            let engine_bars: Vec<EngineBar> = chart.bars.iter().map(|b| EngineBar {
                                timestamp: format_ts(b.ts_ms, chart.timeframe),
                                open: b.open, high: b.high, low: b.low, close: b.close, volume: b.volume,
                            }).collect();
                            let fast: usize = self.bt_fast_period.parse().unwrap_or(10);
                            let slow: usize = self.bt_slow_period.parse().unwrap_or(50);
                            let equity: f64 = self.bt_equity.replace(['$', ','], "").parse().unwrap_or(10000.0);

                            let result = if self.bt_strategy == 0 {
                                let mut strat = backtest::SMACrossStrategy::new(fast, slow);
                                backtest::run_backtest(&engine_bars, &mut strat, equity)
                            } else {
                                let mut strat = backtest::NNFXStrategy::new(fast, slow);
                                backtest::run_backtest(&engine_bars, &mut strat, equity)
                            };
                            self.bt_result = Some(result.report);
                            self.bt_trades = result.trades;
                            self.log.push_back(LogEntry::info(format!(
                                "Backtest complete: {} trades, PF={:.2}, WR={:.1}%",
                                self.bt_trades.len(),
                                self.bt_result.as_ref().map(|r| r.profit_factor).unwrap_or(0.0),
                                self.bt_result.as_ref().map(|r| r.win_rate * 100.0).unwrap_or(0.0),
                            )));
                        }
                    }

                    // Results
                    if let Some(ref report) = self.bt_result {
                        ui.add_space(10.0);
                        ui.heading("Results");
                        ui.separator();
                        egui::Grid::new("bt_report").striped(true).num_columns(4).show(ui, |ui| {
                            ui.label("Trades:"); ui.label(format!("{}", report.total_trades));
                            ui.label("Win Rate:"); ui.label(format!("{:.1}%", report.win_rate * 100.0));
                            ui.end_row();
                            ui.label("Profit Factor:"); ui.label(format!("{:.2}", report.profit_factor));
                            ui.label("Sharpe:"); ui.label(format!("{:.3}", report.sharpe_ratio));
                            ui.end_row();
                            let pnl_c = if report.total_pnl >= 0.0 { UP } else { DOWN };
                            ui.label("Total P&L:"); ui.label(egui::RichText::new(format!("${:.2}", report.total_pnl)).color(pnl_c));
                            ui.label("Max DD:"); ui.label(format!("{:.2}%", report.max_drawdown_pct));
                            ui.end_row();
                            ui.label("Avg Win:"); ui.label(format!("${:.2}", report.avg_win));
                            ui.label("Avg Loss:"); ui.label(format!("${:.2}", report.avg_loss));
                            ui.end_row();
                            ui.label("Max Win Streak:"); ui.label(format!("{}", report.max_consecutive_wins));
                            ui.label("Max Loss Streak:"); ui.label(format!("{}", report.max_consecutive_losses));
                            ui.end_row();
                        });

                        // Trade list
                        if !self.bt_trades.is_empty() {
                            ui.add_space(10.0);
                            ui.collapsing(format!("Trade List ({})", self.bt_trades.len()), |ui| {
                                egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                                    egui::Grid::new("bt_trades").striped(true).num_columns(5).show(ui, |ui| {
                                        ui.strong("#"); ui.strong("Side"); ui.strong("Entry"); ui.strong("Exit"); ui.strong("P&L");
                                        ui.end_row();
                                        for (i, t) in self.bt_trades.iter().enumerate() {
                                            ui.label(format!("{}", i + 1));
                                            ui.label(&t.side);
                                            ui.label(format_price(t.entry_price));
                                            ui.label(format_price(t.exit_price));
                                            let c = if t.pnl >= 0.0 { UP } else { DOWN };
                                            ui.label(egui::RichText::new(format!("{:.2}", t.pnl)).color(c));
                                            ui.end_row();
                                        }
                                    });
                                });
                            });
                        }
                    }
                });
        }

        // Screener — uses cached symbol data
        if self.show_screener {
            egui::Window::new("Symbol Screener")
                .open(&mut self.show_screener)
                .default_size([600.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("Screener");
                    ui.separator();
                    if let Some(ref cache) = self.cache {
                        if let Ok(details) = cache.detailed_stats() {
                            ui.label(format!("{} cached symbols", details.len()));
                            ui.add_space(5.0);
                            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                                egui::Grid::new("screener_grid").striped(true).num_columns(3).show(ui, |ui| {
                                    ui.strong("Symbol / Key"); ui.strong("Bars"); ui.strong("Action");
                                    ui.end_row();
                                    for (key, count, _size) in &details {
                                        ui.label(egui::RichText::new(key).monospace());
                                        ui.label(format!("{}", count));
                                        if ui.small_button("Load").clicked() {
                                            self.log.push_back(LogEntry::info(format!("Screener: load {}", key)));
                                        }
                                        ui.end_row();
                                    }
                                });
                            });
                        }
                    }
                });
        }

        // Optimizer — wired to backtest.optimize_sma_cross
        if self.show_optimizer {
            egui::Window::new("Optimizer")
                .open(&mut self.show_optimizer)
                .default_size([600.0, 500.0])
                .show(ctx, |ui| {
                    ui.heading("SMA Cross Optimizer");
                    ui.separator();
                    let chart = self.charts.get(self.active_tab);
                    let n_bars = chart.map(|c| c.bars.len()).unwrap_or(0);
                    ui.label(format!("Symbol: {}  |  Bars: {}", self.symbol_input, n_bars));
                    ui.horizontal(|ui| {
                        ui.label("Fast range:"); ui.add(egui::TextEdit::singleline(&mut self.opt_fast_range).desired_width(60.0));
                        ui.label("Slow range:"); ui.add(egui::TextEdit::singleline(&mut self.opt_slow_range).desired_width(60.0));
                    });
                    if ui.button("Run Optimization").clicked() && n_bars > 50 {
                        if let Some(chart) = self.charts.get(self.active_tab) {
                            let engine_bars: Vec<EngineBar> = chart.bars.iter().map(|b| EngineBar {
                                timestamp: format_ts(b.ts_ms, chart.timeframe),
                                open: b.open, high: b.high, low: b.low, close: b.close, volume: b.volume,
                            }).collect();
                            let fast: (usize, usize) = parse_range(&self.opt_fast_range, 5, 50);
                            let slow: (usize, usize) = parse_range(&self.opt_slow_range, 20, 200);
                            let report = backtest::optimize_sma_cross(&engine_bars, fast, slow, 10000.0, 20);
                            self.opt_results = report.results;
                            self.log.push_back(LogEntry::info(format!("Optimizer: {} combinations tested", report.total_combinations)));
                        }
                    }
                    if !self.opt_results.is_empty() {
                        ui.add_space(10.0);
                        ui.heading(format!("Top {} Results", self.opt_results.len()));
                        ui.separator();
                        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                            egui::Grid::new("opt_grid").striped(true).num_columns(6).show(ui, |ui| {
                                ui.strong("Fast"); ui.strong("Slow"); ui.strong("Trades"); ui.strong("PF"); ui.strong("Sharpe"); ui.strong("P&L");
                                ui.end_row();
                                for r in &self.opt_results {
                                    ui.label(format!("{}", r.fast_period));
                                    ui.label(format!("{}", r.slow_period));
                                    ui.label(format!("{}", r.total_trades));
                                    ui.label(format!("{:.2}", r.profit_factor));
                                    ui.label(format!("{:.3}", r.sharpe_ratio));
                                    let c = if r.total_pnl >= 0.0 { UP } else { DOWN };
                                    ui.label(egui::RichText::new(format!("${:.0}", r.total_pnl)).color(c));
                                    ui.end_row();
                                }
                            });
                        });
                    }
                });
        }

        // News
        if self.show_news {
            egui::Window::new("News & Events")
                .open(&mut self.show_news)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("Market News");
                    ui.separator();
                    ui.label(egui::RichText::new("News feed requires Alpaca/Finnhub connection.").color(AXIS_TEXT));
                });
        }

        // Economic Calendar
        if self.show_calendar {
            egui::Window::new("Economic Calendar")
                .open(&mut self.show_calendar)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("Economic Calendar");
                    ui.separator();
                    ui.label(egui::RichText::new("GDP, CPI, NFP, FOMC, earnings — requires data feed.").color(AXIS_TEXT));
                });
        }

        // SEC
        if self.show_sec {
            egui::Window::new("SEC Filings")
                .open(&mut self.show_sec)
                .default_size([500.0, 350.0])
                .show(ctx, |ui| {
                    ui.heading("SEC EDGAR Filings");
                    ui.separator();
                    ui.label(egui::RichText::new("10-K, 10-Q, 8-K, proxy — requires EDGAR API.").color(AXIS_TEXT));
                });
        }

        // Insider
        if self.show_insider {
            egui::Window::new("Insider Trades")
                .open(&mut self.show_insider)
                .default_size([500.0, 350.0])
                .show(ctx, |ui| {
                    ui.heading("SEC Form 4 — Insider Transactions");
                    ui.separator();
                    ui.label(egui::RichText::new("Requires Finnhub or SEC EDGAR connection.").color(AXIS_TEXT));
                });
        }

        // Fundamentals
        if self.show_fundamentals {
            egui::Window::new("Fundamentals")
                .open(&mut self.show_fundamentals)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("Company Fundamentals");
                    ui.separator();
                    ui.label(egui::RichText::new("Income statement, balance sheet, cash flow.").color(AXIS_TEXT));
                    ui.label(egui::RichText::new("Requires Finnhub or Yahoo Finance API.").color(AXIS_TEXT).small());
                });
        }

        // Analyst
        if self.show_analyst {
            egui::Window::new("Analyst Ratings")
                .open(&mut self.show_analyst)
                .default_size([400.0, 300.0])
                .show(ctx, |ui| {
                    ui.heading("Analyst Recommendations");
                    ui.separator();
                    ui.label(egui::RichText::new("Buy/Hold/Sell ratings, price targets.").color(AXIS_TEXT));
                    ui.label(egui::RichText::new("Requires Finnhub API.").color(AXIS_TEXT).small());
                });
        }

        // Holders
        if self.show_holders {
            egui::Window::new("Institutional Holders")
                .open(&mut self.show_holders)
                .default_size([500.0, 350.0])
                .show(ctx, |ui| {
                    ui.heading("Top Institutional Holders");
                    ui.separator();
                    ui.label(egui::RichText::new("13F filings — requires SEC EDGAR API.").color(AXIS_TEXT));
                });
        }

        // Symbol Overlap — wired to darwin.rs exposure
        if self.show_symbol_overlap {
            egui::Window::new("Symbol Overlap")
                .open(&mut self.show_symbol_overlap)
                .default_size([600.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("Cross-DARWIN Symbol Overlap");
                    ui.separator();
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = darwin::create_darwin_tables(&conn);
                            if let Ok(exposure) = darwin::get_portfolio_exposure(&conn) {
                                if !exposure.is_empty() {
                                    // Symbols traded by multiple DARWINs
                                    let overlaps: Vec<_> = exposure.iter().filter(|e| e.darwin_count > 1).collect();
                                    if overlaps.is_empty() {
                                        ui.label("No overlapping symbols across DARWINs.");
                                    } else {
                                        ui.label(egui::RichText::new(format!("{} overlapping symbols", overlaps.len())).strong());
                                        egui::Grid::new("overlap_grid").striped(true).num_columns(5).show(ui, |ui| {
                                            ui.strong("Symbol"); ui.strong("Long $"); ui.strong("Short $"); ui.strong("Net $"); ui.strong("DARWINs");
                                            ui.end_row();
                                            for e in &overlaps {
                                                ui.label(&e.symbol);
                                                ui.label(format!("{:.0}", e.long_notional));
                                                ui.label(format!("{:.0}", e.short_notional));
                                                let c = if e.net_notional >= 0.0 { UP } else { DOWN };
                                                ui.label(egui::RichText::new(format!("{:.0}", e.net_notional)).color(c));
                                                ui.label(e.darwins.join(", "));
                                                ui.end_row();
                                            }
                                        });
                                    }
                                } else {
                                    ui.label(egui::RichText::new("Import DARWIN data first.").color(AXIS_TEXT));
                                }
                            }
                        }
                    }
                });
        }

        // Correlation Matrix — wired to darwin.rs correlations
        if self.show_correlation {
            egui::Window::new("Correlation Matrix")
                .open(&mut self.show_correlation)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("DARWIN Correlation Matrix");
                    ui.separator();
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = darwin::create_darwin_tables(&conn);
                            if let Ok(corrs) = darwin::get_darwin_correlations(&conn) {
                                if corrs.is_empty() {
                                    ui.label(egui::RichText::new("Need 2+ DARWINs imported for correlation.").color(AXIS_TEXT));
                                } else {
                                    let high_corr: Vec<_> = corrs.iter().filter(|c| c.correlation.abs() > 0.7).collect();
                                    if !high_corr.is_empty() {
                                        ui.label(egui::RichText::new(format!("{} high-correlation pairs (>0.7)", high_corr.len())).color(egui::Color32::from_rgb(255, 200, 50)));
                                    }
                                    egui::Grid::new("corr_matrix").striped(true).num_columns(3).show(ui, |ui| {
                                        ui.strong("DARWIN A"); ui.strong("DARWIN B"); ui.strong("Correlation");
                                        ui.end_row();
                                        for c in &corrs {
                                            ui.label(&c.darwin_a);
                                            ui.label(&c.darwin_b);
                                            let color = if c.correlation.abs() > 0.95 { egui::Color32::from_rgb(255, 40, 40) }
                                                        else if c.correlation.abs() > 0.7 { egui::Color32::from_rgb(255, 200, 50) }
                                                        else { UP };
                                            ui.label(egui::RichText::new(format!("{:.4}", c.correlation)).color(color));
                                            ui.end_row();
                                        }
                                    });
                                    ui.add_space(5.0);
                                    ui.label(egui::RichText::new("Darwinex limit: 0.95 correlation / 45d").color(AXIS_TEXT));
                                }
                            }
                        }
                    }
                });
        }

        // Seasonals — computed from loaded chart bar data
        if self.show_seasonals {
            egui::Window::new("Seasonal Patterns")
                .open(&mut self.show_seasonals)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("Seasonality Analysis");
                    ui.separator();
                    if let Some(chart) = self.charts.get(self.active_tab) {
                        if chart.bars.len() > 30 {
                            // Compute monthly returns from bar data
                            let mut monthly: std::collections::HashMap<u32, Vec<f64>> = std::collections::HashMap::new();
                            for w in chart.bars.windows(2) {
                                let dt = chrono::DateTime::from_timestamp_millis(w[1].ts_ms);
                                if let Some(dt) = dt {
                                    let month = dt.format("%m").to_string().parse::<u32>().unwrap_or(0);
                                    let ret = (w[1].close - w[0].close) / w[0].close * 100.0;
                                    monthly.entry(month).or_default().push(ret);
                                }
                            }
                            let months = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];
                            egui::Grid::new("seasonal_grid").striped(true).num_columns(4).show(ui, |ui| {
                                ui.strong("Month"); ui.strong("Avg Return %"); ui.strong("Win Rate %"); ui.strong("Samples");
                                ui.end_row();
                                for (i, name) in months.iter().enumerate() {
                                    let m = (i + 1) as u32;
                                    if let Some(rets) = monthly.get(&m) {
                                        if !rets.is_empty() {
                                            let avg: f64 = rets.iter().sum::<f64>() / rets.len() as f64;
                                            let wins = rets.iter().filter(|&&r| r > 0.0).count();
                                            let wr = wins as f64 / rets.len() as f64 * 100.0;
                                            let c = if avg >= 0.0 { UP } else { DOWN };
                                            ui.label(*name);
                                            ui.label(egui::RichText::new(format!("{:.3}", avg)).color(c));
                                            ui.label(format!("{:.1}", wr));
                                            ui.label(format!("{}", rets.len()));
                                            ui.end_row();
                                        }
                                    }
                                }
                            });
                        } else {
                            ui.label(egui::RichText::new("Need more bar data for seasonal analysis.").color(AXIS_TEXT));
                        }
                    }
                });
        }

        // Monte Carlo — simulation using DARWIN daily returns or bar data
        if self.show_montecarlo {
            egui::Window::new("Monte Carlo VaR")
                .open(&mut self.show_montecarlo)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("Monte Carlo Simulation");
                    ui.separator();
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = darwin::create_darwin_tables(&conn);
                            if let Ok(daily) = darwin::get_portfolio_daily_returns(&conn) {
                                if daily.len() > 30 {
                                    let var_stats = darwin::compute_var(&daily);
                                    egui::Grid::new("mc_grid").striped(true).num_columns(2).show(ui, |ui| {
                                        ui.label("Trading Days:"); ui.label(format!("{}", var_stats.trading_days));
                                        ui.end_row();
                                        ui.label("VaR 95% (daily):"); ui.label(format!("${:.2}", var_stats.var_95));
                                        ui.end_row();
                                        ui.label("VaR 99% (daily):"); ui.label(format!("${:.2}", var_stats.var_99));
                                        ui.end_row();
                                        ui.label("CVaR 95%:"); ui.label(format!("${:.2}", var_stats.cvar_95));
                                        ui.end_row();
                                        ui.label("CVaR 99%:"); ui.label(format!("${:.2}", var_stats.cvar_99));
                                        ui.end_row();
                                        ui.label("Daily Volatility:"); ui.label(format!("{:.4}", var_stats.daily_vol));
                                        ui.end_row();
                                        ui.label("Annualized Vol:"); ui.label(format!("{:.4}", var_stats.annualized_vol));
                                        ui.end_row();
                                        ui.label("Sharpe Ratio:"); ui.label(format!("{:.3}", var_stats.sharpe));
                                        ui.end_row();
                                        ui.label("Sortino Ratio:"); ui.label(format!("{:.3}", var_stats.sortino));
                                        ui.end_row();
                                        ui.label("Calmar Ratio:"); ui.label(format!("{:.3}", var_stats.calmar));
                                        ui.end_row();
                                        ui.label("Max Drawdown:"); ui.label(format!("{:.2}%", var_stats.max_drawdown_pct));
                                        ui.end_row();
                                        ui.label("Avg Daily P&L:"); ui.label(format!("${:.2}", var_stats.avg_daily_pnl));
                                        ui.end_row();
                                        ui.label("Best Day:"); ui.label(egui::RichText::new(format!("${:.2}", var_stats.best_day)).color(UP));
                                        ui.end_row();
                                        ui.label("Worst Day:"); ui.label(egui::RichText::new(format!("${:.2}", var_stats.worst_day)).color(DOWN));
                                        ui.end_row();
                                    });
                                } else {
                                    ui.label(egui::RichText::new("Need 30+ daily returns for Monte Carlo. Import DARWIN data.").color(AXIS_TEXT));
                                }
                            }
                        }
                    }
                });
        }

        // Stress Test — apply drawdown scenarios to portfolio
        if self.show_stress_test {
            egui::Window::new("Stress Test")
                .open(&mut self.show_stress_test)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("Portfolio Stress Test");
                    ui.separator();
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = darwin::create_darwin_tables(&conn);
                            if let Ok(portfolio) = darwin::get_portfolio_summary(&conn) {
                                if !portfolio.accounts.is_empty() {
                                    let equity = portfolio.total_final_balance;
                                    ui.label(format!("Current portfolio equity: ${:.2}", equity));
                                    ui.add_space(10.0);
                                    let scenarios = [
                                        ("2008 GFC", -56.8),
                                        ("COVID Mar 2020", -33.9),
                                        ("2022 Bear Market", -25.4),
                                        ("Flash Crash 2010", -9.0),
                                        ("Brexit Vote 2016", -5.3),
                                        ("10% Correction", -10.0),
                                        ("20% Bear", -20.0),
                                        ("50% Crash", -50.0),
                                    ];
                                    egui::Grid::new("stress_grid").striped(true).num_columns(3).show(ui, |ui| {
                                        ui.strong("Scenario"); ui.strong("Drawdown"); ui.strong("Equity After");
                                        ui.end_row();
                                        for (name, dd_pct) in &scenarios {
                                            let after = equity * (1.0 + dd_pct / 100.0);
                                            let loss = equity - after;
                                            ui.label(*name);
                                            ui.label(egui::RichText::new(format!("{:.1}%", dd_pct)).color(DOWN));
                                            ui.label(egui::RichText::new(format!("${:.2} (−${:.2})", after, loss)).color(DOWN));
                                            ui.end_row();
                                        }
                                    });
                                    ui.add_space(5.0);
                                    ui.label(format!("Max historical DD: {:.2}%", portfolio.combined_max_drawdown_pct));
                                } else {
                                    ui.label(egui::RichText::new("Import DARWIN data for stress testing.").color(AXIS_TEXT));
                                }
                            }
                        }
                    }
                });
        }

        // Volume Profile — computed from loaded chart bars
        if self.show_volume_profile {
            egui::Window::new("Volume Profile")
                .open(&mut self.show_volume_profile)
                .default_size([400.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("Volume Profile");
                    ui.separator();
                    if let Some(chart) = self.charts.get(self.active_tab) {
                        let (si, ei) = chart.visible_range();
                        let bars = &chart.bars[si..ei];
                        if bars.len() > 10 {
                            // Build volume-at-price histogram
                            let price_min = bars.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                            let price_max = bars.iter().map(|b| b.high).fold(f64::MIN, f64::max);
                            let num_bins = 30;
                            let bin_size = (price_max - price_min) / num_bins as f64;
                            if bin_size > 0.0 {
                                let mut bins = vec![0.0_f64; num_bins];
                                for b in bars {
                                    let mid = (b.high + b.low) / 2.0;
                                    let idx = ((mid - price_min) / bin_size).floor() as usize;
                                    let idx = idx.min(num_bins - 1);
                                    bins[idx] += b.volume;
                                }
                                let max_vol = bins.iter().fold(0.0_f64, |a, &b| a.max(b));
                                let poc_idx = bins.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).map(|(i, _)| i).unwrap_or(0);
                                let poc_price = price_min + (poc_idx as f64 + 0.5) * bin_size;
                                ui.label(egui::RichText::new(format!("POC: {}", format_price(poc_price))).strong().color(ACCENT));

                                // Value Area (70% of volume)
                                let total_vol: f64 = bins.iter().sum();
                                let va_target = total_vol * 0.7;
                                let mut va_vol = bins[poc_idx];
                                let mut va_lo = poc_idx;
                                let mut va_hi = poc_idx;
                                while va_vol < va_target && (va_lo > 0 || va_hi < num_bins - 1) {
                                    let expand_lo = if va_lo > 0 { bins[va_lo - 1] } else { 0.0 };
                                    let expand_hi = if va_hi < num_bins - 1 { bins[va_hi + 1] } else { 0.0 };
                                    if expand_lo >= expand_hi && va_lo > 0 { va_lo -= 1; va_vol += bins[va_lo]; }
                                    else if va_hi < num_bins - 1 { va_hi += 1; va_vol += bins[va_hi]; }
                                    else { break; }
                                }
                                let vah = price_min + (va_hi as f64 + 1.0) * bin_size;
                                let val = price_min + va_lo as f64 * bin_size;
                                ui.label(format!("VAH: {}  |  VAL: {}", format_price(vah), format_price(val)));
                                ui.add_space(5.0);

                                // Horizontal bar chart
                                let avail = ui.available_size();
                                let (rect, _) = ui.allocate_exact_size(egui::vec2(avail.x, 250.0), egui::Sense::hover());
                                let painter = ui.painter_at(rect);
                                painter.rect_filled(rect, 0.0, BG);
                                let row_h = rect.height() / num_bins as f32;
                                for (i, &vol) in bins.iter().enumerate().rev() {
                                    let frac = if max_vol > 0.0 { vol / max_vol } else { 0.0 };
                                    let y = rect.top() + (num_bins - 1 - i) as f32 * row_h;
                                    let w = frac as f32 * rect.width() * 0.85;
                                    let color = if i == poc_idx { ACCENT }
                                        else if i >= va_lo && i <= va_hi {
                                            egui::Color32::from_rgba_premultiplied(76, 175, 80, 100)
                                        } else {
                                            egui::Color32::from_rgba_premultiplied(100, 100, 140, 80)
                                        };
                                    painter.rect_filled(
                                        egui::Rect::from_min_size(egui::pos2(rect.left(), y), egui::vec2(w, row_h - 1.0)),
                                        0.0, color,
                                    );
                                    // Price label
                                    let price = price_min + (i as f64 + 0.5) * bin_size;
                                    painter.text(
                                        egui::pos2(rect.right() - 2.0, y + row_h * 0.5),
                                        egui::Align2::RIGHT_CENTER,
                                        format_price(price),
                                        egui::FontId::monospace(8.0),
                                        AXIS_TEXT,
                                    );
                                }
                            }
                        } else {
                            ui.label(egui::RichText::new("Need visible bar data for volume profile.").color(AXIS_TEXT));
                        }
                    }
                });
        }

        // Order Flow
        if self.show_order_flow {
            egui::Window::new("Order Flow")
                .open(&mut self.show_order_flow)
                .default_size([400.0, 350.0])
                .show(ctx, |ui| {
                    ui.heading("Order Flow / Delta");
                    ui.separator();
                    ui.label(egui::RichText::new("Bid/ask delta, cumulative delta, footprint.").color(AXIS_TEXT));
                    ui.label(egui::RichText::new("Requires Level 2 data feed.").color(AXIS_TEXT).small());
                });
        }

        // Bookmap
        if self.show_bookmap {
            egui::Window::new("Bookmap Heatmap")
                .open(&mut self.show_bookmap)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("Depth Heatmap");
                    ui.separator();
                    ui.label(egui::RichText::new("Real-time order book heatmap (Bookmap-style).").color(AXIS_TEXT));
                    ui.label(egui::RichText::new("See ADR-048 for architecture. Requires WebSocket L2 data.").color(AXIS_TEXT).small());
                    ui.label(egui::RichText::new("wgpu compute shader pipeline — after Phase 5.").color(AXIS_TEXT).small());
                });
        }

        // VaR Multiplier — wired to DARWIN VaR data
        if self.show_var_mult {
            egui::Window::new("VaR Multiplier")
                .open(&mut self.show_var_mult)
                .default_size([450.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("Darwinex VaR Corridor");
                    ui.separator();
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = darwin::create_darwin_tables(&conn);
                            if let Ok(accounts) = darwin::list_darwin_accounts(&conn) {
                                if !accounts.is_empty() {
                                    egui::Grid::new("var_per_darwin").striped(true).num_columns(5).show(ui, |ui| {
                                        ui.strong("DARWIN"); ui.strong("VaR 95%"); ui.strong("Vol"); ui.strong("Sharpe"); ui.strong("Status");
                                        ui.end_row();
                                        for acct in &accounts {
                                            if let Ok(daily) = darwin::get_daily_returns(&conn, &acct.darwin_ticker) {
                                                if !daily.is_empty() {
                                                    let vs = darwin::compute_var(&daily);
                                                    ui.label(&acct.darwin_ticker);
                                                    ui.label(format!("${:.2}", vs.var_95));
                                                    ui.label(format!("{:.4}", vs.annualized_vol));
                                                    ui.label(format!("{:.3}", vs.sharpe));
                                                    // VaR corridor status
                                                    let var_pct = vs.annualized_vol * 100.0;
                                                    let status = if var_pct >= 3.25 && var_pct <= 6.5 { ("IN", UP) }
                                                                 else if var_pct < 3.25 { ("LOW", egui::Color32::from_rgb(255, 200, 50)) }
                                                                 else { ("HIGH", DOWN) };
                                                    ui.label(egui::RichText::new(status.0).color(status.1).strong());
                                                    ui.end_row();
                                                }
                                            }
                                        }
                                    });
                                } else {
                                    ui.label(egui::RichText::new("Import DARWIN data first.").color(AXIS_TEXT));
                                }
                            }
                        }
                    }
                    ui.add_space(10.0);
                    ui.separator();
                    egui::Grid::new("var_rules").num_columns(2).show(ui, |ui| {
                        ui.label("Target corridor:"); ui.label(egui::RichText::new("3.25% – 6.5%").strong());
                        ui.end_row();
                        ui.label("Correlation limit:"); ui.label(egui::RichText::new("0.95 / 45d").strong());
                        ui.end_row();
                        ui.label("Margin accounts:"); ui.label(egui::RichText::new("100%").strong());
                        ui.end_row();
                    });
                });
        }

        // Margin Monitor — wired to margin.rs functions
        if self.show_margin_monitor {
            egui::Window::new("Margin Monitor")
                .open(&mut self.show_margin_monitor)
                .default_size([450.0, 350.0])
                .show(ctx, |ui| {
                    ui.heading("Margin Calculator");
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Equity:"); ui.add(egui::TextEdit::singleline(&mut self.mm_equity).desired_width(100.0));
                        ui.label("Margin Used:"); ui.add(egui::TextEdit::singleline(&mut self.mm_margin).desired_width(100.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Margin/Lot:"); ui.add(egui::TextEdit::singleline(&mut self.mm_margin_per_lot).desired_width(100.0));
                        ui.label("TRIM %:"); ui.add(egui::TextEdit::singleline(&mut self.mm_trim_pct).desired_width(60.0));
                    });
                    if ui.button("Calculate").clicked() {
                        let equity: f64 = self.mm_equity.replace(['$', ','], "").parse().unwrap_or(0.0);
                        let margin_used: f64 = self.mm_margin.replace(['$', ','], "").parse().unwrap_or(0.0);
                        let margin_per_lot: f64 = self.mm_margin_per_lot.replace(['$', ','], "").parse().unwrap_or(1000.0);
                        let trim_pct: f64 = self.mm_trim_pct.parse().unwrap_or(150.0);
                        if equity > 0.0 {
                            let ml = margin::margin_level_pct(equity, margin_used);
                            let free = margin::usable_margin(equity, margin_used, 10.0);
                            let max_lots = margin::max_safe_lots(equity, margin_used, margin_per_lot, trim_pct);
                            let urgency = margin::protect_urgency(ml, trim_pct);
                            self.mm_result = format!(
                                "Margin Level: {:.2}%\nFree Margin: ${:.2}\nMax Safe Lots: {}\nProtect Urgency: {:.2}",
                                ml, free, max_lots, urgency
                            );
                        }
                    }
                    if !self.mm_result.is_empty() {
                        ui.separator();
                        ui.label(egui::RichText::new(&self.mm_result).monospace().color(egui::Color32::from_rgb(200, 220, 255)));
                    }
                });
        }

        // Cache Stats
        if self.show_cache_stats {
            egui::Window::new("Cache Statistics")
                .open(&mut self.show_cache_stats)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("SQLite Cache");
                    ui.separator();
                    if let Some(ref cache) = self.cache {
                        if let Ok((rows, kv, size)) = cache.stats() {
                            ui.label(format!("Bar entries: {}", rows));
                            ui.label(format!("KV entries: {}", kv));
                            ui.label(format!("DB size: {} KB", size / 1024));
                        }
                        ui.add_space(10.0);
                        if let Ok(details) = cache.detailed_stats() {
                            ui.heading("Cached Symbols");
                            ui.separator();
                            egui::ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
                                egui::Grid::new("cache_detail").striped(true).num_columns(3).show(ui, |ui| {
                                    ui.strong("Key"); ui.strong("Bars"); ui.strong("Size (KB)");
                                    ui.end_row();
                                    for (key, count, size) in &details {
                                        ui.label(key);
                                        ui.label(format!("{}", count));
                                        ui.label(format!("{}", size / 1024));
                                        ui.end_row();
                                    }
                                });
                            });
                        }
                    } else {
                        ui.label(egui::RichText::new("Cache not available").color(egui::Color32::from_rgb(255, 80, 80)));
                    }
                });
        }

        // Help
        if self.show_help {
            egui::Window::new("Keyboard Shortcuts")
                .open(&mut self.show_help)
                .default_size([400.0, 350.0])
                .show(ctx, |ui| {
                    ui.heading("Shortcuts");
                    ui.separator();
                    egui::Grid::new("help_grid").striped(true).num_columns(2).show(ui, |ui| {
                        ui.strong("Key"); ui.strong("Action");
                        ui.end_row();
                        ui.label("~ (tilde)"); ui.label("Command palette (Quake console)");
                        ui.end_row();
                        ui.label("Esc"); ui.label("Close palette / cancel drawing");
                        ui.end_row();
                        ui.label("Scroll wheel"); ui.label("Zoom chart (horizontal)");
                        ui.end_row();
                        ui.label("Ctrl + scroll"); ui.label("Zoom chart (vertical / price)");
                        ui.end_row();
                        ui.label("Double-click"); ui.label("Reset zoom & pan");
                        ui.end_row();
                        ui.label("Click + drag"); ui.label("Pan chart");
                        ui.end_row();
                        ui.label("← →"); ui.label("Bar-by-bar scroll");
                        ui.end_row();
                        ui.label("Home / End"); ui.label("Jump to start / end");
                        ui.end_row();
                        ui.label("PgUp / PgDn"); ui.label("Half-screen scroll");
                        ui.end_row();
                        ui.label("+ / -"); ui.label("Zoom in / out");
                        ui.end_row();
                        ui.label("Delete / Backspace"); ui.label("Remove last drawing");
                        ui.end_row();
                        ui.label("Right-click"); ui.label("Context menu (drawings, chart type)");
                        ui.end_row();
                        ui.label("Ctrl+N"); ui.label("New tab");
                        ui.end_row();
                        ui.label("Ctrl+W"); ui.label("Close tab");
                        ui.end_row();
                        ui.label("Ctrl+Tab"); ui.label("Next tab");
                        ui.end_row();
                        ui.label("Ctrl+Shift+Tab"); ui.label("Previous tab");
                        ui.end_row();
                        ui.label("Alt+F4"); ui.label("Quit");
                        ui.end_row();
                    });
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("TyphooN Terminal — Pure Rust GPU").color(ACCENT));
                });
        }

        // Data Window — all indicator values at crosshair position
        if self.show_data_window {
            egui::Window::new("Data Window")
                .open(&mut self.show_data_window)
                .default_size([280.0, 400.0])
                .show(ctx, |ui| {
                    if let Some(chart) = self.charts.get(self.active_tab) {
                        let (si, ei) = chart.visible_range();
                        let bars = &chart.bars[si..ei];
                        if let Some(_pos) = self.crosshair {
                            // Find bar index from crosshair
                            if !bars.is_empty() {
                                let price_axis_w = 70.0_f32;
                                let _bar_w = (ui.available_width() + price_axis_w) / bars.len() as f32; // approximate
                                let _rel_idx = 0.max(bars.len() / 2); // fallback to middle if we can't calculate
                                // Use most recent bar as fallback
                                let abs_idx = ei.saturating_sub(1);
                                let b = &chart.bars[abs_idx];
                                ui.heading(format!("{} [{}]", chart.symbol, chart.timeframe.label()));
                                ui.separator();
                                egui::Grid::new("data_grid").striped(true).num_columns(2).show(ui, |ui| {
                                    ui.label("Open"); ui.label(format_price(b.open)); ui.end_row();
                                    ui.label("High"); ui.label(format_price(b.high)); ui.end_row();
                                    ui.label("Low"); ui.label(format_price(b.low)); ui.end_row();
                                    ui.label("Close"); ui.label(format_price(b.close)); ui.end_row();
                                    ui.label("Volume"); ui.label(format!("{:.0}", b.volume)); ui.end_row();
                                    ui.end_row();
                                    if let Some(Some(v)) = chart.sma200.get(abs_idx) { ui.label(egui::RichText::new("SMA200").color(SMA200_COL)); ui.label(format_price(*v)); ui.end_row(); }
                                    if let Some(Some(v)) = chart.sma100.get(abs_idx) { ui.label(egui::RichText::new("SMA100").color(SMA100_COL)); ui.label(format_price(*v)); ui.end_row(); }
                                    if let Some(Some(v)) = chart.ema21.get(abs_idx) { ui.label(egui::RichText::new("EMA21").color(EMA_COL)); ui.label(format_price(*v)); ui.end_row(); }
                                    if let Some(Some(v)) = chart.kama.get(abs_idx) { ui.label(egui::RichText::new("KAMA").color(KAMA_COL)); ui.label(format_price(*v)); ui.end_row(); }
                                    if let Some(Some(v)) = chart.wma.get(abs_idx) { ui.label(egui::RichText::new("WMA20").color(WMA_COL)); ui.label(format_price(*v)); ui.end_row(); }
                                    if let Some(Some(v)) = chart.hma.get(abs_idx) { ui.label(egui::RichText::new("HMA20").color(HMA_COL)); ui.label(format_price(*v)); ui.end_row(); }
                                    if let Some(Some(v)) = chart.bb_upper.get(abs_idx) { ui.label(egui::RichText::new("BB Upper").color(BB_COL)); ui.label(format_price(*v)); ui.end_row(); }
                                    if let Some(Some(v)) = chart.bb_lower.get(abs_idx) { ui.label(egui::RichText::new("BB Lower").color(BB_COL)); ui.label(format_price(*v)); ui.end_row(); }
                                    if let Some(Some(v)) = chart.rsi.get(abs_idx) { ui.label(egui::RichText::new("RSI").color(RSI_LINE)); ui.label(format!("{:.1}", v)); ui.end_row(); }
                                    if let Some(Some(v)) = chart.fisher.get(abs_idx) { ui.label(egui::RichText::new("Fisher").color(FISHER_POS)); ui.label(format!("{:.3}", v)); ui.end_row(); }
                                    if let Some(Some(v)) = chart.atr.get(abs_idx) { ui.label("ATR"); ui.label(format_price(*v)); ui.end_row(); }
                                    if let Some(Some(v)) = chart.macd_line.get(abs_idx) { ui.label(egui::RichText::new("MACD").color(MACD_LINE_COL)); ui.label(format!("{:.4}", v)); ui.end_row(); }
                                    if let Some(Some(v)) = chart.stoch_k.get(abs_idx) { ui.label(egui::RichText::new("Stoch %K").color(STOCH_K_COL)); ui.label(format!("{:.1}", v)); ui.end_row(); }
                                    if let Some(Some(v)) = chart.adx.get(abs_idx) { ui.label(egui::RichText::new("ADX").color(ADX_COL)); ui.label(format!("{:.1}", v)); ui.end_row(); }
                                    if let Some(Some(v)) = chart.cci.get(abs_idx) { ui.label(egui::RichText::new("CCI").color(CCI_COL)); ui.label(format!("{:.1}", v)); ui.end_row(); }
                                    if let Some(Some(v)) = chart.williams_r.get(abs_idx) { ui.label(egui::RichText::new("W%R").color(WILLR_COL)); ui.label(format!("{:.1}", v)); ui.end_row(); }
                                    if let Some(Some(v)) = chart.momentum.get(abs_idx) { ui.label("Momentum"); ui.label(format_price(*v)); ui.end_row(); }
                                    if let Some(Some(v)) = chart.psar.get(abs_idx) { ui.label(egui::RichText::new("P.SAR").color(SAR_COL)); ui.label(format_price(*v)); ui.end_row(); }
                                });
                            }
                        } else {
                            ui.label(egui::RichText::new("Move cursor over chart").color(AXIS_TEXT));
                        }
                    }
                });
        }

        // Price Alerts
        if self.show_alerts {
            egui::Window::new("Price Alerts")
                .open(&mut self.show_alerts)
                .default_size([350.0, 300.0])
                .show(ctx, |ui| {
                    ui.heading("Alerts");
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Price:");
                        ui.add(egui::TextEdit::singleline(&mut self.alert_price_input).desired_width(100.0));
                        ui.label("Label:");
                        ui.add(egui::TextEdit::singleline(&mut self.alert_label_input).desired_width(100.0));
                    });
                    if ui.button("Add Alert").clicked() {
                        if let Ok(price) = self.alert_price_input.parse::<f64>() {
                            let label = if self.alert_label_input.is_empty() {
                                format_price(price)
                            } else {
                                self.alert_label_input.clone()
                            };
                            self.alerts.push((price, label));
                            self.alert_price_input.clear();
                            self.alert_label_input.clear();
                            self.log.push_back(LogEntry::info(format!("Alert set at {}", format_price(price))));
                        }
                    }
                    ui.separator();
                    if self.alerts.is_empty() {
                        ui.label(egui::RichText::new("No alerts set.").color(AXIS_TEXT));
                    } else {
                        let mut remove_idx: Option<usize> = None;
                        for (i, (price, label)) in self.alerts.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(format_price(*price)).strong().monospace());
                                ui.label(label);
                                if ui.small_button("X").clicked() {
                                    remove_idx = Some(i);
                                }
                            });
                        }
                        if let Some(idx) = remove_idx {
                            self.alerts.remove(idx);
                        }
                        if ui.button("Clear All Alerts").clicked() {
                            self.alerts.clear();
                        }
                    }

                    // Check alerts against current price
                    if let Some(chart) = self.charts.get(self.active_tab) {
                        if let Some(last) = chart.bars.last() {
                            for (price, label) in &self.alerts {
                                let dist = (last.close - price).abs();
                                let pct = dist / last.close * 100.0;
                                if pct < 0.1 {
                                    ui.label(egui::RichText::new(format!("ALERT TRIGGERED: {} at {}", label, format_price(*price)))
                                        .color(egui::Color32::from_rgb(255, 80, 80)).strong());
                                }
                            }
                        }
                    }
                });
        }

        // DARWIN Equity Curve (uses egui_plot)
        if self.show_darwin_portfolio {
            // Already handled above, but add equity curve to portfolio if we have data
        }
    }
}

// ─── platform helper ─────────────────────────────────────────────────────────

fn dirs_home() -> PathBuf {
    let mut p = if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
    } else {
        PathBuf::from("/tmp")
    };
    p.push(".config");
    p.push("typhoon-terminal");
    p
}

// ─── eframe::App ─────────────────────────────────────────────────────────────

impl eframe::App for TyphooNApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame_count += 1;
        ctx.set_visuals(Self::dark_visuals());

        // ── ~ (tilde) → Quake-style command palette ─────────────────────────
        let open_palette = ctx.input(|i| i.key_pressed(egui::Key::Backtick));
        if open_palette {
            self.command_open = !self.command_open;
            if self.command_open { self.command_input.clear(); }
        }

        // ── Esc → close palette ──────────────────────────────────────────────
        if self.command_open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.command_open = false;
        }

        // ── crosshair from pointer ───────────────────────────────────────────
        self.crosshair = ctx.input(|i| i.pointer.hover_pos());

        // ── keyboard shortcuts ───────────────────────────────────────────────
        if !self.command_open {
            let left  = ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft));
            let right = ctx.input(|i| i.key_pressed(egui::Key::ArrowRight));
            let home  = ctx.input(|i| i.key_pressed(egui::Key::Home));
            let end   = ctx.input(|i| i.key_pressed(egui::Key::End));
            let pgup  = ctx.input(|i| i.key_pressed(egui::Key::PageUp));
            let pgdn  = ctx.input(|i| i.key_pressed(egui::Key::PageDown));
            let plus  = ctx.input(|i| i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals));
            let minus = ctx.input(|i| i.key_pressed(egui::Key::Minus));
            let delete = ctx.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace));

            // Ctrl+N = new tab, Ctrl+W = close tab
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::N)) {
                let tf = self.charts.get(self.active_tab).map(|c| c.timeframe).unwrap_or(Timeframe::H4);
                let mut new_chart = ChartState::new(&self.symbol_input, tf);
                if let Some(ref cache) = self.cache.clone() {
                    new_chart.load(Arc::as_ref(cache), &mut self.log);
                }
                self.charts.push(new_chart);
                self.active_tab = self.charts.len() - 1;
            }
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::W)) {
                if self.charts.len() > 1 {
                    self.charts.remove(self.active_tab);
                    if self.active_tab >= self.charts.len() {
                        self.active_tab = self.charts.len().saturating_sub(1);
                    }
                }
            }
            // Ctrl+Tab / Ctrl+Shift+Tab = cycle tabs
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Tab)) {
                if ctx.input(|i| i.modifiers.shift) {
                    self.active_tab = if self.active_tab == 0 { self.charts.len() - 1 } else { self.active_tab - 1 };
                } else {
                    self.active_tab = (self.active_tab + 1) % self.charts.len();
                }
            }

            // Delete = remove last drawing from active chart
            if delete {
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    chart.drawings.pop();
                }
            }

            // Escape = cancel drawing mode
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.draw_mode = DrawMode::None;
            }

            if let Some(chart) = self.charts.get_mut(self.active_tab) {
                if left  { chart.view_offset = chart.view_offset.saturating_sub(1); }
                if right { chart.view_offset = (chart.view_offset + 1).min(chart.bars.len().saturating_sub(1)); }
                if home  { chart.view_offset = chart.visible_bars.min(chart.bars.len()).saturating_sub(1); }
                if end   { chart.view_offset = chart.bars.len().saturating_sub(1); }
                if pgup  { chart.view_offset = chart.view_offset.saturating_sub(chart.visible_bars / 2); }
                if pgdn  { chart.view_offset = (chart.view_offset + chart.visible_bars / 2).min(chart.bars.len().saturating_sub(1)); }
                if plus  { Self::handle_zoom(chart, 1.0); }
                if minus { Self::handle_zoom(chart, -1.0); }
            }
        }

        // ── top menu bar ─────────────────────────────────────────────────────
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Connect to Broker…").clicked() {
                        self.show_connect = true;
                        ui.close_menu();
                    }
                    if ui.button("Settings").clicked() {
                        self.show_settings = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quit  Alt+F4").clicked() {
                        self.save_session();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui.button(if self.mtf_enabled { "Single Chart" } else { "MTF Grid (4 charts)" }).clicked() {
                        self.mtf_enabled = !self.mtf_enabled;
                        ui.close_menu();
                    }
                    if ui.button("Indicators…").clicked() {
                        self.show_indicators_panel = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.label(egui::RichText::new("Chart Type").color(AXIS_TEXT).small());
                    let ct = self.charts.get(self.active_tab).map(|c| c.chart_type).unwrap_or(ChartType::Candle);
                    for &chart_type in &[ChartType::Candle, ChartType::HeikinAshi, ChartType::Line, ChartType::OhlcBars, ChartType::Renko] {
                        let selected = ct == chart_type;
                        let label = if selected { format!("● {}", chart_type.label()) } else { format!("  {}", chart_type.label()) };
                        if ui.button(label).clicked() {
                            if let Some(c) = self.charts.get_mut(self.active_tab) {
                                c.chart_type = chart_type;
                            }
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    ui.label(egui::RichText::new("Overlay Indicators").color(AXIS_TEXT).small());
                    ui.checkbox(&mut self.show_sma200,    "SMA 200");
                    ui.checkbox(&mut self.show_sma100,    "SMA 100");
                    ui.checkbox(&mut self.show_kama,      "KAMA(10,2,30)");
                    ui.checkbox(&mut self.show_ema21,     "EMA 21");
                    ui.checkbox(&mut self.show_bollinger, "Bollinger Bands");
                    ui.separator();
                    ui.checkbox(&mut self.show_ichimoku, "Ichimoku Cloud");
                    ui.checkbox(&mut self.show_wma,      "WMA(20)");
                    ui.checkbox(&mut self.show_hma,      "HMA(20)");
                    ui.checkbox(&mut self.show_psar,     "Parabolic SAR");
                    ui.checkbox(&mut self.show_atr_proj, "ATR Projection");
                    ui.separator();
                    ui.label(egui::RichText::new("Sub-Panes").color(AXIS_TEXT).small());
                    ui.checkbox(&mut self.show_rsi,            "RSI(14)");
                    ui.checkbox(&mut self.show_fisher,         "Fisher Transform");
                    ui.checkbox(&mut self.show_macd,           "MACD(12,26,9)");
                    ui.checkbox(&mut self.show_stochastic,     "Stochastic(14,3,3)");
                    ui.checkbox(&mut self.show_adx,            "ADX(14)");
                    ui.checkbox(&mut self.show_cci,            "CCI(20)");
                    ui.checkbox(&mut self.show_williams_r,     "Williams %R(14)");
                    ui.checkbox(&mut self.show_obv,            "OBV");
                    ui.checkbox(&mut self.show_momentum,       "Momentum(10)");
                    ui.checkbox(&mut self.show_better_volume,  "Better Volume");
                    ui.checkbox(&mut self.show_volume_pane,    "Volume");
                });
                ui.menu_button("Trading", |ui| {
                    if ui.button("Open Trade").clicked() {
                        self.log.push_back(LogEntry::info("Trading: Open Trade — connect to broker first"));
                        ui.close_menu();
                    }
                    if ui.button("Close All").clicked() {
                        self.log.push_back(LogEntry::info("Trading: Close All — connect to broker first"));
                        ui.close_menu();
                    }
                    if ui.button("Close Partial").clicked() {
                        self.log.push_back(LogEntry::info("Trading: Close Partial — connect to broker first"));
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Set SL").clicked() {
                        self.log.push_back(LogEntry::info("Trading: Set SL — connect to broker first"));
                        ui.close_menu();
                    }
                    if ui.button("Set TP").clicked() {
                        self.log.push_back(LogEntry::info("Trading: Set TP — connect to broker first"));
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Open MG (Martingale Hedge)").clicked() {
                        self.log.push_back(LogEntry::info("Trading: Open MG — connect to broker first"));
                        ui.close_menu();
                    }
                    if ui.button("Buy Lines").clicked() {
                        self.draw_mode = DrawMode::PlacingHLine;
                        self.log.push_back(LogEntry::info("Click chart to place buy reference line (green)"));
                        ui.close_menu();
                    }
                    if ui.button("Sell Lines").clicked() {
                        self.draw_mode = DrawMode::PlacingHLine;
                        self.log.push_back(LogEntry::info("Click chart to place sell reference line (red)"));
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Set SL Line").clicked() {
                        self.draw_mode = DrawMode::PlacingHLine;
                        self.log.push_back(LogEntry::info("Click chart to set SL level"));
                        ui.close_menu();
                    }
                    if ui.button("Set TP Line").clicked() {
                        self.draw_mode = DrawMode::PlacingHLine;
                        self.log.push_back(LogEntry::info("Click chart to set TP level"));
                        ui.close_menu();
                    }
                    if self.sl_price.is_some() || self.tp_price.is_some() {
                        if ui.button("Clear SL/TP Lines").clicked() {
                            self.sl_price = None;
                            self.tp_price = None;
                            ui.close_menu();
                        }
                    }
                });
                ui.menu_button("Tools", |ui| {
                    if ui.button("DARWIN Accounts").clicked() {
                        self.show_darwin_accounts = true;
                        ui.close_menu();
                    }
                    if ui.button("DARWIN Portfolio").clicked() {
                        self.show_darwin_portfolio = true;
                        ui.close_menu();
                    }
                    if ui.button("Symbol Overlap").clicked() {
                        self.show_symbol_overlap = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Backtest").clicked() {
                        self.show_backtest = true;
                        ui.close_menu();
                    }
                    if ui.button("Screener").clicked() {
                        self.show_screener = true;
                        ui.close_menu();
                    }
                    if ui.button("Optimizer").clicked() {
                        self.show_optimizer = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Risk Calculator").clicked() {
                        self.show_risk_calc = true;
                        ui.close_menu();
                    }
                    if ui.button("VaR Multiplier").clicked() {
                        self.show_var_mult = true;
                        ui.close_menu();
                    }
                    if ui.button("Margin Monitor").clicked() {
                        self.show_margin_monitor = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Cache Statistics").clicked() {
                        self.show_cache_stats = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Research", |ui| {
                    if ui.button("News & Events").clicked() {
                        self.show_news = true;
                        ui.close_menu();
                    }
                    if ui.button("Economic Calendar").clicked() {
                        self.show_calendar = true;
                        ui.close_menu();
                    }
                    if ui.button("SEC Filings").clicked() {
                        self.show_sec = true;
                        ui.close_menu();
                    }
                    if ui.button("Insider Trades").clicked() {
                        self.show_insider = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Fundamentals").clicked() {
                        self.show_fundamentals = true;
                        ui.close_menu();
                    }
                    if ui.button("Analyst Ratings").clicked() {
                        self.show_analyst = true;
                        ui.close_menu();
                    }
                    if ui.button("Institutional Holders").clicked() {
                        self.show_holders = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Analysis", |ui| {
                    if ui.button("Correlation Matrix").clicked() {
                        self.show_correlation = true;
                        ui.close_menu();
                    }
                    if ui.button("Seasonals").clicked() {
                        self.show_seasonals = true;
                        ui.close_menu();
                    }
                    if ui.button("Monte Carlo VaR").clicked() {
                        self.show_montecarlo = true;
                        ui.close_menu();
                    }
                    if ui.button("Stress Test").clicked() {
                        self.show_stress_test = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Volume Profile").clicked() {
                        self.show_volume_profile = true;
                        ui.close_menu();
                    }
                    if ui.button("Order Flow").clicked() {
                        self.show_order_flow = true;
                        ui.close_menu();
                    }
                    if ui.button("Bookmap Heatmap").clicked() {
                        self.show_bookmap = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("Keyboard Shortcuts").clicked() {
                        self.show_help = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.label(egui::RichText::new("TyphooN Terminal v0.1.0").color(AXIS_TEXT).small());
                    ui.label(egui::RichText::new("Pure Rust GPU — egui + wgpu").color(AXIS_TEXT).small());
                });
                ui.separator();
                ui.label(
                    egui::RichText::new("TyphooN Terminal — Pure Rust GPU")
                        .color(ACCENT)
                        .strong(),
                );
            });
        });

        // ── symbol + timeframe toolbar ───────────────────────────────────────
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT).small());
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.symbol_input)
                        .desired_width(120.0)
                        .font(egui::TextStyle::Monospace),
                );
                if resp.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let sym = self.symbol_input.trim().to_string();
                    let tf  = self.charts.get(self.active_tab).map(|c| c.timeframe).unwrap_or(Timeframe::H4);
                    self.reload_symbol(&sym, tf);
                }

                ui.separator();

                // Timeframe buttons
                let cur_tf = self.charts.get(self.active_tab).map(|c| c.timeframe).unwrap_or(Timeframe::H4);
                for &tf in &[Timeframe::M1, Timeframe::M5, Timeframe::M15, Timeframe::M30,
                              Timeframe::H1, Timeframe::H4, Timeframe::D1, Timeframe::W1, Timeframe::MN1] {
                    let selected = tf == cur_tf;
                    let btn_color = if selected { ACCENT } else { AXIS_TEXT };
                    let btn_text  = egui::RichText::new(tf.label()).color(btn_color).small().strong();
                    if ui.small_button(btn_text).clicked() {
                        let sym = self.symbol_input.trim().to_string();
                        self.reload_symbol(&sym, tf);
                    }
                }

                ui.separator();

                // MTF toggle
                let mtf_txt = if self.mtf_enabled {
                    egui::RichText::new("MTF ON").color(ACCENT).small().strong()
                } else {
                    egui::RichText::new("MTF").color(AXIS_TEXT).small()
                };
                if ui.small_button(mtf_txt).clicked() {
                    self.mtf_enabled = !self.mtf_enabled;
                }

                ui.separator();

                // Bar count
                if let Some(c) = self.charts.first() {
                    ui.label(
                        egui::RichText::new(format!("{} bars", c.bars.len()))
                            .color(AXIS_TEXT).small(),
                    );
                }

                if self.cache.is_none() {
                    ui.label(egui::RichText::new("NO CACHE").color(egui::Color32::from_rgb(255,80,80)).small().strong());
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("~: command palette").color(AXIS_TEXT).small());
                });
            });
        });

        // ── tab bar ───────────────────────────────────────────────────────────
        egui::TopBottomPanel::top("tab_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let mut switch_to: Option<usize> = None;
                let mut close_tab: Option<usize> = None;
                for (idx, chart) in self.charts.iter().enumerate() {
                    let active = idx == self.active_tab;
                    let label = format!("{} [{}]", chart.symbol, chart.timeframe.label());
                    let color = if active { ACCENT } else { AXIS_TEXT };
                    let text = egui::RichText::new(&label).color(color).small().strong();
                    if ui.selectable_label(active, text).clicked() {
                        switch_to = Some(idx);
                    }
                    // Close button (only if >1 tab)
                    if self.charts.len() > 1 {
                        if ui.small_button(egui::RichText::new("×").color(AXIS_TEXT).small()).clicked() {
                            close_tab = Some(idx);
                        }
                    }
                    ui.separator();
                }
                // + button to add new tab
                if ui.small_button(egui::RichText::new("+").color(ACCENT).strong()).clicked() {
                    let tf = self.charts.get(self.active_tab).map(|c| c.timeframe).unwrap_or(Timeframe::H4);
                    let mut new_chart = ChartState::new(&self.symbol_input, tf);
                    if let Some(ref cache) = self.cache.clone() {
                        new_chart.load(Arc::as_ref(cache), &mut self.log);
                    }
                    self.charts.push(new_chart);
                    self.active_tab = self.charts.len() - 1;
                }
                // Chart type indicator
                if let Some(c) = self.charts.get(self.active_tab) {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(c.chart_type.label()).color(AXIS_TEXT).small());
                    });
                }

                // Apply deferred actions
                if let Some(idx) = switch_to {
                    self.active_tab = idx;
                }
                if let Some(idx) = close_tab {
                    self.charts.remove(idx);
                    if self.active_tab >= self.charts.len() {
                        self.active_tab = self.charts.len().saturating_sub(1);
                    }
                }
            });
        });

        // ── bottom panel (log / volume) ──────────────────────────────────────
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .min_height(80.0)
            .default_height(120.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.bottom_tab, BottomTab::Log, "Log");
                    ui.selectable_value(&mut self.bottom_tab, BottomTab::Volume, "Volume");
                });
                ui.separator();
                match self.bottom_tab {
                    BottomTab::Log => {
                        egui::ScrollArea::vertical()
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                for entry in &self.log {
                                    ui.label(
                                        egui::RichText::new(format!("{}{}", entry.prefix(), entry.msg))
                                            .color(entry.color())
                                            .font(egui::FontId::monospace(11.0)),
                                    );
                                }
                            });
                    }
                    BottomTab::Volume => {
                        if let Some(chart) = self.charts.first() {
                            let (si, ei) = chart.visible_range();
                            let bars = &chart.bars[si..ei];
                            if !bars.is_empty() {
                                let max_vol = bars.iter().map(|b| b.volume).fold(0.0_f64, f64::max);
                                let avail   = ui.available_size();
                                let (rect, _) = ui.allocate_exact_size(avail, egui::Sense::hover());
                                let painter  = ui.painter_at(rect);
                                painter.rect_filled(rect, 0.0, BG);
                                let bar_w = (rect.width() / bars.len() as f32).max(1.0);
                                for (i, b) in bars.iter().enumerate() {
                                    let x     = rect.left() + i as f32 * bar_w;
                                    let h     = if max_vol > 0.0 { (b.volume / max_vol) as f32 * rect.height() } else { 0.0 };
                                    let color = if b.close >= b.open { UP } else { DOWN };
                                    painter.rect_filled(
                                        egui::Rect::from_min_size(
                                            egui::pos2(x, rect.bottom() - h),
                                            egui::vec2(bar_w.max(1.0) - 0.5, h),
                                        ),
                                        0.0,
                                        color,
                                    );
                                }
                            }
                        }
                    }
                }
            });

        // ── bottom status bar ────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(18.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let n_bars = self.charts.first().map(|c| c.bars.len()).unwrap_or(0);
                    let sym    = self.charts.first().map(|c| c.symbol.as_str()).unwrap_or("—");
                    ui.label(
                        egui::RichText::new(format!(
                            "TyphooN Terminal | {} | {} bars | frame {}",
                            sym, n_bars, self.frame_count
                        ))
                        .color(AXIS_TEXT)
                        .small(),
                    );
                    if let Some(err) = &self.cache_err {
                        ui.label(
                            egui::RichText::new(format!(" | {}", err))
                                .color(egui::Color32::from_rgb(255, 80, 80))
                                .small(),
                        );
                    }
                });
            });

        // ── right panel (positions / orders / risk) ──────────────────────────
        egui::SidePanel::right("right_panel")
            .default_width(280.0)
            .min_width(180.0)
            .show(ctx, |ui| {
                ui.heading("Positions");
                ui.separator();
                // Show DARWIN open positions if available
                let mut has_positions = false;
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = darwin::create_darwin_tables(&conn);
                        if let Ok(positions) = darwin::get_portfolio_open_positions(&conn) {
                            if !positions.is_empty() {
                                has_positions = true;
                                for pos in &positions {
                                    let side_c = if pos.side == "buy" { UP } else { DOWN };
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(&pos.symbol).small().strong());
                                        ui.label(egui::RichText::new(&pos.side).color(side_c).small());
                                        ui.label(egui::RichText::new(format!("{:.2}", pos.total_volume)).small());
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(format!("avg: {}", format_price(pos.avg_price))).color(AXIS_TEXT).small());
                                        ui.label(egui::RichText::new(format!("${:.0}", pos.notional)).color(AXIS_TEXT).small());
                                    });
                                    // Show which DARWINs
                                    let darwins: Vec<String> = pos.darwin_breakdown.iter().map(|(d, _, _)| d.clone()).collect();
                                    ui.label(egui::RichText::new(darwins.join(", ")).color(AXIS_TEXT).small());
                                    ui.separator();
                                }
                            }
                        }
                    }
                }
                if !has_positions {
                    ui.label(egui::RichText::new("No open positions.").color(AXIS_TEXT).small());
                }
                ui.add_space(10.0);

                ui.heading("Orders");
                ui.separator();
                ui.label(
                    egui::RichText::new("Connect broker for live orders.").color(AXIS_TEXT).small(),
                );
                ui.add_space(10.0);

                ui.heading("Risk");
                ui.separator();
                // Pull live DARWIN data if available
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = darwin::create_darwin_tables(&conn);
                        if let Ok(portfolio) = darwin::get_portfolio_summary(&conn) {
                            if !portfolio.accounts.is_empty() {
                                egui::Grid::new("risk_grid").striped(true).num_columns(2).show(ui, |ui| {
                                    ui.label(egui::RichText::new("Accounts").color(AXIS_TEXT).small());
                                    ui.label(egui::RichText::new(format!("{}", portfolio.accounts.len())).small());
                                    ui.end_row();
                                    ui.label(egui::RichText::new("Equity").color(AXIS_TEXT).small());
                                    ui.label(egui::RichText::new(format!("${:.0}", portfolio.total_final_balance)).small());
                                    ui.end_row();
                                    let pnl_c = if portfolio.total_net_pnl >= 0.0 { UP } else { DOWN };
                                    ui.label(egui::RichText::new("Net P&L").color(AXIS_TEXT).small());
                                    ui.label(egui::RichText::new(format!("${:.0}", portfolio.total_net_pnl)).color(pnl_c).small());
                                    ui.end_row();
                                    ui.label(egui::RichText::new("Max DD").color(AXIS_TEXT).small());
                                    ui.label(egui::RichText::new(format!("{:.1}%", portfolio.combined_max_drawdown_pct)).small());
                                    ui.end_row();
                                    ui.label(egui::RichText::new("Deals").color(AXIS_TEXT).small());
                                    ui.label(egui::RichText::new(format!("{}", portfolio.total_deals)).small());
                                    ui.end_row();
                                });
                                // Portfolio VaR
                                if let Ok(daily) = darwin::get_portfolio_daily_returns(&conn) {
                                    if !daily.is_empty() {
                                        let vs = darwin::compute_var(&daily);
                                        ui.add_space(4.0);
                                        egui::Grid::new("risk_var").striped(true).num_columns(2).show(ui, |ui| {
                                            ui.label(egui::RichText::new("VaR 95%").color(AXIS_TEXT).small());
                                            ui.label(egui::RichText::new(format!("${:.0}", vs.var_95)).small());
                                            ui.end_row();
                                            ui.label(egui::RichText::new("Sharpe").color(AXIS_TEXT).small());
                                            ui.label(egui::RichText::new(format!("{:.3}", vs.sharpe)).small());
                                            ui.end_row();
                                        });
                                    }
                                }
                            } else {
                                ui.label(egui::RichText::new("Import DARWIN data").color(AXIS_TEXT).small());
                            }
                        }
                    }
                }
                ui.add_space(10.0);

                ui.heading("DARWIN");
                ui.separator();
                ui.label(egui::RichText::new("VaR corridor: 3.25% – 6.5%").color(AXIS_TEXT).small());
                ui.label(egui::RichText::new("Correlation limit: 0.95 / 45d").color(AXIS_TEXT).small());
                ui.add_space(10.0);

                // ── Watchlist ────────────────────────────────────────────
                ui.heading("Watchlist");
                ui.separator();
                if self.watchlist_symbols.is_empty() {
                    ui.label(egui::RichText::new("No cached symbols.").color(AXIS_TEXT).small());
                } else {
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        let mut load_key: Option<String> = None;
                        for (key, count) in &self.watchlist_symbols {
                            let row = ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(key).color(egui::Color32::WHITE).small().monospace());
                                ui.label(egui::RichText::new(format!("{} bars", count)).color(AXIS_TEXT).small());
                            });
                            if row.response.interact(egui::Sense::click()).clicked() {
                                load_key = Some(key.clone());
                            }
                        }
                        if let Some(key) = load_key {
                            // Load this cache key directly into active chart
                            if let Some(ref cache) = self.cache {
                                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                                    // Set the cache key directly by parsing it
                                    match cache.get_bars_raw(&key) {
                                        Ok(Some(raw)) => {
                                            chart.bars = raw.into_iter().map(|(ts, o, h, l, c, v)| Bar {
                                                ts_ms: ts, open: o, high: h, low: l, close: c, volume: v,
                                            }).collect();
                                            chart.view_offset = chart.bars.len().saturating_sub(1);
                                            chart.symbol = key.clone();
                                            chart.compute_indicators();
                                            self.log.push_back(LogEntry::info(format!("Loaded {} bars from {}", chart.bars.len(), key)));
                                        }
                                        Ok(None) => { self.log.push_back(LogEntry::warn(format!("No data for {}", key))); }
                                        Err(e) => { self.log.push_back(LogEntry::err(format!("Load error: {}", e))); }
                                    }
                                }
                            }
                        }
                    });
                }
            });

        // ── floating windows ─────────────────────────────────────────────────
        self.draw_floating_windows(ctx);

        // ── central panel (chart area) ────────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_rect_before_wrap();

            // Scroll → zoom (Ctrl+scroll = vertical zoom)
            let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
            let ctrl_held = ctx.input(|i| i.modifiers.ctrl);
            if scroll_delta != 0.0 && available.contains(ctx.input(|i| i.pointer.hover_pos().unwrap_or_default())) {
                if ctrl_held {
                    // Vertical zoom (price axis)
                    if let Some(chart) = self.charts.get_mut(self.active_tab) {
                        let factor = if scroll_delta > 0.0 { 1.1 } else { 0.9 };
                        chart.price_zoom = (chart.price_zoom * factor).clamp(0.1, 20.0);
                    }
                } else {
                    // Horizontal zoom (time axis)
                    for chart in &mut self.charts {
                        Self::handle_zoom(chart, scroll_delta);
                    }
                }
            }

            // Double-click → reset zoom/pan
            if ctx.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary)) {
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    chart.price_zoom = 1.0;
                    chart.price_pan = 0.0;
                    chart.visible_bars = 200;
                    chart.view_offset = chart.bars.len().saturating_sub(1);
                }
            }

            // Drag → pan
            let pointer   = ctx.input(|i| i.pointer.clone());
            let drag_delta = ctx.input(|i| i.pointer.delta());
            for chart in &mut self.charts {
                if pointer.primary_pressed() {
                    chart.is_dragging = true;
                    chart.drag_start  = pointer.press_origin();
                    chart.drag_start_offset = chart.view_offset;
                    chart.drag_start_ppan   = chart.price_pan;
                } else if pointer.primary_released() {
                    chart.is_dragging = false;
                    chart.drag_start  = None;
                }
                if chart.is_dragging && (drag_delta.x.abs() > 0.0 || drag_delta.y.abs() > 0.0) {
                    Self::handle_pan_h(chart, -drag_delta.x, available.width());
                    if drag_delta.y.abs() > 0.5 {
                        let range = {
                            let bars = &chart.bars;
                            if bars.is_empty() { 1.0 }
                            else {
                                let (si, ei) = chart.visible_range();
                                let slice = &bars[si..ei];
                                let hi = slice.iter().map(|b| b.high).fold(0.0_f64, f64::max);
                                let lo = slice.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                                hi - lo
                            }
                        };
                        chart.price_pan += drag_delta.y as f64 * range / available.height() as f64;
                    }
                }
            }

            // ── command palette overlay ──────────────────────────────────────
            if self.command_open {
                let palette_commands: Vec<&Command> = COMMANDS
                    .iter()
                    .filter(|c| fuzzy_match(&self.command_input, c.name) || fuzzy_match(&self.command_input, c.desc))
                    .collect();

                let palette_height = (palette_commands.len().clamp(1, 12) as f32) * 24.0 + 50.0;

                egui::Window::new("__palette__")
                    .anchor(egui::Align2::CENTER_TOP, [0.0, 40.0])
                    .fixed_size([620.0, palette_height])
                    .title_bar(false)
                    .frame(egui::Frame::window(&ctx.style()).inner_margin(8.0))
                    .show(ctx, |ui| {
                        let input_resp = ui.add(
                            egui::TextEdit::singleline(&mut self.command_input)
                                .desired_width(600.0)
                                .hint_text("Type a command… (Esc or ~ to close)")
                                .font(egui::TextStyle::Monospace),
                        );
                        input_resp.request_focus();
                        ui.separator();

                        let mut execute: Option<String> = None;
                        egui::ScrollArea::vertical().max_height(palette_height - 50.0).show(ui, |ui| {
                            for cmd in &palette_commands {
                                let row = ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(cmd.name).color(ACCENT).strong().monospace());
                                    ui.label(egui::RichText::new(cmd.desc).color(AXIS_TEXT).small());
                                });
                                if row.response.interact(egui::Sense::click()).clicked() {
                                    execute = Some(cmd.name.to_string());
                                }
                            }
                        });

                        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) && !self.command_input.is_empty() {
                            execute = palette_commands.first().map(|c| c.name.to_string());
                        }
                        if let Some(cmd_name) = execute {
                            self.command_open = false;
                            self.handle_command(&cmd_name, ctx);
                        }
                    });
            }

            // ── chart drawing ────────────────────────────────────────────────
            let crosshair = self.crosshair;
            let flags = self.indicator_flags();
            let show_rsi = self.show_rsi;
            let show_fisher = self.show_fisher;
            let show_macd = self.show_macd;
            let show_volume_pane = self.show_volume_pane;
            let show_stochastic = self.show_stochastic;
            let show_adx = self.show_adx;
            let show_cci = self.show_cci;
            let show_williams_r = self.show_williams_r;
            let show_obv = self.show_obv;
            let show_momentum = self.show_momentum;
            let show_better_volume = self.show_better_volume;
            let sl_price = self.sl_price;
            let tp_price = self.tp_price;

            if self.mtf_enabled {
                let total = self.charts.len().min(4);
                let cols   = self.mtf_cols.max(1).min(total);
                let rows   = (total + cols - 1) / cols;
                let cell_w = available.width()  / cols  as f32;
                let cell_h = available.height() / rows  as f32;

                for (idx, chart) in self.charts.iter_mut().take(total).enumerate() {
                    let col = idx % cols;
                    let row = idx / cols;
                    let cell_rect = egui::Rect::from_min_size(
                        egui::pos2(
                            available.left() + col as f32 * cell_w,
                            available.top()  + row as f32 * cell_h,
                        ),
                        egui::vec2(cell_w - 2.0, cell_h - 2.0),
                    );

                    let painter = ui.painter_at(cell_rect);
                    draw_chart(&painter, chart, cell_rect, crosshair, &flags, show_rsi, show_fisher, show_macd, show_volume_pane, show_stochastic, show_adx, show_cci, show_williams_r, show_obv, show_momentum, show_better_volume, sl_price, tp_price);

                    ui.painter_at(cell_rect).rect_stroke(
                        cell_rect,
                        0.0,
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 40, 60)),
                        egui::StrokeKind::Outside,
                    );
                }
            } else {
                let (rect, resp) = ui.allocate_exact_size(available.size(), egui::Sense::click_and_drag());

                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    let painter = ui.painter_at(rect);
                    draw_chart(&painter, chart, rect, crosshair, &flags, show_rsi, show_fisher, show_macd, show_volume_pane, show_stochastic, show_adx, show_cci, show_williams_r, show_obv, show_momentum, show_better_volume, sl_price, tp_price);

                    // ── drawing mode click handling ──────────────────────
                    if resp.clicked() && self.draw_mode != DrawMode::None {
                        if let Some(pos) = crosshair {
                            // Calculate bar index and price from click position
                            let price_axis_w = 70.0_f32;
                            let chart_rect = egui::Rect::from_min_max(
                                rect.min,
                                egui::pos2(rect.right() - price_axis_w, rect.bottom()),
                            );
                            let (start_idx, end_idx) = chart.visible_range();
                            let vis_bars = &chart.bars[start_idx..end_idx];
                            if !vis_bars.is_empty() && chart_rect.contains(pos) {
                                let bar_w = chart_rect.width() / vis_bars.len() as f32;
                                let rel_idx = ((pos.x - chart_rect.left()) / bar_w) as usize;
                                let abs_idx = start_idx + rel_idx.min(vis_bars.len().saturating_sub(1));

                                // Price from y position
                                let mut price_min = vis_bars.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                                let mut price_max = vis_bars.iter().map(|b| b.high).fold(f64::MIN, f64::max);
                                let padding = (price_max - price_min) * 0.05;
                                price_min -= padding;
                                price_max += padding;
                                let range = price_max - price_min;
                                let centre = (price_max + price_min) * 0.5 + chart.price_pan;
                                let half = range * 0.5 / chart.price_zoom;
                                let pmin = centre - half;
                                let pmax = centre + half;
                                let frac = (pos.y - chart_rect.top()) / chart_rect.height();
                                let price = pmax - frac as f64 * (pmax - pmin);

                                match self.draw_mode {
                                    DrawMode::PlacingHLine => {
                                        chart.drawings.push(Drawing::HLine { price, color: HLINE_COL });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingTrendP1 => {
                                        self.draw_mode = DrawMode::PlacingTrendP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingTrendP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::TrendLine {
                                            p1: (bar1, price1),
                                            p2: (abs_idx, price),
                                            color: TRENDLINE_COL,
                                        });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingFiboP1 => {
                                        self.draw_mode = DrawMode::PlacingFiboP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingFiboP2 { bar1, price1 } => {
                                        let (high, low) = if price1 > price { (price1, price) } else { (price, price1) };
                                        chart.drawings.push(Drawing::FiboRetrace {
                                            high, low, bar_start: bar1, bar_end: abs_idx,
                                        });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingVLine => {
                                        chart.drawings.push(Drawing::VLine { bar_idx: abs_idx, color: egui::Color32::from_rgb(200, 200, 100) });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingRectP1 => {
                                        self.draw_mode = DrawMode::PlacingRectP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingRectP2 { bar1, price1 } => {
                                        chart.drawings.push(Drawing::Rectangle {
                                            p1: (bar1, price1), p2: (abs_idx, price),
                                            color: egui::Color32::from_rgba_premultiplied(100, 150, 255, 40),
                                        });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingRayP1 => {
                                        self.draw_mode = DrawMode::PlacingRayP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingRayP2 { bar1, price1 } => {
                                        let slope = if abs_idx != bar1 { (price - price1) / (abs_idx as f64 - bar1 as f64) } else { 0.0 };
                                        chart.drawings.push(Drawing::Ray {
                                            origin: (bar1, price1), slope,
                                            color: egui::Color32::from_rgb(100, 200, 255),
                                        });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::PlacingChannelP1 => {
                                        self.draw_mode = DrawMode::PlacingChannelP2 { bar1: abs_idx, price1: price };
                                    }
                                    DrawMode::PlacingChannelP2 { bar1, price1 } => {
                                        self.draw_mode = DrawMode::PlacingChannelP3 { bar1, price1, bar2: abs_idx, price2: price };
                                    }
                                    DrawMode::PlacingChannelP3 { bar1, price1, bar2, price2 } => {
                                        let width = price - price1; // offset from first line
                                        chart.drawings.push(Drawing::Channel {
                                            p1: (bar1, price1), p2: (bar2, price2), width,
                                            color: egui::Color32::from_rgb(150, 200, 100),
                                        });
                                        self.draw_mode = DrawMode::None;
                                    }
                                    DrawMode::None => {}
                                }
                            }
                        }
                    }

                    // ── right-click context menu ─────────────────────────
                    resp.context_menu(|ui| {
                        ui.label(egui::RichText::new("Drawing Tools").color(ACCENT).strong());
                        ui.separator();
                        if ui.button("Horizontal Line").clicked() {
                            self.draw_mode = DrawMode::PlacingHLine;
                            ui.close_menu();
                        }
                        if ui.button("Trendline (2 clicks)").clicked() {
                            self.draw_mode = DrawMode::PlacingTrendP1;
                            ui.close_menu();
                        }
                        if ui.button("Fibonacci Retracement").clicked() {
                            self.draw_mode = DrawMode::PlacingFiboP1;
                            ui.close_menu();
                        }
                        if ui.button("Vertical Line").clicked() {
                            self.draw_mode = DrawMode::PlacingVLine;
                            ui.close_menu();
                        }
                        if ui.button("Rectangle (2 clicks)").clicked() {
                            self.draw_mode = DrawMode::PlacingRectP1;
                            ui.close_menu();
                        }
                        if ui.button("Ray (2 clicks)").clicked() {
                            self.draw_mode = DrawMode::PlacingRayP1;
                            ui.close_menu();
                        }
                        if ui.button("Channel (3 clicks)").clicked() {
                            self.draw_mode = DrawMode::PlacingChannelP1;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Remove Last Drawing").clicked() {
                            chart.drawings.pop();
                            ui.close_menu();
                        }
                        if ui.button("Clear All Drawings").clicked() {
                            chart.drawings.clear();
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label(egui::RichText::new("Chart").color(ACCENT).strong());
                        ui.separator();
                        if ui.button("Reset Zoom / Pan").clicked() {
                            chart.price_zoom = 1.0;
                            chart.price_pan = 0.0;
                            chart.visible_bars = 200;
                            chart.view_offset = chart.bars.len().saturating_sub(1);
                            ui.close_menu();
                        }
                        for &ct in &[ChartType::Candle, ChartType::HeikinAshi, ChartType::Line, ChartType::OhlcBars, ChartType::Renko] {
                            let label = if chart.chart_type == ct { format!("● {}", ct.label()) } else { format!("  {}", ct.label()) };
                            if ui.button(label).clicked() {
                                chart.chart_type = ct;
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        ui.label(egui::RichText::new("Windows").color(ACCENT).strong());
                        ui.separator();
                        if ui.button("Indicators…").clicked() { self.show_indicators_panel = true; ui.close_menu(); }
                        if ui.button("Data Window").clicked() { self.show_data_window = true; ui.close_menu(); }
                        if ui.button("Volume Profile").clicked() { self.show_volume_profile = true; ui.close_menu(); }
                        if ui.button("Price Alerts…").clicked() { self.show_alerts = true; ui.close_menu(); }
                        // Copy price at crosshair
                        if let Some(pos) = crosshair {
                            ui.separator();
                            if ui.button("Copy Price at Cursor").clicked() {
                                let frac = (pos.y - rect.top()) / (rect.height() - 80.0);
                                let (si, ei) = chart.visible_range();
                                let vis = &chart.bars[si..ei];
                                if !vis.is_empty() {
                                    let hi = vis.iter().map(|b| b.high).fold(f64::MIN, f64::max);
                                    let lo = vis.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                                    let price = hi - frac as f64 * (hi - lo);
                                    ctx.copy_text(format_price(price));
                                }
                                ui.close_menu();
                            }
                        }
                    });
                }
            }
        });

        // Request continuous repainting for real-time tick updates
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
}
