# ADR-181: TA-Lib Parity Round 69 — MIN / MAX / MINMAX / MININDEX / MAXINDEX

**Status:** Accepted
**Date:** 2026-04-18
**Supersedes/extends:** ADR-108 through ADR-180
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| MIN | No | Yes (`MIN`) | Yes | Yes | No (deferred — ADR-188) |
| MAX | No | Yes (`MAX`) | Yes | Yes | No (deferred — ADR-188) |
| MINMAX | No | Yes (`MINMAX`) | Yes | Yes | No (deferred — ADR-188) |
| MININDEX | No | Yes (`MININDEX`) | Yes | Yes | No (deferred — ADR-188) |
| MAXINDEX | No | Yes (`MAXINDEX`) | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure TA-Lib — the rolling-window extrema family (`MIN`, `MAX`, `MINMAX` levels + range) plus the recency scalars (`MININDEX`, `MAXINDEX` bars-since-extreme).

## Context

Round 68 (ADR-180) shipped the TA-Lib rate-of-change family plus CORREL
(ROC / ROCP / ROCR / ROCR100 / CORREL). Round 69 now covers the
**rolling-window extrema family** — the five TA-Lib primitives that
surface "where is close relative to the N-bar high/low, and how fresh
are those extrema?". These primitives are foundational for Donchian
channels, breakout detection, and regime classification (tight range
vs. trending), yet none of them were individually addressable from the
research packet or palette. Godel parity agents asking "is this a
20-bar breakout?" or "how long has this been the swing high?" had no
data to answer without recomputing the rolling window themselves.

1. **No MIN snapshot.** TA-Lib's `MIN_t = min(close_{t-n+1..t})` over
   a 30-bar window. The rolling-window support level — the standard
   Donchian-low surface. Header gives **min_label** (NEAR_LOW / MID /
   NEAR_HIGH / INSUFFICIENT_DATA) from `position_pct = 100·(close −
   min) / (max − min)` (≤20% near_low, ≥80% near_high, else mid),
   plus `min_prev` (one bar back) for change detection.

2. **No MAX snapshot.** Mirror: `MAX_t = max(close_{t-n+1..t})` over
   a 30-bar window. Rolling-window resistance level — the Donchian-
   high surface. Distinct from MIN in that the label inverts
   (NEAR_HIGH when position is high, NEAR_LOW when low), even though
   the underlying `position_pct` scalar is identical. Having both in
   the packet lets agents route label-based signals without
   re-deriving position from raw price.

3. **No MINMAX snapshot.** Combines both endpoints in one snapshot
   plus derived `range_width` (max − min, price-space) and
   `range_pct` (100·range_width / close, unit-free). Header gives
   **minmax_label** (RANGE_WIDE / RANGE_NORMAL / RANGE_TIGHT /
   INSUFFICIENT_DATA) from `range_pct` thresholds (≥8% wide, ≤3%
   tight, else normal). Distinct from MIN/MAX — those emit scalar
   levels, this emits regime classification suitable for gating
   breakout strategies (tight range → setup, wide range → already
   trending).

4. **No MININDEX snapshot.** The recency of the window minimum —
   `min_index_bars_ago ∈ [0, period−1]`, with 0 meaning "the current
   bar *is* the window low" and `period−1` meaning "the low was at
   the start of the window." Header gives **min_index_label**
   (FRESH_LOW / RECENT_LOW / OLD_LOW / STALE_LOW /
   INSUFFICIENT_DATA) from bars_ago bands (≤period/6 fresh, ≤period/3
   recent, ≤2·period/3 old, else stale). Crucial for exhaustion
   detection: "low is fresh but getting older" is a reversal cue that
   price alone can't distinguish from continued weakness.

5. **No MAXINDEX snapshot.** Mirror of MININDEX for the window
   maximum: `max_index_bars_ago ∈ [0, period−1]`. Header gives
   **max_index_label** (FRESH_HIGH / RECENT_HIGH / OLD_HIGH /
   STALE_HIGH / INSUFFICIENT_DATA) with the same band structure.

## Decision

Add five optional per-symbol snapshot blocks to the Godel-parity
pipeline, each reusing the existing `research_historical_price` HP
cache and the standard research-table LAN-sync path (no new API
dependencies):

1. `research::MinSnapshot` + `compute_min_snapshot` + `upsert_min` +
   `get_min` — serialised to `research_min`.
2. `research::MaxSnapshot` + `compute_max_snapshot` + `upsert_max` +
   `get_max` — serialised to `research_max`.
3. `research::MinMaxSnapshot` + `compute_minmax_snapshot` +
   `upsert_minmax` + `get_minmax` — serialised to `research_minmax`.
4. `research::MinIndexSnapshot` + `compute_minindex_snapshot` +
   `upsert_minindex` + `get_minindex` — serialised to
   `research_minindex`.
5. `research::MaxIndexSnapshot` + `compute_maxindex_snapshot` +
   `upsert_maxindex` + `get_maxindex` — serialised to
   `research_maxindex`.

All five compute functions share a private helper
`window_extrema(sorted, end_idx, period)` that walks the
`[end_idx-period+1..=end_idx]` bar slice once and returns the tuple
`(min_val, min_idx, max_val, max_idx)`. Each compute_* then plucks
the relevant two scalars. This keeps per-compute cost bounded to
O(period) and eliminates the drift risk of five ad-hoc scans.

