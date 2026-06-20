use super::*;

impl TyphooNApp {
    pub(super) fn handle_research_quant_snapshot_msg(&mut self, msg: BrokerMsg) {
        match msg {
            // ── Research section ──
            BrokerMsg::CalmarSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.calmar_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.calmar_snapshot = snap.clone();
                    self.calmar_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_calmar(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::UlcerSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ulcer_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ulcer_snapshot = snap.clone();
                    self.ulcer_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_ulcer(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::VarratioSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.varratio_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.varratio_snapshot = snap.clone();
                    self.varratio_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_varratio(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::AmihudSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.amihud_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.amihud_snapshot = snap.clone();
                    self.amihud_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_amihud(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::JbnormSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.jbnorm_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.jbnorm_snapshot = snap.clone();
                    self.jbnorm_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_jbnorm(&conn, &sym_u, &snap);
                    }
                }
            }
            // ── Research section ──
            BrokerMsg::OmegaSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.omega_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.omega_snapshot = snap.clone();
                    self.omega_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_omega(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::DfaSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.dfa_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.dfa_snapshot = snap.clone();
                    self.dfa_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_dfa(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::BurkeSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.burke_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.burke_snapshot = snap.clone();
                    self.burke_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_burke(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::MonthseasSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.monthseas_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.monthseas_snapshot = snap.clone();
                    self.monthseas_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_monthseas(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::RollsprdSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.rollsprd_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.rollsprd_snapshot = snap.clone();
                    self.rollsprd_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_rollsprd(&conn, &sym_u, &snap);
                    }
                }
            }
            // ── Research section ──
            BrokerMsg::ParkinsonSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.parkinson_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.parkinson_snapshot = snap.clone();
                    self.parkinson_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_parkinson(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::GkvolSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.gkvol_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.gkvol_snapshot = snap.clone();
                    self.gkvol_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_gkvol(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::RsvolSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.rsvol_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.rsvol_snapshot = snap.clone();
                    self.rsvol_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_rsvol(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::CvarSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cvar_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cvar_snapshot = snap.clone();
                    self.cvar_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_cvar(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::DoweffectSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.doweffect_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.doweffect_snapshot = snap.clone();
                    self.doweffect_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_doweffect(&conn, &sym_u, &snap);
                    }
                }
            }
            // ── Research section ──
            BrokerMsg::SterlingSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.sterling_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.sterling_snapshot = snap.clone();
                    self.sterling_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_sterling(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::KellyfSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.kellyf_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.kellyf_snapshot = snap.clone();
                    self.kellyf_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_kellyf(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::LjungbSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ljungb_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ljungb_snapshot = snap.clone();
                    self.ljungb_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_ljungb(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::RunstestSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.runstest_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.runstest_snapshot = snap.clone();
                    self.runstest_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_runstest(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::ZeroretSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.zeroret_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.zeroret_snapshot = snap.clone();
                    self.zeroret_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_zeroret(&conn, &sym_u, &snap);
                    }
                }
            }
            // ── Research section ──
            BrokerMsg::PsrSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.psr_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.psr_snapshot = snap.clone();
                    self.psr_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_psr(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::AdfSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.adf_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.adf_snapshot = snap.clone();
                    self.adf_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_adf(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::MnkendallSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.mnkendall_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.mnkendall_snapshot = snap.clone();
                    self.mnkendall_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_mnkendall(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::BipowerSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.bipower_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.bipower_snapshot = snap.clone();
                    self.bipower_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_bipower(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::DddurSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.dddur_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.dddur_snapshot = snap.clone();
                    self.dddur_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_dddur(&conn, &sym_u, &snap);
                    }
                }
            }
            // ── Research section ──
            BrokerMsg::HilltailSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.hilltail_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.hilltail_snapshot = snap.clone();
                    self.hilltail_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_hilltail(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::ArchlmSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.archlm_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.archlm_snapshot = snap.clone();
                    self.archlm_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_archlm(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::PainratioSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.painratio_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.painratio_snapshot = snap.clone();
                    self.painratio_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_painratio(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::CusumSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cusum_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cusum_snapshot = snap.clone();
                    self.cusum_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_cusum(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::CfvarSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cfvar_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cfvar_snapshot = snap.clone();
                    self.cfvar_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_cfvar(&conn, &sym_u, &snap);
                    }
                }
            }
            // ── Research section ──
            BrokerMsg::EntropySnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.entropy_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.entropy_snapshot = snap.clone();
                    self.entropy_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_entropy(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::RachevSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.rachev_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.rachev_snapshot = snap.clone();
                    self.rachev_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_rachev(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::GprSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.gpr_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.gpr_snapshot = snap.clone();
                    self.gpr_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_gpr(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::PacfSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.pacf_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.pacf_snapshot = snap.clone();
                    self.pacf_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_pacf(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::ApenSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.apen_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.apen_snapshot = snap.clone();
                    self.apen_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_apen(&conn, &sym_u, &snap);
                    }
                }
            }
            // ── Research section ──
            BrokerMsg::UprSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.upr_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.upr_snapshot = snap.clone();
                    self.upr_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_upr(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::LevereffSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.levereff_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.levereff_snapshot = snap.clone();
                    self.levereff_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_levereff(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::DrawdarSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.drawdar_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.drawdar_snapshot = snap.clone();
                    self.drawdar_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_drawdar(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::VarhalfSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.varhalf_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.varhalf_snapshot = snap.clone();
                    self.varhalf_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_varhalf(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::GiniSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.gini_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.gini_snapshot = snap.clone();
                    self.gini_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_gini(&conn, &sym_u, &snap);
                    }
                }
            }
            // ── Research section ──
            BrokerMsg::SampenSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.sampen_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.sampen_snapshot = snap.clone();
                    self.sampen_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_sampen(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::PermenSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.permen_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.permen_snapshot = snap.clone();
                    self.permen_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_permen(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::RecfactSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.recfact_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.recfact_snapshot = snap.clone();
                    self.recfact_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_recfact(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::KpssSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.kpss_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.kpss_snapshot = snap.clone();
                    self.kpss_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_kpss(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::SpecentSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.specent_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.specent_snapshot = snap.clone();
                    self.specent_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_specent(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::RobvolSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.robvol_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.robvol_snapshot = snap.clone();
                    self.robvol_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_robvol(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::RenyientSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.renyient_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.renyient_snapshot = snap.clone();
                    self.renyient_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_renyient(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::RetquantSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.retquant_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.retquant_snapshot = snap.clone();
                    self.retquant_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_retquant(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::MsentSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.msent_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.msent_snapshot = snap.clone();
                    self.msent_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_msent(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::EwmavolSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ewmavol_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ewmavol_snapshot = snap.clone();
                    self.ewmavol_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_ewmavol(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::KsnormSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ksnorm_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ksnorm_snapshot = snap.clone();
                    self.ksnorm_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_ksnorm(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::AdtestSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.adtest_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.adtest_snapshot = snap.clone();
                    self.adtest_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_adtest(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::LmomSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.lmom_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.lmom_snapshot = snap.clone();
                    self.lmom_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_lmom(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::KylelamSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.kylelam_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.kylelam_snapshot = snap.clone();
                    self.kylelam_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_kylelam(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::PeakoverSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.peakover_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.peakover_snapshot = snap.clone();
                    self.peakover_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_peakover(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::HiguchiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.higuchi_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.higuchi_snapshot = snap.clone();
                    self.higuchi_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_higuchi(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::PickandsSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.pickands_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.pickands_snapshot = snap.clone();
                    self.pickands_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_pickands(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::Kappa3SnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.kappa3_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.kappa3_snapshot = snap.clone();
                    self.kappa3_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_kappa3(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::LyapunovSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.lyapunov_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.lyapunov_snapshot = snap.clone();
                    self.lyapunov_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_lyapunov(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::RankacSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.rankac_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.rankac_snapshot = snap.clone();
                    self.rankac_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_rankac(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::BnsjumpSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.bnsjump_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.bnsjump_snapshot = snap.clone();
                    self.bnsjump_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_bnsjump(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::PprootSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.pproot_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.pproot_snapshot = snap.clone();
                    self.pproot_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_pproot(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::MfdfaSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.mfdfa_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.mfdfa_snapshot = snap.clone();
                    self.mfdfa_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_mfdfa(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::HillksSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.hillks_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.hillks_snapshot = snap.clone();
                    self.hillks_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_hillks(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::TsiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.tsi_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.tsi_snapshot = snap.clone();
                    self.tsi_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_tsi(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::Garch11SnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.garch11_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.garch11_snapshot = snap.clone();
                    self.garch11_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_garch11(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::SadfSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.sadf_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.sadf_snapshot = snap.clone();
                    self.sadf_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_sadf(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::CordimSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cordim_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cordim_snapshot = snap.clone();
                    self.cordim_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_cordim(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::SkspecSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.skspec_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.skspec_snapshot = snap.clone();
                    self.skspec_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_skspec(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::AutomiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.automi_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.automi_snapshot = snap.clone();
                    self.automi_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_automi(&conn, &sym_u, &snap);
                    }
                }
            }
            _ => {}
        }
    }
}
