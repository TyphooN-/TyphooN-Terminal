use super::*;

// ── : LEV / ACRL / RVOL / FCFY / SHRT ──

pub fn create_research_tables_v10(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_leverage (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_accruals (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_realized_vol (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_fcf_yield (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_short_interest (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_leverage_updated        ON research_leverage(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_accruals_updated        ON research_accruals(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_realized_vol_updated    ON research_realized_vol(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_fcf_yield_updated       ON research_fcf_yield(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_short_interest_updated  ON research_short_interest(updated_at);"
    ).map_err(|e| format!("create research_v10 tables: {e}"))?;
    Ok(())
}

pub fn upsert_leverage(
    conn: &Connection,
    symbol: &str,
    snap: &LeverageSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v10(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("leverage json: {e}"))?;
    conn.execute(
        "INSERT INTO research_leverage(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert leverage: {e}"))?;
    Ok(())
}

pub fn get_leverage(conn: &Connection, symbol: &str) -> Result<Option<LeverageSnapshot>, String> {
    let _ = create_research_tables_v10(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_leverage WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_leverage: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_leverage: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_leverage: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_accruals(
    conn: &Connection,
    symbol: &str,
    snap: &AccrualsSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v10(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("accruals json: {e}"))?;
    conn.execute(
        "INSERT INTO research_accruals(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert accruals: {e}"))?;
    Ok(())
}

pub fn get_accruals(conn: &Connection, symbol: &str) -> Result<Option<AccrualsSnapshot>, String> {
    let _ = create_research_tables_v10(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_accruals WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_accruals: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_accruals: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_accruals: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_realized_vol(
    conn: &Connection,
    symbol: &str,
    snap: &RealizedVolSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v10(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("realized vol json: {e}"))?;
    conn.execute(
        "INSERT INTO research_realized_vol(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert realized vol: {e}"))?;
    Ok(())
}

pub fn get_realized_vol(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<RealizedVolSnapshot>, String> {
    let _ = create_research_tables_v10(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_realized_vol WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_realized_vol: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_realized_vol: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_realized_vol: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_fcf_yield(
    conn: &Connection,
    symbol: &str,
    snap: &FcfYieldSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v10(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("fcf yield json: {e}"))?;
    conn.execute(
        "INSERT INTO research_fcf_yield(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert fcf yield: {e}"))?;
    Ok(())
}

pub fn get_fcf_yield(conn: &Connection, symbol: &str) -> Result<Option<FcfYieldSnapshot>, String> {
    let _ = create_research_tables_v10(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_fcf_yield WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_fcf_yield: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_fcf_yield: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_fcf_yield: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_short_interest(
    conn: &Connection,
    symbol: &str,
    snap: &ShortInterestSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v10(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("short interest json: {e}"))?;
    conn.execute(
        "INSERT INTO research_short_interest(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert short interest: {e}"))?;
    Ok(())
}

pub fn get_short_interest(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<ShortInterestSnapshot>, String> {
    let _ = create_research_tables_v10(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_short_interest WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_short_interest: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_short_interest: {e}"))?;
    if let Some(row) = r
        .next()
        .map_err(|e| format!("row get_short_interest: {e}"))?
    {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

/// Whole-table scan of symbols with a `research_short_interest` row. Used by
/// the SQUEEZE watchlist refresh to drive the recompute set.
pub fn get_all_short_interest_symbols(conn: &Connection) -> Result<Vec<String>, String> {
    let _ = create_research_tables_v10(conn);
    let mut stmt = conn
        .prepare("SELECT symbol FROM research_short_interest")
        .map_err(|e| format!("prepare get_all_short_interest_symbols: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_short_interest_symbols: {e}"))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}
