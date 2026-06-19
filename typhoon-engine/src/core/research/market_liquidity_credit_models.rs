use super::*;

// Market liquidity, momentum, breakout, cash-cycle, and credit compute functions

/// Pick the daily close closest to (and not after) `target_offset_back` bars
/// from the most recent bar. `bars` is newest-first. Returns None if the
/// offset is out of range.
fn pick_close_offset(bars_newest_first: &[HistoricalPriceRow], offset: usize) -> Option<f64> {
    if offset >= bars_newest_first.len() {
        return None;
    }
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
        if from > 0.0 {
            (to - from) / from * 100.0
        } else {
            0.0
        }
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
    let mean: f64 = if log_rets.is_empty() {
        0.0
    } else {
        log_rets.iter().sum::<f64>() / log_rets.len() as f64
    };
    let var: f64 = if log_rets.len() > 1 {
        log_rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (log_rets.len() - 1) as f64
    } else {
        0.0
    };
    let daily_stdev = var.sqrt();
    let vol_ann_pct = daily_stdev * (252f64).sqrt() * 100.0;

    let vol_adj_score = if vol_ann_pct > 0.0 {
        return_12_1 / vol_ann_pct
    } else {
        0.0
    };

    let composite = (50.0 + vol_adj_score * 20.0 + return_6m * 0.3).clamp(0.0, 100.0);
    let regime = if composite >= 75.0 {
        "STRONG"
    } else if composite >= 40.0 {
        "NEUTRAL"
    } else if composite >= 20.0 {
        "WEAK"
    } else {
        "CRASH"
    };
    let trend = if return_1m > return_3m / 3.0 && return_3m > return_6m / 2.0 {
        "ACCELERATING"
    } else if return_1m < return_3m / 3.0 && return_3m < return_6m / 2.0 {
        "DECELERATING"
    } else {
        "STABLE"
    };

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

    let avg_share = if share_vols.is_empty() {
        0.0
    } else {
        share_vols.iter().sum::<f64>() / share_vols.len() as f64
    };
    let avg_dollar = if dollar_vols.is_empty() {
        0.0
    } else {
        dollar_vols.iter().sum::<f64>() / dollar_vols.len() as f64
    };
    let median = |mut v: Vec<f64>| -> f64 {
        if v.is_empty() {
            return 0.0;
        }
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = v.len() / 2;
        if v.len() % 2 == 0 {
            (v[mid - 1] + v[mid]) / 2.0
        } else {
            v[mid]
        }
    };
    let med_share = median(share_vols.clone());
    let med_dollar = median(dollar_vols.clone());

    let turnover_pct = if shares_outstanding > 0.0 {
        avg_share / shares_outstanding * 100.0
    } else {
        0.0
    };
    let amihud = if amihud_terms.is_empty() {
        0.0
    } else {
        amihud_terms.iter().sum::<f64>() / amihud_terms.len() as f64 * 1.0e6
    };
    let atr_pct = if true_range_pcts.is_empty() {
        0.0
    } else {
        true_range_pcts.iter().sum::<f64>() / true_range_pcts.len() as f64
    };
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
            } else {
                0.0
            }
        } else {
            0.0
        }
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
            if b.high > 0.0 && b.high > hi {
                hi = b.high;
            }
            if b.low > 0.0 && b.low < lo {
                lo = b.low;
            }
        }
        if hi == f64::MIN {
            hi = 0.0;
        }
        if lo == f64::MAX {
            lo = 0.0;
        }
        (hi, lo)
    };

    let (h20, l20) = range_high_low(&bars_newest_first[..20.min(n)]);
    let (h60, l60) = range_high_low(&bars_newest_first[..60.min(n)]);
    let (h52, l52) = range_high_low(&bars_newest_first[..252.min(n)]);

    let pct_from = |target: f64, from: f64| -> f64 {
        if from > 0.0 {
            (target - from) / from * 100.0
        } else {
            0.0
        }
    };

    let dist_52w_high = pct_from(current, h52);
    let dist_52w_low = pct_from(current, l52);
    let dist_20d_high = pct_from(current, h20);
    let dist_60d_high = pct_from(current, h60);

    let pos_in_range = |cur: f64, hi: f64, lo: f64| -> f64 {
        let width = hi - lo;
        if width > 0.0 {
            (cur - lo) / width * 100.0
        } else {
            50.0
        }
    };
    let pos_52w = pos_in_range(current, h52, l52);
    let pos_20d = pos_in_range(current, h20, l20);

    let cons_pct = {
        let mean_close = {
            let mut s = 0.0;
            let mut k = 0;
            for b in &bars_newest_first[..20.min(n)] {
                if b.close > 0.0 {
                    s += b.close;
                    k += 1;
                }
            }
            if k > 0 { s / k as f64 } else { current }
        };
        if mean_close > 0.0 {
            (h20 - l20) / mean_close * 100.0
        } else {
            0.0
        }
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

    let (income, balance, basis) = if !statements.income_annual.is_empty()
        && !statements.balance_annual.is_empty()
    {
        (
            &statements.income_annual,
            &statements.balance_annual,
            "annual",
        )
    } else if !statements.income_quarterly.is_empty() && !statements.balance_quarterly.is_empty() {
        (
            &statements.income_quarterly,
            &statements.balance_quarterly,
            "quarterly",
        )
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
        if inc.revenue <= 0.0 || inc.cost_of_revenue <= 0.0 {
            return None;
        }
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
                value: format!(
                    "{} ({:.0}% cash conv)",
                    ac.trend_label, ac.ttm_cash_conversion_pct
                ),
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
    let letter = if composite >= 90.0 {
        "AAA"
    } else if composite >= 80.0 {
        "AA"
    } else if composite >= 70.0 {
        "A"
    } else if composite >= 60.0 {
        "BBB"
    } else if composite >= 50.0 {
        "BB"
    } else if composite >= 35.0 {
        "B"
    } else {
        "CCC"
    };
    let label = if composite >= 70.0 {
        "INVESTMENT_GRADE"
    } else if composite >= 55.0 {
        "BORDERLINE"
    } else if composite >= 35.0 {
        "SPECULATIVE"
    } else {
        "DISTRESSED"
    };

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
