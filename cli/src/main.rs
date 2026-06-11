//! TyphooN Terminal CLI — lean TUI/headless interface for trading, cache access, and ops.
//!
//! This is not a full GUI replacement. Interactive trading is currently Alpaca REST-first;
//! the GUI remains the complete surface for Kraken/tastytrade, full charting/indicators,
//! multi-source sync, strategy tools, AI windows, and storage management.
//!
//! Usage:
//!   typhoon                     # Interactive TUI mode
//!   typhoon --watch AAPL,MSFT   # Watchlist mode
//!   typhoon --positions         # Show positions and exit
//!   typhoon --account           # Show account info and exit
//!   typhoon --cache-stats       # Show shared SQLite cache coverage and exit
//!   typhoon --sync-status       # Show GUI-managed market-data sync snapshot and exit

use clap::Parser;
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Tabs, Wrap},
};
use serde_json::Value;
use std::collections::VecDeque;
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use typhoon_engine::core::cache::SqliteCache;

mod broker;
mod creds;
mod mcp;

fn dirs_home() -> PathBuf {
    let mut p = if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
    } else {
        PathBuf::from("/tmp")
    };
    p.push(".config");
    p.push("typhoon-terminal");
    p
}

fn cache_location_file() -> PathBuf {
    dirs_home().join("cache_location.txt")
}

fn read_custom_cache_dir() -> Option<PathBuf> {
    let s = std::fs::read_to_string(cache_location_file()).ok()?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    let path = PathBuf::from(trimmed);
    if path.is_absolute() { Some(path) } else { None }
}

fn default_cache_dir() -> PathBuf {
    dirs_home().join("cache")
}

fn resolve_cache_dir(explicit: Option<&PathBuf>) -> std::io::Result<PathBuf> {
    let dir = if let Some(path) = explicit {
        path.clone()
    } else if let Some(custom) = read_custom_cache_dir().filter(|p| p.is_dir()) {
        custom
    } else {
        default_cache_dir()
    };
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn format_bytes(bytes: i64) -> String {
    const KIB: f64 = 1024.0;
    let bytes_f = bytes as f64;
    if bytes_f >= KIB * KIB * KIB {
        format!("{:.2} GiB", bytes_f / KIB / KIB / KIB)
    } else if bytes_f >= KIB * KIB {
        format!("{:.2} MiB", bytes_f / KIB / KIB)
    } else if bytes_f >= KIB {
        format!("{:.2} KiB", bytes_f / KIB)
    } else {
        format!("{bytes} B")
    }
}

fn open_cli_cache(
    cache_dir: Option<&PathBuf>,
) -> Result<(SqliteCache, PathBuf), Box<dyn std::error::Error>> {
    let cache_dir = resolve_cache_dir(cache_dir)?;
    let db_path = cache_dir.join("typhoon_cache.db");
    let cache = if db_path.exists() {
        SqliteCache::open_readonly(&db_path).or_else(|_| SqliteCache::open(&db_path))?
    } else {
        SqliteCache::open(&db_path)?
    };
    Ok((cache, db_path))
}

fn format_cache_timestamp(ts: i64) -> String {
    let seconds = if ts > 10_000_000_000 { ts / 1000 } else { ts };
    chrono::DateTime::from_timestamp(seconds, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn cache_stats_lines(cache: &SqliteCache, db_path: &Path) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!("Cache: {}", db_path.display()));
    match cache.stats() {
        Ok((bar_entries, kv_entries, bytes)) => lines.push(format!(
            "Rows: {bar_entries} bar entries, {kv_entries} KV entries, {} on disk",
            format_bytes(bytes)
        )),
        Err(e) => lines.push(format!("Stats error: {e}")),
    }

    match cache.detailed_stats_with_size() {
        Ok(rows) => {
            let mut by_source: std::collections::BTreeMap<String, (i64, i64, i64)> =
                std::collections::BTreeMap::new();
            let mut by_timeframe: std::collections::BTreeMap<String, (i64, i64)> =
                std::collections::BTreeMap::new();
            let mut top = rows.clone();
            for (key, bars, _ts, bytes) in &rows {
                let parts: Vec<&str> = key.split(':').collect();
                let source = if parts.len() >= 3 { parts[0] } else { "cache" };
                let timeframe = parts.last().copied().unwrap_or("unknown");
                let src = by_source.entry(source.to_string()).or_default();
                src.0 += 1;
                src.1 += *bars;
                src.2 += *bytes;
                let tf = by_timeframe.entry(timeframe.to_string()).or_default();
                tf.0 += 1;
                tf.1 += *bars;
            }
            lines.push("By source:".to_string());
            for (source, (entries, bars, bytes)) in by_source {
                lines.push(format!(
                    "  {source:<14} {entries:>6} entries {bars:>12} bars {:>10}",
                    format_bytes(bytes)
                ));
            }
            lines.push("By timeframe:".to_string());
            for (tf, (entries, bars)) in by_timeframe {
                lines.push(format!("  {tf:<10} {entries:>6} entries {bars:>12} bars"));
            }
            top.sort_by(|a, b| b.1.cmp(&a.1));
            lines.push("Deepest entries:".to_string());
            for (key, bars, ts, bytes) in top.into_iter().take(10) {
                lines.push(format!(
                    "  {key:<38} {bars:>8} bars {:>10} updated {}",
                    format_bytes(bytes),
                    format_cache_timestamp(ts)
                ));
            }
        }
        Err(e) => lines.push(format!("Detailed stats error: {e}")),
    }
    lines
}

fn kv_json(cache: &SqliteCache, key: &str) -> Option<Value> {
    cache
        .get_kv(key)
        .ok()
        .flatten()
        .and_then(|json| serde_json::from_str::<Value>(&json).ok())
}

fn kv_array_len(cache: &SqliteCache, key: &str) -> usize {
    kv_json(cache, key)
        .and_then(|v| v.as_array().map(|a| a.len()))
        .unwrap_or(0)
}

fn broker_status_lines(cache: &SqliteCache) -> Vec<String> {
    let mut lines = vec!["Broker snapshot from GUI/shared KV cache:".to_string()];
    for (label, account_key, positions_key, orders_key) in [
        (
            "Alpaca",
            "broker:account",
            "broker:positions",
            "broker:orders",
        ),
        (
            "Kraken",
            "broker:kr_account",
            "broker:kr_positions",
            "broker:kr_orders",
        ),
        (
            "tastytrade",
            "broker:tt_account",
            "broker:tt_positions",
            "broker:tt_orders",
        ),
    ] {
        let account = kv_json(cache, account_key);
        let acct_text = account
            .as_ref()
            .and_then(|v| {
                let equity = v.get("equity").and_then(Value::as_f64);
                let cash = v.get("cash").and_then(Value::as_f64);
                match (equity, cash) {
                    (Some(e), Some(c)) => Some(format!("equity ${e:.2}, cash ${c:.2}")),
                    (Some(e), None) => Some(format!("equity ${e:.2}")),
                    _ => None,
                }
            })
            .unwrap_or_else(|| {
                if account.is_some() {
                    "account cached".to_string()
                } else {
                    "no account snapshot".to_string()
                }
            });
        lines.push(format!(
            "  {label:<11} {acct_text}; {} positions; {} open/order rows",
            kv_array_len(cache, positions_key),
            kv_array_len(cache, orders_key)
        ));
    }
    let watchlist = kv_array_len(cache, "broker:watchlist");
    if watchlist > 0 {
        lines.push(format!("  Watchlist   {watchlist} cached rows"));
    }
    lines.push(
        "Read-only snapshot only; live trading remains through the GUI/broker connection."
            .to_string(),
    );
    lines
}

fn sync_status_lines(cache: &SqliteCache) -> Vec<String> {
    let mut lines = vec!["Market-data sync snapshot from GUI/shared KV cache:".to_string()];
    for (label, key) in [
        ("Alpaca", "alpaca:backfill_complete_pairs"),
        ("Kraken Spot", "kraken:backfill_complete_pairs"),
        ("Kraken Futures", "kraken-futures:backfill_complete_pairs"),
        ("tastytrade", "tastytrade:backfill_complete_pairs"),
    ] {
        lines.push(format!(
            "  {label:<15} {} full-history exhaustion/completion markers",
            kv_array_len(cache, key)
        ));
    }
    lines.push(format!(
        "  Unresolvable   {} broker/symbol/timeframe tombstones",
        kv_array_len(cache, "broker:unresolvable_pairs")
    ));
    match cache.detailed_stats() {
        Ok(rows) => {
            let mut by_source: std::collections::BTreeMap<String, (usize, i64, i64)> =
                std::collections::BTreeMap::new();
            for (key, bars, ts) in rows {
                let parts: Vec<&str> = key.split(':').collect();
                let source = if parts.len() >= 3 { parts[0] } else { "cache" };
                let entry = by_source.entry(source.to_string()).or_insert((0, 0, 0));
                entry.0 += 1;
                entry.1 += bars;
                entry.2 = entry.2.max(ts);
            }
            lines.push("Cached bar coverage by source:".to_string());
            for (source, (entries, bars, newest)) in by_source {
                lines.push(format!(
                    "  {source:<15} {entries:>6} entries {bars:>12} bars newest {}",
                    format_cache_timestamp(newest)
                ));
            }
        }
        Err(e) => lines.push(format!("Cached bar coverage unavailable: {e}")),
    }
    lines
}

fn resolve_timeframe(tf: &str) -> (String, usize) {
    match tf.to_uppercase().as_str() {
        // MT5-style shortcodes
        "M1" | "1M" => ("1Min".into(), 1),
        "M5" | "5M" => ("5Min".into(), 1),
        "M10" | "10M" => ("5Min".into(), 2),
        "M15" | "15M" => ("15Min".into(), 1),
        "M20" | "20M" => ("5Min".into(), 4),
        "M30" | "30M" => ("30Min".into(), 1),
        "H1" | "1H" => ("1Hour".into(), 1),
        "H2" | "2H" => ("1Hour".into(), 2),
        "H3" | "3H" => ("1Hour".into(), 3),
        "H4" | "4H" => ("4Hour".into(), 1),
        "H6" | "6H" => ("1Hour".into(), 6),
        "H8" | "8H" => ("4Hour".into(), 2),
        "H12" | "12H" => ("4Hour".into(), 3),
        "D1" | "1D" | "DAILY" => ("1Day".into(), 1),
        "D2" | "2D" => ("1Day".into(), 2),
        "D3" | "3D" => ("1Day".into(), 3),
        "W1" | "1W" | "WEEKLY" => ("1Week".into(), 1),
        "W2" | "2W" => ("1Week".into(), 2),
        "MN1" | "MN" | "1MN" | "MONTHLY" => ("1Month".into(), 1),
        // Already Alpaca format — pass through
        "1MIN" | "5MIN" | "15MIN" | "30MIN" => (tf.into(), 1),
        "1HOUR" | "4HOUR" => (tf.into(), 1),
        "1DAY" | "1WEEK" | "1MONTH" => (tf.into(), 1),
        // Custom hour TFs
        s if s.ends_with("HOUR") => {
            if let Ok(n) = s.trim_end_matches("HOUR").parse::<usize>() {
                if n <= 4 {
                    ("1Hour".into(), n)
                } else {
                    ("4Hour".into(), n / 4)
                }
            } else {
                (tf.into(), 1)
            }
        }
        // Default: pass through as-is
        _ => (tf.into(), 1),
    }
}

/// Aggregate bars by factor (combine N bars into 1).
fn aggregate_bars(bars: &[broker::Bar], factor: usize) -> Vec<broker::Bar> {
    if factor <= 1 {
        return bars.to_vec();
    }
    let mut result = Vec::new();
    for chunk in bars.chunks(factor) {
        // The is_empty guard above already handles this, but chunk.last() still
        // returns Option — use it properly rather than .expect().
        let Some(last) = chunk.last() else {
            continue;
        };
        let Some(first) = chunk.first() else {
            continue;
        };
        result.push(broker::Bar {
            timestamp: first.timestamp.clone(),
            open: first.open,
            high: chunk.iter().map(|b| b.high).fold(f64::MIN, f64::max),
            low: chunk.iter().map(|b| b.low).fold(f64::MAX, f64::min),
            close: last.close,
            volume: chunk.iter().map(|b| b.volume).sum(),
        });
    }
    result
}

#[derive(Parser)]
#[command(
    name = "typhoon",
    about = "TyphooN Terminal CLI — trading terminal for your terminal"
)]
struct Args {
    /// API key (or set ALPACA_API_KEY env var)
    #[arg(long, env = "ALPACA_API_KEY", hide_env_values = true)]
    api_key: Option<String>,

