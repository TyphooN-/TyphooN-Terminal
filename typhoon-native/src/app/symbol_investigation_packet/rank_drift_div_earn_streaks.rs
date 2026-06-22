use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub(super) fn write_rank_drift_div_earn_streaks(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    if let Ok(Some(dr)) = rx::get_dvdrank(ctx.conn, &sym_upper) {
        if dr.rank_label != "NO_DATA"
            && dr.rank_label != "INSUFFICIENT_DATA"
            && !dr.rank_label.is_empty()
        {
            let _ = writeln!(
                p,
                "### Dividend Growth Rank — DVDRANK ({}, as of {})",
                dr.rank_label, dr.as_of
            );
            let _ = writeln!(
                p,
                "- 3y CAGR {:+.2}% · {} consecutive growth yrs · trend {} · rank {}/{} · pct {:.0}",
                dr.cagr_3y_pct,
                dr.consecutive_growth_years,
                dr.trend_label,
                dr.rank_position,
                dr.peers_considered + 1,
                dr.percentile_rank
            );
            let _ = writeln!(
                p,
                "- Sector {} median / p25 / p75 CAGR: {:+.2}% / {:+.2}% / {:+.2}%",
                dr.sector,
                dr.sector_median_cagr_pct,
                dr.sector_p25_cagr_pct,
                dr.sector_p75_cagr_pct
            );
            if !dr.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", dr.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(er)) = rx::get_earmrank(ctx.conn, &sym_upper) {
        if er.rank_label != "NO_DATA"
            && er.rank_label != "INSUFFICIENT_DATA"
            && !er.rank_label.is_empty()
        {
            let _ = writeln!(
                p,
                "### Earnings Momentum Rank — EARMRANK ({}, as of {})",
                er.rank_label, er.as_of
            );
            let _ = writeln!(
                p,
                "- Composite {:.2} · momentum {} · rank {}/{} · pct {:.0}",
                er.composite_score,
                er.momentum_label,
                er.rank_position,
                er.peers_considered + 1,
                er.percentile_rank
            );
            let _ = writeln!(
                p,
                "- Sector {} median / p25 / p75: {:.2} / {:.2} / {:.2}",
                er.sector, er.sector_median_score, er.sector_p25, er.sector_p75
            );
            if !er.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", er.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(ur)) = rx::get_updgrank(ctx.conn, &sym_upper) {
        if ur.rank_label != "NO_DATA"
            && ur.rank_label != "INSUFFICIENT_DATA"
            && !ur.rank_label.is_empty()
        {
            let _ = writeln!(
                p,
                "### Upgrade/Downgrade Rank — UPDGRANK ({}, as of {})",
                ur.rank_label, ur.as_of
            );
            let _ = writeln!(
                p,
                "- Net 90d {:+} · bias {} · rank {}/{} · pct {:.0}",
                ur.net_90d,
                ur.bias_label,
                ur.rank_position,
                ur.peers_considered + 1,
                ur.percentile_rank
            );
            let _ = writeln!(
                p,
                "- Sector {} median / p25 / p75 net: {:+.1} / {:+.1} / {:+.1}",
                ur.sector, ur.sector_median_net_90d, ur.sector_p25_net_90d, ur.sector_p75_net_90d
            );
            if !ur.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ur.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(gy)) = rx::get_gy(ctx.conn, &sym_upper) {
        if gy.gap_label != "INSUFFICIENT_DATA" && !gy.gap_label.is_empty() {
            let _ = writeln!(
                p,
                "### Gap Yearly — GY ({}, as of {})",
                gy.gap_label, gy.as_of
            );
            let _ = writeln!(
                p,
                "- {} bars · {} gaps · avg |gap| {:.2}%",
                gy.bars_used, gy.gaps_total, gy.avg_abs_gap_pct
            );
            let _ = writeln!(
                p,
                "- Gaps up ≥2/5/10%: {} / {} / {} · gaps down ≥2/5/10%: {} / {} / {}",
                gy.gaps_up_2pct,
                gy.gaps_up_5pct,
                gy.gaps_up_10pct,
                gy.gaps_down_2pct,
                gy.gaps_down_5pct,
                gy.gaps_down_10pct
            );
            if !gy.largest_up_gap_date.is_empty() {
                let _ = writeln!(
                    p,
                    "- Largest up {:+.2}% on {} · largest down {:+.2}% on {}",
                    gy.largest_up_gap_pct,
                    gy.largest_up_gap_date,
                    gy.largest_down_gap_pct,
                    gy.largest_down_gap_date
                );
            }
            if !gy.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", gy.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(ds)) = rx::get_des(ctx.conn, &sym_upper) {
        if ds.streak_label != "INSUFFICIENT_DATA" && !ds.streak_label.is_empty() {
            let _ = writeln!(
                p,
                "### Daily Event Streak — DES ({}, as of {})",
                ds.streak_label, ds.as_of
            );
            let _ = writeln!(
                p,
                "- {} bars · up/down/flat {} / {} / {} · up rate {:.0}%",
                ds.bars_used, ds.up_days, ds.down_days, ds.flat_days, ds.up_day_rate_pct
            );
            let _ = writeln!(
                p,
                "- Current {} × {} · longest up/down {} / {}",
                ds.current_streak_len,
                ds.current_streak_type,
                ds.longest_up_streak,
                ds.longest_down_streak
            );
            let _ = writeln!(
                p,
                "- Avg up move {:+.2}% · avg down move {:+.2}%",
                ds.avg_up_move_pct, ds.avg_down_move_pct
            );
            if !ds.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ds.note);
            }
            let _ = writeln!(p);
        }
    }

    // ── Research section ──
}
