//! Extracted from app.rs: chart helpers.

use super::*;

// Reexports from extracted chart_sources module (modular cut)
pub(crate) use super::chart_sources::chart_source_cache_keys;

mod auto_fibonacci;
mod camera_controls;
mod mtf_overlays;
pub(crate) use mtf_overlays::mtf_htf_cache_put;

// Chart view-model types + indicator palette now live in the typhoon-chart-ui crate
// (ADR-125 Target 2, slice 5); re-exported so chart.rs + the app glob are unchanged.
pub(crate) use typhoon_chart_ui::models::{
    ADX_COL, BVOL_CHURN, BVOL_CLIMAX_DN, BVOL_CLIMAX_UP, BVOL_HIGH, BVOL_LOW, BVOL_NORMAL, CCI_COL,
    CHART_MIN_MAIN_CHART_H, CHART_RIGHT_MARGIN, CHART_SUB_PANE_H, ChartCamera, DI_MINUS_COL,
    DI_PLUS_COL, EHLERS_CG_COL, EHLERS_CYBER_COL, EHLERS_DEC_COL, EHLERS_EBSW_COL, EHLERS_FAMA_COL,
    EHLERS_ITL_COL, EHLERS_MAMA_COL, EHLERS_ROOF_COL, EHLERS_SS_COL, HMA_COL, ICHI_CLOUD_BEAR,
    ICHI_CLOUD_BULL, ICHI_KIJUN, ICHI_SPAN_A, ICHI_SPAN_B, ICHI_TENKAN, IndicatorFlags, MFI_COL,
    OBV_COL, PPO_LINE_COL, PPO_SIG_COL, SAR_COL, STOCH_D_COL, STOCH_K_COL, TRIX_LINE_COL,
    TRIX_SIG_COL, ULTOSC_COL, WILLR_COL, WMA_COL, chart_price_pane_height,
};

