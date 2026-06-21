//! Pure section/table formatters for the symbol investigation packet.
//!
//! Free functions over engine DTOs — no `TyphooNApp` access — so they stay
//! crate-movable for the future `typhoon-research-ui` crate (ADR-125, Phase 1
//! step 2). The pattern: a `write_*_sections` method gathers the data from app
//! state and hands the resolved DTO to one of these formatters; the formatter
//! itself never touches app state, so its output depends only on its inputs.
//!
//! Each `write_*` mirrors the per-snapshot guard the section method used inline
//! (e.g. "only when WACC > 0"), so a default/empty snapshot emits nothing.

use std::fmt::Write as _;
use typhoon_engine::core::fundamentals::{Fundamentals, format_large_number};
use typhoon_engine::core::research::{
    BetaSnapshot, DcfSnapshot, DdmSnapshot, FigiSnapshot, HraSnapshot, IvolSnapshot,
    OptionsChainSnapshot, RelativeValuation, SvmSnapshot, WaccSnapshot,
};

/// Write the symbol-investigation **overview** block for a resolved
/// [`Fundamentals`] record: the company header line, an optional (length-bounded)
/// description, and the "Valuation & Risk" metric table. Pure — identical
/// fundamentals produce identical markdown.
pub(super) fn write_fundamentals_overview(p: &mut String, f: &Fundamentals) {
    let _ = writeln!(
        p,
        "**{}** — {} / {}",
        if f.company_name.is_empty() {
            "(unnamed)"
        } else {
            f.company_name.as_str()
        },
        if f.sector.is_empty() {
            "Unknown"
        } else {
            f.sector.as_str()
        },
        if f.industry.is_empty() {
            "Unknown"
        } else {
            f.industry.as_str()
        }
    );
    if !f.description.is_empty() {
        // Trim long descriptions to keep the prompt bounded.
        let d = if f.description.len() > 800 {
            &f.description[..800]
        } else {
            f.description.as_str()
        };
        let _ = writeln!(p, "{d}");
    }
    let _ = writeln!(p);
    let _ = writeln!(p, "### Valuation & Risk");
    let fmt_money = format_large_number;
    let fmt_opt = |v: Option<f64>| {
        v.map(|x| format!("{:.2}", x))
            .unwrap_or_else(|| "—".to_string())
    };
    let fmt_money_opt = |v: Option<f64>| v.map(fmt_money).unwrap_or_else(|| "—".to_string());
    let _ = writeln!(p, "| Metric | Value |");
    let _ = writeln!(p, "|---|---|");
    let _ = writeln!(p, "| Market Cap | {} |", fmt_money_opt(f.market_cap));
    let _ = writeln!(
        p,
        "| Enterprise Value | {} |",
        fmt_money_opt(f.enterprise_value)
    );
    let _ = writeln!(p, "| MCap/EV % | {} |", fmt_opt(f.mcap_ev_ratio));
    let _ = writeln!(p, "| Total Debt | {} |", fmt_money_opt(f.total_debt));
    let _ = writeln!(
        p,
        "| Cash & Equivalents | {} |",
        fmt_money_opt(f.cash_and_equivalents)
    );
    let _ = writeln!(p, "| Stock Price | {} |", fmt_opt(f.stock_price));
    let _ = writeln!(p, "| P/E (trailing) | {} |", fmt_opt(f.pe_ratio));
    let _ = writeln!(p, "| Forward P/E | {} |", fmt_opt(f.forward_pe));
    let _ = writeln!(p, "| PEG | {} |", fmt_opt(f.peg_ratio));
    let _ = writeln!(p, "| P/B | {} |", fmt_opt(f.price_to_book));
    let _ = writeln!(p, "| P/S | {} |", fmt_opt(f.price_to_sales));
    let _ = writeln!(p, "| EV/EBITDA | {} |", fmt_opt(f.ev_to_ebitda));
    let _ = writeln!(p, "| Profit Margin | {} |", fmt_opt(f.profit_margin));
    let _ = writeln!(p, "| Operating Margin | {} |", fmt_opt(f.operating_margin));
    let _ = writeln!(p, "| ROE | {} |", fmt_opt(f.roe));
    let _ = writeln!(p, "| ROA | {} |", fmt_opt(f.roa));
    let _ = writeln!(p, "| Beta | {} |", fmt_opt(f.beta));
    let _ = writeln!(p, "| Short Ratio | {} |", fmt_opt(f.short_ratio));
    let _ = writeln!(
        p,
        "| Short % of Float | {} |",
        fmt_opt(f.short_percent_of_float)
    );
    let _ = writeln!(p, "| Dividend Yield | {} |", fmt_opt(f.dividend_yield));
    let _ = writeln!(
        p,
        "| Next Earnings | {} |",
        f.next_earnings_date.clone().unwrap_or_else(|| "—".into())
    );
    let _ = writeln!(p);
}

