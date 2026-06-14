use super::*;

#[allow(deprecated)]
impl TyphooNApp {
    pub(super) fn render_tab_bar(&mut self, ctx: &egui::Context) {
        // ── tab bar ───────────────────────────────────────────────────────────
        // Snapshot the per-tab data up front so the tab loop can live inside a
        // horizontal ScrollArea closure without borrowing `self` — every action
        // (switch / close / drag-start / reorder) is deferred to after the strip
        // is drawn. Overflowed tabs used to be clipped with no way to reach them;
        // the ScrollArea makes the strip scroll (mouse wheel + scrollbar).
        let tab_snapshots: Vec<(usize, String, bool, bool)> = self
            .charts
            .iter()
            .enumerate()
            .map(|(idx, c)| {
                (
                    idx,
                    format!("{} [{}]", c.symbol, c.timeframe.label()),
                    idx == self.active_tab,
                    self.dragging_tab == Some(idx),
                )
            })
            .collect();
        let n_charts = self.charts.len();
        let dragging_tab = self.dragging_tab;

        egui::Panel::top("tab_bar")
            .exact_size(26.0) // WebKit: height: 26px
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let mut switch_to: Option<usize> = None;
                    let mut close_tab: Option<usize> = None;
                    let mut drop_target: Option<(usize, usize)> = None; // (drag_src, insert_at)
                    let mut start_drag: Option<usize> = None;
                    let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
                    let pointer_released = ctx.input(|i| i.pointer.primary_released());

                    // Collect tab rects for drag detection
                    let mut tab_rects: Vec<egui::Rect> = Vec::new();

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
                                for (idx, label, active, is_dragging_this) in &tab_snapshots {
                                    let idx = *idx;
                                    let active = *active;
                                    let is_dragging_this = *is_dragging_this;

                                    // Tab colours
                                    let tab_bg = if is_dragging_this {
                                        egui::Color32::from_rgb(20, 50, 80)
                                    } else if active {
                                        BG_BUTTON
                                    } else {
                                        egui::Color32::from_rgb(10, 10, 10)
                                    };
                                    let tab_text = if active {
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
                                    tab_rects.push(tab_rect);

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

                                    // Right border separator
                                    ui.painter().line_segment(
                                        [
                                            egui::pos2(tab_rect.right(), tab_rect.top()),
                                            egui::pos2(tab_rect.right(), tab_rect.bottom()),
                                        ],
                                        egui::Stroke::new(
                                            1.0,
                                            egui::Color32::from_rgb(34, 34, 34),
                                        ),
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

                                    // Close button (×) — right side of tab
                                    if n_charts > 1 {
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

                                    // Middle-click to close tab
                                    if tab_resp.middle_clicked() && n_charts > 1 {
                                        close_tab = Some(idx);
                                    }

                                    // Start drag
                                    if tab_resp.dragged() && dragging_tab.is_none() {
                                        start_drag = Some(idx);
                                    }
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
                        let mut new_chart = ChartState::new(&self.symbol_input, tf);
                        if let Some(ref cache) = self.cache.clone() {
                            {
                                let mut gpu = self.gpu_indicators.take();
                                new_chart.try_load(Arc::as_ref(cache), &mut self.log, gpu.as_mut());
                                self.gpu_indicators = gpu;
                            }
                        }
                        self.charts.push(new_chart);
                        self.active_tab = self.charts.len() - 1;
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
                                for (idx, rect) in tab_rects.iter().enumerate() {
                                    if rect.contains(pos) && idx != drag_src {
                                        let mid = rect.center().x;
                                        // Insert before idx if dropping on left half, after if right half
                                        let insert_at = if pos.x < mid { idx } else { idx + 1 };
                                        drop_target = Some((drag_src, insert_at));
                                        break;
                                    }
                                }
                            }
                            self.dragging_tab = None;
                        }
                    }

                    // Apply deferred actions
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
                        // Lazy-load chart bars on first tab switch
                        if let Some(chart) = self.charts.get_mut(idx) {
                            if chart.bars.is_empty() {
                                if let Some(ref cache) = self.cache {
                                    {
                                        let mut gpu = self.gpu_indicators.take();
                                        if !chart.try_load(cache, &mut self.log, gpu.as_mut()) {
                                            self.queue_chart_reload(0);
                                        }
                                        self.gpu_indicators = gpu;
                                    }
                                }
                            }
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
