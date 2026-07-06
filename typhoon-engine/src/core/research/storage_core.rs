use super::*;

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
            employees REAL NOT NULL DEFAULT 0,
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
        CREATE TABLE IF NOT EXISTS research_stocktwits_sentiment (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_reddit_mentions (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_social_history (
            symbol TEXT NOT NULL,
            source TEXT NOT NULL,
            fetched_at_ts INTEGER NOT NULL,
            bullish INTEGER NOT NULL DEFAULT 0,
            bearish INTEGER NOT NULL DEFAULT 0,
            neutral INTEGER NOT NULL DEFAULT 0,
            messages INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY(symbol, source, fetched_at_ts)
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
        CREATE INDEX IF NOT EXISTS idx_research_peers_updated ON research_peers(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_earnings_updated ON research_earnings(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_corporate_actions_symbol ON research_corporate_actions(symbol);",
    )
    .map_err(|e| format!("create research tables: {e}"))?;
    ensure_column(
        conn,
        "research_profile",
        "description",
        "ALTER TABLE research_profile ADD COLUMN description TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        conn,
        "research_profile",
        "employees",
        "ALTER TABLE research_profile ADD COLUMN employees REAL NOT NULL DEFAULT 0",
    )?;
    Ok(())
}

pub(super) fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    alter_sql: &str,
) -> Result<(), String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| format!("prepare table_info {table}: {e}"))?;
    let mut rows = stmt
        .query([])
        .map_err(|e| format!("query table_info {table}: {e}"))?;
    while let Some(row) = rows
        .next()
        .map_err(|e| format!("row table_info {table}: {e}"))?
    {
        let name: String = row.get(1).unwrap_or_default();
        if name == column {
            return Ok(());
        }
    }
    conn.execute_batch(alter_sql)
        .map_err(|e| format!("alter {table} add {column}: {e}"))?;
    Ok(())
}

// ── profile ────────────────────────────────────────────────────────────────

pub fn upsert_profile(conn: &Connection, p: &CompanyProfile) -> Result<(), String> {
    let _ = create_research_tables(conn);
    conn.execute(
        "INSERT INTO research_profile
         (symbol, name, exchange, country, currency, industry, sector, website, logo, phone, ipo_date, description, market_cap, shares_outstanding, employees, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)
         ON CONFLICT(symbol) DO UPDATE SET
            name=excluded.name, exchange=excluded.exchange, country=excluded.country,
            currency=excluded.currency, industry=excluded.industry, sector=excluded.sector,
            website=excluded.website, logo=excluded.logo, phone=excluded.phone,
            ipo_date=excluded.ipo_date, description=excluded.description, market_cap=excluded.market_cap,
            shares_outstanding=excluded.shares_outstanding, employees=excluded.employees,
            updated_at=excluded.updated_at",
        params![
            p.symbol.to_uppercase(), p.name, p.exchange, p.country, p.currency,
            p.industry, p.sector, p.website, p.logo, p.phone, p.ipo_date,
            p.description, p.market_cap, p.shares_outstanding, p.employees, now_ts(),
        ],
    ).map_err(|e| format!("upsert profile: {e}"))?;
    Ok(())
}

pub fn get_profile(conn: &Connection, symbol: &str) -> Result<Option<CompanyProfile>, String> {
    let _ = create_research_tables(conn);
    let sym = symbol.to_uppercase();
    let mut stmt = conn.prepare(
        "SELECT symbol, name, exchange, country, currency, industry, sector, website, logo, phone, ipo_date, description, market_cap, shares_outstanding, employees
         FROM research_profile WHERE symbol = ?1"
    ).map_err(|e| format!("prepare get_profile: {e}"))?;
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
            employees: row.get(14).unwrap_or(0.0),
        }))
    } else {
        Ok(None)
    }
}

// ── corporate actions ──────────────────────────────────────────────────────

pub fn upsert_corporate_action(conn: &Connection, ca: &CorporateAction) -> Result<(), String> {
    let _ = create_research_tables(conn);
    conn.execute(
        "INSERT INTO research_corporate_actions
         (symbol, date, action_type, value, currency, note, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)
         ON CONFLICT(symbol, date, action_type) DO UPDATE SET
            value=excluded.value, currency=excluded.currency, note=excluded.note, updated_at=excluded.updated_at",
        params![
            ca.symbol.to_uppercase(),
            ca.date,
            ca.action_type,
            ca.value,
            ca.currency,
            ca.note,
            now_ts(),
        ],
    )
    .map_err(|e| format!("upsert corporate action: {e}"))?;
    Ok(())
}