Two private label helpers further dedupe classification logic:
- `position_label(pct, high_is_positive)` — the 3-band classifier
  used by MIN (`high_is_positive=false`, so NEAR_HIGH → positive
  tail) and MAX (`high_is_positive=true`), returning the
  symmetrically-valued `NEAR_HIGH / MID / NEAR_LOW` labels from the
  same 20%/80% cutoffs.
- `recency_label(bars_ago, period, is_high)` — the 4-band recency
  classifier used by MININDEX (`is_high=false` → *_LOW suffix) and
  MAXINDEX (`is_high=true` → *_HIGH suffix) from shared `period/6`,
  `period/3`, `2·period/3` cutoffs.

Schema version bumps to v71 via `create_research_tables_v71` which
wraps v70 and adds five new CREATE TABLE stanzas (plus indexes on
`updated_at`). All five tables register in `SYNCABLE_TABLES`,
`create_table_sql`, and `table_timestamp_column` so LAN sync
incrementally propagates them to peer terminals.

Native wiring adds five `BrokerCmd::Compute*Snapshot` variants, five
`BrokerMsg::*SnapshotMsg` variants, five `show_*_win` / `*_symbol` /
`*_snapshot` / `*_loading` field tuples on `App`, five tokio-spawned
broker handlers (load HP cache → compute → upsert → emit msg), five
palette alias blocks, five packet-emitter blocks after the Round 68
CORREL emitter, five egui windows with Use-Chart / Load-Cached /
Compute controls plus a striped Grid summary, and five `BrokerMsg`
match arms.

Palette aliases were selected to avoid collision with earlier rounds
(verified at implementation time against R60..R68 token sets):
`MIN | MINWIN | MIN_CLOSE | LOW_BAND | ROLL_MIN`;
`MAX | MAXWIN | MAX_CLOSE | HIGH_BAND | ROLL_MAX`;
`MINMAX | MINMAXWIN | RANGE_BAND | HL_RANGE | EXTREMA`;
`MININDEX | MINIDXWIN | LOW_IDX | MIN_AGE | LOW_RECENCY`;
`MAXINDEX | MAXIDXWIN | HIGH_IDX | MAX_AGE | HIGH_RECENCY`. All 25
tokens are fresh — zero collisions with earlier rounds. The bare
`MIN` and `MAX` tokens were both available because no prior round
claimed them: MOM (Round 65) / ROC (Round 68) / ADX (Round 31) all
ship range-related concepts but never under bare-extrema aliases.

## Consequences

- Packet scope grows by up to 10 k/v rows per symbol when all five
  snapshots are populated — roughly +180 bytes for MIN (value, prev,
  max_ref, close, position), +180 bytes for MAX (mirror), +220 bytes
  for MINMAX (both values, width, width_pct, close, position),
  +180 bytes for MININDEX (value, bars_ago, bars_ago_prev, close),
  +180 bytes for MAXINDEX (mirror) — for a typical +0.94 KB per
  symbol.
- Schema is strictly additive; old peers running v70 continue to
  work (new tables are absent but none of the old tables change).
  LAN sync skips unknown tables via the whitelist.
- All five indicators share the HP cache with zero additional
  network dependencies. All five require n≥32 bars (period 30 + 2
  for the prev-bar `*_prev` field). The shared `window_extrema`
  helper makes all five compute_* O(period) per call — optimal by
  any metric.
- Like Round 68 + earlier rounds, the 10 Round 69 tests (5
  roundtrip + 5 compute) guard against serialization drift and
  band-cutoff regressions.

## Verification

1. **Engine tests:** `cargo test --package typhoon-engine --lib`
   passes with +10 Round 69 tests over Round 68's count (1425 total
   including Round 69 additions).
2. **Native build:** `cargo build --package typhoon-native` completes
   cleanly with no warnings/errors in 4m 01s.
3. **Unique palette tokens:** All 25 Round 69 palette tokens fresh —
   zero collisions with earlier rounds (verified against the 25 Round
   68 tokens and the cumulative R60..R67 set).
4. **LAN sync whitelist:** All five `research_*` tables registered
   in `SYNCABLE_TABLES`, `create_table_sql`, and
   `table_timestamp_column`; incremental sync uses the `updated_at`
   column.
5. **Sibling-identity coverage:** the minmax_compute test asserts
   that `snap.min_val ≤ snap.last_close ≤ snap.max_val`,
   `snap.range_width == snap.max_val − snap.min_val`, and
   `snap.range_pct == 100 · range_width / last_close` — guarding
   against arithmetic drift in the combined snapshot.

## Packet envelope delta

Before Round 69: packet emitted 146 k/v rows across Round 60..68
additions. After Round 69: 156 k/v rows when all fifty Round 60..69
additions populate, typical +0.94 KB per symbol on top of the +1.13
KB Round 68 added, +1.22 KB Round 67 added, +1.05 KB Round 66 added,
+1.45 KB Round 65 added, +1.45 KB Round 64 added, +1.45 KB Round 63
added, +1.45 KB Round 62 added, +1.40 KB Round 61 added, and +1.46
KB Round 60 added — bringing the observed per-symbol envelope from
~94-178 KB to ~95-179 KB.
