# ADR-154: Godel Parity Round 45 — VORTEX / CHOP / OBV / TRIX / HMA

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-153
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| VORTEX | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |
| CHOP | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |
| OBV | Canonical (all terminals) | Yes (`OBV`) | Yes | Yes | No (deferred — ADR-188) |
| TRIX | Canonical (all terminals) | Yes (`TRIX`) | Yes | Yes | No (deferred — ADR-188) |
| HMA | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** canonical technical-analysis primitives common across all terminals (Vortex, Choppiness Index, On-Balance Volume via TA-Lib `OBV`, TRIX via `TRIX`, Hull Moving Average). Vortex / Choppiness / HMA are not in the TA-Lib catalog.

## Context

Round 44 (ADR-153) shipped ADX/CCI/CMF/MFI/PSAR, taking HP-local
research surfaces to 172 and per-symbol sub-blocks to 213.
Continuing the standing "combing over for full Godel research
parity" directive, Round 45 closes five more canonical
technical-analysis gaps still missing after Round 44.

1. **No Vortex Indicator.** VI (Botes & Siepman, *Technical
   Analysis of Stocks & Commodities*, January 2010) is a
   directional-movement alternative to ADX that measures positive
   and negative "vortex movement" directly from bar extremes rather
   than from the difference in consecutive highs/lows. At period=14:
   VM+ = |H_t − L_{t−1}|, VM− = |L_t − H_{t−1}|, VI+ = ΣVM+ / ΣTR,
   VI− = ΣVM− / ΣTR. Header gives **vortex_label** (BULL_CROSS if
   VI+ > VI− with VI+ > 1 / BULL VI+ > VI− / NEUTRAL / BEAR VI− > VI+
   / BEAR_CROSS VI− > VI+ with VI− > 1 / INSUFFICIENT_DATA).
   Complements ADX (Round 44): ADX is smoothed and lagged, VI is
   unsmoothed and catches direction changes earlier.

2. **No Choppiness Index.** CI (Bill Dreiss, Australian TA
   pioneer, 1980s) answers the "is this a trend or a range?"
   question as a bounded 0–100 scalar.
   CI = 100 · log10(ΣTR / (maxH − minL)) / log10(N). Values > 61.8
   indicate choppy/ranging, < 38.2 indicate trending; the thresholds
   come from the Fibonacci complements 61.8%/38.2%. Header gives
   **chop_label** (CHOP >61.8 / RANGING >50 / NEUTRAL /
   TRANSITIONAL <50 / TRENDING <38.2 / INSUFFICIENT_DATA). Distinct
   from ADX: ADX measures *trend strength*, CHOP measures *range
   efficiency* and is bounded by construction.

3. **No On-Balance Volume.** OBV (Joseph Granville, *New Key to
   Stock Market Profits*, 1963) is the canonical cumulative
   volume indicator: OBV_t = OBV_{t−1} + sign(ΔClose) · Volume_t.
   First published in Granville's 1963 book; since value depends on
   history, we pair it with a 20-bar linear-regression slope
   normalised against the OBV range to emit a label. Header gives
   **obv_label** (STRONG_UP / UP / NEUTRAL / DOWN / STRONG_DOWN /
   INSUFFICIENT_DATA). Complements CMF (Round 44): CMF is bounded
   [−1, +1] and forgets old volume, OBV is unbounded and remembers
   all history.

4. **No TRIX oscillator.** TRIX (Jack Hutson, *Stocks &
   Commodities*, 1983) applies three successive EMAs to close to
   filter out cycles shorter than N bars, then takes the 1-bar
   rate-of-change as a momentum proxy: EMA3 = EMA(EMA(EMA(close,
   N), N), N); TRIX = 100·(EMA3_t/EMA3_{t−1} − 1); signal =
   EMA(TRIX, 9). Period 15 is the default. Header gives
   **trix_label** (STRONG_BULL TRIX > 0 && TRIX > signal && |TRIX| >
   0.05 / BULL TRIX > 0 / NEUTRAL / BEAR TRIX < 0 / STRONG_BEAR
   TRIX < 0 && TRIX < signal && |TRIX| > 0.05 / INSUFFICIENT_DATA).
   Complements MACD (already shipped): MACD is EMA-EMA spread, TRIX
   is EMA³ rate-of-change — triple smoothing removes more noise at
   the cost of more lag.

