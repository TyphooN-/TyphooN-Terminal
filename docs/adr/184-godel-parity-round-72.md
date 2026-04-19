# ADR-184: TA-Lib Parity Round 72 — CDLDOJI / CDLHAMMER / CDLSHOOTINGSTAR / CDLENGULFING / CDLHARAMI

**Status:** Accepted
**Date:** 2026-04-18
**Supersedes/extends:** ADR-108 through ADR-183
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| CDLDOJI | No | Yes (`CDLDOJI`) | Yes | Yes | No (deferred — ADR-188) |
| CDLHAMMER | No | Yes (`CDLHAMMER`) | Yes | Yes | No (deferred — ADR-188) |
| CDLSHOOTINGSTAR | No | Yes (`CDLSHOOTINGSTAR`) | Yes | Yes | No (deferred — ADR-188) |
| CDLENGULFING | No | Yes (`CDLENGULFING`) | Yes | Yes | No (deferred — ADR-188) |
| CDLHARAMI | No | Yes (`CDLHARAMI`) | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure TA-Lib — opens the TA-Lib `CDL*` candlestick-pattern family with five common one- and two-bar formations (Doji, Hammer, Shooting Star, Engulfing, Harami) and establishes the shared `candle_metrics` + `cdl_scan` plumbing for future CDL* rounds.

## Context

Round 71 (ADR-183) closed out five orphan TA-Lib primitives from
already-partial families (AROONOSC / MINMAXINDEX / MACDEXT / MACDFIX /
MAVP). Round 72 opens a completely new family: **TA-Lib's CDL*
candlestick pattern recognition primitives**. TA-Lib ships 60+ CDL*
functions covering single-bar, two-bar, and three-bar formations.
Round 72 picks the five most commonly referenced patterns and
establishes the pattern-recognition plumbing that future rounds can
extend (MORNINGSTAR / EVENINGSTAR / THREEBLACKCROWS / THREEWHITESOLDIERS
/ DARKCLOUDCOVER / DRAGONFLYDOJI / ... all share the same ingest →
detect → emit signed-integer skeleton).

1. **No CDLDOJI snapshot.** The doji is the canonical single-bar
   indecision pattern — open and close within a small fraction of
   the bar's range, signalling equilibrium between buyers and
   sellers. TA-Lib emits `100` when present and `0` otherwise.
   Traditionally treated as directionally neutral (could precede
   either reversal), so the snapshot uses `DOJI_PATTERN` /
   `NO_PATTERN` (not a bull/bear split). Body cutoff: ≤ 5% of range.

2. **No CDLHAMMER snapshot.** Hammer is the canonical bullish
   single-bar reversal signal at the bottom of a downtrend: small
   body in the upper third of the range, long lower shadow (wick)
   at least 2× the body, minimal upper shadow. TA-Lib emits `+100`
   on match (TA-Lib classifies hammer as unambiguously bullish even
   in isolation). Thresholds: body_pct ≤ 30%, lower_shadow ≥
   2 × body, upper_shadow ≤ body.

3. **No CDLSHOOTINGSTAR snapshot.** Mirror of hammer for bearish
   reversals at tops: small body in the lower third, long upper
   shadow ≥ 2× body, minimal lower shadow. TA-Lib emits `-100`
   (unambiguously bearish). The two snapshots share most of the
   scalar surface but live in separate windows/tables for palette
   clarity (`HAMMER` and `SHOOTING_STAR` are distinct trader
   vocabulary even though the geometry is a reflection).

4. **No CDLENGULFING snapshot.** Two-bar reversal: current bar's
   body fully engulfs the prior bar's body AND is the opposite
   direction. Bullish engulfing = prior red, current green with
   `current_open ≤ prior_close` AND `current_close ≥ prior_open`.
   Bearish is the sign-flip. TA-Lib emits `+100` / `-100` / `0`.
   A `body_size_ratio = cur_body / prior_body` scalar (always
   `> 1.0` when a match exists) gives agents a "how decisive was
   the engulfing" measurement.

