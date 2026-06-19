//! Economic calendar — ForexFactory weekly XML feed.
//!
//! Public XML endpoint: https://nfs.faireconomy.media/ff_calendar_thisweek.xml
//! Returns all events for the current week with impact, forecast, previous, actual.
//!
//! Event schema (from XML):
//!   <event>
//!     <title>...</title>
//!     <country>USD</country>
//!     <date>12-04-2026</date>
//!     <time>8:30am</time>
//!     <impact>High|Medium|Low|Holiday</impact>
//!     <forecast>...</forecast>
//!     <previous>...</previous>
//!     <url>...</url>
//!   </event>
//!
//! No authentication required. Rate limit: polite hourly refresh recommended.

use serde::{Deserialize, Serialize};

/// Impact rating for an economic event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EconImpact {
    High,
    Medium,
    Low,
    Holiday,
}

impl EconImpact {
    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "high" => Self::High,
            "medium" => Self::Medium,
            "low" => Self::Low,
            _ => Self::Holiday,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::High => "High",
            Self::Medium => "Medium",
            Self::Low => "Low",
            Self::Holiday => "Holiday",
        }
    }
}

/// One economic calendar event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconEvent {
    pub title: String,
    pub country: String, // Currency code: USD, EUR, GBP, JPY, CHF, CAD, AUD, NZD, CNY
    pub date: String,    // MM-DD-YYYY (as-published) — consumer should normalize
    pub time: String,    // "8:30am" | "All Day" | "Tentative"
    pub impact: EconImpact,
    pub forecast: String,
    pub previous: String,
    pub url: String,
}

