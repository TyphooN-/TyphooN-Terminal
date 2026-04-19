# ADR-164: Godel Parity Round 53 — TRIMA / T3 / VIDYA / SMI / PVT

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-163
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| TRIMA | Canonical (all terminals) | Yes (`TRIMA`) | Yes | Yes | No (deferred — ADR-188) |
| T3 | Canonical (all terminals) | Yes (`T3`) | Yes | Yes | No (deferred — ADR-188) |
| VIDYA | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |
| SMI | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |
| PVT | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** canonical technical indicators (Triangular MA via TA-Lib `TRIMA`, Tillson T3 via `T3`, Chande VIDYA adaptive MA, Blau Stochastic Momentum Index, Price-Volume Trend).

## Context

Round 52 (ADR-163) shipped ALMA/ZLEMA/ELDERRAY/TSF/RVI, taking
HP-local research surfaces to 212 and per-symbol sub-blocks to 253.
Round 53 continues the additive indicator cadence with five more
canonical surfaces distinct from everything already shipped, each
filling a specific coverage hole in the MA/oscillator/volume axes.

1. **No Triangular Moving Average (TRIMA) snapshot.** TRIMA is
   defined as `TRIMA(N) = SMA(SMA(N/2 + 1), N/2 + 1)` — a double-SMA
   that produces a triangular-weighted central MA where the
   centremost bars get weight proportional to `(N/2 + 1)² / (N/2 +
   1)` and endpoints get the lowest weight. Distinct from every MA
   already shipped: SMA is flat-weighted, WMA/HMA are linear-
   weighted (increasing toward the recent edge), EMA decays
   exponentially toward older bars, ALMA (ADR-163) is Gaussian-
   kernel-peaked in the recent-to-middle third, DEMA/TEMA (ADR-161)
   subtract EMA-of-EMA lag terms algebraically, KAMA (ADR-151) /
   MCGD (ADR-160) / FRAMA / VIDYA (below) are adaptive. TRIMA is
   the only MA in the packet with a **central, symmetric triangular
   weighting profile** — ubiquitous in TA-Lib, Bloomberg, and
   TradingView as the default "smoothed SMA" option. Header gives
   **trima_label** (STRONG_BULL for >+2% deviation / BULL / NEUTRAL /
   BEAR / STRONG_BEAR for <−2% / INSUFFICIENT_DATA for n<31).

2. **No Tim Tillson T3 snapshot.** Tillson's 1998 T3 MA composes
   six iterative EMAs with a volume factor `v` (default 0.7) and
   combines them with the coefficients
   `c1 = −v³; c2 = 3v² + 3v³; c3 = −6v² − 3v − 3v³; c4 = 1 + 3v +
   v³ + 3v²`; then `T3 = c1·e6 + c2·e5 + c3·e4 + c4·e3`. The `v`
   parameter generalises the Mulloy lag-reduction family: `v = 0`
   recovers a single EMA(N) at `c4`, `v = 1` produces the strongest
   lag reduction (with the most overshoot), and `v = 0.7` is the
   canonical balanced default. Distinct from DEMA (second-order
   algebraic subtraction) and TEMA (third-order) — T3 is the full
   six-EMA composite that smooths *and* de-lags simultaneously, at
   the cost of longer warm-up. Header gives **t3_label**
   (STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR /
   INSUFFICIENT_DATA for n<24).

3. **No Chande VIDYA snapshot.** Tushar Chande's 1992 Variable
   Index Dynamic Average computes an EMA-like recursion with an
   **adaptive alpha driven by CMO magnitude**:
   `α_t = (2 / (N+1)) · |CMO(9)_t| / 100;
   VIDYA_t = α_t · price_t + (1 − α_t) · VIDYA_{t−1}`.
   When momentum is strong (CMO large in magnitude) alpha grows
   toward the EMA baseline; when the market is range-bound (CMO
   small) alpha shrinks and VIDYA effectively freezes near its
   prior value. Distinct from the other adaptive MAs already
   shipped — KAMA (ADR-151) scales alpha by an **efficiency ratio**
   (net change over path length); MCGD (ADR-160) uses a **feedback
   loop on price ratio**; FRAMA scales alpha by the **fractal
   dimension**. VIDYA rounds out the adaptive family with a
   momentum-magnitude scaling. Header gives **vidya_label**
   (STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR /
   INSUFFICIENT_DATA for n<31).

