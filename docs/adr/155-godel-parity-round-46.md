# ADR-155: TA-Lib + Godel Parity Round 46 — PPO / DPO / KST / ULTOSC / WILLR

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-154
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| PPO | Canonical (all terminals) | Yes (`PPO`) | Yes | Yes | No (deferred — ADR-188) |
| DPO | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |
| KST | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |
| ULTOSC | No | Yes (`ULTOSC`) | Yes | Yes | No (deferred — ADR-188) |
| WILLR | Canonical (all terminals) | Yes (`WILLR`) | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** canonical technical oscillators (PPO via TA-Lib `PPO`, DPO, KST, Williams %R via `WILLR`); Ultimate Oscillator is a TA-Lib-only primitive (`ULTOSC`) — not universally present on all terminals.

## Context

Round 45 (ADR-154) shipped VORTEX/CHOP/OBV/TRIX/HMA, taking HP-local
research surfaces to 177 and per-symbol sub-blocks to 218.
Continuing the standing "combing over for full Godel research
parity" directive, Round 46 closes five more canonical
technical-analysis gaps still missing after Round 45.

1. **No Percentage Price Oscillator.** PPO (Gerald Appel, the same
   author as MACD) is MACD's normalised twin: the difference of two
   EMAs expressed as a *percentage* of the slow EMA rather than as
   absolute price. Fast=12, slow=26, signal=9 mirror MACD's defaults.
   PPO = 100·(EMA₁₂ − EMA₂₆)/EMA₂₆; signal = EMA(PPO, 9); histogram
   = PPO − signal. Header gives **ppo_label** (STRONG_BULL PPO>0 &&
   PPO>signal && |PPO|>0.1 / BULL PPO>0 / NEUTRAL / BEAR PPO<0 /
   STRONG_BEAR PPO<0 && PPO<signal && |PPO|>0.1 / INSUFFICIENT_DATA).
   Complements MACD (already shipped): MACD's raw-price spread scales
   with price level (a $200 stock can't be compared to a $20 stock
   on MACD alone), PPO's percentage normalisation makes cross-symbol
   comparison meaningful.

2. **No Detrended Price Oscillator.** DPO (standard TA, pre-1980s)
   isolates short-term price cycles by removing the trend component.
   With period N=20 and shift=N/2+1=11: DPO_t = close_{t−shift} −
   SMA(close, N)_t. The shift is deliberate — it aligns the SMA with
   what it's smoothing, so the DPO represents how far a price from
   11 bars ago deviated from the centred moving average. Header gives
   **dpo_label** (PEAK_HIGH dpo%>5 / BULL dpo%>0.5 / NEUTRAL / BEAR
   dpo%<−0.5 / PEAK_LOW dpo%<−5 / INSUFFICIENT_DATA). Distinct from
   centered oscillators like RSI: DPO is *absolute-price-deviation*,
   not ratio-normalised, so fires cleanest on cycle-dominated instruments.

3. **No Know Sure Thing.** KST (Martin Pring, *Stocks & Commodities*,
   1992) is a weighted sum of four smoothed rate-of-change series:
   RCMA1 = SMA(ROC(10), 10), RCMA2 = SMA(ROC(15), 10), RCMA3 =
   SMA(ROC(20), 10), RCMA4 = SMA(ROC(30), 15). KST = 1·RCMA1 +
   2·RCMA2 + 3·RCMA3 + 4·RCMA4 (weights chosen so the longer-cycle
   ROC dominates — Pring designed KST as a *long-term cycle*
   oscillator, unlike MACD/PPO which are medium-term). Signal =
   SMA(KST, 9). Header gives **kst_label** (STRONG_BULL KST>0 &&
   KST>signal && |KST|>1 / BULL KST>0 / NEUTRAL / BEAR KST<0 /
   STRONG_BEAR KST<0 && KST<signal && |KST|>1 / INSUFFICIENT_DATA).
   Complements MACD/PPO/TRIX: those are single-cycle oscillators,
   KST is a four-cycle composite by construction.

