# ADR-160: Godel Parity Round 50 — STOCH / MACD / VWAP / MCGD / RWI

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-159
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 49 (ADR-159) shipped CMO/QSTICK/DISPARITY/BOP/SCHAFF, taking
HP-local research surfaces to 197 and per-symbol sub-blocks to 238.
Round 50 — the half-century milestone — closes five of the most-cited
canonical indicators that were *still* missing after 49 rounds of
additive work. Each has a sharp domain purpose distinct from what is
already shipped.

1. **No stand-alone Stochastic Oscillator snapshot.** STOCH (George C.
   Lane, 1950s) is the canonical %K/%D momentum oscillator:
   `%K = 100 · (close − lowest_low_N) / (highest_high_N − lowest_low_N)`
   smoothed by `%D = SMA(%K, d)`, with an optional smoothing of %K
   itself (the "full" stochastic). Canonical params: 14/3/3. We ship
   STOCHRSI (stochastic of RSI) already, but *not* the original Lane
   stochastic on raw prices. Distinct from RSI (smoothed gain/loss
   ratio), STOCHRSI (stochastic of RSI), MFI (money-flow), and any
   high/low-based signal (e.g., AROON): STOCH measures where the
   current close sits inside the recent high/low range. Overbought
   > 80, oversold < 20 is Lane's original rule. Header gives
   **stoch_label** (OVERBOUGHT >80 / BULL >50 / NEUTRAL / BEAR <50
   / OVERSOLD <20 / INSUFFICIENT_DATA). First surface we ship that
   reports raw stochastic on price (without RSI as intermediate).

2. **No MACD snapshot.** MACD (Gerald Appel, late 1970s, published
   1979) is the most widely-cited momentum indicator in existence:
   `MACD = EMA(close, 12) − EMA(close, 26)`, signal `= EMA(MACD, 9)`,
   histogram `= MACD − signal`. Distinct from PPO (MACD-% —
   `100 · MACD / EMA_slow`), SCHAFF (stochastic-of-MACD), and any
   single-EMA-difference read: MACD canonical 12/26/9 is a baseline
   that almost every other oscillator is benchmarked against. Header
   gives **macd_label** (BULL_CROSS — signal crossed up within last
   2 bars / BULL / NEUTRAL / BEAR / BEAR_CROSS — signal crossed down
   within last 2 bars / INSUFFICIENT_DATA). The explicit cross states
   are what readers typically look for — unlike a plain threshold,
   MACD's informative moments are the zero-cross of the histogram.

3. **No Volume-Weighted Average Price snapshot.** VWAP (institutional
   traders, 1980s, Berkowitz/Logue/Noser formalised) is the canonical
   "fair price" reference line: `VWAP = Σ(price · volume) / Σ(volume)`
   over a rolling window. We use a 20-bar rolling window (daily tape
   → one trading month). Distinct from any plain MA (unweighted),
   from VROC (volume rate-of-change), and from KLINGER (volume
   oscillator): VWAP is a *price* level computed by weighting by
   volume, not a volume indicator. Header gives **vwap_label**
   (STRONG_ABOVE >2% / ABOVE >0 / AT / BELOW <0 / STRONG_BELOW <−2%
   / INSUFFICIENT_DATA) based on deviation of close from VWAP.
   The canonical intraday VWAP (session-anchored, tick-by-tick) is
   beyond free-data scope — see "Paid-API gap" below.

