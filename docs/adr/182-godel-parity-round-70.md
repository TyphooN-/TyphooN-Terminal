# ADR-182: TA-Lib Parity Round 70 — BBANDS / AD / ADOSC / SUM / LINEARREG_INTERCEPT

**Status:** Accepted
**Date:** 2026-04-18
**Supersedes/extends:** ADR-108 through ADR-181
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| BBANDS | No | Yes (`BBANDS`) | Yes | Yes | No (deferred — ADR-188) |
| AD | No | Yes (`AD`) | Yes | Yes | No (deferred — ADR-188) |
| ADOSC | No | Yes (`ADOSC`) | Yes | Yes | No (deferred — ADR-188) |
| SUM | No | Yes (`SUM`) | Yes | Yes | No (deferred — ADR-188) |
| LINEARREG_INTERCEPT | No | Yes (`LINEARREG_INTERCEPT`) | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure TA-Lib — a mixed 5-pack closing partial families: `BBANDS` raw bands, TA-Lib `AD` / `ADOSC` parallels to existing ADL / CHAIKOSC, `SUM` rolling sum primitive, and `LINEARREG_INTERCEPT` completing the linear-regression family.

## Context

Round 69 (ADR-181) shipped the TA-Lib rolling-extrema family (MIN /
MAX / MINMAX / MININDEX / MAXINDEX). Round 70 lands a mixed 5-pack
that closes several partial TA-Lib families at once — picked
opportunistically rather than by cohesive theme because the remaining
TA-Lib surface is increasingly sparse after 9 consecutive rounds of
additions.

1. **No BBANDS snapshot.** TA-Lib `BBANDS` is Bollinger's original
   band structure: `middle = SMA_20(close)`, `upper = middle + 2·σ`,
   `lower = middle − 2·σ`. The most widely cited volatility band
   envelope in technical analysis. While BBSQUEEZE (Round 22) and
   BBWIDTH (ADR-117 / Round 12) already expose *derived* scalars
   (squeeze percentile, bandwidth ratio), neither provides the raw
   band levels themselves or the standard `pct_b = 100·(close −
   lower) / (upper − lower)` position-within-band scalar. Header
   gives **bbands_label** (ABOVE_UPPER / UPPER_HALF / LOWER_HALF /
   BELOW_LOWER / INSUFFICIENT_DATA) from direct close-vs-band
   comparison.

2. **No TA-Lib AD snapshot.** The engine already exposes `AdlSnapshot`
   (Chaikin Accumulation/Distribution Line, Round 7 era) with a
   slope-over-20-bars label classifier. Round 70 adds the *TA-Lib*
   variant with a distinct label scheme (STRONG_ACCUM / ACCUM /
   FLAT / DIST / STRONG_DIST / INSUFFICIENT_DATA) fed by a 10-bar
   OLS slope normalised to mean-|AD| for rel-magnitude thresholding.
   The parallel snapshot is intentional: TA-Lib agents cross-
   referencing against CTD AD examples expect the primitive under
   the bare `AD` name with TA-Lib-style bands. ADL remains available
   under its original palette (`ADL / ADLFIT / ACCUM_DIST / ...`)
   with the longer-horizon classifier for code that depends on it.

3. **No TA-Lib ADOSC snapshot.** Mirror of (2) for the Chaikin A/D
   Oscillator: the engine ships `ChaikoscSnapshot` (Round 7 era)
   with its own label scheme; Round 70 adds the TA-Lib-aliased
   `AdoscSnapshot` with STRONG_BULL / BULL / FLAT / BEAR /
   STRONG_BEAR bands fed by adosc / mean-|AD| ratio thresholds
   (±10% strong, ±2% directional, else flat). Same parallel-
   implementation rationale.

4. **No SUM snapshot.** TA-Lib `SUM = Σ close_{t-n+1..t}` over a
   30-bar window — the primitive that SMA is built on top of (SMA =
   SUM / period). Distinct from SMA because SUM is an absolute
   quantity suitable for compounding and delta comparisons.
   Labels classify sum-over-sum momentum (STRONG_UP / UP / FLAT /
   DOWN / STRONG_DOWN / INSUFFICIENT_DATA) from the percent change
   between the current sum and the sum ending one bar earlier
   (±1.0% strong, ±0.2% directional).

