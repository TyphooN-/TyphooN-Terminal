use rusqlite::{Connection, OptionalExtension, params};

/// Returns true if `ticker` was already scraped on `today` (YYYY-MM-DD).
///
/// `last_scrape_date` is nullable: the SEC universe is pre-seeded into
/// `sec_scrape_index` with a resolved CIK but a NULL date (never scraped). A SQL
/// NULL read into `String` fails with `InvalidColumnType`, and `.optional()` only
/// swallows the *no-row* case — so reading the column as bare `String` once turned
/// every never-scraped ticker (WOK plus ~6.3k others) into a hard error that bailed
/// out before fetching submissions, permanently freezing them at NULL. Reading as
/// `Option<String>` maps NULL → None → "not scraped today" so the scrape proceeds.
pub(super) fn ticker_scraped_on(
    conn: &Connection,
    ticker: &str,
    today: &str,
) -> Result<bool, String> {
    let last: Option<String> = conn
        .query_row(
            "SELECT last_scrape_date FROM sec_scrape_index WHERE ticker = ?1",
            params![ticker],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|e| format!("SEC scrape index check failed: {e}"))?
        .flatten();
    Ok(last.as_deref() == Some(today))
}

#[derive(Clone, Debug, Default)]
pub(super) struct SecScrapeIndexState {
    pub(super) last_scrape_date: Option<String>,
    pub(super) filing_count: i64,
    pub(super) cik: Option<String>,
}

pub(super) fn prioritize_sec_scrape_symbols(
    symbols: &mut [String],
    index: &std::collections::HashMap<String, SecScrapeIndexState>,
    today: &str,
) {
    let original_rank: std::collections::HashMap<String, usize> = symbols
        .iter()
        .enumerate()
        .map(|(i, s)| (s.clone(), i))
        .collect();
    symbols.sort_by(|a, b| {
        let default_a;
        let state_a = if let Some(state) = index.get(a) {
            state
        } else {
            default_a = SecScrapeIndexState::default();
            &default_a
        };
        let default_b;
        let state_b = if let Some(state) = index.get(b) {
            state
        } else {
            default_b = SecScrapeIndexState::default();
            &default_b
        };

        let key = |sym: &String, state: &SecScrapeIndexState| {
            let date = state.last_scrape_date.as_deref().unwrap_or("");
            let scraped_today = date == today;
            let has_cik = !state.cik.as_deref().unwrap_or("").is_empty();
            let gap_rank = if state.filing_count == 0 && has_cik && date.is_empty() {
                0u8
            } else if state.filing_count == 0 && date.is_empty() {
                1u8
            } else if state.filing_count == 0 {
                2u8
            } else {
                3u8
            };
            (
                scraped_today,
                gap_rank,
                date.to_string(),
                *original_rank.get(sym).unwrap_or(&usize::MAX),
            )
        };

        key(a, state_a).cmp(&key(b, state_b))
    });
}
