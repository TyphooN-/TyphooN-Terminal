# ADR-187: TA-Lib Parity Round 75 — CDLHARAMICROSS / CDLLONGLEGGEDDOJI / CDLMARUBOZU / CDLSPINNINGTOP / CDLTRISTAR (research-layer, chart overlay deferred)

**Status:** Implemented (research layer); chart overlays governed by ADR-188
**Date:** 2026-04-19
**Supersedes/extends:** ADR-108 through ADR-186
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel documented? | TA-Lib primitive? | Research packet | egui popup | Chart overlay |
| --- | --- | --- | --- | --- | --- |
| CDLHARAMICROSS | No | Yes (`CDLHARAMICROSS`) | Yes | Yes | No (ADR-188 scope rule) |
| CDLLONGLEGGEDDOJI | No | Yes (`CDLLONGLEGGEDDOJI`) | Yes | Yes | No (ADR-188 scope rule) |
| CDLMARUBOZU | No | Yes (`CDLMARUBOZU`) | Yes | Yes | No (ADR-188 scope rule) |
| CDLSPINNINGTOP | No | Yes (`CDLSPINNINGTOP`) | Yes | Yes | No (ADR-188 scope rule) |
| CDLTRISTAR | No | Yes (`CDLTRISTAR`) | Yes | Yes | No (ADR-188 scope rule) |

**Round classification:** pure TA-Lib candlestick-library coverage; not
derived from Godel Terminal's publicly documented feature set. The
program is reframed from "Godel Parity" to **"TA-Lib + Godel Parity"**
going forward, with per-feature classification tables in every round
ADR (ADR-108..ADR-186 backfilled by the audit ADR). Chart-overlay
surfacing of existing research-layer CDL\* snapshots is deferred
separately — see ADR-188.

## Context

Rounds 72, 73, and 74 (ADR-184/185/186) established and then
twice re-validated the TA-Lib `CDL*` candlestick pattern
pipeline — fifteen primitives covering single-, two-, and
three-bar geometries, all reusing the unchanged
`candle_metrics(bar)` and `cdl_scan<F>(sorted, min_i, detector)`
helpers introduced in R72. Round 75 closes out the
**single-bar + inside-bar pattern remnants** — the five patterns
that most complete the single-bar doji/shadow/body catalogue
plus the last inside-bar variant (harami cross) and the rare
three-doji reversal (tristar):

- **CDLHARAMICROSS** is the stricter variant of R72's CDLHARAMI:
  same inside-bar containment, but the inside bar must itself be
  a doji (body ≤ 5% of range) rather than any small opposite-
  direction body. Completes the harami family pair.
- **CDLLONGLEGGEDDOJI** is the wide-range doji variant of R72's
  neutral CDLDOJI: adds a specific geometric constraint that
  both shadows are ≥ 30% of range, distinguishing it from the
  standard doji and from R74's dragonfly/gravestone dojis (which
  have one-sided shadow dominance). Completes the doji subfamily
  catalogue (neutral → dragonfly → gravestone → long-legged).
- **CDLMARUBOZU** is the opposite end of the body-size spectrum
  from doji: body ≥ 90% of range, shadows ≤ 5% each. The
  strongest single-bar directional conviction signal. Pairs
  conceptually with the doji family as polar extremes.
- **CDLSPINNINGTOP** is the intermediate body-size indecision
  signal: body ≤ 30% of range (same threshold as R72 Hammer),
  but BOTH shadows must exceed the body (vs. hammer's one-sided
  shadow dominance). Completes the body/shadow-size-vs-indecision
  continuum (marubozu > regular bars > spinning top > long-legged
  doji > regular doji).
- **CDLTRISTAR** is the 3-bar rare triple-doji reversal: the
  last explicit doji-based multi-bar pattern left ungeneralized
  in the CDL family. High-conviction but rare pattern that
  completes the multi-bar reversal catalogue.

