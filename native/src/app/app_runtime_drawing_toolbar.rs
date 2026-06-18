use super::*;

impl TyphooNApp {
    pub(crate) fn render_drawing_toolbar(&mut self, ctx: &egui::Context) {
        // ── Drawing toolbar (horizontal top bar, TradingView style) ─────────
        egui::Panel::top("drawing_toolbar")
            .max_height(24.0)
            .frame(
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(18, 18, 25))
                    .inner_margin(egui::Margin::symmetric(4, 1)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
                    let dm = self.draw_mode;
                    let active_col = egui::Color32::from_rgb(80, 200, 255);
                    let normal_col = egui::Color32::from_rgb(140, 140, 160);
                    let drawing_count = self
                        .charts
                        .get(self.active_tab)
                        .map(|c| c.drawings.len())
                        .unwrap_or(0);

                    // ── Lines group ──
                    ui.menu_button(
                        egui::RichText::new("Lines").small().color(normal_col),
                        |ui| {
                            if ui.button("─  Horizontal Line").clicked() {
                                self.draw_mode = DrawMode::PlacingHLine;
                                ui.close();
                            }
                            if ui.button("│  Vertical Line").clicked() {
                                self.draw_mode = DrawMode::PlacingVLine;
                                ui.close();
                            }
                            if ui.button("╲  Trendline").clicked() {
                                self.draw_mode = DrawMode::PlacingTrendP1;
                                ui.close();
                            }
                            if ui.button("╱  Ray").clicked() {
                                self.draw_mode = DrawMode::PlacingRayP1;
                                ui.close();
                            }
                            if ui.button("↔  Extended Line").clicked() {
                                self.draw_mode = DrawMode::PlacingExtLineP1;
                                ui.close();
                            }
                            if ui.button("→  Horizontal Ray").clicked() {
                                self.draw_mode = DrawMode::PlacingHRay;
                                ui.close();
                            }
                            if ui.button("+  Cross Line").clicked() {
                                self.draw_mode = DrawMode::PlacingCrossLine;
                                ui.close();
                            }
                            if ui.button("➤  Arrow Line").clicked() {
                                self.draw_mode = DrawMode::PlacingArrowP1;
                                ui.close();
                            }
                            if ui.button("ℹ  Info Line").clicked() {
                                self.draw_mode = DrawMode::PlacingInfoLineP1;
                                ui.close();
                            }
                            if ui.button("∠  Trend Angle").clicked() {
                                self.draw_mode = DrawMode::PlacingTrendAngleP1;
                                ui.close();
                            }
                            if ui.button("⫼  Parallel Channel").clicked() {
                                self.draw_mode = DrawMode::PlacingParallelChP1;
                                ui.close();
                            }
                            if ui.button("~  Polyline (dbl-click end)").clicked() {
                                self.draw_mode = DrawMode::PlacingPolyline;
                                self.polyline_points.clear();
                                ui.close();
                            }
                            if ui.button("⫽  Trend Channel (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingTrendChannelP1;
                                ui.close();
                            }
                        },
                    );
                    ui.separator();

                    // ── Gann & Fib group ──
                    ui.menu_button(
                        egui::RichText::new("Fib/Gann").small().color(normal_col),
                        |ui| {
                            if ui.button("Fib Retracement").clicked() {
                                self.draw_mode = DrawMode::PlacingFiboP1;
                                ui.close();
                            }
                            if ui.button("Fib Extension (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingFiboExtP1;
                                ui.close();
                            }
                            if ui.button("Fib Channel (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingFibChannelP1;
                                ui.close();
                            }
                            if ui.button("Fib Time Zones").clicked() {
                                self.draw_mode = DrawMode::PlacingFibTimeZones;
                                ui.close();
                            }
                            if ui.button("Andrews Pitchfork (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingPitchforkP1;
                                ui.close();
                            }
                            if ui.button("Schiff Pitchfork (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingSchiffPitchforkP1;
                                ui.close();
                            }
                            if ui.button("Mod Schiff Pitchfork (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingModSchiffPitchforkP1;
                                ui.close();
                            }
                            if ui.button("Gann Fan").clicked() {
                                self.draw_mode = DrawMode::PlacingGannFan;
                                ui.close();
                            }
                            if ui.button("Gann Box (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingGannBoxP1;
                                ui.close();
                            }
                            if ui.button("Cyclic Lines (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingCyclicLinesP1;
                                ui.close();
                            }
                            if ui.button("Sine Wave (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingSineWaveP1;
                                ui.close();
                            }
                            if ui.button("Fib Circle (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingFibCircleP1;
                                ui.close();
                            }
                            if ui.button("Fib Spiral (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingFibSpiralP1;
                                ui.close();
                            }
                            if ui.button("Speed Resistance Fan (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingSpeedFanP1;
                                ui.close();
                            }
                            if ui.button("Speed Resistance Arc (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingSpeedArcP1;
                                ui.close();
                            }
                            if ui.button("Time Cycle (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingTimeCycleP1;
                                ui.close();
                            }
                            if ui.button("Inside Pitchfork (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingInsidePitchforkP1;
                                ui.close();
                            }
                            if ui.button("Fib Wedge (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingFibWedgeP1;
                                ui.close();
                            }
                            if ui.button("Pitch Fan (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingPitchFanP1;
                                ui.close();
                            }
                            if ui.button("Trend Fib Time (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingTrendFibTimeP1;
                                ui.close();
                            }
                            if ui.button("Gann Square (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingGannSquareP1;
                                ui.close();
                            }
                            if ui.button("Gann Square Fixed (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingGannSquareFixedP1;
                                ui.close();
                            }
                        },
                    );
                    ui.separator();

                    // ── Elliott Wave group ──
                    ui.menu_button(
                        egui::RichText::new("Elliott").small().color(normal_col),
                        |ui| {
                            if ui.button("Elliott Wave 1-5 (5 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingElliottWave;
                                self.multi_click_points.clear();
                                ui.close();
                            }
                            if ui.button("ABC Correction (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingAbcCorrection;
                                self.multi_click_points.clear();
                                ui.close();
                            }
                            if ui.button("Elliott Double WXY (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingElliottDouble;
                                self.multi_click_points.clear();
                                ui.close();
                            }
                            if ui.button("Elliott Triangle ABCDE (5 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingElliottTriangle;
                                self.multi_click_points.clear();
                                ui.close();
                            }
                            if ui.button("Elliott Triple WXYXZ (5 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingElliottTripleCombo;
                                self.multi_click_points.clear();
                                ui.close();
                            }
                        },
                    );
                    ui.separator();

                    // ── Measurement group ──
                    ui.menu_button(
                        egui::RichText::new("Measure").small().color(normal_col),
                        |ui| {
                            if ui.button("Date Range (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingDateRangeP1;
                                ui.close();
                            }
                            if ui.button("Price Range (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingPriceRangeP1;
                                ui.close();
                            }
                            if ui.button("Date & Price Range (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingDatePriceRangeP1;
                                ui.close();
                            }
                            if ui.button("Ruler (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingRulerP1;
                                ui.close();
                            }
                            if ui.button("Measure Tool (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingMeasureToolP1;
                                ui.close();
                            }
                            if ui.button("Bars Pattern (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingBarsPatternP1;
                                ui.close();
                            }
                            if ui.button("Projection (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingProjectionP1;
                                ui.close();
                            }
                        },
                    );
                    ui.separator();

                    // ── Patterns group ──
                    ui.menu_button(
                        egui::RichText::new("Patterns").small().color(normal_col),
                        |ui| {
                            if ui.button("Head & Shoulders (5 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingHeadShoulders;
                                self.multi_click_points.clear();
                                ui.close();
                            }
                            if ui.button("XABCD Pattern (5 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingXabcdPattern;
                                self.multi_click_points.clear();
                                ui.close();
                            }
                            if ui.button("Triangle Pattern (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingTrianglePattern;
                                self.multi_click_points.clear();
                                ui.close();
                            }
                            if ui.button("Three Drives (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingThreeDrives;
                                self.multi_click_points.clear();
                                ui.close();
                            }
                            if ui.button("ABCD Pattern (4 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingAbcdPattern;
                                self.multi_click_points.clear();
                                ui.close();
                            }
                            if ui.button("Cypher Pattern (5 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingCypherPattern;
                                self.multi_click_points.clear();
                                ui.close();
                            }
                        },
                    );
                    ui.separator();

                    // ── Shapes group ──
                    ui.menu_button(
                        egui::RichText::new("Shapes").small().color(normal_col),
                        |ui| {
                            if ui.button("▭  Rectangle").clicked() {
                                self.draw_mode = DrawMode::PlacingRectP1;
                                ui.close();
                            }
                            if ui.button("═  Channel (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingChannelP1;
                                ui.close();
                            }
                            if ui.button("◯  Ellipse").clicked() {
                                self.draw_mode = DrawMode::PlacingEllipseP1;
                                ui.close();
                            }
                            if ui.button("△  Triangle (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingTriangleP1;
                                ui.close();
                            }
                            if ui.button("▮  Highlighter").clicked() {
                                self.draw_mode = DrawMode::PlacingHighlighterP1;
                                ui.close();
                            }
                            if ui.button("⊞  Regression Channel").clicked() {
                                self.draw_mode = DrawMode::PlacingRegressionChP1;
                                ui.close();
                            }
                            if ui.button("◇  Rotated Rectangle (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingRotatedRectP1;
                                ui.close();
                            }
                            if ui.button("⌒  Arc (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingArcP1;
                                ui.close();
                            }
                            if ui.button("∿  Curve (4 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingCurveP1;
                                ui.close();
                            }
                            if ui.button("⤳  Path (multi-click)").clicked() {
                                self.draw_mode = DrawMode::PlacingPath;
                                self.polyline_points.clear();
                                ui.close();
                            }
                            if ui.button("◯  Circle (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingCircleP1;
                                ui.close();
                            }
                            if ui.button("∿  Double Curve (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingDoubleCurveP1;
                                ui.close();
                            }
                        },
                    );
                    ui.separator();

                    // ── Annotations ──
                    ui.menu_button(
                        egui::RichText::new("Annotate").small().color(normal_col),
                        |ui| {
                            if ui.button("T  Text Label").clicked() {
                                self.draw_mode = DrawMode::PlacingTextLabel;
                                ui.close();
                            }
                            if ui.button("▲  Arrow Up").clicked() {
                                self.draw_mode = DrawMode::PlacingArrowMarkerUp;
                                ui.close();
                            }
                            if ui.button("▼  Arrow Down").clicked() {
                                self.draw_mode = DrawMode::PlacingArrowMarkerDown;
                                ui.close();
                            }
                            if ui.button("+  Cross Marker").clicked() {
                                self.draw_mode = DrawMode::PlacingCrossMarker;
                                ui.close();
                            }
                            if ui.button("$  Price Label").clicked() {
                                self.draw_mode = DrawMode::PlacingPriceLabel;
                                ui.close();
                            }
                            if ui.button("⌐  Callout").clicked() {
                                self.draw_mode = DrawMode::PlacingCalloutP1;
                                ui.close();
                            }
                            if ui.button("☰  Anchor Note").clicked() {
                                self.draw_mode = DrawMode::PlacingAnchorNote;
                                ui.close();
                            }
                            if ui.button("✎  Brush/Freehand").clicked() {
                                self.draw_mode = DrawMode::PlacingBrush;
                                self.brush_points.clear();
                                ui.close();
                            }
                            if ui.button("\u{1F3AF}  Emoji").clicked() {
                                self.draw_mode = DrawMode::PlacingEmoji;
                                ui.close();
                            }
                            if ui.button("\u{1F6A9}  Flag").clicked() {
                                self.draw_mode = DrawMode::PlacingFlag;
                                ui.close();
                            }
                            if ui.button("\u{1F4AC}  Balloon (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingBalloonP1;
                                ui.close();
                            }
                            if ui.button("|  Session Break").clicked() {
                                self.draw_mode = DrawMode::PlacingSessionBreak;
                                ui.close();
                            }
                            if ui.button("\u{1F9F2}  Magnet Level").clicked() {
                                self.draw_mode = DrawMode::PlacingMagnetLevel;
                                ui.close();
                            }
                            if ui.button("\u{1F9ED}  Signpost").clicked() {
                                self.draw_mode = DrawMode::PlacingSignpost;
                                ui.close();
                            }
                            if ui.button("\u{1F4C8}  Forecast (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingForecastP1;
                                ui.close();
                            }
                            if ui.button("\u{1F47B}  Ghost Feed (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingGhostFeedP1;
                                ui.close();
                            }
                            if ui.button("\u{1F4CA}  Anchored VWAP").clicked() {
                                self.draw_mode = DrawMode::PlacingAnchoredVwap;
                                ui.close();
                            }
                            if ui.button("\u{1F4DD}  Price Note").clicked() {
                                self.draw_mode = DrawMode::PlacingPriceNote;
                                ui.close();
                            }
                            if ui.button("A  Anchored Text").clicked() {
                                self.draw_mode = DrawMode::PlacingAnchoredText;
                                ui.close();
                            }
                            if ui.button("#  Comment").clicked() {
                                self.draw_mode = DrawMode::PlacingComment;
                                ui.close();
                            }
                            if ui.button("<  Arrow Left").clicked() {
                                self.draw_mode = DrawMode::PlacingArrowMarkerLeft;
                                ui.close();
                            }
                            if ui.button(">  Arrow Right").clicked() {
                                self.draw_mode = DrawMode::PlacingArrowMarkerRight;
                                ui.close();
                            }
                        },
                    );
                    ui.separator();

                    // ── Trading ──
                    ui.menu_button(
                        egui::RichText::new("Trade").small().color(normal_col),
                        |ui| {
                            if ui.button("Long Position (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingLongPosP1;
                                ui.close();
                            }
                            if ui.button("Short Position (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingShortPosP1;
                                ui.close();
                            }
                            if ui.button("Price Range (2 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingPriceRangeP1;
                                ui.close();
                            }
                            if ui.button("Risk/Reward Box (3 clicks)").clicked() {
                                self.draw_mode = DrawMode::PlacingRiskRewardP1;
                                ui.close();
                            }
                        },
                    );
                    ui.separator();

                    // ── Manage group ──
                    ui.menu_button(
                        egui::RichText::new("Manage").small().color(normal_col),
                        |ui| {
                            if ui.button("Object List...").clicked() {
                                self.show_object_list = true;
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("Undo Last (Ctrl+Z)").clicked() {
                                if let Some(c) = self.charts.get_mut(self.active_tab) {
                                    if let Some(d) = c.drawings.pop() {
                                        c.drawings_undo.push(d);
                                    }
                                }
                                ui.close();
                            }
                            if ui.button("Redo (Ctrl+Shift+Z)").clicked() {
                                if let Some(c) = self.charts.get_mut(self.active_tab) {
                                    if let Some(d) = c.drawings_undo.pop() {
                                        c.drawings.push(d);
                                    }
                                }
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("Clear All Drawings").clicked() {
                                if let Some(c) = self.charts.get_mut(self.active_tab) {
                                    c.drawings.clear();
                                    c.drawings_undo.clear();
                                }
                                ui.close();
                            }
                        },
                    );

                    // ── Quick trashcan button (always visible) ──
                    if drawing_count > 0 {
                        if ui
                            .small_button(
                                egui::RichText::new("\u{1F5D1}")
                                    .small()
                                    .color(egui::Color32::from_rgb(200, 80, 80)),
                            )
                            .on_hover_text("Delete last drawing (Ctrl+Z to undo)")
                            .clicked()
                        {
                            if let Some(c) = self.charts.get_mut(self.active_tab) {
                                if let Some(d) = c.drawings.pop() {
                                    c.drawing_styles.pop();
                                    c.drawings_undo.push(d);
                                }
                            }
                        }
                    }

                    ui.separator();

                    // ── Magnet (OHLC snap) toggle ──
                    let mag_color = if self.snap_enabled {
                        egui::Color32::from_rgb(26, 188, 156)
                    } else {
                        egui::Color32::from_rgb(100, 100, 110)
                    };
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("\u{1F9F2}").small().color(mag_color),
                            )
                            .min_size(egui::vec2(24.0, 20.0)),
                        )
                        .on_hover_text(if self.snap_enabled {
                            "Magnet ON (OHLC snap)"
                        } else {
                            "Magnet OFF"
                        })
                        .clicked()
                    {
                        self.snap_enabled = !self.snap_enabled;
                    }

                    // Cross-TF sync toggle
                    let xtf_color = if self.drawings_cross_tf {
                        egui::Color32::from_rgb(26, 188, 156)
                    } else {
                        egui::Color32::from_rgb(100, 100, 110)
                    };
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("TF").small().color(xtf_color))
                                .min_size(egui::vec2(24.0, 20.0)),
                        )
                        .on_hover_text(if self.drawings_cross_tf {
                            "Cross-TF drawings ON"
                        } else {
                            "Cross-TF drawings OFF"
                        })
                        .clicked()
                    {
                        self.drawings_cross_tf = !self.drawings_cross_tf;
                    }

