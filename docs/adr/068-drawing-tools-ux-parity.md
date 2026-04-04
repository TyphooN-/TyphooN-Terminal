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

**Infrastructure:** `chart.selected_drawing: Option<usize>` field exists. `is_selected` computed per drawing in render loop. `sel_tint()` helper brightens color when selected, `effective_width` boosts stroke width. Hit-testing not yet implemented.

### 2. Drawing Move/Drag — IN PROGRESS
- [ ] Once selected, drag drawing body to move all points
- [ ] Offset all points by drag delta (bar_idx + price)

**Dependency:** Requires Gap #1 (selection) to be implemented first.

### 3. Drawing Resize via Control Points — PLANNED
- [ ] Selected drawing shows draggable handles at endpoints
- [ ] Drag handle to move that specific point (resize/reshape)

### 4. Line Width Control — DONE
- [x] Per-drawing line width (1-4px)
- [x] Width selector in toolbar
- [x] Persist width in session save/load
- [x] Applied in render loop via `effective_width` (d_width + selection boost)

### 5. Line Style Options — DONE
- [x] Solid, Dashed, Dotted per drawing
- [x] Style selector in toolbar
- [x] Persist style in session save/load
- [x] Applied via `draw_line()` helper in render loop (dashed/dotted segments computed)

### 6. Magnet/Snap Toggle — DONE
- [x] Toggle button in drawing toolbar (teal when active)
- [x] OHLC snap when placing drawings
- [x] Persisted in session

### 7. Cross-Timeframe Drawings — PLANNED
- [ ] Option to show drawings on all timeframes for same symbol
- [ ] Store drawings keyed by symbol (not per-chart instance)
- [ ] Coordinate mapping between timeframes

## Implementation — What's Wired

### Render Loop (draw_chart)
```rust
// Per-drawing state fetched at top of loop:
let (d_width, d_style) = chart.drawing_styles.get(draw_idx).copied().unwrap_or((1.5, LineStyle::Solid));
let is_selected = chart.selected_drawing == Some(draw_idx);
let sel_boost = if is_selected { 1.5 } else { 0.0 };
let effective_width = d_width + sel_boost;
let sel_tint = |c: Color32| -> Color32 { /* brighten toward cyan */ };

// draw_line() helper handles Solid/Dashed/Dotted:
draw_line(&painter, p1, p2, Stroke::new(effective_width, sel_tint(color)), d_style);
```

Applied to: HLine, TrendLine, VLine, Rectangle, Ray, Channel, ExtendedLine, HRay, CrossLine, ArrowLine, InfoLine, Pitchfork, and all remaining drawing types for width.

### Toolbar (drawing_toolbar)
- Magnet toggle button (teal = active)
- Width buttons: 1.0, 1.5, 2.0, 3.0 px
- Style buttons: ━ (solid), ╌ (dashed), ┈ (dotted)

### Session Persistence
- `draw_width`, `draw_line_style`, `snap_enabled` saved/restored
- `drawing_styles: Vec<(f32, LineStyle)>` parallel to `drawings` vec

## Remaining Work

### Gap #1 & #2: Hit-testing + drag
1. On mouse click in chart area (not on existing drawing): check distance to each drawing
2. Point-to-line distance < threshold (8px) → `selected_drawing = Some(idx)`
3. On drag of selected drawing: update all point coordinates by delta
4. ESC key → `selected_drawing = None`

### Gap #3: Control points
1. For selected drawing, render small squares at each endpoint
2. Mouse down on a control point → enter resize mode
3. Drag updates that specific point only

### Gap #7: Cross-TF drawings
- Requires storing drawings as `HashMap<String, Vec<Drawing>>` keyed by symbol
- All charts for the same symbol share the drawing store
- Coordinate mapping: bar_idx stored as timestamp offset, converted per-TF

## Consequences
- Line width + style now fully functional (Gaps 4 & 5 complete)
- Selection highlight infrastructure in place (partial Gap 1)
- Drawing count: 73 tools implemented; ~14 niche tools remaining (see ADR-017)
