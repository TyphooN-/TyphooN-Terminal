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
        CREATE INDEX IF NOT EXISTS idx_darwin_deals_direction ON darwin_deals(account, direction, symbol);
        CREATE INDEX IF NOT EXISTS idx_darwin_deals_balance ON darwin_deals(account, time, balance);
        CREATE INDEX IF NOT EXISTS idx_darwin_positions_account ON darwin_positions(account);
        CREATE INDEX IF NOT EXISTS idx_darwin_positions_symbol ON darwin_positions(account, symbol);
        CREATE INDEX IF NOT EXISTS idx_darwin_positions_time ON darwin_positions(account, open_time);

        CREATE TABLE IF NOT EXISTS darwin_equity_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            darwin_ticker TEXT NOT NULL,
            closed_balance REAL NOT NULL DEFAULT 0,
            unrealized_pnl REAL NOT NULL DEFAULT 0,
            floating_equity REAL NOT NULL DEFAULT 0,
            open_position_count INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (darwin_ticker) REFERENCES darwin_accounts(darwin_ticker)
        );
        CREATE INDEX IF NOT EXISTS idx_equity_snap_ticker ON darwin_equity_snapshots(darwin_ticker, timestamp);
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
/// MT5 exports XLSX with ALL XML/rels files as UTF-16 LE inside the zip.
/// calamine/quick-xml only handles UTF-8. This rewrites every UTF-16 BOM entry to UTF-8 in a temp copy.
fn fix_utf16_xlsx(xlsx_path: &str) -> Result<String, String> {
    use std::io::{Read, Write, Cursor};
    let data = std::fs::read(xlsx_path).map_err(|e| format!("Read failed: {e}"))?;

    // Check if ANY zip entry starts with UTF-16 BOM (FF FE)
    let needs_fix = {
        let mut r = zip::ZipArchive::new(Cursor::new(&data))
            .map_err(|e| format!("ZIP open failed: {e}"))?;
        let mut found = false;
        for i in 0..r.len() {
            if let Ok(mut entry) = r.by_index(i) {
                let mut buf = [0u8; 2];
                if entry.read_exact(&mut buf).is_ok() && buf == [0xFF, 0xFE] {
                    found = true;
                    break;
                }
            }
        }
        found
    };

    if !needs_fix {
        return Ok(xlsx_path.to_string()); // already UTF-8, use original
    }

    // Rewrite to temp file with all UTF-16 entries converted to UTF-8
    let tmp_path = format!("{}.utf8.xlsx", xlsx_path);
    #[cfg(unix)]
    let out_file = {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600)
            .open(&tmp_path).map_err(|e| format!("Create tmp failed: {e}"))?
    };
    #[cfg(not(unix))]
    let out_file = std::fs::File::create(&tmp_path).map_err(|e| format!("Create tmp failed: {e}"))?;
    let mut writer = zip::ZipWriter::new(out_file);
    let mut archive = zip::ZipArchive::new(Cursor::new(&data))
        .map_err(|e| format!("ZIP reopen failed: {e}"))?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| format!("ZIP entry {i} failed: {e}"))?;
        let name = entry.name().to_string();
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(entry.compression());
        writer.start_file(&name, opts).map_err(|e| format!("ZIP write failed: {e}"))?;

        let mut raw = Vec::new();
        entry.read_to_end(&mut raw).map_err(|e| format!("ZIP read failed: {e}"))?;

        // Convert ANY entry with UTF-16 LE BOM (.xml, .rels, [Content_Types].xml, etc.)
        if raw.len() >= 2 && raw[0] == 0xFF && raw[1] == 0xFE {
            let utf16: Vec<u16> = raw[2..].chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            let utf8 = String::from_utf16(&utf16)
                .map_err(|e| format!("UTF-16 decode failed in {name}: {e}"))?;
            writer.write_all(utf8.as_bytes()).map_err(|e| format!("Write failed: {e}"))?;
        } else {
            writer.write_all(&raw).map_err(|e| format!("Write failed: {e}"))?;
        }
    }
    writer.finish().map_err(|e| format!("ZIP finalize failed: {e}"))?;
    Ok(tmp_path)
}

