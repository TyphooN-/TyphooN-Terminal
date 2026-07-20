use super::*;

pub(super) fn reordered_right_panel_sections(
    order: &[RightPanelSectionId],
    dragged: RightPanelSectionId,
    target: RightPanelSectionId,
    after_target: bool,
) -> Option<Vec<RightPanelSectionId>> {
    if dragged == target {
        return None;
    }
    let mut next = order.to_vec();
    let mut index_by_section = [None; RightPanelSectionId::DEFAULT_ORDER.len()];
    for (idx, section) in next.iter().copied().enumerate() {
        index_by_section[section as usize] = Some(idx);
    }
    let from = index_by_section[dragged as usize]?;
    let target_idx = index_by_section[target as usize]?;
    let item = next.remove(from);
    let mut to = if from < target_idx {
        target_idx.saturating_sub(1)
    } else {
        target_idx
    };
    if after_target {
        to += 1;
    }
    next.insert(to.min(next.len()), item);
    Some(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reordered_right_panel_sections_preserves_drop_semantics() {
        let order = RightPanelSectionId::DEFAULT_ORDER;
        let moved_after = reordered_right_panel_sections(
            &order,
            RightPanelSectionId::Trading,
            RightPanelSectionId::Orders,
            true,
        )
        .unwrap();
        assert_eq!(moved_after[2], RightPanelSectionId::Orders);
        assert_eq!(moved_after[3], RightPanelSectionId::Trading);

        let moved_before = reordered_right_panel_sections(
            &order,
            RightPanelSectionId::MtfGrid,
            RightPanelSectionId::Positions,
            false,
        )
        .unwrap();
        assert_eq!(moved_before[1], RightPanelSectionId::MtfGrid);
        assert_eq!(moved_before[2], RightPanelSectionId::Positions);
    }
}

impl TyphooNApp {
    pub(in crate::app) fn refill_market_data_sync_slots(&mut self) {
        let pending_cap = if self.full_tilt_sync_enabled() {
            KRAKEN_SPOT_FULL_TILT_QUEUE_WINDOW
                + KRAKEN_EQUITIES_FULL_TILT_QUEUE_WINDOW
                + KRAKEN_FUTURES_FULL_TILT_QUEUE_WINDOW
                + ALPACA_FULL_TILT_QUEUE_WINDOW
                + YAHOO_CHART_FULL_TILT_QUEUE_WINDOW
        } else {
            KRAKEN_SPOT_QUEUE_WINDOW
                + KRAKEN_FUTURES_QUEUE_WINDOW
                + 96 // Kraken Equities native demand repair lane
                + 64 // Alpaca assist/broad lane
                + YAHOO_CHART_QUEUE_WINDOW
        };
        if self.total_pending_market_data_fetches() > pending_cap {
            return;
        }
        if !self.cache_loaded {
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
            let equity_syms = self.alpaca_equity_rotation_symbols_cached();
            let _ = self.schedule_alpaca_pairs(&equity_syms);
        }
    }

    pub(in crate::app) fn normalized_right_panel_order(&mut self) -> Vec<RightPanelSectionId> {
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

    pub(in crate::app) fn move_right_panel_section(
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

    pub(in crate::app) fn handle_right_panel_section_drag(
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

    pub(in crate::app) fn render_sync_timeframe_controls(
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
                    .on_hover_text(format!(
                        "{} automated scrape/sync, including new Kraken WS OHLC subscriptions",
                        cache
                    ))
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
                "Unchecked TFs are skipped by automated bar sync/backfill and new Kraken WS OHLC subscriptions.",
            )
            .color(AXIS_TEXT)
            .small(),
        );
    }

    pub(in crate::app) fn render_alpaca_sync_profile_controls(
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
}
