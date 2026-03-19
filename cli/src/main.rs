//! TyphooN Terminal CLI — TUI interface for trading, research, and risk management.
//!
//! Full terminal interface using ratatui. Connects to Alpaca Markets via REST API.
//! Shares the same broker logic as the GUI terminal.
//!
//! Usage:
//!   typhoon                     # Interactive TUI mode
//!   typhoon --watch AAPL,MSFT   # Watchlist mode
//!   typhoon --positions         # Show positions and exit
//!   typhoon --account           # Show account info and exit

use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Tabs, Wrap},
    Frame, Terminal,
};
use std::io::stdout;
use std::path::PathBuf;
use std::time::{Duration, Instant};

mod broker;
mod creds;

// ── Multi-Account Registry (MT5 CSV Import) ─────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Mt5Position {
    symbol: String,
    side: String,
    lots: f64,
    price: f64,
    profit: f64,
    commission: f64,
    swap: f64,
    sl: f64,
    tp: f64,
    time: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Mt5Trade {
    symbol: String,
    side: String,
    lots: f64,
    price: f64,
    profit: f64,
    commission: f64,
    swap: f64,
    time: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ImportedAccount {
    name: String,
    #[serde(rename = "type")]
    acct_type: String,
    equity: f64,
    balance: f64,
    currency: String,
    positions: Vec<Mt5Position>,
    history: Vec<Mt5Trade>,
    import_date: String,
}

fn account_registry_path() -> PathBuf {
    let mut p = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("typhoon-terminal");
    std::fs::create_dir_all(&p).ok();
    p.push("account_registry.json");
    p
}

fn load_account_registry() -> Vec<ImportedAccount> {
    let path = account_registry_path();
    if !path.exists() { return vec![]; }
    match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => vec![],
    }
}

fn save_account_registry(reg: &[ImportedAccount]) {
    let path = account_registry_path();
    if let Ok(s) = serde_json::to_string_pretty(reg) {
        std::fs::write(&path, s).ok();
    }
}

/// Parse MT5 Statement CSV into an ImportedAccount
fn parse_mt5_csv(text: &str, account_name: &str) -> ImportedAccount {
    let mut result = ImportedAccount {
        name: account_name.to_string(),
        acct_type: "mt5-import".to_string(),
        equity: 0.0,
        balance: 0.0,
        currency: "USD".to_string(),
        positions: vec![],
        history: vec![],
        import_date: chrono::Local::now().format("%Y-%m-%d").to_string(),
    };

    let lines: Vec<&str> = text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    let mut section = "header";
    let mut section_headers: Vec<String> = vec![];

    for i in 0..lines.len() {
        let line = lines[i];
        let lower = line.to_lowercase();

        // Parse balance/equity from summary lines
        if lower.starts_with("balance") || lower.contains("balance:") || lower.contains("balance\t") {
            if let Some(val) = extract_last_number(line) { result.balance = val; }
        }
        if lower.starts_with("equity") || lower.contains("equity:") || lower.contains("equity\t") {
            if let Some(val) = extract_last_number(line) { result.equity = val; }
        }

        // Currency detection
        for cur in &["EUR", "GBP", "CHF", "JPY", "AUD", "CAD"] {
            if line.contains(cur) && result.currency == "USD" {
                result.currency = cur.to_string();
            }
        }

        // Section detection
        if lower.starts_with("positions") || lower.starts_with("open positions") {
            section = "positions";
            if i + 1 < lines.len() {
                section_headers = parse_csv_row(lines[i + 1]).iter().map(|h| h.to_lowercase()).collect();
            }
            continue;
        }
        if lower.starts_with("deals") || lower.starts_with("closed") || lower.starts_with("trade history") {
            section = "deals";
            if i + 1 < lines.len() {
                section_headers = parse_csv_row(lines[i + 1]).iter().map(|h| h.to_lowercase()).collect();
            }
            continue;
        }

        // Parse data rows
        if (section == "positions" || section == "deals") && !section_headers.is_empty() {
            let vals = parse_csv_row(line);
            if vals.len() < 3 { continue; }
            let row = make_row_map(&section_headers, &vals);

            let symbol = row.get("symbol").or(row.get("instrument")).cloned().unwrap_or_default();
            if symbol.is_empty() || symbol.to_lowercase() == "symbol" { continue; }

            let type_str = row.get("type").or(row.get("action")).or(row.get("side")).cloned().unwrap_or_default().to_lowercase();
            let side = if type_str.contains("buy") { "buy" } else { "sell" }.to_string();
            let lots = parse_f64(row.get("volume").or(row.get("lots")).or(row.get("qty")));
            let price = parse_f64(row.get("price").or(row.get("open price")).or(row.get("entry")));
            let profit = parse_f64(row.get("profit").or(row.get("p/l")).or(row.get("pnl")));
            let commission = parse_f64(row.get("commission").or(row.get("comm")));
            let swap = parse_f64(row.get("swap"));
            let time = row.get("time").or(row.get("open time")).or(row.get("date")).cloned().unwrap_or_default();

            if section == "positions" {
                let sl = parse_f64(row.get("s/l").or(row.get("sl")).or(row.get("stop loss")));
                let tp = parse_f64(row.get("t/p").or(row.get("tp")).or(row.get("take profit")));
                result.positions.push(Mt5Position { symbol, side, lots, price, profit, commission, swap, sl, tp, time });
            } else {
                result.history.push(Mt5Trade { symbol, side, lots, price, profit, commission, swap, time });
            }
        }
    }

    // Fallback: try simple deal CSV (header row + data rows)
    if result.positions.is_empty() && result.history.is_empty() {
        if let Some(headers) = lines.first() {
            let header_vals: Vec<String> = parse_csv_row(headers).iter().map(|h| h.to_lowercase()).collect();
            if header_vals.iter().any(|h| h == "symbol" || h == "deal") {
                for line in &lines[1..] {
                    let vals = parse_csv_row(line);
                    let row = make_row_map(&header_vals, &vals);
                    let symbol = row.get("symbol").cloned().unwrap_or_default();
                    if symbol.is_empty() { continue; }
                    let type_str = row.get("type").cloned().unwrap_or_default().to_lowercase();
                    let side = if type_str.contains("buy") { "buy" } else { "sell" }.to_string();
                    let lots = parse_f64(row.get("volume").or(row.get("lots")));
                    let price = parse_f64(row.get("price"));
                    let profit = parse_f64(row.get("profit"));
                    let commission = parse_f64(row.get("commission"));
                    let swap = parse_f64(row.get("swap"));
                    let time = row.get("time").or(row.get("open time")).cloned().unwrap_or_default();
                    result.history.push(Mt5Trade { symbol, side, lots, price, profit, commission, swap, time });
                }
            }
        }
    }

    if result.equity == 0.0 && result.balance > 0.0 { result.equity = result.balance; }
    result
}

