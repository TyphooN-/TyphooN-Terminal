use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_leverage_quality_liquidity_rank_command(
        &mut self,
        cmd_upper: &String,
    ) -> bool {
        match cmd_upper.as_str() {
            // ── Leverage, quality, and liquidity rank palette aliases ──
            "LEVRANK" | "LEV_RANK" | "LEVERAGE_RANK" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.levrank_symbol = sym;
                }
                self.show_levrank = true;
                if self.levrank_snapshot.symbol.is_empty() && !self.levrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_levrank(
                                &conn,
                                &self.levrank_symbol,
                            ) {
                                self.levrank_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "OPERANK" | "OPER_RANK" | "OP_QUALITY_RANK" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.operank_symbol = sym;
                }
                self.show_operank = true;
                if self.operank_snapshot.symbol.is_empty() && !self.operank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_operank(
                                &conn,
                                &self.operank_symbol,
                            ) {
                                self.operank_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "FQMRANK" | "FQM_RANK" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.fqmrank_symbol = sym;
                }
                self.show_fqmrank = true;
                if self.fqmrank_snapshot.symbol.is_empty() && !self.fqmrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_fqmrank(
                                &conn,
                                &self.fqmrank_symbol,
                            ) {
                                self.fqmrank_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "LIQRANK" | "LIQ_RANK" | "LIQUIDITY_RANK" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.liqrank_symbol = sym;
                }
                self.show_liqrank = true;
                if self.liqrank_snapshot.symbol.is_empty() && !self.liqrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_liqrank(
                                &conn,
                                &self.liqrank_symbol,
                            ) {
                                self.liqrank_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "TLRANK" | "TL_RANK" | "LIQ30_RANK" | "ADV30_RANK" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.tlrank_symbol = sym;
                }
                self.show_tlrank = true;
                if self.tlrank_snapshot.symbol.is_empty() && !self.tlrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_tlrank(
                                &conn,
                                &self.tlrank_symbol,
                            ) {
                                self.tlrank_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "SURPSTK" | "EPS_STREAK" | "SURPRISE_STREAK" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.surpstk_symbol = sym;
                }
                self.show_surpstk = true;
                if self.surpstk_snapshot.symbol.is_empty() && !self.surpstk_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_surpstk(
                                &conn,
                                &self.surpstk_symbol,
                            ) {
                                self.surpstk_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "DVDRANK" | "DIVG_RANK" | "DIVIDEND_RANK" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.dvdrank_symbol = sym;
                }
                self.show_dvdrank = true;
                if self.dvdrank_snapshot.symbol.is_empty() && !self.dvdrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_dvdrank(
                                &conn,
                                &self.dvdrank_symbol,
                            ) {
                                self.dvdrank_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "EARMRANK" | "EARM_RANK" | "EARNINGS_MOMENTUM_RANK" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.earmrank_symbol = sym;
                }
                self.show_earmrank = true;
                if self.earmrank_snapshot.symbol.is_empty() && !self.earmrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_earmrank(
                                &conn,
                                &self.earmrank_symbol,
                            ) {
                                self.earmrank_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "UPDGRANK" | "UPDG_RANK" | "UPGRADE_RANK" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.updgrank_symbol = sym;
                }
                self.show_updgrank = true;
                if self.updgrank_snapshot.symbol.is_empty() && !self.updgrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_updgrank(
                                &conn,
                                &self.updgrank_symbol,
                            ) {
                                self.updgrank_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            _ => false,
        }
    }
}
