//! Darwinex Zero live web scraping via Selenium (thirtyfour WebDriver).
//!
//! Scrapes darwinexzero.com for live DARWIN stats that the FTP feed delivers
//! with 24h+ lag: NAV quotes, D-Score, VaR, investor flow, correlation.
//!
//! Requires ChromeDriver installed on the system.
//! Browser runs visible (not headless) so the user can solve CAPTCHAs.
//!
//! Tabs scraped per DARWIN (all except Signal Account):
//!   - Overview (quote, returns, D-Score, risk metrics, investor data)
//!   - Return/Performance (monthly returns grid, equity curve)
//!   - Risk (VaR history, drawdown periods)
//!   - Investable Attributes (D-Score component history)
//!   - Investor (investor count + AuM timeline)
//!
//! Portfolio-level pages scraped:
//!   - /portfolio/correlation (N×N matrix)
//!   - /portfolio/performance (portfolio equity curve, monthly returns)
//!   - /portfolio/risk (portfolio VaR history, drawdown)
//!   - /portfolio/allocation (DARWIN weights)

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;
use thirtyfour::prelude::*;

// ── Keyring Keys ────────��──────────────────────────────────────────

pub mod keys {
    pub const DARWINEX_EMAIL: &str = "darwinex_email";
    pub const DARWINEX_PASSWORD: &str = "darwinex_password";
}

// ── CSS Selectors ──────────────────────────────────────────────────
// Centralized so site redesigns only require updating one block.

mod selectors {
    // Login page
    pub const LOGIN_EMAIL: &str = "input[name='email'], input[type='email']";
    pub const LOGIN_PASSWORD: &str = "input[name='password'], input[type='password']";
    pub const LOGIN_SUBMIT: &str = "button[type='submit']";
    // CAPTCHA detection — covers reCAPTCHA, hCaptcha, and Cloudflare Turnstile
    pub const CAPTCHA_FRAME: &str = "iframe[src*='captcha'], iframe[src*='recaptcha'], .g-recaptcha, .h-captcha, iframe[src*='turnstile']";
    // Dashboard — presence means login succeeded
    pub const DASHBOARD_MARKER: &str =
        ".dashboard, [data-page='dashboard'], .portfolio-overview, .main-content";

    // ── DARWIN profile page — Overview tab ──────────────────────────
    pub const DARWIN_QUOTE: &str = ".darwin-quote, [data-field='quote'], .current-quote";
    pub const DARWIN_DAILY_RETURN: &str = ".daily-return, [data-field='daily_return']";
    pub const DARWIN_MONTHLY_RETURN: &str = ".monthly-return, [data-field='monthly_return']";
    pub const DARWIN_YTD_RETURN: &str = ".ytd-return, [data-field='ytd_return']";
    pub const DARWIN_ALL_TIME_RETURN: &str = ".all-time-return, [data-field='all_time_return']";
    // D-Score components
    pub const DARWIN_DSCORE: &str = ".d-score, [data-field='dscore'], .dscore-value";
    pub const DARWIN_DS_EXPERIENCE: &str = "[data-field='experience'], .ds-experience";
    pub const DARWIN_DS_RISK_MGMT: &str = "[data-field='risk_stability'], .ds-risk-stability";
    pub const DARWIN_DS_RISK_ADJ: &str = "[data-field='risk_adjustment'], .ds-risk-adjustment";
    pub const DARWIN_DS_PERFORMANCE: &str = "[data-field='performance'], .ds-performance";
    pub const DARWIN_DS_SCALABILITY: &str = "[data-field='scalability'], .ds-scalability";
    pub const DARWIN_DS_MARKET_CORR: &str = "[data-field='market_correlation'], .ds-market-corr";
    // Risk metrics
    pub const DARWIN_VAR: &str = ".var-monthly, [data-field='var'], .risk-var";
    pub const DARWIN_MAX_DD: &str = ".max-drawdown, [data-field='max_drawdown']";
    pub const DARWIN_VOL: &str = ".volatility, [data-field='volatility']";
    pub const DARWIN_SHARPE: &str = ".sharpe, [data-field='sharpe']";
    pub const DARWIN_SORTINO: &str = ".sortino, [data-field='sortino']";
    // Investor data
    pub const DARWIN_INVESTORS: &str = ".investor-count, [data-field='investors']";
    pub const DARWIN_AUM: &str = ".aum-value, [data-field='aum']";
    pub const DARWIN_CAPACITY: &str = ".capacity, [data-field='capacity']";
    // Exclusion/status detection
    pub const DARWIN_EXCLUDED: &str =
        ".excluded-badge, .suspended-badge, [data-status='excluded'], [data-status='suspended']";
    pub const DARWIN_EXCLUSION_REASON: &str = ".exclusion-reason, [data-field='exclusion_reason']";
    // Trading stats (strategy analysis tab)
    pub const DARWIN_TOTAL_TRADES: &str = ".total-trades, [data-field='total_trades']";
    pub const DARWIN_WIN_RATE: &str = ".win-rate, [data-field='win_rate']";
    pub const DARWIN_PROFIT_FACTOR: &str = ".profit-factor, [data-field='profit_factor']";
    pub const DARWIN_AVG_HOLD_TIME: &str = ".avg-hold-time, [data-field='avg_holding_time']";
    pub const DARWIN_AVG_TRADE_RETURN: &str = ".avg-trade-return, [data-field='avg_trade_return']";
    pub const DARWIN_SYMBOLS_TRADED: &str = ".symbols-traded, [data-field='symbols_traded']";

    // ── Tab navigation ─────────────────────────────────────────────
    // Tab selectors for clicking (the actual tab buttons/links on the DARWIN profile)
    pub const TAB_RETURN: &str = "a[href*='return'], a[href*='performance'], [data-tab='return'], [data-tab='performance'], .tab-return, .tab-performance, nav a:nth-child(2), .nav-tabs li:nth-child(2) a";
    pub const TAB_RISK: &str = "a[href*='risk'], [data-tab='risk'], .tab-risk, nav a:nth-child(3), .nav-tabs li:nth-child(3) a";
    pub const TAB_INVESTABLE: &str = "a[href*='investable'], a[href*='attributes'], [data-tab='investable'], .tab-investable, nav a:nth-child(4), .nav-tabs li:nth-child(4) a";
    pub const TAB_INVESTOR: &str = "a[href*='investor'], [data-tab='investor'], .tab-investor, nav a:nth-child(5), .nav-tabs li:nth-child(5) a";

    // ── Return/Performance tab ─────────────────────────────────────
    // Monthly returns grid: table rows with year + 12 monthly cells
    pub const MONTHLY_RETURNS_TABLE: &str =
        ".monthly-returns, table.returns-table, [data-section='monthly-returns'], .return-grid";
    pub const MONTHLY_RETURNS_ROW: &str = "tr[data-year], .returns-row, tbody tr";
    pub const MONTHLY_RETURNS_CELL: &str = "td[data-month], .return-cell, td";
    // Equity curve data points
    pub const EQUITY_CURVE_POINT: &str = "[data-equity], .equity-point, .chart-point";
    // All-time return from performance page (may have more granular data)
    pub const PERF_CAGR: &str = ".cagr, [data-field='cagr']";
    pub const PERF_BEST_MONTH: &str = ".best-month, [data-field='best_month']";
    pub const PERF_WORST_MONTH: &str = ".worst-month, [data-field='worst_month']";
    pub const PERF_AVG_MONTH: &str = ".avg-month, [data-field='avg_month']";
    pub const PERF_POSITIVE_MONTHS: &str = ".positive-months, [data-field='positive_months']";
    pub const PERF_NEGATIVE_MONTHS: &str = ".negative-months, [data-field='negative_months']";

    // ── Risk tab ─────────���─────────────────────────────────────────
    pub const VAR_HISTORY_ROW: &str = "[data-var-date], .var-history-row, .risk-history tr";
    pub const DRAWDOWN_PERIOD_ROW: &str = "[data-drawdown], .drawdown-row, .drawdown-period";
    pub const RISK_CURRENT_VAR: &str = ".current-var, [data-field='current_var']";
    pub const RISK_AVG_VAR: &str = ".avg-var, [data-field='avg_var']";
    pub const RISK_MAX_VAR: &str = ".max-var, [data-field='max_var']";
    pub const RISK_MIN_VAR: &str = ".min-var, [data-field='min_var']";
    pub const RISK_VAR_VIOLATIONS: &str = ".var-violations, [data-field='var_violations']";

    // ── Investable Attributes tab ───────��──────────────────────────
    pub const DSCORE_HISTORY_ROW: &str = "[data-dscore-date], .dscore-history-row";
    pub const _DSCORE_HISTORY_POINT: &str = "[data-dscore-value], .dscore-point";

    // ── Investor tab ────────────��──────────────────────────────────
    pub const INVESTOR_FLOW_ROW: &str =
        "[data-investor-date], .investor-flow-row, .investor-history tr";
    pub const INVESTOR_FLOW_COUNT: &str =
        "[data-investor-count], .investor-count-cell, td:nth-child(2)";
    pub const INVESTOR_FLOW_AUM: &str = "[data-investor-aum], .investor-aum-cell, td:nth-child(3)";
    pub const INVESTOR_CAPITAL_IN: &str = ".capital-in, [data-field='capital_in']";
    pub const INVESTOR_CAPITAL_OUT: &str = ".capital-out, [data-field='capital_out']";
    pub const INVESTOR_NET_FLOW: &str = ".net-flow, [data-field='net_flow']";
    pub const INVESTOR_DIVERGENCE: &str = ".divergence, [data-field='divergence_pct']";

    // ── Portfolio-level pages ──────────────────────────────────────
    // Correlation page
    pub const CORRELATION_CELL: &str = "td[data-correlation]";
    // Performance page
    pub const _PORTFOLIO_EQUITY: &str = ".portfolio-equity, [data-field='portfolio_return']";
    pub const PORTFOLIO_MONTHLY_TABLE: &str =
        ".portfolio-monthly-returns, .portfolio-returns-table";
    pub const PORTFOLIO_TOTAL_RETURN: &str = ".total-return, [data-field='total_return']";
    pub const PORTFOLIO_CAGR: &str = ".portfolio-cagr, [data-field='portfolio_cagr']";
    pub const PORTFOLIO_BEST_MONTH: &str =
        ".portfolio-best-month, [data-field='portfolio_best_month']";
    pub const PORTFOLIO_WORST_MONTH: &str =
        ".portfolio-worst-month, [data-field='portfolio_worst_month']";
    // Risk page
    pub const PORTFOLIO_VAR: &str = ".portfolio-var, [data-field='portfolio_var']";
    pub const PORTFOLIO_MAX_DD: &str = ".portfolio-max-dd, [data-field='portfolio_max_dd']";
    pub const PORTFOLIO_DIVERSIFICATION: &str =
        ".diversification-benefit, [data-field='diversification']";
    pub const PORTFOLIO_VAR_HISTORY_ROW: &str = ".portfolio-var-row, [data-portfolio-var-date]";
    // Allocation page
    pub const ALLOCATION_ROW: &str =
        ".allocation-row, [data-darwin-allocation], .allocation-table tr";
    pub const ALLOCATION_TICKER: &str =
        "[data-allocation-ticker], .allocation-ticker, td:nth-child(1)";
    pub const ALLOCATION_WEIGHT: &str =
        "[data-allocation-weight], .allocation-weight, td:nth-child(2)";
    pub const ALLOCATION_INVESTED: &str =
        "[data-allocation-invested], .allocation-invested, td:nth-child(3)";
    pub const ALLOCATION_PNL: &str = "[data-allocation-pnl], .allocation-pnl, td:nth-child(4)";
}

