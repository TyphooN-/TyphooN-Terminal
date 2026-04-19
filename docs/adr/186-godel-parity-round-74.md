# ADR-186: Godel Parity Round 74 — CDLPIERCING / CDLDRAGONFLYDOJI / CDLGRAVESTONEDOJI / CDLHANGINGMAN / CDLINVERTEDHAMMER

**Status:** Accepted
**Date:** 2026-04-19
**Supersedes/extends:** ADR-108 through ADR-185
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Rounds 72 and 73 (ADR-184/ADR-185) opened and extended the TA-Lib
`CDL*` candlestick pattern family with ten primitives covering
single-, two-, and three-bar geometries. Round 73 explicitly
validated the R72 helper design (`candle_metrics(bar)` and
`cdl_scan<F>(sorted, min_i, detector)`) by adding five new
patterns of different bar-counts with zero helper modifications.
Round 74 tightens the coverage by closing out the *complement
families* of patterns we already ship:

- **CDLPIERCING** is the bullish mirror of R73's CDLDARKCLOUDCOVER
  (two-bar reversal at a bottom instead of a top).
- **CDLDRAGONFLYDOJI** / **CDLGRAVESTONEDOJI** are the specialised
  T-shape variants of R72's neutral CDLDOJI — directionally typed
  dojis with long lower/upper shadows respectively.
- **CDLHANGINGMAN** / **CDLINVERTEDHAMMER** are the geometric
  mirrors of R72's CDLHAMMER / CDLSHOOTINGSTAR respectively, with
  TA-Lib's sign-flipped pattern_value convention that encodes
  market-context rather than raw geometry (hanging man is a
  bearish hammer-shape at tops; inverted hammer is a bullish
  shooting-star-shape at bottoms).

1. **No CDLPIERCING snapshot.** Piercing Line is the canonical 2-bar
   bullish reversal: prior red (≥30% body), current green opens
   below prior low, closes above prior midpoint but below prior
   open (≥50% penetration). TA-Lib emits `+100` on match. Uses
   `penetration_pct = 100 × (current_close - prior_close) /
   prior_body`, naming chosen to match the R73 DarkCloudCover /
   MorningStar / EveningStar `penetration_pct` semantics — agents
   can compare "how decisively confirmed" across all four
   penetration-based reversal patterns using one normalised scalar.

2. **No CDLDRAGONFLYDOJI snapshot.** T-shape doji signalling
   rejection of lower prices at a level of support: body ≤ 5% of
   range, upper shadow ≤ 5%, lower shadow ≥ 60%. TA-Lib emits
   `+100` on match (treated as bullish unlike the neutral regular
   doji). Scalars share the body/upper/lower shadow naming with
   R72's hammer/shooting-star for cross-pattern comparison.

3. **No CDLGRAVESTONEDOJI snapshot.** Inverted-T doji signalling
   rejection of higher prices at a level of resistance: body ≤ 5%,
   lower shadow ≤ 5%, upper shadow ≥ 60%. TA-Lib emits `-100` on
   match. Mirror of dragonfly — sign-flipped pattern_value, swapped
   upper/lower shadow dominance, same scalar surface.

4. **No CDLHANGINGMAN snapshot.** Geometrically identical to R72's
   Hammer (small upper-third body, long lower shadow ≥ 2× body,
   minimal upper shadow) but appearing at market tops. TA-Lib emits
   `-100` on match — the sign flip is contextual (top vs. bottom),
   not geometric. Agents reading this alongside R72's Hammer will
   see the same geometry numbers but opposite `pattern_value`
   sign, which is the informative signal TA-Lib encodes.

5. **No CDLINVERTEDHAMMER snapshot.** Geometric mirror of R72's
   Shooting Star (small lower-third body, long upper shadow ≥ 2×
   body, minimal lower shadow) but appearing at bottoms. TA-Lib
   emits `+100` on match — again a contextual sign flip. The
   Hanging Man ↔ Hammer and Inverted Hammer ↔ Shooting Star pairs
   together make the canonical four-quadrant single-bar
   shadow-dominant reversal catalogue complete.

## Decision

Add five optional per-symbol snapshot blocks to the Godel-parity
pipeline, each reusing the existing `research_historical_price` HP
cache and the standard research-table LAN-sync path (no new API
dependencies):

1. `research::CdlPiercingSnapshot` +
   `compute_cdl_piercing_snapshot` + `upsert_cdl_piercing` +
   `get_cdl_piercing` — serialised to `research_cdl_piercing`.
