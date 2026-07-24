use super::{FilingAlert, InsiderTrade, SecFiling};
use rusqlite::{Connection, params};

// ── Query Functions (synchronous — called from spawn_blocking in commands) ──

/// Hard ceiling on rows returned by [`get_recent_filings`], guarding the
/// caller-supplied limit.
///
/// This used to be a bare `.min(1000)`, which silently clamped every caller: a
/// request for more rows was truncated with no error and no log, so the SEC
/// scanner's snapshot could never grow past 1000 rows however it was called.
/// On a full-universe corpus (1M+ rows spanning decades) the newest 1000 rows
/// cover only weeks and a few hundred tickers, so searching them for a symbol
/// outside that window found nothing even with years of its filings stored.
/// Global browsing is still bounded — an unbounded snapshot is what pushed the
/// process into the OOM killer — but per-ticker history now has room to be a
/// real multi-year answer. Rows average ~150 bytes of payload (~450 bytes
/// in memory), so this ceiling is tens of MB, not hundreds.
pub const MAX_FILING_QUERY_ROWS: usize = 50_000;

/// Ceiling on the insider-trade BG snapshot. Sized like the filings window:
/// tens of MB, newest-first, enough to browse. Depth for one symbol is
/// [`get_insider_trades`] with `Some(ticker)`, not a bigger snapshot.
pub const MAX_INSIDER_SNAPSHOT_ROWS: usize = 50_000;

/// Get recent filings, optionally filtered by ticker.
///
/// With `Some(ticker)` this rides `idx_sec_ticker_date` and is the on-demand
/// path for "show me everything this symbol has filed" — a seek, not a scan.
pub fn get_recent_filings(
    conn: &Connection,
    ticker: Option<&str>,
    limit: usize,
) -> Result<Vec<SecFiling>, String> {
    let limit = limit.min(MAX_FILING_QUERY_ROWS);
    let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(t) = ticker
    {
        (
            "SELECT id, ticker, form_type, accession_number, filing_date, url, company_name, importance_score, category, summary, insider_flag, created_at
             FROM sec_filings WHERE ticker = ?1 ORDER BY filing_date DESC LIMIT ?2".to_string(),
            vec![Box::new(t.to_uppercase()), Box::new(limit as i64)],
        )
    } else {
        (
            "SELECT id, ticker, form_type, accession_number, filing_date, url, company_name, importance_score, category, summary, insider_flag, created_at
             FROM sec_filings ORDER BY filing_date DESC LIMIT ?1".to_string(),
            vec![Box::new(limit as i64)],
        )
    };

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Prepare failed: {e}"))?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();
    let rows = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(SecFiling {
                id: row.get(0)?,
                ticker: row.get(1)?,
                form_type: row.get(2)?,
                accession_number: row.get(3)?,
                filing_date: row.get(4)?,
                url: row.get(5)?,
                company_name: row.get(6)?,
                importance_score: row.get(7)?,
                category: row.get(8)?,
                summary: row.get(9)?,
                insider_flag: row.get(10)?,
                created_at: row.get(11)?,
            })
        })
        .map_err(|e| format!("Query failed: {e}"))?;

    let mut results = Vec::new();
    for row in rows {
        if let Ok(filing) = row {
            results.push(filing);
        }
    }
    Ok(results)
}

/// Get insider trades for the last N days, optionally filtered by ticker.
pub fn get_insider_trades(
    conn: &Connection,
    ticker: Option<&str>,
    days: i32,
) -> Result<Vec<InsiderTrade>, String> {
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(days as i64))
        .format("%Y-%m-%d")
        .to_string();

    let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(t) = ticker
    {
        (
            "SELECT id, ticker, accession_number, insider_name, insider_title, transaction_date, transaction_type, shares, price, aggregate_value, is_officer, is_director, created_at
             FROM sec_insider_trades WHERE ticker = ?1 AND transaction_date >= ?2 ORDER BY transaction_date DESC".to_string(),
            vec![Box::new(t.to_uppercase()), Box::new(cutoff)],
        )
    } else {
        (
            "SELECT id, ticker, accession_number, insider_name, insider_title, transaction_date, transaction_type, shares, price, aggregate_value, is_officer, is_director, created_at
             FROM sec_insider_trades WHERE transaction_date >= ?1 ORDER BY transaction_date DESC".to_string(),
            vec![Box::new(cutoff)],
        )
    };

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Prepare failed: {e}"))?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();
    let rows = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(InsiderTrade {
                id: row.get(0)?,
                ticker: row.get(1)?,
                accession_number: row.get(2)?,
                insider_name: row.get(3)?,
                insider_title: row.get(4)?,
                transaction_date: row.get(5)?,
                transaction_type: row.get(6)?,
                shares: row.get(7)?,
                price: row.get(8)?,
                aggregate_value: row.get(9)?,
                is_officer: row.get(10)?,
                is_director: row.get(11)?,
                created_at: row.get(12)?,
            })
        })
        .map_err(|e| format!("Query failed: {e}"))?;

    let mut results = Vec::new();
    for row in rows {
        if let Ok(trade) = row {
            results.push(trade);
        }
    }
    Ok(results)
}

