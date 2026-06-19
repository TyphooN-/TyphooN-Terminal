//! SEC Filing Scraper — local database of SEC filings with Form 4 insider trade parsing.
//!
//! Fetches filings from SEC EDGAR (data.sec.gov), stores them in SQLite,
//! parses Form 4 insider trades, and generates alerts for significant insider activity.
//!
//! Architecture note: rusqlite::Connection is not Send, so all DB operations happen
//! inside `tokio::task::spawn_blocking` closures with short-lived connections.
//! HTTP fetches happen on the async runtime between DB calls.

use rusqlite::{Connection, OptionalExtension, params};
use std::path::{Path, PathBuf};

mod content_text;
mod scoring;
mod summary;
mod types;
#[cfg(test)]
use content_text::decode_html_entities;
pub use content_text::{polish_filing_text, strip_html_to_text};
pub use scoring::compute_importance;
use scoring::{RELEVANT_FORMS, categorize_form};
pub use summary::summarize_filing;
use types::PendingFiling;
pub use types::{
    DiffChunk, FilingAlert, FilingSection, FilingSummary, InsiderTrade, ScrapeStats, SecFiling,
};

/// SEC EDGAR blocks generic/anonymous user agents with 403s. Keep this
/// descriptive and email-shaped; callers outside this module should reuse this
/// constant instead of inventing their own SEC header.
pub const SEC_EDGAR_USER_AGENT: &str = "TyphooN-Terminal/1.0 typhoon-terminal@example.invalid";

/// Rate limit sleep between SEC requests (200ms = 5 req/sec, well under 10/sec limit).
const RATE_LIMIT_MS: u64 = 250; // SEC EDGAR fair use: max 10 req/sec, use 4/sec for safety

// ── Data Types ──────────────────────────────────────────────────────

// ── Helper: open a WAL connection ───────────────────────────────────

fn open_conn(db_path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(db_path).map_err(|e| format!("SQLite open failed: {e}"))?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
        .map_err(|e| format!("Pragma failed: {e}"))?;
    // SEC scraping writes on this dedicated connection — independent of the
    // `SqliteCache` write connection the UI thread and bar-sync share. Under WAL a
    // single writer holds the lock at a time, so WITHOUT a busy timeout an SEC write
    // that collides with a UI/bar-sync write fails *instantly* with SQLITE_BUSY —
    // silently dropping filings — and thrashes the lock. Retrying for 5s lets SEC and
    // the shared writer interleave politely, while WAL keeps the UI's *reads* (a
    // separate `read_conn`) unblocked throughout. This is what decouples SEC writes
    // from the render path and lets a broad scrape run during heavy market-data sync.
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| format!("busy_timeout failed: {e}"))?;
    Ok(conn)
}

// ── SQLite Schema ───────────────────────────────────────────────────