pub fn get_corporate_actions(
    conn: &Connection,
    symbol: &str,
) -> Result<Vec<CorporateAction>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn
        .prepare(
            "SELECT symbol, date, action_type, value, currency, note
             FROM research_corporate_actions
             WHERE symbol = ?1
             ORDER BY date DESC",
        )
        .map_err(|e| format!("prepare get_corporate_actions: {e}"))?;
    let rows = stmt
        .query_map(params![symbol.to_uppercase()], |row| {
            Ok(CorporateAction {
                symbol: row.get(0)?,
                date: row.get(1)?,
                action_type: row.get(2)?,
                value: row.get(3)?,
                currency: row.get(4)?,
                note: row.get(5)?,
            })
        })
        .map_err(|e| format!("query get_corporate_actions: {e}"))?;
    let mut actions = Vec::new();
    for row in rows {
        actions.push(row.map_err(|e| format!("row corporate action: {e}"))?);
    }
    Ok(actions)
}

// ── peers ──────────────────────────────────────────────────────────────────

pub fn upsert_peers(conn: &Connection, symbol: &str, peers: &[String]) -> Result<(), String> {
    let _ = create_research_tables(conn);
    let json = serde_json::to_string(peers).map_err(|e| format!("peers json: {e}"))?;
    conn.execute(
        "INSERT INTO research_peers(symbol, peers_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET peers_json=excluded.peers_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert peers: {e}"))?;
    Ok(())
}

pub fn get_peers(conn: &Connection, symbol: &str) -> Result<Option<Vec<String>>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn
        .prepare("SELECT peers_json FROM research_peers WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_peers: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_peers: {e}"))?;
    if let Some(row) = rows.next().map_err(|e| format!("row get_peers: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        let peers: Vec<String> = serde_json::from_str(&json).unwrap_or_default();
        Ok(Some(peers))
    } else {
        Ok(None)
    }
}

// ── earnings history ───────────────────────────────────────────────────────

pub fn upsert_earnings_history(
    conn: &Connection,
    symbol: &str,
    rows: &[EarningRow],
) -> Result<(), String> {
    let _ = create_research_tables(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("earnings json: {e}"))?;
    conn.execute(
        "INSERT INTO research_earnings(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert earnings: {e}"))?;
    Ok(())
}

pub fn get_earnings_history(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<Vec<EarningRow>>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn
        .prepare("SELECT rows_json FROM research_earnings WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_earnings: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_earnings: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_earnings: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        let rows: Vec<EarningRow> = serde_json::from_str(&json).unwrap_or_default();
        Ok(Some(rows))
    } else {
        Ok(None)
    }
}

// ── press releases ─────────────────────────────────────────────────────────

pub fn upsert_press_releases(
    conn: &Connection,
    symbol: &str,
    rows: &[PressRelease],
) -> Result<(), String> {
    let _ = create_research_tables(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("press json: {e}"))?;
    conn.execute(
        "INSERT INTO research_press(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert press: {e}"))?;
    Ok(())
}

pub fn get_press_releases(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<Vec<PressRelease>>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn
        .prepare("SELECT rows_json FROM research_press WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_press: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_press: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_press: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        let rows: Vec<PressRelease> = serde_json::from_str(&json).unwrap_or_default();
        Ok(Some(rows))
    } else {
        Ok(None)
    }
}

// ── social sentiment ───────────────────────────────────────────────────────

pub fn upsert_sentiment(
    conn: &Connection,
    symbol: &str,
    rows: &[SocialSentimentRow],
) -> Result<(), String> {
    let _ = create_research_tables(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("sentiment json: {e}"))?;
    conn.execute(
        "INSERT INTO research_sentiment(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert sentiment: {e}"))?;
    Ok(())
}

