use super::*;

impl TyphooNApp {
    pub(super) fn handle_adaptive_momentum_commands(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            "EXPCAL"
            | "OPTCAL"
            | "EXPIRY"
            | "EXPIRATIONS"
            | "OPTION_CALENDAR"
            | "OPTIONS_CALENDAR"
            | "OPTION_EXPIRATION_CALENDAR" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.expcal_win_symbol = sym;
                }
                self.show_expcal_win = true;
                if self.expcal_win_calendar.is_empty() {
                    let today = chrono::Utc::now().date_naive();
                    self.expcal_win_calendar =
                        typhoon_engine::core::research::compute_market_calendar(
                            today,
                            self.expcal_win_horizon_days,
                        );
                }
                if self.expcal_win_snapshot.symbol.is_empty() && !self.expcal_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_symbol_expirations(
                                    &conn,
                                    &self.expcal_win_symbol,
                                )
                            {
                                self.expcal_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Research section ──
            "SMMA" | "SMMAFIT" | "SMMA_WIN" | "WILDER_MA" | "WILDER_SMMA" | "RMA"
            | "SMOOTHED_MA" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.smma_win_symbol = sym;
                }
                self.show_smma_win = true;
                if self.smma_win_snapshot.symbol.is_empty() && !self.smma_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_smma(
                                &conn,
                                &self.smma_win_symbol,
                            ) {
                                self.smma_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ALLIGATOR"
            | "ALLIG"
            | "GATOR"
            | "ALLIGATOR_WIN"
            | "WILLIAMS_ALLIGATOR"
            | "BILL_WILLIAMS_ALLIGATOR" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.alligator_win_symbol = sym;
                }
                self.show_alligator_win = true;
                if self.alligator_win_snapshot.symbol.is_empty()
                    && !self.alligator_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_alligator(
                                &conn,
                                &self.alligator_win_symbol,
                            ) {
                                self.alligator_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CRSI" | "CRSIFIT" | "CRSI_WIN" | "CONNORS_RSI" | "CONNORSRSI" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.crsi_win_symbol = sym;
                }
                self.show_crsi_win = true;
                if self.crsi_win_snapshot.symbol.is_empty() && !self.crsi_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_crsi(
                                &conn,
                                &self.crsi_win_symbol,
                            ) {
                                self.crsi_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SEB" | "SEBFIT" | "SEB_WIN" | "STDERR_BANDS" | "STANDARD_ERROR_BANDS" | "SE_BANDS" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.seb_win_symbol = sym;
                }
                self.show_seb_win = true;
                if self.seb_win_snapshot.symbol.is_empty() && !self.seb_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_seb(&conn, &self.seb_win_symbol)
                            {
                                self.seb_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "IMI" | "IMIFIT" | "IMI_WIN" | "INTRADAY_MOMENTUM_INDEX" | "CHANDE_IMI" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.imi_win_symbol = sym;
                }
                self.show_imi_win = true;
                if self.imi_win_snapshot.symbol.is_empty() && !self.imi_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_imi(&conn, &self.imi_win_symbol)
                            {
                                self.imi_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "GMMA" | "GMMAFIT" | "GMMA_WIN" | "GUPPY" | "GUPPY_MMA" | "GUPPY_MULTIPLE_MA" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.gmma_win_symbol = sym;
                }
                self.show_gmma_win = true;
                if self.gmma_win_snapshot.symbol.is_empty() && !self.gmma_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_gmma(
                                &conn,
                                &self.gmma_win_symbol,
                            ) {
                                self.gmma_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MAENV"
            | "MAENVFIT"
            | "MAENV_WIN"
            | "MA_ENVELOPE"
            | "MOVING_AVG_ENVELOPE"
            | "MA_ENV" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.maenv_win_symbol = sym;
                }
                self.show_maenv_win = true;
                if self.maenv_win_snapshot.symbol.is_empty() && !self.maenv_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_maenv(
                                &conn,
                                &self.maenv_win_symbol,
                            ) {
                                self.maenv_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ADL"
            | "ADLFIT"
            | "ADL_WIN"
            | "ACCUM_DIST"
            | "ACCUMULATION_DISTRIBUTION"
            | "CHAIKIN_ADL"
            | "AD_LINE" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.adl_win_symbol = sym;
                }
                self.show_adl_win = true;
                if self.adl_win_snapshot.symbol.is_empty() && !self.adl_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_adl(&conn, &self.adl_win_symbol)
                            {
                                self.adl_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VHF"
            | "VHFFIT"
            | "VHF_WIN"
            | "VERTHORZ"
            | "VERT_HORZ_FILTER"
            | "VERTICAL_HORIZONTAL_FILTER" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.vhf_win_symbol = sym;
                }
                self.show_vhf_win = true;
                if self.vhf_win_snapshot.symbol.is_empty() && !self.vhf_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_vhf(&conn, &self.vhf_win_symbol)
                            {
                                self.vhf_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VROC"
            | "VROCFIT"
            | "VROC_WIN"
            | "VOLUME_ROC"
            | "VOL_ROC"
            | "VOLUME_RATE_OF_CHANGE" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.vroc_win_symbol = sym;
                }
                self.show_vroc_win = true;
                if self.vroc_win_snapshot.symbol.is_empty() && !self.vroc_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_vroc(
                                &conn,
                                &self.vroc_win_symbol,
                            ) {
                                self.vroc_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "KDJ" | "KDJFIT" | "KDJ_WIN" | "K_D_J" | "KDJ_STOCH" | "STOCH_KDJ" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.kdj_win_symbol = sym;
                }
                self.show_kdj_win = true;
                if self.kdj_win_snapshot.symbol.is_empty() && !self.kdj_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_kdj(&conn, &self.kdj_win_symbol)
                            {
                                self.kdj_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "QQE"
            | "QQEFIT"
            | "QQE_WIN"
            | "QQE_MOD"
            | "QUANT_QUAL_EST"
            | "QUANTITATIVE_QUALITATIVE" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.qqe_win_symbol = sym;
                }
                self.show_qqe_win = true;
                if self.qqe_win_snapshot.symbol.is_empty() && !self.qqe_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_qqe(&conn, &self.qqe_win_symbol)
                            {
                                self.qqe_win_snapshot = snap;
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
