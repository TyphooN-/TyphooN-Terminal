//! Research API helpers — company profiles, earnings, transcripts, IPOs, peers,
//! press releases, social sentiment, commodities futures quotes.
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

mod corporate_actions_analyst_index_types;
pub use corporate_actions_analyst_index_types::{
    AnalystRecommendation, EsgScore, EtfHolding, IndexMember, PriceTarget, StockSplit,
};

mod exec_cot;
pub use exec_cot::{CotReport, Executive};

mod ownership_price_history_types;
pub use ownership_price_history_types::{
    EarningsSurprise, HistoricalPriceRow, InsiderTrade, InstitutionalHolder, SharesFloat,
};

mod dividends_ratings_treasury_types;
pub use dividends_ratings_treasury_types::{
    DividendRecord, EarningsEstimate, RatingChange, TREASURY_TENORS, TreasuryYield,
};

mod fx_beta_valuation_identifier_types;
pub use fx_beta_valuation_identifier_types::{
    BetaSnapshot, BetaWindow, COMMODITIES_UNIVERSE, CurrencyRate, DdmSnapshot, FX_MAJORS_UNIVERSE,
    FigiIdentifier, FigiSnapshot, RelativeValuation, RvMetricRow,
};

mod advanced_valuation_derivatives_types;
pub use advanced_valuation_derivatives_types::{max_pain_by_expiration, max_pain_strike, 
    DcfSnapshot, DcfYear, HraSnapshot, HraWindow, IvolObservation, IvolSnapshot, OptionContract,
    OptionExpiry, OptionsChainSnapshot, SvmModelRow, SvmSnapshot,
};

mod market_statistics_types;
pub use market_statistics_types::{
    CorrelationCell, CorrelationMatrix, SeasonalityDow, SeasonalityMonth, SeasonalitySnapshot,
    SkewExpiry, SkewPoint, TechnicalIndicator, TechnicalSnapshot, TotalReturnSnapshot,
    TotalReturnWindow, VolatilitySkew,
};

mod fundamental_risk_types;
pub use fundamental_risk_types::{
    AccrualPeriod, AccrualsSnapshot, FcfYieldPeriod, FcfYieldSnapshot, LeverageRatio,
    LeverageSnapshot, RealizedVolSnapshot, RealizedVolWindow, ShortInterestHistoryPoint,
    ShortInterestSnapshot,
};

mod transcripts_sentiment;
pub use transcripts_sentiment::{
    PressRelease, RedditMentionSnapshot, RedditPost, SocialHistoryPoint, SocialSentimentRow,
    StockTwitsMessage, StockTwitsSentimentSnapshot, Transcript, TranscriptMeta,
};

mod financials;
pub use financials::{BalanceSheet, CashFlowStatement, FinancialStatements, IncomeStatement};
mod technical;
pub use technical::compute_technical_indicators;
mod finviz;
pub use finviz::{FinvizSnapshot, PerfWindows, build_finviz_snapshot, perf_windows};
mod providers;
pub use providers::{
    fetch_finnhub_company_snapshot, fetch_finnhub_earnings, fetch_finnhub_ipo_calendar,
    fetch_finnhub_peers, fetch_finnhub_press, fetch_finnhub_profile, fetch_finnhub_social,
    fetch_fmp_transcript, fetch_fmp_transcript_list, fetch_reddit_mentions,
    fetch_stocktwits_sentiment, fetch_yahoo_quotes, parse_reddit_search,
    parse_stocktwits_symbol_stream,
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
mod storage_risk_liquidity_stat_snapshots;
pub use storage_risk_liquidity_stat_snapshots::*;
mod storage_quant_stat_snapshots;
pub use storage_quant_stat_snapshots::*;
mod storage_fractal_jump_stationarity_snapshots;
pub use storage_fractal_jump_stationarity_snapshots::*;
mod storage_stat_indicator_snapshots;
pub use storage_stat_indicator_snapshots::*;
mod storage_oscillator_moving_average_snapshots;
pub use storage_oscillator_moving_average_snapshots::*;
mod storage_trend_volume_cycle_snapshots;
pub use storage_trend_volume_cycle_snapshots::*;
mod storage_late_indicator_snapshots;
pub use storage_late_indicator_snapshots::*;
mod storage_candlestick_basic_snapshots;
pub use storage_candlestick_basic_snapshots::*;
mod storage_quant_break_volatility_snapshots;
pub use storage_quant_break_volatility_snapshots::*;
mod storage_candlestick_continuation_snapshots;
pub use storage_candlestick_continuation_snapshots::*;
mod storage_candlestick_extended_snapshots;
pub use storage_candlestick_extended_snapshots::*;
mod storage_web_articles;
pub use storage_web_articles::*;
mod scrape;
pub use scrape::scrape_and_cache_symbol;

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
        let desc: String = profile.description.chars().take(320).collect();
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(&desc);
        if profile.description.chars().count() > 320 {
            out.push('…');
        }
    }

    out
}

// ── SQLite cache schema / first-generation cache helpers ────────────────

