//! Research API helpers — company profiles, earnings, transcripts, IPOs, peers,
//! press releases, social sentiment, commodities futures quotes.
//!
//! Sources:
//! - Finnhub free tier: /stock/profile2, /stock/peers, /stock/earnings,
//!   /stock/social-sentiment, /press-releases, /calendar/ipo
//! - FMP free tier: /earning_call_transcript, /historical/earning_calendar
//! - Yahoo Finance: /v7/finance/quote (commodities, cross-asset quotes)
//!
//! All functions take an existing reqwest::Client so callers control the HTTP stack
//! (rate limiting, user-agent, timeouts).
//!
//! Research results are cached in SQLite so MT5/Darwinex symbols only need to hit
//! the APIs once per scrape cycle — the DES/PEERS/EARNINGS/PRESS/SENTIMENT/
//! TRANSCRIPTS windows read from cache first and fall back to live fetch.

mod types;
pub use types::*;
mod godel;
pub use godel::*;
mod technical;
pub use technical::*;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

// ── Data Types ─────────────────────────────────────────────────────────────

/// Unified company profile — DES command backing data.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompanyProfile {
    pub symbol: String,
    pub name: String,
    pub exchange: String,
    pub country: String,
    pub currency: String,
    pub industry: String,
    pub sector: String,
    pub website: String,
    pub logo: String,
    pub phone: String,
    pub ipo_date: String,
    pub market_cap: f64,            // in USD millions (Finnhub native unit)
    pub shares_outstanding: f64,    // in millions
}

/// One row in the earnings history (actual vs estimate EPS).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EarningRow {
    pub period: String,    // YYYY-MM-DD
    pub actual: Option<f64>,
    pub estimate: Option<f64>,
    pub surprise: Option<f64>,
    pub surprise_pct: Option<f64>,
    pub quarter: Option<i32>,
    pub year: Option<i32>,
}

/// IPO calendar row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IpoEvent {
    pub date: String,
    pub symbol: String,
    pub name: String,
    pub exchange: String,
    pub price_range: String,
    pub shares: i64,
    pub total_value: f64,
    pub status: String,
}

/// Earnings call transcript list entry (metadata only).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TranscriptMeta {
    pub symbol: String,
    pub quarter: i32,
    pub year: i32,
    pub date: String,
}

/// Full transcript content.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Transcript {
    pub symbol: String,
    pub quarter: i32,
    pub year: i32,
    pub date: String,
    pub content: String,
}

/// Social sentiment snapshot (Reddit + Twitter combined from Finnhub).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SocialSentimentRow {
    pub source: String,      // "reddit" | "twitter"
    pub at_time: String,
    pub mention: i64,
    pub positive_mention: i64,
    pub negative_mention: i64,
    pub positive_score: f64,
    pub negative_score: f64,
    pub score: f64,
}

/// Press release item.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PressRelease {
    pub symbol: String,
    pub datetime: String,
    pub headline: String,
    pub description: String,
    pub url: String,
}

/// Commodity futures quote row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommodityQuote {
    pub symbol: String,      // e.g. "GC=F"
    pub display: String,     // e.g. "Gold"
    pub price: f64,
    pub change: f64,
    pub change_pct: f64,
}

// ── Finnhub fetchers ───────────────────────────────────────────────────────

/// Finnhub /stock/profile2 — company profile.
pub async fn fetch_finnhub_profile(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<CompanyProfile, String> {
    if token.is_empty() { return Err("Finnhub API key required".into()); }
    let resp = client
        .get("https://finnhub.io/api/v1/stock/profile2")
        .query(&[("symbol", symbol), ("token", token)])
        .send().await
        .map_err(|e| format!("Finnhub profile failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub profile: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("Finnhub profile parse: {e}"))?;
    Ok(CompanyProfile {
        symbol: symbol.to_uppercase(),
        name: v["name"].as_str().unwrap_or("").to_string(),
        exchange: v["exchange"].as_str().unwrap_or("").to_string(),
        country: v["country"].as_str().unwrap_or("").to_string(),
        currency: v["currency"].as_str().unwrap_or("").to_string(),
        industry: v["finnhubIndustry"].as_str().unwrap_or("").to_string(),
        sector: v["gind"].as_str().unwrap_or("").to_string(),
        website: v["weburl"].as_str().unwrap_or("").to_string(),
        logo: v["logo"].as_str().unwrap_or("").to_string(),
        phone: v["phone"].as_str().unwrap_or("").to_string(),
        ipo_date: v["ipo"].as_str().unwrap_or("").to_string(),
        market_cap: v["marketCapitalization"].as_f64().unwrap_or(0.0),
        shares_outstanding: v["shareOutstanding"].as_f64().unwrap_or(0.0),
    })
}

/// Finnhub /stock/peers — related tickers (up to ~10).
pub async fn fetch_finnhub_peers(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<String>, String> {
    if token.is_empty() { return Err("Finnhub API key required".into()); }
    let resp = client
        .get("https://finnhub.io/api/v1/stock/peers")
        .query(&[("symbol", symbol), ("token", token)])
        .send().await
        .map_err(|e| format!("Finnhub peers failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub peers: HTTP {}", resp.status()));
    }
    let arr: Vec<String> = resp.json().await
        .map_err(|e| format!("Finnhub peers parse: {e}"))?;
    Ok(arr)
}

/// Finnhub /stock/earnings — actual vs estimate EPS per quarter (up to ~16 rows).
pub async fn fetch_finnhub_earnings(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<EarningRow>, String> {
    if token.is_empty() { return Err("Finnhub API key required".into()); }
    let resp = client
        .get("https://finnhub.io/api/v1/stock/earnings")
        .query(&[("symbol", symbol), ("token", token)])
        .send().await
        .map_err(|e| format!("Finnhub earnings failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub earnings: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("Finnhub earnings parse: {e}"))?;
    let rows = arr.into_iter().map(|e| {
        let actual = e["actual"].as_f64();
        let estimate = e["estimate"].as_f64();
        let surprise = e["surprise"].as_f64();
        let surprise_pct = e["surprisePercent"].as_f64();
        EarningRow {
            period: e["period"].as_str().unwrap_or("").to_string(),
            actual, estimate, surprise, surprise_pct,
            quarter: e["quarter"].as_i64().map(|v| v as i32),
            year: e["year"].as_i64().map(|v| v as i32),
        }
    }).collect();
    Ok(rows)
}

/// Finnhub /calendar/ipo — upcoming IPOs in a date range.
pub async fn fetch_finnhub_ipo_calendar(
    client: &reqwest::Client,
    token: &str,
    from: &str,
    to: &str,
) -> Result<Vec<IpoEvent>, String> {
    if token.is_empty() { return Err("Finnhub API key required".into()); }
    let resp = client
        .get("https://finnhub.io/api/v1/calendar/ipo")
        .query(&[("token", token), ("from", from), ("to", to)])
        .send().await
        .map_err(|e| format!("Finnhub IPO calendar failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub IPO: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("Finnhub IPO parse: {e}"))?;
    let mut rows = Vec::new();
    if let Some(arr) = v["ipoCalendar"].as_array() {
        for e in arr {
            rows.push(IpoEvent {
                date: e["date"].as_str().unwrap_or("").to_string(),
                symbol: e["symbol"].as_str().unwrap_or("").to_string(),
                name: e["name"].as_str().unwrap_or("").to_string(),
                exchange: e["exchange"].as_str().unwrap_or("").to_string(),
                price_range: e["price"].as_str().unwrap_or("").to_string(),
                shares: e["numberOfShares"].as_i64().unwrap_or(0),
                total_value: e["totalSharesValue"].as_f64().unwrap_or(0.0),
                status: e["status"].as_str().unwrap_or("").to_string(),
            });
        }
    }
    Ok(rows)
}

/// Finnhub /press-releases — company press releases (last 90 days).
pub async fn fetch_finnhub_press(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<PressRelease>, String> {
    if token.is_empty() { return Err("Finnhub API key required".into()); }
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let ninety_ago = (chrono::Utc::now() - chrono::Duration::days(90)).format("%Y-%m-%d").to_string();
    let resp = client
        .get("https://finnhub.io/api/v1/press-releases")
        .query(&[("symbol", symbol), ("token", token), ("from", ninety_ago.as_str()), ("to", today.as_str())])
        .send().await
        .map_err(|e| format!("Finnhub press failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub press: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("Finnhub press parse: {e}"))?;
    let mut rows = Vec::new();
    if let Some(arr) = v["majorDevelopment"].as_array() {
        for e in arr {
            rows.push(PressRelease {
                symbol: symbol.to_uppercase(),
                datetime: e["datetime"].as_str().unwrap_or("").to_string(),
                headline: e["headline"].as_str().unwrap_or("").to_string(),
                description: e["description"].as_str().unwrap_or("").to_string(),
                url: e["url"].as_str().unwrap_or("").to_string(),
            });
        }
    }
    Ok(rows)
}

/// Finnhub /stock/social-sentiment — Reddit + Twitter daily mention buckets (last 30 days).
pub async fn fetch_finnhub_social(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<SocialSentimentRow>, String> {
    if token.is_empty() { return Err("Finnhub API key required".into()); }
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let month_ago = (chrono::Utc::now() - chrono::Duration::days(30)).format("%Y-%m-%d").to_string();
    let resp = client
        .get("https://finnhub.io/api/v1/stock/social-sentiment")
        .query(&[("symbol", symbol), ("token", token), ("from", month_ago.as_str()), ("to", today.as_str())])
        .send().await
        .map_err(|e| format!("Finnhub social failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub social: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("Finnhub social parse: {e}"))?;
    let mut rows = Vec::new();
    for src in ["reddit", "twitter"].iter() {
        if let Some(arr) = v[src].as_array() {
            for e in arr {
                rows.push(SocialSentimentRow {
                    source: src.to_string(),
                    at_time: e["atTime"].as_str().unwrap_or("").to_string(),
                    mention: e["mention"].as_i64().unwrap_or(0),
                    positive_mention: e["positiveMention"].as_i64().unwrap_or(0),
                    negative_mention: e["negativeMention"].as_i64().unwrap_or(0),
                    positive_score: e["positiveScore"].as_f64().unwrap_or(0.0),
                    negative_score: e["negativeScore"].as_f64().unwrap_or(0.0),
                    score: e["score"].as_f64().unwrap_or(0.0),
                });
            }
        }
    }
    Ok(rows)
}

// ── FMP fetchers ───────────────────────────────────────────────────────────

/// FMP /earning_call_transcript/{symbol} list endpoint — returns available [year, quarter, date] triples.
pub async fn fetch_fmp_transcript_list(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<TranscriptMeta>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    // FMP returns e.g. [[4, 2023, "2024-02-01"], [3, 2023, "2023-11-02"], ...]
    let url = format!("https://financialmodelingprep.com/api/v4/earning_call_transcript?symbol={}&apikey={}", symbol, fmp_key);
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP transcript list failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP transcript list: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("FMP transcript list parse: {e}"))?;
    let mut rows = Vec::new();
    if let Some(arr) = v.as_array() {
        for entry in arr {
            if let Some(triple) = entry.as_array() {
                if triple.len() >= 3 {
                    let quarter = triple[0].as_i64().unwrap_or(0) as i32;
                    let year = triple[1].as_i64().unwrap_or(0) as i32;
                    let date = triple[2].as_str().unwrap_or("").to_string();
                    rows.push(TranscriptMeta {
                        symbol: symbol.to_uppercase(),
                        quarter, year, date,
                    });
                }
            }
        }
    }
    Ok(rows)
}

/// FMP /earning_call_transcript/{symbol}?quarter=N&year=Y — full transcript body.
pub async fn fetch_fmp_transcript(
    client: &reqwest::Client,
    symbol: &str,
    quarter: i32,
    year: i32,
    fmp_key: &str,
) -> Result<Transcript, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!("https://financialmodelingprep.com/api/v3/earning_call_transcript/{}?quarter={}&year={}&apikey={}",
        symbol, quarter, year, fmp_key);
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP transcript failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP transcript: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP transcript parse: {e}"))?;
    if arr.is_empty() {
        return Err(format!("No transcript for {} Q{} {}", symbol, quarter, year));
    }
    let e = &arr[0];
    Ok(Transcript {
        symbol: symbol.to_uppercase(),
        quarter: e["quarter"].as_i64().unwrap_or(quarter as i64) as i32,
        year: e["year"].as_i64().unwrap_or(year as i64) as i32,
        date: e["date"].as_str().unwrap_or("").to_string(),
        content: e["content"].as_str().unwrap_or("").to_string(),
    })
}

// ── Yahoo fetchers ─────────────────────────────────────────────────────────

/// Yahoo /v7/finance/quote — batch commodities quote.
/// Returns (symbol, display_name, price, change, change_pct).
pub async fn fetch_yahoo_quotes(
    client: &reqwest::Client,
    symbols: &[&str],
) -> Result<Vec<(String, f64, f64, f64)>, String> {
    if symbols.is_empty() { return Ok(vec![]); }
    let joined = symbols.join(",");
    let url = format!("https://query1.finance.yahoo.com/v7/finance/quote?symbols={}", joined);
    let resp = client.get(&url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1")
        .send().await
        .map_err(|e| format!("Yahoo quote failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Yahoo quote: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("Yahoo quote parse: {e}"))?;
    let mut out = Vec::new();
    if let Some(arr) = v.pointer("/quoteResponse/result").and_then(|r| r.as_array()) {
        for q in arr {
            let sym = q["symbol"].as_str().unwrap_or("").to_string();
            let price = q["regularMarketPrice"].as_f64().unwrap_or(0.0);
            let change = q["regularMarketChange"].as_f64().unwrap_or(0.0);
            let pct = q["regularMarketChangePercent"].as_f64().unwrap_or(0.0);
            if !sym.is_empty() {
                out.push((sym, price, change, pct));
            }
        }
    }
    Ok(out)
}

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
            market_cap REAL NOT NULL DEFAULT 0,
            shares_outstanding REAL NOT NULL DEFAULT 0,
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
        CREATE INDEX IF NOT EXISTS idx_research_profile_updated ON research_profile(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_peers_updated ON research_peers(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_earnings_updated ON research_earnings(updated_at);"
    ).map_err(|e| format!("create research tables: {e}"))?;
    Ok(())
}

fn now_ts() -> i64 { chrono::Utc::now().timestamp() }

// ── profile ────────────────────────────────────────────────────────────────

pub fn upsert_profile(conn: &Connection, p: &CompanyProfile) -> Result<(), String> {
    let _ = create_research_tables(conn);
    conn.execute(
        "INSERT INTO research_profile
         (symbol, name, exchange, country, currency, industry, sector, website, logo, phone, ipo_date, market_cap, shares_outstanding, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)
         ON CONFLICT(symbol) DO UPDATE SET
            name=excluded.name, exchange=excluded.exchange, country=excluded.country,
            currency=excluded.currency, industry=excluded.industry, sector=excluded.sector,
            website=excluded.website, logo=excluded.logo, phone=excluded.phone,
            ipo_date=excluded.ipo_date, market_cap=excluded.market_cap,
            shares_outstanding=excluded.shares_outstanding, updated_at=excluded.updated_at",
        params![
            p.symbol.to_uppercase(), p.name, p.exchange, p.country, p.currency,
            p.industry, p.sector, p.website, p.logo, p.phone, p.ipo_date,
            p.market_cap, p.shares_outstanding, now_ts(),
        ],
    ).map_err(|e| format!("upsert profile: {e}"))?;
    Ok(())
}

pub fn get_profile(conn: &Connection, symbol: &str) -> Result<Option<CompanyProfile>, String> {
    let _ = create_research_tables(conn);
    let sym = symbol.to_uppercase();
    let mut stmt = conn.prepare(
        "SELECT symbol, name, exchange, country, currency, industry, sector, website, logo, phone, ipo_date, market_cap, shares_outstanding
         FROM research_profile WHERE symbol = ?1"
    ).map_err(|e| format!("prepare get_profile: {e}"))?;
    let mut rows = stmt.query(params![sym]).map_err(|e| format!("query get_profile: {e}"))?;
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
            market_cap: row.get(11).unwrap_or(0.0),
            shares_outstanding: row.get(12).unwrap_or(0.0),
        }))
    } else {
        Ok(None)
    }
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
    let mut stmt = conn.prepare("SELECT peers_json FROM research_peers WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_peers: {e}"))?;
    let mut rows = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_peers: {e}"))?;
    if let Some(row) = rows.next().map_err(|e| format!("row get_peers: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        let peers: Vec<String> = serde_json::from_str(&json).unwrap_or_default();
        Ok(Some(peers))
    } else {
        Ok(None)
    }
}

// ── earnings history ───────────────────────────────────────────────────────

pub fn upsert_earnings_history(conn: &Connection, symbol: &str, rows: &[EarningRow]) -> Result<(), String> {
    let _ = create_research_tables(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("earnings json: {e}"))?;
    conn.execute(
        "INSERT INTO research_earnings(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert earnings: {e}"))?;
    Ok(())
}

pub fn get_earnings_history(conn: &Connection, symbol: &str) -> Result<Option<Vec<EarningRow>>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_earnings WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_earnings: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_earnings: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_earnings: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        let rows: Vec<EarningRow> = serde_json::from_str(&json).unwrap_or_default();
        Ok(Some(rows))
    } else {
        Ok(None)
    }
}

// ── press releases ─────────────────────────────────────────────────────────

pub fn upsert_press_releases(conn: &Connection, symbol: &str, rows: &[PressRelease]) -> Result<(), String> {
    let _ = create_research_tables(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("press json: {e}"))?;
    conn.execute(
        "INSERT INTO research_press(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert press: {e}"))?;
    Ok(())
}

pub fn get_press_releases(conn: &Connection, symbol: &str) -> Result<Option<Vec<PressRelease>>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_press WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_press: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_press: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_press: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        let rows: Vec<PressRelease> = serde_json::from_str(&json).unwrap_or_default();
        Ok(Some(rows))
    } else {
        Ok(None)
    }
}

// ── social sentiment ───────────────────────────────────────────────────────

pub fn upsert_sentiment(conn: &Connection, symbol: &str, rows: &[SocialSentimentRow]) -> Result<(), String> {
    let _ = create_research_tables(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("sentiment json: {e}"))?;
    conn.execute(
        "INSERT INTO research_sentiment(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert sentiment: {e}"))?;
    Ok(())
}

pub fn get_sentiment(conn: &Connection, symbol: &str) -> Result<Option<Vec<SocialSentimentRow>>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_sentiment WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_sentiment: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_sentiment: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_sentiment: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        let rows: Vec<SocialSentimentRow> = serde_json::from_str(&json).unwrap_or_default();
        Ok(Some(rows))
    } else {
        Ok(None)
    }
}

// ── transcripts ────────────────────────────────────────────────────────────

pub fn upsert_transcript_list(conn: &Connection, symbol: &str, rows: &[TranscriptMeta]) -> Result<(), String> {
    let _ = create_research_tables(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("transcript list json: {e}"))?;
    conn.execute(
        "INSERT INTO research_transcript_list(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert transcript list: {e}"))?;
    Ok(())
}

pub fn get_transcript_list(conn: &Connection, symbol: &str) -> Result<Option<Vec<TranscriptMeta>>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_transcript_list WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_tlist: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_tlist: {e}"))?;
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
        params![t.symbol.to_uppercase(), t.quarter, t.year, t.date, t.content, now_ts()],
    ).map_err(|e| format!("upsert transcript: {e}"))?;
    Ok(())
}

