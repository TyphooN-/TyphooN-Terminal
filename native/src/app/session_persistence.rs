use super::*;

fn persisted_bar_zstd_level(value: &serde_json::Value, current: i32) -> i32 {
    value["bar_zstd_level"]
        .as_i64()
        .map(|level| level as i32)
        .unwrap_or(current)
        .clamp(
            typhoon_engine::core::cache::MIN_ZSTD_LEVEL,
            typhoon_engine::core::cache::MAX_ZSTD_LEVEL,
        )
}

fn reordered_right_panel_sections(
    order: &[RightPanelSectionId],
    dragged: RightPanelSectionId,
    target: RightPanelSectionId,
    after_target: bool,
) -> Option<Vec<RightPanelSectionId>> {
    if dragged == target {
        return None;
    }
    let mut next = order.to_vec();
    let from = next.iter().position(|s| *s == dragged)?;
    let item = next.remove(from);
    let mut to = next.iter().position(|s| *s == target)?;
    if after_target {
        to += 1;
    }
    next.insert(to.min(next.len()), item);
    Some(next)
}

impl TyphooNApp {
    pub(super) fn refill_market_data_sync_slots(&mut self) {
        let pending_cap = if self.full_tilt_sync_enabled() {
            KRAKEN_SPOT_FULL_TILT_QUEUE_WINDOW
                + KRAKEN_EQUITIES_FULL_TILT_QUEUE_WINDOW
                + KRAKEN_FUTURES_FULL_TILT_QUEUE_WINDOW
                + ALPACA_FULL_TILT_QUEUE_WINDOW
                + YAHOO_CHART_FULL_TILT_QUEUE_WINDOW
        } else {
            KRAKEN_SPOT_QUEUE_WINDOW
                + KRAKEN_FUTURES_QUEUE_WINDOW
                + 96 // Kraken Equities native/demand repair lane
                + 64 // Alpaca assist/broad lane
                + YAHOO_CHART_QUEUE_WINDOW
        };
        if self.total_pending_market_data_fetches() > pending_cap {
            return;
        }
        if self.lan_sync_mode == "client" || !self.cache_loaded {
            return;
        }
        let _ = self.schedule_light_market_data_targets();
        if self.kraken_full_bar_sync_enabled
            && self.kraken_scrape_xstocks
            && !self.kraken_equity_universe_symbols.is_empty()
        {
            let _ = self.schedule_kraken_equities_universe();
        }
        if self.kraken_full_bar_sync_enabled && !self.kraken_pairs.is_empty() {
            let _ = self.schedule_kraken_universe_sectors();
        }
        if self.kraken_full_bar_sync_enabled {
            let _ = self.schedule_kraken_futures_universe_sectors();
        }
        if self.broker_connected && self.alpaca_full_bar_sync_enabled {
            self.maybe_request_alpaca_asset_universe();
            let equity_syms = self.alpaca_equity_rotation_symbols();
            let _ = self.schedule_alpaca_pairs(&equity_syms);
        }
    }

    pub(super) fn normalized_right_panel_order(&mut self) -> Vec<RightPanelSectionId> {
        // Track membership in a bitset (one bit per RightPanelSectionId variant) so
        // the per-frame normalization is O(n) instead of O(n²) from chained
        // `out.contains(&section)` scans.
        let mut out = Vec::with_capacity(RightPanelSectionId::DEFAULT_ORDER.len());
        let mut seen: u64 = 0;
        let bit = |s: RightPanelSectionId| 1u64 << (s as u32);
        let default_mask: u64 = RightPanelSectionId::DEFAULT_ORDER
            .iter()
            .copied()
            .fold(0u64, |m, s| m | bit(s));
        for section in self.right_panel_order.iter().copied() {
            let b = bit(section);
            if default_mask & b != 0 && seen & b == 0 {
                out.push(section);
                seen |= b;
            }
        }
        for section in RightPanelSectionId::DEFAULT_ORDER {
            let b = bit(section);
            if seen & b == 0 {
                out.push(section);
                seen |= b;
            }
        }
        if out != self.right_panel_order {
            self.right_panel_order = out.clone();
        }
        out
    }

    pub(super) fn move_right_panel_section(
        &mut self,
        dragged: RightPanelSectionId,
        target: RightPanelSectionId,
        after_target: bool,
    ) {
        let order = self.normalized_right_panel_order();
        let Some(order) = reordered_right_panel_sections(&order, dragged, target, after_target)
        else {
            return;
        };
        if order != self.right_panel_order {
            self.right_panel_order = order;
            self.session_dirty_since
                .get_or_insert(std::time::Instant::now());
        }
    }

    pub(super) fn handle_right_panel_section_drag(
        &mut self,
        ui: &mut egui::Ui,
        section: RightPanelSectionId,
        response: &egui::Response,
    ) {
        let drag_response = response.clone().interact(egui::Sense::click_and_drag());
        if drag_response.hovered() {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grab);
        }
        if drag_response.drag_started() {
            self.dragging_right_panel_section = Some(section);
        }

        let Some(dragged) = self.dragging_right_panel_section else {
            return;
        };
        let pointer_down = ui.input(|i| i.pointer.primary_down());
        if !pointer_down {
            // `Response::hovered()` can be false on the release frame while egui is
            // painting the foreground drag preview / temporary window decoration.
            // Hit-test against the header rect directly so dropping a dragged
            // section on a visible header commits the reorder reliably.
            let released_over_target = ui
                .input(|i| i.pointer.hover_pos().or(i.pointer.interact_pos()))
                .map(|pos| response.rect.contains(pos))
                .unwrap_or(false);
            if dragged != section && released_over_target {
                let after_target = ui
                    .input(|i| i.pointer.hover_pos().or(i.pointer.interact_pos()))
                    .map(|pos| pos.y > response.rect.center().y)
                    .unwrap_or(false);
                self.move_right_panel_section(dragged, section, after_target);
            }
            return;
        }

