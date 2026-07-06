use super::*;

impl TyphooNApp {
    pub(super) fn render_storage_sync_windows(&mut self, ctx: &egui::Context) {
        self.render_cache_stats_window(ctx);

        // Storage-sanity worker pump (audit / repair / merged rebuild / export
        // — one job at a time). Take the receiver so the drain loop can mutate
        // self; hand it back if the job is still running.
        if let Some(rx) = self.storage_sanity_rx.take() {
            let mut finished = false;
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    SanityWorkerMsg::Progress { phase, done, total } => {
                        self.storage_sanity_progress = Some((phase, done, total));
                    }
                    SanityWorkerMsg::AuditDone { result, delta } => {
                        finished = true;
                        match result {
                            Ok(report) => {
                                let summary = report.summary_line();
                                let warn = report.error_count > 0 || report.warn_count > 0;
                                self.storage_sanity_delta = delta;
                                // Recompute the "worth backfilling" symbol set
                                // once per audit (one DB read here, off the
                                // render path) so the button/count and worker
                                // read it without per-frame queries.
                                self.storage_sanity_backfill_symbols = self
                                    .cache
                                    .as_ref()
                                    .map(|cache| {
                                        let fetched = already_fetched_split_symbols(cache);
                                        sanity_split_backfill_symbols(&report.issues, &fetched)
                                    })
                                    .unwrap_or_default();
                                self.storage_sanity_report = Some(report);
                                if warn {
                                    self.log.push_back(LogEntry::warn(summary));
                                } else {
                                    self.log.push_back(LogEntry::info(summary));
                                }
                                if let Some(delta) = self.storage_sanity_delta.clone() {
                                    self.log.push_back(LogEntry::info(delta));
                                }
                            }
                            Err(e) => {
                                self.log.push_back(LogEntry::err(format!(
                                    "Data sanity audit failed: {e}"
                                )));
                            }
                        }
                    }
                    SanityWorkerMsg::RepairDone(result) => {
                        finished = true;
                        match result {
                            Ok(outcome) => {
                                for err in outcome.errors.iter().take(5) {
                                    self.log.push_back(LogEntry::warn(format!("repair: {err}")));
                                }
                                let line = outcome.summary_line();
                                if outcome.errors.is_empty() {
                                    self.log.push_back(LogEntry::info(line.clone()));
                                } else {
                                    self.log.push_back(LogEntry::warn(line.clone()));
                                }
                                self.storage_sanity_last_action = Some(line);
                            }
                            Err(e) => {
                                self.storage_sanity_reaudit_after = false;
                                self.log
                                    .push_back(LogEntry::err(format!("Cache repair failed: {e}")));
                            }
                        }
                    }
                    SanityWorkerMsg::MergedRebuildDone(result) => {
                        finished = true;
                        match result {
                            Ok(deleted) => {
                                let line = format!(
                                    "Deleted {deleted} stale merged rows — they re-materialize from raw sources on next chart load/sync"
                                );
                                self.log.push_back(LogEntry::info(line.clone()));
                                self.storage_sanity_last_action = Some(line);
                            }
                            Err(e) => {
                                self.storage_sanity_reaudit_after = false;
                                self.log.push_back(LogEntry::err(format!(
                                    "Merged-row rebuild failed: {e}"
                                )));
                            }
                        }
                    }
                    SanityWorkerMsg::ExportDone(result) => {
                        finished = true;
                        match result {
                            Ok(path) => {
                                let line = format!("Sanity report exported to {path}");
                                self.log.push_back(LogEntry::info(line.clone()));
                                self.storage_sanity_last_action = Some(line);
                            }
                            Err(e) => {
                                self.log.push_back(LogEntry::err(format!(
                                    "Sanity report export failed: {e}"
                                )));
                            }
                        }
                    }
                    SanityWorkerMsg::SplitsBackfillDone(result) => {
                        finished = true;
                        match result {
                            Ok(line) => {
                                self.log.push_back(LogEntry::info(line.clone()));
                                self.storage_sanity_last_action = Some(line);
                            }
                            Err(e) => {
                                self.storage_sanity_reaudit_after = false;
                                self.log.push_back(LogEntry::err(format!(
                                    "Splits backfill failed: {e}"
                                )));
                            }
                        }
                    }
                }
            }
            if finished {
                self.storage_sanity_cancel = None;
                self.storage_sanity_progress = None;
                if std::mem::take(&mut self.storage_sanity_reaudit_after) {
                    self.start_sanity_audit();
                }
            } else {
                self.storage_sanity_rx = Some(rx);
                // Keep frames coming while a worker runs so progress and
                // completion land without waiting for user input.
                ctx.request_repaint_after(std::time::Duration::from_millis(250));
            }
        }

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
                                            "Compression level for all foreground bar-cache writes, including Kraken WS. Lower = faster sync/import writes; higher = smaller disk. Compact only catches old rows/stragglers below zstd-22.",
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
                                ui.horizontal(|ui| {
                                    let job_running = self.storage_sanity_rx.is_some();
                                    if ui
                                        .add_enabled(
                                            self.cache.is_some() && !job_running,
                                            egui::Button::new(
                                                egui::RichText::new("Run data sanity audit").small(),
                                            ),
                                        )
                                        .on_hover_text(
                                            "Read-only full bar-cache audit: decompresses rows, validates metadata/OHLC/timestamps/gaps, and checks recent cross-source overlap mismatches. Runs off the UI thread.",
                                        )
                                        .clicked()
                                    {
                                        self.start_sanity_audit();
                                    }
                                    if let Some((phase, done, total)) = self.storage_sanity_progress {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{phase} running… {done}/{total} rows"
                                            ))
                                            .color(AXIS_TEXT)
                                            .small(),
                                        );
                                        if let Some(cancel) = self.storage_sanity_cancel.as_ref() {
                                            if ui.small_button("Cancel").clicked() {
                                                cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                                            }
                                        }
                                    } else if job_running {
                                        ui.label(
                                            egui::RichText::new("job running…")
                                                .color(AXIS_TEXT)
                                                .small(),
                                        );
                                    }
                                });
                                self.render_sanity_report_panel(ui);
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
                                                dt.with_timezone(&chrono::Utc)
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
                                    let maintenance_running = self.storage_maintenance_rx.is_some();
                                    let reclaim_available = self.cache.is_some()
                                        && !maintenance_running
                                        && !self.heavy_sync_in_progress
                                        && !self.auto_compact_in_progress;
                                    if ui
                                        .add_enabled(
                                            reclaim_available,
                                            egui::Button::new(
                                                egui::RichText::new("Reclaim Free Space").small(),
                                            ),
                                        )
                                        .on_disabled_hover_text(if maintenance_running {
                                            "Storage maintenance is already running."
                                        } else if self.heavy_sync_in_progress {
                                            "Broad market-data sync is active; wait until catch-up settles so VACUUM does not starve bar writes or OOM."
                                        } else if self.auto_compact_in_progress {
                                            "Compaction is already running."
                                        } else {
                                            "Cache is not available."
                                        })
                                        .clicked()
                                    {
                                        if let Some(cache) = self.cache.clone() {
                                            let (tx, rx) = std::sync::mpsc::channel();
                                            self.storage_maintenance_rx = Some(rx);
                                            self.storage_cache_move_result = Some((
                                                true,
                                                "Reclaiming SQLite free pages in background... this can take several minutes for large caches".to_string(),
                                            ));
                                            let tx_on_spawn_err = tx.clone();
                                            if let Err(e) = std::thread::Builder::new()
                                                .name("typhoon-cache-reclaim".into())
                                                .spawn(move || {
                                                    let result = cache.reclaim_space();
                                                    let _ = tx.send(StorageMaintenanceMsg::ReclaimDone(result));
                                                })
                                            {
                                                let _ = tx_on_spawn_err.send(
                                                    StorageMaintenanceMsg::ReclaimDone(Err(format!(
                                                        "Reclaim worker failed to start: {}",
                                                        e
                                                    ))),
                                                );
                                            }
                                        }
                                    }
                                    ui.label(
                                        egui::RichText::new(
                                            "Run WAL checkpoint + VACUUM after prior deletes; disabled during heavy sync/compaction and runs off the UI thread.",
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
                            // Drain any in-flight Storage Manager maintenance result from the worker thread.
                            if let Some(rx) = self.storage_maintenance_rx.take() {
                                match rx.try_recv() {
                                    Ok(StorageMaintenanceMsg::ReclaimDone(result)) => {
                                        match result {
                                            Ok((before, after)) => {
                                                let line = format!(
                                                    "Reclaimed SQLite free pages: {} -> {}",
                                                    format_bytes_human(before),
                                                    format_bytes_human(after)
                                                );
                                                self.storage_cache_move_result = Some((true, line.clone()));
                                                self.log.push_back(LogEntry::info(line));
                                            }
                                            Err(e) => {
                                                let line = format!("Reclaim storage failed: {}", e);
                                                self.storage_cache_move_result = Some((false, line.clone()));
                                                self.log.push_back(LogEntry::err(line));
                                            }
                                        }
                                        self.refresh_storage_snapshot_after_action("reclaim");
                                    }
                                    Ok(StorageMaintenanceMsg::CacheMoveDone(msg)) => {
                                        match msg {
                                            Ok(s) => {
                                                self.storage_cache_move_result = Some((true, s.clone()));
                                                self.log.push_back(LogEntry::info(s));
                                            }
                                            Err(e) => {
                                                self.storage_cache_move_result = Some((false, e.clone()));
                                                self.log.push_back(LogEntry::err(e));
                                            }
                                        }
                                    }
                                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                                        self.storage_maintenance_rx = Some(rx);
                                    }
                                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                        self.storage_cache_move_result = Some((
                                            false,
                                            "Storage maintenance worker disconnected".to_string(),
                                        ));
                                    }
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

                                let in_flight = self.storage_maintenance_rx.is_some();
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
                                        self.storage_maintenance_rx = Some(rx);
                                        self.storage_cache_move_result = Some((true, format!("Copying cache to {} ... this may take several minutes for large caches", target.display())));
                                        if let Some(cache) = self.cache.clone() {
                                            let tx_on_spawn_err = tx.clone();
                                            if let Err(e) = std::thread::Builder::new()
                                                .name("typhoon-cache-vacuum-copy".into())
                                                .spawn(move || {
                                                    if let Err(e) = std::fs::create_dir_all(&target) {
                                                        let _ = tx.send(StorageMaintenanceMsg::CacheMoveDone(Err(format!("mkdir {} failed: {}", target.display(), e))));
                                                        return;
                                                    }
                                                    if target_db.exists() {
                                                        let _ = tx.send(StorageMaintenanceMsg::CacheMoveDone(Err(format!("{} already exists — delete or pick a different dir", target_db.display()))));
                                                        return;
                                                    }
                                                    // VACUUM INTO is the SQLite-blessed way to snapshot a live DB.
                                                    let dest = target_db.display().to_string().replace('\'', "''");
                                                    let sql = format!("VACUUM INTO '{}'", dest);
                                                    let result = match cache.connection() {
                                                        Ok(conn) => match conn.execute(&sql, []) {
                                                            Ok(_) => match write_custom_cache_dir(Some(&target)) {
                                                                Ok(_) => Ok(format!("Cache copied to {}. Restart terminal to use it.", target_db.display())),
                                                                Err(e) => Err(format!("Copy OK but save-setting failed: {}", e)),
                                                            },
                                                            Err(e) => Err(format!("VACUUM INTO failed: {}", e)),
                                                        },
                                                        Err(e) => Err(format!("Could not open cache connection: {}", e)),
                                                    };
                                                    let _ = tx.send(StorageMaintenanceMsg::CacheMoveDone(result));
                                                })
                                            {
                                                let _ = tx_on_spawn_err.send(StorageMaintenanceMsg::CacheMoveDone(Err(format!("Cache copy worker failed to start: {}", e))));
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

    /// Report body under the audit button: summary, run-over-run delta, per-code
    /// counts, repair action row, and a filterable severity-sorted issue browser.
    fn render_sanity_report_panel(&mut self, ui: &mut egui::Ui) {
        use typhoon_engine::core::cache::{
            BarCacheRepairOptions, BarCacheSanitySeverity, issue_display_line,
        };
        // Copy everything the panel displays out of the report first so the
        // click handlers below can borrow self mutably.
        let filter_lc = self.storage_sanity_filter.to_lowercase();
        let Some(report) = self.storage_sanity_report.as_ref() else {
            return;
        };
        let summary = report.summary_line();
        let summary_color = if report.error_count > 0 {
            egui::Color32::from_rgb(231, 76, 60)
        } else if report.warn_count > 0 {
            egui::Color32::from_rgb(241, 196, 15)
        } else {
            egui::Color32::from_rgb(26, 188, 156)
        };
        let code_lines = report.top_code_lines(6);
        let meta_n = report.metadata_repairable_rows;
        let rewrite_n = report.rewritable_rows;
        let corrupt_n = report.corrupt_rows;
        let merged_n = report.merged_mismatch_keys.len();
        let purge_n = report.purgeable_empty_rows;
        let backfill_n = self.storage_sanity_backfill_symbols.len();
        let stored_issues = report.issues.len();
        let issue_lines: Vec<(BarCacheSanitySeverity, String)> = report
            .issues
            .iter()
            .filter(|issue| {
                filter_lc.is_empty()
                    || issue.key.to_lowercase().contains(&filter_lc)
                    || issue.code.contains(&filter_lc)
                    || issue.detail.to_lowercase().contains(&filter_lc)
            })
            .take(300)
            .map(|issue| (issue.severity, issue_display_line(issue)))
            .collect();

        ui.label(
            egui::RichText::new(summary)
                .color(summary_color)
                .small()
                .monospace(),
        );
        if let Some(delta) = self.storage_sanity_delta.as_ref() {
            ui.label(
                egui::RichText::new(delta)
                    .color(AXIS_TEXT)
                    .small()
                    .monospace(),
            );
        }
        if let Some(action) = self.storage_sanity_last_action.as_ref() {
            ui.label(
                egui::RichText::new(action)
                    .color(AXIS_TEXT)
                    .small()
                    .monospace(),
            );
        }
        for line in code_lines {
            ui.label(
                egui::RichText::new(format!("count {line}"))
                    .color(AXIS_TEXT)
                    .small()
                    .monospace(),
            );
        }

        // Repair actions — each scoped to what the audit proved fixable, all
        // re-verified by an automatic follow-up audit.
        let job_running = self.storage_sanity_rx.is_some();
        let actions_enabled = self.cache.is_some() && !job_running;
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    actions_enabled && meta_n > 0,
                    egui::Button::new(
                        egui::RichText::new(format!("Fix metadata ({meta_n})")).small(),
                    ),
                )
                .on_hover_text(
                    "Recompute bar_count/last_ts/second_last_ts metadata from blob contents and clamp bad zstd tags. Safe: bar data untouched; re-audits when done.",
                )
                .clicked()
            {
                self.start_sanity_repair(
                    BarCacheRepairOptions {
                        fix_metadata: true,
                        ..Default::default()
                    },
                    "fix metadata",
                );
            }
            if ui
                .add_enabled(
                    actions_enabled && rewrite_n > 0,
                    egui::Button::new(
                        egui::RichText::new(format!("Rewrite bad rows ({rewrite_n})")).small(),
                    ),
                )
                .on_hover_text(
                    "Re-pack rows that violate write-path invariants: drops invalid-OHLC bars, duplicate/out-of-order buckets, bars >2 days in the future; clamps settled candles whose close lies outside high/low; converts legacy JSON rows to binary. Re-audits when done.",
                )
                .clicked()
            {
                self.start_sanity_repair(
                    BarCacheRepairOptions {
                        rewrite_bad_rows: true,
                        ..Default::default()
                    },
                    "rewrite bad rows",
                );
            }
            let delete_armed =
                self.storage_sanity_confirm == Some(SanityConfirmAction::DeleteCorrupt);
            let delete_text = if delete_armed {
                format!("Confirm delete {corrupt_n} corrupt rows")
            } else {
                format!("Delete corrupt rows ({corrupt_n})")
            };
            let mut delete_rich = egui::RichText::new(delete_text).small();
            if delete_armed {
                delete_rich = delete_rich.color(egui::Color32::from_rgb(231, 76, 60));
            }
            if ui
                .add_enabled(actions_enabled && corrupt_n > 0, egui::Button::new(delete_rich))
                .on_hover_text(
                    "Delete rows that cannot be decoded at all (undecompressable/bad header/truncated). Destructive: the next bar sync re-fetches them. Click twice.",
                )
                .clicked()
            {
                if delete_armed {
                    self.storage_sanity_confirm = None;
                    self.start_sanity_repair(
                        BarCacheRepairOptions {
                            delete_corrupt_rows: true,
                            ..Default::default()
                        },
                        "delete corrupt rows",
                    );
                } else {
                    self.storage_sanity_confirm = Some(SanityConfirmAction::DeleteCorrupt);
                }
            }
            if ui
                .add_enabled(
                    actions_enabled && purge_n > 0,
                    egui::Button::new(
                        egui::RichText::new(format!("Purge empty rows ({purge_n})")).small(),
                    ),
                )
                .on_hover_text(
                    "Delete yahoo-chart rows that decode to zero bars — an empty row occupies the key while holding nothing, so sync never re-fetches the range. Nothing is lost; the next bar sync re-fetches. Re-audits when done.",
                )
                .clicked()
            {
                self.start_sanity_repair(
                    BarCacheRepairOptions {
                        purge_empty_rows: true,
                        ..Default::default()
                    },
                    "purge empty rows",
                );
            }
            let merged_armed =
                self.storage_sanity_confirm == Some(SanityConfirmAction::RebuildMerged);
            let merged_text = if merged_armed {
                format!("Confirm rebuild {merged_n} merged rows")
            } else {
                format!("Rebuild merged ({merged_n})")
            };
            let mut merged_rich = egui::RichText::new(merged_text).small();
            if merged_armed {
                merged_rich = merged_rich.color(egui::Color32::from_rgb(231, 76, 60));
            }
            if ui
                .add_enabled(actions_enabled && merged_n > 0, egui::Button::new(merged_rich))
                .on_hover_text(
                    "Delete merged rows whose recent bars disagree with their raw sources; they re-materialize from raw sources (with current merge logic) on next chart load/sync. Click twice.",
                )
                .clicked()
            {
                if merged_armed {
                    self.storage_sanity_confirm = None;
                    self.start_sanity_merged_rebuild();
                } else {
                    self.storage_sanity_confirm = Some(SanityConfirmAction::RebuildMerged);
                }
            }
            if ui
                .add_enabled(
                    actions_enabled && backfill_n > 0,
                    egui::Button::new(
                        egui::RichText::new(format!("Backfill splits ({backfill_n})")).small(),
                    ),
                )
                .on_hover_text(
                    "Fetch split history (keyless Yahoo chart events + FMP when a key is set) into research_stock_splits for every symbol the cross-source checks flagged. Feeds the merge's known-split back-adjust (ADR-122) and lets the audit reclassify split-explained mismatches as context. Re-audits when done.",
                )
                .clicked()
            {
                self.start_sanity_splits_backfill();
            }
            if ui
                .add_enabled(
                    actions_enabled,
                    egui::Button::new(egui::RichText::new("Export JSON").small()),
                )
                .on_hover_text(
                    "Write the full report (all stored issues + repair counters) to a JSON file next to the cache DB for offline analysis.",
                )
                .clicked()
            {
                self.start_sanity_export();
            }
        });

        // Issue browser: severity-sorted (errors first), substring filter on
        // key/code/detail.
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Filter").color(AXIS_TEXT).small());
            ui.add(
                egui::TextEdit::singleline(&mut self.storage_sanity_filter)
                    .desired_width(160.0)
                    .hint_text("symbol / code / text"),
            );
            ui.label(
                egui::RichText::new(format!(
                    "{} of {stored_issues} stored issues",
                    issue_lines.len()
                ))
                .color(AXIS_TEXT)
                .small(),
            );
        });
        egui::ScrollArea::vertical()
            .id_salt("sanity_issue_browser")
            .max_height(180.0)
            .show(ui, |ui| {
                for (severity, line) in &issue_lines {
                    let color = match severity {
                        BarCacheSanitySeverity::Error => egui::Color32::from_rgb(231, 76, 60),
                        BarCacheSanitySeverity::Warn => egui::Color32::from_rgb(241, 196, 15),
                        BarCacheSanitySeverity::Info => AXIS_TEXT,
                    };
                    ui.label(egui::RichText::new(line).color(color).small().monospace());
                }
            });
    }

    /// Kick off the background data-sanity audit; results land through
    /// `storage_sanity_rx`. History is persisted (and the run-over-run delta
    /// computed) on the worker thread so the render thread never writes.
    pub(crate) fn start_sanity_audit(&mut self) {
        if self.storage_sanity_rx.is_some() {
            return;
        }
        let Some(cache) = self.cache.clone() else {
            return;
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancel_worker = cancel.clone();
        std::thread::spawn(move || {
            let progress_tx = tx.clone();
            let progress = move |done: usize, total: usize| {
                let _ = progress_tx.send(SanityWorkerMsg::Progress {
                    phase: "audit",
                    done,
                    total,
                });
            };
            let result = cache.audit_bar_cache_sanity_with(Some(&progress), Some(&cancel_worker));
            let delta = match &result {
                Ok(report) if !report.cancelled => {
                    let prev = cache.load_bar_sanity_history().pop();
                    let delta = prev.and_then(|p| report.delta_line(&p));
                    if let Err(e) = cache.record_bar_sanity_history(report) {
                        tracing::warn!("sanity history persist failed: {e}");
                    }
                    delta
                }
                _ => None,
            };
            let _ = tx.send(SanityWorkerMsg::AuditDone { result, delta });
        });
        self.storage_sanity_rx = Some(rx);
        self.storage_sanity_cancel = Some(cancel);
        self.storage_sanity_progress = Some(("audit", 0, 0));
        self.storage_sanity_confirm = None;
        self.log.push_back(LogEntry::info(
            "Data sanity audit started (read-only full bar-cache scan)",
        ));
    }

    /// Run one repair class in the background, then re-audit to verify.
    fn start_sanity_repair(
        &mut self,
        opts: typhoon_engine::core::cache::BarCacheRepairOptions,
        label: &'static str,
    ) {
        if self.storage_sanity_rx.is_some() {
            return;
        }
        let Some(cache) = self.cache.clone() else {
            return;
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancel_worker = cancel.clone();
        std::thread::spawn(move || {
            let progress_tx = tx.clone();
            let progress = move |done: usize, total: usize| {
                let _ = progress_tx.send(SanityWorkerMsg::Progress {
                    phase: "repair",
                    done,
                    total,
                });
            };
            let result = cache.repair_bar_cache(opts, Some(&progress), Some(&cancel_worker));
            let _ = tx.send(SanityWorkerMsg::RepairDone(result));
        });
        self.storage_sanity_rx = Some(rx);
        self.storage_sanity_cancel = Some(cancel);
        self.storage_sanity_progress = Some(("repair", 0, 0));
        self.storage_sanity_confirm = None;
        self.storage_sanity_reaudit_after = true;
        self.log
            .push_back(LogEntry::info(format!("Cache repair started ({label})")));
    }

    /// Delete the merged rows the last audit flagged as disagreeing with
    /// their raw sources. Light delete (no VACUUM) — freed pages are
    /// reclaimed by auto-compact; rows re-materialize from raw sources.
    fn start_sanity_merged_rebuild(&mut self) {
        if self.storage_sanity_rx.is_some() {
            return;
        }
        let Some(cache) = self.cache.clone() else {
            return;
        };
        let keys: Vec<String> = self
            .storage_sanity_report
            .as_ref()
            .map(|r| r.merged_mismatch_keys.iter().cloned().collect())
            .unwrap_or_default();
        if keys.is_empty() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let count = keys.len();
        std::thread::spawn(move || {
            let result = cache.delete_keys_light(&keys);
            let _ = tx.send(SanityWorkerMsg::MergedRebuildDone(result));
        });
        self.storage_sanity_rx = Some(rx);
        self.storage_sanity_confirm = None;
        self.storage_sanity_reaudit_after = true;
        self.log.push_back(LogEntry::info(format!(
            "Rebuilding {count} stale merged rows (delete + re-materialize)"
        )));
    }

    /// Write the full last report to a timestamped JSON file next to the DB.
    fn start_sanity_export(&mut self) {
        if self.storage_sanity_rx.is_some() {
            return;
        }
        let Some(report) = self.storage_sanity_report.clone() else {
            return;
        };
        let dir = cache_db_path()
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let path = dir.join(format!(
            "sanity-report-{}.json",
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        ));
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = serde_json::to_string_pretty(&report)
                .map_err(|e| format!("serialize failed: {e}"))
                .and_then(|json| {
                    std::fs::write(&path, json)
                        .map_err(|e| format!("write {} failed: {e}", path.display()))
                        .map(|_| path.display().to_string())
                });
            let _ = tx.send(SanityWorkerMsg::ExportDone(result));
        });
        self.storage_sanity_rx = Some(rx);
    }

    /// Fetch split history for every symbol the last audit's cross-source
    /// checks flagged, into `research_stock_splits` — keyless Yahoo chart
    /// events always, FMP too when a key is set (same combined fetcher the
    /// research scrape uses). Feeds the merge's known-split back-adjust
    /// (ADR-122) and the audit's split-aware mismatch classification without
    /// waiting for a full-catalog research scrape.
    fn start_sanity_splits_backfill(&mut self) {
        if self.storage_sanity_rx.is_some() {
            return;
        }
        let Some(cache) = self.cache.clone() else {
            return;
        };
        let symbols: Vec<String> = self.storage_sanity_backfill_symbols.clone();
        if symbols.is_empty() {
            return;
        }
        let fmp_key = self.fmp_key.clone();
        let with_fmp = if fmp_key.is_empty() { "" } else { " + FMP" };
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancel_worker = cancel.clone();
        let total = symbols.len();
        std::thread::spawn(move || {
            let result = (|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| format!("tokio runtime init failed: {e}"))?;
                rt.block_on(async {
                    use typhoon_engine::core::research;
                    let client = reqwest::Client::builder()
                        .user_agent("TyphooN-Terminal/1.0")
                        .timeout(std::time::Duration::from_secs(15))
                        .build()
                        .map_err(|e| format!("http client init failed: {e}"))?;
                    let mut with_splits = 0usize;
                    let mut failed = 0usize;
                    let mut invalidated = 0usize;
                    let mut done = 0usize;
                    for symbol in &symbols {
                        if cancel_worker.load(std::sync::atomic::Ordering::Relaxed) {
                            break;
                        }
                        match research::fetch_stock_splits(&client, symbol, &fmp_key).await {
                            Ok(rows) => match cache.connection() {
                                Ok(conn) => {
                                    let existing = research::get_stock_splits(&conn, symbol)
                                        .ok()
                                        .flatten()
                                        .unwrap_or_default();
                                    if rows.is_empty() && !existing.is_empty() {
                                        // Provider returned nothing but the table
                                        // already knows splits (e.g. curated WOK)
                                        // — a fetch gap must not erase known
                                        // actions.
                                        with_splits += 1;
                                    } else {
                                        match research::upsert_stock_splits(&conn, symbol, &rows)
                                        {
                                            Ok(()) if !rows.is_empty() => {
                                                with_splits += 1;
                                                // A newly-discovered recent
                                                // material split (either
                                                // direction) means the cached
                                                // bars predate the provider's
                                                // restatement — purge so the
                                                // next sync rebuilds cleanly.
                                                if research::stock_splits_need_bar_cache_invalidation(
                                                    &existing, &rows,
                                                ) {
                                                    drop(conn);
                                                    if let Ok(n) = cache
                                                        .delete_equity_bar_cache_for_symbol(symbol)
                                                    {
                                                        if n > 0 {
                                                            invalidated += 1;
                                                        }
                                                    }
                                                }
                                            }
                                            Ok(()) => {}
                                            Err(e) => {
                                                tracing::warn!("splits backfill {symbol}: {e}");
                                                failed += 1;
                                            }
                                        }
                                    }
                                }
                                Err(_) => failed += 1,
                            },
                            Err(e) => {
                                tracing::debug!("splits backfill {symbol}: {e}");
                                failed += 1;
                                // Record the attempt so a symbol whose splits
                                // fetch persistently fails (ATON-class: a
                                // borderline 1.5x mismatch with no reachable
                                // split feed) stops re-listing in the backfill
                                // count every run. Only when the table has no
                                // prior entry, so a transient failure can't
                                // erase known splits; a later scrape re-fetches.
                                if let Ok(conn) = cache.connection()
                                    && research::get_stock_splits(&conn, symbol)
                                        .ok()
                                        .flatten()
                                        .is_none()
                                {
                                    let _ = research::upsert_stock_splits(&conn, symbol, &[]);
                                }
                            }
                        }
                        done += 1;
                        let _ = tx.send(SanityWorkerMsg::Progress {
                            phase: "splits backfill",
                            done,
                            total,
                        });
                        tokio::time::sleep(std::time::Duration::from_millis(450)).await;
                    }
                    let cancelled = if done < total { " [CANCELLED]" } else { "" };
                    Ok(format!(
                        "Splits backfill: {done}/{total} symbols checked — {with_splits} with split history, {invalidated} cache-reset (new split), {failed} failed{cancelled}"
                    ))
                })
            })();
            let _ = tx.send(SanityWorkerMsg::SplitsBackfillDone(result));
        });
        self.storage_sanity_rx = Some(rx);
        self.storage_sanity_cancel = Some(cancel);
        self.storage_sanity_progress = Some(("splits backfill", 0, total));
        self.storage_sanity_confirm = None;
        self.storage_sanity_reaudit_after = true;
        self.log.push_back(LogEntry::info(format!(
            "Stock-split backfill started ({total} audit-flagged symbols; keyless Yahoo{with_fmp})"
        )));
    }
}

