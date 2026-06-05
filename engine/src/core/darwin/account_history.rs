use rusqlite::{Connection, params};

use super::*;

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
    use std::io::{Cursor, Read, Write};
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
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp_path)
            .map_err(|e| format!("Create tmp failed: {e}"))?
    };
    #[cfg(not(unix))]
    let out_file =
        std::fs::File::create(&tmp_path).map_err(|e| format!("Create tmp failed: {e}"))?;
    let mut writer = zip::ZipWriter::new(out_file);
    let mut archive =
        zip::ZipArchive::new(Cursor::new(&data)).map_err(|e| format!("ZIP reopen failed: {e}"))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("ZIP entry {i} failed: {e}"))?;
        let name = entry.name().to_string();
        let opts = zip::write::SimpleFileOptions::default().compression_method(entry.compression());
        writer
            .start_file(&name, opts)
            .map_err(|e| format!("ZIP write failed: {e}"))?;

        let mut raw = Vec::new();
        entry
            .read_to_end(&mut raw)
            .map_err(|e| format!("ZIP read failed: {e}"))?;

        // Convert ANY entry with UTF-16 LE BOM (.xml, .rels, [Content_Types].xml, etc.)
        if raw.len() >= 2 && raw[0] == 0xFF && raw[1] == 0xFE {
            let utf16: Vec<u16> = raw[2..]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            let utf8 = String::from_utf16(&utf16)
                .map_err(|e| format!("UTF-16 decode failed in {name}: {e}"))?;
            writer
                .write_all(utf8.as_bytes())
                .map_err(|e| format!("Write failed: {e}"))?;
        } else {
            writer
                .write_all(&raw)
                .map_err(|e| format!("Write failed: {e}"))?;
        }
    }
    writer
        .finish()
        .map_err(|e| format!("ZIP finalize failed: {e}"))?;
    Ok(tmp_path)
}

pub fn import_darwin_xlsx(
    conn: &Connection,
    xlsx_path: &str,
    darwin_ticker: &str,
) -> Result<(String, usize, usize), String> {
    use calamine::{Reader, Xlsx, open_workbook};

    // MT5 exports UTF-16 LE XML inside XLSX — convert to UTF-8 if needed
    let effective_path = fix_utf16_xlsx(xlsx_path)?;
    let mut workbook: Xlsx<_> =
        open_workbook(&effective_path).map_err(|e| format!("Failed to open XLSX: {e}"))?;
    // Clean up temp file after opening (workbook reads into memory)
    if effective_path != xlsx_path {
        let _ = std::fs::remove_file(&effective_path);
    }

    let sheet_name = workbook
        .sheet_names()
        .first()
        .ok_or("No sheets in workbook")?
        .clone();
    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|e| format!("Failed to read sheet: {e}"))?;

    let rows: Vec<Vec<calamine::Data>> = range.rows().map(|r| r.to_vec()).collect();

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
        if row.is_empty() {
            continue;
        }
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
    conn.execute_batch("PRAGMA foreign_keys=OFF")
        .map_err(|e| format!("FK off failed: {e}"))?;

    // Single transaction for the entire import
    conn.execute_batch("BEGIN")
        .map_err(|e| format!("BEGIN failed: {e}"))?;

    // Delete existing data for this DARWIN (re-import)
    conn.execute(
        "DELETE FROM darwin_deals WHERE account = ?1",
        params![darwin_ticker],
    )
    .map_err(|e| format!("Delete deals failed: {e}"))?;
    conn.execute(
        "DELETE FROM darwin_positions WHERE account = ?1",
        params![darwin_ticker],
    )
    .map_err(|e| format!("Delete positions failed: {e}"))?;
    conn.execute(
        "DELETE FROM darwin_accounts WHERE darwin_ticker = ?1",
        params![darwin_ticker],
    )
    .map_err(|e| format!("Delete account failed: {e}"))?;

    // Parse Positions section (row positions_start+2 to orders_start-1)
    // Header: Time, Position, Symbol, Type, Volume, Price, S/L, T/P, Time, Price, Commission, Swap, Profit
    let mut position_count = 0;
    if positions_start > 0 && orders_start > positions_start {
        for i in (positions_start + 2)..orders_start {
            let row = &rows[i];
            if row.len() < 13 {
                continue;
            }
            let open_time = cell_str(&row[0]);
            if open_time.is_empty() || open_time == "Time" {
                continue;
            }

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
            if row.len() < 13 {
                continue;
            }
            let time = cell_str(&row[0]);
            if time.is_empty() || time == "Time" {
                continue;
            }
            // Skip summary rows at the bottom
            let deal_type = cell_str(&row[3]);
            if deal_type.is_empty() {
                continue;
            }

            let profit = cell_f64(&row[11]);
            let balance = cell_f64(&row[12]);
            let comment = if row.len() > 13 {
                cell_str(&row[13])
            } else {
                String::new()
            };

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

    conn.execute_batch("COMMIT")
        .map_err(|e| format!("COMMIT failed: {e}"))?;
    conn.execute_batch("PRAGMA foreign_keys=ON").ok();

    Ok((darwin_ticker.to_string(), deal_count, position_count))
}

/// List all imported DARWIN accounts.
pub fn list_darwin_accounts(conn: &Connection) -> Result<Vec<DarwinAccount>, String> {
    // prepare_cached: called at the top of nearly every BG phase entry point.
    let mut stmt = conn.prepare_cached(
        "SELECT darwin_ticker, name, mt5_account, initial_balance, created_at, deal_count, position_count FROM darwin_accounts ORDER BY name"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(DarwinAccount {
                darwin_ticker: row.get(0)?,
                name: row.get(1)?,
                mt5_account: row.get(2)?,
                initial_balance: row.get(3)?,
                created_at: row.get(4)?,
                deal_count: row.get(5)?,
                position_count: row.get(6)?,
            })
        })
        .map_err(|e| format!("Query failed: {e}"))?;

    let mut accounts = Vec::new();
    for row in rows {
        if let Ok(a) = row {
            accounts.push(a);
        }
    }
    Ok(accounts)
}

