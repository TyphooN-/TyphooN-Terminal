use self::context::SymbolResearchContext;
use super::*;
mod cached_research;
mod capital_valuation_sections;
mod composite_signal_blocks;
mod composite_signal_early;
mod composite_signal_factors;
mod composite_signal_sections;
mod context;
mod dispatcher_inline_sections;
mod distribution_risk_sections;
mod format;
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
mod price_transform_adaptive_osc;
mod price_transform_indicator_sections;
mod price_transform_linear_hilbert;
mod price_transform_regression_phase;
mod price_transform_volatility_force;
mod rank_drift_accs_vrp;
mod rank_drift_cone_corrs;
mod rank_drift_core_ranks;
mod rank_drift_div_earn_streaks;
mod rank_drift_fund_quality;
mod rank_drift_growth_drift;
mod rank_drift_liquidity_streaks;
mod rank_drift_research_ranks;
mod rank_drift_sections;
mod rank_drift_vol_perf;
mod rank_drift_yield_short_conc;
mod recent_news;
mod talib_dmi_movement;
mod talib_extended_emitters;
mod talib_momentum_range;
mod talib_price_momentum_sections;
mod talib_price_ohlc_stats;
mod technical_indicator_cloud_trend;
mod technical_indicator_final_osc;
mod technical_indicator_oscillators;
mod technical_indicator_sections;
mod technical_indicator_squeeze_breakouts;
mod technical_indicator_volume_trend;

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
                if let Ok(conn) = cache.open_bg_read_connection() {
                    // ADR-125 step 3: the connection is acquired exactly once here
                    // (an independent read connection, so it never contends with the
                    // render thread's `read_conn`) and threaded to every section via
                    // the read-only context. No section re-acquires `read_conn`.
                    let ctx = SymbolResearchContext { conn: &conn };

                    ownership_price_history::write_symbol_ownership_price_history_sections(
                        &ctx, p, &sym_upper,
                    );

                    capital_valuation_sections::write_symbol_capital_valuation_sections(
                        &ctx, p, &sym_upper,
                    );

                    market_behavior_sections::write_symbol_market_behavior_sections(
                        &ctx, p, &sym_upper,
                    );

                    fundamental_risk_sections::write_symbol_fundamental_risk_sections(
                        &ctx, p, &sym_upper,
                    );

                    composite_signal_sections::write_symbol_composite_signal_sections(
                        &ctx, p, &sym_upper,
                    );

                    rank_drift_sections::write_symbol_rank_drift_sections(&ctx, p, &sym_upper);

                    price_behavior_sections::write_symbol_price_behavior_sections(
                        &ctx, p, &sym_upper,
                    );

                    distribution_risk_sections::write_symbol_distribution_risk_sections(
                        &ctx, p, &sym_upper,
                    );

                    fractal_tail_stationarity_sections::write_symbol_fractal_tail_stationarity_sections(&ctx, p, &sym_upper);

                    technical_indicator_sections::write_symbol_technical_indicator_sections(
                        &ctx, p, &sym_upper,
                    );

                    moving_average_research_sections::write_symbol_moving_average_research_sections(
                        &ctx, p, &sym_upper,
                    );

                    dispatcher_inline_sections::write_expiration_calendar(&ctx, p, &sym_upper);

                    momentum_volume_indicator_sections::write_symbol_momentum_volume_indicator_sections(&ctx, p, &sym_upper);

                    price_transform_indicator_sections::write_symbol_price_transform_indicator_sections(&ctx, p, &sym_upper);

                    talib_price_momentum_sections::write_symbol_talib_price_momentum_sections(
                        &ctx, p, &sym_upper,
                    );

                    dispatcher_inline_sections::write_candlestick_and_stats(&ctx, p, &sym_upper);
                }
            }

            self.write_symbol_sector_peer_comparison(p, &sym_upper, fund);
        }
    }
}
