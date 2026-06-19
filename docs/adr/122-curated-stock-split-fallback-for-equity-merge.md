# ADR-122: Curated Stock-Split Fallback for Equity-Merge Back-Adjustment

**Status:** Accepted | **Date:** 2026-06-13

Companion to **ADR-113** (Cross-Source Equity Bar Merge & Data Integrity).
Governs split input to `chart_back_adjust_bars_for_splits` /
`chart_known_splits_from_cache` in `typhoon-native/src/app/chart.rs`.

## Context

ADR-113 established that the raw best-rank trusted source (`kraken-equities` iapi)
returns **unadjusted** xStock bars, and that across a reverse split it would paint
pre-split history on the wrong scale unless corrected. The merge has two
corrections:

1. **Exact** — `chart_back_adjust_bars_for_splits` multiplies pre-split bars by a
   known split's `pre_split_factor`. This is the precise, source-independent fix
   and works even with no adjusted reference present.
2. **Inferred** — cross-source era reconciliation against Alpaca/Yahoo, a
   best-effort fallback when no explicit split is known.

The exact path is fed solely by `chart_known_splits_from_cache`, which reads the
FMP-sourced `research_stock_splits` table. **In the live cache that table did not
exist at all** (verified via `sqlite_master`): the FMP split scrape
(`research/scrape.rs`) only runs with an FMP key, and research tables are
LAN-synced — so on a node without an FMP key / without that table in the sync set,
splits were never populated. The exact correction was therefore **starved for
every symbol**, and WOK's **1-for-100 reverse split (2025-12-29)** fell entirely
to the inferred path, which cannot cleanly reconstruct a 100× step across a thin
microcap's sparse overlap. The result was the December discontinuity / scale
spikes on the merged chart that TradingView never shows.

The split facts ("WOK, 2025-12-29, factor 100") were documented in five code
comments but had **no runtime source** — the machinery was correct (the test
`known_split_back_adjusts_raw_kraken_equities_even_without_alpaca` passes) and
simply never handed the split.

## Decision

Add a curated, in-code split table as a fallback input, independent of FMP /
LAN sync:

- `chart_curated_known_splits(symbol)` — a static `(symbol, ex-date,
  pre_split_factor)` table. `pre_split_factor = old shares / new shares`
  (= 100 for a 1-for-100 reverse split); dates are the ex-date at 00:00 UTC.
- `chart_known_splits_from_cache` now unions the FMP-cache rows with the curated
  entries, **deduped by ex-date** so real FMP data takes precedence whenever it
  is present.

Seeded with `WOK → 2025-12-29 → 100.0`. This flows straight into the existing,
tested back-adjust path, so the merge re-materializes WOK's pre-split history onto
the post-split scale and matches TradingView. Merged bars rebuild from provider
rows on every load (`chart_load_merged_equity_bars_from_cache`), so the fix takes
effect on the next symbol load with no manual cache clear.

## Consequences

- WOK (and any future curated symbol) is split-corrected deterministically,
  offline, and without an FMP key — closing the gap ADR-113's exact path left
  open when its data source is empty.
- The curated table is opt-in per symbol; it covers only what is listed. It is a
  fallback, not a replacement for the FMP feed.
- **Follow-up (not addressed here):** general coverage still depends on populating
  `research_stock_splits` — i.e. running the FMP split scrape (FMP key) and
  including that table in the LAN sync set. Until then every non-curated split
  symbol relies on the inferred path alone.
- New entries must be verified against the issuer's actual corporate action
  (wrong date/factor mis-scales bars near the ex-date). Covered by
  `curated_known_splits_supply_wok_reverse_split`.
