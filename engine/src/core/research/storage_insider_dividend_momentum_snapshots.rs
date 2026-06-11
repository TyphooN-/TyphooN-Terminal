use super::*;

// ── Godel Parity Round 12 schema + helpers ─────────────────────────

pub fn create_research_tables_v12(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_insider_activity (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_divg (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_earm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_sector_rotation (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_updm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_insider_activity_updated ON research_insider_activity(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_divg_updated             ON research_divg(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_earm_updated             ON research_earm(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_sector_rotation_updated  ON research_sector_rotation(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_updm_updated             ON research_updm(updated_at);"
    ).map_err(|e| format!("create research_v12 tables: {e}"))?;
    Ok(())
}

pub fn upsert_insider_activity(
    conn: &Connection,
    symbol: &str,
    snap: &InsiderActivitySnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v12(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("insider_activity json: {e}"))?;
    conn.execute(
        "INSERT INTO research_insider_activity(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert insider_activity: {e}"))?;
    Ok(())
}

pub fn get_insider_activity(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<InsiderActivitySnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_insider_activity WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_insider_activity: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_insider_activity: {e}"))?;
    if let Some(row) = r
        .next()
        .map_err(|e| format!("row get_insider_activity: {e}"))?
    {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_divg(conn: &Connection, symbol: &str, snap: &DivgSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v12(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("divg json: {e}"))?;
    conn.execute(
        "INSERT INTO research_divg(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert divg: {e}"))?;
    Ok(())
}

pub fn get_divg(conn: &Connection, symbol: &str) -> Result<Option<DivgSnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_divg WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_divg: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_divg: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_divg: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_earm(conn: &Connection, symbol: &str, snap: &EarmSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v12(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("earm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_earm(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert earm: {e}"))?;
    Ok(())
}

pub fn get_earm(conn: &Connection, symbol: &str) -> Result<Option<EarmSnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_earm WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_earm: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_earm: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_earm: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_sector_rotation(
    conn: &Connection,
    symbol: &str,
    snap: &SectorRotationSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v12(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("sector_rotation json: {e}"))?;
    conn.execute(
        "INSERT INTO research_sector_rotation(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert sector_rotation: {e}"))?;
    Ok(())
}

pub fn get_sector_rotation(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<SectorRotationSnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_sector_rotation WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_sector_rotation: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_sector_rotation: {e}"))?;
    if let Some(row) = r
        .next()
        .map_err(|e| format!("row get_sector_rotation: {e}"))?
    {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_updm(conn: &Connection, symbol: &str, snap: &UpdmSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v12(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("updm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_updm(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert updm: {e}"))?;
    Ok(())
}

pub fn get_updm(conn: &Connection, symbol: &str) -> Result<Option<UpdmSnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_updm WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_updm: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_updm: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_updm: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}