/// Unique symbols from the last audit whose cross-source/merged scale checks
/// flagged a disagreement — the set worth fetching split history for.
/// Split-explained mismatches are excluded: their splits are already known.
/// Symbols from the latest audit worth a split-history fetch: only the
/// **overlap-mismatch Warns** (the sole cross-source code `split_explained_scale`
/// can reclassify), and only those NOT in `already_fetched`. The Info-only scale
/// deltas were removed because backfilling splits never moves them, and
/// already-fetched symbols are excluded because re-fetching the same (missing or
/// non-reconciling) split history can't change the result — those are genuine
/// upstream residue, not backfill work. This makes the count reflect remaining
/// work and drop after a backfill instead of restating every flagged symbol.
fn sanity_split_backfill_symbols(
    issues: &[typhoon_engine::core::cache::BarCacheSanityIssue],
    already_fetched: &std::collections::HashSet<String>,
) -> Vec<String> {
    const CODES: &[&str] = &[
        "cross_source_overlap_mismatch",
        "merged_source_overlap_mismatch",
    ];
    let mut symbols = std::collections::BTreeSet::new();
    for issue in issues {
        if !CODES.contains(&issue.code.as_str()) {
            continue;
        }
        // Cross-source issue keys are "SYMBOL:TIMEFRAME".
        if let Some(symbol) = issue.key.split(':').next()
            && !symbol.is_empty()
            && !already_fetched.contains(&symbol.to_ascii_uppercase())
        {
            symbols.insert(symbol.to_string());
        }
    }
    symbols.into_iter().collect()
}

