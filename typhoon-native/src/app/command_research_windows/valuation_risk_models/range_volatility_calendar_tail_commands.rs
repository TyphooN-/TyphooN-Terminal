use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_range_volatility_calendar_tail_command(
        &mut self,
        cmd_upper: &String,
    ) -> bool {
        match cmd_upper.as_str() {
            // ── Range volatility, expected shortfall, and calendar palette aliases ──
            "PARKINSON" | "PARKINSON_VOL" | "PARKVOL" | "HL_VOL" | "RANGE_VOL" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "GKVOL" | "GARMAN_KLASS" | "GARMANKLASS" | "GK_VOL" | "GARMAN_KLASS_VOL" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "RSVOL" | "ROGERS_SATCHELL" | "ROGERSSATCHELL" | "RS_VOL" | "DRIFT_FREE_VOL" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "CVAR" | "EXPECTED_SHORTFALL" | "ES" | "CONDITIONAL_VAR" | "ES5" | "ES_5"
            | "TAIL_EXPECTED" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            "DOWEFFECT" | "DOW_EFFECT" | "DOW" | "WEEKDAY_EFFECT" | "DAY_OF_WEEK" | "DAYOFWEEK" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
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
                true
            }
            _ => false,
        }
    }
}