pub fn import_darwin_xlsx(
    conn: &Connection,
    xlsx_path: &str,
    darwin_ticker: &str,
) -> Result<(String, usize, usize), String> {
    use calamine::{Reader, open_workbook, Xlsx};

    // MT5 exports UTF-16 LE XML inside XLSX — convert to UTF-8 if needed
    let effective_path = fix_utf16_xlsx(xlsx_path)?;
    let mut workbook: Xlsx<_> = open_workbook(&effective_path)
        .map_err(|e| format!("Failed to open XLSX: {e}"))?;
    // Clean up temp file after opening (workbook reads into memory)
    if effective_path != xlsx_path {
        let _ = std::fs::remove_file(&effective_path);
    }

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
/// Single SQL query instead of N per-account queries.
pub fn get_portfolio_equity_curve(conn: &Connection) -> Result<Vec<(String, f64)>, String> {
    let mut stmt = conn.prepare(
        "SELECT SUBSTR(d.time, 1, 10) as date, SUM(d.balance) as total_balance
         FROM darwin_deals d
         INNER JOIN (
           SELECT account, SUBSTR(time, 1, 10) as day, MAX(id) as max_id
           FROM darwin_deals
           WHERE balance > 0
           GROUP BY account, SUBSTR(time, 1, 10)
         ) g ON d.id = g.max_id
         GROUP BY date
         ORDER BY date"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    }).map_err(|e| format!("Query failed: {e}"))?;

    let mut result = Vec::new();
    for row in rows {
        if let Ok(point) = row { result.push(point); }
    }
    Ok(result)
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
/// Uses SQL GROUP BY to deduplicate at the DB level instead of loading all 62K deals.
pub fn get_daily_returns(conn: &Connection, darwin_ticker: &str) -> Result<Vec<DailyReturn>, String> {
    // SQL-level dedup: get last balance per day using MAX(id) subquery
    let mut stmt = conn.prepare(
        "SELECT SUBSTR(d.time, 1, 10) as date, d.balance
         FROM darwin_deals d
         INNER JOIN (
           SELECT SUBSTR(time, 1, 10) as day, MAX(id) as max_id
           FROM darwin_deals
           WHERE account = ?1 AND balance > 0
           GROUP BY SUBSTR(time, 1, 10)
         ) g ON d.id = g.max_id
         ORDER BY date"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let daily_balances: Vec<(String, f64)> = stmt.query_map(params![darwin_ticker], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    }).map_err(|e| format!("Query failed: {e}"))?
    .filter_map(|r| r.ok())
    .collect();

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
/// Compute VaR using a minimum 20-day (1 month) window.
/// For Darwinex-specific VaR, use compute_var_multipliers() which uses 45d+6m blend.
/// This function uses the last 20+ days if available, full history if less than 20 days.
pub fn compute_var(daily_returns: &[DailyReturn]) -> VaRResult {
    // Use at least 20 trading days (1 month) for meaningful VaR
    let windowed = if daily_returns.len() > 20 {
        &daily_returns[daily_returns.len() - 20..]
    } else {
        daily_returns
    };
    compute_var_full(windowed)
}

/// Compute VaR on the full provided dataset (no windowing).
/// Used internally and by functions that pre-window their data.
pub fn compute_var_full(daily_returns: &[DailyReturn]) -> VaRResult {
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
/// Uses a 45-day rolling window to match Darwinex's correlation methodology.
/// Darwinex calculates DARWIN correlation based on the last 45 trading days of returns.
pub fn get_darwin_correlations(conn: &Connection) -> Result<Vec<CorrelationEntry>, String> {
    let accounts = list_darwin_accounts(conn)?;
    let mut all_returns: Vec<(String, std::collections::HashMap<String, f64>)> = Vec::new();

    for account in &accounts {
        let returns = get_daily_returns(conn, &account.darwin_ticker)?;
        // Use only last 45 trading days (Darwinex standard correlation window)
        let recent = if returns.len() > 45 { &returns[returns.len() - 45..] } else { &returns };
        let map: std::collections::HashMap<String, f64> = recent.iter()
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
    // Read __SPECS__ from bar_cache (BarCacheWriter) or kv_cache (legacy)
    let try_table = |table: &str| -> Option<String> {
        // table/col are hardcoded constants, not user input — safe to format into SQL
        let (col, tbl) = if table == "bar_cache" { ("data", "bar_cache") } else { ("value", "kv_cache") };
        let sql = format!("SELECT {col} FROM {tbl} WHERE key LIKE '%__SPECS__%' LIMIT 1");
        let mut stmt = cache_conn.prepare(&sql).map_err(|e| {
            tracing::debug!("Failed to prepare specs query on {}: {}", table, e);
            e
        }).ok()?;
        let result = stmt.query_row([], |row| {
            row.get::<_, Vec<u8>>(0).or_else(|_| row.get::<_, String>(0).map(|s| s.into_bytes()))
        });
        match result {
            Ok(data) => {
                // Try zstd decompress, fall back to raw UTF-8
                if let Ok(d) = zstd::decode_all(data.as_slice()) {
                    String::from_utf8(d).ok()
                } else {
                    String::from_utf8(data).ok()
                }
            }
            Err(_) => None,
        }
    };
    let specs_json: Option<String> = try_table("bar_cache").or_else(|| try_table("kv_cache"));

    if specs_json.is_none() {
        return Err("No MT5 specs data found in cache. Run MT5 sync first.".into());
    }

    let specs = match specs_json {
        Some(s) => s,
        None => return Err("No MT5 specs data found in cache".into()),
    };
    let dir = std::path::Path::new(output_dir);
    std::fs::create_dir_all(dir).map_err(|e| format!("Create dir failed: {e}"))?;

    // Write specs snapshot for radar tracking
    let timestamp = chrono::Utc::now().format("%Y.%m.%d").to_string();

    // Parse the CSV specs and write in darwinex-radar compatible format
    // BarCacheWriter stores: Symbol,SectorName,IndustryName,TradeMode,SwapLong,SwapShort,Spread,
    //   VolumeMin,VolumeMax,VolumeStep,ContractSize,TickSize,TickValue,Digits,MarginInitial,
    //   MarginMaintenance,BaseCurrency,QuoteCurrency,Description
    let lines: Vec<&str> = specs.lines().collect();

    // Categorize symbols and write per-category CSV files for radar
    let mut stocks = Vec::new();
    let mut cfd = Vec::new();
    let mut crypto = Vec::new();
    let mut futures = Vec::new();

    // Header line (first line is the column header from BarCacheWriter)
    let _header = lines.first().copied().unwrap_or("");

    for line in lines.iter().skip(1) {
        if line.trim().is_empty() { continue; }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() < 4 { continue; }
        let symbol = fields[0].trim();
        let sector = if fields.len() > 1 { fields[1].trim() } else { "" };

        // Classify by sector/symbol pattern
        if sector.contains("Crypto") || symbol.ends_with("USD") && (symbol.starts_with("BTC") || symbol.starts_with("ETH") || symbol.starts_with("SOL") || symbol.starts_with("DOGE") || symbol.starts_with("XRP")) {
            crypto.push(*line);
        } else if symbol.contains('_') || symbol.starts_with("6") {
            futures.push(*line);
        } else if sector.is_empty() || sector == "Unknown" || sector == "Forex" || symbol.len() == 6 {
            cfd.push(*line);
        } else {
            stocks.push(*line);
        }
    }

    // Write semicolon-delimited CSVs in darwinex-radar format
    // Convert comma-separated to semicolon-separated
    let to_semicolon = |lines: &[&str]| -> String {
        let mut out = String::new();
        // Write header: Symbol;TradeMode;SwapLong;SwapShort;Spread (minimum radar columns)
        out.push_str("Symbol;TradeMode;SwapLong;SwapShort;Spread;SectorName;IndustryName;VolumeMin;VolumeMax;ContractSize;MarginInitial\n");
        for line in lines {
            let f: Vec<&str> = line.split(',').collect();
            if f.len() >= 7 {
                // Reorder: Symbol(0), TradeMode(3), SwapLong(4), SwapShort(5), Spread(6), Sector(1), Industry(2), VolMin(7), VolMax(8), Contract(10), Margin(14)
                out.push_str(&format!("{};{};{};{};{};{};{};{};{};{};{}\n",
                    f[0].trim(), f.get(3).unwrap_or(&"").trim(), f.get(4).unwrap_or(&"").trim(),
                    f.get(5).unwrap_or(&"").trim(), f.get(6).unwrap_or(&"").trim(),
                    f.get(1).unwrap_or(&"").trim(), f.get(2).unwrap_or(&"").trim(),
                    f.get(7).unwrap_or(&"").trim(), f.get(8).unwrap_or(&"").trim(),
                    f.get(10).unwrap_or(&"").trim(), f.get(14).unwrap_or(&"").trim(),
                ));
            }
        }
        out
    };

    let mut exported = Vec::new();
    if !stocks.is_empty() {
        let path = dir.join(format!("SymbolsExport-Darwinex-Live-Stocks-{}.csv", timestamp));
        std::fs::write(&path, to_semicolon(&stocks))
            .map_err(|e| format!("Write stocks failed: {e}"))?;
        exported.push(format!("stocks:{}", stocks.len()));
    }
    if !cfd.is_empty() {
        let path = dir.join(format!("SymbolsExport-Darwinex-Live-CFD-{}.csv", timestamp));
        std::fs::write(&path, to_semicolon(&cfd))
            .map_err(|e| format!("Write CFD failed: {e}"))?;
        exported.push(format!("cfd:{}", cfd.len()));
    }
    if !crypto.is_empty() {
        let path = dir.join(format!("SymbolsExport-Darwinex-Live-Crypto-{}.csv", timestamp));
        std::fs::write(&path, to_semicolon(&crypto))
            .map_err(|e| format!("Write crypto failed: {e}"))?;
        exported.push(format!("crypto:{}", crypto.len()));
    }
    if !futures.is_empty() {
        let path = dir.join(format!("SymbolsExport-Darwinex-Live-Futures-{}.csv", timestamp));
        std::fs::write(&path, to_semicolon(&futures))
            .map_err(|e| format!("Write futures failed: {e}"))?;
        exported.push(format!("futures:{}", futures.len()));
    }

    // Also write raw specs for debugging
    let raw_path = dir.join(format!("SymbolsExport-Darwinex-Live-All-{}.csv", timestamp));
    std::fs::write(&raw_path, &specs)
        .map_err(|e| format!("Write raw failed: {e}"))?;

    Ok(format!("Exported {} ({} total symbols)", exported.join(", "), lines.len() - 1))
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
        let close = *prices.last().unwrap_or(&100.0);
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

    let var_stats = compute_var_full(&daily_returns);
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
/// Classify a symbol into a sector. Public so other modules can reuse.
pub fn classify_sector(symbol: &str) -> &'static str {
    let s = symbol.to_uppercase();
    let base: &str = s.split('.').next().unwrap_or(&s);
    match base {
        "AAPL" | "MSFT" | "GOOG" | "GOOGL" | "META" | "NVDA" | "AMD" | "INTC" | "TSM"
        | "AVGO" | "ADBE" | "CRM" | "ORCL" | "CSCO" | "QCOM" | "TXN" | "SHOP" | "SQ"
        | "SNOW" | "PLTR" | "NET" | "DDOG" | "MDB" | "CRWD" | "ZS" | "PANW" | "FTNT"
        | "NOW" | "UBER" | "ABNB" | "DASH" | "COIN" => "Technology",
        "JNJ" | "UNH" | "PFE" | "ABBV" | "MRK" | "LLY" | "TMO" | "ABT" | "BMY"
        | "AMGN" | "GILD" | "ISRG" | "MDT" | "SYK" | "REGN" | "VRTX" | "MRNA"
        | "BIIB" => "Healthcare",
        "AMZN" | "TSLA" | "WMT" | "COST" | "HD" | "NKE" | "SBUX" | "MCD" | "PG"
        | "KO" | "PEP" | "PM" | "EL" | "CL" | "TGT" | "LOW" | "LULU" | "ROST"
        | "DG" | "DLTR" => "Consumer",
        "JPM" | "BAC" | "WFC" | "GS" | "MS" | "C" | "BLK" | "SCHW" | "AXP" | "V"
        | "MA" | "PYPL" | "BRK" | "BRKB" | "BRK.B" | "CB" | "MET" | "AIG" | "PRU"
        | "ICE" | "CME" => "Financial",
        "BA" | "CAT" | "HON" | "UNP" | "UPS" | "RTX" | "LMT" | "GD" | "NOC" | "GE"
        | "MMM" | "DE" | "FDX" | "WM" | "EMR" | "ITW" => "Industrial",
        "XOM" | "CVX" | "COP" | "SLB" | "EOG" | "MPC" | "PSX" | "VLO" | "OXY"
        | "HAL" | "DVN" | "FANG" | "WTI" | "XTIUSD" | "XNGUSD" | "USO" | "XLE"
        | "UKOIL" | "USOIL" => "Energy",
        "LIN" | "APD" | "SHW" | "ECL" | "NEM" | "FCX" | "NUE" | "DOW" | "DD"
        | "XAUUSD" | "XAGUSD" | "GOLD" | "SILVER" | "COPPER" | "XCUUSD" | "XPTUSD"
        | "XPDUSD" => "Materials",
        "AMT" | "PLD" | "CCI" | "EQIX" | "SPG" | "O" | "DLR" | "PSA" | "WELL"
        | "AVB" => "Real Estate",
        "NEE" | "DUK" | "SO" | "D" | "AEP" | "EXC" | "SRE" | "XEL" | "ED"
        | "WEC" => "Utilities",
        "DIS" | "CMCSA" | "NFLX" | "T" | "VZ" | "TMUS" | "ATVI" | "EA" | "TTWO"
        | "RBLX" | "SNAP" | "PINS" | "SPOT" => "Communication",
        "BTCUSD" | "ETHUSD" | "SOLUSD" | "DOGEUSD" | "XRPUSD" | "ADAUSD" | "DOTUSD"
        | "AVAXUSD" | "MATICUSD" | "LINKUSD" | "UNIUSD" | "AAVEUSD" | "LTCUSD"
        | "BCHUSD" | "ATOMUSD" | "NEARUSD" | "OPUSD" | "ARBUSD" | "FILUSD"
        | "APTUSD" => "Crypto",
        "SPY" | "QQQ" | "IWM" | "DIA" | "VTI" | "VOO" | "SPX" | "NDX" | "US500"
        | "US100" | "US30" | "US2000" | "USTEC" | "GER40" | "UK100" | "JPN225"
        | "FRA40" | "ESP35" | "AUS200" | "HK50" | "VIX" => "ETF/Index",
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

/// Get sector exposure across all DARWIN open positions.
pub fn get_sector_exposure(conn: &Connection) -> Result<Vec<SectorExposure>, String> {
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

    let current_var_95 = *rolling_vars.last().unwrap_or(&0.0);

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

// ── Seasonal Analysis ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalPattern {
    pub month: i32,
    pub month_name: String,
    pub avg_return_pct: f64,
    pub median_return_pct: f64,
    pub win_rate: f64,
    pub sample_count: i64,
    pub best_year: (i32, f64),
    pub worst_year: (i32, f64),
}

/// Group monthly returns by calendar month (Jan=1..Dec=12) across all years.
/// Computes avg/median return, win rate, best/worst year per month.
pub fn get_seasonal_analysis(daily_returns: &[DailyReturn]) -> Vec<SeasonalPattern> {
    let monthly = get_monthly_returns(daily_returns);
    let month_names = [
        "", "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December",
    ];

    // month -> Vec<(year, return_pct)>
    let mut by_month: std::collections::BTreeMap<i32, Vec<(i32, f64)>> = std::collections::BTreeMap::new();
    for mr in &monthly {
        by_month.entry(mr.month).or_default().push((mr.year, mr.return_pct));
    }

    by_month
        .into_iter()
        .map(|(month, entries)| {
            let n = entries.len();
            let avg_return_pct = entries.iter().map(|(_, r)| r).sum::<f64>() / n as f64;

            let mut sorted: Vec<f64> = entries.iter().map(|(_, r)| *r).collect();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let median_return_pct = if n % 2 == 0 && n > 0 {
                (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
            } else {
                sorted[n / 2]
            };

            let win_count = entries.iter().filter(|(_, r)| *r > 0.0).count();
            let win_rate = win_count as f64 / n as f64 * 100.0;

            let best = entries
                .iter()
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(y, r)| (*y, *r))
                .unwrap_or((0, 0.0));
            let worst = entries
                .iter()
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(y, r)| (*y, *r))
                .unwrap_or((0, 0.0));

            SeasonalPattern {
                month,
                month_name: month_names.get(month as usize).unwrap_or(&"Unknown").to_string(),
                avg_return_pct,
                median_return_pct,
                win_rate,
                sample_count: n as i64,
                best_year: best,
                worst_year: worst,
            }
        })
        .collect()
}

// ── MAE/MFE (Max Adverse/Favorable Excursion) ───────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MAEMFEResult {
    pub avg_mae_pct: f64,
    pub avg_mfe_pct: f64,
    pub mae_mfe_ratio: f64,
    pub entries: Vec<MAEMFEEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MAEMFEEntry {
    pub symbol: String,
    pub side: String,
    pub profit: f64,
    pub mae_pct: f64,
    pub mfe_pct: f64,
}

/// Estimate MAE/MFE for each closed position using daily volatility and hold time.
/// Since intraday data per trade is unavailable, we use:
///   mae ≈ daily_vol * sqrt(hold_days) * 1.5
///   mfe (winners) = profit + estimated adverse
///   mfe (losers) = |loss| * 0.5
pub fn estimate_mae_mfe(conn: &Connection, darwin_ticker: &str) -> Result<MAEMFEResult, String> {
    // Get daily volatility from returns
    let daily_returns = get_daily_returns(conn, darwin_ticker)?;
    let var_stats = compute_var(&daily_returns);
    let daily_vol = if var_stats.daily_vol > 0.0 { var_stats.daily_vol } else { 1.0 };

    let mut stmt = conn
        .prepare(
            "SELECT symbol, pos_type, volume, open_price, close_price, profit, open_time, close_time \
             FROM darwin_positions WHERE account = ?1 AND close_time != '' ORDER BY open_time",
        )
        .map_err(|e| format!("Prepare failed: {e}"))?;

    let rows = stmt
        .query_map(params![darwin_ticker], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, f64>(3)?,
                row.get::<_, f64>(4)?,
                row.get::<_, f64>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
            ))
        })
        .map_err(|e| format!("Query failed: {e}"))?;

    let mut entries = Vec::new();

    for row in rows {
        let (symbol, side, _volume, open_price, _close_price, profit, open_time, close_time) =
            row.map_err(|e| format!("Row failed: {e}"))?;

        let hold_days = match (parse_mt5_datetime(&open_time), parse_mt5_datetime(&close_time)) {
            (Some(open), Some(close)) => {
                let dur = close.signed_duration_since(open);
                (dur.num_hours() as f64 / 24.0).max(0.04) // minimum ~1 hour
            }
            _ => 1.0,
        };

        let notional = open_price; // per-unit basis
        let mae_pct = daily_vol * hold_days.sqrt() * 1.5;
        let profit_pct = if notional > 0.0 { profit / notional * 100.0 } else { 0.0 };

        let mfe_pct = if profit >= 0.0 {
            profit_pct.abs() + mae_pct
        } else {
            profit_pct.abs() * 0.5
        };

        entries.push(MAEMFEEntry {
            symbol,
            side,
            profit,
            mae_pct,
            mfe_pct,
        });
    }

    let n = entries.len().max(1) as f64;
    let avg_mae = entries.iter().map(|e| e.mae_pct).sum::<f64>() / n;
    let avg_mfe = entries.iter().map(|e| e.mfe_pct).sum::<f64>() / n;
    let ratio = if avg_mfe > 0.0 { avg_mae / avg_mfe } else { 0.0 };

    Ok(MAEMFEResult {
        avg_mae_pct: avg_mae,
        avg_mfe_pct: avg_mfe,
        mae_mfe_ratio: ratio,
        entries,
    })
}

// ── What-If Simulator ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatIfResult {
    pub action: String,
    pub current_portfolio_var: f64,
    pub new_portfolio_var: f64,
    pub var_change_pct: f64,
    pub current_notional: f64,
    pub new_notional: f64,
}

/// Compute current portfolio VaR, then recompute without the specified symbol's
/// positions to show the VaR impact of closing that symbol.
pub fn what_if_close_symbol(conn: &Connection, symbol: &str) -> Result<WhatIfResult, String> {
    let portfolio_returns = get_portfolio_daily_returns(conn)?;
    let current_var = compute_var(&portfolio_returns);

    let open_positions = get_portfolio_open_positions(conn)?;
    let current_notional: f64 = open_positions.iter().map(|p| p.notional.abs()).sum();

    // Notional of the symbol being removed
    let symbol_upper = symbol.to_uppercase();
    let removed_notional: f64 = open_positions
        .iter()
        .filter(|p| p.symbol.to_uppercase() == symbol_upper)
        .map(|p| p.notional.abs())
        .sum();

    if removed_notional == 0.0 {
        return Err(format!("No open positions found for symbol '{}'", symbol));
    }

    let new_notional = current_notional - removed_notional;

    // Scale VaR by notional reduction ratio (linear approximation)
    let scale = if current_notional > 0.0 { new_notional / current_notional } else { 1.0 };
    let new_var_95 = current_var.var_95 * scale;
    let var_change = if current_var.var_95 > 0.0 {
        (new_var_95 - current_var.var_95) / current_var.var_95 * 100.0
    } else {
        0.0
    };

    Ok(WhatIfResult {
        action: format!("Close all {} positions", symbol),
        current_portfolio_var: current_var.var_95,
        new_portfolio_var: new_var_95,
        var_change_pct: var_change,
        current_notional,
        new_notional,
    })
}

// ── Liquidity Risk ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityRisk {
    pub symbol: String,
    pub position_volume: f64,
    pub notional: f64,
    pub risk_tier: String,
    pub days_to_exit: f64,
    pub concentration_pct: f64,
}

/// Estimate liquidity risk for each open position based on position size
/// relative to typical daily volume estimates.
/// Volume tiers: mega-cap=50M, large-cap=10M, mid-cap=2M, small-cap=5M, micro=500K.
pub fn get_liquidity_risk(conn: &Connection) -> Result<Vec<LiquidityRisk>, String> {
    fn estimate_daily_volume(symbol: &str) -> f64 {
        let base = symbol.to_uppercase();
        let base: &str = base.split('.').next().unwrap_or(&base);
        match base {
            // Mega-cap: ~50M shares/day
            "AAPL" | "MSFT" | "NVDA" | "AMZN" | "TSLA" | "META" | "GOOG" | "GOOGL"
            | "SPY" | "QQQ" => 50_000_000.0,
            // Large-cap: ~10M shares/day
            "AMD" | "INTC" | "JPM" | "BAC" | "WFC" | "DIS" | "NFLX" | "BA" | "V"
            | "MA" | "PYPL" | "CRM" | "ADBE" | "ORCL" | "PFE" | "ABBV" | "JNJ"
            | "UNH" | "XOM" | "CVX" | "GS" | "MS" | "COIN" | "PLTR" | "UBER"
            | "IWM" | "VTI" | "VOO" => 10_000_000.0,
            // Mid-cap: ~2M shares/day
            "SHOP" | "SQ" | "SNOW" | "NET" | "DDOG" | "MDB" | "CRWD" | "ZS"
            | "PANW" | "ABNB" | "DASH" | "LULU" | "RBLX" | "SNAP" | "PINS"
            | "SPOT" => 2_000_000.0,
            // Small-cap known names: ~5M shares/day
            "CHGG" | "LAZR" | "LCID" | "RIVN" | "SOFI" | "MARA" | "RIOT"
            | "CLNE" | "BB" | "WKHS" | "CLOV" | "WISH" => 5_000_000.0,
            // Forex/Crypto/CFD — effectively unlimited liquidity
            _ if base.contains("USD") || base.contains("EUR") || base.contains("GBP")
                || base.contains("JPY") || base.contains("CHF") || base.contains("AUD")
                || base.contains("CAD") || base.contains("NZD")
                || base.starts_with("XAU") || base.starts_with("XAG")
                || base.starts_with("XTI") || base.starts_with("XNG")
                || base.starts_with("US5") || base.starts_with("US1")
                || base.starts_with("US3") || base.starts_with("GER")
                || base.starts_with("UK1") || base.starts_with("JPN") => 100_000_000.0,
            // Default micro-cap: 500K shares/day
            _ => 500_000.0,
        }
    }

    let open_positions = get_portfolio_open_positions(conn)?;
    if open_positions.is_empty() {
        return Ok(vec![]);
    }

    let total_notional: f64 = open_positions.iter().map(|p| p.notional.abs()).sum();

    let mut result: Vec<LiquidityRisk> = open_positions
        .iter()
        .map(|pos| {
            let daily_vol = estimate_daily_volume(&pos.symbol);
            let avg_price = if pos.avg_price > 0.0 { pos.avg_price } else { 1.0 };
            let daily_shares_value = daily_vol * avg_price;

            // Conservative: assume we can trade 10% of daily volume without impact
            let safe_daily_exit = daily_shares_value * 0.10;
            let days_to_exit = if safe_daily_exit > 0.0 {
                pos.notional.abs() / safe_daily_exit
            } else {
                999.0
            };

            let concentration_pct = if total_notional > 0.0 {
                pos.notional.abs() / total_notional * 100.0
            } else {
                0.0
            };

            let risk_tier = if days_to_exit < 0.1 {
                "LOW"
            } else if days_to_exit < 1.0 {
                "MEDIUM"
            } else if days_to_exit < 5.0 {
                "HIGH"
            } else {
                "EXTREME"
            }
            .to_string();

            LiquidityRisk {
                symbol: pos.symbol.clone(),
                position_volume: pos.total_volume,
                notional: pos.notional.abs(),
                risk_tier,
                days_to_exit,
                concentration_pct,
            }
        })
        .collect();

    result.sort_by(|a, b| {
        b.days_to_exit
            .partial_cmp(&a.days_to_exit)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(result)
}

// ── Tail Risk Dashboard ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TailRiskMetrics {
    pub skewness: f64,
    pub kurtosis: f64,
    pub tail_ratio: f64,
    pub gain_to_pain: f64,
    pub ulcer_index: f64,
    pub pain_index: f64,
    pub omega_ratio: f64,
    pub fat_tail_warning: bool,
}

/// Compute tail risk metrics: skewness, excess kurtosis, tail ratio,
/// gain-to-pain, ulcer index, pain index, omega ratio.
pub fn compute_tail_risk(daily_returns: &[DailyReturn]) -> TailRiskMetrics {
    if daily_returns.len() < 3 {
        return TailRiskMetrics {
            skewness: 0.0, kurtosis: 0.0, tail_ratio: 0.0, gain_to_pain: 0.0,
            ulcer_index: 0.0, pain_index: 0.0, omega_ratio: 0.0, fat_tail_warning: false,
        };
    }

    let rets: Vec<f64> = daily_returns.iter().map(|r| r.return_pct).collect();
    let n = rets.len() as f64;
    let mean = rets.iter().sum::<f64>() / n;
    let variance = rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let std_dev = variance.sqrt();

    // Skewness (sample)
    let skewness = if std_dev > 0.0 {
        let m3 = rets.iter().map(|r| ((r - mean) / std_dev).powi(3)).sum::<f64>();
        m3 * n / ((n - 1.0) * (n - 2.0).max(1.0))
    } else {
        0.0
    };

    // Excess kurtosis (normal = 0)
    let kurtosis = if std_dev > 0.0 && n > 3.0 {
        let m4 = rets.iter().map(|r| ((r - mean) / std_dev).powi(4)).sum::<f64>() / n;
        m4 - 3.0
    } else {
        0.0
    };

    // Tail ratio: 95th percentile gain / |5th percentile loss|
    let mut sorted = rets.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx_5 = ((n * 0.05).floor() as usize).min(sorted.len() - 1);
    let idx_95 = ((n * 0.95).floor() as usize).min(sorted.len() - 1);
    let p5 = sorted[idx_5];
    let p95 = sorted[idx_95];
    let tail_ratio = if p5.abs() > 1e-10 { p95 / p5.abs() } else { 0.0 };

    // Gain-to-pain: sum of all returns / sum of |negative returns|
    let total_return: f64 = rets.iter().sum();
    let total_pain: f64 = rets.iter().filter(|r| **r < 0.0).map(|r| r.abs()).sum();
    let gain_to_pain = if total_pain > 0.0 { total_return / total_pain } else { 0.0 };

    // Ulcer index: RMS of drawdowns
    let drawdowns: Vec<f64> = daily_returns.iter().map(|r| r.drawdown_pct).collect();
    let ulcer_index = if !drawdowns.is_empty() {
        (drawdowns.iter().map(|d| d.powi(2)).sum::<f64>() / drawdowns.len() as f64).sqrt()
    } else {
        0.0
    };

    // Pain index: mean of drawdowns
    let pain_index = if !drawdowns.is_empty() {
        drawdowns.iter().sum::<f64>() / drawdowns.len() as f64
    } else {
        0.0
    };

    // Omega ratio: sum of gains above threshold (0) / sum of losses below threshold
    let gains: f64 = rets.iter().filter(|r| **r > 0.0).sum();
    let losses: f64 = rets.iter().filter(|r| **r < 0.0).map(|r| r.abs()).sum();
    let omega_ratio = if losses > 0.0 { gains / losses } else { 0.0 };

    // Fat tail warning if excess kurtosis > 1.0
    let fat_tail_warning = kurtosis > 1.0;

    TailRiskMetrics {
        skewness,
        kurtosis,
        tail_ratio,
        gain_to_pain,
        ulcer_index,
        pain_index,
        omega_ratio,
        fat_tail_warning,
    }
}

// ── Trade Clustering (Burst Detection) ───────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingBurst {
    pub start_date: String,
    pub end_date: String,
    pub trade_count: i64,
    pub avg_trades_per_day: f64,
    pub total_pnl: f64,
    pub intensity: String,
}

/// Group trades by week, classify intensity based on trades/day relative
/// to the DARWIN's overall average.
pub fn detect_trading_bursts(
    conn: &Connection,
    darwin_ticker: &str,
) -> Result<Vec<TradingBurst>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT time, profit FROM darwin_deals \
             WHERE account = ?1 AND symbol != '' AND direction = 'in' \
             ORDER BY time",
        )
        .map_err(|e| format!("Prepare failed: {e}"))?;

    let rows: Vec<(String, f64)> = stmt
        .query_map(params![darwin_ticker], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        })
        .map_err(|e| format!("Query failed: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    if rows.is_empty() {
        return Ok(vec![]);
    }

    // Group by ISO week: (year, week_number) -> trades
    let mut weeks: std::collections::BTreeMap<(i32, u32), Vec<(String, f64)>> =
        std::collections::BTreeMap::new();

    for (time, profit) in &rows {
        if let Some(dt) = parse_mt5_datetime(time) {
            use chrono::Datelike;
            let iso = dt.date().iso_week();
            let key = (iso.year(), iso.week());
            weeks.entry(key).or_default().push((time.clone(), *profit));
        }
    }

    // Overall average trades per day
    let total_weeks = weeks.len().max(1) as f64;
    let total_trades: usize = weeks.values().map(|v| v.len()).sum();
    let avg_trades_per_week = total_trades as f64 / total_weeks;
    let avg_per_day = avg_trades_per_week / 5.0; // 5 trading days per week

    let mut bursts = Vec::new();
    for ((_year, _week), trades) in &weeks {
        let count = trades.len() as i64;
        let pnl: f64 = trades.iter().map(|(_, p)| p).sum();
        let start = trades.first().map(|(t, _)| t.clone()).unwrap_or_default();
        let end = trades.last().map(|(t, _)| t.clone()).unwrap_or_default();

        // Use 5 trading days per week
        let week_avg_per_day = count as f64 / 5.0;

        let intensity = if avg_per_day > 0.0 {
            let ratio = week_avg_per_day / avg_per_day;
            if ratio > 2.0 {
                "BURST"
            } else if ratio > 1.3 {
                "ACTIVE"
            } else {
                "NORMAL"
            }
        } else {
            "NORMAL"
        }
        .to_string();

        bursts.push(TradingBurst {
            start_date: start,
            end_date: end,
            trade_count: count,
            avg_trades_per_day: week_avg_per_day,
            total_pnl: pnl,
            intensity,
        });
    }

    Ok(bursts)
}

// ── Position Pyramiding Analysis ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PyramidingAnalysis {
    pub symbol: String,
    pub total_adds: i64,
    pub avg_add_interval_hours: f64,
    pub adds_in_profit: i64,
    pub adds_in_loss: i64,
    pub final_pnl: f64,
    pub strategy: String,
}

/// For each symbol, find sequences of same-direction "in" deals and classify
/// pyramiding behavior as SCALING_IN, AVERAGING_DOWN, or MIXED.
pub fn analyze_pyramiding(
    conn: &Connection,
    darwin_ticker: &str,
) -> Result<Vec<PyramidingAnalysis>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT symbol, deal_type, direction, price, profit, time \
             FROM darwin_deals \
             WHERE account = ?1 AND symbol != '' \
             ORDER BY symbol, time",
        )
        .map_err(|e| format!("Prepare failed: {e}"))?;

    let rows: Vec<(String, String, String, f64, f64, String)> = stmt
        .query_map(params![darwin_ticker], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, f64>(3)?,
                row.get::<_, f64>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|e| format!("Query failed: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    // Group "in" deals by symbol, tracking sequences of same-direction adds
    let mut symbol_deals: std::collections::BTreeMap<String, Vec<(String, String, f64, f64, String)>> =
        std::collections::BTreeMap::new();

    for (symbol, deal_type, direction, price, profit, time) in &rows {
        symbol_deals
            .entry(symbol.clone())
            .or_default()
            .push((deal_type.clone(), direction.clone(), *price, *profit, time.clone()));
    }

    let mut results = Vec::new();

    for (symbol, deals) in &symbol_deals {
        // Collect "in" deals (position entries/adds)
        let in_deals: Vec<&(String, String, f64, f64, String)> = deals
            .iter()
            .filter(|(_, dir, _, _, _)| dir == "in")
            .collect();

        if in_deals.len() < 2 {
            continue; // No pyramiding if only 1 entry
        }

        let total_adds = in_deals.len() as i64;

        // Compute average interval between adds
        let mut intervals = Vec::new();
        for i in 1..in_deals.len() {
            if let (Some(prev_dt), Some(curr_dt)) = (
                parse_mt5_datetime(&in_deals[i - 1].4),
                parse_mt5_datetime(&in_deals[i].4),
            ) {
                let hours = curr_dt.signed_duration_since(prev_dt).num_minutes() as f64 / 60.0;
                intervals.push(hours);
            }
        }
        let avg_interval = if !intervals.is_empty() {
            intervals.iter().sum::<f64>() / intervals.len() as f64
        } else {
            0.0
        };

        // Track running P/L to determine if adds were in profit or loss
        // Simple heuristic: compare each add's price to the first entry price
        let first_price = in_deals[0].2;
        let first_type = &in_deals[0].0; // "buy" or "sell"
        let is_long = first_type == "buy";

        let mut adds_in_profit = 0i64;
        let mut adds_in_loss = 0i64;

        for deal in in_deals.iter().skip(1) {
            let add_price = deal.2;
            let in_profit = if is_long {
                add_price > first_price // price went up for long
            } else {
                add_price < first_price // price went down for short
            };

            if in_profit {
                adds_in_profit += 1;
            } else {
                adds_in_loss += 1;
            }
        }

        // Total P/L for this symbol (sum of all deal profits including "out" deals)
        let final_pnl: f64 = deals.iter().map(|(_, _, _, p, _)| p).sum();

        let strategy = if adds_in_loss == 0 {
            "SCALING_IN"
        } else if adds_in_profit == 0 {
            "AVERAGING_DOWN"
        } else {
            "MIXED"
        }
        .to_string();

        results.push(PyramidingAnalysis {
            symbol: symbol.clone(),
            total_adds,
            avg_add_interval_hours: avg_interval,
            adds_in_profit,
            adds_in_loss,
            final_pnl,
            strategy,
        });
    }

    // Sort by total adds descending
    results.sort_by(|a, b| b.total_adds.cmp(&a.total_adds));
    Ok(results)
}

// ── Low-Correlation DARWIN Finder ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiversificationCandidate {
    pub ticker: String,
    pub avg_correlation: f64, // average correlation with user's DARWINs
    pub return_pct: f64,
    pub max_drawdown: f64,
    pub sharpe: f64,
}