/// Uppercased symbols that already have a `research_stock_splits` row (fetched
/// at least once, empty or not) — re-fetching them can't help. One small query,
/// run off the render path when an audit completes.
fn already_fetched_split_symbols(
    cache: &typhoon_engine::core::cache::SqliteCache,
) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    if let Ok(conn) = cache.connection()
        && let Ok(mut stmt) = conn.prepare("SELECT symbol FROM research_stock_splits")
    {
        if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
            for symbol in rows.flatten() {
                out.insert(symbol.trim().to_ascii_uppercase());
            }
        }
    }
    out
}

#[cfg(test)]
mod split_backfill_set_tests {
    use super::sanity_split_backfill_symbols;
    use typhoon_engine::core::cache::{BarCacheSanityIssue, BarCacheSanitySeverity};

    fn issue(code: &str, symbol: &str) -> BarCacheSanityIssue {
        BarCacheSanityIssue {
            severity: BarCacheSanitySeverity::Warn,
            code: code.to_string(),
            key: format!("{symbol}:1Month"),
            detail: String::new(),
            occurrences: 1,
        }
    }

    #[test]
    fn backfill_set_is_unfetched_overlap_mismatch_only() {
        let issues = vec![
            issue("cross_source_overlap_mismatch", "ATON"), // unfetched Warn -> included
            issue("cross_source_overlap_mismatch", "ABTS"), // fetched already -> excluded
            issue("cross_source_historical_scale_delta", "FOO"), // Info code, not backfillable
            issue("cross_source_scale_blowout", "BAR"),     // runaway adjust, not split-explainable
        ];
        let fetched = ["ABTS".to_string()].into_iter().collect();
        assert_eq!(
            sanity_split_backfill_symbols(&issues, &fetched),
            vec!["ATON".to_string()]
        );
    }

    #[test]
    fn backfill_set_empty_when_all_overlap_mismatch_already_fetched() {
        let issues = vec![issue("cross_source_overlap_mismatch", "ABTS")];
        let fetched = ["ABTS".to_string()].into_iter().collect();
        assert!(sanity_split_backfill_symbols(&issues, &fetched).is_empty());
    }
}
