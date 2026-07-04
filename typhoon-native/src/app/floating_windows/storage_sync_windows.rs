use super::*;

impl TyphooNApp {
    pub(super) fn render_storage_sync_windows(&mut self, ctx: &egui::Context) {
        self.render_cache_stats_window(ctx);

        // Storage Manager
        if self.show_storage {
            let mut storage_save_after = false;
            let mut show_storage = self.show_storage;
            egui::Window::new("Storage Manager")
                        .open(&mut show_storage)
                        .resizable(true).default_size([650.0, 500.0])
                        .scroll([false, true])
                        .show(ctx, |ui| {
                            // Summary stats at top
                            if let Some((rows, kv, size)) = self.bg.cache_stats {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(format!("Bar entries: {} | KV entries: {} | DB size on disk: {:.1} MB", rows, kv, size as f64 / 1024.0 / 1024.0)).small());
                                });
                                // One-line bar-sync banner — per-broker % healthy with a
                                // `[Details]` button opening the full Sync Status window.
                                let stats_rows = self.compute_bar_sync_rows();
                                let totals = compute_bar_sync_broker_totals(&stats_rows);
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Sync:").color(AXIS_TEXT).small().strong());
                                    for (broker, total, _healthy, pct) in &totals {
                                        let color = if *total == 0 {
                                            egui::Color32::from_rgb(150, 150, 150)
                                        } else if *pct >= 90.0 {
                                            egui::Color32::from_rgb(26, 188, 156)
                                        } else if *pct >= 50.0 {
                                            egui::Color32::from_rgb(241, 196, 15)
                                        } else {
                                            egui::Color32::from_rgb(231, 76, 60)
                                        };
                                        ui.label(egui::RichText::new(format!("{} {:.1}%", broker, pct)).color(color).small().monospace());
                                        ui.label(egui::RichText::new("|").color(AXIS_TEXT).small());
                                    }
                                    if ui.small_button(egui::RichText::new("Details").small()).clicked() {
                                        self.show_sync_status = true;
                                    }
                                });
                                self.render_sync_timeframe_controls(ui, &mut storage_save_after);
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Base bar zstd").color(AXIS_TEXT).small());
                                    let mut level = self.bar_zstd_level;
                                    if ui
                                        .add(
                                            egui::Slider::new(
                                                &mut level,
                                                typhoon_engine::core::cache::MIN_ZSTD_LEVEL
                                                    ..=typhoon_engine::core::cache::MAX_ZSTD_LEVEL,
                                            )
                                            .integer()
                                            .show_value(true),
                                        )
                                        .on_hover_text(
                                            "Compression level for normal foreground bar-cache writes. Lower = faster sync/import writes; higher = smaller disk. Kraken WS hot writes remain fixed at zstd-3; Compact promotes rows to zstd-22.",
                                        )
                                        .changed()
                                    {
                                        self.bar_zstd_level = typhoon_engine::core::cache::set_bar_zstd_level(level);
                                        storage_save_after = true;
                                        self.log.push_back(LogEntry::info(format!(
                                            "Base bar-cache zstd level set to {}",
                                            self.bar_zstd_level
                                        )));
                                    }
                                    if ui.small_button("Fast 3").on_hover_text("Low CPU, larger blobs; good during broad sync.").clicked() {
                                        self.bar_zstd_level = typhoon_engine::core::cache::set_bar_zstd_level(3);
                                        storage_save_after = true;
                                    }
                                    if ui.small_button("Balanced 9").on_hover_text("Middle ground between CPU and disk size.").clicked() {
                                        self.bar_zstd_level = typhoon_engine::core::cache::set_bar_zstd_level(9);
                                        storage_save_after = true;
                                    }
                                    if ui.small_button("Max 22").on_hover_text("Smallest blobs, highest write CPU. Use with care during broad sync.").clicked() {
                                        self.bar_zstd_level = typhoon_engine::core::cache::set_bar_zstd_level(22);
                                        storage_save_after = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    // Guard against double-launch and sync starvation: compaction
                                    // recompresses cache blobs and competes with historical bar
                                    // writers. During broad catch-up it can make Storage look like
                                    // sync has stopped, so keep manual compact out of the hot path
                                    // just like the auto-compact gate does.
                                    let compact_available = !self.auto_compact_in_progress
                                        && !self.heavy_sync_in_progress;
                                    let compact_btn = egui::Button::new(
                                        egui::RichText::new(format!(
                                            "Compact (zstd-{})",
                                            auto_compact::TARGET_LEVEL
                                        ))
                                        .small(),
                                    );
                                    if ui
                                        .add_enabled(compact_available, compact_btn)
                                        .on_disabled_hover_text(if self.auto_compact_in_progress {
                                            "Compaction is already running."
                                        } else {
                                            "Broad market-data sync is active; wait until catch-up settles so compaction does not starve bar writes."
                                        })
                                        .clicked()
                                    {
                                        let db_path = cache_db_path();
                                        let log_tx = self.broker_tx.clone();
                                        let size_before = size;
                                        let _ = log_tx.send(BrokerCmd::CompactStorage { db_path: db_path.clone(), level: auto_compact::TARGET_LEVEL });
                                        self.auto_compact_in_progress = true;
                                        self.auto_compact_started_ms = chrono::Utc::now().timestamp_millis();
                                        self.log.push_back(LogEntry::info(format!(
                                            "Compacting cache at zstd-{} (current: {:.1} MB)... this may take several minutes",
                                            auto_compact::TARGET_LEVEL,
                                            size_before as f64 / 1024.0 / 1024.0
                                        )));
                                    }
                                    ui.label(egui::RichText::new("Recompress cold rows at max level; disabled during heavy sync.").color(AXIS_TEXT).small());
                                });
                                // Auto-compact controls + readout (ADR-089). Manual compact
                                // ignores the auto-enable setting but still respects the
                                // in-progress/heavy-sync safety gates above.
                                ui.horizontal(|ui| {
                                    let auto_label = format!(
                                        "Auto-compact ({})",
                                        auto_compact::schedule_summary(self.auto_compact_schedule)
                                    );
                                    if ui
                                        .checkbox(
                                            &mut self.auto_compact_enabled,
                                            egui::RichText::new(auto_label).small(),
                                        )
                                        .on_hover_text(
                                            "Promote below-target bar-cache entries to zstd-22 during the configured AC + idle window.",
                                        )
                                        .changed()
                                    {
                                        storage_save_after = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    let mut schedule = self.auto_compact_schedule.sanitized();
                                    let mut changed = false;
                                    ui.label(egui::RichText::new("Cadence").color(AXIS_TEXT).small());
                                    let mut preset =
                                        auto_compact::CadencePreset::from_days(schedule.cadence_days);
                                    let preset_before = preset;
                                    egui::ComboBox::from_id_salt("auto_compact_cadence_preset")
                                        .selected_text(preset.label())
                                        .show_ui(ui, |ui| {
                                            for option in [
                                                auto_compact::CadencePreset::Daily,
                                                auto_compact::CadencePreset::Weekly,
                                                auto_compact::CadencePreset::Monthly,
                                                auto_compact::CadencePreset::Yearly,
                                                auto_compact::CadencePreset::Custom,
                                            ] {
                                                ui.selectable_value(&mut preset, option, option.label());
                                            }
                                        });
                                    if preset != preset_before {
                                        let new_days = preset.to_days(schedule.cadence_days);
                                        if new_days != schedule.cadence_days {
                                            schedule.cadence_days = new_days;
                                            changed = true;
                                        }
                                    }
                                    ui.label(egui::RichText::new("Every").color(AXIS_TEXT).small());
                                    changed |= ui
                                        .add(egui::DragValue::new(&mut schedule.cadence_days).range(1..=365).suffix("d"))
                                        .changed();
                                    // Sub-weekly cadences ignore the weekday gate — hide the picker
                                    // so the UI matches what evaluate_gate actually checks.
                                    if schedule.cadence_days >= 7 {
                                        egui::ComboBox::from_id_salt("auto_compact_weekday")
                                            .selected_text(auto_compact::weekday_label(schedule.window_weekday))
                                            .show_ui(ui, |ui| {
                                                for day in 0..=6 {
                                                    changed |= ui
                                                        .selectable_value(
                                                            &mut schedule.window_weekday,
                                                            day,
                                                            auto_compact::weekday_label(day),
                                                        )
                                                        .changed();
                                                }
                                            });
                                    }
                                    ui.label(egui::RichText::new("Start").color(AXIS_TEXT).small());
                                    changed |= ui
                                        .add(egui::DragValue::new(&mut schedule.window_hour_start).range(0..=23).suffix(":00"))
                                        .changed();
                                    ui.label(egui::RichText::new("End").color(AXIS_TEXT).small());
                                    changed |= ui
                                        .add(egui::DragValue::new(&mut schedule.window_hour_end).range(1..=24).suffix(":00"))
                                        .changed();
                                    ui.label(egui::RichText::new("Min rows").color(AXIS_TEXT).small());
                                    changed |= ui
                                        .add(egui::DragValue::new(&mut schedule.uncompacted_threshold).range(1..=1_000_000))
                                        .changed();
                                    if changed {
                                        self.auto_compact_schedule = schedule.sanitized();
                                        storage_save_after = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    let now_ms = chrono::Utc::now().timestamp_millis();
                                    let last_label = if self.auto_compact_last_run_ms <= 0 {
                                        "never".to_string()
                                    } else {
                                        let secs = ((now_ms - self.auto_compact_last_run_ms) / 1000).max(0);
                                        if secs < 3600 {
                                            format!("{}m ago", secs / 60)
                                        } else if secs < 86_400 {
                                            format!("{}h ago", secs / 3600)
                                        } else {
                                            format!("{}d ago", secs / 86_400)
                                        }
                                    };
                                    ui.label(
                                        egui::RichText::new(format!("last: {}", last_label))
                                            .color(AXIS_TEXT)
                                            .small(),
                                    );
                                    let next_ms = auto_compact::next_eligible_time_ms(
                                        self.auto_compact_schedule,
                                        self.auto_compact_last_run_ms,
                                    );
                                    let next_label = if next_ms <= now_ms + 60_000 {
                                        "now".to_string()
                                    } else {
                                        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(next_ms)
                                            .map(|dt| {
                                                dt.with_timezone(&chrono::Local)
                                                    .format("%a %H:%M")
                                                    .to_string()
                                            })
                                            .unwrap_or_else(|| "unknown".to_string())
                                    };
                                    ui.label(
                                        egui::RichText::new(format!("next: {}", next_label))
                                            .color(AXIS_TEXT)
                                            .small(),
                                    );
                                    if let Some(reason) = self.auto_compact_last_skip.as_deref() {
                                        ui.label(
                                            egui::RichText::new(format!("(skip: {})", reason))
                                                .color(AXIS_TEXT)
                                                .small(),
                                        );
                                    }
                                    if self.auto_compact_in_progress {
                                        ui.label(
                                            egui::RichText::new("running…")
                                                .color(egui::Color32::from_rgb(241, 196, 15))
                                                .small()
                                                .strong(),
                                        );
                                    }
                                });
                                ui.horizontal(|ui| {
                                    if ui.button(egui::RichText::new("Reclaim Free Space").small()).clicked() {
                                        if let Some(cache) = self.cache.clone() {
                                            let result = cache.reclaim_space();
                                            match result {
                                                Ok((before, after)) => self.log.push_back(LogEntry::info(format!(
                                                    "Reclaimed SQLite free pages: {} -> {}",
                                                    format_bytes_human(before),
                                                    format_bytes_human(after)
                                                ))),
                                                Err(e) => self.log.push_back(LogEntry::err(format!(
                                                    "Reclaim storage failed: {}",
                                                    e
                                                ))),
                                            }
                                            self.refresh_storage_snapshot_after_action("reclaim");
                                        }
                                    }
                                    ui.label(
                                        egui::RichText::new(
                                            "Run WAL checkpoint + VACUUM after prior deletes to physically shrink the DB file.",
                                        )
                                        .color(AXIS_TEXT)
                                        .small(),
                                    );
                                });
                                // Purge All Bar Data
                                ui.horizontal(|ui| {
                                    if self.storage_purge_bars_confirm {
                                        ui.label(egui::RichText::new("This will delete ALL cached bar data. This is NOT reversible!").color(egui::Color32::from_rgb(231, 76, 60)).small());
                                        if ui.button(egui::RichText::new("Yes, Delete All Bars").color(egui::Color32::from_rgb(231, 76, 60)).small()).clicked() {
                                            self.storage_purge_bars_confirm = false;
                                            if let Some(cache) = self.cache.clone() {
                                                let result = cache.delete_all_bars();
                                                match result {
                                                    Ok(n) => {
                                                        let size_now = cache
                                                            .stats()
                                                            .ok()
                                                            .map(|(_, _, bytes)| format_bytes_human(bytes))
                                                            .unwrap_or_else(|| "?".to_string());
                                                        self.log.push_back(LogEntry::info(format!(
                                                            "Purged all bar data: {} entries deleted, DB now {}",
                                                            n, size_now
                                                        )));
                                                    }
                                                    Err(e) => self.log.push_back(LogEntry::err(format!("Purge bars failed: {}", e))),
                                                }
                                                self.refresh_storage_snapshot_after_action("purge all bars");
                                            }
                                        }
                                        if ui.small_button(egui::RichText::new("Cancel").small()).clicked() {
                                            self.storage_purge_bars_confirm = false;
                                        }
                                    } else {
                                        if ui.button(egui::RichText::new("Purge All Bar Data").color(egui::Color32::from_rgb(231, 76, 60)).small()).clicked() {
                                            self.storage_purge_bars_confirm = true;
                                            self.storage_purge_broker_confirm = None;
                                            self.storage_purge_timeframe_confirm = false;
                                            self.storage_purge_news_confirm = false;
                                        }
                                    }
                                });
                                ui.horizontal(|ui| {
                                    let broker_label = |prefix: &str| match prefix {
                                        "alpaca" => "Alpaca",
                                        _ => "Broker",
                                    };
                                    ui.label(
                                        egui::RichText::new("Nuclear broker purge:")
                                            .color(AXIS_TEXT)
                                            .small(),
                                    );
                                    if let Some(prefix) = self.storage_purge_broker_confirm.clone() {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "Delete all {} cache rows from storage?",
                                                broker_label(&prefix)
                                            ))
                                            .color(egui::Color32::from_rgb(231, 76, 60))
                                            .small(),
                                        );
                                        if ui
                                            .button(
                                                egui::RichText::new("Yes, Delete Broker")
                                                    .color(egui::Color32::from_rgb(231, 76, 60))
                                                    .small(),
                                            )
                                            .clicked()
                                        {
                                            self.storage_purge_broker_confirm = None;
                                            if let Some(cache) = self.cache.clone() {
                                                let result = cache.delete_broker_data(&prefix);
                                                match result {
                                                    Ok(n) => {
                                                        let size_now = cache
                                                            .stats()
                                                            .ok()
                                                            .map(|(_, _, bytes)| format_bytes_human(bytes))
                                                            .unwrap_or_else(|| "?".to_string());
                                                        self.log.push_back(LogEntry::info(format!(
                                                            "Purged {} cache data: {} rows deleted, DB now {}",
                                                            broker_label(&prefix),
                                                            n,
                                                            size_now
                                                        )));
                                                    }
                                                    Err(e) => self.log.push_back(LogEntry::err(format!(
                                                        "Purge {} failed: {}",
                                                        broker_label(&prefix),
                                                        e
                                                    ))),
                                                }
                                                self.refresh_storage_snapshot_after_action("broker purge");
                                            }
                                        }
                                        if ui.small_button(egui::RichText::new("Cancel").small()).clicked() {
                                            self.storage_purge_broker_confirm = None;
                                        }
                                    } else {
                                        for prefix in ["alpaca"] {
                                            if ui
                                                .button(
                                                    egui::RichText::new(broker_label(prefix))
                                                        .color(egui::Color32::from_rgb(231, 76, 60))
                                                        .small(),
                                                )
                                                .clicked()
                                            {
                                                self.storage_purge_broker_confirm = Some(prefix.to_string());
                                                self.storage_purge_bars_confirm = false;
                                                self.storage_purge_timeframe_confirm = false;
                                                self.storage_purge_news_confirm = false;
                                            }
                                        }
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new("Delete TF across all brokers:")
                                            .color(AXIS_TEXT)
                                            .small(),
                                    );
                                    egui::ComboBox::from_id_salt("storage_delete_timeframe")
                                        .selected_text(sync_timeframe_short_label(&self.storage_delete_timeframe))
                                        .show_ui(ui, |ui| {
                                            for (short, cache) in STANDARD_SYNC_TIMEFRAMES {
                                                ui.selectable_value(
                                                    &mut self.storage_delete_timeframe,
                                                    cache.to_string(),
                                                    format!("{} ({})", short, cache),
                                                );
                                            }
                                        });
                                    if self.storage_purge_timeframe_confirm {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "Delete every {} blob from storage?",
                                                sync_timeframe_short_label(&self.storage_delete_timeframe)
                                            ))
                                            .color(egui::Color32::from_rgb(231, 76, 60))
                                            .small(),
                                        );
                                        if ui
                                            .button(
                                                egui::RichText::new("Yes, Delete TF")
                                                    .color(egui::Color32::from_rgb(231, 76, 60))
                                                    .small(),
                                            )
                                            .clicked()
                                        {
                                            self.storage_purge_timeframe_confirm = false;
                                            if let Some(cache) = self.cache.clone() {
                                                let result = cache.delete_timeframe(&self.storage_delete_timeframe);
                                                match result {
                                                    Ok(n) => {
                                                        let size_now = cache
                                                            .stats()
                                                            .ok()
                                                            .map(|(_, _, bytes)| format_bytes_human(bytes))
                                                            .unwrap_or_else(|| "?".to_string());
                                                        self.log.push_back(LogEntry::info(format!(
                                                            "Purged {} bars across all brokers: {} entries deleted, DB now {}",
                                                            self.storage_delete_timeframe, n, size_now
                                                        )));
                                                    }
                                                    Err(e) => self.log.push_back(LogEntry::err(format!(
                                                        "Purge {} failed: {}",
                                                        self.storage_delete_timeframe, e
                                                    ))),
                                                }
                                                self.refresh_storage_snapshot_after_action("timeframe purge");
                                            }
                                        }
                                        if ui.small_button(egui::RichText::new("Cancel").small()).clicked() {
                                            self.storage_purge_timeframe_confirm = false;
                                        }
                                    } else if ui
                                        .button(
                                            egui::RichText::new("Delete TF")
                                                .color(egui::Color32::from_rgb(231, 76, 60))
                                                .small(),
                                        )
                                        .clicked()
                                    {
                                        self.storage_purge_timeframe_confirm = true;
                                        self.storage_purge_bars_confirm = false;
                                        self.storage_purge_broker_confirm = None;
                                        self.storage_purge_news_confirm = false;
                                    }
                                });
                                // ── News purge by age (slider with date notches) ──
                                // Manual tool only — there is no automatic news TTL
                                // (see ADR-107 + ADR-215). Articles persist
                                // indefinitely; this gives the user a way to
                                // reclaim space without writing SQL.
                                ui.horizontal(|ui| {
                                    // Notches: 1w / 1m / 3m / 6m / 1y / 2y / 5y.
                                    // Days, not seconds, so the cutoff is timezone
                                    // independent and the labels read naturally.
                                    const NEWS_PURGE_NOTCHES_DAYS: &[(i64, &str)] = &[
                                        (7,    "7 days"),
                                        (30,   "30 days"),
                                        (90,   "90 days"),
                                        (180,  "6 months"),
                                        (365,  "1 year"),
                                        (730,  "2 years"),
                                        (1825, "5 years"),
                                    ];
                                    let idx = self
                                        .storage_purge_news_age_idx
                                        .min(NEWS_PURGE_NOTCHES_DAYS.len() - 1);
                                    let (days, label) = NEWS_PURGE_NOTCHES_DAYS[idx];
                                    let cutoff_ts =
                                        chrono::Utc::now().timestamp() - days * 86_400;
                                    let count = self
                                        .cache
                                        .as_ref()
                                        .and_then(|c| c.connection().ok())
                                        .and_then(|conn| {
                                            typhoon_engine::core::news::count_articles_older_than(
                                                &conn, cutoff_ts,
                                            )
                                            .ok()
                                        })
                                        .unwrap_or(0);
                                    ui.label(
                                        egui::RichText::new("Purge news older than:")
                                            .color(AXIS_TEXT)
                                            .small(),
                                    );
                                    let mut slider_idx = idx;
                                    let slider = egui::Slider::new(
                                        &mut slider_idx,
                                        0..=(NEWS_PURGE_NOTCHES_DAYS.len() - 1),
                                    )
                                    .integer()
                                    .show_value(false)
                                    .custom_formatter(|n, _| {
                                        let i = (n as usize)
                                            .min(NEWS_PURGE_NOTCHES_DAYS.len() - 1);
                                        NEWS_PURGE_NOTCHES_DAYS[i].1.to_string()
                                    });
                                    if ui.add(slider).changed() {
                                        self.storage_purge_news_age_idx = slider_idx;
                                        // Cancel any pending confirm if the user is
                                        // re-aiming the slider — they should
                                        // explicitly re-confirm at the new cutoff.
                                        self.storage_purge_news_confirm = false;
                                    }
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "({}) — {} articles affected",
                                            label, count
                                        ))
                                        .color(AXIS_TEXT)
                                        .small(),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    // Re-resolve count for the confirm line so the
                                    // displayed N matches the in-flight slider
                                    // value even on the confirmation frame.
                                    const NEWS_PURGE_NOTCHES_DAYS: &[(i64, &str)] = &[
                                        (7,    "7 days"),
                                        (30,   "30 days"),
                                        (90,   "90 days"),
                                        (180,  "6 months"),
                                        (365,  "1 year"),
                                        (730,  "2 years"),
                                        (1825, "5 years"),
                                    ];
                                    let idx = self
                                        .storage_purge_news_age_idx
                                        .min(NEWS_PURGE_NOTCHES_DAYS.len() - 1);
                                    let (days, label) = NEWS_PURGE_NOTCHES_DAYS[idx];
                                    let cutoff_ts =
                                        chrono::Utc::now().timestamp() - days * 86_400;
                                    if self.storage_purge_news_confirm {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "Delete every news article older than {}? (irreversible)",
                                                label
                                            ))
                                            .color(egui::Color32::from_rgb(231, 76, 60))
                                            .small(),
                                        );
                                        if ui
                                            .button(
                                                egui::RichText::new("Yes, Purge News")
                                                    .color(egui::Color32::from_rgb(231, 76, 60))
                                                    .small(),
                                            )
                                            .clicked()
                                        {
                                            self.storage_purge_news_confirm = false;
                                            if let Some(cache) = self.cache.clone() {
                                                if let Ok(conn) = cache.connection() {
                                                    match typhoon_engine::core::news::purge_older_than(
                                                        &conn, cutoff_ts,
                                                    ) {
                                                        Ok(n) => {
                                                            let size_now = cache
                                                                .stats()
                                                                .ok()
                                                                .map(|(_, _, bytes)| {
                                                                    format_bytes_human(bytes)
                                                                })
                                                                .unwrap_or_else(|| "?".to_string());
                                                            self.log.push_back(LogEntry::info(format!(
                                                                "News purge: removed {} articles older than {}, DB now {}",
                                                                n, label, size_now
                                                            )));
                                                        }
                                                        Err(e) => self.log.push_back(LogEntry::err(
                                                            format!("News purge failed: {}", e),
                                                        )),
                                                    }
                                                }
                                                self.refresh_storage_snapshot_after_action(
                                                    "news age purge",
                                                );
                                            }
                                        }
                                        if ui
                                            .small_button(egui::RichText::new("Cancel").small())
                                            .clicked()
                                        {
                                            self.storage_purge_news_confirm = false;
                                        }
                                    } else if ui
                                        .button(
                                            egui::RichText::new("Purge News")
                                                .color(egui::Color32::from_rgb(231, 76, 60))
                                                .small(),
                                        )
                                        .clicked()
                                    {
                                        self.storage_purge_news_confirm = true;
                                        self.storage_purge_bars_confirm = false;
                                        self.storage_purge_broker_confirm = None;
                                        self.storage_purge_timeframe_confirm = false;
                                    }
                                });
                            }
                            ui.separator();

                            // ─── Cache Location (NAS support) ──────────────────────
                            // Drain any in-flight VACUUM INTO result from the worker thread.
                            if let Some(rx) = &self.storage_cache_move_rx {
                                if let Ok(msg) = rx.try_recv() {
                                    match msg {
                                        Ok(s) => { self.storage_cache_move_result = Some((true, s.clone())); self.log.push_back(LogEntry::info(s)); }
                                        Err(e) => { self.storage_cache_move_result = Some((false, e.clone())); self.log.push_back(LogEntry::err(e)); }
                                    }
                                    self.storage_cache_move_rx = None;
                                }
                            }
                            ui.label(egui::RichText::new("CACHE LOCATION").color(AXIS_TEXT).small().strong());
                            {
                                let default_dir = dirs_home().join("cache");
                                let active_dir = cache_dir();
                                let configured = read_custom_cache_dir();
                                let is_custom_missing = configured.as_ref().map(|p| !p.is_dir()).unwrap_or(false);
                                let is_custom_active = active_dir != default_dir;

                                if is_custom_missing {
                                    let miss = configured.as_ref().unwrap();
                                    ui.colored_label(egui::Color32::from_rgb(231, 76, 60),
                                        egui::RichText::new(format!("⚠ Custom cache UNAVAILABLE: {}", miss.display())).small());
                                    ui.label(egui::RichText::new(format!("Falling back to default: {}", active_dir.display())).small().color(AXIS_TEXT));
                                    ui.label(egui::RichText::new("Mount the drive / restart the NAS, then restart the terminal.").small().color(AXIS_TEXT));
                                } else if is_custom_active {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Custom:").small().color(AXIS_TEXT));
                                        ui.label(egui::RichText::new(active_dir.display().to_string()).small().monospace());
                                    });
                                } else {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Default:").small().color(AXIS_TEXT));
                                        ui.label(egui::RichText::new(active_dir.display().to_string()).small().monospace());
                                    });
                                }

                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("New path:").small());
                                    ui.add(egui::TextEdit::singleline(&mut self.storage_cache_path_input)
                                        .desired_width(420.0)
                                        .hint_text("/mnt/nas/typhoon-cache"));
                                });

                                let in_flight = self.storage_cache_move_rx.is_some();
                                ui.horizontal(|ui| {
                                    let trimmed = self.storage_cache_path_input.trim().to_string();
                                    let enabled = !trimmed.is_empty() && !in_flight;

                                    if ui.add_enabled(enabled, egui::Button::new(egui::RichText::new("Save location (restart required)").small()))
                                        .on_hover_text("Write setting only. Next startup opens/creates cache at this location; old data stays put.")
                                        .clicked()
                                    {
                                        let target = PathBuf::from(&trimmed);
                                        match std::fs::create_dir_all(&target) {
                                            Ok(_) => match write_custom_cache_dir(Some(&target)) {
                                                Ok(_) => {
                                                    self.storage_cache_move_result = Some((true, format!("Saved. Restart terminal to open cache at {}", target.display())));
                                                    self.log.push_back(LogEntry::info(format!("Cache location saved: {} (restart required)", target.display())));
                                                }
                                                Err(e) => { self.storage_cache_move_result = Some((false, format!("Save failed: {}", e))); }
                                            },
                                            Err(e) => { self.storage_cache_move_result = Some((false, format!("mkdir {} failed: {}", target.display(), e))); }
                                        }
                                    }

                                    if ui.add_enabled(enabled && self.cache.is_some(), egui::Button::new(egui::RichText::new("Copy cache here & save").small()))
                                        .on_hover_text("Safely clone the open SQLite DB via VACUUM INTO, then save the setting. Restart required to start using the copy.")
                                        .clicked()
                                    {
                                        let target = PathBuf::from(&trimmed);
                                        let target_db = target.join("typhoon_cache.db");
                                        let (tx, rx) = std::sync::mpsc::channel();
                                        self.storage_cache_move_rx = Some(rx);
                                        self.storage_cache_move_result = Some((true, format!("Copying cache to {} ... this may take several minutes for large caches", target.display())));
                                        if let Some(cache) = self.cache.clone() {
                                            let tx_on_spawn_err = tx.clone();
                                            if let Err(e) = std::thread::Builder::new()
                                                .name("typhoon-cache-vacuum-copy".into())
                                                .spawn(move || {
                                                    if let Err(e) = std::fs::create_dir_all(&target) {
                                                        let _ = tx.send(Err(format!("mkdir {} failed: {}", target.display(), e)));
                                                        return;
                                                    }
                                                    if target_db.exists() {
                                                        let _ = tx.send(Err(format!("{} already exists — delete or pick a different dir", target_db.display())));
                                                        return;
                                                    }
                                                    // VACUUM INTO is the SQLite-blessed way to snapshot a live DB.
                                                    let dest = target_db.display().to_string().replace('\'', "''");
                                                    let sql = format!("VACUUM INTO '{}'", dest);
                                                    match cache.connection() {
                                                        Ok(conn) => match conn.execute(&sql, []) {
                                                            Ok(_) => match write_custom_cache_dir(Some(&target)) {
                                                                Ok(_) => { let _ = tx.send(Ok(format!("Cache copied to {}. Restart terminal to use it.", target_db.display()))); }
                                                                Err(e) => { let _ = tx.send(Err(format!("Copy OK but save-setting failed: {}", e))); }
                                                            },
                                                            Err(e) => { let _ = tx.send(Err(format!("VACUUM INTO failed: {}", e))); }
                                                        },
                                                        Err(e) => { let _ = tx.send(Err(format!("Could not open cache connection: {}", e))); }
                                                    }
                                                })
                                            {
                                                let _ = tx_on_spawn_err.send(Err(format!("Cache copy worker failed to start: {}", e)));
                                            }
                                        }
                                    }

                                    if ui.add_enabled(!in_flight && read_custom_cache_dir().is_some(), egui::Button::new(egui::RichText::new("Reset to default").small()))
                                        .on_hover_text("Clear the override. Next startup uses ~/.config/typhoon-terminal/cache/. Data at the custom location is NOT deleted.")
                                        .clicked()
                                    {
                                        match write_custom_cache_dir(None) {
                                            Ok(_) => {
                                                self.storage_cache_move_result = Some((true, "Reset to default. Restart terminal to apply.".to_string()));
                                                self.log.push_back(LogEntry::info("Cache location reset to default (restart required)"));
                                            }
                                            Err(e) => { self.storage_cache_move_result = Some((false, format!("Reset failed: {}", e))); }
                                        }
                                    }
                                });

                                if in_flight {
                                    ui.label(egui::RichText::new("Copy in progress... VACUUM INTO is running in background.").small().color(AXIS_TEXT));
                                }
                                if let Some((ok, msg)) = &self.storage_cache_move_result {
                                    let color = if *ok { egui::Color32::from_rgb(26, 188, 156) } else { egui::Color32::from_rgb(231, 76, 60) };
                                    ui.colored_label(color, egui::RichText::new(msg).small());
                                }
                            }
                            ui.separator();

                            self.render_storage_table(ui);
                        });
            self.show_storage = show_storage;
            if storage_save_after {
                self.save_session();
            }
        }

        // Sync Status — per-(broker,TF) bar-sync health table, computed
        // from the BG bar_ts_cache on render (cheap: a few thousand keys
        // bucketed into ≤45 rows). Universe is every (symbol, TF) pair
        // the cache has ever seen for Alpaca / Kraken; the trader-facing
        // brokers always get a row even when their cache slice is empty,
        // so "0% Kraken" is visible before the first bar sync lands.
        self.render_sync_status_window(ctx);
    }
}