pub fn get_sentiment(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<Vec<SocialSentimentRow>>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn
        .prepare("SELECT rows_json FROM research_sentiment WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_sentiment: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_sentiment: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_sentiment: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        let rows: Vec<SocialSentimentRow> = serde_json::from_str(&json).unwrap_or_default();
        Ok(Some(rows))
    } else {
        Ok(None)
    }
}

pub fn upsert_stocktwits_sentiment(
    conn: &Connection,
    symbol: &str,
    snapshot: &StockTwitsSentimentSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables(conn);
    let mut normalized = snapshot.clone();
    normalized.symbol = symbol.to_uppercase();
    let json = serde_json::to_string(&normalized).map_err(|e| format!("stocktwits json: {e}"))?;
    conn.execute(
        "INSERT INTO research_stocktwits_sentiment(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert stocktwits sentiment: {e}"))?;
    append_social_history(
        conn,
        symbol,
        "stocktwits",
        snapshot.bullish,
        snapshot.bearish,
        snapshot.neutral,
        snapshot.message_count,
    )
}

/// Bound per (symbol, source) so the local history series never grows without
/// limit (ADR-121 retention discipline applied to the social lane).
const SOCIAL_HISTORY_KEEP: i64 = 500;

/// Append one point to the local social-history series (drives the ADR-117
/// bull/bear + mention sparkline) and prune to the retention bound.
pub fn append_social_history(
    conn: &Connection,
    symbol: &str,
    source: &str,
    bullish: u32,
    bearish: u32,
    neutral: u32,
    messages: u32,
) -> Result<(), String> {
    let _ = create_research_tables(conn);
    let symbol = symbol.to_uppercase();
    conn.execute(
        "INSERT OR REPLACE INTO research_social_history
         (symbol, source, fetched_at_ts, bullish, bearish, neutral, messages)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            symbol,
            source,
            now_ts(),
            bullish,
            bearish,
            neutral,
            messages
        ],
    )
    .map_err(|e| format!("append social history: {e}"))?;
    conn.execute(
        "DELETE FROM research_social_history
         WHERE symbol = ?1 AND source = ?2 AND fetched_at_ts NOT IN (
             SELECT fetched_at_ts FROM research_social_history
             WHERE symbol = ?1 AND source = ?2
             ORDER BY fetched_at_ts DESC LIMIT ?3
         )",
        params![symbol, source, SOCIAL_HISTORY_KEEP],
    )
    .map_err(|e| format!("prune social history: {e}"))?;
    Ok(())
}

/// Stored social-history points for a symbol, oldest-first, across sources.
pub fn get_social_history(
    conn: &Connection,
    symbol: &str,
    limit: usize,
) -> Result<Vec<SocialHistoryPoint>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn
        .prepare(
            "SELECT symbol, source, fetched_at_ts, bullish, bearish, neutral, messages
             FROM research_social_history WHERE symbol = ?1
             ORDER BY fetched_at_ts DESC LIMIT ?2",
        )
        .map_err(|e| format!("prepare social history: {e}"))?;
    let mut rows: Vec<SocialHistoryPoint> = stmt
        .query_map(params![symbol.to_uppercase(), limit as i64], |row| {
            Ok(SocialHistoryPoint {
                symbol: row.get(0)?,
                source: row.get(1)?,
                fetched_at_ts: row.get(2)?,
                bullish: row.get::<_, i64>(3)? as u32,
                bearish: row.get::<_, i64>(4)? as u32,
                neutral: row.get::<_, i64>(5)? as u32,
                messages: row.get::<_, i64>(6)? as u32,
            })
        })
        .map_err(|e| format!("query social history: {e}"))?
        .filter_map(|r| r.ok())
        .collect();
    rows.reverse();
    Ok(rows)
}

pub fn upsert_reddit_mentions(
    conn: &Connection,
    symbol: &str,
    snapshot: &RedditMentionSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables(conn);
    let mut normalized = snapshot.clone();
    normalized.symbol = symbol.to_uppercase();
    let json = serde_json::to_string(&normalized).map_err(|e| format!("reddit json: {e}"))?;
    conn.execute(
        "INSERT INTO research_reddit_mentions(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert reddit mentions: {e}"))?;
    // Reddit has no bull/bear tags — history carries mention count only.
    append_social_history(conn, symbol, "reddit", 0, 0, 0, snapshot.mentions_24h)
}

