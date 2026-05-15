//! Model Context Protocol stdio server for TyphooN Terminal.
//!
//! The server is intentionally read-only. Its primary surface is the
//! `research_packet` tool, which builds an AI-readable markdown packet from the
//! shared TyphooN SQLite cache.

use crate::broker;
use crate::resolve_cache_dir;
use chrono::Utc;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashSet};
use std::fmt::Write as _;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;
use typhoon_engine::core::cache::SqliteCache;
use typhoon_engine::core::{darwin, fundamentals, sec_filing, var};

const LATEST_PROTOCOL_VERSION: &str = "2025-11-25";
const SUPPORTED_PROTOCOL_VERSIONS: &[&str] =
    &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

#[derive(Clone)]
pub struct McpServer {
    cache: Arc<SqliteCache>,
    db_path: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Framing {
    Line,
    Header,
}

#[derive(Debug)]
struct ResearchPacketOptions {
    include_raw_surfaces: bool,
    max_snapshot_chars: usize,
    max_surfaces: usize,
}

#[derive(Debug)]
struct ResearchSurface {
    table: String,
    updated_at: i64,
    snapshot_json: String,
}

pub fn run_mcp_server(cache_dir: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let cache_dir = resolve_cache_dir(cache_dir.as_ref())?;
    let db_path = cache_dir.join("typhoon_cache.db");
    let cache = if db_path.exists() {
        SqliteCache::open_readonly(&db_path).or_else(|_| SqliteCache::open(&db_path))?
    } else {
        SqliteCache::open(&db_path)?
    };
    let server = McpServer {
        cache: Arc::new(cache),
        db_path,
    };
    server.serve_stdio()
}

impl McpServer {
    fn serve_stdio(&self) -> Result<(), Box<dyn std::error::Error>> {
        eprintln!(
            "TyphooN Terminal MCP server using cache {}",
            self.db_path.display()
        );

        let stdin = io::stdin();
        let mut reader = io::BufReader::new(stdin.lock());
        let stdout = io::stdout();
        let mut writer = io::BufWriter::new(stdout.lock());

        while let Some((message, framing)) = read_json_rpc_message(&mut reader)? {
            if let Some(response) = self.handle_message(message) {
                write_json_rpc_message(&mut writer, &response, framing)?;
            }
        }

        Ok(())
    }

    fn handle_message(&self, message: Value) -> Option<Value> {
        let id = message.get("id").cloned();
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            return id.map(|id| jsonrpc_error(id, -32600, "Invalid JSON-RPC request"));
        };

        match method {
            "initialize" => id.map(|id| {
                let requested = message
                    .get("params")
                    .and_then(|p| p.get("protocolVersion"))
                    .and_then(Value::as_str)
                    .unwrap_or(LATEST_PROTOCOL_VERSION);
                let protocol_version = if SUPPORTED_PROTOCOL_VERSIONS.contains(&requested) {
                    requested
                } else {
                    LATEST_PROTOCOL_VERSION
                };
                jsonrpc_result(
                    id,
                    json!({
                        "protocolVersion": protocol_version,
                        "capabilities": {
                            "tools": {
                                "listChanged": false
                            }
                        },
                        "serverInfo": {
                            "name": "typhoon-terminal",
                            "title": "TyphooN Terminal",
                            "version": env!("CARGO_PKG_VERSION")
                        },
                        "instructions": "Use the research_packet tool to request TyphooN Terminal's cached research packet for one or more symbols. The server is read-only and never places trades."
                    }),
                )
            }),
            "notifications/initialized" => None,
            "ping" => id.map(|id| jsonrpc_result(id, json!({}))),
            "tools/list" => id.map(|id| jsonrpc_result(id, self.tools_list())),
            "tools/call" => id.map(|id| match self.handle_tool_call(&message) {
                Ok(result) => jsonrpc_result(id, result),
                Err(err) => jsonrpc_error(id, -32602, &err),
            }),
            _ => id.map(|id| jsonrpc_error(id, -32601, "Method not found")),
        }
    }

