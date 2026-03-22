//! DARWIN trade history import — parses MT5 XLSX exports and stores in SQLite.
//!
//! Supports importing closed positions, orders, and deals from MT5's
//! "Trade History Report" XLSX format. Each DARWIN is stored as a named
//! virtual account with full trade history for analytics.

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

// ── Data Types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarwinAccount {
    pub name: String,           // MT5 name (e.g. "TyphooN_MT5")
    pub darwin_ticker: String,  // 4-letter DARWIN ticker (e.g. "XUQF")
    pub mt5_account: String,    // MT5 account number
    pub initial_balance: f64,
    pub created_at: i64,        // import timestamp
    pub deal_count: i64,
    pub position_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarwinDeal {
    pub id: i64,
    pub account: String,        // darwin_ticker
    pub time: String,           // "2024.10.08 16:47:19"
    pub deal_ticket: i64,
    pub symbol: String,
    pub deal_type: String,      // "buy", "sell", "balance"
    pub direction: String,      // "in", "out", ""
    pub volume: f64,
    pub price: f64,
    pub order_ticket: i64,
    pub commission: f64,
    pub fee: f64,
    pub swap: f64,
    pub profit: f64,
    pub balance: f64,
    pub comment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarwinPosition {
    pub id: i64,
    pub account: String,
    pub open_time: String,
    pub position_ticket: i64,
    pub symbol: String,
    pub pos_type: String,       // "buy", "sell"
    pub volume: f64,
    pub open_price: f64,
    pub sl: f64,
    pub tp: f64,
    pub close_time: String,
    pub close_price: f64,
    pub commission: f64,
    pub swap: f64,
    pub profit: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarwinAccountSummary {
    pub account: DarwinAccount,
    pub total_profit: f64,
    pub total_commission: f64,
    pub total_swap: f64,
    pub win_count: i64,
    pub loss_count: i64,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub final_balance: f64,
    pub max_drawdown_pct: f64,
    pub symbols_traded: Vec<String>,
}

// ── SQLite Schema ───────────────────────────────────────────────────

pub fn create_darwin_tables(conn: &Connection) -> Result<(), String> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS darwin_accounts (
            darwin_ticker TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            mt5_account TEXT NOT NULL,
            initial_balance REAL NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            deal_count INTEGER NOT NULL DEFAULT 0,
            position_count INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS darwin_deals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            account TEXT NOT NULL,
            time TEXT NOT NULL,
            deal_ticket INTEGER NOT NULL,
            symbol TEXT NOT NULL DEFAULT '',
            deal_type TEXT NOT NULL,
            direction TEXT NOT NULL DEFAULT '',
            volume REAL NOT NULL DEFAULT 0,
            price REAL NOT NULL DEFAULT 0,
            order_ticket INTEGER NOT NULL DEFAULT 0,
            commission REAL NOT NULL DEFAULT 0,
            fee REAL NOT NULL DEFAULT 0,
            swap REAL NOT NULL DEFAULT 0,
            profit REAL NOT NULL DEFAULT 0,
            balance REAL NOT NULL DEFAULT 0,
            comment TEXT NOT NULL DEFAULT '',
            FOREIGN KEY (account) REFERENCES darwin_accounts(darwin_ticker)
        );
        CREATE TABLE IF NOT EXISTS darwin_positions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            account TEXT NOT NULL,
            open_time TEXT NOT NULL,
            position_ticket INTEGER NOT NULL,
            symbol TEXT NOT NULL,
            pos_type TEXT NOT NULL,
            volume REAL NOT NULL DEFAULT 0,
            open_price REAL NOT NULL DEFAULT 0,
            sl REAL NOT NULL DEFAULT 0,
            tp REAL NOT NULL DEFAULT 0,
            close_time TEXT NOT NULL DEFAULT '',
            close_price REAL NOT NULL DEFAULT 0,
            commission REAL NOT NULL DEFAULT 0,
            swap REAL NOT NULL DEFAULT 0,
            profit REAL NOT NULL DEFAULT 0,
            FOREIGN KEY (account) REFERENCES darwin_accounts(darwin_ticker)
        );
        CREATE INDEX IF NOT EXISTS idx_darwin_deals_account ON darwin_deals(account);
        CREATE INDEX IF NOT EXISTS idx_darwin_deals_time ON darwin_deals(account, time);
        CREATE INDEX IF NOT EXISTS idx_darwin_deals_symbol ON darwin_deals(account, symbol);
        CREATE INDEX IF NOT EXISTS idx_darwin_positions_account ON darwin_positions(account);
        CREATE INDEX IF NOT EXISTS idx_darwin_positions_symbol ON darwin_positions(account, symbol);
        CREATE INDEX IF NOT EXISTS idx_darwin_positions_time ON darwin_positions(account, open_time);
    ").map_err(|e| format!("Create darwin tables failed: {e}"))?;
    Ok(())
}

// ── XLSX Parsing ────────────────────────────────────────────────────

/// Parse volume string from MT5: "1K" → 1000.0, "262" → 262.0, "1K / 1K" → 1000.0
fn parse_volume(val: &calamine::Data) -> f64 {
    match val {
        calamine::Data::Float(f) => *f,
        calamine::Data::Int(i) => *i as f64,
        calamine::Data::String(s) => {
            let s = s.split('/').next().unwrap_or(s).trim();
            if s.ends_with('K') {
                s.trim_end_matches('K').parse::<f64>().unwrap_or(0.0) * 1000.0
            } else if s.ends_with('M') {
                s.trim_end_matches('M').parse::<f64>().unwrap_or(0.0) * 1_000_000.0
            } else {
                s.parse::<f64>().unwrap_or(0.0)
            }
        }
        _ => 0.0,
    }
}

fn cell_f64(val: &calamine::Data) -> f64 {
    match val {
        calamine::Data::Float(f) => *f,
        calamine::Data::Int(i) => *i as f64,
        calamine::Data::String(s) => s.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn cell_i64(val: &calamine::Data) -> i64 {
    match val {
        calamine::Data::Float(f) => *f as i64,
        calamine::Data::Int(i) => *i,
        calamine::Data::String(s) => s.parse::<i64>().unwrap_or(0),
        _ => 0,
    }
}

fn cell_str(val: &calamine::Data) -> String {
    match val {
        calamine::Data::String(s) => s.clone(),
        calamine::Data::Float(f) => format!("{f}"),
        calamine::Data::Int(i) => format!("{i}"),
        calamine::Data::DateTime(dt) => format!("{dt}"),
        _ => String::new(),
    }
}

/// Import a single DARWIN's MT5 XLSX trade history into SQLite.
/// Returns (darwin_ticker, deal_count, position_count).
pub fn import_darwin_xlsx(
    conn: &Connection,
    xlsx_path: &str,
    darwin_ticker: &str,
) -> Result<(String, usize, usize), String> {
    use calamine::{Reader, open_workbook, Xlsx};

    let mut workbook: Xlsx<_> = open_workbook(xlsx_path)
        .map_err(|e| format!("Failed to open XLSX: {e}"))?;

    let sheet_name = workbook.sheet_names().first()
        .ok_or("No sheets in workbook")?.clone();
    let range = workbook.worksheet_range(&sheet_name)
        .map_err(|e| format!("Failed to read sheet: {e}"))?;

    let rows: Vec<Vec<calamine::Data>> = range.rows()
        .map(|r| r.to_vec())
        .collect();

    // Parse header: Name (row 1, col 3), Account (row 2, col 3)
    let mt5_name = if rows.len() > 1 && rows[1].len() > 3 {
        cell_str(&rows[1][3])
    } else {
        darwin_ticker.to_string()
    };

    let mt5_account = if rows.len() > 2 && rows[2].len() > 3 {
        cell_str(&rows[2][3])
    } else {
        String::new()
    };

    // Find section boundaries
    let mut positions_start = 0;
    let mut orders_start = 0;
    let mut deals_start = 0;
    for (i, row) in rows.iter().enumerate() {
        if row.is_empty() { continue; }
        let first = cell_str(&row[0]);
        match first.as_str() {
            "Positions" => positions_start = i,
            "Orders" => orders_start = i,
            "Deals" => deals_start = i,
            _ => {}
        }
    }

    // Rollback any leftover transaction from a previous failed import
    let _ = conn.execute_batch("ROLLBACK");

    // Disable FK checks during import (we control the data integrity)
    conn.execute_batch("PRAGMA foreign_keys=OFF").map_err(|e| format!("FK off failed: {e}"))?;

    // Single transaction for the entire import
    conn.execute_batch("BEGIN").map_err(|e| format!("BEGIN failed: {e}"))?;

    // Delete existing data for this DARWIN (re-import)
    conn.execute("DELETE FROM darwin_deals WHERE account = ?1", params![darwin_ticker])
        .map_err(|e| format!("Delete deals failed: {e}"))?;
    conn.execute("DELETE FROM darwin_positions WHERE account = ?1", params![darwin_ticker])
        .map_err(|e| format!("Delete positions failed: {e}"))?;
    conn.execute("DELETE FROM darwin_accounts WHERE darwin_ticker = ?1", params![darwin_ticker])
        .map_err(|e| format!("Delete account failed: {e}"))?;

    // Parse Positions section (row positions_start+2 to orders_start-1)
    // Header: Time, Position, Symbol, Type, Volume, Price, S/L, T/P, Time, Price, Commission, Swap, Profit
    let mut position_count = 0;
    if positions_start > 0 && orders_start > positions_start {
        for i in (positions_start + 2)..orders_start {
            let row = &rows[i];
            if row.len() < 13 { continue; }
            let open_time = cell_str(&row[0]);
            if open_time.is_empty() || open_time == "Time" { continue; }

            conn.execute(
                "INSERT INTO darwin_positions (account, open_time, position_ticket, symbol, pos_type, volume, open_price, sl, tp, close_time, close_price, commission, swap, profit)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    darwin_ticker,
                    open_time,
                    cell_i64(&row[1]),
                    cell_str(&row[2]),
                    cell_str(&row[3]),
                    parse_volume(&row[4]),
                    cell_f64(&row[5]),
                    cell_f64(&row[6]),
                    cell_f64(&row[7]),
                    cell_str(&row[8]),
                    cell_f64(&row[9]),
                    cell_f64(&row[10]),
                    cell_f64(&row[11]),
                    cell_f64(&row[12]),
                ],
            ).map_err(|e| format!("Insert position failed: {e}"))?;
            position_count += 1;
        }
    }

    // Parse Deals section (row deals_start+2 to end)
    // Header: Time, Deal, Symbol, Type, Direction, Volume, Price, Order, Commission, Fee, Swap, Profit, Balance, Comment
    let mut deal_count = 0;
    let mut initial_balance = 0.0f64;
    if deals_start > 0 {
        for i in (deals_start + 2)..rows.len() {
            let row = &rows[i];
            if row.len() < 13 { continue; }
            let time = cell_str(&row[0]);
            if time.is_empty() || time == "Time" { continue; }
            // Skip summary rows at the bottom
            let deal_type = cell_str(&row[3]);
            if deal_type.is_empty() { continue; }

            let profit = cell_f64(&row[11]);
            let balance = cell_f64(&row[12]);
            let comment = if row.len() > 13 { cell_str(&row[13]) } else { String::new() };

            // First balance entry is the initial deposit
            if deal_type == "balance" && initial_balance == 0.0 {
                initial_balance = profit;
            }

            conn.execute(
                "INSERT INTO darwin_deals (account, time, deal_ticket, symbol, deal_type, direction, volume, price, order_ticket, commission, fee, swap, profit, balance, comment)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    darwin_ticker,
                    time,
                    cell_i64(&row[1]),
                    cell_str(&row[2]),
                    deal_type,
                    if row.len() > 4 { cell_str(&row[4]) } else { String::new() },
                    if row.len() > 5 { parse_volume(&row[5]) } else { 0.0 },
                    if row.len() > 6 { cell_f64(&row[6]) } else { 0.0 },
                    if row.len() > 7 { cell_i64(&row[7]) } else { 0 },
                    if row.len() > 8 { cell_f64(&row[8]) } else { 0.0 },
                    if row.len() > 9 { cell_f64(&row[9]) } else { 0.0 },
                    if row.len() > 10 { cell_f64(&row[10]) } else { 0.0 },
                    profit,
                    balance,
                    comment,
                ],
            ).map_err(|e| format!("Insert deal failed: {e}"))?;
            deal_count += 1;
        }
    }

    // Upsert account record
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT OR REPLACE INTO darwin_accounts (darwin_ticker, name, mt5_account, initial_balance, created_at, deal_count, position_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![darwin_ticker, mt5_name, mt5_account, initial_balance, now, deal_count as i64, position_count as i64],
    ).map_err(|e| format!("Upsert account failed: {e}"))?;

    conn.execute_batch("COMMIT").map_err(|e| format!("COMMIT failed: {e}"))?;
    conn.execute_batch("PRAGMA foreign_keys=ON").ok();

    Ok((darwin_ticker.to_string(), deal_count, position_count))
}

