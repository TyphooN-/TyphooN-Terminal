//! SEC Filing Scraper — local database of SEC filings with Form 4 insider trade parsing.
//!
//! Fetches filings from SEC EDGAR (data.sec.gov), stores them in SQLite,
//! parses Form 4 insider trades, and generates alerts for significant insider activity.
//!
//! Architecture note: rusqlite::Connection is not Send, so all DB operations happen
//! inside `tokio::task::spawn_blocking` closures with short-lived connections.
//! HTTP fetches happen on the async runtime between DB calls.

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const USER_AGENT: &str = "TyphooN-Terminal/0.1 (support@marketwizardry.org)";

/// Rate limit sleep between SEC requests (200ms = 5 req/sec, well under 10/sec limit).
const RATE_LIMIT_MS: u64 = 200;

/// All SEC filing types we track — comprehensive coverage for trading signals.
const RELEVANT_FORMS: &[&str] = &[
    // Core financials
    "10-K", "10-Q", "20-F", "8-K",
    // Amended (restated = red flag)
    "10-K/A", "10-Q/A", "8-K/A",
    // Late filing (distress signal)
    "NT 10-K", "NT 10-Q",
    // Insider trades
    "4", "3", "5",
    // Proxy/governance
    "DEF 14A", "DEFA14A", "PREM14A",
    // Shareholder disclosures (activist/institutional)
    "SC 13D", "SC 13D/A", "SC 13G", "SC 13G/A", "13F-HR",
    // Offerings/dilution
    "S-1", "S-3", "S-4", "424B5", "424B2", "424B4",
    // M&A
    "SC TO-T", "SC TO-I", "SC 14D9",
    // Deregistration (delisting risk)
    "15-12B", "15-12G",
    // SEC scrutiny
    "CORRESP",
    // Employee plans
    "11-K",
];