    fn tools_list(&self) -> Value {
        json!({
            "tools": [
                {
                    "name": "research_packet",
                    "title": "Research Packet",
                    "description": "Build a TyphooN Terminal research packet for one or more symbols from the local shared cache. Includes fundamentals, positions, filings, news, daily price/risk stats, and cached research_* snapshot surfaces.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "symbols": {
                                "description": "Ticker or comma-separated tickers, or an array of ticker strings.",
                                "oneOf": [
                                    { "type": "string" },
                                    { "type": "array", "items": { "type": "string" } }
                                ]
                            },
                            "question": {
                                "type": "string",
                                "description": "Optional question to append to the packet."
                            },
                            "include_raw_surfaces": {
                                "type": "boolean",
                                "description": "Include generic JSON snapshots from cached research_* tables.",
                                "default": true
                            },
                            "max_snapshot_chars": {
                                "type": "integer",
                                "description": "Maximum characters emitted per raw research snapshot. Use 0 for no per-snapshot truncation.",
                                "default": 1200,
                                "minimum": 0
                            },
                            "max_surfaces": {
                                "type": "integer",
                                "description": "Maximum raw research surfaces per symbol. Use 0 for all available surfaces.",
                                "default": 0,
                                "minimum": 0
                            }
                        },
                        "required": ["symbols"]
                    }
                },
                {
                    "name": "cache_stats",
                    "title": "Cache Stats",
                    "description": "Return read-only summary statistics for the TyphooN SQLite cache.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "list_cached_symbols",
                    "title": "List Cached Symbols",
                    "description": "List symbols inferred from cached bar series keys.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "limit": {
                                "type": "integer",
                                "description": "Maximum symbols to return.",
                                "default": 200,
                                "minimum": 1
                            }
                        }
                    }
                }
            ]
        })
    }

    fn handle_tool_call(&self, message: &Value) -> Result<Value, String> {
        let params = message
            .get("params")
            .ok_or_else(|| "tools/call requires params".to_string())?;
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| "tools/call requires params.name".to_string())?;
        let args = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));

        match name {
            "research_packet" => {
                let symbols = parse_symbols_arg(&args)?;
                let question = args
                    .get("question")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let include_raw_surfaces = args
                    .get("include_raw_surfaces")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                let max_snapshot_chars = args
                    .get("max_snapshot_chars")
                    .and_then(Value::as_u64)
                    .map(|n| n as usize)
                    .unwrap_or(1200);
                let max_surfaces = args
                    .get("max_surfaces")
                    .and_then(Value::as_u64)
                    .map(|n| n as usize)
                    .unwrap_or(0);
                let options = ResearchPacketOptions {
                    include_raw_surfaces,
                    max_snapshot_chars,
                    max_surfaces,
                };
                let packet = self.build_research_packet(&symbols, &question, &options)?;
                Ok(tool_text(packet))
            }
            "cache_stats" => {
                let (bar_entries, kv_entries, disk_bytes) = self.cache.stats()?;
                Ok(tool_text(
                    json!({
                        "db_path": self.db_path,
                        "bar_entries": bar_entries,
                        "kv_entries": kv_entries,
                        "disk_bytes": disk_bytes
                    })
                    .to_string(),
                ))
            }
            "list_cached_symbols" => {
                let limit = args
                    .get("limit")
                    .and_then(Value::as_u64)
                    .map(|n| n.clamp(1, 10_000) as usize)
                    .unwrap_or(200);
                let symbols = self.list_cached_symbols(limit)?;
                Ok(tool_text(
                    serde_json::to_string_pretty(&symbols)
                        .map_err(|e| format!("serialize symbols: {e}"))?,
                ))
            }
            _ => Err(format!("Unknown tool: {name}")),
        }
    }

    fn build_research_packet(
        &self,
        symbols: &[String],
        question: &str,
        options: &ResearchPacketOptions,
    ) -> Result<String, String> {
        let conn = self.cache.open_bg_read_connection()?;
        let all_fundamentals = fundamentals::get_all_fundamentals(&conn).unwrap_or_default();
        let all_filings = sec_filing::get_all_filings(&conn).unwrap_or_default();
        let all_insider_trades = sec_filing::get_all_insider_trades(&conn).unwrap_or_default();

        let mut packet = String::new();
        writeln!(packet, "# TyphooN Terminal Research Packet").ok();
        writeln!(
            packet,
            "Scope: MCP/local cache | Generated: {}",
            Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
        )
        .ok();
        writeln!(packet, "Symbols: {}", symbols.join(", ")).ok();
        writeln!(packet, "Cache: {}", self.db_path.display()).ok();
        writeln!(packet).ok();

        self.emit_global_context(&conn, &mut packet);

        for symbol in symbols {
            writeln!(packet, "---").ok();
            writeln!(packet, "## {symbol}").ok();
            writeln!(packet).ok();

            self.emit_positions(symbol, &mut packet);
            self.emit_fundamentals(symbol, &all_fundamentals, &conn, &mut packet);
            emit_filings(symbol, &all_filings, &mut packet);
            emit_insider_trades(symbol, &all_insider_trades, &mut packet);
            self.emit_price_stats(symbol, &mut packet);
            emit_recent_news(&conn, symbol, &mut packet);
            emit_ingested_web_articles(&conn, symbol, &mut packet);

            if options.include_raw_surfaces {
                emit_research_surfaces(&conn, symbol, options, &mut packet);
            }
        }

        writeln!(packet, "---").ok();
        writeln!(packet, "## Question").ok();
        if question.trim().is_empty() {
            writeln!(
                packet,
                "Using the TyphooN data above, write a concise investment research note for each symbol. Cover valuation, financial trajectory, position context, filing/news activity, volatility/risk, and the most important data gaps."
            )
            .ok();
        } else {
            writeln!(packet, "{}", question.trim()).ok();
        }

        emit_return_path(&mut packet);
        Ok(packet)
    }

    fn emit_global_context(&self, conn: &Connection, packet: &mut String) {
        let mut wrote_header = false;

        if let Some(rows) = latest_json(conn, "research_world_indices", "rows_json")
            .and_then(|v| v.as_array().cloned())
            .filter(|rows| !rows.is_empty())
        {
            ensure_global_header(packet, &mut wrote_header);
            let advancing = rows
                .iter()
                .filter(|r| value_f64(r, "change_pct") > 0.0)
                .count();
            let declining = rows
                .iter()
                .filter(|r| value_f64(r, "change_pct") < 0.0)
                .count();
            writeln!(packet, "### World Equity Indices").ok();
            writeln!(
                packet,
                "- {} indices tracked - {} advancing - {} declining",
                rows.len(),
                advancing,
                declining
            )
            .ok();
            writeln!(packet, "| Region | Ticker | Name | Last | Chg % |").ok();
            writeln!(packet, "|---|---|---|---|---|").ok();
            for row in rows.iter().take(12) {
                writeln!(
                    packet,
                    "| {} | {} | {} | {:.2} | {:+.2}% |",
                    value_str(row, "region"),
                    value_str(row, "ticker"),
                    first_nonempty(row, &["display", "name"]),
                    value_f64(row, "price"),
                    value_f64(row, "change_pct")
                )
                .ok();
            }
            writeln!(packet).ok();
        }

        if let Some(movers) = latest_json(conn, "research_market_movers", "snapshot_json") {
            let groups = [
                ("Top Gainers", "gainers"),
                ("Top Losers", "losers"),
                ("Most Active", "actives"),
            ];
            let has_any = groups.iter().any(|(_, key)| {
                movers
                    .get(*key)
                    .and_then(Value::as_array)
                    .is_some_and(|rows| !rows.is_empty())
            });
            if has_any {
                ensure_global_header(packet, &mut wrote_header);
                writeln!(packet, "### Market Movers (US)").ok();
                for (label, key) in groups {
                    if let Some(rows) = movers.get(key).and_then(Value::as_array) {
                        if !rows.is_empty() {
                            let summary = rows
                                .iter()
                                .take(6)
                                .map(|m| {
                                    format!(
                                        "{} {:+.2}%",
                                        first_nonempty(m, &["symbol", "ticker"]),
                                        value_f64(m, "change_pct")
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join(", ");
                            writeln!(packet, "- **{label}** - {summary}").ok();
                        }
                    }
                }
                writeln!(packet).ok();
            }
        }

        if let Some(rows) = latest_json(conn, "research_sector_performance", "rows_json")
            .and_then(|v| v.as_array().cloned())
            .filter(|rows| !rows.is_empty())
        {
            ensure_global_header(packet, &mut wrote_header);
            let mut sorted = rows;
            sorted.sort_by(|a, b| {
                value_f64(b, "change_pct")
                    .partial_cmp(&value_f64(a, "change_pct"))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let up = sorted
                .iter()
                .filter(|r| value_f64(r, "change_pct") > 0.0)
                .count();
            let down = sorted
                .iter()
                .filter(|r| value_f64(r, "change_pct") < 0.0)
                .count();
            writeln!(packet, "### Sector Performance").ok();
            writeln!(
                packet,
                "- {} sectors - {} up - {} down",
                sorted.len(),
                up,
                down
            )
            .ok();
            for row in sorted.iter() {
                writeln!(
                    packet,
                    "- {} {:+.2}%",
                    first_nonempty(row, &["sector", "name"]),
                    value_f64(row, "change_pct")
                )
                .ok();
            }
            writeln!(packet).ok();
        }

        if let Some(rows) = latest_json(conn, "research_currency_rates", "rows_json")
            .and_then(|v| v.as_array().cloned())
            .filter(|rows| !rows.is_empty())
        {
            ensure_global_header(packet, &mut wrote_header);
            let up = rows
                .iter()
                .filter(|r| value_f64(r, "change_pct") > 0.0)
                .count();
            let down = rows
                .iter()
                .filter(|r| value_f64(r, "change_pct") < 0.0)
                .count();
            writeln!(packet, "### World Currency Rates").ok();
            writeln!(
                packet,
                "- {} pairs - {} strengthening vs quote - {} weakening",
                rows.len(),
                up,
                down
            )
            .ok();
            for row in rows.iter().take(24) {
                writeln!(
                    packet,
                    "- {} {:.4} ({:+.2}%)",
                    first_nonempty(row, &["display", "pair", "ticker"]),
                    value_f64(row, "price"),
                    value_f64(row, "change_pct")
                )
                .ok();
            }
            writeln!(packet).ok();
        }
    }

    fn emit_positions(&self, symbol: &str, packet: &mut String) {
        let mut rows = Vec::new();
        rows.extend(load_broker_positions(
            &self.cache,
            "broker:positions",
            "Alpaca",
            symbol,
        ));
        rows.extend(load_broker_positions(
            &self.cache,
            "broker:tt_positions",
            "tastytrade",
            symbol,
        ));
        rows.extend(load_broker_positions(
            &self.cache,
            "broker:kr_positions",
            "Kraken",
            symbol,
        ));
        rows.extend(load_darwin_positions(&self.cache, symbol));

        if rows.is_empty() {
            return;
        }

        writeln!(packet, "### Current User Position").ok();
        writeln!(
            packet,
            "The user holds the following open position(s) in this symbol. Treat this as primary context when asked about the user's position."
        )
        .ok();
        writeln!(packet).ok();
        for row in rows {
            writeln!(packet, "- {row}").ok();
        }
        writeln!(packet).ok();
    }

    fn emit_fundamentals(
        &self,
        symbol: &str,
        all_fundamentals: &[fundamentals::Fundamentals],
        conn: &Connection,
        packet: &mut String,
    ) {
        let fund = fundamentals::get_fundamentals(conn, symbol)
            .ok()
            .flatten()
            .or_else(|| {
                all_fundamentals
                    .iter()
                    .find(|f| f.symbol.eq_ignore_ascii_case(symbol))
                    .cloned()
            });

        let Some(f) = fund else {
            writeln!(
                packet,
                "_No fundamentals on file for this symbol. Run EVSCRAPE or the GUI research scrape to populate._"
            )
            .ok();
            writeln!(packet).ok();
            return;
        };

        writeln!(
            packet,
            "**{}** - {} / {}",
            if f.company_name.is_empty() {
                "(unnamed)"
            } else {
                f.company_name.as_str()
            },
            if f.sector.is_empty() {
                "Unknown"
            } else {
                f.sector.as_str()
            },
            if f.industry.is_empty() {
                "Unknown"
            } else {
                f.industry.as_str()
            }
        )
        .ok();
        if !f.description.is_empty() {
            writeln!(packet, "{}", truncate_chars(&f.description, 800)).ok();
        }
        writeln!(packet).ok();

        let fmt_money = fundamentals::format_large_number;
        let fmt_opt = |v: Option<f64>| {
            v.map(|x| format!("{x:.2}"))
                .unwrap_or_else(|| "-".to_string())
        };
        let fmt_money_opt = |v: Option<f64>| v.map(fmt_money).unwrap_or_else(|| "-".to_string());

        writeln!(packet, "### Valuation & Risk").ok();
        writeln!(packet, "| Metric | Value |").ok();
        writeln!(packet, "|---|---|").ok();
        writeln!(packet, "| Market Cap | {} |", fmt_money_opt(f.market_cap)).ok();
        writeln!(
            packet,
            "| Enterprise Value | {} |",
            fmt_money_opt(f.enterprise_value)
        )
        .ok();
        writeln!(packet, "| MCap/EV % | {} |", fmt_opt(f.mcap_ev_ratio)).ok();
        writeln!(packet, "| Total Debt | {} |", fmt_money_opt(f.total_debt)).ok();
        writeln!(
            packet,
            "| Cash & Equivalents | {} |",
            fmt_money_opt(f.cash_and_equivalents)
        )
        .ok();
        writeln!(packet, "| Stock Price | {} |", fmt_opt(f.stock_price)).ok();
        writeln!(packet, "| P/E (trailing) | {} |", fmt_opt(f.pe_ratio)).ok();
        writeln!(packet, "| Forward P/E | {} |", fmt_opt(f.forward_pe)).ok();
        writeln!(packet, "| PEG | {} |", fmt_opt(f.peg_ratio)).ok();
        writeln!(packet, "| P/B | {} |", fmt_opt(f.price_to_book)).ok();
        writeln!(packet, "| P/S | {} |", fmt_opt(f.price_to_sales)).ok();
        writeln!(packet, "| EV/EBITDA | {} |", fmt_opt(f.ev_to_ebitda)).ok();
        writeln!(packet, "| Profit Margin | {} |", fmt_opt(f.profit_margin)).ok();
        writeln!(
            packet,
            "| Operating Margin | {} |",
            fmt_opt(f.operating_margin)
        )
        .ok();
        writeln!(packet, "| ROE | {} |", fmt_opt(f.roe)).ok();
        writeln!(packet, "| ROA | {} |", fmt_opt(f.roa)).ok();
        writeln!(packet, "| Beta | {} |", fmt_opt(f.beta)).ok();
        writeln!(packet, "| Short Ratio | {} |", fmt_opt(f.short_ratio)).ok();
        writeln!(
            packet,
            "| Short % of Float | {} |",
            fmt_opt(f.short_percent_of_float)
        )
        .ok();
        writeln!(packet, "| Dividend Yield | {} |", fmt_opt(f.dividend_yield)).ok();
        writeln!(
            packet,
            "| Next Earnings | {} |",
            f.next_earnings_date.clone().unwrap_or_else(|| "-".into())
        )
        .ok();
        writeln!(packet).ok();

        emit_quarterly_financials(conn, symbol, packet);
        emit_institutional_holders(conn, symbol, packet);
        emit_sector_peers(all_fundamentals, &f, symbol, packet);
    }

    fn emit_price_stats(&self, symbol: &str, packet: &mut String) {
        let candidates = self.bar_key_candidates(symbol);
        let mut chosen = None;
        for key in candidates {
            if let Ok(Some(bars)) = self.cache.get_bars_raw(&key) {
                if bars.len() >= 20 {
                    chosen = Some((key, bars));
                    break;
                }
            }
        }

        let Some((key, bars)) = chosen else {
            writeln!(
                packet,
                "_No D1 bar data in cache - price/volatility stats unavailable. Run MT5SYNC or BARDATA to populate._"
            )
            .ok();
            writeln!(packet).ok();
            return;
        };

        let closes: Vec<f64> = bars.iter().map(|(_, _, _, _, c, _)| *c).collect();
        let ohlc: Vec<(f64, f64, f64, f64)> = bars
            .iter()
            .map(|(_, o, h, l, c, _)| (*o, *h, *l, *c))
            .collect();
        let Some(last) = closes.last().copied() else {
            return;
        };
        let n = closes.len();
        let ret_pct = |n_back: usize| -> Option<f64> {
            if n > n_back {
                let prev = closes[n - 1 - n_back];
                if prev > 0.0 {
                    Some((last / prev - 1.0) * 100.0)
                } else {
                    None
                }
            } else {
                None
            }
        };
        let atr = compute_atr(&ohlc, 14);
        let atr_pct = if last > 0.0 { atr / last * 100.0 } else { 0.0 };
        let var95 = var::compute_var_from_closes(&closes, 0.95)
            .map(|(dollars, ratio)| format!("${dollars:.2} ({ratio:.2}% of ask)"))
            .unwrap_or_else(|| "-".to_string());

        writeln!(packet, "### Price & Volatility (D1 bars, n={n})").ok();
        writeln!(packet, "- Source key: `{key}`").ok();
        writeln!(packet, "- Last close: **{last:.4}**").ok();
        writeln!(
            packet,
            "- 20d return: {}",
            ret_pct(20)
                .map(|x| format!("{x:+.2}%"))
                .unwrap_or_else(|| "-".into())
        )
        .ok();
        writeln!(
            packet,
            "- 60d return: {}",
            ret_pct(60)
                .map(|x| format!("{x:+.2}%"))
                .unwrap_or_else(|| "-".into())
        )
        .ok();
        writeln!(
            packet,
            "- 252d return: {}",
            ret_pct(252)
                .map(|x| format!("{x:+.2}%"))
                .unwrap_or_else(|| "-".into())
        )
        .ok();
        writeln!(packet, "- ATR(14): {atr:.4} ({atr_pct:.2}% of price)").ok();
        writeln!(packet, "- VaR 95% (1 lot): {var95}").ok();
        writeln!(packet).ok();
    }

    fn bar_key_candidates(&self, symbol: &str) -> Vec<String> {
        let mut out = vec![
            format!("mt5:{symbol}:1Day"),
            format!("alpaca:{symbol}:1Day"),
            format!("kraken:{symbol}:1Day"),
            format!("kraken-futures:{symbol}:1Day"),
            format!("cryptocompare:{symbol}:1Day"),
            format!("mt5:{symbol}:D1"),
        ];
        let existing: HashSet<String> = out.iter().cloned().collect();
        let needle = format!(":{}:", symbol.to_uppercase());
        if let Ok(rows) = self.cache.detailed_stats() {
            for (key, _, _) in rows {
                let key_upper = key.to_uppercase();
                if key_upper.contains(&needle)
                    && (key_upper.ends_with(":1DAY") || key_upper.ends_with(":D1"))
                    && !existing.contains(&key)
                {
                    out.push(key);
                }
            }
        }
        out
    }

    fn list_cached_symbols(&self, limit: usize) -> Result<Vec<String>, String> {
        let mut symbols = BTreeSet::new();
        for (key, _, _) in self.cache.detailed_stats()? {
            if let Some(symbol) = symbol_from_bar_key(&key) {
                symbols.insert(symbol);
            }
            if symbols.len() >= limit {
                break;
            }
        }
        Ok(symbols.into_iter().collect())
    }
}

fn read_json_rpc_message<R: BufRead>(reader: &mut R) -> io::Result<Option<(Value, Framing)>> {
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            return Ok(None);
        }
        if line.trim().is_empty() {
            continue;
        }

        let lower = line.to_ascii_lowercase();
        if lower.starts_with("content-length:") {
            let len = line
                .split_once(':')
                .and_then(|(_, n)| n.trim().parse::<usize>().ok())
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "bad Content-Length header")
                })?;

            loop {
                line.clear();
                let n = reader.read_line(&mut line)?;
                if n == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "EOF in JSON-RPC headers",
                    ));
                }
                if line.trim().is_empty() {
                    break;
                }
            }

            let mut body = vec![0_u8; len];
            reader.read_exact(&mut body)?;
            let value = serde_json::from_slice(&body)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            return Ok(Some((value, Framing::Header)));
        }

        let value = serde_json::from_str(line.trim_end())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        return Ok(Some((value, Framing::Line)));
    }
}

