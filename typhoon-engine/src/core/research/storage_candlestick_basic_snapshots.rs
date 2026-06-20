use super::*;

// ── CDL* SQLite schema + helpers ──

pub fn create_research_tables_v74(conn: &Connection) -> Result<(), String> {
    create_research_tables_v73(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_cdl_doji (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_doji_updated ON research_cdl_doji(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_hammer (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_hammer_updated ON research_cdl_hammer(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_shooting_star (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_shooting_star_updated ON research_cdl_shooting_star(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_engulfing (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_engulfing_updated ON research_cdl_engulfing(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_harami (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_harami_updated ON research_cdl_harami(updated_at);",
    ).map_err(|e| format!("create v74 tables: {e}"))?;
    Ok(())
}

pub fn upsert_cdl_doji(
    conn: &Connection,
    symbol: &str,
    snap: &CdlDojiSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v74(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_doji json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_doji (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_doji: {e}"))?;
    Ok(())
}

pub fn get_cdl_doji(conn: &Connection, symbol: &str) -> Result<Option<CdlDojiSnapshot>, String> {
    let _ = create_research_tables_v74(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_doji WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_doji: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_doji: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_doji: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_doji: {e}"))?;
        let snap: CdlDojiSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_doji: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_hammer(
    conn: &Connection,
    symbol: &str,
    snap: &CdlHammerSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v74(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_hammer json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_hammer (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_hammer: {e}"))?;
    Ok(())
}

pub fn get_cdl_hammer(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlHammerSnapshot>, String> {
    let _ = create_research_tables_v74(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_hammer WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_hammer: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_hammer: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_hammer: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_hammer: {e}"))?;
        let snap: CdlHammerSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_hammer: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_shooting_star(
    conn: &Connection,
    symbol: &str,
    snap: &CdlShootingStarSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v74(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_shooting_star json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_shooting_star (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_shooting_star: {e}"))?;
    Ok(())
}

pub fn get_cdl_shooting_star(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlShootingStarSnapshot>, String> {
    let _ = create_research_tables_v74(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_shooting_star WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_shooting_star: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_shooting_star: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_shooting_star: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_shooting_star: {e}"))?;
        let snap: CdlShootingStarSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_shooting_star: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_engulfing(
    conn: &Connection,
    symbol: &str,
    snap: &CdlEngulfingSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v74(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_engulfing json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_engulfing (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_engulfing: {e}"))?;
    Ok(())
}

pub fn get_cdl_engulfing(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlEngulfingSnapshot>, String> {
    let _ = create_research_tables_v74(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_engulfing WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_engulfing: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_engulfing: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_engulfing: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_engulfing: {e}"))?;
        let snap: CdlEngulfingSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_engulfing: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_harami(
    conn: &Connection,
    symbol: &str,
    snap: &CdlHaramiSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v74(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_harami json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_harami (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_harami: {e}"))?;
    Ok(())
}

pub fn get_cdl_harami(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlHaramiSnapshot>, String> {
    let _ = create_research_tables_v74(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_harami WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_harami: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_harami: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_harami: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_harami: {e}"))?;
        let snap: CdlHaramiSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_harami: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

// ── CDL* 3-bar/2-bar SQLite schema + helpers ──

pub fn create_research_tables_v75(conn: &Connection) -> Result<(), String> {
    create_research_tables_v74(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_cdl_morning_star (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_morning_star_updated ON research_cdl_morning_star(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_evening_star (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_evening_star_updated ON research_cdl_evening_star(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_three_black_crows (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_three_black_crows_updated ON research_cdl_three_black_crows(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_three_white_soldiers (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_three_white_soldiers_updated ON research_cdl_three_white_soldiers(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_dark_cloud_cover (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_dark_cloud_cover_updated ON research_cdl_dark_cloud_cover(updated_at);",
    ).map_err(|e| format!("create v75 tables: {e}"))?;
    Ok(())
}

pub fn upsert_cdl_morning_star(
    conn: &Connection,
    symbol: &str,
    snap: &CdlMorningStarSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v75(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_morning_star json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_morning_star (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_morning_star: {e}"))?;
    Ok(())
}

pub fn get_cdl_morning_star(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlMorningStarSnapshot>, String> {
    let _ = create_research_tables_v75(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_morning_star WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_morning_star: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_morning_star: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_morning_star: {e}"))?
    {
        let j: String = r.get(0).map_err(|e| format!("get cdl_morning_star: {e}"))?;
        let snap: CdlMorningStarSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_morning_star: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_evening_star(
    conn: &Connection,
    symbol: &str,
    snap: &CdlEveningStarSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v75(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_evening_star json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_evening_star (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_evening_star: {e}"))?;
    Ok(())
}

pub fn get_cdl_evening_star(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlEveningStarSnapshot>, String> {
    let _ = create_research_tables_v75(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_evening_star WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_evening_star: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_evening_star: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_evening_star: {e}"))?
    {
        let j: String = r.get(0).map_err(|e| format!("get cdl_evening_star: {e}"))?;
        let snap: CdlEveningStarSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_evening_star: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_three_black_crows(
    conn: &Connection,
    symbol: &str,
    snap: &CdlThreeBlackCrowsSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v75(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_3bc json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_three_black_crows (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_3bc: {e}"))?;
    Ok(())
}

pub fn get_cdl_three_black_crows(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlThreeBlackCrowsSnapshot>, String> {
    let _ = create_research_tables_v75(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_three_black_crows WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_3bc: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_3bc: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_3bc: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_3bc: {e}"))?;
        let snap: CdlThreeBlackCrowsSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_3bc: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_three_white_soldiers(
    conn: &Connection,
    symbol: &str,
    snap: &CdlThreeWhiteSoldiersSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v75(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_3ws json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_three_white_soldiers (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_3ws: {e}"))?;
    Ok(())
}

pub fn get_cdl_three_white_soldiers(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlThreeWhiteSoldiersSnapshot>, String> {
    let _ = create_research_tables_v75(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_three_white_soldiers WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_3ws: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_3ws: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_3ws: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_3ws: {e}"))?;
        let snap: CdlThreeWhiteSoldiersSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_3ws: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_dark_cloud_cover(
    conn: &Connection,
    symbol: &str,
    snap: &CdlDarkCloudCoverSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v75(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_dcc json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_dark_cloud_cover (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_dcc: {e}"))?;
    Ok(())
}

pub fn get_cdl_dark_cloud_cover(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlDarkCloudCoverSnapshot>, String> {
    let _ = create_research_tables_v75(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_dark_cloud_cover WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_dcc: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_dcc: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_dcc: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_dcc: {e}"))?;
        let snap: CdlDarkCloudCoverSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_dcc: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

// ── v76 + upsert/get fns — CDLPIERCING / ──
//    CDLDRAGONFLYDOJI / CDLGRAVESTONEDOJI / CDLHANGINGMAN /
//    CDLINVERTEDHAMMER ──

pub fn create_research_tables_v76(conn: &Connection) -> Result<(), String> {
    create_research_tables_v75(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_cdl_piercing (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_piercing_updated ON research_cdl_piercing(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_dragonfly_doji (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_dragonfly_doji_updated ON research_cdl_dragonfly_doji(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_gravestone_doji (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_gravestone_doji_updated ON research_cdl_gravestone_doji(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_hanging_man (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_hanging_man_updated ON research_cdl_hanging_man(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_inverted_hammer (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_inverted_hammer_updated ON research_cdl_inverted_hammer(updated_at);",
    ).map_err(|e| format!("create v76 tables: {e}"))?;
    Ok(())
}

pub fn upsert_cdl_piercing(
    conn: &Connection,
    symbol: &str,
    snap: &CdlPiercingSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v76(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_piercing json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_piercing (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_piercing: {e}"))?;
    Ok(())
}

pub fn get_cdl_piercing(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlPiercingSnapshot>, String> {
    let _ = create_research_tables_v76(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_piercing WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_piercing: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_piercing: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_piercing: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_piercing: {e}"))?;
        let snap: CdlPiercingSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_piercing: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_dragonfly_doji(
    conn: &Connection,
    symbol: &str,
    snap: &CdlDragonflyDojiSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v76(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_dragonfly json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_dragonfly_doji (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_dragonfly: {e}"))?;
    Ok(())
}

pub fn get_cdl_dragonfly_doji(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlDragonflyDojiSnapshot>, String> {
    let _ = create_research_tables_v76(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_dragonfly_doji WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_dragonfly: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_dragonfly: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_dragonfly: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_dragonfly: {e}"))?;
        let snap: CdlDragonflyDojiSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_dragonfly: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_gravestone_doji(
    conn: &Connection,
    symbol: &str,
    snap: &CdlGravestoneDojiSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v76(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_gravestone json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_gravestone_doji (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_gravestone: {e}"))?;
    Ok(())
}

pub fn get_cdl_gravestone_doji(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlGravestoneDojiSnapshot>, String> {
    let _ = create_research_tables_v76(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_gravestone_doji WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_gravestone: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_gravestone: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_gravestone: {e}"))?
    {
        let j: String = r.get(0).map_err(|e| format!("get cdl_gravestone: {e}"))?;
        let snap: CdlGravestoneDojiSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_gravestone: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_hanging_man(
    conn: &Connection,
    symbol: &str,
    snap: &CdlHangingManSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v76(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_hanging_man json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_hanging_man (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_hanging_man: {e}"))?;
    Ok(())
}

pub fn get_cdl_hanging_man(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlHangingManSnapshot>, String> {
    let _ = create_research_tables_v76(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_hanging_man WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_hanging_man: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_hanging_man: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_hanging_man: {e}"))?
    {
        let j: String = r.get(0).map_err(|e| format!("get cdl_hanging_man: {e}"))?;
        let snap: CdlHangingManSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_hanging_man: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_inverted_hammer(
    conn: &Connection,
    symbol: &str,
    snap: &CdlInvertedHammerSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v76(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_inv_hammer json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_inverted_hammer (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_inv_hammer: {e}"))?;
    Ok(())
}

pub fn get_cdl_inverted_hammer(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlInvertedHammerSnapshot>, String> {
    let _ = create_research_tables_v76(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_inverted_hammer WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_inv_hammer: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_inv_hammer: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_inv_hammer: {e}"))?
    {
        let j: String = r.get(0).map_err(|e| format!("get cdl_inv_hammer: {e}"))?;
        let snap: CdlInvertedHammerSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_inv_hammer: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

// ── v77 + upsert/get fns — CDLHARAMICROSS / ──
//    CDLLONGLEGGEDDOJI / CDLMARUBOZU / CDLSPINNINGTOP / CDLTRISTAR ──

pub fn create_research_tables_v77(conn: &Connection) -> Result<(), String> {
    create_research_tables_v76(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_cdl_harami_cross (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_harami_cross_updated ON research_cdl_harami_cross(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_long_legged_doji (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_long_legged_doji_updated ON research_cdl_long_legged_doji(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_marubozu (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_marubozu_updated ON research_cdl_marubozu(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_spinning_top (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_spinning_top_updated ON research_cdl_spinning_top(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_tristar (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_tristar_updated ON research_cdl_tristar(updated_at);",
    ).map_err(|e| format!("create v77 tables: {e}"))?;
    Ok(())
}

pub fn upsert_cdl_harami_cross(
    conn: &Connection,
    symbol: &str,
    snap: &CdlHaramiCrossSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v77(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_harami_cross json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_harami_cross (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_harami_cross: {e}"))?;
    Ok(())
}

pub fn get_cdl_harami_cross(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlHaramiCrossSnapshot>, String> {
    let _ = create_research_tables_v77(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_harami_cross WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_harami_cross: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_harami_cross: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_harami_cross: {e}"))?
    {
        let j: String = r.get(0).map_err(|e| format!("get cdl_harami_cross: {e}"))?;
        let snap: CdlHaramiCrossSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_harami_cross: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_long_legged_doji(
    conn: &Connection,
    symbol: &str,
    snap: &CdlLongLeggedDojiSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v77(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_ll_doji json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_long_legged_doji (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_ll_doji: {e}"))?;
    Ok(())
}

pub fn get_cdl_long_legged_doji(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlLongLeggedDojiSnapshot>, String> {
    let _ = create_research_tables_v77(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_long_legged_doji WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_ll_doji: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_ll_doji: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_ll_doji: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_ll_doji: {e}"))?;
        let snap: CdlLongLeggedDojiSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_ll_doji: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_marubozu(
    conn: &Connection,
    symbol: &str,
    snap: &CdlMarubozuSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v77(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_marubozu json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_marubozu (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_marubozu: {e}"))?;
    Ok(())
}

pub fn get_cdl_marubozu(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlMarubozuSnapshot>, String> {
    let _ = create_research_tables_v77(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_marubozu WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_marubozu: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_marubozu: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_marubozu: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_marubozu: {e}"))?;
        let snap: CdlMarubozuSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_marubozu: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_spinning_top(
    conn: &Connection,
    symbol: &str,
    snap: &CdlSpinningTopSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v77(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_spinning json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_spinning_top (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_spinning: {e}"))?;
    Ok(())
}

pub fn get_cdl_spinning_top(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlSpinningTopSnapshot>, String> {
    let _ = create_research_tables_v77(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_spinning_top WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_spinning: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_spinning: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_spinning: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_spinning: {e}"))?;
        let snap: CdlSpinningTopSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_spinning: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_tristar(
    conn: &Connection,
    symbol: &str,
    snap: &CdlTristarSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v77(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_tristar json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_tristar (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_tristar: {e}"))?;
    Ok(())
}

pub fn get_cdl_tristar(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlTristarSnapshot>, String> {
    let _ = create_research_tables_v77(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_tristar WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_tristar: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_tristar: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_tristar: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_tristar: {e}"))?;
        let snap: CdlTristarSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_tristar: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}