/// Fetch + parse the ForexFactory weekly feed.
/// Returns events in chronological order (as delivered by FF).
pub async fn fetch_forexfactory_week(client: &reqwest::Client) -> Result<Vec<EconEvent>, String> {
    let url = "https://nfs.faireconomy.media/ff_calendar_thisweek.xml";
    let resp = client
        .get(url)
        .header("User-Agent", "TyphooN-Terminal/1.0 (+econ-calendar)")
        .send()
        .await
        .map_err(|e| format!("ForexFactory fetch failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("ForexFactory returned HTTP {}", resp.status()));
    }
    let xml = resp
        .text()
        .await
        .map_err(|e| format!("ForexFactory response read failed: {e}"))?;
    parse_xml(&xml)
}

/// Parse the FF XML feed. Uses a pragmatic hand-rolled scanner rather than
/// pulling in a full XML parser — the FF schema is flat, stable, and small.
pub fn parse_xml(xml: &str) -> Result<Vec<EconEvent>, String> {
    let mut events = Vec::new();
    let mut cursor = 0usize;
    while let Some(start) = xml[cursor..].find("<event>") {
        let abs_start = cursor + start + "<event>".len();
        let end = match xml[abs_start..].find("</event>") {
            Some(e) => abs_start + e,
            None => break,
        };
        let block = &xml[abs_start..end];

        let get_tag = |tag: &str| -> String {
            let open = format!("<{tag}>");
            let close = format!("</{tag}>");
            if let Some(o) = block.find(&open) {
                let o = o + open.len();
                if let Some(c) = block[o..].find(&close) {
                    let raw = &block[o..o + c];
                    // Strip CDATA wrapping
                    let trimmed = raw.trim();
                    return if let Some(inner) = trimmed
                        .strip_prefix("<![CDATA[")
                        .and_then(|s| s.strip_suffix("]]>"))
                    {
                        inner.trim().to_string()
                    } else {
                        trimmed.to_string()
                    };
                }
            }
            String::new()
        };

        let title = get_tag("title");
        let country = get_tag("country");
        let date = get_tag("date");
        let time = get_tag("time");
        let impact_s = get_tag("impact");
        let forecast = get_tag("forecast");
        let previous = get_tag("previous");
        let url = get_tag("url");

        if !title.is_empty() {
            events.push(EconEvent {
                title,
                country,
                date,
                time,
                impact: EconImpact::parse(&impact_s),
                forecast,
                previous,
                url,
            });
        }

        cursor = end + "</event>".len();
    }
    Ok(events)
}

/// Parse "MM-DD-YYYY" "h:mmam|pm" into a UTC-ish sortable key.
/// Returns None for "All Day" / "Tentative" entries.
pub fn event_sort_key(e: &EconEvent) -> Option<i64> {
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
    let d = NaiveDate::parse_from_str(&e.date, "%m-%d-%Y").ok()?;
    // FF time format: "8:30am" or "All Day" or "Tentative"
    let t_raw = e.time.trim();
    if t_raw.eq_ignore_ascii_case("all day")
        || t_raw.eq_ignore_ascii_case("tentative")
        || t_raw.is_empty()
    {
        return Some(
            NaiveDateTime::new(d, NaiveTime::from_hms_opt(0, 0, 0)?)
                .and_utc()
                .timestamp(),
        );
    }
    let t = NaiveTime::parse_from_str(t_raw, "%l:%M%P")
        .ok()
        .or_else(|| NaiveTime::parse_from_str(t_raw, "%-l:%M%p").ok())?;
    Some(NaiveDateTime::new(d, t).and_utc().timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<weeklyevents>
  <event>
    <title>Non-Farm Employment Change</title>
    <country>USD</country>
    <date>12-05-2026</date>
    <time>8:30am</time>
    <impact>High</impact>
    <forecast>180K</forecast>
    <previous>150K</previous>
    <url>https://www.forexfactory.com/calendar?nfp</url>
  </event>
  <event>
    <title>Bank Holiday</title>
    <country>GBP</country>
    <date>12-07-2026</date>
    <time>All Day</time>
    <impact>Holiday</impact>
    <forecast></forecast>
    <previous></previous>
    <url></url>
  </event>
</weeklyevents>"#;

    #[test]
    fn test_parse_basic() {
        let events = parse_xml(SAMPLE).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].title, "Non-Farm Employment Change");
        assert_eq!(events[0].country, "USD");
        assert_eq!(events[0].impact, EconImpact::High);
        assert_eq!(events[0].forecast, "180K");
        assert_eq!(events[1].impact, EconImpact::Holiday);
    }

    #[test]
    fn test_parse_empty_fields() {
        let events = parse_xml(SAMPLE).unwrap();
        assert_eq!(events[1].forecast, "");
        assert_eq!(events[1].previous, "");
    }

    #[test]
    fn test_parse_cdata() {
        let cdata = r#"<weeklyevents><event>
            <title><![CDATA[CPI m/m]]></title>
            <country>USD</country>
            <date>12-08-2026</date>
            <time>8:30am</time>
            <impact>High</impact>
            <forecast>0.3%</forecast>
            <previous>0.2%</previous>
            <url></url>
        </event></weeklyevents>"#;
        let events = parse_xml(cdata).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "CPI m/m");
    }

    #[test]
    fn test_impact_parsing() {
        assert_eq!(EconImpact::parse("High"), EconImpact::High);
        assert_eq!(EconImpact::parse("high"), EconImpact::High);
        assert_eq!(EconImpact::parse("Medium"), EconImpact::Medium);
        assert_eq!(EconImpact::parse("Low"), EconImpact::Low);
        assert_eq!(EconImpact::parse("junk"), EconImpact::Holiday);
    }

    #[test]
    fn test_sort_key_ordering() {
        let events = parse_xml(SAMPLE).unwrap();
        let k0 = event_sort_key(&events[0]);
        let k1 = event_sort_key(&events[1]);
        assert!(k0.is_some() && k1.is_some());
        assert!(
            k0.unwrap() < k1.unwrap(),
            "NFP on Dec 5 should sort before holiday on Dec 7"
        );
    }

    #[test]
    fn test_parse_empty() {
        let events = parse_xml("<weeklyevents></weeklyevents>").unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_parse_malformed_tolerant() {
        // Missing closing tag on an event — parser should skip and keep going
        let junk = "<weeklyevents><event><title>Bad</title> (no closing) ";
        let events = parse_xml(junk).unwrap();
        assert!(events.is_empty());
    }
}