5. **No LINEARREG_INTERCEPT snapshot.** The engine ships
   `LinearRegSnapshot`, `LinearRegAngleSnapshot`,
   `LinearRegSlopeSnapshot`, and `TsfSnapshot` — four of the five
   TA-Lib linear-regression primitives. The fifth, `LINEARREG_INTERCEPT`
   — the `b` coefficient in `y = m·x + b` — was skipped in earlier
   rounds. Useful when agents need to reconstruct the full regression
   line or ask "how far has price walked from the oldest bar's
   regression base?" The informative scalar is `drift = last_close −
   intercept` and its `drift_pct` form; labels (STRONG_ADVANCE /
   ADVANCE / FLAT / DECLINE / STRONG_DECLINE / INSUFFICIENT_DATA)
   use ±5% and ±1% cutoffs on drift_pct.

## Decision

Add five optional per-symbol snapshot blocks to the Godel-parity
pipeline, each reusing the existing `research_historical_price` HP
cache and the standard research-table LAN-sync path (no new API
dependencies):

1. `research::BbandsSnapshot` + `compute_bbands_snapshot` +
   `upsert_bbands` + `get_bbands` — serialised to `research_bbands`.
2. `research::AdSnapshot` + `compute_ad_snapshot` + `upsert_ad` +
   `get_ad` — serialised to `research_ad`.
3. `research::AdoscSnapshot` + `compute_adosc_snapshot` +
   `upsert_adosc` + `get_adosc` — serialised to `research_adosc`.
4. `research::SumSnapshot` + `compute_sum_snapshot` + `upsert_sum` +
   `get_sum` — serialised to `research_sum`.
5. `research::LinearRegInterceptSnapshot` +
   `compute_linearreg_intercept_snapshot` + `upsert_linreg_intercept`
   + `get_linreg_intercept` — serialised to
   `research_linreg_intercept`.

Three small private helpers keep the compute math DRY:
- `sma_stddev(sorted, end_idx, period)` — BBANDS's mean + population
  stddev (TA-Lib uses population variance, not sample) computed in
  one pass per end index.
- `ad_line(sorted)` — cumulative Chaikin A/D series, reused by both
  `compute_ad_snapshot` and `compute_adosc_snapshot` so the two
  primitives consume the same numeric base.
- `last_window_slope(values, period)` — OLS slope of `y[n-period..n]`
  vs `x = [0..period)`, used by AD's 10-bar slope. The intercept-
  specific computation in LINEARREG_INTERCEPT is inlined because it
  also needs the `b` coefficient (not just the slope).

Schema version bumps to v72 via `create_research_tables_v72` which
wraps v71 and adds five new CREATE TABLE stanzas (plus indexes on
`updated_at`). All five tables register in `SYNCABLE_TABLES`,
`create_table_sql`, and `table_timestamp_column` so LAN sync
incrementally propagates them to peer terminals.

Native wiring adds five `BrokerCmd::Compute*Snapshot` variants, five
`BrokerMsg::*SnapshotMsg` variants, five `show_*_win` / `*_symbol` /
`*_snapshot` / `*_loading` field tuples on `App`, five tokio-spawned
broker handlers (load HP cache → compute → upsert → emit msg), five
palette alias blocks, five packet-emitter blocks after the Round 69
MAXINDEX emitter, five egui windows with Use-Chart / Load-Cached /
Compute controls plus a striped Grid summary, and five `BrokerMsg`
match arms.

Palette aliases were selected to avoid collision with earlier rounds
(verified at implementation time against R60..R69 token sets):
`BBANDS | BBANDSWIN | BB_BANDS | BBAND | BOLL_BANDS`;
`AD | AD_LINE_TALIB | AD_CHAIKIN | ADWIN | TALIB_AD`;
`ADOSC | ADOSCWIN | TALIB_ADOSC | AD_OSCILLATOR | CHAIKIN_ADO`;
`SUM | SUMWIN | ROLLSUM | CLOSE_SUM | SUM_CLOSE`;
`LINEARREG_INTERCEPT | LINREG_INTERCEPT | LINTERCEPT | LRINTERCEPT |
REG_INTERCEPT | LINEARREG_B`. All 26 tokens are fresh — zero
collisions with earlier rounds. In particular, `BOLLINGER` (a chart-
context filter string) and `CHAIKIN_ADL` / `AD_LINE` / `ACCUM_DIST`
(claimed by the existing `ADL` command) and `CHAIKIN_OSC` /
`CHAIKIN_OSCILLATOR` (claimed by the existing `CHAIKOSC` command)
were explicitly avoided — the TA-Lib-aliased variants use `TALIB_*`
and `*_CHAIKIN` suffixes to disambiguate.

## Consequences

- Packet scope grows by up to 10 k/v rows per symbol when all five
  snapshots are populated — roughly +280 bytes for BBANDS (three
  band values, three prev-bar values, close, %B, bandwidth),
  +180 bytes for AD (ad, prev, delta, slope, close), +200 bytes for
  ADOSC (adosc, prev, ad_ref, close, periods), +180 bytes for SUM
  (sum, prev, delta, pct, close), +200 bytes for LINEARREG_INTERCEPT
  (intercept, prev, slope, close, drift, drift_pct) — for a typical
  +1.04 KB per symbol.
- Schema is strictly additive; old peers running v71 continue to
  work (new tables are absent but none of the old tables change).
  LAN sync skips unknown tables via the whitelist.
- All five indicators share the HP cache with zero additional
  network dependencies. BBANDS needs n≥21 bars, AD needs n≥12,
  ADOSC needs n≥12, SUM needs n≥31, LINEARREG_INTERCEPT needs n≥15.
- Like Round 69 + earlier rounds, the 10 Round 70 tests (5
  roundtrip + 5 compute) guard against serialization drift and
  band-cutoff regressions. The bbands_compute and sum_compute tests
  additionally verify mathematical identities (middle = SMA_20(close),
  sum = Σ close_{n-30..n}) to catch refactor drift.
- The parallel AD / ADL and ADOSC / CHAIKOSC implementations are a
  deliberate tradeoff: they consume a few extra k/v slots per symbol
  but give TA-Lib-style consumers a canonically-named primitive with
  TA-Lib-style bands, without breaking the legacy classifier.

## Verification

1. **Engine tests:** `cargo test --package typhoon-engine --lib`
   passes with +10 Round 70 tests over Round 69's count (1435 total
   including Round 70 additions).
2. **Native build:** `cargo build --package typhoon-native` completes
   cleanly with no warnings/errors in 3m 57s.
3. **Unique palette tokens:** All 26 Round 70 palette tokens fresh —
   zero collisions with earlier rounds (verified against the 25 Round
   69 tokens and the cumulative R60..R68 set).
4. **LAN sync whitelist:** All five `research_*` tables registered
   in `SYNCABLE_TABLES`, `create_table_sql`, and
   `table_timestamp_column`; incremental sync uses the `updated_at`
   column.
5. **Mathematical-identity coverage:** `bbands_compute_oscillating`
   re-derives `middle` as `SMA_20(close)` and asserts the stored
   scalar matches within 1e-6. `sum_compute_oscillating` re-derives
   `sum` as `Σ close_{n-30..n}` and asserts the same.
   `linreg_intercept_compute_oscillating` asserts `drift ≡
   last_close − intercept`. All three fail fast if the shared helpers
   drift from their stated semantics.

## Packet envelope delta

Before Round 70: packet emitted 156 k/v rows across Round 60..69
additions. After Round 70: 166 k/v rows when all fifty-five
Round 60..70 additions populate, typical +1.04 KB per symbol on top
of the +0.94 KB Round 69 added, +1.13 KB Round 68 added, +1.22 KB
Round 67 added, +1.05 KB Round 66 added, +1.45 KB Round 65 added,
+1.45 KB Round 64 added, +1.45 KB Round 63 added, +1.45 KB Round 62
added, +1.40 KB Round 61 added, and +1.46 KB Round 60 added —
bringing the observed per-symbol envelope from ~95-179 KB to ~96-180
KB.
