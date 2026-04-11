# ADR-093 — Darwinex Zero Live Web Scraping via Selenium

**Status:** Accepted
**Date:** 2026-04-10

## Context

The Darwinex FTP feed provides comprehensive data for 50K+ DARWINs but has
inherent lag — files update daily, and the NAS mirror sync adds further
delay. For actively managed DARWINs (~6 accounts), live stats from
darwinexzero.com would provide near-real-time correlation, NAV quotes,
D-Score, investor flow, and VaR data — all critical for portfolio decisions.

The web dashboard updates approximately 15–20 minutes past each hour.
Scraping at :20 past catches fresh data without hitting stale numbers.

CAPTCHAs on the login page require human interaction, so the browser must
be visible (not headless) during login. Once authenticated, subsequent
page loads within the session are CAPTCHA-free.

## Decisions

### 1. Browser Automation: `thirtyfour` (Rust WebDriver)

Use `thirtyfour` — the most mature async WebDriver client for Rust.
Communicates with ChromeDriver via the W3C WebDriver protocol.

```
thirtyfour ──WebDriver──→ chromedriver ──→ Chrome/Chromium (visible)
```

ChromeDriver must be installed on the system (one-time setup). The browser
window is visible (not headless) so the user can solve CAPTCHAs manually.

### 2. New Module: `engine/src/core/darwin_web.rs`

Data types:

| Type | Fields | Purpose |
|------|--------|---------|
| `DarwinWebSnapshot` | `ticker, timestamp_ms, quote, daily/monthly/ytd/all_time_return_pct, dscore + 6 D-Score components, var_monthly, max_drawdown_pct, volatility, sharpe, sortino, investors, aum, capacity, total_trades, win_rate, profit_factor, avg_hold_time, avg_trade_return, symbols_traded, excluded, exclusion_reason, correlation_portfolio` | Full strategy analysis tab per DARWIN |
| `DarwinWebCorrelation` | `darwin_a, darwin_b, correlation` | Live pairwise correlation |
| `CorrelationAlert` | `darwin_a, darwin_b, correlation, threshold, suggestion` | Violation with auto-fix suggestion |
| `DarwinWebConfig` | `managed_darwins, excluded_darwins, auto_scrape, scrape_minute, correlation_alert_threshold` | User configuration |
| `DarwinWebUpdate` | `snapshots, correlations, correlation_alerts, timestamp_ms` | Full scrape cycle result |

Functions:

| Function | Description |
|----------|-------------|
| `launch_browser()` | Start ChromeDriver + Chrome, return WebDriver handle |
| `login(driver, email, password)` | Navigate to login page, fill credentials, submit, detect CAPTCHA |
| `check_captcha(driver)` | Return true if CAPTCHA element present on page |
| `wait_for_captcha_solve(driver, timeout)` | Poll every 500ms for dashboard element, up to timeout |
| `is_session_valid(driver)` | Navigate to known page, check for login redirect |
| `restore_cookies(driver, cookies)` | Load cookies from cache to skip login |
| `scrape_darwin(driver, ticker)` | Navigate to DARWIN profile, extract DarwinWebSnapshot |
| `scrape_correlation(driver, tickers)` | Navigate to portfolio page, extract N×N correlation |
| `scrape_all(driver, config, cache)` | Full scrape cycle for all managed DARWINs |

### 3. Auto-Exclusion Detection

DARWIN exclusion/suspension status is **auto-detected** from the profile
page during scraping. No manual configuration needed — if darwinexzero.com
shows a DARWIN as excluded, the `excluded` flag is set automatically with
the reason captured in `exclusion_reason`.

