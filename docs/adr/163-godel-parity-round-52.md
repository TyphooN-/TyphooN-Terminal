# ADR-163: Godel Parity Round 52 — ALMA / ZLEMA / ELDERRAY / TSF / RVI

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-162
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 51 (ADR-161) shipped DEMA/TEMA/LINREG/PIVOTS/HEIKIN, taking
HP-local research surfaces to 207 and per-symbol sub-blocks to 248.
ADR-162 then added the cross-client AI response cache as an
infrastructure round. Round 52 resumes the additive indicator cadence
with five more canonical surfaces that still had no stand-alone
snapshot after 51 rounds of work. Each has a sharp domain purpose
distinct from what is already shipped.

1. **No Arnaud Legoux Moving Average (ALMA) snapshot.** Legoux &
   Kouzoubov's 2009 ALMA applies a Gaussian-kernel weighting `w[i] =
   exp(-0.5 · ((i − m)/s)²)` with `m = 0.85·(N−1)`, `s = N/6` across
   the length-N window, length 20. The Gaussian kernel is the first
   **bell-shaped** weighting in the repo: EMA weights decay
   exponentially, WMA/HMA weight linearly, SMA weights equally, and
   ALMA peaks in the middle-to-recent third of the window and decays
   on both sides. First Gaussian-MA surface we ship. Distinct from
   HMA (ADR-148, sqrt-WMA lag reduction), DEMA/TEMA (ADR-161,
   algebraic lag subtraction), KAMA (ADR-151, efficiency-ratio
   adaptive), MCGD (ADR-160, feedback adaptive): ALMA reduces lag by
   **peak-biased kernel placement** (offset=0.85 pulls the weight peak
   toward the right/recent edge) while the Gaussian shape suppresses
   whipsaw by down-weighting the single most-recent bar relative to a
   purely-recent-biased weighting. Header gives **alma_label**
   (STRONG_BULL for >+2% deviation / BULL / NEUTRAL / BEAR /
   STRONG_BEAR for <−2% / INSUFFICIENT_DATA for n<21).

2. **No Zero-Lag EMA (ZLEMA) snapshot.** Ehlers's 2002 ZLEMA applies
   a first-order de-lagging transform `price'[i] = 2·price[i] −
   price[i − lag]` (lag = (N−1)/2 = 9 for N=20) and then runs a
   standard EMA(20) over the de-lagged series. Distinct from DEMA
   (ADR-161, **second-order algebraic** lag subtraction on the EMA
   chain) — ZLEMA de-lags the **input series first** then applies a
   single EMA, whereas DEMA applies two EMAs and subtracts. Both
   target lag reduction but via structurally different pathways:
   DEMA trades overshoot for more lag removal, ZLEMA trades slightly
   rougher response for less overshoot. Header gives **zlema_label**
   (STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR /
   INSUFFICIENT_DATA for n<31, where 31 = length + lag + 2).

3. **No Elder Ray (ELDERRAY) snapshot.** Alexander Elder's 1989
   Bull/Bear Power defines `bull_power = high − EMA(13)` and
   `bear_power = low − EMA(13)`. First **dual-channel** trend-
   intensity surface in the packet: unlike BOP (ADR-116, per-bar
   close-vs-range conviction) or Williams %R (ADR-153, N-bar close-
   in-range), ELDERRAY measures **how far buyers and sellers can push
   price away from a central EMA** on the same bar, using the high as
   the bull ceiling and the low as the bear floor. Classic Elder
   regime interpretation: `bull > 0 && bear > 0 && EMA rising` =
   strong uptrend (both channels positive and trend intact);
   `bull < 0 && bear < 0 && EMA falling` = strong downtrend; mixed
   configurations indicate a regime transition. Header gives
   **elder_label** (STRONG_BULL / BULL / NEUTRAL / BEAR /
   STRONG_BEAR / INSUFFICIENT_DATA for n<15).

4. **No Time Series Forecast (TSF) snapshot.** TSF extends the
   existing LINREG (ADR-161, OLS fit at `t = N−1`) with a **forward
   projection** to `t = N` — i.e., the next bar's expected value under
   the regression hypothesis. Where LINREG answers "what is the fair
   value right now," TSF answers "what does the fit *imply* for the
   next bar." Adds four-state LEADING/LAGGING classification:
   `LEADING_UP` when forecast > last_close and slope > 0 (fit says
   price has further to rise), `LAGGING_UP` when forecast > last_close
   but slope < 0 (price is ahead of the fit's turn), `LEADING_DOWN` /
   `LAGGING_DOWN` symmetrically, `FLAT` when `|forecast − last| / last
   < 0.1%`. Complements LINREG which reports current fit and R² but
   makes no forward statement. Header gives **tsf_label**
   (LEADING_UP / LAGGING_UP / FLAT / LAGGING_DOWN / LEADING_DOWN /
   INSUFFICIENT_DATA for n<20).

5. **No Relative Vigor Index (RVI) snapshot.** Ehlers's 2002 Relative
   Vigor Index computes `rvi = SMA₁₀(triangular(close−open)) /
   SMA₁₀(triangular(high−low))` where triangular weighting is
   `x[i] + 2·x[i−1] + 2·x[i−2] + x[i−3]`, with a 4-bar triangular
   signal line `(rvi + 2·rvi[-1] + 2·rvi[-2] + rvi[-3]) / 6`.
   Measures **aggregated closing conviction** — in a bull market
   close-open tends to be positive and so the numerator grows
   relative to the range denominator. Distinct from BOP (ADR-116,
   single-bar close-open/range with no smoothing), from Stochastic
   (ADR-160, close-in-range against low/high extremes rather than
   open), and from RSI-family oscillators (gain/loss based).
   Signal-line cross-over is the canonical trade signal: RVI crossing
   above signal from below = BULL_CROSS, crossing below = BEAR_CROSS.
   Header gives **rvi_label** (BULL_CROSS / BULL / NEUTRAL / BEAR /
   BEAR_CROSS / INSUFFICIENT_DATA for n<17, where 17 = length + 3 +
   4).

## Decision

Adopt the same additive schema-versioning pattern used in every prior
round:

- **Engine** (`engine/src/core/research.rs`): add
  `AlmaSnapshot / ZlemaSnapshot / ElderRaySnapshot / TsfSnapshot /
  RviSnapshot` structs, each with compute/upsert/get helpers;
  `create_research_tables_v53` wraps `_v52` and adds five new tables
  (`research_alma`, `research_zlema`, `research_elderray`,
  `research_tsf`, `research_rvi`). Tests: 5 roundtrip + 5
  compute_oscillating using the shared
  `synthetic_oscillating_bars_150()` fixture. 1251 tests pass (+10).
- **LAN sync** (`engine/src/core/lan_sync.rs`): whitelist the five
  table names in `SYNCABLE_TABLES`; add the five CREATE TABLE
  stanzas and the five `Some("updated_at")` timestamp-column entries.
- **Native** (`native/src/app.rs`): standard 9-section additive
  wiring: (1) 5 BrokerCmd variants, (2) 5 BrokerMsg variants, (3) 15
  struct fields (show/symbol/snapshot/loading × 5), (4) 15 default
  initialisers, (5) 5 compute-handler tokio tasks using
  `shared_cache_broker`, (6) 5 palette command aliases, (7) 5 research
  packet markdown emitters, (8) 5 egui::Window renderers each with
  Use-Chart / Load-Cached / Compute controls and a striped summary
  grid, (9) 5 BrokerMsg result handlers.
- **Documentation**: this ADR plus five new sub-blocks 2.247–2.251
  in `docs/RESEARCH_PACKET.md` (renumbering INGESTED 2.247 → 2.252 and
  Sector peer 2.248 → 2.253), and envelope updates from 77–149 KB to
  78–150 KB single-symbol and 740–1460 KB to 750–1470 KB basket.

### Palette aliases

- ALMA: `ALMA | ALMAFIT | ALMA_WIN | ARNAUD_LEGOUX | GAUSSIAN_MA`
- ZLEMA: `ZLEMA | ZLEMAFIT | ZLEMA_WIN | ZERO_LAG_EMA | EHLERS_ZLEMA`
- ELDERRAY: `ELDERRAY | ELDER_RAY | ELDERRAY_WIN | BULL_BEAR_POWER |
  ELDER_BULL_BEAR`
- TSF: `TSF | TSFFIT | TSF_WIN | TIME_SERIES_FORECAST |
  LINREG_FORECAST`
- RVI: `RVI | RVIFIT | RVI_WIN | RELATIVE_VIGOR | VIGOR_INDEX`

No bare-token collisions with existing chart overlays or toggles —
the `RVI` token was previously unclaimed (distinct from RSI), and
none of the five collide with an existing command.

## Consequences

### Positive

- **Gaussian-kernel MA family now present** (ALMA) — extends the
  lag-reduction axis with a peak-biased bell-shaped kernel, distinct
  from EMA (exponential), WMA/HMA (linear), SMA (flat), DEMA/TEMA
  (algebraic subtraction), KAMA (efficiency-adaptive), MCGD
  (feedback-adaptive), FRAMA (fractal-adaptive).
- **First de-lagged-input EMA** (ZLEMA) — Ehlers's de-lag-first-then-
  smooth pathway is structurally different from DEMA's smooth-then-
  subtract-lag approach, giving the AI a second-order lag-reduction
  surface to cross-check the first-order one.
- **First dual-channel trend intensity** (ELDERRAY) — Bull Power and
  Bear Power on the same bar relative to an EMA midline gives a
  regime-classification surface that BOP and Williams %R cannot.
- **Forward-projected fair-value** (TSF) — complements LINREG by
  answering not just "where is fair value" but "where will the fit
  say it is next bar," with leading/lagging classification that
  highlights when price is ahead of or behind the fit's own turn.
- **Aggregated closing conviction oscillator** (RVI) — Ehlers's
  vigor index fills the gap between BOP (unsmoothed single-bar) and
  Stochastic (close-in-range vs H/L) with a triangular-smoothed
  close-vs-open/range ratio and cross-over signal line.
- +10 engine tests (5 roundtrip + 5 compute_oscillating) maintaining
  the property that every new surface has both persistence and
  compute-determinism coverage.

### Negative / Risks

- **Min-bar warmups vary across the five.** ZLEMA needs ≥31 bars
  (length + lag + 2), while ELDERRAY only needs ≥15. Symbols with
  shallow HP cache will report INSUFFICIENT_DATA for ZLEMA/ALMA
  before ELDERRAY/TSF/RVI. The label/note pair makes this explicit
  to the AI consumer.
- **TSF forward projection is one bar only.** Multi-bar projections
  would compound the slope error; sticking to t = N (one bar ahead)
  matches the canonical TSF definition and keeps the R² context
  meaningful as a quality flag for the single-step forecast.
- **RVI cross detection requires ≥17 bars.** On a fresh IPO the
  signal-line computation needs four RVI samples, which itself needs
  ten SMA samples plus three bars of triangular lookback. Documented
  in the note field on INSUFFICIENT_DATA paths.
- **ALMA offset = 0.85 and sigma = 6 are the canonical defaults** and
  are not user-configurable from the packet. Different (offset,
  sigma) pairs produce noticeably different shapes; the exposed
  fields `offset` and `sigma` let the AI read the parameters rather
  than guessing from the name.

### Neutral

- No new API dependencies; all five surfaces reuse the existing
  `research_historical_price` HP cache. This keeps the round fully
  free-API-compatible per the standing godel-parity directive.
- `GAUSSIAN_MA` is a new palette token and does not collide with
  anything. `LINREG_FORECAST` clarifies the TSF/LINREG relationship
  for users who learned LINREG first.

### Paid-API gap

None introduced in this round. All five surfaces are HP-derived and
work entirely from the existing free-data cache.

## Verification

- `cargo test -p typhoon-engine --lib`: 1251 tests pass (+10 from
  1241).
- `cargo build -p typhoon-native`: clean build, no new warnings.
- `docs/RESEARCH_PACKET.md`: 253 sub-blocks total (up from 248);
  envelope updated to 78–150 KB single-symbol and 750–1470 KB basket.

## Packet envelope delta

| Surface | Field count | Approx bytes when populated | Free / Paid |
|---|---|---|---|
| ALMA | 11 | ~230 | Free (HP cache) |
| ZLEMA | 11 | ~230 | Free (HP cache) |
| ELDERRAY | 13 | ~270 | Free (HP cache) |
| TSF | 13 | ~280 | Free (HP cache) |
| RVI | 11 | ~230 | Free (HP cache) |
| **Round 52 total** | **59 fields** | **≈1.24 KB** | **Free** |

Envelope: 77–149 KB → 78–150 KB single-symbol; 740–1460 KB →
750–1470 KB for the canonical 10-symbol basket.