/// Scan FTP DARWINs, compute correlation of each with the user's portfolio
/// daily returns, rank by lowest avg correlation + highest Sharpe.
pub fn find_low_correlation_darwins(
    conn: &Connection,
    ftp_path: &str,
    limit: usize,
) -> Result<Vec<DiversificationCandidate>, String> {
    // Gather user's portfolio daily returns per DARWIN
    let accounts = list_darwin_accounts(conn)?;
    if accounts.is_empty() {
        return Err("No DARWIN accounts — import at least one first".into());
    }

    let mut user_returns: Vec<(String, std::collections::HashMap<String, f64>)> = Vec::new();
    for acct in &accounts {
        let dr = get_daily_returns(conn, &acct.darwin_ticker)?;
        let map: std::collections::HashMap<String, f64> =
            dr.iter().map(|r| (r.date.clone(), r.return_pct)).collect();
        user_returns.push((acct.darwin_ticker.clone(), map));
    }

    let ftp_dir = std::path::Path::new(ftp_path);
    if !ftp_dir.exists() {
        return Err(format!("FTP path not found: {}", ftp_path));
    }

    let entries = std::fs::read_dir(ftp_dir)
        .map_err(|e| format!("Read dir failed: {e}"))?;

    // Collect portfolio combined daily returns keyed by date (sorted)
    let portfolio_returns = get_portfolio_daily_returns(conn)?;
    let mut portfolio_dates: Vec<String> =
        portfolio_returns.iter().map(|r| r.date.clone()).collect();
    portfolio_dates.sort();

    let mut candidates: Vec<DiversificationCandidate> = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let ticker = entry.file_name().to_str().unwrap_or("").to_string();
        if ticker.is_empty() || ticker.starts_with('.') { continue; }
        // Skip DARWINs already in portfolio
        if accounts.iter().any(|a| a.darwin_ticker == ticker) { continue; }

        let return_path = entry.path().join("RETURN");
        if !return_path.exists() { continue; }

        let content = match std::fs::read_to_string(&return_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let lines: Vec<&str> = content.lines().collect();
        if lines.len() < 60 { continue; } // need meaningful history

        // Build daily return series from RETURN file
        let mut ftp_daily: Vec<f64> = Vec::new();
        let mut prev_val = 1.0f64;
        let mut peak = 0.0f64;
        let mut max_dd = 0.0f64;
        let mut day_idx: usize = 0;

        let mut ftp_returns_by_date: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();

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
                    let ret = if prev_val > 0.0 { (val - prev_val) / prev_val * 100.0 } else { 0.0 };
                    ftp_daily.push(ret);
                    if day_idx < portfolio_dates.len() {
                        ftp_returns_by_date.insert(portfolio_dates[day_idx].clone(), ret);
                    }
                    prev_val = val;
                    day_idx += 1;
                }
            }
        }

        // Total return
        let total_return = (prev_val - 1.0) * 100.0;

        // Sharpe
        let rets: Vec<f64> = ftp_daily.iter().map(|r| r / 100.0).collect();
        let n = rets.len() as f64;
        let avg = if n > 0.0 { rets.iter().sum::<f64>() / n } else { 0.0 };
        let vol = if n > 1.0 {
            (rets.iter().map(|r| (r - avg).powi(2)).sum::<f64>() / (n - 1.0)).sqrt()
        } else { 1.0 };
        let sharpe = if vol > 0.0 { avg / vol * (252.0f64).sqrt() } else { 0.0 };

        // Compute average correlation with each user DARWIN
        let mut corr_sum = 0.0f64;
        let mut corr_count = 0usize;
        for (_name, user_map) in &user_returns {
            let mut pairs: Vec<(f64, f64)> = Vec::new();
            for (date, &ftp_ret) in &ftp_returns_by_date {
                if let Some(&user_ret) = user_map.get(date) {
                    pairs.push((ftp_ret, user_ret));
                }
            }
            if pairs.len() > 10 {
                let pn = pairs.len() as f64;
                let ma = pairs.iter().map(|(a, _)| a).sum::<f64>() / pn;
                let mb = pairs.iter().map(|(_, b)| b).sum::<f64>() / pn;
                let cov = pairs.iter().map(|(a, b)| (a - ma) * (b - mb)).sum::<f64>() / (pn - 1.0);
                let sa = (pairs.iter().map(|(a, _)| (a - ma).powi(2)).sum::<f64>() / (pn - 1.0)).sqrt();
                let sb = (pairs.iter().map(|(_, b)| (b - mb).powi(2)).sum::<f64>() / (pn - 1.0)).sqrt();
                let corr = if sa > 0.0 && sb > 0.0 { cov / (sa * sb) } else { 0.0 };
                corr_sum += corr;
                corr_count += 1;
            }
        }

        let avg_corr = if corr_count > 0 { corr_sum / corr_count as f64 } else { 0.0 };

        candidates.push(DiversificationCandidate {
            ticker,
            avg_correlation: avg_corr,
            return_pct: total_return,
            max_drawdown: max_dd,
            sharpe,
        });
    }

    // Rank: lowest avg correlation first, break ties by highest Sharpe
    candidates.sort_by(|a, b| {
        a.avg_correlation.partial_cmp(&b.avg_correlation)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.sharpe.partial_cmp(&a.sharpe).unwrap_or(std::cmp::Ordering::Equal))
    });
    candidates.truncate(limit);
    Ok(candidates)
}

// ── Investor Flow Analysis (from FTP) ───────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestorFlow {
    pub date: String,
    pub investors: f64,
    pub aum: f64,
}

/// Read INVESTMENT_CHART and INVESTORS_CHART from FTP for a given DARWIN.
/// Both files use the format: `timestamp,value\n`.
pub fn get_investor_flow(
    ftp_path: &str,
    darwin_ticker: &str,
) -> Result<Vec<InvestorFlow>, String> {
    let base = std::path::Path::new(ftp_path).join(darwin_ticker);
    if !base.exists() {
        return Err(format!("DARWIN {} not found in FTP: {}", darwin_ticker, ftp_path));
    }

    // Parse a timestamp,value file into a map keyed by date string
    fn parse_ts_file(path: &std::path::Path) -> std::collections::BTreeMap<String, f64> {
        let mut map = std::collections::BTreeMap::new();
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                let parts: Vec<&str> = line.splitn(2, ',').collect();
                if parts.len() == 2 {
                    if let (Ok(ts), Ok(val)) = (parts[0].trim().parse::<i64>(), parts[1].trim().parse::<f64>()) {
                        // Convert millis timestamp to YYYY-MM-DD
                        let secs = ts / 1000;
                        let dt = chrono::DateTime::from_timestamp(secs, 0);
                        if let Some(dt) = dt {
                            let date = dt.format("%Y-%m-%d").to_string();
                            map.insert(date, val);
                        }
                    }
                }
            }
        }
        map
    }

    let investment_map = parse_ts_file(&base.join("INVESTMENT_CHART"));
    let investors_map = parse_ts_file(&base.join("INVESTORS_CHART"));

    // Merge on date — use the union of all dates
    let mut all_dates: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    all_dates.extend(investment_map.keys().cloned());
    all_dates.extend(investors_map.keys().cloned());

    let mut result = Vec::new();
    let mut last_aum = 0.0f64;
    let mut last_inv = 0.0f64;
    for date in &all_dates {
        let aum = investment_map.get(date).copied().unwrap_or(last_aum);
        let investors = investors_map.get(date).copied().unwrap_or(last_inv);
        last_aum = aum;
        last_inv = investors;
        result.push(InvestorFlow {
            date: date.clone(),
            investors,
            aum,
        });
    }

    if result.is_empty() {
        return Err(format!(
            "No investor flow data for {} (checked INVESTMENT_CHART and INVESTORS_CHART)",
            darwin_ticker
        ));
    }
    Ok(result)
}

// ── D-Score Component Reader (from FTP) ─────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DScoreComponents {
    pub ticker: String,
    pub experience: Option<f64>,
    pub risk_stability: Option<f64>,
    pub risk_adjustment: Option<f64>,
    pub market_correlation: Option<f64>,
    pub winning_consistency: Option<f64>,
    pub losing_consistency: Option<f64>,
    pub performance: Option<f64>,
    pub scalability: Option<f64>,
}

/// Read each D-Score component file from the FTP directory for a DARWIN,
/// parse the last line's value.
pub fn get_dscore_components(
    ftp_path: &str,
    darwin_ticker: &str,
) -> Result<DScoreComponents, String> {
    let base = std::path::Path::new(ftp_path).join(darwin_ticker);
    if !base.exists() {
        return Err(format!("DARWIN {} not found in FTP: {}", darwin_ticker, ftp_path));
    }

    // Read a score file and return the last line's value (second field in "timestamp,value")
    fn read_last_value(path: &std::path::Path) -> Option<f64> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            tracing::debug!("Failed to read D-Score file {}: {}", path.display(), e);
            e
        }).ok()?;
        let last_line = content.lines().last()?;
        let parts: Vec<&str> = last_line.splitn(2, ',').collect();
        if parts.len() == 2 {
            parts[1].trim().parse::<f64>().ok()
        } else {
            last_line.trim().parse::<f64>().ok()
        }
    }

    Ok(DScoreComponents {
        ticker: darwin_ticker.to_string(),
        experience: read_last_value(&base.join("EXPERIENCE")),
        risk_stability: read_last_value(&base.join("RISK_STABILITY")),
        risk_adjustment: read_last_value(&base.join("RISK_ADJUSTMENT")),
        market_correlation: read_last_value(&base.join("MARKET_CORRELATION")),
        winning_consistency: read_last_value(&base.join("WINNING_CONSISTENCY")),
        losing_consistency: read_last_value(&base.join("LOSING_CONSISTENCY")),
        performance: read_last_value(&base.join("PERFORMANCE")),
        scalability: read_last_value(&base.join("SCALABILITY")),
    })
}

// ── Alert Conditions Check ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertCondition {
    pub alert_type: String, // "VAR_BREACH", "DRAWDOWN", "CONCENTRATION", "CORRELATION_SPIKE"
    pub severity: String,   // "INFO", "WARNING", "CRITICAL"
    pub message: String,
    pub value: f64,
    pub threshold: f64,
}

/// Check all alert conditions across the portfolio:
/// - VaR breach: portfolio VaR > $50K (configurable)
/// - Drawdown: any DARWIN DD > 15%
/// - Concentration: any symbol > 30% of portfolio
/// - Symbol overlap: symbol in 4+ DARWINs
/// - High correlation: any DARWIN pair > 0.8 correlation
pub fn check_alerts(conn: &Connection) -> Result<Vec<AlertCondition>, String> {
    let mut alerts: Vec<AlertCondition> = Vec::new();

    const VAR_THRESHOLD: f64 = 50_000.0;
    const DD_THRESHOLD: f64 = 15.0;
    const CONCENTRATION_THRESHOLD: f64 = 30.0;
    const OVERLAP_THRESHOLD: i64 = 4;
    const CORRELATION_THRESHOLD: f64 = 0.8;

    // 1. VaR breach — portfolio-level
    if let Ok(portfolio_dr) = get_portfolio_daily_returns(conn) {
        if portfolio_dr.len() > 2 {
            let var_result = compute_var(&portfolio_dr);
            if var_result.var_95 > VAR_THRESHOLD {
                alerts.push(AlertCondition {
                    alert_type: "VAR_BREACH".into(),
                    severity: if var_result.var_95 > VAR_THRESHOLD * 1.5 { "CRITICAL".into() } else { "WARNING".into() },
                    message: format!(
                        "Portfolio 95% VaR ${:.0} exceeds ${:.0} threshold",
                        var_result.var_95, VAR_THRESHOLD
                    ),
                    value: var_result.var_95,
                    threshold: VAR_THRESHOLD,
                });
            }
            if var_result.var_99 > VAR_THRESHOLD {
                alerts.push(AlertCondition {
                    alert_type: "VAR_BREACH".into(),
                    severity: "CRITICAL".into(),
                    message: format!(
                        "Portfolio 99% VaR ${:.0} exceeds ${:.0} threshold",
                        var_result.var_99, VAR_THRESHOLD
                    ),
                    value: var_result.var_99,
                    threshold: VAR_THRESHOLD,
                });
            }
        }
    }

    // 2. Drawdown — per DARWIN
    let accounts = list_darwin_accounts(conn)?;
    for acct in &accounts {
        if let Ok(dr) = get_daily_returns(conn, &acct.darwin_ticker) {
            if let Some(last) = dr.last() {
                if last.drawdown_pct > DD_THRESHOLD {
                    alerts.push(AlertCondition {
                        alert_type: "DRAWDOWN".into(),
                        severity: if last.drawdown_pct > DD_THRESHOLD * 1.5 { "CRITICAL".into() } else { "WARNING".into() },
                        message: format!(
                            "{} drawdown {:.1}% exceeds {:.0}% threshold",
                            acct.darwin_ticker, last.drawdown_pct, DD_THRESHOLD
                        ),
                        value: last.drawdown_pct,
                        threshold: DD_THRESHOLD,
                    });
                }
            }
        }
    }

    // 3. Concentration — any symbol > 30% of total notional
    if let Ok(exposure) = get_portfolio_exposure(conn) {
        let total_notional: f64 = exposure.iter().map(|e| e.net_notional.abs()).sum();
        if total_notional > 0.0 {
            for e in &exposure {
                let pct = e.net_notional.abs() / total_notional * 100.0;
                if pct > CONCENTRATION_THRESHOLD {
                    alerts.push(AlertCondition {
                        alert_type: "CONCENTRATION".into(),
                        severity: if pct > CONCENTRATION_THRESHOLD * 1.5 { "CRITICAL".into() } else { "WARNING".into() },
                        message: format!(
                            "{} is {:.1}% of portfolio notional (threshold {:.0}%)",
                            e.symbol, pct, CONCENTRATION_THRESHOLD
                        ),
                        value: pct,
                        threshold: CONCENTRATION_THRESHOLD,
                    });
                }

                // 4. Symbol overlap — symbol in 4+ DARWINs
                if e.darwin_count >= OVERLAP_THRESHOLD {
                    alerts.push(AlertCondition {
                        alert_type: "CONCENTRATION".into(),
                        severity: "WARNING".into(),
                        message: format!(
                            "{} held in {} DARWINs ({}) — overlap threshold {}",
                            e.symbol, e.darwin_count, e.darwins.join(", "), OVERLAP_THRESHOLD
                        ),
                        value: e.darwin_count as f64,
                        threshold: OVERLAP_THRESHOLD as f64,
                    });
                }
            }
        }
    }

    // 5. High correlation — any DARWIN pair > 0.8
    if let Ok(correlations) = get_darwin_correlations(conn) {
        for c in &correlations {
            if c.darwin_a != c.darwin_b && c.correlation > CORRELATION_THRESHOLD {
                // Only emit once per pair (A < B alphabetically)
                if c.darwin_a < c.darwin_b {
                    alerts.push(AlertCondition {
                        alert_type: "CORRELATION_SPIKE".into(),
                        severity: if c.correlation > 0.9 { "CRITICAL".into() } else { "WARNING".into() },
                        message: format!(
                            "{} / {} correlation {:.2} exceeds {:.1} threshold",
                            c.darwin_a, c.darwin_b, c.correlation, CORRELATION_THRESHOLD
                        ),
                        value: c.correlation,
                        threshold: CORRELATION_THRESHOLD,
                    });
                }
            }
        }
    }

    // Sort: CRITICAL first, then WARNING, then INFO
    alerts.sort_by(|a, b| {
        let sev_ord = |s: &str| match s { "CRITICAL" => 0, "WARNING" => 1, _ => 2 };
        sev_ord(&a.severity).cmp(&sev_ord(&b.severity))
    });

    Ok(alerts)
}

// ── DARWIN vs Benchmark ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkComparison {
    pub darwin_ticker: String,
    pub darwin_return: f64,
    pub darwin_sharpe: f64,
    pub darwin_max_dd: f64,
    pub benchmark_return: f64,
    pub alpha: f64, // excess return vs benchmark
    pub beta: f64,  // sensitivity to market
    pub information_ratio: f64,
    pub tracking_error: f64,
}

/// Compute alpha, beta, information ratio, tracking error of a DARWIN vs a benchmark.
/// Benchmark returns must be aligned by date with the DARWIN's daily returns.
pub fn compare_to_benchmark(
    conn: &Connection,
    darwin_ticker: &str,
    benchmark_returns: &[DailyReturn],
) -> Result<BenchmarkComparison, String> {
    let darwin_dr = get_daily_returns(conn, darwin_ticker)?;
    if darwin_dr.len() < 5 {
        return Err(format!("{} has fewer than 5 trading days", darwin_ticker));
    }

    let darwin_map: std::collections::HashMap<String, f64> =
        darwin_dr.iter().map(|r| (r.date.clone(), r.return_pct)).collect();
    let bench_map: std::collections::HashMap<String, f64> =
        benchmark_returns.iter().map(|r| (r.date.clone(), r.return_pct)).collect();

    // Align on common dates
    let mut pairs: Vec<(f64, f64)> = Vec::new(); // (darwin_ret, bench_ret)
    for (date, &d_ret) in &darwin_map {
        if let Some(&b_ret) = bench_map.get(date) {
            pairs.push((d_ret, b_ret));
        }
    }

    if pairs.len() < 5 {
        return Err(format!(
            "Only {} overlapping dates between {} and benchmark — need at least 5",
            pairs.len(), darwin_ticker
        ));
    }

    let n = pairs.len() as f64;
    let mean_d = pairs.iter().map(|(d, _)| d).sum::<f64>() / n;
    let mean_b = pairs.iter().map(|(_, b)| b).sum::<f64>() / n;

    // Beta = Cov(d, b) / Var(b)
    let cov = pairs.iter().map(|(d, b)| (d - mean_d) * (b - mean_b)).sum::<f64>() / (n - 1.0);
    let var_b = pairs.iter().map(|(_, b)| (b - mean_b).powi(2)).sum::<f64>() / (n - 1.0);
    let beta = if var_b > 0.0 { cov / var_b } else { 0.0 };

    // Alpha (annualized) = mean(darwin) - beta * mean(bench), scaled to annual
    let alpha = (mean_d - beta * mean_b) * 252.0;

    // Tracking error = annualized StdDev of (darwin - benchmark) returns
    let excess: Vec<f64> = pairs.iter().map(|(d, b)| d - b).collect();
    let mean_excess = excess.iter().sum::<f64>() / n;
    let te_var = excess.iter().map(|e| (e - mean_excess).powi(2)).sum::<f64>() / (n - 1.0);
    let tracking_error = te_var.sqrt() * (252.0f64).sqrt();

    // Information ratio = annualized excess return / tracking error
    let information_ratio = if tracking_error > 0.0 {
        mean_excess * 252.0 / tracking_error
    } else { 0.0 };

    // DARWIN stats (full history for accurate Sharpe / total return)
    let var_result = compute_var_full(&darwin_dr);
    let darwin_return = darwin_dr.last().map(|r| {
        let first_bal = darwin_dr.first().map(|f| f.balance - f.pnl).unwrap_or(1.0);
        if first_bal > 0.0 { (r.balance - first_bal) / first_bal * 100.0 } else { 0.0 }
    }).unwrap_or(0.0);

    // Benchmark total return
    let benchmark_return = if !benchmark_returns.is_empty() {
        let first_bal = benchmark_returns.first().map(|f| f.balance - f.pnl).unwrap_or(1.0);
        let last_bal = benchmark_returns.last().map(|r| r.balance).unwrap_or(1.0);
        if first_bal > 0.0 { (last_bal - first_bal) / first_bal * 100.0 } else { 0.0 }
    } else { 0.0 };

    Ok(BenchmarkComparison {
        darwin_ticker: darwin_ticker.to_string(),
        darwin_return,
        darwin_sharpe: var_result.sharpe,
        darwin_max_dd: var_result.max_drawdown_pct,
        benchmark_return,
        alpha,
        beta,
        information_ratio,
        tracking_error,
    })
}

