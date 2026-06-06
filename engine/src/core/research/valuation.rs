//! Research valuation and option-implied snapshot computations.

use super::*;

pub fn compute_wacc_snapshot(
    symbol: &str,
    as_of: &str,
    beta: f64,
    market_cap: f64,
    risk_free_pct: f64,
    total_debt: f64,
    interest_expense: f64,
    effective_tax_rate_pct: f64,
) -> WaccSnapshot {
    let erp = DEFAULT_EQUITY_RISK_PREMIUM_PCT;
    let cost_of_equity_pct = risk_free_pct + beta * erp;

    let pre_tax_cost_of_debt_pct = if total_debt.abs() > 1e-6 {
        (interest_expense.abs() / total_debt) * 100.0
    } else {
        0.0
    };

    let tax_rate_pct = effective_tax_rate_pct.clamp(0.0, 60.0);
    let after_tax_cost_of_debt_pct = pre_tax_cost_of_debt_pct * (1.0 - tax_rate_pct / 100.0);

    let total_cap = market_cap + total_debt;
    let equity_weight = if total_cap > 1e-6 {
        market_cap / total_cap
    } else {
        1.0
    };
    let debt_weight = 1.0 - equity_weight;
    let wacc_pct = equity_weight * cost_of_equity_pct + debt_weight * after_tax_cost_of_debt_pct;

    WaccSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        beta,
        risk_free_pct,
        equity_risk_premium_pct: erp,
        cost_of_equity_pct,
        pre_tax_cost_of_debt_pct,
        tax_rate_pct,
        after_tax_cost_of_debt_pct,
        market_cap,
        total_debt,
        equity_weight,
        debt_weight,
        wacc_pct,
    }
}

// ── Round 7 — WCR fetcher ─────────────────────────────────────────

/// Fetch the hardcoded FX-majors universe through Yahoo and return the rows
/// in the order declared by `FX_MAJORS_UNIVERSE`.
pub async fn fetch_currency_rates(client: &reqwest::Client) -> Result<Vec<CurrencyRate>, String> {
    let tickers: Vec<&str> = FX_MAJORS_UNIVERSE
        .iter()
        .map(|(t, _, _, _, _)| *t)
        .collect();
    let quotes = fetch_yahoo_quotes(client, &tickers).await?;

    use std::collections::HashMap;
    let by_ticker: HashMap<String, (f64, f64, f64)> = quotes
        .into_iter()
        .map(|(t, p, c, pct)| (t, (p, c, pct)))
        .collect();

    let mut out = Vec::with_capacity(FX_MAJORS_UNIVERSE.len());
    for (tk, display, base, quote, region) in FX_MAJORS_UNIVERSE.iter() {
        let (price, change, change_pct) = by_ticker.get(*tk).copied().unwrap_or((0.0, 0.0, 0.0));
        out.push(CurrencyRate {
            ticker: (*tk).to_string(),
            display: (*display).to_string(),
            base: (*base).to_string(),
            quote: (*quote).to_string(),
            region: (*region).to_string(),
            price,
            change,
            change_pct,
        });
    }
    Ok(out)
}

// ── Round 7 — BETA compute ────────────────────────────────────────

/// Compute an OLS regression of symbol log-returns on market log-returns.
/// Returns (beta, alpha_per_period, r_squared, correlation, n).
/// Pure function, no I/O. Daily returns expected; alpha is per-period
/// (caller annualizes as needed).
fn ols_regression(symbol_returns: &[f64], market_returns: &[f64]) -> (f64, f64, f64, f64, usize) {
    let n = symbol_returns.len().min(market_returns.len());
    if n < 10 {
        return (0.0, 0.0, 0.0, 0.0, n);
    }
    let mean_s: f64 = symbol_returns.iter().take(n).sum::<f64>() / n as f64;
    let mean_m: f64 = market_returns.iter().take(n).sum::<f64>() / n as f64;

    let mut cov = 0.0_f64;
    let mut var_m = 0.0_f64;
    let mut var_s = 0.0_f64;
    for i in 0..n {
        let ds = symbol_returns[i] - mean_s;
        let dm = market_returns[i] - mean_m;
        cov += ds * dm;
        var_m += dm * dm;
        var_s += ds * ds;
    }
    if var_m <= 1e-12 {
        return (0.0, 0.0, 0.0, 0.0, n);
    }
    let beta = cov / var_m;
    let alpha = mean_s - beta * mean_m;

    // R² (symbol variance explained by market) = β² · var_m / var_s
    let r_squared = if var_s > 1e-12 {
        (beta * beta) * var_m / var_s
    } else {
        0.0
    };
    let correlation = if var_m > 1e-12 && var_s > 1e-12 {
        cov / (var_m.sqrt() * var_s.sqrt())
    } else {
        0.0
    };

    (beta, alpha, r_squared.clamp(0.0, 1.0), correlation, n)
}

