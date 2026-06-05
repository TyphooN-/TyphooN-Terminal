use super::*;

#[allow(deprecated)]
impl TyphooNApp {
    pub(super) fn render_bottom_panels(&mut self, ctx: &egui::Context) {
        // ── bottom panel (log / volume) ──────────────────────────────────────
        // ── ADR-094: Result card rendering (above log) ─────────────
        // Auto-dismiss after 30 seconds
        if let Some((_, created)) = &self.result_card {
            if created.elapsed() > std::time::Duration::from_secs(30) {
                self.result_card = None;
            }
        }

        egui::Panel::bottom("bottom_panel")
            .resizable(true)
            .min_size(80.0)
            .default_size(140.0)
            .show(ctx, |ui| {
                // ── Result card (above log) ──
                if let Some((card, _)) = &self.result_card {
                    ui.group(|ui| {
                        match card {
                            ResultCard::Summary { title, metrics } => {
                                ui.horizontal(|ui| {
                                    ui.strong(title);
                                    if ui.small_button("\u{2716}").clicked() { /* dismiss below */ }
                                });
                                ui.horizontal_wrapped(|ui| {
                                    for (label, value, color) in metrics {
                                        ui.label(
                                            egui::RichText::new(label)
                                                .small()
                                                .color(egui::Color32::GRAY),
                                        );
                                        ui.label(
                                            egui::RichText::new(value)
                                                .strong()
                                                .color(*color)
                                                .monospace(),
                                        );
                                        ui.add_space(12.0);
                                    }
                                });
                            }
                            ResultCard::Table {
                                title,
                                headers,
                                rows,
                                sort_col,
                                sort_asc,
                            } => {
                                ui.horizontal(|ui| {
                                    ui.strong(title);
                                    ui.label(
                                        egui::RichText::new(format!("({} rows)", rows.len()))
                                            .small()
                                            .color(egui::Color32::GRAY),
                                    );
                                });
                                egui::ScrollArea::vertical()
                                    .auto_shrink(false)
                                    .max_height(100.0)
                                    .show(ui, |ui| {
                                        egui::Grid::new("result_table").striped(true).show(
                                            ui,
                                            |ui| {
                                                for (i, h) in headers.iter().enumerate() {
                                                    let arrow = if i == *sort_col {
                                                        if *sort_asc {
                                                            " \u{25B2}"
                                                        } else {
                                                            " \u{25BC}"
                                                        }
                                                    } else {
                                                        ""
                                                    };
                                                    ui.label(
                                                        egui::RichText::new(format!("{h}{arrow}"))
                                                            .small()
                                                            .strong(),
                                                    );
                                                }
                                                ui.end_row();
                                                for row in rows.iter().take(50) {
                                                    for cell in row {
                                                        ui.label(
                                                            egui::RichText::new(cell)
                                                                .small()
                                                                .monospace(),
                                                        );
                                                    }
                                                    ui.end_row();
                                                }
                                            },
                                        );
                                    });
                            }
                            ResultCard::Chart {
                                title,
                                label,
                                values,
                            } => {
                                ui.horizontal(|ui| {
                                    ui.strong(title);
                                    if let Some(last) = values.last() {
                                        ui.label(
                                            egui::RichText::new(format!("{label}: {last:.4}"))
                                                .monospace(),
                                        );
                                    }
                                });
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width().min(300.0), 40.0),
                                    egui::Sense::hover(),
                                );
                                draw_sparkline(
                                    ui.painter(),
                                    rect,
                                    values,
                                    egui::Color32::from_rgb(0, 180, 255),
                                );
                            }
                            ResultCard::Gauge {
                                title,
                                label,
                                value,
                                min,
                                max,
                                danger_low,
                                danger_high,
                            } => {
                                ui.horizontal(|ui| {
                                    ui.strong(title);
                                    let color = if *value < *danger_low || *value > *danger_high {
                                        egui::Color32::from_rgb(255, 80, 80)
                                    } else {
                                        egui::Color32::from_rgb(80, 220, 120)
                                    };
                                    ui.label(
                                        egui::RichText::new(format!("{label}: {value:.2}%"))
                                            .strong()
                                            .color(color)
                                            .monospace(),
                                    );
                                });
                                // Draw gauge bar
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(200.0, 12.0),
                                    egui::Sense::hover(),
                                );
                                let painter = ui.painter();
                                painter.rect_filled(rect, 3.0, egui::Color32::from_rgb(40, 40, 50));
                                let range = max - min;
                                if range > 0.0 {
                                    let frac = ((value - min) / range).clamp(0.0, 1.0) as f32;
                                    let fill_rect = egui::Rect::from_min_size(
                                        rect.min,
                                        egui::vec2(rect.width() * frac, rect.height()),
                                    );
                                    let fill_color =
                                        if *value < *danger_low || *value > *danger_high {
                                            egui::Color32::from_rgb(255, 80, 80)
                                        } else {
                                            egui::Color32::from_rgb(80, 220, 120)
                                        };
                                    painter.rect_filled(fill_rect, 3.0, fill_color);
                                    // Danger zone markers
                                    let low_x = rect.min.x
                                        + ((danger_low - min) / range) as f32 * rect.width();
                                    let high_x = rect.min.x
                                        + ((danger_high - min) / range) as f32 * rect.width();
                                    painter.line_segment(
                                        [
                                            egui::pos2(low_x, rect.min.y),
                                            egui::pos2(low_x, rect.max.y),
                                        ],
                                        egui::Stroke::new(1.0, egui::Color32::YELLOW),
                                    );
                                    painter.line_segment(
                                        [
                                            egui::pos2(high_x, rect.min.y),
                                            egui::pos2(high_x, rect.max.y),
                                        ],
                                        egui::Stroke::new(1.0, egui::Color32::YELLOW),
                                    );
                                }
                            }
                        }
                    });
                    ui.separator();
                }