fn parse_csv_row(line: &str) -> Vec<String> {
    let mut vals = vec![];
    let mut in_quote = false;
    let mut current = String::new();
    for ch in line.chars() {
        if ch == '"' { in_quote = !in_quote; continue; }
        if (ch == ',' || ch == '\t') && !in_quote { vals.push(current.trim().to_string()); current.clear(); continue; }
        current.push(ch);
    }
    vals.push(current.trim().to_string());
    vals
}

fn make_row_map(headers: &[String], vals: &[String]) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for (i, h) in headers.iter().enumerate() {
        if let Some(v) = vals.get(i) { map.insert(h.clone(), v.clone()); }
    }
    map
}

fn parse_f64(val: Option<&String>) -> f64 {
    val.and_then(|s| s.replace(',', "").parse::<f64>().ok()).unwrap_or(0.0)
}

fn extract_last_number(line: &str) -> Option<f64> {
    let re_like: Vec<&str> = line.split(|c: char| !c.is_ascii_digit() && c != '.' && c != ',')
        .filter(|s| !s.is_empty() && s.contains('.'))
        .collect();
    re_like.last().and_then(|s| s.replace(',', "").parse::<f64>().ok())
}

#[derive(Parser)]
#[command(name = "typhoon", about = "TyphooN Terminal CLI — trading terminal for your terminal")]
struct Args {
    /// API key (or set ALPACA_API_KEY env var)
    #[arg(long, env = "ALPACA_API_KEY")]
    api_key: Option<String>,

    /// Secret key (or set ALPACA_SECRET_KEY env var)
    #[arg(long, env = "ALPACA_SECRET_KEY")]
    secret_key: Option<String>,

    /// Paper trading (default: true)
    #[arg(long, default_value = "true")]
    paper: bool,

    /// Watch symbols (comma-separated)
    #[arg(long, short = 'w')]
    watch: Option<String>,

    /// Show positions and exit
    #[arg(long)]
    positions: bool,

    /// Show account info and exit
    #[arg(long)]
    account: bool,

    /// Show all accounts (Alpaca + imported MT5) and exit
    #[arg(long)]
    accounts: bool,

    /// Import MT5 CSV as named account (e.g., --import-mt5 DARWIN_EUR:/path/to/statement.csv)
    #[arg(long)]
    import_mt5: Option<String>,

    /// Symbol to load on startup
    #[arg(long, short = 's')]
    symbol: Option<String>,
}

/// App state
struct App {
    broker: broker::AlpacaBroker,
    // Tabs
    active_tab: usize,
    tabs: Vec<&'static str>,
    // Data
    account: Option<broker::AccountInfo>,
    positions: Vec<broker::PositionInfo>,
    orders: Vec<broker::OrderInfo>,
    watchlist: Vec<String>,
    watchlist_prices: Vec<(String, f64, f64)>, // (symbol, price, change%)
    // Chart
    chart_symbol: String,
    chart_bars: Vec<broker::Bar>,
    chart_timeframe: String,
    // Multi-Account
    imported_accounts: Vec<ImportedAccount>,
    // Selection state (for interactive list navigation)
    selected_position: usize,
    selected_order: usize,
    // Action confirmation
    pending_action: Option<String>, // "close:SYMBOL" or "cancel:ORDER_ID"
    // Command
    command_input: String,
    command_mode: bool,
    // Log
    log_messages: Vec<(String, Color)>,
    // Refresh
    last_refresh: Instant,
    refresh_interval: Duration,
}

impl App {
    fn new(broker: broker::AlpacaBroker, symbol: String, watchlist: Vec<String>) -> Self {
        let imported_accounts = load_account_registry();
        Self {
            broker,
            active_tab: 0,
            tabs: vec!["Dashboard", "Chart", "Positions", "Orders", "Watchlist", "Accounts", "Command"],
            account: None,
            positions: vec![],
            orders: vec![],
            watchlist,
            watchlist_prices: vec![],
            imported_accounts,
            chart_symbol: symbol,
            chart_bars: vec![],
            chart_timeframe: "1Day".to_string(),
            selected_position: 0,
            selected_order: 0,
            pending_action: None,
            command_input: String::new(),
            command_mode: false,
            log_messages: vec![
                ("TyphooN Terminal CLI v0.1.0".to_string(), Color::Cyan),
                ("Press Tab to switch views, : for command mode, q to quit".to_string(), Color::DarkGray),
            ],
            last_refresh: Instant::now() - Duration::from_secs(60), // force initial refresh
            refresh_interval: Duration::from_secs(5),
        }
    }

    fn log(&mut self, msg: &str, color: Color) {
        let ts = chrono::Local::now().format("%H:%M:%S").to_string();
        self.log_messages.push((format!("[{ts}] {msg}"), color));
        if self.log_messages.len() > 100 { self.log_messages.remove(0); }
    }

    async fn refresh(&mut self) {
        if self.last_refresh.elapsed() < self.refresh_interval { return; }
        self.last_refresh = Instant::now();

        // Account
        match self.broker.get_account().await {
            Ok(a) => { self.account = Some(a); }
            Err(e) => { self.log(&format!("Account error: {e}"), Color::Red); }
        }

        // Positions
        match self.broker.get_positions().await {
            Ok(p) => { self.positions = p; }
            Err(e) => { self.log(&format!("Positions error: {e}"), Color::Red); }
        }

        // Orders
        match self.broker.get_orders("open", 50).await {
            Ok(o) => { self.orders = o; }
            Err(e) => { self.log(&format!("Orders error: {e}"), Color::Red); }
        }

        // Chart bars
        if !self.chart_symbol.is_empty() {
            match self.broker.get_bars(&self.chart_symbol, &self.chart_timeframe, 100).await {
                Ok(bars) => {
                    self.chart_bars = bars;
                    self.log(&format!("Loaded {} bars for {} @ {}", self.chart_bars.len(), self.chart_symbol, self.chart_timeframe), Color::Green);
                }
                Err(e) => { self.log(&format!("Bars error: {e}"), Color::Red); }
            }
        }
    }

