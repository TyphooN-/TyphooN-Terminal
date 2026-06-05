use super::*;

impl TyphooNApp {
    pub(super) fn render_research_ingest_windows(&mut self, ctx: &egui::Context) {
        // ── Research INGEST_RESEARCH window ──
        if self.show_ingest_research {
            let mut open = self.show_ingest_research;
            egui::Window::new("INGEST — AI Agent Web Research Ingest")
                        .open(&mut open)
                        .resizable(true)
                        .default_size([700.0, 520.0])
                        .show(ctx, |ui| {
                            ui.label(
                                egui::RichText::new(
                                    "Paste the full reply from an AI agent (Claude, Gemini, ChatGPT, …). \
                                 Any ===TYPHOON_INGEST=== block will be parsed and merged into the \
                                 per-symbol web-article cache. LAN peers will pick up the new articles \
                                 on the next sync window.",
                                )
                                .color(AXIS_TEXT)
                                .small(),
                            );
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label("Default agent tag:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.ingest_research_agent)
                                        .desired_width(120.0),
                                );
                                ui.label(
                                    egui::RichText::new(
                                        "(used when an article's 'agent' field is missing)",
                                    )
                                    .color(AXIS_TEXT)
                                    .small(),
                                );
                            });
                            ui.separator();
                            egui::ScrollArea::vertical()
                                .id_salt("ingest_research_scroll")
                                .max_height(360.0)
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut self.ingest_research_text)
                                            .desired_width(f32::INFINITY)
                                            .desired_rows(20)
                                            .font(egui::TextStyle::Monospace)
                                            .hint_text("Paste agent reply here…"),
                                    );
                                });
                            ui.separator();
                            ui.horizontal(|ui| {
                                let can_ingest = !self.ingest_research_busy
                                    && !self.ingest_research_text.trim().is_empty();
                                if ui
                                    .add_enabled(can_ingest, egui::Button::new("Ingest").fill(BTN_MG))
                                    .clicked()
                                {
                                    self.ingest_research_busy = true;
                                    self.ingest_research_status = "Parsing…".into();
                                    let _ = self.broker_tx.send(BrokerCmd::IngestResearchArticles {
                                        text: self.ingest_research_text.clone(),
                                        agent_override: self.ingest_research_agent.clone(),
                                    });
                                }
                                if ui.button("Clear").clicked() {
                                    self.ingest_research_text.clear();
                                    self.ingest_research_status.clear();
                                }
                                if self.ingest_research_busy {
                                    ui.label(egui::RichText::new("Working…").color(AXIS_TEXT).small());
                                }
                            });
                            if !self.ingest_research_status.is_empty() {
                                ui.separator();
                                ui.label(
                                    egui::RichText::new(&self.ingest_research_status)
                                        .color(UP)
                                        .small(),
                                );
                            }
                        });
            self.show_ingest_research = open;
        }

        // ── Research RESEARCH_PACKET viewer window (tree nav + scrollable text) ──
        if self.show_packet_viewer {
            let mut open = self.show_packet_viewer;
            egui::Window::new("RESEARCH_PACKET — Viewer")
                        .open(&mut open)
                        .resizable(true)
                        .default_size([980.0, 680.0])
                        .max_size([980.0, 640.0])
                        .show(ctx, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Symbols:");
                                ui.add(egui::TextEdit::singleline(&mut self.packet_viewer_symbol).desired_width(180.0)
                                    .hint_text("AAPL or AAPL,MSFT"));
                                if ui.button("Use Chart").clicked() {
                                    if let Some(c) = self.charts.get(self.active_tab) {
                                        let s = c.symbol.split(':').rev().nth(1).or_else(|| c.symbol.split(':').last()).unwrap_or("").to_string();
                                        if !s.is_empty() { self.packet_viewer_symbol = s; }
                                    }
                                }
                                ui.label("Question (optional):");
                                ui.add(egui::TextEdit::singleline(&mut self.packet_viewer_question).desired_width(240.0)
                                    .hint_text("e.g. is this cheap vs peers?"));
                                if ui.add(egui::Button::new("Generate").fill(BTN_MG)).clicked() {
                                    let syms: Vec<String> = self.packet_viewer_symbol
                                        .split(',')
                                        .map(|s| s.trim().to_uppercase())
                                        .filter(|s| !s.is_empty())
                                        .collect();
                                    if !syms.is_empty() {
                                        let packet = self.investigate_symbols(&syms, &self.packet_viewer_question.clone());
                                        self.packet_viewer_tree = Self::build_packet_tree(&packet);
                                        self.packet_viewer_text = packet;
                                        self.packet_viewer_selected = None;
                                        self.packet_viewer_scroll_target = Some(0);
                                    }
                                }
                                if ui.button("Copy").clicked() {
                                    ui.ctx().copy_text(self.packet_viewer_text.clone());
                                    self.log.push_back(LogEntry::info("Packet copied to clipboard"));
                                }
                                if ui.button("Save…").clicked() {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .set_title("Save research packet")
                                        .set_file_name(format!(
                                            "research_packet_{}_{}.md",
                                            self.packet_viewer_symbol.replace(',', "_"),
                                            chrono::Utc::now().format("%Y%m%d_%H%M%S")
                                        ))
                                        .add_filter("Markdown", &["md"])
                                        .save_file()
                                    {
                                        if let Err(e) = std::fs::write(&path, &self.packet_viewer_text) {
                                            self.log.push_back(LogEntry::warn(format!("Save failed: {e}")));
                                        } else {
                                            self.log.push_back(LogEntry::info(format!("Saved packet → {}", path.display())));
                                        }
                                    }
                                }
                            });
                            ui.separator();

                            if self.packet_viewer_text.is_empty() {
                                ui.label(egui::RichText::new(
                                    "Enter a symbol (or comma-separated list) and click Generate to build the research packet."
                                ).color(AXIS_TEXT).small());
                                return;
                            }

                            let tree_snapshot = self.packet_viewer_tree.clone();
                            let text_len = self.packet_viewer_text.len();

                            egui::Panel::left("packet_viewer_tree")
                                .resizable(true)
                                .default_size(280.0)
                                .size_range(180.0..=420.0)
                                .show_inside(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(format!("{} sections · {} bytes", tree_snapshot.len(), text_len)).color(AXIS_TEXT).small());
                                        if self.packet_viewer_selected.is_some() {
                                            if ui.small_button("Show All").clicked() {
                                                self.packet_viewer_selected = None;
                                            }
                                        }
                                    });
                                    ui.separator();
                                    egui::ScrollArea::vertical()
                                        .id_salt("packet_viewer_tree_scroll")
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            for (idx, node) in tree_snapshot.iter().enumerate() {
                                                let indent = match node.depth { 2 => 0.0, 3 => 12.0, _ => 24.0 };
                                                let selected = self.packet_viewer_selected == Some(idx);
                                                ui.horizontal(|ui| {
                                                    ui.add_space(indent);
                                                    let text = match node.depth {
                                                        2 => egui::RichText::new(&node.title).strong(),
                                                        3 => egui::RichText::new(&node.title),
                                                        _ => egui::RichText::new(&node.title).small().color(AXIS_TEXT),
                                                    };
                                                    if ui.selectable_label(selected, text).clicked() {
                                                        self.packet_viewer_selected = Some(idx);
                                                    }
                                                });
                                            }
                                        });
                                });

                            egui::CentralPanel::default().show_inside(ui, |ui| {
                                // If a section is selected, slice the text from its byte offset to
                                // the start of the next section with depth <= the selected section's
                                // depth (so selecting an H2 shows its H3/H4 children, selecting an H3
                                // shows only the H3 block, etc.). If nothing is selected, show all.
                                let slice: &str = match self.packet_viewer_selected {
                                    Some(idx) if idx < tree_snapshot.len() => {
                                        let start = tree_snapshot[idx].byte_offset.min(text_len);
                                        let max_depth = tree_snapshot[idx].depth;
                                        let end = tree_snapshot[idx + 1..]
                                            .iter()
                                            .find(|n| n.depth <= max_depth)
                                            .map(|n| n.byte_offset.min(text_len))
                                            .unwrap_or(text_len);
                                        &self.packet_viewer_text[start..end]
                                    }
                                    _ => self.packet_viewer_text.as_str(),
                                };

                                egui::ScrollArea::both()
                                    .id_salt("packet_viewer_body_scroll")
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        let mut body = slice.to_string();
                                        ui.add(
                                            egui::TextEdit::multiline(&mut body)
                                                .font(egui::TextStyle::Monospace)
                                                .desired_width(f32::INFINITY)
                                                .desired_rows(30)
                                                .code_editor(),
                                        );
                                        // Read-only display: edits to `body` are not written back.
                                    });
                            });
                        });
            self.show_packet_viewer = open;
        }
    }
}
