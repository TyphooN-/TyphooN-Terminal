# ADR-156: Godel Parity Round 47 — MASS / CHAIKOSC / KLINGER / STOCHRSI / AWESOME

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-155
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 46 (ADR-155) shipped PPO/DPO/KST/ULTOSC/WILLR, taking HP-local
research surfaces to 182 and per-symbol sub-blocks to 223.
Continuing the standing "combing over for full Godel research
parity" directive, Round 47 closes five more canonical
technical-analysis gaps still missing after Round 46.

1. **No Mass Index.** MASS (Donald Dorsey, *Stocks & Commodities*,
   June 1992) detects potential trend reversals via range expansion
   rather than price direction. For each bar compute high-low range;
   take EMA(H-L, 9) as a "single" smoother, then EMA of that EMA as
   the "double". The single ratio = EMA9(H-L) / EMA9(EMA9(H-L)).
   Mass Index = Σ(single ratio) over the last 25 bars. Header gives
   **mass_label** (REVERSAL_BULGE mass>27 / WATCH mass>25 / NEUTRAL /
   INSUFFICIENT_DATA). Dorsey specifically designed this to fire
   *before* the reversal, not at it — when the index crosses
   above 27 then drops back below 26.5, a change in trend direction
   is probable. Complements momentum/trend oscillators shipped
   so far: those measure direction, MASS measures *volatility
   expansion* as a reversal precursor.

2. **No Chaikin Oscillator.** CHAIKOSC (Marc Chaikin, ~1982) is the
   momentum derivative of the Accumulation/Distribution line.
   For each bar: money flow multiplier MFM = ((C-L)-(H-C))/(H-L),
   money flow volume MFV = MFM × volume, A/D line = cumulative
   Σ(MFV). Oscillator = EMA(A/D, 3) − EMA(A/D, 10). Header gives
   **chaikosc_label** (STRONG_ACCUM osc>thresh>0 / ACCUM osc>0 /
   NEUTRAL / DIST osc<0 / STRONG_DIST osc<−thresh<0 /
   INSUFFICIENT_DATA). Distinct from bare A/D (Round 40): A/D gives
   cumulative flow, CHAIKOSC derivates it so you see *changes* in
   accumulation speed — the raw A/D can trend up slowly for years
   while the oscillator flags local regime changes.

3. **No Klinger Volume Oscillator.** KLINGER (Stephen Klinger,
   1997) is the closest volume-based equivalent to MACD.
   Each bar: trend direction sign = sign of (HLC_curr − HLC_prev);
   daily range DM = H - L; cumulative range CM with trend-change
   reset (when trend flips, CM resets from the prior DM). Volume
   force VF = volume · 2 · ((DM/CM) − 1) · trend_sign · 100.
   KVO = EMA(VF, 34) − EMA(VF, 55). Signal = EMA(KVO, 13).
   Header gives **klinger_label** (STRONG_BULL kvo>signal && norm>1 /
   BULL kvo>signal / NEUTRAL / BEAR kvo<signal / STRONG_BEAR kvo<
   signal && norm<−1 / INSUFFICIENT_DATA). Klinger designed KVO
   specifically to reconcile long-term money flow with short-term
   price moves — a divergence between KVO and price is considered
   stronger-signal than a simple MACD divergence because it
   incorporates both direction AND volume into a single oscillator.

4. **No Stochastic RSI.** STOCHRSI (Tushar Chande & Stanley Kroll,
   *The New Technical Trader*, 1994) applies the Stochastic %K/%D
   formula to RSI values instead of prices. Start with Wilder-
   smoothed RSI(14). Then compute last 14 RSI values' range:
   raw = (RSI - min14(RSI)) / (max14(RSI) - min14(RSI)).
   Smooth with 3-bar %K = SMA(raw, 3) × 100, then 3-bar %D =
   SMA(%K, 3). Header gives **stochrsi_label** (OVERBOUGHT k>80 /
   BULL k>50 / NEUTRAL / BEAR k<50 / OVERSOLD k<20 /
   INSUFFICIENT_DATA). Chande & Kroll introduced this because plain
   RSI rarely hits true overbought/oversold — most RSI values cluster
   in [30, 70]. StochRSI forces the reading back onto [0, 100] of its
   *own* local range, so divergences and overbought/oversold trigger
   more reliably than plain RSI.

