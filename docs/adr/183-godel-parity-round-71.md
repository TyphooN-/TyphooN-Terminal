# ADR-183: TA-Lib Parity Round 71 — AROONOSC / MINMAXINDEX / MACDEXT / MACDFIX / MAVP

**Status:** Accepted
**Date:** 2026-04-18
**Supersedes/extends:** ADR-108 through ADR-182
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| AROONOSC | No | Yes (`AROONOSC`) | Yes | Yes | No (deferred — ADR-188) |
| MINMAXINDEX | No | Yes (`MINMAXINDEX`) | Yes | Yes | No (deferred — ADR-188) |
| MACDEXT | No | Yes (`MACDEXT`) | Yes | Yes | No (deferred — ADR-188) |
| MACDFIX | No | Yes (`MACDFIX`) | Yes | Yes | No (deferred — ADR-188) |
| MAVP | No | Yes (`MAVP`) | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure TA-Lib — five orphan primitives completing partial families: `AROONOSC` standalone oscillator, `MINMAXINDEX` combined-indices extrema, `MACDEXT` configurable-MA MACD, `MACDFIX` fixed-12/26 MACD, `MAVP` variable-period moving average.

## Context

Round 70 (ADR-182) shipped five mixed TA-Lib primitives (BBANDS / AD /
ADOSC / SUM / LINEARREG_INTERCEPT). Round 71 takes a different tack:
rather than picking opportunistically, it closes out five **orphan
primitives from already-partial families** — the TA-Lib surface is
thin enough after 10 rounds of additions that "complete the family"
is now a coherent theme. All five primitives here are direct
complements of existing snapshots.

1. **No AROONOSC snapshot.** The existing AROON primitive (Round 24
   era) already computes `aroon_oscillator = aroon_up − aroon_down`
   as a secondary field, but using period 25 (not TA-Lib's canonical
   14) and routing via the bundled AROON command. TA-Lib ships
   `AROONOSC` as a first-class standalone primitive — it's the
   oscillator form, not a bundled output — and agents cross-
   referencing TA-Lib examples expect it under the bare `AROONOSC`
   name with period 14. Round 71 adds a dedicated snapshot with
   period 14 (TA-Lib default) and a distinct label palette
   (STRONG_BULL / BULL / FLAT / BEAR / STRONG_BEAR on ±50 and ±15
   cutoffs). The `AROONOSC` palette alias has been moved from the
   AROON command (where it was a bundled synonym) to this new
   dedicated command.

2. **No MINMAXINDEX snapshot.** Round 69 (ADR-181) shipped the
   rolling-extrema family — MIN / MAX / MINMAX / MININDEX /
   MAXINDEX — with a shared `window_extrema` helper. The one missing
   TA-Lib primitive from that family was `MINMAXINDEX`, which
   returns both indices together. This round closes the gap: the
   combined snapshot emits both bars-ago values plus a signed
   `age_diff` and an `extrema_order` category (HIGH_FIRST /
   LOW_FIRST / SAME_BAR) so agents can reason about the within-window
   directional signature in one read.

3. **No MACDEXT snapshot.** The existing MACD (Round 7 era) is
   hardcoded to EMA for fast / slow / signal. TA-Lib's `MACDEXT`
   takes a per-MA MAType parameter, enabling e.g. SMA-based MACD (the
   classical simple-MACD form from the 1970s) or hybrid variants.
   Round 71 exposes the SMA form as a separate snapshot — agents
   asking "what would MACD look like if all three MAs were SMA?"
   now get a deterministic answer without re-running the math.

4. **No MACDFIX snapshot.** Historically the most widely used MACD
   parametrisation is 12 / 26 / 9. TA-Lib reserves the `MACDFIX`
   name for this textbook form with *hardcoded* 12 / 26 fast / slow
   (signal remains configurable). While the existing MACD snapshot
   happens to default to 12 / 26 / 9 too, the explicit hardcoded-
   constraint surface gives agents a way to verify they're looking
   at the textbook parameters vs. a configured variant. Uses EMA
   (the TA-Lib default) to keep mathematical equivalence with the
   canonical form.