/// Compute log-returns from a sequence of closes (newest-first or oldest-first
/// both work — the function only cares about adjacent differences). Result is
/// in the same order as the input (length = len - 1).
fn log_returns(closes: &[f64]) -> Vec<f64> {
    if closes.len() < 2 {
        return Vec::new();
    }
    closes
        .windows(2)
        .map(|w| {
            if w[0] > 0.0 && w[1] > 0.0 {
                (w[1] / w[0]).ln()
            } else {
                0.0
            }
        })
        .collect()
}

/// Compute a per-symbol beta snapshot against a market benchmark using
/// cached FMP historical price rows for both series. Caller fetches the bars
/// once (or reuses the HP cache) and hands them in. The bars must be sorted
/// **newest-first** (FMP returns them that way by default).
///
/// We compute 1Y / 3Y / 5Y windows using the trailing N trading days.
/// Windows that don't have enough overlapping data are skipped silently.
pub fn compute_beta_snapshot(
    symbol: &str,
    market_ticker: &str,
    as_of: &str,
    sym_bars_newest_first: &[HistoricalPriceRow],
    mkt_bars_newest_first: &[HistoricalPriceRow],
) -> BetaSnapshot {
    use std::collections::HashMap;
    // Intersect by date to make returns directly comparable.
    let mkt_by_date: HashMap<&str, f64> = mkt_bars_newest_first
        .iter()
        .map(|b| (b.date.as_str(), b.close))
        .collect();
    let mut paired: Vec<(String, f64, f64)> = sym_bars_newest_first
        .iter()
        .filter_map(|b| {
            mkt_by_date
                .get(b.date.as_str())
                .map(|m| (b.date.clone(), b.close, *m))
        })
        .collect();
    // Sort ascending by date so the log_returns helper produces chronological returns.
    paired.sort_by(|a, b| a.0.cmp(&b.0));

    let sym_closes: Vec<f64> = paired.iter().map(|(_, s, _)| *s).collect();
    let mkt_closes: Vec<f64> = paired.iter().map(|(_, _, m)| *m).collect();
    let sym_rets = log_returns(&sym_closes);
    let mkt_rets = log_returns(&mkt_closes);

    let mut windows = Vec::new();
    let mut note = String::new();

    for (label, days) in [("1Y", 252usize), ("3Y", 756), ("5Y", 1260)] {
        let n_available = sym_rets.len().min(mkt_rets.len());
        if n_available == 0 {
            continue;
        }
        // Use the most recent `days` returns (tail slice) — sym_rets/mkt_rets
        // are ordered chronologically (oldest first, newest last).
        let take = days.min(n_available);
        let s_slice = &sym_rets[n_available - take..];
        let m_slice = &mkt_rets[n_available - take..];
        let (beta, alpha, r2, corr, n_obs) = ols_regression(s_slice, m_slice);
        if n_obs < 20 {
            if note.is_empty() && label == "1Y" {
                note = format!("insufficient overlapping data (n={n_obs}) for stable beta");
            }
            continue;
        }
        windows.push(BetaWindow {
            window_label: label.to_string(),
            window_days: days,
            beta,
            alpha_pct: alpha * 252.0 * 100.0, // annualize daily alpha
            r_squared: r2,
            n_observations: n_obs,
            correlation: corr,
        });
    }

    BetaSnapshot {
        symbol: symbol.to_uppercase(),
        market_ticker: market_ticker.to_string(),
        as_of: as_of.to_string(),
        windows,
        note,
    }
}

fn hp_adj_close_or_close(bar: &HistoricalPriceRow) -> f64 {
    if bar.adj_close > 0.0 {
        bar.adj_close
    } else {
        bar.close
    }
}

/// Map a fundamentals-sector string to the canonical SPDR sector ETF used as
/// a liquid benchmark proxy. Matching is intentionally fuzzy because the
/// upstream sector labels vary between "Financials" / "Financial Services",
/// "Consumer Defensive" / "Consumer Staples", etc.
pub fn sector_to_benchmark_etf(sector: &str) -> Option<&'static str> {
    let s = sector.trim().to_lowercase();
    if s.is_empty() {
        None
    } else if s.contains("technology") {
        Some("XLK")
    } else if s.contains("financial") {
        Some("XLF")
    } else if s.contains("health") {
        Some("XLV")
    } else if s.contains("energy") {
        Some("XLE")
    } else if s.contains("industrial") {
        Some("XLI")
    } else if s.contains("utility") {
        Some("XLU")
    } else if s.contains("real estate") {
        Some("XLRE")
    } else if s.contains("communication") {
        Some("XLC")
    } else if s.contains("defensive") || s.contains("staples") {
        Some("XLP")
    } else if s.contains("cyclical") || s.contains("discretionary") {
        Some("XLY")
    } else if s.contains("material") {
        Some("XLB")
    } else {
        None
    }
}