1. **No CDLHARAMICROSS snapshot.** Prior bar has large body
   (≥ 30% of range), current bar's body is contained entirely
   within prior body AND current body is ≤ 5% of range (doji).
   TA-Lib emits `+100` (bullish when prior red) or `-100` (bearish
   when prior green). Reuses the `body_size_ratio` and
   `prior_body_pct_range` / `current_body_pct_range` scalar naming
   from R72 Harami for cross-pattern consistency — but
   `body_size_ratio` here will typically be much smaller than in
   regular Harami since the doji constraint forces the inside
   body to be tiny.

2. **No CDLLONGLEGGEDDOJI snapshot.** Body ≤ 5% of range (doji),
   upper shadow ≥ 30% of range, lower shadow ≥ 30% of range. The
   joint-shadow-dominance constraint distinguishes it from R72
   neutral doji (no shadow constraint) and from R74 dragonfly
   (lower only) / gravestone (upper only). TA-Lib emits `+100`
   on match (treated as neutral indecision; context determines
   implication). Scalar triple body/upper/lower_shadow_pct shares
   naming with the rest of the doji family for cross-pattern
   shadow-dominance analytics.

3. **No CDLMARUBOZU snapshot.** Body ≥ 90% of range, each shadow
   ≤ 5% of range. TA-Lib emits `+100` (bullish green marubozu =
   open ≈ low, close ≈ high) or `-100` (bearish red marubozu =
   open ≈ high, close ≈ low). Strongest conviction single-bar
   signal; polar opposite of the doji family. Shares body/shadow
   scalar naming with the hammer/star/spinning-top family for
   one-stop shadow-dominance cross-analysis.

4. **No CDLSPINNINGTOP snapshot.** Body ≤ 30% of range, both
   shadows must exceed body size. Distinguishable from:
   - R72 Hammer / R74 Hanging Man (one-sided lower shadow ≥ 2×
     body, upper ≤ body),
   - R72 Shooting Star / R74 Inverted Hammer (one-sided upper
     shadow ≥ 2× body, lower ≤ body),
   - Long-legged doji (body ≤ 5% vs. spinning top's ≤ 30%).
   TA-Lib emits `+100` (green body) or `-100` (red body). Sign
   encodes body colour, not directional implication — both treated
   as indecision signals.

5. **No CDLTRISTAR snapshot.** Three consecutive doji bars, each
   with body ≤ 5% of range. Bullish tristar = middle doji gaps
   below outer two AND the third closes above the middle (star at
   bottom); bearish tristar = middle doji gaps above AND third
   closes below. TA-Lib emits `+100` or `-100`. `avg_body_pct_range`
   captures the average doji tightness across the triplet;
   `middle_gap_pct` captures the signed magnitude of the middle
   doji's positional gap from the outer two (negative = below
   = bullish, positive = above = bearish).

## Decision

Add five optional per-symbol snapshot blocks to the Godel-parity
pipeline, each reusing the existing `research_historical_price` HP
cache and the standard research-table LAN-sync path (no new API
dependencies):

1. `research::CdlHaramiCrossSnapshot` +
   `compute_cdl_harami_cross_snapshot` + `upsert_cdl_harami_cross` +
   `get_cdl_harami_cross` — serialised to `research_cdl_harami_cross`.
2. `research::CdlLongLeggedDojiSnapshot` +
   `compute_cdl_long_legged_doji_snapshot` +
   `upsert_cdl_long_legged_doji` + `get_cdl_long_legged_doji` —
   serialised to `research_cdl_long_legged_doji`.
3. `research::CdlMarubozuSnapshot` +
   `compute_cdl_marubozu_snapshot` + `upsert_cdl_marubozu` +
   `get_cdl_marubozu` — serialised to `research_cdl_marubozu`.
4. `research::CdlSpinningTopSnapshot` +
   `compute_cdl_spinning_top_snapshot` + `upsert_cdl_spinning_top` +
   `get_cdl_spinning_top` — serialised to `research_cdl_spinning_top`.
5. `research::CdlTristarSnapshot` +
   `compute_cdl_tristar_snapshot` + `upsert_cdl_tristar` +
   `get_cdl_tristar` — serialised to `research_cdl_tristar`.

