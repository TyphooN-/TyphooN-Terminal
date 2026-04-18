# ADR-178: Godel Parity Round 66 — AVGPRICE / MEDPRICE / TYPPRICE / WCLPRICE / VARIANCE

**Status:** Accepted
**Date:** 2026-04-18
**Supersedes/extends:** ADR-108 through ADR-177
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 65 (ADR-177) shipped MIDPRICE / APO / MOM / SAREXT / ADXR. Round
66 adds the TA-Lib price-transform primitives — the four OHLC-input
surfaces used by every downstream price-study (CCI, VWAP variants,
Heikin-Ashi, etc.) — plus the base statistical variance primitive that
underlies STDDEV / Z-score / Bollinger / many vol-regime classifiers.

1. **No AVGPRICE snapshot.** TA-Lib's AVGPRICE function:
   `avgprice = (open + high + low + close) / 4`. The simplest
   four-component OHLC average, distinct from TYPPRICE (drops open,
   adds close weight indirectly via `(H+L+C)/3`) and WCLPRICE (drops
   open, double-weights close). Used by smoothing transforms that want
   equal weight for all four OHLC components. Header gives
   **avgprice_label** (ABOVE_CLOSE / NEAR_CLOSE / BELOW_CLOSE /
   INSUFFICIENT_DATA for n<1) from `(avgprice − close) / close × 100`
   magnitude (>0.1% above_close, <-0.1% below_close, else near_close).

2. **No MEDPRICE snapshot.** TA-Lib's MEDPRICE function:
   `medprice = (high + low) / 2`. Pure range-median primitive,
   distinct from MIDPRICE (ADR-177 14-bar HHV/LLV midpoint) because
   MEDPRICE is a single-bar primitive with zero lookback, whereas
   MIDPRICE is a 14-bar smoothed range midpoint. Used as the input
   series for AWESOME / ALLIGATOR / FRACTAL detectors that want the
   instantaneous bar median rather than a smoothed band. Header gives
   **medprice_label** (ABOVE_MID / AT_MID / BELOW_MID /
   INSUFFICIENT_DATA) from close position relative to bar midpoint
   (>0.05% above_mid, <-0.05% below_mid, else at_mid).

3. **No TYPPRICE snapshot.** TA-Lib's typical-price function:
   `typprice = (high + low + close) / 3`. The canonical input for
   CCI, Money Flow Index, and VWAP. Distinct from MEDPRICE (excludes
   close) and WCLPRICE (double-weights close) — TYPPRICE equally
   weights the close alongside high/low without the open component,
   balancing range-center with actual settlement. Header gives
   **typprice_label** (ABOVE_CLOSE / NEAR_CLOSE / BELOW_CLOSE /
   INSUFFICIENT_DATA for n<1) from `(typprice − close) / close × 100`
   magnitude (>0.1% above_close, <-0.1% below_close, else near_close).

4. **No WCLPRICE snapshot.** TA-Lib's weighted-close function:
   `wclprice = (high + low + 2 × close) / 4`. Double-weights the
   close to emphasise settlement-biased smoothing. Distinct from
   TYPPRICE (`(H+L+C)/3` equal-weight close) and AVGPRICE
   (`(O+H+L+C)/4` equal-weight all). Used where close is more
   informative than range extremes (end-of-day signals, swing
   studies). Header gives **wclprice_label** (ABOVE_CLOSE /
   NEAR_CLOSE / BELOW_CLOSE / INSUFFICIENT_DATA for n<1) from
   `(wclprice − close) / close × 100` magnitude — semantically
   identical to AVGPRICE/TYPPRICE labels for symmetry.

5. **No VARIANCE snapshot.** TA-Lib's variance function:
   `σ² = Σ(x − μ)² / N` over N bars of close (population form, TA-Lib
   default at `optInNbDev=0`). The base statistical primitive
   underlying STDDEV (`σ = √σ²`), Z-score normalisation, and
   Bollinger/Acceleration bands. Distinct from realised-variance
   (sums of squared log returns) and EWMA-variance (exponential
   decay) because VARIANCE uses a flat 5-bar window over raw close
   values. Header gives **variance_label** (HIGH_VOL / ELEVATED /
   NORMAL / LOW_VOL / INSUFFICIENT_DATA for n<5) from CV
   (coefficient-of-variation = stddev / |mean| × 100) magnitude (>5%
   high_vol, >2% elevated, <0.5% low_vol, else normal).

## Decision

Add five optional per-symbol snapshot blocks to the Godel-parity
pipeline, each reusing the existing `research_historical_price` HP
cache and the standard research-table LAN-sync path (no new API
dependencies):

1. `research::AvgpriceSnapshot` + `compute_avgprice_snapshot` +
   `upsert_avgprice` + `get_avgprice` — serialised to
   `research_avgprice`.
2. `research::MedpriceSnapshot` + `compute_medprice_snapshot` +
   `upsert_medprice` + `get_medprice` — serialised to
   `research_medprice`.
3. `research::TypPriceSnapshot` + `compute_typprice_snapshot` +
   `upsert_typprice` + `get_typprice` — serialised to
   `research_typprice`.