/// All state for one chart viewport.
pub(crate) struct ChartState {
    /// The symbol string shown in the toolbar.
    pub(crate) symbol: String,
    /// Currently selected timeframe.
    pub(crate) timeframe: Timeframe,
    /// Cache source that most recently populated `bars` ("kraken", "alpaca", etc).
    pub(crate) primary_source: &'static str,
    /// User-forced chart source for debugging provider-specific cache rows.
    /// Empty means automatic source selection.
    pub(crate) source_override: &'static str,
    /// Active symbol-level regulatory warnings shown next to the chart symbol.
    pub(crate) regulatory_alerts: Vec<typhoon_engine::core::regulatory_alerts::RegulatoryAlert>,
    /// Chart rendering style.
    pub(crate) chart_type: ChartType,
    /// Logarithmic price scale (vs linear).
    pub(crate) log_scale: bool,
    // ── Configurable indicator periods ─────────────────────────────────
    pub(crate) sma_slow_period: u32,
    pub(crate) sma_fast_period: u32,
    pub(crate) ema_period: u32,
    pub(crate) rsi_period: u32,
    pub(crate) atr_period: u32,
    pub(crate) bb_period: u32,
    pub(crate) macd_fast: u32,
    pub(crate) macd_slow: u32,
    pub(crate) macd_signal_p: u32,
    pub(crate) stoch_period: u32,
    pub(crate) adx_period: u32,
    pub(crate) fisher_period: u32,
    pub(crate) momentum_period: u32,
    // ── Compare symbol overlay ─────────────────────────────────────────
    pub(crate) compare_symbol: Option<String>,
    pub(crate) compare_bars: Vec<Bar>,
    /// Live bid/ask from streaming quotes (for spread line rendering).
    pub(crate) live_bid: f64,
    pub(crate) live_ask: f64,
    /// When `live_bid`/`live_ask` were last refreshed from a streaming quote.
    /// The spread lines are hidden once this goes stale so a frozen quote isn't
    /// drawn next to a live (differently-priced) last/candle.
    pub(crate) live_quote_at: Option<std::time::Instant>,
    /// `true` when the most recent `live_bid`/`live_ask` came from a *delayed*
    /// source (the Kraken iapi equity ticker, ~15 min). Lets a real-time WS quote
    /// take precedence so the chart spread/last don't flip to the delayed snapshot
    /// while live ticks are flowing.
    pub(crate) live_quote_delayed: bool,
    // Extended hours candle (pre/post market)
    pub(crate) ext_open: f64,
    pub(crate) ext_high: f64,
    pub(crate) ext_low: f64,
    pub(crate) ext_close: f64,
    pub(crate) ext_active: bool, // true when ext hours data is available
    /// Authoritative previous-day regular close (Alpaca `prevDailyBar.c` /
    /// Yahoo `regularMarketPreviousClose`), sourced from the shared watchlist
    /// quote so it is timeframe-independent. `0.0` when unknown. Drives the
    /// extended-hours badge "Day %" so a W1/MN chart shows the day move, not a
    /// week/month-ago comparison from its own previous bar.
    pub(crate) prev_daily_close: f64,
    /// Raw bar data loaded from cache.
    pub(crate) bars: Vec<Bar>,
    /// Reusable buffers for full GPU upload path (avoids repeated allocations).
    #[allow(dead_code)]
    pub(crate) upload_opens: Vec<f32>,
    #[allow(dead_code)]
    pub(crate) upload_closes: Vec<f32>,
    #[allow(dead_code)]
    pub(crate) upload_highs: Vec<f32>,
    #[allow(dead_code)]
    pub(crate) upload_lows: Vec<f32>,
    #[allow(dead_code)]
    pub(crate) upload_volumes: Vec<f32>,
    /// Pre-computed SMA(200) — indexed parallel to `bars`.
    pub(crate) sma200: Vec<Option<f64>>,
    /// Pre-computed SMA(100) — indexed parallel to `bars`.
    pub(crate) sma100: Vec<Option<f64>>,
    /// Pre-computed KAMA(10,2,30) — indexed parallel to `bars`.
    pub(crate) kama: Vec<Option<f64>>,
    /// Pre-computed EMA(21).
    pub(crate) ema21: Vec<Option<f64>>,
    /// Bollinger Bands (middle, upper, lower).
    pub(crate) bb_mid: Vec<Option<f64>>,
    pub(crate) bb_upper: Vec<Option<f64>>,
    pub(crate) bb_lower: Vec<Option<f64>>,
    /// RSI(14) — 0..100 range.
    pub(crate) rsi: Vec<Option<f64>>,
    /// Fisher Transform.
    pub(crate) fisher: Vec<Option<f64>>,
    pub(crate) fisher_signal: Vec<Option<f64>>,
    /// ATR(14).
    pub(crate) atr: Vec<Option<f64>>,
    /// MACD(12,26,9).
    pub(crate) macd_line: Vec<Option<f64>>,
    pub(crate) macd_signal: Vec<Option<f64>>,
    pub(crate) macd_hist: Vec<Option<f64>>,
    /// Stochastic(14,3,3).
    pub(crate) stoch_k: Vec<Option<f64>>,
    pub(crate) stoch_d: Vec<Option<f64>>,
    /// ADX(14) + DI+/DI-.
    pub(crate) adx: Vec<Option<f64>>,
    pub(crate) di_plus: Vec<Option<f64>>,
    pub(crate) di_minus: Vec<Option<f64>>,
    /// Ichimoku(9,26,52).
    pub(crate) ichi_tenkan: Vec<Option<f64>>,
    pub(crate) ichi_kijun: Vec<Option<f64>>,
    pub(crate) ichi_span_a: Vec<Option<f64>>,
    pub(crate) ichi_span_b: Vec<Option<f64>>,
    /// Previous candle levels (daily high/low).
    pub(crate) prev_daily_high: Option<f64>,
    pub(crate) prev_daily_low: Option<f64>,
    pub(crate) prev_weekly_high: Option<f64>,
    pub(crate) prev_weekly_low: Option<f64>,
    pub(crate) prev_h4_high: Option<f64>,
    pub(crate) prev_h4_low: Option<f64>,
    pub(crate) prev_h1_high: Option<f64>,
    pub(crate) prev_h1_low: Option<f64>,
    pub(crate) prev_monthly_high: Option<f64>,
    pub(crate) prev_monthly_low: Option<f64>,
    // Current ("Judas") candle levels — the forming D1/W1/MN1 period high/low,
    // drawn alongside the previous-candle levels (PreviousCandleLevels.mqh).
    pub(crate) current_daily_high: Option<f64>,
    pub(crate) current_daily_low: Option<f64>,
    pub(crate) current_weekly_high: Option<f64>,
    pub(crate) current_weekly_low: Option<f64>,
    pub(crate) current_monthly_high: Option<f64>,
    pub(crate) current_monthly_low: Option<f64>,
    /// WMA(20), HMA(20).
    pub(crate) wma: Vec<Option<f64>>,
    pub(crate) hma: Vec<Option<f64>>,
    /// CCI(20), Williams %R(14).
    pub(crate) cci: Vec<Option<f64>>,
    pub(crate) williams_r: Vec<Option<f64>>,
    /// OBV.
    pub(crate) obv: Vec<Option<f64>>,
    /// Momentum(10).
    pub(crate) momentum: Vec<Option<f64>>,
    /// CMO(9).
    pub(crate) cmo: Vec<Option<f64>>,
    /// Running sums for O(1) CMO forming-bar update
    pub(crate) cmo_sum_up: f64,
    pub(crate) cmo_sum_down: f64,
    /// Running sums for O(1) Linear Regression Slope
    pub(crate) linreg_sum_x: f64,
    pub(crate) linreg_sum_y: f64,
    pub(crate) linreg_sum_xy: f64,
    pub(crate) linreg_sum_x2: f64,
    /// QStick(14).
    pub(crate) qstick: Vec<Option<f64>>,
    /// Disparity Index(14).
    pub(crate) disparity: Vec<Option<f64>>,
    /// LINEARREG_SLOPE
    pub(crate) linreg_slope: Vec<Option<f64>>,
    /// LINEARREG_INTERCEPT
    pub(crate) linreg_intercept: Vec<Option<f64>>,
    /// LINEARREG_ANGLE
    #[allow(dead_code)]
    pub(crate) linreg_angle: Vec<Option<f64>>,
    /// LINEARREG (endpoint value)
    #[allow(dead_code)]
    pub(crate) linreg: Vec<Option<f64>>,
    /// BOP(14).
    pub(crate) bop: Vec<Option<f64>>,
    /// StdDev(20).
    pub(crate) stddev: Vec<Option<f64>>,
    /// MFI(14).
    pub(crate) mfi: Vec<Option<f64>>,
    /// TRIX(15,9).
    pub(crate) trix_line: Vec<Option<f64>>,
    pub(crate) trix_signal: Vec<Option<f64>>,
    pub(crate) trix_hist: Vec<Option<f64>>,
    /// PPO(12,26,9).
    pub(crate) ppo_line: Vec<Option<f64>>,
    pub(crate) ppo_signal: Vec<Option<f64>>,
    pub(crate) ppo_hist: Vec<Option<f64>>,
    /// Ultimate Oscillator(7,14,28).
    pub(crate) ultosc: Vec<Option<f64>>,
    /// StochRSI(14,14,3,3).
    pub(crate) stochrsi_k: Vec<Option<f64>>,
    pub(crate) stochrsi_d: Vec<Option<f64>>,
    /// VaR oscillator (20-bar rolling parametric VaR, 95%).
    pub(crate) var_oscillator: Vec<Option<f64>>,
    /// Parabolic SAR(0.02, 0.2).
    pub(crate) psar: Vec<Option<f64>>,
    /// MTF SMA lines — (label, color_idx, projected_points) matching MTF_MA.mqh.
    /// H1/200, H4/200, D1/200, W1/200, W1/100, MN1/100
    pub(crate) mtf_sma: Vec<(String, Vec<(usize, f64)>)>,
    /// ATR Projection (open ± ATR bands — legacy per-bar, kept for GPU path).
    pub(crate) atr_proj_upper: Vec<Option<f64>>,
    pub(crate) atr_proj_lower: Vec<Option<f64>>,
    /// ATR Projection MTF levels (label, open, atr_value, start_bar_idx) — matches ATR_Projection.mqh.
    pub(crate) atr_proj_levels: Vec<(&'static str, f64, f64, usize)>,
    /// Better Volume classification.
    pub(crate) better_vol_type: Vec<u8>, // 0=normal, 1=climax_up, 2=climax_dn, 3=high, 4=low, 5=churn
    /// Pivot points (computed from daily data).
    pub(crate) pivot_p: Option<f64>,
    pub(crate) pivot_r1: Option<f64>,
    pub(crate) pivot_r2: Option<f64>,
    pub(crate) pivot_s1: Option<f64>,
    pub(crate) pivot_s2: Option<f64>,
    /// Bill Williams Fractals (up/down arrows).
    pub(crate) fractal_up: Vec<bool>,
    pub(crate) fractal_down: Vec<bool>,
    // ── Ehlers indicators ──────────────────────────────────────────────
    /// Super Smoother (overlay).
    pub(crate) ehlers_ss: Vec<Option<f64>>,
    /// Decycler (overlay).
    pub(crate) ehlers_decycler: Vec<Option<f64>>,
    /// Instantaneous Trendline (overlay).
    pub(crate) ehlers_itl: Vec<Option<f64>>,
    /// MAMA (overlay).
    pub(crate) ehlers_mama: Vec<Option<f64>>,
    /// FAMA (overlay).
    pub(crate) ehlers_fama: Vec<Option<f64>>,
    /// Even Better Sinewave (sub-pane, -1 to 1).
    pub(crate) ehlers_ebsw: Vec<Option<f64>>,
    /// Cyber Cycle (sub-pane).
    pub(crate) ehlers_cyber: Vec<Option<f64>>,
    /// CG Oscillator (sub-pane).
    pub(crate) ehlers_cg: Vec<Option<f64>>,
    /// Roofing Filter (sub-pane).
    pub(crate) ehlers_roof: Vec<Option<f64>>,
    /// Supply/demand zones: (bar_idx, zone_high, zone_low, status).
    /// Status: 0=untested, 1=tested (price returned), 2=proven (price bounced)
    pub(crate) supply_zones: Vec<(usize, f64, f64, u8)>,
    /// Earliest timestamp from primary data source (Kraken/Alpaca). Bars before this are backfill.
    pub(crate) primary_first_ts: i64,
    /// Timestamps of bars sourced from gap-fill (Kraken). Colored magenta.
    pub(crate) gap_fill_timestamps: std::collections::HashSet<i64>,
    pub(crate) demand_zones: Vec<(usize, f64, f64, u8)>,
    /// Auto Fibonacci levels: (price, label, is_extension).
    pub(crate) auto_fib_levels: Vec<(f64, String, bool)>,
    /// Auto Fibonacci swing: (swing_high_price, swing_low_price, swing_high_idx, swing_low_idx).
    pub(crate) auto_fib_swing: Option<(f64, f64, usize, usize)>,
    /// VWAP (volume-weighted average price), anchored daily.
    pub(crate) vwap: Vec<Option<f64>>,
    /// VWAP upper deviation bands (1σ, 2σ, 3σ).
    pub(crate) vwap_upper1: Vec<Option<f64>>,
    pub(crate) vwap_upper2: Vec<Option<f64>>,
    pub(crate) vwap_upper3: Vec<Option<f64>>,
    /// VWAP lower deviation bands (1σ, 2σ, 3σ).
    pub(crate) vwap_lower1: Vec<Option<f64>>,
    pub(crate) vwap_lower2: Vec<Option<f64>>,
    pub(crate) vwap_lower3: Vec<Option<f64>>,
    /// Supertrend line (ATR-based trend bands, single line that flips).
    pub(crate) supertrend: Vec<Option<f64>>,
    /// Supertrend direction: true = bullish (below price), false = bearish (above).
    pub(crate) supertrend_bull: Vec<bool>,
    /// Donchian Channel upper (highest high over N bars).
    pub(crate) donchian_upper: Vec<Option<f64>>,
    /// Donchian Channel lower (lowest low over N bars).
    pub(crate) donchian_lower: Vec<Option<f64>>,
    /// Keltner Channel middle (EMA).
    pub(crate) keltner_mid: Vec<Option<f64>>,
    /// Keltner Channel upper (EMA + ATR×mult).
    pub(crate) keltner_upper: Vec<Option<f64>>,
    /// Keltner Channel lower (EMA - ATR×mult).
    pub(crate) keltner_lower: Vec<Option<f64>>,
    /// Regression Channel middle (regression line).
    pub(crate) regression_mid: Vec<Option<f64>>,
    /// Regression Channel upper (regression + 2σ).
    pub(crate) regression_upper: Vec<Option<f64>>,
    /// Regression Channel lower (regression - 2σ).
    pub(crate) regression_lower: Vec<Option<f64>>,
    /// Squeeze Momentum: momentum histogram value.
    pub(crate) squeeze_mom: Vec<Option<f64>>,
    /// Squeeze state: true = in squeeze (BB inside KC).
    pub(crate) squeeze_on: Vec<bool>,
    /// Pre-computed 20-bar rolling average volume (for volume heatmap candle coloring).
    pub(crate) vol_avg_20: Vec<f64>,
    /// MultiKAMA: KAMA values from higher timeframes projected onto this chart's x-axis.
    /// Each entry: (timeframe_label, Vec of (bar_index_in_this_chart, kama_value))
    pub(crate) multi_kama: Vec<(String, Vec<(usize, f64)>)>,
    /// Detected harmonic patterns.
    pub(super) harmonics: Vec<HarmonicPattern>,
    /// Drawing annotations.
    pub(crate) drawings: Vec<Drawing>,
    /// O(1) for HLine dedup (price key) during sync (replaces .any).
    pub(crate) hline_set: std::collections::HashSet<String>,
    /// O(1) for FiboRetrace dedup (key by high/low/bars) during cross-TF sync (beyond HLine).
    pub(crate) fibo_set: std::collections::HashSet<String>,
    /// O(1) for VLine dedup (bar_idx key) during sync.
    pub(crate) vline_set: std::collections::HashSet<String>,
    /// O(1) for harmonics dedup (name + key points) for future cross-TF or add paths.
    #[allow(dead_code)]
    pub(crate) harmonic_set: std::collections::HashSet<String>,
    /// Per-drawing style: (line_width, line_style). Indexed parallel to `drawings`.
    pub(crate) drawing_styles: Vec<(f32, LineStyle)>,
    /// Undo stack for drawings (Ctrl+Z pops from drawings into here, Ctrl+Shift+Z restores)
    pub(crate) drawings_undo: Vec<Drawing>,
    pub(crate) selected_drawing: Option<usize>,
    /// Index of the control point being dragged (for resize). None = whole-drawing drag.
    pub(crate) dragging_cp: Option<usize>,
    /// True while dragging a selected drawing (suppresses chart pan).
    pub(crate) is_drawing_drag: bool,
    /// Cached trade overlay (rebuilt when bg data changes, not every frame).
    pub(crate) cached_trade_overlay: TradeOverlay,
    pub(crate) cached_trade_overlay_frame: u64,

    // ── view state ────────────────────────────────────────────────────────
    /// How many bars are visible horizontally (zoom level).
    pub(crate) visible_bars: usize,
    /// Index of the right-most visible bar (0 = oldest, len-1 = newest).
    pub(crate) view_offset: usize,
    /// True after the user manually pans/zooms the chart away from the auto-follow view.
    /// Cache reloads must preserve this viewport instead of snapping back to latest.
    pub(crate) manual_view_override: bool,
    /// Canonical TradingView-style camera. Legacy viewport fields above are kept
    /// in sync for draw/overlay code until the renderer consumes the camera directly.
    pub(crate) camera: ChartCamera,
    /// When replay mode is active, cap visible_range end at this bar index.
    pub(crate) replay_bar_cap: Option<usize>,
    /// Fractional price offset for vertical pan.
    pub(crate) price_pan: f64,
    /// Multiplier applied to the natural price range for vertical zoom.
    pub(crate) price_zoom: f64,

    // ── interaction helpers ───────────────────────────────────────────────
    pub(crate) is_dragging: bool,
    pub(crate) drag_start: Option<egui::Pos2>,
    /// True when dragging on the price axis (TradingView-style vertical scale).
    pub(crate) is_scaling_price: bool,
    /// Price zoom at start of price-axis drag.
    pub(crate) scale_start_zoom: f64,
    /// Y position at start of price-axis drag.
    pub(crate) scale_start_y: f32,

    // ── Live WS performance fields (added 2026-05) ──────────────────────
    /// Incremented whenever the visible closed bars change (not forming bar).
    /// Used by draw_chart for O(1) early-out when data is stable.
    pub(crate) visible_bars_gen: u64,
    /// Set by Kraken WS path when only the last bar was updated.
    /// Allows skipping full indicator recompute on forming-bar ticks.
    pub(crate) forming_bar_dirty: bool,
    /// Propagated from TyphooNApp during heavy backfill/sync so overlay panes can early-out.
    pub(crate) heavy_sync_in_progress: bool,
    /// Timestamp of the right-most bar (used for fast-path decisions).
    pub(crate) last_visible_bar_ts: i64,
    pub(crate) last_rendered_gen: u64,
    pub(crate) last_rendered_bar_ts: i64,
}

mod market_data_helpers;
pub(crate) use market_data_helpers::{
    bare_symbol_from_key, cache_source_from_key, chart_bar_last_valid_ts,
    chart_gap_fill_bar_allowed, chart_merge_bucket_ts, chart_source_bars_match_timeframe,
    kraken_pair_asset_class, kraken_xstock_fundamental_symbol, normalize_market_data_symbol,
};

#[cfg(test)]
pub(crate) use market_data_helpers::chart_quote_overlay_allowed;

mod load_cache;
pub(crate) use load_cache::ChartDataLoad;

mod indicator_compute;

/// Merge equity/ETF bars from multiple providers into one continuous series.
///
/// Sources split into two tiers by [`chart_equity_source_rank`]:
/// * **Trusted** (rank ≤ 2 — `kraken-equities`, `alpaca`): live /
///   native feeds that define the authoritative, corporate-action-adjusted
///   price scale. Merged per-bucket by priority (best rank wins; latest tick
///   wins within a source). The PRIMARY broker (top-bar switch, ADR-126) is
///   rank 0 and defines the scale; the other tradeable broker is rank 2 and
///   corroborates/gap-fills without redefining it.
/// * **Depth** (rank ≥ 3 — `yahoo-chart`, `default`): history-extenders. Yahoo
///   carries far deeper history but is frequently *unadjusted* across splits and
///   token re-denominations (WOK was ~10,000× too high before its 2025 action),
///   which a naïve splice would paste straight onto the trusted scale as a
///   discontinuity. So each depth source is back-adjusted to the trusted scale
///   by the price ratio where they overlap, and only fills buckets the trusted
///   tier lacks (older history + interior gaps). A depth source whose overlap
///   scale is inconsistent — the tell-tale of an unadjusted action mid-history —
///   is dropped entirely rather than splicing scale-jumped bars.
///
/// With no trusted source present (Yahoo-only symbol) we fall back to per-bucket
/// priority across the depth sources so the symbol still charts (best effort).
/// A known stock split / reverse split: bars strictly before `ex_ts_ms` are
/// multiplied by `pre_split_factor` (= old shares / new shares = denominator /
/// numerator) to lift raw, unadjusted pre-split history onto the post-split price
/// scale. For a 1-for-100 reverse split the factor is 100.
mod equity_merge;
pub(crate) use equity_merge::{
    CHART_SOURCE_ORDER, cache_source_label, chart_equity_low_timeframe_requires_native_source,
    chart_equity_native_source_tag, chart_equity_source_rank, chart_forming_bar_allowed,
    chart_load_merged_equity_bars_from_cache, chart_log_merged_cache_load_done,
    chart_log_merged_cache_load_start, chart_materialize_merged_equity_cache,
    chart_merged_equity_cache_key, chart_prefers_fresh_equity_source,
    extract_news_symbols_from_market_data_cache, set_chart_merge_primary_broker,
};

#[cfg(test)]
pub(crate) use equity_merge::{
    ChartSplit, chart_curated_known_splits, chart_equity_source_rank_for,
    chart_merge_equity_raw_bars, chart_merge_equity_raw_bars_with_primary,
    chart_persist_merged_equity_bars_to_cache, news_symbol_from_market_data_cache_key,
};

impl ChartState {
    const MAX_LIVE_QUOTE_SPREAD_PCT_FOR_MID: f64 = 5.0;

