use super::*;

// ── Round 30: PSR / ADF / MNKENDALL / BIPOWER / DDDUR ──

pub fn create_research_tables_v31(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v30(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_psr (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_psr_updated ON research_psr(updated_at);

        CREATE TABLE IF NOT EXISTS research_adf (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_adf_updated ON research_adf(updated_at);

        CREATE TABLE IF NOT EXISTS research_mnkendall (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_mnkendall_updated ON research_mnkendall(updated_at);

        CREATE TABLE IF NOT EXISTS research_bipower (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_bipower_updated ON research_bipower(updated_at);

        CREATE TABLE IF NOT EXISTS research_dddur (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_dddur_updated ON research_dddur(updated_at);",
    )
    .map_err(|e| format!("create v31 tables: {e}"))?;
    Ok(())
}

pub fn upsert_psr(
    conn: &Connection,
    symbol: &str,
    snap: &ProbabilisticSharpeSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v31(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("psr json: {e}"))?;
    conn.execute(
        "INSERT INTO research_psr(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert psr: {e}"))?;
    Ok(())
}

pub fn get_psr(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<ProbabilisticSharpeSnapshot>, String> {
    let _ = create_research_tables_v31(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_psr WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_psr: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_psr: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_psr: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_adf(
    conn: &Connection,
    symbol: &str,
    snap: &DickeyFullerSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v31(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("adf json: {e}"))?;
    conn.execute(
        "INSERT INTO research_adf(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert adf: {e}"))?;
    Ok(())
}

pub fn get_adf(conn: &Connection, symbol: &str) -> Result<Option<DickeyFullerSnapshot>, String> {
    let _ = create_research_tables_v31(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_adf WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_adf: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_adf: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_adf: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_mnkendall(
    conn: &Connection,
    symbol: &str,
    snap: &MannKendallSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v31(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("mnkendall json: {e}"))?;
    conn.execute(
        "INSERT INTO research_mnkendall(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert mnkendall: {e}"))?;
    Ok(())
}

pub fn get_mnkendall(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<MannKendallSnapshot>, String> {
    let _ = create_research_tables_v31(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_mnkendall WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_mnkendall: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_mnkendall: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_mnkendall: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_bipower(
    conn: &Connection,
    symbol: &str,
    snap: &BipowerVariationSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v31(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("bipower json: {e}"))?;
    conn.execute(
        "INSERT INTO research_bipower(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert bipower: {e}"))?;
    Ok(())
}

pub fn get_bipower(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<BipowerVariationSnapshot>, String> {
    let _ = create_research_tables_v31(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_bipower WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_bipower: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_bipower: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_bipower: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_dddur(
    conn: &Connection,
    symbol: &str,
    snap: &DrawdownDurationSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v31(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("dddur json: {e}"))?;
    conn.execute(
        "INSERT INTO research_dddur(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert dddur: {e}"))?;
    Ok(())
}

pub fn get_dddur(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<DrawdownDurationSnapshot>, String> {
    let _ = create_research_tables_v31(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_dddur WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_dddur: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_dddur: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_dddur: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── Round 31: HILLTAIL / ARCHLM / PAINRATIO / CUSUM / CFVAR ──

pub fn create_research_tables_v32(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v31(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_hilltail (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_hilltail_updated ON research_hilltail(updated_at);

        CREATE TABLE IF NOT EXISTS research_archlm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_archlm_updated ON research_archlm(updated_at);

        CREATE TABLE IF NOT EXISTS research_painratio (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_painratio_updated ON research_painratio(updated_at);

        CREATE TABLE IF NOT EXISTS research_cusum (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cusum_updated ON research_cusum(updated_at);

        CREATE TABLE IF NOT EXISTS research_cfvar (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cfvar_updated ON research_cfvar(updated_at);",
    )
    .map_err(|e| format!("create v32 tables: {e}"))?;
    Ok(())
}

pub fn upsert_hilltail(
    conn: &Connection,
    symbol: &str,
    snap: &HillTailSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v32(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("hilltail json: {e}"))?;
    conn.execute(
        "INSERT INTO research_hilltail(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert hilltail: {e}"))?;
    Ok(())
}

pub fn get_hilltail(conn: &Connection, symbol: &str) -> Result<Option<HillTailSnapshot>, String> {
    let _ = create_research_tables_v32(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_hilltail WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_hilltail: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_hilltail: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_hilltail: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_archlm(conn: &Connection, symbol: &str, snap: &ArchLmSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v32(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("archlm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_archlm(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert archlm: {e}"))?;
    Ok(())
}

pub fn get_archlm(conn: &Connection, symbol: &str) -> Result<Option<ArchLmSnapshot>, String> {
    let _ = create_research_tables_v32(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_archlm WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_archlm: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_archlm: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_archlm: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_painratio(
    conn: &Connection,
    symbol: &str,
    snap: &PainRatioSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v32(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("painratio json: {e}"))?;
    conn.execute(
        "INSERT INTO research_painratio(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert painratio: {e}"))?;
    Ok(())
}

pub fn get_painratio(conn: &Connection, symbol: &str) -> Result<Option<PainRatioSnapshot>, String> {
    let _ = create_research_tables_v32(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_painratio WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_painratio: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_painratio: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_painratio: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_cusum(
    conn: &Connection,
    symbol: &str,
    snap: &CusumBreakSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v32(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cusum json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cusum(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cusum: {e}"))?;
    Ok(())
}

pub fn get_cusum(conn: &Connection, symbol: &str) -> Result<Option<CusumBreakSnapshot>, String> {
    let _ = create_research_tables_v32(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cusum WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_cusum: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_cusum: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_cusum: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_cfvar(
    conn: &Connection,
    symbol: &str,
    snap: &CornishFisherSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v32(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cfvar json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cfvar(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cfvar: {e}"))?;
    Ok(())
}

pub fn get_cfvar(conn: &Connection, symbol: &str) -> Result<Option<CornishFisherSnapshot>, String> {
    let _ = create_research_tables_v32(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cfvar WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_cfvar: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_cfvar: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_cfvar: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── Round 32 schema v33 + upsert/get ─────────────────────────────

pub fn create_research_tables_v33(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v32(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_entropy (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_entropy_updated ON research_entropy(updated_at);

        CREATE TABLE IF NOT EXISTS research_rachev (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_rachev_updated ON research_rachev(updated_at);

        CREATE TABLE IF NOT EXISTS research_gpr (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_gpr_updated ON research_gpr(updated_at);

        CREATE TABLE IF NOT EXISTS research_pacf (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_pacf_updated ON research_pacf(updated_at);

        CREATE TABLE IF NOT EXISTS research_apen (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_apen_updated ON research_apen(updated_at);",
    )
    .map_err(|e| format!("create v33 tables: {e}"))?;
    Ok(())
}

pub fn upsert_entropy(
    conn: &Connection,
    symbol: &str,
    snap: &EntropySnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v33(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("entropy json: {e}"))?;
    conn.execute(
        "INSERT INTO research_entropy(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert entropy: {e}"))?;
    Ok(())
}

pub fn get_entropy(conn: &Connection, symbol: &str) -> Result<Option<EntropySnapshot>, String> {
    let _ = create_research_tables_v33(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_entropy WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_entropy: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_entropy: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_entropy: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_rachev(conn: &Connection, symbol: &str, snap: &RachevSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v33(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("rachev json: {e}"))?;
    conn.execute(
        "INSERT INTO research_rachev(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rachev: {e}"))?;
    Ok(())
}

pub fn get_rachev(conn: &Connection, symbol: &str) -> Result<Option<RachevSnapshot>, String> {
    let _ = create_research_tables_v33(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_rachev WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_rachev: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_rachev: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_rachev: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_gpr(conn: &Connection, symbol: &str, snap: &GprSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v33(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("gpr json: {e}"))?;
    conn.execute(
        "INSERT INTO research_gpr(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert gpr: {e}"))?;
    Ok(())
}

pub fn get_gpr(conn: &Connection, symbol: &str) -> Result<Option<GprSnapshot>, String> {
    let _ = create_research_tables_v33(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_gpr WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_gpr: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_gpr: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_gpr: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_pacf(conn: &Connection, symbol: &str, snap: &PacfSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v33(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("pacf json: {e}"))?;
    conn.execute(
        "INSERT INTO research_pacf(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert pacf: {e}"))?;
    Ok(())
}

pub fn get_pacf(conn: &Connection, symbol: &str) -> Result<Option<PacfSnapshot>, String> {
    let _ = create_research_tables_v33(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_pacf WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_pacf: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_pacf: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_pacf: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_apen(conn: &Connection, symbol: &str, snap: &ApenSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v33(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("apen json: {e}"))?;
    conn.execute(
        "INSERT INTO research_apen(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert apen: {e}"))?;
    Ok(())
}

pub fn get_apen(conn: &Connection, symbol: &str) -> Result<Option<ApenSnapshot>, String> {
    let _ = create_research_tables_v33(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_apen WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_apen: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_apen: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_apen: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── Round 33 schema v34 + upsert/get ─────────────────────────────

pub fn create_research_tables_v34(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v33(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_upr (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_upr_updated ON research_upr(updated_at);

        CREATE TABLE IF NOT EXISTS research_levereff (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_levereff_updated ON research_levereff(updated_at);

        CREATE TABLE IF NOT EXISTS research_drawdar (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_drawdar_updated ON research_drawdar(updated_at);

        CREATE TABLE IF NOT EXISTS research_varhalf (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_varhalf_updated ON research_varhalf(updated_at);

        CREATE TABLE IF NOT EXISTS research_gini (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_gini_updated ON research_gini(updated_at);",
    )
    .map_err(|e| format!("create v34 tables: {e}"))?;
    Ok(())
}

pub fn upsert_upr(conn: &Connection, symbol: &str, snap: &UprSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v34(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("upr json: {e}"))?;
    conn.execute(
        "INSERT INTO research_upr(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert upr: {e}"))?;
    Ok(())
}

pub fn get_upr(conn: &Connection, symbol: &str) -> Result<Option<UprSnapshot>, String> {
    let _ = create_research_tables_v34(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_upr WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_upr: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_upr: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_upr: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_levereff(
    conn: &Connection,
    symbol: &str,
    snap: &LeverEffSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v34(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("levereff json: {e}"))?;
    conn.execute(
        "INSERT INTO research_levereff(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert levereff: {e}"))?;
    Ok(())
}

pub fn get_levereff(conn: &Connection, symbol: &str) -> Result<Option<LeverEffSnapshot>, String> {
    let _ = create_research_tables_v34(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_levereff WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_levereff: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_levereff: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_levereff: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_drawdar(
    conn: &Connection,
    symbol: &str,
    snap: &DrawDaRSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v34(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("drawdar json: {e}"))?;
    conn.execute(
        "INSERT INTO research_drawdar(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert drawdar: {e}"))?;
    Ok(())
}

pub fn get_drawdar(conn: &Connection, symbol: &str) -> Result<Option<DrawDaRSnapshot>, String> {
    let _ = create_research_tables_v34(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_drawdar WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_drawdar: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_drawdar: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_drawdar: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_varhalf(
    conn: &Connection,
    symbol: &str,
    snap: &VarHalfSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v34(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("varhalf json: {e}"))?;
    conn.execute(
        "INSERT INTO research_varhalf(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert varhalf: {e}"))?;
    Ok(())
}

pub fn get_varhalf(conn: &Connection, symbol: &str) -> Result<Option<VarHalfSnapshot>, String> {
    let _ = create_research_tables_v34(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_varhalf WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_varhalf: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_varhalf: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_varhalf: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_gini(conn: &Connection, symbol: &str, snap: &GiniSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v34(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("gini json: {e}"))?;
    conn.execute(
        "INSERT INTO research_gini(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert gini: {e}"))?;
    Ok(())
}

pub fn get_gini(conn: &Connection, symbol: &str) -> Result<Option<GiniSnapshot>, String> {
    let _ = create_research_tables_v34(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_gini WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_gini: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_gini: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_gini: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── Round 34 schema v35 ──

pub fn create_research_tables_v35(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v34(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_sampen (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_sampen_updated ON research_sampen(updated_at);

        CREATE TABLE IF NOT EXISTS research_permen (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_permen_updated ON research_permen(updated_at);

        CREATE TABLE IF NOT EXISTS research_recfact (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_recfact_updated ON research_recfact(updated_at);

        CREATE TABLE IF NOT EXISTS research_kpss (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_kpss_updated ON research_kpss(updated_at);

        CREATE TABLE IF NOT EXISTS research_specent (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_specent_updated ON research_specent(updated_at);",
    )
    .map_err(|e| format!("create v35 tables: {e}"))?;
    Ok(())
}

pub fn upsert_sampen(conn: &Connection, symbol: &str, snap: &SampenSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v35(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("sampen json: {e}"))?;
    conn.execute(
        "INSERT INTO research_sampen(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert sampen: {e}"))?;
    Ok(())
}

pub fn get_sampen(conn: &Connection, symbol: &str) -> Result<Option<SampenSnapshot>, String> {
    let _ = create_research_tables_v35(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_sampen WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_sampen: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_sampen: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_sampen: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_permen(conn: &Connection, symbol: &str, snap: &PermenSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v35(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("permen json: {e}"))?;
    conn.execute(
        "INSERT INTO research_permen(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert permen: {e}"))?;
    Ok(())
}

pub fn get_permen(conn: &Connection, symbol: &str) -> Result<Option<PermenSnapshot>, String> {
    let _ = create_research_tables_v35(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_permen WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_permen: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_permen: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_permen: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_recfact(
    conn: &Connection,
    symbol: &str,
    snap: &RecfactSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v35(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("recfact json: {e}"))?;
    conn.execute(
        "INSERT INTO research_recfact(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert recfact: {e}"))?;
    Ok(())
}

pub fn get_recfact(conn: &Connection, symbol: &str) -> Result<Option<RecfactSnapshot>, String> {
    let _ = create_research_tables_v35(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_recfact WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_recfact: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_recfact: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_recfact: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_kpss(conn: &Connection, symbol: &str, snap: &KpssSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v35(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("kpss json: {e}"))?;
    conn.execute(
        "INSERT INTO research_kpss(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert kpss: {e}"))?;
    Ok(())
}

pub fn get_kpss(conn: &Connection, symbol: &str) -> Result<Option<KpssSnapshot>, String> {
    let _ = create_research_tables_v35(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_kpss WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_kpss: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_kpss: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_kpss: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_specent(
    conn: &Connection,
    symbol: &str,
    snap: &SpecentSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v35(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("specent json: {e}"))?;
    conn.execute(
        "INSERT INTO research_specent(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert specent: {e}"))?;
    Ok(())
}

pub fn get_specent(conn: &Connection, symbol: &str) -> Result<Option<SpecentSnapshot>, String> {
    let _ = create_research_tables_v35(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_specent WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_specent: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_specent: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_specent: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── Round 35 schema v36 ──

pub fn create_research_tables_v36(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v35(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_robvol (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_robvol_updated ON research_robvol(updated_at);

        CREATE TABLE IF NOT EXISTS research_renyient (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_renyient_updated ON research_renyient(updated_at);

        CREATE TABLE IF NOT EXISTS research_retquant (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_retquant_updated ON research_retquant(updated_at);

        CREATE TABLE IF NOT EXISTS research_msent (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_msent_updated ON research_msent(updated_at);

        CREATE TABLE IF NOT EXISTS research_ewmavol (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_ewmavol_updated ON research_ewmavol(updated_at);",
    )
    .map_err(|e| format!("create v36 tables: {e}"))?;
    Ok(())
}

pub fn upsert_robvol(conn: &Connection, symbol: &str, snap: &RobVolSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v36(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("robvol json: {e}"))?;
    conn.execute(
        "INSERT INTO research_robvol(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert robvol: {e}"))?;
    Ok(())
}

pub fn get_robvol(conn: &Connection, symbol: &str) -> Result<Option<RobVolSnapshot>, String> {
    let _ = create_research_tables_v36(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_robvol WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_robvol: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_robvol: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_robvol: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_renyient(
    conn: &Connection,
    symbol: &str,
    snap: &RenyientSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v36(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("renyient json: {e}"))?;
    conn.execute(
        "INSERT INTO research_renyient(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert renyient: {e}"))?;
    Ok(())
}

pub fn get_renyient(conn: &Connection, symbol: &str) -> Result<Option<RenyientSnapshot>, String> {
    let _ = create_research_tables_v36(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_renyient WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_renyient: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_renyient: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_renyient: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_retquant(
    conn: &Connection,
    symbol: &str,
    snap: &RetquantSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v36(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("retquant json: {e}"))?;
    conn.execute(
        "INSERT INTO research_retquant(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert retquant: {e}"))?;
    Ok(())
}

pub fn get_retquant(conn: &Connection, symbol: &str) -> Result<Option<RetquantSnapshot>, String> {
    let _ = create_research_tables_v36(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_retquant WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_retquant: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_retquant: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_retquant: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_msent(conn: &Connection, symbol: &str, snap: &MsentSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v36(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("msent json: {e}"))?;
    conn.execute(
        "INSERT INTO research_msent(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert msent: {e}"))?;
    Ok(())
}

pub fn get_msent(conn: &Connection, symbol: &str) -> Result<Option<MsentSnapshot>, String> {
    let _ = create_research_tables_v36(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_msent WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_msent: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_msent: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_msent: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_ewmavol(
    conn: &Connection,
    symbol: &str,
    snap: &EwmaVolSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v36(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ewmavol json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ewmavol(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ewmavol: {e}"))?;
    Ok(())
}

pub fn get_ewmavol(conn: &Connection, symbol: &str) -> Result<Option<EwmaVolSnapshot>, String> {
    let _ = create_research_tables_v36(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_ewmavol WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ewmavol: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_ewmavol: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ewmavol: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── Round 36 schema v37 ──

pub fn create_research_tables_v37(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v36(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_ksnorm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_ksnorm_updated ON research_ksnorm(updated_at);

        CREATE TABLE IF NOT EXISTS research_adtest (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_adtest_updated ON research_adtest(updated_at);

        CREATE TABLE IF NOT EXISTS research_lmom (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_lmom_updated ON research_lmom(updated_at);

        CREATE TABLE IF NOT EXISTS research_kylelam (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_kylelam_updated ON research_kylelam(updated_at);

        CREATE TABLE IF NOT EXISTS research_peakover (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_peakover_updated ON research_peakover(updated_at);",
    )
    .map_err(|e| format!("create v37 tables: {e}"))?;
    Ok(())
}

pub fn upsert_ksnorm(conn: &Connection, symbol: &str, snap: &KsnormSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v37(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ksnorm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ksnorm(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ksnorm: {e}"))?;
    Ok(())
}

pub fn get_ksnorm(conn: &Connection, symbol: &str) -> Result<Option<KsnormSnapshot>, String> {
    let _ = create_research_tables_v37(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_ksnorm WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ksnorm: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_ksnorm: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ksnorm: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_adtest(conn: &Connection, symbol: &str, snap: &AdtestSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v37(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("adtest json: {e}"))?;
    conn.execute(
        "INSERT INTO research_adtest(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert adtest: {e}"))?;
    Ok(())
}

pub fn get_adtest(conn: &Connection, symbol: &str) -> Result<Option<AdtestSnapshot>, String> {
    let _ = create_research_tables_v37(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_adtest WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_adtest: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_adtest: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_adtest: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_lmom(conn: &Connection, symbol: &str, snap: &LmomSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v37(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("lmom json: {e}"))?;
    conn.execute(
        "INSERT INTO research_lmom(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert lmom: {e}"))?;
    Ok(())
}

pub fn get_lmom(conn: &Connection, symbol: &str) -> Result<Option<LmomSnapshot>, String> {
    let _ = create_research_tables_v37(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_lmom WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_lmom: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_lmom: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_lmom: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_kylelam(
    conn: &Connection,
    symbol: &str,
    snap: &KylelamSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v37(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("kylelam json: {e}"))?;
    conn.execute(
        "INSERT INTO research_kylelam(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert kylelam: {e}"))?;
    Ok(())
}

pub fn get_kylelam(conn: &Connection, symbol: &str) -> Result<Option<KylelamSnapshot>, String> {
    let _ = create_research_tables_v37(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_kylelam WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_kylelam: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_kylelam: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_kylelam: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_peakover(
    conn: &Connection,
    symbol: &str,
    snap: &PeakoverSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v37(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("peakover json: {e}"))?;
    conn.execute(
        "INSERT INTO research_peakover(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert peakover: {e}"))?;
    Ok(())
}

pub fn get_peakover(conn: &Connection, symbol: &str) -> Result<Option<PeakoverSnapshot>, String> {
    let _ = create_research_tables_v37(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_peakover WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_peakover: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_peakover: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_peakover: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}
