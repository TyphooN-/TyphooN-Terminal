use super::*;

impl TyphooNApp {
    pub(super) fn write_rank_drift_growth_drift(&self, p: &mut String, sym_upper: &str) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                if let Ok(Some(eg)) = rx::get_relepsgr(&conn, &sym_upper) {
                    if eg.relative_label != "NO_DATA" && !eg.relative_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Relative EPS Growth — RELEPSGR ({}, as of {})",
                            eg.relative_label, eg.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Sector: {} · 3y CAGR {:.1}% (EPS {:.2} → {:.2} over {} yrs)",
                            eg.sector,
                            eg.symbol_cagr_pct,
                            eg.earliest_eps,
                            eg.latest_eps,
                            eg.years_used
                        );
                        let _ = writeln!(
                            p,
                            "- Sector median/p25/p75 CAGR: {:.1}% / {:.1}% / {:.1}% · Gap to median {:+.1}pp ({} peers with data)",
                            eg.sector_median_cagr_pct,
                            eg.sector_p25_cagr_pct,
                            eg.sector_p75_cagr_pct,
                            eg.gap_to_median_pp,
                            eg.peers_with_data
                        );
                        if !eg.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", eg.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(pd)) = rx::get_pead(&conn, &sym_upper) {
                    if pd.drift_direction_label != "INSUFFICIENT_DATA"
                        && !pd.drift_direction_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### Post-Earnings Drift — PEAD ({}, as of {})",
                            pd.drift_direction_label, pd.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Events: {}/{} used · Avg drift 1d/3d/5d/10d: {:+.2}% / {:+.2}% / {:+.2}% / {:+.2}%",
                            pd.events_used,
                            pd.num_events,
                            pd.avg_drift_1d_pct,
                            pd.avg_drift_3d_pct,
                            pd.avg_drift_5d_pct,
                            pd.avg_drift_10d_pct
                        );
                        let _ = writeln!(
                            p,
                            "- Beat 5d {:+.2}% · Miss 5d {:+.2}% · Latest {} ({:+.2}% surprise, {:+.2}% 5d drift)",
                            pd.beat_event_drift_5d_pct,
                            pd.miss_event_drift_5d_pct,
                            pd.latest_event_date,
                            pd.latest_event_surprise_pct,
                            pd.latest_event_drift_5d_pct
                        );
                        if !pd.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", pd.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
                if let Ok(Some(sf)) = rx::get_sizef(&conn, &sym_upper) {
                    if sf.rank_label != "NO_DATA"
                        && sf.rank_label != "INSUFFICIENT_DATA"
                        && !sf.rank_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### Size Factor — SIZEF ({} / {}, as of {})",
                            sf.tier_label, sf.rank_label, sf.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Market cap ${:.2}B · log {:.3} · rank {}/{} · pct {:.0}",
                            sf.market_cap / 1e9,
                            sf.log_market_cap,
                            sf.rank_position,
                            sf.peers_considered + 1,
                            sf.percentile_rank
                        );
                        let _ = writeln!(
                            p,
                            "- Sector {} median / p25 / p75: ${:.2}B / ${:.2}B / ${:.2}B",
                            sf.sector,
                            sf.sector_median_cap / 1e9,
                            sf.sector_p25_cap / 1e9,
                            sf.sector_p75_cap / 1e9
                        );
                        if !sf.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", sf.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(mf)) = rx::get_momf(&conn, &sym_upper) {
                    if mf.rank_label != "NO_DATA"
                        && mf.rank_label != "INSUFFICIENT_DATA"
                        && !mf.rank_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### Momentum Rank — MOMF ({}, as of {})",
                            mf.rank_label, mf.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Composite {:.1} · rank {}/{} · pct {:.0}",
                            mf.composite_score,
                            mf.rank_position,
                            mf.peers_considered + 1,
                            mf.percentile_rank
                        );
                        let _ = writeln!(
                            p,
                            "- Sector {} median / p25 / p75: {:.1} / {:.1} / {:.1}",
                            mf.sector, mf.sector_median_score, mf.sector_p25, mf.sector_p75
                        );
                        if !mf.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", mf.note);
                        }
                        let _ = writeln!(p);
                    }
                }
            }
        }
    }
}
