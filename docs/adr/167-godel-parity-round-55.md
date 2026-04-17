# ADR-167: Godel Parity Round 55 — SMMA / ALLIGATOR / CRSI / SEB / IMI

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-166
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 54 (ADR-165) shipped AC/CHVOL/BBWIDTH/ELDERIMP/RMI; ADR-166
shipped the Options Expiration Calendar. Round 55 continues the
additive indicator cadence with five more canonical surfaces that
fill distinct coverage holes in the smoothing / chart-pattern /
composite / channel / bar-local-momentum axes.

1. **No Wilder Smoothed MA (SMMA / RMA) snapshot.** Wilder's
   SMMA is the recursive `SMMA_t = (SMMA_{t−1}·(N−1) + price_t)/N`
   — equivalent to EMA with `α = 1/N` (vs classical EMA's
   `α = 2/(N+1)`). With length=14 it decays much more slowly than
   EMA₁₄ and underpins ATR, RSI's average gain/loss, and Williams's
   Alligator. Distinct from SMA (ADR-108), EMA (ADR-108), DEMA
   (ADR-144), TEMA (ADR-146), KAMA (ADR-148), FRAMA (ADR-149), HMA
   (ADR-150), TRIMA (ADR-164), T3 (ADR-164), VIDYA (ADR-164), and
   ZLEMA — SMMA is the one "slow-decay Wilder" recursion not yet
   surfaced on its own. Header gives **smma_label**
   (STRONG_BULL ≥ +2% / BULL > 0 / NEUTRAL / BEAR < 0 / STRONG_BEAR
   ≤ −2% / INSUFFICIENT_DATA for n<16) derived from close-vs-SMMA
   deviation percentage.

2. **No Bill Williams Alligator snapshot.** The Alligator is three
   displaced SMMAs of the median price (H+L)/2:
   `jaw = SMMA₁₃ shifted +8`, `teeth = SMMA₈ shifted +5`,
   `lips = SMMA₅ shifted +3`. The ordering and spread of the three
   lines encode Williams's four regime states — SLEEPING when
   tightly intertwined, EATING_UP when `lips > teeth > jaw` and
   spread opens, EATING_DOWN when reversed, AWAKENING when
   crossing. Every chart-pattern system in forex/crypto uses some
   variant of this — shipping it rounds out the Williams Chaos
   Theory trio alongside AO (ADR-156) and AC (ADR-165). Distinct
   from Fractals (peak/trough markers, separate surface). Header
   gives **alligator_label** (EATING_UP / EATING_DOWN / AWAKENING /
   SLEEPING / INSUFFICIENT_DATA for n<23).

3. **No Connors RSI (CRSI) snapshot.** Larry Connors's CRSI is a
   composite: `CRSI = (RSI₃(close) + RSI₂(streak) +
   percent_rank(ROC₁, 100)) / 3`, where `streak` is the signed
   count of consecutive up/down days. The three components
   together produce a very reactive mean-reversion oscillator —
   canonical entries at >90 (short) / <10 (long). Distinct from
   RSI (ADR-108, single-length Wilder), RMI (ADR-165, RSI on
   momentum series), StochRSI (ADR-141, stochastic-of-RSI), and
   Fisher RSI — CRSI's contribution is the explicit streak
   component, which captures regime persistence that none of the
   other RSI variants do. Header gives **crsi_label** (OVERBOUGHT
   ≥75 / BULLISH ≥60 / NEUTRAL / BEARISH ≤40 / OVERSOLD ≤25 /
   INSUFFICIENT_DATA for n<108).

4. **No Standard Error Bands (SEB) snapshot.** Tim Tillson / Don
   Fishback's SEB channels are `center ± k·SE` where center is the
   linear-regression fitted value at `t = N − 1` and SE is the
   residual standard error `sqrt(Σ(y − ŷ)² / (N − 2))`. Narrower
   than Bollinger (ADR-108) when the price fit is good (low
   residual variance) and wider when price is noisy around the
   trend — gives a *trend-aware* channel surface that Bollinger
   (stddev around a flat mean) doesn't. Distinct from Keltner
   (ADR-128, ATR-based channels), Donchian (ADR-147, max-min
   channels), Linear Regression Channel (LRC — stddev-of-price
   around the regression, not residuals), and TSF (ADR-139,
   single regression-endpoint value). SEB is the one channel
   indicator tied to regression-residual variance specifically.
   Header gives **seb_label** (ABOVE_BAND / UPPER_HALF / NEUTRAL /
   LOWER_HALF / BELOW_BAND / INSUFFICIENT_DATA for n<22).

