use super::*;

/// Format an order price field (Alpaca sends prices as strings) via the shared
/// price formatter so it reads like the rest of the panel.
fn fmt_order_price(s: &Option<String>) -> Option<String> {
    let s = s.as_deref()?.trim();
    if s.is_empty() {
        return None;
    }
    Some(
        s.parse::<f64>()
            .ok()
            .map(format_price)
            .unwrap_or_else(|| s.to_string()),
    )
}

/// Compact price descriptor for an order — surfaces the take-profit limit and
/// stop-loss stop levels that the row previously dropped entirely (the user
/// placed bracket orders with SL/TP and saw only "sell limit").
fn order_price_descriptor(o: &OrderInfo) -> Option<String> {
    let limit = fmt_order_price(&o.limit_price);
    let stop = fmt_order_price(&o.stop_price);
    match (stop, limit) {
        (Some(s), Some(l)) => Some(format!("stop {s} → {l}")),
        (Some(s), None) => Some(format!("stop {s}")),
        (None, Some(l)) => Some(format!("@ {l}")),
        (None, None) => o
            .trail_percent
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|p| format!("trail {p}%"))
            .or_else(|| fmt_order_price(&o.trail_price).map(|a| format!("trail {a}"))),
    }
}

/// Role of a bracket leg: a stop level is the stop-loss, a bare limit is the
/// take-profit. Returns the badge and its colour.
fn order_leg_role(o: &OrderInfo) -> (&'static str, egui::Color32) {
    if o.stop_price.as_deref().is_some_and(|s| !s.trim().is_empty()) {
        ("SL", DOWN)
    } else if o.limit_price.as_deref().is_some_and(|s| !s.trim().is_empty()) {
        ("TP", UP)
    } else {
        ("leg", AXIS_TEXT)
    }
}

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
                        // The TP limit / SL stop level — previously dropped, which
                        // is why bracket orders looked like bare "sell limit" rows.
                        if let Some(desc) = order_price_descriptor(order) {
                            ui.label(egui::RichText::new(desc).color(ACCENT).small().strong());
                        }
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
                    // Bracket legs (TP/SL children) are nested under an unfilled
                    // parent — render them so the SL/TP attached to the entry is
                    // visible before it fills.
                    if let Some(legs) = order.legs.as_ref() {
                        for leg in legs {
                            let (role, role_c) = order_leg_role(leg);
                            ui.horizontal(|ui| {
                                ui.add_space(12.0);
                                ui.label(
                                    egui::RichText::new(format!("└ {role}"))
                                        .color(role_c)
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(&leg.order_type)
                                        .color(AXIS_TEXT)
                                        .small(),
                                );
                                if let Some(desc) = order_price_descriptor(leg) {
                                    ui.label(egui::RichText::new(desc).color(role_c).small());
                                }
                            });
                        }
                    }
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

#[cfg(test)]
mod order_descriptor_tests {
    use super::*;

    fn ord(order_type: &str, limit: Option<&str>, stop: Option<&str>) -> OrderInfo {
        OrderInfo {
            id: "x".into(),
            symbol: "HKIT".into(),
            qty: "8".into(),
            filled_qty: "0".into(),
            side: "sell".into(),
            order_type: order_type.into(),
            order_class: None,
            status: "new".into(),
            limit_price: limit.map(|s| s.into()),
            stop_price: stop.map(|s| s.into()),
            trail_price: None,
            trail_percent: None,
            created_at: String::new(),
            filled_at: None,
            filled_avg_price: None,
            legs: None,
        }
    }

    #[test]
    fn descriptor_surfaces_tp_sl_levels_and_roles() {
        // Take-profit limit, stop-loss stop, and a stop-limit carry their levels.
        assert!(order_price_descriptor(&ord("limit", Some("0.35"), None))
            .unwrap()
            .starts_with("@ "));
        assert!(order_price_descriptor(&ord("stop", None, Some("0.25")))
            .unwrap()
            .starts_with("stop "));
        assert!(order_price_descriptor(&ord("stop_limit", Some("0.24"), Some("0.25")))
            .unwrap()
            .contains('→'));
        // A plain market order has no price to show.
        assert!(order_price_descriptor(&ord("market", None, None)).is_none());
        // Bracket leg roles: stop ⇒ SL, bare limit ⇒ TP.
        assert_eq!(order_leg_role(&ord("limit", Some("0.35"), None)).0, "TP");
        assert_eq!(order_leg_role(&ord("stop", None, Some("0.25"))).0, "SL");
    }
}
