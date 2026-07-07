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
    if o.stop_price
        .as_deref()
        .is_some_and(|s| !s.trim().is_empty())
    {
        ("SL", DOWN)
    } else if o
        .limit_price
        .as_deref()
        .is_some_and(|s| !s.trim().is_empty())
    {
        ("TP", UP)
    } else {
        ("leg", AXIS_TEXT)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OrderGroupKey {
    symbol: String,
    side: String,
    order_type: String,
    order_class: Option<String>,
    price_desc: Option<String>,
    status: String,
    /// Non-empty for orders that must stay visually separate. This keeps bracket
    /// parents / leg-bearing orders from being hidden inside an aggregate row.
    unique_id: Option<String>,
}

struct OrderDisplayGroup<'a> {
    key: OrderGroupKey,
    orders: Vec<&'a OrderInfo>,
    total_qty: f64,
    all_qty_numeric: bool,
}

impl<'a> OrderDisplayGroup<'a> {
    fn primary(&self) -> &'a OrderInfo {
        self.orders[0]
    }
}

fn parse_order_qty_value(value: &str) -> Option<f64> {
    let qty = value.trim().parse::<f64>().ok()?;
    (qty.is_finite() && qty >= 0.0).then_some(qty)
}

fn fmt_order_qty_value(qty: f64) -> String {
    if qty.fract().abs() < 0.000_000_01 {
        return format!("{qty:.0}");
    }
    let mut s = format!("{qty:.8}");
    while s.contains('.') && s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    s
}

fn order_group_key(order: &OrderInfo) -> OrderGroupKey {
    let has_legs = order.legs.as_ref().is_some_and(|legs| !legs.is_empty());
    OrderGroupKey {
        symbol: order.symbol.trim().to_ascii_uppercase(),
        side: order.side.trim().to_ascii_lowercase(),
        order_type: order.order_type.trim().to_ascii_lowercase(),
        order_class: order
            .order_class
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_ascii_lowercase),
        price_desc: order_price_descriptor(order),
        status: order.status.trim().to_ascii_lowercase(),
        unique_id: has_legs.then(|| order.id.clone()),
    }
}

fn order_display_groups(orders: &[OrderInfo]) -> Vec<OrderDisplayGroup<'_>> {
    let mut groups: Vec<OrderDisplayGroup<'_>> = Vec::new();
    for order in orders {
        let key = order_group_key(order);
        let qty = parse_order_qty_value(&order.qty);
        if let Some(group) = groups.iter_mut().find(|group| group.key == key) {
            group.orders.push(order);
            if let Some(qty) = qty {
                group.total_qty += qty;
            } else {
                group.all_qty_numeric = false;
            }
        } else {
            groups.push(OrderDisplayGroup {
                key,
                orders: vec![order],
                total_qty: qty.unwrap_or(0.0),
                all_qty_numeric: qty.is_some(),
            });
        }
    }
    groups
}

