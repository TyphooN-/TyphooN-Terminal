//! Regulatory outlier/alert ingest.
//!
//! Initial source: NasdaqTrader Regulation SHO threshold list. This is a public
//! daily TXT download, not a paid/API-key feed. We cache the latest snapshot in
//! SQLite so the UI can show warning badges without per-frame or startup network
//! dependency.

use chrono::{Datelike, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const NASDAQ_REGSHO_PAGE: &str = "https://nasdaqtrader.com/trader.aspx?id=RegSHOThreshold";
const NASDAQ_REGSHO_BASE: &str = "https://www.nasdaqtrader.com/dynamic/symdir/regsho";
// Public, no-key RSS feed of current US trading halts / LULD volatility pauses.
const NASDAQ_HALTS_RSS: &str = "https://www.nasdaqtrader.com/rss.aspx?feed=tradehalts";
const USER_AGENT: &str = "TyphooN-Terminal/1.0 regulatory-alerts";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegulatoryAlert {
    pub symbol: String,
    pub kind: String,
    pub label: String,
    pub source: String,
    pub as_of: String,
    pub details: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegShoEntry {
    pub symbol: String,
    pub security_name: String,
    pub market_category: String,
    pub reg_sho_threshold_flag: String,
    pub rule_3210: String,
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

pub fn create_regulatory_alert_tables(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS regulatory_alerts (
            symbol TEXT NOT NULL,
            kind TEXT NOT NULL,
            label TEXT NOT NULL,
            source TEXT NOT NULL,
            as_of TEXT NOT NULL,
            details TEXT NOT NULL DEFAULT '',
            updated_at INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY(symbol, kind, source)
        );
        CREATE INDEX IF NOT EXISTS idx_regulatory_alerts_symbol
            ON regulatory_alerts(symbol);
        CREATE INDEX IF NOT EXISTS idx_regulatory_alerts_kind
            ON regulatory_alerts(kind, as_of);
        ",
    )
    .map_err(|e| format!("create regulatory alert tables: {e}"))
}

pub fn get_regulatory_alerts(conn: &Connection) -> Result<Vec<RegulatoryAlert>, String> {
    let _ = create_regulatory_alert_tables(conn);
    let mut stmt = conn
        .prepare(
            "SELECT symbol, kind, label, source, as_of, details, updated_at
             FROM regulatory_alerts
             ORDER BY symbol, kind, source",
        )
        .map_err(|e| format!("prepare regulatory alerts: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(RegulatoryAlert {
                symbol: row.get(0)?,
                kind: row.get(1)?,
                label: row.get(2)?,
                source: row.get(3)?,
                as_of: row.get(4)?,
                details: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(|e| format!("query regulatory alerts: {e}"))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn regulatory_alert_map(alerts: &[RegulatoryAlert]) -> HashMap<String, Vec<RegulatoryAlert>> {
    let mut out: HashMap<String, Vec<RegulatoryAlert>> = HashMap::new();
    for alert in alerts {
        out.entry(alert.symbol.to_ascii_uppercase())
            .or_default()
            .push(alert.clone());
    }
    out
}

pub fn replace_regsho_threshold_alerts(
    conn: &Connection,
    as_of: &str,
    rows: &[RegShoEntry],
) -> Result<usize, String> {
    create_regulatory_alert_tables(conn)?;
    let now = now_secs();
    conn.execute(
        "DELETE FROM regulatory_alerts WHERE kind = 'reg_sho_threshold' AND source = 'nasdaqtrader'",
        [],
    )
    .map_err(|e| format!("clear reg sho alerts: {e}"))?;
    let mut inserted = 0usize;
    for row in rows {
        if row.symbol.trim().is_empty() || !row.reg_sho_threshold_flag.eq_ignore_ascii_case("Y") {
            continue;
        }
        let symbol = normalize_regulatory_symbol(&row.symbol);
        if symbol.is_empty() {
            continue;
        }
        let details = format!(
            "{} · Nasdaq market {} · Rule 3210 {}",
            row.security_name.trim(),
            row.market_category.trim(),
            row.rule_3210.trim()
        );
        conn.execute(
            "INSERT OR REPLACE INTO regulatory_alerts
             (symbol, kind, label, source, as_of, details, updated_at)
             VALUES (?1, 'reg_sho_threshold', '!! Reg SHO !!', 'nasdaqtrader', ?2, ?3, ?4)",
            params![symbol, as_of, details, now],
        )
        .map_err(|e| format!("insert reg sho alert: {e}"))?;
        inserted += 1;
    }
    Ok(inserted)
}

pub fn normalize_regulatory_symbol(symbol: &str) -> String {
    symbol
        .trim()
        .trim_end_matches(".EQ")
        .replace('/', "")
        .to_ascii_uppercase()
}

pub fn parse_regsho_threshold_txt(text: &str) -> Vec<RegShoEntry> {
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let line = line.trim_end_matches('\r').trim();
            if line.is_empty() || line.starts_with("File Creation Time") {
                return None;
            }
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() < 5 {
                return None;
            }
            let symbol = parts[0].trim();
            if symbol.is_empty() || symbol.eq_ignore_ascii_case("Symbol") {
                return None;
            }
            Some(RegShoEntry {
                symbol: symbol.to_string(),
                security_name: parts[1].trim().to_string(),
                market_category: parts[2].trim().to_string(),
                reg_sho_threshold_flag: parts[3].trim().to_string(),
                rule_3210: parts[4].trim().to_string(),
            })
        })
        .collect()
}

fn extract_regsho_download_from_page(page: &str) -> Option<(String, String)> {
    let needle = "nasdaqth";
    let start = page.find(needle)?;
    let rest = &page[start..];
    let txt_end = rest.find(".txt")? + 4;
    let file = &rest[..txt_end];
    if file.len() < "nasdaqthYYYYMMDD.txt".len() {
        return None;
    }
    let digits: String = file.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 8 {
        return None;
    }
    let ymd = &digits[0..8];
    let as_of = format!("{}-{}-{}", &ymd[0..4], &ymd[4..6], &ymd[6..8]);
    Some((format!("{NASDAQ_REGSHO_BASE}/{file}"), as_of))
}

async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String, String> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("GET {url}: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("GET {url}: HTTP {status}"));
    }
    resp.text().await.map_err(|e| format!("read {url}: {e}"))
}

pub async fn fetch_regsho_threshold_entries() -> Result<(String, Vec<RegShoEntry>), String> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("build reg sho client: {e}"))?;

    if let Ok(page) = fetch_text(&client, NASDAQ_REGSHO_PAGE).await {
        if let Some((url, as_of)) = extract_regsho_download_from_page(&page) {
            let txt = fetch_text(&client, &url).await?;
            let rows = parse_regsho_threshold_txt(&txt);
            if !rows.is_empty() {
                return Ok((as_of, rows));
            }
        }
    }

    // Fallback: try recent calendar days. Nasdaq publishes after market close;
    // weekends/holidays mean "today" often has no file yet.
    let today = Utc::now().date_naive();
    for days_back in 0..=10 {
        let Some(day) = today.checked_sub_days(chrono::Days::new(days_back)) else {
            continue;
        };
        let ymd = format!("{:04}{:02}{:02}", day.year(), day.month(), day.day());
        let url = format!("{NASDAQ_REGSHO_BASE}/nasdaqth{ymd}.txt");
        if let Ok(txt) = fetch_text(&client, &url).await {
            let rows = parse_regsho_threshold_txt(&txt);
            if !rows.is_empty() {
                return Ok((
                    format!("{:04}-{:02}-{:02}", day.year(), day.month(), day.day()),
                    rows,
                ));
            }
        }
    }

    Err("Reg SHO threshold list unavailable from NasdaqTrader".to_string())
}