pub(crate) fn aligned_log_return_pairs(
    subject_bars: &[HistoricalPriceRow],
    benchmark_bars: &[HistoricalPriceRow],
) -> Vec<(String, f64, f64)> {
    use std::collections::HashMap;

    let mut benchmark_by_date: HashMap<&str, f64> = HashMap::new();
    for row in benchmark_bars {
        let px = hp_adj_close_or_close(row);
        if px > 0.0 {
            benchmark_by_date.insert(row.date.as_str(), px);
        }
    }

    let mut paired: Vec<(String, f64, f64)> = subject_bars
        .iter()
        .filter_map(|row| {
            let px = hp_adj_close_or_close(row);
            if px <= 0.0 {
                return None;
            }
            benchmark_by_date
                .get(row.date.as_str())
                .copied()
                .map(|bpx| (row.date.clone(), px, bpx))
        })
        .collect();
    paired.sort_by(|a, b| a.0.cmp(&b.0));

    let mut out = Vec::with_capacity(paired.len().saturating_sub(1));
    for w in paired.windows(2) {
        let (date, s0, b0) = (&w[1].0, w[0].1, w[0].2);
        let (s1, b1) = (w[1].1, w[1].2);
        if s0 > 0.0 && s1 > 0.0 && b0 > 0.0 && b1 > 0.0 {
            out.push((date.clone(), (s1 / s0).ln(), (b1 / b0).ln()));
        }
    }
    out
}

pub(crate) fn rolling_corr_stats(
    aligned_returns: &[(String, f64, f64)],
    window_days: usize,
) -> (f64, f64, f64, usize) {
    let n = aligned_returns.len();
    if n == 0 {
        return (0.0, 0.0, 0.0, 0);
    }
    let take = window_days.min(n);
    let slice = &aligned_returns[n - take..];
    let subject: Vec<f64> = slice.iter().map(|(_, s, _)| *s).collect();
    let benchmark: Vec<f64> = slice.iter().map(|(_, _, b)| *b).collect();
    let (beta, _, r_squared, corr, used) = ols_regression(&subject, &benchmark);
    (corr, beta, r_squared, used)
}

// ── Round 7 — DDM compute ─────────────────────────────────────────

/// Compute a Gordon Growth dividend-discount-model snapshot from cached
/// dividend history and a required return (typically WACC or cost of equity).
///
/// Dividends are newest-first (matching `get_dividends`). Growth rate is
/// inferred from the 5-year dividend CAGR when at least 5 annual dividends
/// are available, with fallback to a clamped 3% assumption. If r ≤ g, the
/// Gordon formula degenerates — we return implied_price = 0.0 with a note.
pub fn compute_ddm_snapshot(
    symbol: &str,
    as_of: &str,
    dividends_newest_first: &[DividendRecord],
    required_return_pct: f64,
    return_source: &str,
) -> DdmSnapshot {
    if dividends_newest_first.is_empty() {
        return DdmSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            method: "Gordon Growth".to_string(),
            note: "no dividend history on file".to_string(),
            ..Default::default()
        };
    }

    // Trailing 4-quarter dividend ($ per share). We use adjusted_amount
    // so split adjustments don't distort the growth rate.
    let div_amount = |d: &DividendRecord| -> f64 {
        if d.adjusted_amount > 0.0 {
            d.adjusted_amount
        } else {
            d.amount
        }
    };
    let annual_dividend: f64 = dividends_newest_first.iter().take(4).map(div_amount).sum();

    // Infer growth: bucket dividends by ex-date year, then CAGR over 5 years
    // if possible. Each bucket sums the quarterly payments for that year.
    use std::collections::BTreeMap;
    let mut by_year: BTreeMap<i32, f64> = BTreeMap::new();
    for d in dividends_newest_first.iter() {
        // ex_date like "2025-10-31" — parse the 4-digit prefix.
        if let Some(year_str) = d.ex_date.get(..4) {
            if let Ok(y) = year_str.parse::<i32>() {
                *by_year.entry(y).or_insert(0.0) += div_amount(d);
            }
        }
    }
    let years_sorted: Vec<(i32, f64)> = by_year.into_iter().collect();
    let (implied_growth_pct, growth_source) = if years_sorted.len() >= 6 {
        // Use 5-year CAGR: years_sorted.last() vs years_sorted[len-6]
        let end = years_sorted[years_sorted.len() - 2].1; // second-to-last (last might be partial)
        let start_idx = years_sorted.len().saturating_sub(7);
        let start = years_sorted[start_idx].1;
        if start > 1e-9 && end > 1e-9 {
            let n_years = (years_sorted.len() - 2 - start_idx) as f64;
            let cagr = (end / start).powf(1.0 / n_years.max(1.0)) - 1.0;
            (
                cagr.clamp(-0.20, 0.20) * 100.0,
                format!("{:.0}Y dividend CAGR", n_years),
            )
        } else {
            (3.0, "fallback (insufficient history)".to_string())
        }
    } else if years_sorted.len() >= 3 {
        // Short history: compare oldest full year to newest full year.
        let end = years_sorted[years_sorted.len() - 2].1;
        let start = years_sorted[0].1;
        if start > 1e-9 && end > 1e-9 {
            let n_years = (years_sorted.len() - 2) as f64;
            let cagr = (end / start).powf(1.0 / n_years.max(1.0)) - 1.0;
            (
                cagr.clamp(-0.20, 0.20) * 100.0,
                format!("{:.0}Y dividend CAGR", n_years),
            )
        } else {
            (3.0, "fallback (short history)".to_string())
        }
    } else {
        (3.0, "fallback (no growth history)".to_string())
    };

    // Gordon Growth: P = D1 / (r - g), where D1 = D0 * (1 + g).
    let g = implied_growth_pct / 100.0;
    let r = required_return_pct / 100.0;
    let (implied_price, note) = if r > g + 0.005 && annual_dividend > 0.0 {
        let d1 = annual_dividend * (1.0 + g);
        (d1 / (r - g), String::new())
    } else if annual_dividend <= 0.0 {
        (
            0.0,
            "annual dividend is zero — Gordon Growth not applicable".to_string(),
        )
    } else {
        (
            0.0,
            format!(
                "required return {:.2}% ≤ growth {:.2}% — Gordon formula diverges",
                required_return_pct, implied_growth_pct
            ),
        )
    };

    DdmSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        annual_dividend,
        implied_growth_pct,
        required_return_pct,
        growth_source,
        return_source: return_source.to_string(),
        implied_price,
        method: "Gordon Growth".to_string(),
        note,
    }
}

