use super::*;

// ── Research section ──

pub fn create_research_tables_v7(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_currency_rates (
            snapshot_key TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_beta (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_ddm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_relative_valuation (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_figi (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_currency_rates_updated ON research_currency_rates(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_beta_updated ON research_beta(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_ddm_updated ON research_ddm(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_relative_valuation_updated ON research_relative_valuation(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_figi_updated ON research_figi(updated_at);"
    ).map_err(|e| format!("create research_v7 tables: {e}"))?;
    Ok(())
}

pub fn upsert_currency_rates(conn: &Connection, rows: &[CurrencyRate]) -> Result<(), String> {
    let _ = create_research_tables_v7(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("wcr json: {e}"))?;
    conn.execute(
        "INSERT INTO research_currency_rates(snapshot_key, rows_json, updated_at) VALUES ('latest',?1,?2)
         ON CONFLICT(snapshot_key) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![json, now_ts()],
    ).map_err(|e| format!("upsert wcr: {e}"))?;
    Ok(())
}

pub fn get_currency_rates(conn: &Connection) -> Result<Option<Vec<CurrencyRate>>, String> {
    let _ = create_research_tables_v7(conn);
    let mut stmt = conn
        .prepare("SELECT rows_json FROM research_currency_rates WHERE snapshot_key='latest'")
        .map_err(|e| format!("prepare get_wcr: {e}"))?;
    let mut r = stmt.query([]).map_err(|e| format!("query get_wcr: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_wcr: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_beta(conn: &Connection, symbol: &str, snap: &BetaSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v7(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("beta json: {e}"))?;
    conn.execute(
        "INSERT INTO research_beta(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert beta: {e}"))?;
    Ok(())
}

pub fn get_beta(conn: &Connection, symbol: &str) -> Result<Option<BetaSnapshot>, String> {
    let _ = create_research_tables_v7(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_beta WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_beta: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_beta: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_beta: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_ddm(conn: &Connection, symbol: &str, snap: &DdmSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v7(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ddm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ddm(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ddm: {e}"))?;
    Ok(())
}

pub fn get_ddm(conn: &Connection, symbol: &str) -> Result<Option<DdmSnapshot>, String> {
    let _ = create_research_tables_v7(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_ddm WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ddm: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_ddm: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ddm: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_relative_valuation(
    conn: &Connection,
    symbol: &str,
    snap: &RelativeValuation,
) -> Result<(), String> {
    let _ = create_research_tables_v7(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("rv json: {e}"))?;
    conn.execute(
        "INSERT INTO research_relative_valuation(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rv: {e}"))?;
    Ok(())
}

pub fn get_relative_valuation(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<RelativeValuation>, String> {
    let _ = create_research_tables_v7(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_relative_valuation WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_rv: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_rv: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_rv: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_figi(conn: &Connection, symbol: &str, snap: &FigiSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v7(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("figi json: {e}"))?;
    conn.execute(
        "INSERT INTO research_figi(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert figi: {e}"))?;
    Ok(())
}

pub fn get_figi(conn: &Connection, symbol: &str) -> Result<Option<FigiSnapshot>, String> {
    let _ = create_research_tables_v7(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_figi WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_figi: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_figi: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_figi: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}
