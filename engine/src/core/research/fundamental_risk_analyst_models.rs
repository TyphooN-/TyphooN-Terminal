use super::*;

// Fundamental quality, solvency, volatility, EPS-beat, and price-target compute functions

/// ALTZ — classic Altman Z-score for public manufacturers.
/// Z = 1.2(WC/TA) + 1.4(RE/TA) + 3.3(EBIT/TA) + 0.6(MVE/TL) + 1.0(Sales/TA)
pub fn compute_altman_z_snapshot(
    symbol: &str,
    as_of: &str,
    statements: &FinancialStatements,
    market_value_equity: f64,
) -> AltmanZSnapshot {
    let bal = statements
        .balance_annual
        .first()
        .or_else(|| statements.balance_quarterly.first());
    let inc = statements
        .income_annual
        .first()
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
        AltmanComponent {
            name: "A: WC/TA".into(),
            ratio: a,
            coefficient: 1.2,
            contribution: 1.2 * a,
            note: "liquidity".into(),
        },
        AltmanComponent {
            name: "B: RE/TA".into(),
            ratio: b,
            coefficient: 1.4,
            contribution: 1.4 * b,
            note: "cumulative profitability".into(),
        },
        AltmanComponent {
            name: "C: EBIT/TA".into(),
            ratio: c,
            coefficient: 3.3,
            contribution: 3.3 * c,
            note: "operating leverage".into(),
        },
        AltmanComponent {
            name: "D: MVE/TL".into(),
            ratio: d,
            coefficient: 0.6,
            contribution: 0.6 * d,
            note: if mve > 0.0 {
                "solvency"
            } else {
                "no market cap"
            }
            .into(),
        },
        AltmanComponent {
            name: "E: Sales/TA".into(),
            ratio: e,
            coefficient: 1.0,
            contribution: 1.0 * e,
            note: "asset turnover".into(),
        },
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
        "no market cap from Fundamentals — D component is zero, zone reports as INSUFFICIENT_DATA"
            .to_string()
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
    } else {
        0.0
    };
    let cr_prev = if bal_prev.total_current_liabilities > 0.0 {
        bal_prev.total_current_assets / bal_prev.total_current_liabilities
    } else {
        0.0
    };
    let shares_cur = inc_cur.weighted_shares_out;
    let shares_prev = inc_prev.weighted_shares_out;

    let gm_cur = if inc_cur.revenue > 0.0 {
        inc_cur.gross_profit / inc_cur.revenue
    } else {
        0.0
    };
    let gm_prev = if inc_prev.revenue > 0.0 {
        inc_prev.gross_profit / inc_prev.revenue
    } else {
        0.0
    };
    let at_cur = if ta_cur > 0.0 {
        inc_cur.revenue / ta_cur
    } else {
        0.0
    };
    let at_prev = if ta_prev > 0.0 {
        inc_prev.revenue / ta_prev
    } else {
        0.0
    };

    let mut checks: Vec<PiotroskiCheck> = Vec::new();

    // Profitability (4)
    checks.push(PiotroskiCheck {
        category: "Profitability".into(),
        name: "Positive Net Income".into(),
        passed: ni > 0.0,
        value_current: ni,
        value_prior: 0.0,
        note: String::new(),
    });
    checks.push(PiotroskiCheck {
        category: "Profitability".into(),
        name: "Positive OCF".into(),
        passed: cfo > 0.0,
        value_current: cfo,
        value_prior: 0.0,
        note: String::new(),
    });
    checks.push(PiotroskiCheck {
        category: "Profitability".into(),
        name: "ROA ↑".into(),
        passed: roa_cur > roa_prev,
        value_current: roa_cur,
        value_prior: roa_prev,
        note: String::new(),
    });
    checks.push(PiotroskiCheck {
        category: "Profitability".into(),
        name: "OCF > NI (accrual)".into(),
        passed: cfo > ni,
        value_current: cfo,
        value_prior: ni,
        note: format!("accrual = {:.0}", accrual_proxy),
    });

    // Leverage / Liquidity (3)
    checks.push(PiotroskiCheck {
        category: "Leverage/Liquidity".into(),
        name: "LT Debt / Assets ↓".into(),
        passed: ltd_cur < ltd_prev,
        value_current: ltd_cur,
        value_prior: ltd_prev,
        note: String::new(),
    });
    checks.push(PiotroskiCheck {
        category: "Leverage/Liquidity".into(),
        name: "Current Ratio ↑".into(),
        passed: cr_cur > cr_prev,
        value_current: cr_cur,
        value_prior: cr_prev,
        note: String::new(),
    });
    checks.push(PiotroskiCheck {
        category: "Leverage/Liquidity".into(),
        name: "No new share issue".into(),
        passed: shares_cur <= shares_prev * 1.005, // 0.5% tolerance for option grants
        value_current: shares_cur,
        value_prior: shares_prev,
        note: String::new(),
    });

    // Operating Efficiency (2)
    checks.push(PiotroskiCheck {
        category: "Operating Efficiency".into(),
        name: "Gross Margin ↑".into(),
        passed: gm_cur > gm_prev,
        value_current: gm_cur,
        value_prior: gm_prev,
        note: String::new(),
    });
    checks.push(PiotroskiCheck {
        category: "Operating Efficiency".into(),
        name: "Asset Turnover ↑".into(),
        passed: at_cur > at_prev,
        value_current: at_cur,
        value_prior: at_prev,
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
    let valid: Vec<&HistoricalPriceRow> = tail
        .iter()
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
    let log_ret_cc: Vec<f64> = valid
        .windows(2)
        .map(|w| (w[1].close / w[0].close).ln())
        .collect();
    let mean_cc: f64 = log_ret_cc.iter().sum::<f64>() / log_ret_cc.len() as f64;
    let var_cc: f64 = log_ret_cc
        .iter()
        .map(|r| (r - mean_cc).powi(2))
        .sum::<f64>()
        / (log_ret_cc.len() - 1).max(1) as f64;
    let ctc_daily = var_cc.sqrt();
    let ctc = ctc_daily * ann.sqrt() * 100.0;

    // Parkinson (range-based).
    // σ² = (1 / (4·ln2·N)) × Σ ln(H/L)²
    let ln2 = 2.0f64.ln();
    let park_sum: f64 = valid
        .iter()
        .filter(|b| b.low > 0.0)
        .map(|b| (b.high / b.low).ln().powi(2))
        .sum();
    let park_var_daily = park_sum / (4.0 * ln2 * valid.len() as f64);
    let park = park_var_daily.sqrt() * ann.sqrt() * 100.0;

    // Garman-Klass.
    // σ² = (1/N) × Σ [0.5·ln(H/L)² − (2·ln2 − 1)·ln(C/O)²]
    let gk_sum: f64 = valid
        .iter()
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
    let rs_sum: f64 = valid
        .iter()
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
    let overnight_rets: Vec<f64> = valid
        .windows(2)
        .map(|w| (w[1].open / w[0].close).ln())
        .collect();
    let on_mean: f64 = overnight_rets.iter().sum::<f64>() / overnight_rets.len().max(1) as f64;
    let on_var: f64 = overnight_rets
        .iter()
        .map(|r| (r - on_mean).powi(2))
        .sum::<f64>()
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
        efficiency_vs_close: if ctc > 0.0 {
            ctc / vol.max(0.0001)
        } else {
            1.0
        },
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
    let direction = if newest > 0.0 {
        1i32
    } else if newest < 0.0 {
        -1i32
    } else {
        0i32
    };
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
            if run_beat > longest_beat {
                longest_beat = run_beat;
            }
            run_miss = 0;
        } else if r.surprise < 0.0 {
            run_miss += 1;
            if run_miss > longest_miss {
                longest_miss = run_miss;
            }
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
    } else {
        0.0
    };
    let spread_pct = if current_price > 0.0 {
        (target.target_high - target.target_low) / current_price * 100.0
    } else {
        0.0
    };

    let implied_median = if current_price > 0.0 && target.target_median > 0.0 {
        (target.target_median - current_price) / current_price * 100.0
    } else {
        0.0
    };
    let implied_mean = if current_price > 0.0 && target.target_mean > 0.0 {
        (target.target_mean - current_price) / current_price * 100.0
    } else {
        0.0
    };
    let upside_high = if current_price > 0.0 && target.target_high > 0.0 {
        (target.target_high - current_price) / current_price * 100.0
    } else {
        0.0
    };
    let downside_low = if current_price > 0.0 && target.target_low > 0.0 {
        (target.target_low - current_price) / current_price * 100.0
    } else {
        0.0
    };

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