/// Get full account summary with computed stats.
pub fn get_darwin_summary(
    conn: &Connection,
    darwin_ticker: &str,
) -> Result<DarwinAccountSummary, String> {
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

    // Compute stats from positions (closed trades with P/L).
    // prepare_cached: called once per DARWIN account by get_portfolio_summary.
    let mut stmt = conn
        .prepare_cached(
            "SELECT profit, commission, swap, symbol FROM darwin_positions WHERE account = ?1",
        )
        .map_err(|e| format!("Prepare failed: {e}"))?;

    let mut total_profit = 0.0f64;
    let mut total_commission = 0.0f64;
    let mut total_swap = 0.0f64;
    let mut win_count = 0i64;
    let mut loss_count = 0i64;
    let mut gross_wins = 0.0f64;
    let mut gross_losses = 0.0f64;
    let mut symbols = std::collections::HashSet::new();

    let rows = stmt
        .query_map(params![darwin_ticker], |row| {
            Ok((
                row.get::<_, f64>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| format!("Query failed: {e}"))?;

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
    let win_rate = if total_trades > 0 {
        win_count as f64 / total_trades as f64 * 100.0
    } else {
        0.0
    };
    let profit_factor = if gross_losses > 0.0 {
        (gross_wins / gross_losses).min(999.0)
    } else {
        if gross_wins > 0.0 { 999.0 } else { 0.0 }
    };

    // Final balance from last deal
    let final_balance: f64 = conn.query_row(
        "SELECT balance FROM darwin_deals WHERE account = ?1 AND balance > 0 ORDER BY time DESC, id DESC LIMIT 1",
        params![darwin_ticker],
        |row| row.get(0),
    ).unwrap_or(account.initial_balance);

    // Max drawdown from deal balance series.
    // prepare_cached: called once per DARWIN account per summary refresh.
    let mut dd_stmt = conn
        .prepare_cached(
            "SELECT balance FROM darwin_deals WHERE account = ?1 AND balance > 0 ORDER BY time, id",
        )
        .map_err(|e| format!("Prepare failed: {e}"))?;

    let balances: Vec<f64> = dd_stmt
        .query_map(params![darwin_ticker], |row| row.get(0))
        .map_err(|e| format!("Query failed: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    let mut peak = 0.0f64;
    let mut max_dd_pct = 0.0f64;
    for b in &balances {
        if *b > peak {
            peak = *b;
        }
        if peak > 0.0 {
            let dd = (peak - b) / peak * 100.0;
            if dd > max_dd_pct {
                max_dd_pct = dd;
            }
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
    let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(sym) =
        symbol
    {
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

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Prepare failed: {e}"))?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();
    let rows = stmt
        .query_map(params_refs.as_slice(), |row| {
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
        })
        .map_err(|e| format!("Query failed: {e}"))?;

    let mut deals = Vec::new();
    for row in rows {
        if let Ok(d) = row {
            deals.push(d);
        }
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
    let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(sym) =
        symbol
    {
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

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Prepare failed: {e}"))?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();
    let rows = stmt
        .query_map(params_refs.as_slice(), |row| {
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
        })
        .map_err(|e| format!("Query failed: {e}"))?;

    let mut positions = Vec::new();
    for row in rows {
        if let Ok(p) = row {
            positions.push(p);
        }
    }
    Ok(positions)
}

/// Get equity curve from deals (balance over time).
pub fn get_darwin_equity_curve(
    conn: &Connection,
    darwin_ticker: &str,
) -> Result<Vec<(String, f64)>, String> {
    // prepare_cached: called once per DARWIN account by the BG thread.
    let mut stmt = conn.prepare_cached(
        "SELECT time, balance FROM darwin_deals WHERE account = ?1 AND balance > 0 ORDER BY time, id"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let rows = stmt
        .query_map(params![darwin_ticker], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        })
        .map_err(|e| format!("Query failed: {e}"))?;

    let mut curve = Vec::new();
    for row in rows {
        if let Ok(point) = row {
            curve.push(point);
        }
    }
    Ok(curve)
}

/// Get P/L by symbol for a DARWIN account.
pub fn get_darwin_pnl_by_symbol(
    conn: &Connection,
    darwin_ticker: &str,
) -> Result<Vec<(String, f64, f64, f64, i64)>, String> {
    // prepare_cached: called once per DARWIN account by the BG thread.
    let mut stmt = conn.prepare_cached(
        "SELECT symbol, SUM(profit), SUM(commission), SUM(swap), COUNT(*) FROM darwin_positions WHERE account = ?1 GROUP BY symbol ORDER BY SUM(profit) DESC"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let rows = stmt
        .query_map(params![darwin_ticker], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, f64>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .map_err(|e| format!("Query failed: {e}"))?;

    let mut result = Vec::new();
    for row in rows {
        if let Ok(r) = row {
            result.push(r);
        }
    }
    Ok(result)
}
