use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_directional_moneyflow_sar_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── Directional, money-flow, and SAR palette aliases ──
            // Bare ADX / CCI / PSAR are already bound to chart-overlay toggles upstream;
            // only disambiguated forms are used for ADX/CCI/PSAR research windows.
            // Bare CMF and MFI are unbound and kept as aliases.
            "ADXFIT" | "ADX_WIN" | "ADXREG" | "DIRECTIONAL_INDEX" | "WILDERADX" => {
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
                    self.adx_win_symbol = sym;
                }
                self.show_adx_win = true;
                if self.adx_win_snapshot.symbol.is_empty() && !self.adx_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_adx(&conn, &self.adx_win_symbol)
                            {
                                self.adx_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "CCIFIT" | "CCI_WIN" | "CCIREG" | "COMMODITY_CHANNEL" | "LAMBERTCCI" => {
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
                    self.cci_win_symbol = sym;
                }
                self.show_cci_win = true;
                if self.cci_win_snapshot.symbol.is_empty() && !self.cci_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cci(&conn, &self.cci_win_symbol)
                            {
                                self.cci_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "CMF" | "CMFFIT" | "CHAIKIN_MF" | "CHAIKIN_MONEY_FLOW" | "MONEYFLOW_CMF" => {
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
                    self.cmf_win_symbol = sym;
                }
                self.show_cmf_win = true;
                if self.cmf_win_snapshot.symbol.is_empty() && !self.cmf_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cmf(&conn, &self.cmf_win_symbol)
                            {
                                self.cmf_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "MFI" | "MFIFIT" | "MONEY_FLOW_INDEX" | "MFIREG" | "MFI_14" => {
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
                    self.mfi_win_symbol = sym;
                }
                self.show_mfi_win = true;
                if self.mfi_win_snapshot.symbol.is_empty() && !self.mfi_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_mfi(&conn, &self.mfi_win_symbol)
                            {
                                self.mfi_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "PSARFIT" | "PSAR_WIN" | "PARABOLIC_SAR" | "WILDER_SAR" | "SARFIT" => {
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
                    self.psar_win_symbol = sym;
                }
                self.show_psar_win = true;
                if self.psar_win_snapshot.symbol.is_empty() && !self.psar_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_psar(
                                &conn,
                                &self.psar_win_symbol,
                            ) {
                                self.psar_win_snapshot = snap;
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