    /// Point this chart at a new symbol and clear per-symbol live-quote state.
    /// `live_bid`/`live_ask`/`live_quote_at`/`live_quote_delayed` belong to the
    /// *previous* symbol: carrying them over folds a stale mid into the new
    /// symbol's forming bar (see `has_live_quotes`) and can make the real-time
    /// quote guards suppress the new symbol's newest delayed quotes for up to the
    /// freshness window. Use for in-place symbol switches (watchlist / screener /
    /// explorer / peers); `reload_symbol_auto` already builds a fresh ChartState.
    pub(crate) fn switch_symbol(&mut self, symbol: impl Into<String>) {
        self.symbol = symbol.into();
        self.live_bid = 0.0;
        self.live_ask = 0.0;
        self.live_quote_at = None;
        self.live_quote_delayed = false;
    }

    pub(crate) fn fresh_live_quote_mid(&self) -> Option<f64> {
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

    pub(crate) fn live_quote_spread_pct(bid: f64, ask: f64) -> Option<f64> {
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

    pub(crate) fn apply_forming_price_update(&mut self, price: f64) -> bool {
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

    pub(crate) fn apply_live_quote_update(&mut self, bid: f64, ask: f64, delayed: bool) -> bool {
        let mid = (bid + ask) * 0.5;
        if bid <= 0.0 || ask <= 0.0 || mid <= 0.0 || !bid.is_finite() || !ask.is_finite() {
            return false;
        }
        self.live_bid = bid;
        self.live_ask = ask;
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

    pub(crate) fn fold_fresh_live_quote_into_forming_bar(&mut self) -> bool {
        self.fresh_live_quote_mid()
            .is_some_and(|mid| self.apply_forming_price_update(mid))
    }

    pub(crate) fn new(symbol: impl Into<String>, tf: Timeframe) -> Self {
        Self {
            symbol: symbol.into(),
            timeframe: tf,
            primary_source: "",
            source_override: "",
            regulatory_alerts: Vec::new(),
            chart_type: ChartType::Candle,
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

    pub(crate) fn symbol_matches(&self, symbol: &str) -> bool {
        normalize_market_data_symbol(&self.symbol)
            .replace('/', "")
            .eq_ignore_ascii_case(&normalize_market_data_symbol(symbol).replace('/', ""))
    }
}

#[cfg(test)]
mod mtf_scale_guard_tests {
    use super::*;

    fn bars_at(price: f64, n: usize) -> Vec<Bar> {
        (0..n)
            .map(|i| Bar {
                ts_ms: 1_700_000_000_000 + i as i64 * 86_400_000,
                open: price,
                high: price,
                low: price,
                close: price,
                volume: 1.0,
            })
            .collect()
    }

    #[test]
    fn keeps_line_near_price() {
        let bars = bars_at(2.0, 5);
        let projected: Vec<(usize, f64)> = (0..5).map(|i| (i, 2.1)).collect();
        assert!(ChartState::mtf_line_scale_ok(&bars, &projected));
    }

    #[test]
    fn drops_overscaled_line() {
        // Grossly mis-scaled: candles ~$2, line parked at ~$250 (ratio ~125 > the
        // SCALE_TOL=100 ceiling). A merely-lagging average (a post-crash SMA200 at
        // 7.5–100× price) is intentionally *kept* now — see
        // `keeps_lagging_average_within_tolerance` — so only an outright scale
        // fault like this is dropped.
        let bars = bars_at(2.0, 5);
        let projected: Vec<(usize, f64)> = (0..5).map(|i| (i, 250.0)).collect();
        assert!(!ChartState::mtf_line_scale_ok(&bars, &projected));
    }

    #[test]
    fn drops_underscaled_line() {
        // WOK-style: an un-back-adjusted feed orders of magnitude below price.
        let bars = bars_at(2.0, 5);
        let projected: Vec<(usize, f64)> = (0..5).map(|i| (i, 0.0002)).collect();
        assert!(!ChartState::mtf_line_scale_ok(&bars, &projected));
    }

    #[test]
    fn keeps_lagging_average_within_tolerance() {
        // A slow MA lagging at ~3x price is legitimate (median ratio 3 <= SCALE_TOL).
        let bars = bars_at(2.0, 5);
        let projected: Vec<(usize, f64)> = (0..5).map(|i| (i, 6.0)).collect();
        assert!(ChartState::mtf_line_scale_ok(&bars, &projected));
    }

    #[test]
    fn median_ignores_brief_excursion() {
        // One wild point but the median stays near 1 → keep the line (no gaps).
        let bars = bars_at(2.0, 5);
        let projected = vec![(0, 2.0), (1, 2.1), (2, 50.0), (3, 1.9), (4, 2.0)];
        assert!(ChartState::mtf_line_scale_ok(&bars, &projected));
    }

    #[test]
    fn empty_projection_is_rejected() {
        let bars = bars_at(2.0, 5);
        assert!(!ChartState::mtf_line_scale_ok(&bars, &[]));
    }
}