    /// Secret key (or set ALPACA_SECRET_KEY env var)
    #[arg(long, env = "ALPACA_SECRET_KEY", hide_env_values = true)]
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

    /// Show account info and exit
    #[arg(long)]
    accounts: bool,

    /// Symbol to load on startup
    #[arg(long, short = 's')]
    symbol: Option<String>,

    /// Directory containing typhoon_cache.db. Defaults to the GUI cache setting, then ~/.config/typhoon-terminal/cache.
    #[arg(long, env = "TYPHOON_CACHE_DIR", value_name = "PATH")]
    cache_dir: Option<PathBuf>,

    /// Export the SQLite cache to a .typhoon-backup file and exit.
    #[arg(long, value_name = "PATH", conflicts_with = "import_cache")]
    export_cache: Option<PathBuf>,

    /// Import a .typhoon-backup file into the SQLite cache and exit.
    #[arg(long, value_name = "PATH", conflicts_with = "export_cache")]
    import_cache: Option<PathBuf>,

    /// Print read-only SQLite cache statistics and exit.
    #[arg(long)]
    cache_stats: bool,

    /// Print read-only market-data sync status from the shared GUI cache and exit.
    #[arg(long)]
    sync_status: bool,

    /// Print read-only broker account/position/order snapshots from the shared GUI cache and exit.
    #[arg(long)]
    broker_status: bool,

    /// Password for encrypted cache backup export/import.
    #[arg(
        long,
        env = "TYPHOON_CACHE_BACKUP_PASSPHRASE",
        value_name = "PASSPHRASE",
        hide_env_values = true
    )]
    cache_backup_passphrase: Option<String>,

    /// Run the TyphooN Terminal MCP stdio server.
    #[arg(long)]
    mcp_server: bool,
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
    watchlist_quotes: Vec<(String, f64, f64, f64)>, // (symbol, bid, ask, last)
    // Chart
    chart_symbol: String,
    chart_bars: Vec<broker::Bar>,
    chart_timeframe: String,
    // Selection state (for interactive list navigation)
    selected_position: usize,
    selected_order: usize,
    // Action confirmation
    _pending_action: Option<String>, // reserved for confirmation dialogs
    // Status line
    market_open: bool,
    market_next_event: String,
    // Command
    command_input: String,
    command_mode: bool,
    // Log
    log_messages: VecDeque<(String, Color)>,
    // Refresh
    last_refresh: Instant,
    refresh_interval: Duration,
    // Previous close prices for watchlist change tracking
    watchlist_prev_close: std::collections::HashMap<String, f64>,
    // Shared GUI cache location for read-only ops commands
    cache_dir: Option<PathBuf>,
}

impl App {
    fn new(
        broker: broker::AlpacaBroker,
        symbol: String,
        watchlist: Vec<String>,
        cache_dir: Option<PathBuf>,
    ) -> Self {
        Self {
            broker,
            active_tab: 0,
            tabs: vec![
                "Dashboard",
                "Chart",
                "Positions",
                "Orders",
                "Watchlist",
                "Accounts",
                "Command",
            ],
            account: None,
            positions: vec![],
            orders: vec![],
            watchlist,
            watchlist_quotes: vec![],
            chart_symbol: symbol,
            chart_bars: vec![],
            chart_timeframe: "1Day".to_string(),
            selected_position: 0,
            selected_order: 0,
            _pending_action: None,
            market_open: false,
            market_next_event: String::new(),
            command_input: String::new(),
            command_mode: false,
            log_messages: VecDeque::from([
                ("TyphooN Terminal CLI v0.2.0".to_string(), Color::Cyan),
                (
                    "Press Tab to switch views, : for command mode, q to quit".to_string(),
                    Color::DarkGray,
                ),
            ]),
            last_refresh: Instant::now() - Duration::from_secs(60), // force initial refresh
            refresh_interval: Duration::from_secs(5),
            watchlist_prev_close: std::collections::HashMap::new(),
            cache_dir,
        }
    }

    fn log(&mut self, msg: &str, color: Color) {
        let ts = chrono::Local::now().format("%H:%M:%S").to_string();
        self.log_messages
            .push_back((format!("[{ts}] {msg}"), color));
        if self.log_messages.len() > 100 {
            self.log_messages.pop_front();
        }
    }

    fn log_lines(&mut self, lines: Vec<String>, color: Color) {
        for line in lines {
            self.log(&line, color);
        }
    }

    fn with_cache_lines<F>(&self, f: F) -> Result<Vec<String>, String>
    where
        F: FnOnce(&SqliteCache, &Path) -> Vec<String>,
    {
        let (cache, db_path) =
            open_cli_cache(self.cache_dir.as_ref()).map_err(|e| e.to_string())?;
        Ok(f(&cache, &db_path))
    }

    async fn refresh(&mut self) {
        if self.last_refresh.elapsed() < self.refresh_interval {
            return;
        }
        self.last_refresh = Instant::now();

        // Account
        match self.broker.get_account().await {
            Ok(a) => {
                self.account = Some(a);
            }
            Err(e) => {
                self.log(&format!("Account error: {e}"), Color::Red);
            }
        }

        // Positions
        match self.broker.get_positions().await {
            Ok(p) => {
                // Auto-populate watchlist from positions on first load
                if self.watchlist.is_empty() {
                    for pos in &p {
                        if !self.watchlist.contains(&pos.symbol) {
                            self.watchlist.push(pos.symbol.clone());
                        }
                    }
                }
                self.positions = p;
            }
            Err(e) => {
                self.log(&format!("Positions error: {e}"), Color::Red);
            }
        }

        // Orders
        match self.broker.get_orders("open", 50).await {
            Ok(o) => {
                self.orders = o;
            }
            Err(e) => {
                self.log(&format!("Orders error: {e}"), Color::Red);
            }
        }

        // Market clock
        match self.broker.get_clock().await {
            Ok((is_open, next_event)) => {
                self.market_open = is_open;
                self.market_next_event = next_event;
            }
            Err(_) => {} // non-critical, ignore
        }

        // Watchlist quotes
        let mut new_quotes = Vec::new();
        for sym in &self.watchlist.clone() {
            match self.broker.get_quote(sym).await {
                Ok((bid, ask, last)) => {
                    new_quotes.push((sym.clone(), bid, ask, last));
                }
                Err(_) => {
                    new_quotes.push((sym.clone(), 0.0, 0.0, 0.0));
                }
            }
        }
        self.watchlist_quotes = new_quotes;

        // Chart bars (resolve custom TFs → Alpaca format + aggregation)
        if !self.chart_symbol.is_empty() {
            let (api_tf, agg_factor) = resolve_timeframe(&self.chart_timeframe);
            let fetch_limit = if agg_factor > 1 {
                100 * agg_factor as u32
            } else {
                100
            };
            match self
                .broker
                .get_bars(&self.chart_symbol, &api_tf, fetch_limit)
                .await
            {
                Ok(bars) => {
                    self.chart_bars = if agg_factor > 1 {
                        let agg = aggregate_bars(&bars, agg_factor);
                        self.log(
                            &format!(
                                "Loaded {} bars for {} @ {} ({}x from {})",
                                agg.len(),
                                self.chart_symbol,
                                self.chart_timeframe,
                                agg_factor,
                                api_tf
                            ),
                            Color::Green,
                        );
                        agg
                    } else {
                        self.log(
                            &format!(
                                "Loaded {} bars for {} @ {}",
                                bars.len(),
                                self.chart_symbol,
                                self.chart_timeframe
                            ),
                            Color::Green,
                        );
                        bars
                    };
                }
                Err(e) => {
                    self.log(&format!("Bars error: {e}"), Color::Red);
                }
            }
        }
    }

