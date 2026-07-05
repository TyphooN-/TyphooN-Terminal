use super::*;

pub fn create_research_tables_v2(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_dividends (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_earnings_estimates (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_rating_changes (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_dividends_updated ON research_dividends(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_estimates_updated ON research_earnings_estimates(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_rating_changes_updated ON research_rating_changes(updated_at);"
    ).map_err(|e| format!("create research_v2 tables: {e}"))?;
    Ok(())
}

pub fn upsert_dividends(
    conn: &Connection,
    symbol: &str,
    rows: &[DividendRecord],
) -> Result<(), String> {
    let _ = create_research_tables_v2(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("div json: {e}"))?;
    conn.execute(
        "INSERT INTO research_dividends(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert dividends: {e}"))?;
    Ok(())
}

pub fn get_dividends(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<Vec<DividendRecord>>, String> {
    let _ = create_research_tables_v2(conn);
    let mut stmt = conn
        .prepare("SELECT rows_json FROM research_dividends WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_dividends: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_dividends: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_dividends: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_earnings_estimates(
    conn: &Connection,
    symbol: &str,
    rows: &[EarningsEstimate],
) -> Result<(), String> {
    let _ = create_research_tables_v2(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("estimates json: {e}"))?;
    conn.execute(
        "INSERT INTO research_earnings_estimates(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert estimates: {e}"))?;
    Ok(())
}

pub fn get_earnings_estimates(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<Vec<EarningsEstimate>>, String> {
    let _ = create_research_tables_v2(conn);
    let mut stmt = conn
        .prepare("SELECT rows_json FROM research_earnings_estimates WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_estimates: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_estimates: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_estimates: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_rating_changes(
    conn: &Connection,
    symbol: &str,
    rows: &[RatingChange],
) -> Result<(), String> {
    let _ = create_research_tables_v2(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("rating changes json: {e}"))?;
    conn.execute(
        "INSERT INTO research_rating_changes(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rating changes: {e}"))?;
    Ok(())
}

pub fn get_rating_changes(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<Vec<RatingChange>>, String> {
    let _ = create_research_tables_v2(conn);
    let mut stmt = conn
        .prepare("SELECT rows_json FROM research_rating_changes WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_rating_changes: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_rating_changes: {e}"))?;
    if let Some(row) = r
        .next()
        .map_err(|e| format!("row get_rating_changes: {e}"))?
    {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── SQLite schema + helpers ────────────────────────────────────────

pub fn create_research_tables_v3(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_financials (
            symbol TEXT PRIMARY KEY,
            bundle_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_executives (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_financials_updated ON research_financials(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_executives_updated ON research_executives(updated_at);"
    ).map_err(|e| format!("create research_v3 tables: {e}"))?;
    Ok(())
}

pub fn upsert_financials(
    conn: &Connection,
    symbol: &str,
    bundle: &FinancialStatements,
) -> Result<(), String> {
    let _ = create_research_tables_v3(conn);
    let json = serde_json::to_string(bundle).map_err(|e| format!("financials json: {e}"))?;
    conn.execute(
        "INSERT INTO research_financials(symbol, bundle_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET bundle_json=excluded.bundle_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert financials: {e}"))?;
    Ok(())
}

pub fn get_financials(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<FinancialStatements>, String> {
    let _ = create_research_tables_v3(conn);
    let mut stmt = conn
        .prepare("SELECT bundle_json FROM research_financials WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_financials: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_financials: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_financials: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_executives(
    conn: &Connection,
    symbol: &str,
    rows: &[Executive],
) -> Result<(), String> {
    let _ = create_research_tables_v3(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("executives json: {e}"))?;
    conn.execute(
        "INSERT INTO research_executives(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert executives: {e}"))?;
    Ok(())
}

pub fn get_executives(conn: &Connection, symbol: &str) -> Result<Option<Vec<Executive>>, String> {
    let _ = create_research_tables_v3(conn);
    let mut stmt = conn
        .prepare("SELECT rows_json FROM research_executives WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_executives: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_executives: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_executives: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── SQLite schema + helpers ────────────────────────────────────────

pub fn create_research_tables_v4(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_stock_splits (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_etf_holdings (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_analyst_recs (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_price_target (
            symbol TEXT PRIMARY KEY,
            target_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_esg (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_index_members (
            index_code TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_stock_splits_updated ON research_stock_splits(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_etf_holdings_updated ON research_etf_holdings(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_analyst_recs_updated ON research_analyst_recs(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_price_target_updated ON research_price_target(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_esg_updated ON research_esg(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_index_members_updated ON research_index_members(updated_at);"
    ).map_err(|e| format!("create research_v4 tables: {e}"))?;
    Ok(())
}

pub fn upsert_stock_splits(
    conn: &Connection,
    symbol: &str,
    rows: &[StockSplit],
) -> Result<(), String> {
    let _ = create_research_tables_v4(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("splits json: {e}"))?;
    conn.execute(
        "INSERT INTO research_stock_splits(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert stock_splits: {e}"))?;
    Ok(())
}

pub fn get_stock_splits(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<Vec<StockSplit>>, String> {
    let _ = create_research_tables_v4(conn);
    let mut stmt = conn
        .prepare("SELECT rows_json FROM research_stock_splits WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_splits: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_splits: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_splits: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

/// Days a split's ex-date may lie in the past and still warrant a bar-cache
/// purge. An action older than this is already baked into every provider's
/// adjusted history (Alpaca `adjustment=all`, Yahoo adjclose), so discovering
/// it during a bulk scrape must NOT trigger a pointless delete+refetch of the
/// whole equity cache. Recent actions are the ones whose pre-split rows can
/// still be stale or carry-poisoned in the cache.
pub const SPLIT_INVALIDATION_MAX_AGE_DAYS: i64 = 730;

/// Whether a freshly-fetched split set contains a material corporate action —
/// cumulative price factor ≥ 2× in EITHER direction (reverse or forward) — with
/// a recent ex-date that the previously-stored set did not already know. Such a
/// discovery means the cached bars may predate the provider's restatement and
/// must be rebuilt cleanly rather than incrementally merged (incremental merge
/// keeps stale pre-split rows and can retain ex-date carry poison forever).
///
/// Already-known splits, sub-2× actions, and actions whose ex-date is older
/// than [`SPLIT_INVALIDATION_MAX_AGE_DAYS`] (or in the future) do not qualify —
/// the first so re-scrapes are idempotent, the last so a bulk scrape populating
/// years of historical splits doesn't purge the entire cache.
pub fn stock_splits_need_bar_cache_invalidation(
    existing: &[StockSplit],
    incoming: &[StockSplit],
) -> bool {
    stock_splits_need_bar_cache_invalidation_at(existing, incoming, chrono::Utc::now())
}

fn stock_splits_need_bar_cache_invalidation_at(
    existing: &[StockSplit],
    incoming: &[StockSplit],
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    incoming.iter().any(|split| {
        if split.numerator <= 0.0 || split.denominator <= 0.0 {
            return false;
        }
        let factor =
            (split.denominator / split.numerator).max(split.numerator / split.denominator);
        if factor < 2.0 {
            return false;
        }
        let Some(ex) = chrono::NaiveDate::parse_from_str(split.date.trim(), "%Y-%m-%d")
            .ok()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| dt.and_utc())
        else {
            return false;
        };
        let age_days = now.signed_duration_since(ex).num_days();
        if !(0..=SPLIT_INVALIDATION_MAX_AGE_DAYS).contains(&age_days) {
            return false;
        }
        !existing.iter().any(|old| {
            old.date == split.date
                && (old.numerator - split.numerator).abs() < 1e-9
                && (old.denominator - split.denominator).abs() < 1e-9
        })
    })
}

pub fn upsert_etf_holdings(
    conn: &Connection,
    symbol: &str,
    rows: &[EtfHolding],
) -> Result<(), String> {
    let _ = create_research_tables_v4(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("etf holdings json: {e}"))?;
    conn.execute(
        "INSERT INTO research_etf_holdings(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert etf holdings: {e}"))?;
    Ok(())
}

pub fn get_etf_holdings(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<Vec<EtfHolding>>, String> {
    let _ = create_research_tables_v4(conn);
    let mut stmt = conn
        .prepare("SELECT rows_json FROM research_etf_holdings WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_etf_holdings: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_etf_holdings: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_etf_holdings: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_analyst_recs(
    conn: &Connection,
    symbol: &str,
    rows: &[AnalystRecommendation],
) -> Result<(), String> {
    let _ = create_research_tables_v4(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("analyst recs json: {e}"))?;
    conn.execute(
        "INSERT INTO research_analyst_recs(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert analyst_recs: {e}"))?;
    Ok(())
}

pub fn get_analyst_recs(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<Vec<AnalystRecommendation>>, String> {
    let _ = create_research_tables_v4(conn);
    let mut stmt = conn
        .prepare("SELECT rows_json FROM research_analyst_recs WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_analyst_recs: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_analyst_recs: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_analyst_recs: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_price_target(
    conn: &Connection,
    symbol: &str,
    pt: &PriceTarget,
) -> Result<(), String> {
    let _ = create_research_tables_v4(conn);
    let json = serde_json::to_string(pt).map_err(|e| format!("price target json: {e}"))?;
    conn.execute(
        "INSERT INTO research_price_target(symbol, target_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET target_json=excluded.target_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert price_target: {e}"))?;
    Ok(())
}

pub fn get_price_target(conn: &Connection, symbol: &str) -> Result<Option<PriceTarget>, String> {
    let _ = create_research_tables_v4(conn);
    let mut stmt = conn
        .prepare("SELECT target_json FROM research_price_target WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_price_target: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_price_target: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_price_target: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_esg(conn: &Connection, symbol: &str, rows: &[EsgScore]) -> Result<(), String> {
    let _ = create_research_tables_v4(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("esg json: {e}"))?;
    conn.execute(
        "INSERT INTO research_esg(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert esg: {e}"))?;
    Ok(())
}

pub fn get_esg(conn: &Connection, symbol: &str) -> Result<Option<Vec<EsgScore>>, String> {
    let _ = create_research_tables_v4(conn);
    let mut stmt = conn
        .prepare("SELECT rows_json FROM research_esg WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_esg: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_esg: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_esg: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_index_members(
    conn: &Connection,
    index_code: &str,
    rows: &[IndexMember],
) -> Result<(), String> {
    let _ = create_research_tables_v4(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("index members json: {e}"))?;
    conn.execute(
        "INSERT INTO research_index_members(index_code, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(index_code) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![index_code.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert index_members: {e}"))?;
    Ok(())
}

pub fn get_index_members(
    conn: &Connection,
    index_code: &str,
) -> Result<Option<Vec<IndexMember>>, String> {
    let _ = create_research_tables_v4(conn);
    let mut stmt = conn
        .prepare("SELECT rows_json FROM research_index_members WHERE index_code = ?1")
        .map_err(|e| format!("prepare get_index_members: {e}"))?;
    let mut r = stmt
        .query(params![index_code.to_uppercase()])
        .map_err(|e| format!("query get_index_members: {e}"))?;
    if let Some(row) = r
        .next()
        .map_err(|e| format!("row get_index_members: {e}"))?
    {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── Research section ──

pub fn create_research_tables_v5(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_insider_trades (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_institutional_holders (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_shares_float (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_historical_price (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_earnings_surprise (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_insider_trades_updated ON research_insider_trades(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_institutional_holders_updated ON research_institutional_holders(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_shares_float_updated ON research_shares_float(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_historical_price_updated ON research_historical_price(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_earnings_surprise_updated ON research_earnings_surprise(updated_at);"
    ).map_err(|e| format!("create research_v5 tables: {e}"))?;
    Ok(())
}

pub fn upsert_insider_trades(
    conn: &Connection,
    symbol: &str,
    rows: &[InsiderTrade],
) -> Result<(), String> {
    let _ = create_research_tables_v5(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("insider json: {e}"))?;
    conn.execute(
        "INSERT INTO research_insider_trades(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert insider: {e}"))?;
    Ok(())
}

pub fn get_insider_trades(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<Vec<InsiderTrade>>, String> {
    let _ = create_research_tables_v5(conn);
    let mut stmt = conn
        .prepare("SELECT rows_json FROM research_insider_trades WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_insider: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_insider: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_insider: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_institutional_holders(
    conn: &Connection,
    symbol: &str,
    rows: &[InstitutionalHolder],
) -> Result<(), String> {
    let _ = create_research_tables_v5(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("holders json: {e}"))?;
    conn.execute(
        "INSERT INTO research_institutional_holders(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert holders: {e}"))?;
    Ok(())
}

pub fn get_institutional_holders(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<Vec<InstitutionalHolder>>, String> {
    let _ = create_research_tables_v5(conn);
    let mut stmt = conn
        .prepare("SELECT rows_json FROM research_institutional_holders WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_holders: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_holders: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_holders: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_shares_float(
    conn: &Connection,
    symbol: &str,
    snap: &SharesFloat,
) -> Result<(), String> {
    let _ = create_research_tables_v5(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("float json: {e}"))?;
    conn.execute(
        "INSERT INTO research_shares_float(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert float: {e}"))?;
    Ok(())
}

pub fn get_shares_float(conn: &Connection, symbol: &str) -> Result<Option<SharesFloat>, String> {
    let _ = create_research_tables_v5(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_shares_float WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_float: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_float: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_float: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_historical_price(
    conn: &Connection,
    symbol: &str,
    rows: &[HistoricalPriceRow],
) -> Result<(), String> {
    let _ = create_research_tables_v5(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("hp json: {e}"))?;
    conn.execute(
        "INSERT INTO research_historical_price(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert hp: {e}"))?;
    Ok(())
}

pub fn get_historical_price(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<Vec<HistoricalPriceRow>>, String> {
    let _ = create_research_tables_v5(conn);
    let mut stmt = conn
        .prepare("SELECT rows_json FROM research_historical_price WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_hp: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_hp: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_hp: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_earnings_surprises(
    conn: &Connection,
    symbol: &str,
    rows: &[EarningsSurprise],
) -> Result<(), String> {
    let _ = create_research_tables_v5(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("surprise json: {e}"))?;
    conn.execute(
        "INSERT INTO research_earnings_surprise(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert surprise: {e}"))?;
    Ok(())
}

pub fn get_earnings_surprises(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<Vec<EarningsSurprise>>, String> {
    let _ = create_research_tables_v5(conn);
    let mut stmt = conn
        .prepare("SELECT rows_json FROM research_earnings_surprise WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_surprise: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_surprise: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_surprise: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod split_invalidation_tests {
    use super::*;

    fn split(date: &str, numerator: f64, denominator: f64) -> StockSplit {
        StockSplit {
            date: date.to_string(),
            label: format!("{numerator}:{denominator}"),
            numerator,
            denominator,
        }
    }

    fn now() -> chrono::DateTime<chrono::Utc> {
        chrono::NaiveDate::from_ymd_opt(2026, 7, 5)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
    }

    fn needs(existing: &[StockSplit], incoming: &[StockSplit]) -> bool {
        stock_splits_need_bar_cache_invalidation_at(existing, incoming, now())
    }

    #[test]
    fn recent_reverse_split_newly_discovered_invalidates() {
        assert!(needs(&[], &[split("2026-06-19", 1.0, 40.0)]));
    }

    #[test]
    fn recent_forward_split_invalidates_symmetrically() {
        // 4-for-1 forward split (BFOR-class) drops price 4× — the raw cache is
        // just as stale as after a reverse split, so it must invalidate too.
        assert!(needs(&[], &[split("2026-07-01", 4.0, 1.0)]));
    }

    #[test]
    fn already_known_split_is_idempotent() {
        let existing = [split("2026-06-19", 1.0, 40.0)];
        assert!(!needs(&existing, &[split("2026-06-19", 1.0, 40.0)]));
    }

    #[test]
    fn sub_two_times_action_does_not_invalidate() {
        // 3-for-2 (1.5×) is below the material threshold.
        assert!(!needs(&[], &[split("2026-06-19", 3.0, 2.0)]));
    }

    #[test]
    fn old_split_does_not_invalidate() {
        // Already reflected in every provider's adjusted history — discovering
        // it in a bulk scrape must not purge the cache.
        assert!(!needs(&[], &[split("2020-01-15", 1.0, 10.0)]));
    }

    #[test]
    fn future_dated_split_does_not_invalidate_yet() {
        assert!(!needs(&[], &[split("2026-12-31", 1.0, 10.0)]));
    }

    #[test]
    fn degenerate_split_ratios_ignored() {
        assert!(!needs(&[], &[split("2026-06-19", 0.0, 10.0)]));
        assert!(!needs(&[], &[split("2026-06-19", 1.0, 0.0)]));
        assert!(!needs(&[], &[split("bad-date", 1.0, 40.0)]));
    }
}
