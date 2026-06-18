use super::*;

// Candlestick pattern storage/helpers
//    CDLDOJISTAR / CDLMORNINGDOJISTAR / CDLEVENINGDOJISTAR /
//    CDLABANDONEDBABY / CDL3INSIDE

pub fn create_research_tables_v80(conn: &Connection) -> Result<(), String> {
    create_research_tables_v79(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_cdl_doji_star (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_doji_star_updated ON research_cdl_doji_star(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_morning_doji_star (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_morning_doji_star_updated ON research_cdl_morning_doji_star(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_evening_doji_star (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_evening_doji_star_updated ON research_cdl_evening_doji_star(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_abandoned_baby (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_abandoned_baby_updated ON research_cdl_abandoned_baby(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_three_inside (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_three_inside_updated ON research_cdl_three_inside(updated_at);",
    ).map_err(|e| format!("create v80 tables: {e}"))?;
    Ok(())
}

pub fn upsert_cdl_doji_star(
    conn: &Connection,
    symbol: &str,
    snap: &CdlDojiStarSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v80(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_doji_star json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_doji_star (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_doji_star: {e}"))?;
    Ok(())
}

pub fn get_cdl_doji_star(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlDojiStarSnapshot>, String> {
    let _ = create_research_tables_v80(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_doji_star WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_doji_star: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_doji_star: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_doji_star: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_doji_star: {e}"))?;
        let snap: CdlDojiStarSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_doji_star: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_morning_doji_star(
    conn: &Connection,
    symbol: &str,
    snap: &CdlMorningDojiStarSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v80(conn);
    let json =
        serde_json::to_string(snap).map_err(|e| format!("cdl_morning_doji_star json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_morning_doji_star (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_morning_doji_star: {e}"))?;
    Ok(())
}

pub fn get_cdl_morning_doji_star(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlMorningDojiStarSnapshot>, String> {
    let _ = create_research_tables_v80(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_morning_doji_star WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_morning_doji_star: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_morning_doji_star: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_morning_doji_star: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_morning_doji_star: {e}"))?;
        let snap: CdlMorningDojiStarSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_morning_doji_star: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_evening_doji_star(
    conn: &Connection,
    symbol: &str,
    snap: &CdlEveningDojiStarSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v80(conn);
    let json =
        serde_json::to_string(snap).map_err(|e| format!("cdl_evening_doji_star json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_evening_doji_star (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_evening_doji_star: {e}"))?;
    Ok(())
}

pub fn get_cdl_evening_doji_star(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlEveningDojiStarSnapshot>, String> {
    let _ = create_research_tables_v80(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_evening_doji_star WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_evening_doji_star: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_evening_doji_star: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_evening_doji_star: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_evening_doji_star: {e}"))?;
        let snap: CdlEveningDojiStarSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_evening_doji_star: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_abandoned_baby(
    conn: &Connection,
    symbol: &str,
    snap: &CdlAbandonedBabySnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v80(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_abandoned_baby json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_abandoned_baby (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_abandoned_baby: {e}"))?;
    Ok(())
}

pub fn get_cdl_abandoned_baby(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlAbandonedBabySnapshot>, String> {
    let _ = create_research_tables_v80(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_abandoned_baby WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_abandoned_baby: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_abandoned_baby: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_abandoned_baby: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_abandoned_baby: {e}"))?;
        let snap: CdlAbandonedBabySnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_abandoned_baby: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_three_inside(
    conn: &Connection,
    symbol: &str,
    snap: &CdlThreeInsideSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v80(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_three_inside json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_three_inside (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_three_inside: {e}"))?;
    Ok(())
}

pub fn get_cdl_three_inside(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlThreeInsideSnapshot>, String> {
    let _ = create_research_tables_v80(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_three_inside WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_three_inside: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_three_inside: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_three_inside: {e}"))?
    {
        let j: String = r.get(0).map_err(|e| format!("get cdl_three_inside: {e}"))?;
        let snap: CdlThreeInsideSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_three_inside: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

// Candlestick pattern storage/helpers
//    CDL2CROWS / CDL3LINESTRIKE / CDL3OUTSIDE / CDLMATCHINGLOW

pub fn create_research_tables_v83(conn: &Connection) -> Result<(), String> {
    create_research_tables_v82(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_cdl_two_crows (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_two_crows_updated ON research_cdl_two_crows(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_three_line_strike (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_three_line_strike_updated ON research_cdl_three_line_strike(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_three_outside (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_three_outside_updated ON research_cdl_three_outside(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_matching_low (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_matching_low_updated ON research_cdl_matching_low(updated_at);",
    ).map_err(|e| format!("create v83 tables: {e}"))?;
    Ok(())
}

pub fn upsert_cdl_two_crows(
    conn: &Connection,
    symbol: &str,
    snap: &CdlTwoCrowsSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v83(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_two_crows json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_two_crows (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_two_crows: {e}"))?;
    Ok(())
}

pub fn get_cdl_two_crows(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlTwoCrowsSnapshot>, String> {
    let _ = create_research_tables_v83(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_two_crows WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_two_crows: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_two_crows: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_two_crows: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_two_crows: {e}"))?;
        let snap: CdlTwoCrowsSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_two_crows: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_three_line_strike(
    conn: &Connection,
    symbol: &str,
    snap: &CdlThreeLineStrikeSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v83(conn);
    let json =
        serde_json::to_string(snap).map_err(|e| format!("cdl_three_line_strike json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_three_line_strike (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_three_line_strike: {e}"))?;
    Ok(())
}

pub fn get_cdl_three_line_strike(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlThreeLineStrikeSnapshot>, String> {
    let _ = create_research_tables_v83(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_three_line_strike WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_three_line_strike: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_three_line_strike: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_three_line_strike: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_three_line_strike: {e}"))?;
        let snap: CdlThreeLineStrikeSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_three_line_strike: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_three_outside(
    conn: &Connection,
    symbol: &str,
    snap: &CdlThreeOutsideSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v83(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_three_outside json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_three_outside (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_three_outside: {e}"))?;
    Ok(())
}

pub fn get_cdl_three_outside(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlThreeOutsideSnapshot>, String> {
    let _ = create_research_tables_v83(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_three_outside WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_three_outside: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_three_outside: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_three_outside: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_three_outside: {e}"))?;
        let snap: CdlThreeOutsideSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_three_outside: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_matching_low(
    conn: &Connection,
    symbol: &str,
    snap: &CdlMatchingLowSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v83(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_matching_low json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_matching_low (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_matching_low: {e}"))?;
    Ok(())
}

pub fn get_cdl_matching_low(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlMatchingLowSnapshot>, String> {
    let _ = create_research_tables_v83(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_matching_low WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_matching_low: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_matching_low: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_matching_low: {e}"))?
    {
        let j: String = r.get(0).map_err(|e| format!("get cdl_matching_low: {e}"))?;
        let snap: CdlMatchingLowSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_matching_low: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

// Candlestick pattern storage/helpers
//    CDLSEPARATINGLINES / CDLSTICKSANDWICH / CDLRICKSHAWMAN / CDLTAKURI

pub fn create_research_tables_v84(conn: &Connection) -> Result<(), String> {
    create_research_tables_v83(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_cdl_separating_lines (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_separating_lines_updated ON research_cdl_separating_lines(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_stick_sandwich (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_stick_sandwich_updated ON research_cdl_stick_sandwich(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_rickshaw_man (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_rickshaw_man_updated ON research_cdl_rickshaw_man(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_takuri (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_takuri_updated ON research_cdl_takuri(updated_at);",
    ).map_err(|e| format!("create v84 tables: {e}"))?;
    Ok(())
}

pub fn upsert_cdl_separating_lines(
    conn: &Connection,
    symbol: &str,
    snap: &CdlSeparatingLinesSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v84(conn);
    let json =
        serde_json::to_string(snap).map_err(|e| format!("cdl_separating_lines json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_separating_lines (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_separating_lines: {e}"))?;
    Ok(())
}

pub fn get_cdl_separating_lines(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlSeparatingLinesSnapshot>, String> {
    let _ = create_research_tables_v84(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_separating_lines WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_separating_lines: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_separating_lines: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_separating_lines: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_separating_lines: {e}"))?;
        let snap: CdlSeparatingLinesSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_separating_lines: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_stick_sandwich(
    conn: &Connection,
    symbol: &str,
    snap: &CdlStickSandwichSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v84(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_stick_sandwich json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_stick_sandwich (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_stick_sandwich: {e}"))?;
    Ok(())
}

pub fn get_cdl_stick_sandwich(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlStickSandwichSnapshot>, String> {
    let _ = create_research_tables_v84(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_stick_sandwich WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_stick_sandwich: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_stick_sandwich: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_stick_sandwich: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_stick_sandwich: {e}"))?;
        let snap: CdlStickSandwichSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_stick_sandwich: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_rickshaw_man(
    conn: &Connection,
    symbol: &str,
    snap: &CdlRickshawManSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v84(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_rickshaw_man json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_rickshaw_man (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_rickshaw_man: {e}"))?;
    Ok(())
}

pub fn get_cdl_rickshaw_man(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlRickshawManSnapshot>, String> {
    let _ = create_research_tables_v84(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_rickshaw_man WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_rickshaw_man: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_rickshaw_man: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_rickshaw_man: {e}"))?
    {
        let j: String = r.get(0).map_err(|e| format!("get cdl_rickshaw_man: {e}"))?;
        let snap: CdlRickshawManSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_rickshaw_man: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_takuri(
    conn: &Connection,
    symbol: &str,
    snap: &CdlTakuriSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v84(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_takuri json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_takuri (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_takuri: {e}"))?;
    Ok(())
}

pub fn get_cdl_takuri(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlTakuriSnapshot>, String> {
    let _ = create_research_tables_v84(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_takuri WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_takuri: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_takuri: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_takuri: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_takuri: {e}"))?;
        let snap: CdlTakuriSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_takuri: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

// Candlestick pattern storage/helpers
//    CDL3STARSINSOUTH / CDLIDENTICAL3CROWS / CDLKICKING /
//    CDLKICKINGBYLENGTH / CDLLADDERBOTTOM / CDLUNIQUE3RIVER

pub fn create_research_tables_v85(conn: &Connection) -> Result<(), String> {
    create_research_tables_v84(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_cdl_three_stars_in_south (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_three_stars_in_south_updated ON research_cdl_three_stars_in_south(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_identical_three_crows (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_identical_three_crows_updated ON research_cdl_identical_three_crows(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_kicking (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_kicking_updated ON research_cdl_kicking(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_kicking_by_length (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_kicking_by_length_updated ON research_cdl_kicking_by_length(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_ladder_bottom (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_ladder_bottom_updated ON research_cdl_ladder_bottom(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_unique_three_river (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_unique_three_river_updated ON research_cdl_unique_three_river(updated_at);",
    ).map_err(|e| format!("create v85 tables: {e}"))?;
    Ok(())
}

pub fn upsert_cdl_three_stars_in_south(
    conn: &Connection,
    symbol: &str,
    snap: &CdlThreeStarsInSouthSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v85(conn);
    let json =
        serde_json::to_string(snap).map_err(|e| format!("cdl_three_stars_in_south json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_three_stars_in_south (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_three_stars_in_south: {e}"))?;
    Ok(())
}

pub fn get_cdl_three_stars_in_south(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlThreeStarsInSouthSnapshot>, String> {
    let _ = create_research_tables_v85(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_three_stars_in_south WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_three_stars_in_south: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_three_stars_in_south: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_three_stars_in_south: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_three_stars_in_south: {e}"))?;
        let snap: CdlThreeStarsInSouthSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_three_stars_in_south: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_identical_three_crows(
    conn: &Connection,
    symbol: &str,
    snap: &CdlIdenticalThreeCrowsSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v85(conn);
    let json =
        serde_json::to_string(snap).map_err(|e| format!("cdl_identical_three_crows json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_identical_three_crows (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_identical_three_crows: {e}"))?;
    Ok(())
}

pub fn get_cdl_identical_three_crows(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlIdenticalThreeCrowsSnapshot>, String> {
    let _ = create_research_tables_v85(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_identical_three_crows WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_identical_three_crows: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_identical_three_crows: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_identical_three_crows: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_identical_three_crows: {e}"))?;
        let snap: CdlIdenticalThreeCrowsSnapshot = serde_json::from_str(&j)
            .map_err(|e| format!("parse cdl_identical_three_crows: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_kicking(
    conn: &Connection,
    symbol: &str,
    snap: &CdlKickingSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v85(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_kicking json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_kicking (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_kicking: {e}"))?;
    Ok(())
}

pub fn get_cdl_kicking(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlKickingSnapshot>, String> {
    let _ = create_research_tables_v85(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_kicking WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_kicking: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_kicking: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_kicking: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_kicking: {e}"))?;
        let snap: CdlKickingSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_kicking: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_kicking_by_length(
    conn: &Connection,
    symbol: &str,
    snap: &CdlKickingByLengthSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v85(conn);
    let json =
        serde_json::to_string(snap).map_err(|e| format!("cdl_kicking_by_length json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_kicking_by_length (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_kicking_by_length: {e}"))?;
    Ok(())
}

pub fn get_cdl_kicking_by_length(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlKickingByLengthSnapshot>, String> {
    let _ = create_research_tables_v85(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_kicking_by_length WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_kicking_by_length: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_kicking_by_length: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_kicking_by_length: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_kicking_by_length: {e}"))?;
        let snap: CdlKickingByLengthSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_kicking_by_length: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_ladder_bottom(
    conn: &Connection,
    symbol: &str,
    snap: &CdlLadderBottomSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v85(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_ladder_bottom json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_ladder_bottom (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_ladder_bottom: {e}"))?;
    Ok(())
}

pub fn get_cdl_ladder_bottom(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlLadderBottomSnapshot>, String> {
    let _ = create_research_tables_v85(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_ladder_bottom WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_ladder_bottom: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_ladder_bottom: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_ladder_bottom: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_ladder_bottom: {e}"))?;
        let snap: CdlLadderBottomSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_ladder_bottom: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_unique_three_river(
    conn: &Connection,
    symbol: &str,
    snap: &CdlUniqueThreeRiverSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v85(conn);
    let json =
        serde_json::to_string(snap).map_err(|e| format!("cdl_unique_three_river json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_unique_three_river (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_unique_three_river: {e}"))?;
    Ok(())
}

pub fn get_cdl_unique_three_river(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlUniqueThreeRiverSnapshot>, String> {
    let _ = create_research_tables_v85(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_unique_three_river WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_unique_three_river: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_unique_three_river: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_unique_three_river: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_unique_three_river: {e}"))?;
        let snap: CdlUniqueThreeRiverSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_unique_three_river: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

// Candlestick pattern storage/helpers
//    CDLADVANCEBLOCK / CDLBREAKAWAY / CDLGAPSIDESIDEWHITE /
//    CDLUPSIDEGAP2CROWS / CDLXSIDEGAP3METHODS / CDLCONCEALBABYSWALL

pub fn create_research_tables_v86(conn: &Connection) -> Result<(), String> {
    create_research_tables_v85(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_cdl_advance_block (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_advance_block_updated ON research_cdl_advance_block(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_breakaway (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_breakaway_updated ON research_cdl_breakaway(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_gap_side_side_white (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_gap_side_side_white_updated ON research_cdl_gap_side_side_white(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_upside_gap_two_crows (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_upside_gap_two_crows_updated ON research_cdl_upside_gap_two_crows(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_xside_gap_three_methods (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_xside_gap_three_methods_updated ON research_cdl_xside_gap_three_methods(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_conceal_baby_swallow (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_conceal_baby_swallow_updated ON research_cdl_conceal_baby_swallow(updated_at);",
    ).map_err(|e| format!("create v86 tables: {e}"))?;
    Ok(())
}

pub fn upsert_cdl_advance_block(
    conn: &Connection,
    symbol: &str,
    snap: &CdlAdvanceBlockSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v86(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_advance_block json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_advance_block (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_advance_block: {e}"))?;
    Ok(())
}

pub fn get_cdl_advance_block(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlAdvanceBlockSnapshot>, String> {
    let _ = create_research_tables_v86(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_advance_block WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_advance_block: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_advance_block: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_advance_block: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_advance_block: {e}"))?;
        let snap: CdlAdvanceBlockSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_advance_block: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_breakaway(
    conn: &Connection,
    symbol: &str,
    snap: &CdlBreakawaySnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v86(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_breakaway json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_breakaway (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_breakaway: {e}"))?;
    Ok(())
}

pub fn get_cdl_breakaway(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlBreakawaySnapshot>, String> {
    let _ = create_research_tables_v86(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_breakaway WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_breakaway: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_breakaway: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_breakaway: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_breakaway: {e}"))?;
        let snap: CdlBreakawaySnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_breakaway: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_gap_side_side_white(
    conn: &Connection,
    symbol: &str,
    snap: &CdlGapSideSideWhiteSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v86(conn);
    let json =
        serde_json::to_string(snap).map_err(|e| format!("cdl_gap_side_side_white json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_gap_side_side_white (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_gap_side_side_white: {e}"))?;
    Ok(())
}

pub fn get_cdl_gap_side_side_white(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlGapSideSideWhiteSnapshot>, String> {
    let _ = create_research_tables_v86(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_gap_side_side_white WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_gap_side_side_white: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_gap_side_side_white: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_gap_side_side_white: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_gap_side_side_white: {e}"))?;
        let snap: CdlGapSideSideWhiteSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_gap_side_side_white: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_upside_gap_two_crows(
    conn: &Connection,
    symbol: &str,
    snap: &CdlUpsideGapTwoCrowsSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v86(conn);
    let json =
        serde_json::to_string(snap).map_err(|e| format!("cdl_upside_gap_two_crows json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_upside_gap_two_crows (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_upside_gap_two_crows: {e}"))?;
    Ok(())
}

pub fn get_cdl_upside_gap_two_crows(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlUpsideGapTwoCrowsSnapshot>, String> {
    let _ = create_research_tables_v86(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_upside_gap_two_crows WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_upside_gap_two_crows: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_upside_gap_two_crows: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_upside_gap_two_crows: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_upside_gap_two_crows: {e}"))?;
        let snap: CdlUpsideGapTwoCrowsSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_upside_gap_two_crows: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_xside_gap_three_methods(
    conn: &Connection,
    symbol: &str,
    snap: &CdlXSideGapThreeMethodsSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v86(conn);
    let json = serde_json::to_string(snap)
        .map_err(|e| format!("cdl_xside_gap_three_methods json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_xside_gap_three_methods (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_xside_gap_three_methods: {e}"))?;
    Ok(())
}

pub fn get_cdl_xside_gap_three_methods(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlXSideGapThreeMethodsSnapshot>, String> {
    let _ = create_research_tables_v86(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_xside_gap_three_methods WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_xside_gap_three_methods: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_xside_gap_three_methods: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_xside_gap_three_methods: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_xside_gap_three_methods: {e}"))?;
        let snap: CdlXSideGapThreeMethodsSnapshot = serde_json::from_str(&j)
            .map_err(|e| format!("parse cdl_xside_gap_three_methods: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_conceal_baby_swallow(
    conn: &Connection,
    symbol: &str,
    snap: &CdlConcealBabySwallowSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v86(conn);
    let json =
        serde_json::to_string(snap).map_err(|e| format!("cdl_conceal_baby_swallow json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_conceal_baby_swallow (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_conceal_baby_swallow: {e}"))?;
    Ok(())
}

pub fn get_cdl_conceal_baby_swallow(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlConcealBabySwallowSnapshot>, String> {
    let _ = create_research_tables_v86(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_conceal_baby_swallow WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_conceal_baby_swallow: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_conceal_baby_swallow: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_conceal_baby_swallow: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_conceal_baby_swallow: {e}"))?;
        let snap: CdlConcealBabySwallowSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_conceal_baby_swallow: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

// Candlestick pattern storage/helpers
//    CDLHIKKAKE / CDLHIKKAKEMOD / CDLMATHOLD /
//    CDLRISEFALL3METHODS

pub fn create_research_tables_v87(conn: &Connection) -> Result<(), String> {
    create_research_tables_v86(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_cdl_hikkake (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_hikkake_updated ON research_cdl_hikkake(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_hikkake_mod (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_hikkake_mod_updated ON research_cdl_hikkake_mod(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_mat_hold (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_mat_hold_updated ON research_cdl_mat_hold(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_rise_fall_three_methods (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_rise_fall_three_methods_updated ON research_cdl_rise_fall_three_methods(updated_at);",
    ).map_err(|e| format!("create v87 tables: {e}"))?;
    Ok(())
}

pub fn upsert_cdl_hikkake(
    conn: &Connection,
    symbol: &str,
    snap: &CdlHikkakeSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v87(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_hikkake json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_hikkake (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_hikkake: {e}"))?;
    Ok(())
}

pub fn get_cdl_hikkake(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlHikkakeSnapshot>, String> {
    let _ = create_research_tables_v87(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_hikkake WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_hikkake: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_hikkake: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_hikkake: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_hikkake: {e}"))?;
        let snap: CdlHikkakeSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_hikkake: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_hikkake_mod(
    conn: &Connection,
    symbol: &str,
    snap: &CdlHikkakeModSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v87(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_hikkake_mod json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_hikkake_mod (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_hikkake_mod: {e}"))?;
    Ok(())
}

pub fn get_cdl_hikkake_mod(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlHikkakeModSnapshot>, String> {
    let _ = create_research_tables_v87(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_hikkake_mod WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_hikkake_mod: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_hikkake_mod: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_hikkake_mod: {e}"))?
    {
        let j: String = r.get(0).map_err(|e| format!("get cdl_hikkake_mod: {e}"))?;
        let snap: CdlHikkakeModSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_hikkake_mod: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_mat_hold(
    conn: &Connection,
    symbol: &str,
    snap: &CdlMatHoldSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v87(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_mat_hold json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_mat_hold (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_mat_hold: {e}"))?;
    Ok(())
}

pub fn get_cdl_mat_hold(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlMatHoldSnapshot>, String> {
    let _ = create_research_tables_v87(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_mat_hold WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_mat_hold: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_mat_hold: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row cdl_mat_hold: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get cdl_mat_hold: {e}"))?;
        let snap: CdlMatHoldSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_mat_hold: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_rise_fall_three_methods(
    conn: &Connection,
    symbol: &str,
    snap: &CdlRiseFallThreeMethodsSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v87(conn);
    let json = serde_json::to_string(snap)
        .map_err(|e| format!("cdl_rise_fall_three_methods json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_rise_fall_three_methods (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_rise_fall_three_methods: {e}"))?;
    Ok(())
}

pub fn get_cdl_rise_fall_three_methods(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlRiseFallThreeMethodsSnapshot>, String> {
    let _ = create_research_tables_v87(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_rise_fall_three_methods WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_rise_fall_three_methods: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_rise_fall_three_methods: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_rise_fall_three_methods: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_rise_fall_three_methods: {e}"))?;
        let snap: CdlRiseFallThreeMethodsSnapshot = serde_json::from_str(&j)
            .map_err(|e| format!("parse cdl_rise_fall_three_methods: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

// Candlestick pattern storage/helpers
//    CDLSTALLEDPATTERN / CDLTASUKIGAP

pub fn create_research_tables_v88(conn: &Connection) -> Result<(), String> {
    create_research_tables_v87(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_cdl_stalled_pattern (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_stalled_pattern_updated ON research_cdl_stalled_pattern(updated_at);

        CREATE TABLE IF NOT EXISTS research_cdl_tasuki_gap (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cdl_tasuki_gap_updated ON research_cdl_tasuki_gap(updated_at);",
    ).map_err(|e| format!("create v88 tables: {e}"))?;
    Ok(())
}

pub fn upsert_cdl_stalled_pattern(
    conn: &Connection,
    symbol: &str,
    snap: &CdlStalledPatternSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v88(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_stalled_pattern json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_stalled_pattern (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_stalled_pattern: {e}"))?;
    Ok(())
}

pub fn get_cdl_stalled_pattern(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlStalledPatternSnapshot>, String> {
    let _ = create_research_tables_v88(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_stalled_pattern WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_stalled_pattern: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_stalled_pattern: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_stalled_pattern: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get cdl_stalled_pattern: {e}"))?;
        let snap: CdlStalledPatternSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_stalled_pattern: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_cdl_tasuki_gap(
    conn: &Connection,
    symbol: &str,
    snap: &CdlTasukiGapSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v88(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cdl_tasuki_gap json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cdl_tasuki_gap (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cdl_tasuki_gap: {e}"))?;
    Ok(())
}

pub fn get_cdl_tasuki_gap(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CdlTasukiGapSnapshot>, String> {
    let _ = create_research_tables_v88(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cdl_tasuki_gap WHERE symbol = ?1")
        .map_err(|e| format!("prep cdl_tasuki_gap: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query cdl_tasuki_gap: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row cdl_tasuki_gap: {e}"))?
    {
        let j: String = r.get(0).map_err(|e| format!("get cdl_tasuki_gap: {e}"))?;
        let snap: CdlTasukiGapSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse cdl_tasuki_gap: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}