5. **No Intraday Momentum Index (IMI) snapshot.** Tushar Chande's
   IMI is an RSI-style ratio computed from *per-bar* `close − open`
   rather than inter-bar `close − close[-1]`:
   `IMI = 100 · ΣUp / (ΣUp + ΣDown)` over N bars, where
   `Up = max(close − open, 0)`, `Down = max(open − close, 0)`.
   Measures buying vs selling pressure *within* each bar,
   complementing RSI's inter-bar view. Distinct from RSI (inter-bar
   close diff), CMO (Chande — sum of ups/downs, inter-bar), QSTICK
   (ADR-127, EMA of close−open, not RSI-style), and BOP (single-bar
   scaled close-open, not aggregated). Every day-trading desk
   watches IMI alongside RSI precisely because they decouple
   — IMI can print OVERBOUGHT while RSI prints NEUTRAL when the
   market closes near the high every day without inter-bar
   follow-through. Header gives **imi_label** (OVERBOUGHT ≥70 /
   BULL ≥60 / NEUTRAL / BEAR ≤40 / OVERSOLD ≤30 / INSUFFICIENT_DATA
   for n<16).

## Decision

Adopt the same additive schema-versioning pattern used in every prior
round:

- **Engine** (`engine/src/core/research.rs`): add
  `SmmaSnapshot / AlligatorSnapshot / CrsiSnapshot / SebSnapshot
  / ImiSnapshot` structs, each with compute/upsert/get helpers;
  `create_research_tables_v57` wraps `_v56` and adds five new
  tables (`research_smma`, `research_alligator`, `research_crsi`,
  `research_seb`, `research_imi`). Tests: 5 roundtrip + 5
  compute_oscillating using the shared
  `synthetic_oscillating_bars_150()` fixture. 1286 tests pass
  (+10 from 1276).
- **LAN sync** (`engine/src/core/lan_sync.rs`): whitelist the five
  table names in `SYNCABLE_TABLES`; add the five CREATE TABLE
  stanzas and the five `Some("updated_at")` timestamp-column
  entries.
- **Native** (`native/src/app.rs`): standard 9-section additive
  wiring: (1) 5 BrokerCmd variants, (2) 5 BrokerMsg variants, (3)
  20 struct fields (show/symbol/snapshot/loading × 5), (4) 20
  default initialisers, (5) 5 compute-handler tokio tasks using
  `shared_cache_broker`, (6) 5 palette command aliases, (7) 5
  research packet markdown emitters, (8) 5 egui::Window renderers
  each with Use-Chart / Load-Cached / Compute controls and a
  striped summary grid, (9) 5 BrokerMsg result handlers.
- **Documentation**: this ADR plus five new sub-blocks
  2.265–2.269 in `docs/RESEARCH_PACKET.md` (renumbering INGESTED
  2.265 → 2.270 and Sector peer 2.266 → 2.271), and envelope
  updates to account for the new fields.

### Palette aliases

- SMMA: `SMMA | SMMAFIT | SMMA_WIN | WILDER_MA | WILDER_SMMA |
  RMA | SMOOTHED_MA`
- ALLIGATOR: `ALLIGATOR | ALLIG | GATOR | ALLIGATOR_WIN |
  WILLIAMS_ALLIGATOR | BILL_WILLIAMS_ALLIGATOR`
- CRSI: `CRSI | CRSIFIT | CRSI_WIN | CONNORS_RSI | CONNORSRSI`
- SEB: `SEB | SEBFIT | SEB_WIN | STDERR_BANDS |
  STANDARD_ERROR_BANDS | SE_BANDS`
- IMI: `IMI | IMIFIT | IMI_WIN | INTRADAY_MOMENTUM_INDEX |
  CHANDE_IMI`

No remaining collisions with existing palette tokens. `RMA`
distinguishes SMMA's alias set from SMA (ADR-108). `GATOR`
anticipates a future Gator Oscillator surface (difference of
Alligator lines) without colliding — Gator Oscillator itself
would require this Alligator compute as a prerequisite. `CRSI`
avoids `RSI`, `STOCHRSI`, `MRSI`. `SEB` / `SE_BANDS` avoids
Keltner and Bollinger alias space. `IMI` / `INTRADAY_MOMENTUM_INDEX`
avoids RSI/CMO alias space.