2. `research::CdlDragonflyDojiSnapshot` +
   `compute_cdl_dragonfly_doji_snapshot` +
   `upsert_cdl_dragonfly_doji` + `get_cdl_dragonfly_doji` —
   serialised to `research_cdl_dragonfly_doji`.
3. `research::CdlGravestoneDojiSnapshot` +
   `compute_cdl_gravestone_doji_snapshot` +
   `upsert_cdl_gravestone_doji` + `get_cdl_gravestone_doji` —
   serialised to `research_cdl_gravestone_doji`.
4. `research::CdlHangingManSnapshot` +
   `compute_cdl_hanging_man_snapshot` + `upsert_cdl_hanging_man` +
   `get_cdl_hanging_man` — serialised to `research_cdl_hanging_man`.
5. `research::CdlInvertedHammerSnapshot` +
   `compute_cdl_inverted_hammer_snapshot` +
   `upsert_cdl_inverted_hammer` + `get_cdl_inverted_hammer` —
   serialised to `research_cdl_inverted_hammer`.

**No new helpers.** Every R74 compute function follows the exact
same skeleton established by Round 72 and already re-validated by
Round 73: sort bars → bail if `n < min_bars` → define closure
detector → call `cdl_scan(sorted, min_i, detector)` → fill metrics
+ label → return snapshot. The four single-bar detectors pass
`min_i = 0`; the 2-bar Piercing detector passes `min_i = 1`. This
is the third consecutive round where `cdl_scan` + `candle_metrics`
have been extended without modification, and now covers all three
bar-count classes (1-, 2-, 3-bar) and multiple geometric families
(body/shadow-dominant, penetration-based, continuation).

Schema version bumps to v76 via `create_research_tables_v76` which
wraps v75 and adds five new CREATE TABLE stanzas (plus indexes on
`updated_at`). All five tables register in `SYNCABLE_TABLES`,
`create_table_sql`, and `table_timestamp_column` so LAN sync
incrementally propagates them to peer terminals.

Native wiring adds five `BrokerCmd::ComputeCdl*Snapshot` variants,
five `BrokerMsg::Cdl*SnapshotMsg` variants, twenty `App` fields
(show/symbol/snapshot/loading × 5), twenty defaults, five tokio-
spawned broker handlers (load HP cache → compute → upsert → emit
msg), five palette alias blocks, five packet-emitter blocks after
the Round 73 CDLDARKCLOUDCOVER emitter, five egui windows with
Use-Chart / Load-Cached / Compute controls plus a striped Grid
summary, and five `BrokerMsg` match arms.

Palette tokens (verified fresh — all 25 R74 tokens grep-clean
against R60..R73 and the cumulative earlier-round set, with zero
collisions including no overlap with existing `CANDLE` chart-type
or `BULLISH`/`BEARISH` colour-lookup strings, and no overlap with
R72 `HAMMER` or R73 `MORNINGSTAR`/`EVENINGSTAR` tokens):
`CDLPIERCING | PIERCING | PIERCING_LINE | PIERCINGLINE |
CDLPIERCINGWIN`;
`CDLDRAGONFLYDOJI | DRAGONFLYDOJI | DRAGONFLY_DOJI | DRAGONFLY |
CDLDRAGONFLYDOJIWIN`;
`CDLGRAVESTONEDOJI | GRAVESTONEDOJI | GRAVESTONE_DOJI | GRAVESTONE |
CDLGRAVESTONEDOJIWIN`;
`CDLHANGINGMAN | HANGINGMAN | HANGING_MAN | HANGMAN |
CDLHANGINGMANWIN`;
`CDLINVERTEDHAMMER | INVERTEDHAMMER | INVERTED_HAMMER | INVHAMMER |
CDLINVERTEDHAMMERWIN`. All 25 R74 tokens are fresh.

## Consequences

- Packet scope grows by up to 10 k/v rows per symbol when all five
  snapshots are populated — roughly +220 bytes per CDL snapshot
  (pattern_value, prev, body/shadow or penetration metrics,
  last_match, days_since_pattern, close), for a typical +1.10 KB
  per symbol.
- Schema is strictly additive; old peers running v75 continue to
  work (new tables are absent but none of the old tables change).
  LAN sync skips unknown tables via the whitelist.
- All five indicators share the HP cache with zero additional
  network dependencies. The four single-bar patterns
  (DRAGONFLY/GRAVESTONE/HANGINGMAN/INVERTEDHAMMER) need n≥2 bars;
  2-bar Piercing needs n≥3.
