use rusqlite::Connection;

use super::*;

// ── bulk scrape helper (used by fundamentals scrape loop) ──────────────────

/// Fetch and cache all research data for a single symbol, respecting rate limits.
/// Returns Ok(()) even if individual endpoints fail — errors are logged via cb.
pub async fn scrape_and_cache_symbol(
    client: &reqwest::Client,
    conn: &Connection,
    symbol: &str,
    finnhub_key: &str,
    fmp_key: &str,
    mut cb: impl FnMut(&str),
) -> Result<(), String> {
    let sym = symbol.to_uppercase();
    if sym.is_empty() {
        return Err("empty symbol".into());
    }

    // Profile
    if !finnhub_key.is_empty() {
        match fetch_finnhub_profile(client, &sym, finnhub_key).await {
            Ok(p) => {
                if !p.name.is_empty() {
                    let _ = upsert_profile(conn, &p);
                    cb(&format!("research/profile: {} cached", sym));
                }
            }
            Err(e) => cb(&format!("research/profile {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        // Peers
        match fetch_finnhub_peers(client, &sym, finnhub_key).await {
            Ok(peers) => {
                if !peers.is_empty() {
                    let _ = upsert_peers(conn, &sym, &peers);
                }
            }
            Err(e) => cb(&format!("research/peers {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        // Earnings
        match fetch_finnhub_earnings(client, &sym, finnhub_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_earnings_history(conn, &sym, &rows);
                }
            }
            Err(e) => cb(&format!("research/earnings {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        // Press releases
        match fetch_finnhub_press(client, &sym, finnhub_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_press_releases(conn, &sym, &rows);
                }
            }
            Err(e) => cb(&format!("research/press {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        // Social sentiment
        match fetch_finnhub_social(client, &sym, finnhub_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_sentiment(conn, &sym, &rows);
                }
            }
            Err(e) => cb(&format!("research/sentiment {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    }

    // Transcripts (FMP)
    if !fmp_key.is_empty() {
        match fetch_fmp_transcript_list(client, &sym, fmp_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_transcript_list(conn, &sym, &rows);
                }
            }
            Err(e) => cb(&format!("research/transcripts {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // dividend history (FMP)
        match fetch_fmp_dividend_history(client, &sym, fmp_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_dividends(conn, &sym, &rows);
                }
            }
            Err(e) => cb(&format!("research/dividends {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // forward earnings estimates (FMP)
        match fetch_fmp_earnings_estimates(client, &sym, fmp_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_earnings_estimates(conn, &sym, &rows);
                }
            }
            Err(e) => cb(&format!("research/estimates {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // analyst rating changes (FMP)
        match fetch_fmp_rating_changes(client, &sym, fmp_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_rating_changes(conn, &sym, &rows);
                }
            }
            Err(e) => cb(&format!("research/ratings {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // full FA bundle (6 FMP calls, internal 400ms sleeps).
        match fetch_fmp_financial_bundle(client, &sym, fmp_key).await {
            Ok(bundle) => {
                let any = !bundle.income_annual.is_empty()
                    || !bundle.income_quarterly.is_empty()
                    || !bundle.balance_annual.is_empty()
                    || !bundle.balance_quarterly.is_empty()
                    || !bundle.cashflow_annual.is_empty()
                    || !bundle.cashflow_quarterly.is_empty();
                if any {
                    let _ = upsert_financials(conn, &sym, &bundle);
                    cb(&format!("research/financials: {} cached", sym));
                }
            }
            Err(e) => cb(&format!("research/financials {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // ETF holdings (FMP). No-op for non-ETF tickers (empty result).
        match fetch_fmp_etf_holdings(client, &sym, fmp_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_etf_holdings(conn, &sym, &rows);
                    cb(&format!(
                        "research/etf: {} cached ({} holdings)",
                        sym,
                        rows.len()
                    ));
                }
            }
            Err(e) => cb(&format!("research/etf {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // ESG scores (FMP).
        match fetch_fmp_esg(client, &sym, fmp_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_esg(conn, &sym, &rows);
                    cb(&format!(
                        "research/esg: {} cached ({} years)",
                        sym,
                        rows.len()
                    ));
                }
            }
            Err(e) => cb(&format!("research/esg {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    }

    // Stock split history — deliberately outside the FMP gate. The combined
    // fetcher uses FMP when a key exists and always tries Yahoo's keyless
    // chart-events feed, so `research_stock_splits` is populated generally and
    // the exact back-adjust path (ADR-122) is fed for every scraped symbol,
    // not just curated ones (ADR-113/123 follow-up).
    match fetch_stock_splits(client, &sym, fmp_key).await {
        Ok(rows) => {
            if !rows.is_empty() {
                let _ = upsert_stock_splits(conn, &sym, &rows);
                cb(&format!(
                    "research/splits: {} cached ({} rows)",
                    sym,
                    rows.len()
                ));
            }
        }
        Err(e) => cb(&format!("research/splits {} failed: {}", sym, e)),
    }
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;

    // Finnhub executives (separate from FMP block; needs Finnhub key).
    if !finnhub_key.is_empty() {
        match fetch_finnhub_executives(client, &sym, finnhub_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_executives(conn, &sym, &rows);
                    cb(&format!(
                        "research/executives: {} cached ({} rows)",
                        sym,
                        rows.len()
                    ));
                }
            }
            Err(e) => cb(&format!("research/executives {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        // analyst recommendation trends (Finnhub).
        match fetch_finnhub_recommendations(client, &sym, finnhub_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_analyst_recs(conn, &sym, &rows);
                    cb(&format!(
                        "research/recs: {} cached ({} rows)",
                        sym,
                        rows.len()
                    ));
                }
            }
            Err(e) => cb(&format!("research/recs {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        // consensus price target (Finnhub).
        match fetch_finnhub_price_target(client, &sym, finnhub_key).await {
            Ok(pt) => {
                if pt.num_analysts > 0 || pt.target_mean > 0.0 {
                    let _ = upsert_price_target(conn, &sym, &pt);
                    cb(&format!(
                        "research/target: {} cached (n={})",
                        sym, pt.num_analysts
                    ));
                }
            }
            Err(e) => cb(&format!("research/target {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    }

    Ok(())
}
