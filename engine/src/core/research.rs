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

/// Hardcoded commodity-futures universe for the GLCO dashboard.
/// Yahoo continuous-futures tickers, which are free via /v7/finance/quote.
pub const COMMODITIES_UNIVERSE: &[(&str, &str, &str)] = &[
    // Precious metals
    ("GC=F", "Gold",        "Metals"),
    ("SI=F", "Silver",      "Metals"),
    ("PL=F", "Platinum",    "Metals"),
    ("PA=F", "Palladium",   "Metals"),
    ("HG=F", "Copper",      "Metals"),
    // Energy
    ("CL=F", "WTI Crude",   "Energy"),
    ("BZ=F", "Brent Crude", "Energy"),
    ("NG=F", "Natural Gas", "Energy"),
    ("HO=F", "Heating Oil", "Energy"),
    ("RB=F", "Gasoline",    "Energy"),
    // Grains
    ("ZC=F", "Corn",        "Grains"),
    ("ZS=F", "Soybeans",    "Grains"),
    ("ZW=F", "Wheat",       "Grains"),
    ("ZO=F", "Oats",        "Grains"),
    ("ZR=F", "Rice",        "Grains"),
    // Softs
    ("KC=F", "Coffee",      "Softs"),
    ("SB=F", "Sugar",       "Softs"),
    ("CT=F", "Cotton",      "Softs"),
    ("CC=F", "Cocoa",       "Softs"),
    ("OJ=F", "Orange Juice","Softs"),
    // Livestock
    ("LE=F", "Live Cattle", "Livestock"),
    ("HE=F", "Lean Hogs",   "Livestock"),
    ("GF=F", "Feeder Cattle","Livestock"),
];

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
    }

    Ok(())
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
}
