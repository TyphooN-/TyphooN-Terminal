use super::*;

// ── : SMMA/ALLIGATOR/CRSI/SEB/IMI ──

pub fn create_research_tables_v57(conn: &Connection) -> Result<(), String> {
    create_research_tables_v56(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_smma (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_smma_updated ON research_smma(updated_at);

        CREATE TABLE IF NOT EXISTS research_alligator (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_alligator_updated ON research_alligator(updated_at);

        CREATE TABLE IF NOT EXISTS research_crsi (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_crsi_updated ON research_crsi(updated_at);

        CREATE TABLE IF NOT EXISTS research_seb (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_seb_updated ON research_seb(updated_at);

        CREATE TABLE IF NOT EXISTS research_imi (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_imi_updated ON research_imi(updated_at);",
    )
    .map_err(|e| format!("create v57 tables: {e}"))?;
    Ok(())
}

pub fn upsert_smma(conn: &Connection, symbol: &str, snap: &SmmaSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v57(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("smma json: {e}"))?;
    conn.execute(
        "INSERT INTO research_smma(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert smma: {e}"))?;
    Ok(())
}

pub fn get_smma(conn: &Connection, symbol: &str) -> Result<Option<SmmaSnapshot>, String> {
    let _ = create_research_tables_v57(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_smma WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_smma: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_smma: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_smma: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_alligator(
    conn: &Connection,
    symbol: &str,
    snap: &AlligatorSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v57(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("alligator json: {e}"))?;
    conn.execute(
        "INSERT INTO research_alligator(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert alligator: {e}"))?;
    Ok(())
}

pub fn get_alligator(conn: &Connection, symbol: &str) -> Result<Option<AlligatorSnapshot>, String> {
    let _ = create_research_tables_v57(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_alligator WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_alligator: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_alligator: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_alligator: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_crsi(conn: &Connection, symbol: &str, snap: &CrsiSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v57(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("crsi json: {e}"))?;
    conn.execute(
        "INSERT INTO research_crsi(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert crsi: {e}"))?;
    Ok(())
}

pub fn get_crsi(conn: &Connection, symbol: &str) -> Result<Option<CrsiSnapshot>, String> {
    let _ = create_research_tables_v57(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_crsi WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_crsi: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_crsi: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_crsi: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_seb(conn: &Connection, symbol: &str, snap: &SebSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v57(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("seb json: {e}"))?;
    conn.execute(
        "INSERT INTO research_seb(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert seb: {e}"))?;
    Ok(())
}

pub fn get_seb(conn: &Connection, symbol: &str) -> Result<Option<SebSnapshot>, String> {
    let _ = create_research_tables_v57(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_seb WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_seb: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_seb: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_seb: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_imi(conn: &Connection, symbol: &str, snap: &ImiSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v57(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("imi json: {e}"))?;
    conn.execute(
        "INSERT INTO research_imi(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert imi: {e}"))?;
    Ok(())
}

pub fn get_imi(conn: &Connection, symbol: &str) -> Result<Option<ImiSnapshot>, String> {
    let _ = create_research_tables_v57(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_imi WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_imi: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_imi: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_imi: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn create_research_tables_v58(conn: &Connection) -> Result<(), String> {
    create_research_tables_v57(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_gmma (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_gmma_updated ON research_gmma(updated_at);

        CREATE TABLE IF NOT EXISTS research_maenv (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_maenv_updated ON research_maenv(updated_at);

        CREATE TABLE IF NOT EXISTS research_adl (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_adl_updated ON research_adl(updated_at);

        CREATE TABLE IF NOT EXISTS research_vhf (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_vhf_updated ON research_vhf(updated_at);

        CREATE TABLE IF NOT EXISTS research_vroc (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_vroc_updated ON research_vroc(updated_at);",
    )
    .map_err(|e| format!("create v58 tables: {e}"))?;
    Ok(())
}

pub fn upsert_gmma(conn: &Connection, symbol: &str, snap: &GmmaSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v58(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("gmma json: {e}"))?;
    conn.execute(
        "INSERT INTO research_gmma(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert gmma: {e}"))?;
    Ok(())
}

pub fn get_gmma(conn: &Connection, symbol: &str) -> Result<Option<GmmaSnapshot>, String> {
    let _ = create_research_tables_v58(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_gmma WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_gmma: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_gmma: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_gmma: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_maenv(conn: &Connection, symbol: &str, snap: &MaenvSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v58(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("maenv json: {e}"))?;
    conn.execute(
        "INSERT INTO research_maenv(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert maenv: {e}"))?;
    Ok(())
}

pub fn get_maenv(conn: &Connection, symbol: &str) -> Result<Option<MaenvSnapshot>, String> {
    let _ = create_research_tables_v58(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_maenv WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_maenv: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_maenv: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_maenv: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_adl(conn: &Connection, symbol: &str, snap: &AdlSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v58(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("adl json: {e}"))?;
    conn.execute(
        "INSERT INTO research_adl(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert adl: {e}"))?;
    Ok(())
}

pub fn get_adl(conn: &Connection, symbol: &str) -> Result<Option<AdlSnapshot>, String> {
    let _ = create_research_tables_v58(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_adl WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_adl: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_adl: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_adl: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_vhf(conn: &Connection, symbol: &str, snap: &VhfSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v58(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("vhf json: {e}"))?;
    conn.execute(
        "INSERT INTO research_vhf(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert vhf: {e}"))?;
    Ok(())
}

pub fn get_vhf(conn: &Connection, symbol: &str) -> Result<Option<VhfSnapshot>, String> {
    let _ = create_research_tables_v58(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_vhf WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_vhf: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_vhf: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_vhf: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_vroc(conn: &Connection, symbol: &str, snap: &VrocSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v58(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("vroc json: {e}"))?;
    conn.execute(
        "INSERT INTO research_vroc(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert vroc: {e}"))?;
    Ok(())
}

pub fn get_vroc(conn: &Connection, symbol: &str) -> Result<Option<VrocSnapshot>, String> {
    let _ = create_research_tables_v58(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_vroc WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_vroc: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_vroc: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_vroc: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn create_research_tables_v59(conn: &Connection) -> Result<(), String> {
    create_research_tables_v58(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_kdj (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_kdj_updated ON research_kdj(updated_at);

        CREATE TABLE IF NOT EXISTS research_qqe (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_qqe_updated ON research_qqe(updated_at);

        CREATE TABLE IF NOT EXISTS research_pmo (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_pmo_updated ON research_pmo(updated_at);

        CREATE TABLE IF NOT EXISTS research_cfo (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cfo_updated ON research_cfo(updated_at);

        CREATE TABLE IF NOT EXISTS research_tmf (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_tmf_updated ON research_tmf(updated_at);",
    )
    .map_err(|e| format!("create v59 tables: {e}"))?;
    Ok(())
}

pub fn upsert_kdj(conn: &Connection, symbol: &str, snap: &KdjSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v59(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("kdj json: {e}"))?;
    conn.execute(
        "INSERT INTO research_kdj(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert kdj: {e}"))?;
    Ok(())
}

pub fn get_kdj(conn: &Connection, symbol: &str) -> Result<Option<KdjSnapshot>, String> {
    let _ = create_research_tables_v59(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_kdj WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_kdj: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_kdj: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_kdj: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_qqe(conn: &Connection, symbol: &str, snap: &QqeSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v59(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("qqe json: {e}"))?;
    conn.execute(
        "INSERT INTO research_qqe(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert qqe: {e}"))?;
    Ok(())
}

pub fn get_qqe(conn: &Connection, symbol: &str) -> Result<Option<QqeSnapshot>, String> {
    let _ = create_research_tables_v59(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_qqe WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_qqe: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_qqe: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_qqe: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_pmo(conn: &Connection, symbol: &str, snap: &PmoSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v59(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("pmo json: {e}"))?;
    conn.execute(
        "INSERT INTO research_pmo(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert pmo: {e}"))?;
    Ok(())
}

pub fn get_pmo(conn: &Connection, symbol: &str) -> Result<Option<PmoSnapshot>, String> {
    let _ = create_research_tables_v59(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_pmo WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_pmo: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_pmo: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_pmo: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_cfo(conn: &Connection, symbol: &str, snap: &CfoSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v59(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cfo json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cfo(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cfo: {e}"))?;
    Ok(())
}

pub fn get_cfo(conn: &Connection, symbol: &str) -> Result<Option<CfoSnapshot>, String> {
    let _ = create_research_tables_v59(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cfo WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_cfo: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_cfo: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_cfo: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_tmf(conn: &Connection, symbol: &str, snap: &TmfSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v59(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("tmf json: {e}"))?;
    conn.execute(
        "INSERT INTO research_tmf(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert tmf: {e}"))?;
    Ok(())
}

pub fn get_tmf(conn: &Connection, symbol: &str) -> Result<Option<TmfSnapshot>, String> {
    let _ = create_research_tables_v59(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_tmf WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_tmf: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_tmf: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_tmf: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn create_research_tables_v60(conn: &Connection) -> Result<(), String> {
    create_research_tables_v59(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_fractals (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_fractals_updated ON research_fractals(updated_at);

        CREATE TABLE IF NOT EXISTS research_ift_rsi (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_ift_rsi_updated ON research_ift_rsi(updated_at);

        CREATE TABLE IF NOT EXISTS research_mama (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_mama_updated ON research_mama(updated_at);

        CREATE TABLE IF NOT EXISTS research_cog (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cog_updated ON research_cog(updated_at);

        CREATE TABLE IF NOT EXISTS research_didi (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_didi_updated ON research_didi(updated_at);",
    )
    .map_err(|e| format!("create v60 tables: {e}"))?;
    Ok(())
}

pub fn upsert_fractals(
    conn: &Connection,
    symbol: &str,
    snap: &FractalsSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v60(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("fractals json: {e}"))?;
    conn.execute(
        "INSERT INTO research_fractals(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert fractals: {e}"))?;
    Ok(())
}

pub fn get_fractals(conn: &Connection, symbol: &str) -> Result<Option<FractalsSnapshot>, String> {
    let _ = create_research_tables_v60(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_fractals WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_fractals: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_fractals: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_fractals: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_ift_rsi(
    conn: &Connection,
    symbol: &str,
    snap: &IftRsiSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v60(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ift_rsi json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ift_rsi(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ift_rsi: {e}"))?;
    Ok(())
}

pub fn get_ift_rsi(conn: &Connection, symbol: &str) -> Result<Option<IftRsiSnapshot>, String> {
    let _ = create_research_tables_v60(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_ift_rsi WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ift_rsi: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_ift_rsi: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ift_rsi: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_mama(conn: &Connection, symbol: &str, snap: &MamaSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v60(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("mama json: {e}"))?;
    conn.execute(
        "INSERT INTO research_mama(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert mama: {e}"))?;
    Ok(())
}

pub fn get_mama(conn: &Connection, symbol: &str) -> Result<Option<MamaSnapshot>, String> {
    let _ = create_research_tables_v60(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_mama WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_mama: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_mama: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_mama: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_cog(conn: &Connection, symbol: &str, snap: &CogSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v60(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cog json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cog(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cog: {e}"))?;
    Ok(())
}

pub fn get_cog(conn: &Connection, symbol: &str) -> Result<Option<CogSnapshot>, String> {
    let _ = create_research_tables_v60(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cog WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_cog: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_cog: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_cog: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_didi(conn: &Connection, symbol: &str, snap: &DidiSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v60(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("didi json: {e}"))?;
    conn.execute(
        "INSERT INTO research_didi(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert didi: {e}"))?;
    Ok(())
}

pub fn get_didi(conn: &Connection, symbol: &str) -> Result<Option<DidiSnapshot>, String> {
    let _ = create_research_tables_v60(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_didi WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_didi: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_didi: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_didi: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}
