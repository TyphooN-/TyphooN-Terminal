use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub fn write_symbol_fundamental_risk_sections(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    // LEV debt leverage & coverage
    if let Ok(Some(lv)) = rx::get_leverage(ctx.conn, &sym_upper) {
        if !lv.ratios.is_empty() {
            let _ = writeln!(p, "### Leverage & Coverage (as of {})", lv.as_of);
            let _ = writeln!(
                p,
                "- {} · Total Debt ${:.0}M · Net Debt ${:.0}M · EBITDA TTM ${:.0}M · Interest TTM ${:.0}M",
                lv.solvency_summary,
                lv.total_debt / 1e6,
                lv.net_debt / 1e6,
                lv.ebitda_ttm / 1e6,
                lv.interest_expense_ttm / 1e6
            );
            let _ = writeln!(p, "| Ratio | Value | Peer Median | Signal |");
            let _ = writeln!(p, "|---|---|---|---|");
            for r in lv.ratios.iter() {
                let pm = if r.peer_median > 0.0 {
                    format!("{:.2}", r.peer_median)
                } else {
                    "—".to_string()
                };
                let _ = writeln!(p, "| {} | {:.2} | {} | {} |", r.name, r.value, pm, r.signal);
            }
            if !lv.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", lv.note);
            }
            let _ = writeln!(p);
        }
    }

    // ACRL earnings quality (NI vs FCF)
    if let Ok(Some(ac)) = rx::get_accruals(ctx.conn, &sym_upper) {
        if !ac.periods.is_empty() {
            let _ = writeln!(p, "### Earnings Quality / Accruals (as of {})", ac.as_of);
            let _ = writeln!(
                p,
                "- {} · TTM NI ${:.0}M · TTM FCF ${:.0}M · TTM cash conv {:.1}% · avg {:.1}%",
                ac.trend_label,
                ac.ttm_net_income / 1e6,
                ac.ttm_free_cash_flow / 1e6,
                ac.ttm_cash_conversion_pct,
                ac.avg_cash_conversion_pct
            );
            let _ = writeln!(
                p,
                "| Period | Date | NI ($M) | FCF ($M) | Cash Conv % | Quality |"
            );
            let _ = writeln!(p, "|---|---|---|---|---|---|");
            for q in ac.periods.iter().take(8) {
                let _ = writeln!(
                    p,
                    "| {} | {} | {:.0} | {:.0} | {:.1}% | {} |",
                    q.period,
                    q.date,
                    q.net_income / 1e6,
                    q.free_cash_flow / 1e6,
                    q.cash_conversion_pct,
                    q.quality_label
                );
            }
            if !ac.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ac.note);
            }
            let _ = writeln!(p);
        }
    }

    // RVOL realized volatility cone
    if let Ok(Some(rv)) = rx::get_realized_vol(ctx.conn, &sym_upper) {
        if !rv.windows.is_empty() {
            let _ = writeln!(p, "### Realized Volatility Cone (as of {})", rv.as_of);
            let iv_str = if rv.current_atm_iv_pct > 0.0 {
                format!(
                    "IV {:.1}% · gap {:+.1}%",
                    rv.current_atm_iv_pct, rv.iv_rv_gap_pct
                )
            } else {
                "no IV reference".to_string()
            };
            let _ = writeln!(
                p,
                "- Last ${:.2} · {} · regime: {}",
                rv.last_close, iv_str, rv.regime_label
            );
            let _ = writeln!(p, "| Window | Days | Realized Vol % | Percentile |");
            let _ = writeln!(p, "|---|---|---|---|");
            for w in rv.windows.iter() {
                let _ = writeln!(
                    p,
                    "| {} | {} | {:.2} | {:.0}% |",
                    w.label, w.trading_days, w.realized_vol_pct, w.percentile
                );
            }
            if !rv.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", rv.note);
            }
            let _ = writeln!(p);
        }
    }

    // FCFY FCF yield & dividend sustainability
    if let Ok(Some(fy)) = rx::get_fcf_yield(ctx.conn, &sym_upper) {
        if fy.ttm_free_cash_flow != 0.0 || !fy.periods.is_empty() {
            let _ = writeln!(
                p,
                "### FCF Yield & Dividend Sustainability (as of {})",
                fy.as_of
            );
            let _ = writeln!(
                p,
                "- {} · FCF yield {:.2}% · div yield {:.2}% · payout-from-FCF {:.1}% · payout-from-NI {:.1}% · 5Y FCF CAGR {:+.1}%",
                fy.sustainability_label,
                fy.ttm_fcf_yield_pct,
                fy.ttm_dividend_yield_pct,
                fy.ttm_payout_from_fcf_pct,
                fy.ttm_payout_from_ni_pct,
                fy.fcf_cagr_5y_pct
            );
            if !fy.periods.is_empty() {
                let _ = writeln!(
                    p,
                    "| Period | Date | FCF ($M) | Div Paid ($M) | Payout-FCF % | FCF Yield % |"
                );
                let _ = writeln!(p, "|---|---|---|---|---|---|");
                for per in fy.periods.iter().take(6) {
                    let _ = writeln!(
                        p,
                        "| {} | {} | {:.0} | {:.0} | {:.1}% | {:.2}% |",
                        per.period,
                        per.date,
                        per.free_cash_flow / 1e6,
                        per.dividends_paid / 1e6,
                        per.payout_from_fcf_pct,
                        per.fcf_yield_pct
                    );
                }
            }
            if !fy.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", fy.note);
            }
            let _ = writeln!(p);
        }
    }

    // SHRT short interest & days-to-cover
    if let Ok(Some(si)) = rx::get_short_interest(ctx.conn, &sym_upper) {
        if si.shares_float > 0.0 || si.short_percent_of_float > 0.0 {
            let _ = writeln!(p, "### Short Interest & Days-to-Cover (as of {})", si.as_of);
            let _ = writeln!(
                p,
                "- Squeeze risk: {} · Short {:.2}% of float · DTC {:.1} days",
                si.squeeze_risk_label, si.short_percent_of_float, si.days_to_cover
            );
            let _ = writeln!(
                p,
                "- Float {:.0}M · Shares out {:.0}M · Short {:.0}M · Avg vol 20d {:.0}K",
                si.shares_float / 1e6,
                si.shares_outstanding / 1e6,
                si.short_shares / 1e6,
                si.avg_daily_volume_20d / 1e3
            );
            if si.short_ratio_reported > 0.0 {
                let _ = writeln!(
                    p,
                    "- Reported short ratio: {:.2} · utilization proxy: {:.1}%",
                    si.short_ratio_reported, si.utilization_proxy_pct
                );
            }
            if !si.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", si.note);
            }
            let _ = writeln!(p);
        }
    }

    // ALTZ Altman Z-score
    if let Ok(Some(az)) = rx::get_altman_z(ctx.conn, &sym_upper) {
        if !az.components.is_empty() {
            let _ = writeln!(p, "### Altman Z-Score (as of {})", az.as_of);
            let _ = writeln!(p, "- Z = {:.2} · zone: {}", az.z_score, az.zone);
            let _ = writeln!(
                p,
                "- WC ${:.0}M · RE ${:.0}M · EBIT ${:.0}M · MVE ${:.0}M · Sales ${:.0}M · TA ${:.0}M · TL ${:.0}M",
                az.working_capital / 1e6,
                az.retained_earnings / 1e6,
                az.ebit / 1e6,
                az.market_value_equity / 1e6,
                az.sales / 1e6,
                az.total_assets / 1e6,
                az.total_liabilities / 1e6
            );
            let _ = writeln!(p, "| Component | Ratio | Coeff | Contribution |");
            let _ = writeln!(p, "|---|---|---|---|");
            for c in az.components.iter() {
                let _ = writeln!(
                    p,
                    "| {} | {:.3} | {:.1} | {:.3} |",
                    c.name, c.ratio, c.coefficient, c.contribution
                );
            }
            if !az.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", az.note);
            }
            let _ = writeln!(p);
        }
    }

    // PTFS Piotroski F-score
    if let Ok(Some(pf)) = rx::get_piotroski(ctx.conn, &sym_upper) {
        if !pf.checks.is_empty() {
            let _ = writeln!(p, "### Piotroski F-Score (as of {})", pf.as_of);
            let _ = writeln!(
                p,
                "- F-Score {}/9 · {} · {} vs {}",
                pf.f_score, pf.strength_label, pf.current_period, pf.prior_period
            );
            let _ = writeln!(
                p,
                "- Profitability {}/4 · Leverage/Liquidity {}/3 · Efficiency {}/2",
                pf.profitability_score, pf.leverage_score, pf.efficiency_score
            );
            let _ = writeln!(p, "| Category | Check | Passed | Current | Prior |");
            let _ = writeln!(p, "|---|---|---|---|---|");
            for c in pf.checks.iter() {
                let pass = if c.passed { "PASS" } else { "FAIL" };
                let _ = writeln!(
                    p,
                    "| {} | {} | {} | {:.2} | {:.2} |",
                    c.category, c.name, pass, c.value_current, c.value_prior
                );
            }
            if !pf.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", pf.note);
            }
            let _ = writeln!(p);
        }
    }

    // VOLE OHLC volatility estimators
    if let Ok(Some(ov)) = rx::get_ohlc_vol(ctx.conn, &sym_upper) {
        if !ov.estimators.is_empty() {
            let _ = writeln!(p, "### OHLC Volatility Estimators (as of {})", ov.as_of);
            let _ = writeln!(
                p,
                "- Preferred {} = {:.2}% · {} trading days",
                ov.preferred_label, ov.preferred_estimate_pct, ov.trading_days
            );
            let _ = writeln!(p, "| Estimator | Annualized % | Efficiency vs CtC |");
            let _ = writeln!(p, "|---|---|---|");
            for e in ov.estimators.iter() {
                let _ = writeln!(
                    p,
                    "| {} | {:.2} | {:.2}x |",
                    e.name, e.annualized_vol_pct, e.efficiency_vs_close
                );
            }
            if !ov.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ov.note);
            }
            let _ = writeln!(p);
        }
    }

    // EPSB EPS beat streak & surprise
    if let Ok(Some(eb)) = rx::get_eps_beat(ctx.conn, &sym_upper) {
        if eb.total_reports > 0 {
            let _ = writeln!(p, "### EPS Beat Streak & Surprise (as of {})", eb.as_of);
            let _ = writeln!(
                p,
                "- Bias: {} · Trend: {} · Beat rate {:.0}% · Current streak {:+}",
                eb.bias_label, eb.trend_label, eb.beat_rate_pct, eb.current_streak
            );
            let _ = writeln!(
                p,
                "- Reports {} · Beats {} · Misses {} · Inlines {} · Longest beat {} · Longest miss {}",
                eb.total_reports,
                eb.beats,
                eb.misses,
                eb.inlines,
                eb.longest_beat_streak,
                eb.longest_miss_streak
            );
            let _ = writeln!(
                p,
                "- Avg surprise {:+.2}% · Median {:+.2}% · Recent-4 {:+.2}%",
                eb.avg_surprise_pct, eb.median_surprise_pct, eb.recent_avg_surprise_pct
            );
            if !eb.latest_date.is_empty() {
                let _ = writeln!(
                    p,
                    "- Latest: {} ({:+.2}%)",
                    eb.latest_date, eb.latest_surprise_pct
                );
            }
            if !eb.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", eb.note);
            }
            let _ = writeln!(p);
        }
    }

    // PTD price target dispersion & implied return
    if let Ok(Some(pd)) = rx::get_price_target_dispersion(ctx.conn, &sym_upper) {
        if pd.num_analysts > 0 {
            let _ = writeln!(p, "### Price Target Dispersion (as of {})", pd.as_of);
            let _ = writeln!(
                p,
                "- {} · {} analysts · current ${:.2}",
                pd.consensus_label, pd.num_analysts, pd.current_price
            );
            let _ = writeln!(
                p,
                "- Target high ${:.2} · low ${:.2} · mean ${:.2} · median ${:.2}",
                pd.target_high, pd.target_low, pd.target_mean, pd.target_median
            );
            let _ = writeln!(
                p,
                "- Dispersion {:.1}% · spread-vs-current {:.1}% · implied return (median) {:+.1}% · (mean) {:+.1}%",
                pd.dispersion_pct,
                pd.spread_pct,
                pd.implied_return_median_pct,
                pd.implied_return_mean_pct
            );
            let _ = writeln!(
                p,
                "- Upside-to-high {:+.1}% · Downside-to-low {:+.1}%",
                pd.upside_to_high_pct, pd.downside_to_low_pct
            );
            if !pd.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", pd.note);
            }
            let _ = writeln!(p);
        }
    }
}