pub fn create_sec_tables(conn: &Connection) -> Result<(), String> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS sec_filings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ticker TEXT NOT NULL,
            form_type TEXT NOT NULL,
            accession_number TEXT UNIQUE NOT NULL,
            filing_date TEXT NOT NULL,
            url TEXT NOT NULL,
            company_name TEXT DEFAULT '',
            importance_score INTEGER DEFAULT 50,
            category TEXT DEFAULT 'OTHER',
            summary TEXT DEFAULT '',
            insider_flag BOOLEAN DEFAULT FALSE,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_sec_ticker_date ON sec_filings(ticker, filing_date DESC);
        CREATE INDEX IF NOT EXISTS idx_sec_form ON sec_filings(form_type);

        CREATE TABLE IF NOT EXISTS sec_insider_trades (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ticker TEXT NOT NULL,
            accession_number TEXT NOT NULL,
            insider_name TEXT NOT NULL,
            insider_title TEXT DEFAULT '',
            transaction_date TEXT NOT NULL,
            transaction_type TEXT NOT NULL,
            shares REAL DEFAULT 0,
            price REAL DEFAULT 0,
            aggregate_value REAL DEFAULT 0,
            is_officer BOOLEAN DEFAULT FALSE,
            is_director BOOLEAN DEFAULT FALSE,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_insider_ticker ON sec_insider_trades(ticker, transaction_date DESC);

        CREATE TABLE IF NOT EXISTS sec_filing_alerts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ticker TEXT NOT NULL,
            alert_type TEXT NOT NULL,
            message TEXT NOT NULL,
            filing_accession TEXT,
            importance INTEGER DEFAULT 50,
            created_at INTEGER NOT NULL,
            dismissed BOOLEAN DEFAULT FALSE,
            dismissed_reason TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_alerts_created ON sec_filing_alerts(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_alerts_ticker ON sec_filing_alerts(ticker);

        CREATE TABLE IF NOT EXISTS sec_scrape_index (
            ticker TEXT PRIMARY KEY,
            last_scrape_date TEXT,
            filing_count INTEGER DEFAULT 0,
            cik TEXT
        );
    ").map_err(|e| format!("Failed to create SEC tables: {e}"))?;

    // Schema migration: add updated_at column (last-modified tracking)
    let _ = conn.execute(
        "ALTER TABLE sec_scrape_index ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
        [],
    );

    // Schema migration: filing content storage (indefinite, growing database)
    let _ = conn.execute(
        "ALTER TABLE sec_filings ADD COLUMN content_fetched BOOLEAN DEFAULT FALSE",
        [],
    );
    // Retry bookkeeping for SEC content hydration. Without this, permanently
    // unfetchable/blocked documents stay at the front of the queue and the
    // background worker can refetch the same failing batch forever.
    let _ = conn.execute(
        "ALTER TABLE sec_filings ADD COLUMN content_fetch_attempts INTEGER DEFAULT 0",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE sec_filings ADD COLUMN content_last_attempt_at INTEGER DEFAULT 0",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE sec_filings ADD COLUMN content_last_error TEXT DEFAULT ''",
        [],
    );
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS sec_filing_content (
            accession_number TEXT PRIMARY KEY,
            content_plain TEXT NOT NULL,
            content_size INTEGER DEFAULT 0,
            fetched_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sec_keyword_watchlist (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            keyword TEXT NOT NULL UNIQUE,
            created_at INTEGER NOT NULL
        );
    ",
    )
    .map_err(|e| format!("Failed to create SEC content tables: {e}"))?;

    // FTS5 full-text search index (porter stemming + unicode)
    let _ = conn.execute_batch(
        "
        CREATE VIRTUAL TABLE IF NOT EXISTS sec_fts USING fts5(
            accession_number, ticker, form_type, company_name, content,
            tokenize='porter unicode61'
        );
    ",
    );

    Ok(())
}

// ── CIK Lookup ──────────────────────────────────────────────────────

async fn fetch_sec_ticker_cik_map(
    client: &reqwest::Client,
) -> Result<std::collections::HashMap<String, String>, String> {
    let resp = client
        .get("https://www.sec.gov/files/company_tickers.json")
        .header("User-Agent", SEC_EDGAR_USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("SEC ticker map request failed: {e}"))?;

    let tickers_json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("SEC ticker map parse failed: {e}"))?;

    let mut out = std::collections::HashMap::new();
    if let Some(obj) = tickers_json.as_object() {
        for (_, v) in obj {
            let Some(ticker) = v["ticker"].as_str() else {
                continue;
            };
            let Some(cik) = v["cik_str"]
                .as_u64()
                .or_else(|| v["cik_str"].as_str().and_then(|s| s.parse().ok()))
            else {
                continue;
            };
            out.insert(ticker.to_uppercase(), format!("{:010}", cik));
        }
    }
    Ok(out)
}

// ── Filing Scraper ──────────────────────────────────────────────────

