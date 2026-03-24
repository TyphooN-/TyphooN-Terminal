//! Main application state and egui rendering.

use eframe::egui;

/// Main application state.
pub struct TyphooNApp {
    /// Status message for the bottom bar.
    status: String,
    /// Command palette input.
    command_input: String,
    /// Whether command palette is open.
    command_open: bool,
}

impl TyphooNApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            status: "TyphooN Terminal — Pure Rust GPU".to_string(),
            command_input: String::new(),
            command_open: false,
        }
    }

    fn dark_theme() -> egui::Visuals {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(10, 10, 20);
        visuals.window_fill = egui::Color32::from_rgb(15, 15, 25);
        visuals.extreme_bg_color = egui::Color32::from_rgb(5, 5, 15);
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(20, 20, 30);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(25, 25, 40);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(35, 35, 55);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(30, 50, 120);
        visuals.selection.bg_fill = egui::Color32::from_rgb(30, 60, 140);
        visuals
    }
}

impl eframe::App for TyphooNApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply dark theme
        ctx.set_visuals(Self::dark_theme());

        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Connect to Broker...").clicked() { ui.close_menu(); }
                    if ui.button("Settings").clicked() { ui.close_menu(); }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui.button("MTF Grid").clicked() { ui.close_menu(); }
                    if ui.button("Single Chart").clicked() { ui.close_menu(); }
                    ui.separator();
                    if ui.button("Indicators").clicked() { ui.close_menu(); }
                    if ui.button("Drawing Tools").clicked() { ui.close_menu(); }
                });
                ui.menu_button("Trading", |ui| {
                    if ui.button("Open Trade").clicked() { ui.close_menu(); }
                    if ui.button("Close All").clicked() { ui.close_menu(); }
                    if ui.button("Close Partial").clicked() { ui.close_menu(); }
                    ui.separator();
                    if ui.button("Set SL").clicked() { ui.close_menu(); }
                    if ui.button("Set TP").clicked() { ui.close_menu(); }
                    ui.separator();
                    if ui.button("Open MG (Martingale Hedge)").clicked() { ui.close_menu(); }
                    if ui.button("Buy Lines").clicked() { ui.close_menu(); }
                    if ui.button("Sell Lines").clicked() { ui.close_menu(); }
                });
                ui.menu_button("Tools", |ui| {
                    if ui.button("DARWIN Accounts").clicked() { ui.close_menu(); }
                    if ui.button("DARWIN Portfolio").clicked() { ui.close_menu(); }
                    if ui.button("Symbol Overlap").clicked() { ui.close_menu(); }
                    ui.separator();
                    if ui.button("Backtest").clicked() { ui.close_menu(); }
                    if ui.button("Screener").clicked() { ui.close_menu(); }
                    if ui.button("Optimizer").clicked() { ui.close_menu(); }
                    ui.separator();
                    if ui.button("Risk Calculator").clicked() { ui.close_menu(); }
                    if ui.button("VaR Multiplier").clicked() { ui.close_menu(); }
                    if ui.button("Margin Monitor").clicked() { ui.close_menu(); }
                });
                ui.menu_button("Research", |ui| {
                    if ui.button("News & Events").clicked() { ui.close_menu(); }
                    if ui.button("Economic Calendar").clicked() { ui.close_menu(); }
                    if ui.button("SEC Filings").clicked() { ui.close_menu(); }
                    if ui.button("Insider Trades").clicked() { ui.close_menu(); }
                    ui.separator();
                    if ui.button("Fundamentals").clicked() { ui.close_menu(); }
                    if ui.button("Analyst Ratings").clicked() { ui.close_menu(); }
                    if ui.button("Institutional Holders").clicked() { ui.close_menu(); }
                });
                ui.menu_button("Analysis", |ui| {
                    if ui.button("Correlation Matrix").clicked() { ui.close_menu(); }
                    if ui.button("Seasonals").clicked() { ui.close_menu(); }
                    if ui.button("Monte Carlo VaR").clicked() { ui.close_menu(); }
                    if ui.button("Stress Test").clicked() { ui.close_menu(); }
                    ui.separator();
                    if ui.button("Volume Profile").clicked() { ui.close_menu(); }
                    if ui.button("Order Flow").clicked() { ui.close_menu(); }
                    if ui.button("Bookmap Heatmap").clicked() { ui.close_menu(); }
                });
                ui.separator();
                ui.label(
                    egui::RichText::new("TyphooN Terminal — Pure Rust GPU")
                        .color(egui::Color32::from_rgb(76, 175, 80))
                        .strong(),
                );
            });
        });

        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(&self.status)
                        .color(egui::Color32::from_rgb(136, 136, 136))
                        .small(),
                );
            });
        });

        // Right panel (positions, orders, risk)
        egui::SidePanel::right("right_panel")
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading("Positions");
                ui.separator();
                ui.label(
                    egui::RichText::new("Connect to broker to see positions")
                        .color(egui::Color32::from_rgb(136, 136, 136)),
                );
                ui.add_space(20.0);
                ui.heading("Orders");
                ui.separator();
                ui.label(
                    egui::RichText::new("No open orders")
                        .color(egui::Color32::from_rgb(136, 136, 136)),
                );
            });

        // Central panel (chart area)
        egui::CentralPanel::default().show(ctx, |ui| {
            // Command palette (Ctrl+K or /)
            if ctx.input(|i| i.key_pressed(egui::Key::Slash)) ||
               ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::K)) {
                self.command_open = !self.command_open;
                self.command_input.clear();
            }

            if self.command_open {
                egui::Window::new("Command Palette")
                    .anchor(egui::Align2::CENTER_TOP, [0.0, 50.0])
                    .fixed_size([600.0, 40.0])
                    .title_bar(false)
                    .show(ctx, |ui| {
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.command_input)
                                .desired_width(580.0)
                                .hint_text("Type a command...")
                                .font(egui::TextStyle::Monospace),
                        );
                        response.request_focus();
                        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                            self.command_open = false;
                        }
                    });
            }

            // Chart placeholder — this will be replaced with wgpu custom rendering
            let available = ui.available_size();
            let (rect, _response) = ui.allocate_exact_size(available, egui::Sense::click_and_drag());

            let painter = ui.painter_at(rect);

            // Dark background
            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(5, 5, 15));

            // Grid lines
            let grid_color = egui::Color32::from_rgb(30, 30, 40);
            let steps = 10;
            for i in 0..=steps {
                let x = rect.left() + rect.width() * (i as f32 / steps as f32);
                painter.line_segment(
                    [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                    egui::Stroke::new(0.5, grid_color),
                );
                let y = rect.top() + rect.height() * (i as f32 / steps as f32);
                painter.line_segment(
                    [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                    egui::Stroke::new(0.5, grid_color),
                );
            }

            // Center text
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Chart viewport — wgpu renderer coming next",
                egui::FontId::proportional(18.0),
                egui::Color32::from_rgb(100, 100, 100),
            );

            // Engine info
            painter.text(
                egui::pos2(rect.left() + 10.0, rect.bottom() - 20.0),
                egui::Align2::LEFT_BOTTOM,
                "Pure Rust → wgpu | Zero JS | Zero WebKit",
                egui::FontId::monospace(11.0),
                egui::Color32::from_rgb(76, 175, 80),
            );
        });

        // Request continuous repainting for real-time updates
        ctx.request_repaint();
    }
}