5. **No MAVP snapshot.** TA-Lib's `MAVP` (Moving Average with
   Variable Period) takes a *per-bar period array* — unlike SMA /
   EMA / WMA / TRIMA which all use a single fixed period across
   every bar, MAVP allows the lookback to vary per bar. This is
   powerful for regime-adaptive MA calculation (longer window in
   calm markets, shorter in volatile). Round 71 implements a linear
   ramp period function (5 at start → 30 at end) to exercise the
   polymorphic behaviour and emit a scalar at the last bar. Label
   classifies the sign of `mavp_delta` (STRONG_UP / UP / FLAT /
   DOWN / STRONG_DOWN).

## Decision

Add five optional per-symbol snapshot blocks to the Godel-parity
pipeline, each reusing the existing `research_historical_price` HP
cache and the standard research-table LAN-sync path (no new API
dependencies):

1. `research::AroonoscSnapshot` + `compute_aroonosc_snapshot` +
   `upsert_aroonosc` + `get_aroonosc` — serialised to
   `research_aroonosc`.
2. `research::MinMaxIndexSnapshot` + `compute_minmaxindex_snapshot` +
   `upsert_minmaxindex` + `get_minmaxindex` — serialised to
   `research_minmaxindex`.
3. `research::MacdextSnapshot` + `compute_macdext_snapshot` +
   `upsert_macdext` + `get_macdext` — serialised to
   `research_macdext`.
4. `research::MacdfixSnapshot` + `compute_macdfix_snapshot` +
   `upsert_macdfix` + `get_macdfix` — serialised to
   `research_macdfix`.
5. `research::MavpSnapshot` + `compute_mavp_snapshot` +
   `upsert_mavp` + `get_mavp` — serialised to `research_mavp`.

Three small private helpers keep the compute math DRY and
guarantee cross-primitive consistency:
- `aroon_up_down(sorted, end_idx, period)` — shared AROON calculation
  (high/low-based, not close-based) so AROONOSC and potential future
  AROON variants never drift apart arithmetically. Matches the math
  already used by `compute_aroon_snapshot` but generalised over
  period.
- `macd_triplet(closes, fast, slow, signal, ma_fn)` — the MACD
  line + signal + histogram triplet parameterised by a `Fn(&[f64],
  usize) -> Vec<f64>` MA function. MACDEXT calls it with
  `sma_series`, MACDFIX with `ema_series`. Eliminates the drift risk
  of two copies of the same math.
- `macd_label(hist, hist_prev)` — the shared 5-band classifier
  (STRONG_BULL / BULL / FLAT / BEAR / STRONG_BEAR) using both the
  sign of `hist` and the direction of change from `hist_prev`. Same
  function classifies both MACDEXT and MACDFIX so their labels are
  directly comparable.

The existing `window_extrema` helper (Round 69) is reused without
modification for MINMAXINDEX — it already returns both indices in
one walk. The MAVP compute inlines a simple closure-based period
ramp since the per-bar variable-period behaviour is the primitive's
defining characteristic.

Schema version bumps to v73 via `create_research_tables_v73` which
wraps v72 and adds five new CREATE TABLE stanzas (plus indexes on
`updated_at`). All five tables register in `SYNCABLE_TABLES`,
`create_table_sql`, and `table_timestamp_column` so LAN sync
incrementally propagates them to peer terminals.

Native wiring adds five `BrokerCmd::Compute*Snapshot` variants, five
`BrokerMsg::*SnapshotMsg` variants, five `show_*_win` / `*_symbol` /
`*_snapshot` / `*_loading` field tuples on `App`, five tokio-spawned
broker handlers (load HP cache → compute → upsert → emit msg), five
palette alias blocks, five packet-emitter blocks after the Round 70
LINEARREG_INTERCEPT emitter, five egui windows with Use-Chart /
Load-Cached / Compute controls plus a striped Grid summary, and five
`BrokerMsg` match arms.

Palette token analysis yielded one collision: `AROONOSC` was already
a bundled synonym in the existing `AROON` command. Round 71 removes
it from the AROON block and reserves it for the dedicated AROONOSC
command (with `AROONOSCWIN | AROON_OSC | AROONOSCILLATOR |
AROON_DIFF` as alternative aliases). Remaining palette tokens:
`MINMAXINDEX | MMIDXWIN | MINMAX_IDX | EXTREMA_IDX | HL_IDX`;
`MACDEXT | MACDEXTWIN | MACD_EXT | MACD_CONFIG | MACD_FLEX`;
`MACDFIX | MACDFIXWIN | MACD_FIX | MACD_12_26 | MACD_STD`;
`MAVP | MAVPWIN | VAR_PERIOD_MA | MA_VARPERIOD | MA_DYNAMIC`. All
25 R71 tokens are effectively fresh — zero collisions with R60..R70
other than the moved AROONOSC (which is not a loss since the AROON
window still exposes the oscillator value via its internal field).

