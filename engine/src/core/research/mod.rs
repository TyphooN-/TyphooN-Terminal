//! Research API helpers — company profiles, earnings, transcripts, IPOs, peers,
//! press releases, social sentiment, commodities futures quotes, and corporate actions.
//!
//! Sources:
//! - Finnhub free tier: /stock/profile2, /stock/peers, /stock/earnings,
//!   /stock/social-sentiment, /press-releases, /calendar/ipo
//! - FMP free tier: /earning_call_transcript, /historical/earning_calendar
//! - Yahoo Finance: /v7/finance/quote (commodities, cross-asset quotes)
//!
//! All functions take an existing reqwest::Client so callers control the HTTP stack
//! (rate limiting, user-agent, timeouts).
//!
//! Research results are cached in SQLite so symbols only need to hit
//! the APIs once per scrape cycle — the DES/PEERS/EARNINGS/PRESS/SENTIMENT/
//! TRANSCRIPTS windows read from cache first and fall back to live fetch.

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

mod types;
pub use types::*;

mod technical;
pub use technical::compute_technical_indicators;

mod providers;
pub use providers::{
    fetch_finnhub_company_snapshot, fetch_finnhub_earnings, fetch_finnhub_ipo_calendar,
    fetch_finnhub_peers, fetch_finnhub_press, fetch_finnhub_profile, fetch_finnhub_social,
    fetch_fmp_transcript, fetch_fmp_transcript_list, fetch_yahoo_quotes,
};

mod fetchers;
pub use fetchers::*;

mod storage_core;
use storage_core::now_ts;
pub use storage_core::*;

mod storage_market_data;
pub use storage_market_data::*;

mod storage_macro_snapshots;
pub use storage_macro_snapshots::*;

mod storage_valuation_snapshots;
pub use storage_valuation_snapshots::*;

mod storage_valuation_models;
pub use storage_valuation_models::*;

mod storage_market_stat_snapshots;
pub use storage_market_stat_snapshots::*;

mod storage_fundamental_risk_snapshots;
pub use storage_fundamental_risk_snapshots::*;

mod storage_financial_quality_snapshots;
pub use storage_financial_quality_snapshots::*;

mod storage_insider_dividend_momentum_snapshots;
pub use storage_insider_dividend_momentum_snapshots::*;

mod storage_factor_rank_snapshots;
pub use storage_factor_rank_snapshots::*;

mod storage_momentum_quality_rank_snapshots;
pub use storage_momentum_quality_rank_snapshots::*;

mod storage_dividend_volatility_rank_snapshots;
pub use storage_dividend_volatility_rank_snapshots::*;

mod storage_return_distribution_snapshots;
pub use storage_return_distribution_snapshots::*;

mod storage_autocorr_drawup_volatility_snapshots;
pub use storage_autocorr_drawup_volatility_snapshots::*;

/// Returns a compact, GUI-friendly company summary string.
/// Suitable for Symbol Explorer, right panels, tooltips, or floating windows.
pub fn get_company_summary(profile: &CompanyProfile) -> String {
    let mut parts = Vec::new();

    if !profile.name.is_empty() {
        parts.push(profile.name.clone());
    }
    if !profile.exchange.is_empty() {
        parts.push(format!("[{}]", profile.exchange));
    }

    let mut meta = Vec::new();
    if !profile.sector.is_empty() {
        meta.push(profile.sector.clone());
    }
    if !profile.industry.is_empty() && profile.industry != profile.sector {
        meta.push(profile.industry.clone());
    }
    if !meta.is_empty() {
        parts.push(format!("({})", meta.join(" · ")));
    }

    if !profile.ipo_date.is_empty() {
        parts.push(format!("IPO: {}", profile.ipo_date));
    }

    if !profile.website.is_empty() {
        parts.push(profile.website.clone());
    }

    let mut out = parts.join("  ");

    if !profile.description.is_empty() {
        let desc = if profile.description.len() > 320 {
            format!("{}…", &profile.description[..320])
        } else {
            profile.description.clone()
        };
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(&desc);
    }

    out
}