// ── Margin Call Simulator ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarginCallSimulation {
    pub current_equity: f64,
    pub current_margin_used: f64,
    pub margin_level_pct: f64,
    pub days_to_margin_call_50: Option<i64>,
    pub days_to_margin_call_100: Option<i64>,
    pub probability_30d: f64,
    pub probability_90d: f64,
    pub worst_case_equity_30d: f64,
}

/// Simulate margin call risk using portfolio equity, estimated margin, and daily vol.
/// Estimates margin as ~10% of total long+short notional (CFD leverage ~10:1).
/// Uses Monte Carlo (1000 paths) to estimate probability of hitting margin call levels.
pub fn simulate_margin_call(conn: &Connection) -> Result<MarginCallSimulation, String> {
    // Estimate current equity from last deal balances across all accounts
    let accounts = list_darwin_accounts(conn)?;
    let mut total_equity = 0.0f64;
    for account in &accounts {
        let bal: f64 = conn.query_row(
            "SELECT COALESCE(MAX(balance), 0.0) FROM darwin_deals WHERE account = ?1 AND balance > 0",
            params![account.darwin_ticker],
            |row| row.get(0),
        ).unwrap_or(0.0);
        total_equity += bal;
    }

    // Estimate margin used as ~10% of total open notional
    let open_positions = get_portfolio_open_positions(conn)?;
    let total_notional: f64 = open_positions.iter().map(|p| p.notional.abs()).sum();
    let margin_used = total_notional * 0.10;

    let margin_level_pct = if margin_used > 0.0 {
        total_equity / margin_used * 100.0
    } else {
        f64::INFINITY
    };

    // Get portfolio daily returns for vol estimation
    let daily_returns = get_portfolio_daily_returns(conn)?;
    if daily_returns.len() < 5 || total_equity <= 0.0 {
        return Ok(MarginCallSimulation {
            current_equity: total_equity,
            current_margin_used: margin_used,
            margin_level_pct: if margin_level_pct.is_finite() { margin_level_pct } else { 0.0 },
            days_to_margin_call_50: None,
            days_to_margin_call_100: None,
            probability_30d: 0.0,
            probability_90d: 0.0,
            worst_case_equity_30d: total_equity,
        });
    }

    let returns: Vec<f64> = daily_returns.iter().map(|r| r.return_pct / 100.0).collect();
    let mean: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
    let var: f64 = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;
    let daily_vol = var.sqrt();

    // Deterministic days-to-margin-call using daily vol drawdown estimate
    let days_to_50 = if margin_used > 0.0 && daily_vol > 0.0 {
        // Equity must fall to 50% of margin_used
        let target_50 = margin_used * 0.50;
        let drop_needed = (total_equity - target_50).max(0.0) / total_equity;
        // At ~2-sigma daily moves: days ~ (drop / (2 * daily_vol))^2
        if drop_needed > 0.0 {
            let days = (drop_needed / (2.0 * daily_vol)).powi(2);
            Some(days.ceil() as i64)
        } else {
            Some(0)
        }
    } else {
        None
    };

    let days_to_100 = if margin_used > 0.0 && daily_vol > 0.0 {
        let target_100 = margin_used;
        let drop_needed = (total_equity - target_100).max(0.0) / total_equity;
        if drop_needed > 0.0 {
            let days = (drop_needed / (2.0 * daily_vol)).powi(2);
            Some(days.ceil() as i64)
        } else {
            Some(0)
        }
    } else {
        None
    };

    // Monte Carlo: 1000 simulations for 30d and 90d probability
    let n = returns.len();
    let mut rng = Xorshift64::new(0xDEAD_CAFE);
    let sims = 1000usize;
    let margin_call_level = margin_used; // 100% margin level

    let mut mc_30_hits = 0usize;
    let mut mc_90_hits = 0usize;
    let mut worst_30 = total_equity;

    for _ in 0..sims {
        let mut equity = total_equity;
        let mut hit_30 = false;
        let mut hit_90 = false;
        for day in 0..90 {
            let idx = rng.next_usize(n);
            equity *= 1.0 + returns[idx];
            if equity <= margin_call_level {
                if day < 30 && !hit_30 { hit_30 = true; }
                if !hit_90 { hit_90 = true; }
            }
            if day == 29 && equity < worst_30 {
                worst_30 = equity;
            }
        }
        if hit_30 { mc_30_hits += 1; }
        if hit_90 { mc_90_hits += 1; }
    }

    Ok(MarginCallSimulation {
        current_equity: total_equity,
        current_margin_used: margin_used,
        margin_level_pct: if margin_level_pct.is_finite() { margin_level_pct } else { 0.0 },
        days_to_margin_call_50: days_to_50,
        days_to_margin_call_100: days_to_100,
        probability_30d: mc_30_hits as f64 / sims as f64 * 100.0,
        probability_90d: mc_90_hits as f64 / sims as f64 * 100.0,
        worst_case_equity_30d: worst_30,
    })
}

// ── Slippage Analysis ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlippageStats {
    pub avg_slippage_pct: f64,
    pub total_slippage_cost: f64,
    pub worst_slippage: f64,
    pub by_symbol: Vec<(String, f64, i64)>,
    pub by_hour: Vec<(i32, f64, i64)>,
}

/// Analyze slippage by comparing position open_price vs deal entry price for matching tickets.
/// Groups results by symbol and by hour of day.
pub fn analyze_slippage(conn: &Connection, darwin_ticker: &str) -> Result<SlippageStats, String> {
    // Join positions with their entry deals: match on account + symbol + deal direction='in'
    // Position open_price is the intended price; deal price is the execution price.
    let mut stmt = conn.prepare(
        "SELECT p.symbol, p.open_price, d.price, d.volume, d.time
         FROM darwin_positions p
         JOIN darwin_deals d ON d.account = p.account AND d.symbol = p.symbol
           AND d.direction = 'in' AND d.deal_type = p.pos_type
         WHERE p.account = ?1 AND p.open_price > 0 AND d.price > 0"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    struct SlipRow {
        symbol: String,
        slippage_pct: f64,
        slippage_cost: f64,
        hour: i32,
    }

    let rows: Vec<SlipRow> = stmt.query_map(params![darwin_ticker], |row| {
        let symbol: String = row.get(0)?;
        let open_price: f64 = row.get(1)?;
        let deal_price: f64 = row.get(2)?;
        let volume: f64 = row.get(3)?;
        let time: String = row.get(4)?;

        let slippage_pct = if open_price > 0.0 {
            (deal_price - open_price) / open_price * 100.0
        } else {
            0.0
        };
        let slippage_cost = (deal_price - open_price) * volume;

        // Parse hour from time string "YYYY.MM.DD HH:MM:SS" or similar
        let hour = time.split(' ')
            .nth(1)
            .and_then(|t| t.split(':').next())
            .and_then(|h| h.parse::<i32>().ok())
            .unwrap_or(0);

        Ok(SlipRow { symbol, slippage_pct, slippage_cost, hour })
    }).map_err(|e| format!("Query failed: {e}"))?
    .filter_map(|r| r.ok())
    .collect();

    if rows.is_empty() {
        return Ok(SlippageStats {
            avg_slippage_pct: 0.0,
            total_slippage_cost: 0.0,
            worst_slippage: 0.0,
            by_symbol: vec![],
            by_hour: vec![],
        });
    }

    let total_count = rows.len() as f64;
    let avg_slippage_pct = rows.iter().map(|r| r.slippage_pct.abs()).sum::<f64>() / total_count;
    let total_slippage_cost = rows.iter().map(|r| r.slippage_cost.abs()).sum();
    let worst_slippage = rows.iter().map(|r| r.slippage_pct.abs()).fold(0.0f64, f64::max);

    // Group by symbol
    let mut sym_map: std::collections::HashMap<String, (f64, i64)> = std::collections::HashMap::new();
    for r in &rows {
        let entry = sym_map.entry(r.symbol.clone()).or_insert((0.0, 0));
        entry.0 += r.slippage_pct.abs();
        entry.1 += 1;
    }
    let mut by_symbol: Vec<(String, f64, i64)> = sym_map.into_iter()
        .map(|(sym, (total, count))| (sym, total / count as f64, count))
        .collect();
    by_symbol.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Group by hour
    let mut hour_map: std::collections::HashMap<i32, (f64, i64)> = std::collections::HashMap::new();
    for r in &rows {
        let entry = hour_map.entry(r.hour).or_insert((0.0, 0));
        entry.0 += r.slippage_pct.abs();
        entry.1 += 1;
    }
    let mut by_hour: Vec<(i32, f64, i64)> = hour_map.into_iter()
        .map(|(hour, (total, count))| (hour, total / count as f64, count))
        .collect();
    by_hour.sort_by_key(|h| h.0);

    Ok(SlippageStats {
        avg_slippage_pct,
        total_slippage_cost,
        worst_slippage,
        by_symbol,
        by_hour,
    })
}

// ── Optimal DARWIN Allocation (Mean-Variance) ───────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimalAllocation {
    pub darwin_ticker: String,
    pub current_weight: f64,
    pub optimal_weight: f64,
    pub sharpe_contribution: f64,
}

/// Compute inverse-volatility weighted optimal allocation across all DARWINs.
/// Compares to equal weight baseline and computes Sharpe contribution per DARWIN.
pub fn compute_optimal_allocation(conn: &Connection) -> Result<Vec<OptimalAllocation>, String> {
    let accounts = list_darwin_accounts(conn)?;
    if accounts.is_empty() {
        return Ok(vec![]);
    }

    let n = accounts.len() as f64;
    let equal_weight = 1.0 / n;

    struct DarwinStats {
        ticker: String,
        vol: f64,
        sharpe: f64,
    }

    let mut stats: Vec<DarwinStats> = Vec::new();
    for account in &accounts {
        let returns = get_daily_returns(conn, &account.darwin_ticker)?;
        let var_result = compute_var_full(&returns);
        let vol = if var_result.daily_vol > 0.0 { var_result.daily_vol } else { 1e-10 };
        stats.push(DarwinStats {
            ticker: account.darwin_ticker.clone(),
            vol,
            sharpe: var_result.sharpe,
        });
    }

    // Inverse-volatility weighting: weight_i = (1/vol_i) / sum(1/vol_j)
    let inv_vol_sum: f64 = stats.iter().map(|s| 1.0 / s.vol).sum();
    if inv_vol_sum <= 0.0 {
        return Ok(vec![]);
    }

    let result: Vec<OptimalAllocation> = stats.iter().map(|s| {
        let optimal_weight = (1.0 / s.vol) / inv_vol_sum;
        OptimalAllocation {
            darwin_ticker: s.ticker.clone(),
            current_weight: equal_weight,
            optimal_weight,
            sharpe_contribution: s.sharpe * optimal_weight,
        }
    }).collect();

    Ok(result)
}

// ── Portfolio Rebalancing Advisor ────────────────────────────────────
// Identifies correlated positions, suggests partial closes to reduce VaR,
// and recommends reallocations to uncorrelated symbols.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalanceSuggestion {
    pub action: String,          // "REDUCE", "INCREASE", "HOLD"
    pub darwin_ticker: String,
    pub symbol: String,
    pub side: String,
    pub current_volume: f64,
    pub suggested_volume: f64,   // new target volume
    pub reason: String,
    pub impact_var_pct: f64,     // estimated VaR impact (negative = improvement)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationPair {
    pub symbol_a: String,
    pub darwin_a: String,
    pub symbol_b: String,
    pub darwin_b: String,
    pub correlation: f64,
    pub combined_notional: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalanceDashboard {
    pub current_portfolio_var_95: f64,
    pub current_portfolio_var_99: f64,
    pub current_sharpe: f64,
    pub high_correlation_pairs: Vec<CorrelationPair>,
    pub suggestions: Vec<RebalanceSuggestion>,
    pub darwin_var: Vec<(String, f64, f64, f64)>, // (ticker, var95, sharpe, weight)
    pub optimal_allocation: Vec<OptimalAllocation>,
}

/// Analyze portfolio and suggest rebalancing trades to reduce VaR via decorrelation.
/// Only suggests REDUCE on positions that are in profit (current_price vs avg_price).
/// prices: live bid prices per symbol for mark-to-market profit check.
pub fn compute_rebalance_suggestions(
    conn: &Connection,
    prices: &std::collections::HashMap<String, f64>,
) -> Result<RebalanceDashboard, String> {
    let accounts = list_darwin_accounts(conn)?;
    if accounts.is_empty() {
        return Err("No DARWIN accounts imported".into());
    }

    // 1. Compute per-DARWIN VaR and portfolio VaR
    let portfolio_returns = get_portfolio_daily_returns(conn)?;
    let portfolio_var = compute_var(&portfolio_returns);

    let mut darwin_var_list = Vec::new();
    let mut darwin_returns_map: std::collections::HashMap<String, Vec<DailyReturn>> = std::collections::HashMap::new();
    let total_balance: f64 = {
        let mut sum = 0.0;
        for acct in &accounts {
            let returns = get_daily_returns(conn, &acct.darwin_ticker)?;
            let var = compute_var(&returns);
            let weight = if !returns.is_empty() { returns.last().unwrap().balance } else { 0.0 };
            darwin_var_list.push((acct.darwin_ticker.clone(), var.var_95, var.sharpe, weight));
            darwin_returns_map.insert(acct.darwin_ticker.clone(), returns);
            sum += weight;
        }
        sum
    };
    // Normalize weights
    for entry in &mut darwin_var_list {
        if total_balance > 0.0 { entry.3 /= total_balance; }
    }

    // 2. Cross-DARWIN correlation (fetched below with corr_lookup build)

    // 3. Symbol-level correlation across DARWINs (open positions)
    // Include profit info: (darwin, symbol, side, notional, avg_price, in_profit)
    // When no live prices are provided, assume all positions are actionable
    // (rebalance based on correlation risk, not profit state)
    let no_live_prices = prices.is_empty();
    let mut all_open: Vec<(String, String, String, f64, f64, bool)> = Vec::new();
    for acct in &accounts {
        let positions = get_darwin_open_positions(conn, &acct.darwin_ticker)?;
        for pos in &positions {
            let in_profit = if no_live_prices {
                true // No live prices → treat all as actionable for correlation-based suggestions
            } else {
                let current_price = prices.get(&pos.symbol).copied().unwrap_or(pos.avg_price);
                if pos.side == "buy" { current_price > pos.avg_price } else { current_price < pos.avg_price }
            };
            all_open.push((acct.darwin_ticker.clone(), pos.symbol.clone(), pos.side.clone(), pos.notional, pos.avg_price, in_profit));
        }
    }

    // Use actual 45-day return correlation between DARWINs (Darwinex methodology).
    // Build correlation lookup from the 45-day window.
    let correlations = get_darwin_correlations(conn)?;
    let corr_lookup: std::collections::HashMap<(String, String), f64> = correlations.iter()
        .map(|c| ((c.darwin_a.clone(), c.darwin_b.clone()), c.correlation))
        .collect();

    // Find correlated position pairs using actual DARWIN return correlation + same symbol detection.
    // Darwinex threshold: 0.95 correlation = highly correlated (silver/gold pool).
    let mut high_corr_pairs = Vec::new();
    for i in 0..all_open.len() {
        for j in (i+1)..all_open.len() {
            let (ref dw_a, ref sym_a, ref side_a, not_a, _, _) = all_open[i];
            let (ref dw_b, ref sym_b, ref side_b, not_b, _, _) = all_open[j];
            if dw_a == dw_b { continue; }
            if sym_a != sym_b { continue; } // only flag same-symbol pairs

            // Use actual 45-day DARWIN return correlation instead of assuming 1.0
            let darwin_corr = corr_lookup.get(&(dw_a.clone(), dw_b.clone())).copied().unwrap_or(0.0);
            let sign = if side_a == side_b { 1.0 } else { -1.0 }; // same side amplifies, opposite hedges
            let effective_corr = darwin_corr * sign;

            high_corr_pairs.push(CorrelationPair {
                symbol_a: sym_a.clone(), darwin_a: dw_a.clone(),
                symbol_b: sym_b.clone(), darwin_b: dw_b.clone(),
                correlation: effective_corr,
                combined_notional: if side_a == side_b { not_a + not_b } else { (not_a - not_b).abs() },
            });
        }
    }
    high_corr_pairs.sort_by(|a, b| b.combined_notional.partial_cmp(&a.combined_notional).unwrap_or(std::cmp::Ordering::Equal));

    // 4. Generate suggestions
    let mut suggestions = Vec::new();

    // Build deduplicated take-profit suggestions: one per (DARWIN, symbol).
    // Constraint: user can only act on 2-3 accounts at a time (50% of portfolio).
    // So we rank by profit magnitude and limit to top 3 actionable trades.
    // Build protection maps:
    // 1. How many DARWINs hold each symbol (on same side)? If only 1, never suggest reducing.
    // 2. How many symbols does each DARWIN trade? If only 1, never suggest reducing (loses all exposure).
    let mut symbol_darwin_count: std::collections::HashMap<(String, String), usize> = std::collections::HashMap::new(); // (symbol, side) -> count of DARWINs
    let mut darwin_symbol_count: std::collections::HashMap<String, usize> = std::collections::HashMap::new(); // darwin -> count of distinct symbols
    for (dw, sym, side, _, _, _) in &all_open {
        *symbol_darwin_count.entry((sym.clone(), side.clone())).or_insert(0) += 1;
        // Count distinct symbols per DARWIN (use a set approach via HashMap)
        darwin_symbol_count.entry(dw.clone()).or_insert(0);
    }
    // Recount distinct symbols per DARWIN properly
    {
        let mut darwin_syms: std::collections::HashMap<String, std::collections::HashSet<String>> = std::collections::HashMap::new();
        for (dw, sym, _, _, _, _) in &all_open {
            darwin_syms.entry(dw.clone()).or_default().insert(sym.clone());
        }
        for (dw, syms) in &darwin_syms {
            darwin_symbol_count.insert(dw.clone(), syms.len());
        }
    }

    let mut profit_candidates: std::collections::HashMap<(String, String), (f64, String, String, f64)> = std::collections::HashMap::new();

    for pair in &high_corr_pairs {
        if pair.correlation >= 0.95 { // Darwinex upper correlation threshold
            let sharpe_a = darwin_var_list.iter().find(|d| d.0 == pair.darwin_a).map(|d| d.2).unwrap_or(0.0);
            let sharpe_b = darwin_var_list.iter().find(|d| d.0 == pair.darwin_b).map(|d| d.2).unwrap_or(0.0);
            let (reduce_dw, reduce_sym, other_dw) = if sharpe_a < sharpe_b {
                (&pair.darwin_a, &pair.symbol_a, &pair.darwin_b)
            } else {
                (&pair.darwin_b, &pair.symbol_b, &pair.darwin_a)
            };
            if let Some(pos) = all_open.iter().find(|(dw, sym, _, _, _, _)| dw == reduce_dw && sym == reduce_sym) {
                if !pos.5 { continue; } // skip positions at a loss

                // Protection 1: Don't reduce if this DARWIN only trades 1 symbol (would lose all exposure)
                if darwin_symbol_count.get(reduce_dw).copied().unwrap_or(0) <= 1 {
                    continue;
                }

                // Protection 2: Don't reduce if only 1 DARWIN holds this symbol on this side
                // (would eliminate the symbol from the portfolio entirely)
                if symbol_darwin_count.get(&(reduce_sym.to_string(), pos.2.clone())).copied().unwrap_or(0) <= 1 {
                    continue;
                }

                let current_price = prices.get(&pos.1).copied().unwrap_or(pos.4);
                let pnl_per_unit = if pos.2 == "buy" { current_price - pos.4 } else { pos.4 - current_price };
                let key = (reduce_dw.clone(), reduce_sym.clone());
                let entry = profit_candidates.entry(key).or_insert((0.0, pos.2.clone(), String::new(), 0.0));
                if pnl_per_unit.abs() > entry.0.abs() { entry.0 = pnl_per_unit; }
                if !entry.2.is_empty() { entry.2.push_str(", "); }
                entry.2.push_str(other_dw);
                entry.3 += pair.combined_notional;
            }
        }
    }

    // Sort by profit magnitude descending, take top 3
    let mut ranked: Vec<_> = profit_candidates.into_iter().collect();
    ranked.sort_by(|a, b| (b.1).0.abs().partial_cmp(&(a.1).0.abs()).unwrap_or(std::cmp::Ordering::Equal));

    for ((darwin, symbol), (pnl, side, corr_with, notional)) in ranked.iter().take(3) {
        let sym_holders = symbol_darwin_count.get(&(symbol.clone(), side.clone())).copied().unwrap_or(0);
        suggestions.push(RebalanceSuggestion {
            action: "TAKE PROFIT".into(),
            darwin_ticker: darwin.clone(),
            symbol: symbol.clone(),
            side: side.clone(),
            current_volume: *notional,
            suggested_volume: notional * 0.5,
            reason: format!("Partial profit on {} {} in {} (${:.2}/lot) — {}/{} DARWINs hold this, correlated with {}",
                symbol, side, darwin, pnl, sym_holders, accounts.len(), corr_with),
            impact_var_pct: -notional * 0.001,
        });
    }

    // No allocation weight suggestions — all accounts run 100% margin, capital cannot
    // be reallocated between DARWINs. Only position-level profit-taking is actionable.
    let optimal = compute_optimal_allocation(conn)?;

    Ok(RebalanceDashboard {
        current_portfolio_var_95: portfolio_var.var_95,
        current_portfolio_var_99: portfolio_var.var_99,
        current_sharpe: portfolio_var.sharpe,
        high_correlation_pairs: high_corr_pairs,
        suggestions,
        darwin_var: darwin_var_list,
        optimal_allocation: optimal,
    })
}

