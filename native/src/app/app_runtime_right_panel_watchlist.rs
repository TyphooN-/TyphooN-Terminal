use super::*;

#[allow(deprecated)]
impl TyphooNApp {
    pub(super) fn render_right_panel_watchlist_section(&mut self, ui: &mut egui::Ui) {
        // ── Watchlist: populate from cache for symbols not yet in rows ──
        {
            let have_syms: std::collections::HashSet<&str> = self
                .watchlist_rows
                .iter()
                .filter(|r| r.last > 0.0)
                .map(|r| r.symbol.as_str())
                .collect();
            let missing: Vec<String> = self
                .user_watchlist
                .iter()
                .filter(|s| !have_syms.contains(s.as_str()))
                .cloned()
                .collect();
            if !missing.is_empty() && !self.watchlist_cache_tried {
                self.watchlist_cache_tried = true;
                if let Some(ref cache) = self.cache {
                    let primary_tf = self
                        .charts
                        .get(self.active_tab)
                        .map(|c| c.timeframe.cache_suffix().to_string())
                        .unwrap_or_else(|| "1Day".to_string());
                    let mut tfs = vec![primary_tf.clone()];
                    for fallback_tf in ["1Day", "4Hour", "1Hour"] {
                        if !tfs.iter().any(|tf| tf.eq_ignore_ascii_case(fallback_tf)) {
                            tfs.push(fallback_tf.to_string());
                        }
                    }
                    let mut rows: Vec<WatchlistRow> = self.watchlist_rows.clone();
                    let mut updated_from_cache = false;
                    for sym in &missing {
                        let mut found = false;
                        'tf_search: for tf in &tfs {
                            for source in [
                                "alpaca",
                                "kraken",
                                "kraken-equities",
                                "default",
                            ] {
                                for key in chart_source_cache_keys(source, sym, tf) {
                                    if let Ok(Some(raw)) = cache.get_bars_raw(&key) {
                                        if let Some(row) =
                                            watchlist_row_from_raw_bars(sym, &key, &raw)
                                        {
                                            rows.retain(|existing| {
                                                !existing.symbol.eq_ignore_ascii_case(sym)
                                            });
                                            rows.push(row);
                                            updated_from_cache = true;
                                            found = true;
                                            break 'tf_search;
                                        }
                                    }
                                }
                            }
                        }
                        if !found {
                            let stats = &self.bg.detailed_stats;
                            let sym_lower = sym.to_lowercase();
                            for tf in &tfs {
                                let tf_lower = tf.to_lowercase();
                                for (k, _, _) in stats {
                                    // Cache metadata keys (`<prefix>:__NAME__:…`) never
                                    // hold bar blobs — the contains-match below
                                    // could otherwise hit them for a symbol like
                                    // "HEART" or a TF equal to an account tag.
                                    if k.contains(":__") {
                                        continue;
                                    }
                                    let kl = k.to_lowercase();
                                    if kl.contains(&sym_lower) && kl.ends_with(&tf_lower) {
                                        if let Ok(Some(raw)) = cache.get_bars_raw(k) {
                                            if let Some(row) =
                                                watchlist_row_from_raw_bars(sym, k, &raw)
                                            {
                                                rows.retain(|existing| {
                                                    !existing.symbol.eq_ignore_ascii_case(sym)
                                                });
                                                rows.push(row);
                                                updated_from_cache = true;
                                                found = true;
                                                break;
                                            }
                                        }
                                    }
                                }
                                if found {
                                    break;
                                }
                            }
                        }
                    }
                    if updated_from_cache {
                        self.watchlist_rows = rows;
                        self.watchlist_last_update_ts = chrono::Utc::now().timestamp();
                    }
                }
            }
        }

