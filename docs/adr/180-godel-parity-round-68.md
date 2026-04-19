# ADR-180: Godel Parity Round 68 — ROC / ROCP / ROCR / ROCR100 / CORREL

**Status:** Accepted
**Date:** 2026-04-18
**Supersedes/extends:** ADR-108 through ADR-179
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 67 (ADR-179) shipped the Wilder Directional Movement System
(PLUS_DI / MINUS_DI / PLUS_DM / MINUS_DM / DX). Round 68 now closes
the last remaining TA-Lib "rate-of-change family" gap — four nearly-
identical primitives that differ only in their scaling convention —
plus the general-purpose **CORREL** primitive specialised here to a
single-symbol lag-1 autocorrelation.

1. **No ROC snapshot.** TA-Lib's raw Rate of Change
   `ROC_t = close_t − close_{t−n}` (period 10). The only raw-price-
   delta primitive in the TA-Lib catalog — distinct from MOM (same
   formula, different TA-Lib name) in that MOM is documented as an
   oscillator while ROC is documented as a "rate". Godel parity
   agents cross-referencing TA-Lib docs expect both to be present
   and indexable separately. Header gives **roc_label**
   (STRONG_UP / UP / NEUTRAL / DOWN / STRONG_DOWN / INSUFFICIENT_DATA)
   using `roc/close_lag × 100` percentage cutoffs (≥5 strong_up,
   ≥1 up, ≤−1 down, ≤−5 strong_down, else neutral) so the
   label is invariant to absolute price level.

2. **No ROCP snapshot.** `ROCP_t = (close_t − close_{t−n}) / close_{t−n}`
   — the *percentage*-form rate of change (unitless) used widely in
   risk-return math. Distinct from ROC in that ROCP divides by the
   lagged price, giving a compound-friendly ratio rather than an
   absolute delta. Header gives **rocp_label** (same 5-band scheme
   as ROC, fed by `rocp × 100`) plus a convenience `rocp_pct`
   field so downstream consumers avoid the `rocp * 100.0`
   multiplication.

3. **No ROCR snapshot.** `ROCR_t = close_t / close_{t−n}` — the
   ratio form. `1.0` is unchanged, `>1` up, `<1` down. Direct input
   for compounding return aggregations (e.g. `prod(ROCR_i) − 1`
   gives total return over a window). Header gives **rocr_label**
   using the same 5-band scheme fed by `(rocr − 1) × 100`.

4. **No ROCR100 snapshot.** `ROCR100_t = 100 · close_t / close_{t−n}` —
   the index-100 form. `100` is unchanged, `>100` up, `<100` down.
   Scales ROCR to an index-like band directly comparable to CCI / PPO /
   ADX with zero unit-mismatch — useful when agents are composing
   multi-indicator signals without per-signal rescaling. Header gives
   **rocr100_label** using the same 5-band scheme fed by
   `rocr100 − 100`.

5. **No CORREL snapshot.** TA-Lib CORREL computes a rolling Pearson
   correlation of two input series. For a per-symbol snapshot (where
   there is only one series), instantiate as the **lag-1
   autocorrelation** of close: `ρ(close_t, close_{t−1})` over the
   last 30 bars. Provides a scalar signal for serial dependence:
   `ρ → +1` is strong momentum (consecutive closes move together),
   `ρ → 0` is a random walk, `ρ → −1` is strong mean reversion.
   This is the only primitive in Round 68 that is *not* a simple
   re-expression of a price delta — it exposes higher-order
   structure the other four cannot. Header gives **correl_label**
   (STRONG_MOMO / MOMO / RANDOM_WALK / MEAN_REVERT / STRONG_MEAN_REVERT
   / INSUFFICIENT_DATA) using ρ cutoffs (≥0.7 strong_momo, ≥0.2
   momo, ≤−0.7 strong_mean_revert, ≤−0.2 mean_revert, else random_walk).

## Decision

Add five optional per-symbol snapshot blocks to the Godel-parity
pipeline, each reusing the existing `research_historical_price` HP
cache and the standard research-table LAN-sync path (no new API
dependencies):

1. `research::RocSnapshot` + `compute_roc_snapshot` + `upsert_roc` +
   `get_roc` — serialised to `research_roc`.
2. `research::RocpSnapshot` + `compute_rocp_snapshot` + `upsert_rocp`
   + `get_rocp` — serialised to `research_rocp`.
3. `research::RocrSnapshot` + `compute_rocr_snapshot` + `upsert_rocr`
   + `get_rocr` — serialised to `research_rocr`.
4. `research::Rocr100Snapshot` + `compute_rocr100_snapshot` +
   `upsert_rocr100` + `get_rocr100` — serialised to `research_rocr100`.
5. `research::CorrelSnapshot` + `compute_correl_snapshot` +
   `upsert_correl` + `get_correl` — serialised to `research_correl`.

The four ROC-family compute_* fns share a small private `roc_label`
helper that maps a percentage-form delta to the 5-label band — since
all four reduce to the same "STRONG_UP/UP/NEUTRAL/DOWN/STRONG_DOWN"
classification fed by different percentage computations, factoring
the classifier out avoids four separate copies of the same cutoffs
(and the drift risk if one gets tweaked independently).

