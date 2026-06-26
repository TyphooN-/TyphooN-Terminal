use super::context::SymbolResearchContext;
use std::fmt::Write as _;
use typhoon_engine::core::research as rx;

pub fn write_price_behavior_local(ctx: &SymbolResearchContext, p: &mut String, sym_upper: &str) {
    if let Ok(Some(rln)) = rx::get_runlen(ctx.conn, &sym_upper) {
        if rln.trend_label != "INSUFFICIENT_DATA" && !rln.trend_label.is_empty() {
            let _ = writeln!(
                p,
                "### Run Length — RUNLEN ({}, as of {})",
                rln.trend_label, rln.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · avg up run {:.2} (count {}) · avg down run {:.2} (count {})",
                rln.bars_used,
                rln.avg_up_run,
                rln.up_runs_count,
                rln.avg_down_run,
                rln.down_runs_count
            );
            let cur_run = if rln.current_run_length > 0 {
                format!("{} up", rln.current_run_length)
            } else if rln.current_run_length < 0 {
                format!("{} down", -rln.current_run_length)
            } else {
                "flat".to_string()
            };
            let _ = writeln!(
                p,
                "- Longest up {} · longest down {} · current {}",
                rln.longest_up_run, rln.longest_down_run, cur_run
            );
            if !rln.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", rln.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(dr)) = rx::get_dayrange(ctx.conn, &sym_upper) {
        if dr.range_label != "INSUFFICIENT_DATA" && !dr.range_label.is_empty() {
            let _ = writeln!(
                p,
                "### Daily Range — DAYRANGE ({}, as of {})",
                dr.range_label, dr.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · avg range 60d {:.2}% · 252d {:.2}% · latest {:.2}% · compression {:.2}",
                dr.bars_used,
                dr.avg_range_60_pct,
                dr.avg_range_252_pct,
                dr.latest_range_pct,
                dr.compression_ratio
            );
            let _ = writeln!(
                p,
                "- Widest {:.2}% · narrowest {:.2}%",
                dr.widest_range_pct, dr.narrowest_range_pct
            );
            if !dr.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", dr.note);
            }
            let _ = writeln!(p);
        }
    }

    // ── HP-local research surfaces ──
    if let Ok(Some(ac)) = rx::get_autocor(ctx.conn, &sym_upper) {
        if ac.regime_label != "INSUFFICIENT_DATA" && !ac.regime_label.is_empty() {
            let _ = writeln!(
                p,
                "### Return Autocorrelation — AUTOCOR ({}, as of {})",
                ac.regime_label, ac.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · mean log-ret {:.6}",
                ac.bars_used, ac.mean_log_return
            );
            let _ = writeln!(
                p,
                "- ACF lag1 {:.3} · lag5 {:.3} · lag10 {:.3} · lag20 {:.3}",
                ac.lag1_acf, ac.lag5_acf, ac.lag10_acf, ac.lag20_acf
            );
            if !ac.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ac.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(hu)) = rx::get_hurst(ctx.conn, &sym_upper) {
        if hu.memory_label != "INSUFFICIENT_DATA" && !hu.memory_label.is_empty() {
            let _ = writeln!(
                p,
                "### Hurst Exponent — HURST ({}, as of {})",
                hu.memory_label, hu.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · H {:.3} · scales used {} (min {}, max {})",
                hu.bars_used, hu.hurst_exponent, hu.scales_used, hu.min_scale, hu.max_scale
            );
            if !hu.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", hu.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(hr)) = rx::get_hitrate(ctx.conn, &sym_upper) {
        if hr.hit_label != "INSUFFICIENT_DATA" && !hr.hit_label.is_empty() {
            let _ = writeln!(
                p,
                "### Hit Rate — HITRATE ({}, as of {})",
                hr.hit_label, hr.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · up {} · down {} · flat {}",
                hr.bars_used, hr.up_days, hr.down_days, hr.flat_days
            );
            let _ = writeln!(
                p,
                "- 5d {:.1}% · 20d {:.1}% · 60d {:.1}% · 252d {:.1}%",
                hr.hitrate_5d * 100.0,
                hr.hitrate_20d * 100.0,
                hr.hitrate_60d * 100.0,
                hr.hitrate_252d * 100.0
            );
            if !hr.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", hr.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(ga)) = rx::get_glasym(ctx.conn, &sym_upper) {
        if ga.asymmetry_label != "INSUFFICIENT_DATA" && !ga.asymmetry_label.is_empty() {
            let _ = writeln!(
                p,
                "### Gain/Loss Asymmetry — GLASYM ({}, as of {})",
                ga.asymmetry_label, ga.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · up days {} · down days {}",
                ga.bars_used, ga.up_days, ga.down_days
            );
            let _ = writeln!(
                p,
                "- Avg up {:.2}% · avg down {:.2}% · median up {:.2}% · median down {:.2}% · ratio {:.2}",
                ga.avg_up_pct,
                ga.avg_down_pct,
                ga.median_up_pct,
                ga.median_down_pct,
                ga.magnitude_ratio
            );
            if !ga.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", ga.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(vr)) = rx::get_volratio(ctx.conn, &sym_upper) {
        if vr.flow_label != "INSUFFICIENT_DATA" && !vr.flow_label.is_empty() {
            let _ = writeln!(
                p,
                "### Up/Down Volume Ratio — VOLRATIO ({}, as of {})",
                vr.flow_label, vr.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · up days {} · down days {} · ratio {:.2}",
                vr.bars_used, vr.up_days, vr.down_days, vr.up_down_volume_ratio
            );
            let _ = writeln!(
                p,
                "- Avg up vol {:.0} · avg down vol {:.0} · median up {:.0} · median down {:.0}",
                vr.avg_up_volume, vr.avg_down_volume, vr.median_up_volume, vr.median_down_volume
            );
            let _ = writeln!(
                p,
                "- Max up vol {:.0} · max down vol {:.0}",
                vr.max_up_volume, vr.max_down_volume
            );
            if !vr.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", vr.note);
            }
            let _ = writeln!(p);
        }
    }

    // ── HP-local research surfaces ──
    if let Ok(Some(du)) = rx::get_drawup(ctx.conn, &sym_upper) {
        if du.rally_label != "INSUFFICIENT_DATA" && !du.rally_label.is_empty() {
            let _ = writeln!(
                p,
                "### Rally History — DRAWUP ({}, as of {})",
                du.rally_label, du.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · max drawup {:.2}% (trough {} → peak {})",
                du.bars_used, du.max_drawup_pct, du.max_drawup_trough_date, du.max_drawup_peak_date
            );
            let _ = writeln!(
                p,
                "- Longest rally {} sessions · current drawup {:.2}%",
                du.longest_drawup_days, du.current_drawup_pct
            );
            let _ = writeln!(
                p,
                "- Rallies ≥5% {} · rallies ≥10% {}",
                du.rallies_5pct, du.rallies_10pct
            );
            if !du.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", du.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(gs)) = rx::get_gapstats(ctx.conn, &sym_upper) {
        if gs.bias_label != "INSUFFICIENT_DATA" && !gs.bias_label.is_empty() {
            let _ = writeln!(
                p,
                "### Gap Statistics — GAPSTATS ({}, as of {})",
                gs.bias_label, gs.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · gap-ups {} · gap-downs {} · frequency {:.1}%",
                gs.bars_used, gs.gap_up_count, gs.gap_down_count, gs.gap_frequency_pct
            );
            let _ = writeln!(
                p,
                "- Avg gap {:.3}% · avg up {:.3}% · avg down {:.3}%",
                gs.avg_gap_pct, gs.avg_gap_up_pct, gs.avg_gap_down_pct
            );
            let _ = writeln!(
                p,
                "- Largest up {:.2}% · largest down {:.2}%",
                gs.largest_gap_up_pct, gs.largest_gap_down_pct
            );
            if !gs.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", gs.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(vc)) = rx::get_volcluster(ctx.conn, &sym_upper) {
        if vc.cluster_label != "INSUFFICIENT_DATA" && !vc.cluster_label.is_empty() {
            let _ = writeln!(
                p,
                "### Volatility Clustering — VOLCLUSTER ({}, as of {})",
                vc.cluster_label, vc.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · |r| ACF lag1 {:.3} · lag5 {:.3} · lag20 {:.3}",
                vc.bars_used, vc.abs_acf_lag1, vc.abs_acf_lag5, vc.abs_acf_lag20
            );
            let _ = writeln!(
                p,
                "- r² ACF lag1 {:.3} · lag5 {:.3} · lag20 {:.3}",
                vc.sq_acf_lag1, vc.sq_acf_lag5, vc.sq_acf_lag20
            );
            if !vc.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", vc.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(cp)) = rx::get_closeplc(ctx.conn, &sym_upper) {
        if cp.placement_label != "INSUFFICIENT_DATA" && !cp.placement_label.is_empty() {
            let _ = writeln!(
                p,
                "### Close Placement — CLOSEPLC ({}, as of {})",
                cp.placement_label, cp.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · avg {:.3} · median {:.3} · latest {:.3}",
                cp.bars_used, cp.avg_placement, cp.median_placement, cp.latest_placement
            );
            let _ = writeln!(
                p,
                "- Near-high share {:.1}% · near-low share {:.1}%",
                cp.pct_near_high, cp.pct_near_low
            );
            if !cp.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", cp.note);
            }
            let _ = writeln!(p);
        }
    }

    if let Ok(Some(mr)) = rx::get_mrhl(ctx.conn, &sym_upper) {
        if mr.regime_label != "INSUFFICIENT_DATA" && !mr.regime_label.is_empty() {
            let _ = writeln!(
                p,
                "### Mean-Reversion Half-Life — MRHL ({}, as of {})",
                mr.regime_label, mr.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · AR(1) β {:.4} · α {:.6} · R² {:.3}",
                mr.bars_used, mr.beta, mr.alpha, mr.r_squared
            );
            let _ = writeln!(p, "- Half-life {:.1} sessions", mr.half_life_days);
            if !mr.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", mr.note);
            }
            let _ = writeln!(p);
        }
    }

    // ── Research section ──
    if let Ok(Some(dv)) = rx::get_downvol(ctx.conn, &sym_upper) {
        if dv.sortino_label != "INSUFFICIENT_DATA" && !dv.sortino_label.is_empty() {
            let _ = writeln!(
                p,
                "### Downside Deviation / Sortino — DOWNVOL ({}, as of {})",
                dv.sortino_label, dv.as_of
            );
            let _ = writeln!(
                p,
                "- Bars {} · mean r {:.6} · downside dev {:.6} (ann {:.4})",
                dv.bars_used, dv.mean_log_return, dv.downside_dev, dv.downside_dev_ann
            );
            let _ = writeln!(
                p,
                "- Upside dev {:.6} · Sortino {:.3} (ann {:.3}) · downside {:.1}% of total var",
                dv.upside_dev, dv.sortino_ratio, dv.sortino_ratio_ann, dv.downside_pct_of_total
            );
            if !dv.note.is_empty() {
                let _ = writeln!(p, "- Note: {}", dv.note);
            }
            let _ = writeln!(p);
        }
    }
}
