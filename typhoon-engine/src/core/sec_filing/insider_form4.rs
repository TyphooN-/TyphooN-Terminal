use super::{SEC_EDGAR_USER_AGENT, compute_importance, open_conn};
use rusqlite::params;
use std::path::Path;

// ── Form 4 Insider Trade Parsing ────────────────────────────────────

/// Fetch a Form 4 filing and parse insider trades. All DB writes are blocking.
pub(super) async fn fetch_and_parse_form4(
    db_path: &Path,
    client: &reqwest::Client,
    ticker: &str,
    accession: &str,
    url: &str,
) -> Result<(usize, usize), String> {
    // Async: fetch the filing with retry on 429
    let mut body = String::new();
    for attempt in 0..3u32 {
        let resp = client
            .get(url)
            .header("User-Agent", SEC_EDGAR_USER_AGENT)
            .send()
            .await
            .map_err(|e| format!("Form 4 fetch failed: {e}"))?;

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            // Exponential backoff: 1s, 2s, 4s
            let delay = std::time::Duration::from_secs(1 << attempt);
            tracing::debug!("Form 4 429 for {ticker} — retrying in {}s", delay.as_secs());
            tokio::time::sleep(delay).await;
            continue;
        }

        if !resp.status().is_success() {
            return Err(format!("Form 4 HTTP {}", resp.status()));
        }

        body = resp
            .text()
            .await
            .map_err(|e| format!("Form 4 read failed: {e}"))?;
        break;
    }
    if body.is_empty() {
        return Err(format!("Form 4 exhausted retries for {ticker} {accession}"));
    }

    // Parse in-memory (no DB needed)
    let insider_name =
        extract_xml_value(&body, "rptOwnerName").unwrap_or_else(|| "Unknown".to_string());
    let insider_title = extract_xml_value(&body, "officerTitle").unwrap_or_default();
    let is_officer =
        body.contains("<isOfficer>true</isOfficer>") || body.contains("<isOfficer>1</isOfficer>");
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
pub(super) struct ParsedTransaction {
    pub(super) code: String,
    pub(super) shares: f64,
    pub(super) price: f64,
    pub(super) date: String,
}

/// Extract transaction blocks from Form 4 XML/HTML.
pub(super) fn extract_transactions(body: &str) -> Vec<ParsedTransaction> {
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

                let code = extract_xml_value(block, "transactionCode").unwrap_or_default();
                let shares = extract_xml_value(block, "transactionShares")
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .unwrap_or(0.0);
                let price = extract_xml_value(block, "transactionPricePerShare")
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .unwrap_or(0.0);
                let date = extract_xml_value(block, "transactionDate").unwrap_or_default();

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
pub(super) fn extract_xml_value(body: &str, tag: &str) -> Option<String> {
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

/// Human-readable label and direction for an SEC Form 4 transaction code.
/// Direction: `1` = acquired, `-1` = disposed, `0` = neutral/unknown. Used by the
/// structured Form 4 viewer to both describe and color each row — the raw EDGAR
/// document is XSLT table HTML that strips into unreadable pipe-soup, so the
/// parsed transactions are rendered instead.
pub fn form4_transaction_code_label(code: &str) -> (&'static str, i8) {
    match code.trim().to_ascii_uppercase().as_str() {
        "P" => ("Open-market purchase", 1),
        "S" => ("Open-market sale", -1),
        "A" => ("Grant / award", 1),
        "D" => ("Disposition to issuer", -1),
        "M" => ("Option exercise / conversion", 1),
        "X" => ("Derivative exercise", 1),
        "C" => ("Derivative conversion", 1),
        "F" => ("Tax / cost withholding", -1),
        "G" => ("Gift", 0),
        "J" => ("Other acq./disp.", 0),
        "V" => ("Voluntary early report", 0),
        "" => ("—", 0),
        _ => ("Other", 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form4_code_labels_carry_direction_and_are_case_insensitive() {
        assert_eq!(form4_transaction_code_label("P").1, 1);
        assert_eq!(form4_transaction_code_label("s").1, -1);
        assert_eq!(form4_transaction_code_label("A").1, 1);
        assert_eq!(form4_transaction_code_label("F").1, -1);
        assert_eq!(form4_transaction_code_label("G").1, 0);
        assert_eq!(form4_transaction_code_label("").0, "—");
        assert_eq!(form4_transaction_code_label("ZZ").0, "Other");
    }

    #[test]
    fn extract_transactions_parses_nested_value_block() {
        let xml = "<nonDerivativeTransaction>\
            <transactionCode>P</transactionCode>\
            <transactionShares><value>100</value></transactionShares>\
            <transactionPricePerShare><value>10.5</value></transactionPricePerShare>\
            <transactionDate><value>2026-06-23</value></transactionDate>\
            </nonDerivativeTransaction>";
        let txns = extract_transactions(xml);
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].code, "P");
        assert_eq!(txns[0].shares, 100.0);
        assert_eq!(txns[0].price, 10.5);
        assert_eq!(txns[0].date, "2026-06-23");
    }
}
