use super::*;

// ── Round 14 compute fns ───────────────────────────────────────────

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
            if d.cagr_3y_pct >= 10.0 {
                score = (score + 15.0).min(100.0);
            } else if d.cagr_3y_pct >= 5.0 {
                score = (score + 7.0).min(100.0);
            } else if d.cagr_3y_pct < -5.0 {
                score = (score - 15.0).max(0.0);
            }
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
        if mom_has && !earm_has {
            "SPECULATIVE"
        } else {
            "GARP"
        }
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
        if t.transaction_date.is_empty() {
            continue;
        }
        let d = parse_yyyy_mm_dd_to_days(&t.transaction_date);
        if let (Some(cut), Some(dd)) = (cutoff_opt, d) {
            if dd < cut {
                continue;
            }
        }
        trade_count += 1;
        if !t.reporting_name.is_empty() {
            names.insert(t.reporting_name.clone());
        }
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

    let label = if composite >= 80.0 {
        "STRONG_BUY"
    } else if composite >= 60.0 {
        "BUY"
    } else if composite >= 40.0 {
        "NEUTRAL"
    } else if composite >= 20.0 {
        "SELL"
    } else {
        "STRONG_SELL"
    };

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
    } else if adx_value >= 40.0 {
        100.0
    } else if adx_value >= 25.0 {
        60.0 + (adx_value - 25.0) / 15.0 * 40.0
    } else if adx_value >= 15.0 {
        30.0 + (adx_value - 15.0) / 10.0 * 30.0
    } else {
        (adx_value / 15.0 * 30.0).max(0.0)
    };

    // Volatility score: low vol = high score.
    let vol_score: f64 = if realized_vol_pct <= 0.0 {
        50.0
    } else if realized_vol_pct < 15.0 {
        90.0
    } else if realized_vol_pct < 25.0 {
        70.0
    } else if realized_vol_pct < 40.0 {
        50.0
    } else if realized_vol_pct < 60.0 {
        30.0
    } else {
        10.0
    };

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
        if slice.is_empty() {
            return 0.0;
        }
        let mut s = 0.0;
        let mut k = 0;
        for b in slice {
            if b.volume > 0.0 {
                s += b.volume;
                k += 1;
            }
        }
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

    let vol_trend = if avg_20 > 0.0 {
        (avg_5 / avg_20 - 1.0) * 100.0
    } else {
        0.0
    };

    // Percentile rank of current vs last 60 bars (excluding itself).
    let sample_end = (1 + 60).min(n);
    let sample: Vec<f64> = bars_newest_first[1..sample_end]
        .iter()
        .map(|b| b.volume)
        .collect();
    let percentile = if sample.is_empty() {
        50.0
    } else {
        let count_below = sample.iter().filter(|v| **v < current).count();
        count_below as f64 / sample.len() as f64 * 100.0
    };

    let activity = if r20 >= 3.0 {
        "EXTREME"
    } else if r20 >= 2.0 {
        "HIGH"
    } else if r20 >= 1.5 {
        "ELEVATED"
    } else if r20 >= 0.5 {
        "NORMAL"
    } else {
        "LOW"
    };

    let direction = if n >= 2 {
        let prior_close = bars_newest_first[1].close;
        let now_close = bars_newest_first[0].close;
        if prior_close > 0.0 && now_close > prior_close * 1.005 {
            "BULLISH"
        } else if prior_close > 0.0 && now_close < prior_close * 0.995 {
            "BEARISH"
        } else {
            "NEUTRAL"
        }
    } else {
        "NEUTRAL"
    };

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
        if inc.revenue <= 0.0 {
            continue;
        }
        let g = if inc.gross_profit != 0.0 {
            inc.gross_profit / inc.revenue * 100.0
        } else {
            0.0
        };
        let o = if inc.operating_income != 0.0 {
            inc.operating_income / inc.revenue * 100.0
        } else {
            0.0
        };
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
        if chg >= 1.0 {
            "EXPANDING"
        } else if chg <= -1.0 {
            "CONTRACTING"
        } else {
            "STABLE"
        }
    };
    let gross_trend = label_trend(g_chg);
    let op_trend = label_trend(o_chg);
    let net_trend = label_trend(n_chg);

    // Overall — majority rule across the three.
    let mut exp_n = 0;
    let mut con_n = 0;
    for t in [gross_trend, op_trend, net_trend] {
        if t == "EXPANDING" {
            exp_n += 1;
        } else if t == "CONTRACTING" {
            con_n += 1;
        }
    }
    let overall = if exp_n >= 2 {
        "EXPANDING"
    } else if con_n >= 2 {
        "CONTRACTING"
    } else {
        "STABLE"
    };

    let quality = if latest.operating_margin_pct >= 20.0 {
        "HIGH"
    } else if latest.operating_margin_pct >= 8.0 {
        "MEDIUM"
    } else {
        "LOW"
    };

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
