//! Extracted from app.rs: chart helpers.

use super::*;

// Reexports from extracted chart_sources module (modular cut)
pub(crate) use super::chart_sources::chart_source_cache_keys;

mod mtf_overlays;
pub(crate) use mtf_overlays::{
    ChartMtfOverlays, chart_result_cache_get, chart_result_cache_put, mtf_grid_value_get,
    mtf_grid_value_put, mtf_htf_cache_put,
};

// Chart view-model types + indicator palette now live in the typhoon-chart-ui crate
// (ADR-125 Target 2, slice 5); re-exported so chart.rs + the app glob are unchanged.
pub(crate) use typhoon_chart_ui::models::{
    ADX_COL, CCI_COL, CHART_RIGHT_MARGIN, HMA_COL, IndicatorFlags, MFI_COL, PPO_LINE_COL, SAR_COL,
    STOCH_K_COL, TRIX_LINE_COL, ULTOSC_COL, WILLR_COL, WMA_COL, chart_price_pane_height,
};
// ChartCamera is constructed only by native chart tests now (ChartState owns it in-crate).
#[cfg(test)]
pub(crate) use typhoon_chart_ui::models::ChartCamera;

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
pub(crate) use indicator_compute::ChartIndicatorCompute;

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
    chart_live_tick_anchor_guard, chart_load_merged_equity_bars_from_cache,
    chart_log_merged_cache_load_done, chart_log_merged_cache_load_start,
    chart_merged_equity_cache_key, chart_merged_source_bar_counts, chart_missing_data_cache_key,
    chart_prefers_fresh_equity_source, set_chart_merge_primary_broker,
};

#[cfg(test)]
pub(crate) use equity_merge::{
    ChartSplit, chart_curated_known_splits, chart_equity_source_rank_for,
    chart_materialize_merged_equity_cache, chart_merge_equity_raw_bars,
    chart_merge_equity_raw_bars_with_primary, chart_persist_merged_equity_bars_to_cache,
};

// `ChartState` + its chart-local view/behavior, camera controls, and auto-fibonacci now
// live in the typhoon-chart-ui crate (ADR-125 Target 2, slice 6c). Re-exported so the app
// glob and all native chart code keep using `ChartState` unchanged; the broker/cache/gpu
// pipelines stay here as extension traits (ChartDataLoad/ChartIndicatorCompute/ChartMtfOverlays)
// + ChartSymbolMatch below.
pub(crate) use typhoon_chart_ui::state::ChartState;

/// Symbol-equivalence test for a chart viewport (ADR-125 Target 2, slice 6c). Native
/// extension trait, not an inherent method, because it normalizes through the native
/// market-data cache-key helper `normalize_market_data_symbol` (kept in typhoon-native);
/// `ChartState` lives in typhoon-chart-ui. Re-exported from `chart` so call sites keep
/// `chart.symbol_matches(…)` unchanged.
pub(crate) trait ChartSymbolMatch {
    fn symbol_matches(&self, symbol: &str) -> bool;
}
impl ChartSymbolMatch for ChartState {
    fn symbol_matches(&self, symbol: &str) -> bool {
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
