//! `ChartState` — all state for one chart viewport: symbol/timeframe, raw + compare
//! bars, the full pre-computed indicator series, live-quote/forming-bar state, camera,
//! and drawing/overlay collections (ADR-125 Target 2, slice 6c). Pure data + chart-local
//! behavior (symbol switch, live-quote folding, forming-bar updates, the `new` builder)
//! over crate types + engine DTOs + egui — no `TyphooNApp`, no broker/cache/gpu glue
//! (that stays in typhoon-native as extension traits: `ChartDataLoad`, `ChartIndicatorCompute`,
//! `ChartMtfOverlays`, `ChartSymbolMatch`).

use crate::drawing::{Drawing, LineStyle, TradeOverlay};
use crate::indicators::HarmonicPattern;
use crate::models::ChartCamera;
use crate::types::{Bar, ChartType, Timeframe};

/// All state for one chart viewport.
pub struct ChartState {
    /// The symbol string shown in the toolbar.
    pub symbol: String,
    /// Currently selected timeframe.
    pub timeframe: Timeframe,
    /// Cache source that most recently populated `bars` ("kraken", "alpaca", etc).
    pub primary_source: &'static str,
    /// User-forced chart source for debugging provider-specific cache rows.
    /// Empty means automatic source selection.
    pub source_override: &'static str,
    /// Active symbol-level regulatory warnings shown next to the chart symbol.
    pub regulatory_alerts: Vec<typhoon_engine::core::regulatory_alerts::RegulatoryAlert>,
    /// Chart rendering style.
    pub chart_type: ChartType,
    /// Whether this chart appears in the user-facing top tab strip. MTF Grid
    /// backing charts are real chart states for data/indicator computation, but
    /// they must not pollute normal user-opened tabs.
    pub show_in_tab_bar: bool,
    /// Logarithmic price scale (vs linear).
    pub log_scale: bool,
    // ── Configurable indicator periods ─────────────────────────────────
    pub sma_slow_period: u32,
    pub sma_fast_period: u32,
    pub ema_period: u32,
    pub rsi_period: u32,
    pub atr_period: u32,
    pub bb_period: u32,
    pub macd_fast: u32,
    pub macd_slow: u32,
    pub macd_signal_p: u32,
    pub stoch_period: u32,
    pub adx_period: u32,
    pub fisher_period: u32,
    pub momentum_period: u32,
    // ── Compare symbol overlay ─────────────────────────────────────────
    pub compare_symbol: Option<String>,
    pub compare_bars: Vec<Bar>,
    /// Live bid/ask from streaming quotes (for spread line rendering).
    pub live_bid: f64,
    pub live_ask: f64,
    /// Live sizes from rich L1 (Alpaca quotes, Kraken ticker/book).
    pub live_bid_size: f64,
    pub live_ask_size: f64,
    /// Top N live depth levels from L2/L3 book for binned depth profile overlay.
    /// (price, size). Populated from Kraken book updates when available.
    pub live_depth_bids: Vec<(f64, f64)>,
    pub live_depth_asks: Vec<(f64, f64)>,
    /// When `live_bid`/`live_ask` were last refreshed from a streaming quote.
    /// The spread lines are hidden once this goes stale so a frozen quote isn't
    /// drawn next to a live (differently-priced) last/candle.
    pub live_quote_at: Option<std::time::Instant>,
    /// `true` when the most recent `live_bid`/`live_ask` came from a *delayed*
    /// source (the Kraken iapi equity ticker, ~15 min). Lets a real-time WS quote
    /// take precedence so the chart spread/last don't flip to the delayed snapshot
    /// while live ticks are flowing.
    pub live_quote_delayed: bool,
    // Extended hours candle (pre/post market)
    pub ext_open: f64,
    pub ext_high: f64,
    pub ext_low: f64,
    pub ext_close: f64,
    pub ext_active: bool, // true when ext hours data is available
    /// Authoritative previous-day regular close (Alpaca `prevDailyBar.c` /
    /// Yahoo `regularMarketPreviousClose`), sourced from the shared watchlist
    /// quote so it is timeframe-independent. `0.0` when unknown. Drives the
    /// extended-hours badge "Day %" so a W1/MN chart shows the day move, not a
    /// week/month-ago comparison from its own previous bar.
    pub prev_daily_close: f64,
    /// Raw bar data loaded from cache.
    pub bars: Vec<Bar>,
    /// Reusable buffers for full GPU upload path (avoids repeated allocations).
    #[allow(dead_code)]
    pub upload_opens: Vec<f32>,
    #[allow(dead_code)]
    pub upload_closes: Vec<f32>,
    #[allow(dead_code)]
    pub upload_highs: Vec<f32>,
    #[allow(dead_code)]
    pub upload_lows: Vec<f32>,
    #[allow(dead_code)]
    pub upload_volumes: Vec<f32>,
    /// Pre-computed SMA(200) — indexed parallel to `bars`.
    pub sma200: Vec<Option<f64>>,
    /// Pre-computed SMA(100) — indexed parallel to `bars`.
    pub sma100: Vec<Option<f64>>,
    /// Pre-computed KAMA(10,2,30) — indexed parallel to `bars`.
    pub kama: Vec<Option<f64>>,
    /// Pre-computed EMA(21).
    pub ema21: Vec<Option<f64>>,
    /// Bollinger Bands (middle, upper, lower).
    pub bb_mid: Vec<Option<f64>>,
    pub bb_upper: Vec<Option<f64>>,
    pub bb_lower: Vec<Option<f64>>,
    /// RSI(14) — 0..100 range.
    pub rsi: Vec<Option<f64>>,
    /// Fisher Transform.
    pub fisher: Vec<Option<f64>>,
    pub fisher_signal: Vec<Option<f64>>,
    /// ATR(14).
    pub atr: Vec<Option<f64>>,
    /// MACD(12,26,9).
    pub macd_line: Vec<Option<f64>>,
    pub macd_signal: Vec<Option<f64>>,
    pub macd_hist: Vec<Option<f64>>,
    /// Stochastic(14,3,3).
    pub stoch_k: Vec<Option<f64>>,
    pub stoch_d: Vec<Option<f64>>,
    /// ADX(14) + DI+/DI-.
    pub adx: Vec<Option<f64>>,
    pub di_plus: Vec<Option<f64>>,
    pub di_minus: Vec<Option<f64>>,
    /// Ichimoku(9,26,52).
    pub ichi_tenkan: Vec<Option<f64>>,
    pub ichi_kijun: Vec<Option<f64>>,
    pub ichi_span_a: Vec<Option<f64>>,
    pub ichi_span_b: Vec<Option<f64>>,
    /// Previous candle levels (daily high/low).
    pub prev_daily_high: Option<f64>,
    pub prev_daily_low: Option<f64>,
    pub prev_weekly_high: Option<f64>,
    pub prev_weekly_low: Option<f64>,
    pub prev_h4_high: Option<f64>,
    pub prev_h4_low: Option<f64>,
    pub prev_h1_high: Option<f64>,
    pub prev_h1_low: Option<f64>,
    pub prev_monthly_high: Option<f64>,
    pub prev_monthly_low: Option<f64>,
    // Current ("Judas") candle levels — the forming D1/W1/MN1 period high/low,
    // drawn alongside the previous-candle levels (PreviousCandleLevels.mqh).
    pub current_daily_high: Option<f64>,
    pub current_daily_low: Option<f64>,
    pub current_weekly_high: Option<f64>,
    pub current_weekly_low: Option<f64>,
    pub current_monthly_high: Option<f64>,
    pub current_monthly_low: Option<f64>,
    /// WMA(20), HMA(20).
    pub wma: Vec<Option<f64>>,
    pub hma: Vec<Option<f64>>,
    /// CCI(20), Williams %R(14).
    pub cci: Vec<Option<f64>>,
    pub williams_r: Vec<Option<f64>>,
    /// OBV.
    pub obv: Vec<Option<f64>>,
    /// Momentum(10).
    pub momentum: Vec<Option<f64>>,
    /// CMO(9).
    pub cmo: Vec<Option<f64>>,
    /// Running sums for O(1) CMO forming-bar update
    pub cmo_sum_up: f64,
    pub cmo_sum_down: f64,
    /// Running sums for O(1) Linear Regression Slope
    pub linreg_sum_x: f64,
    pub linreg_sum_y: f64,
    pub linreg_sum_xy: f64,
    pub linreg_sum_x2: f64,
    /// QStick(14).
    pub qstick: Vec<Option<f64>>,
    /// Disparity Index(14).
    pub disparity: Vec<Option<f64>>,
    /// LINEARREG_SLOPE
    pub linreg_slope: Vec<Option<f64>>,
    /// LINEARREG_INTERCEPT
    pub linreg_intercept: Vec<Option<f64>>,
    /// LINEARREG_ANGLE
    #[allow(dead_code)]
    pub linreg_angle: Vec<Option<f64>>,
    /// LINEARREG (endpoint value)
    #[allow(dead_code)]
    pub linreg: Vec<Option<f64>>,
    /// BOP(14).
    pub bop: Vec<Option<f64>>,
    /// StdDev(20).
    pub stddev: Vec<Option<f64>>,
    /// MFI(14).
    pub mfi: Vec<Option<f64>>,
    /// TRIX(15,9).
    pub trix_line: Vec<Option<f64>>,
    pub trix_signal: Vec<Option<f64>>,
    pub trix_hist: Vec<Option<f64>>,
    /// PPO(12,26,9).
    pub ppo_line: Vec<Option<f64>>,
    pub ppo_signal: Vec<Option<f64>>,
    pub ppo_hist: Vec<Option<f64>>,
    /// Ultimate Oscillator(7,14,28).
    pub ultosc: Vec<Option<f64>>,
    /// StochRSI(14,14,3,3).
    pub stochrsi_k: Vec<Option<f64>>,
    pub stochrsi_d: Vec<Option<f64>>,
    /// VaR oscillator (20-bar rolling parametric VaR, 95%).
    pub var_oscillator: Vec<Option<f64>>,
    /// Parabolic SAR(0.02, 0.2).
    pub psar: Vec<Option<f64>>,
    /// MTF SMA lines — (label, color_idx, projected_points) matching MTF_MA.mqh.
    /// H1/200, H4/200, D1/200, W1/200, W1/100, MN1/100
    pub mtf_sma: Vec<(String, Vec<(usize, f64)>)>,
    /// ATR Projection (open ± ATR bands — legacy per-bar, kept for GPU path).
    pub atr_proj_upper: Vec<Option<f64>>,
    pub atr_proj_lower: Vec<Option<f64>>,
    /// ATR Projection MTF levels (label, open, atr_value, start_bar_idx) — matches ATR_Projection.mqh.
    pub atr_proj_levels: Vec<(&'static str, f64, f64, usize)>,
    /// Better Volume classification.
    pub better_vol_type: Vec<u8>, // 0=normal, 1=climax_up, 2=climax_dn, 3=high, 4=low, 5=churn
    /// Pivot points (computed from daily data).
    pub pivot_p: Option<f64>,
    pub pivot_r1: Option<f64>,
    pub pivot_r2: Option<f64>,
    pub pivot_s1: Option<f64>,
    pub pivot_s2: Option<f64>,
    /// Bill Williams Fractals (up/down arrows).
    pub fractal_up: Vec<bool>,
    pub fractal_down: Vec<bool>,
    // ── Ehlers indicators ──────────────────────────────────────────────
    /// Super Smoother (overlay).
    pub ehlers_ss: Vec<Option<f64>>,
    /// Decycler (overlay).
    pub ehlers_decycler: Vec<Option<f64>>,
    /// Instantaneous Trendline (overlay).
    pub ehlers_itl: Vec<Option<f64>>,
    /// MAMA (overlay).
    pub ehlers_mama: Vec<Option<f64>>,
    /// FAMA (overlay).
    pub ehlers_fama: Vec<Option<f64>>,
    /// Even Better Sinewave (sub-pane, -1 to 1).
    pub ehlers_ebsw: Vec<Option<f64>>,
    /// Cyber Cycle (sub-pane).
    pub ehlers_cyber: Vec<Option<f64>>,
    /// CG Oscillator (sub-pane).
    pub ehlers_cg: Vec<Option<f64>>,
    /// Roofing Filter (sub-pane).
    pub ehlers_roof: Vec<Option<f64>>,
    /// Supply/demand zones: (bar_idx, zone_high, zone_low, status).
    /// Status: 0=untested, 1=tested (price returned), 2=proven (price bounced)
    pub supply_zones: Vec<(usize, f64, f64, u8)>,
    /// Earliest timestamp from primary data source (Kraken/Alpaca). Bars before this are backfill.
    pub primary_first_ts: i64,
    /// Timestamps of bars sourced from gap-fill (Kraken). Colored magenta.
    pub gap_fill_timestamps: std::collections::HashSet<i64>,
    pub demand_zones: Vec<(usize, f64, f64, u8)>,
    /// Auto Fibonacci levels: (price, label, is_extension).
    pub auto_fib_levels: Vec<(f64, String, bool)>,
    /// Auto Fibonacci swing: (swing_high_price, swing_low_price, swing_high_idx, swing_low_idx).
    pub auto_fib_swing: Option<(f64, f64, usize, usize)>,
    /// VWAP (volume-weighted average price), anchored daily.
    pub vwap: Vec<Option<f64>>,
    /// VWAP upper deviation bands (1σ, 2σ, 3σ).
    pub vwap_upper1: Vec<Option<f64>>,
    pub vwap_upper2: Vec<Option<f64>>,
    pub vwap_upper3: Vec<Option<f64>>,
    /// VWAP lower deviation bands (1σ, 2σ, 3σ).
    pub vwap_lower1: Vec<Option<f64>>,
    pub vwap_lower2: Vec<Option<f64>>,
    pub vwap_lower3: Vec<Option<f64>>,
    /// Supertrend line (ATR-based trend bands, single line that flips).
    pub supertrend: Vec<Option<f64>>,
    /// Supertrend direction: true = bullish (below price), false = bearish (above).
    pub supertrend_bull: Vec<bool>,
    /// Donchian Channel upper (highest high over N bars).
    pub donchian_upper: Vec<Option<f64>>,
    /// Donchian Channel lower (lowest low over N bars).
    pub donchian_lower: Vec<Option<f64>>,
    /// Keltner Channel middle (EMA).
    pub keltner_mid: Vec<Option<f64>>,
    /// Keltner Channel upper (EMA + ATR×mult).
    pub keltner_upper: Vec<Option<f64>>,
    /// Keltner Channel lower (EMA - ATR×mult).
    pub keltner_lower: Vec<Option<f64>>,
    /// Regression Channel middle (regression line).
    pub regression_mid: Vec<Option<f64>>,
    /// Regression Channel upper (regression + 2σ).
    pub regression_upper: Vec<Option<f64>>,
    /// Regression Channel lower (regression - 2σ).
    pub regression_lower: Vec<Option<f64>>,
    /// Squeeze Momentum: momentum histogram value.
    pub squeeze_mom: Vec<Option<f64>>,
    /// Squeeze state: true = in squeeze (BB inside KC).
    pub squeeze_on: Vec<bool>,
    /// Pre-computed 20-bar rolling average volume (for volume heatmap candle coloring).
    pub vol_avg_20: Vec<f64>,
    /// MultiKAMA: KAMA values from higher timeframes projected onto this chart's x-axis.
    /// Each entry: (timeframe_label, Vec of (bar_index_in_this_chart, kama_value))
    pub multi_kama: Vec<(String, Vec<(usize, f64)>)>,
    /// Detected harmonic patterns.
    pub harmonics: Vec<HarmonicPattern>,
    /// Drawing annotations.
    pub drawings: Vec<Drawing>,
    /// O(1) for HLine dedup (price key) during sync (replaces .any).
    pub hline_set: std::collections::HashSet<String>,
    /// O(1) for FiboRetrace dedup (key by high/low/bars) during cross-TF sync (beyond HLine).
    pub fibo_set: std::collections::HashSet<String>,
    /// O(1) for VLine dedup (bar_idx key) during sync.
    pub vline_set: std::collections::HashSet<String>,
    /// O(1) for harmonics dedup (name + key points) for future cross-TF or add paths.
    #[allow(dead_code)]
    pub harmonic_set: std::collections::HashSet<String>,
    /// Per-drawing style: (line_width, line_style). Indexed parallel to `drawings`.
    pub drawing_styles: Vec<(f32, LineStyle)>,
    /// Undo stack for drawings (Ctrl+Z pops from drawings into here, Ctrl+Shift+Z restores)
    pub drawings_undo: Vec<Drawing>,
    pub selected_drawing: Option<usize>,
    /// Index of the control point being dragged (for resize). None = whole-drawing drag.
    pub dragging_cp: Option<usize>,
    /// True while dragging a selected drawing (suppresses chart pan).
    pub is_drawing_drag: bool,
    /// Cached trade overlay (rebuilt when bg data changes, not every frame).
    pub cached_trade_overlay: TradeOverlay,
    pub cached_trade_overlay_frame: u64,

    // ── view state ────────────────────────────────────────────────────────
    /// How many bars are visible horizontally (zoom level).
    pub visible_bars: usize,
    /// Index of the right-most visible bar (0 = oldest, len-1 = newest).
    pub view_offset: usize,
    /// True after the user manually pans/zooms the chart away from the auto-follow view.
    /// Cache reloads must preserve this viewport instead of snapping back to latest.
    pub manual_view_override: bool,
    /// Canonical TradingView-style camera. Legacy viewport fields above are kept
    /// in sync for draw/overlay code until the renderer consumes the camera directly.
    pub camera: ChartCamera,
    /// When replay mode is active, cap visible_range end at this bar index.
    pub replay_bar_cap: Option<usize>,
    /// Fractional price offset for vertical pan.
    pub price_pan: f64,
    /// Multiplier applied to the natural price range for vertical zoom.
    pub price_zoom: f64,

    // ── interaction helpers ───────────────────────────────────────────────
    pub is_dragging: bool,
    pub drag_start: Option<egui::Pos2>,
    /// True when dragging on the price axis (TradingView-style vertical scale).
    pub is_scaling_price: bool,
    /// Price zoom at start of price-axis drag.
    pub scale_start_zoom: f64,
    /// Y position at start of price-axis drag.
    pub scale_start_y: f32,

    // ── Live WS performance fields (added 2026-05) ──────────────────────
    /// Incremented whenever the visible closed bars change (not forming bar).
    /// Used by draw_chart for O(1) early-out when data is stable.
    pub visible_bars_gen: u64,
    /// Set by Kraken WS path when only the last bar was updated.
    /// Allows skipping full indicator recompute on forming-bar ticks.
    pub forming_bar_dirty: bool,
    /// Propagated from TyphooNApp during heavy backfill/sync so overlay panes can early-out.
    pub heavy_sync_in_progress: bool,
    /// Timestamp of the right-most bar (used for fast-path decisions).
    pub last_visible_bar_ts: i64,
    pub last_rendered_gen: u64,
    pub last_rendered_bar_ts: i64,
}

impl ChartState {
    const MAX_LIVE_QUOTE_SPREAD_PCT_FOR_MID: f64 = 5.0;

