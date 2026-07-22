# ADR-108: Research Module Compile-Time Modularization

**Status:** Accepted
**Date:** 2026-06-06

## Context

`typhoon-engine::core::research` had become the dominant engine compile-time and rust-analyzer hot spot. Before this ADR, `typhoon-engine/src/core/research/mod.rs` contained roughly 90k lines, including public DTOs, constants, provider fetchers, SQLite helpers, and many compute surfaces.

Measured before the first split:

- Warm `cargo check -p typhoon-engine`: about 9.4s.
- Touching a small engine helper previously cost about 11.5s.
- Touching the research monolith previously cost about 11.9s.
- `typhoon-engine/src/core/research/mod.rs`: 90,269 lines.

The terminal already uses `mold` and high parallelism, so the next useful compile-speed work is structural: reduce the blast radius of the research module and eventually isolate research from broker/storage edit loops.

## Decision

Split research in verified increments while preserving the public API through root re-exports.

Initial structure:

- `typhoon-engine/src/core/research/mod.rs`
  - orchestration, existing fetch/cache/compute code not yet extracted
  - `pub use` re-exports for extracted modules
- `typhoon-engine/src/core/research/types.rs`
  - public research DTOs and constants formerly at the top of `mod.rs`
- `typhoon-engine/src/core/research/technical.rs`
  - TECH compute surface (`compute_technical_indicators`) and direct dependencies
- `typhoon-engine/src/core/research/providers.rs`
  - small external provider fetchers for Finnhub, FMP transcript endpoints, and Yahoo quotes
- `typhoon-engine/src/core/research/storage_core.rs`
  - first-generation SQLite schema/helpers for profiles, peers, earnings, press, sentiment, transcripts, and IPO calendar
- `typhoon-engine/src/core/research/storage_market_data.rs`
  - v2-v5 SQLite market/fundamentals cache helpers for dividends, estimates, ratings, financials, executives, splits, holdings, recommendations, targets, ESG, index members, insider/institutional holders, shares float, historical prices, and earnings surprises
- `typhoon-engine/src/core/research/storage_macro_snapshots.rs`
  - v6 macro/snapshot storage helpers for world indices, market movers, sector performance, and WACC snapshots
- `typhoon-engine/src/core/research/storage_valuation_snapshots.rs`
  - v7 storage helpers for currency rates, beta, DDM, relative valuation, and FIGI snapshots
- `typhoon-engine/src/core/research/storage_valuation_models.rs`
  - v8 storage helpers for HRA, DCF, SVM, options-chain, and implied-volatility snapshots
- `typhoon-engine/src/core/research/storage_market_stat_snapshots.rs`
  - v9 storage helpers for seasonality, correlation, total-return, technical, and volatility-skew snapshots
- `typhoon-engine/src/core/research/storage_fundamental_risk_snapshots.rs`
  - v10 storage helpers for leverage, accruals, realized volatility, free-cash-flow yield, and short-interest snapshots
- `typhoon-engine/src/core/research/storage_financial_quality_snapshots.rs`
  - v11 storage helpers for Altman Z, Piotroski, OHLC volatility, EPS beat, and price-target dispersion snapshots
- `typhoon-engine/src/core/research/storage_insider_dividend_momentum_snapshots.rs`
  - v12 storage helpers for insider activity, dividend growth, earnings momentum, sector rotation, and upside/downside momentum snapshots
- `typhoon-engine/src/core/research/valuation.rs`
  - valuation and market-stat snapshot computations (`compute_wacc_snapshot`, beta/DDM/relative valuation/HRA/DCF/SVM) plus closely related option-expiry parsing helpers
- `typhoon-engine/src/core/research/market_stats.rs`
  - market/statistical snapshot computations for IV rank, seasonality, peer correlation matrices, total return, and option volatility skew
- `typhoon-engine/src/core/research/fundamental_stats.rs`
  - fundamental leverage and earnings-quality snapshot computations (`compute_leverage_snapshot`, `compute_accruals_snapshot`)
