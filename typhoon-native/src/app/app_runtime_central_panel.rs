use super::*;

use typhoon_chart_ui::drawing_interaction::{
    drawing_anchors, drawing_hit_distance, drawing_set_anchor, translate_drawing,
};
use crate::app::chart_ops::{
    chart_company_name_catalog, low_timeframe_no_data_symbols, mtf_canvas_grid_cols,
    mtf_canvas_grid_rows, mtf_flat_chart_indices, mtf_visible_chart_groups_filtered,
};

impl TyphooNApp {
    pub(crate) fn render_central_panel(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        pointer_over_floating: bool,
    ) {
        let available = ui.available_rect_before_wrap();
        let chart_company_names = chart_company_name_catalog(
            &self.all_broker_assets,
            &self.kraken_equity_names,
            self.primary_broker,
        );

        // ── Price axis rect (right 70px of chart — TradingView-style scale) ──
        let price_axis_w = 70.0_f32;
        let price_axis_rect = egui::Rect::from_min_max(
            egui::pos2(available.right() - price_axis_w, available.top()),
            available.max,
        );
        let chart_body_rect = egui::Rect::from_min_max(
            available.min,
            egui::pos2(available.right() - price_axis_w, available.bottom()),
        );

        let hover_pos = ctx.input(|i| i.pointer.hover_pos().unwrap_or_default());
        // Don't interact with chart when pointer is over a floating window or egui wants pointer
        let egui_hover = ctx.egui_wants_pointer_input()
            || ctx.egui_is_using_pointer()
            || ctx.dragged_id().is_some();
        let layer_at_hover = ctx.layer_id_at(hover_pos);
        let hover_over_window = egui_hover
            || layer_at_hover
                .map(|id| id.order == egui::Order::Middle || id.order == egui::Order::Foreground)
                .unwrap_or(false);
        let on_price_axis = price_axis_rect.contains(hover_pos) && !hover_over_window;
        let on_chart_body = chart_body_rect.contains(hover_pos) && !hover_over_window;

        // Scroll → zoom (only when not over a floating window, skip in MTF mode — cells handle own zoom)
        let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta != 0.0 && !hover_over_window && !self.mtf_enabled {
            let body_local_x = (hover_pos.x - chart_body_rect.left()).max(0.0);
            let body_local_y = (hover_pos.y - chart_body_rect.top()).max(0.0);
            let axis_local_y = (hover_pos.y - price_axis_rect.top()).max(0.0);
            if on_price_axis {
                // Scroll on price axis → vertical zoom (TradingView style: squish/expand), centered on mouse price
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    let target_price = chart.price_from_y(axis_local_y, price_axis_rect.height());
                    let pct = (scroll_delta * 0.002).clamp(-0.08, 0.08);
                    let factor = (1.0 + pct as f64).clamp(0.1, 20.0);
                    chart.zoom_chart_price_around(factor, target_price);
                }
            } else if on_chart_body {
                let ctrl_held = ctx.input(|i| i.modifiers.ctrl);
                if ctrl_held {
                    // Ctrl+scroll on chart → vertical zoom (progressive), mouse-centered
                    if let Some(chart) = self.charts.get_mut(self.active_tab) {
                        let target_price = chart.price_from_y(body_local_y, chart_body_rect.height());
                        let pct = (scroll_delta * 0.002).clamp(-0.08, 0.08);
                        let factor = (1.0 + pct as f64).clamp(0.1, 20.0);
                        chart.zoom_chart_price_around(factor, target_price);
                    }
                } else {
                    // Scroll on chart → horizontal zoom (time axis, progressive), mouse-centered on bar under cursor
                    for chart in &mut self.charts {
                        let target_bar = chart.bar_from_x(body_local_x, chart_body_rect.width());
                        let factor = 1.0 + (scroll_delta as f64 * 0.002).clamp(-0.08, 0.08);
                        chart.zoom_chart_bars_around(factor, target_bar.max(0.0));
                    }
                }
            }
        }