    /// Point this chart at a new symbol and clear per-symbol live-quote state.
    /// `live_bid`/`live_ask`/`live_quote_at`/`live_quote_delayed` belong to the
    /// *previous* symbol: carrying them over folds a stale mid into the new
    /// symbol's forming bar (see `has_live_quotes`) and can make the real-time
    /// quote guards suppress the new symbol's newest delayed quotes for up to the
    /// freshness window. Use for in-place symbol switches (watchlist / screener /
    /// explorer / peers); `reload_symbol_auto` already builds a fresh ChartState.
    pub fn switch_symbol(&mut self, symbol: impl Into<String>) {
        self.symbol = symbol.into();
        self.live_bid = 0.0;
        self.live_ask = 0.0;
        self.live_bid_size = 0.0;
        self.live_ask_size = 0.0;
        self.live_depth_bids.clear();
        self.live_depth_asks.clear();
        self.live_quote_at = None;
        self.live_quote_delayed = false;
    }

    pub fn fresh_live_quote_mid(&self) -> Option<f64> {
        // A *delayed* quote (Kraken iapi equities is always fetched delayed=true) is
        // not a real-time top-of-book: for a non-WS-tokenized xStock it can sit far
        // from the fresher consolidated last that the watchlist folds into the
        // forming bar, which is exactly the "chart bid/ask decoupled from watchlist"
        // desync. Only treat a streaming, real-time quote as the live mid; the
        // forming-bar close (fed by the watchlist) carries the price otherwise.
        if self.live_quote_delayed {
            return None;
        }
        let fresh = self
            .live_quote_at
            .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(30));
        if fresh && self.live_bid > 0.0 && self.live_ask > 0.0 {
            let mid = (self.live_bid + self.live_ask) * 0.5;
            if mid > 0.0
                && mid.is_finite()
                && Self::live_quote_mid_is_usable(self.live_bid, self.live_ask)
            {
                return Some(mid);
            }
        }
        None
    }

