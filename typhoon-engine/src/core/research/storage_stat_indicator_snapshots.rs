use super::*;

/// adds `research_durbinwatson`,
/// `research_bdstest`, `research_breuschpagan`, `research_turnpts`,
/// `research_periodogram`. Additive over v40.
pub fn create_research_tables_v41(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v40(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_durbinwatson (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_durbinwatson_updated ON research_durbinwatson(updated_at);

        CREATE TABLE IF NOT EXISTS research_bdstest (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_bdstest_updated ON research_bdstest(updated_at);

        CREATE TABLE IF NOT EXISTS research_breuschpagan (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_breuschpagan_updated ON research_breuschpagan(updated_at);

        CREATE TABLE IF NOT EXISTS research_turnpts (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_turnpts_updated ON research_turnpts(updated_at);

        CREATE TABLE IF NOT EXISTS research_periodogram (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_periodogram_updated ON research_periodogram(updated_at);",
    ).map_err(|e| format!("create v41 tables: {e}"))?;
    Ok(())
}

pub fn upsert_durbinwatson(
    conn: &Connection,
    symbol: &str,
    snap: &DurbinWatsonSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v41(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("durbinwatson json: {e}"))?;
    conn.execute(
        "INSERT INTO research_durbinwatson(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert durbinwatson: {e}"))?;
    Ok(())
}

pub fn get_durbinwatson(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<DurbinWatsonSnapshot>, String> {
    let _ = create_research_tables_v41(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_durbinwatson WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_durbinwatson: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_durbinwatson: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_durbinwatson: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_bdstest(
    conn: &Connection,
    symbol: &str,
    snap: &BdsTestSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v41(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("bdstest json: {e}"))?;
    conn.execute(
        "INSERT INTO research_bdstest(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert bdstest: {e}"))?;
    Ok(())
}

pub fn get_bdstest(conn: &Connection, symbol: &str) -> Result<Option<BdsTestSnapshot>, String> {
    let _ = create_research_tables_v41(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_bdstest WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_bdstest: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_bdstest: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_bdstest: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_breuschpagan(
    conn: &Connection,
    symbol: &str,
    snap: &BreuschPaganSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v41(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("breuschpagan json: {e}"))?;
    conn.execute(
        "INSERT INTO research_breuschpagan(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert breuschpagan: {e}"))?;
    Ok(())
}

pub fn get_breuschpagan(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<BreuschPaganSnapshot>, String> {
    let _ = create_research_tables_v41(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_breuschpagan WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_breuschpagan: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_breuschpagan: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_breuschpagan: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_turnpts(
    conn: &Connection,
    symbol: &str,
    snap: &TurnPtsSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v41(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("turnpts json: {e}"))?;
    conn.execute(
        "INSERT INTO research_turnpts(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert turnpts: {e}"))?;
    Ok(())
}

pub fn get_turnpts(conn: &Connection, symbol: &str) -> Result<Option<TurnPtsSnapshot>, String> {
    let _ = create_research_tables_v41(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_turnpts WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_turnpts: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_turnpts: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_turnpts: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_periodogram(
    conn: &Connection,
    symbol: &str,
    snap: &PeriodogramSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v41(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("periodogram json: {e}"))?;
    conn.execute(
        "INSERT INTO research_periodogram(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert periodogram: {e}"))?;
    Ok(())
}

pub fn get_periodogram(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<PeriodogramSnapshot>, String> {
    let _ = create_research_tables_v41(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_periodogram WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_periodogram: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_periodogram: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_periodogram: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

/// adds `research_mcleodli`, `research_oufit`,
/// `research_gph`, `research_burgspec`, `research_kendalltau`. Additive over v41.
pub fn create_research_tables_v42(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v41(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_mcleodli (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_mcleodli_updated ON research_mcleodli(updated_at);

        CREATE TABLE IF NOT EXISTS research_oufit (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_oufit_updated ON research_oufit(updated_at);

        CREATE TABLE IF NOT EXISTS research_gph (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_gph_updated ON research_gph(updated_at);

        CREATE TABLE IF NOT EXISTS research_burgspec (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_burgspec_updated ON research_burgspec(updated_at);

        CREATE TABLE IF NOT EXISTS research_kendalltau (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_kendalltau_updated ON research_kendalltau(updated_at);",
    ).map_err(|e| format!("create v42 tables: {e}"))?;
    Ok(())
}

pub fn upsert_mcleodli(
    conn: &Connection,
    symbol: &str,
    snap: &McLeodLiSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v42(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("mcleodli json: {e}"))?;
    conn.execute(
        "INSERT INTO research_mcleodli(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert mcleodli: {e}"))?;
    Ok(())
}

pub fn get_mcleodli(conn: &Connection, symbol: &str) -> Result<Option<McLeodLiSnapshot>, String> {
    let _ = create_research_tables_v42(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_mcleodli WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_mcleodli: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_mcleodli: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_mcleodli: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_oufit(conn: &Connection, symbol: &str, snap: &OuFitSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v42(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("oufit json: {e}"))?;
    conn.execute(
        "INSERT INTO research_oufit(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert oufit: {e}"))?;
    Ok(())
}

pub fn get_oufit(conn: &Connection, symbol: &str) -> Result<Option<OuFitSnapshot>, String> {
    let _ = create_research_tables_v42(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_oufit WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_oufit: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_oufit: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_oufit: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_gph(conn: &Connection, symbol: &str, snap: &GphSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v42(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("gph json: {e}"))?;
    conn.execute(
        "INSERT INTO research_gph(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert gph: {e}"))?;
    Ok(())
}

pub fn get_gph(conn: &Connection, symbol: &str) -> Result<Option<GphSnapshot>, String> {
    let _ = create_research_tables_v42(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_gph WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_gph: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_gph: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_gph: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_burgspec(
    conn: &Connection,
    symbol: &str,
    snap: &BurgSpecSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v42(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("burgspec json: {e}"))?;
    conn.execute(
        "INSERT INTO research_burgspec(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert burgspec: {e}"))?;
    Ok(())
}

pub fn get_burgspec(conn: &Connection, symbol: &str) -> Result<Option<BurgSpecSnapshot>, String> {
    let _ = create_research_tables_v42(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_burgspec WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_burgspec: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_burgspec: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_burgspec: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_kendalltau(
    conn: &Connection,
    symbol: &str,
    snap: &KendallTauSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v42(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("kendalltau json: {e}"))?;
    conn.execute(
        "INSERT INTO research_kendalltau(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert kendalltau: {e}"))?;
    Ok(())
}

pub fn get_kendalltau(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<KendallTauSnapshot>, String> {
    let _ = create_research_tables_v42(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_kendalltau WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_kendalltau: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_kendalltau: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_kendalltau: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── Research section ──

pub fn create_research_tables_v43(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v42(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_squeeze (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_squeeze_updated ON research_squeeze(updated_at);

        CREATE TABLE IF NOT EXISTS research_squeezerank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_squeezerank_updated ON research_squeezerank(updated_at);

        CREATE TABLE IF NOT EXISTS research_bbsqueeze (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_bbsqueeze_updated ON research_bbsqueeze(updated_at);

        CREATE TABLE IF NOT EXISTS research_donchian (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_donchian_updated ON research_donchian(updated_at);

        CREATE TABLE IF NOT EXISTS research_kama (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_kama_updated ON research_kama(updated_at);",
    ).map_err(|e| format!("create v43 tables: {e}"))?;
    Ok(())
}

pub fn upsert_squeeze(
    conn: &Connection,
    symbol: &str,
    snap: &SqueezeSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v43(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("squeeze json: {e}"))?;
    conn.execute(
        "INSERT INTO research_squeeze(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert squeeze: {e}"))?;
    Ok(())
}

pub fn get_squeeze(conn: &Connection, symbol: &str) -> Result<Option<SqueezeSnapshot>, String> {
    let _ = create_research_tables_v43(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_squeeze WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_squeeze: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_squeeze: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_squeeze: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

/// Whole-table scan of `research_squeeze`. Used by SQUEEZERANK and the standalone watchlist UI.
pub fn get_all_squeeze(conn: &Connection) -> Result<Vec<SqueezeSnapshot>, String> {
    let _ = create_research_tables_v43(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_squeeze")
        .map_err(|e| format!("prepare get_all_squeeze: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_squeeze: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<SqueezeSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

pub fn upsert_squeezerank(
    conn: &Connection,
    symbol: &str,
    snap: &SqueezeRankSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v43(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("squeezerank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_squeezerank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert squeezerank: {e}"))?;
    Ok(())
}

pub fn get_squeezerank(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<SqueezeRankSnapshot>, String> {
    let _ = create_research_tables_v43(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_squeezerank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_squeezerank: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_squeezerank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_squeezerank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_bbsqueeze(
    conn: &Connection,
    symbol: &str,
    snap: &BbsqueezeSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v43(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("bbsqueeze json: {e}"))?;
    conn.execute(
        "INSERT INTO research_bbsqueeze(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert bbsqueeze: {e}"))?;
    Ok(())
}

pub fn get_bbsqueeze(conn: &Connection, symbol: &str) -> Result<Option<BbsqueezeSnapshot>, String> {
    let _ = create_research_tables_v43(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_bbsqueeze WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_bbsqueeze: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_bbsqueeze: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_bbsqueeze: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_donchian(
    conn: &Connection,
    symbol: &str,
    snap: &DonchianSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v43(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("donchian json: {e}"))?;
    conn.execute(
        "INSERT INTO research_donchian(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert donchian: {e}"))?;
    Ok(())
}

pub fn get_donchian(conn: &Connection, symbol: &str) -> Result<Option<DonchianSnapshot>, String> {
    let _ = create_research_tables_v43(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_donchian WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_donchian: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_donchian: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_donchian: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_kama(conn: &Connection, symbol: &str, snap: &KamaSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v43(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("kama json: {e}"))?;
    conn.execute(
        "INSERT INTO research_kama(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert kama: {e}"))?;
    Ok(())
}

pub fn get_kama(conn: &Connection, symbol: &str) -> Result<Option<KamaSnapshot>, String> {
    let _ = create_research_tables_v43(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_kama WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_kama: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_kama: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_kama: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── Research section ──

pub fn create_research_tables_v44(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v43(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_ichimoku (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_ichimoku_updated ON research_ichimoku(updated_at);

        CREATE TABLE IF NOT EXISTS research_supertrend (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_supertrend_updated ON research_supertrend(updated_at);

        CREATE TABLE IF NOT EXISTS research_keltner (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_keltner_updated ON research_keltner(updated_at);

        CREATE TABLE IF NOT EXISTS research_fisher (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_fisher_updated ON research_fisher(updated_at);

        CREATE TABLE IF NOT EXISTS research_aroon (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_aroon_updated ON research_aroon(updated_at);",
    ).map_err(|e| format!("create v44 tables: {e}"))?;
    Ok(())
}

pub fn upsert_ichimoku(
    conn: &Connection,
    symbol: &str,
    snap: &IchimokuSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v44(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ichimoku json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ichimoku(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ichimoku: {e}"))?;
    Ok(())
}

pub fn get_ichimoku(conn: &Connection, symbol: &str) -> Result<Option<IchimokuSnapshot>, String> {
    let _ = create_research_tables_v44(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_ichimoku WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ichimoku: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_ichimoku: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ichimoku: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_supertrend(
    conn: &Connection,
    symbol: &str,
    snap: &SupertrendSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v44(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("supertrend json: {e}"))?;
    conn.execute(
        "INSERT INTO research_supertrend(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert supertrend: {e}"))?;
    Ok(())
}

pub fn get_supertrend(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<SupertrendSnapshot>, String> {
    let _ = create_research_tables_v44(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_supertrend WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_supertrend: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_supertrend: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_supertrend: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_keltner(
    conn: &Connection,
    symbol: &str,
    snap: &KeltnerSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v44(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("keltner json: {e}"))?;
    conn.execute(
        "INSERT INTO research_keltner(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert keltner: {e}"))?;
    Ok(())
}

pub fn get_keltner(conn: &Connection, symbol: &str) -> Result<Option<KeltnerSnapshot>, String> {
    let _ = create_research_tables_v44(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_keltner WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_keltner: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_keltner: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_keltner: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_fisher(conn: &Connection, symbol: &str, snap: &FisherSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v44(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("fisher json: {e}"))?;
    conn.execute(
        "INSERT INTO research_fisher(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert fisher: {e}"))?;
    Ok(())
}

pub fn get_fisher(conn: &Connection, symbol: &str) -> Result<Option<FisherSnapshot>, String> {
    let _ = create_research_tables_v44(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_fisher WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_fisher: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_fisher: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_fisher: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_aroon(conn: &Connection, symbol: &str, snap: &AroonSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v44(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("aroon json: {e}"))?;
    conn.execute(
        "INSERT INTO research_aroon(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert aroon: {e}"))?;
    Ok(())
}

pub fn get_aroon(conn: &Connection, symbol: &str) -> Result<Option<AroonSnapshot>, String> {
    let _ = create_research_tables_v44(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_aroon WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_aroon: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_aroon: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_aroon: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn create_research_tables_v45(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v44(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_adx (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_adx_updated ON research_adx(updated_at);

        CREATE TABLE IF NOT EXISTS research_cci (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cci_updated ON research_cci(updated_at);

        CREATE TABLE IF NOT EXISTS research_cmf (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cmf_updated ON research_cmf(updated_at);

        CREATE TABLE IF NOT EXISTS research_mfi (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_mfi_updated ON research_mfi(updated_at);

        CREATE TABLE IF NOT EXISTS research_psar (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_psar_updated ON research_psar(updated_at);",
    )
    .map_err(|e| format!("create v45 tables: {e}"))?;
    Ok(())
}

pub fn upsert_adx(conn: &Connection, symbol: &str, snap: &AdxSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v45(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("adx json: {e}"))?;
    conn.execute(
        "INSERT INTO research_adx(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert adx: {e}"))?;
    Ok(())
}

pub fn get_adx(conn: &Connection, symbol: &str) -> Result<Option<AdxSnapshot>, String> {
    let _ = create_research_tables_v45(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_adx WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_adx: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_adx: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_adx: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_cci(conn: &Connection, symbol: &str, snap: &CciSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v45(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cci json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cci(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cci: {e}"))?;
    Ok(())
}

pub fn get_cci(conn: &Connection, symbol: &str) -> Result<Option<CciSnapshot>, String> {
    let _ = create_research_tables_v45(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cci WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_cci: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_cci: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_cci: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_cmf(conn: &Connection, symbol: &str, snap: &CmfSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v45(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cmf json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cmf(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cmf: {e}"))?;
    Ok(())
}

pub fn get_cmf(conn: &Connection, symbol: &str) -> Result<Option<CmfSnapshot>, String> {
    let _ = create_research_tables_v45(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cmf WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_cmf: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_cmf: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_cmf: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_mfi(conn: &Connection, symbol: &str, snap: &MfiSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v45(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("mfi json: {e}"))?;
    conn.execute(
        "INSERT INTO research_mfi(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert mfi: {e}"))?;
    Ok(())
}

pub fn get_mfi(conn: &Connection, symbol: &str) -> Result<Option<MfiSnapshot>, String> {
    let _ = create_research_tables_v45(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_mfi WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_mfi: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_mfi: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_mfi: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_psar(conn: &Connection, symbol: &str, snap: &PsarSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v45(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("psar json: {e}"))?;
    conn.execute(
        "INSERT INTO research_psar(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert psar: {e}"))?;
    Ok(())
}

pub fn get_psar(conn: &Connection, symbol: &str) -> Result<Option<PsarSnapshot>, String> {
    let _ = create_research_tables_v45(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_psar WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_psar: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_psar: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_psar: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn create_research_tables_v46(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v45(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_vortex (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_vortex_updated ON research_vortex(updated_at);

        CREATE TABLE IF NOT EXISTS research_chop (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_chop_updated ON research_chop(updated_at);

        CREATE TABLE IF NOT EXISTS research_obv (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_obv_updated ON research_obv(updated_at);

        CREATE TABLE IF NOT EXISTS research_trix (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_trix_updated ON research_trix(updated_at);

        CREATE TABLE IF NOT EXISTS research_hma (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_hma_updated ON research_hma(updated_at);",
    )
    .map_err(|e| format!("create v46 tables: {e}"))?;
    Ok(())
}

pub fn upsert_vortex(conn: &Connection, symbol: &str, snap: &VortexSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v46(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("vortex json: {e}"))?;
    conn.execute(
        "INSERT INTO research_vortex(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert vortex: {e}"))?;
    Ok(())
}

pub fn get_vortex(conn: &Connection, symbol: &str) -> Result<Option<VortexSnapshot>, String> {
    let _ = create_research_tables_v46(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_vortex WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_vortex: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_vortex: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_vortex: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_chop(conn: &Connection, symbol: &str, snap: &ChopSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v46(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("chop json: {e}"))?;
    conn.execute(
        "INSERT INTO research_chop(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert chop: {e}"))?;
    Ok(())
}

pub fn get_chop(conn: &Connection, symbol: &str) -> Result<Option<ChopSnapshot>, String> {
    let _ = create_research_tables_v46(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_chop WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_chop: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_chop: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_chop: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_obv(conn: &Connection, symbol: &str, snap: &ObvSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v46(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("obv json: {e}"))?;
    conn.execute(
        "INSERT INTO research_obv(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert obv: {e}"))?;
    Ok(())
}

pub fn get_obv(conn: &Connection, symbol: &str) -> Result<Option<ObvSnapshot>, String> {
    let _ = create_research_tables_v46(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_obv WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_obv: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_obv: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_obv: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_trix(conn: &Connection, symbol: &str, snap: &TrixSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v46(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("trix json: {e}"))?;
    conn.execute(
        "INSERT INTO research_trix(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert trix: {e}"))?;
    Ok(())
}

pub fn get_trix(conn: &Connection, symbol: &str) -> Result<Option<TrixSnapshot>, String> {
    let _ = create_research_tables_v46(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_trix WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_trix: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_trix: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_trix: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_hma(conn: &Connection, symbol: &str, snap: &HmaSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v46(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("hma json: {e}"))?;
    conn.execute(
        "INSERT INTO research_hma(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert hma: {e}"))?;
    Ok(())
}

pub fn get_hma(conn: &Connection, symbol: &str) -> Result<Option<HmaSnapshot>, String> {
    let _ = create_research_tables_v46(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_hma WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_hma: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_hma: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_hma: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn create_research_tables_v47(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v46(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_ppo (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_ppo_updated ON research_ppo(updated_at);

        CREATE TABLE IF NOT EXISTS research_dpo (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_dpo_updated ON research_dpo(updated_at);

        CREATE TABLE IF NOT EXISTS research_kst (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_kst_updated ON research_kst(updated_at);

        CREATE TABLE IF NOT EXISTS research_ultosc (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_ultosc_updated ON research_ultosc(updated_at);

        CREATE TABLE IF NOT EXISTS research_willr (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_willr_updated ON research_willr(updated_at);",
    )
    .map_err(|e| format!("create v47 tables: {e}"))?;
    Ok(())
}

pub fn upsert_ppo(conn: &Connection, symbol: &str, snap: &PpoSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v47(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ppo json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ppo(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ppo: {e}"))?;
    Ok(())
}

pub fn get_ppo(conn: &Connection, symbol: &str) -> Result<Option<PpoSnapshot>, String> {
    let _ = create_research_tables_v47(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_ppo WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ppo: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_ppo: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ppo: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_dpo(conn: &Connection, symbol: &str, snap: &DpoSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v47(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("dpo json: {e}"))?;
    conn.execute(
        "INSERT INTO research_dpo(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert dpo: {e}"))?;
    Ok(())
}

pub fn get_dpo(conn: &Connection, symbol: &str) -> Result<Option<DpoSnapshot>, String> {
    let _ = create_research_tables_v47(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_dpo WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_dpo: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_dpo: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_dpo: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_kst(conn: &Connection, symbol: &str, snap: &KstSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v47(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("kst json: {e}"))?;
    conn.execute(
        "INSERT INTO research_kst(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert kst: {e}"))?;
    Ok(())
}

pub fn get_kst(conn: &Connection, symbol: &str) -> Result<Option<KstSnapshot>, String> {
    let _ = create_research_tables_v47(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_kst WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_kst: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_kst: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_kst: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_ultosc(conn: &Connection, symbol: &str, snap: &UltoscSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v47(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ultosc json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ultosc(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ultosc: {e}"))?;
    Ok(())
}

pub fn get_ultosc(conn: &Connection, symbol: &str) -> Result<Option<UltoscSnapshot>, String> {
    let _ = create_research_tables_v47(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_ultosc WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ultosc: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_ultosc: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ultosc: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_willr(conn: &Connection, symbol: &str, snap: &WillrSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v47(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("willr json: {e}"))?;
    conn.execute(
        "INSERT INTO research_willr(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert willr: {e}"))?;
    Ok(())
}

pub fn get_willr(conn: &Connection, symbol: &str) -> Result<Option<WillrSnapshot>, String> {
    let _ = create_research_tables_v47(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_willr WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_willr: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_willr: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_willr: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn create_research_tables_v48(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v47(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_mass (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_mass_updated ON research_mass(updated_at);

        CREATE TABLE IF NOT EXISTS research_chaikosc (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_chaikosc_updated ON research_chaikosc(updated_at);

        CREATE TABLE IF NOT EXISTS research_klinger (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_klinger_updated ON research_klinger(updated_at);

        CREATE TABLE IF NOT EXISTS research_stochrsi (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_stochrsi_updated ON research_stochrsi(updated_at);

        CREATE TABLE IF NOT EXISTS research_awesome (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_awesome_updated ON research_awesome(updated_at);",
    )
    .map_err(|e| format!("create v48 tables: {e}"))?;
    Ok(())
}

pub fn upsert_mass(conn: &Connection, symbol: &str, snap: &MassSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v48(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("mass json: {e}"))?;
    conn.execute(
        "INSERT INTO research_mass(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert mass: {e}"))?;
    Ok(())
}

pub fn get_mass(conn: &Connection, symbol: &str) -> Result<Option<MassSnapshot>, String> {
    let _ = create_research_tables_v48(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_mass WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_mass: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_mass: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_mass: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_chaikosc(
    conn: &Connection,
    symbol: &str,
    snap: &ChaikoscSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v48(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("chaikosc json: {e}"))?;
    conn.execute(
        "INSERT INTO research_chaikosc(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert chaikosc: {e}"))?;
    Ok(())
}

pub fn get_chaikosc(conn: &Connection, symbol: &str) -> Result<Option<ChaikoscSnapshot>, String> {
    let _ = create_research_tables_v48(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_chaikosc WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_chaikosc: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_chaikosc: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_chaikosc: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_klinger(
    conn: &Connection,
    symbol: &str,
    snap: &KlingerSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v48(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("klinger json: {e}"))?;
    conn.execute(
        "INSERT INTO research_klinger(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert klinger: {e}"))?;
    Ok(())
}

pub fn get_klinger(conn: &Connection, symbol: &str) -> Result<Option<KlingerSnapshot>, String> {
    let _ = create_research_tables_v48(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_klinger WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_klinger: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_klinger: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_klinger: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_stochrsi(
    conn: &Connection,
    symbol: &str,
    snap: &StochRsiSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v48(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("stochrsi json: {e}"))?;
    conn.execute(
        "INSERT INTO research_stochrsi(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert stochrsi: {e}"))?;
    Ok(())
}

pub fn get_stochrsi(conn: &Connection, symbol: &str) -> Result<Option<StochRsiSnapshot>, String> {
    let _ = create_research_tables_v48(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_stochrsi WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_stochrsi: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_stochrsi: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_stochrsi: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_awesome(
    conn: &Connection,
    symbol: &str,
    snap: &AwesomeSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v48(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("awesome json: {e}"))?;
    conn.execute(
        "INSERT INTO research_awesome(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert awesome: {e}"))?;
    Ok(())
}

pub fn get_awesome(conn: &Connection, symbol: &str) -> Result<Option<AwesomeSnapshot>, String> {
    let _ = create_research_tables_v48(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_awesome WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_awesome: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_awesome: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_awesome: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

/// tables: EFI, EMV, NVI, PVI, COPPOCK.
pub fn create_research_tables_v49(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v48(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_efi (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_efi_updated ON research_efi(updated_at);

        CREATE TABLE IF NOT EXISTS research_emv (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_emv_updated ON research_emv(updated_at);

        CREATE TABLE IF NOT EXISTS research_nvi (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_nvi_updated ON research_nvi(updated_at);

        CREATE TABLE IF NOT EXISTS research_pvi (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_pvi_updated ON research_pvi(updated_at);

        CREATE TABLE IF NOT EXISTS research_coppock (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_coppock_updated ON research_coppock(updated_at);",
    )
    .map_err(|e| format!("create v49 tables: {e}"))?;
    Ok(())
}

pub fn upsert_efi(conn: &Connection, symbol: &str, snap: &EfiSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v49(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("efi json: {e}"))?;
    conn.execute(
        "INSERT INTO research_efi(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert efi: {e}"))?;
    Ok(())
}

pub fn get_efi(conn: &Connection, symbol: &str) -> Result<Option<EfiSnapshot>, String> {
    let _ = create_research_tables_v49(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_efi WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_efi: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_efi: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_efi: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_emv(conn: &Connection, symbol: &str, snap: &EmvSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v49(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("emv json: {e}"))?;
    conn.execute(
        "INSERT INTO research_emv(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert emv: {e}"))?;
    Ok(())
}

pub fn get_emv(conn: &Connection, symbol: &str) -> Result<Option<EmvSnapshot>, String> {
    let _ = create_research_tables_v49(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_emv WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_emv: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_emv: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_emv: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_nvi(conn: &Connection, symbol: &str, snap: &NviSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v49(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("nvi json: {e}"))?;
    conn.execute(
        "INSERT INTO research_nvi(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert nvi: {e}"))?;
    Ok(())
}

pub fn get_nvi(conn: &Connection, symbol: &str) -> Result<Option<NviSnapshot>, String> {
    let _ = create_research_tables_v49(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_nvi WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_nvi: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_nvi: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_nvi: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_pvi(conn: &Connection, symbol: &str, snap: &PviSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v49(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("pvi json: {e}"))?;
    conn.execute(
        "INSERT INTO research_pvi(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert pvi: {e}"))?;
    Ok(())
}

pub fn get_pvi(conn: &Connection, symbol: &str) -> Result<Option<PviSnapshot>, String> {
    let _ = create_research_tables_v49(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_pvi WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_pvi: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_pvi: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_pvi: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_coppock(
    conn: &Connection,
    symbol: &str,
    snap: &CoppockSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v49(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("coppock json: {e}"))?;
    conn.execute(
        "INSERT INTO research_coppock(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert coppock: {e}"))?;
    Ok(())
}

pub fn get_coppock(conn: &Connection, symbol: &str) -> Result<Option<CoppockSnapshot>, String> {
    let _ = create_research_tables_v49(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_coppock WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_coppock: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_coppock: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_coppock: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}
