use super::*;

impl TyphooNApp {
    pub(super) fn handle_research_rank_snapshot_msg(&mut self, msg: BrokerMsg) {
        match msg {
            // ── Round 17 ──
            BrokerMsg::SizefSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.sizef_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.sizef_snapshot = snap.clone();
                    self.sizef_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_sizef(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::MomfSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.momf_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.momf_snapshot = snap.clone();
                    self.momf_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_momf(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::PeadrankSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.peadrank_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.peadrank_snapshot = snap.clone();
                    self.peadrank_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_peadrank(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::FqmSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.fqm_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.fqm_snapshot = snap.clone();
                    self.fqm_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_fqm(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::RevrankSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.revrank_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.revrank_snapshot = snap.clone();
                    self.revrank_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_revrank(&conn, &sym_u, &snap);
                    }
                }
            }
            // ── Round 18 ──
            BrokerMsg::LevrankSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.levrank_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.levrank_snapshot = snap.clone();
                    self.levrank_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_levrank(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::OperankSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.operank_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.operank_snapshot = snap.clone();
                    self.operank_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_operank(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::FqmrankSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.fqmrank_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.fqmrank_snapshot = snap.clone();
                    self.fqmrank_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_fqmrank(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::LiqrankSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.liqrank_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.liqrank_snapshot = snap.clone();
                    self.liqrank_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_liqrank(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::SurpstkSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.surpstk_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.surpstk_snapshot = snap.clone();
                    self.surpstk_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_surpstk(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::DvdrankSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.dvdrank_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.dvdrank_snapshot = snap.clone();
                    self.dvdrank_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_dvdrank(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::EarmrankSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.earmrank_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.earmrank_snapshot = snap.clone();
                    self.earmrank_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_earmrank(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::UpdgrankSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.updgrank_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.updgrank_snapshot = snap.clone();
                    self.updgrank_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_updgrank(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::GySnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.gy_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.gy_snapshot = snap.clone();
                    self.gy_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_gy(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::DesSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.des_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.des_snapshot = snap.clone();
                    self.des_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_des(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::DvdyieldrankSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.dvdyieldrank_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.dvdyieldrank_snapshot = snap.clone();
                    self.dvdyieldrank_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_dvdyieldrank(
                            &conn, &sym_u, &snap,
                        );
                    }
                }
            }
            BrokerMsg::ShrankSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.shrank_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.shrank_snapshot = snap.clone();
                    self.shrank_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_shrank(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::ShortrankDeltaSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.shortrank_delta_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.shortrank_delta_snapshot = snap.clone();
                    self.shortrank_delta_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_shortrank_delta(
                            &conn, &sym_u, &snap,
                        );
                    }
                }
            }
            BrokerMsg::InsiderconcSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.insiderconc_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.insiderconc_snapshot = snap.clone();
                    self.insiderconc_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_insiderconc(
                            &conn, &sym_u, &snap,
                        );
                    }
                }
            }
            BrokerMsg::AtrannSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.atrann_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.atrann_snapshot = snap.clone();
                    self.atrann_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_atrann(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::DdhistSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ddhist_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ddhist_snapshot = snap.clone();
                    self.ddhist_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_ddhist(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::PriceperfSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.priceperf_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.priceperf_snapshot = snap.clone();
                    self.priceperf_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_priceperf(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::MomrankMultiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.momrank_multi_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.momrank_multi_snapshot = snap.clone();
                    self.momrank_multi_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_momrank_multi(
                            &conn, &sym_u, &snap,
                        );
                    }
                }
            }
            BrokerMsg::BetarankSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.betarank_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.betarank_snapshot = snap.clone();
                    self.betarank_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_betarank(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::PegrankSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.pegrank_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.pegrank_snapshot = snap.clone();
                    self.pegrank_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_pegrank(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::FhighlowSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.fhighlow_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.fhighlow_snapshot = snap.clone();
                    self.fhighlow_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_fhighlow(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::RvconeSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.rvcone_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.rvcone_snapshot = snap.clone();
                    self.rvcone_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_rvcone(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::CalpbSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.calpb_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.calpb_snapshot = snap.clone();
                    self.calpb_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_calpb(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::CorrstkSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.corrstk_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.corrstk_snapshot = snap.clone();
                    self.corrstk_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_corrstk(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::TlrankSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.tlrank_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.tlrank_snapshot = snap.clone();
                    self.tlrank_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_tlrank(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::CorrrankSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.corrrank_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.corrrank_snapshot = snap.clone();
                    self.corrrank_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_corrrank(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::OperankDeltaSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.operank_delta_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.operank_delta_snapshot = snap.clone();
                    self.operank_delta_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_operank_delta(
                            &conn, &sym_u, &snap,
                        );
                    }
                }
            }
            BrokerMsg::DivaccSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.divacc_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.divacc_snapshot = snap.clone();
                    self.divacc_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_divacc(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::EpsaccSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.epsacc_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.epsacc_snapshot = snap.clone();
                    self.epsacc_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_epsacc(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::VrpSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.vrp_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.vrp_snapshot = snap.clone();
                    self.vrp_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_vrp(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::RetskewSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.retskew_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.retskew_snapshot = snap.clone();
                    self.retskew_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_retskew(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::RetkurtSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.retkurt_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.retkurt_snapshot = snap.clone();
                    self.retkurt_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_retkurt(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::TailrSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.tailr_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.tailr_snapshot = snap.clone();
                    self.tailr_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_tailr(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::RunlenSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.runlen_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.runlen_snapshot = snap.clone();
                    self.runlen_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_runlen(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::DayrangeSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.dayrange_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.dayrange_snapshot = snap.clone();
                    self.dayrange_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_dayrange(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::AutocorSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.autocor_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.autocor_snapshot = snap.clone();
                    self.autocor_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_autocor(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::HurstSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.hurst_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.hurst_snapshot = snap.clone();
                    self.hurst_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_hurst(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::HitrateSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.hitrate_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.hitrate_snapshot = snap.clone();
                    self.hitrate_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_hitrate(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::GlasymSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.glasym_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.glasym_snapshot = snap.clone();
                    self.glasym_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_glasym(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::VolratioSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.volratio_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.volratio_snapshot = snap.clone();
                    self.volratio_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_volratio(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::DrawupSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.drawup_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.drawup_snapshot = snap.clone();
                    self.drawup_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_drawup(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::GapstatsSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.gapstats_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.gapstats_snapshot = snap.clone();
                    self.gapstats_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_gapstats(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::VolclusterSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.volcluster_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.volcluster_snapshot = snap.clone();
                    self.volcluster_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_volcluster(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::CloseplcSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.closeplc_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.closeplc_snapshot = snap.clone();
                    self.closeplc_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_closeplc(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::MrhlSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.mrhl_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.mrhl_snapshot = snap.clone();
                    self.mrhl_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_mrhl(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::DownvolSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.downvol_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.downvol_snapshot = snap.clone();
                    self.downvol_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_downvol(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::SharprSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.sharpr_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.sharpr_snapshot = snap.clone();
                    self.sharpr_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_sharpr(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::EffratioSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.effratio_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.effratio_snapshot = snap.clone();
                    self.effratio_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_effratio(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::WickbiasSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.wickbias_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.wickbias_snapshot = snap.clone();
                    self.wickbias_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_wickbias(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::VolofvolSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.volofvol_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.volofvol_snapshot = snap.clone();
                    self.volofvol_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_volofvol(&conn, &sym_u, &snap);
                    }
                }
            }
            _ => {}
        }
    }
}
