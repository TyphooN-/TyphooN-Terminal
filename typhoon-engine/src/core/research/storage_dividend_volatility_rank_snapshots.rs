use super::*;

// ── Round 20 schema + wrappers ────────────────────────────────────

pub fn create_research_tables_v20(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v19(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_dvdyieldrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_dvdyieldrank_updated ON research_dvdyieldrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_shrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_shrank_updated ON research_shrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_atrann (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_atrann_updated ON research_atrann(updated_at);

        CREATE TABLE IF NOT EXISTS research_ddhist (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_ddhist_updated ON research_ddhist(updated_at);

        CREATE TABLE IF NOT EXISTS research_priceperf (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_priceperf_updated ON research_priceperf(updated_at);",
    ).map_err(|e| format!("create v20 tables: {e}"))?;
    Ok(())
}

pub fn upsert_dvdyieldrank(
    conn: &Connection,
    symbol: &str,
    snap: &DividendYieldRankSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v20(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("dvdyieldrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_dvdyieldrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert dvdyieldrank: {e}"))?;
    Ok(())
}

pub fn get_dvdyieldrank(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<DividendYieldRankSnapshot>, String> {
    let _ = create_research_tables_v20(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_dvdyieldrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_dvdyieldrank: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_dvdyieldrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_dvdyieldrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_shrank(
    conn: &Connection,
    symbol: &str,
    snap: &ShortInterestRankSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v20(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("shrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_shrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert shrank: {e}"))?;
    Ok(())
}

pub fn get_shrank(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<ShortInterestRankSnapshot>, String> {
    let _ = create_research_tables_v20(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_shrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_shrank: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_shrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_shrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_atrann(
    conn: &Connection,
    symbol: &str,
    snap: &AnnualizedAtrSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v20(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("atrann json: {e}"))?;
    conn.execute(
        "INSERT INTO research_atrann(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert atrann: {e}"))?;
    Ok(())
}

pub fn get_atrann(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<AnnualizedAtrSnapshot>, String> {
    let _ = create_research_tables_v20(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_atrann WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_atrann: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_atrann: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_atrann: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_ddhist(
    conn: &Connection,
    symbol: &str,
    snap: &DrawdownHistorySnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v20(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ddhist json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ddhist(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ddhist: {e}"))?;
    Ok(())
}

pub fn get_ddhist(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<DrawdownHistorySnapshot>, String> {
    let _ = create_research_tables_v20(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_ddhist WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ddhist: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_ddhist: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ddhist: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_priceperf(
    conn: &Connection,
    symbol: &str,
    snap: &PricePerformanceSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v20(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("priceperf json: {e}"))?;
    conn.execute(
        "INSERT INTO research_priceperf(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert priceperf: {e}"))?;
    Ok(())
}

pub fn get_priceperf(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<PricePerformanceSnapshot>, String> {
    let _ = create_research_tables_v20(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_priceperf WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_priceperf: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_priceperf: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_priceperf: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── Round 21 schema v21 + wrappers ──

pub fn create_research_tables_v21(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v20(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_betarank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_betarank_updated ON research_betarank(updated_at);

        CREATE TABLE IF NOT EXISTS research_pegrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_pegrank_updated ON research_pegrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_fhighlow (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_fhighlow_updated ON research_fhighlow(updated_at);

        CREATE TABLE IF NOT EXISTS research_rvcone (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_rvcone_updated ON research_rvcone(updated_at);

        CREATE TABLE IF NOT EXISTS research_calpb (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_calpb_updated ON research_calpb(updated_at);",
    )
    .map_err(|e| format!("create v21 tables: {e}"))?;
    Ok(())
}

pub fn upsert_betarank(
    conn: &Connection,
    symbol: &str,
    snap: &BetaRankSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v21(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("betarank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_betarank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert betarank: {e}"))?;
    Ok(())
}

pub fn get_betarank(conn: &Connection, symbol: &str) -> Result<Option<BetaRankSnapshot>, String> {
    let _ = create_research_tables_v21(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_betarank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_betarank: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_betarank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_betarank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_pegrank(
    conn: &Connection,
    symbol: &str,
    snap: &PegRankSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v21(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("pegrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_pegrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert pegrank: {e}"))?;
    Ok(())
}

pub fn get_pegrank(conn: &Connection, symbol: &str) -> Result<Option<PegRankSnapshot>, String> {
    let _ = create_research_tables_v21(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_pegrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_pegrank: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_pegrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_pegrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_fhighlow(
    conn: &Connection,
    symbol: &str,
    snap: &FiftyTwoWeekHighLowSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v21(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("fhighlow json: {e}"))?;
    conn.execute(
        "INSERT INTO research_fhighlow(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert fhighlow: {e}"))?;
    Ok(())
}

pub fn get_fhighlow(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<FiftyTwoWeekHighLowSnapshot>, String> {
    let _ = create_research_tables_v21(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_fhighlow WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_fhighlow: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_fhighlow: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_fhighlow: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_rvcone(
    conn: &Connection,
    symbol: &str,
    snap: &RealizedVolConeSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v21(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("rvcone json: {e}"))?;
    conn.execute(
        "INSERT INTO research_rvcone(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rvcone: {e}"))?;
    Ok(())
}

pub fn get_rvcone(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<RealizedVolConeSnapshot>, String> {
    let _ = create_research_tables_v21(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_rvcone WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_rvcone: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_rvcone: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_rvcone: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_calpb(
    conn: &Connection,
    symbol: &str,
    snap: &CalendarPeriodBreakdownSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v21(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("calpb json: {e}"))?;
    conn.execute(
        "INSERT INTO research_calpb(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert calpb: {e}"))?;
    Ok(())
}

pub fn get_calpb(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CalendarPeriodBreakdownSnapshot>, String> {
    let _ = create_research_tables_v21(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_calpb WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_calpb: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_calpb: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_calpb: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}
