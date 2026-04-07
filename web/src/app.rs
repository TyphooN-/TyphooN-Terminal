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
}

// ── Main app ────────────────────────────────────────────────────────
pub struct WebApp {
    ws: Option<WebSocket>,
    incoming: Rc<RefCell<Vec<WebMsg>>>,
    connected: bool,
    tab: Tab,

    // Data
    account: Option<AccountSnapshot>,
    positions: Vec<PositionSnapshot>,
    orders: Vec<OrderSnapshot>,
    bars: HashMap<String, Vec<BarData>>,
    current_symbol: String,
    current_timeframe: String,

    // Polling
    poll_counter: u32,
}

impl WebApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let incoming: Rc<RefCell<Vec<WebMsg>>> = Rc::new(RefCell::new(Vec::new()));
        let ws = Self::connect_ws(&incoming);

        Self {
            ws,
            incoming,
            connected: false,
            tab: Tab::Account,
            account: None,
            positions: Vec::new(),
            orders: Vec::new(),
            bars: HashMap::new(),
            current_symbol: "AAPL".into(),
            current_timeframe: "1Day".into(),
            poll_counter: 0,
        }
    }

    fn connect_ws(incoming: &Rc<RefCell<Vec<WebMsg>>>) -> Option<WebSocket> {
        let window = web_sys::window()?;
        let location = window.location();
        let hostname = location.hostname().ok()?;
        let port = location.port().ok()?;
        let url = format!("wss://{hostname}:{port}/ws");

        let ws = WebSocket::new(&url).ok()?;
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        // onmessage callback: parse WebMsg and push to queue
        let inc = incoming.clone();
        let onmessage = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |e: web_sys::MessageEvent| {
            if let Some(text) = e.data().as_string() {
                if let Ok(msg) = serde_json::from_str::<WebMsg>(&text) {
                    inc.borrow_mut().push(msg);
                }
            }
        });
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        // onerror callback
        let onerror = Closure::<dyn FnMut(web_sys::ErrorEvent)>::new(move |_e: web_sys::ErrorEvent| {
            web_sys::console::log_1(&"WebSocket error".into());
        });
        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();

        Some(ws)
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

    fn request_data(&self) {
        self.send_cmd(&WebCmd::GetAccount);
        self.send_cmd(&WebCmd::GetPositions);
        self.send_cmd(&WebCmd::GetOrders);
    }

    fn drain_messages(&mut self) {
        let msgs: Vec<WebMsg> = self.incoming.borrow_mut().drain(..).collect();
        for msg in msgs {
            match msg {
                WebMsg::Account(a) => self.account = Some(a),
                WebMsg::Positions { items } => self.positions = items,
                WebMsg::Orders { items } => self.orders = items,
                WebMsg::Bars { symbol, timeframe, bars } => {
                    let key = format!("{symbol}:{timeframe}");
                    self.bars.insert(key, bars);
                }
                WebMsg::Pong => {}
                WebMsg::Error { msg } => {
                    web_sys::console::warn_1(&format!("Server error: {msg}").into());
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
        if let Some(ref ws) = self.ws {
            self.connected = ws.ready_state() == WebSocket::OPEN;
            if ws.ready_state() == WebSocket::CLOSED {
                self.ws = Self::connect_ws(&self.incoming);
            }
        }

        // Drain incoming messages
        self.drain_messages();

        // Poll every ~5 seconds (ui() fires at ~60fps, so every 300 frames)
        self.poll_counter += 1;
        if self.poll_counter >= 300 {
            self.poll_counter = 0;
            self.request_data();
        }

        // Request initial data on connect
        if self.connected && self.account.is_none() {
            self.request_data();
        }

        // Schedule periodic repaint
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        // ── UI ──────────────────────────────────────────────────
        // Header
        ui.horizontal(|ui| {
            ui.heading("TyphooN Terminal");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.connected {
                    ui.colored_label(egui::Color32::from_rgb(0, 200, 0), "Connected");
                } else {
                    ui.colored_label(egui::Color32::from_rgb(200, 0, 0), "Disconnected");
                }
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
        });
        ui.separator();

        // Tab content
        match self.tab {
            Tab::Account => self.render_account(ui),
            Tab::Positions => self.render_positions(ui),
            Tab::Orders => self.render_orders(ui),
            Tab::Chart => self.render_chart(ui),
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

    fn render_positions(&self, ui: &mut egui::Ui) {
        if self.positions.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No open positions");
            });
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("positions_grid")
                .num_columns(6)
                .spacing([12.0, 6.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Symbol");
                    ui.strong("Side");
                    ui.strong("Qty");
                    ui.strong("Entry");
                    ui.strong("Value");
                    ui.strong("P&L");
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
                        ui.end_row();
                    }
                });
        });
    }

    fn render_orders(&self, ui: &mut egui::Ui) {
        if self.orders.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No open orders");
            });
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("orders_grid")
                .num_columns(6)
                .spacing([12.0, 6.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Symbol");
                    ui.strong("Side");
                    ui.strong("Qty");
                    ui.strong("Type");
                    ui.strong("Status");
                    ui.strong("Price");
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
                        ui.end_row();
                    }
                });
        });
    }

    fn render_chart(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Symbol:");
            ui.text_edit_singleline(&mut self.current_symbol);
            ui.label("TF:");
            ui.text_edit_singleline(&mut self.current_timeframe);
            if ui.button("Load").clicked() {
                self.send_cmd(&WebCmd::GetBars {
                    symbol: self.current_symbol.clone(),
                    timeframe: self.current_timeframe.clone(),
                });
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
