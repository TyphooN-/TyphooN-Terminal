//! Fundamentals scraper — Enterprise Value, earnings dates, dividends, quarterly financials,
//! institutional holders, and company summaries.
//!
//! Data sources:
//! - SEC EDGAR XBRL API: Enterprise Value components (debt, cash from `companyfacts`)
//! - SEC EDGAR: CIK lookup from `company_tickers.json`
//! - Yahoo Finance v8 API: Market cap, shares outstanding, earnings dates, dividends,
//!   quarterly/yearly financials, institutional holders, company description
//!
//! All data is stored in SQLite for offline access and cached between scrapes.
//! The scraper respects SEC rate limits (5 req/sec) and Yahoo rate limits.

use rusqlite::{Connection, params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const SEC_USER_AGENT: &str = "TyphooN-Terminal/0.1 (support@marketwizardry.org)";
const SEC_RATE_LIMIT_MS: u64 = 200;
const YAHOO_RATE_LIMIT_MS: u64 = 300;

// ── Data Types ──────────────────────────────────────────────────────

/// Core fundamentals data for a single symbol.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Fundamentals {
    pub symbol: String,
    pub cik: Option<String>,
    pub company_name: String,
    pub sector: String,
    pub industry: String,
    pub description: String,

    // Enterprise Value components
    pub market_cap: Option<f64>,
    pub enterprise_value: Option<f64>,
    pub total_debt: Option<f64>,
    pub cash_and_equivalents: Option<f64>,
    pub shares_outstanding: Option<f64>,
    pub stock_price: Option<f64>,
    pub mcap_ev_ratio: Option<f64>,

    // Key dates
    pub next_earnings_date: Option<String>,
    pub previous_earnings_date: Option<String>,
    pub next_ex_dividend_date: Option<String>,
    pub next_dividend_payment_date: Option<String>,
    pub last_dividend_payment_date: Option<String>,
    pub is_dividend_stock: bool,
    pub dividend_yield: Option<f64>,

    // Key ratios
    pub pe_ratio: Option<f64>,
    pub forward_pe: Option<f64>,
    pub peg_ratio: Option<f64>,
    pub price_to_book: Option<f64>,
    pub price_to_sales: Option<f64>,
    pub ev_to_ebitda: Option<f64>,
    pub profit_margin: Option<f64>,
    pub operating_margin: Option<f64>,
    pub roe: Option<f64>,
    pub roa: Option<f64>,
    pub beta: Option<f64>,
    pub short_ratio: Option<f64>,
    pub short_percent_of_float: Option<f64>,

    // Metadata
    pub last_updated: String,
}

/// Quarterly financial data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarterlyFinancial {
    pub symbol: String,
    pub period_end: String,  // YYYY-MM-DD
    pub total_revenue: Option<f64>,
    pub net_income: Option<f64>,
    pub free_cash_flow: Option<f64>,
    pub gross_profit: Option<f64>,
    pub operating_income: Option<f64>,
    pub ebitda: Option<f64>,
    pub eps: Option<f64>,
}

/// Institutional holder entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstitutionalHolder {
    pub symbol: String,
    pub holder_name: String,
    pub shares: i64,
    pub pct_held: f64,
    pub value: f64,
    pub date_reported: String,
}

/// Batch scrape result summary.
#[derive(Debug, Clone)]
pub struct ScrapeResult {
    pub symbol: String,
    pub success: bool,
    pub message: String,
}

// ── SQLite Tables ───────────────────────────────────────────────────

pub fn create_fundamentals_tables(conn: &Connection) -> Result<(), String> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS fundamentals (
            symbol TEXT PRIMARY KEY,
            cik TEXT,
            company_name TEXT NOT NULL DEFAULT '',
            sector TEXT NOT NULL DEFAULT '',
            industry TEXT NOT NULL DEFAULT '',
            description TEXT NOT NULL DEFAULT '',
            market_cap REAL,
            enterprise_value REAL,
            total_debt REAL,
            cash_and_equivalents REAL,
            shares_outstanding REAL,
            stock_price REAL,
            mcap_ev_ratio REAL,
            next_earnings_date TEXT,
            previous_earnings_date TEXT,
            next_ex_dividend_date TEXT,
            next_dividend_payment_date TEXT,
            last_dividend_payment_date TEXT,
            is_dividend_stock INTEGER NOT NULL DEFAULT 0,
            dividend_yield REAL,
            pe_ratio REAL,
            forward_pe REAL,
            peg_ratio REAL,
            price_to_book REAL,
            price_to_sales REAL,
            ev_to_ebitda REAL,
            profit_margin REAL,
            operating_margin REAL,
            roe REAL,
            roa REAL,
            beta REAL,
            short_ratio REAL,
            short_percent_of_float REAL,
            last_updated TEXT NOT NULL DEFAULT ''
        );
        CREATE TABLE IF NOT EXISTS quarterly_financials (
            symbol TEXT NOT NULL,
            period_end TEXT NOT NULL,
            total_revenue REAL,
            net_income REAL,
            free_cash_flow REAL,
            gross_profit REAL,
            operating_income REAL,
            ebitda REAL,
            eps REAL,
            PRIMARY KEY (symbol, period_end)
        );
        CREATE TABLE IF NOT EXISTS institutional_holders (
            symbol TEXT NOT NULL,
            holder_name TEXT NOT NULL,
            shares INTEGER NOT NULL DEFAULT 0,
            pct_held REAL NOT NULL DEFAULT 0.0,
            value REAL NOT NULL DEFAULT 0.0,
            date_reported TEXT NOT NULL DEFAULT '',
            PRIMARY KEY (symbol, holder_name)
        );
        CREATE INDEX IF NOT EXISTS idx_fundamentals_earnings ON fundamentals(next_earnings_date);
        CREATE INDEX IF NOT EXISTS idx_fundamentals_dividend ON fundamentals(next_ex_dividend_date);
        CREATE INDEX IF NOT EXISTS idx_quarterly_symbol ON quarterly_financials(symbol);
        CREATE TABLE IF NOT EXISTS scrape_failures (
            symbol TEXT PRIMARY KEY,
            reason TEXT NOT NULL DEFAULT '',
            failed_at TEXT NOT NULL DEFAULT ''
        );
    ").map_err(|e| format!("Create fundamentals tables failed: {e}"))?;

    // Schema migration: add updated_at columns for incremental LAN sync
    let _ = conn.execute("ALTER TABLE fundamentals ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE quarterly_financials ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE institutional_holders ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0", []);

    Ok(())
}

// ── CIK Lookup ──────────────────────────────────────────────────────

/// SEC company_tickers.json entry.
#[derive(Deserialize)]
struct SecCompanyEntry {
    cik_str: u64,
    ticker: String,
}

/// Look up CIK for a ticker from SEC EDGAR.
pub async fn lookup_cik(client: &reqwest::Client, ticker: &str) -> Result<String, String> {
    let url = "https://www.sec.gov/files/company_tickers.json";
    let resp = client.get(url)
        .header("User-Agent", SEC_USER_AGENT)
        .send().await
        .map_err(|e| format!("SEC CIK fetch failed: {e}"))?;

    let data: HashMap<String, SecCompanyEntry> = resp.json().await
        .map_err(|e| format!("SEC CIK parse failed: {e}"))?;

    let upper = ticker.to_uppercase();
    for entry in data.values() {
        if entry.ticker.to_uppercase() == upper {
            return Ok(format!("{:010}", entry.cik_str));
        }
    }
    Err(format!("CIK not found for {ticker}"))
}

// ── SEC EDGAR XBRL — Enterprise Value Components ────────────────────

/// Extract a USD fact value from SEC XBRL companyfacts JSON.
fn extract_usd_fact(facts: &serde_json::Value, concept: &str) -> Option<f64> {
    let units = facts.pointer(&format!("/facts/us-gaap/{concept}/units/USD"))?;
    let arr = units.as_array()?;
    // Find the most recent filing with an 'end' date
    arr.iter()
        .filter_map(|entry| {
            let end = entry.get("end")?.as_str()?;
            let val = entry.get("val")?.as_f64()?;
            Some((end.to_string(), val))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, v)| v)
}