5. **No CDLHARAMI snapshot.** Two-bar inside-bar reversal
   signal: current body fully *contained* within prior body AND is
   the opposite direction. Think of harami as the inverted
   engulfing — prior bar is large, current bar is a small opposite-
   direction body nested inside. TA-Lib emits `+100` / `-100` / `0`.
   The `body_size_ratio` here is always `< 1.0` when a match
   exists (nice symmetry with engulfing's `> 1.0`).

## Decision

Add five optional per-symbol snapshot blocks to the Godel-parity
pipeline, each reusing the existing `research_historical_price` HP
cache and the standard research-table LAN-sync path (no new API
dependencies):

1. `research::CdlDojiSnapshot` + `compute_cdl_doji_snapshot` +
   `upsert_cdl_doji` + `get_cdl_doji` — serialised to
   `research_cdl_doji`.
2. `research::CdlHammerSnapshot` + `compute_cdl_hammer_snapshot` +
   `upsert_cdl_hammer` + `get_cdl_hammer` — serialised to
   `research_cdl_hammer`.
3. `research::CdlShootingStarSnapshot` +
   `compute_cdl_shooting_star_snapshot` + `upsert_cdl_shooting_star`
   + `get_cdl_shooting_star` — serialised to
   `research_cdl_shooting_star`.
4. `research::CdlEngulfingSnapshot` +
   `compute_cdl_engulfing_snapshot` + `upsert_cdl_engulfing` +
   `get_cdl_engulfing` — serialised to `research_cdl_engulfing`.
5. `research::CdlHaramiSnapshot` + `compute_cdl_harami_snapshot` +
   `upsert_cdl_harami` + `get_cdl_harami` — serialised to
   `research_cdl_harami`.

Two small private helpers keep the detect math DRY and establish the
plumbing pattern for the 55+ remaining TA-Lib CDL* primitives not
yet shipped:

- `candle_metrics(bar)` — returns `(body, range, upper_shadow,
  lower_shadow, body_pct_range, is_bullish)`. Computed once per
  bar and reused by every CDL detector. Guards against
  `range == 0` (degenerate flat bars) by returning `body_pct = 0`.
  Designed to be the single source of candle geometry for all
  future CDL* additions — body/shadow definitions diverge across
  pattern families, but the raw metrics are universal.
- `cdl_scan<F>(sorted, min_i, detector)` — walks the sorted bars
  backward from the last bar, invoking the pattern detector per
  bar. Returns `(last_bar_match, days_since_pattern, last_val,
  prev_val)` — the four scalars every CDL snapshot needs. Takes
  a `min_i` parameter so 2-bar patterns (ENGULFING / HARAMI) can
  request `min_i = 1` (the detector needs `i - 1` access)
  without off-by-one errors. This generic scanner is what makes
  adding a new CDL pattern in a future round a one-function
  change (define the detector, plug into cdl_scan).

All five compute functions follow the same skeleton: sort bars →
bail if `n < min_bars` → define closure detector → call `cdl_scan`
→ fill metrics + label → return snapshot. The only thing that
varies across the 5 is the per-pattern detector logic and the
final label.

Schema version bumps to v74 via `create_research_tables_v74` which
wraps v73 and adds five new CREATE TABLE stanzas (plus indexes on
`updated_at`). All five tables register in `SYNCABLE_TABLES`,
`create_table_sql`, and `table_timestamp_column` so LAN sync
incrementally propagates them to peer terminals.

Native wiring adds five `BrokerCmd::ComputeCdl*Snapshot` variants,
five `BrokerMsg::Cdl*SnapshotMsg` variants, twenty `App` fields
(show/symbol/snapshot/loading × 5), twenty defaults, five tokio-
spawned broker handlers (load HP cache → compute → upsert → emit
msg), five palette alias blocks, five packet-emitter blocks after
the Round 71 MAVP emitter, five egui windows with
Use-Chart / Load-Cached / Compute controls plus a striped Grid
summary, and five `BrokerMsg` match arms.

Palette tokens (verified fresh against R60..R71 at implementation
time — zero collisions including no overlap with the existing
`CANDLE` chart-type command or the common `BULLISH` / `BEARISH`
colour-lookup strings used elsewhere):
`CDLDOJI | CDLDOJIWIN | DOJI | DOJI_PATTERN | DOJI_CANDLE`;
`CDLHAMMER | CDLHAMMERWIN | HAMMER | HAMMER_PATTERN | HAMMER_CANDLE`;
`CDLSHOOTINGSTAR | SHOOTINGSTAR | SHOOTING_STAR | CDLSHOOTINGSTARWIN |
SHOOTING_STAR_PATTERN`;
`CDLENGULFING | ENGULFING | CDLENGULFINGWIN | ENGULFING_PATTERN |
ENGULFING_CANDLE`;
`CDLHARAMI | HARAMI | CDLHARAMIWIN | HARAMI_PATTERN | INSIDE_BAR`.
All 25 R72 tokens are fresh.

## Consequences

- Packet scope grows by up to 10 k/v rows per symbol when all five
  snapshots are populated — roughly +220 bytes per CDL snapshot
  (pattern_value, prev, body/shadow metrics, last_match,
  days_since_pattern, close), for a typical +1.10 KB per symbol.
- Schema is strictly additive; old peers running v73 continue to
  work (new tables are absent but none of the old tables change).
  LAN sync skips unknown tables via the whitelist.
- All five indicators share the HP cache with zero additional
  network dependencies. CDLDOJI/CDLHAMMER/CDLSHOOTINGSTAR need
  n≥2 bars; CDLENGULFING/CDLHARAMI need n≥3.
- The 10 Round 72 tests (5 roundtrip + 5 compute) guard against
  serialization drift and detector regressions. Each compute test
  builds a synthetic bar with the exact pattern geometry and
  asserts `pattern_value` matches the expected `+100` / `-100`
  sign convention.
- The `cdl_scan` + `candle_metrics` helper pair is the ticket of
  entry for future CDL* rounds. Adding CDLMORNINGSTAR (3-bar)
  means defining a detector closure that takes `i - 2`, `i - 1`,
  `i` and passing it to `cdl_scan(sorted, 2, ...)`. No new
  scaffolding needed.
- Pattern-value convention (`+100` / `-100` / `0`) matches
  TA-Lib's canonical output so downstream code that cross-
  references TA-Lib reference implementations gets directly
  comparable scalars without translation.

## Verification

1. **Engine tests:** `cargo test --package typhoon-engine --lib`
   passes with +10 Round 72 tests over Round 71's count (1455 total
   including Round 72 additions).