**No new helpers.** Every R75 compute function follows the exact
same skeleton established in Round 72 and re-validated in Rounds
73/74: sort bars → bail if `n < min_bars` → define closure detector
→ call `cdl_scan(sorted, min_i, detector)` → fill metrics + label
→ return snapshot. The four single-bar detectors (Long-Legged
Doji / Marubozu / Spinning Top) pass `min_i = 0`; the 2-bar
Harami Cross detector passes `min_i = 1`; the 3-bar Tristar
detector passes `min_i = 2`. Fourth consecutive round where
`cdl_scan` + `candle_metrics` have been extended without
modification across 1-, 2-, and 3-bar patterns.

Schema version bumps to v77 via `create_research_tables_v77` which
wraps v76 and adds five new CREATE TABLE stanzas (plus indexes on
`updated_at`). All five tables register in `SYNCABLE_TABLES`,
`create_table_sql`, and `table_timestamp_column` so LAN sync
incrementally propagates them to peer terminals.

Native wiring adds five `BrokerCmd::ComputeCdl*Snapshot` variants,
five `BrokerMsg::Cdl*SnapshotMsg` variants, twenty `App` fields
(show/symbol/snapshot/loading × 5), twenty defaults, five tokio-
spawned broker handlers (load HP cache → compute → upsert → emit
msg), five palette alias blocks, five packet-emitter blocks after
the Round 74 CDLINVERTEDHAMMER emitter, five egui windows with
Use-Chart / Load-Cached / Compute controls plus a striped Grid
summary, and five `BrokerMsg` match arms.

Palette tokens (verified fresh — all 25 R75 tokens grep-clean
against R60..R74 and the cumulative earlier-round set, with zero
collisions including no overlap with existing `CANDLE`, `HARAMI`
(R72), `DOJI` (R72), `HAMMER` (R72), or any earlier tokens):
`CDLHARAMICROSS | HARAMICROSS | HARAMI_CROSS | CDLHARAMICROSSWIN |
HARAMI_CROSS_PATTERN`;
`CDLLONGLEGGEDDOJI | LONGLEGGEDDOJI | LONG_LEGGED_DOJI | LONGLEGGED |
CDLLONGLEGGEDDOJIWIN`;
`CDLMARUBOZU | MARUBOZU | MARUBOZU_CANDLE | MARUBOZU_PATTERN |
CDLMARUBOZUWIN`;
`CDLSPINNINGTOP | SPINNINGTOP | SPINNING_TOP | SPINNING_TOP_PATTERN |
CDLSPINNINGTOPWIN`;
`CDLTRISTAR | TRISTAR | TRI_STAR | TRIPLE_DOJI | CDLTRISTARWIN`.
All 25 R75 tokens are fresh.

## Consequences

- Packet scope grows by up to 10 k/v rows per symbol when all five
  snapshots are populated — roughly +220 bytes per CDL snapshot
  (pattern_value, prev, body/shadow metrics or tristar avg_body +
  mid_gap, last_match, days_since_pattern, close), for a typical
  +1.10 KB per symbol.
- Schema is strictly additive; old peers running v76 continue to
  work (new tables are absent but none of the old tables change).
  LAN sync skips unknown tables via the whitelist.
- All five indicators share the HP cache with zero additional
  network dependencies. Three single-bar patterns
  (LONG_LEGGED_DOJI/MARUBOZU/SPINNING_TOP) need n≥2 bars; 2-bar
  HARAMICROSS needs n≥3; 3-bar TRISTAR needs n≥4.
- The 10 Round 75 tests (5 roundtrip + 5 compute) guard against
  serialization drift and detector regressions. Each compute test
  builds a synthetic bar sequence with the exact pattern geometry
  (e.g., Tristar = explicit [doji, gap-down-doji, back-up-doji]
  triplet with deterministic highs/lows) and asserts
  `pattern_value` matches the expected `+100` / `-100` sign
  convention.
- **Fourth-round validation of the R72 helper design.** Rounds 72,
  73, 74, and 75 have together added twenty CDL primitives
  spanning 1-, 2-, and 3-bar geometries and all the major pattern
  families: neutral doji, shadow-dominant reversal, penetration-
  based reversal, inside-bar reversal, engulfing reversal,
  continuation, pure-body conviction (marubozu), and triple-doji
  reversal (tristar). The `cdl_scan` + `candle_metrics` helpers
  have remained unchanged across all four rounds. A future round
  adding 5-bar patterns would reuse them verbatim.
