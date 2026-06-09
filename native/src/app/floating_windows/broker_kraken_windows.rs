use super::*;

impl TyphooNApp {
    pub(super) fn render_broker_kraken_windows(&mut self, ctx: &egui::Context) {
        self.render_connect_window(ctx, false);
        self.render_indicators_window(ctx);
        self.render_kraken_spot_sell_dialog(ctx);
        // ── Kraken Trade History Window ─────────────────────────────────────
        if self.show_kraken_trade_history {
            egui::Window::new("Kraken Trade History")
                .open(&mut self.show_kraken_trade_history)
                .default_size([900.0, 500.0])
                .max_size([900.0, 560.0])
                .resizable(true)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} trades loaded", self.kraken_trades.len()));
                        if ui.button("Refresh").clicked() {
                            let _ = self.broker_tx.send(BrokerCmd::KrakenFetchTrades);
                        }
                        if ui.button("Open Orders").clicked() {
                            self.show_kraken_open_orders = true;
                            let _ = self.broker_tx.send(BrokerCmd::KrakenFetchOpenOrders);
                        }
                    });
                    ui.separator();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        egui::Grid::new("kraken_trades_grid")
                            .striped(true)
                            .show(ui, |ui| {
                                // Header
                                ui.label(egui::RichText::new("Time").strong());
                                ui.label(egui::RichText::new("Pair").strong());
                                ui.label(egui::RichText::new("Side").strong());
                                ui.label(egui::RichText::new("Type").strong());
                                ui.label(egui::RichText::new("Price").strong());
                                ui.label(egui::RichText::new("Vol").strong());
                                ui.label(egui::RichText::new("Cost").strong());
                                ui.label(egui::RichText::new("Fee").strong());
                                ui.end_row();

                                for t in &self.kraken_trades {
                                    let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(
                                        t.time as i64,
                                        0,
                                    )
                                    .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                                    .unwrap_or_else(|| format!("{:.0}", t.time));

                                    ui.label(dt);
                                    ui.label(&t.pair);
                                    ui.colored_label(
                                        if t.side == "buy" {
                                            egui::Color32::from_rgb(46, 204, 113)
                                        } else {
                                            egui::Color32::from_rgb(231, 76, 60)
                                        },
                                        &t.side,
                                    );
                                    ui.label(&t.ordertype);
                                    ui.label(format!("{:.4}", t.price));
                                    ui.label(format!("{:.4}", t.vol));
                                    ui.label(format!("{:.2}", t.cost));
                                    ui.label(format!("{:.4}", t.fee));
                                    ui.end_row();
                                }
                            });
                    });
                });
        }

        // ── Kraken Open Orders Window ────────────────────────────────────────
        if self.show_kraken_open_orders {
            egui::Window::new("Kraken Open Orders")
                .open(&mut self.show_kraken_open_orders)
                .default_size([1000.0, 420.0])
                .max_size([1000.0, 560.0])
                .resizable(true)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} open orders", self.kraken_open_orders.len()));
                        if ui.button("Refresh").clicked() {
                            let _ = self.broker_tx.send(BrokerCmd::KrakenFetchOpenOrders);
                        }
                        if ui.button("Trade History").clicked() {
                            self.show_kraken_trade_history = true;
                        }
                    });
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            egui::Grid::new("kraken_open_orders_grid")
                                .striped(true)
                                .num_columns(11)
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new("Time").strong());
                                    ui.label(egui::RichText::new("Pair").strong());
                                    ui.label(egui::RichText::new("Side").strong());
                                    ui.label(egui::RichText::new("Type").strong());
                                    ui.label(egui::RichText::new("Price").strong());
                                    ui.label(egui::RichText::new("Vol").strong());
                                    ui.label(egui::RichText::new("Filled").strong());
                                    ui.label(egui::RichText::new("Remain").strong());
                                    ui.label(egui::RichText::new("Status").strong());
                                    ui.label(egui::RichText::new("TxID").strong());
                                    ui.label(egui::RichText::new("Action").strong());
                                    ui.end_row();

                                    for order in &self.kraken_open_orders {
                                        let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(
                                            order.opentm as i64,
                                            0,
                                        )
                                        .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                                        .unwrap_or_else(|| format!("{:.0}", order.opentm));
                                        let side_color = if order.r#type == "buy" {
                                            egui::Color32::from_rgb(46, 204, 113)
                                        } else {
                                            egui::Color32::from_rgb(231, 76, 60)
                                        };
                                        let remain = (order.vol - order.vol_exec).max(0.0);

                                        ui.label(dt);
                                        ui.label(&order.pair);
                                        ui.colored_label(side_color, &order.r#type);
                                        ui.label(&order.ordertype);
                                        ui.label(format!("{:.6}", order.price));
                                        ui.label(format!("{:.6}", order.vol));
                                        ui.label(format!("{:.6}", order.vol_exec));
                                        ui.label(format!("{:.6}", remain));
                                        ui.label(&order.status);
                                        ui.label(egui::RichText::new(&order.txid).small());
                                        if ui.small_button("Cancel").clicked() {
                                            let _ =
                                                self.broker_tx.send(BrokerCmd::KrakenCancelOrder {
                                                    txid: order.txid.clone(),
                                                });
                                        }
                                        ui.end_row();
                                    }
                                });
                        });
                });
        }
    }
}
