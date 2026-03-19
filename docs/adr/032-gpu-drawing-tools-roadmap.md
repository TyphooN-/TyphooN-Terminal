# ADR-032: GPU Chart Completion + Drawing Tools Parity

**Status:** Accepted
**Date:** 2026-03-18

> **Goal:** Make the GPU chart engine the default renderer, with full drawing tool parity (MT5 46 + TradingView extras), all rendered via WebGL2 for maximum performance.

---

## Phase 1: GPU Drawing Primitives (Foundation)

Add a drawing system to the GPU chart engine. All drawings rendered in WebGL2 — no Canvas2D overlay needed.

### 1.1 New Shader: Drawing Shader

```glsl
// DRAW_VS — handles lines, filled shapes, dashed lines
layout(location = 0) in vec2 a_pos;        // price-space coordinates
layout(location = 1) in vec4 a_color;      // per-vertex RGBA
uniform vec2 u_price_range;
uniform vec2 u_time_range;
// → NDC transform (same as candle shader)
```

```glsl
// DRAW_FS — color pass-through with optional dash pattern
uniform float u_dash_size;   // 0.0 = solid, >0 = dashed
uniform float u_opacity;
```

### 1.2 Drawing Data Structure (Rust)

```rust
pub struct Drawing {
    pub draw_type: DrawingType,
    pub points: Vec<(f64, f64)>,  // (bar_index, price) pairs
    pub color: [f32; 4],          // RGBA
    pub line_width: f32,
    pub style: LineStyle,         // Solid, Dashed, Dotted
    pub fill_color: Option<[f32; 4]>,  // For filled shapes
    pub text: Option<String>,     // For labels
}

pub enum DrawingType {
    TrendLine, Ray, Segment, ExtendedLine, ArrowLine,
    Horizontal, Vertical,
    Rectangle, Triangle, Ellipse, Circle,
    Channel, ParallelChannel,
    Pitchfork, SchiffPitchfork,
    Fibonacci, FibFan, FibArcs, FibChannel, FibExtension,
    GannFan, GannLine, GannBox,
    CycleLines,
    ArrowUp, ArrowDown,
    PriceLabel, TextLabel,
    // New tools (Phase 3)
    RegressionChannel, StdDevChannel,
    RiskReward, PositionBox,
    DateRange, PriceRange,
    ElliottImpulse, ElliottCorrective,
    SpeedLines, AnchoredVWAP,
    Brush,
}
```

### 1.3 New GPU Methods

```rust
/// Add a drawing to the GPU chart.
pub fn add_drawing(&mut self, draw_type: u32, points: &[f64], color: &[f32], line_width: f32, style: u32, fill: &[f32]);

/// Remove drawing by index.
pub fn remove_drawing(&mut self, index: usize);

/// Clear all drawings.
pub fn clear_drawings(&mut self);

/// Hit-test: returns drawing index at canvas (x, y), or -1.
pub fn hit_test_drawing(&self, canvas_x: f64, canvas_y: f64, tolerance: f64) -> i32;

/// Render all drawings (called from render()).
fn render_drawings(&self);
```

### 1.4 Geometry Builders per Drawing Type

| Type | Geometry | Vertices |
|------|----------|----------|
| Lines (trend/ray/segment/extended/arrow/horizontal/vertical) | GL_LINES or GL_LINE_STRIP | 2-4 per line |
| Dashed lines | GL_LINES with gap pattern | N segments |
| Rectangles | GL_TRIANGLES (2 triangles for fill) + GL_LINE_LOOP (border) | 6 + 4 |
| Triangles | GL_TRIANGLES (fill) + GL_LINE_LOOP (border) | 3 + 3 |
| Circles/Ellipses | GL_TRIANGLE_FAN (fill) + GL_LINE_LOOP (border) | 64 segments |
| Fibonacci | Multiple GL_LINES + text labels | 7-13 lines |
| Channels | 2 GL_LINES + optional fill (2 triangles) | 4 + 6 |
| Pitchfork | 3 GL_LINES (median + 2 prongs) | 6 |
| Gann Fan | 9 GL_LINES from origin | 18 |
| Arrows | GL_TRIANGLES (filled arrowhead) | 3 |

### Files to Modify
- `gpu-charts/src/lib.rs` — Add Drawing struct, shader, geometry builders, render method
- `frontend/src/main.js` — Route drawing creation to GPU when in GPU mode

---

## Phase 2: GPU SL/TP Lines + Interactive Drawing

Replace lightweight-charts `createPriceLine()` with GPU-rendered draggable lines.

### 2.1 GPU Price Lines

