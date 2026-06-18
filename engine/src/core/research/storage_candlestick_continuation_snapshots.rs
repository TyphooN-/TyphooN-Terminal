use super::*;

// Candlestick pattern storage/helpers
//    CDLBELTHOLD / CDLCLOSINGMARUBOZU / CDLHIGHWAVE / CDLLONGLINE /
//    CDLSHORTLINE

pub fn create_research_tables_v81(conn: &Connection) -> Result<(), String> {
    create_research_tables_v80(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_cdl_belt_hold (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_belt_hold_updated ON research_cdl_belt_hold(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_closing_marubozu (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_closing_marubozu_updated ON research_cdl_closing_marubozu(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_high_wave (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_high_wave_updated ON research_cdl_high_wave(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_long_line (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_long_line_updated ON research_cdl_long_line(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_short_line (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_short_line_updated ON research_cdl_short_line(updated_at);",
    ).map_err(|e| format!("create v81 tables: {e}"))?;
    Ok(())
}

pub fn upsert_cdl_belt_hold(
    conn: &Connection,
    symbol: &str,
    snap: &CdlBeltHoldSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v81(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_belt_hold json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_belt_hold (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_belt_hold: {e}"))?;
    Ok(())
}

pub fn get_cdl_belt_hold(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlBeltHoldSnapshot>, String> {
    let _ = create_research_tables_v81(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_belt_hold WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_belt_hold: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_belt_hold: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_belt_hold: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_belt_hold: {e}"))?;
        let snap: CdlBeltHoldSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_belt_hold: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_closing_marubozu(
    conn: &Connection,
    symbol: &str,
    snap: &CdlClosingMarubozuSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v81(conn);
    let json =
        serde_json::to_string(snap).map_err(|e| format!("cdl_closing_marubozu json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_closing_marubozu (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_closing_marubozu: {e}"))?;
    Ok(())
}

pub fn get_cdl_closing_marubozu(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlClosingMarubozuSnapshot>, String> {
    let _ = create_research_tables_v81(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_closing_marubozu WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_closing_marubozu: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_closing_marubozu: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_closing_marubozu: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_closing_marubozu: {e}"))?;
        let snap: CdlClosingMarubozuSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_closing_marubozu: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_high_wave(
    conn: &Connection,
    symbol: &str,
    snap: &CdlHighWaveSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v81(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_high_wave json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_high_wave (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_high_wave: {e}"))?;
    Ok(())
}

pub fn get_cdl_high_wave(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlHighWaveSnapshot>, String> {
    let _ = create_research_tables_v81(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_high_wave WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_high_wave: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_high_wave: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_high_wave: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_high_wave: {e}"))?;
        let snap: CdlHighWaveSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_high_wave: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_long_line(
    conn: &Connection,
    symbol: &str,
    snap: &CdlLongLineSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v81(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_long_line json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_long_line (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_long_line: {e}"))?;
    Ok(())
}

pub fn get_cdl_long_line(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlLongLineSnapshot>, String> {
    let _ = create_research_tables_v81(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_long_line WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_long_line: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_long_line: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_long_line: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_long_line: {e}"))?;
        let snap: CdlLongLineSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_long_line: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_short_line(
    conn: &Connection,
    symbol: &str,
    snap: &CdlShortLineSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v81(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_short_line json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_short_line (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_short_line: {e}"))?;
    Ok(())
}

pub fn get_cdl_short_line(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlShortLineSnapshot>, String> {
    let _ = create_research_tables_v81(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_short_line WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_short_line: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_short_line: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_short_line: {e}"))?
    {
        let j: String = r.get(0).map_err(|e| format!("get cdl_short_line: {e}"))?;
        let snap: CdlShortLineSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_short_line: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

// Candlestick pattern storage/helpers
//    CDLCOUNTERATTACK / CDLHOMINGPIGEON / CDLINNECK / CDLONNECK /
//    CDLTHRUSTING

pub fn create_research_tables_v82(conn: &Connection) -> Result<(), String> {
    create_research_tables_v81(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_cdl_counterattack (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_counterattack_updated ON research_cdl_counterattack(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_homing_pigeon (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_homing_pigeon_updated ON research_cdl_homing_pigeon(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_in_neck (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_in_neck_updated ON research_cdl_in_neck(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_on_neck (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_on_neck_updated ON research_cdl_on_neck(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_thrusting (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_thrusting_updated ON research_cdl_thrusting(updated_at);",
    ).map_err(|e| format!("create v82 tables: {e}"))?;
    Ok(())
}

pub fn upsert_cdl_counterattack(
    conn: &Connection,
    symbol: &str,
    snap: &CdlCounterattackSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v82(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_counterattack json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_counterattack (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_counterattack: {e}"))?;
    Ok(())
}

pub fn get_cdl_counterattack(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlCounterattackSnapshot>, String> {
    let _ = create_research_tables_v82(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_counterattack WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_counterattack: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_counterattack: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_counterattack: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_counterattack: {e}"))?;
        let snap: CdlCounterattackSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_counterattack: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_homing_pigeon(
    conn: &Connection,
    symbol: &str,
    snap: &CdlHomingPigeonSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v82(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_homing_pigeon json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_homing_pigeon (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_homing_pigeon: {e}"))?;
    Ok(())
}

pub fn get_cdl_homing_pigeon(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlHomingPigeonSnapshot>, String> {
    let _ = create_research_tables_v82(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_homing_pigeon WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_homing_pigeon: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_homing_pigeon: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_homing_pigeon: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_homing_pigeon: {e}"))?;
        let snap: CdlHomingPigeonSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_homing_pigeon: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_in_neck(
    conn: &Connection,
    symbol: &str,
    snap: &CdlInNeckSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v82(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_in_neck json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_in_neck (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_in_neck: {e}"))?;
    Ok(())
}

pub fn get_cdl_in_neck(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlInNeckSnapshot>, String> {
    let _ = create_research_tables_v82(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_in_neck WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_in_neck: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_in_neck: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_in_neck: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_in_neck: {e}"))?;
        let snap: CdlInNeckSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_in_neck: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_on_neck(
    conn: &Connection,
    symbol: &str,
    snap: &CdlOnNeckSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v82(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_on_neck json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_on_neck (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_on_neck: {e}"))?;
    Ok(())
}

pub fn get_cdl_on_neck(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlOnNeckSnapshot>, String> {
    let _ = create_research_tables_v82(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_on_neck WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_on_neck: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_on_neck: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_on_neck: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_on_neck: {e}"))?;
        let snap: CdlOnNeckSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_on_neck: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_thrusting(
    conn: &Connection,
    symbol: &str,
    snap: &CdlThrustingSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v82(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_thrusting json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_thrusting (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_thrusting: {e}"))?;
    Ok(())
}

pub fn get_cdl_thrusting(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlThrustingSnapshot>, String> {
    let _ = create_research_tables_v82(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_thrusting WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_thrusting: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_thrusting: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_thrusting: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_thrusting: {e}"))?;
        let snap: CdlThrustingSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_thrusting: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}
