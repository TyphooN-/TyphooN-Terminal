use rusqlite::{Connection, params};

// ── Keyword Watchlist ───────────────────────────────────────────────────────

/// Add a keyword to the watchlist.
pub fn add_keyword(conn: &Connection, keyword: &str) -> Result<(), String> {
    conn.execute(
        "INSERT OR IGNORE INTO sec_keyword_watchlist (keyword, created_at) VALUES (?1, ?2)",
        params![keyword.to_lowercase(), chrono::Utc::now().timestamp()],
    )
    .map_err(|e| format!("Add keyword failed: {e}"))?;
    Ok(())
}

/// Remove a keyword from the watchlist.
pub fn remove_keyword(conn: &Connection, keyword: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM sec_keyword_watchlist WHERE keyword = ?1",
        params![keyword.to_lowercase()],
    )
    .map_err(|e| format!("Remove keyword failed: {e}"))?;
    Ok(())
}

/// Get all keywords.
pub fn get_keywords(conn: &Connection) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("SELECT keyword FROM sec_keyword_watchlist ORDER BY keyword")
        .map_err(|e| format!("Prepare keywords failed: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| format!("Query keywords failed: {e}"))?;
    rows.collect::<Result<Vec<String>, _>>()
        .map_err(|e| format!("Collect keywords failed: {e}"))
}

/// Check content against keyword watchlist, return matching keywords.
/// Takes the keyword list as a slice so callers batching many filings don't
/// re-query the DB once per filing.
pub fn check_keywords_in(keywords: &[String], content: &str) -> Vec<String> {
    let content_lower = content.to_lowercase();
    keywords
        .iter()
        .filter(|kw| content_lower.contains(kw.as_str()))
        .cloned()
        .collect()
}

/// Convenience wrapper for single-shot callers.
pub fn check_keywords(conn: &Connection, content: &str) -> Vec<String> {
    let keywords = get_keywords(conn).unwrap_or_default();
    check_keywords_in(&keywords, content)
}
