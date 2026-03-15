# ADR-011: Resizable Chart Panes

**Status:** Implemented
**Date:** 2026-03-15
**Context:** MT5 allows resizing indicator sub-windows by dragging their borders. TyphooN-Terminal needs the same for Fisher and BetterVolume panes.

## Decision

Three stacked chart panes (main, Fisher, Volume) with draggable resize handles between them. Each pane is a separate lightweight-charts instance with synced time scales and crosshairs.

## Implementation

- **Main chart**: `flex: 1` (takes remaining space)
- **Fisher pane**: `height: 120px` (fixed, resizable)
- **Volume pane**: `height: 100px` (fixed, resizable)
- **Resize handles**: 5px dividers between panes, green highlight on hover/drag
- **Min heights**: Main 100px, Fisher 60px, Volume 40px

## Pane Sync

- Time scale: `subscribeVisibleLogicalRangeChange()` — scroll one, all follow
- Crosshair: `subscribeCrosshairMove()` — hover one, all highlight same bar
- Resize: `ResizeObserver` triggers `chart.resize()` on all three instances