    pub fn live_quote_spread_pct(bid: f64, ask: f64) -> Option<f64> {
        let mid = (bid + ask) * 0.5;
        if bid <= 0.0 || ask <= 0.0 || mid <= 0.0 || !bid.is_finite() || !ask.is_finite() {
            return None;
        }
        Some(((ask - bid).abs() / mid) * 100.0)
    }

    fn live_quote_mid_is_usable(bid: f64, ask: f64) -> bool {
        Self::live_quote_spread_pct(bid, ask)
            .is_some_and(|spread_pct| spread_pct <= Self::MAX_LIVE_QUOTE_SPREAD_PCT_FOR_MID)
    }

    pub fn apply_forming_price_update(&mut self, price: f64) -> bool {
        if price <= 0.0 || !price.is_finite() {
            return false;
        }
        let Some(bar) = self.bars.last_mut() else {
            return false;
        };
        bar.close = price;
        bar.high = bar.high.max(price);
        bar.low = if bar.low > 0.0 {
            bar.low.min(price)
        } else {
            price
        };

        // Live update "Cur" levels for Previous Candle Levels so they reflect
        // new highs/lows inside the forming higher-timeframe bar (D1/W1/MN1).
        if let Some(h) = self.current_daily_high {
            if bar.high > h {
                self.current_daily_high = Some(bar.high);
            }
        }
        if let Some(l) = self.current_daily_low {
            if bar.low < l {
                self.current_daily_low = Some(bar.low);
            }
        }
        if let Some(h) = self.current_weekly_high {
            if bar.high > h {
                self.current_weekly_high = Some(bar.high);
            }
        }
        if let Some(l) = self.current_weekly_low {
            if bar.low < l {
                self.current_weekly_low = Some(bar.low);
            }
        }
        if let Some(h) = self.current_monthly_high {
            if bar.high > h {
                self.current_monthly_high = Some(bar.high);
            }
        }
        if let Some(l) = self.current_monthly_low {
            if bar.low < l {
                self.current_monthly_low = Some(bar.low);
            }
        }

        self.forming_bar_dirty = true;
        self.last_visible_bar_ts = self.bars.last().map(|b| b.ts_ms).unwrap_or(0);
        true
    }

