use super::SecFiling;
use rusqlite::{Connection, OptionalExtension, params};

/// Count total filings and how many have content fetched.
pub fn filing_content_stats(conn: &Connection) -> (usize, usize) {
    let total: i64 = conn
        .query_row("SELECT COUNT(*) FROM sec_filings", [], |r| r.get(0))
        .unwrap_or(0);
    let indexed: i64 = conn
        .query_row("SELECT COUNT(*) FROM sec_filing_content", [], |r| r.get(0))
        .unwrap_or(0);
    (total as usize, indexed as usize)
}

/// Store filing content (zstd-compressed) and index in FTS5.
/// Filings are typically 80KB plain text → ~8KB compressed (10x reduction).
/// MEM: Cap at 500KB plain text — extremely large filings (multi-MB 10-Ks) are
/// truncated to keep DB size bounded. Truncation marker appended.
pub fn store_filing_content(
    conn: &Connection,
    accession: &str,
    ticker: &str,
    form_type: &str,
    company: &str,
    content: &str,
) -> Result<(), String> {
    let now = chrono::Utc::now().timestamp();
    const MAX_PLAIN_BYTES: usize = 500_000;
    let stored: std::borrow::Cow<str> = if content.len() > MAX_PLAIN_BYTES {
        // Find a UTF-8 char boundary at or before the limit
        let mut cut = MAX_PLAIN_BYTES;
        while cut > 0 && !content.is_char_boundary(cut) {
            cut -= 1;
        }
        std::borrow::Cow::Owned(format!(
            "{}\n\n[Truncated at 500KB — original {} bytes]",
            &content[..cut],
            content.len()
        ))
    } else {
        std::borrow::Cow::Borrowed(content)
    };
    let compressed = zstd::encode_all(stored.as_bytes(), 3)
        .map_err(|e| format!("Compress content failed: {e}"))?;
    conn.execute(
        "INSERT OR REPLACE INTO sec_filing_content (accession_number, content_plain, content_size, fetched_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![accession, compressed, content.len() as i64, now],
    ).map_err(|e| format!("Store content failed: {e}"))?;

    // Update content_fetched flag and clear any previous retry state.
    let _ = conn.execute(
        "UPDATE sec_filings
         SET content_fetched = TRUE,
             content_fetch_attempts = 0,
             content_last_attempt_at = ?2,
             content_last_error = ''
         WHERE accession_number = ?1",
        params![accession, now],
    );

    // Populate FTS5 index (uncompressed for tokenization, also truncated)
    let fts_content: &str = stored.as_ref();
    let _ = conn.execute(
        "INSERT OR REPLACE INTO sec_fts (accession_number, ticker, form_type, company_name, content)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![accession, ticker, form_type, company, fts_content],
    );

    Ok(())
}

