# ADR-185: TA-Lib Parity Round 73 — CDLMORNINGSTAR / CDLEVENINGSTAR / CDL3BLACKCROWS / CDL3WHITESOLDIERS / CDLDARKCLOUDCOVER

**Status:** Accepted
**Date:** 2026-04-18
**Supersedes/extends:** ADR-108 through ADR-184
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| CDLMORNINGSTAR | No | Yes (`CDLMORNINGSTAR`) | Yes | Yes | No (deferred — ADR-188) |
| CDLEVENINGSTAR | No | Yes (`CDLEVENINGSTAR`) | Yes | Yes | No (deferred — ADR-188) |
| CDL3BLACKCROWS | No | Yes (`CDL3BLACKCROWS`) | Yes | Yes | No (deferred — ADR-188) |
| CDL3WHITESOLDIERS | No | Yes (`CDL3WHITESOLDIERS`) | Yes | Yes | No (deferred — ADR-188) |
| CDLDARKCLOUDCOVER | No | Yes (`CDLDARKCLOUDCOVER`) | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure TA-Lib — extends the `CDL*` family with three-bar patterns (Morning Star, Evening Star, Three Black Crows, Three White Soldiers) plus the two-bar Dark Cloud Cover, all reusing the Round-72 `cdl_scan` helper verbatim.

## Context

Round 72 (ADR-184) opened the TA-Lib `CDL*` candlestick pattern
family with five single- and two-bar primitives (CDLDOJI / CDLHAMMER
/ CDLSHOOTINGSTAR / CDLENGULFING / CDLHARAMI) and introduced two
shared helpers — `candle_metrics(bar)` and `cdl_scan<F>(sorted,
min_i, detector)` — explicitly designed so that future rounds can
add a new pattern by defining one detector closure and plugging it
into `cdl_scan` without any new scaffolding. Round 73 exercises
that design intent for the first time: all five R73 additions are
**three-bar** patterns except Dark Cloud Cover (2-bar), and every
one of them reuses the unchanged R72 `cdl_scan` helper by simply
passing `min_i = 2` (or `min_i = 1` for Dark Cloud Cover).

1. **No CDLMORNINGSTAR snapshot.** Morning Star is the canonical
   3-bar bullish reversal at a bottom: bar 0 = large red body,
   bar 1 = small "star" body (tight range, indecision), bar 2 =
   large green body closing above the midpoint of bar 0's body.
   TA-Lib emits `+100` on match. Captured via a `penetration_pct`
   scalar = `100 × (bar2_close - bar0_midpoint) / bar0_body`,
   which trends positive and grows as the reversal is more
   decisive.

2. **No CDLEVENINGSTAR snapshot.** Mirror of Morning Star for
   bearish reversals at tops: bar 0 large green, bar 1 star,
   bar 2 large red closing below bar-0 midpoint. TA-Lib emits
   `-100` on match. `penetration_pct` sign-flipped from Morning
   Star but using the same absolute-magnitude semantics so agents
   can compare decisiveness directly across the two.

3. **No CDL3BLACKCROWS snapshot.** Three consecutive red bars
   where each opens within the prior body AND closes below the
   prior close — the classic "sustained distribution" bearish
   continuation pattern from Nison's 1991 book. TA-Lib emits
   `-100` on match. Two scalar aggregates help agents gauge
   intensity: `avg_body_pct_range` (average body size across the
   3 bars — larger = more conviction) and `total_close_decline_pct`
   (cumulative return from bar 0 open to bar 2 close, signed
   negative for matches).

4. **No CDL3WHITESOLDIERS snapshot.** Mirror of 3BLACKCROWS for
   bullish continuation: three consecutive green bars, each
   opening in prior body and closing above prior close. TA-Lib
   emits `+100` on match. Uses `avg_body_pct_range` +
   `total_close_advance_pct` (sign-flipped from 3BLACKCROWS'
   decline scalar). Nice parallel: the two scalars for each
   continuation pattern mirror each other by construction.

5. **No CDLDARKCLOUDCOVER snapshot.** 2-bar bearish reversal
   where a prior green (large body) is followed by a red bar
   that *opens above prior high* (gap up) but *closes below
   prior midpoint* (penetrates ≥ 50% into prior body). TA-Lib
   emits `-100` on match with a default penetration threshold
   of 0.5 (configurable). Captured via `penetration_pct` =
   `100 × (prior_close - current_close) / prior_body`, naming
   chosen to match Morning/Evening Star's penetration semantics
   for cross-family scalar consistency.