## Consequences

### Positive

- **Slow-decay Wilder recursion added** (SMMA) — completes the
  moving-average family on the Wilder-style side, distinct from
  the N+1-decay EMA family. Underpins Alligator and every
  Wilder-rooted oscillator (ATR, RSI, RMI).
- **Chart-pattern regime surface added** (ALLIGATOR) — answers
  the canonical Williams question "is the alligator sleeping,
  eating up, eating down, or awakening?" Complements AO and
  AC as the third leg of the Williams Chaos Theory trio.
- **Composite RSI with streak component added** (CRSI) — the
  one RSI variant that explicitly encodes consecutive up/down
  day persistence. Used heavily in Connors's short-term
  mean-reversion systems.
- **Regression-residual channel added** (SEB) — a trend-aware
  channel that contracts when price fits the regression well
  and expands when it doesn't. Distinct from the stddev-around-
  flat-mean (Bollinger) and ATR-around-EMA (Keltner) families.
- **Bar-local momentum index added** (IMI) — the one momentum
  oscillator built from per-bar close-open rather than
  inter-bar close-close. Captures intraday buying/selling
  pressure regimes that RSI and CMO mask.
- +10 engine tests (5 roundtrip + 5 compute_oscillating)
  maintaining the property that every new surface has both
  persistence and compute-determinism coverage.

### Negative / Risks

- **Alligator warm-up is 23 bars.** The longest line (jaw =
  SMMA₁₃ shifted +8) needs at least 22 bars for the current
  value and one more for the prior-bar value. First-run HP
  caches below this threshold produce INSUFFICIENT_DATA, which
  is surfaced via the note field.
- **CRSI warm-up is 108 bars.** The 100-bar percent-rank
  window dominates. This is the highest-warm-up surface in
  this round; on freshly-cached symbols the user should run
  HP_BACKFILL or similar before CRSI will compute.
- **SEB middle is regression-endpoint, not a mean.** This
  surprises users expecting Bollinger semantics. Documented
  in the help text, and the label system stays intuitive
  (ABOVE_BAND / BELOW_BAND extremes + UPPER/LOWER halves).
- **IMI requires reliable open/close data.** On aggregated
  sources where open is synthetic (previous close), IMI
  degenerates toward RSI. Users working with such sources
  should prefer CMO or RSI over IMI.
- **Alligator SLEEPING threshold is heuristic.** We use 0.15%
  spread-vs-close as the sleeping/awake boundary. Users on
  very-low-vol instruments (FX, bond futures) may find the
  threshold too strict; users on high-vol instruments (crypto,
  small caps) may find it too loose. Deferred to per-symbol
  tuning in a future round.

### Neutral

- No new API dependencies. All five surfaces reuse the existing
  `research_historical_price` HP cache.
- Alligator landed here rather than as a standalone ADR because
  it is mechanically a product of SMMA (shipped in the same
  round) and contributes to the Williams trio alongside AO/AC
  already shipped in prior rounds. Bundling it with its SMMA
  prerequisite avoided an awkward two-round split.

### Paid-API gap

None introduced in this round. All five surfaces are HP-derived
and work entirely from the existing free-data cache.

## Verification

- `cargo test -p typhoon-engine --lib`: 1286 tests pass (+10 from
  1276).
- `cargo build -p typhoon-native`: clean build, no new warnings.
- `docs/RESEARCH_PACKET.md`: five new sub-blocks 2.265–2.269 added
  (INGESTED and Sector peer renumbered); envelope updated.

## Packet envelope delta

| Surface | Field count | Approx bytes when populated | Free / Paid |
|---|---|---|---|
| SMMA | 10 | ~230 | Free (HP cache) |
| ALLIGATOR | 13 | ~300 | Free (HP cache) |
| CRSI | 13 | ~290 | Free (HP cache) |
| SEB | 13 | ~300 | Free (HP cache) |
| IMI | 10 | ~230 | Free (HP cache) |
| **Round 55 total** | **59 fields** | **≈1.35 KB** | **Free** |

Envelope: 80–152 KB → 81–154 KB single-symbol; 770–1490 KB →
780–1510 KB for the canonical 10-symbol basket.