- `typhoon-engine/src/core/research/return_risk_stats.rs`
  - dense return-distribution and risk-statistical snapshot computations from the return-risk feature families (`RETSKEW`, `RETKURT`, `TAILR`, drawdown/run-length/range/autocorrelation/fractal/normality/tail-risk surfaces, etc.)

Rules for future slices:

1. Move cohesive feature families, not arbitrary line ranges.
2. Preserve public names via `pub use` from `mod.rs`.
3. Run `cargo check -p typhoon-engine` after each extraction.
4. Run downstream `cargo check -p typhoon-native` before committing a migration slice.
5. Prefer extracting research/provider/storage crates only after module boundaries are stable enough to avoid circular dependencies.

## Historical Follow-up Plan (implemented or superseded)

These were the ordered targets used during the extraction. The storage families
below now exist under finer semantic names; the live remaining work is recorded
under **Current Extraction Ranking**.

1. Continue extracting storage families:
   - next storage slices should be smaller migration/version families (`storage_market_rates.rs`, `storage_quant_snapshots.rs`, `storage_indicator_snapshots.rs`) rather than one giant all-storage dump.
   - keep `storage_core.rs` focused on first-generation DES/PEERS/EARNINGS/PRESS/SENTIMENT/TRANSCRIPTS/IPO cache helpers.
   - keep `storage_market_data.rs` focused on v2-v5 market/fundamentals cache helpers.
   - keep `storage_macro_snapshots.rs` focused on v6 macro/snapshot cache helpers.
   - keep `storage_valuation_snapshots.rs` focused on v7 valuation/reference cache helpers.
   - keep `storage_valuation_models.rs` focused on v8 model-output/options cache helpers.
   - keep `storage_market_stat_snapshots.rs` focused on v9 market-stat/technical cache helpers.
   - keep `storage_fundamental_risk_snapshots.rs` focused on v10 fundamental-risk/cash-flow/short-interest cache helpers.
   - keep `storage_financial_quality_snapshots.rs` focused on v11 financial-quality/earnings/dispersion cache helpers.
   - keep `storage_insider_dividend_momentum_snapshots.rs` focused on v12 insider/dividend/earnings/rotation/upside-momentum cache helpers.
2. Then split remaining research compute families into semantic modules:
   - risk/correlation surfaces
   - high-volume return distribution/statistical surfaces
   - TA/indicator parity surfaces
3. Extract shared lightweight domain types to a small crate only when needed to break cycles.
4. Extract `typhoon-research` once dependencies on `crate::core::fundamentals` and `crate::core::sec_filing` have been inverted or moved to shared crates.
5. Keep broker/cache hot paths out of the research crate so a Kraken/Alpaca sync edit does not invalidate heavy research code.
6. Use `sccache` as the local rustc wrapper when installed/configured on the machine; do not set `rustc-wrapper` to a missing binary.
   - 2026-07-22 check: `sccache 0.16.0` is installed at `/usr/bin/sccache`; `.cargo/config.toml` still configures it as the workspace wrapper.
   - `.cargo/config.toml` now sets `rustc-wrapper = "sccache"` under `[build]`.
   - Verification: normal incremental `cargo check -p typhoon-engine` completed in 10.18s but was non-cacheable because Cargo incremental compilation is enabled; `CARGO_INCREMENTAL=0 cargo check -p typhoon-engine` executed through sccache with 2 Rust cache misses and no cache errors. Do not disable incremental globally for local dev; use `CARGO_INCREMENTAL=0` for CI/clean multi-branch cache reuse.
7. LAN sync and its native-TLS implementation were removed under ADR-115. The
   former TLS blocker is closed; current dependency policy is tracked by
   ADR-031/088 rather than this research-modularization ADR.

## Current Extraction Ranking

**Update (2026-06): the original goal is achieved.** The root `research/mod.rs`
went ~90k → ~36.8k → **1,759 lines** and is **no longer the compile/rust-analyzer
hotspot**. The final two cuts were:

