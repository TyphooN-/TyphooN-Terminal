use super::*;

impl TyphooNApp {
    pub(super) fn handle_momentum_flow_commands(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
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
            _ => return false,
        }
        true
    }
}
