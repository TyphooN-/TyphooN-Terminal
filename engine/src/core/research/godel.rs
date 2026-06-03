use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use super::now_ts;
use super::*;

// ADR-109..ADR-114 types (extracted)

// ── ADR-115 Round 8 — HRA compute (historical return + risk) ──────────────

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
        if b.adj_close > 0.0 { b.adj_close } else { b.close }
    };
    let last_px = px(n - 1);

    let mut windows: Vec<HraWindow> = Vec::new();
    let add_trading_window = |windows: &mut Vec<HraWindow>, label: &str, days: usize| {
        if n <= days { return; }
        let start = n - 1 - days;
        let start_px = px(start);
        if start_px <= 0.0 { return; }
        let ret = (last_px / start_px - 1.0) * 100.0;
        let cagr = if days >= 252 {
            let years = days as f64 / 252.0;
            ((last_px / start_px).powf(1.0 / years) - 1.0) * 100.0
        } else { ret };
        windows.push(HraWindow {
            label: label.to_string(),
            trading_days: days,
            return_pct: ret,
            cagr_pct: cagr,
            n_observations: days,
        });
    };
    add_trading_window(&mut windows, "1D",   1);
    add_trading_window(&mut windows, "5D",   5);
    add_trading_window(&mut windows, "1M",   21);
    add_trading_window(&mut windows, "3M",   63);
    add_trading_window(&mut windows, "6M",   126);
    add_trading_window(&mut windows, "1Y",   252);
    add_trading_window(&mut windows, "3Y",   756);
    add_trading_window(&mut windows, "5Y",   1260);

    // YTD: first bar whose date starts with current year.
    let year_prefix = as_of.get(..4).unwrap_or("");
    if !year_prefix.is_empty() {
        if let Some(ytd_start) = bars_oldest_first.iter()
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
        if p > peak { peak = p; peak_idx = i; }
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
        if p0 > 0.0 && p1 > 0.0 { log_rets.push((p1 / p0).ln()); }
    }
    let (vol_ann_pct, sharpe, sortino) = if log_rets.len() >= 20 {
        let m = log_rets.iter().sum::<f64>() / log_rets.len() as f64;
        let var = log_rets.iter().map(|r| (r - m).powi(2)).sum::<f64>() / log_rets.len() as f64;
        let sd = var.sqrt();
        let down: Vec<f64> = log_rets.iter().copied().filter(|r| *r < 0.0).collect();
        let dsd = if down.is_empty() { sd } else {
            let dm = down.iter().sum::<f64>() / down.len() as f64;
            (down.iter().map(|r| (r - dm).powi(2)).sum::<f64>() / down.len() as f64).sqrt()
        };
        let rf_daily = (risk_free_pct / 100.0) / 252.0;
        let sharpe = if sd > 1e-9 { (m - rf_daily) / sd * (252.0f64).sqrt() } else { 0.0 };
        let sortino = if dsd > 1e-9 { (m - rf_daily) / dsd * (252.0f64).sqrt() } else { 0.0 };
        (sd * (252.0f64).sqrt() * 100.0, sharpe, sortino)
    } else {
        (0.0, 0.0, 0.0)
    };

    let itd_cagr = windows.iter().find(|w| w.label == "ITD").map(|w| w.cagr_pct).unwrap_or(0.0);
    let calmar = if max_dd.abs() > 1e-9 { itd_cagr / max_dd.abs() } else { 0.0 };

    HraSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        last_close,
        windows,
        max_drawdown_pct: max_dd,
        drawdown_peak_date: bars_oldest_first.get(dd_peak_idx).map(|b| b.date.clone()).unwrap_or_default(),
        drawdown_trough_date: bars_oldest_first.get(dd_trough_idx).map(|b| b.date.clone()).unwrap_or_default(),
        volatility_annual_pct: vol_ann_pct,
        sharpe_ratio: sharpe,
        sortino_ratio: sortino,
        calmar_ratio: calmar,
        risk_free_pct,
        note: String::new(),
    }
}

// ── ADR-115 Round 8 — DCF compute (Discounted Cash Flow, FCFF basis) ─────

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
    let g    = growth_pct / 100.0;
    let tg   = terminal_growth_pct / 100.0;

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
            note: format!("terminal growth {:.2}% ≥ WACC {:.2}% — DCF degenerate", terminal_growth_pct, wacc_pct),
            ..Default::default()
        };
    }

    let fcff_margin_pct = if base_revenue > 0.0 { base_fcff / base_revenue * 100.0 } else { 0.0 };

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
    let implied_price = if shares_outstanding > 0.0 { equity_value / shares_outstanding } else { 0.0 };

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

// ── ADR-115 Round 8 — SVM compute (Stock Valuation Model triangulation) ──

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
    peer_pe_median: Option<(f64, f64)>,           // (peer_pe, subject eps)
    peer_ev_ebitda_median: Option<(f64, f64, f64, f64, f64)>, // (peer_ev/ebitda, ebitda, debt, cash, shares)
    peer_pbook_median: Option<(f64, f64)>,        // (peer_pb, book value per share)
) -> SvmSnapshot {
    let mut rows: Vec<SvmModelRow> = Vec::new();
    let push = |rows: &mut Vec<SvmModelRow>, model: &str, implied: f64, source: String, confidence: &str| {
        if implied <= 0.0 { return; }
        let upside = if current_price > 0.0 { (implied / current_price - 1.0) * 100.0 } else { 0.0 };
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
            push(&mut rows, "DDM Gordon Growth", d.implied_price,
                 format!("{} · g={:.2}% · r={:.2}%", d.method, d.implied_growth_pct, d.required_return_pct),
                 "medium");
        }
    }
    if let Some(d) = dcf {
        if d.implied_price > 0.0 {
            push(&mut rows, "DCF on FCFF", d.implied_price,
                 format!("{} · WACC={:.2}% · g={:.2}% · TG={:.2}%", d.method, d.wacc_pct, d.growth_pct, d.terminal_growth_pct),
                 "medium");
        }
    }
    if let Some((peer_pe, eps)) = peer_pe_median {
        if peer_pe > 0.0 && eps > 0.0 {
            push(&mut rows, "RV peer P/E median", peer_pe * eps,
                 format!("peer median P/E {:.2}× · EPS {:.2}", peer_pe, eps), "low");
        }
    }
    if let Some((peer_evebitda, ebitda, debt, cash, shares)) = peer_ev_ebitda_median {
        if peer_evebitda > 0.0 && ebitda > 0.0 && shares > 0.0 {
            let ev_implied = peer_evebitda * ebitda;
            let equity = ev_implied - debt + cash;
            let implied = equity / shares;
            push(&mut rows, "RV peer EV/EBITDA median", implied,
                 format!("peer median EV/EBITDA {:.2}× · EBITDA {:.0}", peer_evebitda, ebitda), "low");
        }
    }
    if let Some((peer_pb, bvps)) = peer_pbook_median {
        if peer_pb > 0.0 && bvps > 0.0 {
            push(&mut rows, "RV peer P/B median", peer_pb * bvps,
                 format!("peer median P/B {:.2}× · BVPS {:.2}", peer_pb, bvps), "low");
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
    let upside_mid = if current_price > 0.0 && fair_mid > 0.0 { (fair_mid / current_price - 1.0) * 100.0 } else { 0.0 };

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

// ── ADR-115 Round 8 — OMON fetch (Yahoo options chain) ───────────────────

/// Fetch a Yahoo options chain for a symbol. Returns all expirations Yahoo
/// is willing to give us in a single call (typically 1–12 weeklies + LEAPS).
pub async fn fetch_yahoo_options_chain(
    client: &reqwest::Client,
    symbol: &str,
) -> Result<OptionsChainSnapshot, String> {
    let url = format!("https://query2.finance.yahoo.com/v7/finance/options/{}", symbol.to_uppercase());
    let resp = client.get(&url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1")
        .send().await
        .map_err(|e| format!("Yahoo options request: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Yahoo options: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("Yahoo options parse: {e}"))?;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let result = v.pointer("/optionChain/result/0")
        .ok_or_else(|| "Yahoo options: empty result".to_string())?;
    let underlying_price = result.pointer("/quote/regularMarketPrice")
        .and_then(|x| x.as_f64()).unwrap_or(0.0);

    let expiration_dates: Vec<i64> = result.get("expirationDates")
        .and_then(|x| x.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect())
        .unwrap_or_default();

    // Yahoo only returns one expiration's chain per call when we don't pass
    // &date=… — we take whatever came back in options[0].
    let options = result.get("options").and_then(|x| x.as_array())
        .and_then(|arr| arr.first())
        .ok_or_else(|| "Yahoo options: options[0] missing".to_string())?;

    let parse_contract = |c: &serde_json::Value, opt_type: &str, underlying: f64| -> OptionContract {
        let strike = c.get("strike").and_then(|x| x.as_f64()).unwrap_or(0.0);
        let itm = match opt_type {
            "CALL" => underlying > strike,
            _      => underlying < strike,
        };
        OptionContract {
            contract_symbol: c.get("contractSymbol").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            option_type: opt_type.to_string(),
            strike,
            last_price: c.get("lastPrice").and_then(|x| x.as_f64()).unwrap_or(0.0),
            bid: c.get("bid").and_then(|x| x.as_f64()).unwrap_or(0.0),
            ask: c.get("ask").and_then(|x| x.as_f64()).unwrap_or(0.0),
            volume: c.get("volume").and_then(|x| x.as_f64()).unwrap_or(0.0),
            open_interest: c.get("openInterest").and_then(|x| x.as_f64()).unwrap_or(0.0),
            implied_volatility: c.get("impliedVolatility").and_then(|x| x.as_f64()).unwrap_or(0.0),
            in_the_money: itm,
        }
    };

    let exp_ts = options.get("expirationDate").and_then(|x| x.as_i64()).unwrap_or(0);
    let expiration = if exp_ts > 0 {
        chrono::DateTime::<chrono::Utc>::from_timestamp(exp_ts, 0)
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default()
    } else { String::new() };
    let now = chrono::Utc::now().timestamp();
    let days_to_expiry = if exp_ts > now { (exp_ts - now) / 86400 } else { 0 };

    let calls: Vec<OptionContract> = options.get("calls").and_then(|x| x.as_array())
        .map(|arr| arr.iter().map(|c| parse_contract(c, "CALL", underlying_price)).collect())
        .unwrap_or_default();
    let puts: Vec<OptionContract> = options.get("puts").and_then(|x| x.as_array())
        .map(|arr| arr.iter().map(|c| parse_contract(c, "PUT", underlying_price)).collect())
        .unwrap_or_default();

    let note = if expiration_dates.len() > 1 {
        format!("Yahoo returned first of {} expirations; additional dates available",
            expiration_dates.len())
    } else { String::new() };

    Ok(OptionsChainSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: today,
        underlying_price,
        expirations: vec![OptionExpiry { expiration, days_to_expiry, calls, puts }],
        note,
    })
}

// ── ADR-115 Round 8 — IVOL compute (IV Rank / IV Percentile) ─────────────

/// Compute an `IvolSnapshot` from a 52-week history of ATM IV observations
/// plus a current ATM IV reading. The caller is responsible for extracting
/// the ATM IV from an `OptionsChainSnapshot` (or from any other source).
///
/// IV Rank: `(current − 52w low) / (52w high − 52w low) × 100`.
/// IV Percentile: `% of history ≤ current`.
pub fn compute_ivol_snapshot(
    symbol: &str,
    as_of: &str,
    current_atm_iv_pct: f64,
    history: &[IvolObservation],
) -> IvolSnapshot {
    if history.is_empty() {
        return IvolSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            current_atm_iv_pct,
            iv_52w_low_pct: current_atm_iv_pct,
            iv_52w_high_pct: current_atm_iv_pct,
            iv_rank: 50.0,
            iv_percentile: 50.0,
            observation_count: 0,
            history: Vec::new(),
            note: "no IV history — rank/percentile are placeholders until history accumulates".to_string(),
        };
    }
    let mut vals: Vec<f64> = history.iter().map(|o| o.atm_iv_pct).filter(|v| v.is_finite() && *v > 0.0).collect();
    vals.push(current_atm_iv_pct);
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let lo = vals.first().copied().unwrap_or(current_atm_iv_pct);
    let hi = vals.last().copied().unwrap_or(current_atm_iv_pct);
    let rank = if (hi - lo).abs() > 1e-9 {
        ((current_atm_iv_pct - lo) / (hi - lo)) * 100.0
    } else { 50.0 };
    let le_count = vals.iter().filter(|v| **v <= current_atm_iv_pct).count();
    let pct = (le_count as f64 / vals.len() as f64) * 100.0;

    IvolSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        current_atm_iv_pct,
        iv_52w_low_pct: lo,
        iv_52w_high_pct: hi,
        iv_rank: rank.clamp(0.0, 100.0),
        iv_percentile: pct.clamp(0.0, 100.0),
        observation_count: history.len(),
        history: history.to_vec(),
        note: if history.len() < 20 {
            format!("only {} observations — rank stabilizes around 252", history.len())
        } else { String::new() },
    }
}

// ── ADR-116 Round 9 — SEAG compute (seasonality) ─────────────────────────

/// Compute a `SeasonalitySnapshot` from a chronologically-ordered slice of
/// bars. Builds monthly buckets (Jan..Dec) of year-over-year per-month returns
/// (first bar of month → last bar of month) and day-of-week buckets of daily
/// log-returns. Pure compute, no network.
pub fn compute_seasonality_snapshot(
    symbol: &str,
    as_of: &str,
    bars_oldest_first: &[HistoricalPriceRow],
) -> SeasonalitySnapshot {
    if bars_oldest_first.len() < 30 {
        return SeasonalitySnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            note: "insufficient bar history (need ≥ 30 bars)".to_string(),
            ..Default::default()
        };
    }

    let px = |b: &HistoricalPriceRow| -> f64 {
        if b.adj_close > 0.0 { b.adj_close } else { b.close }
    };

    // ── Monthly buckets: group bars by YYYY-MM and compute per-(year, month)
    // simple return from first bar to last bar of that month, then aggregate
    // across years into the 12 buckets.
    use std::collections::BTreeMap;
    let mut per_ym: BTreeMap<(i32, u32), (f64, f64)> = BTreeMap::new(); // (year, month) → (first, last)
    let mut years_seen: std::collections::BTreeSet<i32> = std::collections::BTreeSet::new();
    for b in bars_oldest_first {
        if b.date.len() < 10 { continue; }
        let year: i32 = match b.date.get(0..4).and_then(|s| s.parse().ok()) { Some(y) => y, None => continue };
        let month: u32 = match b.date.get(5..7).and_then(|s| s.parse().ok()) { Some(m) => m, None => continue };
        let p = px(b);
        if p <= 0.0 { continue; }
        years_seen.insert(year);
        per_ym.entry((year, month)).and_modify(|e| e.1 = p).or_insert((p, p));
    }

    let month_label = |m: u32| -> &'static str {
        match m {
            1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr",
            5 => "May", 6 => "Jun", 7 => "Jul", 8 => "Aug",
            9 => "Sep", 10 => "Oct", 11 => "Nov", 12 => "Dec",
            _ => "?",
        }
    };

    let mut months: Vec<SeasonalityMonth> = Vec::new();
    for m in 1u32..=12 {
        let rets: Vec<f64> = per_ym.iter()
            .filter_map(|((_y, mm), (first, last))| {
                if *mm == m && *first > 0.0 { Some((last / first - 1.0) * 100.0) } else { None }
            })
            .collect();
        if rets.is_empty() {
            months.push(SeasonalityMonth { month: m, label: month_label(m).to_string(), ..Default::default() });
            continue;
        }
        let mean = rets.iter().sum::<f64>() / rets.len() as f64;
        let var = rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / rets.len() as f64;
        let stdev = var.sqrt();
        let mut sorted = rets.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = sorted[sorted.len() / 2];
        let positive = rets.iter().filter(|r| **r > 0.0).count();
        let best = sorted.last().copied().unwrap_or(0.0);
        let worst = sorted.first().copied().unwrap_or(0.0);
        months.push(SeasonalityMonth {
            month: m,
            label: month_label(m).to_string(),
            avg_return_pct: mean,
            median_return_pct: median,
            stdev_pct: stdev,
            positive_years: positive,
            total_years: rets.len(),
            best_return_pct: best,
            worst_return_pct: worst,
        });
    }

    // ── Day-of-week buckets using log-returns on successive bars.
    let dow_label = |d: u32| -> &'static str {
        match d {
            1 => "Mon", 2 => "Tue", 3 => "Wed", 4 => "Thu",
            5 => "Fri", 6 => "Sat", 7 => "Sun", _ => "?",
        }
    };
    // Zeller-style computation for a YYYY-MM-DD string.
    let dow_of = |date: &str| -> Option<u32> {
        let y: i32 = date.get(0..4)?.parse().ok()?;
        let m: i32 = date.get(5..7)?.parse().ok()?;
        let d: i32 = date.get(8..10)?.parse().ok()?;
        // Zeller's congruence — returns 0=Sat..6=Fri; we remap to 1=Mon..7=Sun.
        let (q, m2, k_year) = if m < 3 { (d, m + 12, y - 1) } else { (d, m, y) };
        let k = k_year % 100;
        let j = k_year / 100;
        let h = (q + (13 * (m2 + 1)) / 5 + k + k / 4 + j / 4 + 5 * j).rem_euclid(7);
        // Zeller h: 0=Sat, 1=Sun, 2=Mon, 3=Tue, 4=Wed, 5=Thu, 6=Fri
        let iso = match h { 0 => 6, 1 => 7, 2 => 1, 3 => 2, 4 => 3, 5 => 4, 6 => 5, _ => 1 };
        Some(iso as u32)
    };

    let mut dow_map: BTreeMap<u32, (f64, usize, usize)> = BTreeMap::new(); // dow → (sum_log_ret, pos_count, total)
    for w in bars_oldest_first.windows(2) {
        let p0 = px(&w[0]);
        let p1 = px(&w[1]);
        if p0 <= 0.0 || p1 <= 0.0 { continue; }
        let r = (p1 / p0).ln();
        if let Some(d) = dow_of(&w[1].date) {
            let entry = dow_map.entry(d).or_insert((0.0, 0, 0));
            entry.0 += r;
            entry.2 += 1;
            if r > 0.0 { entry.1 += 1; }
        }
    }
    let mut dow_out: Vec<SeasonalityDow> = Vec::new();
    for d in 1u32..=5 {
        if let Some((sum, pos, total)) = dow_map.get(&d).cloned() {
            let mean_pct = if total > 0 { (sum / total as f64).exp().ln() * 100.0 } else { 0.0 };
            dow_out.push(SeasonalityDow {
                dow: d,
                label: dow_label(d).to_string(),
                avg_return_pct: mean_pct,
                positive_days: pos,
                total_days: total,
            });
        }
    }

    let mut best_month = String::new();
    let mut worst_month = String::new();
    let mut best_avg = f64::NEG_INFINITY;
    let mut worst_avg = f64::INFINITY;
    for m in &months {
        if m.total_years == 0 { continue; }
        if m.avg_return_pct > best_avg { best_avg = m.avg_return_pct; best_month = m.label.clone(); }
        if m.avg_return_pct < worst_avg { worst_avg = m.avg_return_pct; worst_month = m.label.clone(); }
    }

    SeasonalitySnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        years_covered: years_seen.len(),
        months,
        dow: dow_out,
        best_month,
        worst_month,
        note: String::new(),
    }
}

// ── ADR-116 Round 9 — COR compute (correlation matrix vs peers) ──────────

/// Compute a pairwise correlation matrix for a subject symbol against a set
/// of peer bar series over a rolling window of `window_days`. Uses Pearson
/// correlation on daily log-returns intersected by date, skipping peers with
/// fewer than 30 overlapping observations. Pure compute.
pub fn compute_correlation_matrix(
    symbol: &str,
    as_of: &str,
    window_days: usize,
    subject_bars: &[HistoricalPriceRow],
    peer_series: &[(String, Vec<HistoricalPriceRow>)],
) -> CorrelationMatrix {
    let px = |b: &HistoricalPriceRow| -> f64 {
        if b.adj_close > 0.0 { b.adj_close } else { b.close }
    };
    // Truncate subject to the most recent `window_days` bars (plus one anchor).
    let take = window_days.saturating_add(1).min(subject_bars.len());
    let subject_slice = &subject_bars[subject_bars.len() - take..];
    if subject_slice.len() < 31 {
        return CorrelationMatrix {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            window_days,
            note: "insufficient subject bar history (need ≥ 31)".to_string(),
            ..Default::default()
        };
    }

    // Build date→logret map for subject.
    use std::collections::HashMap;
    let mut sub_map: HashMap<String, f64> = HashMap::new();
    for w in subject_slice.windows(2) {
        let p0 = px(&w[0]);
        let p1 = px(&w[1]);
        if p0 > 0.0 && p1 > 0.0 {
            sub_map.insert(w[1].date.clone(), (p1 / p0).ln());
        }
    }

    let mut cells: Vec<CorrelationCell> = Vec::new();
    for (peer_sym, peer_bars) in peer_series {
        if peer_bars.len() < 31 { continue; }
        let ptake = window_days.saturating_add(1).min(peer_bars.len());
        let peer_slice = &peer_bars[peer_bars.len() - ptake..];
        // Build peer logret and intersect dates.
        let mut paired: Vec<(f64, f64)> = Vec::new();
        for w in peer_slice.windows(2) {
            let p0 = px(&w[0]);
            let p1 = px(&w[1]);
            if p0 <= 0.0 || p1 <= 0.0 { continue; }
            if let Some(s) = sub_map.get(&w[1].date) {
                paired.push((*s, (p1 / p0).ln()));
            }
        }
        if paired.len() < 30 { continue; }
        let n = paired.len() as f64;
        let mean_s: f64 = paired.iter().map(|(s, _)| *s).sum::<f64>() / n;
        let mean_p: f64 = paired.iter().map(|(_, p)| *p).sum::<f64>() / n;
        let mut cov = 0.0;
        let mut var_s = 0.0;
        let mut var_p = 0.0;
        for (s, p) in &paired {
            let ds = s - mean_s;
            let dp = p - mean_p;
            cov += ds * dp;
            var_s += ds * ds;
            var_p += dp * dp;
        }
        let denom = (var_s * var_p).sqrt();
        let rho = if denom > 1e-12 { cov / denom } else { 0.0 };
        let beta = if var_p > 1e-12 { cov / var_p } else { 0.0 };
        cells.push(CorrelationCell {
            peer_symbol: peer_sym.to_uppercase(),
            correlation: rho.clamp(-1.0, 1.0),
            n_observations: paired.len(),
            beta_vs_peer: beta,
        });
    }

    if cells.is_empty() {
        return CorrelationMatrix {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            window_days,
            note: "no peer pairs with ≥ 30 overlapping observations".to_string(),
            ..Default::default()
        };
    }
    let mean_corr = cells.iter().map(|c| c.correlation.abs()).sum::<f64>() / cells.len() as f64;
    let mut highest_sym = String::new();
    let mut lowest_sym = String::new();
    let mut hi = f64::NEG_INFINITY;
    let mut lo = f64::INFINITY;
    for c in &cells {
        if c.correlation > hi { hi = c.correlation; highest_sym = c.peer_symbol.clone(); }
        if c.correlation < lo { lo = c.correlation; lowest_sym = c.peer_symbol.clone(); }
    }

    CorrelationMatrix {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        window_days,
        cells,
        mean_correlation: mean_corr,
        highest_corr_symbol: highest_sym,
        lowest_corr_symbol: lowest_sym,
        note: String::new(),
    }
}

// ── ADR-116 Round 9 — TRA compute (total return = price + dividends) ────

/// Compute a `TotalReturnSnapshot` by combining HP price returns with the
/// sum of cash dividends paid over the same window. Pure compute; inputs are
/// already-cached bars and dividend records.
pub fn compute_total_return_snapshot(
    symbol: &str,
    as_of: &str,
    bars_oldest_first: &[HistoricalPriceRow],
    dividends: &[DividendRecord],
) -> TotalReturnSnapshot {
    if bars_oldest_first.len() < 2 {
        return TotalReturnSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            note: "insufficient bar history (need ≥ 2 bars)".to_string(),
            ..Default::default()
        };
    }
    let n = bars_oldest_first.len();
    let last_close = bars_oldest_first[n - 1].close;
    let last_date = bars_oldest_first[n - 1].date.clone();

    let px = |i: usize| -> f64 {
        let b = &bars_oldest_first[i];
        if b.adj_close > 0.0 { b.adj_close } else { b.close }
    };
    let last_px = px(n - 1);

    // Trailing 12 month dividends by ex_date cutoff.
    let cutoff_ttm = {
        // Naive 12-month cutoff: subtract one from the year component.
        let y: i32 = last_date.get(0..4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let m = last_date.get(5..7).unwrap_or("01");
        let d = last_date.get(8..10).unwrap_or("01");
        format!("{:04}-{}-{}", y - 1, m, d)
    };
    let ttm_divs: f64 = dividends.iter()
        .filter(|d| d.ex_date.as_str() > cutoff_ttm.as_str() && d.ex_date.as_str() <= last_date.as_str())
        .map(|d| d.amount)
        .sum();
    let ttm_yield = if last_close > 0.0 { ttm_divs / last_close * 100.0 } else { 0.0 };

    let mut windows: Vec<TotalReturnWindow> = Vec::new();
    let push_window = |windows: &mut Vec<TotalReturnWindow>, label: &str, start_idx: usize, trading_days: usize| {
        if start_idx >= n - 1 { return; }
        let start_px = px(start_idx);
        if start_px <= 0.0 { return; }
        let start_date = bars_oldest_first[start_idx].date.clone();
        let price_ret = (last_px / start_px - 1.0) * 100.0;
        let window_divs: f64 = dividends.iter()
            .filter(|d| d.ex_date.as_str() > start_date.as_str() && d.ex_date.as_str() <= last_date.as_str())
            .map(|d| d.amount)
            .sum();
        let n_divs = dividends.iter()
            .filter(|d| d.ex_date.as_str() > start_date.as_str() && d.ex_date.as_str() <= last_date.as_str())
            .count();
        let div_yield = if start_px > 0.0 { window_divs / start_px * 100.0 } else { 0.0 };
        let total = price_ret + div_yield;
        let annualized = if trading_days >= 252 {
            let years = trading_days as f64 / 252.0;
            (((total / 100.0) + 1.0).powf(1.0 / years) - 1.0) * 100.0
        } else { total };
        windows.push(TotalReturnWindow {
            label: label.to_string(),
            trading_days,
            price_return_pct: price_ret,
            dividend_yield_pct: div_yield,
            total_return_pct: total,
            annualized_pct: annualized,
            dividends_paid: window_divs,
            n_dividends: n_divs,
        });
    };

    for (label, days) in &[("1M", 21), ("3M", 63), ("6M", 126), ("1Y", 252), ("3Y", 756), ("5Y", 1260)] {
        if n > *days {
            push_window(&mut windows, label, n - 1 - days, *days);
        }
    }
    // YTD
    let year_prefix = as_of.get(..4).unwrap_or("");
    if !year_prefix.is_empty() {
        if let Some(ytd_start) = bars_oldest_first.iter().position(|b| b.date.starts_with(year_prefix)) {
            push_window(&mut windows, "YTD", ytd_start, n - ytd_start);
        }
    }

    TotalReturnSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        last_close,
        trailing_12m_dividends: ttm_divs,
        trailing_12m_yield_pct: ttm_yield,
        windows,
        note: String::new(),
    }
}

// ── ADR-116 Round 9 — SKEW compute (volatility smile/skew) ───────────────

/// Compute a `VolatilitySkew` snapshot from a cached options chain. For each
/// expiry, walk the strike ladder and emit a `SkewPoint` combining call & put
/// IV at that strike; compute ATM IV from the nearest-to-money strike, and
/// approximate a 25-delta put-call skew using ±10% OTM contracts.
pub fn compute_volatility_skew(
    symbol: &str,
    as_of: &str,
    chain: &OptionsChainSnapshot,
) -> VolatilitySkew {
    if chain.expirations.is_empty() || chain.underlying_price <= 0.0 {
        return VolatilitySkew {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            underlying_price: chain.underlying_price,
            note: "no expirations in options chain".to_string(),
            ..Default::default()
        };
    }

    let u = chain.underlying_price;
    let mut out_expiries: Vec<SkewExpiry> = Vec::new();

    for ex in &chain.expirations {
        // Merge calls + puts by strike.
        use std::collections::BTreeMap;
        let mut map: BTreeMap<i64, (Option<f64>, Option<f64>)> = BTreeMap::new(); // key = strike×100
        for c in &ex.calls {
            if c.implied_volatility <= 0.0 { continue; }
            let k = (c.strike * 100.0).round() as i64;
            map.entry(k).and_modify(|e| e.0 = Some(c.implied_volatility)).or_insert((Some(c.implied_volatility), None));
        }
        for p in &ex.puts {
            if p.implied_volatility <= 0.0 { continue; }
            let k = (p.strike * 100.0).round() as i64;
            map.entry(k).and_modify(|e| e.1 = Some(p.implied_volatility)).or_insert((None, Some(p.implied_volatility)));
        }
        let mut points: Vec<SkewPoint> = Vec::new();
        for (k, (cv, pv)) in &map {
            let strike = (*k as f64) / 100.0;
            let moneyness = (strike / u - 1.0) * 100.0;
            let call_iv = cv.map(|v| v * 100.0).unwrap_or(0.0);
            let put_iv = pv.map(|v| v * 100.0).unwrap_or(0.0);
            let combined = match (cv, pv) {
                (Some(a), Some(b)) => (a + b) / 2.0 * 100.0,
                (Some(a), None)    => a * 100.0,
                (None, Some(b))    => b * 100.0,
                (None, None)       => 0.0,
            };
            points.push(SkewPoint {
                strike,
                moneyness_pct: moneyness,
                call_iv_pct: call_iv,
                put_iv_pct: put_iv,
                combined_iv_pct: combined,
            });
        }

        if points.is_empty() {
            out_expiries.push(SkewExpiry {
                expiration: ex.expiration.clone(),
                days_to_expiry: ex.days_to_expiry,
                atm_iv_pct: 0.0,
                points,
                put_call_skew_25d_pct: 0.0,
                term_note: "no IV-populated strikes".to_string(),
            });
            continue;
        }

        // ATM IV: find strike closest to underlying.
        let mut atm_idx = 0usize;
        let mut best_dist = f64::INFINITY;
        for (i, p) in points.iter().enumerate() {
            let d = (p.strike - u).abs();
            if d < best_dist { best_dist = d; atm_idx = i; }
        }
        let atm_iv = points[atm_idx].combined_iv_pct;

        // ±10% OTM skew proxy.
        let target_otm_call = u * 1.10;
        let target_otm_put  = u * 0.90;
        let mut otm_call_iv = 0.0;
        let mut otm_put_iv = 0.0;
        let mut best_c = f64::INFINITY;
        let mut best_p = f64::INFINITY;
        for p in &points {
            let dc = (p.strike - target_otm_call).abs();
            let dp = (p.strike - target_otm_put).abs();
            if dc < best_c && p.call_iv_pct > 0.0 { best_c = dc; otm_call_iv = p.call_iv_pct; }
            if dp < best_p && p.put_iv_pct > 0.0 { best_p = dp; otm_put_iv = p.put_iv_pct; }
        }
        let skew = otm_put_iv - otm_call_iv;

        out_expiries.push(SkewExpiry {
            expiration: ex.expiration.clone(),
            days_to_expiry: ex.days_to_expiry,
            atm_iv_pct: atm_iv,
            points,
            put_call_skew_25d_pct: skew,
            term_note: String::new(),
        });
    }

    VolatilitySkew {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        underlying_price: u,
        expiries: out_expiries,
        note: String::new(),
    }
}

// ── ADR-117 Round 10 — LEV compute (leverage & coverage ratios) ─────────────

/// Compute a `LeverageSnapshot` from cached financial statements and the
/// Fundamentals row. Pulls trailing-12-month EBITDA + interest expense from
/// quarterly income statements, total debt / equity / cash from the most
/// recent annual balance sheet, and produces a standard battery of ratios.
pub fn compute_leverage_snapshot(
    symbol: &str,
    as_of: &str,
    statements: &FinancialStatements,
    total_debt_fund: f64,
    cash_fund: f64,
) -> LeverageSnapshot {
    // Prefer the latest annual balance sheet; fall back to the most recent quarter.
    let bal = statements.balance_annual.first()
        .or_else(|| statements.balance_quarterly.first());

    let total_debt = bal.map(|b| b.total_debt).filter(|v| *v > 0.0).unwrap_or(total_debt_fund);
    let cash = bal.map(|b| b.cash_and_equiv).filter(|v| *v > 0.0).unwrap_or(cash_fund);
    let net_debt = (total_debt - cash).max(0.0);
    let total_equity = bal.map(|b| b.total_equity).unwrap_or(0.0);

    // TTM roll-ups from quarterly income statements (last 4 quarters).
    let q = &statements.income_quarterly;
    let take = q.iter().take(4);
    let ebitda_ttm: f64 = take.clone().map(|i| i.ebitda).sum();
    let interest_ttm: f64 = take.clone().map(|i| i.interest_expense.abs()).sum();
    let op_inc_ttm: f64 = take.clone().map(|i| i.operating_income).sum();

    let cur_assets = bal.map(|b| b.total_current_assets).unwrap_or(0.0);
    let cur_liab = bal.map(|b| b.total_current_liabilities).unwrap_or(0.0);
    let inventory = bal.map(|b| b.inventory).unwrap_or(0.0);

    let mut ratios: Vec<LeverageRatio> = Vec::new();

    // Debt / EBITDA
    if ebitda_ttm > 0.0 && total_debt > 0.0 {
        let v = total_debt / ebitda_ttm;
        let sig = if v < 2.5 { "HEALTHY" } else if v < 4.0 { "ELEVATED" } else { "STRETCHED" };
        ratios.push(LeverageRatio {
            name: "Debt / EBITDA".into(), value: v, peer_median: 0.0,
            signal: sig.into(),
            note: "lower is safer; >4× typically flags high leverage".into(),
        });
    }

    // Net Debt / EBITDA
    if ebitda_ttm > 0.0 {
        let v = net_debt / ebitda_ttm;
        let sig = if v < 2.0 { "HEALTHY" } else if v < 3.5 { "ELEVATED" } else { "STRETCHED" };
        ratios.push(LeverageRatio {
            name: "Net Debt / EBITDA".into(), value: v, peer_median: 0.0,
            signal: sig.into(), note: "net of cash; negative when cash > debt".into(),
        });
    }

    // Debt / Equity
    if total_equity > 0.0 && total_debt > 0.0 {
        let v = total_debt / total_equity;
        let sig = if v < 1.0 { "HEALTHY" } else if v < 2.0 { "ELEVATED" } else { "STRETCHED" };
        ratios.push(LeverageRatio {
            name: "Debt / Equity".into(), value: v, peer_median: 0.0,
            signal: sig.into(), note: "gearing ratio; varies by sector".into(),
        });
    }

    // Interest Coverage (EBIT / Interest)
    if interest_ttm > 0.0 {
        let v = op_inc_ttm / interest_ttm;
        let sig = if v >= 5.0 { "HEALTHY" } else if v >= 2.0 { "ELEVATED" } else { "STRETCHED" };
        ratios.push(LeverageRatio {
            name: "Interest Coverage".into(), value: v, peer_median: 0.0,
            signal: sig.into(),
            note: "EBIT / interest expense; higher is safer; <2× distress signal".into(),
        });
    }

    // Current Ratio
    if cur_liab > 0.0 && cur_assets > 0.0 {
        let v = cur_assets / cur_liab;
        let sig = if v >= 1.5 { "HEALTHY" } else if v >= 1.0 { "ELEVATED" } else { "STRETCHED" };
        ratios.push(LeverageRatio {
            name: "Current Ratio".into(), value: v, peer_median: 0.0,
            signal: sig.into(),
            note: "short-term liquidity; <1 flags near-term squeeze".into(),
        });
    }

    // Quick Ratio
    if cur_liab > 0.0 && cur_assets > 0.0 {
        let v = (cur_assets - inventory) / cur_liab;
        let sig = if v >= 1.0 { "HEALTHY" } else if v >= 0.7 { "ELEVATED" } else { "STRETCHED" };
        ratios.push(LeverageRatio {
            name: "Quick Ratio".into(), value: v, peer_median: 0.0,
            signal: sig.into(),
            note: "excludes inventory; more conservative than current ratio".into(),
        });
    }

    // Solvency summary: count HEALTHY vs STRETCHED signals.
    let n_health = ratios.iter().filter(|r| r.signal == "HEALTHY").count();
    let n_stretch = ratios.iter().filter(|r| r.signal == "STRETCHED").count();
    let solvency_summary = if ratios.is_empty() {
        "insufficient data — run FA + EVSCRAPE first".to_string()
    } else if n_stretch >= 2 {
        format!("STRETCHED — {}/{} ratios flagged", n_stretch, ratios.len())
    } else if n_health >= ratios.len() / 2 + 1 {
        format!("HEALTHY — {}/{} ratios in safe zone", n_health, ratios.len())
    } else {
        "MIXED — some pressure points but no widespread stress".to_string()
    };

    let note = if ratios.is_empty() {
        "no cached financial statements — run FA".to_string()
    } else {
        String::new()
    };

    LeverageSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        total_debt,
        net_debt,
        ebitda_ttm,
        interest_expense_ttm: interest_ttm,
        total_equity,
        ratios,
        solvency_summary,
        note,
    }
}

// ── ADR-117 Round 10 — ACRL compute (earnings quality / accruals) ───────────

/// Compute an `AccrualsSnapshot` from cached financial statements. Walks the
/// last 4 quarterly income + cash-flow pairs, producing an FCF/NI ratio per
/// period plus a TTM roll-up and trend label.
pub fn compute_accruals_snapshot(
    symbol: &str,
    as_of: &str,
    statements: &FinancialStatements,
) -> AccrualsSnapshot {
    let mut periods: Vec<AccrualPeriod> = Vec::new();

    // Match income rows to cashflow rows by date. Keep order as-provided (newest first).
    for inc in statements.income_quarterly.iter().take(8) {
        let cf = statements.cashflow_quarterly.iter().find(|c| c.date == inc.date);
        let ni = inc.net_income;
        let fcf = cf.map(|c| c.free_cash_flow).unwrap_or(0.0);
        if ni == 0.0 && fcf == 0.0 { continue; }
        let ratio = if ni != 0.0 { fcf / ni } else { 0.0 };
        let conv_pct = ratio * 100.0;
        let accruals = ni - fcf;
        let quality_label = if ni <= 0.0 {
            "NEGATIVE_NI".to_string()
        } else if conv_pct >= 90.0 {
            "HIGH".to_string()
        } else if conv_pct >= 60.0 {
            "MEDIUM".to_string()
        } else {
            "LOW".to_string()
        };
        periods.push(AccrualPeriod {
            period: inc.period.clone(),
            date: inc.date.clone(),
            net_income: ni,
            free_cash_flow: fcf,
            fcf_to_ni_ratio: ratio,
            cash_conversion_pct: conv_pct,
            accruals,
            quality_label,
        });
    }

    // TTM roll-up from the last 4 quarters.
    let ttm_ni: f64 = periods.iter().take(4).map(|p| p.net_income).sum();
    let ttm_fcf: f64 = periods.iter().take(4).map(|p| p.free_cash_flow).sum();
    let ttm_conv_pct = if ttm_ni != 0.0 { ttm_fcf / ttm_ni * 100.0 } else { 0.0 };

    let avg_conv_pct: f64 = if !periods.is_empty() {
        periods.iter().map(|p| p.cash_conversion_pct).sum::<f64>() / periods.len() as f64
    } else { 0.0 };

    // Trend: compare recent-2 average vs older-2 average.
    let trend_label = if periods.len() < 4 {
        "INSUFFICIENT".to_string()
    } else {
        let recent: f64 = periods.iter().take(2).map(|p| p.cash_conversion_pct).sum::<f64>() / 2.0;
        let older: f64 = periods.iter().skip(2).take(2).map(|p| p.cash_conversion_pct).sum::<f64>() / 2.0;
        let delta = recent - older;
        if delta.abs() < 5.0 { "STABLE".to_string() }
        else if delta > 0.0 { "IMPROVING".to_string() }
        else { "DETERIORATING".to_string() }
    };

    let note = if periods.is_empty() {
        "no cached quarterly statements — run FA".to_string()
    } else {
        String::new()
    };

    AccrualsSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        ttm_net_income: ttm_ni,
        ttm_free_cash_flow: ttm_fcf,
        ttm_cash_conversion_pct: ttm_conv_pct,
        avg_cash_conversion_pct: avg_conv_pct,
        periods,
        trend_label,
        note,
    }
}

// ── ADR-117 Round 10 — RVOL compute (realized volatility cone) ──────────────

/// Compute a `RealizedVolSnapshot` from oldest-first daily bars. Produces
/// rolling 20d / 60d / 120d / 252d realized volatility (annualized stdev of
/// daily log-returns × √252) plus a cone percentile for each window
/// (where does today's RV rank against the full history of that window?),
/// and — when `current_atm_iv_pct > 0` — an IV / RV gap and ratio.
pub fn compute_realized_vol_snapshot(
    symbol: &str,
    as_of: &str,
    bars_oldest_first: &[HistoricalPriceRow],
    current_atm_iv_pct: f64,
) -> RealizedVolSnapshot {
    if bars_oldest_first.len() < 25 {
        return RealizedVolSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            regime_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥25 daily bars; run HP first".into(),
            ..Default::default()
        };
    }

    let last_close = bars_oldest_first.last().map(|b| b.close).unwrap_or(0.0);

    let mut log_returns: Vec<f64> = Vec::with_capacity(bars_oldest_first.len() - 1);
    for w in bars_oldest_first.windows(2) {
        if w[0].close > 0.0 && w[1].close > 0.0 {
            log_returns.push((w[1].close / w[0].close).ln());
        }
    }

    let stdev = |xs: &[f64]| -> f64 {
        if xs.len() < 2 { return 0.0; }
        let mean = xs.iter().sum::<f64>() / xs.len() as f64;
        let var: f64 = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (xs.len() as f64 - 1.0);
        var.sqrt()
    };
    let ann_vol_pct = |xs: &[f64]| -> f64 { stdev(xs) * (252.0_f64).sqrt() * 100.0 };

    let rolling_vols = |window: usize| -> (f64, Vec<f64>) {
        if log_returns.len() < window {
            return (0.0, Vec::new());
        }
        let mut series: Vec<f64> = Vec::new();
        for i in window..=log_returns.len() {
            let slice = &log_returns[i - window..i];
            series.push(ann_vol_pct(slice));
        }
        let latest = *series.last().unwrap_or(&0.0);
        (latest, series)
    };

    let specs = [
        ("20d", 20usize),
        ("60d", 60usize),
        ("120d", 120usize),
        ("252d", 252usize),
    ];
    let mut windows: Vec<RealizedVolWindow> = Vec::new();
    let mut rv_20d = 0.0;
    for (label, n) in specs.iter() {
        let (latest, series) = rolling_vols(*n);
        if series.is_empty() { continue; }
        if *label == "20d" { rv_20d = latest; }
        // Percentile rank of `latest` within its own rolling history.
        let count_below = series.iter().filter(|v| **v < latest).count();
        let pct = (count_below as f64 / series.len() as f64) * 100.0;
        windows.push(RealizedVolWindow {
            label: (*label).to_string(),
            trading_days: *n,
            realized_vol_pct: latest,
            percentile: pct,
            n_observations: series.len(),
        });
    }

    let (iv_rv_gap, iv_rv_ratio, regime_label) = if current_atm_iv_pct > 0.0 && rv_20d > 0.0 {
        let gap = current_atm_iv_pct - rv_20d;
        let ratio = current_atm_iv_pct / rv_20d;
        let label = if ratio < 0.95 { "CHEAP_IV".to_string() }
                    else if ratio > 1.15 { "RICH_IV".to_string() }
                    else { "FAIR_IV".to_string() };
        (gap, ratio, label)
    } else if rv_20d > 0.0 {
        (0.0, 0.0, "NO_IV_REFERENCE".to_string())
    } else {
        (0.0, 0.0, "INSUFFICIENT_DATA".to_string())
    };

    let note = if windows.is_empty() {
        "need more bars for rolling windows".to_string()
    } else {
        String::new()
    };

    RealizedVolSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        last_close,
        current_atm_iv_pct,
        iv_rv_gap_pct: iv_rv_gap,
        iv_rv_ratio,
        windows,
        regime_label,
        note,
    }
}

// ── ADR-117 Round 10 — FCFY compute (FCF yield + dividend coverage) ─────────

/// Compute an `FcfYieldSnapshot` from cached financial statements + market cap.
/// Builds per-annual FCF yield / dividend coverage rows, rolls TTM from the last
/// 4 quarterly cash flow statements, computes a 5-year FCF CAGR when enough
/// annual rows exist, and emits a dividend-sustainability label.
pub fn compute_fcf_yield_snapshot(
    symbol: &str,
    as_of: &str,
    statements: &FinancialStatements,
    market_cap: f64,
    stock_price: f64,
) -> FcfYieldSnapshot {
    let mut periods: Vec<FcfYieldPeriod> = Vec::new();

    for cf in statements.cashflow_annual.iter().take(5) {
        let ni = statements.income_annual.iter()
            .find(|i| i.date == cf.date).map(|i| i.net_income).unwrap_or(0.0);
        let div = cf.dividends_paid.abs();
        let payout_fcf = if cf.free_cash_flow > 0.0 { div / cf.free_cash_flow * 100.0 } else { 0.0 };
        let payout_ni = if ni > 0.0 { div / ni * 100.0 } else { 0.0 };
        let yield_pct = if market_cap > 0.0 { cf.free_cash_flow / market_cap * 100.0 } else { 0.0 };
        periods.push(FcfYieldPeriod {
            period: cf.period.clone(),
            date: cf.date.clone(),
            free_cash_flow: cf.free_cash_flow,
            dividends_paid: div,
            payout_from_fcf_pct: payout_fcf,
            payout_from_ni_pct: payout_ni,
            fcf_yield_pct: yield_pct,
        });
    }

    // TTM roll-up from the last 4 quarterly cash flow statements.
    let q_cf = &statements.cashflow_quarterly;
    let ttm_fcf: f64 = q_cf.iter().take(4).map(|c| c.free_cash_flow).sum();
    let ttm_div: f64 = q_cf.iter().take(4).map(|c| c.dividends_paid.abs()).sum();
    let ttm_ni: f64 = statements.income_quarterly.iter().take(4).map(|i| i.net_income).sum();
    let ttm_fcf_yield = if market_cap > 0.0 { ttm_fcf / market_cap * 100.0 } else { 0.0 };
    let ttm_div_yield = if market_cap > 0.0 { ttm_div / market_cap * 100.0 } else { 0.0 };
    let ttm_payout_fcf = if ttm_fcf > 0.0 { ttm_div / ttm_fcf * 100.0 } else { 0.0 };
    let ttm_payout_ni = if ttm_ni > 0.0 { ttm_div / ttm_ni * 100.0 } else { 0.0 };

    // 5-year FCF CAGR (oldest → newest) when we have ≥5 annual rows.
    let fcf_cagr = if statements.cashflow_annual.len() >= 5 {
        let sorted_rev: Vec<&CashFlowStatement> = {
            let mut v: Vec<&CashFlowStatement> = statements.cashflow_annual.iter().take(5).collect();
            v.sort_by(|a, b| a.date.cmp(&b.date));
            v
        };
        let start = sorted_rev.first().map(|c| c.free_cash_flow).unwrap_or(0.0);
        let end = sorted_rev.last().map(|c| c.free_cash_flow).unwrap_or(0.0);
        if start > 0.0 && end > 0.0 {
            ((end / start).powf(1.0 / 4.0) - 1.0) * 100.0
        } else { 0.0 }
    } else { 0.0 };

    let sustainability_label = if ttm_div <= 0.0 {
        "NO_DIVIDEND".to_string()
    } else if ttm_fcf <= 0.0 || ttm_payout_fcf > 100.0 {
        "UNSUSTAINABLE".to_string()
    } else if ttm_payout_fcf > 75.0 {
        "STRETCHED".to_string()
    } else {
        "SAFE".to_string()
    };

    let note = if periods.is_empty() && ttm_fcf == 0.0 {
        "no cached cash-flow statements — run FA".to_string()
    } else if market_cap <= 0.0 {
        format!("market cap missing — yield pct not computed (last ${:.2})", stock_price)
    } else {
        String::new()
    };

    FcfYieldSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        market_cap,
        ttm_free_cash_flow: ttm_fcf,
        ttm_dividends_paid: ttm_div,
        ttm_fcf_yield_pct: ttm_fcf_yield,
        ttm_dividend_yield_pct: ttm_div_yield,
        ttm_payout_from_fcf_pct: ttm_payout_fcf,
        ttm_payout_from_ni_pct: ttm_payout_ni,
        fcf_cagr_5y_pct: fcf_cagr,
        periods,
        sustainability_label,
        note,
    }
}

// ── ADR-117 Round 10 — SHRT compute (short interest + days-to-cover) ────────

/// Compute a `ShortInterestSnapshot` from the Fundamentals short fields plus
/// daily HP bars. Days-to-cover comes from `short_shares / avg_daily_volume_20d`.
pub fn compute_short_interest_snapshot(
    symbol: &str,
    as_of: &str,
    shares_outstanding: f64,
    shares_float: f64,
    short_percent_of_float: f64,
    short_ratio_reported: f64,
    bars_oldest_first: &[HistoricalPriceRow],
) -> ShortInterestSnapshot {
    let short_shares = if shares_float > 0.0 && short_percent_of_float > 0.0 {
        shares_float * (short_percent_of_float / 100.0)
    } else { 0.0 };

    // 20-day average daily volume from the tail of the bar series.
    let avg_dv_20d = if bars_oldest_first.len() >= 20 {
        let tail = &bars_oldest_first[bars_oldest_first.len() - 20..];
        tail.iter().map(|b| b.volume).sum::<f64>() / 20.0
    } else if !bars_oldest_first.is_empty() {
        bars_oldest_first.iter().map(|b| b.volume).sum::<f64>() / bars_oldest_first.len() as f64
    } else { 0.0 };

    let days_to_cover = if avg_dv_20d > 0.0 && short_shares > 0.0 {
        short_shares / avg_dv_20d
    } else { 0.0 };

    let squeeze_risk_label = if short_shares <= 0.0 || avg_dv_20d <= 0.0 {
        "INSUFFICIENT_DATA".to_string()
    } else if short_percent_of_float >= 30.0 || days_to_cover >= 10.0 {
        "EXTREME".to_string()
    } else if short_percent_of_float >= 20.0 || days_to_cover >= 7.0 {
        "HIGH".to_string()
    } else if short_percent_of_float >= 10.0 || days_to_cover >= 4.0 {
        "ELEVATED".to_string()
    } else {
        "LOW".to_string()
    };

    let note = if short_shares <= 0.0 {
        "no short data in Fundamentals — run EVSCRAPE".to_string()
    } else if bars_oldest_first.is_empty() {
        "no bar volumes — run HP first".to_string()
    } else {
        String::new()
    };

    ShortInterestSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        shares_outstanding,
        shares_float,
        short_shares,
        short_percent_of_float,
        avg_daily_volume_20d: avg_dv_20d,
        days_to_cover,
        short_ratio_reported,
        utilization_proxy_pct: short_percent_of_float,
        squeeze_risk_label,
        note,
    }
}

// ── ADR-118 Godel Parity Round 11 compute fns ──────────────────────────────

/// ALTZ — classic Altman Z-score for public manufacturers.
/// Z = 1.2(WC/TA) + 1.4(RE/TA) + 3.3(EBIT/TA) + 0.6(MVE/TL) + 1.0(Sales/TA)
pub fn compute_altman_z_snapshot(
    symbol: &str,
    as_of: &str,
    statements: &FinancialStatements,
    market_value_equity: f64,
) -> AltmanZSnapshot {
    let bal = statements.balance_annual.first()
        .or_else(|| statements.balance_quarterly.first());
    let inc = statements.income_annual.first()
        .or_else(|| statements.income_quarterly.first());

    let bal = match bal {
        Some(b) => b,
        None => {
            return AltmanZSnapshot {
                symbol: symbol.to_uppercase(),
                as_of: as_of.to_string(),
                zone: "INSUFFICIENT_DATA".to_string(),
                note: "no balance sheet cached — run FA first".to_string(),
                ..Default::default()
            };
        }
    };
    let inc = match inc {
        Some(i) => i,
        None => {
            return AltmanZSnapshot {
                symbol: symbol.to_uppercase(),
                as_of: as_of.to_string(),
                zone: "INSUFFICIENT_DATA".to_string(),
                note: "no income statement cached — run FA first".to_string(),
                ..Default::default()
            };
        }
    };

    let wc = bal.total_current_assets - bal.total_current_liabilities;
    let re = bal.retained_earnings;
    let ebit = inc.operating_income;
    let mve = market_value_equity;
    let sales = inc.revenue;
    let ta = bal.total_assets;
    let tl = bal.total_liabilities;

    if ta <= 0.0 || tl <= 0.0 {
        return AltmanZSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            total_assets: ta,
            total_liabilities: tl,
            zone: "INSUFFICIENT_DATA".to_string(),
            note: "non-positive assets or liabilities".to_string(),
            ..Default::default()
        };
    }

    let a = wc / ta;
    let b = re / ta;
    let c = ebit / ta;
    let d = if tl > 0.0 { mve / tl } else { 0.0 };
    let e = sales / ta;

    let components = vec![
        AltmanComponent { name: "A: WC/TA".into(), ratio: a, coefficient: 1.2, contribution: 1.2 * a, note: "liquidity".into() },
        AltmanComponent { name: "B: RE/TA".into(), ratio: b, coefficient: 1.4, contribution: 1.4 * b, note: "cumulative profitability".into() },
        AltmanComponent { name: "C: EBIT/TA".into(), ratio: c, coefficient: 3.3, contribution: 3.3 * c, note: "operating leverage".into() },
        AltmanComponent { name: "D: MVE/TL".into(), ratio: d, coefficient: 0.6, contribution: 0.6 * d, note: if mve > 0.0 { "solvency" } else { "no market cap" }.into() },
        AltmanComponent { name: "E: Sales/TA".into(), ratio: e, coefficient: 1.0, contribution: 1.0 * e, note: "asset turnover".into() },
    ];

    let z_score: f64 = components.iter().map(|c| c.contribution).sum();
    let zone = if mve <= 0.0 {
        "INSUFFICIENT_DATA".to_string()
    } else if z_score >= 2.99 {
        "SAFE".to_string()
    } else if z_score >= 1.81 {
        "GRAY".to_string()
    } else {
        "DISTRESS".to_string()
    };

    let note = if mve <= 0.0 {
        "no market cap from Fundamentals — D component is zero, zone reports as INSUFFICIENT_DATA".to_string()
    } else {
        String::new()
    };

    AltmanZSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        working_capital: wc,
        retained_earnings: re,
        ebit,
        market_value_equity: mve,
        sales,
        total_assets: ta,
        total_liabilities: tl,
        z_score,
        zone,
        components,
        note,
    }
}

/// PTFS — Piotroski F-score (9-point quality checklist).
/// Requires at least 2 annual periods of FinancialStatements.
pub fn compute_piotroski_snapshot(
    symbol: &str,
    as_of: &str,
    statements: &FinancialStatements,
) -> PiotroskiSnapshot {
    if statements.income_annual.len() < 2
        || statements.balance_annual.len() < 2
        || statements.cashflow_annual.is_empty()
    {
        return PiotroskiSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            strength_label: "INSUFFICIENT_DATA".to_string(),
            note: "need ≥2 annual statements — run FA first".to_string(),
            ..Default::default()
        };
    }

    let inc_cur = &statements.income_annual[0];
    let inc_prev = &statements.income_annual[1];
    let bal_cur = &statements.balance_annual[0];
    let bal_prev = &statements.balance_annual[1];
    let cf_cur = &statements.cashflow_annual[0];

    let ni = inc_cur.net_income;
    let cfo = cf_cur.cash_from_operations;
    let ta_cur = bal_cur.total_assets.max(1.0);
    let ta_prev = bal_prev.total_assets.max(1.0);
    let roa_cur = ni / ta_cur;
    let roa_prev = inc_prev.net_income / ta_prev;
    let accrual_proxy = cfo - ni;

    let ltd_cur = bal_cur.long_term_debt / ta_cur;
    let ltd_prev = bal_prev.long_term_debt / ta_prev;
    let cr_cur = if bal_cur.total_current_liabilities > 0.0 {
        bal_cur.total_current_assets / bal_cur.total_current_liabilities
    } else { 0.0 };
    let cr_prev = if bal_prev.total_current_liabilities > 0.0 {
        bal_prev.total_current_assets / bal_prev.total_current_liabilities
    } else { 0.0 };
    let shares_cur = inc_cur.weighted_shares_out;
    let shares_prev = inc_prev.weighted_shares_out;

    let gm_cur = if inc_cur.revenue > 0.0 { inc_cur.gross_profit / inc_cur.revenue } else { 0.0 };
    let gm_prev = if inc_prev.revenue > 0.0 { inc_prev.gross_profit / inc_prev.revenue } else { 0.0 };
    let at_cur = if ta_cur > 0.0 { inc_cur.revenue / ta_cur } else { 0.0 };
    let at_prev = if ta_prev > 0.0 { inc_prev.revenue / ta_prev } else { 0.0 };

    let mut checks: Vec<PiotroskiCheck> = Vec::new();

    // Profitability (4)
    checks.push(PiotroskiCheck {
        category: "Profitability".into(), name: "Positive Net Income".into(),
        passed: ni > 0.0, value_current: ni, value_prior: 0.0,
        note: String::new(),
    });
    checks.push(PiotroskiCheck {
        category: "Profitability".into(), name: "Positive OCF".into(),
        passed: cfo > 0.0, value_current: cfo, value_prior: 0.0,
        note: String::new(),
    });
    checks.push(PiotroskiCheck {
        category: "Profitability".into(), name: "ROA ↑".into(),
        passed: roa_cur > roa_prev, value_current: roa_cur, value_prior: roa_prev,
        note: String::new(),
    });
    checks.push(PiotroskiCheck {
        category: "Profitability".into(), name: "OCF > NI (accrual)".into(),
        passed: cfo > ni, value_current: cfo, value_prior: ni,
        note: format!("accrual = {:.0}", accrual_proxy),
    });

    // Leverage / Liquidity (3)
    checks.push(PiotroskiCheck {
        category: "Leverage/Liquidity".into(), name: "LT Debt / Assets ↓".into(),
        passed: ltd_cur < ltd_prev, value_current: ltd_cur, value_prior: ltd_prev,
        note: String::new(),
    });
    checks.push(PiotroskiCheck {
        category: "Leverage/Liquidity".into(), name: "Current Ratio ↑".into(),
        passed: cr_cur > cr_prev, value_current: cr_cur, value_prior: cr_prev,
        note: String::new(),
    });
    checks.push(PiotroskiCheck {
        category: "Leverage/Liquidity".into(), name: "No new share issue".into(),
        passed: shares_cur <= shares_prev * 1.005, // 0.5% tolerance for option grants
        value_current: shares_cur, value_prior: shares_prev,
        note: String::new(),
    });

    // Operating Efficiency (2)
    checks.push(PiotroskiCheck {
        category: "Operating Efficiency".into(), name: "Gross Margin ↑".into(),
        passed: gm_cur > gm_prev, value_current: gm_cur, value_prior: gm_prev,
        note: String::new(),
    });
    checks.push(PiotroskiCheck {
        category: "Operating Efficiency".into(), name: "Asset Turnover ↑".into(),
        passed: at_cur > at_prev, value_current: at_cur, value_prior: at_prev,
        note: String::new(),
    });

    let profitability_score: i32 = checks.iter().take(4).filter(|c| c.passed).count() as i32;
    let leverage_score: i32 = checks.iter().skip(4).take(3).filter(|c| c.passed).count() as i32;
    let efficiency_score: i32 = checks.iter().skip(7).take(2).filter(|c| c.passed).count() as i32;
    let f_score = profitability_score + leverage_score + efficiency_score;

    let strength_label = if f_score >= 7 {
        "STRONG".to_string()
    } else if f_score <= 3 {
        "WEAK".to_string()
    } else {
        "MIXED".to_string()
    };

    PiotroskiSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        current_period: inc_cur.date.clone(),
        prior_period: inc_prev.date.clone(),
        f_score,
        strength_label,
        profitability_score,
        leverage_score,
        efficiency_score,
        checks,
        note: String::new(),
    }
}

/// VOLE — OHLC volatility estimators (Parkinson / Garman-Klass / Rogers-Satchell / Yang-Zhang).
/// Needs ≥20 bars with valid OHLC. Uses the tail of the bar series.
pub fn compute_ohlc_vol_snapshot(
    symbol: &str,
    as_of: &str,
    bars_oldest_first: &[HistoricalPriceRow],
    window_days: usize,
) -> OhlcVolSnapshot {
    let needed = window_days.max(20);
    if bars_oldest_first.len() < needed {
        return OhlcVolSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            trading_days: bars_oldest_first.len(),
            note: format!("need ≥{} bars, have {}", needed, bars_oldest_first.len()),
            ..Default::default()
        };
    }

    let tail_start = bars_oldest_first.len() - needed;
    let tail = &bars_oldest_first[tail_start..];
    let n = tail.len();
    let ann = 252.0f64;

    // Valid bars: positive OHLC and high >= low, high >= open, etc.
    let valid: Vec<&HistoricalPriceRow> = tail.iter()
        .filter(|b| b.open > 0.0 && b.high > 0.0 && b.low > 0.0 && b.close > 0.0 && b.high >= b.low)
        .collect();
    if valid.len() < 20 {
        return OhlcVolSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            trading_days: valid.len(),
            note: "fewer than 20 bars with valid OHLC".to_string(),
            ..Default::default()
        };
    }

    // Close-to-close realized vol (baseline).
    let log_ret_cc: Vec<f64> = valid.windows(2)
        .map(|w| (w[1].close / w[0].close).ln())
        .collect();
    let mean_cc: f64 = log_ret_cc.iter().sum::<f64>() / log_ret_cc.len() as f64;
    let var_cc: f64 = log_ret_cc.iter().map(|r| (r - mean_cc).powi(2)).sum::<f64>() / (log_ret_cc.len() - 1).max(1) as f64;
    let ctc_daily = var_cc.sqrt();
    let ctc = ctc_daily * ann.sqrt() * 100.0;

    // Parkinson (range-based).
    // σ² = (1 / (4·ln2·N)) × Σ ln(H/L)²
    let ln2 = 2.0f64.ln();
    let park_sum: f64 = valid.iter()
        .filter(|b| b.low > 0.0)
        .map(|b| (b.high / b.low).ln().powi(2))
        .sum();
    let park_var_daily = park_sum / (4.0 * ln2 * valid.len() as f64);
    let park = park_var_daily.sqrt() * ann.sqrt() * 100.0;

    // Garman-Klass.
    // σ² = (1/N) × Σ [0.5·ln(H/L)² − (2·ln2 − 1)·ln(C/O)²]
    let gk_sum: f64 = valid.iter()
        .filter(|b| b.low > 0.0 && b.open > 0.0)
        .map(|b| {
            let hl = (b.high / b.low).ln();
            let co = (b.close / b.open).ln();
            0.5 * hl * hl - (2.0 * ln2 - 1.0) * co * co
        })
        .sum();
    let gk_var_daily = gk_sum / valid.len() as f64;
    let gk = gk_var_daily.max(0.0).sqrt() * ann.sqrt() * 100.0;

    // Rogers-Satchell (drift-independent).
    // σ² = (1/N) × Σ [ln(H/C)·ln(H/O) + ln(L/C)·ln(L/O)]
    let rs_sum: f64 = valid.iter()
        .filter(|b| b.low > 0.0 && b.open > 0.0 && b.close > 0.0)
        .map(|b| {
            let hc = (b.high / b.close).ln();
            let ho = (b.high / b.open).ln();
            let lc = (b.low / b.close).ln();
            let lo = (b.low / b.open).ln();
            hc * ho + lc * lo
        })
        .sum();
    let rs_var_daily = rs_sum / valid.len() as f64;
    let rs = rs_var_daily.max(0.0).sqrt() * ann.sqrt() * 100.0;

    // Yang-Zhang = overnight_var + k × open-to-close_var + (1-k) × RS_var
    // k = 0.34 / (1.34 + (N+1)/(N-1)). Overnight returns use previous_close → open.
    let overnight_rets: Vec<f64> = valid.windows(2)
        .map(|w| (w[1].open / w[0].close).ln())
        .collect();
    let on_mean: f64 = overnight_rets.iter().sum::<f64>() / overnight_rets.len().max(1) as f64;
    let on_var: f64 = overnight_rets.iter().map(|r| (r - on_mean).powi(2)).sum::<f64>()
        / (overnight_rets.len().saturating_sub(1)).max(1) as f64;
    let oc_rets: Vec<f64> = valid.iter().map(|b| (b.close / b.open).ln()).collect();
    let oc_mean: f64 = oc_rets.iter().sum::<f64>() / oc_rets.len() as f64;
    let oc_var: f64 = oc_rets.iter().map(|r| (r - oc_mean).powi(2)).sum::<f64>()
        / (oc_rets.len() - 1).max(1) as f64;
    let n_f = n as f64;
    let k = 0.34 / (1.34 + (n_f + 1.0) / (n_f - 1.0).max(1.0));
    let yz_var_daily = on_var + k * oc_var + (1.0 - k) * rs_var_daily.max(0.0);
    let yz = yz_var_daily.max(0.0).sqrt() * ann.sqrt() * 100.0;

    let make_row = |name: &str, vol: f64| VolEstimator {
        name: name.to_string(),
        annualized_vol_pct: vol,
        efficiency_vs_close: if ctc > 0.0 { ctc / vol.max(0.0001) } else { 1.0 },
        note: String::new(),
    };

    let estimators = vec![
        make_row("Close-to-Close", ctc),
        make_row("Parkinson", park),
        make_row("Garman-Klass", gk),
        make_row("Rogers-Satchell", rs),
        make_row("Yang-Zhang", yz),
    ];

    let (preferred_label, preferred) = if yz > 0.0 {
        ("Yang-Zhang".to_string(), yz)
    } else if park > 0.0 {
        ("Parkinson".to_string(), park)
    } else {
        ("Close-to-Close".to_string(), ctc)
    };

    OhlcVolSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        trading_days: valid.len(),
        estimators,
        preferred_estimate_pct: preferred,
        preferred_label,
        note: String::new(),
    }
}

/// EPSB — EPS beat streak & surprise analysis over cached earnings-surprise history.
pub fn compute_eps_beat_snapshot(
    symbol: &str,
    as_of: &str,
    reports: &[EarningsSurprise],
) -> EpsBeatSnapshot {
    if reports.is_empty() {
        return EpsBeatSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            bias_label: "NEUTRAL".to_string(),
            trend_label: "STABLE".to_string(),
            note: "no EPS surprise history — run EPS first".to_string(),
            ..Default::default()
        };
    }

    // Sort oldest-first by date string (YYYY-MM-DD sorts lexicographically).
    let mut sorted: Vec<EarningsSurprise> = reports.to_vec();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));

    let beats = sorted.iter().filter(|r| r.surprise > 0.0).count();
    let misses = sorted.iter().filter(|r| r.surprise < 0.0).count();
    let inlines = sorted.iter().filter(|r| r.surprise == 0.0).count();
    let total = sorted.len();

    // Beat rate (beats / total).
    let beat_rate_pct = (beats as f64 / total as f64) * 100.0;

    // Current streak: walk from the newest report backwards.
    let mut current_streak: i32 = 0;
    let newest = sorted.last().map(|r| r.surprise).unwrap_or(0.0);
    let direction = if newest > 0.0 { 1i32 } else if newest < 0.0 { -1i32 } else { 0i32 };
    if direction != 0 {
        for r in sorted.iter().rev() {
            if r.surprise > 0.0 && direction == 1 {
                current_streak += 1;
            } else if r.surprise < 0.0 && direction == -1 {
                current_streak -= 1;
            } else {
                break;
            }
        }
    }

    // Longest streaks of each kind.
    let mut longest_beat = 0usize;
    let mut longest_miss = 0usize;
    let mut run_beat = 0usize;
    let mut run_miss = 0usize;
    for r in sorted.iter() {
        if r.surprise > 0.0 {
            run_beat += 1;
            if run_beat > longest_beat { longest_beat = run_beat; }
            run_miss = 0;
        } else if r.surprise < 0.0 {
            run_miss += 1;
            if run_miss > longest_miss { longest_miss = run_miss; }
            run_beat = 0;
        } else {
            run_beat = 0;
            run_miss = 0;
        }
    }

    let avg_surprise_pct = sorted.iter().map(|r| r.surprise_pct).sum::<f64>() / total as f64;
    let mut sorted_pcts: Vec<f64> = sorted.iter().map(|r| r.surprise_pct).collect();
    sorted_pcts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_surprise_pct = if sorted_pcts.len() % 2 == 0 {
        (sorted_pcts[sorted_pcts.len() / 2 - 1] + sorted_pcts[sorted_pcts.len() / 2]) / 2.0
    } else {
        sorted_pcts[sorted_pcts.len() / 2]
    };

    let recent_n = 4.min(sorted.len());
    let recent_slice = &sorted[sorted.len() - recent_n..];
    let recent_avg = recent_slice.iter().map(|r| r.surprise_pct).sum::<f64>() / recent_n as f64;

    let bias_label = if avg_surprise_pct > 2.0 {
        "POSITIVE".to_string()
    } else if avg_surprise_pct < -2.0 {
        "NEGATIVE".to_string()
    } else {
        "NEUTRAL".to_string()
    };

    let trend_label = if recent_avg > avg_surprise_pct + 1.0 {
        "ACCELERATING".to_string()
    } else if recent_avg < avg_surprise_pct - 1.0 {
        "DECELERATING".to_string()
    } else {
        "STABLE".to_string()
    };

    let latest = sorted.last().unwrap();

    EpsBeatSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        total_reports: total,
        beats,
        misses,
        inlines,
        beat_rate_pct,
        current_streak,
        longest_beat_streak: longest_beat,
        longest_miss_streak: longest_miss,
        avg_surprise_pct,
        median_surprise_pct,
        recent_avg_surprise_pct: recent_avg,
        bias_label,
        trend_label,
        latest_date: latest.date.clone(),
        latest_surprise_pct: latest.surprise_pct,
        note: String::new(),
    }
}

/// PTD — Price Target Dispersion & Implied Return from cached aggregates.
pub fn compute_price_target_dispersion(
    symbol: &str,
    as_of: &str,
    current_price: f64,
    target: Option<&PriceTarget>,
) -> PriceTargetDispersion {
    let target = match target {
        Some(t) => t,
        None => {
            return PriceTargetDispersion {
                symbol: symbol.to_uppercase(),
                as_of: as_of.to_string(),
                current_price,
                consensus_label: "NO_COVERAGE".to_string(),
                note: "no cached price target — run UPDG / PT first".to_string(),
                ..Default::default()
            };
        }
    };

    let dispersion_pct = if target.target_mean > 0.0 {
        (target.target_high - target.target_low) / target.target_mean * 100.0
    } else { 0.0 };
    let spread_pct = if current_price > 0.0 {
        (target.target_high - target.target_low) / current_price * 100.0
    } else { 0.0 };

    let implied_median = if current_price > 0.0 && target.target_median > 0.0 {
        (target.target_median - current_price) / current_price * 100.0
    } else { 0.0 };
    let implied_mean = if current_price > 0.0 && target.target_mean > 0.0 {
        (target.target_mean - current_price) / current_price * 100.0
    } else { 0.0 };
    let upside_high = if current_price > 0.0 && target.target_high > 0.0 {
        (target.target_high - current_price) / current_price * 100.0
    } else { 0.0 };
    let downside_low = if current_price > 0.0 && target.target_low > 0.0 {
        (target.target_low - current_price) / current_price * 100.0
    } else { 0.0 };

    let consensus_label = if target.num_analysts <= 0 || current_price <= 0.0 {
        "NO_COVERAGE".to_string()
    } else if implied_median >= 10.0 {
        "BULLISH".to_string()
    } else if implied_median <= -5.0 {
        "BEARISH".to_string()
    } else {
        "NEUTRAL".to_string()
    };

    let note = if target.num_analysts <= 0 {
        "target has zero analyst coverage".to_string()
    } else {
        String::new()
    };

    PriceTargetDispersion {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        current_price,
        target_high: target.target_high,
        target_low: target.target_low,
        target_mean: target.target_mean,
        target_median: target.target_median,
        num_analysts: target.num_analysts,
        dispersion_pct,
        spread_pct,
        implied_return_median_pct: implied_median,
        implied_return_mean_pct: implied_mean,
        upside_to_high_pct: upside_high,
        downside_to_low_pct: downside_low,
        consensus_label,
        note,
    }
}

// ── ADR-119 Godel Parity Round 12 compute fns ──────────────────────────────

pub(crate) fn parse_yyyy_mm_dd_to_days(s: &str) -> Option<i64> {
    // Crude julian-ish day number. We don't need calendar correctness — just
    // a monotone integer for sorting & window comparisons against "today".
    let parts: Vec<&str> = s.splitn(3, '-').collect();
    if parts.len() != 3 { return None; }
    let y: i64 = parts[0].parse().ok()?;
    let m: i64 = parts[1].parse().ok()?;
    let d: i64 = parts[2].parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) { return None; }
    Some(y * 372 + m * 31 + d)
}

/// MNGR — Insider Activity Bias score computed over a lookback window.
/// Buckets insider trades into buys/sells/other, computes gross/net values,
/// classifies bias from net-value direction and conviction from trade count.
pub fn compute_insider_activity_snapshot(
    symbol: &str,
    as_of: &str,
    trades: &[InsiderTrade],
    window_days: i32,
) -> InsiderActivitySnapshot {
    let sym = symbol.to_uppercase();
    let as_of_days = parse_yyyy_mm_dd_to_days(as_of);
    let cutoff_days = as_of_days.map(|d| d - window_days as i64);

    let in_window: Vec<&InsiderTrade> = trades.iter().filter(|t| {
        match (cutoff_days, parse_yyyy_mm_dd_to_days(&t.transaction_date)) {
            (Some(c), Some(td)) => td >= c,
            _ => true, // if either date unparsable, include it
        }
    }).collect();

    if in_window.is_empty() {
        return InsiderActivitySnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            window_days,
            bias_label: "NO_ACTIVITY".to_string(),
            conviction_label: "NONE".to_string(),
            note: "no insider trades in lookback window — run INS first".to_string(),
            ..Default::default()
        };
    }

    let classify = |t: &InsiderTrade| -> &'static str {
        let upper = t.transaction_type.to_uppercase();
        let disp = t.acquisition_disposition.to_uppercase();
        if upper.contains("P-PURCHASE") || upper.starts_with("P ") || upper == "P" || upper.contains("PURCHASE") {
            "buy"
        } else if upper.contains("S-SALE") || upper.starts_with("S ") || upper == "S" || upper.contains("SALE") {
            "sell"
        } else if disp == "A" {
            "buy"
        } else if disp == "D" {
            "sell"
        } else {
            "other"
        }
    };

    let mut buy_count = 0usize;
    let mut sell_count = 0usize;
    let mut other_count = 0usize;
    let mut gross_buy_value = 0.0f64;
    let mut gross_sell_value = 0.0f64;
    let mut net_shares = 0.0f64;
    let mut insiders: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut latest_date = String::new();
    let mut latest_days: i64 = i64::MIN;

    for t in &in_window {
        let v = if t.value_usd.abs() > 0.0 {
            t.value_usd.abs()
        } else {
            (t.shares * t.price).abs()
        };
        match classify(t) {
            "buy" => {
                buy_count += 1;
                gross_buy_value += v;
                net_shares += t.shares.abs();
            }
            "sell" => {
                sell_count += 1;
                gross_sell_value += v;
                net_shares -= t.shares.abs();
            }
            _ => other_count += 1,
        }
        if !t.reporting_name.trim().is_empty() {
            insiders.insert(t.reporting_name.trim().to_lowercase());
        }
        if let Some(td) = parse_yyyy_mm_dd_to_days(&t.transaction_date) {
            if td > latest_days {
                latest_days = td;
                latest_date = t.transaction_date.clone();
            }
        }
    }

    let net_value = gross_buy_value - gross_sell_value;
    let buy_sell_ratio = if sell_count > 0 { buy_count as f64 / sell_count as f64 } else { buy_count as f64 };

    let total_trades = in_window.len();
    let unique = insiders.len();

    let bias = if buy_count == 0 && sell_count == 0 {
        "NO_ACTIVITY"
    } else if net_value > 0.0 && buy_count >= sell_count {
        "BULLISH"
    } else if net_value < 0.0 && sell_count > buy_count {
        "BEARISH"
    } else {
        "NEUTRAL"
    };

    let total_gross = gross_buy_value + gross_sell_value;
    let conviction = if total_gross <= 0.0 || unique == 0 {
        "NONE"
    } else if unique >= 3 && total_gross >= 1_000_000.0 {
        "HIGH"
    } else if unique >= 2 || total_gross >= 250_000.0 {
        "MEDIUM"
    } else {
        "LOW"
    };

    InsiderActivitySnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        window_days,
        total_trades,
        buy_count,
        sell_count,
        other_count,
        unique_insiders: unique,
        gross_buy_value_usd: gross_buy_value,
        gross_sell_value_usd: gross_sell_value,
        net_value_usd: net_value,
        buy_sell_ratio,
        net_shares,
        latest_trade_date: latest_date,
        bias_label: bias.to_string(),
        conviction_label: conviction.to_string(),
        note: String::new(),
    }
}

/// DIVG — Dividend Growth Analysis computed over cached DVD rows.
/// Buckets payments by calendar year, computes CAGRs and consistency.
pub fn compute_divg_snapshot(
    symbol: &str,
    as_of: &str,
    dividends: &[DividendRecord],
) -> DivgSnapshot {
    let sym = symbol.to_uppercase();

    if dividends.is_empty() {
        return DivgSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            trend_label: "NO_HISTORY".to_string(),
            note: "no dividend history cached — run DVD first".to_string(),
            ..Default::default()
        };
    }

    // Sort by ex_date ascending
    let mut sorted: Vec<&DividendRecord> = dividends.iter()
        .filter(|d| d.amount > 0.0 && !d.ex_date.is_empty())
        .collect();
    sorted.sort_by(|a, b| a.ex_date.cmp(&b.ex_date));

    if sorted.is_empty() {
        return DivgSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            trend_label: "NO_HISTORY".to_string(),
            note: "dividend rows all zero or missing ex_date".to_string(),
            ..Default::default()
        };
    }

    let first_payment_date = sorted.first().unwrap().ex_date.clone();
    let latest_payment_date = sorted.last().unwrap().ex_date.clone();
    let latest_amount = sorted.last().unwrap().amount;
    let total_payments = sorted.len();

    // Annualized = sum of most recent up-to-4 payments
    let tail_n = sorted.len().min(4);
    let annualized: f64 = sorted.iter().rev().take(tail_n).map(|d| d.amount).sum();

    // Bucket by year
    let mut by_year: std::collections::BTreeMap<i32, (f64, usize)> = std::collections::BTreeMap::new();
    for d in &sorted {
        let year: i32 = match d.ex_date.splitn(2, '-').next().and_then(|y| y.parse().ok()) {
            Some(y) => y,
            None => continue,
        };
        let e = by_year.entry(year).or_insert((0.0, 0));
        e.0 += d.amount;
        e.1 += 1;
    }

    // Determine current year from as_of
    let as_of_year: Option<i32> = as_of.splitn(2, '-').next().and_then(|y| y.parse().ok());

    // Exclude the in-progress current year when it's incomplete (fewer payments than prior year).
    // We still keep prior years as-is. Sort into Vec<(year, amount, count)>.
    let mut years: Vec<(i32, f64, usize)> = by_year.iter().map(|(y, (a, c))| (*y, *a, *c)).collect();
    if let Some(cur) = as_of_year {
        if let Some(last) = years.last() {
            if last.0 == cur {
                let prior_avg_count = if years.len() >= 2 {
                    years[..years.len()-1].iter().rev().take(3).map(|r| r.2).sum::<usize>() as f64 / years.len().min(3) as f64
                } else { 0.0 };
                if (last.2 as f64) < prior_avg_count.max(1.0) * 0.75 {
                    years.pop(); // drop incomplete current year from growth analysis
                }
            }
        }
    }

    let mut annual_rows: Vec<DivgAnnualRow> = Vec::with_capacity(years.len());
    for (i, (y, a, c)) in years.iter().enumerate() {
        let growth = if i == 0 { 0.0 } else {
            let prior = years[i-1].1;
            if prior > 0.0 { (a - prior) / prior * 100.0 } else { 0.0 }
        };
        annual_rows.push(DivgAnnualRow { year: *y, total_amount: *a, payment_count: *c, growth_pct: growth });
    }

    let years_covered = annual_rows.len();
    let cagr = |from: f64, to: f64, n: f64| -> f64 {
        if from <= 0.0 || to <= 0.0 || n <= 0.0 { 0.0 }
        else { ((to / from).powf(1.0 / n) - 1.0) * 100.0 }
    };

    let cagr_1y = if years_covered >= 2 {
        annual_rows.last().unwrap().growth_pct
    } else { 0.0 };
    let cagr_3y = if years_covered >= 4 {
        let n = years_covered;
        cagr(annual_rows[n-4].total_amount, annual_rows[n-1].total_amount, 3.0)
    } else { 0.0 };
    let cagr_5y = if years_covered >= 6 {
        let n = years_covered;
        cagr(annual_rows[n-6].total_amount, annual_rows[n-1].total_amount, 5.0)
    } else { 0.0 };

    // Consecutive growth years counted from the latest backwards
    let mut consecutive = 0usize;
    for row in annual_rows.iter().rev() {
        if row.growth_pct > 0.0 { consecutive += 1; } else { break; }
    }
    // Consecutive counting consumes the latest `consecutive` rows whose growth > 0.
    // The earliest row always has growth_pct = 0.0 so we never count it.

    // Consistency: share of yoy deltas that were non-negative
    let deltas = annual_rows.iter().skip(1).count();
    let non_neg = annual_rows.iter().skip(1).filter(|r| r.growth_pct >= 0.0).count();
    let consistency_pct = if deltas == 0 { 0.0 } else { non_neg as f64 / deltas as f64 * 100.0 };

    let trend_label = if years_covered < 2 {
        "NO_HISTORY"
    } else if cagr_1y >= 3.0 && consistency_pct >= 70.0 {
        "GROWING"
    } else if cagr_1y <= -5.0 {
        "CUTTING"
    } else {
        "STABLE"
    };

    DivgSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        total_payments,
        first_payment_date,
        latest_payment_date,
        latest_amount,
        annualized_dividend: annualized,
        years_covered,
        cagr_1y_pct: cagr_1y,
        cagr_3y_pct: cagr_3y,
        cagr_5y_pct: cagr_5y,
        consecutive_growth_years: consecutive,
        consistency_score_pct: consistency_pct,
        annual_rows,
        trend_label: trend_label.to_string(),
        note: String::new(),
    }
}

/// EARM — Earnings Momentum Trend computed over cached FA + EPS surprises.
pub fn compute_earm_snapshot(
    symbol: &str,
    as_of: &str,
    statements: &FinancialStatements,
    surprises: &[EarningsSurprise],
) -> EarmSnapshot {
    let sym = symbol.to_uppercase();

    let quarters: Vec<&IncomeStatement> = statements.income_quarterly.iter().take(12).collect();

    if quarters.len() < 5 {
        return EarmSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            quarters_used: quarters.len(),
            momentum_label: "INSUFFICIENT_DATA".to_string(),
            note: "need at least 5 quarterly statements — run FA first".to_string(),
            ..Default::default()
        };
    }

    // Assume income_quarterly is newest-first (consistent with other compute fns in this file).
    // quarters[0] = latest, quarters[4] = year ago.
    let mut rows: Vec<EarmQuarterRow> = Vec::with_capacity(quarters.len());
    for (i, q) in quarters.iter().enumerate() {
        let yoy_pct = if i + 4 < quarters.len() {
            let prior = quarters[i + 4].revenue;
            if prior.abs() > 0.0 { (q.revenue - prior) / prior.abs() * 100.0 } else { 0.0 }
        } else { 0.0 };
        let surprise = surprises.iter().find(|s| s.date == q.date);
        rows.push(EarmQuarterRow {
            period: q.date.clone(),
            revenue: q.revenue,
            revenue_yoy_pct: yoy_pct,
            eps_actual: surprise.map(|s| s.eps_actual).unwrap_or(q.eps),
            eps_estimate: surprise.map(|s| s.eps_estimate).unwrap_or(0.0),
            eps_surprise_pct: surprise.map(|s| s.surprise_pct).unwrap_or(0.0),
        });
    }

    // Compute revenue growth averages: latest 4Q vs prior 4Q.
    // Row indices 0..=3 are "recent", 4..=7 are "prior". If we have fewer than 8 rows,
    // use whatever overlap is available for "prior".
    let recent_count = rows.iter().take(4).filter(|r| r.revenue_yoy_pct != 0.0).count();
    let recent_rev_growth: f64 = if recent_count == 0 { 0.0 } else {
        rows.iter().take(4).map(|r| r.revenue_yoy_pct).sum::<f64>() / recent_count as f64
    };
    let prior_slice = if rows.len() >= 8 { &rows[4..8] } else if rows.len() > 4 { &rows[4..] } else { &[] };
    let prior_count = prior_slice.iter().filter(|r| r.revenue_yoy_pct != 0.0).count();
    let prior_rev_growth: f64 = if prior_count == 0 { 0.0 } else {
        prior_slice.iter().map(|r| r.revenue_yoy_pct).sum::<f64>() / prior_count as f64
    };
    let rev_accel = recent_rev_growth - prior_rev_growth;

    // Similar for EPS surprise %. Pull directly from surprises array if FA/surprise alignment is sparse.
    let recent_surprises: Vec<f64> = surprises.iter().take(4).map(|s| s.surprise_pct).collect();
    let prior_surprises: Vec<f64> = surprises.iter().skip(4).take(4).map(|s| s.surprise_pct).collect();
    let recent_eps_surp = if recent_surprises.is_empty() { 0.0 }
        else { recent_surprises.iter().sum::<f64>() / recent_surprises.len() as f64 };
    let prior_eps_surp = if prior_surprises.is_empty() { 0.0 }
        else { prior_surprises.iter().sum::<f64>() / prior_surprises.len() as f64 };
    let eps_accel = recent_eps_surp - prior_eps_surp;

    // Composite 0..100: combine growth level, growth acceleration, surprise level, surprise acceleration.
    // Each component clamped and scaled.
    let clamp = |x: f64, lo: f64, hi: f64| -> f64 { x.max(lo).min(hi) };
    let g1 = (clamp(recent_rev_growth, -30.0, 30.0) + 30.0) / 60.0 * 100.0;
    let g2 = (clamp(rev_accel,          -15.0, 15.0) + 15.0) / 30.0 * 100.0;
    let g3 = (clamp(recent_eps_surp,    -30.0, 30.0) + 30.0) / 60.0 * 100.0;
    let g4 = (clamp(eps_accel,          -15.0, 15.0) + 15.0) / 30.0 * 100.0;
    let composite = (g1 * 0.35 + g2 * 0.25 + g3 * 0.25 + g4 * 0.15).max(0.0).min(100.0);

    let momentum = if composite >= 65.0 && (rev_accel > 0.0 || eps_accel > 0.0) {
        "ACCELERATING"
    } else if composite <= 35.0 && (rev_accel < 0.0 || eps_accel < 0.0) {
        "DECELERATING"
    } else {
        "STABLE"
    };

    EarmSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        quarters_used: quarters.len(),
        recent_revenue_growth_pct: recent_rev_growth,
        prior_revenue_growth_pct: prior_rev_growth,
        revenue_acceleration_pct: rev_accel,
        recent_eps_surprise_pct: recent_eps_surp,
        prior_eps_surprise_pct: prior_eps_surp,
        eps_surprise_acceleration_pct: eps_accel,
        composite_score: composite,
        momentum_label: momentum.to_string(),
        quarters: rows,
        note: String::new(),
    }
}

/// SECTR — Sector Rotation Strength for a symbol, using the latest INDU snapshot.
pub fn compute_sector_rotation_snapshot(
    symbol: &str,
    as_of: &str,
    symbol_sector: &str,
    sectors: &[SectorPerformance],
) -> SectorRotationSnapshot {
    let sym = symbol.to_uppercase();

    if sectors.is_empty() {
        return SectorRotationSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            symbol_sector: symbol_sector.to_string(),
            strength_label: "NO_DATA".to_string(),
            note: "no sector performance cached — run INDU first".to_string(),
            ..Default::default()
        };
    }

    let mut ranked: Vec<&SectorPerformance> = sectors.iter().collect();
    ranked.sort_by(|a, b| b.change_pct.partial_cmp(&a.change_pct).unwrap_or(std::cmp::Ordering::Equal));

    let sectors_total = ranked.len() as i32;
    let avg_change = ranked.iter().map(|s| s.change_pct).sum::<f64>() / ranked.len() as f64;

    let mut sorted_pcts: Vec<f64> = ranked.iter().map(|s| s.change_pct).collect();
    sorted_pcts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_change = if sorted_pcts.is_empty() { 0.0 }
        else if sorted_pcts.len() % 2 == 1 { sorted_pcts[sorted_pcts.len() / 2] }
        else { (sorted_pcts[sorted_pcts.len() / 2 - 1] + sorted_pcts[sorted_pcts.len() / 2]) / 2.0 };

    let breadth = ranked.iter().filter(|s| s.change_pct > 0.0).count() as f64 / ranked.len() as f64 * 100.0;

    let strongest = ranked.first().unwrap();
    let weakest = ranked.last().unwrap();

    // Locate symbol's sector. Fuzzy-match: exact, case-insensitive, contains.
    let target = symbol_sector.trim();
    let target_lower = target.to_lowercase();
    let (symbol_rank, symbol_change) = if target.is_empty() {
        (0i32, 0.0f64)
    } else {
        let mut rank = 0i32;
        let mut change = 0.0f64;
        for (i, s) in ranked.iter().enumerate() {
            let a = s.sector.to_lowercase();
            if a == target_lower || a.contains(&target_lower) || target_lower.contains(&a) {
                rank = (i + 1) as i32;
                change = s.change_pct;
                break;
            }
        }
        (rank, change)
    };

    let rel_strength = symbol_change - avg_change;

    let strength = if symbol_rank == 0 {
        "NO_DATA"
    } else if symbol_rank <= (sectors_total / 3).max(1) && rel_strength > 0.0 {
        "LEADER"
    } else if symbol_rank > sectors_total - (sectors_total / 3).max(1) && rel_strength < 0.0 {
        "LAGGARD"
    } else {
        "NEUTRAL"
    };

    let note = if symbol_rank == 0 && !target.is_empty() {
        format!("symbol sector '{}' not found in cached INDU snapshot", target)
    } else {
        String::new()
    };

    SectorRotationSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        symbol_sector: symbol_sector.to_string(),
        symbol_sector_change_pct: symbol_change,
        sector_rank: symbol_rank,
        sectors_total,
        avg_sector_change_pct: avg_change,
        median_sector_change_pct: median_change,
        relative_strength_pct: rel_strength,
        breadth_pct: breadth,
        strongest_sector: strongest.sector.clone(),
        strongest_sector_pct: strongest.change_pct,
        weakest_sector: weakest.sector.clone(),
        weakest_sector_pct: weakest.change_pct,
        strength_label: strength.to_string(),
        note,
    }
}

/// UPDM — Upgrade/Downgrade Momentum snapshot for a symbol.
pub fn compute_updm_snapshot(
    symbol: &str,
    as_of: &str,
    actions: &[RatingChange],
) -> UpdmSnapshot {
    let sym = symbol.to_uppercase();
    let as_of_days = parse_yyyy_mm_dd_to_days(as_of);

    if actions.is_empty() || as_of_days.is_none() {
        return UpdmSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bias_label: "NO_COVERAGE".to_string(),
            trend_label: "STABLE".to_string(),
            note: "no rating change history cached — run UPDG first".to_string(),
            ..Default::default()
        };
    }
    let as_of_days = as_of_days.unwrap();

    let (mut up30, mut dn30, mut up90, mut dn90, mut up180, mut dn180) = (0,0,0,0,0,0);
    let mut init90 = 0;
    let mut maint90 = 0;
    let mut total = 0;
    let mut latest_days: i64 = i64::MIN;
    let mut latest: Option<&RatingChange> = None;

    for a in actions {
        total += 1;
        let ad = match parse_yyyy_mm_dd_to_days(&a.date) { Some(d) => d, None => continue };
        let delta = as_of_days - ad;
        if delta < 0 { continue; }
        let act = a.action.to_lowercase();
        let is_up = act.contains("upgrade");
        let is_dn = act.contains("downgrade");
        let is_init = act.contains("init");
        let is_maint = act.contains("maintain") || act.contains("reiterat");

        if delta <= 30 {
            if is_up { up30 += 1; }
            if is_dn { dn30 += 1; }
        }
        if delta <= 90 {
            if is_up { up90 += 1; }
            if is_dn { dn90 += 1; }
            if is_init { init90 += 1; }
            if is_maint { maint90 += 1; }
        }
        if delta <= 180 {
            if is_up { up180 += 1; }
            if is_dn { dn180 += 1; }
        }

        if ad > latest_days {
            latest_days = ad;
            latest = Some(a);
        }
    }

    let net_30 = up30 as i32 - dn30 as i32;
    let net_90 = up90 as i32 - dn90 as i32;
    let net_180 = up180 as i32 - dn180 as i32;

    let bias = if up90 == 0 && dn90 == 0 && init90 == 0 && maint90 == 0 {
        "NO_COVERAGE"
    } else if net_90 > 0 {
        "BULLISH"
    } else if net_90 < 0 {
        "BEARISH"
    } else {
        "NEUTRAL"
    };

    let trend = if net_30 > 0 && net_30 as i64 * 3 >= net_90 as i64 {
        "IMPROVING"
    } else if net_30 < 0 && net_30 as i64 * 3 <= net_90 as i64 {
        "DETERIORATING"
    } else {
        "STABLE"
    };

    let (latest_date, latest_action, latest_firm, latest_grade) = latest.map(|l| (
        l.date.clone(), l.action.clone(), l.firm.clone(), l.to_grade.clone(),
    )).unwrap_or_default();

    UpdmSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        total_actions: total,
        upgrades_30d: up30,
        downgrades_30d: dn30,
        upgrades_90d: up90,
        downgrades_90d: dn90,
        upgrades_180d: up180,
        downgrades_180d: dn180,
        initiations_90d: init90,
        maintains_90d: maint90,
        net_30d: net_30,
        net_90d: net_90,
        net_180d: net_180,
        latest_date,
        latest_action,
        latest_firm,
        latest_to_grade: latest_grade,
        bias_label: bias.to_string(),
        trend_label: trend.to_string(),
        note: String::new(),
    }
}

// ── ADR-120 Godel Parity Round 13 compute fns ──────────────────────────────

/// Pick the daily close closest to (and not after) `target_offset_back` bars
/// from the most recent bar. `bars` is newest-first. Returns None if the
/// offset is out of range.
fn pick_close_offset(bars_newest_first: &[HistoricalPriceRow], offset: usize) -> Option<f64> {
    if offset >= bars_newest_first.len() { return None; }
    let c = bars_newest_first[offset].close;
    if c > 0.0 { Some(c) } else { None }
}

/// MOM — 12-1 month momentum snapshot.
pub fn compute_momentum_snapshot(
    symbol: &str,
    as_of: &str,
    bars_newest_first: &[HistoricalPriceRow],
) -> MomentumSnapshot {
    let sym = symbol.to_uppercase();
    let n = bars_newest_first.len();

    if n < 252 {
        return MomentumSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: n as i32,
            regime_label: "INSUFFICIENT_DATA".to_string(),
            trend_label: "STABLE".to_string(),
            note: format!("need ≥252 bars; have {n}"),
            ..Default::default()
        };
    }

    let current = bars_newest_first[0].close;
    let c_1m = pick_close_offset(bars_newest_first, 21).unwrap_or(current);
    let c_3m = pick_close_offset(bars_newest_first, 63).unwrap_or(current);
    let c_6m = pick_close_offset(bars_newest_first, 126).unwrap_or(current);
    let c_12m = pick_close_offset(bars_newest_first, 252).unwrap_or(current);

    let pct = |from: f64, to: f64| -> f64 {
        if from > 0.0 { (to - from) / from * 100.0 } else { 0.0 }
    };
    let return_1m = pct(c_1m, current);
    let return_3m = pct(c_3m, current);
    let return_6m = pct(c_6m, current);
    let return_12m = pct(c_12m, current);
    // 12-1 = return from 12m ago to 1m ago (skipping the most recent month)
    let return_12_1 = pct(c_12m, c_1m);

    // Annualised daily return stdev over the last 252 bars.
    let mut log_rets: Vec<f64> = Vec::with_capacity(251);
    for i in 0..251 {
        let c_new = bars_newest_first[i].close;
        let c_old = bars_newest_first[i + 1].close;
        if c_new > 0.0 && c_old > 0.0 {
            log_rets.push((c_new / c_old).ln());
        }
    }
    let mean: f64 = if log_rets.is_empty() { 0.0 } else { log_rets.iter().sum::<f64>() / log_rets.len() as f64 };
    let var: f64 = if log_rets.len() > 1 {
        log_rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (log_rets.len() - 1) as f64
    } else { 0.0 };
    let daily_stdev = var.sqrt();
    let vol_ann_pct = daily_stdev * (252f64).sqrt() * 100.0;

    let vol_adj_score = if vol_ann_pct > 0.0 { return_12_1 / vol_ann_pct } else { 0.0 };

    let composite = (50.0 + vol_adj_score * 20.0 + return_6m * 0.3).clamp(0.0, 100.0);
    let regime = if composite >= 75.0 { "STRONG" }
                 else if composite >= 40.0 { "NEUTRAL" }
                 else if composite >= 20.0 { "WEAK" }
                 else { "CRASH" };
    let trend = if return_1m > return_3m / 3.0 && return_3m > return_6m / 2.0 { "ACCELERATING" }
                else if return_1m < return_3m / 3.0 && return_3m < return_6m / 2.0 { "DECELERATING" }
                else { "STABLE" };

    MomentumSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: n as i32,
        return_1m_pct: return_1m,
        return_3m_pct: return_3m,
        return_6m_pct: return_6m,
        return_12m_pct: return_12m,
        return_12_1_pct: return_12_1,
        vol_annualized_pct: vol_ann_pct,
        vol_adjusted_score: vol_adj_score,
        composite_score: composite,
        regime_label: regime.to_string(),
        trend_label: trend.to_string(),
        note: String::new(),
    }
}

/// LIQ — Liquidity profile snapshot.
pub fn compute_liquidity_snapshot(
    symbol: &str,
    as_of: &str,
    bars_newest_first: &[HistoricalPriceRow],
    shares_outstanding: f64,
    window_days: i32,
) -> LiquiditySnapshot {
    let sym = symbol.to_uppercase();
    let w = window_days.max(20) as usize;

    if bars_newest_first.len() < 20 {
        return LiquiditySnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            window_days: w as i32,
            shares_outstanding,
            liquidity_tier: "INSUFFICIENT_DATA".to_string(),
            note: format!("need ≥20 bars; have {}", bars_newest_first.len()),
            ..Default::default()
        };
    }

    let slice_len = bars_newest_first.len().min(w);
    let slice = &bars_newest_first[..slice_len];

    let mut share_vols: Vec<f64> = Vec::with_capacity(slice_len);
    let mut dollar_vols: Vec<f64> = Vec::with_capacity(slice_len);
    let mut true_range_pcts: Vec<f64> = Vec::with_capacity(slice_len);
    let mut amihud_terms: Vec<f64> = Vec::new();
    let mut high_low_betas: Vec<f64> = Vec::new();

    for (i, b) in slice.iter().enumerate() {
        if b.volume > 0.0 {
            share_vols.push(b.volume);
            let dv = b.volume * b.close;
            dollar_vols.push(dv);
            if b.high > 0.0 && b.low > 0.0 && b.high >= b.low {
                let hl = b.high - b.low;
                if b.close > 0.0 {
                    true_range_pcts.push(hl / b.close * 100.0);
                }
                // Corwin-Schultz beta term — ln²(H/L)
                if b.high > 0.0 && b.low > 0.0 {
                    let ln_hl = (b.high / b.low).ln();
                    high_low_betas.push(ln_hl * ln_hl);
                }
            }
            // Amihud: |daily return| / dollar volume
            if i + 1 < slice.len() {
                let prev = slice[i + 1].close;
                if prev > 0.0 && dv > 0.0 {
                    let r = (b.close - prev) / prev;
                    amihud_terms.push(r.abs() / dv);
                }
            }
        }
    }

    let avg_share = if share_vols.is_empty() { 0.0 } else { share_vols.iter().sum::<f64>() / share_vols.len() as f64 };
    let avg_dollar = if dollar_vols.is_empty() { 0.0 } else { dollar_vols.iter().sum::<f64>() / dollar_vols.len() as f64 };
    let median = |mut v: Vec<f64>| -> f64 {
        if v.is_empty() { return 0.0; }
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = v.len() / 2;
        if v.len() % 2 == 0 { (v[mid - 1] + v[mid]) / 2.0 } else { v[mid] }
    };
    let med_share = median(share_vols.clone());
    let med_dollar = median(dollar_vols.clone());

    let turnover_pct = if shares_outstanding > 0.0 { avg_share / shares_outstanding * 100.0 } else { 0.0 };
    let amihud = if amihud_terms.is_empty() {
        0.0
    } else {
        amihud_terms.iter().sum::<f64>() / amihud_terms.len() as f64 * 1.0e6
    };
    let atr_pct = if true_range_pcts.is_empty() { 0.0 } else { true_range_pcts.iter().sum::<f64>() / true_range_pcts.len() as f64 };
    // Corwin-Schultz simplified: spread% ≈ 2 · (exp(α) − 1) / (1 + exp(α))
    // where α = (√(2β) − √β) / (3 − 2√2) and β is the average of ln²(H/L).
    let spread_proxy_pct = if high_low_betas.is_empty() {
        0.0
    } else {
        let beta = high_low_betas.iter().sum::<f64>() / high_low_betas.len() as f64;
        let denom = 3.0 - 2.0 * (2f64).sqrt();
        if denom > 0.0 && beta >= 0.0 {
            let alpha = ((2.0 * beta).sqrt() - beta.sqrt()) / denom;
            let ea = alpha.exp();
            if ea + 1.0 > 0.0 {
                (2.0 * (ea - 1.0) / (ea + 1.0)) * 100.0
            } else { 0.0 }
        } else { 0.0 }
    };

    let tier = if avg_dollar >= 5.0e8 {
        "DEEP"
    } else if avg_dollar >= 5.0e7 {
        "LIQUID"
    } else if avg_dollar >= 5.0e6 {
        "MODERATE"
    } else if avg_dollar >= 5.0e5 {
        "THIN"
    } else {
        "ILLIQUID"
    };

    LiquiditySnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        window_days: w as i32,
        avg_daily_share_volume: avg_share,
        median_daily_share_volume: med_share,
        avg_daily_dollar_volume: avg_dollar,
        median_daily_dollar_volume: med_dollar,
        shares_outstanding,
        daily_turnover_pct: turnover_pct,
        amihud_illiquidity: amihud,
        avg_true_range_pct: atr_pct,
        spread_proxy_pct: spread_proxy_pct.max(0.0),
        liquidity_tier: tier.to_string(),
        note: String::new(),
    }
}

/// BREAK — Breakout proximity snapshot.
pub fn compute_breakout_snapshot(
    symbol: &str,
    as_of: &str,
    bars_newest_first: &[HistoricalPriceRow],
) -> BreakoutSnapshot {
    let sym = symbol.to_uppercase();
    let n = bars_newest_first.len();

    if n < 20 {
        return BreakoutSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            breakout_label: "INSUFFICIENT_DATA".to_string(),
            setup_label: "NEUTRAL".to_string(),
            note: format!("need ≥20 bars; have {n}"),
            ..Default::default()
        };
    }

    let current = bars_newest_first[0].close;

    let range_high_low = |slice: &[HistoricalPriceRow]| -> (f64, f64) {
        let mut hi = f64::MIN;
        let mut lo = f64::MAX;
        for b in slice {
            if b.high > 0.0 && b.high > hi { hi = b.high; }
            if b.low > 0.0 && b.low < lo { lo = b.low; }
        }
        if hi == f64::MIN { hi = 0.0; }
        if lo == f64::MAX { lo = 0.0; }
        (hi, lo)
    };

    let (h20, l20) = range_high_low(&bars_newest_first[..20.min(n)]);
    let (h60, l60) = range_high_low(&bars_newest_first[..60.min(n)]);
    let (h52, l52) = range_high_low(&bars_newest_first[..252.min(n)]);

    let pct_from = |target: f64, from: f64| -> f64 {
        if from > 0.0 { (target - from) / from * 100.0 } else { 0.0 }
    };

    let dist_52w_high = pct_from(current, h52);
    let dist_52w_low = pct_from(current, l52);
    let dist_20d_high = pct_from(current, h20);
    let dist_60d_high = pct_from(current, h60);

    let pos_in_range = |cur: f64, hi: f64, lo: f64| -> f64 {
        let width = hi - lo;
        if width > 0.0 { (cur - lo) / width * 100.0 } else { 50.0 }
    };
    let pos_52w = pos_in_range(current, h52, l52);
    let pos_20d = pos_in_range(current, h20, l20);

    let cons_pct = {
        let mean_close = {
            let mut s = 0.0;
            let mut k = 0;
            for b in &bars_newest_first[..20.min(n)] {
                if b.close > 0.0 { s += b.close; k += 1; }
            }
            if k > 0 { s / k as f64 } else { current }
        };
        if mean_close > 0.0 { (h20 - l20) / mean_close * 100.0 } else { 0.0 }
    };

    let breakout = if pos_52w >= 99.0 && current >= h52 {
        "NEW_HIGH"
    } else if pos_52w >= 85.0 {
        "NEAR_HIGH"
    } else if pos_52w >= 15.0 {
        "MID_RANGE"
    } else if pos_52w >= 1.0 {
        "NEAR_LOW"
    } else {
        "NEW_LOW"
    };

    let setup = if cons_pct < 8.0 && pos_20d >= 70.0 {
        "BREAKOUT_IMMINENT"
    } else if cons_pct < 6.0 {
        "CONSOLIDATING"
    } else if dist_60d_high.abs() < 3.0 && pos_52w >= 60.0 {
        "TRENDING_UP"
    } else if pos_52w <= 35.0 && dist_52w_low < 10.0 {
        "TRENDING_DOWN"
    } else {
        "NEUTRAL"
    };

    BreakoutSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        current_price: current,
        high_20d: h20,
        low_20d: l20,
        high_60d: h60,
        low_60d: l60,
        high_52w: h52,
        low_52w: l52,
        dist_from_52w_high_pct: dist_52w_high,
        dist_from_52w_low_pct: dist_52w_low,
        dist_from_20d_high_pct: dist_20d_high,
        dist_from_60d_high_pct: dist_60d_high,
        position_in_52w_range_pct: pos_52w,
        position_in_20d_range_pct: pos_20d,
        consolidation_pct: cons_pct,
        breakout_label: breakout.to_string(),
        setup_label: setup.to_string(),
        note: String::new(),
    }
}

/// CCRL — Cash conversion cycle snapshot.
pub fn compute_cash_cycle_snapshot(
    symbol: &str,
    as_of: &str,
    statements: &FinancialStatements,
) -> CashCycleSnapshot {
    let sym = symbol.to_uppercase();

    let (income, balance, basis) = if !statements.income_annual.is_empty() && !statements.balance_annual.is_empty() {
        (&statements.income_annual, &statements.balance_annual, "annual")
    } else if !statements.income_quarterly.is_empty() && !statements.balance_quarterly.is_empty() {
        (&statements.income_quarterly, &statements.balance_quarterly, "quarterly")
    } else {
        return CashCycleSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            efficiency_label: "INSUFFICIENT_DATA".to_string(),
            trend_label: "STABLE".to_string(),
            note: "need cached FA annual or quarterly statements".to_string(),
            ..Default::default()
        };
    };

    let days_factor: f64 = if basis == "annual" { 365.0 } else { 91.25 };

    let compute_row = |inc: &IncomeStatement, bal: &BalanceSheet| -> Option<CashCycleRow> {
        if inc.revenue <= 0.0 || inc.cost_of_revenue <= 0.0 { return None; }
        let dso = bal.net_receivables / inc.revenue * days_factor;
        let dio = bal.inventory / inc.cost_of_revenue * days_factor;
        let dpo = bal.accounts_payable / inc.cost_of_revenue * days_factor;
        let ccc = dso + dio - dpo;
        Some(CashCycleRow {
            period: inc.date.clone(),
            dso_days: dso,
            dio_days: dio,
            dpo_days: dpo,
            ccc_days: ccc,
        })
    };

    let pair_count = income.len().min(balance.len());
    let mut rows: Vec<CashCycleRow> = Vec::with_capacity(pair_count);
    for i in 0..pair_count {
        if let Some(r) = compute_row(&income[i], &balance[i]) {
            rows.push(r);
        }
    }

    if rows.is_empty() {
        return CashCycleSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            efficiency_label: "INSUFFICIENT_DATA".to_string(),
            trend_label: "STABLE".to_string(),
            note: "revenue or COGS missing / zero in cached statements".to_string(),
            ..Default::default()
        };
    }

    let latest = &rows[0];
    let prior = rows.get(1);
    let prior_ccc = prior.map(|p| p.ccc_days).unwrap_or(latest.ccc_days);
    let change = latest.ccc_days - prior_ccc;

    let avg_window = rows.iter().take(3).map(|r| r.ccc_days).collect::<Vec<_>>();
    let avg_3y = avg_window.iter().sum::<f64>() / avg_window.len() as f64;

    let efficiency = if latest.ccc_days < 30.0 {
        "EFFICIENT"
    } else if latest.ccc_days < 90.0 {
        "NEUTRAL"
    } else {
        "INEFFICIENT"
    };

    let trend = if change <= -5.0 {
        "IMPROVING"
    } else if change >= 5.0 {
        "DETERIORATING"
    } else {
        "STABLE"
    };

    CashCycleSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        latest_period: latest.period.clone(),
        dso_days: latest.dso_days,
        dio_days: latest.dio_days,
        dpo_days: latest.dpo_days,
        ccc_days: latest.ccc_days,
        prior_ccc_days: prior_ccc,
        ccc_change_days: change,
        ccc_3y_avg_days: avg_3y,
        periods_used: rows.len(),
        efficiency_label: efficiency.to_string(),
        trend_label: trend.to_string(),
        periods: rows,
        note: String::new(),
    }
}

/// CREDIT — Unified credit score fusing ALTZ + PTFS + LEV + ACRL snapshots.
pub fn compute_credit_snapshot(
    symbol: &str,
    as_of: &str,
    altman: Option<&AltmanZSnapshot>,
    piotroski: Option<&PiotroskiSnapshot>,
    leverage: Option<&LeverageSnapshot>,
    accruals: Option<&AccrualsSnapshot>,
) -> CreditSnapshot {
    let sym = symbol.to_uppercase();
    let mut components: Vec<CreditComponent> = Vec::new();
    let mut total_weight = 0.0;
    let mut weighted_sum = 0.0;

    let mut altman_z = 0.0;
    let mut altman_zone = String::new();
    let mut piotroski_score = 0;
    let mut piotroski_label = String::new();
    let mut leverage_summary = String::new();
    let mut leverage_score = 0.0;
    let mut accruals_trend = String::new();
    let mut accruals_ttm = 0.0;
    let mut inputs_available = 0usize;

    // ALTZ — weight 35. Map Z via piecewise linear: DISTRESS<1.81→0..30, GRAY→30..70, SAFE≥2.99→70..100.
    if let Some(a) = altman {
        if a.zone != "INSUFFICIENT_DATA" && !a.zone.is_empty() {
            altman_z = a.z_score;
            altman_zone = a.zone.clone();
            let z = a.z_score;
            let score = if z >= 2.99 {
                let extra = (z - 2.99).min(3.0);
                (70.0 + extra / 3.0 * 30.0).min(100.0)
            } else if z >= 1.81 {
                let t = (z - 1.81) / (2.99 - 1.81);
                30.0 + t * 40.0
            } else if z > 0.0 {
                (z / 1.81 * 30.0).clamp(0.0, 30.0)
            } else {
                0.0
            };
            let w = 35.0;
            components.push(CreditComponent {
                name: "Altman Z".to_string(),
                value: format!("Z {:.2} ({})", z, a.zone),
                score,
                weight: w,
                contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // PTFS — weight 25. Map F score linearly 0..9 → 0..100.
    if let Some(p) = piotroski {
        if p.strength_label != "INSUFFICIENT_DATA" && !p.strength_label.is_empty() {
            piotroski_score = p.f_score;
            piotroski_label = p.strength_label.clone();
            let score = (p.f_score as f64 / 9.0 * 100.0).clamp(0.0, 100.0);
            let w = 25.0;
            components.push(CreditComponent {
                name: "Piotroski F".to_string(),
                value: format!("{}/9 ({})", p.f_score, p.strength_label),
                score,
                weight: w,
                contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // LEV — weight 25. Map solvency_summary label to a score.
    if let Some(lv) = leverage {
        if !lv.solvency_summary.is_empty() {
            leverage_summary = lv.solvency_summary.clone();
            let score = match lv.solvency_summary.as_str() {
                "HEALTHY" => 85.0,
                "MODERATE" | "NEUTRAL" => 60.0,
                "ELEVATED" => 40.0,
                "STRETCHED" | "DISTRESSED" => 15.0,
                _ => 50.0,
            };
            leverage_score = score;
            let w = 25.0;
            components.push(CreditComponent {
                name: "Leverage".to_string(),
                value: lv.solvency_summary.clone(),
                score,
                weight: w,
                contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // ACRL — weight 15. Map trend_label and ttm cash conversion to a score.
    if let Some(ac) = accruals {
        if !ac.trend_label.is_empty() {
            accruals_trend = ac.trend_label.clone();
            accruals_ttm = ac.ttm_cash_conversion_pct;
            let mut score: f64 = match ac.trend_label.as_str() {
                "IMPROVING" => 80.0,
                "STABLE" => 60.0,
                "MIXED" => 50.0,
                "DETERIORATING" => 30.0,
                _ => 50.0,
            };
            // Cash conversion >100% is a positive lean; <50% drags.
            if ac.ttm_cash_conversion_pct >= 100.0 {
                score = (score + 10.0).min(100.0);
            } else if ac.ttm_cash_conversion_pct < 50.0 && ac.ttm_cash_conversion_pct != 0.0 {
                score = (score - 10.0).max(0.0);
            }
            let w = 15.0;
            components.push(CreditComponent {
                name: "Accruals".to_string(),
                value: format!("{} ({:.0}% cash conv)", ac.trend_label, ac.ttm_cash_conversion_pct),
                score,
                weight: w,
                contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    if inputs_available == 0 || total_weight <= 0.0 {
        return CreditSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            letter_grade: "INSUFFICIENT_DATA".to_string(),
            credit_label: "INSUFFICIENT_DATA".to_string(),
            inputs_available: 0,
            note: "need at least one of ALTZ / PTFS / LEV / ACRL cached".to_string(),
            ..Default::default()
        };
    }

    let composite = (weighted_sum / total_weight).clamp(0.0, 100.0);
    let letter = if composite >= 90.0 { "AAA" }
                 else if composite >= 80.0 { "AA" }
                 else if composite >= 70.0 { "A" }
                 else if composite >= 60.0 { "BBB" }
                 else if composite >= 50.0 { "BB" }
                 else if composite >= 35.0 { "B" }
                 else { "CCC" };
    let label = if composite >= 70.0 { "INVESTMENT_GRADE" }
                else if composite >= 55.0 { "BORDERLINE" }
                else if composite >= 35.0 { "SPECULATIVE" }
                else { "DISTRESSED" };

    CreditSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        altman_z,
        altman_zone,
        piotroski_score,
        piotroski_label,
        leverage_summary,
        leverage_score,
        accruals_trend,
        accruals_ttm_cash_conversion_pct: accruals_ttm,
        composite_score: composite,
        letter_grade: letter.to_string(),
        credit_label: label.to_string(),
        inputs_available,
        components,
        note: String::new(),
    }
}

// ── ADR-121 Round 14 compute fns ───────────────────────────────────────────

/// GROWM — Growth-at-Reasonable-Price fusion of MOM + EARM + DIVG.
pub fn compute_growm_snapshot(
    symbol: &str,
    as_of: &str,
    momentum: Option<&MomentumSnapshot>,
    earm: Option<&EarmSnapshot>,
    divg: Option<&DivgSnapshot>,
) -> GrowmSnapshot {
    let sym = symbol.to_uppercase();
    let mut components: Vec<GarpComponent> = Vec::new();
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;
    let mut inputs_available = 0usize;

    let mut momentum_score = 0.0;
    let mut momentum_regime = String::new();
    let mut earm_score = 0.0;
    let mut earm_label = String::new();
    let mut divg_cagr = 0.0;
    let mut divg_trend = String::new();

    // MOM — weight 40. Composite is already 0..100.
    if let Some(m) = momentum {
        if m.regime_label != "INSUFFICIENT_DATA" && !m.regime_label.is_empty() {
            momentum_score = m.composite_score;
            momentum_regime = m.regime_label.clone();
            let w = 40.0;
            components.push(GarpComponent {
                name: "Momentum 12-1".to_string(),
                value: format!("{} ({:.1})", m.regime_label, m.composite_score),
                score: momentum_score,
                weight: w,
                contribution: momentum_score * w / 100.0,
            });
            weighted_sum += momentum_score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // EARM — weight 40. Composite is already 0..100.
    if let Some(e) = earm {
        if e.momentum_label != "INSUFFICIENT_DATA" && !e.momentum_label.is_empty() {
            earm_score = e.composite_score;
            earm_label = e.momentum_label.clone();
            let w = 40.0;
            components.push(GarpComponent {
                name: "Earnings Momentum".to_string(),
                value: format!("{} ({:.1})", e.momentum_label, e.composite_score),
                score: earm_score,
                weight: w,
                contribution: earm_score * w / 100.0,
            });
            weighted_sum += earm_score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // DIVG — weight 20. Map 3Y CAGR and trend to a score.
    if let Some(d) = divg {
        if d.trend_label != "NO_HISTORY" && !d.trend_label.is_empty() {
            divg_cagr = d.cagr_3y_pct;
            divg_trend = d.trend_label.clone();
            let mut score: f64 = match d.trend_label.as_str() {
                "GROWING" => 70.0,
                "STABLE" => 55.0,
                "CUTTING" => 25.0,
                _ => 50.0,
            };
            // Boost / penalty from the 3Y CAGR itself.
            if d.cagr_3y_pct >= 10.0 { score = (score + 15.0).min(100.0); }
            else if d.cagr_3y_pct >= 5.0 { score = (score + 7.0).min(100.0); }
            else if d.cagr_3y_pct < -5.0 { score = (score - 15.0).max(0.0); }
            let w = 20.0;
            components.push(GarpComponent {
                name: "Dividend Growth".to_string(),
                value: format!("{} (3Y {:+.1}%)", d.trend_label, d.cagr_3y_pct),
                score,
                weight: w,
                contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    if inputs_available == 0 || total_weight <= 0.0 {
        return GrowmSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            garp_label: "NO_DATA".to_string(),
            inputs_available: 0,
            note: "need at least one of MOM / EARM / DIVG cached".to_string(),
            ..Default::default()
        };
    }

    let composite = (weighted_sum / total_weight).clamp(0.0, 100.0);
    // GARP: balance momentum + earnings growth. Pure GROWTH = high MOM but weak EARM. VALUE = dividend-led. SPECULATIVE = only MOM.
    let mom_has = !momentum_regime.is_empty();
    let earm_has = !earm_label.is_empty();
    let divg_has = !divg_trend.is_empty();
    let label = if composite >= 70.0 && mom_has && earm_has {
        "GARP"
    } else if composite >= 65.0 && mom_has {
        "GROWTH"
    } else if composite >= 55.0 && divg_has && !earm_has {
        "VALUE"
    } else if composite >= 50.0 {
        if mom_has && !earm_has { "SPECULATIVE" } else { "GARP" }
    } else if composite >= 35.0 {
        "VALUE"
    } else {
        "SPECULATIVE"
    };

    GrowmSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        momentum_score,
        momentum_regime,
        earnings_momentum_score: earm_score,
        earnings_label: earm_label,
        dividend_cagr_3y_pct: divg_cagr,
        dividend_trend: divg_trend,
        composite_score: composite,
        garp_label: label.to_string(),
        inputs_available,
        components,
        note: String::new(),
    }
}

/// FLOW — Smart-money flow snapshot (insider + institutional).
pub fn compute_flow_snapshot(
    symbol: &str,
    as_of: &str,
    insider_trades: &[InsiderTrade],
    holders: &[InstitutionalHolder],
    window_days: i32,
) -> FlowSnapshot {
    let sym = symbol.to_uppercase();
    let w = window_days.max(7);

    let as_of_days_opt = parse_yyyy_mm_dd_to_days(as_of);
    let cutoff_opt = as_of_days_opt.map(|a| a - (w as i64 * 31 / 30).max(1));

    let mut buy_value = 0.0f64;
    let mut sell_value = 0.0f64;
    let mut trade_count = 0usize;
    let mut names: std::collections::HashSet<String> = std::collections::HashSet::new();

    for t in insider_trades {
        if t.transaction_date.is_empty() { continue; }
        let d = parse_yyyy_mm_dd_to_days(&t.transaction_date);
        if let (Some(cut), Some(dd)) = (cutoff_opt, d) {
            if dd < cut { continue; }
        }
        trade_count += 1;
        if !t.reporting_name.is_empty() { names.insert(t.reporting_name.clone()); }
        let kind = t.transaction_type.to_ascii_lowercase();
        if kind.contains('p') && kind.contains("purchase") {
            buy_value += t.value_usd.abs();
        } else if kind.contains('s') && kind.contains("sale") {
            sell_value += t.value_usd.abs();
        } else if t.acquisition_disposition.eq_ignore_ascii_case("a") {
            buy_value += t.value_usd.abs();
        } else if t.acquisition_disposition.eq_ignore_ascii_case("d") {
            sell_value += t.value_usd.abs();
        }
    }
    let insider_net = buy_value - sell_value;

    // Institutional flows: use HDS `change` column (delta vs prior 13F).
    let mut positive_delta = 0.0f64;
    let mut negative_delta = 0.0f64;
    let mut buyers = 0usize;
    let mut sellers = 0usize;
    let tracked = holders.len();
    for h in holders {
        if h.change > 0.0 {
            positive_delta += h.change;
            buyers += 1;
        } else if h.change < 0.0 {
            negative_delta += h.change.abs();
            sellers += 1;
        }
    }
    let net_share_delta = positive_delta - negative_delta;
    let net_ratio = if tracked > 0 {
        (buyers as f64 - sellers as f64) / tracked as f64
    } else {
        0.0
    };

    // Insider score: buy_value vs total activity.
    let gross_insider = buy_value + sell_value;
    let insider_score: f64 = if gross_insider <= 0.0 {
        50.0
    } else {
        let ratio = insider_net / gross_insider; // -1..1
        (50.0 + ratio * 50.0).clamp(0.0, 100.0)
    };

    // Institutional score: net_ratio -1..1 → 0..100.
    let institutional_score: f64 = if tracked == 0 {
        50.0
    } else {
        (50.0 + net_ratio * 50.0).clamp(0.0, 100.0)
    };

    let any_insider = trade_count > 0;
    let any_institutional = tracked > 0;

    let composite: f64 = if !any_insider && !any_institutional {
        return FlowSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            window_days: w,
            flow_label: "NO_DATA".to_string(),
            note: "need cached INS or HDS rows".to_string(),
            ..Default::default()
        };
    } else if any_insider && any_institutional {
        // weight insider 60, institutional 40 — insiders are more load-bearing signal
        (insider_score * 0.6 + institutional_score * 0.4).clamp(0.0, 100.0)
    } else if any_insider {
        insider_score
    } else {
        institutional_score
    };

    let label = if composite >= 80.0 { "STRONG_BUY" }
                else if composite >= 60.0 { "BUY" }
                else if composite >= 40.0 { "NEUTRAL" }
                else if composite >= 20.0 { "SELL" }
                else { "STRONG_SELL" };

    FlowSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        window_days: w,
        insider_buy_value_usd: buy_value,
        insider_sell_value_usd: sell_value,
        insider_net_value_usd: insider_net,
        insider_trade_count: trade_count,
        unique_insiders: names.len(),
        institutional_share_delta: net_share_delta,
        institutional_buyers: buyers,
        institutional_sellers: sellers,
        institutional_holders_tracked: tracked,
        institutional_net_ratio: net_ratio,
        insider_score,
        institutional_score,
        composite_score: composite,
        flow_label: label.to_string(),
        note: String::new(),
    }
}

/// REGIME — regime classifier fusing VOLE + TECH + HRA.
pub fn compute_regime_snapshot(
    symbol: &str,
    as_of: &str,
    vole: Option<&OhlcVolSnapshot>,
    tech: Option<&TechnicalSnapshot>,
    hra: Option<&HraSnapshot>,
) -> RegimeSnapshot {
    let sym = symbol.to_uppercase();
    let mut inputs_available = 0usize;

    let mut realized_vol_pct = 0.0;
    let mut vol_source = String::new();
    let mut adx_value = 0.0;
    let mut trend_summary = String::new();
    let mut sharpe = 0.0;
    let mut return_1y = 0.0;

    if let Some(v) = vole {
        if v.preferred_estimate_pct > 0.0 {
            realized_vol_pct = v.preferred_estimate_pct;
            vol_source = v.preferred_label.clone();
            inputs_available += 1;
        }
    }

    if let Some(t) = tech {
        trend_summary = t.trend_summary.clone();
        for ind in &t.indicators {
            if ind.name.to_ascii_uppercase().starts_with("ADX") {
                adx_value = ind.value;
                break;
            }
        }
        if !trend_summary.is_empty() || adx_value > 0.0 {
            inputs_available += 1;
        }
    }

    if let Some(h) = hra {
        sharpe = h.sharpe_ratio;
        for w in &h.windows {
            if w.label.eq_ignore_ascii_case("1Y") || w.label == "1y" {
                return_1y = w.return_pct;
                break;
            }
        }
        if h.volatility_annual_pct > 0.0 || !h.windows.is_empty() {
            inputs_available += 1;
        }
    }

    if inputs_available == 0 {
        return RegimeSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            regime_label: "INSUFFICIENT_DATA".to_string(),
            inputs_available: 0,
            note: "need at least one of VOLE / TECH / HRA cached".to_string(),
            ..Default::default()
        };
    }

    // Trend strength from ADX (25+ = strong trend).
    let trend_strength: f64 = if adx_value <= 0.0 {
        50.0
    } else if adx_value >= 40.0 { 100.0 }
    else if adx_value >= 25.0 { 60.0 + (adx_value - 25.0) / 15.0 * 40.0 }
    else if adx_value >= 15.0 { 30.0 + (adx_value - 15.0) / 10.0 * 30.0 }
    else { (adx_value / 15.0 * 30.0).max(0.0) };

    // Volatility score: low vol = high score.
    let vol_score: f64 = if realized_vol_pct <= 0.0 {
        50.0
    } else if realized_vol_pct < 15.0 { 90.0 }
    else if realized_vol_pct < 25.0 { 70.0 }
    else if realized_vol_pct < 40.0 { 50.0 }
    else if realized_vol_pct < 60.0 { 30.0 }
    else { 10.0 };

    // Return score from 1Y: +20% → 80, -20% → 20.
    let return_score: f64 = (50.0 + return_1y * 1.5).clamp(0.0, 100.0);

    let composite = ((trend_strength + vol_score + return_score) / 3.0).clamp(0.0, 100.0);

    // Regime classification.
    let regime = if realized_vol_pct >= 40.0 {
        "VOLATILE"
    } else if adx_value >= 25.0 && return_score >= 55.0 {
        "TRENDING"
    } else if adx_value >= 20.0 {
        "TRENDING"
    } else if realized_vol_pct > 0.0 && realized_vol_pct < 20.0 && adx_value < 18.0 {
        "QUIET"
    } else if adx_value < 20.0 {
        "MEAN_REVERTING"
    } else {
        "MEAN_REVERTING"
    };

    RegimeSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        realized_vol_pct,
        vol_source,
        adx_value,
        trend_summary,
        sharpe_ratio: sharpe,
        return_1y_pct: return_1y,
        trend_strength_score: trend_strength,
        volatility_score: vol_score,
        return_score,
        composite_score: composite,
        regime_label: regime.to_string(),
        inputs_available,
        note: String::new(),
    }
}

/// RELVOL — Relative volume snapshot over 5d/20d/60d windows.
pub fn compute_relvol_snapshot(
    symbol: &str,
    as_of: &str,
    bars_newest_first: &[HistoricalPriceRow],
) -> RelVolSnapshot {
    let sym = symbol.to_uppercase();
    let n = bars_newest_first.len();

    if n < 20 {
        return RelVolSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: n,
            activity_label: "INSUFFICIENT_DATA".to_string(),
            direction_label: "NEUTRAL".to_string(),
            note: format!("need ≥20 bars; have {n}"),
            ..Default::default()
        };
    }

    let current = bars_newest_first[0].volume;
    let avg = |slice: &[HistoricalPriceRow]| -> f64 {
        if slice.is_empty() { return 0.0; }
        let mut s = 0.0; let mut k = 0;
        for b in slice { if b.volume > 0.0 { s += b.volume; k += 1; } }
        if k > 0 { s / k as f64 } else { 0.0 }
    };
    // Averages exclude the current bar to prevent the current bar from skewing the baseline.
    let avg_5 = avg(&bars_newest_first[1..(1 + 5).min(n)]);
    let avg_20 = avg(&bars_newest_first[1..(1 + 20).min(n)]);
    let avg_60 = avg(&bars_newest_first[1..(1 + 60).min(n)]);

    let rel = |num: f64, den: f64| -> f64 { if den > 0.0 { num / den } else { 0.0 } };
    let r5 = rel(current, avg_5);
    let r20 = rel(current, avg_20);
    let r60 = rel(current, avg_60);

    let vol_trend = if avg_20 > 0.0 { (avg_5 / avg_20 - 1.0) * 100.0 } else { 0.0 };

    // Percentile rank of current vs last 60 bars (excluding itself).
    let sample_end = (1 + 60).min(n);
    let sample: Vec<f64> = bars_newest_first[1..sample_end].iter().map(|b| b.volume).collect();
    let percentile = if sample.is_empty() {
        50.0
    } else {
        let count_below = sample.iter().filter(|v| **v < current).count();
        count_below as f64 / sample.len() as f64 * 100.0
    };

    let activity = if r20 >= 3.0 { "EXTREME" }
                   else if r20 >= 2.0 { "HIGH" }
                   else if r20 >= 1.5 { "ELEVATED" }
                   else if r20 >= 0.5 { "NORMAL" }
                   else { "LOW" };

    let direction = if n >= 2 {
        let prior_close = bars_newest_first[1].close;
        let now_close = bars_newest_first[0].close;
        if prior_close > 0.0 && now_close > prior_close * 1.005 { "BULLISH" }
        else if prior_close > 0.0 && now_close < prior_close * 0.995 { "BEARISH" }
        else { "NEUTRAL" }
    } else { "NEUTRAL" };

    RelVolSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        current_volume: current,
        avg_volume_5d: avg_5,
        avg_volume_20d: avg_20,
        avg_volume_60d: avg_60,
        rel_volume_5d: r5,
        rel_volume_20d: r20,
        rel_volume_60d: r60,
        volume_trend_5d_pct: vol_trend,
        volume_percentile_60d: percentile,
        activity_label: activity.to_string(),
        direction_label: direction.to_string(),
        bars_used: n,
        note: String::new(),
    }
}

/// MARGINS — Margin trajectory snapshot (gross / operating / net).
pub fn compute_margins_snapshot(
    symbol: &str,
    as_of: &str,
    statements: &FinancialStatements,
) -> MarginsSnapshot {
    let sym = symbol.to_uppercase();

    let (income, basis) = if !statements.income_annual.is_empty() {
        (&statements.income_annual, "annual")
    } else if !statements.income_quarterly.is_empty() {
        (&statements.income_quarterly, "quarterly")
    } else {
        return MarginsSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            gross_trend_label: "INSUFFICIENT_DATA".to_string(),
            operating_trend_label: "INSUFFICIENT_DATA".to_string(),
            net_trend_label: "INSUFFICIENT_DATA".to_string(),
            overall_trend_label: "INSUFFICIENT_DATA".to_string(),
            quality_label: "INSUFFICIENT_DATA".to_string(),
            note: "need cached FA annual or quarterly income statements".to_string(),
            ..Default::default()
        };
    };

    let mut rows: Vec<MarginRow> = Vec::new();
    for inc in income.iter() {
        if inc.revenue <= 0.0 { continue; }
        let g = if inc.gross_profit != 0.0 { inc.gross_profit / inc.revenue * 100.0 } else { 0.0 };
        let o = if inc.operating_income != 0.0 { inc.operating_income / inc.revenue * 100.0 } else { 0.0 };
        let n_m = inc.net_income / inc.revenue * 100.0;
        rows.push(MarginRow {
            period: inc.date.clone(),
            gross_margin_pct: g,
            operating_margin_pct: o,
            net_margin_pct: n_m,
        });
    }

    if rows.is_empty() {
        return MarginsSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            basis: basis.to_string(),
            gross_trend_label: "INSUFFICIENT_DATA".to_string(),
            operating_trend_label: "INSUFFICIENT_DATA".to_string(),
            net_trend_label: "INSUFFICIENT_DATA".to_string(),
            overall_trend_label: "INSUFFICIENT_DATA".to_string(),
            quality_label: "INSUFFICIENT_DATA".to_string(),
            note: "no periods with positive revenue in cached statements".to_string(),
            ..Default::default()
        };
    }

    let latest = &rows[0];
    let prior = rows.get(1).cloned().unwrap_or_else(|| latest.clone());
    let g_chg = latest.gross_margin_pct - prior.gross_margin_pct;
    let o_chg = latest.operating_margin_pct - prior.operating_margin_pct;
    let n_chg = latest.net_margin_pct - prior.net_margin_pct;

    let avg_g = rows.iter().map(|r| r.gross_margin_pct).sum::<f64>() / rows.len() as f64;
    let avg_o = rows.iter().map(|r| r.operating_margin_pct).sum::<f64>() / rows.len() as f64;
    let avg_n = rows.iter().map(|r| r.net_margin_pct).sum::<f64>() / rows.len() as f64;

    let label_trend = |chg: f64| -> &'static str {
        if chg >= 1.0 { "EXPANDING" }
        else if chg <= -1.0 { "CONTRACTING" }
        else { "STABLE" }
    };
    let gross_trend = label_trend(g_chg);
    let op_trend = label_trend(o_chg);
    let net_trend = label_trend(n_chg);

    // Overall — majority rule across the three.
    let mut exp_n = 0; let mut con_n = 0;
    for t in [gross_trend, op_trend, net_trend] {
        if t == "EXPANDING" { exp_n += 1; }
        else if t == "CONTRACTING" { con_n += 1; }
    }
    let overall = if exp_n >= 2 { "EXPANDING" }
                  else if con_n >= 2 { "CONTRACTING" }
                  else { "STABLE" };

    let quality = if latest.operating_margin_pct >= 20.0 { "HIGH" }
                  else if latest.operating_margin_pct >= 8.0 { "MEDIUM" }
                  else { "LOW" };

    MarginsSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        basis: basis.to_string(),
        latest_period: latest.period.clone(),
        latest_gross_margin_pct: latest.gross_margin_pct,
        latest_operating_margin_pct: latest.operating_margin_pct,
        latest_net_margin_pct: latest.net_margin_pct,
        prior_gross_margin_pct: prior.gross_margin_pct,
        prior_operating_margin_pct: prior.operating_margin_pct,
        prior_net_margin_pct: prior.net_margin_pct,
        gross_margin_change_pct: g_chg,
        operating_margin_change_pct: o_chg,
        net_margin_change_pct: n_chg,
        avg_gross_margin_pct: avg_g,
        avg_operating_margin_pct: avg_o,
        avg_net_margin_pct: avg_n,
        periods_used: rows.len(),
        gross_trend_label: gross_trend.to_string(),
        operating_trend_label: op_trend.to_string(),
        net_trend_label: net_trend.to_string(),
        overall_trend_label: overall.to_string(),
        quality_label: quality.to_string(),
        periods: rows,
        note: String::new(),
    }
}

// ── ADR-122 Round 15 compute fns ───────────────────────────────────────────

fn median_f64(values: &[f64]) -> f64 {
    if values.is_empty() { return 0.0; }
    let mut v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if v.is_empty() { return 0.0; }
    let mid = v.len() / 2;
    if v.len() % 2 == 0 { (v[mid - 1] + v[mid]) / 2.0 } else { v[mid] }
}

/// Score a "lower is better" multiple vs a peer median.
/// ratio ≤ median × 0.5 → 100; ratio ≥ median × 2.0 → 0; linear in between.
fn score_multiple_lower_better(value: f64, median: f64) -> f64 {
    if !value.is_finite() || value <= 0.0 || !median.is_finite() || median <= 0.0 {
        return 0.0;
    }
    let ratio = value / median;
    if ratio <= 0.5 { 100.0 }
    else if ratio >= 2.0 { 0.0 }
    else { (100.0 * (2.0 - ratio) / 1.5).clamp(0.0, 100.0) }
}

/// Score a "higher is better" yield vs a peer median.
/// yield ≥ median × 1.5 → 100; yield ≤ median × 0.5 → 0; linear in between.
fn score_yield_higher_better(value: f64, median: f64) -> f64 {
    if !value.is_finite() || !median.is_finite() || median <= 0.0 {
        return 0.0;
    }
    let ratio = value / median;
    if ratio >= 1.5 { 100.0 }
    else if ratio <= 0.5 { 0.0 }
    else { (100.0 * (ratio - 0.5) / 1.0).clamp(0.0, 100.0) }
}

/// VAL — Value-factor composite vs sector peers.
pub fn compute_val_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    fund: Option<&crate::core::fundamentals::Fundamentals>,
    peer_fundamentals: &[crate::core::fundamentals::Fundamentals],
    fcfy: Option<&FcfYieldSnapshot>,
    peer_fcf_yields: &[f64],
) -> ValueSnapshot {
    let sym = symbol.to_uppercase();
    let mut components: Vec<FactorComponent> = Vec::new();
    let mut total_weight = 0.0;
    let mut weighted_sum = 0.0;
    let mut inputs_available = 0usize;

    let f = match fund {
        Some(v) => v,
        None => {
            return ValueSnapshot {
                symbol: sym,
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                value_label: "NO_DATA".to_string(),
                note: "no Fundamentals row cached for this symbol".to_string(),
                ..Default::default()
            };
        }
    };

    let peers_considered = peer_fundamentals.len();

    // Collect peer medians for each metric — only non-missing, positive values.
    let peer_pe: Vec<f64> = peer_fundamentals.iter()
        .filter_map(|p| p.pe_ratio).filter(|v| *v > 0.0 && v.is_finite()).collect();
    let peer_fpe: Vec<f64> = peer_fundamentals.iter()
        .filter_map(|p| p.forward_pe).filter(|v| *v > 0.0 && v.is_finite()).collect();
    let peer_pb: Vec<f64> = peer_fundamentals.iter()
        .filter_map(|p| p.price_to_book).filter(|v| *v > 0.0 && v.is_finite()).collect();
    let peer_ps: Vec<f64> = peer_fundamentals.iter()
        .filter_map(|p| p.price_to_sales).filter(|v| *v > 0.0 && v.is_finite()).collect();
    let peer_evebitda: Vec<f64> = peer_fundamentals.iter()
        .filter_map(|p| p.ev_to_ebitda).filter(|v| *v > 0.0 && v.is_finite()).collect();

    let pe_median = median_f64(&peer_pe);
    let fpe_median = median_f64(&peer_fpe);
    let pb_median = median_f64(&peer_pb);
    let ps_median = median_f64(&peer_ps);
    let evebitda_median = median_f64(&peer_evebitda);
    let fcfy_median = median_f64(peer_fcf_yields);

    let pe = f.pe_ratio.unwrap_or(0.0);
    let fpe = f.forward_pe.unwrap_or(0.0);
    let pb = f.price_to_book.unwrap_or(0.0);
    let ps = f.price_to_sales.unwrap_or(0.0);
    let evebitda = f.ev_to_ebitda.unwrap_or(0.0);
    let fcfy_val = fcfy.map(|s| s.ttm_fcf_yield_pct).unwrap_or(0.0);

    // P/E — weight 25
    if pe > 0.0 && pe_median > 0.0 {
        let score = score_multiple_lower_better(pe, pe_median);
        let w = 25.0;
        components.push(FactorComponent {
            name: "P/E".to_string(),
            value: format!("{:.2} vs median {:.2}", pe, pe_median),
            score, weight: w, contribution: score * w / 100.0,
        });
        weighted_sum += score * w;
        total_weight += w;
        inputs_available += 1;
    }

    // Forward P/E — weight 15
    if fpe > 0.0 && fpe_median > 0.0 {
        let score = score_multiple_lower_better(fpe, fpe_median);
        let w = 15.0;
        components.push(FactorComponent {
            name: "Forward P/E".to_string(),
            value: format!("{:.2} vs median {:.2}", fpe, fpe_median),
            score, weight: w, contribution: score * w / 100.0,
        });
        weighted_sum += score * w;
        total_weight += w;
        inputs_available += 1;
    }

    // P/B — weight 15
    if pb > 0.0 && pb_median > 0.0 {
        let score = score_multiple_lower_better(pb, pb_median);
        let w = 15.0;
        components.push(FactorComponent {
            name: "P/B".to_string(),
            value: format!("{:.2} vs median {:.2}", pb, pb_median),
            score, weight: w, contribution: score * w / 100.0,
        });
        weighted_sum += score * w;
        total_weight += w;
        inputs_available += 1;
    }

    // P/S — weight 15
    if ps > 0.0 && ps_median > 0.0 {
        let score = score_multiple_lower_better(ps, ps_median);
        let w = 15.0;
        components.push(FactorComponent {
            name: "P/S".to_string(),
            value: format!("{:.2} vs median {:.2}", ps, ps_median),
            score, weight: w, contribution: score * w / 100.0,
        });
        weighted_sum += score * w;
        total_weight += w;
        inputs_available += 1;
    }

    // EV/EBITDA — weight 20
    if evebitda > 0.0 && evebitda_median > 0.0 {
        let score = score_multiple_lower_better(evebitda, evebitda_median);
        let w = 20.0;
        components.push(FactorComponent {
            name: "EV/EBITDA".to_string(),
            value: format!("{:.2} vs median {:.2}", evebitda, evebitda_median),
            score, weight: w, contribution: score * w / 100.0,
        });
        weighted_sum += score * w;
        total_weight += w;
        inputs_available += 1;
    }

    // FCF Yield — weight 10
    if fcfy_val.is_finite() && fcfy_median > 0.0 {
        let score = score_yield_higher_better(fcfy_val, fcfy_median);
        let w = 10.0;
        components.push(FactorComponent {
            name: "FCF Yield".to_string(),
            value: format!("{:.2}% vs median {:.2}%", fcfy_val, fcfy_median),
            score, weight: w, contribution: score * w / 100.0,
        });
        weighted_sum += score * w;
        total_weight += w;
        inputs_available += 1;
    }

    if inputs_available == 0 || total_weight <= 0.0 {
        return ValueSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            peers_considered,
            value_label: "NO_DATA".to_string(),
            note: "need at least one valuation metric vs a non-empty sector peer median".to_string(),
            ..Default::default()
        };
    }

    let composite = (weighted_sum / total_weight).clamp(0.0, 100.0);
    let label = if composite >= 80.0 { "DEEP_VALUE" }
                else if composite >= 65.0 { "VALUE" }
                else if composite >= 45.0 { "FAIR" }
                else if composite >= 30.0 { "EXPENSIVE" }
                else { "PREMIUM" };

    ValueSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        peers_considered,
        pe_ratio: pe,
        pe_sector_median: pe_median,
        forward_pe: fpe,
        forward_pe_sector_median: fpe_median,
        price_to_book: pb,
        price_to_book_sector_median: pb_median,
        price_to_sales: ps,
        price_to_sales_sector_median: ps_median,
        ev_to_ebitda: evebitda,
        ev_to_ebitda_sector_median: evebitda_median,
        fcf_yield_pct: fcfy_val,
        fcf_yield_sector_median_pct: fcfy_median,
        composite_score: composite,
        value_label: label.to_string(),
        inputs_available,
        components,
        note: String::new(),
    }
}

/// QUAL — Quality-factor composite fusing PTFS + MARGINS + ACRL + LEV.
pub fn compute_qual_snapshot(
    symbol: &str,
    as_of: &str,
    piotroski: Option<&PiotroskiSnapshot>,
    margins: Option<&MarginsSnapshot>,
    accruals: Option<&AccrualsSnapshot>,
    leverage: Option<&LeverageSnapshot>,
) -> QualitySnapshot {
    let sym = symbol.to_uppercase();
    let mut components: Vec<FactorComponent> = Vec::new();
    let mut total_weight = 0.0;
    let mut weighted_sum = 0.0;
    let mut inputs_available = 0usize;

    let mut piotroski_score = 0;
    let mut piotroski_label = String::new();
    let mut operating_margin_pct = 0.0;
    let mut margin_trend_label = String::new();
    let mut cash_conversion_pct = 0.0;
    let mut accruals_trend_label = String::new();
    let mut leverage_summary = String::new();
    let mut debt_to_ebitda = 0.0;

    // PTFS — weight 30. Map F score linearly 0..9 → 0..100.
    if let Some(p) = piotroski {
        if p.strength_label != "INSUFFICIENT_DATA" && !p.strength_label.is_empty() {
            piotroski_score = p.f_score;
            piotroski_label = p.strength_label.clone();
            let score = (p.f_score as f64 / 9.0 * 100.0).clamp(0.0, 100.0);
            let w = 30.0;
            components.push(FactorComponent {
                name: "Piotroski F".to_string(),
                value: format!("{}/9 ({})", p.f_score, p.strength_label),
                score, weight: w, contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // MARGINS — weight 25. Fuse quality_label bucket + trend bonus.
    if let Some(m) = margins {
        if m.quality_label != "INSUFFICIENT_DATA" && !m.quality_label.is_empty() {
            operating_margin_pct = m.latest_operating_margin_pct;
            margin_trend_label = m.overall_trend_label.clone();
            let mut score: f64 = match m.quality_label.as_str() {
                "HIGH" => 85.0,
                "MEDIUM" => 60.0,
                "LOW" => 30.0,
                _ => 50.0,
            };
            match m.overall_trend_label.as_str() {
                "EXPANDING" => score = (score + 10.0).min(100.0),
                "CONTRACTING" => score = (score - 10.0).max(0.0),
                _ => {}
            }
            let w = 25.0;
            components.push(FactorComponent {
                name: "Margins".to_string(),
                value: format!("{} op {:.1}% ({})", m.quality_label, m.latest_operating_margin_pct, m.overall_trend_label),
                score, weight: w, contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // ACRL — weight 25. Fuse trend_label + ttm cash conversion bonus.
    if let Some(ac) = accruals {
        if !ac.trend_label.is_empty() {
            accruals_trend_label = ac.trend_label.clone();
            cash_conversion_pct = ac.ttm_cash_conversion_pct;
            let mut score: f64 = match ac.trend_label.as_str() {
                "IMPROVING" => 80.0,
                "STABLE" => 60.0,
                "MIXED" => 50.0,
                "DETERIORATING" => 30.0,
                _ => 50.0,
            };
            if ac.ttm_cash_conversion_pct >= 100.0 {
                score = (score + 10.0).min(100.0);
            } else if ac.ttm_cash_conversion_pct < 50.0 && ac.ttm_cash_conversion_pct != 0.0 {
                score = (score - 10.0).max(0.0);
            }
            let w = 25.0;
            components.push(FactorComponent {
                name: "Accruals".to_string(),
                value: format!("{} ({:.0}% cash conv)", ac.trend_label, ac.ttm_cash_conversion_pct),
                score, weight: w, contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // LEV — weight 20. Map solvency_summary label to a score + debt/ebitda.
    if let Some(lv) = leverage {
        if !lv.solvency_summary.is_empty() {
            leverage_summary = lv.solvency_summary.clone();
            debt_to_ebitda = if lv.ebitda_ttm > 0.0 { lv.total_debt / lv.ebitda_ttm } else { 0.0 };
            let score = match lv.solvency_summary.as_str() {
                "HEALTHY" => 85.0,
                "MODERATE" | "NEUTRAL" => 60.0,
                "ELEVATED" => 40.0,
                "STRETCHED" | "DISTRESSED" => 15.0,
                _ => 50.0,
            };
            let w = 20.0;
            components.push(FactorComponent {
                name: "Leverage".to_string(),
                value: format!("{} (D/EBITDA {:.2})", lv.solvency_summary, debt_to_ebitda),
                score, weight: w, contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    if inputs_available == 0 || total_weight <= 0.0 {
        return QualitySnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            quality_label: "NO_DATA".to_string(),
            note: "need at least one of PTFS / MARGINS / ACRL / LEV cached".to_string(),
            ..Default::default()
        };
    }

    let composite = (weighted_sum / total_weight).clamp(0.0, 100.0);
    let label = if composite >= 80.0 { "HIGH_QUALITY" }
                else if composite >= 65.0 { "QUALITY" }
                else if composite >= 45.0 { "AVERAGE" }
                else if composite >= 30.0 { "POOR" }
                else { "WEAK" };

    QualitySnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        piotroski_score,
        piotroski_label,
        operating_margin_pct,
        margin_trend_label,
        cash_conversion_pct,
        accruals_trend_label,
        leverage_summary,
        debt_to_ebitda,
        composite_score: composite,
        quality_label: label.to_string(),
        inputs_available,
        components,
        note: String::new(),
    }
}

/// RISK — Risk-factor composite fusing VOLE + BETA + LIQ + SHRT + ALTZ.
/// Higher composite_score = RISKIER.
pub fn compute_risk_snapshot(
    symbol: &str,
    as_of: &str,
    vole: Option<&OhlcVolSnapshot>,
    beta: Option<&BetaSnapshot>,
    liquidity: Option<&LiquiditySnapshot>,
    short_interest: Option<&ShortInterestSnapshot>,
    altman: Option<&AltmanZSnapshot>,
) -> RiskSnapshot {
    let sym = symbol.to_uppercase();
    let mut components: Vec<FactorComponent> = Vec::new();
    let mut total_weight = 0.0;
    let mut weighted_sum = 0.0;
    let mut inputs_available = 0usize;

    let mut realized_vol_pct = 0.0;
    let mut beta_1y = 0.0;
    let mut liquidity_tier = String::new();
    let mut short_percent_of_float = 0.0;
    let mut days_to_cover = 0.0;
    let mut altman_z = 0.0;
    let mut altman_zone = String::new();
    let mut distressed = false;

    // VOLE — weight 25. Higher vol → higher risk score.
    // 10% vol = 0, 30% = 50, 60% = 100 (linear piecewise).
    if let Some(v) = vole {
        if v.preferred_estimate_pct > 0.0 {
            realized_vol_pct = v.preferred_estimate_pct;
            let score = if v.preferred_estimate_pct <= 10.0 { 0.0 }
                        else if v.preferred_estimate_pct <= 30.0 {
                            (v.preferred_estimate_pct - 10.0) / 20.0 * 50.0
                        } else if v.preferred_estimate_pct <= 60.0 {
                            50.0 + (v.preferred_estimate_pct - 30.0) / 30.0 * 50.0
                        } else { 100.0 };
            let w = 25.0;
            components.push(FactorComponent {
                name: "Realized Vol".to_string(),
                value: format!("{:.1}% ({})", v.preferred_estimate_pct, v.preferred_label),
                score, weight: w, contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // BETA — weight 20. |β - 1| contributes to risk; high |β| far from 1 = high risk.
    if let Some(b) = beta {
        if let Some(one_y) = b.windows.iter().find(|w| w.window_label == "1Y") {
            if one_y.n_observations > 0 {
                beta_1y = one_y.beta;
                let dist = (one_y.beta - 1.0).abs();
                let score = (dist / 1.0 * 60.0).min(100.0);   // |β-1|=1 → 60; |β-1|>=1.67 → 100
                let w = 20.0;
                components.push(FactorComponent {
                    name: "Beta 1Y".to_string(),
                    value: format!("β {:.2}", one_y.beta),
                    score, weight: w, contribution: score * w / 100.0,
                });
                weighted_sum += score * w;
                total_weight += w;
                inputs_available += 1;
            }
        }
    }

    // LIQ — weight 15. Thin liquidity = high risk.
    if let Some(l) = liquidity {
        if l.liquidity_tier != "INSUFFICIENT_DATA" && !l.liquidity_tier.is_empty() {
            liquidity_tier = l.liquidity_tier.clone();
            let score = match l.liquidity_tier.as_str() {
                "DEEP" => 5.0,
                "LIQUID" => 20.0,
                "MODERATE" => 45.0,
                "THIN" => 75.0,
                "ILLIQUID" => 95.0,
                _ => 50.0,
            };
            let w = 15.0;
            components.push(FactorComponent {
                name: "Liquidity".to_string(),
                value: l.liquidity_tier.clone(),
                score, weight: w, contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // SHRT — weight 15. High short % of float + days to cover = high squeeze / sentiment risk.
    if let Some(s) = short_interest {
        if s.squeeze_risk_label != "INSUFFICIENT_DATA" && !s.squeeze_risk_label.is_empty() {
            short_percent_of_float = s.short_percent_of_float;
            days_to_cover = s.days_to_cover;
            let score = match s.squeeze_risk_label.as_str() {
                "LOW" => 20.0,
                "ELEVATED" => 55.0,
                "HIGH" => 80.0,
                "EXTREME" => 100.0,
                _ => 40.0,
            };
            let w = 15.0;
            components.push(FactorComponent {
                name: "Short Interest".to_string(),
                value: format!("{:.1}% float, {:.1} DTC ({})", s.short_percent_of_float, s.days_to_cover, s.squeeze_risk_label),
                score, weight: w, contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // ALTZ — weight 25. DISTRESS zone = highest risk.
    if let Some(a) = altman {
        if a.zone != "INSUFFICIENT_DATA" && !a.zone.is_empty() {
            altman_z = a.z_score;
            altman_zone = a.zone.clone();
            let score = match a.zone.as_str() {
                "SAFE" => 10.0,
                "GRAY" => 55.0,
                "DISTRESS" => { distressed = true; 95.0 }
                _ => 50.0,
            };
            let w = 25.0;
            components.push(FactorComponent {
                name: "Altman Z".to_string(),
                value: format!("Z {:.2} ({})", a.z_score, a.zone),
                score, weight: w, contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    if inputs_available == 0 || total_weight <= 0.0 {
        return RiskSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            risk_label: "NO_DATA".to_string(),
            note: "need at least one of VOLE / BETA / LIQ / SHRT / ALTZ cached".to_string(),
            ..Default::default()
        };
    }

    let composite = (weighted_sum / total_weight).clamp(0.0, 100.0);
    let label = if distressed { "DISTRESSED" }
                else if composite >= 75.0 { "HIGH_RISK" }
                else if composite >= 55.0 { "ELEVATED" }
                else if composite >= 35.0 { "MODERATE" }
                else { "LOW_RISK" };

    RiskSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        realized_vol_pct,
        beta_1y,
        liquidity_tier,
        short_percent_of_float,
        days_to_cover,
        altman_z,
        altman_zone,
        composite_score: composite,
        risk_label: label.to_string(),
        inputs_available,
        components,
        note: String::new(),
    }
}

/// INSSTRK — Insider streak detector from cached Form 4 trades.
pub fn compute_insstrk_snapshot(
    symbol: &str,
    as_of: &str,
    trades: &[InsiderTrade],
    window_days: i32,
) -> InsiderStreakSnapshot {
    let sym = symbol.to_uppercase();

    let as_of_days = parse_yyyy_mm_dd_to_days(as_of);
    let window_floor_days = as_of_days.map(|d| d - window_days as i64);

    // Filter to window.
    let mut filtered: Vec<&InsiderTrade> = trades.iter()
        .filter(|t| {
            let txn_days = parse_yyyy_mm_dd_to_days(&t.transaction_date);
            match (txn_days, window_floor_days) {
                (Some(td), Some(floor)) => td >= floor,
                _ => true,
            }
        })
        .collect();

    if filtered.is_empty() {
        return InsiderStreakSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            window_days,
            streak_label: "NONE".to_string(),
            note: "no insider trades within window".to_string(),
            ..Default::default()
        };
    }

    // Sort chronologically (oldest first) so streaks read naturally.
    filtered.sort_by(|a, b| a.transaction_date.cmp(&b.transaction_date));

    // Group by insider name.
    use std::collections::BTreeMap;
    let mut per_insider: BTreeMap<String, Vec<&InsiderTrade>> = BTreeMap::new();
    for t in &filtered {
        per_insider.entry(t.reporting_name.clone()).or_default().push(*t);
    }

    let unique_insiders = per_insider.len();
    let mut rows: Vec<InsiderStreakRow> = Vec::new();
    let mut buy_streak_count = 0usize;
    let mut sell_streak_count = 0usize;
    let mut longest_buy_streak = 0usize;
    let mut longest_sell_streak = 0usize;
    let mut net_buy_value_usd = 0.0;
    let mut net_sell_value_usd = 0.0;

    for (name, ts) in &per_insider {
        // Classify each trade BUY/SELL/OTHER from transaction_type or acquisition_disposition.
        let dir_of = |t: &InsiderTrade| -> &'static str {
            let tt = t.transaction_type.to_uppercase();
            if tt.starts_with("P") || tt.contains("PURCHASE") { return "BUY"; }
            if tt.starts_with("S") || tt.contains("SALE") { return "SELL"; }
            if t.acquisition_disposition.to_uppercase() == "A" { return "BUY"; }
            if t.acquisition_disposition.to_uppercase() == "D" { return "SELL"; }
            "OTHER"
        };

        // Longest consecutive run of same direction (BUY or SELL only, OTHER breaks).
        let mut longest_run: usize = 0;
        let mut longest_dir: &'static str = "MIXED";
        let mut cur_run: usize = 0;
        let mut cur_dir: &'static str = "";
        for t in ts {
            let d = dir_of(t);
            if d == "OTHER" {
                cur_run = 0;
                cur_dir = "";
                continue;
            }
            if d == cur_dir { cur_run += 1; }
            else { cur_run = 1; cur_dir = d; }
            if cur_run > longest_run {
                longest_run = cur_run;
                longest_dir = cur_dir;
            }
        }

        // Net signed totals for this insider in window.
        let mut net_value = 0.0;
        let mut net_shares = 0.0;
        let mut has_buy = false;
        let mut has_sell = false;
        for t in ts {
            let d = dir_of(t);
            if d == "BUY" { net_value += t.value_usd; net_shares += t.shares; has_buy = true; }
            else if d == "SELL" { net_value -= t.value_usd; net_shares -= t.shares; has_sell = true; }
        }

        let mixed = has_buy && has_sell;
        let row_dir = if mixed { "MIXED".to_string() }
                      else if has_buy { "BUY".to_string() }
                      else if has_sell { "SELL".to_string() }
                      else { "OTHER".to_string() };

        if row_dir == "BUY" && longest_run >= 2 { buy_streak_count += 1; }
        if row_dir == "SELL" && longest_run >= 2 { sell_streak_count += 1; }
        if longest_dir == "BUY" && longest_run > longest_buy_streak { longest_buy_streak = longest_run; }
        if longest_dir == "SELL" && longest_run > longest_sell_streak { longest_sell_streak = longest_run; }
        if row_dir == "BUY" { net_buy_value_usd += net_value.max(0.0); }
        if row_dir == "SELL" { net_sell_value_usd += (-net_value).max(0.0); }

        let first_date = ts.first().map(|t| t.transaction_date.clone()).unwrap_or_default();
        let latest_date = ts.last().map(|t| t.transaction_date.clone()).unwrap_or_default();

        rows.push(InsiderStreakRow {
            insider_name: name.clone(),
            streak_direction: row_dir,
            consecutive_events: longest_run,
            net_value_usd: net_value,
            net_shares,
            first_date,
            latest_date,
        });
    }

    // Sort rows: buys first, then by longest streak desc.
    rows.sort_by(|a, b| {
        let ka = match a.streak_direction.as_str() { "BUY" => 0, "SELL" => 1, "MIXED" => 2, _ => 3 };
        let kb = match b.streak_direction.as_str() { "BUY" => 0, "SELL" => 1, "MIXED" => 2, _ => 3 };
        ka.cmp(&kb).then(b.consecutive_events.cmp(&a.consecutive_events))
    });

    let label = if buy_streak_count >= 3 && longest_buy_streak >= 4 {
        "STRONG_ACCUMULATION"
    } else if sell_streak_count >= 3 && longest_sell_streak >= 4 {
        "STRONG_DISTRIBUTION"
    } else if buy_streak_count >= 2 && sell_streak_count >= 2 {
        "MIXED"
    } else if buy_streak_count >= 2 {
        "ACCUMULATION"
    } else if sell_streak_count >= 2 {
        "DISTRIBUTION"
    } else if buy_streak_count > 0 || sell_streak_count > 0 {
        "MIXED"
    } else {
        "NONE"
    };

    InsiderStreakSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        window_days,
        unique_insiders,
        buy_streak_count,
        sell_streak_count,
        longest_buy_streak,
        longest_sell_streak,
        net_buy_value_usd,
        net_sell_value_usd,
        streak_label: label.to_string(),
        rows,
        note: String::new(),
    }
}

/// COVG — Analyst coverage breadth + churn snapshot.
pub fn compute_covg_snapshot(
    symbol: &str,
    as_of: &str,
    price_target: Option<&PriceTarget>,
    recs: &[AnalystRecommendation],
    updm: Option<&UpdmSnapshot>,
) -> CoverageSnapshot {
    let sym = symbol.to_uppercase();
    let mut inputs_available = 0usize;

    let mut num_analysts = 0;
    let mut target_mean = 0.0;
    let mut target_low = 0.0;
    let mut target_high = 0.0;
    if let Some(pt) = price_target {
        num_analysts = pt.num_analysts;
        target_mean = pt.target_mean;
        target_low = pt.target_low;
        target_high = pt.target_high;
        if num_analysts > 0 || target_mean > 0.0 {
            inputs_available += 1;
        }
    }

    // Consensus distribution from latest AnalystRecommendation row (sorted chronologically).
    let mut sb = 0; let mut b = 0; let mut h = 0; let mut s = 0; let mut ss = 0;
    if !recs.is_empty() {
        let mut sorted = recs.to_vec();
        sorted.sort_by(|a, b| a.period.cmp(&b.period));
        if let Some(latest) = sorted.last() {
            sb = latest.strong_buy;
            b = latest.buy;
            h = latest.hold;
            s = latest.sell;
            ss = latest.strong_sell;
            if (sb + b + h + s + ss) > 0 {
                inputs_available += 1;
            }
        }
    }
    let total_recs = sb + b + h + s + ss;
    let bull_ratio = if total_recs > 0 { (sb + b) as f64 / total_recs as f64 } else { 0.0 };

    // UPDM — churn activity (upgrades/downgrades 90d).
    let mut upgrades_90d = 0usize;
    let mut downgrades_90d = 0usize;
    let mut net_90d = 0i32;
    if let Some(u) = updm {
        if u.total_actions > 0 {
            upgrades_90d = u.upgrades_90d;
            downgrades_90d = u.downgrades_90d;
            net_90d = u.net_90d;
            inputs_available += 1;
        }
    }
    let churn_90d = upgrades_90d + downgrades_90d;

    if inputs_available == 0 {
        return CoverageSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            coverage_label: "NONE".to_string(),
            note: "need PriceTarget / AnalystRecommendations / UPDM cached".to_string(),
            ..Default::default()
        };
    }

    // Breadth — num_analysts normalized: ≥20 = 100, 0 = 0.
    let breadth = ((num_analysts as f64 / 20.0) * 100.0).clamp(0.0, 100.0);
    // Consensus — bull ratio × 100.
    let consensus = (bull_ratio * 100.0).clamp(0.0, 100.0);
    // Churn — net_90d centered at 50, ±5 per net action.
    let churn = (50.0 + (net_90d as f64) * 5.0).clamp(0.0, 100.0);

    let composite = breadth * 0.35 + consensus * 0.35 + churn * 0.30;

    let label = if num_analysts > 0 && num_analysts < 5 {
        "THIN"
    } else if net_90d >= 3 && breadth >= 70.0 {
        "EXPANDING"
    } else if net_90d <= -3 {
        "CONTRACTING"
    } else if composite >= 50.0 {
        "STABLE"
    } else if inputs_available == 0 {
        "NONE"
    } else {
        "STABLE"
    };

    CoverageSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        num_analysts,
        target_mean,
        target_low,
        target_high,
        consensus_strong_buy: sb,
        consensus_buy: b,
        consensus_hold: h,
        consensus_sell: s,
        consensus_strong_sell: ss,
        consensus_total: total_recs,
        consensus_bull_ratio: bull_ratio,
        upgrades_90d,
        downgrades_90d,
        net_90d,
        churn_90d,
        breadth_score: breadth,
        consensus_score: consensus,
        churn_score: churn,
        composite_score: composite,
        coverage_label: label.to_string(),
        inputs_available,
        note: String::new(),
    }
}

// ── ADR-123 Godel Parity Round 16 compute fns ──────────────────────────────

/// Simple quartile at `q ∈ [0,1]` via linear interpolation on a sorted slice.
/// Used by the Round 16 rank surfaces for p25 / p75 sector markers.
pub(crate) fn quantile_f64(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let idx = q * (sorted.len() as f64 - 1.0);
    let lo = idx.floor() as usize;
    let hi = idx.ceil() as usize;
    if lo == hi {
        return sorted[lo];
    }
    let frac = idx - lo as f64;
    sorted[lo] * (1.0 - frac) + sorted[hi] * frac
}

/// Percentile-rank `value` vs `others` using the
/// `(below + 0.5 × equal) / total × 100` midrank convention.
/// When `higher_is_better == false`, the returned rank is inverted so that
/// a smaller input value yields a higher percentile (used by RRK where
/// composite is higher = riskier).
pub(crate) fn percentile_rank_score(value: f64, others: &[f64], higher_is_better: bool) -> f64 {
    let total = others.len() + 1;
    if total < 2 {
        return 50.0;
    }
    let (mut below, mut equal) = (0usize, 0usize);
    for &o in others {
        if (o - value).abs() < 1e-9 {
            equal += 1;
        } else if higher_is_better {
            if o < value { below += 1; }
        } else {
            if o > value { below += 1; }
        }
    }
    let raw = (below as f64 + 0.5 * equal as f64 + 0.5) / total as f64 * 100.0;
    raw.clamp(0.0, 100.0)
}

/// Standard 6-bucket rank label ladder for VRK / QRK.
pub(crate) fn rank_label_for_percentile(pct: f64) -> &'static str {
    if pct >= 90.0 { "TOP_DECILE" }
    else if pct >= 75.0 { "TOP_QUARTILE" }
    else if pct >= 50.0 { "ABOVE_MEDIAN" }
    else if pct >= 25.0 { "BELOW_MEDIAN" }
    else if pct >= 10.0 { "BOTTOM_QUARTILE" }
    else { "BOTTOM_DECILE" }
}

/// Risk-inverted rank label ladder for RRK (higher rank = safer).
pub(crate) fn risk_rank_label_for_percentile(pct: f64) -> &'static str {
    if pct >= 90.0 { "SAFEST_DECILE" }
    else if pct >= 75.0 { "SAFEST_QUARTILE" }
    else if pct >= 50.0 { "ABOVE_MEDIAN_SAFE" }
    else if pct >= 25.0 { "BELOW_MEDIAN_RISKY" }
    else if pct >= 10.0 { "BOTTOM_QUARTILE_RISKY" }
    else { "RISKIEST_DECILE" }
}

/// VRK — Value Rank vs sector peers.
///
/// Takes the subject's `ValueSnapshot` and a slice of peer snapshots
/// (caller filters to the same sector). Returns a percentile rank with the
/// standard 6-bucket label ladder. Higher percentile = better value.
pub fn compute_vrk_snapshot(
    symbol: &str,
    as_of: &str,
    subject: Option<&ValueSnapshot>,
    peers: &[&ValueSnapshot],
) -> ValueRankSnapshot {
    let subj = match subject {
        Some(s) if s.value_label != "NO_DATA" && s.composite_score > 0.0 => s,
        _ => {
            return ValueRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No VAL snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_scores: Vec<f64> = peers
        .iter()
        .filter(|p| p.value_label != "NO_DATA" && p.composite_score > 0.0)
        .map(|p| p.composite_score)
        .collect();
    if peer_scores.len() < 3 {
        return ValueRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: subj.sector.clone(),
            composite_score: subj.composite_score,
            peers_considered: peer_scores.len(),
            peers_with_data: peer_scores.len(),
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} VAL peers in sector {} (need ≥3)",
                peer_scores.len(),
                subj.sector
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_scores.clone();
    sorted.push(subj.composite_score);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.composite_score, &peer_scores, true);
    // 1-based rank position: count peers strictly better than subject + 1.
    let better = peer_scores.iter().filter(|&&p| p > subj.composite_score).count();
    let rank_position = better + 1;
    let label = rank_label_for_percentile(pct);
    ValueRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: subj.sector.clone(),
        composite_score: subj.composite_score,
        peers_considered: peer_scores.len(),
        peers_with_data: peer_scores.len(),
        sector_median_score: median,
        sector_p25: p25,
        sector_p75: p75,
        percentile_rank: pct,
        rank_position,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// QRK — Quality Rank vs sector peers.
///
/// `QualitySnapshot` does not carry sector — caller must supply it (typically
/// from `fundamentals::get_fundamentals(symbol).sector`), and peers must be
/// pre-filtered to the same sector.
pub fn compute_qrk_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&QualitySnapshot>,
    peers: &[&QualitySnapshot],
) -> QualityRankSnapshot {
    let subj = match subject {
        Some(s) if s.quality_label != "NO_DATA" && s.composite_score > 0.0 => s,
        _ => {
            return QualityRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No QUAL snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_scores: Vec<f64> = peers
        .iter()
        .filter(|p| p.quality_label != "NO_DATA" && p.composite_score > 0.0)
        .map(|p| p.composite_score)
        .collect();
    if peer_scores.len() < 3 {
        return QualityRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            composite_score: subj.composite_score,
            peers_considered: peer_scores.len(),
            peers_with_data: peer_scores.len(),
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} QUAL peers in sector {} (need ≥3)",
                peer_scores.len(),
                sector
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_scores.clone();
    sorted.push(subj.composite_score);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.composite_score, &peer_scores, true);
    let better = peer_scores.iter().filter(|&&p| p > subj.composite_score).count();
    let label = rank_label_for_percentile(pct);
    QualityRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        composite_score: subj.composite_score,
        peers_considered: peer_scores.len(),
        peers_with_data: peer_scores.len(),
        sector_median_score: median,
        sector_p25: p25,
        sector_p75: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// RRK — Risk Rank vs sector peers.
///
/// Percentile rank is *inverted* relative to VRK/QRK: RISK composite is
/// higher = riskier, so this surface treats a **lower** composite as **better**
/// and reports "higher percentile = safer." Label ladder uses
/// SAFEST_DECILE..RISKIEST_DECILE phrasing so the inversion is explicit.
pub fn compute_rrk_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&RiskSnapshot>,
    peers: &[&RiskSnapshot],
) -> RiskRankSnapshot {
    let subj = match subject {
        Some(s) if s.risk_label != "NO_DATA" && s.composite_score > 0.0 => s,
        _ => {
            return RiskRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No RISK snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_scores: Vec<f64> = peers
        .iter()
        .filter(|p| p.risk_label != "NO_DATA" && p.composite_score > 0.0)
        .map(|p| p.composite_score)
        .collect();
    if peer_scores.len() < 3 {
        return RiskRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            composite_score: subj.composite_score,
            peers_considered: peer_scores.len(),
            peers_with_data: peer_scores.len(),
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} RISK peers in sector {} (need ≥3)",
                peer_scores.len(),
                sector
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_scores.clone();
    sorted.push(subj.composite_score);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    // INVERSION: higher_is_better = false because RISK composite is higher = riskier.
    let pct = percentile_rank_score(subj.composite_score, &peer_scores, false);
    // 1-based: rank position counted by how many peers are SAFER (lower composite).
    let safer = peer_scores.iter().filter(|&&p| p < subj.composite_score).count();
    let label = risk_rank_label_for_percentile(pct);
    RiskRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        composite_score: subj.composite_score,
        peers_considered: peer_scores.len(),
        peers_with_data: peer_scores.len(),
        sector_median_score: median,
        sector_p25: p25,
        sector_p75: p75,
        percentile_rank: pct,
        rank_position: safer + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// Compute 3-year EPS CAGR from a `FinancialStatements`.
/// Prefers annual rows (latest[0] vs latest[3] → 3y CAGR). Returns
/// `(latest_eps, earliest_eps, years_used, cagr_pct)` where `cagr_pct` is
/// `f64::NAN` if the sign-rule rejects the series.
pub(crate) fn eps_cagr_3y_from_statements(statements: &FinancialStatements) -> (f64, f64, usize, f64) {
    let annuals = &statements.income_annual;
    if annuals.len() < 4 {
        return (0.0, 0.0, 0, f64::NAN);
    }
    // Rows are assumed newest-first per the Finnhub fetcher convention.
    let latest = annuals[0].eps;
    let earliest = annuals[3].eps;
    let years = 3usize;
    // CAGR only valid when both endpoints are strictly positive.
    if latest > 0.0 && earliest > 0.0 {
        let cagr = ((latest / earliest).powf(1.0 / years as f64) - 1.0) * 100.0;
        (latest, earliest, years, cagr)
    } else if latest.is_finite() && earliest.is_finite() && earliest.abs() > 1e-9 {
        // Degrade gracefully to a linear annualised growth when signs cross:
        // this is the "CAGR_NEGATIVE" path — the snapshot label captures it.
        let linear = (latest - earliest) / earliest.abs() / years as f64 * 100.0;
        (latest, earliest, years, linear)
    } else {
        (latest, earliest, years, f64::NAN)
    }
}

/// RELEPSGR — Relative 3-year EPS CAGR vs sector median.
///
/// Computes the subject's 3y EPS CAGR and the median CAGR of the peer slice,
/// then labels the subject relative to the sector median. Labels:
/// FAR_ABOVE (≥ +15pp), ABOVE (≥ +5pp), INLINE (within ±5pp), BELOW (≤ -5pp),
/// FAR_BELOW (≤ -15pp), CAGR_NEGATIVE (sign-crossed subject EPS),
/// NO_DATA (insufficient annual rows or empty peer set).
pub fn compute_relepsgr_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&FinancialStatements>,
    peer_statements: &[(String, FinancialStatements)],
) -> RelativeEpsGrowthSnapshot {
    let subj = match subject {
        Some(s) if s.income_annual.len() >= 4 => s,
        _ => {
            return RelativeEpsGrowthSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                relative_label: "NO_DATA".into(),
                note: "Subject has < 4 annual income rows".into(),
                ..Default::default()
            };
        }
    };
    let (latest, earliest, years, subj_cagr) = eps_cagr_3y_from_statements(subj);
    let mut peer_cagrs: Vec<f64> = Vec::new();
    for (_, st) in peer_statements {
        if st.income_annual.len() < 4 {
            continue;
        }
        let (_, _, _, c) = eps_cagr_3y_from_statements(st);
        if c.is_finite() {
            peer_cagrs.push(c);
        }
    }
    let peers_considered = peer_statements.len();
    let peers_with_data = peer_cagrs.len();
    if peer_cagrs.len() < 3 {
        return RelativeEpsGrowthSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            latest_eps: latest,
            earliest_eps: earliest,
            years_used: years,
            symbol_cagr_pct: if subj_cagr.is_finite() { subj_cagr } else { 0.0 },
            peers_considered,
            peers_with_data,
            relative_label: "NO_DATA".into(),
            note: format!("Only {} peers with ≥4 annual rows (need ≥3)", peer_cagrs.len()),
            ..Default::default()
        };
    }
    let mut sorted = peer_cagrs.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    if !subj_cagr.is_finite() || latest <= 0.0 || earliest <= 0.0 {
        return RelativeEpsGrowthSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            latest_eps: latest,
            earliest_eps: earliest,
            years_used: years,
            symbol_cagr_pct: if subj_cagr.is_finite() { subj_cagr } else { 0.0 },
            peers_considered,
            peers_with_data,
            sector_median_cagr_pct: median,
            sector_p25_cagr_pct: p25,
            sector_p75_cagr_pct: p75,
            relative_label: "CAGR_NEGATIVE".into(),
            note: "Subject EPS crosses zero; using linear proxy".into(),
            ..Default::default()
        };
    }
    let gap = subj_cagr - median;
    let label = if gap >= 15.0 { "FAR_ABOVE" }
        else if gap >= 5.0 { "ABOVE" }
        else if gap >= -5.0 { "INLINE" }
        else if gap >= -15.0 { "BELOW" }
        else { "FAR_BELOW" };
    RelativeEpsGrowthSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        latest_eps: latest,
        earliest_eps: earliest,
        years_used: years,
        symbol_cagr_pct: subj_cagr,
        peers_considered,
        peers_with_data,
        sector_median_cagr_pct: median,
        sector_p25_cagr_pct: p25,
        sector_p75_cagr_pct: p75,
        gap_to_median_pp: gap,
        relative_label: label.into(),
        note: String::new(),
    }
}

/// Locate the index of the first bar with `date >= target_date` in a
/// newest-first HP bar slice. Returns `None` if no such bar exists.
pub(crate) fn find_t0_index_newest_first(bars: &[HistoricalPriceRow], target_date: &str) -> Option<usize> {
    // Scan from oldest to newest (reverse iteration) and return the first
    // bar that is on-or-after the target. "newest-first" means bars[0] is
    // the most recent trading day.
    let mut best: Option<usize> = None;
    for (i, b) in bars.iter().enumerate() {
        if b.date.as_str() >= target_date {
            best = Some(i);
        } else {
            break;
        }
    }
    best
}

/// PEAD — Post-Earnings-Announcement Drift snapshot.
///
/// For each surprise row, locate `T0` in the HP bar slice (first trading day
/// at or after the announcement date), then compute forward drift over 1 / 3 /
/// 5 / 10 trading days. Averages over all successfully-matched events.
/// Returns INSUFFICIENT_DATA if fewer than 3 events match.
pub fn compute_pead_snapshot(
    symbol: &str,
    as_of: &str,
    surprises: &[EarningsSurprise],
    bars_newest_first: &[HistoricalPriceRow],
) -> PeadSnapshot {
    let num_events = surprises.len();
    if num_events == 0 || bars_newest_first.len() < 11 {
        return PeadSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            num_events,
            events_used: 0,
            drift_direction_label: "INSUFFICIENT_DATA".into(),
            note: if num_events == 0 {
                "No earnings surprises cached".into()
            } else {
                format!("Need ≥11 HP bars, have {}", bars_newest_first.len())
            },
            ..Default::default()
        };
    }
    let mut rows: Vec<PeadEventRow> = Vec::new();
    let mut beat_drifts_5d: Vec<f64> = Vec::new();
    let mut miss_drifts_5d: Vec<f64> = Vec::new();
    let mut all_1d: Vec<f64> = Vec::new();
    let mut all_3d: Vec<f64> = Vec::new();
    let mut all_5d: Vec<f64> = Vec::new();
    let mut all_10d: Vec<f64> = Vec::new();
    for surprise in surprises {
        let t0_idx = match find_t0_index_newest_first(bars_newest_first, &surprise.date) {
            Some(i) => i,
            None => continue,
        };
        // drift_Nd: close(t0 - N days back in newest-first ordering) vs close(t0).
        // Because bars are newest-first, "N trading days forward" means a
        // *smaller* index. Subtract N from t0_idx.
        if t0_idx < 10 {
            continue;
        }
        let t0_close = bars_newest_first[t0_idx].close;
        if t0_close <= 0.0 {
            continue;
        }
        let drift = |n: usize| {
            let fwd = &bars_newest_first[t0_idx - n];
            (fwd.close / t0_close - 1.0) * 100.0
        };
        let d1 = drift(1);
        let d3 = drift(3);
        let d5 = drift(5);
        let d10 = drift(10);
        let classification = if surprise.surprise_pct > 2.0 {
            "BEAT"
        } else if surprise.surprise_pct < -2.0 {
            "MISS"
        } else {
            "INLINE"
        };
        match classification {
            "BEAT" => beat_drifts_5d.push(d5),
            "MISS" => miss_drifts_5d.push(d5),
            _ => {}
        }
        all_1d.push(d1);
        all_3d.push(d3);
        all_5d.push(d5);
        all_10d.push(d10);
        rows.push(PeadEventRow {
            event_date: surprise.date.clone(),
            surprise_pct: surprise.surprise_pct,
            classification: classification.into(),
            drift_1d_pct: d1,
            drift_3d_pct: d3,
            drift_5d_pct: d5,
            drift_10d_pct: d10,
        });
    }
    let events_used = rows.len();
    if events_used < 3 {
        return PeadSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            num_events,
            events_used,
            drift_direction_label: "INSUFFICIENT_DATA".into(),
            note: format!("Matched only {} events to HP bars (need ≥3)", events_used),
            rows,
            ..Default::default()
        };
    }
    let mean = |v: &[f64]| if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 };
    let avg_1d = mean(&all_1d);
    let avg_3d = mean(&all_3d);
    let avg_5d = mean(&all_5d);
    let avg_10d = mean(&all_10d);
    let beat_5d = mean(&beat_drifts_5d);
    let miss_5d = mean(&miss_drifts_5d);
    // Sort rows newest-first (highest event_date string first) for stable display.
    let mut sorted_rows = rows.clone();
    sorted_rows.sort_by(|a, b| b.event_date.cmp(&a.event_date));
    let latest = sorted_rows.first().cloned().unwrap_or_default();
    let label = if avg_5d >= 2.0 { "DRIFT_UP" }
        else if avg_5d <= -2.0 { "DRIFT_DOWN" }
        else { "MIXED" };
    let display_rows: Vec<PeadEventRow> = sorted_rows.into_iter().take(8).collect();
    PeadSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        num_events,
        events_used,
        avg_drift_1d_pct: avg_1d,
        avg_drift_3d_pct: avg_3d,
        avg_drift_5d_pct: avg_5d,
        avg_drift_10d_pct: avg_10d,
        beat_event_drift_5d_pct: beat_5d,
        miss_event_drift_5d_pct: miss_5d,
        latest_event_date: latest.event_date.clone(),
        latest_event_surprise_pct: latest.surprise_pct,
        latest_event_drift_5d_pct: latest.drift_5d_pct,
        drift_direction_label: label.into(),
        rows: display_rows,
        note: String::new(),
    }
}

// ── ADR-124 Round 17 — rank surfaces + FQM + revenue growth ────────────────

/// Market-cap tier classifier (absolute dollar thresholds).
fn size_tier_label(market_cap: f64) -> &'static str {
    if market_cap >= 200_000_000_000.0 { "MEGA_CAP" }
    else if market_cap >= 10_000_000_000.0 { "LARGE_CAP" }
    else if market_cap >= 2_000_000_000.0 { "MID_CAP" }
    else if market_cap >= 300_000_000.0 { "SMALL_CAP" }
    else if market_cap > 0.0 { "MICRO_CAP" }
    else { "NO_DATA" }
}

/// SIZEF — Size Factor Rank vs sector peers.
///
/// Callers pass the subject's market cap + sector and a slice of
/// `(symbol, market_cap)` tuples for sector peers. Returns a percentile
/// rank (higher = larger) plus a tier label derived from absolute cap.
pub fn compute_sizef_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject_market_cap: Option<f64>,
    peers: &[(String, f64)],
) -> SizeFactorSnapshot {
    let cap = match subject_market_cap {
        Some(c) if c > 0.0 => c,
        _ => {
            return SizeFactorSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                tier_label: "NO_DATA".into(),
                rank_label: "NO_DATA".into(),
                note: "No market cap on file for subject".into(),
                ..Default::default()
            };
        }
    };
    let tier = size_tier_label(cap);
    let peer_caps: Vec<f64> = peers
        .iter()
        .filter(|(_, c)| *c > 0.0)
        .map(|(_, c)| *c)
        .collect();
    if peer_caps.len() < 3 {
        return SizeFactorSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            market_cap: cap,
            log_market_cap: cap.ln(),
            tier_label: tier.into(),
            peers_considered: peer_caps.len(),
            peers_with_data: peer_caps.len(),
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} peers with market cap in sector {} (need ≥3)",
                peer_caps.len(),
                sector
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_caps.clone();
    sorted.push(cap);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(cap, &peer_caps, true);
    let better = peer_caps.iter().filter(|&&c| c > cap).count();
    let label = rank_label_for_percentile(pct);
    SizeFactorSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        market_cap: cap,
        log_market_cap: cap.ln(),
        tier_label: tier.into(),
        peers_considered: peer_caps.len(),
        peers_with_data: peer_caps.len(),
        sector_median_cap: median,
        sector_p25_cap: p25,
        sector_p75_cap: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// MOMF — Momentum Factor Rank vs sector peers.
///
/// `MomentumSnapshot` does not carry sector — caller must supply it and
/// pre-filter peers to the same sector.
pub fn compute_momf_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&MomentumSnapshot>,
    peers: &[&MomentumSnapshot],
) -> MomentumRankSnapshot {
    let subj = match subject {
        Some(s) if s.regime_label != "INSUFFICIENT_DATA" && s.composite_score > 0.0 => s,
        _ => {
            return MomentumRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No MOMENTUM snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_scores: Vec<f64> = peers
        .iter()
        .filter(|p| p.regime_label != "INSUFFICIENT_DATA" && p.composite_score > 0.0)
        .map(|p| p.composite_score)
        .collect();
    if peer_scores.len() < 3 {
        return MomentumRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            composite_score: subj.composite_score,
            peers_considered: peer_scores.len(),
            peers_with_data: peer_scores.len(),
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} MOMENTUM peers in sector {} (need ≥3)",
                peer_scores.len(),
                sector
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_scores.clone();
    sorted.push(subj.composite_score);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.composite_score, &peer_scores, true);
    let better = peer_scores.iter().filter(|&&p| p > subj.composite_score).count();
    let label = rank_label_for_percentile(pct);
    MomentumRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        composite_score: subj.composite_score,
        peers_considered: peer_scores.len(),
        peers_with_data: peer_scores.len(),
        sector_median_score: median,
        sector_p25: p25,
        sector_p75: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// PEADRANK — Post-Earnings Drift Rank vs sector peers.
///
/// Peers must have a valid PEAD snapshot (`drift_direction_label !=
/// "INSUFFICIENT_DATA"` and `events_used >= 3`). Higher percentile =
/// stronger positive drift.
pub fn compute_peadrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&PeadSnapshot>,
    peers: &[&PeadSnapshot],
) -> PeadRankSnapshot {
    let subj = match subject {
        Some(s) if s.drift_direction_label != "INSUFFICIENT_DATA" && s.events_used >= 3 => s,
        _ => {
            return PeadRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No valid PEAD snapshot for subject (need ≥3 events)".into(),
                ..Default::default()
            };
        }
    };
    let peer_drifts: Vec<f64> = peers
        .iter()
        .filter(|p| p.drift_direction_label != "INSUFFICIENT_DATA" && p.events_used >= 3)
        .map(|p| p.avg_drift_5d_pct)
        .collect();
    if peer_drifts.len() < 3 {
        return PeadRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            avg_drift_5d_pct: subj.avg_drift_5d_pct,
            peers_considered: peer_drifts.len(),
            peers_with_data: peer_drifts.len(),
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} valid PEAD peers in sector {} (need ≥3)",
                peer_drifts.len(),
                sector
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_drifts.clone();
    sorted.push(subj.avg_drift_5d_pct);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.avg_drift_5d_pct, &peer_drifts, true);
    let better = peer_drifts.iter().filter(|&&d| d > subj.avg_drift_5d_pct).count();
    let label = rank_label_for_percentile(pct);
    PeadRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        avg_drift_5d_pct: subj.avg_drift_5d_pct,
        peers_considered: peer_drifts.len(),
        peers_with_data: peer_drifts.len(),
        sector_median_drift_5d_pct: median,
        sector_p25_drift_5d_pct: p25,
        sector_p75_drift_5d_pct: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// FQM — Fundamental Quality Meter.
///
/// One-layer composite over PTFS + MARGINS + ACRL (weights 40/30/30).
/// Deliberately excludes LEV so the score reflects **operational** quality
/// (does the business machine convert sales into durable cash?) rather than
/// capital-structure strength. A highly-levered business with elite margins
/// and strong cash conversion will FQM-high and QUAL-mid — that's the
/// intended divergence from ADR-122 QUAL.
pub fn compute_fqm_snapshot(
    symbol: &str,
    as_of: &str,
    piotroski: Option<&PiotroskiSnapshot>,
    margins: Option<&MarginsSnapshot>,
    accruals: Option<&AccrualsSnapshot>,
) -> FundamentalQualityMeterSnapshot {
    let sym = symbol.to_uppercase();
    let mut components: Vec<FactorComponent> = Vec::new();
    let mut total_weight = 0.0;
    let mut weighted_sum = 0.0;
    let mut inputs_available = 0i32;

    let mut piotroski_score = 0;
    let mut piotroski_label = String::new();
    let mut operating_margin_pct = 0.0;
    let mut margin_trend_label = String::new();
    let mut cash_conversion_pct = 0.0;
    let mut accruals_trend_label = String::new();

    // PTFS — weight 40.
    if let Some(p) = piotroski {
        if p.strength_label != "INSUFFICIENT_DATA" && !p.strength_label.is_empty() {
            piotroski_score = p.f_score;
            piotroski_label = p.strength_label.clone();
            let score = (p.f_score as f64 / 9.0 * 100.0).clamp(0.0, 100.0);
            let w = 40.0;
            components.push(FactorComponent {
                name: "Piotroski F".to_string(),
                value: format!("{}/9 ({})", p.f_score, p.strength_label),
                score, weight: w, contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // MARGINS — weight 30.
    if let Some(m) = margins {
        if m.quality_label != "INSUFFICIENT_DATA" && !m.quality_label.is_empty() {
            operating_margin_pct = m.latest_operating_margin_pct;
            margin_trend_label = m.overall_trend_label.clone();
            let mut score: f64 = match m.quality_label.as_str() {
                "HIGH" => 85.0,
                "MEDIUM" => 60.0,
                "LOW" => 30.0,
                _ => 50.0,
            };
            match m.overall_trend_label.as_str() {
                "EXPANDING" => score = (score + 10.0).min(100.0),
                "CONTRACTING" => score = (score - 10.0).max(0.0),
                _ => {}
            }
            let w = 30.0;
            components.push(FactorComponent {
                name: "Margins".to_string(),
                value: format!("{} op {:.1}% ({})", m.quality_label, m.latest_operating_margin_pct, m.overall_trend_label),
                score, weight: w, contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // ACRL — weight 30.
    if let Some(ac) = accruals {
        if !ac.trend_label.is_empty() {
            accruals_trend_label = ac.trend_label.clone();
            cash_conversion_pct = ac.ttm_cash_conversion_pct;
            let mut score: f64 = match ac.trend_label.as_str() {
                "IMPROVING" => 80.0,
                "STABLE" => 60.0,
                "MIXED" => 50.0,
                "DETERIORATING" => 30.0,
                _ => 50.0,
            };
            if ac.ttm_cash_conversion_pct >= 100.0 {
                score = (score + 10.0).min(100.0);
            } else if ac.ttm_cash_conversion_pct < 50.0 && ac.ttm_cash_conversion_pct != 0.0 {
                score = (score - 10.0).max(0.0);
            }
            let w = 30.0;
            components.push(FactorComponent {
                name: "Accruals".to_string(),
                value: format!("{} ({:.0}% cash conv)", ac.trend_label, ac.ttm_cash_conversion_pct),
                score, weight: w, contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    if inputs_available == 0 || total_weight <= 0.0 {
        return FundamentalQualityMeterSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            operator_label: "NO_DATA".to_string(),
            note: "need at least one of PTFS / MARGINS / ACRL cached".to_string(),
            ..Default::default()
        };
    }

    let composite = (weighted_sum / total_weight).clamp(0.0, 100.0);
    let label = if composite >= 85.0 { "ELITE_OPERATOR" }
                else if composite >= 70.0 { "STRONG_OPERATOR" }
                else if composite >= 50.0 { "AVERAGE_OPERATOR" }
                else if composite >= 30.0 { "WEAK_OPERATOR" }
                else { "BROKEN_OPERATOR" };

    FundamentalQualityMeterSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        piotroski_score,
        piotroski_label,
        operating_margin_pct,
        margin_trend_label,
        cash_conversion_pct,
        accruals_trend_label,
        composite_score: composite,
        operator_label: label.to_string(),
        inputs_available,
        components,
        note: String::new(),
    }
}

/// Compute 3-year revenue CAGR from a `FinancialStatements`.
/// Returns `(latest_rev, earliest_rev, years_used, cagr_pct)`. CAGR is
/// `f64::NAN` if the sign rule rejects the series (revenue must both be
/// strictly positive — revenue rarely crosses zero, so NAN usually signals
/// missing data).
fn revenue_cagr_3y_from_statements(statements: &FinancialStatements) -> (f64, f64, usize, f64) {
    let annuals = &statements.income_annual;
    if annuals.len() < 4 {
        return (0.0, 0.0, 0, f64::NAN);
    }
    let latest = annuals[0].revenue;
    let earliest = annuals[3].revenue;
    let years = 3usize;
    if latest > 0.0 && earliest > 0.0 {
        let cagr = ((latest / earliest).powf(1.0 / years as f64) - 1.0) * 100.0;
        (latest, earliest, years, cagr)
    } else if latest.is_finite() && earliest.is_finite() && earliest.abs() > 1e-9 {
        let linear = (latest - earliest) / earliest.abs() / years as f64 * 100.0;
        (latest, earliest, years, linear)
    } else {
        (latest, earliest, years, f64::NAN)
    }
}

/// REVRANK — Relative Revenue Growth Rank.
///
/// Mirrors RELEPSGR but over `IncomeStatement.revenue` instead of EPS.
/// Label ladder: FAR_ABOVE (≥+15pp), ABOVE (≥+5pp), INLINE (±5pp),
/// BELOW (≤-5pp), FAR_BELOW (≤-15pp), CAGR_NEGATIVE (subject endpoints
/// non-positive), NO_DATA.
pub fn compute_revrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&FinancialStatements>,
    peer_statements: &[(String, FinancialStatements)],
) -> RevenueGrowthRankSnapshot {
    let subj = match subject {
        Some(s) if s.income_annual.len() >= 4 => s,
        _ => {
            return RevenueGrowthRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                relative_label: "NO_DATA".into(),
                note: "Subject has < 4 annual income rows".into(),
                ..Default::default()
            };
        }
    };
    let (latest, earliest, years, subj_cagr) = revenue_cagr_3y_from_statements(subj);
    let mut peer_cagrs: Vec<f64> = Vec::new();
    for (_, st) in peer_statements {
        if st.income_annual.len() < 4 {
            continue;
        }
        let (_, _, _, c) = revenue_cagr_3y_from_statements(st);
        if c.is_finite() {
            peer_cagrs.push(c);
        }
    }
    let peers_considered = peer_statements.len();
    let peers_with_data = peer_cagrs.len();
    if peer_cagrs.len() < 3 {
        return RevenueGrowthRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            latest_revenue: latest,
            earliest_revenue: earliest,
            years_used: years,
            symbol_cagr_pct: if subj_cagr.is_finite() { subj_cagr } else { 0.0 },
            peers_considered,
            peers_with_data,
            relative_label: "NO_DATA".into(),
            note: format!("Only {} peers with ≥4 annual rows (need ≥3)", peer_cagrs.len()),
            ..Default::default()
        };
    }
    let mut sorted = peer_cagrs.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    if !subj_cagr.is_finite() || latest <= 0.0 || earliest <= 0.0 {
        return RevenueGrowthRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            latest_revenue: latest,
            earliest_revenue: earliest,
            years_used: years,
            symbol_cagr_pct: if subj_cagr.is_finite() { subj_cagr } else { 0.0 },
            peers_considered,
            peers_with_data,
            sector_median_cagr_pct: median,
            sector_p25_cagr_pct: p25,
            sector_p75_cagr_pct: p75,
            relative_label: "CAGR_NEGATIVE".into(),
            note: "Subject revenue crosses zero; using linear proxy".into(),
            ..Default::default()
        };
    }
    let gap = subj_cagr - median;
    let label = if gap >= 15.0 { "FAR_ABOVE" }
        else if gap >= 5.0 { "ABOVE" }
        else if gap >= -5.0 { "INLINE" }
        else if gap >= -15.0 { "BELOW" }
        else { "FAR_BELOW" };
    RevenueGrowthRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        latest_revenue: latest,
        earliest_revenue: earliest,
        years_used: years,
        symbol_cagr_pct: subj_cagr,
        peers_considered,
        peers_with_data,
        sector_median_cagr_pct: median,
        sector_p25_cagr_pct: p25,
        sector_p75_cagr_pct: p75,
        gap_to_median_pp: gap,
        relative_label: label.into(),
        note: String::new(),
    }
}

// ── ADR-125 Round 18 compute fns ───────────────────────────────────────────

/// Compute the debt-to-equity ratio for a `LeverageSnapshot`.
/// Returns `None` when equity is non-positive (shell / deficit), which is
/// handled by the LEVRANK surface as a special "NEGATIVE_EQUITY" bucket.
fn debt_to_equity_for(lev: &LeverageSnapshot) -> Option<f64> {
    if lev.total_equity > 0.0 { Some(lev.total_debt / lev.total_equity) } else { None }
}

/// LEVRANK — Leverage Rank vs sector peers.
///
/// Percentile-ranks the subject's D/E (from the cached `LeverageSnapshot`)
/// against peer snapshots pre-filtered to the same sector. Uses the
/// risk-inverted rank ladder (SAFEST_DECILE..RISKIEST_DECILE) because lower
/// D/E = safer. Negative-equity subjects get a dedicated label.
pub fn compute_levrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&LeverageSnapshot>,
    peers: &[&LeverageSnapshot],
) -> LeverageRankSnapshot {
    let subj = match subject {
        Some(s) => s,
        None => {
            return LeverageRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No LEV snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let subj_d2e = match debt_to_equity_for(subj) {
        Some(v) => v,
        None => {
            return LeverageRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                total_debt: subj.total_debt,
                total_equity: subj.total_equity,
                rank_label: "NEGATIVE_EQUITY".into(),
                note: "Subject has non-positive equity; D/E undefined".into(),
                ..Default::default()
            };
        }
    };
    let peer_d2es: Vec<f64> = peers
        .iter()
        .filter_map(|p| debt_to_equity_for(p))
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_d2es.len();
    if peer_d2es.len() < 3 {
        return LeverageRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            debt_to_equity: subj_d2e,
            total_debt: subj.total_debt,
            total_equity: subj.total_equity,
            peers_considered,
            peers_with_data,
            rank_label: "NO_DATA".into(),
            note: format!("Only {} LEV peers with positive equity in sector {} (need ≥3)", peer_d2es.len(), sector),
            ..Default::default()
        };
    }
    let mut sorted = peer_d2es.clone();
    sorted.push(subj_d2e);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    // INVERSION: lower D/E = safer = higher rank.
    let pct = percentile_rank_score(subj_d2e, &peer_d2es, false);
    // rank_position counted by how many peers are SAFER (lower D/E).
    let safer = peer_d2es.iter().filter(|&&p| p < subj_d2e).count();
    let label = risk_rank_label_for_percentile(pct);
    LeverageRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        debt_to_equity: subj_d2e,
        total_debt: subj.total_debt,
        total_equity: subj.total_equity,
        peers_considered,
        peers_with_data,
        sector_median_d2e: median,
        sector_p25_d2e: p25,
        sector_p75_d2e: p75,
        percentile_rank: pct,
        rank_position: safer + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// OPERANK — Operating Quality Rank vs sector peers.
///
/// Percentile-ranks `MarginsSnapshot.latest_operating_margin_pct` within
/// the same sector. Higher margin = higher rank. Peers must be pre-filtered.
pub fn compute_operank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&MarginsSnapshot>,
    peers: &[&MarginsSnapshot],
) -> OperatingQualityRankSnapshot {
    let subj = match subject {
        Some(s) if s.periods_used > 0 => s,
        _ => {
            return OperatingQualityRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No MARGINS snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_margins: Vec<f64> = peers
        .iter()
        .filter(|p| p.periods_used > 0)
        .map(|p| p.latest_operating_margin_pct)
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_margins.len();
    if peer_margins.len() < 3 {
        return OperatingQualityRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            operating_margin_pct: subj.latest_operating_margin_pct,
            margin_trend_label: subj.overall_trend_label.clone(),
            peers_considered,
            peers_with_data,
            rank_label: "NO_DATA".into(),
            note: format!("Only {} MARGINS peers in sector {} (need ≥3)", peer_margins.len(), sector),
            ..Default::default()
        };
    }
    let mut sorted = peer_margins.clone();
    sorted.push(subj.latest_operating_margin_pct);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.latest_operating_margin_pct, &peer_margins, true);
    let better = peer_margins.iter().filter(|&&p| p > subj.latest_operating_margin_pct).count();
    let label = rank_label_for_percentile(pct);
    OperatingQualityRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        operating_margin_pct: subj.latest_operating_margin_pct,
        margin_trend_label: subj.overall_trend_label.clone(),
        peers_considered,
        peers_with_data,
        sector_median_margin_pct: median,
        sector_p25_margin_pct: p25,
        sector_p75_margin_pct: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// FQMRANK — Fundamental Quality Meter Rank vs sector peers.
///
/// Percentile-ranks `FundamentalQualityMeterSnapshot.composite_score` within
/// the same sector. Filters out peers with operator_label "NO_DATA".
pub fn compute_fqmrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&FundamentalQualityMeterSnapshot>,
    peers: &[&FundamentalQualityMeterSnapshot],
) -> FqmRankSnapshot {
    let subj = match subject {
        Some(s) if s.operator_label != "NO_DATA" && s.composite_score > 0.0 => s,
        _ => {
            return FqmRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No FQM snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_scores: Vec<f64> = peers
        .iter()
        .filter(|p| p.operator_label != "NO_DATA" && p.composite_score > 0.0)
        .map(|p| p.composite_score)
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_scores.len();
    if peer_scores.len() < 3 {
        return FqmRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            composite_score: subj.composite_score,
            operator_label: subj.operator_label.clone(),
            peers_considered,
            peers_with_data,
            rank_label: "NO_DATA".into(),
            note: format!("Only {} FQM peers in sector {} (need ≥3)", peer_scores.len(), sector),
            ..Default::default()
        };
    }
    let mut sorted = peer_scores.clone();
    sorted.push(subj.composite_score);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.composite_score, &peer_scores, true);
    let better = peer_scores.iter().filter(|&&p| p > subj.composite_score).count();
    let label = rank_label_for_percentile(pct);
    FqmRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        composite_score: subj.composite_score,
        operator_label: subj.operator_label.clone(),
        peers_considered,
        peers_with_data,
        sector_median_score: median,
        sector_p25: p25,
        sector_p75: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// LIQRANK — Liquidity Rank vs sector peers.
///
/// Percentile-ranks `LiquiditySnapshot.avg_daily_dollar_volume` within the
/// same sector. Higher ADV$ = deeper liquidity = higher rank. Filters out
/// peers with INSUFFICIENT_DATA tier.
pub fn compute_liqrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&LiquiditySnapshot>,
    peers: &[&LiquiditySnapshot],
) -> LiquidityRankSnapshot {
    let subj = match subject {
        Some(s) if s.liquidity_tier != "INSUFFICIENT_DATA" && s.avg_daily_dollar_volume > 0.0 => s,
        _ => {
            return LiquidityRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No LIQ snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_advs: Vec<f64> = peers
        .iter()
        .filter(|p| p.liquidity_tier != "INSUFFICIENT_DATA" && p.avg_daily_dollar_volume > 0.0)
        .map(|p| p.avg_daily_dollar_volume)
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_advs.len();
    if peer_advs.len() < 3 {
        return LiquidityRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            avg_daily_dollar_volume: subj.avg_daily_dollar_volume,
            tier_label: subj.liquidity_tier.clone(),
            peers_considered,
            peers_with_data,
            rank_label: "NO_DATA".into(),
            note: format!("Only {} LIQ peers in sector {} (need ≥3)", peer_advs.len(), sector),
            ..Default::default()
        };
    }
    let mut sorted = peer_advs.clone();
    sorted.push(subj.avg_daily_dollar_volume);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.avg_daily_dollar_volume, &peer_advs, true);
    let better = peer_advs.iter().filter(|&&p| p > subj.avg_daily_dollar_volume).count();
    let label = rank_label_for_percentile(pct);
    LiquidityRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        avg_daily_dollar_volume: subj.avg_daily_dollar_volume,
        tier_label: subj.liquidity_tier.clone(),
        peers_considered,
        peers_with_data,
        sector_median_dollar_volume: median,
        sector_p25_dollar_volume: p25,
        sector_p75_dollar_volume: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// SURPSTK — Earnings Surprise Streak snapshot.
///
/// Pure time-series stat over cached `EarningsSurprise` rows. Classifies each
/// row as BEAT / MISS / INLINE using a ±2% band around the estimate, then
/// counts consecutive streaks over the series (sorted newest-first). Emits
/// a streak-strength label from the beat rate + current streak. No sector.
pub fn compute_surpstk_snapshot(
    symbol: &str,
    as_of: &str,
    surprises: &[EarningsSurprise],
) -> EarningsSurpriseStreakSnapshot {
    if surprises.is_empty() {
        return EarningsSurpriseStreakSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            streak_label: "INSUFFICIENT_DATA".into(),
            note: "No EPS rows cached for subject".into(),
            ..Default::default()
        };
    }
    // Sort newest-first by date (lexical works for YYYY-MM-DD).
    let mut rows: Vec<&EarningsSurprise> = surprises.iter().collect();
    rows.sort_by(|a, b| b.date.cmp(&a.date));
    let classify = |s: &EarningsSurprise| -> &'static str {
        if s.surprise_pct >= 2.0 { "BEAT" }
        else if s.surprise_pct <= -2.0 { "MISS" }
        else { "INLINE" }
    };
    let mut beats = 0usize;
    let mut misses = 0usize;
    let mut inlines = 0usize;
    let mut sum_surprise = 0.0f64;
    for r in &rows {
        sum_surprise += r.surprise_pct;
        match classify(r) {
            "BEAT" => beats += 1,
            "MISS" => misses += 1,
            _ => inlines += 1,
        }
    }
    let total = rows.len();
    let beat_rate = beats as f64 / total as f64 * 100.0;
    let avg_surprise = sum_surprise / total as f64;
    // Current streak: starts at rows[0] (newest) and extends while label matches.
    let current_label = classify(rows[0]);
    let mut current_len = 1usize;
    for r in rows.iter().skip(1) {
        if classify(r) == current_label { current_len += 1; } else { break; }
    }
    // Longest streaks scanned across the full series.
    let mut longest_beat = 0usize;
    let mut longest_miss = 0usize;
    let mut run_beat = 0usize;
    let mut run_miss = 0usize;
    for r in &rows {
        match classify(r) {
            "BEAT" => {
                run_beat += 1;
                run_miss = 0;
                if run_beat > longest_beat { longest_beat = run_beat; }
            }
            "MISS" => {
                run_miss += 1;
                run_beat = 0;
                if run_miss > longest_miss { longest_miss = run_miss; }
            }
            _ => {
                run_beat = 0;
                run_miss = 0;
            }
        }
    }
    let streak_label = if total < 4 {
        "INSUFFICIENT_DATA"
    } else if beat_rate >= 75.0 && current_label == "BEAT" && current_len >= 3 {
        "HOT_STREAK"
    } else if beat_rate >= 60.0 {
        "BEAT_TREND"
    } else if beat_rate <= 25.0 && current_label == "MISS" && current_len >= 3 {
        "COLD_STREAK"
    } else if beat_rate <= 40.0 {
        "MISS_TREND"
    } else {
        "MIXED"
    };
    let latest = rows[0];
    EarningsSurpriseStreakSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        total_events: total,
        beats,
        misses,
        inlines,
        beat_rate_pct: beat_rate,
        current_streak_type: current_label.to_string(),
        current_streak_len: current_len,
        longest_beat_streak: longest_beat,
        longest_miss_streak: longest_miss,
        avg_surprise_pct: avg_surprise,
        latest_event_date: latest.date.clone(),
        latest_event_surprise_pct: latest.surprise_pct,
        latest_event_label: classify(latest).to_string(),
        streak_label: streak_label.to_string(),
        note: String::new(),
    }
}

// ── ADR-126 Round 19 compute fns ──────────────────────────────────────────

pub fn compute_dvdrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&DivgSnapshot>,
    peers: &[&DivgSnapshot],
) -> DividendGrowthRankSnapshot {
    let subj = match subject {
        Some(s) if s.trend_label != "NO_HISTORY" && !s.trend_label.is_empty() => s,
        _ => {
            return DividendGrowthRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No DIVG snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_cagr: Vec<f64> = peers
        .iter()
        .filter(|p| !p.symbol.eq_ignore_ascii_case(symbol))
        .filter(|p| p.trend_label != "NO_HISTORY" && !p.trend_label.is_empty())
        .map(|p| p.cagr_3y_pct)
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_cagr.len();
    if peer_cagr.len() < 3 {
        return DividendGrowthRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            cagr_3y_pct: subj.cagr_3y_pct,
            consecutive_growth_years: subj.consecutive_growth_years,
            trend_label: subj.trend_label.clone(),
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!("Only {} DIVG peers with history in sector {} (need ≥3)", peer_cagr.len(), sector),
            ..Default::default()
        };
    }
    let mut sorted = peer_cagr.clone();
    sorted.push(subj.cagr_3y_pct);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.cagr_3y_pct, &peer_cagr, true);
    let better = peer_cagr.iter().filter(|&&p| p > subj.cagr_3y_pct).count();
    let label = rank_label_for_percentile(pct);
    DividendGrowthRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        cagr_3y_pct: subj.cagr_3y_pct,
        consecutive_growth_years: subj.consecutive_growth_years,
        trend_label: subj.trend_label.clone(),
        peers_considered,
        peers_with_data,
        sector_median_cagr_pct: median,
        sector_p25_cagr_pct: p25,
        sector_p75_cagr_pct: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_earmrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&EarmSnapshot>,
    peers: &[&EarmSnapshot],
) -> EarningsMomentumRankSnapshot {
    let subj = match subject {
        Some(s) if s.momentum_label != "INSUFFICIENT_DATA" && !s.momentum_label.is_empty() => s,
        _ => {
            return EarningsMomentumRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No EARM snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_scores: Vec<f64> = peers
        .iter()
        .filter(|p| !p.symbol.eq_ignore_ascii_case(symbol))
        .filter(|p| p.momentum_label != "INSUFFICIENT_DATA" && !p.momentum_label.is_empty())
        .map(|p| p.composite_score)
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_scores.len();
    if peer_scores.len() < 3 {
        return EarningsMomentumRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            composite_score: subj.composite_score,
            momentum_label: subj.momentum_label.clone(),
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!("Only {} EARM peers with data in sector {} (need ≥3)", peer_scores.len(), sector),
            ..Default::default()
        };
    }
    let mut sorted = peer_scores.clone();
    sorted.push(subj.composite_score);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.composite_score, &peer_scores, true);
    let better = peer_scores.iter().filter(|&&p| p > subj.composite_score).count();
    let label = rank_label_for_percentile(pct);
    EarningsMomentumRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        composite_score: subj.composite_score,
        momentum_label: subj.momentum_label.clone(),
        peers_considered,
        peers_with_data,
        sector_median_score: median,
        sector_p25: p25,
        sector_p75: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_updgrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&UpdmSnapshot>,
    peers: &[&UpdmSnapshot],
) -> UpgradeDowngradeRankSnapshot {
    let subj = match subject {
        Some(s) if s.bias_label != "NO_COVERAGE" && !s.bias_label.is_empty() => s,
        _ => {
            return UpgradeDowngradeRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No UPDM snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_nets: Vec<f64> = peers
        .iter()
        .filter(|p| !p.symbol.eq_ignore_ascii_case(symbol))
        .filter(|p| p.bias_label != "NO_COVERAGE" && !p.bias_label.is_empty())
        .map(|p| p.net_90d as f64)
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_nets.len();
    if peer_nets.len() < 3 {
        return UpgradeDowngradeRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            net_90d: subj.net_90d,
            bias_label: subj.bias_label.clone(),
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!("Only {} UPDM peers with coverage in sector {} (need ≥3)", peer_nets.len(), sector),
            ..Default::default()
        };
    }
    let subj_f = subj.net_90d as f64;
    let mut sorted = peer_nets.clone();
    sorted.push(subj_f);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj_f, &peer_nets, true);
    let better = peer_nets.iter().filter(|&&p| p > subj_f).count();
    let label = rank_label_for_percentile(pct);
    UpgradeDowngradeRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        net_90d: subj.net_90d,
        bias_label: subj.bias_label.clone(),
        peers_considered,
        peers_with_data,
        sector_median_net_90d: median,
        sector_p25_net_90d: p25,
        sector_p75_net_90d: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_gy_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> GapYearlySnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 2 {
        return GapYearlySnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            gap_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥2 bars for gap calc".into(),
            ..Default::default()
        };
    }
    // Caller passes newest-first or oldest-first; we want to scan the last
    // 252 sessions worth of "today's open vs yesterday's close" gaps. Sort by
    // date ascending (oldest first) so pairs (i-1, i) go in calendar order.
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let start = if sorted.len() > 253 { sorted.len() - 253 } else { 0 };
    let window = &sorted[start..];
    let bars_used = window.len();
    let mut gaps_total = 0usize;
    let mut gaps_up_2 = 0usize;
    let mut gaps_down_2 = 0usize;
    let mut gaps_up_5 = 0usize;
    let mut gaps_down_5 = 0usize;
    let mut gaps_up_10 = 0usize;
    let mut gaps_down_10 = 0usize;
    let mut sum_abs = 0.0f64;
    let mut largest_up = 0.0f64;
    let mut largest_up_date = String::new();
    let mut largest_down = 0.0f64;
    let mut largest_down_date = String::new();
    for i in 1..window.len() {
        let prev_close = window[i - 1].close;
        let open = window[i].open;
        if prev_close <= 0.0 || open <= 0.0 { continue; }
        let gap_pct = (open - prev_close) / prev_close * 100.0;
        if gap_pct.abs() < 0.01 { continue; } // treat <0.01% as no gap
        gaps_total += 1;
        sum_abs += gap_pct.abs();
        if gap_pct >= 2.0 { gaps_up_2 += 1; }
        if gap_pct <= -2.0 { gaps_down_2 += 1; }
        if gap_pct >= 5.0 { gaps_up_5 += 1; }
        if gap_pct <= -5.0 { gaps_down_5 += 1; }
        if gap_pct >= 10.0 { gaps_up_10 += 1; }
        if gap_pct <= -10.0 { gaps_down_10 += 1; }
        if gap_pct > largest_up {
            largest_up = gap_pct;
            largest_up_date = window[i].date.clone();
        }
        if gap_pct < largest_down {
            largest_down = gap_pct;
            largest_down_date = window[i].date.clone();
        }
    }
    let avg_abs = if gaps_total > 0 { sum_abs / gaps_total as f64 } else { 0.0 };
    // Gap-label ladder:
    // - EXPLOSIVE: any 10% gap OR ≥ 4 gaps at the 5% band
    // - GAPPY: ≥ 12 gaps at the 2% band OR ≥ 2 gaps at the 5% band
    // - SMOOTH: < 6 gaps at the 2% band
    // - NORMAL: anything between
    let gap_2_total = gaps_up_2 + gaps_down_2;
    let gap_5_total = gaps_up_5 + gaps_down_5;
    let gap_10_total = gaps_up_10 + gaps_down_10;
    let gap_label = if bars_used < 20 {
        "INSUFFICIENT_DATA"
    } else if gap_10_total >= 1 || gap_5_total >= 4 {
        "EXPLOSIVE"
    } else if gap_2_total >= 12 || gap_5_total >= 2 {
        "GAPPY"
    } else if gap_2_total < 6 {
        "SMOOTH"
    } else {
        "NORMAL"
    };
    GapYearlySnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        gaps_total,
        gaps_up_2pct: gaps_up_2,
        gaps_down_2pct: gaps_down_2,
        gaps_up_5pct: gaps_up_5,
        gaps_down_5pct: gaps_down_5,
        gaps_up_10pct: gaps_up_10,
        gaps_down_10pct: gaps_down_10,
        largest_up_gap_pct: largest_up,
        largest_up_gap_date: largest_up_date,
        largest_down_gap_pct: largest_down,
        largest_down_gap_date: largest_down_date,
        avg_abs_gap_pct: avg_abs,
        gap_label: gap_label.to_string(),
        note: String::new(),
    }
}

pub fn compute_des_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DailyEventStreakSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 2 {
        return DailyEventStreakSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            streak_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥2 bars for streak calc".into(),
            ..Default::default()
        };
    }
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let start = if sorted.len() > 253 { sorted.len() - 253 } else { 0 };
    let window = &sorted[start..];
    let bars_used = window.len();
    let mut up_days = 0usize;
    let mut down_days = 0usize;
    let mut flat_days = 0usize;
    let mut sum_up = 0.0f64;
    let mut sum_down = 0.0f64;
    let mut dirs: Vec<i8> = Vec::with_capacity(window.len());
    for i in 1..window.len() {
        let prev = window[i - 1].close;
        let cur = window[i].close;
        if prev <= 0.0 || cur <= 0.0 { dirs.push(0); continue; }
        let pct = (cur - prev) / prev * 100.0;
        if pct > 0.0 {
            up_days += 1;
            sum_up += pct;
            dirs.push(1);
        } else if pct < 0.0 {
            down_days += 1;
            sum_down += pct;
            dirs.push(-1);
        } else {
            flat_days += 1;
            dirs.push(0);
        }
    }
    let mut longest_up = 0usize;
    let mut longest_down = 0usize;
    let mut run_up = 0usize;
    let mut run_down = 0usize;
    for d in &dirs {
        match *d {
            1 => {
                run_up += 1;
                run_down = 0;
                if run_up > longest_up { longest_up = run_up; }
            }
            -1 => {
                run_down += 1;
                run_up = 0;
                if run_down > longest_down { longest_down = run_down; }
            }
            _ => {
                run_up = 0;
                run_down = 0;
            }
        }
    }
    // Current streak: trailing run at the end of `dirs`.
    let (current_type, current_len) = if let Some(last) = dirs.last().copied() {
        let mut len = 0usize;
        if last != 0 {
            for d in dirs.iter().rev() {
                if *d == last { len += 1; } else { break; }
            }
        }
        match last {
            1 => ("UP".to_string(), len),
            -1 => ("DOWN".to_string(), len),
            0 => ("FLAT".to_string(), 0usize),
            _ => ("NONE".to_string(), 0usize),
        }
    } else {
        ("NONE".to_string(), 0usize)
    };
    let total_directional = up_days + down_days;
    let up_day_rate = if total_directional > 0 {
        up_days as f64 / total_directional as f64 * 100.0
    } else { 0.0 };
    let avg_up = if up_days > 0 { sum_up / up_days as f64 } else { 0.0 };
    let avg_down = if down_days > 0 { sum_down / down_days as f64 } else { 0.0 };
    let streak_label = if bars_used < 20 {
        "INSUFFICIENT_DATA"
    } else if up_day_rate >= 60.0 && longest_up >= 5 {
        "STRONG_UPTREND"
    } else if up_day_rate >= 55.0 {
        "UPTREND_BIAS"
    } else if up_day_rate <= 40.0 && longest_down >= 5 {
        "STRONG_DOWNTREND"
    } else if up_day_rate <= 45.0 {
        "DOWNTREND_BIAS"
    } else {
        "NEUTRAL"
    };
    DailyEventStreakSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        current_streak_type: current_type,
        current_streak_len: current_len,
        longest_up_streak: longest_up,
        longest_down_streak: longest_down,
        up_days,
        down_days,
        flat_days,
        up_day_rate_pct: up_day_rate,
        avg_up_move_pct: avg_up,
        avg_down_move_pct: avg_down,
        streak_label: streak_label.to_string(),
        note: String::new(),
    }
}

// ── ADR-127 Round 20 compute fns ──────────────────────────────────────────

/// DVDYIELDRANK compute: sector percentile rank of the subject's dividend
/// yield. Non-payers (None or 0.0) are filtered so the cohort is
/// dividend-paying names only. Needs ≥3 peers with yield data.
pub fn compute_dvdyieldrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject_yield_pct: Option<f64>,
    peers: &[(String, Option<f64>)],
) -> DividendYieldRankSnapshot {
    let sym = symbol.to_uppercase();
    let subj = match subject_yield_pct {
        Some(y) if y > 0.0 && y.is_finite() => y,
        _ => {
            return DividendYieldRankSnapshot {
                symbol: sym,
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "subject has no dividend yield (non-payer or missing data)".into(),
                ..Default::default()
            };
        }
    };
    let peer_y: Vec<f64> = peers.iter()
        .filter(|(s, _)| !s.eq_ignore_ascii_case(symbol))
        .filter_map(|(_, y)| y.filter(|v| *v > 0.0 && v.is_finite()))
        .collect();
    let peers_considered = peers.iter().filter(|(s, _)| !s.eq_ignore_ascii_case(symbol)).count();
    let peers_with_data = peer_y.len();
    if peers_with_data < 3 {
        return DividendYieldRankSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            dividend_yield_pct: subj,
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 dividend-paying sector peers, got {}", peers_with_data),
            ..Default::default()
        };
    }
    let mut sorted = peer_y.clone();
    sorted.push(subj);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj, &peer_y, true);
    let better = peer_y.iter().filter(|&&p| p > subj).count();
    let label = rank_label_for_percentile(pct);
    DividendYieldRankSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        dividend_yield_pct: subj,
        peers_considered,
        peers_with_data,
        sector_median_yield_pct: median,
        sector_p25_yield_pct: p25,
        sector_p75_yield_pct: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// SHRANK compute: sector percentile rank of short_percent_of_float,
/// risk-inverted so a *lower* short interest earns a *higher* (safer) rank.
pub fn compute_shrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject_short_pct: Option<f64>,
    peers: &[(String, Option<f64>)],
) -> ShortInterestRankSnapshot {
    let sym = symbol.to_uppercase();
    let subj = match subject_short_pct {
        Some(s) if s.is_finite() && s >= 0.0 => s,
        _ => {
            return ShortInterestRankSnapshot {
                symbol: sym,
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "subject missing short_percent_of_float".into(),
                ..Default::default()
            };
        }
    };
    let peer_s: Vec<f64> = peers.iter()
        .filter(|(s, _)| !s.eq_ignore_ascii_case(symbol))
        .filter_map(|(_, v)| v.filter(|x| x.is_finite() && *x >= 0.0))
        .collect();
    let peers_considered = peers.iter().filter(|(s, _)| !s.eq_ignore_ascii_case(symbol)).count();
    let peers_with_data = peer_s.len();
    if peers_with_data < 3 {
        return ShortInterestRankSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            short_pct_of_float: subj,
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 sector peers with short data, got {}", peers_with_data),
            ..Default::default()
        };
    }
    let mut sorted = peer_s.clone();
    sorted.push(subj);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    // Risk-inverted: lower short = safer = higher percentile
    let pct = percentile_rank_score(subj, &peer_s, false);
    // For risk surfaces, rank_position counts peers who are safer (lower short)
    let safer = peer_s.iter().filter(|&&p| p < subj).count();
    let label = risk_rank_label_for_percentile(pct);
    ShortInterestRankSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        short_pct_of_float: subj,
        peers_considered,
        peers_with_data,
        sector_median_short_pct: median,
        sector_p25_short_pct: p25,
        sector_p75_short_pct: p75,
        percentile_rank: pct,
        rank_position: safer + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// ATRANN compute: pure symbol-local 14-period Wilder ATR annualized, with
/// volatility regime label. Uses the most recent 253 sessions from the HP
/// cache, sorted oldest-first.
pub fn compute_atrann_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> AnnualizedAtrSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 15 {
        return AnnualizedAtrSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            regime_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥15 bars for 14-period ATR warmup".into(),
            ..Default::default()
        };
    }
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let start = if sorted.len() > 253 { sorted.len() - 253 } else { 0 };
    let window = &sorted[start..];
    let bars_used = window.len();
    // True range for each bar i (i>=1): max(high-low, |high-prev_close|, |low-prev_close|)
    let mut trs: Vec<f64> = Vec::with_capacity(window.len());
    for i in 1..window.len() {
        let h = window[i].high;
        let l = window[i].low;
        let pc = window[i - 1].close;
        if h <= 0.0 || l <= 0.0 || pc <= 0.0 { continue; }
        let tr = (h - l).max((h - pc).abs()).max((l - pc).abs());
        trs.push(tr);
    }
    if trs.len() < 14 {
        return AnnualizedAtrSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used,
            regime_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} usable TR bars after filtering", trs.len()),
            ..Default::default()
        };
    }
    // Wilder smoothing: seed = mean of first 14, then ATR_i = (prev_ATR × 13 + TR_i) / 14
    let seed: f64 = trs[..14].iter().sum::<f64>() / 14.0;
    let mut atr = seed;
    for &tr in &trs[14..] {
        atr = (atr * 13.0 + tr) / 14.0;
    }
    let latest_close = window.last().map(|r| r.close).unwrap_or(0.0);
    let atr_pct = if latest_close > 0.0 { atr / latest_close * 100.0 } else { 0.0 };
    let atr_ann = atr_pct * (252.0f64).sqrt();
    let regime = if atr_ann < 15.0 {
        "LOW_VOL"
    } else if atr_ann < 30.0 {
        "NORMAL_VOL"
    } else if atr_ann < 60.0 {
        "HIGH_VOL"
    } else {
        "EXTREME_VOL"
    };
    AnnualizedAtrSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        latest_close,
        atr14: atr,
        atr14_pct: atr_pct,
        atr_annualized_pct: atr_ann,
        regime_label: regime.into(),
        note: String::new(),
    }
}

/// DDHIST compute: pure symbol-local drawdown history stat. Scans the
/// window for the deepest peak-to-trough, the longest peak-to-recovery
/// duration, the count of 5% and 10% corrections, and the current drawdown.
pub fn compute_ddhist_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DrawdownHistorySnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 20 {
        return DrawdownHistorySnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            regime_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥20 bars".into(),
            ..Default::default()
        };
    }
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let start = if sorted.len() > 253 { sorted.len() - 253 } else { 0 };
    let window = &sorted[start..];
    let bars_used = window.len();
    let mut running_peak = window[0].close;
    let mut running_peak_idx = 0usize;
    let mut running_peak_date = window[0].date.clone();
    let mut max_dd_pct = 0.0f64;
    let mut max_dd_peak_date = String::new();
    let mut max_dd_trough_date = String::new();
    let mut longest_dd_days = 0usize;
    let mut current_dd_start: Option<(usize, String)> = None; // (idx, date) of current peak
    let mut corrections_5 = 0usize;
    let mut corrections_10 = 0usize;
    // Track per-correction state: a "correction" is a run from a local peak to a local trough with ≥5% or ≥10% decline.
    let mut in_correction = false;
    let mut correction_peak = window[0].close;
    for (i, bar) in window.iter().enumerate() {
        let c = bar.close;
        if c <= 0.0 { continue; }
        if c >= running_peak {
            // Recovered — close the current drawdown bucket.
            if let Some((peak_idx, _)) = &current_dd_start {
                let duration = i - peak_idx;
                if duration > longest_dd_days { longest_dd_days = duration; }
            }
            current_dd_start = None;
            running_peak = c;
            running_peak_idx = i;
            running_peak_date = bar.date.clone();
            // Close any open correction by measuring its depth.
            if in_correction {
                let depth = (c - correction_peak) / correction_peak * 100.0;
                let abs_depth = -depth;  // positive number
                if abs_depth >= 10.0 { corrections_10 += 1; }
                if abs_depth >= 5.0 { corrections_5 += 1; }
                in_correction = false;
            }
            correction_peak = c;
        } else {
            // In a drawdown.
            if current_dd_start.is_none() {
                current_dd_start = Some((running_peak_idx, running_peak_date.clone()));
            }
            let dd = (c - running_peak) / running_peak * 100.0;  // negative
            if dd < max_dd_pct {
                max_dd_pct = dd;
                max_dd_peak_date = running_peak_date.clone();
                max_dd_trough_date = bar.date.clone();
            }
            // Correction tracking: local-peak-to-trough.
            if c < correction_peak {
                in_correction = true;
            }
        }
    }
    // If we ended the window still in a drawdown, count its duration.
    if let Some((peak_idx, _)) = &current_dd_start {
        let duration = window.len().saturating_sub(*peak_idx);
        if duration > longest_dd_days { longest_dd_days = duration; }
    }
    // Close any still-open correction (open means we ended the window below the local peak).
    if in_correction {
        let last = window.last().map(|r| r.close).unwrap_or(correction_peak);
        let abs_depth = (correction_peak - last) / correction_peak * 100.0;
        if abs_depth >= 10.0 { corrections_10 += 1; }
        if abs_depth >= 5.0 { corrections_5 += 1; }
    }
    let latest = window.last().map(|r| r.close).unwrap_or(0.0);
    let current_dd = if latest > 0.0 && running_peak > 0.0 {
        (latest - running_peak) / running_peak * 100.0
    } else { 0.0 };
    let regime = if current_dd > -1.0 {
        "RECOVERING"
    } else if max_dd_pct > -10.0 {
        "SHALLOW"
    } else if max_dd_pct > -20.0 {
        "MEANINGFUL"
    } else if max_dd_pct > -35.0 {
        "SEVERE"
    } else {
        "CATASTROPHIC"
    };
    DrawdownHistorySnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        max_drawdown_pct: max_dd_pct,
        max_drawdown_peak_date: max_dd_peak_date,
        max_drawdown_trough_date: max_dd_trough_date,
        longest_drawdown_days: longest_dd_days,
        corrections_5pct: corrections_5,
        corrections_10pct: corrections_10,
        current_drawdown_pct: current_dd,
        regime_label: regime.into(),
        note: String::new(),
    }
}

/// PRICEPERF compute: multi-horizon total return stat. Computes returns
/// over trailing 21, 63, 126, and 253 sessions plus YTD from the first
/// session of as_of's calendar year.
pub fn compute_priceperf_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> PricePerformanceSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 2 {
        return PricePerformanceSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            trend_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥2 bars".into(),
            ..Default::default()
        };
    }
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let bars_used = sorted.len();
    let latest = sorted.last().unwrap();
    let latest_close = latest.close;
    if latest_close <= 0.0 {
        return PricePerformanceSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used,
            trend_label: "INSUFFICIENT_DATA".into(),
            note: "latest close not positive".into(),
            ..Default::default()
        };
    }
    let ret_at = |offset: usize| -> f64 {
        if sorted.len() > offset {
            let past = sorted[sorted.len() - 1 - offset].close;
            if past > 0.0 { (latest_close - past) / past * 100.0 } else { 0.0 }
        } else { 0.0 }
    };
    let ret_1m = ret_at(21);
    let ret_3m = ret_at(63);
    let ret_6m = ret_at(126);
    let ret_1y = ret_at(253);
    // YTD: find first bar with date.year == latest.date.year
    let year_prefix = latest.date.get(..4).unwrap_or("");
    let ytd_ret = if !year_prefix.is_empty() {
        let ytd_start = sorted.iter().find(|r| r.date.starts_with(year_prefix));
        match ytd_start {
            Some(start_bar) if start_bar.close > 0.0 => {
                (latest_close - start_bar.close) / start_bar.close * 100.0
            }
            _ => 0.0,
        }
    } else { 0.0 };
    let trend = if bars_used < 20 {
        "INSUFFICIENT_DATA"
    } else if ret_1y > 30.0 && ret_3m > 10.0 {
        "STRONG_BULL"
    } else if ret_1y > 10.0 || ret_3m > 5.0 {
        "BULL"
    } else if ret_1y < -30.0 && ret_3m < -10.0 {
        "STRONG_BEAR"
    } else if ret_1y < -10.0 || ret_3m < -5.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    PricePerformanceSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        latest_close,
        ret_1m_pct: ret_1m,
        ret_3m_pct: ret_3m,
        ret_6m_pct: ret_6m,
        ret_ytd_pct: ytd_ret,
        ret_1y_pct: ret_1y,
        trend_label: trend.into(),
        note: String::new(),
    }
}

// ── ADR-128 Round 21 compute fns ──

/// BETARANK compute: sector percentile rank of Fundamentals.beta,
/// risk-inverted so a *lower* beta earns a *higher* (safer) rank.
pub fn compute_betarank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject_beta: Option<f64>,
    peers: &[(String, Option<f64>)],
) -> BetaRankSnapshot {
    let sym = symbol.to_uppercase();
    let subj = match subject_beta {
        Some(b) if b.is_finite() => b,
        _ => {
            return BetaRankSnapshot {
                symbol: sym,
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "subject missing beta".into(),
                ..Default::default()
            };
        }
    };
    let peer_b: Vec<f64> = peers.iter()
        .filter(|(s, _)| !s.eq_ignore_ascii_case(symbol))
        .filter_map(|(_, v)| v.filter(|x| x.is_finite()))
        .collect();
    let peers_considered = peers.iter().filter(|(s, _)| !s.eq_ignore_ascii_case(symbol)).count();
    let peers_with_data = peer_b.len();
    if peers_with_data < 3 {
        return BetaRankSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            subject_beta: Some(subj),
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 sector peers with beta, got {}", peers_with_data),
            ..Default::default()
        };
    }
    let mut sorted = peer_b.clone();
    sorted.push(subj);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    // Risk-inverted: lower beta = safer = higher percentile
    let pct = percentile_rank_score(subj, &peer_b, false);
    let safer = peer_b.iter().filter(|&&p| p < subj).count();
    let label = risk_rank_label_for_percentile(pct);
    BetaRankSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        subject_beta: Some(subj),
        peers_considered,
        peers_with_data,
        sector_median_beta: median,
        sector_p25_beta: p25,
        sector_p75_beta: p75,
        percentile_rank: pct,
        rank_position: safer + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// PEGRANK compute: sector percentile rank of Fundamentals.peg_ratio,
/// value-inverted so a *lower* PEG (cheaper growth) earns a *higher* rank.
/// Filters out non-positive or non-finite PEG on both subject and peer sides.
pub fn compute_pegrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject_peg: Option<f64>,
    peers: &[(String, Option<f64>)],
) -> PegRankSnapshot {
    let sym = symbol.to_uppercase();
    let subj = match subject_peg {
        Some(p) if p > 0.0 && p.is_finite() => p,
        _ => {
            return PegRankSnapshot {
                symbol: sym,
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "subject has no valid PEG (negative or missing)".into(),
                ..Default::default()
            };
        }
    };
    let peer_p: Vec<f64> = peers.iter()
        .filter(|(s, _)| !s.eq_ignore_ascii_case(symbol))
        .filter_map(|(_, v)| v.filter(|x| *x > 0.0 && x.is_finite()))
        .collect();
    let peers_considered = peers.iter().filter(|(s, _)| !s.eq_ignore_ascii_case(symbol)).count();
    let peers_with_data = peer_p.len();
    if peers_with_data < 3 {
        return PegRankSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            subject_peg: Some(subj),
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 sector peers with positive PEG, got {}", peers_with_data),
            ..Default::default()
        };
    }
    let mut sorted = peer_p.clone();
    sorted.push(subj);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    // Value-inverted: lower PEG = better value = higher percentile
    let pct = percentile_rank_score(subj, &peer_p, false);
    let better = peer_p.iter().filter(|&&p| p < subj).count();
    let label = rank_label_for_percentile(pct);
    PegRankSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        subject_peg: Some(subj),
        peers_considered,
        peers_with_data,
        sector_median_peg: median,
        sector_p25_peg: p25,
        sector_p75_peg: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// FHIGHLOW compute: 52-week high/low distance stat over cached HP bars.
/// Takes the trailing 253 sessions, tracks max close + min close + dates,
/// computes distance-from-high/low and range position, and emits a
/// proximity label band.
pub fn compute_fhighlow_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> FiftyTwoWeekHighLowSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 2 {
        return FiftyTwoWeekHighLowSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            proximity_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥2 bars".into(),
            ..Default::default()
        };
    }
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    // Trailing 253 sessions only.
    let window: Vec<&&HistoricalPriceRow> = sorted.iter().rev().take(253).collect();
    let bars_used = window.len();
    let latest = *window[0];
    let latest_close = latest.close;
    if latest_close <= 0.0 {
        return FiftyTwoWeekHighLowSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used,
            proximity_label: "INSUFFICIENT_DATA".into(),
            note: "latest close not positive".into(),
            ..Default::default()
        };
    }
    let mut high = latest_close;
    let mut high_date = latest.date.clone();
    let mut high_idx: usize = 0; // index from latest (0 = most recent)
    let mut low = latest_close;
    let mut low_date = latest.date.clone();
    let mut low_idx: usize = 0;
    for (i, row) in window.iter().enumerate() {
        if row.close > 0.0 {
            if row.close > high {
                high = row.close;
                high_date = row.date.clone();
                high_idx = i;
            }
            if row.close < low {
                low = row.close;
                low_date = row.date.clone();
                low_idx = i;
            }
        }
    }
    let pct_from_high = if high > 0.0 { (latest_close - high) / high * 100.0 } else { 0.0 };
    let pct_from_low = if low > 0.0 { (latest_close - low) / low * 100.0 } else { 0.0 };
    let range = high - low;
    let range_position = if range > 0.0 { (latest_close - low) / range * 100.0 } else { 50.0 };
    let proximity = if bars_used < 20 {
        "INSUFFICIENT_DATA"
    } else if range_position >= 98.0 {
        "AT_HIGH"
    } else if range_position >= 80.0 {
        "NEAR_HIGH"
    } else if range_position >= 20.0 {
        "MID_RANGE"
    } else if range_position >= 2.0 {
        "NEAR_LOW"
    } else {
        "AT_LOW"
    };
    FiftyTwoWeekHighLowSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        latest_close,
        high_52w: high,
        high_52w_date: high_date,
        days_since_high: high_idx,
        low_52w: low,
        low_52w_date: low_date,
        days_since_low: low_idx,
        pct_from_high,
        pct_from_low,
        range_position_pct: range_position,
        proximity_label: proximity.into(),
        note: String::new(),
    }
}

/// RVCONE compute: multi-horizon annualized realized volatility over the
/// HP cache, plus a rolling 20d RV percentile cone label.
pub fn compute_rvcone_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> RealizedVolConeSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 21 {
        return RealizedVolConeSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            cone_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥21 bars for 20-session realized vol".into(),
            ..Default::default()
        };
    }
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let bars_used = sorted.len();
    let latest_close = sorted.last().unwrap().close;
    if latest_close <= 0.0 {
        return RealizedVolConeSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used,
            cone_label: "INSUFFICIENT_DATA".into(),
            note: "latest close not positive".into(),
            ..Default::default()
        };
    }
    // Log returns.
    let mut log_rets: Vec<f64> = Vec::with_capacity(sorted.len());
    for w in sorted.windows(2) {
        let prev = w[0].close;
        let curr = w[1].close;
        if prev > 0.0 && curr > 0.0 {
            log_rets.push((curr / prev).ln());
        }
    }
    if log_rets.len() < 20 {
        return RealizedVolConeSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used,
            cone_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    // Annualized realized vol of trailing n returns, as percent.
    let ann_rv = |n: usize| -> f64 {
        if log_rets.len() < n { return 0.0; }
        let slice = &log_rets[log_rets.len() - n..];
        let mean: f64 = slice.iter().sum::<f64>() / n as f64;
        let var: f64 = slice.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n as f64;
        var.sqrt() * (252.0_f64).sqrt() * 100.0
    };
    let rv20 = ann_rv(20);
    let rv60 = ann_rv(60);
    let rv120 = ann_rv(120);
    let rv252 = ann_rv(252);
    // Rolling 20d RV distribution across the full return window.
    let mut rolling20: Vec<f64> = Vec::new();
    if log_rets.len() >= 20 {
        for end in 20..=log_rets.len() {
            let slice = &log_rets[end - 20..end];
            let mean: f64 = slice.iter().sum::<f64>() / 20.0;
            let var: f64 = slice.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / 20.0;
            rolling20.push(var.sqrt() * (252.0_f64).sqrt() * 100.0);
        }
    }
    let (rv20_min, rv20_med, rv20_max, rv20_pct) = if !rolling20.is_empty() {
        let mut sorted_r = rolling20.clone();
        sorted_r.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let min = *sorted_r.first().unwrap();
        let max = *sorted_r.last().unwrap();
        let med = quantile_f64(&sorted_r, 0.5);
        // Percentile of latest rv20 within the historical rolling distribution.
        let others: Vec<f64> = rolling20.iter().take(rolling20.len() - 1).copied().collect();
        let pct = if others.is_empty() { 50.0 } else { percentile_rank_score(rv20, &others, true) };
        (min, med, max, pct)
    } else { (rv20, rv20, rv20, 50.0) };
    let cone = if rolling20.len() < 20 {
        "INSUFFICIENT_DATA"
    } else if rv20_pct >= 90.0 {
        "EXTREME"
    } else if rv20_pct >= 70.0 {
        "ELEVATED"
    } else if rv20_pct >= 30.0 {
        "TYPICAL"
    } else if rv20_pct >= 10.0 {
        "BELOW_AVG"
    } else {
        "COMPRESSED"
    };
    RealizedVolConeSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        latest_close,
        rv20_pct: rv20,
        rv60_pct: rv60,
        rv120_pct: rv120,
        rv252_pct: rv252,
        rv20_min_pct: rv20_min,
        rv20_median_pct: rv20_med,
        rv20_max_pct: rv20_max,
        rv20_percentile: rv20_pct,
        cone_label: cone.into(),
        note: String::new(),
    }
}

/// CALPB compute: calendar-aligned period breakdowns over the HP cache.
/// Uses year-prefix / month-prefix string matching on `date` (assumes
/// ISO-8601 YYYY-MM-DD), like PRICEPERF's YTD shortcut.
pub fn compute_calpb_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CalendarPeriodBreakdownSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 2 {
        return CalendarPeriodBreakdownSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            momentum_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥2 bars".into(),
            ..Default::default()
        };
    }
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let bars_used = sorted.len();
    let latest = sorted.last().unwrap();
    let latest_close = latest.close;
    if latest_close <= 0.0 {
        return CalendarPeriodBreakdownSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used,
            momentum_label: "INSUFFICIENT_DATA".into(),
            note: "latest close not positive".into(),
            ..Default::default()
        };
    }
    // Parse latest date as YYYY-MM-DD.
    let year: i32 = latest.date.get(..4).and_then(|s| s.parse().ok()).unwrap_or(0);
    let month: u32 = latest.date.get(5..7).and_then(|s| s.parse().ok()).unwrap_or(0);
    if year == 0 || month == 0 {
        return CalendarPeriodBreakdownSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used,
            momentum_label: "INSUFFICIENT_DATA".into(),
            note: "cannot parse latest bar date".into(),
            ..Default::default()
        };
    }
    let quarter = ((month - 1) / 3) + 1; // 1..=4
    let q_first_month = ((quarter - 1) * 3) + 1;
    // Helpers.
    let pct_from_first_in = |prefix: &str| -> f64 {
        if let Some(start) = sorted.iter().find(|r| r.date.starts_with(prefix)) {
            if start.close > 0.0 {
                return (latest_close - start.close) / start.close * 100.0;
            }
        }
        0.0
    };
    let full_period_return = |start_prefix: &str, end_prefix: &str| -> f64 {
        let first = sorted.iter().find(|r| r.date.starts_with(start_prefix));
        let last = sorted.iter().rev().find(|r| r.date.starts_with(end_prefix));
        match (first, last) {
            (Some(a), Some(b)) if a.close > 0.0 && b.close > 0.0 => (b.close - a.close) / a.close * 100.0,
            _ => 0.0,
        }
    };
    // MTD — bars with year-month prefix matching latest.
    let ym_prefix = format!("{:04}-{:02}", year, month);
    let mtd = pct_from_first_in(&ym_prefix);
    // QTD — bars from q_first_month of current year onwards.
    // Use inclusion filter across the 3 month-prefixes in current quarter.
    let qtd = {
        let q_prefixes: Vec<String> = (0..3)
            .map(|i| format!("{:04}-{:02}", year, q_first_month + i))
            .collect();
        let first = sorted.iter().find(|r| q_prefixes.iter().any(|p| r.date.starts_with(p)));
        match first {
            Some(bar) if bar.close > 0.0 => (latest_close - bar.close) / bar.close * 100.0,
            _ => 0.0,
        }
    };
    // YTD — first bar of current year to latest.
    let y_prefix = format!("{:04}", year);
    let ytd = pct_from_first_in(&y_prefix);
    // Prior quarter — full return over the quarter before the current one.
    let (prior_q_year, prior_q) = if quarter == 1 { (year - 1, 4u32) } else { (year, quarter - 1) };
    let prior_q_first_month = ((prior_q - 1) * 3) + 1;
    let prior_q_prefixes: Vec<String> = (0..3)
        .map(|i| format!("{:04}-{:02}", prior_q_year, prior_q_first_month + i))
        .collect();
    let prior_quarter = {
        let first = sorted.iter().find(|r| prior_q_prefixes.iter().any(|p| r.date.starts_with(p)));
        let last = sorted.iter().rev().find(|r| prior_q_prefixes.iter().any(|p| r.date.starts_with(p)));
        match (first, last) {
            (Some(a), Some(b)) if a.close > 0.0 && b.close > 0.0 => (b.close - a.close) / a.close * 100.0,
            _ => 0.0,
        }
    };
    // Prior year — full-year return of year-1.
    let prior_year_str = format!("{:04}", year - 1);
    let prior_year = full_period_return(&prior_year_str, &prior_year_str);
    // Momentum label: compare QTD vs prior_quarter.
    let momentum = if bars_used < 20 {
        "INSUFFICIENT_DATA"
    } else if qtd > prior_quarter + 5.0 && qtd > 0.0 {
        "ACCELERATING"
    } else if (qtd - prior_quarter).abs() <= 5.0 {
        "STEADY"
    } else if qtd < prior_quarter - 5.0 && qtd < 0.0 && prior_quarter < 0.0 {
        "DECELERATING"
    } else if qtd.signum() != prior_quarter.signum() && prior_quarter != 0.0 {
        "REVERSING"
    } else {
        "DECELERATING"
    };
    CalendarPeriodBreakdownSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        latest_close,
        mtd_pct: mtd,
        qtd_pct: qtd,
        ytd_pct: ytd,
        prior_quarter_pct: prior_quarter,
        prior_year_pct: prior_year,
        current_year: format!("{:04}", year),
        current_quarter: format!("Q{}", quarter),
        momentum_label: momentum.into(),
        note: String::new(),
    }
}

// ── ADR-129 Round 22 compute fns ──

/// Shared helper: collect trailing 253 bars sorted oldest-first and
/// compute log returns. Returns (sorted_bars, log_returns).
pub(crate) fn trailing_log_returns(bars: &[HistoricalPriceRow]) -> (Vec<&HistoricalPriceRow>, Vec<f64>) {
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let window: Vec<&HistoricalPriceRow> = sorted.iter().rev().take(253).rev().copied().collect();
    let mut log_rets: Vec<f64> = Vec::with_capacity(window.len());
    for w in window.windows(2) {
        let prev = w[0].close;
        let curr = w[1].close;
        if prev > 0.0 && curr > 0.0 {
            log_rets.push((curr / prev).ln());
        }
    }
    (window, log_rets)
}

/// RETSKEW compute: skewness of daily log returns over the trailing 253
/// sessions. Uses Fisher-Pearson (sample) skew with N denominator to match
/// RVCONE's stdev convention.
pub fn compute_retskew_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ReturnSkewnessSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 20 {
        return ReturnSkewnessSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            skew_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len() as f64;
    let mean: f64 = log_rets.iter().sum::<f64>() / n;
    let var: f64 = log_rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;
    let stdev = var.sqrt();
    let skew = if stdev > 0.0 {
        let m3: f64 = log_rets.iter().map(|r| (r - mean).powi(3)).sum::<f64>() / n;
        m3 / stdev.powi(3)
    } else {
        0.0
    };
    let positive = log_rets.iter().filter(|&&r| r > 0.0).count() as f64;
    let positive_pct = (positive / n) * 100.0;
    let largest_up = log_rets.iter().cloned().fold(f64::NEG_INFINITY, f64::max) * 100.0;
    let largest_down = log_rets.iter().cloned().fold(f64::INFINITY, f64::min) * 100.0;
    let skew_label = if skew <= -1.0 {
        "STRONG_LEFT"
    } else if skew <= -0.3 {
        "LEFT"
    } else if skew < 0.3 {
        "SYMMETRIC"
    } else if skew < 1.0 {
        "RIGHT"
    } else {
        "STRONG_RIGHT"
    };
    ReturnSkewnessSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        mean_log_return: mean,
        stdev_log_return: stdev,
        skewness: skew,
        positive_return_pct: positive_pct,
        largest_up_pct: largest_up,
        largest_down_pct: largest_down,
        skew_label: skew_label.into(),
        note: String::new(),
    }
}

/// RETKURT compute: excess kurtosis of daily log returns over trailing 253
/// sessions. Counts 2-sigma and 3-sigma outliers for a non-parametric fat-
/// tail check alongside the moment-based number.
pub fn compute_retkurt_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ReturnKurtosisSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 20 {
        return ReturnKurtosisSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            kurt_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len() as f64;
    let mean: f64 = log_rets.iter().sum::<f64>() / n;
    let var: f64 = log_rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;
    let stdev = var.sqrt();
    let excess = if stdev > 0.0 {
        let m4: f64 = log_rets.iter().map(|r| (r - mean).powi(4)).sum::<f64>() / n;
        (m4 / stdev.powi(4)) - 3.0
    } else {
        0.0
    };
    let (out2, out3) = if stdev > 0.0 {
        let mut c2 = 0usize;
        let mut c3 = 0usize;
        for r in &log_rets {
            let z = (r - mean).abs() / stdev;
            if z > 2.0 { c2 += 1; }
            if z > 3.0 { c3 += 1; }
        }
        (c2, c3)
    } else {
        (0, 0)
    };
    let out2_pct = (out2 as f64 / n) * 100.0;
    let kurt_label = if excess <= -0.5 {
        "PLATYKURTIC"
    } else if excess < 1.0 {
        "NORMAL"
    } else if excess < 3.0 {
        "MILD_FAT"
    } else if excess < 6.0 {
        "FAT"
    } else {
        "EXTREME_FAT"
    };
    ReturnKurtosisSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        mean_log_return: mean,
        stdev_log_return: stdev,
        excess_kurtosis: excess,
        outlier_2sigma_count: out2,
        outlier_3sigma_count: out3,
        outlier_2sigma_pct: out2_pct,
        kurt_label: kurt_label.into(),
        note: String::new(),
    }
}

/// TAILR compute: 95/5 and 99/1 tail ratios over trailing 253 sessions.
/// Non-parametric counterpart to RETSKEW.
pub fn compute_tailr_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> TailRatioSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 20 {
        return TailRatioSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            bias_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    let pct_returns: Vec<f64> = log_rets.iter().map(|r| r * 100.0).collect();
    let mut sorted = pct_returns.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p95 = quantile_f64(&sorted, 0.95);
    let p05 = quantile_f64(&sorted, 0.05);
    let p99 = quantile_f64(&sorted, 0.99);
    let p01 = quantile_f64(&sorted, 0.01);
    let tail_ratio = if p05.abs() > f64::EPSILON { p95 / p05.abs() } else { 0.0 };
    let tail_ratio_99_01 = if p01.abs() > f64::EPSILON { p99 / p01.abs() } else { 0.0 };
    let bias_label = if tail_ratio <= 0.6 {
        "DOWNSIDE_HEAVY"
    } else if tail_ratio <= 0.85 {
        "SLIGHT_DOWNSIDE"
    } else if tail_ratio < 1.15 {
        "BALANCED"
    } else if tail_ratio < 1.4 {
        "SLIGHT_UPSIDE"
    } else {
        "UPSIDE_HEAVY"
    };
    TailRatioSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        pct_95_return: p95,
        pct_05_return: p05,
        pct_99_return: p99,
        pct_01_return: p01,
        tail_ratio,
        tail_ratio_99_01,
        bias_label: bias_label.into(),
        note: String::new(),
    }
}

/// RUNLEN compute: up/down day run length statistics over trailing 253
/// sessions. Uses sign of log return (0 → flat, included in neither run).
pub fn compute_runlen_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> RunLengthSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 20 {
        return RunLengthSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            trend_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    let mut up_runs: Vec<usize> = Vec::new();
    let mut down_runs: Vec<usize> = Vec::new();
    let mut longest_up = 0usize;
    let mut longest_down = 0usize;
    let mut cur_up = 0usize;
    let mut cur_down = 0usize;
    for r in &log_rets {
        if *r > 0.0 {
            if cur_down > 0 {
                down_runs.push(cur_down);
                if cur_down > longest_down { longest_down = cur_down; }
                cur_down = 0;
            }
            cur_up += 1;
        } else if *r < 0.0 {
            if cur_up > 0 {
                up_runs.push(cur_up);
                if cur_up > longest_up { longest_up = cur_up; }
                cur_up = 0;
            }
            cur_down += 1;
        } else {
            if cur_up > 0 {
                up_runs.push(cur_up);
                if cur_up > longest_up { longest_up = cur_up; }
                cur_up = 0;
            }
            if cur_down > 0 {
                down_runs.push(cur_down);
                if cur_down > longest_down { longest_down = cur_down; }
                cur_down = 0;
            }
        }
    }
    // Tail: whichever run is still in progress is the "current" run.
    let current_run: i32 = if cur_up > 0 {
        up_runs.push(cur_up);
        if cur_up > longest_up { longest_up = cur_up; }
        cur_up as i32
    } else if cur_down > 0 {
        down_runs.push(cur_down);
        if cur_down > longest_down { longest_down = cur_down; }
        -(cur_down as i32)
    } else {
        0
    };
    let avg_up = if up_runs.is_empty() {
        0.0
    } else {
        up_runs.iter().sum::<usize>() as f64 / up_runs.len() as f64
    };
    let avg_down = if down_runs.is_empty() {
        0.0
    } else {
        down_runs.iter().sum::<usize>() as f64 / down_runs.len() as f64
    };
    let avg_run = (avg_up + avg_down) / 2.0;
    let longest_any = longest_up.max(longest_down) as f64;
    // Label combines avg run length and longest run length.
    let trend_label = if avg_run < 1.4 && longest_any < 4.0 {
        "CHOPPY"
    } else if avg_run < 1.7 && longest_any < 6.0 {
        "MIXED"
    } else if avg_run < 2.2 || longest_any < 8.0 {
        "TRENDING"
    } else {
        "STRONG_TRENDING"
    };
    RunLengthSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        avg_up_run: avg_up,
        avg_down_run: avg_down,
        longest_up_run: longest_up,
        longest_down_run: longest_down,
        up_runs_count: up_runs.len(),
        down_runs_count: down_runs.len(),
        current_run_length: current_run,
        trend_label: trend_label.into(),
        note: String::new(),
    }
}

/// DAYRANGE compute: average (high-low)/close ratio over 60d vs 252d
/// baseline. Compression ratio < 1 → tight; > 1 → expanded.
pub fn compute_dayrange_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DailyRangeSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let window: Vec<&HistoricalPriceRow> = sorted.iter().rev().take(253).rev().copied().collect();
    if window.len() < 20 {
        return DailyRangeSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            range_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} bars", window.len()),
            ..Default::default()
        };
    }
    // Per-bar range ratio.
    let ratios: Vec<f64> = window.iter()
        .filter(|r| r.close > 0.0 && r.high >= r.low)
        .map(|r| ((r.high - r.low) / r.close) * 100.0)
        .collect();
    if ratios.len() < 20 {
        return DailyRangeSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            range_label: "INSUFFICIENT_DATA".into(),
            note: "insufficient valid bars".into(),
            ..Default::default()
        };
    }
    let avg_all: f64 = ratios.iter().sum::<f64>() / ratios.len() as f64;
    let take60 = ratios.len().min(60);
    let slice60 = &ratios[ratios.len() - take60..];
    let avg60: f64 = slice60.iter().sum::<f64>() / take60 as f64;
    let latest = *ratios.last().unwrap();
    let widest = ratios.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let narrowest = ratios.iter().cloned().fold(f64::INFINITY, f64::min);
    let compression = if avg_all > f64::EPSILON { avg60 / avg_all } else { 1.0 };
    let range_label = if compression <= 0.75 {
        "TIGHT"
    } else if compression <= 0.9 {
        "COMPRESSED"
    } else if compression < 1.1 {
        "NORMAL"
    } else if compression < 1.35 {
        "EXPANDED"
    } else {
        "VERY_EXPANDED"
    };
    DailyRangeSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        avg_range_60_pct: avg60,
        avg_range_252_pct: avg_all,
        latest_range_pct: latest,
        compression_ratio: compression,
        widest_range_pct: widest,
        narrowest_range_pct: narrowest,
        range_label: range_label.into(),
        note: String::new(),
    }
}

// ── ADR-131 Round 23 computes (AUTOCOR / HURST / HITRATE / GLASYM / VOLRATIO) ──

/// Helper: autocorrelation of a return series at a given lag, computed
/// via the standard estimator `sum((r_t - mean)(r_{t-k} - mean)) /
/// sum((r_t - mean)^2)`. Returns 0.0 when the series is too short
/// (<= lag) or the denominator is 0.
fn acf_at_lag(rets: &[f64], lag: usize) -> f64 {
    if lag == 0 || rets.len() <= lag {
        return 0.0;
    }
    let n = rets.len() as f64;
    let mean: f64 = rets.iter().sum::<f64>() / n;
    let denom: f64 = rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>();
    if denom <= f64::EPSILON {
        return 0.0;
    }
    let num: f64 = (lag..rets.len())
        .map(|t| (rets[t] - mean) * (rets[t - lag] - mean))
        .sum();
    num / denom
}

/// AUTOCOR compute: autocorrelation of log returns at lags 1/5/10/20.
/// Labels from lag-1 ACF: strong mean-reversion, mean-reversion,
/// neutral, momentum, strong momentum.
pub fn compute_autocor_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> AutocorrelationSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return AutocorrelationSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            regime_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    let mean: f64 = log_rets.iter().sum::<f64>() / log_rets.len() as f64;
    let lag1 = acf_at_lag(&log_rets, 1);
    let lag5 = acf_at_lag(&log_rets, 5);
    let lag10 = acf_at_lag(&log_rets, 10);
    let lag20 = acf_at_lag(&log_rets, 20);
    let regime_label = if lag1 <= -0.15 {
        "STRONG_MEAN_REVERT"
    } else if lag1 <= -0.05 {
        "MEAN_REVERT"
    } else if lag1 < 0.05 {
        "NEUTRAL"
    } else if lag1 < 0.15 {
        "MOMENTUM"
    } else {
        "STRONG_MOMENTUM"
    };
    AutocorrelationSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        lag1_acf: lag1,
        lag5_acf: lag5,
        lag10_acf: lag10,
        lag20_acf: lag20,
        mean_log_return: mean,
        regime_label: regime_label.into(),
        note: String::new(),
    }
}

/// HURST compute: Hurst exponent via rescaled-range analysis.
/// Partitions the log return series into non-overlapping chunks of
/// size `scale`, computes R/S (range of cumulative deviations divided
/// by stdev) per chunk, averages across chunks, and regresses
/// `log(R/S_avg)` against `log(scale)`. The slope is H.
pub fn compute_hurst_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> HurstSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 40 {
        return HurstSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            memory_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    // Build candidate scales: powers-of-two-ish, bounded so we always get
    // at least 2 chunks per scale.
    let n = log_rets.len();
    let candidate_scales: Vec<usize> = [8, 12, 16, 24, 32, 48, 64, 96, 128]
        .into_iter()
        .filter(|&s| s <= n / 2)
        .collect();
    if candidate_scales.len() < 2 {
        return HurstSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            memory_label: "INSUFFICIENT_DATA".into(),
            note: "too few R/S scales".into(),
            ..Default::default()
        };
    }

    let mut xs: Vec<f64> = Vec::new();
    let mut ys: Vec<f64> = Vec::new();
    for &scale in &candidate_scales {
        let num_chunks = n / scale;
        if num_chunks == 0 {
            continue;
        }
        let mut rs_vals: Vec<f64> = Vec::with_capacity(num_chunks);
        for c in 0..num_chunks {
            let start = c * scale;
            let end = start + scale;
            let slice = &log_rets[start..end];
            let mean: f64 = slice.iter().sum::<f64>() / scale as f64;
            // Cumulative deviations from the chunk mean.
            let mut cum: Vec<f64> = Vec::with_capacity(scale);
            let mut running = 0.0;
            for r in slice {
                running += r - mean;
                cum.push(running);
            }
            let max_c = cum.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let min_c = cum.iter().cloned().fold(f64::INFINITY, f64::min);
            let range = max_c - min_c;
            let var: f64 = slice.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / scale as f64;
            let sd = var.sqrt();
            if sd > f64::EPSILON && range > 0.0 {
                rs_vals.push(range / sd);
            }
        }
        if rs_vals.is_empty() {
            continue;
        }
        let avg_rs: f64 = rs_vals.iter().sum::<f64>() / rs_vals.len() as f64;
        if avg_rs > 0.0 {
            xs.push((scale as f64).ln());
            ys.push(avg_rs.ln());
        }
    }
    if xs.len() < 2 {
        return HurstSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            memory_label: "INSUFFICIENT_DATA".into(),
            note: "R/S regression had < 2 points".into(),
            ..Default::default()
        };
    }
    // OLS slope.
    let np = xs.len() as f64;
    let mean_x: f64 = xs.iter().sum::<f64>() / np;
    let mean_y: f64 = ys.iter().sum::<f64>() / np;
    let mut num = 0.0;
    let mut den = 0.0;
    for i in 0..xs.len() {
        let dx = xs[i] - mean_x;
        num += dx * (ys[i] - mean_y);
        den += dx * dx;
    }
    let h = if den > f64::EPSILON { num / den } else { 0.5 };
    let label = if h < 0.35 {
        "STRONG_MEAN_REVERT"
    } else if h < 0.45 {
        "MEAN_REVERT"
    } else if h < 0.55 {
        "RANDOM_WALK"
    } else if h < 0.65 {
        "PERSISTENT"
    } else {
        "STRONG_PERSISTENT"
    };
    HurstSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        hurst_exponent: h,
        scales_used: xs.len(),
        min_scale: *candidate_scales.iter().min().unwrap_or(&0),
        max_scale: *candidate_scales.iter().max().unwrap_or(&0),
        memory_label: label.into(),
        note: String::new(),
    }
}

/// HITRATE compute: share of positive-return bars over 5/20/60/252
/// trailing windows. Label combines the 20d and 60d hit rates: both
/// above 55% → BULLISH, both below 45% → BEARISH, otherwise NEUTRAL /
/// WEAK_BULLISH / WEAK_BEARISH based on the 20d alone.
pub fn compute_hitrate_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> HitRateSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 20 {
        return HitRateSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            hit_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    fn hit_over(rets: &[f64], take: usize) -> f64 {
        let start = rets.len().saturating_sub(take);
        let slice = &rets[start..];
        if slice.is_empty() { return 0.0; }
        let up = slice.iter().filter(|&&r| r > 0.0).count() as f64;
        up / slice.len() as f64
    }
    let h5 = hit_over(&log_rets, 5) * 100.0;
    let h20 = hit_over(&log_rets, 20) * 100.0;
    let h60 = hit_over(&log_rets, 60) * 100.0;
    let h252 = hit_over(&log_rets, 252) * 100.0;
    let up = log_rets.iter().filter(|&&r| r > 0.0).count();
    let down = log_rets.iter().filter(|&&r| r < 0.0).count();
    let flat = log_rets.len() - up - down;

    let label = if h20 >= 60.0 && h60 >= 55.0 {
        "BULLISH"
    } else if h20 >= 55.0 {
        "WEAK_BULLISH"
    } else if h20 <= 40.0 && h60 <= 45.0 {
        "BEARISH"
    } else if h20 <= 45.0 {
        "WEAK_BEARISH"
    } else {
        "NEUTRAL"
    };
    HitRateSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        hitrate_5d: h5,
        hitrate_20d: h20,
        hitrate_60d: h60,
        hitrate_252d: h252,
        up_days: up,
        down_days: down,
        flat_days: flat,
        hit_label: label.into(),
        note: String::new(),
    }
}

/// GLASYM compute: average and median magnitude of up vs down days.
/// Magnitudes are expressed as percent log returns × 100.
pub fn compute_glasym_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> GainLossAsymmetrySnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 20 {
        return GainLossAsymmetrySnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            asymmetry_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    let mut ups: Vec<f64> = log_rets.iter().filter(|&&r| r > 0.0).map(|r| r * 100.0).collect();
    let mut downs: Vec<f64> = log_rets.iter().filter(|&&r| r < 0.0).map(|r| -r * 100.0).collect();
    if ups.is_empty() || downs.is_empty() {
        return GainLossAsymmetrySnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            asymmetry_label: "INSUFFICIENT_DATA".into(),
            note: "all-up or all-down window".into(),
            up_days: ups.len(),
            down_days: downs.len(),
            ..Default::default()
        };
    }
    let avg_up: f64 = ups.iter().sum::<f64>() / ups.len() as f64;
    let avg_down: f64 = downs.iter().sum::<f64>() / downs.len() as f64;
    ups.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    downs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_up = quantile_f64(&ups, 0.5);
    let median_down = quantile_f64(&downs, 0.5);
    let ratio = if avg_down > f64::EPSILON { avg_up / avg_down } else { 0.0 };
    let label = if ratio <= 0.75 {
        "DOWNSIDE_HEAVY"
    } else if ratio <= 0.9 {
        "SLIGHT_DOWNSIDE"
    } else if ratio < 1.1 {
        "BALANCED"
    } else if ratio < 1.3 {
        "SLIGHT_UPSIDE"
    } else {
        "UPSIDE_HEAVY"
    };
    GainLossAsymmetrySnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        avg_up_pct: avg_up,
        avg_down_pct: avg_down,
        median_up_pct: median_up,
        median_down_pct: median_down,
        magnitude_ratio: ratio,
        up_days: ups.len(),
        down_days: downs.len(),
        asymmetry_label: label.into(),
        note: String::new(),
    }
}

/// VOLRATIO compute: up-day vs down-day volume summary over the
/// trailing 253-session window. Emits INSUFFICIENT_DATA when the HP
/// cache was populated without volume (all zeros).
pub fn compute_volratio_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> VolumeRatioSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let window: Vec<&HistoricalPriceRow> = sorted.iter().rev().take(253).rev().copied().collect();
    if window.len() < 20 {
        return VolumeRatioSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            flow_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} bars", window.len()),
            ..Default::default()
        };
    }
    let mut up_vols: Vec<f64> = Vec::new();
    let mut down_vols: Vec<f64> = Vec::new();
    for w in window.windows(2) {
        let prev = w[0].close;
        let curr = w[1].close;
        let vol = w[1].volume;
        if prev > 0.0 && curr > 0.0 && vol > 0.0 {
            let r = (curr / prev).ln();
            if r > 0.0 {
                up_vols.push(vol);
            } else if r < 0.0 {
                down_vols.push(vol);
            }
        }
    }
    if up_vols.is_empty() || down_vols.is_empty() {
        return VolumeRatioSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            flow_label: "INSUFFICIENT_DATA".into(),
            note: "HP cache lacks volume or one side empty".into(),
            up_days: up_vols.len(),
            down_days: down_vols.len(),
            ..Default::default()
        };
    }
    let avg_up: f64 = up_vols.iter().sum::<f64>() / up_vols.len() as f64;
    let avg_down: f64 = down_vols.iter().sum::<f64>() / down_vols.len() as f64;
    let max_up = up_vols.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let max_down = down_vols.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mut sorted_up = up_vols.clone();
    let mut sorted_down = down_vols.clone();
    sorted_up.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    sorted_down.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_up = quantile_f64(&sorted_up, 0.5);
    let median_down = quantile_f64(&sorted_down, 0.5);
    let ratio = if avg_down > f64::EPSILON { avg_up / avg_down } else { 0.0 };
    let label = if ratio <= 0.8 {
        "DISTRIBUTION"
    } else if ratio <= 0.95 {
        "SLIGHT_DISTRIBUTION"
    } else if ratio < 1.05 {
        "NEUTRAL"
    } else if ratio < 1.25 {
        "SLIGHT_ACCUMULATION"
    } else {
        "ACCUMULATION"
    };
    VolumeRatioSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        avg_up_volume: avg_up,
        avg_down_volume: avg_down,
        median_up_volume: median_up,
        median_down_volume: median_down,
        up_down_volume_ratio: ratio,
        max_up_volume: max_up,
        max_down_volume: max_down,
        up_days: up_vols.len(),
        down_days: down_vols.len(),
        flow_label: label.into(),
        note: String::new(),
    }
}

// ── ADR-109 SQLite schema + helpers ────────────────────────────────────────

pub fn create_research_tables_v2(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_dividends (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_earnings_estimates (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_rating_changes (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_dividends_updated ON research_dividends(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_estimates_updated ON research_earnings_estimates(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_rating_changes_updated ON research_rating_changes(updated_at);"
    ).map_err(|e| format!("create research_v2 tables: {e}"))?;
    Ok(())
}

pub fn upsert_dividends(conn: &Connection, symbol: &str, rows: &[DividendRecord]) -> Result<(), String> {
    let _ = create_research_tables_v2(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("div json: {e}"))?;
    conn.execute(
        "INSERT INTO research_dividends(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert dividends: {e}"))?;
    Ok(())
}

pub fn get_dividends(conn: &Connection, symbol: &str) -> Result<Option<Vec<DividendRecord>>, String> {
    let _ = create_research_tables_v2(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_dividends WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_dividends: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_dividends: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_dividends: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_earnings_estimates(conn: &Connection, symbol: &str, rows: &[EarningsEstimate]) -> Result<(), String> {
    let _ = create_research_tables_v2(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("estimates json: {e}"))?;
    conn.execute(
        "INSERT INTO research_earnings_estimates(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert estimates: {e}"))?;
    Ok(())
}

pub fn get_earnings_estimates(conn: &Connection, symbol: &str) -> Result<Option<Vec<EarningsEstimate>>, String> {
    let _ = create_research_tables_v2(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_earnings_estimates WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_estimates: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_estimates: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_estimates: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_rating_changes(conn: &Connection, symbol: &str, rows: &[RatingChange]) -> Result<(), String> {
    let _ = create_research_tables_v2(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("rating changes json: {e}"))?;
    conn.execute(
        "INSERT INTO research_rating_changes(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rating changes: {e}"))?;
    Ok(())
}

pub fn get_rating_changes(conn: &Connection, symbol: &str) -> Result<Option<Vec<RatingChange>>, String> {
    let _ = create_research_tables_v2(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_rating_changes WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_rating_changes: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_rating_changes: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_rating_changes: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── ADR-110 SQLite schema + helpers ────────────────────────────────────────

pub fn create_research_tables_v3(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_financials (
            symbol TEXT PRIMARY KEY,
            bundle_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_executives (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_financials_updated ON research_financials(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_executives_updated ON research_executives(updated_at);"
    ).map_err(|e| format!("create research_v3 tables: {e}"))?;
    Ok(())
}

pub fn upsert_financials(conn: &Connection, symbol: &str, bundle: &FinancialStatements) -> Result<(), String> {
    let _ = create_research_tables_v3(conn);
    let json = serde_json::to_string(bundle).map_err(|e| format!("financials json: {e}"))?;
    conn.execute(
        "INSERT INTO research_financials(symbol, bundle_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET bundle_json=excluded.bundle_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert financials: {e}"))?;
    Ok(())
}

pub fn get_financials(conn: &Connection, symbol: &str) -> Result<Option<FinancialStatements>, String> {
    let _ = create_research_tables_v3(conn);
    let mut stmt = conn.prepare("SELECT bundle_json FROM research_financials WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_financials: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_financials: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_financials: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_executives(conn: &Connection, symbol: &str, rows: &[Executive]) -> Result<(), String> {
    let _ = create_research_tables_v3(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("executives json: {e}"))?;
    conn.execute(
        "INSERT INTO research_executives(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert executives: {e}"))?;
    Ok(())
}

pub fn get_executives(conn: &Connection, symbol: &str) -> Result<Option<Vec<Executive>>, String> {
    let _ = create_research_tables_v3(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_executives WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_executives: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_executives: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_executives: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── ADR-111 SQLite schema + helpers ────────────────────────────────────────

pub fn create_research_tables_v4(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_stock_splits (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_etf_holdings (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_analyst_recs (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_price_target (
            symbol TEXT PRIMARY KEY,
            target_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_esg (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_index_members (
            index_code TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_stock_splits_updated ON research_stock_splits(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_etf_holdings_updated ON research_etf_holdings(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_analyst_recs_updated ON research_analyst_recs(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_price_target_updated ON research_price_target(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_esg_updated ON research_esg(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_index_members_updated ON research_index_members(updated_at);"
    ).map_err(|e| format!("create research_v4 tables: {e}"))?;
    Ok(())
}

pub fn upsert_stock_splits(conn: &Connection, symbol: &str, rows: &[StockSplit]) -> Result<(), String> {
    let _ = create_research_tables_v4(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("splits json: {e}"))?;
    conn.execute(
        "INSERT INTO research_stock_splits(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert stock_splits: {e}"))?;
    Ok(())
}

pub fn get_stock_splits(conn: &Connection, symbol: &str) -> Result<Option<Vec<StockSplit>>, String> {
    let _ = create_research_tables_v4(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_stock_splits WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_splits: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_splits: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_splits: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_etf_holdings(conn: &Connection, symbol: &str, rows: &[EtfHolding]) -> Result<(), String> {
    let _ = create_research_tables_v4(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("etf holdings json: {e}"))?;
    conn.execute(
        "INSERT INTO research_etf_holdings(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert etf holdings: {e}"))?;
    Ok(())
}

pub fn get_etf_holdings(conn: &Connection, symbol: &str) -> Result<Option<Vec<EtfHolding>>, String> {
    let _ = create_research_tables_v4(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_etf_holdings WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_etf_holdings: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_etf_holdings: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_etf_holdings: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_analyst_recs(conn: &Connection, symbol: &str, rows: &[AnalystRecommendation]) -> Result<(), String> {
    let _ = create_research_tables_v4(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("analyst recs json: {e}"))?;
    conn.execute(
        "INSERT INTO research_analyst_recs(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert analyst_recs: {e}"))?;
    Ok(())
}

pub fn get_analyst_recs(conn: &Connection, symbol: &str) -> Result<Option<Vec<AnalystRecommendation>>, String> {
    let _ = create_research_tables_v4(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_analyst_recs WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_analyst_recs: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_analyst_recs: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_analyst_recs: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_price_target(conn: &Connection, symbol: &str, pt: &PriceTarget) -> Result<(), String> {
    let _ = create_research_tables_v4(conn);
    let json = serde_json::to_string(pt).map_err(|e| format!("price target json: {e}"))?;
    conn.execute(
        "INSERT INTO research_price_target(symbol, target_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET target_json=excluded.target_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert price_target: {e}"))?;
    Ok(())
}

pub fn get_price_target(conn: &Connection, symbol: &str) -> Result<Option<PriceTarget>, String> {
    let _ = create_research_tables_v4(conn);
    let mut stmt = conn.prepare("SELECT target_json FROM research_price_target WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_price_target: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_price_target: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_price_target: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_esg(conn: &Connection, symbol: &str, rows: &[EsgScore]) -> Result<(), String> {
    let _ = create_research_tables_v4(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("esg json: {e}"))?;
    conn.execute(
        "INSERT INTO research_esg(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert esg: {e}"))?;
    Ok(())
}

pub fn get_esg(conn: &Connection, symbol: &str) -> Result<Option<Vec<EsgScore>>, String> {
    let _ = create_research_tables_v4(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_esg WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_esg: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_esg: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_esg: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_index_members(conn: &Connection, index_code: &str, rows: &[IndexMember]) -> Result<(), String> {
    let _ = create_research_tables_v4(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("index members json: {e}"))?;
    conn.execute(
        "INSERT INTO research_index_members(index_code, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(index_code) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![index_code.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert index_members: {e}"))?;
    Ok(())
}

pub fn get_index_members(conn: &Connection, index_code: &str) -> Result<Option<Vec<IndexMember>>, String> {
    let _ = create_research_tables_v4(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_index_members WHERE index_code = ?1")
        .map_err(|e| format!("prepare get_index_members: {e}"))?;
    let mut r = stmt.query(params![index_code.to_uppercase()]).map_err(|e| format!("query get_index_members: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_index_members: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── ADR-112 Round 5 SQLite schema + helpers ────────────────────────────────

pub fn create_research_tables_v5(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_insider_trades (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_institutional_holders (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_shares_float (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_historical_price (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_earnings_surprise (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_insider_trades_updated ON research_insider_trades(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_institutional_holders_updated ON research_institutional_holders(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_shares_float_updated ON research_shares_float(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_historical_price_updated ON research_historical_price(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_earnings_surprise_updated ON research_earnings_surprise(updated_at);"
    ).map_err(|e| format!("create research_v5 tables: {e}"))?;
    Ok(())
}

pub fn upsert_insider_trades(conn: &Connection, symbol: &str, rows: &[InsiderTrade]) -> Result<(), String> {
    let _ = create_research_tables_v5(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("insider json: {e}"))?;
    conn.execute(
        "INSERT INTO research_insider_trades(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert insider: {e}"))?;
    Ok(())
}

pub fn get_insider_trades(conn: &Connection, symbol: &str) -> Result<Option<Vec<InsiderTrade>>, String> {
    let _ = create_research_tables_v5(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_insider_trades WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_insider: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_insider: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_insider: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_institutional_holders(conn: &Connection, symbol: &str, rows: &[InstitutionalHolder]) -> Result<(), String> {
    let _ = create_research_tables_v5(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("holders json: {e}"))?;
    conn.execute(
        "INSERT INTO research_institutional_holders(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert holders: {e}"))?;
    Ok(())
}

pub fn get_institutional_holders(conn: &Connection, symbol: &str) -> Result<Option<Vec<InstitutionalHolder>>, String> {
    let _ = create_research_tables_v5(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_institutional_holders WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_holders: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_holders: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_holders: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_shares_float(conn: &Connection, symbol: &str, snap: &SharesFloat) -> Result<(), String> {
    let _ = create_research_tables_v5(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("float json: {e}"))?;
    conn.execute(
        "INSERT INTO research_shares_float(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert float: {e}"))?;
    Ok(())
}

pub fn get_shares_float(conn: &Connection, symbol: &str) -> Result<Option<SharesFloat>, String> {
    let _ = create_research_tables_v5(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_shares_float WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_float: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_float: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_float: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_historical_price(conn: &Connection, symbol: &str, rows: &[HistoricalPriceRow]) -> Result<(), String> {
    let _ = create_research_tables_v5(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("hp json: {e}"))?;
    conn.execute(
        "INSERT INTO research_historical_price(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert hp: {e}"))?;
    Ok(())
}

pub fn get_historical_price(conn: &Connection, symbol: &str) -> Result<Option<Vec<HistoricalPriceRow>>, String> {
    let _ = create_research_tables_v5(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_historical_price WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_hp: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_hp: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_hp: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_earnings_surprises(conn: &Connection, symbol: &str, rows: &[EarningsSurprise]) -> Result<(), String> {
    let _ = create_research_tables_v5(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("surprise json: {e}"))?;
    conn.execute(
        "INSERT INTO research_earnings_surprise(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert surprise: {e}"))?;
    Ok(())
}

pub fn get_earnings_surprises(conn: &Connection, symbol: &str) -> Result<Option<Vec<EarningsSurprise>>, String> {
    let _ = create_research_tables_v5(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_earnings_surprise WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_surprise: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_surprise: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_surprise: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── ADR-113 Round 6 SQLite schema + helpers ────────────────────────────────

pub fn create_research_tables_v6(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_world_indices (
            snapshot_key TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_market_movers (
            snapshot_key TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_sector_performance (
            snapshot_key TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_wacc (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_world_indices_updated ON research_world_indices(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_market_movers_updated ON research_market_movers(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_sector_performance_updated ON research_sector_performance(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_wacc_updated ON research_wacc(updated_at);"
    ).map_err(|e| format!("create research_v6 tables: {e}"))?;
    Ok(())
}

pub fn upsert_world_indices(conn: &Connection, rows: &[WorldIndex]) -> Result<(), String> {
    let _ = create_research_tables_v6(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("wei json: {e}"))?;
    conn.execute(
        "INSERT INTO research_world_indices(snapshot_key, rows_json, updated_at) VALUES ('latest',?1,?2)
         ON CONFLICT(snapshot_key) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![json, now_ts()],
    ).map_err(|e| format!("upsert wei: {e}"))?;
    Ok(())
}

pub fn get_world_indices(conn: &Connection) -> Result<Option<Vec<WorldIndex>>, String> {
    let _ = create_research_tables_v6(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_world_indices WHERE snapshot_key='latest'")
        .map_err(|e| format!("prepare get_wei: {e}"))?;
    let mut r = stmt.query([]).map_err(|e| format!("query get_wei: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_wei: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_market_movers(conn: &Connection, movers: &MarketMovers) -> Result<(), String> {
    let _ = create_research_tables_v6(conn);
    let json = serde_json::to_string(movers).map_err(|e| format!("mov json: {e}"))?;
    conn.execute(
        "INSERT INTO research_market_movers(snapshot_key, snapshot_json, updated_at) VALUES ('latest',?1,?2)
         ON CONFLICT(snapshot_key) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![json, now_ts()],
    ).map_err(|e| format!("upsert mov: {e}"))?;
    Ok(())
}

pub fn get_market_movers(conn: &Connection) -> Result<Option<MarketMovers>, String> {
    let _ = create_research_tables_v6(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_market_movers WHERE snapshot_key='latest'")
        .map_err(|e| format!("prepare get_mov: {e}"))?;
    let mut r = stmt.query([]).map_err(|e| format!("query get_mov: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_mov: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_sector_performance(conn: &Connection, rows: &[SectorPerformance]) -> Result<(), String> {
    let _ = create_research_tables_v6(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("indu json: {e}"))?;
    conn.execute(
        "INSERT INTO research_sector_performance(snapshot_key, rows_json, updated_at) VALUES ('latest',?1,?2)
         ON CONFLICT(snapshot_key) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![json, now_ts()],
    ).map_err(|e| format!("upsert indu: {e}"))?;
    Ok(())
}

pub fn get_sector_performance(conn: &Connection) -> Result<Option<Vec<SectorPerformance>>, String> {
    let _ = create_research_tables_v6(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_sector_performance WHERE snapshot_key='latest'")
        .map_err(|e| format!("prepare get_indu: {e}"))?;
    let mut r = stmt.query([]).map_err(|e| format!("query get_indu: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_indu: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_wacc(conn: &Connection, symbol: &str, snap: &WaccSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v6(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("wacc json: {e}"))?;
    conn.execute(
        "INSERT INTO research_wacc(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert wacc: {e}"))?;
    Ok(())
}

pub fn get_wacc(conn: &Connection, symbol: &str) -> Result<Option<WaccSnapshot>, String> {
    let _ = create_research_tables_v6(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_wacc WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_wacc: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_wacc: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_wacc: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── ADR-114 Round 7 SQLite schema + helpers ───────────────────────────────

pub fn create_research_tables_v7(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_currency_rates (
            snapshot_key TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_beta (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_ddm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_relative_valuation (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_figi (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_currency_rates_updated ON research_currency_rates(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_beta_updated ON research_beta(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_ddm_updated ON research_ddm(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_relative_valuation_updated ON research_relative_valuation(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_figi_updated ON research_figi(updated_at);"
    ).map_err(|e| format!("create research_v7 tables: {e}"))?;
    Ok(())
}

pub fn upsert_currency_rates(conn: &Connection, rows: &[CurrencyRate]) -> Result<(), String> {
    let _ = create_research_tables_v7(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("wcr json: {e}"))?;
    conn.execute(
        "INSERT INTO research_currency_rates(snapshot_key, rows_json, updated_at) VALUES ('latest',?1,?2)
         ON CONFLICT(snapshot_key) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![json, now_ts()],
    ).map_err(|e| format!("upsert wcr: {e}"))?;
    Ok(())
}

pub fn get_currency_rates(conn: &Connection) -> Result<Option<Vec<CurrencyRate>>, String> {
    let _ = create_research_tables_v7(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_currency_rates WHERE snapshot_key='latest'")
        .map_err(|e| format!("prepare get_wcr: {e}"))?;
    let mut r = stmt.query([]).map_err(|e| format!("query get_wcr: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_wcr: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_beta(conn: &Connection, symbol: &str, snap: &BetaSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v7(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("beta json: {e}"))?;
    conn.execute(
        "INSERT INTO research_beta(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert beta: {e}"))?;
    Ok(())
}

pub fn get_beta(conn: &Connection, symbol: &str) -> Result<Option<BetaSnapshot>, String> {
    let _ = create_research_tables_v7(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_beta WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_beta: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_beta: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_beta: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_ddm(conn: &Connection, symbol: &str, snap: &DdmSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v7(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ddm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ddm(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ddm: {e}"))?;
    Ok(())
}

pub fn get_ddm(conn: &Connection, symbol: &str) -> Result<Option<DdmSnapshot>, String> {
    let _ = create_research_tables_v7(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_ddm WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ddm: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_ddm: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ddm: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_relative_valuation(conn: &Connection, symbol: &str, snap: &RelativeValuation) -> Result<(), String> {
    let _ = create_research_tables_v7(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("rv json: {e}"))?;
    conn.execute(
        "INSERT INTO research_relative_valuation(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rv: {e}"))?;
    Ok(())
}

pub fn get_relative_valuation(conn: &Connection, symbol: &str) -> Result<Option<RelativeValuation>, String> {
    let _ = create_research_tables_v7(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_relative_valuation WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_rv: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_rv: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_rv: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_figi(conn: &Connection, symbol: &str, snap: &FigiSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v7(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("figi json: {e}"))?;
    conn.execute(
        "INSERT INTO research_figi(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert figi: {e}"))?;
    Ok(())
}

pub fn get_figi(conn: &Connection, symbol: &str) -> Result<Option<FigiSnapshot>, String> {
    let _ = create_research_tables_v7(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_figi WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_figi: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_figi: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_figi: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── ADR-115 Round 8 schema: HRA / DCF / SVM / OMON / IVOL ────────────────

pub fn create_research_tables_v8(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_hra (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_dcf (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_svm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_options_chain (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_ivol (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_hra_updated            ON research_hra(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_dcf_updated            ON research_dcf(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_svm_updated            ON research_svm(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_options_chain_updated  ON research_options_chain(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_ivol_updated           ON research_ivol(updated_at);"
    ).map_err(|e| format!("create research_v8 tables: {e}"))?;
    Ok(())
}

pub fn upsert_hra(conn: &Connection, symbol: &str, snap: &HraSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v8(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("hra json: {e}"))?;
    conn.execute(
        "INSERT INTO research_hra(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert hra: {e}"))?;
    Ok(())
}

pub fn get_hra(conn: &Connection, symbol: &str) -> Result<Option<HraSnapshot>, String> {
    let _ = create_research_tables_v8(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_hra WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_hra: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_hra: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_hra: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_dcf(conn: &Connection, symbol: &str, snap: &DcfSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v8(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("dcf json: {e}"))?;
    conn.execute(
        "INSERT INTO research_dcf(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert dcf: {e}"))?;
    Ok(())
}

pub fn get_dcf(conn: &Connection, symbol: &str) -> Result<Option<DcfSnapshot>, String> {
    let _ = create_research_tables_v8(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_dcf WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_dcf: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_dcf: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_dcf: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_svm(conn: &Connection, symbol: &str, snap: &SvmSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v8(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("svm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_svm(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert svm: {e}"))?;
    Ok(())
}

pub fn get_svm(conn: &Connection, symbol: &str) -> Result<Option<SvmSnapshot>, String> {
    let _ = create_research_tables_v8(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_svm WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_svm: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_svm: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_svm: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_options_chain(conn: &Connection, symbol: &str, snap: &OptionsChainSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v8(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("options chain json: {e}"))?;
    conn.execute(
        "INSERT INTO research_options_chain(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert options chain: {e}"))?;
    Ok(())
}

pub fn get_options_chain(conn: &Connection, symbol: &str) -> Result<Option<OptionsChainSnapshot>, String> {
    let _ = create_research_tables_v8(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_options_chain WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_options_chain: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_options_chain: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_options_chain: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_ivol(conn: &Connection, symbol: &str, snap: &IvolSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v8(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ivol json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ivol(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ivol: {e}"))?;
    Ok(())
}

pub fn get_ivol(conn: &Connection, symbol: &str) -> Result<Option<IvolSnapshot>, String> {
    let _ = create_research_tables_v8(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_ivol WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ivol: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_ivol: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ivol: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-116 Round 9 schema: SEAG / COR / TRA / TECH / SKEW ───────────────

pub fn create_research_tables_v9(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_seasonality (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_correlation (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_total_return (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_technicals (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_vol_skew (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_seasonality_updated  ON research_seasonality(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_correlation_updated  ON research_correlation(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_total_return_updated ON research_total_return(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_technicals_updated   ON research_technicals(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_vol_skew_updated     ON research_vol_skew(updated_at);"
    ).map_err(|e| format!("create research_v9 tables: {e}"))?;
    Ok(())
}

pub fn upsert_seasonality(conn: &Connection, symbol: &str, snap: &SeasonalitySnapshot) -> Result<(), String> {
    let _ = create_research_tables_v9(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("seasonality json: {e}"))?;
    conn.execute(
        "INSERT INTO research_seasonality(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert seasonality: {e}"))?;
    Ok(())
}

pub fn get_seasonality(conn: &Connection, symbol: &str) -> Result<Option<SeasonalitySnapshot>, String> {
    let _ = create_research_tables_v9(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_seasonality WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_seasonality: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_seasonality: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_seasonality: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_correlation(conn: &Connection, symbol: &str, snap: &CorrelationMatrix) -> Result<(), String> {
    let _ = create_research_tables_v9(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("correlation json: {e}"))?;
    conn.execute(
        "INSERT INTO research_correlation(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert correlation: {e}"))?;
    Ok(())
}

pub fn get_correlation(conn: &Connection, symbol: &str) -> Result<Option<CorrelationMatrix>, String> {
    let _ = create_research_tables_v9(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_correlation WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_correlation: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_correlation: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_correlation: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_total_return(conn: &Connection, symbol: &str, snap: &TotalReturnSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v9(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("total return json: {e}"))?;
    conn.execute(
        "INSERT INTO research_total_return(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert total return: {e}"))?;
    Ok(())
}

pub fn get_total_return(conn: &Connection, symbol: &str) -> Result<Option<TotalReturnSnapshot>, String> {
    let _ = create_research_tables_v9(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_total_return WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_total_return: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_total_return: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_total_return: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_vol_skew(conn: &Connection, symbol: &str, snap: &VolatilitySkew) -> Result<(), String> {
    let _ = create_research_tables_v9(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("vol skew json: {e}"))?;
    conn.execute(
        "INSERT INTO research_vol_skew(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert vol skew: {e}"))?;
    Ok(())
}

pub fn get_vol_skew(conn: &Connection, symbol: &str) -> Result<Option<VolatilitySkew>, String> {
    let _ = create_research_tables_v9(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_vol_skew WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_vol_skew: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_vol_skew: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_vol_skew: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-117 Round 10 schema: LEV / ACRL / RVOL / FCFY / SHRT ──────────────

pub fn create_research_tables_v10(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_leverage (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_accruals (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_realized_vol (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_fcf_yield (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_short_interest (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_leverage_updated        ON research_leverage(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_accruals_updated        ON research_accruals(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_realized_vol_updated    ON research_realized_vol(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_fcf_yield_updated       ON research_fcf_yield(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_short_interest_updated  ON research_short_interest(updated_at);"
    ).map_err(|e| format!("create research_v10 tables: {e}"))?;
    Ok(())
}

pub fn upsert_leverage(conn: &Connection, symbol: &str, snap: &LeverageSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v10(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("leverage json: {e}"))?;
    conn.execute(
        "INSERT INTO research_leverage(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert leverage: {e}"))?;
    Ok(())
}

pub fn get_leverage(conn: &Connection, symbol: &str) -> Result<Option<LeverageSnapshot>, String> {
    let _ = create_research_tables_v10(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_leverage WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_leverage: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_leverage: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_leverage: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_accruals(conn: &Connection, symbol: &str, snap: &AccrualsSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v10(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("accruals json: {e}"))?;
    conn.execute(
        "INSERT INTO research_accruals(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert accruals: {e}"))?;
    Ok(())
}

pub fn get_accruals(conn: &Connection, symbol: &str) -> Result<Option<AccrualsSnapshot>, String> {
    let _ = create_research_tables_v10(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_accruals WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_accruals: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_accruals: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_accruals: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_realized_vol(conn: &Connection, symbol: &str, snap: &RealizedVolSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v10(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("realized vol json: {e}"))?;
    conn.execute(
        "INSERT INTO research_realized_vol(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert realized vol: {e}"))?;
    Ok(())
}

pub fn get_realized_vol(conn: &Connection, symbol: &str) -> Result<Option<RealizedVolSnapshot>, String> {
    let _ = create_research_tables_v10(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_realized_vol WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_realized_vol: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_realized_vol: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_realized_vol: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_fcf_yield(conn: &Connection, symbol: &str, snap: &FcfYieldSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v10(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("fcf yield json: {e}"))?;
    conn.execute(
        "INSERT INTO research_fcf_yield(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert fcf yield: {e}"))?;
    Ok(())
}

pub fn get_fcf_yield(conn: &Connection, symbol: &str) -> Result<Option<FcfYieldSnapshot>, String> {
    let _ = create_research_tables_v10(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_fcf_yield WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_fcf_yield: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_fcf_yield: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_fcf_yield: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_short_interest(conn: &Connection, symbol: &str, snap: &ShortInterestSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v10(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("short interest json: {e}"))?;
    conn.execute(
        "INSERT INTO research_short_interest(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert short interest: {e}"))?;
    Ok(())
}

pub fn get_short_interest(conn: &Connection, symbol: &str) -> Result<Option<ShortInterestSnapshot>, String> {
    let _ = create_research_tables_v10(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_short_interest WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_short_interest: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_short_interest: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_short_interest: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-118 Godel Parity Round 11 schema + helpers ─────────────────────────

pub fn create_research_tables_v11(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_altman_z (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_piotroski (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_ohlc_vol (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_eps_beat (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_price_target_dispersion (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_altman_z_updated                 ON research_altman_z(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_piotroski_updated                ON research_piotroski(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_ohlc_vol_updated                 ON research_ohlc_vol(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_eps_beat_updated                 ON research_eps_beat(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_price_target_dispersion_updated  ON research_price_target_dispersion(updated_at);"
    ).map_err(|e| format!("create research_v11 tables: {e}"))?;
    Ok(())
}

pub fn upsert_altman_z(conn: &Connection, symbol: &str, snap: &AltmanZSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v11(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("altman_z json: {e}"))?;
    conn.execute(
        "INSERT INTO research_altman_z(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert altman_z: {e}"))?;
    Ok(())
}

pub fn get_altman_z(conn: &Connection, symbol: &str) -> Result<Option<AltmanZSnapshot>, String> {
    let _ = create_research_tables_v11(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_altman_z WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_altman_z: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_altman_z: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_altman_z: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_piotroski(conn: &Connection, symbol: &str, snap: &PiotroskiSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v11(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("piotroski json: {e}"))?;
    conn.execute(
        "INSERT INTO research_piotroski(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert piotroski: {e}"))?;
    Ok(())
}

pub fn get_piotroski(conn: &Connection, symbol: &str) -> Result<Option<PiotroskiSnapshot>, String> {
    let _ = create_research_tables_v11(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_piotroski WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_piotroski: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_piotroski: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_piotroski: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_ohlc_vol(conn: &Connection, symbol: &str, snap: &OhlcVolSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v11(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ohlc_vol json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ohlc_vol(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ohlc_vol: {e}"))?;
    Ok(())
}

pub fn get_ohlc_vol(conn: &Connection, symbol: &str) -> Result<Option<OhlcVolSnapshot>, String> {
    let _ = create_research_tables_v11(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_ohlc_vol WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ohlc_vol: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_ohlc_vol: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ohlc_vol: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_eps_beat(conn: &Connection, symbol: &str, snap: &EpsBeatSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v11(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("eps_beat json: {e}"))?;
    conn.execute(
        "INSERT INTO research_eps_beat(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert eps_beat: {e}"))?;
    Ok(())
}

pub fn get_eps_beat(conn: &Connection, symbol: &str) -> Result<Option<EpsBeatSnapshot>, String> {
    let _ = create_research_tables_v11(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_eps_beat WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_eps_beat: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_eps_beat: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_eps_beat: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_price_target_dispersion(conn: &Connection, symbol: &str, snap: &PriceTargetDispersion) -> Result<(), String> {
    let _ = create_research_tables_v11(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("price_target_dispersion json: {e}"))?;
    conn.execute(
        "INSERT INTO research_price_target_dispersion(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert price_target_dispersion: {e}"))?;
    Ok(())
}

pub fn get_price_target_dispersion(conn: &Connection, symbol: &str) -> Result<Option<PriceTargetDispersion>, String> {
    let _ = create_research_tables_v11(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_price_target_dispersion WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_price_target_dispersion: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_price_target_dispersion: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_price_target_dispersion: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-119 Godel Parity Round 12 schema + helpers ─────────────────────────

pub fn create_research_tables_v12(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_insider_activity (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_divg (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_earm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_sector_rotation (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_updm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_insider_activity_updated ON research_insider_activity(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_divg_updated             ON research_divg(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_earm_updated             ON research_earm(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_sector_rotation_updated  ON research_sector_rotation(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_updm_updated             ON research_updm(updated_at);"
    ).map_err(|e| format!("create research_v12 tables: {e}"))?;
    Ok(())
}

pub fn upsert_insider_activity(conn: &Connection, symbol: &str, snap: &InsiderActivitySnapshot) -> Result<(), String> {
    let _ = create_research_tables_v12(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("insider_activity json: {e}"))?;
    conn.execute(
        "INSERT INTO research_insider_activity(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert insider_activity: {e}"))?;
    Ok(())
}

pub fn get_insider_activity(conn: &Connection, symbol: &str) -> Result<Option<InsiderActivitySnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_insider_activity WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_insider_activity: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_insider_activity: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_insider_activity: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_divg(conn: &Connection, symbol: &str, snap: &DivgSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v12(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("divg json: {e}"))?;
    conn.execute(
        "INSERT INTO research_divg(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert divg: {e}"))?;
    Ok(())
}

pub fn get_divg(conn: &Connection, symbol: &str) -> Result<Option<DivgSnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_divg WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_divg: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_divg: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_divg: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_earm(conn: &Connection, symbol: &str, snap: &EarmSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v12(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("earm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_earm(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert earm: {e}"))?;
    Ok(())
}

pub fn get_earm(conn: &Connection, symbol: &str) -> Result<Option<EarmSnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_earm WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_earm: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_earm: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_earm: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_sector_rotation(conn: &Connection, symbol: &str, snap: &SectorRotationSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v12(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("sector_rotation json: {e}"))?;
    conn.execute(
        "INSERT INTO research_sector_rotation(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert sector_rotation: {e}"))?;
    Ok(())
}

pub fn get_sector_rotation(conn: &Connection, symbol: &str) -> Result<Option<SectorRotationSnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_sector_rotation WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_sector_rotation: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_sector_rotation: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_sector_rotation: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_updm(conn: &Connection, symbol: &str, snap: &UpdmSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v12(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("updm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_updm(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert updm: {e}"))?;
    Ok(())
}

pub fn get_updm(conn: &Connection, symbol: &str) -> Result<Option<UpdmSnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_updm WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_updm: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_updm: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_updm: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-120 Godel Parity Round 13 schema + helpers ─────────────────────────

pub fn create_research_tables_v13(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_momentum (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_momentum_updated ON research_momentum(updated_at);

        CREATE TABLE IF NOT EXISTS research_liquidity (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_liquidity_updated ON research_liquidity(updated_at);

        CREATE TABLE IF NOT EXISTS research_breakout (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_breakout_updated ON research_breakout(updated_at);

        CREATE TABLE IF NOT EXISTS research_cash_cycle (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cash_cycle_updated ON research_cash_cycle(updated_at);

        CREATE TABLE IF NOT EXISTS research_credit (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_credit_updated ON research_credit(updated_at);",
    ).map_err(|e| format!("create v13 tables: {e}"))?;
    Ok(())
}

pub fn upsert_momentum(conn: &Connection, symbol: &str, snap: &MomentumSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v13(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("momentum json: {e}"))?;
    conn.execute(
        "INSERT INTO research_momentum(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert momentum: {e}"))?;
    Ok(())
}

pub fn get_momentum(conn: &Connection, symbol: &str) -> Result<Option<MomentumSnapshot>, String> {
    let _ = create_research_tables_v13(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_momentum WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_momentum: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_momentum: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_momentum: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_liquidity(conn: &Connection, symbol: &str, snap: &LiquiditySnapshot) -> Result<(), String> {
    let _ = create_research_tables_v13(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("liquidity json: {e}"))?;
    conn.execute(
        "INSERT INTO research_liquidity(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert liquidity: {e}"))?;
    Ok(())
}

pub fn get_liquidity(conn: &Connection, symbol: &str) -> Result<Option<LiquiditySnapshot>, String> {
    let _ = create_research_tables_v13(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_liquidity WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_liquidity: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_liquidity: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_liquidity: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_breakout(conn: &Connection, symbol: &str, snap: &BreakoutSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v13(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("breakout json: {e}"))?;
    conn.execute(
        "INSERT INTO research_breakout(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert breakout: {e}"))?;
    Ok(())
}

pub fn get_breakout(conn: &Connection, symbol: &str) -> Result<Option<BreakoutSnapshot>, String> {
    let _ = create_research_tables_v13(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_breakout WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_breakout: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_breakout: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_breakout: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_cash_cycle(conn: &Connection, symbol: &str, snap: &CashCycleSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v13(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cash_cycle json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cash_cycle(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cash_cycle: {e}"))?;
    Ok(())
}

pub fn get_cash_cycle(conn: &Connection, symbol: &str) -> Result<Option<CashCycleSnapshot>, String> {
    let _ = create_research_tables_v13(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_cash_cycle WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_cash_cycle: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_cash_cycle: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_cash_cycle: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_credit(conn: &Connection, symbol: &str, snap: &CreditSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v13(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("credit json: {e}"))?;
    conn.execute(
        "INSERT INTO research_credit(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert credit: {e}"))?;
    Ok(())
}

pub fn get_credit(conn: &Connection, symbol: &str) -> Result<Option<CreditSnapshot>, String> {
    let _ = create_research_tables_v13(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_credit WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_credit: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_credit: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_credit: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-121 Godel Parity Round 14 schema + helpers ─────────────────────────

pub fn create_research_tables_v14(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_growm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_growm_updated ON research_growm(updated_at);

        CREATE TABLE IF NOT EXISTS research_flow (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_flow_updated ON research_flow(updated_at);

        CREATE TABLE IF NOT EXISTS research_regime (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_regime_updated ON research_regime(updated_at);

        CREATE TABLE IF NOT EXISTS research_relvol (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_relvol_updated ON research_relvol(updated_at);

        CREATE TABLE IF NOT EXISTS research_margins (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_margins_updated ON research_margins(updated_at);",
    ).map_err(|e| format!("create v14 tables: {e}"))?;
    Ok(())
}

pub fn upsert_growm(conn: &Connection, symbol: &str, snap: &GrowmSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v14(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("growm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_growm(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert growm: {e}"))?;
    Ok(())
}

pub fn get_growm(conn: &Connection, symbol: &str) -> Result<Option<GrowmSnapshot>, String> {
    let _ = create_research_tables_v14(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_growm WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_growm: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_growm: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_growm: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_flow(conn: &Connection, symbol: &str, snap: &FlowSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v14(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("flow json: {e}"))?;
    conn.execute(
        "INSERT INTO research_flow(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert flow: {e}"))?;
    Ok(())
}

pub fn get_flow(conn: &Connection, symbol: &str) -> Result<Option<FlowSnapshot>, String> {
    let _ = create_research_tables_v14(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_flow WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_flow: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_flow: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_flow: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_regime(conn: &Connection, symbol: &str, snap: &RegimeSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v14(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("regime json: {e}"))?;
    conn.execute(
        "INSERT INTO research_regime(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert regime: {e}"))?;
    Ok(())
}

pub fn get_regime(conn: &Connection, symbol: &str) -> Result<Option<RegimeSnapshot>, String> {
    let _ = create_research_tables_v14(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_regime WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_regime: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_regime: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_regime: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_relvol(conn: &Connection, symbol: &str, snap: &RelVolSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v14(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("relvol json: {e}"))?;
    conn.execute(
        "INSERT INTO research_relvol(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert relvol: {e}"))?;
    Ok(())
}

pub fn get_relvol(conn: &Connection, symbol: &str) -> Result<Option<RelVolSnapshot>, String> {
    let _ = create_research_tables_v14(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_relvol WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_relvol: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_relvol: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_relvol: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_margins(conn: &Connection, symbol: &str, snap: &MarginsSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v14(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("margins json: {e}"))?;
    conn.execute(
        "INSERT INTO research_margins(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert margins: {e}"))?;
    Ok(())
}

pub fn get_margins(conn: &Connection, symbol: &str) -> Result<Option<MarginsSnapshot>, String> {
    let _ = create_research_tables_v14(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_margins WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_margins: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_margins: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_margins: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-122 Godel Parity Round 15 schema + helpers ─────────────────────────

pub fn create_research_tables_v15(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_val (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_val_updated ON research_val(updated_at);

        CREATE TABLE IF NOT EXISTS research_qual (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_qual_updated ON research_qual(updated_at);

        CREATE TABLE IF NOT EXISTS research_risk (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_risk_updated ON research_risk(updated_at);

        CREATE TABLE IF NOT EXISTS research_insstrk (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_insstrk_updated ON research_insstrk(updated_at);

        CREATE TABLE IF NOT EXISTS research_covg (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_covg_updated ON research_covg(updated_at);",
    ).map_err(|e| format!("create v15 tables: {e}"))?;
    Ok(())
}

pub fn upsert_val(conn: &Connection, symbol: &str, snap: &ValueSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v15(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("val json: {e}"))?;
    conn.execute(
        "INSERT INTO research_val(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert val: {e}"))?;
    Ok(())
}

pub fn get_val(conn: &Connection, symbol: &str) -> Result<Option<ValueSnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_val WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_val: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_val: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_val: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_qual(conn: &Connection, symbol: &str, snap: &QualitySnapshot) -> Result<(), String> {
    let _ = create_research_tables_v15(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("qual json: {e}"))?;
    conn.execute(
        "INSERT INTO research_qual(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert qual: {e}"))?;
    Ok(())
}

pub fn get_qual(conn: &Connection, symbol: &str) -> Result<Option<QualitySnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_qual WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_qual: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_qual: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_qual: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_risk(conn: &Connection, symbol: &str, snap: &RiskSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v15(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("risk json: {e}"))?;
    conn.execute(
        "INSERT INTO research_risk(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert risk: {e}"))?;
    Ok(())
}

pub fn get_risk(conn: &Connection, symbol: &str) -> Result<Option<RiskSnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_risk WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_risk: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_risk: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_risk: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_insstrk(conn: &Connection, symbol: &str, snap: &InsiderStreakSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v15(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("insstrk json: {e}"))?;
    conn.execute(
        "INSERT INTO research_insstrk(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert insstrk: {e}"))?;
    Ok(())
}

pub fn get_insstrk(conn: &Connection, symbol: &str) -> Result<Option<InsiderStreakSnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_insstrk WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_insstrk: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_insstrk: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_insstrk: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_covg(conn: &Connection, symbol: &str, snap: &CoverageSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v15(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("covg json: {e}"))?;
    conn.execute(
        "INSERT INTO research_covg(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert covg: {e}"))?;
    Ok(())
}

pub fn get_covg(conn: &Connection, symbol: &str) -> Result<Option<CoverageSnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_covg WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_covg: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_covg: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_covg: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-123 Round 16 schema + helpers ──────────────────────────────────────

pub fn create_research_tables_v16(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v15(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_vrk (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_vrk_updated ON research_vrk(updated_at);

        CREATE TABLE IF NOT EXISTS research_qrk (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_qrk_updated ON research_qrk(updated_at);

        CREATE TABLE IF NOT EXISTS research_rrk (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_rrk_updated ON research_rrk(updated_at);

        CREATE TABLE IF NOT EXISTS research_relepsgr (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_relepsgr_updated ON research_relepsgr(updated_at);

        CREATE TABLE IF NOT EXISTS research_pead (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_pead_updated ON research_pead(updated_at);",
    ).map_err(|e| format!("create v16 tables: {e}"))?;
    Ok(())
}

pub fn upsert_vrk(conn: &Connection, symbol: &str, snap: &ValueRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v16(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("vrk json: {e}"))?;
    conn.execute(
        "INSERT INTO research_vrk(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert vrk: {e}"))?;
    Ok(())
}

pub fn get_vrk(conn: &Connection, symbol: &str) -> Result<Option<ValueRankSnapshot>, String> {
    let _ = create_research_tables_v16(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_vrk WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_vrk: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_vrk: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_vrk: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_qrk(conn: &Connection, symbol: &str, snap: &QualityRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v16(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("qrk json: {e}"))?;
    conn.execute(
        "INSERT INTO research_qrk(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert qrk: {e}"))?;
    Ok(())
}

pub fn get_qrk(conn: &Connection, symbol: &str) -> Result<Option<QualityRankSnapshot>, String> {
    let _ = create_research_tables_v16(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_qrk WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_qrk: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_qrk: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_qrk: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_rrk(conn: &Connection, symbol: &str, snap: &RiskRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v16(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("rrk json: {e}"))?;
    conn.execute(
        "INSERT INTO research_rrk(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rrk: {e}"))?;
    Ok(())
}

pub fn get_rrk(conn: &Connection, symbol: &str) -> Result<Option<RiskRankSnapshot>, String> {
    let _ = create_research_tables_v16(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_rrk WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_rrk: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_rrk: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_rrk: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_relepsgr(conn: &Connection, symbol: &str, snap: &RelativeEpsGrowthSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v16(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("relepsgr json: {e}"))?;
    conn.execute(
        "INSERT INTO research_relepsgr(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert relepsgr: {e}"))?;
    Ok(())
}

pub fn get_relepsgr(conn: &Connection, symbol: &str) -> Result<Option<RelativeEpsGrowthSnapshot>, String> {
    let _ = create_research_tables_v16(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_relepsgr WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_relepsgr: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_relepsgr: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_relepsgr: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_pead(conn: &Connection, symbol: &str, snap: &PeadSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v16(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("pead json: {e}"))?;
    conn.execute(
        "INSERT INTO research_pead(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert pead: {e}"))?;
    Ok(())
}

pub fn get_pead(conn: &Connection, symbol: &str) -> Result<Option<PeadSnapshot>, String> {
    let _ = create_research_tables_v16(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_pead WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_pead: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_pead: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_pead: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

/// Whole-table scan of `research_val`. Used by VRK / sector-rank surfaces.
pub fn get_all_val(conn: &Connection) -> Result<Vec<ValueSnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_val")
        .map_err(|e| format!("prepare get_all_val: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_val: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<ValueSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_qual`. Used by QRK.
pub fn get_all_qual(conn: &Connection) -> Result<Vec<QualitySnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_qual")
        .map_err(|e| format!("prepare get_all_qual: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_qual: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<QualitySnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_risk`. Used by RRK.
pub fn get_all_risk(conn: &Connection) -> Result<Vec<RiskSnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_risk")
        .map_err(|e| format!("prepare get_all_risk: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_risk: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<RiskSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

// ── ADR-124 Round 17 schema + wrappers ────────────────────────────────────

pub fn create_research_tables_v17(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v16(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_sizef (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_sizef_updated ON research_sizef(updated_at);

        CREATE TABLE IF NOT EXISTS research_momf (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_momf_updated ON research_momf(updated_at);

        CREATE TABLE IF NOT EXISTS research_peadrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_peadrank_updated ON research_peadrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_fqm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_fqm_updated ON research_fqm(updated_at);

        CREATE TABLE IF NOT EXISTS research_revrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_revrank_updated ON research_revrank(updated_at);",
    ).map_err(|e| format!("create v17 tables: {e}"))?;
    Ok(())
}

pub fn upsert_sizef(conn: &Connection, symbol: &str, snap: &SizeFactorSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v17(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("sizef json: {e}"))?;
    conn.execute(
        "INSERT INTO research_sizef(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert sizef: {e}"))?;
    Ok(())
}

pub fn get_sizef(conn: &Connection, symbol: &str) -> Result<Option<SizeFactorSnapshot>, String> {
    let _ = create_research_tables_v17(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_sizef WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_sizef: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_sizef: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_sizef: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_momf(conn: &Connection, symbol: &str, snap: &MomentumRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v17(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("momf json: {e}"))?;
    conn.execute(
        "INSERT INTO research_momf(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert momf: {e}"))?;
    Ok(())
}

pub fn get_momf(conn: &Connection, symbol: &str) -> Result<Option<MomentumRankSnapshot>, String> {
    let _ = create_research_tables_v17(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_momf WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_momf: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_momf: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_momf: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_peadrank(conn: &Connection, symbol: &str, snap: &PeadRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v17(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("peadrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_peadrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert peadrank: {e}"))?;
    Ok(())
}

pub fn get_peadrank(conn: &Connection, symbol: &str) -> Result<Option<PeadRankSnapshot>, String> {
    let _ = create_research_tables_v17(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_peadrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_peadrank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_peadrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_peadrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_fqm(conn: &Connection, symbol: &str, snap: &FundamentalQualityMeterSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v17(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("fqm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_fqm(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert fqm: {e}"))?;
    Ok(())
}

pub fn get_fqm(conn: &Connection, symbol: &str) -> Result<Option<FundamentalQualityMeterSnapshot>, String> {
    let _ = create_research_tables_v17(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_fqm WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_fqm: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_fqm: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_fqm: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_revrank(conn: &Connection, symbol: &str, snap: &RevenueGrowthRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v17(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("revrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_revrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert revrank: {e}"))?;
    Ok(())
}

pub fn get_revrank(conn: &Connection, symbol: &str) -> Result<Option<RevenueGrowthRankSnapshot>, String> {
    let _ = create_research_tables_v17(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_revrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_revrank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_revrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_revrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

/// Whole-table scan of `research_momentum`. Used by MOMF.
pub fn get_all_momentum(conn: &Connection) -> Result<Vec<MomentumSnapshot>, String> {
    let _ = create_research_tables_v13(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_momentum")
        .map_err(|e| format!("prepare get_all_momentum: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_momentum: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<MomentumSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_pead`. Used by PEADRANK.
pub fn get_all_pead(conn: &Connection) -> Result<Vec<PeadSnapshot>, String> {
    let _ = create_research_tables_v16(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_pead")
        .map_err(|e| format!("prepare get_all_pead: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_pead: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<PeadSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

// ── ADR-125 Round 18 schema + wrappers ────────────────────────────────────

pub fn create_research_tables_v18(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v17(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_levrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_levrank_updated ON research_levrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_operank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_operank_updated ON research_operank(updated_at);

        CREATE TABLE IF NOT EXISTS research_fqmrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_fqmrank_updated ON research_fqmrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_liqrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_liqrank_updated ON research_liqrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_surpstk (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_surpstk_updated ON research_surpstk(updated_at);",
    ).map_err(|e| format!("create v18 tables: {e}"))?;
    Ok(())
}

pub fn upsert_levrank(conn: &Connection, symbol: &str, snap: &LeverageRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v18(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("levrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_levrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert levrank: {e}"))?;
    Ok(())
}

pub fn get_levrank(conn: &Connection, symbol: &str) -> Result<Option<LeverageRankSnapshot>, String> {
    let _ = create_research_tables_v18(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_levrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_levrank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_levrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_levrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_operank(conn: &Connection, symbol: &str, snap: &OperatingQualityRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v18(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("operank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_operank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert operank: {e}"))?;
    Ok(())
}

pub fn get_operank(conn: &Connection, symbol: &str) -> Result<Option<OperatingQualityRankSnapshot>, String> {
    let _ = create_research_tables_v18(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_operank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_operank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_operank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_operank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_fqmrank(conn: &Connection, symbol: &str, snap: &FqmRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v18(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("fqmrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_fqmrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert fqmrank: {e}"))?;
    Ok(())
}

pub fn get_fqmrank(conn: &Connection, symbol: &str) -> Result<Option<FqmRankSnapshot>, String> {
    let _ = create_research_tables_v18(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_fqmrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_fqmrank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_fqmrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_fqmrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_liqrank(conn: &Connection, symbol: &str, snap: &LiquidityRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v18(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("liqrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_liqrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert liqrank: {e}"))?;
    Ok(())
}

pub fn get_liqrank(conn: &Connection, symbol: &str) -> Result<Option<LiquidityRankSnapshot>, String> {
    let _ = create_research_tables_v18(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_liqrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_liqrank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_liqrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_liqrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_surpstk(conn: &Connection, symbol: &str, snap: &EarningsSurpriseStreakSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v18(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("surpstk json: {e}"))?;
    conn.execute(
        "INSERT INTO research_surpstk(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert surpstk: {e}"))?;
    Ok(())
}

pub fn get_surpstk(conn: &Connection, symbol: &str) -> Result<Option<EarningsSurpriseStreakSnapshot>, String> {
    let _ = create_research_tables_v18(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_surpstk WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_surpstk: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_surpstk: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_surpstk: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

/// Whole-table scan of `research_leverage`. Used by LEVRANK.
pub fn get_all_leverage(conn: &Connection) -> Result<Vec<LeverageSnapshot>, String> {
    let _ = create_research_tables_v10(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_leverage")
        .map_err(|e| format!("prepare get_all_leverage: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_leverage: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<LeverageSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_margins`. Used by OPERANK.
pub fn get_all_margins(conn: &Connection) -> Result<Vec<MarginsSnapshot>, String> {
    let _ = create_research_tables_v14(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_margins")
        .map_err(|e| format!("prepare get_all_margins: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_margins: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<MarginsSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_fqm`. Used by FQMRANK.
pub fn get_all_fqm(conn: &Connection) -> Result<Vec<FundamentalQualityMeterSnapshot>, String> {
    let _ = create_research_tables_v17(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_fqm")
        .map_err(|e| format!("prepare get_all_fqm: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_fqm: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<FundamentalQualityMeterSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_liquidity`. Used by LIQRANK.
pub fn get_all_liquidity(conn: &Connection) -> Result<Vec<LiquiditySnapshot>, String> {
    let _ = create_research_tables_v13(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_liquidity")
        .map_err(|e| format!("prepare get_all_liquidity: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_liquidity: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<LiquiditySnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

// ── ADR-126 Round 19 schema + wrappers ────────────────────────────────────

pub fn create_research_tables_v19(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v18(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_dvdrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_dvdrank_updated ON research_dvdrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_earmrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_earmrank_updated ON research_earmrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_updgrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_updgrank_updated ON research_updgrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_gy (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_gy_updated ON research_gy(updated_at);

        CREATE TABLE IF NOT EXISTS research_des (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_des_updated ON research_des(updated_at);",
    ).map_err(|e| format!("create v19 tables: {e}"))?;
    Ok(())
}

pub fn upsert_dvdrank(conn: &Connection, symbol: &str, snap: &DividendGrowthRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v19(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("dvdrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_dvdrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert dvdrank: {e}"))?;
    Ok(())
}

pub fn get_dvdrank(conn: &Connection, symbol: &str) -> Result<Option<DividendGrowthRankSnapshot>, String> {
    let _ = create_research_tables_v19(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_dvdrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_dvdrank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_dvdrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_dvdrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_earmrank(conn: &Connection, symbol: &str, snap: &EarningsMomentumRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v19(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("earmrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_earmrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert earmrank: {e}"))?;
    Ok(())
}

pub fn get_earmrank(conn: &Connection, symbol: &str) -> Result<Option<EarningsMomentumRankSnapshot>, String> {
    let _ = create_research_tables_v19(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_earmrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_earmrank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_earmrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_earmrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_updgrank(conn: &Connection, symbol: &str, snap: &UpgradeDowngradeRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v19(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("updgrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_updgrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert updgrank: {e}"))?;
    Ok(())
}

pub fn get_updgrank(conn: &Connection, symbol: &str) -> Result<Option<UpgradeDowngradeRankSnapshot>, String> {
    let _ = create_research_tables_v19(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_updgrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_updgrank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_updgrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_updgrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_gy(conn: &Connection, symbol: &str, snap: &GapYearlySnapshot) -> Result<(), String> {
    let _ = create_research_tables_v19(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("gy json: {e}"))?;
    conn.execute(
        "INSERT INTO research_gy(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert gy: {e}"))?;
    Ok(())
}

pub fn get_gy(conn: &Connection, symbol: &str) -> Result<Option<GapYearlySnapshot>, String> {
    let _ = create_research_tables_v19(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_gy WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_gy: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_gy: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_gy: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_des(conn: &Connection, symbol: &str, snap: &DailyEventStreakSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v19(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("des json: {e}"))?;
    conn.execute(
        "INSERT INTO research_des(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert des: {e}"))?;
    Ok(())
}

pub fn get_des(conn: &Connection, symbol: &str) -> Result<Option<DailyEventStreakSnapshot>, String> {
    let _ = create_research_tables_v19(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_des WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_des: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_des: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_des: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-127 Round 20 schema + wrappers ────────────────────────────────────

pub fn create_research_tables_v20(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v19(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_dvdyieldrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_dvdyieldrank_updated ON research_dvdyieldrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_shrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_shrank_updated ON research_shrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_atrann (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_atrann_updated ON research_atrann(updated_at);

        CREATE TABLE IF NOT EXISTS research_ddhist (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_ddhist_updated ON research_ddhist(updated_at);

        CREATE TABLE IF NOT EXISTS research_priceperf (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_priceperf_updated ON research_priceperf(updated_at);",
    ).map_err(|e| format!("create v20 tables: {e}"))?;
    Ok(())
}

pub fn upsert_dvdyieldrank(conn: &Connection, symbol: &str, snap: &DividendYieldRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v20(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("dvdyieldrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_dvdyieldrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert dvdyieldrank: {e}"))?;
    Ok(())
}

pub fn get_dvdyieldrank(conn: &Connection, symbol: &str) -> Result<Option<DividendYieldRankSnapshot>, String> {
    let _ = create_research_tables_v20(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_dvdyieldrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_dvdyieldrank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_dvdyieldrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_dvdyieldrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_shrank(conn: &Connection, symbol: &str, snap: &ShortInterestRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v20(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("shrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_shrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert shrank: {e}"))?;
    Ok(())
}

pub fn get_shrank(conn: &Connection, symbol: &str) -> Result<Option<ShortInterestRankSnapshot>, String> {
    let _ = create_research_tables_v20(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_shrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_shrank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_shrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_shrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_atrann(conn: &Connection, symbol: &str, snap: &AnnualizedAtrSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v20(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("atrann json: {e}"))?;
    conn.execute(
        "INSERT INTO research_atrann(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert atrann: {e}"))?;
    Ok(())
}

pub fn get_atrann(conn: &Connection, symbol: &str) -> Result<Option<AnnualizedAtrSnapshot>, String> {
    let _ = create_research_tables_v20(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_atrann WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_atrann: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_atrann: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_atrann: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_ddhist(conn: &Connection, symbol: &str, snap: &DrawdownHistorySnapshot) -> Result<(), String> {
    let _ = create_research_tables_v20(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ddhist json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ddhist(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ddhist: {e}"))?;
    Ok(())
}

pub fn get_ddhist(conn: &Connection, symbol: &str) -> Result<Option<DrawdownHistorySnapshot>, String> {
    let _ = create_research_tables_v20(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_ddhist WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ddhist: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_ddhist: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ddhist: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_priceperf(conn: &Connection, symbol: &str, snap: &PricePerformanceSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v20(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("priceperf json: {e}"))?;
    conn.execute(
        "INSERT INTO research_priceperf(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert priceperf: {e}"))?;
    Ok(())
}

pub fn get_priceperf(conn: &Connection, symbol: &str) -> Result<Option<PricePerformanceSnapshot>, String> {
    let _ = create_research_tables_v20(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_priceperf WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_priceperf: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_priceperf: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_priceperf: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-128 Round 21 schema v21 + wrappers ──

pub fn create_research_tables_v21(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v20(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_betarank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_betarank_updated ON research_betarank(updated_at);

        CREATE TABLE IF NOT EXISTS research_pegrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_pegrank_updated ON research_pegrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_fhighlow (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_fhighlow_updated ON research_fhighlow(updated_at);

        CREATE TABLE IF NOT EXISTS research_rvcone (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_rvcone_updated ON research_rvcone(updated_at);

        CREATE TABLE IF NOT EXISTS research_calpb (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_calpb_updated ON research_calpb(updated_at);",
    ).map_err(|e| format!("create v21 tables: {e}"))?;
    Ok(())
}

pub fn upsert_betarank(conn: &Connection, symbol: &str, snap: &BetaRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v21(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("betarank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_betarank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert betarank: {e}"))?;
    Ok(())
}

pub fn get_betarank(conn: &Connection, symbol: &str) -> Result<Option<BetaRankSnapshot>, String> {
    let _ = create_research_tables_v21(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_betarank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_betarank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_betarank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_betarank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_pegrank(conn: &Connection, symbol: &str, snap: &PegRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v21(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("pegrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_pegrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert pegrank: {e}"))?;
    Ok(())
}

pub fn get_pegrank(conn: &Connection, symbol: &str) -> Result<Option<PegRankSnapshot>, String> {
    let _ = create_research_tables_v21(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_pegrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_pegrank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_pegrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_pegrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_fhighlow(conn: &Connection, symbol: &str, snap: &FiftyTwoWeekHighLowSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v21(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("fhighlow json: {e}"))?;
    conn.execute(
        "INSERT INTO research_fhighlow(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert fhighlow: {e}"))?;
    Ok(())
}

pub fn get_fhighlow(conn: &Connection, symbol: &str) -> Result<Option<FiftyTwoWeekHighLowSnapshot>, String> {
    let _ = create_research_tables_v21(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_fhighlow WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_fhighlow: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_fhighlow: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_fhighlow: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_rvcone(conn: &Connection, symbol: &str, snap: &RealizedVolConeSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v21(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("rvcone json: {e}"))?;
    conn.execute(
        "INSERT INTO research_rvcone(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rvcone: {e}"))?;
    Ok(())
}

pub fn get_rvcone(conn: &Connection, symbol: &str) -> Result<Option<RealizedVolConeSnapshot>, String> {
    let _ = create_research_tables_v21(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_rvcone WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_rvcone: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_rvcone: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_rvcone: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_calpb(conn: &Connection, symbol: &str, snap: &CalendarPeriodBreakdownSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v21(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("calpb json: {e}"))?;
    conn.execute(
        "INSERT INTO research_calpb(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert calpb: {e}"))?;
    Ok(())
}

pub fn get_calpb(conn: &Connection, symbol: &str) -> Result<Option<CalendarPeriodBreakdownSnapshot>, String> {
    let _ = create_research_tables_v21(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_calpb WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_calpb: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_calpb: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_calpb: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-129 Round 22 schema v22 + wrappers ──

pub fn create_research_tables_v22(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v21(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_retskew (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_retskew_updated ON research_retskew(updated_at);

        CREATE TABLE IF NOT EXISTS research_retkurt (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_retkurt_updated ON research_retkurt(updated_at);

        CREATE TABLE IF NOT EXISTS research_tailr (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_tailr_updated ON research_tailr(updated_at);

        CREATE TABLE IF NOT EXISTS research_runlen (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_runlen_updated ON research_runlen(updated_at);

        CREATE TABLE IF NOT EXISTS research_dayrange (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_dayrange_updated ON research_dayrange(updated_at);",
    ).map_err(|e| format!("create v22 tables: {e}"))?;
    Ok(())
}

pub fn upsert_retskew(conn: &Connection, symbol: &str, snap: &ReturnSkewnessSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v22(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("retskew json: {e}"))?;
    conn.execute(
        "INSERT INTO research_retskew(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert retskew: {e}"))?;
    Ok(())
}

pub fn get_retskew(conn: &Connection, symbol: &str) -> Result<Option<ReturnSkewnessSnapshot>, String> {
    let _ = create_research_tables_v22(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_retskew WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_retskew: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_retskew: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_retskew: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_retkurt(conn: &Connection, symbol: &str, snap: &ReturnKurtosisSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v22(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("retkurt json: {e}"))?;
    conn.execute(
        "INSERT INTO research_retkurt(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert retkurt: {e}"))?;
    Ok(())
}

pub fn get_retkurt(conn: &Connection, symbol: &str) -> Result<Option<ReturnKurtosisSnapshot>, String> {
    let _ = create_research_tables_v22(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_retkurt WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_retkurt: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_retkurt: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_retkurt: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_tailr(conn: &Connection, symbol: &str, snap: &TailRatioSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v22(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("tailr json: {e}"))?;
    conn.execute(
        "INSERT INTO research_tailr(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert tailr: {e}"))?;
    Ok(())
}

pub fn get_tailr(conn: &Connection, symbol: &str) -> Result<Option<TailRatioSnapshot>, String> {
    let _ = create_research_tables_v22(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_tailr WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_tailr: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_tailr: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_tailr: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_runlen(conn: &Connection, symbol: &str, snap: &RunLengthSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v22(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("runlen json: {e}"))?;
    conn.execute(
        "INSERT INTO research_runlen(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert runlen: {e}"))?;
    Ok(())
}

pub fn get_runlen(conn: &Connection, symbol: &str) -> Result<Option<RunLengthSnapshot>, String> {
    let _ = create_research_tables_v22(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_runlen WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_runlen: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_runlen: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_runlen: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_dayrange(conn: &Connection, symbol: &str, snap: &DailyRangeSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v22(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("dayrange json: {e}"))?;
    conn.execute(
        "INSERT INTO research_dayrange(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert dayrange: {e}"))?;
    Ok(())
}

pub fn get_dayrange(conn: &Connection, symbol: &str) -> Result<Option<DailyRangeSnapshot>, String> {
    let _ = create_research_tables_v22(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_dayrange WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_dayrange: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_dayrange: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_dayrange: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-130 Web article ingestion (JSON-blob-per-symbol, schema v23) ──
//
// Agent-supplied web research articles. When the research packet's
// "Return Path" footer asks an AI agent to emit a fenced
// `===TYPHOON_INGEST===` block of article objects, the INGEST_RESEARCH
// command parses that block and merges the articles into the
// `research_web_articles` cache. LAN sync then distributes the
// ingested corpus to peer terminals.

/// One web research article captured from an AI agent's reply.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WebArticle {
    pub title: String,
    pub url: String,
    pub source: String,        // publication / domain
    pub published_at: String,  // ISO-8601 preferred, any string tolerated
    pub summary: String,
    pub agent_used: String,    // "claude" | "gemini" | "chatgpt" | free-form
    pub ingested_at: i64,      // unix seconds
}

/// Per-symbol bag of ingested web articles. JSON-blob-per-symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IngestedArticlesSnapshot {
    pub symbol: String,
    pub articles: Vec<WebArticle>,
}

/// Max articles retained per symbol (FIFO drop by ingested_at).
pub const INGESTED_ARTICLES_MAX: usize = 50;

pub fn create_research_tables_v23(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v22(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_web_articles (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_web_articles_updated ON research_web_articles(updated_at);",
    ).map_err(|e| format!("create v23 tables: {e}"))?;
    Ok(())
}

pub fn upsert_ingested_articles(
    conn: &Connection,
    symbol: &str,
    snap: &IngestedArticlesSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v23(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ingested articles json: {e}"))?;
    conn.execute(
        "INSERT INTO research_web_articles(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ingested articles: {e}"))?;
    Ok(())
}

pub fn get_ingested_articles(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<IngestedArticlesSnapshot>, String> {
    let _ = create_research_tables_v23(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_web_articles WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ingested_articles: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_ingested_articles: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ingested_articles: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

/// Merge new articles into the symbol's existing bag.
///
/// Dedupe by URL (case-insensitive). On conflict the newer entry wins
/// (articles with a larger `ingested_at` replace older ones). After
/// merging, the bag is trimmed to the latest `INGESTED_ARTICLES_MAX`
/// articles by `ingested_at` (most-recent first, FIFO drop of oldest).
/// Returns `(added_count, total_count)`.
pub fn append_ingested_articles(
    conn: &Connection,
    symbol: &str,
    incoming: Vec<WebArticle>,
) -> Result<(usize, usize), String> {
    let _ = create_research_tables_v23(conn);
    let mut existing = get_ingested_articles(conn, symbol)?
        .unwrap_or_else(|| IngestedArticlesSnapshot { symbol: symbol.to_uppercase(), articles: Vec::new() });

    let before = existing.articles.len();

    for mut art in incoming {
        if art.url.trim().is_empty() { continue; }
        if art.ingested_at == 0 { art.ingested_at = now_ts(); }
        let key = art.url.trim().to_lowercase();
        if let Some(pos) = existing.articles.iter().position(|a| a.url.trim().to_lowercase() == key) {
            if art.ingested_at >= existing.articles[pos].ingested_at {
                existing.articles[pos] = art;
            }
        } else {
            existing.articles.push(art);
        }
    }

    existing.articles.sort_by(|a, b| b.ingested_at.cmp(&a.ingested_at));
    if existing.articles.len() > INGESTED_ARTICLES_MAX {
        existing.articles.truncate(INGESTED_ARTICLES_MAX);
    }
    let after = existing.articles.len();
    let added = after.saturating_sub(before);

    upsert_ingested_articles(conn, symbol, &existing)?;
    Ok((added, after))
}

/// Parse one or more fenced `===TYPHOON_INGEST===` blocks out of an
/// AI agent reply and return them grouped by uppercase symbol.
///
/// Block format (the footer appended to research packets asks agents
/// to emit exactly this):
///
/// ```text
/// ===TYPHOON_INGEST===
/// [
///   {"symbol": "AAPL", "title": "...", "url": "...", "source": "...",
///    "published_at": "2026-04-15", "summary": "...", "agent": "claude"},
///   ...
/// ]
/// ===END_INGEST===
/// ```
///
/// The parser is lenient: it accepts `published` / `date` as aliases
/// for `published_at`, `agent` for `agent_used`, and silently skips
/// entries with no `url` or no `symbol`. It also tolerates surrounding
/// ```json fences and surrounding whitespace. The `ingested_at` field
/// is always set to the current timestamp at parse time.
pub fn parse_ingest_block(text: &str) -> Vec<(String, Vec<WebArticle>)> {
    let mut out: std::collections::BTreeMap<String, Vec<WebArticle>> = std::collections::BTreeMap::new();
    let now = now_ts();

    let mut rest = text;
    loop {
        let start = match rest.find("===TYPHOON_INGEST===") { Some(i) => i, None => break };
        let after_start = &rest[start + "===TYPHOON_INGEST===".len()..];
        let end_idx = match after_start.find("===END_INGEST===") { Some(i) => i, None => after_start.len() };
        let mut block = after_start[..end_idx].trim().to_string();

        // Strip ```json / ``` fences if present.
        if block.starts_with("```") {
            if let Some(nl) = block.find('\n') {
                block = block[nl + 1..].to_string();
            }
        }
        if block.ends_with("```") {
            let cut = block.len() - 3;
            block = block[..cut].trim_end().to_string();
        }

        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&block) {
            if let Some(arr) = v.as_array() {
                for item in arr {
                    let obj = match item.as_object() { Some(o) => o, None => continue };
                    let symbol = obj.get("symbol").and_then(|s| s.as_str()).unwrap_or("").trim().to_uppercase();
                    if symbol.is_empty() { continue; }
                    let url = obj.get("url").and_then(|s| s.as_str()).unwrap_or("").trim().to_string();
                    if url.is_empty() { continue; }
                    let title = obj.get("title").and_then(|s| s.as_str()).unwrap_or("").to_string();
                    let source = obj.get("source").and_then(|s| s.as_str()).unwrap_or("").to_string();
                    let published_at = obj.get("published_at").and_then(|s| s.as_str())
                        .or_else(|| obj.get("published").and_then(|s| s.as_str()))
                        .or_else(|| obj.get("date").and_then(|s| s.as_str()))
                        .unwrap_or("").to_string();
                    let summary = obj.get("summary").and_then(|s| s.as_str()).unwrap_or("").to_string();
                    let agent_used = obj.get("agent_used").and_then(|s| s.as_str())
                        .or_else(|| obj.get("agent").and_then(|s| s.as_str()))
                        .unwrap_or("").to_string();
                    out.entry(symbol).or_default().push(WebArticle {
                        title, url, source, published_at, summary, agent_used, ingested_at: now,
                    });
                }
            }
        }

        rest = &after_start[end_idx..];
        if rest.is_empty() { break; }
        if let Some(skip) = rest.find("===END_INGEST===") {
            rest = &rest[skip + "===END_INGEST===".len()..];
        } else {
            break;
        }
    }

    out.into_iter().collect()
}

// ── ADR-131 Godel Parity Round 23 schema + helpers ────────────────────────

pub fn create_research_tables_v24(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v23(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_autocor (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_autocor_updated ON research_autocor(updated_at);

        CREATE TABLE IF NOT EXISTS research_hurst (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_hurst_updated ON research_hurst(updated_at);

        CREATE TABLE IF NOT EXISTS research_hitrate (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_hitrate_updated ON research_hitrate(updated_at);

        CREATE TABLE IF NOT EXISTS research_glasym (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_glasym_updated ON research_glasym(updated_at);

        CREATE TABLE IF NOT EXISTS research_volratio (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_volratio_updated ON research_volratio(updated_at);",
    ).map_err(|e| format!("create v24 tables: {e}"))?;
    Ok(())
}

pub fn upsert_autocor(conn: &Connection, symbol: &str, snap: &AutocorrelationSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v24(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("autocor json: {e}"))?;
    conn.execute(
        "INSERT INTO research_autocor(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert autocor: {e}"))?;
    Ok(())
}

pub fn get_autocor(conn: &Connection, symbol: &str) -> Result<Option<AutocorrelationSnapshot>, String> {
    let _ = create_research_tables_v24(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_autocor WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_autocor: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_autocor: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_autocor: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_hurst(conn: &Connection, symbol: &str, snap: &HurstSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v24(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("hurst json: {e}"))?;
    conn.execute(
        "INSERT INTO research_hurst(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert hurst: {e}"))?;
    Ok(())
}

pub fn get_hurst(conn: &Connection, symbol: &str) -> Result<Option<HurstSnapshot>, String> {
    let _ = create_research_tables_v24(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_hurst WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_hurst: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_hurst: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_hurst: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_hitrate(conn: &Connection, symbol: &str, snap: &HitRateSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v24(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("hitrate json: {e}"))?;
    conn.execute(
        "INSERT INTO research_hitrate(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert hitrate: {e}"))?;
    Ok(())
}

pub fn get_hitrate(conn: &Connection, symbol: &str) -> Result<Option<HitRateSnapshot>, String> {
    let _ = create_research_tables_v24(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_hitrate WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_hitrate: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_hitrate: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_hitrate: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_glasym(conn: &Connection, symbol: &str, snap: &GainLossAsymmetrySnapshot) -> Result<(), String> {
    let _ = create_research_tables_v24(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("glasym json: {e}"))?;
    conn.execute(
        "INSERT INTO research_glasym(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert glasym: {e}"))?;
    Ok(())
}

pub fn get_glasym(conn: &Connection, symbol: &str) -> Result<Option<GainLossAsymmetrySnapshot>, String> {
    let _ = create_research_tables_v24(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_glasym WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_glasym: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_glasym: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_glasym: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_volratio(conn: &Connection, symbol: &str, snap: &VolumeRatioSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v24(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("volratio json: {e}"))?;
    conn.execute(
        "INSERT INTO research_volratio(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert volratio: {e}"))?;
    Ok(())
}

pub fn get_volratio(conn: &Connection, symbol: &str) -> Result<Option<VolumeRatioSnapshot>, String> {
    let _ = create_research_tables_v24(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_volratio WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_volratio: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_volratio: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_volratio: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

/// Whole-table scan of `research_divg`. Used by DVDRANK.
pub fn get_all_divg(conn: &Connection) -> Result<Vec<DivgSnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_divg")
        .map_err(|e| format!("prepare get_all_divg: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_divg: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<DivgSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_earm`. Used by EARMRANK.
pub fn get_all_earm(conn: &Connection) -> Result<Vec<EarmSnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_earm")
        .map_err(|e| format!("prepare get_all_earm: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_earm: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<EarmSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_updm`. Used by UPDGRANK.
pub fn get_all_updm(conn: &Connection) -> Result<Vec<UpdmSnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_updm")
        .map_err(|e| format!("prepare get_all_updm: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_updm: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<UpdmSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// One short-interest history observation for a symbol.
/// Stored as a compact per-symbol time series and fed by fundamentals scrapes
/// plus explicit short-interest fetches when available.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ShortInterestHistoryPoint {
    pub as_of: String, // YYYY-MM-DD
    pub short_percent_of_float: f64,
    pub short_ratio: f64,
    pub shares_outstanding: f64,
}

fn normalize_short_interest_history_date(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.len() >= 10 {
        let candidate = &trimmed[..10];
        if chrono::NaiveDate::parse_from_str(candidate, "%Y-%m-%d").is_ok() {
            return Some(candidate.to_string());
        }
    }
    None
}

fn same_short_interest_point(a: &ShortInterestHistoryPoint, b: &ShortInterestHistoryPoint) -> bool {
    (a.short_percent_of_float - b.short_percent_of_float).abs() < 1e-9
        && (a.short_ratio - b.short_ratio).abs() < 1e-9
        && (a.shares_outstanding - b.shares_outstanding).abs() < 1e-6
}

fn merge_short_interest_history_rows(
    existing: &[ShortInterestHistoryPoint],
    new_rows: &[ShortInterestHistoryPoint],
) -> Vec<ShortInterestHistoryPoint> {
    let mut by_date: std::collections::BTreeMap<String, ShortInterestHistoryPoint> =
        std::collections::BTreeMap::new();

    for row in existing.iter().chain(new_rows.iter()) {
        let as_of = match normalize_short_interest_history_date(&row.as_of) {
            Some(v) => v,
            None => continue,
        };
        if !row.short_percent_of_float.is_finite() || row.short_percent_of_float < 0.0 {
            continue;
        }
        let normalized = ShortInterestHistoryPoint {
            as_of: as_of.clone(),
            short_percent_of_float: row.short_percent_of_float,
            short_ratio: if row.short_ratio.is_finite() && row.short_ratio >= 0.0 {
                row.short_ratio
            } else {
                0.0
            },
            shares_outstanding: if row.shares_outstanding.is_finite()
                && row.shares_outstanding >= 0.0
            {
                row.shares_outstanding
            } else {
                0.0
            },
        };
        by_date.insert(as_of, normalized);
    }

    let mut compacted: Vec<ShortInterestHistoryPoint> = Vec::new();
    for row in by_date.into_values() {
        if compacted
            .last()
            .map(|prev| same_short_interest_point(prev, &row))
            .unwrap_or(false)
        {
            continue;
        }
        compacted.push(row);
    }

    let keep_from = compacted.len().saturating_sub(256);
    compacted.into_iter().skip(keep_from).collect()
}

fn create_short_interest_history_table(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_short_interest_history (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_short_interest_history_updated
            ON research_short_interest_history(updated_at);",
    )
    .map_err(|e| format!("create short_interest_history table: {e}"))?;
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
    create_short_interest_history_table(conn)?;
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
        params![symbol.to_uppercase(), json, chrono::Utc::now().timestamp()],
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
    create_short_interest_history_table(conn)?;
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