4. **No Stochastic Momentum Index (SMI) snapshot.** William Blau's
   1993 SMI computes
   `H = max(high, N); L = min(low, N); mid = (H+L)/2;
   numerator = double-EMA(close − mid, smooth);
   denominator = double-EMA((H−L)/2, smooth);
   SMI = 100 · numerator / denominator ∈ [−100, 100];
   signal = EMA(SMI, signal_length)`.
   Measures close position relative to the *midpoint* of the N-bar
   range rather than the low (as raw stochastic does), then applies
   double-EMA smoothing. Distinct from STOCH (ADR-160, raw 0–100
   stochastic anchored at L_min) and STOCHRSI (stochastic-of-RSI),
   and from RVI (ADR-163, aggregated close-open conviction).
   Overbought/oversold thresholds ±40; signal-line crossover is the
   canonical trade signal. Header gives **smi_label**
   (OVERBOUGHT for >+40 / BULL_CROSS / BULL / NEUTRAL / BEAR /
   BEAR_CROSS / OVERSOLD for <−40 / INSUFFICIENT_DATA for n<21).

5. **No Price Volume Trend (PVT) snapshot.** Dysart/Lowry's 1966
   PVT is defined as
   `PVT_t = PVT_{t−1} + volume_t · (close_t − close_{t−1}) /
   close_{t−1}`. A cumulative running sum where each bar's volume
   is scaled by the *percentage* price change — so a 2% up day on
   1 M volume adds more to PVT than a 0.1% up day on the same
   volume. Distinct from OBV (cumulative volume with ±1
   direction based purely on sign of close change) and A/D Line
   (cumulative volume scaled by close-position within H/L range).
   PVT is the only volume surface in the packet with **percent-
   magnitude attribution**: it penalises volume that occurs on
   barely-changing bars and credits volume that occurs on
   conviction bars. Header gives **pvt_label** (STRONG_BULL /
   BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA for
   n<42). Labels driven by 20-bar PVT slope sign and magnitude
   combined with position above/below its own 20-bar EMA.

## Decision

Adopt the same additive schema-versioning pattern used in every prior
round:

- **Engine** (`engine/src/core/research.rs`): add
  `TrimaSnapshot / T3Snapshot / VidyaSnapshot / SmiSnapshot /
  PvtSnapshot` structs, each with compute/upsert/get helpers;
  `create_research_tables_v54` wraps `_v53` and adds five new tables
  (`research_trima`, `research_t3`, `research_vidya`, `research_smi`,
  `research_pvt`). Tests: 5 roundtrip + 5 compute_oscillating using
  the shared `synthetic_oscillating_bars_150()` fixture. 1261 tests
  pass (+10).
- **LAN sync** (`engine/src/core/lan_sync.rs`): whitelist the five
  table names in `SYNCABLE_TABLES`; add the five CREATE TABLE
  stanzas and the five `Some("updated_at")` timestamp-column entries.
- **Native** (`native/src/app.rs`): standard 9-section additive
  wiring: (1) 5 BrokerCmd variants, (2) 5 BrokerMsg variants, (3) 15
  struct fields (show/symbol/snapshot/loading × 5), (4) 15 default
  initialisers, (5) 5 compute-handler tokio tasks using
  `shared_cache_broker`, (6) 5 palette command aliases, (7) 5
  research packet markdown emitters, (8) 5 egui::Window renderers
  each with Use-Chart / Load-Cached / Compute controls and a striped
  summary grid, (9) 5 BrokerMsg result handlers.
- **Documentation**: this ADR plus five new sub-blocks 2.252–2.256
  in `docs/RESEARCH_PACKET.md` (renumbering INGESTED 2.252 → 2.257
  and Sector peer 2.253 → 2.258), and envelope updates from
  78–150 KB to 79–151 KB single-symbol and 750–1470 KB to
  760–1480 KB basket.

### Palette aliases

- TRIMA: `TRIMA | TRIMAFIT | TRIMA_WIN | TRIANGULAR_MA |
  TRIANGULAR_MOVING_AVERAGE`
- T3: `T3 | T3FIT | T3_WIN | TILLSON | TILLSON_T3`
- VIDYA: `VIDYA | VIDYAFIT | VIDYA_WIN | VARIABLE_INDEX_DYNAMIC |
  CHANDE_VIDYA`
- SMI: `SMI | SMIFIT | SMI_WIN | STOCHASTIC_MOMENTUM | BLAU_SMI`
- PVT: `PVT | PVTFIT | PVT_WIN | PRICE_VOLUME_TREND |
  VOLUME_PRICE_TREND`