4. **No Ultimate Oscillator.** ULTOSC (Larry Williams, 1976) addresses
   a known weakness of single-period oscillators: they react to
   either short-term noise (if period is low) or miss fast reversals
   (if period is high). Williams' fix: compute BP (buying pressure =
   close − min(low, prev_close)) and TR (true range = max(high,
   prev_close) − min(low, prev_close)) per bar, then take weighted
   averages across three timeframes: avg₇ = ΣBP₇/ΣTR₇, avg₁₄ =
   ΣBP₁₄/ΣTR₁₄, avg₂₈ = ΣBP₂₈/ΣTR₂₈, UO = 100·(4·avg₇ + 2·avg₁₄ +
   avg₂₈)/7. Header gives **ultosc_label** (OVERBOUGHT >70 / BULL >50
   / NEUTRAL / BEAR <50 / OVERSOLD <30 / INSUFFICIENT_DATA).
   Distinct from RSI/Stochastic: those use a single lookback,
   ULTOSC combines three at once, reducing false-positive divergences.

5. **No Williams %R.** %R (Larry Williams, *How I Made One Million
   Dollars Last Year Trading Commodities*, 1973) is the inverted
   Stochastic %K: %R_t = (highest_high_14 − close_t) / (highest_high_14
   − lowest_low_14) · −100, so output is in [−100, 0] with 0 being
   top of range (overbought) and −100 being bottom (oversold). Header
   gives **willr_label** (OVERBOUGHT >−20 / BULL >−50 / NEUTRAL / BEAR
   <−50 / OVERSOLD <−80 / INSUFFICIENT_DATA). Complements Stochastic
   (already shipped): mathematically %R = −100 − %K of the same
   period, but sign convention differs and the thresholds
   (−20/−80 vs 20/80) mean divergence reads differently in practice.

Round 46 ships these five surfaces as ADR-155. Same additive envelope
as Rounds 5–45: no new fetchers, no cross-symbol scans, no new
external API dependencies. All five compute from the trailing HP
cache.

## Decision

Ship Round 46 as a five-surface additive bundle using schema v47
layered on v46:

| Surface | Table                 | Purpose                                                               |
|---------|-----------------------|-----------------------------------------------------------------------|
| PPO     | `research_ppo`        | Appel Percentage Price Oscillator (MACD's % normalisation twin)       |
| DPO     | `research_dpo`        | Detrended Price Oscillator (SMA shift)                                |
| KST     | `research_kst`        | Pring Know Sure Thing (weighted 4-ROC composite)                      |
| ULTOSC  | `research_ultosc`     | Williams Ultimate Oscillator (3-timeframe weighted BP/TR)             |
| WILLR   | `research_willr`      | Williams %R (inverted stochastic, [−100, 0] output)                   |

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

- **Adds the cross-symbol-normalised MACD twin.** Absolute-spread
  MACD only works within a single symbol's price scale. PPO's
  percentage normalisation makes MACD-style momentum comparable
  across a 20-symbol basket — closes a practical gap when ranking
  the strength of moves rather than just detecting them.
- **First cycle-isolating oscillator.** DPO is the only canonical
  indicator in the TA literature specifically designed to *remove*
  trend and isolate *cycle*. Useful for instruments known to have
  strong seasonal or cyclical components (commodities, utilities).
- **First multi-cycle composite oscillator.** KST combines four
  different ROC periods in one surface — no other surface we've
  shipped does this. Pring's weights 1/2/3/4 explicitly emphasise
  the longer cycles, making KST complementary to MACD/PPO/TRIX
  which are all medium-term.
- **First 3-timeframe oscillator.** ULTOSC's 7/14/28 weighted combo
  is the only TA oscillator that blends short/mid/long into a single
  reading, addressing the "which lookback do I pick?" problem
  directly. Williams designed it specifically to reduce false
  divergences.
- **Adds the oldest canonical stochastic-family indicator.** Williams
  %R (1973) predates Lane's Stochastic (1950s publication, ~1970s
  popularisation) in published form, and is still the second
  most-taught range-location oscillator after Stochastic itself.
- **No new external dependencies, no fetcher expansion.** Pure
  compute on the HP cache — same additive envelope as Rounds 26–45.

### Negative / Risks

