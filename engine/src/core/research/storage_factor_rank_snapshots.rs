use super::*;

// ── Godel Parity Round 13 schema + helpers ─────────────────────────

pub fn create_research_tables_v13(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_momentum (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_momentum_updated ON research_momentum(updated_at);

        CREATE TABLE IF NOT EXISTS research_liquidity (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_liquidity_updated ON research_liquidity(updated_at);

        CREATE TABLE IF NOT EXISTS research_breakout (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_breakout_updated ON research_breakout(updated_at);

        CREATE TABLE IF NOT EXISTS research_cash_cycle (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cash_cycle_updated ON research_cash_cycle(updated_at);

        CREATE TABLE IF NOT EXISTS research_credit (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_credit_updated ON research_credit(updated_at);",
    ).map_err(|e| format!("create v13 tables: {e}"))?;
    Ok(())
}

pub fn upsert_momentum(
    conn: &Connection,
    symbol: &str,
    snap: &MomentumSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v13(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("momentum json: {e}"))?;
    conn.execute(
        "INSERT INTO research_momentum(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert momentum: {e}"))?;
    Ok(())
}

pub fn get_momentum(conn: &Connection, symbol: &str) -> Result<Option<MomentumSnapshot>, String> {
    let _ = create_research_tables_v13(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_momentum WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_momentum: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_momentum: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_momentum: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_liquidity(
    conn: &Connection,
    symbol: &str,
    snap: &LiquiditySnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v13(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("liquidity json: {e}"))?;
    conn.execute(
        "INSERT INTO research_liquidity(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert liquidity: {e}"))?;
    Ok(())
}

pub fn get_liquidity(conn: &Connection, symbol: &str) -> Result<Option<LiquiditySnapshot>, String> {
    let _ = create_research_tables_v13(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_liquidity WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_liquidity: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_liquidity: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_liquidity: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_breakout(
    conn: &Connection,
    symbol: &str,
    snap: &BreakoutSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v13(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("breakout json: {e}"))?;
    conn.execute(
        "INSERT INTO research_breakout(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert breakout: {e}"))?;
    Ok(())
}

pub fn get_breakout(conn: &Connection, symbol: &str) -> Result<Option<BreakoutSnapshot>, String> {
    let _ = create_research_tables_v13(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_breakout WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_breakout: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_breakout: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_breakout: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_cash_cycle(
    conn: &Connection,
    symbol: &str,
    snap: &CashCycleSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v13(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cash_cycle json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cash_cycle(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cash_cycle: {e}"))?;
    Ok(())
}

pub fn get_cash_cycle(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CashCycleSnapshot>, String> {
    let _ = create_research_tables_v13(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_cash_cycle WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_cash_cycle: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_cash_cycle: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_cash_cycle: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_credit(conn: &Connection, symbol: &str, snap: &CreditSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v13(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("credit json: {e}"))?;
    conn.execute(
        "INSERT INTO research_credit(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert credit: {e}"))?;
    Ok(())
}

pub fn get_credit(conn: &Connection, symbol: &str) -> Result<Option<CreditSnapshot>, String> {
    let _ = create_research_tables_v13(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_credit WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_credit: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_credit: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_credit: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── Godel Parity Round 14 schema + helpers ─────────────────────────

pub fn create_research_tables_v14(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_growm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_growm_updated ON research_growm(updated_at);

        CREATE TABLE IF NOT EXISTS research_flow (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_flow_updated ON research_flow(updated_at);

        CREATE TABLE IF NOT EXISTS research_regime (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_regime_updated ON research_regime(updated_at);

        CREATE TABLE IF NOT EXISTS research_relvol (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_relvol_updated ON research_relvol(updated_at);

        CREATE TABLE IF NOT EXISTS research_margins (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_margins_updated ON research_margins(updated_at);",
    )
    .map_err(|e| format!("create v14 tables: {e}"))?;
    Ok(())
}

pub fn upsert_growm(conn: &Connection, symbol: &str, snap: &GrowmSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v14(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("growm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_growm(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert growm: {e}"))?;
    Ok(())
}

pub fn get_growm(conn: &Connection, symbol: &str) -> Result<Option<GrowmSnapshot>, String> {
    let _ = create_research_tables_v14(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_growm WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_growm: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_growm: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_growm: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_flow(conn: &Connection, symbol: &str, snap: &FlowSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v14(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("flow json: {e}"))?;
    conn.execute(
        "INSERT INTO research_flow(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert flow: {e}"))?;
    Ok(())
}

pub fn get_flow(conn: &Connection, symbol: &str) -> Result<Option<FlowSnapshot>, String> {
    let _ = create_research_tables_v14(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_flow WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_flow: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_flow: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_flow: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_regime(conn: &Connection, symbol: &str, snap: &RegimeSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v14(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("regime json: {e}"))?;
    conn.execute(
        "INSERT INTO research_regime(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert regime: {e}"))?;
    Ok(())
}

pub fn get_regime(conn: &Connection, symbol: &str) -> Result<Option<RegimeSnapshot>, String> {
    let _ = create_research_tables_v14(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_regime WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_regime: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_regime: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_regime: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_relvol(conn: &Connection, symbol: &str, snap: &RelVolSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v14(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("relvol json: {e}"))?;
    conn.execute(
        "INSERT INTO research_relvol(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert relvol: {e}"))?;
    Ok(())
}

pub fn get_relvol(conn: &Connection, symbol: &str) -> Result<Option<RelVolSnapshot>, String> {
    let _ = create_research_tables_v14(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_relvol WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_relvol: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_relvol: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_relvol: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_margins(
    conn: &Connection,
    symbol: &str,
    snap: &MarginsSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v14(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("margins json: {e}"))?;
    conn.execute(
        "INSERT INTO research_margins(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert margins: {e}"))?;
    Ok(())
}

pub fn get_margins(conn: &Connection, symbol: &str) -> Result<Option<MarginsSnapshot>, String> {
    let _ = create_research_tables_v14(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_margins WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_margins: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_margins: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_margins: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── Godel Parity Round 15 schema + helpers ─────────────────────────

pub fn create_research_tables_v15(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_val (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_val_updated ON research_val(updated_at);

        CREATE TABLE IF NOT EXISTS research_qual (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_qual_updated ON research_qual(updated_at);

        CREATE TABLE IF NOT EXISTS research_risk (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_risk_updated ON research_risk(updated_at);

        CREATE TABLE IF NOT EXISTS research_insstrk (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_insstrk_updated ON research_insstrk(updated_at);

        CREATE TABLE IF NOT EXISTS research_covg (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_covg_updated ON research_covg(updated_at);",
    )
    .map_err(|e| format!("create v15 tables: {e}"))?;
    Ok(())
}

pub fn upsert_val(conn: &Connection, symbol: &str, snap: &ValueSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v15(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("val json: {e}"))?;
    conn.execute(
        "INSERT INTO research_val(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert val: {e}"))?;
    Ok(())
}

pub fn get_val(conn: &Connection, symbol: &str) -> Result<Option<ValueSnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_val WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_val: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_val: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_val: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_qual(conn: &Connection, symbol: &str, snap: &QualitySnapshot) -> Result<(), String> {
    let _ = create_research_tables_v15(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("qual json: {e}"))?;
    conn.execute(
        "INSERT INTO research_qual(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert qual: {e}"))?;
    Ok(())
}

pub fn get_qual(conn: &Connection, symbol: &str) -> Result<Option<QualitySnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_qual WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_qual: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_qual: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_qual: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_risk(conn: &Connection, symbol: &str, snap: &RiskSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v15(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("risk json: {e}"))?;
    conn.execute(
        "INSERT INTO research_risk(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert risk: {e}"))?;
    Ok(())
}

pub fn get_risk(conn: &Connection, symbol: &str) -> Result<Option<RiskSnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_risk WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_risk: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_risk: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_risk: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_insstrk(
    conn: &Connection,
    symbol: &str,
    snap: &InsiderStreakSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v15(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("insstrk json: {e}"))?;
    conn.execute(
        "INSERT INTO research_insstrk(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert insstrk: {e}"))?;
    Ok(())
}

pub fn get_insstrk(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<InsiderStreakSnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_insstrk WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_insstrk: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_insstrk: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_insstrk: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_covg(conn: &Connection, symbol: &str, snap: &CoverageSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v15(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("covg json: {e}"))?;
    conn.execute(
        "INSERT INTO research_covg(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert covg: {e}"))?;
    Ok(())
}

pub fn get_covg(conn: &Connection, symbol: &str) -> Result<Option<CoverageSnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_covg WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_covg: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_covg: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_covg: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── Round 16 schema + helpers ──────────────────────────────────────

pub fn create_research_tables_v16(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v15(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_vrk (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_vrk_updated ON research_vrk(updated_at);

        CREATE TABLE IF NOT EXISTS research_qrk (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_qrk_updated ON research_qrk(updated_at);

        CREATE TABLE IF NOT EXISTS research_rrk (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_rrk_updated ON research_rrk(updated_at);

        CREATE TABLE IF NOT EXISTS research_relepsgr (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_relepsgr_updated ON research_relepsgr(updated_at);

        CREATE TABLE IF NOT EXISTS research_pead (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_pead_updated ON research_pead(updated_at);",
    )
    .map_err(|e| format!("create v16 tables: {e}"))?;
    Ok(())
}

pub fn upsert_vrk(conn: &Connection, symbol: &str, snap: &ValueRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v16(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("vrk json: {e}"))?;
    conn.execute(
        "INSERT INTO research_vrk(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert vrk: {e}"))?;
    Ok(())
}

pub fn get_vrk(conn: &Connection, symbol: &str) -> Result<Option<ValueRankSnapshot>, String> {
    let _ = create_research_tables_v16(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_vrk WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_vrk: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_vrk: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_vrk: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_qrk(
    conn: &Connection,
    symbol: &str,
    snap: &QualityRankSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v16(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("qrk json: {e}"))?;
    conn.execute(
        "INSERT INTO research_qrk(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert qrk: {e}"))?;
    Ok(())
}

pub fn get_qrk(conn: &Connection, symbol: &str) -> Result<Option<QualityRankSnapshot>, String> {
    let _ = create_research_tables_v16(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_qrk WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_qrk: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_qrk: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_qrk: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_rrk(conn: &Connection, symbol: &str, snap: &RiskRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v16(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("rrk json: {e}"))?;
    conn.execute(
        "INSERT INTO research_rrk(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rrk: {e}"))?;
    Ok(())
}

pub fn get_rrk(conn: &Connection, symbol: &str) -> Result<Option<RiskRankSnapshot>, String> {
    let _ = create_research_tables_v16(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_rrk WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_rrk: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_rrk: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_rrk: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_relepsgr(
    conn: &Connection,
    symbol: &str,
    snap: &RelativeEpsGrowthSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v16(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("relepsgr json: {e}"))?;
    conn.execute(
        "INSERT INTO research_relepsgr(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert relepsgr: {e}"))?;
    Ok(())
}

pub fn get_relepsgr(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<RelativeEpsGrowthSnapshot>, String> {
    let _ = create_research_tables_v16(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_relepsgr WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_relepsgr: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_relepsgr: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_relepsgr: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_pead(conn: &Connection, symbol: &str, snap: &PeadSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v16(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("pead json: {e}"))?;
    conn.execute(
        "INSERT INTO research_pead(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert pead: {e}"))?;
    Ok(())
}

pub fn get_pead(conn: &Connection, symbol: &str) -> Result<Option<PeadSnapshot>, String> {
    let _ = create_research_tables_v16(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_pead WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_pead: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_pead: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_pead: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

/// Whole-table scan of `research_val`. Used by VRK / sector-rank surfaces.
pub fn get_all_val(conn: &Connection) -> Result<Vec<ValueSnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_val")
        .map_err(|e| format!("prepare get_all_val: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_val: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<ValueSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_qual`. Used by QRK.
pub fn get_all_qual(conn: &Connection) -> Result<Vec<QualitySnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_qual")
        .map_err(|e| format!("prepare get_all_qual: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_qual: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<QualitySnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_risk`. Used by RRK.
pub fn get_all_risk(conn: &Connection) -> Result<Vec<RiskSnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_risk")
        .map_err(|e| format!("prepare get_all_risk: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_risk: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<RiskSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}