/// Fetch Enterprise Value components from SEC EDGAR XBRL API.
pub async fn fetch_ev_from_sec(
    client: &reqwest::Client,
    cik: &str,
) -> Result<(Option<f64>, Option<f64>), String> {
    // Validate CIK: must be numeric only (prevent URL injection)
    if cik.is_empty() || !cik.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("Invalid CIK: {cik}"));
    }
    let url = format!("https://data.sec.gov/api/xbrl/companyfacts/CIK{cik}.json");
    let resp = client.get(&url)
        .header("User-Agent", SEC_USER_AGENT)
        .send().await
        .map_err(|e| format!("SEC XBRL fetch failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("SEC XBRL returned {}", resp.status()));
    }

    let facts: serde_json::Value = resp.json().await
        .map_err(|e| format!("SEC XBRL parse failed: {e}"))?;

    // Cash
    let cash = extract_usd_fact(&facts, "CashAndCashEquivalentsAtCarryingValue");

    // Debt: try multiple GAAP concepts
    let long_term_cap_lease = extract_usd_fact(&facts, "LongTermDebtAndCapitalLeaseObligations");
    let current_debt = extract_usd_fact(&facts, "DebtAndCapitalLeaseObligationsCurrent");
    let long_term_debt = extract_usd_fact(&facts, "LongTermDebt");
    let short_term_borr = extract_usd_fact(&facts, "ShortTermBorrowings");

    let total_debt = if long_term_cap_lease.is_some() || current_debt.is_some() {
        Some(long_term_cap_lease.unwrap_or(0.0) + current_debt.unwrap_or(0.0))
    } else if long_term_debt.is_some() || short_term_borr.is_some() {
        Some(long_term_debt.unwrap_or(0.0) + short_term_borr.unwrap_or(0.0))
    } else {
        None
    };

    Ok((total_debt, cash))
}

// ── Yahoo Finance API ───────────────────────────────────────────────

/// Yahoo Finance quoteSummary modules we need.
const YAHOO_MODULES: &str = "financialData,defaultKeyStatistics,calendarEvents,summaryProfile,summaryDetail,earningsHistory,institutionOwnership,incomeStatementHistoryQuarterly,cashflowStatementHistoryQuarterly,price";

/// Yahoo Finance session with crumb authentication.
/// Yahoo requires a crumb token (CSRF) obtained from a cookie-authenticated session.
pub struct YahooSession {
    client: reqwest::Client,
    crumb: String,
}

impl YahooSession {
    /// Get the authenticated HTTP client (with cookie jar).
    pub fn client(&self) -> &reqwest::Client { &self.client }
    /// Get the crumb token for API calls.
    pub fn crumb(&self) -> &str { &self.crumb }

    /// Create a new authenticated Yahoo Finance session.
    /// Uses consent-bypass flow to get cookies + crumb token.
    pub async fn new() -> Result<Self, String> {
        // Build a cookie-jar client with redirect following and timeouts
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::limited(10))
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(10))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .build()
            .map_err(|e| format!("Failed to build Yahoo client: {e}"))?;

        // Step 1: Accept consent / get session cookies via fc.yahoo.com
        // This sets the A1/A3 cookies that bypass the EU consent wall
        match client.get("https://fc.yahoo.com").send().await {
            Ok(r) => tracing::info!("Yahoo fc.yahoo.com: status {}", r.status()),
            Err(e) => tracing::warn!("Yahoo fc.yahoo.com failed (non-fatal): {}", e),
        }

        // Step 2: Get crumb directly (the fc.yahoo.com cookies are enough)
        let crumb_resp = client.get("https://query2.finance.yahoo.com/v1/test/getcrumb")
            .header("Accept", "text/plain")
            .send().await
            .map_err(|e| format!("Yahoo crumb fetch failed: {e}"))?;

        let status = crumb_resp.status();
        let crumb = crumb_resp.text().await
            .map_err(|e| format!("Yahoo crumb read failed: {e}"))?;

        if !status.is_success() {
            tracing::warn!("Yahoo crumb returned {} — trying without crumb", status);
            return Ok(Self { client, crumb: String::new() });
        }

        if crumb.is_empty() || crumb.contains('<') || crumb.len() > 50 {
            tracing::warn!("Yahoo crumb looks invalid ({} bytes) — trying without crumb", crumb.len());
            return Ok(Self { client, crumb: String::new() });
        }

        tracing::info!("Yahoo session established (crumb: {}...)", &crumb[..crumb.len().min(6)]);
        Ok(Self { client, crumb })
    }
}

/// Fetch comprehensive fundamentals from Yahoo Finance quoteSummary API.
pub async fn fetch_yahoo_fundamentals(
    session: &YahooSession,
    ticker: &str,
) -> Result<serde_json::Value, String> {
    // Validate ticker: alphanumeric + dots + hyphens only (prevent URL injection)
    if ticker.is_empty() || ticker.len() > 20
        || !ticker.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    {
        return Err(format!("Invalid ticker for Yahoo: {ticker}"));
    }
    let url = if session.crumb.is_empty() {
        format!("https://query2.finance.yahoo.com/v10/finance/quoteSummary/{ticker}?modules={YAHOO_MODULES}")
    } else {
        format!("https://query2.finance.yahoo.com/v10/finance/quoteSummary/{ticker}?modules={YAHOO_MODULES}&crumb={}", session.crumb)
    };
    let resp = session.client.get(&url)
        .header("Accept", "application/json")
        .send().await
        .map_err(|e| format!("Yahoo fetch failed for {ticker}: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Yahoo returned {} for {ticker}", resp.status()));
    }

    let data: serde_json::Value = resp.json().await
        .map_err(|e| format!("Yahoo parse failed for {ticker}: {e}"))?;

    // Navigate to the result
    let result = data.pointer("/quoteSummary/result/0")
        .ok_or_else(|| format!("No Yahoo data for {ticker}"))?;

    Ok(result.clone())
}

/// Helper to extract a raw number from Yahoo's nested {"raw": 123.45} format.
fn yf_raw(val: &serde_json::Value, path: &str) -> Option<f64> {
    val.pointer(path)?.get("raw")?.as_f64()
}

/// Helper to extract a string from Yahoo's nested {"fmt": "2026-04-15"} format.
fn yf_fmt(val: &serde_json::Value, path: &str) -> Option<String> {
    val.pointer(path)?.get("fmt")?.as_str().map(|s| s.to_string())
}

