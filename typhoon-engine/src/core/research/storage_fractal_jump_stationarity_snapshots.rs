use super::*;

/// Round 37 schema v38: adds `research_higuchi`, `research_pickands`,
/// `research_kappa3`, `research_lyapunov`, `research_rankac`. Additive over v37.
pub fn create_research_tables_v38(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v37(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_higuchi (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_higuchi_updated ON research_higuchi(updated_at);

        CREATE TABLE IF NOT EXISTS research_pickands (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_pickands_updated ON research_pickands(updated_at);

        CREATE TABLE IF NOT EXISTS research_kappa3 (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_kappa3_updated ON research_kappa3(updated_at);

        CREATE TABLE IF NOT EXISTS research_lyapunov (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_lyapunov_updated ON research_lyapunov(updated_at);

        CREATE TABLE IF NOT EXISTS research_rankac (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_rankac_updated ON research_rankac(updated_at);",
    )
    .map_err(|e| format!("create v38 tables: {e}"))?;
    Ok(())
}

pub fn upsert_higuchi(
    conn: &Connection,
    symbol: &str,
    snap: &HiguchiSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v38(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("higuchi json: {e}"))?;
    conn.execute(
        "INSERT INTO research_higuchi(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert higuchi: {e}"))?;
    Ok(())
}

pub fn get_higuchi(conn: &Connection, symbol: &str) -> Result<Option<HiguchiSnapshot>, String> {
    let _ = create_research_tables_v38(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_higuchi WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_higuchi: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_higuchi: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_higuchi: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_pickands(
    conn: &Connection,
    symbol: &str,
    snap: &PickandsSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v38(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("pickands json: {e}"))?;
    conn.execute(
        "INSERT INTO research_pickands(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert pickands: {e}"))?;
    Ok(())
}

pub fn get_pickands(conn: &Connection, symbol: &str) -> Result<Option<PickandsSnapshot>, String> {
    let _ = create_research_tables_v38(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_pickands WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_pickands: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_pickands: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_pickands: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_kappa3(conn: &Connection, symbol: &str, snap: &Kappa3Snapshot) -> Result<(), String> {
    let _ = create_research_tables_v38(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("kappa3 json: {e}"))?;
    conn.execute(
        "INSERT INTO research_kappa3(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert kappa3: {e}"))?;
    Ok(())
}

pub fn get_kappa3(conn: &Connection, symbol: &str) -> Result<Option<Kappa3Snapshot>, String> {
    let _ = create_research_tables_v38(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_kappa3 WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_kappa3: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_kappa3: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_kappa3: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_lyapunov(
    conn: &Connection,
    symbol: &str,
    snap: &LyapunovSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v38(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("lyapunov json: {e}"))?;
    conn.execute(
        "INSERT INTO research_lyapunov(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert lyapunov: {e}"))?;
    Ok(())
}

pub fn get_lyapunov(conn: &Connection, symbol: &str) -> Result<Option<LyapunovSnapshot>, String> {
    let _ = create_research_tables_v38(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_lyapunov WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_lyapunov: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_lyapunov: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_lyapunov: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_rankac(conn: &Connection, symbol: &str, snap: &RankacSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v38(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("rankac json: {e}"))?;
    conn.execute(
        "INSERT INTO research_rankac(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rankac: {e}"))?;
    Ok(())
}

pub fn get_rankac(conn: &Connection, symbol: &str) -> Result<Option<RankacSnapshot>, String> {
    let _ = create_research_tables_v38(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_rankac WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_rankac: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_rankac: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_rankac: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

/// Round 38 schema v39: adds `research_bnsjump`, `research_pproot`,
/// `research_mfdfa`, `research_hillks`, `research_tsi`. Additive over v38.
pub fn create_research_tables_v39(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v38(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_bnsjump (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_bnsjump_updated ON research_bnsjump(updated_at);

        CREATE TABLE IF NOT EXISTS research_pproot (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_pproot_updated ON research_pproot(updated_at);

        CREATE TABLE IF NOT EXISTS research_mfdfa (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_mfdfa_updated ON research_mfdfa(updated_at);

        CREATE TABLE IF NOT EXISTS research_hillks (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_hillks_updated ON research_hillks(updated_at);

        CREATE TABLE IF NOT EXISTS research_tsi (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_tsi_updated ON research_tsi(updated_at);",
    )
    .map_err(|e| format!("create v39 tables: {e}"))?;
    Ok(())
}

pub fn upsert_bnsjump(
    conn: &Connection,
    symbol: &str,
    snap: &BnsjumpSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v39(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("bnsjump json: {e}"))?;
    conn.execute(
        "INSERT INTO research_bnsjump(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert bnsjump: {e}"))?;
    Ok(())
}

pub fn get_bnsjump(conn: &Connection, symbol: &str) -> Result<Option<BnsjumpSnapshot>, String> {
    let _ = create_research_tables_v39(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_bnsjump WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_bnsjump: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_bnsjump: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_bnsjump: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_pproot(conn: &Connection, symbol: &str, snap: &PprootSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v39(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("pproot json: {e}"))?;
    conn.execute(
        "INSERT INTO research_pproot(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert pproot: {e}"))?;
    Ok(())
}

pub fn get_pproot(conn: &Connection, symbol: &str) -> Result<Option<PprootSnapshot>, String> {
    let _ = create_research_tables_v39(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_pproot WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_pproot: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_pproot: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_pproot: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_mfdfa(conn: &Connection, symbol: &str, snap: &MfdfaSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v39(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("mfdfa json: {e}"))?;
    conn.execute(
        "INSERT INTO research_mfdfa(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert mfdfa: {e}"))?;
    Ok(())
}

pub fn get_mfdfa(conn: &Connection, symbol: &str) -> Result<Option<MfdfaSnapshot>, String> {
    let _ = create_research_tables_v39(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_mfdfa WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_mfdfa: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_mfdfa: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_mfdfa: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_hillks(conn: &Connection, symbol: &str, snap: &HillksSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v39(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("hillks json: {e}"))?;
    conn.execute(
        "INSERT INTO research_hillks(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert hillks: {e}"))?;
    Ok(())
}

pub fn get_hillks(conn: &Connection, symbol: &str) -> Result<Option<HillksSnapshot>, String> {
    let _ = create_research_tables_v39(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_hillks WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_hillks: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_hillks: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_hillks: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_tsi(conn: &Connection, symbol: &str, snap: &TsiSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v39(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("tsi json: {e}"))?;
    conn.execute(
        "INSERT INTO research_tsi(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert tsi: {e}"))?;
    Ok(())
}

pub fn get_tsi(conn: &Connection, symbol: &str) -> Result<Option<TsiSnapshot>, String> {
    let _ = create_research_tables_v39(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_tsi WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_tsi: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_tsi: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_tsi: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

/// Round 39 schema v40: adds `research_garch11`, `research_sadf`,
/// `research_cordim`, `research_skspec`, `research_automi`. Additive over v39.
pub fn create_research_tables_v40(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v39(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_garch11 (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_garch11_updated ON research_garch11(updated_at);

        CREATE TABLE IF NOT EXISTS research_sadf (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_sadf_updated ON research_sadf(updated_at);

        CREATE TABLE IF NOT EXISTS research_cordim (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cordim_updated ON research_cordim(updated_at);

        CREATE TABLE IF NOT EXISTS research_skspec (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_skspec_updated ON research_skspec(updated_at);

        CREATE TABLE IF NOT EXISTS research_automi (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_automi_updated ON research_automi(updated_at);",
    )
    .map_err(|e| format!("create v40 tables: {e}"))?;
    Ok(())
}

pub fn upsert_garch11(
    conn: &Connection,
    symbol: &str,
    snap: &Garch11Snapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v40(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("garch11 json: {e}"))?;
    conn.execute(
        "INSERT INTO research_garch11(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert garch11: {e}"))?;
    Ok(())
}

pub fn get_garch11(conn: &Connection, symbol: &str) -> Result<Option<Garch11Snapshot>, String> {
    let _ = create_research_tables_v40(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_garch11 WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_garch11: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_garch11: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_garch11: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_sadf(conn: &Connection, symbol: &str, snap: &SadfSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v40(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("sadf json: {e}"))?;
    conn.execute(
        "INSERT INTO research_sadf(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert sadf: {e}"))?;
    Ok(())
}

pub fn get_sadf(conn: &Connection, symbol: &str) -> Result<Option<SadfSnapshot>, String> {
    let _ = create_research_tables_v40(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_sadf WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_sadf: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_sadf: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_sadf: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_cordim(conn: &Connection, symbol: &str, snap: &CordimSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v40(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cordim json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cordim(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cordim: {e}"))?;
    Ok(())
}

pub fn get_cordim(conn: &Connection, symbol: &str) -> Result<Option<CordimSnapshot>, String> {
    let _ = create_research_tables_v40(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cordim WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_cordim: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_cordim: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_cordim: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_skspec(conn: &Connection, symbol: &str, snap: &SkspecSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v40(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("skspec json: {e}"))?;
    conn.execute(
        "INSERT INTO research_skspec(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert skspec: {e}"))?;
    Ok(())
}

pub fn get_skspec(conn: &Connection, symbol: &str) -> Result<Option<SkspecSnapshot>, String> {
    let _ = create_research_tables_v40(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_skspec WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_skspec: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_skspec: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_skspec: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_automi(conn: &Connection, symbol: &str, snap: &AutomiSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v40(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("automi json: {e}"))?;
    conn.execute(
        "INSERT INTO research_automi(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert automi: {e}"))?;
    Ok(())
}

pub fn get_automi(conn: &Connection, symbol: &str) -> Result<Option<AutomiSnapshot>, String> {
    let _ = create_research_tables_v40(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_automi WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_automi: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_automi: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_automi: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}
