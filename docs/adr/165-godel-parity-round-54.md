# ADR-165: Godel Parity Round 54 — AC / CHVOL / BBWIDTH / ELDERIMP / RMI

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-164
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 53 (ADR-164) shipped TRIMA/T3/VIDYA/SMI/PVT, taking HP-local
research surfaces to 217 and per-symbol sub-blocks to 258. Round 54
continues the additive indicator cadence with five more canonical
surfaces distinct from everything already shipped, each filling a
specific coverage hole in the oscillator/volatility/regime axes.

1. **No Accelerator Oscillator (AC) snapshot.** Bill Williams's AC is
   the first derivative of his Awesome Oscillator (AO): `AO = SMA₅
   (medprice) − SMA₃₄(medprice)`; `AC = AO − SMA₅(AO)`. Where AO
   measures momentum (5-34 median crossover), AC measures the *change*
   in momentum — it crosses zero before AO does, making it the
   earliest turning-point signal in the Williams Alligator /
   Chaos Theory toolkit. Distinct from AO (ADR-156, the underlying
   momentum) in the same way that MACD's histogram is distinct from
   MACD itself: AC is the second-derivative layer Williams designed
   to precede AO by 5 bars. Header gives **ac_label** (STRONG_BULL
   for AC>0 rising / BULL for AC>0 / NEUTRAL / BEAR for AC<0 /
   STRONG_BEAR for AC<0 falling / INSUFFICIENT_DATA for n<40).

2. **No Chaikin Volatility (CHVOL) snapshot.** Marc Chaikin's 1966
   volatility indicator computes
   `CHV = 100 · (EMA₁₀(H−L) − EMA₁₀(H−L)[−10]) / EMA₁₀(H−L)[−10]`
   — the 10-bar percentage rate-of-change of a 10-bar EMA of the
   daily high-low range. Distinct from ATR (ADR-113, exponential
   smoothing of true range), BBWIDTH (below, stddev-based), and
   Volatility Regime (ADR-117, realized-vol term structure). CHVOL
   is the one volatility surface in the packet that asks *"is range
   expansion accelerating or decelerating?"* — positive readings
   indicate range expansion over the last 10 bars; negative
   readings indicate contraction. Canonical thresholds ±10 separate
   EXPANDING / NEUTRAL / CONTRACTING. Header gives **chvol_label**
   (EXPANDING / NEUTRAL / CONTRACTING / INSUFFICIENT_DATA for n<25).

3. **No Bollinger Bandwidth (BBWIDTH) snapshot.** John Bollinger's
   Bandwidth is defined as `BBW = (upper − lower)/middle` using the
   standard SMA₂₀ ± 2σ bands. Low readings indicate a "squeeze"
   (pending volatility expansion); high readings indicate range
   expansion already underway. Distinct from BBSQUEEZE (ADR-127,
   which compares Bollinger Bandwidth to Keltner Channel width as
   a boolean squeeze trigger) — BBWIDTH is the *continuous* value
   plus a 125-bar percentile ranking, so the AI can see not just
   whether we're squeezing but how extreme the squeeze is on a
   0-100 scale. Header gives **bbw_label** (SQUEEZE for pct<10 /
   LOW for pct<30 / NORMAL / EXPANDED for pct>75 /
   INSUFFICIENT_DATA for n<20, with 125-bar window needed for
   percentile).

4. **No Elder Impulse System (ELDERIMP) snapshot.** Alexander
   Elder's 2002 Impulse System colour-codes bars using the sign
   agreement between a 13-EMA slope and the MACD histogram slope.
   GREEN when both rising (buy-side impulse, do not short); RED
   when both falling (sell-side impulse, do not long); BLUE when
   mixed or flat (no impulse, regime undefined). Distinct from
   Elder Ray (ADR-163, bull/bear power around a 13-EMA) — Elder
   Ray is the *oscillator*, Impulse System is the **regime filter**.
   Used together they are Elder's classic trade-filter combo.
   Header gives **impulse_label** (GREEN / RED / BLUE /
   INSUFFICIENT_DATA for n<35).

