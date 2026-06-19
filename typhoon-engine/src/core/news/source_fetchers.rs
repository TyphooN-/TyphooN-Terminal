//! Equity/general news source fetchers and lightweight feed parsers.

use super::{
    GDELT_LAST_REQUEST_TIME, GDELT_MIN_INTERVAL_SECS, NewsArticle, gdelt_cooldown_remaining_secs,
    now_secs, trip_gdelt_cooldown,
};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::time::sleep;

pub async fn fetch_gdelt_news(
    client: &reqwest::Client,
    query: &str,
    max_records: u32,
) -> Result<Vec<NewsArticle>, String> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }
    let remaining_cooldown = gdelt_cooldown_remaining_secs();
    if remaining_cooldown > 0 {
        return Err(format!("GDELT cooldown: {}s remaining", remaining_cooldown));
    }

    let current_time = now_secs();
    let last_request_time = GDELT_LAST_REQUEST_TIME.load(Ordering::Relaxed);
    let elapsed_since_last_request = current_time.saturating_sub(last_request_time);

    if elapsed_since_last_request < GDELT_MIN_INTERVAL_SECS {
        let sleep_duration_secs =
            GDELT_MIN_INTERVAL_SECS.saturating_sub(elapsed_since_last_request);
        sleep(Duration::from_secs(sleep_duration_secs as u64)).await;
    }

    let url = "https://api.gdeltproject.org/api/v2/doc/doc";
    let resp = client
        .get(url)
        .query(&[
            ("query", query),
            ("mode", "ArtList"),
            ("format", "json"),
            ("maxrecords", &max_records.to_string()),
            ("sort", "DateDesc"),
            ("timespan", "24h"),
        ])
        .header("User-Agent", "Mozilla/5.0 TyphooN-Terminal/0.1")
        .send()
        .await
        .map_err(|e| format!("GDELT request failed: {e}"))?;

    GDELT_LAST_REQUEST_TIME.store(now_secs(), Ordering::Relaxed); // Update last request time after sending

    if !resp.status().is_success() {
        let code = resp.status().as_u16();
        if code == 429 {
            trip_gdelt_cooldown();
        }
        return Err(format!("GDELT: HTTP {}", resp.status()));
    }
    let text = resp.text().await.map_err(|e| format!("GDELT read: {e}"))?;
    // GDELT sometimes returns empty body on zero matches — treat as empty list.
    if text.trim().is_empty() {
        return Ok(vec![]);
    }
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        format!(
            "GDELT parse: {e}; body: {}",
            &text.chars().take(120).collect::<String>()
        )
    })?;
    let mut out = Vec::new();
    if let Some(arr) = v["articles"].as_array() {
        for e in arr {
            let url = e["url"].as_str().unwrap_or("").to_string();
            if url.is_empty() {
                continue;
            }
            let title = e["title"].as_str().unwrap_or("").to_string();
            // GDELT timestamps are yyyymmddTHHMMSSZ
            let ts_raw = e["seendate"].as_str().unwrap_or("");
            let published_at = parse_gdelt_ts(ts_raw);
            let art = NewsArticle {
                symbol: query.to_uppercase(),
                source: "GDELT".into(),
                provider: e["domain"].as_str().unwrap_or("").to_string(),
                headline: title,
                summary: String::new(),
                url: url.clone(),
                published_at,
                image_url: e["socialimage"].as_str().unwrap_or("").to_string(),
                ..Default::default()
            }
            .with_hash();
            out.push(art);
        }
    }
    Ok(out)
}

pub(super) fn parse_gdelt_ts(s: &str) -> i64 {
    // Format: "20260413T142030Z"
    if s.len() < 15 {
        return 0;
    }
    let y: i32 = s[0..4].parse().unwrap_or(0);
    let mo: u32 = s[4..6].parse().unwrap_or(0);
    let d: u32 = s[6..8].parse().unwrap_or(0);
    let h: u32 = s[9..11].parse().unwrap_or(0);
    let mi: u32 = s[11..13].parse().unwrap_or(0);
    let sc: u32 = s[13..15].parse().unwrap_or(0);
    chrono::NaiveDate::from_ymd_opt(y, mo, d)
        .and_then(|dt| dt.and_hms_opt(h, mi, sc))
        .map(|ndt| ndt.and_utc().timestamp())
        .unwrap_or(0)
}