```rust
pub fn add_price_line(&mut self, price: f64, color: &[f32], width: f32, style: u32, label: &str);
pub fn remove_price_line(&mut self, index: usize);
pub fn update_price_line(&mut self, index: usize, price: f64);
pub fn get_price_line_price(&self, index: usize) -> f64;
```

Rendered as horizontal GL_LINES spanning visible range, with text label via Canvas2D overlay.

### 2.2 Drawing Interaction (Frontend)

```
Mouse Events:
  mousedown → hit_test_drawing() → if hit: start drag
  mousemove → update drawing point coordinates → rebuild geometry → render
  mouseup   → finalize position → save to localStorage

  click (draw mode) → place anchor point
  click (draw mode, 2nd) → complete drawing → add_drawing()

  right-click → show properties panel (color, width, delete)
```

### 2.3 Drag Infrastructure

```rust
/// Move a drawing point to new coordinates.
pub fn move_drawing_point(&mut self, drawing_idx: usize, point_idx: usize, bar: f64, price: f64);

/// Move entire drawing by delta.
pub fn translate_drawing(&mut self, drawing_idx: usize, d_bar: f64, d_price: f64);
```

### Files to Modify
- `gpu-charts/src/lib.rs` — Price line system, drawing move/translate
- `frontend/src/main.js` — Replace createPriceLine with GPU equivalent, drag handler

---

## Phase 3: Missing Drawing Tools (24 new tools)

### 3.1 Quick Wins — Line Variants (< 30 min each)

| Tool | Implementation | Effort |
|------|---------------|--------|
| **Regression Channel** | Linear regression of closes in range + ±1σ/±2σ parallel lines | Medium — need least-squares fit |
| **Std Dev Channel** | Same as regression but with standard deviation bands | Medium — shares regression math |
| **Risk/Reward Zone** | Rectangle with entry line + colored zones (green profit / red loss) + R:R label | Easy — extend rectangle |
| **Position Box (Long/Short)** | Rectangle entry→TP (green) + entry→SL (red) with P&L labels | Easy — extend rectangle |
| **Date Range Highlighter** | Vertical filled band between two times | Easy — 2 triangles |
| **Price Range Zone** | Horizontal filled band between two prices | Easy — 2 triangles |
| **Speed Lines** | 3 lines from point at 1/3, 1/2, 2/3 of price range | Easy — 6 vertices |
| **Equidistant Channel** | Parallel channel with equal distance above and below center line | Easy — variant of channel |

### 3.2 Medium Effort (30-60 min each)

| Tool | Implementation | Effort |
|------|---------------|--------|
| **Elliott Wave (Impulse)** | 5-point zigzag with labels 1,2,3,4,5 at peaks/troughs | Medium — text overlay |
| **Elliott Wave (Corrective)** | 3-point zigzag with labels A,B,C | Medium — text overlay |
| **Gann Box** | Grid of Gann squares between two corners | Medium — grid lines |
| **Anchored VWAP** | Compute VWAP from anchor bar using cached volume data | Medium — needs volume access |
| **Brush/Freehand** | Store path as array of points, render as GL_LINE_STRIP | Medium — needs smooth sampling |
| **Arrow Variants** | Left, right, check, stop, star, diamond markers | Easy — small triangle/shape geometry |

### 3.3 Label Enhancements

| Tool | Implementation | Effort |
|------|---------------|--------|
| **Callout (text + arrow)** | Text label + line to anchor point | Easy |
| **Note (multi-line text box)** | Rich label with background, border, word wrap | Medium |
| **Emoji/Icon markers** | Star, diamond, flag, exclamation, question | Easy — Unicode in Canvas2D |

### Files to Modify
- `gpu-charts/src/lib.rs` — Geometry builders for each new type
- `frontend/src/main.js` — Click handlers, keyboard shortcuts, properties panel, command palette entries

---

## Phase 4: GPU Sub-Panes (Fisher, Volume, RSI)

Replace lightweight-charts separate panes with GPU-rendered sub-panes.

### 4.1 Multi-Pane Architecture

```rust
pub struct GpuChart {
    // Existing fields...
    panes: Vec<SubPane>,  // Additional indicator panes
}

pub struct SubPane {
    viewport_y: f32,       // Normalized Y start (0.0 = bottom)
    viewport_h: f32,       // Normalized height
    min_value: f64,
    max_value: f64,
    series: Vec<PaneSeries>,
}

pub enum PaneSeries {
    Line { values: Vec<f32>, color: [f32; 4] },
    Histogram { values: Vec<f32>, colors: Vec<[f32; 4]> },
    Baseline { values: Vec<f32>, above_color: [f32; 4], below_color: [f32; 4], base: f32 },
}
```

### 4.2 Pane Layout