/// Returns true if `ticker` was already scraped on `today` (YYYY-MM-DD).
///
/// `last_scrape_date` is nullable: the SEC universe is pre-seeded into
/// `sec_scrape_index` with a resolved CIK but a NULL date (never scraped). A SQL
/// NULL read into `String` fails with `InvalidColumnType`, and `.optional()` only
/// swallows the *no-row* case — so reading the column as bare `String` once turned
/// every never-scraped ticker (WOK plus ~6.3k others) into a hard error that bailed
/// out before fetching submissions, permanently freezing them at NULL. Reading as
/// `Option<String>` maps NULL → None → "not scraped today" so the scrape proceeds.
fn ticker_scraped_on(conn: &Connection, ticker: &str, today: &str) -> Result<bool, String> {
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
struct SecScrapeIndexState {
    last_scrape_date: Option<String>,
    filing_count: i64,
    cik: Option<String>,
}

fn prioritize_sec_scrape_symbols(
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

/// Scrape filings for a single ticker from SEC EDGAR.
/// Returns (new_filings, new_insider_trades, new_alerts).
pub async fn scrape_filings_for_ticker(
    db_path: &Path,
    client: &reqwest::Client,
    ticker: &str,
    cik: &str,
) -> Result<(usize, usize, usize), String> {
    // O(1) scrape gate: if this ticker was scraped today, trust the local DB.
    // SEC submissions are append-only for our purposes; repeated same-day pulls
    // just waste quota and rebuild the same cache rows.
    {
        let db = db_path.to_path_buf();
        let t = ticker.to_uppercase();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        if tokio::task::spawn_blocking(move || -> Result<bool, String> {
            let conn = open_conn(&db)?;
            ticker_scraped_on(&conn, &t, &today)
        })
        .await
        .map_err(|e| format!("spawn_blocking: {e}"))??
        {
            return Ok((0, 0, 0));
        }
    }

    // Step 1: Fetch submissions JSON (async)
    let url = format!("https://data.sec.gov/submissions/CIK{cik}.json");
    let resp = client
        .get(&url)
        .header("User-Agent", SEC_EDGAR_USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("SEC submissions fetch failed for {ticker}: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "SEC submissions HTTP {} for {ticker}",
            resp.status()
        ));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("SEC submissions parse failed for {ticker}: {e}"))?;

    let company_name = body["name"].as_str().unwrap_or("").to_string();
    let upper_ticker = ticker.to_uppercase();

    // Step 2: Parse filings into pending list (no DB needed yet)
    let recent = &body["filings"]["recent"];
    let forms = recent["form"].as_array();
    let dates = recent["filingDate"].as_array();
    let accessions = recent["accessionNumber"].as_array();
    let primary_docs = recent["primaryDocument"].as_array();

    let (forms, dates, accessions, primary_docs) = match (forms, dates, accessions, primary_docs) {
        (Some(f), Some(d), Some(a), Some(p)) => (f, d, a, p),
        _ => return Ok((0, 0, 0)),
    };

    let cik_trimmed = cik.trim_start_matches('0');
    let mut pending: Vec<PendingFiling> = Vec::new();

    for i in 0..forms
        .len()
        .min(dates.len())
        .min(accessions.len())
        .min(primary_docs.len())
    {
        let form = forms[i].as_str().unwrap_or("");
        let date = dates[i].as_str().unwrap_or("");
        let accession = accessions[i].as_str().unwrap_or("");
        let primary_doc = primary_docs[i].as_str().unwrap_or("");

        if !RELEVANT_FORMS.contains(&form) {
            continue;
        }

        let is_late = form == "NT 10-K" || form == "NT 10-Q";
        let is_form4 = form == "4";
        let importance = compute_importance(form, false, is_late);
        let category = categorize_form(form).to_string();

        let accession_nodash = accession.replace('-', "");
        let filing_url = format!(
            "https://www.sec.gov/Archives/edgar/data/{}/{}/{}",
            cik_trimmed, accession_nodash, primary_doc,
        );

        pending.push(PendingFiling {
            ticker: upper_ticker.clone(),
            form_type: form.to_string(),
            accession_number: accession.to_string(),
            filing_date: date.to_string(),
            url: filing_url,
            company_name: company_name.clone(),
            importance_score: importance,
            category,
            insider_flag: is_form4,
            is_late,
        });
    }

    // Step 3: Insert new filings (blocking), get back which ones are actually new.
    // PERF: the previous implementation did an N+1 loop:
    //   1) per-filing `SELECT COUNT(*) FROM sec_filings WHERE accession = ?`
    //   2) per-filing `INSERT OR IGNORE`
    //   3) per-filing `SELECT COUNT(*) FROM sec_filing_alerts WHERE ticker/type/created_at`
    // That's 3 round-trips per filing, repeated for up to 100 filings per ticker.
    // Now: one bulk `SELECT accession_number` preload, one bulk `SELECT ticker,alert_type`
    // preload, and a single INSERT per new filing — all inside one transaction.
    let db = db_path.to_path_buf();
    let pending_clone = pending.clone();
    let new_filings: Vec<PendingFiling> = tokio::task::spawn_blocking(move || {
        let mut conn = open_conn(&db)?;
        let now = chrono::Utc::now().timestamp();
        let yesterday = now - 86400;

        // Preload existing accession numbers for this batch (scoped to the tickers we're
        // about to touch so the lookup stays small).
        let tickers: std::collections::HashSet<String> =
            pending_clone.iter().map(|f| f.ticker.clone()).collect();
        let mut existing_accessions: std::collections::HashSet<String> =
            std::collections::HashSet::with_capacity(pending_clone.len() * 2);
        if !tickers.is_empty() {
            let placeholders = std::iter::repeat("?").take(tickers.len()).collect::<Vec<_>>().join(",");
            let sql = format!(
                "SELECT accession_number FROM sec_filings WHERE ticker IN ({placeholders})"
            );
            let tickers_vec: Vec<&String> = tickers.iter().collect();
            let params_refs: Vec<&dyn rusqlite::types::ToSql> =
                tickers_vec.iter().map(|s| *s as &dyn rusqlite::types::ToSql).collect();
            if let Ok(mut stmt) = conn.prepare(&sql) {
                if let Ok(rows) = stmt.query_map(params_refs.as_slice(), |r| r.get::<_, String>(0)) {
                    for r in rows.flatten() { existing_accessions.insert(r); }
                }
            }
        }

        // Preload today's alerts so we can O(1) dedup without a per-filing COUNT.
        let mut alerts_today: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();
        {
            if let Ok(mut stmt) = conn.prepare(
                "SELECT ticker, alert_type FROM sec_filing_alerts WHERE created_at > ?1"
            ) {
                if let Ok(rows) = stmt.query_map(params![yesterday], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                }) {
                    for r in rows.flatten() { alerts_today.insert(r); }
                }
            }
        }

        let tx = conn.transaction().map_err(|e| format!("begin tx: {e}"))?;
        let mut inserted = Vec::new();
        {
            let mut ins_filing = tx.prepare(
                "INSERT OR IGNORE INTO sec_filings (ticker, form_type, accession_number, filing_date, url, company_name, importance_score, category, summary, insider_flag, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
            ).map_err(|e| format!("prepare filing insert: {e}"))?;
            let mut ins_alert = tx.prepare(
                "INSERT INTO sec_filing_alerts (ticker, alert_type, message, filing_accession, importance, created_at, dismissed)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, FALSE)"
            ).map_err(|e| format!("prepare alert insert: {e}"))?;

            for f in pending_clone {
                if existing_accessions.contains(&f.accession_number) { continue; }
                existing_accessions.insert(f.accession_number.clone());

                ins_filing.execute(params![
                    f.ticker, f.form_type, f.accession_number, f.filing_date, f.url,
                    f.company_name, f.importance_score, f.category, "", f.insider_flag, now
                ]).map_err(|e| format!("Insert filing failed: {e}"))?;

                let alert_info: Option<(&str, String)> = match f.form_type.as_str() {
                    "NT 10-K" | "NT 10-Q" => Some(("LATE_FILING", format!("{}: Late filing — possible internal issues", f.ticker))),
                    "SC 13D" | "SC 13D/A" => Some(("ACTIVIST", format!("{}: Activist investor took >5% position", f.ticker))),
                    "10-K/A" | "10-Q/A" => Some(("RESTATEMENT", format!("{}: Restated financials — review immediately", f.ticker))),
                    "8-K/A" => Some(("AMENDED_EVENT", format!("{}: Amended material event — updated disclosure", f.ticker))),
                    "S-3" => Some(("DILUTION_RISK", format!("{}: Shelf registration filed — potential dilution", f.ticker))),
                    "424B5" | "424B2" | "424B4" => Some(("ACTIVE_DILUTION", format!("{}: Prospectus filed — offering in progress", f.ticker))),
                    "15-12B" | "15-12G" => Some(("DELISTING_RISK", format!("{}: Deregistration filed — delisting risk", f.ticker))),
                    "SC TO-T" | "SC TO-I" => Some(("TENDER_OFFER", format!("{}: Tender offer filed — acquisition bid", f.ticker))),
                    "CORRESP" => Some(("SEC_INQUIRY", format!("{}: SEC correspondence — regulatory scrutiny", f.ticker))),
                    "PREM14A" => Some(("MERGER_PROXY", format!("{}: Preliminary merger proxy filed", f.ticker))),
                    _ => None,
                };
                if let Some((alert_type, message)) = alert_info {
                    let key = (f.ticker.clone(), alert_type.to_string());
                    if alerts_today.insert(key) {
                        let _ = ins_alert.execute(params![
                            f.ticker, alert_type, message, f.accession_number, f.importance_score, now
                        ]);
                    }
                }

                inserted.push(f);
            }
        }
        tx.commit().map_err(|e| format!("commit tx: {e}"))?;
        Ok::<_, String>(inserted)
    }).await.map_err(|e| format!("spawn_blocking: {e}"))??;

    let num_new = new_filings.len();
    let mut total_insider_trades = 0usize;
    let mut total_alerts = new_filings.iter().filter(|f| f.is_late).count();

    // Step 4: For each new Form 4, fetch and parse insider trades
    for f in &new_filings {
        if !f.insider_flag {
            continue;
        }

        tokio::time::sleep(std::time::Duration::from_millis(RATE_LIMIT_MS)).await;

        match fetch_and_parse_form4(db_path, client, &f.ticker, &f.accession_number, &f.url).await {
            Ok((trades, alerts)) => {
                total_insider_trades += trades;
                total_alerts += alerts;
            }
            Err(e) => {
                tracing::debug!(
                    "Form 4 parse failed for {} {}: {e}",
                    f.ticker,
                    f.accession_number
                );
            }
        }
    }

    // Step 5: Update scrape index (blocking)
    {
        let db = db_path.to_path_buf();
        let t = upper_ticker.clone();
        let cik_str = cik.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = open_conn(&db)?;
            let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
            let total_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM sec_filings WHERE ticker = ?1",
                params![t],
                |row| row.get(0),
            ).unwrap_or(0);
            conn.execute(
                "INSERT OR REPLACE INTO sec_scrape_index (ticker, last_scrape_date, filing_count, cik, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![t, today, total_count, cik_str, chrono::Utc::now().timestamp()],
            ).map_err(|e| format!("Update scrape index failed: {e}"))?;
            Ok::<_, String>(())
        }).await.map_err(|e| format!("spawn_blocking: {e}"))??;
    }

    Ok((num_new, total_insider_trades, total_alerts))
}