Excluded DARWINs are:
- Logged as warnings in the console
- Removed from correlation analysis (won't trigger false correlation alerts)
- Shown with exclusion badge in the UI

### 4. Correlation Breach Alerts + Auto-Fix Suggestions

When pairwise correlation between active (non-excluded) DARWINs exceeds
the threshold (default 0.95, Darwinex 45-day standard), the system:

1. Logs a `CORRELATION BREACH` error with the pair and coefficient
2. Generates an auto-fix suggestion based on severity:
   - **CRITICAL (>0.98):** "Consider closing one or switching to a
     different symbol/strategy"
   - **HIGH (>0.95):** "Reduce overlapping symbol exposure — shift
     primary symbol to uncorrelated asset"
   - **WARNING (<0.95):** "Monitor — may resolve naturally as positions
     rotate"

### 5. Credential Storage (single login)

Darwinex Zero email and password stored in system keyring (consistent with
all other credentials in the terminal):

| Key | Content |
|-----|---------|
| `darwinex_email` | Darwinex Zero login email |
| `darwinex_password` | Darwinex Zero login password |

Set via `DWXSETCREDS` command. Never logged, never written to files.

### 6. Cookie Persistence

Session cookies cached in SQLite KV store to avoid re-login every hour:

| KV Key | Value |
|--------|-------|
| `dwx_web:cookies` | JSON-serialized Vec of cookie name/value/domain |
| `dwx_web:config` | JSON DarwinWebConfig |
| `dwx_web:{ticker}:snapshot` | JSON DarwinWebSnapshot |
| `dwx_web:correlation` | JSON Vec\<DarwinWebCorrelation\> |
| `dwx_web:last_scrape` | ISO 8601 timestamp |

### 7. Console Commands

| Command | Aliases | Description |
|---------|---------|-------------|
| `DWXLOGIN` | — | Launch Chrome, login to darwinexzero.com (CAPTCHA if needed) |
| `DWXSYNC` | — | Manual scrape of all managed DARWINs now |
| `DWXAUTO` | — | Toggle automatic hourly scraping at :20 past |
| `DWXSTATUS` | — | Show session status, last scrape, next scheduled |
| `DWXLOGOUT` | — | Close browser, clear session cookies |
| `DWXSETCREDS` | — | Prompt for Darwinex email/password, store in keyring |
| `DWXDARWINS` | — | Set list of managed DARWIN tickers to scrape |

### 8. Scrape Cycle

```
1. Check session validity (navigate to dashboard, detect login redirect)
2. If expired → re-login (may prompt CAPTCHA)
3. For each managed DARWIN ticker:
   a. Navigate to DARWIN profile page
   b. CSS-select: quote, D-Score, VaR, investors, AUM, returns
   c. Build DarwinWebSnapshot
4. Navigate to portfolio correlation page
   a. Extract N×N correlation matrix
5. Cache all data via put_kv()
6. Broadcast DarwinWebUpdate to native UI + web clients
```

### 9. Hourly Timer

When `auto_scrape` is enabled, the native app's background thread checks
every 60 seconds whether the current time is past :20 of the hour and
no scrape has occurred this hour. If so, it triggers the scrape cycle.

```rust
// In background thread loop
if dwx_auto_scrape && now.minute() >= 20 && last_dwx_hour != now.hour() {
    last_dwx_hour = now.hour();
    // trigger scrape via channel
}
```

### 10. Integration with Existing Views

| Scraped Data | Existing Structure | Integration |
|--------------|-------------------|-------------|
| `quote` | Live NAV in DARWIN view header | Direct display |
| `var_monthly` | `DarwinVaRMultiplier` corridor check | Feed into DARWINVAR |
| `correlation` | `CorrelationEntry` for portfolio view | Overlay on GPU correlation |
| `investors` / `aum` | `InvestorFlow` timeline | Append to flow chart |
| `dscore` | `DScoreEstimate` | Compare FTP vs live |
| `capacity_remaining_pct` | New field in DARWIN view | Approaching-limit warning |

### 11. Web Protocol Extension

New WebCmd/WebMsg variants for phone clients:

| WebCmd | Fields | Purpose |
|--------|--------|---------|
| `GetDarwinWeb` | `ticker: Option<String>` | Request live DARWIN web snapshots |

| WebMsg | Fields | Purpose |
|--------|--------|---------|
| `DarwinWebUpdate` | `snapshots: Vec<DarwinWebSnapshot>, correlations: Vec<DarwinWebCorrelation>` | Push live DARWIN data to web clients |

### 12. CSS Selector Strategy

Keep all CSS selectors in a `const` block at the top of `darwin_web.rs`
for easy maintenance when darwinexzero.com changes their markup:

```rust
mod selectors {
    pub const LOGIN_EMAIL: &str = "input[name='email']";
    pub const LOGIN_PASSWORD: &str = "input[name='password']";
    pub const LOGIN_SUBMIT: &str = "button[type='submit']";
    pub const CAPTCHA_FRAME: &str = "iframe[src*='captcha'], .g-recaptcha, .h-captcha";
    pub const DASHBOARD_MARKER: &str = ".dashboard, [data-page='dashboard']";
    // Per-DARWIN profile selectors — will need updating as site evolves
    pub const DARWIN_QUOTE: &str = ".darwin-quote, [data-field='quote']";
    pub const DARWIN_DSCORE: &str = ".d-score, [data-field='dscore']";
    // ... etc
}
```

## Dependencies

```toml
# engine/Cargo.toml
thirtyfour = "0.35"  # Async WebDriver client (Selenium protocol)
```

System requirement: ChromeDriver binary (`sudo pacman -S chromedriver` or
download from https://chromedriver.chromium.org).

## Tests

**Total workspace test count: 854** (up from 836 in ADR-092).

- 216 mql5-compiler (unchanged)
- 511 engine (+14: darwin_web snapshot roundtrip, correlation roundtrip,
  config roundtrip, config deny_unknown, cookie serialization, credential
  keys, config normalize/dedup, cache key format, parse_numeric, snapshot
  deny_unknown, active_darwins excludes, correlation alert with suggestion,
  correlation fix critical, correlation fix warning)
- 78 native (unchanged)
- 49 web-protocol (+4: GetDarwinWeb roundtrip, DarwinWebUpdate msg
  roundtrip, DarwinCorrelationAlert roundtrip, DarwinWebSnapshot
  deny_unknown)

## Post-Implementation Audit (2026-04-10)

### Unwraps

| Crate | Production unwrap() | Production expect() | Notes |
|---|---|---|---|
| mql5-compiler/parser.rs | 0 | 0 | Rewritten: `next_or_err()` returns `CompileError::Internal` |
| mql5-compiler/ir.rs | 0 | 0 | Rewritten: `.ok_or_else()` returns `CompileError::Internal` |
| engine (all) | 0 | 0 | Clean |
| native (all) | 0 | 0 | Clean |
| web-server | 0 | 0 | Clean |
| web-protocol | 0 | 0 | Clean |
| cli | 0 | 0 | Clean |

**Total production unwrap/expect across entire codebase: 0.**

### Security

- 0 credential logging (email/password never appear in log messages)
- 0 command injection (no subprocess/shell execution)
- 0 path traversal (tickers validated: alphanumeric only via normalize())
- `deny_unknown_fields` on all serde types (DarwinWebSnapshot, Config, etc.)
- Session cookies stored in SQLite KV cache (same security model as bar data)
- Constant-time passphrase comparison in web-server auth (`subtle::ConstantTimeEq`)
- Mutex locks dropped before I/O in lan_sync.rs (2 instances fixed)
- Integer overflow: `checked_mul` for GPU Darwin batch buffer calculations
- Write lock released before file I/O + zstd compression in cache.rs `export_backup`
- Decompression failures logged (not silently defaulted) in `repair_bar_counts`
- Client-side table whitelist validation in LAN sync (defense in depth against
  compromised server sending non-whitelisted table names into format! SQL)

### Performance

- HashSet for active DARWIN lookup (O(1) per check vs O(n) Vec scan)
- Iterator fold for correlation averaging (no Vec allocation per snapshot)
- WebDriver handle stored as Arc<Mutex> — single browser instance reused
- Background thread for scraping — UI thread never blocked

### Test Count

854 total (216 mql5-compiler + 511 engine + 78 native + 49 web-protocol)

## Consequences

### Positive

- Live DARWIN stats within ~20 minutes of the hour vs. 24h+ FTP lag
- Correlation data enables real-time portfolio rebalancing decisions
- D-Score tracking detects investability changes before FTP updates
- Investor flow signals (AUM spikes/drops) visible in near-real-time
- Cookie persistence minimizes CAPTCHA prompts (typically once per session)

### Trade-offs

- ChromeDriver dependency adds a system requirement
- CAPTCHA requires human interaction — cannot be fully automated
- CSS selectors will break when darwinexzero.com redesigns (mitigated by
  centralizing selectors in one const block)
- Chrome process consumes ~200-400MB RAM while running
- Rate limiting is self-imposed (one page per DARWIN per hour) — if
  Darwinex adds stricter rate limits, scrape interval may need increasing

### Risks

- Darwinex may block automated access (unlikely at 6 DARWINs/hour rate)
- Session cookies may expire unpredictably — auto-detect and re-prompt
- Chrome updates may break ChromeDriver compatibility (keep versions matched)

## Related

- ADR-041 — DARWIN Import Pipeline & Analytics (foundation)
- ADR-055 — GPU-Accelerated DARWIN Analytics (correlation/VaR compute)
- ADR-076 — DarwinexRadar symbol spec tracking
- ADR-092 — UX Improvements, GPU Compute, Client Parity