        // ── Watchlist Section ─────────────────────────────────
        let wl_count = self.user_watchlist.len();
        let (wl_stale_lbl, wl_stale_col) = self.staleness_badge(self.watchlist_last_update_ts);
        let wl_header = format!("☰ Watchlist ({})  •  {}", wl_count, wl_stale_lbl);
        let watchlist_section = egui::CollapsingHeader::new(
            egui::RichText::new(wl_header)
                .strong()
                .small()
                .color(wl_stale_col),
        )
        .id_salt("watchlist_section") // stable ID — don't reset on count change
        .default_open(self.right_watchlist_open)
        .show(ui, |ui| {
            // ── Add symbol input ──────────────────────────
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let te = egui::TextEdit::singleline(&mut self.watchlist_input)
                    .desired_width(80.0)
                    .hint_text("Symbol")
                    .font(egui::TextStyle::Small)
                    .text_color(egui::Color32::WHITE);
                let te_resp = ui.add(te);
                let enter_pressed =
                    te_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if (ui
                    .add(
                        egui::Button::new(egui::RichText::new("+").color(UP).small())
                            .min_size(egui::vec2(20.0, 18.0)),
                    )
                    .clicked()
                    || enter_pressed)
                    && !self.watchlist_input.trim().is_empty()
                {
                    let sym = self.watchlist_input.trim().to_uppercase();
                    if !self.user_watchlist.contains(&sym) {
                        self.user_watchlist.push(sym);
                        self.watchlist_cache_tried = false; // retry cache lookup
                        // Trigger immediate refresh. The handler falls back to Yahoo/cache
                        // when broker snapshots are unavailable, so don't gate this on
                        // market-hours broker connectivity.
                        let _ = self.broker_tx.send(BrokerCmd::GetWatchlistQuotes {
                            symbols: self.user_watchlist.clone(),
                        });
                    }
                    self.watchlist_input.clear();
                }
            });
            ui.add_space(2.0);

            // Sort watchlist rows
            let mut sorted_wl: Vec<&WatchlistRow> = self.watchlist_rows.iter().collect();
            match self.watchlist_sort.column {
                0 => sorted_wl.sort_by(|a, b| a.symbol.cmp(&b.symbol)),
                1 => sorted_wl.sort_by(|a, b| {
                    a.last
                        .partial_cmp(&b.last)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }),
                2 => sorted_wl.sort_by(|a, b| {
                    a.change
                        .partial_cmp(&b.change)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }),
                3 => sorted_wl.sort_by(|a, b| {
                    a.change_pct
                        .partial_cmp(&b.change_pct)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }),
                4 => sorted_wl.sort_by(|a, b| {
                    a.volume
                        .partial_cmp(&b.volume)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }),
                5 => sorted_wl.sort_by(|a, b| {
                    a.ext_change_pct
                        .partial_cmp(&b.ext_change_pct)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }),
                _ => {}
            }
            if !self.watchlist_sort.ascending {
                sorted_wl.reverse();
            }

