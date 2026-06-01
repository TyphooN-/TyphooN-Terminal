# Chart Camera Rewrite Plan

Goal: replace the bolted-on free-look/free-pan behavior with a single TradingView-style chart camera model that works identically in single-chart and MTF grid views, survives background sync/reloads, and remains responsive under release-max heavy sync.

Current diagnosis:
- Camera state is spread across ChartState fields: visible_bars, view_offset, manual_view_override, price_pan, price_zoom, is_dragging, is_scaling_price, drag_start_*.
- Interaction handling is duplicated/divergent between pre-render legacy chart logic, MTF cell widgets, and single-chart widgets.
- Single-chart interactions are currently registered after draw; MTF interactions are registered before draw. This makes behavior and latency inconsistent.
- Reload/live-update code can still mutate viewport state unless manual_view_override is perfectly maintained.
- Horizontal pan uses integer right-edge bar offsets, which makes slow/free pan feel sticky. It needs fractional camera coordinates.
- Vertical pan is based on natural visible price span plus price_pan/price_zoom, but the model has no explicit viewport price center/range.

Architecture:
- Introduce a pure ChartCamera model, tested independently.
- Camera owns viewport: center_bar: f64, bars_visible: f64, price_center: Option<f64>, price_span: Option<f64>, follow_latest: bool.
- Rendering asks camera for visible index range and price range. It does not infer camera from scattered fields.
- Interaction code sends camera commands: begin_pan, pan_by_pixels, zoom_x_at, zoom_y_at, fit_price, follow_latest.
- Reload/live updates call camera.on_data_len_changed(old_len, new_len), which preserves manual view and only follows latest when follow_latest is true.
- Single-chart and MTF call the same interaction function before draw.

Implementation steps:
1. Add pure ChartCamera struct and unit tests in native/src/app.rs or a new native/src/app/chart_camera.rs.
2. Add compatibility methods to derive legacy visible_range/price range from ChartCamera while existing draw_chart still expects ChartState.
3. Replace handle_chart_body_drag_from_start with ChartCamera::pan_pixels using fractional bar coordinates.
4. Move single-chart interaction registration before draw so pan/zoom affects the same frame.
5. Make MTF and single-chart call one shared render_chart_interactive function.
6. Remove legacy pre-render camera pan state fields or leave as temporary compatibility only after tests prove parity.
7. Add tests for WOK-like tiny candle behavior: zoom Y then drag 10px/100px maps exactly to visible price span, no snap to latest, slow sub-bar drag accumulates.
8. Add release-max verification: cargo test chart_camera, cargo test chart_body, cargo check --profile release-max.

Acceptance criteria:
- Dragging chart body pans smoothly in both X and Y with no threshold/stickiness.
- After price-axis zoom, vertical pan moves by visible price range, not natural/autofit range.
- Horizontal pan supports sub-bar slow drag accumulation.
- Background reloads preserve manual camera position.
- End/latest/reset/fit are the only actions that re-enable latest following or reset camera.
- Single-chart and MTF grid behavior are identical.
