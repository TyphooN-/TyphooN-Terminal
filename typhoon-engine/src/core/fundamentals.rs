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

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const SEC_USER_AGENT: &str = "TyphooN-Terminal/1.0 typhoon-terminal@example.invalid";
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
    pub period_end: String, // YYYY-MM-DD
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
    conn.execute_batch(
        "
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
    ",
    )
    .map_err(|e| format!("Create fundamentals tables failed: {e}"))?;

    // Schema migration: add updated_at columns (last-modified tracking)
    let _ = conn.execute(
        "ALTER TABLE fundamentals ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE quarterly_financials ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE institutional_holders ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
        [],
    );

    Ok(())
}

pub fn prioritize_fundamentals_symbols(conn: &Connection, tickers: &mut [String], force: bool) {
    let original_rank: HashMap<String, usize> = tickers
        .iter()
        .enumerate()
        .map(|(i, s)| (s.clone(), i))
        .collect();

    let mut last_updated: HashMap<String, String> = HashMap::new();
    if let Ok(mut stmt) = conn.prepare("SELECT symbol, last_updated FROM fundamentals") {
        if let Ok(rows) = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }) {
            for row in rows.flatten() {
                last_updated.insert(row.0.to_uppercase(), row.1);
            }
        }
    }

    let mut failures = std::collections::HashSet::new();
    if !force {
        if let Ok(mut stmt) = conn.prepare("SELECT symbol FROM scrape_failures") {
            if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                for row in rows.flatten() {
                    failures.insert(row.to_uppercase());
                }
            }
        }
    }

    tickers.sort_by(|a, b| {
        let key = |sym: &String| {
            let upper = sym.to_uppercase();
            let failed = failures.contains(&upper);
            let updated = last_updated.get(&upper).cloned().unwrap_or_default();
            (
                failed,
                !updated.is_empty(),
                updated,
                *original_rank.get(sym).unwrap_or(&usize::MAX),
            )
        };
        key(a).cmp(&key(b))
    });
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
    let resp = client
        .get(url)
        .header("User-Agent", SEC_USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("SEC CIK fetch failed: {e}"))?;

    let data: HashMap<String, SecCompanyEntry> = resp
        .json()
        .await
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
    let resp = client
        .get(&url)
        .header("User-Agent", SEC_USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("SEC XBRL fetch failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("SEC XBRL returned {}", resp.status()));
    }

    let facts: serde_json::Value = resp
        .json()
        .await
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
/// `summaryProfile` covers equities (sector/industry); `fundProfile` + `quoteType`
/// cover ETFs (categoryName, legalType) since ETFs return an empty summaryProfile.
const YAHOO_MODULES: &str = "financialData,defaultKeyStatistics,calendarEvents,summaryProfile,summaryDetail,earningsHistory,institutionOwnership,incomeStatementHistoryQuarterly,cashflowStatementHistoryQuarterly,price,fundProfile,quoteType";

/// Yahoo Finance session with crumb authentication.
/// Yahoo requires a crumb token (CSRF) obtained from a cookie-authenticated session.
pub struct YahooSession {
    client: reqwest::Client,
    crumb: String,
}

impl YahooSession {
    /// Get the authenticated HTTP client (with cookie jar).
    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }
    /// Get the crumb token for API calls.
    pub fn crumb(&self) -> &str {
        &self.crumb
    }

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

        // Step 1: Accept consent / get session cookies via fc.yahoo.com.
        // This sets the A1/A3 cookies that bypass the EU consent wall.
        // A 404 here is expected — when no consent wall is needed the
        // endpoint reports not-found but the cookies still attach, and the
        // following crumb call succeeds. Demoted to debug so the typical
        // boot trace stays clean.
        match client.get("https://fc.yahoo.com").send().await {
            Ok(r) => tracing::debug!("Yahoo fc.yahoo.com: status {}", r.status()),
            Err(e) => tracing::debug!("Yahoo fc.yahoo.com failed (non-fatal): {}", e),
        }

        // Step 2: Get crumb directly (the fc.yahoo.com cookies are enough)
        let crumb_resp = client
            .get("https://query2.finance.yahoo.com/v1/test/getcrumb")
            .header("Accept", "text/plain")
            .send()
            .await
            .map_err(|e| format!("Yahoo crumb fetch failed: {e}"))?;

        let status = crumb_resp.status();
        let crumb = crumb_resp
            .text()
            .await
            .map_err(|e| format!("Yahoo crumb read failed: {e}"))?;

        if !status.is_success() {
            tracing::debug!("Yahoo crumb returned {} — trying without crumb", status);
            return Ok(Self {
                client,
                crumb: String::new(),
            });
        }

        if crumb.is_empty() || crumb.contains('<') || crumb.len() > 50 {
            tracing::debug!(
                "Yahoo crumb looks invalid ({} bytes) — trying without crumb",
                crumb.len()
            );
            return Ok(Self {
                client,
                crumb: String::new(),
            });
        }

        tracing::debug!("Yahoo session established (crumb acquired)");
        Ok(Self { client, crumb })
    }
}

