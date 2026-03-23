# ADR-019: Multi-Timeframe Grid View

**Status:** Implemented
**Date:** 2026-03-15

## Context

In MT5, traders commonly view the same symbol across 4 timeframes simultaneously in a 2x2 grid (e.g., H4 / W1 / D1 / MN). Each chart has its own indicators, Fisher Transform sub-pane, and BetterVolume sub-pane. Double-clicking one chart expands it to fullscreen for closer analysis.

## Decision

Implement an MT5-style multi-timeframe grid view with selectable timeframes (2-5 panels), each containing an independent lightweight-charts instance with Fisher and BetterVolume sub-panes.

## Implementation

### UI Controls
- **"MTF Grid" button** in top bar — click to show timeframe checkboxes
- **Checkboxes**: H1, H4 (default), D1 (default), W1 (default), MN (default)
- **"Apply"** activates the grid; **"Close Grid"** returns to single chart

### Grid Layout
- CSS Grid with responsive column/row templates:
  - 2 panels: `1fr 1fr` (side by side)
  - 3-4 panels: `1fr 1fr` × `1fr 1fr` (2x2)
  - 5 panels: `1fr 1fr 1fr` × `1fr 1fr` (3+2)

### Per-Cell Content
Each grid cell contains:
1. **Candlestick chart** (lightweight-charts instance) — flex: 1
2. **Fisher sub-pane** — 50px height, color-segmented line
3. **BetterVolume sub-pane** — 40px height, colored histogram
4. **Label overlay** — symbol + timeframe (e.g., "AAPL H4")
5. **Overlay indicators**: SMA200 (gold) + KAMA (white)

### Data Loading
- Uses existing `barCache` — if timeframe is pre-cached (from background pre-fetch), loads instantly
- Falls back to `invoke("get_bars")` for uncached timeframes
- Each cell loads independently (no blocking between cells)

### Interaction
- **Double-click** a cell → fullscreen (position: fixed, z-index: 1500)
- **Double-click again** → restore to grid
- **"Close Grid"** button → dispose all chart instances, restore normal view

### State Management
- `mtfGridActive` flag — prevents normal chart operations while grid is active
- `mtfGridCells[]` — tracks all chart instances for resize/cleanup
- `ResizeObserver` on grid container for responsive resize
- All chart instances properly `.remove()`d on close to prevent memory leaks

## Evolution

- **Original**: Each grid cell had ~170 lines of inline indicator code in `loadMTFCellData()` with a hardcoded subset of indicators
- **ADR-042**: Unified indicator pipeline — grid cells now reuse the same `applyIndicators()` function as the main chart, with context parameter for per-cell chart instances. All indicators appear in grid cells automatically.
- **ADR-043**: GPU-first grid rendering — GPU is the primary renderer for grid cells, with lightweight-charts fallback for Fisher/Volume sub-panes only.

## Consequences

- **Pro**: Exact MT5 workflow — see 4 timeframes simultaneously with indicators
- **Pro**: Double-click fullscreen matches MT5 behavior
- **Pro**: Uses pre-cached bar data for instant loading
- **Pro**: Each cell has the full indicator set (whatever is checked in the registry)
- **Pro**: Single code path — indicator fixes apply to grid cells automatically
- **Con**: 4 charts × 3 instances each = 12 lightweight-charts instances (memory)
- **Con**: No crosshair sync between grid cells (independent charts)
