use super::*;
use rusqlite::{params, Connection};

// ── SQLite cache schema ────────────────────────────────────────────────────

/// Create the research_* cache tables on the given connection (idempotent).
pub fn create_research_tables(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_profile (
            symbol TEXT PRIMARY KEY,
            name TEXT NOT NULL DEFAULT '',
            exchange TEXT NOT NULL DEFAULT '',
            country TEXT NOT NULL DEFAULT '',
            currency TEXT NOT NULL DEFAULT '',
            industry TEXT NOT NULL DEFAULT '',
            sector TEXT NOT NULL DEFAULT '',
            website TEXT NOT NULL DEFAULT '',
            logo TEXT NOT NULL DEFAULT '',
            phone TEXT NOT NULL DEFAULT '',
            ipo_date TEXT NOT NULL DEFAULT '',
            description TEXT NOT NULL DEFAULT '',
            market_cap REAL NOT NULL DEFAULT 0,
            shares_outstanding REAL NOT NULL DEFAULT 0,
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_peers (
            symbol TEXT PRIMARY KEY,
            peers_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_earnings (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_press (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_sentiment (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_transcript_list (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_transcript (
            symbol TEXT NOT NULL,
            quarter INTEGER NOT NULL,
            year INTEGER NOT NULL,
            date TEXT NOT NULL DEFAULT '',
            content TEXT NOT NULL DEFAULT '',
            updated_at INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (symbol, year, quarter)
        );
        CREATE TABLE IF NOT EXISTS research_ipo_calendar (
            snapshot_at INTEGER PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]'
        );
        CREATE TABLE IF NOT EXISTS research_corporate_actions (
            symbol TEXT NOT NULL,
            date TEXT NOT NULL,
            action_type TEXT NOT NULL,
            value REAL NOT NULL DEFAULT 0,
            currency TEXT,
            note TEXT,
            updated_at INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (symbol, date, action_type)
        );
        CREATE INDEX IF NOT EXISTS idx_research_profile_updated ON research_profile(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_corporate_actions_symbol ON research_corporate_actions(symbol);",
    )
    .map_err(|e| format!("create research tables: {e}"))?;
    Ok(())
}

pub(super) fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

// ── profile (updated with description) ─────────────────────────────────────

pub fn upsert_profile(conn: &Connection, p: &CompanyProfile) -> Result<(), String> {
    let _ = create_research_tables(conn);
    conn.execute(
        "INSERT INTO research_profile
         (symbol, name, exchange, country, currency, industry, sector, website, logo, phone, ipo_date, description, market_cap, shares_outstanding, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
         ON CONFLICT(symbol) DO UPDATE SET
            name=excluded.name, exchange=excluded.exchange, country=excluded.country,
            currency=excluded.currency, industry=excluded.industry, sector=excluded.sector,
            website=excluded.website, logo=excluded.logo, phone=excluded.phone,
            ipo_date=excluded.ipo_date, description=excluded.description,
            market_cap=excluded.market_cap, shares_outstanding=excluded.shares_outstanding,
            updated_at=excluded.updated_at",
        params![
            p.symbol, p.name, p.exchange, p.country, p.currency, p.industry, p.sector,
            p.website, p.logo, p.phone, p.ipo_date, p.description, p.market_cap, p.shares_outstanding, now_ts()
        ],
    )
    .map_err(|e| format!("upsert profile: {e}"))?;
    Ok(())
}

pub fn get_profile(conn: &Connection, symbol: &str) -> Result<Option<CompanyProfile>, String> {
    let _ = create_research_tables(conn);
    let sym = symbol.to_uppercase();
    let mut stmt = conn
        .prepare(
            "SELECT symbol, name, exchange, country, currency, industry, sector, website, logo, phone, ipo_date, description, market_cap, shares_outstanding
             FROM research_profile WHERE symbol = ?1",
        )
        .map_err(|e| format!("prepare get_profile: {e}"))?;
    let mut rows = stmt
        .query(params![sym])
        .map_err(|e| format!("query get_profile: {e}"))?;
    if let Some(row) = rows.next().map_err(|e| format!("row get_profile: {e}"))? {
        Ok(Some(CompanyProfile {
            symbol: row.get(0).unwrap_or_default(),
            name: row.get(1).unwrap_or_default(),
            exchange: row.get(2).unwrap_or_default(),
            country: row.get(3).unwrap_or_default(),
            currency: row.get(4).unwrap_or_default(),
            industry: row.get(5).unwrap_or_default(),
            sector: row.get(6).unwrap_or_default(),
            website: row.get(7).unwrap_or_default(),
            logo: row.get(8).unwrap_or_default(),
            phone: row.get(9).unwrap_or_default(),
            ipo_date: row.get(10).unwrap_or_default(),
            description: row.get(11).unwrap_or_default(),
            market_cap: row.get(12).unwrap_or(0.0),
            shares_outstanding: row.get(13).unwrap_or(0.0),
        }))
    } else {
        Ok(None)
    }
}

// ── corporate actions (#2) ────────────────────────────────────────────────

pub fn upsert_corporate_action(conn: &Connection, ca: &CorporateAction) -> Result<(), String> {
    let _ = create_research_tables(conn);
    conn.execute(
        "INSERT INTO research_corporate_actions
         (symbol, date, action_type, value, currency, note, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(symbol, date, action_type) DO UPDATE SET
            value=excluded.value, currency=excluded.currency, note=excluded.note, updated_at=excluded.updated_at",
        params![
            ca.symbol.to_uppercase(),
            ca.date,
            ca.action_type,
            ca.value,
            ca.currency,
            ca.note,
            now_ts()
        ],
    )
    .map_err(|e| format!("upsert corporate_action: {e}"))?;
    Ok(())
}

pub fn get_corporate_actions(conn: &Connection, symbol: &str) -> Result<Vec<CorporateAction>, String> {
    let _ = create_research_tables(conn);
    let sym = symbol.to_uppercase();
    let mut stmt = conn
        .prepare(
            "SELECT symbol, date, action_type, value, currency, note
             FROM research_corporate_actions WHERE symbol = ?1 ORDER BY date DESC",
        )
        .map_err(|e| format!("prepare get_corporate_actions: {e}"))?;
    let rows = stmt
        .query_map(params![sym], |row| {
            Ok(CorporateAction {
                symbol: row.get(0)?,
                date: row.get(1)?,
                action_type: row.get(2)?,
                value: row.get(3)?,
                currency: row.get(4)?,
                note: row.get(5)?,
            })
        })
        .map_err(|e| format!("query corporate_actions: {e}"))?;

    let mut actions = Vec::new();
    for row in rows {
        actions.push(row.map_err(|e| format!("row corporate_action: {e}"))?);
    }
    Ok(actions)
}