pub fn get_latest_regsho_as_of(conn: &Connection) -> Result<Option<String>, String> {
    create_regulatory_alert_tables(conn)?;
    let mut stmt = conn
        .prepare(
            "SELECT as_of FROM regulatory_alerts
             WHERE kind = 'reg_sho_threshold' AND source = 'nasdaqtrader'
             ORDER BY as_of DESC LIMIT 1",
        )
        .map_err(|e| format!("prepare latest regsho as_of: {e}"))?;
    let mut rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query latest regsho as_of: {e}"))?;
    Ok(rows.next().and_then(|r| r.ok()))
}

/// Smart refresh: only downloads the Nasdaq Reg SHO TXT when the remote
/// file's as_of date is newer than what we already have cached.
/// This avoids hammering the public endpoint every 30 minutes when the
/// list has not changed.
pub async fn refresh_regsho_threshold_alerts(conn: &Connection) -> Result<usize, String> {
    let cached_as_of = get_latest_regsho_as_of(conn).ok().flatten();

    let (remote_as_of, rows) = fetch_regsho_threshold_entries().await?;

    if let Some(cached) = cached_as_of {
        if cached == remote_as_of {
            // No new file uploaded yet — reuse cache
            return Ok(0);
        }
    }

    replace_regsho_threshold_alerts(conn, &remote_as_of, &rows)
}

