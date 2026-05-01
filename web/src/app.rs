use eframe::egui;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
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
    Watchlist,
    Alerts,
    News,
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

    // Polling (fallback — push updates reduce frequency)
    poll_counter: u32,

    // Order entry form state
    order_symbol: String,
    order_qty_str: String,
    order_side: String,
    order_type: String,
    order_limit_str: String,
    order_stop_str: String,
    order_broker: String,
    order_confirm_pending: bool,
    last_order_result: Option<(bool, String)>,

    // ADR-092: bracket/trailing/risk fields
    order_tp_str: String,
    order_sl_str: String,
    order_trail_pct_str: String,
    order_trail_offset_str: String,
    order_risk_mode: String,
    order_risk_pct_str: String,

    // ADR-092: watchlist
    watchlist_symbols_str: String,
    watchlist_quotes: Vec<QuoteSnapshot>,

    // ADR-092: live quote tick
    last_quote_tick: Option<(String, f64, f64)>,

    // ADR-092: indicators
    indicator_data: HashMap<String, Vec<Option<f64>>>,
    show_sma200: bool,
    show_ema21: bool,
    show_rsi14: bool,

    // ADR-092: alerts
    alerts: Vec<AlertSnapshot>,
    alert_symbol: String,
    alert_condition: String,
    alert_price_str: String,
    alert_message: String,
    alert_triggered: VecDeque<(String, String, String)>,

    // ADR-092: news
    news_items: Vec<NewsItem>,
    news_symbol: String,

    // ADR-092: subscribed symbol (for push updates)
    subscribed: Option<(String, String)>,

    // Chart zoom/pan
    chart_visible_bars: usize,
    chart_offset: usize,
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
            order_tp_str: String::new(),
            order_sl_str: String::new(),
            order_trail_pct_str: String::new(),
            order_trail_offset_str: String::new(),
            order_risk_mode: "standard".into(),
            order_risk_pct_str: "2.0".into(),
            watchlist_symbols_str: "AAPL,MSFT,TSLA,SPY,QQQ".into(),
            watchlist_quotes: Vec::new(),
            last_quote_tick: None,
            indicator_data: HashMap::new(),
            show_sma200: true,
            show_ema21: false,
            show_rsi14: false,
            alerts: Vec::new(),
            alert_symbol: "AAPL".into(),
            alert_condition: "crosses_above".into(),
            alert_price_str: String::new(),
            alert_message: String::new(),
            alert_triggered: VecDeque::new(),
            news_items: Vec::new(),
            news_symbol: String::new(),
            subscribed: None,
            chart_visible_bars: 200,
            chart_offset: 0,
        }
    }

    fn connect_and_auth(&mut self) {
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

        let passphrase = self.passphrase.clone();
        let onopen = Closure::<dyn FnMut()>::new(move || {
            let _ = &passphrase;
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

    fn request_indicators(&self) {
        let mut indicators = Vec::new();
        if self.show_sma200 { indicators.push("SMA_200".to_string()); }
        if self.show_ema21 { indicators.push("EMA_21".to_string()); }
        if self.show_rsi14 { indicators.push("RSI_14".to_string()); }
        if !indicators.is_empty() {
            self.send_cmd(&WebCmd::GetIndicators {
                symbol: self.current_symbol.clone(),
                timeframe: self.current_timeframe.clone(),
                indicators,
            });
        }
    }

    fn subscribe_chart(&mut self) {
        let new_sub = (self.current_symbol.clone(), self.current_timeframe.clone());
        if self.subscribed.as_ref() != Some(&new_sub) {
            if let Some((ref old_sym, ref old_tf)) = self.subscribed {
                self.send_cmd(&WebCmd::Unsubscribe {
                    symbol: old_sym.clone(),
                    timeframe: old_tf.clone(),
                });
            }
            self.send_cmd(&WebCmd::Subscribe {
                symbol: new_sub.0.clone(),
                timeframe: new_sub.1.clone(),
            });
            self.subscribed = Some(new_sub);
        }
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
                    // Reset offset on new data
                    self.chart_offset = 0;
                }
                WebMsg::Pong => {}
                WebMsg::OrderResult { ok, message } => {
                    self.last_order_result = Some((ok, message));
                    self.request_data();
                }
                WebMsg::Error { msg } => {
                    web_sys::console::warn_1(&format!("Server: {msg}").into());
                }
                // ADR-092: push updates
                WebMsg::BarUpdate { symbol, timeframe, bar } => {
                    let key = format!("{symbol}:{timeframe}");
                    if let Some(bars) = self.bars.get_mut(&key) {
                        // Update last bar or append new one
                        if let Some(last) = bars.last_mut() {
                            if last.timestamp == bar.timestamp {
                                *last = bar;
                            } else {
                                bars.push(bar);
                            }
                        } else {
                            bars.push(bar);
                        }
                    }
                }
                WebMsg::PositionUpdate { items } => self.positions = items,
                WebMsg::AccountUpdate(a) => self.account = Some(a),
                WebMsg::QuoteTick { symbol, bid, ask } => {
                    self.last_quote_tick = Some((symbol.clone(), bid, ask));
                    // Update watchlist quote if present
                    if let Some(q) = self.watchlist_quotes.iter_mut().find(|q| q.symbol == symbol) {
                        q.bid = bid;
                        q.ask = ask;
                        q.last = (bid + ask) / 2.0;
                    }
                }
                WebMsg::WatchlistQuotes { items } => self.watchlist_quotes = items,
                WebMsg::IndicatorData { name, values, .. } => {
                    self.indicator_data.insert(name, values);
                }
                WebMsg::AlertTriggered { alert_id, symbol, message } => {
                    self.alert_triggered.push_back((alert_id, symbol, message));
                    // Keep last 20 triggered alerts — O(1) pop_front via VecDeque
                    while self.alert_triggered.len() > 20 {
                        self.alert_triggered.pop_front();
                    }
                }
                WebMsg::AlertList { items } => self.alerts = items,
                WebMsg::NewsFeed { items } => self.news_items = items,
                _ => {}
            }
        }
    }
}