2. **Native build:** `cargo build --package typhoon-native` completes
   cleanly with no warnings/errors in 4m 06s.
3. **Unique palette tokens:** All 25 Round 72 palette tokens fresh —
   zero collisions with earlier rounds (verified against R60..R71).
4. **LAN sync whitelist:** All five `research_*` tables registered
   in `SYNCABLE_TABLES`, `create_table_sql`, and
   `table_timestamp_column`; incremental sync uses the `updated_at`
   column.
5. **Pattern-value convention coverage:** `cdl_doji_compute_detects_doji`
   asserts `pattern_value == 100` on a synthetic doji bar and
   `body_pct_range <= 5.0`. `cdl_hammer_compute_detects_hammer`
   asserts `pattern_value == 100` with `lower_shadow_pct >
   upper_shadow_pct`. `cdl_shooting_star_compute_detects`
   asserts `pattern_value == -100` with `upper_shadow_pct >
   lower_shadow_pct` (sign-flipped from hammer, as expected).
   `cdl_engulfing_compute_detects_bullish` asserts `pattern_value ==
   100` with `body_size_ratio > 1.0` (current engulfs prior).
   `cdl_harami_compute_detects_bullish` asserts `pattern_value ==
   100` with `body_size_ratio < 1.0` (current contained in prior —
   nice symmetry with engulfing).

## Packet envelope delta

Before Round 72: packet emitted 176 k/v rows across Round 60..71
additions. After Round 72: 186 k/v rows when all sixty-five
Round 60..72 additions populate, typical +1.10 KB per symbol on top
of the +1.10 KB Round 71 added, +1.04 KB Round 70 added, +0.94 KB
Round 69 added, +1.13 KB Round 68 added, +1.22 KB Round 67 added,
+1.05 KB Round 66 added, +1.45 KB Round 65 added, +1.45 KB Round 64
added, +1.45 KB Round 63 added, +1.45 KB Round 62 added, +1.40 KB
Round 61 added, and +1.46 KB Round 60 added — bringing the observed
per-symbol envelope from ~97-181 KB to ~98-182 KB.