4. **No McGinley Dynamic snapshot.** MCGD (John R. McGinley, 1991,
   *Market Technicians Association Journal*) is an adaptive moving
   average that self-tunes its responsiveness to volatility:
   `MCGD_t = MCGD_{t-1} + (close − MCGD_{t-1}) / (k · N · (close / MCGD_{t-1})^4)`,
   with k = 0.6 and N = 14. Distinct from EMA (fixed weight α =
   2/(N+1)), KAMA (Kaufman, which adapts to *efficiency ratio*),
   FRAMA (which adapts to fractal dimension), and HMA (Hull, which
   is just a specific WMA-chain): MCGD adapts to the *ratio* of price
   to its own trailing MA raised to the 4th power — a self-tuning
   feedback loop that slows the MA during fast moves (to reduce
   whipsaw) and speeds it up during slow drift. Header gives
   **mcgd_label** (STRONG_BULL >+2% / BULL / NEUTRAL / BEAR /
   STRONG_BEAR <−2% / INSUFFICIENT_DATA) based on price deviation
   from MCGD. Sits between EMA (non-adaptive) and KAMA (adaptive by
   ER) — a third, price-ratio-based adaptation axis.

5. **No Random Walk Index snapshot.** RWI (E. Michael Poulos, 1991,
   *Technical Analysis of Stocks & Commodities*) tests whether the
   current N-bar move is statistically *larger* than what a random
   walk would produce over the same period:
   `RWI_high_t = max_over_lookback(i in 2..N) { (high_t − low_{t−i+1}) / (ATR * sqrt(i)) }`
   and symmetrically for `RWI_low`. Distinct from ADX (Wilder DMI,
   which tests directional movement > neutral), VORTEX (Botes/Siepman,
   cross-period price action), and AROON (time-since-extremum):
   RWI directly tests the null hypothesis "no trend exists" — a
   reading above 1.0 indicates the move is larger than 1 σ of a random
   walk. Poulos's rule: RWI_high > 1.0 and RWI_high > RWI_low →
   genuine uptrend; RWI_low > 1.0 and RWI_low > RWI_high → genuine
   downtrend; else range-bound. Header gives **rwi_label**
   (TRENDING_UP / TRENDING_DOWN / RANGE_BOUND / INSUFFICIENT_DATA).
   First surface we ship that explicitly models the "random walk"
   null as the threshold.

Round 50 ships these five surfaces as ADR-160. Same additive envelope
as Rounds 5–49: no new fetchers, no cross-symbol scans, no new
external API dependencies. All five compute from the trailing HP
cache.

## Decision

Ship Round 50 as a five-surface additive bundle using schema v51
layered on v50:

| Surface  | Table             | Purpose                                                                      |
|----------|-------------------|------------------------------------------------------------------------------|
| STOCH    | `research_stoch`  | Lane Stochastic Oscillator (raw %K/%D on price, 14/3/3)                      |
| MACD     | `research_macd`   | Appel Moving Average Convergence Divergence (12/26/9 EMA)                    |
| VWAP     | `research_vwap`   | Rolling 20-bar Volume-Weighted Average Price + deviation                     |
| MCGD     | `research_mcgd`   | McGinley Dynamic (adaptive MA with self-tuning responsiveness, length 14)    |
| RWI      | `research_rwi`    | Poulos Random Walk Index (high/low ATR-normalised excursion, length 14)      |

Each table follows the established JSON-blob-per-symbol shape:

```sql
CREATE TABLE research_<name> (
    symbol TEXT PRIMARY KEY,
    snapshot_json TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_research_<name>_updated ON research_<name>(updated_at);
```

Each snapshot carries a regime `label` field. STOCH uses 5 buckets
(OVERBOUGHT/BULL/NEUTRAL/BEAR/OVERSOLD) matching Lane's original.
MACD uses the signal-cross-aware 5-bucket (BULL_CROSS/BULL/NEUTRAL/
BEAR/BEAR_CROSS) reflecting how traders actually read MACD. VWAP and
MCGD use the %-deviation 5-bucket. RWI uses the explicit 3-bucket
(TRENDING_UP / TRENDING_DOWN / RANGE_BOUND) to match Poulos's
original rule set. All surfaces emit INSUFFICIENT_DATA when the
smoother chain or lookback window is too short.

## Consequences

### Positive