5. **No Relative Momentum Index (RMI) snapshot.** Roger Altman's
   1993 RMI is a RSI variant computed on the N-bar momentum series
   `close_t − close_{t−N}` rather than the 1-bar change. The
   result is Wilder-smoothed into a 0–100 oscillator that behaves
   like RSI but with smoother extremes during strong trends. With
   length=14 and momentum_length=5 as canonical defaults, RMI
   stays overbought longer in a trending market than RSI does
   (because the 5-bar momentum series has persistence that the
   1-bar diff series lacks). Distinct from RSI (1-bar diff),
   STOCHRSI (stochastic-of-RSI), CMO (Chande, sum-of-ups /
   sum-of-totals), and QSTICK (EMA of close-open). Header gives
   **rmi_label** (OVERBOUGHT for >70 / BULL for >55 / NEUTRAL /
   BEAR for <45 / OVERSOLD for <30 / INSUFFICIENT_DATA for
   n<length+momentum_length+1).

## Decision

Adopt the same additive schema-versioning pattern used in every prior
round:

- **Engine** (`engine/src/core/research.rs`): add
  `AcSnapshot / ChvolSnapshot / BbwidthSnapshot / ElderImpulseSnapshot
  / RmiSnapshot` structs, each with compute/upsert/get helpers;
  `create_research_tables_v55` wraps `_v54` and adds five new tables
  (`research_ac`, `research_chvol`, `research_bbwidth`,
  `research_elderimp`, `research_rmi`). Tests: 5 roundtrip + 5
  compute_oscillating using the shared
  `synthetic_oscillating_bars_150()` fixture. 1271 tests pass (+10).
- **LAN sync** (`engine/src/core/lan_sync.rs`): whitelist the five
  table names in `SYNCABLE_TABLES`; add the five CREATE TABLE
  stanzas and the five `Some("updated_at")` timestamp-column entries.
- **Native** (`native/src/app.rs`): standard 9-section additive
  wiring: (1) 5 BrokerCmd variants, (2) 5 BrokerMsg variants, (3) 20
  struct fields (show/symbol/snapshot/loading × 5), (4) 20 default
  initialisers, (5) 5 compute-handler tokio tasks using
  `shared_cache_broker`, (6) 5 palette command aliases, (7) 5
  research packet markdown emitters, (8) 5 egui::Window renderers
  each with Use-Chart / Load-Cached / Compute controls and a striped
  summary grid, (9) 5 BrokerMsg result handlers.
- **Documentation**: this ADR plus five new sub-blocks 2.257–2.261
  in `docs/RESEARCH_PACKET.md` (renumbering INGESTED 2.257 → 2.262
  and Sector peer 2.258 → 2.263), and envelope updates from
  79–151 KB to 80–152 KB single-symbol and 760–1480 KB to
  770–1490 KB basket.

### Palette aliases

- AC: `AC | ACFIT | AC_WIN | ACCELERATOR | ACCELERATOR_OSCILLATOR`
- CHVOL: `CHVOL | CHVOLFIT | CHVOL_WIN | CHAIKIN_VOLATILITY |
  CHAIKIN_VOL`
- BBWIDTH: `BBWFIT | BBW_WIN | BOLLINGER_WIDTH | BBW | BBWPCT`
  (note: `BBWIDTH` itself was already claimed by BBSQUEEZE, ADR-127,
  as a legacy alias)
- ELDERIMP: `ELDERIMP | ELDERIMPULSE | IMPULSE | IMPULSE_SYSTEM |
  ELDER_IMPULSE`
- RMI: `RMI | RMIFIT | RMI_WIN | RELATIVE_MOMENTUM | ALTMAN_RMI`

No remaining collisions with existing palette tokens.
`ACCELERATOR_OSCILLATOR` distinguishes AC from AO (ADR-156);
`IMPULSE_SYSTEM` distinguishes ELDERIMP from ELDERRAY (ADR-163);
`RELATIVE_MOMENTUM` distinguishes RMI from RSI.

## Consequences

### Positive

