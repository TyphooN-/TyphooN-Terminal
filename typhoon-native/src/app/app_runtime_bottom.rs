use super::*;

fn merged_source_counts_suffix(
    cache: Option<&SqliteCache>,
    symbol: &str,
    timeframe: Timeframe,
) -> String {
    let Some(cache) = cache else {
        return String::new();
    };
    let counts = chart_merged_source_bar_counts(cache, symbol, timeframe.cache_suffix());
    if counts.is_empty() {
        return String::new();
    }
    let parts: Vec<String> = counts
        .into_iter()
        .map(|(source, count)| format!("{} {}", cache_source_label(source), count))
        .collect();
    format!(" [inputs: {}]", parts.join(" | "))
}

fn chart_data_source_status_label(chart: &ChartState, cache: Option<&SqliteCache>) -> String {
    if chart.primary_source.is_empty() {
        return "Data: unresolved".to_string();
    }

    let effective = if chart.primary_source == "merged" {
        "Merged"
    } else {
        cache_source_label(chart.primary_source)
    };
    let mode = if chart.source_override.is_empty() {
        "Auto"
    } else {
        "Forced"
    };
    let mut label = format!("Data: {mode} → {effective}");
    if chart.primary_source == "merged" {
        label.push_str(&merged_source_counts_suffix(
            cache,
            &chart.symbol,
            chart.timeframe,
        ));
    }
    label
}

#[allow(deprecated)]
impl TyphooNApp {
    pub(super) fn render_bottom_panels(&mut self, root_ui: &mut egui::Ui) {
        let ctx = &root_ui.ctx().clone();
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
            .show(root_ui, |ui| {
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
                        // Rendering every retained log row every frame is extremely
                        // expensive when the terminal is printing high-frequency sync
                        // progress. Virtualize it: layout all rows for scroll height,
                        // paint/click-test only the visible rows. Keep vertical-only
                        // scrolling; horizontal `ScrollArea::both` with unwrapped
                        // long log strings was the dominant bottom-panel stall during
                        // heavy sync.
                        let visible_log_indices: Vec<usize> = self
                            .log
                            .iter()
                            .enumerate()
                            .filter_map(|(idx, entry)| {
                                entry.matches_filter(self.log_filter).then_some(idx)
                            })
                            .collect();
                        let mut selected_ticker: Option<String> = None;
                        egui::ScrollArea::vertical()
                            .stick_to_bottom(true)
                            .auto_shrink(false)
                            .show_rows(ui, 14.0, visible_log_indices.len(), |ui, row_range| {
                                for row_idx in row_range {
                                    let Some(entry) = visible_log_indices
                                        .get(row_idx)
                                        .and_then(|idx| self.log.get(*idx))
                                    else {
                                        continue;
                                    };
                                    let response = ui.add_sized(
                                        [ui.available_width(), 14.0],
                                        egui::Label::new(
                                            egui::RichText::new(&entry.display)
                                                .color(entry.color())
                                                .font(egui::FontId::monospace(11.0)),
                                        )
                                        .halign(egui::Align::LEFT)
                                        .wrap_mode(egui::TextWrapMode::Truncate)
                                        .sense(egui::Sense::click()),
                                    );
                                    // Clickable log entries: detect ticker symbols (ALL CAPS, 2-8 chars)
                                    if response.clicked() {
                                        selected_ticker = entry
                                            .msg
                                            .split_whitespace()
                                            .find(|w| {
                                                w.len() >= 2
                                                    && w.len() <= 8
                                                    && w.chars().all(|c| {
                                                        c.is_ascii_uppercase()
                                                            || c == '.'
                                                            || c == '/'
                                                    })
                                            })
                                            .map(|ticker| ticker.to_string());
                                    }
                                }
                            });
                        if let Some(ticker) = selected_ticker {
                            self.symbol_input = ticker;
                        }
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
                            .corner_radius(6.0)
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
            .show(root_ui, |ui| {
                ui.horizontal(|ui| {
                    // Footer tracks the chart you're actually viewing (focused
                    // MTF cell, else the active tab) — not charts[0]. Symbol/TF/
                    // company/bar-count already appear in the chart title and tab
                    // bar, so the status bar carries only the resolved data
                    // source, position count, and any cache error.
                    let data_source = self
                        .charts
                        .get(self.mtf_focused.unwrap_or(self.active_tab))
                        .map(|c| chart_data_source_status_label(c, self.cache.as_deref()))
                        .unwrap_or_else(|| "Data: unresolved".to_string());
                    ui.label(
                        egui::RichText::new("TyphooN Terminal")
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