                    // ── Line width selector ──
                    let widths = [1.0_f32, 1.5, 2.0, 3.0, 4.0];
                    for w in &widths {
                        let is_sel = (self.draw_width - w).abs() < 0.01;
                        let col = if is_sel {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::from_rgb(80, 80, 90)
                        };
                        let lbl = format!(
                            "{}px",
                            if *w == w.round() {
                                format!("{}", *w as u32)
                            } else {
                                format!("{:.1}", w)
                            }
                        );
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new(&lbl).small().color(col))
                                    .min_size(egui::vec2(28.0, 20.0)),
                            )
                            .clicked()
                        {
                            self.draw_width = *w;
                        }
                    }

                    // ── Line style selector ──
                    let styles = [
                        (LineStyle::Solid, "━"),
                        (LineStyle::Dashed, "╌"),
                        (LineStyle::Dotted, "┈"),
                    ];
                    for (s, lbl) in &styles {
                        let is_sel = self.draw_line_style == *s;
                        let col = if is_sel {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::from_rgb(80, 80, 90)
                        };
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new(*lbl).small().color(col))
                                    .min_size(egui::vec2(24.0, 20.0)),
                            )
                            .clicked()
                        {
                            self.draw_line_style = *s;
                        }
                    }

                    // ── Color picker (pre-placement) ──
                    ui.separator();
                    let colors = [
                        ("W", egui::Color32::WHITE),
                        ("Y", egui::Color32::from_rgb(255, 200, 50)),
                        ("G", egui::Color32::from_rgb(0, 200, 100)),
                        ("R", egui::Color32::from_rgb(220, 50, 50)),
                        ("C", egui::Color32::from_rgb(0, 188, 212)),
                        ("M", egui::Color32::from_rgb(200, 50, 200)),
                        ("O", egui::Color32::from_rgb(255, 140, 50)),
                        ("B", egui::Color32::from_rgb(80, 120, 255)),
                    ];
                    for (lbl, col) in &colors {
                        let is_sel = self.draw_color == *col;
                        let btn = egui::Button::new(
                            egui::RichText::new(*lbl).small().color(*col).strong(),
                        )
                        .min_size(egui::vec2(20.0, 20.0))
                        .fill(if is_sel {
                            egui::Color32::from_rgb(40, 40, 60)
                        } else {
                            egui::Color32::TRANSPARENT
                        });
                        if ui.add(btn).clicked() {
                            self.draw_color = *col;
                        }
                    }

                    // ── Follow latest toggle ──
                    ui.separator();
                    let follow_col = if self.follow_latest {
                        egui::Color32::from_rgb(0, 200, 200)
                    } else {
                        egui::Color32::from_rgb(80, 80, 90)
                    };
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("⟫").small().color(follow_col))
                                .min_size(egui::vec2(22.0, 20.0)),
                        )
                        .on_hover_text("Follow latest bar (auto-scroll)")
                        .clicked()
                    {
                        self.follow_latest = !self.follow_latest;
                    }

                    // ── Status ──
                    if dm != DrawMode::None {
                        ui.separator();
                        let mode_name = match dm {
                            DrawMode::PlacingHLine => "HLine: click price level",
                            DrawMode::PlacingVLine => "VLine: click bar position",
                            DrawMode::PlacingTrendP1 => "Trendline: click start",
                            DrawMode::PlacingTrendP2 { .. } => "Trendline: click end",
                            DrawMode::PlacingRayP1 => "Ray: click origin",
                            DrawMode::PlacingRayP2 { .. } => "Ray: click direction",
                            DrawMode::PlacingRectP1 => "Rect: click corner 1",
                            DrawMode::PlacingRectP2 { .. } => "Rect: click corner 2",
                            DrawMode::PlacingChannelP1 => "Channel: click point 1 of 3",
                            DrawMode::PlacingChannelP2 { .. } => "Channel: click point 2 of 3",
                            DrawMode::PlacingChannelP3 { .. } => {
                                "Channel: click point 3 of 3 (width)"
                            }
                            DrawMode::PlacingFiboP1 => "Fib: click start",
                            DrawMode::PlacingFiboP2 { .. } => "Fib: click end",
                            DrawMode::PlacingExtLineP1 => "Ext Line: click P1",
                            DrawMode::PlacingExtLineP2 { .. } => "Ext Line: click P2",
                            DrawMode::PlacingHRay => "HRay: click start point",
                            DrawMode::PlacingCrossLine => "CrossLine: click intersection",
                            DrawMode::PlacingArrowP1 => "Arrow: click start",
                            DrawMode::PlacingArrowP2 { .. } => "Arrow: click end",
                            DrawMode::PlacingInfoLineP1 => "Info: click start",
                            DrawMode::PlacingInfoLineP2 { .. } => "Info: click end",
                            DrawMode::PlacingPitchforkP1 => "Pitchfork: click point 1 of 3 (pivot)",
                            DrawMode::PlacingPitchforkP2 { .. } => "Pitchfork: click point 2 of 3",
                            DrawMode::PlacingPitchforkP3 { .. } => "Pitchfork: click point 3 of 3",
                            DrawMode::PlacingFiboExtP1 => "Fib Ext: click point 1 of 3",
                            DrawMode::PlacingFiboExtP2 { .. } => "Fib Ext: click point 2 of 3",
                            DrawMode::PlacingFiboExtP3 { .. } => "Fib Ext: click P3",
                            DrawMode::PlacingGannFan => "Gann: click origin",
                            DrawMode::PlacingLongPosP1 => "Long: click entry",
                            DrawMode::PlacingLongPosP2 { .. } => "Long: click stop",
                            DrawMode::PlacingLongPosP3 { .. } => "Long: click target",
                            DrawMode::PlacingShortPosP1 => "Short: click entry",
                            DrawMode::PlacingShortPosP2 { .. } => "Short: click stop",
                            DrawMode::PlacingShortPosP3 { .. } => "Short: click target",
                            DrawMode::PlacingPriceRangeP1 => "Range: click P1",
                            DrawMode::PlacingPriceRangeP2 { .. } => "Range: click P2",
                            DrawMode::PlacingTextLabel => "Text: click to place label",
                            DrawMode::PlacingArrowMarkerUp => "Arrow Up: click to place",
                            DrawMode::PlacingArrowMarkerDown => "Arrow Down: click to place",
                            DrawMode::PlacingEllipseP1 => "Ellipse: click corner 1",
                            DrawMode::PlacingEllipseP2 { .. } => "Ellipse: click corner 2",
                            DrawMode::PlacingTriangleP1 => "Triangle: click P1",
                            DrawMode::PlacingTriangleP2 { .. } => "Triangle: click P2",
                            DrawMode::PlacingTriangleP3 { .. } => "Triangle: click P3",
                            DrawMode::PlacingTrendAngleP1 => "Angle: click start",
                            DrawMode::PlacingTrendAngleP2 { .. } => "Angle: click end",
                            DrawMode::PlacingParallelChP1 => "Parallel Ch: click P1",
                            DrawMode::PlacingParallelChP2 { .. } => {
                                "Parallel Ch: click P2 (offset from midline)"
                            }
                            DrawMode::PlacingFibChannelP1 => "Fib Ch: click P1",
                            DrawMode::PlacingFibChannelP2 { .. } => "Fib Ch: click P2",
                            DrawMode::PlacingFibChannelP3 { .. } => "Fib Ch: click width",
                            DrawMode::PlacingFibTimeZones => "Fib Time: click start",
                            DrawMode::PlacingPriceLabel => "PriceLabel: click price level",
                            DrawMode::PlacingCalloutP1 => "Callout: click anchor",
                            DrawMode::PlacingCalloutP2 { .. } => "Callout: click label pos",
                            DrawMode::PlacingHighlighterP1 => "Highlighter: click corner 1",
                            DrawMode::PlacingHighlighterP2 { .. } => "Highlighter: click corner 2",
                            DrawMode::PlacingCrossMarker => "CrossMarker: click to place",
                            DrawMode::PlacingPolyline => "Polyline: click points, dbl-click end",
                            DrawMode::PlacingAnchorNote => "AnchorNote: click to place",
                            DrawMode::PlacingRegressionChP1 => "Regression: click start",
                            DrawMode::PlacingRegressionChP2 { .. } => "Regression: click end",
                            DrawMode::PlacingGannBoxP1 => "Gann Box: click corner 1",
                            DrawMode::PlacingGannBoxP2 { .. } => "Gann Box: click corner 2",
                            DrawMode::PlacingElliottWave => "Elliott: click swing points (5)",
                            DrawMode::PlacingAbcCorrection => "ABC: click swing points (3)",
                            DrawMode::PlacingDateRangeP1 => "Date Range: click start",
                            DrawMode::PlacingDateRangeP2 { .. } => "Date Range: click end",
                            DrawMode::PlacingDatePriceRangeP1 => "Date+Price: click start",
                            DrawMode::PlacingDatePriceRangeP2 { .. } => "Date+Price: click end",
                            DrawMode::PlacingHeadShoulders => "H&S: click points (5)",
                            DrawMode::PlacingXabcdPattern => "XABCD: click points (5)",
                            DrawMode::PlacingBrush => "Brush: click-drag to draw",
                            DrawMode::PlacingSchiffPitchforkP1 => "Schiff Fork: click pivot",
                            DrawMode::PlacingSchiffPitchforkP2 { .. } => "Schiff Fork: click P2",
                            DrawMode::PlacingSchiffPitchforkP3 { .. } => "Schiff Fork: click P3",
                            DrawMode::PlacingModSchiffPitchforkP1 => "Mod Schiff: click pivot",
                            DrawMode::PlacingModSchiffPitchforkP2 { .. } => "Mod Schiff: click P2",
                            DrawMode::PlacingModSchiffPitchforkP3 { .. } => "Mod Schiff: click P3",
                            DrawMode::PlacingCyclicLinesP1 => "Cyclic: click start",
                            DrawMode::PlacingCyclicLinesP2 { .. } => "Cyclic: click end (interval)",
                            DrawMode::PlacingSineWaveP1 => "Sine: click start",
                            DrawMode::PlacingSineWaveP2 { .. } => "Sine: click end (period/amp)",
                            DrawMode::PlacingEmoji => "Emoji: click to place",
                            DrawMode::PlacingFlag => "Flag: click to place",
                            DrawMode::PlacingBalloonP1 => "Balloon: click anchor",
                            DrawMode::PlacingBalloonP2 { .. } => "Balloon: click label pos",
                            DrawMode::PlacingSessionBreak => "Session Break: click",
                            DrawMode::PlacingMagnetLevel => "Magnet: click price level",
                            DrawMode::PlacingRiskRewardP1 => "R:R Box: click entry",
                            DrawMode::PlacingRiskRewardP2 { .. } => "R:R Box: click stop",
                            DrawMode::PlacingRiskRewardP3 { .. } => "R:R Box: click target",
                            DrawMode::PlacingFibCircleP1 => "Fib Circle: click center",
                            DrawMode::PlacingFibCircleP2 { .. } => "Fib Circle: click radius",
                            DrawMode::PlacingArcP1 => "Arc: click start",
                            DrawMode::PlacingArcP2 { .. } => "Arc: click midpoint",
                            DrawMode::PlacingArcP3 { .. } => "Arc: click end",
                            DrawMode::PlacingCurveP1 => "Curve: click start",
                            DrawMode::PlacingCurveP2 { .. } => "Curve: click ctrl1",
                            DrawMode::PlacingCurveP3 { .. } => "Curve: click ctrl2",
                            DrawMode::PlacingCurveP4 { .. } => "Curve: click end",
                            DrawMode::PlacingPath => "Path: click points, dbl-click end",
                            DrawMode::PlacingForecastP1 => "Forecast: click start",
                            DrawMode::PlacingForecastP2 { .. } => "Forecast: click end",
                            DrawMode::PlacingGhostFeedP1 => "Ghost Feed: click start",
                            DrawMode::PlacingGhostFeedP2 { .. } => "Ghost Feed: click end",
                            DrawMode::PlacingSignpost => "Signpost: click to place",
                            DrawMode::PlacingRulerP1 => "Ruler: click start",
                            DrawMode::PlacingRulerP2 { .. } => "Ruler: click end",
                            DrawMode::PlacingTimeCycleP1 => "Time Cycle: click start",
                            DrawMode::PlacingTimeCycleP2 { .. } => {
                                "Time Cycle: click end (interval)"
                            }
                            DrawMode::PlacingSpeedFanP1 => "Speed Fan: click low",
                            DrawMode::PlacingSpeedFanP2 { .. } => "Speed Fan: click high",
                            DrawMode::PlacingSpeedFanP3 { .. } => "Speed Fan: click time ref",
                            DrawMode::PlacingSpeedArcP1 => "Speed Arc: click low",
                            DrawMode::PlacingSpeedArcP2 { .. } => "Speed Arc: click high",
                            DrawMode::PlacingSpeedArcP3 { .. } => "Speed Arc: click time ref",
                            DrawMode::PlacingFibSpiralP1 => "Fib Spiral: click center",
                            DrawMode::PlacingFibSpiralP2 { .. } => "Fib Spiral: click radius",
                            DrawMode::PlacingRotatedRectP1 => "Rotated Rect: click P1",
                            DrawMode::PlacingRotatedRectP2 { .. } => "Rotated Rect: click P2",
                            DrawMode::PlacingRotatedRectP3 { .. } => "Rotated Rect: click height",
                            DrawMode::PlacingAnchoredVwap => "Anchored VWAP: click anchor bar",
                            DrawMode::PlacingTrendChannelP1 => "Trend Channel: click P1",
                            DrawMode::PlacingTrendChannelP2 { .. } => "Trend Channel: click P2",
                            DrawMode::PlacingTrendChannelP3 { .. } => "Trend Channel: click width",
                            DrawMode::PlacingInsidePitchforkP1 => "Inside Pitchfork: click pivot",
                            DrawMode::PlacingInsidePitchforkP2 { .. } => {
                                "Inside Pitchfork: click P2"
                            }
                            DrawMode::PlacingInsidePitchforkP3 { .. } => {
                                "Inside Pitchfork: click P3"
                            }
                            DrawMode::PlacingFibWedgeP1 => "Fib Wedge: click apex",
                            DrawMode::PlacingFibWedgeP2 { .. } => "Fib Wedge: click P2",
                            DrawMode::PlacingFibWedgeP3 { .. } => "Fib Wedge: click P3",
                            DrawMode::PlacingPriceNote => "Price Note: click price level",
                            DrawMode::PlacingMeasureToolP1 => "Measure: click start",
                            DrawMode::PlacingMeasureToolP2 { .. } => "Measure: click end",
                            DrawMode::PlacingAnchoredText => "Anchored Text: click",
                            DrawMode::PlacingComment => "Comment: click",
                            DrawMode::PlacingArrowMarkerLeft => "Arrow Left: click",
                            DrawMode::PlacingArrowMarkerRight => "Arrow Right: click",
                            DrawMode::PlacingCircleP1 => "Circle: click center",
                            DrawMode::PlacingCircleP2 { .. } => "Circle: click radius",
                            DrawMode::PlacingPitchFanP1 => "Pitch Fan: click start",
                            DrawMode::PlacingPitchFanP2 { .. } => "Pitch Fan: click end",
                            DrawMode::PlacingTrendFibTimeP1 => "Trend Fib Time: click start",
                            DrawMode::PlacingTrendFibTimeP2 { .. } => "Trend Fib Time: click end",
                            DrawMode::PlacingGannSquareP1 => "Gann Square: click corner 1",
                            DrawMode::PlacingGannSquareP2 { .. } => "Gann Square: click corner 2",
                            DrawMode::PlacingGannSquareFixedP1 => {
                                "Gann Square Fixed: click corner 1"
                            }
                            DrawMode::PlacingGannSquareFixedP2 { .. } => {
                                "Gann Square Fixed: click corner 2"
                            }
                            DrawMode::PlacingBarsPatternP1 => "Bars Pattern: click start",
                            DrawMode::PlacingBarsPatternP2 { .. } => "Bars Pattern: click end",
                            DrawMode::PlacingProjectionP1 => "Projection: click start",
                            DrawMode::PlacingProjectionP2 { .. } => "Projection: click end",
                            DrawMode::PlacingDoubleCurveP1 => "Double Curve: click start",
                            DrawMode::PlacingDoubleCurveP2 { .. } => "Double Curve: click end",
                            DrawMode::PlacingTrianglePattern => "Triangle Pattern: click (3)",
                            DrawMode::PlacingThreeDrives => "Three Drives: click (3)",
                            DrawMode::PlacingElliottDouble => "Elliott WXY: click (3)",
                            DrawMode::PlacingAbcdPattern => "ABCD: click (4)",
                            DrawMode::PlacingCypherPattern => "Cypher: click (5)",
                            DrawMode::PlacingElliottTriangle => "Elliott ABCDE: click (5)",
                            DrawMode::PlacingElliottTripleCombo => "Elliott WXYXZ: click (5)",
                            DrawMode::Eraser => "ERASER: click near drawing to delete",
                            DrawMode::None => "",
                        };
                        ui.label(egui::RichText::new(mode_name).small().color(active_col));
                        if ui.small_button("Esc").clicked() {
                            self.draw_mode = DrawMode::None;
                        }
                    }

                    // ── Drawing count ──
                    if drawing_count > 0 {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("{} drawings", drawing_count))
                                    .small()
                                    .color(egui::Color32::from_rgb(80, 80, 100)),
                            );
                        });
                    }
                });
            });
    }
}
