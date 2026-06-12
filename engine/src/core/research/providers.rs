use super::{
    CompanyProfile, CorporateAction, EarningRow, IpoEvent, PressRelease, SocialSentimentRow,
    Transcript, TranscriptMeta,
};

pub async fn fetch_finnhub_profile(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<CompanyProfile, String> {
    if token.is_empty() {
        return Err("Finnhub API key required".into());
    }
    let resp = client
        .get("https://finnhub.io/api/v1/stock/profile2")
        .query(&[("symbol", symbol), ("token", token)])
        .send()
        .await
        .map_err(|e| format!("Finnhub profile failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub profile: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .await
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
        description: v["description"].as_str().unwrap_or("").to_string(),
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
    if token.is_empty() {
        return Err("Finnhub API key required".into());
    }
    let resp = client
        .get("https://finnhub.io/api/v1/stock/peers")
        .query(&[("symbol", symbol), ("token", token)])
        .send()
        .await
        .map_err(|e| format!("Finnhub peers failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub peers: HTTP {}", resp.status()));
    }
    let arr: Vec<String> = resp
        .json()
        .await
        .map_err(|e| format!("Finnhub peers parse: {e}"))?;
    Ok(arr)
}

/// Finnhub /stock/earnings — actual vs estimate EPS per quarter (up to ~16 rows).
pub async fn fetch_finnhub_earnings(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<EarningRow>, String> {
    if token.is_empty() {
        return Err("Finnhub API key required".into());
    }
    let resp = client
        .get("https://finnhub.io/api/v1/stock/earnings")
        .query(&[("symbol", symbol), ("token", token)])
        .send()
        .await
        .map_err(|e| format!("Finnhub earnings failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub earnings: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("Finnhub earnings parse: {e}"))?;
    let rows = arr
        .into_iter()
        .map(|e| {
            let actual = e["actual"].as_f64();
            let estimate = e["estimate"].as_f64();
            let surprise = e["surprise"].as_f64();
            let surprise_pct = e["surprisePercent"].as_f64();
            EarningRow {
                period: e["period"].as_str().unwrap_or("").to_string(),
                actual,
                estimate,
                surprise,
                surprise_pct,
                quarter: e["quarter"].as_i64().map(|q| q as i32),
                year: e["year"].as_i64().map(|y| y as i32),
            }
        })
        .collect();
    Ok(rows)
}

/// Combined helper: fetch profile + earnings and return both.
/// Useful for scraper wiring.
pub async fn fetch_finnhub_company_snapshot(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<(CompanyProfile, Vec<EarningRow>), String> {
    let profile = fetch_finnhub_profile(client, symbol, token).await?;
    let earnings = fetch_finnhub_earnings(client, symbol, token).await?;
    Ok((profile, earnings))
}