/// Fetch comprehensive fundamentals from Yahoo Finance quoteSummary API.
pub async fn fetch_yahoo_fundamentals(
    session: &YahooSession,
    ticker: &str,
) -> Result<serde_json::Value, String> {
    // Validate ticker: alphanumeric + dots + hyphens only (prevent URL injection)
    if ticker.is_empty()
        || ticker.len() > 20
        || !ticker
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    {
        return Err(format!("Invalid ticker for Yahoo: {ticker}"));
    }
    let url = if session.crumb.is_empty() {
        format!(
            "https://query2.finance.yahoo.com/v10/finance/quoteSummary/{ticker}?modules={YAHOO_MODULES}"
        )
    } else {
        format!(
            "https://query2.finance.yahoo.com/v10/finance/quoteSummary/{ticker}?modules={YAHOO_MODULES}&crumb={}",
            session.crumb
        )
    };
    let resp = session
        .client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Yahoo fetch failed for {ticker}: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Yahoo returned {} for {ticker}", resp.status()));
    }

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Yahoo parse failed for {ticker}: {e}"))?;

    // Navigate to the result
    let result = data
        .pointer("/quoteSummary/result/0")
        .ok_or_else(|| format!("No Yahoo data for {ticker}"))?;

    Ok(result.clone())
}

/// Helper to extract a raw number from Yahoo's nested {"raw": 123.45} format.
fn yahoo_json_raw(val: &serde_json::Value, path: &str) -> Option<f64> {
    val.pointer(path)?.get("raw")?.as_f64()
}

/// Helper to extract a string from Yahoo's nested {"fmt": "2026-04-15"} format.
fn yahoo_json_fmt(val: &serde_json::Value, path: &str) -> Option<String> {
    val.pointer(path)?
        .get("fmt")?
        .as_str()
        .map(|s| s.to_string())
}

