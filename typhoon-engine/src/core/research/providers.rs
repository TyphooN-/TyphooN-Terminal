use super::{
    CompanyProfile, EarningRow, IpoEvent, PressRelease, SocialSentimentRow, StockTwitsMessage,
    StockTwitsSentimentSnapshot, Transcript, TranscriptMeta,
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

/// Finnhub profile + earnings snapshot for callers that want a compact company refresh.
pub async fn fetch_finnhub_company_snapshot(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<(CompanyProfile, Vec<EarningRow>), String> {
    let profile = fetch_finnhub_profile(client, symbol, token).await?;
    let earnings = fetch_finnhub_earnings(client, symbol, token).await?;
    Ok((profile, earnings))
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
                quarter: e["quarter"].as_i64().map(|v| v as i32),
                year: e["year"].as_i64().map(|v| v as i32),
            }
        })
        .collect();
    Ok(rows)
}

/// Finnhub /calendar/ipo — upcoming IPOs in a date range.
pub async fn fetch_finnhub_ipo_calendar(
    client: &reqwest::Client,
    token: &str,
    from: &str,
    to: &str,
) -> Result<Vec<IpoEvent>, String> {
    if token.is_empty() {
        return Err("Finnhub API key required".into());
    }
    let resp = client
        .get("https://finnhub.io/api/v1/calendar/ipo")
        .query(&[("token", token), ("from", from), ("to", to)])
        .send()
        .await
        .map_err(|e| format!("Finnhub IPO calendar failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub IPO: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Finnhub IPO parse: {e}"))?;
    let mut rows = Vec::new();
    if let Some(arr) = v["ipoCalendar"].as_array() {
        for e in arr {
            rows.push(IpoEvent {
                date: e["date"].as_str().unwrap_or("").to_string(),
                symbol: e["symbol"].as_str().unwrap_or("").to_string(),
                name: e["name"].as_str().unwrap_or("").to_string(),
                exchange: e["exchange"].as_str().unwrap_or("").to_string(),
                price_range: e["price"].as_str().unwrap_or("").to_string(),
                shares: e["numberOfShares"].as_i64().unwrap_or(0),
                total_value: e["totalSharesValue"].as_f64().unwrap_or(0.0),
                status: e["status"].as_str().unwrap_or("").to_string(),
            });
        }
    }
    Ok(rows)
}