// ── Data Types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecFiling {
    pub id: i64,
    pub ticker: String,
    pub form_type: String,
    pub accession_number: String,
    pub filing_date: String,
    pub url: String,
    pub company_name: String,
    pub importance_score: i32,
    pub category: String,
    pub summary: String,
    pub insider_flag: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsiderTrade {
    pub id: i64,
    pub ticker: String,
    pub accession_number: String,
    pub insider_name: String,
    pub insider_title: String,
    pub transaction_date: String,
    pub transaction_type: String,
    pub shares: f64,
    pub price: f64,
    pub aggregate_value: f64,
    pub is_officer: bool,
    pub is_director: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilingAlert {
    pub id: i64,
    pub ticker: String,
    pub alert_type: String,
    pub message: String,
    pub filing_accession: String,
    pub importance: i32,
    pub created_at: i64,
    pub dismissed: bool,
    pub dismissed_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeStats {
    pub tickers_scanned: usize,
    pub new_filings: usize,
    pub new_insider_trades: usize,
    pub new_alerts: usize,
    pub errors: Vec<String>,
}

/// A filing parsed from the SEC JSON but not yet inserted.
#[derive(Debug, Clone)]
struct PendingFiling {
    ticker: String,
    form_type: String,
    accession_number: String,
    filing_date: String,
    url: String,
    company_name: String,
    importance_score: i32,
    category: String,
    insider_flag: bool,
    is_late: bool,
}

// ── Helper: open a WAL connection ───────────────────────────────────

fn open_conn(db_path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(db_path)
        .map_err(|e| format!("SQLite open failed: {e}"))?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
        .map_err(|e| format!("Pragma failed: {e}"))?;
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

    // Schema migration: add updated_at column for incremental LAN sync
    let _ = conn.execute("ALTER TABLE sec_scrape_index ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0", []);

    Ok(())
}

// ── Importance Scoring ──────────────────────────────────────────────

pub fn compute_importance(form_type: &str, is_insider_sell: bool, _is_late: bool) -> i32 {
    let (base, _cat) = importance_and_category(form_type);
    let mut score = base;
    if is_insider_sell { score += 15; }
    score.min(100)
}

/// Returns (importance_score, category) for a form type.
fn importance_and_category(form_type: &str) -> (i32, &'static str) {
    match form_type {
        "15-12B" | "15-12G" => (85, "DELISTING"),
        "SC TO-T" | "SC TO-I" | "SC 14D9" => (80, "ACQUISITION"),
        "10-K/A" | "10-Q/A" | "8-K/A" => (75, "AMENDED"),
        "NT 10-K" | "NT 10-Q" => (75, "LATE_FILING"),
        "SC 13D" | "SC 13D/A" => (70, "ACTIVIST"),
        "PREM14A" => (70, "ACQUISITION"),
        "424B5" | "424B2" | "424B4" => (65, "DILUTION"),
        "S-3" => (60, "DILUTION"),
        "CORRESP" => (45, "SEC_SCRUTINY"),
        "10-K" | "20-F" => (40, "EARNINGS"),
        "S-1" | "S-4" => (40, "OFFERING"),
        "8-K" => (35, "MATERIAL_EVENT"),
        "SC 13G" | "SC 13G/A" => (35, "INSTITUTIONAL"),
        "DEFA14A" => (35, "GOVERNANCE"),
        "10-Q" => (30, "EARNINGS"),
        "13F-HR" => (30, "INSTITUTIONAL"),
        "4" => (25, "INSIDER_ACTIVITY"),
        "3" | "5" => (20, "INSIDER_ACTIVITY"),
        "DEF 14A" => (20, "GOVERNANCE"),
        "11-K" => (15, "GOVERNANCE"),
        _ => (10, "OTHER"),
    }
}

fn categorize_form(form_type: &str) -> &'static str {
    importance_and_category(form_type).1
}

// ── CIK Lookup ──────────────────────────────────────────────────────

/// Look up CIK for a ticker from the SEC company_tickers.json endpoint.
/// Returns the CIK as a zero-padded 10-digit string.
async fn lookup_cik_online(client: &reqwest::Client, ticker: &str) -> Result<String, String> {
    let resp = client
        .get("https://www.sec.gov/files/company_tickers.json")
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("SEC ticker map request failed: {e}"))?;

    let tickers_json: serde_json::Value = resp.json().await
        .map_err(|e| format!("SEC ticker map parse failed: {e}"))?;

    let upper_ticker = ticker.to_uppercase();
    if let Some(obj) = tickers_json.as_object() {
        for (_, v) in obj {
            if v["ticker"].as_str() == Some(upper_ticker.as_str()) {
                if let Some(cik) = v["cik_str"].as_u64()
                    .or_else(|| v["cik_str"].as_str().and_then(|s| s.parse().ok()))
                {
                    return Ok(format!("{:010}", cik));
                }
            }
        }
    }
    Err(format!("CIK not found for {ticker}"))
}

