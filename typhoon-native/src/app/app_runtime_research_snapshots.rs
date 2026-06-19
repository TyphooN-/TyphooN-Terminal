use super::*;

impl TyphooNApp {
    pub(super) fn handle_research_snapshot_msg(&mut self, msg: BrokerMsg) {
        match msg {
            // Leverage, accruals, realized-volatility, cash-flow, and short-interest research
            BrokerMsg::LeverageSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.lev_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.lev_snapshot = snap.clone();
                    self.lev_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_leverage(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::AccrualsSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.acrl_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.acrl_snapshot = snap.clone();
                    self.acrl_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_accruals(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::RealizedVolSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.rvol_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.rvol_snapshot = snap.clone();
                    self.rvol_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_realized_vol(
                            &conn, &sym_u, &snap,
                        );
                    }
                }
            }
            BrokerMsg::FcfYieldSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.fcfy_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.fcfy_snapshot = snap.clone();
                    self.fcfy_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_fcf_yield(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::ShortInterestSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.shrt_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.shrt_snapshot = snap.clone();
                    self.shrt_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_short_interest(
                            &conn, &sym_u, &snap,
                        );
                    }
                }
            }
            // Solvency, quality, volatility-estimator, EPS-beat, and price-target research
            BrokerMsg::AltmanZSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.altz_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.altz_snapshot = snap.clone();
                    self.altz_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_altman_z(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::PiotroskiSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ptfs_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ptfs_snapshot = snap.clone();
                    self.ptfs_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_piotroski(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::OhlcVolSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.vole_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.vole_snapshot = snap.clone();
                    self.vole_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_ohlc_vol(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::EpsBeatSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.epsb_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.epsb_snapshot = snap.clone();
                    self.epsb_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_eps_beat(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::PriceTargetDispersionSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ptd_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ptd_snapshot = snap.clone();
                    self.ptd_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_price_target_dispersion(
                            &conn, &sym_u, &snap,
                        );
                    }
                }
            }
            // Insider, dividend-growth, earnings-revision, sector-rotation, and upgrade/downgrade research
            BrokerMsg::InsiderActivitySnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.mngr_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.mngr_snapshot = snap.clone();
                    self.mngr_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_insider_activity(
                            &conn, &sym_u, &snap,
                        );
                    }
                }
            }
            BrokerMsg::DivgSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.divg_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.divg_snapshot = snap.clone();
                    self.divg_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_divg(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::EarmSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.earm_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.earm_snapshot = snap.clone();
                    self.earm_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_earm(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::SectorRotationSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.sectr_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.sectr_snapshot = snap.clone();
                    self.sectr_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_sector_rotation(
                            &conn, &sym_u, &snap,
                        );
                    }
                }
            }
            BrokerMsg::UpdmSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.updm_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.updm_snapshot = snap.clone();
                    self.updm_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_updm(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::MomentumSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.mom_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.mom_snapshot = snap.clone();
                    self.mom_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_momentum(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::LiquiditySnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.liq_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.liq_snapshot = snap.clone();
                    self.liq_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_liquidity(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::BreakoutSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.break_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.break_snapshot = snap.clone();
                    self.break_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_breakout(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::CashCycleSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.ccrl_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.ccrl_snapshot = snap.clone();
                    self.ccrl_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_cash_cycle(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::CreditSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.credit_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.credit_snapshot = snap.clone();
                    self.credit_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_credit(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::GrowmSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.growm_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.growm_snapshot = snap.clone();
                    self.growm_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_growm(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::FlowSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.flow_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.flow_snapshot = snap.clone();
                    self.flow_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_flow(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::RegimeSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.regime_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.regime_snapshot = snap.clone();
                    self.regime_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_regime(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::RelvolSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.relvol_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.relvol_snapshot = snap.clone();
                    self.relvol_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_relvol(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::MarginsSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.margins_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.margins_snapshot = snap.clone();
                    self.margins_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_margins(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::ValSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.val_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.val_snapshot = snap.clone();
                    self.val_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_val(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::QualSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.qual_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.qual_snapshot = snap.clone();
                    self.qual_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_qual(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::RiskSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.risk_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.risk_snapshot = snap.clone();
                    self.risk_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_risk(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::InsstrkSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.insstrk_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.insstrk_snapshot = snap.clone();
                    self.insstrk_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_insstrk(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::CovgSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.covg_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.covg_snapshot = snap.clone();
                    self.covg_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_covg(&conn, &sym_u, &snap);
                    }
                }
            }
            // ── Round 16 ─────────────────────────────────────────
            BrokerMsg::VrkSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.vrk_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.vrk_snapshot = snap.clone();
                    self.vrk_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_vrk(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::QrkSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.qrk_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.qrk_snapshot = snap.clone();
                    self.qrk_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_qrk(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::RrkSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.rrk_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.rrk_snapshot = snap.clone();
                    self.rrk_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_rrk(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::RelepsgrSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.relepsgr_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.relepsgr_snapshot = snap.clone();
                    self.relepsgr_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ =
                            typhoon_engine::core::research::upsert_relepsgr(&conn, &sym_u, &snap);
                    }
                }
            }
            BrokerMsg::PeadSnapshotMsg(sym, snap) => {
                let sym_u = sym.to_uppercase();
                if self.pead_symbol.eq_ignore_ascii_case(&sym_u) {
                    self.pead_snapshot = snap.clone();
                    self.pead_loading = false;
                }
                if let Some(ref cache) = self.cache {
                    if let Ok(conn) = cache.connection() {
                        let _ = typhoon_engine::core::research::upsert_pead(&conn, &sym_u, &snap);
                    }
                }
            }
            _ => {}
        }
    }
}