// ── Conditional VaR by Regime ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalVaR {
    pub regime: String,
    pub days_in_regime: i64,
    pub avg_daily_pnl: f64,
    pub var_95: f64,
    pub var_99: f64,
    pub sharpe: f64,
}

/// Compute VaR separately for each volatility regime (LOW/MEDIUM/HIGH).
/// Splits returns into 3 regimes based on 20-day rolling volatility terciles.
pub fn compute_conditional_var(daily_returns: &[DailyReturn]) -> Vec<ConditionalVaR> {
    if daily_returns.len() < 25 {
        return vec![];
    }

    let returns: Vec<f64> = daily_returns.iter().map(|r| r.return_pct).collect();

    // Compute 20-day rolling volatility for each day
    let mut rolling_vols: Vec<(usize, f64)> = Vec::new();
    for i in 19..returns.len() {
        let window = &returns[i - 19..=i];
        let mean = window.iter().sum::<f64>() / window.len() as f64;
        let var = window.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / window.len() as f64;
        rolling_vols.push((i, var.sqrt()));
    }

    if rolling_vols.is_empty() {
        return vec![];
    }

    // Sort vols to find tercile thresholds
    let mut sorted_vols: Vec<f64> = rolling_vols.iter().map(|(_, v)| *v).collect();
    sorted_vols.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let t1 = sorted_vols[sorted_vols.len() / 3];
    let t2 = sorted_vols[2 * sorted_vols.len() / 3];

    // Assign each day to a regime
    let mut low_returns: Vec<f64> = Vec::new();
    let mut med_returns: Vec<f64> = Vec::new();
    let mut high_returns: Vec<f64> = Vec::new();

    for &(idx, vol) in &rolling_vols {
        let ret = returns[idx];
        if vol <= t1 {
            low_returns.push(ret);
        } else if vol <= t2 {
            med_returns.push(ret);
        } else {
            high_returns.push(ret);
        }
    }

    fn regime_var(name: &str, rets: &mut Vec<f64>) -> ConditionalVaR {
        let n = rets.len();
        if n < 2 {
            return ConditionalVaR {
                regime: name.to_string(),
                days_in_regime: n as i64,
                avg_daily_pnl: 0.0,
                var_95: 0.0,
                var_99: 0.0,
                sharpe: 0.0,
            };
        }
        let mean = rets.iter().sum::<f64>() / n as f64;
        let var = rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n as f64;
        let vol = var.sqrt();
        let sharpe = if vol > 0.0 { mean / vol * (252.0f64).sqrt() } else { 0.0 };

        rets.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let var_95 = -rets[((0.05 * (n as f64 - 1.0)).round() as usize).min(n - 1)];
        let var_99 = -rets[((0.01 * (n as f64 - 1.0)).round() as usize).min(n - 1)];

        ConditionalVaR {
            regime: name.to_string(),
            days_in_regime: n as i64,
            avg_daily_pnl: mean,
            var_95,
            var_99,
            sharpe,
        }
    }

    vec![
        regime_var("LOW_VOL", &mut low_returns),
        regime_var("MEDIUM_VOL", &mut med_returns),
        regime_var("HIGH_VOL", &mut high_returns),
    ]
}

// ── Market Regime Detection ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketRegime {
    pub current_regime: String,
    pub regime_start: String,
    pub regime_duration_days: i64,
    pub rolling_vol: f64,
    pub vol_percentile: f64,
    pub regime_history: Vec<(String, String, i64)>,
}

/// Detect current market regime using 20-day rolling volatility.
/// LOW = below 15th percentile, MEDIUM = 15-85th, HIGH = above 85th.
pub fn detect_market_regime(daily_returns: &[DailyReturn]) -> MarketRegime {
    let empty = MarketRegime {
        current_regime: "UNKNOWN".to_string(),
        regime_start: String::new(),
        regime_duration_days: 0,
        rolling_vol: 0.0,
        vol_percentile: 0.0,
        regime_history: vec![],
    };

    if daily_returns.len() < 25 {
        return empty;
    }

    let returns: Vec<f64> = daily_returns.iter().map(|r| r.return_pct).collect();
    let dates: Vec<&str> = daily_returns.iter().map(|r| r.date.as_str()).collect();

    // Compute 20-day rolling vol for each day starting from index 19
    let mut rolling: Vec<(usize, f64)> = Vec::new();
    for i in 19..returns.len() {
        let window = &returns[i - 19..=i];
        let mean = window.iter().sum::<f64>() / window.len() as f64;
        let var = window.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / window.len() as f64;
        rolling.push((i, var.sqrt()));
    }

    if rolling.is_empty() {
        return empty;
    }

    // Sort vols to find percentile thresholds
    let mut sorted_vols: Vec<f64> = rolling.iter().map(|(_, v)| *v).collect();
    sorted_vols.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let p15 = sorted_vols[(0.15 * (sorted_vols.len() as f64 - 1.0)).round() as usize];
    let p85 = sorted_vols[(0.85 * (sorted_vols.len() as f64 - 1.0)).round() as usize];

    let classify = |vol: f64| -> &'static str {
        if vol <= p15 { "LOW_VOL" }
        else if vol <= p85 { "MEDIUM_VOL" }
        else { "HIGH_VOL" }
    };

    // Build regime history
    let mut regime_history: Vec<(String, String, i64)> = Vec::new();
    let mut current_regime = classify(rolling[0].1).to_string();
    let mut regime_start_idx = rolling[0].0;
    let mut regime_days = 1i64;

    for i in 1..rolling.len() {
        let (idx, vol) = rolling[i];
        let regime = classify(vol).to_string();
        if regime != current_regime {
            regime_history.push((
                current_regime.clone(),
                dates[regime_start_idx].to_string(),
                regime_days,
            ));
            current_regime = regime;
            regime_start_idx = idx;
            regime_days = 1;
        } else {
            regime_days += 1;
        }
    }

    // Current vol percentile
    let current_vol = rolling.last().map(|(_, v)| *v).unwrap_or(0.0);
    let vol_percentile = sorted_vols.iter().filter(|&&v| v <= current_vol).count() as f64
        / sorted_vols.len() as f64 * 100.0;

    MarketRegime {
        current_regime,
        regime_start: dates.get(regime_start_idx).unwrap_or(&"").to_string(),
        regime_duration_days: regime_days,
        rolling_vol: current_vol,
        vol_percentile,
        regime_history,
    }
}

// ── Exposure Treemap Data ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreemapNode {
    pub name: String,
    pub value: f64,
    pub color_value: f64,
    pub children: Vec<TreemapNode>,
}

/// Build a treemap data structure: Root -> Sectors -> Symbols.
/// Sized by absolute notional, colored by side (long=+1 green, short=-1 red).
pub fn get_exposure_treemap(conn: &Connection) -> Result<TreemapNode, String> {
    let sector_exposures = get_sector_exposure(conn)?;
    let open_positions = get_portfolio_open_positions(conn)?;

    // Build a lookup: symbol -> (notional, side color: +1 for buy, -1 for sell)
    let mut symbol_info: std::collections::HashMap<String, (f64, f64)> = std::collections::HashMap::new();
    for pos in &open_positions {
        let color = if pos.side == "buy" { 1.0 } else { -1.0 };
        let entry = symbol_info.entry(pos.symbol.clone()).or_insert((0.0, 0.0));
        entry.0 += pos.notional.abs();
        // Weighted average color for mixed positions
        entry.1 = color;
    }

    let mut sector_children: Vec<TreemapNode> = Vec::new();
    let mut root_total = 0.0f64;

    for sector in &sector_exposures {
        let mut sym_children: Vec<TreemapNode> = Vec::new();
        let mut sector_total = 0.0f64;
        let mut sector_color_sum = 0.0f64;

        for sym in &sector.symbols {
            if let Some(&(notional, color)) = symbol_info.get(sym) {
                sym_children.push(TreemapNode {
                    name: sym.clone(),
                    value: notional,
                    color_value: color,
                    children: vec![],
                });
                sector_total += notional;
                sector_color_sum += color * notional;
            }
        }

        if sector_total > 0.0 {
            sym_children.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));
            let sector_color = sector_color_sum / sector_total;
            sector_children.push(TreemapNode {
                name: sector.sector.clone(),
                value: sector_total,
                color_value: sector_color,
                children: sym_children,
            });
            root_total += sector_total;
        }
    }

    sector_children.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));

    Ok(TreemapNode {
        name: "Portfolio".to_string(),
        value: root_total,
        color_value: 0.0,
        children: sector_children,
    })
}

// ── Cross-Account Timing Divergence ─────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingDivergence {
    pub symbol: String,
    pub entries: Vec<(String, String, f64)>, // (darwin_ticker, entry_time, entry_price)
    pub time_spread_hours: f64, // max time difference between first and last entry
    pub price_spread_pct: f64,  // price difference as %
}

/// For symbols traded by multiple DARWINs, compare entry times.
/// Find cases where DARWINs entered the same symbol at different times.
pub fn get_timing_divergences(conn: &Connection) -> Result<Vec<TimingDivergence>, String> {
    let accounts = list_darwin_accounts(conn)?;

    // Collect all open "in" deals per (symbol, side) across DARWINs
    // Key: (symbol, side) -> Vec<(darwin_ticker, time, price)>
    let mut symbol_entries: std::collections::HashMap<(String, String), Vec<(String, String, f64)>> =
        std::collections::HashMap::new();

    for account in &accounts {
        let mut stmt = conn.prepare(
            "SELECT symbol, deal_type, time, price FROM darwin_deals \
             WHERE account = ?1 AND direction = 'in' AND symbol != '' \
             ORDER BY time"
        ).map_err(|e| format!("Prepare failed: {e}"))?;

        let rows = stmt.query_map(params![&account.darwin_ticker], |row| {
            Ok((
                row.get::<_, String>(0)?, // symbol
                row.get::<_, String>(1)?, // deal_type (buy/sell = side)
                row.get::<_, String>(2)?, // time
                row.get::<_, f64>(3)?,    // price
            ))
        }).map_err(|e| format!("Query failed: {e}"))?;

        for row in rows {
            if let Ok((symbol, side, time, price)) = row {
                let key = (symbol, side);
                symbol_entries.entry(key).or_default().push((
                    account.darwin_ticker.clone(),
                    time,
                    price,
                ));
            }
        }
    }

    let mut divergences: Vec<TimingDivergence> = Vec::new();

    for ((symbol, _side), entries) in &symbol_entries {
        // Only care about symbols traded by multiple DARWINs
        let unique_darwins: std::collections::HashSet<&str> =
            entries.iter().map(|(dt, _, _)| dt.as_str()).collect();
        if unique_darwins.len() < 2 {
            continue;
        }

        // Parse times, find earliest and latest entry per DARWIN (first entry only)
        let mut per_darwin: std::collections::HashMap<&str, (&str, f64)> =
            std::collections::HashMap::new();
        for (dt, time, price) in entries {
            per_darwin.entry(dt.as_str()).or_insert((time.as_str(), *price));
        }

        let parsed_times: Vec<(&str, chrono::NaiveDateTime, f64)> = per_darwin
            .iter()
            .filter_map(|(&dt, &(time, price))| {
                parse_mt5_datetime(time).map(|ndt| (dt, ndt, price))
            })
            .collect();

        if parsed_times.len() < 2 {
            continue;
        }

        let earliest = match parsed_times.iter().min_by_key(|(_, t, _)| *t) {
            Some(e) => e,
            None => continue,
        };
        let latest = match parsed_times.iter().max_by_key(|(_, t, _)| *t) {
            Some(l) => l,
            None => continue,
        };

        let time_spread_hours = (latest.1 - earliest.1).num_seconds() as f64 / 3600.0;

        // Price spread: difference between min and max entry prices as % of average
        let prices: Vec<f64> = parsed_times.iter().map(|(_, _, p)| *p).collect();
        let min_price = prices.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_price = prices.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let avg_price = prices.iter().sum::<f64>() / prices.len() as f64;
        let price_spread_pct = if avg_price > 0.0 {
            (max_price - min_price) / avg_price * 100.0
        } else {
            0.0
        };

        let mut entry_list: Vec<(String, String, f64)> = per_darwin
            .iter()
            .map(|(&dt, &(time, price))| (dt.to_string(), time.to_string(), price))
            .collect();
        entry_list.sort_by(|a, b| a.1.cmp(&b.1));

        divergences.push(TimingDivergence {
            symbol: symbol.clone(),
            entries: entry_list,
            time_spread_hours,
            price_spread_pct,
        });
    }

    // Sort by time spread descending (biggest divergences first)
    divergences.sort_by(|a, b| b.time_spread_hours.partial_cmp(&a.time_spread_hours).unwrap_or(std::cmp::Ordering::Equal));
    Ok(divergences)
}

// ── Best/Worst DARWIN by Market Regime ──────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimePerformance {
    pub darwin_ticker: String,
    pub low_vol_sharpe: f64,
    pub medium_vol_sharpe: f64,
    pub high_vol_sharpe: f64,
    pub best_regime: String,
    pub worst_regime: String,
}

/// For each DARWIN, compute Sharpe in each vol regime (reuses compute_conditional_var logic).
pub fn get_regime_performance(conn: &Connection) -> Result<Vec<RegimePerformance>, String> {
    let accounts = list_darwin_accounts(conn)?;
    let mut results: Vec<RegimePerformance> = Vec::new();

    for account in &accounts {
        let daily_returns = get_daily_returns(conn, &account.darwin_ticker)?;
        let cvar = compute_conditional_var(&daily_returns);

        if cvar.is_empty() {
            continue;
        }

        let mut low_sharpe = 0.0f64;
        let mut med_sharpe = 0.0f64;
        let mut high_sharpe = 0.0f64;

        for cv in &cvar {
            match cv.regime.as_str() {
                "LOW_VOL" => low_sharpe = cv.sharpe,
                "MEDIUM_VOL" => med_sharpe = cv.sharpe,
                "HIGH_VOL" => high_sharpe = cv.sharpe,
                _ => {}
            }
        }

        let regimes = [
            ("LOW_VOL", low_sharpe),
            ("MEDIUM_VOL", med_sharpe),
            ("HIGH_VOL", high_sharpe),
        ];

        let best = regimes.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)).unwrap();
        let worst = regimes.iter().min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)).unwrap();

        results.push(RegimePerformance {
            darwin_ticker: account.darwin_ticker.clone(),
            low_vol_sharpe: low_sharpe,
            medium_vol_sharpe: med_sharpe,
            high_vol_sharpe: high_sharpe,
            best_regime: best.0.to_string(),
            worst_regime: worst.0.to_string(),
        });
    }

    Ok(results)
}

// ── Tax Lot Tracking (FIFO) ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxLot {
    pub symbol: String,
    pub open_date: String,
    pub close_date: String,
    pub volume: f64,
    pub cost_basis: f64,
    pub proceeds: f64,
    pub realized_pnl: f64,
    pub holding_period: String, // "SHORT" (<1yr) or "LONG" (>=1yr)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxSummary {
    pub year: i32,
    pub short_term_gains: f64,
    pub short_term_losses: f64,
    pub long_term_gains: f64,
    pub long_term_losses: f64,
    pub net_short_term: f64,
    pub net_long_term: f64,
    pub total_net: f64,
    pub lots: Vec<TaxLot>,
}

/// FIFO matching of closed positions. Parse open_time and close_time,
/// compute holding period, classify as short/long term.
pub fn compute_tax_lots(conn: &Connection, darwin_ticker: &str, year: i32) -> Result<TaxSummary, String> {
    // Get all closed positions for this DARWIN, ordered by close_time
    let mut stmt = conn.prepare(
        "SELECT symbol, open_time, close_time, volume, open_price, close_price, pos_type, profit, commission, swap \
         FROM darwin_positions WHERE account = ?1 AND close_time != '' \
         ORDER BY symbol, open_time"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let rows = stmt.query_map(params![darwin_ticker], |row| {
        Ok((
            row.get::<_, String>(0)?,  // symbol
            row.get::<_, String>(1)?,  // open_time
            row.get::<_, String>(2)?,  // close_time
            row.get::<_, f64>(3)?,     // volume
            row.get::<_, f64>(4)?,     // open_price
            row.get::<_, f64>(5)?,     // close_price
            row.get::<_, String>(6)?,  // pos_type
            row.get::<_, f64>(7)?,     // profit
            row.get::<_, f64>(8)?,     // commission
            row.get::<_, f64>(9)?,     // swap
        ))
    }).map_err(|e| format!("Query failed: {e}"))?;

    // FIFO queue per symbol: collect all positions, then match
    let mut positions: Vec<(String, String, String, f64, f64, f64, String, f64, f64, f64)> = Vec::new();
    for row in rows {
        if let Ok(p) = row {
            positions.push(p);
        }
    }

    let mut lots: Vec<TaxLot> = Vec::new();
    let year_start = format!("{}.01.01", year);
    let year_end = format!("{}.01.01", year + 1);

    for (symbol, open_time, close_time, volume, open_price, close_price, _pos_type, profit, commission, swap) in &positions {
        // Filter to positions closed in the target year
        let close_date_str = if close_time.len() >= 10 { &close_time[..10] } else { close_time.as_str() };
        let open_date_str = if open_time.len() >= 10 { &open_time[..10] } else { open_time.as_str() };

        // Check close_time falls within the year
        if *close_time < year_start || *close_time >= year_end {
            continue;
        }

        // Compute cost basis and proceeds
        let cost_basis = volume * open_price;
        let proceeds = volume * close_price;

        // Net P/L including costs
        let realized_pnl = profit + commission + swap;

        // Holding period: parse dates and check if >= 365 days
        let holding_period = match (parse_mt5_datetime(open_time), parse_mt5_datetime(close_time)) {
            (Some(open_dt), Some(close_dt)) => {
                let days = (close_dt - open_dt).num_days();
                if days >= 365 { "LONG".to_string() } else { "SHORT".to_string() }
            }
            _ => "SHORT".to_string(), // default to short-term if can't parse
        };

        lots.push(TaxLot {
            symbol: symbol.clone(),
            open_date: open_date_str.to_string(),
            close_date: close_date_str.to_string(),
            volume: *volume,
            cost_basis,
            proceeds,
            realized_pnl,
            holding_period,
        });
    }

    // Aggregate
    let mut short_term_gains = 0.0f64;
    let mut short_term_losses = 0.0f64;
    let mut long_term_gains = 0.0f64;
    let mut long_term_losses = 0.0f64;

    for lot in &lots {
        match lot.holding_period.as_str() {
            "SHORT" => {
                if lot.realized_pnl >= 0.0 {
                    short_term_gains += lot.realized_pnl;
                } else {
                    short_term_losses += lot.realized_pnl;
                }
            }
            "LONG" => {
                if lot.realized_pnl >= 0.0 {
                    long_term_gains += lot.realized_pnl;
                } else {
                    long_term_losses += lot.realized_pnl;
                }
            }
            _ => {}
        }
    }

    let net_short_term = short_term_gains + short_term_losses;
    let net_long_term = long_term_gains + long_term_losses;

    Ok(TaxSummary {
        year,
        short_term_gains,
        short_term_losses,
        long_term_gains,
        long_term_losses,
        net_short_term,
        net_long_term,
        total_net: net_short_term + net_long_term,
        lots,
    })
}

// ── Daily Risk Report ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyRiskReport {
    pub date: String,
    pub portfolio_equity: f64,
    pub daily_pnl: f64,
    pub daily_return_pct: f64,
    pub current_var_95: f64,
    pub current_drawdown_pct: f64,
    pub open_position_count: i64,
    pub total_notional: f64,
    pub top_gainers: Vec<(String, f64)>,
    pub top_losers: Vec<(String, f64)>,
    pub alerts: Vec<String>,
    pub regime: String,
}