                // ── Log panel header with filter ──
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.bottom_tab, BottomTab::Log, "Log");
                    ui.separator();
                    ui.label(
                        egui::RichText::new("Filter:")
                            .small()
                            .color(egui::Color32::GRAY),
                    );
                    egui::ComboBox::from_id_salt("log_filter")
                        .width(70.0)
                        .selected_text(match self.log_filter {
                            LogFilter::All => "All",
                            LogFilter::Info => "Info",
                            LogFilter::Warn => "Warn",
                            LogFilter::Error => "Error",
                            LogFilter::Trade => "Trade",
                            LogFilter::Alert => "Alert",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.log_filter, LogFilter::All, "All");
                            ui.selectable_value(
                                &mut self.log_filter,
                                LogFilter::Info,
                                "\u{2139} Info",
                            );
                            ui.selectable_value(
                                &mut self.log_filter,
                                LogFilter::Warn,
                                "\u{26A0} Warn",
                            );
                            ui.selectable_value(
                                &mut self.log_filter,
                                LogFilter::Error,
                                "\u{2716} Error",
                            );
                            ui.selectable_value(
                                &mut self.log_filter,
                                LogFilter::Trade,
                                "\u{1F4B0} Trade",
                            );
                            ui.selectable_value(
                                &mut self.log_filter,
                                LogFilter::Alert,
                                "\u{1F514} Alert",
                            );
                        });
                    // Dismiss result card button
                    if self.result_card.is_some() {
                        if ui.small_button("Dismiss Card").clicked() {
                            self.result_card = None;
                        }
                    }
                });
                ui.separator();
                match self.bottom_tab {
                    BottomTab::Log => {
                        egui::ScrollArea::both()
                            .stick_to_bottom(true)
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                for entry in &self.log {
                                    if !entry.matches_filter(self.log_filter) {
                                        continue;
                                    }
                                    let response = ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&entry.display)
                                                .color(entry.color())
                                                .font(egui::FontId::monospace(11.0)),
                                        )
                                        .wrap_mode(egui::TextWrapMode::Extend)
                                        .sense(egui::Sense::click()),
                                    );
                                    // Clickable log entries: detect ticker symbols (ALL CAPS, 2-6 chars)
                                    if response.clicked() {
                                        // Extract first uppercase word that looks like a ticker
                                        if let Some(ticker) =
                                            entry.msg.split_whitespace().find(|w| {
                                                w.len() >= 2
                                                    && w.len() <= 8
                                                    && w.chars().all(|c| {
                                                        c.is_ascii_uppercase()
                                                            || c == '.'
                                                            || c == '/'
                                                    })
                                            })
                                        {
                                            self.symbol_input = ticker.to_string();
                                        }
                                    }
                                }
                            });
                    }
                }
            });

        // ── ADR-094: Toast notification overlay (top-right, stacked) ──────
        self.toasts.retain(|t| !t.is_expired());
        if !self.toasts.is_empty() {
            let mut y_offset = 40.0_f32;
            for (i, toast) in self.toasts.iter_mut().enumerate() {
                let toast_id = egui::Id::new("toast").with(i);
                egui::Area::new(toast_id)
                    .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, y_offset))
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        egui::Frame::popup(ui.style())
                            .fill(egui::Color32::from_rgb(30, 30, 40))
                            .inner_margin(8.0)
                            .rounding(6.0)
                            .stroke(egui::Stroke::new(1.0, toast.color))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(&toast.message).color(toast.color),
                                    );
                                    if toast.dismissable {
                                        if ui.small_button("\u{2716}").clicked() {
                                            toast.dismissed = true;
                                        }
                                    }
                                });
                            });
                    });
                y_offset += 36.0;
            }
        }

        // ── bottom status bar ────────────────────────────────────────────────
        egui::Panel::bottom("status_bar")
            .exact_size(20.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let n_bars = self.charts.first().map(|c| c.bars.len()).unwrap_or(0);
                    let sym = self
                        .charts
                        .first()
                        .map(|c| c.symbol.as_str())
                        .unwrap_or("—");
                    let tf = self
                        .charts
                        .first()
                        .map(|c| c.timeframe.label())
                        .unwrap_or("—");
                    let data_source = self
                        .charts
                        .first()
                        .map(|c| {
                            if c.primary_source.is_empty() {
                                "Data: unresolved".to_string()
                            } else {
                                format!("Data: Auto → {}", cache_source_label(c.primary_source))
                            }
                        })
                        .unwrap_or_else(|| "Data: unresolved".to_string());
                    ui.label(
                        egui::RichText::new(format!("TyphooN Terminal"))
                            .color(QUAKE_CMD)
                            .small()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("|")
                            .color(egui::Color32::from_rgb(40, 50, 70))
                            .small(),
                    );
                    ui.label(
                        egui::RichText::new(format!("{} [{}]", sym, tf))
                            .color(egui::Color32::WHITE)
                            .small()
                            .monospace(),
                    );
                    ui.label(
                        egui::RichText::new("|")
                            .color(egui::Color32::from_rgb(40, 50, 70))
                            .small(),
                    );
                    ui.label(
                        egui::RichText::new(format!("{} bars", n_bars))
                            .color(AXIS_TEXT)
                            .small(),
                    );
                    ui.label(
                        egui::RichText::new("|")
                            .color(egui::Color32::from_rgb(40, 50, 70))
                            .small(),
                    );
                    ui.label(
                        egui::RichText::new(data_source)
                            .color(AXIS_TEXT)
                            .small()
                            .strong(),
                    );
                    if let Some(err) = &self.cache_err {
                        ui.label(
                            egui::RichText::new(format!(" | {}", err))
                                .color(egui::Color32::from_rgb(255, 80, 80))
                                .small(),
                        );
                    }
                    // Right-aligned: account info
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.broker_connected {
                            ui.label(
                                egui::RichText::new("|")
                                    .color(egui::Color32::from_rgb(40, 50, 70))
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!("{} pos", self.live_positions.len()))
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                        }
                    });
                });
            });
    }
}