fn write_json_rpc_message<W: Write>(
    writer: &mut W,
    value: &Value,
    framing: Framing,
) -> io::Result<()> {
    let body = serde_json::to_vec(value).map_err(io::Error::other)?;
    match framing {
        Framing::Line => {
            writer.write_all(&body)?;
            writer.write_all(b"\n")?;
        }
        Framing::Header => {
            write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
            writer.write_all(&body)?;
        }
    }
    writer.flush()
}

fn jsonrpc_result(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn jsonrpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

fn tool_text(text: String) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ],
        "isError": false
    })
}

fn parse_symbols_arg(args: &Value) -> Result<Vec<String>, String> {
    let raw = args
        .get("symbols")
        .or_else(|| args.get("symbol"))
        .ok_or_else(|| "research_packet requires a symbols argument".to_string())?;

    let mut seen = HashSet::new();
    let mut symbols = Vec::new();
    match raw {
        Value::String(s) => {
            for part in s.split(',') {
                if let Some(symbol) = normalize_symbol(part) {
                    if seen.insert(symbol.clone()) {
                        symbols.push(symbol);
                    }
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                if let Some(s) = item.as_str().and_then(normalize_symbol) {
                    if seen.insert(s.clone()) {
                        symbols.push(s);
                    }
                }
            }
        }
        _ => return Err("symbols must be a string or array of strings".to_string()),
    }

    if symbols.is_empty() {
        Err("No valid ticker symbols were provided".to_string())
    } else {
        Ok(symbols)
    }
}

fn normalize_symbol(raw: &str) -> Option<String> {
    let symbol = raw.trim().to_uppercase();
    let tickerish = !symbol.is_empty()
        && symbol.len() <= 32
        && symbol
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '+' | '/'));
    tickerish.then_some(symbol)
}

