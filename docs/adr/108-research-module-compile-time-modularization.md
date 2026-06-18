# ADR-108: Research Module Compile-Time Modularization

**Status:** Accepted
**Date:** 2026-06-06

## Context

`typhoon-engine::core::research` had become the dominant engine compile-time and rust-analyzer hot spot. Before this ADR, `engine/src/core/research/mod.rs` contained roughly 90k lines, including public DTOs, constants, provider fetchers, SQLite helpers, and many compute surfaces.

Measured before the first split:

- Warm `cargo check -p typhoon-engine`: about 9.4s.
- Touching a small engine helper previously cost about 11.5s.
- Touching the research monolith previously cost about 11.9s.
- `engine/src/core/research/mod.rs`: 90,269 lines.

The terminal already uses `mold` and high parallelism, so the next useful compile-speed work is structural: reduce the blast radius of the research module and eventually isolate research from broker/storage edit loops.

## Decision

Split research in verified increments while preserving the public API through root re-exports.

Initial structure:

- `engine/src/core/research/mod.rs`
  - orchestration, existing fetch/cache/compute code not yet extracted
  - `pub use` re-exports for extracted modules
- `engine/src/core/research/types.rs`
  - public research DTOs and constants formerly at the top of `mod.rs`
- `engine/src/core/research/technical.rs`
  - TECH compute surface (`compute_technical_indicators`) and direct dependencies
- `engine/src/core/research/providers.rs`
  - small external provider fetchers for Finnhub, FMP transcript endpoints, and Yahoo quotes
- `engine/src/core/research/storage_core.rs`
  - first-generation SQLite schema/helpers for profiles, peers, earnings, press, sentiment, transcripts, and IPO calendar
- `engine/src/core/research/storage_market_data.rs`
  - v2-v5 SQLite market/fundamentals cache helpers for dividends, estimates, ratings, financials, executives, splits, holdings, recommendations, targets, ESG, index members, insider/institutional holders, shares float, historical prices, and earnings surprises
- `engine/src/core/research/storage_macro_snapshots.rs`
  - v6 macro/snapshot storage helpers for world indices, market movers, sector performance, and WACC snapshots
- `engine/src/core/research/storage_valuation_snapshots.rs`
  - v7 storage helpers for currency rates, beta, DDM, relative valuation, and FIGI snapshots
- `engine/src/core/research/storage_valuation_models.rs`
  - v8 storage helpers for HRA, DCF, SVM, options-chain, and implied-volatility snapshots
- `engine/src/core/research/storage_market_stat_snapshots.rs`
  - v9 storage helpers for seasonality, correlation, total-return, technical, and volatility-skew snapshots
- `engine/src/core/research/storage_fundamental_risk_snapshots.rs`
  - v10 storage helpers for leverage, accruals, realized volatility, free-cash-flow yield, and short-interest snapshots
- `engine/src/core/research/storage_financial_quality_snapshots.rs`
  - v11 storage helpers for Altman Z, Piotroski, OHLC volatility, EPS beat, and price-target dispersion snapshots
- `engine/src/core/research/storage_insider_dividend_momentum_snapshots.rs`
  - v12 storage helpers for insider activity, dividend growth, earnings momentum, sector rotation, and upside/downside momentum snapshots
- `engine/src/core/research/valuation.rs`
  - valuation and market-stat snapshot computations (`compute_wacc_snapshot`, beta/DDM/relative valuation/HRA/DCF/SVM) plus closely related option-expiry parsing helpers
- `engine/src/core/research/market_stats.rs`
  - market/statistical snapshot computations for IV rank, seasonality, peer correlation matrices, total return, and option volatility skew
- `engine/src/core/research/fundamental_stats.rs`
  - fundamental leverage and earnings-quality snapshot computations (`compute_leverage_snapshot`, `compute_accruals_snapshot`)
- `engine/src/core/research/return_risk_stats.rs`
  - dense return-distribution and risk-statistical snapshot computations from the return-risk feature families (`RETSKEW`, `RETKURT`, `TAILR`, drawdown/run-length/range/autocorrelation/fractal/normality/tail-risk surfaces, etc.)

Rules for future slices:

1. Move cohesive feature families, not arbitrary line ranges.
2. Preserve public names via `pub use` from `mod.rs`.
3. Run `cargo check -p typhoon-engine` after each extraction.
4. Run downstream `cargo check -p typhoon-native` before committing a migration slice.
5. Prefer extracting research/provider/storage crates only after module boundaries are stable enough to avoid circular dependencies.

## Follow-up Plan

Next structural targets, in order:

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
   - 2026-06-06 check: `sccache 0.15.0` is installed at `/usr/bin/sccache` with a local disk cache at `/home/typhoon/.cache/sccache`.
   - `.cargo/config.toml` now sets `rustc-wrapper = "sccache"` under `[build]`.
   - Verification: normal incremental `cargo check -p typhoon-engine` completed in 10.18s but was non-cacheable because Cargo incremental compilation is enabled; `CARGO_INCREMENTAL=0 cargo check -p typhoon-engine` executed through sccache with 2 Rust cache misses and no cache errors. Do not disable incremental globally for local dev; use `CARGO_INCREMENTAL=0` for CI/clean multi-branch cache reuse.
6. Keep `tokio-tungstenite` TLS cleanup gated behind LAN-sync verification, because LAN sync currently uses native-tls self-signed certificate handling.
   - `tokio-tungstenite` is still pulled with `native-tls` through `engine/Cargo.toml`.
   - `engine/src/core/lan_sync.rs` directly builds `native_tls::TlsAcceptor` / `TlsConnector`, wraps with `tokio_native_tls`, passes `Connector::NativeTls`, and reads peer certificates through `MaybeTlsStream::NativeTls`.
   - That means TLS cleanup is real but not a safe quick flag flip; migrate LAN sync to rustls or isolate LAN sync behind a feature before removing native-tls.

## Current Extraction Ranking

**Update (2026-06): the original goal is achieved.** The root `research/mod.rs`
went ~90k → ~36.8k → **1,668 lines** and is **no longer the compile/rust-analyzer
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
| `research/technical_indicator_models.rs` | ~3,806 | TA indicator compute. |
| `research/moving_average_oscillator_models.rs` | ~3,630 | MA/oscillator compute. |
| `research/mod.rs` | ~1,668 | Orchestration + residual storage. No longer the hotspot. |

### Next targets (in order)

1. **Move to the next engine hotspots.** `return_risk_stats`, `candlestick_pattern_models`, and `price_transform_indicator_models` are now thin parents; the next high-value engine files are `technical_indicator_models.rs` and `moving_average_oscillator_models.rs`. Split each by semantic indicator family while preserving the `pub use` re-export surface.
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

- This first slice is not enough to solve engine-wide invalidation by itself.
- `mod.rs` remains large and needs more semantic extractions.
- Crate extraction is deferred until dependency cycles are resolved deliberately rather than by whack-a-mole call-site rewrites.