/// Yahoo Finance RSS per-symbol feed — no key.
pub async fn fetch_yahoo_rss(
    client: &reqwest::Client,
    symbol: &str,
) -> Result<Vec<NewsArticle>, String> {
    let clean = symbol.replace("/USD", "").replace("/", "").to_uppercase();
    if clean.is_empty() {
        return Ok(vec![]);
    }
    let url = format!(
        "https://feeds.finance.yahoo.com/rss/2.0/headline?s={}&region=US&lang=en-US",
        clean
    );
    let resp = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1",
        )
        .send()
        .await
        .map_err(|e| format!("Yahoo RSS request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Yahoo RSS: HTTP {}", resp.status()));
    }
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Yahoo RSS read: {e}"))?;
    Ok(parse_rss_items(&body, &clean, "YahooRSS"))
}

/// SEC EDGAR Atom feed for a single ticker's recent filings — used as a news proxy.
pub async fn fetch_sec_edgar_rss(
    client: &reqwest::Client,
    symbol: &str,
) -> Result<Vec<NewsArticle>, String> {
    let sym = symbol.to_uppercase();
    if sym.is_empty() {
        return Ok(vec![]);
    }
    let url = format!(
        "https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&CIK={}&type=&dateb=&owner=include&count=20&output=atom",
        sym
    );
    let resp = client
        .get(&url)
        .header("User-Agent", "TyphooN Terminal research@typhoon.local")
        .send()
        .await
        .map_err(|e| format!("EDGAR RSS request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("EDGAR RSS: HTTP {}", resp.status()));
    }
    let body = resp
        .text()
        .await
        .map_err(|e| format!("EDGAR RSS read: {e}"))?;
    Ok(parse_atom_items(&body, &sym, "SEC"))
}

/// Marketaux — 100 req/day free, finance-focused, includes sentiment.
pub async fn fetch_marketaux_news(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<NewsArticle>, String> {
    if token.is_empty() {
        return Err("Marketaux API key required".into());
    }
    let sym = symbol.replace("/", "").to_uppercase();
    let resp = client
        .get("https://api.marketaux.com/v1/news/all")
        .query(&[
            ("api_token", token),
            ("symbols", sym.as_str()),
            ("filter_entities", "true"),
            ("language", "en"),
            ("limit", "20"),
        ])
        .send()
        .await
        .map_err(|e| format!("Marketaux request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Marketaux: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Marketaux parse: {e}"))?;
    let mut out = Vec::new();
    if let Some(arr) = v["data"].as_array() {
        for e in arr {
            let url = e["url"].as_str().unwrap_or("").to_string();
            if url.is_empty() {
                continue;
            }
            let published_at = parse_iso_ts(e["published_at"].as_str().unwrap_or(""));
            let mut tickers = Vec::new();
            let mut sent_score = 0.0;
            if let Some(ents) = e["entities"].as_array() {
                for en in ents {
                    if let Some(sym) = en["symbol"].as_str() {
                        if !sym.is_empty() {
                            tickers.push(sym.to_uppercase());
                        }
                    }
                    if sent_score == 0.0 {
                        sent_score = en["sentiment_score"].as_f64().unwrap_or(0.0);
                    }
                }
            }
            let sentiment = if sent_score > 0.15 {
                "bullish"
            } else if sent_score < -0.15 {
                "bearish"
            } else {
                "neutral"
            };
            let art = NewsArticle {
                symbol: sym.clone(),
                source: "Marketaux".into(),
                provider: e["source"].as_str().unwrap_or("").to_string(),
                headline: e["title"].as_str().unwrap_or("").to_string(),
                summary: e["description"].as_str().unwrap_or("").to_string(),
                url: url.clone(),
                published_at,
                image_url: e["image_url"].as_str().unwrap_or("").to_string(),
                sentiment: sentiment.into(),
                sentiment_score: sent_score,
                tickers,
                ..Default::default()
            }
            .with_hash();
            out.push(art);
        }
    }
    Ok(out)
}

/// Alpha Vantage NEWS_SENTIMENT — 25 req/day free, sentiment + ticker relevance baked in.
pub async fn fetch_alpha_vantage_news(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<NewsArticle>, String> {
    if token.is_empty() {
        return Err("Alpha Vantage API key required".into());
    }
    let sym = symbol.replace("/", "").to_uppercase();
    let resp = client
        .get("https://www.alphavantage.co/query")
        .query(&[
            ("function", "NEWS_SENTIMENT"),
            ("tickers", sym.as_str()),
            ("limit", "50"),
            ("apikey", token),
        ])
        .send()
        .await
        .map_err(|e| format!("Alpha Vantage request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Alpha Vantage: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Alpha Vantage parse: {e}"))?;
    if let Some(note) = v["Information"].as_str() {
        if note.contains("limit") || note.contains("rate") {
            return Err(format!("Alpha Vantage rate limit: {note}"));
        }
    }
    let mut out = Vec::new();
    if let Some(arr) = v["feed"].as_array() {
        for e in arr {
            let url = e["url"].as_str().unwrap_or("").to_string();
            if url.is_empty() {
                continue;
            }
            let ts_raw = e["time_published"].as_str().unwrap_or("");
            let published_at = parse_av_ts(ts_raw);
            let mut tickers = Vec::new();
            let mut rel_sent = 0.0;
            if let Some(ts_arr) = e["ticker_sentiment"].as_array() {
                for t in ts_arr {
                    if let Some(tsym) = t["ticker"].as_str() {
                        tickers.push(tsym.to_uppercase());
                        if tsym.to_uppercase() == sym {
                            rel_sent = t["ticker_sentiment_score"]
                                .as_str()
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(0.0);
                        }
                    }
                }
            }
            let overall_label = e["overall_sentiment_label"].as_str().unwrap_or("");
            let sentiment = match overall_label {
                "Bullish" | "Somewhat-Bullish" => "bullish",
                "Bearish" | "Somewhat-Bearish" => "bearish",
                _ => "neutral",
            };
            let mut categories = Vec::new();
            if let Some(topics) = e["topics"].as_array() {
                for t in topics {
                    if let Some(name) = t["topic"].as_str() {
                        categories.push(name.to_string());
                    }
                }
            }
            let art = NewsArticle {
                symbol: sym.clone(),
                source: "AlphaVantage".into(),
                provider: e["source"].as_str().unwrap_or("").to_string(),
                headline: e["title"].as_str().unwrap_or("").to_string(),
                summary: e["summary"].as_str().unwrap_or("").to_string(),
                url: url.clone(),
                published_at,
                image_url: e["banner_image"].as_str().unwrap_or("").to_string(),
                sentiment: sentiment.into(),
                sentiment_score: rel_sent,
                tickers,
                categories,
                ..Default::default()
            }
            .with_hash();
            out.push(art);
        }
    }
    Ok(out)
}

pub(super) fn parse_av_ts(s: &str) -> i64 {
    // Format: "20260413T142030"
    if s.len() < 15 {
        return 0;
    }
    let y: i32 = s[0..4].parse().unwrap_or(0);
    let mo: u32 = s[4..6].parse().unwrap_or(0);
    let d: u32 = s[6..8].parse().unwrap_or(0);
    let h: u32 = s[9..11].parse().unwrap_or(0);
    let mi: u32 = s[11..13].parse().unwrap_or(0);
    let sc: u32 = s[13..15].parse().unwrap_or(0);
    chrono::NaiveDate::from_ymd_opt(y, mo, d)
        .and_then(|dt| dt.and_hms_opt(h, mi, sc))
        .map(|ndt| ndt.and_utc().timestamp())
        .unwrap_or(0)
}

/// FMP /v3/stock_news — 250 req/day free, clean normalized shape.
pub async fn fetch_fmp_news(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<NewsArticle>, String> {
    if token.is_empty() {
        return Err("FMP API key required".into());
    }
    let sym = symbol.replace("/", "").to_uppercase();
    let url = format!(
        "https://financialmodelingprep.com/api/v3/stock_news?tickers={}&limit=50&apikey={}",
        sym, token
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("FMP news request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP news: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("FMP news parse: {e}"))?;
    let mut out = Vec::new();
    for e in arr {
        let url = e["url"].as_str().unwrap_or("").to_string();
        if url.is_empty() {
            continue;
        }
        let published_at = parse_iso_ts(e["publishedDate"].as_str().unwrap_or(""));
        let art = NewsArticle {
            symbol: sym.clone(),
            source: "FMP".into(),
            provider: e["site"].as_str().unwrap_or("").to_string(),
            headline: e["title"].as_str().unwrap_or("").to_string(),
            summary: e["text"].as_str().unwrap_or("").to_string(),
            url: url.clone(),
            published_at,
            image_url: e["image"].as_str().unwrap_or("").to_string(),
            tickers: vec![sym.clone()],
            ..Default::default()
        }
        .with_hash();
        out.push(art);
    }
    Ok(out)
}

pub fn parse_iso_ts(s: &str) -> i64 {
    if s.is_empty() {
        return 0;
    }
    // Handle "2026-04-13T14:20:30Z" or "2026-04-13 14:20:30" or "2026-04-13T14:20:30.000000Z"
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|d| d.timestamp())
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                .map(|ndt| ndt.and_utc().timestamp())
        })
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f")
                .map(|ndt| ndt.and_utc().timestamp())
        })
        .unwrap_or(0)
}

// ── RSS / Atom parsing (regex-based, deliberately simple) ────────────────

fn strip_cdata(s: &str) -> String {
    let s = s.trim();
    if let Some(inner) = s
        .strip_prefix("<![CDATA[")
        .and_then(|x| x.strip_suffix("]]>"))
    {
        inner.to_string()
    } else {
        s.to_string()
    }
}

fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
}