- **Second-derivative momentum surface added** (AC) — closes the
  Williams Chaos Theory trio of Alligator (ADR-151), AO (ADR-156),
  and now AC. Gives the AI the earliest momentum-turn signal in
  the Williams canon.
- **Rate-of-change volatility added** (CHVOL) — the one volatility
  surface that directly answers "is range expansion *accelerating*?"
  Complements ATR (level) and Volatility Regime (term structure).
- **Continuous Bollinger Bandwidth + percentile shipped**
  (BBWIDTH) — gives the AI the underlying continuous value that
  BBSQUEEZE's boolean trigger is based on, plus a 125-bar
  percentile ranking for regime context.
- **Elder regime filter shipped** (ELDERIMP) — completes the
  Elder triple-screen conceptual framework alongside Elder Ray
  (ADR-163) and generic MACD-based trend filters.
- **Momentum-series RSI variant added** (RMI) — rounds out the
  RSI-adjacent family (RSI, STOCHRSI, CMO, QSTICK) with the
  Altman momentum-series variant that behaves more smoothly in
  trending regimes.
- +10 engine tests (5 roundtrip + 5 compute_oscillating)
  maintaining the property that every new surface has both
  persistence and compute-determinism coverage.

### Negative / Risks

- **AC warm-up is 40 bars.** The nested SMA₃₄ then SMA₅ means
  the first 40 bars of a fresh HP cache produce
  INSUFFICIENT_DATA. This matches the existing AO warm-up and
  is documented.
- **CHVOL can produce extreme readings on low-volume symbols**
  where H−L swings wildly. The label is driven by percentage
  ROC, so a symbol with a tiny baseline range can produce
  large CHVOL values for small absolute moves. Users should
  cross-reference with ATR for an absolute-scale check.
- **BBWIDTH percentile requires 125 bars to be meaningful.**
  Below 125 bars the percentile is computed on whatever
  sample is available, which can bias the SQUEEZE/EXPANDED
  classification. The note field surfaces this when the
  window is short.
- **ELDERIMP BLUE label covers a wide regime.** Any time EMA
  slope and MACD hist slope disagree, or either is flat,
  the label is BLUE. This is by design (Elder uses BLUE as
  the "do either direction" permission), but interpretation
  requires the other two labels for context.
- **RMI is not a drop-in RSI replacement.** The 5-bar momentum
  series produces persistent overbought/oversold readings in
  trending regimes — a feature, not a bug, but users expecting
  RSI-like mean reversion should note the difference.

### Neutral

- No new API dependencies; all five surfaces reuse the existing
  `research_historical_price` HP cache. None require volume data
  beyond what HP already carries. This keeps the round fully
  free-API-compatible per the standing godel-parity directive.
- The `BBWIDTH` palette alias being already claimed by BBSQUEEZE
  forced us to use a slightly uglier alias set — this is a
  historical naming-collision cost, not a technical issue.

### Paid-API gap

None introduced in this round. All five surfaces are HP-derived
and work entirely from the existing free-data cache.

## Verification

- `cargo test -p typhoon-engine --lib`: 1271 tests pass (+10 from
  1261).
- `cargo build -p typhoon-native`: clean build, no new warnings.
- `docs/RESEARCH_PACKET.md`: 263 sub-blocks total (up from 258);
  envelope updated to 80–152 KB single-symbol and 770–1490 KB
  basket.

## Packet envelope delta

| Surface | Field count | Approx bytes when populated | Free / Paid |
|---|---|---|---|
| AC | 10 | ~230 | Free (HP cache) |
| CHVOL | 11 | ~240 | Free (HP cache) |
| BBWIDTH | 14 | ~300 | Free (HP cache) |
| ELDERIMP | 12 | ~260 | Free (HP cache) |
| RMI | 10 | ~220 | Free (HP cache) |
| **Round 54 total** | **57 fields** | **≈1.25 KB** | **Free** |

Envelope: 79–151 KB → 80–152 KB single-symbol; 760–1480 KB →
770–1490 KB for the canonical 10-symbol basket.
