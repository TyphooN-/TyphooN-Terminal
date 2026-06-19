use super::{FilingAlert, InsiderTrade, SecFiling};
use rusqlite::{Connection, params};

// ── Query Functions (synchronous — called from spawn_blocking in commands) ──

/// Get recent filings, optionally filtered by ticker.
pub fn get_recent_filings(
    conn: &Connection,
    ticker: Option<&str>,
    limit: usize,
) -> Result<Vec<SecFiling>, String> {
    let limit = limit.min(1000);
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

/// Get ALL insider trades (no date cutoff). For BG thread growing database.
pub fn get_all_insider_trades(conn: &Connection) -> Result<Vec<InsiderTrade>, String> {
    // Memory optimization: limit to last 5 years (1825 days) to bound BG memory footprint.
    // Older trades remain in DB and accessible via get_insider_trades(ticker, days).
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(1825))
        .format("%Y-%m-%d")
        .to_string();
    // prepare_cached: called every BG cycle.
    let mut stmt = conn.prepare_cached(
        "SELECT id, ticker, accession_number, insider_name, insider_title, transaction_date, transaction_type, shares, price, aggregate_value, is_officer, is_director, created_at
         FROM sec_insider_trades WHERE transaction_date >= ?1 ORDER BY transaction_date DESC"
    ).map_err(|e| format!("Prepare all insider trades failed: {e}"))?;

    let rows = stmt
        .query_map(params![cutoff], |row| {
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