## Decision

Add five optional per-symbol snapshot blocks to the Godel-parity
pipeline, each reusing the existing `research_historical_price` HP
cache and the standard research-table LAN-sync path (no new API
dependencies):

1. `research::CdlMorningStarSnapshot` +
   `compute_cdl_morning_star_snapshot` + `upsert_cdl_morning_star` +
   `get_cdl_morning_star` — serialised to `research_cdl_morning_star`.
2. `research::CdlEveningStarSnapshot` +
   `compute_cdl_evening_star_snapshot` + `upsert_cdl_evening_star` +
   `get_cdl_evening_star` — serialised to `research_cdl_evening_star`.
3. `research::CdlThreeBlackCrowsSnapshot` +
   `compute_cdl_three_black_crows_snapshot` +
   `upsert_cdl_three_black_crows` + `get_cdl_three_black_crows` —
   serialised to `research_cdl_three_black_crows`.
4. `research::CdlThreeWhiteSoldiersSnapshot` +
   `compute_cdl_three_white_soldiers_snapshot` +
   `upsert_cdl_three_white_soldiers` + `get_cdl_three_white_soldiers`
   — serialised to `research_cdl_three_white_soldiers`.
5. `research::CdlDarkCloudCoverSnapshot` +
   `compute_cdl_dark_cloud_cover_snapshot` +
   `upsert_cdl_dark_cloud_cover` + `get_cdl_dark_cloud_cover` —
   serialised to `research_cdl_dark_cloud_cover`.

**No new helpers.** Every R73 compute function follows the exact
same skeleton established by Round 72: sort bars → bail if `n <
min_bars` → define closure detector that consumes `s[i - k..=i]`
→ call `cdl_scan(sorted, min_i, detector)` → fill metrics + label
→ return snapshot. The 3-bar detectors access `s[i-2]`, `s[i-1]`,
`s[i]` and pass `min_i = 2`; the Dark Cloud Cover 2-bar detector
accesses `s[i-1]`, `s[i]` and passes `min_i = 1`. This validates
the R72 design intent — adding pattern families of any bar-count
is a one-function change.

Schema version bumps to v75 via `create_research_tables_v75` which
wraps v74 and adds five new CREATE TABLE stanzas (plus indexes on
`updated_at`). All five tables register in `SYNCABLE_TABLES`,
`create_table_sql`, and `table_timestamp_column` so LAN sync
incrementally propagates them to peer terminals.

Native wiring adds five `BrokerCmd::ComputeCdl*Snapshot` variants,
five `BrokerMsg::Cdl*SnapshotMsg` variants, twenty `App` fields
(show/symbol/snapshot/loading × 5), twenty defaults, five tokio-
spawned broker handlers (load HP cache → compute → upsert → emit
msg), five palette alias blocks, five packet-emitter blocks after
the Round 72 CDLHARAMI emitter, five egui windows with
Use-Chart / Load-Cached / Compute controls plus a striped Grid
summary, and five `BrokerMsg` match arms.

Palette tokens (verified fresh — all 25 R73 tokens grep-clean
against R60..R72 and the cumulative earlier-round set, with zero
collisions including no overlap with existing `CANDLE` chart-type
or `BULLISH`/`BEARISH` colour-lookup strings):
`CDLMORNINGSTAR | MORNINGSTAR | MORNING_STAR | CDLMORNINGSTARWIN |
MORNING_STAR_PATTERN`;
`CDLEVENINGSTAR | EVENINGSTAR | EVENING_STAR | CDLEVENINGSTARWIN |
EVENING_STAR_PATTERN`;
`CDL3BLACKCROWS | THREEBLACKCROWS | THREE_BLACK_CROWS | BLACK_CROWS |
CDLTHREEBLACKCROWSWIN`;
`CDL3WHITESOLDIERS | THREEWHITESOLDIERS | THREE_WHITE_SOLDIERS |
WHITE_SOLDIERS | CDLTHREEWHITESOLDIERSWIN`;
`CDLDARKCLOUDCOVER | DARKCLOUDCOVER | DARK_CLOUD_COVER | DARK_CLOUD |
CDLDARKCLOUDCOVERWIN`. All 25 R73 tokens are fresh.

