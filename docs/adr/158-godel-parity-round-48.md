# ADR-158: Godel Parity Round 48 — EFI / EMV / NVI / PVI / COPPOCK

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-157
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 47 (ADR-156) shipped MASS/CHAIKOSC/KLINGER/STOCHRSI/AWESOME,
taking HP-local research surfaces to 187 and per-symbol sub-blocks
to 228. Continuing the standing "combing over for full Godel
research parity" directive, Round 48 closes five more canonical
volume-flow + long-term-momentum gaps still missing after Round 47.

1. **No Elder Force Index.** EFI (Alexander Elder, *Trading for
   a Living*, 1993) is the simplest volume-weighted momentum
   oscillator: `volume × (close − prev_close)`, smoothed by EMA13.
   Positive + rising EFI = active bullish buying pressure; negative
   + falling = active selling pressure; near-zero cross = momentum
   exhaustion. Header gives **efi_label** (STRONG_BULL >0 && rising
   && abs-norm > 5bp / BULL >0 / NEUTRAL / BEAR <0 / STRONG_BEAR
   <0 && falling && abs-norm > 5bp / INSUFFICIENT_DATA). Distinct
   from MACD/PPO (price-only) and KLINGER (two-EMA spread on a
   volume-force construct): EFI is a single-EMA smoothing of a
   raw force-per-bar and is designed to be read in context with
   a long-term trend — Elder's specific prescription was to use
   EFI's zero-line cross to time entries in the direction of the
   dominant weekly trend.

2. **No Ease of Movement.** EMV (Richard Arms, 1980s) combines
   range and volume into a single low-effort-rally detector.
   For each bar: `midpoint_change = (H+L)/2 − (H_prev+L_prev)/2`;
   `box_ratio = (volume/scale) / (H − L)`; raw EMV = midpoint_change
   / box_ratio; smooth with SMA14. High positive = price moved up
   easily with low volume relative to range (low-effort rally,
   bullish); high negative = dropped easily on low volume (low-
   effort sell, bearish). Header gives **emv_label** (STRONG_BULL
   >0 && norm>1% / BULL >0 / NEUTRAL / BEAR <0 / STRONG_BEAR <0
   && norm<−1% / INSUFFICIENT_DATA). Complements CHAIKOSC (A/D-
   derivative) and KLINGER (volume-force oscillator): EMV specifically
   measures whether volume is efficiently producing price movement,
   which neither CHAIKOSC nor KLINGER quantify directly.

3. **No Negative Volume Index.** NVI (Paul Dysart, 1930s;
   popularised by Norman Fosback, *Stock Market Logic*, 1976)
   accumulates percentage-change only when *today's volume is
   LOWER than yesterday's*. Fosback's 1-year EMA rule: **NVI above
   its 1-year EMA historically signals 95%+ odds of a bull market**
   ("smart money" is accumulating quietly on low-volume sessions).
   Header gives **nvi_label** (BULL nvi>signal && spread>0.25% /
   NEUTRAL / BEAR nvi<signal && spread<−0.25% / INSUFFICIENT_DATA).
   First "low-volume cohort" surface we ship — all prior volume
   readings (OBV, A/D, CHAIKOSC, KLINGER) weight high-volume days
   MORE, not less. NVI flips the weighting.

4. **No Positive Volume Index.** PVI (Dysart/Fosback, companion
   to NVI) mirrors NVI: updates only on *UP-volume days*. Fosback
   treats PVI as the crowd-following surface — PVI above its
   1-year EMA means the crowd is actively buying and prices are
   rising on high volume (sentiment confirmation). PVI *below* its
   1-year EMA is the more diagnostic signal: it says the crowd
   bought but the rally failed, implying smart money distributed.
   Header gives **pvi_label** (BULL / NEUTRAL / BEAR /
   INSUFFICIENT_DATA) using same thresholds as NVI. Ship NVI and
   PVI together since Fosback's interpretation system only works
   when both are read side-by-side: NVI up AND PVI up = strongest
   bull; NVI up AND PVI down = smart money accumulating while crowd
   sells; etc.

