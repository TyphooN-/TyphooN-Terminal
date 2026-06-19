use super::*;

// Fundamental quality, solvency, volatility, EPS-beat, and price-target storage

pub fn create_research_tables_v11(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_altman_z (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_piotroski (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_ohlc_vol (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_eps_beat (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_price_target_dispersion (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_altman_z_updated                 ON research_altman_z(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_piotroski_updated                ON research_piotroski(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_ohlc_vol_updated                 ON research_ohlc_vol(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_eps_beat_updated                 ON research_eps_beat(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_price_target_dispersion_updated  ON research_price_target_dispersion(updated_at);"
    ).map_err(|e| format!("create research_v11 tables: {e}"))?;
    Ok(())
}

pub fn upsert_altman_z(
    conn: &Connection,
    symbol: &str,
    snap: &AltmanZSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v11(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("altman_z json: {e}"))?;
    conn.execute(
        "INSERT INTO research_altman_z(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert altman_z: {e}"))?;
    Ok(())
}

pub fn get_altman_z(conn: &Connection, symbol: &str) -> Result<Option<AltmanZSnapshot>, String> {
    let _ = create_research_tables_v11(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_altman_z WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_altman_z: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_altman_z: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_altman_z: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_piotroski(
    conn: &Connection,
    symbol: &str,
    snap: &PiotroskiSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v11(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("piotroski json: {e}"))?;
    conn.execute(
        "INSERT INTO research_piotroski(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert piotroski: {e}"))?;
    Ok(())
}

pub fn get_piotroski(conn: &Connection, symbol: &str) -> Result<Option<PiotroskiSnapshot>, String> {
    let _ = create_research_tables_v11(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_piotroski WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_piotroski: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_piotroski: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_piotroski: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_ohlc_vol(
    conn: &Connection,
    symbol: &str,
    snap: &OhlcVolSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v11(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ohlc_vol json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ohlc_vol(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ohlc_vol: {e}"))?;
    Ok(())
}

pub fn get_ohlc_vol(conn: &Connection, symbol: &str) -> Result<Option<OhlcVolSnapshot>, String> {
    let _ = create_research_tables_v11(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_ohlc_vol WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ohlc_vol: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_ohlc_vol: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ohlc_vol: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_eps_beat(
    conn: &Connection,
    symbol: &str,
    snap: &EpsBeatSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v11(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("eps_beat json: {e}"))?;
    conn.execute(
        "INSERT INTO research_eps_beat(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert eps_beat: {e}"))?;
    Ok(())
}

pub fn get_eps_beat(conn: &Connection, symbol: &str) -> Result<Option<EpsBeatSnapshot>, String> {
    let _ = create_research_tables_v11(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_eps_beat WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_eps_beat: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_eps_beat: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_eps_beat: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_price_target_dispersion(
    conn: &Connection,
    symbol: &str,
    snap: &PriceTargetDispersion,
) -> Result<(), String> {
    let _ = create_research_tables_v11(conn);
    let json =
        serde_json::to_string(snap).map_err(|e| format!("price_target_dispersion json: {e}"))?;
    conn.execute(
        "INSERT INTO research_price_target_dispersion(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert price_target_dispersion: {e}"))?;
    Ok(())
}

pub fn get_price_target_dispersion(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<PriceTargetDispersion>, String> {
    let _ = create_research_tables_v11(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_price_target_dispersion WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_price_target_dispersion: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_price_target_dispersion: {e}"))?;
    if let Some(row) = r
        .next()
        .map_err(|e| format!("row get_price_target_dispersion: {e}"))?
    {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}
