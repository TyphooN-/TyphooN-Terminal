use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_residual_cycle_memory_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── Residual, cycle, memory, and rank-dependence palette aliases ──
            "DURBINWATSON" | "DURBIN_WATSON" | "DW" | "DWSTAT" | "DWTEST" | "RESIDAC" => {
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
                    self.durbinwatson_symbol = sym;
                }
                self.show_durbinwatson = true;
                if self.durbinwatson_snapshot.symbol.is_empty()
                    && !self.durbinwatson_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_durbinwatson(
                                &conn,
                                &self.durbinwatson_symbol,
                            ) {
                                self.durbinwatson_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "BDSTEST" | "BDS_TEST" | "BDS" | "BROCK_DECHERT" | "BROCKDECHERT" | "IIDTEST" => {
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
                    self.bdstest_symbol = sym;
                }
                self.show_bdstest = true;
                if self.bdstest_snapshot.symbol.is_empty() && !self.bdstest_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_bdstest(
                                &conn,
                                &self.bdstest_symbol,
                            ) {
                                self.bdstest_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "BREUSCHPAGAN" | "BREUSCH_PAGAN" | "BP" | "BPTEST" | "HETEROTEST" | "HETEROLMTEST" => {
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
                    self.breuschpagan_symbol = sym;
                }
                self.show_breuschpagan = true;
                if self.breuschpagan_snapshot.symbol.is_empty()
                    && !self.breuschpagan_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_breuschpagan(
                                &conn,
                                &self.breuschpagan_symbol,
                            ) {
                                self.breuschpagan_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "TURNPTS" | "TURN_PTS" | "TURNINGPOINTS" | "BARTELS" | "TURNINGTEST" | "TURNINGPTS" => {
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
                    self.turnpts_symbol = sym;
                }
                self.show_turnpts = true;
                if self.turnpts_snapshot.symbol.is_empty() && !self.turnpts_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_turnpts(
                                &conn,
                                &self.turnpts_symbol,
                            ) {
                                self.turnpts_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "PERIODOGRAM" | "PERGRAM" | "DFTSPEC" | "SPECDENSITY" | "DOMINANTCYCLE"
            | "CYCLEFINDER" => {
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
                    self.periodogram_symbol = sym;
                }
                self.show_periodogram = true;
                if self.periodogram_snapshot.symbol.is_empty()
                    && !self.periodogram_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_periodogram(
                                &conn,
                                &self.periodogram_symbol,
                            ) {
                                self.periodogram_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "MCLEODLI" | "MCLEOD" | "MLTEST" | "SQRETURNS" | "ARCHPORTMANTEAU" => {
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
                    self.mcleodli_symbol = sym;
                }
                self.show_mcleodli = true;
                if self.mcleodli_snapshot.symbol.is_empty() && !self.mcleodli_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_mcleodli(
                                &conn,
                                &self.mcleodli_symbol,
                            ) {
                                self.mcleodli_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "OUFIT" | "ORNSTEIN" | "OU" | "OUPROCESS" | "OU_FIT" | "MEANREVERTFIT" => {
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
                    self.oufit_symbol = sym;
                }
                self.show_oufit = true;
                if self.oufit_snapshot.symbol.is_empty() && !self.oufit_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_oufit(&conn, &self.oufit_symbol)
                            {
                                self.oufit_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "GPH" | "GEWEKE" | "GEWEKEPORTERHUDAK" | "LONGMEMORY" | "FRACTIONAL_D"
            | "LOGPERIODOGRAM" => {
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
                    self.gph_symbol = sym;
                }
                self.show_gph = true;
                if self.gph_snapshot.symbol.is_empty() && !self.gph_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_gph(&conn, &self.gph_symbol)
                            {
                                self.gph_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "BURGSPEC" | "BURG" | "MAXENTROPY" | "ARSPECTRUM" | "MESPEC" => {
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
                    self.burgspec_symbol = sym;
                }
                self.show_burgspec = true;
                if self.burgspec_snapshot.symbol.is_empty() && !self.burgspec_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_burgspec(
                                &conn,
                                &self.burgspec_symbol,
                            ) {
                                self.burgspec_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "KENDALLTAU" | "KTAU" | "RANKAUTOCORR" | "TAULAG1" | "KENDALLLAG" => {
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
                    self.kendalltau_symbol = sym;
                }
                self.show_kendalltau = true;
                if self.kendalltau_snapshot.symbol.is_empty() && !self.kendalltau_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_kendalltau(
                                &conn,
                                &self.kendalltau_symbol,
                            ) {
                                self.kendalltau_snapshot = snap;
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