- The 10 Round 74 tests (5 roundtrip + 5 compute) guard against
  serialization drift and detector regressions. Each compute test
  builds a synthetic bar sequence with the exact pattern geometry
  (e.g., Piercing = explicit [big_red, gap_down_green] pair with
  deterministic closes) and asserts `pattern_value` matches the
  expected `+100` / `-100` sign convention.
- **Third-round validation of the R72 helper design.** Rounds 72,
  73, and 74 have now together added fifteen CDL primitives
  spanning 1-, 2-, and 3-bar geometries and multiple pattern
  families (neutral doji, shadow-dominant reversal, penetration-
  based reversal, inside-bar reversal, engulfing reversal,
  continuation). The `cdl_scan` + `candle_metrics` helpers have
  remained unchanged across all three rounds. A future round
  adding 3-bar Tristar or 5-bar patterns would still reuse them
  verbatim — the helpers are demonstrably pattern-bar-count
  agnostic.
- Pattern-value convention (`+100` / `-100` / `0`) continues
  matching TA-Lib's canonical output so downstream code that
  cross-references TA-Lib reference implementations gets directly
  comparable scalars without translation. Importantly, TA-Lib's
  contextual sign convention for Hanging Man (`-100`) and
  Inverted Hammer (`+100`) is preserved as-is — agents reading
  across the four hammer/shooting-star/hanging-man/inverted-hammer
  quadrants get TA-Lib's semantic classification, not raw
  geometry.
- The `penetration_pct` scalar is now shared across four patterns:
  Morning Star, Evening Star, Dark Cloud Cover, and Piercing Line.
  Agents can compare confirmation strength decisively across the
  entire penetration-based reversal subfamily via one normalised
  metric.
- The body/upper_shadow/lower_shadow_pct scalar triple is now
  shared across eight patterns: Doji, Hammer, Shooting Star,
  Dragonfly Doji, Gravestone Doji, Hanging Man, Inverted Hammer,
  plus the shadow components captured in Engulfing/Harami. The
  naming is stable so agents can compute cross-pattern shadow
  dominance ratios with zero per-pattern translation.

## Verification

1. **Engine tests:** `cargo test --package typhoon-engine --lib`
   passes with +10 Round 74 tests over Round 73's count (1475 total
   including Round 74 additions).
2. **Native build:** `cargo build --package typhoon-native` completes
   cleanly with no warnings/errors.
3. **Unique palette tokens:** All 25 Round 74 palette tokens fresh —
   zero collisions with earlier rounds (verified against R60..R73).
4. **LAN sync whitelist:** All five `research_*` tables registered
   in `SYNCABLE_TABLES`, `create_table_sql`, and
   `table_timestamp_column`; incremental sync uses the `updated_at`
   column.
5. **Pattern-value convention coverage:**
   `cdl_piercing_compute_detects` asserts `pattern_value == 100`
   on a synthetic [big_red, gap_down_green_closing_above_midpoint]
   pair with `penetration_pct > 0`.
   `cdl_dragonfly_doji_compute_detects` asserts `pattern_value ==
   100` on a synthetic T-shape bar with `lower_shadow_pct >
   upper_shadow_pct` and `body_pct_range <= 5.0`.
   `cdl_gravestone_doji_compute_detects` asserts `pattern_value ==
   -100` on a synthetic inverted-T bar with `upper_shadow_pct >
   lower_shadow_pct` (sign-flipped from dragonfly).
   `cdl_hanging_man_compute_detects` asserts `pattern_value ==
   -100` on a hammer-shaped bar (TA-Lib's contextual sign flip
   from R72 Hammer's `+100`).
   `cdl_inverted_hammer_compute_detects` asserts `pattern_value ==
   100` on a shooting-star-shaped bar (TA-Lib's contextual sign
   flip from R72 Shooting Star's `-100`).

## Packet envelope delta

Before Round 74: packet emitted 196 k/v rows across Round 60..73
additions. After Round 74: 206 k/v rows when all seventy-five
Round 60..74 additions populate, typical +1.10 KB per symbol on top
of the +1.10 KB Round 73 added, +1.10 KB Round 72 added, +1.10 KB
Round 71 added, +1.04 KB Round 70 added, +0.94 KB Round 69 added,
+1.13 KB Round 68 added, +1.22 KB Round 67 added, +1.05 KB Round 66
added, +1.45 KB Round 65 added, +1.45 KB Round 64 added, +1.45 KB
Round 63 added, +1.45 KB Round 62 added, +1.40 KB Round 61 added,
and +1.46 KB Round 60 added — bringing the observed per-symbol
envelope from ~99-183 KB to ~100-184 KB.
