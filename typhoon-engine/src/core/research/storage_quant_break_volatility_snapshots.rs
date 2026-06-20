use super::*;

// ── v78 (Quant Stats) ──

pub fn create_research_tables_v78(conn: &Connection) -> Result<(), String> {
    create_research_tables_v77(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_modsharpe (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_modsharpe_updated ON research_modsharpe(updated_at);

        CREATE TABLE IF NOT EXISTS research_hsiehtest (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_hsiehtest_updated ON research_hsiehtest(updated_at);

        CREATE TABLE IF NOT EXISTS research_chowbreak (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_chowbreak_updated ON research_chowbreak(updated_at);

        CREATE TABLE IF NOT EXISTS research_driftburst (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_driftburst_updated ON research_driftburst(updated_at);

        CREATE TABLE IF NOT EXISTS research_hlvclust (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_hlvclust_updated ON research_hlvclust(updated_at);",
    ).map_err(|e| format!("create v78 tables: {e}"))?;
    Ok(())
}

pub fn upsert_modsharpe(
    conn: &Connection,
    symbol: &str,
    snap: &ModSharpeSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v78(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("modsharpe json: {e}"))?;
    conn.execute(
        "INSERT INTO research_modsharpe (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert modsharpe: {e}"))?;
    Ok(())
}

pub fn get_modsharpe(conn: &Connection, symbol: &str) -> Result<Option<ModSharpeSnapshot>, String> {
    let _ = create_research_tables_v78(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_modsharpe WHERE symbol = ?1")
        .map_err(|e| format!("prep modsharpe: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query modsharpe: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row modsharpe: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get modsharpe: {e}"))?;
        let snap: ModSharpeSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse modsharpe: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_hsiehtest(
    conn: &Connection,
    symbol: &str,
    snap: &HsiehTestSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v78(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("hsiehtest json: {e}"))?;
    conn.execute(
        "INSERT INTO research_hsiehtest (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert hsiehtest: {e}"))?;
    Ok(())
}

pub fn get_hsiehtest(conn: &Connection, symbol: &str) -> Result<Option<HsiehTestSnapshot>, String> {
    let _ = create_research_tables_v78(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_hsiehtest WHERE symbol = ?1")
        .map_err(|e| format!("prep hsiehtest: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query hsiehtest: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row hsiehtest: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get hsiehtest: {e}"))?;
        let snap: HsiehTestSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse hsiehtest: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_chowbreak(
    conn: &Connection,
    symbol: &str,
    snap: &ChowBreakSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v78(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("chowbreak json: {e}"))?;
    conn.execute(
        "INSERT INTO research_chowbreak (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert chowbreak: {e}"))?;
    Ok(())
}

pub fn get_chowbreak(conn: &Connection, symbol: &str) -> Result<Option<ChowBreakSnapshot>, String> {
    let _ = create_research_tables_v78(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_chowbreak WHERE symbol = ?1")
        .map_err(|e| format!("prep chowbreak: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query chowbreak: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row chowbreak: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get chowbreak: {e}"))?;
        let snap: ChowBreakSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse chowbreak: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_driftburst(
    conn: &Connection,
    symbol: &str,
    snap: &DriftBurstSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v78(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("driftburst json: {e}"))?;
    conn.execute(
        "INSERT INTO research_driftburst (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert driftburst: {e}"))?;
    Ok(())
}

pub fn get_driftburst(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<DriftBurstSnapshot>, String> {
    let _ = create_research_tables_v78(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_driftburst WHERE symbol = ?1")
        .map_err(|e| format!("prep driftburst: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query driftburst: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row driftburst: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get driftburst: {e}"))?;
        let snap: DriftBurstSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse driftburst: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_hlvclust(
    conn: &Connection,
    symbol: &str,
    snap: &HlvClustSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v78(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("hlvclust json: {e}"))?;
    conn.execute(
        "INSERT INTO research_hlvclust (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert hlvclust: {e}"))?;
    Ok(())
}

pub fn get_hlvclust(conn: &Connection, symbol: &str) -> Result<Option<HlvClustSnapshot>, String> {
    let _ = create_research_tables_v78(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_hlvclust WHERE symbol = ?1")
        .map_err(|e| format!("prep hlvclust: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query hlvclust: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row hlvclust: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get hlvclust: {e}"))?;
        let snap: HlvClustSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse hlvclust: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

// ── v79 (Quant Stats) ──

pub fn create_research_tables_v79(conn: &Connection) -> Result<(), String> {
    create_research_tables_v78(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_yangzhang (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_yangzhang_updated ON research_yangzhang(updated_at);

        CREATE TABLE IF NOT EXISTS research_kuiper (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_kuiper_updated ON research_kuiper(updated_at);

        CREATE TABLE IF NOT EXISTS research_dagostino (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_dagostino_updated ON research_dagostino(updated_at);

        CREATE TABLE IF NOT EXISTS research_baiperron (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_baiperron_updated ON research_baiperron(updated_at);

        CREATE TABLE IF NOT EXISTS research_kupiecpof (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_kupiecpof_updated ON research_kupiecpof(updated_at);",
    ).map_err(|e| format!("create v79 tables: {e}"))?;
    Ok(())
}

pub fn upsert_yangzhang(
    conn: &Connection,
    symbol: &str,
    snap: &YangZhangVolSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v79(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("yangzhang json: {e}"))?;
    conn.execute(
        "INSERT INTO research_yangzhang (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert yangzhang: {e}"))?;
    Ok(())
}

pub fn get_yangzhang(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<YangZhangVolSnapshot>, String> {
    let _ = create_research_tables_v79(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_yangzhang WHERE symbol = ?1")
        .map_err(|e| format!("prep yangzhang: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query yangzhang: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row yangzhang: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get yangzhang: {e}"))?;
        let snap: YangZhangVolSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse yangzhang: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_kuiper(conn: &Connection, symbol: &str, snap: &KuiperSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v79(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("kuiper json: {e}"))?;
    conn.execute(
        "INSERT INTO research_kuiper (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert kuiper: {e}"))?;
    Ok(())
}

pub fn get_kuiper(conn: &Connection, symbol: &str) -> Result<Option<KuiperSnapshot>, String> {
    let _ = create_research_tables_v79(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_kuiper WHERE symbol = ?1")
        .map_err(|e| format!("prep kuiper: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query kuiper: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row kuiper: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get kuiper: {e}"))?;
        let snap: KuiperSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse kuiper: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_dagostino(
    conn: &Connection,
    symbol: &str,
    snap: &DagostinoSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v79(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("dagostino json: {e}"))?;
    conn.execute(
        "INSERT INTO research_dagostino (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert dagostino: {e}"))?;
    Ok(())
}

pub fn get_dagostino(conn: &Connection, symbol: &str) -> Result<Option<DagostinoSnapshot>, String> {
    let _ = create_research_tables_v79(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_dagostino WHERE symbol = ?1")
        .map_err(|e| format!("prep dagostino: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query dagostino: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row dagostino: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get dagostino: {e}"))?;
        let snap: DagostinoSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse dagostino: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_baiperron(
    conn: &Connection,
    symbol: &str,
    snap: &BaiPerronSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v79(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("baiperron json: {e}"))?;
    conn.execute(
        "INSERT INTO research_baiperron (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert baiperron: {e}"))?;
    Ok(())
}

pub fn get_baiperron(conn: &Connection, symbol: &str) -> Result<Option<BaiPerronSnapshot>, String> {
    let _ = create_research_tables_v79(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_baiperron WHERE symbol = ?1")
        .map_err(|e| format!("prep baiperron: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query baiperron: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row baiperron: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get baiperron: {e}"))?;
        let snap: BaiPerronSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse baiperron: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_kupiecpof(
    conn: &Connection,
    symbol: &str,
    snap: &KupiecPofSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v79(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("kupiecpof json: {e}"))?;
    conn.execute(
        "INSERT INTO research_kupiecpof (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert kupiecpof: {e}"))?;
    Ok(())
}

pub fn get_kupiecpof(conn: &Connection, symbol: &str) -> Result<Option<KupiecPofSnapshot>, String> {
    let _ = create_research_tables_v79(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_kupiecpof WHERE symbol = ?1")
        .map_err(|e| format!("prep kupiecpof: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query kupiecpof: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row kupiecpof: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get kupiecpof: {e}"))?;
        let snap: KupiecPofSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse kupiecpof: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}
