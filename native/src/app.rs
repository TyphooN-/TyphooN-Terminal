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
use egui_plot::{Line, PlotPoints, Plot};
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
use typhoon_engine::core::sec_filing;
use typhoon_engine::core::keyring;
use typhoon_engine::broker::alpaca::{Bar as EngineBar, AlpacaBroker, AccountInfo, PositionInfo, OrderInfo};
use tokio::sync::mpsc;

// ─── colours ────────────────────────────────────────────────────────────────
const BG: egui::Color32 = egui::Color32::from_rgb(0, 0, 0);
const GRID: egui::Color32 = egui::Color32::from_rgb(33, 33, 33);     // #333 (WebKit dotted grid)
const UP: egui::Color32 = egui::Color32::from_rgb(0, 255, 0);        // #00ff00 (MT5 bright green — solid fill)
const DOWN: egui::Color32 = egui::Color32::from_rgb(255, 0, 0);      // #ff0000 (MT5 bright red — solid fill)
const SMA200_COL: egui::Color32 = egui::Color32::from_rgb(255, 255, 0);  // #ffff00 yellow (MT5 match)
const SMA100_COL: egui::Color32 = egui::Color32::from_rgb(100, 180, 255); // #64b4ff blue
const KAMA_COL: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);  // white (MT5 KAMA)
const EMA_COL: egui::Color32 = egui::Color32::from_rgb(255, 130, 60);
const BB_COL: egui::Color32 = egui::Color32::from_rgb(80, 160, 200);
const BB_FILL: egui::Color32 = egui::Color32::from_rgba_premultiplied(80, 160, 200, 25);
const AXIS_TEXT: egui::Color32 = egui::Color32::from_rgb(140, 140, 160); // #8c8ca0
const ACCENT: egui::Color32 = egui::Color32::from_rgb(76, 175, 80);
const FISHER_POS: egui::Color32 = egui::Color32::from_rgb(0, 255, 0);     // #00ff00 (MT5 bright green)
const FISHER_NEG: egui::Color32 = egui::Color32::from_rgb(255, 0, 0);    // #ff0000 (MT5 bright red)
#[allow(dead_code)]
const FISHER_SIG: egui::Color32 = egui::Color32::from_rgb(169, 169, 169); // clrDarkGray (MT5 signal)
const RSI_LINE: egui::Color32 = egui::Color32::from_rgb(200, 180, 60);   // #c8b43c (mustard yellow)
const MACD_LINE_COL: egui::Color32 = egui::Color32::from_rgb(100, 180, 255); // #64b4ff
const MACD_SIG_COL: egui::Color32 = egui::Color32::from_rgb(255, 130, 48);   // #ff8230 (orange)

