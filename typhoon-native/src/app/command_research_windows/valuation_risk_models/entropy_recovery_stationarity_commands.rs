use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_entropy_recovery_stationarity_command(
        &mut self,
        cmd_upper: &String,
    ) -> bool {
        match cmd_upper.as_str() {
            // ── Sample/permutation entropy, recovery factor, KPSS stationarity, spectral entropy aliases ──
            "SAMPEN" | "SAMPLE_ENTROPY" | "SAMPLEENTROPY" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "PERMEN" | "PERMUTATION_ENTROPY" | "PERMENTROPY" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "RECFACT" | "RECOVERY_FACTOR" | "RECOVERYFACTOR" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "KPSS" | "KPSS_TEST" | "KPSSTEST" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "SPECENT" | "SPECTRAL_ENTROPY" | "SPECTRALENTROPY" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            _ => false,
        }
    }
}
