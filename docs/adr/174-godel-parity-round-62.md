# ADR-174: TA-Lib + Godel Parity Round 62 — MASSINDEX / NATR / TTM_SQUEEZE / FORCE_INDEX / TRANGE

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-108 through ADR-173
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel Terminal documented | TA-Lib primitive | Research packet | egui popup | Chart overlay |
|---|---|---|---|---|---|
| MASSINDEX | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |
| NATR | No | Yes (`NATR`) | Yes | Yes | No (deferred — ADR-188) |
| TTM_SQUEEZE | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |
| FORCE_INDEX | Canonical (all terminals) | No | Yes | Yes | No (deferred — ADR-188) |
| TRANGE | No | Yes (`TRANGE`) | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** mixed — canonical technical indicators (Donald Dorsey Mass Index, TTM Squeeze regime, Elder Force Index) plus TA-Lib-only primitives (`NATR` normalised ATR, `TRANGE` raw single-bar True Range).

## Context

Round 61 (ADR-173) shipped LAGUERRE_RSI / ZIGZAG / PGO / HT_TRENDLINE /
MIDPOINT. Round 62 continues the additive indicator cadence with five
more canonical surfaces along the volatility-reversal, normalized-ATR,
squeeze-regime, volume-pressure, and raw-true-range axes. MASSINDEX
brings Donald Dorsey's EMA/EMA ratio reversal bulge detector; NATR
provides TA-Lib's normalized ATR as a percentage of close; TTM_SQUEEZE
brings John Carter's BB-inside-KC compression regime with linear-
regression momentum; FORCE_INDEX adds Alexander Elder's volume-weighted
price-change oscillator; TRANGE rounds out the range-math family with
TA-Lib's raw single-bar True Range.

1. **No Dorsey Mass Index snapshot.** The Mass Index is a volatility-
   based reversal indicator: `MI = Σ(EMA(H-L, 9) / EMA(EMA(H-L, 9), 9))`
   summed over 25 bars. Readings above 27 that subsequently fall back
   below 26.5 signal a "reversal bulge" — an imminent trend reversal
   without committing to direction. Distinct from ATR (Wilder TR
   smoothing) and Chaikin Volatility (high-low range EMA rate-of-
   change), this indicator uses the ratio of fast vs slow EMA of the
   H-L range as a pure volatility-expansion signal. Header gives
   **mass_label** (REVERSAL_BULGE / ELEVATED / NEUTRAL / COMPRESSED /
   INSUFFICIENT_DATA for n<35) from the mass_index magnitude (>27
   reversal_bulge, >24 elevated, <21 compressed, else neutral).

2. **No Normalized ATR (NATR) snapshot.** TA-Lib's NATR function:
   `natr = 100 × Wilder_ATR(14) / close`, expressing Wilder's Average
   True Range as a percentage of the closing price. This makes ATR
   scale-invariant so it can be compared across symbols of different
   price levels (a $5 ATR means different things for a $10 stock vs a
   $500 stock). Distinct from raw ATR (price-scaled) and from volatility
   indices like VIX (option-implied vol) — NATR is realized volatility
   expressed as percentage. Header gives **natr_label** (HIGH_VOL /
   ELEVATED / NORMAL / LOW_VOL / INSUFFICIENT_DATA for n<15) from natr
   magnitude (>5% high, >2.5% elevated, <1% low_vol).

3. **No TTM Squeeze snapshot.** John Carter's TTM Squeeze is a regime
   flag: when Bollinger Bands (2σ) fit entirely inside Keltner Channels
   (1.5×ATR), volatility is compressed and a breakout is imminent
   ("squeeze on"). When BB expands outside KC, the squeeze "fires" and
   directional momentum typically follows — paired with a linear-
   regression momentum oscillator on `close - (HHV+LLV)/2` for direction.
   Distinct from Bollinger-only (ADR-108), Keltner-only (ADR-151), and
   from plain BBWIDTH expansion (ADR-140) because it combines both
   bands and a momentum direction. Header gives **squeeze_label**
   (SQUEEZE_ON / FIRE_UP / FIRE_DOWN / NEUTRAL / INSUFFICIENT_DATA for
   n<21) from the BB⊂KC flag and momentum sign.

4. **No Elder Force Index snapshot.** Alexander Elder's Force Index:
   `force = volume × (close − close_prev)`, smoothed by a 13-period
   EMA. Measures the buying/selling pressure behind each price move
   weighted by volume. A bullish divergence (price makes lower low but
   force makes higher low) signals trend exhaustion. Distinct from OBV
   (cumulative volume), MFI (money flow index), and A/D Line (volume
   × close-location weight) because Force Index combines the magnitude
   of the price change with volume. Header gives **force_label**
   (STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR /
   INSUFFICIENT_DATA for n<15) from the ratio of EMA(force) to
   mean(|force|) over the last 50 bars (>1.5 strong_bull, >0.25 bull,
   <-0.25 bear, <-1.5 strong_bear).

5. **No raw True Range (TRANGE) snapshot.** TA-Lib's TRANGE function:
   `tr = max(H − L, |H − C_prev|, |L − C_prev|)`, the single-bar
   volatility measure that underlies Wilder's ATR (ADR-108) but reports
   the current bar's TR directly without any smoothing. Useful for gap-
   aware bar-size comparisons and for building custom volatility systems.
   Distinct from ATR (N-period EMA of TR) and from the bar's raw range
   (H − L) which ignores gaps. Header gives **trange_label** (EXPANSION
   / NORMAL / CONTRACTION / INSUFFICIENT_DATA for n<21) from the ratio
   of the latest TR to the 20-bar mean TR (>1.5 expansion, <0.5
   contraction, else normal).

## Decision

