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
use std::time::{Duration, Instant};

mod broker;
mod creds;

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
        Self {
            broker,
            active_tab: 0,
            tabs: vec!["Dashboard", "Chart", "Positions", "Orders", "Watchlist", "Command"],
            account: None,
            positions: vec![],
            orders: vec![],
            watchlist,
            watchlist_prices: vec![],
            chart_symbol: symbol,
            chart_bars: vec![],
            chart_timeframe: "1Day".to_string(),
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
            "help" | "h" | "?" => {
                self.log("Commands: buy/sell SYMBOL QTY, close SYMBOL, chart SYMBOL [TF],", Color::Cyan);
                self.log("  watch SYMBOL, tf TIMEFRAME, help, quit", Color::Cyan);
                self.log("Tabs: 1-6 or Tab to cycle. : to enter command mode.", Color::Cyan);
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
    let rows: Vec<Row> = app.orders.iter().map(|o| {
        let side_color = if o.side == "buy" { Color::Green } else { Color::Red };
        Row::new(vec![
            Cell::from(o.symbol.clone()).style(Style::default().fg(Color::White)),
            Cell::from(o.side.clone()).style(Style::default().fg(side_color)),
            Cell::from(o.order_type.clone()).style(Style::default().fg(Color::Cyan)),
            Cell::from(o.qty.clone()).style(Style::default().fg(Color::Yellow)),
            Cell::from(o.status.clone()).style(Style::default().fg(Color::DarkGray)),
        ])
    }).collect();

    let table = Table::new(
        rows,
        [Constraint::Length(10), Constraint::Length(6), Constraint::Length(10), Constraint::Length(10), Constraint::Length(12)],
    )
    .header(Row::new(vec!["Symbol", "Side", "Type", "Qty", "Status"]).style(Style::default().fg(Color::DarkGray)))
    .block(Block::default().borders(Borders::ALL).title(format!(" Open Orders ({}) ", app.orders.len())));
    f.render_widget(table, area);
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
            // Full positions view
            let rows: Vec<Row> = app.positions.iter().map(|p| {
                let pl_color = if p.unrealized_pl >= 0.0 { Color::Green } else { Color::Red };
                let price = if p.qty.abs() > 0.0 { p.market_value.abs() / p.qty.abs() } else { p.avg_entry_price };
                Row::new(vec![
                    Cell::from(p.symbol.clone()).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Cell::from(p.side.clone()).style(Style::default().fg(if p.side == "long" { Color::Green } else { Color::Red })),
                    Cell::from(format!("{:.0}", p.qty.abs())),
                    Cell::from(format!("${:.2}", p.avg_entry_price)).style(Style::default().fg(Color::Cyan)),
                    Cell::from(format!("${:.2}", price)).style(Style::default().fg(Color::White)),
                    Cell::from(format!("${:.2}", p.market_value.abs())),
                    Cell::from(format!("{:+.2}", p.unrealized_pl)).style(Style::default().fg(pl_color)),
                ])
            }).collect();
            let table = Table::new(
                rows,
                [Constraint::Length(10), Constraint::Length(6), Constraint::Length(8), Constraint::Length(12), Constraint::Length(12), Constraint::Length(14), Constraint::Length(12)],
            )
            .header(Row::new(vec!["Symbol", "Side", "Qty", "Entry", "Current", "Mkt Value", "P&L"]).style(Style::default().fg(Color::DarkGray)))
            .block(Block::default().borders(Borders::ALL).title(" Positions "));
            f.render_widget(table, main_layout[1]);
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
        5 => draw_log(f, app, main_layout[1]),
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
                        KeyCode::Char('r') => {
                            app.last_refresh = Instant::now() - Duration::from_secs(60);
                            app.log("Refreshing...", Color::Yellow);
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