fn extract_tag<'a>(body: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let start = body.find(&open)?;
    let after_open = &body[start..];
    let content_start = after_open.find('>')? + 1;
    let rest = &after_open[content_start..];
    let end = rest.find(&close)?;
    Some(&rest[..end])
}

fn extract_all_tag_bodies<'a>(body: &'a str, tag: &str) -> Vec<&'a str> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let mut out = Vec::new();
    let mut cursor = 0;
    while let Some(rel_start) = body[cursor..].find(&open) {
        let abs_start = cursor + rel_start;
        let after_open = &body[abs_start..];
        let Some(content_offset) = after_open.find('>') else {
            break;
        };
        let content_start_abs = abs_start + content_offset + 1;
        let rest = &body[content_start_abs..];
        let Some(end_rel) = rest.find(&close) else {
            break;
        };
        out.push(&rest[..end_rel]);
        cursor = content_start_abs + end_rel + close.len();
    }
    out
}

pub(super) fn parse_rss_items(body: &str, symbol: &str, source: &str) -> Vec<NewsArticle> {
    let mut out = Vec::new();
    for item in extract_all_tag_bodies(body, "item") {
        let title = extract_tag(item, "title")
            .map(|s| decode_entities(&strip_cdata(s)))
            .unwrap_or_default();
        let link = extract_tag(item, "link")
            .map(|s| decode_entities(&strip_cdata(s)))
            .unwrap_or_default();
        let desc = extract_tag(item, "description")
            .map(|s| decode_entities(&strip_cdata(s)))
            .unwrap_or_default();
        let pub_date = extract_tag(item, "pubDate")
            .map(|s| strip_cdata(s))
            .unwrap_or_default();
        if link.is_empty() {
            continue;
        }
        let published_at = chrono::DateTime::parse_from_rfc2822(pub_date.trim())
            .map(|d| d.timestamp())
            .unwrap_or(0);
        // Strip inline HTML tags from description.
        let clean_desc = strip_html(&desc);
        let art = NewsArticle {
            symbol: symbol.to_uppercase(),
            source: source.into(),
            provider: String::new(),
            headline: title,
            summary: clean_desc,
            url: link.clone(),
            published_at,
            tickers: vec![symbol.to_uppercase()],
            ..Default::default()
        }
        .with_hash();
        out.push(art);
    }
    out
}

