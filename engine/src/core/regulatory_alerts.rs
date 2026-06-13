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

pub async fn refresh_regsho_threshold_alerts(conn: &Connection) -> Result<usize, String> {
    let (as_of, rows) = fetch_regsho_threshold_entries().await?;
    replace_regsho_threshold_alerts(conn, &as_of, &rows)
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
}