/// Aggregates current state across all DARWINs into a daily risk report.
pub fn generate_daily_report(conn: &Connection) -> Result<DailyRiskReport, String> {
    // Portfolio daily returns for VaR and regime
    let portfolio_returns = get_portfolio_daily_returns(conn)?;

    let (date, portfolio_equity, daily_pnl, daily_return_pct, current_drawdown_pct) =
        if let Some(last) = portfolio_returns.last() {
            (
                last.date.clone(),
                last.balance,
                last.pnl,
                last.return_pct,
                last.drawdown_pct,
            )
        } else {
            (String::new(), 0.0, 0.0, 0.0, 0.0)
        };

    // VaR
    let var_result = compute_var(&portfolio_returns);
    let current_var_95 = var_result.var_95;

    // Market regime
    let regime_info = detect_market_regime(&portfolio_returns);
    let regime = regime_info.current_regime;

    // Open positions count and total notional
    let open_positions = get_portfolio_open_positions(conn)?;
    let open_position_count = open_positions.len() as i64;
    let total_notional: f64 = open_positions.iter().map(|p| p.notional).sum();

    // Top gainers/losers by symbol P/L across all DARWINs
    let accounts = list_darwin_accounts(conn)?;
    let mut symbol_pnl: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    for account in &accounts {
        if let Ok(pnl_by_sym) = get_darwin_pnl_by_symbol(conn, &account.darwin_ticker) {
            for (sym, profit, commission, swap, _count) in pnl_by_sym {
                let net = profit + commission + swap;
                *symbol_pnl.entry(sym).or_insert(0.0) += net;
            }
        }
    }

    let mut sorted_pnl: Vec<(String, f64)> = symbol_pnl.into_iter().collect();
    sorted_pnl.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let top_gainers: Vec<(String, f64)> = sorted_pnl.iter()
        .filter(|(_, pnl)| *pnl > 0.0)
        .take(5)
        .cloned()
        .collect();

    let top_losers: Vec<(String, f64)> = sorted_pnl.iter()
        .rev()
        .filter(|(_, pnl)| *pnl < 0.0)
        .take(5)
        .cloned()
        .collect();

    // Alerts
    let alert_conditions = check_alerts(conn).unwrap_or_default();
    let alerts: Vec<String> = alert_conditions.iter().map(|a| {
        format!("[{}] {}: {}", a.severity, a.alert_type, a.message)
    }).collect();

    Ok(DailyRiskReport {
        date,
        portfolio_equity,
        daily_pnl,
        daily_return_pct,
        current_var_95,
        current_drawdown_pct,
        open_position_count,
        total_notional,
        top_gainers,
        top_losers,
        alerts,
        regime,
    })
}

// ── Combined Drawdown & Best/Worst Days Dashboard ──────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawdownEntry {
    pub date: String,
    pub drawdown_pct: f64,
    pub balance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestWorstDay {
    pub date: String,
    pub pnl: f64,
    pub return_pct: f64,
    pub balance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarwinDrawdownSummary {
    pub darwin_ticker: String,
    pub max_drawdown_pct: f64,
    pub max_dd_date: String,
    pub current_drawdown_pct: f64,
    pub best_days: Vec<BestWorstDay>,
    pub worst_days: Vec<BestWorstDay>,
    pub drawdown_curve: Vec<DrawdownEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedDrawdownDashboard {
    pub darwins: Vec<DarwinDrawdownSummary>,
    pub combined: DarwinDrawdownSummary,
}

/// Combined drawdown view across all DARWINs + portfolio aggregate.
/// Returns per-DARWIN drawdown curves, best/worst days, and combined portfolio view.
pub fn get_combined_drawdown_dashboard(conn: &Connection, top_n: usize) -> Result<CombinedDrawdownDashboard, String> {
    let accounts = list_darwin_accounts(conn)?;
    let mut darwins = Vec::new();

    for acct in &accounts {
        let daily = get_daily_returns(conn, &acct.darwin_ticker)?;
        if daily.is_empty() { continue; }

        let mut max_dd = 0.0f64;
        let mut max_dd_date = String::new();
        let current_dd = daily.last().map(|r| r.drawdown_pct).unwrap_or(0.0);

        let mut drawdown_curve = Vec::with_capacity(daily.len());
        for r in &daily {
            if r.drawdown_pct > max_dd {
                max_dd = r.drawdown_pct;
                max_dd_date = r.date.clone();
            }
            drawdown_curve.push(DrawdownEntry {
                date: r.date.clone(),
                drawdown_pct: r.drawdown_pct,
                balance: r.balance,
            });
        }

        // Best/worst days sorted by P&L
        let mut sorted_by_pnl: Vec<&DailyReturn> = daily.iter().collect();
        sorted_by_pnl.sort_by(|a, b| a.pnl.partial_cmp(&b.pnl).unwrap_or(std::cmp::Ordering::Equal));

        let worst_days: Vec<BestWorstDay> = sorted_by_pnl.iter().take(top_n).map(|r| BestWorstDay {
            date: r.date.clone(), pnl: r.pnl, return_pct: r.return_pct, balance: r.balance,
        }).collect();

        let best_days: Vec<BestWorstDay> = sorted_by_pnl.iter().rev().take(top_n).map(|r| BestWorstDay {
            date: r.date.clone(), pnl: r.pnl, return_pct: r.return_pct, balance: r.balance,
        }).collect();

        darwins.push(DarwinDrawdownSummary {
            darwin_ticker: acct.darwin_ticker.clone(),
            max_drawdown_pct: max_dd,
            max_dd_date,
            current_drawdown_pct: current_dd,
            best_days,
            worst_days,
            drawdown_curve,
        });
    }

    // Combined portfolio
    let portfolio_daily = get_portfolio_daily_returns(conn)?;
    let mut combined_max_dd = 0.0f64;
    let mut combined_max_dd_date = String::new();
    let combined_current_dd = portfolio_daily.last().map(|r| r.drawdown_pct).unwrap_or(0.0);

    let mut combined_dd_curve = Vec::with_capacity(portfolio_daily.len());
    for r in &portfolio_daily {
        if r.drawdown_pct > combined_max_dd {
            combined_max_dd = r.drawdown_pct;
            combined_max_dd_date = r.date.clone();
        }
        combined_dd_curve.push(DrawdownEntry {
            date: r.date.clone(),
            drawdown_pct: r.drawdown_pct,
            balance: r.balance,
        });
    }

    let mut combined_sorted: Vec<&DailyReturn> = portfolio_daily.iter().collect();
    combined_sorted.sort_by(|a, b| a.pnl.partial_cmp(&b.pnl).unwrap_or(std::cmp::Ordering::Equal));

    let combined_worst: Vec<BestWorstDay> = combined_sorted.iter().take(top_n).map(|r| BestWorstDay {
        date: r.date.clone(), pnl: r.pnl, return_pct: r.return_pct, balance: r.balance,
    }).collect();

    let combined_best: Vec<BestWorstDay> = combined_sorted.iter().rev().take(top_n).map(|r| BestWorstDay {
        date: r.date.clone(), pnl: r.pnl, return_pct: r.return_pct, balance: r.balance,
    }).collect();

    let combined = DarwinDrawdownSummary {
        darwin_ticker: "COMBINED".into(),
        max_drawdown_pct: combined_max_dd,
        max_dd_date: combined_max_dd_date,
        current_drawdown_pct: combined_current_dd,
        best_days: combined_best,
        worst_days: combined_worst,
        drawdown_curve: combined_dd_curve,
    };

    Ok(CombinedDrawdownDashboard { darwins, combined })
}

// ── Floating Equity Tracker ────────────────────────────────────────
// DARWIN quotes are based on floating P&L, not just closed balance.
// This tracks mark-to-market equity by combining closed balance + unrealized P&L
// from open positions priced at live quotes.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloatingEquitySnapshot {
    pub timestamp: i64,
    pub darwin_ticker: String,
    pub closed_balance: f64,
    pub unrealized_pnl: f64,
    pub floating_equity: f64,
    pub open_position_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloatingEquityDashboard {
    pub darwins: Vec<DarwinFloatingEquity>,
    pub combined_closed_balance: f64,
    pub combined_unrealized_pnl: f64,
    pub combined_floating_equity: f64,
    pub combined_history: Vec<FloatingEquitySnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarwinFloatingEquity {
    pub darwin_ticker: String,
    pub closed_balance: f64,
    pub unrealized_pnl: f64,
    pub floating_equity: f64,
    pub open_positions: Vec<OpenPositionMTM>,
    pub history: Vec<FloatingEquitySnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenPositionMTM {
    pub symbol: String,
    pub side: String,
    pub volume: f64,
    pub avg_price: f64,
    pub current_price: f64,
    pub unrealized_pnl: f64,
    pub pnl_pct: f64,
}

/// Record a floating equity snapshot for a DARWIN.
/// Called periodically by the frontend with live prices for open positions.
pub fn record_equity_snapshot(
    conn: &Connection,
    darwin_ticker: &str,
    closed_balance: f64,
    unrealized_pnl: f64,
    open_count: i64,
) -> Result<(), String> {
    let ts = chrono::Utc::now().timestamp();
    let floating = closed_balance + unrealized_pnl;
    conn.execute(
        "INSERT INTO darwin_equity_snapshots (timestamp, darwin_ticker, closed_balance, unrealized_pnl, floating_equity, open_position_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![ts, darwin_ticker, closed_balance, unrealized_pnl, floating, open_count],
    ).map_err(|e| format!("Insert equity snapshot failed: {e}"))?;
    Ok(())
}

/// Get equity snapshot history for a DARWIN (last N days).
pub fn get_equity_history(conn: &Connection, darwin_ticker: &str, limit: usize) -> Result<Vec<FloatingEquitySnapshot>, String> {
    let mut stmt = conn.prepare(
        "SELECT timestamp, darwin_ticker, closed_balance, unrealized_pnl, floating_equity, open_position_count FROM darwin_equity_snapshots WHERE darwin_ticker = ?1 ORDER BY timestamp DESC LIMIT ?2"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let rows = stmt.query_map(params![darwin_ticker, limit as i64], |row| {
        Ok(FloatingEquitySnapshot {
            timestamp: row.get(0)?,
            darwin_ticker: row.get(1)?,
            closed_balance: row.get(2)?,
            unrealized_pnl: row.get(3)?,
            floating_equity: row.get(4)?,
            open_position_count: row.get(5)?,
        })
    }).map_err(|e| format!("Query failed: {e}"))?;

    let mut result: Vec<FloatingEquitySnapshot> = rows.filter_map(|r| r.ok()).collect();
    result.reverse(); // chronological order
    Ok(result)
}

/// Compute floating equity for all DARWINs given a map of current prices.
/// prices: HashMap<symbol, current_bid_price>
pub fn compute_floating_equity(
    conn: &Connection,
    prices: &std::collections::HashMap<String, f64>,
) -> Result<FloatingEquityDashboard, String> {
    let accounts = list_darwin_accounts(conn)?;
    let mut darwins = Vec::new();
    let mut combined_closed = 0.0;
    let mut combined_unrealized = 0.0;

    for acct in &accounts {
        // Get closed balance (last deal's balance)
        let closed_balance: f64 = conn.query_row(
            "SELECT COALESCE(balance, 0) FROM darwin_deals WHERE account = ?1 ORDER BY id DESC LIMIT 1",
            params![acct.darwin_ticker],
            |row| row.get(0),
        ).unwrap_or(acct.initial_balance);

        // Get open positions
        let open_positions = get_darwin_open_positions(conn, &acct.darwin_ticker)?;
        let mut total_unrealized = 0.0;
        let mut mtm_positions = Vec::new();

        for pos in &open_positions {
            let current_price = prices.get(&pos.symbol).copied().unwrap_or(pos.avg_price);
            let pnl = if pos.side == "buy" {
                (current_price - pos.avg_price) * pos.total_volume
            } else {
                (pos.avg_price - current_price) * pos.total_volume
            };
            let pnl_pct = if pos.avg_price > 0.0 {
                (current_price - pos.avg_price) / pos.avg_price * 100.0 * if pos.side == "sell" { -1.0 } else { 1.0 }
            } else { 0.0 };

            total_unrealized += pnl;
            mtm_positions.push(OpenPositionMTM {
                symbol: pos.symbol.clone(),
                side: pos.side.clone(),
                volume: pos.total_volume,
                avg_price: pos.avg_price,
                current_price,
                unrealized_pnl: pnl,
                pnl_pct,
            });
        }

        let floating = closed_balance + total_unrealized;
        combined_closed += closed_balance;
        combined_unrealized += total_unrealized;

        // Get snapshot history
        let history = get_equity_history(conn, &acct.darwin_ticker, 1000)?;

        darwins.push(DarwinFloatingEquity {
            darwin_ticker: acct.darwin_ticker.clone(),
            closed_balance,
            unrealized_pnl: total_unrealized,
            floating_equity: floating,
            open_positions: mtm_positions,
            history,
        });
    }

    // Combined history
    let combined_history = get_equity_history(conn, "COMBINED", 1000)?;

    Ok(FloatingEquityDashboard {
        darwins,
        combined_closed_balance: combined_closed,
        combined_unrealized_pnl: combined_unrealized,
        combined_floating_equity: combined_closed + combined_unrealized,
        combined_history,
    })
}

// ── Darwinex VaR Multiplier Prediction ───────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarwinVaRMultiplier {
    pub darwin_ticker: String,
    pub daily_var_95: f64,           // blended daily VaR at 95% confidence (% terms)
    pub var_45d: f64,                // 45-day rolling VaR component
    pub var_6m: f64,                 // 6-month average VaR component
    pub monthly_var: f64,            // blended daily * sqrt(21)
    pub multiplier: f64,             // 6.5 / monthly_var, capped at [0.0, 2.0]
    pub in_corridor: bool,           // true if 3.25% <= monthly_var <= 6.5%
    pub corridor_position: String,   // "below" / "in" / "above"
    pub investor_return_factor: f64, // multiplier * strategy_return = investor return
    pub recommendation: String,      // actionable insight
}

/// Compute Darwinex VaR multiplier predictions for all DARWINs.
///
/// Darwinex normalizes all DARWINs to a 6.5% monthly VaR target.
/// - VaR corridor: 3.25% (lower) to 6.5% (upper)
/// - If monthly VaR > 6.5%: multiplier < 1.0 (reduces exposure)
/// - If monthly VaR in 3.25-6.5%: multiplier 1.0-2.0x
/// - If monthly VaR < 3.25%: multiplier = 2.0 (capped)
pub fn compute_var_multipliers(conn: &Connection) -> Result<Vec<DarwinVaRMultiplier>, String> {
    let accounts = list_darwin_accounts(conn)?;
    let mut results = Vec::new();

    for acct in &accounts {
        let daily_returns = get_daily_returns(conn, &acct.darwin_ticker)?;
        if daily_returns.len() < 10 { continue; }

        // Darwinex VaR methodology: blend of 45-day rolling VaR + 6-month average VaR.
        // This smooths short-term spikes while reflecting recent risk changes.
        let compute_daily_var_95 = |returns: &[DailyReturn]| -> f64 {
            if returns.len() < 5 { return 0.0; }
            let mut sorted: Vec<f64> = returns.iter().map(|r| r.return_pct).collect();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let idx = ((sorted.len() as f64) * 0.05).floor() as usize;
            sorted.get(idx).copied().unwrap_or(0.0).abs()
        };

        // 45-day rolling VaR (current window)
        let window_45: &[DailyReturn] = if daily_returns.len() > 45 {
            &daily_returns[daily_returns.len() - 45..]
        } else {
            &daily_returns
        };
        let var_45d = compute_daily_var_95(window_45);

        // 6-month average VaR (~126 trading days)
        let window_6m: &[DailyReturn] = if daily_returns.len() > 126 {
            &daily_returns[daily_returns.len() - 126..]
        } else {
            &daily_returns
        };
        let var_6m = compute_daily_var_95(window_6m);

        // Darwinex blended VaR: average of 45-day and 6-month
        let daily_var_95 = if daily_returns.len() > 126 {
            (var_45d + var_6m) / 2.0  // blend both windows
        } else {
            var_45d  // not enough history for 6-month, use 45-day only
        };

        // Scale to monthly: monthly_var = daily_var * sqrt(21)
        let monthly_var = daily_var_95 * (21.0f64).sqrt();

        // Compute multiplier = min(6.5 / monthly_var, 2.0)
        let multiplier = if monthly_var > 0.0 {
            (6.5 / monthly_var).min(2.0)
        } else {
            2.0
        };

        let (corridor_position, in_corridor, recommendation) = if monthly_var > 6.5 {
            ("above".to_string(), false, format!(
                "VaR too high ({:.1}%) \u{2014} multiplier {:.2}x reduces investor exposure. Lower position sizes or hedge to increase multiplier.",
                monthly_var, multiplier
            ))
        } else if monthly_var >= 3.25 {
            ("in".to_string(), true, format!(
                "In corridor ({:.1}%) \u{2014} multiplier {:.2}x. Optimal range for investor returns.",
                monthly_var, multiplier
            ))
        } else {
            ("below".to_string(), false, format!(
                "VaR below corridor ({:.1}%) \u{2014} max multiplier 2.0x. Strategy returns amplified for investors.",
                monthly_var
            ))
        };

        results.push(DarwinVaRMultiplier {
            darwin_ticker: acct.darwin_ticker.clone(),
            daily_var_95,
            var_45d: var_45d * (21.0f64).sqrt(), // monthly-scaled 45d component
            var_6m: var_6m * (21.0f64).sqrt(),   // monthly-scaled 6m component
            monthly_var,
            multiplier,
            in_corridor,
            corridor_position,
            investor_return_factor: multiplier,
            recommendation,
        });
    }

    Ok(results)
}

// ── D-Score Estimation (from trade history) ─────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DScoreEstimate {
    pub darwin_ticker: String,
    pub experience: f64,      // 0-10
    pub risk_mgmt: f64,       // 0-10
    pub performance: f64,     // 0-10
    pub market_timing: f64,   // 0-10
    pub capacity: f64,        // 0-10
    pub scalability: f64,     // 0-10
    pub total_dscore: f64,    // weighted composite
    pub investable_months: i64, // months of track record
}

/// Estimate D-Score components from trade history data.
///
/// D-Score has 6 components (each 0-10):
/// - Experience: f(trading_days, trade_count)
/// - Risk Management: f(var_stability, max_drawdown, leverage_consistency)
/// - Performance: f(sharpe, sortino)
/// - Market Timing: f(win_rate on high-vol vs low-vol days)
/// - Capacity: f(avg_trade_duration, concurrent_positions)
/// - Scalability: f(symbol_diversity, position_sizing_consistency)
pub fn estimate_dscore(conn: &Connection, darwin_ticker: &str) -> Result<DScoreEstimate, String> {
    let daily_returns = get_daily_returns(conn, darwin_ticker)?;
    if daily_returns.len() < 5 {
        return Err("Not enough data for D-Score estimation".to_string());
    }

    let var_result = compute_var_full(&daily_returns);

    // ── Experience (0-10): min(10, trading_days / 100)
    let trading_days = daily_returns.len() as f64;
    let experience = (trading_days / 100.0).min(10.0);

    // Investable months
    let investable_months = (trading_days / 21.0).round() as i64;

    // ── Risk Management (0-10): based on VaR stability and max drawdown
    // Compute rolling 20-day VaR standard deviation
    let mut rolling_vars: Vec<f64> = Vec::new();
    if daily_returns.len() >= 20 {
        for i in 20..daily_returns.len() {
            let window = &daily_returns[i - 20..i];
            let mut sorted: Vec<f64> = window.iter().map(|r| r.return_pct).collect();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let idx = (sorted.len() as f64 * 0.05).floor() as usize;
            rolling_vars.push(sorted.get(idx).copied().unwrap_or(0.0).abs());
        }
    }
    let var_std = if rolling_vars.len() > 1 {
        let mean = rolling_vars.iter().sum::<f64>() / rolling_vars.len() as f64;
        let variance = rolling_vars.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
            / (rolling_vars.len() - 1) as f64;
        variance.sqrt()
    } else { 5.0 }; // high uncertainty

    // Low var_std = stable = high score; penalize if max DD > 20%
    let var_stability_score = (10.0 - var_std * 5.0).max(0.0).min(10.0);
    let dd_penalty = if var_result.max_drawdown_pct > 20.0 {
        ((var_result.max_drawdown_pct - 20.0) / 10.0).min(5.0)
    } else { 0.0 };
    let risk_mgmt = (var_stability_score - dd_penalty).max(0.0).min(10.0);

    // ── Performance (0-10): min(10, max(0, sharpe * 3 + 2))
    let performance = (var_result.sharpe * 3.0 + 2.0).max(0.0).min(10.0);

    // ── Market Timing (0-10): compare win rate on high-vol vs low-vol days
    let market_timing = if daily_returns.len() >= 20 {
        // Compute daily volatility for each day (absolute return as proxy)
        let abs_returns: Vec<f64> = daily_returns.iter().map(|r| r.return_pct.abs()).collect();
        let median_vol = {
            let mut sorted = abs_returns.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            sorted[sorted.len() / 2]
        };

        let (mut high_vol_wins, mut high_vol_total) = (0i64, 0i64);
        let (mut low_vol_wins, mut low_vol_total) = (0i64, 0i64);
        for (i, r) in daily_returns.iter().enumerate() {
            if abs_returns[i] > median_vol {
                high_vol_total += 1;
                if r.pnl > 0.0 { high_vol_wins += 1; }
            } else {
                low_vol_total += 1;
                if r.pnl > 0.0 { low_vol_wins += 1; }
            }
        }
        let high_wr = if high_vol_total > 0 { high_vol_wins as f64 / high_vol_total as f64 } else { 0.5 };
        let low_wr = if low_vol_total > 0 { low_vol_wins as f64 / low_vol_total as f64 } else { 0.5 };
        // Big positive difference = good timing (profits on volatile days)
        let timing_diff = high_wr - low_wr;
        (5.0 + timing_diff * 20.0).max(0.0).min(10.0)
    } else { 5.0 };

    // ── Capacity (0-10): based on avg trade duration and concurrent positions
    let capacity = if let Ok(hold_stats) = get_hold_time_stats(conn, darwin_ticker) {
        // Longer avg hold = more capacity (less market impact)
        let duration_score = (hold_stats.avg_hold_hours / 24.0).min(10.0); // 10 days = 10
        duration_score.max(0.0).min(10.0)
    } else { 5.0 };

    // ── Scalability (0-10): symbol diversity + sizing consistency
    let scalability = if let Ok(sizing) = get_sizing_efficiency(conn, darwin_ticker) {
        // Count symbols traded
        let symbols: Result<Vec<SymbolActivity>, _> = get_symbol_rotation(conn, darwin_ticker);
        let sym_count = symbols.map(|s| s.len()).unwrap_or(1) as f64;
        let diversity_score = (sym_count / 5.0).min(5.0); // 25 symbols = 5

        // Sizing consistency: compare Q1 vs Q4 win rates
        let sizing_consistency = if sizing.len() >= 4 {
            let q1_wr = sizing[0].win_rate;
            let q4_wr = sizing[3].win_rate;
            let diff = (q1_wr - q4_wr).abs();
            // Low difference = consistent = good
            (5.0 - diff / 10.0).max(0.0).min(5.0)
        } else { 2.5 };

        (diversity_score + sizing_consistency).min(10.0)
    } else { 5.0 };

    // ── Total D-Score (weighted composite, 0-100)
    // Darwinex weights: Ex 15%, Rs 25%, Pf 25%, Mc 15%, Cp 10%, Sc 10%
    let total_dscore = experience * 1.5
        + risk_mgmt * 2.5
        + performance * 2.5
        + market_timing * 1.5
        + capacity * 1.0
        + scalability * 1.0;

    Ok(DScoreEstimate {
        darwin_ticker: darwin_ticker.to_string(),
        experience,
        risk_mgmt,
        performance,
        market_timing,
        capacity,
        scalability,
        total_dscore,
        investable_months,
    })
}

// ── Symbol Overlap (cross-DARWIN correlation) ────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolOverlap {
    pub symbol: String,
    pub side: String,
    pub darwin_count: usize,
    pub darwins: Vec<String>,
    pub total_volume: f64,
    pub total_notional: f64,
    pub correlation_risk: String, // "HIGH" (5-6), "MEDIUM" (3-4), "LOW" (1-2)
}

/// Find symbols that appear across multiple DARWINs on the same side.
pub fn get_symbol_overlap(conn: &Connection) -> Result<Vec<SymbolOverlap>, String> {
    let accounts = list_darwin_accounts(conn)?;

    // (symbol, side) -> Vec<(darwin_ticker, volume, notional)>
    let mut map: std::collections::HashMap<(String, String), Vec<(String, f64, f64)>> =
        std::collections::HashMap::new();

    for account in &accounts {
        let positions = get_darwin_open_positions(conn, &account.darwin_ticker)?;
        for pos in positions {
            let key = (pos.symbol.clone(), pos.side.clone());
            map.entry(key)
                .or_default()
                .push((account.darwin_ticker.clone(), pos.total_volume, pos.notional));
        }
    }

    let mut result: Vec<SymbolOverlap> = map
        .into_iter()
        .map(|((symbol, side), entries)| {
            let darwin_count = entries.len();
            let darwins: Vec<String> = entries.iter().map(|(t, _, _)| t.clone()).collect();
            let total_volume: f64 = entries.iter().map(|(_, v, _)| v).sum();
            let total_notional: f64 = entries.iter().map(|(_, _, n)| n).sum();
            let correlation_risk = if darwin_count >= 5 {
                "HIGH".to_string()
            } else if darwin_count >= 3 {
                "MEDIUM".to_string()
            } else {
                "LOW".to_string()
            };
            SymbolOverlap {
                symbol,
                side,
                darwin_count,
                darwins,
                total_volume,
                total_notional,
                correlation_risk,
            }
        })
        .collect();

    // Sort by darwin_count desc, then total_notional desc
    result.sort_by(|a, b| {
        b.darwin_count
            .cmp(&a.darwin_count)
            .then_with(|| b.total_notional.partial_cmp(&a.total_notional).unwrap_or(std::cmp::Ordering::Equal))
    });

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

// ── Darwinex Metrics ────────────────────────────────────────────────

/// Compute CAGR from daily returns.
pub fn compute_cagr(daily_returns: &[DailyReturn]) -> f64 {
    if daily_returns.len() < 2 { return 0.0; }
    let first_balance = daily_returns.first().map(|d| d.balance).unwrap_or(1.0);
    let last_balance = daily_returns.last().map(|d| d.balance).unwrap_or(1.0);
    if first_balance <= 0.0 { return 0.0; }
    let years = daily_returns.len() as f64 / 252.0; // trading days
    if years <= 0.0 { return 0.0; }
    ((last_balance / first_balance).powf(1.0 / years) - 1.0) * 100.0
}

/// Recovery Factor = net profit / max drawdown (higher = faster recovery).
pub fn compute_recovery_factor(daily_returns: &[DailyReturn]) -> f64 {
    if daily_returns.len() < 2 { return 0.0; }
    let first_balance = daily_returns.first().map(|d| d.balance).unwrap_or(0.0);
    let last_balance = daily_returns.last().map(|d| d.balance).unwrap_or(0.0);
    let net_profit = last_balance - first_balance;
    let max_dd = daily_returns.iter().map(|d| d.drawdown_pct.abs()).fold(0.0_f64, f64::max);
    if max_dd <= 0.0 { return 0.0; }
    let max_dd_abs = first_balance * max_dd / 100.0;
    if max_dd_abs <= 0.0 { return 0.0; }
    net_profit / max_dd_abs
}

/// Maximum drawdown duration — how many trading days from peak to recovery (or ongoing).
/// Returns (max_dd_duration_days, current_dd_duration_days, avg_dd_duration_days).
pub fn compute_drawdown_duration(daily_returns: &[DailyReturn]) -> (usize, usize, f64) {
    if daily_returns.is_empty() { return (0, 0, 0.0); }
    let mut peak = daily_returns[0].balance;
    let mut dd_start = 0usize;
    let mut in_drawdown = false;
    let mut max_duration = 0usize;
    let mut current_duration = 0usize;
    let mut durations: Vec<usize> = Vec::new();

    for (i, d) in daily_returns.iter().enumerate() {
        if d.balance >= peak {
            peak = d.balance;
            if in_drawdown {
                let dur = i - dd_start;
                durations.push(dur);
                max_duration = max_duration.max(dur);
                in_drawdown = false;
            }
        } else if !in_drawdown {
            in_drawdown = true;
            dd_start = i;
        }
    }
    // If still in drawdown at end
    if in_drawdown {
        current_duration = daily_returns.len() - dd_start;
        max_duration = max_duration.max(current_duration);
    }
    let avg = if durations.is_empty() { 0.0 } else { durations.iter().sum::<usize>() as f64 / durations.len() as f64 };
    (max_duration, current_duration, avg)
}

/// Divergence Index — tracks signal return vs DARWIN quote return over time.
/// Signal returns from daily_returns, quote returns from FTP cumulative_returns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivergencePoint {
    pub day_index: usize,
    pub signal_return_pct: f64,
    pub quote_return_pct: f64,
    pub divergence_pct: f64,  // quote - signal (positive = quote outperforms)
}

pub fn compute_divergence_index(
    daily_returns: &[DailyReturn],
    ftp_returns: &[(f64, f64)],  // (day_index, quote_price) from ftp_equity_curve
) -> Vec<DivergencePoint> {
    if daily_returns.is_empty() || ftp_returns.is_empty() { return Vec::new(); }
    let initial_balance = daily_returns.first().map(|d| d.balance).unwrap_or(1.0);
    if initial_balance <= 0.0 { return Vec::new(); }

    let mut result = Vec::new();
    let quote_start = ftp_returns.first().map(|&(_, p)| p).unwrap_or(100.0);
    if quote_start <= 0.0 { return Vec::new(); }

    // Align by index (both are day-indexed)
    let n = daily_returns.len().min(ftp_returns.len());
    for i in 0..n {
        let signal_ret = (daily_returns[i].balance / initial_balance - 1.0) * 100.0;
        let quote_ret = (ftp_returns[i].1 / quote_start - 1.0) * 100.0;
        result.push(DivergencePoint {
            day_index: i,
            signal_return_pct: signal_ret,
            quote_return_pct: quote_ret,
            divergence_pct: quote_ret - signal_ret,
        });
    }
    result
}

/// Investment velocity — rate of AUM change over time.
/// Returns monthly AUM growth rate %.
pub fn compute_investment_velocity(investor_flow: &[InvestorFlow]) -> Vec<(String, f64)> {
    if investor_flow.len() < 2 { return Vec::new(); }
    let mut result = Vec::new();
    for i in 1..investor_flow.len() {
        let prev_aum = investor_flow[i-1].aum;
        let curr_aum = investor_flow[i].aum;
        let growth = if prev_aum > 0.0 { (curr_aum / prev_aum - 1.0) * 100.0 } else { 0.0 };
        result.push((investor_flow[i].date.clone(), growth));
    }
    result
}

// ── Advanced DARWIN Analytics ────────────────────────────────────────

/// Rolling correlation between two DARWINs over time.
/// Returns Vec of (window_end_idx, correlation) for sliding windows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollingCorrelation {
    pub darwin_a: String,
    pub darwin_b: String,
    pub points: Vec<(usize, f64)>,  // (day_index, correlation)
}

pub fn compute_rolling_correlation(
    conn: &Connection,
    ticker_a: &str,
    ticker_b: &str,
    window: usize,  // typically 45 (Darwinex standard)
) -> Result<RollingCorrelation, String> {
    let returns_a = get_daily_returns(conn, ticker_a)?;
    let returns_b = get_daily_returns(conn, ticker_b)?;

    if returns_a.len() < window || returns_b.len() < window {
        return Ok(RollingCorrelation { darwin_a: ticker_a.into(), darwin_b: ticker_b.into(), points: Vec::new() });
    }

    // Align by date
    let dates_b: std::collections::HashMap<String, f64> = returns_b.iter()
        .map(|d| (d.date.clone(), d.return_pct)).collect();

    let aligned: Vec<(f64, f64)> = returns_a.iter()
        .filter_map(|a| dates_b.get(&a.date).map(|b_ret| (a.return_pct, *b_ret)))
        .collect();

    let mut points = Vec::new();
    for i in window..=aligned.len() {
        let slice = &aligned[i-window..i];
        let mean_a = slice.iter().map(|p| p.0).sum::<f64>() / window as f64;
        let mean_b = slice.iter().map(|p| p.1).sum::<f64>() / window as f64;
        let mut cov = 0.0;
        let mut var_a = 0.0;
        let mut var_b = 0.0;
        for (a, b) in slice {
            cov += (a - mean_a) * (b - mean_b);
            var_a += (a - mean_a).powi(2);
            var_b += (b - mean_b).powi(2);
        }
        let denom = (var_a * var_b).sqrt();
        let corr = if denom > 0.0 { cov / denom } else { 0.0 };
        points.push((i, corr));
    }

    Ok(RollingCorrelation { darwin_a: ticker_a.into(), darwin_b: ticker_b.into(), points })
}

/// Marginal drawdown contribution per DARWIN to portfolio drawdown.
/// Shows which DARWIN caused the most damage during peak-to-trough.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawdownAttribution {
    pub darwin_ticker: String,
    pub contribution_pct: f64,    // % of portfolio DD attributable to this DARWIN
    pub standalone_dd_pct: f64,   // this DARWIN's own DD during the same period
    pub weight_at_peak: f64,      // allocation weight when portfolio peaked
}