- **Candlestick-pattern storage** (v80/v83–v88 `create/upsert/get_cdl_*`) →
  `storage_candlestick_extended_snapshots.rs` (~1.5k lines).
- **The ~21,793-line inline `#[cfg(test)] mod tests`** (93% of the file, 1,030
  tests) was extracted to `tests.rs`, then split into a semantic `research/tests/`
  tree via `include!` — see **ADR-118** for the convention and the
  shared-fixture rationale.

`mod.rs` now holds module declarations + residual storage helpers (v56 expirations,
v89–v93 rank/short-interest/insider, and the trailing TA-indicator snapshot
upsert/get). The dominant research files are now the **compute-model files**, not
the root:

| File | Lines | Notes |
| --- | ---: | --- |
| `research/types.rs` | ~165 | Public DTO root/re-export surface after semantic type-family splits; no longer a hotspot. |
| `research/return_risk_stats.rs` | ~55 | Thin re-export parent after the return-risk compute families were split into semantic children, including `distribution_shape.rs`, `autocorr_regime.rs`, `downside_efficiency.rs`, `drawdown_liquidity_normality.rs`, `drawup_gap_range.rs`, `seasonality_spread.rs`, `volatility_estimators.rs`, `performance_runs_tests.rs`, `significance_stationarity.rs`, `tail_risk_diagnostics.rs`, `entropy_dependence.rs`, `upside_drawdown_risk.rs`, `entropy_stationarity.rs`, `robust_quantile_volatility.rs`, `normality_liquidity_tail.rs`, `fractal_rank_dynamics.rs`, `jump_trend_diagnostics.rs`, and `spectral_nonlinear_diagnostics.rs`. |
| `research/candlestick_pattern_models.rs` | ~80 | Thin candlestick parent/helpers after extracting `basic_reversal.rs`, `multibar_reversal.rs`, `doji_shadow_star.rs`, `body_line_shapes.rs`, `neck_line_reversal.rs`, `crow_line_reversal.rs`, `separating_sandwich_doji.rs`, `rare_multibar_reversal.rs`, `gap_breakaway_reversal.rs`, and `continuation_gap_patterns.rs`. |
| `research/price_transform_indicator_models.rs` | ~21 | Thin price-transform parent after extracting `adaptive_average_transforms.rs`, `price_average_variance.rs`, `directional_movement.rs`, `rate_correlation.rs`, `rolling_extrema.rs`, `bands_accumulation_regression.rs`, `aroon_macd_variable_average.rs`, `oscillator_squeeze_range.rs`, and `regression_hilbert_oscillators.rs`. |
| `research/technical_indicator_models.rs` | ~12 | Thin technical-indicator parent after extracting `squeeze_trend_channels.rs`, `directional_flow_trend.rs`, `volume_momentum_oscillators.rs`, `momentum_volume_pressure.rs`, and `cycle_trend_value.rs`. |
| `research/moving_average_oscillator_models.rs` | ~53 | Thin MA/oscillator parent with shared EMA/SMA helpers after extracting `regression_pivot_candles.rs`, `adaptive_forecast_vigor.rs`, `adaptive_volume_momentum.rs`, `acceleration_range_impulse.rs`, `momentum_envelope_volume.rs`, and `adaptive_cycle_volume.rs`. |
| `research/mod.rs` | 1,759 | Orchestration + residual storage. No longer the hotspot. |

### Next targets (in order)

1. **Move to current native/runtime hotspots.** The earlier chart/render targets moved to `typhoon-chart-ui` under ADR-125. Current high-value compile-time wins are `typhoon-native/src/app/state.rs`, `trade_ops.rs`, `market_data_sync.rs`, `app_runtime_central_panel.rs`, and the larger broker-runtime research-compute children such as `typhoon-broker-runtime/src/research_compute/technical_indicators/candlestick_patterns.rs`.
2. **Extract residual `mod.rs` storage** into `storage_*` modules if `mod.rs` regrows.
3. **Keep semantic type modules and re-exports stable.** `types.rs` is now a small root surface; future DTO additions should land in the matching semantic child module, not back in a monolith.

