use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub fn write_rank_drift_core_ranks(ctx: &SymbolResearchContext, p: &mut String, sym_upper: &str) {
    // ── rank & drift surfaces ──────────────
    if let Ok(Some(vr)) = rx::get_vrk(ctx.conn, &sym_upper) {
        if vr.rank_label != "NO_DATA" && !vr.rank_label.is_empty() {
            let _ = writeln!(
                p,
                "### Value Rank — VRK ({}, as of {})",
                vr.rank_label, vr.as_of
            );
            let _ = writeln!(
                p,
                "- Sector: {} · Subject composite {:.1} · Rank {}/{} · Percentile {:.0}",
                vr.sector,
                vr.composite_score,
                vr.rank_position,
                vr.peers_considered + 1,
                vr.percentile_rank
            );
            let _ = writeln!(
                p,
                "- Sector median/p25/p75: {:.1} / {:.1} / {:.1} ({} peers with data)",
                vr.sector_median_score, vr.sector_p25, vr.sector_p75, vr.peers_with_data
            );
            if !vr.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", vr.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(qr)) = rx::get_qrk(ctx.conn, &sym_upper) {
        if qr.rank_label != "NO_DATA" && !qr.rank_label.is_empty() {
            let _ = writeln!(
                p,
                "### Quality Rank — QRK ({}, as of {})",
                qr.rank_label, qr.as_of
            );
            let _ = writeln!(
                p,
                "- Sector: {} · Subject composite {:.1} · Rank {}/{} · Percentile {:.0}",
                qr.sector,
                qr.composite_score,
                qr.rank_position,
                qr.peers_considered + 1,
                qr.percentile_rank
            );
            let _ = writeln!(
                p,
                "- Sector median/p25/p75: {:.1} / {:.1} / {:.1} ({} peers with data)",
                qr.sector_median_score, qr.sector_p25, qr.sector_p75, qr.peers_with_data
            );
            if !qr.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", qr.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(rr)) = rx::get_rrk(ctx.conn, &sym_upper) {
        if rr.rank_label != "NO_DATA" && !rr.rank_label.is_empty() {
            let _ = writeln!(
                p,
                "### Risk Rank — RRK ({}, as of {}) [higher pct = SAFER]",
                rr.rank_label, rr.as_of
            );
            let _ = writeln!(
                p,
                "- Sector: {} · Subject composite {:.1} (higher = riskier) · Rank {}/{} · Safe percentile {:.0}",
                rr.sector,
                rr.composite_score,
                rr.rank_position,
                rr.peers_considered + 1,
                rr.percentile_rank
            );
            let _ = writeln!(
                p,
                "- Sector median/p25/p75 risk: {:.1} / {:.1} / {:.1} ({} peers with data)",
                rr.sector_median_score, rr.sector_p25, rr.sector_p75, rr.peers_with_data
            );
            if !rr.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", rr.note);
            }
            let _ = writeln!(p);
        }
    }
}
