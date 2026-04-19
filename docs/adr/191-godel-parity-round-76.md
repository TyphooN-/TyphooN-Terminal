# ADR-191: TA-Lib Parity Round 76 — CDLDOJISTAR / CDLMORNINGDOJISTAR / CDLEVENINGDOJISTAR / CDLABANDONEDBABY / CDL3INSIDE (research-layer, chart-overlay TBD)

**Status:** Accepted
**Date:** 2026-04-19
**Supersedes/extends:** ADR-108 through ADR-190
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel documented? | TA-Lib primitive? | Research packet | egui popup | Chart overlay |
| --- | --- | --- | --- | --- | --- |
| CDLDOJISTAR | No | Yes (`CDLDOJISTAR`) | Yes | Yes | No (track B) |
| CDLMORNINGDOJISTAR | No | Yes (`CDLMORNINGDOJISTAR`) | Yes | Yes | No (track B) |
| CDLEVENINGDOJISTAR | No | Yes (`CDLEVENINGDOJISTAR`) | Yes | Yes | No (track B) |
| CDLABANDONEDBABY | No | Yes (`CDLABANDONEDBABY`) | Yes | Yes | No (track B) |
| CDL3INSIDE | No | Yes (`CDL3INSIDE`) | Yes | Yes | No (track B) |

**Round classification:** pure TA-Lib candlestick-library coverage; not
derived from Godel Terminal's publicly documented feature set. Continues
the "TA-Lib + Godel Parity" program. Track B (chart-overlay surfacing of
existing research-layer CDL\* snapshots) is tracked separately — see
ADR-188.

## Context

Rounds 72, 73, 74, and 75 (ADR-184/185/186/187) have delivered twenty
TA-Lib `CDL*` candlestick primitives covering single-, two-, and
three-bar geometries — all reusing the unchanged `candle_metrics(bar)`
and `cdl_scan<F>(sorted, min_i, detector)` helpers introduced in
Round 72. Round 76 closes out the **star family + inside-bar
confirmation remnants** — the five patterns that most complete the
star pattern catalogue and the final confirmed-reversal variant:

- **CDLDOJISTAR** is the 2-bar precursor to the full morning/evening
  doji-star patterns. Prior bar has a real body (≥ 30% of range),
  current bar is a doji (≤ 5% of range) whose body gaps away from
  the prior close. Sign encodes direction: `-100` when prior is
  green and doji gaps up (bearish top); `+100` when prior is red
  and doji gaps down (bullish bottom). Pairs conceptually with the
  full 3-bar morning/evening doji-star variants below.
