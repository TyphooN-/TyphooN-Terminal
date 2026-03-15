# ADR-017: Drawing Tools (Trend Lines + Fibonacci)

**Status:** Implemented
**Date:** 2026-03-15

## Context

MT5 has 46 drawing objects. lightweight-charts v4.2 has no built-in drawing tools — only horizontal price lines. Traders need at minimum trend lines and Fibonacci retracements.

## Decision

Canvas overlay approach: a transparent `<canvas>` element positioned over the chart container, rendering drawings using `chart.timeScale().timeToCoordinate()` and `candleSeries.priceToCoordinate()` for coordinate conversion.

## Tools Implemented

### Trend Lines
- Press `l` to enter drawing mode (cursor → crosshair)
- Click two points on the chart to define the line
- Rendered as cyan (#00bcd4) solid line, 1.5px width
- Persists to localStorage, re-renders on scroll/zoom

### Fibonacci Retracement
- Press `f` to enter drawing mode
- Click high and low points
- Auto-draws 7 levels: 0%, 23.6%, 38.2%, 50%, 61.8%, 78.6%, 100%
- Each level colored distinctly with price labels
- Dashed horizontal lines extending to right edge

### Delete
- Press `x` to delete the most recent drawing (LIFO stack)

## Architecture

```
chart-container (position: relative)
├── lightweight-charts <canvas> (chart rendering)
└── draw-overlay <canvas> (pointer-events: none, z-index: 10)
    └── renderDrawings() — reads drawings[], converts time/price → x/y
```

Re-render triggered by:
- `subscribeVisibleLogicalRangeChange` (scroll/zoom)
- `ResizeObserver` (window resize)
- Drawing added/deleted

## Storage

`typhoon_drawings` in localStorage — JSON array of `{ type, p1: {time, price}, p2: {time, price} }`.

## Consequences

- **Pro**: Works with existing lightweight-charts (no plugins needed)
- **Pro**: Persistent across sessions
- **Pro**: Zero backend changes
- **Pro**: Trend lines + Fibonacci cover ~80% of manual trading drawing needs
- **Con**: Canvas overlay loses sub-pixel alignment on aggressive zoom (acceptable)
- **Con**: No click-to-select or drag-to-move drawings yet (delete is LIFO only)
- **Con**: Drawings are global, not per-symbol (future: key by symbol)