## Consequences

- Packet scope grows by up to 10 k/v rows per symbol when all five
  snapshots are populated — roughly +180 bytes for AROONOSC
  (aroonosc, prev, up, down, close), +220 bytes for MINMAXINDEX
  (min_idx, max_idx, age_diff, order, close), +260 bytes for
  MACDEXT (macd, signal, hist + prev, ma_type, close), +240 bytes
  for MACDFIX (macd, signal, hist + prev, close), +200 bytes for
  MAVP (mavp, prev, delta, last_period, close) — for a typical
  +1.10 KB per symbol.
- Schema is strictly additive; old peers running v72 continue to
  work (new tables are absent but none of the old tables change).
  LAN sync skips unknown tables via the whitelist.
- All five indicators share the HP cache with zero additional
  network dependencies. AROONOSC needs n≥16 bars, MINMAXINDEX
  needs n≥31, MACDEXT/MACDFIX need n≥37 (slow + signal + 2), MAVP
  needs n≥32 (max_period + 2).
- Like prior rounds, the 10 Round 71 tests (5 roundtrip + 5 compute)
  guard against serialization drift and band-cutoff regressions.
  The aroonosc_compute, macdext_compute, macdfix_compute, and
  mavp_compute tests additionally verify mathematical identities
  (`aroonosc ≡ aroon_up − aroon_down`, `hist ≡ macd − signal`,
  `mavp_delta ≡ mavp − mavp_prev`) to catch shared-helper refactor
  drift.
- The parallel MACDEXT / MACDFIX / existing MACD implementations are
  a deliberate tradeoff: three snapshots for what is arguably "one
  indicator with different parameters" — but the TA-Lib surface
  explicitly distinguishes them as separate primitives, and agents
  cross-referencing TA-Lib documentation expect all three names to
  resolve to canonically-named snapshots.

## Verification

1. **Engine tests:** `cargo test --package typhoon-engine --lib`
   passes with +10 Round 71 tests over Round 70's count (1445 total
   including Round 71 additions).
2. **Native build:** `cargo build --package typhoon-native` completes
   cleanly with no warnings/errors in 4m 01s.
3. **Unique palette tokens:** All 25 Round 71 palette tokens
   available — `AROONOSC` recovered from its prior binding in the
   AROON command; remaining 20 tokens verified fresh against R60..R70
   and the cumulative earlier-round set.
4. **LAN sync whitelist:** All five `research_*` tables registered
   in `SYNCABLE_TABLES`, `create_table_sql`, and
   `table_timestamp_column`; incremental sync uses the `updated_at`
   column.
5. **Mathematical-identity coverage:** `aroonosc_compute_oscillating`
   asserts `osc ≡ aroon_up − aroon_down` and `osc ∈ [-100, 100]`.
   `macdext_compute_oscillating` and `macdfix_compute_oscillating`
   both assert `hist ≡ macd − signal`. `mavp_compute_oscillating`
   asserts `delta ≡ mavp − mavp_prev`. `minmaxindex_compute` asserts
   `age_diff == min_idx − max_idx` (signed). All five tests fail
   fast if the shared helpers drift from their stated semantics.

## Packet envelope delta

Before Round 71: packet emitted 166 k/v rows across Round 60..70
additions. After Round 71: 176 k/v rows when all sixty
Round 60..71 additions populate, typical +1.10 KB per symbol on top
of the +1.04 KB Round 70 added, +0.94 KB Round 69 added, +1.13 KB
Round 68 added, +1.22 KB Round 67 added, +1.05 KB Round 66 added,
+1.45 KB Round 65 added, +1.45 KB Round 64 added, +1.45 KB Round 63
added, +1.45 KB Round 62 added, +1.40 KB Round 61 added, and +1.46
KB Round 60 added — bringing the observed per-symbol envelope from
~96-180 KB to ~97-181 KB.