## Consequences

- Packet scope grows by up to 10 k/v rows per symbol when all five
  snapshots are populated — roughly +220 bytes per CDL snapshot
  (pattern_value, prev, body metrics, penetration or total_delta
  scalar, last_match, days_since_pattern, close), for a typical
  +1.10 KB per symbol.
- Schema is strictly additive; old peers running v74 continue to
  work (new tables are absent but none of the old tables change).
  LAN sync skips unknown tables via the whitelist.
- All five indicators share the HP cache with zero additional
  network dependencies. All three 3-bar patterns
  (MORNINGSTAR/EVENINGSTAR/3BLACKCROWS/3WHITESOLDIERS) need n≥4
  bars; Dark Cloud Cover (2-bar) needs n≥3.
- The 10 Round 73 tests (5 roundtrip + 5 compute) guard against
  serialization drift and detector regressions. Each compute test
  builds a synthetic bar sequence with the exact pattern geometry
  (e.g., Morning Star = explicit [big_red, tiny_star, big_green]
  triplet with deterministic closes) and asserts `pattern_value`
  matches the expected `+100` / `-100` sign convention.
- **Validates the R72 helper design.** Round 72 explicitly claimed
  that `cdl_scan` + `candle_metrics` let future rounds add new
  patterns in one function; Round 73's five additions all reuse
  the unchanged helpers verbatim. The claim holds — no R73
  modification to either helper was needed despite adding 3-bar
  patterns for the first time.
- Pattern-value convention (`+100` / `-100` / `0`) continues
  matching TA-Lib's canonical output so downstream code that
  cross-references TA-Lib reference implementations gets directly
  comparable scalars without translation.
- The `penetration_pct` naming is deliberately shared across
  Morning Star, Evening Star, and Dark Cloud Cover — all three
  patterns define their confirmation strength as a body-relative
  penetration into a prior or opening bar's body, so agents can
  compare "how decisively confirmed" across these three
  primitives via a single normalised metric.

## Verification

1. **Engine tests:** `cargo test --package typhoon-engine --lib`
   passes with +10 Round 73 tests over Round 72's count (1465 total
   including Round 73 additions).
2. **Native build:** `cargo build --package typhoon-native` completes
   cleanly with no warnings/errors in 4m 05s.
3. **Unique palette tokens:** All 25 Round 73 palette tokens fresh —
   zero collisions with earlier rounds (verified against R60..R72).
4. **LAN sync whitelist:** All five `research_*` tables registered
   in `SYNCABLE_TABLES`, `create_table_sql`, and
   `table_timestamp_column`; incremental sync uses the `updated_at`
   column.
5. **Pattern-value convention coverage:**
   `cdl_morning_star_compute_detects` asserts `pattern_value ==
   100` on a synthetic bullish triplet with `penetration_pct > 0`.
   `cdl_evening_star_compute_detects` asserts `pattern_value ==
   -100` on the mirrored bearish triplet.
   `cdl_three_black_crows_compute_detects` asserts `pattern_value
   == -100` on three synthetic consecutive red bars with
   `total_close_decline_pct < 0`.
   `cdl_three_white_soldiers_compute_detects` asserts `pattern_value
   == 100` on three synthetic consecutive green bars with
   `total_close_advance_pct > 0`.
   `cdl_dark_cloud_cover_compute_detects` asserts `pattern_value ==
   -100` on a 2-bar green-then-red sequence with penetration_pct
   > 0 (penetration measured as red's close below prior midpoint).

## Packet envelope delta

Before Round 73: packet emitted 186 k/v rows across Round 60..72
additions. After Round 73: 196 k/v rows when all seventy
Round 60..73 additions populate, typical +1.10 KB per symbol on top
of the +1.10 KB Round 72 added, +1.10 KB Round 71 added, +1.04 KB
Round 70 added, +0.94 KB Round 69 added, +1.13 KB Round 68 added,
+1.22 KB Round 67 added, +1.05 KB Round 66 added, +1.45 KB Round 65
added, +1.45 KB Round 64 added, +1.45 KB Round 63 added, +1.45 KB
Round 62 added, +1.40 KB Round 61 added, and +1.46 KB Round 60
added — bringing the observed per-symbol envelope from
~98-182 KB to ~99-183 KB.
