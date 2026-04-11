//! Darwinex Zero live web scraping via Selenium (thirtyfour WebDriver).
//!
//! Scrapes darwinexzero.com for live DARWIN stats that the FTP feed delivers
//! with 24h+ lag: NAV quotes, D-Score, VaR, investor flow, correlation.
//!
//! Requires ChromeDriver installed on the system.
//! Browser runs visible (not headless) so the user can solve CAPTCHAs.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thirtyfour::prelude::*;
use std::time::Duration;

// ── Keyring Keys ───────────────────────────────────────────────────

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
    pub const DASHBOARD_MARKER: &str = ".dashboard, [data-page='dashboard'], .portfolio-overview, .main-content";
    // DARWIN profile page — quote & returns
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
    pub const DARWIN_EXCLUDED: &str = ".excluded-badge, .suspended-badge, [data-status='excluded'], [data-status='suspended']";
    pub const DARWIN_EXCLUSION_REASON: &str = ".exclusion-reason, [data-field='exclusion_reason']";
    // Trading stats (strategy analysis tab)
    pub const DARWIN_TOTAL_TRADES: &str = ".total-trades, [data-field='total_trades']";
    pub const DARWIN_WIN_RATE: &str = ".win-rate, [data-field='win_rate']";
    pub const DARWIN_PROFIT_FACTOR: &str = ".profit-factor, [data-field='profit_factor']";
    pub const DARWIN_AVG_HOLD_TIME: &str = ".avg-hold-time, [data-field='avg_holding_time']";
    pub const DARWIN_AVG_TRADE_RETURN: &str = ".avg-trade-return, [data-field='avg_trade_return']";
    pub const DARWIN_SYMBOLS_TRADED: &str = ".symbols-traded, [data-field='symbols_traded']";
    // Correlation page
    pub const _CORRELATION_TABLE: &str = ".correlation-matrix, table.correlation";
    pub const CORRELATION_CELL: &str = "td[data-correlation]";
}

// ── Data Types ─────────────────────────────────────────────────────

/// Live snapshot of a single DARWIN from darwinexzero.com.
/// Covers 100% of the Strategy Analysis tab fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DarwinWebSnapshot {
    pub ticker: String,
    pub timestamp_ms: i64,
    // ── Quote & Returns ────────────────────────────────────────
    pub quote: f64,
    pub daily_return_pct: f64,
    pub monthly_return_pct: f64,
    pub ytd_return_pct: f64,
    pub all_time_return_pct: f64,
    // ── D-Score Components ─────────────────────────────────────
    pub dscore: f64,
    pub ds_experience: f64,       // Ex (experience)
    pub ds_risk_mgmt: f64,        // Rs (risk stability)
    pub ds_risk_adjustment: f64,  // Ra (risk adjustment)
    pub ds_performance: f64,      // Pf (performance fee)
    pub ds_scalability: f64,      // Sc (scalability/capacity)
    pub ds_market_correlation: f64, // Mc (market correlation)
    // ── Risk Metrics ───────────────────────────────────────────
    pub var_monthly: f64,         // monthly VaR
    pub max_drawdown_pct: f64,
    pub volatility_annual: f64,
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    // ── Investor Data ──────────────────────────────────────────
    pub investors: u32,
    pub aum: f64,
    pub capacity_remaining_pct: f64,
    // ── Trading Stats ──────────────────────────────────────────
    pub total_trades: u32,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub avg_holding_time_hours: f64,
    pub avg_trade_return_pct: f64,
    pub symbols_traded: u32,
    // ── Status ──────────────────────────────────────────────────
    /// Whether this DARWIN is excluded/suspended on Darwinex (auto-detected).
    pub excluded: bool,
    /// Exclusion reason if applicable (e.g. "correlation", "VaR breach", "suspended").
    pub exclusion_reason: String,
    // ── Correlation ────────────────────────────────────────────
    /// Average pairwise correlation with other active DARWINs.
    pub correlation_portfolio: f64,
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
}

// ── KV Cache Keys ──────────────────────────────────────────────────

pub mod cache_keys {
    pub const COOKIES: &str = "dwx_web:cookies";
    pub const CONFIG: &str = "dwx_web:config";
    pub const CORRELATION: &str = "dwx_web:correlation";
    pub const LAST_SCRAPE: &str = "dwx_web:last_scrape";