// ── Form 4 Insider Trade Parsing ────────────────────────────────────

/// Fetch a Form 4 filing and parse insider trades. All DB writes are blocking.
async fn fetch_and_parse_form4(
    db_path: &Path,
    client: &reqwest::Client,
    ticker: &str,
    accession: &str,
    url: &str,
) -> Result<(usize, usize), String> {
    // Async: fetch the filing with retry on 429
    let mut body = String::new();
    for attempt in 0..3u32 {
        let resp = client
            .get(url)
            .header("User-Agent", SEC_EDGAR_USER_AGENT)
            .send()
            .await
            .map_err(|e| format!("Form 4 fetch failed: {e}"))?;

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            // Exponential backoff: 1s, 2s, 4s
            let delay = std::time::Duration::from_secs(1 << attempt);
            tracing::debug!("Form 4 429 for {ticker} — retrying in {}s", delay.as_secs());
            tokio::time::sleep(delay).await;
            continue;
        }

        if !resp.status().is_success() {
            return Err(format!("Form 4 HTTP {}", resp.status()));
        }

        body = resp
            .text()
            .await
            .map_err(|e| format!("Form 4 read failed: {e}"))?;
        break;
    }
    if body.is_empty() {
        return Err(format!("Form 4 exhausted retries for {ticker} {accession}"));
    }

    // Parse in-memory (no DB needed)
    let insider_name =
        extract_xml_value(&body, "rptOwnerName").unwrap_or_else(|| "Unknown".to_string());
    let insider_title = extract_xml_value(&body, "officerTitle").unwrap_or_default();
    let is_officer =
        body.contains("<isOfficer>true</isOfficer>") || body.contains("<isOfficer>1</isOfficer>");
    let is_director = body.contains("<isDirector>true</isDirector>")
        || body.contains("<isDirector>1</isDirector>");

    let transactions = extract_transactions(&body);

    // Blocking: insert trades + create alerts
    let db = db_path.to_path_buf();
    let ticker_owned = ticker.to_string();
    let accession_owned = accession.to_string();

    let (trades_inserted, alerts_created) = tokio::task::spawn_blocking(move || {
        let conn = open_conn(&db)?;
        let now = chrono::Utc::now().timestamp();
        let mut trades = 0usize;
        let mut alerts = 0usize;

        for txn in &transactions {
            let aggregate_value = txn.shares * txn.price;

            conn.execute(
                "INSERT INTO sec_insider_trades (ticker, accession_number, insider_name, insider_title, transaction_date, transaction_type, shares, price, aggregate_value, is_officer, is_director, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    ticker_owned, accession_owned, insider_name, insider_title,
                    txn.date, txn.code, txn.shares, txn.price, aggregate_value,
                    is_officer, is_director, now,
                ],
            ).map_err(|e| format!("Insert insider trade failed: {e}"))?;

            trades += 1;

            // Alert on significant insider sells by officers/directors
            let is_sell = txn.code == "S" || txn.code == "D";
            if is_sell && (is_officer || is_director) && aggregate_value > 100_000.0 {
                let importance = compute_importance("4", true, false);
                let title_display = if insider_title.is_empty() {
                    if is_director { "Director".to_string() } else { "Officer".to_string() }
                } else {
                    insider_title.clone()
                };
                conn.execute(
                    "INSERT INTO sec_filing_alerts (ticker, alert_type, message, filing_accession, importance, created_at, dismissed)
                     VALUES (?1, 'INSIDER_SELL', ?2, ?3, ?4, ?5, FALSE)",
                    params![
                        ticker_owned,
                        format!("{insider_name} ({title_display}) sold ${:.0} of {ticker_owned} ({:.0} shares @ ${:.2})",
                                aggregate_value, txn.shares, txn.price),
                        accession_owned,
                        importance,
                        now,
                    ],
                ).ok();
                alerts += 1;

                conn.execute(
                    "UPDATE sec_filings SET importance_score = MAX(importance_score, ?1) WHERE accession_number = ?2",
                    params![importance, accession_owned],
                ).ok();
            }
        }

        Ok::<_, String>((trades, alerts))
    }).await.map_err(|e| format!("spawn_blocking: {e}"))??;

    Ok((trades_inserted, alerts_created))
}