// ── Round 7 — RV compute (relative valuation peer matrix) ─────────

/// One input row for the relative-valuation calculator: a metric name plus
/// the subject's value and a list of peer values. Caller builds this from
/// cached fundamentals; the function is pure.
pub struct RvMetricInput<'a> {
    pub metric: &'a str,
    pub value: Option<f64>,
    pub peer_values: Vec<f64>,
}

/// Compute a `RelativeValuation` snapshot from a list of metric inputs.
/// Skips metrics where the subject has no value or the peer set has fewer
/// than 3 observations (same threshold the packet's sector-peer block uses).
pub fn compute_relative_valuation(
    symbol: &str,
    sector: &str,
    as_of: &str,
    metrics: &[RvMetricInput<'_>],
) -> RelativeValuation {
    let mut rows = Vec::new();
    let mut max_peer_count = 0;

    for m in metrics {
        let val = match m.value {
            Some(v) if v.is_finite() => v,
            _ => continue,
        };
        let mut peers: Vec<f64> = m
            .peer_values
            .iter()
            .copied()
            .filter(|x| x.is_finite())
            .collect();
        if peers.len() < 3 {
            continue;
        }
        peers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = peers.len();
        max_peer_count = max_peer_count.max(n);

        let median = peers[n / 2];
        let low = peers[0];
        let high = peers[n - 1];
        let mean = peers.iter().sum::<f64>() / n as f64;
        let variance = peers.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        let stdev = variance.sqrt();
        let z_score = if stdev > 1e-9 {
            (val - mean) / stdev
        } else {
            0.0
        };
        let below = peers.iter().filter(|p| **p < val).count();
        let percentile = (below as f64 / n as f64) * 100.0;

        rows.push(RvMetricRow {
            metric: m.metric.to_string(),
            value: val,
            peer_median: median,
            peer_low: low,
            peer_high: high,
            z_score,
            percentile,
        });
    }

    RelativeValuation {
        symbol: symbol.to_uppercase(),
        sector: sector.to_string(),
        as_of: as_of.to_string(),
        peer_count: max_peer_count,
        rows,
    }
}

// ── Round 7 — FIGI (OpenFIGI) fetcher ─────────────────────────────

/// Fetch OpenFIGI identifiers for a symbol. OpenFIGI is a free service run by
/// Bloomberg — no API key required for reasonable volumes. We POST the
/// ticker as an exchange-code lookup against US common-stock space.
pub async fn fetch_openfigi_identifiers(
    client: &reqwest::Client,
    symbol: &str,
) -> Result<Vec<FigiIdentifier>, String> {
    let body = serde_json::json!([{
        "idType": "TICKER",
        "idValue": symbol.to_uppercase(),
        "marketSecDes": "Equity"
    }]);
    let resp = client
        .post("https://api.openfigi.com/v3/mapping")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("OpenFIGI request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("OpenFIGI: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("OpenFIGI parse: {e}"))?;
    let outer = v
        .as_array()
        .ok_or_else(|| "OpenFIGI: expected array".to_string())?;
    let mut out = Vec::new();
    for entry in outer {
        if let Some(data) = entry.get("data").and_then(|d| d.as_array()) {
            for row in data {
                out.push(FigiIdentifier {
                    figi: row["figi"].as_str().unwrap_or("").to_string(),
                    name: row["name"].as_str().unwrap_or("").to_string(),
                    ticker: row["ticker"].as_str().unwrap_or("").to_string(),
                    exch_code: row["exchCode"].as_str().unwrap_or("").to_string(),
                    composite_figi: row["compositeFIGI"].as_str().unwrap_or("").to_string(),
                    share_class_figi: row["shareClassFIGI"].as_str().unwrap_or("").to_string(),
                    security_type: row["securityType"].as_str().unwrap_or("").to_string(),
                    security_type_2: row["securityType2"].as_str().unwrap_or("").to_string(),
                    market_sector: row["marketSector"].as_str().unwrap_or("").to_string(),
                    security_description: row["securityDescription"]
                        .as_str()
                        .unwrap_or("")
                        .to_string(),
                });
            }
        }
    }
    Ok(out)
}

// ── Round 8 — HRA compute (historical return + risk) ──────────────

/// Compute an `HraSnapshot` from a chronologically-ordered slice of bars
/// (oldest → newest). Returns periods are simple-return (close₀→closeₙ),
/// annualized into CAGR for windows ≥ 252 trading days. Max drawdown is
/// computed over the full available history; Sharpe/Sortino use daily
/// log-returns annualized with the supplied risk-free rate.
pub fn compute_hra_snapshot(
    symbol: &str,
    as_of: &str,
    bars_oldest_first: &[HistoricalPriceRow],
    risk_free_pct: f64,
) -> HraSnapshot {
    if bars_oldest_first.len() < 2 {
        return HraSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            note: "insufficient bar history (need ≥ 2 bars)".to_string(),
            ..Default::default()
        };
    }
    let n = bars_oldest_first.len();
    let last_close = bars_oldest_first[n - 1].close;
    let first_close = bars_oldest_first[0].close;

    // Helper: return (pct) from a trading-day lookback (uses adjusted close
    // when available so splits/dividends don't poison the return).
    let px = |i: usize| -> f64 {
        let b = &bars_oldest_first[i];
        if b.adj_close > 0.0 {
            b.adj_close
        } else {
            b.close
        }
    };
    let last_px = px(n - 1);

    let mut windows: Vec<HraWindow> = Vec::new();
    let add_trading_window = |windows: &mut Vec<HraWindow>, label: &str, days: usize| {
        if n <= days {
            return;
        }
        let start = n - 1 - days;
        let start_px = px(start);
        if start_px <= 0.0 {
            return;
        }
        let ret = (last_px / start_px - 1.0) * 100.0;
        let cagr = if days >= 252 {
            let years = days as f64 / 252.0;
            ((last_px / start_px).powf(1.0 / years) - 1.0) * 100.0
        } else {
            ret
        };
        windows.push(HraWindow {
            label: label.to_string(),
            trading_days: days,
            return_pct: ret,
            cagr_pct: cagr,
            n_observations: days,
        });
    };
    add_trading_window(&mut windows, "1D", 1);
    add_trading_window(&mut windows, "5D", 5);
    add_trading_window(&mut windows, "1M", 21);
    add_trading_window(&mut windows, "3M", 63);
    add_trading_window(&mut windows, "6M", 126);
    add_trading_window(&mut windows, "1Y", 252);
    add_trading_window(&mut windows, "3Y", 756);
    add_trading_window(&mut windows, "5Y", 1260);

    // YTD: first bar whose date starts with current year.
    let year_prefix = as_of.get(..4).unwrap_or("");
    if !year_prefix.is_empty() {
        if let Some(ytd_start) = bars_oldest_first
            .iter()
            .position(|b| b.date.starts_with(year_prefix))
        {
            let start_px = px(ytd_start);
            if start_px > 0.0 {
                let ret = (last_px / start_px - 1.0) * 100.0;
                windows.push(HraWindow {
                    label: "YTD".to_string(),
                    trading_days: 0,
                    return_pct: ret,
                    cagr_pct: ret,
                    n_observations: n - ytd_start,
                });
            }
        }
    }

    // ITD: full span.
    if first_close > 0.0 {
        let ret = (last_px / first_close - 1.0) * 100.0;
        let years = (n as f64 / 252.0).max(1.0 / 252.0);
        let cagr = ((last_px / first_close).powf(1.0 / years) - 1.0) * 100.0;
        windows.push(HraWindow {
            label: "ITD".to_string(),
            trading_days: n - 1,
            return_pct: ret,
            cagr_pct: cagr,
            n_observations: n,
        });
    }

    // Max drawdown: walk forward tracking running peak.
    let mut peak = px(0);
    let mut peak_idx = 0usize;
    let mut max_dd = 0.0f64;
    let mut dd_peak_idx = 0usize;
    let mut dd_trough_idx = 0usize;
    for i in 1..n {
        let p = px(i);
        if p > peak {
            peak = p;
            peak_idx = i;
        }
        if peak > 0.0 {
            let dd = (p / peak - 1.0) * 100.0;
            if dd < max_dd {
                max_dd = dd;
                dd_peak_idx = peak_idx;
                dd_trough_idx = i;
            }
        }
    }

    // Daily log returns → annualized volatility and Sharpe/Sortino.
    let mut log_rets: Vec<f64> = Vec::with_capacity(n.saturating_sub(1));
    for i in 1..n {
        let p0 = px(i - 1);
        let p1 = px(i);
        if p0 > 0.0 && p1 > 0.0 {
            log_rets.push((p1 / p0).ln());
        }
    }
    let (vol_ann_pct, sharpe, sortino) = if log_rets.len() >= 20 {
        let m = log_rets.iter().sum::<f64>() / log_rets.len() as f64;
        let var = log_rets.iter().map(|r| (r - m).powi(2)).sum::<f64>() / log_rets.len() as f64;
        let sd = var.sqrt();
        let down: Vec<f64> = log_rets.iter().copied().filter(|r| *r < 0.0).collect();
        let dsd = if down.is_empty() {
            sd
        } else {
            let dm = down.iter().sum::<f64>() / down.len() as f64;
            (down.iter().map(|r| (r - dm).powi(2)).sum::<f64>() / down.len() as f64).sqrt()
        };
        let rf_daily = (risk_free_pct / 100.0) / 252.0;
        let sharpe = if sd > 1e-9 {
            (m - rf_daily) / sd * (252.0f64).sqrt()
        } else {
            0.0
        };
        let sortino = if dsd > 1e-9 {
            (m - rf_daily) / dsd * (252.0f64).sqrt()
        } else {
            0.0
        };
        (sd * (252.0f64).sqrt() * 100.0, sharpe, sortino)
    } else {
        (0.0, 0.0, 0.0)
    };

    let itd_cagr = windows
        .iter()
        .find(|w| w.label == "ITD")
        .map(|w| w.cagr_pct)
        .unwrap_or(0.0);
    let calmar = if max_dd.abs() > 1e-9 {
        itd_cagr / max_dd.abs()
    } else {
        0.0
    };

    HraSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        last_close,
        windows,
        max_drawdown_pct: max_dd,
        drawdown_peak_date: bars_oldest_first
            .get(dd_peak_idx)
            .map(|b| b.date.clone())
            .unwrap_or_default(),
        drawdown_trough_date: bars_oldest_first
            .get(dd_trough_idx)
            .map(|b| b.date.clone())
            .unwrap_or_default(),
        volatility_annual_pct: vol_ann_pct,
        sharpe_ratio: sharpe,
        sortino_ratio: sortino,
        calmar_ratio: calmar,
        risk_free_pct,
        note: String::new(),
    }
}