pub(super) fn parse_atom_items(body: &str, symbol: &str, source: &str) -> Vec<NewsArticle> {
    let mut out = Vec::new();
    for entry in extract_all_tag_bodies(body, "entry") {
        let title = extract_tag(entry, "title")
            .map(|s| decode_entities(&strip_cdata(s)))
            .unwrap_or_default();
        // Atom link is <link href="..."/> — search for href attribute.
        let link = extract_atom_link(entry);
        let summary = extract_tag(entry, "summary")
            .or_else(|| extract_tag(entry, "content"))
            .map(|s| decode_entities(&strip_cdata(s)))
            .unwrap_or_default();
        let updated = extract_tag(entry, "updated")
            .or_else(|| extract_tag(entry, "published"))
            .map(|s| strip_cdata(s))
            .unwrap_or_default();
        if link.is_empty() {
            continue;
        }
        let published_at = chrono::DateTime::parse_from_rfc3339(updated.trim())
            .map(|d| d.timestamp())
            .unwrap_or(0);
        let art = NewsArticle {
            symbol: symbol.to_uppercase(),
            source: source.into(),
            provider: String::new(),
            headline: title,
            summary: strip_html(&summary),
            url: link,
            published_at,
            tickers: vec![symbol.to_uppercase()],
            ..Default::default()
        }
        .with_hash();
        out.push(art);
    }
    out
}

fn extract_atom_link(entry: &str) -> String {
    // <link href="..." /> or <link rel="alternate" href="..."/>
    if let Some(idx) = entry.find("<link") {
        let after = &entry[idx..];
        if let Some(href_idx) = after.find("href=\"") {
            let rest = &after[href_idx + 6..];
            if let Some(end) = rest.find('"') {
                return rest[..end].to_string();
            }
        }
    }
    String::new()
}

pub(super) fn strip_html(s: &str) -> String {
    // Minimal tag stripper — keeps text, drops <a>, <b>, <img>, etc.
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    decode_entities(&out).trim().to_string()
}
