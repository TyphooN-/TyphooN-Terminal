//! FRED (Federal Reserve Economic Data) — free API for economic indicators.
//! Provides: Fed Funds Rate, CPI, GDP, Treasury Yields, VIX, M2 Money Supply, Unemployment.

use serde::{Serialize, Deserialize};

const FRED_BASE: &str = "https://api.stlouisfed.org/fred";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FredObservation {
    pub date: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FredSeries {
    pub id: String,
    pub title: String,
    pub observations: Vec<FredObservation>,
}

/// Fetch a FRED series (e.g., "DFF" for Fed Funds Rate).
pub async fn fetch_series(
    client: &reqwest::Client,
    api_key: &str,
    series_id: &str,
    limit: u32,
) -> Result<FredSeries, String> {
    let url = format!(
        "{}/series/observations?series_id={}&api_key={}&file_type=json&sort_order=desc&limit={}",
        FRED_BASE, series_id, api_key, limit
    );
    let resp = client.get(&url)
        .timeout(std::time::Duration::from_secs(15))
        .send().await
        .map_err(|e| format!("FRED request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FRED HTTP {}", resp.status()));
    }
    let body: serde_json::Value = resp.json().await
        .map_err(|e| format!("FRED parse failed: {e}"))?;

    let mut observations = Vec::new();
    if let Some(obs) = body["observations"].as_array() {
        for o in obs {
            let date = o["date"].as_str().unwrap_or("").to_string();
            let val_str = o["value"].as_str().unwrap_or(".");
            if let Ok(v) = val_str.parse::<f64>() {
                observations.push(FredObservation { date, value: v });
            }
        }
    }
    observations.reverse(); // chronological order

    // Get series title
    let title_url = format!("{}/series?series_id={}&api_key={}&file_type=json", FRED_BASE, series_id, api_key);
    let title = if let Ok(resp) = client.get(&title_url).send().await {
        if let Ok(body) = resp.json::<serde_json::Value>().await {
            body["seriess"][0]["title"].as_str().unwrap_or(series_id).to_string()
        } else { series_id.to_string() }
    } else { series_id.to_string() };

    Ok(FredSeries { id: series_id.to_string(), title, observations })
}

/// Fetch Treasury yield curve (2Y, 5Y, 10Y, 30Y rates).
pub async fn fetch_yield_curve(client: &reqwest::Client, api_key: &str) -> Result<Vec<(String, f64)>, String> {
    let series = ["DGS2", "DGS5", "DGS10", "DGS30"];
    let labels = ["2Y", "5Y", "10Y", "30Y"];
    let mut curve = Vec::new();
    for (id, label) in series.iter().zip(&labels) {
        match fetch_series(client, api_key, id, 1).await {
            Ok(s) => {
                if let Some(obs) = s.observations.last() {
                    curve.push((label.to_string(), obs.value));
                }
            }
            Err(_) => {}
        }
    }
    Ok(curve)
}

/// Key FRED series IDs for economic dashboard.
pub const KEY_SERIES: &[(&str, &str)] = &[
    ("DFF", "Fed Funds Rate"),
    ("CPIAUCSL", "CPI (All Urban)"),
    ("GDP", "GDP"),
    ("UNRATE", "Unemployment Rate"),
    ("M2SL", "M2 Money Supply"),
    ("VIXCLS", "VIX"),
    ("T10Y2Y", "10Y-2Y Spread"),
    ("WALCL", "Fed Balance Sheet"),
    ("RRPONTSYD", "Reverse Repo"),
    ("WTREGEN", "Treasury General Account"),
];
