use super::*;

impl TyphooNApp {
    pub(super) fn handle_runtime_input(&mut self, ctx: &egui::Context) -> bool {
        // ── Quake console toggle ─────────────────────────────────────────
        // Scans ALL input events for any sign of backtick/tilde/grave key.
        // Logs the first 20 unrecognized events for debugging Wayland issues.
        let open_palette = ctx.input_mut(|i| {
            let mut found = false;

            // Check all key methods
            if i.key_pressed(egui::Key::Backtick) {
                found = true;
            }

            // Scan every event
            i.events.retain(|e| {
                match e {
                    egui::Event::Text(t) if t == "`" || t == "~" => {
                        found = true;
                        false // consume
                    }
                    egui::Event::Key {
                        key: egui::Key::Backtick,
                        pressed: true,
                        ..
                    } => {
                        found = true;
                        false // consume
                    }
                    // Catch ANY key press and check the physical key
                    egui::Event::Key {
                        key,
                        pressed: true,
                        physical_key,
                        ..
                    } => {
                        // Check if physical_key matches backtick/grave
                        if let Some(pk) = physical_key {
                            if *pk == egui::Key::Backtick {
                                found = true;
                                return false; // consume
                            }
                        }
                        // Also check if the logical key name contains "grave" or "backtick"
                        let key_name = format!("{:?}", key);
                        if key_name.contains("Backtick") || key_name.contains("Grave") {
                            found = true;
                            return false;
                        }
                        true
                    }
                    _ => true,
                }
            });
            found
        });
        if open_palette {
            self.command_open = !self.command_open;
            if self.command_open {
                self.command_input.clear();
            } else {
                // Strip any trailing ` or ~ from input that might have leaked
                self.command_input = self
                    .command_input
                    .trim_matches(|c| c == '`' || c == '~')
                    .to_string();
            }
        }

        // ── Esc → close palette ──────────────────────────────────────────────
        if self.command_open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.command_open = false;
            self.palette_context = PaletteContext::Global; // reset context on close
        }

        // ── crosshair from pointer ───────────────────────────────────────────
        // Suppress crosshair when pointer is over a floating window (dragging, resizing, scrolling)
        let pointer_over_ui = ctx.egui_wants_pointer_input()
            || ctx.egui_is_using_pointer()
            || ctx.dragged_id().is_some();
        let pointer_over_floating = if !pointer_over_ui {
            let hp = ctx.input(|i| i.pointer.hover_pos().unwrap_or_default());
            ctx.layer_id_at(hp)
                .map(|id| id.order == egui::Order::Middle || id.order == egui::Order::Foreground)
                .unwrap_or(false)
        } else {
            true
        };
        self.crosshair = if pointer_over_floating {
            None
        } else {
            ctx.input(|i| i.pointer.hover_pos())
        };

