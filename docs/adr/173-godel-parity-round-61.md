# ADR-173: Godel Parity Round 61 — LAGUERRE_RSI / ZIGZAG / PGO / HT_TRENDLINE / MIDPOINT

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-172
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 60 (ADR-172) shipped WMA / RAINBOW / MESA_SINE / FRAMA / IBS.
Round 61 continues the additive indicator cadence with five more
canonical surfaces along the bounded-oscillator, pattern-detection,
trend-strength, adaptive-trendline, and range-midpoint axes.
LAGUERRE_RSI extends the RSI family with Ehlers's 4-stage Laguerre
filter; ZIGZAG provides the classic %-threshold reversal detector
distinct from FRACTALS (ADR-170) and PIVOTS (ADR-161); PGO brings
Mark Johnson's volatility-scaled trend oscillator; HT_TRENDLINE
uses the Hilbert-discriminator dominant period to drive a lag-matched
WMA; MIDPOINT rounds out the range-position family with TA-Lib's
canonical `(HHV+LLV)/2`.

1. **No Ehlers Laguerre RSI snapshot.** The Ehlers Laguerre RSI is
   a bounded [0, 1] oscillator built from Ehlers's 4-stage Laguerre
   filter (γ=0.5). The 4-stage filter smooths the close and produces
   L0, L1, L2, L3 intermediate outputs; the Laguerre RSI is then
   computed from the count of upward differences vs total differences
   across the stages, yielding a cleaner oscillator than classic RSI
   with no divergence false signals near extremes. Distinct from
   RSI (Wilder smoothing of gains/losses, ADR-108), STOCHRSI
   (ADR-137), CRSI (Connors, ADR-167), QQE (ADR-169), and IFT_RSI
   (ADR-170). Header gives **lrsi_label** (OVERBOUGHT / BULL /
   NEUTRAL / BEAR / OVERSOLD / INSUFFICIENT_DATA for n<20) from the
   laguerre_rsi magnitude (>0.85 overbought, >0.60 bull, <0.40 bear,
   <0.15 oversold, else neutral).

2. **No ZigZag pattern snapshot.** The classic %-threshold pivot
   reversal detector. A new pivot forms when price reverses by at
   least threshold_pct (default 5%) from the prior extreme. The
   snapshot emits the last high pivot (value + bars_ago), the last
   low pivot, the active leg direction (UP/DOWN), and the projected
   reversal level. Distinct from FRACTALS (ADR-170, Bill Williams
   5-bar strict peaks) and from PIVOTS (ADR-161, prior-session
   math), which use fundamentally different construction — ZigZag
   is a %-threshold reversal detector that tracks active swings
   rather than structural peaks or session-derived levels. Header
   gives **zigzag_label** (UP_LEG / DOWN_LEG / AT_REVERSAL /
   INSUFFICIENT_DATA for n<10) from the current leg and proximity
   to reversal level.

3. **No Pretty Good Oscillator (PGO) snapshot.** Mark Johnson's PGO
   measures the distance of the current close from an N-period SMA
   expressed in multiples of the N-period ATR:
   `pgo = (close − SMA(close, N)) / EMA(TR, N)` with N=14. Extreme
   readings of ±3 were found to be rare and persistent, making PGO
   a trend-following signal rather than mean-reversion — the "pretty
   good" name reflects Johnson's empirical observation that it
   filters noise better than raw ROC. Distinct from ROC (unscaled
   price change), PPO (percentage-scaled MACD, ADR-115), and
   RSI/STOCH (bounded oscillators) because PGO's scaling is by
   volatility, not percent. Header gives **pgo_label** (STRONG_BULL
   / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA for
   n<16) from pgo magnitude (>3 strong bull, >1 bull, <-1 bear,
   <-3 strong bear).

4. **No Hilbert Instantaneous Trendline (HT_TRENDLINE) snapshot.** A
   smoothed trendline based on the dominant cycle period derived
   from Ehlers's Hilbert-transform homodyne discriminator. Unlike
   MAMA (ADR-170) which outputs an adaptive MA proper, HT_TRENDLINE
   reports the `trendline = WMA(close, period)` over the detected
   cycle period — a lag-matched smoother that follows the dominant
   trend without the adaptive α rescaling. Distinct from MAMA
   (adaptive α), FRAMA (ADR-172 fractal-dimension α), and every
   fixed-length smoother. Header gives **ht_label** (BULL /
   WEAK_BULL / NEUTRAL / WEAK_BEAR / BEAR / INSUFFICIENT_DATA for
   n<64) from spread_pct thresholds (±0.5% weak, ±2% strong).

