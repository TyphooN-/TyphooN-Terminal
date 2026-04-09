use eframe::egui;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use typhoon_web_protocol::*;
use wasm_bindgen::prelude::*;
use web_sys::WebSocket;

// ── Tab selector ────────────────────────────────────────────────────
#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Account,
    Positions,
    Orders,
    Chart,
    Trade,
}

// ── Main app ────────────────────────────────────────────────────────
pub struct WebApp {
    ws: Option<WebSocket>,
    incoming: Rc<RefCell<Vec<WebMsg>>>,
    connected: bool,
    authenticated: bool,
    tab: Tab,

    // Auth
    passphrase: String,
    auth_failed: bool,

    // Data
    account: Option<AccountSnapshot>,
    positions: Vec<PositionSnapshot>,
    orders: Vec<OrderSnapshot>,
    bars: HashMap<String, Vec<BarData>>,
    current_symbol: String,
    current_timeframe: String,

    // Polling
    poll_counter: u32,

    // Order entry form state
    order_symbol: String,
    order_qty_str: String,
    order_side: String,     // "buy" | "sell"
    order_type: String,     // "market" | "limit" | "stop"
    order_limit_str: String,
    order_stop_str: String,
    order_broker: String,   // "alpaca" | "tastytrade"
    order_confirm_pending: bool,
    last_order_result: Option<(bool, String)>,
}

impl WebApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            ws: None,
            incoming: Rc::new(RefCell::new(Vec::new())),
            connected: false,
            authenticated: false,
            tab: Tab::Account,
            passphrase: String::new(),
            auth_failed: false,
            account: None,
            positions: Vec::new(),
            orders: Vec::new(),
            bars: HashMap::new(),
            current_symbol: "AAPL".into(),
            current_timeframe: "1Day".into(),
            poll_counter: 0,
            order_symbol: "AAPL".into(),
            order_qty_str: "1".into(),
            order_side: "buy".into(),
            order_type: "market".into(),
            order_limit_str: String::new(),
            order_stop_str: String::new(),
            order_broker: "alpaca".into(),
            order_confirm_pending: false,
            last_order_result: None,
        }
    }

    fn connect_and_auth(&mut self) {
        // Close existing connection cleanly
        if let Some(ref old_ws) = self.ws {
            old_ws.set_onmessage(None);
            old_ws.set_onerror(None);
            old_ws.set_onclose(None);
            let _ = old_ws.close();
        }
        self.incoming.borrow_mut().clear();
        self.authenticated = false;
        self.auth_failed = false;

        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };
        let location = window.location();
        let hostname = location.hostname().unwrap_or_default();
        let port = location.port().unwrap_or_default();
        let url = format!("wss://{hostname}:{port}/ws");

        let ws = match WebSocket::new(&url) {
            Ok(ws) => ws,
            Err(_) => return,
        };
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        // onmessage: parse WebMsg, cap queue at 1000
        let inc = self.incoming.clone();
        let onmessage = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |e: web_sys::MessageEvent| {
            if let Some(text) = e.data().as_string() {
                if let Ok(msg) = serde_json::from_str::<WebMsg>(&text) {
                    let mut q = inc.borrow_mut();
                    if q.len() < 1000 {
                        q.push(msg);
                    }
                }
            }
        });
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        let onerror = Closure::<dyn FnMut(web_sys::ErrorEvent)>::new(move |_: web_sys::ErrorEvent| {
            web_sys::console::log_1(&"WebSocket error".into());
        });
        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();

        // Send Auth immediately on open
        let passphrase = self.passphrase.clone();
        let onopen = Closure::<dyn FnMut()>::new(move || {
            // Auth is sent from update() after connected=true
            let _ = &passphrase; // prevent premature drop
        });
        ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        onopen.forget();

        self.ws = Some(ws);
    }

    fn send_cmd(&self, cmd: &WebCmd) {
        if let Some(ref ws) = self.ws {
            if ws.ready_state() == WebSocket::OPEN {
                if let Ok(json) = serde_json::to_string(cmd) {
                    let _ = ws.send_with_str(&json);
                }
            }
        }
    }

    fn send_auth(&self) {
        self.send_cmd(&WebCmd::Auth { passphrase: self.passphrase.clone() });
    }

    fn request_data(&self) {
        self.send_cmd(&WebCmd::GetAccount);
        self.send_cmd(&WebCmd::GetPositions);
        self.send_cmd(&WebCmd::GetOrders);
    }

    fn drain_messages(&mut self) {
        let msgs: Vec<WebMsg> = self.incoming.borrow_mut().drain(..).collect();
        for msg in msgs {
            match msg {
                WebMsg::AuthResult { ok } => {
                    if ok {
                        self.authenticated = true;
                        self.auth_failed = false;
                        self.request_data();
                    } else {
                        self.authenticated = false;
                        self.auth_failed = true;
                        // Close connection on auth failure
                        if let Some(ref ws) = self.ws {
                            let _ = ws.close();
                        }
                    }
                }
                WebMsg::Account(a) => self.account = Some(a),
                WebMsg::Positions { items } => self.positions = items,
                WebMsg::Orders { items } => self.orders = items,
                WebMsg::Bars { symbol, timeframe, bars } => {
                    let key = format!("{symbol}:{timeframe}");
                    self.bars.insert(key, bars);
                }
                WebMsg::Pong => {}
                WebMsg::OrderResult { ok, message } => {
                    self.last_order_result = Some((ok, message));
                    // Re-poll positions/orders so UI reflects new state
                    self.request_data();
                }
                WebMsg::Error { msg } => {
                    web_sys::console::warn_1(&format!("Server: {msg}").into());
                }
                _ => {}
            }
        }
    }
}

