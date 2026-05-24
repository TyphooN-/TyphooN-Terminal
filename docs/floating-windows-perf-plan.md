# Floating Windows Performance Improvement Plan (2026-05-24)

## Goal
Make **all floating windows** perform well during heavy bar sync and active charting, achieving O(1) or near-O(1) work per frame where possible.

## Identified Issues (from combing)

### 1. News & Research Window (Confirmed Major Offender)
- Renders full syndicated article list on every frame.
- No virtualization or result limiting.
- Duplicate articles across symbols (GDELT syndication).
- No `heavy_sync_in_progress` awareness.
- Full article body not stored on ingest.

### 2. Kraken Trade History / Open Orders
- Refreshes data on button, but list rendering may be expensive.
- No caching of formatted rows.
- Potential O(n) formatting on every draw.

### 3. Strategy / Backtest Windows
- May run GPU or heavy computation on every open/draw.
- Large result tables without pagination/caching.

### 4. Symbol Investigation Window
- Very large file with many sub-views.
- Likely does heavy technical analysis on draw.

### 5. SEC / Filings Window
- Mentioned caching in code, but needs verification.

### 6. General Patterns Across Windows
- Many windows do work unconditionally even when hidden or during heavy sync.
- Lack of early returns when `heavy_sync_in_progress`.
- No shared "light render mode" during sync.
- Expensive string formatting / RichText creation on every frame.
- Large monolithic `floating_windows.rs` (62k lines) — hard to maintain.

## Planned Improvements

### Phase 1: Core Infrastructure
- [ ] Add `heavy_sync_in_progress` awareness to `draw_floating_windows`.
- [ ] Create a shared "light mode" render path for all windows during sync.
- [ ] Add result limiting / virtualization helpers.

### Phase 2: News Window (Highest Impact)
- [ ] Deduplicate articles by URL hash (global table + many-to-many symbols).
- [ ] Store full article body on first ingest.
- [ ] Limit rendered results (e.g. top 50).
- [ ] Throttle or skip during heavy sync.

### Phase 3: Other Heavy Windows
- [ ] Audit and add early-outs or caching to Trade History, Strategy, Symbol Investigation.
- [ ] Cache formatted table rows where possible.

### Phase 4: Polish & Verification
- [ ] Ensure no warnings in release-max.
- [ ] Add unit tests for deduplication logic.
- [ ] Commit and push.

## Execution Order
1. Core infrastructure (heavy_sync flag propagation).
2. News window deduplication + full body storage.
3. Light-mode rendering for all windows.
4. Targeted fixes for other windows.
5. Final verification and push.

Status: In progress — combing complete, plan created.