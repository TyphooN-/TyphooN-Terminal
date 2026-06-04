use super::*;

impl TyphooNApp {
    pub(super) fn handle_research_round17_to34_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── Round 17 ──
            "SIZEF" | "SIZE_FACTOR" | "SIZE_RANK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.sizef_symbol = sym;
                }
                self.show_sizef = true;
                if self.sizef_snapshot.symbol.is_empty() && !self.sizef_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_sizef(&conn, &self.sizef_symbol)
                            {
                                self.sizef_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MOMF" | "MOMENTUM_RANK" | "MOM_RANK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.momf_symbol = sym;
                }
                self.show_momf = true;
                if self.momf_snapshot.symbol.is_empty() && !self.momf_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_momf(&conn, &self.momf_symbol)
                            {
                                self.momf_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PEADRANK" | "PEAD_RANK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.peadrank_symbol = sym;
                }
                self.show_peadrank = true;
                if self.peadrank_snapshot.symbol.is_empty() && !self.peadrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_peadrank(
                                &conn,
                                &self.peadrank_symbol,
                            ) {
                                self.peadrank_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "FQM" | "FUND_QUALITY" | "QUALITY_METER" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.fqm_symbol = sym;
                }
                self.show_fqm = true;
                if self.fqm_snapshot.symbol.is_empty() && !self.fqm_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_fqm(&conn, &self.fqm_symbol)
                            {
                                self.fqm_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "REVRANK" | "REV_RANK" | "REVENUE_GROWTH_RANK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.revrank_symbol = sym;
                }
                self.show_revrank = true;
                if self.revrank_snapshot.symbol.is_empty() && !self.revrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_revrank(
                                &conn,
                                &self.revrank_symbol,
                            ) {
                                self.revrank_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Round 18 ──
            "LEVRANK" | "LEV_RANK" | "LEVERAGE_RANK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
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
            }
            "OPERANK" | "OPER_RANK" | "OP_QUALITY_RANK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
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
            }
            "FQMRANK" | "FQM_RANK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
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
            }
            "LIQRANK" | "LIQ_RANK" | "LIQUIDITY_RANK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
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
            }
            "TLRANK" | "TL_RANK" | "LIQ30_RANK" | "ADV30_RANK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
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
            }
            "SURPSTK" | "EPS_STREAK" | "SURPRISE_STREAK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
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
            }
            "DVDRANK" | "DIVG_RANK" | "DIVIDEND_RANK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
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
            }
            "EARMRANK" | "EARM_RANK" | "EARNINGS_MOMENTUM_RANK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
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
            }
            "UPDGRANK" | "UPDG_RANK" | "UPGRADE_RANK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
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
            }
            "GY_STAT" | "GAP_YEARLY" | "GAPS" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.gy_symbol = sym;
                }
                self.show_gy = true;
                if self.gy_snapshot.symbol.is_empty() && !self.gy_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_gy(&conn, &self.gy_symbol)
                            {
                                self.gy_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DES_STREAK" | "DAILY_STREAK" | "EVENT_STREAK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.des_symbol = sym;
                }
                self.show_des = true;
                if self.des_snapshot.symbol.is_empty() && !self.des_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_des(&conn, &self.des_symbol)
                            {
                                self.des_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DVDYIELDRANK" | "DVDY_RANK" | "DIVIDEND_YIELD_RANK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.dvdyieldrank_symbol = sym;
                }
                self.show_dvdyieldrank = true;
                if self.dvdyieldrank_snapshot.symbol.is_empty()
                    && !self.dvdyieldrank_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_dvdyieldrank(
                                &conn,
                                &self.dvdyieldrank_symbol,
                            ) {
                                self.dvdyieldrank_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SHRANK" | "SHORT_RANK" | "SHORT_INT_RANK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.shrank_symbol = sym;
                }
                self.show_shrank = true;
                if self.shrank_snapshot.symbol.is_empty() && !self.shrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_shrank(
                                &conn,
                                &self.shrank_symbol,
                            ) {
                                self.shrank_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SHORTRANK_DELTA" | "SHORT_DELTA_RANK" | "SHORTTREND_RANK" | "SHORTRANKD" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.shortrank_delta_symbol = sym;
                }
                self.show_shortrank_delta = true;
                if self.shortrank_delta_snapshot.symbol.is_empty()
                    && !self.shortrank_delta_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_shortrank_delta(
                                    &conn,
                                    &self.shortrank_delta_symbol,
                                )
                            {
                                self.shortrank_delta_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "INSIDERCONC" | "INSIDER_CONC" | "INSIDER_OWNERSHIP_CONC" | "INSIDER_HOLD_RANK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.insiderconc_symbol = sym;
                }
                self.show_insiderconc = true;
                if self.insiderconc_snapshot.symbol.is_empty()
                    && !self.insiderconc_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_insiderconc(
                                &conn,
                                &self.insiderconc_symbol,
                            ) {
                                self.insiderconc_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ATRANN" | "ATR_ANN" | "ANNUALIZED_ATR" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.atrann_symbol = sym;
                }
                self.show_atrann = true;
                if self.atrann_snapshot.symbol.is_empty() && !self.atrann_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_atrann(
                                &conn,
                                &self.atrann_symbol,
                            ) {
                                self.atrann_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DDHIST" | "DD_HIST" | "DRAWDOWN_HIST" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.ddhist_symbol = sym;
                }
                self.show_ddhist = true;
                if self.ddhist_snapshot.symbol.is_empty() && !self.ddhist_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ddhist(
                                &conn,
                                &self.ddhist_symbol,
                            ) {
                                self.ddhist_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PRICEPERF" | "PRICE_PERF" | "MULTI_RETURN" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.priceperf_symbol = sym;
                }
                self.show_priceperf = true;
                if self.priceperf_snapshot.symbol.is_empty() && !self.priceperf_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_priceperf(
                                &conn,
                                &self.priceperf_symbol,
                            ) {
                                self.priceperf_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MOMRANK_MULTI" | "MOMRANKM" | "SECTOR_MOM_RANK" | "PRICEPERF_RANK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.momrank_multi_symbol = sym;
                }
                self.show_momrank_multi = true;
                if self.momrank_multi_snapshot.symbol.is_empty()
                    && !self.momrank_multi_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_momrank_multi(
                                    &conn,
                                    &self.momrank_multi_symbol,
                                )
                            {
                                self.momrank_multi_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "BETARANK" | "BETA_RANK" | "BRK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.betarank_symbol = sym;
                }
                self.show_betarank = true;
                if self.betarank_snapshot.symbol.is_empty() && !self.betarank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_betarank(
                                &conn,
                                &self.betarank_symbol,
                            ) {
                                self.betarank_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PEGRANK" | "PEG_RANK" | "PEG_SCORE" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.pegrank_symbol = sym;
                }
                self.show_pegrank = true;
                if self.pegrank_snapshot.symbol.is_empty() && !self.pegrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_pegrank(
                                &conn,
                                &self.pegrank_symbol,
                            ) {
                                self.pegrank_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "FHIGHLOW" | "FHL" | "52_WEEK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.fhighlow_symbol = sym;
                }
                self.show_fhighlow = true;
                if self.fhighlow_snapshot.symbol.is_empty() && !self.fhighlow_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_fhighlow(
                                &conn,
                                &self.fhighlow_symbol,
                            ) {
                                self.fhighlow_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RVCONE" | "RV_CONE" | "REAL_VOL_CONE" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.rvcone_symbol = sym;
                }
                self.show_rvcone = true;
                if self.rvcone_snapshot.symbol.is_empty() && !self.rvcone_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_rvcone(
                                &conn,
                                &self.rvcone_symbol,
                            ) {
                                self.rvcone_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CALPB" | "CAL_PB" | "CAL_BREAK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.calpb_symbol = sym;
                }
                self.show_calpb = true;
                if self.calpb_snapshot.symbol.is_empty() && !self.calpb_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_calpb(&conn, &self.calpb_symbol)
                            {
                                self.calpb_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CORRSTK" | "CORR_STK" | "BENCH_CORR" | "SPY_CORR" | "SECTOR_CORR" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.corrstk_symbol = sym;
                }
                self.show_corrstk = true;
                if self.corrstk_snapshot.symbol.is_empty() && !self.corrstk_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_corrstk(
                                &conn,
                                &self.corrstk_symbol,
                            ) {
                                self.corrstk_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CORRRANK" | "CORR_RANK" | "BENCH_RANK" | "CORR_LINK_RANK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.corrrank_symbol = sym;
                }
                self.show_corrrank = true;
                if self.corrrank_snapshot.symbol.is_empty() && !self.corrrank_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_corrrank(
                                &conn,
                                &self.corrrank_symbol,
                            ) {
                                self.corrrank_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "OPERANK_DELTA" | "OPERANKD" | "OP_MARGIN_DELTA_RANK" | "OPERATING_MARGIN_DELTA" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.operank_delta_symbol = sym;
                }
                self.show_operank_delta = true;
                if self.operank_delta_snapshot.symbol.is_empty()
                    && !self.operank_delta_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_operank_delta(
                                    &conn,
                                    &self.operank_delta_symbol,
                                )
                            {
                                self.operank_delta_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DIVACC" | "DIV_ACCEL" | "DIVIDEND_ACCELERATION" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.divacc_symbol = sym;
                }
                self.show_divacc = true;
                if self.divacc_snapshot.symbol.is_empty() && !self.divacc_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_divacc(
                                &conn,
                                &self.divacc_symbol,
                            ) {
                                self.divacc_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "EPSACC" | "EPS_ACCEL" | "EARNINGS_ACCELERATION" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.epsacc_symbol = sym;
                }
                self.show_epsacc = true;
                if self.epsacc_snapshot.symbol.is_empty() && !self.epsacc_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_epsacc(
                                &conn,
                                &self.epsacc_symbol,
                            ) {
                                self.epsacc_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VRP" | "VOL_RISK_PREMIUM" | "IV_RV_RATIO" | "REALIZED_VS_IMPLIED_VOL_RATIO" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.vrp_symbol = sym;
                }
                self.show_vrp = true;
                if self.vrp_snapshot.symbol.is_empty() && !self.vrp_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_vrp(&conn, &self.vrp_symbol)
                            {
                                self.vrp_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Round 22 palette entries ──
            "RETSKEW" | "RET_SKEW" | "SKEWNESS" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.retskew_symbol = sym;
                }
                self.show_retskew = true;
                if self.retskew_snapshot.symbol.is_empty() && !self.retskew_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_retskew(
                                &conn,
                                &self.retskew_symbol,
                            ) {
                                self.retskew_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RETKURT" | "RET_KURT" | "KURTOSIS" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.retkurt_symbol = sym;
                }
                self.show_retkurt = true;
                if self.retkurt_snapshot.symbol.is_empty() && !self.retkurt_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_retkurt(
                                &conn,
                                &self.retkurt_symbol,
                            ) {
                                self.retkurt_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "TAILR" | "TAIL_RATIO" | "TAILRATIO" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.tailr_symbol = sym;
                }
                self.show_tailr = true;
                if self.tailr_snapshot.symbol.is_empty() && !self.tailr_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_tailr(&conn, &self.tailr_symbol)
                            {
                                self.tailr_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RUNLEN" | "RUN_LEN" | "RUN_LENGTH" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.runlen_symbol = sym;
                }
                self.show_runlen = true;
                if self.runlen_snapshot.symbol.is_empty() && !self.runlen_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_runlen(
                                &conn,
                                &self.runlen_symbol,
                            ) {
                                self.runlen_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DAYRANGE" | "DAY_RANGE" | "RANGESTAT" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.dayrange_symbol = sym;
                }
                self.show_dayrange = true;
                if self.dayrange_snapshot.symbol.is_empty() && !self.dayrange_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_dayrange(
                                &conn,
                                &self.dayrange_symbol,
                            ) {
                                self.dayrange_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Round 23 ──
            "AUTOCOR" | "AUTO_COR" | "ACF" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.autocor_symbol = sym;
                }
                self.show_autocor = true;
                if self.autocor_snapshot.symbol.is_empty() && !self.autocor_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_autocor(
                                &conn,
                                &self.autocor_symbol,
                            ) {
                                self.autocor_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HURST" | "HURST_EXPONENT" | "RESCALED_RANGE" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.hurst_symbol = sym;
                }
                self.show_hurst = true;
                if self.hurst_snapshot.symbol.is_empty() && !self.hurst_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_hurst(&conn, &self.hurst_symbol)
                            {
                                self.hurst_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HITRATE" | "HIT_RATE" | "WIN_RATE" | "WINRATE" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.hitrate_symbol = sym;
                }
                self.show_hitrate = true;
                if self.hitrate_snapshot.symbol.is_empty() && !self.hitrate_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_hitrate(
                                &conn,
                                &self.hitrate_symbol,
                            ) {
                                self.hitrate_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "GLASYM" | "GL_ASYM" | "GAIN_LOSS_ASYM" | "GAINLOSSASYM" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.glasym_symbol = sym;
                }
                self.show_glasym = true;
                if self.glasym_snapshot.symbol.is_empty() && !self.glasym_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_glasym(
                                &conn,
                                &self.glasym_symbol,
                            ) {
                                self.glasym_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VOLRATIO" | "VOL_RATIO" | "VOLUMERATIO" | "VOLUME_RATIO" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.volratio_symbol = sym;
                }
                self.show_volratio = true;
                if self.volratio_snapshot.symbol.is_empty() && !self.volratio_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_volratio(
                                &conn,
                                &self.volratio_symbol,
                            ) {
                                self.volratio_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Round 24 palette ──
            "DRAWUP" | "DRAW_UP" | "RALLYHIST" | "RALLY_HISTORY" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.drawup_symbol = sym;
                }
                self.show_drawup = true;
                if self.drawup_snapshot.symbol.is_empty() && !self.drawup_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_drawup(
                                &conn,
                                &self.drawup_symbol,
                            ) {
                                self.drawup_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "GAPSTATS" | "GAP_STATS" | "GAP" | "OVERNIGHT_GAP" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.gapstats_symbol = sym;
                }
                self.show_gapstats = true;
                if self.gapstats_snapshot.symbol.is_empty() && !self.gapstats_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_gapstats(
                                &conn,
                                &self.gapstats_symbol,
                            ) {
                                self.gapstats_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VOLCLUSTER" | "VOL_CLUSTER" | "ARCH" | "VOLATILITYCLUSTER" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.volcluster_symbol = sym;
                }
                self.show_volcluster = true;
                if self.volcluster_snapshot.symbol.is_empty() && !self.volcluster_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_volcluster(
                                &conn,
                                &self.volcluster_symbol,
                            ) {
                                self.volcluster_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CLOSEPLC" | "CLOSE_PLC" | "CLOSEPLACEMENT" | "CLOSE_PLACEMENT" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.closeplc_symbol = sym;
                }
                self.show_closeplc = true;
                if self.closeplc_snapshot.symbol.is_empty() && !self.closeplc_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_closeplc(
                                &conn,
                                &self.closeplc_symbol,
                            ) {
                                self.closeplc_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MRHL" | "HALF_LIFE" | "HALFLIFE" | "AR1" | "MEAN_REVERT_HL" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.mrhl_symbol = sym;
                }
                self.show_mrhl = true;
                if self.mrhl_snapshot.symbol.is_empty() && !self.mrhl_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_mrhl(&conn, &self.mrhl_symbol)
                            {
                                self.mrhl_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Round 25 palette ──
            "DOWNVOL" | "DOWN_VOL" | "SEMIDEV" | "SORTINO" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.downvol_symbol = sym;
                }
                self.show_downvol = true;
                if self.downvol_snapshot.symbol.is_empty() && !self.downvol_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_downvol(
                                &conn,
                                &self.downvol_symbol,
                            ) {
                                self.downvol_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SHARPR" | "SHARPE" | "SHARPE_RATIO" | "SHARPERATIO" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.sharpr_symbol = sym;
                }
                self.show_sharpr = true;
                if self.sharpr_snapshot.symbol.is_empty() && !self.sharpr_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_sharpr(
                                &conn,
                                &self.sharpr_symbol,
                            ) {
                                self.sharpr_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "EFFRATIO" | "EFF_RATIO" | "KAUFMAN" | "KAUFMAN_ER" | "KER" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.effratio_symbol = sym;
                }
                self.show_effratio = true;
                if self.effratio_snapshot.symbol.is_empty() && !self.effratio_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_effratio(
                                &conn,
                                &self.effratio_symbol,
                            ) {
                                self.effratio_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "WICKBIAS" | "WICK_BIAS" | "WICKS" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.wickbias_symbol = sym;
                }
                self.show_wickbias = true;
                if self.wickbias_snapshot.symbol.is_empty() && !self.wickbias_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_wickbias(
                                &conn,
                                &self.wickbias_symbol,
                            ) {
                                self.wickbias_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VOLOFVOL" | "VOL_OF_VOL" | "VOV" | "VVOL" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.volofvol_symbol = sym;
                }
                self.show_volofvol = true;
                if self.volofvol_snapshot.symbol.is_empty() && !self.volofvol_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_volofvol(
                                &conn,
                                &self.volofvol_symbol,
                            ) {
                                self.volofvol_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Round 26 palette ──
            "CALMAR" | "CALMAR_RATIO" | "CALMARRATIO" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.calmar_symbol = sym;
                }
                self.show_calmar = true;
                if self.calmar_snapshot.symbol.is_empty() && !self.calmar_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_calmar(
                                &conn,
                                &self.calmar_symbol,
                            ) {
                                self.calmar_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ULCER" | "ULCER_INDEX" | "ULCERINDEX" | "MARTIN" | "UPI" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.ulcer_symbol = sym;
                }
                self.show_ulcer = true;
                if self.ulcer_snapshot.symbol.is_empty() && !self.ulcer_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_ulcer(&conn, &self.ulcer_symbol)
                            {
                                self.ulcer_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VARRATIO" | "VAR_RATIO" | "VARIANCE_RATIO" | "LO_MACKINLAY" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.varratio_symbol = sym;
                }
                self.show_varratio = true;
                if self.varratio_snapshot.symbol.is_empty() && !self.varratio_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_varratio(
                                &conn,
                                &self.varratio_symbol,
                            ) {
                                self.varratio_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "AMIHUD" | "AMIHUD_ILLIQ" | "ILLIQ" | "ILLIQUIDITY" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.amihud_symbol = sym;
                }
                self.show_amihud = true;
                if self.amihud_snapshot.symbol.is_empty() && !self.amihud_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_amihud(
                                &conn,
                                &self.amihud_symbol,
                            ) {
                                self.amihud_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "JBNORM" | "JB" | "JARQUE_BERA" | "NORMALITY" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.jbnorm_symbol = sym;
                }
                self.show_jbnorm = true;
                if self.jbnorm_snapshot.symbol.is_empty() && !self.jbnorm_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_jbnorm(
                                &conn,
                                &self.jbnorm_symbol,
                            ) {
                                self.jbnorm_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Round 27 palette ──
            "OMEGA" | "OMEGA_RATIO" | "OMEGARATIO" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.omega_symbol = sym;
                }
                self.show_omega = true;
                if self.omega_snapshot.symbol.is_empty() && !self.omega_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_omega(&conn, &self.omega_symbol)
                            {
                                self.omega_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DFA" | "DETRENDED_FLUCT" | "DETRENDED_FLUCTUATION" | "DFAALPHA" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.dfa_symbol = sym;
                }
                self.show_dfa = true;
                if self.dfa_snapshot.symbol.is_empty() && !self.dfa_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_dfa(&conn, &self.dfa_symbol)
                            {
                                self.dfa_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "BURKE" | "BURKE_RATIO" | "BURKERATIO" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.burke_symbol = sym;
                }
                self.show_burke = true;
                if self.burke_snapshot.symbol.is_empty() && !self.burke_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_burke(&conn, &self.burke_symbol)
                            {
                                self.burke_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MONTHSEAS" | "MONTHLY_SEASONALITY" | "MONTHLYSEASONALITY" | "SEAS" | "MONTH_SEAS" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.monthseas_symbol = sym;
                }
                self.show_monthseas = true;
                if self.monthseas_snapshot.symbol.is_empty() && !self.monthseas_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_monthseas(
                                &conn,
                                &self.monthseas_symbol,
                            ) {
                                self.monthseas_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ROLLSPRD" | "ROLL_SPREAD" | "ROLLSPREAD" | "ROLL" | "EFFECTIVE_SPREAD" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.rollsprd_symbol = sym;
                }
                self.show_rollsprd = true;
                if self.rollsprd_snapshot.symbol.is_empty() && !self.rollsprd_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_rollsprd(
                                &conn,
                                &self.rollsprd_symbol,
                            ) {
                                self.rollsprd_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Round 28 palette ──
            "PARKINSON" | "PARKINSON_VOL" | "PARKVOL" | "HL_VOL" | "RANGE_VOL" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.parkinson_symbol = sym;
                }
                self.show_parkinson = true;
                if self.parkinson_snapshot.symbol.is_empty() && !self.parkinson_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_parkinson(
                                &conn,
                                &self.parkinson_symbol,
                            ) {
                                self.parkinson_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "GKVOL" | "GARMAN_KLASS" | "GARMANKLASS" | "GK_VOL" | "GARMAN_KLASS_VOL" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.gkvol_symbol = sym;
                }
                self.show_gkvol = true;
                if self.gkvol_snapshot.symbol.is_empty() && !self.gkvol_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_gkvol(&conn, &self.gkvol_symbol)
                            {
                                self.gkvol_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RSVOL" | "ROGERS_SATCHELL" | "ROGERSSATCHELL" | "RS_VOL" | "DRIFT_FREE_VOL" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.rsvol_symbol = sym;
                }
                self.show_rsvol = true;
                if self.rsvol_snapshot.symbol.is_empty() && !self.rsvol_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_rsvol(&conn, &self.rsvol_symbol)
                            {
                                self.rsvol_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CVAR" | "EXPECTED_SHORTFALL" | "ES" | "CONDITIONAL_VAR" | "ES5" | "ES_5"
            | "TAIL_EXPECTED" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.cvar_symbol = sym;
                }
                self.show_cvar = true;
                if self.cvar_snapshot.symbol.is_empty() && !self.cvar_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cvar(&conn, &self.cvar_symbol)
                            {
                                self.cvar_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DOWEFFECT" | "DOW_EFFECT" | "DOW" | "WEEKDAY_EFFECT" | "DAY_OF_WEEK" | "DAYOFWEEK" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.doweffect_symbol = sym;
                }
                self.show_doweffect = true;
                if self.doweffect_snapshot.symbol.is_empty() && !self.doweffect_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_doweffect(
                                &conn,
                                &self.doweffect_symbol,
                            ) {
                                self.doweffect_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Round 29 palette ──
            "STERLING" | "STERLING_RATIO" | "STERLINGRATIO" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.sterling_symbol = sym;
                }
                self.show_sterling = true;
                if self.sterling_snapshot.symbol.is_empty() && !self.sterling_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_sterling(
                                &conn,
                                &self.sterling_symbol,
                            ) {
                                self.sterling_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "KELLYF" | "KELLY" | "KELLY_FRACTION" | "KELLY_CRITERION" | "OPTIMAL_F" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.kellyf_symbol = sym;
                }
                self.show_kellyf = true;
                if self.kellyf_snapshot.symbol.is_empty() && !self.kellyf_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_kellyf(
                                &conn,
                                &self.kellyf_symbol,
                            ) {
                                self.kellyf_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "LJUNGB" | "LJUNG_BOX" | "LJUNGBOX" | "PORTMANTEAU" | "QSTAT" | "Q_STAT" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.ljungb_symbol = sym;
                }
                self.show_ljungb = true;
                if self.ljungb_snapshot.symbol.is_empty() && !self.ljungb_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ljungb(
                                &conn,
                                &self.ljungb_symbol,
                            ) {
                                self.ljungb_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RUNSTEST" | "RUNS_TEST" | "WALD_WOLFOWITZ" | "WW_RUNS" | "SIGN_RUNS" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.runstest_symbol = sym;
                }
                self.show_runstest = true;
                if self.runstest_snapshot.symbol.is_empty() && !self.runstest_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_runstest(
                                &conn,
                                &self.runstest_symbol,
                            ) {
                                self.runstest_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ZERORET" | "ZERO_RETURN" | "LOT" | "LESMOND" | "ZERO_DAYS" | "ZERODAYS" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.zeroret_symbol = sym;
                }
                self.show_zeroret = true;
                if self.zeroret_snapshot.symbol.is_empty() && !self.zeroret_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_zeroret(
                                &conn,
                                &self.zeroret_symbol,
                            ) {
                                self.zeroret_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Round 30 palette ──
            "PSR"
            | "PROB_SHARPE"
            | "PROBSHARPE"
            | "PROBABILISTIC_SHARPE"
            | "PROBABILISTICSHARPE" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.psr_symbol = sym;
                }
                self.show_psr = true;
                if self.psr_snapshot.symbol.is_empty() && !self.psr_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_psr(&conn, &self.psr_symbol)
                            {
                                self.psr_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ADF" | "DICKEY_FULLER" | "DICKEYFULLER" | "UNIT_ROOT" | "UNITROOT"
            | "STATIONARITY" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.adf_symbol = sym;
                }
                self.show_adf = true;
                if self.adf_snapshot.symbol.is_empty() && !self.adf_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_adf(&conn, &self.adf_symbol)
                            {
                                self.adf_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MNKENDALL" | "MANN_KENDALL" | "MANNKENDALL" | "KENDALL_TREND" | "TREND_TEST" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.mnkendall_symbol = sym;
                }
                self.show_mnkendall = true;
                if self.mnkendall_snapshot.symbol.is_empty() && !self.mnkendall_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_mnkendall(
                                &conn,
                                &self.mnkendall_symbol,
                            ) {
                                self.mnkendall_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "BIPOWER" | "BPV" | "BIPOWER_VAR" | "BIPOWERVAR" | "JUMP_RATIO" | "JUMPRATIO"
            | "BN_JUMP" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.bipower_symbol = sym;
                }
                self.show_bipower = true;
                if self.bipower_snapshot.symbol.is_empty() && !self.bipower_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_bipower(
                                &conn,
                                &self.bipower_symbol,
                            ) {
                                self.bipower_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DDDUR" | "DD_DURATION" | "DRAWDOWN_DURATION" | "DDDURATION" | "UNDERWATER"
            | "DRAWDOWNDURATION" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.dddur_symbol = sym;
                }
                self.show_dddur = true;
                if self.dddur_snapshot.symbol.is_empty() && !self.dddur_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_dddur(&conn, &self.dddur_symbol)
                            {
                                self.dddur_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Round 31 palette ──
            "HILLTAIL" | "HILL" | "HILL_TAIL" | "TAIL_INDEX" | "TAILINDEX" | "HILLESTIMATOR"
            | "POWER_LAW_TAIL" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.hilltail_symbol = sym;
                }
                self.show_hilltail = true;
                if self.hilltail_snapshot.symbol.is_empty() && !self.hilltail_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_hilltail(
                                &conn,
                                &self.hilltail_symbol,
                            ) {
                                self.hilltail_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ARCHLM" | "ARCH_LM" | "ENGLE_ARCH" | "ARCH_TEST" | "HETEROSKEDASTIC"
            | "HETERO_TEST" | "VOLCLUSTER_TEST" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.archlm_symbol = sym;
                }
                self.show_archlm = true;
                if self.archlm_snapshot.symbol.is_empty() && !self.archlm_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_archlm(
                                &conn,
                                &self.archlm_symbol,
                            ) {
                                self.archlm_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PAINRATIO" | "PAIN_RATIO" | "PAIN_INDEX" | "PAININDEX" | "PAIN" | "ZEPHYR_PAIN" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.painratio_symbol = sym;
                }
                self.show_painratio = true;
                if self.painratio_snapshot.symbol.is_empty() && !self.painratio_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_painratio(
                                &conn,
                                &self.painratio_symbol,
                            ) {
                                self.painratio_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CUSUM" | "BDE_CUSUM" | "STRUCTURAL_BREAK" | "MEAN_BREAK" | "CUSUM_TEST"
            | "STABILITY_TEST" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.cusum_symbol = sym;
                }
                self.show_cusum = true;
                if self.cusum_snapshot.symbol.is_empty() && !self.cusum_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cusum(&conn, &self.cusum_symbol)
                            {
                                self.cusum_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CFVAR" | "CORNISH_FISHER" | "CORNISHFISHER" | "MODIFIED_VAR" | "MODIFIEDVAR"
            | "CF_VAR" | "SKEW_KURT_VAR" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.cfvar_symbol = sym;
                }
                self.show_cfvar = true;
                if self.cfvar_snapshot.symbol.is_empty() && !self.cfvar_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cfvar(&conn, &self.cfvar_symbol)
                            {
                                self.cfvar_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Round 32 palette ──
            "ENTROPY" | "SHANNON" | "SHANNON_ENTROPY" | "SHANNONENTROPY" | "RETURN_ENTROPY"
            | "RETURNENTROPY" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.entropy_symbol = sym;
                }
                self.show_entropy = true;
                if self.entropy_snapshot.symbol.is_empty() && !self.entropy_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_entropy(
                                &conn,
                                &self.entropy_symbol,
                            ) {
                                self.entropy_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RACHEV"
            | "RACHEV_RATIO"
            | "RACHEVRATIO"
            | "ETL_RATIO"
            | "ETLRATIO"
            | "TAIL_EXPECTATION_RATIO" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.rachev_symbol = sym;
                }
                self.show_rachev = true;
                if self.rachev_snapshot.symbol.is_empty() && !self.rachev_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_rachev(
                                &conn,
                                &self.rachev_symbol,
                            ) {
                                self.rachev_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "GPR" | "GAIN_TO_PAIN" | "GAINTOPAIN" | "GAIN_PAIN" | "GAINPAIN" | "PROFIT_FACTOR"
            | "PROFITFACTOR" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.gpr_symbol = sym;
                }
                self.show_gpr = true;
                if self.gpr_snapshot.symbol.is_empty() && !self.gpr_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_gpr(&conn, &self.gpr_symbol)
                            {
                                self.gpr_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PACF"
            | "PARTIAL_ACF"
            | "PARTIALACF"
            | "PARTIAL_AUTOCORRELATION"
            | "PARTIALAUTOCORRELATION"
            | "PACF_LAG" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.pacf_symbol = sym;
                }
                self.show_pacf = true;
                if self.pacf_snapshot.symbol.is_empty() && !self.pacf_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_pacf(&conn, &self.pacf_symbol)
                            {
                                self.pacf_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "APEN"
            | "APPROX_ENTROPY"
            | "APPROXENTROPY"
            | "APPROXIMATE_ENTROPY"
            | "APPROXIMATEENTROPY"
            | "PINCUS" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.apen_symbol = sym;
                }
                self.show_apen = true;
                if self.apen_snapshot.symbol.is_empty() && !self.apen_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_apen(&conn, &self.apen_symbol)
                            {
                                self.apen_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Round 33 palette ──
            "UPR" | "UPSIDE_POTENTIAL" | "UPSIDEPOTENTIAL" | "UPSIDE_RATIO" | "UPSIDERATIO" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.upr_symbol = sym;
                }
                self.show_upr = true;
                if self.upr_snapshot.symbol.is_empty() && !self.upr_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_upr(&conn, &self.upr_symbol)
                            {
                                self.upr_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "LEVEREFF" | "LEVERAGE_EFFECT" | "LEVERAGEEFFECT" | "LEVER_EFF" | "ASYM_VOL"
            | "ASYMVOL" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.levereff_symbol = sym;
                }
                self.show_levereff = true;
                if self.levereff_snapshot.symbol.is_empty() && !self.levereff_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_levereff(
                                &conn,
                                &self.levereff_symbol,
                            ) {
                                self.levereff_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DRAWDAR" | "DRAWDOWN_AT_RISK" | "DRAWDOWNATRISK" | "DAR" | "CDAR"
            | "CONDITIONAL_DAR" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.drawdar_symbol = sym;
                }
                self.show_drawdar = true;
                if self.drawdar_snapshot.symbol.is_empty() && !self.drawdar_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_drawdar(
                                &conn,
                                &self.drawdar_symbol,
                            ) {
                                self.drawdar_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VARHALF"
            | "VOL_HALFLIFE"
            | "VOLHALFLIFE"
            | "VOL_PERSIST"
            | "VOLPERSIST"
            | "VOLATILITY_HALFLIFE" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.varhalf_symbol = sym;
                }
                self.show_varhalf = true;
                if self.varhalf_snapshot.symbol.is_empty() && !self.varhalf_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_varhalf(
                                &conn,
                                &self.varhalf_symbol,
                            ) {
                                self.varhalf_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "GINI" | "GINI_COEFF" | "GINICOEFF" | "GINI_COEFFICIENT" | "RETURN_CONCENTRATION" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.gini_symbol = sym;
                }
                self.show_gini = true;
                if self.gini_snapshot.symbol.is_empty() && !self.gini_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_gini(&conn, &self.gini_symbol)
                            {
                                self.gini_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Round 34 palette ──
            "SAMPEN" | "SAMPLE_ENTROPY" | "SAMPLEENTROPY" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.sampen_symbol = sym;
                }
                self.show_sampen = true;
                if self.sampen_snapshot.symbol.is_empty() && !self.sampen_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_sampen(
                                &conn,
                                &self.sampen_symbol,
                            ) {
                                self.sampen_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PERMEN" | "PERMUTATION_ENTROPY" | "PERMENTROPY" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.permen_symbol = sym;
                }
                self.show_permen = true;
                if self.permen_snapshot.symbol.is_empty() && !self.permen_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_permen(
                                &conn,
                                &self.permen_symbol,
                            ) {
                                self.permen_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RECFACT" | "RECOVERY_FACTOR" | "RECOVERYFACTOR" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.recfact_symbol = sym;
                }
                self.show_recfact = true;
                if self.recfact_snapshot.symbol.is_empty() && !self.recfact_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_recfact(
                                &conn,
                                &self.recfact_symbol,
                            ) {
                                self.recfact_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "KPSS" | "KPSS_TEST" | "KPSSTEST" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.kpss_symbol = sym;
                }
                self.show_kpss = true;
                if self.kpss_snapshot.symbol.is_empty() && !self.kpss_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_kpss(&conn, &self.kpss_symbol)
                            {
                                self.kpss_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SPECENT" | "SPECTRAL_ENTROPY" | "SPECTRALENTROPY" => {
                let sym = self
                    .charts
                    .get(self.active_tab)
                    .map(|c| {
                        c.symbol
                            .split(':')
                            .rev()
                            .nth(1)
                            .or_else(|| c.symbol.split(':').last())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                if !sym.is_empty() {
                    self.specent_symbol = sym;
                }
                self.show_specent = true;
                if self.specent_snapshot.symbol.is_empty() && !self.specent_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_specent(
                                &conn,
                                &self.specent_symbol,
                            ) {
                                self.specent_snapshot = snap;
                            }
                        }
                    }
                }
            }
            _ => return false,
        }
        true
    }
}
