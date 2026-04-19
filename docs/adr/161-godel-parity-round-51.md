# ADR-161: Godel Parity Round 51 — DEMA / TEMA / LINREG / PIVOTS / HEIKIN

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-160
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| DEMA | Canonical (all terminals) | Yes (`DEMA`) | Yes | Yes | No (deferred — ADR-188) |
| TEMA | Canonical (all terminals) | Yes (`TEMA`) | Yes | Yes | No (deferred — ADR-188) |
| LINREG | Canonical (all terminals) | Yes (`LINEARREG` / `LINEARREG_SLOPE` / `LINEARREG_INTERCEPT` / `LINEARREG_ANGLE`) | Yes | Yes | No (deferred — ADR-188) |
| PIVOTS | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |
| HEIKIN | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** canonical technical primitives common across all terminals (Mulloy DEMA via TA-Lib `DEMA`, TEMA via `TEMA`, OLS linear regression channel via `LINEARREG` family, classic floor-trader pivots, Heikin-Ashi candles).

## Context

Round 50 (ADR-160) shipped STOCH/MACD/VWAP/MCGD/RWI, taking HP-local
research surfaces to 202 and per-symbol sub-blocks to 243. Round 51
continues the additive cadence with five more canonical indicators
that still had no stand-alone snapshot in the packet after 50 rounds
of additive work. Each has a sharp domain purpose distinct from what
is already shipped.

1. **No Double EMA (DEMA) snapshot.** Patrick Mulloy's 1994 DEMA is
   defined as `DEMA = 2·EMA(N) − EMA(EMA(N))`, length 20. Subtracting
   the lag component of EMA(EMA(N)) — which lags EMA the same way EMA
   lags price — yields an MA with roughly half the residual lag of a
   standard EMA(20). First surface we ship in the Mulloy lag-reduction
   family. Distinct from MCGD (ADR-160, adaptive-by-feedback) and KAMA
   (ADR-151, adaptive-by-efficiency-ratio): DEMA reduces lag
   *algebraically* (subtracting the lag term) rather than adaptively.
   Header gives **dema_label** (STRONG_BULL for >+2% deviation / BULL /
   NEUTRAL / BEAR / STRONG_BEAR for <−2% / INSUFFICIENT_DATA for
   n<42).

2. **No Triple EMA (TEMA) snapshot.** Patrick Mulloy's 1994 TEMA
   extends DEMA to third order: `TEMA = 3·EMA(N) − 3·EMA(EMA(N)) +
   EMA(EMA(EMA(N)))`, length 20. Further reduces residual lag that
   DEMA leaves after cancelling EMA's first-order lag. Pairs with DEMA
   for the full Mulloy family. Distinct from TRIX (ADR-154, *rate-of-
   change* of triple EMA) — TEMA is a price level MA, TRIX is an
   oscillator derived from the same triple-EMA chain. Header gives
   **tema_label** (STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR /
   INSUFFICIENT_DATA for n<63).

3. **No linear regression channel snapshot.** LINREG runs OLS
   regression `y = slope·t + intercept` over the last N=20 closes,
   with R² coefficient of determination ∈ [0, 1] and σ = standard
   error of residuals. Channel bounds at `fit_value ± 2σ` bracket the
   fair-value envelope under the regression hypothesis. First
   parametric fair-value surface we ship: unlike VWAP (volume-weighted
   mean) or MCGD (adaptive MA), LINREG provides an explicit goodness-
   of-fit score so the AI can discount the channel when R² is low.
   Complements BBSQUEEZE (ADR-151) and DONCHIAN (ADR-151) on the
   channel/envelope axis. Header gives **linreg_label**
   (STRONG_UP_TREND for slope > 0 and R² ≥ 0.7 / UP_TREND for slope >
   0 and R² ≥ 0.4 / RANGE for R² < 0.4 / DOWN_TREND / STRONG_DOWN_TREND
   / INSUFFICIENT_DATA for n<20).

