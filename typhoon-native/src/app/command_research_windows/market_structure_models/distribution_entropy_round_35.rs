use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_distribution_entropy_round_35_command(
        &mut self,
        cmd_upper: &String,
    ) -> bool {
        match cmd_upper.as_str() {
            // ── Round 35 palette aliases ──
            "ROBVOL" | "ROBUST_VOL" | "ROBUSTVOL" => {
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
                    self.robvol_symbol = sym;
                }
                self.show_robvol = true;
                if self.robvol_snapshot.symbol.is_empty() && !self.robvol_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_robvol(
                                &conn,
                                &self.robvol_symbol,
                            ) {
                                self.robvol_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "RENYIENT" | "RENYI_ENTROPY" | "RENYIENTROPY" => {
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
                    self.renyient_symbol = sym;
                }
                self.show_renyient = true;
                if self.renyient_snapshot.symbol.is_empty() && !self.renyient_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_renyient(
                                &conn,
                                &self.renyient_symbol,
                            ) {
                                self.renyient_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "RETQUANT" | "RETURN_QUANTILES" | "RETURNQUANTILES" => {
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
                    self.retquant_symbol = sym;
                }
                self.show_retquant = true;
                if self.retquant_snapshot.symbol.is_empty() && !self.retquant_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_retquant(
                                &conn,
                                &self.retquant_symbol,
                            ) {
                                self.retquant_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "MSENT" | "MULTISCALE_ENTROPY" | "MULTISCALEENTROPY" => {
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
                    self.msent_symbol = sym;
                }
                self.show_msent = true;
                if self.msent_snapshot.symbol.is_empty() && !self.msent_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_msent(&conn, &self.msent_symbol)
                            {
                                self.msent_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "EWMAVOL" | "EWMA_VOL" | "EWMAVOLATILITY" => {
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
                    self.ewmavol_symbol = sym;
                }
                self.show_ewmavol = true;
                if self.ewmavol_snapshot.symbol.is_empty() && !self.ewmavol_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ewmavol(
                                &conn,
                                &self.ewmavol_symbol,
                            ) {
                                self.ewmavol_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "KSNORM" | "KS_NORM" | "KS_TEST" | "KSTEST" => {
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
                    self.ksnorm_symbol = sym;
                }
                self.show_ksnorm = true;
                if self.ksnorm_snapshot.symbol.is_empty() && !self.ksnorm_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ksnorm(
                                &conn,
                                &self.ksnorm_symbol,
                            ) {
                                self.ksnorm_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "ADTEST" | "AD_TEST" | "ANDERSON_DARLING" | "ANDERSONDARLING" => {
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
                    self.adtest_symbol = sym;
                }
                self.show_adtest = true;
                if self.adtest_snapshot.symbol.is_empty() && !self.adtest_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_adtest(
                                &conn,
                                &self.adtest_symbol,
                            ) {
                                self.adtest_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "LMOM" | "L_MOMENTS" | "LMOMENTS" | "HOSKING" => {
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
                    self.lmom_symbol = sym;
                }
                self.show_lmom = true;
                if self.lmom_snapshot.symbol.is_empty() && !self.lmom_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_lmom(&conn, &self.lmom_symbol)
                            {
                                self.lmom_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "KYLELAM" | "KYLE_LAMBDA" | "KYLELAMBDA" | "PRICE_IMPACT" | "PRICEIMPACT" => {
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
                    self.kylelam_symbol = sym;
                }
                self.show_kylelam = true;
                if self.kylelam_snapshot.symbol.is_empty() && !self.kylelam_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_kylelam(
                                &conn,
                                &self.kylelam_symbol,
                            ) {
                                self.kylelam_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "PEAKOVER" | "PEAKS_OVER_THRESHOLD" | "POT" | "EVT_POT" | "EXCEEDANCES" => {
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
                    self.peakover_symbol = sym;
                }
                self.show_peakover = true;
                if self.peakover_snapshot.symbol.is_empty() && !self.peakover_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_peakover(
                                &conn,
                                &self.peakover_symbol,
                            ) {
                                self.peakover_snapshot = snap;
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