/// Build a WACC snapshot by combining FMP profile (beta + market cap) with the
/// latest cached FA income/balance data (interest expense, total debt, tax rate)
/// and a caller-supplied risk-free rate (typically 10Y Treasury yield %).
///
/// This is a pure derivation: it does NOT hit the network.  Callers should
/// fetch the inputs first (profile, financials, yield curve) then pass them in.
mod valuation;
#[cfg(test)]
pub(crate) use valuation::ols_regression;
pub use valuation::*;

mod market_stats;
pub use market_stats::*;

mod fundamental_stats;
pub use fundamental_stats::*;
mod volatility_cashflow_short_interest;
pub use volatility_cashflow_short_interest::*;
mod fundamental_risk_analyst_models;
pub use fundamental_risk_analyst_models::*;
mod fundamental_momentum_models;
use fundamental_momentum_models::parse_yyyy_mm_dd_to_days;
pub use fundamental_momentum_models::*;
mod market_liquidity_credit_models;
pub use market_liquidity_credit_models::*;
mod factor_composite_models;
pub use factor_composite_models::*;
mod relative_rank_event_models;
pub use relative_rank_event_models::*;
mod financial_growth_rank_models;
pub use financial_growth_rank_models::*;
mod growth_flow_regime_models;
pub use growth_flow_regime_models::*;
mod market_price_rank_models;
pub use market_price_rank_models::*;
mod fundamental_signal_continuation_models;
use fundamental_signal_continuation_models::merge_short_interest_history_rows;
pub use fundamental_signal_continuation_models::*;
use relative_rank_event_models::{
    percentile_rank_score, quantile_f64, rank_label_for_percentile, risk_rank_label_for_percentile,
};

mod return_risk_stats;
pub use return_risk_stats::*;
pub(crate) use return_risk_stats::{chi2_upper_tail, std_normal_cdf, trailing_log_returns};
mod return_diagnostics_models;
pub use return_diagnostics_models::*;
mod technical_indicator_models;
pub use technical_indicator_models::*;
mod moving_average_oscillator_models;
pub use moving_average_oscillator_models::*;
use moving_average_oscillator_models::{ema_series, sma_series};
mod price_transform_indicator_models;
pub use price_transform_indicator_models::*;
mod price_momentum_indicator_storage;
pub use price_momentum_indicator_storage::*;
mod candlestick_pattern_models;
pub use candlestick_pattern_models::*;
mod quant_statistical_test_models;
pub use quant_statistical_test_models::*;

// ── SQLite schema + helpers ────────────────────────────────────────

