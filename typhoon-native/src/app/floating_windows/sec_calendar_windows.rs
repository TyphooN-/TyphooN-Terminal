use super::*;
use crate::app::app_runtime_support::should_start_manual_background_scope_scrape;

/// Group an integer-valued amount with thousands separators (`707811.0` →
/// `"707,811"`). Used by the structured Form 4 viewer for share counts / values.
fn fmt_int_commas(n: f64) -> String {
    let v = n.round() as i64;
    let digits = v.unsigned_abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    let len = digits.len();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if v < 0 { format!("-{out}") } else { out }
}

impl TyphooNApp {
    pub(super) fn render_sec_calendar_windows(&mut self, ctx: &egui::Context) {
        // SEC Filing Scanner — tabbed: Filings | Alerts | Insiders | Timeline
        if self.show_sec {
            // PERF: rebuild all SEC caches (filings, insiders, timeline, tab counts)
            // once per frame before rendering. Steady state = zero O(N) work in the
            // render closure; caches only invalidate when bg data, scope, filters,
            // search query, or sort direction change.
            self.rebuild_sec_caches();
            let sec_scope_label = self.broker_scope_label();
            let mut sec_pending_action = SymbolAction::None;
            let mut sec_scrape_clicked = false;
            egui::Window::new("SEC Filing Scanner")
                .open(&mut self.show_sec)
                .resizable(true)
                .default_size([900.0, 650.0])
                .min_size([600.0, 200.0])
                .constrain(true)
                .scroll([false, true])
                .show(ctx, |ui| {
                            let sec_high = egui::Color32::from_rgb(231, 76, 60);
                            let sec_med = egui::Color32::from_rgb(241, 196, 15);
                            let sec_low = egui::Color32::from_rgb(100, 100, 120);
                            let sec_cyan = egui::Color32::from_rgb(26, 188, 156);
                            let sec_blue = egui::Color32::from_rgb(100, 200, 255);
                            let sec_purple = egui::Color32::from_rgb(200, 100, 255);
                            let sec_orange = egui::Color32::from_rgb(255, 130, 60);

                            // ── Tab bar + scrape button + scope ──
                            ui.horizontal(|ui| {
                                let (scoped_count, alert_count, insider_count) = self.sec_cache_tab_counts;
                                if ui.selectable_label(self.sec_tab == 0, egui::RichText::new(format!("Filings ({})", scoped_count)).small()).clicked() { self.sec_tab = 0; }
                                if ui.selectable_label(self.sec_tab == 1, egui::RichText::new(format!("Alerts ({})", alert_count)).small()).clicked() { self.sec_tab = 1; }
                                if ui.selectable_label(self.sec_tab == 2, egui::RichText::new(format!("Insiders ({})", insider_count)).small()).clicked() { self.sec_tab = 2; }
                                if ui.selectable_label(self.sec_tab == 3, egui::RichText::new("Timeline").small()).clicked() { self.sec_tab = 3; }
                                ui.separator();
                                let labels = ["4", "13F", "14A", "S-1", "10-K", "10-Q", "8-K"];
                                for (i, label) in labels.iter().enumerate() {
                                    let prev = self.sec_filters[i];
                                    ui.checkbox(&mut self.sec_filters[i], egui::RichText::new(*label).small());
                                    if self.sec_filters[i] != prev { self.sec_page = 0; self.sec_selected_filing = None; }
                                }
                                ui.separator();
                                if ui
                                    .add_enabled(
                                        !self.scrape_sec_running,
                                        egui::Button::new(
                                            egui::RichText::new(if self.scrape_sec_running {
                                                "Scraping..."
                                            } else {
                                                "Scrape Now"
                                            })
                                            .color(BTN_GREEN_TEXT)
                                            .small(),
                                        )
                                        .fill(BTN_GREEN),
                                    )
                                    .on_hover_text("Scrape SEC EDGAR filings for the current Scope")
                                    .clicked()
                                {
                                    sec_scrape_clicked = true;
                                }
                                if self.scrape_sec_running {
                                    ui.spinner();
                                }
                                ui.separator();
                                let (total_filings, indexed_content) = self.bg.sec_content_stats;
                                ui.label(egui::RichText::new(format!("[{}] {}/{} indexed", sec_scope_label, indexed_content, total_filings)).color(AXIS_TEXT).small());
                            });
                            // ── Search box ──
                            ui.horizontal(|ui| {
                                let search_resp = ui.add(egui::TextEdit::singleline(&mut self.sec_search_query).desired_width(300.0).hint_text("Search: ticker / company / sector / industry").font(egui::TextStyle::Small));
                                if ui.small_button("X").clicked() {
                                    self.sec_search_query.clear();
                                    self.sec_page = 0;
                                }
                                if search_resp.changed() { self.sec_page = 0; }
                            });
                            ui.separator();

                            if self.sec_tab == 0 {
                                // ═══════════ FILINGS TAB (full height) ═══════════
                                // PERF: pull pre-filtered/sorted indices from cache. Cache is
                                // rebuilt by rebuild_sec_caches() only when state changes.
                                let filings = &self.bg.sec_filings;
                                let idxs = &self.sec_cache_filings;

                                // Detail panel at top (if a filing is selected)
                                if let Some(sel) = self.sec_selected_filing {
                                    if let Some(f) = idxs.get(sel).and_then(|&i| filings.get(i)) {
                                        egui::Frame::NONE
                                            .fill(egui::Color32::from_rgb(15, 18, 30))
                                            .inner_margin(8.0)
                                            .corner_radius(4.0)
                                            .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new(format!("{} — {} — {}", f.ticker, f.form_type, f.filing_date)).heading().color(sec_cyan));
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    if ui.small_button("Close").clicked() { self.sec_selected_filing = None; }
                                                });
                                            });
                                            ui.add_space(4.0);
                                            egui::Grid::new("sec_detail").num_columns(2).min_col_width(110.0).spacing([8.0, 2.0]).show(ui, |ui| {
                                                ui.label(egui::RichText::new("Company").color(sec_low)); ui.label(egui::RichText::new(&f.company_name).strong()); ui.end_row();
                                                ui.label(egui::RichText::new("Form Type").color(sec_low)); ui.label(&f.form_type); ui.end_row();
                                                ui.label(egui::RichText::new("Category").color(sec_low)); ui.label(&f.category); ui.end_row();
                                                ui.label(egui::RichText::new("Accession #").color(sec_low)); ui.label(egui::RichText::new(&f.accession_number).monospace().color(sec_blue)); ui.end_row();
                                                let sc = if f.importance_score >= 80 { sec_high } else if f.importance_score >= 50 { sec_med } else { sec_low };
                                                ui.label(egui::RichText::new("Importance").color(sec_low)); ui.label(egui::RichText::new(format!("{}/100", f.importance_score)).color(sc).strong()); ui.end_row();
                                                if !f.url.is_empty() {
                                                    ui.label(egui::RichText::new("EDGAR").color(sec_low));
                                                    ui.horizontal(|ui| {
                                                        ui.label(egui::RichText::new(&f.url).small().color(sec_blue));
                                                        if ui.small_button("View Document").clicked() {
                                                            self.sec_filing_content.clear();
                                                            self.sec_filing_content_for = f.accession_number.clone();
                                                            self.sec_filing_summary = None;
                                                            self.sec_filing_summary_for.clear();
                                                            // Try DB cache first — avoid re-hitting EDGAR if already stored.
                                                            let mut served_from_cache = false;
                                                            if let Some(ref cache) = self.cache {
                                                                if let Ok(conn) = cache.connection() {
                                                                    if let Ok(Some(text)) = sec_filing::get_filing_content(&conn, &f.accession_number) {
                                                                        // polish_filing_text also cleans up legacy
                                                                        // cached blobs stored by older builds that
                                                                        // left numeric HTML entities un-decoded.
                                                                        self.sec_filing_content = sec_filing::polish_filing_text(&text);
                                                                        self.sec_filing_loading = false;
                                                                        served_from_cache = true;
                                                                    }
                                                                }
                                                            }
                                                            if !served_from_cache {
                                                                self.sec_filing_loading = true;
                                                                let _ = self.broker_tx.send(BrokerCmd::FetchFilingContent { url: f.url.clone() });
                                                            }
                                                        }
                                                        let pin_label = if self.sec_filing_pinned { "[unpin]" } else { "[pin]" };
                                                        if ui.small_button(pin_label).clicked() {
                                                            self.sec_filing_pinned = !self.sec_filing_pinned;
                                                        }
                                                    }); ui.end_row();
                                                }
                                                if !f.summary.is_empty() {
                                                    ui.label(egui::RichText::new("Summary").color(sec_low));
                                                    ui.label(&f.summary); ui.end_row();
                                                }
                                                if f.insider_flag {
                                                    ui.label(egui::RichText::new("Insider").color(sec_low));
                                                    ui.label(egui::RichText::new("Yes — insider transaction").color(sec_med)); ui.end_row();
                                                }
                                            });
                                            // ── Structured Form 4 insider-transaction view ──
                                            // The raw EDGAR Form 4 document is XSLT table HTML that
                                            // strips into unreadable pipe-soup, so render the parsed
                                            // transactions (issuer-side data) as a clean table instead.
                                            if matches!(f.form_type.as_str(), "4" | "4/A") {
                                                let txns: Vec<sec_filing::InsiderTrade> = self
                                                    .bg
                                                    .insider_trades
                                                    .get(&f.ticker)
                                                    .map(|v| {
                                                        v.iter()
                                                            .filter(|t| t.accession_number == f.accession_number)
                                                            .cloned()
                                                            .collect()
                                                    })
                                                    .unwrap_or_default();
                                                ui.separator();
                                                ui.label(egui::RichText::new("Insider Transactions").color(sec_med).strong());
                                                if txns.is_empty() {
                                                    ui.label(egui::RichText::new("No parsed transactions for this filing yet (Form 4 parse pending, or a holdings-only amendment).").small().color(sec_low));
                                                } else {
                                                    if let Some(first) = txns.first() {
                                                        let role = if first.is_officer && first.is_director {
                                                            "Officer & Director"
                                                        } else if first.is_officer {
                                                            "Officer"
                                                        } else if first.is_director {
                                                            "Director"
                                                        } else {
                                                            "Insider"
                                                        };
                                                        let who = if first.insider_title.trim().is_empty() {
                                                            format!("{} — {}", first.insider_name, role)
                                                        } else {
                                                            format!("{} — {} ({})", first.insider_name, first.insider_title, role)
                                                        };
                                                        ui.label(egui::RichText::new(who).color(sec_cyan).small().strong());
                                                    }
                                                    egui::Grid::new("sec_form4_txns").striped(true).num_columns(6).min_col_width(52.0).show(ui, |ui| {
                                                        for h in ["Date", "Code", "Type", "Shares", "Price", "Value"] {
                                                            ui.label(egui::RichText::new(h).color(AXIS_TEXT).small().strong());
                                                        }
                                                        ui.end_row();
                                                        for t in &txns {
                                                            let (desc, dir) = sec_filing::form4_transaction_code_label(&t.transaction_type);
                                                            let col = match dir {
                                                                1 => egui::Color32::from_rgb(26, 188, 156),
                                                                -1 => egui::Color32::from_rgb(231, 76, 60),
                                                                _ => AXIS_TEXT,
                                                            };
                                                            ui.label(egui::RichText::new(&t.transaction_date).small().monospace());
                                                            ui.label(egui::RichText::new(&t.transaction_type).color(col).small().strong().monospace());
                                                            ui.label(egui::RichText::new(desc).color(col).small());
                                                            ui.label(egui::RichText::new(fmt_int_commas(t.shares)).small().monospace());
                                                            ui.label(egui::RichText::new(if t.price > 0.0 { format!("${:.2}", t.price) } else { "—".to_string() }).small().monospace());
                                                            ui.label(egui::RichText::new(if t.aggregate_value > 0.0 { format!("${}", fmt_int_commas(t.aggregate_value)) } else { "—".to_string() }).small().monospace());
                                                            ui.end_row();
                                                        }
                                                    });
                                                }
                                            }
                                            // In-window document viewer (sticky if pinned or accession matches selected)
                                            let show_doc = self.sec_filing_pinned
                                                || self.sec_filing_content_for == f.accession_number;
                                            if self.sec_filing_loading {
                                                ui.label(egui::RichText::new("Loading filing document...").color(sec_blue));
                                            } else if !self.sec_filing_content.is_empty() && show_doc {
                                                ui.separator();
                                                // Lazy-compute heuristic summary, keyed by accession so navigating refreshes it.
                                                if self.sec_filing_summary_for != self.sec_filing_content_for {
                                                    self.sec_filing_summary = Some(sec_filing::summarize_filing(
                                                        &f.form_type,
                                                        &self.sec_filing_content,
                                                    ));
                                                    self.sec_filing_summary_for = self.sec_filing_content_for.clone();
                                                }
                                                if let Some(summary) = self.sec_filing_summary.clone() {
                                                    ui.label(egui::RichText::new(&summary.headline).color(sec_med).strong());
                                                    if !summary.bullets.is_empty() {
                                                        egui::CollapsingHeader::new(egui::RichText::new("Summary bullets").small().strong())
                                                            .id_salt("sec_summary_bullets")
                                                            .default_open(true)
                                                            .show(ui, |ui| {
                                                                for b in &summary.bullets {
                                                                    ui.label(egui::RichText::new(format!("\u{2022} {}", b)).small().color(egui::Color32::from_rgb(210, 210, 220)));
                                                                }
                                                            });
                                                    }
                                                    if !summary.sections.is_empty() {
                                                        egui::CollapsingHeader::new(egui::RichText::new("Extracted sections").small().strong())
                                                            .id_salt("sec_summary_sections")
                                                            .default_open(false)
                                                            .show(ui, |ui| {
                                                                for section in &summary.sections {
                                                                    ui.label(egui::RichText::new(&section.title).color(sec_blue).strong().small());
                                                                    ui.label(egui::RichText::new(&section.body).small().color(egui::Color32::from_rgb(200, 200, 210)));
                                                                    ui.add_space(4.0);
                                                                }
                                                            });
                                                    }
                                                    ui.separator();
                                                }
                                                let header = if self.sec_filing_pinned && self.sec_filing_content_for != f.accession_number {
                                                    format!("Raw filing document (pinned: {})", self.sec_filing_content_for)
                                                } else { "Raw filing document".to_string() };
                                                // Form 4 raw text is mangled XSLT pipe-soup and the
                                                // structured table above already carries the data, so
                                                // default-collapse it there; keep it open for prose forms.
                                                let raw_open = !matches!(f.form_type.as_str(), "4" | "4/A");
                                                egui::CollapsingHeader::new(egui::RichText::new(header).small().strong())
                                                    .id_salt("sec_doc_collapse")
                                                    .default_open(raw_open)
                                                    .show(ui, |ui| {
                                                        let doc_h = ui.available_height().max(150.0);
                                                        egui::ScrollArea::vertical().id_salt("sec_doc_viewer").max_height(doc_h).auto_shrink(false).show(ui, |ui| {
                                                            ui.label(egui::RichText::new(&self.sec_filing_content).small().monospace().color(egui::Color32::from_rgb(190, 190, 200)));
                                                        });
                                                    });
                                            }
                                        });
                                        ui.add_space(4.0);
                                    }
                                }

                                // Pagination
                                let page_size = 100;
                                let total = idxs.len();
                                let total_pages = (total + page_size - 1) / page_size;
                                if self.sec_page >= total_pages && total_pages > 0 { self.sec_page = total_pages - 1; }
                                let page_start = self.sec_page * page_size;
                                let page_end = (page_start + page_size).min(total);
                                let page_slice = &idxs[page_start..page_end];

                                // Pagination controls
                                if total_pages > 1 {
                                    ui.horizontal(|ui| {
                                        if ui.add_enabled(self.sec_page > 0, egui::Button::new(egui::RichText::new("◀ Prev").small())).clicked() {
                                            self.sec_page = self.sec_page.saturating_sub(1);
                                            self.sec_selected_filing = None;
                                        }
                                        ui.label(egui::RichText::new(format!("Page {} / {}  ({} filings)", self.sec_page + 1, total_pages, total)).small().color(sec_low));
                                        if ui.add_enabled(self.sec_page + 1 < total_pages, egui::Button::new(egui::RichText::new("Next ▶").small())).clicked() {
                                            self.sec_page += 1;
                                            self.sec_selected_filing = None;
                                        }
                                    });
                                    ui.separator();
                                }

                                // Filing table (scrollable, fill remaining height).
                                // PERF: the symbol → (sector, industry) lookup was rebuilt over all
                                // ~12k fundamentals every frame — the recurring ~250ms `sec_calendar`
                                // stall. Cache it behind `bg_rev` and hand the grid closure a cheap
                                // Rc clone (it can't borrow `self`, which the closure mutates).
                                if self.sec_fund_sector_rev != Some(self.bg_rev) {
                                    self.sec_fund_sector_map = std::rc::Rc::new(
                                        self.bg
                                            .all_fundamentals
                                            .iter()
                                            .map(|f| (f.symbol.clone(), (f.sector.clone(), f.industry.clone())))
                                            .collect(),
                                    );
                                    self.sec_fund_sector_rev = Some(self.bg_rev);
                                }
                                let sec_fund_map = self.sec_fund_sector_map.clone();
                                let avail = ui.available_height().max(200.0);
                                egui::ScrollArea::vertical().id_salt("sec_filings_tab").min_scrolled_height(avail).auto_shrink(false).show(ui, |ui| {
                                    if idxs.is_empty() {
                                        ui.label(egui::RichText::new("No filings. Click Scrape Now to fetch from SEC EDGAR.").color(sec_low));
                                    } else {
                                        egui::Grid::new("sec_filings_grid").striped(true).num_columns(8).min_col_width(45.0).show(ui, |ui| {
                                            if SortState::header(ui, "Date", 0, &self.sec_sort) { self.sec_sort.toggle(0); }
                                            if SortState::header(ui, "Symbol", 1, &self.sec_sort) { self.sec_sort.toggle(1); }
                                            if SortState::header(ui, "Type", 2, &self.sec_sort) { self.sec_sort.toggle(2); }
                                            if SortState::header(ui, "Category", 3, &self.sec_sort) { self.sec_sort.toggle(3); }
                                            if SortState::header(ui, "Sector", 6, &self.sec_sort) { self.sec_sort.toggle(6); }
                                            if SortState::header(ui, "Industry", 7, &self.sec_sort) { self.sec_sort.toggle(7); }
                                            if SortState::header(ui, "Company", 4, &self.sec_sort) { self.sec_sort.toggle(4); }
                                            if SortState::header(ui, "Accession #", 5, &self.sec_sort) { self.sec_sort.toggle(5); }
                                            ui.end_row();
                                            for (local_idx, &fidx) in page_slice.iter().enumerate() {
                                                let f = &filings[fidx];
                                                let global_idx = page_start + local_idx;
                                                let sel = self.sec_selected_filing == Some(global_idx);
                                                let rc = if sel { egui::Color32::WHITE } else { egui::Color32::from_rgb(180, 180, 190) };
                                                if ui.add(egui::Label::new(egui::RichText::new(&f.filing_date).small().color(rc)).sense(egui::Sense::click())).clicked() { self.sec_selected_filing = if sel { None } else { Some(global_idx) }; }
                                                // Symbol cell: label + "+" button wrapped in horizontal so Grid treats them as one column.
                                                let mut sym_clicked = false;
                                                ui.horizontal(|ui| {
                                                    let (sym_resp, action) = symbol_label_with_menu(ui, &f.ticker,
                                                        egui::RichText::new(&f.ticker).small().strong().color(if sel { egui::Color32::WHITE } else { sec_cyan }));
                                                    if !matches!(action, SymbolAction::None) { sec_pending_action = action; }
                                                    if sym_resp.clicked() { sym_clicked = true; }
                                                    if ui.small_button(egui::RichText::new("+").small()).on_hover_text("Open new chart").clicked() {
                                                        sec_pending_action = SymbolAction::OpenChart(f.ticker.clone());
                                                    }
                                                });
                                                if sym_clicked { self.sec_selected_filing = if sel { None } else { Some(global_idx) }; }
                                                let tc = match f.form_type.as_str() { "4" => sec_med, "10-K"|"10-Q" => sec_blue, "8-K" => sec_orange, _ => sec_purple };
                                                ui.label(egui::RichText::new(&f.form_type).color(tc).small());
                                                let cc = match f.category.as_str() { c if c.contains("INSIDER") => sec_med, c if c.contains("DILUTION") => sec_high, c if c.contains("RESTATE") => sec_orange, _ => sec_low };
                                                ui.label(egui::RichText::new(&f.category).color(cc).small());
                                                let (sector, industry) = sec_fund_map
                                                    .get(f.ticker.as_str())
                                                    .map(|(s, i)| (s.as_str(), i.as_str()))
                                                    .unwrap_or(("", ""));
                                                ui.label(egui::RichText::new(sector).small().color(if sector.is_empty() { sec_low } else { rc }));
                                                ui.label(egui::RichText::new(industry).small().color(if industry.is_empty() { sec_low } else { rc }));
                                                ui.label(egui::RichText::new(&f.company_name).small().color(rc));
                                                ui.label(egui::RichText::new(&f.accession_number).color(sec_low).small().monospace());
                                                ui.end_row();
                                            }
                                        });
                                    }
                                });
                            } else {
                                // ═══════════ ALERTS TAB (full height) ═══════════
                                let alerts = &self.bg.sec_alerts;
                                ui.horizontal(|ui| {
                                    if !alerts.is_empty() {
                                        if ui.small_button(egui::RichText::new("Dismiss All").color(sec_low)).clicked() {
                                            if let Some(ref cache) = self.cache {
                                                if let Some(conn) = cache.try_connection() {
                                                    for a in alerts { let _ = sec_filing::dismiss_alert(&conn, a.id, "dismiss all"); }
                                                }
                                            }
                                        }
                                    }
                                    ui.separator();
                                    ui.label(egui::RichText::new("Keywords:").color(AXIS_TEXT).small());
                                    let kw_resp = ui.add(egui::TextEdit::singleline(&mut self.sec_keyword_input).desired_width(150.0).hint_text("add keyword...").font(egui::TextStyle::Small));
                                    if kw_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !self.sec_keyword_input.trim().is_empty() {
                                        let kw = self.sec_keyword_input.trim().to_string();
                                        if let Some(ref cache) = self.cache {
                                            if let Ok(conn) = cache.connection() {
                                                let _ = sec_filing::add_keyword(&conn, &kw);
                                                self.sec_keywords = sec_filing::get_keywords(&conn).unwrap_or_default();
                                            }
                                        }
                                        self.sec_keyword_input.clear();
                                    }
                                });
                                // Lazy-load keywords on first view
                                if self.sec_keywords.is_empty() && self.sec_tab == 1 {
                                    if let Some(ref cache) = self.cache {
                                        if let Some(conn) = cache.try_connection() {
                                            self.sec_keywords = sec_filing::get_keywords(&conn).unwrap_or_default();
                                        }
                                    }
                                }
                                // Show active keywords as removable badges
                                if !self.sec_keywords.is_empty() {
                                    ui.horizontal_wrapped(|ui| {
                                        let mut remove_kw: Option<String> = None;
                                        for kw in &self.sec_keywords {
                                            if ui.small_button(egui::RichText::new(format!("{} x", kw)).color(sec_med).small()).clicked() {
                                                remove_kw = Some(kw.clone());
                                            }
                                        }
                                        if let Some(kw) = remove_kw {
                                            if let Some(ref cache) = self.cache {
                                                if let Ok(conn) = cache.connection() {
                                                    let _ = sec_filing::remove_keyword(&conn, &kw);
                                                    self.sec_keywords = sec_filing::get_keywords(&conn).unwrap_or_default();
                                                }
                                            }
                                        }
                                    });
                                }
                                ui.separator();
                                let avail = ui.available_height().max(200.0);
                                egui::ScrollArea::vertical().id_salt("sec_alerts_tab").min_scrolled_height(avail).auto_shrink(false).show(ui, |ui| {
                                    if alerts.is_empty() {
                                        ui.label(egui::RichText::new("No active alerts.").color(sec_low));
                                    } else {
                                        // Alert type explanations for user understanding
                                        let explain = |t: &str| -> &str {
                                            match t {
                                                t if t.contains("TENDER") => "Acquisition bid filed — potential buyout at premium to market price",
                                                t if t.contains("DELIST") => "Delisting risk — stock may be removed from exchange, position closure forced",
                                                t if t.contains("RESTATE") => "Financial restatement — prior earnings were incorrect, credibility risk",
                                                t if t.contains("DILUTION") => "Share dilution — new shares being issued, existing shares worth less",
                                                t if t.contains("ACTIVIST") => "Activist investor — 5%+ position taken, potential corporate changes",
                                                t if t.contains("AMENDED") => "Material event amended — updated disclosure on significant corporate event",
                                                t if t.contains("LATE") => "Late filing — company missed SEC deadline, potential compliance issues",
                                                t if t.contains("INQUIRY") => "SEC inquiry — regulatory correspondence, potential investigation",
                                                _ => "SEC filing alert",
                                            }
                                        };

                                        let mut dismiss_id: Option<i64> = None;
                                        let mut by_ticker: std::collections::BTreeMap<&str, Vec<_>> = std::collections::BTreeMap::new();
                                        for a in alerts { by_ticker.entry(&a.ticker).or_default().push(a); }
                                        for (ticker, ticker_alerts) in &by_ticker {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.label(egui::RichText::new(*ticker).strong().color(sec_cyan));
                                                for a in ticker_alerts {
                                                    let color = match a.alert_type.as_str() {
                                                        t if t.contains("TENDER") => sec_high, t if t.contains("DELISTING") => sec_high,
                                                        t if t.contains("RESTATEMENT") => sec_orange, t if t.contains("ACTIVIST") => sec_purple,
                                                        t if t.contains("DILUTION") => sec_med, t if t.contains("LATE") => sec_orange,
                                                        t if t.contains("AMENDED") => sec_blue, _ => sec_low,
                                                    };
                                                    let badge = match a.alert_type.as_str() {
                                                        t if t.contains("TENDER") => "TENDER", t if t.contains("DELIST") => "DELIST",
                                                        t if t.contains("RESTATE") => "RESTATE", t if t.contains("DILUTION") => "DILUTION",
                                                        t if t.contains("ACTIVIST") => "ACTIVIST", t if t.contains("AMENDED") => "AMENDED",
                                                        t if t.contains("LATE") => "LATE", t if t.contains("INQUIRY") => "INQUIRY",
                                                        other => other,
                                                    };
                                                    let resp = ui.small_button(egui::RichText::new(badge).color(color).small());
                                                    if resp.clicked() { dismiss_id = Some(a.id); }
                                                    resp.on_hover_text(explain(&a.alert_type));
                                                }
                                            });
                                            // Show explanation for first alert of each ticker
                                            if let Some(first) = ticker_alerts.first() {
                                                if !first.message.is_empty() {
                                                    ui.label(egui::RichText::new(format!("  {}", first.message)).color(sec_low).small());
                                                }
                                            }
                                        }
                                        if let Some(id) = dismiss_id {
                                            if let Some(ref cache) = self.cache {
                                                if let Ok(conn) = cache.connection() { let _ = sec_filing::dismiss_alert(&conn, id, "dismissed"); }
                                            }
                                        }
                                    }
                                });
                            }

                            if self.sec_tab == 2 {
                                // ═══════════ INSIDERS TAB — Cross-symbol insider trade aggregation ═══════════
                                // PERF: pull pre-computed rows and clusters from cache.
                                let rows = &self.sec_cache_insiders;
                                let clusters = &self.sec_cache_insiders_clusters;

                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(format!("{} insider trades across {} symbols",
                                        rows.len(), self.bg.insider_trades.len())).strong());
                                    if !clusters.is_empty() {
                                        ui.separator();
                                        ui.label(egui::RichText::new(format!("{} cluster(s)", clusters.len())).color(sec_high));
                                    }
                                });
                                if !clusters.is_empty() {
                                    ui.horizontal_wrapped(|ui| {
                                        for (ticker, count) in clusters {
                                            ui.label(egui::RichText::new(format!("{}: {}x", ticker, count)).color(sec_high).small());
                                        }
                                    });
                                    ui.separator();
                                }

                                egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                                    egui::Grid::new("insider_agg_grid").striped(true).num_columns(8).min_col_width(50.0).show(ui, |ui| {
                                        ui.strong("Date"); ui.strong("Symbol"); ui.strong("Insider"); ui.strong("Title");
                                        ui.strong("Type"); ui.strong("Shares"); ui.strong("Value"); ui.strong("Flag");
                                        ui.end_row();
                                        for (ticker, trade_idx) in rows.iter().take(500) {
                                            let trade = match self.bg.insider_trades.get(ticker).and_then(|v| v.get(*trade_idx)) {
                                                Some(t) => t,
                                                None => continue, // cache stale for 1 frame — safe to skip
                                            };
                                            let is_sell = matches!(trade.transaction_type.chars().next(), Some('S') | Some('D'));
                                            let row_color = if is_sell { sec_high } else { egui::Color32::from_rgb(46, 204, 113) };
                                            ui.label(egui::RichText::new(&trade.transaction_date).color(AXIS_TEXT).small());
                                            // Symbol cell: label + "+" button (single Grid column via horizontal).
                                            ui.horizontal(|ui| {
                                                let (_, ia_action) = symbol_label_with_menu(ui, &trade.ticker,
                                                    egui::RichText::new(&trade.ticker).color(sec_cyan).small());
                                                if !matches!(ia_action, SymbolAction::None) { sec_pending_action = ia_action; }
                                                if ui.small_button(egui::RichText::new("+").small()).on_hover_text("Open new chart").clicked() {
                                                    sec_pending_action = SymbolAction::OpenChart(trade.ticker.clone());
                                                }
                                            });
                                            ui.label(egui::RichText::new(&trade.insider_name).color(AXIS_TEXT).small());
                                            ui.label(egui::RichText::new(&trade.insider_title).color(sec_low).small());
                                            ui.label(egui::RichText::new(if is_sell { "SELL" } else { "BUY" }).color(row_color).small());
                                            ui.label(egui::RichText::new(format!("{:.0}", trade.shares)).color(AXIS_TEXT).small());
                                            ui.label(egui::RichText::new(format!("${:.0}", trade.aggregate_value)).color(row_color).small());
                                            let flag = if trade.is_officer { "Officer" } else if trade.is_director { "Director" } else { "" };
                                            ui.label(egui::RichText::new(flag).color(sec_purple).small());
                                            ui.end_row();
                                        }
                                    });
                                });
                            }

                            if self.sec_tab == 3 {
                                // ═══════════ TIMELINE TAB — Filing activity heatmap ═══════════
                                // PERF: pre-grouped by month, type breakdown pre-formatted in cache.
                                let timeline = &self.sec_cache_timeline;
                                ui.label(egui::RichText::new(format!("Filing activity: {} months with data", timeline.len())).strong());
                                ui.separator();

                                egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                                    for (month, count, type_str) in timeline {
                                        let intensity = (*count as f32 / 20.0).min(1.0);
                                        let bar_color = egui::Color32::from_rgba_unmultiplied(
                                            (26.0 + 205.0 * intensity) as u8,
                                            (188.0 - 88.0 * intensity) as u8,
                                            (156.0 - 56.0 * intensity) as u8,
                                            200,
                                        );
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(format!("{}: ", month)).color(AXIS_TEXT).monospace().small());
                                            let bar_width = (*count as f32 * 8.0).min(300.0);
                                            let (rect, _) = ui.allocate_exact_size(egui::vec2(bar_width, 14.0), egui::Sense::hover());
                                            ui.painter().rect_filled(rect, 2.0, bar_color);
                                            ui.painter().text(
                                                rect.left_center() + egui::vec2(4.0, 0.0),
                                                egui::Align2::LEFT_CENTER,
                                                format!("{} filings", count),
                                                egui::FontId::proportional(10.0),
                                                egui::Color32::WHITE,
                                            );
                                            ui.label(egui::RichText::new(type_str).color(sec_low).small());
                                        });
                                    }
                                });
                            }
                        });
            if sec_scrape_clicked {
                let symbols = self.sec_scrape_scope_symbols();
                let symbol_count = symbols.len();
                if symbol_count > 0 {
                    // SEC writes go through their own WAL connection with a busy
                    // timeout (see `open_conn` in engine sec_filing), fully decoupled
                    // from the SqliteCache write connection the UI/bar-sync share. A
                    // broad SEC scrape therefore can't freeze the render thread even
                    // mid-catch-up, so it is exempt from the heavy-sync guard (pass
                    // `false`) — you can pull filings any time the scope enumerates.
                    // News still routes writes through the shared conn, so its guard
                    // (and the auto-start bound) stay as-is.
                    if !should_start_manual_background_scope_scrape(
                        self.broker_scope,
                        symbol_count,
                        false,
                    ) {
                        self.scrape_sec_last_msg = format!(
                            "deferred: Scope {} scrape waits for market-data catch-up",
                            sec_scope_label
                        );
                        self.log.push_back(LogEntry::warn(format!(
                            "SEC EDGAR scrape deferred during market-data catch-up for Scope {} ({} symbols); use Active scope or retry after sync settles",
                            sec_scope_label, symbol_count
                        )));
                    } else {
                        let db_path = cache_db_path();
                        let _ = self
                            .broker_tx
                            .send(BrokerCmd::SecScrape { db_path, symbols });
                        self.scrape_sec_running = true;
                        self.scrape_sec_last_msg = format!(
                            "scraping Scope {} ({} symbols)...",
                            sec_scope_label, symbol_count
                        );
                        self.log.push_back(LogEntry::info(format!(
                            "SEC EDGAR scrape initiated for Scope {} ({} symbols)...",
                            sec_scope_label, symbol_count
                        )));
                    }
                } else {
                    self.scrape_sec_last_msg =
                        format!("skipped: Scope {} has no symbols", sec_scope_label);
                    self.log.push_back(LogEntry::warn(format!(
                        "SEC EDGAR scrape skipped: Scope {} has no symbols",
                        sec_scope_label
                    )));
                }
            }
            // Apply deferred symbol context menu action (after window borrow released)
            self.apply_symbol_action(sec_pending_action);
        }

        // Insider Trades (SEC Form 4) — reads from bg cache
        if self.show_insider {
            // UX7: Pre-fetch sparkline for the active chart symbol
            let active_sym = self
                .charts
                .get(self.active_tab)
                .map(|c| c.symbol.clone())
                .unwrap_or_default();
            let active_ticker_only = active_sym
                .split(':')
                .rev()
                .nth(1)
                .or_else(|| active_sym.split(':').last())
                .unwrap_or(&active_sym)
                .to_string();
            let insider_sparkline = self.get_sparkline(&active_ticker_only);
            let mut insider_pending_action = SymbolAction::None;
            egui::Window::new("Insider Trades (Form 4)")
                .open(&mut self.show_insider)
                .resizable(true)
                .default_size([650.0, 400.0])
                .show(ctx, |ui| {
                    let sym = self
                        .charts
                        .get(self.active_tab)
                        .map(|c| c.symbol.clone())
                        .unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Symbol:").color(AXIS_TEXT));
                        let (_, ins_action) = symbol_label_with_menu(
                            ui,
                            &active_ticker_only,
                            egui::RichText::new(&sym).strong().monospace(),
                        );
                        if !matches!(ins_action, SymbolAction::None) {
                            insider_pending_action = ins_action;
                        }
                        // UX7: Inline sparkline next to symbol
                        if !insider_sparkline.is_empty() {
                            draw_inline_sparkline(ui, &insider_sparkline, 100.0, 18.0);
                        }
                    });
                    ui.separator();
                    let ticker = sym
                        .split(':')
                        .rev()
                        .nth(1)
                        .or_else(|| sym.split(':').last())
                        .unwrap_or(&sym);
                    let trades = self.bg.insider_trades.get(ticker);
                    if let Some(trades) = trades {
                        if trades.is_empty() {
                            ui.label(
                                egui::RichText::new(format!(
                                    "No insider trades for {} (last 90 days)",
                                    ticker
                                ))
                                .color(AXIS_TEXT),
                            );
                        } else {
                            // Insider Sentiment Summary
                            {
                                let total_buys = trades
                                    .iter()
                                    .filter(|t| {
                                        let tt = t.transaction_type.to_lowercase();
                                        tt.contains("purchase")
                                            || tt.contains("buy")
                                            || tt.contains("acquisition")
                                    })
                                    .count();
                                let total_sells = trades
                                    .iter()
                                    .filter(|t| {
                                        let tt = t.transaction_type.to_lowercase();
                                        tt.contains("sale")
                                            || tt.contains("sell")
                                            || tt.contains("disposition")
                                    })
                                    .count();
                                let total_value_buy: f64 = trades
                                    .iter()
                                    .filter(|t| {
                                        let tt = t.transaction_type.to_lowercase();
                                        tt.contains("purchase")
                                            || tt.contains("buy")
                                            || tt.contains("acquisition")
                                    })
                                    .map(|t| t.aggregate_value)
                                    .sum();
                                let total_value_sell: f64 = trades
                                    .iter()
                                    .filter(|t| {
                                        let tt = t.transaction_type.to_lowercase();
                                        tt.contains("sale")
                                            || tt.contains("sell")
                                            || tt.contains("disposition")
                                    })
                                    .map(|t| t.aggregate_value)
                                    .sum();
                                let sentiment = if total_buys > total_sells * 2 {
                                    ("BULLISH", UP)
                                } else if total_sells > total_buys * 2 {
                                    ("BEARISH", DOWN)
                                } else {
                                    ("NEUTRAL", AXIS_TEXT)
                                };
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("Sentiment: {}", sentiment.0))
                                            .color(sentiment.1)
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "  Buys: {} (${:.0}M)  Sells: {} (${:.0}M)",
                                            total_buys,
                                            total_value_buy / 1_000_000.0,
                                            total_sells,
                                            total_value_sell / 1_000_000.0
                                        ))
                                        .small(),
                                    );
                                });
                                ui.separator();
                            }
                            egui::ScrollArea::vertical()
                                .auto_shrink(false)
                                .max_height(300.0)
                                .show(ui, |ui| {
                                    let mut insider_sorted: Vec<&_> = trades.iter().collect();
                                    match self.insider_sort.column {
                                        0 => insider_sorted.sort_by(|a, b| {
                                            a.transaction_date.cmp(&b.transaction_date)
                                        }),
                                        1 => insider_sorted
                                            .sort_by(|a, b| a.insider_name.cmp(&b.insider_name)),
                                        2 => insider_sorted
                                            .sort_by(|a, b| a.insider_title.cmp(&b.insider_title)),
                                        3 => insider_sorted.sort_by(|a, b| {
                                            a.transaction_type.cmp(&b.transaction_type)
                                        }),
                                        4 => insider_sorted.sort_by(|a, b| {
                                            a.shares
                                                .partial_cmp(&b.shares)
                                                .unwrap_or(std::cmp::Ordering::Equal)
                                        }),
                                        5 => insider_sorted.sort_by(|a, b| {
                                            a.aggregate_value
                                                .partial_cmp(&b.aggregate_value)
                                                .unwrap_or(std::cmp::Ordering::Equal)
                                        }),
                                        _ => {}
                                    }
                                    if !self.insider_sort.ascending {
                                        insider_sorted.reverse();
                                    }
                                    egui::Grid::new("insider_grid")
                                        .striped(true)
                                        .num_columns(6)
                                        .show(ui, |ui| {
                                            if SortState::header(ui, "Date", 0, &self.insider_sort)
                                            {
                                                self.insider_sort.toggle(0);
                                            }
                                            if SortState::header(
                                                ui,
                                                "Insider",
                                                1,
                                                &self.insider_sort,
                                            ) {
                                                self.insider_sort.toggle(1);
                                            }
                                            if SortState::header(ui, "Title", 2, &self.insider_sort)
                                            {
                                                self.insider_sort.toggle(2);
                                            }
                                            if SortState::header(ui, "Type", 3, &self.insider_sort)
                                            {
                                                self.insider_sort.toggle(3);
                                            }
                                            if SortState::header(
                                                ui,
                                                "Shares",
                                                4,
                                                &self.insider_sort,
                                            ) {
                                                self.insider_sort.toggle(4);
                                            }
                                            if SortState::header(ui, "Value", 5, &self.insider_sort)
                                            {
                                                self.insider_sort.toggle(5);
                                            }
                                            ui.end_row();
                                            for t in &insider_sorted {
                                                ui.label(
                                                    egui::RichText::new(&t.transaction_date)
                                                        .small(),
                                                );
                                                ui.label(
                                                    egui::RichText::new(&t.insider_name)
                                                        .small()
                                                        .strong(),
                                                );
                                                ui.label(
                                                    egui::RichText::new(&t.insider_title)
                                                        .color(AXIS_TEXT)
                                                        .small(),
                                                );
                                                let type_col = if t.transaction_type.contains("Buy")
                                                    || t.transaction_type.contains("Acquisition")
                                                {
                                                    UP
                                                } else {
                                                    DOWN
                                                };
                                                ui.label(
                                                    egui::RichText::new(&t.transaction_type)
                                                        .color(type_col)
                                                        .small(),
                                                );
                                                ui.label(
                                                    egui::RichText::new(format!("{:.0}", t.shares))
                                                        .small(),
                                                );
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "${:.0}",
                                                        t.aggregate_value
                                                    ))
                                                    .small(),
                                                );
                                                ui.end_row();
                                            }
                                        });
                                });
                        }
                    } else {
                        ui.label(
                            egui::RichText::new(format!(
                                "No insider trades for {} (last 90 days)",
                                ticker
                            ))
                            .color(AXIS_TEXT),
                        );
                    }
                });
            self.apply_symbol_action(insider_pending_action);
        }

        // Unusual Volume Scanner
        if self.show_unusual_volume {
            let filter_active = research_sort_indices::active_only_filter_enabled(
                self.volume_active_only,
                self.cached_active_symbols.len(),
            );
            let mut uv_pending_action = SymbolAction::None;
            // UX7: Pre-fetch sparklines for unusual volume symbols
            let mut uv_sparklines: std::collections::HashMap<String, std::sync::Arc<Vec<f64>>> =
                std::collections::HashMap::new();
            let unusual_volume_results = std::sync::Arc::clone(&self.unusual_volume_results);
            for (sym, _, _, _) in unusual_volume_results.iter().take(100) {
                let closes = self.get_sparkline(sym);
                if !closes.is_empty() {
                    uv_sparklines.insert(sym.to_uppercase(), closes);
                }
            }
            egui::Window::new("Unusual Volume Scanner")
                .open(&mut self.show_unusual_volume)
                .resizable(true)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} symbols with volume > 1.5x 20-day average",
                                self.unusual_volume_results.len()
                            ))
                            .strong(),
                        );
                        ui.checkbox(
                            &mut self.volume_active_only,
                            egui::RichText::new("Active Only").small(),
                        );
                    });
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            egui::Grid::new("unusual_vol_grid")
                                .striped(true)
                                .num_columns(5)
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new("Symbol")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("30d")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Today Vol")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Avg Vol")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Ratio")
                                            .color(AXIS_TEXT)
                                            .small()
                                            .strong(),
                                    );
                                    ui.end_row();
                                    for (sym, today, avg, ratio) in
                                        self.unusual_volume_results.iter()
                                    {
                                        // PERF: sym is already uppercase (set at creation) — skip redundant alloc.
                                        if filter_active
                                            && !self
                                                .cached_active_symbols_set
                                                .contains(sym.as_str())
                                        {
                                            continue;
                                        }
                                        let ratio_c = if *ratio > 3.0 {
                                            egui::Color32::from_rgb(231, 76, 60)
                                        } else if *ratio > 2.0 {
                                            egui::Color32::from_rgb(241, 196, 15)
                                        } else {
                                            egui::Color32::from_rgb(46, 204, 113)
                                        };
                                        let (_, uv_action) = symbol_label_with_menu(
                                            ui,
                                            sym,
                                            egui::RichText::new(sym).small().strong(),
                                        );
                                        if !matches!(uv_action, SymbolAction::None) {
                                            uv_pending_action = uv_action;
                                        }
                                        // sym is normalized to uppercase at creation (see ScanUnusualVolume handler)
                                        if let Some(closes) = uv_sparklines.get(sym.as_str()) {
                                            draw_inline_sparkline(ui, closes, 50.0, 12.0);
                                        } else {
                                            ui.label(
                                                egui::RichText::new("—").color(AXIS_TEXT).small(),
                                            );
                                        }
                                        let fmt_vol = |v: f64| -> String {
                                            if v >= 1_000_000.0 {
                                                format!("{:.1}M", v / 1_000_000.0)
                                            } else if v >= 1_000.0 {
                                                format!("{:.1}K", v / 1_000.0)
                                            } else {
                                                format!("{:.0}", v)
                                            }
                                        };
                                        ui.label(egui::RichText::new(fmt_vol(*today)).small());
                                        ui.label(
                                            egui::RichText::new(fmt_vol(*avg))
                                                .small()
                                                .color(AXIS_TEXT),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:.1}x", ratio))
                                                .color(ratio_c)
                                                .small()
                                                .strong(),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                });
            self.apply_symbol_action(uv_pending_action);
        }

        // Sector Rotation Dashboard
        if self.show_sector_rotation {
            egui::Window::new("Sector Rotation")
                .open(&mut self.show_sector_rotation)
                .resizable(true)
                .default_size([600.0, 350.0])
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new("Sector Performance (from fundamentals data)").strong(),
                    );
                    ui.separator();
                    let fund = &self.bg.all_fundamentals;
                    let mut sectors: std::collections::BTreeMap<String, (usize, f64, f64)> =
                        std::collections::BTreeMap::new();
                    for f in fund {
                        if f.sector.is_empty() {
                            continue;
                        }
                        let entry = sectors.entry(f.sector.clone()).or_insert((0, 0.0, 0.0));
                        entry.0 += 1;
                        if let Some(pe) = f.pe_ratio {
                            entry.1 += pe;
                        }
                        if let Some(mc) = f.market_cap {
                            entry.2 += mc;
                        }
                    }
                    egui::Grid::new("sector_rot_grid")
                        .striped(true)
                        .num_columns(4)
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("Sector")
                                    .color(AXIS_TEXT)
                                    .small()
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new("Symbols")
                                    .color(AXIS_TEXT)
                                    .small()
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new("Avg P/E")
                                    .color(AXIS_TEXT)
                                    .small()
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new("Total MCap")
                                    .color(AXIS_TEXT)
                                    .small()
                                    .strong(),
                            );
                            ui.end_row();
                            for (sector, (count, total_pe, total_mcap)) in &sectors {
                                ui.label(egui::RichText::new(sector).small());
                                ui.label(egui::RichText::new(format!("{}", count)).small());
                                let avg_pe = if *count > 0 {
                                    total_pe / *count as f64
                                } else {
                                    0.0
                                };
                                ui.label(egui::RichText::new(format!("{:.1}", avg_pe)).small());
                                ui.label(
                                    egui::RichText::new(fundamentals::format_large_number(
                                        *total_mcap,
                                    ))
                                    .small(),
                                );
                                ui.end_row();
                            }
                        });
                });
        }

        // FRED Economic Data Dashboard
        if self.show_fred {
            egui::Window::new("FRED Economic Data")
                .open(&mut self.show_fred)
                .resizable(true)
                .default_size([700.0, 500.0])
                .show(ctx, |ui| {
                    // Yield Curve
                    if !self.fred_yield_curve.is_empty() {
                        ui.label(egui::RichText::new("Treasury Yield Curve").strong());
                        let points: PlotPoints = PlotPoints::new(
                            self.fred_yield_curve
                                .iter()
                                .enumerate()
                                .map(|(i, (_, v))| [i as f64, *v])
                                .collect(),
                        );
                        let line = Line::new("Yield", points).color(ACCENT).width(2.0);
                        Plot::new("yield_curve_plot")
                            .height(120.0)
                            .allow_drag(false)
                            .allow_zoom(false)
                            .show(ui, |plot_ui| {
                                plot_ui.line(line);
                            });
                        ui.horizontal(|ui| {
                            for (label, rate) in &self.fred_yield_curve {
                                ui.label(
                                    egui::RichText::new(format!("{}: {:.2}%", label, rate))
                                        .small()
                                        .monospace(),
                                );
                            }
                        });
                        // 2Y-10Y inversion check
                        if self.fred_yield_curve.len() >= 3 {
                            let y2 = self.fred_yield_curve[0].1;
                            let y10 = self.fred_yield_curve[2].1;
                            if y2 > y10 {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "INVERTED: 2Y ({:.2}%) > 10Y ({:.2}%) -- recession signal",
                                        y2, y10
                                    ))
                                    .color(DOWN),
                                );
                            }
                        }
                        ui.separator();
                    }

                    // Series charts
                    for series in &self.fred_data {
                        ui.collapsing(format!("{} ({})", series.title, series.id), |ui| {
                            if series.observations.len() > 2 {
                                let last =
                                    series.observations.last().map(|o| o.value).unwrap_or(0.0);
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Latest: {:.2} ({})",
                                        last,
                                        series
                                            .observations
                                            .last()
                                            .map(|o| o.date.as_str())
                                            .unwrap_or("?")
                                    ))
                                    .strong(),
                                );
                                let points: PlotPoints = PlotPoints::new(
                                    series
                                        .observations
                                        .iter()
                                        .enumerate()
                                        .map(|(i, o)| [i as f64, o.value])
                                        .collect(),
                                );
                                let line = Line::new(&series.title, points).color(ACCENT);
                                Plot::new(format!("fred_{}", series.id))
                                    .height(80.0)
                                    .allow_drag(false)
                                    .allow_zoom(false)
                                    .show(ui, |plot_ui| {
                                        plot_ui.line(line);
                                    });
                            }
                        });
                    }

                    if self.fred_data.is_empty() && self.fred_yield_curve.is_empty() {
                        ui.label(egui::RichText::new("Loading FRED data...").color(AXIS_TEXT));
                    }
                });
        }

        // Economic Calendar — ForexFactory (keyless) or Finnhub (if key set).
        // Parses the collapsed "actual" field into forecast/previous/actual columns
        // and adds impact + currency filters with persistent staleness indicator.
        if self.show_econ_calendar {
            egui::Window::new("Economic Calendar")
                .open(&mut self.show_econ_calendar)
                .resizable(true)
                .default_size([960.0, 520.0])
                .show(ctx, |ui| {
                    // ── Header row: refresh, source tag, staleness ──
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Economic Calendar").strong());
                        let source = if self.finnhub_key.is_empty() {
                            "ForexFactory"
                        } else {
                            "Finnhub"
                        };
                        ui.label(
                            egui::RichText::new(format!("[{source}]"))
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if ui.small_button("Refresh").clicked() {
                            let _ = self.broker_tx.send(BrokerCmd::FetchEconCalendar {
                                finnhub_key: self.finnhub_key.clone(),
                            });
                        }
                        if self.econ_last_fetch_ts > 0 {
                            let age = chrono::Utc::now().timestamp() - self.econ_last_fetch_ts;
                            let (label, color) = if age < 60 {
                                (
                                    format!("updated {}s ago", age),
                                    egui::Color32::from_rgb(120, 220, 120),
                                )
                            } else if age < 3600 {
                                (format!("updated {}m ago", age / 60), AXIS_TEXT)
                            } else {
                                (
                                    format!("updated {}h ago — STALE", age / 3600),
                                    egui::Color32::from_rgb(220, 180, 60),
                                )
                            };
                            ui.label(egui::RichText::new(label).small().color(color));
                        } else {
                            ui.label(
                                egui::RichText::new("not yet fetched")
                                    .small()
                                    .color(AXIS_TEXT),
                            );
                        }
                    });
                    // ── Filter row 1: impact ──
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Impact:").strong().small());
                        ui.checkbox(
                            &mut self.econ_filter_high,
                            egui::RichText::new("High").color(egui::Color32::from_rgb(231, 76, 60)),
                        );
                        ui.checkbox(
                            &mut self.econ_filter_medium,
                            egui::RichText::new("Medium")
                                .color(egui::Color32::from_rgb(241, 196, 15)),
                        );
                        ui.checkbox(&mut self.econ_filter_low, "Low");
                        ui.checkbox(&mut self.econ_filter_holiday, "Holiday");
                    });
                    // ── Filter row 2: currency ──
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Currencies:").strong().small());
                        ui.add(
                            egui::TextEdit::singleline(&mut self.econ_filter_currencies)
                                .hint_text("e.g. USD,EUR,GBP (empty = all)")
                                .desired_width(260.0),
                        );
                        if ui.small_button("Clear").clicked() {
                            self.econ_filter_currencies.clear();
                        }
                        // Quick presets
                        if ui.small_button("USD").clicked() {
                            self.econ_filter_currencies = "USD".to_string();
                        }
                        if ui.small_button("Majors").clicked() {
                            self.econ_filter_currencies =
                                "USD,EUR,GBP,JPY,CHF,CAD,AUD,NZD".to_string();
                        }
                    });
                    ui.separator();

                    if self.econ_events.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(40.0);
                            ui.label(egui::RichText::new("No events loaded").color(AXIS_TEXT));
                            ui.label(
                                egui::RichText::new(
                                    "Click Refresh to fetch from ForexFactory (keyless)",
                                )
                                .small()
                                .color(AXIS_TEXT),
                            );
                        });
                    } else {
                        // Build allowed impact set
                        let allow_impact = |imp: &str| -> bool {
                            match imp.to_ascii_lowercase().as_str() {
                                "high" => self.econ_filter_high,
                                "medium" => self.econ_filter_medium,
                                "low" => self.econ_filter_low,
                                _ => self.econ_filter_holiday,
                            }
                        };
                        let allow_currency: Option<std::collections::HashSet<String>> =
                            if self.econ_filter_currencies.trim().is_empty() {
                                None
                            } else {
                                Some(
                                    self.econ_filter_currencies
                                        .split(',')
                                        .map(|s| s.trim().to_ascii_uppercase())
                                        .filter(|s| !s.is_empty())
                                        .collect(),
                                )
                            };

                        // Parse the FF-flattened "actual" field: "fc:X (prev:Y)" → (forecast, previous)
                        let parse_fc_prev = |raw: &str| -> (String, String, String) {
                            if let Some(rest) = raw.strip_prefix("fc:") {
                                if let Some(paren) = rest.find(" (prev:") {
                                    let fc = rest[..paren].to_string();
                                    let rest2 = &rest[paren + 7..];
                                    let prev = rest2.trim_end_matches(')').to_string();
                                    return (String::new(), fc, prev);
                                }
                            }
                            // Finnhub path: actual is a single value
                            (raw.to_string(), String::new(), String::new())
                        };

                        // Count visible for the header badge
                        let visible: Vec<&(String, String, String, String, String)> = self
                            .econ_events
                            .iter()
                            .filter(|(_, country, _, impact, _)| {
                                if !allow_impact(impact) {
                                    return false;
                                }
                                if let Some(ref set) = allow_currency {
                                    if !set.contains(&country.to_ascii_uppercase()) {
                                        return false;
                                    }
                                }
                                true
                            })
                            .collect();

                        ui.label(
                            egui::RichText::new(format!(
                                "{} events shown ({} total)",
                                visible.len(),
                                self.econ_events.len()
                            ))
                            .small()
                            .color(AXIS_TEXT),
                        );
                        ui.separator();

                        egui::ScrollArea::vertical()
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                egui::Grid::new("econ_cal_grid_v2")
                                    .striped(true)
                                    .num_columns(7)
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new("Date/Time")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Curr")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Impact")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Event")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Actual")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Forecast")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Previous")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.end_row();

                                        for (date, country, event, impact, raw) in &visible {
                                            let date_short = if date.len() > 20 {
                                                &date[..20]
                                            } else {
                                                date.as_str()
                                            };
                                            ui.label(
                                                egui::RichText::new(date_short).small().monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(country)
                                                    .small()
                                                    .strong()
                                                    .color(egui::Color32::from_rgb(100, 180, 255)),
                                            );
                                            let impact_c = match impact
                                                .to_ascii_lowercase()
                                                .as_str()
                                            {
                                                "high" => egui::Color32::from_rgb(231, 76, 60),
                                                "medium" => egui::Color32::from_rgb(241, 196, 15),
                                                "low" => egui::Color32::from_rgb(100, 180, 100),
                                                _ => AXIS_TEXT,
                                            };
                                            ui.label(
                                                egui::RichText::new(impact.as_str())
                                                    .color(impact_c)
                                                    .small()
                                                    .strong(),
                                            );
                                            ui.label(egui::RichText::new(event.as_str()).small());
                                            let (actual, forecast, prev) = parse_fc_prev(raw);
                                            let actual_disp = if actual.is_empty() {
                                                "—".to_string()
                                            } else {
                                                actual
                                            };
                                            let fc_disp = if forecast.is_empty() {
                                                "—".to_string()
                                            } else {
                                                forecast
                                            };
                                            let prev_disp = if prev.is_empty() {
                                                "—".to_string()
                                            } else {
                                                prev
                                            };
                                            ui.label(
                                                egui::RichText::new(actual_disp)
                                                    .small()
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(fc_disp)
                                                    .small()
                                                    .monospace()
                                                    .color(AXIS_TEXT),
                                            );
                                            ui.label(
                                                egui::RichText::new(prev_disp)
                                                    .small()
                                                    .monospace()
                                                    .color(AXIS_TEXT),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                });
        }

        // Congressional Trades (House Stock Watcher)
        if self.show_congress {
            let filter_active = research_sort_indices::active_only_filter_enabled(
                self.congress_active_only,
                self.cached_active_symbols.len(),
            );
            let mut cong_pending_action = SymbolAction::None;
            egui::Window::new("Congressional Trades")
                .open(&mut self.show_congress)
                .resizable(true)
                .default_size([750.0, 450.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(
                                "House Stock Watcher \u{2014} Congressional Stock Trades",
                            )
                            .strong(),
                        );
                        if ui.small_button("Refresh").clicked() {
                            let _ = self.broker_tx.send(BrokerCmd::FetchCongressTrades);
                        }
                        ui.checkbox(
                            &mut self.congress_active_only,
                            egui::RichText::new("Active Only").small(),
                        );
                    });
                    ui.separator();
                    if self.congress_trades.is_empty() {
                        ui.label(egui::RichText::new("Loading...").color(AXIS_TEXT));
                    } else {
                        egui::ScrollArea::vertical()
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                egui::Grid::new("congress_grid")
                                    .striped(true)
                                    .num_columns(6)
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new("Date")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Representative")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Ticker")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Type")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Amount")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Party")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.end_row();
                                        for (date, rep, ticker, tx_type, amount, party) in
                                            &self.congress_trades
                                        {
                                            if filter_active
                                                && !self
                                                    .cached_active_symbols_set
                                                    .contains(ticker.as_str())
                                            {
                                                continue;
                                            }
                                            ui.label(egui::RichText::new(date).small().monospace());
                                            ui.label(egui::RichText::new(rep).small());
                                            let (_, ct_action) = symbol_label_with_menu(
                                                ui,
                                                ticker,
                                                egui::RichText::new(ticker)
                                                    .small()
                                                    .strong()
                                                    .color(egui::Color32::WHITE),
                                            );
                                            if !matches!(ct_action, SymbolAction::None) {
                                                cong_pending_action = ct_action;
                                            }
                                            let type_c =
                                                if tx_type.to_lowercase().contains("purchase") {
                                                    UP
                                                } else {
                                                    DOWN
                                                };
                                            ui.label(
                                                egui::RichText::new(tx_type).color(type_c).small(),
                                            );
                                            ui.label(egui::RichText::new(amount).small());
                                            let party_c = match party.as_str() {
                                                "Democrat" => egui::Color32::from_rgb(52, 152, 219),
                                                "Republican" => {
                                                    egui::Color32::from_rgb(231, 76, 60)
                                                }
                                                _ => AXIS_TEXT,
                                            };
                                            ui.label(
                                                egui::RichText::new(party).color(party_c).small(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                });
            self.apply_symbol_action(cong_pending_action);
        }
    }
}