impl eframe::App for WebApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Track connection state
        let ws_open = self.ws.as_ref().map_or(false, |ws| ws.ready_state() == WebSocket::OPEN);
        let ws_closed = self.ws.as_ref().map_or(true, |ws| ws.ready_state() == WebSocket::CLOSED);
        self.connected = ws_open;

        // Send auth when first connected
        if ws_open && !self.authenticated && !self.auth_failed {
            self.send_auth();
        }

        // Drain incoming messages
        self.drain_messages();

        // Poll every ~5 seconds when authenticated
        if self.authenticated {
            self.poll_counter += 1;
            if self.poll_counter >= 300 {
                self.poll_counter = 0;
                self.request_data();
            }
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        // ── Login screen (not connected or not authenticated) ───────
        if !self.authenticated {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.heading("TyphooN Terminal");
                ui.add_space(20.0);

                if self.auth_failed {
                    ui.colored_label(egui::Color32::from_rgb(200, 0, 0), "Authentication failed — check passphrase");
                    ui.add_space(10.0);
                }

                ui.label("LAN Passphrase:");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.passphrase)
                        .desired_width(200.0)
                        .password(true)
                        .hint_text("same as LAN sync"),
                );

                let enter_pressed = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let connect_clicked = ui.button("Connect").clicked();

                if (connect_clicked || enter_pressed) && !self.passphrase.is_empty() {
                    self.connect_and_auth();
                }

                if ws_closed && !self.passphrase.is_empty() && !self.auth_failed {
                    ui.add_space(10.0);
                    ui.colored_label(egui::Color32::from_rgb(200, 200, 0), "Disconnected — click Connect");
                }
            });
            return;
        }

        // ── Authenticated UI ────────────────────────────────────────
        // Header
        ui.horizontal(|ui| {
            ui.heading("TyphooN Terminal");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.colored_label(egui::Color32::from_rgb(0, 200, 0), "Connected");
            });
        });

        // Account summary bar
        if let Some(ref acct) = self.account {
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("Equity: ${:.2}", acct.equity));
                ui.separator();
                let pl_color = if acct.unrealized_pl >= 0.0 {
                    egui::Color32::from_rgb(0, 200, 0)
                } else {
                    egui::Color32::from_rgb(200, 0, 0)
                };
                ui.colored_label(pl_color, format!("P&L: ${:.2}", acct.unrealized_pl));
                ui.separator();
                ui.label(format!("Cash: ${:.2}", acct.cash));
                ui.separator();
                ui.label(format!("BP: ${:.2}", acct.buying_power));
            });
        }

        // Tab bar
        ui.separator();
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tab, Tab::Account, "Account");
            ui.selectable_value(&mut self.tab, Tab::Positions, "Positions");
            ui.selectable_value(&mut self.tab, Tab::Orders, "Orders");
            ui.selectable_value(&mut self.tab, Tab::Chart, "Chart");
            ui.selectable_value(&mut self.tab, Tab::Trade, "Trade");
        });
        ui.separator();

        match self.tab {
            Tab::Account => self.render_account(ui),
            Tab::Positions => self.render_positions(ui),
            Tab::Orders => self.render_orders(ui),
            Tab::Chart => self.render_chart(ui),
            Tab::Trade => self.render_trade(ui),
        }
    }
}