        // Double-click while placing polyline → finalize it
        if ctx.input(|i| {
            i.pointer
                .button_double_clicked(egui::PointerButton::Primary)
        }) && self.draw_mode == DrawMode::PlacingPolyline
        {
            if self.polyline_points.len() >= 2 {
                let pts = std::mem::take(&mut self.polyline_points);
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    chart.drawings.push(Drawing::Polyline {
                        points: pts,
                        color: self.draw_color,
                    });
                }
            }
            self.polyline_points.clear();
            self.draw_mode = DrawMode::None;
        }

        // Double-click while placing path → finalize it
        if ctx.input(|i| {
            i.pointer
                .button_double_clicked(egui::PointerButton::Primary)
        }) && self.draw_mode == DrawMode::PlacingPath
        {
            if self.polyline_points.len() >= 2 {
                let pts = std::mem::take(&mut self.polyline_points);
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    chart.drawings.push(Drawing::PathDraw {
                        points: pts,
                        color: self.draw_color,
                    });
                }
            }
            self.polyline_points.clear();
            self.draw_mode = DrawMode::None;
        }

        // Brush: accumulate points while dragging, finalize on mouse release
        if self.draw_mode == DrawMode::PlacingBrush {
            let is_down = ctx.input(|i| i.pointer.primary_down());
            if is_down {
                if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
                    if on_chart_body {
                        if let Some(chart) = self.charts.get(self.active_tab) {
                            // Freehand samples come from the exact painted
                            // geometry (the old mapping ignored pan/zoom/log,
                            // so strokes landed offset from the cursor).
                            if let Some(g) = chart.last_price_geometry {
                                if !chart.bars.is_empty() {
                                    let max_bar = chart.bars.len().saturating_sub(1);
                                    let abs_idx = g.x_to_bar(pos.x, max_bar);
                                    let price = g.price_from_y(pos.y);
                                    if self
                                        .brush_points
                                        .last()
                                        .map(|last| *last != (abs_idx, price))
                                        .unwrap_or(true)
                                    {
                                        self.brush_points.push((abs_idx, price));
                                    }
                                }
                            }
                        }
                    }
                }
            } else if !self.brush_points.is_empty() {
                // Mouse released → finalize brush
                let pts = std::mem::take(&mut self.brush_points);
                if pts.len() >= 2 {
                    if let Some(chart) = self.charts.get_mut(self.active_tab) {
                        chart.drawings.push(Drawing::Brush {
                            points: pts,
                            color: self.draw_color,
                        });
                    }
                }
                self.draw_mode = DrawMode::None;
            }
        }

        // Double-click → reset zoom/pan
        if ctx.input(|i| {
            i.pointer
                .button_double_clicked(egui::PointerButton::Primary)
        }) && self.draw_mode == DrawMode::None
        {
            if on_price_axis {
                // Double-click price axis → auto-fit vertical only
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    chart.price_zoom = 1.0;
                    chart.price_pan = 0.0;
                    chart.manual_view_override = false;
                    chart.reset_camera_from_legacy();
                }
            } else if on_chart_body {
                if self.mtf_enabled {
                    // MTF body double-clicks are handled per-cell below so the
                    // clicked chart, not the previously active chart, becomes
                    // the single-chart focus.
                } else if self.charts.len() > 1 {
                    // Double-click in single mode with multiple tabs → return to MTF grid.
                    // Queue empty cells for the off-thread deferred loader instead of a
                    // synchronous `try_load` loop here — loading all grid cells on the
                    // render thread on this one click was a multi-second freeze.
                    self.mtf_enabled = true;
                    self.queue_empty_charts_for_load();
                    self.log.push_back(LogEntry::info("MTF grid restored"));
                } else {
                    // Single chart, no MTF → double-click body resets to follow-latest (TV/MT5)
                    if let Some(chart) = self.charts.get_mut(self.active_tab) {
                        chart.reset_to_follow_latest();
                    }
                }
            }
        }

        // Drag interactions — only when pointer is NOT over a floating window
        let pointer = ctx.input(|i| i.pointer.clone());
        let drag_delta = ctx.input(|i| i.pointer.delta());
        // Block chart interaction when ANY egui widget/window is using the pointer
        let egui_wants_pointer = ctx.egui_wants_pointer_input() || ctx.egui_is_using_pointer();
        let anything_dragged = ctx.dragged_id().is_some();
        let layer_id_at_pointer = ctx.layer_id_at(pointer.hover_pos().unwrap_or_default());
        let pointer_over_window = egui_wants_pointer
            || anything_dragged
            || layer_id_at_pointer
                .map(|id| id.order == egui::Order::Middle || id.order == egui::Order::Foreground)
                .unwrap_or(false);

        // ── SL/TP line grab + drag — ACTIVE chart only, single AND MTF ──────
        // One owner for the whole gesture: hit-test against the exact
        // PriceViewGeometry the active chart painted with last frame (the old
        // per-loop re-derivation ignored sub-panes/time-axis/log-scale/camera,
        // so grabs missed — "dragging is broken"), and only when the lines
        // actually belong to the active chart's symbol (ADR-132). The chart
        // body-drag paths below yield while a line drag is live.
        if pointer.primary_released() {
            self.dragging_sl = false;
            self.dragging_tp = false;
        }
        if self.draw_mode == DrawMode::None {
            if pointer.primary_pressed() && !pointer_over_window {
                if let Some(press_pos) = pointer.press_origin() {
                    let in_active_pane = self
                        .charts
                        .get(self.active_tab)
                        .and_then(|c| c.last_price_geometry)
                        .map(|g| g.chart_rect.contains(press_pos))
                        .unwrap_or(false);
                    if in_active_pane {
                        let _ = self.try_begin_sl_tp_drag(self.active_tab, press_pos.y);
                    }
                }
            }
            if (self.dragging_sl || self.dragging_tp)
                && self.apply_sl_tp_drag(self.active_tab, drag_delta.y)
            {
                self.sync_trade_line_inputs();
            }
        }
        let sl_tp_line_drag_live = self.dragging_sl || self.dragging_tp;

        // Skip drag in MTF mode — individual cells handle their own interaction
        if !self.mtf_enabled {
            for (chart_idx, chart) in self.charts.iter_mut().enumerate() {
                if chart_idx != self.active_tab {
                    continue;
                }
                if pointer.primary_pressed() {
                    let press_pos = pointer.press_origin().unwrap_or_default();
                    // Price-axis scaling is handled by the dedicated widget below
                    // (`single_chart_price_axis`). Don't double-handle the press here —
                    // egui's hit-test on that widget already routes correctly even when
                    // a floating window overlaps the right scale strip. We only need to
                    // intercept the press so it doesn't fall through to the chart-pan
                    // branch and start dragging the chart instead.
                    if price_axis_rect.contains(press_pos) {
                        // No-op: widget owns the scale gesture.
                    } else if available.contains(press_pos) && !pointer_over_window {
                        // SL/TP line grabs are handled by the unified pre-pass
                        // above (exact painted geometry); when it claimed the
                        // press, just clear any competing chart drag state.
                        if sl_tp_line_drag_live {
                            chart.is_dragging = false;
                            chart.is_drawing_drag = false;
                            chart.is_scaling_price = false;
                        } else if self.draw_mode == DrawMode::None {
                            // Press directly on a drawing: select it and grab
                            // it in one gesture (TradingView-style), using the
                            // exact painted geometry — the old path required a
                            // separate select click first and re-derived its
                            // own (buggy) screen mapping for the handles.
                            chart.dragging_cp = None;
                            let mut grabbed = false;
                            if let Some(g) = chart.last_price_geometry {
                                if g.chart_rect.contains(press_pos) {
                                    const GRAB_PX: f32 = 8.0;
                                    let mut best: Option<(usize, f32)> = None;
                                    for (i, d) in chart.drawings.iter().enumerate() {
                                        let dist = drawing_hit_distance(d, press_pos, &g);
                                        if dist <= GRAB_PX
                                            && best.map(|(_, bd)| dist < bd).unwrap_or(true)
                                        {
                                            best = Some((i, dist));
                                        }
                                    }
                                    if let Some((idx, _)) = best {
                                        chart.selected_drawing = Some(idx);
                                        // Anchor handle under the press → resize
                                        // that point; otherwise whole-drawing drag.
                                        if let Some(d) = chart.drawings.get(idx) {
                                            for (cp_idx, a) in
                                                drawing_anchors(d).iter().enumerate()
                                            {
                                                let sp = a.to_screen(&g);
                                                let dist = ((press_pos.x - sp.x).powi(2)
                                                    + (press_pos.y - sp.y).powi(2))
                                                .sqrt();
                                                if dist < 10.0 {
                                                    chart.dragging_cp = Some(cp_idx);
                                                    break;
                                                }
                                            }
                                        }
                                        chart.drawing_drag_last = Some((
                                            g.x_to_bar_f(press_pos.x) ,
                                            g.price_from_y(press_pos.y),
                                        ));
                                        grabbed = true;
                                    }
                                }
                            }
                            if grabbed {
                                chart.is_drawing_drag = true;
                                chart.is_dragging = false;
                                chart.is_scaling_price = false;
                            }
                            // No drawing under the press → the dedicated
                            // body-drag widget owns the camera pan.
                        } else {
                            // Normal chart pan is owned exclusively by the dedicated
                            // `single_chart_body_drag` widget registered after drawing.
                            // This legacy pre-render path used to start a second camera
                            // drag for every chart tab, then the widget path mutated the
                            // active chart again in the same gesture. That split-brain
                            // ownership made TradingView-style free-look feel random or
                            // completely dead under release builds.
                        }
                    }
                } else if pointer.primary_released() {
                    // Stop dragging when mouse released
                    chart.is_dragging = false;
                    chart.is_drawing_drag = false;
                    chart.is_scaling_price = false;
                    chart.dragging_cp = None;
                    chart.drawing_drag_last = None;
                    chart.drag_start = None;
                    self.dragging_sl = false;
                    self.dragging_tp = false;
                } else if pointer_over_window
                    && !chart.is_scaling_price
                    && !chart.is_dragging
                    && !chart.is_drawing_drag
                {
                    // Cancel pending drag state if pointer moves over a floating window
                    // but don't interrupt active drags/scaling
                    chart.drag_start = None;
                }

                // SL/TP line drag price updates happen in the unified pre-pass
                // above (exact painted geometry, active chart only).

                // Price axis drag → handled by the dedicated `single_chart_price_axis`
                // widget below. Don't re-apply zoom here or every drag delta double-counts.

                // Drawing drag — anchor-based on the exact painted geometry.
                // Resizes place the grabbed anchor directly under the cursor;
                // whole-drawing moves consume integer bar deltas and carry the
                // fractional remainder in `drawing_drag_last`, so slow drags
                // are no longer eaten by per-frame `as i64` truncation, and
                // log-scale/free-look-camera charts drag 1:1 with the pointer.
                if chart.is_drawing_drag && (drag_delta.x.abs() > 0.0 || drag_delta.y.abs() > 0.0) {
                    if let (Some(sel), Some(g), Some(cur)) = (
                        chart.selected_drawing,
                        chart.last_price_geometry,
                        ctx.input(|i| i.pointer.interact_pos()),
                    ) {
                        let max_bar = chart.bars.len().saturating_sub(1);
                        let bar_f = g.x_to_bar_f(cur.x);
                        let price = g.price_from_y(cur.y);
                        match chart.drawing_drag_last {
                            None => chart.drawing_drag_last = Some((bar_f, price)),
                            Some((last_bar, last_price)) => {
                                if let Some(cp_idx) = chart.dragging_cp {
                                    if let Some(d) = chart.drawings.get_mut(sel) {
                                        drawing_set_anchor(
                                            d,
                                            cp_idx,
                                            g.x_to_bar(cur.x, max_bar),
                                            price,
                                            max_bar,
                                        );
                                    }
                                    chart.drawing_drag_last = Some((bar_f, price));
                                } else {
                                    let bar_delta = (bar_f - last_bar).round() as i64;
                                    let price_delta = price - last_price;
                                    if let Some(d) = chart.drawings.get_mut(sel) {
                                        translate_drawing(d, bar_delta, price_delta, max_bar);
                                    }
                                    chart.drawing_drag_last =
                                        Some((last_bar + bar_delta as f64, price));
                                }
                            }
                        }
                    }
                }

                // Normal chart body pan is handled by `single_chart_body_drag`
                // after drawing. Keep this legacy pre-render block limited to
                // SL/TP and drawing-object drags; applying camera pan here races
                // the widget-owned gesture and can move the active chart twice.
            }
        } // end !mtf_enabled drag guard

        // Console is rendered as egui::Window after CentralPanel (see below)

        // ── chart drawing ────────────────────────────────────────────────
        let crosshair = self.crosshair;
        let flags = self.indicator_flags();
        let show_rsi = self.show_rsi;
        let show_fisher = self.show_fisher;
        let show_macd = self.show_macd;
        let show_volume_pane = self.show_volume_pane;
        let show_stochastic = self.show_stochastic;
        let show_adx = self.show_adx;
        let show_cci = self.show_cci;
        let show_williams_r = self.show_williams_r;
        let show_obv = self.show_obv;
        let show_momentum = self.show_momentum;
        let show_cmo = self.show_cmo;
        let show_qstick = self.show_qstick;
        let show_disparity = self.show_disparity;
        let show_bop = self.show_bop;
        let show_stddev = self.show_stddev;
        let show_mfi = self.show_mfi;
        let show_trix = self.show_trix;
        let show_ppo = self.show_ppo;
        let show_ultosc = self.show_ultosc;
        let show_stochrsi = self.show_stochrsi;
        let show_var_oscillator = self.show_var_oscillator;
        let show_better_volume = self.show_better_volume;
        let show_ehlers_ebsw = self.show_ehlers_ebsw;
        let show_ehlers_cyber = self.show_ehlers_cyber;
        let show_ehlers_cg = self.show_ehlers_cg;
        let show_ehlers_roof = self.show_ehlers_roof;
        let render_cache = self.cache.clone();
        // SL/TP lines render ONLY on the active chart whose symbol owns them
        // (ADR-132) — painting the global lines into every MTF cell invited
        // reading them as levels for other symbols/timeframes.
        let sl_price = self.sl_price;
        let tp_price = self.tp_price;
        let trade_lines_on_active = self.trade_lines_active_on(self.active_tab);
        let trade_lines_tab = self.active_tab;
        for chart in &mut self.charts {
            let symbol = regulatory_alerts::normalize_regulatory_symbol(&chart.symbol);
            chart.regulatory_alerts = self
                .bg
                .regulatory_alerts_by_symbol
                .get(&symbol)
                .cloned()
                .unwrap_or_default();
        }
        let active_sub_pane_count = [
            show_rsi,
            show_fisher,
            show_macd,
            show_volume_pane,
            show_stochastic,
            show_adx,
            show_cci,
            show_williams_r,
            show_obv,
            show_momentum,
            show_cmo,
            show_qstick,
            show_disparity,
            show_bop,
            show_stddev,
            show_mfi,
            show_trix,
            show_ppo,
            show_ultosc,
            show_stochrsi,
            show_var_oscillator,
            show_better_volume,
            show_ehlers_ebsw,
            show_ehlers_cyber,
            show_ehlers_cg,
            show_ehlers_roof,
            self.show_squeeze,
        ]
        .into_iter()
        .filter(|enabled| *enabled)
        .count() as u8;

        if self.mtf_enabled {
            // Central MTF is a flat chart stream: exactly two columns, then
            // additional rows downward as cells become available. The right panel
            // can group by symbol; the canvas must not allocate a separate vertical
            // band per symbol because sparse/no-data groups create a waterfall.
            while self.mtf_visible.len() < self.charts.len() {
                self.mtf_visible.push(true);
            }
            let suppressed_mtf_symbols = low_timeframe_no_data_symbols(&self.unresolvable_pairs);
            // The central MTF grid tiles ONLY the user's open tabs — never the
            // hidden per-timeframe backing charts that the right-panel MTF Grid
            // indicator dots load. Those backing charts (`show_in_tab_bar == false`)
            // stay invisible here, so clicking MTF shows exactly the symbols and
            // timeframes the user has tabbed while the dot panel keeps its full
            // multi-timeframe coverage. AND-in `mtf_visible` so the per-tab
            // visibility checkboxes still hide individual cells.
            let tabbed_mtf_visible: Vec<bool> = self
                .charts
                .iter()
                .enumerate()
                .map(|(i, chart)| {
                    chart.show_in_tab_bar && self.mtf_visible.get(i).copied().unwrap_or(true)
                })
                .collect();
            let mtf_groups = mtf_visible_chart_groups_filtered(
                &self.charts,
                &tabbed_mtf_visible,
                &suppressed_mtf_symbols,
            );
            let mtf_indices = mtf_flat_chart_indices(&mtf_groups);
            if mtf_indices.is_empty() {
                ui.painter().text(
                    available.center(),
                    egui::Align2::CENTER_CENTER,
                    "No supported MTF Grid charts (M15+ only)",
                    egui::FontId::proportional(14.0),
                    AXIS_TEXT,
                );
                return;
            }
            let cols = mtf_canvas_grid_cols(mtf_indices.len());
            let rows = mtf_canvas_grid_rows(mtf_indices.len()).max(1);
            let cell_w = available.width() / cols as f32;
            let cell_h = (available.height().max(80.0) / rows as f32).max(80.0);

            // Detect click on grid cell to focus it
            let click_pos = if ctx.input(|i| i.pointer.primary_clicked()) {
                ctx.input(|i| i.pointer.interact_pos())
            } else {
                None
            };

            // Lazy-load bars for visible MTF grid charts through the paced
            // deferred loader. Doing a synchronous `try_load()` directly from
            // this render loop produced multi-second UI stalls while restored
            // MTF grids pulled M1/M5/M15 merged rows and recomputed overlays.
            // `queue_chart_reload` is O(1)-deduped by `deferred_chart_load_set`.
            let empty_chart_load_now = std::time::Instant::now();
            for &vi in &mtf_indices {
                if self.should_queue_empty_chart_reload(vi, empty_chart_load_now) {
                    self.queue_chart_reload(vi);
                }
            }

            for (grid_pos, &vi) in mtf_indices.iter().enumerate() {
                // Rebuild trade overlay every 120 frames (~30s) or on first load.
                // During heavy sync, keep the cached overlay: rebuilding every
                // restored MTF cell adds avoidable work to already overloaded frames.
                let fc = self.frame_count;
                if !self.heavy_sync_in_progress
                    && (self.charts[vi].cached_trade_overlay_frame == 0
                        || fc.wrapping_sub(self.charts[vi].cached_trade_overlay_frame) > 120)
                {
                    self.charts[vi].cached_trade_overlay =
                        self.build_trade_overlay(&self.charts[vi]);
                    self.charts[vi].cached_trade_overlay_frame = fc;
                }
                // Move the cached overlay out for the duration of this cell render — avoids
                // a Vec<TradeMarker> clone (with String tickers) per cell per frame. We
                // restore it once draw_chart returns, before the next cell iterates.
                let trade_ov = std::mem::take(&mut self.charts[vi].cached_trade_overlay);
                let chart = &mut self.charts[vi];
                // Live trade / forming update from public trades: force gen bump for this MTF cell
                // so renderers (volume profile, depth, tooltip, camera) pick up O(1) changes promptly.
                // MTF Grid is trading-session critical.
                if chart.forming_bar_dirty || chart.live_trade_vol > 0.0 {
                    chart.mark_view_changed();
                }
                let idx = grid_pos;
                let col = idx % cols;
                let row = idx / cols;
                let cell_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        available.left() + col as f32 * cell_w,
                        available.top() + row as f32 * cell_h,
                    ),
                    egui::vec2(cell_w - 2.0, cell_h - 2.0),
                );

                // Click to focus this cell (vi = actual chart index, not grid position)
                if let Some(pos) = click_pos {
                    if cell_rect.contains(pos) {
                        self.mtf_focused = Some(vi);
                        self.active_tab = vi;
                    }
                }

                // Pointer in cell detection (for zoom/pan, NOT for focus change)
                let ptr_in_cell = !pointer_over_floating
                    && ctx.input(|i| {
                        i.pointer
                            .hover_pos()
                            .map(|p| cell_rect.contains(p))
                            .unwrap_or(false)
                    });
                let is_focused = self.mtf_focused == Some(vi);

                // Price-axis vertical scaling for this cell — same pattern as the
                // single-chart path so MTF grid cells also respond to dragging the
                // right scale strip.
                let cell_price_axis_w = 70.0_f32;
                let cell_price_axis_rect = egui::Rect::from_min_max(
                    egui::pos2(cell_rect.right() - cell_price_axis_w, cell_rect.top()),
                    cell_rect.max,
                );
                let cell_scale_resp = ui
                    .interact(
                        cell_price_axis_rect,
                        ui.id().with(("mtf_cell_price_axis", vi)),
                        egui::Sense::click_and_drag(),
                    )
                    .on_hover_cursor(egui::CursorIcon::ResizeVertical);
                let scaling_this_cell = cell_scale_resp.is_pointer_button_down_on();
                if scaling_this_cell {
                    let dy = ctx.input(|i| i.pointer.delta().y);
                    if dy.abs() > 0.0 {
                        let zoom_delta = -dy as f64 * 0.003;
                        let factor = (1.0 + zoom_delta).clamp(0.1, 20.0);
                        chart.scale_chart_price_axis(factor);  // pure vertical scale, no time shift
                    }
                }
                if cell_scale_resp.double_clicked() {
                    // Price axis double → vertical auto-fit only
                    chart.price_zoom = 1.0;
                    chart.price_pan = 0.0;
                    chart.manual_view_override = false;
                    chart.reset_camera_from_legacy();
                }

                let cell_chart_body_rect = egui::Rect::from_min_max(
                    cell_rect.min,
                    egui::pos2(cell_rect.right() - cell_price_axis_w, cell_rect.bottom()),
                );
                let cell_body_resp = ui
                    .interact(
                        cell_chart_body_rect,
                        ui.id().with(("mtf_cell_chart_body", vi)),
                        egui::Sense::click_and_drag(),
                    )
                    .on_hover_cursor(egui::CursorIcon::Grab);
                // Yield to a live SL/TP line drag (unified pre-pass owns it) —
                // without this the cell camera pans underneath the line drag.
                let cell_body_started = cell_body_resp.is_pointer_button_down_on()
                    && !scaling_this_cell
                    && self.draw_mode == DrawMode::None
                    && !sl_tp_line_drag_live;
                let cell_body_press = (cell_body_started
                    || (chart.is_dragging && ctx.input(|i| i.pointer.primary_down())))
                    && !scaling_this_cell
                    && self.draw_mode == DrawMode::None
                    && !sl_tp_line_drag_live;
                if sl_tp_line_drag_live && chart.is_dragging {
                    chart.is_dragging = false;
                    chart.drag_start = None;
                }

                if cell_body_resp.double_clicked() {
                    // Body double-click on an MTF cell → focus exactly that
                    // chart in single-chart mode. The global double-click path
                    // cannot know which MTF tile was clicked early enough, so
                    // doing it here avoids falling back to the old active tab.
                    self.mtf_focused = Some(vi);
                    self.active_tab = vi;
                    self.mtf_enabled = false;
                    self.log.push_back(LogEntry::info(format!(
                        "Focused: {} [{}] — double-click to return to MTF grid",
                        chart.symbol,
                        chart.timeframe.label()
                    )));
                }

                if cell_body_started && !chart.is_dragging {
                    chart.is_dragging = true;
                    chart.is_drawing_drag = false;
                    chart.is_scaling_price = false;
                    chart.drag_start = ctx.input(|i| {
                        i.pointer
                            .press_origin()
                            .or_else(|| i.pointer.interact_pos())
                            .or_else(|| i.pointer.hover_pos())
                    });
                    let price_pane_h = chart_price_pane_height(
                        cell_chart_body_rect.height(),
                        active_sub_pane_count,
                    );
                    chart.begin_chart_camera_pan(cell_chart_body_rect.width(), price_pane_h);
                }
                if cell_body_press && chart.is_dragging {
                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grabbing);
                    if let (Some(start), Some(pos)) =
                        (chart.drag_start, ctx.input(|i| i.pointer.interact_pos()))
                    {
                        let total_drag = pos - start;
                        if total_drag.x.abs() > 0.0 || total_drag.y.abs() > 0.0 {
                            let price_pane_h = chart_price_pane_height(
                                cell_chart_body_rect.height(),
                                active_sub_pane_count,
                            );
                            chart.pan_chart_camera_pixels(
                                total_drag,
                                cell_chart_body_rect.width(),
                                price_pane_h,
                            );
                        }
                    }
                }
                if !cell_body_press && chart.is_dragging {
                    chart.is_dragging = false;
                    chart.drag_start = None;
                }

                // Zoom when pointer is in this cell (no focus-click required) — but
                // skip while the user is actively dragging the price scale so the
                // scroll-zoom and body pan don't fight the vertical scaling.
                if ptr_in_cell && !scaling_this_cell {
                    let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
                    if scroll != 0.0 {
                        // Mouse-centered horizontal zoom for MTF cell
                        let local_x = (hover_pos.x - cell_chart_body_rect.left()).max(0.0);
                        let target_bar = chart.bar_from_x(local_x as f32, cell_chart_body_rect.width());
                        let factor = 1.0 + (scroll as f64 * 0.002).clamp(-0.08, 0.08);
                        chart.zoom_chart_bars_around(factor, target_bar.max(0.0));
                    }
                }

                if ChartState::should_ensure_mql_mtf_overlays_for_render(
                    self.heavy_sync_in_progress,
                    self.mtf_enabled,
                    is_focused,
                ) {
                    if let Some(cache) = render_cache.as_ref() {
                        chart.ensure_mql_mtf_overlays_for_render(
                            std::sync::Arc::as_ref(cache),
                            flags.sma200,
                            flags.kama,
                        );
                    }
                }
                let painter = ui.painter_at(cell_rect);
                // Only the active cell that owns the lines gets them (ADR-132).
                let (cell_sl, cell_tp) = if trade_lines_on_active && vi == trade_lines_tab {
                    (sl_price, tp_price)
                } else {
                    (None, None)
                };
                let cell_geometry = draw_chart(
                    &painter,
                    chart,
                    cell_rect,
                    crosshair,
                    &flags,
                    show_rsi,
                    show_fisher,
                    show_macd,
                    show_volume_pane,
                    show_stochastic,
                    show_adx,
                    show_cci,
                    show_williams_r,
                    show_obv,
                    show_momentum,
                    show_cmo,
                    show_qstick,
                    show_disparity,
                    show_bop,
                    show_stddev,
                    show_mfi,
                    show_trix,
                    show_ppo,
                    show_ultosc,
                    show_stochrsi,
                    show_var_oscillator,
                    show_better_volume,
                    show_ehlers_ebsw,
                    show_ehlers_cyber,
                    show_ehlers_cg,
                    show_ehlers_roof,
                    self.show_squeeze,
                    cell_sl,
                    cell_tp,
                    &trade_ov,
                    &self.alerts,
                    &chart.regulatory_alerts,
                    &self.draw_mode,
                    chart_overlay_company_name(
                        &self.bg.all_fundamentals,
                        &chart_company_names,
                        &chart.symbol,
                    )
                    .as_deref(),
                );
                // Restore the cached overlay we moved out above; stash the
                // painted price geometry for next frame's line hit-testing.
                self.charts[vi].cached_trade_overlay = trade_ov;
                self.charts[vi].last_price_geometry = cell_geometry;

                // Border: green for focused, dim for others (WebKit: .mtf-grid-cell:hover outline)
                let border_color = if is_focused {
                    egui::Color32::from_rgb(76, 175, 80) // green — focused
                } else {
                    egui::Color32::from_rgb(40, 40, 60) // dim
                };
                let border_width = if is_focused { 2.0 } else { 1.0 };
                ui.painter_at(cell_rect).rect_stroke(
                    cell_rect,
                    0.0,
                    egui::Stroke::new(border_width, border_color),
                    egui::StrokeKind::Outside,
                );
            }
        } else {
            // Allocate the visual chart area as hover-only, then create separate
            // interaction targets for the chart body and the price axis. A full-rect
            // click/drag response steals the pointer before the narrow price scale can
            // own it, which regressed TradingView-style scale dragging.
            let (rect, _chart_alloc_resp) =
                ui.allocate_exact_size(available.size(), egui::Sense::hover());
            let price_axis_w = 70.0_f32;
            let price_axis_rect = egui::Rect::from_min_max(
                egui::pos2(rect.right() - price_axis_w, rect.top()),
                rect.max,
            );
            let chart_body_interact_rect = egui::Rect::from_min_max(
                rect.min,
                egui::pos2(rect.right() - price_axis_w, rect.bottom()),
            );
            // Single click_and_drag widget for the price axis. Previous attempts
            // layered a Sense::drag widget and a Sense::click widget on the same
            // rect — but later-registered widgets win egui's hit-test, so the click
            // widget swallowed the press and the drag widget never saw the gesture.
            // The original reason for splitting was that `dragged()` defers until
            // egui decides the gesture is "decidedly dragging" (eats slow scale
            // flicks). We sidestep that by reading drag movement from the raw
            // pointer delta whenever `is_pointer_button_down_on()` is true, which
            // fires from the press frame onward without any movement threshold.
            // Egui's z-order still routes presses on overlapping floating windows
            // to the window, so the old `pointer_over_window` guard is no longer
            // needed for this widget.
            let price_axis_resp = ui
                .interact(
                    price_axis_rect,
                    ui.id().with(("single_chart_price_axis", self.active_tab)),
                    egui::Sense::click_and_drag(),
                )
                .on_hover_cursor(egui::CursorIcon::ResizeVertical);
            // Right-axis drag = pure vertical price scale (zoom span, keep center) per TV/MT5.
            // Body drag (separate rect) = horizontal time pan + vertical price position pan (free-look).
            let resp = ui.interact(
                chart_body_interact_rect,
                ui.id().with(("single_chart_body_drag", self.active_tab)),
                egui::Sense::click_and_drag(),
            );
            if resp.double_clicked()
                && self.draw_mode == DrawMode::None
                && !self.mtf_enabled
                && self.charts.len() > 1
            {
                // Use the actual single-chart body response, not only the
                // frame-global pointer test above. egui can mark the chart
                // body as owning the pointer on the double-click frame, which
                // makes the early `on_chart_body` guard miss the click and
                // leaves the focused MTF chart stuck in single mode.
                self.mtf_enabled = true;
                self.mtf_focused = None;
                self.queue_empty_charts_for_load();
                self.log.push_back(LogEntry::info("MTF grid restored"));
            }
            if let Some(chart) = self.charts.get_mut(self.active_tab) {
                let scale_press = price_axis_resp.is_pointer_button_down_on();
                if scale_press && !chart.is_scaling_price {
                    chart.is_scaling_price = true;
                    chart.is_dragging = false;
                    chart.is_drawing_drag = false;
                    chart.scale_start_zoom = chart.price_zoom;
                    chart.scale_start_y = price_axis_resp
                        .interact_pointer_pos()
                        .map(|pos| pos.y)
                        .unwrap_or(chart.scale_start_y);
                }
                if scale_press {
                    let dy = ctx.input(|i| i.pointer.delta().y);
                    if dy.abs() > 0.0 {
                        let zoom_delta = -dy as f64 * 0.003;
                        let factor = (1.0 + zoom_delta).clamp(0.1, 20.0);
                        chart.scale_chart_price_axis(factor);  // pure vertical scale, no time shift
                        chart.is_dragging = false;
                    }
                } else if chart.is_scaling_price {
                    chart.is_scaling_price = false;
                }
                if price_axis_resp.double_clicked() {
                    chart.price_zoom = 1.0;
                    chart.price_pan = 0.0;
                    chart.manual_view_override = false;
                    chart.reset_camera_from_legacy();
                }

                let sl_tp_line_drag_active = self.dragging_sl || self.dragging_tp;
                let body_started = resp.is_pointer_button_down_on()
                    && self.draw_mode == DrawMode::None
                    && !scale_press
                    && !sl_tp_line_drag_active
                    && !chart.is_drawing_drag;
                let body_press = (body_started
                    || (chart.is_dragging && ctx.input(|i| i.pointer.primary_down())))
                    && self.draw_mode == DrawMode::None
                    && !scale_press
                    && !sl_tp_line_drag_active
                    && !chart.is_drawing_drag;
                if sl_tp_line_drag_active {
                    chart.is_dragging = false;
                    chart.drag_start = None;
                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeVertical);
                } else if resp.hovered()
                    && self.draw_mode != DrawMode::None
                    && self.draw_mode != DrawMode::Eraser
                {
                    // Placement armed → crosshair, like TradingView.
                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Crosshair);
                } else if resp.hovered() && self.draw_mode == DrawMode::None && !scale_press {
                    // Hovering a drawing signals "grabbable" before any click.
                    let over_drawing = chart
                        .last_price_geometry
                        .zip(ctx.input(|i| i.pointer.hover_pos()))
                        .map(|(g, pos)| {
                            g.chart_rect.contains(pos)
                                && chart
                                    .drawings
                                    .iter()
                                    .any(|d| drawing_hit_distance(d, pos, &g) <= 8.0)
                        })
                        .unwrap_or(false);
                    ui.output_mut(|o| {
                        o.cursor_icon = if over_drawing {
                            egui::CursorIcon::Move
                        } else if chart.is_drawing_drag {
                            egui::CursorIcon::Move
                        } else {
                            egui::CursorIcon::Grab
                        }
                    });
                }
                if body_started && !chart.is_dragging {
                    chart.is_dragging = true;
                    chart.is_drawing_drag = false;
                    chart.is_scaling_price = false;
                    chart.drag_start = ctx.input(|i| {
                        i.pointer
                            .press_origin()
                            .or_else(|| i.pointer.interact_pos())
                            .or_else(|| i.pointer.hover_pos())
                    });
                    let price_pane_h = chart_price_pane_height(
                        chart_body_interact_rect.height(),
                        active_sub_pane_count,
                    );
                    chart.begin_chart_camera_pan(chart_body_interact_rect.width(), price_pane_h);
                }
                if body_press && chart.is_dragging && !chart.is_scaling_price {
                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grabbing);
                    if let (Some(start), Some(pos)) =
                        (chart.drag_start, ctx.input(|i| i.pointer.interact_pos()))
                    {
                        let total_drag = pos - start;
                        if total_drag.x.abs() > 0.0 || total_drag.y.abs() > 0.0 {
                            let price_pane_h = chart_price_pane_height(
                                chart_body_interact_rect.height(),
                                active_sub_pane_count,
                            );
                            chart.pan_chart_camera_pixels(
                                total_drag,
                                chart_body_interact_rect.width(),
                                price_pane_h,
                            );
                        }
                    }
                }
                if !body_press && chart.is_dragging {
                    chart.is_dragging = false;
                    chart.drag_start = None;
                }
            }

            // Rebuild trade overlay every 120 frames (~30s) or on first load
            let fc = self.frame_count;
            if let Some(c) = self.charts.get(self.active_tab) {
                if !self.heavy_sync_in_progress && c.cached_trade_overlay_frame == 0
                    || fc.wrapping_sub(c.cached_trade_overlay_frame) > 120
                {
                    let ov = self.build_trade_overlay(c);
                    self.charts[self.active_tab].cached_trade_overlay = ov;
                    self.charts[self.active_tab].cached_trade_overlay_frame = fc;
                }
            }
            // Trade overlay is moved into the chart-mutating block below and
            // restored after draw_chart — same trick as the MTF grid above. Avoids
            // cloning Vec<TradeMarker> (with String tickers) every frame.

            // Replay mode: clamp view to only show replay_bar_idx bars
            if self.replay_active {
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    let count = self.replay_bar_idx.max(1).min(chart.bars.len());
                    chart.view_offset = count.saturating_sub(1);
                    chart.visible_bars = chart.visible_bars.min(count);
                }
            }

            if let Some(chart) = self.charts.get_mut(self.active_tab) {
                if let Some(cache) = render_cache.as_ref() {
                    chart.ensure_mql_mtf_overlays_for_render(
                        std::sync::Arc::as_ref(cache),
                        flags.sma200,
                        flags.kama,
                    );
                }
                let trade_ov = std::mem::take(&mut chart.cached_trade_overlay);
                // Mirror in-progress multi-click placement points so the
                // render-side live preview can complete them with the cursor.
                chart.preview_pending_points = if matches!(
                    self.draw_mode,
                    DrawMode::PlacingPolyline | DrawMode::PlacingPath
                ) {
                    self.polyline_points.clone()
                } else if self.draw_mode == DrawMode::PlacingBrush {
                    self.brush_points.clone()
                } else if self.draw_mode != DrawMode::None {
                    self.multi_click_points.clone()
                } else {
                    Vec::new()
                };
                let painter = ui.painter_at(rect);
                // Single mode renders the active chart; the lines still only
                // appear when their owner symbol matches it (ADR-132).
                let (active_sl, active_tp) = if trade_lines_on_active {
                    (sl_price, tp_price)
                } else {
                    (None, None)
                };
                let single_geometry = draw_chart(
                    &painter,
                    chart,
                    rect,
                    crosshair,
                    &flags,
                    show_rsi,
                    show_fisher,
                    show_macd,
                    show_volume_pane,
                    show_stochastic,
                    show_adx,
                    show_cci,
                    show_williams_r,
                    show_obv,
                    show_momentum,
                    show_cmo,
                    show_qstick,
                    show_disparity,
                    show_bop,
                    show_stddev,
                    show_mfi,
                    show_trix,
                    show_ppo,
                    show_ultosc,
                    show_stochrsi,
                    show_var_oscillator,
                    show_better_volume,
                    show_ehlers_ebsw,
                    show_ehlers_cyber,
                    show_ehlers_cg,
                    show_ehlers_roof,
                    self.show_squeeze,
                    active_sl,
                    active_tp,
                    &trade_ov,
                    &self.alerts,
                    &chart.regulatory_alerts,
                    &self.draw_mode,
                    chart_overlay_company_name(
                        &self.bg.all_fundamentals,
                        &chart_company_names,
                        &chart.symbol,
                    )
                    .as_deref(),
                );
                chart.cached_trade_overlay = trade_ov;
                chart.last_price_geometry = single_geometry;

                // Replay overlay: show bar count and speed
                if self.replay_active {
                    let replay_text = format!(
                        "REPLAY {}/{} | {} | {:.1} bars/s",
                        self.replay_bar_idx,
                        chart.bars.len(),
                        if self.replay_playing {
                            "▶ PLAY"
                        } else {
                            "⏸ PAUSED"
                        },
                        self.replay_speed,
                    );
                    painter.text(
                        egui::pos2(rect.left() + 8.0, rect.top() + 8.0),
                        egui::Align2::LEFT_TOP,
                        &replay_text,
                        egui::FontId::monospace(12.0),
                        egui::Color32::from_rgb(255, 200, 50),
                    );
                }

                // ── drawing selection via click (DrawMode::None) or eraser delete ─────
                // Universal hit-test on the exact painted geometry: every
                // drawing variant is selectable/erasable, including ones with
                // off-screen endpoints (the old per-variant match required
                // both endpoints visible and silently missed ~25 tool types).
                if resp.clicked()
                    && (self.draw_mode == DrawMode::None || self.draw_mode == DrawMode::Eraser)
                {
                    if let (Some(click_pos), Some(g)) = (
                        ctx.input(|i| i.pointer.interact_pos()),
                        chart.last_price_geometry,
                    ) {
                        if g.chart_rect.contains(click_pos) {
                            const HIT_THRESHOLD: f32 = 8.0;
                            let mut best_idx: Option<usize> = None;
                            let mut best_dist = HIT_THRESHOLD;
                            for (i, d) in chart.drawings.iter().enumerate() {
                                let dist = drawing_hit_distance(d, click_pos, &g);
                                if dist < best_dist {
                                    best_dist = dist;
                                    best_idx = Some(i);
                                }
                            }
                            if self.draw_mode == DrawMode::Eraser {
                                // Eraser mode: delete the nearest drawing on click
                                if let Some(idx) = best_idx {
                                    let d = chart.drawings.remove(idx);
                                    if idx < chart.drawing_styles.len() {
                                        chart.drawing_styles.remove(idx);
                                    }
                                    chart.drawings_undo.push(d);
                                    chart.selected_drawing = None;
                                }
                            } else if best_idx.is_some() && best_idx != chart.selected_drawing {
                                chart.selected_drawing = best_idx;
                            } else if best_idx.is_none() {
                                // Click on empty space → deselect
                                chart.selected_drawing = None;
                            }
                        }
                    }
                }
                // ESC → deselect drawing
                if ctx.input(|i| i.key_pressed(egui::Key::Escape))
                    && chart.selected_drawing.is_some()
                {
                    chart.selected_drawing = None;
                }

                // ── drawing mode click handling ──────────────────────
                if resp.clicked()
                    && self.draw_mode != DrawMode::None
                    && self.draw_mode != DrawMode::Eraser
                {
                    if let (Some(pos), Some(g)) = (crosshair, chart.last_price_geometry) {
                        // Bar/price from the exact painted geometry — the old
                        // recomputation ignored log scale and the free-look
                        // camera, so drawings landed offset from the cursor
                        // whenever the view wasn't the legacy autoscale.
                        if !chart.bars.is_empty() && g.chart_rect.contains(pos) {
                            let max_bar = chart.bars.len().saturating_sub(1);
                            let abs_idx = g.x_to_bar(pos.x, max_bar);
                            let raw_price = g.price_from_y(pos.y);

                            // OHLC Snap (magnet): snap to the nearest candle
                            // OHLC level within 8 screen pixels (pixel-based so
                            // it feels identical at any zoom or on log scale).
                            let price = if self.snap_enabled && abs_idx < chart.bars.len() {
                                const SNAP_PX: f32 = 8.0;
                                let bar = &chart.bars[abs_idx];
                                let ohlc = [bar.open, bar.high, bar.low, bar.close];
                                let mut best = raw_price;
                                let mut best_dist = SNAP_PX;
                                for &level in &ohlc {
                                    let dist = (pos.y - g.price_to_y(level)).abs();
                                    if dist < best_dist {
                                        best = level;
                                        best_dist = dist;
                                    }
                                }
                                best
                            } else {
                                raw_price
                            };

                            let dc = self.draw_color; // pre-placement color
                            match self.draw_mode {
                                DrawMode::Eraser | DrawMode::None => {} // handled above
                                DrawMode::PlacingHLine => {
                                    chart.drawings.push(Drawing::HLine { price, color: dc });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingTrendP1 => {
                                    self.draw_mode = DrawMode::PlacingTrendP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingTrendP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::TrendLine {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: TRENDLINE_COL,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingFiboP1 => {
                                    self.draw_mode = DrawMode::PlacingFiboP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingFiboP2 { bar1, price1 } => {
                                    let (high, low) = if price1 > price {
                                        (price1, price)
                                    } else {
                                        (price, price1)
                                    };
                                    chart.drawings.push(Drawing::FiboRetrace {
                                        high,
                                        low,
                                        bar_start: bar1,
                                        bar_end: abs_idx,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingVLine => {
                                    chart.drawings.push(Drawing::VLine {
                                        bar_idx: abs_idx,
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingRectP1 => {
                                    self.draw_mode = DrawMode::PlacingRectP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingRectP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::Rectangle {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: egui::Color32::from_rgba_premultiplied(
                                            100, 150, 255, 40,
                                        ),
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingRayP1 => {
                                    self.draw_mode = DrawMode::PlacingRayP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingRayP2 { bar1, price1 } => {
                                    let slope = if abs_idx != bar1 {
                                        (price - price1) / (abs_idx as f64 - bar1 as f64)
                                    } else {
                                        0.0
                                    };
                                    chart.drawings.push(Drawing::Ray {
                                        origin: (bar1, price1),
                                        slope,
                                        color: egui::Color32::from_rgb(100, 200, 255),
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingChannelP1 => {
                                    self.draw_mode = DrawMode::PlacingChannelP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingChannelP2 { bar1, price1 } => {
                                    self.draw_mode = DrawMode::PlacingChannelP3 {
                                        bar1,
                                        price1,
                                        bar2: abs_idx,
                                        price2: price,
                                    };
                                }
                                DrawMode::PlacingChannelP3 {
                                    bar1,
                                    price1,
                                    bar2,
                                    price2,
                                } => {
                                    let width = price - price1; // offset from first line
                                    chart.drawings.push(Drawing::Channel {
                                        p1: (bar1, price1),
                                        p2: (bar2, price2),
                                        width,
                                        color: egui::Color32::from_rgb(150, 200, 100),
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                // ── New drawing tool handlers ──
                                DrawMode::PlacingExtLineP1 => {
                                    self.draw_mode = DrawMode::PlacingExtLineP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingExtLineP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::ExtendedLine {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingHRay => {
                                    chart.drawings.push(Drawing::HRay {
                                        bar_idx: abs_idx,
                                        price,
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingCrossLine => {
                                    chart.drawings.push(Drawing::CrossLine {
                                        bar_idx: abs_idx,
                                        price,
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingArrowP1 => {
                                    self.draw_mode = DrawMode::PlacingArrowP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingArrowP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::ArrowLine {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingInfoLineP1 => {
                                    self.draw_mode = DrawMode::PlacingInfoLineP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingInfoLineP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::InfoLine {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingPitchforkP1 => {
                                    self.draw_mode = DrawMode::PlacingPitchforkP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingPitchforkP2 { bar1, price1 } => {
                                    self.draw_mode = DrawMode::PlacingPitchforkP3 {
                                        bar1,
                                        price1,
                                        bar2: abs_idx,
                                        price2: price,
                                    };
                                }
                                DrawMode::PlacingPitchforkP3 {
                                    bar1,
                                    price1,
                                    bar2,
                                    price2,
                                } => {
                                    chart.drawings.push(Drawing::Pitchfork {
                                        pivot: (bar1, price1),
                                        p2: (bar2, price2),
                                        p3: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingFiboExtP1 => {
                                    self.draw_mode = DrawMode::PlacingFiboExtP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingFiboExtP2 { bar1, price1 } => {
                                    self.draw_mode = DrawMode::PlacingFiboExtP3 {
                                        bar1,
                                        price1,
                                        bar2: abs_idx,
                                        price2: price,
                                    };
                                }
                                DrawMode::PlacingFiboExtP3 {
                                    bar1,
                                    price1,
                                    bar2,
                                    price2,
                                } => {
                                    chart.drawings.push(Drawing::FiboExtension {
                                        p1: (bar1, price1),
                                        p2: (bar2, price2),
                                        p3: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingGannFan => {
                                    // Scale = visible price range / visible bars (1×1 angle baseline)
                                    let (si, ei) = chart.visible_range();
                                    let vis = &chart.bars[si..ei];
                                    let hi = vis.iter().map(|b| b.high).fold(f64::MIN, f64::max);
                                    let lo = vis.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                                    let scale = if vis.len() > 1 {
                                        (hi - lo) / vis.len() as f64
                                    } else {
                                        1.0
                                    };
                                    chart.drawings.push(Drawing::GannFan {
                                        origin: (abs_idx, price),
                                        scale,
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingLongPosP1 => {
                                    self.draw_mode = DrawMode::PlacingLongPosP2 {
                                        bar1: abs_idx,
                                        entry: price,
                                    };
                                }
                                DrawMode::PlacingLongPosP2 { bar1, entry } => {
                                    self.draw_mode = DrawMode::PlacingLongPosP3 {
                                        bar1,
                                        entry,
                                        stop: price,
                                    };
                                }
                                DrawMode::PlacingLongPosP3 { bar1, entry, stop } => {
                                    chart.drawings.push(Drawing::LongPosition {
                                        entry: (bar1, entry),
                                        stop,
                                        target: price,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingShortPosP1 => {
                                    self.draw_mode = DrawMode::PlacingShortPosP2 {
                                        bar1: abs_idx,
                                        entry: price,
                                    };
                                }
                                DrawMode::PlacingShortPosP2 { bar1, entry } => {
                                    self.draw_mode = DrawMode::PlacingShortPosP3 {
                                        bar1,
                                        entry,
                                        stop: price,
                                    };
                                }
                                DrawMode::PlacingShortPosP3 { bar1, entry, stop } => {
                                    chart.drawings.push(Drawing::ShortPosition {
                                        entry: (bar1, entry),
                                        stop,
                                        target: price,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingPriceRangeP1 => {
                                    self.draw_mode = DrawMode::PlacingPriceRangeP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingPriceRangeP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::PriceRange {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingTextLabel => {
                                    chart.drawings.push(Drawing::TextLabel {
                                        bar_idx: abs_idx,
                                        price,
                                        text: "Label".to_string(),
                                        color: egui::Color32::WHITE,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingArrowMarkerUp => {
                                    chart.drawings.push(Drawing::ArrowMarker {
                                        bar_idx: abs_idx,
                                        price,
                                        is_up: true,
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingArrowMarkerDown => {
                                    chart.drawings.push(Drawing::ArrowMarker {
                                        bar_idx: abs_idx,
                                        price,
                                        is_up: false,
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingEllipseP1 => {
                                    self.draw_mode = DrawMode::PlacingEllipseP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingEllipseP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::Ellipse {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingTriangleP1 => {
                                    self.draw_mode = DrawMode::PlacingTriangleP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingTriangleP2 { bar1, price1 } => {
                                    self.draw_mode = DrawMode::PlacingTriangleP3 {
                                        bar1,
                                        price1,
                                        bar2: abs_idx,
                                        price2: price,
                                    };
                                }
                                DrawMode::PlacingTriangleP3 {
                                    bar1,
                                    price1,
                                    bar2,
                                    price2,
                                } => {
                                    chart.drawings.push(Drawing::Triangle {
                                        p1: (bar1, price1),
                                        p2: (bar2, price2),
                                        p3: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingTrendAngleP1 => {
                                    self.draw_mode = DrawMode::PlacingTrendAngleP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingTrendAngleP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::TrendAngle {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingParallelChP1 => {
                                    self.draw_mode = DrawMode::PlacingParallelChP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingParallelChP2 { bar1, price1 } => {
                                    // Offset = half the vertical distance between p1 and p2 (user clicks define center + one edge)
                                    let offset = (price - (price1 + (price - price1) * 0.5))
                                        .abs()
                                        .max(0.0001);
                                    let mid_price2 = (price1 + price) / 2.0;
                                    chart.drawings.push(Drawing::ParallelChannel {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, mid_price2),
                                        offset,
                                        color: egui::Color32::from_rgb(150, 200, 100),
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingFibChannelP1 => {
                                    self.draw_mode = DrawMode::PlacingFibChannelP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingFibChannelP2 { bar1, price1 } => {
                                    self.draw_mode = DrawMode::PlacingFibChannelP3 {
                                        bar1,
                                        price1,
                                        bar2: abs_idx,
                                        price2: price,
                                    };
                                }
                                DrawMode::PlacingFibChannelP3 {
                                    bar1,
                                    price1,
                                    bar2,
                                    price2,
                                } => {
                                    chart.drawings.push(Drawing::FibChannel {
                                        p1: (bar1, price1),
                                        p2: (bar2, price2),
                                        p3: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingFibTimeZones => {
                                    chart.drawings.push(Drawing::FibTimeZones {
                                        bar_idx: abs_idx,
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingPriceLabel => {
                                    chart.drawings.push(Drawing::PriceLabel {
                                        bar_idx: abs_idx,
                                        price,
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingCalloutP1 => {
                                    self.draw_mode = DrawMode::PlacingCalloutP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingCalloutP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::Callout {
                                        anchor: (bar1, price1),
                                        label_pos: (abs_idx, price),
                                        text: "Note".to_string(),
                                        color: egui::Color32::WHITE,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingHighlighterP1 => {
                                    self.draw_mode = DrawMode::PlacingHighlighterP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingHighlighterP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::Highlighter {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingCrossMarker => {
                                    chart.drawings.push(Drawing::CrossMarker {
                                        bar_idx: abs_idx,
                                        price,
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingPolyline => {
                                    self.polyline_points.push((abs_idx, price));
                                    // Don't change draw_mode — keep collecting points
                                }
                                DrawMode::PlacingAnchorNote => {
                                    chart.drawings.push(Drawing::AnchorNote {
                                        bar_idx: abs_idx,
                                        price,
                                        text: "Note".to_string(),
                                        color: egui::Color32::WHITE,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingRegressionChP1 => {
                                    self.draw_mode = DrawMode::PlacingRegressionChP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingRegressionChP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::RegressionChannel {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingGannBoxP1 => {
                                    self.draw_mode = DrawMode::PlacingGannBoxP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingGannBoxP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::GannBox {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingElliottWave => {
                                    self.multi_click_points.push((abs_idx, price));
                                    if self.multi_click_points.len() >= 5 {
                                        let pts = std::mem::take(&mut self.multi_click_points);
                                        chart.drawings.push(Drawing::ElliottWave {
                                            points: pts,
                                            color: dc,
                                        });
                                        self.draw_mode = DrawMode::None;
                                    }
                                }
                                DrawMode::PlacingAbcCorrection => {
                                    self.multi_click_points.push((abs_idx, price));
                                    if self.multi_click_points.len() >= 3 {
                                        let pts = std::mem::take(&mut self.multi_click_points);
                                        chart.drawings.push(Drawing::AbcCorrection {
                                            points: pts,
                                            color: dc,
                                        });
                                        self.draw_mode = DrawMode::None;
                                    }
                                }
                                DrawMode::PlacingDateRangeP1 => {
                                    self.draw_mode = DrawMode::PlacingDateRangeP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingDateRangeP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::DateRange {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingDatePriceRangeP1 => {
                                    self.draw_mode = DrawMode::PlacingDatePriceRangeP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingDatePriceRangeP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::DatePriceRange {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingHeadShoulders => {
                                    self.multi_click_points.push((abs_idx, price));
                                    if self.multi_click_points.len() >= 5 {
                                        let pts = std::mem::take(&mut self.multi_click_points);
                                        chart.drawings.push(Drawing::HeadShoulders {
                                            points: pts,
                                            color: dc,
                                        });
                                        self.draw_mode = DrawMode::None;
                                    }
                                }
                                DrawMode::PlacingXabcdPattern => {
                                    self.multi_click_points.push((abs_idx, price));
                                    if self.multi_click_points.len() >= 5 {
                                        let pts = std::mem::take(&mut self.multi_click_points);
                                        chart.drawings.push(Drawing::XabcdPattern {
                                            points: pts,
                                            color: dc,
                                        });
                                        self.draw_mode = DrawMode::None;
                                    }
                                }
                                DrawMode::PlacingBrush => {
                                    // Single click adds a point; drag handling below adds more
                                    self.brush_points.push((abs_idx, price));
                                }
                                DrawMode::PlacingSchiffPitchforkP1 => {
                                    self.draw_mode = DrawMode::PlacingSchiffPitchforkP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingSchiffPitchforkP2 { bar1, price1 } => {
                                    self.draw_mode = DrawMode::PlacingSchiffPitchforkP3 {
                                        bar1,
                                        price1,
                                        bar2: abs_idx,
                                        price2: price,
                                    };
                                }
                                DrawMode::PlacingSchiffPitchforkP3 {
                                    bar1,
                                    price1,
                                    bar2,
                                    price2,
                                } => {
                                    chart.drawings.push(Drawing::SchiffPitchfork {
                                        pivot: (bar1, price1),
                                        p2: (bar2, price2),
                                        p3: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingModSchiffPitchforkP1 => {
                                    self.draw_mode = DrawMode::PlacingModSchiffPitchforkP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingModSchiffPitchforkP2 { bar1, price1 } => {
                                    self.draw_mode = DrawMode::PlacingModSchiffPitchforkP3 {
                                        bar1,
                                        price1,
                                        bar2: abs_idx,
                                        price2: price,
                                    };
                                }
                                DrawMode::PlacingModSchiffPitchforkP3 {
                                    bar1,
                                    price1,
                                    bar2,
                                    price2,
                                } => {
                                    chart.drawings.push(Drawing::ModSchiffPitchfork {
                                        pivot: (bar1, price1),
                                        p2: (bar2, price2),
                                        p3: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingCyclicLinesP1 => {
                                    self.draw_mode =
                                        DrawMode::PlacingCyclicLinesP2 { bar1: abs_idx };
                                }
                                DrawMode::PlacingCyclicLinesP2 { bar1 } => {
                                    chart.drawings.push(Drawing::CyclicLines {
                                        bar_start: bar1,
                                        bar_end: abs_idx,
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingSineWaveP1 => {
                                    self.draw_mode = DrawMode::PlacingSineWaveP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingSineWaveP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::SineWave {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingEmoji => {
                                    chart.drawings.push(Drawing::Emoji {
                                        bar_idx: abs_idx,
                                        price,
                                        emoji: "\u{1F3AF}".to_string(),
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingFlag => {
                                    chart.drawings.push(Drawing::Flag {
                                        bar_idx: abs_idx,
                                        price,
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingBalloonP1 => {
                                    self.draw_mode = DrawMode::PlacingBalloonP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingBalloonP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::Balloon {
                                        anchor: (bar1, price1),
                                        label_pos: (abs_idx, price),
                                        text: "Note".to_string(),
                                        color: egui::Color32::WHITE,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingSessionBreak => {
                                    chart.drawings.push(Drawing::SessionBreak {
                                        bar_idx: abs_idx,
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingMagnetLevel => {
                                    chart
                                        .drawings
                                        .push(Drawing::MagnetLevel { price, color: dc });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingRiskRewardP1 => {
                                    self.draw_mode = DrawMode::PlacingRiskRewardP2 {
                                        bar1: abs_idx,
                                        entry: price,
                                    };
                                }
                                DrawMode::PlacingRiskRewardP2 { bar1, entry } => {
                                    self.draw_mode = DrawMode::PlacingRiskRewardP3 {
                                        bar1,
                                        entry,
                                        stop: price,
                                    };
                                }
                                DrawMode::PlacingRiskRewardP3 { bar1, entry, stop } => {
                                    chart.drawings.push(Drawing::RiskRewardBox {
                                        entry: (bar1, entry),
                                        stop,
                                        target: price,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingFibCircleP1 => {
                                    self.draw_mode = DrawMode::PlacingFibCircleP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingFibCircleP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::FibCircle {
                                        center: (bar1, price1),
                                        radius_pt: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingArcP1 => {
                                    self.draw_mode = DrawMode::PlacingArcP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingArcP2 { bar1, price1 } => {
                                    self.draw_mode = DrawMode::PlacingArcP3 {
                                        bar1,
                                        price1,
                                        bar2: abs_idx,
                                        price2: price,
                                    };
                                }
                                DrawMode::PlacingArcP3 {
                                    bar1,
                                    price1,
                                    bar2,
                                    price2,
                                } => {
                                    chart.drawings.push(Drawing::ArcDraw {
                                        p1: (bar1, price1),
                                        p2: (bar2, price2),
                                        p3: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingCurveP1 => {
                                    self.draw_mode = DrawMode::PlacingCurveP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingCurveP2 { bar1, price1 } => {
                                    self.draw_mode = DrawMode::PlacingCurveP3 {
                                        bar1,
                                        price1,
                                        bar2: abs_idx,
                                        price2: price,
                                    };
                                }
                                DrawMode::PlacingCurveP3 {
                                    bar1,
                                    price1,
                                    bar2,
                                    price2,
                                } => {
                                    self.draw_mode = DrawMode::PlacingCurveP4 {
                                        bar1,
                                        price1,
                                        bar2,
                                        price2,
                                        bar3: abs_idx,
                                        price3: price,
                                    };
                                }
                                DrawMode::PlacingCurveP4 {
                                    bar1,
                                    price1,
                                    bar2,
                                    price2,
                                    bar3,
                                    price3,
                                } => {
                                    chart.drawings.push(Drawing::CurveDraw {
                                        p1: (bar1, price1),
                                        ctrl1: (bar2, price2),
                                        ctrl2: (bar3, price3),
                                        p2: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingPath => {
                                    self.polyline_points.push((abs_idx, price));
                                    // Keep collecting — double-click finishes (handled in polyline dbl-click)
                                }
                                DrawMode::PlacingForecastP1 => {
                                    self.draw_mode = DrawMode::PlacingForecastP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingForecastP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::Forecast {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingGhostFeedP1 => {
                                    self.draw_mode = DrawMode::PlacingGhostFeedP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingGhostFeedP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::GhostFeed {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingSignpost => {
                                    chart.drawings.push(Drawing::Signpost {
                                        bar_idx: abs_idx,
                                        price,
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingRulerP1 => {
                                    self.draw_mode = DrawMode::PlacingRulerP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingRulerP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::Ruler {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingTimeCycleP1 => {
                                    self.draw_mode = DrawMode::PlacingTimeCycleP2 { bar1: abs_idx };
                                }
                                DrawMode::PlacingTimeCycleP2 { bar1 } => {
                                    chart.drawings.push(Drawing::TimeCycle {
                                        bar_start: bar1,
                                        bar_end: abs_idx,
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingSpeedFanP1 => {
                                    self.draw_mode = DrawMode::PlacingSpeedFanP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingSpeedFanP2 { bar1, price1 } => {
                                    self.draw_mode = DrawMode::PlacingSpeedFanP3 {
                                        bar1,
                                        price1,
                                        bar2: abs_idx,
                                        price2: price,
                                    };
                                }
                                DrawMode::PlacingSpeedFanP3 {
                                    bar1,
                                    price1,
                                    bar2,
                                    price2,
                                } => {
                                    chart.drawings.push(Drawing::SpeedResistanceFan {
                                        p1: (bar1, price1),
                                        p2: (bar2, price2),
                                        p3: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingSpeedArcP1 => {
                                    self.draw_mode = DrawMode::PlacingSpeedArcP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingSpeedArcP2 { bar1, price1 } => {
                                    self.draw_mode = DrawMode::PlacingSpeedArcP3 {
                                        bar1,
                                        price1,
                                        bar2: abs_idx,
                                        price2: price,
                                    };
                                }
                                DrawMode::PlacingSpeedArcP3 {
                                    bar1,
                                    price1,
                                    bar2,
                                    price2,
                                } => {
                                    chart.drawings.push(Drawing::SpeedResistanceArc {
                                        p1: (bar1, price1),
                                        p2: (bar2, price2),
                                        p3: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingFibSpiralP1 => {
                                    self.draw_mode = DrawMode::PlacingFibSpiralP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingFibSpiralP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::FibSpiral {
                                        center: (bar1, price1),
                                        radius_pt: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingRotatedRectP1 => {
                                    self.draw_mode = DrawMode::PlacingRotatedRectP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingRotatedRectP2 { bar1, price1 } => {
                                    self.draw_mode = DrawMode::PlacingRotatedRectP3 {
                                        bar1,
                                        price1,
                                        bar2: abs_idx,
                                        price2: price,
                                    };
                                }
                                DrawMode::PlacingRotatedRectP3 {
                                    bar1,
                                    price1,
                                    bar2,
                                    price2,
                                } => {
                                    chart.drawings.push(Drawing::RotatedRectangle {
                                        p1: (bar1, price1),
                                        p2: (bar2, price2),
                                        p3: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingAnchoredVwap => {
                                    chart.drawings.push(Drawing::AnchoredVwapLine {
                                        bar_idx: abs_idx,
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingTrendChannelP1 => {
                                    self.draw_mode = DrawMode::PlacingTrendChannelP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingTrendChannelP2 { bar1, price1 } => {
                                    self.draw_mode = DrawMode::PlacingTrendChannelP3 {
                                        bar1,
                                        price1,
                                        bar2: abs_idx,
                                        price2: price,
                                    };
                                }
                                DrawMode::PlacingTrendChannelP3 {
                                    bar1,
                                    price1,
                                    bar2,
                                    price2,
                                } => {
                                    chart.drawings.push(Drawing::TrendChannel {
                                        p1: (bar1, price1),
                                        p2: (bar2, price2),
                                        p3: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingInsidePitchforkP1 => {
                                    self.draw_mode = DrawMode::PlacingInsidePitchforkP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingInsidePitchforkP2 { bar1, price1 } => {
                                    self.draw_mode = DrawMode::PlacingInsidePitchforkP3 {
                                        bar1,
                                        price1,
                                        bar2: abs_idx,
                                        price2: price,
                                    };
                                }
                                DrawMode::PlacingInsidePitchforkP3 {
                                    bar1,
                                    price1,
                                    bar2,
                                    price2,
                                } => {
                                    chart.drawings.push(Drawing::InsidePitchfork {
                                        pivot: (bar1, price1),
                                        p2: (bar2, price2),
                                        p3: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingFibWedgeP1 => {
                                    self.draw_mode = DrawMode::PlacingFibWedgeP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingFibWedgeP2 { bar1, price1 } => {
                                    self.draw_mode = DrawMode::PlacingFibWedgeP3 {
                                        bar1,
                                        price1,
                                        bar2: abs_idx,
                                        price2: price,
                                    };
                                }
                                DrawMode::PlacingFibWedgeP3 {
                                    bar1,
                                    price1,
                                    bar2,
                                    price2,
                                } => {
                                    chart.drawings.push(Drawing::FibWedge {
                                        p1: (bar1, price1),
                                        p2: (bar2, price2),
                                        p3: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingPriceNote => {
                                    chart.drawings.push(Drawing::PriceNote {
                                        price,
                                        text: "Note".to_string(),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingMeasureToolP1 => {
                                    self.draw_mode = DrawMode::PlacingMeasureToolP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingMeasureToolP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::MeasureTool {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                // ── New 1-click tools ──
                                DrawMode::PlacingAnchoredText => {
                                    chart.drawings.push(Drawing::AnchoredText {
                                        bar_idx: abs_idx,
                                        price,
                                        text: "Text".to_string(),
                                        color: egui::Color32::WHITE,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingComment => {
                                    chart.drawings.push(Drawing::Comment {
                                        bar_idx: abs_idx,
                                        price,
                                        text: "Comment".to_string(),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingArrowMarkerLeft => {
                                    chart.drawings.push(Drawing::ArrowMarkerLeft {
                                        bar_idx: abs_idx,
                                        price,
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingArrowMarkerRight => {
                                    chart.drawings.push(Drawing::ArrowMarkerRight {
                                        bar_idx: abs_idx,
                                        price,
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                // ── New 2-click tools ──
                                DrawMode::PlacingCircleP1 => {
                                    self.draw_mode = DrawMode::PlacingCircleP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingCircleP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::Circle {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingPitchFanP1 => {
                                    self.draw_mode = DrawMode::PlacingPitchFanP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingPitchFanP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::PitchFan {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingTrendFibTimeP1 => {
                                    self.draw_mode = DrawMode::PlacingTrendFibTimeP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingTrendFibTimeP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::TrendFibTime {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingGannSquareP1 => {
                                    self.draw_mode = DrawMode::PlacingGannSquareP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingGannSquareP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::GannSquare {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingGannSquareFixedP1 => {
                                    self.draw_mode = DrawMode::PlacingGannSquareFixedP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingGannSquareFixedP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::GannSquareFixed {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingBarsPatternP1 => {
                                    self.draw_mode = DrawMode::PlacingBarsPatternP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingBarsPatternP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::BarsPattern {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingProjectionP1 => {
                                    self.draw_mode = DrawMode::PlacingProjectionP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingProjectionP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::Projection {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                DrawMode::PlacingDoubleCurveP1 => {
                                    self.draw_mode = DrawMode::PlacingDoubleCurveP2 {
                                        bar1: abs_idx,
                                        price1: price,
                                    };
                                }
                                DrawMode::PlacingDoubleCurveP2 { bar1, price1 } => {
                                    chart.drawings.push(Drawing::DoubleCurve {
                                        p1: (bar1, price1),
                                        p2: (abs_idx, price),
                                        color: dc,
                                    });
                                    self.draw_mode = DrawMode::None;
                                }
                                // ── New multi-click tools ──
                                DrawMode::PlacingTrianglePattern => {
                                    self.multi_click_points.push((abs_idx, price));
                                    if self.multi_click_points.len() >= 3 {
                                        let pts = self.multi_click_points.drain(..).collect();
                                        chart.drawings.push(Drawing::TrianglePattern {
                                            points: pts,
                                            color: dc,
                                        });
                                        self.draw_mode = DrawMode::None;
                                    }
                                }
                                DrawMode::PlacingThreeDrives => {
                                    self.multi_click_points.push((abs_idx, price));
                                    if self.multi_click_points.len() >= 3 {
                                        let pts = self.multi_click_points.drain(..).collect();
                                        chart.drawings.push(Drawing::ThreeDrives {
                                            points: pts,
                                            color: dc,
                                        });
                                        self.draw_mode = DrawMode::None;
                                    }
                                }
                                DrawMode::PlacingElliottDouble => {
                                    self.multi_click_points.push((abs_idx, price));
                                    if self.multi_click_points.len() >= 3 {
                                        let pts = self.multi_click_points.drain(..).collect();
                                        chart.drawings.push(Drawing::ElliottDouble {
                                            points: pts,
                                            color: dc,
                                        });
                                        self.draw_mode = DrawMode::None;
                                    }
                                }
                                DrawMode::PlacingAbcdPattern => {
                                    self.multi_click_points.push((abs_idx, price));
                                    if self.multi_click_points.len() >= 4 {
                                        let pts = self.multi_click_points.drain(..).collect();
                                        chart.drawings.push(Drawing::AbcdPattern {
                                            points: pts,
                                            color: dc,
                                        });
                                        self.draw_mode = DrawMode::None;
                                    }
                                }
                                DrawMode::PlacingCypherPattern => {
                                    self.multi_click_points.push((abs_idx, price));
                                    if self.multi_click_points.len() >= 5 {
                                        let pts = self.multi_click_points.drain(..).collect();
                                        chart.drawings.push(Drawing::CypherPattern {
                                            points: pts,
                                            color: dc,
                                        });
                                        self.draw_mode = DrawMode::None;
                                    }
                                }
                                DrawMode::PlacingElliottTriangle => {
                                    self.multi_click_points.push((abs_idx, price));
                                    if self.multi_click_points.len() >= 5 {
                                        let pts = self.multi_click_points.drain(..).collect();
                                        chart.drawings.push(Drawing::ElliottTriangle {
                                            points: pts,
                                            color: dc,
                                        });
                                        self.draw_mode = DrawMode::None;
                                    }
                                }
                                DrawMode::PlacingElliottTripleCombo => {
                                    self.multi_click_points.push((abs_idx, price));
                                    if self.multi_click_points.len() >= 5 {
                                        let pts = self.multi_click_points.drain(..).collect();
                                        chart.drawings.push(Drawing::ElliottTripleCombo {
                                            points: pts,
                                            color: dc,
                                        });
                                        self.draw_mode = DrawMode::None;
                                    }
                                }
                            }
                        }
                    }
                }

                // ── right-click context menu ─────────────────────────
                resp.context_menu(|ui| {
                        ui.label(egui::RichText::new("Drawing Tools").color(ACCENT).strong());
                        ui.separator();
                        if ui.button("Horizontal Line").clicked() {
                            self.draw_mode = DrawMode::PlacingHLine;
                            ui.close();
                        }
                        if ui.button("Trendline (2 clicks)").clicked() {
                            self.draw_mode = DrawMode::PlacingTrendP1;
                            ui.close();
                        }
                        if ui.button("Fibonacci Retracement").clicked() {
                            self.draw_mode = DrawMode::PlacingFiboP1;
                            ui.close();
                        }
                        if ui.button("Vertical Line").clicked() {
                            self.draw_mode = DrawMode::PlacingVLine;
                            ui.close();
                        }
                        if ui.button("Rectangle (2 clicks)").clicked() {
                            self.draw_mode = DrawMode::PlacingRectP1;
                            ui.close();
                        }
                        if ui.button("Ray (2 clicks)").clicked() {
                            self.draw_mode = DrawMode::PlacingRayP1;
                            ui.close();
                        }
                        if ui.button("Channel (3 clicks)").clicked() {
                            self.draw_mode = DrawMode::PlacingChannelP1;
                            ui.close();
                        }
                        ui.separator();
                        if !chart.drawings.is_empty() {
                            ui.menu_button("Drawing Color", |ui| {
                                let colors = [
                                    ("White", egui::Color32::WHITE),
                                    ("Yellow", egui::Color32::from_rgb(255, 200, 50)),
                                    ("Green", egui::Color32::from_rgb(0, 220, 80)),
                                    ("Red", egui::Color32::from_rgb(220, 40, 40)),
                                    ("Cyan", egui::Color32::from_rgb(0, 200, 255)),
                                    ("Magenta", egui::Color32::from_rgb(255, 100, 255)),
                                    ("Orange", egui::Color32::from_rgb(255, 140, 0)),
                                    ("Blue", egui::Color32::from_rgb(80, 120, 255)),
                                ];
                                for (name, color) in &colors {
                                    if ui.button(egui::RichText::new(*name).color(*color)).clicked() {
                                        // Apply color to selected drawing, or last if none selected
                                        let target_idx = chart.selected_drawing.unwrap_or(chart.drawings.len().saturating_sub(1));
                                        if let Some(d) = chart.drawings.get_mut(target_idx) {
                                            // Generic: try setting color on common variants
                                            macro_rules! set_color {
                                                ($d:expr, $c:expr, $($variant:ident),+) => {
                                                    match $d {
                                                        $(Drawing::$variant { color: col, .. } => *col = $c,)+
                                                        _ => {}
                                                    }
                                                };
                                            }
                                            set_color!(d, *color,
                                                HLine, TrendLine, Rectangle, Ray, Channel,
                                                ExtendedLine, HRay, CrossLine, ArrowLine,
                                                InfoLine, Pitchfork, FiboExtension, GannFan,
                                                TextLabel, ArrowMarker, Ellipse, Triangle,
                                                TrendAngle, ParallelChannel, FibChannel,
                                                FibTimeZones, Callout, Highlighter, Polyline,
                                                AnchorNote, RegressionChannel, GannBox,
                                                ElliottWave, AbcCorrection, HeadShoulders,
                                                XabcdPattern, Brush, SchiffPitchfork,
                                                ModSchiffPitchfork, CyclicLines, SineWave,
                                                Flag, Balloon, SessionBreak, MagnetLevel,
                                                FibCircle, ArcDraw, CurveDraw, PathDraw,
                                                Ruler, TimeCycle, SpeedResistanceFan,
                                                SpeedResistanceArc, FibSpiral, RotatedRectangle,
                                                AnchoredVwapLine, TrendChannel, InsidePitchfork,
                                                FibWedge, PriceNote, MeasureTool, PriceLabel,
                                                CrossMarker, Forecast, GhostFeed, Signpost,
                                                VLine, AnchoredText, Comment, ArrowMarkerLeft,
                                                ArrowMarkerRight, Circle, PitchFan, TrendFibTime,
                                                GannSquare, GannSquareFixed, BarsPattern, Projection,
                                                DoubleCurve, TrianglePattern, ThreeDrives,
                                                ElliottDouble, AbcdPattern, CypherPattern,
                                                ElliottTriangle, ElliottTripleCombo
                                            );
                                        }
                                        ui.close();
                                    }
                                }
                            });
                        }
                        // Per-drawing width/style editor (for selected drawing)
                        if let Some(sel) = chart.selected_drawing {
                            ui.menu_button("Drawing Width", |ui| {
                                for w in [1.0_f32, 1.5, 2.0, 3.0, 4.0] {
                                    if ui.button(format!("{}px", w)).clicked() {
                                        if let Some(style) = chart.drawing_styles.get_mut(sel) {
                                            style.0 = w;
                                        }
                                        ui.close();
                                    }
                                }
                            });
                            ui.menu_button("Drawing Style", |ui| {
                                if ui.button("━ Solid").clicked() {
                                    if let Some(style) = chart.drawing_styles.get_mut(sel) { style.1 = LineStyle::Solid; }
                                    ui.close();
                                }
                                if ui.button("╌ Dashed").clicked() {
                                    if let Some(style) = chart.drawing_styles.get_mut(sel) { style.1 = LineStyle::Dashed; }
                                    ui.close();
                                }
                                if ui.button("┈ Dotted").clicked() {
                                    if let Some(style) = chart.drawing_styles.get_mut(sel) { style.1 = LineStyle::Dotted; }
                                    ui.close();
                                }
                            });
                            if ui.button("Delete Selected").clicked() {
                                let d = chart.drawings.remove(sel);
                                if sel < chart.drawing_styles.len() { chart.drawing_styles.remove(sel); }
                                chart.drawings_undo.push(d);
                                chart.selected_drawing = None;
                                ui.close();
                            }
                            ui.separator();
                        }
                        if ui.button("Remove Last Drawing").clicked() {
                            chart.drawings.pop();
                            ui.close();
                        }
                        if ui.button("Clear All Drawings").clicked() {
                            chart.drawings.clear();
                            ui.close();
                        }
                        ui.separator();
                        ui.label(egui::RichText::new("Chart").color(ACCENT).strong());
                        ui.separator();
                        if ui.button("Reset Zoom / Pan").clicked() {
                            chart.price_zoom = 1.0;
                            chart.price_pan = 0.0;
                            chart.visible_bars = 200;
                            chart.view_offset = chart.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
                            chart.manual_view_override = false;
                            ui.close();
                        }
                        if ui.button(if chart.log_scale { "● Log Scale" } else { "  Log Scale" }).clicked() {
                            chart.log_scale = !chart.log_scale;
                            ui.close();
                        }
                        if ui.button("Fit All Bars").clicked() {
                            chart.visible_bars = chart.bars.len().max(50);
                            chart.view_offset = chart.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
                            chart.price_zoom = 1.0;
                            chart.price_pan = 0.0;
                            chart.manual_view_override = false;
                            ui.close();
                        }
                        ui.separator();
                        for &ct in &[ChartType::Candle, ChartType::HeikinAshi, ChartType::Line, ChartType::OhlcBars, ChartType::Renko] {
                            let label = if chart.chart_type == ct { format!("● {}", ct.label()) } else { format!("  {}", ct.label()) };
                            if ui.button(label).clicked() {
                                chart.chart_type = ct;
                                ui.close();
                            }
                        }
                        ui.separator();
                        ui.label(egui::RichText::new("Timeframe").color(ACCENT).strong());
                        ui.separator();
                        for &tf in Timeframe::all() {
                            let label = if chart.timeframe == tf { format!("● {}", tf.label()) } else { format!("  {}", tf.label()) };
                            if ui.button(label).clicked() {
                                chart.timeframe = tf;
                                if let Some(ref cache_arc) = self.cache {
                                    let mut gpu = self.gpu_indicators.take();
                                    chart.try_load(Arc::as_ref(cache_arc), &mut self.log, gpu.as_mut());
                                    self.gpu_indicators = gpu;
                                }
                                ui.close();
                            }
                        }
                        ui.separator();
                        ui.label(egui::RichText::new("Windows").color(ACCENT).strong());
                        ui.separator();
                        if ui.button("Indicators…").clicked() { self.show_indicators_panel = true; ui.close(); }
                        if ui.button("Data Window").clicked() { self.show_data_window = true; ui.close(); }
                        if ui.button("Volume Profile").clicked() { self.show_volume_profile = true; ui.close(); }
                        if ui.button("Depth Profile (L2)").clicked() { self.show_depth_profile = true; ui.close(); }
                        if ui.button("Price Alerts…").clicked() { self.show_alerts = true; ui.close(); }
                        // ADR-094: Open command palette with chart context
                        if ui.button("Command Palette…").clicked() {
                            self.palette_context = PaletteContext::Chart;
                            self.command_open = true;
                            self.command_input.clear();
                            ui.close();
                        }
                        // Copy price at crosshair
                        if let Some(pos) = crosshair {
                            ui.separator();
                            if ui.button("Copy Price at Cursor").clicked() {
                                let frac = (pos.y - rect.top()) / (rect.height() - 80.0);
                                let (si, ei) = chart.visible_range();
                                let vis = &chart.bars[si..ei];
                                if !vis.is_empty() {
                                    let hi = vis.iter().map(|b| b.high).fold(f64::MIN, f64::max);
                                    let lo = vis.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                                    let price = hi - frac as f64 * (hi - lo);
                                    ctx.copy_text(format_price(price));
                                }
                                ui.close();
                            }
                        }
                    });
            }
        }
    }
}
