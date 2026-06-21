use super::*;

impl TyphooNApp {
    pub(super) fn write_rank_drift_vol_perf(&self, p: &mut String, sym_upper: &str) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                if let Ok(Some(at)) = rx::get_atrann(&conn, &sym_upper) {
                    if at.regime_label != "INSUFFICIENT_DATA" && !at.regime_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Annualized ATR — ATRANN ({}, as of {})",
                            at.regime_label, at.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- {} bars · close {:.4} · ATR14 {:.4} ({:.2}%) · annualized {:.2}% (×√252)",
                            at.bars_used,
                            at.latest_close,
                            at.atr14,
                            at.atr14_pct,
                            at.atr_annualized_pct
                        );
                        if !at.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", at.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(dh)) = rx::get_ddhist(&conn, &sym_upper) {
                    if dh.regime_label != "INSUFFICIENT_DATA" && !dh.regime_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Drawdown History — DDHIST ({}, as of {})",
                            dh.regime_label, dh.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- {} bars · max dd {:+.2}% · current dd {:+.2}%",
                            dh.bars_used, dh.max_drawdown_pct, dh.current_drawdown_pct
                        );
                        if !dh.max_drawdown_peak_date.is_empty()
                            && !dh.max_drawdown_trough_date.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "- Max dd peak {} → trough {} · longest drawdown {} sessions",
                                dh.max_drawdown_peak_date,
                                dh.max_drawdown_trough_date,
                                dh.longest_drawdown_days
                            );
                        }
                        let _ = writeln!(
                            p,
                            "- Corrections ≥5% / ≥10%: {} / {}",
                            dh.corrections_5pct, dh.corrections_10pct
                        );
                        if !dh.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", dh.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(pp)) = rx::get_priceperf(&conn, &sym_upper) {
                    if pp.trend_label != "INSUFFICIENT_DATA" && !pp.trend_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Price Performance — PRICEPERF ({}, as of {})",
                            pp.trend_label, pp.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- {} bars · close {:.4} · 1M {:+.2}% · 3M {:+.2}% · 6M {:+.2}%",
                            pp.bars_used,
                            pp.latest_close,
                            pp.ret_1m_pct,
                            pp.ret_3m_pct,
                            pp.ret_6m_pct
                        );
                        let _ = writeln!(
                            p,
                            "- YTD {:+.2}% · 1Y {:+.2}%",
                            pp.ret_ytd_pct, pp.ret_1y_pct
                        );
                        if !pp.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", pp.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(mrm)) = rx::get_momrank_multi(&conn, &sym_upper) {
                    if mrm.rank_label != "NO_DATA"
                        && mrm.rank_label != "INSUFFICIENT_DATA"
                        && !mrm.rank_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### Sector Momentum Rank — MOMRANK_MULTI ({}, as of {})",
                            mrm.rank_label, mrm.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Sector {} · composite pct {:.1} · rank {}/{}",
                            mrm.sector,
                            mrm.composite_percentile,
                            mrm.rank_position,
                            mrm.peers_with_data + 1
                        );
                        let _ = writeln!(
                            p,
                            "- 1M / 3M / 6M pct {:.1} / {:.1} / {:.1} · YTD / 1Y pct {:.1} / {:.1}",
                            mrm.pct_1m, mrm.pct_3m, mrm.pct_6m, mrm.pct_ytd, mrm.pct_1y
                        );
                        if !mrm.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", mrm.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
                if let Ok(Some(br)) = rx::get_betarank(&conn, &sym_upper) {
                    if br.rank_label != "NO_DATA"
                        && br.rank_label != "INSUFFICIENT_DATA"
                        && !br.rank_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### Beta Rank — BETARANK ({}, as of {})",
                            br.rank_label, br.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- β {:.2} · rank {}/{} · pct {:.0} (higher = safer)",
                            br.subject_beta.unwrap_or(0.0),
                            br.rank_position,
                            br.peers_considered + 1,
                            br.percentile_rank
                        );
                        let _ = writeln!(
                            p,
                            "- Sector {} median / p25 / p75 β: {:.2} / {:.2} / {:.2}",
                            br.sector,
                            br.sector_median_beta,
                            br.sector_p25_beta,
                            br.sector_p75_beta
                        );
                        if !br.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", br.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(pgr)) = rx::get_pegrank(&conn, &sym_upper) {
                    if pgr.rank_label != "NO_DATA"
                        && pgr.rank_label != "INSUFFICIENT_DATA"
                        && !pgr.rank_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### PEG Rank — PEGRANK ({}, as of {})",
                            pgr.rank_label, pgr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- PEG {:.2} · rank {}/{} · pct {:.0} (higher = better value)",
                            pgr.subject_peg.unwrap_or(0.0),
                            pgr.rank_position,
                            pgr.peers_considered + 1,
                            pgr.percentile_rank
                        );
                        let _ = writeln!(
                            p,
                            "- Sector {} median / p25 / p75 PEG: {:.2} / {:.2} / {:.2}",
                            pgr.sector,
                            pgr.sector_median_peg,
                            pgr.sector_p25_peg,
                            pgr.sector_p75_peg
                        );
                        if !pgr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", pgr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(fhl)) = rx::get_fhighlow(&conn, &sym_upper) {
                    if fhl.proximity_label != "INSUFFICIENT_DATA" && !fhl.proximity_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### 52-Week High/Low — FHIGHLOW ({}, as of {})",
                            fhl.proximity_label, fhl.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Close {:.2} · high {:.2} ({} sessions ago) · low {:.2} ({} sessions ago)",
                            fhl.latest_close,
                            fhl.high_52w,
                            fhl.days_since_high,
                            fhl.low_52w,
                            fhl.days_since_low
                        );
                        let _ = writeln!(
                            p,
                            "- From high {:+.2}% · from low {:+.2}% · range position {:.1}%",
                            fhl.pct_from_high, fhl.pct_from_low, fhl.range_position_pct
                        );
                        if !fhl.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", fhl.note);
                        }
                        let _ = writeln!(p);
                    }
                }
            }
        }
    }
}