5. **No Coppock Curve.** COPPOCK (E.S.C. Coppock, *Barron's*,
   October 1962) is the longest-lookback momentum oscillator in
   canonical TA. Formula: `WMA(10, ROC(14) + ROC(11))`. Coppock
   designed it on monthly bars for major equity indices and
   described it as a "guide" for long-term investors: when the
   curve is negative and crosses back above zero, buy; when
   positive and crosses back below, sell. Historically fires very
   rarely — typically 3-5 buy signals per decade on the S&P 500 —
   but with a remarkable hit rate (1974, 1982, 2009, etc. all
   Coppock buys). Header gives **coppock_label** (BUY_CROSS
   prev≤0 && now>0 / BULL >0 / NEUTRAL / BEAR <0 / SELL_CROSS
   prev≥0 && now<0 / INSUFFICIENT_DATA). Our implementation works
   on whatever bar granularity the HP cache holds (currently daily)
   — the cross logic is the same, the signal cadence just scales
   with bar size. First surface we ship that carries an explicit
   **cross-event label** distinct from direction-state labels.

Round 48 ships these five surfaces as ADR-158. Same additive envelope
as Rounds 5–47: no new fetchers, no cross-symbol scans, no new
external API dependencies. All five compute from the trailing HP
cache.

## Decision

Ship Round 48 as a five-surface additive bundle using schema v49
layered on v48:

| Surface    | Table                 | Purpose                                                                  |
|------------|-----------------------|--------------------------------------------------------------------------|
| EFI        | `research_efi`        | Elder Force Index (EMA13 of volume × Δclose)                             |
| EMV        | `research_emv`        | Arms Ease of Movement (SMA14 of midpoint-change / box-ratio)             |
| NVI        | `research_nvi`        | Dysart/Fosback Negative Volume Index (low-volume accumulator)            |
| PVI        | `research_pvi`        | Dysart/Fosback Positive Volume Index (high-volume accumulator)           |
| COPPOCK    | `research_coppock`    | E.S.C. Coppock Curve (WMA10 of ROC14+ROC11, buy/sell cross-zero)         |

Each table follows the established JSON-blob-per-symbol shape:

```sql
CREATE TABLE research_<name> (
    symbol TEXT PRIMARY KEY,
    snapshot_json TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_research_<name>_updated ON research_<name>(updated_at);
```

Each snapshot carries a regime `label` field. EFI/EMV use the
5-bucket STRONG_BULL/BULL/NEUTRAL/BEAR/STRONG_BEAR scheme.
NVI/PVI use the 3-bucket BULL/NEUTRAL/BEAR scheme (the
signal-line test only supports one threshold). COPPOCK uses
5 buckets including explicit BUY_CROSS / SELL_CROSS events.
All surfaces emit INSUFFICIENT_DATA when the bar count is too
low for the smoother chain to stabilise.

## Consequences

### Positive

- **First simple volume-weighted momentum.** EFI is the minimal
  force-based oscillator: one multiply and one EMA. Distinct from
  KLINGER (which normalises volume force against running range)
  and OBV (which is a raw cumulator). EFI's zero-line cross is
  Elder's canonical entry signal.
- **First "low-effort vs high-effort" volume surface.** EMV
  directly measures how much price movement volume produced.
  Complements OBV (raw flow), CHAIKOSC (flow-derivative), and
  KLINGER (flow-spread) — none of which answer the "was volume
  efficient?" question.
- **First "smart-money vs crowd" pair.** NVI + PVI together
  form the classic Fosback sentiment system. Fosback's 1978 claim
  (95% bull-market probability when NVI is above its 1-yr EMA) is
  still cited in modern literature. Neither surface is useful
  alone — we ship both so NVI/PVI can be cross-referenced.
- **First true long-term momentum guide.** COPPOCK's 14/11 ROC
  lookback + 10 WMA smoother produces a signal that fires ~3-5×
  per decade on major indices. The BUY_CROSS event is the only
  explicit cross-zero label in the research surface set — all
  prior "STRONG_BULL"/"BULL" labels are state-based, not transition
  events.
- **No new external dependencies, no fetcher expansion.** Pure
  compute on the HP cache — same additive envelope as Rounds 26–47.

### Negative / Risks

- **Schema migration.** `create_research_tables_v49` is additive
  over v48; peers on v48 who receive v49 rows via LAN sync will
  create the 5 new tables via the existing create-before-insert
  path. No back-compat break.
- **NVI/PVI signal EMA clamp on short tapes.** Fosback's spec
  calls for a 255-bar (1-year) EMA of the NVI line. Our compute
  gracefully scales the EMA period down to `min(255, len/2)` when
  fewer bars are available, and reports the effective period in
  `signal_period`. On a 252-bar daily tape this hits exactly 255
  (or close to it); on shorter tapes, the label still fires but
  with less statistical weight. Documented; min_bars=30.