        if dragged == section {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grabbing);
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                egui::Area::new(egui::Id::new("right_panel_section_drag_preview"))
                    .order(egui::Order::Foreground)
                    .fixed_pos(pos + egui::vec2(14.0, 10.0))
                    .interactable(false)
                    .show(ui.ctx(), |ui| {
                        egui::Frame::NONE
                            .fill(egui::Color32::from_rgba_premultiplied(18, 22, 28, 230))
                            .stroke(egui::Stroke::new(1.0, ACCENT))
                            .corner_radius(egui::CornerRadius::same(4))
                            .inner_margin(egui::Margin::symmetric(8, 4))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(format!("☰ {}", dragged.label()))
                                        .color(ACCENT)
                                        .strong()
                                        .small(),
                                );
                            });
                    });
            }
            ui.painter().rect_stroke(
                response.rect.expand(1.0),
                egui::CornerRadius::same(0),
                egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
                egui::StrokeKind::Outside,
            );
            return;
        }

        if response.hovered() {
            let after_target = ui
                .input(|i| i.pointer.hover_pos())
                .map(|pos| pos.y > response.rect.center().y)
                .unwrap_or(false);
            let drop_y = if after_target {
                response.rect.bottom()
            } else {
                response.rect.top()
            };
            // Drop-target indicator only — don't commit the reorder until release.
            // Calling `move_right_panel_section` every frame while dragging causes
            // the dragged section to shift positions live, which moves the pointer
            // off the target and then back on the next frame, oscillating until the
            // user releases. The release branch above (where `!pointer_down`)
            // already does the actual reorder once.
            ui.painter().line_segment(
                [
                    egui::pos2(response.rect.left(), drop_y),
                    egui::pos2(response.rect.right(), drop_y),
                ],
                egui::Stroke::new(2.0, ACCENT),
            );
        }
    }

    pub(super) fn render_sync_timeframe_controls(
        &mut self,
        ui: &mut egui::Ui,
        save_after: &mut bool,
    ) {
        ui.label(
            egui::RichText::new("Enabled Sync TFs")
                .color(AXIS_TEXT)
                .small()
                .strong(),
        );
        ui.horizontal_wrapped(|ui| {
            let mut changed = false;
            for (short, cache) in STANDARD_SYNC_TIMEFRAMES {
                let mut enabled = self.enabled_sync_timeframes.contains(cache);
                if ui
                    .checkbox(&mut enabled, egui::RichText::new(short).small())
                    .on_hover_text(format!("{} automated scrape/sync", cache))
                    .changed()
                {
                    if enabled {
                        self.enabled_sync_timeframes.insert(cache.to_string());
                    } else {
                        self.enabled_sync_timeframes.remove(cache);
                    }
                    changed = true;
                }
            }
            if changed {
                *save_after = true;
            }
        });
        ui.label(
            egui::RichText::new(
                "Unchecked TFs are skipped by automated bar sync/backfill across brokers.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
    }

    pub(super) fn render_alpaca_sync_profile_controls(
        &mut self,
        ui: &mut egui::Ui,
        save_after: &mut bool,
        id_salt: &str,
    ) {
        let mut selected_hint = self.alpaca_historical_rpm_hint;
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Alpaca Sync Tier")
                    .color(AXIS_TEXT)
                    .small()
                    .strong(),
            );
            egui::ComboBox::from_id_salt(format!("alpaca_sync_tier_{id_salt}"))
                .selected_text(alpaca_historical_rpm_hint_label(selected_hint))
                .show_ui(ui, |ui| {
                    for (label, rpm) in ALPACA_HISTORICAL_RPM_PRESETS {
                        ui.selectable_value(&mut selected_hint, rpm, label);
                    }
                });
        });
        if selected_hint != self.alpaca_historical_rpm_hint {
            self.alpaca_historical_rpm_hint = selected_hint;
            self.alpaca_historical_rpm_observed = 0;
            self.push_alpaca_sync_runtime_config();
            *save_after = true;
        }

        let capacity = self.alpaca_sync_capacity();
        ui.label(
            egui::RichText::new(format!(
                "Alpaca sync budget: {} req/min · {} workers · queue {} · batch {}",
                self.alpaca_effective_historical_rpm(),
                capacity.fetch_permits,
                capacity.queue_window,
                capacity.batch_size
            ))
            .color(AXIS_TEXT)
            .small(),
        );
        let observed = self.alpaca_historical_rpm_observed;
        let hint = self.alpaca_historical_rpm_hint;
        if observed > 0 {
            let note = if hint == 0 {
                format!("Live headers detected {} req/min from Alpaca.", observed)
            } else if hint != observed {
                format!(
                    "Live headers overrode the startup hint: {} req/min observed.",
                    observed
                )
            } else {
                format!("Live headers confirmed {} req/min from Alpaca.", observed)
            };
            ui.label(egui::RichText::new(note).color(AXIS_TEXT).small());
        } else if hint == 0 {
            ui.label(
                egui::RichText::new(
                    "Auto starts at Basic cadence and upgrades after the first Alpaca response reveals the real tier.",
                )
                .color(AXIS_TEXT)
                .small(),
            );
        }
    }

    /// Build session JSON string (pure data, no I/O — safe to call from UI thread).
    pub(super) fn build_session_json(&self) -> String {
        let session = self.build_session_value();
        serde_json::to_string_pretty(&session).unwrap_or_default()
    }

    pub(super) fn build_sync_preferences_value(&self) -> serde_json::Value {
        serde_json::json!({
            "kraken_scrape_schema": 3,
            "alpaca_enabled": self.alpaca_enabled,
            "alpaca_full_bar_sync_enabled": self.alpaca_full_bar_sync_enabled,
            "kraken_enabled": self.kraken_enabled,
            "kraken_full_bar_sync_enabled": self.kraken_full_bar_sync_enabled,
            "kraken_scrape_xstocks": self.kraken_scrape_xstocks,
            "kraken_scrape_usd_crypto": self.kraken_scrape_usd_crypto,
            "kraken_scrape_fiat_crypto": self.kraken_scrape_fiat_crypto,
            "kraken_scrape_crypto_crosses": self.kraken_scrape_crypto_crosses,
            "kraken_scrape_futures": self.kraken_scrape_futures,
            "backfill_alpaca_kraken_equities_enabled": self.backfill_alpaca_kraken_equities_enabled,
            "backfill_yahoo_chart_enabled": self.backfill_yahoo_chart_enabled,
            "kraken_ws_ohlc_enabled": self.kraken_ws_ohlc_enabled,
            "crypto_fiat_quote_usd": self.crypto_fiat_quote_usd,
            "crypto_fiat_quote_usdt": self.crypto_fiat_quote_usdt,
            "crypto_fiat_quote_usdc": self.crypto_fiat_quote_usdc,
            "crypto_fiat_quote_usdg": self.crypto_fiat_quote_usdg,
            "crypto_fiat_quote_eur": self.crypto_fiat_quote_eur,
            "crypto_fiat_quote_gbp": self.crypto_fiat_quote_gbp,
            "crypto_fiat_quote_cad": self.crypto_fiat_quote_cad,
            "crypto_fiat_quote_aud": self.crypto_fiat_quote_aud,
            "crypto_fiat_quote_jpy": self.crypto_fiat_quote_jpy,
            "crypto_fiat_quote_chf": self.crypto_fiat_quote_chf,
            "fund_source_alpaca": self.fund_source_alpaca,
            "fund_source_kraken": self.fund_source_kraken,
            "enabled_sync_timeframes": STANDARD_SYNC_TIMEFRAMES.iter()
                .filter_map(|(_, tf)| self.enabled_sync_timeframes.contains(*tf).then(|| serde_json::json!(tf)))
                .collect::<Vec<_>>(),
            "alpaca_historical_rpm_hint": self.alpaca_historical_rpm_hint,
            "bar_zstd_level": self.bar_zstd_level,
            "auto_compact_enabled": self.auto_compact_enabled,
            "auto_compact_last_run_ms": self.auto_compact_last_run_ms,
            "auto_compact_cadence_days": self.auto_compact_schedule.cadence_days,
            "auto_compact_window_weekday": self.auto_compact_schedule.window_weekday,
            "auto_compact_window_hour_start": self.auto_compact_schedule.window_hour_start,
            "auto_compact_window_hour_end": self.auto_compact_schedule.window_hour_end,
            "auto_compact_uncompacted_threshold": self.auto_compact_schedule.uncompacted_threshold,
        })
    }

    pub(super) fn apply_sync_preferences_value(&mut self, value: &serde_json::Value) {
        let kraken_scrape_schema = value["kraken_scrape_schema"].as_u64().unwrap_or(1);
        if let Some(enabled) = value["alpaca_enabled"].as_bool() {
            self.alpaca_enabled = enabled;
        }
        if let Some(enabled) = value["alpaca_full_bar_sync_enabled"].as_bool() {
            self.alpaca_full_bar_sync_enabled = enabled;
        }
        if let Some(enabled) = value["kraken_full_bar_sync_enabled"].as_bool() {
            self.kraken_full_bar_sync_enabled = enabled;
        }
        if let Some(enabled) = value["kraken_enabled"].as_bool() {
            self.kraken_enabled = enabled;
        }
        if let Some(enabled) = value["kraken_scrape_xstocks"].as_bool() {
            self.kraken_scrape_xstocks = enabled;
        }
        if let Some(enabled) = value["kraken_scrape_usd_crypto"].as_bool() {
            self.kraken_scrape_usd_crypto = enabled;
        }
        if let Some(enabled) = value["kraken_scrape_fiat_crypto"].as_bool() {
            self.kraken_scrape_fiat_crypto = enabled;
        }
        if let Some(enabled) = value["kraken_scrape_crypto_crosses"].as_bool() {
            self.kraken_scrape_crypto_crosses = enabled;
        }
        if kraken_scrape_schema < 2 {
            self.kraken_scrape_fiat_crypto = true;
            self.kraken_scrape_crypto_crosses = true;
        }
        if kraken_scrape_schema < 3 {
            self.crypto_fiat_quote_usd = self.kraken_scrape_usd_crypto;
            self.crypto_fiat_quote_usdt = self.kraken_scrape_usd_crypto;
            self.crypto_fiat_quote_usdc = self.kraken_scrape_usd_crypto;
            self.crypto_fiat_quote_usdg = self.kraken_scrape_usd_crypto;
            self.crypto_fiat_quote_eur = self.kraken_scrape_fiat_crypto;
            self.crypto_fiat_quote_gbp = self.kraken_scrape_fiat_crypto;
            self.crypto_fiat_quote_cad = self.kraken_scrape_fiat_crypto;
            self.crypto_fiat_quote_aud = self.kraken_scrape_fiat_crypto;
            self.crypto_fiat_quote_jpy = self.kraken_scrape_fiat_crypto;
            self.crypto_fiat_quote_chf = self.kraken_scrape_fiat_crypto;
        } else {
            if let Some(enabled) = value["crypto_fiat_quote_usd"].as_bool() {
                self.crypto_fiat_quote_usd = enabled;
            }
            if let Some(enabled) = value["crypto_fiat_quote_usdt"].as_bool() {
                self.crypto_fiat_quote_usdt = enabled;
            }
            if let Some(enabled) = value["crypto_fiat_quote_usdc"].as_bool() {
                self.crypto_fiat_quote_usdc = enabled;
            }
            if let Some(enabled) = value["crypto_fiat_quote_usdg"].as_bool() {
                self.crypto_fiat_quote_usdg = enabled;
            }
            if let Some(enabled) = value["crypto_fiat_quote_eur"].as_bool() {
                self.crypto_fiat_quote_eur = enabled;
            }
            if let Some(enabled) = value["crypto_fiat_quote_gbp"].as_bool() {
                self.crypto_fiat_quote_gbp = enabled;
            }
            if let Some(enabled) = value["crypto_fiat_quote_cad"].as_bool() {
                self.crypto_fiat_quote_cad = enabled;
            }
            if let Some(enabled) = value["crypto_fiat_quote_aud"].as_bool() {
                self.crypto_fiat_quote_aud = enabled;
            }
            if let Some(enabled) = value["crypto_fiat_quote_jpy"].as_bool() {
                self.crypto_fiat_quote_jpy = enabled;
            }
            if let Some(enabled) = value["crypto_fiat_quote_chf"].as_bool() {
                self.crypto_fiat_quote_chf = enabled;
            }
        }
        self.kraken_scrape_usd_crypto = self.crypto_fiat_quote_usd
            || self.crypto_fiat_quote_usdt
            || self.crypto_fiat_quote_usdc
            || self.crypto_fiat_quote_usdg;
        self.kraken_scrape_fiat_crypto = self.crypto_fiat_quote_eur
            || self.crypto_fiat_quote_gbp
            || self.crypto_fiat_quote_cad
            || self.crypto_fiat_quote_aud
            || self.crypto_fiat_quote_jpy
            || self.crypto_fiat_quote_chf;
        if let Some(enabled) = value["kraken_scrape_futures"].as_bool() {
            self.kraken_scrape_futures = enabled;
        }
        if let Some(enabled) = value["backfill_alpaca_kraken_equities_enabled"].as_bool() {
            self.backfill_alpaca_kraken_equities_enabled = enabled;
        }
        if let Some(enabled) = value["backfill_yahoo_chart_enabled"].as_bool() {
            self.backfill_yahoo_chart_enabled = enabled;
        }

        if let Some(enabled) = value["kraken_ws_ohlc_enabled"].as_bool() {
            self.kraken_ws_ohlc_enabled = enabled;
        }
        if let Some(arr) = value["enabled_sync_timeframes"].as_array() {
            self.enabled_sync_timeframes = arr
                .iter()
                .filter_map(|v| v.as_str())
                .filter_map(normalize_sync_timeframe_key)
                .map(str::to_string)
                .collect();
        }
        if let Some(rpm_hint) = value["alpaca_historical_rpm_hint"].as_u64() {
            self.alpaca_historical_rpm_hint = (rpm_hint as u32).min(100_000);
        }
        self.bar_zstd_level = typhoon_engine::core::cache::set_bar_zstd_level(
            persisted_bar_zstd_level(value, self.bar_zstd_level),
        );
        if let Some(b) = value["auto_compact_enabled"].as_bool() {
            self.auto_compact_enabled = b;
        }
        if let Some(ms) = value["auto_compact_last_run_ms"].as_i64() {
            self.auto_compact_last_run_ms = ms;
        }
        let mut schedule = self.auto_compact_schedule;
        if let Some(days) = value["auto_compact_cadence_days"].as_i64() {
            schedule.cadence_days = days;
        }
        if let Some(weekday) = value["auto_compact_window_weekday"].as_u64() {
            schedule.window_weekday = weekday as u32;
        }
        if let Some(hour) = value["auto_compact_window_hour_start"].as_u64() {
            schedule.window_hour_start = hour as u32;
        }
        if let Some(hour) = value["auto_compact_window_hour_end"].as_u64() {
            schedule.window_hour_end = hour as u32;
        }
        if let Some(threshold) = value["auto_compact_uncompacted_threshold"].as_i64() {
            schedule.uncompacted_threshold = threshold;
        }
        self.auto_compact_schedule = schedule.sanitized();
    }

    pub(super) fn sync_preferences_save(&self) {
        if let Some(ref cache) = self.cache {
            let json =
                serde_json::to_string(&self.build_sync_preferences_value()).unwrap_or_default();
            let _ = cache.put_kv("app:sync_preferences", &json);
        }
    }

    pub(super) fn sync_preferences_load(&mut self) {
        if let Some(ref cache) = self.cache {
            if let Ok(Some(json)) = cache.get_kv("app:sync_preferences") {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json) {
                    self.apply_sync_preferences_value(&value);
                }
            }
        }
    }

    /// Auto-compact scheduler tick. Cheap on the steady-state path: returns
    /// immediately if the next-check throttle hasn't elapsed. ADR-089.
    pub(super) fn tick_auto_compact(&mut self) {
        let now = std::time::Instant::now();
        if now < self.auto_compact_next_check_at {
            return;
        }
        // Re-evaluate at most once per minute regardless of outcome.
        self.auto_compact_next_check_at = now + std::time::Duration::from_secs(60);

        let now_ms = chrono::Utc::now().timestamp_millis();
        // Stale-flag guard: if a compact has been "in progress" for longer than
        // any sane run (8h), assume the completion log was lost and reset so the
        // gate can recover on its own.
        if self.auto_compact_in_progress {
            let stale_after_ms: i64 = 8 * 60 * 60 * 1000;
            if self.auto_compact_started_ms <= 0
                || (now_ms - self.auto_compact_started_ms) > stale_after_ms
            {
                self.auto_compact_in_progress = false;
                self.auto_compact_started_ms = 0;
                self.auto_compact_last_skip =
                    Some("previous compact run timed out after 8h".to_string());
            }
        }

        let cache = match self.cache.clone() {
            Some(c) => c,
            None => return,
        };
        let uncompacted = cache
            .count_uncompacted_bars(auto_compact::TARGET_LEVEL)
            .unwrap_or(0);

        let (weekday, hour) = auto_compact::local_weekday_hour_now();
        let idle_for = now
            .saturating_duration_since(self.auto_compact_last_input_at)
            .as_secs();
        let inputs = auto_compact::GateInputs {
            enabled: self.auto_compact_enabled,
            schedule: self.auto_compact_schedule,
            last_run_ms: self.auto_compact_last_run_ms,
            now_ms,
            local_weekday: weekday,
            local_hour: hour,
            idle_for_secs: idle_for,
            on_ac: auto_compact::on_ac_power(),
            uncompacted_count: uncompacted,
            in_progress: self.auto_compact_in_progress,
        };
        let decision = auto_compact::evaluate_gate(&inputs);
        if !decision.run {
            self.auto_compact_last_skip = Some(decision.reason);
            return;
        }

        // Gate passed — dispatch the same BrokerCmd the manual button uses, so
        // the existing importing_flag coordination and progress logging apply.
        let db_path = cache_db_path();
        let _ = self.broker_tx.send(BrokerCmd::CompactStorage {
            db_path,
            level: auto_compact::TARGET_LEVEL,
        });
        self.auto_compact_in_progress = true;
        self.auto_compact_started_ms = now_ms;
        self.auto_compact_last_skip = None;
        self.log.push_back(LogEntry::info(format!(
            "Auto-compact (zstd-{}): {} entries pending — running in background",
            auto_compact::TARGET_LEVEL,
            uncompacted
        )));
    }

    pub(super) fn build_session_value(&self) -> serde_json::Value {
        serde_json::json!({
            "symbol": self.symbol_input,
            "active_tab": self.active_tab,
            "tabs": self.charts.iter().map(|c| serde_json::json!({
                "symbol": c.symbol,
                "timeframe": c.timeframe.label(),
                "chart_type": c.chart_type.label(),
                "log_scale": c.log_scale,
                "visible_bars": c.visible_bars,
                "view_offset": c.view_offset,
            })).collect::<Vec<_>>(),
            "indicators": {
                "sma200": self.show_sma200, "sma100": self.show_sma100,
                "kama": self.show_kama, "ema21": self.show_ema21,
                "bollinger": self.show_bollinger, "ichimoku": self.show_ichimoku,
                "wma": self.show_wma, "hma": self.show_hma,
                "psar": self.show_psar, "atr_proj": self.show_atr_proj,
                "prev_levels": self.show_prev_levels, "pivots": self.show_pivots,
                "fractals": self.show_fractals, "harmonics": self.show_harmonics, "supply_demand": self.show_supply_demand,
                "ehlers_ss": self.show_ehlers_ss, "ehlers_decycler": self.show_ehlers_decycler,
                "ehlers_itl": self.show_ehlers_itl, "ehlers_mama": self.show_ehlers_mama,
                "ehlers_ebsw": self.show_ehlers_ebsw, "ehlers_cyber": self.show_ehlers_cyber,
                "ehlers_cg": self.show_ehlers_cg, "ehlers_roof": self.show_ehlers_roof,
                "rsi": self.show_rsi, "fisher": self.show_fisher,
                "macd": self.show_macd, "stochastic": self.show_stochastic,
                "adx": self.show_adx, "cci": self.show_cci,
                "williams_r": self.show_williams_r, "obv": self.show_obv,
                "momentum": self.show_momentum, "cmo": self.show_cmo,
                "qstick": self.show_qstick, "disparity": self.show_disparity,
                "bop": self.show_bop, "stddev": self.show_stddev,
                "mfi": self.show_mfi, "trix": self.show_trix,
                "ppo": self.show_ppo, "ultosc": self.show_ultosc,
                "stochrsi": self.show_stochrsi,
                "var_oscillator": self.show_var_oscillator,
                "better_volume": self.show_better_volume,
                "volume_pane": self.show_volume_pane, "sessions": self.show_sessions,
                "vol_heatmap": self.show_vol_heatmap, "vwap": self.show_vwap,
                "price_histogram": self.show_price_histogram,
                "supertrend": self.show_supertrend, "donchian": self.show_donchian, "keltner": self.show_keltner,
                "regression": self.show_regression, "squeeze": self.show_squeeze,
                "fvg": self.show_fvg, "order_blocks": self.show_order_blocks,
            },
            "mtf_enabled": self.mtf_enabled,
            "mtf_cols": self.mtf_cols,
            "mtf_visible": self.mtf_visible,
            "kraken_scrape_schema": 3,
            "kraken_scrape_xstocks": self.kraken_scrape_xstocks,
            "kraken_scrape_usd_crypto": self.kraken_scrape_usd_crypto,
            "kraken_scrape_fiat_crypto": self.kraken_scrape_fiat_crypto,
            "kraken_scrape_crypto_crosses": self.kraken_scrape_crypto_crosses,
            "kraken_scrape_futures": self.kraken_scrape_futures,
            "alpaca_full_bar_sync_enabled": self.alpaca_full_bar_sync_enabled,
            "kraken_full_bar_sync_enabled": self.kraken_full_bar_sync_enabled,
            "backfill_alpaca_kraken_equities_enabled": self.backfill_alpaca_kraken_equities_enabled,
            "backfill_yahoo_chart_enabled": self.backfill_yahoo_chart_enabled,
            "kraken_ws_ohlc_enabled": self.kraken_ws_ohlc_enabled,
            "crypto_fiat_quote_usd": self.crypto_fiat_quote_usd,
            "crypto_fiat_quote_usdt": self.crypto_fiat_quote_usdt,
            "crypto_fiat_quote_usdc": self.crypto_fiat_quote_usdc,
            "crypto_fiat_quote_usdg": self.crypto_fiat_quote_usdg,
            "crypto_fiat_quote_eur": self.crypto_fiat_quote_eur,
            "crypto_fiat_quote_gbp": self.crypto_fiat_quote_gbp,
            "crypto_fiat_quote_cad": self.crypto_fiat_quote_cad,
            "crypto_fiat_quote_aud": self.crypto_fiat_quote_aud,
            "crypto_fiat_quote_jpy": self.crypto_fiat_quote_jpy,
            "crypto_fiat_quote_chf": self.crypto_fiat_quote_chf,
            "enabled_sync_timeframes": STANDARD_SYNC_TIMEFRAMES.iter()
                .filter_map(|(_, tf)| self.enabled_sync_timeframes.contains(*tf).then(|| serde_json::json!(tf)))
                .collect::<Vec<_>>(),
            "alpaca_historical_rpm_hint": self.alpaca_historical_rpm_hint,
            "command_open": self.command_open,
            "compact_mode": self.compact_mode,
            "broker_scope": match self.broker_scope {
                EventSource::All => "all",
                EventSource::Alpaca => "alpaca",
                EventSource::Kraken => "kraken",
                EventSource::Positions => "positions",
            },
            "econ_filter_high": self.econ_filter_high,
            "econ_filter_medium": self.econ_filter_medium,
            "econ_filter_low": self.econ_filter_low,
            "econ_filter_holiday": self.econ_filter_holiday,
            "econ_filter_currencies": self.econ_filter_currencies,
            "right_tab": match self.right_tab {
                RightTab::Trading => "trading",
                RightTab::Positions => "positions",
                RightTab::Orders => "orders",
                RightTab::Watchlist => "watchlist",
                RightTab::Risk => "risk",
            },
            "right_trading_open": self.right_trading_open,
            "right_positions_open": self.right_positions_open,
            "right_orders_open": self.right_orders_open,
            "right_watchlist_open": self.right_watchlist_open,
            "right_risk_open": self.right_risk_open,
            "right_recent_fills_open": self.right_recent_fills_open,
            "right_news_open": self.right_news_open,
            "news_search_query": self.news_search_query,
            "news_selected_url_hash": self.news_selected
                .and_then(|idx| self.news_full_articles.get(idx))
                .map(|a| a.url_hash.clone())
                .unwrap_or_else(|| self.news_selected_url_hash.clone()),
            "right_mtf_grid_open": self.right_mtf_grid_open,
            "right_panel_order": self.right_panel_order.iter()
                .map(|section| serde_json::json!(section.as_str()))
                .collect::<Vec<_>>(),
            "user_watchlist": self.user_watchlist,
            "workspaces": serde_json::Value::Object(
                self.workspaces.iter()
                    .map(|(k, v)| (k.clone(), serde_json::json!(v)))
                    .collect()
            ),
            "lan_client_enabled": self.lan_client_enabled,
            "lan_server_enabled": self.lan_server_enabled,
            "show_alpaca_positions": self.show_alpaca_positions,
            "show_kr_positions": self.show_kr_positions,
            "snap_enabled": self.snap_enabled,
            "cross_tf_drawings": self.cross_tf_drawings,
            "follow_latest": self.follow_latest,
            "draw_width": self.draw_width,
            "draw_color": [self.draw_color.r(), self.draw_color.g(), self.draw_color.b()],
            "draw_line_style": match self.draw_line_style { LineStyle::Solid => "solid", LineStyle::Dashed => "dashed", LineStyle::Dotted => "dotted" },
            "lan_server_ip": self.lan_server_ip,
            "lan_sync_host": self.lan_sync_host,
            "lan_sync_port": self.lan_sync_port,
            "codex_model": self.codex_model,
            "codex_reasoning_effort": self.codex_reasoning_effort,
            "hermes_model": self.hermes_model,
            "hermes_provider": self.hermes_provider,
            "grok_model": self.grok_model,
            "grok_effort": self.grok_effort,
            // Credentials: keyring-only (secure OS storage). Session stores non-secret config.
            "alpaca_enabled": self.alpaca_enabled,
            "alpaca_full_bar_sync_enabled": self.alpaca_full_bar_sync_enabled,
            "kraken_enabled": self.kraken_enabled,
            "kraken_full_bar_sync_enabled": self.kraken_full_bar_sync_enabled,
            "broker_paper": self.broker_paper,
            "sl_enabled": self.sl_enabled,
            "tp_enabled": self.tp_enabled,
            "windows": {
                "settings": self.show_settings,
                "risk_calc": self.show_risk_calc,
                "compound_calc": self.show_compound_calc,
                "calendar": self.show_calendar,
                "backtest": self.show_backtest,
                "news": self.show_news,
                "indicators_panel": self.show_indicators_panel,
                "screener": self.show_screener,
                "symbols": self.show_symbols,
                "optimizer": self.show_optimizer,
                "ai_chat": self.show_ai_chat,
                "claude_code": self.show_claude_code,
                "gemini_cli": self.show_gemini_cli,
                "codex_cli": self.show_codex_cli,
                "hermes_cli": self.show_hermes_cli,
                "grok_cli": self.show_grok_cli,
                "matrix_chat": self.show_matrix_chat,
                "sec": self.show_sec,
                "insider": self.show_insider,
                "fundamentals": self.show_fundamentals,
                "order_flow": self.show_order_flow,
                "bookmap": self.show_bookmap,
                "journal": self.show_journal,
                "var_mult": self.show_var_mult,
                "montecarlo": self.show_montecarlo,
                "earnings_calendar": self.show_earnings_calendar,
                "dividend_calendar": self.show_dividend_calendar,
                "event_calendar": self.show_event_calendar,
                "ev_scanner": self.show_ev_scanner,
                "stress_test": self.show_stress_test,
                "volume_profile": self.show_volume_profile,
                "hv_cone": self.show_hv_cone,
                "sector_heatmap": self.show_sector_heatmap,
                "dividends_screen": self.show_dividends,
                "alert_builder": self.show_alert_builder,
                "storage": self.show_storage,
                "sync_status": self.show_sync_status,
                "lan_sync": self.show_lan_sync,
                "unusual_volume": self.show_unusual_volume,
                "sector_rotation": self.show_sector_rotation,
                "fred": self.show_fred,
                "econ_calendar": self.show_econ_calendar,
                "congress": self.show_congress,
                "world_indices": self.show_world_indices,
                "crypto_top50": self.show_crypto_top50,
                "forex_matrix": self.show_forex_matrix,
                "help": self.show_help,
                "connect": self.show_connect,
                "data_window": self.show_data_window,
                "alerts": self.show_alerts,
                "scope_window": self.show_scope_window,
                "scrape_status": self.show_scrape_status,
                "fear_greed": self.show_fear_greed,
            },
            "journal": self.journal_entries.iter().map(|e| serde_json::json!({
                "timestamp": e.timestamp, "symbol": e.symbol, "side": e.side,
                "qty": e.qty, "entry_price": e.entry_price,
                "exit_price": e.exit_price, "pnl": e.pnl,
                "strategy": e.strategy, "notes": e.notes,
            })).collect::<Vec<_>>(),
            "drawings": self.charts.get(0).map(|c| {
                c.drawings.iter().filter_map(|d| match d {
                    Drawing::HLine { price, color } => Some(serde_json::json!({"type":"hline","price":price,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::VLine { bar_idx, color } => Some(serde_json::json!({"type":"vline","bar_idx":bar_idx,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::TrendLine { p1, p2, color } => Some(serde_json::json!({"type":"trendline","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::FiboRetrace { high, low, bar_start, bar_end } => Some(serde_json::json!({"type":"fibo","high":high,"low":low,"bar_start":bar_start,"bar_end":bar_end})),
                    Drawing::Rectangle { p1, p2, color } => Some(serde_json::json!({"type":"rect","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::Ray { origin, slope, color } => Some(serde_json::json!({"type":"ray","origin":[origin.0,origin.1],"slope":slope,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::Channel { p1, p2, width, color } => Some(serde_json::json!({"type":"channel","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"width":width,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::ExtendedLine { p1, p2, color } => Some(serde_json::json!({"type":"extline","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::HRay { bar_idx, price, color } => Some(serde_json::json!({"type":"hray","bar_idx":bar_idx,"price":price,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::CrossLine { bar_idx, price, color } => Some(serde_json::json!({"type":"crossline","bar_idx":bar_idx,"price":price,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::ArrowLine { p1, p2, color } => Some(serde_json::json!({"type":"arrowline","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::InfoLine { p1, p2, color } => Some(serde_json::json!({"type":"infoline","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::Pitchfork { pivot, p2, p3, color } => Some(serde_json::json!({"type":"pitchfork","pivot":[pivot.0,pivot.1],"p2":[p2.0,p2.1],"p3":[p3.0,p3.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::FiboExtension { p1, p2, p3, color } => Some(serde_json::json!({"type":"fiboext","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"p3":[p3.0,p3.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::GannFan { origin, scale, color } => Some(serde_json::json!({"type":"gannfan","origin":[origin.0,origin.1],"scale":scale,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::LongPosition { entry, stop, target } => Some(serde_json::json!({"type":"longpos","entry":[entry.0,entry.1],"stop":stop,"target":target})),
                    Drawing::ShortPosition { entry, stop, target } => Some(serde_json::json!({"type":"shortpos","entry":[entry.0,entry.1],"stop":stop,"target":target})),
                    Drawing::PriceRange { p1, p2 } => Some(serde_json::json!({"type":"pricerange","p1":[p1.0,p1.1],"p2":[p2.0,p2.1]})),
                    Drawing::TextLabel { bar_idx, price, text, color } => Some(serde_json::json!({"type":"text","bar_idx":bar_idx,"price":price,"text":text,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::ArrowMarker { bar_idx, price, is_up, color } => Some(serde_json::json!({"type":"arrowmarker","bar_idx":bar_idx,"price":price,"is_up":is_up,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::Ellipse { p1, p2, color } => Some(serde_json::json!({"type":"ellipse","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::Triangle { p1, p2, p3, color } => Some(serde_json::json!({"type":"triangle","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"p3":[p3.0,p3.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::TrendAngle { p1, p2, color } => Some(serde_json::json!({"type":"trendangle","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::ParallelChannel { p1, p2, offset, color } => Some(serde_json::json!({"type":"parallelch","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"offset":offset,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::FibChannel { p1, p2, p3, color } => Some(serde_json::json!({"type":"fibchannel","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"p3":[p3.0,p3.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::FibTimeZones { bar_idx, color } => Some(serde_json::json!({"type":"fibtimezones","bar_idx":bar_idx,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::PriceLabel { bar_idx, price, color } => Some(serde_json::json!({"type":"pricelabel","bar_idx":bar_idx,"price":price,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::Callout { anchor, label_pos, text, color } => Some(serde_json::json!({"type":"callout","anchor":[anchor.0,anchor.1],"label_pos":[label_pos.0,label_pos.1],"text":text,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::Highlighter { p1, p2, color } => Some(serde_json::json!({"type":"highlighter","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::CrossMarker { bar_idx, price, color } => Some(serde_json::json!({"type":"crossmarker","bar_idx":bar_idx,"price":price,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::Polyline { points, color } => Some(serde_json::json!({"type":"polyline","points":points.iter().map(|p| serde_json::json!([p.0, p.1])).collect::<Vec<_>>(),"color":[color.r(),color.g(),color.b()]})),
                    Drawing::AnchorNote { bar_idx, price, text, color } => Some(serde_json::json!({"type":"anchornote","bar_idx":bar_idx,"price":price,"text":text,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::RegressionChannel { p1, p2, color } => Some(serde_json::json!({"type":"regressionch","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::GannBox { p1, p2, color } => Some(serde_json::json!({"type":"gannbox","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::ElliottWave { points, color } => Some(serde_json::json!({"type":"elliott","points":points.iter().map(|p| serde_json::json!([p.0, p.1])).collect::<Vec<_>>(),"color":[color.r(),color.g(),color.b()]})),
                    Drawing::AbcCorrection { points, color } => Some(serde_json::json!({"type":"abc","points":points.iter().map(|p| serde_json::json!([p.0, p.1])).collect::<Vec<_>>(),"color":[color.r(),color.g(),color.b()]})),
                    Drawing::DateRange { p1, p2 } => Some(serde_json::json!({"type":"daterange","p1":[p1.0,p1.1],"p2":[p2.0,p2.1]})),
                    Drawing::DatePriceRange { p1, p2 } => Some(serde_json::json!({"type":"datepricerange","p1":[p1.0,p1.1],"p2":[p2.0,p2.1]})),
                    Drawing::HeadShoulders { points, color } => Some(serde_json::json!({"type":"headshoulders","points":points.iter().map(|p| serde_json::json!([p.0, p.1])).collect::<Vec<_>>(),"color":[color.r(),color.g(),color.b()]})),
                    Drawing::XabcdPattern { points, color } => Some(serde_json::json!({"type":"xabcd","points":points.iter().map(|p| serde_json::json!([p.0, p.1])).collect::<Vec<_>>(),"color":[color.r(),color.g(),color.b()]})),
                    Drawing::Brush { points, color } => Some(serde_json::json!({"type":"brush","points":points.iter().map(|p| serde_json::json!([p.0, p.1])).collect::<Vec<_>>(),"color":[color.r(),color.g(),color.b()]})),
                    Drawing::SchiffPitchfork { pivot, p2, p3, color } => Some(serde_json::json!({"type":"schiffpitchfork","pivot":[pivot.0,pivot.1],"p2":[p2.0,p2.1],"p3":[p3.0,p3.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::ModSchiffPitchfork { pivot, p2, p3, color } => Some(serde_json::json!({"type":"modschiffpitchfork","pivot":[pivot.0,pivot.1],"p2":[p2.0,p2.1],"p3":[p3.0,p3.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::CyclicLines { bar_start, bar_end, color } => Some(serde_json::json!({"type":"cycliclines","bar_start":bar_start,"bar_end":bar_end,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::SineWave { p1, p2, color } => Some(serde_json::json!({"type":"sinewave","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::Emoji { bar_idx, price, emoji } => Some(serde_json::json!({"type":"emoji","bar_idx":bar_idx,"price":price,"emoji":emoji})),
                    Drawing::Flag { bar_idx, price, color } => Some(serde_json::json!({"type":"flag","bar_idx":bar_idx,"price":price,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::Balloon { anchor, label_pos, text, color } => Some(serde_json::json!({"type":"balloon","anchor":[anchor.0,anchor.1],"label_pos":[label_pos.0,label_pos.1],"text":text,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::SessionBreak { bar_idx, color } => Some(serde_json::json!({"type":"sessionbreak","bar_idx":bar_idx,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::MagnetLevel { price, color } => Some(serde_json::json!({"type":"magnetlevel","price":price,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::RiskRewardBox { entry, stop, target } => Some(serde_json::json!({"type":"riskreward","entry":[entry.0,entry.1],"stop":stop,"target":target})),
                    Drawing::FibCircle { center, radius_pt, color } => Some(serde_json::json!({"type":"fibcircle","center":[center.0,center.1],"radius_pt":[radius_pt.0,radius_pt.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::ArcDraw { p1, p2, p3, color } => Some(serde_json::json!({"type":"arcdraw","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"p3":[p3.0,p3.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::CurveDraw { p1, ctrl1, ctrl2, p2, color } => Some(serde_json::json!({"type":"curvedraw","p1":[p1.0,p1.1],"ctrl1":[ctrl1.0,ctrl1.1],"ctrl2":[ctrl2.0,ctrl2.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::PathDraw { points, color } => Some(serde_json::json!({"type":"pathdraw","points":points.iter().map(|p| serde_json::json!([p.0, p.1])).collect::<Vec<_>>(),"color":[color.r(),color.g(),color.b()]})),
                    Drawing::Forecast { p1, p2, color } => Some(serde_json::json!({"type":"forecast","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::GhostFeed { p1, p2, color } => Some(serde_json::json!({"type":"ghostfeed","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::Signpost { bar_idx, price, color } => Some(serde_json::json!({"type":"signpost","bar_idx":bar_idx,"price":price,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::Ruler { p1, p2, color } => Some(serde_json::json!({"type":"ruler","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::TimeCycle { bar_start, bar_end, color } => Some(serde_json::json!({"type":"timecycle","bar_start":bar_start,"bar_end":bar_end,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::SpeedResistanceFan { p1, p2, p3, color } => Some(serde_json::json!({"type":"speedfan","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"p3":[p3.0,p3.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::SpeedResistanceArc { p1, p2, p3, color } => Some(serde_json::json!({"type":"speedarc","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"p3":[p3.0,p3.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::FibSpiral { center, radius_pt, color } => Some(serde_json::json!({"type":"fibspiral","center":[center.0,center.1],"radius_pt":[radius_pt.0,radius_pt.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::RotatedRectangle { p1, p2, p3, color } => Some(serde_json::json!({"type":"rotatedrect","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"p3":[p3.0,p3.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::AnchoredVwapLine { bar_idx, color } => Some(serde_json::json!({"type":"anchoredvwap","bar_idx":bar_idx,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::TrendChannel { p1, p2, p3, color } => Some(serde_json::json!({"type":"trendchannel","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"p3":[p3.0,p3.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::InsidePitchfork { pivot, p2, p3, color } => Some(serde_json::json!({"type":"insidepitchfork","pivot":[pivot.0,pivot.1],"p2":[p2.0,p2.1],"p3":[p3.0,p3.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::FibWedge { p1, p2, p3, color } => Some(serde_json::json!({"type":"fibwedge","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"p3":[p3.0,p3.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::PriceNote { price, text, color } => Some(serde_json::json!({"type":"pricenote","price":price,"text":text,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::MeasureTool { p1, p2, color } => Some(serde_json::json!({"type":"measuretool","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::AnchoredText { bar_idx, price, text, color } => Some(serde_json::json!({"type":"anchoredtext","bar_idx":bar_idx,"price":price,"text":text,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::Comment { bar_idx, price, text, color } => Some(serde_json::json!({"type":"comment","bar_idx":bar_idx,"price":price,"text":text,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::ArrowMarkerLeft { bar_idx, price, color } => Some(serde_json::json!({"type":"arrowleft","bar_idx":bar_idx,"price":price,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::ArrowMarkerRight { bar_idx, price, color } => Some(serde_json::json!({"type":"arrowright","bar_idx":bar_idx,"price":price,"color":[color.r(),color.g(),color.b()]})),
                    Drawing::Circle { p1, p2, color } => Some(serde_json::json!({"type":"circle","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::PitchFan { p1, p2, color } => Some(serde_json::json!({"type":"pitchfan","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::TrendFibTime { p1, p2, color } => Some(serde_json::json!({"type":"trendfibtime","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::GannSquare { p1, p2, color } => Some(serde_json::json!({"type":"gannsquare","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::GannSquareFixed { p1, p2, color } => Some(serde_json::json!({"type":"gannsquarefixed","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::BarsPattern { p1, p2, color } => Some(serde_json::json!({"type":"barspattern","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::Projection { p1, p2, color } => Some(serde_json::json!({"type":"projection","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::DoubleCurve { p1, p2, color } => Some(serde_json::json!({"type":"doublecurve","p1":[p1.0,p1.1],"p2":[p2.0,p2.1],"color":[color.r(),color.g(),color.b()]})),
                    Drawing::TrianglePattern { points, color } => Some(serde_json::json!({"type":"trianglepattern","points":points.iter().map(|p| serde_json::json!([p.0, p.1])).collect::<Vec<_>>(),"color":[color.r(),color.g(),color.b()]})),
                    Drawing::ThreeDrives { points, color } => Some(serde_json::json!({"type":"threedrives","points":points.iter().map(|p| serde_json::json!([p.0, p.1])).collect::<Vec<_>>(),"color":[color.r(),color.g(),color.b()]})),
                    Drawing::ElliottDouble { points, color } => Some(serde_json::json!({"type":"elliottdouble","points":points.iter().map(|p| serde_json::json!([p.0, p.1])).collect::<Vec<_>>(),"color":[color.r(),color.g(),color.b()]})),
                    Drawing::AbcdPattern { points, color } => Some(serde_json::json!({"type":"abcd","points":points.iter().map(|p| serde_json::json!([p.0, p.1])).collect::<Vec<_>>(),"color":[color.r(),color.g(),color.b()]})),
                    Drawing::CypherPattern { points, color } => Some(serde_json::json!({"type":"cypher","points":points.iter().map(|p| serde_json::json!([p.0, p.1])).collect::<Vec<_>>(),"color":[color.r(),color.g(),color.b()]})),
                    Drawing::ElliottTriangle { points, color } => Some(serde_json::json!({"type":"elliotttriangle","points":points.iter().map(|p| serde_json::json!([p.0, p.1])).collect::<Vec<_>>(),"color":[color.r(),color.g(),color.b()]})),
                    Drawing::ElliottTripleCombo { points, color } => Some(serde_json::json!({"type":"elliotttriple","points":points.iter().map(|p| serde_json::json!([p.0, p.1])).collect::<Vec<_>>(),"color":[color.r(),color.g(),color.b()]})),
                }).collect::<Vec<_>>()
            }).unwrap_or_default(),
            "alerts": self.alerts.iter().map(|(p, l)| serde_json::json!({"price": p, "label": l})).collect::<Vec<_>>(),
            "chart_templates": self.chart_templates.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<serde_json::Map<String, serde_json::Value>>(),
        })
    }

    /// Returns true for crypto symbols (sourced from Kraken). Used by the
    /// weekend crypto-sync scheduler and symbol-universe classification.
    pub(super) fn demand_is_crypto(sym: &str) -> bool {
        let crypto_bases = [
            "BTC", "ETH", "SOL", "DOGE", "XRP", "ADA", "LTC", "LINK", "AVAX", "DOT", "XMR", "ZEC",
            "DASH", "UNI", "AAVE", "MATIC", "SHIB", "ATOM", "ALGO", "FTM", "NEAR", "APE", "ARB",
            "OP", "MKR", "COMP", "SNX", "CRV", "SUSHI", "YFI", "BAT", "MANA", "SAND", "AXS", "BCH",
            "ETC", "XLM", "FIL", "HBAR", "ICP", "VET", "THETA",
        ];
        let su = sym.to_uppercase();
        crypto_bases.iter().any(|b| {
            su.starts_with(b)
                && (su.ends_with("USD") || su.ends_with("USDT") || su.ends_with("BTC"))
        })
    }

    pub(super) fn session_json_path() -> PathBuf {
        let mut path = dirs_home();
        path.push("session.json");
        path
    }

    pub(super) fn write_session_json(json: &str) {
        let path = Self::session_json_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, json);
    }

    /// Persist a session snapshot (session.json + the `app:sync_preferences` KV
    /// row), newest-wins. `seq` is the snapshot's monotonic write sequence; the
    /// shared `gate` holds the highest sequence already written, so a stale
    /// (lower-seq) write that lost a race against a newer one is dropped instead
    /// of clobbering it. Safe to call from a blocking worker or the UI thread —
    /// the `put_kv` here is what contends with bulk bar-sync writers, which is
    /// exactly why the per-frame autosave runs this off the render thread.
    fn persist_session_to_disk(
        gate: &std::sync::Mutex<u64>,
        seq: u64,
        session_json: &str,
        pref_json: &str,
        cache: Option<&Arc<SqliteCache>>,
    ) {
        let mut persisted = gate.lock().unwrap_or_else(|p| p.into_inner());
        if seq <= *persisted {
            return;
        }
        Self::write_session_json(session_json);
        if let Some(cache) = cache {
            let _ = cache.put_kv("app:sync_preferences", pref_json);
        }
        *persisted = seq;
    }

    pub(super) fn mark_session_snapshot_clean(&mut self) {
        self.session_last_saved_json = self.build_session_json();
        self.session_dirty_since = None;
        self.session_last_scan_at = std::time::Instant::now();
        self.session_state_ready = true;
    }

    pub(super) fn hydrate_loaded_charts(&mut self) {
        let Some(ref cache) = self.cache else {
            return;
        };
        if self.charts.is_empty() {
            return;
        }
        self.active_tab = self.active_tab.min(self.charts.len().saturating_sub(1));
        if self.mtf_enabled {
            let mut retry_chart_indices = Vec::new();
            for (idx, chart) in self.charts.iter_mut().enumerate() {
                if chart.bars.is_empty() {
                    let mut gpu = self.gpu_indicators.take();
                    if !chart.try_load(cache, &mut self.log, gpu.as_mut()) {
                        retry_chart_indices.push(idx);
                    }
                    self.gpu_indicators = gpu;
                }
            }
            for idx in retry_chart_indices {
                self.queue_chart_reload(idx);
            }
        } else if let Some(chart) = self.charts.get_mut(self.active_tab) {
            let mut gpu = self.gpu_indicators.take();
            if !chart.try_load(cache, &mut self.log, gpu.as_mut()) {
                self.queue_chart_reload(self.active_tab);
            }
            self.gpu_indicators = gpu;
        }
    }

    pub(super) fn maybe_incremental_session_save(&mut self, ctx: &egui::Context) {
        self.flush_alpaca_retry_queue(false);
        self.flush_alpaca_no_data_marks(false);
        self.flush_unresolvable_marks(false);
        self.flush_alpaca_backfill_complete_marks(false);
        self.flush_kraken_backfill_complete_marks(false);
        if self.heavy_sync_in_progress {
            // build_session_json() walks a large amount of UI/session state and
            // write_session_json()/sync_preferences_save() hit disk/SQLite. During
            // startup/full-catalog sync those background states churn constantly,
            // turning autosave into periodic render-thread stalls. Forced saves on
            // exit still persist the latest state; keep the frame loop responsive.
            return;
        }
        if !self.session_state_ready {
            return;
        }
        let now = std::time::Instant::now();
        // Adaptive scan cadence: 500ms while the session is actively changing,
        // backing off toward 2s after sustained no-change so an idle terminal
        // isn't rebuilding+diffing the session JSON twice a second for nothing.
        // Any detected change resets to the fast cadence; the save debounce and
        // forced/exit saves are unaffected.
        let scan_interval =
            std::time::Duration::from_millis(500 + u64::from(self.session_idle_scans.min(6)) * 250);
        let save_debounce = std::time::Duration::from_millis(1200);
        let since_last_scan = now.saturating_duration_since(self.session_last_scan_at);
        if since_last_scan < scan_interval {
            return;
        }
        self.session_last_scan_at = now;
        let json = self.build_session_json();
        if json == self.session_last_saved_json {
            self.session_idle_scans = self.session_idle_scans.saturating_add(1);
            self.session_dirty_since = None;
            return;
        }
        self.session_idle_scans = 0;
        let dirty_since = self.session_dirty_since.get_or_insert(now);
        let dirty_for = now.saturating_duration_since(*dirty_since);
        if dirty_for < save_debounce {
            ctx.request_repaint_after(save_debounce - dirty_for);
            return;
        }
        // A prior off-thread autosave is still writing. Don't pile up a second
        // worker or block the render thread — leave the dirty flag set and retry
        // next scan so the newest state is what eventually lands on disk.
        if self
            .session_save_in_flight
            .load(std::sync::atomic::Ordering::Acquire)
        {
            ctx.request_repaint_after(scan_interval);
            return;
        }
        // Build the small (~6 KB) preference blob on the UI thread (cheap), then
        // hand the session.json write + SQLite put_kv to a blocking worker. The
        // render thread no longer waits on the shared cache write mutex — held
        // for seconds by bulk bar-sync writers — which was the dominant source
        // of the multi-second frame stalls.
        self.session_save_seq += 1;
        let seq = self.session_save_seq;
        let pref_json =
            serde_json::to_string(&self.build_sync_preferences_value()).unwrap_or_default();
        let gate = self.session_write_gate.clone();
        let in_flight = self.session_save_in_flight.clone();
        let cache = self.cache.clone();
        let json_for_disk = json.clone();
        in_flight.store(true, std::sync::atomic::Ordering::Release);
        self.rt_handle.spawn_blocking(move || {
            Self::persist_session_to_disk(&gate, seq, &json_for_disk, &pref_json, cache.as_ref());
            in_flight.store(false, std::sync::atomic::Ordering::Release);
        });
        self.session_last_saved_json = json;
        self.session_dirty_since = None;
    }

    pub(super) fn save_session(&mut self) {
        self.flush_alpaca_retry_queue(true);
        self.flush_alpaca_no_data_marks(true);
        self.flush_unresolvable_marks(true);
        self.flush_alpaca_backfill_complete_marks(true);
        self.flush_kraken_backfill_complete_marks(true);
        // Persist credentials to keyring + SQLite fallback — on background thread to avoid UI freeze
        // (each keyring::store can take 50-200ms on Linux due to DBUS roundtrip × 11 keys = 1-2s freeze)
        let cred_pairs: Vec<(String, String)> = vec![
            (
                keyring::keys::ALPACA_API_KEY.into(),
                self.broker_api_key.clone(),
            ),
            (
                keyring::keys::ALPACA_SECRET.into(),
                self.broker_secret.clone(),
            ),
            (keyring::keys::FINNHUB_KEY.into(), self.finnhub_key.clone()),
            (keyring::keys::FRED_KEY.into(), self.fred_key.clone()),
            (
                keyring::keys::LAN_SYNC_PASS.into(),
                self.lan_sync_passphrase.clone(),
            ),
            (
                keyring::keys::DISCORD_WEBHOOK.into(),
                self.discord_webhook.clone(),
            ),
            (
                keyring::keys::PUSHOVER_TOKEN.into(),
                self.pushover_token.clone(),
            ),
            (
                keyring::keys::PUSHOVER_USER.into(),
                self.pushover_user.clone(),
            ),
            (keyring::keys::NTFY_TOPIC.into(), self.ntfy_topic.clone()),
            (
                keyring::keys::ANTHROPIC_KEY.into(),
                self.anthropic_key.clone(),
            ),
            (keyring::keys::OPENAI_KEY.into(), self.openai_key.clone()),
            (
                keyring::keys::KRAKEN_API_KEY.into(),
                self.kraken_api_key.clone(),
            ),
            (
                keyring::keys::KRAKEN_API_SECRET.into(),
                self.kraken_api_secret.clone(),
            ),
            (
                keyring::keys::KRAKEN_WS_API_KEY.into(),
                self.kraken_ws_api_key.clone(),
            ),
            (
                keyring::keys::KRAKEN_WS_API_SECRET.into(),
                self.kraken_ws_api_secret.clone(),
            ),
            (
                keyring::keys::CRYPTOPANIC_KEY.into(),
                self.cryptopanic_key.clone(),
            ),
        ];
        let cache_clone = self.cache.clone();
        let rt_handle = self.rt_handle.clone();
        rt_handle.spawn_blocking(move || {
            for (key, val) in &cred_pairs {
                let _ = keyring::store(key, val);
                if let Some(ref cache) = cache_clone {
                    let _ = cache.put_kv(&format!("cred:{}", key), val);
                }
            }
        });
        // Explicit save stays synchronous (atomic on the UI thread), but routes
        // through the write gate with a fresh, highest sequence so it always
        // wins over an in-flight background autosave that may finish afterward.
        let json = self.build_session_json();
        self.session_save_seq += 1;
        let seq = self.session_save_seq;
        let pref_json =
            serde_json::to_string(&self.build_sync_preferences_value()).unwrap_or_default();
        // Route the session.json write + SQLite put_kv off the UI thread (mirrors
        // the per-frame autosave). The monotonic `seq` + write gate guarantee this
        // explicit save still wins over any in-flight autosave, while keeping the
        // blocking put_kv — held for seconds by bulk bar-sync writers — off the
        // render thread (it was a 4 s+ frame stall during heavy sync).
        let gate = self.session_write_gate.clone();
        let cache = self.cache.clone();
        let json_for_disk = json.clone();
        self.rt_handle.spawn_blocking(move || {
            Self::persist_session_to_disk(&gate, seq, &json_for_disk, &pref_json, cache.as_ref());
        });
        self.session_last_saved_json = json;
        self.session_dirty_since = None;
        self.session_last_scan_at = std::time::Instant::now();
        self.session_state_ready = true;
    }

    pub(super) fn load_session(&mut self) {
        let path = Self::session_json_path();
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                self.workspaces.clear();
                self.chart_templates.clear();
                self.journal_entries.clear();
                self.alerts.clear();
                if let Some(sym) = v["symbol"].as_str() {
                    self.symbol_input = sym.to_string();
                }
                if let Some(mtf) = v["mtf_enabled"].as_bool() {
                    self.mtf_enabled = mtf;
                }
                if let Some(b) = v["command_open"].as_bool() {
                    self.command_open = b;
                }
                if let Some(b) = v["compact_mode"].as_bool() {
                    self.compact_mode = b;
                }
                self.apply_sync_preferences_value(&v);
                if let Some(tab) = v["active_tab"].as_u64() {
                    self.active_tab = tab as usize;
                }
                if let Some(arr) = v["mtf_visible"].as_array() {
                    self.mtf_visible = arr.iter().map(|v| v.as_bool().unwrap_or(true)).collect();
                }
                // Broker scope + econ filters (added 2026-04-09)
                self.broker_scope = match v["broker_scope"].as_str() {
                    Some("alpaca") => EventSource::Alpaca,
                    Some("kraken") => EventSource::Kraken,
                    Some("positions") => EventSource::Positions,
                    _ => EventSource::All,
                };
                if let Some(b) = v["econ_filter_high"].as_bool() {
                    self.econ_filter_high = b;
                }
                if let Some(b) = v["econ_filter_medium"].as_bool() {
                    self.econ_filter_medium = b;
                }
                if let Some(b) = v["econ_filter_low"].as_bool() {
                    self.econ_filter_low = b;
                }
                if let Some(b) = v["econ_filter_holiday"].as_bool() {
                    self.econ_filter_holiday = b;
                }
                if let Some(s) = v["econ_filter_currencies"].as_str() {
                    self.econ_filter_currencies = s.to_string();
                }
                // Restore tabs: symbol, timeframe, chart type — rebuild charts from session
                if let Some(tabs) = v["tabs"].as_array() {
                    if !tabs.is_empty() {
                        // Rebuild chart set from session data
                        self.charts.clear();
                        for tab in tabs {
                            // Canonicalise legacy sessions: before `bare_symbol_from_key`
                            // was introduced, Screener/watchlist load paths saved full
                            // cache keys (`kraken-equities:SLV:1Hour`) into chart.symbol. Normalise
                            // to bare here so try_load doesn't double-prefix.
                            let raw_sym = tab["symbol"].as_str().unwrap_or("CC");
                            let sym = bare_symbol_from_key(raw_sym);
                            let tf = tab["timeframe"]
                                .as_str()
                                .and_then(Timeframe::from_label)
                                .unwrap_or(Timeframe::H4);
                            let ct = match tab["chart_type"].as_str() {
                                Some("Heikin-Ashi") => ChartType::HeikinAshi,
                                Some("Line") => ChartType::Line,
                                Some("OHLC Bars") => ChartType::OhlcBars,
                                Some("Renko") => ChartType::Renko,
                                _ => ChartType::Candle,
                            };
                            let mut chart = ChartState::new(&sym, tf);
                            chart.chart_type = ct;
                            chart.log_scale = tab["log_scale"].as_bool().unwrap_or(false);
                            if let Some(visible_bars) = tab["visible_bars"].as_u64() {
                                chart.visible_bars = visible_bars as usize;
                            }
                            if let Some(view_offset) = tab["view_offset"].as_u64() {
                                chart.view_offset = view_offset as usize;
                            }
                            self.charts.push(chart);
                        }
                        self.active_tab = self.active_tab.min(self.charts.len().saturating_sub(1));
                        while self.mtf_visible.len() < self.charts.len() {
                            self.mtf_visible.push(true);
                        }
                        self.hydrate_loaded_charts();
                    }
                }
                if let Some(ind) = v.get("indicators") {
                    for (key, field) in [
                        ("sma200", &mut self.show_sma200),
                        ("sma100", &mut self.show_sma100),
                        ("kama", &mut self.show_kama),
                        ("ema21", &mut self.show_ema21),
                        ("bollinger", &mut self.show_bollinger),
                        ("ichimoku", &mut self.show_ichimoku),
                        ("wma", &mut self.show_wma),
                        ("hma", &mut self.show_hma),
                        ("psar", &mut self.show_psar),
                        ("atr_proj", &mut self.show_atr_proj),
                        ("prev_levels", &mut self.show_prev_levels),
                        ("pivots", &mut self.show_pivots),
                        ("fractals", &mut self.show_fractals),
                        ("harmonics", &mut self.show_harmonics),
                        ("supply_demand", &mut self.show_supply_demand),
                        ("ehlers_ss", &mut self.show_ehlers_ss),
                        ("ehlers_decycler", &mut self.show_ehlers_decycler),
                        ("ehlers_itl", &mut self.show_ehlers_itl),
                        ("ehlers_mama", &mut self.show_ehlers_mama),
                        ("ehlers_ebsw", &mut self.show_ehlers_ebsw),
                        ("ehlers_cyber", &mut self.show_ehlers_cyber),
                        ("ehlers_cg", &mut self.show_ehlers_cg),
                        ("ehlers_roof", &mut self.show_ehlers_roof),
                        ("rsi", &mut self.show_rsi),
                        ("fisher", &mut self.show_fisher),
                        ("macd", &mut self.show_macd),
                        ("stochastic", &mut self.show_stochastic),
                        ("adx", &mut self.show_adx),
                        ("cci", &mut self.show_cci),
                        ("williams_r", &mut self.show_williams_r),
                        ("obv", &mut self.show_obv),
                        ("momentum", &mut self.show_momentum),
                        ("cmo", &mut self.show_cmo),
                        ("qstick", &mut self.show_qstick),
                        ("disparity", &mut self.show_disparity),
                        ("bop", &mut self.show_bop),
                        ("stddev", &mut self.show_stddev),
                        ("mfi", &mut self.show_mfi),
                        ("trix", &mut self.show_trix),
                        ("ppo", &mut self.show_ppo),
                        ("ultosc", &mut self.show_ultosc),
                        ("stochrsi", &mut self.show_stochrsi),
                        ("var_oscillator", &mut self.show_var_oscillator),
                        ("better_volume", &mut self.show_better_volume),
                        ("volume_pane", &mut self.show_volume_pane),
                        ("sessions", &mut self.show_sessions),
                        ("vol_heatmap", &mut self.show_vol_heatmap),
                        ("vwap", &mut self.show_vwap),
                        ("price_histogram", &mut self.show_price_histogram),
                        ("supertrend", &mut self.show_supertrend),
                        ("donchian", &mut self.show_donchian),
                        ("keltner", &mut self.show_keltner),
                        ("regression", &mut self.show_regression),
                        ("squeeze", &mut self.show_squeeze),
                        ("fvg", &mut self.show_fvg),
                        ("order_blocks", &mut self.show_order_blocks),
                    ] {
                        if let Some(b) = ind[key].as_bool() {
                            *field = b;
                        }
                    }
                }
                // Restore drawings (all types)
                if let Some(drawings) = v["drawings"].as_array() {
                    if let Some(chart) = self.charts.get_mut(0) {
                        let parse_col = |d: &serde_json::Value| -> egui::Color32 {
                            let c = &d["color"];
                            egui::Color32::from_rgb(
                                c[0].as_u64().unwrap_or(200) as u8,
                                c[1].as_u64().unwrap_or(200) as u8,
                                c[2].as_u64().unwrap_or(200) as u8,
                            )
                        };
                        let parse_pt = |d: &serde_json::Value, key: &str| -> Option<(usize, f64)> {
                            let a = &d[key];
                            Some((a[0].as_u64()? as usize, a[1].as_f64()?))
                        };
                        for d in drawings {
                            match d["type"].as_str() {
                                Some("hline") => {
                                    if let Some(price) = d["price"].as_f64() {
                                        chart.drawings.push(Drawing::HLine {
                                            price,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("vline") => {
                                    if let Some(idx) = d["bar_idx"].as_u64() {
                                        chart.drawings.push(Drawing::VLine {
                                            bar_idx: idx as usize,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("trendline") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::TrendLine {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("fibo") => {
                                    if let (Some(h), Some(l), Some(bs), Some(be)) = (
                                        d["high"].as_f64(),
                                        d["low"].as_f64(),
                                        d["bar_start"].as_u64(),
                                        d["bar_end"].as_u64(),
                                    ) {
                                        chart.drawings.push(Drawing::FiboRetrace {
                                            high: h,
                                            low: l,
                                            bar_start: bs as usize,
                                            bar_end: be as usize,
                                        });
                                    }
                                }
                                Some("rect") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::Rectangle {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("ray") => {
                                    if let (Some(o), Some(s)) =
                                        (parse_pt(d, "origin"), d["slope"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::Ray {
                                            origin: o,
                                            slope: s,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("channel") => {
                                    if let (Some(p1), Some(p2), Some(w)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"), d["width"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::Channel {
                                            p1,
                                            p2,
                                            width: w,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("extline") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::ExtendedLine {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("hray") => {
                                    if let (Some(idx), Some(p)) =
                                        (d["bar_idx"].as_u64(), d["price"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::HRay {
                                            bar_idx: idx as usize,
                                            price: p,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("crossline") => {
                                    if let (Some(idx), Some(p)) =
                                        (d["bar_idx"].as_u64(), d["price"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::CrossLine {
                                            bar_idx: idx as usize,
                                            price: p,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("arrowline") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::ArrowLine {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("infoline") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::InfoLine {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("pitchfork") => {
                                    if let (Some(pv), Some(p2), Some(p3)) =
                                        (parse_pt(d, "pivot"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::Pitchfork {
                                            pivot: pv,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("fiboext") => {
                                    if let (Some(p1), Some(p2), Some(p3)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::FiboExtension {
                                            p1,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("gannfan") => {
                                    if let (Some(o), Some(s)) =
                                        (parse_pt(d, "origin"), d["scale"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::GannFan {
                                            origin: o,
                                            scale: s,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("longpos") => {
                                    if let (Some(e), Some(s), Some(t)) = (
                                        parse_pt(d, "entry"),
                                        d["stop"].as_f64(),
                                        d["target"].as_f64(),
                                    ) {
                                        chart.drawings.push(Drawing::LongPosition {
                                            entry: e,
                                            stop: s,
                                            target: t,
                                        });
                                    }
                                }
                                Some("shortpos") => {
                                    if let (Some(e), Some(s), Some(t)) = (
                                        parse_pt(d, "entry"),
                                        d["stop"].as_f64(),
                                        d["target"].as_f64(),
                                    ) {
                                        chart.drawings.push(Drawing::ShortPosition {
                                            entry: e,
                                            stop: s,
                                            target: t,
                                        });
                                    }
                                }
                                Some("pricerange") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::PriceRange { p1, p2 });
                                    }
                                }
                                Some("text") => {
                                    if let (Some(idx), Some(p), Some(t)) = (
                                        d["bar_idx"].as_u64(),
                                        d["price"].as_f64(),
                                        d["text"].as_str(),
                                    ) {
                                        chart.drawings.push(Drawing::TextLabel {
                                            bar_idx: idx as usize,
                                            price: p,
                                            text: t.to_string(),
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("arrowmarker") => {
                                    if let (Some(idx), Some(p), Some(up)) = (
                                        d["bar_idx"].as_u64(),
                                        d["price"].as_f64(),
                                        d["is_up"].as_bool(),
                                    ) {
                                        chart.drawings.push(Drawing::ArrowMarker {
                                            bar_idx: idx as usize,
                                            price: p,
                                            is_up: up,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("ellipse") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::Ellipse {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("triangle") => {
                                    if let (Some(p1), Some(p2), Some(p3)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::Triangle {
                                            p1,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("trendangle") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::TrendAngle {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("parallelch") => {
                                    if let (Some(p1), Some(p2), Some(off)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"), d["offset"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::ParallelChannel {
                                            p1,
                                            p2,
                                            offset: off,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("fibchannel") => {
                                    if let (Some(p1), Some(p2), Some(p3)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::FibChannel {
                                            p1,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("fibtimezones") => {
                                    if let Some(idx) = d["bar_idx"].as_u64() {
                                        chart.drawings.push(Drawing::FibTimeZones {
                                            bar_idx: idx as usize,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("pricelabel") => {
                                    if let (Some(idx), Some(p)) =
                                        (d["bar_idx"].as_u64(), d["price"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::PriceLabel {
                                            bar_idx: idx as usize,
                                            price: p,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("callout") => {
                                    if let (Some(a), Some(lp), Some(t)) = (
                                        parse_pt(d, "anchor"),
                                        parse_pt(d, "label_pos"),
                                        d["text"].as_str(),
                                    ) {
                                        chart.drawings.push(Drawing::Callout {
                                            anchor: a,
                                            label_pos: lp,
                                            text: t.to_string(),
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("highlighter") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::Highlighter {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("crossmarker") => {
                                    if let (Some(idx), Some(p)) =
                                        (d["bar_idx"].as_u64(), d["price"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::CrossMarker {
                                            bar_idx: idx as usize,
                                            price: p,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("polyline") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::Polyline {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("anchornote") => {
                                    if let (Some(idx), Some(p), Some(t)) = (
                                        d["bar_idx"].as_u64(),
                                        d["price"].as_f64(),
                                        d["text"].as_str(),
                                    ) {
                                        chart.drawings.push(Drawing::AnchorNote {
                                            bar_idx: idx as usize,
                                            price: p,
                                            text: t.to_string(),
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("regressionch") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::RegressionChannel {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("gannbox") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::GannBox {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("elliott") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::ElliottWave {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("abc") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::AbcCorrection {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("daterange") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::DateRange { p1, p2 });
                                    }
                                }
                                Some("datepricerange") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::DatePriceRange { p1, p2 });
                                    }
                                }
                                Some("headshoulders") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::HeadShoulders {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("xabcd") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::XabcdPattern {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("brush") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::Brush {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("schiffpitchfork") => {
                                    if let (Some(pv), Some(p2), Some(p3)) =
                                        (parse_pt(d, "pivot"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::SchiffPitchfork {
                                            pivot: pv,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("modschiffpitchfork") => {
                                    if let (Some(pv), Some(p2), Some(p3)) =
                                        (parse_pt(d, "pivot"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::ModSchiffPitchfork {
                                            pivot: pv,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("cycliclines") => {
                                    if let (Some(bs), Some(be)) =
                                        (d["bar_start"].as_u64(), d["bar_end"].as_u64())
                                    {
                                        chart.drawings.push(Drawing::CyclicLines {
                                            bar_start: bs as usize,
                                            bar_end: be as usize,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("sinewave") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::SineWave {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("emoji") => {
                                    if let (Some(idx), Some(p)) =
                                        (d["bar_idx"].as_u64(), d["price"].as_f64())
                                    {
                                        let emoji =
                                            d["emoji"].as_str().unwrap_or("\u{1F3AF}").to_string();
                                        chart.drawings.push(Drawing::Emoji {
                                            bar_idx: idx as usize,
                                            price: p,
                                            emoji,
                                        });
                                    }
                                }
                                Some("flag") => {
                                    if let (Some(idx), Some(p)) =
                                        (d["bar_idx"].as_u64(), d["price"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::Flag {
                                            bar_idx: idx as usize,
                                            price: p,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("balloon") => {
                                    if let (Some(a), Some(lp), Some(t)) = (
                                        parse_pt(d, "anchor"),
                                        parse_pt(d, "label_pos"),
                                        d["text"].as_str(),
                                    ) {
                                        chart.drawings.push(Drawing::Balloon {
                                            anchor: a,
                                            label_pos: lp,
                                            text: t.to_string(),
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("sessionbreak") => {
                                    if let Some(idx) = d["bar_idx"].as_u64() {
                                        chart.drawings.push(Drawing::SessionBreak {
                                            bar_idx: idx as usize,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("magnetlevel") => {
                                    if let Some(price) = d["price"].as_f64() {
                                        chart.drawings.push(Drawing::MagnetLevel {
                                            price,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("riskreward") => {
                                    if let (Some(e), Some(s), Some(t)) = (
                                        parse_pt(d, "entry"),
                                        d["stop"].as_f64(),
                                        d["target"].as_f64(),
                                    ) {
                                        chart.drawings.push(Drawing::RiskRewardBox {
                                            entry: e,
                                            stop: s,
                                            target: t,
                                        });
                                    }
                                }
                                Some("fibcircle") => {
                                    if let (Some(c), Some(r)) =
                                        (parse_pt(d, "center"), parse_pt(d, "radius_pt"))
                                    {
                                        chart.drawings.push(Drawing::FibCircle {
                                            center: c,
                                            radius_pt: r,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("arcdraw") => {
                                    if let (Some(p1), Some(p2), Some(p3)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::ArcDraw {
                                            p1,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("curvedraw") => {
                                    if let (Some(p1), Some(c1), Some(c2), Some(p2)) = (
                                        parse_pt(d, "p1"),
                                        parse_pt(d, "ctrl1"),
                                        parse_pt(d, "ctrl2"),
                                        parse_pt(d, "p2"),
                                    ) {
                                        chart.drawings.push(Drawing::CurveDraw {
                                            p1,
                                            ctrl1: c1,
                                            ctrl2: c2,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("pathdraw") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::PathDraw {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("forecast") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::Forecast {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("ghostfeed") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::GhostFeed {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("signpost") => {
                                    if let (Some(idx), Some(p)) =
                                        (d["bar_idx"].as_u64(), d["price"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::Signpost {
                                            bar_idx: idx as usize,
                                            price: p,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("ruler") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::Ruler {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("timecycle") => {
                                    if let (Some(bs), Some(be)) =
                                        (d["bar_start"].as_u64(), d["bar_end"].as_u64())
                                    {
                                        chart.drawings.push(Drawing::TimeCycle {
                                            bar_start: bs as usize,
                                            bar_end: be as usize,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("speedfan") => {
                                    if let (Some(p1), Some(p2), Some(p3)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::SpeedResistanceFan {
                                            p1,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("speedarc") => {
                                    if let (Some(p1), Some(p2), Some(p3)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::SpeedResistanceArc {
                                            p1,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("fibspiral") => {
                                    if let (Some(c), Some(r)) =
                                        (parse_pt(d, "center"), parse_pt(d, "radius_pt"))
                                    {
                                        chart.drawings.push(Drawing::FibSpiral {
                                            center: c,
                                            radius_pt: r,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("rotatedrect") => {
                                    if let (Some(p1), Some(p2), Some(p3)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::RotatedRectangle {
                                            p1,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("anchoredvwap") => {
                                    if let Some(idx) = d["bar_idx"].as_u64() {
                                        chart.drawings.push(Drawing::AnchoredVwapLine {
                                            bar_idx: idx as usize,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("trendchannel") => {
                                    if let (Some(p1), Some(p2), Some(p3)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::TrendChannel {
                                            p1,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("insidepitchfork") => {
                                    if let (Some(pv), Some(p2), Some(p3)) =
                                        (parse_pt(d, "pivot"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::InsidePitchfork {
                                            pivot: pv,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("fibwedge") => {
                                    if let (Some(p1), Some(p2), Some(p3)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::FibWedge {
                                            p1,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("pricenote") => {
                                    if let (Some(p), Some(t)) =
                                        (d["price"].as_f64(), d["text"].as_str())
                                    {
                                        chart.drawings.push(Drawing::PriceNote {
                                            price: p,
                                            text: t.to_string(),
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("measuretool") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::MeasureTool {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("anchoredtext") => {
                                    if let (Some(idx), Some(p), Some(t)) = (
                                        d["bar_idx"].as_u64(),
                                        d["price"].as_f64(),
                                        d["text"].as_str(),
                                    ) {
                                        chart.drawings.push(Drawing::AnchoredText {
                                            bar_idx: idx as usize,
                                            price: p,
                                            text: t.to_string(),
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("comment") => {
                                    if let (Some(idx), Some(p), Some(t)) = (
                                        d["bar_idx"].as_u64(),
                                        d["price"].as_f64(),
                                        d["text"].as_str(),
                                    ) {
                                        chart.drawings.push(Drawing::Comment {
                                            bar_idx: idx as usize,
                                            price: p,
                                            text: t.to_string(),
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("arrowleft") => {
                                    if let (Some(idx), Some(p)) =
                                        (d["bar_idx"].as_u64(), d["price"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::ArrowMarkerLeft {
                                            bar_idx: idx as usize,
                                            price: p,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("arrowright") => {
                                    if let (Some(idx), Some(p)) =
                                        (d["bar_idx"].as_u64(), d["price"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::ArrowMarkerRight {
                                            bar_idx: idx as usize,
                                            price: p,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("circle") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::Circle {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("pitchfan") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::PitchFan {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("trendfibtime") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::TrendFibTime {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("gannsquare") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::GannSquare {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("gannsquarefixed") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::GannSquareFixed {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("barspattern") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::BarsPattern {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("projection") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::Projection {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("doublecurve") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::DoubleCurve {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("trianglepattern") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::TrianglePattern {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("threedrives") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::ThreeDrives {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("elliottdouble") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::ElliottDouble {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("abcd") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::AbcdPattern {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("cypher") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::CypherPattern {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("elliotttriangle") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::ElliottTriangle {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("elliotttriple") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::ElliottTripleCombo {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                // Restore alerts
                if let Some(alerts) = v["alerts"].as_array() {
                    for a in alerts {
                        if let (Some(p), Some(l)) = (a["price"].as_f64(), a["label"].as_str()) {
                            self.alerts.push((p, l.to_string()));
                        }
                    }
                }
                // Restore chart templates
                if let Some(templates) = v["chart_templates"].as_object() {
                    for (name, snap) in templates {
                        self.chart_templates.insert(name.clone(), snap.clone());
                    }
                }
                // Restore MTF cols
                if let Some(cols) = v["mtf_cols"].as_u64() {
                    self.mtf_cols = cols as usize;
                }
                if let Some(b) = v["fund_source_alpaca"].as_bool() {
                    self.fund_source_alpaca = b;
                }
                if let Some(b) = v["fund_source_kraken"].as_bool() {
                    self.fund_source_kraken = b;
                }
                // Restore right panel tab
                self.right_tab = match v["right_tab"].as_str() {
                    Some("positions") => RightTab::Positions,
                    Some("orders") => RightTab::Orders,
                    Some("watchlist") => RightTab::Watchlist,
                    Some("risk") => RightTab::Risk,
                    _ => RightTab::Trading,
                };
                if let Some(b) = v["right_trading_open"].as_bool() {
                    self.right_trading_open = b;
                }
                if let Some(b) = v["right_positions_open"].as_bool() {
                    self.right_positions_open = b;
                }
                if let Some(b) = v["right_orders_open"].as_bool() {
                    self.right_orders_open = b;
                }
                if let Some(b) = v["right_watchlist_open"].as_bool() {
                    self.right_watchlist_open = b;
                }
                if let Some(b) = v["right_risk_open"].as_bool() {
                    self.right_risk_open = b;
                }
                if let Some(b) = v["right_recent_fills_open"].as_bool() {
                    self.right_recent_fills_open = b;
                }
                if let Some(b) = v["right_news_open"].as_bool() {
                    self.right_news_open = b;
                }
                if let Some(s) = v["news_search_query"].as_str() {
                    self.news_search_query = s.to_string();
                }
                if let Some(s) = v["news_selected_url_hash"].as_str() {
                    self.news_selected_url_hash = s.to_string();
                }
                if let Some(b) = v["right_mtf_grid_open"].as_bool() {
                    self.right_mtf_grid_open = b;
                }
                if let Some(order) = v["right_panel_order"].as_array() {
                    self.right_panel_order = order
                        .iter()
                        .filter_map(|value| value.as_str())
                        .filter_map(RightPanelSectionId::from_str)
                        .collect();
                    self.normalized_right_panel_order();
                }
                if let Some(model) = v["codex_model"].as_str() {
                    self.codex_model = model.to_string();
                }
                if let Some(effort) = v["codex_reasoning_effort"].as_str() {
                    self.codex_reasoning_effort =
                        Self::normalize_codex_reasoning_effort(effort).to_string();
                }
                if let Some(model) = v["hermes_model"].as_str() {
                    self.hermes_model = model.to_string();
                }
                if let Some(provider) = v["hermes_provider"].as_str() {
                    self.hermes_provider = provider.to_string();
                }
                if let Some(model) = v["grok_model"].as_str() {
                    self.grok_model = model.to_string();
                }
                if let Some(effort) = v["grok_effort"].as_str() {
                    self.grok_effort = Self::normalize_grok_effort(effort).to_string();
                }
                // Migration fallback: load credentials from old session.json if keyring is empty.
                // Secrets are no longer written to session.json (see save_session).
                // Once a session has been saved under the new code these keys will be absent.
                if self.finnhub_key.is_empty() {
                    if let Some(fk) = v["finnhub_key"].as_str() {
                        self.finnhub_key = fk.to_string();
                    }
                }
                if self.fred_key.is_empty() {
                    if let Some(fk) = v["fred_key"].as_str() {
                        self.fred_key = fk.to_string();
                    }
                }
                if self.broker_api_key.is_empty() {
                    if let Some(ak) = v["broker_api_key"].as_str() {
                        self.broker_api_key = ak.to_string();
                    }
                }
                if self.broker_secret.is_empty() {
                    if let Some(bs) = v["broker_secret"].as_str() {
                        self.broker_secret = bs.to_string();
                    }
                }
                if let Some(enabled) = v["alpaca_enabled"].as_bool() {
                    self.alpaca_enabled = enabled;
                }
                if let Some(enabled) = v["alpaca_full_bar_sync_enabled"].as_bool() {
                    self.alpaca_full_bar_sync_enabled = enabled;
                }
                if let Some(enabled) = v["kraken_full_bar_sync_enabled"].as_bool() {
                    self.kraken_full_bar_sync_enabled = enabled;
                }
                if let Some(enabled) = v["kraken_enabled"].as_bool() {
                    self.kraken_enabled = enabled;
                }
                if let Some(enabled) = v["backfill_alpaca_kraken_equities_enabled"].as_bool() {
                    self.backfill_alpaca_kraken_equities_enabled = enabled;
                }
                if let Some(enabled) = v["backfill_yahoo_chart_enabled"].as_bool() {
                    self.backfill_yahoo_chart_enabled = enabled;
                }

                if let Some(bp) = v["broker_paper"].as_bool() {
                    self.broker_paper = bp;
                }
                // Restore user watchlist
                if let Some(wl) = v["user_watchlist"].as_array() {
                    self.user_watchlist = wl
                        .iter()
                        .filter_map(|s| s.as_str().map(String::from))
                        .collect();
                }
                if let Some(obj) = v["workspaces"].as_object() {
                    self.workspaces = obj
                        .iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect();
                }
                // Restore LAN client config
                if let Some(b) = v["lan_client_enabled"].as_bool() {
                    self.lan_client_enabled = b;
                }
                if let Some(b) = v["lan_server_enabled"].as_bool() {
                    self.lan_server_enabled = b;
                }
                if let Some(b) = v["show_alpaca_positions"].as_bool() {
                    self.show_alpaca_positions = b;
                }
                if let Some(b) = v["show_kr_positions"].as_bool() {
                    self.show_kr_positions = b;
                }
                if let Some(b) = v["snap_enabled"].as_bool() {
                    self.snap_enabled = b;
                }
                if let Some(b) = v["cross_tf_drawings"].as_bool() {
                    self.cross_tf_drawings = b;
                }
                if let Some(b) = v["follow_latest"].as_bool() {
                    self.follow_latest = b;
                }
                if let Some(w) = v["draw_width"].as_f64() {
                    self.draw_width = w as f32;
                }
                if let Some(arr) = v["draw_color"].as_array() {
                    if arr.len() == 3 {
                        let r = arr[0].as_u64().unwrap_or(0) as u8;
                        let g = arr[1].as_u64().unwrap_or(188) as u8;
                        let b = arr[2].as_u64().unwrap_or(212) as u8;
                        self.draw_color = egui::Color32::from_rgb(r, g, b);
                    }
                }
                if let Some(s) = v["draw_line_style"].as_str() {
                    self.draw_line_style = match s {
                        "dashed" => LineStyle::Dashed,
                        "dotted" => LineStyle::Dotted,
                        _ => LineStyle::Solid,
                    };
                }
                if let Some(s) = v["lan_server_ip"].as_str() {
                    self.lan_server_ip = s.to_string();
                }
                if let Some(s) = v["lan_sync_host"].as_str() {
                    self.lan_sync_host = s.to_string();
                }
                if let Some(s) = v["lan_sync_port"].as_str() {
                    self.lan_sync_port = s.to_string();
                }
                // Restore SL/TP state
                if let Some(sl) = v["sl_enabled"].as_bool() {
                    self.sl_enabled = sl;
                }
                if let Some(tp) = v["tp_enabled"].as_bool() {
                    self.tp_enabled = tp;
                }
                // Restore window visibility
                if let Some(w) = v.get("windows") {
                    if let Some(b) = w["settings"].as_bool() {
                        self.show_settings = b;
                    }
                    if let Some(b) = w["risk_calc"].as_bool() {
                        self.show_risk_calc = b;
                    }
                    if let Some(b) = w["compound_calc"].as_bool() {
                        self.show_compound_calc = b;
                    }
                    if let Some(b) = w["calendar"].as_bool() {
                        self.show_calendar = b;
                    }
                    if let Some(b) = w["backtest"].as_bool() {
                        self.show_backtest = b;
                    }
                    if let Some(b) = w["news"].as_bool() {
                        self.show_news = b;
                    }
                    if let Some(b) = w["indicators_panel"].as_bool() {
                        self.show_indicators_panel = b;
                    }
                    if let Some(b) = w["screener"].as_bool() {
                        self.show_screener = b;
                    }
                    if let Some(b) = w["symbols"].as_bool() {
                        self.show_symbols = b;
                    }
                    if let Some(b) = w["optimizer"].as_bool() {
                        self.show_optimizer = b;
                    }
                    if let Some(b) = w["ai_chat"].as_bool() {
                        self.show_ai_chat = b;
                    }
                    if let Some(b) = w["claude_code"].as_bool() {
                        self.show_claude_code = b;
                    }
                    if let Some(b) = w["gemini_cli"].as_bool() {
                        self.show_gemini_cli = b;
                    }
                    if let Some(b) = w["codex_cli"].as_bool() {
                        self.show_codex_cli = b;
                    }
                    if let Some(b) = w["hermes_cli"].as_bool() {
                        self.show_hermes_cli = b;
                    }
                    if let Some(b) = w["grok_cli"].as_bool() {
                        self.show_grok_cli = b;
                    }
                    if let Some(b) = w["matrix_chat"].as_bool() {
                        self.show_matrix_chat = b;
                    }
                    if let Some(b) = w["sec"].as_bool() {
                        self.show_sec = b;
                    }
                    if let Some(b) = w["insider"].as_bool() {
                        self.show_insider = b;
                    }
                    if let Some(b) = w["fundamentals"].as_bool() {
                        self.show_fundamentals = b;
                    }
                    if let Some(b) = w["order_flow"].as_bool() {
                        self.show_order_flow = b;
                    }
                    if let Some(b) = w["bookmap"].as_bool() {
                        self.show_bookmap = b;
                    }
                    if let Some(b) = w["journal"].as_bool() {
                        self.show_journal = b;
                    }
                    if let Some(b) = w["var_mult"].as_bool() {
                        self.show_var_mult = b;
                    }
                    if let Some(b) = w["montecarlo"].as_bool() {
                        self.show_montecarlo = b;
                    }
                    if let Some(b) = w["earnings_calendar"].as_bool() {
                        self.show_earnings_calendar = b;
                    }
                    if let Some(b) = w["dividend_calendar"].as_bool() {
                        self.show_dividend_calendar = b;
                    }
                    if let Some(b) = w["event_calendar"].as_bool() {
                        self.show_event_calendar = b;
                    }
                    if let Some(b) = w["ev_scanner"].as_bool() {
                        self.show_ev_scanner = b;
                    }
                    if let Some(b) = w["stress_test"].as_bool() {
                        self.show_stress_test = b;
                    }
                    if let Some(b) = w["volume_profile"].as_bool() {
                        self.show_volume_profile = b;
                    }
                    if let Some(b) = w["hv_cone"].as_bool() {
                        self.show_hv_cone = b;
                    }
                    if let Some(b) = w["sector_heatmap"].as_bool() {
                        self.show_sector_heatmap = b;
                    }
                    if let Some(b) = w["dividends_screen"].as_bool() {
                        self.show_dividends = b;
                    }
                    if let Some(b) = w["alert_builder"].as_bool() {
                        self.show_alert_builder = b;
                    }
                    if let Some(b) = w["storage"].as_bool() {
                        self.show_storage = b;
                    }
                    if let Some(b) = w["sync_status"].as_bool() {
                        self.show_sync_status = b;
                    }
                    if let Some(b) = w["lan_sync"].as_bool() {
                        self.show_lan_sync = b;
                    }
                    if let Some(b) = w["unusual_volume"].as_bool() {
                        self.show_unusual_volume = b;
                    }
                    if let Some(b) = w["sector_rotation"].as_bool() {
                        self.show_sector_rotation = b;
                    }
                    if let Some(b) = w["fred"].as_bool() {
                        self.show_fred = b;
                    }
                    if let Some(b) = w["econ_calendar"].as_bool() {
                        self.show_econ_calendar = b;
                    }
                    if let Some(b) = w["congress"].as_bool() {
                        self.show_congress = b;
                    }
                    if let Some(b) = w["world_indices"].as_bool() {
                        self.show_world_indices = b;
                    }
                    if let Some(b) = w["crypto_top50"].as_bool() {
                        self.show_crypto_top50 = b;
                    }
                    if let Some(b) = w["forex_matrix"].as_bool() {
                        self.show_forex_matrix = b;
                    }
                    if let Some(b) = w["help"].as_bool() {
                        self.show_help = b;
                    }
                    if let Some(b) = w["connect"].as_bool() {
                        self.show_connect = b;
                    }
                    if let Some(b) = w["data_window"].as_bool() {
                        self.show_data_window = b;
                    }
                    if let Some(b) = w["alerts"].as_bool() {
                        self.show_alerts = b;
                    }
                    if let Some(b) = w["scope_window"].as_bool() {
                        self.show_scope_window = b;
                    }
                    if let Some(b) = w["scrape_status"].as_bool() {
                        self.show_scrape_status = b;
                    }
                    if let Some(b) = w["fear_greed"].as_bool() {
                        self.show_fear_greed = b;
                    }
                }
                // Restore journal entries
                if let Some(journal) = v["journal"].as_array() {
                    for entry in journal {
                        self.journal_entries.push(JournalEntry {
                            timestamp: entry["timestamp"].as_str().unwrap_or("").to_string(),
                            symbol: entry["symbol"].as_str().unwrap_or("").to_string(),
                            side: entry["side"].as_str().unwrap_or("BUY").to_string(),
                            qty: entry["qty"].as_f64().unwrap_or(1.0),
                            entry_price: entry["entry_price"].as_f64().unwrap_or(0.0),
                            exit_price: entry["exit_price"].as_f64(),
                            pnl: entry["pnl"].as_f64(),
                            strategy: entry["strategy"].as_str().unwrap_or("").to_string(),
                            notes: entry["notes"].as_str().unwrap_or("").to_string(),
                        });
                    }
                }
                self.log.push_back(LogEntry::info("Session restored"));
            }
        }
        self.sync_preferences_load();
    }
}

#[cfg(test)]
mod tests {
    use super::{RightPanelSectionId, persisted_bar_zstd_level, reordered_right_panel_sections};

    #[test]
    fn persisted_bar_zstd_level_uses_saved_value() {
        let value = serde_json::json!({ "bar_zstd_level": 9 });
        assert_eq!(persisted_bar_zstd_level(&value, 3), 9);
    }

    #[test]
    fn persisted_bar_zstd_level_clamps_saved_value() {
        let high = serde_json::json!({ "bar_zstd_level": 999 });
        let low = serde_json::json!({ "bar_zstd_level": -99 });
        assert_eq!(
            persisted_bar_zstd_level(&high, 3),
            typhoon_engine::core::cache::MAX_ZSTD_LEVEL
        );
        assert_eq!(
            persisted_bar_zstd_level(&low, 3),
            typhoon_engine::core::cache::MIN_ZSTD_LEVEL
        );
    }

    #[test]
    fn persisted_bar_zstd_level_keeps_current_when_missing() {
        let value = serde_json::json!({});
        assert_eq!(persisted_bar_zstd_level(&value, 11), 11);
    }

    #[test]
    fn right_panel_reorder_moves_dragged_section_before_or_after_target() {
        use RightPanelSectionId::{
            MtfGrid, News, Orders, Positions, RecentFills, Risk, Trading, Watchlist,
        };

        let order = vec![
            Trading,
            Positions,
            RecentFills,
            Orders,
            Watchlist,
            Risk,
            News,
            MtfGrid,
        ];

        assert_eq!(
            reordered_right_panel_sections(&order, Watchlist, Positions, false).unwrap(),
            vec![
                Trading,
                Watchlist,
                Positions,
                RecentFills,
                Orders,
                Risk,
                News,
                MtfGrid
            ]
        );
        assert_eq!(
            reordered_right_panel_sections(&order, Watchlist, Positions, true).unwrap(),
            vec![
                Trading,
                Positions,
                Watchlist,
                RecentFills,
                Orders,
                Risk,
                News,
                MtfGrid
            ]
        );
        assert_eq!(
            reordered_right_panel_sections(&order, Positions, Watchlist, true).unwrap(),
            vec![
                Trading,
                RecentFills,
                Orders,
                Watchlist,
                Positions,
                Risk,
                News,
                MtfGrid
            ]
        );
    }

    #[test]
    fn right_panel_reorder_ignores_noop_and_missing_targets() {
        use RightPanelSectionId::{Positions, Trading, Watchlist};

        let order = vec![Trading, Positions];
        assert!(reordered_right_panel_sections(&order, Trading, Trading, false).is_none());
        assert!(reordered_right_panel_sections(&order, Watchlist, Trading, false).is_none());
        assert!(reordered_right_panel_sections(&order, Trading, Watchlist, false).is_none());
    }
}