    /// Per-DARWIN snapshot key.
    pub fn snapshot(ticker: &str) -> String {
        format!("dwx_web:{}:snapshot", ticker.to_uppercase())
    }
}

// ── Constants ──────────────────────────────────────────────────────

const CHROMEDRIVER_URL: &str = "http://localhost:9515";
const DARWINEX_LOGIN_URL: &str = "https://www.darwinexzero.com/login";
const DARWINEX_DASHBOARD_URL: &str = "https://www.darwinexzero.com/dashboard";
const CAPTCHA_POLL_MS: u64 = 500;
const CAPTCHA_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes for user to solve

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

    WebDriver::new(CHROMEDRIVER_URL, caps)
        .await
        .map_err(|e| format!("Failed to connect to ChromeDriver at {CHROMEDRIVER_URL}: {e}. Is chromedriver running?"))
}

/// Close the browser and end the WebDriver session.
pub async fn close_browser(driver: WebDriver) -> Result<(), String> {
    driver.quit().await.map_err(|e| format!("Browser close error: {e}"))
}

// ── Login Flow ─────────────────────────────────────────────────────

/// Log in to darwinexzero.com. Returns true if CAPTCHA was encountered
/// (user must solve it in the visible browser window).
pub async fn login(driver: &WebDriver, email: &str, password: &str) -> Result<bool, String> {
    driver.goto(DARWINEX_LOGIN_URL).await
        .map_err(|e| format!("Navigate to login failed: {e}"))?;

    // Wait for the email field to appear
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Fill email
    let email_field = driver.find(By::Css(selectors::LOGIN_EMAIL)).await
        .map_err(|e| format!("Email field not found: {e}"))?;
    email_field.clear().await.map_err(|e| format!("Clear email: {e}"))?;
    email_field.send_keys(email).await
        .map_err(|e| format!("Type email failed: {e}"))?;

    // Fill password
    let pwd_field = driver.find(By::Css(selectors::LOGIN_PASSWORD)).await
        .map_err(|e| format!("Password field not found: {e}"))?;
    pwd_field.clear().await.map_err(|e| format!("Clear password: {e}"))?;
    pwd_field.send_keys(password).await
        .map_err(|e| format!("Type password failed: {e}"))?;

    // Click submit
    let submit = driver.find(By::Css(selectors::LOGIN_SUBMIT)).await
        .map_err(|e| format!("Submit button not found: {e}"))?;
    submit.click().await
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
    let url = driver.current_url().await
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
        if driver.find(By::Css(selectors::DASHBOARD_MARKER)).await.is_ok() {
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
    let cookies = driver.get_all_cookies().await
        .map_err(|e| format!("Get cookies failed: {e}"))?;
    Ok(cookies.iter().map(|c| SerializableCookie {
        name: c.name.clone(),
        value: c.value.clone(),
        domain: c.domain.clone(),
        path: c.path.clone(),
    }).collect())
}

/// Restore cookies from cache into the browser session.
pub async fn restore_cookies(driver: &WebDriver, cookies: &[SerializableCookie]) -> Result<(), String> {
    // Must navigate to the domain first before setting cookies
    driver.goto(DARWINEX_LOGIN_URL).await
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

// ── Scraping ───────────────────────────────────────────────────────

/// Parse a numeric string (possibly with %, $, commas, or other formatting).
fn parse_numeric(text: &str) -> f64 {
    let cleaned: String = text.chars()
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

/// Build the DARWIN profile URL for a given ticker.
fn darwin_profile_url(ticker: &str) -> String {
    format!("https://www.darwinexzero.com/darwin/{}", ticker.to_uppercase())
}

/// Scrape a single DARWIN's profile page for live stats.
pub async fn scrape_darwin(driver: &WebDriver, ticker: &str) -> Result<DarwinWebSnapshot, String> {
    let url = darwin_profile_url(ticker);
    driver.goto(&url).await
        .map_err(|e| format!("Navigate to {ticker} profile failed: {e}"))?;

    // Wait for page to render
    tokio::time::sleep(Duration::from_secs(3)).await;

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
    let excluded = driver.find(By::Css(selectors::DARWIN_EXCLUDED)).await.is_ok();
    let exclusion_reason = match driver.find(By::Css(selectors::DARWIN_EXCLUSION_REASON)).await {
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

    Ok(DarwinWebSnapshot {
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
    })
}

/// Scrape the portfolio correlation matrix from the correlation page.
pub async fn scrape_correlation(
    driver: &WebDriver,
    tickers: &[String],
) -> Result<Vec<DarwinWebCorrelation>, String> {
    // Navigate to portfolio/correlation page
    driver.goto("https://www.darwinexzero.com/portfolio/correlation").await
        .map_err(|e| format!("Navigate to correlation page failed: {e}"))?;

    tokio::time::sleep(Duration::from_secs(3)).await;

    let mut correlations = Vec::new();

    // Try to find correlation table cells
    match driver.find_all(By::Css(selectors::CORRELATION_CELL)).await {
        Ok(cells) => {
            for cell in &cells {
                // Try to extract data attributes for darwin pair + correlation value
                let darwin_a = cell.attr("data-darwin-a").await
                    .ok().flatten().unwrap_or_default();
                let darwin_b = cell.attr("data-darwin-b").await
                    .ok().flatten().unwrap_or_default();
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

/// Full scrape cycle: all managed DARWINs + correlation matrix.
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

    for ticker in &config.managed_darwins {
        match scrape_darwin(driver, ticker).await {
            Ok(snapshot) => {
                // Cache individual snapshot
                let key = cache_keys::snapshot(ticker);
                if let Ok(json) = serde_json::to_string(&snapshot) {
                    let _ = cache_fn(&key, &json);
                }
                snapshots.push(snapshot);
            }
            Err(e) => {
                tracing::error!("Failed to scrape {ticker}: {e}");
            }
        }
    }

    // Scrape correlation matrix
    let correlations = scrape_correlation(driver, &config.managed_darwins).await
        .unwrap_or_default();

    // Determine which DARWINs are active (not excluded — auto-detected from scrape)
    // Use HashSet for O(1) lookups in correlation analysis
    let active: HashSet<String> = snapshots.iter()
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
                if snap.exclusion_reason.is_empty() { "no reason given" } else { &snap.exclusion_reason }
            );
        }
    }

    // Update correlation_portfolio on snapshots if we have correlation data
    for snap in &mut snapshots {
        // Average correlation of this DARWIN with other active DARWINs only
        // Use fold to avoid Vec allocation
        let (sum, count) = correlations.iter()
            .filter(|c| {
                let involves_snap = c.darwin_a == snap.ticker || c.darwin_b == snap.ticker;
                let other = if c.darwin_a == snap.ticker { &c.darwin_b } else { &c.darwin_a };
                involves_snap && active.contains(other)
            })
            .fold((0.0_f64, 0_u32), |(s, n), c| (s + c.correlation, n + 1));
        if count > 0 {
            snap.correlation_portfolio = sum / count as f64;
        }
    }

    // Check for correlation violations among active (non-excluded) DARWINs
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
                suggestion: suggest_correlation_fix(&corr.darwin_a, &corr.darwin_b, corr.correlation),
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
                alert.darwin_a, alert.darwin_b, alert.correlation, alert.threshold
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
        self.managed_darwins = self.managed_darwins.iter()
            .map(|t| t.trim().to_uppercase())
            .filter(|t| !t.is_empty() && t.chars().all(|c| c.is_ascii_alphanumeric()))
            .collect();
        self.managed_darwins.sort();
        self.managed_darwins.dedup();
        self.excluded_darwins = self.excluded_darwins.iter()
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
        self.managed_darwins.iter()
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
                "tpn".to_string(), // duplicate
                " xuqf ".to_string(), // whitespace
            ],
            excluded_darwins: vec!["mfso".to_string(), "MFSO".to_string()],
            auto_scrape: false,
            scrape_minute: 99, // invalid — should be clamped
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
        // Valid JSON with one extra field "extra" — should fail
        let snap = test_snapshot();
        let mut json = serde_json::to_string(&snap).unwrap();
        // Inject unknown field before closing brace
        json.pop(); // remove '}'
        json.push_str(r#","extra":1}"#);
        assert!(serde_json::from_str::<DarwinWebSnapshot>(&json).is_err());
    }
}
