//! SEC Filing Scraper — local database of SEC filings with Form 4 insider trade parsing.
//!
//! Fetches filings from SEC EDGAR (data.sec.gov), stores them in SQLite,
//! parses Form 4 insider trades, and generates alerts for significant insider activity.
//!
//! Architecture note: rusqlite::Connection is not Send, so all DB operations happen
//! inside `tokio::task::spawn_blocking` closures with short-lived connections.
//! HTTP fetches happen on the async runtime between DB calls.

use rusqlite::params;
use std::path::{Path, PathBuf};

mod content_text;
mod diff;
mod insider_form4;
mod keywords;
mod query;
mod schema;
mod scoring;
mod scrape_index;
mod storage_content;
mod summary;
mod symbols;
mod types;
#[cfg(test)]
use content_text::decode_html_entities;
pub use content_text::{polish_filing_text, strip_html_to_text};
pub use diff::{diff_filing_content, find_previous_filing};
use insider_form4::fetch_and_parse_form4;
pub use insider_form4::form4_transaction_code_label;
#[cfg(test)]
use insider_form4::{extract_transactions, extract_xml_value};
pub use keywords::{add_keyword, check_keywords, check_keywords_in, get_keywords, remove_keyword};
pub use query::{
    dismiss_alert, get_all_filings, get_all_insider_trades, get_filing_alerts, get_insider_trades,
    get_recent_filings,
};
pub use schema::create_sec_tables;
use schema::open_conn;
pub use scoring::compute_importance;
use scoring::{RELEVANT_FORMS, categorize_form};
use scrape_index::{SecScrapeIndexState, prioritize_sec_scrape_symbols, ticker_scraped_on};
pub use storage_content::{
    filing_content_stats, get_filing_content, get_unfetched_filings,
    mark_filing_content_fetch_failed, search_filings_fts, store_filing_content,
};
pub use summary::summarize_filing;
#[cfg(test)]
use symbols::is_equity_symbol;
use symbols::{
    collect_equity_symbols_from_kv_blob, normalize_sec_equity_symbol,
    normalize_sec_equity_symbols_preserving_order,
};
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

#[cfg(test)]
mod tests;
