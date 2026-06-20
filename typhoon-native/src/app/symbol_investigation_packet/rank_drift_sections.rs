use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_rank_drift_sections(&self, p: &mut String, sym_upper: &str) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;

                // ── rank & drift surfaces ──────────────
                if let Ok(Some(vr)) = rx::get_vrk(&conn, &sym_upper) {
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
                            vr.sector_median_score,
                            vr.sector_p25,
                            vr.sector_p75,
                            vr.peers_with_data
                        );
                        if !vr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", vr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(qr)) = rx::get_qrk(&conn, &sym_upper) {
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
                            qr.sector_median_score,
                            qr.sector_p25,
                            qr.sector_p75,
                            qr.peers_with_data
                        );
                        if !qr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", qr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(rr)) = rx::get_rrk(&conn, &sym_upper) {
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
                            rr.sector_median_score,
                            rr.sector_p25,
                            rr.sector_p75,
                            rr.peers_with_data
                        );
                        if !rr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", rr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

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

                if let Ok(Some(pr)) = rx::get_peadrank(&conn, &sym_upper) {
                    if pr.rank_label != "NO_DATA"
                        && pr.rank_label != "INSUFFICIENT_DATA"
                        && !pr.rank_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### PEAD Rank — PEADRANK ({}, as of {})",
                            pr.rank_label, pr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Avg 5d drift {:+.2}% · rank {}/{} · pct {:.0}",
                            pr.avg_drift_5d_pct,
                            pr.rank_position,
                            pr.peers_considered + 1,
                            pr.percentile_rank
                        );
                        let _ = writeln!(
                            p,
                            "- Sector {} median / p25 / p75: {:+.2}% / {:+.2}% / {:+.2}%",
                            pr.sector,
                            pr.sector_median_drift_5d_pct,
                            pr.sector_p25_drift_5d_pct,
                            pr.sector_p75_drift_5d_pct
                        );
                        if !pr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", pr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(fq)) = rx::get_fqm(&conn, &sym_upper) {
                    if fq.operator_label != "NO_DATA" && !fq.operator_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Fundamental Quality Meter — FQM ({}, as of {})",
                            fq.operator_label, fq.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Composite {:.1}/100 · {} inputs · Piotroski {:.0} ({}) · Op margin {:.2}% ({}) · Cash conv {:.2}% ({})",
                            fq.composite_score,
                            fq.inputs_available,
                            fq.piotroski_score,
                            fq.piotroski_label,
                            fq.operating_margin_pct,
                            fq.margin_trend_label,
                            fq.cash_conversion_pct,
                            fq.accruals_trend_label
                        );
                        if !fq.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", fq.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(rv)) = rx::get_revrank(&conn, &sym_upper) {
                    if rv.relative_label != "NO_DATA"
                        && rv.relative_label != "INSUFFICIENT_DATA"
                        && !rv.relative_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### Relative Revenue Growth — REVRANK ({}, as of {})",
                            rv.relative_label, rv.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- 3y CAGR {:+.2}% ({} yrs) · gap {:+.2}pp vs sector {} median {:+.2}%",
                            rv.symbol_cagr_pct,
                            rv.years_used,
                            rv.gap_to_median_pp,
                            rv.sector,
                            rv.sector_median_cagr_pct
                        );
                        let _ = writeln!(
                            p,
                            "- Latest ${:.2}B → earliest ${:.2}B · p25 / p75: {:+.2}% / {:+.2}%",
                            rv.latest_revenue / 1e9,
                            rv.earliest_revenue / 1e9,
                            rv.sector_p25_cagr_pct,
                            rv.sector_p75_cagr_pct
                        );
                        if !rv.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", rv.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
                if let Ok(Some(lv)) = rx::get_levrank(&conn, &sym_upper) {
                    if lv.rank_label != "NO_DATA"
                        && lv.rank_label != "INSUFFICIENT_DATA"
                        && !lv.rank_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### Leverage Rank — LEVRANK ({}, as of {})",
                            lv.rank_label, lv.as_of
                        );
                        if lv.rank_label == "NEGATIVE_EQUITY" {
                            let _ = writeln!(
                                p,
                                "- Negative/zero equity: total_debt ${:.2}B · total_equity ${:.2}B",
                                lv.total_debt / 1e9,
                                lv.total_equity / 1e9
                            );
                        } else {
                            let _ = writeln!(
                                p,
                                "- D/E {:.2} · rank {}/{} · pct {:.0} (higher = safer)",
                                lv.debt_to_equity,
                                lv.rank_position,
                                lv.peers_considered + 1,
                                lv.percentile_rank
                            );
                            let _ = writeln!(
                                p,
                                "- Sector {} median / p25 / p75 D/E: {:.2} / {:.2} / {:.2}",
                                lv.sector,
                                lv.sector_median_d2e,
                                lv.sector_p25_d2e,
                                lv.sector_p75_d2e
                            );
                        }
                        if !lv.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", lv.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(op)) = rx::get_operank(&conn, &sym_upper) {
                    if op.rank_label != "NO_DATA"
                        && op.rank_label != "INSUFFICIENT_DATA"
                        && !op.rank_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### Operating Quality Rank — OPERANK ({}, as of {})",
                            op.rank_label, op.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Op margin {:.2}% ({}) · rank {}/{} · pct {:.0}",
                            op.operating_margin_pct,
                            op.margin_trend_label,
                            op.rank_position,
                            op.peers_considered + 1,
                            op.percentile_rank
                        );
                        let _ = writeln!(
                            p,
                            "- Sector {} median / p25 / p75 op margin: {:.2}% / {:.2}% / {:.2}%",
                            op.sector,
                            op.sector_median_margin_pct,
                            op.sector_p25_margin_pct,
                            op.sector_p75_margin_pct
                        );
                        if !op.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", op.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(fqr)) = rx::get_fqmrank(&conn, &sym_upper) {
                    if fqr.rank_label != "NO_DATA"
                        && fqr.rank_label != "INSUFFICIENT_DATA"
                        && !fqr.rank_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### Fundamental Quality Rank — FQMRANK ({} / {}, as of {})",
                            fqr.operator_label, fqr.rank_label, fqr.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Composite {:.1}/100 · rank {}/{} · pct {:.0}",
                            fqr.composite_score,
                            fqr.rank_position,
                            fqr.peers_considered + 1,
                            fqr.percentile_rank
                        );
                        let _ = writeln!(
                            p,
                            "- Sector {} median / p25 / p75: {:.1} / {:.1} / {:.1}",
                            fqr.sector, fqr.sector_median_score, fqr.sector_p25, fqr.sector_p75
                        );
                        if !fqr.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", fqr.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(lq)) = rx::get_liqrank(&conn, &sym_upper) {
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

                if let Ok(Some(tlr)) = rx::get_tlrank(&conn, &sym_upper) {
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

                if let Ok(Some(ss)) = rx::get_surpstk(&conn, &sym_upper) {
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
                                ss.latest_event_date,
                                ss.latest_event_label,
                                ss.latest_event_surprise_pct
                            );
                        }
                        if !ss.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ss.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // ── Research section ──
                if let Ok(Some(dr)) = rx::get_dvdrank(&conn, &sym_upper) {
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

                if let Ok(Some(er)) = rx::get_earmrank(&conn, &sym_upper) {
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

                if let Ok(Some(ur)) = rx::get_updgrank(&conn, &sym_upper) {
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
                            ur.sector,
                            ur.sector_median_net_90d,
                            ur.sector_p25_net_90d,
                            ur.sector_p75_net_90d
                        );
                        if !ur.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ur.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(gy)) = rx::get_gy(&conn, &sym_upper) {
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

                if let Ok(Some(ds)) = rx::get_des(&conn, &sym_upper) {
                    if ds.streak_label != "INSUFFICIENT_DATA" && !ds.streak_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Daily Event Streak — DES ({}, as of {})",
                            ds.streak_label, ds.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- {} bars · up/down/flat {} / {} / {} · up rate {:.0}%",
                            ds.bars_used,
                            ds.up_days,
                            ds.down_days,
                            ds.flat_days,
                            ds.up_day_rate_pct
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
                if let Ok(Some(dyr)) = rx::get_dvdyieldrank(&conn, &sym_upper) {
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

                if let Ok(Some(sr)) = rx::get_shrank(&conn, &sym_upper) {
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

                if let Ok(Some(sd)) = rx::get_shortrank_delta(&conn, &sym_upper) {
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

                if let Ok(Some(ic)) = rx::get_insiderconc(&conn, &sym_upper) {
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

                if let Ok(Some(rvc)) = rx::get_rvcone(&conn, &sym_upper) {
                    if rvc.cone_label != "INSUFFICIENT_DATA" && !rvc.cone_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Realized Volatility Cone — RVCONE ({}, as of {})",
                            rvc.cone_label, rvc.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- 20d / 60d / 120d / 252d RV: {:.1}% / {:.1}% / {:.1}% / {:.1}%",
                            rvc.rv20_pct, rvc.rv60_pct, rvc.rv120_pct, rvc.rv252_pct
                        );
                        let _ = writeln!(
                            p,
                            "- 20d rolling min / median / max: {:.1}% / {:.1}% / {:.1}% · latest 20d pct {:.0}",
                            rvc.rv20_min_pct,
                            rvc.rv20_median_pct,
                            rvc.rv20_max_pct,
                            rvc.rv20_percentile
                        );
                        if !rvc.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", rvc.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(cpb)) = rx::get_calpb(&conn, &sym_upper) {
                    if cpb.momentum_label != "INSUFFICIENT_DATA" && !cpb.momentum_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Calendar Period Breakdown — CALPB ({} / {} {}, as of {})",
                            cpb.momentum_label, cpb.current_year, cpb.current_quarter, cpb.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- MTD {:+.2}% · QTD {:+.2}% · YTD {:+.2}%",
                            cpb.mtd_pct, cpb.qtd_pct, cpb.ytd_pct
                        );
                        let _ = writeln!(
                            p,
                            "- Prior quarter {:+.2}% · prior year {:+.2}%",
                            cpb.prior_quarter_pct, cpb.prior_year_pct
                        );
                        if !cpb.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", cpb.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(cs)) = rx::get_corrstk(&conn, &sym_upper) {
                    if cs.correlation_label != "INSUFFICIENT_DATA"
                        && !cs.correlation_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### Benchmark Correlation — CORRSTK ({}, as of {})",
                            cs.correlation_label, cs.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Dominant {} · SPY 20/60/252 {:.2} / {:.2} / {:.2}",
                            cs.dominant_benchmark,
                            cs.corr_spy_20d,
                            cs.corr_spy_60d,
                            cs.corr_spy_252d
                        );
                        let _ = writeln!(
                            p,
                            "- Sector 20/60/252 {:.2} / {:.2} / {:.2} · SPY β {:.2} · sector β {:.2}",
                            cs.corr_sector_20d,
                            cs.corr_sector_60d,
                            cs.corr_sector_252d,
                            cs.beta_spy_252d,
                            cs.beta_sector_252d
                        );
                        if !cs.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", cs.note);
                        }
                        let _ = writeln!(p);
                    }
                }

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

                if let Ok(Some(od)) = rx::get_operank_delta(&conn, &sym_upper) {
                    if od.rank_label != "NO_DATA"
                        && od.rank_label != "INSUFFICIENT_DATA"
                        && !od.rank_label.is_empty()
                    {
                        let _ = writeln!(
                            p,
                            "### Operating Margin Trend Rank — OPERANK_DELTA ({} / {}, as of {})",
                            od.operating_trend_label, od.rank_label, od.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- {} basis {} · operating margin {:.2}% · change {:+.2} pts · rank {}/{} · pct {:.0}",
                            od.basis,
                            od.latest_period,
                            od.operating_margin_pct,
                            od.operating_margin_change_pct,
                            od.rank_position,
                            od.peers_considered + 1,
                            od.percentile_rank
                        );
                        let _ = writeln!(
                            p,
                            "- Sector {} median / p25 / p75 change: {:+.2} / {:+.2} / {:+.2} pts",
                            od.sector,
                            od.sector_median_change_pct,
                            od.sector_p25_change_pct,
                            od.sector_p75_change_pct
                        );
                        if !od.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", od.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(da)) = rx::get_divacc(&conn, &sym_upper) {
                    if da.divacc_label != "NO_HISTORY" && !da.divacc_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Dividend Acceleration — DIVACC ({}, as of {})",
                            da.divacc_label, da.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- {} years · latest annual dividend {:.4} ({}) · latest/prior y/y {:+.2}% / {:+.2}%",
                            da.years_covered,
                            da.latest_annual_dividend,
                            da.latest_year,
                            da.latest_yoy_growth_pct,
                            da.prior_yoy_growth_pct
                        );
                        let _ = writeln!(
                            p,
                            "- Acceleration {:+.2} pts · 3y avg growth {:+.2}% vs prior {:+.2}% · consistency {:.0}%",
                            da.acceleration_pct_pts,
                            da.recent_3y_avg_growth_pct,
                            da.prior_3y_avg_growth_pct,
                            da.consistency_score_pct
                        );
                        if !da.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", da.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(ea)) = rx::get_epsacc(&conn, &sym_upper) {
                    if ea.epsacc_label != "INSUFFICIENT_DATA" && !ea.epsacc_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### EPS Acceleration — EPSACC ({}, as of {})",
                            ea.epsacc_label, ea.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- Latest {} EPS {:.3} vs {:.3} y/y · latest/prior y/y {:+.2}% / {:+.2}%",
                            ea.latest_period,
                            ea.latest_eps,
                            ea.prior_year_eps,
                            ea.latest_yoy_growth_pct,
                            ea.prior_yoy_growth_pct
                        );
                        let _ = writeln!(
                            p,
                            "- Acceleration {:+.2} pts · recent 2q avg {:+.2}% vs prior {:+.2}% · positive y/y quarters {}",
                            ea.acceleration_pct_pts,
                            ea.recent_2q_avg_yoy_growth_pct,
                            ea.prior_2q_avg_yoy_growth_pct,
                            ea.positive_yoy_quarters
                        );
                        if !ea.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", ea.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                if let Ok(Some(vrp)) = rx::get_vrp(&conn, &sym_upper) {
                    if vrp.premium_label != "INSUFFICIENT_DATA" && !vrp.premium_label.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Vol Risk Premium — VRP ({} / {}, as of {})",
                            vrp.premium_label, vrp.rv_cone_label, vrp.as_of
                        );
                        let _ = writeln!(
                            p,
                            "- ATM IV {:.1}% (rank {:.0} / pct {:.0}) · RV20 / RV60 / RV252 {:.1}% / {:.1}% / {:.1}%",
                            vrp.current_atm_iv_pct,
                            vrp.iv_rank,
                            vrp.iv_percentile,
                            vrp.rv20_pct,
                            vrp.rv60_pct,
                            vrp.rv252_pct
                        );
                        let _ = writeln!(
                            p,
                            "- IV-RV20 {:+.1} pts ({:.2}x) · IV-RV252 {:+.1} pts ({:.2}x) · RV20 cone pct {:.0}",
                            vrp.iv_minus_rv20_pct,
                            vrp.iv_to_rv20_ratio,
                            vrp.iv_minus_rv252_pct,
                            vrp.iv_to_rv252_ratio,
                            vrp.rv20_percentile
                        );
                        if !vrp.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", vrp.note);
                        }
                        let _ = writeln!(p);
                    }
                }
            }
        }
    }
}