/// List all imported DARWIN accounts.
pub fn list_darwin_accounts(conn: &Connection) -> Result<Vec<DarwinAccount>, String> {
    let mut stmt = conn.prepare(
        "SELECT darwin_ticker, name, mt5_account, initial_balance, created_at, deal_count, position_count FROM darwin_accounts ORDER BY name"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let rows = stmt.query_map([], |row| {
        Ok(DarwinAccount {
            darwin_ticker: row.get(0)?,
            name: row.get(1)?,
            mt5_account: row.get(2)?,
            initial_balance: row.get(3)?,
            created_at: row.get(4)?,
            deal_count: row.get(5)?,
            position_count: row.get(6)?,
        })
    }).map_err(|e| format!("Query failed: {e}"))?;

    let mut accounts = Vec::new();
    for row in rows {
        if let Ok(a) = row { accounts.push(a); }
    }
    Ok(accounts)
}

/// Get full account summary with computed stats.
pub fn get_darwin_summary(conn: &Connection, darwin_ticker: &str) -> Result<DarwinAccountSummary, String> {
    // Get account
    let account: DarwinAccount = conn.query_row(
        "SELECT darwin_ticker, name, mt5_account, initial_balance, created_at, deal_count, position_count FROM darwin_accounts WHERE darwin_ticker = ?1",
        params![darwin_ticker],
        |row| Ok(DarwinAccount {
            darwin_ticker: row.get(0)?,
            name: row.get(1)?,
            mt5_account: row.get(2)?,
            initial_balance: row.get(3)?,
            created_at: row.get(4)?,
            deal_count: row.get(5)?,
            position_count: row.get(6)?,
        })
    ).map_err(|e| format!("Account not found: {e}"))?;

    // Compute stats from positions (closed trades with P/L)
    let mut stmt = conn.prepare(
        "SELECT profit, commission, swap, symbol FROM darwin_positions WHERE account = ?1"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let mut total_profit = 0.0f64;
    let mut total_commission = 0.0f64;
    let mut total_swap = 0.0f64;
    let mut win_count = 0i64;
    let mut loss_count = 0i64;
    let mut gross_wins = 0.0f64;
    let mut gross_losses = 0.0f64;
    let mut symbols = std::collections::HashSet::new();

    let rows = stmt.query_map(params![darwin_ticker], |row| {
        Ok((row.get::<_, f64>(0)?, row.get::<_, f64>(1)?, row.get::<_, f64>(2)?, row.get::<_, String>(3)?))
    }).map_err(|e| format!("Query failed: {e}"))?;

    for row in rows {
        if let Ok((profit, commission, swap, symbol)) = row {
            let net = profit + commission + swap;
            total_profit += profit;
            total_commission += commission;
            total_swap += swap;
            if net > 0.0 {
                win_count += 1;
                gross_wins += net;
            } else if net < 0.0 {
                loss_count += 1;
                gross_losses += net.abs();
            }
            if !symbol.is_empty() {
                symbols.insert(symbol);
            }
        }
    }

    let total_trades = win_count + loss_count;
    let win_rate = if total_trades > 0 { win_count as f64 / total_trades as f64 * 100.0 } else { 0.0 };
    let profit_factor = if gross_losses > 0.0 { gross_wins / gross_losses } else { if gross_wins > 0.0 { f64::INFINITY } else { 0.0 } };

    // Final balance from last deal
    let final_balance: f64 = conn.query_row(
        "SELECT balance FROM darwin_deals WHERE account = ?1 AND balance > 0 ORDER BY time DESC, id DESC LIMIT 1",
        params![darwin_ticker],
        |row| row.get(0),
    ).unwrap_or(account.initial_balance);

    // Max drawdown from deal balance series
    let mut dd_stmt = conn.prepare(
        "SELECT balance FROM darwin_deals WHERE account = ?1 AND balance > 0 ORDER BY time, id"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let balances: Vec<f64> = dd_stmt.query_map(params![darwin_ticker], |row| row.get(0))
        .map_err(|e| format!("Query failed: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    let mut peak = 0.0f64;
    let mut max_dd_pct = 0.0f64;
    for b in &balances {
        if *b > peak { peak = *b; }
        if peak > 0.0 {
            let dd = (peak - b) / peak * 100.0;
            if dd > max_dd_pct { max_dd_pct = dd; }
        }
    }

    let mut syms: Vec<String> = symbols.into_iter().collect();
    syms.sort();

    Ok(DarwinAccountSummary {
        account,
        total_profit,
        total_commission,
        total_swap,
        win_count,
        loss_count,
        win_rate,
        profit_factor,
        final_balance,
        max_drawdown_pct: max_dd_pct,
        symbols_traded: syms,
    })
}

/// Get deals for a DARWIN account, with optional symbol filter and limit.
pub fn get_darwin_deals(
    conn: &Connection,
    darwin_ticker: &str,
    symbol: Option<&str>,
    limit: Option<u32>,
) -> Result<Vec<DarwinDeal>, String> {
    let limit = limit.unwrap_or(10000);
    let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(sym) = symbol {
        (
            "SELECT id, account, time, deal_ticket, symbol, deal_type, direction, volume, price, order_ticket, commission, fee, swap, profit, balance, comment FROM darwin_deals WHERE account = ?1 AND symbol = ?2 ORDER BY time, id LIMIT ?3".to_string(),
            vec![Box::new(darwin_ticker.to_string()), Box::new(sym.to_string()), Box::new(limit)],
        )
    } else {
        (
            "SELECT id, account, time, deal_ticket, symbol, deal_type, direction, volume, price, order_ticket, commission, fee, swap, profit, balance, comment FROM darwin_deals WHERE account = ?1 ORDER BY time, id LIMIT ?2".to_string(),
            vec![Box::new(darwin_ticker.to_string()), Box::new(limit)],
        )
    };

    let mut stmt = conn.prepare(&sql).map_err(|e| format!("Prepare failed: {e}"))?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(params_refs.as_slice(), |row| {
        Ok(DarwinDeal {
            id: row.get(0)?,
            account: row.get(1)?,
            time: row.get(2)?,
            deal_ticket: row.get(3)?,
            symbol: row.get(4)?,
            deal_type: row.get(5)?,
            direction: row.get(6)?,
            volume: row.get(7)?,
            price: row.get(8)?,
            order_ticket: row.get(9)?,
            commission: row.get(10)?,
            fee: row.get(11)?,
            swap: row.get(12)?,
            profit: row.get(13)?,
            balance: row.get(14)?,
            comment: row.get(15)?,
        })
    }).map_err(|e| format!("Query failed: {e}"))?;

    let mut deals = Vec::new();
    for row in rows {
        if let Ok(d) = row { deals.push(d); }
    }
    Ok(deals)
}

/// Get positions for a DARWIN account, with optional symbol filter.
pub fn get_darwin_positions(
    conn: &Connection,
    darwin_ticker: &str,
    symbol: Option<&str>,
    limit: Option<u32>,
) -> Result<Vec<DarwinPosition>, String> {
    let limit = limit.unwrap_or(10000);
    let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(sym) = symbol {
        (
            "SELECT id, account, open_time, position_ticket, symbol, pos_type, volume, open_price, sl, tp, close_time, close_price, commission, swap, profit FROM darwin_positions WHERE account = ?1 AND symbol = ?2 ORDER BY open_time, id LIMIT ?3".to_string(),
            vec![Box::new(darwin_ticker.to_string()), Box::new(sym.to_string()), Box::new(limit)],
        )
    } else {
        (
            "SELECT id, account, open_time, position_ticket, symbol, pos_type, volume, open_price, sl, tp, close_time, close_price, commission, swap, profit FROM darwin_positions WHERE account = ?1 ORDER BY open_time, id LIMIT ?2".to_string(),
            vec![Box::new(darwin_ticker.to_string()), Box::new(limit)],
        )
    };

    let mut stmt = conn.prepare(&sql).map_err(|e| format!("Prepare failed: {e}"))?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(params_refs.as_slice(), |row| {
        Ok(DarwinPosition {
            id: row.get(0)?,
            account: row.get(1)?,
            open_time: row.get(2)?,
            position_ticket: row.get(3)?,
            symbol: row.get(4)?,
            pos_type: row.get(5)?,
            volume: row.get(6)?,
            open_price: row.get(7)?,
            sl: row.get(8)?,
            tp: row.get(9)?,
            close_time: row.get(10)?,
            close_price: row.get(11)?,
            commission: row.get(12)?,
            swap: row.get(13)?,
            profit: row.get(14)?,
        })
    }).map_err(|e| format!("Query failed: {e}"))?;

    let mut positions = Vec::new();
    for row in rows {
        if let Ok(p) = row { positions.push(p); }
    }
    Ok(positions)
}

/// Get equity curve from deals (balance over time).
pub fn get_darwin_equity_curve(conn: &Connection, darwin_ticker: &str) -> Result<Vec<(String, f64)>, String> {
    let mut stmt = conn.prepare(
        "SELECT time, balance FROM darwin_deals WHERE account = ?1 AND balance > 0 ORDER BY time, id"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let rows = stmt.query_map(params![darwin_ticker], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    }).map_err(|e| format!("Query failed: {e}"))?;

    let mut curve = Vec::new();
    for row in rows {
        if let Ok(point) = row { curve.push(point); }
    }
    Ok(curve)
}

/// Get P/L by symbol for a DARWIN account.
pub fn get_darwin_pnl_by_symbol(conn: &Connection, darwin_ticker: &str) -> Result<Vec<(String, f64, f64, f64, i64)>, String> {
    let mut stmt = conn.prepare(
        "SELECT symbol, SUM(profit), SUM(commission), SUM(swap), COUNT(*) FROM darwin_positions WHERE account = ?1 GROUP BY symbol ORDER BY SUM(profit) DESC"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let rows = stmt.query_map(params![darwin_ticker], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?, row.get::<_, f64>(2)?, row.get::<_, f64>(3)?, row.get::<_, i64>(4)?))
    }).map_err(|e| format!("Query failed: {e}"))?;

    let mut result = Vec::new();
    for row in rows {
        if let Ok(r) = row { result.push(r); }
    }
    Ok(result)
}

// ── Open Position Reconstruction ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarwinOpenPosition {
    pub symbol: String,
    pub side: String,         // "buy" or "sell"
    pub total_volume: f64,
    pub avg_price: f64,
    pub position_count: i64,  // number of individual tickets
    pub notional: f64,        // volume * avg_price
    pub earliest_open: String,
}

/// Reconstruct currently open positions from deals.
/// Uses net volume balance per symbol+side: sum(in volumes) - sum(out volumes).
/// If net > 0, that volume is still open. VWAP computed from "in" deals.
pub fn get_darwin_open_positions(conn: &Connection, darwin_ticker: &str) -> Result<Vec<DarwinOpenPosition>, String> {
    // Aggregate in/out volumes and notional per symbol+side directly in SQL
    // "in" deals add volume, "out" deals subtract volume
    // deal_type for "in" = the side (buy/sell), for "out" = the opposite side
    // So we group by symbol + deal_type on "in" deals only for side detection
    let mut stmt = conn.prepare(
        "SELECT symbol, deal_type, direction, volume, price, time FROM darwin_deals WHERE account = ?1 AND direction IN ('in', 'out') AND symbol != '' ORDER BY time, id"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    // Track per (symbol, side): net volume, weighted notional for VWAP, deal count, earliest
    struct Agg {
        vol_in: f64,
        vol_out: f64,
        notional_in: f64,  // sum of (volume * price) for "in" deals
        count_in: i64,
        earliest: String,
    }

    let mut agg: std::collections::HashMap<(String, String), Agg> = std::collections::HashMap::new();

    let rows = stmt.query_map(params![darwin_ticker], |row| {
        Ok((
            row.get::<_, String>(0)?, // symbol
            row.get::<_, String>(1)?, // deal_type (buy/sell)
            row.get::<_, String>(2)?, // direction (in/out)
            row.get::<_, f64>(3)?,    // volume
            row.get::<_, f64>(4)?,    // price
            row.get::<_, String>(5)?, // time
        ))
    }).map_err(|e| format!("Query failed: {e}"))?;

    for row in rows {
        if let Ok((symbol, deal_type, direction, volume, price, time)) = row {
            if direction == "in" {
                // "in" deal: deal_type is the position side
                let key = (symbol, deal_type);
                let entry = agg.entry(key).or_insert(Agg {
                    vol_in: 0.0, vol_out: 0.0, notional_in: 0.0, count_in: 0, earliest: time.clone(),
                });
                entry.vol_in += volume;
                entry.notional_in += volume * price;
                entry.count_in += 1;
                if time < entry.earliest { entry.earliest = time.clone(); }
            } else if direction == "out" {
                // "out" deal: deal_type is OPPOSITE of position side
                // buy out = closing a sell position, sell out = closing a buy position
                let side = if deal_type == "buy" { "sell" } else { "buy" };
                let key = (symbol, side.to_string());
                if let Some(entry) = agg.get_mut(&key) {
                    entry.vol_out += volume;
                }
            }
        }
    }

    let mut result: Vec<DarwinOpenPosition> = Vec::new();
    for ((symbol, side), a) in agg {
        let net_vol = a.vol_in - a.vol_out;
        if net_vol <= 0.0 { continue; } // fully closed
        let avg_price = if a.vol_in > 0.0 { a.notional_in / a.vol_in } else { 0.0 };
        let notional = net_vol * avg_price;
        result.push(DarwinOpenPosition {
            symbol,
            side,
            total_volume: net_vol,
            avg_price,
            position_count: a.count_in, // total "in" deal count (some may be closed)
            notional,
            earliest_open: a.earliest,
        });
    }

    // Sort by notional descending
    result.sort_by(|a, b| b.notional.partial_cmp(&a.notional).unwrap_or(std::cmp::Ordering::Equal));
    Ok(result)
}

// ── Combined Portfolio Analytics ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSummary {
    pub accounts: Vec<DarwinAccountSummary>,
    pub total_initial_balance: f64,
    pub total_final_balance: f64,
    pub total_net_pnl: f64,
    pub total_commission: f64,
    pub total_deals: i64,
    pub total_positions: i64,
    pub combined_max_drawdown_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioOpenPosition {
    pub symbol: String,
    pub side: String,
    pub total_volume: f64,
    pub avg_price: f64,
    pub notional: f64,
    pub darwin_breakdown: Vec<(String, f64, f64)>, // (ticker, volume, avg_price)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSymbolExposure {
    pub symbol: String,
    pub long_notional: f64,
    pub short_notional: f64,
    pub net_notional: f64,
    pub darwin_count: i64,      // how many DARWINs hold this symbol
    pub darwins: Vec<String>,   // which DARWINs
}

/// Get combined portfolio summary across all DARWINs.
pub fn get_portfolio_summary(conn: &Connection) -> Result<PortfolioSummary, String> {
    let accounts = list_darwin_accounts(conn)?;
    let mut summaries = Vec::new();
    let mut total_initial = 0.0f64;
    let mut total_final = 0.0f64;
    let mut total_pnl = 0.0f64;
    let mut total_comm = 0.0f64;
    let mut total_deals = 0i64;
    let mut total_positions = 0i64;

    for account in &accounts {
        match get_darwin_summary(conn, &account.darwin_ticker) {
            Ok(s) => {
                total_initial += s.account.initial_balance;
                total_final += s.final_balance;
                total_pnl += s.total_profit + s.total_commission + s.total_swap;
                total_comm += s.total_commission;
                total_deals += s.account.deal_count;
                total_positions += s.account.position_count;
                summaries.push(s);
            }
            Err(_) => continue,
        }
    }

    // Combined max drawdown from aggregate equity curve
    let combined_dd = get_portfolio_max_drawdown(conn).unwrap_or(0.0);

    Ok(PortfolioSummary {
        accounts: summaries,
        total_initial_balance: total_initial,
        total_final_balance: total_final,
        total_net_pnl: total_pnl,
        total_commission: total_comm,
        total_deals,
        total_positions,
        combined_max_drawdown_pct: combined_dd,
    })
}

/// Get combined open positions across all DARWINs, aggregated by symbol.
pub fn get_portfolio_open_positions(conn: &Connection) -> Result<Vec<PortfolioOpenPosition>, String> {
    let accounts = list_darwin_accounts(conn)?;

    // (symbol, side) -> vec of (ticker, volume, avg_price)
    let mut combined: std::collections::HashMap<(String, String), Vec<(String, f64, f64)>> = std::collections::HashMap::new();

    for account in &accounts {
        if let Ok(positions) = get_darwin_open_positions(conn, &account.darwin_ticker) {
            for p in positions {
                let key = (p.symbol.clone(), p.side.clone());
                combined.entry(key).or_default().push((
                    account.darwin_ticker.clone(),
                    p.total_volume,
                    p.avg_price,
                ));
            }
        }
    }

    let mut result: Vec<PortfolioOpenPosition> = combined.into_iter().map(|((symbol, side), entries)| {
        let total_vol: f64 = entries.iter().map(|(_, v, _)| v).sum();
        let total_notional: f64 = entries.iter().map(|(_, v, p)| v * p).sum();
        let avg_price = if total_vol > 0.0 { total_notional / total_vol } else { 0.0 };
        PortfolioOpenPosition {
            symbol,
            side,
            total_volume: total_vol,
            avg_price,
            notional: total_notional,
            darwin_breakdown: entries,
        }
    }).collect();

    result.sort_by(|a, b| b.notional.partial_cmp(&a.notional).unwrap_or(std::cmp::Ordering::Equal));
    Ok(result)
}

/// Get symbol exposure across all DARWINs (long + short per symbol).
pub fn get_portfolio_exposure(conn: &Connection) -> Result<Vec<PortfolioSymbolExposure>, String> {
    let accounts = list_darwin_accounts(conn)?;

    // symbol -> (long_notional, short_notional, darwins_set)
    let mut exposure: std::collections::HashMap<String, (f64, f64, std::collections::HashSet<String>)> = std::collections::HashMap::new();

    for account in &accounts {
        if let Ok(positions) = get_darwin_open_positions(conn, &account.darwin_ticker) {
            for p in positions {
                let entry = exposure.entry(p.symbol.clone()).or_insert((0.0, 0.0, std::collections::HashSet::new()));
                if p.side == "buy" {
                    entry.0 += p.notional;
                } else {
                    entry.1 += p.notional;
                }
                entry.2.insert(account.darwin_ticker.clone());
            }
        }
    }

    let mut result: Vec<PortfolioSymbolExposure> = exposure.into_iter().map(|(symbol, (long, short, darwins))| {
        let mut d: Vec<String> = darwins.into_iter().collect();
        d.sort();
        PortfolioSymbolExposure {
            symbol,
            long_notional: long,
            short_notional: short,
            net_notional: long - short,
            darwin_count: d.len() as i64,
            darwins: d,
        }
    }).collect();

    // Sort by absolute net notional descending
    result.sort_by(|a, b| b.net_notional.abs().partial_cmp(&a.net_notional.abs()).unwrap_or(std::cmp::Ordering::Equal));
    Ok(result)
}

/// Get combined equity curve across all DARWINs (daily aggregate).
pub fn get_portfolio_equity_curve(conn: &Connection) -> Result<Vec<(String, f64)>, String> {
    let accounts = list_darwin_accounts(conn)?;

    // Collect all equity points from all accounts, keyed by date
    let mut daily: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();

    for account in &accounts {
        if let Ok(curve) = get_darwin_equity_curve(conn, &account.darwin_ticker) {
            // Track last known balance per account
            let mut last_bal = account.initial_balance;
            for (time, balance) in &curve {
                let date = time.get(..10).unwrap_or(time).to_string();
                last_bal = *balance;
                *daily.entry(date).or_insert(0.0) += last_bal;
            }
        }
    }

    Ok(daily.into_iter().collect())
}

fn get_portfolio_max_drawdown(conn: &Connection) -> Result<f64, String> {
    let curve = get_portfolio_equity_curve(conn)?;
    let mut peak = 0.0f64;
    let mut max_dd = 0.0f64;
    for (_, balance) in &curve {
        if *balance > peak { peak = *balance; }
        if peak > 0.0 {
            let dd = (peak - balance) / peak * 100.0;
            if dd > max_dd { max_dd = dd; }
        }
    }
    Ok(max_dd)
}

// ── Advanced Analytics ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyReturn {
    pub date: String,
    pub pnl: f64,
    pub balance: f64,
    pub return_pct: f64,
    pub drawdown_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaRResult {
    pub var_95: f64,
    pub var_99: f64,
    pub cvar_95: f64,       // conditional VaR (expected shortfall)
    pub cvar_99: f64,
    pub daily_vol: f64,     // daily volatility
    pub annualized_vol: f64,
    pub sharpe: f64,
    pub sortino: f64,
    pub calmar: f64,
    pub max_drawdown_pct: f64,
    pub avg_daily_pnl: f64,
    pub worst_day: f64,
    pub best_day: f64,
    pub trading_days: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyReturn {
    pub year: i32,
    pub month: i32,
    pub pnl: f64,
    pub return_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollingVaR {
    pub date: String,
    pub var_95: f64,
    pub var_99: f64,
    pub rolling_sharpe: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationEntry {
    pub darwin_a: String,
    pub darwin_b: String,
    pub correlation: f64,
}

/// Get daily returns from deal balance changes for a DARWIN.
pub fn get_daily_returns(conn: &Connection, darwin_ticker: &str) -> Result<Vec<DailyReturn>, String> {
    // Get daily balance snapshots (last balance per day)
    let mut stmt = conn.prepare(
        "SELECT SUBSTR(time, 1, 10) as date, balance FROM darwin_deals WHERE account = ?1 AND balance > 0 ORDER BY time, id"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let rows = stmt.query_map(params![darwin_ticker], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    }).map_err(|e| format!("Query failed: {e}"))?;

    // Deduplicate to last balance per day
    let mut daily_balances: Vec<(String, f64)> = Vec::new();
    let mut last_date = String::new();
    for row in rows {
        if let Ok((date, balance)) = row {
            if date == last_date {
                if let Some(last) = daily_balances.last_mut() { last.1 = balance; }
            } else {
                daily_balances.push((date.clone(), balance));
                last_date = date;
            }
        }
    }

    // Compute returns and drawdown
    let mut result = Vec::new();
    let mut peak = 0.0f64;
    for i in 0..daily_balances.len() {
        let (ref date, balance) = daily_balances[i];
        let prev_balance = if i > 0 { daily_balances[i - 1].1 } else { balance };
        let pnl = balance - prev_balance;
        let return_pct = if prev_balance > 0.0 { pnl / prev_balance * 100.0 } else { 0.0 };
        if balance > peak { peak = balance; }
        let drawdown_pct = if peak > 0.0 { (peak - balance) / peak * 100.0 } else { 0.0 };

        result.push(DailyReturn { date: date.clone(), pnl, balance, return_pct, drawdown_pct });
    }
    Ok(result)
}

/// Compute VaR and risk metrics for a DARWIN or portfolio.
pub fn compute_var(daily_returns: &[DailyReturn]) -> VaRResult {
    if daily_returns.len() < 2 {
        return VaRResult {
            var_95: 0.0, var_99: 0.0, cvar_95: 0.0, cvar_99: 0.0,
            daily_vol: 0.0, annualized_vol: 0.0, sharpe: 0.0, sortino: 0.0, calmar: 0.0,
            max_drawdown_pct: 0.0, avg_daily_pnl: 0.0, worst_day: 0.0, best_day: 0.0, trading_days: 0,
        };
    }

    let pnls: Vec<f64> = daily_returns.iter().map(|r| r.pnl).collect();
    let returns: Vec<f64> = daily_returns.iter().map(|r| r.return_pct).collect();
    let n = pnls.len() as f64;

    let avg_pnl = pnls.iter().sum::<f64>() / n;
    let avg_ret = returns.iter().sum::<f64>() / n;

    // Daily volatility
    let variance = returns.iter().map(|r| (r - avg_ret).powi(2)).sum::<f64>() / (n - 1.0);
    let daily_vol = variance.sqrt();
    let annualized_vol = daily_vol * (252.0f64).sqrt();

    // Downside deviation (for Sortino)
    let downside_var = returns.iter().filter(|r| **r < 0.0).map(|r| r.powi(2)).sum::<f64>()
        / returns.iter().filter(|r| **r < 0.0).count().max(1) as f64;
    let downside_dev = downside_var.sqrt();

    // Sort returns for percentile VaR
    let mut sorted_pnls = pnls.clone();
    sorted_pnls.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let idx_95 = ((pnls.len() as f64) * 0.05).floor() as usize;
    let idx_99 = ((pnls.len() as f64) * 0.01).floor() as usize;

    let var_95 = sorted_pnls.get(idx_95).copied().unwrap_or(0.0).abs();
    let var_99 = sorted_pnls.get(idx_99).copied().unwrap_or(0.0).abs();

    // CVaR (expected shortfall) = average of losses beyond VaR
    let cvar_95 = if idx_95 > 0 {
        sorted_pnls[..idx_95].iter().sum::<f64>() / idx_95 as f64
    } else { sorted_pnls[0] }.abs();
    let cvar_99 = if idx_99 > 0 {
        sorted_pnls[..idx_99].iter().sum::<f64>() / idx_99 as f64
    } else { sorted_pnls[0] }.abs();

    let max_dd = daily_returns.iter().map(|r| r.drawdown_pct).fold(0.0f64, |a, b| a.max(b));
    let worst = sorted_pnls.first().copied().unwrap_or(0.0);
    let best = sorted_pnls.last().copied().unwrap_or(0.0);

    // Sharpe (annualized, risk-free = 0)
    let sharpe = if daily_vol > 0.0 { avg_ret / daily_vol * (252.0f64).sqrt() } else { 0.0 };
    let sortino = if downside_dev > 0.0 { avg_ret / downside_dev * (252.0f64).sqrt() } else { 0.0 };

    // Calmar (annualized return / max drawdown)
    let annualized_return = avg_ret * 252.0;
    let calmar = if max_dd > 0.0 { annualized_return / max_dd } else { 0.0 };

    VaRResult {
        var_95, var_99, cvar_95, cvar_99,
        daily_vol, annualized_vol, sharpe, sortino, calmar,
        max_drawdown_pct: max_dd, avg_daily_pnl: avg_pnl,
        worst_day: worst, best_day: best,
        trading_days: pnls.len() as i64,
    }
}

/// Get monthly returns for a DARWIN.
pub fn get_monthly_returns(daily_returns: &[DailyReturn]) -> Vec<MonthlyReturn> {
    let mut monthly: std::collections::BTreeMap<(i32, i32), (f64, f64, f64)> = std::collections::BTreeMap::new();
    // (year, month) -> (total_pnl, start_balance, end_balance)

    for r in daily_returns {
        if r.date.len() < 7 { continue; }
        let year: i32 = r.date[..4].parse().unwrap_or(0);
        let month: i32 = r.date[5..7].parse().unwrap_or(0);
        if year == 0 || month == 0 { continue; }
        let entry = monthly.entry((year, month)).or_insert((0.0, r.balance - r.pnl, r.balance));
        entry.0 += r.pnl;
        entry.2 = r.balance; // update end balance
    }

    monthly.into_iter().map(|((year, month), (pnl, start, _end))| {
        let return_pct = if start > 0.0 { pnl / start * 100.0 } else { 0.0 };
        MonthlyReturn { year, month, pnl, return_pct }
    }).collect()
}

/// Compute rolling VaR (window_days lookback).
pub fn get_rolling_var(daily_returns: &[DailyReturn], window_days: usize) -> Vec<RollingVaR> {
    if daily_returns.len() < window_days { return Vec::new(); }

    let mut result = Vec::new();
    for i in window_days..daily_returns.len() {
        let window = &daily_returns[i - window_days..i];
        let mut pnls: Vec<f64> = window.iter().map(|r| r.pnl).collect();
        pnls.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let idx_95 = ((pnls.len() as f64) * 0.05).floor() as usize;
        let idx_99 = ((pnls.len() as f64) * 0.01).floor() as usize;
        let var_95 = pnls.get(idx_95).copied().unwrap_or(0.0).abs();
        let var_99 = pnls.get(idx_99).copied().unwrap_or(0.0).abs();

        let rets: Vec<f64> = window.iter().map(|r| r.return_pct).collect();
        let avg = rets.iter().sum::<f64>() / rets.len() as f64;
        let vol = (rets.iter().map(|r| (r - avg).powi(2)).sum::<f64>() / (rets.len() - 1) as f64).sqrt();
        let sharpe = if vol > 0.0 { avg / vol * (252.0f64).sqrt() } else { 0.0 };

        result.push(RollingVaR {
            date: daily_returns[i].date.clone(), var_95, var_99, rolling_sharpe: sharpe,
        });
    }
    result
}

/// Compute cross-DARWIN correlation matrix from daily returns.
pub fn get_darwin_correlations(conn: &Connection) -> Result<Vec<CorrelationEntry>, String> {
    let accounts = list_darwin_accounts(conn)?;
    let mut all_returns: Vec<(String, std::collections::HashMap<String, f64>)> = Vec::new();

    for account in &accounts {
        let returns = get_daily_returns(conn, &account.darwin_ticker)?;
        let map: std::collections::HashMap<String, f64> = returns.iter()
            .map(|r| (r.date.clone(), r.return_pct))
            .collect();
        all_returns.push((account.darwin_ticker.clone(), map));
    }

    let mut result = Vec::new();
    for i in 0..all_returns.len() {
        for j in i..all_returns.len() {
            let (ref name_a, ref map_a) = all_returns[i];
            let (ref name_b, ref map_b) = all_returns[j];

            // Find common dates
            let mut pairs: Vec<(f64, f64)> = Vec::new();
            for (date, ret_a) in map_a {
                if let Some(ret_b) = map_b.get(date) {
                    pairs.push((*ret_a, *ret_b));
                }
            }

            let corr = if pairs.len() > 2 {
                let n = pairs.len() as f64;
                let mean_a = pairs.iter().map(|(a, _)| a).sum::<f64>() / n;
                let mean_b = pairs.iter().map(|(_, b)| b).sum::<f64>() / n;
                let cov = pairs.iter().map(|(a, b)| (a - mean_a) * (b - mean_b)).sum::<f64>() / (n - 1.0);
                let std_a = (pairs.iter().map(|(a, _)| (a - mean_a).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
                let std_b = (pairs.iter().map(|(_, b)| (b - mean_b).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
                if std_a > 0.0 && std_b > 0.0 { cov / (std_a * std_b) } else { 0.0 }
            } else { 0.0 };

            result.push(CorrelationEntry { darwin_a: name_a.clone(), darwin_b: name_b.clone(), correlation: corr });
            if i != j {
                result.push(CorrelationEntry { darwin_a: name_b.clone(), darwin_b: name_a.clone(), correlation: corr });
            }
        }
    }
    Ok(result)
}

/// Get combined daily returns across all DARWINs (portfolio-level).
pub fn get_portfolio_daily_returns(conn: &Connection) -> Result<Vec<DailyReturn>, String> {
    let accounts = list_darwin_accounts(conn)?;
    let mut combined: std::collections::BTreeMap<String, (f64, f64)> = std::collections::BTreeMap::new(); // date -> (total_pnl, total_balance)

    for account in &accounts {
        let returns = get_daily_returns(conn, &account.darwin_ticker)?;
        for r in &returns {
            let entry = combined.entry(r.date.clone()).or_insert((0.0, 0.0));
            entry.0 += r.pnl;
            entry.1 += r.balance;
        }
    }

    let mut result = Vec::new();
    let mut peak = 0.0f64;
    for (date, (pnl, balance)) in &combined {
        let prev_balance = balance - pnl;
        let return_pct = if prev_balance > 0.0 { pnl / prev_balance * 100.0 } else { 0.0 };
        if *balance > peak { peak = *balance; }
        let drawdown_pct = if peak > 0.0 { (peak - balance) / peak * 100.0 } else { 0.0 };
        result.push(DailyReturn { date: date.clone(), pnl: *pnl, balance: *balance, return_pct, drawdown_pct });
    }
    Ok(result)
}

// ── Trade Pattern Analytics ──────────────────────────────────────────

/// Parse MT5 datetime string "YYYY.MM.DD HH:MM:SS" into chrono NaiveDateTime.
fn parse_mt5_datetime(s: &str) -> Option<chrono::NaiveDateTime> {
    chrono::NaiveDateTime::parse_from_str(s, "%Y.%m.%d %H:%M:%S").ok()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreakAnalysis {
    pub max_win_streak: i64,
    pub max_loss_streak: i64,
    pub current_streak: i64, // positive = wins, negative = losses
    pub avg_win_streak: f64,
    pub avg_loss_streak: f64,
    pub streak_distribution: Vec<(i64, i64)>, // (streak_length, count) — positive = win, negative = loss
}

/// Analyze win/loss streaks from closed positions ordered by open_time.
/// A "win" is profit + commission + swap > 0.
pub fn get_streak_analysis(conn: &Connection, darwin_ticker: &str) -> Result<StreakAnalysis, String> {
    let mut stmt = conn.prepare(
        "SELECT profit, commission, swap FROM darwin_positions WHERE account = ?1 ORDER BY open_time, id"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let rows = stmt.query_map(params![darwin_ticker], |row| {
        Ok((row.get::<_, f64>(0)?, row.get::<_, f64>(1)?, row.get::<_, f64>(2)?))
    }).map_err(|e| format!("Query failed: {e}"))?;

    let mut outcomes: Vec<bool> = Vec::new(); // true = win
    for row in rows {
        if let Ok((profit, commission, swap)) = row {
            outcomes.push(profit + commission + swap > 0.0);
        }
    }

    if outcomes.is_empty() {
        return Ok(StreakAnalysis {
            max_win_streak: 0, max_loss_streak: 0, current_streak: 0,
            avg_win_streak: 0.0, avg_loss_streak: 0.0, streak_distribution: Vec::new(),
        });
    }

    // Build streaks: list of signed streak lengths
    let mut streaks: Vec<i64> = Vec::new();
    let mut current_len: i64 = 0;
    let mut current_is_win = outcomes[0];

    for &win in &outcomes {
        if win == current_is_win {
            current_len += 1;
        } else {
            streaks.push(if current_is_win { current_len } else { -current_len });
            current_is_win = win;
            current_len = 1;
        }
    }
    // Push final streak
    streaks.push(if current_is_win { current_len } else { -current_len });

    let max_win_streak = streaks.iter().filter(|s| **s > 0).copied().max().unwrap_or(0);
    let max_loss_streak = streaks.iter().filter(|s| **s < 0).map(|s| s.abs()).max().unwrap_or(0);
    let current_streak = *streaks.last().unwrap_or(&0);

    let win_streaks: Vec<i64> = streaks.iter().filter(|s| **s > 0).copied().collect();
    let loss_streaks: Vec<i64> = streaks.iter().filter(|s| **s < 0).map(|s| s.abs()).collect();

    let avg_win_streak = if !win_streaks.is_empty() {
        win_streaks.iter().sum::<i64>() as f64 / win_streaks.len() as f64
    } else { 0.0 };
    let avg_loss_streak = if !loss_streaks.is_empty() {
        loss_streaks.iter().sum::<i64>() as f64 / loss_streaks.len() as f64
    } else { 0.0 };

    // Distribution: count occurrences of each streak length
    let mut dist_map: std::collections::BTreeMap<i64, i64> = std::collections::BTreeMap::new();
    for &s in &streaks {
        *dist_map.entry(s).or_insert(0) += 1;
    }
    let streak_distribution: Vec<(i64, i64)> = dist_map.into_iter().collect();

    Ok(StreakAnalysis {
        max_win_streak, max_loss_streak, current_streak,
        avg_win_streak, avg_loss_streak, streak_distribution,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyPnL {
    pub hour: i32, // 0-23
    pub total_pnl: f64,
    pub trade_count: i64,
    pub win_count: i64,
    pub avg_pnl: f64,
}

/// Get P/L broken down by hour of day (from open_time).
pub fn get_hourly_pnl(conn: &Connection, darwin_ticker: &str) -> Result<Vec<HourlyPnL>, String> {
    let mut stmt = conn.prepare(
        "SELECT open_time, profit, commission, swap FROM darwin_positions WHERE account = ?1"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let rows = stmt.query_map(params![darwin_ticker], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?, row.get::<_, f64>(2)?, row.get::<_, f64>(3)?))
    }).map_err(|e| format!("Query failed: {e}"))?;

    // hour -> (total_pnl, count, wins)
    let mut buckets: [( f64, i64, i64); 24] = [(0.0, 0, 0); 24];

    for row in rows {
        if let Ok((open_time, profit, commission, swap)) = row {
            if let Some(dt) = parse_mt5_datetime(&open_time) {
                let h = dt.format("%H").to_string().parse::<usize>().unwrap_or(0);
                if h < 24 {
                    let net = profit + commission + swap;
                    buckets[h].0 += net;
                    buckets[h].1 += 1;
                    if net > 0.0 { buckets[h].2 += 1; }
                }
            }
        }
    }

    let result: Vec<HourlyPnL> = (0..24).map(|h| {
        let (total_pnl, trade_count, win_count) = buckets[h];
        HourlyPnL {
            hour: h as i32,
            total_pnl,
            trade_count,
            win_count,
            avg_pnl: if trade_count > 0 { total_pnl / trade_count as f64 } else { 0.0 },
        }
    }).collect();

    Ok(result)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayOfWeekPnL {
    pub day: String, // "Monday" etc
    pub day_num: i32, // 0=Mon..6=Sun
    pub total_pnl: f64,
    pub trade_count: i64,
    pub win_rate: f64,
    pub avg_pnl: f64,
}

/// Get P/L broken down by day of week (from open_time).
pub fn get_day_of_week_pnl(conn: &Connection, darwin_ticker: &str) -> Result<Vec<DayOfWeekPnL>, String> {
    use chrono::Datelike;

    let mut stmt = conn.prepare(
        "SELECT open_time, profit, commission, swap FROM darwin_positions WHERE account = ?1"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let rows = stmt.query_map(params![darwin_ticker], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?, row.get::<_, f64>(2)?, row.get::<_, f64>(3)?))
    }).map_err(|e| format!("Query failed: {e}"))?;

    // day_num (0=Mon..6=Sun) -> (total_pnl, count, wins)
    let mut buckets: [(f64, i64, i64); 7] = [(0.0, 0, 0); 7];

    for row in rows {
        if let Ok((open_time, profit, commission, swap)) = row {
            if let Some(dt) = parse_mt5_datetime(&open_time) {
                let dow = dt.date().weekday().num_days_from_monday() as usize; // 0=Mon
                if dow < 7 {
                    let net = profit + commission + swap;
                    buckets[dow].0 += net;
                    buckets[dow].1 += 1;
                    if net > 0.0 { buckets[dow].2 += 1; }
                }
            }
        }
    }

    let day_names = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
    let result: Vec<DayOfWeekPnL> = (0..7).map(|d| {
        let (total_pnl, trade_count, win_count) = buckets[d];
        DayOfWeekPnL {
            day: day_names[d].to_string(),
            day_num: d as i32,
            total_pnl,
            trade_count,
            win_rate: if trade_count > 0 { win_count as f64 / trade_count as f64 * 100.0 } else { 0.0 },
            avg_pnl: if trade_count > 0 { total_pnl / trade_count as f64 } else { 0.0 },
        }
    }).collect();

    Ok(result)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoldTimeStats {
    pub avg_hold_hours: f64,
    pub median_hold_hours: f64,
    pub min_hold_hours: f64,
    pub max_hold_hours: f64,
    pub buckets: Vec<(String, i64, f64)>, // (label like "<1h", count, avg_pnl)
}

/// Compute hold time distribution from open_time to close_time.
pub fn get_hold_time_stats(conn: &Connection, darwin_ticker: &str) -> Result<HoldTimeStats, String> {
    let mut stmt = conn.prepare(
        "SELECT open_time, close_time, profit, commission, swap FROM darwin_positions WHERE account = ?1 AND close_time != ''"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let rows = stmt.query_map(params![darwin_ticker], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, f64>(2)?,
            row.get::<_, f64>(3)?,
            row.get::<_, f64>(4)?,
        ))
    }).map_err(|e| format!("Query failed: {e}"))?;

    struct HoldEntry {
        hours: f64,
        pnl: f64,
    }

    let mut entries: Vec<HoldEntry> = Vec::new();

    for row in rows {
        if let Ok((open_time, close_time, profit, commission, swap)) = row {
            if let (Some(open_dt), Some(close_dt)) = (parse_mt5_datetime(&open_time), parse_mt5_datetime(&close_time)) {
                let duration = close_dt.signed_duration_since(open_dt);
                let hours = duration.num_seconds() as f64 / 3600.0;
                if hours >= 0.0 {
                    entries.push(HoldEntry { hours, pnl: profit + commission + swap });
                }
            }
        }
    }

    if entries.is_empty() {
        return Ok(HoldTimeStats {
            avg_hold_hours: 0.0, median_hold_hours: 0.0,
            min_hold_hours: 0.0, max_hold_hours: 0.0, buckets: Vec::new(),
        });
    }

    let mut hours_list: Vec<f64> = entries.iter().map(|e| e.hours).collect();
    hours_list.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let avg_hold_hours = hours_list.iter().sum::<f64>() / hours_list.len() as f64;
    let median_hold_hours = if hours_list.len() % 2 == 0 {
        (hours_list[hours_list.len() / 2 - 1] + hours_list[hours_list.len() / 2]) / 2.0
    } else {
        hours_list[hours_list.len() / 2]
    };
    let min_hold_hours = hours_list.first().copied().unwrap_or(0.0);
    let max_hold_hours = hours_list.last().copied().unwrap_or(0.0);

    // Bucket definitions: (label, min_hours, max_hours)
    let bucket_defs: Vec<(&str, f64, f64)> = vec![
        ("<1h",   0.0,    1.0),
        ("1-4h",  1.0,    4.0),
        ("4-24h", 4.0,   24.0),
        ("1-3d", 24.0,   72.0),
        ("3-7d", 72.0,  168.0),
        ("1-4w", 168.0, 672.0),
        (">4w",  672.0, f64::MAX),
    ];

    let mut buckets: Vec<(String, i64, f64)> = Vec::new();
    for (label, lo, hi) in &bucket_defs {
        let matching: Vec<&HoldEntry> = entries.iter()
            .filter(|e| e.hours >= *lo && e.hours < *hi)
            .collect();
        let count = matching.len() as i64;
        let avg_pnl = if count > 0 {
            matching.iter().map(|e| e.pnl).sum::<f64>() / count as f64
        } else { 0.0 };
        buckets.push((label.to_string(), count, avg_pnl));
    }

    Ok(HoldTimeStats { avg_hold_hours, median_hold_hours, min_hold_hours, max_hold_hours, buckets })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolActivity {
    pub symbol: String,
    pub first_trade: String,
    pub last_trade: String,
    pub trade_count: i64,
    pub total_pnl: f64,
    pub active_months: i64,
}

/// Get symbol rotation timeline showing when each symbol was first/last traded.
pub fn get_symbol_rotation(conn: &Connection, darwin_ticker: &str) -> Result<Vec<SymbolActivity>, String> {
    let mut stmt = conn.prepare(
        "SELECT symbol, MIN(open_time) as first_trade, MAX(open_time) as last_trade, COUNT(*) as trade_count, SUM(profit + commission + swap) as total_pnl FROM darwin_positions WHERE account = ?1 GROUP BY symbol ORDER BY MIN(open_time)"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let rows = stmt.query_map(params![darwin_ticker], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, f64>(4)?,
        ))
    }).map_err(|e| format!("Query failed: {e}"))?;

    let mut result = Vec::new();
    for row in rows {
        if let Ok((symbol, first_trade, last_trade, trade_count, total_pnl)) = row {
            // Compute active months from first to last trade
            let active_months = match (parse_mt5_datetime(&first_trade), parse_mt5_datetime(&last_trade)) {
                (Some(first), Some(last)) => {
                    use chrono::Datelike;
                    let months = (last.date().year() - first.date().year()) * 12
                        + (last.date().month() as i32 - first.date().month() as i32)
                        + 1; // inclusive
                    months.max(1) as i64
                }
                _ => 1,
            };
            result.push(SymbolActivity { symbol, first_trade, last_trade, trade_count, total_pnl, active_months });
        }
    }
    Ok(result)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizingEfficiency {
    pub quartile: String, // "Q1 (smallest)" etc
    pub avg_volume: f64,
    pub trade_count: i64,
    pub avg_pnl: f64,
    pub win_rate: f64,
    pub total_pnl: f64,
}

/// Split trades into quartiles by volume and compute stats per quartile.
pub fn get_sizing_efficiency(conn: &Connection, darwin_ticker: &str) -> Result<Vec<SizingEfficiency>, String> {
    let mut stmt = conn.prepare(
        "SELECT volume, profit, commission, swap FROM darwin_positions WHERE account = ?1 ORDER BY volume"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let rows = stmt.query_map(params![darwin_ticker], |row| {
        Ok((row.get::<_, f64>(0)?, row.get::<_, f64>(1)?, row.get::<_, f64>(2)?, row.get::<_, f64>(3)?))
    }).map_err(|e| format!("Query failed: {e}"))?;

    struct Trade { volume: f64, pnl: f64 }
    let mut trades: Vec<Trade> = Vec::new();
    for row in rows {
        if let Ok((volume, profit, commission, swap)) = row {
            trades.push(Trade { volume, pnl: profit + commission + swap });
        }
    }

    if trades.is_empty() {
        return Ok(Vec::new());
    }

    // Already sorted by volume from SQL ORDER BY
    let n = trades.len();
    let quartile_size = n / 4;
    let remainder = n % 4;

    let labels = [
        "Q1 (smallest)",
        "Q2",
        "Q3",
        "Q4 (largest)",
    ];

    let mut result = Vec::new();
    let mut offset = 0;
    for q in 0..4 {
        // Distribute remainder trades across first quartiles
        let size = quartile_size + if q < remainder { 1 } else { 0 };
        if size == 0 { continue; }
        let slice = &trades[offset..offset + size];
        offset += size;

        let total_vol: f64 = slice.iter().map(|t| t.volume).sum();
        let total_pnl: f64 = slice.iter().map(|t| t.pnl).sum();
        let win_count = slice.iter().filter(|t| t.pnl > 0.0).count();
        let count = slice.len() as i64;

        result.push(SizingEfficiency {
            quartile: labels[q].to_string(),
            avg_volume: total_vol / count as f64,
            trade_count: count,
            avg_pnl: total_pnl / count as f64,
            win_rate: win_count as f64 / count as f64 * 100.0,
            total_pnl,
        });
    }

    Ok(result)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostAnalysis {
    pub total_commission: f64,
    pub total_swap: f64,
    pub commission_pct_of_equity: f64,
    pub avg_commission_per_trade: f64,
    pub commission_per_symbol: Vec<(String, f64, i64)>, // (symbol, total_commission, trade_count)
    pub cumulative_costs: Vec<(String, f64)>, // (date, running total)
}

/// Analyze commission and swap costs.
pub fn get_cost_analysis(conn: &Connection, darwin_ticker: &str) -> Result<CostAnalysis, String> {
    // Total commission and swap
    let (total_commission, total_swap, trade_count): (f64, f64, i64) = conn.query_row(
        "SELECT COALESCE(SUM(commission), 0), COALESCE(SUM(swap), 0), COUNT(*) FROM darwin_positions WHERE account = ?1",
        params![darwin_ticker],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).map_err(|e| format!("Query failed: {e}"))?;

    // Get final balance for percentage calculation
    let final_balance: f64 = conn.query_row(
        "SELECT balance FROM darwin_deals WHERE account = ?1 AND balance > 0 ORDER BY time DESC, id DESC LIMIT 1",
        params![darwin_ticker],
        |row| row.get(0),
    ).unwrap_or(0.0);

    let commission_pct_of_equity = if final_balance > 0.0 {
        total_commission.abs() / final_balance * 100.0
    } else { 0.0 };

    let avg_commission_per_trade = if trade_count > 0 {
        total_commission / trade_count as f64
    } else { 0.0 };

    // Commission per symbol
    let mut stmt = conn.prepare(
        "SELECT symbol, SUM(commission), COUNT(*) FROM darwin_positions WHERE account = ?1 GROUP BY symbol ORDER BY SUM(commission)"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let rows = stmt.query_map(params![darwin_ticker], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?, row.get::<_, i64>(2)?))
    }).map_err(|e| format!("Query failed: {e}"))?;

    let mut commission_per_symbol = Vec::new();
    for row in rows {
        if let Ok(r) = row { commission_per_symbol.push(r); }
    }

    // Cumulative costs over time (by date from positions ordered by open_time)
    let mut stmt2 = conn.prepare(
        "SELECT open_time, commission, swap FROM darwin_positions WHERE account = ?1 ORDER BY open_time, id"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let rows2 = stmt2.query_map(params![darwin_ticker], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?, row.get::<_, f64>(2)?))
    }).map_err(|e| format!("Query failed: {e}"))?;

    let mut running_total = 0.0f64;
    let mut cumulative_costs: Vec<(String, f64)> = Vec::new();
    let mut last_date = String::new();
    for row in rows2 {
        if let Ok((open_time, comm, swp)) = row {
            running_total += comm.abs() + swp.abs();
            let date = open_time.get(..10).unwrap_or(&open_time).to_string();
            if date == last_date {
                if let Some(last) = cumulative_costs.last_mut() {
                    last.1 = running_total;
                }
            } else {
                cumulative_costs.push((date.clone(), running_total));
                last_date = date;
            }
        }
    }

    Ok(CostAnalysis {
        total_commission,
        total_swap,
        commission_pct_of_equity,
        avg_commission_per_trade,
        commission_per_symbol,
        cumulative_costs,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeOverlap {
    pub symbol: String,
    pub darwins: Vec<String>,
    pub darwin_count: i64,
    pub combined_volume: f64,
    pub combined_notional: f64,
}

/// Find symbols held simultaneously across multiple DARWINs from open positions.
pub fn get_trade_overlaps(conn: &Connection) -> Result<Vec<TradeOverlap>, String> {
    let accounts = list_darwin_accounts(conn)?;

    // symbol -> vec of (darwin_ticker, volume, notional)
    let mut symbol_map: std::collections::HashMap<String, Vec<(String, f64, f64)>> = std::collections::HashMap::new();

    for account in &accounts {
        if let Ok(positions) = get_darwin_open_positions(conn, &account.darwin_ticker) {
            for p in positions {
                symbol_map.entry(p.symbol.clone()).or_default().push((
                    account.darwin_ticker.clone(),
                    p.total_volume,
                    p.notional,
                ));
            }
        }
    }

    // Filter to only symbols held in multiple DARWINs
    let mut result: Vec<TradeOverlap> = symbol_map.into_iter()
        .filter(|(_, entries)| {
            let unique_darwins: std::collections::HashSet<&str> = entries.iter().map(|(d, _, _)| d.as_str()).collect();
            unique_darwins.len() > 1
        })
        .map(|(symbol, entries)| {
            let combined_volume: f64 = entries.iter().map(|(_, v, _)| v).sum();
            let combined_notional: f64 = entries.iter().map(|(_, _, n)| n).sum();
            let mut darwins: Vec<String> = entries.iter().map(|(d, _, _)| d.clone()).collect();
            darwins.sort();
            darwins.dedup();
            let darwin_count = darwins.len() as i64;
            TradeOverlap { symbol, darwins, darwin_count, combined_volume, combined_notional }
        })
        .collect();

    result.sort_by(|a, b| b.combined_notional.partial_cmp(&a.combined_notional).unwrap_or(std::cmp::Ordering::Equal));
    Ok(result)
}

// ── DARWIN FTP Screener ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarwinScreenResult {
    pub ticker: String,
    pub return_pct: f64,
    pub max_drawdown: f64,
    pub sharpe: f64,
    pub trading_days: i64,
    pub avg_trades_per_day: f64,
    pub symbols_traded: Vec<String>,
    pub score: f64,  // composite ranking score
}

/// Scan Darwinex FTP RETURN files to find DARWINs matching criteria.
/// Reads from /mnt/bigraidz2/Darwinex_FTP/<ticker>/RETURN
pub fn scan_darwin_ftp(
    ftp_path: &str,
    min_days: i64,
    min_return: f64,
    max_drawdown: f64,
    limit: usize,
) -> Result<Vec<DarwinScreenResult>, String> {
    let ftp_dir = std::path::Path::new(ftp_path);
    if !ftp_dir.exists() {
        return Err(format!("FTP path not found: {}", ftp_path));
    }

    let mut results: Vec<DarwinScreenResult> = Vec::new();
    let entries = std::fs::read_dir(ftp_dir)
        .map_err(|e| format!("Read dir failed: {e}"))?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let ticker = entry.file_name().to_str().unwrap_or("").to_string();
        if ticker.is_empty() || ticker.starts_with('.') { continue; }

        let return_path = entry.path().join("RETURN");
        if !return_path.exists() { continue; }

        // Parse RETURN file: timestamp,experience_score,[return_values...]
        let content = match std::fs::read_to_string(&return_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let lines: Vec<&str> = content.lines().collect();
        if lines.len() < min_days as usize { continue; }

        // Extract return values from last line
        let last_line = lines.last().unwrap_or(&"");
        let parts: Vec<&str> = last_line.splitn(3, ',').collect();
        if parts.len() < 3 { continue; }

        // Parse the return array from the last line
        let return_str = parts[2].trim_start_matches('[').trim_end_matches(']');
        let last_return: f64 = return_str.split(',')
            .last()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .unwrap_or(1.0);

        let total_return = (last_return - 1.0) * 100.0;
        if total_return < min_return { continue; }

        // Compute max drawdown from all return values across all lines
        let mut peak = 0.0f64;
        let mut max_dd = 0.0f64;
        let mut daily_returns: Vec<f64> = Vec::new();
        let mut prev_val = 1.0f64;

        for line in &lines {
            let lp: Vec<&str> = line.splitn(3, ',').collect();
            if lp.len() < 3 { continue; }
            let vals_str = lp[2].trim_start_matches('[').trim_end_matches(']');
            for val_str in vals_str.split(',') {
                if let Ok(val) = val_str.trim().parse::<f64>() {
                    if val > peak { peak = val; }
                    if peak > 0.0 {
                        let dd = (peak - val) / peak * 100.0;
                        if dd > max_dd { max_dd = dd; }
                    }
                    let ret = (val - prev_val) / prev_val;
                    daily_returns.push(ret);
                    prev_val = val;
                }
            }
        }

        if max_dd > max_drawdown { continue; }

        // Compute Sharpe
        let n = daily_returns.len() as f64;
        let avg = if n > 0.0 { daily_returns.iter().sum::<f64>() / n } else { 0.0 };
        let vol = if n > 1.0 {
            (daily_returns.iter().map(|r| (r - avg).powi(2)).sum::<f64>() / (n - 1.0)).sqrt()
        } else { 1.0 };
        let sharpe = if vol > 0.0 { avg / vol * (252.0f64).sqrt() } else { 0.0 };

        // Read POSITIONS for symbols traded
        let positions_path = entry.path().join("POSITIONS");
        let mut symbols = Vec::new();
        if positions_path.exists() {
            if let Ok(pos_content) = std::fs::read_to_string(&positions_path) {
                let mut sym_set = std::collections::HashSet::new();
                for line in pos_content.lines() {
                    // Find symbol names in ['SYMBOL', ...] patterns
                    for part in line.split("'") {
                        if part.len() >= 2 && part.len() <= 10 && part.chars().all(|c| c.is_ascii_uppercase() || c == '/') {
                            sym_set.insert(part.to_string());
                        }
                    }
                }
                symbols = sym_set.into_iter().collect();
                symbols.sort();
            }
        }

        let score = sharpe * 0.4 + total_return * 0.3 - max_dd * 0.3;

        results.push(DarwinScreenResult {
            ticker,
            return_pct: total_return,
            max_drawdown: max_dd,
            sharpe,
            trading_days: lines.len() as i64,
            avg_trades_per_day: 0.0, // would need TRADES file
            symbols_traded: symbols,
            score,
        });
    }

    // Sort by score descending
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);
    Ok(results)
}

/// Export symbol radar data in MarketWizardry format.
/// Reads MT5 specs from SQLite and generates the .txt report.
pub fn export_radar_txt(_conn: &Connection, cache_conn: &Connection, output_dir: &str) -> Result<String, String> {
    // This reads the __SPECS__ key from kv_cache to get current symbol data
    // and compares with previous snapshots if available
    let specs_json: Option<String> = {
        let mut stmt = cache_conn.prepare(
            "SELECT value FROM kv_cache WHERE key LIKE '%__SPECS__%' LIMIT 1"
        ).map_err(|e| format!("Prepare failed: {e}"))?;

        match stmt.query_row([], |row| {
            let data: Vec<u8> = row.get(0)?;
            Ok(data)
        }) {
            Ok(compressed) => {
                let decompressed = zstd::decode_all(compressed.as_slice())
                    .map_err(|e| format!("Decompress failed: {e}"))?;
                Some(String::from_utf8(decompressed)
                    .map_err(|e| format!("UTF-8 failed: {e}"))?)
            }
            Err(_) => None,
        }
    };

    if specs_json.is_none() {
        return Err("No MT5 specs data found in cache. Run MT5 sync first.".into());
    }

    let specs = specs_json.unwrap();
    let dir = std::path::Path::new(output_dir);
    std::fs::create_dir_all(dir).map_err(|e| format!("Create dir failed: {e}"))?;

    // Write specs snapshot for radar tracking
    let timestamp = chrono::Utc::now().format("%Y.%m.%d").to_string();
    let output_path = dir.join(format!("SymbolsExport-Darwinex-Live-All-{}.json", timestamp));
    std::fs::write(&output_path, &specs)
        .map_err(|e| format!("Write failed: {e}"))?;

    Ok(format!("{{\"exported\":\"{}\",\"size\":{}}}", output_path.display(), specs.len()))
}

// ── FTP Quote / Price Series ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarwinQuoteBar {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

/// Build a synthetic OHLC price series from a DARWIN's FTP RETURN file.
///
/// The RETURN file contains one line per day:
///   `timestamp,experience_score,[cumulative_return_values...]`
/// where return values are multipliers (1.0 = starting point).
///
/// We convert to a price series starting at 100.0.  Each day's intra-day
/// values become the high/low; first value = open, last value = close.
/// The `timeframe` parameter controls aggregation: "1Day", "1Week", or
/// "1Month".
pub fn get_darwin_price_series(
    ftp_path: &str,
    darwin_ticker: &str,
    timeframe: &str,
) -> Result<Vec<DarwinQuoteBar>, String> {
    let return_path = std::path::Path::new(ftp_path)
        .join(darwin_ticker)
        .join("RETURN");

    if !return_path.exists() {
        return Err(format!(
            "RETURN file not found: {}",
            return_path.display()
        ));
    }

    let content = std::fs::read_to_string(&return_path)
        .map_err(|e| format!("Read RETURN failed: {e}"))?;

    let base_price = 100.0f64;

    // Parse each line into a daily bar
    let mut daily_bars: Vec<DarwinQuoteBar> = Vec::new();
    let mut prev_close = base_price;

    for line in content.lines() {
        let parts: Vec<&str> = line.splitn(3, ',').collect();
        if parts.len() < 3 {
            continue;
        }

        let timestamp = parts[0].trim();
        // Extract date portion (YYYY-MM-DD) from timestamp
        let date = if timestamp.len() >= 10 {
            &timestamp[..10]
        } else {
            timestamp
        };

        let vals_str = parts[2]
            .trim_start_matches('[')
            .trim_end_matches(']');

        let values: Vec<f64> = vals_str
            .split(',')
            .filter_map(|s| s.trim().parse::<f64>().ok())
            .collect();

        if values.is_empty() {
            continue;
        }

        let prices: Vec<f64> = values.iter().map(|v| v * base_price).collect();

        let open = prev_close;
        let close = *prices.last().unwrap();
        let mut high = open.max(close);
        let mut low = open.min(close);
        for &p in &prices {
            if p > high { high = p; }
            if p < low { low = p; }
        }

        prev_close = close;

        daily_bars.push(DarwinQuoteBar {
            date: date.to_string(),
            open,
            high,
            low,
            close,
        });
    }

    // Aggregate by timeframe
    match timeframe {
        "1Day" => Ok(daily_bars),
        "1Week" => Ok(aggregate_bars(&daily_bars, |d| {
            // ISO week: group by YYYY-Www
            week_key(d)
        })),
        "1Month" => Ok(aggregate_bars(&daily_bars, |d| {
            if d.len() >= 7 { d[..7].to_string() } else { d.to_string() }
        })),
        _ => Err(format!("Unsupported timeframe: {timeframe}. Use 1Day, 1Week, or 1Month.")),
    }
}

/// Aggregate daily bars into larger periods using a key function.
fn aggregate_bars<F>(bars: &[DarwinQuoteBar], key_fn: F) -> Vec<DarwinQuoteBar>
where
    F: Fn(&str) -> String,
{
    if bars.is_empty() {
        return Vec::new();
    }

    let mut result: Vec<DarwinQuoteBar> = Vec::new();
    let mut current_key = key_fn(&bars[0].date);
    let mut open = bars[0].open;
    let mut high = bars[0].high;
    let mut low = bars[0].low;
    let mut close = bars[0].close;
    let mut date = bars[0].date.clone();

    for bar in bars.iter().skip(1) {
        let k = key_fn(&bar.date);
        if k == current_key {
            if bar.high > high { high = bar.high; }
            if bar.low < low { low = bar.low; }
            close = bar.close;
        } else {
            result.push(DarwinQuoteBar { date: date.clone(), open, high, low, close });
            current_key = k;
            date = bar.date.clone();
            open = bar.open;
            high = bar.high;
            low = bar.low;
            close = bar.close;
        }
    }
    result.push(DarwinQuoteBar { date, open, high, low, close });
    result
}

/// Derive an ISO-week key "YYYY-Www" from a "YYYY-MM-DD" date string.
fn week_key(date: &str) -> String {
    if date.len() < 10 {
        return date.to_string();
    }
    // Parse year, month, day
    let y: i32 = date[..4].parse().unwrap_or(0);
    let m: u32 = date[5..7].parse().unwrap_or(1);
    let d: u32 = date[8..10].parse().unwrap_or(1);

    // Day-of-year
    let is_leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let mdays: [u32; 12] = [31, if is_leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let doy: u32 = mdays[..(m as usize - 1)].iter().sum::<u32>() + d;

    // Day of week (Mon=1 .. Sun=7) via Tomohiko Sakamoto
    let t = [0i32, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let yy = if m < 3 { y - 1 } else { y };
    let dow_sun0 = (yy + yy / 4 - yy / 100 + yy / 400 + t[(m - 1) as usize] + d as i32) % 7; // 0=Sun
    let dow_mon1 = if dow_sun0 == 0 { 7u32 } else { dow_sun0 as u32 }; // 1=Mon..7=Sun

    let week = (doy + 7 - dow_mon1) / 7;
    format!("{y}-W{week:02}")
}

// ── Xorshift64 RNG ──────────────────────────────────────────────────

struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        Self { state: if seed == 0 { 0xDEAD_BEEF_CAFE_1337 } else { seed } }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Returns a usize in [0, n)
    fn next_usize(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

// ── Monte Carlo VaR Simulation ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloResult {
    pub simulations: i64,
    pub days_forward: i64,
    pub var_95: f64,
    pub var_99: f64,
    pub median_outcome: f64,
    pub worst_case: f64,
    pub best_case: f64,
    pub probability_of_loss: f64,
    pub percentiles: Vec<(i32, f64)>,
}

/// Run Monte Carlo simulation using daily return distribution.
/// Randomly samples (with replacement) `days_forward` daily returns per path,
/// cumulates them, and computes percentiles across all simulated outcomes.
pub fn monte_carlo_var(
    daily_returns: &[DailyReturn],
    days_forward: i64,
    simulations: i64,
) -> MonteCarloResult {
    let empty = MonteCarloResult {
        simulations, days_forward,
        var_95: 0.0, var_99: 0.0, median_outcome: 0.0,
        worst_case: 0.0, best_case: 0.0, probability_of_loss: 0.0,
        percentiles: vec![],
    };

    if daily_returns.len() < 2 || simulations <= 0 || days_forward <= 0 {
        return empty;
    }

    let n = daily_returns.len();
    let mut rng = Xorshift64::new(42);
    let mut outcomes: Vec<f64> = Vec::with_capacity(simulations as usize);

    for _ in 0..simulations {
        let mut cumulative = 0.0;
        for _ in 0..days_forward {
            let idx = rng.next_usize(n);
            cumulative += daily_returns[idx].return_pct;
        }
        outcomes.push(cumulative);
    }

    outcomes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let total = outcomes.len();
    let loss_count = outcomes.iter().filter(|&&x| x < 0.0).count();

    let percentile = |p: f64| -> f64 {
        let idx = ((p / 100.0) * (total as f64 - 1.0)).round() as usize;
        outcomes[idx.min(total - 1)]
    };

    let percentiles_list: Vec<(i32, f64)> = vec![
        (1, percentile(1.0)),
        (5, percentile(5.0)),
        (10, percentile(10.0)),
        (25, percentile(25.0)),
        (50, percentile(50.0)),
        (75, percentile(75.0)),
        (90, percentile(90.0)),
        (95, percentile(95.0)),
        (99, percentile(99.0)),
    ];

    MonteCarloResult {
        simulations,
        days_forward,
        var_95: -percentile(5.0),
        var_99: -percentile(1.0),
        median_outcome: percentile(50.0),
        worst_case: outcomes[0],
        best_case: outcomes[total - 1],
        probability_of_loss: loss_count as f64 / total as f64 * 100.0,
        percentiles: percentiles_list,
    }
}

// ── Stress Test ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestResult {
    pub scenario: String,
    pub description: String,
    pub market_drop_pct: f64,
    pub estimated_portfolio_impact: f64,
    pub estimated_portfolio_impact_pct: f64,
}

/// Run stress tests against historical crash scenarios.
/// Estimates portfolio impact based on portfolio beta (correlation with market)
/// scaled by annualized volatility.
pub fn run_stress_tests(conn: &Connection) -> Result<Vec<StressTestResult>, String> {
    let daily_returns = get_portfolio_daily_returns(conn)?;
    if daily_returns.len() < 10 {
        return Err("Insufficient daily returns for stress testing (need >= 10 days)".into());
    }

    let var_stats = compute_var(&daily_returns);
    let ann_vol = var_stats.annualized_vol;

    // Estimate portfolio beta: use vol ratio as proxy (portfolio vol / typical market vol ~16%)
    let market_vol = 16.0;
    let beta = if market_vol > 0.0 { ann_vol / market_vol } else { 1.0 };

    // Current portfolio balance (last known)
    let current_balance = daily_returns.last().map(|d| d.balance).unwrap_or(0.0);

    let scenarios = vec![
        ("2020 COVID Crash", "March 2020: 34% equity drawdown in 23 trading days", -34.0),
        ("2022 Rate Hikes", "2022 bear market: 25% drawdown over several months", -25.0),
        ("2008 GFC", "Global Financial Crisis: 57% peak-to-trough equity drawdown", -57.0),
        ("Flash Crash", "Sudden intraday 10% market drop with rapid partial recovery", -10.0),
        ("Tech Wreck 2000", "Dot-com bust: 78% drawdown concentrated in growth/tech", -78.0),
        ("Crypto Winter", "80% drawdown in crypto assets (2018/2022-style bear)", -80.0),
    ];

    let results = scenarios
        .into_iter()
        .map(|(name, desc, drop_pct)| {
            let impact_pct = drop_pct * beta;
            let impact_abs = current_balance * impact_pct / 100.0;
            StressTestResult {
                scenario: name.to_string(),
                description: desc.to_string(),
                market_drop_pct: drop_pct,
                estimated_portfolio_impact: impact_abs,
                estimated_portfolio_impact_pct: impact_pct,
            }
        })
        .collect();

    Ok(results)
}

// ── Sector Exposure ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorExposure {
    pub sector: String,
    pub symbols: Vec<String>,
    pub long_notional: f64,
    pub short_notional: f64,
    pub net_notional: f64,
    pub pct_of_portfolio: f64,
}

/// Aggregate open position notional by GICS sector.
/// Uses a hardcoded sector map for common symbols.
pub fn get_sector_exposure(conn: &Connection) -> Result<Vec<SectorExposure>, String> {
    fn classify_sector(symbol: &str) -> &'static str {
        let s = symbol.to_uppercase();
        // Strip trailing suffixes (.US, .NAS, etc.)
        let base: &str = s.split('.').next().unwrap_or(&s);
        match base {
            // Technology
            "AAPL" | "MSFT" | "GOOG" | "GOOGL" | "META" | "NVDA" | "AMD" | "INTC" | "TSM"
            | "AVGO" | "ADBE" | "CRM" | "ORCL" | "CSCO" | "QCOM" | "TXN" | "SHOP" | "SQ"
            | "SNOW" | "PLTR" | "NET" | "DDOG" | "MDB" | "CRWD" | "ZS" | "PANW" | "FTNT"
            | "NOW" | "UBER" | "ABNB" | "DASH" | "COIN" => "Technology",
            // Healthcare
            "JNJ" | "UNH" | "PFE" | "ABBV" | "MRK" | "LLY" | "TMO" | "ABT" | "BMY"
            | "AMGN" | "GILD" | "ISRG" | "MDT" | "SYK" | "REGN" | "VRTX" | "MRNA"
            | "BIIB" => "Healthcare",
            // Consumer
            "AMZN" | "TSLA" | "WMT" | "COST" | "HD" | "NKE" | "SBUX" | "MCD" | "PG"
            | "KO" | "PEP" | "PM" | "EL" | "CL" | "TGT" | "LOW" | "LULU" | "ROST"
            | "DG" | "DLTR" => "Consumer",
            // Financial
            "JPM" | "BAC" | "WFC" | "GS" | "MS" | "C" | "BLK" | "SCHW" | "AXP" | "V"
            | "MA" | "PYPL" | "BRK" | "BRKB" | "BRK.B" | "CB" | "MET" | "AIG" | "PRU"
            | "ICE" | "CME" => "Financial",
            // Industrial
            "BA" | "CAT" | "HON" | "UNP" | "UPS" | "RTX" | "LMT" | "GD" | "NOC" | "GE"
            | "MMM" | "DE" | "FDX" | "WM" | "EMR" | "ITW" => "Industrial",
            // Energy
            "XOM" | "CVX" | "COP" | "SLB" | "EOG" | "MPC" | "PSX" | "VLO" | "OXY"
            | "HAL" | "DVN" | "FANG" | "WTI" | "XTIUSD" | "XNGUSD" | "USO" | "XLE"
            | "UKOIL" | "USOIL" => "Energy",
            // Materials
            "LIN" | "APD" | "SHW" | "ECL" | "NEM" | "FCX" | "NUE" | "DOW" | "DD"
            | "XAUUSD" | "XAGUSD" | "GOLD" | "SILVER" | "COPPER" | "XCUUSD" | "XPTUSD"
            | "XPDUSD" => "Materials",
            // Real Estate
            "AMT" | "PLD" | "CCI" | "EQIX" | "SPG" | "O" | "DLR" | "PSA" | "WELL"
            | "AVB" => "Real Estate",
            // Utilities
            "NEE" | "DUK" | "SO" | "D" | "AEP" | "EXC" | "SRE" | "XEL" | "ED"
            | "WEC" => "Utilities",
            // Communication
            "DIS" | "CMCSA" | "NFLX" | "T" | "VZ" | "TMUS" | "ATVI" | "EA" | "TTWO"
            | "RBLX" | "SNAP" | "PINS" | "SPOT" => "Communication",
            // Crypto
            "BTCUSD" | "ETHUSD" | "SOLUSD" | "DOGEUSD" | "XRPUSD" | "ADAUSD" | "DOTUSD"
            | "AVAXUSD" | "MATICUSD" | "LINKUSD" | "UNIUSD" | "AAVEUSD" | "LTCUSD"
            | "BCHUSD" | "ATOMUSD" | "NEARUSD" | "OPUSD" | "ARBUSD" | "FILUSD"
            | "APTUSD" => "Crypto",
            // ETF/Index
            "SPY" | "QQQ" | "IWM" | "DIA" | "VTI" | "VOO" | "SPX" | "NDX" | "US500"
            | "US100" | "US30" | "US2000" | "USTEC" | "GER40" | "UK100" | "JPN225"
            | "FRA40" | "ESP35" | "AUS200" | "HK50" | "VIX" => "ETF/Index",
            // Forex (common pairs)
            _ if base.len() == 6
                && ["USD", "EUR", "GBP", "JPY", "CHF", "AUD", "NZD", "CAD"]
                    .iter()
                    .any(|c| base.starts_with(c) || base.ends_with(c)) =>
            {
                "Forex"
            }
            _ => "Other",
        }
    }

    // Get all open positions across all accounts
    let open_positions = get_portfolio_open_positions(conn)?;
    if open_positions.is_empty() {
        return Ok(vec![]);
    }

    // Compute total notional for percentage calculation
    let mut sector_map: std::collections::HashMap<String, (Vec<String>, f64, f64)> =
        std::collections::HashMap::new();

    let mut total_notional = 0.0f64;

    for pos in &open_positions {
        let sector = classify_sector(&pos.symbol).to_string();
        let notional = pos.notional.abs();
        total_notional += notional;

        let entry = sector_map
            .entry(sector)
            .or_insert_with(|| (Vec::new(), 0.0, 0.0));
        if !entry.0.contains(&pos.symbol) {
            entry.0.push(pos.symbol.clone());
        }
        if pos.side == "buy" {
            entry.1 += notional;
        } else {
            entry.2 += notional;
        }
    }

    let mut result: Vec<SectorExposure> = sector_map
        .into_iter()
        .map(|(sector, (symbols, long_n, short_n))| {
            let net = long_n - short_n;
            let pct = if total_notional > 0.0 {
                (long_n + short_n) / total_notional * 100.0
            } else {
                0.0
            };
            SectorExposure {
                sector,
                symbols,
                long_notional: long_n,
                short_notional: short_n,
                net_notional: net,
                pct_of_portfolio: pct,
            }
        })
        .collect();

    result.sort_by(|a, b| {
        b.pct_of_portfolio
            .partial_cmp(&a.pct_of_portfolio)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(result)
}

// ── VaR Forecast ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaRForecast {
    pub current_var_95: f64,
    pub projected_30d: f64,
    pub projected_60d: f64,
    pub projected_90d: f64,
    pub var_trend: String,
    pub days_until_threshold: Option<i64>,
}

/// Forecast VaR by fitting a linear trend to rolling 30-day VaR over the last 90 days.
/// Projects forward 30/60/90 days and estimates when VaR will exceed `threshold_pct`.
pub fn forecast_var(daily_returns: &[DailyReturn], threshold_pct: f64) -> VaRForecast {
    let empty = VaRForecast {
        current_var_95: 0.0, projected_30d: 0.0, projected_60d: 0.0, projected_90d: 0.0,
        var_trend: "stable".to_string(), days_until_threshold: None,
    };

    if daily_returns.len() < 60 {
        return empty;
    }

    // Compute rolling 30-day VaR (95%) over the last 90 days
    let n = daily_returns.len();
    let lookback = 90.min(n - 30);
    let mut rolling_vars: Vec<f64> = Vec::new();

    for i in (n - lookback)..n {
        if i < 30 {
            continue;
        }
        let window = &daily_returns[(i - 30)..i];
        let mut returns: Vec<f64> = window.iter().map(|d| d.return_pct).collect();
        returns.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx_95 = ((0.05) * (returns.len() as f64 - 1.0)).round() as usize;
        let var_95 = -returns[idx_95.min(returns.len() - 1)];
        rolling_vars.push(var_95);
    }

    if rolling_vars.is_empty() {
        return empty;
    }

    let current_var_95 = *rolling_vars.last().unwrap();

    // Linear regression: y = a + b*x where x is day index
    let m = rolling_vars.len() as f64;
    let sum_x: f64 = (0..rolling_vars.len()).map(|i| i as f64).sum();
    let sum_y: f64 = rolling_vars.iter().sum();
    let sum_xy: f64 = rolling_vars.iter().enumerate().map(|(i, y)| i as f64 * y).sum();
    let sum_x2: f64 = (0..rolling_vars.len()).map(|i| (i as f64) * (i as f64)).sum();

    let denom = m * sum_x2 - sum_x * sum_x;
    let (intercept, slope) = if denom.abs() > 1e-12 {
        let b = (m * sum_xy - sum_x * sum_y) / denom;
        let a = (sum_y - b * sum_x) / m;
        (a, b)
    } else {
        (current_var_95, 0.0)
    };

    let last_x = rolling_vars.len() as f64 - 1.0;
    let projected_30d = intercept + slope * (last_x + 30.0);
    let projected_60d = intercept + slope * (last_x + 60.0);
    let projected_90d = intercept + slope * (last_x + 90.0);

    let var_trend = if slope > 0.01 {
        "increasing".to_string()
    } else if slope < -0.01 {
        "decreasing".to_string()
    } else {
        "stable".to_string()
    };

    // Estimate days until VaR exceeds threshold
    let days_until_threshold = if slope > 1e-9 && current_var_95 < threshold_pct {
        let days = ((threshold_pct - current_var_95) / slope).ceil() as i64;
        if days > 0 && days < 3650 { Some(days) } else { None }
    } else {
        None
    };

    VaRForecast {
        current_var_95,
        projected_30d,
        projected_60d,
        projected_90d,
        var_trend,
        days_until_threshold,
    }
}

// ── Kelly Criterion ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KellyResult {
    pub win_rate: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub kelly_fraction: f64,
    pub half_kelly: f64,
    pub optimal_risk_pct: f64,
}

/// Compute Kelly criterion for a DARWIN based on closed position P/L.
/// Kelly: f = (p * b - q) / b  where p=win rate, q=loss rate, b=avg_win/avg_loss.
pub fn compute_kelly(conn: &Connection, darwin_ticker: &str) -> Result<KellyResult, String> {
    let mut stmt = conn
        .prepare(
            "SELECT profit FROM darwin_positions WHERE account = ?1 AND profit != 0.0",
        )
        .map_err(|e| format!("Prepare failed: {e}"))?;

    let profits: Vec<f64> = stmt
        .query_map(params![darwin_ticker], |row| row.get::<_, f64>(0))
        .map_err(|e| format!("Query failed: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    if profits.is_empty() {
        return Err("No closed positions found for Kelly calculation".into());
    }

    let wins: Vec<f64> = profits.iter().filter(|&&p| p > 0.0).copied().collect();
    let losses: Vec<f64> = profits.iter().filter(|&&p| p < 0.0).copied().collect();

    let total = profits.len() as f64;
    let win_count = wins.len() as f64;
    let loss_count = losses.len() as f64;

    let win_rate = win_count / total;
    let loss_rate = loss_count / total;

    let avg_win = if !wins.is_empty() {
        wins.iter().sum::<f64>() / win_count
    } else {
        0.0
    };

    let avg_loss = if !losses.is_empty() {
        (losses.iter().sum::<f64>() / loss_count).abs()
    } else {
        0.0
    };

    let b = if avg_loss > 0.0 { avg_win / avg_loss } else { 0.0 };

    let kelly_fraction = if b > 0.0 {
        (win_rate * b - loss_rate) / b
    } else {
        0.0
    };

    let half_kelly = kelly_fraction / 2.0;
    let optimal_risk_pct = (half_kelly * 100.0).max(0.0);

    Ok(KellyResult {
        win_rate: win_rate * 100.0,
        avg_win,
        avg_loss,
        kelly_fraction,
        half_kelly,
        optimal_risk_pct,
    })
}

// ── Consecutive Trade Dependency (Autocorrelation) ──────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutocorrelationResult {
    pub lag1: f64,
    pub lag2: f64,
    pub lag3: f64,
    pub lag5: f64,
    pub is_random: bool,
    pub interpretation: String,
}

/// Compute autocorrelation of trade P/L at various lags.
/// If |corr| < 0.05 at all lags, trades are considered independent (random).
pub fn compute_trade_autocorrelation(
    conn: &Connection,
    darwin_ticker: &str,
) -> Result<AutocorrelationResult, String> {
    let mut stmt = conn
        .prepare(
            "SELECT profit FROM darwin_positions WHERE account = ?1 ORDER BY open_time, id",
        )
        .map_err(|e| format!("Prepare failed: {e}"))?;

    let profits: Vec<f64> = stmt
        .query_map(params![darwin_ticker], |row| row.get::<_, f64>(0))
        .map_err(|e| format!("Query failed: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    if profits.len() < 10 {
        return Err("Insufficient trades for autocorrelation analysis (need >= 10)".into());
    }

    let n = profits.len();
    let mean = profits.iter().sum::<f64>() / n as f64;
    let variance: f64 = profits.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / n as f64;

    let autocorrelation = |lag: usize| -> f64 {
        if lag >= n || variance.abs() < 1e-15 {
            return 0.0;
        }
        let covariance: f64 = (0..(n - lag))
            .map(|i| (profits[i] - mean) * (profits[i + lag] - mean))
            .sum::<f64>()
            / (n - lag) as f64;
        covariance / variance
    };

    let lag1 = autocorrelation(1);
    let lag2 = autocorrelation(2);
    let lag3 = autocorrelation(3);
    let lag5 = autocorrelation(5);

    let threshold = 0.05;
    let is_random =
        lag1.abs() < threshold && lag2.abs() < threshold && lag3.abs() < threshold && lag5.abs() < threshold;

    let interpretation = if is_random {
        "Trade outcomes appear independent — no significant serial correlation detected. \
         Position sizing and risk models can assume trade independence."
            .to_string()
    } else {
        let max_lag = [(1, lag1.abs()), (2, lag2.abs()), (3, lag3.abs()), (5, lag5.abs())]
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(l, _)| *l)
            .unwrap_or(1);
        let direction = if autocorrelation(max_lag) > 0.0 { "positive" } else { "negative" };
        format!(
            "Significant {} autocorrelation detected at lag {}. \
             Consecutive trades show dependency — consider adjusting position sizing \
             after streaks.",
            direction, max_lag
        )
    };

    Ok(AutocorrelationResult {
        lag1,
        lag2,
        lag3,
        lag5,
        is_random,
        interpretation,
    })
}

/// Delete a DARWIN account and all its data.
pub fn delete_darwin_account(conn: &Connection, darwin_ticker: &str) -> Result<(), String> {
    conn.execute("DELETE FROM darwin_deals WHERE account = ?1", params![darwin_ticker])
        .map_err(|e| format!("Delete deals failed: {e}"))?;
    conn.execute("DELETE FROM darwin_positions WHERE account = ?1", params![darwin_ticker])
        .map_err(|e| format!("Delete positions failed: {e}"))?;
    conn.execute("DELETE FROM darwin_accounts WHERE darwin_ticker = ?1", params![darwin_ticker])
        .map_err(|e| format!("Delete account failed: {e}"))?;
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// Create an in-memory database with darwin tables and sample data.
    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        create_darwin_tables(&conn).unwrap();

        // Insert test account
        conn.execute(
            "INSERT INTO darwin_accounts (darwin_ticker, name, mt5_account, initial_balance, created_at, deal_count, position_count) VALUES ('TEST', 'Test_MT5', '1234567', 100000.0, 1700000000, 20, 10)",
            [],
        ).unwrap();

        // Insert test positions (closed trades)
        let positions = vec![
            ("2024.06.01 10:00:00", 1001, "AAPL", "buy",  100.0, 150.0, 0.0, 0.0, "2024.06.05 14:00:00", 155.0, -5.0, 0.0,  500.0),
            ("2024.06.02 14:30:00", 1002, "AAPL", "buy",  200.0, 151.0, 0.0, 0.0, "2024.06.03 10:00:00", 149.0, -10.0, 0.0, -400.0),
            ("2024.06.03 09:00:00", 1003, "MSFT", "sell", 50.0,  420.0, 0.0, 0.0, "2024.06.07 16:00:00", 415.0, -3.0, -1.0, 250.0),
            ("2024.06.04 11:00:00", 1004, "AAPL", "buy",  150.0, 148.0, 0.0, 0.0, "2024.06.04 15:00:00", 152.0, -7.5, 0.0,  600.0),
            ("2024.06.05 08:00:00", 1005, "TSLA", "sell", 30.0,  180.0, 0.0, 0.0, "2024.06.05 12:00:00", 185.0, -2.0, 0.0, -150.0),
            ("2024.06.06 10:00:00", 1006, "MSFT", "buy",  80.0,  418.0, 0.0, 0.0, "2024.06.10 14:00:00", 425.0, -4.0, -2.0, 560.0),
            ("2024.06.07 13:00:00", 1007, "AAPL", "sell", 100.0, 155.0, 0.0, 0.0, "2024.06.08 10:00:00", 158.0, -5.0, 0.0, -300.0),
            ("2024.06.10 09:30:00", 1008, "TSLA", "buy",  60.0,  175.0, 0.0, 0.0, "2024.06.12 11:00:00", 172.0, -3.0, 0.0, -180.0),
            ("2024.06.11 14:00:00", 1009, "AAPL", "buy",  300.0, 152.0, 0.0, 0.0, "2024.06.13 16:00:00", 156.0, -15.0, 0.0, 1200.0),
            ("2024.06.12 10:00:00", 1010, "MSFT", "sell", 40.0,  430.0, 0.0, 0.0, "2024.06.14 14:00:00", 428.0, -2.0, 0.0,  80.0),
        ];
        for (open_t, ticket, sym, ptype, vol, oprice, sl, tp, close_t, cprice, comm, swap, profit) in &positions {
            conn.execute(
                "INSERT INTO darwin_positions (account, open_time, position_ticket, symbol, pos_type, volume, open_price, sl, tp, close_time, close_price, commission, swap, profit) VALUES ('TEST', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![open_t, ticket, sym, ptype, vol, oprice, sl, tp, close_t, cprice, comm, swap, profit],
            ).unwrap();
        }

        // Insert test deals (balance + trade entries)
        let deals = vec![
            ("2024.06.01 00:00:00", 1, "", "balance", "", 0.0, 0.0, 0, 0.0, 0.0, 0.0, 100000.0, 100000.0),
            ("2024.06.01 10:00:00", 2, "AAPL", "buy", "in", 100.0, 150.0, 1001, -5.0, 0.0, 0.0, 0.0, 99995.0),
            ("2024.06.05 14:00:00", 3, "AAPL", "sell", "out", 100.0, 155.0, 1001, 0.0, 0.0, 0.0, 500.0, 100495.0),
            ("2024.06.02 14:30:00", 4, "AAPL", "buy", "in", 200.0, 151.0, 1002, -10.0, 0.0, 0.0, 0.0, 100485.0),
            ("2024.06.03 10:00:00", 5, "AAPL", "sell", "out", 200.0, 149.0, 1002, 0.0, 0.0, 0.0, -400.0, 100085.0),
            ("2024.06.03 09:00:00", 6, "MSFT", "sell", "in", 50.0, 420.0, 1003, -3.0, 0.0, 0.0, 0.0, 100082.0),
            ("2024.06.07 16:00:00", 7, "MSFT", "buy", "out", 50.0, 415.0, 1003, 0.0, 0.0, -1.0, 250.0, 100331.0),
            ("2024.06.04 11:00:00", 8, "AAPL", "buy", "in", 150.0, 148.0, 1004, -7.5, 0.0, 0.0, 0.0, 100323.5),
            ("2024.06.04 15:00:00", 9, "AAPL", "sell", "out", 150.0, 152.0, 1004, 0.0, 0.0, 0.0, 600.0, 100923.5),
            ("2024.06.10 09:30:00", 10, "TSLA", "buy", "in", 60.0, 175.0, 1008, -3.0, 0.0, 0.0, 0.0, 100920.5),
            // Leave TSLA open (no out deal for ticket 1008)
        ];
        for (time, deal_ticket, sym, dtype, dir, vol, price, order, comm, fee, swap, profit, balance) in &deals {
            conn.execute(
                "INSERT INTO darwin_deals (account, time, deal_ticket, symbol, deal_type, direction, volume, price, order_ticket, commission, fee, swap, profit, balance, comment) VALUES ('TEST', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, '')",
                params![time, deal_ticket, sym, dtype, dir, vol, price, order, comm, fee, swap, profit, balance],
            ).unwrap();
        }

        conn
    }

    #[test]
    fn test_create_tables() {
        let conn = Connection::open_in_memory().unwrap();
        assert!(create_darwin_tables(&conn).is_ok());
        // Verify tables exist
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM darwin_accounts", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_list_accounts() {
        let conn = setup_test_db();
        let accounts = list_darwin_accounts(&conn).unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].darwin_ticker, "TEST");
        assert_eq!(accounts[0].name, "Test_MT5");
        assert_eq!(accounts[0].initial_balance, 100000.0);
    }

    #[test]
    fn test_darwin_summary() {
        let conn = setup_test_db();
        let summary = get_darwin_summary(&conn, "TEST").unwrap();
        assert_eq!(summary.account.darwin_ticker, "TEST");
        assert!(summary.total_profit != 0.0);
        assert!(summary.win_count > 0);
        assert!(summary.loss_count > 0);
        assert!(summary.win_rate > 0.0 && summary.win_rate < 100.0);
        assert!(summary.symbols_traded.len() >= 2); // AAPL, MSFT, TSLA
    }

    #[test]
    fn test_open_positions() {
        let conn = setup_test_db();
        let open = get_darwin_open_positions(&conn, "TEST").unwrap();
        // TSLA buy 60 @ 175 should be open (no matching out deal)
        assert!(!open.is_empty());
        let tsla = open.iter().find(|p| p.symbol == "TSLA" && p.side == "buy");
        assert!(tsla.is_some());
        let tsla = tsla.unwrap();
        assert_eq!(tsla.total_volume, 60.0);
        assert!((tsla.avg_price - 175.0).abs() < 0.01);
    }

    #[test]
    fn test_daily_returns() {
        let conn = setup_test_db();
        let returns = get_daily_returns(&conn, "TEST").unwrap();
        assert!(!returns.is_empty());
        // First day should have 0 P/L (balance entry)
        // Subsequent days should have non-zero P/L
        let non_zero = returns.iter().filter(|r| r.pnl != 0.0).count();
        assert!(non_zero > 0);
    }

    #[test]
    fn test_compute_var() {
        let conn = setup_test_db();
        let returns = get_daily_returns(&conn, "TEST").unwrap();
        let var = compute_var(&returns);
        assert!(var.trading_days > 0);
        assert!(var.var_95 >= 0.0);
        assert!(var.var_99 >= var.var_95); // 99% VaR >= 95% VaR
    }

    #[test]
    fn test_monthly_returns() {
        let conn = setup_test_db();
        let returns = get_daily_returns(&conn, "TEST").unwrap();
        let monthly = get_monthly_returns(&returns);
        assert!(!monthly.is_empty());
        assert_eq!(monthly[0].year, 2024);
        assert_eq!(monthly[0].month, 6);
    }

    #[test]
    fn test_rolling_var() {
        // Need enough data points; our test data is small
        let returns = vec![
            DailyReturn { date: "2024.06.01".into(), pnl: 100.0, balance: 100100.0, return_pct: 0.1, drawdown_pct: 0.0 },
            DailyReturn { date: "2024.06.02".into(), pnl: -50.0, balance: 100050.0, return_pct: -0.05, drawdown_pct: 0.05 },
            DailyReturn { date: "2024.06.03".into(), pnl: 200.0, balance: 100250.0, return_pct: 0.2, drawdown_pct: 0.0 },
        ];
        let rolling = get_rolling_var(&returns, 2);
        assert_eq!(rolling.len(), 1); // 3 points - 2 window = 1
    }

    #[test]
    fn test_equity_curve() {
        let conn = setup_test_db();
        let curve = get_darwin_equity_curve(&conn, "TEST").unwrap();
        assert!(!curve.is_empty());
        // All balances should be positive
        for (_, balance) in &curve {
            assert!(*balance > 0.0);
        }
    }

    #[test]
    fn test_pnl_by_symbol() {
        let conn = setup_test_db();
        let pnl = get_darwin_pnl_by_symbol(&conn, "TEST").unwrap();
        assert!(!pnl.is_empty());
        // Should have AAPL, MSFT, TSLA
        let symbols: Vec<&str> = pnl.iter().map(|p| p.0.as_str()).collect();
        assert!(symbols.contains(&"AAPL"));
    }

    #[test]
    fn test_streak_analysis() {
        let conn = setup_test_db();
        let streaks = get_streak_analysis(&conn, "TEST").unwrap();
        assert!(streaks.max_win_streak >= 1);
        assert!(streaks.max_loss_streak >= 1);
        assert!(!streaks.streak_distribution.is_empty());
    }

    #[test]
    fn test_hourly_pnl() {
        let conn = setup_test_db();
        let hourly = get_hourly_pnl(&conn, "TEST").unwrap();
        assert_eq!(hourly.len(), 24);
        let active_hours = hourly.iter().filter(|h| h.trade_count > 0).count();
        assert!(active_hours > 0);
    }

    #[test]
    fn test_day_of_week() {
        let conn = setup_test_db();
        let dow = get_day_of_week_pnl(&conn, "TEST").unwrap();
        assert_eq!(dow.len(), 7);
        let active_days = dow.iter().filter(|d| d.trade_count > 0).count();
        assert!(active_days > 0);
    }

    #[test]
    fn test_hold_time() {
        let conn = setup_test_db();
        let hold = get_hold_time_stats(&conn, "TEST").unwrap();
        assert!(hold.avg_hold_hours > 0.0);
        assert!(hold.min_hold_hours >= 0.0);
        assert!(hold.max_hold_hours >= hold.min_hold_hours);
        assert!(!hold.buckets.is_empty());
    }

    #[test]
    fn test_symbol_rotation() {
        let conn = setup_test_db();
        let rotation = get_symbol_rotation(&conn, "TEST").unwrap();
        assert!(!rotation.is_empty());
        let aapl = rotation.iter().find(|r| r.symbol == "AAPL");
        assert!(aapl.is_some());
        assert!(aapl.unwrap().trade_count >= 3);
    }

    #[test]
    fn test_sizing_efficiency() {
        let conn = setup_test_db();
        let sizing = get_sizing_efficiency(&conn, "TEST").unwrap();
        assert!(!sizing.is_empty());
        // Should have up to 4 quartiles
        assert!(sizing.len() <= 4);
        for q in &sizing {
            assert!(q.trade_count > 0);
            assert!(q.avg_volume > 0.0);
        }
    }

    #[test]
    fn test_cost_analysis() {
        let conn = setup_test_db();
        let costs = get_cost_analysis(&conn, "TEST").unwrap();
        assert!(costs.total_commission < 0.0); // commissions are negative
        assert!(!costs.commission_per_symbol.is_empty());
    }

    #[test]
    fn test_monte_carlo() {
        // Generate enough synthetic data for Monte Carlo
        let mut returns = Vec::new();
        let mut balance = 100000.0;
        for i in 0..100 {
            let pnl = if i % 3 == 0 { -200.0 } else { 150.0 };
            balance += pnl;
            let ret_pct = pnl / (balance - pnl) * 100.0;
            returns.push(DailyReturn {
                date: format!("2024.06.{:02}", (i % 28) + 1),
                pnl, balance, return_pct: ret_pct, drawdown_pct: 0.0,
            });
        }
        let mc = monte_carlo_var(&returns, 10, 1000);
        assert_eq!(mc.simulations, 1000);
        assert_eq!(mc.days_forward, 10);
        // probability_of_loss is 0..100 (percentage)
        assert!(mc.probability_of_loss >= 0.0 && mc.probability_of_loss <= 100.0);
        assert!(!mc.percentiles.is_empty());
        assert!(mc.best_case >= mc.worst_case);
    }

    #[test]
    fn test_stress_tests() {
        let conn = setup_test_db();
        // Stress tests need >= 10 daily returns. Add more deals.
        let mut balance = 100920.5;
        for i in 0..20 {
            let pnl = if i % 2 == 0 { 500.0 } else { -300.0 };
            balance += pnl;
            conn.execute(
                "INSERT INTO darwin_deals (account, time, deal_ticket, symbol, deal_type, direction, volume, price, order_ticket, commission, fee, swap, profit, balance, comment) VALUES ('TEST', ?1, ?2, 'AAPL', 'buy', 'in', 100.0, 150.0, 0, 0.0, 0.0, 0.0, ?3, ?4, '')",
                params![format!("2024.07.{:02} 10:00:00", i + 1), 100 + i, pnl, balance],
            ).unwrap();
        }
        let result = run_stress_tests(&conn);
        assert!(result.is_ok());
        let tests = result.unwrap();
        assert!(!tests.is_empty());
        for t in &tests {
            assert!(!t.scenario.is_empty());
            assert!(t.market_drop_pct < 0.0);
        }
    }

    #[test]
    fn test_kelly() {
        let conn = setup_test_db();
        let kelly = compute_kelly(&conn, "TEST").unwrap();
        // win_rate is stored as percentage (0..100)
        assert!(kelly.win_rate >= 0.0 && kelly.win_rate <= 100.0);
        assert!(kelly.avg_win >= 0.0);
        assert!(kelly.avg_loss >= 0.0);
        assert!(kelly.kelly_fraction > -2.0 && kelly.kelly_fraction < 2.0);
    }

    #[test]
    fn test_autocorrelation() {
        let conn = setup_test_db();
        let ac = compute_trade_autocorrelation(&conn, "TEST").unwrap();
        // Autocorrelation should be between -1 and 1
        assert!(ac.lag1 >= -1.0 && ac.lag1 <= 1.0);
        assert!(ac.lag2 >= -1.0 && ac.lag2 <= 1.0);
        assert!(!ac.interpretation.is_empty());
    }

    #[test]
    fn test_var_forecast() {
        let returns = vec![
            DailyReturn { date: "d1".into(), pnl: 100.0, balance: 100100.0, return_pct: 0.1, drawdown_pct: 0.0 },
            DailyReturn { date: "d2".into(), pnl: -50.0, balance: 100050.0, return_pct: -0.05, drawdown_pct: 0.05 },
            DailyReturn { date: "d3".into(), pnl: 200.0, balance: 100250.0, return_pct: 0.2, drawdown_pct: 0.0 },
        ];
        let forecast = forecast_var(&returns, 10.0);
        assert!(forecast.current_var_95 >= 0.0);
        assert!(!forecast.var_trend.is_empty());
    }

    #[test]
    fn test_sector_exposure() {
        let conn = setup_test_db();
        let exposure = get_sector_exposure(&conn).unwrap();
        // Should classify TSLA (open position) into a sector
        if !exposure.is_empty() {
            for sec in &exposure {
                assert!(!sec.sector.is_empty());
            }
        }
    }

    #[test]
    fn test_trade_overlaps() {
        let conn = setup_test_db();
        // With only one account, there should be no overlaps
        let overlaps = get_trade_overlaps(&conn).unwrap();
        assert!(overlaps.is_empty());
    }

    #[test]
    fn test_correlations_single_account() {
        let conn = setup_test_db();
        let corr = get_darwin_correlations(&conn).unwrap();
        // With one account, self-correlation should be 1.0
        assert!(!corr.is_empty());
        let self_corr = corr.iter().find(|c| c.darwin_a == "TEST" && c.darwin_b == "TEST");
        assert!(self_corr.is_some());
        assert!((self_corr.unwrap().correlation - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_portfolio_summary() {
        let conn = setup_test_db();
        let summary = get_portfolio_summary(&conn).unwrap();
        assert_eq!(summary.accounts.len(), 1);
        assert!(summary.total_initial_balance > 0.0);
    }

    #[test]
    fn test_portfolio_open_positions() {
        let conn = setup_test_db();
        let positions = get_portfolio_open_positions(&conn).unwrap();
        // Should find TSLA open position
        let tsla = positions.iter().find(|p| p.symbol == "TSLA");
        assert!(tsla.is_some());
    }

    #[test]
    fn test_delete_account() {
        let conn = setup_test_db();
        assert!(delete_darwin_account(&conn, "TEST").is_ok());
        let accounts = list_darwin_accounts(&conn).unwrap();
        assert!(accounts.is_empty());
        // Deals and positions should be gone too
        let deal_count: i64 = conn.query_row("SELECT COUNT(*) FROM darwin_deals WHERE account = 'TEST'", [], |r| r.get(0)).unwrap();
        assert_eq!(deal_count, 0);
    }

    #[test]
    fn test_deals_query() {
        let conn = setup_test_db();
        let deals = get_darwin_deals(&conn, "TEST", None, None).unwrap();
        assert!(!deals.is_empty());
        // Filter by symbol
        let aapl_deals = get_darwin_deals(&conn, "TEST", Some("AAPL"), None).unwrap();
        assert!(aapl_deals.len() < deals.len());
    }

    #[test]
    fn test_positions_query() {
        let conn = setup_test_db();
        let positions = get_darwin_positions(&conn, "TEST", None, None).unwrap();
        assert_eq!(positions.len(), 10);
        let msft = get_darwin_positions(&conn, "TEST", Some("MSFT"), None).unwrap();
        assert!(msft.len() < positions.len());
    }

    #[test]
    fn test_portfolio_daily_returns() {
        let conn = setup_test_db();
        let returns = get_portfolio_daily_returns(&conn).unwrap();
        assert!(!returns.is_empty());
    }

    #[test]
    fn test_portfolio_equity_curve() {
        let conn = setup_test_db();
        let curve = get_portfolio_equity_curve(&conn).unwrap();
        assert!(!curve.is_empty());
    }
}