    async fn handle_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.trim().split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        match parts[0].to_lowercase().as_str() {
            "buy" | "b" => {
                if parts.len() < 3 {
                    self.log("Usage: buy SYMBOL QTY", Color::Yellow);
                } else {
                    let symbol = parts[1].to_uppercase();
                    let qty: f64 = parts[2].parse().unwrap_or(0.0);
                    if qty <= 0.0 {
                        self.log("Invalid qty", Color::Red);
                        return;
                    }
                    match self.broker.market_order(&symbol, qty, "buy").await {
                        Ok(r) => {
                            self.log(&format!("BUY {qty} {symbol}: {}", r.status), Color::Green)
                        }
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
                    if qty <= 0.0 {
                        self.log("Invalid qty", Color::Red);
                        return;
                    }
                    match self.broker.market_order(&symbol, qty, "sell").await {
                        Ok(r) => {
                            self.log(&format!("SELL {qty} {symbol}: {}", r.status), Color::Green)
                        }
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
                    if parts.len() >= 3 {
                        self.chart_timeframe = parts[2].to_string();
                    }
                    self.last_refresh = Instant::now() - Duration::from_secs(60); // force refresh
                    self.active_tab = 1; // switch to chart tab
                    self.log(
                        &format!("Chart: {} @ {}", self.chart_symbol, self.chart_timeframe),
                        Color::Cyan,
                    );
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
            "accounts" | "acct" => {
                self.active_tab = 5; // switch to Accounts tab
                self.log("Switched to Accounts view", Color::Cyan);
            }
            "limit" => {
                // limit buy/sell SYMBOL QTY PRICE
                if parts.len() < 5 {
                    self.log("Usage: limit buy|sell SYMBOL QTY PRICE", Color::Yellow);
                } else {
                    let side = parts[1].to_lowercase();
                    if side != "buy" && side != "sell" {
                        self.log("Side must be 'buy' or 'sell'", Color::Red);
                        return;
                    }
                    let symbol = parts[2].to_uppercase();
                    let qty: f64 = parts[3].parse().unwrap_or(0.0);
                    let price: f64 = parts[4].parse().unwrap_or(0.0);
                    if qty <= 0.0 || price <= 0.0 {
                        self.log("Invalid qty or price", Color::Red);
                        return;
                    }
                    match self.broker.limit_order(&symbol, qty, &side, price).await {
                        Ok(r) => self.log(
                            &format!(
                                "LIMIT {} {qty} {symbol} @ ${price:.2}: {}",
                                side.to_uppercase(),
                                r.status
                            ),
                            Color::Green,
                        ),
                        Err(e) => self.log(&format!("Limit order failed: {e}"), Color::Red),
                    }
                }
            }
            "stop" => {
                // stop buy/sell SYMBOL QTY STOP_PRICE
                if parts.len() < 5 {
                    self.log("Usage: stop buy|sell SYMBOL QTY STOP_PRICE", Color::Yellow);
                } else {
                    let side = parts[1].to_lowercase();
                    if side != "buy" && side != "sell" {
                        self.log("Side must be 'buy' or 'sell'", Color::Red);
                        return;
                    }
                    let symbol = parts[2].to_uppercase();
                    let qty: f64 = parts[3].parse().unwrap_or(0.0);
                    let stop_price: f64 = parts[4].parse().unwrap_or(0.0);
                    if qty <= 0.0 || stop_price <= 0.0 {
                        self.log("Invalid qty or stop price", Color::Red);
                        return;
                    }
                    match self
                        .broker
                        .stop_order(&symbol, qty, &side, stop_price)
                        .await
                    {
                        Ok(r) => self.log(
                            &format!(
                                "STOP {} {qty} {symbol} @ ${stop_price:.2}: {}",
                                side.to_uppercase(),
                                r.status
                            ),
                            Color::Green,
                        ),
                        Err(e) => self.log(&format!("Stop order failed: {e}"), Color::Red),
                    }
                }
            }
            "bracket" | "brk" => {
                // bracket buy/sell SYMBOL QTY SL TP
                if parts.len() < 6 {
                    self.log("Usage: bracket buy|sell SYMBOL QTY SL TP", Color::Yellow);
                } else {
                    let side = parts[1].to_lowercase();
                    if side != "buy" && side != "sell" {
                        self.log("Side must be 'buy' or 'sell'", Color::Red);
                        return;
                    }
                    let symbol = parts[2].to_uppercase();
                    let qty: f64 = parts[3].parse().unwrap_or(0.0);
                    let sl: f64 = parts[4].parse().unwrap_or(0.0);
                    let tp: f64 = parts[5].parse().unwrap_or(0.0);
                    if qty <= 0.0 || sl <= 0.0 || tp <= 0.0 {
                        self.log("Invalid qty, SL, or TP", Color::Red);
                        return;
                    }
                    match self.broker.bracket_order(&symbol, qty, &side, sl, tp).await {
                        Ok(r) => self.log(
                            &format!(
                                "BRACKET {} {qty} {symbol} SL=${sl:.2} TP=${tp:.2}: {}",
                                side.to_uppercase(),
                                r.status
                            ),
                            Color::Green,
                        ),
                        Err(e) => self.log(&format!("Bracket order failed: {e}"), Color::Red),
                    }
                }
            }
            "trailing" | "trail" => {
                // trailing buy/sell SYMBOL QTY TRAIL_PCT
                if parts.len() < 5 {
                    self.log(
                        "Usage: trailing buy|sell SYMBOL QTY TRAIL_PERCENT",
                        Color::Yellow,
                    );
                } else {
                    let side = parts[1].to_lowercase();
                    if side != "buy" && side != "sell" {
                        self.log("Side must be 'buy' or 'sell'", Color::Red);
                        return;
                    }
                    let symbol = parts[2].to_uppercase();
                    let qty: f64 = parts[3].parse().unwrap_or(0.0);
                    let trail_pct: f64 = parts[4].parse().unwrap_or(0.0);
                    if qty <= 0.0 || trail_pct <= 0.0 || trail_pct > 50.0 {
                        self.log("Invalid qty or trail % (must be 0 < pct <= 50)", Color::Red);
                        return;
                    }
                    match self
                        .broker
                        .trailing_stop_order(&symbol, qty, &side, trail_pct)
                        .await
                    {
                        Ok(r) => self.log(
                            &format!(
                                "TRAILING {} {qty} {symbol} trail={trail_pct}%: {}",
                                side.to_uppercase(),
                                r.status
                            ),
                            Color::Green,
                        ),
                        Err(e) => self.log(&format!("Trailing stop failed: {e}"), Color::Red),
                    }
                }
            }
            "oco" => {
                // oco sell SYMBOL QTY TP SL
                if parts.len() < 6 {
                    self.log(
                        "Usage: oco sell|buy SYMBOL QTY TP_PRICE SL_PRICE",
                        Color::Yellow,
                    );
                } else {
                    let side = parts[1].to_lowercase();
                    if side != "buy" && side != "sell" {
                        self.log("Side must be 'buy' or 'sell'", Color::Red);
                        return;
                    }
                    let symbol = parts[2].to_uppercase();
                    let qty: f64 = parts[3].parse().unwrap_or(0.0);
                    let tp: f64 = parts[4].parse().unwrap_or(0.0);
                    let sl: f64 = parts[5].parse().unwrap_or(0.0);
                    if qty <= 0.0 || tp <= 0.0 || sl <= 0.0 {
                        self.log("Invalid qty, TP, or SL price", Color::Red);
                        return;
                    }
                    match self
                        .broker
                        .oco_order(&symbol, qty, &side, tp, sl, None)
                        .await
                    {
                        Ok(r) => self.log(
                            &format!(
                                "OCO {} {qty} {symbol} TP={tp} SL={sl}: {}",
                                side.to_uppercase(),
                                r.status
                            ),
                            Color::Green,
                        ),
                        Err(e) => self.log(&format!("OCO failed: {e}"), Color::Red),
                    }
                }
            }
            "cancel" => {
                // cancel ORDER_ID
                if parts.len() < 2 {
                    self.log("Usage: cancel ORDER_ID", Color::Yellow);
                } else {
                    let order_id = parts[1].to_string();
                    match self.broker.cancel_order(&order_id).await {
                        Ok(()) => self.log(&format!("Order {order_id} cancelled"), Color::Green),
                        Err(e) => self.log(&format!("Cancel failed: {e}"), Color::Red),
                    }
                }
            }
            "closeall" => {
                self.log("Closing all positions...", Color::Yellow);
                match self.broker.close_all().await {
                    Ok(()) => {
                        self.log("All positions closed", Color::Green);
                        self.last_refresh = Instant::now() - Duration::from_secs(60);
                    }
                    Err(e) => self.log(&format!("Close all failed: {e}"), Color::Red),
                }
            }
            "cancelall" => {
                self.log("Cancelling all open orders...", Color::Yellow);
                match self.broker.cancel_all().await {
                    Ok(()) => {
                        self.log("All orders cancelled", Color::Green);
                        self.last_refresh = Instant::now() - Duration::from_secs(60);
                    }
                    Err(e) => self.log(&format!("Cancel all failed: {e}"), Color::Red),
                }
            }
            "history" | "hist" => {
                let limit = parts
                    .get(1)
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(20);
                match self.broker.get_order_history(limit).await {
                    Ok(orders) => {
                        if orders.is_empty() {
                            self.log("No recent filled orders", Color::Yellow);
                        } else {
                            self.log(
                                &format!("--- Order History (last {}) ---", orders.len()),
                                Color::Cyan,
                            );
                            for o in &orders {
                                let price_str = o
                                    .limit_price
                                    .as_deref()
                                    .or(o.stop_price.as_deref())
                                    .unwrap_or("mkt");
                                let ts = if o.created_at.len() >= 16 {
                                    &o.created_at[..16]
                                } else {
                                    &o.created_at
                                };
                                let side_color = if o.side == "buy" {
                                    Color::Green
                                } else {
                                    Color::Red
                                };
                                self.log(
                                    &format!(
                                        "{} {} {} {} @ {} [{}] {}",
                                        ts,
                                        o.side.to_uppercase(),
                                        o.qty,
                                        o.symbol,
                                        price_str,
                                        o.order_type,
                                        o.status
                                    ),
                                    side_color,
                                );
                            }
                        }
                    }
                    Err(e) => self.log(&format!("History failed: {e}"), Color::Red),
                }
            }
            "search" | "find" => {
                if parts.len() < 2 {
                    self.log("Usage: search QUERY", Color::Yellow);
                } else {
                    let query = parts[1..].join(" ");
                    match self.broker.search_symbols(&query).await {
                        Ok(results) => {
                            if results.is_empty() {
                                self.log(
                                    &format!("No symbols matching '{}'", query),
                                    Color::Yellow,
                                );
                            } else {
                                self.log(
                                    &format!(
                                        "--- Symbol Search: {} ({} results) ---",
                                        query,
                                        results.len()
                                    ),
                                    Color::Cyan,
                                );
                                for (sym, name, class) in &results {
                                    let c = if class == "crypto" {
                                        Color::Magenta
                                    } else {
                                        Color::White
                                    };
                                    self.log(&format!("  {:<12} {:<40} ({})", sym, name, class), c);
                                }
                            }
                        }
                        Err(e) => self.log(&format!("Search failed: {e}"), Color::Red),
                    }
                }
            }
            "movers" | "top" => match self.broker.get_top_movers().await {
                Ok(movers) => {
                    if movers.is_empty() {
                        self.log("No movers data available", Color::Yellow);
                    } else {
                        self.log("--- Top Movers (Most Active) ---", Color::Cyan);
                        for (sym, price, change) in &movers {
                            let c = if *change >= 0.0 {
                                Color::Green
                            } else {
                                Color::Red
                            };
                            self.log(&format!("  {:<8} ${:.2}  {:+.2}%", sym, price, change), c);
                        }
                    }
                }
                Err(e) => self.log(&format!("Movers failed: {e}"), Color::Red),
            },
            "fills" | "activity" => {
                let limit = parts
                    .get(1)
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(20);
                match self.broker.get_activities(limit).await {
                    Ok(fills) => {
                        if fills.is_empty() {
                            self.log("No recent fills", Color::Yellow);
                        } else {
                            self.log(
                                &format!("--- Recent Fills ({}) ---", fills.len()),
                                Color::Cyan,
                            );
                            for (ts, sym, side, qty, price) in &fills {
                                let c = if side == "buy" {
                                    Color::Green
                                } else {
                                    Color::Red
                                };
                                let ts_short = if ts.len() >= 16 { &ts[..16] } else { ts };
                                self.log(
                                    &format!(
                                        "  {} {} {} {} @ ${}",
                                        ts_short,
                                        side.to_uppercase(),
                                        qty,
                                        sym,
                                        price
                                    ),
                                    c,
                                );
                            }
                        }
                    }
                    Err(e) => self.log(&format!("Fills failed: {e}"), Color::Red),
                }
            }
            "unwatch" | "uw" => {
                if parts.len() >= 2 {
                    let sym = parts[1].to_uppercase();
                    let before = self.watchlist.len();
                    self.watchlist.retain(|s| s != &sym);
                    if self.watchlist.len() < before {
                        self.log(&format!("Removed {sym} from watchlist"), Color::Yellow);
                    } else {
                        self.log(&format!("{sym} not in watchlist"), Color::Red);
                    }
                } else {
                    self.log("Usage: unwatch SYMBOL", Color::Yellow);
                }
            }
            "cache" | "storage" if parts.get(1).is_some_and(|s| *s == "stats") => {
                match self.with_cache_lines(cache_stats_lines) {
                    Ok(lines) => self.log_lines(lines, Color::Cyan),
                    Err(e) => self.log(&format!("Cache stats failed: {e}"), Color::Red),
                }
            }
            "sync" if parts.get(1).is_some_and(|s| *s == "status") => {
                match self.with_cache_lines(|cache, _| sync_status_lines(cache)) {
                    Ok(lines) => self.log_lines(lines, Color::Cyan),
                    Err(e) => self.log(&format!("Sync status failed: {e}"), Color::Red),
                }
            }
            "broker" if parts.get(1).is_some_and(|s| *s == "status") => {
                match self.with_cache_lines(|cache, _| broker_status_lines(cache)) {
                    Ok(lines) => self.log_lines(lines, Color::Cyan),
                    Err(e) => self.log(&format!("Broker status failed: {e}"), Color::Red),
                }
            }
            "cache" | "storage" => {
                self.log(
                    "Cache ops: use 'cache stats' for read-only stats; CLI also supports import/export and MCP research packets; GUI owns full Storage Manager.",
                    Color::Yellow,
                );
            }
            "capabilities" | "caps" | "parity" => {
                self.log("--- CLI/TUI Capabilities ---", Color::Cyan);
                self.log("  Alpaca REST: account, positions, orders, market/limit/stop/bracket/trailing/OCO, fills, clock, quotes, bars", Color::Cyan);
                self.log(
                    "  Charts: lightweight terminal candles + volume + SMA(20), Alpaca-backed",
                    Color::Cyan,
                );
                self.log("  Ops: read-only cache stats, broker/sync snapshots, cache backup import/export, MCP research packet server", Color::Cyan);
                self.log(
                    "--- GUI-only / not reasonable to fully mirror in TUI ---",
                    Color::Yellow,
                );
                self.log("  Kraken live broker surfaces, KrakenPro controls, private/public WebSockets, DOM/bookmap/orderbook", Color::Yellow);
                self.log("  Multi-chart egui workspace, draggable panels, full indicator stack/settings, GPU overlays", Color::Yellow);
                self.log("  Backtester/optimizer/walk-forward UI, AI assistant windows, full Storage/Sync Status managers", Color::Yellow);
            }
            "symbols" | "sym" => {
                let filter = if parts.len() > 1 {
                    Some(parts[1..].join(" ").to_uppercase())
                } else {
                    None
                };
                match self.broker.list_all_symbols().await {
                    Ok(all) => {
                        let filtered: Vec<&(String, String, String)> = if let Some(ref f) = filter {
                            all.iter()
                                .filter(|(s, n, _)| {
                                    s.to_uppercase().contains(f) || n.to_uppercase().contains(f)
                                })
                                .collect()
                        } else {
                            all.iter().collect()
                        };
                        if filtered.is_empty() {
                            self.log("No symbols found", Color::Yellow);
                        } else {
                            self.log(
                                &format!(
                                    "--- Symbols ({} total, showing {}) ---",
                                    all.len(),
                                    filtered.len().min(100)
                                ),
                                Color::Cyan,
                            );
                            // Group by asset class
                            let mut by_class: std::collections::BTreeMap<
                                String,
                                Vec<&(String, String, String)>,
                            > = std::collections::BTreeMap::new();
                            for s in filtered.iter().take(500) {
                                let class = if s.2.is_empty() { "other" } else { &s.2 };
                                by_class.entry(class.to_string()).or_default().push(s);
                            }
                            for (class, syms) in &by_class {
                                let c = if class == "crypto" {
                                    Color::Magenta
                                } else {
                                    Color::Cyan
                                };
                                self.log(&format!("  [{} — {} symbols]", class, syms.len()), c);
                                for (sym, name, _) in syms.iter().take(100) {
                                    let sc = if class == "crypto" {
                                        Color::Magenta
                                    } else {
                                        Color::White
                                    };
                                    self.log(&format!("    {:<12} {}", sym, name), sc);
                                }
                                if syms.len() > 100 {
                                    self.log(
                                        &format!(
                                            "    ... and {} more (use 'symbols FILTER')",
                                            syms.len() - 100
                                        ),
                                        Color::DarkGray,
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => self.log(&format!("Failed to fetch symbols: {e}"), Color::Red),
                }
            }
            "help" | "h" | "?" => {
                self.log("--- Trade Commands ---", Color::Cyan);
                self.log("  buy/sell SYMBOL QTY           Market order", Color::Cyan);
                self.log("  limit buy|sell SYM QTY PRICE  Limit order", Color::Cyan);
                self.log("  stop buy|sell SYM QTY PRICE   Stop order", Color::Cyan);
                self.log(
                    "  bracket buy|sell SYM QTY SL TP  Bracket (market+SL+TP)",
                    Color::Cyan,
                );
                self.log(
                    "  trailing buy|sell SYM QTY PCT Trailing stop order",
                    Color::Cyan,
                );
                self.log(
                    "  oco sell|buy SYM QTY TP SL    OCO exit (one-cancels-other)",
                    Color::Cyan,
                );
                self.log(
                    "  close SYMBOL [QTY]            Close position",
                    Color::Cyan,
                );
                self.log(
                    "  cancel ORDER_ID               Cancel specific order",
                    Color::Cyan,
                );
                self.log(
                    "  closeall                      Close ALL positions",
                    Color::Cyan,
                );
                self.log(
                    "  cancelall                     Cancel ALL open orders",
                    Color::Cyan,
                );
                self.log("--- Research ---", Color::Cyan);
                self.log("  chart SYM [TF]                Load chart", Color::Cyan);
                self.log(
                    "  tf TF                         Change timeframe",
                    Color::Cyan,
                );
                self.log(
                    "  watch SYM / unwatch SYM       Manage watchlist",
                    Color::Cyan,
                );
                self.log(
                    "  search QUERY                  Search symbols (stocks + crypto)",
                    Color::Cyan,
                );
                self.log(
                    "  symbols [FILTER]              Browse all tradeable symbols by class",
                    Color::Cyan,
                );
                self.log(
                    "  movers                        Top market movers",
                    Color::Cyan,
                );
                self.log(
                    "  fills [N] / history [N]       Recent fills / order history",
                    Color::Cyan,
                );
                self.log("--- Accounts / Ops ---", Color::Cyan);
                self.log(
                    "  accounts                      Show account summary",
                    Color::Cyan,
                );
                self.log(
                    "  capabilities                  Show CLI vs GUI coverage",
                    Color::Cyan,
                );
                self.log("  cache stats / sync status / broker status", Color::Cyan);
                self.log(
                    "Tabs: 1-7 or Tab. : for command mode. q to quit.",
                    Color::Cyan,
                );
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
        .constraints([
            Constraint::Length(10), // Account + Risk metrics
            Constraint::Length(5),  // Position summary stats
            Constraint::Min(5),     // Positions table
            Constraint::Length(5),  // Total P&L + VaR
        ])
        .split(area);

    // ── Account info + Risk metrics ──
    let account_text = if let Some(ref a) = app.account {
        let total_pl: f64 = app.positions.iter().map(|p| p.unrealized_pl).sum();
        let pl_color = if total_pl >= 0.0 {
            Color::Green
        } else {
            Color::Red
        };

        // Margin level: equity / initial_margin * 100
        let margin_level = if a.initial_margin > 0.0 {
            a.equity / a.initial_margin * 100.0
        } else {
            0.0
        };
        let margin_color = if margin_level > 200.0 {
            Color::Green
        } else if margin_level > 150.0 {
            Color::Yellow
        } else {
            Color::Red
        };

        // Buying power utilization: (portfolio_value - cash) / buying_power * 100 or equity-based
        let bp_util = if a.buying_power > 0.0 {
            (a.equity - a.cash) / a.buying_power * 100.0
        } else {
            0.0
        };
        let bp_color = if bp_util < 50.0 {
            Color::Green
        } else if bp_util < 80.0 {
            Color::Yellow
        } else {
            Color::Red
        };

        // Simple VaR estimate: 2% assumption * total absolute market value * sqrt(1/N) diversification
        let total_mv: f64 = app.positions.iter().map(|p| p.market_value.abs()).sum();
        let n_positions = app.positions.len().max(1) as f64;
        let diversification = 1.0 / n_positions.sqrt();
        let portfolio_var = 0.02 * total_mv * diversification;

        vec![
            Line::from(vec![
                Span::styled("Equity: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("${:.2}", a.equity),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled("Cash: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("${:.2}", a.cash), Style::default().fg(Color::Cyan)),
                Span::raw("  "),
                Span::styled("Buying Power: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("${:.2}", a.buying_power),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw("  "),
                Span::styled("Total P&L: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("${:+.2}", total_pl),
                    Style::default().fg(pl_color).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Portfolio: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("${:.2}", a.portfolio_value),
                    Style::default().fg(Color::White),
                ),
                Span::raw("  "),
                Span::styled("Init Margin: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("${:.2}", a.initial_margin),
                    Style::default().fg(Color::Magenta),
                ),
                Span::raw("  "),
                Span::styled("Maint Margin: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("${:.2}", a.maintenance_margin),
                    Style::default().fg(Color::Magenta),
                ),
                Span::raw("  "),
                Span::styled(
                    if a.pattern_day_trader { "PDT " } else { "" },
                    Style::default().fg(Color::Red),
                ),
                Span::styled(
                    if a.trading_blocked { "BLOCKED" } else { "" },
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Margin Level: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    if a.initial_margin > 0.0 {
                        format!("{:.1}%", margin_level)
                    } else {
                        "N/A".to_string()
                    },
                    Style::default()
                        .fg(margin_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled("BP Util: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{:.1}%", bp_util), Style::default().fg(bp_color)),
                Span::raw("  "),
                Span::styled("VaR (2%): ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("${:.2}", portfolio_var),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw("  "),
                Span::styled("VaR/Equity: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    if a.equity > 0.0 {
                        format!("{:.2}%", portfolio_var / a.equity * 100.0)
                    } else {
                        "N/A".to_string()
                    },
                    Style::default().fg(if a.equity > 0.0 && portfolio_var / a.equity < 0.05 {
                        Color::Green
                    } else {
                        Color::Yellow
                    }),
                ),
            ]),
        ]
    } else {
        vec![Line::from(Span::styled(
            "Connecting...",
            Style::default().fg(Color::Yellow),
        ))]
    };
    let account_block = Paragraph::new(account_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Risk Dashboard "),
    );
    f.render_widget(account_block, chunks[0]);

    // ── Position summary stats ──
    let n_long = app.positions.iter().filter(|p| p.qty > 0.0).count();
    let n_short = app.positions.iter().filter(|p| p.qty < 0.0).count();
    let largest = app.positions.iter().max_by(|a, b| {
        a.market_value
            .abs()
            .partial_cmp(&b.market_value.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let best = app.positions.iter().max_by(|a, b| {
        a.unrealized_pl
            .partial_cmp(&b.unrealized_pl)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let worst = app.positions.iter().min_by(|a, b| {
        a.unrealized_pl
            .partial_cmp(&b.unrealized_pl)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut summary_lines = vec![
        Line::from(vec![
            Span::styled("Positions: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", app.positions.len()),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  ("),
            Span::styled(format!("{}L", n_long), Style::default().fg(Color::Green)),
            Span::raw(" / "),
            Span::styled(format!("{}S", n_short), Style::default().fg(Color::Red)),
            Span::raw(")  "),
            Span::styled("Largest: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                largest
                    .map(|p| format!("{} ${:.0}", p.symbol, p.market_value.abs()))
                    .unwrap_or_else(|| "-".to_string()),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("Best: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                best.map(|p| format!("{} {:+.2}", p.symbol, p.unrealized_pl))
                    .unwrap_or_else(|| "-".to_string()),
                Style::default().fg(Color::Green),
            ),
            Span::raw("  "),
            Span::styled("Worst: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                worst
                    .map(|p| format!("{} {:+.2}", p.symbol, p.unrealized_pl))
                    .unwrap_or_else(|| "-".to_string()),
                Style::default().fg(Color::Red),
            ),
        ]),
    ];
    if app.positions.is_empty() {
        summary_lines = vec![Line::from(Span::styled(
            "No open positions",
            Style::default().fg(Color::DarkGray),
        ))];
    }
    let summary_block = Paragraph::new(summary_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Position Summary "),
    );
    f.render_widget(summary_block, chunks[1]);

    // ── Positions table with per-position VaR ──
    let pos_rows: Vec<Row> = app
        .positions
        .iter()
        .map(|p| {
            let pl_color = if p.unrealized_pl >= 0.0 {
                Color::Green
            } else {
                Color::Red
            };
            let price = if p.qty.abs() > 0.0 {
                p.market_value.abs() / p.qty.abs()
            } else {
                p.avg_entry_price
            };
            let pos_var = 0.02 * p.market_value.abs(); // 2% VaR per position
            Row::new(vec![
                Cell::from(p.symbol.clone()).style(
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(format!(
                    "{} {:.0}",
                    if p.qty > 0.0 { "L" } else { "S" },
                    p.qty.abs()
                ))
                .style(Style::default().fg(if p.qty > 0.0 {
                    Color::Green
                } else {
                    Color::Red
                })),
                Cell::from(format!("${:.2}", price)).style(Style::default().fg(Color::Cyan)),
                Cell::from(format!("${:.0}", p.market_value.abs()))
                    .style(Style::default().fg(Color::White)),
                Cell::from(format!("{:+.2}", p.unrealized_pl)).style(Style::default().fg(pl_color)),
                Cell::from(format!("${:.0}", pos_var)).style(Style::default().fg(Color::Yellow)),
            ])
        })
        .collect();

    let pos_table = Table::new(
        pos_rows,
        [
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(10),
        ],
    )
    .header(
        Row::new(vec![
            "Symbol", "Side/Qty", "Price", "Mkt Val", "P&L", "VaR(2%)",
        ])
        .style(Style::default().fg(Color::DarkGray)),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Positions ({}) ", app.positions.len())),
    );
    f.render_widget(pos_table, chunks[2]);

    // ── Total P&L + Portfolio VaR ──
    let total_pl: f64 = app.positions.iter().map(|p| p.unrealized_pl).sum();
    let pl_color = if total_pl >= 0.0 {
        Color::Green
    } else {
        Color::Red
    };
    let total_mv: f64 = app.positions.iter().map(|p| p.market_value.abs()).sum();
    let n_pos = app.positions.len().max(1) as f64;
    let portfolio_var = 0.02 * total_mv * (1.0 / n_pos.sqrt());
    let open_orders = app.orders.len();

    let pl_text = Paragraph::new(vec![Line::from(vec![
        Span::styled("Total P&L: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("${:+.2}", total_pl),
            Style::default().fg(pl_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("Portfolio VaR: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("${:.2}", portfolio_var),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("Open Orders: ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{}", open_orders), Style::default().fg(Color::Cyan)),
    ])])
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(pl_text, chunks[3]);
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
    if price_range <= 0.0 {
        return;
    }

    let chart_width = chunks[0].width.saturating_sub(2) as usize;
    let chart_height = chunks[0].height.saturating_sub(2) as usize;
    let visible_bars = bars.len().min(chart_width);
    let start = bars.len().saturating_sub(visible_bars);

    // ADR-092: Compute SMA(20) overlay for visible bars
    let sma_period = 20;
    let mut sma_values: Vec<Option<f64>> = vec![None; bars.len()];
    if bars.len() >= sma_period {
        for i in (sma_period - 1)..bars.len() {
            let sum: f64 = bars[(i + 1 - sma_period)..=i].iter().map(|b| b.close).sum();
            sma_values[i] = Some(sum / sma_period as f64);
        }
    }

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

            // Check if SMA line passes through this row
            let sma_row = sma_values[start + col]
                .map(|v| ((max_price - v) / price_range * chart_height as f64) as usize);

            let ch = if row >= body_top && row <= body_bot {
                Span::styled("█", Style::default().fg(color))
            } else if row >= high_row && row <= low_row {
                // Wick — but show SMA if it coincides
                if sma_row == Some(row) {
                    Span::styled("╋", Style::default().fg(Color::Yellow))
                } else {
                    Span::styled("│", Style::default().fg(Color::DarkGray))
                }
            } else if sma_row == Some(row) {
                Span::styled("─", Style::default().fg(Color::Yellow))
            } else {
                Span::raw(" ")
            };
            spans.push(ch);
        }

        // Price label on right
        if row % 4 == 0 {
            spans.push(Span::styled(
                format!(" {:.2}", y_price),
                Style::default().fg(Color::DarkGray),
            ));
        }

        lines.push(Line::from(spans));
    }

    let last = match bars.last() {
        Some(b) => b,
        None => return, // guarded above, but defensive
    };
    let change = last.close - last.open;
    let change_pct = if last.open > 0.0 {
        change / last.open * 100.0
    } else {
        0.0
    };
    let title = format!(
        " {} @ {} | O:{:.2} H:{:.2} L:{:.2} C:{:.2} | {:+.2} ({:+.2}%) ",
        app.chart_symbol,
        app.chart_timeframe,
        last.open,
        last.high,
        last.low,
        last.close,
        change,
        change_pct,
    );

    let chart_widget =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(chart_widget, chunks[0]);

    // Volume bar at bottom
    let max_vol = bars[start..]
        .iter()
        .map(|b| b.volume)
        .fold(0.0f64, f64::max);
    let vol_spans: Vec<Span> = bars[start..]
        .iter()
        .map(|b| {
            let h = if max_vol > 0.0 {
                (b.volume / max_vol * 8.0) as usize
            } else {
                0
            };
            let ch = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"][h.min(7)];
            let color = if b.close >= b.open {
                Color::Green
            } else {
                Color::Red
            };
            Span::styled(ch, Style::default().fg(color))
        })
        .collect();
    let vol_line = Paragraph::new(Line::from(vol_spans))
        .block(Block::default().borders(Borders::ALL).title(" Volume "));
    f.render_widget(vol_line, chunks[1]);
}

fn draw_orders(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let rows: Vec<Row> = app
        .orders
        .iter()
        .enumerate()
        .map(|(i, o)| {
            let side_color = if o.side == "buy" {
                Color::Green
            } else {
                Color::Red
            };
            let selected = i == app.selected_order;
            let row_style = if selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            let marker = if selected { "▸ " } else { "  " };
            let price_str = o
                .limit_price
                .as_deref()
                .or(o.stop_price.as_deref())
                .unwrap_or("market");
            Row::new(vec![
                Cell::from(format!("{}{}", marker, o.symbol))
                    .style(Style::default().fg(Color::White)),
                Cell::from(o.side.clone()).style(Style::default().fg(side_color)),
                Cell::from(o.order_type.clone()).style(Style::default().fg(Color::Cyan)),
                Cell::from(o.qty.clone()).style(Style::default().fg(Color::Yellow)),
                Cell::from(price_str.to_string()).style(Style::default().fg(Color::Magenta)),
                Cell::from(o.status.clone()).style(Style::default().fg(Color::DarkGray)),
            ])
            .style(row_style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(6),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(12),
        ],
    )
    .header(
        Row::new(vec!["  Symbol", "Side", "Type", "Qty", "Price", "Status"])
            .style(Style::default().fg(Color::DarkGray)),
    )
    .block(Block::default().borders(Borders::ALL).title(format!(
        " Open Orders ({}) — ↑↓ select, d=cancel ",
        app.orders.len()
    )));
    f.render_widget(table, chunks[0]);

    // Selected order detail
    if !app.orders.is_empty() {
        let sel = &app.orders[app.selected_order.min(app.orders.len() - 1)];
        let detail = Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{} ", sel.symbol),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{} {} {} ", sel.side, sel.order_type, sel.qty),
                Style::default().fg(Color::White),
            ),
            Span::styled(
                format!("ID: {} ", &sel.id[..sel.id.len().min(8)]),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                &sel.created_at[..sel.created_at.len().min(19)],
                Style::default().fg(Color::DarkGray),
            ),
        ]))
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(detail, chunks[1]);
    }
}

fn draw_accounts(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(5),
            Constraint::Length(8),
        ])
        .split(area);

    // Aggregate summary
    let alpaca_equity = app.account.as_ref().map(|a| a.equity).unwrap_or(0.0);
    let alpaca_pl: f64 = app.positions.iter().map(|p| p.unrealized_pl).sum();
    let alpaca_pos_count = app.positions.len();
    let total_equity = alpaca_equity;
    let total_pl = alpaca_pl;
    let total_pos = alpaca_pos_count;

    let summary_text = vec![
        Line::from(vec![
            Span::styled("Total Equity: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("${:.2}", total_equity),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("Total P&L: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("${:+.2}", total_pl),
                Style::default()
                    .fg(if total_pl >= 0.0 {
                        Color::Green
                    } else {
                        Color::Red
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("Positions: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", total_pos), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![Span::styled(
            "Accounts: 1 Alpaca",
            Style::default().fg(Color::DarkGray),
        )]),
    ];
    let summary_block = Paragraph::new(summary_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Aggregate Portfolio "),
    );
    f.render_widget(summary_block, chunks[0]);

    // Account table
    let mut rows: Vec<Row> = vec![];

    // Alpaca row
    if app.account.is_some() {
        let alpaca_name = "Alpaca (Paper)";
        let pl_color = if alpaca_pl >= 0.0 {
            Color::Green
        } else {
            Color::Red
        };
        rows.push(Row::new(vec![
            Cell::from(alpaca_name).style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from("Alpaca").style(Style::default().fg(Color::Green)),
            Cell::from("USD").style(Style::default().fg(Color::DarkGray)),
            Cell::from(format!("${:.2}", alpaca_equity)).style(Style::default().fg(Color::White)),
            Cell::from(format!("{}", alpaca_pos_count)).style(Style::default().fg(Color::Cyan)),
            Cell::from(format!("{:+.2}", alpaca_pl)).style(Style::default().fg(pl_color)),
            Cell::from("Connected").style(Style::default().fg(Color::Green)),
        ]));
    }

    let account_table = Table::new(
        rows,
        [
            Constraint::Length(18),
            Constraint::Length(12),
            Constraint::Length(5),
            Constraint::Length(14),
            Constraint::Length(5),
            Constraint::Length(14),
            Constraint::Length(16),
        ],
    )
    .header(
        Row::new(vec![
            "Account", "Type", "Cur", "Equity", "Pos", "P&L", "Status",
        ])
        .style(Style::default().fg(Color::DarkGray)),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Accounts (1) "),
    );
    f.render_widget(account_table, chunks[1]);

    // Weight breakdown
    let mut weight_lines: Vec<Line> = vec![];
    if total_equity > 0.0 {
        if alpaca_equity > 0.0 {
            let pct = alpaca_equity / total_equity * 100.0;
            let bar_len = (pct / 100.0 * 40.0) as usize;
            weight_lines.push(Line::from(vec![
                Span::styled(
                    format!("{:<16}", "Alpaca"),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled("█".repeat(bar_len), Style::default().fg(Color::Green)),
                Span::styled(
                    format!(" {:.1}%", pct),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
    } else {
        weight_lines.push(Line::from(Span::styled(
            "No account data",
            Style::default().fg(Color::DarkGray),
        )));
    }
    let weight_widget = Paragraph::new(weight_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Account Weights "),
    );
    f.render_widget(weight_widget, chunks[2]);
}

fn draw_log(f: &mut Frame, app: &App, area: Rect) {
    let visible = app.log_messages.len().min(area.height as usize);
    let start = app.log_messages.len().saturating_sub(visible);
    let lines: Vec<Line> = app
        .log_messages
        .iter()
        .skip(start)
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
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
            Constraint::Length(8),
        ])
        .split(f.area());

    // Tab bar with status line
    let tab_titles: Vec<Line> = app.tabs.iter().map(|t| Line::from(*t)).collect();
    let market_status = if app.market_open { "OPEN" } else { "CLOSED" };
    let market_color = if app.market_open {
        Color::Green
    } else {
        Color::Red
    };
    let refresh_ago = app.last_refresh.elapsed().as_secs();
    let equity_str = app
        .account
        .as_ref()
        .map(|a| format!("${:.2}", a.equity))
        .unwrap_or_else(|| "---".to_string());
    let title_str = format!(
        " TyphooN Terminal | Mkt:{} | {}s ago | Eq:{} ",
        market_status, refresh_ago, equity_str,
    );
    let title_spans = vec![
        Span::styled(
            " TyphooN Terminal",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" | Mkt:", Style::default().fg(Color::DarkGray)),
        Span::styled(
            market_status,
            Style::default()
                .fg(market_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" | {}s ago", refresh_ago),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(" | Eq:", Style::default().fg(Color::DarkGray)),
        Span::styled(
            equity_str,
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ];
    let _ = title_str; // suppress unused warning; we use title_spans below
    let tab_widget = Tabs::new(tab_titles)
        .select(app.active_tab)
        .highlight_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Line::from(title_spans)),
        );
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

            let rows: Vec<Row> = app
                .positions
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let pl_color = if p.unrealized_pl >= 0.0 {
                        Color::Green
                    } else {
                        Color::Red
                    };
                    let price = if p.qty.abs() > 0.0 {
                        p.market_value.abs() / p.qty.abs()
                    } else {
                        p.avg_entry_price
                    };
                    let selected = i == app.selected_position;
                    let row_style = if selected {
                        Style::default().bg(Color::DarkGray)
                    } else {
                        Style::default()
                    };
                    let marker = if selected { "▸ " } else { "  " };
                    Row::new(vec![
                        Cell::from(format!("{}{}", marker, p.symbol)).style(
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Cell::from(p.side.clone()).style(Style::default().fg(
                            if p.side == "long" {
                                Color::Green
                            } else {
                                Color::Red
                            },
                        )),
                        Cell::from(format!("{:.0}", p.qty.abs())),
                        Cell::from(format!("${:.2}", p.avg_entry_price))
                            .style(Style::default().fg(Color::Cyan)),
                        Cell::from(format!("${:.2}", price))
                            .style(Style::default().fg(Color::White)),
                        Cell::from(format!("${:.2}", p.market_value.abs())),
                        Cell::from(format!("{:+.2}", p.unrealized_pl))
                            .style(Style::default().fg(pl_color)),
                    ])
                    .style(row_style)
                })
                .collect();
            let table = Table::new(
                rows,
                [
                    Constraint::Length(12),
                    Constraint::Length(6),
                    Constraint::Length(8),
                    Constraint::Length(12),
                    Constraint::Length(12),
                    Constraint::Length(14),
                    Constraint::Length(12),
                ],
            )
            .header(
                Row::new(vec![
                    "  Symbol",
                    "Side",
                    "Qty",
                    "Entry",
                    "Current",
                    "Mkt Value",
                    "P&L",
                ])
                .style(Style::default().fg(Color::DarkGray)),
            )
            .block(Block::default().borders(Borders::ALL).title(format!(
                " Positions ({}) — ↑↓ select, Enter=chart, x=close, p=partial ",
                app.positions.len()
            )));
            f.render_widget(table, chunks[0]);

            // Selected position detail
            if !app.positions.is_empty() {
                let sel = &app.positions[app.selected_position.min(app.positions.len() - 1)];
                let price = if sel.qty.abs() > 0.0 {
                    sel.market_value.abs() / sel.qty.abs()
                } else {
                    sel.avg_entry_price
                };
                let pl_pct = if sel.avg_entry_price > 0.0 {
                    (price / sel.avg_entry_price - 1.0) * 100.0
                } else {
                    0.0
                };
                let detail = Paragraph::new(Line::from(vec![
                    Span::styled(
                        format!("{} ", sel.symbol),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(
                            "{} {:.0} @ ${:.2} ",
                            sel.side,
                            sel.qty.abs(),
                            sel.avg_entry_price
                        ),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(
                        format!("→ ${:.2} ({:+.1}%) ", price, pl_pct),
                        Style::default().fg(if pl_pct >= 0.0 {
                            Color::Green
                        } else {
                            Color::Red
                        }),
                    ),
                    Span::styled(
                        format!("P&L: {:+.2}", sel.unrealized_pl),
                        Style::default()
                            .fg(if sel.unrealized_pl >= 0.0 {
                                Color::Green
                            } else {
                                Color::Red
                            })
                            .add_modifier(Modifier::BOLD),
                    ),
                ]))
                .block(Block::default().borders(Borders::ALL));
                f.render_widget(detail, chunks[1]);
            }
        }
        3 => draw_orders(f, app, main_layout[1]),
        4 => {
            // Watchlist with live quotes
            let rows: Vec<Row> = app
                .watchlist_quotes
                .iter()
                .map(|(sym, bid, ask, last)| {
                    // Find previous close from positions avg_entry if available, else show N/A for change
                    let pos_entry = app
                        .positions
                        .iter()
                        .find(|p| &p.symbol == sym)
                        .map(|p| p.avg_entry_price);
                    let prev = app.watchlist_prev_close.get(sym).copied().or(pos_entry);
                    let (change, change_pct) = if let Some(prev_price) = prev {
                        if prev_price > 0.0 && *last > 0.0 {
                            (last - prev_price, (last / prev_price - 1.0) * 100.0)
                        } else {
                            (0.0, 0.0)
                        }
                    } else {
                        (0.0, 0.0)
                    };
                    let chg_color = if change >= 0.0 {
                        Color::Green
                    } else {
                        Color::Red
                    };
                    let last_str = if *last > 0.0 {
                        format!("{:.2}", last)
                    } else {
                        "---".to_string()
                    };
                    let bid_str = if *bid > 0.0 {
                        format!("{:.2}", bid)
                    } else {
                        "---".to_string()
                    };
                    let ask_str = if *ask > 0.0 {
                        format!("{:.2}", ask)
                    } else {
                        "---".to_string()
                    };
                    let chg_str = if prev.is_some() && *last > 0.0 {
                        format!("{:+.2}", change)
                    } else {
                        "---".to_string()
                    };
                    let pct_str = if prev.is_some() && *last > 0.0 {
                        format!("{:+.2}%", change_pct)
                    } else {
                        "---".to_string()
                    };

                    Row::new(vec![
                        Cell::from(sym.clone()).style(
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Cell::from(last_str).style(Style::default().fg(Color::Cyan)),
                        Cell::from(chg_str).style(Style::default().fg(chg_color)),
                        Cell::from(pct_str).style(Style::default().fg(chg_color)),
                        Cell::from(bid_str).style(Style::default().fg(Color::Green)),
                        Cell::from(ask_str).style(Style::default().fg(Color::Red)),
                    ])
                })
                .collect();

            // Also show symbols with no quote yet
            let quoted_syms: Vec<&String> =
                app.watchlist_quotes.iter().map(|(s, _, _, _)| s).collect();
            let mut extra_rows: Vec<Row> = app
                .watchlist
                .iter()
                .filter(|s| !quoted_syms.contains(s))
                .map(|s| {
                    Row::new(vec![
                        Cell::from(s.clone()).style(Style::default().fg(Color::White)),
                        Cell::from("---"),
                        Cell::from("---"),
                        Cell::from("---"),
                        Cell::from("---"),
                        Cell::from("---"),
                    ])
                })
                .collect();
            let mut all_rows = rows;
            all_rows.append(&mut extra_rows);

            let wl_table = Table::new(
                all_rows,
                [
                    Constraint::Length(12),
                    Constraint::Length(12),
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Length(12),
                    Constraint::Length(12),
                ],
            )
            .header(
                Row::new(vec!["Symbol", "Last", "Change", "Chg%", "Bid", "Ask"])
                    .style(Style::default().fg(Color::DarkGray)),
            )
            .block(Block::default().borders(Borders::ALL).title(format!(
                " Watchlist ({}) — :watch SYM to add ",
                app.watchlist.len()
            )));
            f.render_widget(wl_table, main_layout[1]);
        }
        5 => draw_accounts(f, app, main_layout[1]),
        6 => draw_log(f, app, main_layout[1]),
        _ => {}
    }

    // Context-sensitive keybinds bar + command input
    let cmd_style = if app.command_mode {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let cmd_text = if app.command_mode {
        format!(":{}", app.command_input)
    } else {
        // Show relevant keybinds for current tab
        match app.active_tab {
            0 => "Tab:next  1-7:jump  ::cmd  r:refresh  :closeall :cancelall  q:quit".to_string(),
            1 => "Tab:next  :chart SYM [TF]  :tf TF  r:refresh  q:quit".to_string(),
            2 => "↑↓/jk:sel  Enter:chart  x:close  p:partial  :closeall  q:quit".to_string(),
            3 => "↑↓:sel  d:cancel  :cancelall  :history  q:quit".to_string(),
            4 => ":watch SYM  :limit/:stop/:bracket  q:quit".to_string(),
            5 => "Tab:next  r:refresh  q:quit".to_string(),
            6 => "Tab:next  :history [N]  r:refresh  q:quit".to_string(),
            _ => "Tab:next  ::cmd  q:quit".to_string(),
        }
    };
    let cmd_widget = Paragraph::new(cmd_text)
        .style(cmd_style)
        .block(Block::default().borders(Borders::ALL).title(" Command "));
    f.render_widget(cmd_widget, main_layout[2]);

    // Log (always visible)
    draw_log(f, app, main_layout[3]);
}

fn run_cache_backup_mode(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let cache_dir = resolve_cache_dir(args.cache_dir.as_ref())?;
    let db_path = cache_dir.join("typhoon_cache.db");
    let cache = SqliteCache::open(&db_path).map_err(std::io::Error::other)?;

    if let Some(path) = args.export_cache.as_ref() {
        let path_str = path.to_string_lossy();
        let result = if let Some(passphrase) = args.cache_backup_passphrase.as_deref() {
            cache.export_backup_encrypted(path_str.as_ref(), passphrase)
        } else {
            cache.export_backup(path_str.as_ref())
        }
        .map_err(std::io::Error::other)?;
        println!(
            "Exported cache {} to {}: {}",
            db_path.display(),
            path.display(),
            result
        );
        return Ok(());
    }

    if let Some(path) = args.import_cache.as_ref() {
        let path_str = path.to_string_lossy();
        let is_encrypted = SqliteCache::backup_file_is_encrypted(path_str.as_ref())
            .map_err(std::io::Error::other)?;
        let result = if is_encrypted {
            let Some(passphrase) = args.cache_backup_passphrase.as_deref() else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Encrypted cache import requires --cache-backup-passphrase or TYPHOON_CACHE_BACKUP_PASSPHRASE.",
                )
                .into());
            };
            cache.import_backup_encrypted(path_str.as_ref(), passphrase)
        } else {
            cache.import_backup(path_str.as_ref())
        }
        .map_err(std::io::Error::other)?;
        println!(
            "Imported cache backup {} into {}: {}",
            path.display(),
            db_path.display(),
            result
        );
        return Ok(());
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "Cache backup mode requires --export-cache PATH or --import-cache PATH.",
    )
    .into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.export_cache.is_some() || args.import_cache.is_some() {
        return run_cache_backup_mode(&args);
    }

    if args.mcp_server {
        return mcp::run_mcp_server(args.cache_dir.clone());
    }

    if args.cache_stats || args.sync_status || args.broker_status {
        let (cache, db_path) = open_cli_cache(args.cache_dir.as_ref())?;
        if args.cache_stats {
            for line in cache_stats_lines(&cache, &db_path) {
                println!("{line}");
            }
        }
        if args.sync_status {
            for line in sync_status_lines(&cache) {
                println!("{line}");
            }
        }
        if args.broker_status {
            for line in broker_status_lines(&cache) {
                println!("{line}");
            }
        }
        return Ok(());
    }

    // Load keys: CLI args → env vars → GUI terminal's encrypted storage
    let (api_key, secret_key) = match (args.api_key.clone(), args.secret_key.clone()) {
        (Some(k), Some(s)) => (k, s),
        _ => {
            // Try env vars
            match (
                std::env::var("ALPACA_API_KEY").ok(),
                std::env::var("ALPACA_SECRET_KEY").ok(),
            ) {
                (Some(k), Some(s)) => (k, s),
                _ => {
                    // Try GUI terminal's encrypted credential storage
                    match creds::load_saved_credentials(args.paper) {
                        Some((k, s, name)) => {
                            eprintln!(
                                "Using saved credentials: {} ({})",
                                name,
                                if args.paper { "paper" } else { "live" }
                            );
                            (k, s)
                        }
                        None => {
                            eprintln!(
                                "No API keys found. Provide via --api-key/--secret-key, ALPACA_API_KEY/ALPACA_SECRET_KEY env vars, or save credentials in the GUI terminal (Ctrl+K → SETTINGS)."
                            );
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
        println!(
            "Equity: ${:.2} | Cash: ${:.2} | BP: ${:.2} | Portfolio: ${:.2}",
            a.equity, a.cash, a.buying_power, a.portfolio_value
        );
        return Ok(());
    }
    if args.positions {
        let positions = broker.get_positions().await?;
        if positions.is_empty() {
            println!("No open positions.");
            return Ok(());
        }
        println!(
            "{:<10} {:<6} {:<8} {:<12} {:<12} {:<12}",
            "Symbol", "Side", "Qty", "Entry", "MktVal", "P&L"
        );
        for p in &positions {
            let price = if p.qty.abs() > 0.0 {
                p.market_value.abs() / p.qty.abs()
            } else {
                p.avg_entry_price
            };
            println!(
                "{:<10} {:<6} {:<8.0} ${:<11.2} ${:<11.2} {:+.2}",
                p.symbol,
                p.side,
                p.qty.abs(),
                price,
                p.market_value.abs(),
                p.unrealized_pl
            );
        }
        return Ok(());
    }

    // Show account one-shot mode
    if args.accounts {
        match broker.get_account().await {
            Ok(a) => {
                let positions = broker.get_positions().await.unwrap_or_default();
                let pl: f64 = positions.iter().map(|p| p.unrealized_pl).sum();
                println!(
                    "{:<18} {:<12} {:<5} {:>14} {:>5} {:>14} {:<12}",
                    "Account", "Type", "Cur", "Equity", "Pos", "P&L", "Status"
                );
                println!("{:-<80}", "");
                println!(
                    "{:<18} {:<12} {:<5} {:>14.2} {:>5} {:>14.2} {:<12}",
                    "Alpaca",
                    "Alpaca",
                    "USD",
                    a.equity,
                    positions.len(),
                    pl,
                    "Connected"
                );
            }
            Err(e) => {
                eprintln!("Alpaca connection failed: {e}");
            }
        }
        return Ok(());
    }

    // Interactive TUI mode
    let watchlist = args
        .watch
        .map(|w| w.split(',').map(|s| s.trim().to_uppercase()).collect())
        .unwrap_or_default();
    let symbol = args.symbol.unwrap_or_else(|| "SMCI".to_string());

    let mut app = App::new(broker, symbol, watchlist, args.cache_dir.clone());

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
                            if cmd.to_lowercase() == "q" || cmd.to_lowercase() == "quit" {
                                break;
                            }
                            app.handle_command(&cmd).await;
                        }
                        KeyCode::Esc => {
                            app.command_input.clear();
                            app.command_mode = false;
                        }
                        KeyCode::Backspace => {
                            app.command_input.pop();
                        }
                        KeyCode::Char(c) => {
                            app.command_input.push(c);
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char(':') => {
                            app.command_mode = true;
                        }
                        KeyCode::Tab => {
                            app.active_tab = (app.active_tab + 1) % app.tabs.len();
                        }
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
                            if app.active_tab == 2 && app.selected_position > 0 {
                                app.selected_position -= 1;
                            }
                            if app.active_tab == 3 && app.selected_order > 0 {
                                app.selected_order -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if app.active_tab == 2
                                && app.selected_position + 1 < app.positions.len()
                            {
                                app.selected_position += 1;
                            }
                            if app.active_tab == 3 && app.selected_order + 1 < app.orders.len() {
                                app.selected_order += 1;
                            }
                        }
                        // Enter: action on selected item
                        KeyCode::Enter => {
                            if app.active_tab == 2 && !app.positions.is_empty() {
                                let pos = &app.positions[app.selected_position];
                                let sym = pos.symbol.clone();
                                // Switch chart to this symbol
                                app.chart_symbol = sym.clone();
                                app.last_refresh = Instant::now() - Duration::from_secs(60);
                                app.log(
                                    &format!(
                                        "Selected {sym} — press 'x' to close, 'p' for partial close"
                                    ),
                                    Color::Cyan,
                                );
                            }
                            if app.active_tab == 3 && !app.orders.is_empty() {
                                let order = &app.orders[app.selected_order];
                                app.log(
                                    &format!(
                                        "Order {} {} {} — press 'd' to cancel",
                                        order.symbol, order.side, order.order_type
                                    ),
                                    Color::Cyan,
                                );
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
                                        app.log(
                                            &format!("CLOSED {sym}: {}", r.status),
                                            Color::Green,
                                        );
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
                                app.log(
                                    &format!(
                                        "Partial close {sym} ({half} of {:.0})...",
                                        pos.qty.abs()
                                    ),
                                    Color::Yellow,
                                );
                                match app.broker.close_position(&sym, Some(half)).await {
                                    Ok(r) => {
                                        app.log(
                                            &format!("PARTIAL CLOSE {sym} {half}: {}", r.status),
                                            Color::Green,
                                        );
                                        app.last_refresh = Instant::now() - Duration::from_secs(60);
                                    }
                                    Err(e) => {
                                        app.log(&format!("Partial close failed: {e}"), Color::Red)
                                    }
                                }
                            }
                        }
                        // d: cancel selected order
                        KeyCode::Char('d') => {
                            if app.active_tab == 3 && !app.orders.is_empty() {
                                let order = &app.orders[app.selected_order];
                                let id = order.id.clone();
                                let sym = order.symbol.clone();
                                app.log(
                                    &format!("Cancelling order {sym} ({id})..."),
                                    Color::Yellow,
                                );
                                match app.broker.cancel_order(&id).await {
                                    Ok(_) => {
                                        app.log(&format!("CANCELLED {sym} order"), Color::Green);
                                        app.last_refresh = Instant::now() - Duration::from_secs(60);
                                    }
                                    Err(e) => app.log(&format!("Cancel failed: {e}"), Color::Red),
                                }
                            }
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            break;
                        }
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
