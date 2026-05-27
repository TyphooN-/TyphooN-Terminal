# ADR-074: Comprehensive Performance / UX / Memory Pass

**Status:** Implemented
**Date:** 2026-04-12

## Context

Brainstormed audit identified 18 potential improvements across UX, O(1)/performance,
and memory footprint. This ADR documents the audit, implementation outcomes, and
rationale for items deferred or already in place.

## Implemented

### UX
- **Fuzzy command palette scoring** (`fuzzy_score()` in app.rs): subsequence
  matching with positional bonus. "voutl" → "VAROUTLIER", "SEC" still ranks
  "SEC Filings" above "Screener". Replaces old `contains()`-only filter.
  Recent commands MRU shown when input is empty (was already in place).
- **Sticky filing detail viewer**: New `sec_filing_pinned` + `sec_filing_content_for`
  fields. Pin button keeps document open while navigating other filings; document
  also shown when accession matches selected filing (no false-positive carryover).

### Performance
- **WAL checkpoint on quit** (`on_exit` hook): explicit
  `PRAGMA wal_checkpoint(TRUNCATE)` keeps WAL file small for next startup
  instead of relying on autocheckpoint(2000).
- **Bar cache timestamp index**: Already exists (`idx_bar_cache_ts ON
  bar_cache(timestamp)`), no change needed.
- **Lazy DARWIN analytics**: Already implemented via `full_refresh_done` flag —
  expensive computation runs once at startup, BG cycles do lightweight refresh.
- **GPU buffer pooling**: Already done via hoisted `ind_out_buffer` /
  `ind_params_buffer` in GpuContext (ADR-064).

### Memory
- **Filing content compression** (`store_filing_content`): zstd-3 compression on
  stored filing text (~80KB → ~8KB typical, 10x reduction). Decompression in
  `get_filing_content()` is transparent with legacy uncompressed fallback.
- **Insider trade SQL filter** (`get_all_insider_trades`): Limit to last 5
  years (1825 days) at SQL level. Older trades remain in DB and accessible
  via `get_insider_trades(ticker, days)`.
- **Bar cache LRU eviction** (`evict_lru` in cache.rs): 500 MB soft limit
  enforced from BG thread every 30min vacuum cycle. Skips entries newer than
  7 days to preserve hot data.
- **SEC symbol dedup** (sec_filing.rs): `Vec::contains` O(n²) → `HashSet` O(n)
  for portfolio symbol collection during scrape.

## Already In Place (No Change Needed)

- **Drop chart bars on close**: `charts.remove()` already drops `Vec<Bar>` via
  Rust ownership.
- **DARWIN deal pruning**: Deals are SQLite-backed, queried on demand — not
  held in memory permanently.
- **Right-click symbol context menu**: `PaletteContext` system already exists
  with right-click context-filtered palette on chart, DARWIN rows, watchlist.
- **Bar cache timestamp index**: Pre-existing migration in cache.rs.
- **mmap_size 256MB**: Already configured in PRAGMA block.
- **Lazy indicator recompute**: `indicators_dirty` flag already throttles
  recomputation to UI parameter changes + bar load.

## Deferred (Cost > Benefit)

- **Per-frame scope HashSet caching**: Existing callsites already cache locally
  per window. Adding mutable cache requires refactoring all callsites.
- **Arc\<str\> for sectors/industries**: Requires changing Fundamentals struct
  fields String → Arc\<str\> across engine + native + all callsites. ~50 unique
  sectors × few thousand fundamentals = ~150KB savings — not worth the
  invasive refactor.
- **GPU buffer pool keyed by bar count**: Major bind-group refactor for
  marginal gain. Existing reuse via hoisted buffers is already O(1) per
  dispatch.
- **Workspace presets (named layouts)**: session.json already saves window
  states. SAVE_TEMPLATE/LOAD_TEMPLATE exist for chart-specific templates.
  Named workspace UI is a separate feature.
- **Auto-scroll outlier table to highlights**: egui supports `scroll_to_me` but
  outliers are already displayed top-first by IQR severity. Marginal value.
- **Inline sparklines in tables**: Requires fetching 7-day prices per row.
  Better fit for dedicated screener view than inline grids.

## Tests

904 tests pass (216 mql5-compiler + 553 engine + 78 cli + 57 web-protocol). Zero warnings.
Zero production unwrap/expect violations (ADR-061 compliant).