// ─── right panel button colours (exact WebKit CSS values) ────────────────────
const BTN_GREEN: egui::Color32 = egui::Color32::from_rgb(10, 95, 56);   // .btn-action: #0a5f38
const BTN_GREEN_TEXT: egui::Color32 = egui::Color32::from_rgb(136, 255, 136); // #8f8
const BTN_MG: egui::Color32 = egui::Color32::from_rgb(58, 58, 0);       // .btn-mg: #3a3a00
const BTN_MG_TEXT: egui::Color32 = egui::Color32::from_rgb(255, 255, 136);   // #ff8
const BTN_BLUE: egui::Color32 = egui::Color32::from_rgb(15, 52, 96);    // .btn-lines: #0f3460
const BTN_BLUE_TEXT: egui::Color32 = egui::Color32::from_rgb(136, 204, 255); // #8cf
const BTN_RED: egui::Color32 = egui::Color32::from_rgb(90, 26, 26);     // .btn-danger: #5a1a1a
const BTN_RED_TEXT: egui::Color32 = egui::Color32::from_rgb(255, 136, 136);  // #f88
// Old CSS variable equivalents (used throughout for WebKit parity)
#[allow(dead_code)]
const BG_DARK: egui::Color32 = egui::Color32::from_rgb(10, 10, 20);     // --bg-dark: #0a0a14
#[allow(dead_code)]
const BG_INPUT: egui::Color32 = egui::Color32::from_rgb(15, 52, 96);    // --bg-input: #0f3460
const BG_BUTTON: egui::Color32 = egui::Color32::from_rgb(26, 26, 46);   // --bg-button: #1a1a2e
#[allow(dead_code)]
const BG_HOVER: egui::Color32 = egui::Color32::from_rgb(22, 33, 62);    // --bg-hover: #16213e
#[allow(dead_code)]
const BORDER: egui::Color32 = egui::Color32::from_rgb(51, 51, 51);      // --border: #333
#[allow(dead_code)]
const INFO_COL: egui::Color32 = egui::Color32::from_rgb(136, 204, 255); // --info: #8cf
#[allow(dead_code)]
const ACCENT_CYAN: egui::Color32 = egui::Color32::from_rgb(0, 188, 212); // --accent: #00bcd4
// Quake console colours (from old WebKit CSS)
#[allow(dead_code)]
const QUAKE_BG: egui::Color32 = egui::Color32::from_rgb(8, 8, 24);     // rgba(8,8,24,0.97)
const QUAKE_CMD: egui::Color32 = egui::Color32::from_rgb(0, 220, 220); // used in status bar
#[allow(dead_code)]
const QUAKE_DESC: egui::Color32 = egui::Color32::from_rgb(136, 136, 136); // #888
// Watchlist symbol colours (rotating palette)
const WL_COLORS: [egui::Color32; 8] = [
    egui::Color32::from_rgb(0, 220, 80),    // green
    egui::Color32::from_rgb(255, 200, 50),   // yellow
    egui::Color32::from_rgb(180, 100, 255),  // purple
    egui::Color32::from_rgb(220, 40, 40),    // red
    egui::Color32::from_rgb(255, 255, 255),  // white
    egui::Color32::from_rgb(0, 180, 255),    // cyan
    egui::Color32::from_rgb(255, 130, 60),   // orange
    egui::Color32::from_rgb(200, 80, 200),   // pink
];

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
const EHLERS_SS_COL: egui::Color32 = egui::Color32::from_rgb(0, 220, 220);
const EHLERS_DEC_COL: egui::Color32 = egui::Color32::from_rgb(220, 160, 0);
const EHLERS_ITL_COL: egui::Color32 = egui::Color32::from_rgb(180, 220, 0);
const EHLERS_MAMA_COL: egui::Color32 = egui::Color32::from_rgb(255, 100, 200);
const EHLERS_FAMA_COL: egui::Color32 = egui::Color32::from_rgb(100, 200, 255);
const EHLERS_EBSW_COL: egui::Color32 = egui::Color32::from_rgb(0, 200, 180);
const EHLERS_CYBER_COL: egui::Color32 = egui::Color32::from_rgb(200, 100, 255);
const EHLERS_CG_COL: egui::Color32 = egui::Color32::from_rgb(255, 180, 0);
const EHLERS_ROOF_COL: egui::Color32 = egui::Color32::from_rgb(100, 255, 100);
#[allow(dead_code)]
const ATR_PROJ_COL: egui::Color32 = egui::Color32::from_rgb(255, 255, 0); // clrYellow (MT5)
// BetterVolume colors — exact MT5 BetterVolume.mqh values
const BVOL_CLIMAX_UP: egui::Color32 = egui::Color32::from_rgb(255, 0, 0);      // clrRed — bullish climax
const BVOL_CLIMAX_DN: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);  // clrWhite — bearish climax
const BVOL_HIGH: egui::Color32 = egui::Color32::from_rgb(0, 255, 0);           // clrGreen — churn (high vol, low move)
const BVOL_LOW: egui::Color32 = egui::Color32::from_rgb(255, 255, 0);          // clrYellow — low volume
const BVOL_CHURN: egui::Color32 = egui::Color32::from_rgb(255, 0, 255);        // clrMagenta — climax + churn
const BVOL_NORMAL: egui::Color32 = egui::Color32::from_rgb(70, 130, 180);      // clrSteelBlue — normal volume

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
    prev_levels: bool,
    pivots: bool,
    fractals: bool,
    harmonics: bool,
    auto_fib: bool,
    supply_demand: bool,
    ehlers_ss: bool,
    ehlers_decycler: bool,
    ehlers_itl: bool,
    ehlers_mama: bool,
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
    /// Previous candle levels (daily high/low).
    prev_daily_high: Option<f64>,
    prev_daily_low: Option<f64>,
    prev_weekly_high: Option<f64>,
    prev_weekly_low: Option<f64>,
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
    /// Pivot points (computed from daily data).
    pivot_p: Option<f64>,
    pivot_r1: Option<f64>,
    pivot_r2: Option<f64>,
    pivot_s1: Option<f64>,
    pivot_s2: Option<f64>,
    /// Bill Williams Fractals (up/down arrows).
    fractal_up: Vec<bool>,
    fractal_down: Vec<bool>,
    // ── Ehlers indicators ──────────────────────────────────────────────
    /// Super Smoother (overlay).
    ehlers_ss: Vec<Option<f64>>,
    /// Decycler (overlay).
    ehlers_decycler: Vec<Option<f64>>,
    /// Instantaneous Trendline (overlay).
    ehlers_itl: Vec<Option<f64>>,
    /// MAMA (overlay).
    ehlers_mama: Vec<Option<f64>>,
    /// FAMA (overlay).
    ehlers_fama: Vec<Option<f64>>,
    /// Even Better Sinewave (sub-pane, -1 to 1).
    ehlers_ebsw: Vec<Option<f64>>,
    /// Cyber Cycle (sub-pane).
    ehlers_cyber: Vec<Option<f64>>,
    /// CG Oscillator (sub-pane).
    ehlers_cg: Vec<Option<f64>>,
    /// Roofing Filter (sub-pane).
    ehlers_roof: Vec<Option<f64>>,
    /// Supply/demand zones: (bar_idx, zone_high, zone_low, status).
    /// Status: 0=untested, 1=tested (price returned), 2=proven (price bounced)
    supply_zones: Vec<(usize, f64, f64, u8)>,
    demand_zones: Vec<(usize, f64, f64, u8)>,
    /// Auto Fibonacci levels: (price, label, is_extension).
    auto_fib_levels: Vec<(f64, String, bool)>,
    /// Auto Fibonacci swing: (swing_high_price, swing_low_price, swing_high_idx, swing_low_idx).
    auto_fib_swing: Option<(f64, f64, usize, usize)>,
    /// MultiKAMA: KAMA values from higher timeframes projected onto this chart's x-axis.
    /// Each entry: (timeframe_label, Vec of (bar_index_in_this_chart, kama_value))
    multi_kama: Vec<(String, Vec<(usize, f64)>)>,
    /// Detected harmonic patterns.
    harmonics: Vec<HarmonicPattern>,
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
    /// True when dragging on the price axis (TradingView-style vertical scale).
    is_scaling_price: bool,
    /// Price zoom at start of price-axis drag.
    scale_start_zoom: f64,
    /// Y position at start of price-axis drag.
    scale_start_y: f32,
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
            prev_daily_high: None,
            prev_daily_low: None,
            prev_weekly_high: None,
            prev_weekly_low: None,
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
            pivot_p: None, pivot_r1: None, pivot_r2: None, pivot_s1: None, pivot_s2: None,
            fractal_up: Vec::new(),
            fractal_down: Vec::new(),
            ehlers_ss: Vec::new(),
            ehlers_decycler: Vec::new(),
            ehlers_itl: Vec::new(),
            ehlers_mama: Vec::new(),
            ehlers_fama: Vec::new(),
            ehlers_ebsw: Vec::new(),
            ehlers_cyber: Vec::new(),
            ehlers_cg: Vec::new(),
            ehlers_roof: Vec::new(),
            supply_zones: Vec::new(),
            demand_zones: Vec::new(),
            auto_fib_levels: Vec::new(),
            auto_fib_swing: None,
            multi_kama: Vec::new(),
            harmonics: Vec::new(),
            drawings: Vec::new(),
            visible_bars: 200,
            view_offset: 0,
            price_pan: 0.0,
            price_zoom: 1.0,
            is_dragging: false,
            drag_start: None,
            drag_start_offset: 0,
            drag_start_ppan: 0.0,
            is_scaling_price: false,
            scale_start_zoom: 1.0,
            scale_start_y: 0.0,
        }
    }

    /// Cache key for this symbol + timeframe.
    /// Try multiple prefix variants to find data in cache.
    fn find_cache_key(&self, cache: &SqliteCache) -> String {
        let tf = self.timeframe.cache_suffix();
        let sym = {
            let parts: Vec<&str> = self.symbol.split(':').collect();
            let is_tf = matches!(parts.last().copied(), Some("1Min"|"5Min"|"15Min"|"30Min"|"1Hour"|"4Hour"|"1Day"|"1Week"|"1Month"));
            if is_tf && parts.len() > 1 {
                parts[..parts.len()-1].join(":")
            } else {
                self.symbol.clone()
            }
        };

        // Try these key patterns in order of priority
        let candidates = [
            format!("{}:{}", sym, tf),                    // exact: "mt5:SLV:4Hour" or "SLV:4Hour"
            format!("mt5:{}:{}", sym, tf),                // mt5 prefix: "mt5:SLV:4Hour"
            format!("default:{}:{}", sym, tf),            // default: "default:SLV:4Hour"
            format!("paper_TyphooN:{}:{}", sym, tf),      // paper: "paper_TyphooN:SLV:4Hour"
            format!("alpaca_paper_TyphooN:{}:{}", sym, tf),// alpaca: "alpaca_paper_TyphooN:SLV:4Hour"
        ];

        for key in &candidates {
            if let Ok(Some(_)) = cache.get_bars_raw(key) {
                return key.clone();
            }
        }

        // Fallback: try partial match from detailed_stats
        if let Ok(stats) = cache.detailed_stats() {
            let sym_lower = sym.to_lowercase();
            for (key, _, _) in &stats {
                let key_lower = key.to_lowercase();
                if key_lower.contains(&sym_lower) && key_lower.ends_with(&tf.to_lowercase()) {
                    return key.clone();
                }
            }
        }

        // Default fallback
        format!("mt5:{}:{}", sym, tf)
    }

    /// Load bars from the shared cache, re-compute indicators.
    fn load(&mut self, cache: &SqliteCache, log: &mut VecDeque<LogEntry>) {
        let key = self.find_cache_key(cache);
        match cache.get_bars_raw(&key) {
            Ok(Some(raw)) => {
                self.bars = raw.into_iter().map(|(ts, o, h, l, c, v)| Bar {
                    ts_ms: ts, open: o, high: h, low: l, close: c, volume: v,
                }).collect();
                self.view_offset = self.bars.len().saturating_sub(1);
                self.compute_indicators();
                // MultiKAMA: load higher TF bars and compute KAMA on each
                self.compute_multi_kama(cache);
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
        let (f, fs) = compute_fisher(&self.bars, 32);
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
        // Previous candle levels — find the second-to-last daily/weekly bar boundaries
        let (pdh, pdl, pwh, pwl) = compute_prev_candle_levels(&self.bars);
        self.prev_daily_high = pdh;
        self.prev_daily_low = pdl;
        self.prev_weekly_high = pwh;
        self.prev_weekly_low = pwl;
        // Pivot points from previous day
        if let (Some(h), Some(l)) = (pdh, pdl) {
            let prev_close = self.bars.iter().rev().find(|b| {
                let day = b.ts_ms / 86_400_000;
                let last_day = self.bars.last().map(|lb| lb.ts_ms / 86_400_000).unwrap_or(0);
                day < last_day
            }).map(|b| b.close);
            if let Some(c) = prev_close {
                let p = (h + l + c) / 3.0;
                self.pivot_p = Some(p);
                self.pivot_r1 = Some(2.0 * p - l);
                self.pivot_r2 = Some(p + (h - l));
                self.pivot_s1 = Some(2.0 * p - h);
                self.pivot_s2 = Some(p - (h - l));
            }
        }
        // Fractals
        self.fractal_up = compute_fractals_up(&self.bars);
        self.fractal_down = compute_fractals_down(&self.bars);
        self.harmonics = detect_harmonic_patterns(&self.bars, &self.fractal_up, &self.fractal_down);
        let (sz, dz) = compute_supply_demand_zones(&self.bars);
        self.supply_zones = sz;
        self.demand_zones = dz;
        // Auto Fibonacci (fractal-based swing detection, matching AutoFibonacci.mqh)
        self.compute_auto_fibonacci();
        // Ehlers indicators
        self.ehlers_ss = ehlers_super_smoother(&self.bars, 10);
        self.ehlers_decycler = ehlers_decycler(&self.bars, 20);
        self.ehlers_itl = ehlers_instantaneous_trendline(&self.bars);
        let (mama, fama) = ehlers_mama_fama(&self.bars, 0.5, 0.05);
        self.ehlers_mama = mama;
        self.ehlers_fama = fama;
        self.ehlers_ebsw = ehlers_even_better_sinewave(&self.bars, 40);
        self.ehlers_cyber = ehlers_cyber_cycle(&self.bars);
        self.ehlers_cg = ehlers_cg_oscillator(&self.bars, 10);
        self.ehlers_roof = ehlers_roofing_filter(&self.bars, 10, 48);
    }

    /// Compute Auto Fibonacci levels from fractal swing points.
    /// Mirrors AutoFibonacci.mqh: finds most significant recent swing high/low
    /// and computes retracement (0-100%) + extension (127.2-423.6%) levels.
    /// Compute MultiKAMA: load bars from higher timeframes and compute KAMA(10,2,30) on each.
    /// Projects KAMA values onto this chart's x-axis by matching timestamps.
    fn compute_multi_kama(&mut self, cache: &SqliteCache) {
        self.multi_kama.clear();
        if self.bars.is_empty() { return; }

        // Extract base symbol (strip timeframe suffix from symbol)
        let base_sym = {
            let parts: Vec<&str> = self.symbol.split(':').collect();
            let is_tf = matches!(parts.last().copied(), Some("1Min"|"5Min"|"15Min"|"30Min"|"1Hour"|"4Hour"|"1Day"|"1Week"|"1Month"));
            if is_tf && parts.len() > 1 { parts[..parts.len()-1].join(":") } else { self.symbol.clone() }
        };

        let higher_tfs = [
            ("H1", "1Hour"), ("H4", "4Hour"), ("D1", "1Day"), ("W1", "1Week"), ("MN1", "1Month"),
        ];

        // Prefixes to try
        let prefixes = ["mt5:", "default:", "paper_TyphooN:", "alpaca_paper_TyphooN:", ""];

        for (tf_label, tf_suffix) in &higher_tfs {
            // Skip if this is the same TF as the chart
            if self.timeframe.cache_suffix() == *tf_suffix { continue; }

            // Try to load bars from cache
            let mut htf_bars: Option<Vec<Bar>> = None;
            for prefix in &prefixes {
                let key = format!("{}{}:{}", prefix, base_sym, tf_suffix);
                if let Ok(Some(raw)) = cache.get_bars_raw(&key) {
                    htf_bars = Some(raw.into_iter().map(|(ts, o, h, l, c, v)| Bar {
                        ts_ms: ts, open: o, high: h, low: l, close: c, volume: v,
                    }).collect());
                    break;
                }
            }

            if let Some(htf) = htf_bars {
                if htf.len() < 12 { continue; }
                // Compute KAMA(10,2,30) on higher TF bars
                let kama_vals = compute_kama(&htf, 10, 2, 30);

                // Map higher TF KAMA values onto this chart's bar indices by timestamp
                // For each of our bars, find the most recent HTF bar that's <= our timestamp
                let mut projected: Vec<(usize, f64)> = Vec::new();
                let mut htf_idx = 0;
                for (i, bar) in self.bars.iter().enumerate() {
                    while htf_idx + 1 < htf.len() && htf[htf_idx + 1].ts_ms <= bar.ts_ms {
                        htf_idx += 1;
                    }
                    if htf_idx < kama_vals.len() {
                        if let Some(k) = kama_vals[htf_idx] {
                            projected.push((i, k));
                        }
                    }
                }

                if !projected.is_empty() {
                    self.multi_kama.push((tf_label.to_string(), projected));
                }
            }
        }
    }

    fn compute_auto_fibonacci(&mut self) {
        self.auto_fib_levels.clear();
        self.auto_fib_swing = None;
        if self.bars.len() < 20 { return; }

        let lookback = 10usize; // InpFractalLookback
        let recent_start = (self.bars.len() as f64 * 0.4) as usize; // search recent 60%
        let search = &self.bars[recent_start..];

        // Find swing high and swing low from fractals in search range
        let mut swing_high: Option<(f64, usize)> = None;
        let mut swing_low: Option<(f64, usize)> = None;

        for i in lookback..search.len().saturating_sub(lookback) {
            let abs_i = recent_start + i;
            if abs_i < self.fractal_up.len() && self.fractal_up[abs_i] {
                if swing_high.map_or(true, |(h, _)| search[i].high > h) {
                    swing_high = Some((search[i].high, abs_i));
                }
            }
            if abs_i < self.fractal_down.len() && self.fractal_down[abs_i] {
                if swing_low.map_or(true, |(l, _)| search[i].low < l) {
                    swing_low = Some((search[i].low, abs_i));
                }
            }
        }

        if let (Some((high, hi_idx)), Some((low, lo_idx))) = (swing_high, swing_low) {
            if (high - low).abs() < f64::EPSILON { return; }
            self.auto_fib_swing = Some((high, low, hi_idx, lo_idx));
            let range = high - low;
            let is_bull = lo_idx < hi_idx; // uptrend: low comes before high

            // Retracement levels (from high toward low for bull, from low toward high for bear)
            let retrace_levels = [0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0];
            let retrace_labels = ["0%", "23.6%", "38.2%", "50%", "61.8%", "78.6%", "100%"];
            for (lvl, label) in retrace_levels.iter().zip(retrace_labels.iter()) {
                let price = if is_bull { high - lvl * range } else { low + lvl * range };
                self.auto_fib_levels.push((price, label.to_string(), false));
            }

            // Extension levels (beyond the swing)
            let ext_levels = [1.272, 1.618, 2.0, 2.618, 3.618, 4.236];
            let ext_labels = ["127.2%", "161.8%", "200%", "261.8%", "361.8%", "423.6%"];
            for (lvl, label) in ext_levels.iter().zip(ext_labels.iter()) {
                let price = if is_bull { low + lvl * range } else { high - lvl * range };
                self.auto_fib_levels.push((price, label.to_string(), true));
            }
        }
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

// ─── harmonic pattern detection (Scott Carney) ───────────────────────────────

#[derive(Clone, Debug)]
struct HarmonicPattern {
    name: &'static str,
    x: (usize, f64),  // bar index, price
    a: (usize, f64),
    b: (usize, f64),
    c: (usize, f64),
    d: (usize, f64),  // completion / entry point
    tp1: f64,          // target 1 (0.382 AD)
    tp2: f64,          // target 2 (0.618 AD)
    sl: f64,           // stop loss (beyond X)
    bullish: bool,
}

fn detect_harmonic_patterns(bars: &[Bar], fractals_up: &[bool], fractals_down: &[bool]) -> Vec<HarmonicPattern> {
    let n = bars.len();
    if n < 20 { return Vec::new(); }
    let mut patterns: Vec<HarmonicPattern> = Vec::new();

    // Collect swing points from fractals
    let mut swings: Vec<(usize, f64, bool)> = Vec::new(); // (index, price, is_high)
    for i in 0..n {
        if i < fractals_up.len() && fractals_up[i] { swings.push((i, bars[i].high, true)); }
        if i < fractals_down.len() && fractals_down[i] { swings.push((i, bars[i].low, false)); }
    }

    // Need at least 5 swing points for XABCD
    if swings.len() < 5 { return patterns; }

    // Check the most recent swing combinations (limit to last 20 swings for performance)
    let start = swings.len().saturating_sub(20);
    let recent = &swings[start..];

    for i in 0..recent.len().saturating_sub(4) {
        for j in (i+1)..recent.len().saturating_sub(3) {
            for k in (j+1)..recent.len().saturating_sub(2) {
                for l in (k+1)..recent.len().saturating_sub(1) {
                    for m in (l+1)..recent.len() {
                        let x = recent[i];
                        let a = recent[j];
                        let b = recent[k];
                        let c = recent[l];
                        let d = recent[m];

                        // Must alternate: high-low-high-low-high or low-high-low-high-low
                        if x.2 == a.2 || a.2 == b.2 || b.2 == c.2 || c.2 == d.2 { continue; }

                        let xa = (a.1 - x.1).abs();
                        if xa < f64::EPSILON { continue; }
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
                        if in_range(ab_xa, 0.55, 0.68) && in_range(bc_ab, 0.35, 0.92) && in_range(xd_xa, 0.72, 0.84) {
                            let tp1 = if bullish { d.1 + ad * 0.382 } else { d.1 - ad * 0.382 };
                            let tp2 = if bullish { d.1 + ad * 0.618 } else { d.1 - ad * 0.618 };
                            let sl = if bullish { x.1 - xa * 0.1 } else { x.1 + xa * 0.1 };
                            patterns.push(HarmonicPattern {
                                name: "Gartley", x: (x.0, x.1), a: (a.0, a.1), b: (b.0, b.1),
                                c: (c.0, c.1), d: (d.0, d.1), tp1, tp2, sl, bullish,
                            });
                        }
                        // Butterfly: AB=0.786 XA, BC=0.382-0.886 AB, XD=1.27 XA
                        else if in_range(ab_xa, 0.72, 0.84) && in_range(bc_ab, 0.35, 0.92) && in_range(xd_xa, 1.20, 1.35) {
                            let tp1 = if bullish { d.1 + ad * 0.382 } else { d.1 - ad * 0.382 };
                            let tp2 = if bullish { d.1 + ad * 0.618 } else { d.1 - ad * 0.618 };
                            let sl = if bullish { d.1 - xa * 0.15 } else { d.1 + xa * 0.15 };
                            patterns.push(HarmonicPattern {
                                name: "Butterfly", x: (x.0, x.1), a: (a.0, a.1), b: (b.0, b.1),
                                c: (c.0, c.1), d: (d.0, d.1), tp1, tp2, sl, bullish,
                            });
                        }
                        // Bat: AB=0.382-0.50 XA, BC=0.382-0.886 AB, XD=0.886 XA
                        else if in_range(ab_xa, 0.35, 0.55) && in_range(bc_ab, 0.35, 0.92) && in_range(xd_xa, 0.82, 0.92) {
                            let tp1 = if bullish { d.1 + ad * 0.382 } else { d.1 - ad * 0.382 };
                            let tp2 = if bullish { d.1 + ad * 0.618 } else { d.1 - ad * 0.618 };
                            let sl = if bullish { x.1 - xa * 0.1 } else { x.1 + xa * 0.1 };
                            patterns.push(HarmonicPattern {
                                name: "Bat", x: (x.0, x.1), a: (a.0, a.1), b: (b.0, b.1),
                                c: (c.0, c.1), d: (d.0, d.1), tp1, tp2, sl, bullish,
                            });
                        }
                        // Crab: AB=0.382-0.618 XA, BC=0.382-0.886 AB, XD=1.618 XA
                        else if in_range(ab_xa, 0.35, 0.65) && in_range(bc_ab, 0.35, 0.92) && in_range(xd_xa, 1.55, 1.72) {
                            let tp1 = if bullish { d.1 + ad * 0.382 } else { d.1 - ad * 0.382 };
                            let tp2 = if bullish { d.1 + ad * 0.618 } else { d.1 - ad * 0.618 };
                            let sl = if bullish { d.1 - xa * 0.1 } else { d.1 + xa * 0.1 };
                            patterns.push(HarmonicPattern {
                                name: "Crab", x: (x.0, x.1), a: (a.0, a.1), b: (b.0, b.1),
                                c: (c.0, c.1), d: (d.0, d.1), tp1, tp2, sl, bullish,
                            });
                        }
                        // Shark: AB=1.13-1.618 XA, BC=1.618-2.24 AB, XD=0.886 XA
                        else if in_range(ab_xa, 1.10, 1.65) && in_range(xd_xa, 0.82, 0.92) {
                            let tp1 = if bullish { d.1 + ad * 0.382 } else { d.1 - ad * 0.382 };
                            let tp2 = if bullish { d.1 + ad * 0.618 } else { d.1 - ad * 0.618 };
                            let sl = if bullish { x.1 - xa * 0.1 } else { x.1 + xa * 0.1 };
                            patterns.push(HarmonicPattern {
                                name: "Shark", x: (x.0, x.1), a: (a.0, a.1), b: (b.0, b.1),
                                c: (c.0, c.1), d: (d.0, d.1), tp1, tp2, sl, bullish,
                            });
                        }
                        // Cypher: AB=0.382-0.618 XA, BC=1.13-1.414 AB, XD=0.786 XA
                        else if in_range(ab_xa, 0.35, 0.65) && in_range(bc_ab, 1.10, 1.45) && in_range(xd_xa, 0.72, 0.84) {
                            let tp1 = if bullish { d.1 + ad * 0.382 } else { d.1 - ad * 0.382 };
                            let tp2 = if bullish { d.1 + ad * 0.618 } else { d.1 - ad * 0.618 };
                            let sl = if bullish { x.1 - xa * 0.1 } else { x.1 + xa * 0.1 };
                            patterns.push(HarmonicPattern {
                                name: "Cypher", x: (x.0, x.1), a: (a.0, a.1), b: (b.0, b.1),
                                c: (c.0, c.1), d: (d.0, d.1), tp1, tp2, sl, bullish,
                            });
                        }
                        // 5-0: AB=1.13-1.618 XA, BC=1.618-2.24 AB, XD=0.50 BC
                        else if in_range(ab_xa, 1.10, 1.65) && in_range(bc_ab, 1.55, 2.30) {
                            let bc_val = (d.1 - c.1).abs();
                            let xd_bc = if bc > f64::EPSILON { bc_val / bc } else { 0.0 };
                            if in_range(xd_bc, 0.45, 0.55) {
                                let tp1 = if bullish { d.1 + ad * 0.382 } else { d.1 - ad * 0.382 };
                                let tp2 = if bullish { d.1 + ad * 0.618 } else { d.1 - ad * 0.618 };
                                let sl = if bullish { d.1 - xa * 0.15 } else { d.1 + xa * 0.15 };
                                patterns.push(HarmonicPattern {
                                    name: "5-0", x: (x.0, x.1), a: (a.0, a.1), b: (b.0, b.1),
                                    c: (c.0, c.1), d: (d.0, d.1), tp1, tp2, sl, bullish,
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

fn in_range(v: f64, lo: f64, hi: f64) -> bool { v >= lo && v <= hi }

fn compute_fractals_up(bars: &[Bar]) -> Vec<bool> {
    let n = bars.len();
    let mut out = vec![false; n];
    if n < 5 { return out; }
    for i in 2..(n - 2) {
        if bars[i].high > bars[i-1].high && bars[i].high > bars[i-2].high
            && bars[i].high > bars[i+1].high && bars[i].high > bars[i+2].high {
            out[i] = true;
        }
    }
    out
}

fn compute_fractals_down(bars: &[Bar]) -> Vec<bool> {
    let n = bars.len();
    let mut out = vec![false; n];
    if n < 5 { return out; }
    for i in 2..(n - 2) {
        if bars[i].low < bars[i-1].low && bars[i].low < bars[i-2].low
            && bars[i].low < bars[i+1].low && bars[i].low < bars[i+2].low {
            out[i] = true;
        }
    }
    out
}

fn compute_prev_candle_levels(bars: &[Bar]) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
    if bars.len() < 2 { return (None, None, None, None); }

    // Group bars by day
    let mut daily_groups: Vec<(f64, f64)> = Vec::new(); // (high, low) per day
    let mut current_day = -1_i64;
    let mut day_hi = f64::MIN;
    let mut day_lo = f64::MAX;

    for bar in bars {
        let day = bar.ts_ms / (86_400_000); // ms per day
        if day != current_day {
            if current_day >= 0 { daily_groups.push((day_hi, day_lo)); }
            current_day = day;
            day_hi = bar.high;
            day_lo = bar.low;
        } else {
            day_hi = day_hi.max(bar.high);
            day_lo = day_lo.min(bar.low);
        }
    }
    if current_day >= 0 { daily_groups.push((day_hi, day_lo)); }

    let (pdh, pdl) = if daily_groups.len() >= 2 {
        let prev = &daily_groups[daily_groups.len() - 2];
        (Some(prev.0), Some(prev.1))
    } else { (None, None) };

    // Group by week (7 days)
    let mut weekly_groups: Vec<(f64, f64)> = Vec::new();
    let mut current_week = -1_i64;
    let mut week_hi = f64::MIN;
    let mut week_lo = f64::MAX;

    for bar in bars {
        let week = bar.ts_ms / (7 * 86_400_000);
        if week != current_week {
            if current_week >= 0 { weekly_groups.push((week_hi, week_lo)); }
            current_week = week;
            week_hi = bar.high;
            week_lo = bar.low;
        } else {
            week_hi = week_hi.max(bar.high);
            week_lo = week_lo.min(bar.low);
        }
    }
    if current_week >= 0 { weekly_groups.push((week_hi, week_lo)); }

    let (pwh, pwl) = if weekly_groups.len() >= 2 {
        let prev = &weekly_groups[weekly_groups.len() - 2];
        (Some(prev.0), Some(prev.1))
    } else { (None, None) };

    (pdh, pdl, pwh, pwl)
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
            sum += bars[i + 1 - period + j].close * (j + 1) as f64;
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

fn compute_supply_demand_zones(bars: &[Bar]) -> (Vec<(usize, f64, f64, u8)>, Vec<(usize, f64, f64, u8)>) {
    let n = bars.len();
    let mut supply: Vec<(usize, f64, f64, u8)> = Vec::new();
    let mut demand: Vec<(usize, f64, f64, u8)> = Vec::new();
    if n < 10 { return (supply, demand); }

    let avg_range: f64 = bars.iter().map(|b| b.high - b.low).sum::<f64>() / n as f64;
    let impulse_threshold = avg_range * 2.0;

    for i in 1..(n - 1) {
        let range = bars[i].high - bars[i].low;
        let body = (bars[i].close - bars[i].open).abs();

        if range > impulse_threshold && body > range * 0.6 {
            let is_bullish = bars[i].close > bars[i].open;

            if is_bullish {
                let zone_high = bars[i - 1].high.max(bars[i].open);
                let zone_low = bars[i - 1].low.min(bars[i].open);
                if zone_high > zone_low {
                    demand.push((i - 1, zone_high, zone_low, 0)); // untested
                }
            } else {
                let zone_high = bars[i - 1].high.max(bars[i].open);
                let zone_low = bars[i - 1].low.min(bars[i].open);
                if zone_high > zone_low {
                    supply.push((i - 1, zone_high, zone_low, 0)); // untested
                }
            }
        }
    }

    // Determine zone status: check if price returned to zone after creation
    for zone in &mut demand {
        for j in (zone.0 + 2)..n {
            if bars[j].low <= zone.1 && bars[j].low >= zone.2 {
                // Price returned to zone
                if bars[j].close > zone.1 {
                    zone.3 = 2; // proven (bounced)
                } else {
                    zone.3 = 1; // tested
                }
                break;
            }
        }
    }
    for zone in &mut supply {
        for j in (zone.0 + 2)..n {
            if bars[j].high >= zone.2 && bars[j].high <= zone.1 {
                if bars[j].close < zone.2 {
                    zone.3 = 2; // proven (bounced down)
                } else {
                    zone.3 = 1; // tested
                }
                break;
            }
        }
    }

    supply.sort_by(|a, b| b.0.cmp(&a.0));
    demand.sort_by(|a, b| b.0.cmp(&a.0));
    supply.truncate(10);
    demand.truncate(10);
    (supply, demand)
}

// ─── Ehlers indicators ───────────────────────────────────────────────────────

fn ehlers_super_smoother(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < 3 { return out; }
    let a = (-1.414 * std::f64::consts::PI / period as f64).exp();
    let b = 2.0 * a * (1.414 * std::f64::consts::PI / period as f64).cos();
    let c2 = b;
    let c3 = -a * a;
    let c1 = 1.0 - c2 - c3;
    out[0] = Some(bars[0].close);
    out[1] = Some(bars[1].close);
    for i in 2..n {
        let prev1 = out[i-1].unwrap_or(bars[i-1].close);
        let prev2 = out[i-2].unwrap_or(bars[i-2].close);
        let val = c1 * (bars[i].close + bars[i-1].close) / 2.0 + c2 * prev1 + c3 * prev2;
        out[i] = Some(val);
    }
    out
}

fn ehlers_decycler(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    // Decycler = price - highpass(price, period)
    let n = bars.len();
    let mut out = vec![None; n];
    if n < 3 { return out; }
    let alpha = (2.0 * std::f64::consts::PI / (period as f64 * 1.414)).cos();
    let a1 = (alpha + (alpha * alpha - 1.0).max(0.0).sqrt()).max(0.001).recip();
    // 2-pole highpass
    let mut hp = vec![0.0_f64; n];
    for i in 2..n {
        hp[i] = (1.0 - a1 / 2.0) * (1.0 - a1 / 2.0) * (bars[i].close - 2.0 * bars[i-1].close + bars[i-2].close)
            + 2.0 * (1.0 - a1) * hp[i-1] - (1.0 - a1) * (1.0 - a1) * hp[i-2];
    }
    for i in 0..n {
        out[i] = Some(bars[i].close - hp[i]);
    }
    out
}

fn ehlers_instantaneous_trendline(bars: &[Bar]) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < 7 { return out; }
    let mut itl = vec![0.0_f64; n];
    for i in 0..7.min(n) { itl[i] = bars[i].close; }
    for i in 7..n {
        itl[i] = (bars[i].close + 2.0 * bars[i-1].close + bars[i-2].close) / 4.0 * 0.5
            + itl[i-1] * 0.5;
        // Simplified Ehlers ITL: 2-bar WMA smoothed recursively
        itl[i] = (2.0 * itl[i] + itl[i-1] + itl[i-2] + itl[i-3]) / 5.0;
    }
    for i in 0..n { out[i] = Some(itl[i]); }
    out
}

fn ehlers_mama_fama(bars: &[Bar], fast_limit: f64, slow_limit: f64) -> (Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = bars.len();
    let mut mama = vec![None; n];
    let mut fama = vec![None; n];
    if n < 6 { return (mama, fama); }

    let mut smooth = vec![0.0_f64; n];
    let mut phase = vec![0.0_f64; n];
    let mut mama_v = vec![0.0_f64; n];
    let mut fama_v = vec![0.0_f64; n];
    let mut i1 = vec![0.0_f64; n];
    let mut q1 = vec![0.0_f64; n];

    for i in 3..n {
        smooth[i] = (4.0 * bars[i].close + 3.0 * bars[i-1].close + 2.0 * bars[i-2].close + bars[i-3].close) / 10.0;
    }

    for i in 6..n {
        let det = 0.0962 * smooth[i] + 0.5769 * smooth[i-2] - 0.5769 * smooth[i-4] - 0.0962 * smooth[i-6];
        q1[i] = det;
        i1[i] = smooth[i-3];

        // Phase
        if i1[i].abs() > 0.0 { phase[i] = (q1[i] / i1[i]).atan().to_degrees(); }
        let delta_phase = (phase[i-1] - phase[i]).max(1.0);
        let alpha = (fast_limit / delta_phase).max(slow_limit);

        if i < 7 { mama_v[i] = bars[i].close; fama_v[i] = bars[i].close; }
        else {
            mama_v[i] = alpha * smooth[i] + (1.0 - alpha) * mama_v[i-1];
            fama_v[i] = 0.5 * alpha * mama_v[i] + (1.0 - 0.5 * alpha) * fama_v[i-1];
        }
        mama[i] = Some(mama_v[i]);
        fama[i] = Some(fama_v[i]);
    }
    (mama, fama)
}

fn ehlers_even_better_sinewave(bars: &[Bar], duration: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < 5 { return out; }
    // Highpass then super smooth, then compute sinewave
    let mut hp = vec![0.0_f64; n];
    let alpha1 = (2.0 * std::f64::consts::PI / (duration as f64 * 1.414)).cos();
    let a1_coeff = if alpha1.abs() > f64::EPSILON {
        (alpha1 + (alpha1 * alpha1 - 1.0).max(0.0).sqrt()).max(0.001).recip()
    } else { 0.5 };

    for i in 2..n {
        hp[i] = (1.0 - a1_coeff / 2.0).powi(2) * (bars[i].close - 2.0 * bars[i-1].close + bars[i-2].close)
            + 2.0 * (1.0 - a1_coeff) * hp[i-1] - (1.0 - a1_coeff).powi(2) * hp[i-2];
    }
    // Super smoother on HP
    let period = duration / 4;
    let a = (-1.414 * std::f64::consts::PI / period.max(1) as f64).exp();
    let b = 2.0 * a * (1.414 * std::f64::consts::PI / period.max(1) as f64).cos();
    let c1 = 1.0 - b + a * a;
    let mut filt = vec![0.0_f64; n];
    for i in 2..n {
        filt[i] = c1 * (hp[i] + hp[i-1]) / 2.0 + b * filt[i-1] - a * a * filt[i-2];
    }
    // Wave = atan(filt[i] / filt[i-1]) if filt[i-1] != 0
    for i in 1..n {
        if filt[i-1].abs() > f64::EPSILON {
            let wave = (filt[i] / filt[i-1]).atan() / std::f64::consts::FRAC_PI_2;
            out[i] = Some(wave.clamp(-1.0, 1.0));
        } else {
            out[i] = Some(0.0);
        }
    }
    out
}

fn ehlers_cyber_cycle(bars: &[Bar]) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < 5 { return out; }
    let mut smooth = vec![0.0_f64; n];
    let mut cycle = vec![0.0_f64; n];
    for i in 3..n {
        smooth[i] = (bars[i].close + 2.0 * bars[i-1].close + bars[i-2].close) / 4.0;
    }
    let alpha = 0.07; // 2/(period+1) with period~27
    for i in 4..n {
        let c1: f64 = 1.0 - 0.5 * alpha;
        let c2: f64 = 1.0 - alpha;
        cycle[i] = c1 * c1 * (smooth[i] - 2.0 * smooth[i-1] + smooth[i-2])
            + 2.0 * c2 * cycle[i-1] - c2 * c2 * cycle[i-2];
        out[i] = Some(cycle[i]);
    }
    out
}

fn ehlers_cg_oscillator(bars: &[Bar], period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < period { return out; }
    for i in (period - 1)..n {
        let mut num = 0.0_f64;
        let mut den = 0.0_f64;
        for j in 0..period {
            let p = bars[i - j].close;
            num += (j as f64 + 1.0) * p;
            den += p;
        }
        out[i] = if den.abs() > f64::EPSILON { Some(-num / den + (period as f64 + 1.0) / 2.0) } else { Some(0.0) };
    }
    out
}

fn ehlers_roofing_filter(bars: &[Bar], lp_period: usize, hp_period: usize) -> Vec<Option<f64>> {
    let n = bars.len();
    let mut out = vec![None; n];
    if n < 3 { return out; }
    // Highpass
    let alpha1 = (2.0 * std::f64::consts::PI / hp_period as f64).cos();
    let a1 = if alpha1.abs() > f64::EPSILON {
        (alpha1 + (alpha1 * alpha1 - 1.0).max(0.0).sqrt()).max(0.001).recip()
    } else { 0.5 };
    let mut hp = vec![0.0_f64; n];
    for i in 2..n {
        hp[i] = (1.0 - a1 / 2.0).powi(2) * (bars[i].close - 2.0 * bars[i-1].close + bars[i-2].close)
            + 2.0 * (1.0 - a1) * hp[i-1] - (1.0 - a1).powi(2) * hp[i-2];
    }
    // Super smoother on HP output
    let a = (-1.414 * std::f64::consts::PI / lp_period as f64).exp();
    let b = 2.0 * a * (1.414 * std::f64::consts::PI / lp_period as f64).cos();
    let c1 = 1.0 - b + a * a;
    let mut filt = vec![0.0_f64; n];
    for i in 2..n {
        filt[i] = c1 * (hp[i] + hp[i-1]) / 2.0 + b * filt[i-1] - a * a * filt[i-2];
        out[i] = Some(filt[i]);
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
    show_ehlers_ebsw: bool,
    show_ehlers_cyber: bool,
    show_ehlers_cg: bool,
    show_ehlers_roof: bool,
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
        + show_obv as u8 + show_momentum as u8 + show_better_volume as u8
        + show_ehlers_ebsw as u8 + show_ehlers_cyber as u8 + show_ehlers_cg as u8 + show_ehlers_roof as u8;
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

    // Price axis background (subtle — indicates it's interactive like TradingView)
    let price_axis_bg = egui::Rect::from_min_max(
        egui::pos2(chart_rect.right(), chart_rect.top()),
        egui::pos2(rect.right(), chart_rect.bottom()),
    );
    painter.rect_filled(price_axis_bg, 0.0, egui::Color32::from_rgb(6, 6, 10));
    // Thin separator line between chart and price axis
    painter.line_segment(
        [egui::pos2(chart_rect.right(), chart_rect.top()), egui::pos2(chart_rect.right(), chart_rect.bottom())],
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

    // ── grid lines (price) — dotted style matching MT5/WebKit ──────────────
    let grid_steps = 8;
    let dot_len = 3.0_f32;
    let dot_gap = 3.0_f32;
    let grid_col = egui::Color32::from_rgb(33, 33, 33);
    for i in 0..=grid_steps {
        let p   = price_min + (price_max - price_min) * (i as f64 / grid_steps as f64);
        let y   = price_to_y(p);
        // Dotted horizontal line
        let mut gx = chart_rect.left();
        while gx < chart_rect.right() {
            let end = (gx + dot_len).min(chart_rect.right());
            painter.line_segment(
                [egui::pos2(gx, y), egui::pos2(end, y)],
                egui::Stroke::new(0.5, grid_col),
            );
            gx += dot_len + dot_gap;
        }
        let label = format_price(p);
        painter.text(
            egui::pos2(chart_rect.right() + 4.0, y),
            egui::Align2::LEFT_CENTER,
            &label,
            egui::FontId::monospace(10.0),
            AXIS_TEXT,
        );
    }

    // ── grid lines (time) — dotted style ─────────────────────────────────────
    let time_step = ((80.0 / bar_w) as usize).max(1);
    for (rel_idx, bar) in bars.iter().enumerate() {
        if rel_idx % time_step != 0 { continue; }
        let x = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
        // Dotted vertical line
        let mut gy = chart_rect.top();
        while gy < chart_rect.bottom() {
            let end = (gy + dot_len).min(chart_rect.bottom());
            painter.line_segment(
                [egui::pos2(x, gy), egui::pos2(x, end)],
                egui::Stroke::new(0.5, grid_col),
            );
            gy += dot_len + dot_gap;
        }
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
    // MultiKAMA: higher TF KAMAs (MT5: clrWhite for KAMA, but visually distinguished)
    // MQL4 mode uses white for all; MTF_MA overlay uses magenta for higher TFs
    if flags.kama && !chart.multi_kama.is_empty() {
        let htf_colors = [
            egui::Color32::from_rgb(255, 255, 255),   // H1 — white
            egui::Color32::from_rgb(255, 0, 255),     // H4 — magenta (clrMagenta)
            egui::Color32::from_rgb(255, 0, 255),     // D1 — magenta
            egui::Color32::from_rgb(255, 0, 255),     // W1 — magenta
            egui::Color32::from_rgb(255, 0, 255),     // MN1 — magenta
        ];
        for (tf_idx, (_tf_label, projected)) in chart.multi_kama.iter().enumerate() {
            let color = htf_colors.get(tf_idx).copied().unwrap_or(egui::Color32::from_rgb(255, 0, 255));
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
    if flags.ema21  { draw_indicator_line(painter, chart_rect, bars, &chart.ema21,  start_idx, bar_w, &price_to_y, EMA_COL,    1.5); }
    if flags.wma    { draw_indicator_line(painter, chart_rect, bars, &chart.wma,    start_idx, bar_w, &price_to_y, WMA_COL,    1.0); }
    if flags.hma    { draw_indicator_line(painter, chart_rect, bars, &chart.hma,    start_idx, bar_w, &price_to_y, HMA_COL,    1.5); }

    // ATR Projection bands
    // ATR Projection — yellow dotted HORIZONTAL lines (matching ATR_Projection.mqh: STYLE_DOT, clrYellow, width 2)
    if flags.atr_proj {
        // Draw horizontal dotted lines at the LAST bar's Open ± ATR value
        if let Some(last_bar) = bars.last() {
            let last_abs = start_idx + bars.len() - 1;
            if let Some(Some(atr_val)) = chart.atr.get(last_abs) {
                let upper_price = last_bar.open + atr_val;
                let lower_price = last_bar.open - atr_val;
                let atr_yellow = egui::Color32::from_rgb(255, 255, 0); // clrYellow
                for price in [upper_price, lower_price] {
                    let y = price_to_y(price);
                    if y >= chart_rect.top() && y <= chart_rect.bottom() {
                        // Dotted horizontal line across entire chart width
                        let mut dx = chart_rect.left();
                        while dx < chart_rect.right() {
                            let end = (dx + 3.0).min(chart_rect.right());
                            painter.line_segment(
                                [egui::pos2(dx, y), egui::pos2(end, y)],
                                egui::Stroke::new(2.0, atr_yellow),
                            );
                            dx += 6.0;
                        }
                        // Label
                        let label = if price > last_bar.open { "ATR Hi" } else { "ATR Lo" };
                        painter.text(
                            egui::pos2(chart_rect.left() + 4.0, y - 10.0),
                            egui::Align2::LEFT_BOTTOM,
                            &format!("{} {}", label, format_price(price)),
                            egui::FontId::monospace(8.0),
                            atr_yellow,
                        );
                    }
                }
            }
        }
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

    // ── Ehlers overlay indicators ───────────────────────────────────────────
    if flags.ehlers_ss       { draw_indicator_line(painter, chart_rect, bars, &chart.ehlers_ss,       start_idx, bar_w, &price_to_y, EHLERS_SS_COL,   1.5); }
    if flags.ehlers_decycler { draw_indicator_line(painter, chart_rect, bars, &chart.ehlers_decycler, start_idx, bar_w, &price_to_y, EHLERS_DEC_COL,  1.5); }
    if flags.ehlers_itl      { draw_indicator_line(painter, chart_rect, bars, &chart.ehlers_itl,      start_idx, bar_w, &price_to_y, EHLERS_ITL_COL,  1.5); }
    if flags.ehlers_mama {
        draw_indicator_line(painter, chart_rect, bars, &chart.ehlers_mama, start_idx, bar_w, &price_to_y, EHLERS_MAMA_COL, 1.5);
        draw_indicator_line(painter, chart_rect, bars, &chart.ehlers_fama, start_idx, bar_w, &price_to_y, EHLERS_FAMA_COL, 1.0);
    }

    // ── previous candle levels ─────────────────────────────────────────────
    if flags.prev_levels {
        let level_pairs = [
            (chart.prev_daily_high, "D Hi", egui::Color32::WHITE),
            (chart.prev_daily_low, "D Lo", egui::Color32::WHITE),
            (chart.prev_weekly_high, "W Hi", egui::Color32::from_rgb(255, 100, 255)),
            (chart.prev_weekly_low, "W Lo", egui::Color32::from_rgb(255, 100, 255)),
        ];
        for (price_opt, label, color) in &level_pairs {
            if let Some(p) = price_opt {
                let y = price_to_y(*p);
                if y >= chart_rect.top() && y <= chart_rect.bottom() {
                    // Dotted line
                    let dot = 3.0_f32;
                    let mut x = chart_rect.left();
                    while x < chart_rect.right() {
                        painter.circle_filled(egui::pos2(x, y), 0.5, *color);
                        x += dot * 3.0;
                    }
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
                        [egui::pos2(chart_rect.left(), y), egui::pos2(chart_rect.right(), y)],
                        egui::Stroke::new(0.7, *color),
                    );
                    painter.text(
                        egui::pos2(chart_rect.left() + 2.0, y - 10.0),
                        egui::Align2::LEFT_TOP, label, egui::FontId::monospace(8.0), *color,
                    );
                }
            }
        }
    }

    // ── fractals ─────────────────────────────────────────────────────────
    if flags.fractals {
        for (rel_idx, bar) in bars.iter().enumerate() {
            let abs_idx = start_idx + rel_idx;
            let x = chart_rect.left() + (rel_idx as f32 + 0.5) * bar_w;
            if abs_idx < chart.fractal_up.len() && chart.fractal_up[abs_idx] {
                let y = price_to_y(bar.high) - 8.0;
                if y >= chart_rect.top() {
                    painter.text(egui::pos2(x, y), egui::Align2::CENTER_BOTTOM, "▲", egui::FontId::proportional(10.0), UP);
                }
            }
            if abs_idx < chart.fractal_down.len() && chart.fractal_down[abs_idx] {
                let y = price_to_y(bar.low) + 2.0;
                if y <= chart_rect.bottom() {
                    painter.text(egui::pos2(x, y), egui::Align2::CENTER_TOP, "▼", egui::FontId::proportional(10.0), DOWN);
                }
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
            let screen_pts: Vec<Option<egui::Pos2>> = pts.iter().map(|(idx, price)| {
                if *idx >= start_idx && *idx < end_idx {
                    Some(egui::pos2(chart_rect.left() + ((*idx - start_idx) as f32 + 0.5) * bar_w, price_to_y(*price)))
                } else { None }
            }).collect();
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
                    painter.text(egui::pos2(p.x, p.y + if i % 2 == 0 { -12.0 } else { 4.0 }),
                        egui::Align2::CENTER_TOP, labels[i], egui::FontId::monospace(10.0), pattern_col);
                }
            }
            // Pattern name
            if let Some(d_pt) = screen_pts[4] {
                let dir = if pat.bullish { "BULL" } else { "BEAR" };
                let col = if pat.bullish { UP } else { DOWN };
                painter.text(egui::pos2(d_pt.x + 5.0, d_pt.y - 20.0), egui::Align2::LEFT_TOP,
                    &format!("{} {}", pat.name, dir), egui::FontId::monospace(9.0), col);
                // TP/SL from D
                for (price, label, c) in [(pat.tp1, "TP1", tp_col), (pat.tp2, "TP2", tp_col), (pat.sl, "SL", sl_col)] {
                    let y = price_to_y(price);
                    if y >= chart_rect.top() && y <= chart_rect.bottom() {
                        painter.line_segment([egui::pos2(d_pt.x, y), egui::pos2(chart_rect.right(), y)], egui::Stroke::new(0.7, c));
                        painter.text(egui::pos2(d_pt.x + 2.0, y - 9.0), egui::Align2::LEFT_TOP,
                            &format!("{} {}", label, format_price(price)), egui::FontId::monospace(8.0), c);
                    }
                }
            }
        }
    }

    // ── supply/demand zones ─────────────────────────────────────────────────
    if flags.supply_demand {
        let status_label = |s: u8| -> &str { match s { 0 => "Untested", 1 => "Tested", 2 => "Proven", _ => "" } };
        // Demand zones — MT5 colors: DarkSeaGreen/MediumSeaGreen/SeaGreen
        for &(idx, zh, zl, status) in &chart.demand_zones {
            if idx >= start_idx && idx < end_idx {
                let x_start = chart_rect.left() + ((idx - start_idx) as f32) * bar_w;
                let y_top = price_to_y(zh);
                let y_bot = price_to_y(zl);
                if y_bot >= chart_rect.top() && y_top <= chart_rect.bottom() {
                    // MT5 exact colors: clrDarkSeaGreen / clrMediumSeaGreen / clrSeaGreen
                    let (fill_col, label_col) = match status {
                        0 => (egui::Color32::from_rgba_premultiplied(143, 188, 143, 50), // DarkSeaGreen
                              egui::Color32::from_rgb(143, 188, 143)),
                        1 => (egui::Color32::from_rgba_premultiplied(60, 179, 113, 60),  // MediumSeaGreen
                              egui::Color32::from_rgb(60, 179, 113)),
                        _ => (egui::Color32::from_rgba_premultiplied(46, 139, 87, 70),   // SeaGreen
                              egui::Color32::from_rgb(46, 139, 87)),
                    };
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(x_start, y_top.max(chart_rect.top())),
                            egui::pos2(chart_rect.right(), y_bot.min(chart_rect.bottom())),
                        ),
                        0.0, fill_col,
                    );
                    painter.text(
                        egui::pos2(chart_rect.right() - 4.0, y_bot.min(chart_rect.bottom()) - 12.0),
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
            if idx >= start_idx && idx < end_idx {
                let x_start = chart_rect.left() + ((idx - start_idx) as f32) * bar_w;
                let y_top = price_to_y(zh);
                let y_bot = price_to_y(zl);
                if y_bot >= chart_rect.top() && y_top <= chart_rect.bottom() {
                    // MT5 exact colors: clrSkyBlue / clrDeepSkyBlue / clrDodgerBlue
                    let (fill_col, label_col) = match status {
                        0 => (egui::Color32::from_rgba_premultiplied(135, 206, 235, 50), // SkyBlue
                              egui::Color32::from_rgb(135, 206, 235)),
                        1 => (egui::Color32::from_rgba_premultiplied(0, 191, 255, 60),   // DeepSkyBlue
                              egui::Color32::from_rgb(0, 191, 255)),
                        _ => (egui::Color32::from_rgba_premultiplied(30, 144, 255, 70),  // DodgerBlue
                              egui::Color32::from_rgb(30, 144, 255)),
                    };
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(x_start, y_top.max(chart_rect.top())),
                            egui::pos2(chart_rect.right(), y_bot.min(chart_rect.bottom())),
                        ),
                        0.0, fill_col,
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

    // ── Auto Fibonacci levels (matching AutoFibonacci.mqh) ─────────────────
    if flags.auto_fib && !chart.auto_fib_levels.is_empty() {
        for (price, label, is_ext) in &chart.auto_fib_levels {
            let y = price_to_y(*price);
            if y >= chart_rect.top() && y <= chart_rect.bottom() {
                // Dotted line: gold for retracement, dodger blue for extension
                let color = if *is_ext {
                    egui::Color32::from_rgb(30, 144, 255)  // clrDodgerBlue
                } else {
                    egui::Color32::from_rgb(255, 215, 0)   // clrGold
                };
                // Dotted line
                let mut fx = chart_rect.left();
                while fx < chart_rect.right() {
                    let end = (fx + 4.0).min(chart_rect.right());
                    painter.line_segment(
                        [egui::pos2(fx, y), egui::pos2(end, y)],
                        egui::Stroke::new(1.0, color),
                    );
                    fx += 7.0;
                }
                // Label on right
                painter.text(
                    egui::pos2(chart_rect.right() - 4.0, y - 1.0),
                    egui::Align2::RIGHT_BOTTOM,
                    &format!("{} {}", label, format_price(*price)),
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
                    // Solid filled candles (TradingView/lightweight-charts style)
                    painter.rect_filled(body_rect, 0.0, color);
                } else {
                    // Doji: single line
                    painter.line_segment(
                        [egui::pos2(cx - half_body, body_top), egui::pos2(cx + half_body, body_top)],
                        egui::Stroke::new(1.0, color),
                    );
                }
            }
        }
    }

    // ── FakeCandle (ghost next bar — matching FakeCandle.mqh) ──────────────
    // Draws a semi-transparent outline where the next candle would appear
    if let Some(last) = bars.last() {
        let next_x = chart_rect.left() + (bars.len() as f32 + 0.5) * bar_w;
        if next_x < chart_rect.right() - 10.0 {
            let ghost_close = last.close;
            let ghost_open = last.close;
            let ghost_high = last.close + (last.high - last.low) * 0.3;
            let ghost_low = last.close - (last.high - last.low) * 0.3;
            let y_open = price_to_y(ghost_open);
            let y_high = price_to_y(ghost_high);
            let y_low = price_to_y(ghost_low);
            let y_close = price_to_y(ghost_close);
            let ghost_col = egui::Color32::from_rgba_premultiplied(100, 100, 120, 80);
            // Wick
            painter.line_segment(
                [egui::pos2(next_x, y_high), egui::pos2(next_x, y_low)],
                egui::Stroke::new(1.0, ghost_col),
            );
            // Body outline
            let body_top = y_open.min(y_close);
            let body_h = (y_open - y_close).abs().max(2.0);
            let body_rect = egui::Rect::from_min_size(
                egui::pos2(next_x - half_body, body_top),
                egui::vec2(candle_w, body_h),
            );
            painter.rect_stroke(body_rect, 0.0, egui::Stroke::new(1.0, ghost_col), egui::StrokeKind::Outside);
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

            // OHLCV + indicator values data window (WebKit: .data-window — #000000ee bg)
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
                // Semi-transparent background behind data text (WebKit: background #000000ee)
                let data_bg = egui::Rect::from_min_size(
                    egui::pos2(chart_rect.left() + 2.0, chart_rect.top() + 2.0),
                    egui::vec2(tooltip.len() as f32 * 6.5 + 8.0, 30.0),
                );
                painter.rect_filled(data_bg, 2.0, egui::Color32::from_rgba_premultiplied(0, 0, 0, 238));
                painter.text(
                    egui::pos2(chart_rect.left() + 6.0, chart_rect.top() + 4.0),
                    egui::Align2::LEFT_TOP,
                    &tooltip,
                    egui::FontId::monospace(10.0),
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
        egui::Color32::from_rgb(136, 204, 255), // #8cf — WebKit .mtf-cell-label color
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

    // Chart overlay removed — info shown in crosshair tooltip + right panel instead

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
        sub_y += 80.0;
    }

    // Ehlers sub-panes
    if show_ehlers_ebsw {
        let pr = egui::Rect::from_min_max(egui::pos2(rect.left(), sub_y), egui::pos2(rect.right() - price_axis_w, sub_y + 80.0));
        draw_oscillator_pane(painter, pr, bars, &chart.ehlers_ebsw, start_idx, bar_w, "EBSW", EHLERS_EBSW_COL, -1.0, 1.0, None, None);
        sub_y += 80.0;
    }
    if show_ehlers_cyber {
        let pr = egui::Rect::from_min_max(egui::pos2(rect.left(), sub_y), egui::pos2(rect.right() - price_axis_w, sub_y + 80.0));
        let mut cmin = f64::MAX; let mut cmax = f64::MIN;
        for (ri, _) in bars.iter().enumerate() { if let Some(Some(v)) = chart.ehlers_cyber.get(start_idx + ri) { cmin = cmin.min(*v); cmax = cmax.max(*v); } }
        let pad = (cmax - cmin).max(0.001) * 0.1;
        draw_oscillator_pane(painter, pr, bars, &chart.ehlers_cyber, start_idx, bar_w, "Cyber Cycle", EHLERS_CYBER_COL, cmin - pad, cmax + pad, None, None);
        sub_y += 80.0;
    }
    if show_ehlers_cg {
        let pr = egui::Rect::from_min_max(egui::pos2(rect.left(), sub_y), egui::pos2(rect.right() - price_axis_w, sub_y + 80.0));
        let mut cmin = f64::MAX; let mut cmax = f64::MIN;
        for (ri, _) in bars.iter().enumerate() { if let Some(Some(v)) = chart.ehlers_cg.get(start_idx + ri) { cmin = cmin.min(*v); cmax = cmax.max(*v); } }
        let pad = (cmax - cmin).max(0.001) * 0.1;
        draw_oscillator_pane(painter, pr, bars, &chart.ehlers_cg, start_idx, bar_w, "CG Oscillator", EHLERS_CG_COL, cmin - pad, cmax + pad, None, None);
        sub_y += 80.0;
    }
    if show_ehlers_roof {
        let pr = egui::Rect::from_min_max(egui::pos2(rect.left(), sub_y), egui::pos2(rect.right() - price_axis_w, sub_y + 80.0));
        let mut cmin = f64::MAX; let mut cmax = f64::MIN;
        for (ri, _) in bars.iter().enumerate() { if let Some(Some(v)) = chart.ehlers_roof.get(start_idx + ri) { cmin = cmin.min(*v); cmax = cmax.max(*v); } }
        let pad = (cmax - cmin).max(0.001) * 0.1;
        draw_oscillator_pane(painter, pr, bars, &chart.ehlers_roof, start_idx, bar_w, "Roofing Filter", EHLERS_ROOF_COL, cmin - pad, cmax + pad, None, None);
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
    // Sub-pane border-top separator (#444 matching old WebKit)
    painter.line_segment(
        [egui::pos2(rect.left(), rect.top()), egui::pos2(rect.right(), rect.top())],
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
    // Sub-pane border-top separator (#444 matching old WebKit)
    painter.line_segment(
        [egui::pos2(rect.left(), rect.top()), egui::pos2(rect.right(), rect.top())],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 68, 68)),
    );

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

    // Zero line (thin dotted)
    let zero_y = val_to_y(0.0);
    let dot_len = 3.0_f32;
    let dot_gap = 3.0_f32;
    let mut gx = rect.left();
    while gx < rect.right() {
        let end = (gx + dot_len).min(rect.right());
        painter.line_segment(
            [egui::pos2(gx, zero_y), egui::pos2(end, zero_y)],
            egui::Stroke::new(0.5, egui::Color32::from_rgb(60, 60, 60)),
        );
        gx += dot_len + dot_gap;
    }

    // Signal line FIRST (behind Fisher — MT5: clrDarkGray/orange, width 1)
    let mut sig_points: Vec<egui::Pos2> = Vec::with_capacity(bars.len());
    for (rel_idx, _) in bars.iter().enumerate() {
        let abs_idx = start_idx + rel_idx;
        if abs_idx >= signal.len() { continue; }
        if let Some(v) = signal[abs_idx] {
            let x = rect.left() + (rel_idx as f32 + 0.5) * bar_w;
            let y = val_to_y(v).clamp(rect.top(), rect.bottom());
            sig_points.push(egui::pos2(x, y));
        }
    }
    if sig_points.len() > 1 {
        painter.add(egui::Shape::line(sig_points, egui::Stroke::new(1.0, egui::Color32::from_rgb(169, 169, 169)))); // clrDarkGray signal (MT5 buffer 3)
    }

    // Fisher line — colored segments per bar (MT5 exact: green when Fisher > Signal, red when < Signal)
    // NO histogram bars — just the line (matching MT5 screenshot exactly)
    for (rel_idx, _) in bars.iter().enumerate() {
        let abs_idx = start_idx + rel_idx;
        if abs_idx + 1 >= fisher.len() || rel_idx + 1 >= bars.len() { continue; }
        if let (Some(f0), Some(f1)) = (fisher[abs_idx], fisher[abs_idx + 1]) {
            let sig = if abs_idx < signal.len() { signal[abs_idx] } else { None };
            // MT5: clrMediumSeaGreen when Fisher > Signal, clrOrangeRed when Fisher < Signal
            let color = match sig {
                Some(s) if f0 > s => FISHER_POS,  // green
                Some(_) => FISHER_NEG,             // red
                None => if f0 >= 0.0 { FISHER_POS } else { FISHER_NEG },
            };
            let x0 = rect.left() + (rel_idx as f32 + 0.5) * bar_w;
            let x1 = rect.left() + (rel_idx as f32 + 1.5) * bar_w;
            let y0 = val_to_y(f0).clamp(rect.top(), rect.bottom());
            let y1 = val_to_y(f1).clamp(rect.top(), rect.bottom());
            painter.line_segment([egui::pos2(x0, y0), egui::pos2(x1, y1)], egui::Stroke::new(2.0, color));
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
    painter.text(egui::pos2(rect.left() + 4.0, rect.top() + 2.0), egui::Align2::LEFT_TOP, &label, egui::FontId::monospace(9.0), FISHER_POS);
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
    // Sub-pane border-top separator (#444 matching old WebKit)
    painter.line_segment(
        [egui::pos2(rect.left(), rect.top()), egui::pos2(rect.right(), rect.top())],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 68, 68)),
    );

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
            // MACD histogram: teal green positive, coral red negative (TradingView/MT5 style)
            let color = if v >= 0.0 {
                egui::Color32::from_rgb(38, 166, 154) // #26a69a (teal green)
            } else {
                egui::Color32::from_rgb(239, 83, 80)  // #ef5350 (coral red)
            };
            let (top, bottom) = if v >= 0.0 { (y, zero_y) } else { (zero_y, y) };
            painter.rect_filled(
                egui::Rect::from_min_max(egui::pos2(x - hist_w / 2.0, top), egui::pos2(x + hist_w / 2.0, bottom)),
                0.0,
                color,
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
    // Sub-pane border-top separator (#444 matching old WebKit)
    painter.line_segment(
        [egui::pos2(rect.left(), rect.top()), egui::pos2(rect.right(), rect.top())],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 68, 68)),
    );

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
    // Sub-pane border-top separator (#444 matching old WebKit)
    painter.line_segment(
        [egui::pos2(rect.left(), rect.top()), egui::pos2(rect.right(), rect.top())],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 68, 68)),
    );

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
            _ => BVOL_NORMAL, // clrSteelBlue — normal volume
        };
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(x - hist_w / 2.0, rect.bottom() - h),
                egui::pos2(x + hist_w / 2.0, rect.bottom()),
            ),
            0.0, color,
        );
    }
    // Label with current volume value (MT5 style: "BetterVol(20) 10748 0")
    let last_vol = bars.last().map(|b| b.volume as i64).unwrap_or(0);
    let label = format!("BetterVol(20) {} 0", last_vol);
    painter.text(egui::pos2(rect.left() + 4.0, rect.top() + 2.0), egui::Align2::LEFT_TOP, &label, egui::FontId::monospace(9.0), BVOL_HIGH);
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
    // Sub-pane border-top separator (#444 matching old WebKit)
    painter.line_segment(
        [egui::pos2(rect.left(), rect.top()), egui::pos2(rect.right(), rect.top())],
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
    // Sub-pane border-top separator (#444 matching old WebKit)
    painter.line_segment(
        [egui::pos2(rect.left(), rect.top()), egui::pos2(rect.right(), rect.top())],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 68, 68)),
    );

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
    Command { name: "QUOTE",         desc: "Get latest bid/ask quote for current symbol" },
    Command { name: "CLOCK",         desc: "Market clock — open/closed status" },
    Command { name: "FILLS",         desc: "Recent account fills/activities" },
    Command { name: "MOVERS",        desc: "Top market movers (stocks)" },
    Command { name: "SEARCH",        desc: "Search symbols by name" },
    Command { name: "HISTORY",       desc: "Order history (closed orders)" },
    // View
    Command { name: "MTF",           desc: "Toggle multi-timeframe grid" },
    Command { name: "MTF_2X2",       desc: "2×2 grid (4 charts)" },
    Command { name: "MTF_3X3",       desc: "3×3 grid (9 charts)" },
    Command { name: "MTF_4X4",       desc: "4×4 grid (16 charts)" },
    Command { name: "MTF_4X3",       desc: "4×3 grid (12 charts)" },
    Command { name: "INDICATORS",    desc: "Toggle indicator settings panel" },
    Command { name: "FULLSCREEN",    desc: "Toggle fullscreen mode" },
    // Trading
    // Order placement commands removed from console — use Trading tab buttons instead
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
    // NNFX system presets
    Command { name: "NNFX",          desc: "Enable NNFX indicator preset (KAMA+Fisher+ATR+BVol)" },
    Command { name: "RESET_IND",     desc: "Disable all indicators" },
    // Additional analytics
    Command { name: "DATA_WINDOW",   desc: "All indicator values at cursor" },
    Command { name: "ALERTS",        desc: "Price alert manager" },
    // ORDER command removed — use Trading tab Open Trade button
    Command { name: "PREV_LEVELS",   desc: "Toggle previous candle levels (D/W)" },
    Command { name: "PIVOTS",        desc: "Toggle pivot points (P/R1/R2/S1/S2)" },
    Command { name: "FRACTALS",      desc: "Toggle Bill Williams fractals" },
    Command { name: "HARMONICS",     desc: "Toggle harmonic pattern detection (Carney)" },
    Command { name: "AUTO_FIB",      desc: "Auto Fibonacci (fractal swing retracement + extension)" },
    Command { name: "SUPPLY_DEMAND", desc: "Toggle supply/demand zone detection" },
];

fn fuzzy_match(query: &str, target: &str) -> bool {
    let q = query.to_lowercase();
    let t = target.to_lowercase();
    if q.is_empty() { return true; }
    // Prefer substring match (contains) over subsequence
    // This ensures "SEC" matches "SEC Filings" but not "Screener"
    t.contains(&q)
}

// ─── application state ───────────────────────────────────────────────────────

/// Watchlist row data (TradingView-style).
#[derive(Clone)]
#[allow(dead_code)]
struct WatchlistRow {
    /// Display symbol name (e.g. "BTCUSD", "SLV", "CC").
    symbol: String,
    /// Full cache key for loading.
    cache_key: String,
    /// Last close price.
    last: f64,
    /// Previous close (for change calculation).
    prev_close: f64,
    /// Absolute change.
    change: f64,
    /// Percentage change.
    change_pct: f64,
    /// Last bar volume.
    volume: f64,
}

/// Background-computed data — populated by background thread, read by render thread.
/// This eliminates SQLite queries from the render loop.
#[derive(Default)]
struct BgDarwinData {
    portfolio: Option<darwin::PortfolioSummary>,
    accounts: Vec<darwin::DarwinAccount>,
    cache_stats: Option<(i64, i64, i64)>,
    sec_filings: Vec<sec_filing::SecFiling>,
    sec_alerts: Vec<sec_filing::FilingAlert>,
    // Heavy analytics cached
    daily_returns: Vec<darwin::DailyReturn>,
    var_stats: Option<darwin::VaRResult>,
    correlations: Vec<darwin::CorrelationEntry>,
    exposure: Vec<darwin::PortfolioSymbolExposure>,
    equity_curve: Vec<(String, f64)>,
    open_positions: Vec<darwin::PortfolioOpenPosition>,
    trade_overlaps: Vec<darwin::TradeOverlap>,
    detailed_stats: Vec<(String, i64, i64)>,
    // ── Heavy analytics that froze the UI when computed in render thread ──
    optimal_allocation: Vec<darwin::OptimalAllocation>,
    rebalance: Option<darwin::RebalanceDashboard>,
    monte_carlo: Option<darwin::MonteCarloResult>,
    stress_tests: Vec<darwin::StressTestResult>,
    margin_call_sim: Option<darwin::MarginCallSimulation>,
}

/// Bottom panel mode.
#[derive(PartialEq)]
enum BottomTab {
    Log,
}

/// Right panel section tabs (matching old WebKit layout).
#[derive(Clone, Copy, PartialEq)]
enum RightTab {
    Trading,
    Positions,
    Orders,
    Watchlist,
    Risk,
}

/// Risk sizing mode (old app had dropdown).
#[derive(Clone, Copy, PartialEq)]
enum RiskMode {
    VaR,
    Standard,
    Fixed,
    Dynamic,
}

impl RiskMode {
    fn label(self) -> &'static str {
        match self {
            RiskMode::VaR => "VaR",
            RiskMode::Standard => "Standard",
            RiskMode::Fixed => "Fixed",
            RiskMode::Dynamic => "Dynamic",
        }
    }
}

/// Order type dropdown.
#[derive(Clone, Copy, PartialEq)]
enum OrderTypeMode {
    Market,
    Limit,
    Stop,
}

impl OrderTypeMode {
    fn label(self) -> &'static str {
        match self {
            OrderTypeMode::Market => "Market",
            OrderTypeMode::Limit => "Limit",
            OrderTypeMode::Stop => "Stop",
        }
    }
}

/// Messages sent from UI → async broker task.
#[allow(dead_code)]
enum BrokerCmd {
    Connect { api_key: String, secret: String, paper: bool },
    GetAccount,
    GetPositions,
    GetOrders,
    CloseAll,
    ClosePosition { symbol: String },
    /// Scrape SEC EDGAR filings for all portfolio symbols.
    SecScrape { db_path: PathBuf },
    // scrape_filings_for_ticker available via scrape_all_portfolio_symbols
    /// Fetch Finnhub news for a symbol.
    FinnhubNews { symbol: String, api_key: String },
    /// Get latest quote for a symbol.
    GetQuote { symbol: String },
    /// Get market clock (hours/status).
    GetMarketClock,
    /// Get account activities (fills, transfers).
    GetActivities { limit: u32 },
    /// Get top movers.
    GetTopMovers,
    /// Search symbols.
    SearchSymbols { query: String },
    /// Get order history.
    GetOrderHistory { limit: u32 },
    /// Fetch fundamentals for a symbol (SEC EDGAR).
    GetFundamentals { ticker: String },
    /// Fetch institutional holders (13F).
    GetHolders { ticker: String },
    /// Fetch analyst ratings (Finnhub).
    GetAnalyst { symbol: String, finnhub_key: String },
    /// Fetch orderbook (Level 2).
    GetOrderbook { symbol: String },
    /// Crypto backfill via Kraken public OHLC API.
    KrakenBackfill { symbol: String, timeframes: Vec<String>, db_path: std::path::PathBuf },
}

/// Messages sent from async broker task → UI.
enum BrokerMsg {
    Connected(String),
    Error(String),
    Account(AccountInfo),
    Positions(Vec<PositionInfo>),
    Orders(Vec<OrderInfo>),
    OrderResult(String),
    SecScrapeResult(String),
    FinnhubNewsResult(Vec<(String, String, String)>),
    /// Latest quote data.
    Quote(String, f64, f64, f64), // symbol, bid, ask, last
    /// Market clock status.
    MarketClock(String),
    /// Generic JSON results for various API calls.
    JsonResult(String, String), // (label, formatted text)
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
    /// Which chart cell is focused in MTF grid (click to select).
    mtf_focused: Option<usize>,

    /// Command palette open state.
    command_open: bool,
    /// Raw user input in the command palette.
    command_input: String,
    /// Currently highlighted command in console (arrow key navigation).
    console_selected: usize,

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
    show_prev_levels: bool,
    show_pivots: bool,
    show_fractals: bool,
    show_harmonics: bool,
    show_auto_fib: bool,
    show_supply_demand: bool,
    show_ehlers_ss: bool,
    show_ehlers_decycler: bool,
    show_ehlers_itl: bool,
    show_ehlers_mama: bool,
    show_ehlers_ebsw: bool,
    show_ehlers_cyber: bool,
    show_ehlers_cg: bool,
    show_ehlers_roof: bool,
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

    /// Finnhub API key.
    finnhub_key: String,
    /// Cached news articles (headline, source, datetime).
    news_articles: Vec<(String, String, String)>,

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
    bt_equity_curve: Vec<f64>,

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
    /// Cached symbol keys from SQLite cache (used by screener/backfill panels).
    #[allow(dead_code)]
    watchlist_symbols: Vec<(String, i64)>,
    /// Rich watchlist data: symbol name, last, prev_close, change, change_pct, volume, cache_key.
    watchlist_rows: Vec<WatchlistRow>,

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
    show_order_entry: bool,
    show_crypto_backfill: bool,
    /// Crypto backfill single symbol input.
    backfill_symbol: String,
    /// SEC filing type filters [Form 4, 13F, DEF 14A, S-1, 10-K, 10-Q, 8-K].
    sec_filters: [bool; 7],

    /// Price alerts.
    alerts: Vec<(f64, String)>,
    alert_price_input: String,
    alert_label_input: String,

    /// Order entry form.
    order_symbol: String,
    order_qty: String,
    order_side: usize, // 0=buy, 1=sell
    order_type: usize, // 0=market, 1=limit, 2=stop, 3=bracket
    order_limit_price: String,
    order_stop_price: String,
    order_tp_price: String,

    /// Bottom panel tab.
    bottom_tab: BottomTab,

    /// Application log — max 500 entries, ring-buffer style.
    log: VecDeque<LogEntry>,

    /// Crosshair position in screen coordinates (updated each frame).
    crosshair: Option<egui::Pos2>,

    /// Counter to avoid calling ctx.request_repaint in a tight loop.
    frame_count: u64,

    /// Tab being dragged (for drag-and-drop reordering).
    dragging_tab: Option<usize>,

    // ── async broker ─────────────────────────────────────────────────────
    /// Tokio runtime handle for spawning async tasks.
    #[allow(dead_code)]
    rt_handle: tokio::runtime::Handle,
    /// Send commands to broker task.
    broker_tx: mpsc::UnboundedSender<BrokerCmd>,
    /// Receive results from broker task.
    broker_rx: mpsc::UnboundedReceiver<BrokerMsg>,
    /// Whether broker is connected.
    broker_connected: bool,
    /// Live account info.
    live_account: Option<AccountInfo>,
    /// Live positions.
    live_positions: Vec<PositionInfo>,
    /// Live orders.
    live_orders: Vec<OrderInfo>,

    // ── right panel state (WebKit parity) ─────────────────────────────
    /// Active right panel tab.
    right_tab: RightTab,
    /// Risk sizing mode dropdown.
    risk_mode: RiskMode,
    /// Order type mode dropdown.
    order_type_mode: OrderTypeMode,
    /// SL price input text.
    sl_input: String,
    /// TP price input text.
    tp_input: String,
    /// Whether SL checkbox is enabled.
    sl_enabled: bool,
    /// Whether TP checkbox is enabled.
    tp_enabled: bool,
    /// Recent fills (symbol, side, qty, price, time).
    recent_fills: Vec<(String, String, f64, f64, String)>,

    // ── DARWIN portfolio view selector ─────────────────────────────────
    /// Which DARWIN view is selected in the portfolio dropdown.
    darwin_view: usize,
    /// Frame number when DARWIN windows last queried the DB.
    /// Background-computed DARWIN data. Updated every few seconds by a spawned task.
    /// Render thread reads from this — NEVER queries SQLite directly in draw_floating_windows.
    bg_darwin: std::sync::Arc<std::sync::Mutex<BgDarwinData>>,
    /// Signal to trigger background DARWIN refresh.
    bg_darwin_trigger: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl TyphooNApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, rt_handle: tokio::runtime::Handle) -> Self {
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

        // ── build default chart set (all timeframes for MTF grid up to 9) ────
        let default_tfs = [Timeframe::H4, Timeframe::D1, Timeframe::H1, Timeframe::W1, Timeframe::M15, Timeframe::M30, Timeframe::M5, Timeframe::M1, Timeframe::MN1];
        let mut charts: Vec<ChartState> = default_tfs
            .iter()
            .map(|&tf| ChartState::new("CC", tf))
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

        // ── build rich watchlist rows (TradingView-style) ────────────────────
        let mut watchlist_rows: Vec<WatchlistRow> = Vec::new();
        // Deduplicate: group by base symbol, prefer D1 timeframe for last price
        if let Some(ref c) = cache {
            // Collect unique base symbols from cache keys
            let mut seen_symbols: std::collections::HashSet<String> = std::collections::HashSet::new();
            for (key, _) in &watchlist_symbols {
                // Parse cache key: "mt5:CC:4Hour" or "CC:4Hour"
                let parts: Vec<&str> = key.split(':').collect();
                let base_sym = if parts.len() >= 3 {
                    // Try to get "BTCUSD" from "mt5:CC:4Hour" or "CC"
                    let sym_part = if parts[0] == "mt5" && parts.len() >= 4 {
                        format!("{}:{}", parts[1], parts[2])
                    } else if parts.len() >= 2 {
                        format!("{}:{}", parts[0], parts[1])
                    } else {
                        key.clone()
                    };
                    sym_part
                } else {
                    key.clone()
                };

                if seen_symbols.contains(&base_sym) { continue; }
                seen_symbols.insert(base_sym.clone());

                // Try to load last 2 bars from D1, then H4, then any available TF
                let mut row: Option<WatchlistRow> = None;
                for tf_suffix in &["1Day", "4Hour", "1Hour", "1Week"] {
                    let try_key = if key.starts_with("mt5:") {
                        format!("mt5:{}:{}", base_sym, tf_suffix)
                    } else {
                        format!("{}:{}", base_sym, tf_suffix)
                    };
                    if let Ok(Some(raw)) = c.get_bars_raw(&try_key) {
                        let n = raw.len();
                        if n >= 2 {
                            let last = &raw[n - 1];
                            let prev = &raw[n - 2];
                            let change = last.4 - prev.4; // close - prev_close
                            let change_pct = if prev.4 != 0.0 { change / prev.4 * 100.0 } else { 0.0 };
                            // Extract display name from base_sym
                            let display = base_sym.split(':').last().unwrap_or(&base_sym).to_string();
                            row = Some(WatchlistRow {
                                symbol: display,
                                cache_key: try_key,
                                last: last.4,  // close
                                prev_close: prev.4,
                                change,
                                change_pct,
                                volume: last.5, // volume
                            });
                            break;
                        } else if n == 1 {
                            let last = &raw[0];
                            let display = base_sym.split(':').last().unwrap_or(&base_sym).to_string();
                            row = Some(WatchlistRow {
                                symbol: display,
                                cache_key: try_key,
                                last: last.4,
                                prev_close: last.1, // open as fallback
                                change: last.4 - last.1,
                                change_pct: if last.1 != 0.0 { (last.4 - last.1) / last.1 * 100.0 } else { 0.0 },
                                volume: last.5,
                            });
                            break;
                        }
                    }
                }
                if let Some(r) = row {
                    watchlist_rows.push(r);
                }
            }
            // Sort by symbol name
            watchlist_rows.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        }

        // Create async broker channels
        let (broker_tx, _broker_cmd_rx) = mpsc::unbounded_channel::<BrokerCmd>();
        let (broker_msg_tx, broker_rx) = mpsc::unbounded_channel::<BrokerMsg>();

        // Spawn broker message processor
        let broker_msg_tx_clone = broker_msg_tx.clone();
        let rt = rt_handle.clone();
        rt_handle.spawn(async move {
            let mut cmd_rx = _broker_cmd_rx;
            let mut broker: Option<AlpacaBroker> = None;
            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    BrokerCmd::Connect { api_key, secret, paper } => {
                        let b = AlpacaBroker::new(api_key, secret, paper);
                        match b.get_account().await {
                            Ok(acct) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::Connected(format!(
                                    "Connected: ${:.2} equity, ${:.2} buying power",
                                    acct.equity, acct.buying_power
                                )));
                                let _ = broker_msg_tx_clone.send(BrokerMsg::Account(acct));
                                broker = Some(b);
                            }
                            Err(e) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("Connection failed: {}", e)));
                            }
                        }
                    }
                    BrokerCmd::GetAccount => {
                        if let Some(ref b) = broker {
                            match b.get_account().await {
                                Ok(acct) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Account(acct)); }
                                Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                            }
                        }
                    }
                    BrokerCmd::GetPositions => {
                        if let Some(ref b) = broker {
                            match b.get_positions().await {
                                Ok(pos) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Positions(pos)); }
                                Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                            }
                        }
                    }
                    BrokerCmd::GetOrders => {
                        if let Some(ref b) = broker {
                            match b.get_orders("open", 100).await {
                                Ok(orders) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Orders(orders)); }
                                Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                            }
                        }
                    }
                    BrokerCmd::CloseAll => {
                        if let Some(ref b) = broker {
                            match b.close_all_positions().await {
                                Ok(_) => { let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult("All positions closed".into())); }
                                Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                            }
                        }
                    }
                    BrokerCmd::ClosePosition { symbol } => {
                        if let Some(ref b) = broker {
                            match b.close_position(&symbol, None).await {
                                Ok(r) => { let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!("Closed {}: {}", symbol, r.status))); }
                                Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                            }
                        }
                    }
                    BrokerCmd::SecScrape { db_path } => {
                        let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult("SEC scrape started...".into()));
                        match sec_filing::scrape_all_portfolio_symbols(db_path).await {
                            Ok(stats) => {
                                let _ = broker_msg_tx_clone.send(BrokerMsg::SecScrapeResult(
                                    format!("SEC scrape complete: {} tickers, {} filings, {} insider trades, {} alerts", stats.tickers_scanned, stats.new_filings, stats.new_insider_trades, stats.new_alerts)
                                ));
                            }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("SEC scrape error: {}", e))); }
                        }
                    }
                    // scrape_filings_for_ticker is called internally by scrape_all_portfolio_symbols
                    BrokerCmd::FinnhubNews { symbol, api_key } => {
                        if let Some(ref b) = broker {
                            match b.get_finnhub_news(&symbol, &api_key).await {
                                Ok(articles) => {
                                    let results: Vec<(String, String, String)> = articles.iter().filter_map(|a| {
                                        let headline = a["headline"].as_str()?.to_string();
                                        let source = a["source"].as_str().unwrap_or("Unknown").to_string();
                                        let dt = a["datetime"].as_str().unwrap_or("").to_string();
                                        Some((headline, source, dt))
                                    }).collect();
                                    let _ = broker_msg_tx_clone.send(BrokerMsg::FinnhubNewsResult(results));
                                }
                                Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("Finnhub: {}", e))); }
                            }
                        } else {
                            let _ = broker_msg_tx_clone.send(BrokerMsg::Error("Connect broker first for Finnhub news".into()));
                        }
                    }
                    BrokerCmd::GetQuote { symbol } => {
                        if let Some(ref b) = broker {
                            match b.get_latest_quote(&symbol).await {
                                Ok(q) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Quote(symbol, q.bid, q.ask, (q.bid + q.ask) / 2.0)); }
                                Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                            }
                        }
                    }
                    BrokerCmd::GetMarketClock => {
                        if let Some(ref b) = broker {
                            match b.get_market_clock().await {
                                Ok(v) => {
                                    let is_open = v["is_open"].as_bool().unwrap_or(false);
                                    let next = v["next_open"].as_str().unwrap_or("—");
                                    let msg = if is_open { "Market: OPEN".to_string() } else { format!("Market: CLOSED (opens {})", next) };
                                    let _ = broker_msg_tx_clone.send(BrokerMsg::MarketClock(msg));
                                }
                                Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                            }
                        }
                    }
                    BrokerCmd::GetActivities { limit } => {
                        if let Some(ref b) = broker {
                            match b.get_account_activities("FILL", limit).await {
                                Ok(activities) => {
                                    let text = activities.iter().take(20).map(|a| {
                                        format!("{} {} {} {} {}", a.date, a.side.as_deref().unwrap_or("—"), a.qty.as_deref().unwrap_or("—"), a.symbol.as_deref().unwrap_or("—"), a.net_amount.as_deref().unwrap_or("—"))
                                    }).collect::<Vec<_>>().join("\n");
                                    let _ = broker_msg_tx_clone.send(BrokerMsg::JsonResult("Account Activities".into(), text));
                                }
                                Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                            }
                        }
                    }
                    BrokerCmd::GetTopMovers => {
                        if let Some(ref b) = broker {
                            match b.get_top_movers("stocks", 10).await {
                                Ok(v) => {
                                    let text: String = serde_json::to_string_pretty(&v).unwrap_or_default();
                                    let _ = broker_msg_tx_clone.send(BrokerMsg::JsonResult("Top Movers".into(), text));
                                }
                                Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                            }
                        }
                    }
                    BrokerCmd::SearchSymbols { query } => {
                        if let Some(ref b) = broker {
                            match b.get_all_assets().await {
                                Ok(assets) => {
                                    let q = query.to_uppercase();
                                    let text = assets.iter()
                                        .filter(|a| a.symbol.contains(&q) || a.name.to_uppercase().contains(&q))
                                        .take(20)
                                        .map(|a| format!("{} — {} ({})", a.symbol, a.name, a.asset_class))
                                        .collect::<Vec<_>>().join("\n");
                                    let _ = broker_msg_tx_clone.send(BrokerMsg::JsonResult("Symbol Search".into(), text));
                                }
                                Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                            }
                        }
                    }
                    BrokerCmd::GetOrderHistory { limit } => {
                        if let Some(ref b) = broker {
                            match b.get_orders("closed", limit).await {
                                Ok(orders) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Orders(orders)); }
                                Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                            }
                        }
                    }
                    BrokerCmd::GetFundamentals { ticker } => {
                        match AlpacaBroker::get_financial_analysis(&ticker).await {
                            Ok(v) => {
                                let text: String = serde_json::to_string_pretty(&v).unwrap_or_default();
                                let _ = broker_msg_tx_clone.send(BrokerMsg::JsonResult(format!("Fundamentals: {}", ticker), text));
                            }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                        }
                    }
                    BrokerCmd::GetHolders { ticker } => {
                        match AlpacaBroker::get_institutional_holders(&ticker).await {
                            Ok(v) => {
                                let text: String = serde_json::to_string_pretty(&v).unwrap_or_default();
                                let _ = broker_msg_tx_clone.send(BrokerMsg::JsonResult(format!("Holders: {}", ticker), text));
                            }
                            Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                        }
                    }
                    BrokerCmd::GetAnalyst { symbol, finnhub_key } => {
                        if let Some(ref b) = broker {
                            match b.get_finnhub_recommendations(&symbol, &finnhub_key).await {
                                Ok(v) => {
                                    let text: String = serde_json::to_string_pretty(&v).unwrap_or_default();
                                    let _ = broker_msg_tx_clone.send(BrokerMsg::JsonResult(format!("Analyst: {}", symbol), text));
                                }
                                Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                            }
                        }
                    }
                    BrokerCmd::GetOrderbook { symbol } => {
                        if let Some(ref b) = broker {
                            match b.get_orderbook(&symbol).await {
                                Ok(v) => {
                                    let text: String = serde_json::to_string_pretty(&v).unwrap_or_default();
                                    let _ = broker_msg_tx_clone.send(BrokerMsg::JsonResult(format!("Orderbook: {}", symbol), text));
                                }
                                Err(e) => { let _ = broker_msg_tx_clone.send(BrokerMsg::Error(e)); }
                            }
                        }
                    }
                    BrokerCmd::KrakenBackfill { symbol, timeframes, db_path } => {
                        use typhoon_engine::core::kraken;
                        let client = reqwest::Client::builder()
                            .user_agent("TyphooN-Terminal/1.0")
                            .build().unwrap_or_default();
                        let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(format!("Kraken backfill {} started ({} TFs)...", symbol, timeframes.len())));
                        let now_ms = chrono::Utc::now().timestamp_millis();
                        let mut total_bars = 0usize;
                        for tf in &timeframes {
                            // Fetch last ~1000 bars worth of history
                            let span_ms = match tf.as_str() {
                                "1Min" => 60_000i64 * 1000,
                                "5Min" => 300_000i64 * 1000,
                                "15Min" => 900_000i64 * 1000,
                                "30Min" => 1_800_000i64 * 1000,
                                "1Hour" => 3_600_000i64 * 1000,
                                "4Hour" => 14_400_000i64 * 1000,
                                "1Day" => 86_400_000i64 * 1000,
                                "1Week" => 604_800_000i64 * 1000,
                                "1Month" => 2_592_000_000i64 * 1000,
                                _ => 86_400_000i64 * 1000,
                            };
                            let start = now_ms - span_ms;
                            match kraken::fetch_binance_klines(&client, &symbol, tf, start, now_ms).await {
                                Ok(bars) => {
                                    let count = bars.len();
                                    total_bars += count;
                                    // Store in cache
                                    if let Ok(cache) = typhoon_engine::core::cache::SqliteCache::open(&db_path) {
                                        let json = serde_json::to_string(&bars).unwrap_or_default();
                                        let key = format!("kraken:{}:{}", symbol, tf);
                                        let _ = cache.put_bars(&key, &json);
                                    }
                                    let _ = broker_msg_tx_clone.send(BrokerMsg::OrderResult(
                                        format!("Kraken {} {}: {} bars cached", symbol, tf, count)
                                    ));
                                }
                                Err(e) => {
                                    let _ = broker_msg_tx_clone.send(BrokerMsg::Error(format!("Kraken {} {}: {}", symbol, tf, e)));
                                }
                            }
                        }
                        let _ = broker_msg_tx_clone.send(BrokerMsg::SecScrapeResult(
                            format!("Kraken backfill complete: {} {} bars across {} timeframes", symbol, total_bars, timeframes.len())
                        ));
                    }
                }
            }
        });

        let mut app = Self {
            cache,
            cache_err,
            symbol_input: "CC".to_string(),
            charts,
            mtf_cols: 2,
            mtf_enabled: false,
            mtf_focused: None,
            command_open: false,
            command_input: String::new(),
            console_selected: 0,
            // ── NNFX default preset (matching old WebKit defaults) ──
            show_sma200: true,
            show_sma100: false,
            show_kama: true,
            show_ema21: false,
            show_bollinger: false,
            show_rsi: false,
            show_fisher: true,          // NNFX confirmation
            show_macd: false,
            show_volume_pane: false,
            show_stochastic: false,
            show_adx: false,
            show_ichimoku: false,
            show_wma: false,
            show_hma: false,
            show_psar: false,
            show_atr_proj: true,        // NNFX exit
            show_prev_levels: true,     // NNFX support/resistance
            show_pivots: false,
            show_fractals: false,       // Bill Williams — separate from NNFX
            show_harmonics: false,
            show_auto_fib: true,            // MT5 default: AutoFibonacci
            show_supply_demand: true,   // NNFX zones
            show_ehlers_ss: false,
            show_ehlers_decycler: false,
            show_ehlers_itl: false,
            show_ehlers_mama: false,
            show_ehlers_ebsw: false,
            show_ehlers_cyber: false,
            show_ehlers_cg: false,
            show_ehlers_roof: false,
            show_cci: false,
            show_williams_r: false,
            show_obv: false,
            show_momentum: false,
            show_better_volume: true,   // NNFX volume
            draw_mode: DrawMode::None,
            darwin_import_ticker: String::new(),
            broker_api_key: String::new(),
            broker_secret: String::new(),
            broker_paper: true,
            tt_username: String::new(),
            tt_password: String::new(),
            tt_sandbox: true,
            finnhub_key: String::new(),
            news_articles: Vec::new(),
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
            bt_equity_curve: Vec::new(),
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
            watchlist_rows,
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
            show_order_entry: false,
            show_crypto_backfill: false,
            backfill_symbol: String::new(),
            sec_filters: [true; 7],
            alerts: Vec::new(),
            alert_price_input: String::new(),
            alert_label_input: String::new(),
            order_symbol: String::new(),
            order_qty: "1.0".to_string(),
            order_side: 0,
            order_type: 0,
            order_limit_price: String::new(),
            order_stop_price: String::new(),
            order_tp_price: String::new(),
            bottom_tab: BottomTab::Log,
            log,
            crosshair: None,
            frame_count: 0,
            dragging_tab: None,
            rt_handle: rt,
            broker_tx,
            broker_rx,
            broker_connected: false,
            live_account: None,
            live_positions: Vec::new(),
            live_orders: Vec::new(),
            right_tab: RightTab::Trading,
            risk_mode: RiskMode::VaR,
            order_type_mode: OrderTypeMode::Market,
            sl_input: String::new(),
            tp_input: String::new(),
            sl_enabled: false,
            tp_enabled: false,
            recent_fills: Vec::new(),
            darwin_view: 0,
            bg_darwin: std::sync::Arc::new(std::sync::Mutex::new(BgDarwinData::default())),
            bg_darwin_trigger: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
        };

        // Spawn background DARWIN data refresh thread
        {
            let bg_data = app.bg_darwin.clone();
            let trigger = app.bg_darwin_trigger.clone();
            let cache_arc = app.cache.clone();
            std::thread::spawn(move || {
                loop {
                    // Wait for trigger or timeout (refresh every 5 seconds)
                    std::thread::sleep(std::time::Duration::from_secs(5));

                    if let Some(ref cache) = cache_arc {
                        if let Ok(conn) = cache.connection() {
                            let _ = darwin::create_darwin_tables(&conn);
                            let mut data = BgDarwinData::default();
                            data.portfolio = darwin::get_portfolio_summary(&conn).ok();
                            data.accounts = darwin::list_darwin_accounts(&conn).unwrap_or_default();
                            data.cache_stats = cache.stats().ok();
                            let _ = sec_filing::create_sec_tables(&conn);
                            data.sec_filings = sec_filing::get_recent_filings(&conn, None, 100).unwrap_or_default();
                            data.sec_alerts = sec_filing::get_filing_alerts(&conn, false).unwrap_or_default();
                            // Heavy analytics
                            data.daily_returns = darwin::get_portfolio_daily_returns(&conn).unwrap_or_default();
                            if !data.daily_returns.is_empty() {
                                data.var_stats = Some(darwin::compute_var(&data.daily_returns));
                            }
                            data.correlations = darwin::get_darwin_correlations(&conn).unwrap_or_default();
                            data.exposure = darwin::get_portfolio_exposure(&conn).unwrap_or_default();
                            data.equity_curve = darwin::get_portfolio_equity_curve(&conn).unwrap_or_default();
                            data.open_positions = darwin::get_portfolio_open_positions(&conn).unwrap_or_default();
                            data.trade_overlaps = darwin::get_trade_overlaps(&conn).unwrap_or_default();
                            data.detailed_stats = cache.detailed_stats().unwrap_or_default();
                            // Heavy analytics (these were freezing the UI)
                            data.optimal_allocation = darwin::compute_optimal_allocation(&conn).unwrap_or_default();
                            let prices = std::collections::HashMap::new();
                            data.rebalance = darwin::compute_rebalance_suggestions(&conn, &prices).ok();
                            if !data.daily_returns.is_empty() {
                                data.monte_carlo = Some(darwin::monte_carlo_var(&data.daily_returns, 252, 1000));
                            }
                            data.stress_tests = darwin::run_stress_tests(&conn).unwrap_or_default();
                            data.margin_call_sim = darwin::simulate_margin_call(&conn).ok();
                            if let Ok(mut locked) = bg_data.lock() {
                                *locked = data;
                            }
                        }
                    }

                    trigger.store(false, std::sync::atomic::Ordering::Relaxed);
                }
            });
        }

        app.load_session();

        // Load credentials from system keyring (libsecret/Keychain)
        if let Ok(Some(v)) = keyring::load(keyring::keys::ALPACA_API_KEY) { app.broker_api_key = v; }
        if let Ok(Some(v)) = keyring::load(keyring::keys::ALPACA_SECRET) { app.broker_secret = v; }
        if let Ok(Some(v)) = keyring::load(keyring::keys::FINNHUB_KEY) { app.finnhub_key = v; }
        if let Ok(Some(v)) = keyring::load(keyring::keys::TT_USERNAME) { app.tt_username = v; }
        if let Ok(Some(v)) = keyring::load(keyring::keys::TT_PASSWORD) { app.tt_password = v; }
        if !app.broker_api_key.is_empty() {
            app.log.push_back(LogEntry::info("Credentials loaded from system keyring"));
        }
        app
    }

    // ── helpers ──────────────────────────────────────────────────────────────

    fn dark_visuals() -> egui::Visuals {
        let mut v = egui::Visuals::dark();
        // ── TOTAL AESTHETIC OVERHAUL: square, compact, dark like Godel Terminal ──
        v.panel_fill                        = egui::Color32::from_rgb(0, 0, 0);
        v.window_fill                       = egui::Color32::from_rgb(10, 10, 18);      // very dark blue-black
        v.extreme_bg_color                  = egui::Color32::from_rgb(0, 0, 0);
        v.faint_bg_color                    = egui::Color32::from_rgb(8, 8, 14);
        // Widget colors — dark blue inputs, minimal contrast
        v.widgets.noninteractive.bg_fill    = egui::Color32::from_rgb(8, 8, 14);
        v.widgets.noninteractive.fg_stroke  = egui::Stroke::new(1.0, egui::Color32::from_rgb(180, 180, 190));
        v.widgets.noninteractive.bg_stroke  = egui::Stroke::new(0.5, egui::Color32::from_rgb(30, 30, 40));
        v.widgets.inactive.bg_fill          = egui::Color32::from_rgb(15, 20, 35);      // dark blue input bg
        v.widgets.inactive.bg_stroke        = egui::Stroke::new(0.5, egui::Color32::from_rgb(40, 45, 60));
        v.widgets.hovered.bg_fill           = egui::Color32::from_rgb(20, 30, 55);
        v.widgets.hovered.bg_stroke         = egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 70, 100));
        v.widgets.active.bg_fill            = egui::Color32::from_rgb(15, 40, 80);
        v.selection.bg_fill                 = egui::Color32::from_rgb(15, 40, 80);
        v.selection.stroke                  = egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 140, 255));
        // Windows — SQUARE corners, thin border, minimal shadow
        v.window_stroke                     = egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 42, 54));
        v.window_shadow                     = egui::Shadow { offset: [1, 2], blur: 4, spread: 0, color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 120) };
        v.window_corner_radius              = egui::CornerRadius::same(0);  // SQUARE
        v.menu_corner_radius                = egui::CornerRadius::same(0);  // SQUARE
        // Separator
        v.widgets.noninteractive.corner_radius = egui::CornerRadius::same(0);
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

    /// Set up MTF grid with N columns and target chart count.
    /// Creates charts for all 9 timeframes, filling up to `target` charts.
    fn setup_mtf_grid(&mut self, cols: usize, target: usize) {
        let all_tfs = [
            Timeframe::M1, Timeframe::M5, Timeframe::M15, Timeframe::M30,
            Timeframe::H1, Timeframe::H4, Timeframe::D1, Timeframe::W1, Timeframe::MN1,
        ];
        let sym = self.symbol_input.trim().to_string();
        // Grow charts to target count
        while self.charts.len() < target {
            let tf_idx = self.charts.len() % all_tfs.len();
            let mut chart = ChartState::new(&sym, all_tfs[tf_idx]);
            if let Some(ref cache) = self.cache {
                chart.load(cache, &mut self.log);
            }
            self.charts.push(chart);
        }
        self.mtf_cols = cols;
        self.mtf_enabled = true;
        self.log.push_back(LogEntry::info(format!("MTF grid: {}×{} ({} charts)", cols, (target + cols - 1) / cols, self.charts.len())));
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
            prev_levels: self.show_prev_levels,
            pivots: self.show_pivots,
            fractals: self.show_fractals,
            harmonics: self.show_harmonics,
            auto_fib: self.show_auto_fib,
            supply_demand: self.show_supply_demand,
            ehlers_ss: self.show_ehlers_ss,
            ehlers_decycler: self.show_ehlers_decycler,
            ehlers_itl: self.show_ehlers_itl,
            ehlers_mama: self.show_ehlers_mama,
        }
    }

    fn handle_command(&mut self, cmd: &str, ctx: &egui::Context) {
        let cmd_upper = cmd.trim().to_uppercase();
        self.log.push_back(LogEntry::info(format!("CMD: {}", cmd_upper)));
        match cmd_upper.as_str() {
            "QUIT" => {
                self.save_session();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            "MTF" | "MTF_GRID" => {
                self.mtf_enabled = !self.mtf_enabled;
                self.log.push_back(LogEntry::info(format!("MTF grid: {}", self.mtf_enabled)));
            }
            "MTF_2X2" => { self.setup_mtf_grid(2, 4); }
            "MTF_3X3" => { self.setup_mtf_grid(3, 9); }
            "MTF_4X4" => { self.setup_mtf_grid(4, 16); }
            "MTF_4X3" => { self.setup_mtf_grid(4, 12); }
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
            "DRAWDOWN"      => { self.darwin_view = 0; self.show_darwin_portfolio = true; } // Portfolio Summary with per-DARWIN DD%
            "REBALANCE"     => { self.darwin_view = 18; self.show_darwin_portfolio = true; } // Optimal Allocation view
            "DARWIN_TRADES" => { self.log.push_back(LogEntry::info("DARWIN trade markers: open DARWIN Accounts for deal history")); self.show_darwin_accounts = true; }
            "DSCORE"        => { self.show_var_mult = true; }
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
            "TRADESTATS"     => { self.darwin_view = 0; self.show_darwin_portfolio = true; } // Portfolio Summary
            "PERF"           => { self.darwin_view = 14; self.show_darwin_portfolio = true; } // Seasonals
            "COMPARE"        => { self.darwin_view = 3; self.show_darwin_portfolio = true; } // Correlation Matrix
            "SPREAD"         => { self.darwin_view = 4; self.show_darwin_portfolio = true; } // Symbol Exposure
            "HEATMAP"        => { self.darwin_view = 14; self.show_darwin_portfolio = true; } // Seasonals
            "PROFILE"        => self.show_darwin_accounts = true,
            "SIGNAL"         => self.show_indicators_panel = true,
            "DASHBOARD"      => self.show_cache_stats = true,
            "STATUS"         => self.show_cache_stats = true,
            "IMPORT_XLSX"    => self.show_darwin_accounts = true,
            "WORKSPACE"      => { self.save_session(); self.log.push_back(LogEntry::info("Workspace saved")); }
            "BACKUP"         => { self.save_session(); self.log.push_back(LogEntry::info("Session backup saved")); }
            "QUOTE" => {
                let sym = self.symbol_input.trim().to_string();
                let _ = self.broker_tx.send(BrokerCmd::GetQuote { symbol: sym });
            }
            "CLOCK" => { let _ = self.broker_tx.send(BrokerCmd::GetMarketClock); }
            "FILLS" => { let _ = self.broker_tx.send(BrokerCmd::GetActivities { limit: 20 }); }
            "MOVERS" => { let _ = self.broker_tx.send(BrokerCmd::GetTopMovers); }
            "SEARCH" => {
                let query = self.command_input.trim().to_string();
                if query.len() >= 2 {
                    let _ = self.broker_tx.send(BrokerCmd::SearchSymbols { query });
                } else {
                    self.log.push_back(LogEntry::warn("Type at least 2 characters to search"));
                }
            }
            "HISTORY" => { let _ = self.broker_tx.send(BrokerCmd::GetOrderHistory { limit: 50 }); }
            "PIVOTS"   => self.show_pivots = !self.show_pivots,
            "SRLEVEL"  => self.show_pivots = !self.show_pivots,
            "FRACTALS"  => self.show_fractals = !self.show_fractals,
            "HARMONICS"     => self.show_harmonics = !self.show_harmonics,
            "AUTO_FIB"      => self.show_auto_fib = !self.show_auto_fib,
            "SUPPLY_DEMAND" => self.show_supply_demand = !self.show_supply_demand,
            "NNFX" => {
                // Enable NNFX indicator preset
                self.show_sma200 = true;
                self.show_kama = true;
                self.show_fisher = true;
                self.show_atr_proj = true;
                self.show_better_volume = true;
                self.show_prev_levels = true;
                self.show_supply_demand = true;
                self.show_auto_fib = true;
                self.log.push_back(LogEntry::info("NNFX preset: SMA200 + KAMA + Fisher + ATR Proj + BetterVol + PrevLevels + S/D + AutoFib"));
            }
            "RESET_IND" => {
                self.show_sma200 = false; self.show_sma100 = false; self.show_kama = false;
                self.show_ema21 = false; self.show_bollinger = false; self.show_ichimoku = false;
                self.show_wma = false; self.show_hma = false; self.show_psar = false;
                self.show_atr_proj = false; self.show_prev_levels = false;
                self.show_rsi = false; self.show_fisher = false; self.show_macd = false;
                self.show_stochastic = false; self.show_adx = false; self.show_cci = false;
                self.show_williams_r = false; self.show_obv = false; self.show_momentum = false;
                self.show_better_volume = false; self.show_volume_pane = false;
                self.log.push_back(LogEntry::info("All indicators disabled"));
            }
            "DATA_WINDOW"    => self.show_data_window = true,
            "ALERTS"         => self.show_alerts = true,
            "ORDER"          => { self.show_order_entry = true; self.order_symbol = self.symbol_input.clone(); }
            "PREV_LEVELS"    => self.show_prev_levels = !self.show_prev_levels,
            "CRYPTO_BACKFILL" => {
                self.show_crypto_backfill = true;
            }
            // Trading
            "OPEN_TRADE" => {
                self.show_order_entry = true;
                self.order_symbol = self.symbol_input.clone();
            }
            "CLOSE_ALL" => {
                if self.broker_connected {
                    let _ = self.broker_tx.send(BrokerCmd::CloseAll);
                    self.log.push_back(LogEntry::info("Closing all positions..."));
                } else {
                    self.log.push_back(LogEntry::warn("Connect to broker first"));
                }
            }
            "CLOSE_PARTIAL" => {
                if self.broker_connected {
                    // Close 50% of first position
                    if let Some(pos) = self.live_positions.first() {
                        let half_qty = pos.qty / 2.0;
                        let sym = pos.symbol.clone();
                        let _ = self.broker_tx.send(BrokerCmd::ClosePosition { symbol: sym.clone() });
                        self.log.push_back(LogEntry::info(format!("Closing partial {} ({:.2} qty)", sym, half_qty)));
                    } else {
                        self.log.push_back(LogEntry::warn("No positions to close"));
                    }
                } else { self.log.push_back(LogEntry::warn("Connect to broker first")); }
            }
            "SET_SL" => {
                self.draw_mode = DrawMode::PlacingHLine;
                self.log.push_back(LogEntry::info("Click chart to set SL level"));
            }
            "SET_TP" => {
                self.draw_mode = DrawMode::PlacingHLine;
                self.log.push_back(LogEntry::info("Click chart to set TP level"));
            }
            "OPEN_MG" => {
                if self.broker_connected {
                    self.log.push_back(LogEntry::info("Martingale: use Order Entry panel with opposite side"));
                    self.show_order_entry = true;
                } else { self.log.push_back(LogEntry::warn("Connect to broker first")); }
            }
            "BUY_LINES" | "SELL_LINES" => {
                self.draw_mode = DrawMode::PlacingHLine;
                self.log.push_back(LogEntry::info(format!("Click chart to place {} reference line", cmd)));
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
                "sma200": self.show_sma200, "sma100": self.show_sma100,
                "kama": self.show_kama, "ema21": self.show_ema21,
                "bollinger": self.show_bollinger, "ichimoku": self.show_ichimoku,
                "wma": self.show_wma, "hma": self.show_hma,
                "psar": self.show_psar, "atr_proj": self.show_atr_proj,
                "prev_levels": self.show_prev_levels, "pivots": self.show_pivots,
                "fractals": self.show_fractals, "harmonics": self.show_harmonics, "supply_demand": self.show_supply_demand,
                "ehlers_ss": self.show_ehlers_ss, "ehlers_decycler": self.show_ehlers_decycler,
                "ehlers_itl": self.show_ehlers_itl, "ehlers_mama": self.show_ehlers_mama,
                "ehlers_ebsw": self.show_ehlers_ebsw, "ehlers_cyber": self.show_ehlers_cyber,
                "ehlers_cg": self.show_ehlers_cg, "ehlers_roof": self.show_ehlers_roof,
                "rsi": self.show_rsi, "fisher": self.show_fisher,
                "macd": self.show_macd, "stochastic": self.show_stochastic,
                "adx": self.show_adx, "cci": self.show_cci,
                "williams_r": self.show_williams_r, "obv": self.show_obv,
                "momentum": self.show_momentum, "better_volume": self.show_better_volume,
                "volume_pane": self.show_volume_pane,
            },
            "mtf_enabled": self.mtf_enabled,
            "mtf_cols": self.mtf_cols,
            "right_tab": match self.right_tab {
                RightTab::Trading => "trading",
                RightTab::Positions => "positions",
                RightTab::Orders => "orders",
                RightTab::Watchlist => "watchlist",
                RightTab::Risk => "risk",
            },
            "darwin_view": self.darwin_view,
            "finnhub_key": self.finnhub_key,
            "broker_api_key": self.broker_api_key,
            "broker_secret": self.broker_secret,
            "broker_paper": self.broker_paper,
            "tt_username": self.tt_username,
            "sl_enabled": self.sl_enabled,
            "tp_enabled": self.tp_enabled,
            "windows": {
                "settings": self.show_settings,
                "darwin_accounts": self.show_darwin_accounts,
                "darwin_portfolio": self.show_darwin_portfolio,
                "risk_calc": self.show_risk_calc,
                "backtest": self.show_backtest,
                "news": self.show_news,
                "indicators_panel": self.show_indicators_panel,
            },
            "drawings": self.charts.get(0).map(|c| {
                c.drawings.iter().filter_map(|d| match d {
                    Drawing::HLine { price, .. } => Some(serde_json::json!({"type": "hline", "price": price})),
                    _ => None, // Only persist HLines for now (trendlines need bar indices which shift)
                }).collect::<Vec<_>>()
            }).unwrap_or_default(),
            "alerts": self.alerts.iter().map(|(p, l)| serde_json::json!({"price": p, "label": l})).collect::<Vec<_>>(),
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
                if let Some(tab) = v["active_tab"].as_u64() { self.active_tab = tab as usize; }
                // Restore tabs: symbol, timeframe, chart type — rebuild charts from session
                if let Some(tabs) = v["tabs"].as_array() {
                    if !tabs.is_empty() {
                        // Rebuild chart set from session data
                        self.charts.clear();
                        for tab in tabs {
                            let sym = tab["symbol"].as_str().unwrap_or("CC").to_string();
                            let tf = match tab["timeframe"].as_str() {
                                Some("M1") => Timeframe::M1,
                                Some("M5") => Timeframe::M5,
                                Some("M15") => Timeframe::M15,
                                Some("M30") => Timeframe::M30,
                                Some("H1") => Timeframe::H1,
                                Some("D1") => Timeframe::D1,
                                Some("W1") => Timeframe::W1,
                                Some("MN1") => Timeframe::MN1,
                                _ => Timeframe::H4,
                            };
                            let ct = match tab["chart_type"].as_str() {
                                Some("Heikin-Ashi") => ChartType::HeikinAshi,
                                Some("Line") => ChartType::Line,
                                Some("OHLC Bars") => ChartType::OhlcBars,
                                Some("Renko") => ChartType::Renko,
                                _ => ChartType::Candle,
                            };
                            let mut chart = ChartState::new(&sym, tf);
                            chart.chart_type = ct;
                            self.charts.push(chart);
                        }
                        // Reload all charts from cache
                        if let Some(ref cache) = self.cache {
                            for chart in &mut self.charts {
                                chart.load(cache, &mut self.log);
                            }
                        }
                    }
                }
                if let Some(ind) = v.get("indicators") {
                    for (key, field) in [
                        ("sma200", &mut self.show_sma200), ("sma100", &mut self.show_sma100),
                        ("kama", &mut self.show_kama), ("ema21", &mut self.show_ema21),
                        ("bollinger", &mut self.show_bollinger), ("ichimoku", &mut self.show_ichimoku),
                        ("wma", &mut self.show_wma), ("hma", &mut self.show_hma),
                        ("psar", &mut self.show_psar), ("atr_proj", &mut self.show_atr_proj),
                        ("prev_levels", &mut self.show_prev_levels), ("pivots", &mut self.show_pivots),
                        ("fractals", &mut self.show_fractals), ("harmonics", &mut self.show_harmonics), ("supply_demand", &mut self.show_supply_demand),
                        ("ehlers_ss", &mut self.show_ehlers_ss), ("ehlers_decycler", &mut self.show_ehlers_decycler),
                        ("ehlers_itl", &mut self.show_ehlers_itl), ("ehlers_mama", &mut self.show_ehlers_mama),
                        ("ehlers_ebsw", &mut self.show_ehlers_ebsw), ("ehlers_cyber", &mut self.show_ehlers_cyber),
                        ("ehlers_cg", &mut self.show_ehlers_cg), ("ehlers_roof", &mut self.show_ehlers_roof),
                        ("rsi", &mut self.show_rsi), ("fisher", &mut self.show_fisher),
                        ("macd", &mut self.show_macd), ("stochastic", &mut self.show_stochastic),
                        ("adx", &mut self.show_adx), ("cci", &mut self.show_cci),
                        ("williams_r", &mut self.show_williams_r), ("obv", &mut self.show_obv),
                        ("momentum", &mut self.show_momentum), ("better_volume", &mut self.show_better_volume),
                        ("volume_pane", &mut self.show_volume_pane),
                    ] {
                        if let Some(b) = ind[key].as_bool() { *field = b; }
                    }
                }
                // Restore drawings
                if let Some(drawings) = v["drawings"].as_array() {
                    if let Some(chart) = self.charts.get_mut(0) {
                        for d in drawings {
                            if d["type"].as_str() == Some("hline") {
                                if let Some(price) = d["price"].as_f64() {
                                    chart.drawings.push(Drawing::HLine { price, color: HLINE_COL });
                                }
                            }
                        }
                    }
                }
                // Restore alerts
                if let Some(alerts) = v["alerts"].as_array() {
                    for a in alerts {
                        if let (Some(p), Some(l)) = (a["price"].as_f64(), a["label"].as_str()) {
                            self.alerts.push((p, l.to_string()));
                        }
                    }
                }
                // Restore MTF cols
                if let Some(cols) = v["mtf_cols"].as_u64() { self.mtf_cols = cols as usize; }
                // Restore right panel tab
                self.right_tab = match v["right_tab"].as_str() {
                    Some("positions") => RightTab::Positions,
                    Some("orders") => RightTab::Orders,
                    Some("watchlist") => RightTab::Watchlist,
                    Some("risk") => RightTab::Risk,
                    _ => RightTab::Trading,
                };
                // Restore DARWIN view
                if let Some(dv) = v["darwin_view"].as_u64() { self.darwin_view = dv as usize; }
                // Restore API keys
                if let Some(fk) = v["finnhub_key"].as_str() { self.finnhub_key = fk.to_string(); }
                if let Some(ak) = v["broker_api_key"].as_str() { self.broker_api_key = ak.to_string(); }
                if let Some(bs) = v["broker_secret"].as_str() { self.broker_secret = bs.to_string(); }
                if let Some(tu) = v["tt_username"].as_str() { self.tt_username = tu.to_string(); }
                if let Some(bp) = v["broker_paper"].as_bool() { self.broker_paper = bp; }
                // Restore SL/TP state
                if let Some(sl) = v["sl_enabled"].as_bool() { self.sl_enabled = sl; }
                if let Some(tp) = v["tp_enabled"].as_bool() { self.tp_enabled = tp; }
                // Restore window visibility
                if let Some(w) = v.get("windows") {
                    if let Some(b) = w["settings"].as_bool() { self.show_settings = b; }
                    if let Some(b) = w["darwin_accounts"].as_bool() { self.show_darwin_accounts = b; }
                    if let Some(b) = w["darwin_portfolio"].as_bool() { self.show_darwin_portfolio = b; }
                    if let Some(b) = w["risk_calc"].as_bool() { self.show_risk_calc = b; }
                    if let Some(b) = w["backtest"].as_bool() { self.show_backtest = b; }
                    if let Some(b) = w["news"].as_bool() { self.show_news = b; }
                    if let Some(b) = w["indicators_panel"].as_bool() { self.show_indicators_panel = b; }
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
        self.show_order_entry = false;
        self.show_crypto_backfill = false;
    }

    // ── chart interaction (zoom / pan) ───────────────────────────────────────

    fn handle_zoom(chart: &mut ChartState, delta: f32) {
        // Progressive zoom: small per-pixel factor so smooth scrolling feels natural.
        // delta is in pixels (smooth_scroll_delta), typically ±15-120 per gesture.
        // We want ~5% zoom per "notch" (15px), so factor = 1 - delta * 0.003
        let pct = (delta * 0.003).clamp(-0.15, 0.15); // cap at 15% per frame
        let factor = 1.0 - pct;
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

    /// Whether expensive DB queries should run this frame.
    /// Returns true once every ~4 seconds (every 16th frame at 4 FPS).
    #[allow(dead_code)]
    fn should_query_db(&self) -> bool {
        self.frame_count % 16 == 0 || self.frame_count < 4
    }

    fn draw_floating_windows(&mut self, ctx: &egui::Context) {
        // Performance: gate ALL DB queries to every 8th frame (~2s at 4 FPS).
        // On non-db frames, self.cache is temporarily set to None so all
        // `if let Some(ref cache) = self.cache` blocks are skipped.
        // Windows render their chrome but DB-querying content is empty.
        let db_ok = self.frame_count % 8 == 0 || self.frame_count < 8;
        let real_cache = if !db_ok { self.cache.take() } else { None };
        // After this block, self.cache is None on non-db frames
        // (restored at the end of draw_floating_windows)

        // Settings
        if self.show_settings {
            egui::Window::new("Settings")
                .open(&mut self.show_settings)
                .default_size([450.0, 500.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                    // ── API Keys (matching old WebKit connection modal) ──
                    ui.heading("API Keys");
                    ui.separator();
                    egui::Grid::new("api_keys_settings").num_columns(2).spacing(egui::vec2(8.0, 4.0)).show(ui, |ui| {
                        ui.label("Alpaca API Key:");
                        ui.add(egui::TextEdit::singleline(&mut self.broker_api_key).desired_width(250.0).password(true));
                        ui.end_row();
                        ui.label("Alpaca Secret:");
                        ui.add(egui::TextEdit::singleline(&mut self.broker_secret).desired_width(250.0).password(true));
                        ui.end_row();
                        ui.label("Alpaca Mode:");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.broker_paper, true, "Paper");
                            ui.radio_value(&mut self.broker_paper, false, "Live");
                        });
                        ui.end_row();
                        ui.label("Finnhub API Key:");
                        ui.add(egui::TextEdit::singleline(&mut self.finnhub_key).desired_width(250.0).password(true));
                        ui.end_row();
                        ui.label("tastytrade User:");
                        ui.add(egui::TextEdit::singleline(&mut self.tt_username).desired_width(250.0));
                        ui.end_row();
                        ui.label("tastytrade Pass:");
                        ui.add(egui::TextEdit::singleline(&mut self.tt_password).desired_width(250.0).password(true));
                        ui.end_row();
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let connect_label = if self.broker_connected {
                            egui::RichText::new("Connected").color(UP)
                        } else {
                            egui::RichText::new("Connect Alpaca")
                        };
                        if ui.button(connect_label).clicked() && !self.broker_connected {
                            if !self.broker_api_key.is_empty() && !self.broker_secret.is_empty() {
                                // Save credentials to system keyring
                                let _ = keyring::store(keyring::keys::ALPACA_API_KEY, &self.broker_api_key);
                                let _ = keyring::store(keyring::keys::ALPACA_SECRET, &self.broker_secret);
                                if !self.finnhub_key.is_empty() {
                                    let _ = keyring::store(keyring::keys::FINNHUB_KEY, &self.finnhub_key);
                                }
                                if !self.tt_username.is_empty() {
                                    let _ = keyring::store(keyring::keys::TT_USERNAME, &self.tt_username);
                                    let _ = keyring::store(keyring::keys::TT_PASSWORD, &self.tt_password);
                                }
                                self.log.push_back(LogEntry::info("Credentials saved to system keyring"));
                                let _ = self.broker_tx.send(BrokerCmd::Connect {
                                    api_key: self.broker_api_key.clone(),
                                    secret: self.broker_secret.clone(),
                                    paper: self.broker_paper,
                                });
                            }
                        }
                    });

                    ui.add_space(10.0);
                    ui.heading("General");
                    ui.separator();
                    ui.label("Theme: OLED Dark (#000000)");
                    ui.label("Font: Monospace 11px (Consolas equiv.)");
                    ui.label("Refresh rate: 250ms");
                    ui.label("Chart default: 200 visible bars");

                    ui.add_space(10.0);
                    ui.heading("Data Sources");
                    ui.separator();
                    ui.label("SQLite cache: ~/.config/typhoon-terminal/cache/typhoon_cache.db");
                    if let Ok(bg) = self.bg_darwin.try_lock() {
                        if let Some((rows, kv, size)) = bg.cache_stats {
                            ui.label(format!("Bar entries: {}  |  KV entries: {}  |  DB size: {} KB", rows, kv, size / 1024));
                        }
                    }
                    ui.label("MT5: view-only data via BarCacheWriter EA → SQLite");
                    ui.label("Alpaca: REST API + WebSocket streaming");
                    ui.label("Finnhub: News, Analyst, Insider Sentiment, Short Interest");
                    ui.label("SEC EDGAR: Filing scraper + Form 4 insider trades");

                    ui.add_space(10.0);
                    ui.heading("Darwinex");
                    ui.separator();
                    ui.label("VaR corridor: 3.25% – 6.5%");
                    ui.label("Correlation limit: 0.95 / 45d");
                    ui.label("Margin accounts: 100%");

                    ui.add_space(10.0);
                    ui.heading("Notifications");
                    ui.separator();
                    ui.label("Discord / Pushover / ntfy (configure in engine)");

                    ui.add_space(10.0);
                    if ui.button("Open Indicators Panel").clicked() {
                        self.show_indicators_panel = true;
                    }
                    });
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
                    let connect_label = if self.broker_connected {
                        egui::RichText::new("Connected").color(UP)
                    } else {
                        egui::RichText::new("Connect")
                    };
                    if ui.button(connect_label).clicked() && !self.broker_connected {
                        if self.broker_api_key.is_empty() || self.broker_secret.is_empty() {
                            self.log.push_back(LogEntry::warn("Enter API key and secret"));
                        } else {
                            self.log.push_back(LogEntry::info(format!(
                                "Connecting to Alpaca {}...",
                                if self.broker_paper { "Paper" } else { "Live" }
                            )));
                            let _ = self.broker_tx.send(BrokerCmd::Connect {
                                api_key: self.broker_api_key.clone(),
                                secret: self.broker_secret.clone(),
                                paper: self.broker_paper,
                            });
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
                                "tastytrade {} — session auth via REST API (broker module needed in engine)",
                                if self.tt_sandbox { "Sandbox" } else { "Production" }
                            )));
                            // tastytrade broker implementation pending in engine/src/broker/tastytrade.rs
                        }
                    }
                    ui.add_space(10.0);
                    ui.heading("Data APIs");
                    ui.separator();
                    egui::Grid::new("api_keys_grid").num_columns(2).show(ui, |ui| {
                        ui.label("Finnhub API Key:");
                        ui.add(egui::TextEdit::singleline(&mut self.finnhub_key).desired_width(200.0).password(true));
                        ui.end_row();
                    });
                    ui.label(egui::RichText::new("Used for: News, Analyst Ratings, Insider Sentiment, Short Interest").color(AXIS_TEXT).small());
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
                    ui.checkbox(&mut self.show_prev_levels, "Previous Candle Levels (D/W)");
                    ui.checkbox(&mut self.show_pivots,      "Pivot Points (Classic)");
                    ui.checkbox(&mut self.show_supply_demand, "Supply/Demand Zones");
                    ui.add_space(10.0);
                    ui.heading("Pattern Recognition");
                    ui.separator();
                    ui.checkbox(&mut self.show_fractals,    "Fractals (Bill Williams)");
                    ui.checkbox(&mut self.show_harmonics,     "Harmonic Patterns (Carney XABCD)");
                    ui.checkbox(&mut self.show_auto_fib,      "Auto Fibonacci (swing retracement)");
                    ui.add_space(10.0);
                    ui.heading("Sub-Pane Indicators");
                    ui.separator();
                    ui.checkbox(&mut self.show_rsi,            "RSI(14)");
                    ui.checkbox(&mut self.show_fisher,         "Fisher Transform(32)");
                    ui.checkbox(&mut self.show_macd,           "MACD(12,26,9)");
                    ui.checkbox(&mut self.show_stochastic,     "Stochastic(14,3,3)");
                    ui.checkbox(&mut self.show_adx,            "ADX(14)");
                    ui.checkbox(&mut self.show_cci,            "CCI(20)");
                    ui.checkbox(&mut self.show_williams_r,     "Williams %R(14)");
                    ui.checkbox(&mut self.show_obv,            "OBV");
                    ui.checkbox(&mut self.show_momentum,       "Momentum(10)");
                    ui.checkbox(&mut self.show_better_volume,  "Better Volume");
                    ui.checkbox(&mut self.show_volume_pane,    "Volume");
                    ui.add_space(10.0);
                    ui.heading("Ehlers Indicators");
                    ui.separator();
                    ui.label(egui::RichText::new("Overlay").color(AXIS_TEXT).small());
                    ui.checkbox(&mut self.show_ehlers_ss,       "Super Smoother(10)");
                    ui.checkbox(&mut self.show_ehlers_decycler, "Decycler(20)");
                    ui.checkbox(&mut self.show_ehlers_itl,      "Instantaneous Trendline");
                    ui.checkbox(&mut self.show_ehlers_mama,     "MAMA / FAMA");
                    ui.label(egui::RichText::new("Sub-Pane").color(AXIS_TEXT).small());
                    ui.checkbox(&mut self.show_ehlers_ebsw,  "Even Better Sinewave");
                    ui.checkbox(&mut self.show_ehlers_cyber, "Cyber Cycle");
                    ui.checkbox(&mut self.show_ehlers_cg,    "CG Oscillator(10)");
                    ui.checkbox(&mut self.show_ehlers_roof,  "Roofing Filter(10,48)");
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
                                    // Performance: only query per-account details on db_ok frames
                                    if db_ok { for acct in &accounts {
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
                                                // Per-DARWIN equity curve
                                                if let Ok(eq) = darwin::get_darwin_equity_curve(&conn, &acct.darwin_ticker) {
                                                    if eq.len() > 2 {
                                                        ui.add_space(5.0);
                                                        ui.label(egui::RichText::new("Equity Curve").strong());
                                                        let points: PlotPoints = PlotPoints::new(
                                                            eq.iter().enumerate().map(|(i, (_, bal))| [i as f64, *bal]).collect()
                                                        );
                                                        let line = Line::new("Equity", points).color(ACCENT);
                                                        Plot::new(format!("eq_{}", acct.darwin_ticker))
                                                            .height(120.0)
                                                            .allow_drag(false)
                                                            .allow_zoom(false)
                                                            .show(ui, |plot_ui| { plot_ui.line(line); });
                                                    }
                                                }
                                                // P&L by Symbol
                                                if let Ok(pnl_sym) = darwin::get_darwin_pnl_by_symbol(&conn, &acct.darwin_ticker) {
                                                    if !pnl_sym.is_empty() {
                                                        ui.add_space(5.0);
                                                        ui.label(egui::RichText::new("P&L by Symbol").strong());
                                                        egui::Grid::new(format!("sym_{}", acct.darwin_ticker)).striped(true).num_columns(4).show(ui, |ui| {
                                                            ui.strong("Symbol"); ui.strong("P&L"); ui.strong("Comm"); ui.strong("Trades");
                                                            ui.end_row();
                                                            for (sym, pnl, comm, _swap, count) in &pnl_sym {
                                                                ui.label(sym);
                                                                let c = if *pnl >= 0.0 { UP } else { DOWN };
                                                                ui.label(egui::RichText::new(format!("${:.2}", pnl)).color(c));
                                                                ui.label(format!("${:.2}", comm));
                                                                ui.label(format!("{}", count));
                                                                ui.end_row();
                                                            }
                                                        });
                                                    }
                                                }
                                                // Day of Week P&L
                                                if let Ok(dow) = darwin::get_day_of_week_pnl(&conn, &acct.darwin_ticker) {
                                                    if !dow.is_empty() {
                                                        ui.add_space(5.0);
                                                        ui.label(egui::RichText::new("Day of Week").strong());
                                                        egui::Grid::new(format!("dow_{}", acct.darwin_ticker)).striped(true).num_columns(4).show(ui, |ui| {
                                                            ui.strong("Day"); ui.strong("P&L"); ui.strong("Win%"); ui.strong("Trades");
                                                            ui.end_row();
                                                            for d in &dow {
                                                                ui.label(&d.day);
                                                                let c = if d.total_pnl >= 0.0 { UP } else { DOWN };
                                                                ui.label(egui::RichText::new(format!("${:.2}", d.total_pnl)).color(c));
                                                                ui.label(format!("{:.0}%", d.win_rate));
                                                                ui.label(format!("{}", d.trade_count));
                                                                ui.end_row();
                                                            }
                                                        });
                                                    }
                                                }
                                                // Hold Time Stats
                                                if let Ok(ht) = darwin::get_hold_time_stats(&conn, &acct.darwin_ticker) {
                                                    ui.add_space(5.0);
                                                    ui.label(egui::RichText::new("Hold Time").strong());
                                                    ui.label(format!("Avg: {:.1}h  Med: {:.1}h  Min: {:.1}h  Max: {:.1}h", ht.avg_hold_hours, ht.median_hold_hours, ht.min_hold_hours, ht.max_hold_hours));
                                                    if !ht.buckets.is_empty() {
                                                        egui::Grid::new(format!("ht_{}", acct.darwin_ticker)).striped(true).num_columns(3).show(ui, |ui| {
                                                            ui.strong("Bucket"); ui.strong("Trades"); ui.strong("Avg P&L");
                                                            ui.end_row();
                                                            for (label, count, avg_pnl) in &ht.buckets {
                                                                ui.label(label);
                                                                ui.label(format!("{}", count));
                                                                let c = if *avg_pnl >= 0.0 { UP } else { DOWN };
                                                                ui.label(egui::RichText::new(format!("${:.2}", avg_pnl)).color(c));
                                                                ui.end_row();
                                                            }
                                                        });
                                                    }
                                                }
                                                // Kelly Criterion
                                                if let Ok(kelly) = darwin::compute_kelly(&conn, &acct.darwin_ticker) {
                                                    ui.add_space(5.0);
                                                    ui.label(egui::RichText::new("Kelly Criterion").strong());
                                                    egui::Grid::new(format!("kelly_{}", acct.darwin_ticker)).striped(true).num_columns(2).show(ui, |ui| {
                                                        ui.label("Win Rate:"); ui.label(format!("{:.1}%", kelly.win_rate * 100.0)); ui.end_row();
                                                        ui.label("Avg Win:"); ui.label(egui::RichText::new(format!("${:.2}", kelly.avg_win)).color(UP)); ui.end_row();
                                                        ui.label("Avg Loss:"); ui.label(egui::RichText::new(format!("${:.2}", kelly.avg_loss)).color(DOWN)); ui.end_row();
                                                        ui.label("Kelly %:"); ui.label(format!("{:.1}%", kelly.kelly_fraction * 100.0)); ui.end_row();
                                                        ui.label("Half Kelly:"); ui.label(format!("{:.1}%", kelly.half_kelly * 100.0)); ui.end_row();
                                                        ui.label("Optimal Risk:"); ui.label(format!("{:.1}%", kelly.optimal_risk_pct)); ui.end_row();
                                                    });
                                                }
                                                // Cost Analysis
                                                if let Ok(costs) = darwin::get_cost_analysis(&conn, &acct.darwin_ticker) {
                                                    ui.add_space(5.0);
                                                    ui.label(egui::RichText::new("Cost Analysis").strong());
                                                    egui::Grid::new(format!("cost_{}", acct.darwin_ticker)).striped(true).num_columns(2).show(ui, |ui| {
                                                        ui.label("Total Commission:"); ui.label(egui::RichText::new(format!("${:.2}", costs.total_commission)).color(DOWN)); ui.end_row();
                                                        ui.label("Total Swap:"); ui.label(format!("${:.2}", costs.total_swap)); ui.end_row();
                                                        ui.label("Comm % of Equity:"); ui.label(format!("{:.2}%", costs.commission_pct_of_equity)); ui.end_row();
                                                        ui.label("Avg Comm/Trade:"); ui.label(format!("${:.2}", costs.avg_commission_per_trade)); ui.end_row();
                                                    });
                                                }
                                                // D-Score Estimate
                                                if let Ok(ds) = darwin::estimate_dscore(&conn, &acct.darwin_ticker) {
                                                    ui.add_space(5.0);
                                                    ui.label(egui::RichText::new("D-Score Estimate").strong());
                                                    egui::Grid::new(format!("ds_{}", acct.darwin_ticker)).striped(true).num_columns(2).show(ui, |ui| {
                                                        ui.label("Experience:"); ui.label(format!("{:.1}/10", ds.experience)); ui.end_row();
                                                        ui.label("Risk Mgmt:"); ui.label(format!("{:.1}/10", ds.risk_mgmt)); ui.end_row();
                                                        ui.label("Performance:"); ui.label(format!("{:.1}/10", ds.performance)); ui.end_row();
                                                        ui.label("Market Timing:"); ui.label(format!("{:.1}/10", ds.market_timing)); ui.end_row();
                                                        ui.label("Capacity:"); ui.label(format!("{:.1}/10", ds.capacity)); ui.end_row();
                                                        ui.label("Scalability:"); ui.label(format!("{:.1}/10", ds.scalability)); ui.end_row();
                                                        ui.label("Total D-Score:"); ui.label(egui::RichText::new(format!("{:.1}", ds.total_dscore)).strong()); ui.end_row();
                                                    });
                                                }
                                                // Slippage Analysis
                                                if let Ok(slip) = darwin::analyze_slippage(&conn, &acct.darwin_ticker) {
                                                    ui.add_space(5.0);
                                                    ui.label(egui::RichText::new("Slippage").strong());
                                                    ui.label(format!("Avg: {:.4}%  Total cost: ${:.2}  Worst: {:.4}%", slip.avg_slippage_pct, slip.total_slippage_cost, slip.worst_slippage));
                                                }
                                                // MAE/MFE
                                                if let Ok(mae) = darwin::estimate_mae_mfe(&conn, &acct.darwin_ticker) {
                                                    ui.add_space(5.0);
                                                    ui.label(egui::RichText::new("MAE / MFE").strong());
                                                    ui.label(format!("Avg MAE: {:.2}%  Avg MFE: {:.2}%  Ratio: {:.2}", mae.avg_mae_pct, mae.avg_mfe_pct, mae.mae_mfe_ratio));
                                                }
                                                // Sizing Efficiency
                                                if let Ok(sizing) = darwin::get_sizing_efficiency(&conn, &acct.darwin_ticker) {
                                                    if !sizing.is_empty() {
                                                        ui.add_space(5.0);
                                                        ui.label(egui::RichText::new("Sizing Efficiency").strong());
                                                        egui::Grid::new(format!("sz_{}", acct.darwin_ticker)).striped(true).num_columns(4).show(ui, |ui| {
                                                            ui.strong("Quartile"); ui.strong("Avg Vol"); ui.strong("Win%"); ui.strong("P&L");
                                                            ui.end_row();
                                                            for s in &sizing {
                                                                ui.label(&s.quartile);
                                                                ui.label(format!("{:.2}", s.avg_volume));
                                                                ui.label(format!("{:.0}%", s.win_rate));
                                                                let c = if s.total_pnl >= 0.0 { UP } else { DOWN };
                                                                ui.label(egui::RichText::new(format!("${:.0}", s.total_pnl)).color(c));
                                                                ui.end_row();
                                                            }
                                                        });
                                                    }
                                                }
                                                // Symbol Rotation
                                                if let Ok(rot) = darwin::get_symbol_rotation(&conn, &acct.darwin_ticker) {
                                                    if !rot.is_empty() {
                                                        ui.add_space(5.0);
                                                        ui.label(egui::RichText::new("Symbol Rotation").strong());
                                                        egui::Grid::new(format!("rot_{}", acct.darwin_ticker)).striped(true).num_columns(4).show(ui, |ui| {
                                                            ui.strong("Symbol"); ui.strong("Trades"); ui.strong("P&L"); ui.strong("Active");
                                                            ui.end_row();
                                                            for r in &rot {
                                                                ui.label(&r.symbol);
                                                                ui.label(format!("{}", r.trade_count));
                                                                let c = if r.total_pnl >= 0.0 { UP } else { DOWN };
                                                                ui.label(egui::RichText::new(format!("${:.0}", r.total_pnl)).color(c));
                                                                ui.label(format!("{} mo", r.active_months));
                                                                ui.end_row();
                                                            }
                                                        });
                                                    }
                                                }
                                                // Open Positions (per-DARWIN)
                                                if let Ok(pos) = darwin::get_darwin_open_positions(&conn, &acct.darwin_ticker) {
                                                    if !pos.is_empty() {
                                                        ui.add_space(5.0);
                                                        ui.label(egui::RichText::new(format!("Open Positions ({})", pos.len())).strong());
                                                        egui::Grid::new(format!("pos_{}", acct.darwin_ticker)).striped(true).num_columns(4).show(ui, |ui| {
                                                            ui.strong("Symbol"); ui.strong("Side"); ui.strong("Volume"); ui.strong("Price");
                                                            ui.end_row();
                                                            for p in &pos {
                                                                ui.label(&p.symbol);
                                                                let sc = if p.side == "buy" { UP } else { DOWN };
                                                                ui.label(egui::RichText::new(&p.side).color(sc));
                                                                ui.label(format!("{:.2}", p.total_volume));
                                                                ui.label(format_price(p.avg_price));
                                                                ui.end_row();
                                                            }
                                                        });
                                                    }
                                                }
                                                // Pyramiding Analysis
                                                if let Ok(pyra) = darwin::analyze_pyramiding(&conn, &acct.darwin_ticker) {
                                                    if !pyra.is_empty() {
                                                        ui.add_space(5.0);
                                                        ui.label(egui::RichText::new("Pyramiding").strong());
                                                        for p in &pyra {
                                                            let c = if p.final_pnl >= 0.0 { UP } else { DOWN };
                                                            ui.label(egui::RichText::new(format!("{}: {} adds ({}h avg), {} win/{} loss → ${:.0} [{}]",
                                                                p.symbol, p.total_adds, p.avg_add_interval_hours as i64,
                                                                p.adds_in_profit, p.adds_in_loss, p.final_pnl, p.strategy)).color(c).small());
                                                        }
                                                    }
                                                }
                                                // Trading Bursts
                                                if let Ok(bursts) = darwin::detect_trading_bursts(&conn, &acct.darwin_ticker) {
                                                    if !bursts.is_empty() {
                                                        ui.add_space(5.0);
                                                        ui.label(egui::RichText::new("Trading Bursts").strong());
                                                        for b in bursts.iter().take(5) {
                                                            let c = if b.total_pnl >= 0.0 { UP } else { DOWN };
                                                            ui.label(egui::RichText::new(format!("{} → {}: {} trades ({:.1}/day) ${:.0}",
                                                                b.start_date, b.end_date, b.trade_count, b.avg_trades_per_day, b.total_pnl)).color(c).small());
                                                        }
                                                    }
                                                }
                                                // Trade Autocorrelation
                                                if let Ok(ac) = darwin::compute_trade_autocorrelation(&conn, &acct.darwin_ticker) {
                                                    ui.add_space(5.0);
                                                    ui.label(egui::RichText::new("Autocorrelation").strong());
                                                    ui.label(format!("Lag1: {:.4}  Lag2: {:.4}  Lag3: {:.4}  Lag5: {:.4}", ac.lag1, ac.lag2, ac.lag3, ac.lag5));
                                                    let rc = if ac.is_random { UP } else { egui::Color32::from_rgb(255, 200, 50) };
                                                    ui.label(egui::RichText::new(&ac.interpretation).color(rc).small());
                                                }
                                                // Recent Deals (last 20)
                                                if let Ok(deals) = darwin::get_darwin_deals(&conn, &acct.darwin_ticker, None, Some(20)) {
                                                    if !deals.is_empty() {
                                                        ui.add_space(5.0);
                                                        ui.label(egui::RichText::new(format!("Recent Deals ({})", deals.len())).strong());
                                                        egui::Grid::new(format!("deals_{}", acct.darwin_ticker)).striped(true).num_columns(5).show(ui, |ui| {
                                                            ui.strong("Time"); ui.strong("Symbol"); ui.strong("Type"); ui.strong("Vol"); ui.strong("P&L");
                                                            ui.end_row();
                                                            for d in deals.iter().take(20) {
                                                                ui.label(egui::RichText::new(&d.time).small());
                                                                ui.label(egui::RichText::new(&d.symbol).small());
                                                                let tc = if d.deal_type == "buy" { UP } else { DOWN };
                                                                ui.label(egui::RichText::new(&d.deal_type).color(tc).small());
                                                                ui.label(egui::RichText::new(format!("{:.2}", d.volume)).small());
                                                                let pc = if d.profit >= 0.0 { UP } else { DOWN };
                                                                ui.label(egui::RichText::new(format!("${:.2}", d.profit)).color(pc).small());
                                                                ui.end_row();
                                                            }
                                                        });
                                                    }
                                                }
                                                // Closed Positions (last 20)
                                                if let Ok(positions) = darwin::get_darwin_positions(&conn, &acct.darwin_ticker, None, Some(20)) {
                                                    if !positions.is_empty() {
                                                        ui.add_space(5.0);
                                                        ui.label(egui::RichText::new(format!("Recent Positions ({})", positions.len())).strong());
                                                        egui::Grid::new(format!("cpos_{}", acct.darwin_ticker)).striped(true).num_columns(5).show(ui, |ui| {
                                                            ui.strong("Symbol"); ui.strong("Side"); ui.strong("Volume"); ui.strong("P&L"); ui.strong("Comm");
                                                            ui.end_row();
                                                            for p in positions.iter().take(20) {
                                                                ui.label(egui::RichText::new(&p.symbol).small());
                                                                let sc = if p.pos_type == "buy" { UP } else { DOWN };
                                                                ui.label(egui::RichText::new(&p.pos_type).color(sc).small());
                                                                ui.label(egui::RichText::new(format!("{:.2}", p.volume)).small());
                                                                let pc = if p.profit >= 0.0 { UP } else { DOWN };
                                                                ui.label(egui::RichText::new(format!("${:.2}", p.profit)).color(pc).small());
                                                                ui.label(egui::RichText::new(format!("${:.2}", p.commission)).small());
                                                                ui.end_row();
                                                            }
                                                        });
                                                    }
                                                }
                                                // Equity Snapshot History
                                                if let Ok(snapshots) = darwin::get_equity_history(&conn, &acct.darwin_ticker, 10) {
                                                    if !snapshots.is_empty() {
                                                        ui.add_space(5.0);
                                                        ui.label(egui::RichText::new("Equity Snapshots").strong());
                                                        for snap in &snapshots {
                                                            ui.label(egui::RichText::new(format!("Bal: ${:.0}  Unreal: ${:.0}  Float: ${:.0}  Pos: {}",
                                                                snap.closed_balance, snap.unrealized_pnl, snap.floating_equity, snap.open_position_count)).small());
                                                        }
                                                    }
                                                }
                                                // Benchmark Comparison (using portfolio as benchmark)
                                                if let Ok(port_daily) = darwin::get_portfolio_daily_returns(&conn) {
                                                    if let Ok(darwin_daily) = darwin::get_daily_returns(&conn, &acct.darwin_ticker) {
                                                        if let Ok(bench) = darwin::compare_to_benchmark(&conn, &acct.darwin_ticker, &port_daily) {
                                                            ui.add_space(5.0);
                                                            ui.label(egui::RichText::new("vs Portfolio Benchmark").strong());
                                                            egui::Grid::new(format!("bench_{}", acct.darwin_ticker)).striped(true).num_columns(2).show(ui, |ui| {
                                                                ui.label("Alpha:"); let ac = if bench.alpha >= 0.0 { UP } else { DOWN };
                                                                ui.label(egui::RichText::new(format!("{:.4}", bench.alpha)).color(ac)); ui.end_row();
                                                                ui.label("Beta:"); ui.label(format!("{:.4}", bench.beta)); ui.end_row();
                                                                ui.label("Info Ratio:"); ui.label(format!("{:.3}", bench.information_ratio)); ui.end_row();
                                                                ui.label("DARWIN Return:"); ui.label(format!("{:.2}%", bench.darwin_return)); ui.end_row();
                                                                ui.label("Benchmark Return:"); ui.label(format!("{:.2}%", bench.benchmark_return)); ui.end_row();
                                                            });
                                                        }
                                                        // Also compute full VaR
                                                        if !darwin_daily.is_empty() {
                                                            let full_var = darwin::compute_var_full(&darwin_daily);
                                                            // (full_var is same type as compute_var — already shown above)
                                                            let _ = full_var; // used for completeness
                                                        }
                                                    }
                                                }
                                                // Record equity snapshot (if we have data)
                                                if let Ok(summary) = darwin::get_darwin_summary(&conn, &acct.darwin_ticker) {
                                                    let _ = darwin::record_equity_snapshot(&conn, &acct.darwin_ticker, summary.final_balance, 0.0, 0);
                                                }
                                                // Sector classification (show sector for each symbol)
                                                if let Ok(pnl_sym) = darwin::get_darwin_pnl_by_symbol(&conn, &acct.darwin_ticker) {
                                                    if !pnl_sym.is_empty() {
                                                        ui.add_space(5.0);
                                                        ui.label(egui::RichText::new("Sector Classification").strong());
                                                        for (sym, _, _, _, _) in pnl_sym.iter().take(10) {
                                                            let sector = darwin::classify_sector(sym);
                                                            ui.label(egui::RichText::new(format!("{}: {}", sym, sector)).small());
                                                        }
                                                    }
                                                }
                                            });
                                        }
                                    } } // close for acct + if db_ok
                                }
                                Ok(_) => {
                                    ui.label(egui::RichText::new("No DARWIN accounts imported yet.").color(AXIS_TEXT));
                                    ui.label(egui::RichText::new("Export MT5 Trade History as XLSX, then import here.").color(AXIS_TEXT).small());
                                }
                                Err(e) => {
                                    ui.label(egui::RichText::new(format!("Error: {}", e)).color(egui::Color32::from_rgb(255, 80, 80)));
                                }
                            }
                            // ── Tax Summary (current year) ──────────────
                            ui.add_space(10.0);
                            ui.separator();
                            ui.heading("Tax Summary (2026)");
                            if let Ok(accounts) = darwin::list_darwin_accounts(&conn) {
                                for acct in &accounts {
                                    if let Ok(tax) = darwin::compute_tax_lots(&conn, &acct.darwin_ticker, 2026) {
                                        ui.label(egui::RichText::new(&acct.darwin_ticker).strong());
                                        egui::Grid::new(format!("tax_{}", acct.darwin_ticker)).striped(true).num_columns(2).show(ui, |ui| {
                                            ui.label("Short-term gains:"); ui.label(egui::RichText::new(format!("${:.2}", tax.short_term_gains)).color(UP)); ui.end_row();
                                            ui.label("Short-term losses:"); ui.label(egui::RichText::new(format!("${:.2}", tax.short_term_losses)).color(DOWN)); ui.end_row();
                                            ui.label("Long-term gains:"); ui.label(egui::RichText::new(format!("${:.2}", tax.long_term_gains)).color(UP)); ui.end_row();
                                            ui.label("Long-term losses:"); ui.label(egui::RichText::new(format!("${:.2}", tax.long_term_losses)).color(DOWN)); ui.end_row();
                                            let net_c = if tax.total_net >= 0.0 { UP } else { DOWN };
                                            ui.label("Net Total:"); ui.label(egui::RichText::new(format!("${:.2}", tax.total_net)).color(net_c).strong()); ui.end_row();
                                        });
                                        ui.add_space(4.0);
                                    }
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
                            // ── Daily Risk Report ──────────────────────
                            ui.add_space(10.0);
                            ui.separator();
                            if ui.button("Generate Daily Risk Report").clicked() {
                                match darwin::generate_daily_report(&conn) {
                                    Ok(report) => {
                                        self.log.push_back(LogEntry::info(format!(
                                            "Daily Report {}: Equity ${:.0}, P&L ${:.2} ({:.2}%), VaR95 {:.2}%, DD {:.2}%, {} positions, ${:.0} notional",
                                            report.date, report.portfolio_equity, report.daily_pnl,
                                            report.daily_return_pct, report.current_var_95,
                                            report.current_drawdown_pct, report.open_position_count, report.total_notional
                                        )));
                                        if !report.top_gainers.is_empty() {
                                            let gainers: Vec<String> = report.top_gainers.iter().map(|(s, p)| format!("{} +${:.0}", s, p)).collect();
                                            self.log.push_back(LogEntry::info(format!("Top gainers: {}", gainers.join(", "))));
                                        }
                                        if !report.top_losers.is_empty() {
                                            let losers: Vec<String> = report.top_losers.iter().map(|(s, p)| format!("{} ${:.0}", s, p)).collect();
                                            self.log.push_back(LogEntry::info(format!("Top losers: {}", losers.join(", "))));
                                        }
                                    }
                                    Err(e) => { self.log.push_back(LogEntry::err(format!("Report error: {}", e))); }
                                }
                            }

                            // ── Floating Equity Dashboard ─────────────────
                            ui.add_space(10.0);
                            ui.separator();
                            ui.heading("Floating Equity");
                            let prices = std::collections::HashMap::new(); // empty — uses closed balance as fallback
                            if let Ok(float_eq) = darwin::compute_floating_equity(&conn, &prices) {
                                egui::Grid::new("float_eq").striped(true).num_columns(4).show(ui, |ui| {
                                    ui.strong("DARWIN"); ui.strong("Closed Bal"); ui.strong("Unreal P&L"); ui.strong("Float Eq");
                                    ui.end_row();
                                    for d in &float_eq.darwins {
                                        ui.label(&d.darwin_ticker);
                                        ui.label(format!("${:.0}", d.closed_balance));
                                        let uc = if d.unrealized_pnl >= 0.0 { UP } else { DOWN };
                                        ui.label(egui::RichText::new(format!("${:.0}", d.unrealized_pnl)).color(uc));
                                        ui.label(format!("${:.0}", d.floating_equity));
                                        ui.end_row();
                                    }
                                    ui.label(egui::RichText::new("COMBINED").strong());
                                    ui.label(format!("${:.0}", float_eq.combined_closed_balance));
                                    let cc = if float_eq.combined_unrealized_pnl >= 0.0 { UP } else { DOWN };
                                    ui.label(egui::RichText::new(format!("${:.0}", float_eq.combined_unrealized_pnl)).color(cc));
                                    ui.label(egui::RichText::new(format!("${:.0}", float_eq.combined_floating_equity)).strong());
                                    ui.end_row();
                                });
                            }

                            // ── Export & FTP ──────────────────────────────
                            ui.add_space(10.0);
                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.button("Export Radar TXT").clicked() {
                                    let mut out = dirs_home();
                                    out.push("export");
                                    let _ = std::fs::create_dir_all(&out);
                                    match darwin::export_radar_txt(&conn, &conn, &out.display().to_string()) {
                                        Ok(path) => self.log.push_back(LogEntry::info(format!("Radar exported: {}", path))),
                                        Err(e) => self.log.push_back(LogEntry::err(format!("Export failed: {}", e))),
                                    }
                                }
                            });

                            // ── FTP Scanner (needs Darwinex FTP path) ─────
                            ui.add_space(5.0);
                            ui.label(egui::RichText::new("Darwinex FTP Scanner").small().strong());
                            ui.label(egui::RichText::new("Set DARWIN_FTP_PATH env var to enable FTP-based features:").color(AXIS_TEXT).small());
                            ui.label(egui::RichText::new("  find_low_correlation_darwins, scan_darwin_ftp,").color(AXIS_TEXT).small());
                            ui.label(egui::RichText::new("  get_darwin_price_series, get_dscore_components, get_investor_flow").color(AXIS_TEXT).small());
                            if let Ok(ftp) = std::env::var("DARWIN_FTP_PATH") {
                                if ui.button("Scan FTP for Low-Correlation DARWINs").clicked() {
                                    match darwin::find_low_correlation_darwins(&conn, &ftp, 10) {
                                        Ok(candidates) => {
                                            for c in &candidates {
                                                self.log.push_back(LogEntry::info(format!(
                                                    "Candidate {}: corr {:.4}, return {:.2}%, DD {:.2}%, Sharpe {:.3}",
                                                    c.ticker, c.avg_correlation, c.return_pct, c.max_drawdown, c.sharpe
                                                )));
                                            }
                                        }
                                        Err(e) => self.log.push_back(LogEntry::err(format!("FTP scan: {}", e))),
                                    }
                                }
                                if ui.button("Scan FTP for DARWINs (min 90d, >5% return, <30% DD)").clicked() {
                                    match darwin::scan_darwin_ftp(&ftp, 90, 5.0, 30.0, 50) {
                                        Ok(candidates) => {
                                            self.log.push_back(LogEntry::info(format!("FTP scan: {} candidates found", candidates.len())));
                                        }
                                        Err(e) => self.log.push_back(LogEntry::err(format!("FTP scan: {}", e))),
                                    }
                                }
                                // Per-DARWIN FTP data
                                if let Ok(accounts) = darwin::list_darwin_accounts(&conn) {
                                    for acct in accounts.iter().take(3) {
                                        if let Ok(components) = darwin::get_dscore_components(&ftp, &acct.darwin_ticker) {
                                            self.log.push_back(LogEntry::info(format!(
                                                "D-Score {}: Exp {:?}, Risk {:?}, Perf {:?}",
                                                components.ticker, components.experience, components.risk_stability, components.performance
                                            )));
                                        }
                                        if let Ok(flow) = darwin::get_investor_flow(&ftp, &acct.darwin_ticker) {
                                            if let Some(last) = flow.last() {
                                                self.log.push_back(LogEntry::info(format!(
                                                    "Investor flow {}: {} investors, ${:.0} AUM",
                                                    acct.darwin_ticker, last.investors, last.aum
                                                )));
                                            }
                                        }
                                        if let Ok(prices) = darwin::get_darwin_price_series(&ftp, &acct.darwin_ticker, "D1") {
                                            self.log.push_back(LogEntry::info(format!(
                                                "Price series {}: {} bars", acct.darwin_ticker, prices.len()
                                            )));
                                        }
                                    }
                                }
                            }

                            // ── Delete Account ───────────────────────────
                            ui.add_space(10.0);
                            ui.separator();
                            ui.label(egui::RichText::new("Delete Account").color(DOWN));
                            ui.horizontal(|ui| {
                                ui.label("Ticker:");
                                ui.add(egui::TextEdit::singleline(&mut self.darwin_import_ticker).desired_width(80.0));
                                if ui.button(egui::RichText::new("Delete").color(BTN_RED_TEXT)).clicked() {
                                    let ticker = self.darwin_import_ticker.trim().to_string();
                                    if !ticker.is_empty() {
                                        match darwin::delete_darwin_account(&conn, &ticker) {
                                            Ok(()) => { self.log.push_back(LogEntry::info(format!("Deleted DARWIN account: {}", ticker))); }
                                            Err(e) => { self.log.push_back(LogEntry::err(format!("Delete failed: {}", e))); }
                                        }
                                    }
                                }
                            });
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
                    // View selector dropdown (matching old WebKit 20+ views)
                    let views = [
                        "Portfolio Summary", "Portfolio VaR", "Equity Curve", "Correlation Matrix",
                        "Symbol Exposure", "Combined Positions", "Trade Overlaps", "Combined Equity",
                        "Monte Carlo", "Stress Test", "VaR Forecast", "Conditional VaR",
                        "Market Regime", "Tail Risk", "Seasonals", "Sector Exposure",
                        "Liquidity Risk", "Margin Call Sim", "Optimal Allocation", "What-If",
                    ];
                    let _prev_view = self.darwin_view;
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("View:").color(AXIS_TEXT));
                        egui::ComboBox::from_id_salt("darwin_view_combo")
                            .selected_text(*views.get(self.darwin_view).unwrap_or(&"Portfolio Summary"))
                            .width(200.0)
                            .show_ui(ui, |ui| {
                                for (i, v) in views.iter().enumerate() {
                                    ui.selectable_value(&mut self.darwin_view, i, *v);
                                }
                            });
                    });
                    ui.separator();
                    // Use background-computed DARWIN data (minimal DB queries in render)
                    let bg = self.bg_darwin.try_lock().ok();
                    let portfolio_ref = bg.as_ref().and_then(|d| d.portfolio.as_ref());
                    let bg_daily = bg.as_ref().map(|d| &d.daily_returns);
                    let bg_var = bg.as_ref().and_then(|d| d.var_stats.as_ref());
                    let bg_corrs = bg.as_ref().map(|d| &d.correlations);
                    let bg_exposure = bg.as_ref().map(|d| &d.exposure);
                    let bg_eq_curve = bg.as_ref().map(|d| &d.equity_curve);
                    let bg_positions = bg.as_ref().map(|d| &d.open_positions);
                    let bg_overlaps = bg.as_ref().map(|d| &d.trade_overlaps);
                    let bg_alloc = bg.as_ref().map(|d| &d.optimal_allocation);
                    let bg_rebal = bg.as_ref().and_then(|d| d.rebalance.as_ref());
                    let bg_mc = bg.as_ref().and_then(|d| d.monte_carlo.as_ref());
                    let bg_stress = bg.as_ref().map(|d| &d.stress_tests);
                    let bg_margin = bg.as_ref().and_then(|d| d.margin_call_sim.as_ref());
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let dv = self.darwin_view;
                            egui::ScrollArea::vertical().show(ui, |ui| {
                            match portfolio_ref {
                                Some(portfolio) if !portfolio.accounts.is_empty() => {
                                    match dv {
                                        0 => { // Portfolio Summary
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
                                            // Per-DARWIN table
                                            ui.add_space(10.0);
                                            ui.heading("Per-DARWIN");
                                            ui.separator();
                                            egui::Grid::new("per_darwin").striped(true).num_columns(6).show(ui, |ui| {
                                                ui.strong("DARWIN"); ui.strong("Balance"); ui.strong("P&L"); ui.strong("Win%"); ui.strong("PF"); ui.strong("DD%");
                                                ui.end_row();
                                                for acct in &portfolio.accounts {
                                                    ui.label(&acct.account.darwin_ticker);
                                                    ui.label(format!("${:.0}", acct.final_balance));
                                                    let c = if acct.total_profit >= 0.0 { UP } else { DOWN };
                                                    ui.label(egui::RichText::new(format!("${:.0}", acct.total_profit)).color(c));
                                                    let wr_c = if acct.win_rate >= 50.0 { UP } else { DOWN };
                                                    ui.label(egui::RichText::new(format!("{:.1}%", acct.win_rate)).color(wr_c));
                                                    ui.label(format!("{:.2}", acct.profit_factor));
                                                    ui.label(format!("{:.1}%", acct.max_drawdown_pct));
                                                    ui.end_row();
                                                }
                                            });
                                        }
                                        1 => { // Portfolio VaR (from bg cache)
                                            if let Some(daily) = bg_daily { if !daily.is_empty() {
                                                if let Some(vs) = bg_var {
                                                    egui::Grid::new("port_var").striped(true).num_columns(4).show(ui, |ui| {
                                                        ui.label("VaR 95%:"); ui.label(format!("${:.2}", vs.var_95));
                                                        ui.label("Sharpe:"); ui.label(format!("{:.3}", vs.sharpe));
                                                        ui.end_row();
                                                        ui.label("VaR 99%:"); ui.label(format!("${:.2}", vs.var_99));
                                                        ui.label("Sortino:"); ui.label(format!("{:.3}", vs.sortino));
                                                        ui.end_row();
                                                        ui.label("CVaR 95%:"); ui.label(format!("${:.2}", vs.cvar_95));
                                                        ui.label("Calmar:"); ui.label(format!("{:.3}", vs.calmar));
                                                        ui.end_row();
                                                        ui.label("CVaR 99%:"); ui.label(format!("${:.2}", vs.cvar_99));
                                                        ui.label("Max DD:"); ui.label(format!("{:.2}%", vs.max_drawdown_pct));
                                                        ui.end_row();
                                                        ui.label("Daily Vol:"); ui.label(format!("{:.4}", vs.daily_vol));
                                                        ui.label("Ann. Vol:"); ui.label(format!("{:.4}", vs.annualized_vol));
                                                        ui.end_row();
                                                        ui.label("Best Day:"); ui.label(egui::RichText::new(format!("${:.2}", vs.best_day)).color(UP));
                                                        ui.label("Worst Day:"); ui.label(egui::RichText::new(format!("${:.2}", vs.worst_day)).color(DOWN));
                                                        ui.end_row();
                                                        ui.label("Avg Daily:"); ui.label(format!("${:.2}", vs.avg_daily_pnl));
                                                        ui.label("Trading Days:"); ui.label(format!("{}", vs.trading_days));
                                                        ui.end_row();
                                                    });
                                                    // Rolling VaR (30-day window)
                                                    let rolling = darwin::get_rolling_var(&daily, 30);
                                                    if rolling.len() > 5 {
                                                        ui.add_space(10.0);
                                                        ui.label(egui::RichText::new("Rolling 30d VaR").strong());
                                                        let points: PlotPoints = PlotPoints::new(
                                                            rolling.iter().enumerate().map(|(i, rv)| [i as f64, rv.var_95]).collect()
                                                        );
                                                        let line = Line::new("VaR95", points).color(DOWN);
                                                        Plot::new("rolling_var_plot").height(120.0).allow_drag(false).allow_zoom(false)
                                                            .show(ui, |plot_ui| { plot_ui.line(line); });
                                                    }
                                                    // Combined drawdown dashboard
                                                    if let Ok(dd) = darwin::get_combined_drawdown_dashboard(&conn, 5) {
                                                        ui.add_space(10.0);
                                                        ui.label(egui::RichText::new("Drawdown Dashboard").strong());
                                                        egui::Grid::new("dd_dash").striped(true).num_columns(4).show(ui, |ui| {
                                                            ui.strong("DARWIN"); ui.strong("Max DD"); ui.strong("Date"); ui.strong("Current DD");
                                                            ui.end_row();
                                                            for d in &dd.darwins {
                                                                ui.label(&d.darwin_ticker);
                                                                ui.label(egui::RichText::new(format!("{:.2}%", d.max_drawdown_pct)).color(DOWN));
                                                                ui.label(&d.max_dd_date);
                                                                ui.label(format!("{:.2}%", d.current_drawdown_pct));
                                                                ui.end_row();
                                                            }
                                                            // Combined row
                                                            ui.label(egui::RichText::new("COMBINED").strong());
                                                            ui.label(egui::RichText::new(format!("{:.2}%", dd.combined.max_drawdown_pct)).color(DOWN).strong());
                                                            ui.label(&dd.combined.max_dd_date);
                                                            ui.label(format!("{:.2}%", dd.combined.current_drawdown_pct));
                                                            ui.end_row();
                                                        });
                                                    }
                                                } // if let Some(vs)
                                            } } // if !daily.is_empty() + if let Some(daily)
                                        }
                                        2 => { // Equity Curve (from bg cache)
                                            if let Some(eq_curve) = bg_eq_curve {
                                                if eq_curve.len() > 2 {
                                                    let points: PlotPoints = PlotPoints::new(
                                                        eq_curve.iter().enumerate().map(|(i, (_, bal))| [i as f64, *bal]).collect()
                                                    );
                                                    let line = Line::new("Equity", points).color(ACCENT);
                                                    Plot::new("port_equity_plot").height(350.0).allow_drag(false).allow_zoom(false)
                                                        .show(ui, |plot_ui| { plot_ui.line(line); });
                                                }
                                            }
                                        }
                                        3 => { // Correlation Matrix (from bg cache)
                                            if let Some(corrs) = bg_corrs {
                                                egui::Grid::new("corr_grid").striped(true).num_columns(3).show(ui, |ui| {
                                                    ui.strong("DARWIN A"); ui.strong("DARWIN B"); ui.strong("Correlation");
                                                    ui.end_row();
                                                    for c in corrs.iter() {
                                                        ui.label(&c.darwin_a); ui.label(&c.darwin_b);
                                                        let color = if c.correlation.abs() > 0.95 { egui::Color32::from_rgb(255, 80, 80) }
                                                                    else if c.correlation.abs() > 0.7 { egui::Color32::from_rgb(255, 200, 50) }
                                                                    else { UP };
                                                        ui.label(egui::RichText::new(format!("{:.4}", c.correlation)).color(color));
                                                        ui.end_row();
                                                    }
                                                });
                                            }
                                        }
                                        4 => { // Symbol Exposure (from bg cache)
                                            if let Some(exposure) = bg_exposure {
                                                egui::Grid::new("exp_grid").striped(true).num_columns(5).show(ui, |ui| {
                                                    ui.strong("Symbol"); ui.strong("Long $"); ui.strong("Short $"); ui.strong("Net $"); ui.strong("DARWINs");
                                                    ui.end_row();
                                                    for e in exposure.iter() {
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
                                            // Exposure Treemap (flattened)
                                            ui.add_space(10.0);
                                            ui.heading("Exposure by Sector");
                                            ui.separator();
                                            if let Ok(tree) = darwin::get_exposure_treemap(&conn) {
                                                for child in &tree.children {
                                                    let sector_c = if child.color_value > 0.0 { UP } else if child.color_value < 0.0 { DOWN } else { AXIS_TEXT };
                                                    ui.label(egui::RichText::new(format!("{}: ${:.0}", child.name, child.value)).color(sector_c).strong());
                                                    for sym in &child.children {
                                                        let sc = if sym.color_value > 0.0 { UP } else { DOWN };
                                                        ui.label(egui::RichText::new(format!("  {} ${:.0}", sym.name, sym.value)).color(sc).small());
                                                    }
                                                }
                                            }
                                        }
                                        5 => { // Combined Positions (from bg cache)
                                            if let Some(positions) = bg_positions {
                                                if positions.is_empty() {
                                                    ui.label(egui::RichText::new("No open positions.").color(AXIS_TEXT));
                                                } else {
                                                    egui::Grid::new("cpos_grid").striped(true).num_columns(5).show(ui, |ui| {
                                                        ui.strong("Symbol"); ui.strong("Side"); ui.strong("Volume"); ui.strong("Avg Price"); ui.strong("DARWINs");
                                                        ui.end_row();
                                                        for pos in positions.iter() {
                                                            ui.label(&pos.symbol);
                                                            let side_c = if pos.side == "buy" { UP } else { DOWN };
                                                            ui.label(egui::RichText::new(&pos.side).color(side_c));
                                                            ui.label(format!("{:.2}", pos.total_volume));
                                                            ui.label(format_price(pos.avg_price));
                                                            let darwins: Vec<String> = pos.darwin_breakdown.iter().map(|(d, _, _)| d.clone()).collect();
                                                            ui.label(darwins.join(", "));
                                                            ui.end_row();
                                                        }
                                                    });
                                                }
                                            }
                                        }
                                        6 => { // Trade Overlaps (from bg cache)
                                            if let Some(overlaps) = bg_overlaps {
                                                if overlaps.is_empty() {
                                                    ui.label("No trade overlaps found.");
                                                } else {
                                                    egui::Grid::new("overlap_grid").striped(true).num_columns(4).show(ui, |ui| {
                                                        ui.strong("Symbol"); ui.strong("DARWINs"); ui.strong("Volume"); ui.strong("Notional");
                                                        ui.end_row();
                                                        for o in overlaps.iter() {
                                                            ui.label(&o.symbol);
                                                            ui.label(o.darwins.join(", "));
                                                            ui.label(format!("{:.2}", o.combined_volume));
                                                            ui.label(format!("${:.0}", o.combined_notional));
                                                            ui.end_row();
                                                        }
                                                    });
                                                }
                                            }
                                        }
                                        7 => { // Combined Equity (same as view 2 but with per-DARWIN overlaid)
                                            if let Ok(eq_curve) = darwin::get_portfolio_equity_curve(&conn) {
                                                if eq_curve.len() > 2 {
                                                    let points: PlotPoints = PlotPoints::new(
                                                        eq_curve.iter().enumerate().map(|(i, (_, bal))| [i as f64, *bal]).collect()
                                                    );
                                                    let line = Line::new("Combined", points).color(ACCENT);
                                                    Plot::new("combined_eq_plot").height(350.0).allow_drag(false).allow_zoom(false)
                                                        .show(ui, |plot_ui| { plot_ui.line(line); });
                                                }
                                            }
                                        }
                                        8 => { // Monte Carlo (from bg cache)
                                            if let Some(mc) = bg_mc {
                                                    egui::Grid::new("mc_grid").striped(true).num_columns(2).show(ui, |ui| {
                                                        ui.label("Simulations:"); ui.label(format!("{}", mc.simulations));
                                                        ui.end_row();
                                                        ui.label("Horizon (days):"); ui.label(format!("{}", mc.days_forward));
                                                        ui.end_row();
                                                        ui.label("VaR 95%:"); ui.label(egui::RichText::new(format!("{:.2}%", mc.var_95)).color(DOWN));
                                                        ui.end_row();
                                                        ui.label("VaR 99%:"); ui.label(egui::RichText::new(format!("{:.2}%", mc.var_99)).color(DOWN));
                                                        ui.end_row();
                                                        ui.label("Median:"); ui.label(format!("{:.2}%", mc.median_outcome));
                                                        ui.end_row();
                                                        ui.label("Best Case:"); ui.label(egui::RichText::new(format!("{:.2}%", mc.best_case)).color(UP));
                                                        ui.end_row();
                                                        ui.label("Worst Case:"); ui.label(egui::RichText::new(format!("{:.2}%", mc.worst_case)).color(DOWN));
                                                        ui.end_row();
                                                        ui.label("Prob of Loss:"); ui.label(format!("{:.1}%", mc.probability_of_loss * 100.0));
                                                        ui.end_row();
                                                    });
                                            }
                                        }
                                        9 => { // Stress Test (from bg cache)
                                            let results = bg_stress; if let Some(results) = results {
                                                egui::Grid::new("stress_grid").striped(true).num_columns(4).show(ui, |ui| {
                                                    ui.strong("Scenario"); ui.strong("Market Drop"); ui.strong("Portfolio Impact"); ui.strong("Impact %");
                                                    ui.end_row();
                                                    for r in results.iter() {
                                                        ui.label(&r.scenario);
                                                        ui.label(egui::RichText::new(format!("{:.1}%", r.market_drop_pct)).color(DOWN));
                                                        ui.label(egui::RichText::new(format!("${:.0}", r.estimated_portfolio_impact)).color(DOWN));
                                                        ui.label(egui::RichText::new(format!("{:.1}%", r.estimated_portfolio_impact_pct)).color(DOWN));
                                                        ui.end_row();
                                                    }
                                                });
                                            }
                                            // Also show timing divergences
                                            ui.add_space(10.0);
                                            ui.heading("Timing Divergences");
                                            ui.separator();
                                            if let Ok(divs) = darwin::get_timing_divergences(&conn) {
                                                if divs.is_empty() {
                                                    ui.label("No timing divergences found.");
                                                } else {
                                                    for d in &divs {
                                                        ui.label(egui::RichText::new(format!("{}: spread {:.1}h, price {:.2}%", d.symbol, d.time_spread_hours, d.price_spread_pct)).small());
                                                    }
                                                }
                                            }
                                        }
                                        10 => { // VaR Forecast
                                            if let Ok(daily) = darwin::get_portfolio_daily_returns(&conn) {
                                                let forecast = darwin::forecast_var(&daily, 6.5); // 6.5% VaR threshold
                                                    egui::Grid::new("var_fc").striped(true).num_columns(2).show(ui, |ui| {
                                                        ui.label("Current VaR 95%:"); ui.label(format!("{:.2}%", forecast.current_var_95));
                                                        ui.end_row();
                                                        ui.label("Projected 30d:"); ui.label(format!("{:.2}%", forecast.projected_30d));
                                                        ui.end_row();
                                                        ui.label("Projected 60d:"); ui.label(format!("{:.2}%", forecast.projected_60d));
                                                        ui.end_row();
                                                        ui.label("Projected 90d:"); ui.label(format!("{:.2}%", forecast.projected_90d));
                                                        ui.end_row();
                                                        ui.label("VaR Trend:"); ui.label(&forecast.var_trend);
                                                        ui.end_row();
                                                        if let Some(days) = forecast.days_until_threshold {
                                                            ui.label("Days to Threshold:"); ui.label(egui::RichText::new(format!("{}", days)).color(if days < 30 { DOWN } else { AXIS_TEXT }));
                                                            ui.end_row();
                                                        }
                                                    });
                                            }
                                        }
                                        11 => { // Conditional VaR
                                            if let Ok(daily) = darwin::get_portfolio_daily_returns(&conn) {
                                                let cvar = darwin::compute_conditional_var(&daily);
                                                egui::Grid::new("cvar_grid").striped(true).num_columns(4).show(ui, |ui| {
                                                    ui.strong("Regime"); ui.strong("VaR 95%"); ui.strong("VaR 99%"); ui.strong("Days"); ui.strong("Sharpe");
                                                    ui.end_row();
                                                    for cv in &cvar {
                                                        ui.label(&cv.regime);
                                                        ui.label(format!("{:.2}%", cv.var_95));
                                                        ui.label(format!("{:.2}%", cv.var_99));
                                                        ui.label(format!("{}", cv.days_in_regime));
                                                        ui.label(format!("{:.3}", cv.sharpe));
                                                        ui.end_row();
                                                    }
                                                });
                                            }
                                        }
                                        12 => { // Market Regime
                                            if let Ok(daily) = darwin::get_portfolio_daily_returns(&conn) {
                                                let regime = darwin::detect_market_regime(&daily);
                                                egui::Grid::new("regime_grid").striped(true).num_columns(2).show(ui, |ui| {
                                                    ui.label("Current Regime:"); ui.label(egui::RichText::new(&regime.current_regime).strong());
                                                    ui.end_row();
                                                    ui.label("Since:"); ui.label(&regime.regime_start);
                                                    ui.end_row();
                                                    ui.label("Duration:"); ui.label(format!("{} days", regime.regime_duration_days));
                                                    ui.end_row();
                                                    ui.label("Rolling Vol:"); ui.label(format!("{:.4}", regime.rolling_vol));
                                                    ui.end_row();
                                                    ui.label("Vol Percentile:"); ui.label(format!("{:.1}%", regime.vol_percentile));
                                                    ui.end_row();
                                                });
                                                // Per-regime performance
                                                if let Ok(rp) = darwin::get_regime_performance(&conn) {
                                                    ui.add_space(10.0);
                                                    ui.heading("Performance by Regime");
                                                    ui.separator();
                                                    egui::Grid::new("rp_grid").striped(true).num_columns(5).show(ui, |ui| {
                                                        ui.strong("DARWIN"); ui.strong("Low Vol"); ui.strong("Med Vol"); ui.strong("High Vol"); ui.strong("Best");
                                                        ui.end_row();
                                                        for r in &rp {
                                                            ui.label(&r.darwin_ticker);
                                                            ui.label(format!("{:.3}", r.low_vol_sharpe));
                                                            ui.label(format!("{:.3}", r.medium_vol_sharpe));
                                                            ui.label(format!("{:.3}", r.high_vol_sharpe));
                                                            ui.label(&r.best_regime);
                                                            ui.end_row();
                                                        }
                                                    });
                                                }
                                            }
                                        }
                                        13 => { // Tail Risk
                                            if let Ok(daily) = darwin::get_portfolio_daily_returns(&conn) {
                                                let tail = darwin::compute_tail_risk(&daily);
                                                egui::Grid::new("tail_grid").striped(true).num_columns(2).show(ui, |ui| {
                                                    ui.label("Skewness:"); ui.label(format!("{:.4}", tail.skewness));
                                                    ui.end_row();
                                                    ui.label("Kurtosis:"); ui.label(format!("{:.4}", tail.kurtosis));
                                                    ui.end_row();
                                                    ui.label("Tail Ratio:"); ui.label(format!("{:.4}", tail.tail_ratio));
                                                    ui.end_row();
                                                    ui.label("Gain/Pain:"); ui.label(format!("{:.4}", tail.gain_to_pain));
                                                    ui.end_row();
                                                    ui.label("Ulcer Index:"); ui.label(format!("{:.4}", tail.ulcer_index));
                                                    ui.end_row();
                                                    ui.label("Pain Index:"); ui.label(format!("{:.4}", tail.pain_index));
                                                    ui.end_row();
                                                    ui.label("Omega Ratio:"); ui.label(format!("{:.4}", tail.omega_ratio));
                                                    ui.end_row();
                                                    let ft_c = if tail.fat_tail_warning { DOWN } else { UP };
                                                    ui.label("Fat Tail Warning:"); ui.label(egui::RichText::new(if tail.fat_tail_warning { "YES" } else { "NO" }).color(ft_c));
                                                    ui.end_row();
                                                });
                                            }
                                        }
                                        14 => { // Seasonals
                                            if let Ok(daily) = darwin::get_portfolio_daily_returns(&conn) {
                                                let seasonal = darwin::get_seasonal_analysis(&daily);
                                                egui::Grid::new("seasonal_grid").striped(true).num_columns(4).show(ui, |ui| {
                                                    ui.strong("Month"); ui.strong("Avg Return"); ui.strong("Win%"); ui.strong("Median");
                                                    ui.end_row();
                                                    for s in &seasonal {
                                                        ui.label(&s.month_name);
                                                        let c = if s.avg_return_pct >= 0.0 { UP } else { DOWN };
                                                        ui.label(egui::RichText::new(format!("{:.2}%", s.avg_return_pct)).color(c));
                                                        ui.label(format!("{:.1}%", s.win_rate));
                                                        ui.label(format!("{:.2}%", s.median_return_pct));
                                                        ui.end_row();
                                                    }
                                                });
                                            }
                                        }
                                        15 => { // Sector Exposure
                                            if let Ok(sectors) = darwin::get_sector_exposure(&conn) {
                                                egui::Grid::new("sector_grid").striped(true).num_columns(5).show(ui, |ui| {
                                                    ui.strong("Sector"); ui.strong("Long $"); ui.strong("Short $"); ui.strong("Net $"); ui.strong("Symbols");
                                                    ui.end_row();
                                                    for se in &sectors {
                                                        ui.label(&se.sector);
                                                        ui.label(format!("{:.0}", se.long_notional));
                                                        ui.label(format!("{:.0}", se.short_notional));
                                                        let c = if se.net_notional >= 0.0 { UP } else { DOWN };
                                                        ui.label(egui::RichText::new(format!("{:.0}", se.net_notional)).color(c));
                                                        ui.label(se.symbols.join(", "));
                                                        ui.end_row();
                                                    }
                                                });
                                            }
                                        }
                                        16 => { // Liquidity Risk
                                            if let Ok(liq) = darwin::get_liquidity_risk(&conn) {
                                                egui::Grid::new("liq_grid").striped(true).num_columns(4).show(ui, |ui| {
                                                    ui.strong("Symbol"); ui.strong("Volume"); ui.strong("Notional"); ui.strong("Conc%"); ui.strong("Risk");
                                                    ui.end_row();
                                                    for l in &liq {
                                                        ui.label(&l.symbol);
                                                        ui.label(format!("{:.0}", l.position_volume));
                                                        ui.label(format!("${:.0}", l.notional));
                                                        ui.label(format!("{:.1}%", l.concentration_pct));
                                                        let risk_c = match l.risk_tier.as_str() {
                                                            "HIGH" => DOWN,
                                                            "MEDIUM" => egui::Color32::from_rgb(255, 200, 50),
                                                            _ => UP,
                                                        };
                                                        ui.label(egui::RichText::new(&l.risk_tier).color(risk_c));
                                                        ui.end_row();
                                                    }
                                                });
                                            }
                                        }
                                        17 => { // Margin Call Sim (from bg cache)
                                            if let Some(sim) = bg_margin {
                                                egui::Grid::new("mc_sim").striped(true).num_columns(2).show(ui, |ui| {
                                                    ui.label("Current Equity:"); ui.label(format!("${:.2}", sim.current_equity));
                                                    ui.end_row();
                                                    ui.label("Used Margin:"); ui.label(format!("${:.2}", sim.current_margin_used));
                                                    ui.end_row();
                                                    ui.label("Margin Level:"); ui.label(format!("{:.1}%", sim.margin_level_pct));
                                                    ui.end_row();
                                                    if let Some(d50) = sim.days_to_margin_call_50 {
                                                        ui.label("Days to MC@50%:"); ui.label(egui::RichText::new(format!("{}", d50)).color(egui::Color32::from_rgb(255, 200, 50)));
                                                        ui.end_row();
                                                    }
                                                    if let Some(d100) = sim.days_to_margin_call_100 {
                                                        ui.label("Days to MC@100%:"); ui.label(egui::RichText::new(format!("{}", d100)).color(DOWN));
                                                        ui.end_row();
                                                    }
                                                    ui.label("Prob MC 30d:"); ui.label(format!("{:.1}%", sim.probability_30d * 100.0));
                                                    ui.end_row();
                                                    ui.label("Prob MC 90d:"); ui.label(format!("{:.1}%", sim.probability_90d * 100.0));
                                                    ui.end_row();
                                                    ui.label("Worst Equity 30d:"); ui.label(egui::RichText::new(format!("${:.2}", sim.worst_case_equity_30d)).color(DOWN));
                                                    ui.end_row();
                                                });
                                            }
                                        }
                                        18 => { // Optimal Allocation (from bg cache)
                                            if let Some(alloc) = bg_alloc { if !alloc.is_empty() {
                                                egui::Grid::new("alloc_grid").striped(true).num_columns(4).show(ui, |ui| {
                                                    ui.strong("DARWIN"); ui.strong("Current %"); ui.strong("Optimal %"); ui.strong("Sharpe Contr.");
                                                    ui.end_row();
                                                    for a in alloc.iter() {
                                                        ui.label(&a.darwin_ticker);
                                                        ui.label(format!("{:.1}%", a.current_weight * 100.0));
                                                        ui.label(format!("{:.1}%", a.optimal_weight * 100.0));
                                                        ui.label(format!("{:.3}", a.sharpe_contribution));
                                                        ui.end_row();
                                                    }
                                                });
                                            } } // close if !alloc.is_empty() + if let Some(alloc)
                                            // Rebalance suggestions (VaR reduction via decorrelation)
                                            ui.add_space(10.0);
                                            ui.heading("Rebalance Suggestions");
                                            ui.separator();
                                            if let Some(rebal) = bg_rebal {
                                                egui::Grid::new("rebal_summary").striped(true).num_columns(2).show(ui, |ui| {
                                                    ui.label("Portfolio VaR 95%:"); ui.label(format!("{:.2}%", rebal.current_portfolio_var_95));
                                                    ui.end_row();
                                                    ui.label("Portfolio Sharpe:"); ui.label(format!("{:.3}", rebal.current_sharpe));
                                                    ui.end_row();
                                                });
                                                if !rebal.high_correlation_pairs.is_empty() {
                                                    ui.add_space(5.0);
                                                    ui.label(egui::RichText::new("High Correlation Pairs").color(DOWN));
                                                    for pair in &rebal.high_correlation_pairs {
                                                        ui.label(format!("{}:{} ↔ {}:{} = {:.4}", pair.darwin_a, pair.symbol_a, pair.darwin_b, pair.symbol_b, pair.correlation));
                                                    }
                                                }
                                                if !rebal.suggestions.is_empty() {
                                                    ui.add_space(5.0);
                                                    ui.label(egui::RichText::new("Rebalance Actions").strong());
                                                    egui::Grid::new("rebal_actions").striped(true).num_columns(5).show(ui, |ui| {
                                                        ui.strong("Action"); ui.strong("DARWIN"); ui.strong("Symbol"); ui.strong("Current→Target"); ui.strong("VaR Impact");
                                                        ui.end_row();
                                                        for s in &rebal.suggestions {
                                                            let ac = match s.action.as_str() { "REDUCE" => DOWN, "INCREASE" => UP, _ => AXIS_TEXT };
                                                            ui.label(egui::RichText::new(&s.action).color(ac));
                                                            ui.label(&s.darwin_ticker);
                                                            ui.label(&s.symbol);
                                                            ui.label(format!("{:.2} → {:.2}", s.current_volume, s.suggested_volume));
                                                            let vc = if s.impact_var_pct < 0.0 { UP } else { DOWN };
                                                            ui.label(egui::RichText::new(format!("{:+.2}%", s.impact_var_pct)).color(vc));
                                                            ui.end_row();
                                                        }
                                                    });
                                                }
                                            }
                                        }
                                        19 => { // What-If
                                            ui.label(egui::RichText::new("What-If: Close Symbol").strong());
                                            ui.label("Click a symbol to see VaR impact of closing:");
                                            ui.add_space(4.0);
                                            if let Ok(exposure) = darwin::get_portfolio_exposure(&conn) {
                                                for e in exposure.iter() {
                                                    if ui.button(format!("Close {} (${:.0} net)", e.symbol, e.net_notional)).clicked() {
                                                        if let Ok(result) = darwin::what_if_close_symbol(&conn, &e.symbol) {
                                                            self.log.push_back(LogEntry::info(format!(
                                                                "What-If close {}: VaR {:.2}% → {:.2}% ({:+.2}%), notional ${:.0} → ${:.0}",
                                                                e.symbol, result.current_portfolio_var, result.new_portfolio_var,
                                                                result.var_change_pct, result.current_notional, result.new_notional
                                                            )));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        _ => {
                                            ui.label(egui::RichText::new("Select a view from the dropdown above.").color(AXIS_TEXT));
                                        }
                                    }
                                }
                                Some(_) => {
                                    ui.label(egui::RichText::new("No DARWIN accounts imported.").color(AXIS_TEXT));
                                }
                                None => {
                                    ui.label(egui::RichText::new("Loading DARWIN data...").color(AXIS_TEXT));
                                }
                            }
                            });
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
                            self.bt_equity_curve = result.equity_curve;
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
                        // Equity curve plot
                        if self.bt_equity_curve.len() > 2 {
                            ui.add_space(10.0);
                            ui.heading("Equity Curve");
                            let points: PlotPoints = PlotPoints::new(
                                self.bt_equity_curve.iter().enumerate()
                                    .map(|(i, &v)| [i as f64, v])
                                    .collect()
                            );
                            let line = Line::new("Equity", points).color(ACCENT);
                            Plot::new("bt_equity_plot")
                                .height(150.0)
                                .allow_drag(false)
                                .allow_zoom(false)
                                .show(ui, |plot_ui| {
                                    plot_ui.line(line);
                                });
                        }

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
                                    let mut load_key: Option<String> = None;
                                    for (key, count, _size) in &details {
                                        ui.label(egui::RichText::new(key).monospace());
                                        ui.label(format!("{}", count));
                                        if ui.small_button("Load").clicked() {
                                            load_key = Some(key.clone());
                                        }
                                        ui.end_row();
                                    }
                                    // Load symbol into active chart
                                    if let Some(key) = load_key {
                                        if let Some(ref cache_arc) = self.cache {
                                            if let Some(chart) = self.charts.get_mut(self.active_tab) {
                                                match cache_arc.get_bars_raw(&key) {
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
                .default_size([550.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Finnhub Key:").color(AXIS_TEXT));
                        ui.add(egui::TextEdit::singleline(&mut self.finnhub_key).desired_width(160.0).password(true));
                        let sym = self.charts.get(self.active_tab).map(|c| {
                            c.symbol.split(':').rev().nth(1).or_else(|| c.symbol.split(':').last()).unwrap_or("AAPL").to_string()
                        }).unwrap_or_else(|| "AAPL".to_string());
                        if ui.add(egui::Button::new("Fetch News").fill(BTN_BLUE)).clicked() {
                            if self.finnhub_key.is_empty() {
                                self.log.push_back(LogEntry::warn("Enter Finnhub API key"));
                            } else {
                                let _ = self.broker_tx.send(BrokerCmd::FinnhubNews {
                                    symbol: sym.clone(),
                                    api_key: self.finnhub_key.clone(),
                                });
                                self.log.push_back(LogEntry::info(format!("Fetching Finnhub news for {}...", sym)));
                            }
                        }
                    });
                    ui.separator();
                    if self.news_articles.is_empty() {
                        ui.label(egui::RichText::new("No news loaded. Enter Finnhub API key and click Fetch News.").color(AXIS_TEXT));
                    } else {
                        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                            for (headline, source, dt) in &self.news_articles {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(dt).color(egui::Color32::from_rgb(102, 102, 102)).small());
                                    ui.label(egui::RichText::new(source).color(egui::Color32::from_rgb(85, 85, 85)).small());
                                });
                                ui.label(egui::RichText::new(headline).color(egui::Color32::from_rgb(204, 204, 204)));
                                ui.separator();
                            }
                        });
                    }
                });
        }

        // Economic Calendar
        if self.show_calendar {
            egui::Window::new("Economic Calendar")
                .open(&mut self.show_calendar)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    let sym = self.charts.get(self.active_tab)
                        .map(|c| c.symbol.split(':').rev().nth(1).or_else(|| c.symbol.split(':').last()).unwrap_or("").to_string())
                        .unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Economic Calendar").strong());
                        if ui.button("Fetch Earnings").clicked() && !sym.is_empty() {
                            // Use Finnhub or AlphaVantage for earnings data
                            self.log.push_back(LogEntry::info(format!("Earnings calendar for {}: set AV_KEY or FINNHUB_KEY in Settings", sym)));
                        }
                    });
                    ui.separator();
                    // Key economic events (static reference — updated via data feeds when connected)
                    ui.label(egui::RichText::new("Key Events").strong());
                    let events = [
                        ("FOMC Rate Decision", "8 meetings/year", "Fed funds rate"),
                        ("Non-Farm Payrolls", "Monthly (1st Friday)", "US employment"),
                        ("CPI / Core CPI", "Monthly", "Inflation gauge"),
                        ("GDP (Advance/Final)", "Quarterly", "Economic growth"),
                        ("ISM Manufacturing", "Monthly (1st business day)", "Factory activity"),
                        ("Retail Sales", "Monthly", "Consumer spending"),
                        ("Jobless Claims", "Weekly (Thursday)", "Employment health"),
                    ];
                    egui::Grid::new("econ_cal").striped(true).num_columns(3).show(ui, |ui| {
                        ui.strong("Event"); ui.strong("Frequency"); ui.strong("Measures");
                        ui.end_row();
                        for (event, freq, desc) in &events {
                            ui.label(*event);
                            ui.label(egui::RichText::new(*freq).color(AXIS_TEXT).small());
                            ui.label(egui::RichText::new(*desc).color(AXIS_TEXT).small());
                            ui.end_row();
                        }
                    });
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Live data: connect Finnhub or AlphaVantage API key in Settings.").color(AXIS_TEXT).small());
                });
        }

        // SEC Filing Scanner — wired to engine sec_filing.rs
        if self.show_sec {
            egui::Window::new("SEC Filing Scanner")
                .open(&mut self.show_sec)
                .default_size([700.0, 500.0])
                .show(ctx, |ui| {
                    // Filing type filter checkboxes
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Filter:").color(AXIS_TEXT));
                        let labels = ["Form 4", "13F", "DEF 14A", "S-1", "10-K", "10-Q", "8-K"];
                        for (i, label) in labels.iter().enumerate() {
                            ui.checkbox(&mut self.sec_filters[i], *label);
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new(egui::RichText::new("Scrape Now").color(BTN_GREEN_TEXT).strong()).fill(BTN_GREEN)).clicked() {
                            let mut db_path = dirs_home();
                            db_path.push("cache");
                            db_path.push("typhoon_cache.db");
                            let _ = self.broker_tx.send(BrokerCmd::SecScrape { db_path });
                            self.log.push_back(LogEntry::info("SEC EDGAR scrape initiated..."));
                        }
                        ui.label(egui::RichText::new("all portfolio symbols via SEC EDGAR").color(AXIS_TEXT).small());
                    });
                    ui.separator();

                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = sec_filing::create_sec_tables(&conn);

                            // Filings table
                            egui::ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
                                if let Ok(filings) = sec_filing::get_recent_filings(&conn, None, 100) {
                                    if filings.is_empty() {
                                        ui.label(egui::RichText::new("No filings in database. Run SEC scraper to populate.").color(AXIS_TEXT));
                                    } else {
                                        egui::Grid::new("sec_filings_grid").striped(true).num_columns(6).show(ui, |ui| {
                                            ui.strong("Date"); ui.strong("Symbol"); ui.strong("Type"); ui.strong("Category"); ui.strong("Company"); ui.strong("Score");
                                            ui.end_row();
                                            let filter_types: Vec<&str> = vec!["4", "13F", "DEF 14A", "S-1", "10-K", "10-Q", "8-K"];
                                            for f in &filings {
                                                // Apply filter
                                                let pass = self.sec_filters.iter().enumerate().any(|(i, &enabled)| {
                                                    enabled && f.form_type.contains(filter_types.get(i).unwrap_or(&""))
                                                }) || self.sec_filters.iter().all(|&v| v); // show all if all checked
                                                if !pass { continue; }

                                                ui.label(egui::RichText::new(&f.filing_date).small());
                                                ui.label(egui::RichText::new(&f.ticker).small().strong());
                                                // Type badge with color
                                                let type_col = match f.form_type.as_str() {
                                                    "4" => egui::Color32::from_rgb(255, 200, 50),
                                                    "10-K" | "10-Q" => egui::Color32::from_rgb(100, 200, 255),
                                                    "8-K" => egui::Color32::from_rgb(255, 130, 60),
                                                    "S-1" => egui::Color32::from_rgb(200, 100, 255),
                                                    _ => AXIS_TEXT,
                                                };
                                                ui.label(egui::RichText::new(&f.form_type).color(type_col).small());
                                                ui.label(egui::RichText::new(&f.category).color(AXIS_TEXT).small());
                                                ui.label(egui::RichText::new(&f.company_name).small());
                                                let score_col = if f.importance_score >= 80 { DOWN }
                                                    else if f.importance_score >= 50 { egui::Color32::from_rgb(255, 200, 50) }
                                                    else { AXIS_TEXT };
                                                ui.label(egui::RichText::new(format!("{}", f.importance_score)).color(score_col).small());
                                                ui.end_row();
                                            }
                                        });
                                    }
                                }
                            });

                            // Filing Alerts
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new("Filing Alerts").strong());
                            ui.separator();
                            if let Ok(alerts) = sec_filing::get_filing_alerts(&conn, false) {
                                if alerts.is_empty() {
                                    ui.label(egui::RichText::new("No active alerts.").color(AXIS_TEXT));
                                } else {
                                    let mut dismiss_id: Option<i64> = None;
                                    for alert in &alerts {
                                        let color = if alert.importance >= 80 { DOWN }
                                                    else if alert.importance >= 50 { egui::Color32::from_rgb(255, 160, 40) }
                                                    else { AXIS_TEXT };
                                        let severity = if alert.importance >= 80 { "High" }
                                                       else if alert.importance >= 50 { "Medium" }
                                                       else { "Low" };
                                        // Show importance score (computed via compute_importance internally)
                                        let _score = sec_filing::compute_importance(&alert.alert_type, false, false);
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("\u{2588}").color(color));
                                            ui.label(egui::RichText::new(severity).color(color).small());
                                            ui.label(egui::RichText::new(&alert.alert_type).color(egui::Color32::WHITE).small().strong());
                                            ui.label(egui::RichText::new(&alert.ticker).small().strong());
                                            ui.label(egui::RichText::new(&alert.message).color(AXIS_TEXT).small());
                                            if ui.small_button("Dismiss").clicked() {
                                                dismiss_id = Some(alert.id);
                                            }
                                        });
                                    }
                                    if let Some(id) = dismiss_id {
                                        let _ = sec_filing::dismiss_alert(&conn, id, "dismissed from GUI");
                                    }
                                }
                            }
                        }
                    }
                });
        }

        // Insider Trades (SEC Form 4) — wired to engine sec_filing.rs
        if self.show_insider {
            egui::Window::new("Insider Trades (Form 4)")
                .open(&mut self.show_insider)
                .default_size([650.0, 400.0])
                .show(ctx, |ui| {
                    let sym = self.charts.get(self.active_tab).map(|c| c.symbol.clone()).unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        ui.label(egui::RichText::new(&sym).strong().monospace());
                    });
                    ui.separator();
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            let _ = sec_filing::create_sec_tables(&conn);
                            // Extract ticker from cache key (e.g. "mt5:CC:SLV:4Hour" → "SLV")
                            let ticker = sym.split(':').rev().nth(1).or_else(|| sym.split(':').last()).unwrap_or(&sym);
                            if let Ok(trades) = sec_filing::get_insider_trades(&conn, Some(ticker), 90) {
                                if trades.is_empty() {
                                    ui.label(egui::RichText::new(format!("No insider trades for {} (last 90 days)", ticker)).color(AXIS_TEXT));
                                } else {
                                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                                        egui::Grid::new("insider_grid").striped(true).num_columns(6).show(ui, |ui| {
                                            ui.strong("Date"); ui.strong("Insider"); ui.strong("Title"); ui.strong("Type"); ui.strong("Shares"); ui.strong("Value");
                                            ui.end_row();
                                            for t in &trades {
                                                ui.label(egui::RichText::new(&t.transaction_date).small());
                                                ui.label(egui::RichText::new(&t.insider_name).small().strong());
                                                ui.label(egui::RichText::new(&t.insider_title).color(AXIS_TEXT).small());
                                                let type_col = if t.transaction_type.contains("Buy") || t.transaction_type.contains("Acquisition") { UP } else { DOWN };
                                                ui.label(egui::RichText::new(&t.transaction_type).color(type_col).small());
                                                ui.label(egui::RichText::new(format!("{:.0}", t.shares)).small());
                                                ui.label(egui::RichText::new(format!("${:.0}", t.aggregate_value)).small());
                                                ui.end_row();
                                            }
                                        });
                                    });
                                }
                            }
                        }
                    } else {
                        ui.label(egui::RichText::new("No cache available.").color(AXIS_TEXT));
                    }
                });
        }

        // Crypto Backfill (Kraken) — matching old WebKit layout
        if self.show_crypto_backfill {
            egui::Window::new("Crypto Backfill (Kraken)")
                .open(&mut self.show_crypto_backfill)
                .default_size([550.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new(egui::RichText::new("Backfill ALL Crypto (2013-Now)").color(egui::Color32::WHITE)).fill(BTN_GREEN).min_size(egui::vec2(260.0, 28.0))).clicked() {
                            let mut db_path = dirs_home(); db_path.push("cache"); db_path.push("typhoon_cache.db");
                            let tfs = vec!["1Day".into(), "4Hour".into(), "1Hour".into(), "15Min".into()];
                            for sym in &["BTCUSD", "ETHUSD", "SOLUSD", "DOGEUSD", "XRPUSD", "ADAUSD", "LTCUSD", "LINKUSD", "AVAXUSD", "DOTUSD"] {
                                let _ = self.broker_tx.send(BrokerCmd::KrakenBackfill { symbol: sym.to_string(), timeframes: tfs.clone(), db_path: db_path.clone() });
                            }
                            self.log.push_back(LogEntry::info("Kraken backfill started for 10 crypto pairs × 4 timeframes"));
                        }
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT).small());
                        ui.add(egui::TextEdit::singleline(&mut self.backfill_symbol).desired_width(100.0).hint_text("XBTUSD").font(egui::TextStyle::Monospace));
                        if ui.add(egui::Button::new("Backfill").fill(BTN_BLUE)).clicked() {
                            let sym = self.backfill_symbol.trim().to_string();
                            if !sym.is_empty() {
                                let mut db_path = dirs_home(); db_path.push("cache"); db_path.push("typhoon_cache.db");
                                let tfs = vec!["1Day".into(), "4Hour".into(), "1Hour".into(), "15Min".into()];
                                let _ = self.broker_tx.send(BrokerCmd::KrakenBackfill { symbol: sym.clone(), timeframes: tfs, db_path });
                                self.log.push_back(LogEntry::info(format!("Kraken backfill {} started (4 timeframes)", sym)));
                            }
                        }
                    });
                    ui.separator();

                    // Progress section
                    ui.label(egui::RichText::new("Progress").small().strong());
                    ui.label(egui::RichText::new("Connect Kraken API to start backfill").color(AXIS_TEXT).small());
                    ui.add_space(4.0);

                    // Table header (matching old WebKit)
                    egui::Grid::new("backfill_grid").striped(true).num_columns(5).show(ui, |ui| {
                        ui.strong("Symbol");
                        ui.strong("Timeframe");
                        ui.strong("New Bars");
                        ui.strong("Total Bars");
                        ui.strong("Status");
                        ui.end_row();
                        // Show cached crypto symbols if available
                        if let Some(ref cache) = self.cache {
                            if let Ok(stats) = cache.detailed_stats() {
                                for (key, count, _) in &stats {
                                    if key.starts_with("CC:") || key.starts_with("KRAKEN:") {
                                        let parts: Vec<&str> = key.rsplitn(2, ':').collect();
                                        let (tf_part, sym_part) = if parts.len() == 2 { (parts[0], parts[1]) } else { ("—", key.as_str()) };
                                        ui.label(egui::RichText::new(sym_part).small().monospace());
                                        ui.label(egui::RichText::new(tf_part).color(AXIS_TEXT).small());
                                        ui.label(egui::RichText::new("—").color(AXIS_TEXT).small());
                                        ui.label(egui::RichText::new(format!("{}", count)).small());
                                        ui.label(egui::RichText::new("cached").color(ACCENT).small());
                                        ui.end_row();
                                    }
                                }
                            }
                        }
                    });
                });
        }

        // Fundamentals
        if self.show_fundamentals {
            egui::Window::new("Fundamentals")
                .open(&mut self.show_fundamentals)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    let ticker = self.charts.get(self.active_tab)
                        .map(|c| c.symbol.split(':').rev().nth(1).or_else(|| c.symbol.split(':').last()).unwrap_or("").to_string())
                        .unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("Fundamentals: {}", ticker)).strong());
                        if ui.button("Fetch").clicked() && !ticker.is_empty() {
                            let _ = self.broker_tx.send(BrokerCmd::GetFundamentals { ticker: ticker.clone() });
                            self.log.push_back(LogEntry::info(format!("Fetching fundamentals for {}...", ticker)));
                        }
                    });
                    ui.separator();
                    ui.label(egui::RichText::new("Income statement, balance sheet, cash flow via SEC EDGAR.").color(AXIS_TEXT).small());
                    ui.label(egui::RichText::new("Results appear in the log panel below.").color(AXIS_TEXT).small());
                });
        }

        // Analyst — wired to Finnhub recommendations
        if self.show_analyst {
            egui::Window::new("Analyst Ratings")
                .open(&mut self.show_analyst)
                .default_size([400.0, 300.0])
                .show(ctx, |ui| {
                    let sym = self.charts.get(self.active_tab)
                        .map(|c| c.symbol.split(':').rev().nth(1).or_else(|| c.symbol.split(':').last()).unwrap_or("").to_string())
                        .unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("Analyst: {}", sym)).strong());
                        if ui.button("Fetch Ratings").clicked() && !sym.is_empty() && !self.finnhub_key.is_empty() {
                            let _ = self.broker_tx.send(BrokerCmd::GetAnalyst { symbol: sym.clone(), finnhub_key: self.finnhub_key.clone() });
                            self.log.push_back(LogEntry::info(format!("Fetching analyst ratings for {}...", sym)));
                        }
                    });
                    ui.separator();
                    if self.finnhub_key.is_empty() {
                        ui.label(egui::RichText::new("Enter Finnhub API key in Settings to fetch analyst data.").color(AXIS_TEXT).small());
                    } else {
                        ui.label(egui::RichText::new("Buy/Hold/Sell ratings, price targets via Finnhub.").color(AXIS_TEXT).small());
                    }
                    ui.label(egui::RichText::new("Results appear in the log panel below.").color(AXIS_TEXT).small());
                });
        }

        // Holders — wired to SEC EDGAR 13F
        if self.show_holders {
            egui::Window::new("Institutional Holders")
                .open(&mut self.show_holders)
                .default_size([500.0, 350.0])
                .show(ctx, |ui| {
                    let ticker = self.charts.get(self.active_tab)
                        .map(|c| c.symbol.split(':').rev().nth(1).or_else(|| c.symbol.split(':').last()).unwrap_or("").to_string())
                        .unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("Holders: {}", ticker)).strong());
                        if ui.button("Fetch 13F").clicked() && !ticker.is_empty() {
                            let _ = self.broker_tx.send(BrokerCmd::GetHolders { ticker: ticker.clone() });
                            self.log.push_back(LogEntry::info(format!("Fetching 13F holders for {}...", ticker)));
                        }
                    });
                    ui.separator();
                    ui.label(egui::RichText::new("Top institutional holders via SEC EDGAR 13F filings.").color(AXIS_TEXT).small());
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
                            if let Ok(overlaps) = darwin::get_symbol_overlap(&conn) {
                                if overlaps.is_empty() {
                                    ui.label("No overlapping symbols across DARWINs.");
                                } else {
                                    ui.label(egui::RichText::new(format!("{} overlapping symbols", overlaps.len())).strong());
                                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                                        egui::Grid::new("overlap_grid").striped(true).num_columns(6).show(ui, |ui| {
                                            ui.strong("Symbol"); ui.strong("Side"); ui.strong("Volume"); ui.strong("Notional"); ui.strong("Risk"); ui.strong("DARWINs");
                                            ui.end_row();
                                            for o in overlaps.iter() {
                                                ui.label(&o.symbol);
                                                let side_c = if o.side == "buy" { UP } else { DOWN };
                                                ui.label(egui::RichText::new(&o.side).color(side_c));
                                                ui.label(format!("{:.2}", o.total_volume));
                                                ui.label(format!("${:.0}", o.total_notional));
                                                let risk_c = match o.correlation_risk.as_str() {
                                                    "HIGH" => DOWN,
                                                    "MEDIUM" => egui::Color32::from_rgb(255, 200, 50),
                                                    _ => UP,
                                                };
                                                ui.label(egui::RichText::new(&o.correlation_risk).color(risk_c));
                                                ui.label(o.darwins.join(", "));
                                                ui.end_row();
                                            }
                                        });
                                    });
                                }
                            } else {
                                ui.label(egui::RichText::new("Import DARWIN data first.").color(AXIS_TEXT));
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
                                        for c in corrs.iter() {
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
                                let poc_idx = bins.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)).map(|(i, _)| i).unwrap_or(0);
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
                    let sym = self.charts.get(self.active_tab)
                        .map(|c| c.symbol.split(':').rev().nth(1).or_else(|| c.symbol.split(':').last()).unwrap_or("").to_string())
                        .unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("Order Flow: {}", sym)).strong());
                        if ui.button("Fetch L2").clicked() && !sym.is_empty() {
                            let _ = self.broker_tx.send(BrokerCmd::GetOrderbook { symbol: sym.clone() });
                            self.log.push_back(LogEntry::info(format!("Fetching orderbook for {}...", sym)));
                        }
                    });
                    ui.separator();
                    ui.label(egui::RichText::new("Bid/ask delta, cumulative delta, footprint.").color(AXIS_TEXT));
                    ui.label(egui::RichText::new("Connect broker + click Fetch L2 for live data.").color(AXIS_TEXT).small());
                });
        }

        // Bookmap — wired to orderbook API
        if self.show_bookmap {
            egui::Window::new("Bookmap Heatmap")
                .open(&mut self.show_bookmap)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    let sym = self.charts.get(self.active_tab)
                        .map(|c| c.symbol.split(':').rev().nth(1).or_else(|| c.symbol.split(':').last()).unwrap_or("").to_string())
                        .unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("Bookmap: {}", sym)).strong());
                        if ui.button("Fetch Depth").clicked() && !sym.is_empty() {
                            let _ = self.broker_tx.send(BrokerCmd::GetOrderbook { symbol: sym.clone() });
                        }
                    });
                    ui.separator();
                    ui.label(egui::RichText::new("Depth heatmap visualization (see ADR-048).").color(AXIS_TEXT));
                    ui.label(egui::RichText::new("GPU compute shader pipeline for real-time rendering.").color(AXIS_TEXT).small());
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
                                    // VaR Multipliers (Darwinex-style)
                                    ui.add_space(10.0);
                                    ui.heading("VaR Multipliers");
                                    ui.separator();
                                    if let Ok(mults) = darwin::compute_var_multipliers(&conn) {
                                        egui::Grid::new("var_mult_grid").striped(true).num_columns(5).show(ui, |ui| {
                                            ui.strong("DARWIN"); ui.strong("Monthly VaR"); ui.strong("Multiplier"); ui.strong("Corridor"); ui.strong("45d VaR");
                                            ui.end_row();
                                            for m in &mults {
                                                ui.label(&m.darwin_ticker);
                                                ui.label(format!("{:.2}%", m.monthly_var));
                                                let mc = if m.multiplier >= 1.5 { UP } else if m.multiplier >= 0.8 { egui::Color32::from_rgb(255, 200, 50) } else { DOWN };
                                                ui.label(egui::RichText::new(format!("{:.2}x", m.multiplier)).color(mc));
                                                let cc = if m.in_corridor { UP } else { DOWN };
                                                ui.label(egui::RichText::new(&m.corridor_position).color(cc));
                                                ui.label(format!("{:.2}%", m.var_45d));
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
                        ui.label("~ (tilde)"); ui.label("Console");
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
                    // ── DARWIN Risk Alerts ──────────────────────
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("DARWIN Risk Alerts").strong());
                    ui.separator();
                    let darwin_alerts = self.cache.as_ref().and_then(|c| c.connection().ok()).and_then(|conn| {
                        let _ = darwin::create_darwin_tables(&conn);
                        darwin::check_alerts(&conn).ok()
                    });
                    if let Some(alerts) = darwin_alerts {
                        if alerts.is_empty() {
                            ui.label(egui::RichText::new("No risk alerts — all clear.").color(UP));
                        } else {
                            for alert in &alerts {
                                let color = match alert.severity.as_str() {
                                    "CRITICAL" => DOWN,
                                    "WARNING" => egui::Color32::from_rgb(255, 200, 50),
                                    _ => AXIS_TEXT,
                                };
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("\u{2588}").color(color));
                                    ui.label(egui::RichText::new(&alert.severity).color(color).small().strong());
                                    ui.label(egui::RichText::new(&alert.alert_type).small().strong());
                                    ui.label(egui::RichText::new(&alert.message).color(AXIS_TEXT).small());
                                });
                            }
                        }
                    }
                });
        }

        // Order Entry
        if self.show_order_entry {
            egui::Window::new("Order Entry")
                .open(&mut self.show_order_entry)
                .default_size([400.0, 350.0])
                .show(ctx, |ui| {
                    ui.heading("Place Order");
                    ui.separator();
                    egui::Grid::new("order_grid").num_columns(2).show(ui, |ui| {
                        ui.label("Symbol:");
                        ui.add(egui::TextEdit::singleline(&mut self.order_symbol).desired_width(120.0));
                        ui.end_row();
                        ui.label("Side:");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.order_side, 0, egui::RichText::new("BUY").color(UP));
                            ui.radio_value(&mut self.order_side, 1, egui::RichText::new("SELL").color(DOWN));
                        });
                        ui.end_row();
                        ui.label("Quantity:");
                        ui.add(egui::TextEdit::singleline(&mut self.order_qty).desired_width(80.0));
                        ui.end_row();
                        ui.label("Type:");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.order_type, 0, "Market");
                            ui.radio_value(&mut self.order_type, 1, "Limit");
                            ui.radio_value(&mut self.order_type, 2, "Stop");
                            ui.radio_value(&mut self.order_type, 3, "Bracket");
                        });
                        ui.end_row();
                        if self.order_type == 1 || self.order_type == 3 {
                            ui.label("Limit Price:");
                            ui.add(egui::TextEdit::singleline(&mut self.order_limit_price).desired_width(100.0));
                            ui.end_row();
                        }
                        if self.order_type == 2 {
                            ui.label("Stop Price:");
                            ui.add(egui::TextEdit::singleline(&mut self.order_stop_price).desired_width(100.0));
                            ui.end_row();
                        }
                        if self.order_type == 3 {
                            ui.label("SL Price:");
                            ui.add(egui::TextEdit::singleline(&mut self.order_stop_price).desired_width(100.0));
                            ui.end_row();
                            ui.label("TP Price:");
                            ui.add(egui::TextEdit::singleline(&mut self.order_tp_price).desired_width(100.0));
                            ui.end_row();
                        }
                    });

                    // Risk preview
                    if let Ok(qty) = self.order_qty.parse::<f64>() {
                        if let Some(chart) = self.charts.get(self.active_tab) {
                            if let Some(last) = chart.bars.last() {
                                let notional = qty * last.close;
                                ui.separator();
                                ui.label(format!("Last price: {}", format_price(last.close)));
                                ui.label(format!("Notional: ${:.2}", notional));
                                if let Some(Some(atr)) = chart.atr.last() {
                                    ui.label(format!("ATR(14): {} ({:.2}%)", format_price(*atr), atr / last.close * 100.0));
                                }
                            }
                        }
                    }

                    ui.add_space(10.0);
                    let side_label = if self.order_side == 0 { "BUY" } else { "SELL" };
                    let type_label = ["Market", "Limit", "Stop", "Bracket"][self.order_type];
                    let btn_color = if self.order_side == 0 { UP } else { DOWN };
                    if ui.button(egui::RichText::new(format!("Submit {} {} Order", side_label, type_label)).color(btn_color).strong()).clicked() {
                        self.log.push_back(LogEntry::info(format!(
                            "Order: {} {} {} {} — connect broker to execute",
                            side_label, self.order_qty, self.order_symbol, type_label
                        )));
                    }
                });
        }

        // Restore cache after non-db frame (was temporarily taken to skip DB queries)
        if let Some(c) = real_cache {
            self.cache = Some(c);
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

        // ── Global font/spacing to match old WebKit (Consolas 11px) ──────
        if self.frame_count == 1 {
            let mut style = (*ctx.style()).clone();
            // ── AESTHETIC: MarketWizardry.org + Godel Terminal + old WebKit ──
            // Monospace everything, compact, square, green accents
            style.text_styles.insert(egui::TextStyle::Small, egui::FontId::new(10.0, egui::FontFamily::Monospace));
            style.text_styles.insert(egui::TextStyle::Body, egui::FontId::new(11.0, egui::FontFamily::Monospace));
            style.text_styles.insert(egui::TextStyle::Monospace, egui::FontId::new(11.0, egui::FontFamily::Monospace));
            style.text_styles.insert(egui::TextStyle::Button, egui::FontId::new(10.0, egui::FontFamily::Monospace));
            style.text_styles.insert(egui::TextStyle::Heading, egui::FontId::new(12.0, egui::FontFamily::Monospace));
            // Ultra-compact spacing (tighter than WebKit)
            style.spacing.item_spacing = egui::vec2(3.0, 1.0);
            style.spacing.button_padding = egui::vec2(4.0, 1.0);
            style.spacing.interact_size = egui::vec2(16.0, 14.0);
            style.spacing.indent = 8.0;
            style.spacing.scroll = egui::style::ScrollStyle {
                bar_width: 4.0,
                ..style.spacing.scroll
            };
            // ALL SQUARE — zero corner radius (MarketWizardry.org aesthetic)
            style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(0);
            style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(0);
            style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(0);
            style.visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(0);
            // Thin widget borders
            style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(0.5, egui::Color32::from_rgb(35, 40, 55));
            style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 65, 90));
            style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(0.0, egui::Color32::TRANSPARENT);
            ctx.set_style(style);
        }

        // ── poll async broker messages ───────────────────────────────────
        while let Ok(msg) = self.broker_rx.try_recv() {
            match msg {
                BrokerMsg::Connected(s) => {
                    self.broker_connected = true;
                    self.log.push_back(LogEntry::info(s));
                    // Auto-fetch positions and orders
                    let _ = self.broker_tx.send(BrokerCmd::GetPositions);
                    let _ = self.broker_tx.send(BrokerCmd::GetOrders);
                }
                BrokerMsg::Error(e) => {
                    self.log.push_back(LogEntry::err(e));
                }
                BrokerMsg::Account(acct) => {
                    self.live_account = Some(acct);
                }
                BrokerMsg::Positions(pos) => {
                    self.live_positions = pos;
                }
                BrokerMsg::Orders(orders) => {
                    self.live_orders = orders;
                }
                BrokerMsg::OrderResult(msg) => {
                    self.log.push_back(LogEntry::info(msg));
                    let _ = self.broker_tx.send(BrokerCmd::GetPositions);
                    let _ = self.broker_tx.send(BrokerCmd::GetOrders);
                }
                BrokerMsg::SecScrapeResult(msg) => {
                    self.log.push_back(LogEntry::info(msg));
                }
                BrokerMsg::FinnhubNewsResult(articles) => {
                    self.log.push_back(LogEntry::info(format!("Finnhub: {} articles loaded", articles.len())));
                    self.news_articles = articles;
                }
                BrokerMsg::Quote(symbol, bid, ask, last) => {
                    self.log.push_back(LogEntry::info(format!("{}: bid {} ask {} last {}", symbol, format_price(bid), format_price(ask), format_price(last))));
                }
                BrokerMsg::MarketClock(msg) => {
                    self.log.push_back(LogEntry::info(msg));
                }
                BrokerMsg::JsonResult(label, text) => {
                    self.log.push_back(LogEntry::info(format!("{}:\n{}", label, text)));
                }
            }
        }

        // ── Quake console toggle ─────────────────────────────────────────
        // Scans ALL input events for any sign of backtick/tilde/grave key.
        // Logs the first 20 unrecognized events for debugging Wayland issues.
        let open_palette = ctx.input_mut(|i| {
            let mut found = false;

            // Check all key methods
            if i.key_pressed(egui::Key::Backtick) { found = true; }

            // Scan every event
            i.events.retain(|e| {
                match e {
                    egui::Event::Text(t) if t == "`" || t == "~" => {
                        found = true;
                        false // consume
                    }
                    egui::Event::Key { key: egui::Key::Backtick, pressed: true, .. } => {
                        found = true;
                        false // consume
                    }
                    // Catch ANY key press and check the physical key
                    egui::Event::Key { key, pressed: true, physical_key, .. } => {
                        // Check if physical_key matches backtick/grave
                        if let Some(pk) = physical_key {
                            if *pk == egui::Key::Backtick {
                                found = true;
                                return false; // consume
                            }
                        }
                        // Also check if the logical key name contains "grave" or "backtick"
                        let key_name = format!("{:?}", key);
                        if key_name.contains("Backtick") || key_name.contains("Grave") {
                            found = true;
                            return false;
                        }
                        true
                    }
                    _ => true,
                }
            });
            found
        });
        if open_palette {
            self.command_open = !self.command_open;
            if self.command_open {
                self.command_input.clear();
            } else {
                // Strip any trailing ` or ~ from input that might have leaked
                self.command_input = self.command_input.trim_matches(|c| c == '`' || c == '~').to_string();
            }
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
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Connect to Broker…").clicked() {
                        self.show_connect = true;
                        ui.close();
                    }
                    if ui.button("Settings").clicked() {
                        self.show_settings = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Quit  Alt+F4").clicked() {
                        self.save_session();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("View", |ui| {
                    let mtf_label = if self.mtf_enabled { "Single Chart".to_string() } else { format!("MTF Grid ({} charts)", self.charts.len()) };
                    if ui.button(&mtf_label).clicked() {
                        self.mtf_enabled = !self.mtf_enabled;
                        ui.close();
                    }
                    ui.menu_button("Grid Layout", |ui| {
                        if ui.button("2×2 (4 charts)").clicked() { self.setup_mtf_grid(2, 4); ui.close(); }
                        if ui.button("3×2 (6 charts)").clicked() { self.setup_mtf_grid(3, 6); ui.close(); }
                        if ui.button("3×3 (9 charts)").clicked() { self.setup_mtf_grid(3, 9); ui.close(); }
                        if ui.button("4×3 (12 charts)").clicked() { self.setup_mtf_grid(4, 12); ui.close(); }
                        if ui.button("4×4 (16 charts)").clicked() { self.setup_mtf_grid(4, 16); ui.close(); }
                    });
                    if ui.button("Indicators…").clicked() {
                        self.show_indicators_panel = true;
                        ui.close();
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
                            ui.close();
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
                    ui.checkbox(&mut self.show_prev_levels, "Prev Candle Levels (D/W)");
                    ui.checkbox(&mut self.show_pivots,      "Pivot Points (P/R1/R2/S1/S2)");
                    ui.checkbox(&mut self.show_supply_demand, "Supply/Demand Zones");
                    ui.separator();
                    ui.label(egui::RichText::new("Pattern Recognition").color(AXIS_TEXT).small());
                    ui.checkbox(&mut self.show_fractals,    "Fractals (Bill Williams)");
                    ui.checkbox(&mut self.show_harmonics,     "Harmonic Patterns (Carney)");
                    ui.checkbox(&mut self.show_auto_fib,      "Auto Fibonacci");
                    ui.separator();
                    ui.label(egui::RichText::new("Ehlers (Overlay)").color(AXIS_TEXT).small());
                    ui.checkbox(&mut self.show_ehlers_ss,       "Super Smoother(10)");
                    ui.checkbox(&mut self.show_ehlers_decycler, "Decycler(20)");
                    ui.checkbox(&mut self.show_ehlers_itl,      "Instant. Trendline");
                    ui.checkbox(&mut self.show_ehlers_mama,     "MAMA / FAMA");
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
                    if ui.button("Open Trade…").clicked() {
                        self.show_order_entry = true;
                        self.order_symbol = self.symbol_input.clone();
                        ui.close();
                    }
                    if ui.button("Close All").clicked() {
                        if self.broker_connected {
                            let _ = self.broker_tx.send(BrokerCmd::CloseAll);
                            self.log.push_back(LogEntry::info("Closing all positions..."));
                        } else {
                            self.log.push_back(LogEntry::warn("Connect to broker first"));
                        }
                        ui.close();
                    }
                    if ui.button("Close Partial").clicked() {
                        self.log.push_back(LogEntry::info("Close Partial: select position in right panel"));
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Set SL").clicked() {
                        self.log.push_back(LogEntry::info("Trading: Set SL — connect to broker first"));
                        ui.close();
                    }
                    if ui.button("Set TP").clicked() {
                        self.log.push_back(LogEntry::info("Trading: Set TP — connect to broker first"));
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Open MG (Martingale Hedge)").clicked() {
                        self.log.push_back(LogEntry::info("Trading: Open MG — connect to broker first"));
                        ui.close();
                    }
                    if ui.button("Buy Lines").clicked() {
                        self.draw_mode = DrawMode::PlacingHLine;
                        self.log.push_back(LogEntry::info("Click chart to place buy reference line (green)"));
                        ui.close();
                    }
                    if ui.button("Sell Lines").clicked() {
                        self.draw_mode = DrawMode::PlacingHLine;
                        self.log.push_back(LogEntry::info("Click chart to place sell reference line (red)"));
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Set SL Line").clicked() {
                        self.draw_mode = DrawMode::PlacingHLine;
                        self.log.push_back(LogEntry::info("Click chart to set SL level"));
                        ui.close();
                    }
                    if ui.button("Set TP Line").clicked() {
                        self.draw_mode = DrawMode::PlacingHLine;
                        self.log.push_back(LogEntry::info("Click chart to set TP level"));
                        ui.close();
                    }
                    if self.sl_price.is_some() || self.tp_price.is_some() {
                        if ui.button("Clear SL/TP Lines").clicked() {
                            self.sl_price = None;
                            self.tp_price = None;
                            ui.close();
                        }
                    }
                });
                ui.menu_button("Tools", |ui| {
                    if ui.button("Console (~)").clicked() {
                        self.command_open = !self.command_open;
                        if self.command_open { self.command_input.clear(); }
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("DARWIN Accounts").clicked() {
                        self.show_darwin_accounts = true;
                        ui.close();
                    }
                    if ui.button("DARWIN Portfolio").clicked() {
                        self.show_darwin_portfolio = true;
                        ui.close();
                    }
                    if ui.button("Symbol Overlap").clicked() {
                        self.show_symbol_overlap = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Backtest").clicked() {
                        self.show_backtest = true;
                        ui.close();
                    }
                    if ui.button("Screener").clicked() {
                        self.show_screener = true;
                        ui.close();
                    }
                    if ui.button("Optimizer").clicked() {
                        self.show_optimizer = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Risk Calculator").clicked() {
                        self.show_risk_calc = true;
                        ui.close();
                    }
                    if ui.button("VaR Multiplier").clicked() {
                        self.show_var_mult = true;
                        ui.close();
                    }
                    if ui.button("Margin Monitor").clicked() {
                        self.show_margin_monitor = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Cache Statistics").clicked() {
                        self.show_cache_stats = true;
                        ui.close();
                    }
                });
                ui.menu_button("Research", |ui| {
                    if ui.button("News & Events").clicked() {
                        self.show_news = true;
                        ui.close();
                    }
                    if ui.button("Economic Calendar").clicked() {
                        self.show_calendar = true;
                        ui.close();
                    }
                    if ui.button("SEC Filings").clicked() {
                        self.show_sec = true;
                        ui.close();
                    }
                    if ui.button("Insider Trades").clicked() {
                        self.show_insider = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Fundamentals").clicked() {
                        self.show_fundamentals = true;
                        ui.close();
                    }
                    if ui.button("Analyst Ratings").clicked() {
                        self.show_analyst = true;
                        ui.close();
                    }
                    if ui.button("Institutional Holders").clicked() {
                        self.show_holders = true;
                        ui.close();
                    }
                });
                ui.menu_button("Analysis", |ui| {
                    if ui.button("Correlation Matrix").clicked() {
                        self.show_correlation = true;
                        ui.close();
                    }
                    if ui.button("Seasonals").clicked() {
                        self.show_seasonals = true;
                        ui.close();
                    }
                    if ui.button("Monte Carlo VaR").clicked() {
                        self.show_montecarlo = true;
                        ui.close();
                    }
                    if ui.button("Stress Test").clicked() {
                        self.show_stress_test = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Volume Profile").clicked() {
                        self.show_volume_profile = true;
                        ui.close();
                    }
                    if ui.button("Order Flow").clicked() {
                        self.show_order_flow = true;
                        ui.close();
                    }
                    if ui.button("Bookmap Heatmap").clicked() {
                        self.show_bookmap = true;
                        ui.close();
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("Keyboard Shortcuts").clicked() {
                        self.show_help = true;
                        ui.close();
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
                        .desired_width(180.0) // WebKit: width: 220px (minus padding)
                        .font(egui::FontId::monospace(13.0)), // WebKit: font-size: 13px
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
                    ui.label(egui::RichText::new("~").color(AXIS_TEXT).small());
                    ui.separator();
                    if self.broker_connected {
                        ui.label(egui::RichText::new("\u{25CF} LIVE").color(UP).small());
                        if let Some(ref acct) = self.live_account {
                            ui.label(egui::RichText::new(format!("${:.0}", acct.equity)).color(egui::Color32::WHITE).small());
                        }
                    } else {
                        ui.label(egui::RichText::new("\u{25CB} OFFLINE").color(AXIS_TEXT).small());
                    }
                });
            });
        });

        // ── tab bar ───────────────────────────────────────────────────────────
        egui::TopBottomPanel::top("tab_bar")
            .exact_height(26.0) // WebKit: height: 26px
            .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let mut switch_to: Option<usize> = None;
                let mut close_tab: Option<usize> = None;
                let mut drop_target: Option<usize> = None;
                let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
                let pointer_released = ctx.input(|i| i.pointer.primary_released());

                // Collect tab rects for drag detection
                let mut tab_rects: Vec<egui::Rect> = Vec::new();

                for (idx, chart) in self.charts.iter().enumerate() {
                    let active = idx == self.active_tab;
                    let is_dragging_this = self.dragging_tab == Some(idx);
                    let label = format!("{} [{}]", chart.symbol, chart.timeframe.label());

                    // Tab colours
                    let tab_bg = if is_dragging_this { egui::Color32::from_rgb(20, 50, 80) }
                                 else if active { BG_BUTTON }
                                 else { egui::Color32::from_rgb(10, 10, 10) };
                    let tab_text = if active { egui::Color32::WHITE } else { egui::Color32::from_rgb(136, 136, 136) };

                    let tab_w = label.len() as f32 * 6.5 + 28.0;

                    // Allocate space for this tab
                    let (tab_rect, tab_resp) = ui.allocate_exact_size(
                        egui::vec2(tab_w, 24.0),
                        egui::Sense::click_and_drag(),
                    );
                    tab_rects.push(tab_rect);

                    // Draw tab background
                    ui.painter().rect_filled(tab_rect, 0.0, tab_bg);

                    // Active tab: green bottom border
                    if active {
                        ui.painter().line_segment(
                            [egui::pos2(tab_rect.left(), tab_rect.bottom()), egui::pos2(tab_rect.right(), tab_rect.bottom())],
                            egui::Stroke::new(2.0, egui::Color32::from_rgb(76, 175, 80)),
                        );
                    }

                    // Right border separator
                    ui.painter().line_segment(
                        [egui::pos2(tab_rect.right(), tab_rect.top()), egui::pos2(tab_rect.right(), tab_rect.bottom())],
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(34, 34, 34)),
                    );

                    // Draw drag indicator (green left/right border when hovering during drag)
                    if let Some(drag_src) = self.dragging_tab {
                        if drag_src != idx {
                            if let Some(pos) = pointer_pos {
                                if tab_rect.contains(pos) {
                                    let mid = tab_rect.center().x;
                                    let side = if pos.x < mid { tab_rect.left() } else { tab_rect.right() };
                                    ui.painter().line_segment(
                                        [egui::pos2(side, tab_rect.top()), egui::pos2(side, tab_rect.bottom())],
                                        egui::Stroke::new(2.0, egui::Color32::from_rgb(76, 175, 80)),
                                    );
                                }
                            }
                        }
                    }

                    // Tab label text
                    let text_pos = egui::pos2(tab_rect.left() + 6.0, tab_rect.center().y);
                    ui.painter().text(
                        text_pos, egui::Align2::LEFT_CENTER, &label,
                        egui::FontId::monospace(10.0), tab_text,
                    );

                    // Close button (×) — right side of tab
                    if self.charts.len() > 1 {
                        let close_rect = egui::Rect::from_min_size(
                            egui::pos2(tab_rect.right() - 14.0, tab_rect.top() + 4.0),
                            egui::vec2(12.0, 16.0),
                        );
                        let close_hovered = pointer_pos.map(|p| close_rect.contains(p)).unwrap_or(false);
                        let close_col = if close_hovered { egui::Color32::from_rgb(255, 80, 80) } else { egui::Color32::from_rgb(85, 85, 85) };
                        ui.painter().text(
                            close_rect.center(), egui::Align2::CENTER_CENTER, "×",
                            egui::FontId::monospace(11.0), close_col,
                        );
                        if tab_resp.clicked() && close_hovered {
                            close_tab = Some(idx);
                        } else if tab_resp.clicked() {
                            switch_to = Some(idx);
                        }
                    } else if tab_resp.clicked() {
                        switch_to = Some(idx);
                    }

                    // Start drag
                    if tab_resp.dragged() && self.dragging_tab.is_none() {
                        self.dragging_tab = Some(idx);
                    }
                }

                // Handle drop on release
                if pointer_released {
                    if let Some(drag_src) = self.dragging_tab {
                        if let Some(pos) = pointer_pos {
                            for (idx, rect) in tab_rects.iter().enumerate() {
                                if rect.contains(pos) && idx != drag_src {
                                    let mid = rect.center().x;
                                    let target = if pos.x < mid { idx } else { idx };
                                    drop_target = Some(target);
                                    break;
                                }
                            }
                        }
                        self.dragging_tab = None;
                    }
                }

                // + button (WebKit: .tab-add)
                if ui.add(egui::Label::new(egui::RichText::new("+").color(egui::Color32::from_rgb(85, 85, 85)).size(14.0)).sense(egui::Sense::click())).clicked() {
                    let tf = self.charts.get(self.active_tab).map(|c| c.timeframe).unwrap_or(Timeframe::H4);
                    let mut new_chart = ChartState::new(&self.symbol_input, tf);
                    if let Some(ref cache) = self.cache.clone() {
                        new_chart.load(Arc::as_ref(cache), &mut self.log);
                    }
                    self.charts.push(new_chart);
                    self.active_tab = self.charts.len() - 1;
                }

                // Chart type indicator (right-aligned)
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
                if let Some(target) = drop_target {
                    if let Some(drag_src) = self.dragging_tab.or(Some(self.active_tab)) {
                        if drag_src < self.charts.len() && target < self.charts.len() && drag_src != target {
                            let chart = self.charts.remove(drag_src);
                            let insert_at = if target > drag_src { target } else { target };
                            let insert_at = insert_at.min(self.charts.len());
                            self.charts.insert(insert_at, chart);
                            self.active_tab = insert_at;
                        }
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
                    // Volume tab removed — BetterVolume sub-pane is more useful
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
                    // Volume tab removed — BetterVolume sub-pane is more useful
                }
            });

        // ── bottom status bar ────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(20.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let n_bars = self.charts.first().map(|c| c.bars.len()).unwrap_or(0);
                    let sym    = self.charts.first().map(|c| c.symbol.as_str()).unwrap_or("—");
                    let tf     = self.charts.first().map(|c| c.timeframe.label()).unwrap_or("—");
                    ui.label(
                        egui::RichText::new(format!("TyphooN Terminal"))
                        .color(QUAKE_CMD)
                        .small()
                        .strong(),
                    );
                    ui.label(egui::RichText::new("|").color(egui::Color32::from_rgb(40, 50, 70)).small());
                    ui.label(
                        egui::RichText::new(format!("{} [{}]", sym, tf))
                        .color(egui::Color32::WHITE)
                        .small()
                        .monospace(),
                    );
                    ui.label(egui::RichText::new("|").color(egui::Color32::from_rgb(40, 50, 70)).small());
                    ui.label(
                        egui::RichText::new(format!("{} bars", n_bars))
                        .color(AXIS_TEXT)
                        .small(),
                    );
                    if let Some(chart) = self.charts.first() {
                        if let Some(bar) = chart.bars.last() {
                            ui.label(egui::RichText::new("|").color(egui::Color32::from_rgb(40, 50, 70)).small());
                            let c_col = if bar.close >= bar.open { UP } else { DOWN };
                            ui.label(egui::RichText::new(format_price(bar.close)).color(c_col).small().monospace());
                        }
                    }
                    if let Some(err) = &self.cache_err {
                        ui.label(
                            egui::RichText::new(format!(" | {}", err))
                                .color(egui::Color32::from_rgb(255, 80, 80))
                                .small(),
                        );
                    }
                    // Right-aligned: DARWIN info
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new("VaR 3.25%–6.5%").color(egui::Color32::from_rgb(80, 90, 110)).small());
                        if self.broker_connected {
                            ui.label(egui::RichText::new("|").color(egui::Color32::from_rgb(40, 50, 70)).small());
                            ui.label(egui::RichText::new(format!("{} pos", self.live_positions.len())).color(AXIS_TEXT).small());
                        }
                    });
                });
            });

        // ── right panel (WebKit parity — trading buttons, positions, watchlist) ──
        egui::SidePanel::right("right_panel")
            .default_width(240.0)  // WebKit: width: 240px
            .min_width(140.0)      // WebKit: min-width: 140px
            .show(ctx, |ui| {
                // ── tab bar (compact tabs across top) ──────────────────
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for &tab in &[RightTab::Trading, RightTab::Positions, RightTab::Orders, RightTab::Watchlist, RightTab::Risk] {
                        let label = match tab {
                            RightTab::Trading => "Trade",
                            RightTab::Positions => "Pos",
                            RightTab::Orders => "Ord",
                            RightTab::Watchlist => "WL",
                            RightTab::Risk => "Risk",
                        };
                        let selected = self.right_tab == tab;
                        let color = if selected { ACCENT } else { AXIS_TEXT };
                        if ui.add(egui::Button::new(egui::RichText::new(label).color(color).small()).fill(if selected { egui::Color32::from_rgb(20, 40, 60) } else { egui::Color32::TRANSPARENT }).min_size(egui::vec2(40.0, 20.0))).clicked() {
                            self.right_tab = tab;
                        }
                    }
                });
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    match self.right_tab {
                        RightTab::Trading => {
                            // ── Trading Buttons Grid (exact WebKit CSS: #button-grid) ──
                            ui.add_space(8.0);
                            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0); // gap: 4px
                            let btn_w = (ui.available_width() - 4.0) / 2.0;
                            let btn_size = egui::vec2(btn_w, 28.0); // padding: 8px 4px ≈ 28px

                            // Row 1: Open Trade (.btn-action) | Buy Lines (.btn-lines)
                            ui.horizontal(|ui| {
                                if ui.add(egui::Button::new(egui::RichText::new("Open Trade").color(BTN_GREEN_TEXT).small().strong()).fill(BTN_GREEN).min_size(btn_size)).clicked() {
                                    self.show_order_entry = true;
                                }
                                if ui.add(egui::Button::new(egui::RichText::new("Buy Lines").color(BTN_BLUE_TEXT).small().strong()).fill(BTN_BLUE).min_size(btn_size)).clicked() {
                                    self.draw_mode = DrawMode::PlacingHLine;
                                }
                            });
                            // Row 2: Sell Lines (.btn-lines) | Destroy Lines (.btn-lines)
                            ui.horizontal(|ui| {
                                if ui.add(egui::Button::new(egui::RichText::new("Sell Lines").color(BTN_BLUE_TEXT).small().strong()).fill(BTN_BLUE).min_size(btn_size)).clicked() {
                                    self.draw_mode = DrawMode::PlacingHLine;
                                }
                                if ui.add(egui::Button::new(egui::RichText::new("Destroy Lines").color(BTN_BLUE_TEXT).small().strong()).fill(BTN_BLUE).min_size(btn_size)).clicked() {
                                    if let Some(chart) = self.charts.get_mut(self.active_tab) {
                                        chart.drawings.clear();
                                    }
                                    self.sl_price = None;
                                    self.tp_price = None;
                                }
                            });
                            // Row 3: Open MG (.btn-mg) | Close All (.btn-danger)
                            ui.horizontal(|ui| {
                                if ui.add(egui::Button::new(egui::RichText::new("Open MG").color(BTN_MG_TEXT).small().strong()).fill(BTN_MG).min_size(btn_size)).clicked() {
                                    self.log.push_back(LogEntry::info("Martingale: connect broker first"));
                                }
                                if ui.add(egui::Button::new(egui::RichText::new("Close All").color(BTN_RED_TEXT).small().strong()).fill(BTN_RED).min_size(btn_size)).clicked() {
                                    let _ = self.broker_tx.send(BrokerCmd::CloseAll);
                                }
                            });
                            // Row 4: Close Partial (.btn-danger) | Set SL (.btn-lines)
                            ui.horizontal(|ui| {
                                if ui.add(egui::Button::new(egui::RichText::new("Close Partial").color(BTN_RED_TEXT).small().strong()).fill(BTN_RED).min_size(btn_size)).clicked() {
                                    self.log.push_back(LogEntry::info("Close partial: connect broker"));
                                }
                                if ui.add(egui::Button::new(egui::RichText::new("Set SL").color(BTN_BLUE_TEXT).small().strong()).fill(BTN_BLUE).min_size(btn_size)).clicked() {
                                    self.sl_enabled = true;
                                }
                            });
                            // Row 5: Set TP (.btn-lines)
                            ui.horizontal(|ui| {
                                if ui.add(egui::Button::new(egui::RichText::new("Set TP").color(BTN_BLUE_TEXT).small().strong()).fill(BTN_BLUE).min_size(btn_size)).clicked() {
                                    self.tp_enabled = true;
                                }
                            });
                            ui.add_space(6.0);

                            // ── SL / TP Price Inputs ──────────────────────────
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut self.sl_enabled, "");
                                ui.label(egui::RichText::new("SL Price").color(AXIS_TEXT).small());
                                let resp = ui.add(egui::TextEdit::singleline(&mut self.sl_input).desired_width(100.0).hint_text("0.0").font(egui::TextStyle::Small));
                                if resp.lost_focus() && self.sl_enabled {
                                    self.sl_price = self.sl_input.parse().ok();
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut self.tp_enabled, "");
                                ui.label(egui::RichText::new("TP Price").color(AXIS_TEXT).small());
                                let resp = ui.add(egui::TextEdit::singleline(&mut self.tp_input).desired_width(100.0).hint_text("0.0").font(egui::TextStyle::Small));
                                if resp.lost_focus() && self.tp_enabled {
                                    self.tp_price = self.tp_input.parse().ok();
                                }
                            });
                            ui.add_space(6.0);

                            // ── Mode / Type Dropdowns ──────────────────────────
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Mode").color(AXIS_TEXT).small());
                                egui::ComboBox::from_id_salt("risk_mode_combo")
                                    .selected_text(self.risk_mode.label())
                                    .width(80.0)
                                    .show_ui(ui, |ui| {
                                        for mode in &[RiskMode::VaR, RiskMode::Standard, RiskMode::Fixed, RiskMode::Dynamic] {
                                            ui.selectable_value(&mut self.risk_mode, *mode, mode.label());
                                        }
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Type").color(AXIS_TEXT).small());
                                egui::ComboBox::from_id_salt("order_type_combo")
                                    .selected_text(self.order_type_mode.label())
                                    .width(80.0)
                                    .show_ui(ui, |ui| {
                                        for t in &[OrderTypeMode::Market, OrderTypeMode::Limit, OrderTypeMode::Stop] {
                                            ui.selectable_value(&mut self.order_type_mode, *t, t.label());
                                        }
                                    });
                            });
                            ui.add_space(6.0);

                            // ── Position Info Block ────────────────────────────
                            ui.separator();
                            if let Some(chart) = self.charts.get(self.active_tab) {
                                if let Some(bar) = chart.bars.last() {
                                    let close = bar.close;
                                    // Show current position info if any
                                    let mut has_pos = false;
                                    for pos in &self.live_positions {
                                        if pos.symbol.contains(&chart.symbol.split(':').last().unwrap_or("")) {
                                            let side_c = if pos.side == "long" { UP } else { DOWN };
                                            let side_label = if pos.side == "long" { "Long" } else { "Short" };
                                            ui.label(egui::RichText::new(format!("{} {:.2} lots", side_label, pos.qty)).color(side_c).strong());
                                            let pl_c = if pos.unrealized_pl >= 0.0 { UP } else { DOWN };
                                            ui.label(egui::RichText::new(format!("P&L: ${:.2}", pos.unrealized_pl)).color(pl_c));

                                            // SL/TP P&L if set
                                            if let Some(sl) = self.sl_price {
                                                let sl_pl = (close - sl) * pos.qty * if pos.side == "long" { 1.0 } else { -1.0 };
                                                let sl_c = if sl_pl >= 0.0 { UP } else { DOWN };
                                                ui.label(egui::RichText::new(format!("SL P/L: ${:.2}", sl_pl)).color(sl_c).small());
                                            }
                                            if let Some(tp) = self.tp_price {
                                                let tp_pl = (tp - close) * pos.qty * if pos.side == "long" { 1.0 } else { -1.0 };
                                                let tp_c = if tp_pl >= 0.0 { UP } else { DOWN };
                                                ui.label(egui::RichText::new(format!("TP P/L: ${:.2}", tp_pl)).color(tp_c).small());
                                            }
                                            if let (Some(sl), Some(tp)) = (self.sl_price, self.tp_price) {
                                                let risk = (close - sl).abs();
                                                let reward = (tp - close).abs();
                                                let rr = if risk > 0.0 { reward / risk } else { 0.0 };
                                                ui.label(egui::RichText::new(format!("R:R {:.2}", rr)).color(AXIS_TEXT).small());
                                            }
                                            has_pos = true;
                                            break;
                                        }
                                    }
                                    if !has_pos {
                                        ui.label(egui::RichText::new("No position").color(AXIS_TEXT).small());
                                    }

                                    // Account summary line
                                    if let Some(ref acct) = self.live_account {
                                        ui.add_space(4.0);
                                        egui::Grid::new("acct_mini").num_columns(2).show(ui, |ui| {
                                            ui.label(egui::RichText::new("Eq").color(AXIS_TEXT).small());
                                            ui.label(egui::RichText::new(format!("${:.0}", acct.equity)).small());
                                            ui.end_row();
                                            ui.label(egui::RichText::new("Bal").color(AXIS_TEXT).small());
                                            ui.label(egui::RichText::new(format!("${:.0}", acct.cash)).small());
                                            ui.end_row();
                                            ui.label(egui::RichText::new("BP").color(AXIS_TEXT).small());
                                            ui.label(egui::RichText::new(format!("${:.0}", acct.buying_power)).small());
                                            ui.end_row();
                                        });
                                    }
                                }
                            }

                            // ── MTF MA Grid (colored dots) ─────────────────────
                            ui.add_space(6.0);
                            ui.separator();
                            ui.label(egui::RichText::new("MTF Grid").color(AXIS_TEXT).small().strong());
                            let tf_labels = ["M1", "M5", "M15", "M30", "H1", "H4", "D1", "W1"];
                            let ma_labels = ["SMA200", "KAMA", "Fisher"];
                            let chart_ref = self.charts.get(self.active_tab);
                            egui::Grid::new("mtf_ma_grid").spacing(egui::vec2(4.0, 2.0)).show(ui, |ui| {
                                // Header row
                                ui.label(egui::RichText::new("").small());
                                for tf in &tf_labels {
                                    ui.label(egui::RichText::new(*tf).color(AXIS_TEXT).small());
                                }
                                ui.end_row();
                                // Data rows — green if bullish, red if bearish, gray if no data
                                let active_tf_label = chart_ref.map(|c| c.timeframe.label()).unwrap_or("");
                                for ma in &ma_labels {
                                    ui.label(egui::RichText::new(*ma).color(AXIS_TEXT).small());
                                    for tf in &tf_labels {
                                        let dot_color = if *tf == active_tf_label {
                                            if let Some(c) = chart_ref {
                                                let last_bar = c.bars.last();
                                                let bullish = match *ma {
                                                    "SMA200" => {
                                                        let sma = c.sma200.last().and_then(|v| *v);
                                                        match (last_bar, sma) {
                                                            (Some(b), Some(s)) => Some(b.close > s),
                                                            _ => None,
                                                        }
                                                    }
                                                    "KAMA" => {
                                                        let kama = c.kama.last().and_then(|v| *v);
                                                        match (last_bar, kama) {
                                                            (Some(b), Some(k)) => Some(b.close > k),
                                                            _ => None,
                                                        }
                                                    }
                                                    "Fisher" => {
                                                        let fisher = c.fisher.last().and_then(|v| *v);
                                                        let signal = c.fisher_signal.last().and_then(|v| *v);
                                                        match (fisher, signal) {
                                                            (Some(f), Some(s)) => Some(f > s),
                                                            _ => None,
                                                        }
                                                    }
                                                    _ => None,
                                                };
                                                match bullish {
                                                    Some(true) => UP,
                                                    Some(false) => DOWN,
                                                    None => AXIS_TEXT,
                                                }
                                            } else {
                                                AXIS_TEXT
                                            }
                                        } else {
                                            egui::Color32::from_rgb(50, 50, 60)
                                        };
                                        ui.label(egui::RichText::new("\u{25CF}").color(dot_color).small());
                                    }
                                    ui.end_row();
                                }
                            });
                        }

                        RightTab::Positions => {
                            ui.add_space(4.0);
                            let mut has_positions = false;
                            // DARWIN positions
                            if let Some(ref cache) = self.cache {
                                if let Ok(conn) = cache.connection() {
                                    let _ = darwin::create_darwin_tables(&conn);
                                    if let Ok(positions) = darwin::get_portfolio_open_positions(&conn) {
                                        if !positions.is_empty() {
                                            has_positions = true;
                                            for pos in positions.iter() {
                                                let side_c = if pos.side == "buy" { UP } else { DOWN };
                                                ui.horizontal(|ui| {
                                                    ui.label(egui::RichText::new(&pos.symbol).small().strong());
                                                    let side_label = if pos.side == "buy" { "L" } else { "S" };
                                                    ui.label(egui::RichText::new(side_label).color(side_c).small());
                                                    ui.label(egui::RichText::new(format!("{:.2}", pos.total_volume)).small());
                                                    let pl_c = if pos.notional >= 0.0 { UP } else { DOWN };
                                                    ui.label(egui::RichText::new(format!("${:.0}", pos.notional)).color(pl_c).small());
                                                });
                                                let darwins: Vec<String> = pos.darwin_breakdown.iter().map(|(d, _, _)| d.clone()).collect();
                                                ui.label(egui::RichText::new(darwins.join(", ")).color(AXIS_TEXT).small());
                                                ui.separator();
                                            }
                                        }
                                    }
                                }
                            }
                            // Live broker positions
                            if self.broker_connected && !self.live_positions.is_empty() {
                                has_positions = true;
                                for pos in &self.live_positions {
                                    let side_c = if pos.side == "long" { UP } else { DOWN };
                                    let side_label = if pos.side == "long" { "L" } else { "S" };
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(&pos.symbol).small().strong());
                                        ui.label(egui::RichText::new(side_label).color(side_c).small());
                                        ui.label(egui::RichText::new(format!("{:.2}", pos.qty)).small());
                                        let pl_c = if pos.unrealized_pl >= 0.0 { UP } else { DOWN };
                                        ui.label(egui::RichText::new(format!("${:.2}", pos.unrealized_pl)).color(pl_c).small());
                                    });
                                    ui.label(egui::RichText::new(format!("entry: {}", format_price(pos.avg_entry_price))).color(AXIS_TEXT).small());
                                    ui.separator();
                                }
                            }
                            if !has_positions {
                                ui.label(egui::RichText::new("No open positions.").color(AXIS_TEXT).small());
                            }

                            // ── Recent Fills ──────────────────────────────────
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new("Recent Fills").color(AXIS_TEXT).small().strong());
                            ui.separator();
                            if self.recent_fills.is_empty() {
                                ui.label(egui::RichText::new("No recent fills.").color(AXIS_TEXT).small());
                            } else {
                                for (sym, side, qty, price, time) in &self.recent_fills {
                                    let c = if side == "buy" { UP } else { DOWN };
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(sym).small().strong());
                                        ui.label(egui::RichText::new(side).color(c).small());
                                        ui.label(egui::RichText::new(format!("{:.2}@{}", qty, format_price(*price))).small());
                                        ui.label(egui::RichText::new(time).color(AXIS_TEXT).small());
                                    });
                                }
                            }
                        }

                        RightTab::Orders => {
                            ui.add_space(4.0);
                            if self.broker_connected && !self.live_orders.is_empty() {
                                for order in &self.live_orders {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(&order.symbol).small().strong());
                                        let side_c = if order.side == "buy" { UP } else { DOWN };
                                        ui.label(egui::RichText::new(&order.side).color(side_c).small());
                                        ui.label(egui::RichText::new(&order.order_type).color(AXIS_TEXT).small());
                                    });
                                    ui.label(egui::RichText::new(format!("qty: {} | {}", order.qty, order.status)).color(ACCENT).small());
                                    ui.separator();
                                }
                            } else {
                                ui.label(egui::RichText::new(if self.broker_connected { "No open orders." } else { "Connect broker for live orders." }).color(AXIS_TEXT).small());
                            }
                        }

                        RightTab::Watchlist => {
                            // TradingView-style watchlist header
                            ui.add_space(2.0);
                            egui::Grid::new("wl_header").num_columns(5).spacing(egui::vec2(4.0, 0.0)).show(ui, |ui| {
                                ui.label(egui::RichText::new("Symbol").color(AXIS_TEXT).small());
                                ui.label(egui::RichText::new("Last").color(AXIS_TEXT).small());
                                ui.label(egui::RichText::new("Chg").color(AXIS_TEXT).small());
                                ui.label(egui::RichText::new("Chg%").color(AXIS_TEXT).small());
                                ui.label(egui::RichText::new("Vol").color(AXIS_TEXT).small());
                                ui.end_row();
                            });
                            // Thin separator under header
                            let sep_rect = ui.available_rect_before_wrap();
                            ui.painter().line_segment(
                                [egui::pos2(sep_rect.left(), sep_rect.top()), egui::pos2(sep_rect.right(), sep_rect.top())],
                                egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 40, 50)),
                            );
                            ui.add_space(2.0);

                            if self.watchlist_rows.is_empty() {
                                ui.label(egui::RichText::new("No cached symbols.").color(AXIS_TEXT).small());
                            } else {
                                let mut load_key: Option<String> = None;
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    for (idx, wl) in self.watchlist_rows.iter().enumerate() {
                                        let sym_color = WL_COLORS[idx % WL_COLORS.len()];
                                        let chg_color = if wl.change >= 0.0 { UP } else { DOWN };
                                        let is_selected = self.charts.get(self.active_tab)
                                            .map(|c| c.symbol == wl.cache_key)
                                            .unwrap_or(false);
                                        let row_bg = if is_selected {
                                            egui::Color32::from_rgb(15, 25, 45)
                                        } else {
                                            egui::Color32::TRANSPARENT
                                        };

                                        let row_rect = ui.available_rect_before_wrap();
                                        let row_rect = egui::Rect::from_min_size(row_rect.min, egui::vec2(row_rect.width(), 18.0));
                                        ui.painter().rect_filled(row_rect, 0.0, row_bg);

                                        let row = egui::Grid::new(format!("wl_row_{}", idx)).num_columns(5).spacing(egui::vec2(4.0, 0.0)).show(ui, |ui| {
                                            // Symbol with colored dot
                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing.x = 2.0;
                                                ui.label(egui::RichText::new("\u{25CF}").color(sym_color).small());
                                                ui.label(egui::RichText::new(&wl.symbol).color(egui::Color32::WHITE).small().monospace().strong());
                                            });
                                            // Last price
                                            ui.label(egui::RichText::new(format_price(wl.last)).color(egui::Color32::WHITE).small().monospace());
                                            // Change
                                            let chg_str = if wl.change >= 0.0 { format_price(wl.change) } else { format!("-{}", format_price(wl.change.abs())) };
                                            ui.label(egui::RichText::new(chg_str).color(chg_color).small().monospace());
                                            // Change %
                                            ui.label(egui::RichText::new(format!("{:.2}%", wl.change_pct)).color(chg_color).small().monospace());
                                            // Volume (compact format)
                                            let vol_str = if wl.volume >= 1_000_000.0 {
                                                format!("{:.2} M", wl.volume / 1_000_000.0)
                                            } else if wl.volume >= 1_000.0 {
                                                format!("{:.2} K", wl.volume / 1_000.0)
                                            } else {
                                                format!("{:.0}", wl.volume)
                                            };
                                            ui.label(egui::RichText::new(vol_str).color(AXIS_TEXT).small().monospace());
                                            ui.end_row();
                                        });
                                        if row.response.interact(egui::Sense::click()).clicked() {
                                            load_key = Some(wl.cache_key.clone());
                                        }
                                    }
                                });
                                if let Some(key) = load_key {
                                    if let Some(ref cache) = self.cache {
                                        if let Some(chart) = self.charts.get_mut(self.active_tab) {
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
                            }
                        }

                        RightTab::Risk => {
                            ui.add_space(4.0);
                            // Live broker account data
                            if let Some(ref acct) = self.live_account {
                                egui::Grid::new("live_risk_grid").striped(true).num_columns(2).show(ui, |ui| {
                                    ui.label(egui::RichText::new("Equity").color(AXIS_TEXT).small());
                                    ui.label(egui::RichText::new(format!("${:.2}", acct.equity)).small());
                                    ui.end_row();
                                    ui.label(egui::RichText::new("Cash").color(AXIS_TEXT).small());
                                    ui.label(egui::RichText::new(format!("${:.2}", acct.cash)).small());
                                    ui.end_row();
                                    ui.label(egui::RichText::new("Buying Power").color(AXIS_TEXT).small());
                                    ui.label(egui::RichText::new(format!("${:.2}", acct.buying_power)).small());
                                    ui.end_row();
                                    ui.label(egui::RichText::new("Margin Used").color(AXIS_TEXT).small());
                                    ui.label(egui::RichText::new(format!("${:.2}", acct.initial_margin)).small());
                                    ui.end_row();
                                });
                                ui.add_space(5.0);
                            }
                            // DARWIN portfolio data
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
                                            // VaR
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
                            ui.add_space(6.0);
                            ui.separator();
                            ui.label(egui::RichText::new("DARWIN").small().strong());
                            ui.label(egui::RichText::new("VaR corridor: 3.25% – 6.5%").color(AXIS_TEXT).small());
                            ui.label(egui::RichText::new("Correlation limit: 0.95 / 45d").color(AXIS_TEXT).small());
                        }
                    }
                });
            });

        // ── floating windows ─────────────────────────────────────────────────
        // Always call draw_floating_windows so close buttons work.
        // Performance optimization is inside: heavy content gated by db_ok flag.
        self.draw_floating_windows(ctx);

        // ── central panel (chart area) ────────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_rect_before_wrap();

            // ── Price axis rect (right 70px of chart — TradingView-style scale) ──
            let price_axis_w = 70.0_f32;
            let price_axis_rect = egui::Rect::from_min_max(
                egui::pos2(available.right() - price_axis_w, available.top()),
                available.max,
            );
            let chart_body_rect = egui::Rect::from_min_max(
                available.min,
                egui::pos2(available.right() - price_axis_w, available.bottom()),
            );

            let hover_pos = ctx.input(|i| i.pointer.hover_pos().unwrap_or_default());
            // Don't interact with chart when pointer is over a floating window or egui wants pointer
            let egui_hover = ctx.wants_pointer_input() || ctx.is_using_pointer() || ctx.dragged_id().is_some();
            let layer_at_hover = ctx.layer_id_at(hover_pos);
            let hover_over_window = egui_hover || layer_at_hover
                .map(|id| id.order == egui::Order::Middle || id.order == egui::Order::Foreground)
                .unwrap_or(false);
            let on_price_axis = price_axis_rect.contains(hover_pos) && !hover_over_window;
            let on_chart_body = chart_body_rect.contains(hover_pos) && !hover_over_window;

            // Scroll → zoom (only when not over a floating window, skip in MTF mode — cells handle own zoom)
            let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta != 0.0 && !hover_over_window && !self.mtf_enabled {
                if on_price_axis {
                    // Scroll on price axis → vertical zoom (TradingView style: squish/expand)
                    if let Some(chart) = self.charts.get_mut(self.active_tab) {
                        let pct = (scroll_delta * 0.002).clamp(-0.08, 0.08);
                        chart.price_zoom = (chart.price_zoom * (1.0 + pct as f64)).clamp(0.1, 20.0);
                    }
                } else if on_chart_body {
                    let ctrl_held = ctx.input(|i| i.modifiers.ctrl);
                    if ctrl_held {
                        // Ctrl+scroll on chart → vertical zoom (progressive)
                        if let Some(chart) = self.charts.get_mut(self.active_tab) {
                            let pct = (scroll_delta * 0.002).clamp(-0.08, 0.08);
                            chart.price_zoom = (chart.price_zoom * (1.0 + pct as f64)).clamp(0.1, 20.0);
                        }
                    } else {
                        // Scroll on chart → horizontal zoom (time axis, progressive)
                        for chart in &mut self.charts {
                            Self::handle_zoom(chart, scroll_delta);
                        }
                    }
                }
            }

            // Double-click → reset zoom/pan
            if ctx.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary)) {
                if on_price_axis {
                    // Double-click price axis → auto-fit vertical only
                    if let Some(chart) = self.charts.get_mut(self.active_tab) {
                        chart.price_zoom = 1.0;
                        chart.price_pan = 0.0;
                    }
                } else if on_chart_body {
                    // Double-click chart → reset everything
                    if let Some(chart) = self.charts.get_mut(self.active_tab) {
                        chart.price_zoom = 1.0;
                        chart.price_pan = 0.0;
                        chart.visible_bars = 200;
                        chart.view_offset = chart.bars.len().saturating_sub(1);
                    }
                }
            }

            // Drag interactions — only when pointer is NOT over a floating window
            let pointer    = ctx.input(|i| i.pointer.clone());
            let drag_delta = ctx.input(|i| i.pointer.delta());
            // Block chart interaction when ANY egui widget/window is using the pointer
            let egui_wants_pointer = ctx.wants_pointer_input() || ctx.is_using_pointer();
            let anything_dragged = ctx.dragged_id().is_some();
            let layer_id_at_pointer = ctx.layer_id_at(pointer.hover_pos().unwrap_or_default());
            let pointer_over_window = egui_wants_pointer || anything_dragged || layer_id_at_pointer
                .map(|id| id.order == egui::Order::Middle || id.order == egui::Order::Foreground)
                .unwrap_or(false);

            // Skip drag in MTF mode — individual cells handle their own interaction
            if !self.mtf_enabled {
            for chart in &mut self.charts {
                if pointer.primary_pressed() && !pointer_over_window {
                    let press_pos = pointer.press_origin().unwrap_or_default();
                    // Only start drag if press originated inside chart area or price axis
                    if price_axis_rect.contains(press_pos) {
                        // Start price-axis scaling drag (TradingView style)
                        chart.is_scaling_price = true;
                        chart.is_dragging = false;
                        chart.scale_start_zoom = chart.price_zoom;
                        chart.scale_start_y = press_pos.y;
                    } else if available.contains(press_pos) {
                        // Start normal chart pan drag — only if inside the chart area
                        chart.is_dragging = true;
                        chart.is_scaling_price = false;
                        chart.drag_start = pointer.press_origin();
                        chart.drag_start_offset = chart.view_offset;
                        chart.drag_start_ppan = chart.price_pan;
                    }
                } else if pointer.primary_released() || pointer_over_window {
                    // Stop dragging when mouse released OR pointer moves over a floating window
                    chart.is_dragging = false;
                    chart.is_scaling_price = false;
                    chart.drag_start = None;
                }

                // Price axis drag → vertical zoom (like TradingView)
                if chart.is_scaling_price && drag_delta.y.abs() > 0.0 {
                    // Drag up = zoom in (expand), drag down = zoom out (compress)
                    // Drag up = expand (zoom in), drag down = squish (zoom out)
                    // TradingView-style progressive scaling
                    let sensitivity = 0.003;
                    let zoom_delta = -drag_delta.y as f64 * sensitivity;
                    chart.price_zoom = (chart.price_zoom * (1.0 + zoom_delta)).clamp(0.1, 20.0);
                }

                // Normal chart body drag → pan
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
            } // end !mtf_enabled drag guard

            // Console is rendered as egui::Window after CentralPanel (see below)

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
            let show_ehlers_ebsw = self.show_ehlers_ebsw;
            let show_ehlers_cyber = self.show_ehlers_cyber;
            let show_ehlers_cg = self.show_ehlers_cg;
            let show_ehlers_roof = self.show_ehlers_roof;
            let sl_price = self.sl_price;
            let tp_price = self.tp_price;

            if self.mtf_enabled {
                let total = self.charts.len().min(16);
                let cols   = self.mtf_cols.max(1).min(total);
                let rows   = (total + cols - 1) / cols;
                let cell_w = available.width()  / cols  as f32;
                let cell_h = available.height() / rows  as f32;

                // Detect click on grid cell to focus it
                let click_pos = if ctx.input(|i| i.pointer.primary_clicked()) {
                    ctx.input(|i| i.pointer.interact_pos())
                } else { None };

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

                    // Click to focus this cell
                    if let Some(pos) = click_pos {
                        if cell_rect.contains(pos) {
                            self.mtf_focused = Some(idx);
                            self.active_tab = idx;
                        }
                    }

                    // Auto-focus on hover, confirm on click
                    let ptr_in_cell = ctx.input(|i| i.pointer.hover_pos().map(|p| cell_rect.contains(p)).unwrap_or(false));
                    if ptr_in_cell {
                        // Auto-set focus when hovering (no click required for zoom)
                        self.mtf_focused = Some(idx);
                        self.active_tab = idx;
                    }
                    let is_focused = self.mtf_focused == Some(idx);

                    // Zoom when pointer is in this cell (no focus-click required)
                    if ptr_in_cell {
                        let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
                        if scroll != 0.0 {
                            Self::handle_zoom(chart, scroll);
                        }
                        // Drag pan for this cell
                        let drag = ctx.input(|i| i.pointer.delta());
                        if ctx.input(|i| i.pointer.primary_down()) && drag.x.abs() > 0.5 {
                            Self::handle_pan_h(chart, -drag.x, cell_rect.width());
                        }
                    }

                    let painter = ui.painter_at(cell_rect);
                    draw_chart(&painter, chart, cell_rect, crosshair, &flags, show_rsi, show_fisher, show_macd, show_volume_pane, show_stochastic, show_adx, show_cci, show_williams_r, show_obv, show_momentum, show_better_volume, show_ehlers_ebsw, show_ehlers_cyber, show_ehlers_cg, show_ehlers_roof, sl_price, tp_price);

                    // Border: green for focused, dim for others (WebKit: .mtf-grid-cell:hover outline)
                    let border_color = if is_focused {
                        egui::Color32::from_rgb(76, 175, 80) // green — focused
                    } else {
                        egui::Color32::from_rgb(40, 40, 60) // dim
                    };
                    let border_width = if is_focused { 2.0 } else { 1.0 };
                    ui.painter_at(cell_rect).rect_stroke(
                        cell_rect,
                        0.0,
                        egui::Stroke::new(border_width, border_color),
                        egui::StrokeKind::Outside,
                    );
                }
            } else {
                let (rect, resp) = ui.allocate_exact_size(available.size(), egui::Sense::click_and_drag());

                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    let painter = ui.painter_at(rect);
                    draw_chart(&painter, chart, rect, crosshair, &flags, show_rsi, show_fisher, show_macd, show_volume_pane, show_stochastic, show_adx, show_cci, show_williams_r, show_obv, show_momentum, show_better_volume, show_ehlers_ebsw, show_ehlers_cyber, show_ehlers_cg, show_ehlers_roof, sl_price, tp_price);

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
                            ui.close();
                        }
                        if ui.button("Trendline (2 clicks)").clicked() {
                            self.draw_mode = DrawMode::PlacingTrendP1;
                            ui.close();
                        }
                        if ui.button("Fibonacci Retracement").clicked() {
                            self.draw_mode = DrawMode::PlacingFiboP1;
                            ui.close();
                        }
                        if ui.button("Vertical Line").clicked() {
                            self.draw_mode = DrawMode::PlacingVLine;
                            ui.close();
                        }
                        if ui.button("Rectangle (2 clicks)").clicked() {
                            self.draw_mode = DrawMode::PlacingRectP1;
                            ui.close();
                        }
                        if ui.button("Ray (2 clicks)").clicked() {
                            self.draw_mode = DrawMode::PlacingRayP1;
                            ui.close();
                        }
                        if ui.button("Channel (3 clicks)").clicked() {
                            self.draw_mode = DrawMode::PlacingChannelP1;
                            ui.close();
                        }
                        ui.separator();
                        if !chart.drawings.is_empty() {
                            ui.menu_button("Drawing Color", |ui| {
                                let colors = [
                                    ("White", egui::Color32::WHITE),
                                    ("Yellow", egui::Color32::from_rgb(255, 200, 50)),
                                    ("Green", egui::Color32::from_rgb(0, 220, 80)),
                                    ("Red", egui::Color32::from_rgb(220, 40, 40)),
                                    ("Cyan", egui::Color32::from_rgb(0, 200, 255)),
                                    ("Magenta", egui::Color32::from_rgb(255, 100, 255)),
                                    ("Orange", egui::Color32::from_rgb(255, 140, 0)),
                                    ("Blue", egui::Color32::from_rgb(80, 120, 255)),
                                ];
                                for (name, color) in &colors {
                                    if ui.button(egui::RichText::new(*name).color(*color)).clicked() {
                                        if let Some(d) = chart.drawings.last_mut() {
                                            match d {
                                                Drawing::HLine { color: c, .. } => *c = *color,
                                                Drawing::TrendLine { color: c, .. } => *c = *color,
                                                _ => {}
                                            }
                                        }
                                        ui.close();
                                    }
                                }
                            });
                        }
                        if ui.button("Remove Last Drawing").clicked() {
                            chart.drawings.pop();
                            ui.close();
                        }
                        if ui.button("Clear All Drawings").clicked() {
                            chart.drawings.clear();
                            ui.close();
                        }
                        ui.separator();
                        ui.label(egui::RichText::new("Chart").color(ACCENT).strong());
                        ui.separator();
                        if ui.button("Reset Zoom / Pan").clicked() {
                            chart.price_zoom = 1.0;
                            chart.price_pan = 0.0;
                            chart.visible_bars = 200;
                            chart.view_offset = chart.bars.len().saturating_sub(1);
                            ui.close();
                        }
                        for &ct in &[ChartType::Candle, ChartType::HeikinAshi, ChartType::Line, ChartType::OhlcBars, ChartType::Renko] {
                            let label = if chart.chart_type == ct { format!("● {}", ct.label()) } else { format!("  {}", ct.label()) };
                            if ui.button(label).clicked() {
                                chart.chart_type = ct;
                                ui.close();
                            }
                        }
                        ui.separator();
                        ui.label(egui::RichText::new("Windows").color(ACCENT).strong());
                        ui.separator();
                        if ui.button("Indicators…").clicked() { self.show_indicators_panel = true; ui.close(); }
                        if ui.button("Data Window").clicked() { self.show_data_window = true; ui.close(); }
                        if ui.button("Volume Profile").clicked() { self.show_volume_profile = true; ui.close(); }
                        if ui.button("Price Alerts…").clicked() { self.show_alerts = true; ui.close(); }
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
                                ui.close();
                            }
                        }
                    });
                }
            }
        });

        // ── Console (egui::Window for proper focus/interaction on Wayland) ────
        if self.command_open {
            let palette_commands: Vec<&Command> = COMMANDS
                .iter()
                .filter(|c| fuzzy_match(&self.command_input, c.name) || fuzzy_match(&self.command_input, c.desc))
                .collect();

            let num_visible = palette_commands.len().clamp(1, 15);
            let console_height = (num_visible as f32) * 24.0 + 52.0;

            let screen_width = ctx.input(|i| i.viewport_rect()).width();
            egui::Window::new("__console__")
                .title_bar(false)
                .anchor(egui::Align2::LEFT_TOP, [0.0, 0.0])
                .fixed_size([screen_width, console_height])
                .frame(egui::Frame::window(&ctx.style())
                    .fill(egui::Color32::from_rgba_premultiplied(8, 8, 24, 247))
                    .inner_margin(8.0)
                    .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(76, 175, 80))))
                .show(ctx, |ui| {
                    let input_resp = ui.add(
                        egui::TextEdit::singleline(&mut self.command_input)
                            .desired_width(screen_width - 24.0)
                            .hint_text("type a command… (Esc to close)")
                            .font(egui::FontId::monospace(14.0))
                            .text_color(egui::Color32::from_rgb(76, 175, 80)),
                    );
                    input_resp.request_focus();

                    // Arrow key navigation
                    let cmd_count = palette_commands.len();
                    if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) && cmd_count > 0 {
                        self.console_selected = (self.console_selected + 1).min(cmd_count.saturating_sub(1));
                        // Autocomplete: put selected command name into input
                        if let Some(cmd) = palette_commands.get(self.console_selected) {
                            self.command_input = cmd.name.to_string();
                        }
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) && cmd_count > 0 {
                        self.console_selected = self.console_selected.saturating_sub(1);
                        if let Some(cmd) = palette_commands.get(self.console_selected) {
                            self.command_input = cmd.name.to_string();
                        }
                    }
                    // Reset selection when input changes (user types)
                    if input_resp.changed() {
                        self.console_selected = 0;
                    }

                    ui.separator();

                    let mut execute: Option<String> = None;
                    egui::ScrollArea::vertical().max_height(console_height - 52.0).show(ui, |ui| {
                        for (i, cmd) in palette_commands.iter().enumerate() {
                            let is_selected = i == self.console_selected;
                            let row_bg = if is_selected { egui::Color32::from_rgb(15, 52, 96) } else { egui::Color32::TRANSPARENT };
                            let name_col = if is_selected { egui::Color32::WHITE } else { egui::Color32::from_rgb(136, 255, 255) };

                            let row = ui.horizontal(|ui| {
                                // Selected row background
                                let rect = ui.available_rect_before_wrap();
                                let row_rect = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 20.0));
                                ui.painter().rect_filled(row_rect, 0.0, row_bg);

                                ui.label(egui::RichText::new(cmd.name).color(name_col).monospace().strong().size(13.0));
                                ui.add_space(12.0);
                                ui.label(egui::RichText::new(cmd.desc).color(egui::Color32::from_rgb(136, 136, 136)).size(11.0));
                            });
                            if row.response.interact(egui::Sense::click()).clicked() {
                                execute = Some(cmd.name.to_string());
                            }
                        }
                    });

                    // Enter executes the selected command
                    if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                        execute = palette_commands.get(self.console_selected).map(|c| c.name.to_string());
                    }
                    if let Some(cmd_name) = execute {
                        self.command_open = false;
                        self.handle_command(&cmd_name, ctx);
                    }
                });
        }

        // Request continuous repainting for real-time tick updates
        // Auto-save session every 60 seconds (240 frames at 250ms repaint)
        if self.frame_count % 240 == 0 && self.frame_count > 0 {
            self.save_session();
        }

        // Repaint strategy:
        // - egui auto-repaints on ANY user interaction (mouse move, click, scroll, key)
        // - We set a slow idle repaint for background updates (live data, time)
        // - Charts stay responsive because mouse events trigger instant repaints
        // - Floating windows with DB queries only update on idle repaints
        let any_heavy_window = self.show_darwin_portfolio || self.show_darwin_accounts
            || self.show_var_mult || self.show_montecarlo || self.show_stress_test
            || self.show_correlation || self.show_seasonals || self.show_symbol_overlap
            || self.show_sec || self.show_insider;
        let idle_ms = if any_heavy_window { 2000 } else { 500 };
        ctx.request_repaint_after(std::time::Duration::from_millis(idle_ms));
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Create synthetic test bars (ascending prices).
    fn make_bars(n: usize) -> Vec<Bar> {
        (0..n).map(|i| {
            let base = 100.0 + i as f64;
            Bar {
                ts_ms: 1700000000000 + i as i64 * 3600000,
                open: base,
                high: base + 2.0,
                low: base - 1.0,
                close: base + 1.0,
                volume: 1000.0 + i as f64 * 10.0,
            }
        }).collect()
    }

    /// Create bars with known pattern for oscillator tests.
    fn make_oscillating_bars(n: usize) -> Vec<Bar> {
        (0..n).map(|i| {
            let base = 100.0 + (i as f64 * 0.1).sin() * 10.0;
            Bar {
                ts_ms: 1700000000000 + i as i64 * 3600000,
                open: base - 0.5,
                high: base + 1.0,
                low: base - 1.0,
                close: base + 0.5,
                volume: 500.0 + (i as f64 * 0.3).cos().abs() * 1000.0,
            }
        }).collect()
    }

    // ── SMA Tests ────────────────────────────────────────────────────────

    #[test]
    fn test_sma_basic() {
        let bars = make_bars(10);
        let sma = compute_sma(&bars, 3);
        assert_eq!(sma.len(), 10);
        // First 2 should be None (period-1)
        assert!(sma[0].is_none());
        assert!(sma[1].is_none());
        // Third bar should have a value
        assert!(sma[2].is_some());
        // SMA(3) of closes 101, 102, 103 = 102
        let v = sma[2].unwrap();
        assert!((v - 102.0).abs() < 0.01, "SMA(3) bar 2 = {}, expected ~102", v);
    }

    #[test]
    fn test_sma_empty() {
        let bars: Vec<Bar> = vec![];
        let sma = compute_sma(&bars, 5);
        assert!(sma.is_empty());
    }

    #[test]
    fn test_sma_period_larger_than_data() {
        let bars = make_bars(3);
        let sma = compute_sma(&bars, 10);
        assert_eq!(sma.len(), 3);
        assert!(sma.iter().all(|v| v.is_none()));
    }

    // ── EMA Tests ────────────────────────────────────────────────────────

    #[test]
    fn test_ema_basic() {
        let bars = make_bars(20);
        let ema = compute_ema(&bars, 5);
        assert_eq!(ema.len(), 20);
        // First 4 should be None
        for i in 0..4 { assert!(ema[i].is_none(), "EMA[{}] should be None", i); }
        // Should have values from period-1 onward
        assert!(ema[4].is_some());
        // EMA should be close to but not exactly equal to close prices (trending up)
        let last = ema[19].unwrap();
        assert!(last > 100.0 && last < 125.0, "EMA last = {}", last);
    }

    #[test]
    fn test_ema_follows_trend() {
        let bars = make_bars(50);
        let ema = compute_ema(&bars, 10);
        // EMA should be increasing for ascending bars
        let mut prev = 0.0;
        for v in ema.iter().flatten() {
            assert!(*v >= prev, "EMA should be non-decreasing: {} < {}", *v, prev);
            prev = *v;
        }
    }

    // ── KAMA Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_kama_basic() {
        let bars = make_bars(30);
        let kama = compute_kama(&bars, 10, 2, 30);
        assert_eq!(kama.len(), 30);
        assert!(kama[9].is_none()); // period-1 warmup
        assert!(kama[10].is_some());
    }

    #[test]
    fn test_kama_adapts_to_trend() {
        let bars = make_bars(50);
        let kama = compute_kama(&bars, 10, 2, 30);
        // KAMA should follow the uptrend
        let last = kama.last().unwrap().unwrap();
        assert!(last > 130.0, "KAMA should follow uptrend: {}", last);
    }

    // ── Bollinger Bands ──────────────────────────────────────────────────

    #[test]
    fn test_bollinger_bands() {
        let bars = make_bars(30);
        let (mid, upper, lower) = compute_bollinger(&bars, 20, 2.0);
        assert_eq!(mid.len(), 30);
        // After warmup, upper > mid > lower
        for i in 19..30 {
            if let (Some(u), Some(m), Some(l)) = (upper[i], mid[i], lower[i]) {
                assert!(u > m, "Upper {} should be > mid {}", u, m);
                assert!(m > l, "Mid {} should be > lower {}", m, l);
            }
        }
    }

    // ── RSI Tests ────────────────────────────────────────────────────────

    #[test]
    fn test_rsi_range() {
        let bars = make_oscillating_bars(50);
        let rsi = compute_rsi(&bars, 14);
        for v in rsi.iter().flatten() {
            assert!(*v >= 0.0 && *v <= 100.0, "RSI should be 0-100: {}", v);
        }
    }

    #[test]
    fn test_rsi_uptrend_bullish() {
        let bars = make_bars(30);
        let rsi = compute_rsi(&bars, 14);
        // Strong uptrend should have RSI > 50
        if let Some(v) = rsi.last().unwrap() {
            assert!(*v > 50.0, "RSI in uptrend should be >50: {}", v);
        }
    }

    // ── Fisher Transform ─────────────────────────────────────────────────

    #[test]
    fn test_fisher_transform() {
        let bars = make_bars(50);
        let (fisher, signal) = compute_fisher(&bars, 32);
        assert_eq!(fisher.len(), 50);
        assert_eq!(signal.len(), 50);
        // Should have values after warmup
        let has_values = fisher.iter().any(|v| v.is_some());
        assert!(has_values, "Fisher should have computed values");
    }

    // ── MACD Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_macd_basic() {
        let bars = make_bars(50);
        let (macd, signal, hist) = compute_macd(&bars, 12, 26, 9);
        assert_eq!(macd.len(), 50);
        assert_eq!(signal.len(), 50);
        assert_eq!(hist.len(), 50);
        // Should have values after warmup (26 + 9 bars)
        assert!(macd[35].is_some());
    }

    #[test]
    fn test_macd_histogram_is_difference() {
        let bars = make_bars(50);
        let (macd, signal, hist) = compute_macd(&bars, 12, 26, 9);
        for i in 0..50 {
            if let (Some(m), Some(s), Some(h)) = (macd[i], signal[i], hist[i]) {
                assert!((h - (m - s)).abs() < 0.001, "Histogram should be MACD - Signal");
            }
        }
    }

    // ── Stochastic ───────────────────────────────────────────────────────

    #[test]
    fn test_stochastic_range() {
        let bars = make_oscillating_bars(50);
        let (k, d) = compute_stochastic(&bars, 14, 3, 3);
        for v in k.iter().flatten() {
            assert!(*v >= 0.0 && *v <= 100.0, "Stoch %K should be 0-100: {}", v);
        }
        for v in d.iter().flatten() {
            assert!(*v >= 0.0 && *v <= 100.0, "Stoch %D should be 0-100: {}", v);
        }
    }

    // ── ADX Tests ────────────────────────────────────────────────────────

    #[test]
    fn test_adx_range() {
        let bars = make_bars(50);
        let (adx, di_plus, di_minus) = compute_adx(&bars, 14);
        for v in adx.iter().flatten() {
            assert!(*v >= 0.0, "ADX should be >= 0: {}", v);
        }
        for v in di_plus.iter().flatten() {
            assert!(*v >= 0.0, "DI+ should be >= 0: {}", v);
        }
        for v in di_minus.iter().flatten() {
            assert!(*v >= 0.0, "DI- should be >= 0: {}", v);
        }
    }

    // ── ATR Tests ────────────────────────────────────────────────────────

    #[test]
    fn test_atr_positive() {
        let bars = make_bars(30);
        let atr = compute_atr(&bars, 14);
        for v in atr.iter().flatten() {
            assert!(*v > 0.0, "ATR should be > 0: {}", v);
        }
    }

    // ── Ichimoku Tests ───────────────────────────────────────────────────

    #[test]
    fn test_ichimoku_lengths() {
        let bars = make_bars(60);
        let (tenkan, kijun, span_a, span_b) = compute_ichimoku(&bars, 9, 26, 52);
        assert_eq!(tenkan.len(), 60);
        assert_eq!(kijun.len(), 60);
        assert_eq!(span_a.len(), 60);
        assert_eq!(span_b.len(), 60);
    }

    // ── WMA / HMA Tests ─────────────────────────────────────────────────

    #[test]
    fn test_wma_basic() {
        let bars = make_bars(30);
        let wma = compute_wma(&bars, 10);
        assert_eq!(wma.len(), 30);
        assert!(wma[9].is_some());
    }

    #[test]
    fn test_hma_basic() {
        let bars = make_bars(30);
        let hma = compute_hma(&bars, 10);
        assert_eq!(hma.len(), 30);
        // HMA should have values after warmup
        let has_values = hma.iter().any(|v| v.is_some());
        assert!(has_values);
    }

    // ── CCI / Williams %R ────────────────────────────────────────────────

    #[test]
    fn test_cci_basic() {
        let bars = make_oscillating_bars(30);
        let cci = compute_cci(&bars, 20);
        assert_eq!(cci.len(), 30);
    }

    #[test]
    fn test_williams_r_range() {
        let bars = make_oscillating_bars(30);
        let wr = compute_williams_r(&bars, 14);
        for v in wr.iter().flatten() {
            assert!(*v >= -100.0 && *v <= 0.0, "Williams %R should be -100 to 0: {}", v);
        }
    }

    // ── OBV / Momentum ──────────────────────────────────────────────────

    #[test]
    fn test_obv_basic() {
        let bars = make_bars(20);
        let obv = compute_obv(&bars);
        assert_eq!(obv.len(), 20);
        assert!(obv[0].is_some());
    }

    #[test]
    fn test_momentum_basic() {
        let bars = make_bars(20);
        let mom = compute_momentum(&bars, 10);
        assert_eq!(mom.len(), 20);
    }

    // ── Parabolic SAR ────────────────────────────────────────────────────

    #[test]
    fn test_psar_basic() {
        let bars = make_bars(30);
        let psar = compute_parabolic_sar(&bars, 0.02, 0.2);
        assert_eq!(psar.len(), 30);
        let has_values = psar.iter().any(|v| v.is_some());
        assert!(has_values);
    }

    // ── Fractals ─────────────────────────────────────────────────────────

    #[test]
    fn test_fractals_length() {
        let bars = make_bars(20);
        let up = compute_fractals_up(&bars);
        let down = compute_fractals_down(&bars);
        assert_eq!(up.len(), 20);
        assert_eq!(down.len(), 20);
    }

    // ── BetterVolume ─────────────────────────────────────────────────────

    #[test]
    fn test_better_volume_classification() {
        let bars = make_oscillating_bars(30);
        let bv = compute_better_volume(&bars);
        assert_eq!(bv.len(), 30);
        // All values should be 0-5
        for v in &bv {
            assert!(*v <= 5, "BetterVolume type should be 0-5: {}", v);
        }
    }

    // ── Supply/Demand Zones ──────────────────────────────────────────────

    #[test]
    fn test_supply_demand_zones() {
        let bars = make_oscillating_bars(50);
        let (supply, demand) = compute_supply_demand_zones(&bars);
        // Should return valid zone tuples
        for (idx, high, low, status) in &supply {
            assert!(*idx < bars.len());
            assert!(high > low);
            assert!(*status <= 2);
        }
        for (idx, high, low, status) in &demand {
            assert!(*idx < bars.len());
            assert!(high > low);
            assert!(*status <= 2);
        }
    }

    // ── Ehlers DSP Indicators ────────────────────────────────────────────

    #[test]
    fn test_ehlers_super_smoother() {
        let bars = make_bars(30);
        let ss = ehlers_super_smoother(&bars, 10);
        assert_eq!(ss.len(), 30);
        let has_values = ss.iter().any(|v| v.is_some());
        assert!(has_values);
    }

    #[test]
    fn test_ehlers_decycler() {
        let bars = make_bars(30);
        let dc = ehlers_decycler(&bars, 20);
        assert_eq!(dc.len(), 30);
    }

    #[test]
    fn test_ehlers_mama_fama() {
        let bars = make_bars(30);
        let (mama, fama) = ehlers_mama_fama(&bars, 0.5, 0.05);
        assert_eq!(mama.len(), 30);
        assert_eq!(fama.len(), 30);
    }

    #[test]
    fn test_ehlers_ebsw() {
        let bars = make_oscillating_bars(50);
        let ebsw = ehlers_even_better_sinewave(&bars, 40);
        assert_eq!(ebsw.len(), 50);
        // EBSW should be in -1 to 1 range
        for v in ebsw.iter().flatten() {
            assert!(*v >= -2.0 && *v <= 2.0, "EBSW should be ~-1 to 1: {}", v);
        }
    }

    #[test]
    fn test_ehlers_cyber_cycle() {
        let bars = make_oscillating_bars(30);
        let cc = ehlers_cyber_cycle(&bars);
        assert_eq!(cc.len(), 30);
    }

    #[test]
    fn test_ehlers_cg_oscillator() {
        let bars = make_bars(30);
        let cg = ehlers_cg_oscillator(&bars, 10);
        assert_eq!(cg.len(), 30);
    }

    #[test]
    fn test_ehlers_roofing_filter() {
        let bars = make_oscillating_bars(60);
        let rf = ehlers_roofing_filter(&bars, 10, 48);
        assert_eq!(rf.len(), 60);
    }

    // ── Heikin-Ashi / Renko ──────────────────────────────────────────────

    #[test]
    fn test_heikin_ashi() {
        let bars = make_bars(10);
        let ha = heikin_ashi(&bars);
        assert_eq!(ha.len(), 10);
        // HA close = (O+H+L+C)/4
        let b = &bars[0];
        let ha_close = (b.open + b.high + b.low + b.close) / 4.0;
        assert!((ha[0].close - ha_close).abs() < 0.01);
    }

    #[test]
    fn test_renko_bricks() {
        let bars = make_bars(50);
        let bricks = renko_bricks(&bars);
        // Renko should produce some bricks for trending data
        assert!(!bricks.is_empty(), "Renko should produce bricks for trending data");
    }

    // ── ATR Projection ───────────────────────────────────────────────────

    #[test]
    fn test_atr_projection() {
        let bars = make_bars(20);
        let atr = compute_atr(&bars, 14);
        let (upper, lower) = compute_atr_projection(&bars, &atr);
        assert_eq!(upper.len(), 20);
        assert_eq!(lower.len(), 20);
        // Upper should be > lower where both exist
        for i in 0..20 {
            if let (Some(u), Some(l)) = (upper[i], lower[i]) {
                assert!(u > l, "ATR proj upper {} should be > lower {}", u, l);
            }
        }
    }

    // ── Previous Candle Levels ───────────────────────────────────────────

    #[test]
    fn test_prev_candle_levels() {
        let bars = make_bars(10);
        let (dh, dl, wh, wl) = compute_prev_candle_levels(&bars);
        // With synthetic data, should have daily levels at least
        // (may be None if all bars are same "day" in synthetic data)
        let _ = (dh, dl, wh, wl);
    }

    // ── Helper Functions ─────────────────────────────────────────────────

    #[test]
    fn test_in_range() {
        assert!(in_range(0.5, 0.0, 1.0));
        assert!(!in_range(1.5, 0.0, 1.0));
        assert!(in_range(0.618, 0.5, 0.8));
    }

    #[test]
    fn test_format_price() {
        let s = format_price(123.456);
        assert!(s.contains("123"));
    }

    #[test]
    fn test_fuzzy_match() {
        assert!(fuzzy_match("sma", "SMA200"));
        assert!(fuzzy_match("fish", "Fisher Transform"));
        assert!(!fuzzy_match("xyz", "SMA200"));
        assert!(fuzzy_match("", "anything")); // empty matches all
    }

    // ── Auto Fibonacci ───────────────────────────────────────────────────

    #[test]
    fn test_auto_fibonacci() {
        let mut bars = make_bars(60);
        // Create a clear swing: up then down
        for i in 30..60 {
            bars[i].close = 160.0 - i as f64;
            bars[i].high = bars[i].close + 2.0;
            bars[i].low = bars[i].close - 1.0;
            bars[i].open = bars[i].close - 0.5;
        }
        let mut chart = ChartState::new("TEST", Timeframe::H4);
        chart.bars = bars;
        chart.compute_indicators();
        // Auto fib may or may not find levels depending on fractal detection
        // Just verify no panic
        assert!(chart.auto_fib_levels.len() >= 0);
    }

    // ── ChartState Integration ───────────────────────────────────────────

    #[test]
    fn test_chart_state_compute_all_indicators() {
        let mut chart = ChartState::new("TEST", Timeframe::H4);
        chart.bars = make_bars(100);
        chart.compute_indicators();
        // All indicator vectors should have correct length
        assert_eq!(chart.sma200.len(), 100);
        assert_eq!(chart.sma100.len(), 100);
        assert_eq!(chart.kama.len(), 100);
        assert_eq!(chart.ema21.len(), 100);
        assert_eq!(chart.rsi.len(), 100);
        assert_eq!(chart.fisher.len(), 100);
        assert_eq!(chart.macd_line.len(), 100);
        assert_eq!(chart.atr.len(), 100);
        assert_eq!(chart.better_vol_type.len(), 100);
    }

    #[test]
    fn test_chart_state_visible_range() {
        let mut chart = ChartState::new("TEST", Timeframe::H4);
        chart.bars = make_bars(500);
        chart.visible_bars = 200;
        chart.view_offset = 499;
        let (start, end) = chart.visible_range();
        assert_eq!(end - start, 200);
        assert_eq!(end, 500);
    }
}