pub fn get_transcript(conn: &Connection, symbol: &str, quarter: i32, year: i32) -> Result<Option<Transcript>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn.prepare(
        "SELECT symbol, quarter, year, date, content FROM research_transcript
         WHERE symbol = ?1 AND year = ?2 AND quarter = ?3"
    ).map_err(|e| format!("prepare get_transcript: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase(), year, quarter])
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

// ── IPO calendar ───────────────────────────────────────────────────────────

pub fn upsert_ipo_calendar(conn: &Connection, rows: &[IpoEvent]) -> Result<(), String> {
    let _ = create_research_tables(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("ipo json: {e}"))?;
    conn.execute("DELETE FROM research_ipo_calendar", []).map_err(|e| format!("ipo delete: {e}"))?;
    conn.execute(
        "INSERT INTO research_ipo_calendar(snapshot_at, rows_json) VALUES (?1,?2)",
        params![now_ts(), json],
    ).map_err(|e| format!("upsert ipo: {e}"))?;
    Ok(())
}

pub fn get_ipo_calendar(conn: &Connection) -> Result<Option<Vec<IpoEvent>>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_ipo_calendar ORDER BY snapshot_at DESC LIMIT 1")
        .map_err(|e| format!("prepare get_ipo: {e}"))?;
    let mut r = stmt.query([]).map_err(|e| format!("query get_ipo: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ipo: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── bulk scrape helper (used by fundamentals scrape loop) ──────────────────

/// Fetch and cache all research data for a single symbol, respecting rate limits.
/// Returns Ok(()) even if individual endpoints fail — errors are logged via cb.
pub async fn scrape_and_cache_symbol(
    client: &reqwest::Client,
    conn: &Connection,
    symbol: &str,
    finnhub_key: &str,
    fmp_key: &str,
    mut cb: impl FnMut(&str),
) -> Result<(), String> {
    let sym = symbol.to_uppercase();
    if sym.is_empty() { return Err("empty symbol".into()); }

    // Profile
    if !finnhub_key.is_empty() {
        match fetch_finnhub_profile(client, &sym, finnhub_key).await {
            Ok(p) => {
                if !p.name.is_empty() {
                    let _ = upsert_profile(conn, &p);
                    cb(&format!("research/profile: {} cached", sym));
                }
            }
            Err(e) => cb(&format!("research/profile {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        // Peers
        match fetch_finnhub_peers(client, &sym, finnhub_key).await {
            Ok(peers) => {
                if !peers.is_empty() {
                    let _ = upsert_peers(conn, &sym, &peers);
                }
            }
            Err(e) => cb(&format!("research/peers {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        // Earnings
        match fetch_finnhub_earnings(client, &sym, finnhub_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_earnings_history(conn, &sym, &rows);
                }
            }
            Err(e) => cb(&format!("research/earnings {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        // Press releases
        match fetch_finnhub_press(client, &sym, finnhub_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_press_releases(conn, &sym, &rows);
                }
            }
            Err(e) => cb(&format!("research/press {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        // Social sentiment
        match fetch_finnhub_social(client, &sym, finnhub_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_sentiment(conn, &sym, &rows);
                }
            }
            Err(e) => cb(&format!("research/sentiment {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    }

    // Transcripts (FMP)
    if !fmp_key.is_empty() {
        match fetch_fmp_transcript_list(client, &sym, fmp_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_transcript_list(conn, &sym, &rows);
                }
            }
            Err(e) => cb(&format!("research/transcripts {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // ADR-109: dividend history (FMP)
        match fetch_fmp_dividend_history(client, &sym, fmp_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_dividends(conn, &sym, &rows);
                }
            }
            Err(e) => cb(&format!("research/dividends {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // ADR-109: forward earnings estimates (FMP)
        match fetch_fmp_earnings_estimates(client, &sym, fmp_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_earnings_estimates(conn, &sym, &rows);
                }
            }
            Err(e) => cb(&format!("research/estimates {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // ADR-109: analyst rating changes (FMP)
        match fetch_fmp_rating_changes(client, &sym, fmp_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_rating_changes(conn, &sym, &rows);
                }
            }
            Err(e) => cb(&format!("research/ratings {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // ADR-110: full FA bundle (6 FMP calls, internal 400ms sleeps).
        match fetch_fmp_financial_bundle(client, &sym, fmp_key).await {
            Ok(bundle) => {
                let any = !bundle.income_annual.is_empty()
                    || !bundle.income_quarterly.is_empty()
                    || !bundle.balance_annual.is_empty()
                    || !bundle.balance_quarterly.is_empty()
                    || !bundle.cashflow_annual.is_empty()
                    || !bundle.cashflow_quarterly.is_empty();
                if any {
                    let _ = upsert_financials(conn, &sym, &bundle);
                    cb(&format!("research/financials: {} cached", sym));
                }
            }
            Err(e) => cb(&format!("research/financials {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // ADR-111: stock split history (FMP).
        match fetch_fmp_stock_splits(client, &sym, fmp_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_stock_splits(conn, &sym, &rows);
                    cb(&format!("research/splits: {} cached ({} rows)", sym, rows.len()));
                }
            }
            Err(e) => cb(&format!("research/splits {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // ADR-111: ETF holdings (FMP). No-op for non-ETF tickers (empty result).
        match fetch_fmp_etf_holdings(client, &sym, fmp_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_etf_holdings(conn, &sym, &rows);
                    cb(&format!("research/etf: {} cached ({} holdings)", sym, rows.len()));
                }
            }
            Err(e) => cb(&format!("research/etf {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // ADR-111: ESG scores (FMP).
        match fetch_fmp_esg(client, &sym, fmp_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_esg(conn, &sym, &rows);
                    cb(&format!("research/esg: {} cached ({} years)", sym, rows.len()));
                }
            }
            Err(e) => cb(&format!("research/esg {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    }

    // ADR-110: Finnhub executives (separate from FMP block; needs Finnhub key).
    if !finnhub_key.is_empty() {
        match fetch_finnhub_executives(client, &sym, finnhub_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_executives(conn, &sym, &rows);
                    cb(&format!("research/executives: {} cached ({} rows)", sym, rows.len()));
                }
            }
            Err(e) => cb(&format!("research/executives {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        // ADR-111: analyst recommendation trends (Finnhub).
        match fetch_finnhub_recommendations(client, &sym, finnhub_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_analyst_recs(conn, &sym, &rows);
                    cb(&format!("research/recs: {} cached ({} rows)", sym, rows.len()));
                }
            }
            Err(e) => cb(&format!("research/recs {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        // ADR-111: consensus price target (Finnhub).
        match fetch_finnhub_price_target(client, &sym, finnhub_key).await {
            Ok(pt) => {
                if pt.num_analysts > 0 || pt.target_mean > 0.0 {
                    let _ = upsert_price_target(conn, &sym, &pt);
                    cb(&format!("research/target: {} cached (n={})", sym, pt.num_analysts));
                }
            }
            Err(e) => cb(&format!("research/target {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    }

    Ok(())
}

// ── ADR-109 fetchers ───────────────────────────────────────────────────────

/// FMP /historical-price-full/stock_dividend/{symbol} — full dividend payment history.
pub async fn fetch_fmp_dividend_history(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<DividendRecord>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/historical-price-full/stock_dividend/{}?apikey={}",
        symbol, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP dividends failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP dividends: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("FMP dividends parse: {e}"))?;
    let mut rows = Vec::new();
    if let Some(arr) = v["historical"].as_array() {
        for e in arr {
            rows.push(DividendRecord {
                ex_date: e["date"].as_str().unwrap_or("").to_string(),
                pay_date: e["paymentDate"].as_str().unwrap_or("").to_string(),
                record_date: e["recordDate"].as_str().unwrap_or("").to_string(),
                declaration_date: e["declarationDate"].as_str().unwrap_or("").to_string(),
                amount: e["dividend"].as_f64().unwrap_or(0.0),
                adjusted_amount: e["adjDividend"].as_f64().unwrap_or(0.0),
                label: e["label"].as_str().unwrap_or("").to_string(),
            });
        }
    }
    Ok(rows)
}

/// FMP /analyst-estimates/{symbol} — forward EPS and revenue consensus estimates.
pub async fn fetch_fmp_earnings_estimates(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<EarningsEstimate>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/analyst-estimates/{}?apikey={}",
        symbol, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP estimates failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP estimates: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP estimates parse: {e}"))?;
    let rows = arr.into_iter().map(|e| EarningsEstimate {
        date: e["date"].as_str().unwrap_or("").to_string(),
        eps_avg: e["estimatedEpsAvg"].as_f64().unwrap_or(0.0),
        eps_high: e["estimatedEpsHigh"].as_f64().unwrap_or(0.0),
        eps_low: e["estimatedEpsLow"].as_f64().unwrap_or(0.0),
        revenue_avg: e["estimatedRevenueAvg"].as_f64().unwrap_or(0.0),
        revenue_high: e["estimatedRevenueHigh"].as_f64().unwrap_or(0.0),
        revenue_low: e["estimatedRevenueLow"].as_f64().unwrap_or(0.0),
        num_analysts_eps: e["numberAnalystEstimatedEps"].as_i64().unwrap_or(0) as i32,
        num_analysts_rev: e["numberAnalystsEstimatedRevenue"].as_i64().unwrap_or(0) as i32,
    }).collect();
    Ok(rows)
}

/// FMP /upgrades-downgrades (v4) — analyst rating change feed for a symbol.
pub async fn fetch_fmp_rating_changes(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<RatingChange>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v4/upgrades-downgrades?symbol={}&apikey={}",
        symbol, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP rating changes failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP rating changes: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP rating changes parse: {e}"))?;
    let rows = arr.into_iter().map(|e| {
        let to = e["newGrade"].as_str().unwrap_or("").to_string();
        let from = e["previousGrade"].as_str().unwrap_or("").to_string();
        let action_raw = e["action"].as_str().unwrap_or("").to_lowercase();
        // FMP action strings like "hold","buy" — map to upgrade/downgrade where we can.
        let action = if action_raw.is_empty() {
            if from.is_empty() { "initiation" } else if to != from { "changed" } else { "maintain" }.to_string()
        } else { action_raw };
        RatingChange {
            date: e["publishedDate"].as_str().unwrap_or("").chars().take(10).collect(),
            symbol: e["symbol"].as_str().unwrap_or(symbol).to_uppercase(),
            company: e["gradingCompany"].as_str().unwrap_or("").to_string(),
            firm: e["gradingCompany"].as_str().unwrap_or("").to_string(),
            action,
            from_grade: from,
            to_grade: to,
            price_target: e["priceTarget"].as_f64().unwrap_or(0.0),
        }
    }).collect();
    Ok(rows)
}

/// Yahoo batch quote → Treasury yield curve snapshot (no auth).
pub async fn fetch_treasury_yields(
    client: &reqwest::Client,
) -> Result<Vec<TreasuryYield>, String> {
    let tickers: Vec<&str> = TREASURY_TENORS.iter().map(|(t, _)| *t).collect();
    let quotes = fetch_yahoo_quotes(client, &tickers).await?;
    let mut out = Vec::new();
    for (sym, price, change, pct) in quotes {
        if let Some((_, tenor)) = TREASURY_TENORS.iter().find(|(t, _)| *t == sym.as_str()) {
            out.push(TreasuryYield {
                tenor: (*tenor).to_string(),
                ticker: sym,
                yield_pct: price,
                change,
                change_pct: pct,
            });
        }
    }
    // Preserve ladder order (13W, 5Y, 10Y, 30Y).
    out.sort_by_key(|t| TREASURY_TENORS.iter().position(|(_, lbl)| *lbl == t.tenor.as_str()).unwrap_or(99));
    Ok(out)
}

// ── ADR-110 fetchers ───────────────────────────────────────────────────────

/// Parse a Socrata numeric field that arrives as either a JSON number or a string.
fn socrata_f64(v: &serde_json::Value) -> f64 {
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0.0)
}

/// FMP /income-statement/{symbol} — up to 20 historical periods. `period` = "annual" or "quarter".
pub async fn fetch_fmp_income_statement(
    client: &reqwest::Client,
    symbol: &str,
    period: &str,
    fmp_key: &str,
) -> Result<Vec<IncomeStatement>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/income-statement/{}?period={}&limit=20&apikey={}",
        symbol, period, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP income failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP income: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP income parse: {e}"))?;
    let rows = arr.into_iter().map(|e| IncomeStatement {
        date: e["date"].as_str().unwrap_or("").to_string(),
        period: e["period"].as_str().unwrap_or("").to_string(),
        revenue: e["revenue"].as_f64().unwrap_or(0.0),
        cost_of_revenue: e["costOfRevenue"].as_f64().unwrap_or(0.0),
        gross_profit: e["grossProfit"].as_f64().unwrap_or(0.0),
        research_and_development: e["researchAndDevelopmentExpenses"].as_f64().unwrap_or(0.0),
        selling_general_admin: e["sellingGeneralAndAdministrativeExpenses"].as_f64().unwrap_or(0.0),
        operating_expenses: e["operatingExpenses"].as_f64().unwrap_or(0.0),
        operating_income: e["operatingIncome"].as_f64().unwrap_or(0.0),
        interest_expense: e["interestExpense"].as_f64().unwrap_or(0.0),
        ebitda: e["ebitda"].as_f64().unwrap_or(0.0),
        income_before_tax: e["incomeBeforeTax"].as_f64().unwrap_or(0.0),
        income_tax_expense: e["incomeTaxExpense"].as_f64().unwrap_or(0.0),
        net_income: e["netIncome"].as_f64().unwrap_or(0.0),
        eps: e["eps"].as_f64().unwrap_or(0.0),
        eps_diluted: e["epsdiluted"].as_f64().unwrap_or(0.0),
        weighted_shares_out: e["weightedAverageShsOut"].as_f64().unwrap_or(0.0),
    }).collect();
    Ok(rows)
}

/// FMP /balance-sheet-statement/{symbol} — up to 20 historical periods.
pub async fn fetch_fmp_balance_sheet(
    client: &reqwest::Client,
    symbol: &str,
    period: &str,
    fmp_key: &str,
) -> Result<Vec<BalanceSheet>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/balance-sheet-statement/{}?period={}&limit=20&apikey={}",
        symbol, period, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP balance failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP balance: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP balance parse: {e}"))?;
    let rows = arr.into_iter().map(|e| BalanceSheet {
        date: e["date"].as_str().unwrap_or("").to_string(),
        period: e["period"].as_str().unwrap_or("").to_string(),
        cash_and_equiv: e["cashAndCashEquivalents"].as_f64().unwrap_or(0.0),
        short_term_investments: e["shortTermInvestments"].as_f64().unwrap_or(0.0),
        net_receivables: e["netReceivables"].as_f64().unwrap_or(0.0),
        inventory: e["inventory"].as_f64().unwrap_or(0.0),
        total_current_assets: e["totalCurrentAssets"].as_f64().unwrap_or(0.0),
        property_plant_equipment: e["propertyPlantEquipmentNet"].as_f64().unwrap_or(0.0),
        goodwill: e["goodwill"].as_f64().unwrap_or(0.0),
        intangible_assets: e["intangibleAssets"].as_f64().unwrap_or(0.0),
        long_term_investments: e["longTermInvestments"].as_f64().unwrap_or(0.0),
        total_non_current_assets: e["totalNonCurrentAssets"].as_f64().unwrap_or(0.0),
        total_assets: e["totalAssets"].as_f64().unwrap_or(0.0),
        accounts_payable: e["accountPayables"].as_f64().unwrap_or(0.0),
        short_term_debt: e["shortTermDebt"].as_f64().unwrap_or(0.0),
        total_current_liabilities: e["totalCurrentLiabilities"].as_f64().unwrap_or(0.0),
        long_term_debt: e["longTermDebt"].as_f64().unwrap_or(0.0),
        total_non_current_liabilities: e["totalNonCurrentLiabilities"].as_f64().unwrap_or(0.0),
        total_liabilities: e["totalLiabilities"].as_f64().unwrap_or(0.0),
        common_stock: e["commonStock"].as_f64().unwrap_or(0.0),
        retained_earnings: e["retainedEarnings"].as_f64().unwrap_or(0.0),
        total_equity: e["totalStockholdersEquity"].as_f64().unwrap_or(0.0),
        total_debt: e["totalDebt"].as_f64().unwrap_or(0.0),
        net_debt: e["netDebt"].as_f64().unwrap_or(0.0),
    }).collect();
    Ok(rows)
}

/// FMP /cash-flow-statement/{symbol} — up to 20 historical periods.
pub async fn fetch_fmp_cash_flow(
    client: &reqwest::Client,
    symbol: &str,
    period: &str,
    fmp_key: &str,
) -> Result<Vec<CashFlowStatement>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/cash-flow-statement/{}?period={}&limit=20&apikey={}",
        symbol, period, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP cash flow failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP cash flow: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP cash flow parse: {e}"))?;
    let rows = arr.into_iter().map(|e| CashFlowStatement {
        date: e["date"].as_str().unwrap_or("").to_string(),
        period: e["period"].as_str().unwrap_or("").to_string(),
        net_income: e["netIncome"].as_f64().unwrap_or(0.0),
        depreciation_amortization: e["depreciationAndAmortization"].as_f64().unwrap_or(0.0),
        stock_based_comp: e["stockBasedCompensation"].as_f64().unwrap_or(0.0),
        change_working_capital: e["changeInWorkingCapital"].as_f64().unwrap_or(0.0),
        cash_from_operations: e["operatingCashFlow"].as_f64().unwrap_or(0.0),
        capex: e["capitalExpenditure"].as_f64().unwrap_or(0.0),
        acquisitions: e["acquisitionsNet"].as_f64().unwrap_or(0.0),
        investments_purchases: e["purchasesOfInvestments"].as_f64().unwrap_or(0.0),
        cash_from_investing: e["netCashUsedForInvestingActivites"].as_f64().unwrap_or(0.0),
        debt_repayment: e["debtRepayment"].as_f64().unwrap_or(0.0),
        dividends_paid: e["dividendsPaid"].as_f64().unwrap_or(0.0),
        stock_repurchases: e["commonStockRepurchased"].as_f64().unwrap_or(0.0),
        cash_from_financing: e["netCashUsedProvidedByFinancingActivities"].as_f64().unwrap_or(0.0),
        net_change_cash: e["netChangeInCash"].as_f64().unwrap_or(0.0),
        free_cash_flow: e["freeCashFlow"].as_f64().unwrap_or(0.0),
    }).collect();
    Ok(rows)
}

/// Convenience: fetch the full FA bundle (all 3 statements × annual+quarterly) in one call.
/// 6 FMP calls, 400 ms between each = ~2.4 s per symbol.
pub async fn fetch_fmp_financial_bundle(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<FinancialStatements, String> {
    let mut bundle = FinancialStatements::default();
    bundle.income_annual = fetch_fmp_income_statement(client, symbol, "annual", fmp_key).await.unwrap_or_default();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    bundle.income_quarterly = fetch_fmp_income_statement(client, symbol, "quarter", fmp_key).await.unwrap_or_default();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    bundle.balance_annual = fetch_fmp_balance_sheet(client, symbol, "annual", fmp_key).await.unwrap_or_default();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    bundle.balance_quarterly = fetch_fmp_balance_sheet(client, symbol, "quarter", fmp_key).await.unwrap_or_default();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    bundle.cashflow_annual = fetch_fmp_cash_flow(client, symbol, "annual", fmp_key).await.unwrap_or_default();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    bundle.cashflow_quarterly = fetch_fmp_cash_flow(client, symbol, "quarter", fmp_key).await.unwrap_or_default();
    Ok(bundle)
}

/// Finnhub /stock/executive — company officers with compensation.
pub async fn fetch_finnhub_executives(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<Executive>, String> {
    if token.is_empty() { return Err("Finnhub API key required".into()); }
    let resp = client
        .get("https://finnhub.io/api/v1/stock/executive")
        .query(&[("symbol", symbol), ("token", token)])
        .send().await
        .map_err(|e| format!("Finnhub executives failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub executives: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("Finnhub executives parse: {e}"))?;
    let mut rows = Vec::new();
    if let Some(arr) = v["executive"].as_array() {
        for e in arr {
            rows.push(Executive {
                name: e["name"].as_str().unwrap_or("").to_string(),
                position: e["position"].as_str().unwrap_or("").to_string(),
                age: e["age"].as_i64().unwrap_or(0) as i32,
                sex: e["sex"].as_str().unwrap_or("").to_string(),
                since: e["since"].as_str().unwrap_or("").to_string(),
                compensation: e["compensation"].as_f64().unwrap_or(0.0),
                year: e["year"].as_i64().unwrap_or(0) as i32,
            });
        }
    }
    Ok(rows)
}

/// CFTC Socrata — Commitments of Traders, Legacy Futures combined.
/// Public JSON endpoint, no API key. Returns one row per market for the most recent report date.
/// WoW change in non-commercial net is computed from the prior week found in the same payload.
pub async fn fetch_cftc_cot(
    client: &reqwest::Client,
) -> Result<Vec<CotReport>, String> {
    // Legacy futures-only combined. Ordered by report date descending so the first rows
    // define the latest week, subsequent rows include the prior week for WoW delta.
    let url = "https://publicreporting.cftc.gov/resource/6dca-aqww.json?\
               $limit=2000&$order=report_date_as_yyyy_mm_dd DESC";
    let resp = client.get(url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1")
        .send().await
        .map_err(|e| format!("CFTC COT failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("CFTC COT: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("CFTC COT parse: {e}"))?;
    if arr.is_empty() { return Ok(vec![]); }

    // Latest report date is the max date seen in the payload (rows come sorted DESC but be safe).
    let latest_date = arr.iter()
        .filter_map(|e| e["report_date_as_yyyy_mm_dd"].as_str())
        .map(|s| s.chars().take(10).collect::<String>())
        .max()
        .unwrap_or_default();
    if latest_date.is_empty() { return Ok(vec![]); }

    // For each market, remember the first (latest) non-commercial net and the first *prior-week* net.
    use std::collections::HashMap;
    let mut prior: HashMap<String, f64> = HashMap::new();
    for e in arr.iter() {
        let market = e["market_and_exchange_names"].as_str().unwrap_or("").to_string();
        if market.is_empty() { continue; }
        let date: String = e["report_date_as_yyyy_mm_dd"].as_str().unwrap_or("").chars().take(10).collect();
        if date == latest_date { continue; }
        let nc_net = socrata_f64(&e["noncomm_positions_long_all"]) - socrata_f64(&e["noncomm_positions_short_all"]);
        prior.entry(market).or_insert(nc_net);
    }

    // Build the latest-week rows.
    let mut rows = Vec::new();
    for e in arr.iter() {
        let date: String = e["report_date_as_yyyy_mm_dd"].as_str().unwrap_or("").chars().take(10).collect();
        if date != latest_date { continue; }
        let market = e["market_and_exchange_names"].as_str().unwrap_or("").to_string();
        if market.is_empty() { continue; }
        let nc_long = socrata_f64(&e["noncomm_positions_long_all"]);
        let nc_short = socrata_f64(&e["noncomm_positions_short_all"]);
        let net = nc_long - nc_short;
        let prev = prior.get(&market).copied().unwrap_or(net);
        rows.push(CotReport {
            market_name: market,
            market_code: e["cftc_contract_market_code"].as_str().unwrap_or("").to_string(),
            report_date: date,
            open_interest: socrata_f64(&e["open_interest_all"]),
            noncomm_long: nc_long,
            noncomm_short: nc_short,
            // Socrata column name intentionally has the typo from the CFTC source feed.
            noncomm_spreads: socrata_f64(&e["noncomm_postions_spread_all"]),
            comm_long: socrata_f64(&e["comm_positions_long_all"]),
            comm_short: socrata_f64(&e["comm_positions_short_all"]),
            nonrept_long: socrata_f64(&e["nonrept_positions_long_all"]),
            nonrept_short: socrata_f64(&e["nonrept_positions_short_all"]),
            noncomm_net: net,
            noncomm_net_change: net - prev,
        });
    }
    rows.sort_by(|a, b| a.market_name.cmp(&b.market_name));
    Ok(rows)
}

// ── ADR-111 fetchers ───────────────────────────────────────────────────────

/// FMP /historical-price-full/stock_split/{symbol} — historical stock splits.
pub async fn fetch_fmp_stock_splits(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<StockSplit>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/historical-price-full/stock_split/{}?apikey={}",
        symbol, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP splits failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP splits: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("FMP splits parse: {e}"))?;
    let mut rows = Vec::new();
    if let Some(arr) = v["historical"].as_array() {
        for e in arr {
            let num = e["numerator"].as_f64().unwrap_or(0.0);
            let den = e["denominator"].as_f64().unwrap_or(0.0);
            let label = e["label"].as_str().map(|s| s.to_string())
                .unwrap_or_else(|| if num > 0.0 && den > 0.0 { format!("{}:{}", num, den) } else { String::new() });
            rows.push(StockSplit {
                date: e["date"].as_str().unwrap_or("").to_string(),
                label,
                numerator: num,
                denominator: den,
            });
        }
    }
    Ok(rows)
}

/// FMP /etf-holder/{symbol} — up to 1000 constituent holdings of an ETF.
pub async fn fetch_fmp_etf_holdings(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<EtfHolding>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/etf-holder/{}?apikey={}",
        symbol, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP etf-holder failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP etf-holder: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP etf-holder parse: {e}"))?;
    let rows = arr.into_iter().map(|e| EtfHolding {
        symbol: e["asset"].as_str().unwrap_or("").to_string(),
        name: e["name"].as_str().unwrap_or("").to_string(),
        weight_pct: e["weightPercentage"].as_f64().unwrap_or(0.0),
        shares: e["sharesNumber"].as_f64().unwrap_or(0.0),
        market_value: e["marketValue"].as_f64().unwrap_or(0.0),
        updated: e["updated"].as_str().unwrap_or("").to_string(),
    }).collect();
    Ok(rows)
}

/// Finnhub /stock/recommendation — last ~12 months of monthly buy/hold/sell bucket counts.
pub async fn fetch_finnhub_recommendations(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<AnalystRecommendation>, String> {
    if token.is_empty() { return Err("Finnhub API key required".into()); }
    let resp = client
        .get("https://finnhub.io/api/v1/stock/recommendation")
        .query(&[("symbol", symbol), ("token", token)])
        .send().await
        .map_err(|e| format!("Finnhub recommendations failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub recommendations: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("Finnhub recommendations parse: {e}"))?;
    let rows = arr.into_iter().map(|e| AnalystRecommendation {
        period: e["period"].as_str().unwrap_or("").to_string(),
        strong_buy: e["strongBuy"].as_i64().unwrap_or(0) as i32,
        buy: e["buy"].as_i64().unwrap_or(0) as i32,
        hold: e["hold"].as_i64().unwrap_or(0) as i32,
        sell: e["sell"].as_i64().unwrap_or(0) as i32,
        strong_sell: e["strongSell"].as_i64().unwrap_or(0) as i32,
    }).collect();
    Ok(rows)
}

/// Finnhub /stock/price-target — consensus high/low/mean target snapshot.
pub async fn fetch_finnhub_price_target(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<PriceTarget, String> {
    if token.is_empty() { return Err("Finnhub API key required".into()); }
    let resp = client
        .get("https://finnhub.io/api/v1/stock/price-target")
        .query(&[("symbol", symbol), ("token", token)])
        .send().await
        .map_err(|e| format!("Finnhub price-target failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub price-target: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("Finnhub price-target parse: {e}"))?;
    Ok(PriceTarget {
        symbol: symbol.to_uppercase(),
        target_high: v["targetHigh"].as_f64().unwrap_or(0.0),
        target_low: v["targetLow"].as_f64().unwrap_or(0.0),
        target_mean: v["targetMean"].as_f64().unwrap_or(0.0),
        target_median: v["targetMedian"].as_f64().unwrap_or(0.0),
        last_updated: v["lastUpdated"].as_str().unwrap_or("").chars().take(10).collect(),
        num_analysts: v["numberOfAnalysts"].as_i64().unwrap_or(0) as i32,
    })
}

/// FMP /esg-environmental-social-governance-data?symbol={sym} — historical ESG score rows.
pub async fn fetch_fmp_esg(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<EsgScore>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v4/esg-environmental-social-governance-data?symbol={}&apikey={}",
        symbol, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP esg failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP esg: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP esg parse: {e}"))?;
    let rows = arr.into_iter().map(|e| EsgScore {
        symbol: e["symbol"].as_str().unwrap_or(symbol).to_uppercase(),
        environmental_score: e["environmentalScore"].as_f64().unwrap_or(0.0),
        social_score: e["socialScore"].as_f64().unwrap_or(0.0),
        governance_score: e["governanceScore"].as_f64().unwrap_or(0.0),
        esg_score: e["ESGScore"].as_f64().unwrap_or(0.0),
        year: e["year"].as_i64().unwrap_or(0) as i32,
    }).collect();
    Ok(rows)
}

/// FMP index constituent endpoint (/sp500_constituent, /nasdaq_constituent, /dowjones_constituent).
/// `index_code` accepts "SP500" | "NDX" | "DJIA"; mapped to the right FMP path.
pub async fn fetch_fmp_index_members(
    client: &reqwest::Client,
    index_code: &str,
    fmp_key: &str,
) -> Result<Vec<IndexMember>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let (path, idx_label) = match index_code.to_uppercase().as_str() {
        "SP500" | "SPX" | "S&P500" => ("sp500_constituent", "SP500"),
        "NDX" | "NASDAQ" | "NDX100" => ("nasdaq_constituent", "NDX"),
        "DJIA" | "DOW" | "INDU" => ("dowjones_constituent", "DJIA"),
        other => return Err(format!("Unknown index code: {}", other)),
    };
    let url = format!(
        "https://financialmodelingprep.com/api/v3/{}?apikey={}",
        path, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP index members failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP index members: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP index members parse: {e}"))?;
    let rows = arr.into_iter().map(|e| IndexMember {
        index: idx_label.to_string(),
        symbol: e["symbol"].as_str().unwrap_or("").to_uppercase(),
        name: e["name"].as_str().unwrap_or("").to_string(),
        sector: e["sector"].as_str().unwrap_or("").to_string(),
        sub_sector: e["subSector"].as_str().unwrap_or("").to_string(),
        headquarters: e["headQuarter"].as_str().unwrap_or("").to_string(),
        date_added: e["dateFirstAdded"].as_str().unwrap_or("").to_string(),
    }).collect();
    Ok(rows)
}

// ── ADR-112 Round 5 fetchers ───────────────────────────────────────────────

/// FMP /v4/insider-trading — SEC Form 4 insider trade rows (default page=0, up to 100 rows).
pub async fn fetch_fmp_insider_trades(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<InsiderTrade>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v4/insider-trading?symbol={}&page=0&apikey={}",
        symbol, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP insider failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP insider: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP insider parse: {e}"))?;
    let rows = arr.into_iter().map(|e| {
        let shares = e["securitiesTransacted"].as_f64().unwrap_or(0.0);
        let price = e["price"].as_f64().unwrap_or(0.0);
        InsiderTrade {
            filing_date: e["filingDate"].as_str().unwrap_or("").chars().take(10).collect(),
            transaction_date: e["transactionDate"].as_str().unwrap_or("").chars().take(10).collect(),
            reporting_name: e["reportingName"].as_str().unwrap_or("").to_string(),
            transaction_type: e["transactionType"].as_str().unwrap_or("").to_string(),
            acquisition_disposition: e["acquistionOrDisposition"].as_str().unwrap_or("").to_string(),
            shares,
            price,
            value_usd: shares * price,
            shares_owned_after: e["securitiesOwned"].as_f64().unwrap_or(0.0),
            link: e["link"].as_str().unwrap_or("").to_string(),
        }
    }).collect();
    Ok(rows)
}

/// FMP /v3/institutional-holder/{symbol} — 13F-derived top holders of a stock.
pub async fn fetch_fmp_institutional_holders(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<InstitutionalHolder>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/institutional-holder/{}?apikey={}",
        symbol, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP holders failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP holders: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP holders parse: {e}"))?;
    let rows = arr.into_iter().map(|e| InstitutionalHolder {
        holder: e["holder"].as_str().unwrap_or("").to_string(),
        shares: e["shares"].as_f64().unwrap_or(0.0),
        date_reported: e["dateReported"].as_str().unwrap_or("").chars().take(10).collect(),
        change: e["change"].as_f64().unwrap_or(0.0),
    }).collect();
    Ok(rows)
}

/// FMP /v4/shares_float?symbol=… — latest free-float / outstanding-shares snapshot.
pub async fn fetch_fmp_shares_float(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<SharesFloat, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v4/shares_float?symbol={}&apikey={}",
        symbol, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP shares_float failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP shares_float: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("FMP shares_float parse: {e}"))?;
    // Response is a 1-element array or a bare object — handle both.
    let e = if let Some(first) = v.as_array().and_then(|a| a.first()) { first.clone() } else { v };
    Ok(SharesFloat {
        symbol: e["symbol"].as_str().unwrap_or(symbol).to_uppercase(),
        date: e["date"].as_str().unwrap_or("").chars().take(10).collect(),
        free_float_pct: e["freeFloat"].as_f64().unwrap_or(0.0),
        float_shares: e["floatShares"].as_f64().unwrap_or(0.0),
        outstanding_shares: e["outstandingShares"].as_f64().unwrap_or(0.0),
        source: e["source"].as_str().unwrap_or("").to_string(),
    })
}

/// FMP /v3/historical-price-full/{symbol} — up to ~5 years of daily OHLCV.
/// `limit` is applied client-side after parsing (FMP returns all history by default).
pub async fn fetch_fmp_historical_price(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
    limit: usize,
) -> Result<Vec<HistoricalPriceRow>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/historical-price-full/{}?apikey={}",
        symbol, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP historical failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP historical: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("FMP historical parse: {e}"))?;
    let mut rows = Vec::new();
    if let Some(arr) = v["historical"].as_array() {
        for e in arr.iter().take(limit.max(1)) {
            rows.push(HistoricalPriceRow {
                date: e["date"].as_str().unwrap_or("").to_string(),
                open: e["open"].as_f64().unwrap_or(0.0),
                high: e["high"].as_f64().unwrap_or(0.0),
                low: e["low"].as_f64().unwrap_or(0.0),
                close: e["close"].as_f64().unwrap_or(0.0),
                adj_close: e["adjClose"].as_f64().unwrap_or(0.0),
                volume: e["volume"].as_f64().unwrap_or(0.0),
                change: e["change"].as_f64().unwrap_or(0.0),
                change_pct: e["changePercent"].as_f64().unwrap_or(0.0),
            });
        }
    }
    Ok(rows)
}

/// FMP /v3/earning_surprise/{symbol} — quarterly actual-vs-estimate EPS history.
pub async fn fetch_fmp_earnings_surprises(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<EarningsSurprise>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/earning_surprise/{}?apikey={}",
        symbol, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP surprise failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP surprise: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP surprise parse: {e}"))?;
    let rows = arr.into_iter().map(|e| {
        let actual = e["actualEarningResult"].as_f64().unwrap_or(0.0);
        let est = e["estimatedEarning"].as_f64().unwrap_or(0.0);
        let surprise = actual - est;
        let surprise_pct = if est.abs() > 1e-9 { (surprise / est.abs()) * 100.0 } else { 0.0 };
        EarningsSurprise {
            date: e["date"].as_str().unwrap_or("").to_string(),
            symbol: e["symbol"].as_str().unwrap_or(symbol).to_uppercase(),
            eps_actual: actual,
            eps_estimate: est,
            surprise,
            surprise_pct,
        }
    }).collect();
    Ok(rows)
}

// ── ADR-113 Round 6 fetchers ───────────────────────────────────────────────

/// Yahoo batch-quote the WORLD_INDICES_UNIVERSE tickers for the WEI dashboard.
/// Returns rows in the universe's declared order so the UI grouping stays stable.
pub async fn fetch_world_indices(
    client: &reqwest::Client,
) -> Result<Vec<WorldIndex>, String> {
    let tickers: Vec<&str> = WORLD_INDICES_UNIVERSE.iter().map(|(t, _, _)| *t).collect();
    let quotes = fetch_yahoo_quotes(client, &tickers).await?;
    let mut by_sym: std::collections::HashMap<String, (f64, f64, f64)> =
        std::collections::HashMap::new();
    for (sym, price, change, pct) in quotes {
        by_sym.insert(sym, (price, change, pct));
    }
    let rows: Vec<WorldIndex> = WORLD_INDICES_UNIVERSE.iter().map(|(t, d, r)| {
        let (price, change, pct) = by_sym.get(*t).cloned().unwrap_or((0.0, 0.0, 0.0));
        WorldIndex {
            ticker: (*t).to_string(),
            display: (*d).to_string(),
            region: (*r).to_string(),
            price,
            change,
            change_pct: pct,
        }
    }).collect();
    Ok(rows)
}

/// Helper — parse a single FMP mover row into MarketMover.
fn parse_fmp_mover(e: &serde_json::Value) -> MarketMover {
    let price = e["price"].as_f64().unwrap_or(0.0);
    let change = e["change"].as_f64()
        .or_else(|| e["changes"].as_f64())
        .unwrap_or(0.0);
    // FMP often returns "changesPercentage" as a string like "-5.60%"
    let change_pct = e["changesPercentage"].as_f64()
        .or_else(|| e["changesPercentage"].as_str().map(|s| {
            s.trim_matches(|c: char| c == '%' || c.is_whitespace()).parse::<f64>().unwrap_or(0.0)
        }))
        .unwrap_or(0.0);
    MarketMover {
        symbol: e["symbol"].as_str().unwrap_or("").to_string(),
        name: e["name"].as_str().unwrap_or("").to_string(),
        price,
        change,
        change_pct,
        volume: e["volume"].as_f64().unwrap_or(0.0),
    }
}

/// FMP /v3/stock_market/{gainers|losers|actives} — bundled into one MarketMovers.
pub async fn fetch_fmp_market_movers(
    client: &reqwest::Client,
    fmp_key: &str,
) -> Result<MarketMovers, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let mut out = MarketMovers::default();
    for (bucket, field) in [("gainers", 0), ("losers", 1), ("actives", 2)] {
        let url = format!(
            "https://financialmodelingprep.com/api/v3/stock_market/{}?apikey={}",
            bucket, fmp_key
        );
        let resp = client.get(&url).send().await
            .map_err(|e| format!("FMP {} failed: {}", bucket, e))?;
        if !resp.status().is_success() {
            return Err(format!("FMP {}: HTTP {}", bucket, resp.status()));
        }
        let arr: Vec<serde_json::Value> = resp.json().await
            .map_err(|e| format!("FMP {} parse: {}", bucket, e))?;
        let rows: Vec<MarketMover> = arr.iter().map(parse_fmp_mover).collect();
        match field {
            0 => out.gainers = rows,
            1 => out.losers = rows,
            _ => out.actives = rows,
        }
    }
    Ok(out)
}

/// FMP /v3/sector-performance — intraday performance for all GICS sectors.
pub async fn fetch_fmp_sector_performance(
    client: &reqwest::Client,
    fmp_key: &str,
) -> Result<Vec<SectorPerformance>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/sector-performance?apikey={}",
        fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP sector-performance failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP sector-performance: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP sector-performance parse: {e}"))?;
    let rows: Vec<SectorPerformance> = arr.into_iter().map(|e| {
        let sector = e["sector"].as_str().unwrap_or("").to_string();
        // FMP returns "changesPercentage" as a "1.23%" string.
        let pct_raw = e["changesPercentage"].as_str().unwrap_or("0");
        let change_pct = pct_raw
            .trim_matches(|c: char| c == '%' || c.is_whitespace())
            .parse::<f64>()
            .unwrap_or(0.0);
        SectorPerformance { sector, change_pct }
    }).collect();
    Ok(rows)
}

/// Build a WACC snapshot by combining FMP profile (beta + market cap) with the
/// latest cached FA income/balance data (interest expense, total debt, tax rate)
/// and a caller-supplied risk-free rate (typically 10Y Treasury yield %).
///
/// This is a pure derivation: it does NOT hit the network.  Callers should
/// fetch the inputs first (profile, financials, yield curve) then pass them in.
pub fn compute_wacc_snapshot(
    symbol: &str,
    as_of: &str,
    beta: f64,
    market_cap: f64,
    risk_free_pct: f64,
    total_debt: f64,
    interest_expense: f64,
    effective_tax_rate_pct: f64,
) -> WaccSnapshot {
    let erp = DEFAULT_EQUITY_RISK_PREMIUM_PCT;
    let cost_of_equity_pct = risk_free_pct + beta * erp;

    let pre_tax_cost_of_debt_pct = if total_debt.abs() > 1e-6 {
        (interest_expense.abs() / total_debt) * 100.0
    } else { 0.0 };

    let tax_rate_pct = effective_tax_rate_pct.clamp(0.0, 60.0);
    let after_tax_cost_of_debt_pct = pre_tax_cost_of_debt_pct * (1.0 - tax_rate_pct / 100.0);

    let total_cap = market_cap + total_debt;
    let equity_weight = if total_cap > 1e-6 { market_cap / total_cap } else { 1.0 };
    let debt_weight = 1.0 - equity_weight;
    let wacc_pct = equity_weight * cost_of_equity_pct + debt_weight * after_tax_cost_of_debt_pct;

    WaccSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        beta,
        risk_free_pct,
        equity_risk_premium_pct: erp,
        cost_of_equity_pct,
        pre_tax_cost_of_debt_pct,
        tax_rate_pct,
        after_tax_cost_of_debt_pct,
        market_cap,
        total_debt,
        equity_weight,
        debt_weight,
        wacc_pct,
    }
}

// ── ADR-114 Round 7 — WCR fetcher ─────────────────────────────────────────

/// Fetch the hardcoded FX-majors universe through Yahoo and return the rows
/// in the order declared by `FX_MAJORS_UNIVERSE`.
pub async fn fetch_currency_rates(
    client: &reqwest::Client,
) -> Result<Vec<CurrencyRate>, String> {
    let tickers: Vec<&str> = FX_MAJORS_UNIVERSE.iter().map(|(t, _, _, _, _)| *t).collect();
    let quotes = fetch_yahoo_quotes(client, &tickers).await?;

    use std::collections::HashMap;
    let by_ticker: HashMap<String, (f64, f64, f64)> = quotes.into_iter()
        .map(|(t, p, c, pct)| (t, (p, c, pct)))
        .collect();

    let mut out = Vec::with_capacity(FX_MAJORS_UNIVERSE.len());
    for (tk, display, base, quote, region) in FX_MAJORS_UNIVERSE.iter() {
        let (price, change, change_pct) = by_ticker.get(*tk)
            .copied()
            .unwrap_or((0.0, 0.0, 0.0));
        out.push(CurrencyRate {
            ticker: (*tk).to_string(),
            display: (*display).to_string(),
            base: (*base).to_string(),
            quote: (*quote).to_string(),
            region: (*region).to_string(),
            price,
            change,
            change_pct,
        });
    }
    Ok(out)
}

// ── ADR-114 Round 7 — BETA compute ────────────────────────────────────────

/// Compute an OLS regression of symbol log-returns on market log-returns.
/// Returns (beta, alpha_per_period, r_squared, correlation, n).
/// Pure function, no I/O. Daily returns expected; alpha is per-period
/// (caller annualizes as needed).
fn ols_regression(symbol_returns: &[f64], market_returns: &[f64]) -> (f64, f64, f64, f64, usize) {
    let n = symbol_returns.len().min(market_returns.len());
    if n < 10 { return (0.0, 0.0, 0.0, 0.0, n); }
    let mean_s: f64 = symbol_returns.iter().take(n).sum::<f64>() / n as f64;
    let mean_m: f64 = market_returns.iter().take(n).sum::<f64>() / n as f64;

    let mut cov = 0.0_f64;
    let mut var_m = 0.0_f64;
    let mut var_s = 0.0_f64;
    for i in 0..n {
        let ds = symbol_returns[i] - mean_s;
        let dm = market_returns[i] - mean_m;
        cov += ds * dm;
        var_m += dm * dm;
        var_s += ds * ds;
    }
    if var_m <= 1e-12 { return (0.0, 0.0, 0.0, 0.0, n); }
    let beta = cov / var_m;
    let alpha = mean_s - beta * mean_m;

    // R² (symbol variance explained by market) = β² · var_m / var_s
    let r_squared = if var_s > 1e-12 { (beta * beta) * var_m / var_s } else { 0.0 };
    let correlation = if var_m > 1e-12 && var_s > 1e-12 {
        cov / (var_m.sqrt() * var_s.sqrt())
    } else { 0.0 };

    (beta, alpha, r_squared.clamp(0.0, 1.0), correlation, n)
}

/// Compute log-returns from a sequence of closes (newest-first or oldest-first
/// both work — the function only cares about adjacent differences). Result is
/// in the same order as the input (length = len - 1).
fn log_returns(closes: &[f64]) -> Vec<f64> {
    if closes.len() < 2 { return Vec::new(); }
    closes.windows(2)
        .map(|w| if w[0] > 0.0 && w[1] > 0.0 { (w[1] / w[0]).ln() } else { 0.0 })
        .collect()
}

/// Compute a per-symbol beta snapshot against a market benchmark using
/// cached FMP historical price rows for both series. Caller fetches the bars
/// once (or reuses the HP cache) and hands them in. The bars must be sorted
/// **newest-first** (FMP returns them that way by default).
///
/// We compute 1Y / 3Y / 5Y windows using the trailing N trading days.
/// Windows that don't have enough overlapping data are skipped silently.
pub fn compute_beta_snapshot(
    symbol: &str,
    market_ticker: &str,
    as_of: &str,
    sym_bars_newest_first: &[HistoricalPriceRow],
    mkt_bars_newest_first: &[HistoricalPriceRow],
) -> BetaSnapshot {
    use std::collections::HashMap;
    // Intersect by date to make returns directly comparable.
    let mkt_by_date: HashMap<&str, f64> = mkt_bars_newest_first.iter()
        .map(|b| (b.date.as_str(), b.close))
        .collect();
    let mut paired: Vec<(String, f64, f64)> = sym_bars_newest_first.iter()
        .filter_map(|b| mkt_by_date.get(b.date.as_str())
            .map(|m| (b.date.clone(), b.close, *m)))
        .collect();
    // Sort ascending by date so the log_returns helper produces chronological returns.
    paired.sort_by(|a, b| a.0.cmp(&b.0));

    let sym_closes: Vec<f64> = paired.iter().map(|(_, s, _)| *s).collect();
    let mkt_closes: Vec<f64> = paired.iter().map(|(_, _, m)| *m).collect();
    let sym_rets = log_returns(&sym_closes);
    let mkt_rets = log_returns(&mkt_closes);

    let mut windows = Vec::new();
    let mut note = String::new();

    for (label, days) in [("1Y", 252usize), ("3Y", 756), ("5Y", 1260)] {
        let n_available = sym_rets.len().min(mkt_rets.len());
        if n_available == 0 {
            continue;
        }
        // Use the most recent `days` returns (tail slice) — sym_rets/mkt_rets
        // are ordered chronologically (oldest first, newest last).
        let take = days.min(n_available);
        let s_slice = &sym_rets[n_available - take..];
        let m_slice = &mkt_rets[n_available - take..];
        let (beta, alpha, r2, corr, n_obs) = ols_regression(s_slice, m_slice);
        if n_obs < 20 {
            if note.is_empty() && label == "1Y" {
                note = format!("insufficient overlapping data (n={n_obs}) for stable beta");
            }
            continue;
        }
        windows.push(BetaWindow {
            window_label: label.to_string(),
            window_days: days,
            beta,
            alpha_pct: alpha * 252.0 * 100.0, // annualize daily alpha
            r_squared: r2,
            n_observations: n_obs,
            correlation: corr,
        });
    }

    BetaSnapshot {
        symbol: symbol.to_uppercase(),
        market_ticker: market_ticker.to_string(),
        as_of: as_of.to_string(),
        windows,
        note,
    }
}

// ── ADR-114 Round 7 — DDM compute ─────────────────────────────────────────

/// Compute a Gordon Growth dividend-discount-model snapshot from cached
/// dividend history and a required return (typically WACC or cost of equity).
///
/// Dividends are newest-first (matching `get_dividends`). Growth rate is
/// inferred from the 5-year dividend CAGR when at least 5 annual dividends
/// are available, with fallback to a clamped 3% assumption. If r ≤ g, the
/// Gordon formula degenerates — we return implied_price = 0.0 with a note.
pub fn compute_ddm_snapshot(
    symbol: &str,
    as_of: &str,
    dividends_newest_first: &[DividendRecord],
    required_return_pct: f64,
    return_source: &str,
) -> DdmSnapshot {
    if dividends_newest_first.is_empty() {
        return DdmSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            method: "Gordon Growth".to_string(),
            note: "no dividend history on file".to_string(),
            ..Default::default()
        };
    }

    // Trailing 4-quarter dividend ($ per share). We use adjusted_amount
    // so split adjustments don't distort the growth rate.
    let div_amount = |d: &DividendRecord| -> f64 {
        if d.adjusted_amount > 0.0 { d.adjusted_amount } else { d.amount }
    };
    let annual_dividend: f64 = dividends_newest_first.iter()
        .take(4)
        .map(div_amount)
        .sum();

    // Infer growth: bucket dividends by ex-date year, then CAGR over 5 years
    // if possible. Each bucket sums the quarterly payments for that year.
    use std::collections::BTreeMap;
    let mut by_year: BTreeMap<i32, f64> = BTreeMap::new();
    for d in dividends_newest_first.iter() {
        // ex_date like "2025-10-31" — parse the 4-digit prefix.
        if let Some(year_str) = d.ex_date.get(..4) {
            if let Ok(y) = year_str.parse::<i32>() {
                *by_year.entry(y).or_insert(0.0) += div_amount(d);
            }
        }
    }
    let years_sorted: Vec<(i32, f64)> = by_year.into_iter().collect();
    let (implied_growth_pct, growth_source) = if years_sorted.len() >= 6 {
        // Use 5-year CAGR: years_sorted.last() vs years_sorted[len-6]
        let end = years_sorted[years_sorted.len() - 2].1; // second-to-last (last might be partial)
        let start_idx = years_sorted.len().saturating_sub(7);
        let start = years_sorted[start_idx].1;
        if start > 1e-9 && end > 1e-9 {
            let n_years = (years_sorted.len() - 2 - start_idx) as f64;
            let cagr = (end / start).powf(1.0 / n_years.max(1.0)) - 1.0;
            (cagr.clamp(-0.20, 0.20) * 100.0, format!("{:.0}Y dividend CAGR", n_years))
        } else {
            (3.0, "fallback (insufficient history)".to_string())
        }
    } else if years_sorted.len() >= 3 {
        // Short history: compare oldest full year to newest full year.
        let end = years_sorted[years_sorted.len() - 2].1;
        let start = years_sorted[0].1;
        if start > 1e-9 && end > 1e-9 {
            let n_years = (years_sorted.len() - 2) as f64;
            let cagr = (end / start).powf(1.0 / n_years.max(1.0)) - 1.0;
            (cagr.clamp(-0.20, 0.20) * 100.0, format!("{:.0}Y dividend CAGR", n_years))
        } else {
            (3.0, "fallback (short history)".to_string())
        }
    } else {
        (3.0, "fallback (no growth history)".to_string())
    };

    // Gordon Growth: P = D1 / (r - g), where D1 = D0 * (1 + g).
    let g = implied_growth_pct / 100.0;
    let r = required_return_pct / 100.0;
    let (implied_price, note) = if r > g + 0.005 && annual_dividend > 0.0 {
        let d1 = annual_dividend * (1.0 + g);
        (d1 / (r - g), String::new())
    } else if annual_dividend <= 0.0 {
        (0.0, "annual dividend is zero — Gordon Growth not applicable".to_string())
    } else {
        (0.0, format!(
            "required return {:.2}% ≤ growth {:.2}% — Gordon formula diverges",
            required_return_pct, implied_growth_pct))
    };

    DdmSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        annual_dividend,
        implied_growth_pct,
        required_return_pct,
        growth_source,
        return_source: return_source.to_string(),
        implied_price,
        method: "Gordon Growth".to_string(),
        note,
    }
}

// ── ADR-114 Round 7 — RV compute (relative valuation peer matrix) ─────────

/// One input row for the relative-valuation calculator: a metric name plus
/// the subject's value and a list of peer values. Caller builds this from
/// cached fundamentals; the function is pure.
pub struct RvMetricInput<'a> {
    pub metric: &'a str,
    pub value: Option<f64>,
    pub peer_values: Vec<f64>,
}

/// Compute a `RelativeValuation` snapshot from a list of metric inputs.
/// Skips metrics where the subject has no value or the peer set has fewer
/// than 3 observations (same threshold the packet's sector-peer block uses).
pub fn compute_relative_valuation(
    symbol: &str,
    sector: &str,
    as_of: &str,
    metrics: &[RvMetricInput<'_>],
) -> RelativeValuation {
    let mut rows = Vec::new();
    let mut max_peer_count = 0;

    for m in metrics {
        let val = match m.value { Some(v) if v.is_finite() => v, _ => continue };
        let mut peers: Vec<f64> = m.peer_values.iter().copied()
            .filter(|x| x.is_finite())
            .collect();
        if peers.len() < 3 { continue; }
        peers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = peers.len();
        max_peer_count = max_peer_count.max(n);

        let median = peers[n / 2];
        let low = peers[0];
        let high = peers[n - 1];
        let mean = peers.iter().sum::<f64>() / n as f64;
        let variance = peers.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        let stdev = variance.sqrt();
        let z_score = if stdev > 1e-9 { (val - mean) / stdev } else { 0.0 };
        let below = peers.iter().filter(|p| **p < val).count();
        let percentile = (below as f64 / n as f64) * 100.0;

        rows.push(RvMetricRow {
            metric: m.metric.to_string(),
            value: val,
            peer_median: median,
            peer_low: low,
            peer_high: high,
            z_score,
            percentile,
        });
    }

    RelativeValuation {
        symbol: symbol.to_uppercase(),
        sector: sector.to_string(),
        as_of: as_of.to_string(),
        peer_count: max_peer_count,
        rows,
    }
}

// ── ADR-114 Round 7 — FIGI (OpenFIGI) fetcher ─────────────────────────────

/// Fetch OpenFIGI identifiers for a symbol. OpenFIGI is a free service run by
/// Bloomberg — no API key required for reasonable volumes. We POST the
/// ticker as an exchange-code lookup against US common-stock space.
pub async fn fetch_openfigi_identifiers(
    client: &reqwest::Client,
    symbol: &str,
) -> Result<Vec<FigiIdentifier>, String> {
    let body = serde_json::json!([{
        "idType": "TICKER",
        "idValue": symbol.to_uppercase(),
        "marketSecDes": "Equity"
    }]);
    let resp = client.post("https://api.openfigi.com/v3/mapping")
        .header("Content-Type", "application/json")
        .json(&body)
        .send().await
        .map_err(|e| format!("OpenFIGI request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("OpenFIGI: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("OpenFIGI parse: {e}"))?;
    let outer = v.as_array().ok_or_else(|| "OpenFIGI: expected array".to_string())?;
    let mut out = Vec::new();
    for entry in outer {
        if let Some(data) = entry.get("data").and_then(|d| d.as_array()) {
            for row in data {
                out.push(FigiIdentifier {
                    figi: row["figi"].as_str().unwrap_or("").to_string(),
                    name: row["name"].as_str().unwrap_or("").to_string(),
                    ticker: row["ticker"].as_str().unwrap_or("").to_string(),
                    exch_code: row["exchCode"].as_str().unwrap_or("").to_string(),
                    composite_figi: row["compositeFIGI"].as_str().unwrap_or("").to_string(),
                    share_class_figi: row["shareClassFIGI"].as_str().unwrap_or("").to_string(),
                    security_type: row["securityType"].as_str().unwrap_or("").to_string(),
                    security_type_2: row["securityType2"].as_str().unwrap_or("").to_string(),
                    market_sector: row["marketSector"].as_str().unwrap_or("").to_string(),
                    security_description: row["securityDescription"].as_str().unwrap_or("").to_string(),
                });
            }
        }
    }
    Ok(out)
}

// ── Tests ──────────────────────────────────────────────────────────────────


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commodities_universe_has_expected_sectors() {
        let sectors: std::collections::HashSet<&str> = COMMODITIES_UNIVERSE.iter().map(|(_, _, s)| *s).collect();
        assert!(sectors.contains("Metals"));
        assert!(sectors.contains("Energy"));
        assert!(sectors.contains("Grains"));
        assert!(sectors.contains("Softs"));
        assert!(sectors.contains("Livestock"));
    }

    #[test]
    fn commodities_universe_all_yahoo_futures_format() {
        for (sym, _, _) in COMMODITIES_UNIVERSE {
            assert!(sym.ends_with("=F"), "{} should end with =F", sym);
        }
    }

    #[test]
    fn company_profile_default_is_empty() {
        let p = CompanyProfile::default();
        assert!(p.symbol.is_empty());
        assert_eq!(p.market_cap, 0.0);
    }

    #[test]
    fn earning_row_all_optional() {
        let r = EarningRow::default();
        assert!(r.actual.is_none());
        assert!(r.estimate.is_none());
        assert!(r.surprise.is_none());
    }

    #[test]
    fn transcript_meta_roundtrip_json() {
        let m = TranscriptMeta { symbol: "AAPL".into(), quarter: 4, year: 2023, date: "2024-02-01".into() };
        let j = serde_json::to_string(&m).unwrap();
        let b: TranscriptMeta = serde_json::from_str(&j).unwrap();
        assert_eq!(b.symbol, "AAPL");
        assert_eq!(b.quarter, 4);
    }

    // ── ADR-109 ─────────────────────────────────────────────────────────

    fn open_mem_conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v2(&c).unwrap();
        c
    }

    #[test]
    fn dividend_record_roundtrip() {
        let c = open_mem_conn();
        let rows = vec![
            DividendRecord {
                ex_date: "2024-11-01".into(), pay_date: "2024-11-14".into(),
                record_date: "2024-11-04".into(), declaration_date: "2024-10-15".into(),
                amount: 0.24, adjusted_amount: 0.24, label: "Regular Cash".into(),
            },
        ];
        upsert_dividends(&c, "AAPL", &rows).unwrap();
        let got = get_dividends(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].amount, 0.24);
        assert_eq!(got[0].label, "Regular Cash");
    }

    #[test]
    fn earnings_estimate_roundtrip() {
        let c = open_mem_conn();
        let rows = vec![
            EarningsEstimate {
                date: "2025-12-31".into(),
                eps_avg: 2.45, eps_high: 2.60, eps_low: 2.30,
                revenue_avg: 123_000_000.0, revenue_high: 128_000_000.0, revenue_low: 118_000_000.0,
                num_analysts_eps: 12, num_analysts_rev: 12,
            },
        ];
        upsert_earnings_estimates(&c, "MSFT", &rows).unwrap();
        let got = get_earnings_estimates(&c, "MSFT").unwrap().unwrap();
        assert_eq!(got.len(), 1);
        assert!((got[0].eps_avg - 2.45).abs() < 1e-9);
        assert_eq!(got[0].num_analysts_eps, 12);
    }

    #[test]
    fn rating_change_roundtrip() {
        let c = open_mem_conn();
        let rows = vec![
            RatingChange {
                date: "2024-03-01".into(), symbol: "AAPL".into(),
                company: "Apple Inc.".into(), firm: "Morgan Stanley".into(),
                action: "upgrade".into(),
                from_grade: "Hold".into(), to_grade: "Buy".into(),
                price_target: 220.0,
            },
        ];
        upsert_rating_changes(&c, "AAPL", &rows).unwrap();
        let got = get_rating_changes(&c, "AAPL").unwrap().unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].action, "upgrade");
        assert!((got[0].price_target - 220.0).abs() < 1e-9);
    }

    #[test]
    fn treasury_tenor_ladder_has_four_rungs() {
        let tenors: std::collections::HashSet<&str> = TREASURY_TENORS.iter().map(|(_, t)| *t).collect();
        assert!(tenors.contains("13W"));
        assert!(tenors.contains("5Y"));
        assert!(tenors.contains("10Y"));
        assert!(tenors.contains("30Y"));
    }

    #[test]
    fn treasury_yield_default_is_empty() {
        let y = TreasuryYield::default();
        assert!(y.tenor.is_empty());
        assert_eq!(y.yield_pct, 0.0);
    }

    #[test]
    fn dividend_upsert_overwrites() {
        let c = open_mem_conn();
        upsert_dividends(&c, "IBM", &[
            DividendRecord { ex_date: "2024-05-01".into(), amount: 1.66, ..Default::default() }
        ]).unwrap();
        upsert_dividends(&c, "IBM", &[
            DividendRecord { ex_date: "2024-05-01".into(), amount: 1.67, ..Default::default() },
            DividendRecord { ex_date: "2024-08-01".into(), amount: 1.67, ..Default::default() },
        ]).unwrap();
        let rows = get_dividends(&c, "IBM").unwrap().unwrap();
        assert_eq!(rows.len(), 2);
    }

    // ── ADR-110 ─────────────────────────────────────────────────────────

    fn open_mem_conn_v3() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v3(&c).unwrap();
        c
    }

    #[test]
    fn financials_bundle_default_is_empty() {
        let b = FinancialStatements::default();
        assert!(b.income_annual.is_empty());
        assert!(b.income_quarterly.is_empty());
        assert!(b.balance_annual.is_empty());
        assert!(b.balance_quarterly.is_empty());
        assert!(b.cashflow_annual.is_empty());
        assert!(b.cashflow_quarterly.is_empty());
    }

    #[test]
    fn financials_bundle_roundtrip() {
        let c = open_mem_conn_v3();
        let mut b = FinancialStatements::default();
        b.income_annual.push(IncomeStatement {
            date: "2024-09-30".into(), period: "FY".into(),
            revenue: 400_000_000_000.0, net_income: 97_000_000_000.0,
            ebitda: 135_000_000_000.0, eps: 6.12, eps_diluted: 6.08,
            ..Default::default()
        });
        b.balance_quarterly.push(BalanceSheet {
            date: "2024-06-30".into(), period: "Q3".into(),
            total_assets: 350_000_000_000.0, total_liabilities: 270_000_000_000.0,
            total_equity: 80_000_000_000.0, total_debt: 110_000_000_000.0,
            ..Default::default()
        });
        b.cashflow_annual.push(CashFlowStatement {
            date: "2024-09-30".into(), period: "FY".into(),
            cash_from_operations: 118_000_000_000.0, capex: -11_000_000_000.0,
            free_cash_flow: 107_000_000_000.0,
            ..Default::default()
        });
        upsert_financials(&c, "AAPL", &b).unwrap();
        let got = get_financials(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.income_annual.len(), 1);
        assert_eq!(got.balance_quarterly.len(), 1);
        assert_eq!(got.cashflow_annual.len(), 1);
        assert!((got.income_annual[0].eps - 6.12).abs() < 1e-9);
        assert!((got.cashflow_annual[0].free_cash_flow - 107_000_000_000.0).abs() < 1.0);
    }

    #[test]
    fn financials_upsert_replaces() {
        let c = open_mem_conn_v3();
        let mut b1 = FinancialStatements::default();
        b1.income_annual.push(IncomeStatement { date: "2023-09-30".into(), revenue: 1.0, ..Default::default() });
        upsert_financials(&c, "T", &b1).unwrap();
        let mut b2 = FinancialStatements::default();
        b2.income_annual.push(IncomeStatement { date: "2024-09-30".into(), revenue: 2.0, ..Default::default() });
        b2.income_annual.push(IncomeStatement { date: "2023-09-30".into(), revenue: 1.0, ..Default::default() });
        upsert_financials(&c, "T", &b2).unwrap();
        let got = get_financials(&c, "T").unwrap().unwrap();
        assert_eq!(got.income_annual.len(), 2);
    }

    #[test]
    fn executive_roundtrip() {
        let c = open_mem_conn_v3();
        let rows = vec![
            Executive {
                name: "Tim Cook".into(), position: "CEO".into(),
                age: 64, sex: "M".into(), since: "2011".into(),
                compensation: 74_600_000.0, year: 2023,
            },
            Executive {
                name: "Luca Maestri".into(), position: "CFO".into(),
                age: 60, sex: "M".into(), since: "2014".into(),
                compensation: 27_100_000.0, year: 2023,
            },
        ];
        upsert_executives(&c, "AAPL", &rows).unwrap();
        let got = get_executives(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].name, "Tim Cook");
        assert!((got[1].compensation - 27_100_000.0).abs() < 1.0);
    }

    #[test]
    fn cot_report_default_is_empty() {
        let r = CotReport::default();
        assert!(r.market_name.is_empty());
        assert_eq!(r.open_interest, 0.0);
        assert_eq!(r.noncomm_net, 0.0);
        assert_eq!(r.noncomm_net_change, 0.0);
    }

    #[test]
    fn cot_report_net_math() {
        // Derived invariant used by the UI's coloring / direction signal.
        let r = CotReport {
            noncomm_long: 120_000.0, noncomm_short: 45_000.0,
            noncomm_net: 120_000.0 - 45_000.0,
            noncomm_net_change: 5_000.0,
            ..Default::default()
        };
        assert!((r.noncomm_net - 75_000.0).abs() < 1e-9);
        assert!(r.noncomm_net_change > 0.0);
    }

    // ── ADR-111 ─────────────────────────────────────────────────────────

    fn open_mem_conn_v4() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v4(&c).unwrap();
        c
    }

    #[test]
    fn stock_split_default_is_empty() {
        let s = StockSplit::default();
        assert!(s.date.is_empty());
        assert!(s.label.is_empty());
        assert_eq!(s.numerator, 0.0);
        assert_eq!(s.denominator, 0.0);
    }

    #[test]
    fn stock_split_roundtrip() {
        let c = open_mem_conn_v4();
        let rows = vec![
            StockSplit { date: "2020-08-31".into(), label: "4:1".into(), numerator: 4.0, denominator: 1.0 },
            StockSplit { date: "2014-06-09".into(), label: "7:1".into(), numerator: 7.0, denominator: 1.0 },
        ];
        upsert_stock_splits(&c, "AAPL", &rows).unwrap();
        let got = get_stock_splits(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].label, "4:1");
        assert!((got[1].numerator - 7.0).abs() < 1e-9);
    }

    #[test]
    fn etf_holding_roundtrip() {
        let c = open_mem_conn_v4();
        let rows = vec![
            EtfHolding {
                symbol: "AAPL".into(), name: "Apple Inc.".into(),
                weight_pct: 7.21, shares: 176_000_000.0, market_value: 34_500_000_000.0,
                updated: "2024-06-30".into(),
            },
            EtfHolding {
                symbol: "MSFT".into(), name: "Microsoft Corp.".into(),
                weight_pct: 6.87, shares: 83_000_000.0, market_value: 32_900_000_000.0,
                updated: "2024-06-30".into(),
            },
        ];
        upsert_etf_holdings(&c, "SPY", &rows).unwrap();
        let got = get_etf_holdings(&c, "spy").unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].symbol, "AAPL");
        assert!((got[1].weight_pct - 6.87).abs() < 1e-9);
    }

    #[test]
    fn analyst_rec_roundtrip() {
        let c = open_mem_conn_v4();
        let rows = vec![
            AnalystRecommendation {
                period: "2026-04-01".into(),
                strong_buy: 15, buy: 12, hold: 8, sell: 1, strong_sell: 0,
            },
            AnalystRecommendation {
                period: "2026-03-01".into(),
                strong_buy: 14, buy: 13, hold: 9, sell: 1, strong_sell: 0,
            },
        ];
        upsert_analyst_recs(&c, "AAPL", &rows).unwrap();
        let got = get_analyst_recs(&c, "AAPL").unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].strong_buy, 15);
        assert_eq!(got[1].hold, 9);
    }

    #[test]
    fn price_target_default_is_empty() {
        let p = PriceTarget::default();
        assert!(p.symbol.is_empty());
        assert_eq!(p.target_mean, 0.0);
        assert_eq!(p.num_analysts, 0);
    }

    #[test]
    fn price_target_roundtrip() {
        let c = open_mem_conn_v4();
        let pt = PriceTarget {
            symbol: "NVDA".into(),
            target_high: 220.0, target_low: 140.0,
            target_mean: 185.50, target_median: 190.0,
            last_updated: "2026-04-10".into(),
            num_analysts: 45,
        };
        upsert_price_target(&c, "NVDA", &pt).unwrap();
        let got = get_price_target(&c, "nvda").unwrap().unwrap();
        assert_eq!(got.num_analysts, 45);
        assert!((got.target_mean - 185.50).abs() < 1e-9);
    }

    #[test]
    fn price_target_upsert_replaces() {
        let c = open_mem_conn_v4();
        upsert_price_target(&c, "T", &PriceTarget {
            symbol: "T".into(), target_mean: 20.0, num_analysts: 10, ..Default::default()
        }).unwrap();
        upsert_price_target(&c, "T", &PriceTarget {
            symbol: "T".into(), target_mean: 22.5, num_analysts: 12, ..Default::default()
        }).unwrap();
        let got = get_price_target(&c, "T").unwrap().unwrap();
        assert_eq!(got.num_analysts, 12);
        assert!((got.target_mean - 22.5).abs() < 1e-9);
    }

    #[test]
    fn esg_roundtrip() {
        let c = open_mem_conn_v4();
        let rows = vec![
            EsgScore {
                symbol: "AAPL".into(),
                environmental_score: 78.5, social_score: 71.2, governance_score: 82.3,
                esg_score: 77.3, year: 2024,
            },
            EsgScore {
                symbol: "AAPL".into(),
                environmental_score: 76.0, social_score: 70.0, governance_score: 80.5,
                esg_score: 75.5, year: 2023,
            },
        ];
        upsert_esg(&c, "AAPL", &rows).unwrap();
        let got = get_esg(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].year, 2024);
        assert!((got[0].esg_score - 77.3).abs() < 1e-9);
    }

    #[test]
    fn index_member_roundtrip() {
        let c = open_mem_conn_v4();
        let rows = vec![
            IndexMember {
                index: "SP500".into(), symbol: "AAPL".into(), name: "Apple Inc.".into(),
                sector: "Information Technology".into(), sub_sector: "Technology Hardware".into(),
                headquarters: "Cupertino, CA".into(), date_added: "1982-11-30".into(),
            },
            IndexMember {
                index: "SP500".into(), symbol: "MSFT".into(), name: "Microsoft Corp.".into(),
                sector: "Information Technology".into(), sub_sector: "Software".into(),
                headquarters: "Redmond, WA".into(), date_added: "1994-06-01".into(),
            },
        ];
        upsert_index_members(&c, "SP500", &rows).unwrap();
        let got = get_index_members(&c, "sp500").unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].symbol, "AAPL");
        assert_eq!(got[1].sector, "Information Technology");
    }

    // ── ADR-112 Round 5 ─────────────────────────────────────────────────

    fn open_mem_conn_v5() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v5(&c).unwrap();
        c
    }

    #[test]
    fn insider_trade_default_is_empty() {
        let t = InsiderTrade::default();
        assert!(t.reporting_name.is_empty());
        assert_eq!(t.shares, 0.0);
        assert_eq!(t.value_usd, 0.0);
    }

    #[test]
    fn insider_trade_roundtrip() {
        let c = open_mem_conn_v5();
        let rows = vec![
            InsiderTrade {
                filing_date: "2026-03-10".into(),
                transaction_date: "2026-03-08".into(),
                reporting_name: "Musk, Elon".into(),
                transaction_type: "S-Sale".into(),
                acquisition_disposition: "D".into(),
                shares: 150_000.0,
                price: 245.60,
                value_usd: 150_000.0 * 245.60,
                shares_owned_after: 411_000_000.0,
                link: "https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&CIK=0001318605".into(),
            },
            InsiderTrade {
                filing_date: "2026-02-11".into(),
                transaction_date: "2026-02-10".into(),
                reporting_name: "Taneja, Vaibhav".into(),
                transaction_type: "P-Purchase".into(),
                acquisition_disposition: "A".into(),
                shares: 2_500.0,
                price: 180.00,
                value_usd: 2_500.0 * 180.0,
                shares_owned_after: 42_000.0,
                link: "".into(),
            },
        ];
        upsert_insider_trades(&c, "TSLA", &rows).unwrap();
        let got = get_insider_trades(&c, "tsla").unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].transaction_type, "S-Sale");
        assert_eq!(got[1].acquisition_disposition, "A");
        assert!((got[0].value_usd - 150_000.0 * 245.60).abs() < 1e-6);
    }

    #[test]
    fn institutional_holder_roundtrip() {
        let c = open_mem_conn_v5();
        let rows = vec![
            InstitutionalHolder {
                holder: "Vanguard Group Inc.".into(),
                shares: 1_200_000_000.0,
                date_reported: "2025-12-31".into(),
                change: 12_000_000.0,
            },
            InstitutionalHolder {
                holder: "BlackRock Inc.".into(),
                shares: 1_050_000_000.0,
                date_reported: "2025-12-31".into(),
                change: -4_500_000.0,
            },
        ];
        upsert_institutional_holders(&c, "AAPL", &rows).unwrap();
        let got = get_institutional_holders(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].holder, "Vanguard Group Inc.");
        assert!(got[1].change < 0.0);
    }

    #[test]
    fn shares_float_default_is_empty() {
        let f = SharesFloat::default();
        assert!(f.symbol.is_empty());
        assert_eq!(f.free_float_pct, 0.0);
        assert_eq!(f.outstanding_shares, 0.0);
    }

    #[test]
    fn shares_float_roundtrip() {
        let c = open_mem_conn_v5();
        let snap = SharesFloat {
            symbol: "NVDA".into(),
            date: "2026-04-01".into(),
            free_float_pct: 95.8,
            float_shares: 23_500_000_000.0,
            outstanding_shares: 24_530_000_000.0,
            source: "FMP".into(),
        };
        upsert_shares_float(&c, "NVDA", &snap).unwrap();
        let got = get_shares_float(&c, "nvda").unwrap().unwrap();
        assert_eq!(got.symbol, "NVDA");
        assert!((got.free_float_pct - 95.8).abs() < 1e-9);
        assert!((got.outstanding_shares - 24_530_000_000.0).abs() < 1.0);
    }

    #[test]
    fn historical_price_roundtrip() {
        let c = open_mem_conn_v5();
        let rows = vec![
            HistoricalPriceRow {
                date: "2026-04-13".into(),
                open: 180.0, high: 183.5, low: 179.2, close: 182.9,
                adj_close: 182.9, volume: 48_500_000.0,
                change: 2.9, change_pct: 1.61,
            },
            HistoricalPriceRow {
                date: "2026-04-12".into(),
                open: 178.1, high: 180.4, low: 177.8, close: 180.0,
                adj_close: 180.0, volume: 42_100_000.0,
                change: 1.9, change_pct: 1.07,
            },
        ];
        upsert_historical_price(&c, "AAPL", &rows).unwrap();
        let got = get_historical_price(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].date, "2026-04-13");
        assert!((got[0].change_pct - 1.61).abs() < 1e-9);
    }

    #[test]
    fn earnings_surprise_roundtrip() {
        let c = open_mem_conn_v5();
        let rows = vec![
            EarningsSurprise {
                date: "2026-02-01".into(),
                symbol: "AAPL".into(),
                eps_actual: 2.18,
                eps_estimate: 2.11,
                surprise: 0.07,
                surprise_pct: (0.07 / 2.11) * 100.0,
            },
            EarningsSurprise {
                date: "2025-11-01".into(),
                symbol: "AAPL".into(),
                eps_actual: 1.64,
                eps_estimate: 1.60,
                surprise: 0.04,
                surprise_pct: (0.04 / 1.60) * 100.0,
            },
        ];
        upsert_earnings_surprises(&c, "AAPL", &rows).unwrap();
        let got = get_earnings_surprises(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert!(got[0].surprise > 0.0);
        assert!((got[0].surprise_pct - (0.07 / 2.11) * 100.0).abs() < 1e-9);
    }

    #[test]
    fn earnings_surprise_upsert_replaces() {
        let c = open_mem_conn_v5();
        upsert_earnings_surprises(&c, "T", &[
            EarningsSurprise { date: "2025-10-01".into(), symbol: "T".into(),
                eps_actual: 0.55, eps_estimate: 0.58, surprise: -0.03, surprise_pct: -5.17 }
        ]).unwrap();
        upsert_earnings_surprises(&c, "T", &[
            EarningsSurprise { date: "2026-01-01".into(), symbol: "T".into(),
                eps_actual: 0.60, eps_estimate: 0.57, surprise: 0.03, surprise_pct: 5.26 }
        ]).unwrap();
        let got = get_earnings_surprises(&c, "T").unwrap().unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].date, "2026-01-01");
        assert!(got[0].surprise > 0.0);
    }

    // ── ADR-113 Round 6 ─────────────────────────────────────────────────

    fn open_mem_conn_v6() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v6(&c).unwrap();
        c
    }

    #[test]
    fn world_indices_universe_has_all_regions() {
        let regions: std::collections::HashSet<&str> =
            WORLD_INDICES_UNIVERSE.iter().map(|(_, _, r)| *r).collect();
        assert!(regions.contains("Americas"));
        assert!(regions.contains("EMEA"));
        assert!(regions.contains("Asia-Pacific"));
    }

    #[test]
    fn world_indices_universe_has_sp500_and_nikkei() {
        let tickers: std::collections::HashSet<&str> =
            WORLD_INDICES_UNIVERSE.iter().map(|(t, _, _)| *t).collect();
        assert!(tickers.contains("^GSPC"));
        assert!(tickers.contains("^N225"));
        assert!(tickers.contains("^FTSE"));
    }

    #[test]
    fn world_indices_roundtrip() {
        let c = open_mem_conn_v6();
        let rows = vec![
            WorldIndex { ticker: "^GSPC".into(), display: "S&P 500".into(), region: "Americas".into(),
                price: 5200.0, change: 12.5, change_pct: 0.24 },
            WorldIndex { ticker: "^N225".into(), display: "Nikkei 225".into(), region: "Asia-Pacific".into(),
                price: 39_800.0, change: -150.0, change_pct: -0.38 },
        ];
        upsert_world_indices(&c, &rows).unwrap();
        let got = get_world_indices(&c).unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].ticker, "^GSPC");
        assert!(got[1].change < 0.0);
    }

    #[test]
    fn world_indices_upsert_replaces() {
        let c = open_mem_conn_v6();
        upsert_world_indices(&c, &[
            WorldIndex { ticker: "^GSPC".into(), price: 5000.0, ..Default::default() },
        ]).unwrap();
        upsert_world_indices(&c, &[
            WorldIndex { ticker: "^GSPC".into(), price: 5300.0, ..Default::default() },
            WorldIndex { ticker: "^DJI".into(), price: 42_000.0, ..Default::default() },
        ]).unwrap();
        let got = get_world_indices(&c).unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert!((got[0].price - 5300.0).abs() < 1e-9);
    }

    #[test]
    fn market_movers_roundtrip() {
        let c = open_mem_conn_v6();
        let movers = MarketMovers {
            gainers: vec![
                MarketMover { symbol: "AAA".into(), name: "Alpha Inc.".into(),
                    price: 12.5, change: 2.1, change_pct: 20.19, volume: 1_200_000.0 },
            ],
            losers: vec![
                MarketMover { symbol: "ZZZ".into(), name: "Omega Corp.".into(),
                    price: 4.8, change: -1.1, change_pct: -18.64, volume: 900_000.0 },
            ],
            actives: vec![
                MarketMover { symbol: "TSLA".into(), name: "Tesla Inc.".into(),
                    price: 190.25, change: 1.15, change_pct: 0.61, volume: 120_000_000.0 },
            ],
        };
        upsert_market_movers(&c, &movers).unwrap();
        let got = get_market_movers(&c).unwrap().unwrap();
        assert_eq!(got.gainers.len(), 1);
        assert_eq!(got.losers.len(), 1);
        assert_eq!(got.actives.len(), 1);
        assert_eq!(got.gainers[0].symbol, "AAA");
        assert!(got.losers[0].change_pct < 0.0);
        assert_eq!(got.actives[0].symbol, "TSLA");
    }

    #[test]
    fn sector_performance_roundtrip() {
        let c = open_mem_conn_v6();
        let rows = vec![
            SectorPerformance { sector: "Technology".into(),      change_pct: 1.23 },
            SectorPerformance { sector: "Energy".into(),          change_pct: -0.45 },
            SectorPerformance { sector: "Financial Services".into(), change_pct: 0.78 },
        ];
        upsert_sector_performance(&c, &rows).unwrap();
        let got = get_sector_performance(&c).unwrap().unwrap();
        assert_eq!(got.len(), 3);
        assert_eq!(got[0].sector, "Technology");
        assert!(got[1].change_pct < 0.0);
    }

    #[test]
    fn wacc_compute_basic_calc() {
        let s = compute_wacc_snapshot(
            "AAPL", "2026-04-14",
            1.20,              // beta
            3_000_000_000_000.0, // market cap (3T)
            4.50,              // Rf %
            100_000_000_000.0, // total debt (100B)
            5_000_000_000.0,  // interest expense (5B)
            16.0,              // effective tax rate %
        );
        // Cost of equity = 4.5 + 1.20 * 5.0 = 10.5 %
        assert!((s.cost_of_equity_pct - 10.5).abs() < 1e-6);
        // Pre-tax cost of debt = (5B / 100B) * 100 = 5.0 %
        assert!((s.pre_tax_cost_of_debt_pct - 5.0).abs() < 1e-6);
        // After-tax = 5.0 * (1 - 0.16) = 4.2 %
        assert!((s.after_tax_cost_of_debt_pct - 4.2).abs() < 1e-6);
        // Weights: E=3T / (3T+100B) ≈ 0.9677, D ≈ 0.0323
        assert!((s.equity_weight - 3000.0/3100.0).abs() < 1e-6);
        // WACC ≈ 0.9677*10.5 + 0.0323*4.2 ≈ 10.296
        let expected = (3000.0/3100.0)*10.5 + (100.0/3100.0)*4.2;
        assert!((s.wacc_pct - expected).abs() < 1e-6);
    }

    #[test]
    fn wacc_handles_zero_debt() {
        let s = compute_wacc_snapshot(
            "NVDA", "2026-04-14",
            1.80,   // beta
            2_500_000_000_000.0, // market cap
            4.30,   // Rf
            0.0,    // no debt
            0.0,    // no interest expense
            12.0,   // tax
        );
        assert_eq!(s.pre_tax_cost_of_debt_pct, 0.0);
        assert_eq!(s.debt_weight, 0.0);
        assert!((s.equity_weight - 1.0).abs() < 1e-9);
        // WACC == Re when all equity
        assert!((s.wacc_pct - s.cost_of_equity_pct).abs() < 1e-9);
    }

    #[test]
    fn wacc_roundtrip() {
        let c = open_mem_conn_v6();
        let snap = compute_wacc_snapshot("AAPL", "2026-04-14",
            1.20, 3_000_000_000_000.0, 4.50,
            100_000_000_000.0, 5_000_000_000.0, 16.0);
        upsert_wacc(&c, "AAPL", &snap).unwrap();
        let got = get_wacc(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.symbol, "AAPL");
        assert!((got.wacc_pct - snap.wacc_pct).abs() < 1e-9);
        assert!((got.beta - 1.20).abs() < 1e-9);
    }

    #[test]
    fn fmp_mover_parses_string_percentage() {
        // FMP sometimes returns changesPercentage as "1.23%" (string), sometimes as f64.
        let v: serde_json::Value = serde_json::from_str(r#"{
            "symbol":"AAPL","name":"Apple","price":185.5,"change":2.1,
            "changesPercentage":"1.15%","volume":45000000
        }"#).unwrap();
        let m = parse_fmp_mover(&v);
        assert_eq!(m.symbol, "AAPL");
        assert!((m.change_pct - 1.15).abs() < 1e-9);
        assert!((m.volume - 45_000_000.0).abs() < 1.0);
    }

    // ── ADR-114 Round 7 ─────────────────────────────────────────────────

    fn open_mem_conn_v7() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v7(&c).unwrap();
        c
    }

    #[test]
    fn fx_majors_universe_has_regions() {
        let regions: std::collections::HashSet<&str> =
            FX_MAJORS_UNIVERSE.iter().map(|(_, _, _, _, r)| *r).collect();
        assert!(regions.contains("Majors"));
        assert!(regions.contains("Crosses"));
        assert!(regions.contains("EM"));
    }

    #[test]
    fn fx_majors_universe_has_eurusd_and_usdjpy() {
        let tickers: std::collections::HashSet<&str> =
            FX_MAJORS_UNIVERSE.iter().map(|(t, _, _, _, _)| *t).collect();
        assert!(tickers.contains("EURUSD=X"));
        assert!(tickers.contains("USDJPY=X"));
        assert!(tickers.contains("USDMXN=X"));
    }

    #[test]
    fn currency_rates_roundtrip() {
        let c = open_mem_conn_v7();
        let rows = vec![
            CurrencyRate {
                ticker: "EURUSD=X".into(), display: "EUR/USD".into(),
                base: "EUR".into(), quote: "USD".into(), region: "Majors".into(),
                price: 1.0850, change: 0.0020, change_pct: 0.18,
            },
            CurrencyRate {
                ticker: "USDJPY=X".into(), display: "USD/JPY".into(),
                base: "USD".into(), quote: "JPY".into(), region: "Majors".into(),
                price: 151.25, change: -0.35, change_pct: -0.23,
            },
        ];
        upsert_currency_rates(&c, &rows).unwrap();
        let got = get_currency_rates(&c).unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].display, "EUR/USD");
        assert!(got[1].change < 0.0);
    }

    #[test]
    fn ols_regression_perfect_correlation() {
        // If s_i == 2 * m_i exactly, beta should be exactly 2.0, R² = 1.
        let m: Vec<f64> = vec![0.01, -0.005, 0.02, -0.01, 0.015, 0.008, -0.003, 0.012, 0.005, -0.007, 0.018, -0.002];
        let s: Vec<f64> = m.iter().map(|x| 2.0 * x).collect();
        let (beta, _alpha, r2, corr, n) = ols_regression(&s, &m);
        assert!((beta - 2.0).abs() < 1e-9);
        assert!((r2 - 1.0).abs() < 1e-9);
        assert!((corr - 1.0).abs() < 1e-9);
        assert_eq!(n, 12);
    }

    #[test]
    fn compute_beta_snapshot_synthetic_2x_market() {
        // Build symbol bars that exactly track 2× market moves. Expected β ≈ 2.0.
        // Use 300 bars so the 1Y window (252) fits with headroom. FMP order is
        // newest-first, so we build newest → oldest. Dates must be unique —
        // we use a simple days-since-epoch counter so the join by date key
        // does not collide.
        let mut sym_bars: Vec<HistoricalPriceRow> = Vec::new();
        let mut mkt_bars: Vec<HistoricalPriceRow> = Vec::new();
        let mut sym_close = 100.0_f64;
        let mut mkt_close = 400.0_f64;
        for i in 0..300 {
            let daily = 0.01 * ((i as f64 * 0.37).sin());
            mkt_close *= 1.0 + daily;
            sym_close *= 1.0 + 2.0 * daily;
            // Fake-but-unique ISO date: walk calendar by 1-day increments from 2024-01-01.
            let base_day = 1 + (i % 28); // 1..=28
            let month = 1 + ((i / 28) % 12); // 1..=12
            let year = 2024 + ((i / (28 * 12)) as i32);
            let date = format!("{:04}-{:02}-{:02}", year, month, base_day);
            sym_bars.push(HistoricalPriceRow {
                date: date.clone(), open: sym_close, high: sym_close, low: sym_close,
                close: sym_close, adj_close: sym_close, volume: 0.0, change: 0.0, change_pct: 0.0,
            });
            mkt_bars.push(HistoricalPriceRow {
                date, open: mkt_close, high: mkt_close, low: mkt_close,
                close: mkt_close, adj_close: mkt_close, volume: 0.0, change: 0.0, change_pct: 0.0,
            });
        }
        // The loop already pushes in synthetic chronological order — we need
        // FMP's newest-first orientation, so reverse.
        sym_bars.reverse();
        mkt_bars.reverse();
        let snap = compute_beta_snapshot("TST", "SPY", "2026-04-14", &sym_bars, &mkt_bars);
        assert!(!snap.windows.is_empty());
        let w1y = snap.windows.iter().find(|w| w.window_label == "1Y").unwrap();
        assert!((w1y.beta - 2.0).abs() < 0.01, "beta was {}", w1y.beta);
        assert!(w1y.r_squared > 0.99);
    }

    #[test]
    fn beta_snapshot_roundtrip() {
        let c = open_mem_conn_v7();
        let snap = BetaSnapshot {
            symbol: "AAPL".into(),
            market_ticker: "SPY".into(),
            as_of: "2026-04-14".into(),
            windows: vec![
                BetaWindow { window_label: "1Y".into(), window_days: 252,
                    beta: 1.18, alpha_pct: 2.4, r_squared: 0.67,
                    n_observations: 252, correlation: 0.82 },
                BetaWindow { window_label: "5Y".into(), window_days: 1260,
                    beta: 1.23, alpha_pct: 4.1, r_squared: 0.71,
                    n_observations: 1260, correlation: 0.84 },
            ],
            note: String::new(),
        };
        upsert_beta(&c, "AAPL", &snap).unwrap();
        let got = get_beta(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.symbol, "AAPL");
        assert_eq!(got.windows.len(), 2);
        assert!((got.windows[0].beta - 1.18).abs() < 1e-9);
    }

    #[test]
    fn compute_ddm_basic_growth() {
        // 10 years of dividends with 7% annual growth, required return 12% → finite price.
        let mut divs: Vec<DividendRecord> = Vec::new();
        let base = 1.00_f64;
        for y in 2016..=2025 {
            let growth = 1.07_f64.powi(y - 2016);
            for q in 1..=4 {
                divs.push(DividendRecord {
                    ex_date: format!("{}-{:02}-15", y, 1 + (q - 1) * 3),
                    pay_date: format!("{}-{:02}-28", y, 1 + (q - 1) * 3),
                    record_date: String::new(), declaration_date: String::new(),
                    amount: base * growth * 0.25,
                    adjusted_amount: base * growth * 0.25,
                    label: "Regular Cash".into(),
                });
            }
        }
        // Newest-first: sort descending by ex_date.
        divs.sort_by(|a, b| b.ex_date.cmp(&a.ex_date));
        let snap = compute_ddm_snapshot("AAA", "2026-04-14", &divs, 12.0, "WACC 12%");
        assert!(snap.annual_dividend > 0.0);
        assert!(snap.implied_growth_pct > 4.0 && snap.implied_growth_pct < 10.0,
            "growth was {}", snap.implied_growth_pct);
        assert!(snap.implied_price > 0.0);
        assert!(snap.note.is_empty());
    }

    #[test]
    fn compute_ddm_diverges_when_growth_exceeds_return() {
        let divs = vec![
            DividendRecord { ex_date: "2025-01-15".into(), amount: 1.0, adjusted_amount: 1.0, ..Default::default() },
            DividendRecord { ex_date: "2024-01-15".into(), amount: 0.80, adjusted_amount: 0.80, ..Default::default() },
            DividendRecord { ex_date: "2023-01-15".into(), amount: 0.60, adjusted_amount: 0.60, ..Default::default() },
            DividendRecord { ex_date: "2022-01-15".into(), amount: 0.45, adjusted_amount: 0.45, ..Default::default() },
        ];
        // Ask for very low required return — Gordon must diverge.
        let snap = compute_ddm_snapshot("BBB", "2026-04-14", &divs, 2.0, "manual");
        assert_eq!(snap.implied_price, 0.0);
        assert!(!snap.note.is_empty());
    }

    #[test]
    fn ddm_roundtrip() {
        let c = open_mem_conn_v7();
        let snap = DdmSnapshot {
            symbol: "KO".into(),
            as_of: "2026-04-14".into(),
            annual_dividend: 1.92,
            implied_growth_pct: 4.5,
            required_return_pct: 8.0,
            growth_source: "5Y dividend CAGR".into(),
            return_source: "WACC 8.0%".into(),
            implied_price: 57.34,
            method: "Gordon Growth".into(),
            note: String::new(),
        };
        upsert_ddm(&c, "KO", &snap).unwrap();
        let got = get_ddm(&c, "ko").unwrap().unwrap();
        assert_eq!(got.symbol, "KO");
        assert!((got.implied_price - 57.34).abs() < 1e-9);
    }

    #[test]
    fn compute_relative_valuation_z_scores() {
        let inputs = vec![
            RvMetricInput {
                metric: "P/E",
                value: Some(30.0),
                peer_values: vec![10.0, 15.0, 20.0, 25.0, 28.0, 35.0, 40.0],
            },
            RvMetricInput {
                metric: "P/B",
                value: None, // should skip
                peer_values: vec![1.0, 2.0, 3.0, 4.0],
            },
            RvMetricInput {
                metric: "EV/EBITDA",
                value: Some(12.0),
                peer_values: vec![8.0, 10.0], // <3 peers — should skip
            },
        ];
        let rv = compute_relative_valuation("SUBJ", "Tech", "2026-04-14", &inputs);
        assert_eq!(rv.rows.len(), 1);
        let pe = &rv.rows[0];
        assert_eq!(pe.metric, "P/E");
        assert_eq!(pe.peer_low, 10.0);
        assert_eq!(pe.peer_high, 40.0);
        // 30 is higher than 5 of 7 peers → percentile ≈ 71.4
        assert!(pe.percentile > 60.0 && pe.percentile < 80.0);
        assert!(pe.z_score > 0.0); // above mean
    }

    #[test]
    fn relative_valuation_roundtrip() {
        let c = open_mem_conn_v7();
        let rv = RelativeValuation {
            symbol: "AAPL".into(),
            sector: "Technology".into(),
            as_of: "2026-04-14".into(),
            peer_count: 8,
            rows: vec![
                RvMetricRow { metric: "P/E".into(), value: 32.0, peer_median: 28.0,
                    peer_low: 12.0, peer_high: 60.0, z_score: 0.4, percentile: 62.5 },
            ],
        };
        upsert_relative_valuation(&c, "AAPL", &rv).unwrap();
        let got = get_relative_valuation(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.symbol, "AAPL");
        assert_eq!(got.rows.len(), 1);
        assert!((got.rows[0].value - 32.0).abs() < 1e-9);
    }

    #[test]
    fn figi_roundtrip() {
        let c = open_mem_conn_v7();
        let snap = FigiSnapshot {
            symbol: "AAPL".into(),
            as_of: "2026-04-14".into(),
            identifiers: vec![
                FigiIdentifier {
                    figi: "BBG000B9XRY4".into(),
                    name: "APPLE INC".into(),
                    ticker: "AAPL".into(),
                    exch_code: "US".into(),
                    composite_figi: "BBG000B9Y5X2".into(),
                    share_class_figi: "BBG001S5N8V8".into(),
                    security_type: "Common Stock".into(),
                    security_type_2: "Common Stock".into(),
                    market_sector: "Equity".into(),
                    security_description: "AAPL".into(),
                },
            ],
        };
        upsert_figi(&c, "AAPL", &snap).unwrap();
        let got = get_figi(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.symbol, "AAPL");
        assert_eq!(got.identifiers.len(), 1);
        assert_eq!(got.identifiers[0].figi, "BBG000B9XRY4");
    }

    // ── ADR-115 Round 8 tests ──────────────────────────────────────────

    fn open_mem_conn_v8() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v8(&c).unwrap();
        c
    }

    #[test]
    fn hra_roundtrip() {
        let c = open_mem_conn_v8();
        let snap = HraSnapshot {
            symbol: "AAPL".into(),
            as_of: "2026-04-14".into(),
            last_close: 190.0,
            windows: vec![HraWindow { label: "1Y".into(), trading_days: 252, return_pct: 22.0, cagr_pct: 22.0, n_observations: 252 }],
            max_drawdown_pct: -15.5,
            drawdown_peak_date: "2025-10-01".into(),
            drawdown_trough_date: "2025-12-15".into(),
            volatility_annual_pct: 24.0,
            sharpe_ratio: 1.1,
            sortino_ratio: 1.4,
            calmar_ratio: 1.4,
            risk_free_pct: 4.5,
            note: String::new(),
        };
        upsert_hra(&c, "AAPL", &snap).unwrap();
        let got = get_hra(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.symbol, "AAPL");
        assert_eq!(got.windows.len(), 1);
        assert!((got.max_drawdown_pct - (-15.5)).abs() < 1e-6);
    }

    #[test]
    fn dcf_roundtrip() {
        let c = open_mem_conn_v8();
        let snap = DcfSnapshot {
            symbol: "NVDA".into(),
            as_of: "2026-04-14".into(),
            method: "DCF on FCFF".into(),
            base_revenue: 60_000.0,
            base_fcff: 24_000.0,
            growth_pct: 20.0,
            terminal_growth_pct: 3.0,
            wacc_pct: 9.0,
            tax_rate_pct: 15.0,
            fcff_margin_pct: 40.0,
            projection_years: 5,
            years: Vec::new(),
            pv_sum: 100_000.0,
            terminal_value: 500_000.0,
            pv_terminal: 350_000.0,
            enterprise_value: 450_000.0,
            total_debt: 10_000.0,
            cash_and_equivalents: 30_000.0,
            equity_value: 470_000.0,
            shares_outstanding: 2_500.0,
            implied_price: 188.0,
            note: String::new(),
        };
        upsert_dcf(&c, "NVDA", &snap).unwrap();
        let got = get_dcf(&c, "nvda").unwrap().unwrap();
        assert_eq!(got.symbol, "NVDA");
        assert!((got.implied_price - 188.0).abs() < 1e-6);
    }

    #[test]
    fn svm_roundtrip() {
        let c = open_mem_conn_v8();
        let snap = SvmSnapshot {
            symbol: "MSFT".into(),
            as_of: "2026-04-14".into(),
            current_price: 420.0,
            rows: vec![SvmModelRow {
                model: "DCF on FCFF".into(),
                implied_price: 450.0,
                current_price: 420.0,
                upside_pct: 7.14,
                confidence: "medium".into(),
                source: "test".into(),
            }],
            fair_low: 450.0, fair_high: 450.0, fair_mid: 450.0,
            upside_mid_pct: 7.14,
            note: String::new(),
        };
        upsert_svm(&c, "MSFT", &snap).unwrap();
        let got = get_svm(&c, "msft").unwrap().unwrap();
        assert_eq!(got.rows.len(), 1);
        assert!((got.fair_mid - 450.0).abs() < 1e-6);
    }

    #[test]
    fn options_chain_roundtrip() {
        let c = open_mem_conn_v8();
        let snap = OptionsChainSnapshot {
            symbol: "SPY".into(),
            as_of: "2026-04-14".into(),
            underlying_price: 520.0,
            expirations: vec![OptionExpiry {
                expiration: "2026-05-16".into(),
                days_to_expiry: 32,
                calls: vec![OptionContract {
                    contract_symbol: "SPY260516C00520000".into(),
                    option_type: "CALL".into(),
                    strike: 520.0,
                    last_price: 8.5, bid: 8.4, ask: 8.6,
                    volume: 1200.0, open_interest: 5000.0,
                    implied_volatility: 0.18, in_the_money: false,
                }],
                puts: vec![],
            }],
            note: String::new(),
        };
        upsert_options_chain(&c, "SPY", &snap).unwrap();
        let got = get_options_chain(&c, "spy").unwrap().unwrap();
        assert_eq!(got.expirations.len(), 1);
        assert_eq!(got.expirations[0].calls.len(), 1);
        assert!((got.expirations[0].calls[0].strike - 520.0).abs() < 1e-6);
    }

    #[test]
    fn ivol_roundtrip() {
        let c = open_mem_conn_v8();
        let snap = IvolSnapshot {
            symbol: "TSLA".into(),
            as_of: "2026-04-14".into(),
            current_atm_iv_pct: 55.0,
            iv_52w_low_pct: 30.0,
            iv_52w_high_pct: 80.0,
            iv_rank: 50.0,
            iv_percentile: 60.0,
            observation_count: 100,
            history: vec![IvolObservation { date: "2026-01-01".into(), atm_iv_pct: 40.0 }],
            note: String::new(),
        };
        upsert_ivol(&c, "TSLA", &snap).unwrap();
        let got = get_ivol(&c, "tsla").unwrap().unwrap();
        assert!((got.iv_rank - 50.0).abs() < 1e-6);
        assert_eq!(got.history.len(), 1);
    }

    #[test]
    fn compute_hra_on_synthetic_uptrend() {
        // 300 daily bars, +0.1% per day → terminal ~ 1.001^299.
        let mut bars: Vec<HistoricalPriceRow> = Vec::new();
        let mut px = 100.0;
        for i in 0..300 {
            let base_day = 1 + (i % 28);
            let month    = 1 + ((i / 28) % 12);
            let year     = 2024 + (i / (28 * 12));
            bars.push(HistoricalPriceRow {
                date: format!("{:04}-{:02}-{:02}", year, month, base_day),
                open: px, high: px, low: px, close: px, adj_close: px,
                volume: 1_000.0, change: 0.0, change_pct: 0.1,
            });
            px *= 1.001;
        }
        let snap = compute_hra_snapshot("TEST", "2026-04-14", &bars, 4.5);
        assert_eq!(snap.symbol, "TEST");
        // 1Y window present
        assert!(snap.windows.iter().any(|w| w.label == "1Y"));
        // ITD should be strongly positive
        let itd = snap.windows.iter().find(|w| w.label == "ITD").unwrap();
        assert!(itd.return_pct > 0.0);
        // Monotonic uptrend → drawdown effectively zero (we accept very small
        // rounding-scale negatives).
        assert!(snap.max_drawdown_pct > -0.1, "expected near-zero drawdown on monotonic uptrend, got {}", snap.max_drawdown_pct);
    }

    #[test]
    fn compute_hra_on_empty_bars_returns_note() {
        let snap = compute_hra_snapshot("EMPTY", "2026-04-14", &[], 4.5);
        assert!(!snap.note.is_empty());
        assert_eq!(snap.windows.len(), 0);
    }

    #[test]
    fn compute_hra_drawdown_detects_peak_and_trough() {
        // 50 bars that rise to 150 at day 20, then fall to 100 by day 40, then
        // recover to 130 at day 49. Max DD is from peak 150 to trough 100.
        let mut bars: Vec<HistoricalPriceRow> = Vec::new();
        let mut push = |i: usize, close: f64| {
            let base_day = 1 + (i % 28);
            let month    = 1 + ((i / 28) % 12);
            let year     = 2024 + (i / (28 * 12));
            bars.push(HistoricalPriceRow {
                date: format!("{:04}-{:02}-{:02}", year, month, base_day),
                open: close, high: close, low: close, close, adj_close: close,
                volume: 1_000.0, change: 0.0, change_pct: 0.0,
            });
        };
        for i in 0..20 { push(i, 100.0 + (i as f64 * 2.5)); } // 100 → 147.5
        push(20, 150.0);                                      // peak
        for i in 21..=40 { push(i, 150.0 - ((i - 20) as f64 * 2.5)); } // 150 → 100
        for i in 41..50 { push(i, 100.0 + ((i - 40) as f64 * 3.333)); } // 100 → 130
        let snap = compute_hra_snapshot("X", "2026-04-14", &bars, 0.0);
        // Peak-to-trough 150→100 = -33.33%
        assert!(snap.max_drawdown_pct < -32.0 && snap.max_drawdown_pct > -34.0,
            "expected ~-33% drawdown, got {:.2}", snap.max_drawdown_pct);
    }

    #[test]
    fn compute_dcf_basic() {
        let snap = compute_dcf_snapshot("NVDA", "2026-04-14",
            /*revenue*/ 60_000.0, /*fcff*/ 24_000.0,
            /*g*/ 20.0, /*tg*/ 3.0, /*wacc*/ 9.0, /*tax*/ 15.0,
            /*years*/ 5, /*debt*/ 10_000.0, /*cash*/ 30_000.0,
            /*shares*/ 2_500.0);
        assert_eq!(snap.years.len(), 5);
        assert!(snap.enterprise_value > 0.0);
        assert!(snap.implied_price > 0.0);
        // Each projection year's fcff should compound
        assert!(snap.years[4].fcff > snap.years[0].fcff);
    }

    #[test]
    fn compute_dcf_rejects_terminal_growth_above_wacc() {
        let snap = compute_dcf_snapshot("X", "2026-04-14", 100.0, 40.0, 5.0, 15.0, 8.0, 20.0, 5, 10.0, 5.0, 100.0);
        assert!(!snap.note.is_empty());
        assert_eq!(snap.implied_price, 0.0);
    }

    #[test]
    fn compute_svm_triangulates_multiple_models() {
        let ddm = DdmSnapshot {
            symbol: "XYZ".into(), as_of: "2026-04-14".into(),
            annual_dividend: 3.0, implied_growth_pct: 4.0, required_return_pct: 10.0,
            growth_source: "test".into(), return_source: "test".into(),
            implied_price: 52.0, method: "Gordon Growth".into(), note: String::new(),
        };
        let dcf = DcfSnapshot {
            symbol: "XYZ".into(), as_of: "2026-04-14".into(), method: "DCF on FCFF".into(),
            base_revenue: 100.0, base_fcff: 20.0, growth_pct: 5.0, terminal_growth_pct: 2.0,
            wacc_pct: 10.0, tax_rate_pct: 20.0, fcff_margin_pct: 20.0, projection_years: 5,
            years: Vec::new(), pv_sum: 0.0, terminal_value: 0.0, pv_terminal: 0.0,
            enterprise_value: 0.0, total_debt: 0.0, cash_and_equivalents: 0.0,
            equity_value: 0.0, shares_outstanding: 1.0, implied_price: 58.0, note: String::new(),
        };
        let snap = compute_svm_snapshot(
            "XYZ", "2026-04-14", /*current*/ 50.0,
            Some(&ddm), Some(&dcf),
            Some((12.0, 4.5)),              // P/E × EPS → 54
            Some((10.0, 10.0, 5.0, 2.0, 1.0)), // EV/EBITDA 10 × 10 → EV 100 - 5 + 2 = 97 / 1 shares = 97
            Some((1.2, 45.0)),              // P/B × BVPS → 54
        );
        assert!(snap.rows.len() >= 4, "expected ≥4 triangulation rows, got {}", snap.rows.len());
        assert!(snap.fair_low > 0.0);
        assert!(snap.fair_mid >= snap.fair_low);
        assert!(snap.fair_high >= snap.fair_mid);
        assert!(snap.upside_mid_pct > 0.0, "at $50 current vs mid, upside should be positive");
    }

    #[test]
    fn compute_svm_with_no_models_emits_note() {
        let snap = compute_svm_snapshot("X", "2026-04-14", 50.0, None, None, None, None, None);
        assert!(snap.rows.is_empty());
        assert!(!snap.note.is_empty());
    }

    #[test]
    fn compute_ivol_rank_and_percentile() {
        let history: Vec<IvolObservation> = (0..100)
            .map(|i| IvolObservation { date: format!("2025-{:03}", i), atm_iv_pct: 20.0 + (i as f64 * 0.3) })
            .collect();
        // History spans 20% → 49.7%; current = 40%.
        let snap = compute_ivol_snapshot("TEST", "2026-04-14", 40.0, &history);
        // Rank: (40 - 20) / (49.7 - 20) × 100 ≈ 67
        assert!(snap.iv_rank > 50.0 && snap.iv_rank < 80.0,
            "expected rank 50-80, got {:.2}", snap.iv_rank);
        // Percentile: ~67% of observations ≤ 40
        assert!(snap.iv_percentile > 50.0 && snap.iv_percentile < 80.0,
            "expected percentile 50-80, got {:.2}", snap.iv_percentile);
    }

    #[test]
    fn compute_ivol_with_no_history_uses_placeholder() {
        let snap = compute_ivol_snapshot("NEW", "2026-04-14", 25.0, &[]);
        assert!(!snap.note.is_empty());
        assert!((snap.iv_52w_low_pct - 25.0).abs() < 1e-6);
        assert!((snap.iv_52w_high_pct - 25.0).abs() < 1e-6);
    }

    // ── ADR-116 Round 9 tests ──────────────────────────────────────────

    fn open_mem_conn_v9() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v9(&c).unwrap();
        c
    }

    fn synth_bars(n: usize, start: f64, daily_drift: f64) -> Vec<HistoricalPriceRow> {
        let mut bars = Vec::with_capacity(n);
        let mut px = start;
        for i in 0..n {
            let base_day = 1 + (i % 28);
            let month    = 1 + ((i / 28) % 12);
            let year     = 2024 + (i / (28 * 12));
            bars.push(HistoricalPriceRow {
                date: format!("{:04}-{:02}-{:02}", year, month, base_day),
                open: px, high: px * 1.005, low: px * 0.995,
                close: px, adj_close: px,
                volume: 1_000.0, change: 0.0, change_pct: 0.0,
            });
            px *= 1.0 + daily_drift;
        }
        bars
    }

    #[test]
    fn seasonality_snapshot_roundtrip() {
        let c = open_mem_conn_v9();
        let snap = SeasonalitySnapshot {
            symbol: "AAPL".into(),
            as_of: "2026-04-14".into(),
            years_covered: 3,
            months: vec![SeasonalityMonth {
                month: 1, label: "Jan".into(),
                avg_return_pct: 2.1, median_return_pct: 1.8, stdev_pct: 3.4,
                positive_years: 2, total_years: 3,
                best_return_pct: 5.1, worst_return_pct: -1.2,
            }],
            dow: vec![SeasonalityDow { dow: 1, label: "Mon".into(), avg_return_pct: 0.05, positive_days: 28, total_days: 52 }],
            best_month: "Jul".into(),
            worst_month: "Sep".into(),
            note: String::new(),
        };
        upsert_seasonality(&c, "AAPL", &snap).unwrap();
        let got = get_seasonality(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.symbol, "AAPL");
        assert_eq!(got.months.len(), 1);
        assert_eq!(got.best_month, "Jul");
    }

    #[test]
    fn correlation_matrix_roundtrip() {
        let c = open_mem_conn_v9();
        let snap = CorrelationMatrix {
            symbol: "AAPL".into(),
            as_of: "2026-04-14".into(),
            window_days: 252,
            cells: vec![CorrelationCell { peer_symbol: "MSFT".into(), correlation: 0.85, n_observations: 245, beta_vs_peer: 0.92 }],
            mean_correlation: 0.85,
            highest_corr_symbol: "MSFT".into(),
            lowest_corr_symbol: "MSFT".into(),
            note: String::new(),
        };
        upsert_correlation(&c, "AAPL", &snap).unwrap();
        let got = get_correlation(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.cells.len(), 1);
        assert!((got.mean_correlation - 0.85).abs() < 1e-6);
    }

    #[test]
    fn total_return_snapshot_roundtrip() {
        let c = open_mem_conn_v9();
        let snap = TotalReturnSnapshot {
            symbol: "KO".into(),
            as_of: "2026-04-14".into(),
            last_close: 60.0,
            trailing_12m_dividends: 1.84,
            trailing_12m_yield_pct: 3.07,
            windows: vec![TotalReturnWindow {
                label: "1Y".into(),
                trading_days: 252,
                price_return_pct: 8.0,
                dividend_yield_pct: 3.1,
                total_return_pct: 11.1,
                annualized_pct: 11.1,
                dividends_paid: 1.84,
                n_dividends: 4,
            }],
            note: String::new(),
        };
        upsert_total_return(&c, "KO", &snap).unwrap();
        let got = get_total_return(&c, "ko").unwrap().unwrap();
        assert_eq!(got.windows.len(), 1);
        assert!((got.trailing_12m_yield_pct - 3.07).abs() < 1e-6);
    }

    #[test]
    fn technicals_snapshot_roundtrip() {
        let c = open_mem_conn_v9();
        let snap = TechnicalSnapshot {
            symbol: "NVDA".into(),
            as_of: "2026-04-14".into(),
            last_close: 850.0,
            indicators: vec![TechnicalIndicator {
                name: "RSI(14)".into(),
                value: 72.5, value_secondary: 0.0, value_tertiary: 0.0,
                signal: "overbought".into(), note: String::new(),
            }],
            trend_summary: "bullish composite".into(),
            note: String::new(),
        };
        upsert_technicals(&c, "NVDA", &snap).unwrap();
        let got = get_technicals(&c, "nvda").unwrap().unwrap();
        assert_eq!(got.indicators.len(), 1);
        assert_eq!(got.trend_summary, "bullish composite");
    }

    #[test]
    fn vol_skew_roundtrip() {
        let c = open_mem_conn_v9();
        let snap = VolatilitySkew {
            symbol: "SPY".into(),
            as_of: "2026-04-14".into(),
            underlying_price: 520.0,
            expiries: vec![SkewExpiry {
                expiration: "2026-05-16".into(),
                days_to_expiry: 32,
                atm_iv_pct: 18.5,
                points: vec![SkewPoint {
                    strike: 520.0, moneyness_pct: 0.0,
                    call_iv_pct: 18.3, put_iv_pct: 18.7, combined_iv_pct: 18.5,
                }],
                put_call_skew_25d_pct: 2.1,
                term_note: String::new(),
            }],
            note: String::new(),
        };
        upsert_vol_skew(&c, "SPY", &snap).unwrap();
        let got = get_vol_skew(&c, "spy").unwrap().unwrap();
        assert_eq!(got.expiries.len(), 1);
        assert_eq!(got.expiries[0].points.len(), 1);
    }

    #[test]
    fn compute_seasonality_on_monthly_uptrend() {
        // 2 full years × 12 months × 21 bars = 504 bars.
        // Deterministic upward drift so every month is positive.
        let bars = synth_bars(504, 100.0, 0.001);
        let snap = compute_seasonality_snapshot("TEST", "2026-04-14", &bars);
        assert_eq!(snap.symbol, "TEST");
        assert!(snap.years_covered >= 2);
        assert!(snap.months.iter().any(|m| m.total_years > 0));
        // With uniform positive drift the best month should have a positive mean.
        let best = snap.months.iter().max_by(|a, b| a.avg_return_pct.partial_cmp(&b.avg_return_pct).unwrap()).unwrap();
        assert!(best.avg_return_pct > 0.0);
    }

    #[test]
    fn compute_seasonality_on_empty_returns_note() {
        let snap = compute_seasonality_snapshot("X", "2026-04-14", &[]);
        assert!(!snap.note.is_empty());
        assert_eq!(snap.years_covered, 0);
    }

    #[test]
    fn compute_correlation_matrix_perfect_copy() {
        // Bars need variable returns — constant drift produces zero variance
        // and an undefined ρ (our compute treats this as 0).
        let mut bars: Vec<HistoricalPriceRow> = Vec::new();
        let mut px = 100.0;
        for i in 0..300 {
            let base_day = 1 + (i % 28);
            let month    = 1 + ((i / 28) % 12);
            let year     = 2024 + (i / (28 * 12));
            let drift = if i % 2 == 0 { 0.005 } else { -0.003 };
            bars.push(HistoricalPriceRow {
                date: format!("{:04}-{:02}-{:02}", year, month, base_day),
                open: px, high: px * 1.01, low: px * 0.99,
                close: px, adj_close: px,
                volume: 1_000.0, change: 0.0, change_pct: 0.0,
            });
            px *= 1.0 + drift;
        }
        let peer = bars.clone();
        let snap = compute_correlation_matrix("A", "2026-04-14", 252,
            &bars, &[("B".into(), peer)]);
        assert_eq!(snap.cells.len(), 1);
        // Perfect copy ⇒ correlation ≈ 1.0 (allow numerical slack).
        assert!(snap.cells[0].correlation > 0.999,
            "expected ρ≈1.0, got {}", snap.cells[0].correlation);
        assert!((snap.cells[0].beta_vs_peer - 1.0).abs() < 1e-6);
    }

    #[test]
    fn compute_correlation_matrix_skips_empty_peers() {
        let bars = synth_bars(300, 100.0, 0.001);
        let snap = compute_correlation_matrix("A", "2026-04-14", 252,
            &bars, &[("NO_DATA".into(), vec![])]);
        assert!(!snap.note.is_empty() || snap.cells.is_empty());
    }

    #[test]
    fn compute_total_return_with_dividends_sums_windows() {
        // synth_bars(260, ...) spans 2024-01-01 through roughly 2024-10-08, so
        // dividend ex-dates must live inside that window to be counted.
        let bars = synth_bars(260, 100.0, 0.0004);
        let divs: Vec<DividendRecord> = vec![
            DividendRecord { ex_date: "2024-03-15".into(), amount: 0.5, ..Default::default() },
            DividendRecord { ex_date: "2024-06-15".into(), amount: 0.5, ..Default::default() },
            DividendRecord { ex_date: "2024-09-15".into(), amount: 0.5, ..Default::default() },
        ];
        let snap = compute_total_return_snapshot("TEST", "2024-10-15", &bars, &divs);
        assert!(snap.windows.iter().any(|w| w.label == "1Y"));
        // At least one window should record some dividends paid.
        assert!(snap.windows.iter().any(|w| w.dividends_paid > 0.0));
    }

    #[test]
    fn compute_technical_indicators_on_uptrend_is_bullish() {
        let bars = synth_bars(120, 100.0, 0.002);
        let snap = compute_technical_indicators("TEST", "2026-04-14", &bars);
        assert!(!snap.indicators.is_empty());
        // RSI on a steady uptrend should bias above 50 (often into overbought).
        let rsi = snap.indicators.iter().find(|i| i.name.starts_with("RSI")).unwrap();
        assert!(rsi.value > 50.0, "expected RSI > 50 on uptrend, got {:.2}", rsi.value);
    }

    #[test]
    fn compute_technical_indicators_insufficient_bars_returns_note() {
        let bars = synth_bars(10, 100.0, 0.001);
        let snap = compute_technical_indicators("X", "2026-04-14", &bars);
        assert!(!snap.note.is_empty());
        assert!(snap.indicators.is_empty());
    }

    #[test]
    fn compute_volatility_skew_basic_smile() {
        let chain = OptionsChainSnapshot {
            symbol: "SPY".into(),
            as_of: "2026-04-14".into(),
            underlying_price: 500.0,
            expirations: vec![OptionExpiry {
                expiration: "2026-05-16".into(),
                days_to_expiry: 32,
                calls: vec![
                    OptionContract { strike: 450.0, option_type: "CALL".into(), implied_volatility: 0.23, in_the_money: true, ..Default::default() },
                    OptionContract { strike: 500.0, option_type: "CALL".into(), implied_volatility: 0.18, in_the_money: false, ..Default::default() },
                    OptionContract { strike: 550.0, option_type: "CALL".into(), implied_volatility: 0.21, in_the_money: false, ..Default::default() },
                ],
                puts: vec![
                    OptionContract { strike: 450.0, option_type: "PUT".into(), implied_volatility: 0.25, in_the_money: false, ..Default::default() },
                    OptionContract { strike: 500.0, option_type: "PUT".into(), implied_volatility: 0.19, in_the_money: false, ..Default::default() },
                    OptionContract { strike: 550.0, option_type: "PUT".into(), implied_volatility: 0.20, in_the_money: true, ..Default::default() },
                ],
            }],
            note: String::new(),
        };
        let snap = compute_volatility_skew("SPY", "2026-04-14", &chain);
        assert_eq!(snap.expiries.len(), 1);
        let e = &snap.expiries[0];
        assert_eq!(e.points.len(), 3);
        // ATM (500) IV should be lowest (smile).
        assert!(e.atm_iv_pct > 0.0);
        // OTM put (450) IV 25% > OTM call (550) IV 21% → positive skew.
        assert!(e.put_call_skew_25d_pct > 0.0, "expected positive skew, got {}", e.put_call_skew_25d_pct);
    }

    #[test]
    fn compute_volatility_skew_empty_chain_returns_note() {
        let chain = OptionsChainSnapshot {
            symbol: "X".into(), as_of: "2026-04-14".into(),
            underlying_price: 100.0, expirations: Vec::new(), note: String::new(),
        };
        let snap = compute_volatility_skew("X", "2026-04-14", &chain);
        assert!(!snap.note.is_empty());
    }

    // ── ADR-117 Round 10 tests ──────────────────────────────────────────

    fn open_mem_conn_v10() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v10(&c).unwrap();
        c
    }

    fn sample_statements() -> FinancialStatements {
        // Build 4 quarters of synthetic financials with positive EBITDA + FCF.
        let q = |d: &str, p: &str, ni: f64, ebitda: f64, int_exp: f64, fcf: f64| (
            IncomeStatement {
                date: d.into(), period: p.into(),
                revenue: ni * 10.0, cost_of_revenue: ni * 5.0, gross_profit: ni * 5.0,
                research_and_development: ni * 0.5, selling_general_admin: ni * 1.0,
                operating_expenses: ni * 1.5, operating_income: ni * 2.0,
                interest_expense: int_exp, ebitda, income_before_tax: ni * 1.2,
                income_tax_expense: ni * 0.2, net_income: ni, eps: ni / 1000.0,
                eps_diluted: ni / 1000.0, weighted_shares_out: 1000.0,
            },
            CashFlowStatement {
                date: d.into(), period: p.into(),
                net_income: ni, depreciation_amortization: ebitda - ni * 2.0,
                stock_based_comp: 0.0, change_working_capital: 0.0,
                cash_from_operations: fcf + 50.0, capex: -50.0,
                acquisitions: 0.0, investments_purchases: 0.0,
                cash_from_investing: -50.0, debt_repayment: 0.0,
                dividends_paid: -20.0, stock_repurchases: 0.0,
                cash_from_financing: -20.0, net_change_cash: 10.0,
                free_cash_flow: fcf,
            }
        );
        let periods = [
            ("2024-12-31", "Q4", 300.0, 500.0, 30.0, 280.0),
            ("2024-09-30", "Q3", 280.0, 480.0, 30.0, 260.0),
            ("2024-06-30", "Q2", 260.0, 460.0, 30.0, 240.0),
            ("2024-03-31", "Q1", 240.0, 440.0, 30.0, 220.0),
        ];
        let mut income_q = Vec::new();
        let mut cf_q = Vec::new();
        for (d, p, ni, ebitda, int_exp, fcf) in periods.iter() {
            let (i, c) = q(d, p, *ni, *ebitda, *int_exp, *fcf);
            income_q.push(i);
            cf_q.push(c);
        }
        let bal = BalanceSheet {
            date: "2024-12-31".into(), period: "FY".into(),
            cash_and_equiv: 500.0, short_term_investments: 0.0,
            net_receivables: 100.0, inventory: 200.0,
            total_current_assets: 800.0,
            property_plant_equipment: 1000.0, goodwill: 0.0, intangible_assets: 0.0,
            long_term_investments: 0.0, total_non_current_assets: 1000.0,
            total_assets: 1800.0,
            accounts_payable: 150.0, short_term_debt: 100.0,
            total_current_liabilities: 400.0, long_term_debt: 600.0,
            total_non_current_liabilities: 800.0, total_liabilities: 1200.0,
            common_stock: 200.0, retained_earnings: 400.0, total_equity: 600.0,
            total_debt: 700.0, net_debt: 200.0,
        };
        FinancialStatements {
            income_annual: vec![income_q[0].clone()],
            income_quarterly: income_q,
            balance_annual: vec![bal.clone()],
            balance_quarterly: vec![bal],
            cashflow_annual: vec![cf_q[0].clone()],
            cashflow_quarterly: cf_q,
        }
    }

    #[test]
    fn leverage_snapshot_roundtrip() {
        let c = open_mem_conn_v10();
        let snap = LeverageSnapshot {
            symbol: "AAPL".into(),
            as_of: "2026-04-14".into(),
            total_debt: 700.0, net_debt: 200.0,
            ebitda_ttm: 1880.0, interest_expense_ttm: 120.0,
            total_equity: 600.0,
            ratios: vec![LeverageRatio {
                name: "Debt / EBITDA".into(), value: 0.37,
                peer_median: 0.0, signal: "HEALTHY".into(),
                note: "".into(),
            }],
            solvency_summary: "HEALTHY".into(),
            note: "".into(),
        };
        upsert_leverage(&c, "AAPL", &snap).unwrap();
        let got = get_leverage(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.symbol, "AAPL");
        assert_eq!(got.ratios.len(), 1);
        assert_eq!(got.solvency_summary, "HEALTHY");
    }

    #[test]
    fn accruals_snapshot_roundtrip() {
        let c = open_mem_conn_v10();
        let snap = AccrualsSnapshot {
            symbol: "MSFT".into(), as_of: "2026-04-14".into(),
            ttm_net_income: 1000.0, ttm_free_cash_flow: 900.0,
            ttm_cash_conversion_pct: 90.0, avg_cash_conversion_pct: 85.0,
            periods: vec![], trend_label: "STABLE".into(), note: "".into(),
        };
        upsert_accruals(&c, "MSFT", &snap).unwrap();
        let got = get_accruals(&c, "msft").unwrap().unwrap();
        assert_eq!(got.ttm_cash_conversion_pct, 90.0);
    }

    #[test]
    fn realized_vol_snapshot_roundtrip() {
        let c = open_mem_conn_v10();
        let snap = RealizedVolSnapshot {
            symbol: "NVDA".into(), as_of: "2026-04-14".into(),
            last_close: 900.0, current_atm_iv_pct: 45.0,
            iv_rv_gap_pct: 10.0, iv_rv_ratio: 1.28,
            windows: vec![RealizedVolWindow {
                label: "20d".into(), trading_days: 20,
                realized_vol_pct: 35.0, percentile: 60.0, n_observations: 100,
            }],
            regime_label: "RICH_IV".into(), note: "".into(),
        };
        upsert_realized_vol(&c, "NVDA", &snap).unwrap();
        let got = get_realized_vol(&c, "nvda").unwrap().unwrap();
        assert_eq!(got.windows.len(), 1);
        assert_eq!(got.regime_label, "RICH_IV");
    }

    #[test]
    fn fcf_yield_snapshot_roundtrip() {
        let c = open_mem_conn_v10();
        let snap = FcfYieldSnapshot {
            symbol: "KO".into(), as_of: "2026-04-14".into(),
            market_cap: 300_000_000_000.0, ttm_free_cash_flow: 10_000_000_000.0,
            ttm_dividends_paid: 8_000_000_000.0, ttm_fcf_yield_pct: 3.33,
            ttm_dividend_yield_pct: 2.67, ttm_payout_from_fcf_pct: 80.0,
            ttm_payout_from_ni_pct: 70.0, fcf_cagr_5y_pct: 5.2,
            periods: vec![], sustainability_label: "STRETCHED".into(),
            note: "".into(),
        };
        upsert_fcf_yield(&c, "KO", &snap).unwrap();
        let got = get_fcf_yield(&c, "ko").unwrap().unwrap();
        assert_eq!(got.sustainability_label, "STRETCHED");
    }

    #[test]
    fn short_interest_snapshot_roundtrip() {
        let c = open_mem_conn_v10();
        let snap = ShortInterestSnapshot {
            symbol: "GME".into(), as_of: "2026-04-14".into(),
            shares_outstanding: 300_000_000.0, shares_float: 200_000_000.0,
            short_shares: 50_000_000.0, short_percent_of_float: 25.0,
            avg_daily_volume_20d: 5_000_000.0, days_to_cover: 10.0,
            short_ratio_reported: 0.0, utilization_proxy_pct: 25.0,
            squeeze_risk_label: "EXTREME".into(), note: "".into(),
        };
        upsert_short_interest(&c, "GME", &snap).unwrap();
        let got = get_short_interest(&c, "gme").unwrap().unwrap();
        assert_eq!(got.squeeze_risk_label, "EXTREME");
        assert_eq!(got.days_to_cover, 10.0);
    }

    #[test]
    fn compute_leverage_on_healthy_statements() {
        let st = sample_statements();
        let snap = compute_leverage_snapshot("TEST", "2026-04-14", &st, 700.0, 500.0);
        // TTM EBITDA = 500+480+460+440 = 1880 → Debt/EBITDA = 700/1880 ≈ 0.37.
        assert!(!snap.ratios.is_empty());
        let de = snap.ratios.iter().find(|r| r.name == "Debt / EBITDA").unwrap();
        assert!((de.value - 700.0 / 1880.0).abs() < 1e-6);
        assert_eq!(de.signal, "HEALTHY");
        assert_eq!(snap.ebitda_ttm, 1880.0);
    }

    #[test]
    fn compute_leverage_empty_statements_produces_note() {
        let st = FinancialStatements::default();
        let snap = compute_leverage_snapshot("X", "2026-04-14", &st, 0.0, 0.0);
        assert!(snap.ratios.is_empty());
        assert!(!snap.note.is_empty());
    }

    #[test]
    fn compute_accruals_high_conversion_labels_high() {
        let st = sample_statements();
        let snap = compute_accruals_snapshot("TEST", "2026-04-14", &st);
        // Q4: NI=300, FCF=280 → conv=93.3% → HIGH.
        assert_eq!(snap.periods.len(), 4);
        let latest = &snap.periods[0];
        assert_eq!(latest.quality_label, "HIGH");
        let ttm_ni: f64 = 300.0 + 280.0 + 260.0 + 240.0;
        assert!((snap.ttm_net_income - ttm_ni).abs() < 1e-6);
    }

    #[test]
    fn compute_accruals_insufficient_periods_labels_insufficient() {
        let mut st = FinancialStatements::default();
        st.income_quarterly.push(IncomeStatement {
            date: "2024-12-31".into(), period: "Q4".into(),
            net_income: 100.0, ..Default::default()
        });
        st.cashflow_quarterly.push(CashFlowStatement {
            date: "2024-12-31".into(), period: "Q4".into(),
            net_income: 100.0, free_cash_flow: 90.0,
            ..Default::default()
        });
        let snap = compute_accruals_snapshot("X", "2026-04-14", &st);
        assert_eq!(snap.periods.len(), 1);
        assert_eq!(snap.trend_label, "INSUFFICIENT");
    }

    #[test]
    fn compute_realized_vol_with_drift_produces_rich_regime() {
        let bars = synth_bars(260, 100.0, 0.001);
        let snap = compute_realized_vol_snapshot("TEST", "2026-04-14", &bars, 40.0);
        assert!(!snap.windows.is_empty());
        assert!(snap.windows.iter().any(|w| w.label == "20d"));
        // Constant drift → near-zero RV → IV/RV should flag RICH_IV or NO_IV_REFERENCE.
        assert!(snap.regime_label == "RICH_IV" || snap.regime_label == "NO_IV_REFERENCE");
    }

    #[test]
    fn compute_realized_vol_insufficient_bars_returns_note() {
        let bars = synth_bars(10, 100.0, 0.001);
        let snap = compute_realized_vol_snapshot("X", "2026-04-14", &bars, 40.0);
        assert_eq!(snap.regime_label, "INSUFFICIENT_DATA");
        assert!(!snap.note.is_empty());
    }

    #[test]
    fn compute_fcf_yield_with_market_cap() {
        let st = sample_statements();
        let snap = compute_fcf_yield_snapshot("TEST", "2026-04-14", &st, 100_000.0, 100.0);
        // TTM FCF = 280+260+240+220 = 1000; yield = 1000/100000 = 1.0%
        assert!((snap.ttm_free_cash_flow - 1000.0).abs() < 1e-6);
        assert!((snap.ttm_fcf_yield_pct - 1.0).abs() < 1e-6);
        // TTM dividends paid = 20*4 = 80, payout_fcf = 80/1000 = 8%, label SAFE.
        assert_eq!(snap.sustainability_label, "SAFE");
    }

    #[test]
    fn compute_fcf_yield_no_market_cap_emits_note() {
        let st = sample_statements();
        let snap = compute_fcf_yield_snapshot("X", "2026-04-14", &st, 0.0, 100.0);
        assert!(!snap.note.is_empty());
    }

    #[test]
    fn compute_short_interest_high_risk_squeeze() {
        let bars = synth_bars(30, 100.0, 0.0);
        // 200M float × 25% = 50M short; 50M / 1K avg = 50K days-to-cover → EXTREME.
        let snap = compute_short_interest_snapshot(
            "GME", "2026-04-14",
            300_000_000.0, 200_000_000.0, 25.0, 0.0, &bars,
        );
        assert_eq!(snap.short_shares, 50_000_000.0);
        assert_eq!(snap.squeeze_risk_label, "EXTREME");
    }

    #[test]
    fn compute_short_interest_no_shorts_insufficient() {
        let bars = synth_bars(30, 100.0, 0.0);
        let snap = compute_short_interest_snapshot(
            "X", "2026-04-14",
            100_000_000.0, 80_000_000.0, 0.0, 0.0, &bars,
        );
        assert_eq!(snap.squeeze_risk_label, "INSUFFICIENT_DATA");
    }

    // ── ADR-118 Godel Parity Round 11 tests ─────────────────────────────

    fn open_mem_conn_v11() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v11(&c).unwrap();
        c
    }

    fn two_year_statements() -> FinancialStatements {
        // Build two annual snapshots so Piotroski and Altman have enough data.
        let inc_current = IncomeStatement {
            date: "2024-12-31".into(), period: "FY".into(),
            revenue: 2000.0, cost_of_revenue: 1000.0, gross_profit: 1000.0,
            research_and_development: 100.0, selling_general_admin: 200.0,
            operating_expenses: 300.0, operating_income: 700.0,
            interest_expense: 50.0, ebitda: 800.0,
            income_before_tax: 650.0, income_tax_expense: 150.0,
            net_income: 500.0, eps: 5.0, eps_diluted: 5.0,
            weighted_shares_out: 100.0,
        };
        let inc_prior = IncomeStatement {
            date: "2023-12-31".into(), period: "FY".into(),
            revenue: 1800.0, cost_of_revenue: 1000.0, gross_profit: 800.0,
            research_and_development: 100.0, selling_general_admin: 200.0,
            operating_expenses: 300.0, operating_income: 500.0,
            interest_expense: 50.0, ebitda: 600.0,
            income_before_tax: 450.0, income_tax_expense: 100.0,
            net_income: 350.0, eps: 3.5, eps_diluted: 3.5,
            weighted_shares_out: 100.0,
        };
        let bal_current = BalanceSheet {
            date: "2024-12-31".into(), period: "FY".into(),
            cash_and_equiv: 400.0, short_term_investments: 0.0,
            net_receivables: 200.0, inventory: 300.0,
            total_current_assets: 900.0,
            property_plant_equipment: 1500.0, goodwill: 0.0, intangible_assets: 0.0,
            long_term_investments: 0.0, total_non_current_assets: 1500.0,
            total_assets: 2400.0,
            accounts_payable: 150.0, short_term_debt: 100.0,
            total_current_liabilities: 400.0, long_term_debt: 500.0,
            total_non_current_liabilities: 700.0, total_liabilities: 1100.0,
            common_stock: 300.0, retained_earnings: 1000.0, total_equity: 1300.0,
            total_debt: 600.0, net_debt: 200.0,
        };
        let bal_prior = BalanceSheet {
            date: "2023-12-31".into(), period: "FY".into(),
            cash_and_equiv: 300.0, short_term_investments: 0.0,
            net_receivables: 180.0, inventory: 280.0,
            total_current_assets: 760.0,
            property_plant_equipment: 1400.0, goodwill: 0.0, intangible_assets: 0.0,
            long_term_investments: 0.0, total_non_current_assets: 1400.0,
            total_assets: 2160.0,
            accounts_payable: 150.0, short_term_debt: 100.0,
            total_current_liabilities: 400.0, long_term_debt: 600.0,
            total_non_current_liabilities: 800.0, total_liabilities: 1200.0,
            common_stock: 300.0, retained_earnings: 660.0, total_equity: 960.0,
            total_debt: 700.0, net_debt: 400.0,
        };
        let cf_current = CashFlowStatement {
            date: "2024-12-31".into(), period: "FY".into(),
            net_income: 500.0, depreciation_amortization: 100.0,
            stock_based_comp: 0.0, change_working_capital: 0.0,
            cash_from_operations: 600.0, capex: -200.0,
            acquisitions: 0.0, investments_purchases: 0.0,
            cash_from_investing: -200.0, debt_repayment: -100.0,
            dividends_paid: 0.0, stock_repurchases: 0.0,
            cash_from_financing: -100.0, net_change_cash: 300.0,
            free_cash_flow: 400.0,
        };
        let cf_prior = CashFlowStatement {
            date: "2023-12-31".into(), period: "FY".into(),
            net_income: 350.0, depreciation_amortization: 90.0,
            stock_based_comp: 0.0, change_working_capital: 0.0,
            cash_from_operations: 420.0, capex: -180.0,
            acquisitions: 0.0, investments_purchases: 0.0,
            cash_from_investing: -180.0, debt_repayment: 0.0,
            dividends_paid: 0.0, stock_repurchases: 0.0,
            cash_from_financing: 0.0, net_change_cash: 240.0,
            free_cash_flow: 240.0,
        };
        FinancialStatements {
            income_annual: vec![inc_current.clone(), inc_prior.clone()],
            income_quarterly: vec![inc_current],
            balance_annual: vec![bal_current.clone(), bal_prior.clone()],
            balance_quarterly: vec![bal_current],
            cashflow_annual: vec![cf_current.clone(), cf_prior.clone()],
            cashflow_quarterly: vec![cf_current],
        }
    }

    #[test]
    fn altman_z_snapshot_roundtrip() {
        let c = open_mem_conn_v11();
        let snap = AltmanZSnapshot {
            symbol: "AAPL".into(),
            as_of: "2026-04-14".into(),
            working_capital: 500.0, retained_earnings: 1000.0,
            ebit: 700.0, market_value_equity: 5000.0,
            sales: 2000.0, total_assets: 2400.0, total_liabilities: 1100.0,
            z_score: 4.2, zone: "SAFE".into(),
            components: vec![AltmanComponent {
                name: "A: WC/TA".into(), ratio: 0.208, coefficient: 1.2,
                contribution: 0.25, note: "".into(),
            }],
            note: "".into(),
        };
        upsert_altman_z(&c, "AAPL", &snap).unwrap();
        let got = get_altman_z(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.symbol, "AAPL");
        assert_eq!(got.zone, "SAFE");
        assert_eq!(got.components.len(), 1);
    }

    #[test]
    fn piotroski_snapshot_roundtrip() {
        let c = open_mem_conn_v11();
        let snap = PiotroskiSnapshot {
            symbol: "MSFT".into(), as_of: "2026-04-14".into(),
            current_period: "2024-12-31".into(), prior_period: "2023-12-31".into(),
            f_score: 8, strength_label: "STRONG".into(),
            profitability_score: 4, leverage_score: 2, efficiency_score: 2,
            checks: vec![PiotroskiCheck {
                category: "Profitability".into(),
                name: "Positive Net Income".into(),
                passed: true, value_current: 500.0, value_prior: 350.0,
                note: "".into(),
            }],
            note: "".into(),
        };
        upsert_piotroski(&c, "MSFT", &snap).unwrap();
        let got = get_piotroski(&c, "msft").unwrap().unwrap();
        assert_eq!(got.f_score, 8);
        assert_eq!(got.strength_label, "STRONG");
    }

    #[test]
    fn ohlc_vol_snapshot_roundtrip() {
        let c = open_mem_conn_v11();
        let snap = OhlcVolSnapshot {
            symbol: "NVDA".into(), as_of: "2026-04-14".into(),
            trading_days: 60,
            estimators: vec![VolEstimator {
                name: "YangZhang".into(), annualized_vol_pct: 35.0,
                efficiency_vs_close: 1.1, note: "".into(),
            }],
            preferred_estimate_pct: 35.0, preferred_label: "YangZhang".into(),
            note: "".into(),
        };
        upsert_ohlc_vol(&c, "NVDA", &snap).unwrap();
        let got = get_ohlc_vol(&c, "nvda").unwrap().unwrap();
        assert_eq!(got.preferred_label, "YangZhang");
        assert_eq!(got.estimators.len(), 1);
    }

    #[test]
    fn eps_beat_snapshot_roundtrip() {
        let c = open_mem_conn_v11();
        let snap = EpsBeatSnapshot {
            symbol: "AMZN".into(), as_of: "2026-04-14".into(),
            total_reports: 8, beats: 6, misses: 1, inlines: 1,
            beat_rate_pct: 75.0, current_streak: 3,
            longest_beat_streak: 4, longest_miss_streak: 1,
            avg_surprise_pct: 5.2, median_surprise_pct: 4.8,
            recent_avg_surprise_pct: 6.5,
            bias_label: "POSITIVE".into(), trend_label: "ACCELERATING".into(),
            latest_date: "2024-10-31".into(), latest_surprise_pct: 7.0,
            note: "".into(),
        };
        upsert_eps_beat(&c, "AMZN", &snap).unwrap();
        let got = get_eps_beat(&c, "amzn").unwrap().unwrap();
        assert_eq!(got.beats, 6);
        assert_eq!(got.trend_label, "ACCELERATING");
    }

    #[test]
    fn price_target_dispersion_roundtrip() {
        let c = open_mem_conn_v11();
        let snap = PriceTargetDispersion {
            symbol: "TSLA".into(), as_of: "2026-04-14".into(),
            current_price: 200.0,
            target_high: 350.0, target_low: 150.0,
            target_mean: 250.0, target_median: 240.0,
            num_analysts: 25,
            dispersion_pct: 80.0, spread_pct: 100.0,
            implied_return_median_pct: 20.0, implied_return_mean_pct: 25.0,
            upside_to_high_pct: 75.0, downside_to_low_pct: -25.0,
            consensus_label: "BULLISH".into(), note: "".into(),
        };
        upsert_price_target_dispersion(&c, "TSLA", &snap).unwrap();
        let got = get_price_target_dispersion(&c, "tsla").unwrap().unwrap();
        assert_eq!(got.consensus_label, "BULLISH");
        assert_eq!(got.num_analysts, 25);
    }

    #[test]
    fn compute_altman_z_on_healthy_statements() {
        let st = two_year_statements();
        let snap = compute_altman_z_snapshot("TEST", "2026-04-14", &st, 5000.0);
        // WC = 900-400 = 500, TA = 2400, RE = 1000, EBIT = 700, MVE = 5000, TL = 1100, Sales = 2000
        // Z = 1.2*(500/2400) + 1.4*(1000/2400) + 3.3*(700/2400) + 0.6*(5000/1100) + 1.0*(2000/2400)
        //   ≈ 0.25 + 0.583 + 0.963 + 2.727 + 0.833 ≈ 5.36 → SAFE
        assert_eq!(snap.components.len(), 5);
        assert!(snap.z_score > 2.99);
        assert_eq!(snap.zone, "SAFE");
        assert_eq!(snap.total_assets, 2400.0);
    }

    #[test]
    fn compute_altman_z_insufficient_data_returns_note() {
        let st = FinancialStatements::default();
        let snap = compute_altman_z_snapshot("X", "2026-04-14", &st, 1000.0);
        assert_eq!(snap.zone, "INSUFFICIENT_DATA");
        assert!(!snap.note.is_empty());
    }

    #[test]
    fn compute_piotroski_strong_score() {
        let st = two_year_statements();
        let snap = compute_piotroski_snapshot("TEST", "2026-04-14", &st);
        // Improving NI (350→500), positive OCF (600), OCF>NI, LTDebt↓ (600→500),
        // current ratio ≈ 900/400 vs 760/400 → improved, no new shares, GM ≈ 50% vs 44% → improved,
        // asset turnover ≈ 2000/2400 vs 1800/2160 → similar, expect STRONG (≥7).
        assert!(snap.f_score >= 7);
        assert_eq!(snap.strength_label, "STRONG");
        assert_eq!(snap.checks.len(), 9);
    }

    #[test]
    fn compute_piotroski_insufficient_data() {
        let st = FinancialStatements::default();
        let snap = compute_piotroski_snapshot("X", "2026-04-14", &st);
        assert_eq!(snap.strength_label, "INSUFFICIENT_DATA");
        assert!(!snap.note.is_empty());
    }

    #[test]
    fn compute_ohlc_vol_five_estimators() {
        let bars = synth_bars(60, 100.0, 0.001);
        let snap = compute_ohlc_vol_snapshot("TEST", "2026-04-14", &bars, 30);
        assert_eq!(snap.estimators.len(), 5);
        assert!(snap.preferred_estimate_pct >= 0.0);
        assert_eq!(snap.preferred_label, "Yang-Zhang");
    }

    #[test]
    fn compute_ohlc_vol_insufficient_bars() {
        let bars = synth_bars(10, 100.0, 0.001);
        let snap = compute_ohlc_vol_snapshot("X", "2026-04-14", &bars, 20);
        assert!(!snap.note.is_empty());
        assert!(snap.estimators.is_empty());
    }

    #[test]
    fn compute_eps_beat_six_beats_labels_positive() {
        let rows: Vec<EarningsSurprise> = (0..8).map(|i| EarningsSurprise {
            date: format!("2024-{:02}-01", i + 1),
            symbol: "TEST".into(),
            eps_actual: 1.0 + (i as f64) * 0.05,
            eps_estimate: 1.0,
            surprise: (i as f64) * 0.05,
            surprise_pct: (i as f64) * 5.0,
        }).collect();
        let snap = compute_eps_beat_snapshot("TEST", "2026-04-14", &rows);
        assert_eq!(snap.total_reports, 8);
        assert!(snap.beats >= 6);
        assert_eq!(snap.bias_label, "POSITIVE");
        assert!(snap.current_streak > 0);
    }

    #[test]
    fn compute_eps_beat_empty_reports() {
        let snap = compute_eps_beat_snapshot("X", "2026-04-14", &[]);
        assert_eq!(snap.total_reports, 0);
        assert!(!snap.note.is_empty());
    }

    #[test]
    fn compute_price_target_dispersion_bullish() {
        let target = PriceTarget {
            symbol: "TEST".into(),
            target_high: 150.0, target_low: 110.0,
            target_mean: 130.0, target_median: 125.0,
            last_updated: "2024-11-01".into(),
            num_analysts: 15,
        };
        let snap = compute_price_target_dispersion("TEST", "2026-04-14", 100.0, Some(&target));
        // implied_median = (125-100)/100 = 25% → BULLISH
        assert_eq!(snap.consensus_label, "BULLISH");
        assert!((snap.implied_return_median_pct - 25.0).abs() < 1e-6);
        assert!((snap.spread_pct - 40.0).abs() < 1e-6);
    }

    #[test]
    fn compute_price_target_dispersion_no_coverage() {
        let snap = compute_price_target_dispersion("X", "2026-04-14", 100.0, None);
        assert_eq!(snap.consensus_label, "NO_COVERAGE");
        assert!(!snap.note.is_empty());
    }

    // ── ADR-119 Godel Parity Round 12 tests ────────────────────────────────

    fn open_mem_conn_v12() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v12(&c).unwrap();
        c
    }

    fn trade(date: &str, name: &str, ttype: &str, disp: &str, shares: f64, price: f64) -> InsiderTrade {
        InsiderTrade {
            filing_date: date.to_string(),
            transaction_date: date.to_string(),
            reporting_name: name.to_string(),
            transaction_type: ttype.to_string(),
            acquisition_disposition: disp.to_string(),
            shares,
            price,
            value_usd: shares * price,
            shares_owned_after: 0.0,
            link: String::new(),
        }
    }

    #[test]
    fn insider_activity_snapshot_roundtrip() {
        let c = open_mem_conn_v12();
        let snap = InsiderActivitySnapshot {
            symbol: "AAPL".into(),
            as_of: "2026-04-14".into(),
            window_days: 90,
            total_trades: 5,
            buy_count: 3,
            sell_count: 2,
            other_count: 0,
            unique_insiders: 2,
            gross_buy_value_usd: 1_000_000.0,
            gross_sell_value_usd: 400_000.0,
            net_value_usd: 600_000.0,
            buy_sell_ratio: 1.5,
            net_shares: 5_000.0,
            latest_trade_date: "2026-04-10".into(),
            bias_label: "BULLISH".into(),
            conviction_label: "MEDIUM".into(),
            note: String::new(),
        };
        upsert_insider_activity(&c, "AAPL", &snap).unwrap();
        let got = get_insider_activity(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.bias_label, "BULLISH");
        assert_eq!(got.buy_count, 3);
        assert!((got.buy_sell_ratio - 1.5).abs() < 1e-6);
    }

    #[test]
    fn compute_insider_activity_bullish_net_buys() {
        let trades = vec![
            trade("2026-04-10", "Alice CEO", "P-Purchase", "A", 1000.0, 150.0),
            trade("2026-04-05", "Bob CFO",   "P-Purchase", "A", 2000.0, 148.0),
            trade("2026-03-20", "Alice CEO", "P-Purchase", "A",  500.0, 145.0),
            trade("2026-03-10", "Carol COO", "S-Sale",     "D", 500.0, 160.0),
        ];
        let snap = compute_insider_activity_snapshot("AAPL", "2026-04-14", &trades, 90);
        assert_eq!(snap.bias_label, "BULLISH");
        assert_eq!(snap.buy_count, 3);
        assert_eq!(snap.sell_count, 1);
        assert_eq!(snap.unique_insiders, 3);
        assert!(snap.gross_buy_value_usd > snap.gross_sell_value_usd);
        assert!(snap.net_value_usd > 0.0);
        assert_eq!(snap.latest_trade_date, "2026-04-10");
    }

    #[test]
    fn compute_insider_activity_bearish_net_sales() {
        let trades = vec![
            trade("2026-04-10", "Alice CEO", "S-Sale", "D", 10_000.0, 150.0),
            trade("2026-04-05", "Bob CFO",   "S-Sale", "D",  5_000.0, 148.0),
            trade("2026-03-10", "Alice CEO", "P-Purchase", "A", 100.0, 145.0),
        ];
        let snap = compute_insider_activity_snapshot("AAPL", "2026-04-14", &trades, 90);
        assert_eq!(snap.bias_label, "BEARISH");
        assert_eq!(snap.sell_count, 2);
        assert!(snap.net_value_usd < 0.0);
    }

    #[test]
    fn compute_insider_activity_no_activity() {
        let snap = compute_insider_activity_snapshot("X", "2026-04-14", &[], 90);
        assert_eq!(snap.bias_label, "NO_ACTIVITY");
        assert_eq!(snap.conviction_label, "NONE");
        assert!(!snap.note.is_empty());
    }

    #[test]
    fn insider_activity_respects_lookback_window() {
        let trades = vec![
            trade("2026-04-01", "Alice CEO", "P-Purchase", "A", 1000.0, 150.0),
            trade("2025-01-01", "Old CEO",   "P-Purchase", "A", 9999.0, 100.0),
        ];
        // 90-day window from 2026-04-14 should exclude 2025-01-01
        let snap = compute_insider_activity_snapshot("X", "2026-04-14", &trades, 90);
        assert_eq!(snap.total_trades, 1);
        assert_eq!(snap.buy_count, 1);
    }

    fn dvd(ex: &str, amt: f64) -> DividendRecord {
        DividendRecord { ex_date: ex.to_string(), amount: amt, ..Default::default() }
    }

    #[test]
    fn divg_snapshot_roundtrip() {
        let c = open_mem_conn_v12();
        let snap = DivgSnapshot {
            symbol: "KO".into(),
            as_of: "2026-04-14".into(),
            total_payments: 20,
            latest_amount: 0.49,
            annualized_dividend: 1.96,
            years_covered: 5,
            cagr_1y_pct: 5.0,
            cagr_3y_pct: 4.5,
            cagr_5y_pct: 4.0,
            trend_label: "GROWING".into(),
            ..Default::default()
        };
        upsert_divg(&c, "KO", &snap).unwrap();
        let got = get_divg(&c, "ko").unwrap().unwrap();
        assert_eq!(got.trend_label, "GROWING");
        assert!((got.cagr_1y_pct - 5.0).abs() < 1e-6);
    }

    #[test]
    fn compute_divg_growing_consistent() {
        // Five complete years of 4 payments each, 5 % YoY growth, last payment before as_of.
        let mut rows = Vec::new();
        for (y, base) in [(2020, 0.40), (2021, 0.42), (2022, 0.44), (2023, 0.46), (2024, 0.49)] {
            for q in ["03-15", "06-15", "09-15", "12-15"] {
                rows.push(dvd(&format!("{y}-{q}"), base));
            }
        }
        let snap = compute_divg_snapshot("KO", "2026-04-14", &rows);
        assert_eq!(snap.trend_label, "GROWING");
        assert!(snap.years_covered >= 5);
        assert!(snap.cagr_1y_pct > 0.0);
        assert!(snap.consecutive_growth_years >= 4);
        assert!(snap.consistency_score_pct >= 70.0);
    }

    #[test]
    fn compute_divg_cutting() {
        let rows = vec![
            dvd("2022-03-15", 0.80),
            dvd("2022-06-15", 0.80),
            dvd("2022-09-15", 0.80),
            dvd("2022-12-15", 0.80),
            dvd("2023-03-15", 0.70),
            dvd("2023-06-15", 0.60),
            dvd("2023-09-15", 0.50),
            dvd("2023-12-15", 0.40),
        ];
        let snap = compute_divg_snapshot("X", "2026-04-14", &rows);
        assert_eq!(snap.trend_label, "CUTTING");
        assert!(snap.cagr_1y_pct < 0.0);
    }

    #[test]
    fn compute_divg_no_history() {
        let snap = compute_divg_snapshot("X", "2026-04-14", &[]);
        assert_eq!(snap.trend_label, "NO_HISTORY");
    }

    fn inc(date: &str, rev: f64, eps: f64) -> IncomeStatement {
        IncomeStatement {
            date: date.to_string(),
            period: "Q".to_string(),
            revenue: rev,
            eps,
            ..Default::default()
        }
    }

    fn surp(date: &str, actual: f64, est: f64) -> EarningsSurprise {
        let s = actual - est;
        EarningsSurprise {
            date: date.to_string(),
            symbol: "X".into(),
            eps_actual: actual,
            eps_estimate: est,
            surprise: s,
            surprise_pct: if est.abs() > 0.0 { s / est.abs() * 100.0 } else { 0.0 },
        }
    }

    #[test]
    fn earm_snapshot_roundtrip() {
        let c = open_mem_conn_v12();
        let snap = EarmSnapshot {
            symbol: "NVDA".into(),
            as_of: "2026-04-14".into(),
            quarters_used: 8,
            recent_revenue_growth_pct: 40.0,
            prior_revenue_growth_pct: 25.0,
            revenue_acceleration_pct: 15.0,
            recent_eps_surprise_pct: 10.0,
            prior_eps_surprise_pct: 5.0,
            eps_surprise_acceleration_pct: 5.0,
            composite_score: 80.0,
            momentum_label: "ACCELERATING".into(),
            quarters: vec![EarmQuarterRow {
                period: "2026-01-31".into(),
                revenue: 22000.0,
                revenue_yoy_pct: 40.0,
                eps_actual: 5.0,
                eps_estimate: 4.5,
                eps_surprise_pct: 11.1,
            }],
            note: String::new(),
        };
        upsert_earm(&c, "NVDA", &snap).unwrap();
        let got = get_earm(&c, "nvda").unwrap().unwrap();
        assert_eq!(got.momentum_label, "ACCELERATING");
        assert!((got.composite_score - 80.0).abs() < 1e-6);
    }

    #[test]
    fn compute_earm_accelerating() {
        // 8 quarters, newest-first. Revenue grows yoy, recent pace faster than prior.
        let statements = FinancialStatements {
            income_quarterly: vec![
                inc("2026-03-31", 140.0, 2.40),   // 0  yoy vs 4 = +16.67%
                inc("2025-12-31", 135.0, 2.30),   // 1  yoy vs 5 = +17.39%
                inc("2025-09-30", 130.0, 2.20),   // 2  yoy vs 6 = +18.18%
                inc("2025-06-30", 125.0, 2.10),   // 3  yoy vs 7 = +19.05%
                inc("2025-03-31", 120.0, 2.00),   // 4
                inc("2024-12-31", 115.0, 1.90),   // 5
                inc("2024-09-30", 110.0, 1.80),   // 6
                inc("2024-06-30", 105.0, 1.75),   // 7
            ],
            ..Default::default()
        };
        let surprises = vec![
            surp("2026-03-31", 2.40, 2.30),
            surp("2025-12-31", 2.30, 2.20),
            surp("2025-09-30", 2.20, 2.15),
            surp("2025-06-30", 2.10, 2.08),
            surp("2025-03-31", 2.00, 1.99),
            surp("2024-12-31", 1.90, 1.90),
            surp("2024-09-30", 1.80, 1.81),
            surp("2024-06-30", 1.75, 1.77),
        ];
        let snap = compute_earm_snapshot("NVDA", "2026-04-14", &statements, &surprises);
        assert_eq!(snap.quarters_used, 8);
        assert!(snap.recent_revenue_growth_pct > 0.0);
        assert!(snap.composite_score > 0.0);
        assert!(matches!(snap.momentum_label.as_str(), "ACCELERATING" | "STABLE"));
    }

    #[test]
    fn compute_earm_insufficient_data() {
        let statements = FinancialStatements {
            income_quarterly: vec![inc("2026-03-31", 100.0, 1.0)],
            ..Default::default()
        };
        let snap = compute_earm_snapshot("X", "2026-04-14", &statements, &[]);
        assert_eq!(snap.momentum_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn sector_rotation_snapshot_roundtrip() {
        let c = open_mem_conn_v12();
        let snap = SectorRotationSnapshot {
            symbol: "AAPL".into(),
            as_of: "2026-04-14".into(),
            symbol_sector: "Technology".into(),
            symbol_sector_change_pct: 1.5,
            sector_rank: 1,
            sectors_total: 11,
            avg_sector_change_pct: 0.3,
            relative_strength_pct: 1.2,
            strength_label: "LEADER".into(),
            ..Default::default()
        };
        upsert_sector_rotation(&c, "AAPL", &snap).unwrap();
        let got = get_sector_rotation(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.strength_label, "LEADER");
        assert_eq!(got.sector_rank, 1);
    }

    #[test]
    fn compute_sector_rotation_leader() {
        let sectors = vec![
            SectorPerformance { sector: "Technology".into(),          change_pct: 2.0 },
            SectorPerformance { sector: "Healthcare".into(),          change_pct: 0.5 },
            SectorPerformance { sector: "Financial Services".into(),  change_pct: 0.1 },
            SectorPerformance { sector: "Energy".into(),              change_pct: -0.5 },
            SectorPerformance { sector: "Consumer Cyclical".into(),   change_pct: 1.0 },
            SectorPerformance { sector: "Utilities".into(),           change_pct: -1.0 },
        ];
        let snap = compute_sector_rotation_snapshot("AAPL", "2026-04-14", "Technology", &sectors);
        assert_eq!(snap.strength_label, "LEADER");
        assert_eq!(snap.sector_rank, 1);
        assert_eq!(snap.strongest_sector, "Technology");
        assert_eq!(snap.weakest_sector, "Utilities");
        assert!(snap.relative_strength_pct > 0.0);
    }

    #[test]
    fn compute_sector_rotation_laggard() {
        let sectors = vec![
            SectorPerformance { sector: "Technology".into(),  change_pct: 2.0 },
            SectorPerformance { sector: "Healthcare".into(),  change_pct: 1.5 },
            SectorPerformance { sector: "Financials".into(),  change_pct: 1.0 },
            SectorPerformance { sector: "Energy".into(),      change_pct: -1.5 },
        ];
        let snap = compute_sector_rotation_snapshot("XOM", "2026-04-14", "Energy", &sectors);
        assert_eq!(snap.strength_label, "LAGGARD");
        assert!(snap.relative_strength_pct < 0.0);
    }

    #[test]
    fn compute_sector_rotation_no_data() {
        let snap = compute_sector_rotation_snapshot("X", "2026-04-14", "Technology", &[]);
        assert_eq!(snap.strength_label, "NO_DATA");
    }

    fn rc(date: &str, action: &str, firm: &str, to: &str) -> RatingChange {
        RatingChange {
            date: date.to_string(),
            symbol: "X".into(),
            company: String::new(),
            firm: firm.to_string(),
            action: action.to_string(),
            from_grade: String::new(),
            to_grade: to.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn updm_snapshot_roundtrip() {
        let c = open_mem_conn_v12();
        let snap = UpdmSnapshot {
            symbol: "NVDA".into(),
            as_of: "2026-04-14".into(),
            total_actions: 8,
            upgrades_90d: 5,
            downgrades_90d: 2,
            net_90d: 3,
            latest_date: "2026-04-10".into(),
            latest_action: "upgrade".into(),
            bias_label: "BULLISH".into(),
            trend_label: "IMPROVING".into(),
            ..Default::default()
        };
        upsert_updm(&c, "NVDA", &snap).unwrap();
        let got = get_updm(&c, "nvda").unwrap().unwrap();
        assert_eq!(got.bias_label, "BULLISH");
        assert_eq!(got.net_90d, 3);
    }

    #[test]
    fn compute_updm_bullish_improving() {
        let actions = vec![
            rc("2026-04-10", "upgrade",   "Morgan Stanley", "Overweight"),
            rc("2026-04-05", "upgrade",   "Goldman Sachs",  "Buy"),
            rc("2026-03-28", "initiation","JPM",            "Overweight"),
            rc("2026-03-15", "downgrade", "Bernstein",      "Market-Perform"),
            rc("2026-02-20", "upgrade",   "Wells Fargo",    "Outperform"),
            rc("2025-12-10", "downgrade", "Citi",           "Neutral"),
        ];
        let snap = compute_updm_snapshot("NVDA", "2026-04-14", &actions);
        assert_eq!(snap.bias_label, "BULLISH");
        assert!(snap.net_90d > 0);
        assert_eq!(snap.latest_date, "2026-04-10");
        assert_eq!(snap.latest_action, "upgrade");
    }

    #[test]
    fn compute_updm_bearish() {
        let actions = vec![
            rc("2026-04-10", "downgrade", "Morgan Stanley", "Underweight"),
            rc("2026-04-01", "downgrade", "Goldman Sachs",  "Sell"),
            rc("2026-03-20", "downgrade", "Bernstein",      "Market-Perform"),
            rc("2026-03-10", "upgrade",   "Wells Fargo",    "Outperform"),
        ];
        let snap = compute_updm_snapshot("X", "2026-04-14", &actions);
        assert_eq!(snap.bias_label, "BEARISH");
        assert!(snap.net_90d < 0);
    }

    #[test]
    fn compute_updm_no_coverage() {
        let snap = compute_updm_snapshot("X", "2026-04-14", &[]);
        assert_eq!(snap.bias_label, "NO_COVERAGE");
    }

    // ── ADR-120 Godel Parity Round 13 tests ────────────────────────────────

    fn open_mem_conn_v13() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v13(&c).unwrap();
        c
    }

    fn make_bar(date: &str, close: f64, volume: f64) -> HistoricalPriceRow {
        HistoricalPriceRow {
            date: date.to_string(),
            open: close,
            high: close * 1.01,
            low: close * 0.99,
            close,
            adj_close: close,
            volume,
            change: 0.0,
            change_pct: 0.0,
        }
    }

    fn make_bar_flat(date: &str, close: f64, volume: f64) -> HistoricalPriceRow {
        HistoricalPriceRow {
            date: date.to_string(),
            open: close,
            high: close,
            low: close,
            close,
            adj_close: close,
            volume,
            change: 0.0,
            change_pct: 0.0,
        }
    }

    #[test]
    fn momentum_snapshot_roundtrip() {
        let c = open_mem_conn_v13();
        let snap = MomentumSnapshot {
            symbol: "X".to_string(),
            as_of: "2026-04-14".to_string(),
            bars_used: 252,
            return_12m_pct: 35.0,
            regime_label: "STRONG".to_string(),
            trend_label: "ACCELERATING".to_string(),
            ..Default::default()
        };
        upsert_momentum(&c, "x", &snap).unwrap();
        let got = get_momentum(&c, "X").unwrap().unwrap();
        assert_eq!(got.symbol, "X");
        assert_eq!(got.regime_label, "STRONG");
    }

    #[test]
    fn liquidity_snapshot_roundtrip() {
        let c = open_mem_conn_v13();
        let snap = LiquiditySnapshot {
            symbol: "Y".to_string(),
            as_of: "2026-04-14".to_string(),
            window_days: 60,
            avg_daily_dollar_volume: 1.5e9,
            liquidity_tier: "DEEP".to_string(),
            ..Default::default()
        };
        upsert_liquidity(&c, "y", &snap).unwrap();
        let got = get_liquidity(&c, "Y").unwrap().unwrap();
        assert_eq!(got.liquidity_tier, "DEEP");
    }

    #[test]
    fn breakout_snapshot_roundtrip() {
        let c = open_mem_conn_v13();
        let snap = BreakoutSnapshot {
            symbol: "Z".to_string(),
            as_of: "2026-04-14".to_string(),
            current_price: 100.0,
            high_52w: 100.0,
            low_52w: 60.0,
            position_in_52w_range_pct: 100.0,
            breakout_label: "NEW_HIGH".to_string(),
            setup_label: "TRENDING_UP".to_string(),
            ..Default::default()
        };
        upsert_breakout(&c, "z", &snap).unwrap();
        let got = get_breakout(&c, "Z").unwrap().unwrap();
        assert_eq!(got.breakout_label, "NEW_HIGH");
    }

    #[test]
    fn cash_cycle_snapshot_roundtrip() {
        let c = open_mem_conn_v13();
        let snap = CashCycleSnapshot {
            symbol: "W".to_string(),
            as_of: "2026-04-14".to_string(),
            latest_period: "2025-12-31".to_string(),
            ccc_days: 45.0,
            efficiency_label: "NEUTRAL".to_string(),
            trend_label: "STABLE".to_string(),
            periods: vec![CashCycleRow {
                period: "2025-12-31".to_string(),
                dso_days: 40.0,
                dio_days: 30.0,
                dpo_days: 25.0,
                ccc_days: 45.0,
            }],
            ..Default::default()
        };
        upsert_cash_cycle(&c, "w", &snap).unwrap();
        let got = get_cash_cycle(&c, "W").unwrap().unwrap();
        assert_eq!(got.latest_period, "2025-12-31");
        assert_eq!(got.periods.len(), 1);
    }

    #[test]
    fn credit_snapshot_roundtrip() {
        let c = open_mem_conn_v13();
        let snap = CreditSnapshot {
            symbol: "V".to_string(),
            as_of: "2026-04-14".to_string(),
            composite_score: 78.0,
            letter_grade: "A".to_string(),
            credit_label: "INVESTMENT_GRADE".to_string(),
            inputs_available: 4,
            ..Default::default()
        };
        upsert_credit(&c, "v", &snap).unwrap();
        let got = get_credit(&c, "V").unwrap().unwrap();
        assert_eq!(got.letter_grade, "A");
        assert_eq!(got.credit_label, "INVESTMENT_GRADE");
    }

    #[test]
    fn compute_momentum_strong() {
        // 260 bars, steadily rising from 100 → 140 with low vol.
        // Needs > 252 bars because compute_momentum_snapshot reads offset 252.
        let mut bars: Vec<HistoricalPriceRow> = Vec::new();
        for i in 0..260 {
            // newest-first: i=0 is the newest bar
            let days_old = i as f64;
            let close = 140.0 - days_old * (40.0 / 260.0); // newest=140, oldest≈100
            bars.push(make_bar(&format!("2026-{:02}-{:02}", 1 + (i / 30) % 12, 1 + (i % 28)), close, 1_000_000.0));
        }
        let snap = compute_momentum_snapshot("AAA", "2026-04-14", &bars);
        assert!(snap.bars_used >= 252);
        assert!(snap.return_12m_pct > 0.0);
        assert_ne!(snap.regime_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_momentum_insufficient() {
        let bars: Vec<HistoricalPriceRow> = (0..50).map(|i| make_bar(&format!("2026-04-{:02}", (i % 28) + 1), 100.0, 1_000.0)).collect();
        let snap = compute_momentum_snapshot("AAA", "2026-04-14", &bars);
        assert_eq!(snap.regime_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_liquidity_deep() {
        // 60 bars, large dollar volume
        let bars: Vec<HistoricalPriceRow> = (0..60).map(|i| {
            make_bar(&format!("2026-04-{:02}", (i % 28) + 1), 200.0, 10_000_000.0)
        }).collect();
        let snap = compute_liquidity_snapshot("BBB", "2026-04-14", &bars, 1_000_000_000.0, 60);
        assert_eq!(snap.liquidity_tier, "DEEP");
        assert!(snap.avg_daily_dollar_volume > 1.0e9);
    }

    #[test]
    fn compute_liquidity_thin() {
        let bars: Vec<HistoricalPriceRow> = (0..60).map(|i| {
            make_bar(&format!("2026-04-{:02}", (i % 28) + 1), 5.0, 1_000.0)
        }).collect();
        let snap = compute_liquidity_snapshot("BBB", "2026-04-14", &bars, 100_000_000.0, 60);
        assert!(matches!(snap.liquidity_tier.as_str(), "THIN" | "ILLIQUID"));
    }

    #[test]
    fn compute_liquidity_insufficient() {
        let bars: Vec<HistoricalPriceRow> = (0..10).map(|i| make_bar(&format!("2026-04-{:02}", i + 1), 100.0, 1_000.0)).collect();
        let snap = compute_liquidity_snapshot("BBB", "2026-04-14", &bars, 1_000_000.0, 60);
        assert_eq!(snap.liquidity_tier, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_breakout_new_high() {
        // Uses flat bars (high = low = close) so `current >= h52` holds.
        let mut bars: Vec<HistoricalPriceRow> = Vec::new();
        for i in 0..252 {
            let close = 100.0 - (i as f64) * 0.3; // newest = 100, oldest = ~24
            bars.push(make_bar_flat(&format!("2025-{:02}-{:02}", (1 + i / 30) % 12 + 1, (i % 28) + 1), close, 1_000_000.0));
        }
        let snap = compute_breakout_snapshot("CCC", "2026-04-14", &bars);
        assert_eq!(snap.breakout_label, "NEW_HIGH");
        assert!(snap.position_in_52w_range_pct >= 99.0);
    }

    #[test]
    fn compute_breakout_near_low() {
        // Newest bar is near the 52w low (older bars are higher)
        let mut bars: Vec<HistoricalPriceRow> = Vec::new();
        for i in 0..252 {
            let close = 10.0 + (i as f64) * 0.3;
            bars.push(make_bar_flat(&format!("2025-{:02}-{:02}", (1 + i / 30) % 12 + 1, (i % 28) + 1), close, 1_000_000.0));
        }
        let snap = compute_breakout_snapshot("CCC", "2026-04-14", &bars);
        assert!(matches!(snap.breakout_label.as_str(), "NEAR_LOW" | "NEW_LOW"));
    }

    #[test]
    fn compute_breakout_insufficient() {
        let bars: Vec<HistoricalPriceRow> = (0..5).map(|i| make_bar(&format!("2026-04-{:02}", i + 1), 100.0, 1_000.0)).collect();
        let snap = compute_breakout_snapshot("CCC", "2026-04-14", &bars);
        assert_eq!(snap.breakout_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_cash_cycle_efficient() {
        let income = IncomeStatement {
            date: "2025-12-31".to_string(),
            period: "FY".to_string(),
            revenue: 10_000.0,
            cost_of_revenue: 6_000.0,
            ..Default::default()
        };
        let income_prior = IncomeStatement {
            date: "2024-12-31".to_string(),
            period: "FY".to_string(),
            revenue: 9_000.0,
            cost_of_revenue: 5_400.0,
            ..Default::default()
        };
        let balance = BalanceSheet {
            date: "2025-12-31".to_string(),
            period: "FY".to_string(),
            net_receivables: 400.0,   // ~14.6 DSO
            inventory: 300.0,         // ~18.25 DIO
            accounts_payable: 900.0,  // ~54.75 DPO → CCC ≈ -21.9
            ..Default::default()
        };
        let balance_prior = BalanceSheet {
            date: "2024-12-31".to_string(),
            period: "FY".to_string(),
            net_receivables: 500.0,
            inventory: 350.0,
            accounts_payable: 850.0,
            ..Default::default()
        };
        let statements = FinancialStatements {
            income_annual: vec![income, income_prior],
            balance_annual: vec![balance, balance_prior],
            ..Default::default()
        };
        let snap = compute_cash_cycle_snapshot("DDD", "2026-04-14", &statements);
        assert!(snap.ccc_days < 30.0);
        assert_eq!(snap.efficiency_label, "EFFICIENT");
        assert_eq!(snap.periods.len(), 2);
    }

    #[test]
    fn compute_cash_cycle_insufficient() {
        let statements = FinancialStatements::default();
        let snap = compute_cash_cycle_snapshot("DDD", "2026-04-14", &statements);
        assert_eq!(snap.efficiency_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_credit_investment_grade() {
        let altz = AltmanZSnapshot {
            z_score: 4.5,
            zone: "SAFE".to_string(),
            ..Default::default()
        };
        let ptfs = PiotroskiSnapshot {
            f_score: 8,
            strength_label: "STRONG".to_string(),
            ..Default::default()
        };
        let lev = LeverageSnapshot {
            solvency_summary: "HEALTHY".to_string(),
            ..Default::default()
        };
        let acrl = AccrualsSnapshot {
            trend_label: "IMPROVING".to_string(),
            ttm_cash_conversion_pct: 120.0,
            ..Default::default()
        };
        let snap = compute_credit_snapshot("EEE", "2026-04-14", Some(&altz), Some(&ptfs), Some(&lev), Some(&acrl));
        assert_eq!(snap.credit_label, "INVESTMENT_GRADE");
        assert!(snap.composite_score >= 70.0);
        assert_eq!(snap.inputs_available, 4);
        assert_eq!(snap.components.len(), 4);
    }

    #[test]
    fn compute_credit_distressed() {
        let altz = AltmanZSnapshot {
            z_score: 0.8,
            zone: "DISTRESS".to_string(),
            ..Default::default()
        };
        let ptfs = PiotroskiSnapshot {
            f_score: 1,
            strength_label: "WEAK".to_string(),
            ..Default::default()
        };
        let lev = LeverageSnapshot {
            solvency_summary: "STRETCHED".to_string(),
            ..Default::default()
        };
        let acrl = AccrualsSnapshot {
            trend_label: "DETERIORATING".to_string(),
            ttm_cash_conversion_pct: 30.0,
            ..Default::default()
        };
        let snap = compute_credit_snapshot("EEE", "2026-04-14", Some(&altz), Some(&ptfs), Some(&lev), Some(&acrl));
        assert_eq!(snap.credit_label, "DISTRESSED");
        assert!(snap.composite_score < 35.0);
    }

    #[test]
    fn compute_credit_no_inputs() {
        let snap = compute_credit_snapshot("EEE", "2026-04-14", None, None, None, None);
        assert_eq!(snap.letter_grade, "INSUFFICIENT_DATA");
        assert_eq!(snap.inputs_available, 0);
    }

    // ── ADR-121 Round 14 tests ─────────────────────────────────────────────

    #[test]
    fn growm_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v14(&c).unwrap();
        let snap = GrowmSnapshot {
            symbol: "AAA".to_string(),
            as_of: "2026-04-14".to_string(),
            composite_score: 82.5,
            garp_label: "GARP".to_string(),
            inputs_available: 3,
            ..Default::default()
        };
        upsert_growm(&c, "aaa", &snap).unwrap();
        let got = get_growm(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.garp_label, "GARP");
        assert_eq!(got.inputs_available, 3);
    }

    #[test]
    fn flow_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v14(&c).unwrap();
        let snap = FlowSnapshot {
            symbol: "BBB".to_string(),
            as_of: "2026-04-14".to_string(),
            window_days: 90,
            composite_score: 72.0,
            flow_label: "BUY".to_string(),
            ..Default::default()
        };
        upsert_flow(&c, "bbb", &snap).unwrap();
        let got = get_flow(&c, "BBB").unwrap().unwrap();
        assert_eq!(got.flow_label, "BUY");
    }

    #[test]
    fn regime_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v14(&c).unwrap();
        let snap = RegimeSnapshot {
            symbol: "CCC".to_string(),
            as_of: "2026-04-14".to_string(),
            regime_label: "TRENDING".to_string(),
            inputs_available: 3,
            ..Default::default()
        };
        upsert_regime(&c, "ccc", &snap).unwrap();
        let got = get_regime(&c, "CCC").unwrap().unwrap();
        assert_eq!(got.regime_label, "TRENDING");
    }

    #[test]
    fn relvol_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v14(&c).unwrap();
        let snap = RelVolSnapshot {
            symbol: "DDD".to_string(),
            as_of: "2026-04-14".to_string(),
            activity_label: "HIGH".to_string(),
            direction_label: "BULLISH".to_string(),
            bars_used: 60,
            ..Default::default()
        };
        upsert_relvol(&c, "ddd", &snap).unwrap();
        let got = get_relvol(&c, "DDD").unwrap().unwrap();
        assert_eq!(got.activity_label, "HIGH");
    }

    #[test]
    fn margins_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v14(&c).unwrap();
        let snap = MarginsSnapshot {
            symbol: "EEE".to_string(),
            as_of: "2026-04-14".to_string(),
            basis: "annual".to_string(),
            overall_trend_label: "EXPANDING".to_string(),
            quality_label: "HIGH".to_string(),
            ..Default::default()
        };
        upsert_margins(&c, "eee", &snap).unwrap();
        let got = get_margins(&c, "EEE").unwrap().unwrap();
        assert_eq!(got.overall_trend_label, "EXPANDING");
    }

    #[test]
    fn compute_growm_garp() {
        let mom = MomentumSnapshot {
            composite_score: 78.0,
            regime_label: "STRONG".to_string(),
            ..Default::default()
        };
        let earm = EarmSnapshot {
            composite_score: 72.0,
            momentum_label: "ACCELERATING".to_string(),
            ..Default::default()
        };
        let divg = DivgSnapshot {
            cagr_3y_pct: 8.0,
            trend_label: "GROWING".to_string(),
            ..Default::default()
        };
        let snap = compute_growm_snapshot("AAA", "2026-04-14", Some(&mom), Some(&earm), Some(&divg));
        assert!(snap.composite_score >= 65.0);
        assert_eq!(snap.inputs_available, 3);
        assert!(matches!(snap.garp_label.as_str(), "GARP" | "GROWTH"));
    }

    #[test]
    fn compute_growm_no_inputs() {
        let snap = compute_growm_snapshot("AAA", "2026-04-14", None, None, None);
        assert_eq!(snap.garp_label, "NO_DATA");
        assert_eq!(snap.inputs_available, 0);
    }

    #[test]
    fn compute_flow_buy() {
        let trades = vec![
            InsiderTrade {
                transaction_date: "2026-04-10".to_string(),
                reporting_name: "Alice CFO".to_string(),
                transaction_type: "P-Purchase".to_string(),
                value_usd: 500_000.0,
                ..Default::default()
            },
            InsiderTrade {
                transaction_date: "2026-04-01".to_string(),
                reporting_name: "Bob CEO".to_string(),
                transaction_type: "P-Purchase".to_string(),
                value_usd: 800_000.0,
                ..Default::default()
            },
        ];
        let holders = vec![
            InstitutionalHolder { holder: "X Fund".to_string(), change: 100_000.0, ..Default::default() },
            InstitutionalHolder { holder: "Y Fund".to_string(), change: 50_000.0, ..Default::default() },
            InstitutionalHolder { holder: "Z Fund".to_string(), change: -20_000.0, ..Default::default() },
        ];
        let snap = compute_flow_snapshot("BBB", "2026-04-14", &trades, &holders, 90);
        assert!(matches!(snap.flow_label.as_str(), "BUY" | "STRONG_BUY"));
        assert_eq!(snap.insider_trade_count, 2);
        assert_eq!(snap.institutional_buyers, 2);
        assert_eq!(snap.institutional_sellers, 1);
    }

    #[test]
    fn compute_flow_no_data() {
        let snap = compute_flow_snapshot("BBB", "2026-04-14", &[], &[], 90);
        assert_eq!(snap.flow_label, "NO_DATA");
    }

    #[test]
    fn compute_regime_trending() {
        let tech = TechnicalSnapshot {
            indicators: vec![
                TechnicalIndicator { name: "ADX(14)".to_string(), value: 32.0, ..Default::default() },
            ],
            trend_summary: "bullish trend".to_string(),
            ..Default::default()
        };
        let vole = OhlcVolSnapshot {
            preferred_estimate_pct: 18.0,
            preferred_label: "Yang-Zhang".to_string(),
            ..Default::default()
        };
        let hra = HraSnapshot {
            sharpe_ratio: 1.8,
            volatility_annual_pct: 18.0,
            windows: vec![HraWindow { label: "1Y".to_string(), return_pct: 22.0, ..Default::default() }],
            ..Default::default()
        };
        let snap = compute_regime_snapshot("CCC", "2026-04-14", Some(&vole), Some(&tech), Some(&hra));
        assert_eq!(snap.regime_label, "TRENDING");
        assert_eq!(snap.inputs_available, 3);
    }

    #[test]
    fn compute_regime_volatile() {
        let vole = OhlcVolSnapshot {
            preferred_estimate_pct: 55.0,
            preferred_label: "Yang-Zhang".to_string(),
            ..Default::default()
        };
        let snap = compute_regime_snapshot("CCC", "2026-04-14", Some(&vole), None, None);
        assert_eq!(snap.regime_label, "VOLATILE");
    }

    #[test]
    fn compute_regime_no_inputs() {
        let snap = compute_regime_snapshot("CCC", "2026-04-14", None, None, None);
        assert_eq!(snap.regime_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_relvol_high() {
        let mut bars: Vec<HistoricalPriceRow> = Vec::new();
        // Current bar (index 0) has 5x avg volume.
        bars.push(HistoricalPriceRow {
            date: "2026-04-14".to_string(),
            volume: 5_000_000.0,
            close: 105.0,
            ..Default::default()
        });
        for i in 1..=60 {
            bars.push(HistoricalPriceRow {
                date: format!("2026-04-{:02}", 14 - (i % 14)),
                volume: 1_000_000.0,
                close: 100.0,
                ..Default::default()
            });
        }
        let snap = compute_relvol_snapshot("DDD", "2026-04-14", &bars);
        assert!(snap.rel_volume_20d >= 4.0);
        assert_eq!(snap.activity_label, "EXTREME");
        assert_eq!(snap.direction_label, "BULLISH");
    }

    #[test]
    fn compute_relvol_insufficient() {
        let bars: Vec<HistoricalPriceRow> = (0..10).map(|i| HistoricalPriceRow {
            date: format!("2026-04-{:02}", 14 - i),
            volume: 1_000.0,
            close: 100.0,
            ..Default::default()
        }).collect();
        let snap = compute_relvol_snapshot("DDD", "2026-04-14", &bars);
        assert_eq!(snap.activity_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_margins_expanding() {
        let latest = IncomeStatement {
            date: "2025-12-31".to_string(),
            period: "FY".to_string(),
            revenue: 10_000.0,
            gross_profit: 4_500.0,     // 45%
            operating_income: 2_500.0, // 25%
            net_income: 1_800.0,       // 18%
            ..Default::default()
        };
        let prior = IncomeStatement {
            date: "2024-12-31".to_string(),
            period: "FY".to_string(),
            revenue: 9_000.0,
            gross_profit: 3_600.0,     // 40%
            operating_income: 1_800.0, // 20%
            net_income: 1_350.0,       // 15%
            ..Default::default()
        };
        let statements = FinancialStatements {
            income_annual: vec![latest, prior],
            ..Default::default()
        };
        let snap = compute_margins_snapshot("EEE", "2026-04-14", &statements);
        assert_eq!(snap.overall_trend_label, "EXPANDING");
        assert_eq!(snap.quality_label, "HIGH");
        assert_eq!(snap.periods_used, 2);
        assert!(snap.operating_margin_change_pct > 0.0);
    }

    #[test]
    fn compute_margins_insufficient() {
        let statements = FinancialStatements::default();
        let snap = compute_margins_snapshot("EEE", "2026-04-14", &statements);
        assert_eq!(snap.overall_trend_label, "INSUFFICIENT_DATA");
    }

    // ── ADR-122 Round 15 tests ─────────────────────────────────────────────

    #[test]
    fn val_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v15(&c).unwrap();
        let snap = ValueSnapshot {
            symbol: "VAL1".to_string(),
            as_of: "2026-04-14".to_string(),
            sector: "Technology".to_string(),
            peers_considered: 9,
            value_label: "VALUE".to_string(),
            ..Default::default()
        };
        upsert_val(&c, "val1", &snap).unwrap();
        let got = get_val(&c, "VAL1").unwrap().unwrap();
        assert_eq!(got.value_label, "VALUE");
        assert_eq!(got.peers_considered, 9);
    }

    #[test]
    fn qual_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v15(&c).unwrap();
        let snap = QualitySnapshot {
            symbol: "QL1".to_string(),
            as_of: "2026-04-14".to_string(),
            quality_label: "HIGH_QUALITY".to_string(),
            composite_score: 82.0,
            ..Default::default()
        };
        upsert_qual(&c, "ql1", &snap).unwrap();
        let got = get_qual(&c, "QL1").unwrap().unwrap();
        assert_eq!(got.quality_label, "HIGH_QUALITY");
    }

    #[test]
    fn risk_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v15(&c).unwrap();
        let snap = RiskSnapshot {
            symbol: "RK1".to_string(),
            as_of: "2026-04-14".to_string(),
            risk_label: "MODERATE".to_string(),
            composite_score: 42.0,
            ..Default::default()
        };
        upsert_risk(&c, "rk1", &snap).unwrap();
        let got = get_risk(&c, "RK1").unwrap().unwrap();
        assert_eq!(got.risk_label, "MODERATE");
    }

    #[test]
    fn insstrk_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v15(&c).unwrap();
        let snap = InsiderStreakSnapshot {
            symbol: "INS1".to_string(),
            as_of: "2026-04-14".to_string(),
            window_days: 180,
            unique_insiders: 4,
            streak_label: "ACCUMULATION".to_string(),
            ..Default::default()
        };
        upsert_insstrk(&c, "ins1", &snap).unwrap();
        let got = get_insstrk(&c, "INS1").unwrap().unwrap();
        assert_eq!(got.streak_label, "ACCUMULATION");
    }

    #[test]
    fn covg_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v15(&c).unwrap();
        let snap = CoverageSnapshot {
            symbol: "CVG1".to_string(),
            as_of: "2026-04-14".to_string(),
            num_analysts: 18,
            coverage_label: "STABLE".to_string(),
            ..Default::default()
        };
        upsert_covg(&c, "cvg1", &snap).unwrap();
        let got = get_covg(&c, "CVG1").unwrap().unwrap();
        assert_eq!(got.coverage_label, "STABLE");
    }

    #[test]
    fn compute_val_value_label() {
        use crate::core::fundamentals::Fundamentals;
        let subject = Fundamentals {
            symbol: "SUB".to_string(),
            pe_ratio: Some(10.0),
            forward_pe: Some(9.0),
            price_to_book: Some(1.0),
            price_to_sales: Some(1.0),
            ev_to_ebitda: Some(6.0),
            ..Default::default()
        };
        let peers: Vec<Fundamentals> = (0..5).map(|i| Fundamentals {
            symbol: format!("P{}", i),
            pe_ratio: Some(20.0),
            forward_pe: Some(18.0),
            price_to_book: Some(3.0),
            price_to_sales: Some(3.0),
            ev_to_ebitda: Some(14.0),
            ..Default::default()
        }).collect();
        let fcfy = FcfYieldSnapshot {
            ttm_fcf_yield_pct: 8.0,
            ..Default::default()
        };
        let peer_fcfy = vec![4.0, 4.0, 4.0, 4.0, 4.0];
        let snap = compute_val_snapshot("SUB", "2026-04-14", "Technology", Some(&subject), &peers, Some(&fcfy), &peer_fcfy);
        assert!(matches!(snap.value_label.as_str(), "DEEP_VALUE" | "VALUE"));
        assert!(snap.composite_score >= 65.0);
        assert_eq!(snap.inputs_available, 6);
    }

    #[test]
    fn compute_val_no_data() {
        let snap = compute_val_snapshot("SUB", "2026-04-14", "Technology", None, &[], None, &[]);
        assert_eq!(snap.value_label, "NO_DATA");
    }

    #[test]
    fn compute_qual_high_quality() {
        let pt = PiotroskiSnapshot {
            f_score: 8,
            strength_label: "STRONG".to_string(),
            ..Default::default()
        };
        let mg = MarginsSnapshot {
            latest_operating_margin_pct: 28.0,
            overall_trend_label: "EXPANDING".to_string(),
            quality_label: "HIGH".to_string(),
            ..Default::default()
        };
        let ac = AccrualsSnapshot {
            ttm_cash_conversion_pct: 115.0,
            trend_label: "IMPROVING".to_string(),
            ..Default::default()
        };
        let lv = LeverageSnapshot {
            total_debt: 100.0,
            ebitda_ttm: 200.0,
            solvency_summary: "HEALTHY".to_string(),
            ..Default::default()
        };
        let snap = compute_qual_snapshot("QQ", "2026-04-14", Some(&pt), Some(&mg), Some(&ac), Some(&lv));
        assert!(matches!(snap.quality_label.as_str(), "HIGH_QUALITY" | "QUALITY"));
        assert_eq!(snap.inputs_available, 4);
        assert!(snap.composite_score >= 70.0);
    }

    #[test]
    fn compute_qual_no_inputs() {
        let snap = compute_qual_snapshot("QQ", "2026-04-14", None, None, None, None);
        assert_eq!(snap.quality_label, "NO_DATA");
    }

    #[test]
    fn compute_risk_distressed() {
        let altz = AltmanZSnapshot {
            z_score: 1.2,
            zone: "DISTRESS".to_string(),
            ..Default::default()
        };
        let snap = compute_risk_snapshot("RK", "2026-04-14", None, None, None, None, Some(&altz));
        assert_eq!(snap.risk_label, "DISTRESSED");
    }

    #[test]
    fn compute_risk_low() {
        let vole = OhlcVolSnapshot {
            preferred_estimate_pct: 12.0,
            preferred_label: "Yang-Zhang".to_string(),
            ..Default::default()
        };
        let beta = BetaSnapshot {
            windows: vec![BetaWindow {
                window_label: "1Y".to_string(),
                beta: 1.05,
                n_observations: 252,
                ..Default::default()
            }],
            ..Default::default()
        };
        let liq = LiquiditySnapshot {
            liquidity_tier: "DEEP".to_string(),
            ..Default::default()
        };
        let altz = AltmanZSnapshot {
            z_score: 4.5,
            zone: "SAFE".to_string(),
            ..Default::default()
        };
        let snap = compute_risk_snapshot("RK", "2026-04-14", Some(&vole), Some(&beta), Some(&liq), None, Some(&altz));
        assert_eq!(snap.risk_label, "LOW_RISK");
        assert_eq!(snap.inputs_available, 4);
    }

    #[test]
    fn compute_risk_no_inputs() {
        let snap = compute_risk_snapshot("RK", "2026-04-14", None, None, None, None, None);
        assert_eq!(snap.risk_label, "NO_DATA");
    }

    #[test]
    fn compute_insstrk_accumulation() {
        let trades = vec![
            InsiderTrade {
                transaction_date: "2026-03-01".to_string(),
                reporting_name: "Alice CFO".to_string(),
                transaction_type: "P-Purchase".to_string(),
                acquisition_disposition: "A".to_string(),
                shares: 1_000.0,
                value_usd: 50_000.0,
                ..Default::default()
            },
            InsiderTrade {
                transaction_date: "2026-03-10".to_string(),
                reporting_name: "Alice CFO".to_string(),
                transaction_type: "P-Purchase".to_string(),
                acquisition_disposition: "A".to_string(),
                shares: 1_000.0,
                value_usd: 55_000.0,
                ..Default::default()
            },
            InsiderTrade {
                transaction_date: "2026-03-05".to_string(),
                reporting_name: "Bob CEO".to_string(),
                transaction_type: "P-Purchase".to_string(),
                acquisition_disposition: "A".to_string(),
                shares: 500.0,
                value_usd: 25_000.0,
                ..Default::default()
            },
            InsiderTrade {
                transaction_date: "2026-03-20".to_string(),
                reporting_name: "Bob CEO".to_string(),
                transaction_type: "P-Purchase".to_string(),
                acquisition_disposition: "A".to_string(),
                shares: 500.0,
                value_usd: 27_000.0,
                ..Default::default()
            },
        ];
        let snap = compute_insstrk_snapshot("INS", "2026-04-14", &trades, 180);
        assert_eq!(snap.unique_insiders, 2);
        assert!(matches!(snap.streak_label.as_str(), "ACCUMULATION" | "STRONG_ACCUMULATION" | "MIXED"));
        assert!(snap.buy_streak_count >= 2);
    }

    #[test]
    fn compute_insstrk_none() {
        let snap = compute_insstrk_snapshot("INS", "2026-04-14", &[], 180);
        assert_eq!(snap.streak_label, "NONE");
    }

    #[test]
    fn compute_covg_stable() {
        let pt = PriceTarget {
            symbol: "CVG".to_string(),
            target_mean: 150.0,
            target_low: 120.0,
            target_high: 180.0,
            num_analysts: 18,
            ..Default::default()
        };
        let recs = vec![
            AnalystRecommendation {
                period: "2026-04-01".to_string(),
                strong_buy: 6,
                buy: 8,
                hold: 3,
                sell: 1,
                strong_sell: 0,
            },
        ];
        let updm = UpdmSnapshot {
            total_actions: 10,
            upgrades_90d: 5,
            downgrades_90d: 3,
            net_90d: 2,
            ..Default::default()
        };
        let snap = compute_covg_snapshot("CVG", "2026-04-14", Some(&pt), &recs, Some(&updm));
        assert!(matches!(snap.coverage_label.as_str(), "STABLE" | "EXPANDING"));
        assert_eq!(snap.inputs_available, 3);
    }

    #[test]
    fn compute_covg_none() {
        let snap = compute_covg_snapshot("CVG", "2026-04-14", None, &[], None);
        assert_eq!(snap.coverage_label, "NONE");
    }

    // ── ADR-123 Round 16 tests ─────────────────────────────────────────────

    #[test]
    fn vrk_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v16(&c).unwrap();
        let snap = ValueRankSnapshot {
            symbol: "VRK1".into(),
            as_of: "2026-04-15".into(),
            sector: "Tech".into(),
            rank_label: "TOP_QUARTILE".into(),
            percentile_rank: 78.5,
            ..Default::default()
        };
        upsert_vrk(&c, "vrk1", &snap).unwrap();
        let got = get_vrk(&c, "VRK1").unwrap().unwrap();
        assert_eq!(got.rank_label, "TOP_QUARTILE");
        assert!((got.percentile_rank - 78.5).abs() < 1e-6);
    }

    #[test]
    fn qrk_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v16(&c).unwrap();
        let snap = QualityRankSnapshot {
            symbol: "QRK1".into(),
            as_of: "2026-04-15".into(),
            sector: "Healthcare".into(),
            rank_label: "ABOVE_MEDIAN".into(),
            ..Default::default()
        };
        upsert_qrk(&c, "qrk1", &snap).unwrap();
        let got = get_qrk(&c, "QRK1").unwrap().unwrap();
        assert_eq!(got.rank_label, "ABOVE_MEDIAN");
    }

    #[test]
    fn rrk_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v16(&c).unwrap();
        let snap = RiskRankSnapshot {
            symbol: "RRK1".into(),
            as_of: "2026-04-15".into(),
            sector: "Energy".into(),
            rank_label: "SAFEST_QUARTILE".into(),
            ..Default::default()
        };
        upsert_rrk(&c, "rrk1", &snap).unwrap();
        let got = get_rrk(&c, "RRK1").unwrap().unwrap();
        assert_eq!(got.rank_label, "SAFEST_QUARTILE");
    }

    #[test]
    fn relepsgr_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v16(&c).unwrap();
        let snap = RelativeEpsGrowthSnapshot {
            symbol: "RELEPS1".into(),
            as_of: "2026-04-15".into(),
            sector: "Tech".into(),
            symbol_cagr_pct: 22.0,
            sector_median_cagr_pct: 12.0,
            relative_label: "ABOVE".into(),
            ..Default::default()
        };
        upsert_relepsgr(&c, "releps1", &snap).unwrap();
        let got = get_relepsgr(&c, "RELEPS1").unwrap().unwrap();
        assert_eq!(got.relative_label, "ABOVE");
    }

    #[test]
    fn pead_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v16(&c).unwrap();
        let snap = PeadSnapshot {
            symbol: "PEAD1".into(),
            as_of: "2026-04-15".into(),
            num_events: 8,
            events_used: 6,
            avg_drift_5d_pct: 3.2,
            drift_direction_label: "DRIFT_UP".into(),
            ..Default::default()
        };
        upsert_pead(&c, "pead1", &snap).unwrap();
        let got = get_pead(&c, "PEAD1").unwrap().unwrap();
        assert_eq!(got.drift_direction_label, "DRIFT_UP");
    }

    fn mk_val_snap(sym: &str, sector: &str, score: f64) -> ValueSnapshot {
        ValueSnapshot {
            symbol: sym.to_string(),
            as_of: "2026-04-15".into(),
            sector: sector.to_string(),
            composite_score: score,
            value_label: if score >= 65.0 { "VALUE".into() } else { "FAIR".into() },
            inputs_available: 6,
            ..Default::default()
        }
    }

    #[test]
    fn compute_vrk_top_decile() {
        let subj = mk_val_snap("SUB", "Tech", 92.0);
        let peer_vec: Vec<ValueSnapshot> = (0..9)
            .map(|i| mk_val_snap(&format!("P{}", i), "Tech", 30.0 + i as f64 * 5.0))
            .collect();
        let peers: Vec<&ValueSnapshot> = peer_vec.iter().collect();
        let snap = compute_vrk_snapshot("SUB", "2026-04-15", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "TOP_DECILE");
        assert!(snap.percentile_rank >= 90.0);
        assert_eq!(snap.rank_position, 1);
        assert_eq!(snap.peers_considered, 9);
    }

    #[test]
    fn compute_vrk_insufficient_peers() {
        let subj = mk_val_snap("SUB", "Tech", 70.0);
        let peer_vec = vec![mk_val_snap("P1", "Tech", 50.0), mk_val_snap("P2", "Tech", 60.0)];
        let peers: Vec<&ValueSnapshot> = peer_vec.iter().collect();
        let snap = compute_vrk_snapshot("SUB", "2026-04-15", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "NO_DATA");
    }

    fn mk_qual_snap(sym: &str, score: f64) -> QualitySnapshot {
        QualitySnapshot {
            symbol: sym.to_string(),
            as_of: "2026-04-15".into(),
            composite_score: score,
            quality_label: if score >= 65.0 { "QUALITY".into() } else { "AVERAGE".into() },
            inputs_available: 4,
            ..Default::default()
        }
    }

    #[test]
    fn compute_qrk_above_median() {
        let subj = mk_qual_snap("SUB", 72.0);
        let peer_vec: Vec<QualitySnapshot> = (0..9)
            .map(|i| mk_qual_snap(&format!("P{}", i), 40.0 + i as f64 * 4.0))
            .collect();
        let peers: Vec<&QualitySnapshot> = peer_vec.iter().collect();
        let snap = compute_qrk_snapshot("SUB", "2026-04-15", "Tech", Some(&subj), &peers);
        assert!(matches!(
            snap.rank_label.as_str(),
            "ABOVE_MEDIAN" | "TOP_QUARTILE" | "TOP_DECILE"
        ));
        assert!(snap.percentile_rank >= 50.0);
    }

    #[test]
    fn compute_qrk_no_data() {
        let snap = compute_qrk_snapshot("SUB", "2026-04-15", "Tech", None, &[]);
        assert_eq!(snap.rank_label, "NO_DATA");
    }

    fn mk_risk_snap(sym: &str, composite: f64) -> RiskSnapshot {
        RiskSnapshot {
            symbol: sym.to_string(),
            as_of: "2026-04-15".into(),
            composite_score: composite,
            risk_label: if composite >= 75.0 { "HIGH_RISK".into() } else { "MODERATE".into() },
            inputs_available: 5,
            ..Default::default()
        }
    }

    #[test]
    fn compute_rrk_safest() {
        // Subject has the LOWEST risk composite in the cohort → SAFEST_DECILE.
        let subj = mk_risk_snap("SUB", 10.0);
        let peer_vec: Vec<RiskSnapshot> = (0..9)
            .map(|i| mk_risk_snap(&format!("P{}", i), 40.0 + i as f64 * 5.0))
            .collect();
        let peers: Vec<&RiskSnapshot> = peer_vec.iter().collect();
        let snap = compute_rrk_snapshot("SUB", "2026-04-15", "Tech", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "SAFEST_DECILE");
        assert!(snap.percentile_rank >= 90.0);
        assert_eq!(snap.rank_position, 1);
    }

    #[test]
    fn compute_rrk_riskiest() {
        // Subject has the HIGHEST risk composite → RISKIEST_DECILE.
        let subj = mk_risk_snap("SUB", 95.0);
        let peer_vec: Vec<RiskSnapshot> = (0..9)
            .map(|i| mk_risk_snap(&format!("P{}", i), 20.0 + i as f64 * 5.0))
            .collect();
        let peers: Vec<&RiskSnapshot> = peer_vec.iter().collect();
        let snap = compute_rrk_snapshot("SUB", "2026-04-15", "Tech", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "RISKIEST_DECILE");
        assert!(snap.percentile_rank < 10.0);
    }

    fn mk_financials_with_eps(annual_eps_newest_first: &[f64]) -> FinancialStatements {
        let income: Vec<IncomeStatement> = annual_eps_newest_first
            .iter()
            .enumerate()
            .map(|(i, &eps)| IncomeStatement {
                date: format!("202{}-12-31", 6 - i),
                period: "FY".into(),
                eps,
                eps_diluted: eps,
                ..Default::default()
            })
            .collect();
        FinancialStatements {
            income_annual: income,
            ..Default::default()
        }
    }

    #[test]
    fn compute_relepsgr_above() {
        // Subject EPS: 8 → 4 (newest to oldest), 3-year CAGR ≈ 26 %.
        let subj = mk_financials_with_eps(&[8.0, 7.0, 5.5, 4.0]);
        // Peers: flat ~12 % CAGR (2.0 → 1.42)
        let peer_vec: Vec<(String, FinancialStatements)> = (0..5)
            .map(|i| (format!("P{}", i), mk_financials_with_eps(&[2.0, 1.8, 1.6, 1.42])))
            .collect();
        let snap = compute_relepsgr_snapshot("SUB", "2026-04-15", "Tech", Some(&subj), &peer_vec);
        assert!(matches!(snap.relative_label.as_str(), "ABOVE" | "FAR_ABOVE"));
        assert!(snap.symbol_cagr_pct > snap.sector_median_cagr_pct);
        assert_eq!(snap.years_used, 3);
    }

    #[test]
    fn compute_relepsgr_insufficient() {
        let subj = mk_financials_with_eps(&[5.0, 4.0]); // only 2 rows
        let snap = compute_relepsgr_snapshot("SUB", "2026-04-15", "Tech", Some(&subj), &[]);
        assert_eq!(snap.relative_label, "NO_DATA");
    }

    #[test]
    fn compute_pead_drift_up() {
        // Build 15 newest-first HP bars with a steady 1%/day advance.
        let mut bars: Vec<HistoricalPriceRow> = (0..15)
            .map(|i| HistoricalPriceRow {
                date: format!("2026-04-{:02}", 15 - i),
                close: 100.0 * (1.01f64).powi((14 - i) as i32),
                ..Default::default()
            })
            .collect();
        // newest-first
        bars.sort_by(|a, b| b.date.cmp(&a.date));
        // Build 3 beat surprises at increasing dates (all old enough that t0_idx ≥ 10).
        let surprises = vec![
            EarningsSurprise {
                date: "2026-04-01".into(),
                symbol: "PEAD".into(),
                eps_actual: 1.10,
                eps_estimate: 1.00,
                surprise: 0.10,
                surprise_pct: 10.0,
            },
            EarningsSurprise {
                date: "2026-04-02".into(),
                symbol: "PEAD".into(),
                eps_actual: 1.20,
                eps_estimate: 1.00,
                surprise: 0.20,
                surprise_pct: 20.0,
            },
            EarningsSurprise {
                date: "2026-04-03".into(),
                symbol: "PEAD".into(),
                eps_actual: 1.15,
                eps_estimate: 1.00,
                surprise: 0.15,
                surprise_pct: 15.0,
            },
        ];
        let snap = compute_pead_snapshot("PEAD", "2026-04-15", &surprises, &bars);
        assert_eq!(snap.drift_direction_label, "DRIFT_UP");
        assert!(snap.events_used >= 3);
        assert!(snap.avg_drift_5d_pct > 2.0);
    }

    #[test]
    fn compute_pead_no_events() {
        let snap = compute_pead_snapshot("PEAD", "2026-04-15", &[], &[]);
        assert_eq!(snap.drift_direction_label, "INSUFFICIENT_DATA");
    }

    // ── ADR-124 Round 17 tests ────────────────────────────────────────────

    fn mk_mom(sym: &str, composite: f64) -> MomentumSnapshot {
        MomentumSnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            bars_used: 252,
            composite_score: composite,
            regime_label: "STRONG".into(),
            trend_label: "STABLE".into(),
            ..Default::default()
        }
    }

    fn mk_pead(sym: &str, avg_5d: f64, events: usize) -> PeadSnapshot {
        PeadSnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            num_events: events,
            events_used: events,
            avg_drift_5d_pct: avg_5d,
            drift_direction_label: if avg_5d > 0.5 { "DRIFT_UP".into() } else { "MIXED".into() },
            ..Default::default()
        }
    }

    fn mk_ptfs(sym: &str, f_score: i32) -> PiotroskiSnapshot {
        PiotroskiSnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            f_score,
            strength_label: if f_score >= 7 { "STRONG".into() } else if f_score >= 4 { "MIXED".into() } else { "WEAK".into() },
            ..Default::default()
        }
    }

    fn mk_margins_q(sym: &str, op_margin: f64, quality: &str, trend: &str) -> MarginsSnapshot {
        MarginsSnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            latest_operating_margin_pct: op_margin,
            quality_label: quality.into(),
            overall_trend_label: trend.into(),
            ..Default::default()
        }
    }

    fn mk_accruals(sym: &str, cash_conv: f64, trend: &str) -> AccrualsSnapshot {
        AccrualsSnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            ttm_cash_conversion_pct: cash_conv,
            trend_label: trend.into(),
            ..Default::default()
        }
    }

    fn mk_financials_with_revenue(_sym: &str, revs: &[f64]) -> FinancialStatements {
        let income_annual: Vec<IncomeStatement> = revs
            .iter()
            .enumerate()
            .map(|(i, r)| IncomeStatement {
                date: format!("202{}-12-31", 6 - i),
                period: "FY".into(),
                revenue: *r,
                ..Default::default()
            })
            .collect();
        FinancialStatements {
            income_annual,
            ..Default::default()
        }
    }

    #[test]
    fn sizef_snapshot_roundtrip() {
        let conn = Connection::open_in_memory().expect("open conn");
        create_research_tables_v17(&conn).expect("create v17");
        let snap = SizeFactorSnapshot {
            symbol: "SIZ".into(),
            as_of: "2026-04-15".into(),
            sector: "Technology".into(),
            market_cap: 5e11,
            tier_label: "MEGA_CAP".into(),
            percentile_rank: 92.0,
            rank_label: "TOP_DECILE".into(),
            ..Default::default()
        };
        upsert_sizef(&conn, "SIZ", &snap).unwrap();
        let got = get_sizef(&conn, "SIZ").unwrap().unwrap();
        assert_eq!(got.tier_label, "MEGA_CAP");
        assert_eq!(got.rank_label, "TOP_DECILE");
    }

    #[test]
    fn momf_snapshot_roundtrip() {
        let conn = Connection::open_in_memory().expect("open conn");
        create_research_tables_v17(&conn).expect("create v17");
        let snap = MomentumRankSnapshot {
            symbol: "MMF".into(),
            as_of: "2026-04-15".into(),
            sector: "Energy".into(),
            composite_score: 72.0,
            rank_label: "TOP_QUARTILE".into(),
            ..Default::default()
        };
        upsert_momf(&conn, "MMF", &snap).unwrap();
        let got = get_momf(&conn, "MMF").unwrap().unwrap();
        assert_eq!(got.rank_label, "TOP_QUARTILE");
    }

    #[test]
    fn peadrank_snapshot_roundtrip() {
        let conn = Connection::open_in_memory().expect("open conn");
        create_research_tables_v17(&conn).expect("create v17");
        let snap = PeadRankSnapshot {
            symbol: "PRK".into(),
            as_of: "2026-04-15".into(),
            sector: "Healthcare".into(),
            avg_drift_5d_pct: 2.1,
            rank_label: "ABOVE_MEDIAN".into(),
            ..Default::default()
        };
        upsert_peadrank(&conn, "PRK", &snap).unwrap();
        let got = get_peadrank(&conn, "PRK").unwrap().unwrap();
        assert_eq!(got.rank_label, "ABOVE_MEDIAN");
    }

    #[test]
    fn fqm_snapshot_roundtrip() {
        let conn = Connection::open_in_memory().expect("open conn");
        create_research_tables_v17(&conn).expect("create v17");
        let snap = FundamentalQualityMeterSnapshot {
            symbol: "FQM".into(),
            as_of: "2026-04-15".into(),
            piotroski_score: 8,
            composite_score: 88.0,
            operator_label: "ELITE_OPERATOR".into(),
            inputs_available: 3,
            ..Default::default()
        };
        upsert_fqm(&conn, "FQM", &snap).unwrap();
        let got = get_fqm(&conn, "FQM").unwrap().unwrap();
        assert_eq!(got.operator_label, "ELITE_OPERATOR");
    }

    #[test]
    fn revrank_snapshot_roundtrip() {
        let conn = Connection::open_in_memory().expect("open conn");
        create_research_tables_v17(&conn).expect("create v17");
        let snap = RevenueGrowthRankSnapshot {
            symbol: "RVR".into(),
            as_of: "2026-04-15".into(),
            sector: "Technology".into(),
            symbol_cagr_pct: 25.0,
            sector_median_cagr_pct: 10.0,
            gap_to_median_pp: 15.0,
            relative_label: "FAR_ABOVE".into(),
            ..Default::default()
        };
        upsert_revrank(&conn, "RVR", &snap).unwrap();
        let got = get_revrank(&conn, "RVR").unwrap().unwrap();
        assert_eq!(got.relative_label, "FAR_ABOVE");
    }

    #[test]
    fn compute_sizef_top_decile() {
        let peers: Vec<(String, f64)> = vec![
            ("A".into(), 1e9),
            ("B".into(), 2e9),
            ("C".into(), 5e9),
            ("D".into(), 3e9),
        ];
        let snap = compute_sizef_snapshot("MEGA", "2026-04-15", "Technology", Some(5e11), &peers);
        assert_eq!(snap.tier_label, "MEGA_CAP");
        assert_eq!(snap.rank_label, "TOP_DECILE");
        assert_eq!(snap.peers_considered, 4);
    }

    #[test]
    fn compute_sizef_no_subject() {
        let snap = compute_sizef_snapshot("NIL", "2026-04-15", "Technology", None, &[]);
        assert_eq!(snap.tier_label, "NO_DATA");
        assert_eq!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_momf_above_median() {
        let peers_owned = [mk_mom("A", 40.0), mk_mom("B", 50.0), mk_mom("C", 60.0), mk_mom("D", 70.0)];
        let peers: Vec<&MomentumSnapshot> = peers_owned.iter().collect();
        let subj = mk_mom("MMF", 65.0);
        let snap = compute_momf_snapshot("MMF", "2026-04-15", "Energy", Some(&subj), &peers);
        assert!(snap.percentile_rank > 50.0);
        assert_ne!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_momf_no_subject() {
        let snap = compute_momf_snapshot("N", "2026-04-15", "S", None, &[]);
        assert_eq!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_peadrank_above_median() {
        let peers_owned = [
            mk_pead("A", 0.5, 4),
            mk_pead("B", 1.0, 5),
            mk_pead("C", 1.5, 4),
            mk_pead("D", 2.0, 4),
        ];
        let peers: Vec<&PeadSnapshot> = peers_owned.iter().collect();
        let subj = mk_pead("PRK", 1.8, 5);
        let snap = compute_peadrank_snapshot("PRK", "2026-04-15", "Healthcare", Some(&subj), &peers);
        assert!(snap.percentile_rank > 50.0);
        assert_ne!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_peadrank_insufficient() {
        let snap = compute_peadrank_snapshot("N", "2026-04-15", "S", None, &[]);
        assert_eq!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_fqm_elite_operator() {
        let p = mk_ptfs("FQM", 9);
        let m = mk_margins_q("FQM", 35.0, "HIGH", "EXPANDING");
        let a = mk_accruals("FQM", 115.0, "IMPROVING");
        let snap = compute_fqm_snapshot("FQM", "2026-04-15", Some(&p), Some(&m), Some(&a));
        assert_eq!(snap.operator_label, "ELITE_OPERATOR");
        assert_eq!(snap.inputs_available, 3);
        assert!(snap.composite_score >= 85.0);
    }

    #[test]
    fn compute_fqm_no_inputs() {
        let snap = compute_fqm_snapshot("NIL", "2026-04-15", None, None, None);
        assert_eq!(snap.operator_label, "NO_DATA");
        assert_eq!(snap.inputs_available, 0);
    }

    #[test]
    fn compute_revrank_far_above() {
        let subj = mk_financials_with_revenue("RVR", &[2000.0, 1600.0, 1300.0, 1000.0]);
        let peer_stmts: Vec<(String, FinancialStatements)> = vec![
            ("A".into(), mk_financials_with_revenue("A", &[1100.0, 1080.0, 1050.0, 1000.0])),
            ("B".into(), mk_financials_with_revenue("B", &[1080.0, 1060.0, 1030.0, 1000.0])),
            ("C".into(), mk_financials_with_revenue("C", &[1050.0, 1030.0, 1020.0, 1000.0])),
            ("D".into(), mk_financials_with_revenue("D", &[1070.0, 1050.0, 1030.0, 1000.0])),
        ];
        let snap = compute_revrank_snapshot("RVR", "2026-04-15", "Technology", Some(&subj), &peer_stmts);
        assert_eq!(snap.relative_label, "FAR_ABOVE");
        assert!(snap.symbol_cagr_pct > 20.0);
        assert!(snap.gap_to_median_pp > 15.0);
    }

    #[test]
    fn compute_revrank_insufficient_subject() {
        let snap = compute_revrank_snapshot("NIL", "2026-04-15", "S", None, &[]);
        assert_eq!(snap.relative_label, "NO_DATA");
    }

    // ── ADR-125 Round 18 tests ────────────────────────────────────────────

    fn mk_lev(sym: &str, debt: f64, equity: f64) -> LeverageSnapshot {
        LeverageSnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            total_debt: debt,
            total_equity: equity,
            ..Default::default()
        }
    }

    fn mk_margins_op(sym: &str, op_margin: f64) -> MarginsSnapshot {
        MarginsSnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            latest_operating_margin_pct: op_margin,
            overall_trend_label: "STABLE".into(),
            periods_used: 4,
            ..Default::default()
        }
    }

    fn mk_fqm(sym: &str, composite: f64, operator: &str) -> FundamentalQualityMeterSnapshot {
        FundamentalQualityMeterSnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            composite_score: composite,
            operator_label: operator.into(),
            inputs_available: 3,
            ..Default::default()
        }
    }

    fn mk_liq(sym: &str, adv_dollar: f64, tier: &str) -> LiquiditySnapshot {
        LiquiditySnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            avg_daily_dollar_volume: adv_dollar,
            liquidity_tier: tier.into(),
            ..Default::default()
        }
    }

    fn mk_surp(date: &str, surprise_pct: f64) -> EarningsSurprise {
        EarningsSurprise {
            date: date.into(),
            symbol: "X".into(),
            eps_actual: 1.0 + surprise_pct / 100.0,
            eps_estimate: 1.0,
            surprise: surprise_pct / 100.0,
            surprise_pct,
        }
    }

    #[test]
    fn levrank_snapshot_roundtrip() {
        let conn = Connection::open_in_memory().expect("open conn");
        create_research_tables_v18(&conn).expect("create v18");
        let snap = LeverageRankSnapshot {
            symbol: "LVR".into(),
            as_of: "2026-04-15".into(),
            sector: "Industrials".into(),
            debt_to_equity: 0.3,
            rank_label: "SAFEST_QUARTILE".into(),
            ..Default::default()
        };
        upsert_levrank(&conn, "LVR", &snap).unwrap();
        let got = get_levrank(&conn, "LVR").unwrap().unwrap();
        assert_eq!(got.rank_label, "SAFEST_QUARTILE");
        assert!((got.debt_to_equity - 0.3).abs() < 1e-9);
    }

    #[test]
    fn operank_snapshot_roundtrip() {
        let conn = Connection::open_in_memory().expect("open conn");
        create_research_tables_v18(&conn).expect("create v18");
        let snap = OperatingQualityRankSnapshot {
            symbol: "OPR".into(),
            as_of: "2026-04-15".into(),
            sector: "Technology".into(),
            operating_margin_pct: 35.0,
            rank_label: "TOP_DECILE".into(),
            ..Default::default()
        };
        upsert_operank(&conn, "OPR", &snap).unwrap();
        let got = get_operank(&conn, "OPR").unwrap().unwrap();
        assert_eq!(got.rank_label, "TOP_DECILE");
    }

    #[test]
    fn fqmrank_snapshot_roundtrip() {
        let conn = Connection::open_in_memory().expect("open conn");
        create_research_tables_v18(&conn).expect("create v18");
        let snap = FqmRankSnapshot {
            symbol: "FQR".into(),
            as_of: "2026-04-15".into(),
            sector: "Healthcare".into(),
            composite_score: 85.0,
            operator_label: "ELITE_OPERATOR".into(),
            rank_label: "TOP_DECILE".into(),
            ..Default::default()
        };
        upsert_fqmrank(&conn, "FQR", &snap).unwrap();
        let got = get_fqmrank(&conn, "FQR").unwrap().unwrap();
        assert_eq!(got.operator_label, "ELITE_OPERATOR");
        assert_eq!(got.rank_label, "TOP_DECILE");
    }

    #[test]
    fn liqrank_snapshot_roundtrip() {
        let conn = Connection::open_in_memory().expect("open conn");
        create_research_tables_v18(&conn).expect("create v18");
        let snap = LiquidityRankSnapshot {
            symbol: "LQR".into(),
            as_of: "2026-04-15".into(),
            sector: "Financials".into(),
            avg_daily_dollar_volume: 2.5e9,
            tier_label: "DEEP".into(),
            rank_label: "TOP_QUARTILE".into(),
            ..Default::default()
        };
        upsert_liqrank(&conn, "LQR", &snap).unwrap();
        let got = get_liqrank(&conn, "LQR").unwrap().unwrap();
        assert_eq!(got.tier_label, "DEEP");
        assert_eq!(got.rank_label, "TOP_QUARTILE");
    }

    #[test]
    fn surpstk_snapshot_roundtrip() {
        let conn = Connection::open_in_memory().expect("open conn");
        create_research_tables_v18(&conn).expect("create v18");
        let snap = EarningsSurpriseStreakSnapshot {
            symbol: "SUR".into(),
            as_of: "2026-04-15".into(),
            total_events: 8,
            beats: 7,
            misses: 1,
            beat_rate_pct: 87.5,
            current_streak_type: "BEAT".into(),
            current_streak_len: 5,
            streak_label: "HOT_STREAK".into(),
            ..Default::default()
        };
        upsert_surpstk(&conn, "SUR", &snap).unwrap();
        let got = get_surpstk(&conn, "SUR").unwrap().unwrap();
        assert_eq!(got.streak_label, "HOT_STREAK");
        assert_eq!(got.beats, 7);
    }

    #[test]
    fn compute_levrank_safest_decile() {
        // Subject has the LOWEST D/E in sector → should rank safest.
        let peers_owned = [
            mk_lev("A", 500.0, 1000.0), // D/E 0.50
            mk_lev("B", 800.0, 1000.0), // D/E 0.80
            mk_lev("C", 1200.0, 1000.0),// D/E 1.20
            mk_lev("D", 1500.0, 1000.0),// D/E 1.50
        ];
        let peers: Vec<&LeverageSnapshot> = peers_owned.iter().collect();
        let subj = mk_lev("LVR", 100.0, 1000.0); // D/E 0.10
        let snap = compute_levrank_snapshot("LVR", "2026-04-15", "Industrials", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "SAFEST_DECILE");
        assert!(snap.percentile_rank >= 90.0);
        assert_eq!(snap.rank_position, 1);
    }

    #[test]
    fn compute_levrank_negative_equity() {
        let subj = mk_lev("NEG", 500.0, -100.0);
        let snap = compute_levrank_snapshot("NEG", "2026-04-15", "S", Some(&subj), &[]);
        assert_eq!(snap.rank_label, "NEGATIVE_EQUITY");
    }

    #[test]
    fn compute_levrank_no_subject() {
        let snap = compute_levrank_snapshot("NIL", "2026-04-15", "S", None, &[]);
        assert_eq!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_operank_top_decile() {
        let peers_owned = [
            mk_margins_op("A", 5.0),
            mk_margins_op("B", 10.0),
            mk_margins_op("C", 15.0),
            mk_margins_op("D", 20.0),
        ];
        let peers: Vec<&MarginsSnapshot> = peers_owned.iter().collect();
        let subj = mk_margins_op("OPR", 45.0);
        let snap = compute_operank_snapshot("OPR", "2026-04-15", "Technology", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "TOP_DECILE");
        assert!(snap.percentile_rank >= 90.0);
    }

    #[test]
    fn compute_operank_no_subject() {
        let snap = compute_operank_snapshot("NIL", "2026-04-15", "S", None, &[]);
        assert_eq!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_fqmrank_top_decile() {
        let peers_owned = [
            mk_fqm("A", 40.0, "WEAK_OPERATOR"),
            mk_fqm("B", 55.0, "AVERAGE_OPERATOR"),
            mk_fqm("C", 65.0, "STRONG_OPERATOR"),
            mk_fqm("D", 72.0, "STRONG_OPERATOR"),
        ];
        let peers: Vec<&FundamentalQualityMeterSnapshot> = peers_owned.iter().collect();
        let subj = mk_fqm("FQR", 92.0, "ELITE_OPERATOR");
        let snap = compute_fqmrank_snapshot("FQR", "2026-04-15", "Technology", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "TOP_DECILE");
        assert_eq!(snap.operator_label, "ELITE_OPERATOR");
    }

    #[test]
    fn compute_fqmrank_filters_no_data_peers() {
        let peers_owned = [
            mk_fqm("A", 0.0, "NO_DATA"),
            mk_fqm("B", 0.0, "NO_DATA"),
            mk_fqm("C", 0.0, "NO_DATA"),
            mk_fqm("D", 0.0, "NO_DATA"),
        ];
        let peers: Vec<&FundamentalQualityMeterSnapshot> = peers_owned.iter().collect();
        let subj = mk_fqm("FQR", 90.0, "ELITE_OPERATOR");
        let snap = compute_fqmrank_snapshot("FQR", "2026-04-15", "T", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_liqrank_deepest() {
        let peers_owned = [
            mk_liq("A", 1e6, "THIN"),
            mk_liq("B", 5e7, "MODERATE"),
            mk_liq("C", 2e8, "LIQUID"),
            mk_liq("D", 8e8, "LIQUID"),
        ];
        let peers: Vec<&LiquiditySnapshot> = peers_owned.iter().collect();
        let subj = mk_liq("LQR", 5e9, "DEEP");
        let snap = compute_liqrank_snapshot("LQR", "2026-04-15", "Financials", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "TOP_DECILE");
        assert_eq!(snap.rank_position, 1);
        assert_eq!(snap.tier_label, "DEEP");
    }

    #[test]
    fn compute_liqrank_filters_insufficient_data() {
        let peers_owned = [
            mk_liq("A", 0.0, "INSUFFICIENT_DATA"),
            mk_liq("B", 0.0, "INSUFFICIENT_DATA"),
            mk_liq("C", 0.0, "INSUFFICIENT_DATA"),
        ];
        let peers: Vec<&LiquiditySnapshot> = peers_owned.iter().collect();
        let subj = mk_liq("LQR", 1e9, "DEEP");
        let snap = compute_liqrank_snapshot("LQR", "2026-04-15", "Financials", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_surpstk_hot_streak() {
        let rows = vec![
            mk_surp("2026-04-01", 12.0),
            mk_surp("2026-01-01", 9.0),
            mk_surp("2025-10-01", 6.0),
            mk_surp("2025-07-01", 4.0),
            mk_surp("2025-04-01", 5.0),
            mk_surp("2025-01-01", 3.0),
            mk_surp("2024-10-01", 1.0),
            mk_surp("2024-07-01", -3.0),
        ];
        let snap = compute_surpstk_snapshot("HOT", "2026-04-15", &rows);
        assert_eq!(snap.streak_label, "HOT_STREAK");
        assert_eq!(snap.current_streak_type, "BEAT");
        assert!(snap.current_streak_len >= 3);
        assert!(snap.beat_rate_pct >= 75.0);
    }

    #[test]
    fn compute_surpstk_cold_streak() {
        let rows = vec![
            mk_surp("2026-04-01", -8.0),
            mk_surp("2026-01-01", -5.0),
            mk_surp("2025-10-01", -4.0),
            mk_surp("2025-07-01", -3.0),
            mk_surp("2025-04-01", -6.0),
            mk_surp("2025-01-01", -2.5),
            mk_surp("2024-10-01", 1.0),
            mk_surp("2024-07-01", 2.5),
        ];
        let snap = compute_surpstk_snapshot("CLD", "2026-04-15", &rows);
        assert_eq!(snap.streak_label, "COLD_STREAK");
        assert_eq!(snap.current_streak_type, "MISS");
        assert!(snap.current_streak_len >= 3);
    }

    #[test]
    fn compute_surpstk_mixed() {
        // 50% beat rate, alternating → neither BEAT_TREND nor MISS_TREND.
        let rows = vec![
            mk_surp("2026-04-01", 3.0),
            mk_surp("2026-01-01", -3.0),
            mk_surp("2025-10-01", 3.0),
            mk_surp("2025-07-01", -3.0),
            mk_surp("2025-04-01", 3.0),
            mk_surp("2025-01-01", -3.0),
        ];
        let snap = compute_surpstk_snapshot("MIX", "2026-04-15", &rows);
        assert_eq!(snap.streak_label, "MIXED");
        assert_eq!(snap.beats, 3);
        assert_eq!(snap.misses, 3);
    }

    #[test]
    fn compute_surpstk_insufficient_data() {
        let snap = compute_surpstk_snapshot("NIL", "2026-04-15", &[]);
        assert_eq!(snap.streak_label, "INSUFFICIENT_DATA");
    }

    // ── ADR-126 Round 19 tests ────────────────────────────────────────────

    fn mk_divg(sym: &str, cagr3: f64, trend: &str) -> DivgSnapshot {
        DivgSnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            total_payments: 12,
            years_covered: 10,
            cagr_3y_pct: cagr3,
            consecutive_growth_years: 5,
            trend_label: trend.into(),
            ..Default::default()
        }
    }

    fn mk_earm(sym: &str, score: f64, label: &str) -> EarmSnapshot {
        EarmSnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            quarters_used: 8,
            composite_score: score,
            momentum_label: label.into(),
            ..Default::default()
        }
    }

    fn mk_updm(sym: &str, net90: i32, bias: &str) -> UpdmSnapshot {
        UpdmSnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            total_actions: 10,
            net_90d: net90,
            bias_label: bias.into(),
            ..Default::default()
        }
    }

    fn mk_hp(date: &str, open: f64, high: f64, low: f64, close: f64) -> HistoricalPriceRow {
        HistoricalPriceRow {
            date: date.into(),
            open,
            high,
            low,
            close,
            adj_close: close,
            volume: 1_000_000.0,
            change: close - open,
            change_pct: 0.0,
        }
    }

    #[test]
    fn dvdrank_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v19(&c).unwrap();
        let snap = DividendGrowthRankSnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            sector: "Tech".into(),
            cagr_3y_pct: 12.5,
            rank_label: "TOP_DECILE".into(),
            ..Default::default()
        };
        upsert_dvdrank(&c, "AAA", &snap).unwrap();
        let got = get_dvdrank(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.rank_label, "TOP_DECILE");
    }

    #[test]
    fn earmrank_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v19(&c).unwrap();
        let snap = EarningsMomentumRankSnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            sector: "Tech".into(),
            composite_score: 72.0,
            rank_label: "ABOVE_MEDIAN".into(),
            ..Default::default()
        };
        upsert_earmrank(&c, "AAA", &snap).unwrap();
        let got = get_earmrank(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.rank_label, "ABOVE_MEDIAN");
    }

    #[test]
    fn updgrank_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v19(&c).unwrap();
        let snap = UpgradeDowngradeRankSnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            sector: "Tech".into(),
            net_90d: 5,
            rank_label: "BELOW_MEDIAN".into(),
            ..Default::default()
        };
        upsert_updgrank(&c, "AAA", &snap).unwrap();
        let got = get_updgrank(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.net_90d, 5);
    }

    #[test]
    fn gy_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v19(&c).unwrap();
        let snap = GapYearlySnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            gaps_total: 8,
            gap_label: "NORMAL".into(),
            ..Default::default()
        };
        upsert_gy(&c, "AAA", &snap).unwrap();
        let got = get_gy(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.gap_label, "NORMAL");
    }

    #[test]
    fn des_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v19(&c).unwrap();
        let snap = DailyEventStreakSnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            bars_used: 252,
            current_streak_type: "UP".into(),
            current_streak_len: 3,
            streak_label: "STRONG_UPTREND".into(),
            ..Default::default()
        };
        upsert_des(&c, "AAA", &snap).unwrap();
        let got = get_des(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.streak_label, "STRONG_UPTREND");
    }

    #[test]
    fn compute_dvdrank_top_decile() {
        let subj = mk_divg("AAA", 15.0, "GROWING");
        let p1 = mk_divg("BBB", 3.0, "GROWING");
        let p2 = mk_divg("CCC", 5.0, "GROWING");
        let p3 = mk_divg("DDD", 2.0, "STABLE");
        let p4 = mk_divg("EEE", 1.0, "GROWING");
        let peers = vec![&p1, &p2, &p3, &p4];
        let snap = compute_dvdrank_snapshot("AAA", "2026-04-15", "Tech", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "TOP_DECILE");
        assert!(snap.percentile_rank >= 90.0);
    }

    #[test]
    fn compute_dvdrank_no_history_filtered() {
        let subj = mk_divg("AAA", 5.0, "GROWING");
        let bad = mk_divg("BBB", 0.0, "NO_HISTORY");
        let ok1 = mk_divg("CCC", 3.0, "STABLE");
        let ok2 = mk_divg("DDD", 2.0, "GROWING");
        let ok3 = mk_divg("EEE", 4.0, "STABLE");
        let peers = vec![&bad, &ok1, &ok2, &ok3];
        let snap = compute_dvdrank_snapshot("AAA", "2026-04-15", "Tech", Some(&subj), &peers);
        assert_eq!(snap.peers_considered, 4);
        assert_eq!(snap.peers_with_data, 3);
        assert!(snap.rank_label != "INSUFFICIENT_DATA");
        assert!(snap.rank_label != "NO_DATA");
    }

    #[test]
    fn compute_earmrank_above_median() {
        let subj = mk_earm("AAA", 75.0, "ACCELERATING");
        let p1 = mk_earm("BBB", 40.0, "STABLE");
        let p2 = mk_earm("CCC", 50.0, "STABLE");
        let p3 = mk_earm("DDD", 60.0, "ACCELERATING");
        let p4 = mk_earm("EEE", 30.0, "DECELERATING");
        let peers = vec![&p1, &p2, &p3, &p4];
        let snap = compute_earmrank_snapshot("AAA", "2026-04-15", "Tech", Some(&subj), &peers);
        assert!(snap.percentile_rank > 50.0);
        assert_eq!(snap.composite_score, 75.0);
    }

    #[test]
    fn compute_earmrank_insufficient_filtered() {
        let subj = mk_earm("AAA", 65.0, "STABLE");
        let bad = mk_earm("BBB", 0.0, "INSUFFICIENT_DATA");
        let peers = vec![&bad];
        let snap = compute_earmrank_snapshot("AAA", "2026-04-15", "Tech", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_updgrank_bullish() {
        let subj = mk_updm("AAA", 8, "BULLISH");
        let p1 = mk_updm("BBB", -2, "BEARISH");
        let p2 = mk_updm("CCC", 1, "NEUTRAL");
        let p3 = mk_updm("DDD", 3, "BULLISH");
        let p4 = mk_updm("EEE", -5, "BEARISH");
        let peers = vec![&p1, &p2, &p3, &p4];
        let snap = compute_updgrank_snapshot("AAA", "2026-04-15", "Tech", Some(&subj), &peers);
        assert_eq!(snap.net_90d, 8);
        assert!(snap.percentile_rank > 60.0);
    }

    #[test]
    fn compute_updgrank_no_coverage_filtered() {
        let subj = mk_updm("AAA", 3, "BULLISH");
        let bad = mk_updm("BBB", 0, "NO_COVERAGE");
        let ok = mk_updm("CCC", 0, "NEUTRAL");
        let peers = vec![&bad, &ok];
        let snap = compute_updgrank_snapshot("AAA", "2026-04-15", "Tech", Some(&subj), &peers);
        assert_eq!(snap.peers_with_data, 1);
    }

    #[test]
    fn compute_gy_normal() {
        // 30-bar window, one small up gap, one small down gap, rest flat.
        let mut bars = Vec::new();
        for i in 0..30 {
            let date = format!("2025-01-{:02}", i + 1);
            // Day 5: prev close 100, today open 102.5 → +2.5% gap
            // Day 10: prev close 100, today open 97.5 → -2.5% gap
            let open = if i == 5 { 102.5 } else if i == 10 { 97.5 } else { 100.0 };
            bars.push(mk_hp(&date, open, open + 1.0, open - 1.0, 100.0));
        }
        let snap = compute_gy_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.bars_used >= 20);
        assert!(snap.gaps_up_2pct >= 1);
        assert!(snap.gaps_down_2pct >= 1);
    }

    #[test]
    fn compute_gy_explosive() {
        // 30-bar window with a single 12% gap up → EXPLOSIVE via 10% band.
        let mut bars = Vec::new();
        for i in 0..30 {
            let date = format!("2025-02-{:02}", i + 1);
            let open = if i == 15 { 112.0 } else { 100.0 };
            bars.push(mk_hp(&date, open, open + 1.0, open - 1.0, 100.0));
        }
        let snap = compute_gy_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.gap_label, "EXPLOSIVE");
        assert_eq!(snap.gaps_up_10pct, 1);
    }

    #[test]
    fn compute_gy_insufficient() {
        let bars = vec![mk_hp("2025-03-01", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_gy_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.gap_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_des_uptrend() {
        // 30 bars, strictly rising close each day → STRONG_UPTREND.
        let mut bars = Vec::new();
        for i in 0..30 {
            let date = format!("2025-04-{:02}", i + 1);
            let close = 100.0 + i as f64;
            bars.push(mk_hp(&date, close - 0.5, close + 0.5, close - 0.5, close));
        }
        let snap = compute_des_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.streak_label, "STRONG_UPTREND");
        assert_eq!(snap.current_streak_type, "UP");
        assert!(snap.longest_up_streak >= 5);
    }

    #[test]
    fn compute_des_downtrend() {
        let mut bars = Vec::new();
        for i in 0..30 {
            let date = format!("2025-05-{:02}", i + 1);
            let close = 200.0 - i as f64;
            bars.push(mk_hp(&date, close + 0.5, close + 0.5, close - 0.5, close));
        }
        let snap = compute_des_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.streak_label, "STRONG_DOWNTREND");
        assert_eq!(snap.current_streak_type, "DOWN");
    }

    #[test]
    fn compute_des_insufficient() {
        let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_des_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.streak_label, "INSUFFICIENT_DATA");
    }

    // ── ADR-127 Round 20 tests ────────────────────────────────────────────

    #[test]
    fn dvdyieldrank_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v20(&c).unwrap();
        let snap = DividendYieldRankSnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            sector: "Utilities".into(),
            dividend_yield_pct: 4.5,
            rank_label: "TOP_DECILE".into(),
            ..Default::default()
        };
        upsert_dvdyieldrank(&c, "AAA", &snap).unwrap();
        let got = get_dvdyieldrank(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.rank_label, "TOP_DECILE");
        assert_eq!(got.dividend_yield_pct, 4.5);
    }

    #[test]
    fn shrank_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v20(&c).unwrap();
        let snap = ShortInterestRankSnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            sector: "Tech".into(),
            short_pct_of_float: 2.5,
            rank_label: "SAFEST_DECILE".into(),
            ..Default::default()
        };
        upsert_shrank(&c, "AAA", &snap).unwrap();
        let got = get_shrank(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.rank_label, "SAFEST_DECILE");
        assert_eq!(got.short_pct_of_float, 2.5);
    }

    #[test]
    fn atrann_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v20(&c).unwrap();
        let snap = AnnualizedAtrSnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            latest_close: 100.0,
            atr14: 1.5,
            atr14_pct: 1.5,
            atr_annualized_pct: 23.8,
            regime_label: "NORMAL_VOL".into(),
            ..Default::default()
        };
        upsert_atrann(&c, "AAA", &snap).unwrap();
        let got = get_atrann(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.regime_label, "NORMAL_VOL");
        assert!((got.atr_annualized_pct - 23.8).abs() < 1e-9);
    }

    #[test]
    fn ddhist_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v20(&c).unwrap();
        let snap = DrawdownHistorySnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            max_drawdown_pct: -12.0,
            longest_drawdown_days: 45,
            corrections_5pct: 3,
            corrections_10pct: 1,
            current_drawdown_pct: -2.0,
            regime_label: "MEANINGFUL".into(),
            ..Default::default()
        };
        upsert_ddhist(&c, "AAA", &snap).unwrap();
        let got = get_ddhist(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.regime_label, "MEANINGFUL");
        assert_eq!(got.corrections_5pct, 3);
    }

    #[test]
    fn priceperf_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v20(&c).unwrap();
        let snap = PricePerformanceSnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            latest_close: 120.0,
            ret_1m_pct: 2.5,
            ret_3m_pct: 8.0,
            ret_6m_pct: 12.0,
            ret_ytd_pct: 15.0,
            ret_1y_pct: 20.0,
            trend_label: "BULL".into(),
            ..Default::default()
        };
        upsert_priceperf(&c, "AAA", &snap).unwrap();
        let got = get_priceperf(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.trend_label, "BULL");
        assert!((got.ret_1y_pct - 20.0).abs() < 1e-9);
    }

    #[test]
    fn compute_dvdyieldrank_top_decile() {
        let peers = vec![
            ("BBB".to_string(), Some(1.5)),
            ("CCC".to_string(), Some(2.0)),
            ("DDD".to_string(), Some(2.5)),
            ("EEE".to_string(), Some(1.0)),
        ];
        let snap = compute_dvdyieldrank_snapshot("AAA", "2026-04-15", "Utilities", Some(6.0), &peers);
        assert!(snap.percentile_rank >= 90.0);
        assert_eq!(snap.rank_label, "TOP_DECILE");
        assert_eq!(snap.peers_with_data, 4);
    }

    #[test]
    fn compute_dvdyieldrank_non_payer_filtered() {
        let peers = vec![
            ("BBB".to_string(), None),              // non-payer
            ("CCC".to_string(), Some(0.0)),         // non-payer (zero yield)
            ("DDD".to_string(), Some(3.0)),
            ("EEE".to_string(), Some(4.0)),
            ("FFF".to_string(), Some(2.0)),
        ];
        let snap = compute_dvdyieldrank_snapshot("AAA", "2026-04-15", "Utilities", Some(5.0), &peers);
        assert_eq!(snap.peers_considered, 5);
        assert_eq!(snap.peers_with_data, 3);
        assert_ne!(snap.rank_label, "INSUFFICIENT_DATA");
        assert_ne!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_dvdyieldrank_subject_non_payer() {
        let peers = vec![
            ("BBB".to_string(), Some(3.0)),
            ("CCC".to_string(), Some(4.0)),
            ("DDD".to_string(), Some(2.0)),
        ];
        let snap = compute_dvdyieldrank_snapshot("AAA", "2026-04-15", "Tech", Some(0.0), &peers);
        assert_eq!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_shrank_safest_decile() {
        // Subject has lowest short interest → risk-inverted top rank (SAFEST).
        let peers = vec![
            ("BBB".to_string(), Some(8.0)),
            ("CCC".to_string(), Some(10.0)),
            ("DDD".to_string(), Some(12.0)),
            ("EEE".to_string(), Some(15.0)),
        ];
        let snap = compute_shrank_snapshot("AAA", "2026-04-15", "Tech", Some(1.0), &peers);
        assert!(snap.percentile_rank >= 90.0);
        assert_eq!(snap.rank_label, "SAFEST_DECILE");
        assert_eq!(snap.short_pct_of_float, 1.0);
    }

    #[test]
    fn compute_shrank_riskiest_decile() {
        // Subject has highest short interest → risk-inverted bottom rank (RISKIEST).
        // Need ≥10 peers so the floor 0.5/total*100 is strictly below 10.
        let peers = vec![
            ("BBB".to_string(), Some(2.0)),
            ("CCC".to_string(), Some(3.0)),
            ("DDD".to_string(), Some(4.0)),
            ("EEE".to_string(), Some(5.0)),
            ("FFF".to_string(), Some(6.0)),
            ("GGG".to_string(), Some(7.0)),
            ("HHH".to_string(), Some(8.0)),
            ("III".to_string(), Some(9.0)),
            ("JJJ".to_string(), Some(10.0)),
            ("KKK".to_string(), Some(11.0)),
        ];
        let snap = compute_shrank_snapshot("AAA", "2026-04-15", "Tech", Some(25.0), &peers);
        assert!(snap.percentile_rank < 10.0);
        assert_eq!(snap.rank_label, "RISKIEST_DECILE");
    }

    #[test]
    fn compute_shrank_insufficient() {
        let peers = vec![
            ("BBB".to_string(), None),
            ("CCC".to_string(), Some(5.0)),
        ];
        let snap = compute_shrank_snapshot("AAA", "2026-04-15", "Tech", Some(3.0), &peers);
        assert_eq!(snap.rank_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_atrann_low_vol() {
        // 30 quiet bars (≤0.5% HL range) → LOW_VOL regime.
        let mut bars = Vec::new();
        for i in 0..30 {
            let date = format!("2025-01-{:02}", i + 1);
            bars.push(mk_hp(&date, 100.0, 100.3, 99.7, 100.0));
        }
        let snap = compute_atrann_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.regime_label, "LOW_VOL");
        assert!(snap.atr_annualized_pct < 15.0);
    }

    #[test]
    fn compute_atrann_high_vol() {
        // 30 wild bars (5% HL range) → HIGH_VOL or EXTREME_VOL.
        let mut bars = Vec::new();
        for i in 0..30 {
            let date = format!("2025-02-{:02}", i + 1);
            bars.push(mk_hp(&date, 100.0, 103.0, 97.0, 100.0));
        }
        let snap = compute_atrann_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.atr_annualized_pct > 30.0, "expected > 30% annualized, got {}", snap.atr_annualized_pct);
        assert!(snap.regime_label == "HIGH_VOL" || snap.regime_label == "EXTREME_VOL");
    }

    #[test]
    fn compute_atrann_insufficient() {
        let bars = vec![mk_hp("2025-03-01", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_atrann_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.regime_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_ddhist_shallow() {
        // 30 quiet bars, max 3% dip → SHALLOW regime.
        let mut bars = Vec::new();
        for i in 0..30 {
            let date = format!("2025-04-{:02}", i + 1);
            let c = if (5..15).contains(&i) { 98.0 } else { 100.0 };
            bars.push(mk_hp(&date, c, c + 0.5, c - 0.5, c));
        }
        let snap = compute_ddhist_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.max_drawdown_pct > -5.0);
        assert!(snap.regime_label == "SHALLOW" || snap.regime_label == "RECOVERING");
    }

    #[test]
    fn compute_ddhist_severe() {
        // Rise to peak, then 25% decline and partial recovery → SEVERE.
        let mut bars = Vec::new();
        for i in 0..20 {
            let date = format!("2025-05-{:02}", i + 1);
            bars.push(mk_hp(&date, 100.0, 101.0, 99.0, 100.0));
        }
        // Peak at 120
        for i in 0..10 {
            let date = format!("2025-06-{:02}", i + 1);
            let c = 100.0 + (i as f64 + 1.0) * 2.0;
            bars.push(mk_hp(&date, c, c + 0.5, c - 0.5, c));
        }
        // Crash down 25% to 90
        for i in 0..10 {
            let date = format!("2025-07-{:02}", i + 1);
            let c = 120.0 - (i as f64 + 1.0) * 3.0;
            bars.push(mk_hp(&date, c, c + 0.5, c - 0.5, c));
        }
        let snap = compute_ddhist_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.max_drawdown_pct < -20.0, "expected < -20%, got {}", snap.max_drawdown_pct);
        assert!(snap.regime_label == "SEVERE" || snap.regime_label == "CATASTROPHIC" || snap.regime_label == "MEANINGFUL");
    }

    #[test]
    fn compute_priceperf_bull() {
        // Sustained 25%+ rally over the window → BULL or STRONG_BULL.
        let mut bars = Vec::new();
        for i in 0..260 {
            let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
            let c = 100.0 + i as f64 * 0.15;  // ~39% rise over window
            bars.push(mk_hp(&date, c, c + 0.2, c - 0.2, c));
        }
        let snap = compute_priceperf_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.ret_1y_pct > 10.0);
        assert!(snap.trend_label == "BULL" || snap.trend_label == "STRONG_BULL");
    }

    #[test]
    fn compute_priceperf_bear() {
        // Sustained decline → BEAR or STRONG_BEAR.
        let mut bars = Vec::new();
        for i in 0..260 {
            let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
            let c = 200.0 - i as f64 * 0.3;  // ~39% decline
            bars.push(mk_hp(&date, c, c + 0.2, c - 0.2, c));
        }
        let snap = compute_priceperf_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.ret_1y_pct < -10.0);
        assert!(snap.trend_label == "BEAR" || snap.trend_label == "STRONG_BEAR");
    }

    #[test]
    fn compute_priceperf_insufficient() {
        let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_priceperf_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.trend_label, "INSUFFICIENT_DATA");
    }

    // ── ADR-128 Round 21 tests ──

    #[test]
    fn betarank_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v21(&c).unwrap();
        let snap = BetaRankSnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            sector: "Technology".into(),
            subject_beta: Some(0.8),
            percentile_rank: 85.0,
            rank_label: "SAFEST_QUARTILE".into(),
            ..Default::default()
        };
        upsert_betarank(&c, "AAA", &snap).unwrap();
        let got = get_betarank(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.rank_label, "SAFEST_QUARTILE");
        assert!((got.subject_beta.unwrap() - 0.8).abs() < 1e-9);
    }

    #[test]
    fn pegrank_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v21(&c).unwrap();
        let snap = PegRankSnapshot {
            symbol: "BBB".into(),
            as_of: "2026-04-15".into(),
            sector: "Technology".into(),
            subject_peg: Some(0.9),
            percentile_rank: 90.0,
            rank_label: "TOP_DECILE".into(),
            ..Default::default()
        };
        upsert_pegrank(&c, "BBB", &snap).unwrap();
        let got = get_pegrank(&c, "BBB").unwrap().unwrap();
        assert_eq!(got.rank_label, "TOP_DECILE");
    }

    #[test]
    fn fhighlow_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v21(&c).unwrap();
        let snap = FiftyTwoWeekHighLowSnapshot {
            symbol: "CCC".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            latest_close: 120.0,
            high_52w: 150.0,
            high_52w_date: "2025-11-01".into(),
            days_since_high: 100,
            low_52w: 80.0,
            low_52w_date: "2025-07-01".into(),
            days_since_low: 200,
            pct_from_high: -20.0,
            pct_from_low: 50.0,
            range_position_pct: 57.0,
            proximity_label: "MID_RANGE".into(),
            ..Default::default()
        };
        upsert_fhighlow(&c, "CCC", &snap).unwrap();
        let got = get_fhighlow(&c, "CCC").unwrap().unwrap();
        assert_eq!(got.proximity_label, "MID_RANGE");
    }

    #[test]
    fn rvcone_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v21(&c).unwrap();
        let snap = RealizedVolConeSnapshot {
            symbol: "DDD".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            latest_close: 120.0,
            rv20_pct: 25.0,
            rv60_pct: 22.0,
            rv120_pct: 20.0,
            rv252_pct: 18.0,
            rv20_min_pct: 10.0,
            rv20_median_pct: 20.0,
            rv20_max_pct: 40.0,
            rv20_percentile: 75.0,
            cone_label: "ELEVATED".into(),
            ..Default::default()
        };
        upsert_rvcone(&c, "DDD", &snap).unwrap();
        let got = get_rvcone(&c, "DDD").unwrap().unwrap();
        assert_eq!(got.cone_label, "ELEVATED");
    }

    #[test]
    fn calpb_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v21(&c).unwrap();
        let snap = CalendarPeriodBreakdownSnapshot {
            symbol: "EEE".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            latest_close: 100.0,
            mtd_pct: 2.0,
            qtd_pct: 5.0,
            ytd_pct: 8.0,
            prior_quarter_pct: 1.0,
            prior_year_pct: 10.0,
            current_year: "2026".into(),
            current_quarter: "Q2".into(),
            momentum_label: "ACCELERATING".into(),
            ..Default::default()
        };
        upsert_calpb(&c, "EEE", &snap).unwrap();
        let got = get_calpb(&c, "EEE").unwrap().unwrap();
        assert_eq!(got.momentum_label, "ACCELERATING");
    }

    #[test]
    fn compute_betarank_safest_decile() {
        // Subject has the lowest beta by far → safest decile.
        let peers = vec![
            ("B".to_string(), Some(1.5)),
            ("C".to_string(), Some(1.8)),
            ("D".to_string(), Some(2.0)),
            ("E".to_string(), Some(1.6)),
            ("F".to_string(), Some(1.7)),
            ("G".to_string(), Some(1.9)),
            ("H".to_string(), Some(2.1)),
            ("I".to_string(), Some(1.4)),
            ("J".to_string(), Some(1.55)),
            ("K".to_string(), Some(2.2)),
        ];
        let snap = compute_betarank_snapshot("AAA", "2026-04-15", "Technology", Some(0.6), &peers);
        assert_eq!(snap.rank_label, "SAFEST_DECILE");
        assert_eq!(snap.rank_position, 1);
    }

    #[test]
    fn compute_betarank_riskiest_decile() {
        // Subject has the highest beta → riskiest decile.
        let peers = vec![
            ("B".to_string(), Some(0.6)),
            ("C".to_string(), Some(0.7)),
            ("D".to_string(), Some(0.8)),
            ("E".to_string(), Some(0.9)),
            ("F".to_string(), Some(1.0)),
            ("G".to_string(), Some(1.1)),
            ("H".to_string(), Some(1.2)),
            ("I".to_string(), Some(1.3)),
            ("J".to_string(), Some(1.4)),
            ("K".to_string(), Some(1.5)),
        ];
        let snap = compute_betarank_snapshot("AAA", "2026-04-15", "Technology", Some(2.5), &peers);
        assert_eq!(snap.rank_label, "RISKIEST_DECILE");
    }

    #[test]
    fn compute_betarank_insufficient() {
        let peers = vec![("B".to_string(), Some(1.0)), ("C".to_string(), Some(1.1))];
        let snap = compute_betarank_snapshot("AAA", "2026-04-15", "Technology", Some(0.9), &peers);
        assert_eq!(snap.rank_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_pegrank_top_decile() {
        // Subject has the lowest PEG → top (best value) decile.
        let peers = vec![
            ("B".to_string(), Some(2.0)),
            ("C".to_string(), Some(2.2)),
            ("D".to_string(), Some(2.5)),
            ("E".to_string(), Some(2.8)),
            ("F".to_string(), Some(3.0)),
            ("G".to_string(), Some(3.2)),
            ("H".to_string(), Some(3.5)),
            ("I".to_string(), Some(3.8)),
            ("J".to_string(), Some(4.0)),
            ("K".to_string(), Some(4.5)),
        ];
        let snap = compute_pegrank_snapshot("AAA", "2026-04-15", "Technology", Some(0.5), &peers);
        assert_eq!(snap.rank_label, "TOP_DECILE");
        assert_eq!(snap.rank_position, 1);
    }

    #[test]
    fn compute_pegrank_filters_negative() {
        // Negative/missing peer PEGs get filtered out.
        let peers = vec![
            ("B".to_string(), Some(2.0)),
            ("C".to_string(), Some(2.5)),
            ("D".to_string(), None),
            ("E".to_string(), Some(-1.5)),
            ("F".to_string(), Some(3.0)),
        ];
        let snap = compute_pegrank_snapshot("AAA", "2026-04-15", "Technology", Some(1.5), &peers);
        assert_eq!(snap.peers_with_data, 3);
    }

    #[test]
    fn compute_pegrank_subject_negative() {
        let peers = vec![("B".to_string(), Some(2.0)), ("C".to_string(), Some(2.5)), ("D".to_string(), Some(3.0))];
        let snap = compute_pegrank_snapshot("AAA", "2026-04-15", "Technology", Some(-0.5), &peers);
        assert_eq!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_fhighlow_at_high() {
        // Latest close == the highest close in the window.
        let mut bars = Vec::new();
        for i in 0..253 {
            let date = format!("2025-{:02}-{:02}", (i / 21) + 1, (i % 21) + 1);
            let c = 100.0 + i as f64 * 0.5;  // monotone up
            bars.push(mk_hp(&date, c, c + 0.5, c - 0.5, c));
        }
        let snap = compute_fhighlow_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.proximity_label, "AT_HIGH");
        assert_eq!(snap.days_since_high, 0);
        assert!((snap.pct_from_high - 0.0).abs() < 1e-9);
    }

    #[test]
    fn compute_fhighlow_at_low() {
        // Monotone down → latest close == lowest close.
        let mut bars = Vec::new();
        for i in 0..253 {
            let date = format!("2025-{:02}-{:02}", (i / 21) + 1, (i % 21) + 1);
            let c = 200.0 - i as f64 * 0.5;
            bars.push(mk_hp(&date, c, c + 0.5, c - 0.5, c));
        }
        let snap = compute_fhighlow_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.proximity_label, "AT_LOW");
        assert_eq!(snap.days_since_low, 0);
    }

    #[test]
    fn compute_fhighlow_insufficient() {
        let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_fhighlow_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.proximity_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_rvcone_compressed() {
        // Flat-ish series → compressed / below-avg realized vol.
        let mut bars = Vec::new();
        for i in 0..260 {
            let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
            let c = 100.0 + (i as f64 * 0.0001);
            bars.push(mk_hp(&date, c, c + 0.001, c - 0.001, c));
        }
        let snap = compute_rvcone_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.cone_label != "INSUFFICIENT_DATA");
        assert!(snap.rv252_pct < 5.0, "expected low vol, got {}", snap.rv252_pct);
    }

    #[test]
    fn compute_rvcone_extreme() {
        // Highly variable → elevated / extreme.
        let mut bars = Vec::new();
        for i in 0..260 {
            let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
            // Large oscillations with bigger swings at the end.
            let base = 100.0;
            let amp = if i < 200 { 1.0 } else { 10.0 };
            let c = base + amp * ((i as f64 * 0.5).sin() * (i as f64 * 0.3).cos());
            bars.push(mk_hp(&date, c, c + 0.5, c - 0.5, c));
        }
        let snap = compute_rvcone_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.cone_label != "INSUFFICIENT_DATA");
        assert!(snap.rv20_pct > snap.rv252_pct,
            "expected recent 20d RV > 252d RV due to amplitude shift, got rv20={} rv252={}",
            snap.rv20_pct, snap.rv252_pct);
    }

    #[test]
    fn compute_rvcone_insufficient() {
        let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_rvcone_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.cone_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_calpb_accelerating() {
        // Q1 2026 flat, Q2 2026 big up move — accelerating vs prior quarter.
        let mut bars = Vec::new();
        // Prior year 2025 Q4 (Oct-Dec): bars from 100→100 (flat prior year Q4).
        for m in 10..=12 {
            for d in 1..=20 {
                let c = 100.0;
                bars.push(mk_hp(&format!("2025-{:02}-{:02}", m, d), c, c + 0.1, c - 0.1, c));
            }
        }
        // 2026 Q1 (Jan-Mar) flat at 100 → 100.5
        for m in 1..=3 {
            for d in 1..=20 {
                let c = 100.0 + ((m - 1) * 20 + d) as f64 * 0.01;
                bars.push(mk_hp(&format!("2026-{:02}-{:02}", m, d), c, c + 0.1, c - 0.1, c));
            }
        }
        // 2026 Q2 (Apr): big up move.
        for d in 1..=15 {
            let c = 100.5 + d as f64 * 1.0;
            bars.push(mk_hp(&format!("2026-04-{:02}", d), c, c + 0.1, c - 0.1, c));
        }
        let snap = compute_calpb_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.current_year, "2026");
        assert_eq!(snap.current_quarter, "Q2");
        assert!(snap.qtd_pct > 5.0, "expected QTD up, got {}", snap.qtd_pct);
        assert_eq!(snap.momentum_label, "ACCELERATING");
    }

    #[test]
    fn compute_calpb_insufficient() {
        let bars = vec![mk_hp("2026-04-15", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_calpb_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.momentum_label, "INSUFFICIENT_DATA");
    }

    // ── ADR-129 Round 22 tests ──

    #[test]
    fn retskew_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v22(&c).unwrap();
        let snap = ReturnSkewnessSnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            mean_log_return: 0.0005,
            stdev_log_return: 0.012,
            skewness: -0.45,
            positive_return_pct: 52.0,
            largest_up_pct: 4.5,
            largest_down_pct: -6.0,
            skew_label: "LEFT".into(),
            ..Default::default()
        };
        upsert_retskew(&c, "AAA", &snap).unwrap();
        let got = get_retskew(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.skew_label, "LEFT");
        assert!((got.skewness + 0.45).abs() < 1e-9);
    }

    #[test]
    fn retkurt_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v22(&c).unwrap();
        let snap = ReturnKurtosisSnapshot {
            symbol: "BBB".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            excess_kurtosis: 3.5,
            outlier_2sigma_count: 12,
            outlier_3sigma_count: 2,
            outlier_2sigma_pct: 4.74,
            kurt_label: "FAT".into(),
            ..Default::default()
        };
        upsert_retkurt(&c, "BBB", &snap).unwrap();
        let got = get_retkurt(&c, "BBB").unwrap().unwrap();
        assert_eq!(got.kurt_label, "FAT");
    }

    #[test]
    fn tailr_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v22(&c).unwrap();
        let snap = TailRatioSnapshot {
            symbol: "CCC".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            pct_95_return: 2.0,
            pct_05_return: -2.5,
            tail_ratio: 0.8,
            bias_label: "SLIGHT_DOWNSIDE".into(),
            ..Default::default()
        };
        upsert_tailr(&c, "CCC", &snap).unwrap();
        let got = get_tailr(&c, "CCC").unwrap().unwrap();
        assert_eq!(got.bias_label, "SLIGHT_DOWNSIDE");
    }

    #[test]
    fn runlen_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v22(&c).unwrap();
        let snap = RunLengthSnapshot {
            symbol: "DDD".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            avg_up_run: 1.8,
            avg_down_run: 1.6,
            longest_up_run: 5,
            longest_down_run: 4,
            trend_label: "MIXED".into(),
            ..Default::default()
        };
        upsert_runlen(&c, "DDD", &snap).unwrap();
        let got = get_runlen(&c, "DDD").unwrap().unwrap();
        assert_eq!(got.trend_label, "MIXED");
    }

    #[test]
    fn dayrange_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v22(&c).unwrap();
        let snap = DailyRangeSnapshot {
            symbol: "EEE".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            avg_range_60_pct: 1.2,
            avg_range_252_pct: 1.5,
            compression_ratio: 0.8,
            range_label: "COMPRESSED".into(),
            ..Default::default()
        };
        upsert_dayrange(&c, "EEE", &snap).unwrap();
        let got = get_dayrange(&c, "EEE").unwrap().unwrap();
        assert_eq!(got.range_label, "COMPRESSED");
    }

    #[test]
    fn compute_retskew_insufficient() {
        let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_retskew_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.skew_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_retskew_left_tail() {
        // Series with mostly small up-days and a few large down-days → negative skew.
        let mut bars = Vec::new();
        let mut c = 100.0;
        bars.push(mk_hp("2025-01-01", c, c + 0.1, c - 0.1, c));
        for i in 0..200 {
            let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
            let change = if i % 25 == 0 { -8.0 } else { 0.3 }; // occasional large down
            c *= 1.0 + change / 100.0;
            bars.push(mk_hp(&date, c, c + 0.1, c - 0.1, c));
        }
        let snap = compute_retskew_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.skew_label != "INSUFFICIENT_DATA");
        assert!(snap.skewness < -0.2, "expected negative skew, got {}", snap.skewness);
    }

    #[test]
    fn compute_retkurt_fat_tails() {
        // Mostly quiet with rare 5-sigma events → fat-tailed.
        let mut bars = Vec::new();
        let mut c = 100.0;
        bars.push(mk_hp("2025-01-01", c, c + 0.1, c - 0.1, c));
        for i in 0..200 {
            let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
            let change = if i % 40 == 0 { 10.0 } else { 0.05 };
            c *= 1.0 + change / 100.0;
            bars.push(mk_hp(&date, c, c + 0.1, c - 0.1, c));
        }
        let snap = compute_retkurt_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.kurt_label != "INSUFFICIENT_DATA");
        assert!(snap.excess_kurtosis > 1.0, "expected fat-tailed, got {}", snap.excess_kurtosis);
    }

    #[test]
    fn compute_retkurt_insufficient() {
        let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_retkurt_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.kurt_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_tailr_balanced() {
        // Symmetric small random moves → balanced tail ratio.
        let mut bars = Vec::new();
        let mut c = 100.0;
        bars.push(mk_hp("2025-01-01", c, c + 0.1, c - 0.1, c));
        for i in 0..200 {
            let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
            let change = if i % 2 == 0 { 1.0 } else { -1.0 };
            c *= 1.0 + change / 100.0;
            bars.push(mk_hp(&date, c, c + 0.1, c - 0.1, c));
        }
        let snap = compute_tailr_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.bias_label != "INSUFFICIENT_DATA");
        assert!(snap.tail_ratio > 0.5 && snap.tail_ratio < 2.0);
    }

    #[test]
    fn compute_tailr_insufficient() {
        let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_tailr_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.bias_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_runlen_trending() {
        // Monotone up → one long run.
        let mut bars = Vec::new();
        for i in 0..100 {
            let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
            let c = 100.0 + i as f64 * 0.5;
            bars.push(mk_hp(&date, c, c + 0.1, c - 0.1, c));
        }
        let snap = compute_runlen_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.trend_label != "INSUFFICIENT_DATA");
        assert!(snap.longest_up_run >= 30, "expected long up-run, got {}", snap.longest_up_run);
        assert_eq!(snap.longest_down_run, 0);
        assert!(snap.current_run_length > 0);
    }

    #[test]
    fn compute_runlen_choppy() {
        // Alternating up/down → average run length 1.
        let mut bars = Vec::new();
        let mut c = 100.0;
        bars.push(mk_hp("2025-01-01", c, c + 0.1, c - 0.1, c));
        for i in 0..100 {
            let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
            c += if i % 2 == 0 { 1.0 } else { -1.0 };
            bars.push(mk_hp(&date, c, c + 0.1, c - 0.1, c));
        }
        let snap = compute_runlen_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.trend_label, "CHOPPY");
        assert!(snap.avg_up_run < 1.5);
        assert!(snap.avg_down_run < 1.5);
    }

    #[test]
    fn compute_runlen_insufficient() {
        let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_runlen_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.trend_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_dayrange_compressed() {
        // Wide ranges historically, tighter recently → compressed.
        let mut bars = Vec::new();
        for i in 0..253 {
            let date = format!("2025-{:02}-{:02}", (i / 21) + 1, (i % 21) + 1);
            let c = 100.0;
            let (h, l) = if i < 200 {
                (c + 2.0, c - 2.0) // wide historical
            } else {
                (c + 0.3, c - 0.3) // tight recent
            };
            bars.push(mk_hp(&date, c, h, l, c));
        }
        let snap = compute_dayrange_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.range_label == "TIGHT" || snap.range_label == "COMPRESSED",
            "expected compressed, got {}", snap.range_label);
        assert!(snap.compression_ratio < 1.0);
    }

    #[test]
    fn compute_dayrange_expanded() {
        // Tight historical, wide recent → expanded.
        let mut bars = Vec::new();
        for i in 0..253 {
            let date = format!("2025-{:02}-{:02}", (i / 21) + 1, (i % 21) + 1);
            let c = 100.0;
            let (h, l) = if i < 200 {
                (c + 0.3, c - 0.3)
            } else {
                (c + 3.0, c - 3.0)
            };
            bars.push(mk_hp(&date, c, h, l, c));
        }
        let snap = compute_dayrange_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.range_label == "EXPANDED" || snap.range_label == "VERY_EXPANDED",
            "expected expanded, got {}", snap.range_label);
        assert!(snap.compression_ratio > 1.0);
    }

    #[test]
    fn compute_dayrange_insufficient() {
        let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_dayrange_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.range_label, "INSUFFICIENT_DATA");
    }

    // ── ADR-130 Web article ingestion tests ──

    #[test]
    fn ingested_articles_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v23(&c).unwrap();
        let snap = IngestedArticlesSnapshot {
            symbol: "AAPL".into(),
            articles: vec![WebArticle {
                title: "iPhone sales beat".into(),
                url: "https://example.com/a".into(),
                source: "Reuters".into(),
                published_at: "2026-04-10".into(),
                summary: "Strong quarter.".into(),
                agent_used: "claude".into(),
                ingested_at: 1_700_000_000,
            }],
        };
        upsert_ingested_articles(&c, "AAPL", &snap).unwrap();
        let got = get_ingested_articles(&c, "AAPL").unwrap().unwrap();
        assert_eq!(got.articles.len(), 1);
        assert_eq!(got.articles[0].url, "https://example.com/a");
    }

    #[test]
    fn ingested_articles_append_dedupe_and_cap() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v23(&c).unwrap();
        let mk = |url: &str, ts: i64| WebArticle {
            url: url.into(), ingested_at: ts, ..Default::default()
        };
        let batch1 = vec![mk("u1", 100), mk("u2", 110)];
        let (added1, total1) = append_ingested_articles(&c, "AAA", batch1).unwrap();
        assert_eq!(added1, 2);
        assert_eq!(total1, 2);

        // Same URL newer timestamp should replace, not add.
        let batch2 = vec![mk("u1", 200), mk("u3", 210)];
        let (added2, total2) = append_ingested_articles(&c, "AAA", batch2).unwrap();
        assert_eq!(added2, 1);
        assert_eq!(total2, 3);

        // Cap at INGESTED_ARTICLES_MAX: inject 60 more unique URLs.
        let big: Vec<WebArticle> = (0..60).map(|i| mk(&format!("big{}", i), 300 + i)).collect();
        let (_, total3) = append_ingested_articles(&c, "AAA", big).unwrap();
        assert_eq!(total3, INGESTED_ARTICLES_MAX);

        let got = get_ingested_articles(&c, "AAA").unwrap().unwrap();
        // Most recent first: big59 should be at the top.
        assert_eq!(got.articles[0].url, "big59");
    }

    #[test]
    fn parse_ingest_block_extracts_articles() {
        let text = r#"
Some preamble from the agent.

===TYPHOON_INGEST===
[
  {"symbol": "AAPL", "title": "iPhone sales beat", "url": "https://r.com/a",
   "source": "Reuters", "published_at": "2026-04-10", "summary": "Strong.",
   "agent": "claude"},
  {"symbol": "aapl", "title": "Services growth", "url": "https://b.com/b",
   "source": "Bloomberg", "published": "2026-04-11", "summary": "Good.",
   "agent": "claude"},
  {"symbol": "MSFT", "title": "Azure outage", "url": "https://c.com/c",
   "source": "TheVerge", "date": "2026-04-09", "summary": "Brief.",
   "agent": "claude"}
]
===END_INGEST===

Trailing text.
"#;
        let parsed = parse_ingest_block(text);
        let by_sym: std::collections::HashMap<_, _> = parsed.into_iter().collect();
        assert_eq!(by_sym.get("AAPL").map(|v| v.len()), Some(2));
        assert_eq!(by_sym.get("MSFT").map(|v| v.len()), Some(1));
        let msft = &by_sym["MSFT"][0];
        assert_eq!(msft.published_at, "2026-04-09");
        assert_eq!(msft.agent_used, "claude");
    }

    #[test]
    fn parse_ingest_block_with_json_fence() {
        let text = r#"
===TYPHOON_INGEST===
```json
[
  {"symbol": "NVDA", "title": "Blackwell", "url": "https://x.com/n",
   "source": "CNBC", "published_at": "2026-04-12", "summary": "Demand.",
   "agent": "gemini"}
]
```
===END_INGEST===
"#;
        let parsed = parse_ingest_block(text);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].0, "NVDA");
        assert_eq!(parsed[0].1.len(), 1);
    }

    #[test]
    fn parse_ingest_block_skips_malformed_entries() {
        let text = r#"
===TYPHOON_INGEST===
[
  {"symbol": "AAPL"},
  {"url": "https://no-symbol.com/x"},
  {"symbol": "TSLA", "url": "https://good.com/g"}
]
===END_INGEST===
"#;
        let parsed = parse_ingest_block(text);
        let by_sym: std::collections::HashMap<_, _> = parsed.into_iter().collect();
        assert_eq!(by_sym.get("TSLA").map(|v| v.len()), Some(1));
        assert!(!by_sym.contains_key("AAPL"));
    }

    #[test]
    fn parse_ingest_block_returns_empty_when_missing() {
        let text = "No ingest block here.";
        let parsed = parse_ingest_block(text);
        assert!(parsed.is_empty());
    }

    // ── ADR-131 Round 23 tests ──

    fn synthetic_up_trend_bars() -> Vec<HistoricalPriceRow> {
        // 60 bars, each slightly higher than the previous (deterministic
        // drift). Simulates a persistent uptrend: Hurst should be > 0.5,
        // hit rate should be high, GLASYM ratio should be ≥ 1, AUTOCOR
        // lag 1 ~ 0.
        (0..60).map(|i| {
            let close = 100.0 + (i as f64) * 0.5;
            HistoricalPriceRow {
                date: format!("2025-{:02}-{:02}", 1 + (i / 20) as u32, 1 + (i % 20) as u32),
                open: close - 0.25,
                high: close + 0.5,
                low: close - 0.75,
                close,
                adj_close: close,
                volume: 1_000_000.0 + (i as f64) * 50_000.0,
                change: 0.5,
                change_pct: 0.5,
            }
        }).collect()
    }

    fn synthetic_mixed_bars() -> Vec<HistoricalPriceRow> {
        // 60 bars, alternating up/down ~equally — tests the BALANCED /
        // NEUTRAL / RANDOM_WALK paths.
        (0..60).map(|i| {
            let base = 100.0;
            let close = if i % 2 == 0 { base + 1.0 } else { base - 1.0 };
            HistoricalPriceRow {
                date: format!("2025-{:02}-{:02}", 1 + (i / 20) as u32, 1 + (i % 20) as u32),
                open: base,
                high: base + 1.5,
                low: base - 1.5,
                close,
                adj_close: close,
                volume: if i % 2 == 0 { 2_000_000.0 } else { 1_000_000.0 },
                change: 0.0,
                change_pct: 0.0,
            }
        }).collect()
    }

    #[test]
    fn autocor_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        let snap = AutocorrelationSnapshot {
            symbol: "TEST".into(),
            as_of: "2026-04-15".into(),
            bars_used: 200,
            lag1_acf: -0.12,
            lag5_acf: 0.02,
            lag10_acf: -0.01,
            lag20_acf: 0.03,
            mean_log_return: 0.0008,
            regime_label: "MEAN_REVERT".into(),
            note: String::new(),
        };
        upsert_autocor(&c, "TEST", &snap).unwrap();
        let got = get_autocor(&c, "TEST").unwrap().unwrap();
        assert_eq!(got.symbol, "TEST");
        assert!((got.lag1_acf - -0.12).abs() < 1e-9);
        assert_eq!(got.regime_label, "MEAN_REVERT");
    }

    #[test]
    fn autocor_compute_insufficient_data() {
        let snap = compute_autocor_snapshot("X", "2026-04-15", &[]);
        assert_eq!(snap.regime_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn autocor_compute_uptrend_labels() {
        let bars = synthetic_up_trend_bars();
        let snap = compute_autocor_snapshot("X", "2026-04-15", &bars);
        assert_ne!(snap.regime_label, "INSUFFICIENT_DATA");
        assert!(snap.bars_used >= 30);
    }

    #[test]
    fn hurst_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        let snap = HurstSnapshot {
            symbol: "TEST".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            hurst_exponent: 0.58,
            scales_used: 4,
            min_scale: 8,
            max_scale: 64,
            memory_label: "PERSISTENT".into(),
            note: String::new(),
        };
        upsert_hurst(&c, "TEST", &snap).unwrap();
        let got = get_hurst(&c, "TEST").unwrap().unwrap();
        assert!((got.hurst_exponent - 0.58).abs() < 1e-9);
        assert_eq!(got.memory_label, "PERSISTENT");
    }

    #[test]
    fn hurst_compute_insufficient_data() {
        let snap = compute_hurst_snapshot("X", "2026-04-15", &[]);
        assert_eq!(snap.memory_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn hurst_compute_picks_label() {
        let bars = synthetic_mixed_bars();
        let snap = compute_hurst_snapshot("X", "2026-04-15", &bars);
        assert_ne!(snap.memory_label, "INSUFFICIENT_DATA");
        assert!(snap.scales_used >= 2);
    }

    #[test]
    fn hitrate_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        let snap = HitRateSnapshot {
            symbol: "TEST".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            hitrate_5d: 60.0,
            hitrate_20d: 55.0,
            hitrate_60d: 52.0,
            hitrate_252d: 51.0,
            up_days: 130,
            down_days: 120,
            flat_days: 3,
            hit_label: "WEAK_BULLISH".into(),
            note: String::new(),
        };
        upsert_hitrate(&c, "TEST", &snap).unwrap();
        let got = get_hitrate(&c, "TEST").unwrap().unwrap();
        assert_eq!(got.up_days, 130);
        assert_eq!(got.hit_label, "WEAK_BULLISH");
    }

    #[test]
    fn hitrate_compute_uptrend_is_bullish() {
        let bars = synthetic_up_trend_bars();
        let snap = compute_hitrate_snapshot("X", "2026-04-15", &bars);
        assert_ne!(snap.hit_label, "INSUFFICIENT_DATA");
        assert!(snap.up_days > snap.down_days, "uptrend should have more up days");
    }

    #[test]
    fn glasym_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        let snap = GainLossAsymmetrySnapshot {
            symbol: "TEST".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            avg_up_pct: 1.2,
            avg_down_pct: 1.1,
            median_up_pct: 0.9,
            median_down_pct: 0.8,
            magnitude_ratio: 1.09,
            up_days: 130,
            down_days: 120,
            asymmetry_label: "BALANCED".into(),
            note: String::new(),
        };
        upsert_glasym(&c, "TEST", &snap).unwrap();
        let got = get_glasym(&c, "TEST").unwrap().unwrap();
        assert!((got.magnitude_ratio - 1.09).abs() < 1e-9);
        assert_eq!(got.asymmetry_label, "BALANCED");
    }

    #[test]
    fn glasym_compute_insufficient_when_empty() {
        let snap = compute_glasym_snapshot("X", "2026-04-15", &[]);
        assert_eq!(snap.asymmetry_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn glasym_compute_mixed_is_balanced() {
        let bars = synthetic_mixed_bars();
        let snap = compute_glasym_snapshot("X", "2026-04-15", &bars);
        assert_ne!(snap.asymmetry_label, "INSUFFICIENT_DATA");
        assert!(snap.up_days > 0 && snap.down_days > 0);
    }

    #[test]
    fn volratio_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        let snap = VolumeRatioSnapshot {
            symbol: "TEST".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            avg_up_volume: 2_500_000.0,
            avg_down_volume: 2_000_000.0,
            median_up_volume: 2_400_000.0,
            median_down_volume: 1_900_000.0,
            up_down_volume_ratio: 1.25,
            max_up_volume: 8_000_000.0,
            max_down_volume: 5_500_000.0,
            up_days: 130,
            down_days: 120,
            flow_label: "SLIGHT_ACCUMULATION".into(),
            note: String::new(),
        };
        upsert_volratio(&c, "TEST", &snap).unwrap();
        let got = get_volratio(&c, "TEST").unwrap().unwrap();
        assert!((got.up_down_volume_ratio - 1.25).abs() < 1e-9);
        assert_eq!(got.flow_label, "SLIGHT_ACCUMULATION");
    }

    #[test]
    fn volratio_compute_no_volume_returns_insufficient() {
        let bars: Vec<HistoricalPriceRow> = (0..30).map(|i| HistoricalPriceRow {
            date: format!("2025-01-{:02}", i + 1),
            open: 100.0, high: 101.0, low: 99.0, close: 100.0 + i as f64,
            adj_close: 100.0, volume: 0.0, change: 0.0, change_pct: 0.0,
        }).collect();
        let snap = compute_volratio_snapshot("X", "2026-04-15", &bars);
        assert_eq!(snap.flow_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn volratio_compute_with_volume() {
        let bars = synthetic_mixed_bars();
        let snap = compute_volratio_snapshot("X", "2026-04-15", &bars);
        assert_ne!(snap.flow_label, "INSUFFICIENT_DATA");
        assert!(snap.up_days > 0 && snap.down_days > 0);
    }
}