/// Whole-table scan of `research_divg`. Used by DVDRANK.
pub fn get_all_divg(conn: &Connection) -> Result<Vec<DivgSnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_divg")
        .map_err(|e| format!("prepare get_all_divg: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_divg: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<DivgSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_earm`. Used by EARMRANK.
pub fn get_all_earm(conn: &Connection) -> Result<Vec<EarmSnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_earm")
        .map_err(|e| format!("prepare get_all_earm: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_earm: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<EarmSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_updm`. Used by UPDGRANK.
pub fn get_all_updm(conn: &Connection) -> Result<Vec<UpdmSnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_updm")
        .map_err(|e| format!("prepare get_all_updm: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_updm: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<UpdmSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

// ── Options Expiration Calendar ────────────────────────────────────

/// Tier 1: entry in the offline market calendar — one expiration candidate
/// generated from pure date math. `expiry_type` is the canonical
/// classification (WEEKLY / MONTHLY / QUARTERLY / TRIPLE_WITCHING / LEAPS)
/// and `is_triple_witching` is a convenience flag that is also true for the
/// TRIPLE_WITCHING type. `days_from_now` is `(expiry_date - from_date)` at
/// generation time; consumers should re-compute if they cache the struct.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CalendarExpiry {
    pub date: String,    // YYYY-MM-DD
    pub weekday: String, // e.g. "Friday"
    pub days_from_now: i64,
    pub expiry_type: String, // WEEKLY / MONTHLY / QUARTERLY / TRIPLE_WITCHING / LEAPS
    pub is_triple_witching: bool,
}

/// Tier 2: per-expiration aggregate derived from a concrete options chain
/// snapshot. Volume and OI are summed across all strikes; `put_call_ratio`
/// is `put_volume / call_volume` (0 if call volume is zero). Labelled
/// `expiry_type` uses the same classifier as the Tier 1 generator.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SymbolExpiration {
    pub date: String, // YYYY-MM-DD
    pub days_to_expiry: i64,
    pub expiry_type: String, // WEEKLY / MONTHLY / QUARTERLY / TRIPLE_WITCHING / LEAPS
    pub call_count: usize,   // number of call strikes
    pub put_count: usize,    // number of put strikes
    pub total_call_volume: f64,
    pub total_put_volume: f64,
    pub total_call_oi: f64,
    pub total_put_oi: f64,
    pub put_call_ratio: f64, // put_volume / call_volume
}

/// Tier 2 snapshot: all upcoming expirations for a single symbol, derived
/// from `research_options_chain.expirations[]`.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SymbolExpirationsSnapshot {
    pub symbol: String,
    pub as_of: String, // YYYY-MM-DD of the underlying chain snapshot
    pub underlying_price: f64,
    pub expirations: Vec<SymbolExpiration>,
    pub next_triple_witching: String, // YYYY-MM-DD or "" if none in chain
    pub note: String,
}

/// True for the 3rd Friday of the month (canonical monthly equity-option
/// expiration in US markets).
pub fn is_third_friday(date: &chrono::NaiveDate) -> bool {
    use chrono::Datelike;
    date.weekday() == chrono::Weekday::Fri && (15..=21).contains(&date.day())
}

/// True for the 3rd Friday of March/June/September/December — the CBOE
/// "triple witching" dates where stock options, stock-index options, and
/// stock-index futures expire simultaneously.
pub fn is_triple_witching(date: &chrono::NaiveDate) -> bool {
    use chrono::Datelike;
    matches!(date.month(), 3 | 6 | 9 | 12) && is_third_friday(date)
}

/// Classify an expiration date into one of: WEEKLY / MONTHLY / QUARTERLY /
/// TRIPLE_WITCHING / LEAPS. `reference_year` is the current year; an
/// expiration more than ~9 months after the reference year's January 3rd-
/// Friday is considered LEAPS.
pub fn classify_expiration(date: &chrono::NaiveDate, reference: &chrono::NaiveDate) -> String {
    use chrono::Datelike;
    if is_triple_witching(date) {
        return "TRIPLE_WITCHING".to_string();
    }
    // LEAPS: more than ~9 months out (typically January expirations 1-2 years ahead)
    let days_out = (date.num_days_from_ce() - reference.num_days_from_ce()).max(0);
    if days_out > 270 && is_third_friday(date) {
        return "LEAPS".to_string();
    }
    // Quarterly: last trading day of March/June/September/December (non-triple-witching
    // monthly in those months is classified QUARTERLY)
    if matches!(date.month(), 3 | 6 | 9 | 12) && is_third_friday(date) {
        return "QUARTERLY".to_string();
    }
    if is_third_friday(date) {
        return "MONTHLY".to_string();
    }
    "WEEKLY".to_string()
}

/// Generate Tier 1 offline market calendar entries for the next `horizon_days`
/// from `from_date`. Emits every Friday within the horizon, classified.
pub fn compute_market_calendar(
    from_date: chrono::NaiveDate,
    horizon_days: u32,
) -> Vec<CalendarExpiry> {
    use chrono::{Datelike, Duration, Weekday};
    let mut out = Vec::new();
    let mut cursor = from_date;
    // Advance to the next Friday (including today if today is Friday)
    while cursor.weekday() != Weekday::Fri {
        cursor += Duration::days(1);
    }
    let end = from_date + Duration::days(horizon_days as i64);
    while cursor <= end {
        let expiry_type = classify_expiration(&cursor, &from_date);
        let is_tw = expiry_type == "TRIPLE_WITCHING";
        let days_from_now = (cursor.num_days_from_ce() - from_date.num_days_from_ce()) as i64;
        out.push(CalendarExpiry {
            date: cursor.format("%Y-%m-%d").to_string(),
            weekday: "Friday".to_string(),
            days_from_now,
            expiry_type,
            is_triple_witching: is_tw,
        });
        cursor += Duration::days(7);
    }
    out
}

/// Tier 2: read the options chain snapshot from cache, aggregate volume/OI
/// per expiration, classify each one, and return a SymbolExpirationsSnapshot.
pub fn compute_symbol_expirations(
    conn: &Connection,
    symbol: &str,
) -> Result<SymbolExpirationsSnapshot, String> {
    use chrono::NaiveDate;
    let chain = match get_options_chain(conn, symbol)? {
        Some(c) => c,
        None => {
            return Ok(SymbolExpirationsSnapshot {
                symbol: symbol.to_uppercase(),
                note: "No options chain cached for symbol; run OPTIONS first.".to_string(),
                ..Default::default()
            });
        }
    };
    let reference = chrono::Local::now().date_naive();
    let mut expirations: Vec<SymbolExpiration> = Vec::with_capacity(chain.expirations.len());
    let mut next_tw = String::new();
    for e in &chain.expirations {
        let parsed = match NaiveDate::parse_from_str(&e.expiration, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => continue,
        };
        let ex_type = classify_expiration(&parsed, &reference);
        if ex_type == "TRIPLE_WITCHING" && next_tw.is_empty() {
            next_tw = e.expiration.clone();
        }
        let total_call_volume: f64 = e.calls.iter().map(|c| c.volume).sum();
        let total_put_volume: f64 = e.puts.iter().map(|p| p.volume).sum();
        let total_call_oi: f64 = e.calls.iter().map(|c| c.open_interest).sum();
        let total_put_oi: f64 = e.puts.iter().map(|p| p.open_interest).sum();
        let put_call_ratio = if total_call_volume > 0.0 {
            total_put_volume / total_call_volume
        } else {
            0.0
        };
        expirations.push(SymbolExpiration {
            date: e.expiration.clone(),
            days_to_expiry: e.days_to_expiry,
            expiry_type: ex_type,
            call_count: e.calls.len(),
            put_count: e.puts.len(),
            total_call_volume,
            total_put_volume,
            total_call_oi,
            total_put_oi,
            put_call_ratio,
        });
    }
    let note = if expirations.is_empty() {
        "Chain present but no parseable expirations.".to_string()
    } else {
        String::new()
    };
    Ok(SymbolExpirationsSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: chain.as_of,
        underlying_price: chain.underlying_price,
        expirations,
        next_triple_witching: next_tw,
        note,
    })
}

pub fn create_research_tables_v56(conn: &Connection) -> Result<(), String> {
    create_research_tables_v55(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_symbol_expirations (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_symbol_expirations_updated ON research_symbol_expirations(updated_at);",
    ).map_err(|e| format!("create v56 tables: {e}"))?;
    Ok(())
}

pub fn upsert_symbol_expirations(
    conn: &Connection,
    symbol: &str,
    snap: &SymbolExpirationsSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v56(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("symexp json: {e}"))?;
    conn.execute(
        "INSERT INTO research_symbol_expirations(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert symexp: {e}"))?;
    Ok(())
}

pub fn get_symbol_expirations(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<SymbolExpirationsSnapshot>, String> {
    let _ = create_research_tables_v56(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_symbol_expirations WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_symexp: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_symexp: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_symexp: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── v89 (deferred Godel parity surfaces) ──
//    MOMRANK_MULTI / CORRSTK

pub fn create_research_tables_v89(conn: &Connection) -> Result<(), String> {
    create_research_tables_v88(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_momrank_multi (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_momrank_multi_updated ON research_momrank_multi(updated_at);

        CREATE TABLE IF NOT EXISTS research_corrstk (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_corrstk_updated ON research_corrstk(updated_at);",
    ).map_err(|e| format!("create v89 tables: {e}"))?;
    Ok(())
}

pub fn upsert_momrank_multi(
    conn: &Connection,
    symbol: &str,
    snap: &MomentumRankMultiSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v89(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("momrank_multi json: {e}"))?;
    conn.execute(
        "INSERT INTO research_momrank_multi (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert momrank_multi: {e}"))?;
    Ok(())
}

pub fn get_momrank_multi(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<MomentumRankMultiSnapshot>, String> {
    let _ = create_research_tables_v89(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_momrank_multi WHERE symbol = ?1")
        .map_err(|e| format!("prep momrank_multi: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query momrank_multi: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row momrank_multi: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get momrank_multi: {e}"))?;
        let snap: MomentumRankMultiSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse momrank_multi: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_corrstk(
    conn: &Connection,
    symbol: &str,
    snap: &CorrStkSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v89(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("corrstk json: {e}"))?;
    conn.execute(
        "INSERT INTO research_corrstk (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert corrstk: {e}"))?;
    Ok(())
}

pub fn get_corrstk(conn: &Connection, symbol: &str) -> Result<Option<CorrStkSnapshot>, String> {
    let _ = create_research_tables_v89(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_corrstk WHERE symbol = ?1")
        .map_err(|e| format!("prep corrstk: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query corrstk: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row corrstk: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get corrstk: {e}"))?;
        let snap: CorrStkSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse corrstk: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

/// Whole-table scan of `research_corrstk`. Used by CORRRANK.
pub fn get_all_corrstk(conn: &Connection) -> Result<Vec<CorrStkSnapshot>, String> {
    let _ = create_research_tables_v89(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_corrstk")
        .map_err(|e| format!("prepare get_all_corrstk: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_corrstk: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<CorrStkSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

// ── v90 (deferred peer-rank follow-through) ──
//    TLRANK / CORRRANK

pub fn create_research_tables_v90(conn: &Connection) -> Result<(), String> {
    create_research_tables_v89(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_tlrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_tlrank_updated ON research_tlrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_corrrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_corrrank_updated ON research_corrrank(updated_at);",
    )
    .map_err(|e| format!("create v90 tables: {e}"))?;
    Ok(())
}

pub fn upsert_tlrank(
    conn: &Connection,
    symbol: &str,
    snap: &ThirtyDayLiquidityRankSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v90(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("tlrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_tlrank (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    )
    .map_err(|e| format!("upsert tlrank: {e}"))?;
    Ok(())
}

pub fn get_tlrank(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<ThirtyDayLiquidityRankSnapshot>, String> {
    let _ = create_research_tables_v90(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_tlrank WHERE symbol = ?1")
        .map_err(|e| format!("prep tlrank: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query tlrank: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row tlrank: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get tlrank: {e}"))?;
        let snap: ThirtyDayLiquidityRankSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse tlrank: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_corrrank(
    conn: &Connection,
    symbol: &str,
    snap: &CorrelationRankSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v90(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("corrrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_corrrank (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    )
    .map_err(|e| format!("upsert corrrank: {e}"))?;
    Ok(())
}

pub fn get_corrrank(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<CorrelationRankSnapshot>, String> {
    let _ = create_research_tables_v90(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_corrrank WHERE symbol = ?1")
        .map_err(|e| format!("prep corrrank: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query corrrank: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row corrrank: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get corrrank: {e}"))?;
        let snap: CorrelationRankSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse corrrank: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

// ── v91 (remaining cache-backed parity) ──
//    OPERANK_DELTA / DIVACC / EPSACC / VRP

pub fn create_research_tables_v91(conn: &Connection) -> Result<(), String> {
    create_research_tables_v90(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_operank_delta (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_operank_delta_updated ON research_operank_delta(updated_at);

        CREATE TABLE IF NOT EXISTS research_divacc (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_divacc_updated ON research_divacc(updated_at);

        CREATE TABLE IF NOT EXISTS research_epsacc (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_epsacc_updated ON research_epsacc(updated_at);

        CREATE TABLE IF NOT EXISTS research_vrp (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_vrp_updated ON research_vrp(updated_at);",
    )
    .map_err(|e| format!("create v91 tables: {e}"))?;
    Ok(())
}

pub fn upsert_operank_delta(
    conn: &Connection,
    symbol: &str,
    snap: &OperatingMarginDeltaRankSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v91(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("operank_delta json: {e}"))?;
    conn.execute(
        "INSERT INTO research_operank_delta (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    )
    .map_err(|e| format!("upsert operank_delta: {e}"))?;
    Ok(())
}

pub fn get_operank_delta(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<OperatingMarginDeltaRankSnapshot>, String> {
    let _ = create_research_tables_v91(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_operank_delta WHERE symbol = ?1")
        .map_err(|e| format!("prep operank_delta: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query operank_delta: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row operank_delta: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get operank_delta: {e}"))?;
        let snap: OperatingMarginDeltaRankSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse operank_delta: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_divacc(
    conn: &Connection,
    symbol: &str,
    snap: &DividendAccelerationSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v91(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("divacc json: {e}"))?;
    conn.execute(
        "INSERT INTO research_divacc (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    )
    .map_err(|e| format!("upsert divacc: {e}"))?;
    Ok(())
}

pub fn get_divacc(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<DividendAccelerationSnapshot>, String> {
    let _ = create_research_tables_v91(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_divacc WHERE symbol = ?1")
        .map_err(|e| format!("prep divacc: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query divacc: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row divacc: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get divacc: {e}"))?;
        let snap: DividendAccelerationSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse divacc: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_epsacc(
    conn: &Connection,
    symbol: &str,
    snap: &EpsAccelerationSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v91(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("epsacc json: {e}"))?;
    conn.execute(
        "INSERT INTO research_epsacc (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    )
    .map_err(|e| format!("upsert epsacc: {e}"))?;
    Ok(())
}

pub fn get_epsacc(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<EpsAccelerationSnapshot>, String> {
    let _ = create_research_tables_v91(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_epsacc WHERE symbol = ?1")
        .map_err(|e| format!("prep epsacc: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query epsacc: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row epsacc: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get epsacc: {e}"))?;
        let snap: EpsAccelerationSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse epsacc: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_vrp(
    conn: &Connection,
    symbol: &str,
    snap: &VolRiskPremiumSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v91(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("vrp json: {e}"))?;
    conn.execute(
        "INSERT INTO research_vrp (symbol, snapshot_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    )
    .map_err(|e| format!("upsert vrp: {e}"))?;
    Ok(())
}

pub fn get_vrp(conn: &Connection, symbol: &str) -> Result<Option<VolRiskPremiumSnapshot>, String> {
    let _ = create_research_tables_v91(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_vrp WHERE symbol = ?1")
        .map_err(|e| format!("prep vrp: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query vrp: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row vrp: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get vrp: {e}"))?;
        let snap: VolRiskPremiumSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse vrp: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

// ── v92 (short-interest history + trend rank) ──
//    SHORT_INTEREST_HISTORY / SHORTRANK_DELTA

pub fn create_research_tables_v92(conn: &Connection) -> Result<(), String> {
    create_research_tables_v91(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_short_interest_history (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_short_interest_history_updated
            ON research_short_interest_history(updated_at);

        CREATE TABLE IF NOT EXISTS research_shortrank_delta (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_shortrank_delta_updated
            ON research_shortrank_delta(updated_at);",
    )
    .map_err(|e| format!("create v92 tables: {e}"))?;
    Ok(())
}

pub fn upsert_short_interest_history(
    conn: &Connection,
    symbol: &str,
    rows: &[ShortInterestHistoryPoint],
) -> Result<(), String> {
    if rows.is_empty() {
        return Ok(());
    }
    let _ = create_research_tables_v92(conn);
    let existing = get_short_interest_history(conn, symbol)
        .ok()
        .flatten()
        .unwrap_or_default();
    let merged = merge_short_interest_history_rows(&existing, rows);
    if merged.is_empty() {
        return Ok(());
    }
    let json =
        serde_json::to_string(&merged).map_err(|e| format!("short_interest_history json: {e}"))?;
    conn.execute(
        "INSERT INTO research_short_interest_history (symbol, rows_json, updated_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    )
    .map_err(|e| format!("upsert short_interest_history: {e}"))?;
    Ok(())
}

pub fn append_short_interest_history_point(
    conn: &Connection,
    symbol: &str,
    row: ShortInterestHistoryPoint,
) -> Result<(), String> {
    upsert_short_interest_history(conn, symbol, &[row])
}

pub fn get_short_interest_history(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<Vec<ShortInterestHistoryPoint>>, String> {
    let _ = create_research_tables_v92(conn);
    let mut stmt = conn
        .prepare("SELECT rows_json FROM research_short_interest_history WHERE symbol = ?1")
        .map_err(|e| format!("prep short_interest_history: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query short_interest_history: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row short_interest_history: {e}"))?
    {
        let j: String = r
            .get(0)
            .map_err(|e| format!("get short_interest_history: {e}"))?;
        let parsed: Vec<ShortInterestHistoryPoint> =
            serde_json::from_str(&j).map_err(|e| format!("parse short_interest_history: {e}"))?;
        Ok(Some(parsed))
    } else {
        Ok(None)
    }
}

pub fn upsert_shortrank_delta(
    conn: &Connection,
    symbol: &str,
    snap: &ShortInterestDeltaRankSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v92(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("shortrank_delta json: {e}"))?;
    conn.execute(
        "INSERT INTO research_shortrank_delta (symbol, snapshot_json, updated_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    )
    .map_err(|e| format!("upsert shortrank_delta: {e}"))?;
    Ok(())
}

pub fn get_shortrank_delta(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<ShortInterestDeltaRankSnapshot>, String> {
    let _ = create_research_tables_v92(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_shortrank_delta WHERE symbol = ?1")
        .map_err(|e| format!("prep shortrank_delta: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query shortrank_delta: {e}"))?;
    if let Some(r) = rows
        .next()
        .map_err(|e| format!("row shortrank_delta: {e}"))?
    {
        let j: String = r.get(0).map_err(|e| format!("get shortrank_delta: {e}"))?;
        let snap: ShortInterestDeltaRankSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse shortrank_delta: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

// ── v93 (insider ownership concentration) ──
//    INSIDERCONC

pub fn create_research_tables_v93(conn: &Connection) -> Result<(), String> {
    create_research_tables_v92(conn)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_insiderconc (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_insiderconc_updated
            ON research_insiderconc(updated_at);",
    )
    .map_err(|e| format!("create v93 tables: {e}"))?;
    Ok(())
}

pub fn upsert_insiderconc(
    conn: &Connection,
    symbol: &str,
    snap: &InsiderConcentrationSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v93(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("insiderconc json: {e}"))?;
    conn.execute(
        "INSERT INTO research_insiderconc (symbol, snapshot_json, updated_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    )
    .map_err(|e| format!("upsert insiderconc: {e}"))?;
    Ok(())
}

pub fn get_insiderconc(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<InsiderConcentrationSnapshot>, String> {
    let _ = create_research_tables_v93(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_insiderconc WHERE symbol = ?1")
        .map_err(|e| format!("prep insiderconc: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query insiderconc: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row insiderconc: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get insiderconc: {e}"))?;
        let snap: InsiderConcentrationSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse insiderconc: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_mass_index(
    conn: &Connection,
    symbol: &str,
    snap: &MassIndexSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v64(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("mass_index json: {e}"))?;
    conn.execute(
        "INSERT INTO research_mass_index(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert mass_index: {e}"))?;
    Ok(())
}

pub fn get_mass_index(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<MassIndexSnapshot>, String> {
    let _ = create_research_tables_v64(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_mass_index WHERE symbol = ?1")
        .map_err(|e| format!("prep mass_index: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query mass_index: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row mass_index: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get mass_index: {e}"))?;
        let snap: MassIndexSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse mass_index: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_natr(conn: &Connection, symbol: &str, snap: &NatrSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v64(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("natr json: {e}"))?;
    conn.execute(
        "INSERT INTO research_natr(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert natr: {e}"))?;
    Ok(())
}

pub fn get_natr(conn: &Connection, symbol: &str) -> Result<Option<NatrSnapshot>, String> {
    let _ = create_research_tables_v64(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_natr WHERE symbol = ?1")
        .map_err(|e| format!("prep natr: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query natr: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row natr: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get natr: {e}"))?;
        let snap: NatrSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse natr: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_ttm_squeeze(
    conn: &Connection,
    symbol: &str,
    snap: &TtmSqueezeSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v64(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ttm_squeeze json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ttm_squeeze(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ttm_squeeze: {e}"))?;
    Ok(())
}

pub fn get_ttm_squeeze(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<TtmSqueezeSnapshot>, String> {
    let _ = create_research_tables_v64(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_ttm_squeeze WHERE symbol = ?1")
        .map_err(|e| format!("prep ttm_squeeze: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query ttm_squeeze: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row ttm_squeeze: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get ttm_squeeze: {e}"))?;
        let snap: TtmSqueezeSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse ttm_squeeze: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_force_index(
    conn: &Connection,
    symbol: &str,
    snap: &ForceIndexSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v64(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("force_index json: {e}"))?;
    conn.execute(
        "INSERT INTO research_force_index(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert force_index: {e}"))?;
    Ok(())
}

pub fn get_force_index(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<ForceIndexSnapshot>, String> {
    let _ = create_research_tables_v64(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_force_index WHERE symbol = ?1")
        .map_err(|e| format!("prep force_index: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query force_index: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row force_index: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get force_index: {e}"))?;
        let snap: ForceIndexSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse force_index: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_trange(conn: &Connection, symbol: &str, snap: &TrangeSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v64(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("trange json: {e}"))?;
    conn.execute(
        "INSERT INTO research_trange(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert trange: {e}"))?;
    Ok(())
}

pub fn get_trange(conn: &Connection, symbol: &str) -> Result<Option<TrangeSnapshot>, String> {
    let _ = create_research_tables_v64(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_trange WHERE symbol = ?1")
        .map_err(|e| format!("prep trange: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query trange: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row trange: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get trange: {e}"))?;
        let snap: TrangeSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse trange: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_laguerre_rsi(
    conn: &Connection,
    symbol: &str,
    snap: &LaguerreRsiSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v63(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("laguerre_rsi json: {e}"))?;
    conn.execute(
        "INSERT INTO research_laguerre_rsi(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert laguerre_rsi: {e}"))?;
    Ok(())
}

pub fn get_laguerre_rsi(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<LaguerreRsiSnapshot>, String> {
    let _ = create_research_tables_v63(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_laguerre_rsi WHERE symbol = ?1")
        .map_err(|e| format!("prep laguerre_rsi: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query laguerre_rsi: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row laguerre_rsi: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get laguerre_rsi: {e}"))?;
        let snap: LaguerreRsiSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse laguerre_rsi: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_zigzag(conn: &Connection, symbol: &str, snap: &ZigzagSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v63(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("zigzag json: {e}"))?;
    conn.execute(
        "INSERT INTO research_zigzag(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert zigzag: {e}"))?;
    Ok(())
}

pub fn get_zigzag(conn: &Connection, symbol: &str) -> Result<Option<ZigzagSnapshot>, String> {
    let _ = create_research_tables_v63(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_zigzag WHERE symbol = ?1")
        .map_err(|e| format!("prep zigzag: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query zigzag: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row zigzag: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get zigzag: {e}"))?;
        let snap: ZigzagSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse zigzag: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_pgo(conn: &Connection, symbol: &str, snap: &PgoSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v63(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("pgo json: {e}"))?;
    conn.execute(
        "INSERT INTO research_pgo(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert pgo: {e}"))?;
    Ok(())
}

pub fn get_pgo(conn: &Connection, symbol: &str) -> Result<Option<PgoSnapshot>, String> {
    let _ = create_research_tables_v63(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_pgo WHERE symbol = ?1")
        .map_err(|e| format!("prep pgo: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query pgo: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row pgo: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get pgo: {e}"))?;
        let snap: PgoSnapshot = serde_json::from_str(&j).map_err(|e| format!("parse pgo: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_ht_trendline(
    conn: &Connection,
    symbol: &str,
    snap: &HtTrendlineSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v63(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ht_trendline json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ht_trendline(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ht_trendline: {e}"))?;
    Ok(())
}

pub fn get_ht_trendline(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<HtTrendlineSnapshot>, String> {
    let _ = create_research_tables_v63(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_ht_trendline WHERE symbol = ?1")
        .map_err(|e| format!("prep ht_trendline: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query ht_trendline: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row ht_trendline: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get ht_trendline: {e}"))?;
        let snap: HtTrendlineSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse ht_trendline: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_midpoint(
    conn: &Connection,
    symbol: &str,
    snap: &MidpointSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v63(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("midpoint json: {e}"))?;
    conn.execute(
        "INSERT INTO research_midpoint(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert midpoint: {e}"))?;
    Ok(())
}

pub fn get_midpoint(conn: &Connection, symbol: &str) -> Result<Option<MidpointSnapshot>, String> {
    let _ = create_research_tables_v63(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_midpoint WHERE symbol = ?1")
        .map_err(|e| format!("prep midpoint: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query midpoint: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row midpoint: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get midpoint: {e}"))?;
        let snap: MidpointSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse midpoint: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_wma(conn: &Connection, symbol: &str, snap: &WmaSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v62(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("wma json: {e}"))?;
    conn.execute(
        "INSERT INTO research_wma(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert wma: {e}"))?;
    Ok(())
}

pub fn get_wma(conn: &Connection, symbol: &str) -> Result<Option<WmaSnapshot>, String> {
    let _ = create_research_tables_v62(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_wma WHERE symbol = ?1")
        .map_err(|e| format!("prep wma: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query wma: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row wma: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get wma: {e}"))?;
        let snap: WmaSnapshot = serde_json::from_str(&j).map_err(|e| format!("parse wma: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_rainbow(
    conn: &Connection,
    symbol: &str,
    snap: &RainbowSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v62(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("rainbow json: {e}"))?;
    conn.execute(
        "INSERT INTO research_rainbow(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rainbow: {e}"))?;
    Ok(())
}

pub fn get_rainbow(conn: &Connection, symbol: &str) -> Result<Option<RainbowSnapshot>, String> {
    let _ = create_research_tables_v62(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_rainbow WHERE symbol = ?1")
        .map_err(|e| format!("prep rainbow: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query rainbow: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row rainbow: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get rainbow: {e}"))?;
        let snap: RainbowSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse rainbow: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_mesa_sine(
    conn: &Connection,
    symbol: &str,
    snap: &MesaSineSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v62(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("mesa_sine json: {e}"))?;
    conn.execute(
        "INSERT INTO research_mesa_sine(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert mesa_sine: {e}"))?;
    Ok(())
}

pub fn get_mesa_sine(conn: &Connection, symbol: &str) -> Result<Option<MesaSineSnapshot>, String> {
    let _ = create_research_tables_v62(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_mesa_sine WHERE symbol = ?1")
        .map_err(|e| format!("prep mesa_sine: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query mesa_sine: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row mesa_sine: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get mesa_sine: {e}"))?;
        let snap: MesaSineSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse mesa_sine: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_frama(conn: &Connection, symbol: &str, snap: &FramaSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v62(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("frama json: {e}"))?;
    conn.execute(
        "INSERT INTO research_frama(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert frama: {e}"))?;
    Ok(())
}

pub fn get_frama(conn: &Connection, symbol: &str) -> Result<Option<FramaSnapshot>, String> {
    let _ = create_research_tables_v62(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_frama WHERE symbol = ?1")
        .map_err(|e| format!("prep frama: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query frama: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row frama: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get frama: {e}"))?;
        let snap: FramaSnapshot =
            serde_json::from_str(&j).map_err(|e| format!("parse frama: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_ibs(conn: &Connection, symbol: &str, snap: &IbsSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v62(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ibs json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ibs(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ibs: {e}"))?;
    Ok(())
}

pub fn get_ibs(conn: &Connection, symbol: &str) -> Result<Option<IbsSnapshot>, String> {
    let _ = create_research_tables_v62(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_ibs WHERE symbol = ?1")
        .map_err(|e| format!("prep ibs: {e}"))?;
    let mut rows = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query ibs: {e}"))?;
    if let Some(r) = rows.next().map_err(|e| format!("row ibs: {e}"))? {
        let j: String = r.get(0).map_err(|e| format!("get ibs: {e}"))?;
        let snap: IbsSnapshot = serde_json::from_str(&j).map_err(|e| format!("parse ibs: {e}"))?;
        Ok(Some(snap))
    } else {
        Ok(None)
    }
}

pub fn upsert_demarker(
    conn: &Connection,
    symbol: &str,
    snap: &DemarkerSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v61(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("demarker json: {e}"))?;
    conn.execute(
        "INSERT INTO research_demarker(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert demarker: {e}"))?;
    Ok(())
}

pub fn get_demarker(conn: &Connection, symbol: &str) -> Result<Option<DemarkerSnapshot>, String> {
    let _ = create_research_tables_v61(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_demarker WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_demarker: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_demarker: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_demarker: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_gator(conn: &Connection, symbol: &str, snap: &GatorSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v61(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("gator json: {e}"))?;
    conn.execute(
        "INSERT INTO research_gator(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert gator: {e}"))?;
    Ok(())
}

pub fn get_gator(conn: &Connection, symbol: &str) -> Result<Option<GatorSnapshot>, String> {
    let _ = create_research_tables_v61(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_gator WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_gator: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_gator: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_gator: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_bw_mfi(conn: &Connection, symbol: &str, snap: &BwMfiSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v61(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("bw_mfi json: {e}"))?;
    conn.execute(
        "INSERT INTO research_bw_mfi(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert bw_mfi: {e}"))?;
    Ok(())
}

pub fn get_bw_mfi(conn: &Connection, symbol: &str) -> Result<Option<BwMfiSnapshot>, String> {
    let _ = create_research_tables_v61(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_bw_mfi WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_bw_mfi: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_bw_mfi: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_bw_mfi: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_vwma(conn: &Connection, symbol: &str, snap: &VwmaSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v61(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("vwma json: {e}"))?;
    conn.execute(
        "INSERT INTO research_vwma(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert vwma: {e}"))?;
    Ok(())
}

pub fn get_vwma(conn: &Connection, symbol: &str) -> Result<Option<VwmaSnapshot>, String> {
    let _ = create_research_tables_v61(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_vwma WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_vwma: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_vwma: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_vwma: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_stddev(conn: &Connection, symbol: &str, snap: &StddevSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v61(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("stddev json: {e}"))?;
    conn.execute(
        "INSERT INTO research_stddev(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert stddev: {e}"))?;
    Ok(())
}

pub fn get_stddev(conn: &Connection, symbol: &str) -> Result<Option<StddevSnapshot>, String> {
    let _ = create_research_tables_v61(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_stddev WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_stddev: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_stddev: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_stddev: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;

mod fundamental_quality_types;
pub use fundamental_quality_types::*;
mod insider_dividend_momentum_types;
pub use insider_dividend_momentum_types::*;
mod market_liquidity_credit_types;
pub use market_liquidity_credit_types::*;
mod growth_flow_regime_types;
pub use growth_flow_regime_types::*;
mod factor_signal_types;
pub use factor_signal_types::*;
mod relative_rank_event_types;
pub use relative_rank_event_types::*;
mod rank_overlay_types;
pub use rank_overlay_types::*;
mod return_risk_distribution_types;
pub use return_risk_distribution_types::*;
mod autocorr_drawup_volatility_types;
pub use autocorr_drawup_volatility_types::*;
mod quant_risk_stat_types;
pub use quant_risk_stat_types::*;
mod fractal_jump_stationarity_types;
pub use fractal_jump_stationarity_types::*;
mod stat_indicator_types;
pub use stat_indicator_types::*;
mod oscillator_moving_average_types;
pub use oscillator_moving_average_types::*;
mod price_transform_indicator_types;
pub use price_transform_indicator_types::*;
mod candlestick_pattern_types;
pub use candlestick_pattern_types::*;
mod quant_stat_surface_types;
pub use quant_stat_surface_types::*;