/// Get CIK from local DB cache or fetch from SEC.
async fn get_cik(db_path: &Path, client: &reqwest::Client, ticker: &str) -> Result<String, String> {
    let db = db_path.to_path_buf();
    let t = ticker.to_uppercase();

    // Check local cache first (blocking)
    let cached = {
        let db2 = db.clone();
        let t2 = t.clone();
        tokio::task::spawn_blocking(move || {
            let conn = open_conn(&db2)?;
            let cik: Option<String> = conn.query_row(
                "SELECT cik FROM sec_scrape_index WHERE ticker = ?1",
                params![t2],
                |row| row.get(0),
            ).ok().flatten();
            Ok::<_, String>(cik)
        }).await.map_err(|e| format!("spawn_blocking: {e}"))??
    };

    if let Some(ref cik) = cached {
        if !cik.is_empty() {
            return Ok(cik.clone());
        }
    }

    // Fetch from SEC (async)
    let cik = lookup_cik_online(client, ticker).await?;

    // Cache it (blocking)
    {
        let db2 = db.clone();
        let t2 = t.clone();
        let cik2 = cik.clone();
        tokio::task::spawn_blocking(move || {
            let conn = open_conn(&db2)?;
            conn.execute(
                "INSERT OR REPLACE INTO sec_scrape_index (ticker, cik, last_scrape_date, filing_count, updated_at)
                 VALUES (?1, ?2, COALESCE((SELECT last_scrape_date FROM sec_scrape_index WHERE ticker = ?1), NULL),
                         COALESCE((SELECT filing_count FROM sec_scrape_index WHERE ticker = ?1), 0), ?3)",
                params![t2, cik2, chrono::Utc::now().timestamp()],
            ).map_err(|e| format!("Cache CIK failed: {e}"))?;
            Ok::<_, String>(())
        }).await.map_err(|e| format!("spawn_blocking: {e}"))??;
    }

    Ok(cik)
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
    // Step 1: Fetch submissions JSON (async)
    let url = format!("https://data.sec.gov/submissions/CIK{cik}.json");
    let resp = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("SEC submissions fetch failed for {ticker}: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("SEC submissions HTTP {} for {ticker}", resp.status()));
    }

    let body: serde_json::Value = resp.json().await
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

    for i in 0..forms.len().min(dates.len()).min(accessions.len()).min(primary_docs.len()) {
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

    // Step 3: Insert new filings (blocking), get back which ones are actually new
    let db = db_path.to_path_buf();
    let pending_clone = pending.clone();
    let new_filings: Vec<PendingFiling> = tokio::task::spawn_blocking(move || {
        let conn = open_conn(&db)?;
        let now = chrono::Utc::now().timestamp();
        let mut inserted = Vec::new();

        for f in pending_clone {
            let exists: bool = conn.query_row(
                "SELECT COUNT(*) FROM sec_filings WHERE accession_number = ?1",
                params![f.accession_number],
                |row| row.get::<_, i64>(0),
            ).unwrap_or(0) > 0;

            if exists { continue; }

            conn.execute(
                "INSERT OR IGNORE INTO sec_filings (ticker, form_type, accession_number, filing_date, url, company_name, importance_score, category, summary, insider_flag, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    f.ticker, f.form_type, f.accession_number, f.filing_date, f.url,
                    f.company_name, f.importance_score, f.category, "", f.insider_flag, now
                ],
            ).map_err(|e| format!("Insert filing failed: {e}"))?;

            // Create alerts for critical filing types
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
                // Deduplicate: only one alert per (ticker, alert_type) per day
                let _today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let existing: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM sec_filing_alerts WHERE ticker = ?1 AND alert_type = ?2 AND created_at > ?3",
                    params![f.ticker, alert_type, chrono::Utc::now().timestamp() - 86400],
                    |row| row.get(0),
                ).unwrap_or(0);
                if existing == 0 {
                conn.execute(
                    "INSERT INTO sec_filing_alerts (ticker, alert_type, message, filing_accession, importance, created_at, dismissed)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, FALSE)",
                    params![f.ticker, alert_type, message, f.accession_number, f.importance_score, now],
                ).ok();
                }
            }

            inserted.push(f);
        }
        Ok::<_, String>(inserted)
    }).await.map_err(|e| format!("spawn_blocking: {e}"))??;

    let num_new = new_filings.len();
    let mut total_insider_trades = 0usize;
    let mut total_alerts = new_filings.iter().filter(|f| f.is_late).count();

    // Step 4: For each new Form 4, fetch and parse insider trades
    for f in &new_filings {
        if !f.insider_flag { continue; }

        tokio::time::sleep(std::time::Duration::from_millis(RATE_LIMIT_MS)).await;

        match fetch_and_parse_form4(db_path, client, &f.ticker, &f.accession_number, &f.url).await {
            Ok((trades, alerts)) => {
                total_insider_trades += trades;
                total_alerts += alerts;
            }
            Err(e) => {
                tracing::warn!("Form 4 parse failed for {} {}: {e}", f.ticker, f.accession_number);
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
    // Async: fetch the filing
    let resp = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Form 4 fetch failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Form 4 HTTP {}", resp.status()));
    }

    let body = resp.text().await
        .map_err(|e| format!("Form 4 read failed: {e}"))?;

    // Parse in-memory (no DB needed)
    let insider_name = extract_xml_value(&body, "rptOwnerName")
        .unwrap_or_else(|| "Unknown".to_string());
    let insider_title = extract_xml_value(&body, "officerTitle")
        .unwrap_or_default();
    let is_officer = body.contains("<isOfficer>true</isOfficer>")
        || body.contains("<isOfficer>1</isOfficer>");
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

                let code = extract_xml_value(block, "transactionCode")
                    .unwrap_or_default();
                let shares = extract_xml_value(block, "transactionShares")
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .unwrap_or(0.0);
                let price = extract_xml_value(block, "transactionPricePerShare")
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .unwrap_or(0.0);
                let date = extract_xml_value(block, "transactionDate")
                    .unwrap_or_default();

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

/// Scrape SEC filings for all portfolio symbols (from darwin_deals + kv_cache).
/// All DB access happens in spawn_blocking; HTTP is async.
pub async fn scrape_all_portfolio_symbols(db_path: PathBuf) -> Result<ScrapeStats, String> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    // Step 1: Collect portfolio symbols (blocking)
    let db = db_path.clone();
    let symbols: Vec<String> = tokio::task::spawn_blocking(move || {
        let conn = open_conn(&db)?;
        let mut syms: Vec<String> = Vec::new();

        // From darwin_deals
        if let Ok(mut stmt) = conn.prepare(
            "SELECT DISTINCT symbol FROM darwin_deals WHERE symbol != '' AND symbol IS NOT NULL"
        ) {
            if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                for row in rows.flatten() {
                    let sym = row.trim().to_uppercase();
                    if is_equity_symbol(&sym) && !syms.contains(&sym) {
                        syms.push(sym);
                    }
                }
            }
        }

        // From kv_cache mt5 keys
        if let Ok(mut stmt) = conn.prepare(
            "SELECT DISTINCT key FROM kv_cache WHERE key LIKE 'mt5:%'"
        ) {
            if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                for row in rows.flatten() {
                    let parts: Vec<&str> = row.split(':').collect();
                    if parts.len() >= 2 {
                        let sym = parts[1].to_uppercase();
                        if is_equity_symbol(&sym) && !syms.contains(&sym) {
                            syms.push(sym);
                        }
                    }
                }
            }
        }

        Ok::<_, String>(syms)
    }).await.map_err(|e| format!("spawn_blocking: {e}"))??;

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

    for sym in &symbols {
        // Check if already scraped today (blocking)
        let db = db_path.clone();
        let s = sym.clone();
        let today2 = today.clone();
        let skip = tokio::task::spawn_blocking(move || {
            let conn = open_conn(&db)?;
            let last: Option<String> = conn.query_row(
                "SELECT last_scrape_date FROM sec_scrape_index WHERE ticker = ?1",
                params![s],
                |row| row.get(0),
            ).ok().flatten();
            Ok::<_, String>(last.as_deref() == Some(today2.as_str()))
        }).await.map_err(|e| format!("spawn_blocking: {e}"))??;

        if skip { continue; }

        // Look up CIK
        let cik = match get_cik(&db_path, &client, sym).await {
            Ok(c) => c,
            Err(e) => {
                stats.errors.push(format!("{sym}: {e}"));
                continue;
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

// ── Query Functions (synchronous — called from spawn_blocking in commands) ──

/// Get recent filings, optionally filtered by ticker.
pub fn get_recent_filings(
    conn: &Connection,
    ticker: Option<&str>,
    limit: usize,
) -> Result<Vec<SecFiling>, String> {
    let limit = limit.min(1000);
    let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(t) = ticker {
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

    let mut stmt = conn.prepare(&sql).map_err(|e| format!("Prepare failed: {e}"))?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(params_refs.as_slice(), |row| {
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
    }).map_err(|e| format!("Query failed: {e}"))?;

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
        .format("%Y-%m-%d").to_string();

    let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(t) = ticker {
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

    let mut stmt = conn.prepare(&sql).map_err(|e| format!("Prepare failed: {e}"))?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(params_refs.as_slice(), |row| {
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
    }).map_err(|e| format!("Query failed: {e}"))?;

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
    let mut stmt = conn.prepare(
        "SELECT id, ticker, alert_type, message, COALESCE(filing_accession, ''), importance, created_at, dismissed, COALESCE(dismissed_reason, '')
         FROM sec_filing_alerts WHERE dismissed = ?1 ORDER BY created_at DESC"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let rows = stmt.query_map(params![dismissed], |row| {
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
    }).map_err(|e| format!("Query failed: {e}"))?;

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
    ).map_err(|e| format!("Dismiss alert failed: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        create_sec_tables(&conn).unwrap();
        conn
    }

    fn insert_filing(conn: &Connection, ticker: &str, form_type: &str, accession: &str, date: &str) {
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO sec_filings (ticker, form_type, accession_number, filing_date, url, company_name, importance_score, category, summary, insider_flag, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![ticker, form_type, accession, date,
                    format!("https://sec.gov/test/{accession}"), "Test Corp",
                    compute_importance(form_type, false, false),
                    categorize_form(form_type), "", form_type == "4", now],
        ).unwrap();
    }

    fn insert_insider_trade(conn: &Connection, ticker: &str, accession: &str, name: &str, txn_type: &str, date: &str, shares: f64, price: f64) {
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO sec_insider_trades (ticker, accession_number, insider_name, insider_title, transaction_date, transaction_type, shares, price, aggregate_value, is_officer, is_director, created_at)
             VALUES (?1, ?2, ?3, 'CEO', ?4, ?5, ?6, ?7, ?8, TRUE, FALSE, ?9)",
            params![ticker, accession, name, date, txn_type, shares, price, shares * price, now],
        ).unwrap();
    }

    fn insert_alert(conn: &Connection, ticker: &str, alert_type: &str, message: &str, dismissed: bool) -> i64 {
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO sec_filing_alerts (ticker, alert_type, message, filing_accession, importance, created_at, dismissed, dismissed_reason)
             VALUES (?1, ?2, ?3, 'acc-001', 50, ?4, ?5, '')",
            params![ticker, alert_type, message, now, dismissed],
        ).unwrap();
        conn.last_insert_rowid()
    }

    // ── create_sec_tables ──────────────────────────────────────────

    #[test]
    fn create_tables_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        create_sec_tables(&conn).unwrap();
        create_sec_tables(&conn).unwrap(); // second call should not fail
    }

    // ── compute_importance ─────────────────────────────────────────

    #[test]
    fn compute_importance_base_scores() {
        assert_eq!(compute_importance("10-K", false, false), 40);
        assert_eq!(compute_importance("10-Q", false, false), 30);
        assert_eq!(compute_importance("8-K", false, false), 35);
        assert_eq!(compute_importance("4", false, false), 25);
    }

    #[test]
    fn compute_importance_insider_sell_boost() {
        let base = compute_importance("4", false, false);
        let with_sell = compute_importance("4", true, false);
        assert_eq!(with_sell, base + 15);
    }

    #[test]
    fn compute_importance_capped_at_100() {
        // 15-12B has base 85, + 15 for insider sell = 100 (capped)
        assert_eq!(compute_importance("15-12B", true, false), 100);
    }

    #[test]
    fn compute_importance_unknown_form() {
        assert_eq!(compute_importance("UNKNOWN-FORM", false, false), 10);
    }

    // ── importance_and_category ────────────────────────────────────

    #[test]
    fn categorize_form_categories() {
        assert_eq!(categorize_form("10-K"), "EARNINGS");
        assert_eq!(categorize_form("10-Q"), "EARNINGS");
        assert_eq!(categorize_form("SC 13D"), "ACTIVIST");
        assert_eq!(categorize_form("15-12B"), "DELISTING");
        assert_eq!(categorize_form("4"), "INSIDER_ACTIVITY");
        assert_eq!(categorize_form("S-3"), "DILUTION");
        assert_eq!(categorize_form("CORRESP"), "SEC_SCRUTINY");
        assert_eq!(categorize_form("RANDOM"), "OTHER");
    }

    // ── is_equity_symbol ───────────────────────────────────────────

    #[test]
    fn is_equity_symbol_valid() {
        assert!(is_equity_symbol("AAPL"));
        assert!(is_equity_symbol("MSFT"));
        assert!(is_equity_symbol("GOOG"));
        assert!(is_equity_symbol("A"));    // single letter tickers exist
    }

    #[test]
    fn is_equity_symbol_invalid() {
        assert!(!is_equity_symbol(""));         // empty
        assert!(!is_equity_symbol("EUR/USD"));  // forex with slash
        assert!(!is_equity_symbol("XAUUSD"));   // gold
        assert!(!is_equity_symbol("XAGUSD"));   // silver
        assert!(!is_equity_symbol("XNGUSD"));   // natural gas
        assert!(!is_equity_symbol("TOOLONG"));  // > 5 chars
        assert!(!is_equity_symbol("AB123"));    // contains digits
    }

    // ── extract_xml_value ──────────────────────────────────────────

    #[test]
    fn extract_xml_value_simple() {
        let xml = "<ownershipDocument><rptOwnerName>John Doe</rptOwnerName></ownershipDocument>";
        assert_eq!(extract_xml_value(xml, "rptOwnerName"), Some("John Doe".to_string()));
    }

    #[test]
    fn extract_xml_value_nested_value_tag() {
        let xml = "<transactionShares><value>10000</value></transactionShares>";
        assert_eq!(extract_xml_value(xml, "transactionShares"), Some("10000".to_string()));
    }

    #[test]
    fn extract_xml_value_missing_tag() {
        let xml = "<document><name>Test</name></document>";
        assert_eq!(extract_xml_value(xml, "missing"), None);
    }

    #[test]
    fn extract_xml_value_empty_content() {
        let xml = "<officerTitle></officerTitle>";
        assert_eq!(extract_xml_value(xml, "officerTitle"), None);
    }

    // ── extract_transactions ───────────────────────────────────────

    #[test]
    fn extract_transactions_non_derivative() {
        let xml = r#"
        <ownershipDocument>
            <nonDerivativeTransaction>
                <transactionCode>S</transactionCode>
                <transactionShares><value>5000</value></transactionShares>
                <transactionPricePerShare><value>150.50</value></transactionPricePerShare>
                <transactionDate><value>2024-03-15</value></transactionDate>
            </nonDerivativeTransaction>
        </ownershipDocument>
        "#;
        let txns = extract_transactions(xml);
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].code, "S");
        assert_eq!(txns[0].shares, 5000.0);
        assert!((txns[0].price - 150.50).abs() < 0.01);
        assert_eq!(txns[0].date, "2024-03-15");
    }

    #[test]
    fn extract_transactions_multiple() {
        let xml = r#"
        <doc>
            <nonDerivativeTransaction>
                <transactionCode>P</transactionCode>
                <transactionShares><value>1000</value></transactionShares>
                <transactionPricePerShare><value>100.00</value></transactionPricePerShare>
                <transactionDate><value>2024-01-01</value></transactionDate>
            </nonDerivativeTransaction>
            <nonDerivativeTransaction>
                <transactionCode>S</transactionCode>
                <transactionShares><value>2000</value></transactionShares>
                <transactionPricePerShare><value>110.00</value></transactionPricePerShare>
                <transactionDate><value>2024-01-02</value></transactionDate>
            </nonDerivativeTransaction>
        </doc>
        "#;
        let txns = extract_transactions(xml);
        assert_eq!(txns.len(), 2);
        assert_eq!(txns[0].code, "P");
        assert_eq!(txns[1].code, "S");
    }

    #[test]
    fn extract_transactions_empty_body() {
        let txns = extract_transactions("<doc>nothing relevant here</doc>");
        assert!(txns.is_empty());
    }

    #[test]
    fn extract_transactions_derivative() {
        let xml = r#"
        <doc>
            <derivativeTransaction>
                <transactionCode>A</transactionCode>
                <transactionShares><value>3000</value></transactionShares>
                <transactionPricePerShare><value>0</value></transactionPricePerShare>
                <transactionDate><value>2024-06-01</value></transactionDate>
            </derivativeTransaction>
        </doc>
        "#;
        let txns = extract_transactions(xml);
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].code, "A");
    }

    // ── get_recent_filings ─────────────────────────────────────────

    #[test]
    fn get_recent_filings_all() {
        let conn = setup_test_db();
        insert_filing(&conn, "AAPL", "10-K", "acc-001", "2024-03-01");
        insert_filing(&conn, "AAPL", "10-Q", "acc-002", "2024-06-01");
        insert_filing(&conn, "MSFT", "8-K", "acc-003", "2024-05-15");

        let filings = get_recent_filings(&conn, None, 100).unwrap();
        assert_eq!(filings.len(), 3);
        // Ordered by filing_date DESC
        assert_eq!(filings[0].filing_date, "2024-06-01");
        assert_eq!(filings[1].filing_date, "2024-05-15");
    }

    #[test]
    fn get_recent_filings_filtered_by_ticker() {
        let conn = setup_test_db();
        insert_filing(&conn, "AAPL", "10-K", "acc-001", "2024-03-01");
        insert_filing(&conn, "MSFT", "10-Q", "acc-002", "2024-06-01");

        let filings = get_recent_filings(&conn, Some("AAPL"), 100).unwrap();
        assert_eq!(filings.len(), 1);
        assert_eq!(filings[0].ticker, "AAPL");
    }

    #[test]
    fn get_recent_filings_respects_limit() {
        let conn = setup_test_db();
        for i in 0..10 {
            insert_filing(&conn, "AAPL", "10-Q", &format!("acc-{i:03}"), &format!("2024-{:02}-01", i + 1));
        }

        let filings = get_recent_filings(&conn, None, 3).unwrap();
        assert_eq!(filings.len(), 3);
    }

    #[test]
    fn get_recent_filings_empty_db() {
        let conn = setup_test_db();
        let filings = get_recent_filings(&conn, None, 100).unwrap();
        assert!(filings.is_empty());
    }

    // ── get_insider_trades ─────────────────────────────────────────

    #[test]
    fn get_insider_trades_recent() {
        let conn = setup_test_db();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        insert_insider_trade(&conn, "AAPL", "acc-001", "Tim Cook", "S", &today, 10000.0, 195.0);
        insert_insider_trade(&conn, "AAPL", "acc-002", "Jeff Williams", "P", &today, 5000.0, 190.0);

        let trades = get_insider_trades(&conn, Some("AAPL"), 30).unwrap();
        assert_eq!(trades.len(), 2);
        assert_eq!(trades[0].insider_name, "Tim Cook");
    }

    #[test]
    fn get_insider_trades_all_tickers() {
        let conn = setup_test_db();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        insert_insider_trade(&conn, "AAPL", "acc-001", "Tim Cook", "S", &today, 10000.0, 195.0);
        insert_insider_trade(&conn, "MSFT", "acc-002", "Satya Nadella", "S", &today, 5000.0, 420.0);

        let trades = get_insider_trades(&conn, None, 30).unwrap();
        assert_eq!(trades.len(), 2);
    }

    #[test]
    fn get_insider_trades_old_excluded() {
        let conn = setup_test_db();
        // Insert a trade from 60 days ago
        let old_date = (chrono::Utc::now() - chrono::Duration::days(60))
            .format("%Y-%m-%d").to_string();
        insert_insider_trade(&conn, "AAPL", "acc-001", "Tim Cook", "S", &old_date, 10000.0, 195.0);

        let trades = get_insider_trades(&conn, None, 30).unwrap();
        assert!(trades.is_empty());
    }

    // ── get_filing_alerts / dismiss_alert ──────────────────────────

    #[test]
    fn get_filing_alerts_undismissed() {
        let conn = setup_test_db();
        insert_alert(&conn, "AAPL", "LATE_FILING", "AAPL: Late filing", false);
        insert_alert(&conn, "MSFT", "ACTIVIST", "MSFT: Activist position", false);
        insert_alert(&conn, "GOOG", "RESTATEMENT", "GOOG: dismissed", true);

        let active = get_filing_alerts(&conn, false).unwrap();
        assert_eq!(active.len(), 2);

        let dismissed = get_filing_alerts(&conn, true).unwrap();
        assert_eq!(dismissed.len(), 1);
        assert_eq!(dismissed[0].ticker, "GOOG");
    }

    #[test]
    fn dismiss_alert_works() {
        let conn = setup_test_db();
        let id = insert_alert(&conn, "AAPL", "LATE_FILING", "AAPL: Late filing", false);

        // Before dismiss
        let active = get_filing_alerts(&conn, false).unwrap();
        assert_eq!(active.len(), 1);

        dismiss_alert(&conn, id, "Reviewed and not material").unwrap();

        // After dismiss
        let active = get_filing_alerts(&conn, false).unwrap();
        assert!(active.is_empty());

        let dismissed = get_filing_alerts(&conn, true).unwrap();
        assert_eq!(dismissed.len(), 1);
        assert_eq!(dismissed[0].dismissed_reason, "Reviewed and not material");
    }

    #[test]
    fn dismiss_alert_nonexistent_id() {
        let conn = setup_test_db();
        // Should succeed (UPDATE affects 0 rows, no error)
        dismiss_alert(&conn, 99999, "no such alert").unwrap();
    }

    // ── get_filing_alerts field mapping ────────────────────────────

    #[test]
    fn filing_alert_fields_populated() {
        let conn = setup_test_db();
        insert_alert(&conn, "TSLA", "DILUTION_RISK", "TSLA: Shelf reg", false);

        let alerts = get_filing_alerts(&conn, false).unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].ticker, "TSLA");
        assert_eq!(alerts[0].alert_type, "DILUTION_RISK");
        assert_eq!(alerts[0].message, "TSLA: Shelf reg");
        assert_eq!(alerts[0].filing_accession, "acc-001");
        assert!(!alerts[0].dismissed);
    }

    // ── get_recent_filings field mapping ───────────────────────────

    #[test]
    fn filing_fields_populated() {
        let conn = setup_test_db();
        insert_filing(&conn, "NVDA", "SC 13D", "acc-activist-001", "2024-07-01");

        let filings = get_recent_filings(&conn, Some("NVDA"), 10).unwrap();
        assert_eq!(filings.len(), 1);
        assert_eq!(filings[0].ticker, "NVDA");
        assert_eq!(filings[0].form_type, "SC 13D");
        assert_eq!(filings[0].category, "ACTIVIST");
        assert_eq!(filings[0].importance_score, 70);
        assert!(!filings[0].insider_flag);
    }
}