// ── Round 8 — DCF compute (Discounted Cash Flow, FCFF basis) ─────

/// Compute a multi-year DCF fair-value snapshot on a free cash flow to firm
/// (FCFF) basis. All inputs are already-cached values — this is pure compute.
///
/// Formula: EV = Σ(FCFFₜ / (1 + wacc)ᵗ) + TV / (1 + wacc)ⁿ
/// where TV = FCFFₙ × (1 + terminal_g) / (wacc − terminal_g).
/// Equity value = EV − debt + cash. Implied price = equity / shares.
#[allow(clippy::too_many_arguments)]
pub fn compute_dcf_snapshot(
    symbol: &str,
    as_of: &str,
    base_revenue: f64,
    base_fcff: f64,
    growth_pct: f64,
    terminal_growth_pct: f64,
    wacc_pct: f64,
    tax_rate_pct: f64,
    projection_years: usize,
    total_debt: f64,
    cash_and_equivalents: f64,
    shares_outstanding: f64,
) -> DcfSnapshot {
    let wacc = wacc_pct / 100.0;
    let g = growth_pct / 100.0;
    let tg = terminal_growth_pct / 100.0;

    if wacc <= 0.0 || shares_outstanding <= 0.0 || base_fcff.abs() < 1e-6 {
        return DcfSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            method: "DCF on FCFF".to_string(),
            base_revenue,
            base_fcff,
            growth_pct,
            terminal_growth_pct,
            wacc_pct,
            tax_rate_pct,
            projection_years,
            shares_outstanding,
            total_debt,
            cash_and_equivalents,
            note: "insufficient inputs (wacc, shares, or base fcff ≈ 0)".to_string(),
            ..Default::default()
        };
    }
    if tg + 0.005 >= wacc {
        return DcfSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            method: "DCF on FCFF".to_string(),
            base_revenue,
            base_fcff,
            growth_pct,
            terminal_growth_pct,
            wacc_pct,
            tax_rate_pct,
            projection_years,
            shares_outstanding,
            total_debt,
            cash_and_equivalents,
            note: format!(
                "terminal growth {:.2}% ≥ WACC {:.2}% — DCF degenerate",
                terminal_growth_pct, wacc_pct
            ),
            ..Default::default()
        };
    }

    let fcff_margin_pct = if base_revenue > 0.0 {
        base_fcff / base_revenue * 100.0
    } else {
        0.0
    };

    let mut years: Vec<DcfYear> = Vec::with_capacity(projection_years);
    let mut pv_sum = 0.0f64;
    let mut last_fcff = base_fcff;
    let mut last_revenue = base_revenue;
    for t in 1..=projection_years {
        last_revenue *= 1.0 + g;
        last_fcff *= 1.0 + g;
        let discount = (1.0 + wacc).powi(t as i32);
        let df = 1.0 / discount;
        let pv = last_fcff * df;
        pv_sum += pv;
        years.push(DcfYear {
            year: t as i32,
            revenue: last_revenue,
            ebit: 0.0,
            nopat: 0.0,
            fcff: last_fcff,
            discount_factor: df,
            pv_fcff: pv,
        });
    }

    let terminal_value = last_fcff * (1.0 + tg) / (wacc - tg);
    let pv_terminal = terminal_value / (1.0 + wacc).powi(projection_years as i32);
    let enterprise_value = pv_sum + pv_terminal;
    let equity_value = enterprise_value - total_debt + cash_and_equivalents;
    let implied_price = if shares_outstanding > 0.0 {
        equity_value / shares_outstanding
    } else {
        0.0
    };

    DcfSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        method: "DCF on FCFF".to_string(),
        base_revenue,
        base_fcff,
        growth_pct,
        terminal_growth_pct,
        wacc_pct,
        tax_rate_pct,
        fcff_margin_pct,
        projection_years,
        years,
        pv_sum,
        terminal_value,
        pv_terminal,
        enterprise_value,
        total_debt,
        cash_and_equivalents,
        equity_value,
        shares_outstanding,
        implied_price,
        note: String::new(),
    }
}