Add five optional per-symbol snapshot blocks to the Godel-parity
pipeline, each reusing the existing `research_historical_price` HP
cache and the standard research-table LAN-sync path (no new API
dependencies):

1. `research::MassIndexSnapshot` + `compute_mass_index_snapshot` +
   `upsert_mass_index` + `get_mass_index` — serialised to
   `research_mass_index`.
2. `research::NatrSnapshot` + `compute_natr_snapshot` + `upsert_natr`
   + `get_natr` — serialised to `research_natr`.
3. `research::TtmSqueezeSnapshot` + `compute_ttm_squeeze_snapshot` +
   `upsert_ttm_squeeze` + `get_ttm_squeeze` — serialised to
   `research_ttm_squeeze`.
4. `research::ForceIndexSnapshot` + `compute_force_index_snapshot` +
   `upsert_force_index` + `get_force_index` — serialised to
   `research_force_index`.
5. `research::TrangeSnapshot` + `compute_trange_snapshot` +
   `upsert_trange` + `get_trange` — serialised to `research_trange`.

Schema version bumps to v64 via `create_research_tables_v64` which
wraps v63 and adds five new CREATE TABLE stanzas (plus indexes on
`updated_at`). All five tables register in `SYNCABLE_TABLES`,
`create_table_sql`, and `table_timestamp_column` so LAN sync
incrementally propagates them to peer terminals.

Native wiring adds five `BrokerCmd::Compute*Snapshot` variants, five
`BrokerMsg::*SnapshotMsg` variants, five `show_*_win` / `*_symbol` /
`*_snapshot` / `*_loading` field tuples on `App`, five tokio-spawned
broker handlers (load HP cache → compute → upsert → emit msg), five
palette alias blocks, five packet-emitter blocks under section 2.298+
of the research packet, five egui windows with Use-Chart / Load-Cached
/ Compute controls plus a striped Grid summary, and five `BrokerMsg`
match arms.

Palette aliases were selected to avoid collision with earlier rounds:
`MASSINDEX | MI | MASS_INDEX_WIN | MINDEX | MASS_25` (bare
`MASS_INDEX`/`DORSEY_MASS` are claimed by ADR-156 Round 47 curvefit);
`NATR | NORMALIZED_ATR | NATR_WIN | NORMALIZED_ATR_WIN | ATR_PCT`;
`TTM_SQUEEZE | TTMSQUEEZE | TTM_SQUEEZE_WIN | CARTER_SQUEEZE | TTM`
(bare `SQUEEZE` is a chart toggle); `FORCEINDEX | FORCE | FI |
FORCE_INDEX_WIN | FORCE13` (bare `FORCE_INDEX`/`ELDER_FORCE` are
claimed by ADR-158 Round 48 EFI curvefit); `TRANGE | TRUE_RANGE | TR
| TRANGE_WIN | RAW_TRUE_RANGE`.

The research packet emits fresh sub-blocks 2.298 MASSINDEX, 2.299 NATR,
2.300 TTM_SQUEEZE, 2.301 FORCE_INDEX, 2.302 TRANGE after the existing
2.297 MIDPOINT sub-block; INGESTED renumbers 2.298 → 2.303 and Sector
peer 2.299 → 2.304. Envelope paragraph bumps "~87–167 KB" → "~88–169
KB" with a description chain of the five new indicators prepended.

## Consequences

- Packet scope grows by 10 k/v rows per symbol when all five
  snapshots are populated — roughly +270 bytes for MASSINDEX
  (EMA/EMA stack), +220 bytes for NATR, +300 bytes for TTM_SQUEEZE
  (BB + KC quad-band), +260 bytes for FORCE_INDEX, +280 bytes for
  TRANGE (raw + gap-aware prior close) — for a typical +1.45 KB per
  symbol.
- Schema is strictly additive; old peers running v63 continue to
  work (new tables are absent but none of the old tables change).
  LAN sync skips unknown tables via the whitelist.
- All five indicators share the HP cache, so no additional API cost.
- Like Round 61 + earlier rounds, the five tests + five
  roundtrip/compute tests guard against regressions.

## Verification

1. **Engine tests:** `cargo test --package typhoon-engine --lib`
   passes 1356 tests (+10 from Round 61's 1346).
2. **Native build:** `cargo build --package typhoon-native` completes
   in 3m 29s with no warnings/errors.
3. **Unique palette tokens:** Colliding aliases avoided — only fresh
   tokens (`MASSINDEX`, `MI`, `MINDEX`, `MASS_25`, `NATR`,
   `NORMALIZED_ATR`, `ATR_PCT`, `TTM_SQUEEZE`, `TTMSQUEEZE`, `TTM`,
   `CARTER_SQUEEZE`, `FORCEINDEX`, `FORCE`, `FI`, `FORCE13`, `TRANGE`,
   `TRUE_RANGE`, `TR`, `RAW_TRUE_RANGE`) are bound. Bare `MASS_INDEX`,
   `DORSEY_MASS`, `FORCE_INDEX`, `ELDER_FORCE`, and `SQUEEZE` remain
   with their original owners.
4. **LAN sync whitelist:** All five `research_*` tables registered
   in `SYNCABLE_TABLES`, `create_table_sql`, and
   `table_timestamp_column`; incremental sync uses the `updated_at`
   column.

## Packet envelope delta

Before Round 62: packet emitted 76 k/v rows across Round 59 + Round
60 + Round 61 additions. After Round 62: 86 k/v rows when all twenty
Round 59 + Round 60 + Round 61 + Round 62 additions populate, typical
+1.45 KB per symbol on top of the +1.40 KB Round 61 added, +1.46 KB
Round 60 added, and +1.49 KB Round 59 added — bringing the observed
per-symbol envelope from ~87–167 KB to ~88–169 KB.
