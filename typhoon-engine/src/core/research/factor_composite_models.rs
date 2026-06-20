use super::*;

// ── compute fns ──

fn median_f64(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if v.is_empty() {
        return 0.0;
    }
    let mid = v.len() / 2;
    if v.len() % 2 == 0 {
        (v[mid - 1] + v[mid]) / 2.0
    } else {
        v[mid]
    }
}

/// Score a "lower is better" multiple vs a peer median.
/// ratio ≤ median × 0.5 → 100; ratio ≥ median × 2.0 → 0; linear in between.
fn score_multiple_lower_better(value: f64, median: f64) -> f64 {
    if !value.is_finite() || value <= 0.0 || !median.is_finite() || median <= 0.0 {
        return 0.0;
    }
    let ratio = value / median;
    if ratio <= 0.5 {
        100.0
    } else if ratio >= 2.0 {
        0.0
    } else {
        (100.0 * (2.0 - ratio) / 1.5).clamp(0.0, 100.0)
    }
}

/// Score a "higher is better" yield vs a peer median.
/// yield ≥ median × 1.5 → 100; yield ≤ median × 0.5 → 0; linear in between.
fn score_yield_higher_better(value: f64, median: f64) -> f64 {
    if !value.is_finite() || !median.is_finite() || median <= 0.0 {
        return 0.0;
    }
    let ratio = value / median;
    if ratio >= 1.5 {
        100.0
    } else if ratio <= 0.5 {
        0.0
    } else {
        (100.0 * (ratio - 0.5) / 1.0).clamp(0.0, 100.0)
    }
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
    let peer_pe: Vec<f64> = peer_fundamentals
        .iter()
        .filter_map(|p| p.pe_ratio)
        .filter(|v| *v > 0.0 && v.is_finite())
        .collect();
    let peer_fpe: Vec<f64> = peer_fundamentals
        .iter()
        .filter_map(|p| p.forward_pe)
        .filter(|v| *v > 0.0 && v.is_finite())
        .collect();
    let peer_pb: Vec<f64> = peer_fundamentals
        .iter()
        .filter_map(|p| p.price_to_book)
        .filter(|v| *v > 0.0 && v.is_finite())
        .collect();
    let peer_ps: Vec<f64> = peer_fundamentals
        .iter()
        .filter_map(|p| p.price_to_sales)
        .filter(|v| *v > 0.0 && v.is_finite())
        .collect();
    let peer_evebitda: Vec<f64> = peer_fundamentals
        .iter()
        .filter_map(|p| p.ev_to_ebitda)
        .filter(|v| *v > 0.0 && v.is_finite())
        .collect();

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
            score,
            weight: w,
            contribution: score * w / 100.0,
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
            score,
            weight: w,
            contribution: score * w / 100.0,
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
            score,
            weight: w,
            contribution: score * w / 100.0,
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
            score,
            weight: w,
            contribution: score * w / 100.0,
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
            score,
            weight: w,
            contribution: score * w / 100.0,
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
            score,
            weight: w,
            contribution: score * w / 100.0,
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
            note: "need at least one valuation metric vs a non-empty sector peer median"
                .to_string(),
            ..Default::default()
        };
    }

    let composite = (weighted_sum / total_weight).clamp(0.0, 100.0);
    let label = if composite >= 80.0 {
        "DEEP_VALUE"
    } else if composite >= 65.0 {
        "VALUE"
    } else if composite >= 45.0 {
        "FAIR"
    } else if composite >= 30.0 {
        "EXPENSIVE"
    } else {
        "PREMIUM"
    };

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
                score,
                weight: w,
                contribution: score * w / 100.0,
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
                value: format!(
                    "{} op {:.1}% ({})",
                    m.quality_label, m.latest_operating_margin_pct, m.overall_trend_label
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

    // LEV — weight 20. Map solvency_summary label to a score + debt/ebitda.
    if let Some(lv) = leverage {
        if !lv.solvency_summary.is_empty() {
            leverage_summary = lv.solvency_summary.clone();
            debt_to_ebitda = if lv.ebitda_ttm > 0.0 {
                lv.total_debt / lv.ebitda_ttm
            } else {
                0.0
            };
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
        return QualitySnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            quality_label: "NO_DATA".to_string(),
            note: "need at least one of PTFS / MARGINS / ACRL / LEV cached".to_string(),
            ..Default::default()
        };
    }

    let composite = (weighted_sum / total_weight).clamp(0.0, 100.0);
    let label = if composite >= 80.0 {
        "HIGH_QUALITY"
    } else if composite >= 65.0 {
        "QUALITY"
    } else if composite >= 45.0 {
        "AVERAGE"
    } else if composite >= 30.0 {
        "POOR"
    } else {
        "WEAK"
    };

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
            let score = if v.preferred_estimate_pct <= 10.0 {
                0.0
            } else if v.preferred_estimate_pct <= 30.0 {
                (v.preferred_estimate_pct - 10.0) / 20.0 * 50.0
            } else if v.preferred_estimate_pct <= 60.0 {
                50.0 + (v.preferred_estimate_pct - 30.0) / 30.0 * 50.0
            } else {
                100.0
            };
            let w = 25.0;
            components.push(FactorComponent {
                name: "Realized Vol".to_string(),
                value: format!("{:.1}% ({})", v.preferred_estimate_pct, v.preferred_label),
                score,
                weight: w,
                contribution: score * w / 100.0,
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
                let score = (dist / 1.0 * 60.0).min(100.0); // |β-1|=1 → 60; |β-1|>=1.67 → 100
                let w = 20.0;
                components.push(FactorComponent {
                    name: "Beta 1Y".to_string(),
                    value: format!("β {:.2}", one_y.beta),
                    score,
                    weight: w,
                    contribution: score * w / 100.0,
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
                score,
                weight: w,
                contribution: score * w / 100.0,
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
                value: format!(
                    "{:.1}% float, {:.1} DTC ({})",
                    s.short_percent_of_float, s.days_to_cover, s.squeeze_risk_label
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

    // ALTZ — weight 25. DISTRESS zone = highest risk.
    if let Some(a) = altman {
        if a.zone != "INSUFFICIENT_DATA" && !a.zone.is_empty() {
            altman_z = a.z_score;
            altman_zone = a.zone.clone();
            let score = match a.zone.as_str() {
                "SAFE" => 10.0,
                "GRAY" => 55.0,
                "DISTRESS" => {
                    distressed = true;
                    95.0
                }
                _ => 50.0,
            };
            let w = 25.0;
            components.push(FactorComponent {
                name: "Altman Z".to_string(),
                value: format!("Z {:.2} ({})", a.z_score, a.zone),
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
        return RiskSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            risk_label: "NO_DATA".to_string(),
            note: "need at least one of VOLE / BETA / LIQ / SHRT / ALTZ cached".to_string(),
            ..Default::default()
        };
    }

    let composite = (weighted_sum / total_weight).clamp(0.0, 100.0);
    let label = if distressed {
        "DISTRESSED"
    } else if composite >= 75.0 {
        "HIGH_RISK"
    } else if composite >= 55.0 {
        "ELEVATED"
    } else if composite >= 35.0 {
        "MODERATE"
    } else {
        "LOW_RISK"
    };

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
    let mut filtered: Vec<&InsiderTrade> = trades
        .iter()
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
        per_insider
            .entry(t.reporting_name.clone())
            .or_default()
            .push(*t);
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
            if tt.starts_with("P") || tt.contains("PURCHASE") {
                return "BUY";
            }
            if tt.starts_with("S") || tt.contains("SALE") {
                return "SELL";
            }
            if t.acquisition_disposition.to_uppercase() == "A" {
                return "BUY";
            }
            if t.acquisition_disposition.to_uppercase() == "D" {
                return "SELL";
            }
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
            if d == cur_dir {
                cur_run += 1;
            } else {
                cur_run = 1;
                cur_dir = d;
            }
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
            if d == "BUY" {
                net_value += t.value_usd;
                net_shares += t.shares;
                has_buy = true;
            } else if d == "SELL" {
                net_value -= t.value_usd;
                net_shares -= t.shares;
                has_sell = true;
            }
        }

        let mixed = has_buy && has_sell;
        let row_dir = if mixed {
            "MIXED".to_string()
        } else if has_buy {
            "BUY".to_string()
        } else if has_sell {
            "SELL".to_string()
        } else {
            "OTHER".to_string()
        };

        if row_dir == "BUY" && longest_run >= 2 {
            buy_streak_count += 1;
        }
        if row_dir == "SELL" && longest_run >= 2 {
            sell_streak_count += 1;
        }
        if longest_dir == "BUY" && longest_run > longest_buy_streak {
            longest_buy_streak = longest_run;
        }
        if longest_dir == "SELL" && longest_run > longest_sell_streak {
            longest_sell_streak = longest_run;
        }
        if row_dir == "BUY" {
            net_buy_value_usd += net_value.max(0.0);
        }
        if row_dir == "SELL" {
            net_sell_value_usd += (-net_value).max(0.0);
        }

        let first_date = ts
            .first()
            .map(|t| t.transaction_date.clone())
            .unwrap_or_default();
        let latest_date = ts
            .last()
            .map(|t| t.transaction_date.clone())
            .unwrap_or_default();

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
        let ka = match a.streak_direction.as_str() {
            "BUY" => 0,
            "SELL" => 1,
            "MIXED" => 2,
            _ => 3,
        };
        let kb = match b.streak_direction.as_str() {
            "BUY" => 0,
            "SELL" => 1,
            "MIXED" => 2,
            _ => 3,
        };
        ka.cmp(&kb)
            .then(b.consecutive_events.cmp(&a.consecutive_events))
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
    let mut sb = 0;
    let mut b = 0;
    let mut h = 0;
    let mut s = 0;
    let mut ss = 0;
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
    let bull_ratio = if total_recs > 0 {
        (sb + b) as f64 / total_recs as f64
    } else {
        0.0
    };

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
