use super::*;

/// Sector band for the Finviz-style market map: total cap plus the symbols
/// (cap, change %) sorted largest-first.
struct SectorBand {
    sector: String,
    total_cap: f64,
    avg_change: f64,
    symbols: Vec<(String, f64, f64)>, // (symbol, market_cap, change_pct)
}

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
    /// and colored by daily change (watchlist quote when available). Built
    /// from `bg.all_fundamentals` — a per-frame walk over an already-cached
    /// vec, no DB access (ADR-098).
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
                // Group cached fundamentals into sector bands.
                let mut by_sector: std::collections::BTreeMap<String, Vec<(String, f64, f64)>> =
                    std::collections::BTreeMap::new();
                for f in &self.bg.all_fundamentals {
                    let Some(cap) = f.market_cap.filter(|c| *c > 0.0) else {
                        continue;
                    };
                    let sector = if f.sector.trim().is_empty() {
                        "Other".to_string()
                    } else {
                        f.sector.trim().to_string()
                    };
                    let watchlist_key = bare_symbol_from_key(&f.symbol)
                        .replace('/', "")
                        .trim_end_matches(".EQ")
                        .trim_end_matches(".eq")
                        .to_ascii_uppercase();
                    let change = self
                        .watchlist_by_bare
                        .get(&watchlist_key)
                        .and_then(|&index| self.watchlist_rows.get(index))
                        .map(|row| row.change_pct)
                        .unwrap_or(0.0);
                    by_sector
                        .entry(sector)
                        .or_default()
                        .push((f.symbol.clone(), cap, change));
                }
                if by_sector.is_empty() {
                    ui.label(
                        egui::RichText::new(
                            "No cached fundamentals with market caps yet — run the \
                             fundamentals scrape first (Research → Fundamentals).",
                        )
                        .color(AXIS_TEXT),
                    );
                    return;
                }
                let mut bands: Vec<SectorBand> = by_sector
                    .into_iter()
                    .map(|(sector, mut symbols)| {
                        symbols.sort_by(|a, b| b.1.total_cmp(&a.1));
                        let total_cap: f64 = symbols.iter().map(|s| s.1).sum();
                        let cap_weighted: f64 =
                            symbols.iter().map(|s| s.1 * s.2).sum::<f64>() / total_cap.max(1.0);
                        SectorBand {
                            sector,
                            total_cap,
                            avg_change: cap_weighted,
                            symbols,
                        }
                    })
                    .collect();
                bands.sort_by(|a, b| b.total_cap.total_cmp(&a.total_cap));
                let grand_total: f64 = bands.iter().map(|b| b.total_cap).sum();

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
                                for b in &bands {
                                    ui.label(&b.sector);
                                    ui.label(format!("{}", b.symbols.len()));
                                    ui.label(format!("${:.1}B", b.total_cap / 1e9));
                                    let col = heat_color(b.avg_change);
                                    ui.label(
                                        egui::RichText::new(format!("{:+.2}%", b.avg_change))
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
                let mut hovered: Option<(String, f64, f64)> = None;
                let mut y = rect.top();
                for band in &bands {
                    let band_h = (band.total_cap / grand_total.max(1.0)) as f32 * rect.height();
                    if band_h < 3.0 {
                        break; // long tail too thin to draw
                    }
                    let mut x = rect.left();
                    for (symbol, cap, change) in band.symbols.iter().take(40) {
                        let w = (cap / band.total_cap.max(1.0)) as f32 * rect.width();
                        if w < 2.0 {
                            break;
                        }
                        let cell = egui::Rect::from_min_size(
                            egui::pos2(x, y),
                            egui::vec2(w - 1.0, band_h - 1.0),
                        );
                        painter.rect_filled(cell, 1.0, heat_color(*change));
                        if w > 34.0 && band_h > 14.0 {
                            painter.text(
                                cell.center(),
                                egui::Align2::CENTER_CENTER,
                                symbol,
                                egui::FontId::monospace(10.0),
                                egui::Color32::WHITE,
                            );
                        }
                        if let Some(pos) = hover {
                            if cell.contains(pos) {
                                hovered = Some((symbol.clone(), *cap, *change));
                                if response.clicked() {
                                    pending_action = SymbolAction::OpenChart(symbol.clone());
                                }
                            }
                        }
                        x += w;
                    }
                    painter.text(
                        egui::pos2(rect.left() + 2.0, y + 1.0),
                        egui::Align2::LEFT_TOP,
                        &band.sector,
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
