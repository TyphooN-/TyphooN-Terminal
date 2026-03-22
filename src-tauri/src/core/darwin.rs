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