- **Schema migration.** `create_research_tables_v47` is additive
  over v46; peers on v46 who receive v47 rows via LAN sync will
  create the 5 new tables via the existing create-before-insert
  path. No back-compat break.
- **KST warmup is long.** ROC(30) + SMA(15) + 9-bar signal needs
  ~56 bars before stabilising; we require 56+ bars minimum and
  label shorter tapes as `INSUFFICIENT_DATA`. Documented.
- **PPO division by EMA_slow.** If EMA_slow ≈ 0 (essentially
  impossible for equity prices but theoretically possible for
  futures spreads or synthetic instruments), we guard with epsilon
  and emit ppo_value=0. Documented.
- **DPO past-index selection.** The shift is N/2+1=11 for N=20, so
  past_idx = t − 11. For tapes shorter than period+shift we return
  INSUFFICIENT_DATA. Documented.
- **ULTOSC bar 0 has no prev_close.** We skip bar 0 (BP₀=TR₀=0) and
  start computing at bar 1. For tapes of exactly period_long+1 = 29
  bars, the 28-period sum uses bars 1..28 giving exactly 28 samples.
  Documented.
- **Packet weight.** PPO adds ~270 bytes, DPO ~220, KST ~280,
  ULTOSC ~250, WILLR ~200. Total Round 46 addition: ~1.2 KB/symbol.
  Updated envelope numbers appear in the RESEARCH_PACKET.md
  header.

### Neutral

- **Label-based color scheme continues** the convention from
  Rounds 24–45. For DPO, PEAK_HIGH uses the *favorable* color
  (green) even though peaks predict mean-reversion down —
  consistent with the "label text is the regime, color is the
  typical-strategy orientation" convention used throughout.
- **Palette alias verification.** Bare `PPO`, `DPO`, `KST`,
  `ULTOSC`, `WILLR` are all unbound upstream (verified via grep
  across `native/src/app.rs` for `show_ppo|show_dpo|show_kst|
  show_ultosc|show_willr` — only the Round 46 `_win` struct fields
  match). Bare names and disambiguated forms both kept as aliases.
- **All five surfaces use the same broker handler shape** stable
  since Round 22. All compute purely from the HP cache — no
  cross-symbol reads.
- **Field-name `_win` suffix** on native struct fields follows the
  Round 43/44/45 convention to avoid colliding with chart-overlay
  booleans in the same `TyphoonApp` struct, even where no
  collision currently exists — consistency over case-by-case.

### Paid-API gap (for later revisit)

Same as ADR-154. The remaining gaps are data-access-gated
(intraday bars for intraday-specific indicators like VWAP-by-
session or time-and-sales-driven signals, Level-2 order book
depth, options IV surfaces, corporate actions feeds, realised-
variance matrices, insider transactions feed). No Round 46 surface
needed any of these; all compute from the daily HP cache.

## Verification

- `cargo test -p typhoon-engine --lib` — 1176 passing (up from
  1166 in Round 45, +10 new: 5 roundtrip + 5 compute_oscillating).
- `cargo check -p typhoon-engine` — clean.
- `cargo check -p typhoon-native` — clean.
- PPO/DPO/KST/ULTOSC/WILLR compute_oscillating use the ±0.5%
  oscillating fixture (150 bars). Each asserts label belongs to
  its regime set, scalars are finite when label is not
  INSUFFICIENT_DATA, and axis-specific invariants:
  PPO ema_fast>0, ema_slow>0, fast_period=12 slow_period=26
  signal_period=9;
  DPO sma_value>0, period=20 shift=11;
  KST rcma1/rcma4 finite, signal finite;
  ULTOSC ultosc ∈ [0, 100] (bounded by construction),
  period_short=7 period_mid=14 period_long=28;
  WILLR willr ∈ [−100, 0] (bounded by construction),
  highest_high ≥ lowest_low, period=14.

## Packet envelope

After Round 46, single-symbol packet target envelope is **~72-143 KB**
(up from 71-141 in Round 45). Basket (10 symbols via BASKET) is
**~720-1430 KB** (up from 710-1410). Sub-block count grows 218 → 223.

Total HP-local research snapshot count after Round 46: **182**
(177 + 5). Total cross-symbol rank snapshots unchanged.