    async fn handle_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.trim().split_whitespace().collect();
        if parts.is_empty() { return; }

        match parts[0].to_lowercase().as_str() {
            "buy" | "b" => {
                if parts.len() < 3 {
                    self.log("Usage: buy SYMBOL QTY", Color::Yellow);
                } else {
                    let symbol = parts[1].to_uppercase();
                    let qty: f64 = parts[2].parse().unwrap_or(0.0);
                    if qty <= 0.0 { self.log("Invalid qty", Color::Red); return; }
                    match self.broker.market_order(&symbol, qty, "buy").await {
                        Ok(r) => self.log(&format!("BUY {qty} {symbol}: {}", r.status), Color::Green),
                        Err(e) => self.log(&format!("Order failed: {e}"), Color::Red),
                    }
                }
            }
            "sell" | "s" => {
                if parts.len() < 3 {
                    self.log("Usage: sell SYMBOL QTY", Color::Yellow);
                } else {
                    let symbol = parts[1].to_uppercase();
                    let qty: f64 = parts[2].parse().unwrap_or(0.0);
                    if qty <= 0.0 { self.log("Invalid qty", Color::Red); return; }
                    match self.broker.market_order(&symbol, qty, "sell").await {
                        Ok(r) => self.log(&format!("SELL {qty} {symbol}: {}", r.status), Color::Green),
                        Err(e) => self.log(&format!("Order failed: {e}"), Color::Red),
                    }
                }
            }
            "close" | "c" => {
                if parts.len() < 2 {
                    self.log("Usage: close SYMBOL [QTY]", Color::Yellow);
                } else {
                    let symbol = parts[1].to_uppercase();
                    let qty = parts.get(2).and_then(|s| s.parse::<f64>().ok());
                    match self.broker.close_position(&symbol, qty).await {
                        Ok(r) => self.log(&format!("CLOSE {symbol}: {}", r.status), Color::Green),
                        Err(e) => self.log(&format!("Close failed: {e}"), Color::Red),
                    }
                }
            }
            "chart" | "ch" => {
                if parts.len() >= 2 {
                    self.chart_symbol = parts[1].to_uppercase();
                    if parts.len() >= 3 { self.chart_timeframe = parts[2].to_string(); }
                    self.last_refresh = Instant::now() - Duration::from_secs(60); // force refresh
                    self.active_tab = 1; // switch to chart tab
                    self.log(&format!("Chart: {} @ {}", self.chart_symbol, self.chart_timeframe), Color::Cyan);
                } else {
                    self.log("Usage: chart SYMBOL [TIMEFRAME]", Color::Yellow);
                }
            }
            "watch" | "w" => {
                if parts.len() >= 2 {
                    let sym = parts[1].to_uppercase();
                    if !self.watchlist.contains(&sym) {
                        self.watchlist.push(sym.clone());
                        self.log(&format!("Added {sym} to watchlist"), Color::Green);
                    }
                } else {
                    self.log("Usage: watch SYMBOL", Color::Yellow);
                }
            }
            "tf" => {
                if parts.len() >= 2 {
                    self.chart_timeframe = parts[1].to_string();
                    self.last_refresh = Instant::now() - Duration::from_secs(60);
                    self.log(&format!("Timeframe: {}", self.chart_timeframe), Color::Cyan);
                }
            }
            "import" => {
                // import ACCOUNT_NAME /path/to/file.csv
                if parts.len() < 3 {
                    self.log("Usage: import ACCOUNT_NAME /path/to/file.csv", Color::Yellow);
                } else {
                    let name = parts[1].to_string();
                    let path = parts[2..].join(" ");
                    match std::fs::read_to_string(&path) {
                        Ok(text) => {
                            let acct = parse_mt5_csv(&text, &name);
                            let pos_count = acct.positions.len();
                            let hist_count = acct.history.len();
                            let equity = acct.equity;
                            let currency = acct.currency.clone();

                            // Remove existing with same name
                            self.imported_accounts.retain(|a| a.name != name);
                            self.imported_accounts.push(acct);
                            save_account_registry(&self.imported_accounts);

                            self.log(&format!(
                                "Imported \"{name}\": {pos_count} positions, {hist_count} trades, equity {currency} {equity:.2}"
                            ), Color::Green);
                            self.active_tab = 5; // switch to Accounts tab
                        }
                        Err(e) => self.log(&format!("Failed to read {path}: {e}"), Color::Red),
                    }
                }
            }
            "accounts" | "acct" => {
                self.active_tab = 5; // switch to Accounts tab
                self.log("Switched to Accounts view", Color::Cyan);
            }
            "rmacct" => {
                if parts.len() < 2 {
                    self.log("Usage: rmacct ACCOUNT_NAME", Color::Yellow);
                } else {
                    let name = parts[1];
                    let before = self.imported_accounts.len();
                    self.imported_accounts.retain(|a| a.name != name);
                    if self.imported_accounts.len() < before {
                        save_account_registry(&self.imported_accounts);
                        self.log(&format!("Removed account \"{name}\""), Color::Yellow);
                    } else {
                        self.log(&format!("Account \"{name}\" not found"), Color::Red);
                    }
                }
            }
            "help" | "h" | "?" => {
                self.log("Commands: buy/sell SYMBOL QTY, close SYMBOL, chart SYMBOL [TF],", Color::Cyan);
                self.log("  watch SYMBOL, tf TIMEFRAME, help, quit", Color::Cyan);
                self.log("  import NAME /path/to.csv, accounts, rmacct NAME", Color::Cyan);
                self.log("Tabs: 1-7 or Tab to cycle. : to enter command mode.", Color::Cyan);
            }
            "quit" | "q" | "exit" => {
                // Handled in main loop
            }
            _ => {
                self.log(&format!("Unknown command: {}", parts[0]), Color::Red);
            }
        }
    }
}