/// Full-text search across filing content using FTS5.
/// Returns filings matching the query, optionally filtered by ticker set.
pub fn search_filings_fts(
    conn: &Connection,
    query: &str,
    tickers: Option<&[String]>,
    limit: usize,
) -> Result<Vec<SecFiling>, String> {
    // Sanitize FTS5 query: escape double quotes, wrap terms
    let sanitized = query.replace('"', "\"\"");
    let fts_query = format!("\"{}\"", sanitized);

    let sql = if tickers.is_some() {
        format!(
            "SELECT f.id, f.ticker, f.form_type, f.accession_number, f.filing_date, f.url,
                    f.company_name, f.importance_score, f.category, f.summary, f.insider_flag, f.created_at
             FROM sec_fts s
             JOIN sec_filings f ON f.accession_number = s.accession_number
             WHERE sec_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2")
    } else {
        "SELECT f.id, f.ticker, f.form_type, f.accession_number, f.filing_date, f.url,
                f.company_name, f.importance_score, f.category, f.summary, f.insider_flag, f.created_at
         FROM sec_fts s
         JOIN sec_filings f ON f.accession_number = s.accession_number
         WHERE sec_fts MATCH ?1
         ORDER BY rank
         LIMIT ?2".to_string()
    };

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("FTS prepare failed: {e}"))?;
    let limit_i64 = limit as i64;
    let rows = stmt
        .query_map(params![fts_query, limit_i64], |row| {
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
        .map_err(|e| format!("FTS query failed: {e}"))?;

    let mut results: Vec<SecFiling> = rows.filter_map(|r| r.ok()).collect();

    // Post-filter by tickers if provided (FTS5 doesn't natively support IN-clause on separate column)
    if let Some(ticker_set) = tickers {
        let set: std::collections::HashSet<String> =
            ticker_set.iter().map(|t| t.to_uppercase()).collect();
        results.retain(|f| set.contains(&f.ticker.to_uppercase()));
    }

    Ok(results)
}

/// Get filing content for display / diff (decompresses zstd blob).
pub fn get_filing_content(conn: &Connection, accession: &str) -> Result<Option<String>, String> {
    let blob: Option<Vec<u8>> = conn
        .query_row(
            "SELECT content_plain FROM sec_filing_content WHERE accession_number = ?1",
            params![accession],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("Get content failed: {e}"))?;
    match blob {
        Some(bytes) => {
            // Try zstd decompress; fallback to UTF-8 string for legacy uncompressed entries
            let decoded = zstd::decode_all(bytes.as_slice())
                .ok()
                .and_then(|b| String::from_utf8(b).ok())
                .or_else(|| String::from_utf8(bytes).ok());
            Ok(decoded)
        }
        None => Ok(None),
    }
}

/// Get filings that haven't had their content fetched yet.
pub fn get_unfetched_filings(conn: &Connection, limit: usize) -> Result<Vec<SecFiling>, String> {
    const MAX_CONTENT_FETCH_ATTEMPTS: i64 = 3;
    const CONTENT_FETCH_RETRY_COOLDOWN_SECS: i64 = 6 * 60 * 60;

    // prepare_cached: called repeatedly by the content backfill worker. Skip
    // recently failed fetches and stop after a few hard failures so one SEC
    // 403/404 batch cannot monopolize every background cycle.
    let retry_before = chrono::Utc::now().timestamp() - CONTENT_FETCH_RETRY_COOLDOWN_SECS;
    let mut stmt = conn.prepare_cached(
        "SELECT f.id, f.ticker, f.form_type, f.accession_number, f.filing_date, f.url,
                f.company_name, f.importance_score, f.category, f.summary, f.insider_flag, f.created_at
         FROM sec_filings f
         LEFT JOIN sec_filing_content c ON c.accession_number = f.accession_number
         WHERE c.accession_number IS NULL
           AND COALESCE(f.content_fetched, FALSE) = FALSE
           AND COALESCE(f.content_fetch_attempts, 0) < ?2
           AND COALESCE(f.content_last_attempt_at, 0) <= ?3
         ORDER BY f.filing_date DESC, f.importance_score DESC, f.created_at DESC
         LIMIT ?1"
    ).map_err(|e| format!("Prepare unfetched failed: {e}"))?;

    let rows = stmt
        .query_map(
            params![limit as i64, MAX_CONTENT_FETCH_ATTEMPTS, retry_before],
            |row| {
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
            },
        )
        .map_err(|e| format!("Query unfetched failed: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect unfetched failed: {e}"))
}

/// Record a failed content hydration attempt so unfetchable SEC documents do
/// not stay at the front of the background queue forever.
pub fn mark_filing_content_fetch_failed(
    conn: &Connection,
    accession: &str,
    error: &str,
) -> Result<(), String> {
    let truncated_error: std::borrow::Cow<str> = if error.len() > 240 {
        std::borrow::Cow::Owned(format!("{}…", &error[..240]))
    } else {
        std::borrow::Cow::Borrowed(error)
    };
    conn.execute(
        "UPDATE sec_filings
         SET content_fetch_attempts = COALESCE(content_fetch_attempts, 0) + 1,
             content_last_attempt_at = ?2,
             content_last_error = ?3
         WHERE accession_number = ?1",
        params![
            accession,
            chrono::Utc::now().timestamp(),
            truncated_error.as_ref()
        ],
    )
    .map_err(|e| format!("Mark content fetch failed: {e}"))?;
    Ok(())
}

// ── Form 4 insider backfill queue ───────────────────────────────────

/// Form 4 filings whose insider trades have not been parsed yet, newest first.
///
/// Insider parsing only ever ran inline over filings inserted during the
/// current scrape pass, so anything that failed was never revisited — and until
/// the `xslF345X0*` URL fix in [`super::form4_xml_url`] every Form 4 failed,
/// because the parser was handed SEC's rendered HTML instead of the raw XML.
/// That left 537,648 stored Form 4 filings and 0 rows in `sec_insider_trades`.
/// This is the queue that drains that backlog.
///
/// Mirrors [`get_unfetched_filings`]: attempt-capped and cooldown'd so a
/// permanently unparseable document cannot monopolise every cycle, and ordered
/// newest-first so recent insider activity lands before a decade-old backlog.
pub fn get_unparsed_form4_filings(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<SecFiling>, String> {
    const MAX_INSIDER_PARSE_ATTEMPTS: i64 = 3;
    const INSIDER_PARSE_RETRY_COOLDOWN_SECS: i64 = 6 * 60 * 60;

    let retry_before = chrono::Utc::now().timestamp() - INSIDER_PARSE_RETRY_COOLDOWN_SECS;
    let mut stmt = conn
        .prepare_cached(
            "SELECT id, ticker, form_type, accession_number, filing_date, url,
                company_name, importance_score, category, summary, insider_flag, created_at
         FROM sec_filings
         WHERE insider_flag = TRUE
           AND COALESCE(insider_parsed, FALSE) = FALSE
           AND COALESCE(insider_parse_attempts, 0) < ?2
           AND COALESCE(insider_last_attempt_at, 0) <= ?3
         ORDER BY filing_date DESC
         LIMIT ?1",
        )
        .map_err(|e| format!("Prepare unparsed form4 failed: {e}"))?;

    let rows = stmt
        .query_map(
            params![limit as i64, MAX_INSIDER_PARSE_ATTEMPTS, retry_before],
            |row| {
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
            },
        )
        .map_err(|e| format!("Query unparsed form4 failed: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect unparsed form4 failed: {e}"))
}

/// Mark a Form 4 as parsed so the backfill queue stops returning it.
///
/// Called on success *including* a zero-transaction result: a Form 4 can
/// legitimately report only holdings, and retrying those forever would wedge
/// the queue behind documents that will never yield a row.
pub fn mark_form4_insider_parsed(conn: &Connection, accession: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE sec_filings
         SET insider_parsed = TRUE,
             insider_parse_attempts = COALESCE(insider_parse_attempts, 0) + 1,
             insider_last_attempt_at = ?2
         WHERE accession_number = ?1",
        params![accession, chrono::Utc::now().timestamp()],
    )
    .map_err(|e| format!("Mark form4 parsed failed: {e}"))?;
    Ok(())
}

/// Record a failed Form 4 parse attempt (network error, HTTP failure).
/// Leaves `insider_parsed` false so the row is retried after the cooldown,
/// until the attempt cap is reached.
pub fn mark_form4_insider_parse_failed(conn: &Connection, accession: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE sec_filings
         SET insider_parse_attempts = COALESCE(insider_parse_attempts, 0) + 1,
             insider_last_attempt_at = ?2
         WHERE accession_number = ?1",
        params![accession, chrono::Utc::now().timestamp()],
    )
    .map_err(|e| format!("Mark form4 parse failed: {e}"))?;
    Ok(())
}