4. `research::WclPriceSnapshot` + `compute_wclprice_snapshot` +
   `upsert_wclprice` + `get_wclprice` — serialised to
   `research_wclprice`.
5. `research::VarianceSnapshot` + `compute_variance_snapshot` +
   `upsert_variance` + `get_variance` — serialised to
   `research_variance`.

Schema version bumps to v68 via `create_research_tables_v68` which
wraps v67 and adds five new CREATE TABLE stanzas (plus indexes on
`updated_at`). All five tables register in `SYNCABLE_TABLES`,
`create_table_sql`, and `table_timestamp_column` so LAN sync
incrementally propagates them to peer terminals.

Native wiring adds five `BrokerCmd::Compute*Snapshot` variants, five
`BrokerMsg::*SnapshotMsg` variants, five `show_*_win` / `*_symbol` /
`*_snapshot` / `*_loading` field tuples on `App`, five tokio-spawned
broker handlers (load HP cache → compute → upsert → emit msg), five
palette alias blocks, five packet-emitter blocks under section 2.318+
of the research packet, five egui windows with Use-Chart / Load-Cached
/ Compute controls plus a striped Grid summary, and five `BrokerMsg`
match arms.

Palette aliases were selected to avoid collision with earlier rounds
(verified at implementation time):
`AVGPRICE | AVG_PRICE | OHLC_AVG | OHLCAVG | AVGOHLC`;
`MEDPRICE | MED_PRICE | HLMED | HLMEDIAN | RANGEMEDIAN`;
`TYPPRICE | TYP_PRICE | TYPICAL_PRICE | TYPICALPRICE | HLC3`;
`WCLPRICE | WCL_PRICE | WEIGHTED_CLOSE | WEIGHTEDCLOSE | HLCC4`;
`VARIANCE | VARIANCE_WIN | CLOSE_VARIANCE | CVARIANCE | VARWIN`. All
25 tokens are fresh — zero collisions with earlier rounds. `VAR` was
initially considered for VARIANCE but collides with the existing
`show_var_mult` alias (ADR-045 Value-at-Risk Multiplier) which claimed
it earlier in the dispatch; `VARWIN` used instead. `HLC3` and `HLCC4`
are canonical TradingView-style aliases for typical-price and
weighted-close respectively, useful for users migrating over.

The research packet emits fresh sub-blocks 2.318 AVGPRICE, 2.319
MEDPRICE, 2.320 TYPPRICE, 2.321 WCLPRICE, 2.322 VARIANCE after the
existing 2.317 ADXR sub-block from Round 65; INGESTED renumbers
2.318 → 2.323 and Sector peer 2.319 → 2.324. Envelope paragraph bumps
"~91-175 KB" → "~92-176 KB" with a description chain of the five new
primitives prepended.

## Consequences

- Packet scope grows by 10 k/v rows per symbol when all five
  snapshots are populated — roughly +200 bytes for AVGPRICE (O/H/L/C
  + avgprice + delta_pct), +190 bytes for MEDPRICE (H/L/C + medprice
  + delta_pct), +200 bytes for TYPPRICE (H/L/C + typprice + delta_pct),
  +200 bytes for WCLPRICE (H/L/C + wclprice + delta_pct), +260 bytes
  for VARIANCE (mean + variance + variance_prev + stddev + CV) — for
  a typical +1.05 KB per symbol.
- Schema is strictly additive; old peers running v67 continue to
  work (new tables are absent but none of the old tables change).
  LAN sync skips unknown tables via the whitelist.
- All five indicators share the HP cache with zero additional network
  dependencies. AVGPRICE / MEDPRICE / TYPPRICE / WCLPRICE are
  per-bar primitives (n≥1) — cheap; VARIANCE needs n≥5 and uses the
  population-form divisor `N` (not `N−1`) to match TA-Lib default.
- Like Round 65 + earlier rounds, the five roundtrip + five compute
  tests guard against regressions.

## Verification

1. **Engine tests:** `cargo test --package typhoon-engine --lib`
   passes with +10 Round 66 tests over Round 65's count.
2. **Native build:** `cargo build --package typhoon-native` completes
   cleanly with no warnings/errors.
3. **Unique palette tokens:** All 25 Round 66 palette tokens fresh —
   zero collisions with earlier rounds (verified against the 25 Round
   65 tokens and the cumulative R60..R64 set).
4. **LAN sync whitelist:** All five `research_*` tables registered
   in `SYNCABLE_TABLES`, `create_table_sql`, and
   `table_timestamp_column`; incremental sync uses the `updated_at`
   column.

## Packet envelope delta

Before Round 66: packet emitted 116 k/v rows across Round 60 + Round
61 + Round 62 + Round 63 + Round 64 + Round 65 additions. After Round
66: 126 k/v rows when all thirty-five Round 60..66 additions populate,
typical +1.05 KB per symbol on top of the +1.45 KB Round 65 added,
+1.45 KB Round 64 added, +1.45 KB Round 63 added, +1.45 KB Round 62
added, +1.40 KB Round 61 added, and +1.46 KB Round 60 added —
bringing the observed per-symbol envelope from ~91-175 KB to ~92-176
KB.
