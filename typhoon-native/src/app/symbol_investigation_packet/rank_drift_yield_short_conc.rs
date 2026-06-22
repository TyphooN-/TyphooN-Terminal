use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub(super) fn write_rank_drift_yield_short_conc(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    if let Ok(Some(dyr)) = rx::get_dvdyieldrank(ctx.conn, &sym_upper) {
        if dyr.rank_label != "NO_DATA"
            && dyr.rank_label != "INSUFFICIENT_DATA"
            && !dyr.rank_label.is_empty()
        {
            let _ = writeln!(
                p,
                "### Dividend Yield Rank — DVDYIELDRANK ({}, as of {})",
                dyr.rank_label, dyr.as_of
            );
            let _ = writeln!(
                p,
                "- Yield {:.2}% · rank {}/{} · pct {:.0}",
                dyr.dividend_yield_pct,
                dyr.rank_position,
                dyr.peers_considered + 1,
                dyr.percentile_rank
            );
            let _ = writeln!(
                p,
                "- Sector {} median / p25 / p75 yield: {:.2}% / {:.2}% / {:.2}%",
                dyr.sector,
                dyr.sector_median_yield_pct,
                dyr.sector_p25_yield_pct,
                dyr.sector_p75_yield_pct
            );
            if !dyr.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", dyr.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(sr)) = rx::get_shrank(ctx.conn, &sym_upper) {
        if sr.rank_label != "NO_DATA"
            && sr.rank_label != "INSUFFICIENT_DATA"
            && !sr.rank_label.is_empty()
        {
            let _ = writeln!(
                p,
                "### Short Interest Rank — SHRANK ({}, as of {})",
                sr.rank_label, sr.as_of
            );
            let _ = writeln!(
                p,
                "- Short {:.2}% of float · rank {}/{} · pct {:.0} (risk-inverted: higher = safer)",
                sr.short_pct_of_float,
                sr.rank_position,
                sr.peers_considered + 1,
                sr.percentile_rank
            );
            let _ = writeln!(
                p,
                "- Sector {} median / p25 / p75 short: {:.2}% / {:.2}% / {:.2}%",
                sr.sector,
                sr.sector_median_short_pct,
                sr.sector_p25_short_pct,
                sr.sector_p75_short_pct
            );
            if !sr.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", sr.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(sd)) = rx::get_shortrank_delta(ctx.conn, &sym_upper) {
        if sd.rank_label != "NO_DATA"
            && sd.rank_label != "INSUFFICIENT_DATA"
            && !sd.rank_label.is_empty()
        {
            let _ = writeln!(
                p,
                "### Short Interest Trend Rank — SHORTRANK_DELTA ({} / {}, as of {})",
                sd.subject_trend_label, sd.rank_label, sd.as_of
            );
            let _ = writeln!(
                p,
                "- {}d window {} → {} · short {:.2}% from {:.2}% ({:+.2} pts) · rank {}/{} · pct {:.0}",
                sd.lookback_days,
                sd.history_start_date,
                sd.history_end_date,
                sd.latest_short_pct_of_float,
                sd.prior_short_pct_of_float,
                sd.delta_short_pct_points,
                sd.rank_position,
                sd.peers_considered + 1,
                sd.percentile_rank
            );
            let _ = writeln!(
                p,
                "- Sector {} median / p25 / p75 delta: {:+.2} / {:+.2} / {:+.2} pts",
                sd.sector,
                sd.sector_median_delta_pct_pts,
                sd.sector_p25_delta_pct_pts,
                sd.sector_p75_delta_pct_pts
            );
            if !sd.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", sd.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(ic)) = rx::get_insiderconc(ctx.conn, &sym_upper) {
        if ic.rank_label != "NO_DATA"
            && ic.rank_label != "INSUFFICIENT_DATA"
            && !ic.rank_label.is_empty()
        {
            let _ = writeln!(
                p,
                "### Insider Concentration — INSIDERCONC ({}, as of {})",
                ic.rank_label, ic.as_of
            );
            let _ = writeln!(
                p,
                "- Estimated insider-held {:.2}% ({:.0} shares) from {} tracked reporters / {} active holders, latest holdings {} · rank {}/{} · pct {:.0}",
                ic.estimated_insider_pct_held,
                ic.total_estimated_insider_shares,
                ic.reporters_covered,
                ic.reporters_holding_shares,
                ic.latest_holdings_date,
                ic.rank_position,
                ic.peers_considered + 1,
                ic.percentile_rank
            );
            if !ic.largest_reporter.is_empty() {
                let _ = writeln!(
                    p,
                    "- Largest reporter {}: {:.0} shares ({:.2}% of outstanding, {:.1}% of tracked insider holdings)",
                    ic.largest_reporter,
                    ic.largest_reporter_shares,
                    ic.largest_reporter_pct_of_outstanding,
                    ic.largest_reporter_weight_pct
                );
            }
            let _ = writeln!(
                p,
                "- Sector {} median / p25 / p75 insider-held: {:.2}% / {:.2}% / {:.2}%",
                ic.sector,
                ic.sector_median_pct_held,
                ic.sector_p25_pct_held,
                ic.sector_p75_pct_held
            );
            if !ic.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ic.note);
            }
            let _ = writeln!(p);
        }
    }
}
