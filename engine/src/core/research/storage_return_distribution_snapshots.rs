use super::*;

// ── Round 22 schema v22 + wrappers ──

pub fn create_research_tables_v22(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v21(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_retskew (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_retskew_updated ON research_retskew(updated_at);

        CREATE TABLE IF NOT EXISTS research_retkurt (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_retkurt_updated ON research_retkurt(updated_at);

        CREATE TABLE IF NOT EXISTS research_tailr (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_tailr_updated ON research_tailr(updated_at);

        CREATE TABLE IF NOT EXISTS research_runlen (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_runlen_updated ON research_runlen(updated_at);

        CREATE TABLE IF NOT EXISTS research_dayrange (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_dayrange_updated ON research_dayrange(updated_at);",
    )
    .map_err(|e| format!("create v22 tables: {e}"))?;
    Ok(())
}

pub fn upsert_retskew(
    conn: &Connection,
    symbol: &str,
    snap: &ReturnSkewnessSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v22(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("retskew json: {e}"))?;
    conn.execute(
        "INSERT INTO research_retskew(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert retskew: {e}"))?;
    Ok(())
}

pub fn get_retskew(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<ReturnSkewnessSnapshot>, String> {
    let _ = create_research_tables_v22(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_retskew WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_retskew: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_retskew: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_retskew: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_retkurt(
    conn: &Connection,
    symbol: &str,
    snap: &ReturnKurtosisSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v22(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("retkurt json: {e}"))?;
    conn.execute(
        "INSERT INTO research_retkurt(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert retkurt: {e}"))?;
    Ok(())
}

pub fn get_retkurt(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<ReturnKurtosisSnapshot>, String> {
    let _ = create_research_tables_v22(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_retkurt WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_retkurt: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_retkurt: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_retkurt: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_tailr(
    conn: &Connection,
    symbol: &str,
    snap: &TailRatioSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v22(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("tailr json: {e}"))?;
    conn.execute(
        "INSERT INTO research_tailr(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert tailr: {e}"))?;
    Ok(())
}

pub fn get_tailr(conn: &Connection, symbol: &str) -> Result<Option<TailRatioSnapshot>, String> {
    let _ = create_research_tables_v22(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_tailr WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_tailr: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_tailr: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_tailr: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_runlen(
    conn: &Connection,
    symbol: &str,
    snap: &RunLengthSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v22(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("runlen json: {e}"))?;
    conn.execute(
        "INSERT INTO research_runlen(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert runlen: {e}"))?;
    Ok(())
}

pub fn get_runlen(conn: &Connection, symbol: &str) -> Result<Option<RunLengthSnapshot>, String> {
    let _ = create_research_tables_v22(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_runlen WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_runlen: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_runlen: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_runlen: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_dayrange(
    conn: &Connection,
    symbol: &str,
    snap: &DailyRangeSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v22(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("dayrange json: {e}"))?;
    conn.execute(
        "INSERT INTO research_dayrange(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert dayrange: {e}"))?;
    Ok(())
}

pub fn get_dayrange(conn: &Connection, symbol: &str) -> Result<Option<DailyRangeSnapshot>, String> {
    let _ = create_research_tables_v22(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_dayrange WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_dayrange: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_dayrange: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_dayrange: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}