        // ── keyboard shortcuts ───────────────────────────────────────────────
        if !self.command_open {
            let left = ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft));
            let right = ctx.input(|i| i.key_pressed(egui::Key::ArrowRight));
            let home = ctx.input(|i| i.key_pressed(egui::Key::Home));
            let end = ctx.input(|i| i.key_pressed(egui::Key::End));
            let pgup = ctx.input(|i| i.key_pressed(egui::Key::PageUp));
            let pgdn = ctx.input(|i| i.key_pressed(egui::Key::PageDown));
            let plus =
                ctx.input(|i| i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals));
            let minus = ctx.input(|i| i.key_pressed(egui::Key::Minus));
            let delete = ctx
                .input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace));

            // Ctrl+N = new tab, Ctrl+W = close tab
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::N)) {
                let tf = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| c.timeframe)
                    .unwrap_or(Timeframe::H4);
                let new_chart = ChartState::new(&self.symbol_input, tf);
                self.charts.push(new_chart);
                self.active_tab = self.charts.len() - 1;
                // Defer the expensive load to the paced loader so opening a tab never
                // blocks the render thread on a heavy symbol (ADR-098).
                self.queue_chart_reload(self.active_tab);
                let sym = self.symbol_input.clone();
                self.queue_open_symbol_sync_all_timeframes(&sym);
            }
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::W)) {
                if self.charts.len() > 1 {
                    self.charts.remove(self.active_tab);
                    if self.active_tab >= self.charts.len() {
                        self.active_tab = self.charts.len().saturating_sub(1);
                    }
                }
            }

            // ADR-094: Analytics keyboard shortcuts (Alt+key)
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::V)) {
                self.command_input = "VAR".to_string();
                self.log.push_back(LogEntry::info("Shortcut: Alt+V → VAR"));
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::C)) {
                self.command_input = "CORRELATION".to_string();
                self.log
                    .push_back(LogEntry::info("Shortcut: Alt+C → CORRELATION"));
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::S)) {
                self.command_input = "SCREENER".to_string();
                self.log
                    .push_back(LogEntry::info("Shortcut: Alt+S → SCREENER"));
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::R)) {
                self.command_input = "RISK_CALC".to_string();
                self.log
                    .push_back(LogEntry::info("Shortcut: Alt+R → RISK_CALC"));
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::B)) {
                self.command_input = "BACKTEST".to_string();
                self.log
                    .push_back(LogEntry::info("Shortcut: Alt+B → BACKTEST"));
            }
            if ctx.input(|i| i.key_pressed(egui::Key::F5)) {
                self.log
                    .push_back(LogEntry::info("F5: Refreshing all analytics..."));
                self.indicators_dirty = true;
            }
            // Esc: dismiss result card or close topmost window
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && !self.command_open {
                if self.result_card.is_some() {
                    self.result_card = None;
                }
            }
            // Ctrl+1..9 = jump to tab by number
            for digit in 1..=9_u32 {
                let key = match digit {
                    1 => egui::Key::Num1,
                    2 => egui::Key::Num2,
                    3 => egui::Key::Num3,
                    4 => egui::Key::Num4,
                    5 => egui::Key::Num5,
                    6 => egui::Key::Num6,
                    7 => egui::Key::Num7,
                    8 => egui::Key::Num8,
                    9 => egui::Key::Num9,
                    _ => continue,
                };
                if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(key)) {
                    let idx = (digit - 1) as usize;
                    if idx < self.charts.len() {
                        self.active_tab = idx;
                    }
                }
            }

            // Ctrl+Tab / Ctrl+Shift+Tab = cycle tabs
            if !self.charts.is_empty()
                && ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Tab))
            {
                if ctx.input(|i| i.modifiers.shift) {
                    self.active_tab = if self.active_tab == 0 {
                        self.charts.len() - 1
                    } else {
                        self.active_tab - 1
                    };
                } else {
                    self.active_tab = (self.active_tab + 1) % self.charts.len();
                }
            }

            // Delete/Backspace = remove selected drawing, or last drawing if none selected
            if delete {
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    if let Some(sel) = chart.selected_drawing {
                        if sel < chart.drawings.len() {
                            let d = chart.drawings.remove(sel);
                            chart.drawing_styles.remove(sel);
                            chart.drawings_undo.push(d);
                            chart.selected_drawing = None;
                        }
                    } else if let Some(d) = chart.drawings.pop() {
                        chart.drawing_styles.pop();
                        chart.drawings_undo.push(d);
                    }
                }
            }
            // Ctrl+Z = undo last drawing (same as delete but explicit)
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && !i.modifiers.shift)
            {
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    if let Some(d) = chart.drawings.pop() {
                        chart.drawing_styles.pop();
                        chart.drawings_undo.push(d);
                        chart.selected_drawing = None;
                        self.log.push_back(LogEntry::info("Undo: drawing removed"));
                    }
                }
            }
            // Ctrl+Shift+Z = redo (restore from undo stack)
            if ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z)) {
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    if let Some(d) = chart.drawings_undo.pop() {
                        chart.drawings.push(d);
                        chart.drawing_styles.push((1.5, LineStyle::Solid));
                        self.log.push_back(LogEntry::info("Redo: drawing restored"));
                    }
                }
            }

            // Escape = cancel drawing mode or exit replay
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                if self.replay_active {
                    self.replay_active = false;
                    self.replay_playing = false;
                    self.replay_bar_idx = 0;
                } else {
                    self.draw_mode = DrawMode::None;
                }
            }

            // Alt+1-9 = quick timeframe switch (TradingView standard)
            {
                let alt_tf = ctx.input(|i| {
                    if !i.modifiers.alt {
                        return None;
                    }
                    if i.key_pressed(egui::Key::Num1) {
                        Some(Timeframe::M1)
                    } else if i.key_pressed(egui::Key::Num2) {
                        Some(Timeframe::M5)
                    } else if i.key_pressed(egui::Key::Num3) {
                        Some(Timeframe::M15)
                    } else if i.key_pressed(egui::Key::Num4) {
                        Some(Timeframe::M30)
                    } else if i.key_pressed(egui::Key::Num5) {
                        Some(Timeframe::H1)
                    } else if i.key_pressed(egui::Key::Num6) {
                        Some(Timeframe::H4)
                    } else if i.key_pressed(egui::Key::Num7) {
                        Some(Timeframe::D1)
                    } else if i.key_pressed(egui::Key::Num8) {
                        Some(Timeframe::W1)
                    } else if i.key_pressed(egui::Key::Num9) {
                        Some(Timeframe::MN1)
                    } else {
                        None
                    }
                });
                if let Some(tf) = alt_tf {
                    let active = self.active_tab;
                    if let Some(chart) = self.charts.get_mut(active) {
                        chart.timeframe = tf;
                    }
                    // Defer the reload (cache read + GPU indicators + MTF overlays) so a
                    // timeframe hotkey never blocks the render thread (ADR-098).
                    self.queue_chart_reload(active);
                }
            }

            // Alt+letter = drawing tool shortcuts (TradingView standard)
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::H)) {
                self.draw_mode = DrawMode::PlacingHLine;
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::V)) {
                self.draw_mode = DrawMode::PlacingVLine;
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::T)) {
                self.draw_mode = DrawMode::PlacingTrendP1;
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::F)) {
                self.draw_mode = DrawMode::PlacingFiboP1;
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::R)) {
                self.draw_mode = DrawMode::PlacingRectP1;
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::E)) {
                self.draw_mode = DrawMode::Eraser;
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::L)) {
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    chart.log_scale = !chart.log_scale;
                }
            }
            if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::C)) {
                // Alt+C = cycle chart type
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    chart.chart_type = match chart.chart_type {
                        ChartType::Candle => ChartType::HeikinAshi,
                        ChartType::HeikinAshi => ChartType::Line,
                        ChartType::Line => ChartType::OhlcBars,
                        ChartType::OhlcBars => ChartType::Renko,
                        ChartType::Renko => ChartType::Candle,
                    };
                }
            }

            // Replay mode controls
            if self.replay_active {
                let total_bars = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| c.bars.len())
                    .unwrap_or(0);
                if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
                    self.replay_playing = !self.replay_playing;
                }
                if right && !self.replay_playing {
                    self.replay_bar_idx = (self.replay_bar_idx + 1).min(total_bars);
                }
                if left && !self.replay_playing {
                    self.replay_bar_idx = self.replay_bar_idx.saturating_sub(1).max(1);
                }
                // Up/Down = adjust speed
                let up = ctx.input(|i| i.key_pressed(egui::Key::ArrowUp));
                let down = ctx.input(|i| i.key_pressed(egui::Key::ArrowDown));
                if up {
                    self.replay_speed = (self.replay_speed * 1.5).min(60.0);
                }
                if down {
                    self.replay_speed = (self.replay_speed / 1.5).max(0.5);
                }

                // Auto-play timer
                if self.replay_playing {
                    let dt = ctx.input(|i| i.stable_dt);
                    self.replay_timer += dt;
                    let interval = 1.0 / self.replay_speed;
                    while self.replay_timer >= interval {
                        self.replay_timer -= interval;
                        self.replay_bar_idx = (self.replay_bar_idx + 1).min(total_bars);
                        if self.replay_bar_idx >= total_bars {
                            self.replay_playing = false;
                        }
                    }
                    ctx.request_repaint(); // keep animating
                }
                // Sync replay_bar_cap + view_offset on active chart
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    chart.replay_bar_cap = Some(self.replay_bar_idx);
                    // Lock view to replay position so chart scrolls with replay
                    let half_vis = chart.visible_bars / 2;
                    chart.view_offset = self.replay_bar_idx.saturating_sub(1) + half_vis.min(10);
                }
            } else {
                // Replay not active — ensure cap is cleared
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    if chart.replay_bar_cap.is_some() {
                        chart.replay_bar_cap = None;
                    }
                }
            }

            if let Some(chart) = self.charts.get_mut(self.active_tab) {
                // In replay mode, arrow keys are used for bar stepping, not panning
                if !self.replay_active {
                    if left {
                        chart.view_offset = chart.view_offset.saturating_sub(1);
                        chart.manual_view_override = true;
                    }
                    if right {
                        chart.view_offset = (chart.view_offset + 1)
                            .min(chart.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN);
                        chart.manual_view_override = chart.view_offset
                            < chart.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
                    }
                    if home {
                        chart.view_offset =
                            chart.visible_bars.min(chart.bars.len()).saturating_sub(1);
                        chart.manual_view_override = true;
                    }
                    if end {
                        chart.view_offset = chart.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
                        chart.manual_view_override = false;
                    }
                    if pgup {
                        chart.view_offset =
                            chart.view_offset.saturating_sub(chart.visible_bars / 2);
                        chart.manual_view_override = true;
                    }
                    if pgdn {
                        chart.view_offset = (chart.view_offset + chart.visible_bars / 2)
                            .min(chart.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN);
                        chart.manual_view_override = chart.view_offset
                            < chart.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
                    }
                }
                if plus {
                    Self::handle_zoom(chart, 1.0);
                }
                if minus {
                    Self::handle_zoom(chart, -1.0);
                }
            }
        }
        pointer_over_floating
    }
}