Do not start a full `typhoon-research` crate split yet. The module is still entangled with `crate::core::{fundamentals, sec_filing, cache}`; crate extraction comes after the compute-model files are sub-split and the re-export surface is stable.

## Consequences

Positive:

- Return-distribution/risk-statistical compute edits no longer require editing the root research file, and the first return-risk families now compile as child modules.

- Market-stat compute edits no longer require editing the root research file.
- Fundamental leverage/accrual compute edits no longer require editing the root research file.
- Valuation compute edits no longer require editing the root research file.
- V2-v12 market/fundamentals/macro/valuation/model-output/market-stat/fundamental-risk/financial-quality/insider-dividend-momentum cache edits no longer require editing the root research file.
- First-generation storage/cache edits no longer require editing the root research file.
- DTO/constant edits no longer require editing the root 80k+ line research file.
- TECH compute edits are isolated into a small module.
- The public API remains compatible for downstream callers.
- This creates a safer path toward a future `typhoon-research` crate.

Tradeoffs:

- Intra-crate module splits improve ownership, navigation, and incremental-query
  locality but do not create separate compilation units; crate boundaries remain
  the stronger compile-time lever.
- `mod.rs` is no longer a dominant hotspot. Extract residual storage only if it
  regrows or a cohesive ownership boundary warrants the move.
- Crate extraction is deferred until dependency cycles are resolved deliberately rather than by whack-a-mole call-site rewrites.

## Update (2026-07-02) — research-ui snapshot-renderer segmentation

The same program applied to the UI side of research:
`typhoon-research-ui/src/render.rs` had grown to **23,312 lines** (64% of the
crate, 5× the largest remaining file in the workspace) holding 259 uniform,
self-contained `pub fn render_*_snapshot(&mut egui::Ui, &Snapshot)` free
functions. It was split mechanically, preserving file order, into eight
segment modules `render/s01_avgprice.rs` … `render/s08_updm.rs` (~2.9k lines,
32–33 renderers each; each stem names its first renderer so the numbered
files stay self-indexing), glob re-exported from `render.rs` — callers keep
the `render::render_*_snapshot` paths unchanged. Segment boundaries are
mechanical, not semantic: hand-sorting 259 quant surfaces into families
risks miscategorization that would mislead more than neutral chunking.

Measured honestly (warm incremental caches, 44-core workstation): a
touch-only `cargo check -p typhoon-research-ui` is ~0.8s both before and
after the split — rustc's incremental system already handled that case — and
a real one-function edit checks in ~1.9s after the split. The win is
edit/review/diff locality, not check latency. The cold-cache check of the
crate (~44s) is unchanged by intra-crate splitting because a crate is still
one rustc invocation.

That points at the actual compile-time lever, consistent with this ADR's
crate-split caution: the segmented renderer corpus is now the cleanest
**leaf-crate extraction candidate** in the workspace (pure functions over
`egui` + `typhoon_engine` research DTOs + the small `theme` constants; no
`TyphooNApp`, no storage). Extracting it would take the renderer corpus out
of every non-render `typhoon-research-ui` rebuild. Prerequisite to resolve
deliberately: `theme` placement (shared by `window_shell`/`packet`), and the
same rule as the deferred engine research crate — extract only when the
dependency direction is clean, not to relocate a hotspot.

### Next targets (unchanged order, engine + native)

The ADR's earlier native/runtime hotspot list stands: `state.rs`,
`trade_ops.rs`, `market_data_sync.rs`, `app_runtime_central_panel.rs`, and
the broker-runtime research-compute children. `typhoon-engine/src/broker/alpaca.rs`
(4.7k lines) and `core/cache.rs` (2.6k) are the engine-side files most likely
to warrant the ADR-118 treatment when next touched semantically.