// ── Trading halts / LULD volatility pauses ───────────────────────────────────
//
// Second free regulatory source (ADR-120 future extension): NasdaqTrader's
// public trade-halts RSS feed. No API key. Halts are transient, so unlike the
// daily Reg SHO list we re-fetch on a tight cadence and fully replace the cached
// `trade_halt` rows each time; resumed halts (a resumption trade time is
// published) are dropped so only currently-halted symbols carry a badge.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeHaltEntry {
    pub symbol: String,
    pub name: String,
    pub market: String,
    pub reason_code: String,
    pub halt_date: String,
    pub halt_time: String,
    pub resumption_date: String,
    pub resumption_trade_time: String,
}

impl TradeHaltEntry {
    /// A halt is still active for badge purposes until a resumption trade time
    /// is published. Resumed halts are dropped from the alert layer.
    pub fn is_active(&self) -> bool {
        self.resumption_trade_time.trim().is_empty()
    }
}

/// Human-readable description for a Nasdaq/UTP halt reason code.
pub fn halt_reason_description(code: &str) -> &'static str {
    match code.trim().to_ascii_uppercase().as_str() {
        "LUDP" => "Volatility trading pause (LULD)",
        "LUDS" => "Volatility pause, straddle state (LULD)",
        "T1" => "News pending",
        "T2" => "News released",
        "T3" => "News and resumption times",
        "T5" => "Single-stock trading pause (10% move)",
        "T6" => "Extraordinary market activity",
        "T7" => "Single-stock pause, quotation only",
        "T8" => "Exchange-traded product halt",
        "T12" => "Additional information requested",
        "H4" => "Non-compliance with listing requirements",
        "H9" => "Not current in regulatory filings",
        "H10" => "SEC trading suspension",
        "H11" => "Regulatory concern",
        "IPO1" | "IPOQ" | "IPOE" => "IPO not yet trading",
        "M1" | "M2" | "MWC0" | "MWC1" | "MWC2" | "MWC3" | "MWCQ" => "Market-wide circuit breaker",
        "D" => "Security deletion / delisting",
        _ => "Trading halt",
    }
}

fn decode_basic_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

/// Extract `<...local>VALUE</...local>` from one RSS `<item>`, tolerant of the
/// `ndaq:` namespace prefix. The first `local>` is the opening tag's end; the
/// value runs to the next `<`.
fn rss_local_field(item: &str, local: &str) -> String {
    let needle = format!("{local}>");
    let Some(start) = item.find(&needle) else {
        return String::new();
    };
    let after = &item[start + needle.len()..];
    let value = after.split('<').next().unwrap_or("");
    decode_basic_entities(value).trim().to_string()
}

