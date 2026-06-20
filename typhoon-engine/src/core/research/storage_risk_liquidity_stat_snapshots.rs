use super::*;

// ── + upsert/get ──

pub fn create_research_tables_v27(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v26(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_calmar (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_calmar_updated ON research_calmar(updated_at);

        CREATE TABLE IF NOT EXISTS research_ulcer (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_ulcer_updated ON research_ulcer(updated_at);

        CREATE TABLE IF NOT EXISTS research_varratio (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_varratio_updated ON research_varratio(updated_at);

        CREATE TABLE IF NOT EXISTS research_amihud (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_amihud_updated ON research_amihud(updated_at);

        CREATE TABLE IF NOT EXISTS research_jbnorm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_jbnorm_updated ON research_jbnorm(updated_at);",
    )
    .map_err(|e| format!("create v27 tables: {e}"))?;
    Ok(())
}

pub fn upsert_calmar(
    conn: &Connection,
    symbol: &str,
    snap: &CalmarRatioSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v27(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("calmar json: {e}"))?;
    conn.execute(
        "INSERT INTO research_calmar(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert calmar: {e}"))?;
    Ok(())
}

pub fn get_calmar(conn: &Connection, symbol: &str) -> Result<Option<CalmarRatioSnapshot>, String> {
    let _ = create_research_tables_v27(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_calmar WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_calmar: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_calmar: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_calmar: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_ulcer(
    conn: &Connection,
    symbol: &str,
    snap: &UlcerIndexSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v27(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ulcer json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ulcer(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ulcer: {e}"))?;
    Ok(())
}

pub fn get_ulcer(conn: &Connection, symbol: &str) -> Result<Option<UlcerIndexSnapshot>, String> {
    let _ = create_research_tables_v27(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_ulcer WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ulcer: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_ulcer: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ulcer: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_varratio(
    conn: &Connection,
    symbol: &str,
    snap: &VarianceRatioSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v27(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("varratio json: {e}"))?;
    conn.execute(
        "INSERT INTO research_varratio(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert varratio: {e}"))?;
    Ok(())
}

pub fn get_varratio(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<VarianceRatioSnapshot>, String> {
    let _ = create_research_tables_v27(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_varratio WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_varratio: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_varratio: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_varratio: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_amihud(
    conn: &Connection,
    symbol: &str,
    snap: &AmihudIlliqSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v27(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("amihud json: {e}"))?;
    conn.execute(
        "INSERT INTO research_amihud(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert amihud: {e}"))?;
    Ok(())
}

pub fn get_amihud(conn: &Connection, symbol: &str) -> Result<Option<AmihudIlliqSnapshot>, String> {
    let _ = create_research_tables_v27(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_amihud WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_amihud: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_amihud: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_amihud: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_jbnorm(
    conn: &Connection,
    symbol: &str,
    snap: &JarqueBeraSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v27(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("jbnorm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_jbnorm(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert jbnorm: {e}"))?;
    Ok(())
}

pub fn get_jbnorm(conn: &Connection, symbol: &str) -> Result<Option<JarqueBeraSnapshot>, String> {
    let _ = create_research_tables_v27(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_jbnorm WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_jbnorm: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_jbnorm: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_jbnorm: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── + upsert/get ──

pub fn create_research_tables_v28(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v27(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_omega (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_omega_updated ON research_omega(updated_at);

        CREATE TABLE IF NOT EXISTS research_dfa (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_dfa_updated ON research_dfa(updated_at);

        CREATE TABLE IF NOT EXISTS research_burke (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_burke_updated ON research_burke(updated_at);

        CREATE TABLE IF NOT EXISTS research_monthseas (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_monthseas_updated ON research_monthseas(updated_at);

        CREATE TABLE IF NOT EXISTS research_rollsprd (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_rollsprd_updated ON research_rollsprd(updated_at);",
    )
    .map_err(|e| format!("create v28 tables: {e}"))?;
    Ok(())
}

pub fn upsert_omega(
    conn: &Connection,
    symbol: &str,
    snap: &OmegaRatioSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v28(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("omega json: {e}"))?;
    conn.execute(
        "INSERT INTO research_omega(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert omega: {e}"))?;
    Ok(())
}

pub fn get_omega(conn: &Connection, symbol: &str) -> Result<Option<OmegaRatioSnapshot>, String> {
    let _ = create_research_tables_v28(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_omega WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_omega: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_omega: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_omega: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_dfa(
    conn: &Connection,
    symbol: &str,
    snap: &DetrendedFluctuationSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v28(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("dfa json: {e}"))?;
    conn.execute(
        "INSERT INTO research_dfa(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert dfa: {e}"))?;
    Ok(())
}

pub fn get_dfa(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<DetrendedFluctuationSnapshot>, String> {
    let _ = create_research_tables_v28(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_dfa WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_dfa: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_dfa: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_dfa: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_burke(
    conn: &Connection,
    symbol: &str,
    snap: &BurkeRatioSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v28(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("burke json: {e}"))?;
    conn.execute(
        "INSERT INTO research_burke(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert burke: {e}"))?;
    Ok(())
}

pub fn get_burke(conn: &Connection, symbol: &str) -> Result<Option<BurkeRatioSnapshot>, String> {
    let _ = create_research_tables_v28(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_burke WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_burke: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_burke: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_burke: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_monthseas(
    conn: &Connection,
    symbol: &str,
    snap: &MonthlySeasonalitySnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v28(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("monthseas json: {e}"))?;
    conn.execute(
        "INSERT INTO research_monthseas(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert monthseas: {e}"))?;
    Ok(())
}

pub fn get_monthseas(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<MonthlySeasonalitySnapshot>, String> {
    let _ = create_research_tables_v28(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_monthseas WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_monthseas: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_monthseas: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_monthseas: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_rollsprd(
    conn: &Connection,
    symbol: &str,
    snap: &RollSpreadSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v28(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("rollsprd json: {e}"))?;
    conn.execute(
        "INSERT INTO research_rollsprd(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rollsprd: {e}"))?;
    Ok(())
}

pub fn get_rollsprd(conn: &Connection, symbol: &str) -> Result<Option<RollSpreadSnapshot>, String> {
    let _ = create_research_tables_v28(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_rollsprd WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_rollsprd: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_rollsprd: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_rollsprd: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── v29 ──

pub fn create_research_tables_v29(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v28(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_parkinson (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_parkinson_updated ON research_parkinson(updated_at);

        CREATE TABLE IF NOT EXISTS research_gkvol (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_gkvol_updated ON research_gkvol(updated_at);

        CREATE TABLE IF NOT EXISTS research_rsvol (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_rsvol_updated ON research_rsvol(updated_at);

        CREATE TABLE IF NOT EXISTS research_cvar (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cvar_updated ON research_cvar(updated_at);

        CREATE TABLE IF NOT EXISTS research_doweffect (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_doweffect_updated ON research_doweffect(updated_at);",
    ).map_err(|e| format!("create v29 tables: {e}"))?;
    Ok(())
}

pub fn upsert_parkinson(
    conn: &Connection,
    symbol: &str,
    snap: &ParkinsonVolSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v29(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("parkinson json: {e}"))?;
    conn.execute(
        "INSERT INTO research_parkinson(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert parkinson: {e}"))?;
    Ok(())
}

pub fn get_parkinson(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<ParkinsonVolSnapshot>, String> {
    let _ = create_research_tables_v29(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_parkinson WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_parkinson: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_parkinson: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_parkinson: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_gkvol(
    conn: &Connection,
    symbol: &str,
    snap: &GarmanKlassVolSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v29(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("gkvol json: {e}"))?;
    conn.execute(
        "INSERT INTO research_gkvol(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert gkvol: {e}"))?;
    Ok(())
}

pub fn get_gkvol(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<GarmanKlassVolSnapshot>, String> {
    let _ = create_research_tables_v29(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_gkvol WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_gkvol: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_gkvol: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_gkvol: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_rsvol(
    conn: &Connection,
    symbol: &str,
    snap: &RogersSatchellVolSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v29(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("rsvol json: {e}"))?;
    conn.execute(
        "INSERT INTO research_rsvol(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rsvol: {e}"))?;
    Ok(())
}

pub fn get_rsvol(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<RogersSatchellVolSnapshot>, String> {
    let _ = create_research_tables_v29(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_rsvol WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_rsvol: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_rsvol: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_rsvol: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_cvar(conn: &Connection, symbol: &str, snap: &CVaRSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v29(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cvar json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cvar(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cvar: {e}"))?;
    Ok(())
}

pub fn get_cvar(conn: &Connection, symbol: &str) -> Result<Option<CVaRSnapshot>, String> {
    let _ = create_research_tables_v29(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cvar WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_cvar: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_cvar: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_cvar: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_doweffect(
    conn: &Connection,
    symbol: &str,
    snap: &DayOfWeekEffectSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v29(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("doweffect json: {e}"))?;
    conn.execute(
        "INSERT INTO research_doweffect(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert doweffect: {e}"))?;
    Ok(())
}

pub fn get_doweffect(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<DayOfWeekEffectSnapshot>, String> {
    let _ = create_research_tables_v29(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_doweffect WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_doweffect: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_doweffect: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_doweffect: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── STERLING / KELLYF / LJUNGB / RUNSTEST / ZERORET ──

pub fn create_research_tables_v30(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v29(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_sterling (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_sterling_updated ON research_sterling(updated_at);

        CREATE TABLE IF NOT EXISTS research_kellyf (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_kellyf_updated ON research_kellyf(updated_at);

        CREATE TABLE IF NOT EXISTS research_ljungb (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_ljungb_updated ON research_ljungb(updated_at);

        CREATE TABLE IF NOT EXISTS research_runstest (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_runstest_updated ON research_runstest(updated_at);

        CREATE TABLE IF NOT EXISTS research_zeroret (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_zeroret_updated ON research_zeroret(updated_at);",
    )
    .map_err(|e| format!("create v30 tables: {e}"))?;
    Ok(())
}

pub fn upsert_sterling(
    conn: &Connection,
    symbol: &str,
    snap: &SterlingRatioSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v30(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("sterling json: {e}"))?;
    conn.execute(
        "INSERT INTO research_sterling(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert sterling: {e}"))?;
    Ok(())
}

pub fn get_sterling(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<SterlingRatioSnapshot>, String> {
    let _ = create_research_tables_v30(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_sterling WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_sterling: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_sterling: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_sterling: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_kellyf(
    conn: &Connection,
    symbol: &str,
    snap: &KellyFractionSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v30(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("kellyf json: {e}"))?;
    conn.execute(
        "INSERT INTO research_kellyf(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert kellyf: {e}"))?;
    Ok(())
}

pub fn get_kellyf(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<KellyFractionSnapshot>, String> {
    let _ = create_research_tables_v30(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_kellyf WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_kellyf: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_kellyf: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_kellyf: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_ljungb(
    conn: &Connection,
    symbol: &str,
    snap: &LjungBoxSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v30(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ljungb json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ljungb(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ljungb: {e}"))?;
    Ok(())
}

pub fn get_ljungb(conn: &Connection, symbol: &str) -> Result<Option<LjungBoxSnapshot>, String> {
    let _ = create_research_tables_v30(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_ljungb WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ljungb: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_ljungb: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ljungb: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_runstest(
    conn: &Connection,
    symbol: &str,
    snap: &RunsTestSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v30(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("runstest json: {e}"))?;
    conn.execute(
        "INSERT INTO research_runstest(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert runstest: {e}"))?;
    Ok(())
}

pub fn get_runstest(conn: &Connection, symbol: &str) -> Result<Option<RunsTestSnapshot>, String> {
    let _ = create_research_tables_v30(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_runstest WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_runstest: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_runstest: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_runstest: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_zeroret(
    conn: &Connection,
    symbol: &str,
    snap: &ZeroReturnSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v30(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("zeroret json: {e}"))?;
    conn.execute(
        "INSERT INTO research_zeroret(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert zeroret: {e}"))?;
    Ok(())
}

pub fn get_zeroret(conn: &Connection, symbol: &str) -> Result<Option<ZeroReturnSnapshot>, String> {
    let _ = create_research_tables_v30(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_zeroret WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_zeroret: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_zeroret: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_zeroret: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}
