use super::*;

// ── + SQLite storage helpers (macro snapshots) ──

pub fn create_research_tables_v6(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_world_indices (
            snapshot_key TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_market_movers (
            snapshot_key TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_sector_performance (
            snapshot_key TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_wacc (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_world_indices_updated ON research_world_indices(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_market_movers_updated ON research_market_movers(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_sector_performance_updated ON research_sector_performance(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_wacc_updated ON research_wacc(updated_at);"
    ).map_err(|e| format!("create research_v6 tables: {e}"))?;
    Ok(())
}

pub fn upsert_world_indices(conn: &Connection, rows: &[WorldIndex]) -> Result<(), String> {
    let _ = create_research_tables_v6(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("wei json: {e}"))?;
    conn.execute(
        "INSERT INTO research_world_indices(snapshot_key, rows_json, updated_at) VALUES ('latest',?1,?2)
         ON CONFLICT(snapshot_key) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![json, now_ts()],
    ).map_err(|e| format!("upsert wei: {e}"))?;
    Ok(())
}

pub fn get_world_indices(conn: &Connection) -> Result<Option<Vec<WorldIndex>>, String> {
    let _ = create_research_tables_v6(conn);
    let mut stmt = conn
        .prepare("SELECT rows_json FROM research_world_indices WHERE snapshot_key='latest'")
        .map_err(|e| format!("prepare get_wei: {e}"))?;
    let mut r = stmt.query([]).map_err(|e| format!("query get_wei: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_wei: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_market_movers(conn: &Connection, movers: &MarketMovers) -> Result<(), String> {
    let _ = create_research_tables_v6(conn);
    let json = serde_json::to_string(movers).map_err(|e| format!("mov json: {e}"))?;
    conn.execute(
        "INSERT INTO research_market_movers(snapshot_key, snapshot_json, updated_at) VALUES ('latest',?1,?2)
         ON CONFLICT(snapshot_key) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![json, now_ts()],
    ).map_err(|e| format!("upsert mov: {e}"))?;
    Ok(())
}

pub fn get_market_movers(conn: &Connection) -> Result<Option<MarketMovers>, String> {
    let _ = create_research_tables_v6(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_market_movers WHERE snapshot_key='latest'")
        .map_err(|e| format!("prepare get_mov: {e}"))?;
    let mut r = stmt.query([]).map_err(|e| format!("query get_mov: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_mov: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_sector_performance(
    conn: &Connection,
    rows: &[SectorPerformance],
) -> Result<(), String> {
    let _ = create_research_tables_v6(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("indu json: {e}"))?;
    conn.execute(
        "INSERT INTO research_sector_performance(snapshot_key, rows_json, updated_at) VALUES ('latest',?1,?2)
         ON CONFLICT(snapshot_key) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![json, now_ts()],
    ).map_err(|e| format!("upsert indu: {e}"))?;
    Ok(())
}

pub fn get_sector_performance(conn: &Connection) -> Result<Option<Vec<SectorPerformance>>, String> {
    let _ = create_research_tables_v6(conn);
    let mut stmt = conn
        .prepare("SELECT rows_json FROM research_sector_performance WHERE snapshot_key='latest'")
        .map_err(|e| format!("prepare get_indu: {e}"))?;
    let mut r = stmt.query([]).map_err(|e| format!("query get_indu: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_indu: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_wacc(conn: &Connection, symbol: &str, snap: &WaccSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v6(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("wacc json: {e}"))?;
    conn.execute(
        "INSERT INTO research_wacc(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert wacc: {e}"))?;
    Ok(())
}

pub fn get_wacc(conn: &Connection, symbol: &str) -> Result<Option<WaccSnapshot>, String> {
    let _ = create_research_tables_v6(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_wacc WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_wacc: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_wacc: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_wacc: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}