// ── Round 8 — SVM compute (Stock Valuation Model triangulation) ──

/// Build a multi-model fair-value triangulation from the caller's cached
/// WACC / DDM / DCF / RV snapshots plus any peer-median multiples the
/// caller has already computed. All inputs are optional — rows with no
/// implied price are skipped.
pub fn compute_svm_snapshot(
    symbol: &str,
    as_of: &str,
    current_price: f64,
    ddm: Option<&DdmSnapshot>,
    dcf: Option<&DcfSnapshot>,
    peer_pe_median: Option<(f64, f64)>, // (peer_pe, subject eps)
    peer_ev_ebitda_median: Option<(f64, f64, f64, f64, f64)>, // (peer_ev/ebitda, ebitda, debt, cash, shares)
    peer_pbook_median: Option<(f64, f64)>,                    // (peer_pb, book value per share)
) -> SvmSnapshot {
    let mut rows: Vec<SvmModelRow> = Vec::new();
    let push = |rows: &mut Vec<SvmModelRow>,
                model: &str,
                implied: f64,
                source: String,
                confidence: &str| {
        if implied <= 0.0 {
            return;
        }
        let upside = if current_price > 0.0 {
            (implied / current_price - 1.0) * 100.0
        } else {
            0.0
        };
        rows.push(SvmModelRow {
            model: model.to_string(),
            implied_price: implied,
            current_price,
            upside_pct: upside,
            confidence: confidence.to_string(),
            source,
        });
    };

    if let Some(d) = ddm {
        if d.implied_price > 0.0 {
            push(
                &mut rows,
                "DDM Gordon Growth",
                d.implied_price,
                format!(
                    "{} · g={:.2}% · r={:.2}%",
                    d.method, d.implied_growth_pct, d.required_return_pct
                ),
                "medium",
            );
        }
    }
    if let Some(d) = dcf {
        if d.implied_price > 0.0 {
            push(
                &mut rows,
                "DCF on FCFF",
                d.implied_price,
                format!(
                    "{} · WACC={:.2}% · g={:.2}% · TG={:.2}%",
                    d.method, d.wacc_pct, d.growth_pct, d.terminal_growth_pct
                ),
                "medium",
            );
        }
    }
    if let Some((peer_pe, eps)) = peer_pe_median {
        if peer_pe > 0.0 && eps > 0.0 {
            push(
                &mut rows,
                "RV peer P/E median",
                peer_pe * eps,
                format!("peer median P/E {:.2}× · EPS {:.2}", peer_pe, eps),
                "low",
            );
        }
    }
    if let Some((peer_evebitda, ebitda, debt, cash, shares)) = peer_ev_ebitda_median {
        if peer_evebitda > 0.0 && ebitda > 0.0 && shares > 0.0 {
            let ev_implied = peer_evebitda * ebitda;
            let equity = ev_implied - debt + cash;
            let implied = equity / shares;
            push(
                &mut rows,
                "RV peer EV/EBITDA median",
                implied,
                format!(
                    "peer median EV/EBITDA {:.2}× · EBITDA {:.0}",
                    peer_evebitda, ebitda
                ),
                "low",
            );
        }
    }
    if let Some((peer_pb, bvps)) = peer_pbook_median {
        if peer_pb > 0.0 && bvps > 0.0 {
            push(
                &mut rows,
                "RV peer P/B median",
                peer_pb * bvps,
                format!("peer median P/B {:.2}× · BVPS {:.2}", peer_pb, bvps),
                "low",
            );
        }
    }

    let implied: Vec<f64> = rows.iter().map(|r| r.implied_price).collect();
    let (fair_low, fair_high, fair_mid) = if implied.is_empty() {
        (0.0, 0.0, 0.0)
    } else {
        let lo = implied.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = implied.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let mid = implied.iter().sum::<f64>() / implied.len() as f64;
        (lo, hi, mid)
    };
    let upside_mid = if current_price > 0.0 && fair_mid > 0.0 {
        (fair_mid / current_price - 1.0) * 100.0
    } else {
        0.0
    };

    let note = if rows.is_empty() {
        "no valuation models available — run WACC/DDM/DCF/RV first".to_string()
    } else {
        String::new()
    };

    SvmSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        current_price,
        rows,
        fair_low,
        fair_high,
        fair_mid,
        upside_mid_pct: upside_mid,
        note,
    }
}

