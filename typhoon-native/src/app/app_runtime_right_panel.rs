use super::*;

#[allow(deprecated)]
impl TyphooNApp {
    pub(super) fn render_right_panel(&mut self, root_ui: &mut egui::Ui) {
        // ── right panel (collapsible sections — all visible, individually expandable) ──
        egui::Panel::right("right_panel")
            .min_size(320.0)
            .max_size(760.0)
            // Multi-account broker rows are dense trading controls, not a toy
            // sidebar. Start wider and allow the user to expand further; rows
            // below still wrap so loaded account state cannot shove the navbar
            // off its left edge.
            .default_size(560.0)
            .resizable(true)
            .show(root_ui, |ui| {
                let panel_width = ui.available_width();
                egui::ScrollArea::vertical()
                    .id_salt("right_panel_vertical_scroll")
                    // Some loaded broker/account rows are intentionally wider than
                    // the navbar. A vertical ScrollArea still persists an x-offset;
                    // if a wide row or a horizontal wheel event leaves that offset
                    // non-zero, the whole navbar renders shifted/clipped on the next
                    // loaded frame. Keep the navbar left edge pinned and let long
                    // rows clip/wrap locally instead of moving the panel contents.
                    .horizontal_scroll_offset(0.0)
                    .max_width(panel_width)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_max_width(panel_width);
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
