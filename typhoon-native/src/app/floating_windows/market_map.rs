use super::*;

fn heat_color(change_pct: f64) -> egui::Color32 {
    let t = (change_pct / 3.0).clamp(-1.0, 1.0) as f32;
    if t >= 0.0 {
        egui::Color32::from_rgb(
            (20.0 + 10.0 * (1.0 - t)) as u8,
            (70.0 + 130.0 * t) as u8,
            30,
        )
    } else {
        egui::Color32::from_rgb(
            (70.0 - 130.0 * t) as u8,
            (20.0 + 10.0 * (1.0 + t)) as u8,
            30,
        )
    }
}

impl TyphooNApp {
    /// Finviz-style market map (ADR-116 "Maps" + "Groups"): sector bands
    /// sized by total cached market cap, symbols within a band sized by cap
    /// and colored by daily change (watchlist quote when available). The static
    /// sector/capitalization model is built on the background refresh thread;
    /// each frame only joins current quote changes and paints it (ADR-098).
    pub(super) fn render_market_map_window(&mut self, ctx: &egui::Context) {
        if !self.show_market_map {
            return;
        }
        let mut open = self.show_market_map;
        let mut pending_action = SymbolAction::None;
        egui::Window::new("Market Map — sectors × market cap")
            .open(&mut open)
            .resizable(true)
            .default_size([760.0, 520.0])
            .show(ctx, |ui| {
                let model = &self.bg.market_map_model;
                if model.sectors.is_empty() {
                    ui.label(
                        egui::RichText::new(
                            "No cached fundamentals with market caps yet — run the \
                             fundamentals scrape first (Research → Fundamentals).",
                        )
                        .color(AXIS_TEXT),
                    );
                    return;
                }
                let live_change = |watchlist_key: &str| {
                    self.watchlist_by_bare
                        .get(watchlist_key)
                        .and_then(|&index| self.watchlist_rows.get(index))
                        .map(|row| row.change_pct)
                        .unwrap_or(0.0)
                };

                // Sector groups table (Finviz "Groups").
                egui::CollapsingHeader::new("Sector groups (cap-weighted performance)")
                    .default_open(false)
                    .show(ui, |ui| {
                        egui::Grid::new("sector_groups")
                            .striped(true)
                            .show(ui, |ui| {
                                for h in ["Sector", "Symbols", "Total MCap", "Cap-weighted Chg"] {
                                    ui.strong(h);
                                }
                                ui.end_row();
                                for sector in &model.sectors {
                                    let avg_change =
                                        market_map_model::cap_weighted_change(sector, &live_change);
                                    ui.label(&sector.sector);
                                    ui.label(format!("{}", sector.symbols.len()));
                                    ui.label(format!("${:.1}B", sector.total_cap / 1e9));
                                    let col = heat_color(avg_change);
                                    ui.label(
                                        egui::RichText::new(format!("{avg_change:+.2}%"))
                                            .color(col),
                                    );
                                    ui.end_row();
                                }
                            });
                    });
                ui.separator();

                // Treemap: horizontal sector bands sized by cap; symbols as
                // vertical slices within a band (slice-and-dice layout).
                let avail = ui.available_size();
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(avail.x, (avail.y - 4.0).max(220.0)),
                    egui::Sense::click(),
                );
                let painter = ui.painter_at(rect);
                let hover = response.hover_pos();
                let mut hovered: Option<(&str, f64, f64)> = None;
                let mut y = rect.top();
                for sector in &model.sectors {
                    let band_h =
                        (sector.total_cap / model.grand_total.max(1.0)) as f32 * rect.height();
                    if band_h < 3.0 {
                        break; // long tail too thin to draw
                    }
                    let mut x = rect.left();
                    for symbol in sector.symbols.iter().take(40) {
                        let change = live_change(&symbol.watchlist_key);
                        let w =
                            (symbol.market_cap / sector.total_cap.max(1.0)) as f32 * rect.width();
                        if w < 2.0 {
                            break;
                        }
                        let cell = egui::Rect::from_min_size(
                            egui::pos2(x, y),
                            egui::vec2(w - 1.0, band_h - 1.0),
                        );
                        painter.rect_filled(cell, 1.0, heat_color(change));
                        if w > 34.0 && band_h > 14.0 {
                            painter.text(
                                cell.center(),
                                egui::Align2::CENTER_CENTER,
                                &symbol.symbol,
                                egui::FontId::monospace(10.0),
                                egui::Color32::WHITE,
                            );
                        }
                        if let Some(pos) = hover {
                            if cell.contains(pos) {
                                hovered = Some((&symbol.symbol, symbol.market_cap, change));
                                if response.clicked() {
                                    pending_action = SymbolAction::OpenChart(symbol.symbol.clone());
                                }
                            }
                        }
                        x += w;
                    }
                    painter.text(
                        egui::pos2(rect.left() + 2.0, y + 1.0),
                        egui::Align2::LEFT_TOP,
                        &sector.sector,
                        egui::FontId::proportional(9.0),
                        AXIS_TEXT,
                    );
                    y += band_h;
                }
                if let Some((symbol, cap, change)) = hovered {
                    response.clone().on_hover_text(format!(
                        "{symbol} · ${:.1}B · {change:+.2}% (click to chart)",
                        cap / 1e9
                    ));
                }
            });
        self.show_market_map = open;
        self.apply_symbol_action(pending_action);
    }
}