pub fn get_reddit_mentions(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<RedditMentionSnapshot>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_reddit_mentions WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_reddit_mentions: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_reddit_mentions: {e}"))?;
    if let Some(row) = rows
        .next()
        .map_err(|e| format!("row get_reddit_mentions: {e}"))?
    {
        let json: String = row.get(0).unwrap_or_default();
        let snapshot: RedditMentionSnapshot = serde_json::from_str(&json).unwrap_or_default();
        Ok(Some(snapshot))
    } else {
        Ok(None)
    }
}

pub fn get_stocktwits_sentiment(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<StockTwitsSentimentSnapshot>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_stocktwits_sentiment WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_stocktwits_sentiment: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_stocktwits_sentiment: {e}"))?;
    if let Some(row) = rows
        .next()
        .map_err(|e| format!("row get_stocktwits_sentiment: {e}"))?
    {
        let json: String = row.get(0).unwrap_or_default();
        let snapshot: StockTwitsSentimentSnapshot = serde_json::from_str(&json).unwrap_or_default();
        Ok(Some(snapshot))
    } else {
        Ok(None)
    }
}

// ── transcripts ────────────────────────────────────────────────────────────

pub fn upsert_transcript_list(
    conn: &Connection,
    symbol: &str,
    rows: &[TranscriptMeta],
) -> Result<(), String> {
    let _ = create_research_tables(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("transcript list json: {e}"))?;
    conn.execute(
        "INSERT INTO research_transcript_list(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert transcript list: {e}"))?;
    Ok(())
}