// ── Data Types ───────��─────────────────────────────────────────────

/// Live snapshot of a single DARWIN from darwinexzero.com.
/// Covers the Overview tab fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DarwinWebSnapshot {
    pub ticker: String,
    pub timestamp_ms: i64,
    // ── Quote & Returns ────────────��───────────────────────────
    pub quote: f64,
    pub daily_return_pct: f64,
    pub monthly_return_pct: f64,
    pub ytd_return_pct: f64,
    pub all_time_return_pct: f64,
    // ── D-Score Components ���────────────────────────────────────
    pub dscore: f64,
    pub ds_experience: f64,         // Ex (experience)
    pub ds_risk_mgmt: f64,          // Rs (risk stability)
    pub ds_risk_adjustment: f64,    // Ra (risk adjustment)
    pub ds_performance: f64,        // Pf (performance fee)
    pub ds_scalability: f64,        // Sc (scalability/capacity)
    pub ds_market_correlation: f64, // Mc (market correlation)
    // ── Risk Metrics ─────��─────────────────────────────────────
    pub var_monthly: f64, // monthly VaR
    pub max_drawdown_pct: f64,
    pub volatility_annual: f64,
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    // ── Investor Data ─────────────��────────────────────────────
    pub investors: u32,
    pub aum: f64,
    pub capacity_remaining_pct: f64,
    // ── Trading Stats ───────────��──────────────────────────────
    pub total_trades: u32,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub avg_holding_time_hours: f64,
    pub avg_trade_return_pct: f64,
    pub symbols_traded: u32,
    // ── Status ───────���──────────────────────────────────────────
    /// Whether this DARWIN is excluded/suspended on Darwinex (auto-detected).
    pub excluded: bool,
    /// Exclusion reason if applicable (e.g. "correlation", "VaR breach", "suspended").
    pub exclusion_reason: String,
    // ── Correlation ────────────────────────────────────────────
    /// Average pairwise correlation with other active DARWINs.
    pub correlation_portfolio: f64,
}

/// Monthly returns grid for a single DARWIN (year × month).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DarwinMonthlyReturns {
    pub ticker: String,
    /// Each entry: (year, [jan, feb, ..., dec]) — None if no data for that month.
    pub rows: Vec<MonthlyReturnRow>,
    /// Performance stats from the Return tab.
    pub cagr: f64,
    pub best_month_pct: f64,
    pub worst_month_pct: f64,
    pub avg_month_pct: f64,
    pub positive_months: u32,
    pub negative_months: u32,
}

/// One row of the monthly returns grid (one year).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonthlyReturnRow {
    pub year: u16,
    /// 12 entries (Jan=0 .. Dec=11). None if no data for that month.
    pub months: [Option<f64>; 12],
    /// Full-year return if shown.
    pub year_total: Option<f64>,
}

/// Equity curve time series for a DARWIN (from Return tab chart data).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DarwinEquityCurve {
    pub ticker: String,
    /// (timestamp_ms, nav_value) points.
    pub points: Vec<EquityPoint>,
}

/// A single equity curve data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EquityPoint {
    pub timestamp_ms: i64,
    pub value: f64,
}

/// VaR history for a DARWIN (from Risk tab).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DarwinVaRHistory {
    pub ticker: String,
    /// (timestamp_ms, var_pct) series.
    pub points: Vec<VaRPoint>,
    /// Summary risk stats from the Risk tab.
    pub current_var: f64,
    pub avg_var: f64,
    pub max_var: f64,
    pub min_var: f64,
    pub var_violations: u32,
    /// Drawdown periods: (start_ms, end_ms, depth_pct).
    pub drawdown_periods: Vec<DrawdownPeriod>,
}

/// A single VaR data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VaRPoint {
    pub timestamp_ms: i64,
    pub var_pct: f64,
}

/// A drawdown period with start/end dates and depth.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrawdownPeriod {
    pub start_ms: i64,
    pub end_ms: i64,
    pub depth_pct: f64,
    /// Recovery time in days (0 if still in drawdown).
    pub recovery_days: u32,
}

/// D-Score component history for a DARWIN (from Investable Attributes tab).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DarwinDScoreHistory {
    pub ticker: String,
    /// (timestamp_ms, dscore, ex, rs, ra, pf, sc, mc) series.
    pub points: Vec<DScorePoint>,
}

/// A single D-Score history data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DScorePoint {
    pub timestamp_ms: i64,
    pub dscore: f64,
    pub experience: f64,
    pub risk_stability: f64,
    pub risk_adjustment: f64,
    pub performance: f64,
    pub scalability: f64,
    pub market_correlation: f64,
}

/// Investor flow data for a DARWIN (from Investor tab).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DarwinInvestorFlow {
    pub ticker: String,
    /// Time series of investor count + AuM.
    pub points: Vec<InvestorFlowPoint>,
    /// Summary: net capital flow.
    pub capital_in: f64,
    pub capital_out: f64,
    pub net_flow: f64,
    /// Signal vs Quote divergence (%).
    pub divergence_pct: f64,
}

/// A single investor flow data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvestorFlowPoint {
    pub timestamp_ms: i64,
    pub investor_count: u32,
    pub aum: f64,
}

/// Portfolio-level performance data from /portfolio/performance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortfolioPerformance {
    pub total_return_pct: f64,
    pub cagr: f64,
    pub best_month_pct: f64,
    pub worst_month_pct: f64,
    /// Monthly returns grid for the portfolio.
    pub monthly_returns: Vec<MonthlyReturnRow>,
    /// Equity curve points.
    pub equity_points: Vec<EquityPoint>,
}

/// Portfolio-level risk data from /portfolio/risk.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortfolioRisk {
    pub current_var: f64,
    pub max_drawdown_pct: f64,
    /// Diversification benefit: how much the portfolio VaR is reduced vs
    /// sum-of-individual VaRs (positive = good diversification).
    pub diversification_benefit_pct: f64,
    /// Portfolio VaR over time.
    pub var_history: Vec<VaRPoint>,
}

/// Single DARWIN allocation entry from /portfolio/allocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DarwinAllocation {
    pub ticker: String,
    /// Current weight in portfolio (0.0–100.0 %).
    pub weight_pct: f64,
    /// Capital invested in this DARWIN.
    pub invested: f64,
    /// P&L from this DARWIN.
    pub pnl: f64,
}

/// Pairwise correlation between two DARWINs from the web dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DarwinWebCorrelation {
    pub darwin_a: String,
    pub darwin_b: String,
    pub correlation: f64,
}

/// User configuration for which DARWINs to scrape and scheduling.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DarwinWebConfig {
    pub managed_darwins: Vec<String>,
    /// DARWINs excluded from correlation analysis (e.g. MFSO).
    pub excluded_darwins: Vec<String>,
    pub auto_scrape: bool,
    /// Minute past the hour to trigger scrape (default: 20).
    pub scrape_minute: u32,
    /// Correlation threshold — alert if pairwise correlation exceeds this.
    pub correlation_alert_threshold: f64,
}

impl Default for DarwinWebConfig {
    fn default() -> Self {
        Self {
            managed_darwins: Vec::new(),
            excluded_darwins: Vec::new(),
            auto_scrape: false,
            scrape_minute: 20,
            correlation_alert_threshold: 0.95, // Darwinex 45-day standard
        }
    }
}

/// Serializable cookie for KV cache persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SerializableCookie {
    pub name: String,
    pub value: String,
    pub domain: Option<String>,
    pub path: Option<String>,
}

/// A correlation violation between two DARWINs with fix suggestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationAlert {
    pub darwin_a: String,
    pub darwin_b: String,
    pub correlation: f64,
    pub threshold: f64,
    /// Auto-generated suggestion to fix the correlation breach.
    pub suggestion: String,
}

/// Generate a suggestion for fixing a correlation breach between two DARWINs.
fn suggest_correlation_fix(darwin_a: &str, darwin_b: &str, correlation: f64) -> String {
    if correlation > 0.98 {
        format!(
            "CRITICAL: {} and {} are near-identical ({:.4}). Consider closing one or switching \
             one to a different symbol/strategy to break correlation immediately.",
            darwin_a, darwin_b, correlation
        )
    } else if correlation > 0.95 {
        format!(
            "HIGH: {} and {} correlated at {:.4}. Reduce overlapping symbol exposure — \
             shift one DARWIN's primary symbol to an uncorrelated asset or reduce position \
             size on the shared symbols.",
            darwin_a, darwin_b, correlation
        )
    } else {
        format!(
            "WARNING: {} and {} correlated at {:.4} (threshold: 0.95). Monitor — may \
             resolve naturally as positions rotate. If persistent, diversify symbol mix.",
            darwin_a, darwin_b, correlation
        )
    }
}

/// Result of a full scrape cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarwinWebUpdate {
    pub snapshots: Vec<DarwinWebSnapshot>,
    pub correlations: Vec<DarwinWebCorrelation>,
    /// Pairs that exceed the correlation threshold (excluding excluded DARWINs).
    pub correlation_alerts: Vec<CorrelationAlert>,
    pub timestamp_ms: i64,
    // ── Expanded tab data (per-DARWIN) ─────────────────────────
    pub monthly_returns: Vec<DarwinMonthlyReturns>,
    pub equity_curves: Vec<DarwinEquityCurve>,
    pub var_histories: Vec<DarwinVaRHistory>,
    pub dscore_histories: Vec<DarwinDScoreHistory>,
    pub investor_flows: Vec<DarwinInvestorFlow>,
    // ── Portfolio-level data ───────��───────────────────────────
    pub portfolio_performance: Option<PortfolioPerformance>,
    pub portfolio_risk: Option<PortfolioRisk>,
    pub allocations: Vec<DarwinAllocation>,
}

/// Warnings from snapshot validation (non-fatal issues).
#[derive(Debug, Clone)]
pub struct ScrapeWarning {
    pub ticker: String,
    pub field: String,
    pub message: String,
}

// ── KV Cache Keys ──────────────────────────────────────────────────

pub mod cache_keys {
    pub const COOKIES: &str = "dwx_web:cookies";
    pub const CONFIG: &str = "dwx_web:config";
    pub const CORRELATION: &str = "dwx_web:correlation";
    pub const LAST_SCRAPE: &str = "dwx_web:last_scrape";
    pub const PORTFOLIO_PERF: &str = "dwx_web:portfolio_performance";
    pub const PORTFOLIO_RISK: &str = "dwx_web:portfolio_risk";
    pub const ALLOCATIONS: &str = "dwx_web:allocations";