    /// Apply a real-time executed trade: update forming price (close/high/low) + accumulate trade volume into the current forming bar.
    /// O(1) per trade. Used for Kraken public trades feed (and MTF cells via chart_by_bare).
    pub fn apply_forming_trade(&mut self, price: f64, trade_vol: f64) -> bool {
        let price_updated = self.apply_forming_price_update(price);
        if price_updated && trade_vol > 0.0 && trade_vol.is_finite() {
            if let Some(bar) = self.bars.last_mut() {
                // Low-TF (M1/M5) Kraken: public trades volume accumulated O(1) into forming bar.
                // Per-chart (per TF) so M1/M5 get precise live volume without double-count (new bar starts clean).
                bar.volume += trade_vol;
            }
        }
        // Manual camera (free-look) preservation: forming live updates (price + vol from trades) only dirty the bar,
        // do not reset manual price_center or follow_latest. Camera code already respects manual_override.
        price_updated
    }

    pub fn apply_live_quote_update(&mut self, bid: f64, ask: f64, bid_size: f64, ask_size: f64, delayed: bool) -> bool {
        let mid = (bid + ask) * 0.5;
        if bid <= 0.0 || ask <= 0.0 || mid <= 0.0 || !bid.is_finite() || !ask.is_finite() {
            return false;
        }
        self.live_bid = bid;
        self.live_ask = ask;
        if bid_size > 0.0 { self.live_bid_size = bid_size; }
        if ask_size > 0.0 { self.live_ask_size = ask_size; }
        // For depth profile binning: seed with top level (full book levels can be pushed from L2/L3 updates)
        self.live_depth_bids.clear();
        if bid > 0.0 && bid_size > 0.0 { self.live_depth_bids.push((bid, bid_size)); }
        self.live_depth_asks.clear();
        if ask > 0.0 && ask_size > 0.0 { self.live_depth_asks.push((ask, ask_size)); }
        self.live_quote_at = Some(std::time::Instant::now());
        self.live_quote_delayed = delayed;
        // A delayed quote (Kraken iapi equities is always delayed=true) is stored
        // for reference but must not drive the forming candle: the consolidated
        // last folded by the watchlist is fresher, and folding a stale delayed mid
        // here is what decoupled the candle/bid/ask from the watchlist. Leave the
        // forming bar to the watchlist path (which prefers row.last when the chart
        // quote is delayed/stale — see handle_watchlist_quotes' realtime_fresh).
        if delayed {
            return false;
        }
        if self.ext_active {
            return false;
        }
        if !Self::live_quote_mid_is_usable(bid, ask) {
            return false;
        }
        self.apply_forming_price_update(mid)
    }