5. **No Hull Moving Average.** HMA (Alan Hull, 2005) is a weighted-
   moving-average construct specifically designed to reduce lag
   while smoothing: HMA = WMA(2·WMA(n/2) − WMA(n), √n). The inner
   term 2·WMA(n/2) − WMA(n) has near-zero lag (the difference
   cancels the linear phase delay), and the outer WMA(√n) smooths
   the result over √n bars. Period 20 is the common default (half
   10, √ ~4). Header gives **hma_label** (STRONG_UP slope >2% /
   UP slope >0.2% / NEUTRAL / DOWN slope <−0.2% / STRONG_DOWN
   slope <−2% / INSUFFICIENT_DATA). Complements KAMA (Round 42)
   and the simple/exp MAs: HMA is specifically the *least-lagged*
   smoother of the three.

Round 45 ships these five surfaces as ADR-154. Same additive envelope
as Rounds 5–44: no new fetchers, no cross-symbol scans, no new
external API dependencies. All five compute from the trailing HP
cache.

## Decision

Ship Round 45 as a five-surface additive bundle using schema v46
layered on v45:

| Surface | Table                 | Purpose                                                               |
|---------|-----------------------|-----------------------------------------------------------------------|
| VORTEX  | `research_vortex`     | Botes & Siepman Vortex Indicator (VI+, VI−, VM±, TR sums)             |
| CHOP    | `research_chop`       | Bill Dreiss Choppiness Index (ΣTR / range ratio in log space)         |
| OBV     | `research_obv`        | Granville On-Balance Volume cumulative + 20-bar slope                 |
| TRIX    | `research_trix`       | Triple-EMA rate-of-change momentum oscillator + 9-bar signal          |
| HMA     | `research_hma`        | Hull Moving Average (WMA-of-diff-of-WMAs)                             |

Each table follows the established JSON-blob-per-symbol shape:

```sql
CREATE TABLE research_<name> (
    symbol TEXT PRIMARY KEY,
    snapshot_json TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_research_<name>_updated ON research_<name>(updated_at);
```

Each snapshot carries a regime `label` field (5 active buckets +
`INSUFFICIENT_DATA` sentinel). Label thresholds summarised above.

## Consequences

### Positive

- **Closes the "directional movement alternatives" gap.** VORTEX
  is the other standard directional-movement system (ADX is the
  Wilder one); absence was a parity blind spot. CHOP is the
  canonical bounded trend-vs-range scalar and was missing.
- **Adds the canonical cumulative-volume indicator.** OBV is
  taught in every intro-TA course and is the oldest of the
  volume-based indicators (Granville 1963, predates CMF/MFI by
  ~20 years). Including it closes the "cumulative vs period-based"
  volume-indicator dimension.
- **First triple-smoothed momentum oscillator.** MACD is EMA-EMA
  spread, TSI is double-smoothed, TRIX is triple-smoothed —
  different trade-offs between noise rejection and lag. TRIX
  gives us the highest-smoothing end of that spectrum.
- **HMA is explicitly the least-lagged smoother.** We have SMA,
  EMA, KAMA; HMA is the zero-lag-by-construction member of the MA
  family. Useful for any signal that needs fast turn-detection
  without false-positive-ing on noise.
- **No new external dependencies, no fetcher expansion.** Pure
  compute on the HP cache — same additive envelope as Rounds 26–44.

### Negative / Risks

- **Schema migration.** `create_research_tables_v46` is additive
  over v45; peers on v45 who receive v46 rows via LAN sync will
  create the 5 new tables via the existing create-before-insert
  path. No back-compat break.
- **TRIX warmup is long.** Triple EMA at N=15 plus 9-bar signal
  needs about 3·N + 9 = 54 bars before stabilising; we require
  55+ bars minimum and label shorter tapes as `INSUFFICIENT_DATA`.
  Documented.
