# ADR-195: Parity Expansion R89-R90 — Deferred Benchmark and Peer-Relative Momentum Surfaces

**Status:** Accepted  
**Date:** 2026-04-21  
**Extends:** ADR-194  
**Related:** `engine/src/core/research.rs`, `engine/src/core/lan_sync.rs`, `native/src/app.rs`, ADR-128 (future-work list), ADR-188 (chart-drawing parity deferred)

## Context

After closing the TA-Lib candlestick backlog in ADR-194, the next
meaningful parity gaps were not more `CDL*` surfaces but two
long-deferred Godel research items:

- `MOMRANK_MULTI` — sector-relative multi-horizon momentum rank
- `CORRSTK` — benchmark correlation vs SPY and sector ETF

These were deferred in earlier ADRs because the original compute path
would have required repeated peer historical-price scans or brittle
dependence on a warmed benchmark cache.

That constraint has changed:

- `PRICEPERF` snapshots now exist and can be reused for peer-relative
  momentum ranking without rescanning every peer's HP bars.
- `BETA` already established the benchmark-bar fetch pattern, so
  `CORRSTK` can reuse the same FMP historical-price path with cache
  fallback instead of assuming SPY is already resident.

## Scope

### Round 89

- `MOMRANK_MULTI`

### Round 90

- `CORRSTK`

Both features are research-packet and popup surfaces only. Chart
overlays remain deferred by ADR-188.

## Decision

Ship both deferred surfaces as one bundled parity pass.

Implementation details:

- `MOMRANK_MULTI`
  uses cached `PRICEPERF` snapshots for the subject and same-sector
  peers, percentile-ranks each horizon (`1M`, `3M`, `6M`, `YTD`, `1Y`),
  and blends them into one composite percentile.
- `CORRSTK`
  aligns daily log returns for the subject against `SPY` plus an
  optional sector ETF benchmark, then exposes rolling `20d`, `60d`, and
  `252d` correlations with long-window beta / R² context.

The additive schema layer is:

- `v89`: `research_momrank_multi`, `research_corrstk`

For every feature in this ADR:

- Research packet: yes
- egui popup: yes
- Palette aliases: yes
- LAN sync: yes
- Chart overlay: no, deferred by ADR-188

## Consequences

### Positive

- The parity sweep resumes on deferred Godel surfaces instead of
  continuing low-yield candlestick expansion.
- `MOMRANK_MULTI` now reuses the existing `PRICEPERF` cache layer
  efficiently, which is cleaner than the originally rejected
  peer-bar-scan design.
- `CORRSTK` becomes robust enough for interactive use by combining cache
  reuse with the same benchmark-fetch path already used for `BETA`.

### Negative / risks

- `CORRSTK` still depends on benchmark bar availability when no FMP key
  is configured and the relevant benchmark history is missing locally.
- `MOMRANK_MULTI` depends on peer `PRICEPERF` snapshots being present;
  sparse caches will still yield `INSUFFICIENT_DATA`.
- The additive schema chain grows to `v89`.

### Neutral

- This continues the consolidated ADR bundling pattern used since
  ADR-189.
- The bundle mixes one pure cache-reuse surface and one benchmark-fetch
  surface, but both still fit the same research/popup architecture.

## Verification

- `cargo test --manifest-path engine/Cargo.toml momrank_multi -- --nocapture`
  verifies the new `MOMRANK_MULTI` compute and roundtrip coverage.
- `cargo test --manifest-path engine/Cargo.toml corrstk -- --nocapture`
  verifies the new `CORRSTK` compute and roundtrip coverage.
- `cargo check --manifest-path native/Cargo.toml`
  verifies the broker wiring, cache persistence, research packet output,
  palette aliases, and popup windows.
