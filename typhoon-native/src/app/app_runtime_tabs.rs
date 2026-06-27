use super::*;

pub(super) fn tab_bar_chart_indices(charts: &[ChartState]) -> Vec<usize> {
    charts
        .iter()
        .enumerate()
        .filter_map(|(idx, chart)| chart.show_in_tab_bar.then_some(idx))
        .collect()
}

#[allow(deprecated)]
impl TyphooNApp {
    pub(super) fn render_tab_bar(&mut self, ctx: &egui::Context) {
        // ── tab bar ───────────────────────────────────────────────────────────
        // Snapshot the per-tab data up front so the tab loop can live inside a
        // horizontal ScrollArea closure without borrowing `self` — every action
        // (switch / close / drag-start / reorder) is deferred to after the strip
        // is drawn. Overflowed tabs used to be clipped with no way to reach them;
        // the ScrollArea makes the strip scroll (mouse wheel + scrollbar).
        let tab_indices = tab_bar_chart_indices(&self.charts);
        // In MTF mode the tab strip doubles as the grid selector: each tab is
        // highlighted by whether it's included in the grid (`mtf_visible`). Left-click
        // still switches to the chart (and × closes it); right-click toggles grid
        // inclusion. `in_grid` rides in the snapshot so the draw loop needs no `self`
        // borrow.
        let mtf_on = self.mtf_enabled;
        let tab_snapshots: Vec<(usize, String, bool, bool, bool)> = tab_indices
            .iter()
            .filter_map(|idx| self.charts.get(*idx).map(|c| (*idx, c)))
            .map(|(idx, c)| {
                (
                    idx,
                    format!("{} [{}]", c.symbol, c.timeframe.label()),
                    idx == self.active_tab,
                    self.dragging_tab == Some(idx),
                    self.mtf_visible.get(idx).copied().unwrap_or(true),
                )
            })
            .collect();
        let n_tabs = tab_snapshots.len();
        let dragging_tab = self.dragging_tab;

        // Detect an active-tab change since last frame so the active tab can be
        // scrolled into view (covers tab clicks, the + button, NEW_TAB, and close
        // adjustments). Deferred actions mutate active_tab *after* the strip is
        // drawn, so the change is picked up on the following frame — by which
        // point a newly-added tab already exists in the scroll content.
        let scroll_to_active = self.active_tab != self.tab_bar_last_active;
        self.tab_bar_last_active = self.active_tab;

        egui::Panel::top("tab_bar")
            .exact_size(26.0) // WebKit: height: 26px
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let mut switch_to: Option<usize> = None;
                    let mut close_tab: Option<usize> = None;
                    let mut toggle_grid: Option<usize> = None; // MTF mode: toggle grid inclusion
                    let mut drop_target: Option<(usize, usize)> = None; // (drag_src, insert_at)
                    let mut start_drag: Option<usize> = None;
                    let mut active_rect: Option<egui::Rect> = None;
                    let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
                    let pointer_released = ctx.input(|i| i.pointer.primary_released());

                    // Collect tab rects for drag detection
                    let mut tab_rects: Vec<(usize, egui::Rect)> = Vec::new();

                    // Reserve room on the right for the always-visible + button and
                    // chart-type indicator; the scrollable tab strip takes the rest.
                    let reserved = 110.0_f32;
                    let tabs_w = (ui.available_width() - reserved).max(120.0);

                    // Floating scrollbar: overlays the strip instead of consuming
                    // vertical space, so it fits the 26px bar and still thickens on
                    // hover to stay clickable.
                    ui.spacing_mut().scroll.floating = true;

                    egui::ScrollArea::horizontal()
                        .id_salt("tab_strip_scroll")
                        .max_width(tabs_w)
                        .auto_shrink([true, false])
                        // Dragging a tab reorders it — don't let the scroll area
                        // hijack that as a drag-to-scroll gesture. Wheel + scrollbar
                        // still scroll the strip.
                        .drag_to_scroll(false)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.0;
                                for (idx, label, active, is_dragging_this, in_grid) in
                                    &tab_snapshots
                                {
                                    let idx = *idx;
                                    let active = *active;
                                    let is_dragging_this = *is_dragging_this;
                                    let in_grid = *in_grid;
                                    // MTF mode dims tabs excluded from the grid so the
                                    // included ones (outlined below) read as selected.
                                    let excluded = mtf_on && !in_grid;

                                    // Tab colours
                                    let tab_bg = if is_dragging_this {
                                        egui::Color32::from_rgb(20, 50, 80)
                                    } else if excluded {
                                        egui::Color32::from_rgb(8, 8, 8)
                                    } else if active {
                                        BG_BUTTON
                                    } else {
                                        egui::Color32::from_rgb(10, 10, 10)
                                    };
                                    let tab_text = if excluded {
                                        egui::Color32::from_rgb(70, 70, 70)
                                    } else if active {
                                        egui::Color32::WHITE
                                    } else {
                                        egui::Color32::from_rgb(136, 136, 136)
                                    };

                                    let tab_w = label.len() as f32 * 6.5 + 28.0;

                                    // Allocate space for this tab
                                    let (tab_rect, tab_resp) = ui.allocate_exact_size(
                                        egui::vec2(tab_w, 24.0),
                                        egui::Sense::click_and_drag(),
                                    );
                                    tab_rects.push((idx, tab_rect));
                                    if active && scroll_to_active {
                                        active_rect = Some(tab_rect);
                                    }

                                    // Draw tab background
                                    ui.painter().rect_filled(tab_rect, 0.0, tab_bg);

                                    // Active tab: green bottom border
                                    if active {
                                        ui.painter().line_segment(
                                            [
                                                egui::pos2(tab_rect.left(), tab_rect.bottom()),
                                                egui::pos2(tab_rect.right(), tab_rect.bottom()),
                                            ],
                                            egui::Stroke::new(
                                                2.0,
                                                egui::Color32::from_rgb(76, 175, 80),
                                            ),
                                        );
                                    }

                                    // MTF mode: accent outline around tabs included in
                                    // the grid — the "highlight to select" selector.
                                    if mtf_on && in_grid {
                                        ui.painter().rect_stroke(
                                            tab_rect.shrink(1.0),
                                            0.0,
                                            egui::Stroke::new(1.5, ACCENT),
                                            egui::StrokeKind::Inside,
                                        );
                                    }

                                    // Right border separator
                                    ui.painter().line_segment(
                                        [
                                            egui::pos2(tab_rect.right(), tab_rect.top()),
                                            egui::pos2(tab_rect.right(), tab_rect.bottom()),
                                        ],
                                        egui::Stroke::new(1.0, egui::Color32::from_rgb(34, 34, 34)),
                                    );

                                    // Draw drag indicator (green left/right border when
                                    // hovering during drag)
                                    if let Some(drag_src) = dragging_tab {
                                        if drag_src != idx {
                                            if let Some(pos) = pointer_pos {
                                                if tab_rect.contains(pos) {
                                                    let mid = tab_rect.center().x;
                                                    let side = if pos.x < mid {
                                                        tab_rect.left()
                                                    } else {
                                                        tab_rect.right()
                                                    };
                                                    ui.painter().line_segment(
                                                        [
                                                            egui::pos2(side, tab_rect.top()),
                                                            egui::pos2(side, tab_rect.bottom()),
                                                        ],
                                                        egui::Stroke::new(
                                                            2.0,
                                                            egui::Color32::from_rgb(76, 175, 80),
                                                        ),
                                                    );
                                                }
                                            }
                                        }
                                    }

                                    // Tab label text
                                    let text_pos =
                                        egui::pos2(tab_rect.left() + 6.0, tab_rect.center().y);
                                    ui.painter().text(
                                        text_pos,
                                        egui::Align2::LEFT_CENTER,
                                        label,
                                        egui::FontId::monospace(10.0),
                                        tab_text,
                                    );

                                    // Left-click switches to the chart and the × closes
                                    // it, exactly as in single-chart mode. In MTF mode a
                                    // right-click toggles whether the tab is included in
                                    // the grid (the accent outline above reflects it) —
                                    // navigate with left, curate the grid with right.
                                    if mtf_on && tab_resp.secondary_clicked() {
                                        toggle_grid = Some(idx);
                                    }
                                    if n_tabs > 1 {
                                        let close_rect = egui::Rect::from_min_size(
                                            egui::pos2(
                                                tab_rect.right() - 14.0,
                                                tab_rect.top() + 4.0,
                                            ),
                                            egui::vec2(12.0, 16.0),
                                        );
                                        let close_hovered = pointer_pos
                                            .map(|p| close_rect.contains(p))
                                            .unwrap_or(false);
                                        let close_col = if close_hovered {
                                            egui::Color32::from_rgb(255, 80, 80)
                                        } else {
                                            egui::Color32::from_rgb(85, 85, 85)
                                        };
                                        ui.painter().text(
                                            close_rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            "×",
                                            egui::FontId::monospace(11.0),
                                            close_col,
                                        );
                                        if tab_resp.clicked() && close_hovered {
                                            close_tab = Some(idx);
                                        } else if tab_resp.clicked() {
                                            switch_to = Some(idx);
                                        }
                                    } else if tab_resp.clicked() {
                                        switch_to = Some(idx);
                                    }

                                    // Middle-click to close tab.
                                    if tab_resp.middle_clicked() && n_tabs > 1 {
                                        close_tab = Some(idx);
                                    }

                                    // Start drag
                                    if tab_resp.dragged() && dragging_tab.is_none() {
                                        start_drag = Some(idx);
                                    }
                                }
                                // Bring the active tab into view after an active-tab
                                // change (None = scroll the minimum needed).
                                if let Some(rect) = active_rect {
                                    ui.scroll_to_rect(rect, None);
                                }
                            });
                        });

                    // + button (WebKit: .tab-add) — kept outside the scroll strip so
                    // it stays reachable no matter how many tabs are open.
                    if ui
                        .add(
                            egui::Label::new(
                                egui::RichText::new("+")
                                    .color(egui::Color32::from_rgb(85, 85, 85))
                                    .size(14.0),
                            )
                            .sense(egui::Sense::click()),
                        )
                        .clicked()
                    {
                        let tf = self
                            .charts
                            .get(self.active_tab)
                            .map(|c| c.timeframe)
                            .unwrap_or(Timeframe::H4);
                        let new_chart = ChartState::new(&self.symbol_input, tf);
                        self.charts.push(new_chart);
                        self.active_tab = self.charts.len() - 1;
                        // Defer the expensive load (cache read + GPU indicators + MTF
                        // overlays) to the paced loader so opening a tab never blocks the
                        // render thread on a heavy symbol (multi-second stalls — ADR-098).
                        self.queue_chart_reload(self.active_tab);
                        let sym = self.symbol_input.clone();
                        self.queue_open_symbol_sync_all_timeframes(&sym);
                    }

                    // Chart type indicator (right-aligned)
                    if let Some(c) = self.charts.get(self.active_tab) {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(c.chart_type.label())
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                        });
                    }

                    // Handle drop on release
                    if pointer_released {
                        if let Some(drag_src) = dragging_tab {
                            if let Some(pos) = pointer_pos {
                                for (idx, rect) in &tab_rects {
                                    if rect.contains(pos) && *idx != drag_src {
                                        let mid = rect.center().x;
                                        // Insert before idx if dropping on left half, after if right half
                                        let insert_at = if pos.x < mid { *idx } else { *idx + 1 };
                                        drop_target = Some((drag_src, insert_at));
                                        break;
                                    }
                                }
                            }
                            self.dragging_tab = None;
                        }
                    }

                    // Apply deferred actions
                    if let Some(idx) = toggle_grid {
                        while self.mtf_visible.len() < self.charts.len() {
                            self.mtf_visible.push(true);
                        }
                        if let Some(v) = self.mtf_visible.get_mut(idx) {
                            *v = !*v;
                        }
                        // Keep at least one tab in the grid so it never renders empty.
                        if self.mtf_visible.iter().all(|v| !v) {
                            if let Some(v) = self.mtf_visible.get_mut(idx) {
                                *v = true;
                            }
                        }
                    }
                    if let Some(idx) = start_drag {
                        self.dragging_tab = Some(idx);
                    }
                    if let Some(idx) = switch_to {
                        self.active_tab = idx;
                        // Sync symbol_input to the clicked tab's symbol.
                        // Without this, clicking a timeframe button after switching tabs
                        // reloads the OLD symbol (from the text box) instead of the tab's symbol.
                        if let Some(chart) = self.charts.get(idx) {
                            self.symbol_input = chart.symbol.clone();
                        }
                        // Defer chart bar loading on tab switch to the paced loader so
                        // switching to a heavy symbol never blocks the render thread
                        // (multi-second stalls — ADR-098). Also fixes the prior fallback
                        // that re-queued chart 0 instead of the switched-to tab.
                        let needs_load = self
                            .charts
                            .get(idx)
                            .is_some_and(|chart| chart.bars.is_empty());
                        if needs_load {
                            self.queue_chart_reload(idx);
                        }
                    }
                    if let Some(idx) = close_tab {
                        self.charts.remove(idx);
                        if self.active_tab >= self.charts.len() {
                            self.active_tab = self.charts.len().saturating_sub(1);
                        }
                    }
                    if let Some((drag_src, insert_at)) = drop_target {
                        if drag_src < self.charts.len() {
                            let chart = self.charts.remove(drag_src);
                            // Adjust insert_at since removal shifts indices
                            let adjusted = if insert_at > drag_src {
                                insert_at - 1
                            } else {
                                insert_at
                            };
                            let adjusted = adjusted.min(self.charts.len());
                            self.charts.insert(adjusted, chart);
                            self.active_tab = adjusted;
                        }
                    }
                });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_bar_indices_exclude_mtf_backing_charts() {
        let mut user_chart = ChartState::new("CC", Timeframe::D1);
        user_chart.show_in_tab_bar = true;
        let mut backing_chart = ChartState::new("CC", Timeframe::M1);
        backing_chart.show_in_tab_bar = false;

        assert_eq!(tab_bar_chart_indices(&[user_chart, backing_chart]), vec![0]);
    }
}
