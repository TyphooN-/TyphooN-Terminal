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
- GPU SMA routing exists and forming-bar updates now use partial last-bar GPU buffer writes when buffers are resident.
- Bookmap has a per-symbol window model, current snapshot/depth paths, symbol-filtered live L2 rendering, and Kraken spot-pair guards; retained streaming L2 history remains data/API/renderer scoped.

## Corrected notes from the old scratchpad

The old file contained obsolete transient chores, verification-pass reminders, and repeated progress-log notes. Those are not durable project tasks and have been removed.

The old M1/M5 note is also outdated. Current policy (see ADR-112/-128 and the
doc-drift checklist for the authoritative statement) is:

- M1/M5 are valid low-TF targets for Kraken Spot and Kraken Equities: live
  public-trades WS + forming-bar volume + WS-freshness + sync priority are
  wired for low-TF MTF use. Assist rows (Alpaca/Yahoo) remain non-target for
  those TFs.
- Sync Status drops rows for timeframes unchecked in *Enabled Sync TFs* from
  both the view and the percentages (ADR-130 §6).
- Logic must still not assume low-timeframe inputs are always available —
  provider data availability governs.

## Remaining real work

These are not leftovers from the floating-window performance pass. They are separate roadmap-scale epics and should be tracked in ADRs or feature plans, not as stale checkboxes in this scratchpad.

### Performance / rendering

- More indicator families routed through GPU compute where profiling proves value.
- Optional table-row caching for extremely large trade/order/history grids if profiling shows formatting dominates frame time.

### Market depth / Bookmap

- Dedicated retained depth history with a ring buffer and GPU-backed rendering.
- Broker-specific entitlement handling beyond currently guarded Kraken spot depth streams.

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
