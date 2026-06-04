use super::*;

impl TyphooNApp {
    pub(super) fn handle_research_round55_to68_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // ── Options Expiration Calendar ──
            "EXPCAL"
            | "OPTCAL"
            | "EXPIRY"
            | "EXPIRATIONS"
            | "OPTION_CALENDAR"
            | "OPTIONS_CALENDAR"
            | "OPTION_EXPIRATION_CALENDAR" => {
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
                    self.expcal_win_symbol = sym;
                }
                self.show_expcal_win = true;
                if self.expcal_win_calendar.is_empty() {
                    let today = chrono::Local::now().date_naive();
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
            // ── Round 55 ──
            "SMMA" | "SMMAFIT" | "SMMA_WIN" | "WILDER_MA" | "WILDER_SMMA" | "RMA"
            | "SMOOTHED_MA" => {
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
            "PMO"
            | "PMOFIT"
            | "PMO_WIN"
            | "PRING_PMO"
            | "PRICE_MOMENTUM_OSC"
            | "PRICE_MOMENTUM_OSCILLATOR" => {
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
                    self.pmo_win_symbol = sym;
                }
                self.show_pmo_win = true;
                if self.pmo_win_snapshot.symbol.is_empty() && !self.pmo_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_pmo(&conn, &self.pmo_win_symbol)
                            {
                                self.pmo_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CFO"
            | "CFOFIT"
            | "CFO_WIN"
            | "FORECAST_OSC"
            | "CHANDE_FORECAST"
            | "FORECAST_OSCILLATOR" => {
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
                    self.cfo_win_symbol = sym;
                }
                self.show_cfo_win = true;
                if self.cfo_win_snapshot.symbol.is_empty() && !self.cfo_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cfo(&conn, &self.cfo_win_symbol)
                            {
                                self.cfo_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "TMF" | "TMFFIT" | "TMF_WIN" | "TWIGGS_MF" | "TWIGGS_MONEY_FLOW"
            | "TWIGGSMONEYFLOW" => {
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
                    self.tmf_win_symbol = sym;
                }
                self.show_tmf_win = true;
                if self.tmf_win_snapshot.symbol.is_empty() && !self.tmf_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_tmf(&conn, &self.tmf_win_symbol)
                            {
                                self.tmf_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "FRACTALS_WIN"
            | "FRACTAL_WIN"
            | "FRACTALS_RESEARCH"
            | "BILL_WILLIAMS_FRACTALS"
            | "BW_FRACTALS" => {
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
                    self.fractals_win_symbol = sym;
                }
                self.show_fractals_win = true;
                if self.fractals_win_snapshot.symbol.is_empty()
                    && !self.fractals_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_fractals(
                                &conn,
                                &self.fractals_win_symbol,
                            ) {
                                self.fractals_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "IFT_RSI" | "IFTRSI" | "INVERSE_FISHER_RSI" | "EHLERS_IFT_RSI" | "INVFISHER_RSI" => {
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
                    self.ift_rsi_win_symbol = sym;
                }
                self.show_ift_rsi_win = true;
                if self.ift_rsi_win_snapshot.symbol.is_empty()
                    && !self.ift_rsi_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ift_rsi(
                                &conn,
                                &self.ift_rsi_win_symbol,
                            ) {
                                self.ift_rsi_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MAMA" | "MAMA_WIN" | "MESA_ADAPTIVE_MA" | "MESA_AMA" | "EHLERS_MAMA" => {
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
                    self.mama_win_symbol = sym;
                }
                self.show_mama_win = true;
                if self.mama_win_snapshot.symbol.is_empty() && !self.mama_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_mama(
                                &conn,
                                &self.mama_win_symbol,
                            ) {
                                self.mama_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "COG" | "COG_WIN" | "CENTER_OF_GRAVITY" | "EHLERS_COG" | "COG_OSC" => {
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
                    self.cog_win_symbol = sym;
                }
                self.show_cog_win = true;
                if self.cog_win_snapshot.symbol.is_empty() && !self.cog_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cog(&conn, &self.cog_win_symbol)
                            {
                                self.cog_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DIDI" | "DIDI_INDEX" | "DIDI_NEEDLES" | "AGUIAR_DIDI" | "DIDI_WIN" => {
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
                    self.didi_win_symbol = sym;
                }
                self.show_didi_win = true;
                if self.didi_win_snapshot.symbol.is_empty() && !self.didi_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_didi(
                                &conn,
                                &self.didi_win_symbol,
                            ) {
                                self.didi_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DEMARKER" | "DEM" | "DEMARK" | "DEMARKER_WIN" | "DEMARKER_RESEARCH" => {
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
                    self.demarker_win_symbol = sym;
                }
                self.show_demarker_win = true;
                if self.demarker_win_snapshot.symbol.is_empty()
                    && !self.demarker_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_demarker(
                                &conn,
                                &self.demarker_win_symbol,
                            ) {
                                self.demarker_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "GATOR_OSC" | "GATOR_OSCILLATOR" | "GATOR_WIN" | "BW_GATOR" | "BILL_WILLIAMS_GATOR" => {
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
                    self.gator_win_symbol = sym;
                }
                self.show_gator_win = true;
                if self.gator_win_snapshot.symbol.is_empty() && !self.gator_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_gator(
                                &conn,
                                &self.gator_win_symbol,
                            ) {
                                self.gator_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "BW_MFI"
            | "BWMFI"
            | "MARKET_FACILITATION_INDEX"
            | "BILL_WILLIAMS_MFI"
            | "BWMFI_WIN" => {
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
                    self.bw_mfi_win_symbol = sym;
                }
                self.show_bw_mfi_win = true;
                if self.bw_mfi_win_snapshot.symbol.is_empty() && !self.bw_mfi_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_bw_mfi(
                                &conn,
                                &self.bw_mfi_win_symbol,
                            ) {
                                self.bw_mfi_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VWMA" | "VWMA_WIN" | "VOL_WEIGHTED_MA" | "VOLUME_WEIGHTED_MA" | "VWMA_RESEARCH" => {
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
                    self.vwma_win_symbol = sym;
                }
                self.show_vwma_win = true;
                if self.vwma_win_snapshot.symbol.is_empty() && !self.vwma_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_vwma(
                                &conn,
                                &self.vwma_win_symbol,
                            ) {
                                self.vwma_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "STDDEV" | "STD_DEV" | "STANDARD_DEVIATION" | "ROLLING_STDDEV" | "STDDEV_WIN" => {
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
                    self.stddev_win_symbol = sym;
                }
                self.show_stddev_win = true;
                if self.stddev_win_snapshot.symbol.is_empty() && !self.stddev_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_stddev(
                                &conn,
                                &self.stddev_win_symbol,
                            ) {
                                self.stddev_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Round 60: WMA / RAINBOW / MESA_SINE / FRAMA / IBS ──
            "WMA" | "WEIGHTED_MA" | "WMA_WIN" | "LINEAR_WEIGHTED_MA" => {
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
                    self.wma_win_symbol = sym;
                }
                self.show_wma_win = true;
                if self.wma_win_snapshot.symbol.is_empty() && !self.wma_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_wma(&conn, &self.wma_win_symbol)
                            {
                                self.wma_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "RAINBOW" | "RAINBOW_MA" | "RAINBOW_OSC" | "RAINBOW_WIN" | "WIDNER_RAINBOW" => {
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
                    self.rainbow_win_symbol = sym;
                }
                self.show_rainbow_win = true;
                if self.rainbow_win_snapshot.symbol.is_empty()
                    && !self.rainbow_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_rainbow(
                                &conn,
                                &self.rainbow_win_symbol,
                            ) {
                                self.rainbow_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MESA_SINE" | "MESASINE" | "MESA_SINEWAVE" | "SINE_WAVE" | "EHLERS_SINE" => {
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
                    self.mesa_sine_win_symbol = sym;
                }
                self.show_mesa_sine_win = true;
                if self.mesa_sine_win_snapshot.symbol.is_empty()
                    && !self.mesa_sine_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_mesa_sine(
                                &conn,
                                &self.mesa_sine_win_symbol,
                            ) {
                                self.mesa_sine_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "FRAMA" | "FRACTAL_ADAPTIVE_MA" | "FRAMA_WIN" | "EHLERS_FRAMA" => {
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
                    self.frama_win_symbol = sym;
                }
                self.show_frama_win = true;
                if self.frama_win_snapshot.symbol.is_empty() && !self.frama_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_frama(
                                &conn,
                                &self.frama_win_symbol,
                            ) {
                                self.frama_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "IBS" | "INTERNAL_BAR_STRENGTH" | "IBS_WIN" | "BAR_STRENGTH" => {
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
                    self.ibs_win_symbol = sym;
                }
                self.show_ibs_win = true;
                if self.ibs_win_snapshot.symbol.is_empty() && !self.ibs_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_ibs(&conn, &self.ibs_win_symbol)
                            {
                                self.ibs_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "LAGUERRE_RSI" | "LAGUERRERSI" | "LRSI" | "LAGUERRE_RSI_WIN" | "EHLERS_LAGUERRE" => {
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
                    self.laguerre_rsi_win_symbol = sym;
                }
                self.show_laguerre_rsi_win = true;
                if self.laguerre_rsi_win_snapshot.symbol.is_empty()
                    && !self.laguerre_rsi_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_laguerre_rsi(
                                &conn,
                                &self.laguerre_rsi_win_symbol,
                            ) {
                                self.laguerre_rsi_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ZIGZAG" | "ZIG_ZAG" | "ZIGZAG_WIN" | "ZZ" | "PIVOT_REVERSAL" => {
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
                    self.zigzag_win_symbol = sym;
                }
                self.show_zigzag_win = true;
                if self.zigzag_win_snapshot.symbol.is_empty() && !self.zigzag_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_zigzag(
                                &conn,
                                &self.zigzag_win_symbol,
                            ) {
                                self.zigzag_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PGO" | "PRETTY_GOOD_OSC" | "PRETTY_GOOD_OSCILLATOR" | "PGO_WIN" | "JOHNSON_PGO" => {
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
                    self.pgo_win_symbol = sym;
                }
                self.show_pgo_win = true;
                if self.pgo_win_snapshot.symbol.is_empty() && !self.pgo_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_pgo(&conn, &self.pgo_win_symbol)
                            {
                                self.pgo_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HT_TRENDLINE" | "HTTRENDLINE" | "HT_TREND" | "HT_TRENDLINE_WIN"
            | "HILBERT_TRENDLINE" => {
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
                    self.ht_trendline_win_symbol = sym;
                }
                self.show_ht_trendline_win = true;
                if self.ht_trendline_win_snapshot.symbol.is_empty()
                    && !self.ht_trendline_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_trendline(
                                &conn,
                                &self.ht_trendline_win_symbol,
                            ) {
                                self.ht_trendline_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MIDPOINT" | "MIDPOINT_WIN" | "HL_MIDPOINT" | "MIDPOINT_N" | "MIDPT" => {
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
                    self.midpoint_win_symbol = sym;
                }
                self.show_midpoint_win = true;
                if self.midpoint_win_snapshot.symbol.is_empty()
                    && !self.midpoint_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_midpoint(
                                &conn,
                                &self.midpoint_win_symbol,
                            ) {
                                self.midpoint_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Round 62 palette aliases ──
            // Note: "MASS_INDEX"/"DORSEY_MASS" already claimed by Round 47 curvefit.
            "MASSINDEX" | "MI" | "MASS_INDEX_WIN" | "MINDEX" | "MASS_25" => {
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
                    self.mass_index_win_symbol = sym;
                }
                self.show_mass_index_win = true;
                if self.mass_index_win_snapshot.symbol.is_empty()
                    && !self.mass_index_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_mass_index(
                                &conn,
                                &self.mass_index_win_symbol,
                            ) {
                                self.mass_index_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "NATR" | "NORMALIZED_ATR" | "NATR_WIN" | "NORMALIZED_ATR_WIN" | "ATR_PCT" => {
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
                    self.natr_win_symbol = sym;
                }
                self.show_natr_win = true;
                if self.natr_win_snapshot.symbol.is_empty() && !self.natr_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_natr(
                                &conn,
                                &self.natr_win_symbol,
                            ) {
                                self.natr_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // Note: bare "SQUEEZE" is a chart toggle, not claimed here.
            "TTM_SQUEEZE" | "TTMSQUEEZE" | "TTM_SQUEEZE_WIN" | "CARTER_SQUEEZE" | "TTM" => {
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
                    self.ttm_squeeze_win_symbol = sym;
                }
                self.show_ttm_squeeze_win = true;
                if self.ttm_squeeze_win_snapshot.symbol.is_empty()
                    && !self.ttm_squeeze_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ttm_squeeze(
                                &conn,
                                &self.ttm_squeeze_win_symbol,
                            ) {
                                self.ttm_squeeze_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // Note: "FORCE_INDEX"/"ELDER_FORCE" already claimed by Round 48 EFI curvefit.
            "FORCEINDEX" | "FORCE" | "FI" | "FORCE_INDEX_WIN" | "FORCE13" => {
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
                    self.force_index_win_symbol = sym;
                }
                self.show_force_index_win = true;
                if self.force_index_win_snapshot.symbol.is_empty()
                    && !self.force_index_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_force_index(
                                &conn,
                                &self.force_index_win_symbol,
                            ) {
                                self.force_index_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "TRANGE" | "TRUE_RANGE" | "TR" | "TRANGE_WIN" | "RAW_TRUE_RANGE" => {
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
                    self.trange_win_symbol = sym;
                }
                self.show_trange_win = true;
                if self.trange_win_snapshot.symbol.is_empty() && !self.trange_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_trange(
                                &conn,
                                &self.trange_win_symbol,
                            ) {
                                self.trange_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Round 63 palette aliases ──
            "LINEARREG_SLOPE" | "LINREG_SLOPE" | "LINREGSLOPE" | "LRSLOPE" | "SLOPE" => {
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
                    self.linearreg_slope_win_symbol = sym;
                }
                self.show_linearreg_slope_win = true;
                if self.linearreg_slope_win_snapshot.symbol.is_empty()
                    && !self.linearreg_slope_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_linearreg_slope(
                                    &conn,
                                    &self.linearreg_slope_win_symbol,
                                )
                            {
                                self.linearreg_slope_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HT_DCPERIOD" | "HTDCPERIOD" | "DCPERIOD" | "HILBERT_PERIOD" | "CYCLE_PERIOD" => {
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
                    self.ht_dcperiod_win_symbol = sym;
                }
                self.show_ht_dcperiod_win = true;
                if self.ht_dcperiod_win_snapshot.symbol.is_empty()
                    && !self.ht_dcperiod_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_dcperiod(
                                &conn,
                                &self.ht_dcperiod_win_symbol,
                            ) {
                                self.ht_dcperiod_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HT_TRENDMODE" | "HTTRENDMODE" | "TRENDMODE" | "HILBERT_TRENDMODE"
            | "CYCLE_TRENDMODE" => {
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
                    self.ht_trendmode_win_symbol = sym;
                }
                self.show_ht_trendmode_win = true;
                if self.ht_trendmode_win_snapshot.symbol.is_empty()
                    && !self.ht_trendmode_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_trendmode(
                                &conn,
                                &self.ht_trendmode_win_symbol,
                            ) {
                                self.ht_trendmode_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ACCBANDS" | "ACCELERATION_BANDS" | "ACCBAND" | "HEADLEY" | "ACC_BANDS" => {
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
                    self.accbands_win_symbol = sym;
                }
                self.show_accbands_win = true;
                if self.accbands_win_snapshot.symbol.is_empty()
                    && !self.accbands_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_accbands(
                                &conn,
                                &self.accbands_win_symbol,
                            ) {
                                self.accbands_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "STOCHF" | "STOCHFAST" | "FAST_STOCH" | "FASTSTOCH" | "STOCH_FAST" => {
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
                    self.stochf_win_symbol = sym;
                }
                self.show_stochf_win = true;
                if self.stochf_win_snapshot.symbol.is_empty() && !self.stochf_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_stochf(
                                &conn,
                                &self.stochf_win_symbol,
                            ) {
                                self.stochf_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Round 64 palette aliases ──
            "LINEARREG" | "LINEARREG_FIT" | "LINEAR_REG" | "LINEARREG_WIN" | "LINREG_FITTED" => {
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
                    self.linearreg_win_symbol = sym;
                }
                self.show_linearreg_win = true;
                if self.linearreg_win_snapshot.symbol.is_empty()
                    && !self.linearreg_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_linearreg(
                                &conn,
                                &self.linearreg_win_symbol,
                            ) {
                                self.linearreg_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "LINEARREG_ANGLE" | "LREGANGLE" | "LINEAR_REG_ANGLE" | "LINREGANGLE" | "LRANGLE" => {
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
                    self.linearreg_angle_win_symbol = sym;
                }
                self.show_linearreg_angle_win = true;
                if self.linearreg_angle_win_snapshot.symbol.is_empty()
                    && !self.linearreg_angle_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_linearreg_angle(
                                    &conn,
                                    &self.linearreg_angle_win_symbol,
                                )
                            {
                                self.linearreg_angle_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HT_DCPHASE" | "DCPHASE" | "HILBERT_DCPHASE" | "HTDCPHASE" | "CYCLE_PHASE" => {
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
                    self.ht_dcphase_win_symbol = sym;
                }
                self.show_ht_dcphase_win = true;
                if self.ht_dcphase_win_snapshot.symbol.is_empty()
                    && !self.ht_dcphase_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_dcphase(
                                &conn,
                                &self.ht_dcphase_win_symbol,
                            ) {
                                self.ht_dcphase_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HT_SINE" | "HTSINE" | "HILBERT_SINE" | "SINEWAVE" | "LEADSINE" => {
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
                    self.ht_sine_win_symbol = sym;
                }
                self.show_ht_sine_win = true;
                if self.ht_sine_win_snapshot.symbol.is_empty()
                    && !self.ht_sine_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_sine(
                                &conn,
                                &self.ht_sine_win_symbol,
                            ) {
                                self.ht_sine_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HT_PHASOR" | "HTPHASOR" | "HILBERT_PHASOR" | "PHASOR" | "IQ_COMP" => {
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
                    self.ht_phasor_win_symbol = sym;
                }
                self.show_ht_phasor_win = true;
                if self.ht_phasor_win_snapshot.symbol.is_empty()
                    && !self.ht_phasor_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_ht_phasor(
                                &conn,
                                &self.ht_phasor_win_symbol,
                            ) {
                                self.ht_phasor_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Round 65 palette aliases ──
            "MIDPRICE" | "MID_PRICE" | "MIDBAR" | "MIDBARPRICE" | "HLMIDPRICE" => {
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
                    self.midprice_win_symbol = sym;
                }
                self.show_midprice_win = true;
                if self.midprice_win_snapshot.symbol.is_empty()
                    && !self.midprice_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_midprice(
                                &conn,
                                &self.midprice_win_symbol,
                            ) {
                                self.midprice_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "APO" | "ABS_PRICE_OSC" | "ABSPRICEOSC" | "ABSPO" | "APOWIN" => {
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
                    self.apo_win_symbol = sym;
                }
                self.show_apo_win = true;
                if self.apo_win_snapshot.symbol.is_empty() && !self.apo_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_apo(&conn, &self.apo_win_symbol)
                            {
                                self.apo_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MOMRAW" | "MOMENTUM_RAW" | "MOM_TA" | "RAWMOM" | "TALIB_MOM" => {
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
                    self.mom_win_symbol = sym;
                }
                self.show_mom_win = true;
                if self.mom_win_snapshot.symbol.is_empty() && !self.mom_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_mom(&conn, &self.mom_win_symbol)
                            {
                                self.mom_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SAREXT" | "SAR_EXT" | "EXTENDED_SAR" | "SAREXTENDED" | "PSAR_EXT" => {
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
                    self.sarext_win_symbol = sym;
                }
                self.show_sarext_win = true;
                if self.sarext_win_snapshot.symbol.is_empty() && !self.sarext_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_sarext(
                                &conn,
                                &self.sarext_win_symbol,
                            ) {
                                self.sarext_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ADXR" | "ADX_RATING" | "ADX_R" | "ADXRATING" | "ADX_RANK" => {
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
                    self.adxr_win_symbol = sym;
                }
                self.show_adxr_win = true;
                if self.adxr_win_snapshot.symbol.is_empty() && !self.adxr_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_adxr(
                                &conn,
                                &self.adxr_win_symbol,
                            ) {
                                self.adxr_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "AVGPRICE" | "AVG_PRICE" | "OHLC_AVG" | "OHLCAVG" | "AVGOHLC" => {
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
                    self.avgprice_win_symbol = sym;
                }
                self.show_avgprice_win = true;
                if self.avgprice_win_snapshot.symbol.is_empty()
                    && !self.avgprice_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_avgprice(
                                &conn,
                                &self.avgprice_win_symbol,
                            ) {
                                self.avgprice_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MEDPRICE" | "MED_PRICE" | "HLMED" | "HLMEDIAN" | "RANGEMEDIAN" => {
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
                    self.medprice_win_symbol = sym;
                }
                self.show_medprice_win = true;
                if self.medprice_win_snapshot.symbol.is_empty()
                    && !self.medprice_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_medprice(
                                &conn,
                                &self.medprice_win_symbol,
                            ) {
                                self.medprice_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "TYPPRICE" | "TYP_PRICE" | "TYPICAL_PRICE" | "TYPICALPRICE" | "HLC3" => {
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
                    self.typprice_win_symbol = sym;
                }
                self.show_typprice_win = true;
                if self.typprice_win_snapshot.symbol.is_empty()
                    && !self.typprice_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_typprice(
                                &conn,
                                &self.typprice_win_symbol,
                            ) {
                                self.typprice_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "WCLPRICE" | "WCL_PRICE" | "WEIGHTED_CLOSE" | "WEIGHTEDCLOSE" | "HLCC4" => {
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
                    self.wclprice_win_symbol = sym;
                }
                self.show_wclprice_win = true;
                if self.wclprice_win_snapshot.symbol.is_empty()
                    && !self.wclprice_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_wclprice(
                                &conn,
                                &self.wclprice_win_symbol,
                            ) {
                                self.wclprice_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "VARIANCE" | "VARIANCE_WIN" | "CLOSE_VARIANCE" | "CVARIANCE" | "VARWIN" => {
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
                    self.variance_win_symbol = sym;
                }
                self.show_variance_win = true;
                if self.variance_win_snapshot.symbol.is_empty()
                    && !self.variance_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_variance(
                                &conn,
                                &self.variance_win_symbol,
                            ) {
                                self.variance_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Round 67: DMI family ──
            "PLUS_DI" | "PDI" | "DI_PLUS" | "DIPOS" | "WILDER_PDI" => {
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
                    self.plus_di_win_symbol = sym;
                }
                self.show_plus_di_win = true;
                if self.plus_di_win_snapshot.symbol.is_empty()
                    && !self.plus_di_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_plus_di(
                                &conn,
                                &self.plus_di_win_symbol,
                            ) {
                                self.plus_di_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MINUS_DI" | "MDI" | "DI_MINUS" | "DINEG" | "WILDER_MDI" => {
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
                    self.minus_di_win_symbol = sym;
                }
                self.show_minus_di_win = true;
                if self.minus_di_win_snapshot.symbol.is_empty()
                    && !self.minus_di_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_minus_di(
                                &conn,
                                &self.minus_di_win_symbol,
                            ) {
                                self.minus_di_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "PLUS_DM" | "PDM" | "DM_PLUS" | "DMPOS" | "WILDER_PDM" => {
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
                    self.plus_dm_win_symbol = sym;
                }
                self.show_plus_dm_win = true;
                if self.plus_dm_win_snapshot.symbol.is_empty()
                    && !self.plus_dm_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_plus_dm(
                                &conn,
                                &self.plus_dm_win_symbol,
                            ) {
                                self.plus_dm_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MINUS_DM" | "MDM" | "DM_MINUS" | "DMNEG" | "WILDER_MDM" => {
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
                    self.minus_dm_win_symbol = sym;
                }
                self.show_minus_dm_win = true;
                if self.minus_dm_win_snapshot.symbol.is_empty()
                    && !self.minus_dm_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_minus_dm(
                                &conn,
                                &self.minus_dm_win_symbol,
                            ) {
                                self.minus_dm_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DX" | "DX_WILDER" | "DXWIN" | "DIRIDX" | "WILDER_DX" => {
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
                    self.dx_win_symbol = sym;
                }
                self.show_dx_win = true;
                if self.dx_win_snapshot.symbol.is_empty() && !self.dx_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_dx(&conn, &self.dx_win_symbol)
                            {
                                self.dx_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Round 68 ──
            "ROC" | "ROC_WILDER" | "ROCWIN" | "ROCRATE" | "RATE_OF_CHANGE" => {
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
                    self.roc_win_symbol = sym;
                }
                self.show_roc_win = true;
                if self.roc_win_snapshot.symbol.is_empty() && !self.roc_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_roc(&conn, &self.roc_win_symbol)
                            {
                                self.roc_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ROCP" | "ROCP_WILDER" | "ROCPWIN" | "ROCPCT" | "ROC_PCT" => {
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
                    self.rocp_win_symbol = sym;
                }
                self.show_rocp_win = true;
                if self.rocp_win_snapshot.symbol.is_empty() && !self.rocp_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_rocp(
                                &conn,
                                &self.rocp_win_symbol,
                            ) {
                                self.rocp_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ROCR" | "ROCR_WILDER" | "ROCRWIN" | "ROCRATIO" | "ROC_RATIO" => {
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
                    self.rocr_win_symbol = sym;
                }
                self.show_rocr_win = true;
                if self.rocr_win_snapshot.symbol.is_empty() && !self.rocr_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_rocr(
                                &conn,
                                &self.rocr_win_symbol,
                            ) {
                                self.rocr_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ROCR100" | "ROCR100_WILDER" | "ROCR100WIN" | "ROCR100IDX" | "ROC_RATIO_100" => {
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
                    self.rocr100_win_symbol = sym;
                }
                self.show_rocr100_win = true;
                if self.rocr100_win_snapshot.symbol.is_empty()
                    && !self.rocr100_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_rocr100(
                                &conn,
                                &self.rocr100_win_symbol,
                            ) {
                                self.rocr100_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CORREL" | "CORRWIN" | "ROLLCORR" | "AUTOCORR" | "PEARSON_AUTO" => {
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
                    self.correl_win_symbol = sym;
                }
                self.show_correl_win = true;
                if self.correl_win_snapshot.symbol.is_empty() && !self.correl_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_correl(
                                &conn,
                                &self.correl_win_symbol,
                            ) {
                                self.correl_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MIN" | "MINWIN" | "MIN_CLOSE" | "LOW_BAND" | "ROLL_MIN" => {
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
                    self.min_win_symbol = sym;
                }
                self.show_min_win = true;
                if self.min_win_snapshot.symbol.is_empty() && !self.min_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_min(&conn, &self.min_win_symbol)
                            {
                                self.min_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MAX" | "MAXWIN" | "MAX_CLOSE" | "HIGH_BAND" | "ROLL_MAX" => {
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
                    self.max_win_symbol = sym;
                }
                self.show_max_win = true;
                if self.max_win_snapshot.symbol.is_empty() && !self.max_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_max(&conn, &self.max_win_symbol)
                            {
                                self.max_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MINMAX" | "MINMAXWIN" | "RANGE_BAND" | "HL_RANGE" | "EXTREMA" => {
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
                    self.minmax_win_symbol = sym;
                }
                self.show_minmax_win = true;
                if self.minmax_win_snapshot.symbol.is_empty() && !self.minmax_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_minmax(
                                &conn,
                                &self.minmax_win_symbol,
                            ) {
                                self.minmax_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MININDEX" | "MINIDXWIN" | "LOW_IDX" | "MIN_AGE" | "LOW_RECENCY" => {
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
                    self.minindex_win_symbol = sym;
                }
                self.show_minindex_win = true;
                if self.minindex_win_snapshot.symbol.is_empty()
                    && !self.minindex_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_minindex(
                                &conn,
                                &self.minindex_win_symbol,
                            ) {
                                self.minindex_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "MAXINDEX" | "MAXIDXWIN" | "HIGH_IDX" | "MAX_AGE" | "HIGH_RECENCY" => {
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
                    self.maxindex_win_symbol = sym;
                }
                self.show_maxindex_win = true;
                if self.maxindex_win_snapshot.symbol.is_empty()
                    && !self.maxindex_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_maxindex(
                                &conn,
                                &self.maxindex_win_symbol,
                            ) {
                                self.maxindex_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "BBANDS" | "BBANDSWIN" | "BB_BANDS" | "BBAND" | "BOLL_BANDS" => {
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
                    self.bbands_win_symbol = sym;
                }
                self.show_bbands_win = true;
                if self.bbands_win_snapshot.symbol.is_empty() && !self.bbands_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_bbands(
                                &conn,
                                &self.bbands_win_symbol,
                            ) {
                                self.bbands_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "AD" | "AD_LINE_TALIB" | "AD_CHAIKIN" | "ADWIN" | "TALIB_AD" => {
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
                    self.ad_win_symbol = sym;
                }
                self.show_ad_win = true;
                if self.ad_win_snapshot.symbol.is_empty() && !self.ad_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_ad(&conn, &self.ad_win_symbol)
                            {
                                self.ad_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "ADOSC" | "ADOSCWIN" | "TALIB_ADOSC" | "AD_OSCILLATOR" | "CHAIKIN_ADO" => {
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
                    self.adosc_win_symbol = sym;
                }
                self.show_adosc_win = true;
                if self.adosc_win_snapshot.symbol.is_empty() && !self.adosc_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_adosc(
                                &conn,
                                &self.adosc_win_symbol,
                            ) {
                                self.adosc_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "SUM" | "SUMWIN" | "ROLLSUM" | "CLOSE_SUM" | "SUM_CLOSE" => {
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
                    self.sum_win_symbol = sym;
                }
                self.show_sum_win = true;
                if self.sum_win_snapshot.symbol.is_empty() && !self.sum_win_symbol.is_empty() {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_sum(&conn, &self.sum_win_symbol)
                            {
                                self.sum_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "LINEARREG_INTERCEPT"
            | "LINREG_INTERCEPT"
            | "LINTERCEPT"
            | "LRINTERCEPT"
            | "REG_INTERCEPT"
            | "LINEARREG_B" => {
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
                    self.linreg_intercept_win_symbol = sym;
                }
                self.show_linreg_intercept_win = true;
                if self.linreg_intercept_win_snapshot.symbol.is_empty()
                    && !self.linreg_intercept_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_linreg_intercept(
                                    &conn,
                                    &self.linreg_intercept_win_symbol,
                                )
                            {
                                self.linreg_intercept_win_snapshot = snap;
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
