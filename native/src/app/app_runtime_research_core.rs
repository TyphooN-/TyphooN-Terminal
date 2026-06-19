use super::*;

impl TyphooNApp {
    pub(super) fn handle_research_core_msg(&mut self, msg: BrokerMsg) {
        match msg {
            // ── Godel parity results (ADR-107) ──
            BrokerMsg::CompanyProfile(profile) => {
                self.desc_loading = false;
                let sym_u = profile.symbol.to_uppercase();
                if self.desc_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.desc_profile = Some(profile.clone());
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_profile(&conn, &profile);
                    }
                }
            }
            BrokerMsg::StockPeers(sym, peers) => {
                let sym_u = sym.to_uppercase();
                if self.peers_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.peers_list = peers.clone();
                    self.peers_loading = false;
                }
                if self.desc_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.desc_peers = peers.clone();
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_peers(&conn, &sym_u, &peers);
                    }
                }
            }
            BrokerMsg::EarningsHistory(sym, rows) => {
                let sym_u = sym.to_uppercase();
                if self.earnings_history_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.earnings_history_rows = rows.clone();
                    self.earnings_history_loading = false;
                }
                if self.desc_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.desc_earnings = rows.clone();
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_earnings_history(
                            &conn, &sym_u, &rows,
                        );
                    }
                }
            }
            BrokerMsg::IpoCalendar(rows) => {
                self.ipo_events = rows.clone();
                self.ipo_loading = false;
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_ipo_calendar(&conn, &rows);
                    }
                }
                self.log.push_back(LogEntry::info(format!(
                    "IPO calendar: {} events",
                    self.ipo_events.len()
                )));
            }
            BrokerMsg::PressReleases(sym, rows) => {
                let sym_u = sym.to_uppercase();
                if self.press_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.press_releases_list = rows.clone();
                    self.press_loading = false;
                }
                if self.desc_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.desc_press = rows.clone();
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_press_releases(
                            &conn, &sym_u, &rows,
                        );
                    }
                }
            }
            BrokerMsg::SocialSentiment(sym, rows) => {
                let sym_u = sym.to_uppercase();
                if self.sentiment_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.sentiment_rows = rows.clone();
                    self.sentiment_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_sentiment(&conn, &sym_u, &rows);
                    }
                }
            }
            BrokerMsg::TranscriptList(sym, rows) => {
                let sym_u = sym.to_uppercase();
                if self.transcripts_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.transcripts_list = rows.clone();
                    self.transcripts_loading_list = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_transcript_list(
                            &conn, &sym_u, &rows,
                        );
                    }
                }
            }
            BrokerMsg::TranscriptBody(t) => {
                if self.transcripts_symbol.eq_ignore_ascii_case(&t.symbol) {
                    self.transcripts_body = Some(t.clone());
                    self.transcripts_loading_body = false;
                    self.transcripts_summary = None;
                    self.transcripts_summary_for = (String::new(), 0, 0);
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_transcript(&conn, &t);
                    }
                }
            }
            BrokerMsg::CommoditiesQuotes(quotes) => {
                self.commodities_quotes = quotes;
                self.commodities_loading = false;
                self.commodities_last_fetch = Some(std::time::Instant::now());
            }
            BrokerMsg::DividendHistory(sym, rows) => {
                let sym_u = sym.to_uppercase();
                if self.dividend_history_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.dividend_history = rows.clone();
                    self.dividend_history_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_dividends(&conn, &sym_u, &rows);
                    }
                }
            }
            BrokerMsg::EarningsEstimates(sym, rows) => {
                let sym_u = sym.to_uppercase();
                if self.earnings_estimates_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.earnings_estimates = rows.clone();
                    self.earnings_estimates_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_earnings_estimates(
                            &conn, &sym_u, &rows,
                        );
                    }
                }
            }
            BrokerMsg::RatingChanges(sym, rows) => {
                let sym_u = sym.to_uppercase();
                if self.rating_changes_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.rating_changes = rows.clone();
                    self.rating_changes_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_rating_changes(
                            &conn, &sym_u, &rows,
                        );
                    }
                }
            }
            BrokerMsg::TreasuryYields(rows) => {
                self.treasury_yields = rows;
                self.treasury_yields_loading = false;
                self.treasury_yields_last_fetch = Some(std::time::Instant::now());
            }
            BrokerMsg::FinancialStatementsMsg(sym, bundle) => {
                let sym_u = sym.to_uppercase();
                if self.financials_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.financials = bundle.clone();
                    self.financials_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_financials(
                            &conn, &sym_u, &bundle,
                        );
                    }
                }
            }
            BrokerMsg::Executives(sym, rows) => {
                let sym_u = sym.to_uppercase();
                if self.executives_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.executives = rows.clone();
                    self.executives_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_executives(&conn, &sym_u, &rows);
                    }
                }
            }
            BrokerMsg::CotReports(rows) => {
                self.cot_reports = rows;
                self.cot_loading = false;
                self.cot_last_fetch = Some(std::time::Instant::now());
            }
            BrokerMsg::StockSplitsMsg(sym, rows) => {
                let sym_u = sym.to_uppercase();
                if self.splits_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.splits_list = rows.clone();
                    self.splits_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    let existing = cache
                        .connection()
                        .ok()
                        .and_then(|conn| {
                            typhoon_engine::core::research::get_stock_splits(&conn, &sym_u).ok()
                        })
                        .flatten()
                        .unwrap_or_default();
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_stock_splits(
                            &conn, &sym_u, &rows,
                        );
                    }
                    if stock_splits_need_bar_cache_invalidation(&existing, &rows) {
                        match cache.delete_equity_bar_cache_for_symbol(&sym_u) {
                            Ok(deleted) if deleted > 0 => self.log.push_back(LogEntry::warn(format!(
                                "Corporate action cache reset: deleted {deleted} stale bar cache row(s) for {sym_u}; refetch full adjusted bars before trusting the chart"
                            ))),
                            Ok(_) => {}
                            Err(e) => self.log.push_back(LogEntry::err(format!(
                                "Corporate action cache reset failed for {sym_u}: {e}"
                            ))),
                        }
                    }
                }
            }
            BrokerMsg::EtfHoldingsMsg(sym, rows) => {
                let sym_u = sym.to_uppercase();
                if self.etf_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.etf_holdings = rows.clone();
                    self.etf_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_etf_holdings(
                            &conn, &sym_u, &rows,
                        );
                    }
                }
            }
            BrokerMsg::AnalystRecsMsg(sym, rows) => {
                let sym_u = sym.to_uppercase();
                if self.anr_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.analyst_recs = rows.clone();
                    self.anr_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_analyst_recs(
                            &conn, &sym_u, &rows,
                        );
                    }
                }
            }
            BrokerMsg::PriceTargetMsg(sym, pt) => {
                let sym_u = sym.to_uppercase();
                if self.anr_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.price_target = pt.clone();
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_price_target(&conn, &sym_u, &pt);
                    }
                }
            }
            BrokerMsg::EsgScoresMsg(sym, rows) => {
                let sym_u = sym.to_uppercase();
                if self.esg_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.esg_rows = rows.clone();
                    self.esg_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_esg(&conn, &sym_u, &rows);
                    }
                }
            }
            BrokerMsg::IndexMembersMsg(index_code, rows) => {
                let code_u = index_code.to_uppercase();
                if self.index_code.eq_ignore_ascii_case(&code_u) {
                    self.index_members = rows.clone();
                    self.memb_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_index_members(
                            &conn, &code_u, &rows,
                        );
                    }
                }
            }
            BrokerMsg::InsiderTradesMsg(sym, rows) => {
                let sym_u = sym.to_uppercase();
                if self.insider_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.insider_trades = rows.clone();
                    self.insider_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_insider_trades(
                            &conn, &sym_u, &rows,
                        );
                    }
                }
            }
            BrokerMsg::InstitutionalHoldersMsg(sym, rows) => {
                let sym_u = sym.to_uppercase();
                if self.inst_holders_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.institutional_holders = rows.clone();
                    self.inst_holders_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_institutional_holders(
                            &conn, &sym_u, &rows,
                        );
                    }
                }
            }
            BrokerMsg::SharesFloatMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.float_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.shares_float = snap.clone();
                    self.float_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_shares_float(
                            &conn, &sym_u, &snap,
                        );
                    }
                }
            }
            BrokerMsg::HistoricalPriceMsg(sym, rows) => {
                let sym_u = sym.to_uppercase();
                if self.hp_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.hp_rows = rows.clone();
                    self.hp_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_historical_price(
                            &conn, &sym_u, &rows,
                        );
                    }
                }
            }
            BrokerMsg::EarningsSurpriseMsg(sym, rows) => {
                let sym_u = sym.to_uppercase();
                if self.eps_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.eps_surprises = rows.clone();
                    self.eps_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_earnings_surprises(
                            &conn, &sym_u, &rows,
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

fn stock_splits_need_bar_cache_invalidation(
    existing: &[typhoon_engine::core::research::StockSplit],
    incoming: &[typhoon_engine::core::research::StockSplit],
) -> bool {
    incoming.iter().any(|split| {
        split.numerator > 0.0
            && split.denominator > 0.0
            && (split.denominator / split.numerator) >= 2.0
            && !existing.iter().any(|old| {
                old.date == split.date
                    && (old.numerator - split.numerator).abs() < 1e-9
                    && (old.denominator - split.denominator).abs() < 1e-9
            })
    })
}

#[cfg(test)]
mod tests {
    use super::stock_splits_need_bar_cache_invalidation;
    use typhoon_engine::core::research::StockSplit;

    fn split(date: &str, numerator: f64, denominator: f64) -> StockSplit {
        StockSplit {
            date: date.to_string(),
            label: format!("{numerator}:{denominator}"),
            numerator,
            denominator,
        }
    }

    #[test]
    fn new_reverse_split_invalidates_bar_cache() {
        assert!(stock_splits_need_bar_cache_invalidation(
            &[],
            &[split("2026-06-19", 1.0, 100.0)]
        ));
    }

    #[test]
    fn existing_or_forward_splits_do_not_invalidate() {
        let existing = [split("2026-06-19", 1.0, 100.0)];
        assert!(!stock_splits_need_bar_cache_invalidation(
            &existing,
            &[split("2026-06-19", 1.0, 100.0)]
        ));
        assert!(!stock_splits_need_bar_cache_invalidation(
            &[],
            &[split("2026-06-19", 2.0, 1.0)]
        ));
    }
}
