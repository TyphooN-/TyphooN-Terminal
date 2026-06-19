use super::*;

impl TyphooNApp {
    pub(super) fn render_workspace_reference_windows(&mut self, ctx: &egui::Context) {
        // Object List (drawing management)
        if self.show_object_list {
            let mut delete_idx: Option<usize> = None;
            egui::Window::new("Object List")
                .open(&mut self.show_object_list)
                .resizable(true)
                .default_size([400.0, 300.0])
                .show(ctx, |ui| {
                    if let Some(chart) = self.charts.get(self.active_tab) {
                        if chart.drawings.is_empty() {
                            ui.label("No drawings on this chart.");
                        } else {
                            ui.label(
                                egui::RichText::new(format!("{} drawings", chart.drawings.len()))
                                    .small()
                                    .color(AXIS_TEXT),
                            );
                            ui.separator();
                            egui::ScrollArea::vertical()
                                .auto_shrink(false)
                                .max_height(250.0)
                                .show(ui, |ui| {
                                    egui::Grid::new("object_list_grid").striped(true).show(
                                        ui,
                                        |ui| {
                                            ui.label(egui::RichText::new("#").small().strong());
                                            ui.label(egui::RichText::new("Type").small().strong());
                                            ui.label(
                                                egui::RichText::new("Details").small().strong(),
                                            );
                                            ui.label(egui::RichText::new("").small());
                                            ui.end_row();
                                            for (idx, drawing) in chart.drawings.iter().enumerate()
                                            {
                                                ui.label(
                                                    egui::RichText::new(format!("{}", idx + 1))
                                                        .small(),
                                                );
                                                let (type_name, details) = match drawing {
                                                    Drawing::HLine { price, .. } => {
                                                        ("H-Line", format!("{:.5}", price))
                                                    }
                                                    Drawing::VLine { bar_idx, .. } => {
                                                        ("V-Line", format!("bar {}", bar_idx))
                                                    }
                                                    Drawing::TrendLine { p1, p2, .. } => (
                                                        "Trendline",
                                                        format!("{:.4}→{:.4}", p1.1, p2.1),
                                                    ),
                                                    Drawing::FiboRetrace { high, low, .. } => (
                                                        "Fib Retrace",
                                                        format!("{:.4}–{:.4}", high, low),
                                                    ),
                                                    Drawing::Rectangle { .. } => {
                                                        ("Rectangle", String::new())
                                                    }
                                                    Drawing::Ray { origin, .. } => {
                                                        ("Ray", format!("{:.4}", origin.1))
                                                    }
                                                    Drawing::Channel { .. } => {
                                                        ("Channel", String::new())
                                                    }
                                                    Drawing::ExtendedLine { .. } => {
                                                        ("Ext Line", String::new())
                                                    }
                                                    Drawing::HRay { price, .. } => {
                                                        ("H-Ray", format!("{:.5}", price))
                                                    }
                                                    Drawing::CrossLine { price, .. } => {
                                                        ("Cross", format!("{:.5}", price))
                                                    }
                                                    Drawing::ArrowLine { .. } => {
                                                        ("Arrow", String::new())
                                                    }
                                                    Drawing::InfoLine { p1, p2, .. } => (
                                                        "Info Line",
                                                        format!("{:.4}→{:.4}", p1.1, p2.1),
                                                    ),
                                                    Drawing::Pitchfork { .. } => {
                                                        ("Pitchfork", String::new())
                                                    }
                                                    Drawing::FiboExtension { .. } => {
                                                        ("Fib Extension", String::new())
                                                    }
                                                    Drawing::GannFan { .. } => {
                                                        ("Gann Fan", String::new())
                                                    }
                                                    Drawing::LongPosition {
                                                        entry,
                                                        stop,
                                                        target,
                                                    } => (
                                                        "Long Pos",
                                                        format!(
                                                            "E:{:.4} S:{:.4} T:{:.4}",
                                                            entry.1, stop, target
                                                        ),
                                                    ),
                                                    Drawing::ShortPosition {
                                                        entry,
                                                        stop,
                                                        target,
                                                    } => (
                                                        "Short Pos",
                                                        format!(
                                                            "E:{:.4} S:{:.4} T:{:.4}",
                                                            entry.1, stop, target
                                                        ),
                                                    ),
                                                    Drawing::PriceRange { .. } => {
                                                        ("Price Range", String::new())
                                                    }
                                                    Drawing::TextLabel { text, .. } => {
                                                        ("Text", text.clone())
                                                    }
                                                    Drawing::ArrowMarker { is_up, .. } => (
                                                        if *is_up {
                                                            "Arrow Up"
                                                        } else {
                                                            "Arrow Down"
                                                        },
                                                        String::new(),
                                                    ),
                                                    Drawing::Ellipse { .. } => {
                                                        ("Ellipse", String::new())
                                                    }
                                                    Drawing::Triangle { .. } => {
                                                        ("Triangle", String::new())
                                                    }
                                                    Drawing::TrendAngle { .. } => {
                                                        ("Trend Angle", String::new())
                                                    }
                                                    Drawing::ParallelChannel { .. } => {
                                                        ("Parallel Ch", String::new())
                                                    }
                                                    Drawing::FibChannel { .. } => {
                                                        ("Fib Channel", String::new())
                                                    }
                                                    Drawing::FibTimeZones { bar_idx, .. } => {
                                                        ("Fib Time", format!("bar {}", bar_idx))
                                                    }
                                                    Drawing::PriceLabel { price, .. } => {
                                                        ("Price Label", format!("{:.5}", price))
                                                    }
                                                    Drawing::Callout { text, .. } => {
                                                        ("Callout", text.clone())
                                                    }
                                                    Drawing::Highlighter { .. } => {
                                                        ("Highlighter", String::new())
                                                    }
                                                    Drawing::CrossMarker { price, .. } => {
                                                        ("Cross", format!("{:.5}", price))
                                                    }
                                                    Drawing::Polyline { points, .. } => (
                                                        "Polyline",
                                                        format!("{} pts", points.len()),
                                                    ),
                                                    Drawing::AnchorNote { text, .. } => {
                                                        ("Note", text.clone())
                                                    }
                                                    Drawing::RegressionChannel { .. } => {
                                                        ("Regression", String::new())
                                                    }
                                                    Drawing::GannBox { .. } => {
                                                        ("Gann Box", String::new())
                                                    }
                                                    Drawing::ElliottWave { points, .. } => (
                                                        "Elliott Wave",
                                                        format!("{} pts", points.len()),
                                                    ),
                                                    Drawing::AbcCorrection { .. } => {
                                                        ("ABC Correction", String::new())
                                                    }
                                                    Drawing::DateRange { p1, p2, .. } => (
                                                        "Date Range",
                                                        format!(
                                                            "{} bars",
                                                            if p2.0 > p1.0 {
                                                                p2.0 - p1.0
                                                            } else {
                                                                p1.0 - p2.0
                                                            }
                                                        ),
                                                    ),
                                                    Drawing::DatePriceRange { p1, p2, .. } => (
                                                        "Date+Price",
                                                        format!(
                                                            "{} bars",
                                                            if p2.0 > p1.0 {
                                                                p2.0 - p1.0
                                                            } else {
                                                                p1.0 - p2.0
                                                            }
                                                        ),
                                                    ),
                                                    Drawing::HeadShoulders { .. } => {
                                                        ("H&S Pattern", String::new())
                                                    }
                                                    Drawing::XabcdPattern { .. } => {
                                                        ("XABCD", String::new())
                                                    }
                                                    Drawing::Brush { points, .. } => {
                                                        ("Brush", format!("{} pts", points.len()))
                                                    }
                                                    Drawing::SchiffPitchfork { .. } => {
                                                        ("Schiff Fork", String::new())
                                                    }
                                                    Drawing::ModSchiffPitchfork { .. } => {
                                                        ("Mod Schiff", String::new())
                                                    }
                                                    Drawing::CyclicLines {
                                                        bar_start,
                                                        bar_end,
                                                        ..
                                                    } => (
                                                        "Cyclic Lines",
                                                        format!(
                                                            "{} interval",
                                                            if *bar_end > *bar_start {
                                                                bar_end - bar_start
                                                            } else {
                                                                1
                                                            }
                                                        ),
                                                    ),
                                                    Drawing::SineWave { .. } => {
                                                        ("Sine Wave", String::new())
                                                    }
                                                    Drawing::Emoji { emoji, .. } => {
                                                        ("Emoji", emoji.clone())
                                                    }
                                                    Drawing::Flag { .. } => ("Flag", String::new()),
                                                    Drawing::Balloon { text, .. } => {
                                                        ("Balloon", text.clone())
                                                    }
                                                    Drawing::SessionBreak { bar_idx, .. } => (
                                                        "Session Break",
                                                        format!("bar {}", bar_idx),
                                                    ),
                                                    Drawing::MagnetLevel { price, .. } => {
                                                        ("Magnet Level", format!("{:.5}", price))
                                                    }
                                                    Drawing::RiskRewardBox {
                                                        entry,
                                                        stop,
                                                        target,
                                                    } => (
                                                        "R:R Box",
                                                        format!(
                                                            "E:{:.4} S:{:.4} T:{:.4}",
                                                            entry.1, stop, target
                                                        ),
                                                    ),
                                                    Drawing::FibCircle { .. } => {
                                                        ("Fib Circle", String::new())
                                                    }
                                                    Drawing::ArcDraw { .. } => {
                                                        ("Arc", String::new())
                                                    }
                                                    Drawing::CurveDraw { .. } => {
                                                        ("Curve", String::new())
                                                    }
                                                    Drawing::PathDraw { points, .. } => {
                                                        ("Path", format!("{} pts", points.len()))
                                                    }
                                                    Drawing::Forecast { .. } => {
                                                        ("Forecast", String::new())
                                                    }
                                                    Drawing::GhostFeed { p1, p2, .. } => (
                                                        "Ghost Feed",
                                                        format!(
                                                            "{} bars",
                                                            if p2.0 > p1.0 {
                                                                p2.0 - p1.0
                                                            } else {
                                                                p1.0 - p2.0
                                                            }
                                                        ),
                                                    ),
                                                    Drawing::Signpost { .. } => {
                                                        ("Signpost", String::new())
                                                    }
                                                    Drawing::Ruler { p1, p2, .. } => {
                                                        ("Ruler", format!("{:.4}", p2.1 - p1.1))
                                                    }
                                                    Drawing::TimeCycle {
                                                        bar_start,
                                                        bar_end,
                                                        ..
                                                    } => (
                                                        "Time Cycle",
                                                        format!(
                                                            "{} interval",
                                                            if *bar_end > *bar_start {
                                                                bar_end - bar_start
                                                            } else {
                                                                1
                                                            }
                                                        ),
                                                    ),
                                                    Drawing::SpeedResistanceFan { .. } => {
                                                        ("Speed Fan", String::new())
                                                    }
                                                    Drawing::SpeedResistanceArc { .. } => {
                                                        ("Speed Arc", String::new())
                                                    }
                                                    Drawing::FibSpiral { .. } => {
                                                        ("Fib Spiral", String::new())
                                                    }
                                                    Drawing::RotatedRectangle { .. } => {
                                                        ("Rotated Rect", String::new())
                                                    }
                                                    Drawing::AnchoredVwapLine {
                                                        bar_idx, ..
                                                    } => ("aVWAP", format!("bar {}", bar_idx)),
                                                    Drawing::TrendChannel { .. } => {
                                                        ("Trend Channel", String::new())
                                                    }
                                                    Drawing::InsidePitchfork { .. } => {
                                                        ("Inside Pitchfork", String::new())
                                                    }
                                                    Drawing::FibWedge { .. } => {
                                                        ("Fib Wedge", String::new())
                                                    }
                                                    Drawing::PriceNote { price, text, .. } => (
                                                        "Price Note",
                                                        format!("{:.4} {}", price, text),
                                                    ),
                                                    Drawing::MeasureTool { p1, p2, .. } => {
                                                        ("Measure", format!("{:.4}", p2.1 - p1.1))
                                                    }
                                                    Drawing::AnchoredText { text, .. } => {
                                                        ("Anchored Text", text.clone())
                                                    }
                                                    Drawing::Comment { text, .. } => {
                                                        ("Comment", text.clone())
                                                    }
                                                    Drawing::ArrowMarkerLeft { .. } => {
                                                        ("Arrow Left", String::new())
                                                    }
                                                    Drawing::ArrowMarkerRight { .. } => {
                                                        ("Arrow Right", String::new())
                                                    }
                                                    Drawing::Circle { .. } => {
                                                        ("Circle", String::new())
                                                    }
                                                    Drawing::PitchFan { .. } => {
                                                        ("Pitch Fan", String::new())
                                                    }
                                                    Drawing::TrendFibTime { .. } => {
                                                        ("Trend Fib Time", String::new())
                                                    }
                                                    Drawing::GannSquare { .. } => {
                                                        ("Gann Square", String::new())
                                                    }
                                                    Drawing::GannSquareFixed { .. } => {
                                                        ("Gann Square Fixed", String::new())
                                                    }
                                                    Drawing::BarsPattern { .. } => {
                                                        ("Bars Pattern", String::new())
                                                    }
                                                    Drawing::Projection { .. } => {
                                                        ("Projection", String::new())
                                                    }
                                                    Drawing::DoubleCurve { .. } => {
                                                        ("Double Curve", String::new())
                                                    }
                                                    Drawing::TrianglePattern { .. } => {
                                                        ("Triangle Pattern", String::new())
                                                    }
                                                    Drawing::ThreeDrives { .. } => {
                                                        ("Three Drives", String::new())
                                                    }
                                                    Drawing::ElliottDouble { .. } => {
                                                        ("Elliott WXY", String::new())
                                                    }
                                                    Drawing::AbcdPattern { .. } => {
                                                        ("ABCD", String::new())
                                                    }
                                                    Drawing::CypherPattern { .. } => {
                                                        ("Cypher", String::new())
                                                    }
                                                    Drawing::ElliottTriangle { .. } => {
                                                        ("Elliott ABCDE", String::new())
                                                    }
                                                    Drawing::ElliottTripleCombo { .. } => {
                                                        ("Elliott WXYXZ", String::new())
                                                    }
                                                };
                                                ui.label(egui::RichText::new(type_name).small());
                                                ui.label(
                                                    egui::RichText::new(details)
                                                        .small()
                                                        .color(AXIS_TEXT),
                                                );
                                                if ui.small_button("Del").clicked() {
                                                    delete_idx = Some(idx);
                                                }
                                                ui.end_row();
                                            }
                                        },
                                    );
                                });
                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.button("Clear All").clicked() {
                                    delete_idx = Some(usize::MAX); // sentinel for clear all
                                }
                            });
                        }
                    }
                });
            if let Some(idx) = delete_idx {
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    if idx == usize::MAX {
                        chart.drawings.clear();
                    } else if idx < chart.drawings.len() {
                        chart.drawings.remove(idx);
                    }
                }
            }
        }

        // Help — keyboard shortcuts + quick command reference.
        // Searchable filter covers both sections.
        if self.show_help {
            egui::Window::new("Keyboard Shortcuts & Command Reference")
                .open(&mut self.show_help)
                .resizable(true)
                .default_size([720.0, 560.0])
                .max_size([720.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Help");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.help_filter)
                                .hint_text("filter keys/commands…")
                                .desired_width(260.0),
                        );
                        if ui.small_button("Clear").clicked() {
                            self.help_filter.clear();
                        }
                    });
                    ui.separator();

                    let filter_lower = self.help_filter.to_lowercase();
                    let matches = |key: &str, desc: &str| -> bool {
                        filter_lower.is_empty()
                            || key.to_lowercase().contains(&filter_lower)
                            || desc.to_lowercase().contains(&filter_lower)
                    };

                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            // ── Chart navigation ──
                            ui.label(
                                egui::RichText::new("Chart navigation")
                                    .color(ACCENT)
                                    .strong(),
                            );
                            egui::Grid::new("help_nav")
                                .striped(true)
                                .num_columns(2)
                                .show(ui, |ui| {
                                    let nav: &[(&str, &str)] = &[
                                        ("Scroll wheel", "Zoom chart (horizontal)"),
                                        ("Ctrl + scroll", "Zoom chart (vertical / price)"),
                                        ("Double-click", "Reset zoom & pan"),
                                        ("Click + drag", "Pan chart"),
                                        ("← →", "Bar-by-bar scroll"),
                                        ("Home / End", "Jump to start / end"),
                                        ("PgUp / PgDn", "Half-screen scroll"),
                                        ("+ / -", "Zoom in / out"),
                                        ("Delete / Backspace", "Remove last drawing"),
                                        ("Right-click", "Context menu (drawings, chart type)"),
                                    ];
                                    for (k, d) in nav {
                                        if !matches(k, d) {
                                            continue;
                                        }
                                        ui.label(egui::RichText::new(*k).monospace());
                                        ui.label(*d);
                                        ui.end_row();
                                    }
                                });
                            ui.add_space(8.0);

                            // ── App / window management ──
                            ui.label(egui::RichText::new("App & window").color(ACCENT).strong());
                            egui::Grid::new("help_app")
                                .striped(true)
                                .num_columns(2)
                                .show(ui, |ui| {
                                    let app: &[(&str, &str)] = &[
                                        (
                                            "~ (tilde/backtick)",
                                            "Open command palette (Quake-style)",
                                        ),
                                        (
                                            "Esc",
                                            "Close palette / cancel drawing / close top window",
                                        ),
                                        ("Ctrl+N", "New chart tab"),
                                        ("Ctrl+W", "Close current tab"),
                                        ("Ctrl+Tab", "Next tab"),
                                        ("Ctrl+Shift+Tab", "Previous tab"),
                                        ("Alt+1..9", "Jump to timeframe 1..9"),
                                        ("F5", "Reload bars from cache"),
                                        ("F11", "Toggle fullscreen"),
                                        ("Alt+F4", "Quit"),
                                    ];
                                    for (k, d) in app {
                                        if !matches(k, d) {
                                            continue;
                                        }
                                        ui.label(egui::RichText::new(*k).monospace());
                                        ui.label(*d);
                                        ui.end_row();
                                    }
                                });
                            ui.add_space(8.0);

                            // ── Commands reference (auto-generated from COMMANDS registry) ──
                            // Skips the DRAW_* cluster — they're listed in their own section below.
                            ui.label(
                                egui::RichText::new(format!(
                                    "Command palette ({} commands)",
                                    COMMANDS
                                        .iter()
                                        .filter(|c| !c.name.starts_with("DRAW_"))
                                        .count()
                                ))
                                .color(ACCENT)
                                .strong(),
                            );
                            ui.label(
                                egui::RichText::new(
                                    "Press ~ then type. All commands are case-insensitive.",
                                )
                                .small()
                                .color(AXIS_TEXT),
                            );
                            egui::Grid::new("help_cmds")
                                .striped(true)
                                .num_columns(2)
                                .show(ui, |ui| {
                                    for cmd in COMMANDS {
                                        if cmd.name.starts_with("DRAW_") {
                                            continue;
                                        }
                                        if !matches(cmd.name, cmd.desc) {
                                            continue;
                                        }
                                        ui.label(
                                            egui::RichText::new(cmd.name)
                                                .monospace()
                                                .color(egui::Color32::from_rgb(150, 200, 255)),
                                        );
                                        ui.label(cmd.desc);
                                        ui.end_row();
                                    }
                                });
                            ui.add_space(8.0);

                            // ── Drawing tools (separate section) ──
                            ui.collapsing(
                                egui::RichText::new(format!(
                                    "Drawing tools ({} types)",
                                    COMMANDS
                                        .iter()
                                        .filter(|c| c.name.starts_with("DRAW_"))
                                        .count()
                                ))
                                .color(ACCENT)
                                .strong(),
                                |ui| {
                                    egui::Grid::new("help_draw")
                                        .striped(true)
                                        .num_columns(2)
                                        .show(ui, |ui| {
                                            for cmd in COMMANDS {
                                                if !cmd.name.starts_with("DRAW_") {
                                                    continue;
                                                }
                                                if !matches(cmd.name, cmd.desc) {
                                                    continue;
                                                }
                                                ui.label(
                                                    egui::RichText::new(cmd.name)
                                                        .monospace()
                                                        .color(egui::Color32::from_rgb(
                                                            150, 200, 255,
                                                        )),
                                                );
                                                ui.label(cmd.desc);
                                                ui.end_row();
                                            }
                                        });
                                },
                            );
                            ui.add_space(10.0);

                            // ── Status footer ──
                            ui.separator();
                            ui.label(egui::RichText::new("TyphooN Terminal").color(ACCENT));
                            let gpu_ind = if self.gpu_indicators.is_some() {
                                "GPU Indicators: Active"
                            } else {
                                "GPU Indicators: CPU fallback"
                            };
                            ui.label(
                                egui::RichText::new(gpu_ind)
                                    .color(if self.gpu_indicators.is_some() {
                                        UP
                                    } else {
                                        DOWN
                                    })
                                    .small(),
                            );
                        });
                });
        }

        // Data Window — all indicator values at crosshair position
        if self.show_data_window {
            egui::Window::new("Data Window")
                .open(&mut self.show_data_window)
                .resizable(true)
                .default_size([400.0, 500.0])
                .show(ctx, |ui| {
                    if let Some(chart) = self.charts.get(self.active_tab) {
                        let (si, ei) = chart.visible_range();
                        let bars = &chart.bars[si..ei];
                        if let Some(_pos) = self.crosshair {
                            // Find bar index from crosshair
                            if !bars.is_empty() {
                                let price_axis_w = 70.0_f32;
                                let _bar_w =
                                    (ui.available_width() + price_axis_w) / bars.len() as f32; // approximate
                                let _rel_idx = 0.max(bars.len() / 2); // fallback to middle if we can't calculate
                                // Use most recent bar as fallback
                                let abs_idx = ei.saturating_sub(1);
                                let b = &chart.bars[abs_idx];
                                ui.heading(format!(
                                    "{} [{}]",
                                    chart.symbol,
                                    chart.timeframe.label()
                                ));
                                ui.separator();
                                egui::Grid::new("data_grid")
                                    .striped(true)
                                    .num_columns(2)
                                    .show(ui, |ui| {
                                        ui.label("Open");
                                        ui.label(format_price(b.open));
                                        ui.end_row();
                                        ui.label("High");
                                        ui.label(format_price(b.high));
                                        ui.end_row();
                                        ui.label("Low");
                                        ui.label(format_price(b.low));
                                        ui.end_row();
                                        ui.label("Close");
                                        ui.label(format_price(b.close));
                                        ui.end_row();
                                        ui.label("Volume");
                                        ui.label(format!("{:.0}", b.volume));
                                        ui.end_row();
                                        ui.end_row();
                                        if let Some(Some(v)) = chart.sma200.get(abs_idx) {
                                            ui.label(
                                                egui::RichText::new("SMA200").color(SMA200_COL),
                                            );
                                            ui.label(format_price(*v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.sma100.get(abs_idx) {
                                            ui.label(
                                                egui::RichText::new("SMA100").color(SMA100_COL),
                                            );
                                            ui.label(format_price(*v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.ema21.get(abs_idx) {
                                            ui.label(egui::RichText::new("EMA21").color(EMA_COL));
                                            ui.label(format_price(*v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.kama.get(abs_idx) {
                                            ui.label(egui::RichText::new("KAMA").color(KAMA_COL));
                                            ui.label(format_price(*v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.wma.get(abs_idx) {
                                            ui.label(egui::RichText::new("WMA20").color(WMA_COL));
                                            ui.label(format_price(*v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.hma.get(abs_idx) {
                                            ui.label(egui::RichText::new("HMA20").color(HMA_COL));
                                            ui.label(format_price(*v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.bb_upper.get(abs_idx) {
                                            ui.label(egui::RichText::new("BB Upper").color(BB_COL));
                                            ui.label(format_price(*v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.bb_lower.get(abs_idx) {
                                            ui.label(egui::RichText::new("BB Lower").color(BB_COL));
                                            ui.label(format_price(*v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.rsi.get(abs_idx) {
                                            let rsi_col = if *v > 70.0 {
                                                DOWN
                                            } else if *v < 30.0 {
                                                UP
                                            } else {
                                                RSI_LINE
                                            };
                                            ui.label(egui::RichText::new("RSI").color(rsi_col));
                                            ui.label(
                                                egui::RichText::new(format!("{:.1}", v))
                                                    .color(rsi_col),
                                            );
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.fisher.get(abs_idx) {
                                            let f_col =
                                                if *v > 0.0 { FISHER_POS } else { FISHER_NEG };
                                            ui.label(egui::RichText::new("Fisher").color(f_col));
                                            ui.label(
                                                egui::RichText::new(format!("{:.3}", v))
                                                    .color(f_col),
                                            );
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.atr.get(abs_idx) {
                                            ui.label(egui::RichText::new("ATR").color(AXIS_TEXT));
                                            ui.label(
                                                egui::RichText::new(format_price(*v))
                                                    .color(AXIS_TEXT),
                                            );
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.macd_line.get(abs_idx) {
                                            ui.label(
                                                egui::RichText::new("MACD").color(MACD_LINE_COL),
                                            );
                                            ui.label(format!("{:.4}", v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.stoch_k.get(abs_idx) {
                                            ui.label(
                                                egui::RichText::new("Stoch %K").color(STOCH_K_COL),
                                            );
                                            ui.label(format!("{:.1}", v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.adx.get(abs_idx) {
                                            ui.label(egui::RichText::new("ADX").color(ADX_COL));
                                            ui.label(format!("{:.1}", v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.cci.get(abs_idx) {
                                            ui.label(egui::RichText::new("CCI").color(CCI_COL));
                                            ui.label(format!("{:.1}", v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.williams_r.get(abs_idx) {
                                            ui.label(egui::RichText::new("W%R").color(WILLR_COL));
                                            ui.label(format!("{:.1}", v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.momentum.get(abs_idx) {
                                            ui.label("Momentum");
                                            ui.label(format_price(*v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.mfi.get(abs_idx) {
                                            let col = if *v > 80.0 {
                                                DOWN
                                            } else if *v < 20.0 {
                                                UP
                                            } else {
                                                MFI_COL
                                            };
                                            ui.label(egui::RichText::new("MFI").color(col));
                                            ui.label(
                                                egui::RichText::new(format!("{:.1}", v)).color(col),
                                            );
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.trix_line.get(abs_idx) {
                                            ui.label(
                                                egui::RichText::new("TRIX").color(TRIX_LINE_COL),
                                            );
                                            ui.label(format!("{:+.4}", v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.ppo_line.get(abs_idx) {
                                            ui.label(
                                                egui::RichText::new("PPO").color(PPO_LINE_COL),
                                            );
                                            ui.label(format!("{:+.3}", v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.ultosc.get(abs_idx) {
                                            let col = if *v > 70.0 {
                                                DOWN
                                            } else if *v < 30.0 {
                                                UP
                                            } else {
                                                ULTOSC_COL
                                            };
                                            ui.label(egui::RichText::new("ULTOSC").color(col));
                                            ui.label(
                                                egui::RichText::new(format!("{:.1}", v)).color(col),
                                            );
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.stochrsi_k.get(abs_idx) {
                                            ui.label(
                                                egui::RichText::new("StochRSI %K")
                                                    .color(STOCH_K_COL),
                                            );
                                            ui.label(format!("{:.1}", v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.var_oscillator.get(abs_idx) {
                                            ui.label("VaR Osc");
                                            ui.label(format!("{:.1}", v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.psar.get(abs_idx) {
                                            ui.label(egui::RichText::new("P.SAR").color(SAR_COL));
                                            ui.label(format_price(*v));
                                            ui.end_row();
                                        }
                                    });
                            }
                        } else {
                            ui.label(
                                egui::RichText::new("Move cursor over chart").color(AXIS_TEXT),
                            );
                        }
                    }
                });
        }
    }
}
