use super::*;

pub fn create_research_tables_v50(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v49(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_cmo (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cmo_updated ON research_cmo(updated_at);

        CREATE TABLE IF NOT EXISTS research_qstick (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_qstick_updated ON research_qstick(updated_at);

        CREATE TABLE IF NOT EXISTS research_disparity (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_disparity_updated ON research_disparity(updated_at);

        CREATE TABLE IF NOT EXISTS research_bop (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_bop_updated ON research_bop(updated_at);

        CREATE TABLE IF NOT EXISTS research_schaff (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_schaff_updated ON research_schaff(updated_at);",
    )
    .map_err(|e| format!("create v50 tables: {e}"))?;
    Ok(())
}

pub fn upsert_cmo(conn: &Connection, symbol: &str, snap: &CmoSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v50(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cmo json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cmo(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cmo: {e}"))?;
    Ok(())
}

pub fn get_cmo(conn: &Connection, symbol: &str) -> Result<Option<CmoSnapshot>, String> {
    let _ = create_research_tables_v50(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cmo WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_cmo: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_cmo: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_cmo: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_qstick(conn: &Connection, symbol: &str, snap: &QstickSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v50(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("qstick json: {e}"))?;
    conn.execute(
        "INSERT INTO research_qstick(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert qstick: {e}"))?;
    Ok(())
}

pub fn get_qstick(conn: &Connection, symbol: &str) -> Result<Option<QstickSnapshot>, String> {
    let _ = create_research_tables_v50(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_qstick WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_qstick: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_qstick: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_qstick: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_disparity(
    conn: &Connection,
    symbol: &str,
    snap: &DisparitySnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v50(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("disparity json: {e}"))?;
    conn.execute(
        "INSERT INTO research_disparity(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert disparity: {e}"))?;
    Ok(())
}

pub fn get_disparity(conn: &Connection, symbol: &str) -> Result<Option<DisparitySnapshot>, String> {
    let _ = create_research_tables_v50(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_disparity WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_disparity: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_disparity: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_disparity: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_bop(conn: &Connection, symbol: &str, snap: &BopSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v50(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("bop json: {e}"))?;
    conn.execute(
        "INSERT INTO research_bop(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert bop: {e}"))?;
    Ok(())
}

pub fn get_bop(conn: &Connection, symbol: &str) -> Result<Option<BopSnapshot>, String> {
    let _ = create_research_tables_v50(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_bop WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_bop: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_bop: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_bop: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_schaff(conn: &Connection, symbol: &str, snap: &SchaffSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v50(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("schaff json: {e}"))?;
    conn.execute(
        "INSERT INTO research_schaff(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert schaff: {e}"))?;
    Ok(())
}

pub fn get_schaff(conn: &Connection, symbol: &str) -> Result<Option<SchaffSnapshot>, String> {
    let _ = create_research_tables_v50(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_schaff WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_schaff: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_schaff: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_schaff: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn create_research_tables_v51(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v50(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_stoch (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_stoch_updated ON research_stoch(updated_at);

        CREATE TABLE IF NOT EXISTS research_macd (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_macd_updated ON research_macd(updated_at);

        CREATE TABLE IF NOT EXISTS research_vwap (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_vwap_updated ON research_vwap(updated_at);

        CREATE TABLE IF NOT EXISTS research_mcgd (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_mcgd_updated ON research_mcgd(updated_at);

        CREATE TABLE IF NOT EXISTS research_rwi (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_rwi_updated ON research_rwi(updated_at);",
    )
    .map_err(|e| format!("create v51 tables: {e}"))?;
    Ok(())
}

pub fn upsert_stoch(conn: &Connection, symbol: &str, snap: &StochSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v51(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("stoch json: {e}"))?;
    conn.execute(
        "INSERT INTO research_stoch(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert stoch: {e}"))?;
    Ok(())
}

pub fn get_stoch(conn: &Connection, symbol: &str) -> Result<Option<StochSnapshot>, String> {
    let _ = create_research_tables_v51(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_stoch WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_stoch: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_stoch: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_stoch: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_macd(conn: &Connection, symbol: &str, snap: &MacdSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v51(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("macd json: {e}"))?;
    conn.execute(
        "INSERT INTO research_macd(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert macd: {e}"))?;
    Ok(())
}

pub fn get_macd(conn: &Connection, symbol: &str) -> Result<Option<MacdSnapshot>, String> {
    let _ = create_research_tables_v51(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_macd WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_macd: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_macd: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_macd: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_vwap(conn: &Connection, symbol: &str, snap: &VwapSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v51(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("vwap json: {e}"))?;
    conn.execute(
        "INSERT INTO research_vwap(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert vwap: {e}"))?;
    Ok(())
}

pub fn get_vwap(conn: &Connection, symbol: &str) -> Result<Option<VwapSnapshot>, String> {
    let _ = create_research_tables_v51(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_vwap WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_vwap: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_vwap: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_vwap: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_mcgd(conn: &Connection, symbol: &str, snap: &McgdSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v51(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("mcgd json: {e}"))?;
    conn.execute(
        "INSERT INTO research_mcgd(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert mcgd: {e}"))?;
    Ok(())
}

pub fn get_mcgd(conn: &Connection, symbol: &str) -> Result<Option<McgdSnapshot>, String> {
    let _ = create_research_tables_v51(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_mcgd WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_mcgd: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_mcgd: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_mcgd: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_rwi(conn: &Connection, symbol: &str, snap: &RwiSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v51(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("rwi json: {e}"))?;
    conn.execute(
        "INSERT INTO research_rwi(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rwi: {e}"))?;
    Ok(())
}

pub fn get_rwi(conn: &Connection, symbol: &str) -> Result<Option<RwiSnapshot>, String> {
    let _ = create_research_tables_v51(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_rwi WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_rwi: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_rwi: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_rwi: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn create_research_tables_v52(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v51(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_dema (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_dema_updated ON research_dema(updated_at);

        CREATE TABLE IF NOT EXISTS research_tema (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_tema_updated ON research_tema(updated_at);

        CREATE TABLE IF NOT EXISTS research_linreg (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_linreg_updated ON research_linreg(updated_at);

        CREATE TABLE IF NOT EXISTS research_pivots (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_pivots_updated ON research_pivots(updated_at);

        CREATE TABLE IF NOT EXISTS research_heikin (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_heikin_updated ON research_heikin(updated_at);",
    )
    .map_err(|e| format!("create v52 tables: {e}"))?;
    Ok(())
}

pub fn upsert_dema(conn: &Connection, symbol: &str, snap: &DemaSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v52(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("dema json: {e}"))?;
    conn.execute(
        "INSERT INTO research_dema(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert dema: {e}"))?;
    Ok(())
}

pub fn get_dema(conn: &Connection, symbol: &str) -> Result<Option<DemaSnapshot>, String> {
    let _ = create_research_tables_v52(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_dema WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_dema: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_dema: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_dema: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_tema(conn: &Connection, symbol: &str, snap: &TemaSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v52(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("tema json: {e}"))?;
    conn.execute(
        "INSERT INTO research_tema(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert tema: {e}"))?;
    Ok(())
}

pub fn get_tema(conn: &Connection, symbol: &str) -> Result<Option<TemaSnapshot>, String> {
    let _ = create_research_tables_v52(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_tema WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_tema: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_tema: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_tema: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_linreg(conn: &Connection, symbol: &str, snap: &LinregSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v52(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("linreg json: {e}"))?;
    conn.execute(
        "INSERT INTO research_linreg(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert linreg: {e}"))?;
    Ok(())
}

pub fn get_linreg(conn: &Connection, symbol: &str) -> Result<Option<LinregSnapshot>, String> {
    let _ = create_research_tables_v52(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_linreg WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_linreg: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_linreg: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_linreg: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_pivots(conn: &Connection, symbol: &str, snap: &PivotsSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v52(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("pivots json: {e}"))?;
    conn.execute(
        "INSERT INTO research_pivots(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert pivots: {e}"))?;
    Ok(())
}

pub fn get_pivots(conn: &Connection, symbol: &str) -> Result<Option<PivotsSnapshot>, String> {
    let _ = create_research_tables_v52(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_pivots WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_pivots: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_pivots: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_pivots: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_heikin(conn: &Connection, symbol: &str, snap: &HeikinSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v52(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("heikin json: {e}"))?;
    conn.execute(
        "INSERT INTO research_heikin(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert heikin: {e}"))?;
    Ok(())
}

pub fn get_heikin(conn: &Connection, symbol: &str) -> Result<Option<HeikinSnapshot>, String> {
    let _ = create_research_tables_v52(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_heikin WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_heikin: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_heikin: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_heikin: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn create_research_tables_v53(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v52(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_alma (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_alma_updated ON research_alma(updated_at);

        CREATE TABLE IF NOT EXISTS research_zlema (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_zlema_updated ON research_zlema(updated_at);

        CREATE TABLE IF NOT EXISTS research_elderray (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_elderray_updated ON research_elderray(updated_at);

        CREATE TABLE IF NOT EXISTS research_tsf (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_tsf_updated ON research_tsf(updated_at);

        CREATE TABLE IF NOT EXISTS research_rvi (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_rvi_updated ON research_rvi(updated_at);",
    )
    .map_err(|e| format!("create v53 tables: {e}"))?;
    Ok(())
}

pub fn upsert_alma(conn: &Connection, symbol: &str, snap: &AlmaSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v53(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("alma json: {e}"))?;
    conn.execute(
        "INSERT INTO research_alma(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert alma: {e}"))?;
    Ok(())
}

pub fn get_alma(conn: &Connection, symbol: &str) -> Result<Option<AlmaSnapshot>, String> {
    let _ = create_research_tables_v53(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_alma WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_alma: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_alma: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_alma: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_zlema(conn: &Connection, symbol: &str, snap: &ZlemaSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v53(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("zlema json: {e}"))?;
    conn.execute(
        "INSERT INTO research_zlema(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert zlema: {e}"))?;
    Ok(())
}

pub fn get_zlema(conn: &Connection, symbol: &str) -> Result<Option<ZlemaSnapshot>, String> {
    let _ = create_research_tables_v53(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_zlema WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_zlema: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_zlema: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_zlema: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_elderray(
    conn: &Connection,
    symbol: &str,
    snap: &ElderRaySnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v53(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("elderray json: {e}"))?;
    conn.execute(
        "INSERT INTO research_elderray(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert elderray: {e}"))?;
    Ok(())
}

pub fn get_elderray(conn: &Connection, symbol: &str) -> Result<Option<ElderRaySnapshot>, String> {
    let _ = create_research_tables_v53(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_elderray WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_elderray: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_elderray: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_elderray: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_tsf(conn: &Connection, symbol: &str, snap: &TsfSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v53(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("tsf json: {e}"))?;
    conn.execute(
        "INSERT INTO research_tsf(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert tsf: {e}"))?;
    Ok(())
}

pub fn get_tsf(conn: &Connection, symbol: &str) -> Result<Option<TsfSnapshot>, String> {
    let _ = create_research_tables_v53(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_tsf WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_tsf: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_tsf: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_tsf: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_rvi(conn: &Connection, symbol: &str, snap: &RviSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v53(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("rvi json: {e}"))?;
    conn.execute(
        "INSERT INTO research_rvi(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rvi: {e}"))?;
    Ok(())
}

pub fn get_rvi(conn: &Connection, symbol: &str) -> Result<Option<RviSnapshot>, String> {
    let _ = create_research_tables_v53(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_rvi WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_rvi: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_rvi: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_rvi: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn create_research_tables_v54(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v53(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_trima (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_trima_updated ON research_trima(updated_at);

        CREATE TABLE IF NOT EXISTS research_t3 (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_t3_updated ON research_t3(updated_at);

        CREATE TABLE IF NOT EXISTS research_vidya (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_vidya_updated ON research_vidya(updated_at);

        CREATE TABLE IF NOT EXISTS research_smi (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_smi_updated ON research_smi(updated_at);

        CREATE TABLE IF NOT EXISTS research_pvt (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_pvt_updated ON research_pvt(updated_at);",
    )
    .map_err(|e| format!("create v54 tables: {e}"))?;
    Ok(())
}

pub fn upsert_trima(conn: &Connection, symbol: &str, snap: &TrimaSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v54(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("trima json: {e}"))?;
    conn.execute(
        "INSERT INTO research_trima(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert trima: {e}"))?;
    Ok(())
}

pub fn get_trima(conn: &Connection, symbol: &str) -> Result<Option<TrimaSnapshot>, String> {
    let _ = create_research_tables_v54(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_trima WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_trima: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_trima: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_trima: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_t3(conn: &Connection, symbol: &str, snap: &T3Snapshot) -> Result<(), String> {
    let _ = create_research_tables_v54(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("t3 json: {e}"))?;
    conn.execute(
        "INSERT INTO research_t3(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert t3: {e}"))?;
    Ok(())
}

pub fn get_t3(conn: &Connection, symbol: &str) -> Result<Option<T3Snapshot>, String> {
    let _ = create_research_tables_v54(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_t3 WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_t3: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_t3: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_t3: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_vidya(conn: &Connection, symbol: &str, snap: &VidyaSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v54(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("vidya json: {e}"))?;
    conn.execute(
        "INSERT INTO research_vidya(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert vidya: {e}"))?;
    Ok(())
}

pub fn get_vidya(conn: &Connection, symbol: &str) -> Result<Option<VidyaSnapshot>, String> {
    let _ = create_research_tables_v54(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_vidya WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_vidya: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_vidya: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_vidya: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_smi(conn: &Connection, symbol: &str, snap: &SmiSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v54(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("smi json: {e}"))?;
    conn.execute(
        "INSERT INTO research_smi(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert smi: {e}"))?;
    Ok(())
}

pub fn get_smi(conn: &Connection, symbol: &str) -> Result<Option<SmiSnapshot>, String> {
    let _ = create_research_tables_v54(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_smi WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_smi: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_smi: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_smi: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_pvt(conn: &Connection, symbol: &str, snap: &PvtSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v54(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("pvt json: {e}"))?;
    conn.execute(
        "INSERT INTO research_pvt(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert pvt: {e}"))?;
    Ok(())
}

pub fn get_pvt(conn: &Connection, symbol: &str) -> Result<Option<PvtSnapshot>, String> {
    let _ = create_research_tables_v54(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_pvt WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_pvt: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_pvt: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_pvt: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

/// Round 54 schema: AC / CHVOL / BBWIDTH / ELDERIMP / RMI
pub fn create_research_tables_v55(conn: &Connection) -> Result<(), String> {
    create_research_tables_v54(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_ac (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_ac_updated ON research_ac(updated_at);

        CREATE TABLE IF NOT EXISTS research_chvol (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_chvol_updated ON research_chvol(updated_at);

        CREATE TABLE IF NOT EXISTS research_bbwidth (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_bbwidth_updated ON research_bbwidth(updated_at);

        CREATE TABLE IF NOT EXISTS research_elderimp (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_elderimp_updated ON research_elderimp(updated_at);

        CREATE TABLE IF NOT EXISTS research_rmi (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_rmi_updated ON research_rmi(updated_at);",
    )
    .map_err(|e| format!("create v55 tables: {e}"))?;
    Ok(())
}

pub fn upsert_ac(conn: &Connection, symbol: &str, snap: &AcSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v55(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ac json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ac(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ac: {e}"))?;
    Ok(())
}

pub fn get_ac(conn: &Connection, symbol: &str) -> Result<Option<AcSnapshot>, String> {
    let _ = create_research_tables_v55(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_ac WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ac: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_ac: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ac: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_chvol(conn: &Connection, symbol: &str, snap: &ChvolSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v55(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("chvol json: {e}"))?;
    conn.execute(
        "INSERT INTO research_chvol(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert chvol: {e}"))?;
    Ok(())
}

pub fn get_chvol(conn: &Connection, symbol: &str) -> Result<Option<ChvolSnapshot>, String> {
    let _ = create_research_tables_v55(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_chvol WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_chvol: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_chvol: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_chvol: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_bbwidth(
    conn: &Connection,
    symbol: &str,
    snap: &BbwidthSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v55(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("bbwidth json: {e}"))?;
    conn.execute(
        "INSERT INTO research_bbwidth(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert bbwidth: {e}"))?;
    Ok(())
}

pub fn get_bbwidth(conn: &Connection, symbol: &str) -> Result<Option<BbwidthSnapshot>, String> {
    let _ = create_research_tables_v55(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_bbwidth WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_bbwidth: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_bbwidth: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_bbwidth: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_elderimp(
    conn: &Connection,
    symbol: &str,
    snap: &ElderImpulseSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v55(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("elderimp json: {e}"))?;
    conn.execute(
        "INSERT INTO research_elderimp(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert elderimp: {e}"))?;
    Ok(())
}

pub fn get_elderimp(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<ElderImpulseSnapshot>, String> {
    let _ = create_research_tables_v55(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_elderimp WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_elderimp: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_elderimp: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_elderimp: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_rmi(conn: &Connection, symbol: &str, snap: &RmiSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v55(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("rmi json: {e}"))?;
    conn.execute(
        "INSERT INTO research_rmi(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rmi: {e}"))?;
    Ok(())
}

pub fn get_rmi(conn: &Connection, symbol: &str) -> Result<Option<RmiSnapshot>, String> {
    let _ = create_research_tables_v55(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_rmi WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_rmi: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_rmi: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_rmi: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}