// ── Round 8 — OMON fetch (Yahoo options chain) ───────────────────

fn parse_yahoo_option_contract(
    c: &serde_json::Value,
    opt_type: &str,
    underlying: f64,
) -> OptionContract {
    let strike = c.get("strike").and_then(|x| x.as_f64()).unwrap_or(0.0);
    let itm = match opt_type {
        "CALL" => underlying > strike,
        _ => underlying < strike,
    };
    OptionContract {
        contract_symbol: c
            .get("contractSymbol")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        option_type: opt_type.to_string(),
        strike,
        last_price: c.get("lastPrice").and_then(|x| x.as_f64()).unwrap_or(0.0),
        bid: c.get("bid").and_then(|x| x.as_f64()).unwrap_or(0.0),
        ask: c.get("ask").and_then(|x| x.as_f64()).unwrap_or(0.0),
        volume: c.get("volume").and_then(|x| x.as_f64()).unwrap_or(0.0),
        open_interest: c
            .get("openInterest")
            .and_then(|x| x.as_f64())
            .unwrap_or(0.0),
        implied_volatility: c
            .get("impliedVolatility")
            .and_then(|x| x.as_f64())
            .unwrap_or(0.0),
        in_the_money: itm,
    }
}

pub(crate) fn parse_yahoo_option_expiry(
    options: &serde_json::Value,
    underlying_price: f64,
) -> OptionExpiry {
    let exp_ts = options
        .get("expirationDate")
        .and_then(|x| x.as_i64())
        .unwrap_or(0);
    let expiration = if exp_ts > 0 {
        chrono::DateTime::<chrono::Utc>::from_timestamp(exp_ts, 0)
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };
    let now = chrono::Utc::now().timestamp();
    let days_to_expiry = if exp_ts > now {
        (exp_ts - now) / 86400
    } else {
        0
    };

    let calls: Vec<OptionContract> = options
        .get("calls")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .map(|c| parse_yahoo_option_contract(c, "CALL", underlying_price))
                .collect()
        })
        .unwrap_or_default();
    let puts: Vec<OptionContract> = options
        .get("puts")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .map(|c| parse_yahoo_option_contract(c, "PUT", underlying_price))
                .collect()
        })
        .unwrap_or_default();

    OptionExpiry {
        expiration,
        days_to_expiry,
        calls,
        puts,
    }
}