```
┌────────────────────────────────────┐ ← viewport_y=0.30, viewport_h=0.70
│         Main Chart (candles)       │    (70% of canvas height)
│                                    │
├────────────────────────────────────┤ ← viewport_y=0.15, viewport_h=0.15
│     Fisher Transform Pane         │    (15% of canvas height)
├────────────────────────────────────┤ ← viewport_y=0.00, viewport_h=0.15
│        Volume Pane                │    (15% of canvas height)
└────────────────────────────────────┘
```

### 4.3 New GPU Methods

```rust
pub fn add_pane(&mut self, y: f32, h: f32) -> usize;
pub fn add_pane_line(&mut self, pane: usize, values: &[f64], r: f32, g: f32, b: f32, a: f32);
pub fn add_pane_histogram(&mut self, pane: usize, values: &[f64], colors: &[f32]);
pub fn set_pane_range(&mut self, pane: usize, min: f64, max: f64);
```

### Files to Modify
- `gpu-charts/src/lib.rs` — SubPane struct, viewport clipping, per-pane rendering
- `frontend/src/main.js` — Route Fisher/Volume/RSI data to GPU panes

---

## Phase 5: Full lightweight-charts Replacement

Once Phases 1-4 are complete, make GPU the default and remove lightweight-charts dependency.

### 5.1 Feature Parity Checklist

| Feature | lightweight-charts | GPU equivalent | Status |
|---------|-------------------|----------------|--------|
| Candlestick rendering | ✅ | ✅ Phase 0 (done) | Complete |
| Line/Bar/Renko charts | ✅ | ✅ Phase 0 (done) | Complete |
| Indicator overlays | ✅ | ✅ Phase 0 (done) | Complete |
| Price/time axis labels | ✅ | ✅ Phase 0 (Canvas2D) | Complete |
| Crosshair + tooltip | ✅ | ✅ Phase 0 | Complete |
| Scroll/zoom/pan | ✅ | ✅ Phase 0 | Complete |
| Drawing tools | ✅ (Canvas2D) | Phase 1 | **TODO** |
| SL/TP draggable lines | ✅ (createPriceLine) | Phase 2 | **TODO** |
| Sub-panes (Fisher/Vol) | ✅ (separate charts) | Phase 4 | **TODO** |
| Histogram series | ✅ | Phase 4 | **TODO** |
| Baseline series (fills) | ✅ | Phase 4 | **TODO** |

### 5.2 Migration Strategy

1. **Phase 1-2**: GPU drawings + SL/TP work alongside lightweight-charts (dual mode)
2. **Phase 3**: All 46+ drawing tools implemented in GPU
3. **Phase 4**: GPU sub-panes replace separate lightweight-charts instances
4. **Phase 5**: Remove lightweight-charts import, GPU is sole renderer
5. **Phase 5b**: Remove `frontend/node_modules/lightweight-charts/` (~170KB savings)

### 5.3 Risk Mitigation

- Keep lightweight-charts as fallback (selectable via chart type dropdown) until Phase 5 is battle-tested
- GPU mode already has "GPU Candles" selector — extend to "GPU Full" when all phases complete
- Per-phase testing: each phase is independently deployable and testable

---

## Implementation Order & Estimates

| Phase | Scope | Sessions | Dependencies |
|-------|-------|----------|--------------|
| **Phase 1** | Drawing primitives (shader + geometry + hit test) | 2-3 | None |
| **Phase 2** | SL/TP lines + drag interaction | 1-2 | Phase 1 |
| **Phase 3** | 24 new drawing tools | 2-3 | Phase 1 |
| **Phase 4** | GPU sub-panes (Fisher/Volume/RSI) | 2-3 | None (parallel with 2-3) |
| **Phase 5** | Remove lightweight-charts | 1 | Phases 1-4 |

**Total: ~8-12 sessions** to fully replace lightweight-charts with GPU rendering.

---

## Drawing Tool Count After Completion

| Category | Current | After Phase 3 | MT5 | TradingView |
|----------|---------|---------------|-----|-------------|
| Lines | 6 | 7 (+equidistant) | 6 | 8 |
| Fibonacci | 5 | 5 | 5 | 6 |
| Channels | 3 | 5 (+regression, stddev) | 5 | 5 |
| Shapes | 4 | 4 | 4 | 4 |
| Gann | 2 | 3 (+box) | 3 | 3 |
| Markers/Labels | 4 | 8 (+callout, note, variants) | 8 | 10 |
| Measurement | 1 | 4 (+R:R, position, date range) | 1 | 5 |
| Elliott | 0 | 2 | 2 | 2 |
| Special | 2 | 5 (+AVWAP, brush, price range) | 2 | 5 |
| **TOTAL** | **24** | **43** | **46** | **48** |

93% MT5 parity, 90% TradingView parity — with all rendering on GPU.