5. **No Midpoint of N (MIDPOINT) snapshot.** TA-Lib's MIDPOINT
   function: `midpoint = (HHV(N) + LLV(N)) / 2` emitting the
   midpoint of the N-bar range along with the HHV, LLV, and the
   close's position within the range. N=14. Useful as a simple
   anchor for detecting where price sits relative to the most
   recent trading range. Distinct from Donchian channel (ADR-151,
   raw HHV/LLV bands), from SMA, and from pivot systems (ADR-161)
   because it uses only HHV+LLV extremes rather than OHLC4 or
   session math. Header gives **midpoint_label** (UPPER /
   NEAR_UPPER / MIDRANGE / NEAR_LOWER / LOWER / INSUFFICIENT_DATA
   for n<15) from close_position (>0.85 upper, >0.60 near_upper,
   <0.40 near_lower, <0.15 lower, else midrange).

## Decision

Add five optional per-symbol snapshot blocks to the Godel-parity
pipeline, each reusing the existing `research_historical_price` HP
cache and the standard research-table LAN-sync path (no new API
dependencies):

1. `research::LaguerreRsiSnapshot` + `compute_laguerre_rsi_snapshot`
   + `upsert_laguerre_rsi` + `get_laguerre_rsi` — serialised to
   `research_laguerre_rsi`.
2. `research::ZigzagSnapshot` + `compute_zigzag_snapshot` +
   `upsert_zigzag` + `get_zigzag` — serialised to `research_zigzag`.
3. `research::PgoSnapshot` + `compute_pgo_snapshot` + `upsert_pgo`
   + `get_pgo` — serialised to `research_pgo`.
4. `research::HtTrendlineSnapshot` + `compute_ht_trendline_snapshot`
   + `upsert_ht_trendline` + `get_ht_trendline` — serialised to
   `research_ht_trendline`.
5. `research::MidpointSnapshot` + `compute_midpoint_snapshot` +
   `upsert_midpoint` + `get_midpoint` — serialised to
   `research_midpoint`.

Schema version bumps to v63 via `create_research_tables_v63` which
wraps v62 and adds five new CREATE TABLE stanzas (plus indexes on
`updated_at`). All five tables register in `SYNCABLE_TABLES`,
`create_table_sql`, and `table_timestamp_column` so LAN sync
incrementally propagates them to peer terminals.

Native wiring adds five `BrokerCmd::Compute*Snapshot` variants, five
`BrokerMsg::*SnapshotMsg` variants, five `show_*_win` / `*_symbol` /
`*_snapshot` / `*_loading` field tuples on `App`, five tokio-spawned
broker handlers (load HP cache → compute → upsert → emit msg), five
palette alias blocks (`LAGUERRE_RSI | LAGUERRERSI | LRSI |
LAGUERRE_RSI_WIN | EHLERS_LAGUERRE` etc.), five packet-emitter blocks
under section 2.293+ of the research packet, five egui windows with
Use-Chart / Load-Cached / Compute controls plus a striped Grid
summary, and five `BrokerMsg` match arms.

The research packet emits fresh sub-blocks 2.293 LAGUERRE_RSI, 2.294
ZIGZAG, 2.295 PGO, 2.296 HT_TRENDLINE, 2.297 MIDPOINT after the
existing 2.292 IBS sub-block; INGESTED renumbers 2.293 → 2.298 and
Sector peer 2.294 → 2.299. Envelope paragraph bumps "~86–165 KB" →
"~87–167 KB" with a description chain of the five new indicators
prepended.

## Consequences

- Packet scope grows by 10 k/v rows per symbol when all five
  snapshots are populated — roughly +260 bytes for LAGUERRE_RSI
  (L0..L3 stack), +280 bytes for ZIGZAG (dual-pivot fields), +240
  bytes for PGO, +260 bytes for HT_TRENDLINE, +260 bytes for
  MIDPOINT — for a typical +1.40 KB per symbol.
- Schema is strictly additive; old peers running v62 continue to
  work (new tables are absent but none of the old tables change).
  LAN sync skips unknown tables via the whitelist.
- All five indicators share the HP cache, so no additional API cost.
- Like Round 60 + earlier rounds, the five tests + five
  roundtrip/compute tests guard against regressions.

## Verification

1. **Engine tests:** `cargo test --package typhoon-engine --lib`
   passes 1346 tests (+10 from Round 60's 1336).
2. **Native build:** `cargo build --package typhoon-native` completes
   in 3m 23s with no warnings/errors.
3. **Unique palette tokens:** `LAGUERRE_RSI`, `ZIGZAG`, `PGO`,
   `HT_TRENDLINE`, `MIDPOINT` + their suffixed aliases are all fresh
   — no palette collisions with prior rounds.
4. **LAN sync whitelist:** All five `research_*` tables registered
   in `SYNCABLE_TABLES`, `create_table_sql`, and
   `table_timestamp_column`; incremental sync uses the `updated_at`
   column.

## Packet envelope delta

Before Round 61: packet emitted 66 k/v rows across Round 59 + Round
60 additions. After Round 61: 76 k/v rows when all fifteen Round 59
+ Round 60 + Round 61 additions populate, typical +1.40 KB per
symbol on top of the +1.46 KB Round 60 added and +1.49 KB Round 59
added — bringing the observed per-symbol envelope from ~86–165 KB
to ~87–167 KB.
