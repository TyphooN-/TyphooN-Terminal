use super::*;

impl TyphooNApp {
    pub(super) fn handle_extended_indicator_snapshot_msg(&mut self, msg: BrokerMsg) {
        match msg {
            // ── Round 51 result handlers ──
            BrokerMsg::DemaSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.dema_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.dema_win_snapshot = snap.clone();
                    self.dema_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::TemaSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.tema_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.tema_win_snapshot = snap.clone();
                    self.tema_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::LinregSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.linreg_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.linreg_win_snapshot = snap.clone();
                    self.linreg_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::PivotsSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.pivots_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.pivots_win_snapshot = snap.clone();
                    self.pivots_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::HeikinSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.heikin_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.heikin_win_snapshot = snap.clone();
                    self.heikin_win_loading = false;
                }
                let _ = snap;
            }
            // ── Round 52 result handlers ──
            BrokerMsg::AlmaSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.alma_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.alma_win_snapshot = snap.clone();
                    self.alma_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::ZlemaSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.zlema_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.zlema_win_snapshot = snap.clone();
                    self.zlema_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::ElderRaySnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.elderray_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.elderray_win_snapshot = snap.clone();
                    self.elderray_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::TsfSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.tsf_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.tsf_win_snapshot = snap.clone();
                    self.tsf_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::RviSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.rvi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.rvi_win_snapshot = snap.clone();
                    self.rvi_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::TrimaSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.trima_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.trima_win_snapshot = snap.clone();
                    self.trima_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::T3SnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.t3_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.t3_win_snapshot = snap.clone();
                    self.t3_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::VidyaSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.vidya_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.vidya_win_snapshot = snap.clone();
                    self.vidya_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::SmiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.smi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.smi_win_snapshot = snap.clone();
                    self.smi_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::PvtSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.pvt_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.pvt_win_snapshot = snap.clone();
                    self.pvt_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::AcSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ac_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ac_win_snapshot = snap.clone();
                    self.ac_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::ChvolSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.chvol_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.chvol_win_snapshot = snap.clone();
                    self.chvol_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::BbwidthSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.bbwidth_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.bbwidth_win_snapshot = snap.clone();
                    self.bbwidth_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::ElderImpSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.elderimp_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.elderimp_win_snapshot = snap.clone();
                    self.elderimp_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::RmiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.rmi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.rmi_win_snapshot = snap.clone();
                    self.rmi_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::SymbolExpirationsMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.expcal_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.expcal_win_snapshot = snap.clone();
                    self.expcal_win_loading = false;
                }
                let _ = snap;
            }
            // ── Round 55 receive arms ──
            BrokerMsg::SmmaSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.smma_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.smma_win_snapshot = snap.clone();
                    self.smma_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::AlligatorSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.alligator_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.alligator_win_snapshot = snap.clone();
                    self.alligator_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CrsiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.crsi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.crsi_win_snapshot = snap.clone();
                    self.crsi_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::SebSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.seb_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.seb_win_snapshot = snap.clone();
                    self.seb_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::ImiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.imi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.imi_win_snapshot = snap.clone();
                    self.imi_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::GmmaSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.gmma_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.gmma_win_snapshot = snap.clone();
                    self.gmma_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::MaenvSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.maenv_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.maenv_win_snapshot = snap.clone();
                    self.maenv_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::AdlSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.adl_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.adl_win_snapshot = snap.clone();
                    self.adl_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::VhfSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.vhf_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.vhf_win_snapshot = snap.clone();
                    self.vhf_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::VrocSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.vroc_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.vroc_win_snapshot = snap.clone();
                    self.vroc_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::KdjSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.kdj_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.kdj_win_snapshot = snap.clone();
                    self.kdj_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::QqeSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.qqe_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.qqe_win_snapshot = snap.clone();
                    self.qqe_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::PmoSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.pmo_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.pmo_win_snapshot = snap.clone();
                    self.pmo_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CfoSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cfo_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cfo_win_snapshot = snap.clone();
                    self.cfo_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::TmfSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.tmf_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.tmf_win_snapshot = snap.clone();
                    self.tmf_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::FractalsSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.fractals_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.fractals_win_snapshot = snap.clone();
                    self.fractals_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::IftRsiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ift_rsi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ift_rsi_win_snapshot = snap.clone();
                    self.ift_rsi_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::MamaSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.mama_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.mama_win_snapshot = snap.clone();
                    self.mama_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CogSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cog_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cog_win_snapshot = snap.clone();
                    self.cog_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::DidiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.didi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.didi_win_snapshot = snap.clone();
                    self.didi_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::DemarkerSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.demarker_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.demarker_win_snapshot = snap.clone();
                    self.demarker_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::GatorSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.gator_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.gator_win_snapshot = snap.clone();
                    self.gator_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::BwMfiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.bw_mfi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.bw_mfi_win_snapshot = snap.clone();
                    self.bw_mfi_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::VwmaSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.vwma_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.vwma_win_snapshot = snap.clone();
                    self.vwma_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::StddevSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.stddev_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.stddev_win_snapshot = snap.clone();
                    self.stddev_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::WmaSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.wma_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.wma_win_snapshot = snap.clone();
                    self.wma_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::RainbowSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.rainbow_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.rainbow_win_snapshot = snap.clone();
                    self.rainbow_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::MesaSineSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.mesa_sine_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.mesa_sine_win_snapshot = snap.clone();
                    self.mesa_sine_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::FramaSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.frama_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.frama_win_snapshot = snap.clone();
                    self.frama_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::IbsSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ibs_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ibs_win_snapshot = snap.clone();
                    self.ibs_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::LaguerreRsiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.laguerre_rsi_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.laguerre_rsi_win_snapshot = snap.clone();
                    self.laguerre_rsi_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::ZigzagSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.zigzag_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.zigzag_win_snapshot = snap.clone();
                    self.zigzag_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::PgoSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.pgo_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.pgo_win_snapshot = snap.clone();
                    self.pgo_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::HtTrendlineSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ht_trendline_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ht_trendline_win_snapshot = snap.clone();
                    self.ht_trendline_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::MidpointSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.midpoint_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.midpoint_win_snapshot = snap.clone();
                    self.midpoint_win_loading = false;
                }
                let _ = snap;
            }
            // ── Round 62 match arms ──
            BrokerMsg::MassIndexSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.mass_index_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.mass_index_win_snapshot = snap.clone();
                    self.mass_index_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::NatrSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.natr_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.natr_win_snapshot = snap.clone();
                    self.natr_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::TtmSqueezeSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ttm_squeeze_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ttm_squeeze_win_snapshot = snap.clone();
                    self.ttm_squeeze_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::ForceIndexSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.force_index_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.force_index_win_snapshot = snap.clone();
                    self.force_index_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::TrangeSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.trange_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.trange_win_snapshot = snap.clone();
                    self.trange_win_loading = false;
                }
                let _ = snap;
            }
            // ── Round 63 match arms ──
            BrokerMsg::LinearregSlopeSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.linearreg_slope_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.linearreg_slope_win_snapshot = snap.clone();
                    self.linearreg_slope_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::HtDcperiodSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ht_dcperiod_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ht_dcperiod_win_snapshot = snap.clone();
                    self.ht_dcperiod_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::HtTrendmodeSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ht_trendmode_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ht_trendmode_win_snapshot = snap.clone();
                    self.ht_trendmode_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::AccbandsSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.accbands_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.accbands_win_snapshot = snap.clone();
                    self.accbands_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::StochfSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.stochf_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.stochf_win_snapshot = snap.clone();
                    self.stochf_win_loading = false;
                }
                let _ = snap;
            }
            // ── Round 64 match arms ──
            BrokerMsg::LinearregSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.linearreg_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.linearreg_win_snapshot = snap.clone();
                    self.linearreg_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::LinearregAngleSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.linearreg_angle_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.linearreg_angle_win_snapshot = snap.clone();
                    self.linearreg_angle_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::HtDcphaseSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ht_dcphase_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ht_dcphase_win_snapshot = snap.clone();
                    self.ht_dcphase_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::HtSineSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ht_sine_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ht_sine_win_snapshot = snap.clone();
                    self.ht_sine_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::HtPhasorSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ht_phasor_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ht_phasor_win_snapshot = snap.clone();
                    self.ht_phasor_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::MidpriceSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.midprice_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.midprice_win_snapshot = snap.clone();
                    self.midprice_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::ApoSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.apo_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.apo_win_snapshot = snap.clone();
                    self.apo_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::MomSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.mom_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.mom_win_snapshot = snap.clone();
                    self.mom_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::SarextSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.sarext_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.sarext_win_snapshot = snap.clone();
                    self.sarext_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::AdxrSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.adxr_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.adxr_win_snapshot = snap.clone();
                    self.adxr_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::AvgpriceSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.avgprice_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.avgprice_win_snapshot = snap.clone();
                    self.avgprice_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::MedpriceSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.medprice_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.medprice_win_snapshot = snap.clone();
                    self.medprice_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::TypPriceSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.typprice_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.typprice_win_snapshot = snap.clone();
                    self.typprice_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::WclPriceSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.wclprice_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.wclprice_win_snapshot = snap.clone();
                    self.wclprice_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::VarianceSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.variance_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.variance_win_snapshot = snap.clone();
                    self.variance_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::PlusDiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.plus_di_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.plus_di_win_snapshot = snap.clone();
                    self.plus_di_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::MinusDiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.minus_di_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.minus_di_win_snapshot = snap.clone();
                    self.minus_di_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::PlusDmSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.plus_dm_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.plus_dm_win_snapshot = snap.clone();
                    self.plus_dm_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::MinusDmSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.minus_dm_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.minus_dm_win_snapshot = snap.clone();
                    self.minus_dm_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::DxSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.dx_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.dx_win_snapshot = snap.clone();
                    self.dx_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::RocSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.roc_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.roc_win_snapshot = snap.clone();
                    self.roc_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::RocpSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.rocp_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.rocp_win_snapshot = snap.clone();
                    self.rocp_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::RocrSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.rocr_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.rocr_win_snapshot = snap.clone();
                    self.rocr_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::Rocr100SnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.rocr100_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.rocr100_win_snapshot = snap.clone();
                    self.rocr100_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CorrelSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.correl_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.correl_win_snapshot = snap.clone();
                    self.correl_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::MinSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.min_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.min_win_snapshot = snap.clone();
                    self.min_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::MaxSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.max_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.max_win_snapshot = snap.clone();
                    self.max_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::MinMaxSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.minmax_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.minmax_win_snapshot = snap.clone();
                    self.minmax_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::MinIndexSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.minindex_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.minindex_win_snapshot = snap.clone();
                    self.minindex_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::MaxIndexSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.maxindex_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.maxindex_win_snapshot = snap.clone();
                    self.maxindex_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::BbandsSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.bbands_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.bbands_win_snapshot = snap.clone();
                    self.bbands_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::AdSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ad_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ad_win_snapshot = snap.clone();
                    self.ad_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::AdoscSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.adosc_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.adosc_win_snapshot = snap.clone();
                    self.adosc_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::SumSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.sum_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.sum_win_snapshot = snap.clone();
                    self.sum_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::LinearRegInterceptSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .linreg_intercept_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.linreg_intercept_win_snapshot = snap.clone();
                    self.linreg_intercept_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::AroonoscSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.aroonosc_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.aroonosc_win_snapshot = snap.clone();
                    self.aroonosc_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::MinMaxIndexSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.minmaxindex_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.minmaxindex_win_snapshot = snap.clone();
                    self.minmaxindex_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::MacdextSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.macdext_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.macdext_win_snapshot = snap.clone();
                    self.macdext_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::MacdfixSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.macdfix_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.macdfix_win_snapshot = snap.clone();
                    self.macdfix_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::MavpSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.mavp_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.mavp_win_snapshot = snap.clone();
                    self.mavp_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlDojiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_doji_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_doji_win_snapshot = snap.clone();
                    self.cdl_doji_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlHammerSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_hammer_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_hammer_win_snapshot = snap.clone();
                    self.cdl_hammer_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlShootingStarSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_shooting_star_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_shooting_star_win_snapshot = snap.clone();
                    self.cdl_shooting_star_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlEngulfingSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_engulfing_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_engulfing_win_snapshot = snap.clone();
                    self.cdl_engulfing_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlHaramiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_harami_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_harami_win_snapshot = snap.clone();
                    self.cdl_harami_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlMorningStarSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_morning_star_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_morning_star_win_snapshot = snap.clone();
                    self.cdl_morning_star_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlEveningStarSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_evening_star_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_evening_star_win_snapshot = snap.clone();
                    self.cdl_evening_star_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlThreeBlackCrowsSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_three_black_crows_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_three_black_crows_win_snapshot = snap.clone();
                    self.cdl_three_black_crows_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlThreeWhiteSoldiersSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_three_white_soldiers_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_three_white_soldiers_win_snapshot = snap.clone();
                    self.cdl_three_white_soldiers_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlDarkCloudCoverSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_dark_cloud_cover_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_dark_cloud_cover_win_snapshot = snap.clone();
                    self.cdl_dark_cloud_cover_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlPiercingSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_piercing_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_piercing_win_snapshot = snap.clone();
                    self.cdl_piercing_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlDragonflyDojiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_dragonfly_doji_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_dragonfly_doji_win_snapshot = snap.clone();
                    self.cdl_dragonfly_doji_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlGravestoneDojiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_gravestone_doji_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_gravestone_doji_win_snapshot = snap.clone();
                    self.cdl_gravestone_doji_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlHangingManSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_hanging_man_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_hanging_man_win_snapshot = snap.clone();
                    self.cdl_hanging_man_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlInvertedHammerSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_inverted_hammer_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_inverted_hammer_win_snapshot = snap.clone();
                    self.cdl_inverted_hammer_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlHaramiCrossSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_harami_cross_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_harami_cross_win_snapshot = snap.clone();
                    self.cdl_harami_cross_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlLongLeggedDojiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_long_legged_doji_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_long_legged_doji_win_snapshot = snap.clone();
                    self.cdl_long_legged_doji_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlMarubozuSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_marubozu_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_marubozu_win_snapshot = snap.clone();
                    self.cdl_marubozu_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlSpinningTopSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_spinning_top_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_spinning_top_win_snapshot = snap.clone();
                    self.cdl_spinning_top_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlTristarSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_tristar_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_tristar_win_snapshot = snap.clone();
                    self.cdl_tristar_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlDojiStarSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_doji_star_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_doji_star_win_snapshot = snap.clone();
                    self.cdl_doji_star_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlMorningDojiStarSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_morning_doji_star_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_morning_doji_star_win_snapshot = snap.clone();
                    self.cdl_morning_doji_star_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlEveningDojiStarSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_evening_doji_star_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_evening_doji_star_win_snapshot = snap.clone();
                    self.cdl_evening_doji_star_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlAbandonedBabySnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_abandoned_baby_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_abandoned_baby_win_snapshot = snap.clone();
                    self.cdl_abandoned_baby_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlThreeInsideSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_three_inside_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_three_inside_win_snapshot = snap.clone();
                    self.cdl_three_inside_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlBeltHoldSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_belt_hold_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_belt_hold_win_snapshot = snap.clone();
                    self.cdl_belt_hold_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlClosingMarubozuSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_closing_marubozu_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_closing_marubozu_win_snapshot = snap.clone();
                    self.cdl_closing_marubozu_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlHighWaveSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_high_wave_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_high_wave_win_snapshot = snap.clone();
                    self.cdl_high_wave_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlLongLineSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_long_line_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_long_line_win_snapshot = snap.clone();
                    self.cdl_long_line_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlShortLineSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_short_line_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_short_line_win_snapshot = snap.clone();
                    self.cdl_short_line_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlCounterattackSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_counterattack_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_counterattack_win_snapshot = snap.clone();
                    self.cdl_counterattack_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlHomingPigeonSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_homing_pigeon_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_homing_pigeon_win_snapshot = snap.clone();
                    self.cdl_homing_pigeon_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlInNeckSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_in_neck_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_in_neck_win_snapshot = snap.clone();
                    self.cdl_in_neck_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlOnNeckSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_on_neck_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_on_neck_win_snapshot = snap.clone();
                    self.cdl_on_neck_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlThrustingSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_thrusting_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_thrusting_win_snapshot = snap.clone();
                    self.cdl_thrusting_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlTwoCrowsSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_two_crows_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_two_crows_win_snapshot = snap.clone();
                    self.cdl_two_crows_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlThreeLineStrikeSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_three_line_strike_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_three_line_strike_win_snapshot = snap.clone();
                    self.cdl_three_line_strike_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlThreeOutsideSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_three_outside_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_three_outside_win_snapshot = snap.clone();
                    self.cdl_three_outside_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlMatchingLowSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_matching_low_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_matching_low_win_snapshot = snap.clone();
                    self.cdl_matching_low_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlSeparatingLinesSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_separating_lines_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_separating_lines_win_snapshot = snap.clone();
                    self.cdl_separating_lines_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlStickSandwichSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_stick_sandwich_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_stick_sandwich_win_snapshot = snap.clone();
                    self.cdl_stick_sandwich_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlRickshawManSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_rickshaw_man_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_rickshaw_man_win_snapshot = snap.clone();
                    self.cdl_rickshaw_man_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlTakuriSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_takuri_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_takuri_win_snapshot = snap.clone();
                    self.cdl_takuri_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlThreeStarsInSouthSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_three_stars_in_south_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_three_stars_in_south_win_snapshot = snap.clone();
                    self.cdl_three_stars_in_south_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlIdenticalThreeCrowsSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_identical_three_crows_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_identical_three_crows_win_snapshot = snap.clone();
                    self.cdl_identical_three_crows_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlKickingSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_kicking_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_kicking_win_snapshot = snap.clone();
                    self.cdl_kicking_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlKickingByLengthSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_kicking_by_length_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_kicking_by_length_win_snapshot = snap.clone();
                    self.cdl_kicking_by_length_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlLadderBottomSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_ladder_bottom_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_ladder_bottom_win_snapshot = snap.clone();
                    self.cdl_ladder_bottom_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlUniqueThreeRiverSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_unique_three_river_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_unique_three_river_win_snapshot = snap.clone();
                    self.cdl_unique_three_river_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlAdvanceBlockSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_advance_block_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_advance_block_win_snapshot = snap.clone();
                    self.cdl_advance_block_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlBreakawaySnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_breakaway_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_breakaway_win_snapshot = snap.clone();
                    self.cdl_breakaway_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlGapSideSideWhiteSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_gap_side_side_white_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_gap_side_side_white_win_snapshot = snap.clone();
                    self.cdl_gap_side_side_white_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlUpsideGapTwoCrowsSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_upside_gap_two_crows_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_upside_gap_two_crows_win_snapshot = snap.clone();
                    self.cdl_upside_gap_two_crows_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlXSideGapThreeMethodsSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_xside_gap_three_methods_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_xside_gap_three_methods_win_snapshot = snap.clone();
                    self.cdl_xside_gap_three_methods_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlConcealBabySwallowSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_conceal_baby_swallow_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_conceal_baby_swallow_win_snapshot = snap.clone();
                    self.cdl_conceal_baby_swallow_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlHikkakeSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_hikkake_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_hikkake_win_snapshot = snap.clone();
                    self.cdl_hikkake_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlHikkakeModSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_hikkake_mod_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_hikkake_mod_win_snapshot = snap.clone();
                    self.cdl_hikkake_mod_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlMatHoldSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_mat_hold_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_mat_hold_win_snapshot = snap.clone();
                    self.cdl_mat_hold_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlRiseFallThreeMethodsSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_rise_fall_three_methods_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_rise_fall_three_methods_win_snapshot = snap.clone();
                    self.cdl_rise_fall_three_methods_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlStalledPatternSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self
                    .cdl_stalled_pattern_win_symbol
                    .eq_ignore_ascii_case(&sym_u)
                {
                    self.cdl_stalled_pattern_win_snapshot = snap.clone();
                    self.cdl_stalled_pattern_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::CdlTasukiGapSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.cdl_tasuki_gap_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.cdl_tasuki_gap_win_snapshot = snap.clone();
                    self.cdl_tasuki_gap_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::ModSharpeSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.modsharpe_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.modsharpe_win_snapshot = snap.clone();
                    self.modsharpe_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::HsiehTestSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.hsiehtest_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.hsiehtest_win_snapshot = snap.clone();
                    self.hsiehtest_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::ChowBreakSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.chowbreak_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.chowbreak_win_snapshot = snap.clone();
                    self.chowbreak_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::DriftBurstSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.driftburst_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.driftburst_win_snapshot = snap.clone();
                    self.driftburst_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::HlvClustSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.hlvclust_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.hlvclust_win_snapshot = snap.clone();
                    self.hlvclust_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::YangZhangSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.yangzhang_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.yangzhang_win_snapshot = snap.clone();
                    self.yangzhang_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::KuiperSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.kuiper_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.kuiper_win_snapshot = snap.clone();
                    self.kuiper_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::DagostinoSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.dagostino_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.dagostino_win_snapshot = snap.clone();
                    self.dagostino_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::BaiPerronSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.baiperron_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.baiperron_win_snapshot = snap.clone();
                    self.baiperron_win_loading = false;
                }
                let _ = snap;
            }
            BrokerMsg::KupiecPofSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.kupiecpof_win_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.kupiecpof_win_snapshot = snap.clone();
                    self.kupiecpof_win_loading = false;
                }
                let _ = snap;
            }
            _ => {}
        }
    }
}
