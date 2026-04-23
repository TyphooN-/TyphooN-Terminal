use super::*;

impl TyphooNApp {
    pub(super) fn render_cache_stats_window(&mut self, ctx: &egui::Context) {
        if !self.show_cache_stats {
            return;
        }
        egui::Window::new("Cache Statistics")
            .open(&mut self.show_cache_stats)
            .resizable(true)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                ui.heading("SQLite Cache");
                ui.separator();
                if let Some((rows, kv, size)) = self.bg.cache_stats {
                    ui.label(format!("Bar entries: {}", rows));
                    ui.label(format!("KV entries: {}", kv));
                    ui.label(format!("DB size: {} KB", size / 1024));

                    let total = (rows + kv) as f32;
                    if total > 0.0 {
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("Entry Distribution").strong());
                        let bar_w = 380.0_f32;
                        let bar_h = 20.0_f32;
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(bar_w, bar_h), egui::Sense::hover());
                        let painter = ui.painter_at(rect);
                        let bar_frac = rows as f32 / total;
                        let bar_px = bar_frac * bar_w;
                        painter.rect_filled(
                            egui::Rect::from_min_size(rect.min, egui::vec2(bar_px, bar_h)),
                            2.0,
                            egui::Color32::from_rgb(0, 188, 212),
                        );
                        painter.rect_filled(
                            egui::Rect::from_min_size(
                                egui::pos2(rect.left() + bar_px, rect.top()),
                                egui::vec2(bar_w - bar_px, bar_h),
                            ),
                            2.0,
                            egui::Color32::from_rgb(255, 152, 0),
                        );
                        if bar_px > 50.0 {
                            painter.text(
                                egui::pos2(rect.left() + bar_px * 0.5, rect.center().y),
                                egui::Align2::CENTER_CENTER,
                                format!("Bars {}", rows),
                                egui::FontId::proportional(9.0),
                                egui::Color32::WHITE,
                            );
                        }
                        if bar_w - bar_px > 50.0 {
                            painter.text(
                                egui::pos2(
                                    rect.left() + bar_px + (bar_w - bar_px) * 0.5,
                                    rect.center().y,
                                ),
                                egui::Align2::CENTER_CENTER,
                                format!("KV {}", kv),
                                egui::FontId::proportional(9.0),
                                egui::Color32::WHITE,
                            );
                        }
                    }
                }
                ui.add_space(10.0);
                if !self.bg.detailed_stats.is_empty() {
                    ui.heading("Cached Symbols");
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .max_height(250.0)
                        .show(ui, |ui| {
                            egui::Grid::new("cache_detail")
                                .striped(true)
                                .num_columns(3)
                                .show(ui, |ui| {
                                    ui.strong("Key");
                                    ui.strong("Bars");
                                    ui.strong("Size");
                                    ui.end_row();
                                    for (key, count, _) in &self.bg.detailed_stats {
                                        if key.contains(":__") {
                                            continue;
                                        }
                                        let size_label = self
                                            .bg
                                            .cache_blob_sizes
                                            .get(key.as_str())
                                            .copied()
                                            .map(format_bytes_human)
                                            .unwrap_or_else(|| "\u{2026}".to_string());
                                        ui.label(key);
                                        ui.label(format!("{}", count));
                                        ui.label(size_label);
                                        ui.end_row();
                                    }
                                });
                        });
                }
                if self.cache.is_none() {
                    ui.label(
                        egui::RichText::new("Cache not available")
                            .color(egui::Color32::from_rgb(255, 80, 80)),
                    );
                }
            });
    }

    pub(super) fn refresh_storage_snapshot_from_cache(&mut self) -> Result<(), String> {
        let Some(cache) = self.cache.clone() else {
            return Ok(());
        };
        let cache_stats = cache.stats()?;
        let detailed_rows = cache.detailed_stats_with_size()?;
        apply_storage_snapshot(&mut self.bg, cache_stats, detailed_rows);
        self.bg_rev = self.bg_rev.wrapping_add(1);
        Ok(())
    }

    pub(super) fn refresh_storage_snapshot_after_action(&mut self, action: &str) {
        if let Err(e) = self.refresh_storage_snapshot_from_cache() {
            self.log.push_back(LogEntry::err(format!(
                "Storage refresh after {} failed: {}",
                action, e
            )));
        }
    }

    pub(super) fn render_storage_table(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Filter:");
            if ui
                .add(
                    egui::TextEdit::singleline(&mut self.storage_filter)
                        .desired_width(200.0)
                        .hint_text("symbol or prefix..."),
                )
                .changed()
            {
                // Editing the filter invalidates the pending bulk-delete confirm,
                // so the red button never targets a set the user hasn't actually seen.
                self.storage_delete_filtered_confirm = false;
            }
            if ui.small_button("Clear").clicked() {
                self.storage_filter.clear();
                self.storage_page = 0;
                self.storage_delete_filtered_confirm = false;
            }
        });
        ui.separator();

        // Own the filtered rows so the UI can mutate `self` later in the frame
        // without borrowing `self.bg.detailed_stats` across nested closures.
        let filter = self.storage_filter.to_uppercase();
        let filtered: Vec<(String, i64, i64)> = self
            .bg
            .detailed_stats
            .iter()
            .filter(|(key, _, _)| filter.is_empty() || key.to_uppercase().contains(&filter))
            .map(|(key, count, ts)| (key.clone(), *count, *ts))
            .collect();

        let page_size = 200;
        let total = filtered.len();
        let total_pages = (total + page_size - 1) / page_size;
        if self.storage_page >= total_pages && total_pages > 0 {
            self.storage_page = total_pages - 1;
        }
        let page_start = self.storage_page * page_size;
        let page_end = (page_start + page_size).min(total);
        let page_rows: Vec<(String, i64, i64)> = filtered[page_start..page_end].to_vec();

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{} entries", total))
                    .small()
                    .color(AXIS_TEXT),
            );
            if !self.bg.accounts.is_empty() {
                ui.label(
                    egui::RichText::new(format!("| {} DARWIN accounts", self.bg.accounts.len()))
                        .small()
                        .color(AXIS_TEXT),
                );
            }

            let can_bulk = !filter.is_empty() && total > 0;
            if self.storage_delete_filtered_confirm {
                if ui
                    .add_enabled(
                        can_bulk,
                        egui::Button::new(
                            egui::RichText::new(format!("Confirm delete {} filtered?", total))
                                .color(egui::Color32::from_rgb(231, 76, 60))
                                .small()
                                .strong(),
                        ),
                    )
                    .clicked()
                {
                    let keys: Vec<String> = filtered.iter().map(|(key, _, _)| key.clone()).collect();
                    if let Some(cache) = self.cache.clone() {
                        let result = cache.delete_keys(&keys);
                        match result {
                            Ok(deleted) => {
                                let size_now = cache
                                    .stats()
                                    .ok()
                                    .map(|(_, _, bytes)| format_bytes_human(bytes))
                                    .unwrap_or_else(|| "?".to_string());
                                self.log.push_back(LogEntry::info(format!(
                                    "Deleted {} filtered cache entries, DB now {}",
                                    deleted, size_now
                                )));
                            }
                            Err(e) => self
                                .log
                                .push_back(LogEntry::err(format!("Delete filtered failed: {}", e))),
                        }
                        self.refresh_storage_snapshot_after_action("filtered delete");
                    }
                    self.storage_delete_filtered_confirm = false;
                    self.storage_delete_confirm = None;
                    self.storage_page = 0;
                }
                if ui
                    .small_button(egui::RichText::new("Cancel").small())
                    .clicked()
                {
                    self.storage_delete_filtered_confirm = false;
                }
            } else if ui
                .add_enabled(
                    can_bulk,
                    egui::Button::new(
                        egui::RichText::new(format!("Delete {} filtered", total))
                            .color(egui::Color32::from_rgb(231, 76, 60))
                            .small(),
                    ),
                )
                .on_hover_text(
                    "Delete every cache entry currently shown by the filter. Requires a non-empty filter.",
                )
                .on_disabled_hover_text(
                    "Enter a filter first — bulk delete is disabled for empty filter.",
                )
                .clicked()
            {
                self.storage_delete_filtered_confirm = true;
            }
        });

        if total_pages > 1 {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.storage_page > 0,
                        egui::Button::new(egui::RichText::new("◀ Prev").small()),
                    )
                    .clicked()
                {
                    self.storage_page = self.storage_page.saturating_sub(1);
                }
                ui.label(
                    egui::RichText::new(format!(
                        "Page {} / {}",
                        self.storage_page + 1,
                        total_pages
                    ))
                    .small()
                    .color(AXIS_TEXT),
                );
                if ui
                    .add_enabled(
                        self.storage_page + 1 < total_pages,
                        egui::Button::new(egui::RichText::new("Next ▶").small()),
                    )
                    .clicked()
                {
                    self.storage_page += 1;
                }
            });
        }

        let pending_mt5: std::collections::HashSet<(String, String)> = self
            .mt5_gap_requests
            .iter()
            .map(|(symbol, tf, _, _)| (symbol.clone(), tf.clone()))
            .collect();
        let capped_mt5: std::collections::HashSet<(String, String)> = self
            .mt5_shallow_saturation
            .iter()
            .filter(|(_, (_, noops))| *noops >= 2)
            .map(|(key, _)| key.clone())
            .collect();

        let avail = ui.available_height().max(200.0);
        egui::ScrollArea::vertical()
            .id_salt("storage_scroll")
            .min_scrolled_height(avail)
            .auto_shrink(false)
            .show(ui, |ui| {
                egui::Grid::new("storage_grid")
                    .striped(true)
                    .num_columns(8)
                    .min_col_width(60.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Key").color(AXIS_TEXT).small().strong());
                        ui.label(
                            egui::RichText::new("Bars")
                                .color(AXIS_TEXT)
                                .small()
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("Size")
                                .color(AXIS_TEXT)
                                .small()
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("First Bar")
                                .color(AXIS_TEXT)
                                .small()
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("Last Bar")
                                .color(AXIS_TEXT)
                                .small()
                                .strong(),
                        );
                        ui.label(egui::RichText::new("Age").color(AXIS_TEXT).small().strong());
                        ui.label(
                            egui::RichText::new("Status")
                                .color(AXIS_TEXT)
                                .small()
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("Action")
                                .color(AXIS_TEXT)
                                .small()
                                .strong(),
                        );
                        ui.end_row();

                        let now = chrono::Utc::now().timestamp();
                        let now_ms = now * 1000;
                        let mut delete_key: Option<String> = None;
                        let frozen_threshold_ms = |tf: &str| -> Option<i64> {
                            let period_ms: i64 = match tf {
                                "1Min" => 60_000,
                                "5Min" => 300_000,
                                "15Min" => 900_000,
                                "30Min" => 1_800_000,
                                "1Hour" => 3_600_000,
                                "4Hour" => 14_400_000,
                                "1Day" => 86_400_000,
                                "1Week" => 604_800_000,
                                "1Month" => 2_592_000_000,
                                _ => return None,
                            };
                            Some(period_ms * 24)
                        };
                        let fmt_date = |ms: i64| -> String {
                            if ms <= 0 {
                                "\u{2014}".to_string()
                            } else {
                                chrono::DateTime::from_timestamp_millis(ms)
                                    .map(|d| d.format("%Y-%m-%d").to_string())
                                    .unwrap_or_else(|| "?".to_string())
                            }
                        };

                        for (key, count, ts) in &page_rows {
                            let key_color = if key.starts_with("mt5:") {
                                egui::Color32::from_rgb(26, 188, 156)
                            } else if key.starts_with("kraken:") {
                                egui::Color32::from_rgb(255, 130, 60)
                            } else if key.starts_with("alpaca:") {
                                egui::Color32::from_rgb(52, 152, 219)
                            } else if key.starts_with("tastytrade:") {
                                egui::Color32::from_rgb(170, 100, 220)
                            } else if key.starts_with("cryptocompare:") {
                                egui::Color32::from_rgb(200, 170, 80)
                            } else {
                                egui::Color32::from_rgb(180, 180, 190)
                            };
                            ui.label(
                                egui::RichText::new(key.as_str())
                                    .color(key_color)
                                    .small()
                                    .monospace(),
                            );
                            ui.label(egui::RichText::new(format!("{}", count)).small());
                            let size_label = self
                                .bg
                                .cache_blob_sizes
                                .get(key.as_str())
                                .copied()
                                .map(format_bytes_human)
                                .unwrap_or_else(|| "\u{2026}".to_string());
                            ui.label(
                                egui::RichText::new(size_label)
                                    .color(AXIS_TEXT)
                                    .small()
                                    .monospace(),
                            );

                            let range = self.bg.bar_ts_cache.get(key.as_str()).copied();
                            let (first_ms, last_ms) = match range {
                                Some((first, last, _)) => (first, last),
                                None => (0, 0),
                            };
                            let first_label = if range.is_none() {
                                "\u{2026}".to_string()
                            } else {
                                fmt_date(first_ms)
                            };
                            let last_label = if range.is_none() {
                                "\u{2026}".to_string()
                            } else {
                                fmt_date(last_ms)
                            };
                            ui.label(egui::RichText::new(first_label).color(AXIS_TEXT).small());
                            ui.label(egui::RichText::new(last_label).color(AXIS_TEXT).small());

                            let age_secs = now - ts;
                            let age_str = if age_secs < 3600 {
                                format!("{}m", age_secs / 60)
                            } else if age_secs < 86400 {
                                format!("{}h", age_secs / 3600)
                            } else {
                                format!("{}d", age_secs / 86400)
                            };
                            ui.label(egui::RichText::new(age_str).color(AXIS_TEXT).small());

                            let tf_suffix = key.rsplit(':').next().unwrap_or("");
                            let parts: Vec<&str> = key.splitn(3, ':').collect();
                            let mt5_pair: Option<(String, String)> =
                                if parts.len() == 3 && parts[0] == "mt5" {
                                    Some((parts[1].to_string(), parts[2].to_string()))
                                } else {
                                    None
                                };
                            let status_label = if range.is_none() {
                                ("\u{2026}", AXIS_TEXT)
                            } else if last_ms <= 0 {
                                ("empty", egui::Color32::from_rgb(150, 150, 150))
                            } else if mt5_pair
                                .as_ref()
                                .is_some_and(|pair| pending_mt5.contains(pair))
                            {
                                ("pending", egui::Color32::from_rgb(241, 196, 15))
                            } else if mt5_pair
                                .as_ref()
                                .is_some_and(|pair| capped_mt5.contains(pair))
                            {
                                ("capped", egui::Color32::from_rgb(230, 140, 60))
                            } else if let Some(thresh) = frozen_threshold_ms(tf_suffix) {
                                let lag_ms = now_ms - last_ms;
                                if lag_ms > thresh {
                                    ("FROZEN", egui::Color32::from_rgb(231, 76, 60))
                                } else {
                                    ("ok", egui::Color32::from_rgb(26, 188, 156))
                                }
                            } else {
                                ("?", AXIS_TEXT)
                            };
                            ui.label(
                                egui::RichText::new(status_label.0)
                                    .color(status_label.1)
                                    .small()
                                    .strong(),
                            );

                            if self.storage_delete_confirm.as_deref() == Some(key.as_str()) {
                                if ui
                                    .small_button(
                                        egui::RichText::new("Confirm?")
                                            .color(egui::Color32::from_rgb(231, 76, 60)),
                                    )
                                    .clicked()
                                {
                                    delete_key = Some(key.clone());
                                    self.storage_delete_confirm = None;
                                }
                            } else if ui
                                .small_button(egui::RichText::new("Del").color(AXIS_TEXT))
                                .clicked()
                            {
                                self.storage_delete_confirm = Some(key.clone());
                            }
                            ui.end_row();
                        }

                        if let Some(key) = delete_key {
                            if let Some(cache) = self.cache.clone() {
                                let single_key = [key.clone()];
                                match cache.delete_keys(&single_key) {
                                    Ok(_) => self.log.push_back(LogEntry::info(format!(
                                        "Deleted cache key: {}",
                                        key
                                    ))),
                                    Err(e) => self.log.push_back(LogEntry::err(format!(
                                        "Delete cache key failed: {}",
                                        e
                                    ))),
                                }
                                self.refresh_storage_snapshot_after_action("single-key delete");
                            }
                        }
                    });
            });
    }
}
