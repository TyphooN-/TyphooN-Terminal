use super::*;

impl TyphooNApp {
    pub(crate) fn tick_screenshot_capture(&mut self, ctx: &egui::Context) {
        // ── Screenshot: issue capture command ────────────────────────────
        if self.screenshot_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.screenshot_requested = false;
        }

        // ── Screenshot: handle captured image (offload PNG encode to background thread) ──
        {
            let screenshot_data: Option<(Vec<u8>, u32, u32, std::path::PathBuf)> = ctx.input(|i| {
                for event in &i.events {
                    if let egui::Event::Screenshot { image, .. } = event {
                        let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                        let pictures_dir = if let Ok(home) = std::env::var("HOME") {
                            let p = std::path::PathBuf::from(home).join("Pictures");
                            let _ = std::fs::create_dir_all(&p);
                            p
                        } else {
                            std::path::PathBuf::from("/tmp")
                        };
                        let path = pictures_dir.join(format!("typhoon_chart_{}.webp", ts));
                        let w = image.width() as u32;
                        let h = image.height() as u32;
                        let rgba: Vec<u8> = image
                            .pixels
                            .iter()
                            .flat_map(|c| [c.r(), c.g(), c.b(), c.a()])
                            .collect();
                        return Some((rgba, w, h, path));
                    }
                }
                None
            });
            if let Some((rgba, w, h, path)) = screenshot_data {
                // Lossless WebP encoding on background thread (smaller than PNG, no quality loss)
                let last_screenshot_path = path.clone();
                self.log.push_back(LogEntry::info(format!(
                    "Saving screenshot ({w}x{h}) to {}...",
                    path.display()
                )));
                self.last_screenshot_path = Some(last_screenshot_path);
                self.rt_handle.spawn_blocking(move || {
                    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba) {
                        let dyn_img = image::DynamicImage::ImageRgba8(img);
                        match dyn_img.save(&path) {
                            Ok(()) => tracing::info!("Screenshot saved: {}", path.display()),
                            Err(e) => tracing::error!("Screenshot save failed: {}", e),
                        }
                    } else {
                        tracing::error!("Screenshot: failed to construct image from RGBA data");
                    }
                });
            }
        }
    }

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
                                    let order = self.screenshots_sorted_indices.order_then_reverse(
                                        &self.screenshots_list,
                                        self.screenshots_sort_col,
                                        self.screenshots_sort_asc,
                                        |left, right| {
                                            research_sort_indices::screenshot_order(
                                                left,
                                                right,
                                                self.screenshots_sort_col,
                                            )
                                        },
                                    );
                                    for &index in order.iter() {
                                        let (path, mtime, size) = &self.screenshots_list[index];
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