#[derive(Debug, Clone)]
struct ParsedTransaction {
    code: String,
    shares: f64,
    price: f64,
    date: String,
}

/// Extract transaction blocks from Form 4 XML/HTML.
fn extract_transactions(body: &str) -> Vec<ParsedTransaction> {
    let mut transactions = Vec::new();

    let block_tags = ["nonDerivativeTransaction", "derivativeTransaction"];
    for tag in block_tags {
        let open_tag = format!("<{tag}>");
        let close_tag = format!("</{tag}>");
        let mut search_from = 0;
        while let Some(start) = body[search_from..].find(&open_tag) {
            let abs_start = search_from + start;
            if let Some(end) = body[abs_start..].find(&close_tag) {
                let block = &body[abs_start..abs_start + end + close_tag.len()];

                let code = extract_xml_value(block, "transactionCode").unwrap_or_default();
                let shares = extract_xml_value(block, "transactionShares")
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .unwrap_or(0.0);
                let price = extract_xml_value(block, "transactionPricePerShare")
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .unwrap_or(0.0);
                let date = extract_xml_value(block, "transactionDate").unwrap_or_default();

                if !code.is_empty() {
                    transactions.push(ParsedTransaction {
                        code,
                        shares,
                        price,
                        date,
                    });
                }

                search_from = abs_start + end + close_tag.len();
            } else {
                break;
            }
        }
    }

    transactions
}