pub fn parse_trade_halts_rss(text: &str) -> Vec<TradeHaltEntry> {
    let mut out = Vec::new();
    for chunk in text.split("<item>").skip(1) {
        let item = chunk.split("</item>").next().unwrap_or(chunk);
        // Symbol: prefer the precise IssueSymbol element, fall back to Symbol.
        let mut symbol = rss_local_field(item, "IssueSymbol");
        if symbol.is_empty() {
            symbol = rss_local_field(item, "Symbol");
        }
        let symbol = symbol.to_ascii_uppercase();
        if symbol.is_empty() {
            continue;
        }
        let mut name = rss_local_field(item, "IssueName");
        if name.is_empty() {
            name = rss_local_field(item, "CompanyName");
        }
        out.push(TradeHaltEntry {
            symbol,
            name,
            market: rss_local_field(item, "Market"),
            reason_code: rss_local_field(item, "ReasonCode"),
            halt_date: rss_local_field(item, "HaltDate"),
            halt_time: rss_local_field(item, "HaltTime"),
            resumption_date: rss_local_field(item, "ResumptionDate"),
            resumption_trade_time: rss_local_field(item, "ResumptionTradeTime"),
        });
    }
    out
}

pub fn replace_trade_halt_alerts(
    conn: &Connection,
    rows: &[TradeHaltEntry],
) -> Result<usize, String> {
    create_regulatory_alert_tables(conn)?;
    let now = now_secs();
    conn.execute(
        "DELETE FROM regulatory_alerts WHERE kind = 'trade_halt' AND source = 'nasdaqtrader'",
        [],
    )
    .map_err(|e| format!("clear trade halt alerts: {e}"))?;
    let mut inserted = 0usize;
    for row in rows {
        if !row.is_active() {
            continue;
        }
        let symbol = normalize_regulatory_symbol(&row.symbol);
        if symbol.is_empty() {
            continue;
        }
        let mut details = format!(
            "{} ({})",
            halt_reason_description(&row.reason_code),
            row.reason_code.trim()
        );
        if !row.halt_time.trim().is_empty() {
            details.push_str(&format!(" · Halted {}", row.halt_time.trim()));
        }
        if !row.market.trim().is_empty() {
            details.push_str(&format!(" · {}", row.market.trim()));
        }
        if !row.name.trim().is_empty() {
            details.push_str(&format!(" · {}", row.name.trim()));
        }
        conn.execute(
            "INSERT OR REPLACE INTO regulatory_alerts
             (symbol, kind, label, source, as_of, details, updated_at)
             VALUES (?1, 'trade_halt', '!! HALT !!', 'nasdaqtrader', ?2, ?3, ?4)",
            params![symbol, row.halt_date.trim(), details, now],
        )
        .map_err(|e| format!("insert trade halt alert: {e}"))?;
        inserted += 1;
    }
    Ok(inserted)
}

pub async fn fetch_trade_halt_entries() -> Result<Vec<TradeHaltEntry>, String> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("build trade halt client: {e}"))?;
    let xml = fetch_text(&client, NASDAQ_HALTS_RSS).await?;
    Ok(parse_trade_halts_rss(&xml))
}

/// Re-fetch current halts and fully replace the cached `trade_halt` rows. Halts
/// are transient, so there is no smart-skip — each call reflects the live feed.
pub async fn refresh_trade_halt_alerts(conn: &Connection) -> Result<usize, String> {
    let rows = fetch_trade_halt_entries().await?;
    replace_trade_halt_alerts(conn, &rows)
}

// ── SSR (Short Sale Restriction, SEC Rule 201) — computed, no feed ─────────
//
// SSR triggers when a covered US equity trades ≥10% below the prior day's
// close intraday, and stays in effect for the remainder of that day plus the
// next trading day. No feed is needed: the terminal already holds last +
// prior close for watched symbols, so this is a computed state machine
// (ADR-120 "next free extension"). Alerts use kind='ssr', source='computed';
// `as_of` is the ET trigger date, and expiry derives from the trading
// calendar (`market_session::is_us_market_trading_day`), so weekends and
// holidays extend the restriction exactly like the real rule.

