use super::*;

impl TyphooNApp {
    pub(super) fn render_screenshots_gallery_window(&mut self, ctx: &egui::Context) {
        // ── Screenshots Gallery (palette: SCREENSHOTS / GALLERY) ──
        if self.show_screenshots_gallery {
            let now_ts = chrono::Utc::now().timestamp();
            if now_ts - self.screenshots_last_refresh > 10 {
                self.scan_screenshots();
            }
            let mut action_save_as: Option<std::path::PathBuf> = None;
            let mut action_matrix: Option<std::path::PathBuf> = None;
            let mut action_delete: Option<std::path::PathBuf> = None;
            let mut action_open: Option<std::path::PathBuf> = None;
            let mut refresh = false;
            let mut capture = false;
            egui::Window::new("Screenshots Gallery")
                .open(&mut self.show_screenshots_gallery)
                .resizable(true)
                .default_size([820.0, 520.0])
                .min_width(560.0)
                .min_height(320.0)
                .constrain(true)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} screenshots on disk",
                                self.screenshots_list.len()
                            ))
                            .small()
                            .color(AXIS_TEXT),
                        );
                        if ui
                            .small_button("\u{1F4F8} Capture")
                            .on_hover_text("Take a new screenshot (next frame)")
                            .clicked()
                        {
                            capture = true;
                        }
                        if ui.small_button("Refresh").clicked() {
                            refresh = true;
                        }
                    });
                    ui.separator();
                    let scroll_h = (ui.available_height() - 10.0).max(120.0);
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .max_height(scroll_h)
                        .show(ui, |ui| {
                            egui::Grid::new("screenshots_grid")
                                .striped(true)
                                .num_columns(4)
                                .show(ui, |ui| {
                                    sortable_header(
                                        ui,
                                        "File",
                                        0,
                                        &mut self.screenshots_sort_col,
                                        &mut self.screenshots_sort_asc,
                                    );
                                    sortable_header(
                                        ui,
                                        "Size",
                                        1,
                                        &mut self.screenshots_sort_col,
                                        &mut self.screenshots_sort_asc,
                                    );
                                    sortable_header(
                                        ui,
                                        "Taken",
                                        2,
                                        &mut self.screenshots_sort_col,
                                        &mut self.screenshots_sort_asc,
                                    );
                                    ui.label("");
                                    ui.end_row();
                                    let mut rows = self.screenshots_list.clone();
                                    rows.sort_by(|a, b| {
                                        let ord = match self.screenshots_sort_col {
                                            0 => {
                                                a.0.file_name()
                                                    .and_then(|s| s.to_str())
                                                    .unwrap_or("")
                                                    .cmp(
                                                        b.0.file_name()
                                                            .and_then(|s| s.to_str())
                                                            .unwrap_or(""),
                                                    )
                                            }
                                            1 => a.2.cmp(&b.2),
                                            2 => a.1.cmp(&b.1),
                                            _ => a.1.cmp(&b.1),
                                        };
                                        if self.screenshots_sort_asc {
                                            ord
                                        } else {
                                            ord.reverse()
                                        }
                                    });
                                    for (path, mtime, size) in rows.iter() {
                                        let name = path
                                            .file_name()
                                            .and_then(|s| s.to_str())
                                            .unwrap_or("?");
                                        ui.label(egui::RichText::new(name).small().monospace());
                                        let kb = (*size as f64) / 1024.0;
                                        ui.label(
                                            egui::RichText::new(format!("{:.1} KB", kb)).small(),
                                        );
                                        let ts = chrono::DateTime::from_timestamp(*mtime, 0)
                                            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                                            .unwrap_or_else(|| "-".into());
                                        ui.label(egui::RichText::new(ts).small());
                                        ui.horizontal(|ui| {
                                            if ui
                                                .small_button("Open")
                                                .on_hover_text("Open in system image viewer")
                                                .clicked()
                                            {
                                                action_open = Some(path.clone());
                                            }
                                            if ui
                                                .small_button("\u{1F4BE}")
                                                .on_hover_text("Save a copy elsewhere")
                                                .clicked()
                                            {
                                                action_save_as = Some(path.clone());
                                            }
                                            if ui
                                                .small_button("\u{1F4E8}")
                                                .on_hover_text("Send to Matrix community chat")
                                                .clicked()
                                            {
                                                action_matrix = Some(path.clone());
                                            }
                                            if ui
                                                .small_button("\u{1F5D1}")
                                                .on_hover_text("Delete from disk")
                                                .clicked()
                                            {
                                                action_delete = Some(path.clone());
                                            }
                                        });
                                        ui.end_row();
                                    }
                                });
                        });
                });
            if capture {
                self.screenshot_requested = true;
                self.log.push_back(LogEntry::info(
                    "Screenshot requested — capturing next frame...",
                ));
            }
            if refresh {
                self.scan_screenshots();
            }
            if let Some(p) = action_open {
                // xdg-open is universally available on desktop Linux (per env Platform=linux).
                if let Err(e) = std::process::Command::new("xdg-open").arg(&p).spawn() {
                    self.log.push_back(LogEntry::err(format!(
                        "xdg-open failed for {}: {e}",
                        p.display()
                    )));
                }
            }
            if let Some(src) = action_save_as {
                let default_name = src
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("screenshot.webp")
                    .to_string();
                let picked = rfd::FileDialog::new()
                    .add_filter("WebP image", &["webp"])
                    .set_file_name(&default_name)
                    .set_title("Export screenshot")
                    .save_file();
                if let Some(dst) = picked {
                    match std::fs::copy(&src, &dst) {
                        Ok(n) => self.log.push_back(LogEntry::info(format!(
                            "Copied screenshot to {} ({} bytes)",
                            dst.display(),
                            n
                        ))),
                        Err(e) => self
                            .log
                            .push_back(LogEntry::err(format!("Copy failed: {e}"))),
                    }
                }
            }
            if let Some(p) = action_matrix {
                let tok = self.matrix_access_token.as_str();
                if tok.is_empty() || tok == "pending" || tok == "none" {
                    self.log.push_back(LogEntry::warn(
                        "Matrix: no access token — open Community Chat → Settings to log in",
                    ));
                } else if !p.exists() {
                    self.log.push_back(LogEntry::warn(format!(
                        "Screenshot file missing: {}",
                        p.display()
                    )));
                } else {
                    let _ = self.broker_tx.send(BrokerCmd::MatrixSendImage {
                        room_id: self.matrix_room.clone(),
                        access_token: self.matrix_access_token.clone(),
                        file_path: p.clone(),
                    });
                    self.log.push_back(LogEntry::info(format!(
                        "Sharing screenshot to community chat: {}",
                        p.display()
                    )));
                }
            }
            if let Some(p) = action_delete {
                match std::fs::remove_file(&p) {
                    Ok(()) => {
                        self.log
                            .push_back(LogEntry::info(format!("Deleted: {}", p.display())));
                        self.scan_screenshots();
                    }
                    Err(e) => self
                        .log
                        .push_back(LogEntry::err(format!("Delete failed: {e}"))),
                }
            }
        }
    }
}
