# ADR-048: Drawing Tools & UX Parity with TradingView

**Status:** Complete
**Date:** 2026-04-03 | **Updated:** 2026-04-08

## Context

UX audit identified 7 gaps vs TradingView-style drawing tool experience. These are the most impactful missing features for daily trading workflow.

## Gaps

### 1. Post-Placement Drawing Selection — DONE (partial)
- [x] Click near a drawing to select it (8px hit threshold; HLine, VLine, TrendLine, HRay, Ray, Rectangle)
- [x] Selected drawing shows cyan tint + boosted stroke width
- [x] ESC to deselect, click empty space to deselect
- [x] Delete/Backspace removes selected drawing (syncs drawing_styles)
- [x] Hit-testing for remaining types: Pitchfork (3-point), Ellipse (normalized distance), GannFan, FibCircle, FibSpiral, FibWedge (segment distance)

**Implementation:** `chart.selected_drawing: Option<usize>`. On click in DrawMode::None, iterates drawings with point-to-line distance function, picks closest within 8px. Applied to 6 of the most common drawing types; others return `HIT_THRESHOLD + 1.0` (miss).

### 2. Drawing Move/Drag — DONE
- [x] When a drawing is selected and the user drags, `is_drawing_drag = true` (blocks chart pan)
- [x] Drag delta converted to (bar_delta, price_delta) from visible range / chart height
- [x] All 89 drawing types have correct field patterns matched and moved
- [x] `primary_released` clears `is_drawing_drag`, returning to normal pan behavior
- [x] `ChartState.is_drawing_drag: bool` field added

### 3. Drawing Control Points — DONE (visual)
- [x] Selected drawing shows cyan square handles at endpoints
- [x] Handles rendered for all multi-point drawing types (lines, pitchforks, patterns, etc.)
- [x] Drag handle to resize — control points are draggable. Click near a handle to enter resize mode (moves single point). Click elsewhere for whole-drawing drag.

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
- [x] Cross-TF toggle (TF button in toolbar): syncs HLines to all charts with same symbol
- [x] Price-based drawings (HLines) are TF-independent — auto-synced when toggle is ON
- [x] Per-chart drawing storage preserved; cross-TF sync copies on placement

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

## Formerly Deferred Drawing Work

### Gap #1 & #2: Hit-testing + drag — DONE
Implemented: click hit-testing selects the nearest drawing
(`chart.selected_drawing`, `app_runtime_central_panel.rs`), dragging a
selected drawing moves all its points, and Esc/click-away deselects.

### Gap #3: Control points — DONE
Implemented: selected drawings render small square drag handles at each
endpoint (`typhoon-chart-ui/src/render/chart_helpers/planning_overlays.rs`).
Handles, hit-testing, and resize all read one registry in
`typhoon-chart-ui/src/drawing_interaction.rs` — `drawing_anchors` enumerates a
variant's control points, `drawing_set_anchor` moves the grabbed one, and
`translate_drawing` moves the whole drawing when no anchor is grabbed
(dispatched from `typhoon-native/src/app/app_runtime_central_panel.rs`). Using
one registry for both draw and grab is what keeps every variant's handles
exactly where the hit test looks.

### Gap #7: Cross-TF drawings — still open (assessed 2026-07-04, deliberately not rushed)
- Requires storing drawings as `HashMap<String, Vec<Drawing>>` keyed by symbol
- All charts for the same symbol share the drawing store
- Coordinate mapping: bar_idx stored as timestamp offset, converted per-TF
- **Why it stays open:** the `Drawing` enum stores **absolute bar indices**
  (`usize`) in every one of the 89 variants (`typhoon-chart-ui/src/drawing.rs`).
  Cross-TF sharing needs a timestamp coordinate model, which means migrating
  every variant, every render + hit-test + drag path, and the
  session-persisted drawing data users already have (a bad migration corrupts
  saved drawings — user data). That refactor deserves its own dedicated pass
  with a session-migration plan and drawing-by-drawing render verification,
  not a tail-end change. Every other gap in this ADR is closed.

### 8. Pre-Placement Color Picker — DONE (2026-04-05)
- [x] 8-color palette in drawing toolbar (W/Y/G/R/C/M/O/B)
- [x] All 89 drawing types use `draw_color` instead of hardcoded constants
- [x] Color persisted in session save/load

### 9. Per-Drawing Property Editor — DONE (2026-04-05)
- [x] Right-click selected drawing → Drawing Color submenu (targets selected, not just last)
- [x] Right-click selected drawing → Drawing Width submenu (1-4px)
- [x] Right-click selected drawing → Drawing Style submenu (Solid/Dashed/Dotted)
- [x] Right-click selected drawing → Delete Selected button

### 10. Follow Latest Toggle — DONE (2026-04-05)
- [x] ⟫ button in drawing toolbar toggles auto-scroll
- [x] FOLLOW command in palette
- [x] Session-persisted

### 11. Keyboard Shortcuts — DONE (2026-04-05)
- [x] Alt+H = HLine, Alt+V = VLine, Alt+T = Trendline, Alt+F = Fibo, Alt+R = Rectangle
- [x] Alt+E = Eraser, Alt+C = Cycle chart type

## 2026-07-04 comb-over: unified painted-geometry interaction layer

A full audit found the drawing system's interaction paths structurally broken
despite the per-gap checkmarks above. Root causes and fixes, all shipped:

1. **Every interaction re-derived its own screen mapping** (selection
   hit-test, control-point pick, drag deltas, placement, brush sampling —
   five different hand-rolled mappings), and none matched the painted pixels:
   no log scale, no free-look camera (`visible_slot_count`/`first_bar_slot`),
   and the control-point centre had a padding-math bug. Any pan/zoom made
   placement land offset from the cursor and made handles/grabs miss.
   **Fix:** `PriceViewGeometry` (the exact geometry each frame paints,
   already used for SL/TP drags per ADR-132) now carries the bar mapping
   (`data_left`/`bar_w`/`start_idx`, `bar_to_x`/`x_to_bar_f`/`x_to_bar`) and
   is the *only* source for every drawing interaction.
2. **Renderers hid drawings whose endpoints scrolled off-viewport** (~75
   `>= start_idx && < end_idx` gates across the 9 annotation files — a long
   trendline vanished when you zoomed into its middle; one PriceLabel gate
   even aborted the whole annotation pass via `return Some(true)`).
   **Fix:** unconditional signed bar→x mapping everywhere; the painter clips.
3. **Partial per-variant interaction coverage**: hit-testing covered ~55/80
   variants (`_ =>` miss arm), control points 8/80, with a second divergent
   copy in the handle overlay. **Fix:** new `typhoon-chart-ui/src/
   drawing_interaction.rs` — `drawing_hit_distance`, `drawing_anchors` +
   `AnchorPos` (Data/PriceOnly/BarOnly), `drawing_set_anchor`,
   `translate_drawing`, `preview_drawing` — all **exhaustive matches with no
   wildcard arm**, so adding a Drawing variant fails compilation until its
   interaction is defined. Slope tools (Ray/GannFan) get a slope handle;
   channels get a width handle; position tools get entry/stop/target handles.
4. **Drag quantization**: per-frame `(dx/bar_w) as i64` truncated sub-bar
   deltas to zero — slow horizontal drags never moved; vertical used an
   unpadded/unzoomed range so drawings slipped under the cursor. **Fix:**
   anchor-based drag (`drawing_drag_last`): integer bars are consumed, the
   fractional remainder carries, resize places the anchor exactly under the
   cursor, and log-scale drags are exact.
5. **Dragging a drawing also panned the camera** (the body-drag widget never
   checked `is_drawing_drag`). **Fix:** gated; plus press-on-drawing now
   selects and grabs in one gesture (no select-click-first required).
6. **Live preview existed for only 4 of ~80 tools** — and parsed `Debug`
   strings per frame to guess pending points. **Fix:** every placement mode
   renders a dashed ghost of the exact would-be drawing through the same
   annotation chain (`draw_one_drawing_annotation` + `preview_drawing`),
   multi-click tools included (`preview_pending_points` mirror).
7. **UX polish:** crosshair cursor while a tool is armed; Move cursor when
   hovering a grabbable drawing; OHLC magnet snap is now pixel-based (8px)
   so it feels identical at any zoom and works on log scale; Esc cancels a
   placement AND clears pending multi-click buffers (they used to leak into
   the next pattern tool); Delete guards against a drifted styles vec.

**Follow-up fix (same day) — placement clicks dead; root cause PROVEN by a
headless probe (`armed_click_gate_over_central_panel`):** in egui 0.35,
`egui_wants_pointer_input()` is TRUE on **every frame** over a
`CentralPanel` (panel widgets register a Background layer, and the
root-rect test classifies that as "over egui"). The app used that method —
plus `egui_is_using_pointer()`, which is true on the chart-body widget's
own press frames — as an "over floating UI" test in four places, silently
disabling, since the egui upgrade: the crosshair (input handler nulled
`self.crosshair` every frame — which also starved the old placement gate),
scroll-zoom / `on_chart_body` hover gating, the whole pre-render press
routing (drawing grab, SL/TP claim clearing), and finally the placement
guard added in the first follow-up attempt. Fixes: floating UI is detected
by **layer order only** (windows = Middle, popups/menus = Foreground; the
chart is Background) plus `dragged_id()` for in-flight widget drags;
placement reads the raw `primary_clicked()` with the arming click excluded
via a `prev_draw_mode` frame check (tool must have been armed on an earlier
frame) instead of any wants-pointer test. The probe test permanently
documents the 0.35 behavior so the gate cannot be reintroduced. Also
unified `PRICE_AXIS_W` (98px): the interact-side widget split assumed 70px,
leaving a 28px strip of painted price axis that panned the chart instead of
scaling it.

Regression guards: `drawing_interaction` unit tests (geometry round-trip
including off-screen bars, off-screen-endpoint hit-testing, anchor/translate
invariants per shape family, slope/position `set_anchor` semantics, preview
completion for multi-click tools) + the existing render tests.

## Consequences
- All 7 original UX gaps complete (Gaps 1-7)
- 4 additional UX features added (color picker, property editor, follow toggle, shortcuts)
- 89 drawing tools with full TradingView-style support + 7 bonus tools
- All drawing colors user-configurable pre-placement
- Per-drawing right-click editing of color/width/style
- 2026-07-04: interaction layer unified on the painted geometry (see above) —
  select/drag/resize/erase work for every variant, drawings never vanish
  off-viewport, and every tool has a live placement preview