`compute_correl_snapshot` implements a textbook Pearson correlation
with `sxx / syy / sxy` accumulators in one O(n) pass per bar —
memoised nowhere because we only evaluate at `n−1` and `n−2` for the
`correl_prev` field. Cost is bounded to O(2·period) per call.

Schema version bumps to v70 via `create_research_tables_v70` which
wraps v69 and adds five new CREATE TABLE stanzas (plus indexes on
`updated_at`). All five tables register in `SYNCABLE_TABLES`,
`create_table_sql`, and `table_timestamp_column` so LAN sync
incrementally propagates them to peer terminals.

Native wiring adds five `BrokerCmd::Compute*Snapshot` variants, five
`BrokerMsg::*SnapshotMsg` variants, five `show_*_win` / `*_symbol` /
`*_snapshot` / `*_loading` field tuples on `App`, five tokio-spawned
broker handlers (load HP cache → compute → upsert → emit msg), five
palette alias blocks, five packet-emitter blocks after the Round 67
PLUS_DI / MINUS_DI / PLUS_DM / MINUS_DM / DX emitters, five egui
windows with Use-Chart / Load-Cached / Compute controls plus a
striped Grid summary, and five `BrokerMsg` match arms.

Palette aliases were selected to avoid collision with earlier rounds
(verified at implementation time against R60..R67 token sets):
`ROC | ROC_WILDER | ROCWIN | ROCRATE | RATE_OF_CHANGE`;
`ROCP | ROCP_WILDER | ROCPWIN | ROCPCT | ROC_PCT`;
`ROCR | ROCR_WILDER | ROCRWIN | ROCRATIO | ROC_RATIO`;
`ROCR100 | ROCR100_WILDER | ROCR100WIN | ROCR100IDX | ROC_RATIO_100`;
`CORREL | CORRWIN | ROLLCORR | AUTOCORR | PEARSON_AUTO`. All 25
tokens are fresh — zero collisions with earlier rounds. `ROC`
itself was available because MOM (Round 65) shipped as `MOM / MOMX /
MOMWIN / MOMENTUM / MOMSCALE` and never claimed either `ROC` or any
rate-prefixed variants.

## Consequences

- Packet scope grows by up to 10 k/v rows per symbol when all five
  snapshots are populated — roughly +200 bytes for ROC (raw delta +
  prev + close + lag), +230 bytes for ROCP (adds pct field),
  +200 bytes for ROCR, +200 bytes for ROCR100, +300 bytes for CORREL
  (mean/stddev for both x and y series) — for a typical +1.13 KB
  per symbol.
- Schema is strictly additive; old peers running v69 continue to
  work (new tables are absent but none of the old tables change).
  LAN sync skips unknown tables via the whitelist.
- All five indicators share the HP cache with zero additional
  network dependencies. The four ROC-family snapshots need n≥12
  bars (period 10 + 2 for the prev-bar `*_prev` field); CORREL
  needs n≥32 bars (period 30 + 2).
- Like Round 67 + earlier rounds, the 10 Round 68 tests (5
  roundtrip + 5 compute_oscillating) guard against serialization
  drift and band-cutoff regressions; the ROC-family tests each
  verify the mathematical identity (`rocp = roc / close_lag`,
  `rocr = close_now / close_lag`, etc.) in addition to label
  validity, guaranteeing the four sibling computes don't drift
  apart.

## Verification

1. **Engine tests:** `cargo test --package typhoon-engine --lib`
   passes with +10 Round 68 tests over Round 67's count (1415 total
   including Round 68 additions).
2. **Native build:** `cargo build --package typhoon-native` completes
   cleanly with no warnings/errors in 3m 56s.
3. **Unique palette tokens:** All 25 Round 68 palette tokens fresh —
   zero collisions with earlier rounds (verified against the 25 Round
   67 tokens and the cumulative R60..R66 set).
4. **LAN sync whitelist:** All five `research_*` tables registered
   in `SYNCABLE_TABLES`, `create_table_sql`, and
   `table_timestamp_column`; incremental sync uses the `updated_at`
   column.
5. **Sibling-identity coverage:** the rocp_compute_oscillating,
   rocr_compute_oscillating, and rocr100_compute_oscillating tests
   each re-derive the primitive from `close_now` and `close_lag` and
   compare to the stored scalar, guarding against refactor drift.

## Packet envelope delta

Before Round 68: packet emitted 136 k/v rows across Round 60 +
Round 61 + Round 62 + Round 63 + Round 64 + Round 65 + Round 66 +
Round 67 additions. After Round 68: 146 k/v rows when all forty-five
Round 60..68 additions populate, typical +1.13 KB per symbol on top
of the +1.22 KB Round 67 added, +1.05 KB Round 66 added, +1.45 KB
Round 65 added, +1.45 KB Round 64 added, +1.45 KB Round 63 added,
+1.45 KB Round 62 added, +1.40 KB Round 61 added, and +1.46 KB
Round 60 added — bringing the observed per-symbol envelope from
~93-177 KB to ~94-178 KB.
