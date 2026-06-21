use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_rank_drift_sections(&self, p: &mut String, sym_upper: &str) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Ok(conn) = cache.open_bg_read_connection() {
                use typhoon_engine::core::research as rx;

                self.write_rank_drift_core_ranks(p, sym_upper);

                self.write_rank_drift_growth_drift(p, sym_upper);

                self.write_rank_drift_fund_quality(p, sym_upper);

                self.write_rank_drift_research_ranks(p, sym_upper);

                self.write_rank_drift_liquidity_streaks(p, sym_upper);

                self.write_rank_drift_div_earn_streaks(p, sym_upper);

                self.write_rank_drift_yield_short_conc(p, sym_upper);

                self.write_rank_drift_vol_perf(p, sym_upper);

                self.write_rank_drift_cone_corrs(p, sym_upper);

                if let Ok(Some(cr)) = rx::get_corrrank(&conn, &sym_upper) {
                    if cr.rank_label != "NO_DATA"
                        && cr.rank_label != "INSUFFICIENT_DATA"
                        && !cr.rank_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### Benchmark Linkage Rank — CORRRANK ({} / {}, as of {})",
                            cr.subject_correlation_label, cr.rank_label, cr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- {} {} corr {:.2} (|corr| {:.2}) · β {:.2} · R² {:.2} · rank {}/{} · pct {:.0}",
                            cr.benchmark_kind,
                            cr.benchmark_name,
                            cr.subject_corr_252d,
                            cr.subject_abs_corr_252d,
                            cr.subject_beta_252d,
                            cr.subject_r_squared_252d,
                            cr.rank_position,
                            cr.peers_considered + 1,
                            cr.percentile_rank
                        );
                        let _ = writeln!(
                            p,
                            "- Sector {} median / p25 / p75 |corr|: {:.2} / {:.2} / {:.2}",
                            cr.sector,
                            cr.sector_median_abs_corr_252d,
                            cr.sector_p25_abs_corr_252d,
                            cr.sector_p75_abs_corr_252d
                        );
                        if !cr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", cr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                self.write_rank_drift_accs_vrp(p, sym_upper);
            }
        }
    }
}