- **OBV requires volume.** Bars with zero volume (halts, holidays)
  contribute zero to the cumulative — this is the standard
  Granville treatment but means OBV can *plateau* during halted
  sessions, not continue trending. Documented.
- **CHOP divides by (maxH − minL).** On perfectly flat tape this
  denominator is 0; we guard and emit chop_value=0 with the
  INSUFFICIENT_DATA label rather than NaN. Documented.
- **HMA uses floor(√period).** √20 = 4.47 → floor = 4; some
  implementations use round (5) or ceiling. We chose floor to
  match TradingView's default. Documented.
- **Packet weight.** VORTEX adds ~260 bytes, CHOP ~220, OBV ~260,
  TRIX ~230, HMA ~220. Total Round 45 addition: ~1.2 KB/symbol.
  Updated envelope numbers appear in the RESEARCH_PACKET.md
  header.

### Neutral

- **Label-based color scheme continues** the convention from
  Rounds 24–44. For CHOP, TRENDING uses the *favorable* color
  (green) even though trending is directionally-agnostic —
  consistent with the "trend-friendly is what most strategies
  prefer" rule.
- **Palette alias disambiguation.** Bare `OBV` and `HMA` are
  already bound to chart-overlay toggles upstream (chart-overlay
  booleans `show_obv`, `show_hma`). Round 45 research windows for
  OBV/HMA use disambiguated aliases only (`OBVFIT`, `OBV_WIN`,
  `HMAFIT`, `HMA_WIN`, etc.) to avoid shadowing the chart-overlay
  handlers. Bare `VORTEX`, `CHOP`, `TRIX` are unbound (verified
  via grep across `native/src/app.rs`) and kept as aliases for
  their research windows.
- **All five surfaces use the same broker handler shape** stable
  since Round 22. All compute purely from the HP cache — no
  cross-symbol reads.
- **Field-name `_win` suffix** on native struct fields follows the
  Round 43/44 convention to avoid colliding with chart-overlay
  booleans in the same `TyphoonApp` struct.

### Paid-API gap (for later revisit)

Same as ADR-153. The remaining gaps are data-access-gated
(intraday bars for intraday-specific indicators like VWAP-by-
session or time-and-sales-driven signals, Level-2 order book
depth, options IV surfaces, corporate actions feeds, realised-
variance matrices, insider transactions feed). No Round 45 surface
needed any of these; all compute from the daily HP cache.

## Verification

- `cargo test -p typhoon-engine --lib` — 1166 passing (up from
  1156 in Round 44, +10 new: 5 roundtrip + 5 compute_oscillating).
- `cargo check -p typhoon-engine` — clean.
- `cargo check -p typhoon-native` — clean; Round 45 field names use
  `_win` suffix to avoid collision with existing chart-overlay
  booleans (`show_obv`, `show_hma`).
- VORTEX/CHOP/OBV/TRIX/HMA compute_oscillating use the ±0.5%
  oscillating fixture (150 bars). Each asserts label belongs to
  its regime set, scalars are finite when label is not
  INSUFFICIENT_DATA, and axis-specific invariants:
  VORTEX vi_plus/vi_minus ≥ 0, sum_tr > 0, period=14;
  CHOP chop_value ∈ [0, 110] (allowing small numeric overshoot),
  range_high ≥ range_low, period=14;
  OBV obv_value/obv_slope finite, obv_max_20 ≥ obv_min_20,
  slope_window=20; TRIX trix_value/signal_value finite,
  ema3_value > 0, period=15 signal_period=9; HMA hma_value finite
  and positive, hma_slope_pct finite, period=20 half_period=10
  sqrt_period≥4.

## Packet envelope

After Round 45, single-symbol packet target envelope is **~71-141 KB**
(up from 70-139 in Round 44). Basket (10 symbols via BASKET) is
**~710-1410 KB** (up from 700-1390). Sub-block count grows 213 → 218.

Total HP-local research snapshot count after Round 45: **177**
(172 + 5). Total cross-symbol rank snapshots unchanged.
