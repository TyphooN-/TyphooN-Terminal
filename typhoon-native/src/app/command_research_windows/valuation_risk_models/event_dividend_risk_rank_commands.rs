use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_event_dividend_risk_rank_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── Event, dividend, and risk-rank palette aliases ──
            "GY_STAT" | "GAP_YEARLY" | "GAPS" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "DES_STREAK" | "DAILY_STREAK" | "EVENT_STREAK" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "DVDYIELDRANK" | "DVDY_RANK" | "DIVIDEND_YIELD_RANK" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "SHRANK" | "SHORT_RANK" | "SHORT_INT_RANK" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "SHORTRANK_DELTA" | "SHORT_DELTA_RANK" | "SHORTTREND_RANK" | "SHORTRANKD" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "INSIDERCONC" | "INSIDER_CONC" | "INSIDER_OWNERSHIP_CONC" | "INSIDER_HOLD_RANK" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "ATRANN" | "ATR_ANN" | "ANNUALIZED_ATR" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "DDHIST" | "DD_HIST" | "DRAWDOWN_HIST" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "PRICEPERF" | "PRICE_PERF" | "MULTI_RETURN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "MOMRANK_MULTI" | "MOMRANKM" | "SECTOR_MOM_RANK" | "PRICEPERF_RANK" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "BETARANK" | "BETA_RANK" | "BRK" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "PEGRANK" | "PEG_RANK" | "PEG_SCORE" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "FHIGHLOW" | "FHL" | "52_WEEK" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "RVCONE" | "RV_CONE" | "REAL_VOL_CONE" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "CALPB" | "CAL_PB" | "CAL_BREAK" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "CORRSTK" | "CORR_STK" | "BENCH_CORR" | "SPY_CORR" | "SECTOR_CORR" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "CORRRANK" | "CORR_RANK" | "BENCH_RANK" | "CORR_LINK_RANK" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "OPERANK_DELTA" | "OPERANKD" | "OP_MARGIN_DELTA_RANK" | "OPERATING_MARGIN_DELTA" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "DIVACC" | "DIV_ACCEL" | "DIVIDEND_ACCELERATION" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "EPSACC" | "EPS_ACCEL" | "EARNINGS_ACCELERATION" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "VRP" | "VOL_RISK_PREMIUM" | "IV_RV_RATIO" | "REALIZED_VS_IMPLIED_VOL_RATIO" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            _ => false,
        }
    }
}
