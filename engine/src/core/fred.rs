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
///
/// NOTE: FRED requires the API key as a URL query parameter — there is no
/// header-based auth option.  We intentionally avoid logging the full URL
/// to prevent key leakage in traces.
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
    tracing::debug!("FRED fetch series_id={} limit={}", series_id, limit);
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

    // Get series title (URL contains API key — do not log it)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn key_series_has_ten_entries() {
        assert_eq!(KEY_SERIES.len(), 10);
    }

    #[test]
    fn key_series_ids_non_empty() {
        for (id, _) in KEY_SERIES {
            assert!(!id.is_empty(), "found empty series ID");
        }
    }

    #[test]
    fn key_series_titles_non_empty() {
        for (_, title) in KEY_SERIES {
            assert!(!title.is_empty(), "found empty series title");
        }
    }

    #[test]
    fn key_series_no_duplicate_ids() {
        let mut seen = HashSet::new();
        for (id, _) in KEY_SERIES {
            assert!(seen.insert(id), "duplicate series ID: {id}");
        }
    }

    #[test]
    fn fred_series_roundtrip() {
        let series = FredSeries {
            id: "DFF".to_string(),
            title: "Fed Funds Rate".to_string(),
            observations: vec![
                FredObservation { date: "2024-01-01".to_string(), value: 5.33 },
                FredObservation { date: "2024-01-02".to_string(), value: 5.33 },
            ],
        };
        let json = serde_json::to_string(&series).unwrap();
        let back: FredSeries = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "DFF");
        assert_eq!(back.observations.len(), 2);
        assert!((back.observations[0].value - 5.33).abs() < f64::EPSILON);
    }

    #[test]
    fn fred_observation_empty_date() {
        let obs = FredObservation { date: String::new(), value: 0.0 };
        let json = serde_json::to_string(&obs).unwrap();
        let back: FredObservation = serde_json::from_str(&json).unwrap();
        assert!(back.date.is_empty());
    }
}
