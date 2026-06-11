use super::*;
mod cached_research;
mod capital_valuation_sections;
mod composite_signal_sections;
mod distribution_risk_sections;
mod fractal_tail_stationarity_sections;
mod fundamental_risk_sections;
mod market_behavior_sections;
mod overview;
mod ownership_price_history;
mod peer_comparison;
mod price_behavior_sections;
mod rank_drift_sections;
mod recent_news;
mod technical_indicator_sections;

impl TyphooNApp {
    pub(super) fn write_symbol_investigation_sections(&self, p: &mut String, syms: &[String]) {
        use std::fmt::Write as _;
        // Per-symbol section
        for sym_raw in syms {
            let sym_upper = sym_raw.to_uppercase();
            let _ = writeln!(p, "---");
            let _ = writeln!(p, "## {sym_upper}");

            self.write_symbol_investigation_overview_sections(p, &sym_upper);
            let fund = self
                .bg
                .all_fundamentals
                .iter()
                .find(|f| f.symbol.eq_ignore_ascii_case(&sym_upper));

            // Quarterly financials (from DB if available)
            if let Some(ref cache) = self.cache {
                if let Some(conn) = cache.try_connection() {
                    if let Ok(quarters) =
                        typhoon_engine::core::fundamentals::get_quarterly_financials(
                            &conn, &sym_upper,
                        )
                    {
                        if !quarters.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Last {} Quarterly Financials",
                                quarters.len().min(4)
                            );
                            let _ = writeln!(
                                p,
                                "| Period | Revenue | Net Income | FCF | Gross Profit | Op Income | EPS |"
                            );
                            let _ = writeln!(p, "|---|---|---|---|---|---|---|");
                            let fmt_money = typhoon_engine::core::fundamentals::format_large_number;
                            let fmt_mopt = |v: Option<f64>| {
                                v.map(fmt_money).unwrap_or_else(|| "—".to_string())
                            };
                            let fmt_opt2 = |v: Option<f64>| {
                                v.map(|x| format!("{:.2}", x))
                                    .unwrap_or_else(|| "—".to_string())
                            };
                            for q in quarters.iter().take(4) {
                                let _ = writeln!(
                                    p,
                                    "| {} | {} | {} | {} | {} | {} | {} |",
                                    q.period_end,
                                    fmt_mopt(q.total_revenue),
                                    fmt_mopt(q.net_income),
                                    fmt_mopt(q.free_cash_flow),
                                    fmt_mopt(q.gross_profit),
                                    fmt_mopt(q.operating_income),
                                    fmt_opt2(q.eps)
                                );
                            }
                            let _ = writeln!(p);
                        }
                    }
                    // Top institutional holders
                    if let Ok(holders) =
                        typhoon_engine::core::fundamentals::get_institutional_holders(
                            &conn, &sym_upper,
                        )
                    {
                        if !holders.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Top {} Institutional Holders",
                                holders.len().min(5)
                            );
                            let _ = writeln!(p, "| Holder | Shares | % Held | Value |");
                            let _ = writeln!(p, "|---|---|---|---|");
                            let fmt_money = typhoon_engine::core::fundamentals::format_large_number;
                            for h in holders.iter().take(5) {
                                let _ = writeln!(
                                    p,
                                    "| {} | {} | {:.2}% | {} |",
                                    h.holder_name,
                                    fmt_money(h.shares as f64),
                                    h.pct_held * 100.0,
                                    fmt_money(h.value)
                                );
                            }
                            let _ = writeln!(p);
                        }
                    }
                }
            }

            // Recent SEC filings
            let recent_filings: Vec<_> = self
                .bg
                .sec_filings
                .iter()
                .filter(|fl| fl.ticker.eq_ignore_ascii_case(&sym_upper))
                .take(10)
                .collect();
            if !recent_filings.is_empty() {
                let _ = writeln!(p, "### Recent SEC Filings ({})", recent_filings.len());
                let _ = writeln!(p, "| Date | Form | Category | Summary |");
                let _ = writeln!(p, "|---|---|---|---|");
                for fl in &recent_filings {
                    let summary = if fl.summary.len() > 120 {
                        &fl.summary[..120]
                    } else {
                        fl.summary.as_str()
                    };
                    let _ = writeln!(
                        p,
                        "| {} | {} | {} | {} |",
                        fl.filing_date, fl.form_type, fl.category, summary
                    );
                }
                let _ = writeln!(p);
            }

            // Insider trade summary (aggregates from bg cache)
            if let Some(trades) = self.bg.insider_trades.get(&sym_upper) {
                if !trades.is_empty() {
                    let n_buys = trades
                        .iter()
                        .filter(|t| {
                            t.transaction_type.eq_ignore_ascii_case("P")
                                || t.transaction_type.to_lowercase().contains("buy")
                        })
                        .count();
                    let n_sells = trades
                        .iter()
                        .filter(|t| {
                            t.transaction_type.eq_ignore_ascii_case("S")
                                || t.transaction_type.to_lowercase().contains("sell")
                        })
                        .count();
                    let buy_value: f64 = trades
                        .iter()
                        .filter(|t| t.transaction_type.eq_ignore_ascii_case("P"))
                        .map(|t| t.aggregate_value)
                        .sum();
                    let sell_value: f64 = trades
                        .iter()
                        .filter(|t| t.transaction_type.eq_ignore_ascii_case("S"))
                        .map(|t| t.aggregate_value)
                        .sum();
                    let net = buy_value - sell_value;
                    let _ = writeln!(p, "### Insider Activity");
                    let fmt_money = typhoon_engine::core::fundamentals::format_large_number;
                    let _ = writeln!(
                        p,
                        "- {} transactions on file ({} buys, {} sells)",
                        trades.len(),
                        n_buys,
                        n_sells
                    );
                    let _ = writeln!(
                        p,
                        "- Buy aggregate: {} | Sell aggregate: {} | Net: {}",
                        fmt_money(buy_value),
                        fmt_money(sell_value),
                        fmt_money(net)
                    );
                    // Show last 5 trades
                    let _ = writeln!(p, "| Date | Insider | Title | Type | Shares | Value |");
                    let _ = writeln!(p, "|---|---|---|---|---|---|");
                    for t in trades.iter().take(5) {
                        let _ = writeln!(
                            p,
                            "| {} | {} | {} | {} | {} | {} |",
                            t.transaction_date,
                            t.insider_name,
                            t.insider_title,
                            t.transaction_type,
                            fmt_money(t.shares),
                            fmt_money(t.aggregate_value)
                        );
                    }
                    let _ = writeln!(p);
                }
            }

            // Price stats from bar cache
            if let Some(ref cache) = self.cache {
                let keys = [
                    format!("kraken-equities:{}:1Day", sym_upper),
                    format!("alpaca:{}:1Day", sym_upper),
                ];
                let mut closes: Vec<f64> = Vec::new();
                let mut ohlc: Vec<(f64, f64, f64, f64)> = Vec::new();
                for key in &keys {
                    if let Ok(Some(bars)) = cache.get_bars_raw(key) {
                        if bars.len() >= 20 {
                            closes = bars.iter().map(|(_, _, _, _, c, _)| *c).collect();
                            ohlc = bars
                                .iter()
                                .map(|(_, o, h, l, c, _)| (*o, *h, *l, *c))
                                .collect();
                            break;
                        }
                    }
                }
                if closes.len() >= 20 {
                    let last = *closes.last().unwrap();
                    let n = closes.len();
                    let ret_pct = |n_back: usize| -> Option<f64> {
                        if n > n_back {
                            let prev = closes[n - 1 - n_back];
                            if prev > 0.0 {
                                Some((last / prev - 1.0) * 100.0)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };
                    let r20 = ret_pct(20);
                    let r60 = ret_pct(60);
                    let r252 = ret_pct(252);
                    // ATR(14)
                    let period = 14usize;
                    let mut atr = 0.0_f64;
                    if ohlc.len() > period {
                        for i in 1..=period {
                            let tr = (ohlc[i].1 - ohlc[i].2)
                                .max((ohlc[i].1 - ohlc[i - 1].3).abs())
                                .max((ohlc[i].2 - ohlc[i - 1].3).abs());
                            atr += tr;
                        }
                        atr /= period as f64;
                        for i in (period + 1)..ohlc.len() {
                            let tr = (ohlc[i].1 - ohlc[i].2)
                                .max((ohlc[i].1 - ohlc[i - 1].3).abs())
                                .max((ohlc[i].2 - ohlc[i - 1].3).abs());
                            atr = (atr * (period as f64 - 1.0) + tr) / period as f64;
                        }
                    }
                    let atr_pct = if last > 0.0 { atr / last * 100.0 } else { 0.0 };
                    // VaR 95% from closes
                    let var95 = typhoon_engine::core::var::compute_var_from_closes(&closes, 0.95)
                        .map(|(dollars, ratio)| format!("${:.2} ({:.2}% of ask)", dollars, ratio))
                        .unwrap_or_else(|| "—".to_string());
                    let _ = writeln!(p, "### Price & Volatility (D1 bars, n={n})");
                    let _ = writeln!(p, "- Last close: **{:.4}**", last);
                    let _ = writeln!(
                        p,
                        "- 20d return: {}",
                        r20.map(|x| format!("{:+.2}%", x))
                            .unwrap_or_else(|| "—".into())
                    );
                    let _ = writeln!(
                        p,
                        "- 60d return: {}",
                        r60.map(|x| format!("{:+.2}%", x))
                            .unwrap_or_else(|| "—".into())
                    );
                    let _ = writeln!(
                        p,
                        "- 252d return: {}",
                        r252.map(|x| format!("{:+.2}%", x))
                            .unwrap_or_else(|| "—".into())
                    );
                    let _ = writeln!(p, "- ATR(14): {:.4} ({:.2}% of price)", atr, atr_pct);
                    let _ = writeln!(p, "- VaR 95% (1 lot): {}", var95);
                    let _ = writeln!(p);
                } else {
                    let _ = writeln!(
                        p,
                        "_No D1 bar data in cache — price/volatility stats unavailable. Run BARDATA to populate._"
                    );
                    let _ = writeln!(p);
                }
            }

            // ── Godel-parity research surfaces (/109/110/111) ─────────
            // Pull cached DVD/EEB/UPDG/FA/MGMT/SPLT/ANR/ESG rows into the packet
            // so the AI has the same data the user sees in the research windows.
            if let Some(ref cache) = self.cache {
                if let Some(conn) = cache.try_connection() {
                    const NEWS_ARTICLE_COUNT: usize = 8;
                    if let Ok(articles) = typhoon_engine::core::news::get_news_by_symbol(
                        &conn,
                        &sym_upper,
                        NEWS_ARTICLE_COUNT,
                    ) {
                        self.write_symbol_recent_news_section(p, &articles);
                    }
                }
            }

            self.write_symbol_cached_research_surfaces(p, &sym_upper);

            if let Some(ref cache) = self.cache {
                if let Some(conn) = cache.try_connection() {
                    use typhoon_engine::core::research as rx;

                    self.write_symbol_ownership_price_history_sections(p, &sym_upper);

                    self.write_symbol_capital_valuation_sections(p, &sym_upper);

                    self.write_symbol_market_behavior_sections(p, &sym_upper);

                    self.write_symbol_fundamental_risk_sections(p, &sym_upper);

                    self.write_symbol_composite_signal_sections(p, &sym_upper);

                    self.write_symbol_rank_drift_sections(p, &sym_upper);

                    self.write_symbol_price_behavior_sections(p, &sym_upper);

                    self.write_symbol_distribution_risk_sections(p, &sym_upper);

                    self.write_symbol_fractal_tail_stationarity_sections(p, &sym_upper);

                    self.write_symbol_technical_indicator_sections(p, &sym_upper);

                    // ── Round 51 research emitters ──
                    if let Ok(Some(dm)) = rx::get_dema(&conn, &sym_upper) {
                        if dm.dema_label != "INSUFFICIENT_DATA" && !dm.dema_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Double EMA — DEMA ({}, as of {})",
                                dm.dema_label, dm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · DEMA {:.4} · close {:.4} · dev {:+.2}%",
                                dm.bars_used,
                                dm.length,
                                dm.dema_value,
                                dm.last_close,
                                dm.deviation_pct
                            );
                            if !dm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", dm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(tm)) = rx::get_tema(&conn, &sym_upper) {
                        if tm.tema_label != "INSUFFICIENT_DATA" && !tm.tema_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Triple EMA — TEMA ({}, as of {})",
                                tm.tema_label, tm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · TEMA {:.4} · close {:.4} · dev {:+.2}%",
                                tm.bars_used,
                                tm.length,
                                tm.tema_value,
                                tm.last_close,
                                tm.deviation_pct
                            );
                            if !tm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", tm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(lr)) = rx::get_linreg(&conn, &sym_upper) {
                        if lr.linreg_label != "INSUFFICIENT_DATA" && !lr.linreg_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Linear Regression Channel — LINREG ({}, as of {})",
                                lr.linreg_label, lr.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · slope {:.5} · intercept {:.4} · R² {:.3} · σ {:.4} · fit {:.4} · ±2σ [{:.4}, {:.4}] · close {:.4}",
                                lr.bars_used,
                                lr.length,
                                lr.slope,
                                lr.intercept,
                                lr.r_squared,
                                lr.sigma,
                                lr.fit_value,
                                lr.channel_lower,
                                lr.channel_upper,
                                lr.last_close
                            );
                            if !lr.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", lr.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(pv)) = rx::get_pivots(&conn, &sym_upper) {
                        if pv.pivots_label != "INSUFFICIENT_DATA" && !pv.pivots_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Floor-Trader Pivots — PIVOTS ({}, as of {})",
                                pv.pivots_label, pv.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · PP {:.4} · R1 {:.4} · R2 {:.4} · S1 {:.4} · S2 {:.4} · prior OHLC [{:.4}/{:.4}/{:.4}] · close {:.4}",
                                pv.bars_used,
                                pv.pp,
                                pv.r1,
                                pv.r2,
                                pv.s1,
                                pv.s2,
                                pv.prior_high,
                                pv.prior_low,
                                pv.prior_close,
                                pv.last_close
                            );
                            if !pv.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", pv.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(hk)) = rx::get_heikin(&conn, &sym_upper) {
                        if hk.heikin_label != "INSUFFICIENT_DATA" && !hk.heikin_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Heikin-Ashi Candle — HEIKIN ({}, as of {})",
                                hk.heikin_label, hk.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · HA_O {:.4} · HA_H {:.4} · HA_L {:.4} · HA_C {:.4} · body {:.4} · wicks [u {:.4} / l {:.4}] · run {}",
                                hk.bars_used,
                                hk.ha_open,
                                hk.ha_high,
                                hk.ha_low,
                                hk.ha_close,
                                hk.body_abs,
                                hk.upper_wick,
                                hk.lower_wick,
                                hk.consecutive_same_color
                            );
                            if !hk.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", hk.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    // ── Round 52 research emitters ──
                    if let Ok(Some(al)) = rx::get_alma(&conn, &sym_upper) {
                        if al.alma_label != "INSUFFICIENT_DATA" && !al.alma_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Arnaud Legoux MA — ALMA ({}, as of {})",
                                al.alma_label, al.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · offset {:.2} · sigma {:.1} · ALMA {:.4} · close {:.4} · dev {:+.2}%",
                                al.bars_used,
                                al.length,
                                al.offset,
                                al.sigma,
                                al.alma_value,
                                al.last_close,
                                al.deviation_pct
                            );
                            if !al.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", al.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(zl)) = rx::get_zlema(&conn, &sym_upper) {
                        if zl.zlema_label != "INSUFFICIENT_DATA" && !zl.zlema_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Zero-Lag EMA — ZLEMA ({}, as of {})",
                                zl.zlema_label, zl.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · lag {} · ZLEMA {:.4} · close {:.4} · dev {:+.2}%",
                                zl.bars_used,
                                zl.length,
                                zl.lag_shift,
                                zl.zlema_value,
                                zl.last_close,
                                zl.deviation_pct
                            );
                            if !zl.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", zl.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(er)) = rx::get_elderray(&conn, &sym_upper) {
                        if er.elder_label != "INSUFFICIENT_DATA" && !er.elder_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Elder Ray Bull/Bear Power — ELDERRAY ({}, as of {})",
                                er.elder_label, er.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · EMA{} {:.4} · Bull {:+.4} (prev {:+.4}) · Bear {:+.4} (prev {:+.4}) · close {:.4}",
                                er.bars_used,
                                er.ema_length,
                                er.ema13,
                                er.bull_power,
                                er.bull_power_prev,
                                er.bear_power,
                                er.bear_power_prev,
                                er.last_close
                            );
                            if !er.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", er.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ts)) = rx::get_tsf(&conn, &sym_upper) {
                        if ts.tsf_label != "INSUFFICIENT_DATA" && !ts.tsf_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Time Series Forecast — TSF ({}, as of {})",
                                ts.tsf_label, ts.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · slope {:+.5} · intercept {:.4} · forecast(t+1) {:.4} · close {:.4} · dev {:+.2}% · R² {:.3}",
                                ts.bars_used,
                                ts.length,
                                ts.slope,
                                ts.intercept,
                                ts.forecast_value,
                                ts.last_close,
                                ts.forecast_deviation_pct,
                                ts.r_squared
                            );
                            if !ts.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ts.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(rv)) = rx::get_rvi(&conn, &sym_upper) {
                        if rv.rvi_label != "INSUFFICIENT_DATA" && !rv.rvi_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Relative Vigor Index — RVI ({}, as of {})",
                                rv.rvi_label, rv.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · RVI {:+.4} (prev {:+.4}) · signal {:+.4} (prev {:+.4}) · close {:.4}",
                                rv.bars_used,
                                rv.length,
                                rv.rvi_value,
                                rv.rvi_prev,
                                rv.signal_value,
                                rv.signal_prev,
                                rv.last_close
                            );
                            if !rv.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", rv.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(tm)) = rx::get_trima(&conn, &sym_upper) {
                        if tm.trima_label != "INSUFFICIENT_DATA" && !tm.trima_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Triangular MA — TRIMA ({}, as of {})",
                                tm.trima_label, tm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · TRIMA {:.4} (prev {:.4}) · deviation {:+.2}% · close {:.4}",
                                tm.bars_used,
                                tm.length,
                                tm.trima_value,
                                tm.trima_prev,
                                tm.deviation_pct,
                                tm.last_close
                            );
                            if !tm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", tm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(t3)) = rx::get_t3(&conn, &sym_upper) {
                        if t3.t3_label != "INSUFFICIENT_DATA" && !t3.t3_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Tillson T3 — T3 ({}, as of {})",
                                t3.t3_label, t3.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · v {:.2} · T3 {:.4} (prev {:.4}) · deviation {:+.2}% · close {:.4}",
                                t3.bars_used,
                                t3.length,
                                t3.v_factor,
                                t3.t3_value,
                                t3.t3_prev,
                                t3.deviation_pct,
                                t3.last_close
                            );
                            if !t3.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", t3.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(vd)) = rx::get_vidya(&conn, &sym_upper) {
                        if vd.vidya_label != "INSUFFICIENT_DATA" && !vd.vidya_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Chande VIDYA — VIDYA ({}, as of {})",
                                vd.vidya_label, vd.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · CMO length {} · VIDYA {:.4} (prev {:.4}) · α {:.4} · |CMO| {:.2} · deviation {:+.2}% · close {:.4}",
                                vd.bars_used,
                                vd.length,
                                vd.cmo_length,
                                vd.vidya_value,
                                vd.vidya_prev,
                                vd.current_alpha,
                                vd.cmo_magnitude,
                                vd.deviation_pct,
                                vd.last_close
                            );
                            if !vd.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", vd.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(sm)) = rx::get_smi(&conn, &sym_upper) {
                        if sm.smi_label != "INSUFFICIENT_DATA" && !sm.smi_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Stochastic Momentum Index — SMI ({}, as of {})",
                                sm.smi_label, sm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · smooth {} · signal {} · SMI {:+.2} (prev {:+.2}) · signal {:+.2} (prev {:+.2}) · close {:.4}",
                                sm.bars_used,
                                sm.length,
                                sm.smooth_length,
                                sm.signal_length,
                                sm.smi_value,
                                sm.smi_prev,
                                sm.signal_value,
                                sm.signal_prev,
                                sm.last_close
                            );
                            if !sm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", sm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(pv)) = rx::get_pvt(&conn, &sym_upper) {
                        if pv.pvt_label != "INSUFFICIENT_DATA" && !pv.pvt_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Price Volume Trend — PVT ({}, as of {})",
                                pv.pvt_label, pv.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · PVT {:.2} (prev {:.2}) · PVT EMA20 {:.2} · 20-bar slope {:+.2} · close {:.4}",
                                pv.bars_used,
                                pv.pvt_value,
                                pv.pvt_prev,
                                pv.pvt_ema,
                                pv.pvt_slope,
                                pv.last_close
                            );
                            if !pv.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", pv.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ac)) = rx::get_ac(&conn, &sym_upper) {
                        if ac.ac_label != "INSUFFICIENT_DATA" && !ac.ac_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Accelerator Oscillator — AC ({}, as of {})",
                                ac.ac_label, ac.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · AC {:+.4} (prev {:+.4}) · AO {:+.4} · AO SMA5 {:+.4} · close {:.4}",
                                ac.bars_used,
                                ac.ac_value,
                                ac.ac_prev,
                                ac.ao_value,
                                ac.ao_sma5,
                                ac.last_close
                            );
                            if !ac.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ac.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cv)) = rx::get_chvol(&conn, &sym_upper) {
                        if cv.chvol_label != "INSUFFICIENT_DATA" && !cv.chvol_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Chaikin Volatility — CHVOL ({}, as of {})",
                                cv.chvol_label, cv.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · EMA length {} · ROC length {} · CHVOL {:+.2}% (prev {:+.2}%) · EMA(H−L) {:.4} · close {:.4}",
                                cv.bars_used,
                                cv.ema_length,
                                cv.roc_length,
                                cv.chvol_value,
                                cv.chvol_prev,
                                cv.ema_range,
                                cv.last_close
                            );
                            if !cv.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cv.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(bw)) = rx::get_bbwidth(&conn, &sym_upper) {
                        if bw.bbw_label != "INSUFFICIENT_DATA" && !bw.bbw_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Bollinger Bandwidth — BBWIDTH ({}, as of {})",
                                bw.bbw_label, bw.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · ±{:.1}σ · BBW {:.4} (prev {:.4}) · 125-bar pct {:.1} · upper {:.4} · mid {:.4} · lower {:.4} · close {:.4}",
                                bw.bars_used,
                                bw.length,
                                bw.num_stdev,
                                bw.bbw_value,
                                bw.bbw_prev,
                                bw.bbw_percentile,
                                bw.upper,
                                bw.middle,
                                bw.lower,
                                bw.last_close
                            );
                            if !bw.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", bw.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ei)) = rx::get_elderimp(&conn, &sym_upper) {
                        if ei.impulse_label != "INSUFFICIENT_DATA" && !ei.impulse_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Elder Impulse System — ELDERIMP ({}, as of {})",
                                ei.impulse_label, ei.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · EMA length {} · EMA {:.4} (slope {:+.4}) · MACD hist {:+.4} (prev {:+.4}, slope {:+.4}) · close {:.4}",
                                ei.bars_used,
                                ei.ema_length,
                                ei.ema_value,
                                ei.ema_slope,
                                ei.macd_hist,
                                ei.macd_hist_prev,
                                ei.macd_hist_slope,
                                ei.last_close
                            );
                            if !ei.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ei.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(rm)) = rx::get_rmi(&conn, &sym_upper) {
                        if rm.rmi_label != "INSUFFICIENT_DATA" && !rm.rmi_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Relative Momentum Index — RMI ({}, as of {})",
                                rm.rmi_label, rm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · momentum {} · RMI {:.2} (prev {:.2}) · close {:.4}",
                                rm.bars_used,
                                rm.length,
                                rm.momentum_length,
                                rm.rmi_value,
                                rm.rmi_prev,
                                rm.last_close
                            );
                            if !rm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", rm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(se)) = rx::get_symbol_expirations(&conn, &sym_upper) {
                        if !se.expirations.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Options Expiration Calendar — EXPCAL ({} expirations, as of {})",
                                se.expirations.len(),
                                se.as_of
                            );
                            if !se.next_triple_witching.is_empty() {
                                let _ = writeln!(
                                    p,
                                    "- Next triple witching: **{}**",
                                    se.next_triple_witching
                                );
                            }
                            let _ = writeln!(p, "- Underlying price: {:.4}", se.underlying_price);
                            for ex in se.expirations.iter().take(12) {
                                let _ = writeln!(
                                    p,
                                    "- **{}** ({} DTE · {}) — {} calls / {} puts · call vol {:.0} · put vol {:.0} · call OI {:.0} · put OI {:.0} · P/C {:.2}",
                                    ex.date,
                                    ex.days_to_expiry,
                                    ex.expiry_type,
                                    ex.call_count,
                                    ex.put_count,
                                    ex.total_call_volume,
                                    ex.total_put_volume,
                                    ex.total_call_oi,
                                    ex.total_put_oi,
                                    ex.put_call_ratio
                                );
                            }
                            if se.expirations.len() > 12 {
                                let _ = writeln!(
                                    p,
                                    "- ({} more expirations, not shown)",
                                    se.expirations.len() - 12
                                );
                            }
                            if !se.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", se.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    // ── Round 55: SMMA / ALLIGATOR / CRSI / SEB / IMI ──
                    if let Ok(Some(sm)) = rx::get_smma(&conn, &sym_upper) {
                        if sm.smma_label != "INSUFFICIENT_DATA" && !sm.smma_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Wilder Smoothed MA — SMMA ({}, as of {})",
                                sm.smma_label, sm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · SMMA {:.4} (prev {:.4}) · deviation {:+.2}% · close {:.4}",
                                sm.bars_used,
                                sm.length,
                                sm.smma_value,
                                sm.smma_prev,
                                sm.deviation_pct,
                                sm.last_close
                            );
                            if !sm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", sm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(al)) = rx::get_alligator(&conn, &sym_upper) {
                        if al.alligator_label != "INSUFFICIENT_DATA"
                            && !al.alligator_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Bill Williams Alligator — ALLIGATOR ({}, as of {})",
                                al.alligator_label, al.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · jaw {:.4} (prev {:.4}) · teeth {:.4} (prev {:.4}) · lips {:.4} (prev {:.4}) · spread {:.2}% · close {:.4}",
                                al.bars_used,
                                al.jaw,
                                al.jaw_prev,
                                al.teeth,
                                al.teeth_prev,
                                al.lips,
                                al.lips_prev,
                                al.spread_pct,
                                al.last_close
                            );
                            if !al.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", al.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cr)) = rx::get_crsi(&conn, &sym_upper) {
                        if cr.crsi_label != "INSUFFICIENT_DATA" && !cr.crsi_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Connors RSI — CRSI ({}, as of {})",
                                cr.crsi_label, cr.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · RSI₃ close {:.2} · RSI₂ streak {:.2} · pct-rank ROC {:.2} · CRSI {:.2} (prev {:.2}) · close {:.4}",
                                cr.bars_used,
                                cr.rsi_close,
                                cr.rsi_streak,
                                cr.percent_rank,
                                cr.crsi_value,
                                cr.crsi_prev,
                                cr.last_close
                            );
                            if !cr.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cr.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(sb)) = rx::get_seb(&conn, &sym_upper) {
                        if sb.seb_label != "INSUFFICIENT_DATA" && !sb.seb_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Standard Error Bands — SEB ({}, as of {})",
                                sb.seb_label, sb.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · ±{:.1}·SE · upper {:.4} · mid {:.4} · lower {:.4} · bandwidth {:.4} · position {:.1}% · close {:.4}",
                                sb.bars_used,
                                sb.length,
                                sb.num_se,
                                sb.upper,
                                sb.middle,
                                sb.lower,
                                sb.bandwidth,
                                sb.position_pct,
                                sb.last_close
                            );
                            if !sb.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", sb.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(im)) = rx::get_imi(&conn, &sym_upper) {
                        if im.imi_label != "INSUFFICIENT_DATA" && !im.imi_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Intraday Momentum Index — IMI ({}, as of {})",
                                im.imi_label, im.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · ΣUp {:.4} · ΣDown {:.4} · IMI {:.2} (prev {:.2}) · close {:.4}",
                                im.bars_used,
                                im.length,
                                im.sum_gains,
                                im.sum_losses,
                                im.imi_value,
                                im.imi_prev,
                                im.last_close
                            );
                            if !im.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", im.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    // ── Round 56: GMMA / MAENV / ADL / VHF / VROC ──
                    if let Ok(Some(gm)) = rx::get_gmma(&conn, &sym_upper) {
                        if gm.gmma_label != "INSUFFICIENT_DATA" && !gm.gmma_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Guppy Multiple MA — GMMA ({}, as of {})",
                                gm.gmma_label, gm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · short-avg {:.4} (min {:.4} max {:.4} comp {:.2}%) · long-avg {:.4} (min {:.4} max {:.4} comp {:.2}%) · group-gap {:+.2}% · close {:.4}",
                                gm.bars_used,
                                gm.short_ema_avg,
                                gm.short_min,
                                gm.short_max,
                                gm.short_compression_pct,
                                gm.long_ema_avg,
                                gm.long_min,
                                gm.long_max,
                                gm.long_compression_pct,
                                gm.group_gap_pct,
                                gm.last_close
                            );
                            if !gm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", gm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(me)) = rx::get_maenv(&conn, &sym_upper) {
                        if me.maenv_label != "INSUFFICIENT_DATA" && !me.maenv_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Moving Average Envelope — MAENV ({}, as of {})",
                                me.maenv_label, me.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · ±{:.2}% · upper {:.4} · mid {:.4} · lower {:.4} · bandwidth {:.2}% · position {:.1}% · close {:.4}",
                                me.bars_used,
                                me.length,
                                me.pct_band,
                                me.upper,
                                me.middle,
                                me.lower,
                                me.bandwidth_pct,
                                me.position_pct,
                                me.last_close
                            );
                            if !me.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", me.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ad)) = rx::get_adl(&conn, &sym_upper) {
                        if ad.adl_label != "INSUFFICIENT_DATA" && !ad.adl_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Accumulation/Distribution Line — ADL ({}, as of {})",
                                ad.adl_label, ad.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · ADL {:.2} (prev {:.2}) · SMA{} {:.2} · slope/bar {:+.2} · price Δ {:+.2}% · close {:.4}",
                                ad.bars_used,
                                ad.adl_value,
                                ad.adl_prev,
                                ad.adl_sma_length,
                                ad.adl_sma,
                                ad.slope_per_bar,
                                ad.price_delta_pct,
                                ad.last_close
                            );
                            if !ad.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ad.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(vh)) = rx::get_vhf(&conn, &sym_upper) {
                        if vh.vhf_label != "INSUFFICIENT_DATA" && !vh.vhf_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Vertical Horizontal Filter — VHF ({}, as of {})",
                                vh.vhf_label, vh.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · HHV {:.4} · LLV {:.4} · Σ|Δc| {:.4} · VHF {:.4} (prev {:.4}) · close {:.4}",
                                vh.bars_used,
                                vh.length,
                                vh.highest_high,
                                vh.lowest_low,
                                vh.sum_abs_delta,
                                vh.vhf_value,
                                vh.vhf_prev,
                                vh.last_close
                            );
                            if !vh.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", vh.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(vr)) = rx::get_vroc(&conn, &sym_upper) {
                        if vr.vroc_label != "INSUFFICIENT_DATA" && !vr.vroc_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Volume Rate of Change — VROC ({}, as of {})",
                                vr.vroc_label, vr.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · V_now {:.0} · V_then {:.0} · VROC {:+.2}% (prev {:+.2}%) · close {:.4}",
                                vr.bars_used,
                                vr.length,
                                vr.volume_now,
                                vr.volume_then,
                                vr.vroc_value,
                                vr.vroc_prev,
                                vr.last_close
                            );
                            if !vr.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", vr.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    // ── Round 57: KDJ / QQE / PMO / CFO / TMF ──
                    if let Ok(Some(kj)) = rx::get_kdj(&conn, &sym_upper) {
                        if kj.kdj_label != "INSUFFICIENT_DATA" && !kj.kdj_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### KDJ — Chinese Stochastic Variant ({}, as of {})",
                                kj.kdj_label, kj.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · stoch {} · smooth {} · RSV {:.2} · K {:.2} · D {:.2} · J {:.2} (prev {:.2}) · close {:.4}",
                                kj.bars_used,
                                kj.stoch_length,
                                kj.k_smooth,
                                kj.rsv,
                                kj.k_value,
                                kj.d_value,
                                kj.j_value,
                                kj.j_prev,
                                kj.last_close
                            );
                            if !kj.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", kj.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(qq)) = rx::get_qqe(&conn, &sym_upper) {
                        if qq.qqe_label != "INSUFFICIENT_DATA" && !qq.qqe_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Quantitative Qualitative Estimation — QQE ({}, as of {})",
                                qq.qqe_label, qq.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · RSI{} · smooth{} · factor {:.3} · RSI {:.2} · smoothed {:.2} (prev {:.2}) · ATR_RSI {:.3} · band [{:.2}, {:.2}] · close {:.4}",
                                qq.bars_used,
                                qq.rsi_length,
                                qq.smooth_length,
                                qq.qqe_factor,
                                qq.rsi_value,
                                qq.rsi_smoothed,
                                qq.qqe_prev,
                                qq.fast_atr_rsi_avg,
                                qq.lower_band,
                                qq.upper_band,
                                qq.last_close
                            );
                            if !qq.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", qq.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(pm)) = rx::get_pmo(&conn, &sym_upper) {
                        if pm.pmo_label != "INSUFFICIENT_DATA" && !pm.pmo_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Price Momentum Oscillator — PMO ({}, as of {})",
                                pm.pmo_label, pm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · smooth1 {} · smooth2 {} · signal {} · PMO {:+.4} (prev {:+.4}) · signal {:+.4} · histogram {:+.4} · close {:.4}",
                                pm.bars_used,
                                pm.smooth1_length,
                                pm.smooth2_length,
                                pm.signal_length,
                                pm.pmo_value,
                                pm.pmo_prev,
                                pm.pmo_signal,
                                pm.histogram,
                                pm.last_close
                            );
                            if !pm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", pm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cf)) = rx::get_cfo(&conn, &sym_upper) {
                        if cf.cfo_label != "INSUFFICIENT_DATA" && !cf.cfo_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Chande Forecast Oscillator — CFO ({}, as of {})",
                                cf.cfo_label, cf.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · slope {:+.6} · intercept {:.4} · forecast {:.4} · CFO {:+.2}% (prev {:+.2}%) · close {:.4}",
                                cf.bars_used,
                                cf.length,
                                cf.slope,
                                cf.intercept,
                                cf.forecast,
                                cf.cfo_value,
                                cf.cfo_prev,
                                cf.last_close
                            );
                            if !cf.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cf.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(tm)) = rx::get_tmf(&conn, &sym_upper) {
                        if tm.tmf_label != "INSUFFICIENT_DATA" && !tm.tmf_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Twiggs Money Flow — TMF ({}, as of {})",
                                tm.tmf_label, tm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · EMA money-flow {:.2} · EMA volume {:.2} · TMF {:+.4} (prev {:+.4}) · close {:.4}",
                                tm.bars_used,
                                tm.length,
                                tm.ema_money_flow,
                                tm.ema_volume,
                                tm.tmf_value,
                                tm.tmf_prev,
                                tm.last_close
                            );
                            if !tm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", tm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(fr)) = rx::get_fractals(&conn, &sym_upper) {
                        if fr.fractals_label != "INSUFFICIENT_DATA" && !fr.fractals_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Bill Williams Fractals — FRACTALS ({}, as of {})",
                                fr.fractals_label, fr.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · window {} · last up {:.4} ({} bars ago) · last down {:.4} ({} bars ago) · up/down count {}/{} · close {:.4}",
                                fr.bars_used,
                                fr.window,
                                fr.last_up_high,
                                fr.last_up_bars_ago,
                                fr.last_down_low,
                                fr.last_down_bars_ago,
                                fr.up_fractal_count,
                                fr.down_fractal_count,
                                fr.last_close
                            );
                            if !fr.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", fr.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ir)) = rx::get_ift_rsi(&conn, &sym_upper) {
                        if ir.ift_rsi_label != "INSUFFICIENT_DATA" && !ir.ift_rsi_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Inverse Fisher RSI — IFT_RSI ({}, as of {})",
                                ir.ift_rsi_label, ir.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · RSI length {} · WMA length {} · RSI {:.2} · v {:+.4} · IFT {:+.4} (prev {:+.4}) · close {:.4}",
                                ir.bars_used,
                                ir.rsi_length,
                                ir.wma_length,
                                ir.rsi_value,
                                ir.v_value,
                                ir.ift_value,
                                ir.ift_prev,
                                ir.last_close
                            );
                            if !ir.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ir.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ma)) = rx::get_mama(&conn, &sym_upper) {
                        if ma.mama_label != "INSUFFICIENT_DATA" && !ma.mama_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### MESA Adaptive MA — MAMA ({}, as of {})",
                                ma.mama_label, ma.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · fast_limit {:.2} · slow_limit {:.2} · MAMA {:.4} (prev {:.4}) · FAMA {:.4} (prev {:.4}) · α {:.4} · period {:.2} · close {:.4}",
                                ma.bars_used,
                                ma.fast_limit,
                                ma.slow_limit,
                                ma.mama_value,
                                ma.mama_prev,
                                ma.fama_value,
                                ma.fama_prev,
                                ma.alpha,
                                ma.period,
                                ma.last_close
                            );
                            if !ma.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ma.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cg)) = rx::get_cog(&conn, &sym_upper) {
                        if cg.cog_label != "INSUFFICIENT_DATA" && !cg.cog_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Ehlers Center of Gravity — COG ({}, as of {})",
                                cg.cog_label, cg.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · COG {:+.4} (prev {:+.4}) · signal {:+.4} · close {:.4}",
                                cg.bars_used,
                                cg.length,
                                cg.cog_value,
                                cg.cog_prev,
                                cg.cog_signal,
                                cg.last_close
                            );
                            if !cg.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cg.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(dd)) = rx::get_didi(&conn, &sym_upper) {
                        if dd.didi_label != "INSUFFICIENT_DATA" && !dd.didi_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Didi Index — DIDI ({}, as of {})",
                                dd.didi_label, dd.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · short/medium/long {}/{}/{} · short ratio {:+.4} (prev {:+.4}) · long ratio {:+.4} (prev {:+.4}) · close {:.4}",
                                dd.bars_used,
                                dd.short_length,
                                dd.medium_length,
                                dd.long_length,
                                dd.short_ratio,
                                dd.short_prev,
                                dd.long_ratio,
                                dd.long_prev,
                                dd.last_close
                            );
                            if !dd.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", dd.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    // ── Round 59: DEMARKER / GATOR / BW_MFI / VWMA / STDDEV ──
                    if let Ok(Some(dm)) = rx::get_demarker(&conn, &sym_upper) {
                        if dm.demarker_label != "INSUFFICIENT_DATA" && !dm.demarker_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### DeMarker — DEMARKER ({}, as of {})",
                                dm.demarker_label, dm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · ΣDeMax {:.4} · ΣDeMin {:.4} · DeM {:.4} (prev {:.4}) · close {:.4}",
                                dm.bars_used,
                                dm.length,
                                dm.demax_sum,
                                dm.demin_sum,
                                dm.demarker_value,
                                dm.demarker_prev,
                                dm.last_close
                            );
                            if !dm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", dm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(gt)) = rx::get_gator(&conn, &sym_upper) {
                        if gt.gator_label != "INSUFFICIENT_DATA" && !gt.gator_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Gator Oscillator — GATOR ({}, as of {})",
                                gt.gator_label, gt.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · jaw/teeth/lips {}/{}/{} · upper {:+.4} (prev {:+.4}) · lower {:+.4} (prev {:+.4}) · close {:.4}",
                                gt.bars_used,
                                gt.jaw_length,
                                gt.teeth_length,
                                gt.lips_length,
                                gt.upper_bar,
                                gt.upper_prev,
                                gt.lower_bar,
                                gt.lower_prev,
                                gt.last_close
                            );
                            if !gt.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", gt.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(bw)) = rx::get_bw_mfi(&conn, &sym_upper) {
                        if bw.bwmfi_label != "INSUFFICIENT_DATA" && !bw.bwmfi_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Bill Williams Market Facilitation Index — BW_MFI ({}, as of {})",
                                bw.bwmfi_label, bw.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · MFI {:.4} (prev {:.4}) · volume {:.0} (prev {:.0}) · color {} · close {:.4}",
                                bw.bars_used,
                                bw.mfi_value,
                                bw.mfi_prev,
                                bw.volume,
                                bw.volume_prev,
                                bw.bwmfi_color,
                                bw.last_close
                            );
                            if !bw.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", bw.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(vw)) = rx::get_vwma(&conn, &sym_upper) {
                        if vw.vwma_label != "INSUFFICIENT_DATA" && !vw.vwma_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Volume-Weighted Moving Average — VWMA ({}, as of {})",
                                vw.vwma_label, vw.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · VWMA {:.4} (prev {:.4}) · SMA {:.4} · spread {:+.4} ({:+.3}%) · close {:.4}",
                                vw.bars_used,
                                vw.length,
                                vw.vwma_value,
                                vw.vwma_prev,
                                vw.sma_value,
                                vw.spread,
                                vw.spread_ratio * 100.0,
                                vw.last_close
                            );
                            if !vw.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", vw.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(sd)) = rx::get_stddev(&conn, &sym_upper) {
                        if sd.regime_label != "INSUFFICIENT_DATA" && !sd.regime_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Rolling Standard Deviation — STDDEV ({}, as of {})",
                                sd.regime_label, sd.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} / long {} · mean {:.4} · σ {:.4} · σ_long {:.4} · cv {:.4} · annualized {:.4} · close {:.4}",
                                sd.bars_used,
                                sd.length,
                                sd.long_length,
                                sd.mean,
                                sd.stddev,
                                sd.stddev_long,
                                sd.cv,
                                sd.annualized,
                                sd.last_close
                            );
                            if !sd.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", sd.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    // ── Round 60: WMA / RAINBOW / MESA_SINE / FRAMA / IBS ──
                    if let Ok(Some(wm)) = rx::get_wma(&conn, &sym_upper) {
                        if wm.wma_label != "INSUFFICIENT_DATA" && !wm.wma_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Weighted Moving Average — WMA ({}, as of {})",
                                wm.wma_label, wm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · WMA {:.4} (prev {:.4}) · SMA {:.4} · spread {:+.4} ({:+.3}%) · close {:.4}",
                                wm.bars_used,
                                wm.length,
                                wm.wma_value,
                                wm.wma_prev,
                                wm.sma_value,
                                wm.spread,
                                wm.spread_pct * 100.0,
                                wm.last_close
                            );
                            if !wm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", wm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(rb)) = rx::get_rainbow(&conn, &sym_upper) {
                        if rb.rainbow_label != "INSUFFICIENT_DATA" && !rb.rainbow_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Rainbow MA Oscillator — RAINBOW ({}, as of {})",
                                rb.rainbow_label, rb.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · levels {} · highest {:.4} · lowest {:.4} · width {:.4} ({:.3}%) · center {:.4} · r1 {:.4} · r5 {:.4} · r10 {:.4} · close {:.4}",
                                rb.bars_used,
                                rb.levels,
                                rb.highest_level,
                                rb.lowest_level,
                                rb.rainbow_width,
                                rb.rainbow_width_pct * 100.0,
                                rb.center_value,
                                rb.r1,
                                rb.r5,
                                rb.r10,
                                rb.last_close
                            );
                            if !rb.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", rb.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ms)) = rx::get_mesa_sine(&conn, &sym_upper) {
                        if ms.mesa_label != "INSUFFICIENT_DATA" && !ms.mesa_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### MESA Sine Wave — MESA_SINE ({}, as of {})",
                                ms.mesa_label, ms.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {:.2} · phase {:+.4} rad · sine {:+.4} (prev {:+.4}) · lead_sine {:+.4} (prev {:+.4}) · close {:.4}",
                                ms.bars_used,
                                ms.period,
                                ms.phase_rad,
                                ms.sine_value,
                                ms.sine_prev,
                                ms.lead_sine,
                                ms.lead_prev,
                                ms.last_close
                            );
                            if !ms.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ms.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(fm)) = rx::get_frama(&conn, &sym_upper) {
                        if fm.frama_label != "INSUFFICIENT_DATA" && !fm.frama_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Fractal Adaptive Moving Average — FRAMA ({}, as of {})",
                                fm.frama_label, fm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · D {:.4} · α {:.4} · FRAMA {:.4} (prev {:.4}) · spread {:+.4} · close {:.4}",
                                fm.bars_used,
                                fm.length,
                                fm.fractal_dim,
                                fm.alpha,
                                fm.frama_value,
                                fm.frama_prev,
                                fm.spread,
                                fm.last_close
                            );
                            if !fm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", fm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ib)) = rx::get_ibs(&conn, &sym_upper) {
                        if ib.ibs_label != "INSUFFICIENT_DATA" && !ib.ibs_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Internal Bar Strength — IBS ({}, as of {})",
                                ib.ibs_label, ib.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · IBS raw {:.4} · smoothed {:.4} (prev {:.4}) · bar H {:.4} L {:.4} C {:.4}",
                                ib.bars_used,
                                ib.length,
                                ib.ibs_raw,
                                ib.ibs_smoothed,
                                ib.ibs_prev,
                                ib.last_high,
                                ib.last_low,
                                ib.last_close
                            );
                            if !ib.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ib.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(lr)) = rx::get_laguerre_rsi(&conn, &sym_upper) {
                        if lr.lrsi_label != "INSUFFICIENT_DATA" && !lr.lrsi_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Laguerre RSI — LAGUERRE_RSI ({}, as of {})",
                                lr.lrsi_label, lr.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · γ {:.2} · L0 {:.6} L1 {:.6} L2 {:.6} L3 {:.6} · LRSI {:.4} (prev {:.4}) · close {:.4}",
                                lr.bars_used,
                                lr.gamma,
                                lr.l0,
                                lr.l1,
                                lr.l2,
                                lr.l3,
                                lr.laguerre_rsi,
                                lr.laguerre_rsi_prev,
                                lr.last_close
                            );
                            if !lr.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", lr.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(zz)) = rx::get_zigzag(&conn, &sym_upper) {
                        if zz.zigzag_label != "INSUFFICIENT_DATA" && !zz.zigzag_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### ZigZag Pattern — ZIGZAG ({}, as of {})",
                                zz.zigzag_label, zz.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · threshold {:.2}% · leg {} · last high {:.4} ({} bars ago) · last low {:.4} ({} bars ago) · reversal at {:.4} · close {:.4}",
                                zz.bars_used,
                                zz.threshold_pct,
                                zz.current_leg,
                                zz.last_high_value,
                                zz.last_high_bars_ago,
                                zz.last_low_value,
                                zz.last_low_bars_ago,
                                zz.reversal_level,
                                zz.last_close
                            );
                            if !zz.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", zz.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(pg)) = rx::get_pgo(&conn, &sym_upper) {
                        if pg.pgo_label != "INSUFFICIENT_DATA" && !pg.pgo_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Pretty Good Oscillator — PGO ({}, as of {})",
                                pg.pgo_label, pg.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · SMA {:.4} · ATR {:.4} · PGO {:.4} (prev {:.4}) · close {:.4}",
                                pg.bars_used,
                                pg.length,
                                pg.sma_value,
                                pg.atr_value,
                                pg.pgo_value,
                                pg.pgo_prev,
                                pg.last_close
                            );
                            if !pg.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", pg.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ht)) = rx::get_ht_trendline(&conn, &sym_upper) {
                        if ht.ht_label != "INSUFFICIENT_DATA" && !ht.ht_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Hilbert Instantaneous Trendline — HT_TRENDLINE ({}, as of {})",
                                ht.ht_label, ht.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · detected period {:.2} · trendline {:.4} (prev {:.4}) · spread {:.4} ({:+.3}%) · close {:.4}",
                                ht.bars_used,
                                ht.period,
                                ht.trendline_value,
                                ht.trendline_prev,
                                ht.spread,
                                ht.spread_pct * 100.0,
                                ht.last_close
                            );
                            if !ht.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ht.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(mp)) = rx::get_midpoint(&conn, &sym_upper) {
                        if mp.midpoint_label != "INSUFFICIENT_DATA" && !mp.midpoint_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Midpoint of N — MIDPOINT ({}, as of {})",
                                mp.midpoint_label, mp.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · HHV {:.4} · LLV {:.4} · midpoint {:.4} (prev {:.4}) · close position {:.4} · close {:.4}",
                                mp.bars_used,
                                mp.length,
                                mp.hhv,
                                mp.llv,
                                mp.midpoint,
                                mp.midpoint_prev,
                                mp.close_position,
                                mp.last_close
                            );
                            if !mp.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", mp.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    // ── Round 62: MASSINDEX / NATR / TTM_SQUEEZE / FORCE_INDEX / TRANGE ──
                    if let Ok(Some(mi)) = rx::get_mass_index(&conn, &sym_upper) {
                        if mi.mass_label != "INSUFFICIENT_DATA" && !mi.mass_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Dorsey Mass Index — MASSINDEX ({}, as of {})",
                                mi.mass_label, mi.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · EMA len {} · sum len {} · EMA(H-L) {:.4} · EMA-of-EMA {:.4} · ratio {:.4} · MI {:.2} (prev {:.2}) · close {:.4}",
                                mi.bars_used,
                                mi.ema_len,
                                mi.sum_len,
                                mi.ema_range,
                                mi.ema_ema_range,
                                mi.ratio,
                                mi.mass_index,
                                mi.mass_index_prev,
                                mi.last_close
                            );
                            if !mi.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", mi.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(na)) = rx::get_natr(&conn, &sym_upper) {
                        if na.natr_label != "INSUFFICIENT_DATA" && !na.natr_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Normalized ATR — NATR ({}, as of {})",
                                na.natr_label, na.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · ATR {:.4} · NATR {:.4}% (prev {:.4}%) · close {:.4}",
                                na.bars_used,
                                na.length,
                                na.atr_value,
                                na.natr_value,
                                na.natr_prev,
                                na.last_close
                            );
                            if !na.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", na.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(tq)) = rx::get_ttm_squeeze(&conn, &sym_upper) {
                        if tq.squeeze_label != "INSUFFICIENT_DATA" && !tq.squeeze_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### TTM Squeeze — TTM_SQUEEZE ({}, as of {})",
                                tq.squeeze_label, tq.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · BB [{:.4} .. {:.4}] · KC [{:.4} .. {:.4}] · squeeze_on {} · momentum {:+.4} (prev {:+.4}) · close {:.4}",
                                tq.bars_used,
                                tq.length,
                                tq.bb_lower,
                                tq.bb_upper,
                                tq.kc_lower,
                                tq.kc_upper,
                                tq.squeeze_on,
                                tq.momentum,
                                tq.momentum_prev,
                                tq.last_close
                            );
                            if !tq.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", tq.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(fi)) = rx::get_force_index(&conn, &sym_upper) {
                        if fi.force_label != "INSUFFICIENT_DATA" && !fi.force_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Elder Force Index — FORCE_INDEX ({}, as of {})",
                                fi.force_label, fi.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · raw {:.2} · EMA {:.2} (prev {:.2}) · volume {:.0} · close {:.4}",
                                fi.bars_used,
                                fi.length,
                                fi.force_raw,
                                fi.force_ema,
                                fi.force_ema_prev,
                                fi.last_volume,
                                fi.last_close
                            );
                            if !fi.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", fi.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(tr)) = rx::get_trange(&conn, &sym_upper) {
                        if tr.trange_label != "INSUFFICIENT_DATA" && !tr.trange_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### True Range (raw) — TRANGE ({}, as of {})",
                                tr.trange_label, tr.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · TR {:.4} (prev {:.4}) · mean(20) {:.4} · ratio {:.3} · H {:.4} · L {:.4} · prev close {:.4} · close {:.4}",
                                tr.bars_used,
                                tr.trange_value,
                                tr.trange_prev,
                                tr.mean_trange_20,
                                tr.trange_ratio,
                                tr.last_high,
                                tr.last_low,
                                tr.prev_close,
                                tr.last_close
                            );
                            if !tr.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", tr.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    // ── Round 63 packet emitters ──
                    if let Ok(Some(ls)) = rx::get_linearreg_slope(&conn, &sym_upper) {
                        if ls.slope_label != "INSUFFICIENT_DATA" && !ls.slope_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Linear Regression Slope — LINEARREG_SLOPE ({}, as of {})",
                                ls.slope_label, ls.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · slope {:+.6} (prev {:+.6}) · slope_pct {:+.3}% · close {:.4}",
                                ls.bars_used,
                                ls.length,
                                ls.slope,
                                ls.slope_prev,
                                ls.slope_pct,
                                ls.last_close
                            );
                            if !ls.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ls.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(dc)) = rx::get_ht_dcperiod(&conn, &sym_upper) {
                        if dc.period_label != "INSUFFICIENT_DATA" && !dc.period_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Hilbert Dominant Cycle Period — HT_DCPERIOD ({}, as of {})",
                                dc.period_label, dc.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {:.2} (prev {:.2}) · min(64) {:.2} · max(64) {:.2} · close {:.4}",
                                dc.bars_used,
                                dc.period,
                                dc.period_prev,
                                dc.period_min_64,
                                dc.period_max_64,
                                dc.last_close
                            );
                            if !dc.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", dc.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(tm)) = rx::get_ht_trendmode(&conn, &sym_upper) {
                        if tm.mode_label != "INSUFFICIENT_DATA" && !tm.mode_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Hilbert Trend vs Cycle Mode — HT_TRENDMODE ({}, as of {})",
                                tm.mode_label, tm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · trendmode {} (prev {}) · lock_in_bars {} · period {:.2} · close {:.4}",
                                tm.bars_used,
                                tm.trendmode,
                                tm.trendmode_prev,
                                tm.lock_in_bars,
                                tm.period,
                                tm.last_close
                            );
                            if !tm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", tm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ab)) = rx::get_accbands(&conn, &sym_upper) {
                        if ab.accbands_label != "INSUFFICIENT_DATA" && !ab.accbands_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Acceleration Bands — ACCBANDS ({}, as of {})",
                                ab.accbands_label, ab.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · upper {:.4} · middle {:.4} · lower {:.4} · width {:.4} · pos {:.3} · close {:.4}",
                                ab.bars_used,
                                ab.length,
                                ab.acc_upper,
                                ab.acc_middle,
                                ab.acc_lower,
                                ab.width,
                                ab.position,
                                ab.last_close
                            );
                            if !ab.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ab.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(sf)) = rx::get_stochf(&conn, &sym_upper) {
                        if sf.stochf_label != "INSUFFICIENT_DATA" && !sf.stochf_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Fast Stochastic — STOCHF ({}, as of {})",
                                sf.stochf_label, sf.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · d_period {} · fastK {:.2} (prev {:.2}) · fastD {:.2} (prev {:.2}) · close {:.4}",
                                sf.bars_used,
                                sf.length,
                                sf.d_period,
                                sf.fastk,
                                sf.fastk_prev,
                                sf.fastd,
                                sf.fastd_prev,
                                sf.last_close
                            );
                            if !sf.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", sf.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    // ── Round 64 packet emitters ──
                    if let Ok(Some(lr)) = rx::get_linearreg(&conn, &sym_upper) {
                        if lr.linearreg_label != "INSUFFICIENT_DATA"
                            && !lr.linearreg_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Linear Regression — LINEARREG ({}, as of {})",
                                lr.linearreg_label, lr.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · fitted {:.4} (prev {:.4}) · residual {:+.4} · residual_pct {:+.3}% · close {:.4}",
                                lr.bars_used,
                                lr.length,
                                lr.fitted,
                                lr.fitted_prev,
                                lr.residual,
                                lr.residual_pct,
                                lr.last_close
                            );
                            if !lr.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", lr.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(la)) = rx::get_linearreg_angle(&conn, &sym_upper) {
                        if la.angle_label != "INSUFFICIENT_DATA" && !la.angle_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Linear Regression Angle — LINEARREG_ANGLE ({}, as of {})",
                                la.angle_label, la.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · slope {:+.6} · angle {:+.3}° (prev {:+.3}°) · close {:.4}",
                                la.bars_used,
                                la.length,
                                la.slope,
                                la.angle_deg,
                                la.angle_deg_prev,
                                la.last_close
                            );
                            if !la.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", la.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(dp)) = rx::get_ht_dcphase(&conn, &sym_upper) {
                        if dp.phase_label != "INSUFFICIENT_DATA" && !dp.phase_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Hilbert Dominant Cycle Phase — HT_DCPHASE ({}, as of {})",
                                dp.phase_label, dp.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · phase {:.2}° (prev {:.2}°) · delta {:+.2}° · period {:.2} · close {:.4}",
                                dp.bars_used,
                                dp.phase_deg,
                                dp.phase_deg_prev,
                                dp.phase_delta,
                                dp.period,
                                dp.last_close
                            );
                            if !dp.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", dp.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(hs)) = rx::get_ht_sine(&conn, &sym_upper) {
                        if hs.sine_label != "INSUFFICIENT_DATA" && !hs.sine_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Hilbert Sine Wave — HT_SINE ({}, as of {})",
                                hs.sine_label, hs.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · sine {:+.3} (prev {:+.3}) · leadsine {:+.3} (prev {:+.3}) · crossover {} · period {:.2} · close {:.4}",
                                hs.bars_used,
                                hs.sine,
                                hs.sine_prev,
                                hs.leadsine,
                                hs.leadsine_prev,
                                hs.crossover,
                                hs.period,
                                hs.last_close
                            );
                            if !hs.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", hs.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(hp)) = rx::get_ht_phasor(&conn, &sym_upper) {
                        if hp.phasor_label != "INSUFFICIENT_DATA" && !hp.phasor_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Hilbert Phasor — HT_PHASOR ({}, as of {})",
                                hp.phasor_label, hp.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · I {:+.4} (prev {:+.4}) · Q {:+.4} (prev {:+.4}) · magnitude {:.4} · phase {:+.2}° · close {:.4}",
                                hp.bars_used,
                                hp.i_comp,
                                hp.i_prev,
                                hp.q_comp,
                                hp.q_prev,
                                hp.magnitude,
                                hp.phase_deg,
                                hp.last_close
                            );
                            if !hp.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", hp.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(mp)) = rx::get_midprice(&conn, &sym_upper) {
                        if mp.midprice_label != "INSUFFICIENT_DATA" && !mp.midprice_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Range Midpoint — MIDPRICE ({}, as of {})",
                                mp.midprice_label, mp.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · length {} · midprice {:.4} (prev {:.4}) · HHV {:.4} · LLV {:.4} · position {:.3} · close {:.4}",
                                mp.bars_used,
                                mp.length,
                                mp.midprice,
                                mp.midprice_prev,
                                mp.hhv,
                                mp.llv,
                                mp.position,
                                mp.last_close
                            );
                            if !mp.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", mp.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ap)) = rx::get_apo(&conn, &sym_upper) {
                        if ap.apo_label != "INSUFFICIENT_DATA" && !ap.apo_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Absolute Price Oscillator — APO ({}, as of {})",
                                ap.apo_label, ap.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · fast {} · slow {} · APO {:+.4} (prev {:+.4}) · fast_EMA {:.4} · slow_EMA {:.4} · close {:.4}",
                                ap.bars_used,
                                ap.fast_period,
                                ap.slow_period,
                                ap.apo,
                                ap.apo_prev,
                                ap.fast_ema,
                                ap.slow_ema,
                                ap.last_close
                            );
                            if !ap.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ap.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(mm)) = rx::get_mom(&conn, &sym_upper) {
                        if mm.mom_label != "INSUFFICIENT_DATA" && !mm.mom_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Raw Momentum — MOM ({}, as of {})",
                                mm.mom_label, mm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · MOM {:+.4} (prev {:+.4}) · MOM% {:+.3} · close {:.4}",
                                mm.bars_used,
                                mm.period,
                                mm.mom,
                                mm.mom_prev,
                                mm.mom_pct,
                                mm.last_close
                            );
                            if !mm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", mm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(sx)) = rx::get_sarext(&conn, &sym_upper) {
                        if sx.sarext_label != "INSUFFICIENT_DATA" && !sx.sarext_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Extended Parabolic SAR — SAREXT ({}, as of {})",
                                sx.sarext_label, sx.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · AF long init/step/max {:.3}/{:.3}/{:.3} · AF short init/step/max {:.3}/{:.3}/{:.3} · SAR {:.4} · EP {:.4} · AF {:.3} · trend {} · in-trend {} · distance {:+.3}% · close {:.4}",
                                sx.bars_used,
                                sx.af_init_long,
                                sx.af_step_long,
                                sx.af_max_long,
                                sx.af_init_short,
                                sx.af_step_short,
                                sx.af_max_short,
                                sx.sar_value,
                                sx.extreme_point,
                                sx.acceleration_factor,
                                if sx.trend_is_up { "UP" } else { "DOWN" },
                                sx.bars_in_trend,
                                sx.distance_pct,
                                sx.last_close
                            );
                            if !sx.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", sx.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ar)) = rx::get_adxr(&conn, &sym_upper) {
                        if ar.adxr_label != "INSUFFICIENT_DATA" && !ar.adxr_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### ADX Rating — ADXR ({}, as of {})",
                                ar.adxr_label, ar.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · ADX now {:.3} · ADX prior {:.3} · ADXR {:.3} (prev {:.3}) · close {:.4}",
                                ar.bars_used,
                                ar.period,
                                ar.adx_now,
                                ar.adx_prior,
                                ar.adxr,
                                ar.adxr_prev,
                                ar.last_close
                            );
                            if !ar.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ar.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    // ── Round 66 packet emitters ──
                    if let Ok(Some(ap)) = rx::get_avgprice(&conn, &sym_upper) {
                        if ap.avgprice_label != "INSUFFICIENT_DATA" && !ap.avgprice_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### OHLC Average — AVGPRICE ({}, as of {})",
                                ap.avgprice_label, ap.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · avgprice {:.4} (prev {:.4}) · O {:.4} · H {:.4} · L {:.4} · C {:.4} · Δ {:+.3}%",
                                ap.bars_used,
                                ap.avgprice,
                                ap.avgprice_prev,
                                ap.open,
                                ap.high,
                                ap.low,
                                ap.close,
                                ap.delta_pct
                            );
                            if !ap.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ap.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(mp)) = rx::get_medprice(&conn, &sym_upper) {
                        if mp.medprice_label != "INSUFFICIENT_DATA" && !mp.medprice_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Range Median — MEDPRICE ({}, as of {})",
                                mp.medprice_label, mp.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · medprice {:.4} (prev {:.4}) · H {:.4} · L {:.4} · C {:.4} · Δ {:+.3}%",
                                mp.bars_used,
                                mp.medprice,
                                mp.medprice_prev,
                                mp.high,
                                mp.low,
                                mp.close,
                                mp.delta_pct
                            );
                            if !mp.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", mp.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(tp)) = rx::get_typprice(&conn, &sym_upper) {
                        if tp.typprice_label != "INSUFFICIENT_DATA" && !tp.typprice_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Typical Price — TYPPRICE ({}, as of {})",
                                tp.typprice_label, tp.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · typprice {:.4} (prev {:.4}) · H {:.4} · L {:.4} · C {:.4} · Δ {:+.3}%",
                                tp.bars_used,
                                tp.typprice,
                                tp.typprice_prev,
                                tp.high,
                                tp.low,
                                tp.close,
                                tp.delta_pct
                            );
                            if !tp.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", tp.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(wp)) = rx::get_wclprice(&conn, &sym_upper) {
                        if wp.wclprice_label != "INSUFFICIENT_DATA" && !wp.wclprice_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Weighted Close — WCLPRICE ({}, as of {})",
                                wp.wclprice_label, wp.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · wclprice {:.4} (prev {:.4}) · H {:.4} · L {:.4} · C {:.4} · Δ {:+.3}%",
                                wp.bars_used,
                                wp.wclprice,
                                wp.wclprice_prev,
                                wp.high,
                                wp.low,
                                wp.close,
                                wp.delta_pct
                            );
                            if !wp.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", wp.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(vr)) = rx::get_variance(&conn, &sym_upper) {
                        if vr.variance_label != "INSUFFICIENT_DATA" && !vr.variance_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Close Variance — VARIANCE ({}, as of {})",
                                vr.variance_label, vr.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · mean {:.4} · variance {:.6} (prev {:.6}) · stddev {:.4} · CV {:.3}% · close {:.4}",
                                vr.bars_used,
                                vr.period,
                                vr.mean,
                                vr.variance,
                                vr.variance_prev,
                                vr.stddev,
                                vr.cv,
                                vr.last_close
                            );
                            if !vr.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", vr.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    // ── Round 67 packet emitters (DMI family) ──
                    if let Ok(Some(pd)) = rx::get_plus_di(&conn, &sym_upper) {
                        if pd.plus_di_label != "INSUFFICIENT_DATA" && !pd.plus_di_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Positive Directional Indicator — PLUS_DI ({}, as of {})",
                                pd.plus_di_label, pd.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · +DI {:.3} (prev {:.3}) · -DI {:.3} · ATR {:.4} · close {:.4}",
                                pd.bars_used,
                                pd.period,
                                pd.plus_di,
                                pd.plus_di_prev,
                                pd.minus_di,
                                pd.atr,
                                pd.last_close
                            );
                            if !pd.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", pd.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(md)) = rx::get_minus_di(&conn, &sym_upper) {
                        if md.minus_di_label != "INSUFFICIENT_DATA" && !md.minus_di_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Negative Directional Indicator — MINUS_DI ({}, as of {})",
                                md.minus_di_label, md.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · -DI {:.3} (prev {:.3}) · +DI {:.3} · ATR {:.4} · close {:.4}",
                                md.bars_used,
                                md.period,
                                md.minus_di,
                                md.minus_di_prev,
                                md.plus_di,
                                md.atr,
                                md.last_close
                            );
                            if !md.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", md.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(pm)) = rx::get_plus_dm(&conn, &sym_upper) {
                        if pm.plus_dm_label != "INSUFFICIENT_DATA" && !pm.plus_dm_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Positive Directional Movement — PLUS_DM ({}, as of {})",
                                pm.plus_dm_label, pm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · +DM raw {:.4} · +DM smoothed {:.4} (prev {:.4}) · up {:+.4} · dn {:+.4} · close {:.4}",
                                pm.bars_used,
                                pm.period,
                                pm.plus_dm_raw,
                                pm.plus_dm_smoothed,
                                pm.plus_dm_smoothed_prev,
                                pm.up_move,
                                pm.down_move,
                                pm.last_close
                            );
                            if !pm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", pm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(mm)) = rx::get_minus_dm(&conn, &sym_upper) {
                        if mm.minus_dm_label != "INSUFFICIENT_DATA" && !mm.minus_dm_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Negative Directional Movement — MINUS_DM ({}, as of {})",
                                mm.minus_dm_label, mm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · -DM raw {:.4} · -DM smoothed {:.4} (prev {:.4}) · up {:+.4} · dn {:+.4} · close {:.4}",
                                mm.bars_used,
                                mm.period,
                                mm.minus_dm_raw,
                                mm.minus_dm_smoothed,
                                mm.minus_dm_smoothed_prev,
                                mm.up_move,
                                mm.down_move,
                                mm.last_close
                            );
                            if !mm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", mm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(dxr)) = rx::get_dx(&conn, &sym_upper) {
                        if dxr.dx_label != "INSUFFICIENT_DATA" && !dxr.dx_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Directional Movement Index — DX ({}, as of {})",
                                dxr.dx_label, dxr.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · DX {:.3} (prev {:.3}) · +DI {:.3} · -DI {:.3} · close {:.4}",
                                dxr.bars_used,
                                dxr.period,
                                dxr.dx,
                                dxr.dx_prev,
                                dxr.plus_di,
                                dxr.minus_di,
                                dxr.last_close
                            );
                            if !dxr.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", dxr.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    // ── Round 68 ──
                    if let Ok(Some(rc)) = rx::get_roc(&conn, &sym_upper) {
                        if rc.roc_label != "INSUFFICIENT_DATA" && !rc.roc_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Rate of Change — ROC ({}, as of {})",
                                rc.roc_label, rc.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · ROC {:+.4} (prev {:+.4}) · close {:.4} · lag {:.4}",
                                rc.bars_used,
                                rc.period,
                                rc.roc,
                                rc.roc_prev,
                                rc.close_now,
                                rc.close_lag
                            );
                            if !rc.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", rc.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(rcp)) = rx::get_rocp(&conn, &sym_upper) {
                        if rcp.rocp_label != "INSUFFICIENT_DATA" && !rcp.rocp_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Rate of Change Percentage — ROCP ({}, as of {})",
                                rcp.rocp_label, rcp.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · ROCP {:+.6} ({:+.4}%) · prev {:+.6} · close {:.4} · lag {:.4}",
                                rcp.bars_used,
                                rcp.period,
                                rcp.rocp,
                                rcp.rocp_pct,
                                rcp.rocp_prev,
                                rcp.close_now,
                                rcp.close_lag
                            );
                            if !rcp.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", rcp.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(rcr)) = rx::get_rocr(&conn, &sym_upper) {
                        if rcr.rocr_label != "INSUFFICIENT_DATA" && !rcr.rocr_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Rate of Change Ratio — ROCR ({}, as of {})",
                                rcr.rocr_label, rcr.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · ROCR {:.6} (prev {:.6}) · close {:.4} · lag {:.4}",
                                rcr.bars_used,
                                rcr.period,
                                rcr.rocr,
                                rcr.rocr_prev,
                                rcr.close_now,
                                rcr.close_lag
                            );
                            if !rcr.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", rcr.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(rc1)) = rx::get_rocr100(&conn, &sym_upper) {
                        if rc1.rocr100_label != "INSUFFICIENT_DATA" && !rc1.rocr100_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Rate of Change Ratio ×100 — ROCR100 ({}, as of {})",
                                rc1.rocr100_label, rc1.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · ROCR100 {:.4} (prev {:.4}) · close {:.4} · lag {:.4}",
                                rc1.bars_used,
                                rc1.period,
                                rc1.rocr100,
                                rc1.rocr100_prev,
                                rc1.close_now,
                                rc1.close_lag
                            );
                            if !rc1.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", rc1.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cr)) = rx::get_correl(&conn, &sym_upper) {
                        if cr.correl_label != "INSUFFICIENT_DATA" && !cr.correl_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Lag-1 Autocorrelation — CORREL ({}, as of {})",
                                cr.correl_label, cr.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · ρ {:+.4} (prev {:+.4}) · mean(x) {:.4} · mean(y) {:.4} · σ(x) {:.4} · σ(y) {:.4} · close {:.4}",
                                cr.bars_used,
                                cr.period,
                                cr.correl,
                                cr.correl_prev,
                                cr.mean_x,
                                cr.mean_y,
                                cr.stddev_x,
                                cr.stddev_y,
                                cr.last_close
                            );
                            if !cr.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cr.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(mn)) = rx::get_min(&conn, &sym_upper) {
                        if mn.min_label != "INSUFFICIENT_DATA" && !mn.min_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Rolling Minimum — MIN ({}, as of {})",
                                mn.min_label, mn.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · min {:.4} (prev {:.4}) · max_ref {:.4} · close {:.4} · pos {:.2}%",
                                mn.bars_used,
                                mn.period,
                                mn.min_val,
                                mn.min_prev,
                                mn.max_ref,
                                mn.last_close,
                                mn.position_pct
                            );
                            if !mn.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", mn.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(mx)) = rx::get_max(&conn, &sym_upper) {
                        if mx.max_label != "INSUFFICIENT_DATA" && !mx.max_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Rolling Maximum — MAX ({}, as of {})",
                                mx.max_label, mx.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · max {:.4} (prev {:.4}) · min_ref {:.4} · close {:.4} · pos {:.2}%",
                                mx.bars_used,
                                mx.period,
                                mx.max_val,
                                mx.max_prev,
                                mx.min_ref,
                                mx.last_close,
                                mx.position_pct
                            );
                            if !mx.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", mx.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(mm)) = rx::get_minmax(&conn, &sym_upper) {
                        if mm.minmax_label != "INSUFFICIENT_DATA" && !mm.minmax_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Rolling Range — MINMAX ({}, as of {})",
                                mm.minmax_label, mm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · [{:.4}..{:.4}] · width {:.4} · width% {:.2}% · close {:.4} · pos {:.2}%",
                                mm.bars_used,
                                mm.period,
                                mm.min_val,
                                mm.max_val,
                                mm.range_width,
                                mm.range_pct,
                                mm.last_close,
                                mm.position_pct
                            );
                            if !mm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", mm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(mi)) = rx::get_minindex(&conn, &sym_upper) {
                        if mi.min_index_label != "INSUFFICIENT_DATA"
                            && !mi.min_index_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Low Recency — MININDEX ({}, as of {})",
                                mi.min_index_label, mi.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · min {:.4} · {} bars ago (prev {} bars ago) · close {:.4}",
                                mi.bars_used,
                                mi.period,
                                mi.min_val,
                                mi.min_index_bars_ago,
                                mi.min_index_bars_ago_prev,
                                mi.last_close
                            );
                            if !mi.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", mi.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(mxi)) = rx::get_maxindex(&conn, &sym_upper) {
                        if mxi.max_index_label != "INSUFFICIENT_DATA"
                            && !mxi.max_index_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### High Recency — MAXINDEX ({}, as of {})",
                                mxi.max_index_label, mxi.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · max {:.4} · {} bars ago (prev {} bars ago) · close {:.4}",
                                mxi.bars_used,
                                mxi.period,
                                mxi.max_val,
                                mxi.max_index_bars_ago,
                                mxi.max_index_bars_ago_prev,
                                mxi.last_close
                            );
                            if !mxi.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", mxi.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(bb)) = rx::get_bbands(&conn, &sym_upper) {
                        if bb.bbands_label != "INSUFFICIENT_DATA" && !bb.bbands_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Bollinger Bands — BBANDS ({}, as of {})",
                                bb.bbands_label, bb.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · σ×{:.1} · upper {:.4} · mid {:.4} · lower {:.4} · close {:.4} · %B {:.2} · bw {:.2}%",
                                bb.bars_used,
                                bb.period,
                                bb.num_std,
                                bb.upper,
                                bb.middle,
                                bb.lower,
                                bb.last_close,
                                bb.pct_b,
                                bb.bandwidth
                            );
                            if !bb.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", bb.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ad)) = rx::get_ad(&conn, &sym_upper) {
                        if ad.ad_label != "INSUFFICIENT_DATA" && !ad.ad_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Chaikin A/D Line — AD ({}, as of {})",
                                ad.ad_label, ad.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · AD {:.4} (prev {:.4}, Δ {:+.4}) · slope10 {:+.6} · close {:.4}",
                                ad.bars_used,
                                ad.ad,
                                ad.ad_prev,
                                ad.ad_delta,
                                ad.ad_slope,
                                ad.last_close
                            );
                            if !ad.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ad.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ao)) = rx::get_adosc(&conn, &sym_upper) {
                        if ao.adosc_label != "INSUFFICIENT_DATA" && !ao.adosc_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Chaikin A/D Oscillator — ADOSC ({}, as of {})",
                                ao.adosc_label, ao.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · fast {} · slow {} · ADOSC {:+.4} (prev {:+.4}) · AD ref {:.4} · close {:.4}",
                                ao.bars_used,
                                ao.fast_period,
                                ao.slow_period,
                                ao.adosc,
                                ao.adosc_prev,
                                ao.ad_ref,
                                ao.last_close
                            );
                            if !ao.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ao.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(su)) = rx::get_sum(&conn, &sym_upper) {
                        if su.sum_label != "INSUFFICIENT_DATA" && !su.sum_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Rolling Sum — SUM ({}, as of {})",
                                su.sum_label, su.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · sum {:.4} (prev {:.4}, Δ {:+.4}, {:+.2}%) · close {:.4}",
                                su.bars_used,
                                su.period,
                                su.sum,
                                su.sum_prev,
                                su.sum_delta,
                                su.sum_pct_change,
                                su.last_close
                            );
                            if !su.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", su.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(li)) = rx::get_linreg_intercept(&conn, &sym_upper) {
                        if li.linreg_intercept_label != "INSUFFICIENT_DATA"
                            && !li.linreg_intercept_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Linear-Regression Intercept — LINEARREG_INTERCEPT ({}, as of {})",
                                li.linreg_intercept_label, li.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · b {:.6} (prev {:.6}) · m {:+.6} · close {:.4} · drift {:+.4} ({:+.2}%)",
                                li.bars_used,
                                li.period,
                                li.intercept,
                                li.intercept_prev,
                                li.slope,
                                li.last_close,
                                li.drift,
                                li.drift_pct
                            );
                            if !li.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", li.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    // ── Round 71 emitters ──
                    if let Ok(Some(ao)) = rx::get_aroonosc(&conn, &sym_upper) {
                        if ao.aroonosc_label != "INSUFFICIENT_DATA" && !ao.aroonosc_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Aroon Oscillator — AROONOSC ({}, as of {})",
                                ao.aroonosc_label, ao.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · osc {:+.2} (prev {:+.2}) · up {:.2} · down {:.2} · close {:.4}",
                                ao.bars_used,
                                ao.period,
                                ao.aroonosc,
                                ao.aroonosc_prev,
                                ao.aroon_up,
                                ao.aroon_down,
                                ao.last_close
                            );
                            if !ao.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ao.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(mmi)) = rx::get_minmaxindex(&conn, &sym_upper) {
                        if mmi.minmaxindex_label != "INSUFFICIENT_DATA"
                            && !mmi.minmaxindex_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Min/Max Index — MINMAXINDEX ({}, as of {})",
                                mmi.minmaxindex_label, mmi.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · period {} · low {} ago · high {} ago · age_diff {:+} · order {} · close {:.4}",
                                mmi.bars_used,
                                mmi.period,
                                mmi.min_index_bars_ago,
                                mmi.max_index_bars_ago,
                                mmi.age_diff,
                                mmi.extrema_order,
                                mmi.last_close
                            );
                            if !mmi.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", mmi.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(me)) = rx::get_macdext(&conn, &sym_upper) {
                        if me.macdext_label != "INSUFFICIENT_DATA" && !me.macdext_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### MACD Extended — MACDEXT ({}, as of {})",
                                me.macdext_label, me.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · {}/{}/{} · ma_type {} · macd {:+.6} · signal {:+.6} · hist {:+.6} (prev {:+.6}) · close {:.4}",
                                me.bars_used,
                                me.fast_period,
                                me.slow_period,
                                me.signal_period,
                                me.ma_type,
                                me.macd,
                                me.signal,
                                me.hist,
                                me.hist_prev,
                                me.last_close
                            );
                            if !me.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", me.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(mf)) = rx::get_macdfix(&conn, &sym_upper) {
                        if mf.macdfix_label != "INSUFFICIENT_DATA" && !mf.macdfix_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### MACD Fix — MACDFIX ({}, as of {})",
                                mf.macdfix_label, mf.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · {}/{}/{} · macd {:+.6} · signal {:+.6} · hist {:+.6} (prev {:+.6}) · close {:.4}",
                                mf.bars_used,
                                mf.fast_period,
                                mf.slow_period,
                                mf.signal_period,
                                mf.macd,
                                mf.signal,
                                mf.hist,
                                mf.hist_prev,
                                mf.last_close
                            );
                            if !mf.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", mf.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(mv)) = rx::get_mavp(&conn, &sym_upper) {
                        if mv.mavp_label != "INSUFFICIENT_DATA" && !mv.mavp_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Moving Avg Variable Period — MAVP ({}, as of {})",
                                mv.mavp_label, mv.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · periods {}..{} · last_period {} · mavp {:.6} (prev {:.6}, Δ {:+.6}) · close {:.4}",
                                mv.bars_used,
                                mv.min_period,
                                mv.max_period,
                                mv.last_bar_period,
                                mv.mavp,
                                mv.mavp_prev,
                                mv.mavp_delta,
                                mv.last_close
                            );
                            if !mv.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", mv.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    // ── Round 72 CDL* candlestick patterns ──
                    if let Ok(Some(cd)) = rx::get_cdl_doji(&conn, &sym_upper) {
                        if cd.cdl_doji_label != "INSUFFICIENT_DATA" && !cd.cdl_doji_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Doji — CDLDOJI ({}, as of {})",
                                cd.cdl_doji_label, cd.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body {:.1}% · upper {:.1}% · lower {:.1}% · last_match {} · days_since {} · close {:.4}",
                                cd.bars_used,
                                cd.pattern_value,
                                cd.pattern_value_prev,
                                cd.body_pct_range,
                                cd.upper_shadow_pct,
                                cd.lower_shadow_pct,
                                cd.last_bar_match,
                                cd.days_since_pattern,
                                cd.last_close
                            );
                            if !cd.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cd.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ch)) = rx::get_cdl_hammer(&conn, &sym_upper) {
                        if ch.cdl_hammer_label != "INSUFFICIENT_DATA"
                            && !ch.cdl_hammer_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Hammer — CDLHAMMER ({}, as of {})",
                                ch.cdl_hammer_label, ch.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body {:.1}% · upper {:.1}% · lower {:.1}% · last_match {} · days_since {} · close {:.4}",
                                ch.bars_used,
                                ch.pattern_value,
                                ch.pattern_value_prev,
                                ch.body_pct_range,
                                ch.upper_shadow_pct,
                                ch.lower_shadow_pct,
                                ch.last_bar_match,
                                ch.days_since_pattern,
                                ch.last_close
                            );
                            if !ch.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ch.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cs)) = rx::get_cdl_shooting_star(&conn, &sym_upper) {
                        if cs.cdl_shooting_star_label != "INSUFFICIENT_DATA"
                            && !cs.cdl_shooting_star_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Shooting Star — CDLSHOOTINGSTAR ({}, as of {})",
                                cs.cdl_shooting_star_label, cs.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body {:.1}% · upper {:.1}% · lower {:.1}% · last_match {} · days_since {} · close {:.4}",
                                cs.bars_used,
                                cs.pattern_value,
                                cs.pattern_value_prev,
                                cs.body_pct_range,
                                cs.upper_shadow_pct,
                                cs.lower_shadow_pct,
                                cs.last_bar_match,
                                cs.days_since_pattern,
                                cs.last_close
                            );
                            if !cs.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cs.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ce)) = rx::get_cdl_engulfing(&conn, &sym_upper) {
                        if ce.cdl_engulfing_label != "INSUFFICIENT_DATA"
                            && !ce.cdl_engulfing_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Engulfing — CDLENGULFING ({}, as of {})",
                                ce.cdl_engulfing_label, ce.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · ratio {:.3}× · prior_body {:.1}% · cur_body {:.1}% · last_match {} · days_since {} · close {:.4}",
                                ce.bars_used,
                                ce.pattern_value,
                                ce.pattern_value_prev,
                                ce.body_size_ratio,
                                ce.prior_body_pct_range,
                                ce.current_body_pct_range,
                                ce.last_bar_match,
                                ce.days_since_pattern,
                                ce.last_close
                            );
                            if !ce.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ce.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cr)) = rx::get_cdl_harami(&conn, &sym_upper) {
                        if cr.cdl_harami_label != "INSUFFICIENT_DATA"
                            && !cr.cdl_harami_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Harami — CDLHARAMI ({}, as of {})",
                                cr.cdl_harami_label, cr.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · ratio {:.3}× · prior_body {:.1}% · cur_body {:.1}% · last_match {} · days_since {} · close {:.4}",
                                cr.bars_used,
                                cr.pattern_value,
                                cr.pattern_value_prev,
                                cr.body_size_ratio,
                                cr.prior_body_pct_range,
                                cr.current_body_pct_range,
                                cr.last_bar_match,
                                cr.days_since_pattern,
                                cr.last_close
                            );
                            if !cr.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cr.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cm)) = rx::get_cdl_morning_star(&conn, &sym_upper) {
                        if cm.cdl_morning_star_label != "INSUFFICIENT_DATA"
                            && !cm.cdl_morning_star_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Morning Star — CDLMORNINGSTAR ({}, as of {})",
                                cm.cdl_morning_star_label, cm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · penetration {:.2}% · star_body {:.1}% · first_body {:.1}% · last_body {:.1}% · last_match {} · days_since {} · close {:.4}",
                                cm.bars_used,
                                cm.pattern_value,
                                cm.pattern_value_prev,
                                cm.penetration_pct,
                                cm.star_body_pct_range,
                                cm.first_body_pct_range,
                                cm.last_body_pct_range,
                                cm.last_bar_match,
                                cm.days_since_pattern,
                                cm.last_close
                            );
                            if !cm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cv)) = rx::get_cdl_evening_star(&conn, &sym_upper) {
                        if cv.cdl_evening_star_label != "INSUFFICIENT_DATA"
                            && !cv.cdl_evening_star_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Evening Star — CDLEVENINGSTAR ({}, as of {})",
                                cv.cdl_evening_star_label, cv.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · penetration {:.2}% · star_body {:.1}% · first_body {:.1}% · last_body {:.1}% · last_match {} · days_since {} · close {:.4}",
                                cv.bars_used,
                                cv.pattern_value,
                                cv.pattern_value_prev,
                                cv.penetration_pct,
                                cv.star_body_pct_range,
                                cv.first_body_pct_range,
                                cv.last_body_pct_range,
                                cv.last_bar_match,
                                cv.days_since_pattern,
                                cv.last_close
                            );
                            if !cv.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cv.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cb)) = rx::get_cdl_three_black_crows(&conn, &sym_upper) {
                        if cb.cdl_three_black_crows_label != "INSUFFICIENT_DATA"
                            && !cb.cdl_three_black_crows_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Three Black Crows — CDL3BLACKCROWS ({}, as of {})",
                                cb.cdl_three_black_crows_label, cb.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · avg_body {:.1}% · decline {:.2}% · last_match {} · days_since {} · close {:.4}",
                                cb.bars_used,
                                cb.pattern_value,
                                cb.pattern_value_prev,
                                cb.avg_body_pct_range,
                                cb.total_close_decline_pct,
                                cb.last_bar_match,
                                cb.days_since_pattern,
                                cb.last_close
                            );
                            if !cb.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cb.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cw)) = rx::get_cdl_three_white_soldiers(&conn, &sym_upper) {
                        if cw.cdl_three_white_soldiers_label != "INSUFFICIENT_DATA"
                            && !cw.cdl_three_white_soldiers_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Three White Soldiers — CDL3WHITESOLDIERS ({}, as of {})",
                                cw.cdl_three_white_soldiers_label, cw.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · avg_body {:.1}% · advance {:.2}% · last_match {} · days_since {} · close {:.4}",
                                cw.bars_used,
                                cw.pattern_value,
                                cw.pattern_value_prev,
                                cw.avg_body_pct_range,
                                cw.total_close_advance_pct,
                                cw.last_bar_match,
                                cw.days_since_pattern,
                                cw.last_close
                            );
                            if !cw.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cw.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cd)) = rx::get_cdl_dark_cloud_cover(&conn, &sym_upper) {
                        if cd.cdl_dark_cloud_cover_label != "INSUFFICIENT_DATA"
                            && !cd.cdl_dark_cloud_cover_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Dark Cloud Cover — CDLDARKCLOUDCOVER ({}, as of {})",
                                cd.cdl_dark_cloud_cover_label, cd.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · penetration {:.2}% · prior_body {:.1}% · cur_body {:.1}% · last_match {} · days_since {} · close {:.4}",
                                cd.bars_used,
                                cd.pattern_value,
                                cd.pattern_value_prev,
                                cd.penetration_pct,
                                cd.prior_body_pct_range,
                                cd.current_body_pct_range,
                                cd.last_bar_match,
                                cd.days_since_pattern,
                                cd.last_close
                            );
                            if !cd.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cd.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cp)) = rx::get_cdl_piercing(&conn, &sym_upper) {
                        if cp.cdl_piercing_label != "INSUFFICIENT_DATA"
                            && !cp.cdl_piercing_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Piercing Line — CDLPIERCING ({}, as of {})",
                                cp.cdl_piercing_label, cp.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · penetration {:.2}% · prior_body {:.1}% · cur_body {:.1}% · last_match {} · days_since {} · close {:.4}",
                                cp.bars_used,
                                cp.pattern_value,
                                cp.pattern_value_prev,
                                cp.penetration_pct,
                                cp.prior_body_pct_range,
                                cp.current_body_pct_range,
                                cp.last_bar_match,
                                cp.days_since_pattern,
                                cp.last_close
                            );
                            if !cp.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cp.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cdd)) = rx::get_cdl_dragonfly_doji(&conn, &sym_upper) {
                        if cdd.cdl_dragonfly_doji_label != "INSUFFICIENT_DATA"
                            && !cdd.cdl_dragonfly_doji_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Dragonfly Doji — CDLDRAGONFLYDOJI ({}, as of {})",
                                cdd.cdl_dragonfly_doji_label, cdd.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body {:.1}% · upper {:.1}% · lower {:.1}% · last_match {} · days_since {} · close {:.4}",
                                cdd.bars_used,
                                cdd.pattern_value,
                                cdd.pattern_value_prev,
                                cdd.body_pct_range,
                                cdd.upper_shadow_pct,
                                cdd.lower_shadow_pct,
                                cdd.last_bar_match,
                                cdd.days_since_pattern,
                                cdd.last_close
                            );
                            if !cdd.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cdd.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cgd)) = rx::get_cdl_gravestone_doji(&conn, &sym_upper) {
                        if cgd.cdl_gravestone_doji_label != "INSUFFICIENT_DATA"
                            && !cgd.cdl_gravestone_doji_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Gravestone Doji — CDLGRAVESTONEDOJI ({}, as of {})",
                                cgd.cdl_gravestone_doji_label, cgd.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body {:.1}% · upper {:.1}% · lower {:.1}% · last_match {} · days_since {} · close {:.4}",
                                cgd.bars_used,
                                cgd.pattern_value,
                                cgd.pattern_value_prev,
                                cgd.body_pct_range,
                                cgd.upper_shadow_pct,
                                cgd.lower_shadow_pct,
                                cgd.last_bar_match,
                                cgd.days_since_pattern,
                                cgd.last_close
                            );
                            if !cgd.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cgd.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(chm)) = rx::get_cdl_hanging_man(&conn, &sym_upper) {
                        if chm.cdl_hanging_man_label != "INSUFFICIENT_DATA"
                            && !chm.cdl_hanging_man_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Hanging Man — CDLHANGINGMAN ({}, as of {})",
                                chm.cdl_hanging_man_label, chm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body {:.1}% · upper {:.1}% · lower {:.1}% · last_match {} · days_since {} · close {:.4}",
                                chm.bars_used,
                                chm.pattern_value,
                                chm.pattern_value_prev,
                                chm.body_pct_range,
                                chm.upper_shadow_pct,
                                chm.lower_shadow_pct,
                                chm.last_bar_match,
                                chm.days_since_pattern,
                                chm.last_close
                            );
                            if !chm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", chm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cih)) = rx::get_cdl_inverted_hammer(&conn, &sym_upper) {
                        if cih.cdl_inverted_hammer_label != "INSUFFICIENT_DATA"
                            && !cih.cdl_inverted_hammer_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Inverted Hammer — CDLINVERTEDHAMMER ({}, as of {})",
                                cih.cdl_inverted_hammer_label, cih.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body {:.1}% · upper {:.1}% · lower {:.1}% · last_match {} · days_since {} · close {:.4}",
                                cih.bars_used,
                                cih.pattern_value,
                                cih.pattern_value_prev,
                                cih.body_pct_range,
                                cih.upper_shadow_pct,
                                cih.lower_shadow_pct,
                                cih.last_bar_match,
                                cih.days_since_pattern,
                                cih.last_close
                            );
                            if !cih.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cih.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(chc)) = rx::get_cdl_harami_cross(&conn, &sym_upper) {
                        if chc.cdl_harami_cross_label != "INSUFFICIENT_DATA"
                            && !chc.cdl_harami_cross_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Harami Cross — CDLHARAMICROSS ({}, as of {})",
                                chc.cdl_harami_cross_label, chc.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · prior_body {:.1}% · cur_body {:.1}% · ratio {:.3} · last_match {} · days_since {} · close {:.4}",
                                chc.bars_used,
                                chc.pattern_value,
                                chc.pattern_value_prev,
                                chc.prior_body_pct_range,
                                chc.current_body_pct_range,
                                chc.body_size_ratio,
                                chc.last_bar_match,
                                chc.days_since_pattern,
                                chc.last_close
                            );
                            if !chc.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", chc.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(clld)) = rx::get_cdl_long_legged_doji(&conn, &sym_upper) {
                        if clld.cdl_long_legged_doji_label != "INSUFFICIENT_DATA"
                            && !clld.cdl_long_legged_doji_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Long-Legged Doji — CDLLONGLEGGEDDOJI ({}, as of {})",
                                clld.cdl_long_legged_doji_label, clld.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body {:.1}% · upper {:.1}% · lower {:.1}% · last_match {} · days_since {} · close {:.4}",
                                clld.bars_used,
                                clld.pattern_value,
                                clld.pattern_value_prev,
                                clld.body_pct_range,
                                clld.upper_shadow_pct,
                                clld.lower_shadow_pct,
                                clld.last_bar_match,
                                clld.days_since_pattern,
                                clld.last_close
                            );
                            if !clld.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", clld.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cm)) = rx::get_cdl_marubozu(&conn, &sym_upper) {
                        if cm.cdl_marubozu_label != "INSUFFICIENT_DATA"
                            && !cm.cdl_marubozu_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Marubozu — CDLMARUBOZU ({}, as of {})",
                                cm.cdl_marubozu_label, cm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body {:.1}% · upper {:.1}% · lower {:.1}% · last_match {} · days_since {} · close {:.4}",
                                cm.bars_used,
                                cm.pattern_value,
                                cm.pattern_value_prev,
                                cm.body_pct_range,
                                cm.upper_shadow_pct,
                                cm.lower_shadow_pct,
                                cm.last_bar_match,
                                cm.days_since_pattern,
                                cm.last_close
                            );
                            if !cm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cst)) = rx::get_cdl_spinning_top(&conn, &sym_upper) {
                        if cst.cdl_spinning_top_label != "INSUFFICIENT_DATA"
                            && !cst.cdl_spinning_top_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Spinning Top — CDLSPINNINGTOP ({}, as of {})",
                                cst.cdl_spinning_top_label, cst.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body {:.1}% · upper {:.1}% · lower {:.1}% · last_match {} · days_since {} · close {:.4}",
                                cst.bars_used,
                                cst.pattern_value,
                                cst.pattern_value_prev,
                                cst.body_pct_range,
                                cst.upper_shadow_pct,
                                cst.lower_shadow_pct,
                                cst.last_bar_match,
                                cst.days_since_pattern,
                                cst.last_close
                            );
                            if !cst.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cst.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cts)) = rx::get_cdl_tristar(&conn, &sym_upper) {
                        if cts.cdl_tristar_label != "INSUFFICIENT_DATA"
                            && !cts.cdl_tristar_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Tri-Star — CDLTRISTAR ({}, as of {})",
                                cts.cdl_tristar_label, cts.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · avg_body {:.1}% · mid_gap {:.2}% · last_match {} · days_since {} · close {:.4}",
                                cts.bars_used,
                                cts.pattern_value,
                                cts.pattern_value_prev,
                                cts.avg_body_pct_range,
                                cts.middle_gap_pct,
                                cts.last_bar_match,
                                cts.days_since_pattern,
                                cts.last_close
                            );
                            if !cts.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cts.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    // ── Round 76 packet blocks ──
                    if let Ok(Some(cds)) = rx::get_cdl_doji_star(&conn, &sym_upper) {
                        if cds.cdl_doji_star_label != "INSUFFICIENT_DATA"
                            && !cds.cdl_doji_star_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Doji Star — CDLDOJISTAR ({}, as of {})",
                                cds.cdl_doji_star_label, cds.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · prior_body {:.1}% · cur_body {:.1}% · gap {:+.2}% · last_match {} · days_since {} · close {:.4}",
                                cds.bars_used,
                                cds.pattern_value,
                                cds.pattern_value_prev,
                                cds.prior_body_pct_range,
                                cds.current_body_pct_range,
                                cds.gap_pct,
                                cds.last_bar_match,
                                cds.days_since_pattern,
                                cds.last_close
                            );
                            if !cds.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cds.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cmds)) = rx::get_cdl_morning_doji_star(&conn, &sym_upper) {
                        if cmds.cdl_morning_doji_star_label != "INSUFFICIENT_DATA"
                            && !cmds.cdl_morning_doji_star_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Morning Doji Star — CDLMORNINGDOJISTAR ({}, as of {})",
                                cmds.cdl_morning_doji_star_label, cmds.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · b1_body {:.1}% · b2_body {:.1}% · b3_vs_mid {:+.2}% · last_match {} · days_since {} · close {:.4}",
                                cmds.bars_used,
                                cmds.pattern_value,
                                cmds.pattern_value_prev,
                                cmds.bar1_body_pct_range,
                                cmds.bar2_body_pct_range,
                                cmds.bar3_close_vs_bar1_mid_pct,
                                cmds.last_bar_match,
                                cmds.days_since_pattern,
                                cmds.last_close
                            );
                            if !cmds.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cmds.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ceds)) = rx::get_cdl_evening_doji_star(&conn, &sym_upper) {
                        if ceds.cdl_evening_doji_star_label != "INSUFFICIENT_DATA"
                            && !ceds.cdl_evening_doji_star_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Evening Doji Star — CDLEVENINGDOJISTAR ({}, as of {})",
                                ceds.cdl_evening_doji_star_label, ceds.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · b1_body {:.1}% · b2_body {:.1}% · b3_vs_mid {:+.2}% · last_match {} · days_since {} · close {:.4}",
                                ceds.bars_used,
                                ceds.pattern_value,
                                ceds.pattern_value_prev,
                                ceds.bar1_body_pct_range,
                                ceds.bar2_body_pct_range,
                                ceds.bar3_close_vs_bar1_mid_pct,
                                ceds.last_bar_match,
                                ceds.days_since_pattern,
                                ceds.last_close
                            );
                            if !ceds.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ceds.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cab)) = rx::get_cdl_abandoned_baby(&conn, &sym_upper) {
                        if cab.cdl_abandoned_baby_label != "INSUFFICIENT_DATA"
                            && !cab.cdl_abandoned_baby_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Abandoned Baby — CDLABANDONEDBABY ({}, as of {})",
                                cab.cdl_abandoned_baby_label, cab.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · b1_body {:.1}% · b2_body {:.1}% · gap_down {:+.2}% · gap_up {:+.2}% · last_match {} · days_since {} · close {:.4}",
                                cab.bars_used,
                                cab.pattern_value,
                                cab.pattern_value_prev,
                                cab.bar1_body_pct_range,
                                cab.bar2_body_pct_range,
                                cab.gap_down_pct,
                                cab.gap_up_pct,
                                cab.last_bar_match,
                                cab.days_since_pattern,
                                cab.last_close
                            );
                            if !cab.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cab.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(c3i)) = rx::get_cdl_three_inside(&conn, &sym_upper) {
                        if c3i.cdl_three_inside_label != "INSUFFICIENT_DATA"
                            && !c3i.cdl_three_inside_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Three Inside — CDL3INSIDE ({}, as of {})",
                                c3i.cdl_three_inside_label, c3i.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · b1_body {:.1}% · body_ratio {:.3} · b3_vs_b1_open {:+.2}% · last_match {} · days_since {} · close {:.4}",
                                c3i.bars_used,
                                c3i.pattern_value,
                                c3i.pattern_value_prev,
                                c3i.bar1_body_pct_range,
                                c3i.body_size_ratio,
                                c3i.bar3_close_vs_bar1_open_pct,
                                c3i.last_bar_match,
                                c3i.days_since_pattern,
                                c3i.last_close
                            );
                            if !c3i.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", c3i.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    // ── Round 77 packet blocks ──
                    if let Ok(Some(cbh)) = rx::get_cdl_belt_hold(&conn, &sym_upper) {
                        if cbh.cdl_belt_hold_label != "INSUFFICIENT_DATA"
                            && !cbh.cdl_belt_hold_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Belt Hold — CDLBELTHOLD ({}, as of {})",
                                cbh.cdl_belt_hold_label, cbh.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body {:.1}% · opening_shadow {:.1}% · closing_shadow {:.1}% · last_match {} · days_since {} · close {:.4}",
                                cbh.bars_used,
                                cbh.pattern_value,
                                cbh.pattern_value_prev,
                                cbh.body_pct_range,
                                cbh.opening_shadow_pct,
                                cbh.closing_shadow_pct,
                                cbh.last_bar_match,
                                cbh.days_since_pattern,
                                cbh.last_close
                            );
                            if !cbh.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cbh.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ccm)) = rx::get_cdl_closing_marubozu(&conn, &sym_upper) {
                        if ccm.cdl_closing_marubozu_label != "INSUFFICIENT_DATA"
                            && !ccm.cdl_closing_marubozu_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Closing Marubozu — CDLCLOSINGMARUBOZU ({}, as of {})",
                                ccm.cdl_closing_marubozu_label, ccm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body {:.1}% · opening_shadow {:.1}% · closing_shadow {:.1}% · last_match {} · days_since {} · close {:.4}",
                                ccm.bars_used,
                                ccm.pattern_value,
                                ccm.pattern_value_prev,
                                ccm.body_pct_range,
                                ccm.opening_shadow_pct,
                                ccm.closing_shadow_pct,
                                ccm.last_bar_match,
                                ccm.days_since_pattern,
                                ccm.last_close
                            );
                            if !ccm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ccm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(chw)) = rx::get_cdl_high_wave(&conn, &sym_upper) {
                        if chw.cdl_high_wave_label != "INSUFFICIENT_DATA"
                            && !chw.cdl_high_wave_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick High Wave — CDLHIGHWAVE ({}, as of {})",
                                chw.cdl_high_wave_label, chw.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body {:.1}% · upper {:.1}% · lower {:.1}% · last_match {} · days_since {} · close {:.4}",
                                chw.bars_used,
                                chw.pattern_value,
                                chw.pattern_value_prev,
                                chw.body_pct_range,
                                chw.upper_shadow_pct,
                                chw.lower_shadow_pct,
                                chw.last_bar_match,
                                chw.days_since_pattern,
                                chw.last_close
                            );
                            if !chw.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", chw.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cll)) = rx::get_cdl_long_line(&conn, &sym_upper) {
                        if cll.cdl_long_line_label != "INSUFFICIENT_DATA"
                            && !cll.cdl_long_line_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Long Line — CDLLONGLINE ({}, as of {})",
                                cll.cdl_long_line_label, cll.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body {:.1}% · upper {:.1}% · lower {:.1}% · last_match {} · days_since {} · close {:.4}",
                                cll.bars_used,
                                cll.pattern_value,
                                cll.pattern_value_prev,
                                cll.body_pct_range,
                                cll.upper_shadow_pct,
                                cll.lower_shadow_pct,
                                cll.last_bar_match,
                                cll.days_since_pattern,
                                cll.last_close
                            );
                            if !cll.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cll.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(csl)) = rx::get_cdl_short_line(&conn, &sym_upper) {
                        if csl.cdl_short_line_label != "INSUFFICIENT_DATA"
                            && !csl.cdl_short_line_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Short Line — CDLSHORTLINE ({}, as of {})",
                                csl.cdl_short_line_label, csl.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body {:.1}% · upper {:.1}% · lower {:.1}% · last_match {} · days_since {} · close {:.4}",
                                csl.bars_used,
                                csl.pattern_value,
                                csl.pattern_value_prev,
                                csl.body_pct_range,
                                csl.upper_shadow_pct,
                                csl.lower_shadow_pct,
                                csl.last_bar_match,
                                csl.days_since_pattern,
                                csl.last_close
                            );
                            if !csl.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", csl.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    // ── Round 78 packet blocks ──
                    if let Ok(Some(cca)) = rx::get_cdl_counterattack(&conn, &sym_upper) {
                        if cca.cdl_counterattack_label != "INSUFFICIENT_DATA"
                            && !cca.cdl_counterattack_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Counterattack — CDLCOUNTERATTACK ({}, as of {})",
                                cca.cdl_counterattack_label, cca.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · prior_body {:.1}% · current_body {:.1}% · gap_open {:.2}% · close_diff/body {:.2}% · last_match {} · days_since {} · close {:.4}",
                                cca.bars_used,
                                cca.pattern_value,
                                cca.pattern_value_prev,
                                cca.prior_body_pct_range,
                                cca.current_body_pct_range,
                                cca.gap_open_pct,
                                cca.close_diff_pct_body,
                                cca.last_bar_match,
                                cca.days_since_pattern,
                                cca.last_close
                            );
                            if !cca.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cca.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(chp)) = rx::get_cdl_homing_pigeon(&conn, &sym_upper) {
                        if chp.cdl_homing_pigeon_label != "INSUFFICIENT_DATA"
                            && !chp.cdl_homing_pigeon_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Homing Pigeon — CDLHOMINGPIGEON ({}, as of {})",
                                chp.cdl_homing_pigeon_label, chp.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · prior_body {:.1}% · current_body {:.1}% · body_ratio {:.3} · inner_margin {:.2}% · last_match {} · days_since {} · close {:.4}",
                                chp.bars_used,
                                chp.pattern_value,
                                chp.pattern_value_prev,
                                chp.prior_body_pct_range,
                                chp.current_body_pct_range,
                                chp.body_size_ratio,
                                chp.inner_body_margin_pct,
                                chp.last_bar_match,
                                chp.days_since_pattern,
                                chp.last_close
                            );
                            if !chp.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", chp.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cin)) = rx::get_cdl_in_neck(&conn, &sym_upper) {
                        if cin.cdl_in_neck_label != "INSUFFICIENT_DATA"
                            && !cin.cdl_in_neck_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick In-Neck — CDLINNECK ({}, as of {})",
                                cin.cdl_in_neck_label, cin.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · prior_body {:.1}% · current_body {:.1}% · gap_open {:.2}% · penetration {:.2}% · last_match {} · days_since {} · close {:.4}",
                                cin.bars_used,
                                cin.pattern_value,
                                cin.pattern_value_prev,
                                cin.prior_body_pct_range,
                                cin.current_body_pct_range,
                                cin.gap_open_pct,
                                cin.penetration_pct,
                                cin.last_bar_match,
                                cin.days_since_pattern,
                                cin.last_close
                            );
                            if !cin.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cin.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(con)) = rx::get_cdl_on_neck(&conn, &sym_upper) {
                        if con.cdl_on_neck_label != "INSUFFICIENT_DATA"
                            && !con.cdl_on_neck_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick On-Neck — CDLONNECK ({}, as of {})",
                                con.cdl_on_neck_label, con.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · prior_body {:.1}% · current_body {:.1}% · gap_open {:.2}% · close_match {:.2}% · last_match {} · days_since {} · close {:.4}",
                                con.bars_used,
                                con.pattern_value,
                                con.pattern_value_prev,
                                con.prior_body_pct_range,
                                con.current_body_pct_range,
                                con.gap_open_pct,
                                con.close_match_pct,
                                con.last_bar_match,
                                con.days_since_pattern,
                                con.last_close
                            );
                            if !con.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", con.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cth)) = rx::get_cdl_thrusting(&conn, &sym_upper) {
                        if cth.cdl_thrusting_label != "INSUFFICIENT_DATA"
                            && !cth.cdl_thrusting_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Thrusting — CDLTHRUSTING ({}, as of {})",
                                cth.cdl_thrusting_label, cth.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · prior_body {:.1}% · current_body {:.1}% · gap_open {:.2}% · penetration {:.2}% · last_match {} · days_since {} · close {:.4}",
                                cth.bars_used,
                                cth.pattern_value,
                                cth.pattern_value_prev,
                                cth.prior_body_pct_range,
                                cth.current_body_pct_range,
                                cth.gap_open_pct,
                                cth.penetration_pct,
                                cth.last_bar_match,
                                cth.days_since_pattern,
                                cth.last_close
                            );
                            if !cth.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cth.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(c2)) = rx::get_cdl_two_crows(&conn, &sym_upper) {
                        if c2.cdl_two_crows_label != "INSUFFICIENT_DATA"
                            && !c2.cdl_two_crows_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Two Crows — CDL2CROWS ({}, as of {})",
                                c2.cdl_two_crows_label, c2.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body1 {:.1}% · gap {:.2}% · penetration {:.2}% · last_match {} · days_since {} · close {:.4}",
                                c2.bars_used,
                                c2.pattern_value,
                                c2.pattern_value_prev,
                                c2.first_body_pct_range,
                                c2.second_gap_pct,
                                c2.third_penetration_pct,
                                c2.last_bar_match,
                                c2.days_since_pattern,
                                c2.last_close
                            );
                            if !c2.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", c2.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(c3ls)) = rx::get_cdl_three_line_strike(&conn, &sym_upper) {
                        if c3ls.cdl_three_line_strike_label != "INSUFFICIENT_DATA"
                            && !c3ls.cdl_three_line_strike_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Three Line Strike — CDL3LINESTRIKE ({}, as of {})",
                                c3ls.cdl_three_line_strike_label, c3ls.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · avg_body123 {:.1}% · strike_body {:.1}% · strike_vs_first_open {:.2}% · last_match {} · days_since {} · close {:.4}",
                                c3ls.bars_used,
                                c3ls.pattern_value,
                                c3ls.pattern_value_prev,
                                c3ls.avg_first_three_body_pct_range,
                                c3ls.strike_body_pct_range,
                                c3ls.strike_close_vs_first_open_pct,
                                c3ls.last_bar_match,
                                c3ls.days_since_pattern,
                                c3ls.last_close
                            );
                            if !c3ls.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", c3ls.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(c3o)) = rx::get_cdl_three_outside(&conn, &sym_upper) {
                        if c3o.cdl_three_outside_label != "INSUFFICIENT_DATA"
                            && !c3o.cdl_three_outside_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Three Outside — CDL3OUTSIDE ({}, as of {})",
                                c3o.cdl_three_outside_label, c3o.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body1 {:.1}% · engulf_ratio {:.3} · confirm {:.2}% · last_match {} · days_since {} · close {:.4}",
                                c3o.bars_used,
                                c3o.pattern_value,
                                c3o.pattern_value_prev,
                                c3o.first_body_pct_range,
                                c3o.engulf_body_ratio,
                                c3o.confirmation_pct_body2,
                                c3o.last_bar_match,
                                c3o.days_since_pattern,
                                c3o.last_close
                            );
                            if !c3o.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", c3o.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cml)) = rx::get_cdl_matching_low(&conn, &sym_upper) {
                        if cml.cdl_matching_low_label != "INSUFFICIENT_DATA"
                            && !cml.cdl_matching_low_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Matching Low — CDLMATCHINGLOW ({}, as of {})",
                                cml.cdl_matching_low_label, cml.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · prior_body {:.1}% · current_body {:.1}% · close_match {:.2}% · last_match {} · days_since {} · close {:.4}",
                                cml.bars_used,
                                cml.pattern_value,
                                cml.pattern_value_prev,
                                cml.prior_body_pct_range,
                                cml.current_body_pct_range,
                                cml.close_match_pct_body,
                                cml.last_bar_match,
                                cml.days_since_pattern,
                                cml.last_close
                            );
                            if !cml.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cml.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(csl)) = rx::get_cdl_separating_lines(&conn, &sym_upper) {
                        if csl.cdl_separating_lines_label != "INSUFFICIENT_DATA"
                            && !csl.cdl_separating_lines_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Separating Lines — CDLSEPARATINGLINES ({}, as of {})",
                                csl.cdl_separating_lines_label, csl.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · prior_body {:.1}% · current_body {:.1}% · open_match {:.2}% · continuation {:.2}% · last_match {} · days_since {} · close {:.4}",
                                csl.bars_used,
                                csl.pattern_value,
                                csl.pattern_value_prev,
                                csl.prior_body_pct_range,
                                csl.current_body_pct_range,
                                csl.open_match_pct_body,
                                csl.continuation_pct_body,
                                csl.last_bar_match,
                                csl.days_since_pattern,
                                csl.last_close
                            );
                            if !csl.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", csl.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(css)) = rx::get_cdl_stick_sandwich(&conn, &sym_upper) {
                        if css.cdl_stick_sandwich_label != "INSUFFICIENT_DATA"
                            && !css.cdl_stick_sandwich_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Stick Sandwich — CDLSTICKSANDWICH ({}, as of {})",
                                css.cdl_stick_sandwich_label, css.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body1 {:.1}% · body3 {:.1}% · close_match {:.2}% · rebound {:.2}% · last_match {} · days_since {} · close {:.4}",
                                css.bars_used,
                                css.pattern_value,
                                css.pattern_value_prev,
                                css.first_body_pct_range,
                                css.third_body_pct_range,
                                css.close_match_pct_body,
                                css.middle_rebound_pct,
                                css.last_bar_match,
                                css.days_since_pattern,
                                css.last_close
                            );
                            if !css.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", css.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(crm)) = rx::get_cdl_rickshaw_man(&conn, &sym_upper) {
                        if crm.cdl_rickshaw_man_label != "INSUFFICIENT_DATA"
                            && !crm.cdl_rickshaw_man_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Rickshaw Man — CDLRICKSHAWMAN ({}, as of {})",
                                crm.cdl_rickshaw_man_label, crm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body {:.2}% · upper {:.2}% · lower {:.2}% · midpoint_offset {:.2}% · last_match {} · days_since {} · close {:.4}",
                                crm.bars_used,
                                crm.pattern_value,
                                crm.pattern_value_prev,
                                crm.body_pct_range,
                                crm.upper_shadow_pct,
                                crm.lower_shadow_pct,
                                crm.body_midpoint_offset_pct,
                                crm.last_bar_match,
                                crm.days_since_pattern,
                                crm.last_close
                            );
                            if !crm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", crm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ctk)) = rx::get_cdl_takuri(&conn, &sym_upper) {
                        if ctk.cdl_takuri_label != "INSUFFICIENT_DATA"
                            && !ctk.cdl_takuri_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Takuri — CDLTAKURI ({}, as of {})",
                                ctk.cdl_takuri_label, ctk.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body {:.2}% · upper {:.2}% · lower {:.2}% · lower/upper {:.2}x · last_match {} · days_since {} · close {:.4}",
                                ctk.bars_used,
                                ctk.pattern_value,
                                ctk.pattern_value_prev,
                                ctk.body_pct_range,
                                ctk.upper_shadow_pct,
                                ctk.lower_shadow_pct,
                                ctk.lower_to_upper_ratio,
                                ctk.last_bar_match,
                                ctk.days_since_pattern,
                                ctk.last_close
                            );
                            if !ctk.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ctk.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(c3sis)) = rx::get_cdl_three_stars_in_south(&conn, &sym_upper) {
                        if c3sis.cdl_three_stars_in_south_label != "INSUFFICIENT_DATA"
                            && !c3sis.cdl_three_stars_in_south_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Three Stars In The South — CDL3STARSINSOUTH ({}, as of {})",
                                c3sis.cdl_three_stars_in_south_label, c3sis.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body1 {:.1}% · lower1 {:.1}% · body2 {:.1}% · body3 {:.1}% · inside3 {:.1}% · last_match {} · days_since {} · close {:.4}",
                                c3sis.bars_used,
                                c3sis.pattern_value,
                                c3sis.pattern_value_prev,
                                c3sis.first_body_pct_range,
                                c3sis.first_lower_shadow_pct,
                                c3sis.second_body_pct_range,
                                c3sis.third_body_pct_range,
                                c3sis.third_inside_pct_range,
                                c3sis.last_bar_match,
                                c3sis.days_since_pattern,
                                c3sis.last_close
                            );
                            if !c3sis.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", c3sis.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(citc)) = rx::get_cdl_identical_three_crows(&conn, &sym_upper) {
                        if citc.cdl_identical_three_crows_label != "INSUFFICIENT_DATA"
                            && !citc.cdl_identical_three_crows_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Identical Three Crows — CDLIDENTICAL3CROWS ({}, as of {})",
                                citc.cdl_identical_three_crows_label, citc.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · avg_body {:.1}% · open1/close0 {:.2}% · open2/close1 {:.2}% · total_decline {:.2}% · last_match {} · days_since {} · close {:.4}",
                                citc.bars_used,
                                citc.pattern_value,
                                citc.pattern_value_prev,
                                citc.avg_body_pct_range,
                                citc.open1_vs_close0_pct_body,
                                citc.open2_vs_close1_pct_body,
                                citc.total_close_decline_pct,
                                citc.last_bar_match,
                                citc.days_since_pattern,
                                citc.last_close
                            );
                            if !citc.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", citc.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ckk)) = rx::get_cdl_kicking(&conn, &sym_upper) {
                        if ckk.cdl_kicking_label != "INSUFFICIENT_DATA"
                            && !ckk.cdl_kicking_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Kicking — CDLKICKING ({}, as of {})",
                                ckk.cdl_kicking_label, ckk.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body1 {:.1}% · body2 {:.1}% · gap {:.2}% · body_ratio {:.2}x · last_match {} · days_since {} · close {:.4}",
                                ckk.bars_used,
                                ckk.pattern_value,
                                ckk.pattern_value_prev,
                                ckk.first_body_pct_range,
                                ckk.second_body_pct_range,
                                ckk.gap_pct_range,
                                ckk.second_to_first_body_ratio,
                                ckk.last_bar_match,
                                ckk.days_since_pattern,
                                ckk.last_close
                            );
                            if !ckk.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ckk.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ckbl)) = rx::get_cdl_kicking_by_length(&conn, &sym_upper) {
                        if ckbl.cdl_kicking_by_length_label != "INSUFFICIENT_DATA"
                            && !ckbl.cdl_kicking_by_length_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Kicking By Length — CDLKICKINGBYLENGTH ({}, as of {})",
                                ckbl.cdl_kicking_by_length_label, ckbl.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body1 {:.1}% · body2 {:.1}% · gap {:.2}% · dominant {:.2}x ({}) · last_match {} · days_since {} · close {:.4}",
                                ckbl.bars_used,
                                ckbl.pattern_value,
                                ckbl.pattern_value_prev,
                                ckbl.first_body_pct_range,
                                ckbl.second_body_pct_range,
                                ckbl.gap_pct_range,
                                ckbl.dominant_body_ratio,
                                ckbl.dominant_side,
                                ckbl.last_bar_match,
                                ckbl.days_since_pattern,
                                ckbl.last_close
                            );
                            if !ckbl.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ckbl.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(clb)) = rx::get_cdl_ladder_bottom(&conn, &sym_upper) {
                        if clb.cdl_ladder_bottom_label != "INSUFFICIENT_DATA"
                            && !clb.cdl_ladder_bottom_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Ladder Bottom — CDLLADDERBOTTOM ({}, as of {})",
                                clb.cdl_ladder_bottom_label, clb.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · avg_body123 {:.1}% · body4 {:.1}% · upper4 {:.1}% · body5 {:.1}% · breakout {:.2}% · last_match {} · days_since {} · close {:.4}",
                                clb.bars_used,
                                clb.pattern_value,
                                clb.pattern_value_prev,
                                clb.avg_first_three_body_pct_range,
                                clb.fourth_body_pct_range,
                                clb.fourth_upper_shadow_pct,
                                clb.fifth_body_pct_range,
                                clb.breakout_pct_vs_fourth_high,
                                clb.last_bar_match,
                                clb.days_since_pattern,
                                clb.last_close
                            );
                            if !clb.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", clb.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cutr)) = rx::get_cdl_unique_three_river(&conn, &sym_upper) {
                        if cutr.cdl_unique_three_river_label != "INSUFFICIENT_DATA"
                            && !cutr.cdl_unique_three_river_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Unique 3 River — CDLUNIQUE3RIVER ({}, as of {})",
                                cutr.cdl_unique_three_river_label, cutr.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body1 {:.1}% · body2 {:.1}% · lower2 {:.1}% · body3 {:.1}% · close3-vs-close2 {:.2}% · last_match {} · days_since {} · close {:.4}",
                                cutr.bars_used,
                                cutr.pattern_value,
                                cutr.pattern_value_prev,
                                cutr.first_body_pct_range,
                                cutr.second_body_pct_range,
                                cutr.second_lower_shadow_pct,
                                cutr.third_body_pct_range,
                                cutr.third_close_vs_second_close_pct,
                                cutr.last_bar_match,
                                cutr.days_since_pattern,
                                cutr.last_close
                            );
                            if !cutr.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cutr.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cab)) = rx::get_cdl_advance_block(&conn, &sym_upper) {
                        if cab.cdl_advance_block_label != "INSUFFICIENT_DATA"
                            && !cab.cdl_advance_block_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Advance Block — CDLADVANCEBLOCK ({}, as of {})",
                                cab.cdl_advance_block_label, cab.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body1 {:.1}% · body2 {:.1}% · body3 {:.1}% · upper3 {:.1}% · close_gain {:.2}% · last_match {} · days_since {} · close {:.4}",
                                cab.bars_used,
                                cab.pattern_value,
                                cab.pattern_value_prev,
                                cab.first_body_pct_range,
                                cab.second_body_pct_range,
                                cab.third_body_pct_range,
                                cab.third_upper_shadow_pct,
                                cab.total_close_gain_pct,
                                cab.last_bar_match,
                                cab.days_since_pattern,
                                cab.last_close
                            );
                            if !cab.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cab.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cbr)) = rx::get_cdl_breakaway(&conn, &sym_upper) {
                        if cbr.cdl_breakaway_label != "INSUFFICIENT_DATA"
                            && !cbr.cdl_breakaway_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Breakaway — CDLBREAKAWAY ({}, as of {})",
                                cbr.cdl_breakaway_label, cbr.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body1 {:.1}% · gap {:.2}% · body5 {:.1}% · retrace {:.2}% · last_match {} · days_since {} · close {:.4}",
                                cbr.bars_used,
                                cbr.pattern_value,
                                cbr.pattern_value_prev,
                                cbr.first_body_pct_range,
                                cbr.initial_gap_pct_range,
                                cbr.fifth_body_pct_range,
                                cbr.gap_retracement_pct,
                                cbr.last_bar_match,
                                cbr.days_since_pattern,
                                cbr.last_close
                            );
                            if !cbr.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cbr.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cgssw)) = rx::get_cdl_gap_side_side_white(&conn, &sym_upper) {
                        if cgssw.cdl_gap_side_side_white_label != "INSUFFICIENT_DATA"
                            && !cgssw.cdl_gap_side_side_white_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Gap Side Side White — CDLGAPSIDESIDEWHITE ({}, as of {})",
                                cgssw.cdl_gap_side_side_white_label, cgssw.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · gap {:.2}% · body2 {:.1}% · body3 {:.1}% · open_sim {:.2}% · close_sim {:.2}% · last_match {} · days_since {} · close {:.4}",
                                cgssw.bars_used,
                                cgssw.pattern_value,
                                cgssw.pattern_value_prev,
                                cgssw.gap_pct_range,
                                cgssw.second_body_pct_range,
                                cgssw.third_body_pct_range,
                                cgssw.open_similarity_pct_body,
                                cgssw.close_similarity_pct_body,
                                cgssw.last_bar_match,
                                cgssw.days_since_pattern,
                                cgssw.last_close
                            );
                            if !cgssw.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cgssw.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cug2c)) = rx::get_cdl_upside_gap_two_crows(&conn, &sym_upper) {
                        if cug2c.cdl_upside_gap_two_crows_label != "INSUFFICIENT_DATA"
                            && !cug2c.cdl_upside_gap_two_crows_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Upside Gap Two Crows — CDLUPSIDEGAP2CROWS ({}, as of {})",
                                cug2c.cdl_upside_gap_two_crows_label, cug2c.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body1 {:.1}% · gap {:.2}% · open3>{} {:.2}% · close3-into-gap {:.2}% · last_match {} · days_since {} · close {:.4}",
                                cug2c.bars_used,
                                cug2c.pattern_value,
                                cug2c.pattern_value_prev,
                                cug2c.first_body_pct_range,
                                cug2c.upside_gap_pct_range,
                                "open2",
                                cug2c.third_open_above_second_pct_body,
                                cug2c.third_close_into_gap_pct,
                                cug2c.last_bar_match,
                                cug2c.days_since_pattern,
                                cug2c.last_close
                            );
                            if !cug2c.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cug2c.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cxgm)) = rx::get_cdl_xside_gap_three_methods(&conn, &sym_upper) {
                        if cxgm.cdl_xside_gap_three_methods_label != "INSUFFICIENT_DATA"
                            && !cxgm.cdl_xside_gap_three_methods_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick X-Side Gap Three Methods — CDLXSIDEGAP3METHODS ({}, as of {})",
                                cxgm.cdl_xside_gap_three_methods_label, cxgm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · gap {:.2}% · body2 {:.1}% · body3 {:.1}% · gap_fill {:.2}% · last_match {} · days_since {} · close {:.4}",
                                cxgm.bars_used,
                                cxgm.pattern_value,
                                cxgm.pattern_value_prev,
                                cxgm.gap_pct_range,
                                cxgm.second_body_pct_range,
                                cxgm.third_body_pct_range,
                                cxgm.gap_fill_pct,
                                cxgm.last_bar_match,
                                cxgm.days_since_pattern,
                                cxgm.last_close
                            );
                            if !cxgm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cxgm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ccbs)) = rx::get_cdl_conceal_baby_swallow(&conn, &sym_upper) {
                        if ccbs.cdl_conceal_baby_swallow_label != "INSUFFICIENT_DATA"
                            && !ccbs.cdl_conceal_baby_swallow_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Conceal Baby Swallow — CDLCONCEALBABYSWALL ({}, as of {})",
                                ccbs.cdl_conceal_baby_swallow_label, ccbs.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body1 {:.1}% · body2 {:.1}% · upper3 {:.1}% · engulf4 {:.2}% · last_match {} · days_since {} · close {:.4}",
                                ccbs.bars_used,
                                ccbs.pattern_value,
                                ccbs.pattern_value_prev,
                                ccbs.first_body_pct_range,
                                ccbs.second_body_pct_range,
                                ccbs.third_upper_shadow_pct,
                                ccbs.fourth_range_engulf_pct,
                                ccbs.last_bar_match,
                                ccbs.days_since_pattern,
                                ccbs.last_close
                            );
                            if !ccbs.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ccbs.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(chk)) = rx::get_cdl_hikkake(&conn, &sym_upper) {
                        if chk.cdl_hikkake_label != "INSUFFICIENT_DATA"
                            && !chk.cdl_hikkake_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Hikkake — CDLHIKKAKE ({}, as of {})",
                                chk.cdl_hikkake_label, chk.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · inside {:.2}% · false_break {:.2}% · trigger_body {:.1}% · last_match {} · days_since {} · close {:.4}",
                                chk.bars_used,
                                chk.pattern_value,
                                chk.pattern_value_prev,
                                chk.inside_width_pct_mother,
                                chk.false_break_extension_pct,
                                chk.trigger_body_pct_range,
                                chk.last_bar_match,
                                chk.days_since_pattern,
                                chk.last_close
                            );
                            if !chk.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", chk.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(chkm)) = rx::get_cdl_hikkake_mod(&conn, &sym_upper) {
                        if chkm.cdl_hikkake_mod_label != "INSUFFICIENT_DATA"
                            && !chkm.cdl_hikkake_mod_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Modified Hikkake — CDLHIKKAKEMOD ({}, as of {})",
                                chkm.cdl_hikkake_mod_label, chkm.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · inside {:.2}% · false_break {:.2}% · confirm {:.2}% · last_match {} · days_since {} · close {:.4}",
                                chkm.bars_used,
                                chkm.pattern_value,
                                chkm.pattern_value_prev,
                                chkm.inside_width_pct_mother,
                                chkm.false_break_extension_pct,
                                chkm.confirmation_extension_pct,
                                chkm.last_bar_match,
                                chkm.days_since_pattern,
                                chkm.last_close
                            );
                            if !chkm.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", chkm.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cmh)) = rx::get_cdl_mat_hold(&conn, &sym_upper) {
                        if cmh.cdl_mat_hold_label != "INSUFFICIENT_DATA"
                            && !cmh.cdl_mat_hold_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Mat Hold — CDLMATHOLD ({}, as of {})",
                                cmh.cdl_mat_hold_label, cmh.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body1 {:.1}% · middle_avg {:.1}% · gap {:.2}% · hold_depth {:.2}% · body5 {:.1}% · last_match {} · days_since {} · close {:.4}",
                                cmh.bars_used,
                                cmh.pattern_value,
                                cmh.pattern_value_prev,
                                cmh.first_body_pct_range,
                                cmh.middle_avg_body_pct_range,
                                cmh.initial_gap_pct_range,
                                cmh.hold_depth_pct_body,
                                cmh.final_body_pct_range,
                                cmh.last_bar_match,
                                cmh.days_since_pattern,
                                cmh.last_close
                            );
                            if !cmh.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cmh.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(crf3m)) = rx::get_cdl_rise_fall_three_methods(&conn, &sym_upper)
                    {
                        if crf3m.cdl_rise_fall_three_methods_label != "INSUFFICIENT_DATA"
                            && !crf3m.cdl_rise_fall_three_methods_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Rising/Falling Three Methods — CDLRISEFALL3METHODS ({}, as of {})",
                                crf3m.cdl_rise_fall_three_methods_label, crf3m.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body1 {:.1}% · middle_avg {:.1}% · contain {:.2}% · body5 {:.1}% · last_match {} · days_since {} · close {:.4}",
                                crf3m.bars_used,
                                crf3m.pattern_value,
                                crf3m.pattern_value_prev,
                                crf3m.first_body_pct_range,
                                crf3m.middle_avg_body_pct_range,
                                crf3m.containment_pct_body,
                                crf3m.final_body_pct_range,
                                crf3m.last_bar_match,
                                crf3m.days_since_pattern,
                                crf3m.last_close
                            );
                            if !crf3m.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", crf3m.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(csp)) = rx::get_cdl_stalled_pattern(&conn, &sym_upper) {
                        if csp.cdl_stalled_pattern_label != "INSUFFICIENT_DATA"
                            && !csp.cdl_stalled_pattern_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Stalled Pattern — CDLSTALLEDPATTERN ({}, as of {})",
                                csp.cdl_stalled_pattern_label, csp.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body1 {:.1}% · body2 {:.1}% · body3 {:.1}% · gap3 {:.2}% · upper3 {:.1}% · progress {:.2}% · last_match {} · days_since {} · close {:.4}",
                                csp.bars_used,
                                csp.pattern_value,
                                csp.pattern_value_prev,
                                csp.first_body_pct_range,
                                csp.second_body_pct_range,
                                csp.third_body_pct_range,
                                csp.third_open_gap_pct_range,
                                csp.third_upper_shadow_pct,
                                csp.close_progress_pct_prev_leg,
                                csp.last_bar_match,
                                csp.days_since_pattern,
                                csp.last_close
                            );
                            if !csp.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", csp.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ctg)) = rx::get_cdl_tasuki_gap(&conn, &sym_upper) {
                        if ctg.cdl_tasuki_gap_label != "INSUFFICIENT_DATA"
                            && !ctg.cdl_tasuki_gap_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Candlestick Tasuki Gap — CDLTASUKIGAP ({}, as of {})",
                                ctg.cdl_tasuki_gap_label, ctg.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · value {} (prev {}) · body1 {:.1}% · body2 {:.1}% · body3 {:.1}% · gap {:.2}% · gap_fill {:.2}% · open3 {:.2}% body2 · last_match {} · days_since {} · close {:.4}",
                                ctg.bars_used,
                                ctg.pattern_value,
                                ctg.pattern_value_prev,
                                ctg.first_body_pct_range,
                                ctg.second_body_pct_range,
                                ctg.third_body_pct_range,
                                ctg.gap_pct_range,
                                ctg.gap_fill_pct,
                                ctg.third_open_pct_second_body,
                                ctg.last_bar_match,
                                ctg.days_since_pattern,
                                ctg.last_close
                            );
                            if !ctg.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ctg.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    // ── Round 76 packet blocks ──
                    if let Ok(Some(ms)) = rx::get_modsharpe(&conn, &sym_upper) {
                        if ms.modsharpe_label != "INSUFFICIENT_DATA"
                            && !ms.modsharpe_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Pezier-White Adjusted Sharpe — MODSHARPE ({}, as of {})",
                                ms.modsharpe_label, ms.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · ann={} · μ/bar={:+.5} · σ/bar={:.5} · skew={:+.3} · ex-kurt={:+.3}",
                                ms.bars_used,
                                ms.annualization_factor,
                                ms.mean_return_bar,
                                ms.stdev_return_bar,
                                ms.skewness,
                                ms.excess_kurtosis
                            );
                            let _ = writeln!(
                                p,
                                "- SR(ann)={:+.3} · ASR(ann)={:+.3} · adj factor={:.3}",
                                ms.sharpe_ratio, ms.adjusted_sharpe, ms.adjustment_factor
                            );
                            if !ms.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ms.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(ht)) = rx::get_hsiehtest(&conn, &sym_upper) {
                        if ht.hsieh_label != "INSUFFICIENT_DATA" && !ht.hsieh_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Hsieh 3rd-Moment Nonlinearity — HSIEHTEST ({}, as of {})",
                                ht.hsieh_label, ht.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Returns {} · AR({}) · T(1,1)={:+.4} z={:+.2} · T(2,2)={:+.4} z={:+.2} · max|z|={:.2} · c95={:.2} · reject null={}",
                                ht.bars_used,
                                ht.ar_order,
                                ht.t_11,
                                ht.z_11,
                                ht.t_22,
                                ht.z_22,
                                ht.max_abs_z,
                                ht.critical_95,
                                ht.reject_null
                            );
                            if !ht.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", ht.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(cb)) = rx::get_chowbreak(&conn, &sym_upper) {
                        if cb.chowbreak_label != "INSUFFICIENT_DATA"
                            && !cb.chowbreak_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Chow Mean-Shift Structural Break — CHOWBREAK ({}, as of {})",
                                cb.chowbreak_label, cb.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · break@{} · μ_pre={:+.5} · μ_post={:+.5} · RSS_p={:.5} · RSS_u={:.5}",
                                cb.bars_used,
                                cb.break_point_idx,
                                cb.mean_pre,
                                cb.mean_post,
                                cb.rss_pooled,
                                cb.rss_unrestricted
                            );
                            let _ = writeln!(
                                p,
                                "- F={:.3} · df=({},{}) · c95={:.2} · reject null={}",
                                cb.f_stat, cb.df_num, cb.df_den, cb.critical_95, cb.reject_null
                            );
                            if !cb.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", cb.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(db)) = rx::get_driftburst(&conn, &sym_upper) {
                        if db.driftburst_label != "INSUFFICIENT_DATA"
                            && !db.driftburst_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Christensen-Oomen-Renò Drift-Burst — DRIFTBURST ({}, as of {})",
                                db.driftburst_label, db.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · bw={:.1} · max|T|={:.3} (signed {:+.3}) · at offset {} · excursions>3={} · c99≈{:.1}",
                                db.bars_used,
                                db.kernel_bandwidth_bars,
                                db.max_abs_statistic,
                                db.max_stat_signed,
                                db.max_at_offset,
                                db.excursions_gt_3,
                                db.critical_99_approx
                            );
                            if !db.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", db.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(hc)) = rx::get_hlvclust(&conn, &sym_upper) {
                        if hc.hlvclust_label != "INSUFFICIENT_DATA" && !hc.hlvclust_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Parkinson High-Low Volatility Clustering — HLVCLUST ({}, as of {})",
                                hc.hlvclust_label, hc.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · h={} · σ_P/bar={:.5} · σ_P(ann)={:.4} · AC(1)={:+.3} · AC(5)={:+.3}",
                                hc.bars_used,
                                hc.lag_h,
                                hc.parkinson_vol_bar,
                                hc.parkinson_vol_annualised,
                                hc.ac_lag1,
                                hc.ac_lag5
                            );
                            let _ = writeln!(
                                p,
                                "- LB-Q={:.3} · c95={:.3} · p={:.4} · reject null={}",
                                hc.lb_q_stat, hc.critical_95, hc.p_value, hc.reject_null
                            );
                            if !hc.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", hc.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(yz)) = rx::get_yangzhang(&conn, &sym_upper) {
                        if yz.yangzhang_label != "INSUFFICIENT_DATA"
                            && !yz.yangzhang_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Yang-Zhang Range-Volatility Estimator — YANGZHANG ({}, as of {})",
                                yz.yangzhang_label, yz.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · σ²_O={:.3e} · σ²_C={:.3e} · σ²_RS={:.3e} · k={:.4}",
                                yz.bars_used,
                                yz.overnight_var,
                                yz.open_to_close_var,
                                yz.rs_component,
                                yz.k_weight
                            );
                            let _ = writeln!(
                                p,
                                "- σ_YZ/bar={:.6} · σ_YZ(ann)={:.2}% · σ_CC(ann)={:.2}% · eff={:.3}×",
                                yz.yz_vol_bar,
                                yz.yz_vol_annualised_pct,
                                yz.cc_vol_annualised_pct,
                                yz.efficiency_vs_close
                            );
                            if !yz.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", yz.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(kp)) = rx::get_kuiper(&conn, &sym_upper) {
                        if kp.kuiper_label != "INSUFFICIENT_DATA" && !kp.kuiper_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Kuiper Two-Sided CDF vs Normal — KUIPER ({}, as of {})",
                                kp.kuiper_label, kp.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · μ={:+.6} · σ={:.6} · D⁺={:.4} · D⁻={:.4} · V={:.4}",
                                kp.bars_used, kp.mean, kp.stdev, kp.d_plus, kp.d_minus, kp.v_stat
                            );
                            let _ = writeln!(
                                p,
                                "- V*={:.3} · c95={:.3} · p≈{:.4} · reject null={}",
                                kp.v_stat_adj, kp.critical_95, kp.p_value_approx, kp.reject_null
                            );
                            if !kp.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", kp.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(dg)) = rx::get_dagostino(&conn, &sym_upper) {
                        if dg.dagostino_label != "INSUFFICIENT_DATA"
                            && !dg.dagostino_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### D'Agostino-Pearson K² Omnibus Normality — DAGOSTINO ({}, as of {})",
                                dg.dagostino_label, dg.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · skew={:+.4} · excess kurt={:+.4} · z_skew={:+.3} · z_kurt={:+.3}",
                                dg.bars_used, dg.skewness, dg.excess_kurtosis, dg.z_skew, dg.z_kurt
                            );
                            let _ = writeln!(
                                p,
                                "- K²={:.3} · c95={:.3} · p={:.4} · reject null={}",
                                dg.k2_stat, dg.critical_95, dg.p_value, dg.reject_null
                            );
                            if !dg.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", dg.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(bp)) = rx::get_baiperron(&conn, &sym_upper) {
                        if bp.baiperron_label != "INSUFFICIENT_DATA"
                            && !bp.baiperron_label.is_empty()
                        {
                            let _ = writeln!(
                                p,
                                "### Bai-Perron sup-F Structural Break Search — BAIPERRON ({}, as of {})",
                                bp.baiperron_label, bp.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · π₀={:.2} · search [{}, {}] · best={} · sup-F={:.3}",
                                bp.bars_used,
                                bp.trim_fraction,
                                bp.search_lo,
                                bp.search_hi,
                                bp.best_break_idx,
                                bp.sup_f_stat
                            );
                            let _ = writeln!(
                                p,
                                "- μ_pre={:+.6} · μ_post={:+.6} · RSS₀={:.3e} · RSS*={:.3e} · c95={:.2} · p≈{:.4} · reject null={}",
                                bp.mean_pre,
                                bp.mean_post,
                                bp.rss_no_break,
                                bp.rss_at_best,
                                bp.critical_95,
                                bp.p_value_approx,
                                bp.reject_null
                            );
                            if !bp.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", bp.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    if let Ok(Some(kp2)) = rx::get_kupiecpof(&conn, &sym_upper) {
                        if kp2.kupiec_label != "INSUFFICIENT_DATA" && !kp2.kupiec_label.is_empty() {
                            let _ = writeln!(
                                p,
                                "### Kupiec Proportion-of-Failures VaR Backtest — KUPIECPOF ({}, as of {})",
                                kp2.kupiec_label, kp2.as_of
                            );
                            let _ = writeln!(
                                p,
                                "- Bars {} · α={:.2}% (nominal {:.2}%) · window {} · test {} · VaR(last)={:.6}",
                                kp2.bars_used,
                                kp2.confidence_level * 100.0,
                                kp2.nominal_exceedance_rate * 100.0,
                                kp2.rolling_window,
                                kp2.test_window,
                                kp2.var_latest_bar
                            );
                            let _ = writeln!(
                                p,
                                "- exceed obs={} · exp={:.2} · rate={:.3}% · LR_POF={:.3} · c95={:.3} · p={:.4} · reject null={}",
                                kp2.n_exceedances,
                                kp2.expected_exceedances,
                                kp2.realised_exceedance_rate * 100.0,
                                kp2.lr_pof_stat,
                                kp2.critical_95,
                                kp2.p_value,
                                kp2.reject_null
                            );
                            if !kp2.note.is_empty() {
                                let _ = writeln!(p, "- Note: {}", kp2.note);
                            }
                            let _ = writeln!(p);
                        }
                    }

                    // ── prior-ingested web research (if any) ──
                    if let Ok(Some(ing)) = rx::get_ingested_articles(&conn, &sym_upper) {
                        if !ing.articles.is_empty() {
                            // Char limits match the news section so a long body
                            // doesn't blow the token budget when a chatty agent
                            // dumps multi-thousand-character article text.
                            const INGEST_BODY_CHAR_LIMIT: usize = 1500;
                            const INGEST_SUMMARY_CHAR_LIMIT: usize = 260;
                            let bodies_present = ing
                                .articles
                                .iter()
                                .take(15)
                                .filter(|a| !a.body.is_empty())
                                .count();
                            let _ = writeln!(
                                p,
                                "### Prior Ingested Web Research — INGESTED ({} articles, {} with body)",
                                ing.articles.len(),
                                bodies_present
                            );
                            for a in ing.articles.iter().take(15) {
                                let src = if !a.source.is_empty() {
                                    &a.source
                                } else {
                                    "—"
                                };
                                let when = if !a.published_at.is_empty() {
                                    a.published_at.as_str()
                                } else {
                                    "—"
                                };
                                let agent = if !a.agent_used.is_empty() {
                                    a.agent_used.as_str()
                                } else {
                                    "—"
                                };
                                let title = if a.title.is_empty() {
                                    "(untitled)"
                                } else {
                                    a.title.as_str()
                                };
                                let _ = writeln!(
                                    p,
                                    "- **{}** — {} · {} · via {}",
                                    title, src, when, agent
                                );
                                if !a.summary.is_empty() {
                                    // char-aware truncate so multi-byte UTF-8 sequences
                                    // (em-dashes, smart quotes, accented letters) don't
                                    // get sliced mid-code-point.
                                    let s = if a.summary.chars().count() > INGEST_SUMMARY_CHAR_LIMIT
                                    {
                                        let mut buf = a
                                            .summary
                                            .chars()
                                            .take(INGEST_SUMMARY_CHAR_LIMIT)
                                            .collect::<String>();
                                        buf.push('…');
                                        buf
                                    } else {
                                        a.summary.clone()
                                    };
                                    let _ = writeln!(p, "  - {}", s);
                                }
                                if !a.body.is_empty() {
                                    let b = if a.body.chars().count() > INGEST_BODY_CHAR_LIMIT {
                                        let mut buf = a
                                            .body
                                            .chars()
                                            .take(INGEST_BODY_CHAR_LIMIT)
                                            .collect::<String>();
                                        buf.push('…');
                                        buf
                                    } else {
                                        a.body.clone()
                                    };
                                    let _ = writeln!(p, "  - Body: {}", b);
                                }
                                if !a.url.is_empty() {
                                    let _ = writeln!(p, "  - {}", a.url);
                                }
                            }
                            if ing.articles.len() > 15 {
                                let _ = writeln!(
                                    p,
                                    "- ({} more articles in cache, not shown)",
                                    ing.articles.len() - 15
                                );
                            }
                            let _ = writeln!(p);
                        }
                    }
                }
            }

            self.write_symbol_sector_peer_comparison(p, &sym_upper, fund);
        }
    }
}
