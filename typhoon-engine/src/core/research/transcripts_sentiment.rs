use serde::{Deserialize, Serialize};

/// Earnings call transcript list entry (metadata only).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TranscriptMeta {
    pub symbol: String,
    pub quarter: i32,
    pub year: i32,
    pub date: String,
}

/// Full transcript content.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Transcript {
    pub symbol: String,
    pub quarter: i32,
    pub year: i32,
    pub date: String,
    pub content: String,
}

/// Social sentiment snapshot (Reddit + Twitter combined from Finnhub).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SocialSentimentRow {
    pub source: String, // "reddit" | "twitter"
    pub at_time: String,
    pub mention: i64,
    pub positive_mention: i64,
    pub negative_mention: i64,
    pub positive_score: f64,
    pub negative_score: f64,
    pub score: f64,
}

/// Recent StockTwits message retained for local research-packet provenance.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StockTwitsMessage {
    pub id: i64,
    pub created_at: String,
    pub username: String,
    pub body: String,
    pub sentiment: String,
    pub like_count: i64,
    pub reshare_count: i64,
}

/// Local StockTwits public-stream reduction for one symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StockTwitsSentimentSnapshot {
    pub symbol: String,
    pub fetched_at: String,
    pub bullish: u32,
    pub bearish: u32,
    pub neutral: u32,
    pub message_count: u32,
    pub bull_bear_ratio: f64,
    pub velocity_24h: u32,
    pub top_messages: Vec<StockTwitsMessage>,
}

/// One Reddit post retained for local research-packet provenance (ADR-117
/// Reddit lane). Reddit has no user-tagged bull/bear sentiment, so the lane
/// presents raw mention counts + engagement with provenance, never a derived
/// buy/sell signal.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RedditPost {
    pub title: String,
    pub subreddit: String,
    pub score: i64,
    pub num_comments: i64,
    pub created_utc: i64,
    pub permalink: String,
}

/// Keyless Reddit mention reduction for one symbol over the trailing day
/// across the finance subreddits (r/wallstreetbets, r/stocks, r/investing,
/// r/StockMarket).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RedditMentionSnapshot {
    pub symbol: String,
    pub fetched_at: String,
    pub mentions_24h: u32,
    pub score_sum_24h: i64,
    pub comments_sum_24h: i64,
    pub top_posts: Vec<RedditPost>,
}

/// One stored point of social history (per symbol × source) so bull/bear and
/// mention counts can be charted over time (ADR-117 sparkline).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SocialHistoryPoint {
    pub symbol: String,
    pub source: String,
    pub fetched_at_ts: i64,
    pub bullish: u32,
    pub bearish: u32,
    pub neutral: u32,
    pub messages: u32,
}

/// Press release item.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PressRelease {
    pub symbol: String,
    pub datetime: String,
    pub headline: String,
    pub description: String,
    pub url: String,
}