- **CDLMORNINGDOJISTAR** is the doji-middle variant of R73's
  MORNINGSTAR. Same 3-bar bullish reversal structure (long red →
  small middle → strong green close past midpoint of bar 1) but
  with the middle bar constrained to a doji (≤ 5% body vs.
  R73's ≤ 30% body) and gapping below bar 1's close. The doji
  expresses explicit equilibrium after the sell-off before the
  recovery — higher conviction than regular morning star.
- **CDLEVENINGDOJISTAR** is the bearish mirror of the above.
  Long green → doji gapping above → strong red close past the
  midpoint. Doji-middle variant of R73's EVENINGSTAR.
- **CDLABANDONEDBABY** is the strongest star variant in the TA-Lib
  catalogue: the doji is "abandoned" by full-shadow gaps on BOTH
  sides, i.e., no wick overlap between consecutive bars. Bullish:
  bar 1 long red, bar 2 doji with `bar2.high < bar1.low`, bar 3
  green with `bar3.low > bar2.high`. Bearish: mirror. Very rare
  but very high-conviction reversal signal.
- **CDL3INSIDE** is the confirmed-harami 3-bar reversal. Bars 1
  and 2 form a Harami (bar 2 fully contained within bar 1's body,
  opposite colour); bar 3 closes beyond bar 1's body in the
  direction opposite to bar 1 — i.e., the confirmation that
  completes the reversal. Bullish: bar 1 red long + small green
  Harami inside + bar 3 closes above bar 1's open. Bearish: mirror.
  Completes the Harami family (neutral R72 Harami, doji-variant
  R75 Harami Cross, confirmed R76 3INSIDE).

1. **No CDLDOJISTAR snapshot.** Prior bar with real body
   (≥ 30% of range), current bar doji (≤ 5% of range) whose entire
   body (open AND close) is on one side of the prior close, away
   from the prior body. TA-Lib emits `+100` or `-100`. Scalar
   triple `prior_body_pct_range`, `current_body_pct_range`, and
   signed `gap_pct` captures the geometry across direction.

2. **No CDLMORNINGDOJISTAR snapshot.** Three bars: long red (≥ 30%
   body) → doji (≤ 5% body) gapping below bar 1 close → long green
   closing above bar 1 midpoint. TA-Lib emits `+100`. Scalar triple
   `bar1_body_pct_range`, `bar2_body_pct_range` (doji),
   `bar3_close_vs_bar1_mid_pct` (signed % above midpoint, positive
   on match).

3. **No CDLEVENINGDOJISTAR snapshot.** Mirror of MORNINGDOJISTAR.
   Scalar triple identical but `bar3_close_vs_bar1_mid_pct` is
   negative on match.

4. **No CDLABANDONEDBABY snapshot.** Full-shadow gaps on both
   sides. Scalar quad `bar1_body_pct_range`, `bar2_body_pct_range`
   (doji), signed `gap_down_pct` (prior-to-middle), signed
   `gap_up_pct` (middle-to-last). TA-Lib emits `+100` or `-100`.

5. **No CDL3INSIDE snapshot.** Harami geometry in bars 1-2 plus
   confirmation close in bar 3. Scalar triple `bar1_body_pct_range`,
   `body_size_ratio` (bar 2 body / bar 1 body, always `< 1.0` when
   match — same convention as R72 Harami and R75 Harami Cross),
   and `bar3_close_vs_bar1_open_pct` (signed % distance from bar 1's
   open). TA-Lib emits `+100` (bullish) or `-100` (bearish).

## Decision

Add five optional per-symbol snapshot blocks to the Godel-parity
pipeline, each reusing the existing `research_historical_price` HP
cache and the standard research-table LAN-sync path (no new API
dependencies):

1. `research::CdlDojiStarSnapshot` + `compute_cdl_doji_star_snapshot`
   + `upsert_cdl_doji_star` + `get_cdl_doji_star` — serialised to
   `research_cdl_doji_star`.
2. `research::CdlMorningDojiStarSnapshot` +
   `compute_cdl_morning_doji_star_snapshot` +
   `upsert_cdl_morning_doji_star` + `get_cdl_morning_doji_star` —
   serialised to `research_cdl_morning_doji_star`.
3. `research::CdlEveningDojiStarSnapshot` +
   `compute_cdl_evening_doji_star_snapshot` +
   `upsert_cdl_evening_doji_star` + `get_cdl_evening_doji_star` —
   serialised to `research_cdl_evening_doji_star`.
4. `research::CdlAbandonedBabySnapshot` +
   `compute_cdl_abandoned_baby_snapshot` +
   `upsert_cdl_abandoned_baby` + `get_cdl_abandoned_baby` —
   serialised to `research_cdl_abandoned_baby`.
5. `research::CdlThreeInsideSnapshot` +
   `compute_cdl_three_inside_snapshot` +
   `upsert_cdl_three_inside` + `get_cdl_three_inside` —
   serialised to `research_cdl_three_inside`.

**No new helpers.** Every R76 compute function follows the exact
same skeleton established in Round 72 and re-validated in Rounds
73/74/75: sort bars → bail if `n < min_bars` → define closure
detector → call `cdl_scan(sorted, min_i, detector)` → fill metrics
+ label → return snapshot. The 2-bar Doji Star detector passes
`min_i = 1`; the four 3-bar detectors (Morning/Evening Doji Star,
Abandoned Baby, 3 Inside) all pass `min_i = 2`. Fifth consecutive
round where `cdl_scan` + `candle_metrics` have been extended
without modification across 1-, 2-, and 3-bar patterns.

Schema version bumps to v80 via `create_research_tables_v80` which
wraps v79 and adds five new CREATE TABLE stanzas (plus indexes on
`updated_at`). All five tables register in `SYNCABLE_TABLES`,
`create_table_sql`, and `table_timestamp_column` so LAN sync
incrementally propagates them to peer terminals.

Native wiring adds five `BrokerCmd::ComputeCdl*Snapshot` variants,
five `BrokerMsg::Cdl*SnapshotMsg` variants, twenty `App` fields
(show/symbol/snapshot/loading × 5), twenty defaults, five tokio-
spawned broker handlers (load HP cache → compute → upsert → emit
msg), five palette alias blocks, five packet-emitter blocks after
the Round 75 CDLTRISTAR emitter, five egui windows with Use-Chart /
Load-Cached / Compute controls plus a striped Grid summary, and
five `BrokerMsg` match arms.

Palette tokens (verified fresh — all 25 R76 tokens grep-clean
against R60..R75 and the cumulative earlier-round set, with zero
collisions including no overlap with existing `CANDLE`, `HARAMI`
(R72), `DOJI` (R72/R74/R75), `HAMMER` (R72/R74), `STAR` (R73), or
any earlier tokens):
`CDLDOJISTAR | DOJISTAR | DOJI_STAR | CDLDOJISTARWIN |
DOJISTAR_PATTERN`;
`CDLMORNINGDOJISTAR | MORNINGDOJISTAR | MORNING_DOJI_STAR |
CDLMORNINGDOJISTARWIN | MORNING_DOJI_STAR_PATTERN`;
`CDLEVENINGDOJISTAR | EVENINGDOJISTAR | EVENING_DOJI_STAR |
CDLEVENINGDOJISTARWIN | EVENING_DOJI_STAR_PATTERN`;
`CDLABANDONEDBABY | ABANDONEDBABY | ABANDONED_BABY |
CDLABANDONEDBABYWIN | ABANDONED_BABY_PATTERN`;
`CDL3INSIDE | THREEINSIDE | THREE_INSIDE | CDL3INSIDEWIN |
THREE_INSIDE_PATTERN`.
All 25 R76 tokens are fresh.

## Consequences

- Packet scope grows by up to 10 k/v rows per symbol when all five
  snapshots are populated — roughly +220 bytes per CDL snapshot
  (pattern_value, prev, body/shadow metrics or gap/ratio fields,
  last_match, days_since_pattern, close), for a typical +1.10 KB
  per symbol.
- Schema is strictly additive; old peers running v79 continue to
  work (new tables are absent but none of the old tables change).
  LAN sync skips unknown tables via the whitelist.
- All five indicators share the HP cache with zero additional
  network dependencies. 2-bar DOJISTAR needs n≥3; the four 3-bar
  patterns (MORNINGDOJISTAR/EVENINGDOJISTAR/ABANDONEDBABY/3INSIDE)
  need n≥4.
- The 10 Round 76 tests (5 roundtrip + 5 compute) guard against
  serialization drift and detector regressions. Each compute test
  builds a synthetic bar sequence with the exact pattern geometry
  and asserts `pattern_value` matches the expected `+100` / `-100`
  sign convention.
- **Fifth-round validation of the R72 helper design.** Rounds 72,
  73, 74, 75, and 76 have together added twenty-five CDL primitives
  spanning 1-, 2-, and 3-bar geometries and all the major pattern
  families: neutral doji, shadow-dominant reversal, penetration-
  based reversal, inside-bar reversal, engulfing reversal,
  continuation, pure-body conviction (marubozu), triple-doji
  reversal (tristar), star family precursor/doji-middle/abandoned-
  baby variants, and confirmed harami (3INSIDE). The `cdl_scan` +
  `candle_metrics` helpers have remained unchanged across all five
  rounds. A future round adding 5-bar patterns would reuse them
  verbatim.
- Pattern-value convention (`+100` / `-100` / `0`) continues
  matching TA-Lib's canonical output so downstream code that
  cross-references TA-Lib reference implementations gets directly
  comparable scalars without translation.
- The `body_size_ratio` scalar now spans three patterns (R72 Harami,
  R75 Harami Cross, R76 3INSIDE) — always `< 1.0` for inside-bar
  patterns. Agents can cross-reference tightness and confirmation
  strength across the full inside-bar family via one scalar.
- The gap-based scalars (`gap_pct` on DOJISTAR, `gap_down_pct` /
  `gap_up_pct` on ABANDONEDBABY, `bar3_close_vs_bar1_mid_pct` on
  MORNINGDOJISTAR / EVENINGDOJISTAR) are all signed, so agents can
  compute directional-magnitude analytics spanning the star family
  via one consistent sign convention.

## Verification

1. **Engine tests:** `cargo test --package typhoon-engine --lib`
   passes with +10 Round 76 tests over Round 75's count.
2. **Native build:** `cargo build --package typhoon-native` completes
   cleanly with no warnings/errors.
3. **Unique palette tokens:** All 25 Round 76 palette tokens fresh —
   zero collisions with earlier rounds (verified against R60..R75).
4. **LAN sync whitelist:** All five `research_*` tables registered
   in `SYNCABLE_TABLES`, `create_table_sql`, and
   `table_timestamp_column`; incremental sync uses the `updated_at`
   column.
5. **Pattern-value convention coverage:**
   `cdl_doji_star_compute_detects` asserts `pattern_value == 100`
   on a [big_red, doji-gapped-down] pair with `current_body_pct_range
   <= 5.0` and negative `gap_pct`.
   `cdl_morning_doji_star_compute_detects` asserts `pattern_value ==
   100` on a synthetic [long-red, gap-down-doji, strong-green]
   triplet with `bar2_body_pct_range <= 5.0` and positive
   `bar3_close_vs_bar1_mid_pct`.
   `cdl_evening_doji_star_compute_detects` asserts `pattern_value ==
   -100` on the bearish mirror triplet.
   `cdl_abandoned_baby_compute_detects` asserts `pattern_value ==
   100` on a synthetic triplet with full-shadow gaps on both sides.
   `cdl_three_inside_compute_detects` asserts `pattern_value == 100`
   on a synthetic [big-red, small-green-inside, close-above-bar1-
   open] triplet with `body_size_ratio < 1.0` and positive
   `bar3_close_vs_bar1_open_pct`.

## Packet envelope delta

Before Round 76: packet emitted 216 k/v rows across Round 60..75
additions. After Round 76: 226 k/v rows when all eighty-five Round
60..76 additions populate, typical +1.10 KB per symbol on top of
the +1.10 KB Round 75 added, +1.10 KB Round 74 added, +1.10 KB
Round 73 added, +1.10 KB Round 72 added, +1.10 KB Round 71 added,
+1.04 KB Round 70 added, +0.94 KB Round 69 added, +1.13 KB Round 68
added, +1.22 KB Round 67 added, +1.05 KB Round 66 added, +1.45 KB
Round 65 added, +1.45 KB Round 64 added, +1.45 KB Round 63 added,
+1.45 KB Round 62 added, +1.40 KB Round 61 added, and +1.46 KB
Round 60 added — bringing the observed per-symbol envelope from
~101-185 KB to ~102-186 KB.
