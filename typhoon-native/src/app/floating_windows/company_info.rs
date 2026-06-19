use super::*;

impl TyphooNApp {
    pub(super) fn render_company_info_window(&mut self, ctx: &egui::Context) {
        if !self.show_company_info_window {
            return;
        }

        let title = if self.company_info_symbol.is_empty() {
            "Company Info".to_string()
        } else {
            format!("Company Info — {}", self.company_info_symbol)
        };

        egui::Window::new(title)
            .open(&mut self.show_company_info_window)
            .resizable(true)
            .default_size([520.0, 420.0])
            .max_size([720.0, 680.0])
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.label(&self.company_info_text);
                    });
            });
    }
}