- **Fills the last canonical-oscillator gap.** Before Round 50 the
  repo shipped RSI, STOCHRSI, MFI, Williams %R, MACD-of-STC (SCHAFF),
  but not *bare* Lane STOCH or *bare* Appel MACD. Round 50 closes
  both — the two most frequently cited oscillators in technical
  analysis are now direct native queries.
- **First fair-price reference line.** VWAP-on-daily-bars is the
  canonical "where should the price be given the volume profile"
  anchor. Complements all MA/EMA surfaces (unweighted) and all
  volume oscillators (KLINGER, CHAIKOSC, OBV, CMF, AD) — none of
  those report a *price level*, only momentum/flow. First Round-50
  deliverable that gives an institutional "fair value" reference.
- **First non-EMA-family adaptive MA.** MCGD joins EMA, DEMA, TEMA,
  HMA, KAMA, FRAMA, ALMA in our MA surface set but is the *only*
  one whose adaptation coefficient is `(close / MCGD_{t-1})^4` —
  a self-referential price-ratio feedback loop. Slows down in fast
  markets, speeds up in slow ones; the opposite of a fixed-weight
  EMA. Fills a real gap — until now, users wanting MCGD (popular
  in trend-following CTA stacks) had no native query.
- **First statistical null-hypothesis trend test.** RWI explicitly
  frames trend detection as "is this move larger than 1σ of a random
  walk over N bars?" — the rigorous statistical statement behind
  what ADX and VORTEX imply. Complements ADX (DMI-based trend
  strength) and VORTEX (cross-period VI+ / VI−) with an explicit
  random-walk null.
- **No new external dependencies, no fetcher expansion.** Pure
  compute on the HP cache — same additive envelope as Rounds 26–49.

### Negative / Risks

- **Schema migration.** `create_research_tables_v51` is additive
  over v50; peers on v50 who receive v51 rows via LAN sync will
  create the 5 new tables via the existing create-before-insert
  path. No back-compat break.
- **STOCH divide-by-zero guard.** When highest_high == lowest_low
  over the N-bar lookback (dead-flat tape), we default %K to 50
  (NEUTRAL). Documented; min_bars = k_period + d_period + smoothing
  = 20 minimum for stable compute. Ship with 14/3/3 → 20.
- **MACD signal-cross window.** BULL_CROSS / BEAR_CROSS are detected
  if the histogram flipped sign in the last 2 bars. A window of 1
  would miss the commonly-referenced "cross bar"; 2 is the smallest
  window that gives the user one full bar to respond. Tradeoff:
  repeated-cross whipsaw in range-bound tapes will flash CROSS
  multiple times.
- **VWAP window choice.** Rolling 20 bars on daily tape (one month
  of trading days) is a tradeoff: true intraday VWAP is session-
  anchored and tick-weighted (requires intraday data), but that
  sits behind paid data feeds. A 20-bar rolling window is the
  canonical "monthly VWAP" on daily bars and is what Bloomberg's
  `VWAP 20` field reports. Documented; min_bars = 20.
- **MCGD initial bar.** The McGinley recursion requires a seed; we
  seed `MCGD_0 = close_0`. First 14 bars should be considered
  warmup (hence `bars_used` in the snapshot — can filter). Docs
  reflect this; min_bars = length + 1 = 15.
- **RWI numerical edges.** When ATR is effectively zero (dead-flat
  tape), RWI is undefined — we emit INSUFFICIENT_DATA in that
  specific case even if bar count is sufficient. Documented;
  min_bars = length + 1 = 15.
- **Packet weight.** STOCH adds ~210 bytes, MACD ~250 (extra fields
  for histogram), VWAP ~200, MCGD ~220, RWI ~200. Total Round 50
  addition: ~1.08 KB/symbol. Updated envelope numbers appear in
  the RESEARCH_PACKET.md header.

### Neutral

