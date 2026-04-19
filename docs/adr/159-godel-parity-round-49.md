# ADR-159: TA-Lib + Godel Parity Round 49 — CMO / QSTICK / DISPARITY / BOP / SCHAFF

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-158
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| CMO | Canonical (all terminals) | Yes (`CMO`) | Yes | Yes | No (deferred — ADR-188) |
| QSTICK | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |
| DISPARITY | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |
| BOP | No | Yes (`BOP`) | Yes | Yes | No (deferred — ADR-188) |
| SCHAFF | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** canonical technical momentum / trend-cycle oscillators (Chande CMO via TA-Lib `CMO`, Q-Stick, Disparity Index, Schaff Trend Cycle); Balance of Power is a TA-Lib-only primitive (`BOP`) not broadly visible on chart terminals.

## Context

Round 48 (ADR-158) shipped EFI/EMV/NVI/PVI/COPPOCK, taking HP-local
research surfaces to 192 and per-symbol sub-blocks to 233. Continuing
the standing "combing over for full Godel research parity" directive,
Round 49 closes five more canonical momentum / trend-cycle gaps still
missing after Round 48.

1. **No Chande Momentum Oscillator.** CMO (Tushar Chande, 1994, in
   *The New Technical Trader*) is the simplest raw-gain-vs-loss
   spread oscillator: `100 · (Σ gains − Σ losses) / (Σ gains + Σ losses)`
   over an N-bar lookback (default 9). Bounded in [-100, +100].
   Distinct from RSI and STOCHRSI: RSI divides by the *sum of
   averages* (so bounded [0, 100] centred at 50), CMO divides by
   the sum of absolute changes (bounded at -100/0/+100 with zero
   as neutral). Overbought > +50, oversold < -50 is Chande's
   original rule. Header gives **cmo_label** (OVERBOUGHT >50 /
   BULL >0 / NEUTRAL / BEAR <0 / OVERSOLD <−50 / INSUFFICIENT_DATA).
   Complements RSI (smoothed) and STOCHRSI (stochastic of RSI) with
   a raw, un-smoothed momentum spread.

2. **No Q-Stick.** QSTICK (Tushar Chande, 1995) is an SMA over the
   candle body (close − open). Positive sustained value = consistent
   bullish candles (closes above opens); negative = consistent
   bearish candles. Distinct from AWESOME (SMA5 − SMA34 on hl2) and
   from any raw price-level indicator: QSTICK measures intra-bar
   sentiment directly (did buyers or sellers dominate inside each
   bar?) rather than inter-bar momentum. Header gives **qstick_label**
   (STRONG_BULL > 0 && |norm|>1% / BULL > 0 / NEUTRAL / BEAR < 0 /
   STRONG_BEAR < 0 && |norm|>1% / INSUFFICIENT_DATA). First surface
   we ship that aggregates candle-body sentiment.

3. **No Disparity Index.** DISPARITY (Japanese technical-analysis
   tradition, popularised in the West by Steve Nison) is
   `(close / SMA(close, n) − 1) · 100` — the percentage deviation of
   the current close from its N-bar SMA. Positive = price above the
   mean (bullish); extreme readings (|disparity| > ~3-5%) suggest
   mean-reversion pressure. Distinct from BOLLPCT (which normalises
   by volatility) and from any MA slope: DISPARITY measures the
   *gap* between price and its smoother, not its direction of
   change. Header gives **disparity_label** (STRONG_BULL >3% /
   BULL >0% / NEUTRAL / BEAR <0% / STRONG_BEAR <−3% /
   INSUFFICIENT_DATA). First surface we ship that reports price
   deviation in raw percentage terms (independent of volatility).