/// Parse Yahoo Finance JSON into Fundamentals struct.
pub fn parse_yahoo_data(ticker: &str, yahoo: &serde_json::Value) -> Fundamentals {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let mut f = Fundamentals {
        symbol: ticker.to_uppercase(),
        last_updated: now,
        ..Default::default()
    };

    // summaryProfile
    if let Some(p) = yahoo.get("summaryProfile") {
        f.sector = p.get("sector").and_then(|v| v.as_str()).unwrap_or("").to_string();
        f.industry = p.get("industry").and_then(|v| v.as_str()).unwrap_or("").to_string();
        f.description = p.get("longBusinessSummary").and_then(|v| v.as_str()).unwrap_or("").to_string();
    }

    // price module
    if let Some(p) = yahoo.get("price") {
        f.company_name = p.get("shortName").and_then(|v| v.as_str()).unwrap_or("").to_string();
        f.market_cap = yf_raw(p, "/marketCap");
        f.stock_price = yf_raw(p, "/regularMarketPrice");
    }

    // defaultKeyStatistics
    if let Some(ks) = yahoo.get("defaultKeyStatistics") {
        f.enterprise_value = yf_raw(ks, "/enterpriseValue");
        f.shares_outstanding = yf_raw(ks, "/sharesOutstanding");
        f.pe_ratio = yf_raw(ks, "/trailingEps").and_then(|eps| {
            f.stock_price.map(|p| if eps != 0.0 { p / eps } else { 0.0 })
        });
        f.forward_pe = yf_raw(ks, "/forwardPE");
        f.peg_ratio = yf_raw(ks, "/pegRatio");
        f.price_to_book = yf_raw(ks, "/priceToBook");
        f.beta = yf_raw(ks, "/beta");
        f.short_ratio = yf_raw(ks, "/shortRatio");
        f.short_percent_of_float = yf_raw(ks, "/shortPercentOfFloat");
    }

    // summaryDetail
    if let Some(sd) = yahoo.get("summaryDetail") {
        f.dividend_yield = yf_raw(sd, "/dividendYield");
        f.pe_ratio = f.pe_ratio.or_else(|| yf_raw(sd, "/trailingPE"));
        f.forward_pe = f.forward_pe.or_else(|| yf_raw(sd, "/forwardPE"));
        f.price_to_sales = yf_raw(sd, "/priceToSalesTrailing12Months");
        // Check if pays dividends
        if let Some(rate) = yf_raw(sd, "/dividendRate") {
            f.is_dividend_stock = rate > 0.0;
        }
    }

    // financialData
    if let Some(fd) = yahoo.get("financialData") {
        f.profit_margin = yf_raw(fd, "/profitMargins");
        f.operating_margin = yf_raw(fd, "/operatingMargins");
        f.roe = yf_raw(fd, "/returnOnEquity");
        f.roa = yf_raw(fd, "/returnOnAssets");
        f.total_debt = f.total_debt.or_else(|| yf_raw(fd, "/totalDebt"));
        f.cash_and_equivalents = f.cash_and_equivalents.or_else(|| yf_raw(fd, "/totalCash"));
        f.ev_to_ebitda = yf_raw(fd, "/enterpriseToEbitda");
    }

    // EV components: prefer SEC XBRL (filled later), fallback to Yahoo
    if f.enterprise_value.is_none() {
        if let (Some(mc), Some(debt), Some(cash)) = (f.market_cap, f.total_debt, f.cash_and_equivalents) {
            f.enterprise_value = Some(mc + debt - cash);
        }
    }
    // MCap/EV ratio
    if let (Some(mc), Some(ev)) = (f.market_cap, f.enterprise_value) {
        if ev > 0.0 {
            f.mcap_ev_ratio = Some(mc / ev * 100.0);
        }
    }

    // calendarEvents — earnings & dividends
    if let Some(cal) = yahoo.get("calendarEvents") {
        // Earnings dates
        if let Some(earnings) = cal.pointer("/earnings/earningsDate") {
            if let Some(arr) = earnings.as_array() {
                let today = chrono::Utc::now().date_naive();
                for entry in arr {
                    if let Some(fmt) = entry.get("fmt").and_then(|v| v.as_str()) {
                        if let Ok(d) = chrono::NaiveDate::parse_from_str(fmt, "%Y-%m-%d") {
                            if d > today {
                                f.next_earnings_date = Some(fmt.to_string());
                                break;
                            } else {
                                f.previous_earnings_date = Some(fmt.to_string());
                            }
                        }
                    }
                }
            }
        }
        // Dividend dates
        f.next_ex_dividend_date = yf_fmt(cal, "/exDividendDate");
        f.next_dividend_payment_date = yf_fmt(cal, "/dividendDate");
    }

    f
}

/// Parse quarterly financials from Yahoo Finance JSON.
pub fn parse_quarterly_financials(ticker: &str, yahoo: &serde_json::Value) -> Vec<QuarterlyFinancial> {
    let mut results = Vec::new();

    // Income statement quarterly
    let income_stmts = yahoo.pointer("/incomeStatementHistoryQuarterly/incomeStatementHistory");
    let cashflow_stmts = yahoo.pointer("/cashflowStatementHistoryQuarterly/cashflowStatements");

    // Build cashflow lookup by end date
    let mut cf_map: HashMap<String, &serde_json::Value> = HashMap::new();
    if let Some(arr) = cashflow_stmts.and_then(|v| v.as_array()) {
        for entry in arr {
            if let Some(date) = entry.pointer("/endDate/fmt").and_then(|v| v.as_str()) {
                cf_map.insert(date.to_string(), entry);
            }
        }
    }

    if let Some(arr) = income_stmts.and_then(|v| v.as_array()) {
        for entry in arr {
            let period_end = match entry.pointer("/endDate/fmt").and_then(|v| v.as_str()) {
                Some(d) => d.to_string(),
                None => continue,
            };

            let cf = cf_map.get(&period_end).copied();

            let mut q = QuarterlyFinancial {
                symbol: ticker.to_uppercase(),
                period_end,
                total_revenue: yf_raw(entry, "/totalRevenue"),
                net_income: yf_raw(entry, "/netIncome"),
                gross_profit: yf_raw(entry, "/grossProfit"),
                operating_income: yf_raw(entry, "/operatingIncome"),
                ebitda: yf_raw(entry, "/ebitda"),
                eps: yf_raw(entry, "/dilutedEPS").or_else(|| yf_raw(entry, "/basicEPS")),
                free_cash_flow: None,
            };

            // Free cash flow from cashflow statement
            if let Some(cf_entry) = cf {
                let op_cf = yf_raw(cf_entry, "/totalCashFromOperatingActivities");
                let capex = yf_raw(cf_entry, "/capitalExpenditures");
                if let (Some(op), Some(cx)) = (op_cf, capex) {
                    q.free_cash_flow = Some(op - cx.abs());
                }
            }

            results.push(q);
        }
    }

    results
}

/// Parse institutional holders from Yahoo Finance JSON.
pub fn parse_institutional_holders(ticker: &str, yahoo: &serde_json::Value) -> Vec<InstitutionalHolder> {
    let mut results = Vec::new();

    if let Some(inst) = yahoo.pointer("/institutionOwnership/ownershipList") {
        if let Some(arr) = inst.as_array() {
            for entry in arr {
                let name = entry.pointer("/organization")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string();
                let shares = entry.pointer("/position/raw")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let pct = entry.pointer("/pctHeld/raw")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let value = entry.pointer("/value/raw")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let date = entry.pointer("/reportDate/fmt")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                results.push(InstitutionalHolder {
                    symbol: ticker.to_uppercase(),
                    holder_name: name,
                    shares,
                    pct_held: pct,
                    value,
                    date_reported: date,
                });
            }
        }
    }

    results
}

// ── SQLite Storage ──────────────────────────────────────────────────

/// Store or update fundamentals for a symbol.
pub fn upsert_fundamentals(conn: &Connection, f: &Fundamentals) -> Result<(), String> {
    conn.execute(
        "INSERT INTO fundamentals (
            symbol, cik, company_name, sector, industry, description,
            market_cap, enterprise_value, total_debt, cash_and_equivalents,
            shares_outstanding, stock_price, mcap_ev_ratio,
            next_earnings_date, previous_earnings_date,
            next_ex_dividend_date, next_dividend_payment_date, last_dividend_payment_date,
            is_dividend_stock, dividend_yield,
            pe_ratio, forward_pe, peg_ratio, price_to_book, price_to_sales,
            ev_to_ebitda, profit_margin, operating_margin, roe, roa,
            beta, short_ratio, short_percent_of_float, last_updated, updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6,
            ?7, ?8, ?9, ?10, ?11, ?12, ?13,
            ?14, ?15, ?16, ?17, ?18, ?19, ?20,
            ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30,
            ?31, ?32, ?33, ?34, ?35
        ) ON CONFLICT(symbol) DO UPDATE SET
            cik=excluded.cik, company_name=excluded.company_name,
            sector=excluded.sector, industry=excluded.industry,
            description=excluded.description,
            market_cap=excluded.market_cap, enterprise_value=excluded.enterprise_value,
            total_debt=excluded.total_debt, cash_and_equivalents=excluded.cash_and_equivalents,
            shares_outstanding=excluded.shares_outstanding, stock_price=excluded.stock_price,
            mcap_ev_ratio=excluded.mcap_ev_ratio,
            next_earnings_date=excluded.next_earnings_date,
            previous_earnings_date=excluded.previous_earnings_date,
            next_ex_dividend_date=excluded.next_ex_dividend_date,
            next_dividend_payment_date=excluded.next_dividend_payment_date,
            last_dividend_payment_date=excluded.last_dividend_payment_date,
            is_dividend_stock=excluded.is_dividend_stock,
            dividend_yield=excluded.dividend_yield,
            pe_ratio=excluded.pe_ratio, forward_pe=excluded.forward_pe,
            peg_ratio=excluded.peg_ratio, price_to_book=excluded.price_to_book,
            price_to_sales=excluded.price_to_sales, ev_to_ebitda=excluded.ev_to_ebitda,
            profit_margin=excluded.profit_margin, operating_margin=excluded.operating_margin,
            roe=excluded.roe, roa=excluded.roa, beta=excluded.beta,
            short_ratio=excluded.short_ratio, short_percent_of_float=excluded.short_percent_of_float,
            last_updated=excluded.last_updated, updated_at=excluded.updated_at",
        params![
            f.symbol, f.cik, f.company_name, f.sector, f.industry, f.description,
            f.market_cap, f.enterprise_value, f.total_debt, f.cash_and_equivalents,
            f.shares_outstanding, f.stock_price, f.mcap_ev_ratio,
            f.next_earnings_date, f.previous_earnings_date,
            f.next_ex_dividend_date, f.next_dividend_payment_date, f.last_dividend_payment_date,
            f.is_dividend_stock as i32, f.dividend_yield,
            f.pe_ratio, f.forward_pe, f.peg_ratio, f.price_to_book, f.price_to_sales,
            f.ev_to_ebitda, f.profit_margin, f.operating_margin, f.roe, f.roa,
            f.beta, f.short_ratio, f.short_percent_of_float, f.last_updated,
            chrono::Utc::now().timestamp(),
        ],
    ).map_err(|e| format!("Upsert fundamentals failed: {e}"))?;
    Ok(())
}