/// SEC Rule 201 trigger ratio: at or below 90% of the prior close.
pub const SSR_TRIGGER_RATIO: f64 = 0.90;

/// True when an intraday price satisfies the Rule 201 trigger versus the
/// prior close (≥10% decline). Non-positive inputs never trigger.
pub fn ssr_triggered(last: f64, prev_close: f64) -> bool {
    last > 0.0
        && prev_close > 0.0
        && last.is_finite()
        && prev_close.is_finite()
        && last <= prev_close * SSR_TRIGGER_RATIO
}

/// Last ET calendar date the restriction covers for a trigger on
/// `triggered_date` (YYYY-MM-DD): the next US trading day after the trigger.
pub fn ssr_active_through(triggered_date: &str) -> Option<chrono::NaiveDate> {
    let date = chrono::NaiveDate::parse_from_str(triggered_date.trim(), "%Y-%m-%d").ok()?;
    let mut next = date;
    for _ in 0..14 {
        next = next.succ_opt()?;
        if crate::core::market_session::is_us_market_trading_day(next) {
            return Some(next);
        }
    }
    None
}

/// Record (or refresh) a computed SSR alert for `symbol`. Idempotent per
/// (symbol, kind, source); a re-trigger on a later date advances `as_of`.
pub fn upsert_ssr_alert(
    conn: &Connection,
    symbol: &str,
    triggered_date: &str,
    drop_pct: f64,
    prev_close: f64,
) -> Result<(), String> {
    create_regulatory_alert_tables(conn)?;
    let symbol = normalize_regulatory_symbol(symbol);
    if symbol.is_empty() {
        return Err("SSR alert: empty symbol".into());
    }
    let through = ssr_active_through(triggered_date)
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "next trading day".to_string());
    let details = format!(
        "Short Sale Restriction (Rule 201): {:.1}% below prior close ${:.2} on {} · active through {}",
        -drop_pct.abs(),
        prev_close,
        triggered_date.trim(),
        through
    );
    conn.execute(
        "INSERT OR REPLACE INTO regulatory_alerts
         (symbol, kind, label, source, as_of, details, updated_at)
         VALUES (?1, 'ssr', '! SSR !', 'computed', ?2, ?3, ?4)",
        params![symbol, triggered_date.trim(), details, now_secs()],
    )
    .map_err(|e| format!("insert ssr alert: {e}"))?;
    Ok(())
}