- **Palette alias `VWAP` collision.** Bare `VWAP` was already
  shipping as a chart-overlay toggle (toggles `self.show_vwap` —
  draws a VWAP line on the active chart). To avoid breaking that
  muscle memory, the *snapshot* window uses `VWAPFIT` / `VWAP_WIN` /
  `VWAP_SNAPSHOT` / `VOLUME_WEIGHTED` / `VOL_WEIGHTED_AVG` as
  palette aliases. Bare `VWAP` still toggles the overlay — the
  research snapshot is a separate view. Documented in code and
  in-app help.
- **Palette alias verification for others.** Bare `STOCH`, `MACD`,
  `MCGD`, `RWI` are all unbound upstream (verified via grep across
  `native/src/app.rs` for
  `show_stoch|show_macd|show_mcgd|show_rwi` — no pre-Round-50
  matches). Bare names and disambiguated forms both kept as aliases.
- **STOCH vs STOCHRSI distinction.** The two are related but distinct
  surfaces — STOCH runs on *raw prices*, STOCHRSI runs on *RSI
  values*. STOCHRSI will typically be more sensitive in
  already-trending tapes; STOCH will give clearer regime reads in
  range-bound tapes. Users should consult both when analysing
  momentum context.
- **MACD vs PPO vs SCHAFF distinction.** PPO is `100 · MACD /
  EMA_slow` (a percentage version — easier to compare across price
  levels). SCHAFF is stochastic-of-MACD-double-smoothed (a lead
  indicator with more whipsaw). Raw MACD (this surface) is the
  baseline everyone else is compared against. All three have
  complementary use cases.
- **All five surfaces use the same broker handler shape** stable
  since Round 22. All compute purely from the HP cache — no
  cross-symbol reads.
- **Field-name `_win` suffix** on native struct fields follows the
  Round 43–49 convention.

### Paid-API gap (for later revisit)

Remaining gaps are data-access-gated:

- **Intraday session-anchored VWAP.** Bloomberg's canonical intraday
  VWAP uses tick-by-tick data anchored at the session open, which
  requires a paid intraday feed. Our Round 50 VWAP is a rolling
  daily-bar VWAP — still useful as a longer-horizon fair-value
  reference, but distinct from the session-anchored institutional
  VWAP. Revisit when an intraday data feed lands (see ADR-108's
  deferred list).
- **Level-2 order book depth, options IV surfaces, corporate
  actions feeds, realised-variance matrices, insider
  transactions** — same as ADR-159. No Round 50 surface needed any
  of these.

## Verification

- `cargo test -p typhoon-engine --lib` — 1220 passing after Round 50
  engine layer committed (up from 1210 after Round 49, +10 new:
  5 roundtrip + 5 compute_oscillating).
- `cargo build -p typhoon-engine` — clean (35s).
- `cargo build -p typhoon-native` — clean (1m 15s).
- STOCH/MACD/VWAP/MCGD/RWI compute_oscillating use the ±0.5%
  oscillating fixture (150 bars). Each asserts label belongs to its
  regime set, scalars are finite when label is not
  INSUFFICIENT_DATA, and axis-specific invariants:
  STOCH percent_k/percent_d finite and in [0, 100], k_period=14,
  d_period=3, smoothing=3;
  MACD macd_value/signal_value/histogram/histogram_prev finite,
  fast=12, slow=26, signal=9;
  VWAP vwap_value finite and > 0, deviation_pct finite, window=20;
  MCGD mcgd_value/mcgd_prev finite and > 0, deviation_pct finite,
  length=14;
  RWI rwi_high/rwi_low finite and ≥ 0, length=14.

## Packet envelope

After Round 50, single-symbol packet target envelope is **~76-148 KB**
(up from 75-147 in Round 49). Basket (10 symbols via BASKET) is
**~760-1480 KB** (up from 750-1470). Sub-block count grows 238 → 243.

Total HP-local research snapshot count after Round 50: **202**
(197 + 5). Total cross-symbol rank snapshots unchanged.
