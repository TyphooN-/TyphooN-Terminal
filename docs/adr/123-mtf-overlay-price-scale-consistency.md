# ADR-123: MTF Overlay Price-Scale Consistency (MTF_MA / MultiKAMA)

**Status:** Accepted | **Date:** 2026-06-14

Companion to **ADR-004** (Multi-Timeframe Indicator Support), **ADR-113**
(Cross-Source Equity Bar Merge & Data Integrity) and **ADR-122** (Curated
Stock-Split Fallback). Governs `compute_mtf_sma` / `compute_multi_kama` in
`typhoon-native/src/app/chart.rs`.

## Context

The `MTF_MA` and `MultiKAMA` overlays draw moving averages computed on higher
timeframes (H1/H4/D1/W1/MN1) projected onto the host chart's x-axis. Both build
each line by resolving that timeframe's bars **independently**, trying cache key
prefixes in order — `merged:` → `kraken:` → `alpaca:` → `yahoo-chart:` → … —
and taking the first hit, validated only for **timeframe spacing**
(`chart_source_bars_match_timeframe`), never for **price scale**.

The displayed candles come from the `merged:` back-adjusted equity series
(ADR-113). When a timeframe has no `merged:` key, the overlay silently fell back
to a raw provider source on a different adjustment basis. Observed on **CDLX
[W1]**: the candles sat near ~$2 while two intraday MA lines were parked at
~$13–19, jagged, only on the right (recent) side of the chart.

Verified against the live cache:

```
merged:CDLX:{15Min,30Min,1Hour,1Day,1Week,1Month}   ← no merged 4Hour
alpaca:CDLX:{...,4Hour,...}
yahoo-chart:CDLX:{...}                                ← also no 4Hour
```

So the H4 line had no `merged:CDLX:4Hour` and fell through to
`alpaca:CDLX:4Hour`, which is on a different scale than the back-adjusted weekly
candles. The merged intraday splice can itself be mis-scaled when the exact
split back-adjust is starved — the `research_stock_splits` table is frequently
absent (ADR-122), so non-curated symbols rely on the inferred path alone. Either
way the **MA math is correct**; the **inputs are on inconsistent scales**.

(Distinct from a non-bug also visible on that chart: a slow W1/MN1 average
sitting *above* a −90% price is expected lag, not a scale fault.)

## Decision

Three guards in both overlays, belt-and-suspenders:

1. **Source consistency** — `ChartState::load_mtf_htf_bars` prefers the **same**
   cache source the candles loaded from (`self.primary_source`). When that source
   is known, the loader restricts to the canonical `{source}:{sym}:{tf}` key and
   returns `None` — **dropping the line** — if that timeframe is absent, rather
   than crossing to a differently-adjusted source. Only when the source is
   unknown (`""`) does it fall back to the legacy broad-prefix search.
2. **Price-scale sanity guard** — `ChartState::mtf_line_scale_ok` rejects a line
   whose values sit on a wildly different scale than price. It takes the
   **median** of `projected_value / close` over the matched bars and keeps the
   line only when that median is within `[1/4, 4]`. The median ignores brief
   excursions, so a legitimately lagging average (median near 1) survives while a
   persistently many-fold-off feed (CDLX ~7–15×, WOK ~10,000×) is dropped whole —
   no per-point gaps.
3. **Bar-level source-scale check** — `ChartState::htf_source_matches_host_scale`
   rejects an HTF source whose **bars** (not the projected average) sit off the
   host candle scale for more than 8% of their overlap (`value/host_close` outside
   `[1/4, 4]`). This catches a **localized** mis-scaled era that the median guard
   (#2) cannot — e.g. **YI**'s `1Hour`/`4Hour`/`30Min`/`15Min` ran ~10× high in the
   pre-split window (unadjusted intraday across a reverse split, with no intraday
   corroborator to correct it) while its `1Day`/`1Week` were adjusted, so the H1/H4
   `MTF_MA` + `MultiKAMA` lines plateaued ~10× above price for months. Because the
   line's median over its whole span stayed near 1, #2 kept it. Checking the source
   **bars** distinguishes this from a clean higher-TF whose *lagging* SMA legitimately
   rides above a crashed price (a `W1/200` over a −90% move): there the bars are
   on-scale (the lag lives only in the average), so the line is kept. Applied in
   `load_mtf_htf_bars`, so both overlays drop the bad source at load time.

Both `compute_mtf_sma` and `compute_multi_kama` route through the shared loader
and apply the guards at the push site, replacing their duplicated per-timeframe
prefix-resolution blocks.

## Consequences

- Overlay lines never mix price scales with the candles: a timeframe missing
  from the chart's source is skipped, and any surviving mis-scaled line is
  dropped by the median guard. The CDLX H4 line disappears at the source; any
  residual mis-scaled merged line is caught by the guard.
- Conservative by design: a TF available only under a *different but same-scale*
  source is now skipped rather than shown. We prefer a missing line to a
  wrong-scale one.
- The guard is relative to local price, so it catches mismatches the stock's own
  history would hide (CDLX traded at $13–19 in 2024, so a global min/max guard
  would not have flagged $13–19 lines in 2026).
- The bar-level check (#3) catches a *localized* mis-scaled era (YI's unadjusted
  intraday) that the whole-line median (#2) averages away, while still keeping a
  clean source whose only "excursion" is a lagging average over a crashed price.
  It is a render-time guard: the deeper cause is the unadjusted intraday merge
  with no corroborator, which a populated split feed (ADR-122) would back-adjust.

## Future work (#3 — not addressed here)

The robust *cause* fix is correct back-adjustment, which needs split data we do
not always have:

- Populate `research_stock_splits` generally — run the FMP split scrape (FMP key)
  and include that table in the data set — so the exact back-adjust path
  (ADR-122) is fed for every symbol, not just curated ones.
- Ensure `merged:` coverage exists for **every** overlay timeframe (notably
  4Hour, missing for CDLX) so source consistency keeps the line instead of
  dropping it.
- Until then, CDLX-class symbols can be added to the curated split table
  (`chart_curated_known_splits`, ADR-122) once their corporate actions are
  verified.