            if self.watchlist_rows.is_empty() && self.user_watchlist.is_empty() {
                ui.label(
                    egui::RichText::new("Add symbols above.")
                        .color(AXIS_TEXT)
                        .small(),
                );
            } else if self.watchlist_rows.is_empty() {
                // Call self.refresh_watchlist_fallback_prices() periodically from main loop
                // (recommended: every 5-10 minutes or when watchlist tab is opened)
                nav_muted(ui, "No cached data (Yahoo fallback available)");
                for sym in &self.user_watchlist {
                    if let Some((price, source, ts)) = self.watchlist_fallback_prices.get(sym) {
                        let age = ts.elapsed().as_secs() / 3600;
                        ui.label(
                            egui::RichText::new(format!(
                                "{} {:.2} ({} • {}h ago)",
                                sym, price, source, age
                            ))
                            .small()
                            .monospace(),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new(sym)
                                .color(egui::Color32::from_rgb(100, 100, 110))
                                .small()
                                .monospace(),
                        );
                    }
                }
            } else {
                let mut load_key: Option<String> = None;
                let mut remove_sym: Option<String> = None;
                let mut open_new_sym: Option<String> = None;
                let mut move_up_sym: Option<String> = None;
                let mut move_down_sym: Option<String> = None;
                let mut move_top_sym: Option<String> = None;
                let row_h = 18.0_f32;
                let font = egui::FontId::monospace(10.0);
                let hdr_font = egui::FontId::monospace(9.0);
                let avail_w = ui.available_width();

                // Column layout: Symbol | Last | Chg | Chg% | Ext% | Vol | + | x
                let col_last = avail_w * 0.26;
                let col_chg = avail_w * 0.42;
                let col_pct = avail_w * 0.56;
                // Hide Exts column during CORE session to avoid wasting space
                // (only useful in PRE/POST for xStocks/equities)
                let is_core = self.charts.iter().any(|ch| ch.session_label.contains("CORE"));
                let col_ext = if is_core { 0.0 } else { avail_w * 0.70 }; // Extended hours change%
                let col_vol = avail_w * 0.82;
                let col_x = avail_w - 12.0;
                let col_plus = avail_w - 28.0; // "+" button (open new chart)

                // Sortable header row
                let (hdr_rect, hdr_resp) =
                    ui.allocate_exact_size(egui::vec2(avail_w, row_h), egui::Sense::click());
                let hp = ui.painter_at(hdr_rect);
                let hy = hdr_rect.center().y;
                let hdr_col = egui::Color32::from_rgb(120, 120, 140);
                let sort_arrow = |col: usize| -> &str {
                    if self.watchlist_sort.column == col {
                        if self.watchlist_sort.ascending {
                            " \u{25B2}"
                        } else {
                            " \u{25BC}"
                        }
                    } else {
                        ""
                    }
                };
                hp.text(
                    egui::pos2(hdr_rect.left() + 2.0, hy),
                    egui::Align2::LEFT_CENTER,
                    &format!("Symbol{}", sort_arrow(0)),
                    hdr_font.clone(),
                    hdr_col,
                );
                hp.text(
                    egui::pos2(hdr_rect.left() + col_last - 2.0, hy),
                    egui::Align2::RIGHT_CENTER,
                    &format!("Last{}", sort_arrow(1)),
                    hdr_font.clone(),
                    hdr_col,
                );
                hp.text(
                    egui::pos2(hdr_rect.left() + col_chg - 2.0, hy),
                    egui::Align2::RIGHT_CENTER,
                    &format!("Chg{}", sort_arrow(2)),
                    hdr_font.clone(),
                    hdr_col,
                );
                hp.text(
                    egui::pos2(hdr_rect.left() + col_pct - 2.0, hy),
                    egui::Align2::RIGHT_CENTER,
                    &format!("Chg%{}", sort_arrow(3)),
                    hdr_font.clone(),
                    hdr_col,
                );
                hp.text(
                    egui::pos2(hdr_rect.left() + col_ext - 2.0, hy),
                    egui::Align2::RIGHT_CENTER,
                    &format!("Ext%{}", sort_arrow(5)),
                    hdr_font.clone(),
                    hdr_col,
                );
                hp.text(
                    egui::pos2(hdr_rect.left() + col_vol - 2.0, hy),
                    egui::Align2::RIGHT_CENTER,
                    &format!("Vol{}", sort_arrow(4)),
                    hdr_font.clone(),
                    hdr_col,
                );
                // Click header to sort
                if hdr_resp.clicked() {
                    if let Some(pos) = hdr_resp.interact_pointer_pos() {
                        let rx = pos.x - hdr_rect.left();
                        let col = if rx < col_last * 0.5 {
                            0
                        } else if rx < (col_last + col_chg) * 0.5 {
                            1
                        } else if rx < (col_chg + col_pct) * 0.5 {
                            2
                        } else if rx < (col_pct + col_ext) * 0.5 {
                            3
                        } else if rx < (col_ext + col_vol) * 0.5 {
                            5
                        } else {
                            4
                        };
                        self.watchlist_sort.toggle(col);
                    }
                }
                // Separator
                let sep_y = hdr_rect.bottom();
                ui.painter().line_segment(
                    [
                        egui::pos2(hdr_rect.left(), sep_y),
                        egui::pos2(hdr_rect.right(), sep_y),
                    ],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 40, 55)),
                );

                // Data rows
                for (idx, wl) in sorted_wl.iter().enumerate() {
                    let sym_color = WL_COLORS[idx % WL_COLORS.len()];
                    let chg_color = if wl.change >= 0.0 { UP } else { DOWN };
                    let is_selected = self
                        .charts
                        .get(self.active_tab)
                        .map(|c| c.symbol == wl.cache_key || c.symbol.contains(&wl.symbol))
                        .unwrap_or(false);

                    let (row_rect, row_resp) =
                        ui.allocate_exact_size(egui::vec2(avail_w, row_h), egui::Sense::click());
                    let rp = ui.painter_at(row_rect);

                    // ADR-092: Row background with P&L heatmap intensity
                    let heat = (wl.change_pct.abs() * 8.0).min(40.0) as u8;
                    let row_bg = if is_selected {
                        egui::Color32::from_rgb(15, 25, 45)
                    } else if heat > 0 {
                        if wl.change_pct >= 0.0 {
                            egui::Color32::from_rgb(0, heat / 2, 0)
                        } else {
                            egui::Color32::from_rgb(heat / 2, 0, 0)
                        }
                    } else if idx % 2 == 1 {
                        egui::Color32::from_rgb(8, 8, 14)
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    rp.rect_filled(row_rect, 0.0, row_bg);

                    let ry = row_rect.center().y;
                    let rx = row_rect.left();

                    // Symbol with colored dot
                    rp.text(
                        egui::pos2(rx + 2.0, ry),
                        egui::Align2::LEFT_CENTER,
                        "\u{25CF}",
                        font.clone(),
                        sym_color,
                    );
                    rp.text(
                        egui::pos2(rx + 14.0, ry),
                        egui::Align2::LEFT_CENTER,
                        &wl.symbol,
                        font.clone(),
                        egui::Color32::WHITE,
                    );

                    // Last / Change / Change% — show extended hours price if available
                    let (disp_last, disp_chg, disp_pct, disp_color) = if wl.ext_change_pct.abs()
                        > 0.001
                        && wl.prev_close > 0.0
                        && wl.last > 0.0
                    {
                        // row.last is already the actual extended-hours
                        // price when ext_change_pct is populated. Do not
                        // reconstruct it from prev_close: Yahoo reports
                        // ext_change_pct versus the regular close/price,
                        // not necessarily versus previous close.
                        let ext_chg = wl.last - wl.prev_close;
                        let ext_pct = (wl.last / wl.prev_close - 1.0) * 100.0;
                        let c = if ext_chg >= 0.0 { UP } else { DOWN };
                        (wl.last, ext_chg, ext_pct, c)
                    } else {
                        (wl.last, wl.change, wl.change_pct, chg_color)
                    };
                    rp.text(
                        egui::pos2(rx + col_last - 2.0, ry),
                        egui::Align2::RIGHT_CENTER,
                        &format_price(disp_last),
                        font.clone(),
                        egui::Color32::WHITE,
                    );

                    let chg_str = if disp_chg >= 0.0 {
                        format_price(disp_chg)
                    } else {
                        format!("-{}", format_price(disp_chg.abs()))
                    };
                    rp.text(
                        egui::pos2(rx + col_chg - 2.0, ry),
                        egui::Align2::RIGHT_CENTER,
                        &chg_str,
                        font.clone(),
                        disp_color,
                    );

                    rp.text(
                        egui::pos2(rx + col_pct - 2.0, ry),
                        egui::Align2::RIGHT_CENTER,
                        &format!("{:.2}%", disp_pct),
                        font.clone(),
                        disp_color,
                    );

                    // Extended hours change % (right-aligned, colored, dimmed if zero)
                    if wl.ext_change_pct.abs() > 0.001 {
                        let ext_color = if wl.ext_change_pct >= 0.0 { UP } else { DOWN };
                        rp.text(
                            egui::pos2(rx + col_ext - 2.0, ry),
                            egui::Align2::RIGHT_CENTER,
                            &format!("{:+.2}%", wl.ext_change_pct),
                            font.clone(),
                            ext_color,
                        );
                    } else {
                        rp.text(
                            egui::pos2(rx + col_ext - 2.0, ry),
                            egui::Align2::RIGHT_CENTER,
                            "-",
                            font.clone(),
                            egui::Color32::from_rgb(60, 60, 70),
                        );
                    }

                    // Volume (right-aligned, dimmed)
                    let vol_str = if wl.volume >= 1_000_000.0 {
                        format!("{:.2}M", wl.volume / 1_000_000.0)
                    } else if wl.volume >= 1_000.0 {
                        format!("{:.1}K", wl.volume / 1_000.0)
                    } else {
                        format!("{:.0}", wl.volume)
                    };
                    rp.text(
                        egui::pos2(rx + col_vol - 2.0, ry),
                        egui::Align2::RIGHT_CENTER,
                        &vol_str,
                        font.clone(),
                        AXIS_TEXT,
                    );

                    // "+" button (open new chart tab)
                    rp.text(
                        egui::pos2(rx + col_plus, ry),
                        egui::Align2::CENTER_CENTER,
                        "+",
                        egui::FontId::monospace(10.0),
                        egui::Color32::from_rgb(80, 180, 80),
                    );
                    // Remove button (x)
                    rp.text(
                        egui::pos2(rx + col_x, ry),
                        egui::Align2::CENTER_CENTER,
                        "x",
                        egui::FontId::monospace(9.0),
                        egui::Color32::from_rgb(100, 50, 50),
                    );

                    // Interactions
                    if row_resp.clicked() {
                        if let Some(pos) = row_resp.interact_pointer_pos() {
                            let rel_x = pos.x - rx;
                            if rel_x >= col_x - 8.0 {
                                remove_sym = Some(wl.symbol.clone()); // clicked x
                            } else if rel_x >= col_plus - 8.0 && rel_x < col_plus + 8.0 {
                                open_new_sym = Some(wl.symbol.clone()); // clicked +
                            } else {
                                load_key = Some(wl.cache_key.clone()); // clicked row
                            }
                        }
                    }
                    row_resp.context_menu(|ui| {
                        if ui.button(format!("Chart {}", wl.symbol)).clicked() {
                            load_key = Some(wl.cache_key.clone());
                            ui.close();
                        }
                        if ui.button("View fundamentals").clicked() {
                            self.show_fundamentals = true;
                            ui.close();
                        }
                        if ui.button("View SEC filings").clicked() {
                            self.show_sec = true;
                            self.sec_search_query = wl.symbol.clone();
                            ui.close();
                        }
                        if ui.button("View insider trades").clicked() {
                            self.show_insider = true;
                            ui.close();
                        }
                        ui.separator();
                        if ui.button(format!("Move Up  {}", wl.symbol)).clicked() {
                            move_up_sym = Some(wl.symbol.clone());
                            ui.close();
                        }
                        if ui.button(format!("Move Down  {}", wl.symbol)).clicked() {
                            move_down_sym = Some(wl.symbol.clone());
                            ui.close();
                        }
                        if ui.button(format!("Move to Top  {}", wl.symbol)).clicked() {
                            move_top_sym = Some(wl.symbol.clone());
                            ui.close();
                        }
                        if ui.button(format!("Remove {}", wl.symbol)).clicked() {
                            remove_sym = Some(wl.symbol.clone());
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Command Palette…").clicked() {
                            self.palette_context = PaletteContext::Watchlist;
                            self.command_open = true;
                            self.command_input.clear();
                            ui.close();
                        }
                    });
                }
                // Handle reorder — one-step neighbour swap or jump-to-top
                if let Some(ref sym) = move_up_sym {
                    if let Some(idx) = self.user_watchlist.iter().position(|s| s == sym) {
                        if idx > 0 {
                            self.user_watchlist.swap(idx, idx - 1);
                            // Also reorder the displayed rows
                            if let Some(row_idx) =
                                self.watchlist_rows.iter().position(|r| &r.symbol == sym)
                            {
                                self.watchlist_rows.swap(row_idx, row_idx - 1);
                            }
                        }
                    }
                }
                if let Some(ref sym) = move_down_sym {
                    if let Some(idx) = self.user_watchlist.iter().position(|s| s == sym) {
                        if idx + 1 < self.user_watchlist.len() {
                            self.user_watchlist.swap(idx, idx + 1);
                            if let Some(row_idx) =
                                self.watchlist_rows.iter().position(|r| &r.symbol == sym)
                            {
                                self.watchlist_rows.swap(row_idx, row_idx + 1);
                            }
                        }
                    }
                }
                if let Some(ref sym) = move_top_sym {
                    if let Some(idx) = self.user_watchlist.iter().position(|s| s == sym) {
                        if idx > 0 {
                            let item = self.user_watchlist.remove(idx);
                            self.user_watchlist.insert(0, item);
                            if let Some(row_idx) =
                                self.watchlist_rows.iter().position(|r| &r.symbol == sym)
                            {
                                let row = self.watchlist_rows.remove(row_idx);
                                self.watchlist_rows.insert(0, row);
                            }
                        }
                    }
                }
                // Handle remove
                if let Some(ref sym) = remove_sym {
                    self.user_watchlist.retain(|s| s != sym);
                    self.watchlist_rows.retain(|r| &r.symbol != sym);
                }
                // Handle + button → open new chart tab
                if let Some(sym) = open_new_sym {
                    self.deferred_symbol_action = SymbolAction::OpenChart(sym);
                }
                // Handle load
                if let Some(key) = load_key {
                    // First try loading from cache
                    let mut loaded = false;
                    if let Some(ref cache) = self.cache {
                        if let Some(chart) = self.charts.get_mut(self.active_tab) {
                            match cache.get_bars_raw(&key) {
                                Ok(Some(raw)) if !raw.is_empty() => {
                                    chart.bars = raw
                                        .into_iter()
                                        .map(|(ts, o, h, l, c, v)| Bar {
                                            ts_ms: ts,
                                            open: o,
                                            high: h,
                                            low: l,
                                            close: c,
                                            volume: v,
                                        })
                                        .collect();
                                    chart.view_offset =
                                        chart.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
                                    chart.symbol = bare_symbol_from_key(&key);
                                    chart.compute_indicators();
                                    self.log.push_back(LogEntry::info(format!(
                                        "Loaded {} bars from {}",
                                        chart.bars.len(),
                                        key
                                    )));
                                    loaded = true;
                                }
                                Ok(_) => {
                                    // Key not found or empty — will try Alpaca below
                                }
                                Err(e) => {
                                    self.log
                                        .push_back(LogEntry::err(format!("Load error: {}", e)));
                                }
                            }
                        }
                    }
                    // Fetch from Alpaca if no cached data and broker is connected
                    if !loaded && self.broker_connected {
                        let tf = self
                            .charts
                            .get(self.active_tab)
                            .map(|c| c.timeframe.cache_suffix().to_string())
                            .unwrap_or_else(|| "1Day".to_string());
                        if self.sync_timeframe_enabled(&tf) {
                            self.queue_alpaca_fetch(&key, &tf);
                            self.log.push_back(LogEntry::info(format!(
                                "Fetching {} from Alpaca...",
                                key
                            )));
                        } else {
                            self.log.push_back(LogEntry::warn(format!(
                                "Skipped {} fetch — sync for {} is disabled",
                                key,
                                sync_timeframe_short_label(&tf)
                            )));
                        }
                    }
                }
            }
        });
        self.right_watchlist_open = watchlist_section.fully_open();
        self.handle_right_panel_section_drag(
            ui,
            RightPanelSectionId::Watchlist,
            &watchlist_section.header_response,
        );
    }
}