    pub fn fold_fresh_live_quote_into_forming_bar(&mut self) -> bool {
        self.fresh_live_quote_mid()
            .is_some_and(|mid| self.apply_forming_price_update(mid))
    }

    pub fn new(symbol: impl Into<String>, tf: Timeframe) -> Self {
        Self {
            symbol: symbol.into(),
            timeframe: tf,
            primary_source: "",
            source_override: "",
            regulatory_alerts: Vec::new(),
            chart_type: ChartType::Candle,
            show_in_tab_bar: true,
            log_scale: false,
            sma_slow_period: 200,
            sma_fast_period: 100,
            ema_period: 21,
            rsi_period: 14,
            atr_period: 14,
            bb_period: 20,
            macd_fast: 12,
            macd_slow: 26,
            macd_signal_p: 9,
            stoch_period: 14,
            adx_period: 14,
            fisher_period: 32,
            momentum_period: 10,
            compare_symbol: None,
            compare_bars: Vec::new(),
            live_bid: 0.0,
            live_ask: 0.0,
            live_bid_size: 0.0,
            live_ask_size: 0.0,
            live_depth_bids: Vec::new(),
            live_depth_asks: Vec::new(),
            live_quote_at: None,
            live_quote_delayed: false,
            ext_open: 0.0,
            ext_high: 0.0,
            ext_low: 0.0,
            ext_close: 0.0,
            ext_active: false,
            prev_daily_close: 0.0,
            bars: Vec::new(),
            upload_opens: Vec::new(),
            upload_closes: Vec::new(),
            upload_highs: Vec::new(),
            upload_lows: Vec::new(),
            upload_volumes: Vec::new(),
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
            prev_h4_high: None,
            prev_h4_low: None,
            prev_h1_high: None,
            prev_h1_low: None,
            prev_monthly_high: None,
            prev_monthly_low: None,
            current_daily_high: None,
            current_daily_low: None,
            current_weekly_high: None,
            current_weekly_low: None,
            current_monthly_high: None,
            current_monthly_low: None,
            wma: Vec::new(),
            hma: Vec::new(),
            cci: Vec::new(),
            williams_r: Vec::new(),
            obv: Vec::new(),
            momentum: Vec::new(),
            cmo: Vec::new(),
            cmo_sum_up: 0.0,
            cmo_sum_down: 0.0,
            linreg_sum_x: 0.0,
            linreg_sum_y: 0.0,
            linreg_sum_xy: 0.0,
            linreg_sum_x2: 0.0,
            linreg: Vec::new(),
            linreg_angle: Vec::new(),
            linreg_intercept: Vec::new(),
            linreg_slope: Vec::new(),
            qstick: Vec::new(),
            disparity: Vec::new(),
            bop: Vec::new(),
            stddev: Vec::new(),
            mfi: Vec::new(),
            trix_line: Vec::new(),
            trix_signal: Vec::new(),
            trix_hist: Vec::new(),
            ppo_line: Vec::new(),
            ppo_signal: Vec::new(),
            ppo_hist: Vec::new(),
            ultosc: Vec::new(),
            stochrsi_k: Vec::new(),
            stochrsi_d: Vec::new(),
            var_oscillator: Vec::new(),
            psar: Vec::new(),
            mtf_sma: Vec::new(),
            atr_proj_upper: Vec::new(),
            atr_proj_lower: Vec::new(),
            atr_proj_levels: Vec::new(),
            better_vol_type: Vec::new(),
            pivot_p: None,
            pivot_r1: None,
            pivot_r2: None,
            pivot_s1: None,
            pivot_s2: None,
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
            primary_first_ts: 0,
            gap_fill_timestamps: std::collections::HashSet::new(),
            demand_zones: Vec::new(),
            auto_fib_levels: Vec::new(),
            auto_fib_swing: None,
            supertrend: Vec::new(),
            supertrend_bull: Vec::new(),
            donchian_upper: Vec::new(),
            donchian_lower: Vec::new(),
            keltner_mid: Vec::new(),
            keltner_upper: Vec::new(),
            keltner_lower: Vec::new(),
            regression_mid: Vec::new(),
            regression_upper: Vec::new(),
            regression_lower: Vec::new(),
            squeeze_mom: Vec::new(),
            squeeze_on: Vec::new(),
            vol_avg_20: Vec::new(),
            vwap: Vec::new(),
            vwap_upper1: Vec::new(),
            vwap_upper2: Vec::new(),
            vwap_upper3: Vec::new(),
            vwap_lower1: Vec::new(),
            vwap_lower2: Vec::new(),
            vwap_lower3: Vec::new(),
            multi_kama: Vec::new(),
            harmonics: Vec::new(),
            drawings: Vec::new(),
            hline_set: std::collections::HashSet::new(),
            fibo_set: std::collections::HashSet::new(),
            vline_set: std::collections::HashSet::new(),
            harmonic_set: std::collections::HashSet::new(),
            drawing_styles: Vec::new(),
            drawings_undo: Vec::new(),
            selected_drawing: None,
            dragging_cp: None,
            is_drawing_drag: false,
            cached_trade_overlay: TradeOverlay::default(),
            cached_trade_overlay_frame: 0,
            visible_bars: 200,
            view_offset: 0,
            manual_view_override: false,
            camera: ChartCamera::from_legacy(0, 200, false),
            replay_bar_cap: None,
            price_pan: 0.0,
            price_zoom: 1.0,
            is_dragging: false,
            drag_start: None,
            is_scaling_price: false,
            scale_start_zoom: 1.0,
            scale_start_y: 0.0,
            visible_bars_gen: 0,
            forming_bar_dirty: false,
            heavy_sync_in_progress: false,
            last_visible_bar_ts: 0,
            last_rendered_gen: 0,
            last_rendered_bar_ts: 0,
        }
    }

    /// Fast-path update used by live Kraken WS: only mutate the last bar
    /// and set the dirty flag so draw_chart can early-out everything else.
    #[allow(dead_code)]
    pub fn apply_forming_bar_update(&mut self, bar: Bar) {
        if let Some(last) = self.bars.last_mut() {
            if last.ts_ms == bar.ts_ms {
                *last = bar;
            } else {
                self.bars.push(bar);
            }
        } else {
            self.bars.push(bar);
        }
        self.forming_bar_dirty = true;
        self.last_visible_bar_ts = self.bars.last().map(|b| b.ts_ms).unwrap_or(0);
    }

    /// Call when a closed bar is added or the visible range structurally changes.
    #[allow(dead_code)]
    pub fn mark_structural_change(&mut self) {
        self.visible_bars_gen = self.visible_bars_gen.wrapping_add(1);
        self.forming_bar_dirty = false;
        self.last_visible_bar_ts = self.bars.last().map(|b| b.ts_ms).unwrap_or(0);
    }
}