/// Get filing alerts (dismissed or undismissed).
pub fn get_filing_alerts(conn: &Connection, dismissed: bool) -> Result<Vec<FilingAlert>, String> {
    // prepare_cached: called per BG cycle to refresh the alerts panel.
    let mut stmt = conn.prepare_cached(
        "SELECT id, ticker, alert_type, message, COALESCE(filing_accession, ''), importance, created_at, dismissed, COALESCE(dismissed_reason, '')
         FROM sec_filing_alerts WHERE dismissed = ?1 ORDER BY created_at DESC"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let rows = stmt
        .query_map(params![dismissed], |row| {
            Ok(FilingAlert {
                id: row.get(0)?,
                ticker: row.get(1)?,
                alert_type: row.get(2)?,
                message: row.get(3)?,
                filing_accession: row.get(4)?,
                importance: row.get(5)?,
                created_at: row.get(6)?,
                dismissed: row.get(7)?,
                dismissed_reason: row.get(8)?,
            })
        })
        .map_err(|e| format!("Query failed: {e}"))?;

    let mut results = Vec::new();
    for row in rows {
        if let Ok(alert) = row {
            results.push(alert);
        }
    }
    Ok(results)
}

/// Dismiss an alert with a reason.
pub fn dismiss_alert(conn: &Connection, alert_id: i64, reason: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE sec_filing_alerts SET dismissed = TRUE, dismissed_reason = ?1 WHERE id = ?2",
        params![reason, alert_id],
    )
    .map_err(|e| format!("Dismiss alert failed: {e}"))?;
    Ok(())
}

// ── Unlimited Query Functions (for growing database) ────────────────────────

/// Get ALL filings (no limit). For BG thread — builds the growing searchable database.
pub fn get_all_filings(conn: &Connection) -> Result<Vec<SecFiling>, String> {
    // prepare_cached: called every BG cycle to refresh the filings cache.
    let mut stmt = conn.prepare_cached(
        "SELECT id, ticker, form_type, accession_number, filing_date, url, company_name, importance_score, category, summary, insider_flag, created_at
         FROM sec_filings ORDER BY filing_date DESC"
    ).map_err(|e| format!("Prepare all filings failed: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(SecFiling {
                id: row.get(0)?,
                ticker: row.get(1)?,
                form_type: row.get(2)?,
                accession_number: row.get(3)?,
                filing_date: row.get(4)?,
                url: row.get(5)?,
                company_name: row.get(6)?,
                importance_score: row.get(7)?,
                category: row.get(8)?,
                summary: row.get(9)?,
                insider_flag: row.get(10)?,
                created_at: row.get(11)?,
            })
        })
        .map_err(|e| format!("Query all filings failed: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect filings failed: {e}"))
}

/// Recent insider trades for the BG snapshot: last 5 years, newest first,
/// bounded by [`MAX_INSIDER_SNAPSHOT_ROWS`].
///
/// The date cutoff alone was enough only while the table was empty. Fixing the
/// Form 4 parser (it had been handed SEC's rendered HTML instead of XML, so
/// 537,648 stored Form 4 filings produced zero rows) turns this into ~1.5M
/// trades as the backfill drains — an unbounded `SELECT` cloned into every app
/// snapshot each BG cycle, which is precisely the shape that pushed the filings
/// snapshot into the OOM killer. Same rule as filings: this is a browse window;
/// per-symbol depth goes through `get_insider_trades(conn, Some(ticker), days)`,
/// which rides `idx_insider_ticker`.
pub fn get_all_insider_trades(conn: &Connection) -> Result<Vec<InsiderTrade>, String> {
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(1825))
        .format("%Y-%m-%d")
        .to_string();
    // prepare_cached: called every BG cycle.
    let mut stmt = conn.prepare_cached(
        "SELECT id, ticker, accession_number, insider_name, insider_title, transaction_date, transaction_type, shares, price, aggregate_value, is_officer, is_director, created_at
         FROM sec_insider_trades WHERE transaction_date >= ?1 ORDER BY transaction_date DESC LIMIT ?2"
    ).map_err(|e| format!("Prepare all insider trades failed: {e}"))?;

    let rows = stmt
        .query_map(params![cutoff, MAX_INSIDER_SNAPSHOT_ROWS as i64], |row| {
            Ok(InsiderTrade {
                id: row.get(0)?,
                ticker: row.get(1)?,
                accession_number: row.get(2)?,
                insider_name: row.get(3)?,
                insider_title: row.get(4)?,
                transaction_date: row.get(5)?,
                transaction_type: row.get(6)?,
                shares: row.get(7)?,
                price: row.get(8)?,
                aggregate_value: row.get(9)?,
                is_officer: row.get(10)?,
                is_director: row.get(11)?,
                created_at: row.get(12)?,
            })
        })
        .map_err(|e| format!("Query all insider trades failed: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect insider trades failed: {e}"))
}
