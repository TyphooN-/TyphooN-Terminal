//! Market/statistical research snapshot computations.

use super::*;

// ── IVOL compute (IV Rank / IV Percentile) ─────────────

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
            note: "no IV history — rank/percentile are placeholders until history accumulates"
                .to_string(),
        };
    }
    let mut vals: Vec<f64> = history
        .iter()
        .map(|o| o.atm_iv_pct)
        .filter(|v| v.is_finite() && *v > 0.0)
        .collect();
    vals.push(current_atm_iv_pct);
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let lo = vals.first().copied().unwrap_or(current_atm_iv_pct);
    let hi = vals.last().copied().unwrap_or(current_atm_iv_pct);
    let rank = if (hi - lo).abs() > 1e-9 {
        ((current_atm_iv_pct - lo) / (hi - lo)) * 100.0
    } else {
        50.0
    };
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
            format!(
                "only {} observations — rank stabilizes around 252",
                history.len()
            )
        } else {
            String::new()
        },
    }
}

// ── SEAG compute (seasonality) ─────────────────────────

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
        if b.adj_close > 0.0 {
            b.adj_close
        } else {
            b.close
        }
    };

    // ── Monthly buckets: group bars by YYYY-MM and compute per-(year, month)
    // simple return from first bar to last bar of that month, then aggregate
    // across years into the 12 buckets.
    use std::collections::BTreeMap;
    let mut per_ym: BTreeMap<(i32, u32), (f64, f64)> = BTreeMap::new(); // (year, month) → (first, last)
    let mut years_seen: std::collections::BTreeSet<i32> = std::collections::BTreeSet::new();
    for b in bars_oldest_first {
        if b.date.len() < 10 {
            continue;
        }
        let year: i32 = match b.date.get(0..4).and_then(|s| s.parse().ok()) {
            Some(y) => y,
            None => continue,
        };
        let month: u32 = match b.date.get(5..7).and_then(|s| s.parse().ok()) {
            Some(m) => m,
            None => continue,
        };
        let p = px(b);
        if p <= 0.0 {
            continue;
        }
        years_seen.insert(year);
        per_ym
            .entry((year, month))
            .and_modify(|e| e.1 = p)
            .or_insert((p, p));
    }

    let month_label = |m: u32| -> &'static str {
        match m {
            1 => "Jan",
            2 => "Feb",
            3 => "Mar",
            4 => "Apr",
            5 => "May",
            6 => "Jun",
            7 => "Jul",
            8 => "Aug",
            9 => "Sep",
            10 => "Oct",
            11 => "Nov",
            12 => "Dec",
            _ => "?",
        }
    };

    let mut months: Vec<SeasonalityMonth> = Vec::new();
    for m in 1u32..=12 {
        let rets: Vec<f64> = per_ym
            .iter()
            .filter_map(|((_y, mm), (first, last))| {
                if *mm == m && *first > 0.0 {
                    Some((last / first - 1.0) * 100.0)
                } else {
                    None
                }
            })
            .collect();
        if rets.is_empty() {
            months.push(SeasonalityMonth {
                month: m,
                label: month_label(m).to_string(),
                ..Default::default()
            });
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
            1 => "Mon",
            2 => "Tue",
            3 => "Wed",
            4 => "Thu",
            5 => "Fri",
            6 => "Sat",
            7 => "Sun",
            _ => "?",
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
        let iso = match h {
            0 => 6,
            1 => 7,
            2 => 1,
            3 => 2,
            4 => 3,
            5 => 4,
            6 => 5,
            _ => 1,
        };
        Some(iso as u32)
    };

    let mut dow_map: BTreeMap<u32, (f64, usize, usize)> = BTreeMap::new(); // dow → (sum_log_ret, pos_count, total)
    for w in bars_oldest_first.windows(2) {
        let p0 = px(&w[0]);
        let p1 = px(&w[1]);
        if p0 <= 0.0 || p1 <= 0.0 {
            continue;
        }
        let r = (p1 / p0).ln();
        if let Some(d) = dow_of(&w[1].date) {
            let entry = dow_map.entry(d).or_insert((0.0, 0, 0));
            entry.0 += r;
            entry.2 += 1;
            if r > 0.0 {
                entry.1 += 1;
            }
        }
    }
    let mut dow_out: Vec<SeasonalityDow> = Vec::new();
    for d in 1u32..=5 {
        if let Some((sum, pos, total)) = dow_map.get(&d).cloned() {
            let mean_pct = if total > 0 {
                (sum / total as f64).exp().ln() * 100.0
            } else {
                0.0
            };
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
        if m.total_years == 0 {
            continue;
        }
        if m.avg_return_pct > best_avg {
            best_avg = m.avg_return_pct;
            best_month = m.label.clone();
        }
        if m.avg_return_pct < worst_avg {
            worst_avg = m.avg_return_pct;
            worst_month = m.label.clone();
        }
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

// ── COR compute (correlation matrix vs peers) ──────────

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
        if b.adj_close > 0.0 {
            b.adj_close
        } else {
            b.close
        }
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
        if peer_bars.len() < 31 {
            continue;
        }
        let ptake = window_days.saturating_add(1).min(peer_bars.len());
        let peer_slice = &peer_bars[peer_bars.len() - ptake..];
        // Build peer logret and intersect dates.
        let mut paired: Vec<(f64, f64)> = Vec::new();
        for w in peer_slice.windows(2) {
            let p0 = px(&w[0]);
            let p1 = px(&w[1]);
            if p0 <= 0.0 || p1 <= 0.0 {
                continue;
            }
            if let Some(s) = sub_map.get(&w[1].date) {
                paired.push((*s, (p1 / p0).ln()));
            }
        }
        if paired.len() < 30 {
            continue;
        }
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
        if c.correlation > hi {
            hi = c.correlation;
            highest_sym = c.peer_symbol.clone();
        }
        if c.correlation < lo {
            lo = c.correlation;
            lowest_sym = c.peer_symbol.clone();
        }
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

// ── TRA compute (total return = price + dividends) ────

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
        if b.adj_close > 0.0 {
            b.adj_close
        } else {
            b.close
        }
    };
    let last_px = px(n - 1);

    // Trailing 12 month dividends by ex_date cutoff.
    let cutoff_ttm = {
        // Naive 12-month cutoff: subtract one from the year component.
        let y: i32 = last_date
            .get(0..4)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let m = last_date.get(5..7).unwrap_or("01");
        let d = last_date.get(8..10).unwrap_or("01");
        format!("{:04}-{}-{}", y - 1, m, d)
    };
    let ttm_divs: f64 = dividends
        .iter()
        .filter(|d| {
            d.ex_date.as_str() > cutoff_ttm.as_str() && d.ex_date.as_str() <= last_date.as_str()
        })
        .map(|d| d.amount)
        .sum();
    let ttm_yield = if last_close > 0.0 {
        ttm_divs / last_close * 100.0
    } else {
        0.0
    };

    let mut windows: Vec<TotalReturnWindow> = Vec::new();
    let push_window = |windows: &mut Vec<TotalReturnWindow>,
                       label: &str,
                       start_idx: usize,
                       trading_days: usize| {
        if start_idx >= n - 1 {
            return;
        }
        let start_px = px(start_idx);
        if start_px <= 0.0 {
            return;
        }
        let start_date = bars_oldest_first[start_idx].date.clone();
        let price_ret = (last_px / start_px - 1.0) * 100.0;
        let window_divs: f64 = dividends
            .iter()
            .filter(|d| {
                d.ex_date.as_str() > start_date.as_str() && d.ex_date.as_str() <= last_date.as_str()
            })
            .map(|d| d.amount)
            .sum();
        let n_divs = dividends
            .iter()
            .filter(|d| {
                d.ex_date.as_str() > start_date.as_str() && d.ex_date.as_str() <= last_date.as_str()
            })
            .count();
        let div_yield = if start_px > 0.0 {
            window_divs / start_px * 100.0
        } else {
            0.0
        };
        let total = price_ret + div_yield;
        let annualized = if trading_days >= 252 {
            let years = trading_days as f64 / 252.0;
            (((total / 100.0) + 1.0).powf(1.0 / years) - 1.0) * 100.0
        } else {
            total
        };
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

    for (label, days) in &[
        ("1M", 21),
        ("3M", 63),
        ("6M", 126),
        ("1Y", 252),
        ("3Y", 756),
        ("5Y", 1260),
    ] {
        if n > *days {
            push_window(&mut windows, label, n - 1 - days, *days);
        }
    }
    // YTD
    let year_prefix = as_of.get(..4).unwrap_or("");
    if !year_prefix.is_empty() {
        if let Some(ytd_start) = bars_oldest_first
            .iter()
            .position(|b| b.date.starts_with(year_prefix))
        {
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

// ── SKEW compute (volatility smile/skew) ───────────────

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
            if c.implied_volatility <= 0.0 {
                continue;
            }
            let k = (c.strike * 100.0).round() as i64;
            map.entry(k)
                .and_modify(|e| e.0 = Some(c.implied_volatility))
                .or_insert((Some(c.implied_volatility), None));
        }
        for p in &ex.puts {
            if p.implied_volatility <= 0.0 {
                continue;
            }
            let k = (p.strike * 100.0).round() as i64;
            map.entry(k)
                .and_modify(|e| e.1 = Some(p.implied_volatility))
                .or_insert((None, Some(p.implied_volatility)));
        }
        let mut points: Vec<SkewPoint> = Vec::new();
        for (k, (cv, pv)) in &map {
            let strike = (*k as f64) / 100.0;
            let moneyness = (strike / u - 1.0) * 100.0;
            let call_iv = cv.map(|v| v * 100.0).unwrap_or(0.0);
            let put_iv = pv.map(|v| v * 100.0).unwrap_or(0.0);
            let combined = match (cv, pv) {
                (Some(a), Some(b)) => (a + b) / 2.0 * 100.0,
                (Some(a), None) => a * 100.0,
                (None, Some(b)) => b * 100.0,
                (None, None) => 0.0,
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
            if d < best_dist {
                best_dist = d;
                atm_idx = i;
            }
        }
        let atm_iv = points[atm_idx].combined_iv_pct;

        // ±10% OTM skew proxy.
        let target_otm_call = u * 1.10;
        let target_otm_put = u * 0.90;
        let mut otm_call_iv = 0.0;
        let mut otm_put_iv = 0.0;
        let mut best_c = f64::INFINITY;
        let mut best_p = f64::INFINITY;
        for p in &points {
            let dc = (p.strike - target_otm_call).abs();
            let dp = (p.strike - target_otm_put).abs();
            if dc < best_c && p.call_iv_pct > 0.0 {
                best_c = dc;
                otm_call_iv = p.call_iv_pct;
            }
            if dp < best_p && p.put_iv_pct > 0.0 {
                best_p = dp;
                otm_put_iv = p.put_iv_pct;
            }
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