fn ensure_global_header(packet: &mut String, wrote_header: &mut bool) {
    if !*wrote_header {
        writeln!(packet, "## Global Market Context").ok();
        *wrote_header = true;
    }
}

fn table_exists(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name = ?1 LIMIT 1",
        params![table],
        |_| Ok(()),
    )
    .is_ok()
}

fn latest_json(conn: &Connection, table: &str, column: &str) -> Option<Value> {
    if !table_exists(conn, table) {
        return None;
    }
    let sql = format!(
        "SELECT {} FROM {} ORDER BY updated_at DESC LIMIT 1",
        quote_ident(column),
        quote_ident(table)
    );
    let text: String = conn.query_row(&sql, [], |row| row.get(0)).ok()?;
    serde_json::from_str(&text).ok()
}

fn table_columns(conn: &Connection, table: &str) -> HashSet<String> {
    let mut columns = HashSet::new();
    let sql = format!("PRAGMA table_info({})", quote_ident(table));
    let Ok(mut stmt) = conn.prepare(&sql) else {
        return columns;
    };
    let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(1)) else {
        return columns;
    };
    for column in rows.flatten() {
        columns.insert(column);
    }
    columns
}

fn quote_ident(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
}

fn value_str<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

fn first_nonempty<'a>(value: &'a Value, keys: &[&str]) -> &'a str {
    keys.iter()
        .find_map(|key| {
            let s = value_str(value, key);
            (!s.is_empty()).then_some(s)
        })
        .unwrap_or("")
}