    /// Per-DARWIN snapshot key.
    pub fn snapshot(ticker: &str) -> String {
        format!("dwx_web:{}:snapshot", ticker.to_uppercase())
    }
    /// Per-DARWIN monthly returns key.
    pub fn monthly_returns(ticker: &str) -> String {
        format!("dwx_web:{}:monthly_returns", ticker.to_uppercase())
    }
    /// Per-DARWIN equity curve key.
    pub fn equity_curve(ticker: &str) -> String {
        format!("dwx_web:{}:equity_curve", ticker.to_uppercase())
    }
    /// Per-DARWIN VaR history key.
    pub fn var_history(ticker: &str) -> String {
        format!("dwx_web:{}:var_history", ticker.to_uppercase())
    }
    /// Per-DARWIN D-Score history key.
    pub fn dscore_history(ticker: &str) -> String {
        format!("dwx_web:{}:dscore_history", ticker.to_uppercase())
    }
    /// Per-DARWIN investor flow key.
    pub fn investor_flow(ticker: &str) -> String {
        format!("dwx_web:{}:investor_flow", ticker.to_uppercase())
    }
}

// ── Constants ────────��─────────────────────────────────────────────

const CHROMEDRIVER_URL: &str = "http://localhost:9515";
const DARWINEX_LOGIN_URL: &str = "https://www.darwinexzero.com/login";
const DARWINEX_DASHBOARD_URL: &str = "https://www.darwinexzero.com/dashboard";
const CAPTCHA_POLL_MS: u64 = 500;
const CAPTCHA_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes for user to solve
const PAGE_LOAD_WAIT: Duration = Duration::from_secs(3);
const TAB_SWITCH_WAIT: Duration = Duration::from_secs(2);
/// Max retries for scraping a single page/element.
const MAX_RETRIES: u32 = 2;

// ── Browser Lifecycle ──────────────────────────────────────────────

/// Launch Chrome via ChromeDriver. Returns a visible (non-headless) browser.
pub async fn launch_browser() -> Result<WebDriver, String> {
    let mut caps = DesiredCapabilities::chrome();
    // Visible browser for CAPTCHA solving — do NOT add --headless
    caps.add_arg("--no-sandbox")
        .map_err(|e| format!("Chrome caps error: {e}"))?;
    caps.add_arg("--disable-dev-shm-usage")
        .map_err(|e| format!("Chrome caps error: {e}"))?;
    // Reasonable window size for scraping
    caps.add_arg("--window-size=1280,900")
        .map_err(|e| format!("Chrome caps error: {e}"))?;

    WebDriver::new(CHROMEDRIVER_URL, caps).await.map_err(|e| {
        format!(
            "Failed to connect to ChromeDriver at {CHROMEDRIVER_URL}: {e}. Is chromedriver running?"
        )
    })
}

/// Close the browser and end the WebDriver session.
pub async fn close_browser(driver: WebDriver) -> Result<(), String> {
    driver
        .quit()
        .await
        .map_err(|e| format!("Browser close error: {e}"))
}

// ── Login Flow ─────────────��────────────────────��──────────────────

/// Log in to darwinexzero.com. Returns true if CAPTCHA was encountered
/// (user must solve it in the visible browser window).
pub async fn login(driver: &WebDriver, email: &str, password: &str) -> Result<bool, String> {
    driver
        .goto(DARWINEX_LOGIN_URL)
        .await
        .map_err(|e| format!("Navigate to login failed: {e}"))?;

    // Wait for the email field to appear
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Fill email
    let email_field = driver
        .find(By::Css(selectors::LOGIN_EMAIL))
        .await
        .map_err(|e| format!("Email field not found: {e}"))?;
    email_field
        .clear()
        .await
        .map_err(|e| format!("Clear email: {e}"))?;
    email_field
        .send_keys(email)
        .await
        .map_err(|e| format!("Type email failed: {e}"))?;

    // Fill password
    let pwd_field = driver
        .find(By::Css(selectors::LOGIN_PASSWORD))
        .await
        .map_err(|e| format!("Password field not found: {e}"))?;
    pwd_field
        .clear()
        .await
        .map_err(|e| format!("Clear password: {e}"))?;
    pwd_field
        .send_keys(password)
        .await
        .map_err(|e| format!("Type password failed: {e}"))?;

    // Click submit
    let submit = driver
        .find(By::Css(selectors::LOGIN_SUBMIT))
        .await
        .map_err(|e| format!("Submit button not found: {e}"))?;
    submit
        .click()
        .await
        .map_err(|e| format!("Click submit failed: {e}"))?;

    // Wait a moment for page response
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Check for CAPTCHA
    let has_captcha = check_captcha(driver).await;
    if has_captcha {
        tracing::warn!("CAPTCHA detected — solve it in the browser window");
        wait_for_captcha_solve(driver).await?;
    }

    // Verify we reached the dashboard
    let url = driver
        .current_url()
        .await
        .map_err(|e| format!("Get URL failed: {e}"))?;
    if url.as_str().contains("login") {
        return Err("Login failed — still on login page after submit".to_string());
    }

    Ok(has_captcha)
}

/// Check if a CAPTCHA element is present on the current page.
pub async fn check_captcha(driver: &WebDriver) -> bool {
    driver.find(By::Css(selectors::CAPTCHA_FRAME)).await.is_ok()
}

/// Wait for the user to solve the CAPTCHA (poll for dashboard element).
pub async fn wait_for_captcha_solve(driver: &WebDriver) -> Result<(), String> {
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > CAPTCHA_TIMEOUT {
            return Err("CAPTCHA timeout — user did not solve within 5 minutes".to_string());
        }

        // Check if we've left the login page (CAPTCHA solved + login succeeded)
        if let Ok(url) = driver.current_url().await {
            if !url.as_str().contains("login") {
                return Ok(());
            }
        }

        // Also check for dashboard marker
        if driver
            .find(By::Css(selectors::DASHBOARD_MARKER))
            .await
            .is_ok()
        {
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(CAPTCHA_POLL_MS)).await;
    }
}

/// Check if the current session is still valid (not redirected to login).
pub async fn is_session_valid(driver: &WebDriver) -> bool {
    if let Err(_) = driver.goto(DARWINEX_DASHBOARD_URL).await {
        return false;
    }
    tokio::time::sleep(Duration::from_secs(2)).await;
    match driver.current_url().await {
        Ok(url) => !url.as_str().contains("login"),
        Err(_) => false,
    }
}

// ── Cookie Persistence ─────────────────────────────────────────────

/// Extract all cookies from the browser session for caching.
pub async fn get_cookies(driver: &WebDriver) -> Result<Vec<SerializableCookie>, String> {
    let cookies = driver
        .get_all_cookies()
        .await
        .map_err(|e| format!("Get cookies failed: {e}"))?;
    Ok(cookies
        .iter()
        .map(|c| SerializableCookie {
            name: c.name.clone(),
            value: c.value.clone(),
            domain: c.domain.clone(),
            path: c.path.clone(),
        })
        .collect())
}

/// Restore cookies from cache into the browser session.
pub async fn restore_cookies(
    driver: &WebDriver,
    cookies: &[SerializableCookie],
) -> Result<(), String> {
    // Must navigate to the domain first before setting cookies
    driver
        .goto(DARWINEX_LOGIN_URL)
        .await
        .map_err(|e| format!("Navigate for cookie restore failed: {e}"))?;
    tokio::time::sleep(Duration::from_secs(1)).await;

    for sc in cookies {
        let mut cookie = Cookie::new(&sc.name, &sc.value);
        if let Some(ref domain) = sc.domain {
            cookie.set_domain(domain.clone());
        }
        if let Some(ref path) = sc.path {
            cookie.set_path(path.clone());
        }
        // Ignore errors for individual cookies (some may be expired/invalid)
        let _ = driver.add_cookie(cookie).await;
    }
    Ok(())
}

// ── Scraping Helpers ──────────���───────────────────────────────────

/// Parse a numeric string (possibly with %, $, commas, or other formatting).
fn parse_numeric(text: &str) -> f64 {
    let cleaned: String = text
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    cleaned.parse::<f64>().unwrap_or(0.0)
}

/// Try to extract text from an element matched by a CSS selector.
/// Returns 0.0 if the element is not found or text parsing fails.
async fn scrape_numeric(driver: &WebDriver, selector: &str) -> f64 {
    match driver.find(By::Css(selector)).await {
        Ok(elem) => match elem.text().await {
            Ok(text) => parse_numeric(&text),
            Err(_) => 0.0,
        },
        Err(_) => 0.0,
    }
}