/// Store quarterly financials (replace all for a symbol).
pub fn upsert_quarterly(conn: &Connection, quarters: &[QuarterlyFinancial]) -> Result<(), String> {
    for q in quarters {
        conn.execute(
            "INSERT INTO quarterly_financials (symbol, period_end, total_revenue, net_income,
             free_cash_flow, gross_profit, operating_income, ebitda, eps, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(symbol, period_end) DO UPDATE SET
                total_revenue=excluded.total_revenue, net_income=excluded.net_income,
                free_cash_flow=excluded.free_cash_flow, gross_profit=excluded.gross_profit,
                operating_income=excluded.operating_income, ebitda=excluded.ebitda,
                eps=excluded.eps, updated_at=excluded.updated_at",
            params![
                q.symbol, q.period_end, q.total_revenue, q.net_income,
                q.free_cash_flow, q.gross_profit, q.operating_income, q.ebitda, q.eps,
                chrono::Utc::now().timestamp(),
            ],
        ).map_err(|e| format!("Upsert quarterly failed: {e}"))?;
    }
    Ok(())
}

/// Store institutional holders (replace all for a symbol).
pub fn upsert_holders(conn: &Connection, holders: &[InstitutionalHolder]) -> Result<(), String> {
    if let Some(first) = holders.first() {
        conn.execute("DELETE FROM institutional_holders WHERE symbol = ?1", params![first.symbol])
            .map_err(|e| format!("Delete holders failed: {e}"))?;
    }
    for h in holders {
        conn.execute(
            "INSERT INTO institutional_holders (symbol, holder_name, shares, pct_held, value, date_reported, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![h.symbol, h.holder_name, h.shares, h.pct_held, h.value, h.date_reported,
                    chrono::Utc::now().timestamp()],
        ).map_err(|e| format!("Insert holder failed: {e}"))?;
    }
    Ok(())
}

// ── Query Functions ─────────────────────────────────────────────────

/// Get fundamentals for a single symbol.
pub fn get_fundamentals(conn: &Connection, symbol: &str) -> Result<Option<Fundamentals>, String> {
    let mut stmt = conn.prepare(
        "SELECT symbol, cik, company_name, sector, industry, description,
                market_cap, enterprise_value, total_debt, cash_and_equivalents,
                shares_outstanding, stock_price, mcap_ev_ratio,
                next_earnings_date, previous_earnings_date,
                next_ex_dividend_date, next_dividend_payment_date, last_dividend_payment_date,
                is_dividend_stock, dividend_yield,
                pe_ratio, forward_pe, peg_ratio, price_to_book, price_to_sales,
                ev_to_ebitda, profit_margin, operating_margin, roe, roa,
                beta, short_ratio, short_percent_of_float, last_updated
         FROM fundamentals WHERE symbol = ?1"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    let result = stmt.query_row(params![symbol.to_uppercase()], |row| {
        Ok(Fundamentals {
            symbol: row.get(0)?,
            cik: row.get(1)?,
            company_name: row.get(2)?,
            sector: row.get(3)?,
            industry: row.get(4)?,
            description: row.get(5)?,
            market_cap: row.get(6)?,
            enterprise_value: row.get(7)?,
            total_debt: row.get(8)?,
            cash_and_equivalents: row.get(9)?,
            shares_outstanding: row.get(10)?,
            stock_price: row.get(11)?,
            mcap_ev_ratio: row.get(12)?,
            next_earnings_date: row.get(13)?,
            previous_earnings_date: row.get(14)?,
            next_ex_dividend_date: row.get(15)?,
            next_dividend_payment_date: row.get(16)?,
            last_dividend_payment_date: row.get(17)?,
            is_dividend_stock: row.get::<_, i32>(18)? != 0,
            dividend_yield: row.get(19)?,
            pe_ratio: row.get(20)?,
            forward_pe: row.get(21)?,
            peg_ratio: row.get(22)?,
            price_to_book: row.get(23)?,
            price_to_sales: row.get(24)?,
            ev_to_ebitda: row.get(25)?,
            profit_margin: row.get(26)?,
            operating_margin: row.get(27)?,
            roe: row.get(28)?,
            roa: row.get(29)?,
            beta: row.get(30)?,
            short_ratio: row.get(31)?,
            short_percent_of_float: row.get(32)?,
            last_updated: row.get(33)?,
        })
    });

    match result {
        Ok(f) => Ok(Some(f)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(format!("Query fundamentals failed: {e}")),
    }
}

/// Get all fundamentals (for EV scanner table).
pub fn get_all_fundamentals(conn: &Connection) -> Result<Vec<Fundamentals>, String> {
    let mut stmt = conn.prepare(
        "SELECT symbol, cik, company_name, sector, industry, '',
                market_cap, enterprise_value, total_debt, cash_and_equivalents,
                shares_outstanding, stock_price, mcap_ev_ratio,
                next_earnings_date, previous_earnings_date,
                next_ex_dividend_date, next_dividend_payment_date, last_dividend_payment_date,
                is_dividend_stock, dividend_yield,
                pe_ratio, forward_pe, peg_ratio, price_to_book, price_to_sales,
                ev_to_ebitda, profit_margin, operating_margin, roe, roa,
                beta, short_ratio, short_percent_of_float, last_updated
         FROM fundamentals ORDER BY symbol"
    ).map_err(|e| format!("Prepare all fundamentals failed: {e}"))?;

    let rows = stmt.query_map([], |row| {
        Ok(Fundamentals {
            symbol: row.get(0)?,
            cik: row.get(1)?,
            company_name: row.get(2)?,
            sector: row.get(3)?,
            industry: row.get(4)?,
            description: row.get(5)?,
            market_cap: row.get(6)?,
            enterprise_value: row.get(7)?,
            total_debt: row.get(8)?,
            cash_and_equivalents: row.get(9)?,
            shares_outstanding: row.get(10)?,
            stock_price: row.get(11)?,
            mcap_ev_ratio: row.get(12)?,
            next_earnings_date: row.get(13)?,
            previous_earnings_date: row.get(14)?,
            next_ex_dividend_date: row.get(15)?,
            next_dividend_payment_date: row.get(16)?,
            last_dividend_payment_date: row.get(17)?,
            is_dividend_stock: row.get::<_, i32>(18)? != 0,
            dividend_yield: row.get(19)?,
            pe_ratio: row.get(20)?,
            forward_pe: row.get(21)?,
            peg_ratio: row.get(22)?,
            price_to_book: row.get(23)?,
            price_to_sales: row.get(24)?,
            ev_to_ebitda: row.get(25)?,
            profit_margin: row.get(26)?,
            operating_margin: row.get(27)?,
            roe: row.get(28)?,
            roa: row.get(29)?,
            beta: row.get(30)?,
            short_ratio: row.get(31)?,
            short_percent_of_float: row.get(32)?,
            last_updated: row.get(33)?,
        })
    }).map_err(|e| format!("Query all fundamentals failed: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect fundamentals failed: {e}"))
}

/// Get upcoming earnings dates sorted by date.
pub fn get_upcoming_earnings(conn: &Connection, limit: usize) -> Result<Vec<(String, String, String)>, String> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let mut stmt = conn.prepare(
        "SELECT symbol, company_name, next_earnings_date FROM fundamentals
         WHERE next_earnings_date IS NOT NULL AND next_earnings_date >= ?1
         ORDER BY next_earnings_date ASC LIMIT ?2"
    ).map_err(|e| format!("Prepare earnings query failed: {e}"))?;

    let rows = stmt.query_map(params![today, limit as i64], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
    }).map_err(|e| format!("Query earnings failed: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect earnings failed: {e}"))
}

