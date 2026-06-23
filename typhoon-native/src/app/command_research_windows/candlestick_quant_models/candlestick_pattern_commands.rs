use super::*;

impl TyphooNApp {
    pub(super) fn handle_candlestick_pattern_command(&mut self, cmd_upper: &String) -> bool {
        match cmd_upper.as_str() {
            // Candlestick pattern storage/helpers
            "CDLDOJI" | "CDLDOJIWIN" | "DOJI" | "DOJI_PATTERN" | "DOJI_CANDLE" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_doji_win_symbol = sym;
                }
                self.show_cdl_doji_win = true;
                if self.cdl_doji_win_snapshot.symbol.is_empty()
                    && !self.cdl_doji_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_doji(
                                &conn,
                                &self.cdl_doji_win_symbol,
                            ) {
                                self.cdl_doji_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLHAMMER" | "CDLHAMMERWIN" | "HAMMER" | "HAMMER_PATTERN" | "HAMMER_CANDLE" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_hammer_win_symbol = sym;
                }
                self.show_cdl_hammer_win = true;
                if self.cdl_hammer_win_snapshot.symbol.is_empty()
                    && !self.cdl_hammer_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_hammer(
                                &conn,
                                &self.cdl_hammer_win_symbol,
                            ) {
                                self.cdl_hammer_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLSHOOTINGSTAR"
            | "SHOOTINGSTAR"
            | "SHOOTING_STAR"
            | "CDLSHOOTINGSTARWIN"
            | "SHOOTING_STAR_PATTERN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_shooting_star_win_symbol = sym;
                }
                self.show_cdl_shooting_star_win = true;
                if self.cdl_shooting_star_win_snapshot.symbol.is_empty()
                    && !self.cdl_shooting_star_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_shooting_star(
                                    &conn,
                                    &self.cdl_shooting_star_win_symbol,
                                )
                            {
                                self.cdl_shooting_star_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLENGULFING" | "ENGULFING" | "CDLENGULFINGWIN" | "ENGULFING_PATTERN"
            | "ENGULFING_CANDLE" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_engulfing_win_symbol = sym;
                }
                self.show_cdl_engulfing_win = true;
                if self.cdl_engulfing_win_snapshot.symbol.is_empty()
                    && !self.cdl_engulfing_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_engulfing(
                                    &conn,
                                    &self.cdl_engulfing_win_symbol,
                                )
                            {
                                self.cdl_engulfing_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLHARAMI" | "HARAMI" | "CDLHARAMIWIN" | "HARAMI_PATTERN" | "INSIDE_BAR" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_harami_win_symbol = sym;
                }
                self.show_cdl_harami_win = true;
                if self.cdl_harami_win_snapshot.symbol.is_empty()
                    && !self.cdl_harami_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_harami(
                                &conn,
                                &self.cdl_harami_win_symbol,
                            ) {
                                self.cdl_harami_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── CDL* 3-bar / 2-bar patterns ──
            "CDLMORNINGSTAR"
            | "MORNINGSTAR"
            | "MORNING_STAR"
            | "CDLMORNINGSTARWIN"
            | "MORNING_STAR_PATTERN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_morning_star_win_symbol = sym;
                }
                self.show_cdl_morning_star_win = true;
                if self.cdl_morning_star_win_snapshot.symbol.is_empty()
                    && !self.cdl_morning_star_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_morning_star(
                                    &conn,
                                    &self.cdl_morning_star_win_symbol,
                                )
                            {
                                self.cdl_morning_star_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLEVENINGSTAR"
            | "EVENINGSTAR"
            | "EVENING_STAR"
            | "CDLEVENINGSTARWIN"
            | "EVENING_STAR_PATTERN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_evening_star_win_symbol = sym;
                }
                self.show_cdl_evening_star_win = true;
                if self.cdl_evening_star_win_snapshot.symbol.is_empty()
                    && !self.cdl_evening_star_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_evening_star(
                                    &conn,
                                    &self.cdl_evening_star_win_symbol,
                                )
                            {
                                self.cdl_evening_star_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDL3BLACKCROWS"
            | "THREEBLACKCROWS"
            | "THREE_BLACK_CROWS"
            | "BLACK_CROWS"
            | "CDLTHREEBLACKCROWSWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_three_black_crows_win_symbol = sym;
                }
                self.show_cdl_three_black_crows_win = true;
                if self.cdl_three_black_crows_win_snapshot.symbol.is_empty()
                    && !self.cdl_three_black_crows_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_three_black_crows(
                                    &conn,
                                    &self.cdl_three_black_crows_win_symbol,
                                )
                            {
                                self.cdl_three_black_crows_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDL3WHITESOLDIERS"
            | "THREEWHITESOLDIERS"
            | "THREE_WHITE_SOLDIERS"
            | "WHITE_SOLDIERS"
            | "CDLTHREEWHITESOLDIERSWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_three_white_soldiers_win_symbol = sym;
                }
                self.show_cdl_three_white_soldiers_win = true;
                if self.cdl_three_white_soldiers_win_snapshot.symbol.is_empty()
                    && !self.cdl_three_white_soldiers_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_three_white_soldiers(
                                    &conn,
                                    &self.cdl_three_white_soldiers_win_symbol,
                                )
                            {
                                self.cdl_three_white_soldiers_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLDARKCLOUDCOVER"
            | "DARKCLOUDCOVER"
            | "DARK_CLOUD_COVER"
            | "DARK_CLOUD"
            | "CDLDARKCLOUDCOVERWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_dark_cloud_cover_win_symbol = sym;
                }
                self.show_cdl_dark_cloud_cover_win = true;
                if self.cdl_dark_cloud_cover_win_snapshot.symbol.is_empty()
                    && !self.cdl_dark_cloud_cover_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_dark_cloud_cover(
                                    &conn,
                                    &self.cdl_dark_cloud_cover_win_symbol,
                                )
                            {
                                self.cdl_dark_cloud_cover_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── CDL* piercing / doji variants / hammer mirrors ──
            "CDLPIERCING" | "PIERCING" | "PIERCING_LINE" | "PIERCINGLINE" | "CDLPIERCINGWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_piercing_win_symbol = sym;
                }
                self.show_cdl_piercing_win = true;
                if self.cdl_piercing_win_snapshot.symbol.is_empty()
                    && !self.cdl_piercing_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_piercing(
                                &conn,
                                &self.cdl_piercing_win_symbol,
                            ) {
                                self.cdl_piercing_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLDRAGONFLYDOJI"
            | "DRAGONFLYDOJI"
            | "DRAGONFLY_DOJI"
            | "DRAGONFLY"
            | "CDLDRAGONFLYDOJIWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_dragonfly_doji_win_symbol = sym;
                }
                self.show_cdl_dragonfly_doji_win = true;
                if self.cdl_dragonfly_doji_win_snapshot.symbol.is_empty()
                    && !self.cdl_dragonfly_doji_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_dragonfly_doji(
                                    &conn,
                                    &self.cdl_dragonfly_doji_win_symbol,
                                )
                            {
                                self.cdl_dragonfly_doji_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLGRAVESTONEDOJI"
            | "GRAVESTONEDOJI"
            | "GRAVESTONE_DOJI"
            | "GRAVESTONE"
            | "CDLGRAVESTONEDOJIWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_gravestone_doji_win_symbol = sym;
                }
                self.show_cdl_gravestone_doji_win = true;
                if self.cdl_gravestone_doji_win_snapshot.symbol.is_empty()
                    && !self.cdl_gravestone_doji_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_gravestone_doji(
                                    &conn,
                                    &self.cdl_gravestone_doji_win_symbol,
                                )
                            {
                                self.cdl_gravestone_doji_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLHANGINGMAN" | "HANGINGMAN" | "HANGING_MAN" | "CDLHANGINGMANWIN" | "HANGMAN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_hanging_man_win_symbol = sym;
                }
                self.show_cdl_hanging_man_win = true;
                if self.cdl_hanging_man_win_snapshot.symbol.is_empty()
                    && !self.cdl_hanging_man_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_hanging_man(
                                    &conn,
                                    &self.cdl_hanging_man_win_symbol,
                                )
                            {
                                self.cdl_hanging_man_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLINVERTEDHAMMER"
            | "INVERTEDHAMMER"
            | "INVERTED_HAMMER"
            | "INVHAMMER"
            | "CDLINVERTEDHAMMERWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_inverted_hammer_win_symbol = sym;
                }
                self.show_cdl_inverted_hammer_win = true;
                if self.cdl_inverted_hammer_win_snapshot.symbol.is_empty()
                    && !self.cdl_inverted_hammer_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_inverted_hammer(
                                    &conn,
                                    &self.cdl_inverted_hammer_win_symbol,
                                )
                            {
                                self.cdl_inverted_hammer_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── CDL* harami cross / long-legged doji / marubozu / spinning top / tristar ──
            "CDLHARAMICROSS"
            | "HARAMICROSS"
            | "HARAMI_CROSS"
            | "CDLHARAMICROSSWIN"
            | "HARAMI_CROSS_PATTERN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_harami_cross_win_symbol = sym;
                }
                self.show_cdl_harami_cross_win = true;
                if self.cdl_harami_cross_win_snapshot.symbol.is_empty()
                    && !self.cdl_harami_cross_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_harami_cross(
                                    &conn,
                                    &self.cdl_harami_cross_win_symbol,
                                )
                            {
                                self.cdl_harami_cross_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLLONGLEGGEDDOJI"
            | "LONGLEGGEDDOJI"
            | "LONG_LEGGED_DOJI"
            | "LONGLEGGED"
            | "CDLLONGLEGGEDDOJIWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_long_legged_doji_win_symbol = sym;
                }
                self.show_cdl_long_legged_doji_win = true;
                if self.cdl_long_legged_doji_win_snapshot.symbol.is_empty()
                    && !self.cdl_long_legged_doji_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_long_legged_doji(
                                    &conn,
                                    &self.cdl_long_legged_doji_win_symbol,
                                )
                            {
                                self.cdl_long_legged_doji_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLMARUBOZU" | "MARUBOZU" | "MARUBOZU_CANDLE" | "MARUBOZU_PATTERN"
            | "CDLMARUBOZUWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_marubozu_win_symbol = sym;
                }
                self.show_cdl_marubozu_win = true;
                if self.cdl_marubozu_win_snapshot.symbol.is_empty()
                    && !self.cdl_marubozu_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_marubozu(
                                &conn,
                                &self.cdl_marubozu_win_symbol,
                            ) {
                                self.cdl_marubozu_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLSPINNINGTOP"
            | "SPINNINGTOP"
            | "SPINNING_TOP"
            | "SPINNING_TOP_PATTERN"
            | "CDLSPINNINGTOPWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_spinning_top_win_symbol = sym;
                }
                self.show_cdl_spinning_top_win = true;
                if self.cdl_spinning_top_win_snapshot.symbol.is_empty()
                    && !self.cdl_spinning_top_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_spinning_top(
                                    &conn,
                                    &self.cdl_spinning_top_win_symbol,
                                )
                            {
                                self.cdl_spinning_top_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLTRISTAR" | "TRISTAR" | "TRI_STAR" | "TRIPLE_DOJI" | "CDLTRISTARWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_tristar_win_symbol = sym;
                }
                self.show_cdl_tristar_win = true;
                if self.cdl_tristar_win_snapshot.symbol.is_empty()
                    && !self.cdl_tristar_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_tristar(
                                &conn,
                                &self.cdl_tristar_win_symbol,
                            ) {
                                self.cdl_tristar_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── CDL* doji star / morning doji star / evening doji star / abandoned baby / three inside ──
            "CDLDOJISTAR" | "DOJISTAR" | "DOJI_STAR" | "CDLDOJISTARWIN" | "DOJISTAR_PATTERN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_doji_star_win_symbol = sym;
                }
                self.show_cdl_doji_star_win = true;
                if self.cdl_doji_star_win_snapshot.symbol.is_empty()
                    && !self.cdl_doji_star_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_doji_star(
                                    &conn,
                                    &self.cdl_doji_star_win_symbol,
                                )
                            {
                                self.cdl_doji_star_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLMORNINGDOJISTAR"
            | "MORNINGDOJISTAR"
            | "MORNING_DOJI_STAR"
            | "CDLMORNINGDOJISTARWIN"
            | "MORNING_DOJI_STAR_PATTERN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_morning_doji_star_win_symbol = sym;
                }
                self.show_cdl_morning_doji_star_win = true;
                if self.cdl_morning_doji_star_win_snapshot.symbol.is_empty()
                    && !self.cdl_morning_doji_star_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_morning_doji_star(
                                    &conn,
                                    &self.cdl_morning_doji_star_win_symbol,
                                )
                            {
                                self.cdl_morning_doji_star_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLEVENINGDOJISTAR"
            | "EVENINGDOJISTAR"
            | "EVENING_DOJI_STAR"
            | "CDLEVENINGDOJISTARWIN"
            | "EVENING_DOJI_STAR_PATTERN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_evening_doji_star_win_symbol = sym;
                }
                self.show_cdl_evening_doji_star_win = true;
                if self.cdl_evening_doji_star_win_snapshot.symbol.is_empty()
                    && !self.cdl_evening_doji_star_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_evening_doji_star(
                                    &conn,
                                    &self.cdl_evening_doji_star_win_symbol,
                                )
                            {
                                self.cdl_evening_doji_star_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLABANDONEDBABY"
            | "ABANDONEDBABY"
            | "ABANDONED_BABY"
            | "CDLABANDONEDBABYWIN"
            | "ABANDONED_BABY_PATTERN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_abandoned_baby_win_symbol = sym;
                }
                self.show_cdl_abandoned_baby_win = true;
                if self.cdl_abandoned_baby_win_snapshot.symbol.is_empty()
                    && !self.cdl_abandoned_baby_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_abandoned_baby(
                                    &conn,
                                    &self.cdl_abandoned_baby_win_symbol,
                                )
                            {
                                self.cdl_abandoned_baby_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDL3INSIDE"
            | "THREEINSIDE"
            | "THREE_INSIDE"
            | "CDL3INSIDEWIN"
            | "THREE_INSIDE_PATTERN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_three_inside_win_symbol = sym;
                }
                self.show_cdl_three_inside_win = true;
                if self.cdl_three_inside_win_snapshot.symbol.is_empty()
                    && !self.cdl_three_inside_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_three_inside(
                                    &conn,
                                    &self.cdl_three_inside_win_symbol,
                                )
                            {
                                self.cdl_three_inside_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── CDL* belt hold / closing marubozu / high wave / long line / short line ──
            "CDLBELTHOLD" | "BELTHOLD" | "BELT_HOLD" | "CDLBELTHOLDWIN" | "BELT_HOLD_PATTERN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_belt_hold_win_symbol = sym;
                }
                self.show_cdl_belt_hold_win = true;
                if self.cdl_belt_hold_win_snapshot.symbol.is_empty()
                    && !self.cdl_belt_hold_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_belt_hold(
                                    &conn,
                                    &self.cdl_belt_hold_win_symbol,
                                )
                            {
                                self.cdl_belt_hold_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLCLOSINGMARUBOZU"
            | "CLOSINGMARUBOZU"
            | "CLOSING_MARUBOZU"
            | "CDLCLOSINGMARUBOZUWIN"
            | "CLOSING_MARUBOZU_PATTERN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_closing_marubozu_win_symbol = sym;
                }
                self.show_cdl_closing_marubozu_win = true;
                if self.cdl_closing_marubozu_win_snapshot.symbol.is_empty()
                    && !self.cdl_closing_marubozu_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_closing_marubozu(
                                    &conn,
                                    &self.cdl_closing_marubozu_win_symbol,
                                )
                            {
                                self.cdl_closing_marubozu_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLHIGHWAVE" | "HIGHWAVE" | "HIGH_WAVE" | "CDLHIGHWAVEWIN" | "HIGH_WAVE_PATTERN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_high_wave_win_symbol = sym;
                }
                self.show_cdl_high_wave_win = true;
                if self.cdl_high_wave_win_snapshot.symbol.is_empty()
                    && !self.cdl_high_wave_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_high_wave(
                                    &conn,
                                    &self.cdl_high_wave_win_symbol,
                                )
                            {
                                self.cdl_high_wave_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLLONGLINE" | "LONGLINE" | "LONG_LINE" | "CDLLONGLINEWIN" | "LONG_LINE_PATTERN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_long_line_win_symbol = sym;
                }
                self.show_cdl_long_line_win = true;
                if self.cdl_long_line_win_snapshot.symbol.is_empty()
                    && !self.cdl_long_line_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_long_line(
                                    &conn,
                                    &self.cdl_long_line_win_symbol,
                                )
                            {
                                self.cdl_long_line_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLSHORTLINE" | "SHORTLINE" | "SHORT_LINE" | "CDLSHORTLINEWIN"
            | "SHORT_LINE_PATTERN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_short_line_win_symbol = sym;
                }
                self.show_cdl_short_line_win = true;
                if self.cdl_short_line_win_snapshot.symbol.is_empty()
                    && !self.cdl_short_line_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_short_line(
                                    &conn,
                                    &self.cdl_short_line_win_symbol,
                                )
                            {
                                self.cdl_short_line_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── CDL* counterattack / homing pigeon / in-neck / on-neck / thrusting ──
            "CDLCOUNTERATTACK"
            | "COUNTERATTACK"
            | "COUNTER_ATTACK"
            | "CDLCOUNTERATTACKWIN"
            | "COUNTERATTACK_PATTERN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_counterattack_win_symbol = sym;
                }
                self.show_cdl_counterattack_win = true;
                if self.cdl_counterattack_win_snapshot.symbol.is_empty()
                    && !self.cdl_counterattack_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_counterattack(
                                    &conn,
                                    &self.cdl_counterattack_win_symbol,
                                )
                            {
                                self.cdl_counterattack_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLHOMINGPIGEON"
            | "HOMINGPIGEON"
            | "HOMING_PIGEON"
            | "CDLHOMINGPIGEONWIN"
            | "HOMING_PIGEON_PATTERN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_homing_pigeon_win_symbol = sym;
                }
                self.show_cdl_homing_pigeon_win = true;
                if self.cdl_homing_pigeon_win_snapshot.symbol.is_empty()
                    && !self.cdl_homing_pigeon_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_homing_pigeon(
                                    &conn,
                                    &self.cdl_homing_pigeon_win_symbol,
                                )
                            {
                                self.cdl_homing_pigeon_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLINNECK" | "INNECK" | "IN_NECK" | "CDLINNECKWIN" | "IN_NECK_PATTERN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_in_neck_win_symbol = sym;
                }
                self.show_cdl_in_neck_win = true;
                if self.cdl_in_neck_win_snapshot.symbol.is_empty()
                    && !self.cdl_in_neck_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_in_neck(
                                &conn,
                                &self.cdl_in_neck_win_symbol,
                            ) {
                                self.cdl_in_neck_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLONNECK" | "ONNECK" | "ON_NECK" | "CDLONNECKWIN" | "ON_NECK_PATTERN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_on_neck_win_symbol = sym;
                }
                self.show_cdl_on_neck_win = true;
                if self.cdl_on_neck_win_snapshot.symbol.is_empty()
                    && !self.cdl_on_neck_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_on_neck(
                                &conn,
                                &self.cdl_on_neck_win_symbol,
                            ) {
                                self.cdl_on_neck_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLTHRUSTING" | "THRUSTING" | "THRUST" | "CDLTHRUSTINGWIN" | "THRUSTING_PATTERN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_thrusting_win_symbol = sym;
                }
                self.show_cdl_thrusting_win = true;
                if self.cdl_thrusting_win_snapshot.symbol.is_empty()
                    && !self.cdl_thrusting_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_thrusting(
                                    &conn,
                                    &self.cdl_thrusting_win_symbol,
                                )
                            {
                                self.cdl_thrusting_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDL2CROWS" | "TWOCROWS" | "TWO_CROWS" | "CDL2CROWSWIN" | "TWO_CROWS_PATTERN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_two_crows_win_symbol = sym;
                }
                self.show_cdl_two_crows_win = true;
                if self.cdl_two_crows_win_snapshot.symbol.is_empty()
                    && !self.cdl_two_crows_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_two_crows(
                                    &conn,
                                    &self.cdl_two_crows_win_symbol,
                                )
                            {
                                self.cdl_two_crows_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDL3LINESTRIKE" | "THREELINESTRIKE" | "THREE_LINE_STRIKE" | "CDL3LINESTRIKEWIN"
            | "LINE_STRIKE" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_three_line_strike_win_symbol = sym;
                }
                self.show_cdl_three_line_strike_win = true;
                if self.cdl_three_line_strike_win_snapshot.symbol.is_empty()
                    && !self.cdl_three_line_strike_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_three_line_strike(
                                    &conn,
                                    &self.cdl_three_line_strike_win_symbol,
                                )
                            {
                                self.cdl_three_line_strike_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDL3OUTSIDE" | "THREEOUTSIDE" | "THREE_OUTSIDE" | "CDL3OUTSIDEWIN" | "OUTSIDE3" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_three_outside_win_symbol = sym;
                }
                self.show_cdl_three_outside_win = true;
                if self.cdl_three_outside_win_snapshot.symbol.is_empty()
                    && !self.cdl_three_outside_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_three_outside(
                                    &conn,
                                    &self.cdl_three_outside_win_symbol,
                                )
                            {
                                self.cdl_three_outside_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLMATCHINGLOW" | "MATCHINGLOW" | "MATCHING_LOW" | "CDLMATCHINGLOWWIN"
            | "MATCH_LOW" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_matching_low_win_symbol = sym;
                }
                self.show_cdl_matching_low_win = true;
                if self.cdl_matching_low_win_snapshot.symbol.is_empty()
                    && !self.cdl_matching_low_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_matching_low(
                                    &conn,
                                    &self.cdl_matching_low_win_symbol,
                                )
                            {
                                self.cdl_matching_low_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLSEPARATINGLINES"
            | "SEPARATINGLINES"
            | "SEPARATING_LINES"
            | "CDLSEPARATINGLINESWIN"
            | "SEP_LINES" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_separating_lines_win_symbol = sym;
                }
                self.show_cdl_separating_lines_win = true;
                if self.cdl_separating_lines_win_snapshot.symbol.is_empty()
                    && !self.cdl_separating_lines_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_separating_lines(
                                    &conn,
                                    &self.cdl_separating_lines_win_symbol,
                                )
                            {
                                self.cdl_separating_lines_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLSTICKSANDWICH"
            | "STICKSANDWICH"
            | "STICK_SANDWICH"
            | "CDLSTICKSANDWICHWIN"
            | "SANDWICH" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_stick_sandwich_win_symbol = sym;
                }
                self.show_cdl_stick_sandwich_win = true;
                if self.cdl_stick_sandwich_win_snapshot.symbol.is_empty()
                    && !self.cdl_stick_sandwich_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_stick_sandwich(
                                    &conn,
                                    &self.cdl_stick_sandwich_win_symbol,
                                )
                            {
                                self.cdl_stick_sandwich_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLRICKSHAWMAN" | "RICKSHAWMAN" | "RICKSHAW_MAN" | "CDLRICKSHAWMANWIN"
            | "RICKSHAW" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_rickshaw_man_win_symbol = sym;
                }
                self.show_cdl_rickshaw_man_win = true;
                if self.cdl_rickshaw_man_win_snapshot.symbol.is_empty()
                    && !self.cdl_rickshaw_man_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_rickshaw_man(
                                    &conn,
                                    &self.cdl_rickshaw_man_win_symbol,
                                )
                            {
                                self.cdl_rickshaw_man_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLTAKURI" | "TAKURI" | "CDLTAKURIWIN" | "TAKURI_CANDLE" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_takuri_win_symbol = sym;
                }
                self.show_cdl_takuri_win = true;
                if self.cdl_takuri_win_snapshot.symbol.is_empty()
                    && !self.cdl_takuri_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_takuri(
                                &conn,
                                &self.cdl_takuri_win_symbol,
                            ) {
                                self.cdl_takuri_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDL3STARSINSOUTH"
            | "THREESTARSINSOUTH"
            | "THREE_STARS_IN_SOUTH"
            | "SOUTH_STARS"
            | "CDL3STARSINSOUTHWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_three_stars_in_south_win_symbol = sym;
                }
                self.show_cdl_three_stars_in_south_win = true;
                if self.cdl_three_stars_in_south_win_snapshot.symbol.is_empty()
                    && !self.cdl_three_stars_in_south_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_three_stars_in_south(
                                    &conn,
                                    &self.cdl_three_stars_in_south_win_symbol,
                                )
                            {
                                self.cdl_three_stars_in_south_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLIDENTICAL3CROWS"
            | "IDENTICAL3CROWS"
            | "IDENTICAL_THREE_CROWS"
            | "THREE_IDENTICAL_CROWS"
            | "CDLIDENTICAL3CROWSWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_identical_three_crows_win_symbol = sym;
                }
                self.show_cdl_identical_three_crows_win = true;
                if self
                    .cdl_identical_three_crows_win_snapshot
                    .symbol
                    .is_empty()
                    && !self.cdl_identical_three_crows_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_identical_three_crows(
                                    &conn,
                                    &self.cdl_identical_three_crows_win_symbol,
                                )
                            {
                                self.cdl_identical_three_crows_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLKICKING" | "KICKING" | "CDLKICKINGWIN" | "KICKING_CANDLE" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_kicking_win_symbol = sym;
                }
                self.show_cdl_kicking_win = true;
                if self.cdl_kicking_win_snapshot.symbol.is_empty()
                    && !self.cdl_kicking_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_kicking(
                                &conn,
                                &self.cdl_kicking_win_symbol,
                            ) {
                                self.cdl_kicking_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLKICKINGBYLENGTH"
            | "KICKINGBYLENGTH"
            | "KICKING_BY_LENGTH"
            | "CDLKICKINGBYLENGTHWIN"
            | "KICKING_LENGTH" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_kicking_by_length_win_symbol = sym;
                }
                self.show_cdl_kicking_by_length_win = true;
                if self.cdl_kicking_by_length_win_snapshot.symbol.is_empty()
                    && !self.cdl_kicking_by_length_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_kicking_by_length(
                                    &conn,
                                    &self.cdl_kicking_by_length_win_symbol,
                                )
                            {
                                self.cdl_kicking_by_length_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLLADDERBOTTOM" | "LADDERBOTTOM" | "LADDER_BOTTOM" | "BOTTOM_LADDER"
            | "CDLLADDERBOTTOMWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_ladder_bottom_win_symbol = sym;
                }
                self.show_cdl_ladder_bottom_win = true;
                if self.cdl_ladder_bottom_win_snapshot.symbol.is_empty()
                    && !self.cdl_ladder_bottom_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_ladder_bottom(
                                    &conn,
                                    &self.cdl_ladder_bottom_win_symbol,
                                )
                            {
                                self.cdl_ladder_bottom_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLUNIQUE3RIVER" | "UNIQUE3RIVER" | "UNIQUE_THREE_RIVER" | "THREE_RIVER"
            | "CDLUNIQUE3RIVERWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_unique_three_river_win_symbol = sym;
                }
                self.show_cdl_unique_three_river_win = true;
                if self.cdl_unique_three_river_win_snapshot.symbol.is_empty()
                    && !self.cdl_unique_three_river_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_unique_three_river(
                                    &conn,
                                    &self.cdl_unique_three_river_win_symbol,
                                )
                            {
                                self.cdl_unique_three_river_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLADVANCEBLOCK" | "ADVANCEBLOCK" | "ADVANCE_BLOCK" | "CDLADVANCEBLOCKWIN"
            | "ADV_BLOCK" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_advance_block_win_symbol = sym;
                }
                self.show_cdl_advance_block_win = true;
                if self.cdl_advance_block_win_snapshot.symbol.is_empty()
                    && !self.cdl_advance_block_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_advance_block(
                                    &conn,
                                    &self.cdl_advance_block_win_symbol,
                                )
                            {
                                self.cdl_advance_block_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLBREAKAWAY" | "BREAKAWAY" | "CDLBREAKAWAYWIN" | "BREAK_AWAY" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_breakaway_win_symbol = sym;
                }
                self.show_cdl_breakaway_win = true;
                if self.cdl_breakaway_win_snapshot.symbol.is_empty()
                    && !self.cdl_breakaway_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_breakaway(
                                    &conn,
                                    &self.cdl_breakaway_win_symbol,
                                )
                            {
                                self.cdl_breakaway_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLGAPSIDESIDEWHITE"
            | "GAPSIDESIDEWHITE"
            | "GAP_SIDE_SIDE_WHITE"
            | "CDLGAPSIDESIDEWHITEWIN"
            | "SIDE_BY_SIDE_WHITE" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_gap_side_side_white_win_symbol = sym;
                }
                self.show_cdl_gap_side_side_white_win = true;
                if self.cdl_gap_side_side_white_win_snapshot.symbol.is_empty()
                    && !self.cdl_gap_side_side_white_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_gap_side_side_white(
                                    &conn,
                                    &self.cdl_gap_side_side_white_win_symbol,
                                )
                            {
                                self.cdl_gap_side_side_white_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLUPSIDEGAP2CROWS"
            | "UPSIDEGAP2CROWS"
            | "UPSIDE_GAP_TWO_CROWS"
            | "CDLUPSIDEGAP2CROWSWIN"
            | "GAP2CROWS" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_upside_gap_two_crows_win_symbol = sym;
                }
                self.show_cdl_upside_gap_two_crows_win = true;
                if self.cdl_upside_gap_two_crows_win_snapshot.symbol.is_empty()
                    && !self.cdl_upside_gap_two_crows_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_upside_gap_two_crows(
                                    &conn,
                                    &self.cdl_upside_gap_two_crows_win_symbol,
                                )
                            {
                                self.cdl_upside_gap_two_crows_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLXSIDEGAP3METHODS"
            | "XSIDEGAP3METHODS"
            | "XSIDE_GAP_THREE_METHODS"
            | "CDLXSIDEGAP3METHODSWIN"
            | "GAP3METHODS" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_xside_gap_three_methods_win_symbol = sym;
                }
                self.show_cdl_xside_gap_three_methods_win = true;
                if self
                    .cdl_xside_gap_three_methods_win_snapshot
                    .symbol
                    .is_empty()
                    && !self.cdl_xside_gap_three_methods_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_xside_gap_three_methods(
                                    &conn,
                                    &self.cdl_xside_gap_three_methods_win_symbol,
                                )
                            {
                                self.cdl_xside_gap_three_methods_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLCONCEALBABYSWALL"
            | "CONCEALBABYSWALL"
            | "CONCEAL_BABY_SWALLOW"
            | "CDLCONCEALBABYSWALLWIN"
            | "BABY_SWALLOW" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_conceal_baby_swallow_win_symbol = sym;
                }
                self.show_cdl_conceal_baby_swallow_win = true;
                if self.cdl_conceal_baby_swallow_win_snapshot.symbol.is_empty()
                    && !self.cdl_conceal_baby_swallow_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_conceal_baby_swallow(
                                    &conn,
                                    &self.cdl_conceal_baby_swallow_win_symbol,
                                )
                            {
                                self.cdl_conceal_baby_swallow_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLHIKKAKE" | "HIKKAKE" | "HIKKAKEWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_hikkake_win_symbol = sym;
                }
                self.show_cdl_hikkake_win = true;
                if self.cdl_hikkake_win_snapshot.symbol.is_empty()
                    && !self.cdl_hikkake_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_hikkake(
                                &conn,
                                &self.cdl_hikkake_win_symbol,
                            ) {
                                self.cdl_hikkake_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLHIKKAKEMOD" | "HIKKAKEMOD" | "MODHIKKAKE" | "HIKKAKEMODWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_hikkake_mod_win_symbol = sym;
                }
                self.show_cdl_hikkake_mod_win = true;
                if self.cdl_hikkake_mod_win_snapshot.symbol.is_empty()
                    && !self.cdl_hikkake_mod_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_hikkake_mod(
                                    &conn,
                                    &self.cdl_hikkake_mod_win_symbol,
                                )
                            {
                                self.cdl_hikkake_mod_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLMATHOLD" | "MATHOLD" | "MAT_HOLD" | "MATHOLDWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_mat_hold_win_symbol = sym;
                }
                self.show_cdl_mat_hold_win = true;
                if self.cdl_mat_hold_win_snapshot.symbol.is_empty()
                    && !self.cdl_mat_hold_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_cdl_mat_hold(
                                &conn,
                                &self.cdl_mat_hold_win_symbol,
                            ) {
                                self.cdl_mat_hold_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLRISEFALL3METHODS"
            | "RISEFALL3METHODS"
            | "RISE_FALL_THREE_METHODS"
            | "CDLRISEFALL3METHODSWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_rise_fall_three_methods_win_symbol = sym;
                }
                self.show_cdl_rise_fall_three_methods_win = true;
                if self
                    .cdl_rise_fall_three_methods_win_snapshot
                    .symbol
                    .is_empty()
                    && !self.cdl_rise_fall_three_methods_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_rise_fall_three_methods(
                                    &conn,
                                    &self.cdl_rise_fall_three_methods_win_symbol,
                                )
                            {
                                self.cdl_rise_fall_three_methods_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLSTALLEDPATTERN"
            | "STALLEDPATTERN"
            | "STALLED_PATTERN"
            | "STALLPATTERN"
            | "CDLSTALLEDPATTERNWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_stalled_pattern_win_symbol = sym;
                }
                self.show_cdl_stalled_pattern_win = true;
                if self.cdl_stalled_pattern_win_snapshot.symbol.is_empty()
                    && !self.cdl_stalled_pattern_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_stalled_pattern(
                                    &conn,
                                    &self.cdl_stalled_pattern_win_symbol,
                                )
                            {
                                self.cdl_stalled_pattern_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CDLTASUKIGAP" | "TASUKIGAP" | "TASUKI_GAP" | "CDLTASUKIGAPWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.cdl_tasuki_gap_win_symbol = sym;
                }
                self.show_cdl_tasuki_gap_win = true;
                if self.cdl_tasuki_gap_win_snapshot.symbol.is_empty()
                    && !self.cdl_tasuki_gap_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) =
                                typhoon_engine::core::research::get_cdl_tasuki_gap(
                                    &conn,
                                    &self.cdl_tasuki_gap_win_symbol,
                                )
                            {
                                self.cdl_tasuki_gap_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── Quant Stats aliases ──
            "MODSHARPE" | "ADJSHARPE" | "ADJUSTED_SHARPE" | "PEZIER_WHITE" | "MODSHARPEWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.modsharpe_win_symbol = sym;
                }
                self.show_modsharpe_win = true;
                if self.modsharpe_win_snapshot.symbol.is_empty()
                    && !self.modsharpe_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_modsharpe(
                                &conn,
                                &self.modsharpe_win_symbol,
                            ) {
                                self.modsharpe_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HSIEHTEST" | "HSIEH" | "HSIEH_NONLIN" | "NONLIN_3RDMOM" | "HSIEHTESTWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.hsiehtest_win_symbol = sym;
                }
                self.show_hsiehtest_win = true;
                if self.hsiehtest_win_snapshot.symbol.is_empty()
                    && !self.hsiehtest_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_hsiehtest(
                                &conn,
                                &self.hsiehtest_win_symbol,
                            ) {
                                self.hsiehtest_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "CHOWBREAK" | "CHOW" | "CHOW_TEST" | "STRUCT_BREAK" | "CHOWBREAKWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.chowbreak_win_symbol = sym;
                }
                self.show_chowbreak_win = true;
                if self.chowbreak_win_snapshot.symbol.is_empty()
                    && !self.chowbreak_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_chowbreak(
                                &conn,
                                &self.chowbreak_win_symbol,
                            ) {
                                self.chowbreak_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "DRIFTBURST" | "DRIFT_BURST" | "COR18" | "KERNEL_DRIFT" | "DRIFTBURSTWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.driftburst_win_symbol = sym;
                }
                self.show_driftburst_win = true;
                if self.driftburst_win_snapshot.symbol.is_empty()
                    && !self.driftburst_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_driftburst(
                                &conn,
                                &self.driftburst_win_symbol,
                            ) {
                                self.driftburst_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "HLVCLUST" | "PARKINSON_CLUST" | "HL_CLUSTER" | "HL_VOLCLUST" | "HLVCLUSTWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.hlvclust_win_symbol = sym;
                }
                self.show_hlvclust_win = true;
                if self.hlvclust_win_snapshot.symbol.is_empty()
                    && !self.hlvclust_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_hlvclust(
                                &conn,
                                &self.hlvclust_win_symbol,
                            ) {
                                self.hlvclust_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            // ── palette aliases (Quant Stats) ──
            "YANGZHANG" | "YZ_VOL" | "YZVOL" | "YZ_RANGEVOL" | "YANGZHANGWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.yangzhang_win_symbol = sym;
                }
                self.show_yangzhang_win = true;
                if self.yangzhang_win_snapshot.symbol.is_empty()
                    && !self.yangzhang_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_yangzhang(
                                &conn,
                                &self.yangzhang_win_symbol,
                            ) {
                                self.yangzhang_win_snapshot = snap;
                            }
                        }
                    }
                }
            }
            "KUIPER" | "KUIPERTEST" | "KUIPER_CDF" | "VSTAT" | "KUIPERWIN" => {
                let sym = command_chart_symbol(
                    self.charts.get(self.active_tab).map(|c| c.symbol.as_str()),
                );
                if !sym.is_empty() {
                    self.kuiper_win_symbol = sym;
                }
                self.show_kuiper_win = true;
                if self.kuiper_win_snapshot.symbol.is_empty() && !self.kuiper_win_symbol.is_empty()
                {
                    if let Some(ref cache) = self.cache {
                        if let Ok(conn) = cache.connection() {
                            if let Ok(Some(snap)) = typhoon_engine::core::research::get_kuiper(
                                &conn,
                                &self.kuiper_win_symbol,
                            ) {
                                self.kuiper_win_snapshot = snap;
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