- **EMV zero-range guard.** `box_ratio = (volume/scale) / (H − L)`
  divides by H-L, which is zero for flat-bar "doji" sessions.
  We guard with epsilon (1e-9) and treat the bar's raw_emv as
  effectively zero in that case — doesn't contribute to the SMA
  but doesn't crash either. Documented.
- **COPPOCK bar-granularity sensitivity.** Coppock's original
  1962 rule-of-thumb was designed for monthly bars. On daily
  bars (our HP default), the 14/11 ROC lookback covers ~2-3
  weeks, not ~2-3 months. BUY_CROSS events fire more often as a
  result. Users should interpret cross signals relative to the
  tape granularity. Documented.
- **EFI raw-value scale.** `volume × Δclose` produces huge
  absolute numbers (billions for US mega-caps). We normalise for
  label-threshold purposes against `|close| × volume` to get a
  dimensionless regime score. Raw value is preserved in the
  `raw_efi` and `efi_value` fields for the UI/packet.
- **Packet weight.** EFI adds ~240 bytes, EMV ~210, NVI ~190,
  PVI ~190, COPPOCK ~230. Total Round 48 addition: ~1.06 KB/symbol.
  Updated envelope numbers appear in the RESEARCH_PACKET.md header.

### Neutral

- **NVI/PVI 3-bucket scheme** uses only one threshold (spread
  vs signal EMA) because the Fosback system is binary (above/
  below 1-yr EMA). Adding a second threshold would over-fit the
  spec. We stick to Fosback's original binary test.
- **COPPOCK cross-event labels** (BUY_CROSS/SELL_CROSS) are
  transient — they fire only on the bar immediately after the
  sign change. State-based BULL/BEAR labels handle the continuing
  regime. This matches Coppock's original "guide" framing where
  the cross is the decision point and subsequent bars just
  confirm.
- **Palette alias verification.** Bare `EFI`, `EMV`, `NVI`,
  `PVI`, `COPPOCK` are all unbound upstream (verified via grep
  across `native/src/app.rs` for
  `show_efi|show_emv|show_nvi|show_pvi|show_coppock` — no
  matches before Round 48 fields added). Bare names and
  disambiguated forms both kept as aliases.
- **All five surfaces use the same broker handler shape** stable
  since Round 22. All compute purely from the HP cache — no
  cross-symbol reads.
- **Field-name `_win` suffix** on native struct fields follows the
  Round 43–47 convention to avoid colliding with chart-overlay
  booleans in the same `TyphoonApp` struct, even where no
  collision currently exists — consistency over case-by-case.

### Paid-API gap (for later revisit)

Same as ADR-157. Remaining gaps are data-access-gated (intraday
session-specific VWAP, Level-2 order book depth, options IV
surfaces, corporate actions feeds, realised-variance matrices,
insider transactions). No Round 48 surface needed any of these;
all compute from the daily HP cache.

## Verification

- `cargo test -p typhoon-engine --lib` — 1200 passing (up from
  1190 after Round 47 + 4 AI-sessions tests, +10 new: 5 roundtrip
  + 5 compute_oscillating).
- `cargo build -p typhoon-engine` — clean.
- `cargo build -p typhoon-native` — clean.
- EFI/EMV/NVI/PVI/COPPOCK compute_oscillating use the ±0.5%
  oscillating fixture (150 bars). Each asserts label belongs to
  its regime set, scalars are finite when label is not
  INSUFFICIENT_DATA, and axis-specific invariants:
  EFI raw_efi/efi_value/efi_prev finite, ema_period=13;
  EMV raw_emv/emv_value finite, sma_period=14, volume_scale>0;
  NVI nvi_value/signal_value finite and >0, signal_period≥3;
  PVI pvi_value/signal_value finite and >0, signal_period≥3;
  COPPOCK coppock_value/coppock_prev finite, roc_fast=11,
  roc_slow=14, wma_period=10.

## Packet envelope

After Round 48, single-symbol packet target envelope is **~74-146 KB**
(up from 73-145 in Round 47). Basket (10 symbols via BASKET) is
**~740-1460 KB** (up from 730-1450). Sub-block count grows 228 → 233.

Total HP-local research snapshot count after Round 48: **192**
(187 + 5). Total cross-symbol rank snapshots unchanged.
