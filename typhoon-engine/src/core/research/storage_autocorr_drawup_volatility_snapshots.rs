use super::*;

// Autocorrelation, Hurst, hit-rate, asymmetry, and volatility-ratio storage

pub fn create_research_tables_v24(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v23(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_autocor (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_autocor_updated ON research_autocor(updated_at);

        CREATE TABLE IF NOT EXISTS research_hurst (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_hurst_updated ON research_hurst(updated_at);

        CREATE TABLE IF NOT EXISTS research_hitrate (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_hitrate_updated ON research_hitrate(updated_at);

        CREATE TABLE IF NOT EXISTS research_glasym (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_glasym_updated ON research_glasym(updated_at);

        CREATE TABLE IF NOT EXISTS research_volratio (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_volratio_updated ON research_volratio(updated_at);",
    )
    .map_err(|e| format!("create v24 tables: {e}"))?;
    Ok(())
}

pub fn upsert_autocor(
    conn: &Connection,
    symbol: &str,
    snap: &AutocorrelationSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v24(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("autocor json: {e}"))?;
    conn.execute(
        "INSERT INTO research_autocor(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert autocor: {e}"))?;
    Ok(())
}

pub fn get_autocor(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<AutocorrelationSnapshot>, String> {
    let _ = create_research_tables_v24(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_autocor WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_autocor: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_autocor: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_autocor: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_hurst(conn: &Connection, symbol: &str, snap: &HurstSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v24(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("hurst json: {e}"))?;
    conn.execute(
        "INSERT INTO research_hurst(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert hurst: {e}"))?;
    Ok(())
}

pub fn get_hurst(conn: &Connection, symbol: &str) -> Result<Option<HurstSnapshot>, String> {
    let _ = create_research_tables_v24(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_hurst WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_hurst: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_hurst: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_hurst: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_hitrate(
    conn: &Connection,
    symbol: &str,
    snap: &HitRateSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v24(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("hitrate json: {e}"))?;
    conn.execute(
        "INSERT INTO research_hitrate(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert hitrate: {e}"))?;
    Ok(())
}

pub fn get_hitrate(conn: &Connection, symbol: &str) -> Result<Option<HitRateSnapshot>, String> {
    let _ = create_research_tables_v24(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_hitrate WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_hitrate: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_hitrate: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_hitrate: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_glasym(
    conn: &Connection,
    symbol: &str,
    snap: &GainLossAsymmetrySnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v24(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("glasym json: {e}"))?;
    conn.execute(
        "INSERT INTO research_glasym(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert glasym: {e}"))?;
    Ok(())
}

pub fn get_glasym(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<GainLossAsymmetrySnapshot>, String> {
    let _ = create_research_tables_v24(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_glasym WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_glasym: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_glasym: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_glasym: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_volratio(
    conn: &Connection,
    symbol: &str,
    snap: &VolumeRatioSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v24(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("volratio json: {e}"))?;
    conn.execute(
        "INSERT INTO research_volratio(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert volratio: {e}"))?;
    Ok(())
}

pub fn get_volratio(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<VolumeRatioSnapshot>, String> {
    let _ = create_research_tables_v24(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_volratio WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_volratio: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_volratio: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_volratio: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── Round 24 schema + upsert/get ──

pub fn create_research_tables_v25(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v24(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_drawup (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_drawup_updated ON research_drawup(updated_at);

        CREATE TABLE IF NOT EXISTS research_gapstats (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_gapstats_updated ON research_gapstats(updated_at);

        CREATE TABLE IF NOT EXISTS research_volcluster (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_volcluster_updated ON research_volcluster(updated_at);

        CREATE TABLE IF NOT EXISTS research_closeplc (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_closeplc_updated ON research_closeplc(updated_at);

        CREATE TABLE IF NOT EXISTS research_mrhl (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_mrhl_updated ON research_mrhl(updated_at);",
    ).map_err(|e| format!("create v25 tables: {e}"))?;
    Ok(())
}

pub fn upsert_drawup(
    conn: &Connection,
    symbol: &str,
    snap: &DrawupHistorySnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v25(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("drawup json: {e}"))?;
    conn.execute(
        "INSERT INTO research_drawup(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert drawup: {e}"))?;
    Ok(())
}

pub fn get_drawup(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<DrawupHistorySnapshot>, String> {
    let _ = create_research_tables_v25(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_drawup WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_drawup: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_drawup: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_drawup: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_gapstats(
    conn: &Connection,
    symbol: &str,
    snap: &GapStatsSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v25(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("gapstats json: {e}"))?;
    conn.execute(
        "INSERT INTO research_gapstats(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert gapstats: {e}"))?;
    Ok(())
}

pub fn get_gapstats(conn: &Connection, symbol: &str) -> Result<Option<GapStatsSnapshot>, String> {
    let _ = create_research_tables_v25(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_gapstats WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_gapstats: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_gapstats: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_gapstats: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_volcluster(
    conn: &Connection,
    symbol: &str,
    snap: &VolClusterSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v25(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("volcluster json: {e}"))?;
    conn.execute(
        "INSERT INTO research_volcluster(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert volcluster: {e}"))?;
    Ok(())
}

pub fn get_volcluster(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<VolClusterSnapshot>, String> {
    let _ = create_research_tables_v25(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_volcluster WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_volcluster: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_volcluster: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_volcluster: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_closeplc(
    conn: &Connection,
    symbol: &str,
    snap: &ClosePlacementSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v25(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("closeplc json: {e}"))?;
    conn.execute(
        "INSERT INTO research_closeplc(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert closeplc: {e}"))?;
    Ok(())
}

pub fn get_closeplc(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<ClosePlacementSnapshot>, String> {
    let _ = create_research_tables_v25(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_closeplc WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_closeplc: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_closeplc: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_closeplc: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_mrhl(
    conn: &Connection,
    symbol: &str,
    snap: &MeanReversionHalfLifeSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v25(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("mrhl json: {e}"))?;
    conn.execute(
        "INSERT INTO research_mrhl(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert mrhl: {e}"))?;
    Ok(())
}

pub fn get_mrhl(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<MeanReversionHalfLifeSnapshot>, String> {
    let _ = create_research_tables_v25(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_mrhl WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_mrhl: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_mrhl: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_mrhl: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── Round 25 schema + upsert/get ──

pub fn create_research_tables_v26(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v25(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_downvol (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_downvol_updated ON research_downvol(updated_at);

        CREATE TABLE IF NOT EXISTS research_sharpr (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_sharpr_updated ON research_sharpr(updated_at);

        CREATE TABLE IF NOT EXISTS research_effratio (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_effratio_updated ON research_effratio(updated_at);

        CREATE TABLE IF NOT EXISTS research_wickbias (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_wickbias_updated ON research_wickbias(updated_at);

        CREATE TABLE IF NOT EXISTS research_volofvol (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_volofvol_updated ON research_volofvol(updated_at);",
    )
    .map_err(|e| format!("create v26 tables: {e}"))?;
    Ok(())
}

pub fn upsert_downvol(
    conn: &Connection,
    symbol: &str,
    snap: &DownsideVolSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v26(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("downvol json: {e}"))?;
    conn.execute(
        "INSERT INTO research_downvol(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert downvol: {e}"))?;
    Ok(())
}

pub fn get_downvol(conn: &Connection, symbol: &str) -> Result<Option<DownsideVolSnapshot>, String> {
    let _ = create_research_tables_v26(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_downvol WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_downvol: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_downvol: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_downvol: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_sharpr(
    conn: &Connection,
    symbol: &str,
    snap: &SharpeRatioSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v26(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("sharpr json: {e}"))?;
    conn.execute(
        "INSERT INTO research_sharpr(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert sharpr: {e}"))?;
    Ok(())
}

pub fn get_sharpr(conn: &Connection, symbol: &str) -> Result<Option<SharpeRatioSnapshot>, String> {
    let _ = create_research_tables_v26(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_sharpr WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_sharpr: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_sharpr: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_sharpr: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_effratio(
    conn: &Connection,
    symbol: &str,
    snap: &EfficiencyRatioSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v26(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("effratio json: {e}"))?;
    conn.execute(
        "INSERT INTO research_effratio(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert effratio: {e}"))?;
    Ok(())
}

pub fn get_effratio(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<EfficiencyRatioSnapshot>, String> {
    let _ = create_research_tables_v26(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_effratio WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_effratio: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_effratio: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_effratio: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_wickbias(
    conn: &Connection,
    symbol: &str,
    snap: &WickBiasSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v26(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("wickbias json: {e}"))?;
    conn.execute(
        "INSERT INTO research_wickbias(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert wickbias: {e}"))?;
    Ok(())
}

pub fn get_wickbias(conn: &Connection, symbol: &str) -> Result<Option<WickBiasSnapshot>, String> {
    let _ = create_research_tables_v26(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_wickbias WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_wickbias: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_wickbias: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_wickbias: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_volofvol(
    conn: &Connection,
    symbol: &str,
    snap: &VolOfVolSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v26(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("volofvol json: {e}"))?;
    conn.execute(
        "INSERT INTO research_volofvol(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert volofvol: {e}"))?;
    Ok(())
}

pub fn get_volofvol(conn: &Connection, symbol: &str) -> Result<Option<VolOfVolSnapshot>, String> {
    let _ = create_research_tables_v26(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_volofvol WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_volofvol: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_volofvol: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_volofvol: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}