// ── Colors ──────────────────────────────────────────────────────────
const COLOR_UP: egui::Color32 = egui::Color32::from_rgb(0, 255, 0);
const COLOR_DOWN: egui::Color32 = egui::Color32::from_rgb(255, 0, 0);
const COLOR_SMA200: egui::Color32 = egui::Color32::from_rgb(255, 255, 0);
const COLOR_EMA21: egui::Color32 = egui::Color32::from_rgb(255, 130, 48);
const COLOR_RSI: egui::Color32 = egui::Color32::from_rgb(200, 180, 60);
const COLOR_GRID: egui::Color32 = egui::Color32::from_rgb(40, 40, 40);
const COLOR_CROSSHAIR: egui::Color32 = egui::Color32::from_rgb(128, 128, 128);
const COLOR_VOLUME: egui::Color32 = egui::Color32::from_rgb(50, 80, 120);

impl eframe::App for WebApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        let ws_open = self.ws.as_ref().map_or(false, |ws| ws.ready_state() == WebSocket::OPEN);
        let ws_closed = self.ws.as_ref().map_or(true, |ws| ws.ready_state() == WebSocket::CLOSED);
        self.connected = ws_open;

        if ws_open && !self.authenticated && !self.auth_failed {
            self.send_auth();
        }

        self.drain_messages();

        // Poll every ~30 seconds as fallback (push updates handle most data)
        if self.authenticated {
            self.poll_counter += 1;
            if self.poll_counter >= 1800 {
                self.poll_counter = 0;
                self.request_data();
            }
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        // ── Login screen ───────────────────────────────────────────
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
        ui.horizontal(|ui| {
            ui.heading("TyphooN Terminal");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Show live quote tick if available
                if let Some((ref sym, bid, ask)) = self.last_quote_tick {
                    ui.label(format!("{sym} {bid:.4}/{ask:.4}"));
                    ui.separator();
                }
                ui.colored_label(egui::Color32::from_rgb(0, 200, 0), "Connected");
            });
        });

        // Account summary bar
        if let Some(ref acct) = self.account {
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("Equity: ${:.2}", acct.equity));
                ui.separator();
                let pl_color = if acct.unrealized_pl >= 0.0 { COLOR_UP } else { COLOR_DOWN };
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
            ui.selectable_value(&mut self.tab, Tab::Watchlist, "Watch");
            ui.selectable_value(&mut self.tab, Tab::Alerts, "Alerts");
            ui.selectable_value(&mut self.tab, Tab::News, "News");
        });
        ui.separator();

        // Show alert notifications
        if !self.alert_triggered.is_empty() {
            let last = self.alert_triggered.back().cloned();
            if let Some((_id, sym, msg)) = last {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(255, 200, 0),
                        format!("ALERT: {sym} — {msg}"));
                });
                ui.separator();
            }
        }

        match self.tab {
            Tab::Account => self.render_account(ui),
            Tab::Positions => self.render_positions(ui),
            Tab::Orders => self.render_orders(ui),
            Tab::Chart => self.render_chart(ui),
            Tab::Trade => self.render_trade(ui),
            Tab::Watchlist => self.render_watchlist(ui),
            Tab::Alerts => self.render_alerts(ui),
            Tab::News => self.render_news(ui),
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
                    let color = if pl >= 0.0 { COLOR_UP } else { COLOR_DOWN };
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
                        let color = if pos.unrealized_pl >= 0.0 { COLOR_UP } else { COLOR_DOWN };
                        ui.colored_label(color, format!("${:.2}", pos.unrealized_pl));
                        if ui.button("Close").on_hover_text("Close at market").clicked() {
                            close_requests.push(pos.symbol.clone());
                        }
                        ui.end_row();
                    }
                });
        });

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
                        let price = ord.limit_price.as_deref()
                            .or(ord.stop_price.as_deref())
                            .unwrap_or("-");
                        ui.label(price);
                        if ui.button("Cancel").on_hover_text("Cancel order").clicked() {
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
                        ui.selectable_value(&mut self.order_broker, "kraken".into(), "Kraken");
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
                    ui.selectable_value(&mut self.order_type, "trailing_stop".into(), "Trail");
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
                    ui.add(egui::TextEdit::singleline(&mut self.order_limit_str).desired_width(100.0).char_limit(16));
                    ui.end_row();
                }

                if self.order_type == "stop" || self.order_type == "stop_limit" {
                    ui.label("Stop Price");
                    ui.add(egui::TextEdit::singleline(&mut self.order_stop_str).desired_width(100.0).char_limit(16));
                    ui.end_row();
                }

                if self.order_type == "trailing_stop" {
                    ui.label("Trail %");
                    ui.add(egui::TextEdit::singleline(&mut self.order_trail_pct_str).desired_width(80.0).char_limit(8));
                    ui.end_row();
                    ui.label("Trail Offset");
                    ui.add(egui::TextEdit::singleline(&mut self.order_trail_offset_str).desired_width(80.0).char_limit(10));
                    ui.end_row();
                }

                // Bracket order fields (TP/SL)
                ui.label("Take Profit");
                ui.add(egui::TextEdit::singleline(&mut self.order_tp_str).desired_width(100.0).char_limit(16));
                ui.end_row();
                ui.label("Stop Loss");
                ui.add(egui::TextEdit::singleline(&mut self.order_sl_str).desired_width(100.0).char_limit(16));
                ui.end_row();

                // Risk mode
                ui.label("Risk Mode");
                egui::ComboBox::from_id_salt("risk_mode_combo")
                    .selected_text(&self.order_risk_mode)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.order_risk_mode, "standard".into(), "Standard");
                        ui.selectable_value(&mut self.order_risk_mode, "fixed".into(), "Fixed");
                        ui.selectable_value(&mut self.order_risk_mode, "dynamic".into(), "Dynamic");
                        ui.selectable_value(&mut self.order_risk_mode, "var".into(), "VaR");
                    });
                ui.end_row();

                ui.label("Risk %");
                ui.add(egui::TextEdit::singleline(&mut self.order_risk_pct_str).desired_width(60.0).char_limit(6));
                ui.end_row();
            });

        ui.add_space(10.0);

        let qty = self.order_qty_str.parse::<f64>().ok();
        let limit = self.order_limit_str.parse::<f64>().ok();
        let stop = self.order_stop_str.parse::<f64>().ok();
        let tp = self.order_tp_str.parse::<f64>().ok();
        let sl = self.order_sl_str.parse::<f64>().ok();
        let trail_pct = self.order_trail_pct_str.parse::<f64>().ok();
        let trail_off = self.order_trail_offset_str.parse::<f64>().ok();
        let risk_pct = self.order_risk_pct_str.parse::<f64>().ok();

        let sym_ok = is_valid_symbol(&self.order_symbol);
        let qty_ok = qty.map(is_valid_order_qty).unwrap_or(false);
        let side_ok = is_valid_order_side(&self.order_side);
        let type_ok = is_valid_order_type(&self.order_type);
        let price_ok = match self.order_type.as_str() {
            "limit" => limit.map(|p| p.is_finite() && p > 0.0).unwrap_or(false),
            "stop" => stop.map(|p| p.is_finite() && p > 0.0).unwrap_or(false),
            "trailing_stop" => trail_pct.is_some() || trail_off.is_some(),
            _ => true,
        };
        let form_valid = sym_ok && qty_ok && side_ok && type_ok && price_ok;

        if !sym_ok {
            ui.colored_label(egui::Color32::from_rgb(200, 120, 0), "Invalid symbol");
        }
        if !qty_ok && !self.order_qty_str.is_empty() {
            ui.colored_label(egui::Color32::from_rgb(200, 120, 0), "Qty must be 0 < q <= 100,000");
        }
        if !price_ok {
            ui.colored_label(egui::Color32::from_rgb(200, 120, 0), "Price required for this order type");
        }

        ui.add_space(6.0);

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
                if ui.add(egui::Button::new("SEND").fill(egui::Color32::from_rgb(0, 140, 0))).clicked()
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
                        take_profit: tp,
                        stop_loss: sl,
                        trail_percent: trail_pct,
                        trail_offset: trail_off,
                        risk_mode: Some(self.order_risk_mode.clone()),
                        risk_pct: risk_pct,
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
            let color = if ok { COLOR_UP } else { COLOR_DOWN };
            ui.colored_label(color, format!("Last: {msg}"));
        } else {
            ui.label("No orders sent this session");
        }
    }

    fn render_chart(&mut self, ui: &mut egui::Ui) {
        // Chart controls
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
                    self.subscribe_chart();
                    self.request_indicators();
                }
            }
        });

        // Indicator toggles
        ui.horizontal(|ui| {
            let old_sma = self.show_sma200;
            let old_ema = self.show_ema21;
            let old_rsi = self.show_rsi14;
            ui.checkbox(&mut self.show_sma200, "SMA(200)");
            ui.checkbox(&mut self.show_ema21, "EMA(21)");
            ui.checkbox(&mut self.show_rsi14, "RSI(14)");
            if self.show_sma200 != old_sma || self.show_ema21 != old_ema || self.show_rsi14 != old_rsi {
                self.request_indicators();
            }
        });

        let key = format!("{}:{}", self.current_symbol, self.current_timeframe);
        let bars = match self.bars.get(&key) {
            Some(b) if !b.is_empty() => b,
            _ => {
                ui.centered_and_justified(|ui| {
                    ui.label("Click Load to fetch chart data");
                });
                return;
            }
        };

        // Handle scroll zoom
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta != 0.0 {
            if scroll_delta > 0.0 {
                self.chart_visible_bars = (self.chart_visible_bars * 4 / 5).max(20);
            } else {
                self.chart_visible_bars = (self.chart_visible_bars * 5 / 4).min(bars.len());
            }
        }

        // Compute visible range
        let total = bars.len();
        let visible = self.chart_visible_bars.min(total);
        let start = if total > visible { total - visible - self.chart_offset.min(total - visible) } else { 0 };
        let end = (start + visible).min(total);
        let visible_bars = &bars[start..end];

        if visible_bars.is_empty() {
            ui.label("No visible bars");
            return;
        }

        // Price range for chart
        let mut price_min = f64::MAX;
        let mut price_max = f64::MIN;
        for b in visible_bars {
            if b.low < price_min { price_min = b.low; }
            if b.high > price_max { price_max = b.high; }
        }
        let price_range = price_max - price_min;
        let padding = price_range * 0.05;
        price_min -= padding;
        price_max += padding;

        // Volume max for volume bars
        let vol_max = visible_bars.iter().map(|b| b.volume).fold(0.0f64, f64::max);

        // Allocate chart area
        let rsi_height = if self.show_rsi14 { 60.0 } else { 0.0 };
        let vol_height = 40.0;
        let available_height = ui.available_height() - rsi_height - vol_height - 4.0;
        let chart_height = available_height.max(100.0);

        let (response, painter) = ui.allocate_painter(
            egui::vec2(ui.available_width(), chart_height + vol_height + rsi_height),
            egui::Sense::hover().union(egui::Sense::drag()),
        );
        let rect = response.rect;
        let chart_rect = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), chart_height));
        let vol_rect = egui::Rect::from_min_size(
            egui::pos2(rect.min.x, rect.min.y + chart_height),
            egui::vec2(rect.width(), vol_height),
        );
        let rsi_rect = if self.show_rsi14 {
            Some(egui::Rect::from_min_size(
                egui::pos2(rect.min.x, rect.min.y + chart_height + vol_height),
                egui::vec2(rect.width(), rsi_height),
            ))
        } else {
            None
        };

        // Background
        painter.rect_filled(rect, 0.0, egui::Color32::BLACK);

        // Price grid (5 levels)
        for i in 0..=4 {
            let frac = i as f64 / 4.0;
            let price = price_min + frac * (price_max - price_min);
            let y = chart_rect.max.y - (frac as f32) * chart_rect.height();
            painter.line_segment(
                [egui::pos2(chart_rect.min.x, y), egui::pos2(chart_rect.max.x, y)],
                egui::Stroke::new(0.5, COLOR_GRID),
            );
            painter.text(
                egui::pos2(chart_rect.max.x - 60.0, y - 8.0),
                egui::Align2::LEFT_TOP,
                format!("{price:.2}"),
                egui::FontId::monospace(9.0),
                egui::Color32::from_rgb(140, 140, 160),
            );
        }

        let bar_count = visible_bars.len() as f32;
        let bar_width = (chart_rect.width() / bar_count).max(1.0);
        let candle_width = (bar_width * 0.7).max(1.0);

        // Helper: price to y coordinate
        let price_to_y = |price: f64| -> f32 {
            let frac = (price - price_min) / (price_max - price_min);
            chart_rect.max.y - (frac as f32) * chart_rect.height()
        };

        // Draw candlesticks
        for (i, bar) in visible_bars.iter().enumerate() {
            let x = chart_rect.min.x + (i as f32 + 0.5) * bar_width;
            let is_up = bar.close >= bar.open;
            let color = if is_up { COLOR_UP } else { COLOR_DOWN };

            let body_top = price_to_y(if is_up { bar.close } else { bar.open });
            let body_bot = price_to_y(if is_up { bar.open } else { bar.close });
            let wick_top = price_to_y(bar.high);
            let wick_bot = price_to_y(bar.low);

            // Wick
            painter.line_segment(
                [egui::pos2(x, wick_top), egui::pos2(x, wick_bot)],
                egui::Stroke::new(1.0, color),
            );

            // Body
            let body_height = (body_bot - body_top).max(1.0);
            painter.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(x - candle_width / 2.0, body_top),
                    egui::vec2(candle_width, body_height),
                ),
                0.0,
                color,
            );

            // Volume bar
            if vol_max > 0.0 {
                let vol_frac = (bar.volume / vol_max) as f32;
                let vol_bar_height = vol_frac * vol_rect.height();
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(x - candle_width / 2.0, vol_rect.max.y - vol_bar_height),
                        egui::vec2(candle_width, vol_bar_height),
                    ),
                    0.0,
                    if is_up { COLOR_VOLUME } else { egui::Color32::from_rgb(120, 50, 50) },
                );
            }
        }

        // Draw indicator overlays
        if self.show_sma200 {
            self.draw_indicator_line(&painter, "SMA_200", visible_bars, start, bar_width, &chart_rect, price_min, price_max, COLOR_SMA200);
        }
        if self.show_ema21 {
            self.draw_indicator_line(&painter, "EMA_21", visible_bars, start, bar_width, &chart_rect, price_min, price_max, COLOR_EMA21);
        }

        // Draw RSI sub-pane
        if let Some(rsi_rect) = rsi_rect {
            painter.rect_filled(rsi_rect, 0.0, egui::Color32::from_rgb(10, 10, 10));
            // RSI 70/30 levels
            let y70 = rsi_rect.max.y - (70.0 / 100.0) * rsi_rect.height();
            let y30 = rsi_rect.max.y - (30.0 / 100.0) * rsi_rect.height();
            let y50 = rsi_rect.max.y - (50.0 / 100.0) * rsi_rect.height();
            painter.line_segment(
                [egui::pos2(rsi_rect.min.x, y70), egui::pos2(rsi_rect.max.x, y70)],
                egui::Stroke::new(0.5, egui::Color32::from_rgb(80, 0, 0)),
            );
            painter.line_segment(
                [egui::pos2(rsi_rect.min.x, y30), egui::pos2(rsi_rect.max.x, y30)],
                egui::Stroke::new(0.5, egui::Color32::from_rgb(0, 80, 0)),
            );
            painter.line_segment(
                [egui::pos2(rsi_rect.min.x, y50), egui::pos2(rsi_rect.max.x, y50)],
                egui::Stroke::new(0.5, COLOR_GRID),
            );

            if let Some(values) = self.indicator_data.get("RSI_14") {
                let mut prev: Option<egui::Pos2> = None;
                for (i, _bar) in visible_bars.iter().enumerate() {
                    let data_idx = start + i;
                    if data_idx < values.len() {
                        if let Some(val) = values[data_idx] {
                            let x = rsi_rect.min.x + (i as f32 + 0.5) * bar_width;
                            let y = rsi_rect.max.y - (val as f32 / 100.0) * rsi_rect.height();
                            let pt = egui::pos2(x, y);
                            if let Some(p) = prev {
                                painter.line_segment([p, pt], egui::Stroke::new(1.0, COLOR_RSI));
                            }
                            prev = Some(pt);
                        }
                    }
                }
            }
        }

        // Crosshair
        if let Some(hover_pos) = response.hover_pos() {
            if chart_rect.contains(hover_pos) {
                painter.line_segment(
                    [egui::pos2(hover_pos.x, chart_rect.min.y), egui::pos2(hover_pos.x, chart_rect.max.y)],
                    egui::Stroke::new(0.5, COLOR_CROSSHAIR),
                );
                painter.line_segment(
                    [egui::pos2(chart_rect.min.x, hover_pos.y), egui::pos2(chart_rect.max.x, hover_pos.y)],
                    egui::Stroke::new(0.5, COLOR_CROSSHAIR),
                );

                // Price at cursor
                let frac = (chart_rect.max.y - hover_pos.y) / chart_rect.height();
                let hover_price = price_min + (frac as f64) * (price_max - price_min);
                painter.text(
                    egui::pos2(hover_pos.x + 8.0, hover_pos.y - 12.0),
                    egui::Align2::LEFT_TOP,
                    format!("{hover_price:.4}"),
                    egui::FontId::monospace(10.0),
                    egui::Color32::WHITE,
                );

                // Bar index at cursor
                let bar_idx = ((hover_pos.x - chart_rect.min.x) / bar_width) as usize;
                if bar_idx < visible_bars.len() {
                    let b = &visible_bars[bar_idx];
                    painter.text(
                        egui::pos2(chart_rect.min.x + 4.0, chart_rect.min.y + 2.0),
                        egui::Align2::LEFT_TOP,
                        format!("O:{:.4} H:{:.4} L:{:.4} C:{:.4} V:{:.0}", b.open, b.high, b.low, b.close, b.volume),
                        egui::FontId::monospace(10.0),
                        egui::Color32::from_rgb(200, 200, 200),
                    );
                }
            }
        }

        // Handle drag for panning
        if response.dragged() {
            let delta = response.drag_delta();
            let bars_shift = (delta.x / bar_width) as i32;
            if bars_shift > 0 {
                self.chart_offset = self.chart_offset.saturating_add(bars_shift as usize);
            } else if bars_shift < 0 {
                self.chart_offset = self.chart_offset.saturating_sub((-bars_shift) as usize);
            }
        }
    }

    fn draw_indicator_line(
        &self,
        painter: &egui::Painter,
        name: &str,
        visible_bars: &[BarData],
        start: usize,
        bar_width: f32,
        chart_rect: &egui::Rect,
        price_min: f64,
        price_max: f64,
        color: egui::Color32,
    ) {
        let values = match self.indicator_data.get(name) {
            Some(v) => v,
            None => return,
        };
        let price_range = price_max - price_min;
        if price_range <= 0.0 { return; }

        let mut prev: Option<egui::Pos2> = None;
        for (i, _bar) in visible_bars.iter().enumerate() {
            let data_idx = start + i;
            if data_idx < values.len() {
                if let Some(val) = values[data_idx] {
                    let x = chart_rect.min.x + (i as f32 + 0.5) * bar_width;
                    let frac = (val - price_min) / price_range;
                    let y = chart_rect.max.y - (frac as f32) * chart_rect.height();
                    let pt = egui::pos2(x, y);
                    if let Some(p) = prev {
                        painter.line_segment([p, pt], egui::Stroke::new(1.5, color));
                    }
                    prev = Some(pt);
                }
            }
        }
    }

    fn render_watchlist(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Symbols (comma-separated):");
            ui.add(
                egui::TextEdit::singleline(&mut self.watchlist_symbols_str)
                    .desired_width(200.0),
            );
            if ui.button("Refresh").clicked() {
                let symbols: Vec<String> = self.watchlist_symbols_str
                    .split(',')
                    .map(|s| s.trim().to_uppercase())
                    .filter(|s| !s.is_empty() && is_valid_symbol(s))
                    .collect();
                if !symbols.is_empty() {
                    self.send_cmd(&WebCmd::GetWatchlistQuotes { symbols });
                }
            }
        });

        ui.add_space(4.0);

        if self.watchlist_quotes.is_empty() {
            ui.label("No watchlist data — click Refresh");
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("watchlist_grid")
                .num_columns(5)
                .spacing([16.0, 6.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Symbol");
                    ui.strong("Last");
                    ui.strong("Bid");
                    ui.strong("Ask");
                    ui.strong("Chg %");
                    ui.end_row();

                    for q in &self.watchlist_quotes {
                        ui.label(&q.symbol);
                        ui.label(format!("{:.4}", q.last));
                        ui.label(format!("{:.4}", q.bid));
                        ui.label(format!("{:.4}", q.ask));
                        let color = if q.change_pct >= 0.0 { COLOR_UP } else { COLOR_DOWN };
                        // Background intensity based on magnitude
                        let intensity = (q.change_pct.abs() * 20.0).min(60.0) as u8;
                        let bg = if q.change_pct >= 0.0 {
                            egui::Color32::from_rgba_premultiplied(0, intensity, 0, intensity)
                        } else {
                            egui::Color32::from_rgba_premultiplied(intensity, 0, 0, intensity)
                        };
                        let label = egui::RichText::new(format!("{:+.2}%", q.change_pct))
                            .color(color)
                            .background_color(bg);
                        ui.label(label);
                        ui.end_row();
                    }
                });
        });
    }

    fn render_alerts(&mut self, ui: &mut egui::Ui) {
        ui.heading("Alerts");
        ui.add_space(4.0);

        // Create alert form
        egui::Grid::new("alert_form")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Symbol");
                ui.add(egui::TextEdit::singleline(&mut self.alert_symbol).desired_width(80.0).char_limit(MAX_SYMBOL_LEN));
                ui.end_row();

                ui.label("Condition");
                egui::ComboBox::from_id_salt("alert_cond")
                    .selected_text(&self.alert_condition)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.alert_condition, "crosses_above".into(), "Crosses Above");
                        ui.selectable_value(&mut self.alert_condition, "crosses_below".into(), "Crosses Below");
                        ui.selectable_value(&mut self.alert_condition, "reaches".into(), "Reaches");
                        ui.selectable_value(&mut self.alert_condition, "breaks_above".into(), "Breaks Above");
                        ui.selectable_value(&mut self.alert_condition, "breaks_below".into(), "Breaks Below");
                    });
                ui.end_row();

                ui.label("Price");
                ui.add(egui::TextEdit::singleline(&mut self.alert_price_str).desired_width(80.0).char_limit(16));
                ui.end_row();

                ui.label("Message");
                ui.add(egui::TextEdit::singleline(&mut self.alert_message).desired_width(160.0).char_limit(MAX_ALERT_MSG_LEN));
                ui.end_row();
            });

        ui.add_space(4.0);
        let price = self.alert_price_str.parse::<f64>().ok();
        let valid = is_valid_symbol(&self.alert_symbol)
            && is_valid_alert_condition(&self.alert_condition)
            && price.map(|p| p.is_finite() && p > 0.0).unwrap_or(false);

        if ui.add_enabled(valid, egui::Button::new("Create Alert")).clicked() {
            if let Some(p) = price {
                self.send_cmd(&WebCmd::CreateAlert {
                    symbol: self.alert_symbol.clone(),
                    condition: self.alert_condition.clone(),
                    price: p,
                    message: self.alert_message.clone(),
                });
                self.alert_price_str.clear();
                self.alert_message.clear();
                // Refresh alert list
                self.send_cmd(&WebCmd::ListAlerts);
            }
        }

        ui.add_space(8.0);
        ui.separator();

        // Active alerts
        if self.alerts.is_empty() {
            if ui.button("Load Alerts").clicked() {
                self.send_cmd(&WebCmd::ListAlerts);
            }
            ui.label("No active alerts");
        } else {
            let mut delete_ids: Vec<String> = Vec::new();

            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("alerts_grid")
                    .num_columns(6)
                    .spacing([10.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Symbol");
                        ui.strong("Condition");
                        ui.strong("Price");
                        ui.strong("Message");
                        ui.strong("Active");
                        ui.strong("");
                        ui.end_row();

                        for alert in &self.alerts {
                            ui.label(&alert.symbol);
                            ui.label(&alert.condition);
                            ui.label(format!("{:.4}", alert.price));
                            ui.label(&alert.message);
                            let status = if alert.active { "Yes" } else { "No" };
                            ui.label(status);
                            if ui.button("Delete").clicked() {
                                delete_ids.push(alert.id.clone());
                            }
                            ui.end_row();
                        }
                    });
            });

            for id in delete_ids {
                self.send_cmd(&WebCmd::DeleteAlert { alert_id: id });
                self.send_cmd(&WebCmd::ListAlerts);
            }
        }

        // Triggered alerts history
        if !self.alert_triggered.is_empty() {
            ui.add_space(8.0);
            ui.separator();
            ui.heading("Recent Triggers");
            for (id, sym, msg) in self.alert_triggered.iter().rev().take(10) {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(255, 200, 0),
                        format!("[{id}] {sym}: {msg}"));
                });
            }
        }
    }

    fn render_news(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Symbol (blank for general):");
            ui.add(egui::TextEdit::singleline(&mut self.news_symbol).desired_width(80.0).char_limit(MAX_SYMBOL_LEN));
            if ui.button("Fetch News").clicked() {
                let sym = if self.news_symbol.is_empty() {
                    None
                } else if is_valid_symbol(&self.news_symbol) {
                    Some(self.news_symbol.clone())
                } else {
                    None
                };
                self.send_cmd(&WebCmd::GetNews { symbol: sym });
            }
        });

        ui.add_space(4.0);

        if self.news_items.is_empty() {
            ui.label("No news — click Fetch");
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for item in &self.news_items {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.strong(&item.headline);
                        if let Some(ref sym) = item.symbol {
                            ui.colored_label(egui::Color32::from_rgb(100, 180, 255), sym);
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(&item.source);
                        ui.colored_label(egui::Color32::from_rgb(120, 120, 120),
                            format_timestamp(item.timestamp));
                    });
                    if !item.summary.is_empty() {
                        ui.label(&item.summary);
                    }
                });
                ui.add_space(2.0);
            }
        });
    }
}

fn format_timestamp(ts: i64) -> String {
    // Simple relative display — full date formatting would need chrono in WASM
    let minutes = ts / 60;
    let hours = minutes / 60;
    if hours > 24 {
        format!("{}d ago", hours / 24)
    } else if hours > 0 {
        format!("{hours}h ago")
    } else {
        format!("{minutes}m ago")
    }
}