pub fn compute_drawdown_attribution(conn: &Connection) -> Result<Vec<DrawdownAttribution>, String> {
    let accounts = list_darwin_accounts(conn)?;
    if accounts.is_empty() { return Ok(Vec::new()); }

    // Get daily returns per DARWIN
    let mut all_returns: Vec<(String, Vec<DailyReturn>)> = Vec::new();
    for acct in &accounts {
        if let Ok(daily) = get_daily_returns(conn, &acct.darwin_ticker) {
            if !daily.is_empty() {
                all_returns.push((acct.darwin_ticker.clone(), daily));
            }
        }
    }
    if all_returns.is_empty() { return Ok(Vec::new()); }

    // Compute portfolio equity curve (sum of all DARWIN balances per day)
    // Align all DARWINs by date
    let mut date_set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (_, returns) in &all_returns {
        for d in returns { date_set.insert(d.date.clone()); }
    }
    let dates: Vec<String> = date_set.into_iter().collect();

    // Build portfolio daily balance + per-DARWIN contribution
    let mut portfolio_balance = Vec::new();
    let mut per_darwin_balance: Vec<Vec<f64>> = vec![Vec::new(); all_returns.len()];

    for date in &dates {
        let mut total = 0.0;
        for (i, (_, returns)) in all_returns.iter().enumerate() {
            let bal = returns.iter().find(|d| d.date == *date).map(|d| d.balance).unwrap_or(0.0);
            per_darwin_balance[i].push(bal);
            total += bal;
        }
        portfolio_balance.push(total);
    }

    if portfolio_balance.len() < 2 { return Ok(Vec::new()); }

    // Find portfolio peak and trough
    let mut peak_idx = 0;
    let mut peak_val = portfolio_balance[0];
    let mut trough_idx = 0;
    let mut max_dd = 0.0;

    for (i, &bal) in portfolio_balance.iter().enumerate() {
        if bal > peak_val { peak_val = bal; peak_idx = i; }
        let dd = if peak_val > 0.0 { (peak_val - bal) / peak_val } else { 0.0 };
        if dd > max_dd { max_dd = dd; trough_idx = i; }
    }

    if trough_idx <= peak_idx || peak_val <= 0.0 { return Ok(Vec::new()); }

    // Compute each DARWIN's contribution during peak-to-trough
    let mut result = Vec::new();
    let portfolio_loss = portfolio_balance[peak_idx] - portfolio_balance[trough_idx];

    for (i, (ticker, _)) in all_returns.iter().enumerate() {
        let darwin_at_peak = per_darwin_balance[i].get(peak_idx).copied().unwrap_or(0.0);
        let darwin_at_trough = per_darwin_balance[i].get(trough_idx).copied().unwrap_or(0.0);
        let darwin_loss = darwin_at_peak - darwin_at_trough;

        let contribution = if portfolio_loss > 0.0 { darwin_loss / portfolio_loss * 100.0 } else { 0.0 };
        let standalone_dd = if darwin_at_peak > 0.0 { (darwin_at_peak - darwin_at_trough) / darwin_at_peak * 100.0 } else { 0.0 };
        let weight = if peak_val > 0.0 { darwin_at_peak / peak_val * 100.0 } else { 0.0 };

        result.push(DrawdownAttribution {
            darwin_ticker: ticker.clone(),
            contribution_pct: contribution,
            standalone_dd_pct: standalone_dd,
            weight_at_peak: weight,
        });
    }

    result.sort_by(|a, b| b.contribution_pct.partial_cmp(&a.contribution_pct).unwrap_or(std::cmp::Ordering::Equal));
    Ok(result)
}

/// Signal decay: rolling Sharpe ratio over time to detect strategy degradation.
/// Returns Vec of (window_end_date, rolling_sharpe) per DARWIN.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalDecay {
    pub darwin_ticker: String,
    pub points: Vec<(String, f64)>,  // (date, rolling_sharpe)
    pub current_sharpe: f64,
    pub peak_sharpe: f64,
    pub decay_pct: f64,  // (peak - current) / peak * 100 — 0% = no decay, 100% = fully decayed
}

pub fn compute_signal_decay(conn: &Connection, ticker: &str, window: usize) -> Result<SignalDecay, String> {
    let daily = get_daily_returns(conn, ticker)?;
    if daily.len() < window { return Err("Not enough data for decay analysis".into()); }

    let mut points = Vec::new();
    let mut peak_sharpe = f64::NEG_INFINITY;

    for i in window..=daily.len() {
        let slice = &daily[i-window..i];
        let returns: Vec<f64> = slice.iter().map(|d| d.return_pct).collect();
        let n = returns.len() as f64;
        let mean = returns.iter().sum::<f64>() / n;
        let var = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0);
        let std = var.sqrt();
        let sharpe = if std > 0.0 { mean / std * (252.0_f64).sqrt() } else { 0.0 };

        let date = slice.last().map(|d| d.date.clone()).unwrap_or_default();
        points.push((date, sharpe));
        if sharpe > peak_sharpe { peak_sharpe = sharpe; }
    }

    let current = points.last().map(|p| p.1).unwrap_or(0.0);
    let decay = if peak_sharpe > 0.0 { ((peak_sharpe - current) / peak_sharpe * 100.0).max(0.0) } else { 0.0 };

    Ok(SignalDecay {
        darwin_ticker: ticker.into(),
        points,
        current_sharpe: current,
        peak_sharpe: if peak_sharpe.is_finite() { peak_sharpe } else { 0.0 },
        decay_pct: decay,
    })
}

// ── Replication Quality ─────────────────────────────────────────────