/// Delete computed SSR alerts whose restriction window has ended as of the
/// ET date `today` (YYYY-MM-DD). Returns how many were purged.
pub fn purge_expired_ssr_alerts(conn: &Connection, today: &str) -> Result<usize, String> {
    create_regulatory_alert_tables(conn)?;
    let Ok(today) = chrono::NaiveDate::parse_from_str(today.trim(), "%Y-%m-%d") else {
        return Err(format!("SSR purge: bad date '{today}'"));
    };
    let mut stmt = conn
        .prepare("SELECT symbol, as_of FROM regulatory_alerts WHERE kind = 'ssr' AND source = 'computed'")
        .map_err(|e| format!("prepare ssr purge: {e}"))?;
    let rows: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| format!("query ssr purge: {e}"))?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);
    let mut purged = 0usize;
    for (symbol, as_of) in rows {
        let expired = match ssr_active_through(&as_of) {
            Some(through) => today > through,
            // Unparseable as_of: keep one day, then drop via this arm.
            None => true,
        };
        if expired {
            conn.execute(
                "DELETE FROM regulatory_alerts WHERE symbol = ?1 AND kind = 'ssr' AND source = 'computed'",
                params![symbol],
            )
            .map_err(|e| format!("delete ssr alert: {e}"))?;
            purged += 1;
        }
    }
    Ok(purged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_regsho_threshold_txt_rows() {
        let txt = "Symbol|Security Name|Market Category|Reg SHO Threshold Flag|Rule 3210|Filler\r\nWOK|WORK MED TECHNOLOGY GROUP LTD|S|Y|N|\r\nZZZ|NOPE|Q|N|N|\r\nFile Creation Time: 6/12/2026 11:00:08 PM|||||";
        let rows = parse_regsho_threshold_txt(txt);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "WOK");
        assert_eq!(rows[0].reg_sho_threshold_flag, "Y");
    }

    #[test]
    fn ssr_trigger_fires_at_ten_percent_decline() {
        assert!(ssr_triggered(9.0, 10.0)); // exactly -10%
        assert!(ssr_triggered(8.5, 10.0));
        assert!(!ssr_triggered(9.01, 10.0));
        assert!(!ssr_triggered(0.0, 10.0));
        assert!(!ssr_triggered(9.0, 0.0));
        assert!(!ssr_triggered(f64::NAN, 10.0));
    }

    #[test]
    fn ssr_active_through_skips_weekends_and_holidays() {
        // Friday trigger → restriction runs through Monday.
        assert_eq!(
            ssr_active_through("2026-06-12").unwrap().to_string(),
            "2026-06-15"
        );
        // Trigger the Thursday before Good Friday 2026-04-03 → through Monday.
        assert_eq!(
            ssr_active_through("2026-04-02").unwrap().to_string(),
            "2026-04-06"
        );
        // Midweek trigger → through the next day.
        assert_eq!(
            ssr_active_through("2026-06-10").unwrap().to_string(),
            "2026-06-11"
        );
        assert!(ssr_active_through("garbage").is_none());
    }

    #[test]
    fn ssr_upsert_and_purge_lifecycle() {
        let conn = Connection::open_in_memory().unwrap();
        upsert_ssr_alert(&conn, "wok", "2026-06-12", -12.5, 4.20).unwrap();
        let alerts = get_regulatory_alerts(&conn).unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].symbol, "WOK");
        assert_eq!(alerts[0].kind, "ssr");
        assert!(alerts[0].details.contains("active through 2026-06-15"));

        // Still active on the trigger Friday, the weekend, and Monday.
        assert_eq!(purge_expired_ssr_alerts(&conn, "2026-06-12").unwrap(), 0);
        assert_eq!(purge_expired_ssr_alerts(&conn, "2026-06-14").unwrap(), 0);
        assert_eq!(purge_expired_ssr_alerts(&conn, "2026-06-15").unwrap(), 0);
        assert_eq!(get_regulatory_alerts(&conn).unwrap().len(), 1);

        // Expired Tuesday.
        assert_eq!(purge_expired_ssr_alerts(&conn, "2026-06-16").unwrap(), 1);
        assert!(get_regulatory_alerts(&conn).unwrap().is_empty());

        // Re-trigger after expiry re-inserts cleanly (idempotent upsert).
        upsert_ssr_alert(&conn, "WOK", "2026-06-16", -11.0, 3.10).unwrap();
        upsert_ssr_alert(&conn, "WOK", "2026-06-16", -14.0, 3.10).unwrap();
        assert_eq!(get_regulatory_alerts(&conn).unwrap().len(), 1);
    }

    #[test]
    fn replaces_regsho_alerts_in_sqlite() {
        let conn = Connection::open_in_memory().unwrap();
        let rows = vec![RegShoEntry {
            symbol: "WOK".into(),
            security_name: "WORK MED TECHNOLOGY GROUP LTD".into(),
            market_category: "S".into(),
            reg_sho_threshold_flag: "Y".into(),
            rule_3210: "N".into(),
        }];
        let inserted = replace_regsho_threshold_alerts(&conn, "2026-06-12", &rows).unwrap();
        assert_eq!(inserted, 1);
        let alerts = get_regulatory_alerts(&conn).unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].symbol, "WOK");
        assert_eq!(alerts[0].label, "!! Reg SHO !!");
    }

    const HALTS_RSS_SAMPLE: &str = r#"<rss xmlns:ndaq="http://www.nasdaqtrader.com/">