/// Extract text content of the first occurrence of an XML tag.
/// Handles nested <value> tags (SEC XML wraps values this way).
fn extract_xml_value(body: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    if let Some(start) = body.find(&open) {
        let after = start + open.len();
        if let Some(end) = body[after..].find(&close) {
            let content = body[after..after + end].trim();
            // Handle nested <value> tags
            if let Some(val) = extract_xml_value(content, "value") {
                return Some(val);
            }
            if !content.is_empty() {
                return Some(content.to_string());
            }
        }
    }
    None
}

// ── Portfolio-wide Scraper ──────────────────────────────────────────

/// Scrape SEC filings for all portfolio symbols (from broker/user state + kv_cache).
/// All DB access happens in spawn_blocking; HTTP is async.
pub async fn scrape_all_portfolio_symbols(
    db_path: PathBuf,
    scoped_symbols: Option<Vec<String>>,
) -> Result<ScrapeStats, String> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    // Step 1: collect caller-provided scope symbols, or fall back to the legacy
    // self-discovery path for non-UI callers/tests. UI-triggered scrapes must pass
    // the top-level broker Scope explicitly.
    let mut symbols: Vec<String> = if let Some(symbols) = scoped_symbols {
        normalize_sec_equity_symbols_preserving_order(symbols)
    } else {
        let db = db_path.clone();
        tokio::task::spawn_blocking(move || {
            let conn = open_conn(&db)?;
            let mut sym_set: std::collections::HashSet<String> = std::collections::HashSet::new();

            // Do not treat every `kraken-equities:*` bar-cache key as a SEC
            // target: the Kraken cache can contain the broad exchange universe,
            // which would turn "Scrape Now" into a quota-burning crawl. User/
            // broker symbols come from kv_cache below.

            // From compressed broker/user state: watchlist, positions, Kraken
            // positions. These are the symbols the user actually cares about in a
            // Kraken-equities-only session, and they are much smaller than the
            // broad cached equities universe.
            if let Ok(mut stmt) = conn.prepare(
                "SELECT value FROM kv_cache
                 WHERE key IN ('broker:watchlist', 'broker:positions', 'broker:kr_positions')",
            ) {
                if let Ok(rows) = stmt.query_map([], |row| row.get::<_, Vec<u8>>(0)) {
                    for row in rows.flatten() {
                        collect_equity_symbols_from_kv_blob(&row, &mut sym_set);
                    }
                }
            }

            // If the live/current universe sources are empty, keep the existing SEC
            // database warm by falling back to tickers we already know how to scrape.
            // Without this, sessions with no open positions/watchlist return
            // "0 tickers" and the filings database silently freezes even though
            // `sec_scrape_index` has a valid tracked universe from previous scrapes.
            if sym_set.is_empty() {
                if let Ok(mut stmt) = conn.prepare(
                    "SELECT DISTINCT ticker FROM sec_scrape_index
                     WHERE ticker != '' AND ticker IS NOT NULL",
                ) {
                    if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                        for row in rows.flatten() {
                            if let Some(sym) = normalize_sec_equity_symbol(&row) {
                                sym_set.insert(sym);
                            }
                        }
                    }
                }
            }
            let mut syms: Vec<String> = sym_set.into_iter().collect();
            syms.sort_unstable();

            Ok::<_, String>(syms)
        })
        .await
        .map_err(|e| format!("spawn_blocking: {e}"))??
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .pool_max_idle_per_host(2)
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let mut stats = ScrapeStats {
        tickers_scanned: 0,
        new_filings: 0,
        new_insider_trades: 0,
        new_alerts: 0,
        errors: Vec::new(),
    };

    // PERF: single batch load of last_scrape_date and cached CIKs for ALL tickers.
    // Broad tradable-universe SEC sync can be 12k+ symbols; doing a SQLite open
    // and SEC company_tickers.json fetch per symbol turns that into a crawl.
    let scrape_index: std::collections::HashMap<String, SecScrapeIndexState> = {
        let db = db_path.clone();
        tokio::task::spawn_blocking(move || -> Result<_, String> {
            let conn = open_conn(&db)?;
            let mut stmt = conn
                .prepare("SELECT ticker, last_scrape_date, filing_count, cik FROM sec_scrape_index")
                .map_err(|e| format!("prepare scrape_index: {e}"))?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, i64>(2).unwrap_or(0),
                        row.get::<_, Option<String>>(3)?,
                    ))
                })
                .map_err(|e| format!("query scrape_index: {e}"))?;
            let mut index = std::collections::HashMap::with_capacity(1024);
            for r in rows.flatten() {
                let ticker = r.0.to_uppercase();
                index.insert(
                    ticker,
                    SecScrapeIndexState {
                        last_scrape_date: r.1.filter(|date| !date.is_empty()),
                        filing_count: r.2,
                        cik: r.3.filter(|cik| !cik.is_empty()),
                    },
                );
            }
            Ok(index)
        })
        .await
        .map_err(|e| format!("spawn_blocking: {e}"))??
    };

    // Gap-first ordering: visit resolved CIKs with no indexed filings first, then
    // other never-scraped names, then zero-filing stale names, then oldest normal
    // refreshes, with already-scraped-today names last. Without this, interrupted
    // or quota-bounded passes can keep rechecking recently warm names while symbols
    // like FNGR sit at NULL/0 even though the CIK is known and SEC has filings.
    // The caller's active/watchlist priority order is preserved as the final
    // tiebreaker within each bucket.
    prioritize_sec_scrape_symbols(&mut symbols, &scrape_index, &today);

    let mut sec_ticker_cik_map: Option<std::collections::HashMap<String, String>> = None;

    for sym in &symbols {
        // O(1) HashMap lookup replaces the N+1 per-symbol SELECT.
        if scrape_index
            .get(sym)
            .and_then(|state| state.last_scrape_date.as_deref())
            == Some(today.as_str())
        {
            continue;
        }

        // Look up CIK. For broad universes, fetch SEC's ticker map once and
        // reuse it for every uncached symbol instead of hitting SEC once per
        // ticker before even reaching submissions.
        let cik = if let Some(cik) = scrape_index
            .get(sym)
            .and_then(|state| state.cik.as_ref())
            .filter(|cik| !cik.is_empty())
        {
            cik.clone()
        } else {
            if sec_ticker_cik_map.is_none() {
                match fetch_sec_ticker_cik_map(&client).await {
                    Ok(map) => sec_ticker_cik_map = Some(map),
                    Err(e) => {
                        stats.errors.push(format!("SEC ticker map: {e}"));
                        break;
                    }
                }
            }
            match sec_ticker_cik_map
                .as_ref()
                .and_then(|map| map.get(sym))
                .cloned()
            {
                Some(cik) => cik,
                None => {
                    stats.errors.push(format!("{sym}: CIK not found"));
                    continue;
                }
            }
        };

        // Rate limit between tickers
        tokio::time::sleep(std::time::Duration::from_millis(RATE_LIMIT_MS)).await;

        match scrape_filings_for_ticker(&db_path, &client, sym, &cik).await {
            Ok((filings, trades, alerts)) => {
                stats.tickers_scanned += 1;
                stats.new_filings += filings;
                stats.new_insider_trades += trades;
                stats.new_alerts += alerts;
            }
            Err(e) => {
                stats.errors.push(format!("{sym}: {e}"));
            }
        }
    }

    Ok(stats)
}