fn draw_dashboard(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(5), Constraint::Length(5)])
        .split(area);

    // Account info
    let account_text = if let Some(ref a) = app.account {
        vec![
            Line::from(vec![
                Span::styled("Equity: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("${:.2}", a.equity), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw("  "),
                Span::styled("Cash: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("${:.2}", a.cash), Style::default().fg(Color::Cyan)),
                Span::raw("  "),
                Span::styled("Buying Power: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("${:.2}", a.buying_power), Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::styled("Portfolio: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("${:.2}", a.portfolio_value), Style::default().fg(Color::White)),
                Span::raw("  "),
                Span::styled("Margin: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("${:.2}", a.initial_margin), Style::default().fg(Color::Magenta)),
                Span::raw("  "),
                Span::styled(if a.pattern_day_trader { "PDT" } else { "" }, Style::default().fg(Color::Red)),
            ]),
        ]
    } else {
        vec![Line::from(Span::styled("Connecting...", Style::default().fg(Color::Yellow)))]
    };
    let account_block = Paragraph::new(account_text)
        .block(Block::default().borders(Borders::ALL).title(" Account "));
    f.render_widget(account_block, chunks[0]);

    // Positions summary
    let pos_rows: Vec<Row> = app.positions.iter().map(|p| {
        let pl_color = if p.unrealized_pl >= 0.0 { Color::Green } else { Color::Red };
        let price = if p.qty.abs() > 0.0 { p.market_value.abs() / p.qty.abs() } else { p.avg_entry_price };
        Row::new(vec![
            Cell::from(p.symbol.clone()).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Cell::from(format!("{} {:.0}", if p.qty > 0.0 { "L" } else { "S" }, p.qty.abs())).style(Style::default().fg(if p.qty > 0.0 { Color::Green } else { Color::Red })),
            Cell::from(format!("${:.2}", price)).style(Style::default().fg(Color::Cyan)),
            Cell::from(format!("${:.2}", p.market_value.abs())).style(Style::default().fg(Color::White)),
            Cell::from(format!("{:+.2}", p.unrealized_pl)).style(Style::default().fg(pl_color)),
        ])
    }).collect();

    let pos_table = Table::new(
        pos_rows,
        [Constraint::Length(10), Constraint::Length(10), Constraint::Length(12), Constraint::Length(14), Constraint::Length(12)],
    )
    .header(Row::new(vec!["Symbol", "Side/Qty", "Price", "Mkt Value", "P&L"]).style(Style::default().fg(Color::DarkGray)))
    .block(Block::default().borders(Borders::ALL).title(format!(" Positions ({}) ", app.positions.len())));
    f.render_widget(pos_table, chunks[1]);

    // Total P&L
    let total_pl: f64 = app.positions.iter().map(|p| p.unrealized_pl).sum();
    let pl_color = if total_pl >= 0.0 { Color::Green } else { Color::Red };
    let pl_text = Paragraph::new(Line::from(vec![
        Span::styled("Total P&L: ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("${:+.2}", total_pl), Style::default().fg(pl_color).add_modifier(Modifier::BOLD)),
    ]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(pl_text, chunks[2]);
}

fn draw_chart(f: &mut Frame, app: &App, area: Rect) {
    if app.chart_bars.is_empty() {
        let msg = Paragraph::new("No chart data. Use :chart SYMBOL to load.")
            .block(Block::default().borders(Borders::ALL).title(" Chart "));
        f.render_widget(msg, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(3)])
        .split(area);

    // ASCII candlestick chart using braille characters
    let bars = &app.chart_bars;
    let min_price = bars.iter().map(|b| b.low).fold(f64::MAX, f64::min);
    let max_price = bars.iter().map(|b| b.high).fold(f64::MIN, f64::max);
    let price_range = max_price - min_price;
    if price_range <= 0.0 { return; }

    let chart_width = chunks[0].width.saturating_sub(2) as usize;
    let chart_height = chunks[0].height.saturating_sub(2) as usize;
    let visible_bars = bars.len().min(chart_width);
    let start = bars.len().saturating_sub(visible_bars);

    let mut lines: Vec<Line> = Vec::new();
    for row in 0..chart_height {
        let y_price = max_price - (row as f64 / chart_height as f64) * price_range;
        let mut spans: Vec<Span> = Vec::new();

        for col in 0..visible_bars {
            let bar = &bars[start + col];
            let high_row = ((max_price - bar.high) / price_range * chart_height as f64) as usize;
            let low_row = ((max_price - bar.low) / price_range * chart_height as f64) as usize;
            let open_row = ((max_price - bar.open) / price_range * chart_height as f64) as usize;
            let close_row = ((max_price - bar.close) / price_range * chart_height as f64) as usize;
            let body_top = open_row.min(close_row);
            let body_bot = open_row.max(close_row);
            let bullish = bar.close >= bar.open;
            let color = if bullish { Color::Green } else { Color::Red };

            let ch = if row >= body_top && row <= body_bot {
                // Body
                Span::styled("█", Style::default().fg(color))
            } else if row >= high_row && row <= low_row {
                // Wick
                Span::styled("│", Style::default().fg(Color::DarkGray))
            } else {
                Span::raw(" ")
            };
            spans.push(ch);
        }

        // Price label on right
        if row % 4 == 0 {
            spans.push(Span::styled(format!(" {:.2}", y_price), Style::default().fg(Color::DarkGray)));
        }

        lines.push(Line::from(spans));
    }

    let last = bars.last().unwrap();
    let change = last.close - last.open;
    let change_pct = if last.open > 0.0 { change / last.open * 100.0 } else { 0.0 };
    let title = format!(
        " {} @ {} | O:{:.2} H:{:.2} L:{:.2} C:{:.2} | {:+.2} ({:+.2}%) ",
        app.chart_symbol, app.chart_timeframe,
        last.open, last.high, last.low, last.close,
        change, change_pct,
    );

    let chart_widget = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(chart_widget, chunks[0]);

    // Volume bar at bottom
    let max_vol = bars[start..].iter().map(|b| b.volume).fold(0.0f64, f64::max);
    let vol_spans: Vec<Span> = bars[start..].iter().map(|b| {
        let h = if max_vol > 0.0 { (b.volume / max_vol * 8.0) as usize } else { 0 };
        let ch = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"][h.min(7)];
        let color = if b.close >= b.open { Color::Green } else { Color::Red };
        Span::styled(ch, Style::default().fg(color))
    }).collect();
    let vol_line = Paragraph::new(Line::from(vol_spans))
        .block(Block::default().borders(Borders::ALL).title(" Volume "));
    f.render_widget(vol_line, chunks[1]);
}

fn draw_orders(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let rows: Vec<Row> = app.orders.iter().enumerate().map(|(i, o)| {
        let side_color = if o.side == "buy" { Color::Green } else { Color::Red };
        let selected = i == app.selected_order;
        let row_style = if selected { Style::default().bg(Color::DarkGray) } else { Style::default() };
        let marker = if selected { "▸ " } else { "  " };
        let price_str = o.limit_price.as_deref()
            .or(o.stop_price.as_deref())
            .unwrap_or("market");
        Row::new(vec![
            Cell::from(format!("{}{}", marker, o.symbol)).style(Style::default().fg(Color::White)),
            Cell::from(o.side.clone()).style(Style::default().fg(side_color)),
            Cell::from(o.order_type.clone()).style(Style::default().fg(Color::Cyan)),
            Cell::from(o.qty.clone()).style(Style::default().fg(Color::Yellow)),
            Cell::from(price_str.to_string()).style(Style::default().fg(Color::Magenta)),
            Cell::from(o.status.clone()).style(Style::default().fg(Color::DarkGray)),
        ]).style(row_style)
    }).collect();

    let table = Table::new(
        rows,
        [Constraint::Length(12), Constraint::Length(6), Constraint::Length(10), Constraint::Length(10), Constraint::Length(12), Constraint::Length(12)],
    )
    .header(Row::new(vec!["  Symbol", "Side", "Type", "Qty", "Price", "Status"]).style(Style::default().fg(Color::DarkGray)))
    .block(Block::default().borders(Borders::ALL).title(format!(" Open Orders ({}) — ↑↓ select, d=cancel ", app.orders.len())));
    f.render_widget(table, chunks[0]);

    // Selected order detail
    if !app.orders.is_empty() {
        let sel = &app.orders[app.selected_order.min(app.orders.len() - 1)];
        let detail = Paragraph::new(Line::from(vec![
            Span::styled(format!("{} ", sel.symbol), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} {} {} ", sel.side, sel.order_type, sel.qty), Style::default().fg(Color::White)),
            Span::styled(format!("ID: {} ", &sel.id[..sel.id.len().min(8)]), Style::default().fg(Color::DarkGray)),
            Span::styled(&sel.created_at[..sel.created_at.len().min(19)], Style::default().fg(Color::DarkGray)),
        ]))
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(detail, chunks[1]);
    }
}

fn draw_accounts(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(5), Constraint::Length(8)])
        .split(area);

    // Aggregate summary
    let alpaca_equity = app.account.as_ref().map(|a| a.equity).unwrap_or(0.0);
    let alpaca_pl: f64 = app.positions.iter().map(|p| p.unrealized_pl).sum();
    let alpaca_pos_count = app.positions.len();
    let import_equity: f64 = app.imported_accounts.iter().map(|a| a.equity).sum();
    let import_pl: f64 = app.imported_accounts.iter().map(|a| {
        a.positions.iter().map(|p| p.profit).sum::<f64>() + a.history.iter().map(|h| h.profit).sum::<f64>()
    }).sum();
    let import_pos: usize = app.imported_accounts.iter().map(|a| a.positions.len()).sum();
    let total_equity = alpaca_equity + import_equity;
    let total_pl = alpaca_pl + import_pl;
    let total_pos = alpaca_pos_count + import_pos;

    let summary_text = vec![
        Line::from(vec![
            Span::styled("Total Equity: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("${:.2}", total_equity), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled("Total P&L: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("${:+.2}", total_pl), Style::default().fg(if total_pl >= 0.0 { Color::Green } else { Color::Red }).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled("Positions: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", total_pos), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled(format!("Accounts: 1 Alpaca + {} imported", app.imported_accounts.len()), Style::default().fg(Color::DarkGray)),
            Span::raw("  "),
            Span::styled("Use :import NAME /path.csv to add | :rmacct NAME to remove", Style::default().fg(Color::DarkGray)),
        ]),
    ];
    let summary_block = Paragraph::new(summary_text)
        .block(Block::default().borders(Borders::ALL).title(" Aggregate Portfolio "));
    f.render_widget(summary_block, chunks[0]);

    // Account table
    let mut rows: Vec<Row> = vec![];

    // Alpaca row
    if app.account.is_some() {
        let alpaca_name = "Alpaca (Paper)";
        let pl_color = if alpaca_pl >= 0.0 { Color::Green } else { Color::Red };
        rows.push(Row::new(vec![
            Cell::from(alpaca_name).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Cell::from("Alpaca").style(Style::default().fg(Color::Green)),
            Cell::from("USD").style(Style::default().fg(Color::DarkGray)),
            Cell::from(format!("${:.2}", alpaca_equity)).style(Style::default().fg(Color::White)),
            Cell::from(format!("{}", alpaca_pos_count)).style(Style::default().fg(Color::Cyan)),
            Cell::from(format!("{:+.2}", alpaca_pl)).style(Style::default().fg(pl_color)),
            Cell::from("Connected").style(Style::default().fg(Color::Green)),
        ]));
    }

    // Imported account rows
    for acct in &app.imported_accounts {
        let acct_pl: f64 = acct.positions.iter().map(|p| p.profit).sum::<f64>()
            + acct.history.iter().map(|h| h.profit).sum::<f64>();
        let pl_color = if acct_pl >= 0.0 { Color::Green } else { Color::Red };
        let pos_count = acct.positions.len();
        rows.push(Row::new(vec![
            Cell::from(acct.name.clone()).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Cell::from("MT5 Import").style(Style::default().fg(Color::Yellow)),
            Cell::from(acct.currency.clone()).style(Style::default().fg(Color::DarkGray)),
            Cell::from(format!("{:.2}", acct.equity)).style(Style::default().fg(Color::White)),
            Cell::from(format!("{}", pos_count)).style(Style::default().fg(Color::Cyan)),
            Cell::from(format!("{:+.2}", acct_pl)).style(Style::default().fg(pl_color)),
            Cell::from(format!("Imported {}", &acct.import_date)).style(Style::default().fg(Color::Yellow)),
        ]));
    }

    let account_table = Table::new(
        rows,
        [Constraint::Length(18), Constraint::Length(12), Constraint::Length(5), Constraint::Length(14), Constraint::Length(5), Constraint::Length(14), Constraint::Length(16)],
    )
    .header(Row::new(vec!["Account", "Type", "Cur", "Equity", "Pos", "P&L", "Status"]).style(Style::default().fg(Color::DarkGray)))
    .block(Block::default().borders(Borders::ALL).title(format!(
        " Accounts ({}) ",
        1 + app.imported_accounts.len()
    )));
    f.render_widget(account_table, chunks[1]);

    // Weight breakdown
    let mut weight_lines: Vec<Line> = vec![];
    if total_equity > 0.0 {
        if alpaca_equity > 0.0 {
            let pct = alpaca_equity / total_equity * 100.0;
            let bar_len = (pct / 100.0 * 40.0) as usize;
            weight_lines.push(Line::from(vec![
                Span::styled(format!("{:<16}", "Alpaca"), Style::default().fg(Color::Cyan)),
                Span::styled("█".repeat(bar_len), Style::default().fg(Color::Green)),
                Span::styled(format!(" {:.1}%", pct), Style::default().fg(Color::DarkGray)),
            ]));
        }
        for acct in &app.imported_accounts {
            let pct = acct.equity / total_equity * 100.0;
            let bar_len = (pct / 100.0 * 40.0) as usize;
            weight_lines.push(Line::from(vec![
                Span::styled(format!("{:<16}", &acct.name), Style::default().fg(Color::Yellow)),
                Span::styled("█".repeat(bar_len), Style::default().fg(Color::Yellow)),
                Span::styled(format!(" {:.1}%", pct), Style::default().fg(Color::DarkGray)),
            ]));
        }
    } else {
        weight_lines.push(Line::from(Span::styled("No account data", Style::default().fg(Color::DarkGray))));
    }
    let weight_widget = Paragraph::new(weight_lines)
        .block(Block::default().borders(Borders::ALL).title(" Account Weights "));
    f.render_widget(weight_widget, chunks[2]);
}

fn draw_log(f: &mut Frame, app: &App, area: Rect) {
    let visible = app.log_messages.len().min(area.height as usize);
    let start = app.log_messages.len().saturating_sub(visible);
    let lines: Vec<Line> = app.log_messages[start..].iter()
        .map(|(msg, color)| Line::from(Span::styled(msg.clone(), Style::default().fg(*color))))
        .collect();

    let log_widget = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Log "))
        .wrap(Wrap { trim: false });
    f.render_widget(log_widget, area);
}

fn draw(f: &mut Frame, app: &App) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(10), Constraint::Length(3), Constraint::Length(8)])
        .split(f.area());

    // Tab bar
    let tab_titles: Vec<Line> = app.tabs.iter().map(|t| Line::from(*t)).collect();
    let tab_widget = Tabs::new(tab_titles)
        .select(app.active_tab)
        .highlight_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).title(" TyphooN Terminal CLI "));
    f.render_widget(tab_widget, main_layout[0]);

    // Main content
    match app.active_tab {
        0 => draw_dashboard(f, app, main_layout[1]),
        1 => draw_chart(f, app, main_layout[1]),
        2 => {
            // Interactive positions view with selection
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(5), Constraint::Length(3)])
                .split(main_layout[1]);

            let rows: Vec<Row> = app.positions.iter().enumerate().map(|(i, p)| {
                let pl_color = if p.unrealized_pl >= 0.0 { Color::Green } else { Color::Red };
                let price = if p.qty.abs() > 0.0 { p.market_value.abs() / p.qty.abs() } else { p.avg_entry_price };
                let selected = i == app.selected_position;
                let row_style = if selected { Style::default().bg(Color::DarkGray) } else { Style::default() };
                let marker = if selected { "▸ " } else { "  " };
                Row::new(vec![
                    Cell::from(format!("{}{}", marker, p.symbol)).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Cell::from(p.side.clone()).style(Style::default().fg(if p.side == "long" { Color::Green } else { Color::Red })),
                    Cell::from(format!("{:.0}", p.qty.abs())),
                    Cell::from(format!("${:.2}", p.avg_entry_price)).style(Style::default().fg(Color::Cyan)),
                    Cell::from(format!("${:.2}", price)).style(Style::default().fg(Color::White)),
                    Cell::from(format!("${:.2}", p.market_value.abs())),
                    Cell::from(format!("{:+.2}", p.unrealized_pl)).style(Style::default().fg(pl_color)),
                ]).style(row_style)
            }).collect();
            let table = Table::new(
                rows,
                [Constraint::Length(12), Constraint::Length(6), Constraint::Length(8), Constraint::Length(12), Constraint::Length(12), Constraint::Length(14), Constraint::Length(12)],
            )
            .header(Row::new(vec!["  Symbol", "Side", "Qty", "Entry", "Current", "Mkt Value", "P&L"]).style(Style::default().fg(Color::DarkGray)))
            .block(Block::default().borders(Borders::ALL).title(format!(" Positions ({}) — ↑↓ select, Enter=chart, x=close, p=partial ", app.positions.len())));
            f.render_widget(table, chunks[0]);

            // Selected position detail
            if !app.positions.is_empty() {
                let sel = &app.positions[app.selected_position.min(app.positions.len() - 1)];
                let price = if sel.qty.abs() > 0.0 { sel.market_value.abs() / sel.qty.abs() } else { sel.avg_entry_price };
                let pl_pct = if sel.avg_entry_price > 0.0 { (price / sel.avg_entry_price - 1.0) * 100.0 } else { 0.0 };
                let detail = Paragraph::new(Line::from(vec![
                    Span::styled(format!("{} ", sel.symbol), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{} {:.0} @ ${:.2} ", sel.side, sel.qty.abs(), sel.avg_entry_price), Style::default().fg(Color::White)),
                    Span::styled(format!("→ ${:.2} ({:+.1}%) ", price, pl_pct), Style::default().fg(if pl_pct >= 0.0 { Color::Green } else { Color::Red })),
                    Span::styled(format!("P&L: {:+.2}", sel.unrealized_pl), Style::default().fg(if sel.unrealized_pl >= 0.0 { Color::Green } else { Color::Red }).add_modifier(Modifier::BOLD)),
                ]))
                .block(Block::default().borders(Borders::ALL));
                f.render_widget(detail, chunks[1]);
            }
        }
        3 => draw_orders(f, app, main_layout[1]),
        4 => {
            // Watchlist
            let lines: Vec<Line> = app.watchlist.iter().map(|s| {
                Line::from(Span::styled(s.clone(), Style::default().fg(Color::Cyan)))
            }).collect();
            let wl = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title(" Watchlist "));
            f.render_widget(wl, main_layout[1]);
        }
        5 => draw_accounts(f, app, main_layout[1]),
        6 => draw_log(f, app, main_layout[1]),
        _ => {}
    }

    // Command input
    let cmd_style = if app.command_mode { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::DarkGray) };
    let cmd_text = if app.command_mode {
        format!(":{}", app.command_input)
    } else {
        "Press : to enter command mode | Tab: switch views | q: quit".to_string()
    };
    let cmd_widget = Paragraph::new(cmd_text)
        .style(cmd_style)
        .block(Block::default().borders(Borders::ALL).title(" Command "));
    f.render_widget(cmd_widget, main_layout[2]);

    // Log (always visible)
    draw_log(f, app, main_layout[3]);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Load keys: CLI args → env vars → GUI terminal's encrypted storage
    let (api_key, secret_key) = match (args.api_key.clone(), args.secret_key.clone()) {
        (Some(k), Some(s)) => (k, s),
        _ => {
            // Try env vars
            match (std::env::var("ALPACA_API_KEY").ok(), std::env::var("ALPACA_SECRET_KEY").ok()) {
                (Some(k), Some(s)) => (k, s),
                _ => {
                    // Try GUI terminal's encrypted credential storage
                    match creds::load_saved_credentials(args.paper) {
                        Some((k, s, name)) => {
                            eprintln!("Using saved credentials: {} ({})", name, if args.paper { "paper" } else { "live" });
                            (k, s)
                        }
                        None => {
                            eprintln!("No API keys found. Provide via --api-key/--secret-key, ALPACA_API_KEY/ALPACA_SECRET_KEY env vars, or save credentials in the GUI terminal (Ctrl+K → SETTINGS).");
                            std::process::exit(1);
                        }
                    }
                }
            }
        }
    };

    let broker = broker::AlpacaBroker::new(&api_key, &secret_key, args.paper);

    // One-shot modes
    if args.account {
        let a = broker.get_account().await?;
        println!("Equity: ${:.2} | Cash: ${:.2} | BP: ${:.2} | Portfolio: ${:.2}",
            a.equity, a.cash, a.buying_power, a.portfolio_value);
        return Ok(());
    }
    if args.positions {
        let positions = broker.get_positions().await?;
        if positions.is_empty() { println!("No open positions."); return Ok(()); }
        println!("{:<10} {:<6} {:<8} {:<12} {:<12} {:<12}", "Symbol", "Side", "Qty", "Entry", "MktVal", "P&L");
        for p in &positions {
            let price = if p.qty.abs() > 0.0 { p.market_value.abs() / p.qty.abs() } else { p.avg_entry_price };
            println!("{:<10} {:<6} {:<8.0} ${:<11.2} ${:<11.2} {:+.2}",
                p.symbol, p.side, p.qty.abs(), price, p.market_value.abs(), p.unrealized_pl);
        }
        return Ok(());
    }

    // Import MT5 CSV one-shot mode
    if let Some(ref import_spec) = args.import_mt5 {
        let parts: Vec<&str> = import_spec.splitn(2, ':').collect();
        if parts.len() != 2 {
            eprintln!("Usage: --import-mt5 ACCOUNT_NAME:/path/to/statement.csv");
            std::process::exit(1);
        }
        let name = parts[0];
        let path = parts[1];
        match std::fs::read_to_string(path) {
            Ok(text) => {
                let acct = parse_mt5_csv(&text, name);
                let mut registry = load_account_registry();
                registry.retain(|a| a.name != name);
                println!("Imported \"{}\": {} positions, {} trades, equity {} {:.2}",
                    name, acct.positions.len(), acct.history.len(), acct.currency, acct.equity);
                registry.push(acct);
                save_account_registry(&registry);
            }
            Err(e) => { eprintln!("Failed to read {}: {}", path, e); std::process::exit(1); }
        }
        return Ok(());
    }

    // Show all accounts one-shot mode
    if args.accounts {
        let registry = load_account_registry();
        // Alpaca
        match broker.get_account().await {
            Ok(a) => {
                let positions = broker.get_positions().await.unwrap_or_default();
                let pl: f64 = positions.iter().map(|p| p.unrealized_pl).sum();
                println!("{:<18} {:<12} {:<5} {:>14} {:>5} {:>14} {:<12}",
                    "Account", "Type", "Cur", "Equity", "Pos", "P&L", "Status");
                println!("{:-<80}", "");
                println!("{:<18} {:<12} {:<5} {:>14.2} {:>5} {:>14.2} {:<12}",
                    "Alpaca", "Alpaca", "USD", a.equity, positions.len(), pl, "Connected");
                for acct in &registry {
                    let acct_pl: f64 = acct.positions.iter().map(|p| p.profit).sum::<f64>()
                        + acct.history.iter().map(|h| h.profit).sum::<f64>();
                    println!("{:<18} {:<12} {:<5} {:>14.2} {:>5} {:>14.2} {:<12}",
                        acct.name, "MT5 Import", acct.currency, acct.equity,
                        acct.positions.len(), acct_pl, format!("Imported {}", acct.import_date));
                }
                let total_equity = a.equity + registry.iter().map(|a| a.equity).sum::<f64>();
                let total_pl = pl + registry.iter().map(|a| {
                    a.positions.iter().map(|p| p.profit).sum::<f64>() + a.history.iter().map(|h| h.profit).sum::<f64>()
                }).sum::<f64>();
                println!("{:-<80}", "");
                println!("{:<18} {:<12} {:<5} {:>14.2} {:>5} {:>14.2}",
                    "TOTAL", format!("{} accts", 1 + registry.len()), "", total_equity,
                    positions.len() + registry.iter().map(|a| a.positions.len()).sum::<usize>(), total_pl);
            }
            Err(e) => { eprintln!("Alpaca connection failed: {e}"); }
        }
        return Ok(());
    }

    // Interactive TUI mode
    let watchlist = args.watch.map(|w| w.split(',').map(|s| s.trim().to_uppercase()).collect())
        .unwrap_or_else(|| vec!["AAPL".into(), "MSFT".into(), "TSLA".into()]);
    let symbol = args.symbol.unwrap_or_else(|| "SMCI".to_string());

    let mut app = App::new(broker, symbol, watchlist);

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    loop {
        // Refresh data periodically
        app.refresh().await;

        // Draw
        terminal.draw(|f| draw(f, &app))?;

        // Handle input (non-blocking with timeout)
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if app.command_mode {
                    match key.code {
                        KeyCode::Enter => {
                            let cmd = app.command_input.clone();
                            app.command_input.clear();
                            app.command_mode = false;
                            if cmd.to_lowercase() == "q" || cmd.to_lowercase() == "quit" { break; }
                            app.handle_command(&cmd).await;
                        }
                        KeyCode::Esc => {
                            app.command_input.clear();
                            app.command_mode = false;
                        }
                        KeyCode::Backspace => { app.command_input.pop(); }
                        KeyCode::Char(c) => { app.command_input.push(c); }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char(':') => { app.command_mode = true; }
                        KeyCode::Tab => { app.active_tab = (app.active_tab + 1) % app.tabs.len(); }
                        KeyCode::Char('1') => app.active_tab = 0,
                        KeyCode::Char('2') => app.active_tab = 1,
                        KeyCode::Char('3') => app.active_tab = 2,
                        KeyCode::Char('4') => app.active_tab = 3,
                        KeyCode::Char('5') => app.active_tab = 4,
                        KeyCode::Char('6') => app.active_tab = 5,
                        KeyCode::Char('7') => app.active_tab = 6,
                        KeyCode::Char('r') => {
                            app.last_refresh = Instant::now() - Duration::from_secs(60);
                            app.log("Refreshing...", Color::Yellow);
                        }
                        // Arrow keys for list navigation (Positions=2, Orders=3)
                        KeyCode::Up | KeyCode::Char('k') => {
                            if app.active_tab == 2 && app.selected_position > 0 { app.selected_position -= 1; }
                            if app.active_tab == 3 && app.selected_order > 0 { app.selected_order -= 1; }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if app.active_tab == 2 && app.selected_position + 1 < app.positions.len() { app.selected_position += 1; }
                            if app.active_tab == 3 && app.selected_order + 1 < app.orders.len() { app.selected_order += 1; }
                        }
                        // Enter: action on selected item
                        KeyCode::Enter => {
                            if app.active_tab == 2 && !app.positions.is_empty() {
                                let pos = &app.positions[app.selected_position];
                                let sym = pos.symbol.clone();
                                // Switch chart to this symbol
                                app.chart_symbol = sym.clone();
                                app.last_refresh = Instant::now() - Duration::from_secs(60);
                                app.log(&format!("Selected {sym} — press 'x' to close, 'p' for partial close"), Color::Cyan);
                            }
                            if app.active_tab == 3 && !app.orders.is_empty() {
                                let order = &app.orders[app.selected_order];
                                app.log(&format!("Order {} {} {} — press 'd' to cancel", order.symbol, order.side, order.order_type), Color::Cyan);
                            }
                        }
                        // x: close selected position entirely
                        KeyCode::Char('x') => {
                            if app.active_tab == 2 && !app.positions.is_empty() {
                                let pos = &app.positions[app.selected_position];
                                let sym = pos.symbol.clone();
                                let qty = pos.qty.abs();
                                app.log(&format!("Closing {sym} ({qty} shares)..."), Color::Yellow);
                                match app.broker.close_position(&sym, None).await {
                                    Ok(r) => {
                                        app.log(&format!("CLOSED {sym}: {}", r.status), Color::Green);
                                        app.last_refresh = Instant::now() - Duration::from_secs(60);
                                    }
                                    Err(e) => app.log(&format!("Close failed: {e}"), Color::Red),
                                }
                            }
                        }
                        // p: partial close (50%)
                        KeyCode::Char('p') => {
                            if app.active_tab == 2 && !app.positions.is_empty() {
                                let pos = &app.positions[app.selected_position];
                                let sym = pos.symbol.clone();
                                let half = (pos.qty.abs() / 2.0).floor().max(1.0);
                                app.log(&format!("Partial close {sym} ({half} of {:.0})...", pos.qty.abs()), Color::Yellow);
                                match app.broker.close_position(&sym, Some(half)).await {
                                    Ok(r) => {
                                        app.log(&format!("PARTIAL CLOSE {sym} {half}: {}", r.status), Color::Green);
                                        app.last_refresh = Instant::now() - Duration::from_secs(60);
                                    }
                                    Err(e) => app.log(&format!("Partial close failed: {e}"), Color::Red),
                                }
                            }
                        }
                        // d: cancel selected order
                        KeyCode::Char('d') => {
                            if app.active_tab == 3 && !app.orders.is_empty() {
                                let order = &app.orders[app.selected_order];
                                let id = order.id.clone();
                                let sym = order.symbol.clone();
                                app.log(&format!("Cancelling order {sym} ({id})..."), Color::Yellow);
                                match app.broker.cancel_order(&id).await {
                                    Ok(_) => {
                                        app.log(&format!("CANCELLED {sym} order"), Color::Green);
                                        app.last_refresh = Instant::now() - Duration::from_secs(60);
                                    }
                                    Err(e) => app.log(&format!("Cancel failed: {e}"), Color::Red),
                                }
                            }
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
