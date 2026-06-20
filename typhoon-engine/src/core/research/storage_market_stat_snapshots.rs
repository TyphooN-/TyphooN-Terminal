use super::*;

// ── : SEAG / COR / TRA / TECH / SKEW ──

pub fn create_research_tables_v9(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_seasonality (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_correlation (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_total_return (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_technicals (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_vol_skew (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_seasonality_updated  ON research_seasonality(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_correlation_updated  ON research_correlation(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_total_return_updated ON research_total_return(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_technicals_updated   ON research_technicals(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_vol_skew_updated     ON research_vol_skew(updated_at);"
    ).map_err(|e| format!("create research_v9 tables: {e}"))?;
    Ok(())
}

pub fn upsert_seasonality(
    conn: &Connection,
    symbol: &str,
    snap: &SeasonalitySnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v9(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("seasonality json: {e}"))?;
    conn.execute(
        "INSERT INTO research_seasonality(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert seasonality: {e}"))?;
    Ok(())
}

pub fn get_seasonality(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<SeasonalitySnapshot>, String> {
    let _ = create_research_tables_v9(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_seasonality WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_seasonality: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_seasonality: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_seasonality: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_correlation(
    conn: &Connection,
    symbol: &str,
    snap: &CorrelationMatrix,
) -> Result<(), String> {
    let _ = create_research_tables_v9(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("correlation json: {e}"))?;
    conn.execute(
        "INSERT INTO research_correlation(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert correlation: {e}"))?;
    Ok(())
}

pub fn get_correlation(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CorrelationMatrix>, String> {
    let _ = create_research_tables_v9(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_correlation WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_correlation: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_correlation: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_correlation: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_total_return(
    conn: &Connection,
    symbol: &str,
    snap: &TotalReturnSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v9(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("total return json: {e}"))?;
    conn.execute(
        "INSERT INTO research_total_return(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert total return: {e}"))?;
    Ok(())
}

pub fn get_total_return(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<TotalReturnSnapshot>, String> {
    let _ = create_research_tables_v9(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_total_return WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_total_return: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_total_return: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_total_return: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_technicals(
    conn: &Connection,
    symbol: &str,
    snap: &TechnicalSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v9(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("technicals json: {e}"))?;
    conn.execute(
        "INSERT INTO research_technicals(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert technicals: {e}"))?;
    Ok(())
}

pub fn get_technicals(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<TechnicalSnapshot>, String> {
    let _ = create_research_tables_v9(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_technicals WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_technicals: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_technicals: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_technicals: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_vol_skew(
    conn: &Connection,
    symbol: &str,
    snap: &VolatilitySkew,
) -> Result<(), String> {
    let _ = create_research_tables_v9(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("vol skew json: {e}"))?;
    conn.execute(
        "INSERT INTO research_vol_skew(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert vol skew: {e}"))?;
    Ok(())
}

pub fn get_vol_skew(conn: &Connection, symbol: &str) -> Result<Option<VolatilitySkew>, String> {
    let _ = create_research_tables_v9(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_vol_skew WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_vol_skew: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_vol_skew: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_vol_skew: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}