/// Parse Yahoo Finance JSON into Fundamentals struct.
pub fn parse_yahoo_data(ticker: &str, yahoo: &serde_json::Value) -> Fundamentals {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let mut f = Fundamentals {
        symbol: ticker.to_uppercase(),
        last_updated: now,
        ..Default::default()
    };

    // summaryProfile (equities) — may be empty for ETFs/mutual funds
    if let Some(p) = yahoo.get("summaryProfile") {
        f.sector = p
            .get("sector")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        f.industry = p
            .get("industry")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        f.description = p
            .get("longBusinessSummary")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
    }

    // ETF/fund fallback: when summaryProfile is empty, pull from fundProfile + quoteType.
    // Yahoo returns sector/industry only for operating companies — ETFs and mutual
    // funds use fundProfile.categoryName ("Large Blend", "Emerging Markets", etc.)
    // and quoteType.quoteType ("ETF", "MUTUALFUND") for classification.
    if f.sector.is_empty() && f.industry.is_empty() {
        let qt = yahoo
            .get("quoteType")
            .and_then(|q| q.get("quoteType"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if let Some(fp) = yahoo.get("fundProfile") {
            let category = fp
                .get("categoryName")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let family = fp.get("family").and_then(|v| v.as_str()).unwrap_or("");
            let legal_type = fp.get("legalType").and_then(|v| v.as_str()).unwrap_or("");
            if !category.is_empty() || !family.is_empty() || !legal_type.is_empty() {
                // Sector bucket: "ETF" / "MUTUALFUND" / fallback to legalType
                f.sector = match qt {
                    "ETF" => "ETF".to_string(),
                    "MUTUALFUND" => "Mutual Fund".to_string(),
                    _ if !legal_type.is_empty() => legal_type.to_string(),
                    _ => "Fund".to_string(),
                };
                // Industry: fund category (asset class + style), e.g. "Large Blend"
                f.industry = if !category.is_empty() {
                    category.to_string()
                } else {
                    legal_type.to_string()
                };
                // Description: family + category as a quick label
                if f.description.is_empty() {
                    f.description = if !family.is_empty() {
                        format!("{family} — {category}")
                    } else {
                        category.to_string()
                    };
                }
            }
        }
        // Last resort: use quoteType alone if even fundProfile is missing
        if f.sector.is_empty() && !qt.is_empty() && qt != "EQUITY" {
            f.sector = match qt {
                "ETF" => "ETF".to_string(),
                "MUTUALFUND" => "Mutual Fund".to_string(),
                "CRYPTOCURRENCY" => "Crypto".to_string(),
                "CURRENCY" => "FX".to_string(),
                "INDEX" => "Index".to_string(),
                "FUTURE" => "Futures".to_string(),
                other => other.to_string(),
            };
        }
    }

    // price module
    if let Some(p) = yahoo.get("price") {
        f.company_name = p
            .get("shortName")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        f.market_cap = yahoo_json_raw(p, "/marketCap");
        f.stock_price = yahoo_json_raw(p, "/regularMarketPrice");
    }

    // defaultKeyStatistics
    if let Some(ks) = yahoo.get("defaultKeyStatistics") {
        f.enterprise_value = yahoo_json_raw(ks, "/enterpriseValue");
        f.shares_outstanding = yahoo_json_raw(ks, "/sharesOutstanding");
        f.pe_ratio = yahoo_json_raw(ks, "/trailingEps").and_then(|eps| {
            f.stock_price
                .map(|p| if eps != 0.0 { p / eps } else { 0.0 })
        });
        f.forward_pe = yahoo_json_raw(ks, "/forwardPE");
        f.peg_ratio = yahoo_json_raw(ks, "/pegRatio");
        f.price_to_book = yahoo_json_raw(ks, "/priceToBook");
        f.beta = yahoo_json_raw(ks, "/beta");
        f.short_ratio = yahoo_json_raw(ks, "/shortRatio");
        f.short_percent_of_float = yahoo_json_raw(ks, "/shortPercentOfFloat");
    }

    // summaryDetail
    if let Some(sd) = yahoo.get("summaryDetail") {
        f.dividend_yield = yahoo_json_raw(sd, "/dividendYield");
        f.pe_ratio = f.pe_ratio.or_else(|| yahoo_json_raw(sd, "/trailingPE"));
        f.forward_pe = f.forward_pe.or_else(|| yahoo_json_raw(sd, "/forwardPE"));
        f.price_to_sales = yahoo_json_raw(sd, "/priceToSalesTrailing12Months");
        // Check if pays dividends
        if let Some(rate) = yahoo_json_raw(sd, "/dividendRate") {
            f.is_dividend_stock = rate > 0.0;
        }
    }

    // financialData
    if let Some(fd) = yahoo.get("financialData") {
        f.profit_margin = yahoo_json_raw(fd, "/profitMargins");
        f.operating_margin = yahoo_json_raw(fd, "/operatingMargins");
        f.roe = yahoo_json_raw(fd, "/returnOnEquity");
        f.roa = yahoo_json_raw(fd, "/returnOnAssets");
        f.total_debt = f.total_debt.or_else(|| yahoo_json_raw(fd, "/totalDebt"));
        f.cash_and_equivalents = f
            .cash_and_equivalents
            .or_else(|| yahoo_json_raw(fd, "/totalCash"));
        f.ev_to_ebitda = yahoo_json_raw(fd, "/enterpriseToEbitda");
    }

    // EV components: prefer SEC XBRL (filled later), fallback to Yahoo
    if f.enterprise_value.is_none() {
        if let (Some(mc), Some(debt), Some(cash)) =
            (f.market_cap, f.total_debt, f.cash_and_equivalents)
        {
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
        f.next_ex_dividend_date = yahoo_json_fmt(cal, "/exDividendDate");
        f.next_dividend_payment_date = yahoo_json_fmt(cal, "/dividendDate");
    }

    f
}

/// Parse quarterly financials from Yahoo Finance JSON.
pub fn parse_quarterly_financials(
    ticker: &str,
    yahoo: &serde_json::Value,
) -> Vec<QuarterlyFinancial> {
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
                total_revenue: yahoo_json_raw(entry, "/totalRevenue"),
                net_income: yahoo_json_raw(entry, "/netIncome"),
                gross_profit: yahoo_json_raw(entry, "/grossProfit"),
                operating_income: yahoo_json_raw(entry, "/operatingIncome"),
                ebitda: yahoo_json_raw(entry, "/ebitda"),
                eps: yahoo_json_raw(entry, "/dilutedEPS")
                    .or_else(|| yahoo_json_raw(entry, "/basicEPS")),
                free_cash_flow: None,
            };

            // Free cash flow from cashflow statement
            if let Some(cf_entry) = cf {
                let op_cf = yahoo_json_raw(cf_entry, "/totalCashFromOperatingActivities");
                let capex = yahoo_json_raw(cf_entry, "/capitalExpenditures");
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
pub fn parse_institutional_holders(
    ticker: &str,
    yahoo: &serde_json::Value,
) -> Vec<InstitutionalHolder> {
    let mut results = Vec::new();

    if let Some(inst) = yahoo.pointer("/institutionOwnership/ownershipList") {
        if let Some(arr) = inst.as_array() {
            for entry in arr {
                let name = entry
                    .pointer("/organization")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string();
                let shares = entry
                    .pointer("/position/raw")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let pct = entry
                    .pointer("/pctHeld/raw")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let value = entry
                    .pointer("/value/raw")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let date = entry
                    .pointer("/reportDate/fmt")
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

    if f.short_percent_of_float
        .map(|v| v.is_finite() && v >= 0.0)
        .unwrap_or(false)
        || f.short_ratio
            .map(|v| v.is_finite() && v >= 0.0)
            .unwrap_or(false)
    {
        let as_of = if f.last_updated.len() >= 10 {
            f.last_updated[..10].to_string()
        } else {
            chrono::Utc::now().format("%Y-%m-%d").to_string()
        };
        let _ = crate::core::research::append_short_interest_history_point(
            conn,
            &f.symbol,
            crate::core::research::ShortInterestHistoryPoint {
                as_of,
                short_percent_of_float: f.short_percent_of_float.unwrap_or(0.0).max(0.0),
                short_ratio: f.short_ratio.unwrap_or(0.0).max(0.0),
                shares_outstanding: f.shares_outstanding.unwrap_or(0.0).max(0.0),
            },
        );
    }

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
                q.symbol,
                q.period_end,
                q.total_revenue,
                q.net_income,
                q.free_cash_flow,
                q.gross_profit,
                q.operating_income,
                q.ebitda,
                q.eps,
                chrono::Utc::now().timestamp(),
            ],
        )
        .map_err(|e| format!("Upsert quarterly failed: {e}"))?;
    }
    Ok(())
}

/// Store institutional holders (replace all for a symbol).
pub fn upsert_holders(conn: &Connection, holders: &[InstitutionalHolder]) -> Result<(), String> {
    if let Some(first) = holders.first() {
        conn.execute(
            "DELETE FROM institutional_holders WHERE symbol = ?1",
            params![first.symbol],
        )
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
    // prepare_cached: called frequently during research panel refreshes.
    let mut stmt = conn
        .prepare_cached(
            "SELECT symbol, cik, company_name, sector, industry, description,
                market_cap, enterprise_value, total_debt, cash_and_equivalents,
                shares_outstanding, stock_price, mcap_ev_ratio,
                next_earnings_date, previous_earnings_date,
                next_ex_dividend_date, next_dividend_payment_date, last_dividend_payment_date,
                is_dividend_stock, dividend_yield,
                pe_ratio, forward_pe, peg_ratio, price_to_book, price_to_sales,
                ev_to_ebitda, profit_margin, operating_margin, roe, roa,
                beta, short_ratio, short_percent_of_float, last_updated
         FROM fundamentals WHERE symbol = ?1",
        )
        .map_err(|e| format!("Prepare failed: {e}"))?;

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
    // prepare_cached: called every BG cycle to refresh the fundamentals cache.
    let mut stmt = conn
        .prepare_cached(
            "SELECT symbol, cik, company_name, sector, industry, '',
                market_cap, enterprise_value, total_debt, cash_and_equivalents,
                shares_outstanding, stock_price, mcap_ev_ratio,
                next_earnings_date, previous_earnings_date,
                next_ex_dividend_date, next_dividend_payment_date, last_dividend_payment_date,
                is_dividend_stock, dividend_yield,
                pe_ratio, forward_pe, peg_ratio, price_to_book, price_to_sales,
                ev_to_ebitda, profit_margin, operating_margin, roe, roa,
                beta, short_ratio, short_percent_of_float, last_updated
         FROM fundamentals ORDER BY symbol",
        )
        .map_err(|e| format!("Prepare all fundamentals failed: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
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
        })
        .map_err(|e| format!("Query all fundamentals failed: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect fundamentals failed: {e}"))
}

/// Get upcoming earnings dates sorted by date.
pub fn get_upcoming_earnings(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<(String, String, String)>, String> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    // prepare_cached: called every BG cycle.
    let mut stmt = conn
        .prepare_cached(
            "SELECT symbol, company_name, next_earnings_date FROM fundamentals
         WHERE next_earnings_date IS NOT NULL AND next_earnings_date >= ?1
         ORDER BY next_earnings_date ASC LIMIT ?2",
        )
        .map_err(|e| format!("Prepare earnings query failed: {e}"))?;

    let rows = stmt
        .query_map(params![today, limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| format!("Query earnings failed: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect earnings failed: {e}"))
}

/// Get upcoming ex-dividend dates sorted by date.
pub fn get_upcoming_dividends(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<(String, String, String, Option<f64>)>, String> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    // prepare_cached: called every BG cycle.
    let mut stmt = conn
        .prepare_cached(
            "SELECT symbol, company_name, next_ex_dividend_date, dividend_yield FROM fundamentals
         WHERE next_ex_dividend_date IS NOT NULL AND next_ex_dividend_date >= ?1
         AND is_dividend_stock = 1
         ORDER BY next_ex_dividend_date ASC LIMIT ?2",
        )
        .map_err(|e| format!("Prepare dividend query failed: {e}"))?;

    let rows = stmt
        .query_map(params![today, limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<f64>>(3)?,
            ))
        })
        .map_err(|e| format!("Query dividends failed: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect dividends failed: {e}"))
}

/// Get quarterly financials for a symbol.
pub fn get_quarterly_financials(
    conn: &Connection,
    symbol: &str,
) -> Result<Vec<QuarterlyFinancial>, String> {
    // prepare_cached: called per-symbol by the research panel.
    let mut stmt = conn
        .prepare_cached(
            "SELECT symbol, period_end, total_revenue, net_income, free_cash_flow,
                gross_profit, operating_income, ebitda, eps
         FROM quarterly_financials WHERE symbol = ?1 ORDER BY period_end DESC LIMIT 8",
        )
        .map_err(|e| format!("Prepare quarterly query failed: {e}"))?;

    let rows = stmt
        .query_map(params![symbol.to_uppercase()], |row| {
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
        })
        .map_err(|e| format!("Query quarterly failed: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect quarterly failed: {e}"))
}

/// Get institutional holders for a symbol.
pub fn get_institutional_holders(
    conn: &Connection,
    symbol: &str,
) -> Result<Vec<InstitutionalHolder>, String> {
    // prepare_cached: called per-symbol by the research panel.
    let mut stmt = conn
        .prepare_cached(
            "SELECT symbol, holder_name, shares, pct_held, value, date_reported
         FROM institutional_holders WHERE symbol = ?1 ORDER BY shares DESC",
        )
        .map_err(|e| format!("Prepare holders query failed: {e}"))?;

    let rows = stmt
        .query_map(params![symbol.to_uppercase()], |row| {
            Ok(InstitutionalHolder {
                symbol: row.get(0)?,
                holder_name: row.get(1)?,
                shares: row.get(2)?,
                pct_held: row.get(3)?,
                value: row.get(4)?,
                date_reported: row.get(5)?,
            })
        })
        .map_err(|e| format!("Query holders failed: {e}"))?;

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
            if let Some(d) = sec_debt {
                fund.total_debt = Some(d);
            }
            if let Some(c) = sec_cash {
                fund.cash_and_equivalents = Some(c);
            }
            // Recalculate EV with SEC data
            if let (Some(mc), Some(debt), Some(cash)) =
                (fund.market_cap, fund.total_debt, fund.cash_and_equivalents)
            {
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

    // PERF: preload the blocklist + last_updated index in two bulk queries instead
    // of two SELECTs per ticker inside the loop. For a 500-ticker scrape that's
    // 1000 → 2 DB round-trips. Both tables are small; full scans are fine.
    let blocklist: std::collections::HashSet<String> = {
        let mut set = std::collections::HashSet::new();
        if let Ok(mut stmt) = conn.prepare("SELECT symbol FROM scrape_failures") {
            if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                for r in rows.flatten() {
                    set.insert(r);
                }
            }
        }
        set
    };
    let last_updated_map: std::collections::HashMap<String, String> = {
        let mut map = std::collections::HashMap::new();
        if let Ok(mut stmt) = conn.prepare("SELECT symbol, last_updated FROM fundamentals") {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            }) {
                for r in rows.flatten() {
                    map.insert(r.0, r.1);
                }
            }
        }
        map
    };

    let mut ordered_tickers: Vec<String> =
        tickers.iter().map(|ticker| ticker.to_string()).collect();
    prioritize_fundamentals_symbols(conn, &mut ordered_tickers, false);

    let mut results = Vec::new();

    for ticker in &ordered_tickers {
        let ticker = ticker.trim().to_uppercase();

        // Skip forex pairs and indices
        if ticker.contains('/') || ticker.contains('#') || ticker.is_empty() {
            continue;
        }

        // Skip permanently failed symbols (404 from Yahoo — won't magically start working).
        if blocklist.contains(&ticker) {
            continue;
        }

        // Skip if recently updated (O(1) HashMap lookup instead of a SELECT per ticker)
        if let Some(ref cutoff_str) = cutoff {
            if let Some(existing_ts) = last_updated_map.get(&ticker) {
                if existing_ts.as_str() >= cutoff_str.as_str() {
                    let r = ScrapeResult {
                        symbol: ticker.clone(),
                        success: true,
                        message: "Skipped (recently updated)".into(),
                    };
                    if let Some(tx) = progress_tx {
                        let _ = tx.send(r.clone());
                    }
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
                if let Some(tx) = progress_tx {
                    let _ = tx.send(r.clone());
                }
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
                if let Some(tx) = progress_tx {
                    let _ = tx.send(r.clone());
                }
                results.push(r);
            }
        }
    }

    Ok(results)
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
mod tests;
