use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_volume_momentum_trend_cycle_command(
        &mut self,
        cmd_upper: &String,
    ) -> bool {
        match cmd_upper.as_str() {
            // ── Volume, momentum, and trend-cycle palette aliases ──
            // Bare EFI / EMV / NVI / PVI / COPPOCK are unbound upstream (verified) and kept as aliases.
            "EFI" | "EFIFIT" | "EFI_WIN" | "FORCE_INDEX" | "ELDER_FORCE" => {
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
                    self.efi_win_symbol = sym;
                }
                self.show_efi_win = true;
                if self.efi_win_snapshot.symbol.is_empty() && !self.efi_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_efi(&conn, &self.efi_win_symbol)
                            {
                                self.efi_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "EMV" | "EMVFIT" | "EMV_WIN" | "EASE_OF_MOVEMENT" | "ARMS_EMV" => {
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
                    self.emv_win_symbol = sym;
                }
                self.show_emv_win = true;
                if self.emv_win_snapshot.symbol.is_empty() && !self.emv_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_emv(&conn, &self.emv_win_symbol)
                            {
                                self.emv_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "NVI" | "NVIFIT" | "NVI_WIN" | "NEG_VOLUME_INDEX" | "NEGATIVE_VOLUME" => {
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
                    self.nvi_win_symbol = sym;
                }
                self.show_nvi_win = true;
                if self.nvi_win_snapshot.symbol.is_empty() && !self.nvi_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_nvi(&conn, &self.nvi_win_symbol)
                            {
                                self.nvi_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "PVI" | "PVIFIT" | "PVI_WIN" | "POS_VOLUME_INDEX" | "POSITIVE_VOLUME" => {
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
                    self.pvi_win_symbol = sym;
                }
                self.show_pvi_win = true;
                if self.pvi_win_snapshot.symbol.is_empty() && !self.pvi_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_pvi(&conn, &self.pvi_win_symbol)
                            {
                                self.pvi_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "COPPOCK" | "COPPOCKFIT" | "COPPOCK_WIN" | "COPPOCK_CURVE" | "COPPOCK_GUIDE" => {
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
                    self.coppock_win_symbol = sym;
                }
                self.show_coppock_win = true;
                if self.coppock_win_snapshot.symbol.is_empty()
                    && !self.coppock_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_coppock(
                                &conn,
                                &self.coppock_win_symbol,
                            ) {
                                self.coppock_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "CMO" | "CMOFIT" | "CMO_WIN" | "CHANDE_MOMENTUM" | "CHANDE_MO" => {
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
                    self.cmo_win_symbol = sym;
                }
                self.show_cmo_win = true;
                if self.cmo_win_snapshot.symbol.is_empty() && !self.cmo_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cmo(&conn, &self.cmo_win_symbol)
                            {
                                self.cmo_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "QSTICK" | "QSTICKFIT" | "QSTICK_WIN" | "Q_STICK" | "CHANDE_QSTICK" => {
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
                    self.qstick_win_symbol = sym;
                }
                self.show_qstick_win = true;
                if self.qstick_win_snapshot.symbol.is_empty() && !self.qstick_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_qstick(
                                &conn,
                                &self.qstick_win_symbol,
                            ) {
                                self.qstick_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "DISPARITY" | "DISPARITYFIT" | "DISPARITY_WIN" | "DISPARITY_INDEX" | "DISP_INDEX" => {
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
                    self.disparity_win_symbol = sym;
                }
                self.show_disparity_win = true;
                if self.disparity_win_snapshot.symbol.is_empty()
                    && !self.disparity_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_disparity(
                                &conn,
                                &self.disparity_win_symbol,
                            ) {
                                self.disparity_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "BOP" | "BOPFIT" | "BOP_WIN" | "BALANCE_OF_POWER" | "LIVSHIN_BOP" => {
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
                    self.bop_win_symbol = sym;
                }
                self.show_bop_win = true;
                if self.bop_win_snapshot.symbol.is_empty() && !self.bop_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_bop(&conn, &self.bop_win_symbol)
                            {
                                self.bop_win_snapshot = snap;
                            }
                        }
                    }
                }
                true
            }
            "SCHAFF" | "SCHAFFFIT" | "SCHAFF_WIN" | "STC" | "SCHAFF_TREND_CYCLE" => {
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
                    self.schaff_win_symbol = sym;
                }
                self.show_schaff_win = true;
                if self.schaff_win_snapshot.symbol.is_empty() && !self.schaff_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_schaff(
                                &conn,
                                &self.schaff_win_symbol,
                            ) {
                                self.schaff_win_snapshot = snap;
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