/// Get upcoming ex-dividend dates sorted by date.
pub fn get_upcoming_dividends(conn: &Connection, limit: usize) -> Result<Vec<(String, String, String, Option<f64>)>, String> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let mut stmt = conn.prepare(
        "SELECT symbol, company_name, next_ex_dividend_date, dividend_yield FROM fundamentals
         WHERE next_ex_dividend_date IS NOT NULL AND next_ex_dividend_date >= ?1
         AND is_dividend_stock = 1
         ORDER BY next_ex_dividend_date ASC LIMIT ?2"
    ).map_err(|e| format!("Prepare dividend query failed: {e}"))?;

    let rows = stmt.query_map(params![today, limit as i64], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, Option<f64>>(3)?))
    }).map_err(|e| format!("Query dividends failed: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect dividends failed: {e}"))
}

/// Get quarterly financials for a symbol.
pub fn get_quarterly_financials(conn: &Connection, symbol: &str) -> Result<Vec<QuarterlyFinancial>, String> {
    let mut stmt = conn.prepare(
        "SELECT symbol, period_end, total_revenue, net_income, free_cash_flow,
                gross_profit, operating_income, ebitda, eps
         FROM quarterly_financials WHERE symbol = ?1 ORDER BY period_end DESC LIMIT 8"
    ).map_err(|e| format!("Prepare quarterly query failed: {e}"))?;

    let rows = stmt.query_map(params![symbol.to_uppercase()], |row| {
        Ok(QuarterlyFinancial {
            symbol: row.get(0)?,
            period_end: row.get(1)?,
            total_revenue: row.get(2)?,
            net_income: row.get(3)?,
            free_cash_flow: row.get(4)?,
            gross_profit: row.get(5)?,
            operating_income: row.get(6)?,
            ebitda: row.get(7)?,
            eps: row.get(8)?,
        })
    }).map_err(|e| format!("Query quarterly failed: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect quarterly failed: {e}"))
}

/// Get institutional holders for a symbol.
pub fn get_institutional_holders(conn: &Connection, symbol: &str) -> Result<Vec<InstitutionalHolder>, String> {
    let mut stmt = conn.prepare(
        "SELECT symbol, holder_name, shares, pct_held, value, date_reported
         FROM institutional_holders WHERE symbol = ?1 ORDER BY shares DESC"
    ).map_err(|e| format!("Prepare holders query failed: {e}"))?;

    let rows = stmt.query_map(params![symbol.to_uppercase()], |row| {
        Ok(InstitutionalHolder {
            symbol: row.get(0)?,
            holder_name: row.get(1)?,
            shares: row.get(2)?,
            pct_held: row.get(3)?,
            value: row.get(4)?,
            date_reported: row.get(5)?,
        })
    }).map_err(|e| format!("Query holders failed: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect holders failed: {e}"))
}

// ── Full Scrape Orchestrator ────────────────────────────────────────

/// Scrape fundamentals for a single ticker (Yahoo + optionally SEC XBRL for EV).
pub async fn scrape_ticker(
    session: &YahooSession,
    conn: &Connection,
    ticker: &str,
) -> Result<Fundamentals, String> {
    // 1. Yahoo Finance (primary source for everything)
    let yahoo = fetch_yahoo_fundamentals(session, ticker).await?;
    let mut fund = parse_yahoo_data(ticker, &yahoo);

    // 2. SEC XBRL for more accurate EV components (optional, may fail for non-US)
    if let Ok(cik) = lookup_cik(&session.client, ticker).await {
        fund.cik = Some(cik.clone());
        tokio::time::sleep(std::time::Duration::from_millis(SEC_RATE_LIMIT_MS)).await;
        if let Ok((sec_debt, sec_cash)) = fetch_ev_from_sec(&session.client, &cik).await {
            // Prefer SEC XBRL values over Yahoo for EV accuracy
            if let Some(d) = sec_debt { fund.total_debt = Some(d); }
            if let Some(c) = sec_cash { fund.cash_and_equivalents = Some(c); }
            // Recalculate EV with SEC data
            if let (Some(mc), Some(debt), Some(cash)) = (fund.market_cap, fund.total_debt, fund.cash_and_equivalents) {
                fund.enterprise_value = Some(mc + debt - cash);
                if let Some(ev) = fund.enterprise_value {
                    if ev > 0.0 {
                        fund.mcap_ev_ratio = Some(mc / ev * 100.0);
                    }
                }
            }
        }
    }

    // 3. Parse quarterly financials
    let quarters = parse_quarterly_financials(ticker, &yahoo);

    // 4. Parse institutional holders
    let holders = parse_institutional_holders(ticker, &yahoo);

    // 5. Store everything
    upsert_fundamentals(conn, &fund)?;
    upsert_quarterly(conn, &quarters)?;
    upsert_holders(conn, &holders)?;

    Ok(fund)
}

/// Batch scrape fundamentals for multiple tickers.
/// Skips currency pairs (contains '/') and tickers that were updated within `skip_if_recent_hours`.
pub async fn scrape_batch(
    session: &YahooSession,
    conn: &Connection,
    tickers: &[String],
    skip_if_recent_hours: u64,
    progress_tx: Option<&tokio::sync::mpsc::UnboundedSender<ScrapeResult>>,
) -> Result<Vec<ScrapeResult>, String> {
    create_fundamentals_tables(conn)?;

    let cutoff = if skip_if_recent_hours > 0 {
        let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(skip_if_recent_hours as i64);
        Some(cutoff_time.format("%Y-%m-%dT%H:%M:%SZ").to_string())
    } else {
        None
    };

    let mut results = Vec::new();

    for ticker in tickers {
        let ticker = ticker.trim().to_uppercase();

        // Skip forex pairs and indices
        if ticker.contains('/') || ticker.contains('#') || ticker.is_empty() {
            continue;
        }

        // Skip permanently failed symbols (404 from Yahoo — won't magically start working)
        {
            let check: Result<Option<String>, _> = conn.query_row(
                "SELECT reason FROM scrape_failures WHERE symbol = ?1",
                rusqlite::params![&ticker],
                |row| row.get(0),
            ).optional();
            if let Ok(Some(_)) = check {
                continue; // permanently blocklisted
            }
        }

        // Skip if recently updated
        if let Some(ref cutoff_str) = cutoff {
            if let Ok(Some(existing)) = get_fundamentals(conn, &ticker) {
                if existing.last_updated >= *cutoff_str {
                    let r = ScrapeResult {
                        symbol: ticker.clone(),
                        success: true,
                        message: "Skipped (recently updated)".into(),
                    };
                    if let Some(tx) = progress_tx { let _ = tx.send(r.clone()); }
                    results.push(r);
                    continue;
                }
            }
        }

        // Rate limit
        tokio::time::sleep(std::time::Duration::from_millis(YAHOO_RATE_LIMIT_MS)).await;

        match scrape_ticker(session, conn, &ticker).await {
            Ok(_fund) => {
                let r = ScrapeResult {
                    symbol: ticker.clone(),
                    success: true,
                    message: format!("OK"),
                };
                if let Some(tx) = progress_tx { let _ = tx.send(r.clone()); }
                results.push(r);
            }
            Err(e) => {
                // Record permanent failures (404 Not Found) so we skip them next time
                if e.contains("404") || e.contains("Not Found") {
                    let _ = conn.execute(
                        "INSERT OR REPLACE INTO scrape_failures (symbol, reason, failed_at) VALUES (?1, ?2, datetime('now'))",
                        rusqlite::params![&ticker, &e],
                    );
                }
                let r = ScrapeResult {
                    symbol: ticker.clone(),
                    success: false,
                    message: e,
                };
                if let Some(tx) = progress_tx { let _ = tx.send(r.clone()); }
                results.push(r);
            }
        }
    }

    Ok(results)
}