4. **No Balance of Power.** BOP (Igor Livshin) is per-bar
   `(close − open) / (high − low)`, smoothed by SMA14. Bounded in
   [-1, +1] per bar. BOP > 0.5 means buyers dominated the bar's
   range (close landed in the upper half); BOP < -0.5 means sellers
   dominated. Distinct from QSTICK (which reports body size in
   raw price units) and from CMF / AD (which weight by volume):
   BOP is a pure price-action sentiment indicator, independent of
   volume and independent of magnitude (only the *position* of the
   close within the bar's range matters). Header gives **bop_label**
   (STRONG_BULL >0.5 / BULL >0 / NEUTRAL / BEAR <0 / STRONG_BEAR
   <−0.5 / INSUFFICIENT_DATA). Tight with QSTICK (both measure
   intra-bar sentiment) but orthogonal — BOP normalises by range,
   QSTICK reports raw body size.

5. **No Schaff Trend Cycle.** SCHAFF (Doug Schaff, 2008) applies
   stochastic-oscillator logic to the MACD line, then smooths,
   then applies stochastic again, then smooths again. Result:
   a [0, 100] oscillator with *much* tighter turning points than
   bare MACD or bare stochastic. Schaff's original 2008 params:
   fast EMA = 23, slow EMA = 50, cycle = 10. Distinct from MACD
   (trend + sign), PPO (MACD-%), STOCHRSI (stochastic of RSI) and
   any single-smoothed oscillator: STC's double-stochastic +
   double-smoother chain produces turning points that lead most
   other momentum oscillators by 3-7 bars. Header gives
   **schaff_label** (OVERBOUGHT >75 && falling / BULL / NEUTRAL /
   BEAR / OVERSOLD <25 && rising / INSUFFICIENT_DATA). First
   surface we ship that combines *two* smoothing primitives
   (MACD + stochastic) rather than applying one smoother to one
   input. Min bars = ema_slow + cycle × 3 = 80 for stable compute.

Round 49 ships these five surfaces as ADR-159. Same additive envelope
as Rounds 5–48: no new fetchers, no cross-symbol scans, no new
external API dependencies. All five compute from the trailing HP
cache.

## Decision

Ship Round 49 as a five-surface additive bundle using schema v50
layered on v49:

| Surface    | Table                 | Purpose                                                                        |
|------------|-----------------------|--------------------------------------------------------------------------------|
| CMO        | `research_cmo`        | Chande Momentum Oscillator (raw gain/loss spread on [-100, +100], period 9)    |
| QSTICK     | `research_qstick`     | Chande Q-Stick (SMA14 of close − open candle body)                             |
| DISPARITY  | `research_disparity`  | Japanese Disparity Index ((close / SMA14 − 1) · 100)                           |
| BOP        | `research_bop`        | Livshin Balance of Power (SMA14 of (close − open) / (high − low))              |
| SCHAFF     | `research_schaff`     | Schaff Trend Cycle (stochastic-of-MACD, double-smoothed, 23/50/10)             |

Each table follows the established JSON-blob-per-symbol shape:

```sql
CREATE TABLE research_<name> (
    symbol TEXT PRIMARY KEY,
    snapshot_json TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_research_<name>_updated ON research_<name>(updated_at);
```

Each snapshot carries a regime `label` field. CMO/QSTICK/DISPARITY/BOP
use the 5-bucket STRONG_BULL/BULL/NEUTRAL/BEAR/STRONG_BEAR (or
OVERBOUGHT/OVERSOLD for CMO) scheme. SCHAFF uses 5 buckets including
explicit OVERBOUGHT && falling / OVERSOLD && rising gating.
All surfaces emit INSUFFICIENT_DATA when the bar count is too low
for the smoother chain to stabilise.

## Consequences

### Positive

- **First raw gain/loss spread oscillator.** CMO fills the gap
  between RSI (smoothed, centred at 50) and STOCHRSI (stochastic
  of RSI). The raw spread is Chande's original design and is
  prized for showing un-smoothed momentum directly. The ±50
  threshold is historically conservative (rarer signals, higher
  hit rate).
- **First intra-bar candle-body sentiment pair.** QSTICK + BOP
  together form a complete intra-bar sentiment read: QSTICK gives
  magnitude (SMA of raw body), BOP gives normalised position
  (SMA of body-over-range). Neither alone is sufficient — a wide
  body near the midpoint of a wider range is "mixed" to BOP but
  "strong" to QSTICK, and vice versa. Shipping both lets the
  reader disambiguate.
- **First raw percentage-deviation surface.** DISPARITY reports
  close-vs-SMA in raw percent, independent of volatility.
  Complements BOLLPCT (which normalises by σ) and any MA-slope
  read. Japanese traders have used this as a mean-reversion
  trigger for decades.
- **First double-smoothed, double-stochastic oscillator.** SCHAFF
  is the only surface we ship that combines two distinct smoothing
  primitives (MACD + stochastic) in a recursive chain. STC is the
  most responsive momentum oscillator in common use — a frequent
  answer to "what's the fastest read on trend change?" Adding it
  fills the end of the momentum-spectrum: CMO (raw) → RSI → MACD
  → STC (double-smoothed).
- **No new external dependencies, no fetcher expansion.** Pure
  compute on the HP cache — same additive envelope as Rounds 26–48.

### Negative / Risks

- **Schema migration.** `create_research_tables_v50` is additive
  over v49; peers on v49 who receive v50 rows via LAN sync will
  create the 5 new tables via the existing create-before-insert
  path. No back-compat break.
- **CMO divide-by-zero guard.** When Σ(up) + Σ(down) = 0 (flat
  bars throughout the lookback), we default CMO to 0 (NEUTRAL).
  Documented; min_bars = period + 2 = 11.
- **DISPARITY divide-by-zero guard.** When the SMA is effectively
  zero (near-zero prices), we default disparity to 0. Not a real-
  world risk on equities but guarded for safety. Documented.
- **BOP divide-by-zero guard.** When high = low (dead flat bar),
  we clamp (high − low) to 1e-9 so per-bar BOP contributes ~0
  rather than NaN. Documented; min_bars = period + 2 = 16.
- **SCHAFF high min-bars.** ema_slow (50) + cycle × 3 (30) = 80
  bars minimum for stable compute. Below that, INSUFFICIENT_DATA.
  Shorter tapes will see no SCHAFF reading. Daily HP cache normally
  carries ≥252 bars, so this is rarely a constraint — but worth
  calling out for symbols with short listing histories.
- **Packet weight.** CMO adds ~210 bytes, QSTICK ~200, DISPARITY
  ~210, BOP ~210, SCHAFF ~230. Total Round 49 addition: ~1.06
  KB/symbol. Updated envelope numbers appear in the
  RESEARCH_PACKET.md header.

### Neutral

- **SCHAFF turning-point speed** — STC's aggressive smoothing
  produces turning points earlier than most oscillators, but at
  the cost of more whipsaw in range-bound tapes. The OVERBOUGHT
  "&& falling" / OVERSOLD "&& rising" gating reduces whipsaw
  modestly but doesn't eliminate it. Users should interpret STC
  as a lead indicator, not a standalone signal.
- **CMO period** — Chande's original default is 9, but 14 and 20
  are also common. We ship 9 to match the canonical period and
  avoid parameter proliferation. If different periods are needed,
  compute flow is parametrisable via period arg on the helper.
- **Palette alias verification.** Bare `CMO`, `QSTICK`,
  `DISPARITY`, `BOP`, `SCHAFF` are all unbound upstream (verified
  via grep across `native/src/app.rs` for
  `show_cmo|show_qstick|show_disparity|show_bop|show_schaff` — no
  matches before Round 49 fields added). Bare names and
  disambiguated forms both kept as aliases.
- **All five surfaces use the same broker handler shape** stable
  since Round 22. All compute purely from the HP cache — no
  cross-symbol reads.
- **Field-name `_win` suffix** on native struct fields follows the
  Round 43–48 convention.

### Paid-API gap (for later revisit)

Same as ADR-158. Remaining gaps are data-access-gated (intraday
session-specific VWAP, Level-2 order book depth, options IV
surfaces, corporate actions feeds, realised-variance matrices,
insider transactions). No Round 49 surface needed any of these;
all compute from the daily HP cache.

## Verification

- `cargo test -p typhoon-engine --lib` — 1210 passing (up from
  1200 after Round 48, +10 new: 5 roundtrip + 5 compute_oscillating).
- `cargo build -p typhoon-engine` — clean.
- `cargo build -p typhoon-native` — clean (2m 47s).
- CMO/QSTICK/DISPARITY/BOP/SCHAFF compute_oscillating use the
  ±0.5% oscillating fixture (150 bars). Each asserts label belongs
  to its regime set, scalars are finite when label is not
  INSUFFICIENT_DATA, and axis-specific invariants:
  CMO cmo_value finite and in [-100, +100], period=9;
  QSTICK qstick_value/qstick_prev finite, period=14;
  DISPARITY disparity_value finite, sma_value finite and >0,
  period=14;
  BOP bop_value finite and in [-1, +1], raw_bop finite, period=14;
  SCHAFF stc_value finite and in [0, 100], stc_prev finite,
  ema_fast=23, ema_slow=50, cycle=10.

## Packet envelope

After Round 49, single-symbol packet target envelope is **~75-147 KB**
(up from 74-146 in Round 48). Basket (10 symbols via BASKET) is
**~750-1470 KB** (up from 740-1460). Sub-block count grows 233 → 238.

Total HP-local research snapshot count after Round 49: **197**
(192 + 5). Total cross-symbol rank snapshots unchanged.
