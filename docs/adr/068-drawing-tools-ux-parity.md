# ADR-068: Drawing Tools & UX Parity with TradingView

**Status:** In Progress
**Date:** 2026-04-03

## Context

UX audit identified 7 gaps vs TradingView's drawing tool experience. These are the most impactful missing features for daily trading workflow.

## Gaps

### 1. Post-Placement Drawing Selection — IN PROGRESS
- [ ] Click near a drawing to select it (hit-test: point-to-line/shape distance)
- [ ] Selected drawing shows highlighted border + control point handles
- [ ] ESC to deselect, click away to deselect

### 2. Drawing Move/Drag — IN PROGRESS
- [ ] Once selected, drag drawing body to move all points
- [ ] Offset all points by drag delta (bar_idx + price)

### 3. Drawing Resize via Control Points — PLANNED
- [ ] Selected drawing shows draggable handles at endpoints
- [ ] Drag handle to move that specific point (resize/reshape)

### 4. Line Width Control — IN PROGRESS
- [ ] Per-drawing line width (1-5px)
- [ ] Width selector in toolbar or right-click menu
- [ ] Persist width in session save/load

### 5. Line Style Options — IN PROGRESS
- [ ] Solid, Dashed, Dotted per drawing
- [ ] Style selector in right-click menu
- [ ] Persist style in session save/load

### 6. Magnet/Snap Toggle — DONE
- [ ] Toggle button to enable/disable OHLC snap
- [ ] Visual indicator when snap is active
- [ ] Persisted in session

### 7. Cross-Timeframe Drawings — PLANNED
- [ ] Option to show drawings on all timeframes for same symbol
- [ ] Store drawings keyed by symbol (not per-chart instance)
- [ ] Coordinate mapping between timeframes

## Implementation Strategy

Wrap `Drawing` enum in `DrawingEntry` struct:
```rust
struct DrawingEntry {
    drawing: Drawing,
    width: f32,       // 1.0-5.0 (default 1.5)
    style: LineStyle,  // Solid, Dashed, Dotted
}
```

Change `chart.drawings: Vec<Drawing>` → `Vec<DrawingEntry>`.

Selection state stored transiently (not persisted):
```rust
selected_drawing: Option<usize>,  // index into drawings vec
```

## Consequences
- Major UX improvement for daily trading workflow
- Drawing refactor touches rendering, serialization, placement, deletion
- Cross-TF drawings require architectural change (shared drawing store)
