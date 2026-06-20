use super::*;

// ── + wrappers ──

pub fn create_research_tables_v17(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v16(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_sizef (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_sizef_updated ON research_sizef(updated_at);

        CREATE TABLE IF NOT EXISTS research_momf (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_momf_updated ON research_momf(updated_at);

        CREATE TABLE IF NOT EXISTS research_peadrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_peadrank_updated ON research_peadrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_fqm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_fqm_updated ON research_fqm(updated_at);

        CREATE TABLE IF NOT EXISTS research_revrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_revrank_updated ON research_revrank(updated_at);",
    )
    .map_err(|e| format!("create v17 tables: {e}"))?;
    Ok(())
}

pub fn upsert_sizef(
    conn: &Connection,
    symbol: &str,
    snap: &SizeFactorSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v17(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("sizef json: {e}"))?;
    conn.execute(
        "INSERT INTO research_sizef(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert sizef: {e}"))?;
    Ok(())
}

pub fn get_sizef(conn: &Connection, symbol: &str) -> Result<Option<SizeFactorSnapshot>, String> {
    let _ = create_research_tables_v17(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_sizef WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_sizef: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_sizef: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_sizef: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_momf(
    conn: &Connection,
    symbol: &str,
    snap: &MomentumRankSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v17(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("momf json: {e}"))?;
    conn.execute(
        "INSERT INTO research_momf(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert momf: {e}"))?;
    Ok(())
}

pub fn get_momf(conn: &Connection, symbol: &str) -> Result<Option<MomentumRankSnapshot>, String> {
    let _ = create_research_tables_v17(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_momf WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_momf: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_momf: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_momf: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_peadrank(
    conn: &Connection,
    symbol: &str,
    snap: &PeadRankSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v17(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("peadrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_peadrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert peadrank: {e}"))?;
    Ok(())
}

pub fn get_peadrank(conn: &Connection, symbol: &str) -> Result<Option<PeadRankSnapshot>, String> {
    let _ = create_research_tables_v17(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_peadrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_peadrank: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_peadrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_peadrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_fqm(
    conn: &Connection,
    symbol: &str,
    snap: &FundamentalQualityMeterSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v17(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("fqm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_fqm(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert fqm: {e}"))?;
    Ok(())
}

pub fn get_fqm(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<FundamentalQualityMeterSnapshot>, String> {
    let _ = create_research_tables_v17(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_fqm WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_fqm: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_fqm: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_fqm: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_revrank(
    conn: &Connection,
    symbol: &str,
    snap: &RevenueGrowthRankSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v17(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("revrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_revrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert revrank: {e}"))?;
    Ok(())
}

pub fn get_revrank(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<RevenueGrowthRankSnapshot>, String> {
    let _ = create_research_tables_v17(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_revrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_revrank: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_revrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_revrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

/// Whole-table scan of `research_momentum`. Used by MOMF.
pub fn get_all_momentum(conn: &Connection) -> Result<Vec<MomentumSnapshot>, String> {
    let _ = create_research_tables_v13(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_momentum")
        .map_err(|e| format!("prepare get_all_momentum: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_momentum: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<MomentumSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_pead`. Used by PEADRANK.
pub fn get_all_pead(conn: &Connection) -> Result<Vec<PeadSnapshot>, String> {
    let _ = create_research_tables_v16(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_pead")
        .map_err(|e| format!("prepare get_all_pead: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_pead: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<PeadSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

// ── + wrappers ──

pub fn create_research_tables_v18(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v17(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_levrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_levrank_updated ON research_levrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_operank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_operank_updated ON research_operank(updated_at);

        CREATE TABLE IF NOT EXISTS research_fqmrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_fqmrank_updated ON research_fqmrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_liqrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_liqrank_updated ON research_liqrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_surpstk (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_surpstk_updated ON research_surpstk(updated_at);",
    )
    .map_err(|e| format!("create v18 tables: {e}"))?;
    Ok(())
}

pub fn upsert_levrank(
    conn: &Connection,
    symbol: &str,
    snap: &LeverageRankSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v18(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("levrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_levrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert levrank: {e}"))?;
    Ok(())
}

pub fn get_levrank(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<LeverageRankSnapshot>, String> {
    let _ = create_research_tables_v18(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_levrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_levrank: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_levrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_levrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_operank(
    conn: &Connection,
    symbol: &str,
    snap: &OperatingQualityRankSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v18(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("operank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_operank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert operank: {e}"))?;
    Ok(())
}

pub fn get_operank(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<OperatingQualityRankSnapshot>, String> {
    let _ = create_research_tables_v18(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_operank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_operank: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_operank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_operank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_fqmrank(
    conn: &Connection,
    symbol: &str,
    snap: &FqmRankSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v18(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("fqmrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_fqmrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert fqmrank: {e}"))?;
    Ok(())
}

pub fn get_fqmrank(conn: &Connection, symbol: &str) -> Result<Option<FqmRankSnapshot>, String> {
    let _ = create_research_tables_v18(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_fqmrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_fqmrank: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_fqmrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_fqmrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_liqrank(
    conn: &Connection,
    symbol: &str,
    snap: &LiquidityRankSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v18(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("liqrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_liqrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert liqrank: {e}"))?;
    Ok(())
}

pub fn get_liqrank(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<LiquidityRankSnapshot>, String> {
    let _ = create_research_tables_v18(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_liqrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_liqrank: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_liqrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_liqrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_surpstk(
    conn: &Connection,
    symbol: &str,
    snap: &EarningsSurpriseStreakSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v18(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("surpstk json: {e}"))?;
    conn.execute(
        "INSERT INTO research_surpstk(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert surpstk: {e}"))?;
    Ok(())
}

pub fn get_surpstk(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<EarningsSurpriseStreakSnapshot>, String> {
    let _ = create_research_tables_v18(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_surpstk WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_surpstk: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_surpstk: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_surpstk: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

/// Whole-table scan of `research_leverage`. Used by LEVRANK.
pub fn get_all_leverage(conn: &Connection) -> Result<Vec<LeverageSnapshot>, String> {
    let _ = create_research_tables_v10(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_leverage")
        .map_err(|e| format!("prepare get_all_leverage: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_leverage: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<LeverageSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_margins`. Used by OPERANK.
pub fn get_all_margins(conn: &Connection) -> Result<Vec<MarginsSnapshot>, String> {
    let _ = create_research_tables_v14(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_margins")
        .map_err(|e| format!("prepare get_all_margins: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_margins: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<MarginsSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_fqm`. Used by FQMRANK.
pub fn get_all_fqm(conn: &Connection) -> Result<Vec<FundamentalQualityMeterSnapshot>, String> {
    let _ = create_research_tables_v17(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_fqm")
        .map_err(|e| format!("prepare get_all_fqm: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_fqm: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<FundamentalQualityMeterSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_liquidity`. Used by LIQRANK.
pub fn get_all_liquidity(conn: &Connection) -> Result<Vec<LiquiditySnapshot>, String> {
    let _ = create_research_tables_v13(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_liquidity")
        .map_err(|e| format!("prepare get_all_liquidity: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_liquidity: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<LiquiditySnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

// ── + wrappers ──

pub fn create_research_tables_v19(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v18(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_dvdrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_dvdrank_updated ON research_dvdrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_earmrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_earmrank_updated ON research_earmrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_updgrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_updgrank_updated ON research_updgrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_gy (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_gy_updated ON research_gy(updated_at);

        CREATE TABLE IF NOT EXISTS research_des (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_des_updated ON research_des(updated_at);",
    )
    .map_err(|e| format!("create v19 tables: {e}"))?;
    Ok(())
}

pub fn upsert_dvdrank(
    conn: &Connection,
    symbol: &str,
    snap: &DividendGrowthRankSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v19(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("dvdrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_dvdrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert dvdrank: {e}"))?;
    Ok(())
}

pub fn get_dvdrank(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<DividendGrowthRankSnapshot>, String> {
    let _ = create_research_tables_v19(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_dvdrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_dvdrank: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_dvdrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_dvdrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_earmrank(
    conn: &Connection,
    symbol: &str,
    snap: &EarningsMomentumRankSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v19(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("earmrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_earmrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert earmrank: {e}"))?;
    Ok(())
}

pub fn get_earmrank(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<EarningsMomentumRankSnapshot>, String> {
    let _ = create_research_tables_v19(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_earmrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_earmrank: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_earmrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_earmrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_updgrank(
    conn: &Connection,
    symbol: &str,
    snap: &UpgradeDowngradeRankSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v19(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("updgrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_updgrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert updgrank: {e}"))?;
    Ok(())
}

pub fn get_updgrank(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<UpgradeDowngradeRankSnapshot>, String> {
    let _ = create_research_tables_v19(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_updgrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_updgrank: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_updgrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_updgrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_gy(conn: &Connection, symbol: &str, snap: &GapYearlySnapshot) -> Result<(), String> {
    let _ = create_research_tables_v19(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("gy json: {e}"))?;
    conn.execute(
        "INSERT INTO research_gy(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert gy: {e}"))?;
    Ok(())
}

pub fn get_gy(conn: &Connection, symbol: &str) -> Result<Option<GapYearlySnapshot>, String> {
    let _ = create_research_tables_v19(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_gy WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_gy: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_gy: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_gy: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_des(
    conn: &Connection,
    symbol: &str,
    snap: &DailyEventStreakSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v19(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("des json: {e}"))?;
    conn.execute(
        "INSERT INTO research_des(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert des: {e}"))?;
    Ok(())
}

pub fn get_des(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<DailyEventStreakSnapshot>, String> {
    let _ = create_research_tables_v19(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_des WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_des: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_des: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_des: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}
