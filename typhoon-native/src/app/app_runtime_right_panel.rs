use super::*;

#[allow(deprecated)]
impl TyphooNApp {
    pub(super) fn render_right_panel(&mut self, ctx: &egui::Context) {
        // ── right panel (collapsible sections — all visible, individually expandable) ──
        egui::Panel::right("right_panel")
            .min_size(220.0)
            .max_size(500.0)
            // Open at the widest supported navbar width. Users can still
            // collapse manually; individual sections handle narrower layouts.
            .default_size(500.0)
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        let right_panel_order = self.normalized_right_panel_order();
                        for section in right_panel_order {
                            match section {
                                RightPanelSectionId::Trading => {
                                    self.render_right_panel_trading_section(ui);
                                }
                                RightPanelSectionId::Positions => {
                                    self.render_right_panel_positions_section(ui);
                                }
                                RightPanelSectionId::RecentFills => {
                                    self.render_right_panel_recent_fills_section(ui);
                                }
                                RightPanelSectionId::Orders => {
                                    self.render_right_panel_orders_section(ui);
                                }
                                RightPanelSectionId::Watchlist => {
                                    self.render_right_panel_watchlist_section(ui);
                                }
                                RightPanelSectionId::Risk => {
                                    self.render_right_panel_risk_section(ui);
                                }
                                RightPanelSectionId::News => {
                                    self.render_right_panel_news_section(ui);
                                }
                                RightPanelSectionId::MtfGrid => {
                                    self.render_right_panel_mtf_grid_section(ui);
                                }
                            }
                        }
                        if self.dragging_right_panel_section.is_some()
                            && !ui.input(|i| i.pointer.primary_down())
                        {
                            self.dragging_right_panel_section = None;
                        }
                    });
            });
    }
}