/// Check if a symbol looks like a US equity (not forex, commodities, etc.)
fn is_equity_symbol(sym: &str) -> bool {
    !sym.is_empty()
        && !sym.contains('/')
        && !sym.starts_with("XAU")
        && !sym.starts_with("XAG")
        && !sym.starts_with("XNG")
        && !sym.starts_with("XBR")
        && !sym.starts_with("XTI")
        && sym.len() <= 5
        && sym.chars().all(|c| c.is_ascii_alphabetic())
}

fn normalize_sec_equity_symbol(sym: &str) -> Option<String> {
    let mut sym = sym.trim().to_uppercase();
    if sym.is_empty() || sym.starts_with("__") || sym.contains('/') {
        return None;
    }
    // Kraken xStocks can be stored/transmitted as venue-qualified symbols
    // (WOK.EQ, etc.). SEC EDGAR lookup needs the underlying equity
    // ticker. Normalize before applying the equity filter so scoped SEC scrapes
    // don't silently drop xStock holdings.
    if let Some(stripped) = sym.strip_suffix(".EQ") {
        sym = stripped.to_string();
    } else if let Some(stripped) = sym.strip_suffix(".X") {
        sym = stripped.to_string();
    }
    if crate::core::news::is_crypto_symbol(&sym) {
        return None;
    }
    if is_equity_symbol(&sym) {
        Some(sym)
    } else {
        None
    }
}