4. **No classic floor-trader pivot-points snapshot.** PIVOTS emits
   the canonical daily pivot grid from the prior bar OHLC:
   `PP = (H+L+C)/3; R1 = 2·PP − L; S1 = 2·PP − H; R2 = PP + (H−L);
   S2 = PP − (H−L)`. The single most-recognised intraday S/R framework
   in US equities; still the default overlay on Bloomberg,
   TradingView, and most retail charting stacks. Distinct from
   SUPERTREND (ADR-152, ATR-channel), DONCHIAN (ADR-151, N-bar H/L),
   and BBSQUEEZE (ADR-151, σ-envelope): PIVOTS is a *prior-bar-derived
   fixed grid* — no moving averages, no volatility scaling, just the
   canonical floor-pit arithmetic. Header gives **pivots_label**
   describing where the current close sits relative to the grid
   (ABOVE_R2 / BETWEEN_R1_R2 / BETWEEN_PP_R1 / AT_PP / BETWEEN_S1_PP /
   BETWEEN_S2_S1 / BELOW_S2 / INSUFFICIENT_DATA for n<2).

   **Palette conflict resolution:** Bare `PIVOTS` already toggles the
   chart overlay (`show_pivots`) via the existing palette handler. To
   preserve muscle memory, the Round-51 snapshot command aliases use
   `PIVOTSFIT | PIVOTS_WIN | PIVOTS_SNAPSHOT | FLOOR_PIVOTS |
   PIVOT_POINTS_WIN` — same disambiguation pattern as Round 50 used
   for VWAP.

5. **No Heikin-Ashi numerical snapshot.** HEIKIN emits the recursive
   candle transform: `HA_close = (O+H+L+C)/4; HA_open = (prior_HA_open
   + prior_HA_close)/2; HA_high = max(H, HA_open, HA_close);
   HA_low = min(L, HA_open, HA_close)`. The recursive definition
   smooths noise by partially averaging consecutive bars, producing
   cleaner uninterrupted colour runs than raw candles. Particularly
   effective at filtering single-bar reversals that otherwise create
   false-signal chop. First sentiment-run-length surface in the
   packet: unlike RUNLEN (ADR-129, *raw-close* run length), HEIKIN
   measures run length after the HA smoothing — which the AI can
   compare to detect raw/smoothed divergence. Header gives
   **heikin_label** (STRONG_BULL_RUN for ≥4 consecutive same-colour
   bullish candles / BULL / DOJI / BEAR / STRONG_BEAR_RUN
   symmetrically / INSUFFICIENT_DATA for n<2).

   **Palette conflict resolution:** Bare `HEIKINASHI` already switches
   the chart type to Heikin-Ashi candles via the existing palette
   handler. The Round-51 snapshot aliases are `HEIKIN | HEIKIN_WIN |
   HEIKIN_SNAPSHOT | HEIKIN_ASHI_SNAPSHOT | HA_SNAPSHOT` — the chart
   transform and the numerical snapshot stay distinct so both remain
   reachable. Similarly, `TRIPLE_EMA` was already claimed by TRIX's
   alias set, so TEMA's disambiguated alias uses `TRIPLE_EMA_WIN`.

## Decision

Adopt the same additive schema-versioning pattern used in every prior
round:

- **Engine** (`engine/src/core/research.rs`): add
  `DemaSnapshot / TemaSnapshot / LinregSnapshot / PivotsSnapshot /
  HeikinSnapshot` structs, each with compute/upsert/get helpers;
  `create_research_tables_v52` wraps `_v51` and adds five new tables
  (`research_dema`, `research_tema`, `research_linreg`,
  `research_pivots`, `research_heikin`). Tests: 5 roundtrip + 5
  compute_oscillating using the shared `synthetic_oscillating_bars_150()`
  fixture. 1230 tests pass (+10).
- **LAN sync** (`engine/src/core/lan_sync.rs`): whitelist the five
  table names in `SYNCABLE_TABLES`; add the five CREATE TABLE stanzas
  and the five `Some("updated_at")` timestamp-column entries.
- **Native** (`native/src/app.rs`): standard 9-section additive wiring:
  (1) 5 BrokerCmd variants, (2) 5 BrokerMsg variants, (3) 15 struct
  fields (show/symbol/snapshot/loading × 5), (4) 15 default
  initialisers, (5) 5 compute-handler tokio tasks using
  `shared_cache_broker`, (6) 5 palette command aliases (with
  disambiguated forms for PIVOTS/HEIKIN/TEMA), (7) 5 research packet
  markdown emitters in the research packet builder, (8) 5 egui::Window
  renderers each with Use-Chart / Load-Cached / Compute controls and
  a striped summary grid, (9) 5 BrokerMsg result handlers.