/// Extract unique stock tickers from Darwinex MT5 cache keys.
/// Cache keys look like "mt5:CC:SLV:4Hour" — we extract "SLV".
/// Filters out known currency pairs, indices, and crypto.
pub fn extract_stock_tickers_from_cache(conn: &Connection) -> Result<Vec<String>, String> {
    // Only query MT5-sourced keys — Alpaca/Kraken/CryptoCompare keys are separate sources
    let mut stmt = conn.prepare("SELECT DISTINCT key FROM bar_cache WHERE key LIKE 'mt5:%'")
        .map_err(|e| format!("Prepare cache keys failed: {e}"))?;

    let keys: Vec<String> = stmt.query_map([], |row| row.get(0))
        .map_err(|e| format!("Query cache keys failed: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    let mut symbols: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Known non-stock patterns to skip
    let forex_suffixes = ["USD", "JPY", "GBP", "EUR", "CHF", "CAD", "AUD", "NZD", "SEK", "NOK", "HKD", "SGD", "TRY", "MXN", "ZAR", "PLN", "CZK", "HUF"];
    // Crypto symbols (USD-denominated pairs)
    let crypto_patterns = ["BTC", "ETH", "SOL", "DOGE", "XRP", "ADA", "LTC", "LINK", "AVAX", "DOT",
        "UNI", "AAVE", "MATIC", "SHIB", "FIL", "ATOM", "NEAR", "APE", "SAND", "MANA",
        "CRV", "COMP", "MKR", "SNX", "GRT", "BAT", "1INCH", "SUSHI", "YFI", "ENJ"];
    // Futures contract suffixes (e.g., 6C_M, GC_M, CL_M, ES_M, NQ_M)
    let futures_suffixes = ["_M", "_H", "_U", "_Z", "_F", "_G", "_J", "_K", "_N", "_Q", "_V", "_X"];
    // Known futures root symbols
    let futures_roots = ["6A", "6B", "6C", "6E", "6J", "6M", "6N", "6S", "6Z",
        "GC", "SI", "HG", "PL", "PA", "CL", "NG", "HO", "RB", "BZ",
        "ES", "NQ", "YM", "RTY", "EMD", "ZB", "ZN", "ZF", "ZT",
        "ZC", "ZW", "ZS", "ZM", "ZL", "CT", "KC", "SB", "CC", "OJ",
        "LE", "HE", "GF", "DX", "VX"];

    for key in &keys {
        // Parse "mt5:CC:SLV:4Hour" → parts = ["mt5", "CC", "SLV", "4Hour"]
        let parts: Vec<&str> = key.split(':').collect();
        if parts.len() >= 3 {
            let sym = if parts[0] == "mt5" && parts.len() >= 4 {
                parts[2] // "mt5:CC:SLV:4Hour" → "SLV"
            } else {
                parts[1] // "CC:SLV:4Hour" → "SLV"
            };

            let sym_upper = sym.to_uppercase();

            // Skip internal/meta keys (BarCacheWriter stores __SERVER__, __SPECS__, __SYMBOLS__)
            if sym.starts_with("__") || sym.starts_with("_") && sym.ends_with("_") {
                continue;
            }
            // Skip forex (pairs like EURUSD, GBPJPY, NZDUSD — any length ending in currency code)
            if forex_suffixes.iter().any(|s| sym_upper.ends_with(s) && sym_upper.len() >= 5 && sym_upper.len() <= 7) {
                continue;
            }
            // Skip symbols starting with currency code + another currency (AUDCAD, NZDCHF, etc.)
            let forex_prefixes = ["AUD", "CAD", "CHF", "EUR", "GBP", "JPY", "NZD", "USD", "SEK", "NOK", "TRY", "MXN", "ZAR", "PLN", "HKD", "SGD", "CZK", "HUF"];
            if forex_prefixes.iter().any(|p| sym_upper.starts_with(p) && sym_upper.len() >= 6 && sym_upper.len() <= 7
                && forex_suffixes.iter().any(|s| sym_upper.ends_with(s))) {
                continue;
            }
            // Skip crypto — exact match or with USD/USDT suffix
            if crypto_patterns.iter().any(|c| {
                sym_upper == *c
                || sym_upper.starts_with(c) && (sym_upper.ends_with("USD") || sym_upper.ends_with("USDT") || sym_upper.ends_with("BTC") || sym_upper.ends_with("ETH"))
            }) {
                continue;
            }
            // Skip futures contracts (contain _M, _H, _U, _Z suffixes or known roots with underscore)
            if futures_suffixes.iter().any(|s| sym_upper.ends_with(s)) {
                continue;
            }
            // Skip known futures root symbols (exact match or with digits)
            if futures_roots.iter().any(|r| sym_upper == *r || (sym_upper.starts_with(r) && sym_upper.len() <= r.len() + 2 && sym_upper[r.len()..].chars().all(|c| c.is_ascii_digit()))) {
                continue;
            }
            // Skip indices (start with #, ., or are known index names)
            if sym.starts_with('#') || sym.starts_with('.') {
                continue;
            }
            // Skip symbols with only digits (contract codes)
            if sym_upper.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            // Skip very short symbols (likely not stocks)
            if sym_upper.len() < 1 {
                continue;
            }
            // Skip CFDs, metals, indices, and non-Yahoo symbols
            let skip_exact = ["XNGUSD", "XNG", "XAGUSD", "XAUUSD", "XPDUSD", "XPTUSD",
                "AUS200", "NI225", "SP500", "STOXX50E", "GDAXI", "SPA35", "FCHI40",
                "FTSE100", "DAX40", "CAC40", "NIKKEI", "HSI", "KOSPI", "MASI",
                "US30", "US500", "US2000", "USTEC", "JP225", "UK100", "DE40", "FR40",
                "EU50", "HK50", "CN50", "PAPER", "TPL"];
            if skip_exact.iter().any(|&s| sym_upper == s) {
                continue;
            }
            // Skip symbols with digits in them (likely indices: AUS200, NI225, etc.)
            if sym_upper.len() > 3 && sym_upper.chars().any(|c| c.is_ascii_digit()) && !sym_upper.chars().all(|c| c.is_ascii_uppercase()) {
                // Has mixed letters+digits and is longer than 3 chars — likely an index
                let letter_count = sym_upper.chars().filter(|c| c.is_ascii_alphabetic()).count();
                let digit_count = sym_upper.chars().filter(|c| c.is_ascii_digit()).count();
                if digit_count >= 2 && letter_count >= 2 { continue; } // e.g., AUS200, NI225, SP500
            }

            symbols.insert(sym_upper);
        }
    }

    let mut sorted: Vec<String> = symbols.into_iter().collect();
    sorted.sort();
    Ok(sorted)
}

// ── Formatting Helpers ──────────────────────────────────────────────

/// Format a large number into human-readable string (T, B, M, K).
pub fn format_large_number(num: f64) -> String {
    let abs = num.abs();
    if abs >= 1_000_000_000_000.0 {
        format!("${:.2}T", num / 1_000_000_000_000.0)
    } else if abs >= 1_000_000_000.0 {
        format!("${:.2}B", num / 1_000_000_000.0)
    } else if abs >= 1_000_000.0 {
        format!("${:.2}M", num / 1_000_000.0)
    } else if abs >= 1_000.0 {
        format!("${:.1}K", num / 1_000.0)
    } else {
        format!("${:.2}", num)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        create_fundamentals_tables(&conn).unwrap();
        conn
    }

    // ── format_large_number ────────────────────────────────────────

    #[test]
    fn format_large_number_trillions() {
        assert_eq!(format_large_number(1_500_000_000_000.0), "$1.50T");
        assert_eq!(format_large_number(2_000_000_000_000.0), "$2.00T");
    }

    #[test]
    fn format_large_number_billions() {
        assert_eq!(format_large_number(3_450_000_000.0), "$3.45B");
        assert_eq!(format_large_number(999_999_999.99), "$1000.00M"); // just under 1B
    }

    #[test]
    fn format_large_number_millions() {
        assert_eq!(format_large_number(12_340_000.0), "$12.34M");
        assert_eq!(format_large_number(1_000_000.0), "$1.00M");
    }

    #[test]
    fn format_large_number_thousands() {
        assert_eq!(format_large_number(5_000.0), "$5.0K");
        assert_eq!(format_large_number(1_234.5), "$1.2K");
    }

    #[test]
    fn format_large_number_small() {
        assert_eq!(format_large_number(42.5), "$42.50");
        assert_eq!(format_large_number(0.99), "$0.99");
    }

    #[test]
    fn format_large_number_negative() {
        // Negative values should use the same thresholds based on abs
        assert_eq!(format_large_number(-2_000_000_000.0), "$-2.00B");
        assert_eq!(format_large_number(-500.0), "$-500.00");
    }

    // ── extract_usd_fact ───────────────────────────────────────────

    #[test]
    fn extract_usd_fact_picks_most_recent() {
        let json = serde_json::json!({
            "facts": {
                "us-gaap": {
                    "CashAndCashEquivalentsAtCarryingValue": {
                        "units": {
                            "USD": [
                                { "end": "2023-12-31", "val": 10000.0 },
                                { "end": "2024-06-30", "val": 25000.0 },
                                { "end": "2024-03-31", "val": 15000.0 }
                            ]
                        }
                    }
                }
            }
        });
        let result = extract_usd_fact(&json, "CashAndCashEquivalentsAtCarryingValue");
        assert_eq!(result, Some(25000.0));
    }

    #[test]
    fn extract_usd_fact_missing_concept() {
        let json = serde_json::json!({ "facts": { "us-gaap": {} } });
        assert_eq!(extract_usd_fact(&json, "NonExistent"), None);
    }

    #[test]
    fn extract_usd_fact_empty_array() {
        let json = serde_json::json!({
            "facts": { "us-gaap": { "LongTermDebt": { "units": { "USD": [] } } } }
        });
        assert_eq!(extract_usd_fact(&json, "LongTermDebt"), None);
    }

    // ── yf_raw / yf_fmt ────────────────────────────────────────────

    #[test]
    fn yf_raw_extracts_raw_value() {
        let json = serde_json::json!({
            "price": { "marketCap": { "raw": 1234567890.0, "fmt": "1.23B" } }
        });
        assert_eq!(yf_raw(&json, "/price/marketCap"), Some(1234567890.0));
    }

    #[test]
    fn yf_raw_returns_none_on_missing() {
        let json = serde_json::json!({ "price": {} });
        assert_eq!(yf_raw(&json, "/price/marketCap"), None);
    }

    #[test]
    fn yf_fmt_extracts_formatted_string() {
        let json = serde_json::json!({
            "calendarEvents": { "exDividendDate": { "raw": 1718409600, "fmt": "2024-06-15" } }
        });
        assert_eq!(yf_fmt(&json, "/calendarEvents/exDividendDate"), Some("2024-06-15".to_string()));
    }

    #[test]
    fn yf_fmt_returns_none_on_missing() {
        let json = serde_json::json!({});
        assert_eq!(yf_fmt(&json, "/calendarEvents/exDividendDate"), None);
    }

    // ── parse_yahoo_data ───────────────────────────────────────────

    #[test]
    fn parse_yahoo_data_basic_fields() {
        let yahoo = serde_json::json!({
            "summaryProfile": {
                "sector": "Technology",
                "industry": "Semiconductors",
                "longBusinessSummary": "A chip company."
            },
            "price": {
                "shortName": "ACME Corp",
                "marketCap": { "raw": 50_000_000_000.0 },
                "regularMarketPrice": { "raw": 150.0 }
            },
            "defaultKeyStatistics": {
                "enterpriseValue": { "raw": 55_000_000_000.0 },
                "sharesOutstanding": { "raw": 333_333_333.0 },
                "beta": { "raw": 1.2 }
            }
        });

        let f = parse_yahoo_data("ACME", &yahoo);
        assert_eq!(f.symbol, "ACME");
        assert_eq!(f.company_name, "ACME Corp");
        assert_eq!(f.sector, "Technology");
        assert_eq!(f.industry, "Semiconductors");
        assert_eq!(f.description, "A chip company.");
        assert_eq!(f.market_cap, Some(50_000_000_000.0));
        assert_eq!(f.stock_price, Some(150.0));
        assert_eq!(f.enterprise_value, Some(55_000_000_000.0));
        assert_eq!(f.beta, Some(1.2));
    }

    #[test]
    fn parse_yahoo_data_ev_fallback_calculation() {
        // If enterpriseValue is missing, it should be calculated from mcap + debt - cash
        let yahoo = serde_json::json!({
            "price": {
                "marketCap": { "raw": 100_000.0 },
                "regularMarketPrice": { "raw": 10.0 }
            },
            "financialData": {
                "totalDebt": { "raw": 30_000.0 },
                "totalCash": { "raw": 5_000.0 }
            }
        });

        let f = parse_yahoo_data("TEST", &yahoo);
        assert_eq!(f.enterprise_value, Some(125_000.0)); // 100k + 30k - 5k
        // mcap_ev_ratio = 100k / 125k * 100 = 80.0
        let ratio = f.mcap_ev_ratio.unwrap();
        assert!((ratio - 80.0).abs() < 0.01);
    }

    #[test]
    fn parse_yahoo_data_dividend_stock() {
        let yahoo = serde_json::json!({
            "summaryDetail": {
                "dividendRate": { "raw": 2.5 },
                "dividendYield": { "raw": 0.015 }
            }
        });

        let f = parse_yahoo_data("DIV", &yahoo);
        assert!(f.is_dividend_stock);
        assert_eq!(f.dividend_yield, Some(0.015));
    }

    // ── parse_quarterly_financials ─────────────────────────────────

    #[test]
    fn parse_quarterly_financials_basic() {
        let yahoo = serde_json::json!({
            "incomeStatementHistoryQuarterly": {
                "incomeStatementHistory": [
                    {
                        "endDate": { "raw": 1711843200, "fmt": "2024-03-31" },
                        "totalRevenue": { "raw": 50_000_000.0 },
                        "netIncome": { "raw": 5_000_000.0 },
                        "grossProfit": { "raw": 20_000_000.0 },
                        "operatingIncome": { "raw": 8_000_000.0 }
                    }
                ]
            }
        });

        let quarters = parse_quarterly_financials("TEST", &yahoo);
        assert_eq!(quarters.len(), 1);
        assert_eq!(quarters[0].symbol, "TEST");
        assert_eq!(quarters[0].period_end, "2024-03-31");
        assert_eq!(quarters[0].total_revenue, Some(50_000_000.0));
        assert_eq!(quarters[0].net_income, Some(5_000_000.0));
    }

    #[test]
    fn parse_quarterly_financials_with_cashflow() {
        let yahoo = serde_json::json!({
            "incomeStatementHistoryQuarterly": {
                "incomeStatementHistory": [
                    {
                        "endDate": { "raw": 1711843200, "fmt": "2024-03-31" },
                        "totalRevenue": { "raw": 100_000.0 },
                        "netIncome": { "raw": 10_000.0 }
                    }
                ]
            },
            "cashflowStatementHistoryQuarterly": {
                "cashflowStatements": [
                    {
                        "endDate": { "raw": 1711843200, "fmt": "2024-03-31" },
                        "totalCashFromOperatingActivities": { "raw": 15_000.0 },
                        "capitalExpenditures": { "raw": -3_000.0 }
                    }
                ]
            }
        });

        let quarters = parse_quarterly_financials("TEST", &yahoo);
        assert_eq!(quarters.len(), 1);
        // FCF = operating CF - |capex| = 15000 - 3000 = 12000
        assert_eq!(quarters[0].free_cash_flow, Some(12_000.0));
    }

    #[test]
    fn parse_quarterly_financials_empty() {
        let yahoo = serde_json::json!({});
        let quarters = parse_quarterly_financials("TEST", &yahoo);
        assert!(quarters.is_empty());
    }

    // ── parse_institutional_holders ────────────────────────────────

    #[test]
    fn parse_institutional_holders_basic() {
        let yahoo = serde_json::json!({
            "institutionOwnership": {
                "ownershipList": [
                    {
                        "organization": "Vanguard Group",
                        "position": { "raw": 50_000_000 },
                        "pctHeld": { "raw": 0.08 },
                        "value": { "raw": 7_500_000_000.0 },
                        "reportDate": { "raw": 1711843200, "fmt": "2024-03-31" }
                    },
                    {
                        "organization": "BlackRock",
                        "position": { "raw": 40_000_000 },
                        "pctHeld": { "raw": 0.065 },
                        "value": { "raw": 6_000_000_000.0 },
                        "reportDate": { "raw": 1711843200, "fmt": "2024-03-31" }
                    }
                ]
            }
        });

        let holders = parse_institutional_holders("TEST", &yahoo);
        assert_eq!(holders.len(), 2);
        assert_eq!(holders[0].holder_name, "Vanguard Group");
        assert_eq!(holders[0].shares, 50_000_000);
        assert_eq!(holders[1].holder_name, "BlackRock");
        assert_eq!(holders[1].symbol, "TEST");
    }

    #[test]
    fn parse_institutional_holders_empty() {
        let yahoo = serde_json::json!({});
        let holders = parse_institutional_holders("TEST", &yahoo);
        assert!(holders.is_empty());
    }

    // ── SQLite CRUD roundtrip ──────────────────────────────────────

    #[test]
    fn upsert_and_get_fundamentals_roundtrip() {
        let conn = setup_test_db();
        let f = Fundamentals {
            symbol: "AAPL".to_string(),
            company_name: "Apple Inc".to_string(),
            sector: "Technology".to_string(),
            industry: "Consumer Electronics".to_string(),
            market_cap: Some(3_000_000_000_000.0),
            enterprise_value: Some(3_100_000_000_000.0),
            stock_price: Some(195.0),
            pe_ratio: Some(30.5),
            beta: Some(1.25),
            is_dividend_stock: true,
            dividend_yield: Some(0.005),
            last_updated: "2024-01-15T12:00:00Z".to_string(),
            ..Default::default()
        };
        upsert_fundamentals(&conn, &f).unwrap();

        let loaded = get_fundamentals(&conn, "AAPL").unwrap().unwrap();
        assert_eq!(loaded.symbol, "AAPL");
        assert_eq!(loaded.company_name, "Apple Inc");
        assert_eq!(loaded.market_cap, Some(3_000_000_000_000.0));
        assert!(loaded.is_dividend_stock);
        assert_eq!(loaded.pe_ratio, Some(30.5));
    }

    #[test]
    fn upsert_fundamentals_updates_existing() {
        let conn = setup_test_db();
        let f1 = Fundamentals {
            symbol: "MSFT".to_string(),
            company_name: "Microsoft".to_string(),
            stock_price: Some(400.0),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
            ..Default::default()
        };
        upsert_fundamentals(&conn, &f1).unwrap();

        let f2 = Fundamentals {
            symbol: "MSFT".to_string(),
            company_name: "Microsoft Corporation".to_string(),
            stock_price: Some(420.0),
            last_updated: "2024-02-01T00:00:00Z".to_string(),
            ..Default::default()
        };
        upsert_fundamentals(&conn, &f2).unwrap();

        let loaded = get_fundamentals(&conn, "MSFT").unwrap().unwrap();
        assert_eq!(loaded.company_name, "Microsoft Corporation");
        assert_eq!(loaded.stock_price, Some(420.0));
    }

    #[test]
    fn get_fundamentals_not_found() {
        let conn = setup_test_db();
        let result = get_fundamentals(&conn, "ZZZZ").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn get_all_fundamentals_multiple() {
        let conn = setup_test_db();
        for sym in ["AAPL", "GOOG", "MSFT"] {
            let f = Fundamentals {
                symbol: sym.to_string(),
                last_updated: "2024-01-01T00:00:00Z".to_string(),
                ..Default::default()
            };
            upsert_fundamentals(&conn, &f).unwrap();
        }
        let all = get_all_fundamentals(&conn).unwrap();
        assert_eq!(all.len(), 3);
        // Should be ordered alphabetically
        assert_eq!(all[0].symbol, "AAPL");
        assert_eq!(all[1].symbol, "GOOG");
        assert_eq!(all[2].symbol, "MSFT");
    }

    #[test]
    fn upsert_and_get_quarterly_financials() {
        let conn = setup_test_db();
        let quarters = vec![
            QuarterlyFinancial {
                symbol: "TEST".to_string(),
                period_end: "2024-03-31".to_string(),
                total_revenue: Some(1_000_000.0),
                net_income: Some(100_000.0),
                free_cash_flow: Some(80_000.0),
                gross_profit: None,
                operating_income: None,
                ebitda: None,
                eps: Some(1.5),
            },
            QuarterlyFinancial {
                symbol: "TEST".to_string(),
                period_end: "2023-12-31".to_string(),
                total_revenue: Some(900_000.0),
                net_income: Some(90_000.0),
                free_cash_flow: None,
                gross_profit: None,
                operating_income: None,
                ebitda: None,
                eps: Some(1.3),
            },
        ];
        upsert_quarterly(&conn, &quarters).unwrap();

        let loaded = get_quarterly_financials(&conn, "TEST").unwrap();
        assert_eq!(loaded.len(), 2);
        // Ordered by period_end DESC
        assert_eq!(loaded[0].period_end, "2024-03-31");
        assert_eq!(loaded[0].total_revenue, Some(1_000_000.0));
        assert_eq!(loaded[1].period_end, "2023-12-31");
    }

    #[test]
    fn upsert_and_get_institutional_holders() {
        let conn = setup_test_db();
        let holders = vec![
            InstitutionalHolder {
                symbol: "TEST".to_string(),
                holder_name: "Vanguard".to_string(),
                shares: 1_000_000,
                pct_held: 0.05,
                value: 150_000_000.0,
                date_reported: "2024-03-31".to_string(),
            },
        ];
        upsert_holders(&conn, &holders).unwrap();

        let loaded = get_institutional_holders(&conn, "TEST").unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].holder_name, "Vanguard");
        assert_eq!(loaded[0].shares, 1_000_000);
    }

    // ── extract_stock_tickers_from_cache ───────────────────────────

    #[test]
    fn extract_stock_tickers_filters_correctly() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE bar_cache (key TEXT PRIMARY KEY, value BLOB)").unwrap();

        // Insert test cache keys
        let keys = [
            "mt5:CC:AAPL:4Hour",    // stock — should be kept
            "mt5:CC:MSFT:Daily",     // stock — should be kept
            "mt5:CC:EURUSD:1Hour",   // forex — should be skipped
            "mt5:CC:BTCUSD:4Hour",   // crypto — should be skipped
            "mt5:CC:GC:Daily",       // futures — should be skipped
            "mt5:CC:XAUUSD:4Hour",   // gold CFD — should be skipped
            "mt5:CC:SOLUSD:1Hour",   // crypto — should be skipped
            "mt5:CC:NVDA:Daily",     // stock — should be kept
        ];
        for key in keys {
            conn.execute("INSERT INTO bar_cache (key, value) VALUES (?1, x'00')", params![key]).unwrap();
        }

        let tickers = extract_stock_tickers_from_cache(&conn).unwrap();
        assert!(tickers.contains(&"AAPL".to_string()));
        assert!(tickers.contains(&"MSFT".to_string()));
        assert!(tickers.contains(&"NVDA".to_string()));
        assert!(!tickers.contains(&"EURUSD".to_string()));
        assert!(!tickers.contains(&"BTCUSD".to_string()));
        assert!(!tickers.contains(&"XAUUSD".to_string()));
        assert!(!tickers.contains(&"SOLUSD".to_string()));
    }

    #[test]
    fn extract_stock_tickers_skips_futures() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE bar_cache (key TEXT PRIMARY KEY, value BLOB)").unwrap();

        let keys = [
            "mt5:CC:ES_M:Daily",     // futures — skip
            "mt5:CC:CL_H:4Hour",     // futures — skip
            "mt5:CC:GOOG:Daily",     // stock — keep
        ];
        for key in keys {
            conn.execute("INSERT INTO bar_cache (key, value) VALUES (?1, x'00')", params![key]).unwrap();
        }

        let tickers = extract_stock_tickers_from_cache(&conn).unwrap();
        assert!(tickers.contains(&"GOOG".to_string()));
        assert!(!tickers.contains(&"ES_M".to_string()));
        assert!(!tickers.contains(&"CL_H".to_string()));
    }

    #[test]
    fn extract_stock_tickers_deduplicates() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE bar_cache (key TEXT PRIMARY KEY, value BLOB)").unwrap();

        let keys = [
            "mt5:CC:AAPL:4Hour",
            "mt5:CC:AAPL:Daily",
            "mt5:CC:AAPL:1Hour",
        ];
        for key in keys {
            conn.execute("INSERT INTO bar_cache (key, value) VALUES (?1, x'00')", params![key]).unwrap();
        }

        let tickers = extract_stock_tickers_from_cache(&conn).unwrap();
        assert_eq!(tickers.iter().filter(|t| *t == "AAPL").count(), 1);
    }

    // ── create_fundamentals_tables idempotent ──────────────────────

    #[test]
    fn create_tables_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        create_fundamentals_tables(&conn).unwrap();
        create_fundamentals_tables(&conn).unwrap(); // second call should not fail
    }
}