/// Replication quality: how well does the DARWIN quote track the signal?
/// Lower tracking error = better replication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationQuality {
    pub darwin_ticker: String,
    pub tracking_error: f64,      // annualized std dev of (signal_ret - quote_ret)
    pub information_ratio: f64,   // (signal_ret - quote_ret) / tracking_error
    pub r_squared: f64,           // R² of signal vs quote returns
    pub avg_lag_days: f64,        // average number of days quote lags signal
    pub quality_grade: String,    // "A" (excellent) to "F" (poor)
}

pub fn compute_replication_quality(
    daily_returns: &[DailyReturn],
    ftp_equity_curve: &[(f64, f64)],  // (day_index, quote_price)
) -> Option<ReplicationQuality> {
    if daily_returns.len() < 30 || ftp_equity_curve.len() < 30 { return None; }
    let initial = daily_returns.first()?.balance;
    if initial <= 0.0 { return None; }
    let quote_start = ftp_equity_curve.first()?.1;
    if quote_start <= 0.0 { return None; }

    let n = daily_returns.len().min(ftp_equity_curve.len());
    let mut signal_rets = Vec::new();
    let mut quote_rets = Vec::new();

    for i in 1..n {
        let sr = daily_returns[i].return_pct;
        let qr = if ftp_equity_curve[i].1 > 0.0 && ftp_equity_curve[i-1].1 > 0.0 {
            (ftp_equity_curve[i].1 / ftp_equity_curve[i-1].1 - 1.0) * 100.0
        } else { 0.0 };
        signal_rets.push(sr);
        quote_rets.push(qr);
    }

    if signal_rets.is_empty() { return None; }
    let nn = signal_rets.len() as f64;

    // Tracking error: annualized std dev of difference
    let diffs: Vec<f64> = signal_rets.iter().zip(&quote_rets).map(|(s, q)| s - q).collect();
    let diff_mean = diffs.iter().sum::<f64>() / nn;
    let diff_var = diffs.iter().map(|d| (d - diff_mean).powi(2)).sum::<f64>() / (nn - 1.0);
    let tracking_error = diff_var.sqrt() * (252.0_f64).sqrt();

    // Information ratio
    let info_ratio = if tracking_error > 0.0 { diff_mean * 252.0 / tracking_error } else { 0.0 };

    // R-squared
    let mean_s = signal_rets.iter().sum::<f64>() / nn;
    let mean_q = quote_rets.iter().sum::<f64>() / nn;
    let mut cov = 0.0;
    let mut var_s = 0.0;
    let mut var_q = 0.0;
    for i in 0..signal_rets.len() {
        cov += (signal_rets[i] - mean_s) * (quote_rets[i] - mean_q);
        var_s += (signal_rets[i] - mean_s).powi(2);
        var_q += (quote_rets[i] - mean_q).powi(2);
    }
    let r = if var_s > 0.0 && var_q > 0.0 { cov / (var_s * var_q).sqrt() } else { 0.0 };
    let r_squared = r * r;

    // Quality grade
    let grade = if r_squared > 0.95 && tracking_error < 5.0 { "A" }
        else if r_squared > 0.90 && tracking_error < 10.0 { "B" }
        else if r_squared > 0.80 { "C" }
        else if r_squared > 0.60 { "D" }
        else { "F" };

    Some(ReplicationQuality {
        darwin_ticker: String::new(),
        tracking_error,
        information_ratio: info_ratio,
        r_squared,
        avg_lag_days: 0.0, // would need cross-correlation to compute properly
        quality_grade: grade.to_string(),
    })
}

// ── Risk Budget Consumption ─────────────────────────────────────────

/// Risk budget: how much of the VaR corridor is each DARWIN consuming?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskBudget {
    pub darwin_ticker: String,
    pub standalone_var: f64,        // this DARWIN's individual VaR
    pub marginal_var: f64,          // additional VaR this DARWIN adds to portfolio
    pub component_var: f64,         // proportion of portfolio VaR due to this DARWIN
    pub risk_contribution_pct: f64, // component_var / portfolio_var * 100
    pub diversification_benefit: f64, // standalone - marginal (positive = diversifies)
}

pub fn compute_risk_budget(conn: &Connection) -> Result<Vec<RiskBudget>, String> {
    let accounts = list_darwin_accounts(conn)?;
    if accounts.len() < 2 { return Ok(Vec::new()); }

    // Get daily returns per DARWIN
    let mut all_returns: Vec<(String, Vec<f64>)> = Vec::new();
    for acct in &accounts {
        if let Ok(daily) = get_daily_returns(conn, &acct.darwin_ticker) {
            let rets: Vec<f64> = daily.iter().map(|d| d.return_pct).collect();
            if rets.len() >= 30 {
                all_returns.push((acct.darwin_ticker.clone(), rets));
            }
        }
    }
    if all_returns.len() < 2 { return Ok(Vec::new()); }

    // Portfolio daily returns (sum of all)
    let max_len = all_returns.iter().map(|(_, r)| r.len()).min().unwrap_or(0);
    let portfolio_rets: Vec<f64> = (0..max_len).map(|i| {
        all_returns.iter().map(|(_, r)| r.get(i).copied().unwrap_or(0.0)).sum()
    }).collect();

    let portfolio_var = var_95(&portfolio_rets);

    let mut result = Vec::new();
    for (ticker, rets) in &all_returns {
        let standalone = var_95(rets);

        // Marginal VaR: portfolio VaR without this DARWIN
        let without: Vec<f64> = (0..max_len).map(|i| {
            all_returns.iter()
                .filter(|(t, _)| t != ticker)
                .map(|(_, r)| r.get(i).copied().unwrap_or(0.0))
                .sum()
        }).collect();
        let var_without = var_95(&without);
        let marginal = portfolio_var - var_without;

        let risk_contrib = if portfolio_var.abs() > 0.0 { marginal / portfolio_var * 100.0 } else { 0.0 };

        result.push(RiskBudget {
            darwin_ticker: ticker.clone(),
            standalone_var: standalone,
            marginal_var: marginal,
            component_var: marginal,
            risk_contribution_pct: risk_contrib,
            diversification_benefit: standalone - marginal,
        });
    }

    result.sort_by(|a, b| b.risk_contribution_pct.partial_cmp(&a.risk_contribution_pct).unwrap_or(std::cmp::Ordering::Equal));
    Ok(result)
}

/// Helper: compute VaR 95% from returns
fn var_95(returns: &[f64]) -> f64 {
    if returns.len() < 10 { return 0.0; }
    let mut sorted = returns.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = (sorted.len() as f64 * 0.05) as usize;
    sorted.get(idx).copied().unwrap_or(0.0).abs()
}

// ── Performance Attribution by Symbol ───────────────────────────────

/// Performance attribution: which symbols drive returns for each DARWIN?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolAttribution {
    pub symbol: String,
    pub total_pnl: f64,
    pub trade_count: i64,
    pub win_rate: f64,
    pub avg_pnl: f64,
    pub contribution_pct: f64,  // % of total DARWIN P&L from this symbol
}

pub fn compute_performance_attribution(conn: &Connection, darwin_ticker: &str) -> Result<Vec<SymbolAttribution>, String> {
    let positions = get_darwin_positions(conn, darwin_ticker, None, None)?;
    if positions.is_empty() { return Ok(Vec::new()); }

    let total_pnl: f64 = positions.iter().map(|p| p.profit).sum();

    let mut by_symbol: std::collections::HashMap<String, (f64, i64, i64)> = std::collections::HashMap::new(); // (pnl, wins, total)
    for pos in &positions {
        let entry = by_symbol.entry(pos.symbol.clone()).or_insert((0.0, 0, 0));
        entry.0 += pos.profit;
        if pos.profit > 0.0 { entry.1 += 1; }
        entry.2 += 1;
    }

    let mut result: Vec<SymbolAttribution> = by_symbol.into_iter().map(|(sym, (pnl, wins, total))| {
        SymbolAttribution {
            symbol: sym,
            total_pnl: pnl,
            trade_count: total,
            win_rate: if total > 0 { wins as f64 / total as f64 * 100.0 } else { 0.0 },
            avg_pnl: if total > 0 { pnl / total as f64 } else { 0.0 },
            contribution_pct: if total_pnl.abs() > 0.0 { pnl / total_pnl * 100.0 } else { 0.0 },
        }
    }).collect();

    result.sort_by(|a, b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));
    Ok(result)
}

// ── LAN Sync: DARWIN data export/import ──────────────────────────────

/// Export all DARWIN data (accounts, deals, positions) as compressed JSON for LAN sync.
pub fn export_darwin_data(conn: &Connection) -> Result<(String, usize, usize, usize), String> {
    let accounts = list_darwin_accounts(conn)?;
    let mut all_deals: Vec<DarwinDeal> = Vec::new();
    let mut all_positions: Vec<DarwinPosition> = Vec::new();
    for acct in &accounts {
        if let Ok(deals) = get_darwin_deals(conn, &acct.darwin_ticker, None, None) {
            all_deals.extend(deals);
        }
        if let Ok(positions) = get_darwin_positions(conn, &acct.darwin_ticker, None, None) {
            all_positions.extend(positions);
        }
    }
    let payload = serde_json::json!({
        "accounts": accounts,
        "deals": all_deals,
        "positions": all_positions,
    });
    let json = serde_json::to_string(&payload).map_err(|e| format!("JSON serialize failed: {e}"))?;
    let n_acct = accounts.len();
    let n_deals = all_deals.len();
    let n_pos = all_positions.len();
    Ok((json, n_acct, n_deals, n_pos))
}

/// Import DARWIN data from JSON (received via LAN sync).
/// Merges into existing tables (INSERT OR REPLACE).
pub fn import_darwin_data(conn: &Connection, json: &str) -> Result<(usize, usize, usize), String> {
    let payload: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| format!("JSON parse failed: {e}"))?;

    // Ensure tables exist
    create_darwin_tables(conn)?;

    // Wrap entire DELETE + INSERT in a transaction.
    // Without this, a failed DELETE leaves old data, and new INSERTs add duplicates
    // (AUTOINCREMENT ids → INSERT OR REPLACE always creates new rows).
    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(|e| format!("BEGIN IMMEDIATE failed: {e}"))?;

    let mut n_acct = 0usize;
    let mut n_deals = 0usize;
    let mut n_pos = 0usize;

    // Clear ALL deals and positions before reimporting — this is a full snapshot from the server.
    // Without this, AUTOINCREMENT ids cause duplicates on every sync (INSERT OR REPLACE
    // creates new rows when PK is auto-generated). Also prevents stale data from
    // accounts that may have been removed on the server.
    // CRITICAL: these DELETEs must succeed. If they fail (e.g., database locked), the
    // subsequent INSERTs add duplicates → stale positions computed as "open".
    conn.execute("DELETE FROM darwin_deals", [])
        .map_err(|e| format!("DELETE darwin_deals failed: {e}"))?;
    conn.execute("DELETE FROM darwin_positions", [])
        .map_err(|e| format!("DELETE darwin_positions failed: {e}"))?;
    conn.execute("DELETE FROM darwin_accounts", [])
        .map_err(|e| format!("DELETE darwin_accounts failed: {e}"))?;

    if let Some(accounts) = payload["accounts"].as_array() {
        for a in accounts {
            let ticker = a["darwin_ticker"].as_str().unwrap_or("");
            if ticker.is_empty() { continue; }
            conn.execute(
                "INSERT OR REPLACE INTO darwin_accounts (darwin_ticker, name, mt5_account, initial_balance, created_at, deal_count, position_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    ticker,
                    a["name"].as_str().unwrap_or(""),
                    a["mt5_account"].as_str().unwrap_or(""),
                    a["initial_balance"].as_f64().unwrap_or(0.0),
                    a["created_at"].as_i64().unwrap_or(0),
                    a["deal_count"].as_i64().unwrap_or(0),
                    a["position_count"].as_i64().unwrap_or(0),
                ],
            ).map_err(|e| format!("Insert account failed: {e}"))?;
            n_acct += 1;
        }
    }

    // Import deals (table was cleared per-account above, no duplicates)
    if let Some(deals) = payload["deals"].as_array() {
        for d in deals {
            conn.execute(
                "INSERT OR REPLACE INTO darwin_deals (account, time, deal_ticket, symbol, deal_type, direction, volume, price, order_ticket, commission, fee, swap, profit, balance, comment) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                rusqlite::params![
                    d["account"].as_str().unwrap_or(""),
                    d["time"].as_str().unwrap_or(""),
                    d["deal_ticket"].as_i64().unwrap_or(0),
                    d["symbol"].as_str().unwrap_or(""),
                    d["deal_type"].as_str().unwrap_or(""),
                    d["direction"].as_str().unwrap_or(""),
                    d["volume"].as_f64().unwrap_or(0.0),
                    d["price"].as_f64().unwrap_or(0.0),
                    d["order_ticket"].as_i64().unwrap_or(0),
                    d["commission"].as_f64().unwrap_or(0.0),
                    d["fee"].as_f64().unwrap_or(0.0),
                    d["swap"].as_f64().unwrap_or(0.0),
                    d["profit"].as_f64().unwrap_or(0.0),
                    d["balance"].as_f64().unwrap_or(0.0),
                    d["comment"].as_str().unwrap_or(""),
                ],
            ).map_err(|e| format!("Insert deal failed: {e}"))?;
            n_deals += 1;
        }
    }

    // Import positions
    if let Some(positions) = payload["positions"].as_array() {
        for p in positions {
            conn.execute(
                "INSERT OR REPLACE INTO darwin_positions (account, open_time, position_ticket, symbol, pos_type, volume, open_price, sl, tp, close_time, close_price, commission, swap, profit) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                rusqlite::params![
                    p["account"].as_str().unwrap_or(""),
                    p["open_time"].as_str().unwrap_or(""),
                    p["position_ticket"].as_i64().unwrap_or(0),
                    p["symbol"].as_str().unwrap_or(""),
                    p["pos_type"].as_str().unwrap_or(""),
                    p["volume"].as_f64().unwrap_or(0.0),
                    p["open_price"].as_f64().unwrap_or(0.0),
                    p["sl"].as_f64().unwrap_or(0.0),
                    p["tp"].as_f64().unwrap_or(0.0),
                    p["close_time"].as_str().unwrap_or(""),
                    p["close_price"].as_f64().unwrap_or(0.0),
                    p["commission"].as_f64().unwrap_or(0.0),
                    p["swap"].as_f64().unwrap_or(0.0),
                    p["profit"].as_f64().unwrap_or(0.0),
                ],
            ).map_err(|e| format!("Insert position failed: {e}"))?;
            n_pos += 1;
        }
    }

    conn.execute_batch("COMMIT")
        .map_err(|e| format!("COMMIT failed: {e}"))?;

    Ok((n_acct, n_deals, n_pos))
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

    #[test]
    fn test_seasonals() {
        let conn = setup_test_db();
        let returns = get_daily_returns(&conn, "TEST").unwrap();
        let seasonals = get_seasonal_analysis(&returns);
        assert!(!seasonals.is_empty());
        // June should be present (our test data is in June)
        let june = seasonals.iter().find(|s| s.month == 6);
        assert!(june.is_some());
        assert_eq!(june.unwrap().month_name, "June");
        assert!(june.unwrap().sample_count >= 1);
    }

    #[test]
    fn test_mae_mfe() {
        let conn = setup_test_db();
        let result = estimate_mae_mfe(&conn, "TEST").unwrap();
        assert!(result.avg_mae_pct >= 0.0);
        assert!(result.avg_mfe_pct >= 0.0);
        assert!(!result.entries.is_empty());
    }

    #[test]
    fn test_what_if() {
        let conn = setup_test_db();
        // TSLA is open, so closing it should change VaR
        let result = what_if_close_symbol(&conn, "TSLA");
        // May succeed or fail depending on data sufficiency
        if let Ok(r) = result {
            assert!(!r.action.is_empty());
        }
    }

    #[test]
    fn test_liquidity_risk() {
        let conn = setup_test_db();
        let risk = get_liquidity_risk(&conn).unwrap();
        // Should find TSLA open position
        if !risk.is_empty() {
            for r in &risk {
                assert!(!r.symbol.is_empty());
                assert!(!r.risk_tier.is_empty());
                assert!(r.days_to_exit >= 0.0);
            }
        }
    }

    #[test]
    fn test_tail_risk() {
        let conn = setup_test_db();
        let returns = get_daily_returns(&conn, "TEST").unwrap();
        let tail = compute_tail_risk(&returns);
        // Skewness and kurtosis should be finite
        assert!(tail.skewness.is_finite());
        assert!(tail.kurtosis.is_finite());
        assert!(tail.ulcer_index >= 0.0);
    }

    #[test]
    fn test_trading_bursts() {
        let conn = setup_test_db();
        let bursts = detect_trading_bursts(&conn, "TEST").unwrap();
        // Should detect at least one week of activity
        assert!(!bursts.is_empty());
        for b in &bursts {
            assert!(b.trade_count > 0);
            assert!(!b.intensity.is_empty());
        }
    }

    #[test]
    fn test_pyramiding() {
        let conn = setup_test_db();
        let pyramids = analyze_pyramiding(&conn, "TEST").unwrap();
        // AAPL has multiple buy entries, should detect pyramiding
        if !pyramids.is_empty() {
            for p in &pyramids {
                assert!(!p.symbol.is_empty());
                assert!(!p.strategy.is_empty());
            }
        }
    }

    #[test]
    fn test_alerts() {
        let conn = setup_test_db();
        let alerts = check_alerts(&conn).unwrap();
        // Alerts may or may not fire on test data
        for a in &alerts {
            assert!(!a.alert_type.is_empty());
            assert!(!a.severity.is_empty());
            assert!(!a.message.is_empty());
        }
    }

    #[test]
    fn test_dscore_components_missing() {
        let result = get_dscore_components("/nonexistent", "TEST");
        assert!(result.is_err() || result.unwrap().experience.is_none());
    }

    #[test]
    fn test_margin_call_sim() {
        let conn = setup_test_db();
        let result = simulate_margin_call(&conn);
        if let Ok(m) = result {
            assert!(m.current_equity > 0.0);
            assert!(m.margin_level_pct >= 0.0);
            assert!(m.probability_30d >= 0.0);
        }
    }

    #[test]
    fn test_slippage() {
        let conn = setup_test_db();
        let result = analyze_slippage(&conn, "TEST").unwrap();
        assert!(result.avg_slippage_pct.is_finite());
    }

    #[test]
    fn test_optimal_allocation() {
        let conn = setup_test_db();
        let result = compute_optimal_allocation(&conn).unwrap();
        assert!(!result.is_empty());
        let total_weight: f64 = result.iter().map(|a| a.optimal_weight).sum();
        assert!((total_weight - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_conditional_var() {
        let mut returns = Vec::new();
        let mut balance = 100000.0;
        for i in 0..100 {
            let pnl = if i % 3 == 0 { -200.0 } else { 150.0 };
            balance += pnl;
            returns.push(DailyReturn {
                date: format!("2024.{:02}.{:02}", (i / 28) + 1, (i % 28) + 1),
                pnl, balance, return_pct: pnl / (balance - pnl) * 100.0, drawdown_pct: 0.0,
            });
        }
        let cv = compute_conditional_var(&returns);
        // Should have up to 3 regimes
        assert!(cv.len() <= 3);
        for c in &cv {
            assert!(!c.regime.is_empty());
            assert!(c.days_in_regime > 0);
        }
    }

    #[test]
    fn test_market_regime() {
        let mut returns = Vec::new();
        let mut balance = 100000.0;
        for i in 0..60 {
            let pnl = if i % 3 == 0 { -200.0 } else { 150.0 };
            balance += pnl;
            returns.push(DailyReturn {
                date: format!("2024.06.{:02}", (i % 28) + 1),
                pnl, balance, return_pct: pnl / (balance - pnl) * 100.0, drawdown_pct: 0.0,
            });
        }
        let regime = detect_market_regime(&returns);
        assert!(!regime.current_regime.is_empty());
        assert!(regime.rolling_vol >= 0.0);
    }

    #[test]
    fn test_exposure_treemap() {
        let conn = setup_test_db();
        let tree = get_exposure_treemap(&conn).unwrap();
        assert_eq!(tree.name, "Portfolio");
    }
}