fn value_f64(value: &Value, key: &str) -> f64 {
    value
        .get(key)
        .and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
        })
        .unwrap_or(0.0)
}

fn load_broker_positions(
    cache: &SqliteCache,
    key: &str,
    broker_name: &str,
    symbol: &str,
) -> Vec<String> {
    let Ok(Some(json)) = cache.get_kv(key) else {
        return Vec::new();
    };
    let positions: Vec<broker::PositionInfo> = serde_json::from_str(&json).unwrap_or_default();
    positions
        .iter()
        .filter(|p| p.symbol.eq_ignore_ascii_case(symbol) && p.qty != 0.0)
        .map(|p| {
            let side = if p.side.eq_ignore_ascii_case("short") || p.qty < 0.0 {
                "SHORT"
            } else {
                "LONG"
            };
            let abs_qty = p.qty.abs();
            let current_price = if abs_qty > 0.0 {
                p.market_value.abs() / abs_qty
            } else {
                0.0
            };
            let cost_basis = p.avg_entry_price * abs_qty;
            let unreal_pct = if cost_basis > 0.0 {
                (p.unrealized_pl / cost_basis) * 100.0
            } else {
                0.0
            };
            format!(
                "**{broker_name}** - {side} {abs_qty:.4} @ avg {avg:.4} (current ~{current:.4}); market value {mv:.2}; unrealized {pnl:+.2} ({pct:+.2}%)",
                avg = p.avg_entry_price,
                current = current_price,
                mv = p.market_value,
                pnl = p.unrealized_pl,
                pct = unreal_pct
            )
        })
        .collect()
}

