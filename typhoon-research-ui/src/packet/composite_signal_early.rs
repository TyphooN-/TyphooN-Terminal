use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub fn write_composite_signal_early(ctx: &SymbolResearchContext, p: &mut String, sym_upper: &str) {
    // MNGR insider activity bias
    if let Ok(Some(ia)) = rx::get_insider_activity(ctx.conn, &sym_upper) {
        if ia.total_trades > 0 {
            let _ = writeln!(
                p,
                "### Insider Activity Bias (as of {}, window {}d)",
                ia.as_of, ia.window_days
            );
            let _ = writeln!(
                p,
                "- {} · conviction: {} · {} trades from {} insiders",
                ia.bias_label, ia.conviction_label, ia.total_trades, ia.unique_insiders
            );
            let _ = writeln!(
                p,
                "- Buys {} · Sells {} · Other {}",
                ia.buy_count, ia.sell_count, ia.other_count
            );
            let _ = writeln!(
                p,
                "- Gross buy ${:.0} · gross sell ${:.0} · net ${:+.0} · buy/sell ratio {:.2}",
                ia.gross_buy_value_usd,
                ia.gross_sell_value_usd,
                ia.net_value_usd,
                ia.buy_sell_ratio
            );
            let _ = writeln!(
                p,
                "- Net shares {:+.0} · latest trade {}",
                ia.net_shares, ia.latest_trade_date
            );
            if !ia.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ia.note);
            }
            let _ = writeln!(p);
        }
    }

    // DIVG dividend growth
    if let Ok(Some(dg)) = rx::get_divg(ctx.conn, &sym_upper) {
        if dg.total_payments > 0 {
            let _ = writeln!(p, "### Dividend Growth (as of {})", dg.as_of);
            let _ = writeln!(
                p,
                "- {} · {} years covered · {} total payments",
                dg.trend_label, dg.years_covered, dg.total_payments
            );
            let _ = writeln!(
                p,
                "- Latest ${:.4} on {} · annualized ${:.2}",
                dg.latest_amount, dg.latest_payment_date, dg.annualized_dividend
            );
            let _ = writeln!(
                p,
                "- 1Y {:+.2}% · 3Y CAGR {:+.2}% · 5Y CAGR {:+.2}% · consistency {:.0}% · consecutive growth years {}",
                dg.cagr_1y_pct,
                dg.cagr_3y_pct,
                dg.cagr_5y_pct,
                dg.consistency_score_pct,
                dg.consecutive_growth_years
            );
            let tail_n = dg.annual_rows.len().min(6);
            if tail_n > 0 {
                let rows_tail = &dg.annual_rows[dg.annual_rows.len() - tail_n..];
                let parts: Vec<String> = rows_tail
                    .iter()
                    .map(|r| format!("{} ${:.2} ({:+.1}%)", r.year, r.total_amount, r.growth_pct))
                    .collect();
                let _ = writeln!(p, "- Recent years: {}", parts.join(" · "));
            }
            if !dg.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", dg.note);
            }
            let _ = writeln!(p);
        }
    }

    // EARM earnings momentum trend
    if let Ok(Some(em)) = rx::get_earm(ctx.conn, &sym_upper) {
        if em.quarters_used >= 5 {
            let _ = writeln!(p, "### Earnings Momentum (as of {})", em.as_of);
            let _ = writeln!(
                p,
                "- {} · composite {:.0}/100 · {} quarters used",
                em.momentum_label, em.composite_score, em.quarters_used
            );
            let _ = writeln!(
                p,
                "- Revenue: recent 4Q {:+.2}% · prior 4Q {:+.2}% · acceleration {:+.2}%",
                em.recent_revenue_growth_pct,
                em.prior_revenue_growth_pct,
                em.revenue_acceleration_pct
            );
            let _ = writeln!(
                p,
                "- EPS surprise: recent 4Q {:+.2}% · prior 4Q {:+.2}% · acceleration {:+.2}%",
                em.recent_eps_surprise_pct,
                em.prior_eps_surprise_pct,
                em.eps_surprise_acceleration_pct
            );
            let tail_n = em.quarters.len().min(4);
            if tail_n > 0 {
                let parts: Vec<String> = em
                    .quarters
                    .iter()
                    .take(tail_n)
                    .map(|q| {
                        format!(
                            "{} rev {:+.1}% surp {:+.1}%",
                            q.period, q.revenue_yoy_pct, q.eps_surprise_pct
                        )
                    })
                    .collect();
                let _ = writeln!(p, "- Recent quarters: {}", parts.join(" · "));
            }
            if !em.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", em.note);
            }
            let _ = writeln!(p);
        }
    }

    // SECTR sector rotation strength
    if let Ok(Some(sr)) = rx::get_sector_rotation(ctx.conn, &sym_upper) {
        if sr.sectors_total > 0 {
            let _ = writeln!(p, "### Sector Rotation Strength (as of {})", sr.as_of);
            let _ = writeln!(
                p,
                "- {} · sector: {} · rank {}/{}",
                sr.strength_label, sr.symbol_sector, sr.sector_rank, sr.sectors_total
            );
            let _ = writeln!(
                p,
                "- Symbol sector change {:+.2}% · avg {:+.2}% · median {:+.2}% · relative strength {:+.2}%",
                sr.symbol_sector_change_pct,
                sr.avg_sector_change_pct,
                sr.median_sector_change_pct,
                sr.relative_strength_pct
            );
            let _ = writeln!(
                p,
                "- Market breadth {:.0}% positive · strongest {} {:+.2}% · weakest {} {:+.2}%",
                sr.breadth_pct,
                sr.strongest_sector,
                sr.strongest_sector_pct,
                sr.weakest_sector,
                sr.weakest_sector_pct
            );
            if !sr.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", sr.note);
            }
            let _ = writeln!(p);
        }
    }

    // UPDM upgrade/downgrade momentum
    if let Ok(Some(um)) = rx::get_updm(ctx.conn, &sym_upper) {
        if um.total_actions > 0 {
            let _ = writeln!(p, "### Upgrade/Downgrade Momentum (as of {})", um.as_of);
            let _ = writeln!(
                p,
                "- {} · trend: {} · {} total actions",
                um.bias_label, um.trend_label, um.total_actions
            );
            let _ = writeln!(
                p,
                "- 30d: +{} / -{} (net {:+}) · 90d: +{} / -{} (net {:+}) · 180d: +{} / -{} (net {:+})",
                um.upgrades_30d,
                um.downgrades_30d,
                um.net_30d,
                um.upgrades_90d,
                um.downgrades_90d,
                um.net_90d,
                um.upgrades_180d,
                um.downgrades_180d,
                um.net_180d
            );
            let _ = writeln!(
                p,
                "- 90d initiations {} · maintains {}",
                um.initiations_90d, um.maintains_90d
            );
            if !um.latest_date.is_empty() {
                let _ = writeln!(
                    p,
                    "- Latest: {} — {} — {} ({})",
                    um.latest_date, um.latest_firm, um.latest_action, um.latest_to_grade
                );
            }
            if !um.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", um.note);
            }
            let _ = writeln!(p);
        }
    }

    // MOM 12-1 month momentum score
    if let Ok(Some(mom)) = rx::get_momentum(ctx.conn, &sym_upper) {
        if mom.bars_used > 0 {
            let _ = writeln!(p, "### Momentum 12-1 (as of {})", mom.as_of);
            let _ = writeln!(
                p,
                "- Regime: {} · trend: {} · composite {:.1}/100 · {} bars used",
                mom.regime_label, mom.trend_label, mom.composite_score, mom.bars_used
            );
            let _ = writeln!(
                p,
                "- Returns: 1m {:+.2}% · 3m {:+.2}% · 6m {:+.2}% · 12m {:+.2}% · 12-1 {:+.2}%",
                mom.return_1m_pct,
                mom.return_3m_pct,
                mom.return_6m_pct,
                mom.return_12m_pct,
                mom.return_12_1_pct
            );
            let _ = writeln!(
                p,
                "- Annualized vol {:.2}% · vol-adjusted score {:+.3}",
                mom.vol_annualized_pct, mom.vol_adjusted_score
            );
            if !mom.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", mom.note);
            }
            let _ = writeln!(p);
        }
    }

    // LIQ liquidity profile
    if let Ok(Some(lq)) = rx::get_liquidity(ctx.conn, &sym_upper) {
        if lq.window_days > 0 {
            let _ = writeln!(
                p,
                "### Liquidity Profile (window {}d, as of {})",
                lq.window_days, lq.as_of
            );
            let _ = writeln!(
                p,
                "- Tier: {} · avg $/day {:.0} · median $/day {:.0}",
                lq.liquidity_tier, lq.avg_daily_dollar_volume, lq.median_daily_dollar_volume
            );
            let _ = writeln!(
                p,
                "- Avg shares/day {:.0} · daily turnover {:.3}%",
                lq.avg_daily_share_volume, lq.daily_turnover_pct
            );
            let _ = writeln!(
                p,
                "- Amihud illiquidity ×1e6 {:.4} · ATR {:.2}% · spread proxy {:.3}%",
                lq.amihud_illiquidity, lq.avg_true_range_pct, lq.spread_proxy_pct
            );
            if !lq.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", lq.note);
            }
            let _ = writeln!(p);
        }
    }

    // BREAK breakout proximity
    if let Ok(Some(bk)) = rx::get_breakout(ctx.conn, &sym_upper) {
        if bk.current_price > 0.0 {
            let _ = writeln!(p, "### Breakout Proximity (as of {})", bk.as_of);
            let _ = writeln!(
                p,
                "- {} · setup: {} · last {:.2}",
                bk.breakout_label, bk.setup_label, bk.current_price
            );
            let _ = writeln!(
                p,
                "- 20d [{:.2} .. {:.2}] · 60d [{:.2} .. {:.2}] · 52w [{:.2} .. {:.2}]",
                bk.low_20d, bk.high_20d, bk.low_60d, bk.high_60d, bk.low_52w, bk.high_52w
            );
            let _ = writeln!(
                p,
                "- Dist 52w high {:+.2}% · dist 52w low {:+.2}% · pos in 52w range {:.0}% · consolidation {:.2}%",
                bk.dist_from_52w_high_pct,
                bk.dist_from_52w_low_pct,
                bk.position_in_52w_range_pct,
                bk.consolidation_pct
            );
            if !bk.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", bk.note);
            }
            let _ = writeln!(p);
        }
    }

    // CCRL cash conversion cycle
    if let Ok(Some(cc)) = rx::get_cash_cycle(ctx.conn, &sym_upper) {
        if cc.periods_used > 0 {
            let _ = writeln!(
                p,
                "### Cash Conversion Cycle (latest {}, as of {})",
                cc.latest_period, cc.as_of
            );
            let _ = writeln!(
                p,
                "- {} · trend: {} · CCC {:.1}d · prior {:.1}d · Δ {:+.1}d · 3y avg {:.1}d",
                cc.efficiency_label,
                cc.trend_label,
                cc.ccc_days,
                cc.prior_ccc_days,
                cc.ccc_change_days,
                cc.ccc_3y_avg_days
            );
            let _ = writeln!(
                p,
                "- DSO {:.1}d · DIO {:.1}d · DPO {:.1}d",
                cc.dso_days, cc.dio_days, cc.dpo_days
            );
            if !cc.periods.is_empty() {
                let _ = writeln!(p, "- Per-period:");
                for row in cc.periods.iter().take(8) {
                    let _ = writeln!(
                        p,
                        "  - {}: DSO {:.0} · DIO {:.0} · DPO {:.0} · CCC {:.0}",
                        row.period, row.dso_days, row.dio_days, row.dpo_days, row.ccc_days
                    );
                }
            }
            if !cc.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", cc.note);
            }
            let _ = writeln!(p);
        }
    }

    // CREDIT unified credit score
    if let Ok(Some(cr)) = rx::get_credit(ctx.conn, &sym_upper) {
        if cr.inputs_available > 0 {
            let _ = writeln!(p, "### Credit Score (as of {})", cr.as_of);
            let _ = writeln!(
                p,
                "- {} · {} · composite {:.1}/100 · {}/4 inputs",
                cr.letter_grade, cr.credit_label, cr.composite_score, cr.inputs_available
            );
            let _ = writeln!(
                p,
                "- Altman Z {:.2} ({}) · Piotroski {}/9 ({}) · leverage: {} · accruals: {}",
                cr.altman_z,
                cr.altman_zone,
                cr.piotroski_score,
                cr.piotroski_label,
                cr.leverage_summary,
                cr.accruals_trend
            );
            let _ = writeln!(
                p,
                "- TTM cash conversion {:.1}%",
                cr.accruals_ttm_cash_conversion_pct
            );
            if !cr.components.is_empty() {
                let _ = writeln!(p, "- Components:");
                for c in cr.components.iter().take(6) {
                    let _ = writeln!(
                        p,
                        "  - {}: value {} · score {:.1} · weight {:.0}% · contrib {:.1}",
                        c.name, c.value, c.score, c.weight, c.contribution
                    );
                }
            }
            if !cr.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", cr.note);
            }
            let _ = writeln!(p);
        }
    }
}
