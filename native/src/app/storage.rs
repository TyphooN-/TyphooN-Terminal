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

    pub(super) fn disabled_kraken_quote_cache_keys(&self) -> Vec<String> {
        const FIAT_QUOTES: [&str; 10] = [
            "USD", "USDT", "USDC", "USDG", "EUR", "GBP", "CAD", "AUD", "JPY", "CHF",
        ];
        const FIAT_BASES: [&str; 7] = ["USD", "EUR", "GBP", "CAD", "AUD", "JPY", "CHF"];

        let mut keys = Vec::new();
        for (key, _, _) in &self.bg.detailed_stats {
            let Some(rest) = key.strip_prefix("kraken:") else {
                continue;
            };
            let Some((symbol, _timeframe)) = rest.split_once(':') else {
                continue;
            };
            let symbol = typhoon_engine::core::kraken::normalize_pair_symbol(symbol);
            let Some(quote) = Self::kraken_symbol_quote(&symbol) else {
                continue;
            };
            if !FIAT_QUOTES.contains(&quote) || self.crypto_fiat_quote_scrape_enabled(quote) {
                continue;
            }
            let base = symbol
                .strip_suffix(quote)
                .unwrap_or(symbol.as_str())
                .trim_end_matches('/');
            if FIAT_BASES.contains(&base) {
                continue;
            }
            keys.push(key.clone());
        }
        keys.sort();
        keys.dedup();
        keys
    }

    pub(super) fn render_storage_table(&mut self, ui: &mut egui::Ui) {
        if !self.alpaca_no_data_loaded {
            self.alpaca_no_data_load();
        }
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "Alpaca no-data marks: {}",
                    self.alpaca_no_data_pairs.len()
                ))
                .small()
                .color(AXIS_TEXT),
            );
            if ui
                .add_enabled(
                    !self.alpaca_no_data_pairs.is_empty(),
                    egui::Button::new(
                        egui::RichText::new("Clear no-data marks")
                            .small()
                            .color(egui::Color32::from_rgb(231, 76, 60)),
                    ),
                )
                .on_hover_text(
                    "Remove persisted Alpaca no-data tombstones so automated sync can try those symbols again.",
                )
                .clicked()
            {
                let cleared = self.alpaca_no_data_pairs.len();
                self.alpaca_no_data_clear_all();
                self.log.push_back(LogEntry::info(format!(
                    "Cleared {} Alpaca no-data mark(s)",
                    cleared
                )));
            }
        });
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "Unresolvable broker pairs: {}",
                    self.unresolvable_pairs.len()
                ))
                .small()
                .color(AXIS_TEXT),
            );
            if ui
                .add_enabled(
                    !self.unresolvable_pairs.is_empty(),
                    egui::Button::new(
                        egui::RichText::new("Clear unresolvable")
                            .small()
                            .color(egui::Color32::from_rgb(231, 76, 60)),
                    ),
                )
                .on_hover_text(
                    "Remove generic broker unresolvable tombstones so sync can retry those broker/symbol/timeframe pairs.",
                )
                .clicked()
            {
                let cleared = self.unresolvable_pairs.len();
                self.unresolvable_clear_all();
                self.log.push_back(LogEntry::info(format!(
                    "Cleared {} unresolvable broker pair(s)",
                    cleared
                )));
            }
        });
        if !self.unresolvable_pairs.is_empty() {
            egui::CollapsingHeader::new("Unresolvable")
                .default_open(false)
                .show(ui, |ui| {
                    let mut entries: Vec<_> = self.unresolvable_pairs.values().cloned().collect();
                    entries.sort_by(|a, b| {
                        a.broker.cmp(&b.broker).then(a.symbol.cmp(&b.symbol)).then(
                            sync_timeframe_sort_key(&a.timeframe)
                                .cmp(&sync_timeframe_sort_key(&b.timeframe)),
                        )
                    });
                    egui::ScrollArea::vertical()
                        .max_height(140.0)
                        .show(ui, |ui| {
                            egui::Grid::new("unresolvable_pairs_grid")
                                .striped(true)
                                .spacing([10.0, 2.0])
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new("Broker").small().strong());
                                    ui.label(egui::RichText::new("Symbol").small().strong());
                                    ui.label(egui::RichText::new("TF").small().strong());
                                    ui.label(egui::RichText::new("Reason").small().strong());
                                    ui.end_row();
                                    for entry in entries.iter().take(200) {
                                        ui.label(egui::RichText::new(&entry.broker).small());
                                        ui.label(egui::RichText::new(&entry.symbol).small());
                                        ui.label(egui::RichText::new(&entry.timeframe).small());
                                        ui.label(
                                            egui::RichText::new(
                                                entry.reason.chars().take(120).collect::<String>(),
                                            )
                                            .small(),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                });
        }
        let disabled_kraken_quote_keys = self.disabled_kraken_quote_cache_keys();
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "Disabled Kraken quote caches: {}",
                    disabled_kraken_quote_keys.len()
                ))
                .small()
                .color(AXIS_TEXT),
            );
            if self.storage_prune_disabled_kraken_quotes_confirm {
                if ui
                    .add_enabled(
                        !disabled_kraken_quote_keys.is_empty(),
                        egui::Button::new(
                            egui::RichText::new(format!(
                                "Confirm prune {} disabled Kraken quotes?",
                                disabled_kraken_quote_keys.len()
                            ))
                            .small()
                            .strong()
                            .color(egui::Color32::from_rgb(231, 76, 60)),
                        ),
                    )
                    .on_hover_text("Delete Kraken Spot crypto/fiat cache entries whose quote currency is disabled in the global crypto quote filters. Keeps enabled USD/stablecoin caches and pure fiat FX pairs.")
                    .clicked()
                {
                    if let Some(cache) = self.cache.clone() {
                        match cache.delete_keys(&disabled_kraken_quote_keys) {
                            Ok(deleted) => self.log.push_back(LogEntry::info(format!(
                                "Pruned {} disabled Kraken quote cache entr{}",
                                deleted,
                                if deleted == 1 { "y" } else { "ies" }
                            ))),
                            Err(e) => self.log.push_back(LogEntry::err(format!(
                                "Prune disabled Kraken quotes failed: {}",
                                e
                            ))),
                        }
                        self.refresh_storage_snapshot_after_action("disabled Kraken quote prune");
                    }
                    self.pending_kraken_fetches.clear();
                    self.storage_prune_disabled_kraken_quotes_confirm = false;
                    self.storage_delete_confirm = None;
                    self.storage_delete_filtered_confirm = false;
                    self.storage_page = 0;
                }
                if ui.small_button(egui::RichText::new("Cancel").small()).clicked() {
                    self.storage_prune_disabled_kraken_quotes_confirm = false;
                }
            } else if ui
                .add_enabled(
                    !disabled_kraken_quote_keys.is_empty(),
                    egui::Button::new(
                        egui::RichText::new("Prune disabled Kraken quotes")
                            .small()
                            .color(egui::Color32::from_rgb(231, 76, 60)),
                    ),
                )
                .on_hover_text("Delete cached Kraken Spot crypto/fiat pairs whose quote currency is disabled. Useful after narrowing to USD/stablecoin quotes.")
                .on_disabled_hover_text("No cached Kraken Spot entries currently match disabled quote filters.")
                .clicked()
            {
                self.storage_prune_disabled_kraken_quotes_confirm = true;
            }
        });
        ui.separator();
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
