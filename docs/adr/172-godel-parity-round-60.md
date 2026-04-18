# ADR-172: Godel Parity Round 60 — WMA / RAINBOW / MESA_SINE / FRAMA / IBS

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-171
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 59 (ADR-171) shipped DEMARKER / GATOR / BW_MFI / VWMA / STDDEV.
Round 60 continues the additive indicator cadence with five more
canonical surfaces along the moving-average, multi-level rainbow,
cycle-phase, and single-bar position axes. WMA completes the plain
fixed-length MA family (SMA, EMA, HMA, DEMA, TEMA, ALMA, ...);
RAINBOW offers Mel Widner's multi-level recursive SMA fan distinct
from Guppy's GMMA (ADR-168); MESA_SINE brings Ehlers's cycle-phase
oscillator distinct from MAMA (ADR-170) and FISHER (ADR-129); FRAMA
adds Ehlers's fractal-dimension-adaptive moving average alongside
KAMA (ADR-117), VIDYA (ADR-148), T3 (ADR-142), and MAMA (ADR-170);
IBS provides a single-bar position metric unrelated to every
multi-bar oscillator on the shipped list.

1. **No Weighted Moving Average (WMA) snapshot.** The linearly-
   weighted N-period average `wma = Σ(price[i] · (i+1)) / Σ(i+1)`
   over i=0..N-1 with N=20. WMA puts more emphasis on recent bars
   than SMA (equal weights) but less than EMA (exponential decay),
   producing a smoother line that still reacts to recent price
   changes. The WMA is a building block of HMA (Hull MA, ADR-122)
   which computes `WMA(2·WMA(n/2) − WMA(n), √n)`, but the plain WMA
   itself was missing from the shipped list — distinct from SMA,
   EMA, HMA, DEMA (ADR-117), TEMA (ADR-117), T3 (ADR-142), ALMA
   (ADR-148), KAMA (ADR-117), MAMA (ADR-170), and every adaptive MA.
   Header gives **wma_label** (BULL / WEAK_BULL / NEUTRAL /
   WEAK_BEAR / BEAR / INSUFFICIENT_DATA for n<21) derived from
   close/WMA spread thresholds (±0.5% for weak, ±2.0% for strong).

2. **No Rainbow MA Oscillator snapshot.** Mel Widner's 10-level
   recursive SMA stack where `r_1 = SMA(close, 2)`, `r_2 = SMA(r_1,
   2)`, ..., `r_10 = SMA(r_9, 2)`. Each level is a 2-bar SMA of the
   prior level, creating a "rainbow" fan around price. The
   oscillator reports the highest-high minus lowest-low across the
   10 levels (the rainbow width) along with the fan's current
   center (mean of all levels). A wide rainbow means strong trend
   (levels spread apart as price runs); a narrow rainbow means
   consolidation (levels bunched tightly). Distinct from GMMA
   (Guppy's 12-line EMA fan with varying lengths 3/5/8/10/12/15 and
   30/35/40/45/50/60, ADR-168) — the construction methods differ
   fundamentally. Header gives **rainbow_label** (STRONG_TREND /
   TRENDING / CONSOLIDATING / INSUFFICIENT_DATA for n<22) from the
   width-to-center ratio (>2% strong, >0.5% trending, else
   consolidating).

3. **No Ehlers MESA Sine Wave snapshot.** Uses a simplified Hilbert-
   transform phase estimator (4-tap smoother + 7-tap quadrature
   detrender) to detect the dominant cycle phase angle from in-phase
   and quadrature components, then emits `sine = sin(phase)` and
   `lead_sine = sin(phase + π/4)`. When the sine crosses above the
   lead_sine, a CYCLE_BUY signal fires (cycle-bottom); when it
   crosses below, a CYCLE_SELL signal fires (cycle-top). In trending
   markets the two lines separate and fail to cross (|sine − lead| >
   0.6), producing no signals — a useful regime filter in itself
   (TRENDING label). Distinct from MAMA (phase-adaptive MA, ADR-170)
   which uses the same Hilbert-discriminator to drive α rather than
   emit a phase-based sine; from FISHER (probability Gaussianization
   of price percentile, ADR-129); and from COG (weighted centroid,
   ADR-170). Header gives **mesa_label** (CYCLE_BUY / CYCLE_SELL /
   TRENDING / NEUTRAL / INSUFFICIENT_DATA for n<32).

4. **No Fractal Adaptive Moving Average (FRAMA) snapshot.** Ehlers's
   adaptive MA where the smoothing α is driven by the fractal
   dimension D of the price series over the last N=16 bars. Computed
   by dividing N into two halves, measuring the (H−L) range of each
   half (n1, n2) and the combined range (n3), then `D = (log(n1 +
   n2) − log(n3)) / log(2)`. `α = exp(−4.6·(D − 1))` clamped to
   [0.01, 1.0]. Strong trends (D near 1.0) yield α ≈ 1
   (fast-following); choppy markets (D near 2.0, which
   characterizes Brownian-motion-like random walks) yield α ≈ 0.01
   (heavy smoothing). Distinct from KAMA (efficiency-ratio adaptive,
   ADR-117), VIDYA (volatility-index adaptive, ADR-148), MAMA
   (Hilbert-phase adaptive, ADR-170), and T3 (Tillson triple-DEMA,
   ADR-142). Header gives **frama_label** (STRONG_TREND / TREND /
   CHOP / INSUFFICIENT_DATA for n<32) from D magnitude (<1.35
   strong, <1.65 trending, else chop).

