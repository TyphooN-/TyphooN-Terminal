use super::*;

impl TyphooNApp {
    pub(super) fn handle_indicator_snapshot_msg(&mut self, msg: BrokerMsg) {
        match msg {
            // ── Round 40 receive ──
            BrokerMsg::DurbinWatsonSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.durbinwatson_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.durbinwatson_snapshot = snap.clone();
                    self.durbinwatson_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_durbinwatson(
                            &conn, &sym_u, &snap,
                        );
                    }
                }
            }
            BrokerMsg::BdsTestSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.bdstest_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.bdstest_snapshot = snap.clone();
                    self.bdstest_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_bdstest(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::BreuschPaganSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.breuschpagan_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.breuschpagan_snapshot = snap.clone();
                    self.breuschpagan_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_breuschpagan(
                            &conn, &sym_u, &snap,
                        );
                    }
                }
            }
            BrokerMsg::TurnPtsSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.turnpts_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.turnpts_snapshot = snap.clone();
                    self.turnpts_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_turnpts(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::PeriodogramSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.periodogram_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.periodogram_snapshot = snap.clone();
                    self.periodogram_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_periodogram(
                            &conn, &sym_u, &snap,
                        );
                    }
                }
            }
            BrokerMsg::McLeodLiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.mcleodli_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.mcleodli_snapshot = snap.clone();
                    self.mcleodli_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_mcleodli(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::OuFitSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.oufit_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.oufit_snapshot = snap.clone();
                    self.oufit_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_oufit(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::GphSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.gph_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.gph_snapshot = snap.clone();
                    self.gph_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_gph(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::BurgSpecSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.burgspec_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.burgspec_snapshot = snap.clone();
                    self.burgspec_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_burgspec(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::KendallTauSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.kendalltau_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.kendalltau_snapshot = snap.clone();
                    self.kendalltau_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_kendalltau(&conn, &sym_u, &snap);
                    }
                }
            }
            // ── Round 42 receive arms ──
            BrokerMsg::SqueezeSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.squeeze_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.squeeze_win_snapshot = snap.clone();
                    self.squeeze_win_loading = false;
                }
                // Upsert is already performed inside the broker handler.
                let _ = snap;
            }
            BrokerMsg::SqueezeRankSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.squeezerank_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.squeezerank_snapshot = snap.clone();
                    self.squeezerank_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::SqueezeWatchlistLoaded(rows) => {
                self.squeeze_watchlist_rows = rows;
                self.squeeze_watchlist_loading = false;
            }
            BrokerMsg::BbsqueezeSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.bbsqueeze_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.bbsqueeze_snapshot = snap.clone();
                    self.bbsqueeze_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::DonchianSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.donchian_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.donchian_win_snapshot = snap.clone();
                    self.donchian_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::KamaSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.kama_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.kama_win_snapshot = snap.clone();
                    self.kama_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::IchimokuSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ichimoku_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ichimoku_win_snapshot = snap.clone();
                    self.ichimoku_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::SupertrendSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.supertrend_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.supertrend_win_snapshot = snap.clone();
                    self.supertrend_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::KeltnerSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.keltner_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.keltner_win_snapshot = snap.clone();
                    self.keltner_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::FisherSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.fisher_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.fisher_win_snapshot = snap.clone();
                    self.fisher_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::AroonSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.aroon_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.aroon_win_snapshot = snap.clone();
                    self.aroon_win_loading = false;
                }
                let _ = snap;
            }
            // ── Round 44 receive arms ──
            BrokerMsg::AdxSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.adx_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.adx_win_snapshot = snap.clone();
                    self.adx_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CciSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cci_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cci_win_snapshot = snap.clone();
                    self.cci_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CmfSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cmf_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cmf_win_snapshot = snap.clone();
                    self.cmf_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::MfiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.mfi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.mfi_win_snapshot = snap.clone();
                    self.mfi_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::PsarSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.psar_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.psar_win_snapshot = snap.clone();
                    self.psar_win_loading = false;
                }
                let _ = snap;
            }
            // ── Round 45 receive arms ──
            BrokerMsg::VortexSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.vortex_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.vortex_win_snapshot = snap.clone();
                    self.vortex_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::ChopSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.chop_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.chop_win_snapshot = snap.clone();
                    self.chop_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::ObvSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.obv_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.obv_win_snapshot = snap.clone();
                    self.obv_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::TrixSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.trix_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.trix_win_snapshot = snap.clone();
                    self.trix_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::HmaSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.hma_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.hma_win_snapshot = snap.clone();
                    self.hma_win_loading = false;
                }
                let _ = snap;
            }
            // ── Round 46 receive arms ──
            BrokerMsg::PpoSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ppo_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ppo_win_snapshot = snap.clone();
                    self.ppo_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::DpoSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.dpo_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.dpo_win_snapshot = snap.clone();
                    self.dpo_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::KstSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.kst_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.kst_win_snapshot = snap.clone();
                    self.kst_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::UltoscSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ultosc_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ultosc_win_snapshot = snap.clone();
                    self.ultosc_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::WillrSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.willr_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.willr_win_snapshot = snap.clone();
                    self.willr_win_loading = false;
                }
                let _ = snap;
            }
            // ── Round 47 receive arms ──
            BrokerMsg::MassSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.mass_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.mass_win_snapshot = snap.clone();
                    self.mass_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::ChaikoscSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.chaikosc_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.chaikosc_win_snapshot = snap.clone();
                    self.chaikosc_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::KlingerSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.klinger_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.klinger_win_snapshot = snap.clone();
                    self.klinger_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::StochRsiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.stochrsi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.stochrsi_win_snapshot = snap.clone();
                    self.stochrsi_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::AwesomeSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.awesome_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.awesome_win_snapshot = snap.clone();
                    self.awesome_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::EfiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.efi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.efi_win_snapshot = snap.clone();
                    self.efi_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::EmvSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.emv_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.emv_win_snapshot = snap.clone();
                    self.emv_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::NviSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.nvi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.nvi_win_snapshot = snap.clone();
                    self.nvi_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::PviSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.pvi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.pvi_win_snapshot = snap.clone();
                    self.pvi_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CoppockSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.coppock_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.coppock_win_snapshot = snap.clone();
                    self.coppock_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CmoSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cmo_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cmo_win_snapshot = snap.clone();
                    self.cmo_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::QstickSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.qstick_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.qstick_win_snapshot = snap.clone();
                    self.qstick_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::DisparitySnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.disparity_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.disparity_win_snapshot = snap.clone();
                    self.disparity_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::BopSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.bop_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.bop_win_snapshot = snap.clone();
                    self.bop_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::SchaffSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.schaff_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.schaff_win_snapshot = snap.clone();
                    self.schaff_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::StochSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.stoch_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.stoch_win_snapshot = snap.clone();
                    self.stoch_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::MacdSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.macd_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.macd_win_snapshot = snap.clone();
                    self.macd_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::VwapSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.vwap_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.vwap_win_snapshot = snap.clone();
                    self.vwap_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::McgdSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.mcgd_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.mcgd_win_snapshot = snap.clone();
                    self.mcgd_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::RwiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.rwi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.rwi_win_snapshot = snap.clone();
                    self.rwi_win_loading = false;
                }
                let _ = snap;
            }
            _ => {}
        }
    }
}