#[allow(deprecated)]
impl TyphooNApp {
    pub(super) fn render_right_panel_orders_section(&mut self, ui: &mut egui::Ui) {
        // ── Orders Section ────────────────────────────────────
        let alpaca_orders_available = self.alpaca_enabled;
        let kr_orders_available = self.kraken_enabled;
        let mut alpaca_order_groups: Vec<AccountOrders> = if self.show_alpaca_orders {
            if !self.alpaca_account_orders.is_empty() {
                self.alpaca_account_orders.clone()
            } else if !self.live_orders.is_empty() {
                vec![AccountOrders {
                    account_id: self.alpaca_primary_account_id.clone(),
                    label: self
                        .alpaca_account_roster
                        .iter()
                        .find(|account| account.is_primary)
                        .map(|account| account.label.clone())
                        .unwrap_or_else(|| "Alpaca 1".to_string()),
                    is_primary: true,
                    orders: self.live_orders.clone(),
                }]
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        alpaca_order_groups.retain(|account| {
            !self
                .hidden_alpaca_order_account_ids
                .contains(&account.account_id)
        });
        let mut kraken_order_groups: Vec<KrakenAccountOrders> = if self.show_kr_orders {
            if !self.kraken_account_orders.is_empty() {
                self.kraken_account_orders.clone()
            } else if !self.kraken_open_orders.is_empty() {
                vec![KrakenAccountOrders {
                    account_id: self.kraken_primary_account_id.clone(),
                    label: self
                        .kraken_account_roster
                        .iter()
                        .find(|account| account.is_primary)
                        .map(|account| account.label.clone())
                        .unwrap_or_else(|| "Kraken".to_string()),
                    is_primary: true,
                    orders: self.kraken_open_orders.clone(),
                }]
            } else {
                self.kraken_account_roster
                    .iter()
                    .filter(|account| account.connected)
                    .map(|account| KrakenAccountOrders {
                        account_id: account.id.clone(),
                        label: account.label.clone(),
                        is_primary: account.is_primary,
                        orders: Vec::new(),
                    })
                    .collect()
            }
        } else {
            Vec::new()
        };
        kraken_order_groups.retain(|account| {
            !self
                .hidden_kraken_order_account_ids
                .contains(&account.account_id)
        });
        let alpaca_count = alpaca_order_groups
            .iter()
            .map(|account| account.orders.len())
            .sum::<usize>();
        let kr_count = kraken_order_groups
            .iter()
            .map(|account| account.orders.len())
            .sum::<usize>();
        let ord_count = alpaca_count + kr_count;
        if !alpaca_orders_available && !kr_orders_available && ord_count == 0 {
            return;
        }

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
            let alpaca_toggles: Vec<(String, String, usize, bool)> = if !self.alpaca_account_orders.is_empty() {
                self.alpaca_account_orders
                    .iter()
                    .map(|account| {
                        (
                            account.account_id.clone(),
                            account.label.clone(),
                            account.orders.len(),
                            account.is_primary,
                        )
                    })
                    .collect()
            } else {
                alpaca_order_groups
                    .iter()
                    .map(|account| {
                        (
                            account.account_id.clone(),
                            account.label.clone(),
                            account.orders.len(),
                            account.is_primary,
                        )
                    })
                    .collect()
            };
            let kraken_toggles: Vec<(String, String, usize, bool)> = if !self.kraken_account_orders.is_empty() {
                self.kraken_account_orders
                    .iter()
                    .map(|account| {
                        (
                            account.account_id.clone(),
                            account.label.clone(),
                            account.orders.len(),
                            account.is_primary,
                        )
                    })
                    .collect()
            } else {
                kraken_order_groups
                    .iter()
                    .map(|account| {
                        (
                            account.account_id.clone(),
                            account.label.clone(),
                            account.orders.len(),
                            account.is_primary,
                        )
                    })
                    .collect()
            };
            if alpaca_toggles.len() > 1 || kraken_toggles.len() > 1 || [alpaca_orders_available, kr_orders_available].into_iter().filter(|visible| *visible).count() > 1 {
                ui.horizontal_wrapped(|ui| {
                    if alpaca_orders_available && alpaca_toggles.len() <= 1 {
                        ui.checkbox(&mut self.show_alpaca_orders, egui::RichText::new("Alpaca").small());
                    }
                    for (account_id, label, count, is_primary) in alpaca_toggles {
                        let mut shown = self.show_alpaca_orders
                            && !self.hidden_alpaca_order_account_ids.contains(&account_id);
                        if ui
                            .checkbox(
                                &mut shown,
                                egui::RichText::new(format!(
                                    "{}{} ({})",
                                    label,
                                    if is_primary { " ★" } else { "" },
                                    count
                                ))
                                .small(),
                            )
                            .on_hover_text(format!("Alpaca account id: {account_id}"))
                            .changed()
                        {
                            if shown {
                                self.hidden_alpaca_order_account_ids.remove(&account_id);
                                self.show_alpaca_orders = true;
                            } else {
                                self.hidden_alpaca_order_account_ids.insert(account_id);
                            }
                        }
                    }
                    if kr_orders_available && kraken_toggles.len() <= 1 {
                        ui.checkbox(&mut self.show_kr_orders, egui::RichText::new("Kraken").small());
                    }
                    let single_kraken_account = kraken_toggles.len() <= 1;
                    if !single_kraken_account {
                        for (account_id, label, count, is_primary) in kraken_toggles {
                            let mut shown = self.show_kr_orders
                                && !self.hidden_kraken_order_account_ids.contains(&account_id);
                            if ui
                                .checkbox(
                                    &mut shown,
                                    egui::RichText::new(format!(
                                        "{}{} ({})",
                                        super::app_runtime_right_panel_positions::single_kraken_account_label(&label, single_kraken_account),
                                        super::app_runtime_right_panel_positions::primary_marker(is_primary, single_kraken_account),
                                        count
                                    ))
                                    .small(),
                                )
                                .on_hover_text(format!("Kraken account id: {account_id}"))
                                .changed()
                            {
                                if shown {
                                    self.hidden_kraken_order_account_ids.remove(&account_id);
                                    self.show_kr_orders = true;
                                } else {
                                    self.hidden_kraken_order_account_ids.insert(account_id);
                                }
                            }
                        }
                    }
                });
                ui.add_space(4.0);
            }

            let mut cancel_ids: Vec<String> = Vec::new();
            let mut kr_cancel_ids: Vec<String> = Vec::new();
            let mut lo_action = SymbolAction::None;
            let mut has_orders = false;
            for account in &alpaca_order_groups {
                ui.label(
                    egui::RichText::new(format!(
                        "{}{} ({})",
                        account.label,
                        if account.is_primary { " ★" } else { "" },
                        account.orders.len()
                    ))
                    .small()
                    .strong()
                    .color(if account.is_primary { ACCENT } else { AXIS_TEXT }),
                )
                .on_hover_text(format!("Alpaca account id: {}", account.account_id));
                if account.orders.is_empty() {
                    ui.label(egui::RichText::new("no open orders").small().color(AXIS_TEXT));
                    ui.separator();
                    continue;
                }
                has_orders = true;
                for group in order_display_groups(&account.orders) {
                    let order = group.primary();
                    ui.horizontal_wrapped(|ui| {
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
                        ui.label(egui::RichText::new(&order.order_type).color(AXIS_TEXT).small());
                        if let Some(desc) = order_price_descriptor(order) {
                            ui.label(egui::RichText::new(desc).color(ACCENT).small().strong());
                        }
                        if group.orders.len() > 1 {
                            ui.label(egui::RichText::new(format!("×{}", group.orders.len())).color(AXIS_TEXT).small().strong())
                                .on_hover_text("Identical open orders grouped by symbol, side, type, price, and status");
                        }
                        if account.is_primary && self.broker_connected {
                            if ui
                                .small_button(egui::RichText::new("X").color(DOWN))
                                .on_hover_text(if group.orders.len() > 1 { "Cancel all orders in this group" } else { "Cancel order" })
                                .clicked()
                            {
                                cancel_ids.extend(group.orders.iter().map(|order| order.id.clone()));
                            }
                        }
                    });
                    let qty_text = if group.orders.len() > 1 && group.all_qty_numeric {
                        fmt_order_qty_value(group.total_qty)
                    } else {
                        order.qty.clone()
                    };
                    let order_count_text = if group.orders.len() > 1 {
                        format!(" | {} orders", group.orders.len())
                    } else {
                        String::new()
                    };
                    ui.label(
                        egui::RichText::new(format!("qty: {}{} | {}", qty_text, order_count_text, order.status))
                            .color(ACCENT)
                            .small(),
                    );
                    if let Some(legs) = order.legs.as_ref() {
                        for leg in legs {
                            let (role, role_c) = order_leg_role(leg);
                            ui.horizontal_wrapped(|ui| {
                                ui.add_space(12.0);
                                ui.label(egui::RichText::new(format!("└ {role}")).color(role_c).small().strong());
                                ui.label(egui::RichText::new(&leg.order_type).color(AXIS_TEXT).small());
                                if let Some(desc) = order_price_descriptor(leg) {
                                    ui.label(egui::RichText::new(desc).color(role_c).small());
                                }
                            });
                        }
                    }
                    ui.separator();
                }
            }
            if self.show_kr_orders && kraken_order_groups.iter().any(|account| !account.orders.is_empty()) {
                let single_kraken_account = kraken_order_groups.len() <= 1;
                for account in &kraken_order_groups {
                    if account.orders.is_empty() {
                        continue;
                    }
                    has_orders = true;
                    ui.label(
                        egui::RichText::new(format!(
                            "{}{} ({})",
                            super::app_runtime_right_panel_positions::single_kraken_account_label(&account.label, single_kraken_account),
                            super::app_runtime_right_panel_positions::primary_marker(account.is_primary, single_kraken_account),
                            account.orders.len()
                        ))
                        .small()
                        .strong()
                        .color(if account.is_primary { ACCENT } else { AXIS_TEXT }),
                    )
                    .on_hover_text(format!("Kraken account id: {}", account.account_id));
                for order in &account.orders {
                    let remain = (order.vol - order.vol_exec).max(0.0);
                    ui.horizontal_wrapped(|ui| {
                        let (_, act) = symbol_label_with_menu(ui, &order.pair, egui::RichText::new(&order.pair).small().strong());
                        if !matches!(act, SymbolAction::None) {
                            lo_action = act;
                        }
                        let side_c = if order.r#type == "buy" { UP } else { DOWN };
                        ui.label(egui::RichText::new(&order.r#type).color(side_c).small());
                        ui.label(egui::RichText::new(&order.ordertype).color(AXIS_TEXT).small());
                        ui.label(egui::RichText::new(format!("@ {}", format_price(order.price))).color(ACCENT).small().strong());
                        if account.is_primary && ui.small_button(egui::RichText::new("X").color(DOWN)).on_hover_text("Cancel Kraken order").clicked() {
                            kr_cancel_ids.push(order.txid.clone());
                        }
                    });
                    ui.label(egui::RichText::new(format!("qty: {} rem {} | {}", format_price(order.vol), format_price(remain), order.status)).color(ACCENT).small());
                    ui.separator();
                }
                }
            }
            for oid in cancel_ids {
                let _ = self.broker_tx.send(BrokerCmd::AlpacaCancelOrder { order_id: oid });
            }
            for txid in kr_cancel_ids {
                let _ = self.broker_tx.send(BrokerCmd::KrakenCancelOrder { txid });
            }
            if !matches!(lo_action, SymbolAction::None) {
                self.deferred_symbol_action = lo_action;
            }
            if !has_orders {
                ui.label(egui::RichText::new("No open orders.").color(AXIS_TEXT).small());
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
        assert!(
            order_price_descriptor(&ord("limit", Some("0.35"), None))
                .unwrap()
                .starts_with("@ ")
        );
        assert!(
            order_price_descriptor(&ord("stop", None, Some("0.25")))
                .unwrap()
                .starts_with("stop ")
        );
        assert!(
            order_price_descriptor(&ord("stop_limit", Some("0.24"), Some("0.25")))
                .unwrap()
                .contains('→')
        );
        // A plain market order has no price to show.
        assert!(order_price_descriptor(&ord("market", None, None)).is_none());
        // Bracket leg roles: stop ⇒ SL, bare limit ⇒ TP.
        assert_eq!(order_leg_role(&ord("limit", Some("0.35"), None)).0, "TP");
        assert_eq!(order_leg_role(&ord("stop", None, Some("0.25"))).0, "SL");
    }

    #[test]
    fn display_groups_identical_simple_orders_and_sums_qty() {
        let mut a = ord("limit", Some("420.6000"), None);
        a.id = "a".into();
        a.symbol = "NXXT".into();
        a.qty = "10".into();
        let mut b = a.clone();
        b.id = "b".into();
        b.qty = "100".into();
        let mut c = a.clone();
        c.id = "c".into();
        c.qty = "1000".into();

        let orders = [a, b, c];
        let groups = order_display_groups(&orders);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].orders.len(), 3);
        assert_eq!(fmt_order_qty_value(groups[0].total_qty), "1110");
        assert!(groups[0].all_qty_numeric);
    }

    #[test]
    fn display_groups_keep_different_prices_and_leg_orders_separate() {
        let mut at_420 = ord("limit", Some("420.6000"), None);
        at_420.id = "420-a".into();
        let mut at_421 = at_420.clone();
        at_421.id = "421".into();
        at_421.limit_price = Some("421.0000".into());
        let mut bracket_parent_a = at_420.clone();
        bracket_parent_a.id = "bracket-a".into();
        bracket_parent_a.legs = Some(vec![ord("stop", None, Some("400.0000"))]);
        let mut bracket_parent_b = bracket_parent_a.clone();
        bracket_parent_b.id = "bracket-b".into();

        let orders = [at_420, at_421, bracket_parent_a, bracket_parent_b];
        let groups = order_display_groups(&orders);

        assert_eq!(groups.len(), 4);
        assert!(groups.iter().all(|group| group.orders.len() == 1));
    }
}
