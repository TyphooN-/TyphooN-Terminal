use super::*;
use crate::app::chart_ops::MtfCellValues;

/// Dot colour for one MTF Grid cell: green when the close leads the average
/// (SMA200 / KAMA) or Fisher leads its signal, red when it lags, grey when a value
/// is missing. Mirrors the per-cell logic the grid has always used.
fn mtf_dot_color(ma: &str, (close, sma, kama, fisher, fsig): MtfCellValues) -> egui::Color32 {
    let bullish = match ma {
        "SMA200" => match (close, sma) {
            (Some(c), Some(s)) => Some(c > s),
            _ => None,
        },
        "KAMA" => match (close, kama) {
            (Some(c), Some(k)) => Some(c > k),
            _ => None,
        },
        "Fisher" => match (fisher, fsig) {
            (Some(f), Some(s)) => Some(f > s),
            _ => None,
        },
        _ => None,
    };
    match bullish {
        Some(true) => UP,
        Some(false) => DOWN,
        None => AXIS_TEXT,
    }
}

#[allow(deprecated)]
impl TyphooNApp {
    pub(super) fn render_right_panel_mtf_grid_section(&mut self, ui: &mut egui::Ui) {
        // Keep the unified result cache warm for navbar cells with no open tab. The
        // fill (`compute_mtf_grid_status`) loads those cells off the render thread and
        // writes the cache the dot rows read — no hidden backing charts. We trigger it
        // only when no pass is in flight and the section is open, on:
        //   • active symbol change (focused row should refresh now), or
        //   • a chart opened/closed/retimeframed (a just-closed timeframe must
        //     repopulate from the cache), or
        //   • a short throttle, so cells the fill loads a batch at a time keep
        //     appearing. Self-terminating: once every cell is warm the fill finds
        //     nothing to load and idles until one of the above changes.
        if self.right_mtf_grid_open
            && self.cache.is_some()
            && self.mtf_grid_rx.is_none()
            && !self.symbol_input.trim().is_empty()
        {
            let symbol_changed = self.mtf_grid_status_symbol != self.symbol_input.trim();
            let open_changed = self.mtf_grid_status_open_sig != self.mtf_open_chart_signature();
            let throttle_ok = self
                .mtf_grid_status_at
                .map(|t| t.elapsed().as_secs() >= 6)
                .unwrap_or(true);
            if symbol_changed || open_changed || throttle_ok {
                self.compute_mtf_grid_status();
            }
        }
        // ── MTF Grid ────────────────────────────────────────
        // Assemble the rows up front (one borrow of `self`) so the closure only reads
        // the owned snapshot: per open-tab symbol, its in-order timeframe values
        // (live open tab → unified result cache).
        let symbols = self.mtf_grid_navbar_symbols();
        let symbol_rows: Vec<(String, Vec<(&'static str, MtfCellValues)>)> = symbols
            .iter()
            .map(|symbol| (symbol.clone(), self.mtf_grid_symbol_values(symbol)))
            .collect();
        let max_tfs = symbol_rows
            .iter()
            .map(|(_, vals)| vals.len())
            .max()
            .unwrap_or(0);
        let ma_labels = ["SMA200", "KAMA", "Fisher"];

        let mtf_grid_section = egui::CollapsingHeader::new(
            egui::RichText::new("☰ MTF Grid")
                .color(AXIS_TEXT)
                .small()
                .strong(),
        )
        .id_salt("mtf_grid_section")
        .default_open(self.right_mtf_grid_open)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(format!(
                    "{} symbol{} · {} TFs",
                    symbol_rows.len(),
                    if symbol_rows.len() == 1 { "" } else { "s" },
                    max_tfs,
                ))
                .color(AXIS_TEXT)
                .small(),
            );
            if symbol_rows.is_empty() {
                ui.label(
                    egui::RichText::new("No open chart tabs")
                        .color(AXIS_TEXT)
                        .small(),
                );
                return;
            }
            for (symbol, values) in &symbol_rows {
                ui.label(egui::RichText::new(symbol).color(ACCENT).small().strong());
                if values.is_empty() {
                    // Tab(s) present but bars/cache not loaded yet — the fill warms it.
                    ui.label(egui::RichText::new("loading…").color(AXIS_TEXT).small());
                    continue;
                }
                egui::Grid::new(format!("mtf_ma_grid_{symbol}"))
                    .spacing(egui::vec2(4.0, 2.0))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("").small());
                        for (label, _) in values {
                            ui.label(egui::RichText::new(*label).color(AXIS_TEXT).small());
                        }
                        ui.end_row();
                        for ma in &ma_labels {
                            ui.label(egui::RichText::new(*ma).color(AXIS_TEXT).small());
                            for (_, vals) in values {
                                ui.label(
                                    egui::RichText::new("\u{25CF}")
                                        .color(mtf_dot_color(ma, *vals))
                                        .small(),
                                );
                            }
                            ui.end_row();
                        }
                    });
            }
        });
        self.right_mtf_grid_open = mtf_grid_section.fully_open();
        self.handle_right_panel_section_drag(
            ui,
            RightPanelSectionId::MtfGrid,
            &mtf_grid_section.header_response,
        );
    }
}
