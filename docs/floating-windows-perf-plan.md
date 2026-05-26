# Floating Windows Performance Status

Last reviewed: 2026-05-26

## Purpose

This document tracks the current state of floating-window performance work. It replaces the old 2026-05-24 scratchpad log, which mixed completed work, stale checkboxes, and roadmap notes in a way that made the repository look less resolved than it was.

## Current status

The high-impact floating-window performance pass is complete for the current scope.

Implemented and verified in code:

- App-level `heavy_sync_in_progress` state exists and is used to avoid expensive chart work during heavy sync.
- Empty-chart indicator computation has an O(1) early-out.
- News & Research uses cached full-article state, client-side filtering, headline grouping, bounded scroll panes, and resizable two-pane layout.
- News & Research can shrink and expand both horizontally and vertically without outer size caps.
- The right-panel news slice is bounded and filtered against the active focus set.
- Kraken trade/order-related windows and several other heavy floating windows have sync-aware guards or bounded scroll rendering.
- Yahoo fallback price plumbing exists for watchlist fallback display with source and age metadata.
- GPU SMA200 routing and forming-bar upload hooks exist; full incremental GPU upload remains a separate performance epic.
- Bookmap has a per-symbol window model and current snapshot/depth paths; true streaming L2 heatmap remains data/API/renderer scoped.

## Corrected notes from the old scratchpad

The old file contained obsolete work items such as "commit and push", "final verification pass", and repeated "continuing" progress notes. Those are not durable project tasks and have been removed.

The old M1/M5 note is also outdated. Current policy is:

- M1/M5 are not part of the critical MTF Grid/live-priority foreground sync lane.
- Broader sync defaults may still include lower timeframes when data exists and the provider supports them.
- Kraken Spot M1/M5 remains constrained by provider data availability; logic must not assume those low-timeframe inputs are always available.

## Remaining real work

These are not unfinished leftovers from the floating-window performance pass. They are separate roadmap-scale epics and should be tracked in ADRs or feature plans, not as stale checkboxes in this scratchpad.

### Performance / rendering

- True incremental GPU buffer updates for forming bars instead of the current forming-bar hook.
- More indicator families routed through GPU compute where profiling proves value.
- Optional table-row caching for extremely large trade/order/history grids if profiling shows formatting dominates frame time.

### Market depth / Bookmap

- Streaming L2 heatmap with dedicated retained depth history and GPU-backed rendering.
- Broker-specific entitlement handling for real-time L2 feeds.

### News / research

- Optional background refresh policy for watchlist fallback prices.
- Optional deeper article-body sentiment scoring after hydration.

### UX / workspace

- Named workspace/layout persistence.
- Session templates such as RTH/ETH/overnight overlays.
- Advanced trade-history filtering/export if user workflow demands it.

## Verification references

Relevant focused checks from recent work:

- `cargo check -p typhoon-native --quiet`
- `cargo check -p typhoon-native --tests --quiet`
- `cargo test -p typhoon-native sync_workset --quiet`
- `git diff --check`

## Maintenance rule

Do not use this file as a running session log. If a future pass adds work here, write current architecture, real remaining risks, and durable next actions only. Temporary progress updates belong in commits or the active task list, not in docs.