<channel>
<item>
<title>ABCD - Halt</title>
<ndaq:HaltDate>06/15/2026</ndaq:HaltDate>
<ndaq:HaltTime>09:35:00</ndaq:HaltTime>
<ndaq:IssueSymbol>ABCD</ndaq:IssueSymbol>
<ndaq:IssueName>Alpha Beta Corp &amp; Co</ndaq:IssueName>
<ndaq:Market>NASDAQ</ndaq:Market>
<ndaq:ReasonCode>LUDP</ndaq:ReasonCode>
<ndaq:ResumptionDate></ndaq:ResumptionDate>
<ndaq:ResumptionTradeTime></ndaq:ResumptionTradeTime>
</item>
<item>
<ndaq:HaltDate>06/15/2026</ndaq:HaltDate>
<ndaq:HaltTime>10:00:00</ndaq:HaltTime>
<ndaq:IssueSymbol>RESM</ndaq:IssueSymbol>
<ndaq:IssueName>Resumed Inc</ndaq:IssueName>
<ndaq:Market>NYSE</ndaq:Market>
<ndaq:ReasonCode>T1</ndaq:ReasonCode>
<ndaq:ResumptionDate>06/15/2026</ndaq:ResumptionDate>
<ndaq:ResumptionTradeTime>10:05:30</ndaq:ResumptionTradeTime>
</item>
</channel>
</rss>"#;

    #[test]
    fn parses_trade_halts_rss_items() {
        let rows = parse_trade_halts_rss(HALTS_RSS_SAMPLE);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "ABCD");
        assert_eq!(rows[0].reason_code, "LUDP");
        assert_eq!(rows[0].name, "Alpha Beta Corp & Co"); // entity decoded
        assert!(rows[0].is_active(), "no resumption time → active");
        assert_eq!(rows[1].symbol, "RESM");
        assert!(!rows[1].is_active(), "has resumption trade time → resumed");
    }

    #[test]
    fn replace_trade_halts_keeps_only_active_halts() {
        let conn = Connection::open_in_memory().unwrap();
        let rows = parse_trade_halts_rss(HALTS_RSS_SAMPLE);
        let inserted = replace_trade_halt_alerts(&conn, &rows).unwrap();
        assert_eq!(inserted, 1, "only the un-resumed halt is stored");
        let alerts = get_regulatory_alerts(&conn).unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].symbol, "ABCD");
        assert_eq!(alerts[0].kind, "trade_halt");
        assert_eq!(alerts[0].label, "!! HALT !!");
        assert!(
            alerts[0]
                .details
                .contains("Volatility trading pause (LULD)"),
            "details carry the decoded reason: {}",
            alerts[0].details
        );
    }

    #[test]
    fn replace_trade_halts_is_idempotent_and_clears_resolved() {
        let conn = Connection::open_in_memory().unwrap();
        let rows = parse_trade_halts_rss(HALTS_RSS_SAMPLE);
        assert_eq!(replace_trade_halt_alerts(&conn, &rows).unwrap(), 1);
        // A later empty feed (all halts resolved) clears the trade_halt rows.
        assert_eq!(replace_trade_halt_alerts(&conn, &[]).unwrap(), 0);
        assert!(get_regulatory_alerts(&conn).unwrap().is_empty());
    }

    #[test]
    fn halt_reason_descriptions_cover_common_codes() {
        assert_eq!(
            halt_reason_description("LUDP"),
            "Volatility trading pause (LULD)"
        );
        assert_eq!(halt_reason_description("t1"), "News pending");
        assert_eq!(halt_reason_description("H10"), "SEC trading suspension");
        assert_eq!(halt_reason_description("ZZ9"), "Trading halt"); // unknown fallback
    }
}
