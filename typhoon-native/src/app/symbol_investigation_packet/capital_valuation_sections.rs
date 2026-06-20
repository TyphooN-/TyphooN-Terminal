use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_capital_valuation_sections(&self, p: &mut String, sym_upper: &str) {
        use std::fmt::Write as _;
        if let Some(ref cache) = self.cache {
            if let Some(conn) = cache.try_connection() {
                use typhoon_engine::core::research as rx;
                let fmt_money = typhoon_engine::core::fundamentals::format_large_number;

                // WACC snapshot (CAPM-derived cost of capital)
                if let Ok(Some(w)) = rx::get_wacc(&conn, &sym_upper) {
                    if w.wacc_pct > 0.0 {
                        let _ = writeln!(p, "### WACC Snapshot (CAPM, as of {})", w.as_of);
                        let _ = writeln!(
                            p,
                            "- Cost of equity (Re) {:.2}% · after-tax cost of debt {:.2}% · **WACC {:.2}%**",
                            w.cost_of_equity_pct, w.after_tax_cost_of_debt_pct, w.wacc_pct
                        );
                        let _ = writeln!(
                            p,
                            "- Inputs — β {:.3} · Rf {:.2}% · ERP {:.2}% · tax {:.2}%",
                            w.beta, w.risk_free_pct, w.equity_risk_premium_pct, w.tax_rate_pct
                        );
                        let _ = writeln!(
                            p,
                            "- Capital mix — equity {:.1}% ({}) · debt {:.1}% ({})",
                            w.equity_weight * 100.0,
                            fmt_money(w.market_cap),
                            w.debt_weight * 100.0,
                            fmt_money(w.total_debt)
                        );
                        let _ = writeln!(p);
                    }
                }

                // BETA rolling history (1Y / 3Y / 5Y vs SPY)
                if let Ok(Some(b)) = rx::get_beta(&conn, &sym_upper) {
                    if !b.windows.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Rolling Beta vs {} (as of {})",
                            b.market_ticker, b.as_of
                        );
                        let _ = writeln!(p, "| Window | β | α (ann) | R² | Corr | N |");
                        let _ = writeln!(p, "|---|---|---|---|---|---|");
                        for w in b.windows.iter() {
                            let _ = writeln!(
                                p,
                                "| {} | {:.3} | {:+.2}% | {:.3} | {:.3} | {} |",
                                w.window_label,
                                w.beta,
                                w.alpha_pct,
                                w.r_squared,
                                w.correlation,
                                w.n_observations
                            );
                        }
                        if !b.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", b.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // DDM Gordon Growth fair value
                if let Ok(Some(d)) = rx::get_ddm(&conn, &sym_upper) {
                    if d.annual_dividend > 0.0 || d.implied_price > 0.0 {
                        let _ = writeln!(p, "### Gordon Growth DDM (as of {})", d.as_of);
                        let _ = writeln!(
                            p,
                            "- Trailing D0 ${:.4} · implied g {:.2}% ({}) · required r {:.2}% ({})",
                            d.annual_dividend,
                            d.implied_growth_pct,
                            d.growth_source,
                            d.required_return_pct,
                            d.return_source
                        );
                        if d.implied_price > 0.0 {
                            let _ = writeln!(
                                p,
                                "- **Implied price ${:.2}** (method: {})",
                                d.implied_price, d.method
                            );
                        } else if !d.note.is_empty() {
                            let _ = writeln!(p, "- Caveat: {}", d.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // RV relative-valuation matrix (peer Z-scores)
                if let Ok(Some(rv)) = rx::get_relative_valuation(&conn, &sym_upper) {
                    if !rv.rows.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Relative Valuation vs Sector Peers (n={}, as of {})",
                            rv.peer_count, rv.as_of
                        );
                        let _ = writeln!(p, "| Metric | Value | Peer Median | Z | Percentile |");
                        let _ = writeln!(p, "|---|---|---|---|---|");
                        for r in rv.rows.iter() {
                            let _ = writeln!(
                                p,
                                "| {} | {:.2} | {:.2} | {:+.2} | {:.0}% |",
                                r.metric, r.value, r.peer_median, r.z_score, r.percentile
                            );
                        }
                        let _ = writeln!(p);
                    }
                }

                // FIGI instrument identifiers
                if let Ok(Some(f)) = rx::get_figi(&conn, &sym_upper) {
                    if !f.identifiers.is_empty() {
                        let _ = writeln!(
                            p,
                            "### Instrument Identifiers (OpenFIGI, as of {})",
                            f.as_of
                        );
                        for id in f.identifiers.iter().take(3) {
                            let _ = writeln!(
                                p,
                                "- **{}** — FIGI {} · share-class {} · {} · {}",
                                id.ticker,
                                if id.figi.is_empty() {
                                    "—".into()
                                } else {
                                    id.figi.clone()
                                },
                                if id.share_class_figi.is_empty() {
                                    "—".into()
                                } else {
                                    id.share_class_figi.clone()
                                },
                                if id.exch_code.is_empty() {
                                    "—".into()
                                } else {
                                    id.exch_code.clone()
                                },
                                if id.security_description.is_empty() {
                                    id.name.clone()
                                } else {
                                    id.security_description.clone()
                                }
                            );
                        }
                        let _ = writeln!(p);
                    }
                }

                // HRA historical return / risk snapshot
                if let Ok(Some(h)) = rx::get_hra(&conn, &sym_upper) {
                    if !h.windows.is_empty() {
                        let _ = writeln!(p, "### Historical Return / Risk (as of {})", h.as_of);
                        let _ = writeln!(
                            p,
                            "- Vol (ann) {:.2}% · Sharpe {:.2} · Sortino {:.2} · Calmar {:.2} · Rf {:.2}%",
                            h.volatility_annual_pct,
                            h.sharpe_ratio,
                            h.sortino_ratio,
                            h.calmar_ratio,
                            h.risk_free_pct
                        );
                        let _ = writeln!(
                            p,
                            "- Max drawdown {:.2}% ({} → {})",
                            h.max_drawdown_pct, h.drawdown_peak_date, h.drawdown_trough_date
                        );
                        let _ = writeln!(p, "| Window | Return | CAGR | N |");
                        let _ = writeln!(p, "|---|---|---|---|");
                        for w in h.windows.iter() {
                            let cagr = if w.cagr_pct == 0.0 {
                                "—".to_string()
                            } else {
                                format!("{:+.2}%", w.cagr_pct)
                            };
                            let _ = writeln!(
                                p,
                                "| {} | {:+.2}% | {} | {} |",
                                w.label, w.return_pct, cagr, w.n_observations
                            );
                        }
                        if !h.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", h.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // DCF fair value (FCFF model)
                if let Ok(Some(d)) = rx::get_dcf(&conn, &sym_upper) {
                    if d.implied_price > 0.0 || !d.note.is_empty() {
                        let _ = writeln!(p, "### DCF (FCFF) Fair Value (as of {})", d.as_of);
                        let _ = writeln!(
                            p,
                            "- Base rev {} · base FCFF {} · margin {:.2}% · growth {:.2}% · tg {:.2}% · WACC {:.2}% · tax {:.2}%",
                            fmt_money(d.base_revenue),
                            fmt_money(d.base_fcff),
                            d.fcff_margin_pct,
                            d.growth_pct,
                            d.terminal_growth_pct,
                            d.wacc_pct,
                            d.tax_rate_pct
                        );
                        let _ = writeln!(
                            p,
                            "- PV explicit FCFF {} · PV terminal {} · EV {}",
                            fmt_money(d.pv_sum),
                            fmt_money(d.pv_terminal),
                            fmt_money(d.enterprise_value)
                        );
                        let _ = writeln!(
                            p,
                            "- (−) Debt {} · (+) Cash {} · equity value {} · shares {:.0}M",
                            fmt_money(d.total_debt),
                            fmt_money(d.cash_and_equivalents),
                            fmt_money(d.equity_value),
                            d.shares_outstanding / 1e6
                        );
                        if d.implied_price > 0.0 {
                            let _ = writeln!(
                                p,
                                "- **Implied price ${:.2}** ({}-year projection)",
                                d.implied_price, d.projection_years
                            );
                        }
                        if !d.years.is_empty() {
                            let _ =
                                writeln!(p, "| Year | Revenue | EBIT | NOPAT | FCFF | PV FCFF |");
                            let _ = writeln!(p, "|---|---|---|---|---|---|");
                            for y in d.years.iter() {
                                let _ = writeln!(
                                    p,
                                    "| {} | {} | {} | {} | {} | {} |",
                                    y.year,
                                    fmt_money(y.revenue),
                                    fmt_money(y.ebit),
                                    fmt_money(y.nopat),
                                    fmt_money(y.fcff),
                                    fmt_money(y.pv_fcff)
                                );
                            }
                        }
                        if !d.note.is_empty() {
                            let _ = writeln!(p, "- Caveat: {}", d.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // SVM multi-model fair value triangulation
                if let Ok(Some(s)) = rx::get_svm(&conn, &sym_upper) {
                    if !s.rows.is_empty() {
                        let _ = writeln!(p, "### Stock Valuation Model (as of {})", s.as_of);
                        let _ = writeln!(
                            p,
                            "- Current ${:.2} · fair mid ${:.2} ({:+.2}%) · range ${:.2}–${:.2}",
                            s.current_price, s.fair_mid, s.upside_mid_pct, s.fair_low, s.fair_high
                        );
                        let _ = writeln!(p, "| Model | Implied | Upside | Confidence | Source |");
                        let _ = writeln!(p, "|---|---|---|---|---|");
                        for r in s.rows.iter() {
                            let _ = writeln!(
                                p,
                                "| {} | ${:.2} | {:+.2}% | {} | {} |",
                                r.model, r.implied_price, r.upside_pct, r.confidence, r.source
                            );
                        }
                        if !s.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", s.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // OMON options chain summary (nearest expiry)
                if let Ok(Some(o)) = rx::get_options_chain(&conn, &sym_upper) {
                    if !o.expirations.is_empty() {
                        let _ = writeln!(p, "### Options Chain (OMON, as of {})", o.as_of);
                        let _ = writeln!(
                            p,
                            "- Underlying ${:.2} · {} expiration(s) cached",
                            o.underlying_price,
                            o.expirations.len()
                        );
                        if let Some(exp) = o.expirations.first() {
                            let total_call_vol: f64 = exp.calls.iter().map(|c| c.volume).sum();
                            let total_put_vol: f64 = exp.puts.iter().map(|p| p.volume).sum();
                            let total_call_oi: f64 =
                                exp.calls.iter().map(|c| c.open_interest).sum();
                            let total_put_oi: f64 = exp.puts.iter().map(|c| c.open_interest).sum();
                            let pcr_vol = if total_call_vol > 0.0 {
                                total_put_vol / total_call_vol
                            } else {
                                0.0
                            };
                            let pcr_oi = if total_call_oi > 0.0 {
                                total_put_oi / total_call_oi
                            } else {
                                0.0
                            };
                            let atm_iv = {
                                let mut all: Vec<_> =
                                    exp.calls.iter().chain(exp.puts.iter()).collect();
                                all.sort_by(|a, b| {
                                    (a.strike - o.underlying_price)
                                        .abs()
                                        .partial_cmp(&(b.strike - o.underlying_price).abs())
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                });
                                all.first()
                                    .map(|c| c.implied_volatility * 100.0)
                                    .unwrap_or(0.0)
                            };
                            let _ = writeln!(
                                p,
                                "- Nearest expiry {} ({} DTE) — {} calls / {} puts",
                                exp.expiration,
                                exp.days_to_expiry,
                                exp.calls.len(),
                                exp.puts.len()
                            );
                            let _ = writeln!(
                                p,
                                "- P/C vol {:.2} · P/C OI {:.2} · ATM IV {:.1}% · call vol {:.0} · put vol {:.0}",
                                pcr_vol, pcr_oi, atm_iv, total_call_vol, total_put_vol
                            );
                            // ATM-zone chain table: 5 strikes below and 5 above underlying, side-by-side calls / puts.
                            let mut strikes: Vec<f64> =
                                exp.calls.iter().map(|c| c.strike).collect();
                            for pt in &exp.puts {
                                if !strikes.iter().any(|s| (s - pt.strike).abs() < 1e-6) {
                                    strikes.push(pt.strike);
                                }
                            }
                            strikes.sort_by(|a, b| {
                                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                            });
                            if !strikes.is_empty() {
                                // Find the strike closest to underlying.
                                let (atm_idx, _) = strikes
                                    .iter()
                                    .enumerate()
                                    .min_by(|(_, a), (_, b)| {
                                        (**a - o.underlying_price)
                                            .abs()
                                            .partial_cmp(&(**b - o.underlying_price).abs())
                                            .unwrap_or(std::cmp::Ordering::Equal)
                                    })
                                    .unwrap_or((0, &0.0));
                                let lo = atm_idx.saturating_sub(5);
                                let hi = (atm_idx + 5).min(strikes.len().saturating_sub(1));
                                let _ = writeln!(
                                    p,
                                    "| Strike | C Last | C IV | C Vol | C OI | P Last | P IV | P Vol | P OI |"
                                );
                                let _ = writeln!(p, "|---|---|---|---|---|---|---|---|---|");
                                for k in &strikes[lo..=hi] {
                                    let c = exp.calls.iter().find(|c| (c.strike - k).abs() < 1e-6);
                                    let pt =
                                        exp.puts.iter().find(|pt| (pt.strike - k).abs() < 1e-6);
                                    let atm_mark = if (k - o.underlying_price).abs()
                                        < (strikes[atm_idx] - o.underlying_price).abs() + 1e-6
                                        && (k - strikes[atm_idx]).abs() < 1e-6
                                    {
                                        "**"
                                    } else {
                                        ""
                                    };
                                    let (cl, civ, cv, coi) = c
                                        .map(|c| {
                                            (
                                                format!("${:.2}", c.last_price),
                                                format!("{:.1}%", c.implied_volatility * 100.0),
                                                format!("{:.0}", c.volume),
                                                format!("{:.0}", c.open_interest),
                                            )
                                        })
                                        .unwrap_or_else(|| {
                                            ("—".into(), "—".into(), "—".into(), "—".into())
                                        });
                                    let (pl, piv, pv, poi) = pt
                                        .map(|p| {
                                            (
                                                format!("${:.2}", p.last_price),
                                                format!("{:.1}%", p.implied_volatility * 100.0),
                                                format!("{:.0}", p.volume),
                                                format!("{:.0}", p.open_interest),
                                            )
                                        })
                                        .unwrap_or_else(|| {
                                            ("—".into(), "—".into(), "—".into(), "—".into())
                                        });
                                    let _ = writeln!(
                                        p,
                                        "| {}${:.2}{} | {} | {} | {} | {} | {} | {} | {} | {} |",
                                        atm_mark, k, atm_mark, cl, civ, cv, coi, pl, piv, pv, poi
                                    );
                                }
                            }
                        }
                        if !o.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", o.note);
                        }
                        let _ = writeln!(p);
                    }
                }

                // IVOL implied-vol rank / percentile
                if let Ok(Some(iv)) = rx::get_ivol(&conn, &sym_upper) {
                    if iv.current_atm_iv_pct > 0.0 || iv.observation_count > 0 {
                        let _ = writeln!(p, "### Implied Vol Rank (as of {})", iv.as_of);
                        let _ = writeln!(
                            p,
                            "- Current ATM IV {:.2}% · 52w range {:.2}%–{:.2}% · rank {:.0} · percentile {:.0} (n={})",
                            iv.current_atm_iv_pct,
                            iv.iv_52w_low_pct,
                            iv.iv_52w_high_pct,
                            iv.iv_rank,
                            iv.iv_percentile,
                            iv.observation_count
                        );
                        if !iv.history.is_empty() {
                            let recent: Vec<String> = iv
                                .history
                                .iter()
                                .rev()
                                .take(8)
                                .map(|h| format!("{}={:.1}%", h.date, h.atm_iv_pct))
                                .collect();
                            let _ = writeln!(p, "- Recent trail: {}", recent.join(" · "));
                        }
                        if !iv.note.is_empty() {
                            let _ = writeln!(p, "- Note: {}", iv.note);
                        }
                        let _ = writeln!(p);
                    }
                }
            }
        }
    }
}