5. **No Awesome Oscillator.** AWESOME / AO (Bill Williams,
   *New Trading Dimensions*, 1998) is SMA(hl2, 5) − SMA(hl2, 34),
   where hl2 = (high + low) / 2. The simplest possible cross-period
   momentum oscillator — no EMA, no weighting, just two simple
   moving averages of the bar midpoint. AO tracks color change
   (green if AO > prev AO, red if AO < prev AO) as the primary
   signal; the zero-line cross is secondary. Header gives
   **awesome_label** (STRONG_BULL ao>0 && %pct>0.2 / BULL ao>0 /
   NEUTRAL / BEAR ao<0 / STRONG_BEAR ao<0 && %pct<−0.2 /
   INSUFFICIENT_DATA) plus `ao_color_up: bool`. Part of Williams'
   "alligator/awesome/fractal" trading-chaos framework — useful as
   a clean, slow-crossing momentum confirmation alongside
   faster oscillators.

Round 47 ships these five surfaces as ADR-156. Same additive envelope
as Rounds 5–46: no new fetchers, no cross-symbol scans, no new
external API dependencies. All five compute from the trailing HP
cache.

## Decision

Ship Round 47 as a five-surface additive bundle using schema v48
layered on v47:

| Surface    | Table                 | Purpose                                                               |
|------------|-----------------------|-----------------------------------------------------------------------|
| MASS       | `research_mass`       | Dorsey Mass Index (double-smoothed H-L ratio, 25-bar sum)             |
| CHAIKOSC   | `research_chaikosc`   | Chaikin Oscillator (EMA3(AD) − EMA10(AD))                             |
| KLINGER    | `research_klinger`    | Klinger Volume Oscillator (34/55/13, volume-based MACD)               |
| STOCHRSI   | `research_stochrsi`   | Chande/Kroll Stochastic RSI (14/14/3/3)                               |
| AWESOME    | `research_awesome`    | Bill Williams Awesome Oscillator (SMA5 − SMA34 on hl2)                |

Each table follows the established JSON-blob-per-symbol shape:

```sql
CREATE TABLE research_<name> (
    symbol TEXT PRIMARY KEY,
    snapshot_json TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_research_<name>_updated ON research_<name>(updated_at);
```

Each snapshot carries a regime `label` field (3-5 active buckets +
`INSUFFICIENT_DATA` sentinel). Label thresholds summarised above.

## Consequences

### Positive

- **First range-expansion reversal detector.** Every oscillator
  shipped so far is direction-aware (momentum, trend, cycle).
  MASS is direction-agnostic — it flags volatility expansion
  regardless of which way price is moving. Closes a distinct gap
  in the TA toolkit and is the only surface whose signal fires
  *before* a confirmed reversal.
- **Adds the momentum derivative of A/D.** Round 40 shipped raw
  A/D. CHAIKOSC is the natural next step: it tells you when A/D's
  slope is changing, not just whether accumulation is positive.
  For instruments like equity indices where A/D trends slowly
  upward for years, CHAIKOSC is the more actionable reading.
- **First volume-native MACD-family oscillator.** MACD and PPO
  are price-only. OBV (Round 45) is a volume-flow cumulator but
  not an oscillator. KLINGER combines both into one reading with
  explicit trend-change detection on HLC pivots — the only
  volume-native surface we've shipped that crosses a signal line.
- **First oscillator-of-oscillator.** STOCHRSI is RSI re-ranged
  on its own local values, producing a more evenly distributed
  [0, 100] signal than bare RSI. Particularly useful for
  instruments whose RSI rarely leaves [40, 60] — STOCHRSI forces
  those into observable overbought/oversold regimes.
- **Simplest of the new classics.** AWESOME/AO is the cleanest
  SMA-based momentum oscillator (zero EMA, zero weights) — useful
  as a "null hypothesis" reading to confirm signals from more
  complex oscillators like MACD/PPO/KST. Also ships the red/green
  color-change flag that's the core Williams trading rule.
- **No new external dependencies, no fetcher expansion.** Pure
  compute on the HP cache — same additive envelope as Rounds 26–46.

### Negative / Risks

- **Schema migration.** `create_research_tables_v48` is additive
  over v47; peers on v47 who receive v48 rows via LAN sync will
  create the 5 new tables via the existing create-before-insert
  path. No back-compat break.
- **KLINGER warmup is long.** EMA55 on volume-force needs ~55
  bars before stabilising, plus the 13-bar signal line means the
  effective minimum is ~71 bars. Documented; shorter tapes get
  `INSUFFICIENT_DATA`.
