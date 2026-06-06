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
- `engine/src/core/research/valuation.rs`
  - valuation and market-stat snapshot computations (`compute_wacc_snapshot`, beta/DDM/relative valuation/HRA/DCF/SVM) plus closely related option-expiry parsing helpers

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
2. Then split remaining research compute families into semantic modules:
   - risk/correlation surfaces
   - market/seasonality surfaces
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

After extracting `providers.rs`, `storage_core.rs`, `storage_market_data.rs`, and `valuation.rs`, the root research file is still the dominant target:

| File | Lines | Notes |
| --- | ---: | --- |
| `engine/src/core/research/mod.rs` | ~77,948 | Still the primary compile/rust-analyzer hotspot. |
| `engine/src/core/research/types.rs` | ~9,342 | Already extracted; leave alone unless type ownership needs cleanup. |
| `engine/src/core/darwin.rs` | ~7,055 | Secondary candidate, but smaller and already has proven child-module patterns. |
| `engine/src/broker/alpaca.rs` | ~4,467 | Broker split candidate, but lower impact than research. |
| `engine/src/core/research/valuation.rs` | ~1,132 | Extracted valuation/market-stat compute slice. |
| `engine/src/core/research/storage_market_data.rs` | ~661 | Extracted v2-v5 market/fundamentals storage slice. |
| `engine/src/core/research/storage_core.rs` | ~501 | Extracted first-generation storage slice; keep as low-level cache helper boundary. |
| `engine/src/core/research/providers.rs` | ~390 | Extracted first provider slice. |

Next best research slice is not another provider fetcher; it is a semantic compute/storage family from the remaining root file. Good candidates:

1. `research/storage.rs` for schema/create/upsert/get helper families once the public API surface is inventoried.
2. `research/quant_stats.rs` for the dense return/risk/statistical compute family currently clustered around the 13k-22k range.
3. `research/indicator_snapshots.rs` for the large TA-style `compute_*_snapshot` families around 22k-31k, coordinated with existing `technical.rs` so indicator naming/parity stays clean.

Do not start with a full `typhoon-research` crate split yet. The module is still entangled with `crate::core::{fundamentals, sec_filing, cache}` and LAN/broker infrastructure; crate extraction should come after at least two more semantic submodule cuts and API re-export stabilization.

## Consequences

Positive:

- Valuation/market-stat compute edits no longer require editing the root research file.
- V2-v5 market/fundamentals cache edits no longer require editing the root research file.
- First-generation storage/cache edits no longer require editing the root research file.
- DTO/constant edits no longer require editing the root 80k+ line research file.
- TECH compute edits are isolated into a small module.
- The public API remains compatible for downstream callers.
- This creates a safer path toward a future `typhoon-research` crate.

Tradeoffs:

- This first slice is not enough to solve engine-wide invalidation by itself.
- `mod.rs` remains large and needs more semantic extractions.
- Crate extraction is deferred until dependency cycles are resolved deliberately rather than by whack-a-mole call-site rewrites.
