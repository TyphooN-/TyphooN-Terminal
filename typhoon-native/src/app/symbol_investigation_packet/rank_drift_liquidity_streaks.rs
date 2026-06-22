use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub(super) fn write_rank_drift_liquidity_streaks(
    ctx: &SymbolResearchContext,
    p: &mut String,
    sym_upper: &str,
) {
    if let Ok(Some(lq)) = rx::get_liqrank(ctx.conn, &sym_upper) {
        if lq.rank_label != "NO_DATA"
            && lq.rank_label != "INSUFFICIENT_DATA"
            && !lq.rank_label.is_empty()
        {
            let _ = writeln!(
                p,
                "### Liquidity Rank — LIQRANK ({} / {}, as of {})",
                lq.tier_label, lq.rank_label, lq.as_of
            );
            let _ = writeln!(
                p,
                "- ADV$ ${:.2}M · rank {}/{} · pct {:.0}",
                lq.avg_daily_dollar_volume / 1e6,
                lq.rank_position,
                lq.peers_considered + 1,
                lq.percentile_rank
            );
            let _ = writeln!(
                p,
                "- Sector {} median / p25 / p75 ADV$: ${:.2}M / ${:.2}M / ${:.2}M",
                lq.sector,
                lq.sector_median_dollar_volume / 1e6,
                lq.sector_p25_dollar_volume / 1e6,
                lq.sector_p75_dollar_volume / 1e6
            );
            if !lq.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", lq.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(tlr)) = rx::get_tlrank(ctx.conn, &sym_upper) {
        if tlr.rank_label != "NO_DATA"
            && tlr.rank_label != "INSUFFICIENT_DATA"
            && !tlr.rank_label.is_empty()
        {
            let _ = writeln!(
                p,
                "### 30-Day Liquidity Rank — TLRANK ({} / {}, as of {})",
                tlr.tier_label, tlr.rank_label, tlr.as_of
            );
            let _ = writeln!(
                p,
                "- 30d ADV$ ${:.2}M · rank {}/{} · pct {:.0} · {} valid bars",
                tlr.avg_30d_dollar_volume / 1e6,
                tlr.rank_position,
                tlr.peers_considered + 1,
                tlr.percentile_rank,
                tlr.bars_used
            );
            let _ = writeln!(
                p,
                "- Sector {} median / p25 / p75 30d ADV$: ${:.2}M / ${:.2}M / ${:.2}M",
                tlr.sector,
                tlr.sector_median_dollar_volume / 1e6,
                tlr.sector_p25_dollar_volume / 1e6,
                tlr.sector_p75_dollar_volume / 1e6
            );
            if !tlr.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", tlr.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(ss)) = rx::get_surpstk(ctx.conn, &sym_upper) {
        if ss.streak_label != "INSUFFICIENT_DATA" && !ss.streak_label.is_empty() {
            let _ = writeln!(
                p,
                "### Earnings Surprise Streak — SURPSTK ({}, as of {})",
                ss.streak_label, ss.as_of
            );
            let _ = writeln!(
                p,
                "- {} events · {} beats / {} misses / {} inlines · beat rate {:.0}% · avg surprise {:+.2}%",
                ss.total_events,
                ss.beats,
                ss.misses,
                ss.inlines,
                ss.beat_rate_pct,
                ss.avg_surprise_pct
            );
            let _ = writeln!(
                p,
                "- Current streak: {} × {} · longest beat/miss: {} / {}",
                ss.current_streak_len,
                ss.current_streak_type,
                ss.longest_beat_streak,
                ss.longest_miss_streak
            );
            if !ss.latest_event_date.is_empty() {
                let _ = writeln!(
                    p,
                    "- Latest event {} ({}): {:+.2}% surprise",
                    ss.latest_event_date, ss.latest_event_label, ss.latest_event_surprise_pct
                );
            }
            if !ss.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ss.note);
            }
            let _ = writeln!(p);
        }
    }

    // ── Research section ──
}
