use super::*;

impl TyphooNApp {
    pub(super) fn handle_research_macro_valuation_msg(&mut self, msg: BrokerMsg) {
        match msg {
            // ── Round 6 receive arms ──
            BrokerMsg::WorldIndicesMsg(rows) => {
                self.wei_indices = rows.clone();
                self.wei_loading = false;
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_world_indices(&conn, &rows);
                    }
                }
            }
            BrokerMsg::MarketMoversMsg(movers) => {
                self.market_movers = movers.clone();
                self.mov_loading = false;
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_market_movers(&conn, &movers);
                    }
                }
            }
            BrokerMsg::SectorPerformanceMsg(rows) => {
                self.sector_perf = rows.clone();
                self.indu_loading = false;
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_sector_performance(&conn, &rows);
                    }
                }
            }
            BrokerMsg::WaccSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.wacc_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.wacc_snapshot = snap.clone();
                    self.wacc_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_wacc(&conn, &sym_u, &snap);
                    }
                }
            }
            // FX, beta, valuation, and identifier research
            BrokerMsg::CurrencyRatesMsg(rows) => {
                self.wcr_rates = rows.clone();
                self.wcr_loading = false;
                self.log
                    .push_back(LogEntry::info(format!("WCR: {} rates loaded", rows.len())));
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_currency_rates(&conn, &rows);
                    }
                }
            }
            BrokerMsg::BetaSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.beta_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.beta_snapshot = snap.clone();
                    self.beta_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_beta(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::DdmSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ddm_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ddm_snapshot = snap.clone();
                    self.ddm_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_ddm(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::RelativeValuationMsg(sym, rv) => {
                let sym_u = sym.to_uppercase();
                if self.rv_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.rv_snapshot = rv.clone();
                    self.rv_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_relative_valuation(
                            &conn, &sym_u, &rv,
                        );
                    }
                }
            }
            BrokerMsg::FigiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.figi_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.figi_snapshot = snap.clone();
                    self.figi_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_figi(&conn, &sym_u, &snap);
                    }
                }
            }
            // ── Round 8 receive arms ──
            BrokerMsg::HraSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.hra_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.hra_snapshot = snap.clone();
                    self.hra_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_hra(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::DcfSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.dcf_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.dcf_snapshot = snap.clone();
                    self.dcf_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_dcf(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::SvmSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.svm_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.svm_snapshot = snap.clone();
                    self.svm_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_svm(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::OptionsChainMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.omon_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.omon_snapshot = snap.clone();
                    self.omon_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_options_chain(
                            &conn, &sym_u, &snap,
                        );
                    }
                }
            }
            BrokerMsg::IvolSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ivol_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ivol_snapshot = snap.clone();
                    self.ivol_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_ivol(&conn, &sym_u, &snap);
                    }
                }
            }
            // ── Round 9 receive arms ──
            BrokerMsg::SeasonalitySnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.seag_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.seag_snapshot = snap.clone();
                    self.seag_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_seasonality(
                            &conn, &sym_u, &snap,
                        );
                    }
                }
            }
            BrokerMsg::CorrelationMatrixMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cor_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cor_snapshot = snap.clone();
                    self.cor_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_correlation(
                            &conn, &sym_u, &snap,
                        );
                    }
                }
            }
            BrokerMsg::TotalReturnSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.tra_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.tra_snapshot = snap.clone();
                    self.tra_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_total_return(
                            &conn, &sym_u, &snap,
                        );
                    }
                }
            }
            BrokerMsg::TechnicalsSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.tech_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.tech_snapshot = snap.clone();
                    self.tech_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_technicals(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::VolSkewSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.skew_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.skew_snapshot = snap.clone();
                    self.skew_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_vol_skew(&conn, &sym_u, &snap);
                    }
                }
            }
            _ => {}
        }
    }
}