/// Finnhub /press-releases — company press releases (last 90 days).
pub async fn fetch_finnhub_press(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<PressRelease>, String> {
    if token.is_empty() {
        return Err("Finnhub API key required".into());
    }
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let ninety_ago = (chrono::Utc::now() - chrono::Duration::days(90))
        .format("%Y-%m-%d")
        .to_string();
    let resp = client
        .get("https://finnhub.io/api/v1/press-releases")
        .query(&[
            ("symbol", symbol),
            ("token", token),
            ("from", ninety_ago.as_str()),
            ("to", today.as_str()),
        ])
        .send()
        .await
        .map_err(|e| format!("Finnhub press failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub press: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Finnhub press parse: {e}"))?;
    let mut rows = Vec::new();
    if let Some(arr) = v["majorDevelopment"].as_array() {
        for e in arr {
            rows.push(PressRelease {
                symbol: symbol.to_uppercase(),
                datetime: e["datetime"].as_str().unwrap_or("").to_string(),
                headline: e["headline"].as_str().unwrap_or("").to_string(),
                description: e["description"].as_str().unwrap_or("").to_string(),
                url: e["url"].as_str().unwrap_or("").to_string(),
            });
        }
    }
    Ok(rows)
}

/// Finnhub /stock/social-sentiment — Reddit + Twitter daily mention buckets (last 30 days).
pub async fn fetch_finnhub_social(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<SocialSentimentRow>, String> {
    if token.is_empty() {
        return Err("Finnhub API key required".into());
    }
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let month_ago = (chrono::Utc::now() - chrono::Duration::days(30))
        .format("%Y-%m-%d")
        .to_string();
    let resp = client
        .get("https://finnhub.io/api/v1/stock/social-sentiment")
        .query(&[
            ("symbol", symbol),
            ("token", token),
            ("from", month_ago.as_str()),
            ("to", today.as_str()),
        ])
        .send()
        .await
        .map_err(|e| format!("Finnhub social failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub social: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Finnhub social parse: {e}"))?;
    let mut rows = Vec::new();
    for src in ["reddit", "twitter"].iter() {
        if let Some(arr) = v[src].as_array() {
            for e in arr {
                rows.push(SocialSentimentRow {
                    source: src.to_string(),
                    at_time: e["atTime"].as_str().unwrap_or("").to_string(),
                    mention: e["mention"].as_i64().unwrap_or(0),
                    positive_mention: e["positiveMention"].as_i64().unwrap_or(0),
                    negative_mention: e["negativeMention"].as_i64().unwrap_or(0),
                    positive_score: e["positiveScore"].as_f64().unwrap_or(0.0),
                    negative_score: e["negativeScore"].as_f64().unwrap_or(0.0),
                    score: e["score"].as_f64().unwrap_or(0.0),
                });
            }
        }
    }
    Ok(rows)
}

/// StockTwits public symbol stream — unauthenticated recent messages with optional user sentiment tags.
pub async fn fetch_stocktwits_sentiment(
    client: &reqwest::Client,
    symbol: &str,
) -> Result<StockTwitsSentimentSnapshot, String> {
    let symbol = symbol.trim().to_uppercase();
    if symbol.is_empty() {
        return Err("StockTwits symbol required".into());
    }
    let url = format!(
        "https://api.stocktwits.com/api/2/streams/symbol/{}.json",
        symbol
    );
    let resp = client
        .get(&url)
        .header("User-Agent", "TyphooN-Terminal/0.1")
        .send()
        .await
        .map_err(|e| format!("StockTwits failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("StockTwits: HTTP {}", resp.status()));
    }
    let text = resp
        .text()
        .await
        .map_err(|e| format!("StockTwits body: {e}"))?;
    parse_stocktwits_symbol_stream(&symbol, &text)
}

pub fn parse_stocktwits_symbol_stream(
    symbol: &str,
    payload: &str,
) -> Result<StockTwitsSentimentSnapshot, String> {
    let v: serde_json::Value =
        serde_json::from_str(payload).map_err(|e| format!("StockTwits parse: {e}"))?;
    let messages = v["messages"]
        .as_array()
        .ok_or_else(|| "StockTwits parse: missing messages array".to_string())?;
    let now = chrono::Utc::now();
    let mut snapshot = StockTwitsSentimentSnapshot {
        symbol: symbol.trim().to_uppercase(),
        fetched_at: now.to_rfc3339(),
        ..Default::default()
    };
    for msg in messages.iter().take(30) {
        let sentiment = msg
            .pointer("/entities/sentiment/basic")
            .and_then(|s| s.as_str())
            .unwrap_or("Neutral")
            .to_string();
        match sentiment.as_str() {
            "Bullish" => snapshot.bullish += 1,
            "Bearish" => snapshot.bearish += 1,
            _ => snapshot.neutral += 1,
        }
        let created_at = msg["created_at"].as_str().unwrap_or("").to_string();
        if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&created_at) {
            if now.signed_duration_since(ts.with_timezone(&chrono::Utc))
                <= chrono::Duration::hours(24)
            {
                snapshot.velocity_24h += 1;
            }
        }
        snapshot.top_messages.push(StockTwitsMessage {
            id: msg["id"].as_i64().unwrap_or_default(),
            created_at,
            username: msg
                .pointer("/user/username")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
            body: msg["body"].as_str().unwrap_or("").to_string(),
            sentiment,
            like_count: msg
                .pointer("/likes/total")
                .and_then(|v| v.as_i64())
                .unwrap_or_default(),
            reshare_count: msg
                .pointer("/reshares/reshared_count")
                .and_then(|v| v.as_i64())
                .unwrap_or_default(),
        });
    }
    snapshot.message_count = snapshot.top_messages.len() as u32;
    snapshot.bull_bear_ratio = if snapshot.bearish == 0 {
        snapshot.bullish as f64
    } else {
        snapshot.bullish as f64 / snapshot.bearish as f64
    };
    Ok(snapshot)
}

// ── FMP fetchers ───────────────────────────────────────────────────────────

/// FMP /earning_call_transcript/{symbol} list endpoint — returns available [year, quarter, date] triples.
pub async fn fetch_fmp_transcript_list(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<TranscriptMeta>, String> {
    if fmp_key.is_empty() {
        return Err("FMP API key required".into());
    }
    // FMP returns e.g. [[4, 2023, "2024-02-01"], [3, 2023, "2023-11-02"], ...]
    let url = format!(
        "https://financialmodelingprep.com/api/v4/earning_call_transcript?symbol={}&apikey={}",
        symbol, fmp_key
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("FMP transcript list failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP transcript list: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("FMP transcript list parse: {e}"))?;
    let mut rows = Vec::new();
    if let Some(arr) = v.as_array() {
        for entry in arr {
            if let Some(triple) = entry.as_array() {
                if triple.len() >= 3 {
                    let quarter = triple[0].as_i64().unwrap_or(0) as i32;
                    let year = triple[1].as_i64().unwrap_or(0) as i32;
                    let date = triple[2].as_str().unwrap_or("").to_string();
                    rows.push(TranscriptMeta {
                        symbol: symbol.to_uppercase(),
                        quarter,
                        year,
                        date,
                    });
                }
            }
        }
    }
    Ok(rows)
}

/// FMP /earning_call_transcript/{symbol}?quarter=N&year=Y — full transcript body.
pub async fn fetch_fmp_transcript(
    client: &reqwest::Client,
    symbol: &str,
    quarter: i32,
    year: i32,
    fmp_key: &str,
) -> Result<Transcript, String> {
    if fmp_key.is_empty() {
        return Err("FMP API key required".into());
    }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/earning_call_transcript/{}?quarter={}&year={}&apikey={}",
        symbol, quarter, year, fmp_key
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("FMP transcript failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP transcript: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("FMP transcript parse: {e}"))?;
    if arr.is_empty() {
        return Err(format!(
            "No transcript for {} Q{} {}",
            symbol, quarter, year
        ));
    }
    let e = &arr[0];
    Ok(Transcript {
        symbol: symbol.to_uppercase(),
        quarter: e["quarter"].as_i64().unwrap_or(quarter as i64) as i32,
        year: e["year"].as_i64().unwrap_or(year as i64) as i32,
        date: e["date"].as_str().unwrap_or("").to_string(),
        content: e["content"].as_str().unwrap_or("").to_string(),
    })
}

// ── Yahoo fetchers ─────────────────────────────────────────────────────────

/// Yahoo /v7/finance/quote — batch commodities quote.
/// Returns (symbol, display_name, price, change, change_pct).
pub async fn fetch_yahoo_quotes(
    client: &reqwest::Client,
    symbols: &[&str],
) -> Result<Vec<(String, f64, f64, f64)>, String> {
    if symbols.is_empty() {
        return Ok(vec![]);
    }
    let joined = symbols.join(",");
    let url = format!(
        "https://query1.finance.yahoo.com/v7/finance/quote?symbols={}",
        joined
    );
    let resp = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1",
        )
        .send()
        .await
        .map_err(|e| format!("Yahoo quote failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Yahoo quote: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Yahoo quote parse: {e}"))?;
    let mut out = Vec::new();
    if let Some(arr) = v
        .pointer("/quoteResponse/result")
        .and_then(|r| r.as_array())
    {
        for q in arr {
            let sym = q["symbol"].as_str().unwrap_or("").to_string();
            let price = q["regularMarketPrice"].as_f64().unwrap_or(0.0);
            let change = q["regularMarketChange"].as_f64().unwrap_or(0.0);
            let pct = q["regularMarketChangePercent"].as_f64().unwrap_or(0.0);
            if !sym.is_empty() {
                out.push((sym, price, change, pct));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::parse_stocktwits_symbol_stream;

    #[test]
    fn parse_stocktwits_symbol_stream_counts_sentiment_and_preserves_top_messages() {
        let payload = r#"
        {
          "messages": [
            {
              "id": 101,
              "created_at": "2026-06-25T12:00:00Z",
              "body": "AMC squeeze setup",
              "user": { "username": "bull" },
              "likes": { "total": 7 },
              "reshares": { "reshared_count": 2 },
              "entities": { "sentiment": { "basic": "Bullish" } }
            },
            {
              "id": 102,
              "created_at": "2026-06-25T11:00:00Z",
              "body": "Looks weak",
              "user": { "username": "bear" },
              "entities": { "sentiment": { "basic": "Bearish" } }
            },
            {
              "id": 103,
              "created_at": "2026-06-25T10:30:00Z",
              "body": "Watching volume",
              "user": { "username": "neutral" },
              "entities": { "sentiment": null }
            }
          ]
        }"#;

        let snapshot = parse_stocktwits_symbol_stream("amc", payload).unwrap();

        assert_eq!(snapshot.symbol, "AMC");
        assert_eq!(snapshot.bullish, 1);
        assert_eq!(snapshot.bearish, 1);
        assert_eq!(snapshot.neutral, 1);
        assert_eq!(snapshot.message_count, 3);
        assert_eq!(snapshot.bull_bear_ratio, 1.0);
        assert_eq!(snapshot.top_messages.len(), 3);
        assert_eq!(snapshot.top_messages[0].sentiment, "Bullish");
        assert_eq!(snapshot.top_messages[0].username, "bull");
        assert_eq!(snapshot.top_messages[0].like_count, 7);
        assert_eq!(snapshot.top_messages[0].reshare_count, 2);
    }
}