/// Try to click a tab/link element. Returns true if found and clicked.
async fn click_tab(driver: &WebDriver, selector: &str) -> bool {
    match driver.find(By::Css(selector)).await {
        Ok(elem) => {
            if elem.click().await.is_ok() {
                tokio::time::sleep(TAB_SWITCH_WAIT).await;
                true
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

/// Parse a date string (various formats) to a unix timestamp in ms.
/// Returns 0 if parsing fails.
fn parse_date_to_ms(text: &str) -> i64 {
    let trimmed = text.trim();
    // Try full date formats first
    for fmt in &["%Y-%m-%d", "%d/%m/%Y", "%m/%d/%Y"] {
        if let Ok(dt) = chrono::NaiveDate::parse_from_str(trimmed, fmt) {
            return dt
                .and_hms_opt(0, 0, 0)
                .map(|ndt| ndt.and_utc().timestamp_millis())
                .unwrap_or(0);
        }
    }
    // Try month+year (no day — default to 1st)
    for fmt in &["%b %Y", "%B %Y"] {
        // NaiveDate needs a day, so prepend "01 " and adjust format
        let with_day = format!("01 {}", trimmed);
        let day_fmt = format!("%d {}", fmt);
        if let Ok(dt) = chrono::NaiveDate::parse_from_str(&with_day, &day_fmt) {
            return dt
                .and_hms_opt(0, 0, 0)
                .map(|ndt| ndt.and_utc().timestamp_millis())
                .unwrap_or(0);
        }
    }
    // Try year-only
    if let Ok(year) = trimmed.parse::<i32>() {
        if (2000..=2100).contains(&year) {
            if let Some(dt) = chrono::NaiveDate::from_ymd_opt(year, 1, 1) {
                return dt
                    .and_hms_opt(0, 0, 0)
                    .map(|ndt| ndt.and_utc().timestamp_millis())
                    .unwrap_or(0);
            }
        }
    }
    0
}

/// Validate a snapshot — returns warnings for suspicious values.
pub fn validate_snapshot(snap: &DarwinWebSnapshot) -> Vec<ScrapeWarning> {
    let mut warnings = Vec::new();
    let t = &snap.ticker;

    if snap.quote <= 0.0 {
        warnings.push(ScrapeWarning {
            ticker: t.clone(),
            field: "quote".into(),
            message: format!("{t}: quote is {:.2} (expected > 0)", snap.quote),
        });
    }
    if snap.dscore <= 0.0 && snap.all_time_return_pct != 0.0 {
        warnings.push(ScrapeWarning {
            ticker: t.clone(),
            field: "dscore".into(),
            message: format!("{t}: D-Score is 0 but has returns — selectors may be broken"),
        });
    }
    if snap.var_monthly <= 0.0 {
        warnings.push(ScrapeWarning {
            ticker: t.clone(),
            field: "var_monthly".into(),
            message: format!("{t}: VaR is {:.2} (expected > 0)", snap.var_monthly),
        });
    }
    // Count how many key fields are zero — if nearly all are 0, page likely didn't load
    let zero_count = [
        snap.quote,
        snap.dscore,
        snap.var_monthly,
        snap.sharpe_ratio,
        snap.all_time_return_pct,
        snap.aum,
    ]
    .iter()
    .filter(|v| **v == 0.0)
    .count();
    if zero_count >= 5 {
        warnings.push(ScrapeWarning {
            ticker: t.clone(),
            field: "all".into(),
            message: format!("{t}: {zero_count}/6 key fields are 0.0 — page may not have loaded"),
        });
    }

    warnings
}

// ── Per-DARWIN Scraping ───────���───────────────────────────────────

/// Build the DARWIN profile URL for a given ticker.
fn darwin_profile_url(ticker: &str) -> String {
    format!(
        "https://www.darwinexzero.com/darwin/{}",
        ticker.to_uppercase()
    )
}

/// Scrape a single DARWIN's profile page — Overview tab (with retry).
pub async fn scrape_darwin(driver: &WebDriver, ticker: &str) -> Result<DarwinWebSnapshot, String> {
    let url = darwin_profile_url(ticker);

    for attempt in 0..=MAX_RETRIES {
        driver
            .goto(&url)
            .await
            .map_err(|e| format!("Navigate to {ticker} profile failed: {e}"))?;
        tokio::time::sleep(PAGE_LOAD_WAIT).await;

        let now_ms = chrono::Utc::now().timestamp_millis();

        // Quote & returns
        let quote = scrape_numeric(driver, selectors::DARWIN_QUOTE).await;
        let daily_return_pct = scrape_numeric(driver, selectors::DARWIN_DAILY_RETURN).await;
        let monthly_return_pct = scrape_numeric(driver, selectors::DARWIN_MONTHLY_RETURN).await;
        let ytd_return_pct = scrape_numeric(driver, selectors::DARWIN_YTD_RETURN).await;
        let all_time_return_pct = scrape_numeric(driver, selectors::DARWIN_ALL_TIME_RETURN).await;

        // D-Score components
        let dscore = scrape_numeric(driver, selectors::DARWIN_DSCORE).await;
        let ds_experience = scrape_numeric(driver, selectors::DARWIN_DS_EXPERIENCE).await;
        let ds_risk_mgmt = scrape_numeric(driver, selectors::DARWIN_DS_RISK_MGMT).await;
        let ds_risk_adjustment = scrape_numeric(driver, selectors::DARWIN_DS_RISK_ADJ).await;
        let ds_performance = scrape_numeric(driver, selectors::DARWIN_DS_PERFORMANCE).await;
        let ds_scalability = scrape_numeric(driver, selectors::DARWIN_DS_SCALABILITY).await;
        let ds_market_correlation = scrape_numeric(driver, selectors::DARWIN_DS_MARKET_CORR).await;

        // Risk metrics
        let var_monthly = scrape_numeric(driver, selectors::DARWIN_VAR).await;
        let max_drawdown_pct = scrape_numeric(driver, selectors::DARWIN_MAX_DD).await;
        let volatility_annual = scrape_numeric(driver, selectors::DARWIN_VOL).await;
        let sharpe_ratio = scrape_numeric(driver, selectors::DARWIN_SHARPE).await;
        let sortino_ratio = scrape_numeric(driver, selectors::DARWIN_SORTINO).await;

        // Investor data
        let investors = scrape_numeric(driver, selectors::DARWIN_INVESTORS).await as u32;
        let aum = scrape_numeric(driver, selectors::DARWIN_AUM).await;
        let capacity_remaining_pct = scrape_numeric(driver, selectors::DARWIN_CAPACITY).await;

        // Exclusion status (auto-detected from page)
        let excluded = driver
            .find(By::Css(selectors::DARWIN_EXCLUDED))
            .await
            .is_ok();
        let exclusion_reason = match driver
            .find(By::Css(selectors::DARWIN_EXCLUSION_REASON))
            .await
        {
            Ok(elem) => elem.text().await.unwrap_or_default(),
            Err(_) => String::new(),
        };

        // Trading stats (strategy analysis tab)
        let total_trades = scrape_numeric(driver, selectors::DARWIN_TOTAL_TRADES).await as u32;
        let win_rate = scrape_numeric(driver, selectors::DARWIN_WIN_RATE).await;
        let profit_factor = scrape_numeric(driver, selectors::DARWIN_PROFIT_FACTOR).await;
        let avg_holding_time_hours = scrape_numeric(driver, selectors::DARWIN_AVG_HOLD_TIME).await;
        let avg_trade_return_pct = scrape_numeric(driver, selectors::DARWIN_AVG_TRADE_RETURN).await;
        let symbols_traded = scrape_numeric(driver, selectors::DARWIN_SYMBOLS_TRADED).await as u32;

        let snap = DarwinWebSnapshot {
            ticker: ticker.to_uppercase(),
            timestamp_ms: now_ms,
            quote,
            daily_return_pct,
            monthly_return_pct,
            ytd_return_pct,
            all_time_return_pct,
            dscore,
            ds_experience,
            ds_risk_mgmt,
            ds_risk_adjustment,
            ds_performance,
            ds_scalability,
            ds_market_correlation,
            var_monthly,
            max_drawdown_pct,
            volatility_annual,
            sharpe_ratio,
            sortino_ratio,
            investors,
            aum,
            capacity_remaining_pct,
            total_trades,
            win_rate,
            profit_factor,
            avg_holding_time_hours,
            avg_trade_return_pct,
            symbols_traded,
            excluded,
            exclusion_reason,
            correlation_portfolio: 0.0, // filled from correlation page
        };

        // Validate — retry if most fields are zero (page didn't load)
        let warnings = validate_snapshot(&snap);
        let critical = warnings.iter().any(|w| w.field == "all");
        if critical && attempt < MAX_RETRIES {
            tracing::warn!(
                "{ticker}: snapshot validation failed (attempt {}/{}) — retrying",
                attempt + 1,
                MAX_RETRIES + 1
            );
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        }

        // Log non-critical warnings
        for w in &warnings {
            if w.field != "all" {
                tracing::warn!("DWX scrape warning: {}", w.message);
            }
        }

        return Ok(snap);
    }

    Err(format!(
        "{ticker}: scrape failed after {} attempts — page did not load",
        MAX_RETRIES + 1
    ))
}

/// Scrape the Return/Performance tab for a single DARWIN.
/// Must be called while already on the DARWIN profile page.
pub async fn scrape_return_tab(driver: &WebDriver, ticker: &str) -> DarwinMonthlyReturns {
    let mut result = DarwinMonthlyReturns {
        ticker: ticker.to_uppercase(),
        rows: Vec::new(),
        cagr: 0.0,
        best_month_pct: 0.0,
        worst_month_pct: 0.0,
        avg_month_pct: 0.0,
        positive_months: 0,
        negative_months: 0,
    };

    // Click Return/Performance tab
    if !click_tab(driver, selectors::TAB_RETURN).await {
        tracing::warn!("{ticker}: Return tab not found — skipping");
        return result;
    }

    // Scrape performance stats
    result.cagr = scrape_numeric(driver, selectors::PERF_CAGR).await;
    result.best_month_pct = scrape_numeric(driver, selectors::PERF_BEST_MONTH).await;
    result.worst_month_pct = scrape_numeric(driver, selectors::PERF_WORST_MONTH).await;
    result.avg_month_pct = scrape_numeric(driver, selectors::PERF_AVG_MONTH).await;
    result.positive_months = scrape_numeric(driver, selectors::PERF_POSITIVE_MONTHS).await as u32;
    result.negative_months = scrape_numeric(driver, selectors::PERF_NEGATIVE_MONTHS).await as u32;

    // Scrape monthly returns table
    if let Ok(table) = driver.find(By::Css(selectors::MONTHLY_RETURNS_TABLE)).await {
        if let Ok(rows) = table
            .find_all(By::Css(selectors::MONTHLY_RETURNS_ROW))
            .await
        {
            for row in &rows {
                if let Ok(cells) = row.find_all(By::Css(selectors::MONTHLY_RETURNS_CELL)).await {
                    if cells.is_empty() {
                        continue;
                    }
                    // First cell is usually the year
                    let year_text = cells[0].text().await.unwrap_or_default();
                    let year = parse_numeric(&year_text) as u16;
                    if year < 2000 || year > 2100 {
                        continue;
                    }

                    let mut months = [None; 12];
                    // Cells 1..=12 are months (Jan..Dec)
                    for (i, cell) in cells.iter().skip(1).take(12).enumerate() {
                        let text = cell.text().await.unwrap_or_default();
                        let trimmed = text.trim();
                        if !trimmed.is_empty() && trimmed != "-" && trimmed != "N/A" {
                            months[i] = Some(parse_numeric(trimmed));
                        }
                    }
                    // Last cell might be year total
                    let year_total = if cells.len() > 13 {
                        let text = cells[cells.len() - 1].text().await.unwrap_or_default();
                        let trimmed = text.trim();
                        if !trimmed.is_empty() && trimmed != "-" {
                            Some(parse_numeric(trimmed))
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    result.rows.push(MonthlyReturnRow {
                        year,
                        months,
                        year_total,
                    });
                }
            }
        }
    }

    result
}

/// Scrape the Risk tab for a single DARWIN.
/// Must be called while already on the DARWIN profile page.
pub async fn scrape_risk_tab(driver: &WebDriver, ticker: &str) -> DarwinVaRHistory {
    let mut result = DarwinVaRHistory {
        ticker: ticker.to_uppercase(),
        points: Vec::new(),
        current_var: 0.0,
        avg_var: 0.0,
        max_var: 0.0,
        min_var: 0.0,
        var_violations: 0,
        drawdown_periods: Vec::new(),
    };

    // Click Risk tab
    if !click_tab(driver, selectors::TAB_RISK).await {
        tracing::warn!("{ticker}: Risk tab not found — skipping");
        return result;
    }

    // Scrape summary risk stats
    result.current_var = scrape_numeric(driver, selectors::RISK_CURRENT_VAR).await;
    result.avg_var = scrape_numeric(driver, selectors::RISK_AVG_VAR).await;
    result.max_var = scrape_numeric(driver, selectors::RISK_MAX_VAR).await;
    result.min_var = scrape_numeric(driver, selectors::RISK_MIN_VAR).await;
    result.var_violations = scrape_numeric(driver, selectors::RISK_VAR_VIOLATIONS).await as u32;

    // Scrape VaR history rows
    if let Ok(rows) = driver.find_all(By::Css(selectors::VAR_HISTORY_ROW)).await {
        for row in &rows {
            let date_attr = row
                .attr("data-var-date")
                .await
                .ok()
                .flatten()
                .unwrap_or_default();
            let ts = parse_date_to_ms(&date_attr);
            let var_text = row.text().await.unwrap_or_default();
            let var_pct = parse_numeric(&var_text);
            if ts > 0 {
                result.points.push(VaRPoint {
                    timestamp_ms: ts,
                    var_pct,
                });
            }
        }
    }

    // Scrape drawdown periods
    if let Ok(rows) = driver
        .find_all(By::Css(selectors::DRAWDOWN_PERIOD_ROW))
        .await
    {
        for row in &rows {
            let start_attr = row
                .attr("data-start")
                .await
                .ok()
                .flatten()
                .unwrap_or_default();
            let end_attr = row
                .attr("data-end")
                .await
                .ok()
                .flatten()
                .unwrap_or_default();
            let depth_text = row.text().await.unwrap_or_default();
            let start_ms = parse_date_to_ms(&start_attr);
            let end_ms = parse_date_to_ms(&end_attr);
            let depth_pct = parse_numeric(&depth_text);
            let recovery_attr = row
                .attr("data-recovery-days")
                .await
                .ok()
                .flatten()
                .unwrap_or_default();
            let recovery_days = parse_numeric(&recovery_attr) as u32;
            if start_ms > 0 {
                result.drawdown_periods.push(DrawdownPeriod {
                    start_ms,
                    end_ms,
                    depth_pct,
                    recovery_days,
                });
            }
        }
    }

    result
}

/// Scrape the Investable Attributes tab for D-Score history.
/// Must be called while already on the DARWIN profile page.
pub async fn scrape_investable_tab(driver: &WebDriver, ticker: &str) -> DarwinDScoreHistory {
    let mut result = DarwinDScoreHistory {
        ticker: ticker.to_uppercase(),
        points: Vec::new(),
    };

    // Click Investable Attributes tab
    if !click_tab(driver, selectors::TAB_INVESTABLE).await {
        tracing::warn!("{ticker}: Investable Attributes tab not found — skipping");
        return result;
    }

    // Scrape D-Score history data points
    if let Ok(rows) = driver
        .find_all(By::Css(selectors::DSCORE_HISTORY_ROW))
        .await
    {
        for row in &rows {
            let date_attr = row
                .attr("data-dscore-date")
                .await
                .ok()
                .flatten()
                .unwrap_or_default();
            let ts = parse_date_to_ms(&date_attr);
            if ts == 0 {
                continue;
            }

            // Try to get individual component values from the row
            let dscore_val = row
                .attr("data-dscore-value")
                .await
                .ok()
                .flatten()
                .map(|v| parse_numeric(&v))
                .unwrap_or(0.0);
            let ex = row
                .attr("data-ex")
                .await
                .ok()
                .flatten()
                .map(|v| parse_numeric(&v))
                .unwrap_or(0.0);
            let rs = row
                .attr("data-rs")
                .await
                .ok()
                .flatten()
                .map(|v| parse_numeric(&v))
                .unwrap_or(0.0);
            let ra = row
                .attr("data-ra")
                .await
                .ok()
                .flatten()
                .map(|v| parse_numeric(&v))
                .unwrap_or(0.0);
            let pf = row
                .attr("data-pf")
                .await
                .ok()
                .flatten()
                .map(|v| parse_numeric(&v))
                .unwrap_or(0.0);
            let sc = row
                .attr("data-sc")
                .await
                .ok()
                .flatten()
                .map(|v| parse_numeric(&v))
                .unwrap_or(0.0);
            let mc = row
                .attr("data-mc")
                .await
                .ok()
                .flatten()
                .map(|v| parse_numeric(&v))
                .unwrap_or(0.0);

            result.points.push(DScorePoint {
                timestamp_ms: ts,
                dscore: dscore_val,
                experience: ex,
                risk_stability: rs,
                risk_adjustment: ra,
                performance: pf,
                scalability: sc,
                market_correlation: mc,
            });
        }
    }

    result
}

/// Scrape the Investor tab for investor count + AuM history.
/// Must be called while already on the DARWIN profile page.
pub async fn scrape_investor_tab(driver: &WebDriver, ticker: &str) -> DarwinInvestorFlow {
    let mut result = DarwinInvestorFlow {
        ticker: ticker.to_uppercase(),
        points: Vec::new(),
        capital_in: 0.0,
        capital_out: 0.0,
        net_flow: 0.0,
        divergence_pct: 0.0,
    };

    // Click Investor tab
    if !click_tab(driver, selectors::TAB_INVESTOR).await {
        tracing::warn!("{ticker}: Investor tab not found — skipping");
        return result;
    }

    // Scrape summary flow stats
    result.capital_in = scrape_numeric(driver, selectors::INVESTOR_CAPITAL_IN).await;
    result.capital_out = scrape_numeric(driver, selectors::INVESTOR_CAPITAL_OUT).await;
    result.net_flow = scrape_numeric(driver, selectors::INVESTOR_NET_FLOW).await;
    result.divergence_pct = scrape_numeric(driver, selectors::INVESTOR_DIVERGENCE).await;

    // Scrape investor flow history rows
    if let Ok(rows) = driver.find_all(By::Css(selectors::INVESTOR_FLOW_ROW)).await {
        for row in &rows {
            let date_attr = row
                .attr("data-investor-date")
                .await
                .ok()
                .flatten()
                .unwrap_or_default();
            let ts = parse_date_to_ms(&date_attr);
            if ts == 0 {
                continue;
            }

            // Get count and AuM from cells within the row
            let count_text = match row.find(By::Css(selectors::INVESTOR_FLOW_COUNT)).await {
                Ok(cell) => cell.text().await.unwrap_or_default(),
                Err(_) => String::new(),
            };
            let aum_text = match row.find(By::Css(selectors::INVESTOR_FLOW_AUM)).await {
                Ok(cell) => cell.text().await.unwrap_or_default(),
                Err(_) => String::new(),
            };

            result.points.push(InvestorFlowPoint {
                timestamp_ms: ts,
                investor_count: parse_numeric(&count_text) as u32,
                aum: parse_numeric(&aum_text),
            });
        }
    }

    result
}

/// Scrape equity curve data points from chart elements on the current page.
async fn scrape_equity_curve(driver: &WebDriver, ticker: &str) -> DarwinEquityCurve {
    let mut points = Vec::new();

    if let Ok(elems) = driver
        .find_all(By::Css(selectors::EQUITY_CURVE_POINT))
        .await
    {
        for elem in &elems {
            let mut ts_attr = elem
                .attr("data-timestamp")
                .await
                .ok()
                .flatten()
                .unwrap_or_default();
            if ts_attr.is_empty() {
                ts_attr = elem
                    .attr("data-date")
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_default();
            }
            let mut val_attr = elem
                .attr("data-equity")
                .await
                .ok()
                .flatten()
                .unwrap_or_default();
            if val_attr.is_empty() {
                val_attr = elem
                    .attr("data-value")
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_default();
            }

            let ts = if ts_attr.contains('-') || ts_attr.contains('/') {
                parse_date_to_ms(&ts_attr)
            } else {
                parse_numeric(&ts_attr) as i64
            };
            let value = parse_numeric(&val_attr);

            if ts > 0 && value > 0.0 {
                points.push(EquityPoint {
                    timestamp_ms: ts,
                    value,
                });
            }
        }
    }

    DarwinEquityCurve {
        ticker: ticker.to_uppercase(),
        points,
    }
}

/// Scrape ALL tabs for a single DARWIN (Overview + Return + Risk + Investable + Investor).
/// Navigates to the DARWIN profile and clicks through each tab.
pub async fn scrape_darwin_full(
    driver: &WebDriver,
    ticker: &str,
) -> Result<
    (
        DarwinWebSnapshot,
        DarwinMonthlyReturns,
        DarwinEquityCurve,
        DarwinVaRHistory,
        DarwinDScoreHistory,
        DarwinInvestorFlow,
    ),
    String,
> {
    // 1. Overview tab (includes navigation to the DARWIN page)
    let snapshot = scrape_darwin(driver, ticker).await?;

    // 2. Return/Performance tab (we're already on the DARWIN profile page)
    let monthly_returns = scrape_return_tab(driver, ticker).await;

    // 3. Scrape equity curve from the Return tab (while we're on it)
    let equity_curve = scrape_equity_curve(driver, ticker).await;

    // 4. Risk tab
    let var_history = scrape_risk_tab(driver, ticker).await;

    // 5. Investable Attributes tab
    let dscore_history = scrape_investable_tab(driver, ticker).await;

    // 6. Investor tab
    let investor_flow = scrape_investor_tab(driver, ticker).await;

    Ok((
        snapshot,
        monthly_returns,
        equity_curve,
        var_history,
        dscore_history,
        investor_flow,
    ))
}

// ── Portfolio-Level Scraping ──────────────────────────────────────

/// Scrape the portfolio correlation matrix from the correlation page.
pub async fn scrape_correlation(
    driver: &WebDriver,
    tickers: &[String],
) -> Result<Vec<DarwinWebCorrelation>, String> {
    // Navigate to portfolio/correlation page
    driver
        .goto("https://www.darwinexzero.com/portfolio/correlation")
        .await
        .map_err(|e| format!("Navigate to correlation page failed: {e}"))?;

    tokio::time::sleep(PAGE_LOAD_WAIT).await;

    let mut correlations = Vec::new();

    // Try to find correlation table cells
    match driver.find_all(By::Css(selectors::CORRELATION_CELL)).await {
        Ok(cells) => {
            for cell in &cells {
                // Try to extract data attributes for darwin pair + correlation value
                let darwin_a = cell
                    .attr("data-darwin-a")
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_default();
                let darwin_b = cell
                    .attr("data-darwin-b")
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_default();
                let corr_text = cell.text().await.unwrap_or_default();
                let correlation = parse_numeric(&corr_text);

                if !darwin_a.is_empty() && !darwin_b.is_empty() {
                    correlations.push(DarwinWebCorrelation {
                        darwin_a,
                        darwin_b,
                        correlation,
                    });
                }
            }
        }
        Err(_) => {
            tracing::warn!("Correlation table not found — selectors may need updating");
        }
    }

    // If table parsing failed, try extracting from page source as fallback
    if correlations.is_empty() && tickers.len() >= 2 {
        tracing::info!("Correlation table empty — site layout may have changed");
    }

    Ok(correlations)
}

/// Scrape portfolio performance from /portfolio/performance.
pub async fn scrape_portfolio_performance(driver: &WebDriver) -> Option<PortfolioPerformance> {
    if driver
        .goto("https://www.darwinexzero.com/portfolio/performance")
        .await
        .is_err()
    {
        tracing::warn!("Failed to navigate to portfolio/performance");
        return None;
    }
    tokio::time::sleep(PAGE_LOAD_WAIT).await;

    let total_return_pct = scrape_numeric(driver, selectors::PORTFOLIO_TOTAL_RETURN).await;
    let cagr = scrape_numeric(driver, selectors::PORTFOLIO_CAGR).await;
    let best_month_pct = scrape_numeric(driver, selectors::PORTFOLIO_BEST_MONTH).await;
    let worst_month_pct = scrape_numeric(driver, selectors::PORTFOLIO_WORST_MONTH).await;

    // Scrape monthly returns table (same format as per-DARWIN)
    let mut monthly_returns = Vec::new();
    if let Ok(table) = driver
        .find(By::Css(selectors::PORTFOLIO_MONTHLY_TABLE))
        .await
    {
        if let Ok(rows) = table
            .find_all(By::Css(selectors::MONTHLY_RETURNS_ROW))
            .await
        {
            for row in &rows {
                if let Ok(cells) = row.find_all(By::Css(selectors::MONTHLY_RETURNS_CELL)).await {
                    if cells.is_empty() {
                        continue;
                    }
                    let year_text = cells[0].text().await.unwrap_or_default();
                    let year = parse_numeric(&year_text) as u16;
                    if year < 2000 || year > 2100 {
                        continue;
                    }

                    let mut months = [None; 12];
                    for (i, cell) in cells.iter().skip(1).take(12).enumerate() {
                        let text = cell.text().await.unwrap_or_default();
                        let trimmed = text.trim();
                        if !trimmed.is_empty() && trimmed != "-" && trimmed != "N/A" {
                            months[i] = Some(parse_numeric(trimmed));
                        }
                    }
                    let year_total = if cells.len() > 13 {
                        let text = cells[cells.len() - 1].text().await.unwrap_or_default();
                        let trimmed = text.trim();
                        if !trimmed.is_empty() && trimmed != "-" {
                            Some(parse_numeric(trimmed))
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    monthly_returns.push(MonthlyReturnRow {
                        year,
                        months,
                        year_total,
                    });
                }
            }
        }
    }

    // Scrape equity curve
    let mut equity_points = Vec::new();
    if let Ok(elems) = driver
        .find_all(By::Css(selectors::EQUITY_CURVE_POINT))
        .await
    {
        for elem in &elems {
            let mut ts_attr = elem
                .attr("data-timestamp")
                .await
                .ok()
                .flatten()
                .unwrap_or_default();
            if ts_attr.is_empty() {
                ts_attr = elem
                    .attr("data-date")
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_default();
            }
            let mut val_attr = elem
                .attr("data-equity")
                .await
                .ok()
                .flatten()
                .unwrap_or_default();
            if val_attr.is_empty() {
                val_attr = elem
                    .attr("data-value")
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_default();
            }
            let ts = if ts_attr.contains('-') {
                parse_date_to_ms(&ts_attr)
            } else {
                parse_numeric(&ts_attr) as i64
            };
            let value = parse_numeric(&val_attr);
            if ts > 0 && value != 0.0 {
                equity_points.push(EquityPoint {
                    timestamp_ms: ts,
                    value,
                });
            }
        }
    }

    Some(PortfolioPerformance {
        total_return_pct,
        cagr,
        best_month_pct,
        worst_month_pct,
        monthly_returns,
        equity_points,
    })
}

/// Scrape portfolio risk from /portfolio/risk.
pub async fn scrape_portfolio_risk(driver: &WebDriver) -> Option<PortfolioRisk> {
    if driver
        .goto("https://www.darwinexzero.com/portfolio/risk")
        .await
        .is_err()
    {
        tracing::warn!("Failed to navigate to portfolio/risk");
        return None;
    }
    tokio::time::sleep(PAGE_LOAD_WAIT).await;

    let current_var = scrape_numeric(driver, selectors::PORTFOLIO_VAR).await;
    let max_drawdown_pct = scrape_numeric(driver, selectors::PORTFOLIO_MAX_DD).await;
    let diversification_benefit_pct =
        scrape_numeric(driver, selectors::PORTFOLIO_DIVERSIFICATION).await;

    // VaR history
    let mut var_history = Vec::new();
    if let Ok(rows) = driver
        .find_all(By::Css(selectors::PORTFOLIO_VAR_HISTORY_ROW))
        .await
    {
        for row in &rows {
            let date_attr = row
                .attr("data-portfolio-var-date")
                .await
                .ok()
                .flatten()
                .unwrap_or_default();
            let ts = parse_date_to_ms(&date_attr);
            let var_text = row.text().await.unwrap_or_default();
            let var_pct = parse_numeric(&var_text);
            if ts > 0 {
                var_history.push(VaRPoint {
                    timestamp_ms: ts,
                    var_pct,
                });
            }
        }
    }

    Some(PortfolioRisk {
        current_var,
        max_drawdown_pct,
        diversification_benefit_pct,
        var_history,
    })
}

/// Scrape portfolio allocation from /portfolio/allocation.
pub async fn scrape_portfolio_allocation(driver: &WebDriver) -> Vec<DarwinAllocation> {
    let mut allocations = Vec::new();

    if driver
        .goto("https://www.darwinexzero.com/portfolio/allocation")
        .await
        .is_err()
    {
        tracing::warn!("Failed to navigate to portfolio/allocation");
        return allocations;
    }
    tokio::time::sleep(PAGE_LOAD_WAIT).await;

    if let Ok(rows) = driver.find_all(By::Css(selectors::ALLOCATION_ROW)).await {
        for row in &rows {
            let ticker_text = match row.find(By::Css(selectors::ALLOCATION_TICKER)).await {
                Ok(cell) => cell.text().await.unwrap_or_default(),
                Err(_) => continue,
            };
            let ticker = ticker_text.trim().to_uppercase();
            if ticker.is_empty() || !ticker.chars().all(|c| c.is_ascii_alphanumeric()) {
                continue;
            }

            let weight_pct = match row.find(By::Css(selectors::ALLOCATION_WEIGHT)).await {
                Ok(cell) => parse_numeric(&cell.text().await.unwrap_or_default()),
                Err(_) => 0.0,
            };
            let invested = match row.find(By::Css(selectors::ALLOCATION_INVESTED)).await {
                Ok(cell) => parse_numeric(&cell.text().await.unwrap_or_default()),
                Err(_) => 0.0,
            };
            let pnl = match row.find(By::Css(selectors::ALLOCATION_PNL)).await {
                Ok(cell) => parse_numeric(&cell.text().await.unwrap_or_default()),
                Err(_) => 0.0,
            };

            allocations.push(DarwinAllocation {
                ticker,
                weight_pct,
                invested,
                pnl,
            });
        }
    }

    allocations
}

// ── Full Scrape Cycle ─────────────────────────────────────────────

/// Full scrape cycle: all managed DARWINs (all tabs) + portfolio pages.
/// Caches results via the provided put_kv function.
pub async fn scrape_all<F>(
    driver: &WebDriver,
    config: &DarwinWebConfig,
    mut cache_fn: F,
) -> Result<DarwinWebUpdate, String>
where
    F: FnMut(&str, &str) -> Result<(), String>,
{
    let mut snapshots = Vec::with_capacity(config.managed_darwins.len());
    let mut monthly_returns = Vec::with_capacity(config.managed_darwins.len());
    let mut equity_curves = Vec::with_capacity(config.managed_darwins.len());
    let mut var_histories = Vec::with_capacity(config.managed_darwins.len());
    let mut dscore_histories = Vec::with_capacity(config.managed_darwins.len());
    let mut investor_flows = Vec::with_capacity(config.managed_darwins.len());

    for ticker in &config.managed_darwins {
        match scrape_darwin_full(driver, ticker).await {
            Ok((snapshot, mr, ec, vh, dh, ifl)) => {
                // Cache individual results
                if let Ok(json) = serde_json::to_string(&snapshot) {
                    let _ = cache_fn(&cache_keys::snapshot(ticker), &json);
                }
                if let Ok(json) = serde_json::to_string(&mr) {
                    let _ = cache_fn(&cache_keys::monthly_returns(ticker), &json);
                }
                if let Ok(json) = serde_json::to_string(&ec) {
                    let _ = cache_fn(&cache_keys::equity_curve(ticker), &json);
                }
                if let Ok(json) = serde_json::to_string(&vh) {
                    let _ = cache_fn(&cache_keys::var_history(ticker), &json);
                }
                if let Ok(json) = serde_json::to_string(&dh) {
                    let _ = cache_fn(&cache_keys::dscore_history(ticker), &json);
                }
                if let Ok(json) = serde_json::to_string(&ifl) {
                    let _ = cache_fn(&cache_keys::investor_flow(ticker), &json);
                }
                snapshots.push(snapshot);
                monthly_returns.push(mr);
                equity_curves.push(ec);
                var_histories.push(vh);
                dscore_histories.push(dh);
                investor_flows.push(ifl);
            }
            Err(e) => {
                tracing::error!("Failed to scrape {ticker}: {e}");
            }
        }
    }

    // ── Portfolio-level pages ──────────────────────────────────────

    // Scrape correlation matrix
    let correlations = scrape_correlation(driver, &config.managed_darwins)
        .await
        .unwrap_or_default();

    // Scrape portfolio performance
    let portfolio_performance = scrape_portfolio_performance(driver).await;
    if let Some(ref pp) = portfolio_performance {
        if let Ok(json) = serde_json::to_string(pp) {
            let _ = cache_fn(cache_keys::PORTFOLIO_PERF, &json);
        }
    }

    // Scrape portfolio risk
    let portfolio_risk = scrape_portfolio_risk(driver).await;
    if let Some(ref pr) = portfolio_risk {
        if let Ok(json) = serde_json::to_string(pr) {
            let _ = cache_fn(cache_keys::PORTFOLIO_RISK, &json);
        }
    }

    // Scrape portfolio allocation
    let allocations = scrape_portfolio_allocation(driver).await;
    if let Ok(json) = serde_json::to_string(&allocations) {
        let _ = cache_fn(cache_keys::ALLOCATIONS, &json);
    }

    // ── Correlation analysis ──────────────────────────────────────

    // Determine which DARWINs are active (not excluded)
    let active: HashSet<String> = snapshots
        .iter()
        .filter(|s| !s.excluded)
        .map(|s| s.ticker.clone())
        .filter(|t| !config.excluded_darwins.contains(t))
        .collect();

    // Log excluded DARWINs
    for snap in &snapshots {
        if snap.excluded {
            tracing::warn!(
                "DARWIN {} is EXCLUDED on Darwinex: {}",
                snap.ticker,
                if snap.exclusion_reason.is_empty() {
                    "no reason given"
                } else {
                    &snap.exclusion_reason
                }
            );
        }
    }

    // Update correlation_portfolio on snapshots if we have correlation data
    for snap in &mut snapshots {
        let (sum, count) = correlations
            .iter()
            .filter(|c| {
                let involves_snap = c.darwin_a == snap.ticker || c.darwin_b == snap.ticker;
                let other = if c.darwin_a == snap.ticker {
                    &c.darwin_b
                } else {
                    &c.darwin_a
                };
                involves_snap && active.contains(other)
            })
            .fold((0.0_f64, 0_u32), |(s, n), c| (s + c.correlation, n + 1));
        if count > 0 {
            snap.correlation_portfolio = sum / count as f64;
        }
    }

    // Check for correlation violations among active DARWINs
    let mut correlation_alerts = Vec::new();
    for corr in &correlations {
        if active.contains(&corr.darwin_a)
            && active.contains(&corr.darwin_b)
            && corr.correlation.abs() >= config.correlation_alert_threshold
        {
            correlation_alerts.push(CorrelationAlert {
                darwin_a: corr.darwin_a.clone(),
                darwin_b: corr.darwin_b.clone(),
                correlation: corr.correlation,
                threshold: config.correlation_alert_threshold,
                suggestion: suggest_correlation_fix(
                    &corr.darwin_a,
                    &corr.darwin_b,
                    corr.correlation,
                ),
            });
        }
    }

    if !correlation_alerts.is_empty() {
        tracing::warn!(
            "CORRELATION ALERT: {} pairs exceed {:.2} threshold",
            correlation_alerts.len(),
            config.correlation_alert_threshold
        );
        for alert in &correlation_alerts {
            tracing::warn!(
                "  {} × {} = {:.4} (threshold: {:.2})",
                alert.darwin_a,
                alert.darwin_b,
                alert.correlation,
                alert.threshold
            );
        }
    }

    let now_ms = chrono::Utc::now().timestamp_millis();

    // Cache correlation matrix
    if let Ok(json) = serde_json::to_string(&correlations) {
        let _ = cache_fn(cache_keys::CORRELATION, &json);
    }

    // Cache last scrape timestamp
    let _ = cache_fn(cache_keys::LAST_SCRAPE, &chrono::Utc::now().to_rfc3339());

    let update = DarwinWebUpdate {
        snapshots,
        correlations,
        correlation_alerts,
        timestamp_ms: now_ms,
        monthly_returns,
        equity_curves,
        var_histories,
        dscore_histories,
        investor_flows,
        portfolio_performance,
        portfolio_risk,
        allocations,
    };

    Ok(update)
}

/// Full login + scrape flow with cookie restoration attempt.
pub async fn login_and_scrape<F>(
    driver: &WebDriver,
    email: &str,
    password: &str,
    cached_cookies: Option<&[SerializableCookie]>,
    config: &DarwinWebConfig,
    cache_fn: F,
) -> Result<DarwinWebUpdate, String>
where
    F: FnMut(&str, &str) -> Result<(), String>,
{
    // Try restoring cookies first
    let mut need_login = true;
    if let Some(cookies) = cached_cookies {
        if !cookies.is_empty() {
            tracing::info!("Restoring {} cached cookies...", cookies.len());
            let _ = restore_cookies(driver, cookies).await;
            if is_session_valid(driver).await {
                tracing::info!("Session restored from cookies — skipping login");
                need_login = false;
            } else {
                tracing::info!("Cached cookies expired — proceeding with login");
            }
        }
    }

    if need_login {
        login(driver, email, password).await?;
    }

    scrape_all(driver, config, cache_fn).await
}

// ── Config Helpers ─────────────────────────────────────────────────

impl DarwinWebConfig {
    /// Validate and deduplicate managed DARWIN tickers.
    /// Rejects tickers with special characters (only alphanumeric allowed).
    pub fn normalize(&mut self) {
        self.managed_darwins = self
            .managed_darwins
            .iter()
            .map(|t| t.trim().to_uppercase())
            .filter(|t| !t.is_empty() && t.chars().all(|c| c.is_ascii_alphanumeric()))
            .collect();
        self.managed_darwins.sort();
        self.managed_darwins.dedup();
        self.excluded_darwins = self
            .excluded_darwins
            .iter()
            .map(|t| t.trim().to_uppercase())
            .filter(|t| !t.is_empty() && t.chars().all(|c| c.is_ascii_alphanumeric()))
            .collect();
        self.excluded_darwins.sort();
        self.excluded_darwins.dedup();
        if self.scrape_minute > 59 {
            self.scrape_minute = 20;
        }
        if self.correlation_alert_threshold <= 0.0 || self.correlation_alert_threshold > 1.0 {
            self.correlation_alert_threshold = 0.95;
        }
    }

    /// Return managed DARWINs that are NOT excluded (for correlation analysis).
    pub fn active_darwins(&self) -> Vec<&str> {
        self.managed_darwins
            .iter()
            .filter(|t| !self.excluded_darwins.contains(t))
            .map(|t| t.as_str())
            .collect()
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_snapshot() -> DarwinWebSnapshot {
        DarwinWebSnapshot {
            ticker: "TPN".to_string(),
            timestamp_ms: 1712764800000,
            quote: 123.45,
            daily_return_pct: 0.5,
            monthly_return_pct: 2.1,
            ytd_return_pct: 8.3,
            all_time_return_pct: 45.2,
            dscore: 65.0,
            ds_experience: 8.0,
            ds_risk_mgmt: 7.5,
            ds_risk_adjustment: 6.0,
            ds_performance: 9.0,
            ds_scalability: 5.0,
            ds_market_correlation: 4.0,
            var_monthly: 4.2,
            max_drawdown_pct: 12.5,
            volatility_annual: 15.3,
            sharpe_ratio: 1.8,
            sortino_ratio: 2.1,
            investors: 42,
            aum: 150000.0,
            capacity_remaining_pct: 80.0,
            total_trades: 1500,
            win_rate: 62.3,
            profit_factor: 1.85,
            avg_holding_time_hours: 48.5,
            avg_trade_return_pct: 0.12,
            symbols_traded: 8,
            excluded: false,
            exclusion_reason: String::new(),
            correlation_portfolio: 0.35,
        }
    }

    #[test]
    fn snapshot_roundtrip() {
        let snap = test_snapshot();
        let json = serde_json::to_string(&snap).unwrap();
        let back: DarwinWebSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.ticker, "TPN");
        assert!((back.quote - 123.45).abs() < f64::EPSILON);
        assert_eq!(back.investors, 42);
        assert!((back.ds_experience - 8.0).abs() < f64::EPSILON);
        assert_eq!(back.total_trades, 1500);
        assert!((back.sharpe_ratio - 1.8).abs() < f64::EPSILON);
    }

    #[test]
    fn correlation_roundtrip() {
        let corr = DarwinWebCorrelation {
            darwin_a: "TPN".to_string(),
            darwin_b: "AJT".to_string(),
            correlation: 0.42,
        };
        let json = serde_json::to_string(&corr).unwrap();
        let back: DarwinWebCorrelation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.darwin_a, "TPN");
        assert!((back.correlation - 0.42).abs() < f64::EPSILON);
    }

    #[test]
    fn config_roundtrip() {
        let cfg = DarwinWebConfig {
            managed_darwins: vec!["TPN".to_string(), "AJT".to_string()],
            excluded_darwins: vec!["MFSO".to_string()],
            auto_scrape: true,
            scrape_minute: 20,
            correlation_alert_threshold: 0.95,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: DarwinWebConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.managed_darwins.len(), 2);
        assert_eq!(back.excluded_darwins, vec!["MFSO"]);
        assert!(back.auto_scrape);
    }

    #[test]
    fn config_deny_unknown_fields() {
        let json = r#"{"managed_darwins":[],"excluded_darwins":[],"auto_scrape":false,"scrape_minute":20,"correlation_alert_threshold":0.95,"extra":"bad"}"#;
        assert!(serde_json::from_str::<DarwinWebConfig>(json).is_err());
    }

    #[test]
    fn config_active_darwins_excludes_mfso() {
        let cfg = DarwinWebConfig {
            managed_darwins: vec!["TPN".to_string(), "AJT".to_string(), "MFSO".to_string()],
            excluded_darwins: vec!["MFSO".to_string()],
            auto_scrape: false,
            scrape_minute: 20,
            correlation_alert_threshold: 0.95,
        };
        let active = cfg.active_darwins();
        assert_eq!(active, vec!["TPN", "AJT"]); // MFSO excluded
        assert!(!active.contains(&"MFSO"));
    }

    #[test]
    fn correlation_alert_with_suggestion() {
        let alert = CorrelationAlert {
            darwin_a: "TPN".to_string(),
            darwin_b: "AJT".to_string(),
            correlation: 0.97,
            threshold: 0.95,
            suggestion: suggest_correlation_fix("TPN", "AJT", 0.97),
        };
        assert!(alert.correlation >= alert.threshold);
        assert!(alert.suggestion.contains("HIGH"));
        assert!(alert.suggestion.contains("TPN"));
    }

    #[test]
    fn correlation_fix_critical() {
        let s = suggest_correlation_fix("X", "Y", 0.99);
        assert!(s.contains("CRITICAL"));
    }

    #[test]
    fn correlation_fix_warning() {
        let s = suggest_correlation_fix("X", "Y", 0.90);
        assert!(s.contains("WARNING"));
    }

    #[test]
    fn cookie_serialization() {
        let cookie = SerializableCookie {
            name: "session_id".to_string(),
            value: "abc123".to_string(),
            domain: Some(".darwinexzero.com".to_string()),
            path: Some("/".to_string()),
        };
        let json = serde_json::to_string(&cookie).unwrap();
        let back: SerializableCookie = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "session_id");
        assert_eq!(back.domain.as_deref(), Some(".darwinexzero.com"));
    }

    #[test]
    fn credential_key_constants() {
        assert_eq!(keys::DARWINEX_EMAIL, "darwinex_email");
        assert_eq!(keys::DARWINEX_PASSWORD, "darwinex_password");
    }

    #[test]
    fn config_normalize_dedup_and_uppercase() {
        let mut cfg = DarwinWebConfig {
            managed_darwins: vec![
                "tpn".to_string(),
                "AJT".to_string(),
                "tpn".to_string(),    // duplicate
                " xuqf ".to_string(), // whitespace
            ],
            excluded_darwins: vec!["mfso".to_string(), "MFSO".to_string()],
            auto_scrape: false,
            scrape_minute: 99,                // invalid — should be clamped
            correlation_alert_threshold: 2.0, // invalid — should be clamped
        };
        cfg.normalize();
        assert_eq!(cfg.managed_darwins, vec!["AJT", "TPN", "XUQF"]);
        assert_eq!(cfg.excluded_darwins, vec!["MFSO"]); // deduped + uppercased
        assert_eq!(cfg.scrape_minute, 20); // clamped from 99
        assert!((cfg.correlation_alert_threshold - 0.95).abs() < f64::EPSILON); // clamped
    }

    #[test]
    fn cache_key_format() {
        assert_eq!(cache_keys::snapshot("tpn"), "dwx_web:TPN:snapshot");
        assert_eq!(cache_keys::COOKIES, "dwx_web:cookies");
        assert_eq!(cache_keys::CONFIG, "dwx_web:config");
        assert_eq!(cache_keys::CORRELATION, "dwx_web:correlation");
        assert_eq!(cache_keys::LAST_SCRAPE, "dwx_web:last_scrape");
        assert_eq!(cache_keys::PORTFOLIO_PERF, "dwx_web:portfolio_performance");
        assert_eq!(cache_keys::PORTFOLIO_RISK, "dwx_web:portfolio_risk");
        assert_eq!(cache_keys::ALLOCATIONS, "dwx_web:allocations");
        assert_eq!(
            cache_keys::monthly_returns("tpn"),
            "dwx_web:TPN:monthly_returns"
        );
        assert_eq!(cache_keys::equity_curve("ajt"), "dwx_web:AJT:equity_curve");
        assert_eq!(cache_keys::var_history("XUQF"), "dwx_web:XUQF:var_history");
        assert_eq!(
            cache_keys::dscore_history("tpn"),
            "dwx_web:TPN:dscore_history"
        );
        assert_eq!(
            cache_keys::investor_flow("ajt"),
            "dwx_web:AJT:investor_flow"
        );
    }

    #[test]
    fn parse_numeric_various_formats() {
        assert!((parse_numeric("123.45") - 123.45).abs() < f64::EPSILON);
        assert!((parse_numeric("$1,234.56") - 1234.56).abs() < f64::EPSILON);
        assert!((parse_numeric("-2.5%") - -2.5).abs() < f64::EPSILON);
        assert!((parse_numeric("N/A") - 0.0).abs() < f64::EPSILON);
        assert!((parse_numeric("") - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn snapshot_deny_unknown_fields() {
        let snap = test_snapshot();
        let mut json = serde_json::to_string(&snap).unwrap();
        json.pop(); // remove '}'
        json.push_str(r#","extra":1}"#);
        assert!(serde_json::from_str::<DarwinWebSnapshot>(&json).is_err());
    }

    #[test]
    fn parse_date_to_ms_formats() {
        assert!(parse_date_to_ms("2024-01-15") > 0);
        assert!(parse_date_to_ms("15/01/2024") > 0);
        assert!(parse_date_to_ms("Jan 2024") > 0);
        assert!(parse_date_to_ms("2024") > 0);
        assert_eq!(parse_date_to_ms(""), 0);
        assert_eq!(parse_date_to_ms("garbage"), 0);
    }

    #[test]
    fn validate_snapshot_healthy() {
        let snap = test_snapshot();
        let warnings = validate_snapshot(&snap);
        assert!(
            warnings.is_empty(),
            "healthy snapshot should have no warnings"
        );
    }

    #[test]
    fn validate_snapshot_all_zeros() {
        let mut snap = test_snapshot();
        snap.quote = 0.0;
        snap.dscore = 0.0;
        snap.var_monthly = 0.0;
        snap.sharpe_ratio = 0.0;
        snap.all_time_return_pct = 0.0;
        snap.aum = 0.0;
        let warnings = validate_snapshot(&snap);
        assert!(
            warnings.iter().any(|w| w.field == "all"),
            "all-zeros snapshot should trigger 'all' warning"
        );
    }

    #[test]
    fn validate_snapshot_zero_quote() {
        let mut snap = test_snapshot();
        snap.quote = 0.0;
        let warnings = validate_snapshot(&snap);
        assert!(warnings.iter().any(|w| w.field == "quote"));
    }

    #[test]
    fn monthly_return_row_roundtrip() {
        let row = MonthlyReturnRow {
            year: 2024,
            months: [
                Some(1.2),
                Some(-0.5),
                None,
                Some(2.1),
                None,
                None,
                Some(0.3),
                None,
                Some(-1.1),
                Some(0.8),
                None,
                Some(3.2),
            ],
            year_total: Some(6.0),
        };
        let json = serde_json::to_string(&row).unwrap();
        let back: MonthlyReturnRow = serde_json::from_str(&json).unwrap();
        assert_eq!(back.year, 2024);
        assert_eq!(back.months[0], Some(1.2));
        assert_eq!(back.months[2], None);
        assert_eq!(back.year_total, Some(6.0));
    }

    #[test]
    fn equity_point_roundtrip() {
        let pt = EquityPoint {
            timestamp_ms: 1700000000000,
            value: 125.67,
        };
        let json = serde_json::to_string(&pt).unwrap();
        let back: EquityPoint = serde_json::from_str(&json).unwrap();
        assert_eq!(back.timestamp_ms, 1700000000000);
        assert!((back.value - 125.67).abs() < f64::EPSILON);
    }

    #[test]
    fn var_point_roundtrip() {
        let pt = VaRPoint {
            timestamp_ms: 1700000000000,
            var_pct: 4.5,
        };
        let json = serde_json::to_string(&pt).unwrap();
        let back: VaRPoint = serde_json::from_str(&json).unwrap();
        assert!((back.var_pct - 4.5).abs() < f64::EPSILON);
    }

    #[test]
    fn drawdown_period_roundtrip() {
        let dd = DrawdownPeriod {
            start_ms: 1700000000000,
            end_ms: 1700500000000,
            depth_pct: 8.3,
            recovery_days: 15,
        };
        let json = serde_json::to_string(&dd).unwrap();
        let back: DrawdownPeriod = serde_json::from_str(&json).unwrap();
        assert_eq!(back.recovery_days, 15);
        assert!((back.depth_pct - 8.3).abs() < f64::EPSILON);
    }

    #[test]
    fn dscore_point_roundtrip() {
        let pt = DScorePoint {
            timestamp_ms: 1700000000000,
            dscore: 65.0,
            experience: 8.0,
            risk_stability: 7.5,
            risk_adjustment: 6.0,
            performance: 9.0,
            scalability: 5.0,
            market_correlation: 4.0,
        };
        let json = serde_json::to_string(&pt).unwrap();
        let back: DScorePoint = serde_json::from_str(&json).unwrap();
        assert!((back.dscore - 65.0).abs() < f64::EPSILON);
        assert!((back.experience - 8.0).abs() < f64::EPSILON);
    }

    #[test]
    fn investor_flow_point_roundtrip() {
        let pt = InvestorFlowPoint {
            timestamp_ms: 1700000000000,
            investor_count: 42,
            aum: 150000.0,
        };
        let json = serde_json::to_string(&pt).unwrap();
        let back: InvestorFlowPoint = serde_json::from_str(&json).unwrap();
        assert_eq!(back.investor_count, 42);
    }

    #[test]
    fn darwin_allocation_roundtrip() {
        let alloc = DarwinAllocation {
            ticker: "TPN".to_string(),
            weight_pct: 25.0,
            invested: 50000.0,
            pnl: 1234.56,
        };
        let json = serde_json::to_string(&alloc).unwrap();
        let back: DarwinAllocation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.ticker, "TPN");
        assert!((back.weight_pct - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn portfolio_performance_roundtrip() {
        let pp = PortfolioPerformance {
            total_return_pct: 45.2,
            cagr: 12.5,
            best_month_pct: 8.3,
            worst_month_pct: -5.1,
            monthly_returns: vec![MonthlyReturnRow {
                year: 2024,
                months: [Some(1.0); 12],
                year_total: Some(12.0),
            }],
            equity_points: vec![EquityPoint {
                timestamp_ms: 1700000000000,
                value: 100.0,
            }],
        };
        let json = serde_json::to_string(&pp).unwrap();
        let back: PortfolioPerformance = serde_json::from_str(&json).unwrap();
        assert!((back.cagr - 12.5).abs() < f64::EPSILON);
        assert_eq!(back.monthly_returns.len(), 1);
    }

    #[test]
    fn portfolio_risk_roundtrip() {
        let pr = PortfolioRisk {
            current_var: 4.5,
            max_drawdown_pct: 12.0,
            diversification_benefit_pct: 15.3,
            var_history: vec![VaRPoint {
                timestamp_ms: 1700000000000,
                var_pct: 4.5,
            }],
        };
        let json = serde_json::to_string(&pr).unwrap();
        let back: PortfolioRisk = serde_json::from_str(&json).unwrap();
        assert!((back.diversification_benefit_pct - 15.3).abs() < f64::EPSILON);
    }

    #[test]
    fn darwin_monthly_returns_roundtrip() {
        let mr = DarwinMonthlyReturns {
            ticker: "TPN".to_string(),
            rows: vec![MonthlyReturnRow {
                year: 2024,
                months: [
                    Some(1.2),
                    None,
                    Some(-0.5),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ],
                year_total: Some(0.7),
            }],
            cagr: 10.5,
            best_month_pct: 5.0,
            worst_month_pct: -3.0,
            avg_month_pct: 0.8,
            positive_months: 8,
            negative_months: 4,
        };
        let json = serde_json::to_string(&mr).unwrap();
        let back: DarwinMonthlyReturns = serde_json::from_str(&json).unwrap();
        assert_eq!(back.ticker, "TPN");
        assert_eq!(back.positive_months, 8);
    }

    #[test]
    fn darwin_var_history_roundtrip() {
        let vh = DarwinVaRHistory {
            ticker: "TPN".to_string(),
            points: vec![VaRPoint {
                timestamp_ms: 1700000000000,
                var_pct: 4.5,
            }],
            current_var: 4.5,
            avg_var: 4.0,
            max_var: 6.5,
            min_var: 2.1,
            var_violations: 3,
            drawdown_periods: vec![DrawdownPeriod {
                start_ms: 1700000000000,
                end_ms: 1700500000000,
                depth_pct: 8.3,
                recovery_days: 15,
            }],
        };
        let json = serde_json::to_string(&vh).unwrap();
        let back: DarwinVaRHistory = serde_json::from_str(&json).unwrap();
        assert_eq!(back.var_violations, 3);
        assert_eq!(back.drawdown_periods.len(), 1);
    }

    #[test]
    fn darwin_dscore_history_roundtrip() {
        let dh = DarwinDScoreHistory {
            ticker: "TPN".to_string(),
            points: vec![DScorePoint {
                timestamp_ms: 1700000000000,
                dscore: 65.0,
                experience: 8.0,
                risk_stability: 7.5,
                risk_adjustment: 6.0,
                performance: 9.0,
                scalability: 5.0,
                market_correlation: 4.0,
            }],
        };
        let json = serde_json::to_string(&dh).unwrap();
        let back: DarwinDScoreHistory = serde_json::from_str(&json).unwrap();
        assert_eq!(back.points.len(), 1);
    }

    #[test]
    fn darwin_investor_flow_roundtrip() {
        let ifl = DarwinInvestorFlow {
            ticker: "TPN".to_string(),
            points: vec![InvestorFlowPoint {
                timestamp_ms: 1700000000000,
                investor_count: 42,
                aum: 150000.0,
            }],
            capital_in: 200000.0,
            capital_out: 50000.0,
            net_flow: 150000.0,
            divergence_pct: 1.5,
        };
        let json = serde_json::to_string(&ifl).unwrap();
        let back: DarwinInvestorFlow = serde_json::from_str(&json).unwrap();
        assert!((back.net_flow - 150000.0).abs() < f64::EPSILON);
        assert!((back.divergence_pct - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn darwin_web_update_roundtrip() {
        let update = DarwinWebUpdate {
            snapshots: vec![test_snapshot()],
            correlations: vec![DarwinWebCorrelation {
                darwin_a: "TPN".into(),
                darwin_b: "AJT".into(),
                correlation: 0.42,
            }],
            correlation_alerts: Vec::new(),
            timestamp_ms: 1700000000000,
            monthly_returns: Vec::new(),
            equity_curves: Vec::new(),
            var_histories: Vec::new(),
            dscore_histories: Vec::new(),
            investor_flows: Vec::new(),
            portfolio_performance: None,
            portfolio_risk: None,
            allocations: Vec::new(),
        };
        let json = serde_json::to_string(&update).unwrap();
        let back: DarwinWebUpdate = serde_json::from_str(&json).unwrap();
        assert_eq!(back.snapshots.len(), 1);
        assert_eq!(back.correlations.len(), 1);
    }
}