- Pattern-value convention (`+100` / `-100` / `0`) continues
  matching TA-Lib's canonical output so downstream code that
  cross-references TA-Lib reference implementations gets directly
  comparable scalars without translation. R75 Spinning Top uses
  TA-Lib's body-colour sign convention (not directional
  implication), which is documented in the snapshot label values
  (GREEN_BODY_PATTERN / RED_BODY_PATTERN) so agents can
  distinguish directional signals from indecision-with-colour-
  encoding signals at a glance.
- The body/upper_shadow/lower_shadow_pct scalar triple is now
  shared across ten patterns: Doji, Hammer, Shooting Star,
  Dragonfly Doji, Gravestone Doji, Hanging Man, Inverted Hammer,
  Long-Legged Doji, Marubozu, Spinning Top. Agents can now
  compute cross-family shadow-dominance-vs-body-size analytics
  spanning the entire single-bar CDL catalogue via one normalised
  scalar triple.
- The `body_size_ratio` scalar continues the R72 Harami /
  Engulfing convention: always `< 1.0` for inside-bar patterns
  (Harami / Harami Cross), always `> 1.0` for outside-bar
  patterns (Engulfing). Harami Cross's ratio will typically be
  < 0.05 since the doji constraint forces the inside body to be
  tiny, distinguishing it from regular Harami (which can have
  inside bodies up to ~30% of prior body).

## Verification

1. **Engine tests:** `cargo test --package typhoon-engine --lib`
   passes with +10 Round 75 tests over Round 74's count (1485 total
   including Round 75 additions).
2. **Native build:** `cargo build --package typhoon-native` completes
   cleanly with no warnings/errors.
3. **Unique palette tokens:** All 25 Round 75 palette tokens fresh —
   zero collisions with earlier rounds (verified against R60..R74).
4. **LAN sync whitelist:** All five `research_*` tables registered
   in `SYNCABLE_TABLES`, `create_table_sql`, and
   `table_timestamp_column`; incremental sync uses the `updated_at`
   column.
5. **Pattern-value convention coverage:**
   `cdl_harami_cross_compute_detects` asserts `pattern_value == 100`
   on a [big_red, doji-contained-in-prior-body] pair with
   `current_body_pct_range <= 5.0` and `body_size_ratio < 1.0`.
   `cdl_long_legged_doji_compute_detects` asserts `pattern_value ==
   100` on a synthetic wide-range doji with `body_pct_range <= 5.0`
   AND `upper_shadow_pct >= 30.0` AND `lower_shadow_pct >= 30.0`.
   `cdl_marubozu_compute_detects` asserts `pattern_value == 100`
   on a synthetic bullish marubozu with `body_pct_range >= 90.0`
   AND both shadows ≤ 5%.
   `cdl_spinning_top_compute_detects` asserts `pattern_value == 100`
   on a synthetic green-body spinning top with `body_pct_range <=
   30.0` AND both shadows > body.
   `cdl_tristar_compute_detects` asserts `pattern_value == 100` on
   a synthetic [doji, gap-down-doji, back-up-doji] triplet with
   `avg_body_pct_range <= 5.0`.

## Packet envelope delta

Before Round 75: packet emitted 206 k/v rows across Round 60..74
additions. After Round 75: 216 k/v rows when all eighty Round
60..75 additions populate, typical +1.10 KB per symbol on top of
the +1.10 KB Round 74 added, +1.10 KB Round 73 added, +1.10 KB
Round 72 added, +1.10 KB Round 71 added, +1.04 KB Round 70 added,
+0.94 KB Round 69 added, +1.13 KB Round 68 added, +1.22 KB Round
67 added, +1.05 KB Round 66 added, +1.45 KB Round 65 added,
+1.45 KB Round 64 added, +1.45 KB Round 63 added, +1.45 KB Round
62 added, +1.40 KB Round 61 added, and +1.46 KB Round 60 added —
bringing the observed per-symbol envelope from ~100-184 KB to
~101-185 KB.
