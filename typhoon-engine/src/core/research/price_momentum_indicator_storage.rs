use super::*;

// ── Research section ──
pub fn create_research_tables_v68(conn: &Connection) -> Result<(), String> {
    create_research_tables_v67(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_avgprice (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_avgprice_updated ON research_avgprice(updated_at);

        CREATE TABLE IF NOT EXISTS research_medprice (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_medprice_updated ON research_medprice(updated_at);

        CREATE TABLE IF NOT EXISTS research_typprice (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_typprice_updated ON research_typprice(updated_at);

        CREATE TABLE IF NOT EXISTS research_wclprice (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_wclprice_updated ON research_wclprice(updated_at);

        CREATE TABLE IF NOT EXISTS research_variance (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_variance_updated ON research_variance(updated_at);",
    )
    .map_err(|e| format!("create v68 tables: {e}"))?;
    Ok(())
}

pub fn upsert_avgprice(
    conn: &Connection,
    symbol: &str,
    snap: &AvgpriceSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v68(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("avgprice json: {e}"))?;
    conn.execute(
        "INSERT INTO research_avgprice (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert avgprice: {e}"))?;
    Ok(())
}

pub fn get_avgprice(conn: &Connection, symbol: &str) -> Result<Option<AvgpriceSnapshot>, String> {
    let _ = create_research_tables_v68(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_avgprice WHERE symbol = ?1")
        .map_err(|e| format!("prep avgprice: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query avgprice: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row avgprice: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get avgprice: {e}"))?;
        let snap: AvgpriceSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse avgprice: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_medprice(
    conn: &Connection,
    symbol: &str,
    snap: &MedpriceSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v68(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("medprice json: {e}"))?;
    conn.execute(
        "INSERT INTO research_medprice (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert medprice: {e}"))?;
    Ok(())
}

pub fn get_medprice(conn: &Connection, symbol: &str) -> Result<Option<MedpriceSnapshot>, String> {
    let _ = create_research_tables_v68(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_medprice WHERE symbol = ?1")
        .map_err(|e| format!("prep medprice: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query medprice: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row medprice: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get medprice: {e}"))?;
        let snap: MedpriceSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse medprice: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_typprice(
    conn: &Connection,
    symbol: &str,
    snap: &TypPriceSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v68(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("typprice json: {e}"))?;
    conn.execute(
        "INSERT INTO research_typprice (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert typprice: {e}"))?;
    Ok(())
}

pub fn get_typprice(conn: &Connection, symbol: &str) -> Result<Option<TypPriceSnapshot>, String> {
    let _ = create_research_tables_v68(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_typprice WHERE symbol = ?1")
        .map_err(|e| format!("prep typprice: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query typprice: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row typprice: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get typprice: {e}"))?;
        let snap: TypPriceSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse typprice: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_wclprice(
    conn: &Connection,
    symbol: &str,
    snap: &WclPriceSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v68(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("wclprice json: {e}"))?;
    conn.execute(
        "INSERT INTO research_wclprice (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert wclprice: {e}"))?;
    Ok(())
}

pub fn get_wclprice(conn: &Connection, symbol: &str) -> Result<Option<WclPriceSnapshot>, String> {
    let _ = create_research_tables_v68(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_wclprice WHERE symbol = ?1")
        .map_err(|e| format!("prep wclprice: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query wclprice: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row wclprice: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get wclprice: {e}"))?;
        let snap: WclPriceSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse wclprice: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_variance(
    conn: &Connection,
    symbol: &str,
    snap: &VarianceSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v68(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("variance json: {e}"))?;
    conn.execute(
        "INSERT INTO research_variance (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert variance: {e}"))?;
    Ok(())
}

pub fn get_variance(conn: &Connection, symbol: &str) -> Result<Option<VarianceSnapshot>, String> {
    let _ = create_research_tables_v68(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_variance WHERE symbol = ?1")
        .map_err(|e| format!("prep variance: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query variance: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row variance: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get variance: {e}"))?;
        let snap: VarianceSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse variance: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

// ── Research section ──
pub fn create_research_tables_v69(conn: &Connection) -> Result<(), String> {
    create_research_tables_v68(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_plus_di (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_plus_di_updated ON research_plus_di(updated_at);

        CREATE TABLE IF NOT EXISTS research_minus_di (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_minus_di_updated ON research_minus_di(updated_at);

        CREATE TABLE IF NOT EXISTS research_plus_dm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_plus_dm_updated ON research_plus_dm(updated_at);

        CREATE TABLE IF NOT EXISTS research_minus_dm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_minus_dm_updated ON research_minus_dm(updated_at);

        CREATE TABLE IF NOT EXISTS research_dx (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_dx_updated ON research_dx(updated_at);",
    )
    .map_err(|e| format!("create v69 tables: {e}"))?;
    Ok(())
}

pub fn upsert_plus_di(
    conn: &Connection,
    symbol: &str,
    snap: &PlusDiSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v69(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("plus_di json: {e}"))?;
    conn.execute(
        "INSERT INTO research_plus_di (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert plus_di: {e}"))?;
    Ok(())
}

pub fn get_plus_di(conn: &Connection, symbol: &str) -> Result<Option<PlusDiSnapshot>, String> {
    let _ = create_research_tables_v69(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_plus_di WHERE symbol = ?1")
        .map_err(|e| format!("prep plus_di: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query plus_di: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row plus_di: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get plus_di: {e}"))?;
        let snap: PlusDiSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse plus_di: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_minus_di(
    conn: &Connection,
    symbol: &str,
    snap: &MinusDiSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v69(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("minus_di json: {e}"))?;
    conn.execute(
        "INSERT INTO research_minus_di (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert minus_di: {e}"))?;
    Ok(())
}

pub fn get_minus_di(conn: &Connection, symbol: &str) -> Result<Option<MinusDiSnapshot>, String> {
    let _ = create_research_tables_v69(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_minus_di WHERE symbol = ?1")
        .map_err(|e| format!("prep minus_di: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query minus_di: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row minus_di: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get minus_di: {e}"))?;
        let snap: MinusDiSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse minus_di: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_plus_dm(
    conn: &Connection,
    symbol: &str,
    snap: &PlusDmSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v69(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("plus_dm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_plus_dm (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert plus_dm: {e}"))?;
    Ok(())
}

pub fn get_plus_dm(conn: &Connection, symbol: &str) -> Result<Option<PlusDmSnapshot>, String> {
    let _ = create_research_tables_v69(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_plus_dm WHERE symbol = ?1")
        .map_err(|e| format!("prep plus_dm: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query plus_dm: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row plus_dm: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get plus_dm: {e}"))?;
        let snap: PlusDmSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse plus_dm: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_minus_dm(
    conn: &Connection,
    symbol: &str,
    snap: &MinusDmSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v69(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("minus_dm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_minus_dm (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert minus_dm: {e}"))?;
    Ok(())
}

pub fn get_minus_dm(conn: &Connection, symbol: &str) -> Result<Option<MinusDmSnapshot>, String> {
    let _ = create_research_tables_v69(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_minus_dm WHERE symbol = ?1")
        .map_err(|e| format!("prep minus_dm: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query minus_dm: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row minus_dm: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get minus_dm: {e}"))?;
        let snap: MinusDmSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse minus_dm: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_dx(conn: &Connection, symbol: &str, snap: &DxSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v69(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("dx json: {e}"))?;
    conn.execute(
        "INSERT INTO research_dx (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert dx: {e}"))?;
    Ok(())
}

pub fn get_dx(conn: &Connection, symbol: &str) -> Result<Option<DxSnapshot>, String> {
    let _ = create_research_tables_v69(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_dx WHERE symbol = ?1")
        .map_err(|e| format!("prep dx: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query dx: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row dx: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get dx: {e}"))?;
        let snap: DxSnapshot = serde_json::from_str(&j).map_err(|e| format!("parse dx: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

// ── schema v70 + upsert/get helpers ─────────────────────

pub fn create_research_tables_v70(conn: &Connection) -> Result<(), String> {
    create_research_tables_v69(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_roc (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_roc_updated ON research_roc(updated_at);

        CREATE TABLE IF NOT EXISTS research_rocp (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_rocp_updated ON research_rocp(updated_at);

        CREATE TABLE IF NOT EXISTS research_rocr (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_rocr_updated ON research_rocr(updated_at);

        CREATE TABLE IF NOT EXISTS research_rocr100 (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_rocr100_updated ON research_rocr100(updated_at);

        CREATE TABLE IF NOT EXISTS research_correl (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_correl_updated ON research_correl(updated_at);",
    )
    .map_err(|e| format!("create v70 tables: {e}"))?;
    Ok(())
}

pub fn upsert_roc(conn: &Connection, symbol: &str, snap: &RocSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v70(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("roc json: {e}"))?;
    conn.execute(
        "INSERT INTO research_roc (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert roc: {e}"))?;
    Ok(())
}

pub fn get_roc(conn: &Connection, symbol: &str) -> Result<Option<RocSnapshot>, String> {
    let _ = create_research_tables_v70(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_roc WHERE symbol = ?1")
        .map_err(|e| format!("prep roc: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query roc: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row roc: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get roc: {e}"))?;
        let snap: RocSnapshot = serde_json::from_str(&j).map_err(|e| format!("parse roc: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_rocp(conn: &Connection, symbol: &str, snap: &RocpSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v70(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("rocp json: {e}"))?;
    conn.execute(
        "INSERT INTO research_rocp (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rocp: {e}"))?;
    Ok(())
}

pub fn get_rocp(conn: &Connection, symbol: &str) -> Result<Option<RocpSnapshot>, String> {
    let _ = create_research_tables_v70(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_rocp WHERE symbol = ?1")
        .map_err(|e| format!("prep rocp: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query rocp: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row rocp: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get rocp: {e}"))?;
        let snap: RocpSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse rocp: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_rocr(conn: &Connection, symbol: &str, snap: &RocrSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v70(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("rocr json: {e}"))?;
    conn.execute(
        "INSERT INTO research_rocr (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rocr: {e}"))?;
    Ok(())
}

pub fn get_rocr(conn: &Connection, symbol: &str) -> Result<Option<RocrSnapshot>, String> {
    let _ = create_research_tables_v70(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_rocr WHERE symbol = ?1")
        .map_err(|e| format!("prep rocr: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query rocr: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row rocr: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get rocr: {e}"))?;
        let snap: RocrSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse rocr: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_rocr100(
    conn: &Connection,
    symbol: &str,
    snap: &Rocr100Snapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v70(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("rocr100 json: {e}"))?;
    conn.execute(
        "INSERT INTO research_rocr100 (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rocr100: {e}"))?;
    Ok(())
}

pub fn get_rocr100(conn: &Connection, symbol: &str) -> Result<Option<Rocr100Snapshot>, String> {
    let _ = create_research_tables_v70(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_rocr100 WHERE symbol = ?1")
        .map_err(|e| format!("prep rocr100: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query rocr100: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row rocr100: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get rocr100: {e}"))?;
        let snap: Rocr100Snapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse rocr100: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_correl(conn: &Connection, symbol: &str, snap: &CorrelSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v70(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("correl json: {e}"))?;
    conn.execute(
        "INSERT INTO research_correl (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert correl: {e}"))?;
    Ok(())
}

pub fn get_correl(conn: &Connection, symbol: &str) -> Result<Option<CorrelSnapshot>, String> {
    let _ = create_research_tables_v70(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_correl WHERE symbol = ?1")
        .map_err(|e| format!("prep correl: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query correl: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row correl: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get correl: {e}"))?;
        let snap: CorrelSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse correl: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

// ── schema v71 + upsert/get helpers ─────────────────────

pub fn create_research_tables_v71(conn: &Connection) -> Result<(), String> {
    create_research_tables_v70(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_min (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_min_updated ON research_min(updated_at);

        CREATE TABLE IF NOT EXISTS research_max (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_max_updated ON research_max(updated_at);

        CREATE TABLE IF NOT EXISTS research_minmax (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_minmax_updated ON research_minmax(updated_at);

        CREATE TABLE IF NOT EXISTS research_minindex (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_minindex_updated ON research_minindex(updated_at);

        CREATE TABLE IF NOT EXISTS research_maxindex (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_maxindex_updated ON research_maxindex(updated_at);",
    )
    .map_err(|e| format!("create v71 tables: {e}"))?;
    Ok(())
}

pub fn upsert_min(conn: &Connection, symbol: &str, snap: &MinSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v71(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("min json: {e}"))?;
    conn.execute(
        "INSERT INTO research_min (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert min: {e}"))?;
    Ok(())
}

pub fn get_min(conn: &Connection, symbol: &str) -> Result<Option<MinSnapshot>, String> {
    let _ = create_research_tables_v71(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_min WHERE symbol = ?1")
        .map_err(|e| format!("prep min: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query min: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row min: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get min: {e}"))?;
        let snap: MinSnapshot = serde_json::from_str(&j).map_err(|e| format!("parse min: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_max(conn: &Connection, symbol: &str, snap: &MaxSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v71(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("max json: {e}"))?;
    conn.execute(
        "INSERT INTO research_max (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert max: {e}"))?;
    Ok(())
}

pub fn get_max(conn: &Connection, symbol: &str) -> Result<Option<MaxSnapshot>, String> {
    let _ = create_research_tables_v71(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_max WHERE symbol = ?1")
        .map_err(|e| format!("prep max: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query max: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row max: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get max: {e}"))?;
        let snap: MaxSnapshot = serde_json::from_str(&j).map_err(|e| format!("parse max: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_minmax(conn: &Connection, symbol: &str, snap: &MinMaxSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v71(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("minmax json: {e}"))?;
    conn.execute(
        "INSERT INTO research_minmax (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert minmax: {e}"))?;
    Ok(())
}

pub fn get_minmax(conn: &Connection, symbol: &str) -> Result<Option<MinMaxSnapshot>, String> {
    let _ = create_research_tables_v71(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_minmax WHERE symbol = ?1")
        .map_err(|e| format!("prep minmax: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query minmax: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row minmax: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get minmax: {e}"))?;
        let snap: MinMaxSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse minmax: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_minindex(
    conn: &Connection,
    symbol: &str,
    snap: &MinIndexSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v71(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("minindex json: {e}"))?;
    conn.execute(
        "INSERT INTO research_minindex (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert minindex: {e}"))?;
    Ok(())
}

pub fn get_minindex(conn: &Connection, symbol: &str) -> Result<Option<MinIndexSnapshot>, String> {
    let _ = create_research_tables_v71(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_minindex WHERE symbol = ?1")
        .map_err(|e| format!("prep minindex: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query minindex: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row minindex: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get minindex: {e}"))?;
        let snap: MinIndexSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse minindex: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_maxindex(
    conn: &Connection,
    symbol: &str,
    snap: &MaxIndexSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v71(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("maxindex json: {e}"))?;
    conn.execute(
        "INSERT INTO research_maxindex (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert maxindex: {e}"))?;
    Ok(())
}

pub fn get_maxindex(conn: &Connection, symbol: &str) -> Result<Option<MaxIndexSnapshot>, String> {
    let _ = create_research_tables_v71(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_maxindex WHERE symbol = ?1")
        .map_err(|e| format!("prep maxindex: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query maxindex: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row maxindex: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get maxindex: {e}"))?;
        let snap: MaxIndexSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse maxindex: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

// ── v72 schema: BBANDS / AD / ADOSC / SUM / LINEARREG_INTERCEPT ──

pub fn create_research_tables_v72(conn: &Connection) -> Result<(), String> {
    create_research_tables_v71(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_bbands (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_bbands_updated ON research_bbands(updated_at);

        CREATE TABLE IF NOT EXISTS research_ad (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_ad_updated ON research_ad(updated_at);

        CREATE TABLE IF NOT EXISTS research_adosc (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_adosc_updated ON research_adosc(updated_at);

        CREATE TABLE IF NOT EXISTS research_sum (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_sum_updated ON research_sum(updated_at);

        CREATE TABLE IF NOT EXISTS research_linreg_intercept (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_linreg_intercept_updated ON research_linreg_intercept(updated_at);",
    ).map_err(|e| format!("create v72 tables: {e}"))?;
    Ok(())
}

pub fn upsert_bbands(conn: &Connection, symbol: &str, snap: &BbandsSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v72(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("bbands json: {e}"))?;
    conn.execute(
        "INSERT INTO research_bbands (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert bbands: {e}"))?;
    Ok(())
}

pub fn get_bbands(conn: &Connection, symbol: &str) -> Result<Option<BbandsSnapshot>, String> {
    let _ = create_research_tables_v72(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_bbands WHERE symbol = ?1")
        .map_err(|e| format!("prep bbands: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query bbands: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row bbands: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get bbands: {e}"))?;
        let snap: BbandsSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse bbands: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_ad(conn: &Connection, symbol: &str, snap: &AdSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v72(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ad json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ad (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ad: {e}"))?;
    Ok(())
}

pub fn get_ad(conn: &Connection, symbol: &str) -> Result<Option<AdSnapshot>, String> {
    let _ = create_research_tables_v72(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_ad WHERE symbol = ?1")
        .map_err(|e| format!("prep ad: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query ad: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row ad: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get ad: {e}"))?;
        let snap: AdSnapshot = serde_json::from_str(&j).map_err(|e| format!("parse ad: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_adosc(conn: &Connection, symbol: &str, snap: &AdoscSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v72(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("adosc json: {e}"))?;
    conn.execute(
        "INSERT INTO research_adosc (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert adosc: {e}"))?;
    Ok(())
}

pub fn get_adosc(conn: &Connection, symbol: &str) -> Result<Option<AdoscSnapshot>, String> {
    let _ = create_research_tables_v72(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_adosc WHERE symbol = ?1")
        .map_err(|e| format!("prep adosc: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query adosc: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row adosc: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get adosc: {e}"))?;
        let snap: AdoscSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse adosc: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_sum(conn: &Connection, symbol: &str, snap: &SumSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v72(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("sum json: {e}"))?;
    conn.execute(
        "INSERT INTO research_sum (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert sum: {e}"))?;
    Ok(())
}

pub fn get_sum(conn: &Connection, symbol: &str) -> Result<Option<SumSnapshot>, String> {
    let _ = create_research_tables_v72(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_sum WHERE symbol = ?1")
        .map_err(|e| format!("prep sum: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query sum: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row sum: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get sum: {e}"))?;
        let snap: SumSnapshot = serde_json::from_str(&j).map_err(|e| format!("parse sum: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_linreg_intercept(
    conn: &Connection,
    symbol: &str,
    snap: &LinearRegInterceptSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v72(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("linreg_intercept json: {e}"))?;
    conn.execute(
        "INSERT INTO research_linreg_intercept (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert linreg_intercept: {e}"))?;
    Ok(())
}

pub fn get_linreg_intercept(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<LinearRegInterceptSnapshot>, String> {
    let _ = create_research_tables_v72(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_linreg_intercept WHERE symbol = ?1")
        .map_err(|e| format!("prep linreg_intercept: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query linreg_intercept: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row linreg_intercept: {e}"))?
    {
        let j: String = r.get(0).map_err(|e| format!("get linreg_intercept: {e}"))?;
        let snap: LinearRegInterceptSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse linreg_intercept: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

// ── AROONOSC / MINMAXINDEX / MACDEXT / MACDFIX / MAVP ──

pub fn create_research_tables_v73(conn: &Connection) -> Result<(), String> {
    create_research_tables_v72(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_aroonosc (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_aroonosc_updated ON research_aroonosc(updated_at);

        CREATE TABLE IF NOT EXISTS research_minmaxindex (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_minmaxindex_updated ON research_minmaxindex(updated_at);

        CREATE TABLE IF NOT EXISTS research_macdext (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_macdext_updated ON research_macdext(updated_at);

        CREATE TABLE IF NOT EXISTS research_macdfix (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_macdfix_updated ON research_macdfix(updated_at);

        CREATE TABLE IF NOT EXISTS research_mavp (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_mavp_updated ON research_mavp(updated_at);",
    ).map_err(|e| format!("create v73 tables: {e}"))?;
    Ok(())
}

pub fn upsert_aroonosc(
    conn: &Connection,
    symbol: &str,
    snap: &AroonoscSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v73(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("aroonosc json: {e}"))?;
    conn.execute(
        "INSERT INTO research_aroonosc (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert aroonosc: {e}"))?;
    Ok(())
}

pub fn get_aroonosc(conn: &Connection, symbol: &str) -> Result<Option<AroonoscSnapshot>, String> {
    let _ = create_research_tables_v73(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_aroonosc WHERE symbol = ?1")
        .map_err(|e| format!("prep aroonosc: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query aroonosc: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row aroonosc: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get aroonosc: {e}"))?;
        let snap: AroonoscSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse aroonosc: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_minmaxindex(
    conn: &Connection,
    symbol: &str,
    snap: &MinMaxIndexSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v73(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("minmaxindex json: {e}"))?;
    conn.execute(
        "INSERT INTO research_minmaxindex (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert minmaxindex: {e}"))?;
    Ok(())
}

pub fn get_minmaxindex(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<MinMaxIndexSnapshot>, String> {
    let _ = create_research_tables_v73(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_minmaxindex WHERE symbol = ?1")
        .map_err(|e| format!("prep minmaxindex: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query minmaxindex: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row minmaxindex: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get minmaxindex: {e}"))?;
        let snap: MinMaxIndexSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse minmaxindex: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_macdext(
    conn: &Connection,
    symbol: &str,
    snap: &MacdextSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v73(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("macdext json: {e}"))?;
    conn.execute(
        "INSERT INTO research_macdext (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert macdext: {e}"))?;
    Ok(())
}

pub fn get_macdext(conn: &Connection, symbol: &str) -> Result<Option<MacdextSnapshot>, String> {
    let _ = create_research_tables_v73(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_macdext WHERE symbol = ?1")
        .map_err(|e| format!("prep macdext: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query macdext: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row macdext: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get macdext: {e}"))?;
        let snap: MacdextSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse macdext: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_macdfix(
    conn: &Connection,
    symbol: &str,
    snap: &MacdfixSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v73(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("macdfix json: {e}"))?;
    conn.execute(
        "INSERT INTO research_macdfix (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert macdfix: {e}"))?;
    Ok(())
}

pub fn get_macdfix(conn: &Connection, symbol: &str) -> Result<Option<MacdfixSnapshot>, String> {
    let _ = create_research_tables_v73(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_macdfix WHERE symbol = ?1")
        .map_err(|e| format!("prep macdfix: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query macdfix: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row macdfix: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get macdfix: {e}"))?;
        let snap: MacdfixSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse macdfix: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_mavp(conn: &Connection, symbol: &str, snap: &MavpSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v73(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("mavp json: {e}"))?;
    conn.execute(
        "INSERT INTO research_mavp (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert mavp: {e}"))?;
    Ok(())
}

pub fn get_mavp(conn: &Connection, symbol: &str) -> Result<Option<MavpSnapshot>, String> {
    let _ = create_research_tables_v73(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_mavp WHERE symbol = ?1")
        .map_err(|e| format!("prep mavp: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query mavp: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row mavp: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get mavp: {e}"))?;
        let snap: MavpSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse mavp: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}
