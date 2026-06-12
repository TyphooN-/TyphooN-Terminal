use super::*;

pub fn create_research_tables_v61(conn: &Connection) -> Result<(), String> {
    create_research_tables_v60(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_demarker (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_demarker_updated ON research_demarker(updated_at);

        CREATE TABLE IF NOT EXISTS research_gator (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_gator_updated ON research_gator(updated_at);

        CREATE TABLE IF NOT EXISTS research_bw_mfi (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_bw_mfi_updated ON research_bw_mfi(updated_at);

        CREATE TABLE IF NOT EXISTS research_vwma (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_vwma_updated ON research_vwma(updated_at);

        CREATE TABLE IF NOT EXISTS research_stddev (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_stddev_updated ON research_stddev(updated_at);",
    )
    .map_err(|e| format!("create v61 tables: {e}"))?;
    Ok(())
}

pub fn create_research_tables_v62(conn: &Connection) -> Result<(), String> {
    create_research_tables_v61(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_wma (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_wma_updated ON research_wma(updated_at);

        CREATE TABLE IF NOT EXISTS research_rainbow (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_rainbow_updated ON research_rainbow(updated_at);

        CREATE TABLE IF NOT EXISTS research_mesa_sine (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_mesa_sine_updated ON research_mesa_sine(updated_at);

        CREATE TABLE IF NOT EXISTS research_frama (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_frama_updated ON research_frama(updated_at);

        CREATE TABLE IF NOT EXISTS research_ibs (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_ibs_updated ON research_ibs(updated_at);",
    )
    .map_err(|e| format!("create v62 tables: {e}"))?;
    Ok(())
}

pub fn create_research_tables_v63(conn: &Connection) -> Result<(), String> {
    create_research_tables_v62(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_laguerre_rsi (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_laguerre_rsi_updated ON research_laguerre_rsi(updated_at);

        CREATE TABLE IF NOT EXISTS research_zigzag (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_zigzag_updated ON research_zigzag(updated_at);

        CREATE TABLE IF NOT EXISTS research_pgo (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_pgo_updated ON research_pgo(updated_at);

        CREATE TABLE IF NOT EXISTS research_ht_trendline (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_ht_trendline_updated ON research_ht_trendline(updated_at);

        CREATE TABLE IF NOT EXISTS research_midpoint (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_midpoint_updated ON research_midpoint(updated_at);",
    ).map_err(|e| format!("create v63 tables: {e}"))?;
    Ok(())
}

pub fn create_research_tables_v64(conn: &Connection) -> Result<(), String> {
    create_research_tables_v63(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_mass_index (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_mass_index_updated ON research_mass_index(updated_at);

        CREATE TABLE IF NOT EXISTS research_natr (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_natr_updated ON research_natr(updated_at);

        CREATE TABLE IF NOT EXISTS research_ttm_squeeze (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_ttm_squeeze_updated ON research_ttm_squeeze(updated_at);

        CREATE TABLE IF NOT EXISTS research_force_index (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_force_index_updated ON research_force_index(updated_at);

        CREATE TABLE IF NOT EXISTS research_trange (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_trange_updated ON research_trange(updated_at);",
    ).map_err(|e| format!("create v64 tables: {e}"))?;
    Ok(())
}

pub fn create_research_tables_v65(conn: &Connection) -> Result<(), String> {
    create_research_tables_v64(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_linearreg_slope (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_linearreg_slope_updated ON research_linearreg_slope(updated_at);

        CREATE TABLE IF NOT EXISTS research_ht_dcperiod (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_ht_dcperiod_updated ON research_ht_dcperiod(updated_at);

        CREATE TABLE IF NOT EXISTS research_ht_trendmode (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_ht_trendmode_updated ON research_ht_trendmode(updated_at);

        CREATE TABLE IF NOT EXISTS research_accbands (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_accbands_updated ON research_accbands(updated_at);

        CREATE TABLE IF NOT EXISTS research_stochf (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_stochf_updated ON research_stochf(updated_at);",
    ).map_err(|e| format!("create v65 tables: {e}"))?;
    Ok(())
}

pub fn upsert_linearreg_slope(
    conn: &Connection,
    symbol: &str,
    snap: &LinearregSlopeSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v65(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("linearreg_slope json: {e}"))?;
    conn.execute(
        "INSERT INTO research_linearreg_slope (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert linearreg_slope: {e}"))?;
    Ok(())
}

pub fn get_linearreg_slope(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<LinearregSlopeSnapshot>, String> {
    let _ = create_research_tables_v65(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_linearreg_slope WHERE symbol = ?1")
        .map_err(|e| format!("prep linearreg_slope: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query linearreg_slope: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row linearreg_slope: {e}"))?
    {
        let j: String = r.get(0).map_err(|e| format!("get linearreg_slope: {e}"))?;
        let snap: LinearregSlopeSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse linearreg_slope: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_ht_dcperiod(
    conn: &Connection,
    symbol: &str,
    snap: &HtDcperiodSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v65(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ht_dcperiod json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ht_dcperiod (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ht_dcperiod: {e}"))?;
    Ok(())
}

pub fn get_ht_dcperiod(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<HtDcperiodSnapshot>, String> {
    let _ = create_research_tables_v65(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_ht_dcperiod WHERE symbol = ?1")
        .map_err(|e| format!("prep ht_dcperiod: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query ht_dcperiod: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row ht_dcperiod: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get ht_dcperiod: {e}"))?;
        let snap: HtDcperiodSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse ht_dcperiod: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_ht_trendmode(
    conn: &Connection,
    symbol: &str,
    snap: &HtTrendmodeSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v65(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ht_trendmode json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ht_trendmode (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ht_trendmode: {e}"))?;
    Ok(())
}

pub fn get_ht_trendmode(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<HtTrendmodeSnapshot>, String> {
    let _ = create_research_tables_v65(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_ht_trendmode WHERE symbol = ?1")
        .map_err(|e| format!("prep ht_trendmode: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query ht_trendmode: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row ht_trendmode: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get ht_trendmode: {e}"))?;
        let snap: HtTrendmodeSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse ht_trendmode: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_accbands(
    conn: &Connection,
    symbol: &str,
    snap: &AccbandsSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v65(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("accbands json: {e}"))?;
    conn.execute(
        "INSERT INTO research_accbands (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert accbands: {e}"))?;
    Ok(())
}

pub fn get_accbands(conn: &Connection, symbol: &str) -> Result<Option<AccbandsSnapshot>, String> {
    let _ = create_research_tables_v65(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_accbands WHERE symbol = ?1")
        .map_err(|e| format!("prep accbands: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query accbands: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row accbands: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get accbands: {e}"))?;
        let snap: AccbandsSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse accbands: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_stochf(conn: &Connection, symbol: &str, snap: &StochfSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v65(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("stochf json: {e}"))?;
    conn.execute(
        "INSERT INTO research_stochf (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert stochf: {e}"))?;
    Ok(())
}

pub fn get_stochf(conn: &Connection, symbol: &str) -> Result<Option<StochfSnapshot>, String> {
    let _ = create_research_tables_v65(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_stochf WHERE symbol = ?1")
        .map_err(|e| format!("prep stochf: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query stochf: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row stochf: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get stochf: {e}"))?;
        let snap: StochfSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse stochf: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

// ── Round 64 v66 schema + helpers ──────────────────────────────────

pub fn create_research_tables_v66(conn: &Connection) -> Result<(), String> {
    create_research_tables_v65(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_linearreg (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_linearreg_updated ON research_linearreg(updated_at);

        CREATE TABLE IF NOT EXISTS research_linearreg_angle (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_linearreg_angle_updated ON research_linearreg_angle(updated_at);

        CREATE TABLE IF NOT EXISTS research_ht_dcphase (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_ht_dcphase_updated ON research_ht_dcphase(updated_at);

        CREATE TABLE IF NOT EXISTS research_ht_sine (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_ht_sine_updated ON research_ht_sine(updated_at);

        CREATE TABLE IF NOT EXISTS research_ht_phasor (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_ht_phasor_updated ON research_ht_phasor(updated_at);",
    ).map_err(|e| format!("create v66 tables: {e}"))?;
    Ok(())
}

pub fn upsert_linearreg(
    conn: &Connection,
    symbol: &str,
    snap: &LinearregSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v66(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("linearreg json: {e}"))?;
    conn.execute(
        "INSERT INTO research_linearreg (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert linearreg: {e}"))?;
    Ok(())
}

pub fn get_linearreg(conn: &Connection, symbol: &str) -> Result<Option<LinearregSnapshot>, String> {
    let _ = create_research_tables_v66(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_linearreg WHERE symbol = ?1")
        .map_err(|e| format!("prep linearreg: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query linearreg: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row linearreg: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get linearreg: {e}"))?;
        let snap: LinearregSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse linearreg: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_linearreg_angle(
    conn: &Connection,
    symbol: &str,
    snap: &LinearregAngleSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v66(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("linearreg_angle json: {e}"))?;
    conn.execute(
        "INSERT INTO research_linearreg_angle (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert linearreg_angle: {e}"))?;
    Ok(())
}

pub fn get_linearreg_angle(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<LinearregAngleSnapshot>, String> {
    let _ = create_research_tables_v66(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_linearreg_angle WHERE symbol = ?1")
        .map_err(|e| format!("prep linearreg_angle: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query linearreg_angle: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row linearreg_angle: {e}"))?
    {
        let j: String = r.get(0).map_err(|e| format!("get linearreg_angle: {e}"))?;
        let snap: LinearregAngleSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse linearreg_angle: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_ht_dcphase(
    conn: &Connection,
    symbol: &str,
    snap: &HtDcphaseSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v66(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ht_dcphase json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ht_dcphase (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ht_dcphase: {e}"))?;
    Ok(())
}

pub fn get_ht_dcphase(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<HtDcphaseSnapshot>, String> {
    let _ = create_research_tables_v66(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_ht_dcphase WHERE symbol = ?1")
        .map_err(|e| format!("prep ht_dcphase: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query ht_dcphase: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row ht_dcphase: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get ht_dcphase: {e}"))?;
        let snap: HtDcphaseSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse ht_dcphase: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_ht_sine(
    conn: &Connection,
    symbol: &str,
    snap: &HtSineSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v66(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ht_sine json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ht_sine (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ht_sine: {e}"))?;
    Ok(())
}

pub fn get_ht_sine(conn: &Connection, symbol: &str) -> Result<Option<HtSineSnapshot>, String> {
    let _ = create_research_tables_v66(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_ht_sine WHERE symbol = ?1")
        .map_err(|e| format!("prep ht_sine: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query ht_sine: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row ht_sine: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get ht_sine: {e}"))?;
        let snap: HtSineSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse ht_sine: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_ht_phasor(
    conn: &Connection,
    symbol: &str,
    snap: &HtPhasorSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v66(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ht_phasor json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ht_phasor (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ht_phasor: {e}"))?;
    Ok(())
}

pub fn get_ht_phasor(conn: &Connection, symbol: &str) -> Result<Option<HtPhasorSnapshot>, String> {
    let _ = create_research_tables_v66(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_ht_phasor WHERE symbol = ?1")
        .map_err(|e| format!("prep ht_phasor: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query ht_phasor: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row ht_phasor: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get ht_phasor: {e}"))?;
        let snap: HtPhasorSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse ht_phasor: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

// ── Round 65 schema (v67) ──────────────────────────────────────────
pub fn create_research_tables_v67(conn: &Connection) -> Result<(), String> {
    create_research_tables_v66(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_midprice (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_midprice_updated ON research_midprice(updated_at);

        CREATE TABLE IF NOT EXISTS research_apo (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_apo_updated ON research_apo(updated_at);

        CREATE TABLE IF NOT EXISTS research_mom (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_mom_updated ON research_mom(updated_at);

        CREATE TABLE IF NOT EXISTS research_sarext (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_sarext_updated ON research_sarext(updated_at);

        CREATE TABLE IF NOT EXISTS research_adxr (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_adxr_updated ON research_adxr(updated_at);",
    )
    .map_err(|e| format!("create v67 tables: {e}"))?;
    Ok(())
}

pub fn upsert_midprice(
    conn: &Connection,
    symbol: &str,
    snap: &MidpriceSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v67(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("midprice json: {e}"))?;
    conn.execute(
        "INSERT INTO research_midprice (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert midprice: {e}"))?;
    Ok(())
}

pub fn get_midprice(conn: &Connection, symbol: &str) -> Result<Option<MidpriceSnapshot>, String> {
    let _ = create_research_tables_v67(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_midprice WHERE symbol = ?1")
        .map_err(|e| format!("prep midprice: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query midprice: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row midprice: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get midprice: {e}"))?;
        let snap: MidpriceSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse midprice: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_apo(conn: &Connection, symbol: &str, snap: &ApoSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v67(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("apo json: {e}"))?;
    conn.execute(
        "INSERT INTO research_apo (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert apo: {e}"))?;
    Ok(())
}

pub fn get_apo(conn: &Connection, symbol: &str) -> Result<Option<ApoSnapshot>, String> {
    let _ = create_research_tables_v67(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_apo WHERE symbol = ?1")
        .map_err(|e| format!("prep apo: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query apo: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row apo: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get apo: {e}"))?;
        let snap: ApoSnapshot = serde_json::from_str(&j).map_err(|e| format!("parse apo: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_mom(conn: &Connection, symbol: &str, snap: &MomSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v67(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("mom json: {e}"))?;
    conn.execute(
        "INSERT INTO research_mom (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert mom: {e}"))?;
    Ok(())
}

pub fn get_mom(conn: &Connection, symbol: &str) -> Result<Option<MomSnapshot>, String> {
    let _ = create_research_tables_v67(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_mom WHERE symbol = ?1")
        .map_err(|e| format!("prep mom: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query mom: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row mom: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get mom: {e}"))?;
        let snap: MomSnapshot = serde_json::from_str(&j).map_err(|e| format!("parse mom: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_sarext(conn: &Connection, symbol: &str, snap: &SarextSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v67(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("sarext json: {e}"))?;
    conn.execute(
        "INSERT INTO research_sarext (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert sarext: {e}"))?;
    Ok(())
}

pub fn get_sarext(conn: &Connection, symbol: &str) -> Result<Option<SarextSnapshot>, String> {
    let _ = create_research_tables_v67(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_sarext WHERE symbol = ?1")
        .map_err(|e| format!("prep sarext: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query sarext: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row sarext: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get sarext: {e}"))?;
        let snap: SarextSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse sarext: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_adxr(conn: &Connection, symbol: &str, snap: &AdxrSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v67(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("adxr json: {e}"))?;
    conn.execute(
        "INSERT INTO research_adxr (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert adxr: {e}"))?;
    Ok(())
}

pub fn get_adxr(conn: &Connection, symbol: &str) -> Result<Option<AdxrSnapshot>, String> {
    let _ = create_research_tables_v67(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_adxr WHERE symbol = ?1")
        .map_err(|e| format!("prep adxr: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query adxr: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row adxr: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get adxr: {e}"))?;
        let snap: AdxrSnapshot = serde_json::from_str(&j).map_err(|e| {
            format!(
                "// ── Round 66-71 storage helpers moved to price_momentum_indicator_storage.rs ──

parse mavp: {e}"
            )
        })?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}