- **Documentation**: this ADR plus five new sub-blocks 2.242–2.246 in
  `docs/RESEARCH_PACKET.md` (renumbering INGESTED 2.242 → 2.247 and
  Sector peer 2.243 → 2.248), and envelope updates from 76–148 KB
  to 77–149 KB single-symbol and 730–1450 KB to 740–1460 KB basket.

## Consequences

### Positive

- Mulloy lag-reduction family now present in the packet (DEMA + TEMA)
  — complements the feedback-adaptive family (MCGD) and ER-adaptive
  family (KAMA) already shipped.
- First parametric fair-value surface with explicit goodness-of-fit
  (LINREG R²) lets the AI discount the channel when fit is poor —
  previously unavailable for VWAP, MCGD, KAMA, or any other fair-value
  reference.
- Canonical floor-trader PIVOTS now in the packet — the single most-
  recognised intraday S/R framework in US equities, previously only
  present as a chart overlay toggle.
- First numerical Heikin-Ashi snapshot — the chart type was already
  supported as a visual transform, but the AI had no access to the
  numerical OHLC, body/wick geometry, or consecutive-same-colour run
  length that make HA useful for trend-continuation analysis.
- +10 engine tests (5 roundtrip + 5 compute_oscillating) maintaining
  the property that every new surface has both persistence and
  compute-determinism coverage.

### Negative / Risks

- TEMA requires ≥63 bars of HP cache warm-up (3× the length of 20).
  Symbols with recent IPOs or gapped data will report
  INSUFFICIENT_DATA more often than DEMA (≥42) or LINREG (≥20). The
  label/note pair makes this explicit to the AI consumer.
- LINREG with R² < 0.4 is labelled RANGE, but the slope and channel
  are still reported — the AI must honour the label rather than
  extrapolating the slope naively. The emitter prints R² alongside
  slope to make this visible.
- Bare `PIVOTS` and `HEIKINASHI` palette commands retain their
  existing chart-overlay / chart-type meanings; the snapshot windows
  require the disambiguated aliases documented above. Users expecting
  bare `PIVOTS` to open a snapshot window will need to adapt to
  `PIVOTSFIT` or similar.

### Neutral

- No new API dependencies; all five surfaces reuse the existing
  `research_historical_price` HP cache. This keeps the round fully
  free-API-compatible per the standing godel-parity directive.
- `DOUBLE_EMA` / `DOUBLE_EXPONENTIAL` for DEMA are new palette tokens
  and do not collide with anything.

### Paid-API gap

None introduced in this round. All five surfaces are HP-derived and
work entirely from the existing free-data cache.

## Verification

- `cargo test -p typhoon-engine`: 1230 tests pass (+10 from 1220).
- `cargo build -p typhoon-native`: clean build, no warnings after the
  TEMA `TRIPLE_EMA` → `TRIPLE_EMA_WIN` alias rename that resolved the
  one transient unreachable-pattern warning (TRIX had claimed
  `TRIPLE_EMA` in ADR-154).
- `docs/RESEARCH_PACKET.md`: 248 sub-blocks total (up from 243);
  envelope updated to 77–149 KB single-symbol and 740–1460 KB basket.

## Packet envelope delta

| Surface | Field count | Approx bytes when populated | Free / Paid |
|---|---|---|---|
| DEMA | 10 | ~220 | Free (HP cache) |
| TEMA | 10 | ~220 | Free (HP cache) |
| LINREG | 13 | ~280 | Free (HP cache) |
| PIVOTS | 12 | ~260 | Free (HP cache) |
| HEIKIN | 13 | ~260 | Free (HP cache) |
| **Round 51 total** | **58 fields** | **≈1.24 KB** | **Free** |

Envelope: 76–148 KB → 77–149 KB single-symbol; 730–1450 KB →
740–1460 KB for the canonical 10-symbol basket.
