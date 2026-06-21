use super::*;
mod cached_research;
mod capital_valuation_sections;
mod composite_signal_sections;
mod composite_signal_early;
mod composite_signal_blocks;
mod distribution_risk_sections;
mod fractal_tail_stationarity_sections;
mod fundamental_risk_sections;
mod market_behavior_sections;
mod momentum_volume_indicator_sections;
mod moving_average_research_sections;
mod overview;
mod ownership_price_history;
mod peer_comparison;
mod price_behavior_distribution;
mod price_behavior_illiquidity_norm;
mod price_behavior_local;
mod price_behavior_ratios;
mod price_behavior_risk_metrics;
mod price_behavior_seasonality_vol;
mod price_behavior_sections;
mod price_behavior_stat_tests;
mod price_behavior_tests_ratios;
mod price_behavior_vol_estimators;
mod price_transform_indicator_sections;
mod rank_drift_core_ranks;
mod rank_drift_fund_quality;
mod rank_drift_research_ranks;
mod rank_drift_liquidity_streaks;
mod rank_drift_div_earn_streaks;
mod rank_drift_yield_short_conc;
mod rank_drift_vol_perf;
mod rank_drift_cone_corrs;
mod rank_drift_accs_vrp;
mod rank_drift_growth_drift;
mod rank_drift_sections;
mod recent_news;
mod talib_price_momentum_sections;
mod technical_indicator_sections;
mod technical_indicator_squeeze_breakouts;
mod technical_indicator_cloud_trend;
mod technical_indicator_oscillators;
mod technical_indicator_volume_trend;
mod technical_indicator_final_osc;

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
                    let mut n_buys = 0usize;
                    let mut n_sells = 0usize;
                    let mut buy_value = 0.0f64;
                    let mut sell_value = 0.0f64;
                    for t in trades.iter() {
                        let typ = t.transaction_type.as_str();
                        let is_buy =
                            typ.eq_ignore_ascii_case("P") || typ.to_lowercase().contains("buy");
                        let is_sell =
                            typ.eq_ignore_ascii_case("S") || typ.to_lowercase().contains("sell");
                        if is_buy {
                            n_buys += 1;
                            buy_value += t.aggregate_value;
                        }
                        if is_sell {
                            n_sells += 1;
                            sell_value += t.aggregate_value;
                        }
                    }
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

                    self.write_symbol_moving_average_research_sections(p, &sym_upper);

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

                    self.write_symbol_momentum_volume_indicator_sections(p, &sym_upper);

                    self.write_symbol_price_transform_indicator_sections(p, &sym_upper);

                    self.write_symbol_talib_price_momentum_sections(p, &sym_upper);

                    // Candlestick pattern storage/helpers
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

                    // ── Research section ──
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

                    // ── Research section ──
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

                    // ── Research section ──
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

                    // ── Research section ──
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
