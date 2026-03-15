# ADR-014: Draggable SL/TP Lines

**Status:** Implemented
**Date:** 2026-03-15

## Context

In MT5, SL/TP lines can be double-clicked to enter drag mode, then dragged to a new price level. This is essential for manual trading — adjusting risk/reward by visually moving stop-loss and take-profit levels on the chart. The previous implementation used static `createPriceLine()` with `draggable: true`, but lightweight-charts v4.2 does not support native price line dragging.

## Decision

Implement custom drag-to-move using mouse events on the chart container, coordinated with lightweight-charts' `priceToCoordinate()` / `coordinateToPrice()` API.

## Implementation

### Hit Detection
- On mouse hover near an SL or TP line (within 8px), show `ns-resize` cursor
- Use `candleSeries.priceToCoordinate(line.options().price)` to convert line price to Y pixel

### Drag Activation
- **Double-click** near a line starts the drag (matches MT5 behavior)
- Single-click is not intercepted (preserves normal chart interaction)

### Drag Feedback
- During drag, `line.applyOptions({ price: newPrice })` updates the line position in real-time
- Price is derived from `candleSeries.coordinateToPrice(mouseY)`

### Drag Completion
- **Mouseup** finalizes the drag — syncs new price to Rust backend via `set_sl_level` / `set_tp_level`
- **Mouseleave** cancels the drag (safety — don't set price if mouse exits chart)

### State Flow
```
dblclick near line → draggingLine = "sl"|"tp"
  → mousemove: update line.applyOptions({ price })
  → mouseup: invoke("set_sl_level"/"set_tp_level"), clear drag state
  → mouseleave: cancel drag, clear state
```

## Consequences

- **Pro**: Matches MT5 workflow — double-click to grab, drag to adjust
- **Pro**: Visual feedback during drag (line moves with cursor)
- **Pro**: Backend synced on release (not during drag — no API spam)
- **Pro**: Resize cursor provides visual affordance
- **Con**: No native drag support in lightweight-charts — custom implementation
- **Con**: Double-click required (single-click would interfere with chart panning)