5. **No Internal Bar Strength (IBS) snapshot.** The position of
   close within the current bar's high/low range: `ibs = (close −
   low) / (high − low)`, bounded on [0, 1]. A 14-bar SMA smooths the
   raw reading. Values near 1 indicate close at the high (bullish
   conviction within the bar); values near 0 indicate close at the
   low (bearish conviction). IBS is a mean-reversion favorite —
   high IBS (>0.8) often precedes short-term pullbacks, low IBS
   (<0.2) often precedes bounces. Distinct from STOCH (%K over N-bar
   HHV/LLV, ADR-108) which spans multiple bars for the range, and
   from every momentum oscillator; IBS is a single-bar position
   metric that averages over a window rather than aggregating
   across bars. Header gives **ibs_label** (OVERBOUGHT / BULL /
   NEUTRAL / BEAR / OVERSOLD / INSUFFICIENT_DATA for n<15) from the
   smoothed IBS magnitude (>0.8 overbought, >0.6 bull, <0.4 bear,
   <0.2 oversold, else neutral).

## Decision

Add five optional per-symbol snapshot blocks to the Godel-parity
pipeline, each reusing the existing `research_historical_price` HP
cache and the standard research-table LAN-sync path (no new API
dependencies):

1. `research::WmaSnapshot` + `compute_wma_snapshot` + `upsert_wma` +
   `get_wma` — serialised to `research_wma`.
2. `research::RainbowSnapshot` + `compute_rainbow_snapshot` +
   `upsert_rainbow` + `get_rainbow` — serialised to `research_rainbow`.
3. `research::MesaSineSnapshot` + `compute_mesa_sine_snapshot` +
   `upsert_mesa_sine` + `get_mesa_sine` — serialised to
   `research_mesa_sine`.
4. `research::FramaSnapshot` + `compute_frama_snapshot` +
   `upsert_frama` + `get_frama` — serialised to `research_frama`.
5. `research::IbsSnapshot` + `compute_ibs_snapshot` + `upsert_ibs` +
   `get_ibs` — serialised to `research_ibs`.

Schema version bumps to v62 via `create_research_tables_v62` which
wraps v61 and adds five new CREATE TABLE stanzas (plus indexes on
`updated_at`). All five tables register in `SYNCABLE_TABLES`,
`create_table_sql`, and `table_timestamp_column` so LAN sync
incrementally propagates them to peer terminals.

Native wiring adds five `BrokerCmd::Compute*Snapshot` variants, five
`BrokerMsg::*SnapshotMsg` variants, five `show_*_win` / `*_symbol` /
`*_snapshot` / `*_loading` field tuples on `App`, five tokio-spawned
broker handlers (load HP cache → compute → upsert → emit msg), five
palette alias blocks (`WMA | WEIGHTED_MA | WMA_WIN | LINEAR_WEIGHTED_MA`
etc.), five packet-emitter blocks under section 2.288+ of the
research packet, five egui windows with Use-Chart / Load-Cached /
Compute controls plus a striped Grid summary, and five
`BrokerMsg` match arms.

The research packet emits fresh sub-blocks 2.288 WMA, 2.289 RAINBOW,
2.290 MESA_SINE, 2.291 FRAMA, 2.292 IBS after the existing 2.287
STDDEV sub-block; INGESTED renumbers 2.288 → 2.293 and Sector peer
2.289 → 2.294. Envelope paragraph bumps "~85–163 KB" → "~86–165 KB"
with a description chain of the five new indicators prepended.

## Consequences

- Packet scope grows by 10 k/v rows per symbol when all five
  snapshots are populated — roughly +260 bytes for WMA, +340 bytes
  for RAINBOW (10-level stack needs more fields), +320 bytes for
  MESA_SINE (phase-pair), +280 bytes for FRAMA, +260 bytes for IBS —
  for a typical +1.46 KB per symbol.
- Schema is strictly additive; old peers running v61 continue to
  work (new tables are absent but none of the old tables change).
  LAN sync skips unknown tables via the whitelist.
- All five indicators share the HP cache, so no additional API cost.
- Like Round 59 + earlier rounds, the five tests + five
  roundtrip/compute tests guard against regressions.

## Verification

1. **Engine tests:** `cargo test --package typhoon-engine --lib`
   passes 1336 tests (+10 from Round 59's 1326).
2. **Native build:** `cargo build --package typhoon-native` completes
   in 3m 23s with no warnings/errors.
3. **Unique palette tokens:** `WMA`, `RAINBOW`, `MESA_SINE`, `FRAMA`,
   `IBS` + their suffixed aliases are all fresh — no palette
   collisions with prior rounds.
4. **LAN sync whitelist:** All five `research_*` tables registered
   in `SYNCABLE_TABLES`, `create_table_sql`, and
   `table_timestamp_column`; incremental sync uses the `updated_at`
   column.

## Packet envelope delta

Before Round 60: packet emitted 56 k/v rows across Round 59
additions. After Round 60: 66 k/v rows when all ten Round 59 + Round
60 additions populate, typical +1.46 KB per symbol on top of the
+1.49 KB Round 59 added — bringing the observed per-symbol envelope
from ~85–163 KB to ~86–165 KB.
