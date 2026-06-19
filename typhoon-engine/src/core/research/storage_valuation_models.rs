use super::*;

// ── Round 8 schema: HRA / DCF / SVM / OMON / IVOL ────────────────

pub fn create_research_tables_v8(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_hra (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_dcf (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_svm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_options_chain (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_ivol (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_hra_updated            ON research_hra(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_dcf_updated            ON research_dcf(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_svm_updated            ON research_svm(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_options_chain_updated  ON research_options_chain(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_ivol_updated           ON research_ivol(updated_at);"
    ).map_err(|e| format!("create research_v8 tables: {e}"))?;
    Ok(())
}

pub fn upsert_hra(conn: &Connection, symbol: &str, snap: &HraSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v8(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("hra json: {e}"))?;
    conn.execute(
        "INSERT INTO research_hra(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert hra: {e}"))?;
    Ok(())
}

pub fn get_hra(conn: &Connection, symbol: &str) -> Result<Option<HraSnapshot>, String> {
    let _ = create_research_tables_v8(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_hra WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_hra: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_hra: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_hra: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_dcf(conn: &Connection, symbol: &str, snap: &DcfSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v8(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("dcf json: {e}"))?;
    conn.execute(
        "INSERT INTO research_dcf(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert dcf: {e}"))?;
    Ok(())
}

pub fn get_dcf(conn: &Connection, symbol: &str) -> Result<Option<DcfSnapshot>, String> {
    let _ = create_research_tables_v8(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_dcf WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_dcf: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_dcf: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_dcf: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_svm(conn: &Connection, symbol: &str, snap: &SvmSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v8(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("svm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_svm(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert svm: {e}"))?;
    Ok(())
}

pub fn get_svm(conn: &Connection, symbol: &str) -> Result<Option<SvmSnapshot>, String> {
    let _ = create_research_tables_v8(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_svm WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_svm: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_svm: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_svm: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_options_chain(
    conn: &Connection,
    symbol: &str,
    snap: &OptionsChainSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v8(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("options chain json: {e}"))?;
    conn.execute(
        "INSERT INTO research_options_chain(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert options chain: {e}"))?;
    Ok(())
}

pub fn get_options_chain(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<OptionsChainSnapshot>, String> {
    let _ = create_research_tables_v8(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_options_chain WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_options_chain: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_options_chain: {e}"))?;
    if let Some(row) = r
        .next()
        .map_err(|e| format!("row get_options_chain: {e}"))?
    {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_ivol(conn: &Connection, symbol: &str, snap: &IvolSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v8(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ivol json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ivol(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ivol: {e}"))?;
    Ok(())
}

pub fn get_ivol(conn: &Connection, symbol: &str) -> Result<Option<IvolSnapshot>, String> {
    let _ = create_research_tables_v8(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_ivol WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ivol: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_ivol: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ivol: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}