- **CHAIKOSC flat-bar guard.** The money flow multiplier
  `((C-L)-(H-C))/(H-L)` divides by H-L, which is zero for any
  "doji" bar where H==L. We guard with epsilon and treat flat
  bars as MFM=0 (no accumulation signal). Documented.
- **MASS double-smooth guard.** The single-ratio is `EMA9(H-L) /
  EMA9(EMA9(H-L))`, so division by the double-EMA of H-L. For
  instruments that go truly flat (H=L on every bar), this divides
  by zero. We guard with epsilon and return mass=0 /
  single_ratio=1 / label=NEUTRAL. Documented.
- **STOCHRSI denominator guard.** Raw = `(RSI - min) / (max - min)`.
  If max == min (RSI unchanged across 14 bars — rare but possible),
  we clamp raw=0.5 (neutral). Documented.
- **Packet weight.** MASS adds ~200 bytes, CHAIKOSC ~280, KLINGER
  ~300, STOCHRSI ~260, AWESOME ~230. Total Round 47 addition:
  ~1.3 KB/symbol. Updated envelope numbers appear in the
  RESEARCH_PACKET.md header.

### Neutral

- **Label-based color scheme continues** the convention from
  Rounds 24–46. For MASS, REVERSAL_BULGE uses the *cautionary*
  color (red) since the signal primarily indicates exhaustion
  and probable reversal, regardless of current trend direction.
- **Palette alias verification.** Bare `MASS`, `CHAIKOSC`,
  `KLINGER`, `STOCHRSI`, `AWESOME` are all unbound upstream
  (verified via grep across `native/src/app.rs` for
  `show_mass|show_chaikosc|show_klinger|show_stochrsi|show_awesome`
  — no matches before Round 47 fields added). Bare names and
  disambiguated forms both kept as aliases.
- **All five surfaces use the same broker handler shape** stable
  since Round 22. All compute purely from the HP cache — no
  cross-symbol reads.
- **Field-name `_win` suffix** on native struct fields follows the
  Round 43–46 convention to avoid colliding with chart-overlay
  booleans in the same `TyphoonApp` struct, even where no
  collision currently exists — consistency over case-by-case.
- **AO uses hl2, not close.** Bill Williams specifically chose
  the bar midpoint (high+low)/2 over close to represent "where
  price spent most of the bar" rather than the specific closing
  snapshot. We follow the original spec rather than substituting
  close.

### Paid-API gap (for later revisit)

Same as ADR-155. The remaining gaps are data-access-gated
(intraday bars for intraday-specific indicators like VWAP-by-
session or time-and-sales-driven signals, Level-2 order book
depth, options IV surfaces, corporate actions feeds, realised-
variance matrices, insider transactions feed). No Round 47 surface
needed any of these; all compute from the daily HP cache.

## Verification

- `cargo test -p typhoon-engine --lib` — 1186 passing (up from
  1176 in Round 46, +10 new: 5 roundtrip + 5 compute_oscillating).
- `cargo check -p typhoon-engine` — clean.
- `cargo check -p typhoon-native` — clean.
- MASS/CHAIKOSC/KLINGER/STOCHRSI/AWESOME compute_oscillating use the ±0.5%
  oscillating fixture (150 bars). Each asserts label belongs to
  its regime set, scalars are finite when label is not
  INSUFFICIENT_DATA, and axis-specific invariants:
  MASS mass_value ≥ 0, single_ratio ≥ 0, ema_period=9, sum_period=25;
  CHAIKOSC ema_fast_ad/ema_slow_ad finite, fast_period=3, slow_period=10;
  KLINGER kvo/signal finite, fast_period=34, slow_period=55, signal_period=13;
  STOCHRSI rsi ∈ [0, 100], %K ∈ [0, 100], %D ∈ [0, 100], rsi_max ≥ rsi_min,
  rsi_period=14, stoch_period=14;
  AWESOME sma_fast>0, sma_slow>0, fast_period=5, slow_period=34.

## Packet envelope

After Round 47, single-symbol packet target envelope is **~73-145 KB**
(up from 72-143 in Round 46). Basket (10 symbols via BASKET) is
**~730-1450 KB** (up from 720-1430). Sub-block count grows 223 → 228.

Total HP-local research snapshot count after Round 47: **187**
(182 + 5). Total cross-symbol rank snapshots unchanged.