pub fn get_transcript_list(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<Vec<TranscriptMeta>>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn
        .prepare("SELECT rows_json FROM research_transcript_list WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_tlist: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_tlist: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_tlist: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_transcript(conn: &Connection, t: &Transcript) -> Result<(), String> {
    let _ = create_research_tables(conn);
    conn.execute(
        "INSERT INTO research_transcript(symbol, quarter, year, date, content, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6)
         ON CONFLICT(symbol, year, quarter) DO UPDATE SET
            date=excluded.date, content=excluded.content, updated_at=excluded.updated_at",
        params![
            t.symbol.to_uppercase(),
            t.quarter,
            t.year,
            t.date,
            t.content,
            now_ts()
        ],
    )
    .map_err(|e| format!("upsert transcript: {e}"))?;
    Ok(())
}

pub fn get_transcript(
    conn: &Connection,
    symbol: &str,
    quarter: i32,
    year: i32,
) -> Result<Option<Transcript>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn
        .prepare(
            "SELECT symbol, quarter, year, date, content FROM research_transcript
         WHERE symbol = ?1 AND year = ?2 AND quarter = ?3",
        )
        .map_err(|e| format!("prepare get_transcript: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase(), year, quarter])
        .map_err(|e| format!("query get_transcript: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_transcript: {e}"))? {
        Ok(Some(Transcript {
            symbol: row.get(0).unwrap_or_default(),
            quarter: row.get(1).unwrap_or(0),
            year: row.get(2).unwrap_or(0),
            date: row.get(3).unwrap_or_default(),
            content: row.get(4).unwrap_or_default(),
        }))
    } else {
        Ok(None)
    }
}

fn truncate_text(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max_chars).collect();
    out.push('…');
    out
}

fn transcript_paragraphs(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(|p| {
            p.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .map(|p| p.trim().to_string())
        .filter(|p| p.len() > 60)
        .filter(|p| {
            let lower = p.to_ascii_lowercase();
            !(lower.starts_with("operator")
                || lower.starts_with("conference call participants")
                || lower.starts_with("company participants")
                || lower.starts_with("analysts"))
        })
        .collect()
}

fn split_prepared_and_qa(content: &str) -> (&str, Option<&str>) {
    let lower = content.to_ascii_lowercase();
    let markers = [
        "question-and-answer session",
        "question and answer session",
        "questions and answers",
        "\nq&a",
        "\nqa",
    ];
    for marker in markers {
        if let Some(idx) = lower.find(marker) {
            return (&content[..idx], Some(&content[idx..]));
        }
    }
    (content, None)
}

/// Deterministic earnings-call transcript summarizer used by the native
/// TRANSCRIPTS window. This intentionally stays local/offline: it extracts
/// substantial prepared-remarks paragraphs and a small Q&A sample without
/// invoking an LLM or adding a provider dependency.
pub fn summarize_transcript(t: &Transcript) -> crate::core::sec_filing::FilingSummary {
    use crate::core::sec_filing::{FilingSection, FilingSummary};

    let mut summary = FilingSummary {
        headline: format!(
            "{} Q{} {} earnings call transcript",
            t.symbol, t.quarter, t.year
        ),
        ..Default::default()
    };
    let content = t.content.trim();
    if content.is_empty() {
        summary
            .bullets
            .push("Transcript body is empty.".to_string());
        return summary;
    }

    let (prepared, qa) = split_prepared_and_qa(content);
    let prepared_paras = transcript_paragraphs(prepared);
    let qa_paras = qa.map(transcript_paragraphs).unwrap_or_default();

    if !prepared_paras.is_empty() {
        let body = prepared_paras
            .iter()
            .take(3)
            .map(|p| truncate_text(p, 700))
            .collect::<Vec<_>>()
            .join("\n\n");
        summary.sections.push(FilingSection {
            title: "Prepared remarks".to_string(),
            body,
        });
    }
    if !qa_paras.is_empty() {
        let body = qa_paras
            .iter()
            .take(3)
            .map(|p| truncate_text(p, 700))
            .collect::<Vec<_>>()
            .join("\n\n");
        summary.sections.push(FilingSection {
            title: "Q&A".to_string(),
            body,
        });
    }

    for p in prepared_paras.iter().chain(qa_paras.iter()).take(5) {
        summary.bullets.push(truncate_text(p, 220));
    }
    if summary.bullets.is_empty() {
        for p in transcript_paragraphs(content).into_iter().take(3) {
            summary.bullets.push(truncate_text(&p, 220));
        }
    }
    if summary.sections.is_empty() && !summary.bullets.is_empty() {
        summary.sections.push(FilingSection {
            title: "Transcript extract".to_string(),
            body: summary.bullets.join("\n\n"),
        });
    }
    summary.headline = format!(
        "{} - {} extract(s)",
        summary.headline,
        summary.sections.len()
    );
    summary
}

// ── IPO calendar ───────────────────────────────────────────────────────────

pub fn upsert_ipo_calendar(conn: &Connection, rows: &[IpoEvent]) -> Result<(), String> {
    let _ = create_research_tables(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("ipo json: {e}"))?;
    conn.execute("DELETE FROM research_ipo_calendar", [])
        .map_err(|e| format!("ipo delete: {e}"))?;
    conn.execute(
        "INSERT INTO research_ipo_calendar(snapshot_at, rows_json) VALUES (?1,?2)",
        params![now_ts(), json],
    )
    .map_err(|e| format!("upsert ipo: {e}"))?;
    Ok(())
}

pub fn get_ipo_calendar(conn: &Connection) -> Result<Option<Vec<IpoEvent>>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn
        .prepare("SELECT rows_json FROM research_ipo_calendar ORDER BY snapshot_at DESC LIMIT 1")
        .map_err(|e| format!("prepare get_ipo: {e}"))?;
    let mut r = stmt.query([]).map_err(|e| format!("query get_ipo: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ipo: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod stocktwits_storage_tests {
    use super::*;

    #[test]
    fn stocktwits_sentiment_roundtrips_by_uppercase_symbol() {
        let conn = Connection::open_in_memory().unwrap();
        let snapshot = StockTwitsSentimentSnapshot {
            symbol: "AMC".to_string(),
            bullish: 2,
            bearish: 1,
            neutral: 1,
            message_count: 4,
            bull_bear_ratio: 2.0,
            top_messages: vec![StockTwitsMessage {
                id: 7,
                username: "ape".to_string(),
                body: "Watching".to_string(),
                sentiment: "Bullish".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };

        upsert_stocktwits_sentiment(&conn, "amc", &snapshot).unwrap();
        let loaded = get_stocktwits_sentiment(&conn, "AMC").unwrap().unwrap();

        assert_eq!(loaded.symbol, "AMC");
        assert_eq!(loaded.bullish, 2);
        assert_eq!(loaded.top_messages[0].username, "ape");
    }
}