No collisions with existing palette tokens. `STOCHASTIC_MOMENTUM`
distinguishes SMI from STOCH (ADR-160); `VOLUME_PRICE_TREND` is the
alternate industry name for PVT.

## Consequences

### Positive

- **Triangular-weighted central MA filled** (TRIMA) — the one MA
  shape missing from the lag-profile axis (central, symmetric,
  falling on both sides). TA-Lib ships TRIMA as a standard
  smoother; this round brings parity.
- **Full six-EMA Tillson composite shipped** (T3) — generalises the
  DEMA/TEMA family with a single parameterised `v` that spans the
  lag-reduction trade-off from EMA (v=0) to aggressive composite
  (v=1), with the canonical 0.7 exposed.
- **Momentum-adaptive MA added to the adaptive family** (VIDYA) —
  rounds out KAMA's efficiency-ratio scaling, MCGD's feedback
  scaling, and FRAMA's fractal scaling with the fourth canonical
  scaling pathway (CMO magnitude).
- **Double-smoothed mid-range stochastic** (SMI) — the one
  stochastic variant missing from the oscillator axis (STOCH
  anchored at L_min, STOCHRSI stochastic-of-RSI, now SMI anchored
  at `(H+L)/2`).
- **Percent-attribution volume indicator** (PVT) — the one volume
  indicator missing from the cumulative-volume axis; complements
  OBV (sign-attribution) and CHAIKOSC's underlying A/D Line
  (H-L-position attribution).
- +10 engine tests (5 roundtrip + 5 compute_oscillating)
  maintaining the property that every new surface has both
  persistence and compute-determinism coverage.

### Negative / Risks

- **T3 warm-up is 6×length.** Chaining six EMAs means the first few
  dozen bars of a fresh HP cache produce numerically-settling
  values even after the INSUFFICIENT_DATA floor is cleared. The
  label remains meaningful but the deviation magnitude for short
  warm-ups should be interpreted cautiously. The v_factor field
  is exposed so the AI can reason about which coefficient path
  dominates.
- **VIDYA alpha can collapse to ~0 during flat regimes** when
  |CMO| → 0; in that case VIDYA freezes near its prior value and
  deviation_pct grows purely from price drift. This is the
  *intended* behaviour (Chande designed VIDYA to freeze in
  ranges), but the label stays BULL/BEAR when it should perhaps
  read RANGE. We accept the standard VIDYA semantics and expose
  `current_alpha` + `cmo_magnitude` so the AI can see when VIDYA
  is effectively frozen.
- **SMI thresholds ±40 are more conservative than raw stochastic's
  80/20.** The double-EMA smoothing compresses the range in
  practice. Users coming from STOCH should not expect identical
  overbought/oversold frequency.
- **PVT is scale-dependent** (absolute cumulative number). The
  label is driven by *slope* and position relative to its own EMA,
  not the absolute level — so the label is comparable across
  symbols but the raw PVT value is not.

### Neutral

- No new API dependencies; all five surfaces reuse the existing
  `research_historical_price` HP cache. PVT requires volume data
  which HP already carries. This keeps the round fully
  free-API-compatible per the standing godel-parity directive.
- CMO is computed inline in VIDYA rather than reading from a
  separate CMO snapshot — keeps the implementation self-contained
  and avoids cross-table joins.

### Paid-API gap

None introduced in this round. All five surfaces are HP-derived
and work entirely from the existing free-data cache.

## Verification

- `cargo test -p typhoon-engine --lib`: 1261 tests pass (+10 from
  1251).
- `cargo build -p typhoon-native`: clean build, no new warnings.
- `docs/RESEARCH_PACKET.md`: 258 sub-blocks total (up from 253);
  envelope updated to 79–151 KB single-symbol and 760–1480 KB
  basket.

## Packet envelope delta

| Surface | Field count | Approx bytes when populated | Free / Paid |
|---|---|---|---|
| TRIMA | 10 | ~220 | Free (HP cache) |
| T3 | 11 | ~240 | Free (HP cache) |
| VIDYA | 13 | ~280 | Free (HP cache) |
| SMI | 13 | ~280 | Free (HP cache) |
| PVT | 10 | ~240 | Free (HP cache) |
| **Round 53 total** | **57 fields** | **≈1.26 KB** | **Free** |

Envelope: 78–150 KB → 79–151 KB single-symbol; 750–1470 KB →
760–1480 KB for the canonical 10-symbol basket.