// ── Tab renderers ───────────────────────────────────────────────────
impl WebApp {
    fn render_account(&self, ui: &mut egui::Ui) {
        if let Some(ref acct) = self.account {
            egui::Grid::new("account_grid")
                .num_columns(2)
                .spacing([20.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Equity");
                    ui.label(format!("${:.2}", acct.equity));
                    ui.end_row();
                    ui.label("Cash");
                    ui.label(format!("${:.2}", acct.cash));
                    ui.end_row();
                    ui.label("Portfolio Value");
                    ui.label(format!("${:.2}", acct.portfolio_value));
                    ui.end_row();
                    ui.label("Buying Power");
                    ui.label(format!("${:.2}", acct.buying_power));
                    ui.end_row();
                    ui.label("Unrealized P&L");
                    let pl = acct.unrealized_pl;
                    let color = if pl >= 0.0 {
                        egui::Color32::from_rgb(0, 200, 0)
                    } else {
                        egui::Color32::from_rgb(200, 0, 0)
                    };
                    ui.colored_label(color, format!("${:.2}", pl));
                    ui.end_row();
                    ui.label("Initial Margin");
                    ui.label(format!("${:.2}", acct.initial_margin));
                    ui.end_row();
                    ui.label("Maintenance Margin");
                    ui.label(format!("${:.2}", acct.maintenance_margin));
                    ui.end_row();
                    ui.label("Currency");
                    ui.label(&acct.currency);
                    ui.end_row();
                });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Waiting for account data...");
            });
        }
    }

    fn render_positions(&mut self, ui: &mut egui::Ui) {
        if self.positions.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No open positions");
            });
            return;
        }

        // Collect close requests so we don't hold &self.positions across send_cmd
        let mut close_requests: Vec<String> = Vec::new();

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("positions_grid")
                .num_columns(7)
                .spacing([12.0, 6.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Symbol");
                    ui.strong("Side");
                    ui.strong("Qty");
                    ui.strong("Entry");
                    ui.strong("Value");
                    ui.strong("P&L");
                    ui.strong("");
                    ui.end_row();

                    for pos in &self.positions {
                        ui.label(&pos.symbol);
                        ui.label(&pos.side);
                        ui.label(format!("{:.2}", pos.qty));
                        ui.label(format!("${:.2}", pos.avg_entry_price));
                        ui.label(format!("${:.2}", pos.market_value));
                        let color = if pos.unrealized_pl >= 0.0 {
                            egui::Color32::from_rgb(0, 200, 0)
                        } else {
                            egui::Color32::from_rgb(200, 0, 0)
                        };
                        ui.colored_label(color, format!("${:.2}", pos.unrealized_pl));
                        if ui
                            .button("Close")
                            .on_hover_text("Close this position at market")
                            .clicked()
                        {
                            close_requests.push(pos.symbol.clone());
                        }
                        ui.end_row();
                    }
                });
        });

        // Broker for close is the currently-selected order broker (Trade tab) — sensible default.
        let broker = self.order_broker.clone();
        for sym in close_requests {
            self.send_cmd(&WebCmd::ClosePosition { symbol: sym, broker: broker.clone() });
        }
    }

    fn render_orders(&mut self, ui: &mut egui::Ui) {
        if self.orders.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No open orders");
            });
            return;
        }

        let mut cancel_requests: Vec<String> = Vec::new();

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("orders_grid")
                .num_columns(7)
                .spacing([12.0, 6.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Symbol");
                    ui.strong("Side");
                    ui.strong("Qty");
                    ui.strong("Type");
                    ui.strong("Status");
                    ui.strong("Price");
                    ui.strong("");
                    ui.end_row();

                    for ord in &self.orders {
                        ui.label(&ord.symbol);
                        ui.label(&ord.side);
                        ui.label(&ord.qty);
                        ui.label(&ord.order_type);
                        ui.label(&ord.status);
                        let price = ord
                            .limit_price
                            .as_deref()
                            .or(ord.stop_price.as_deref())
                            .unwrap_or("-");
                        ui.label(price);
                        if ui
                            .button("Cancel")
                            .on_hover_text("Cancel this open order")
                            .clicked()
                        {
                            cancel_requests.push(ord.id.clone());
                        }
                        ui.end_row();
                    }
                });
        });

        let broker = self.order_broker.clone();
        for id in cancel_requests {
            self.send_cmd(&WebCmd::CancelOrder { order_id: id, broker: broker.clone() });
        }
    }

    fn render_trade(&mut self, ui: &mut egui::Ui) {
        ui.heading("Place Order");
        ui.add_space(6.0);

        egui::Grid::new("trade_form")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label("Broker");
                egui::ComboBox::from_id_salt("broker_combo")
                    .selected_text(&self.order_broker)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.order_broker, "alpaca".into(), "Alpaca");
                        ui.selectable_value(&mut self.order_broker, "tastytrade".into(), "Tastytrade");
                    });
                ui.end_row();

                ui.label("Symbol");
                ui.add(
                    egui::TextEdit::singleline(&mut self.order_symbol)
                        .desired_width(120.0)
                        .char_limit(MAX_SYMBOL_LEN),
                );
                ui.end_row();

                ui.label("Side");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.order_side, "buy".into(), "Buy");
                    ui.selectable_value(&mut self.order_side, "sell".into(), "Sell");
                });
                ui.end_row();

                ui.label("Type");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.order_type, "market".into(), "Market");
                    ui.selectable_value(&mut self.order_type, "limit".into(), "Limit");
                    ui.selectable_value(&mut self.order_type, "stop".into(), "Stop");
                });
                ui.end_row();

                ui.label("Qty");
                ui.add(
                    egui::TextEdit::singleline(&mut self.order_qty_str)
                        .desired_width(100.0)
                        .char_limit(12),
                );
                ui.end_row();

                if self.order_type == "limit" {
                    ui.label("Limit Price");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.order_limit_str)
                            .desired_width(100.0)
                            .char_limit(16),
                    );
                    ui.end_row();
                }

                if self.order_type == "stop" {
                    ui.label("Stop Price");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.order_stop_str)
                            .desired_width(100.0)
                            .char_limit(16),
                    );
                    ui.end_row();
                }
            });

        ui.add_space(10.0);

        // Validate locally before showing Submit button
        let qty = self.order_qty_str.parse::<f64>().ok();
        let limit = self.order_limit_str.parse::<f64>().ok();
        let stop = self.order_stop_str.parse::<f64>().ok();
        let sym_ok = is_valid_symbol(&self.order_symbol);
        let qty_ok = qty.map(is_valid_order_qty).unwrap_or(false);
        let side_ok = is_valid_order_side(&self.order_side);
        let type_ok = is_valid_order_type(&self.order_type);
        let price_ok = match self.order_type.as_str() {
            "limit" => limit.map(|p| p.is_finite() && p > 0.0).unwrap_or(false),
            "stop" => stop.map(|p| p.is_finite() && p > 0.0).unwrap_or(false),
            _ => true,
        };
        let form_valid = sym_ok && qty_ok && side_ok && type_ok && price_ok;

        if !sym_ok {
            ui.colored_label(egui::Color32::from_rgb(200, 120, 0), "Invalid symbol");
        }
        if !qty_ok && !self.order_qty_str.is_empty() {
            ui.colored_label(egui::Color32::from_rgb(200, 120, 0), "Qty must be 0 < q ≤ 100,000");
        }
        if !price_ok {
            ui.colored_label(egui::Color32::from_rgb(200, 120, 0), "Price must be positive");
        }

        ui.add_space(6.0);

        // Two-step confirm so a stray tap doesn't send an order
        if !self.order_confirm_pending {
            let btn = egui::Button::new(format!(
                "Review {} {} {} ({})",
                self.order_side.to_uppercase(),
                self.order_qty_str,
                self.order_symbol,
                self.order_broker
            ));
            if ui.add_enabled(form_valid, btn).clicked() {
                self.order_confirm_pending = true;
            }
        } else {
            ui.colored_label(
                egui::Color32::from_rgb(220, 180, 0),
                format!(
                    "CONFIRM: {} {} {} on {} ({})",
                    self.order_side.to_uppercase(),
                    self.order_qty_str,
                    self.order_symbol,
                    self.order_broker,
                    self.order_type
                ),
            );
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new("SEND").fill(egui::Color32::from_rgb(0, 140, 0)))
                    .clicked()
                    && form_valid
                {
                    self.send_cmd(&WebCmd::PlaceOrder {
                        symbol: self.order_symbol.clone(),
                        qty: qty.unwrap_or(0.0),
                        side: self.order_side.clone(),
                        order_type: self.order_type.clone(),
                        limit_price: if self.order_type == "limit" { limit } else { None },
                        stop_price: if self.order_type == "stop" { stop } else { None },
                        broker: self.order_broker.clone(),
                    });
                    self.order_confirm_pending = false;
                }
                if ui.button("Cancel").clicked() {
                    self.order_confirm_pending = false;
                }
            });
        }

        ui.add_space(12.0);
        ui.separator();
        if let Some((ok, ref msg)) = self.last_order_result {
            let color = if ok {
                egui::Color32::from_rgb(0, 200, 0)
            } else {
                egui::Color32::from_rgb(200, 0, 0)
            };
            ui.colored_label(color, format!("Last: {msg}"));
        } else {
            ui.label("No orders sent this session");
        }
    }

    fn render_chart(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Symbol:");
            let sym_response = ui.add(
                egui::TextEdit::singleline(&mut self.current_symbol)
                    .desired_width(80.0)
                    .char_limit(MAX_SYMBOL_LEN),
            );
            ui.label("TF:");
            ui.add(
                egui::TextEdit::singleline(&mut self.current_timeframe)
                    .desired_width(60.0)
                    .char_limit(MAX_TIMEFRAME_LEN),
            );
            let enter = sym_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.button("Load").clicked() || enter {
                if is_valid_symbol(&self.current_symbol) && is_valid_timeframe(&self.current_timeframe) {
                    self.send_cmd(&WebCmd::GetBars {
                        symbol: self.current_symbol.clone(),
                        timeframe: self.current_timeframe.clone(),
                    });
                }
            }
        });

        let key = format!("{}:{}", self.current_symbol, self.current_timeframe);
        if let Some(bars) = self.bars.get(&key) {
            if bars.is_empty() {
                ui.label("No bar data");
                return;
            }

            use egui_plot::{Line, Plot, PlotPoints};

            let closes: PlotPoints = bars
                .iter()
                .enumerate()
                .map(|(i, b)| [i as f64, b.close])
                .collect();

            Plot::new("price_chart")
                .height(ui.available_height())
                .show(ui, |plot_ui| {
                    plot_ui.line(
                        Line::new(&self.current_symbol, closes)
                            .color(egui::Color32::from_rgb(100, 200, 255)),
                    );
                });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Click Load to fetch chart data");
            });
        }
    }
}