fn load_darwin_positions(cache: &SqliteCache, symbol: &str) -> Vec<String> {
    let Ok(Some(json)) = cache.get_kv("darwin:open_positions") else {
        return Vec::new();
    };
    let positions: Vec<darwin::PortfolioOpenPosition> =
        serde_json::from_str(&json).unwrap_or_default();
    positions
        .iter()
        .filter(|p| p.symbol.eq_ignore_ascii_case(symbol) && p.total_volume != 0.0)
        .map(|p| {
            format!(
                "**DARWIN** - {} {:.4} @ avg {:.4}; notional {:.2}; DARWINs {}",
                p.side,
                p.total_volume,
                p.avg_price,
                p.notional,
                p.darwin_breakdown
                    .iter()
                    .map(|(ticker, volume, avg)| format!("{ticker} {volume:.2}@{avg:.4}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect()
}

fn emit_quarterly_financials(conn: &Connection, symbol: &str, packet: &mut String) {
    let Ok(quarters) = fundamentals::get_quarterly_financials(conn, symbol) else {
        return;
    };
    if quarters.is_empty() {
        return;
    }

    let fmt_money = fundamentals::format_large_number;
    let fmt_mopt = |v: Option<f64>| v.map(fmt_money).unwrap_or_else(|| "-".to_string());
    let fmt_opt = |v: Option<f64>| {
        v.map(|x| format!("{x:.2}"))
            .unwrap_or_else(|| "-".to_string())
    };
    writeln!(
        packet,
        "### Last {} Quarterly Financials",
        quarters.len().min(4)
    )
    .ok();
    writeln!(
        packet,
        "| Period | Revenue | Net Income | FCF | Gross Profit | Op Income | EPS |"
    )
    .ok();
    writeln!(packet, "|---|---|---|---|---|---|---|").ok();
    for q in quarters.iter().take(4) {
        writeln!(
            packet,
            "| {} | {} | {} | {} | {} | {} | {} |",
            q.period_end,
            fmt_mopt(q.total_revenue),
            fmt_mopt(q.net_income),
            fmt_mopt(q.free_cash_flow),
            fmt_mopt(q.gross_profit),
            fmt_mopt(q.operating_income),
            fmt_opt(q.eps)
        )
        .ok();
    }
    writeln!(packet).ok();
}

fn emit_institutional_holders(conn: &Connection, symbol: &str, packet: &mut String) {
    let Ok(holders) = fundamentals::get_institutional_holders(conn, symbol) else {
        return;
    };
    if holders.is_empty() {
        return;
    }

    let fmt_money = fundamentals::format_large_number;
    writeln!(
        packet,
        "### Top {} Institutional Holders",
        holders.len().min(5)
    )
    .ok();
    writeln!(packet, "| Holder | Shares | % Held | Value |").ok();
    writeln!(packet, "|---|---|---|---|").ok();
    for h in holders.iter().take(5) {
        writeln!(
            packet,
            "| {} | {} | {:.2}% | {} |",
            h.holder_name,
            fmt_money(h.shares as f64),
            h.pct_held * 100.0,
            fmt_money(h.value)
        )
        .ok();
    }
    writeln!(packet).ok();
}

fn emit_sector_peers(
    all_fundamentals: &[fundamentals::Fundamentals],
    fund: &fundamentals::Fundamentals,
    symbol: &str,
    packet: &mut String,
) {
    if fund.sector.is_empty() {
        return;
    }
    let peers: Vec<_> = all_fundamentals
        .iter()
        .filter(|o| {
            o.sector.eq_ignore_ascii_case(&fund.sector) && !o.symbol.eq_ignore_ascii_case(symbol)
        })
        .collect();
    if peers.len() < 3 {
        return;
    }

    fn median(mut values: Vec<f64>) -> Option<f64> {
        if values.is_empty() {
            return None;
        }
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Some(values[values.len() / 2])
    }

    let collect = |getter: fn(&fundamentals::Fundamentals) -> Option<f64>| -> Vec<f64> {
        peers.iter().filter_map(|p| getter(p)).collect()
    };
    let fmt_o = |v: Option<f64>| v.map(|x| format!("{x:.2}")).unwrap_or_else(|| "-".into());

    writeln!(
        packet,
        "### Sector Peer Comparison ({} - {} peers)",
        fund.sector,
        peers.len()
    )
    .ok();
    writeln!(packet, "| Metric | This Symbol | Sector Median |").ok();
    writeln!(packet, "|---|---|---|").ok();
    writeln!(
        packet,
        "| P/E | {} | {} |",
        fmt_o(fund.pe_ratio),
        fmt_o(median(collect(|x| x.pe_ratio)))
    )
    .ok();
    writeln!(
        packet,
        "| Forward P/E | {} | {} |",
        fmt_o(fund.forward_pe),
        fmt_o(median(collect(|x| x.forward_pe)))
    )
    .ok();
    writeln!(
        packet,
        "| P/B | {} | {} |",
        fmt_o(fund.price_to_book),
        fmt_o(median(collect(|x| x.price_to_book)))
    )
    .ok();
    writeln!(
        packet,
        "| P/S | {} | {} |",
        fmt_o(fund.price_to_sales),
        fmt_o(median(collect(|x| x.price_to_sales)))
    )
    .ok();
    writeln!(
        packet,
        "| EV/EBITDA | {} | {} |",
        fmt_o(fund.ev_to_ebitda),
        fmt_o(median(collect(|x| x.ev_to_ebitda)))
    )
    .ok();
    writeln!(
        packet,
        "| Profit Margin | {} | {} |",
        fmt_o(fund.profit_margin),
        fmt_o(median(collect(|x| x.profit_margin)))
    )
    .ok();
    writeln!(
        packet,
        "| ROE | {} | {} |",
        fmt_o(fund.roe),
        fmt_o(median(collect(|x| x.roe)))
    )
    .ok();
    writeln!(
        packet,
        "| Beta | {} | {} |",
        fmt_o(fund.beta),
        fmt_o(median(collect(|x| x.beta)))
    )
    .ok();
    writeln!(
        packet,
        "| Short % Float | {} | {} |",
        fmt_o(fund.short_percent_of_float),
        fmt_o(median(collect(|x| x.short_percent_of_float)))
    )
    .ok();
    writeln!(
        packet,
        "| Div Yield | {} | {} |",
        fmt_o(fund.dividend_yield),
        fmt_o(median(collect(|x| x.dividend_yield)))
    )
    .ok();
    writeln!(packet).ok();
}

fn emit_filings(symbol: &str, filings: &[sec_filing::SecFiling], packet: &mut String) {
    let recent: Vec<_> = filings
        .iter()
        .filter(|f| f.ticker.eq_ignore_ascii_case(symbol))
        .take(10)
        .collect();
    if recent.is_empty() {
        return;
    }

    writeln!(packet, "### Recent SEC Filings ({})", recent.len()).ok();
    writeln!(packet, "| Date | Form | Category | Summary |").ok();
    writeln!(packet, "|---|---|---|---|").ok();
    for filing in recent {
        writeln!(
            packet,
            "| {} | {} | {} | {} |",
            filing.filing_date,
            filing.form_type,
            filing.category,
            markdown_table_text(&truncate_chars(&filing.summary, 120))
        )
        .ok();
    }
    writeln!(packet).ok();
}

fn emit_insider_trades(symbol: &str, trades: &[sec_filing::InsiderTrade], packet: &mut String) {
    let rows: Vec<_> = trades
        .iter()
        .filter(|t| t.ticker.eq_ignore_ascii_case(symbol))
        .collect();
    if rows.is_empty() {
        return;
    }

    let n_buys = rows
        .iter()
        .filter(|t| {
            t.transaction_type.eq_ignore_ascii_case("P")
                || t.transaction_type.to_lowercase().contains("buy")
        })
        .count();
    let n_sells = rows
        .iter()
        .filter(|t| {
            t.transaction_type.eq_ignore_ascii_case("S")
                || t.transaction_type.to_lowercase().contains("sell")
        })
        .count();
    let buy_value: f64 = rows
        .iter()
        .filter(|t| t.transaction_type.eq_ignore_ascii_case("P"))
        .map(|t| t.aggregate_value)
        .sum();
    let sell_value: f64 = rows
        .iter()
        .filter(|t| t.transaction_type.eq_ignore_ascii_case("S"))
        .map(|t| t.aggregate_value)
        .sum();
    let fmt_money = fundamentals::format_large_number;

    writeln!(packet, "### Insider Activity").ok();
    writeln!(
        packet,
        "- {} transactions on file ({} buys, {} sells)",
        rows.len(),
        n_buys,
        n_sells
    )
    .ok();
    writeln!(
        packet,
        "- Buy aggregate: {} | Sell aggregate: {} | Net: {}",
        fmt_money(buy_value),
        fmt_money(sell_value),
        fmt_money(buy_value - sell_value)
    )
    .ok();
    writeln!(packet, "| Date | Insider | Title | Type | Shares | Value |").ok();
    writeln!(packet, "|---|---|---|---|---|---|").ok();
    for t in rows.iter().take(5) {
        writeln!(
            packet,
            "| {} | {} | {} | {} | {} | {} |",
            t.transaction_date,
            markdown_table_text(&t.insider_name),
            markdown_table_text(&t.insider_title),
            t.transaction_type,
            fmt_money(t.shares),
            fmt_money(t.aggregate_value)
        )
        .ok();
    }
    writeln!(packet).ok();
}

fn emit_recent_news(conn: &Connection, symbol: &str, packet: &mut String) {
    if !table_exists(conn, "research_news") {
        return;
    }
    let Ok(mut stmt) = conn.prepare(
        "SELECT source, provider, headline, published_at, sentiment, url
         FROM research_news
         WHERE symbol = ?1 OR tickers_json LIKE ?2
         ORDER BY published_at DESC, updated_at DESC
         LIMIT 8",
    ) else {
        return;
    };
    let like = format!("%{}%", symbol.to_uppercase());
    let Ok(rows) = stmt.query_map(params![symbol.to_uppercase(), like], |row| {
        Ok((
            row.get::<_, String>(0).unwrap_or_default(),
            row.get::<_, String>(1).unwrap_or_default(),
            row.get::<_, String>(2).unwrap_or_default(),
            row.get::<_, i64>(3).unwrap_or_default(),
            row.get::<_, String>(4).unwrap_or_default(),
            row.get::<_, String>(5).unwrap_or_default(),
        ))
    }) else {
        return;
    };
    let rows: Vec<_> = rows.flatten().collect();
    if rows.is_empty() {
        return;
    }

    writeln!(packet, "### Recent News ({})", rows.len()).ok();
    writeln!(packet, "| Date | Source | Sentiment | Headline |").ok();
    writeln!(packet, "|---|---|---|---|").ok();
    for (source, provider, headline, published_at, sentiment, url) in rows {
        let date = format_unix_date(published_at);
        let source = if provider.is_empty() {
            source
        } else {
            provider
        };
        let title = if url.is_empty() {
            truncate_chars(&headline, 120)
        } else {
            format!("[{}]({})", truncate_chars(&headline, 100), url)
        };
        writeln!(
            packet,
            "| {} | {} | {} | {} |",
            date,
            markdown_table_text(&source),
            if sentiment.is_empty() {
                "-".to_string()
            } else {
                markdown_table_text(&sentiment)
            },
            markdown_table_text(&title)
        )
        .ok();
    }
    writeln!(packet).ok();
}

fn emit_ingested_web_articles(conn: &Connection, symbol: &str, packet: &mut String) {
    let Some(snapshot) = symbol_snapshot_json(conn, "research_web_articles", symbol) else {
        return;
    };
    let articles = snapshot
        .get("articles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if articles.is_empty() {
        return;
    }

    writeln!(packet, "### Agent-Ingested Web Research").ok();
    writeln!(packet, "| Published | Source | Agent | Title | Summary |").ok();
    writeln!(packet, "|---|---|---|---|---|").ok();
    for article in articles.iter().take(10) {
        let title = first_nonempty(article, &["title"]);
        let url = value_str(article, "url");
        let linked = if url.is_empty() {
            markdown_table_text(title)
        } else {
            markdown_table_text(&format!("[{}]({})", truncate_chars(title, 80), url))
        };
        writeln!(
            packet,
            "| {} | {} | {} | {} | {} |",
            markdown_table_text(value_str(article, "published_at")),
            markdown_table_text(value_str(article, "source")),
            markdown_table_text(first_nonempty(article, &["agent_used", "agent"])),
            linked,
            markdown_table_text(&truncate_chars(value_str(article, "summary"), 180))
        )
        .ok();
    }
    writeln!(packet).ok();
}

fn emit_research_surfaces(
    conn: &Connection,
    symbol: &str,
    options: &ResearchPacketOptions,
    packet: &mut String,
) {
    let surfaces = query_research_surfaces(conn, symbol, options.max_surfaces);
    if surfaces.is_empty() {
        return;
    }

    writeln!(
        packet,
        "### Cached Research Surfaces ({} tables)",
        surfaces.len()
    )
    .ok();
    writeln!(
        packet,
        "These are raw cached `research_*` snapshots for the symbol. They keep the MCP packet aligned with TyphooN's expanding research surface without requiring every table to have a hand-written renderer."
    )
    .ok();
    writeln!(packet).ok();

    for surface in surfaces {
        writeln!(packet, "#### {}", surface_name(&surface.table)).ok();
        if surface.updated_at > 0 {
            writeln!(
                packet,
                "Updated: {}",
                format_unix_datetime(surface.updated_at)
            )
            .ok();
        }
        writeln!(packet).ok();
        writeln!(packet, "```json").ok();
        writeln!(
            packet,
            "{}",
            compact_json_for_packet(&surface.snapshot_json, options.max_snapshot_chars)
        )
        .ok();
        writeln!(packet, "```").ok();
        writeln!(packet).ok();
    }
}

fn query_research_surfaces(
    conn: &Connection,
    symbol: &str,
    max_surfaces: usize,
) -> Vec<ResearchSurface> {
    let Ok(mut stmt) = conn.prepare(
        "SELECT name FROM sqlite_master
         WHERE type='table' AND name LIKE 'research_%'
         ORDER BY name",
    ) else {
        return Vec::new();
    };
    let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) else {
        return Vec::new();
    };

    let mut surfaces = Vec::new();
    for table in rows.flatten() {
        let columns = table_columns(conn, &table);
        if !(columns.contains("symbol") && columns.contains("snapshot_json")) {
            continue;
        }
        let updated_expr = if columns.contains("updated_at") {
            "updated_at"
        } else {
            "0"
        };
        let sql = format!(
            "SELECT snapshot_json, {updated_expr} FROM {} WHERE UPPER(symbol) = UPPER(?1) LIMIT 1",
            quote_ident(&table)
        );
        let row = conn.query_row(&sql, params![symbol], |row| {
            Ok(ResearchSurface {
                table: table.clone(),
                snapshot_json: row.get::<_, String>(0).unwrap_or_default(),
                updated_at: row.get::<_, i64>(1).unwrap_or_default(),
            })
        });
        if let Ok(surface) = row {
            if !surface.snapshot_json.trim().is_empty() && surface.snapshot_json.trim() != "{}" {
                surfaces.push(surface);
                if max_surfaces > 0 && surfaces.len() >= max_surfaces {
                    break;
                }
            }
        }
    }
    surfaces
}

fn symbol_snapshot_json(conn: &Connection, table: &str, symbol: &str) -> Option<Value> {
    if !table_exists(conn, table) {
        return None;
    }
    let columns = table_columns(conn, table);
    if !(columns.contains("symbol") && columns.contains("snapshot_json")) {
        return None;
    }
    let sql = format!(
        "SELECT snapshot_json FROM {} WHERE UPPER(symbol) = UPPER(?1) LIMIT 1",
        quote_ident(table)
    );
    let text: String = conn
        .query_row(&sql, params![symbol], |row| row.get(0))
        .ok()?;
    serde_json::from_str(&text).ok()
}

fn compact_json_for_packet(text: &str, max_chars: usize) -> String {
    let compact = serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|v| serde_json::to_string(&v).ok())
        .unwrap_or_else(|| text.trim().to_string());
    if max_chars == 0 {
        compact
    } else {
        truncate_chars(&compact, max_chars)
    }
}

fn compute_atr(ohlc: &[(f64, f64, f64, f64)], period: usize) -> f64 {
    if ohlc.len() <= period || period == 0 {
        return 0.0;
    }
    let mut atr = 0.0;
    for i in 1..=period {
        let tr = (ohlc[i].1 - ohlc[i].2)
            .max((ohlc[i].1 - ohlc[i - 1].3).abs())
            .max((ohlc[i].2 - ohlc[i - 1].3).abs());
        atr += tr;
    }
    atr /= period as f64;
    for i in (period + 1)..ohlc.len() {
        let tr = (ohlc[i].1 - ohlc[i].2)
            .max((ohlc[i].1 - ohlc[i - 1].3).abs())
            .max((ohlc[i].2 - ohlc[i - 1].3).abs());
        atr = (atr * (period as f64 - 1.0) + tr) / period as f64;
    }
    atr
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return text.to_string();
    }
    let mut out = String::new();
    for (idx, ch) in text.chars().enumerate() {
        if idx >= max_chars {
            out.push_str("... [truncated]");
            return out;
        }
        out.push(ch);
    }
    out
}

fn markdown_table_text(text: &str) -> String {
    text.replace('|', "\\|")
        .replace('\n', " ")
        .replace('\r', " ")
}

fn format_unix_date(ts: i64) -> String {
    chrono::DateTime::<Utc>::from_timestamp(ts, 0)
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn format_unix_datetime(ts: i64) -> String {
    chrono::DateTime::<Utc>::from_timestamp(ts, 0)
        .map(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn surface_name(table: &str) -> String {
    table
        .strip_prefix("research_")
        .unwrap_or(table)
        .replace('_', " ")
        .to_uppercase()
}

fn symbol_from_bar_key(key: &str) -> Option<String> {
    let parts: Vec<&str> = key.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let symbol = parts[1];
    normalize_symbol(symbol)
}

fn emit_return_path(packet: &mut String) {
    writeln!(packet).ok();
    writeln!(packet, "---").ok();
    writeln!(packet, "## Return Path - Web Research Ingest").ok();
    writeln!(packet).ok();
    writeln!(
        packet,
        "If you consulted any web sources to answer the above, emit a fenced ingest block at the end of your reply so TyphooN-Terminal can cache and share your findings with LAN peers:"
    )
    .ok();
    writeln!(packet).ok();
    writeln!(packet, "```").ok();
    writeln!(packet, "===TYPHOON_INGEST===").ok();
    writeln!(packet, "[").ok();
    writeln!(
        packet,
        "  {{\"symbol\": \"TICKER\", \"title\": \"article headline\", \"url\": \"https://...\","
    )
    .ok();
    writeln!(
        packet,
        "   \"source\": \"Reuters|Bloomberg|WSJ|...\", \"published_at\": \"YYYY-MM-DD\","
    )
    .ok();
    writeln!(
        packet,
        "   \"summary\": \"2-3 sentence takeaway\", \"agent\": \"claude|gemini|chatgpt|...\"}}"
    )
    .ok();
    writeln!(packet, "]").ok();
    writeln!(packet, "===END_INGEST===").ok();
    writeln!(packet, "```").ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_symbols_accepts_string_and_dedupes() {
        let args = json!({"symbols": "aapl, MSFT, aapl, BTC/USD"});
        let symbols = parse_symbols_arg(&args).unwrap();
        assert_eq!(symbols, vec!["AAPL", "MSFT", "BTC/USD"]);
    }

    #[test]
    fn parse_symbols_accepts_array() {
        let args = json!({"symbols": ["cc", "NCLH", "bad symbol"]});
        let symbols = parse_symbols_arg(&args).unwrap();
        assert_eq!(symbols, vec!["CC", "NCLH"]);
    }

    #[test]
    fn compact_json_truncates_on_char_boundary() {
        let compact = compact_json_for_packet(r#"{"symbol":"AAPL","note":"abc"}"#, 18);
        assert!(compact.starts_with('{'));
        assert!(compact.ends_with("[truncated]"));
    }
}