/// WACC snapshot (CAPM-derived cost of capital). Emits only when `wacc_pct > 0`.
pub(super) fn write_wacc(p: &mut String, w: &WaccSnapshot) {
    if w.wacc_pct > 0.0 {
        let fmt_money = format_large_number;
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

/// Rolling BETA history (1Y / 3Y / 5Y vs the market ticker).
pub(super) fn write_beta(p: &mut String, b: &BetaSnapshot) {
    if !b.windows.is_empty() {
        let _ = writeln!(p, "### Rolling Beta vs {} (as of {})", b.market_ticker, b.as_of);
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

/// DDM Gordon Growth fair value.
pub(super) fn write_ddm(p: &mut String, d: &DdmSnapshot) {
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

/// RV relative-valuation matrix (peer Z-scores).
pub(super) fn write_relative_valuation(p: &mut String, rv: &RelativeValuation) {
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

/// FIGI instrument identifiers (top 3).
pub(super) fn write_figi(p: &mut String, f: &FigiSnapshot) {
    if !f.identifiers.is_empty() {
        let _ = writeln!(p, "### Instrument Identifiers (OpenFIGI, as of {})", f.as_of);
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

/// HRA historical return / risk snapshot.
pub(super) fn write_hra(p: &mut String, h: &HraSnapshot) {
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

/// DCF (FCFF model) fair value.
pub(super) fn write_dcf(p: &mut String, d: &DcfSnapshot) {
    if d.implied_price > 0.0 || !d.note.is_empty() {
        let fmt_money = format_large_number;
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
            let _ = writeln!(p, "| Year | Revenue | EBIT | NOPAT | FCFF | PV FCFF |");
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

/// SVM multi-model fair-value triangulation.
pub(super) fn write_svm(p: &mut String, s: &SvmSnapshot) {
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

/// OMON options-chain summary (nearest expiry + ATM-zone strike table). The
/// put/call ratios, ATM IV, and ATM-window selection are derived purely from the
/// snapshot, so this stays a pure function of `o`.
pub(super) fn write_options_chain(p: &mut String, o: &OptionsChainSnapshot) {
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
            let total_call_oi: f64 = exp.calls.iter().map(|c| c.open_interest).sum();
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
                let mut all: Vec<_> = exp.calls.iter().chain(exp.puts.iter()).collect();
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
            let mut strikes: Vec<f64> = exp.calls.iter().map(|c| c.strike).collect();
            for pt in &exp.puts {
                if !strikes.iter().any(|s| (s - pt.strike).abs() < 1e-6) {
                    strikes.push(pt.strike);
                }
            }
            strikes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
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
                    let pt = exp.puts.iter().find(|pt| (pt.strike - k).abs() < 1e-6);
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
                        .unwrap_or_else(|| ("—".into(), "—".into(), "—".into(), "—".into()));
                    let (pl, piv, pv, poi) = pt
                        .map(|p| {
                            (
                                format!("${:.2}", p.last_price),
                                format!("{:.1}%", p.implied_volatility * 100.0),
                                format!("{:.0}", p.volume),
                                format!("{:.0}", p.open_interest),
                            )
                        })
                        .unwrap_or_else(|| ("—".into(), "—".into(), "—".into(), "—".into()));
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

/// IVOL implied-vol rank / percentile.
pub(super) fn write_ivol(p: &mut String, iv: &IvolSnapshot) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overview_emits_header_and_valuation_table() {
        let f = Fundamentals {
            company_name: "Acme Corp".to_string(),
            sector: "Technology".to_string(),
            industry: "Software".to_string(),
            market_cap: Some(1_500_000_000.0),
            pe_ratio: Some(12.5),
            ..Default::default()
        };
        let mut out = String::new();
        write_fundamentals_overview(&mut out, &f);
        assert!(out.contains("**Acme Corp** — Technology / Software"));
        assert!(out.contains("### Valuation & Risk"));
        assert!(out.contains("| P/E (trailing) | 12.50 |"));
        // Absent optionals render as the em-dash placeholder.
        assert!(out.contains("| ROE | — |"));
    }

    #[test]
    fn overview_uses_placeholders_for_unnamed_fields() {
        let f = Fundamentals::default();
        let mut out = String::new();
        write_fundamentals_overview(&mut out, &f);
        assert!(out.contains("**(unnamed)** — Unknown / Unknown"));
    }

    #[test]
    fn wacc_skips_when_nonpositive_and_emits_when_positive() {
        // The per-snapshot guard moved into the formatter: a default (wacc 0)
        // emits nothing; a positive WACC emits the section.
        let mut out = String::new();
        write_wacc(&mut out, &WaccSnapshot::default());
        assert!(out.is_empty());
        let w = WaccSnapshot {
            wacc_pct: 8.5,
            ..Default::default()
        };
        write_wacc(&mut out, &w);
        assert!(out.contains("### WACC Snapshot"));
        assert!(out.contains("**WACC 8.50%**"));
    }
}
