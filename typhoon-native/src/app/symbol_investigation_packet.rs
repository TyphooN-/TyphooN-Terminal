// Packet section tree now lives in the typhoon-research-ui crate (ADR-125
// Phase 2). Re-exported so the dispatcher's section calls (e.g.
// `capital_valuation_sections::write_…`) and `SymbolResearchContext` resolve
// unchanged; the dispatcher itself (gathers app state, builds the context)
// stays here in native.
use typhoon_research_ui::packet::context::SymbolResearchContext;
use typhoon_research_ui::packet::*;
use super::*;

impl TyphooNApp {
    pub(super) fn write_symbol_investigation_sections(&self, p: &mut String, syms: &[String]) {
        use std::fmt::Write as _;
        // Per-symbol section
        for sym_raw in syms {
            let sym_upper = sym_raw.to_uppercase();
            let _ = writeln!(p, "---");
            let _ = writeln!(p, "## {sym_upper}");

            let fund = self
                .bg
                .all_fundamentals
                .iter()
                .find(|f| f.symbol.eq_ignore_ascii_case(&sym_upper));
            overview::write_symbol_investigation_overview_sections(
                p,
                &sym_upper,
                fund,
                &self.live_positions,
                &self.kr_positions,
            );

            // Quarterly financials + top institutional holders (from DB).
            if let Some(ref cache) = self.cache {
                dispatcher_inline_sections::write_quarterly_and_holders(cache, p, &sym_upper);
            }

            // Recent SEC filings (from the bg.sec_filings cache).
            dispatcher_inline_sections::write_sec_filings(p, &sym_upper, &self.bg.sec_filings);

            // Insider trade summary (from the bg.insider_trades cache).
            dispatcher_inline_sections::write_insider_activity(
                p,
                self.bg.insider_trades.get(&sym_upper).map(|v| v.as_slice()),
            );

            // Price & volatility stats (from the D1 bar cache).
            if let Some(ref cache) = self.cache {
                dispatcher_inline_sections::write_price_volatility(cache, p, &sym_upper);
            }

            // Recent news (fetched from the DB) + cached research surfaces.
            if let Some(ref cache) = self.cache {
                dispatcher_inline_sections::write_recent_news(cache, p, &sym_upper);
            }

            if let Some(ref cache) = self.cache {
                if let Ok(conn) = cache.open_bg_read_connection() {
                    // ADR-125 step 3: the connection is acquired exactly once here
                    // (an independent read connection, so it never contends with the
                    // render thread's `read_conn`) and threaded to every section via
                    // the read-only context. No section re-acquires `read_conn`.
                    let ctx = SymbolResearchContext { conn: &conn };

                    cached_research::write_symbol_cached_research_surfaces(&ctx, p, &sym_upper);

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

            peer_comparison::write_symbol_sector_peer_comparison(
                p,
                &sym_upper,
                fund,
                &self.bg.all_fundamentals,
            );
        }
    }
}
