use super::*;

#[allow(deprecated)]
impl TyphooNApp {
    pub(super) fn render_right_panel_orders_section(&mut self, ui: &mut egui::Ui) {
        // ── Orders Section ────────────────────────────────────
        let alpaca_live =
            self.show_alpaca_positions && self.broker_connected && !self.live_orders.is_empty();
        if alpaca_live {
            let ord_count = self.live_orders.len();
            let (ord_stale_lbl, ord_stale_col) = self.staleness_badge(self.orders_last_update_ts);
            let ord_header = format!("☰ Orders ({})  •  {}", ord_count, ord_stale_lbl);
            let orders_section = egui::CollapsingHeader::new(
                egui::RichText::new(ord_header)
                    .strong()
                    .small()
                    .color(ord_stale_col),
            )
            .id_salt("orders_section")
            .default_open(self.right_orders_open)
            .show(ui, |ui| {
                ui.add_space(4.0);
                let mut cancel_id: Option<String> = None;
                let mut lo_action = SymbolAction::None;
                for order in &self.live_orders {
                    ui.horizontal(|ui| {
                        let (_, act) = symbol_label_with_menu(
                            ui,
                            &order.symbol,
                            egui::RichText::new(&order.symbol).small().strong(),
                        );
                        if !matches!(act, SymbolAction::None) {
                            lo_action = act;
                        }
                        let side_c = if order.side == "buy" { UP } else { DOWN };
                        ui.label(egui::RichText::new(&order.side).color(side_c).small());
                        ui.label(
                            egui::RichText::new(&order.order_type)
                                .color(AXIS_TEXT)
                                .small(),
                        );
                        if self.broker_connected {
                            if ui
                                .small_button(egui::RichText::new("X").color(DOWN))
                                .on_hover_text("Cancel order")
                                .clicked()
                            {
                                cancel_id = Some(order.id.clone());
                            }
                        }
                    });
                    ui.label(
                        egui::RichText::new(format!("qty: {} | {}", order.qty, order.status))
                            .color(ACCENT)
                            .small(),
                    );
                    ui.separator();
                }
                if let Some(oid) = cancel_id {
                    let _ = self
                        .broker_tx
                        .send(BrokerCmd::AlpacaCancelOrder { order_id: oid });
                }
                if !matches!(lo_action, SymbolAction::None) {
                    self.deferred_symbol_action = lo_action;
                }
            });
            self.right_orders_open = orders_section.fully_open();
            self.handle_right_panel_section_drag(
                ui,
                RightPanelSectionId::Orders,
                &orders_section.header_response,
            );
        }
    }
}