fn normalize_sec_equity_symbols_preserving_order<I>(symbols: I) -> Vec<String>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for sym in symbols {
        let Some(normalized) = normalize_sec_equity_symbol(sym.as_ref()) else {
            continue;
        };
        if seen.insert(normalized.clone()) {
            out.push(normalized);
        }
    }
    out
}

fn collect_equity_symbols_from_kv_blob(
    compressed: &[u8],
    out: &mut std::collections::HashSet<String>,
) {
    let Ok(decompressed) = zstd::decode_all(compressed) else {
        return;
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&decompressed) else {
        return;
    };
    collect_equity_symbols_from_json(&value, false, out);
}

fn collect_equity_symbols_from_json(
    value: &serde_json::Value,
    symbol_context: bool,
    out: &mut std::collections::HashSet<String>,
) {
    match value {
        serde_json::Value::String(raw) => {
            if !symbol_context {
                return;
            }
            if let Some(sym) = normalize_sec_equity_symbol(raw) {
                out.insert(sym);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_equity_symbols_from_json(item, true, out);
            }
        }
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                let is_symbol_field = matches!(
                    key.as_str(),
                    "symbol" | "ticker" | "sym" | "asset" | "underlying_symbol"
                );
                collect_equity_symbols_from_json(child, is_symbol_field, out);
            }
        }
        _ => {}
    }
}

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

// ── Filing Diff Comparison ───────────────────────────────────────────────────

/// A chunk of diff output.
pub fn diff_filing_content(old: &str, new: &str) -> Vec<DiffChunk> {
    let old_paras: Vec<&str> = old
        .split("\n\n")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    let new_paras: Vec<&str> = new
        .split("\n\n")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    // Simple LCS-based diff at paragraph level
    let m = old_paras.len();
    let n = new_paras.len();
    // Build LCS table
    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if old_paras[i - 1] == new_paras[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }
    // Backtrack to build diff
    let mut chunks = Vec::new();
    let (mut i, mut j) = (m, n);
    let mut stack = Vec::new();
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old_paras[i - 1] == new_paras[j - 1] {
            stack.push(DiffChunk::Same(old_paras[i - 1].to_string()));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            stack.push(DiffChunk::Added(new_paras[j - 1].to_string()));
            j -= 1;
        } else if i > 0 {
            stack.push(DiffChunk::Removed(old_paras[i - 1].to_string()));
            i -= 1;
        }
    }
    stack.reverse();
    chunks.extend(stack);
    chunks
}

/// Find the previous filing of the same type for the same ticker.
pub fn find_previous_filing(
    conn: &Connection,
    ticker: &str,
    form_type: &str,
    current_date: &str,
) -> Result<Option<SecFiling>, String> {
    let mut stmt = conn.prepare(
        "SELECT id, ticker, form_type, accession_number, filing_date, url, company_name, importance_score, category, summary, insider_flag, created_at
         FROM sec_filings WHERE ticker = ?1 AND form_type = ?2 AND filing_date < ?3
         ORDER BY filing_date DESC LIMIT 1"
    ).map_err(|e| format!("Prepare previous filing failed: {e}"))?;

    stmt.query_row(
        rusqlite::params![ticker.to_uppercase(), form_type, current_date],
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
    .optional()
    .map_err(|e| format!("Query previous filing failed: {e}"))
}

#[cfg(test)]
mod tests;
