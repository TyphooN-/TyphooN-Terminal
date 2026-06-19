use super::super::*;

impl TyphooNApp {
    pub(super) fn handle_chart_drawing_command(
        &mut self,
        cmd_upper: &str,
        ctx: &egui::Context,
    ) -> bool {
        match cmd_upper {
            "SNAP" | "MAGNET" => {
                self.snap_enabled = !self.snap_enabled;
                self.log.push_back(LogEntry::info(format!(
                    "Magnet snap: {}",
                    if self.snap_enabled { "ON" } else { "OFF" }
                )));
            }
            "CROSS_TF" | "CROSS_TF_DRAWINGS" => {
                self.cross_tf_drawings = !self.cross_tf_drawings;
                self.log.push_back(LogEntry::info(format!(
                    "Cross-TF drawings: {}",
                    if self.cross_tf_drawings {
                        "ON — drawings shared across timeframes"
                    } else {
                        "OFF"
                    }
                )));
            }
            "FIT" | "FIT_ALL" | "AUTO_FIT" => {
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    chart.visible_bars = chart.bars.len().max(50);
                    chart.view_offset = chart.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
                    chart.price_zoom = 1.0;
                    chart.price_pan = 0.0;
                    self.log
                        .push_back(LogEntry::info("Auto-fit: showing all bars"));
                }
            }
            "LOG_SCALE" | "LOG" => {
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    chart.log_scale = !chart.log_scale;
                    self.log.push_back(LogEntry::info(format!(
                        "Price scale: {}",
                        if chart.log_scale {
                            "logarithmic"
                        } else {
                            "linear"
                        }
                    )));
                }
            }
            "FOLLOW" | "AUTO_SCROLL" => {
                self.follow_latest = !self.follow_latest;
                self.log.push_back(LogEntry::info(format!(
                    "Follow latest: {}",
                    if self.follow_latest {
                        "ON — chart auto-scrolls"
                    } else {
                        "OFF — locked position"
                    }
                )));
            }
            "DRAW_HLINE" => self.draw_mode = DrawMode::PlacingHLine,
            "DRAW_TRENDLINE" => self.draw_mode = DrawMode::PlacingTrendP1,
            "DRAW_FIBO" => self.draw_mode = DrawMode::PlacingFiboP1,
            "DRAW_VLINE" => self.draw_mode = DrawMode::PlacingVLine,
            "DRAW_RECT" => self.draw_mode = DrawMode::PlacingRectP1,
            "DRAW_RAY" => self.draw_mode = DrawMode::PlacingRayP1,
            "DRAW_CHANNEL" => self.draw_mode = DrawMode::PlacingChannelP1,
            "DRAW_PARALLEL_CH" => self.draw_mode = DrawMode::PlacingParallelChP1,
            "DRAW_FIB_CHANNEL" => self.draw_mode = DrawMode::PlacingFibChannelP1,
            "DRAW_FIB_TIME" => self.draw_mode = DrawMode::PlacingFibTimeZones,
            "DRAW_PRICE_LABEL" => self.draw_mode = DrawMode::PlacingPriceLabel,
            "DRAW_CALLOUT" => self.draw_mode = DrawMode::PlacingCalloutP1,
            "DRAW_HIGHLIGHTER" => self.draw_mode = DrawMode::PlacingHighlighterP1,
            "DRAW_CROSS_MARKER" => self.draw_mode = DrawMode::PlacingCrossMarker,
            "DRAW_POLYLINE" => {
                self.draw_mode = DrawMode::PlacingPolyline;
                self.polyline_points.clear();
            }
            "DRAW_ANCHOR_NOTE" => self.draw_mode = DrawMode::PlacingAnchorNote,
            "DRAW_REGRESSION" => self.draw_mode = DrawMode::PlacingRegressionChP1,
            "DRAW_GANN_BOX" => self.draw_mode = DrawMode::PlacingGannBoxP1,
            "DRAW_ELLIOTT" => {
                self.draw_mode = DrawMode::PlacingElliottWave;
                self.multi_click_points.clear();
            }
            "DRAW_ABC" => {
                self.draw_mode = DrawMode::PlacingAbcCorrection;
                self.multi_click_points.clear();
            }
            "DRAW_DATE_RANGE" => self.draw_mode = DrawMode::PlacingDateRangeP1,
            "DRAW_DATE_PRICE" => self.draw_mode = DrawMode::PlacingDatePriceRangeP1,
            "DRAW_HEAD_SHOULDERS" => {
                self.draw_mode = DrawMode::PlacingHeadShoulders;
                self.multi_click_points.clear();
            }
            "DRAW_XABCD" => {
                self.draw_mode = DrawMode::PlacingXabcdPattern;
                self.multi_click_points.clear();
            }
            "DRAW_BRUSH" => {
                self.draw_mode = DrawMode::PlacingBrush;
                self.brush_points.clear();
            }
            "DRAW_SCHIFF_FORK" => self.draw_mode = DrawMode::PlacingSchiffPitchforkP1,
            "DRAW_MOD_SCHIFF_FORK" => self.draw_mode = DrawMode::PlacingModSchiffPitchforkP1,
            "DRAW_CYCLIC_LINES" => self.draw_mode = DrawMode::PlacingCyclicLinesP1,
            "DRAW_SINE_WAVE" => self.draw_mode = DrawMode::PlacingSineWaveP1,
            "DRAW_EMOJI" => self.draw_mode = DrawMode::PlacingEmoji,
            "DRAW_FLAG" => self.draw_mode = DrawMode::PlacingFlag,
            "DRAW_BALLOON" => self.draw_mode = DrawMode::PlacingBalloonP1,
            "DRAW_SESSION_BREAK" => self.draw_mode = DrawMode::PlacingSessionBreak,
            "DRAW_MAGNET_LEVEL" => self.draw_mode = DrawMode::PlacingMagnetLevel,
            "DRAW_RISK_REWARD" => self.draw_mode = DrawMode::PlacingRiskRewardP1,
            "DRAW_FIB_CIRCLE" => self.draw_mode = DrawMode::PlacingFibCircleP1,
            "DRAW_ARC" => self.draw_mode = DrawMode::PlacingArcP1,
            "DRAW_CURVE" => self.draw_mode = DrawMode::PlacingCurveP1,
            "DRAW_PATH" => {
                self.draw_mode = DrawMode::PlacingPath;
                self.polyline_points.clear();
            }
            "DRAW_FORECAST" => self.draw_mode = DrawMode::PlacingForecastP1,
            "DRAW_GHOST_FEED" => self.draw_mode = DrawMode::PlacingGhostFeedP1,
            "DRAW_SIGNPOST" => self.draw_mode = DrawMode::PlacingSignpost,
            "DRAW_RULER" => self.draw_mode = DrawMode::PlacingRulerP1,
            "DRAW_TIME_CYCLE" => self.draw_mode = DrawMode::PlacingTimeCycleP1,
            "DRAW_SPEED_FAN" => self.draw_mode = DrawMode::PlacingSpeedFanP1,
            "DRAW_SPEED_ARC" => self.draw_mode = DrawMode::PlacingSpeedArcP1,
            "DRAW_FIB_SPIRAL" => self.draw_mode = DrawMode::PlacingFibSpiralP1,
            "DRAW_ROTATED_RECT" => self.draw_mode = DrawMode::PlacingRotatedRectP1,
            "DRAW_ANCHORED_VWAP" => self.draw_mode = DrawMode::PlacingAnchoredVwap,
            "DRAW_TREND_CHANNEL" => self.draw_mode = DrawMode::PlacingTrendChannelP1,
            "DRAW_INSIDE_PITCHFORK" => self.draw_mode = DrawMode::PlacingInsidePitchforkP1,
            "DRAW_FIB_WEDGE" => self.draw_mode = DrawMode::PlacingFibWedgeP1,
            "DRAW_PRICE_NOTE" => self.draw_mode = DrawMode::PlacingPriceNote,
            "DRAW_MEASURE_TOOL" => self.draw_mode = DrawMode::PlacingMeasureToolP1,
            "DRAW_ANCHORED_TEXT" => self.draw_mode = DrawMode::PlacingAnchoredText,
            "DRAW_COMMENT" => self.draw_mode = DrawMode::PlacingComment,
            "DRAW_ARROW_LEFT" => self.draw_mode = DrawMode::PlacingArrowMarkerLeft,
            "DRAW_ARROW_RIGHT" => self.draw_mode = DrawMode::PlacingArrowMarkerRight,
            "DRAW_CIRCLE" => self.draw_mode = DrawMode::PlacingCircleP1,
            "DRAW_PITCH_FAN" => self.draw_mode = DrawMode::PlacingPitchFanP1,
            "DRAW_TREND_FIB_TIME" => self.draw_mode = DrawMode::PlacingTrendFibTimeP1,
            "DRAW_GANN_SQUARE" => self.draw_mode = DrawMode::PlacingGannSquareP1,
            "DRAW_GANN_SQUARE_FIXED" => self.draw_mode = DrawMode::PlacingGannSquareFixedP1,
            "DRAW_BARS_PATTERN" => self.draw_mode = DrawMode::PlacingBarsPatternP1,
            "DRAW_PROJECTION" => self.draw_mode = DrawMode::PlacingProjectionP1,
            "DRAW_DOUBLE_CURVE" => self.draw_mode = DrawMode::PlacingDoubleCurveP1,
            "DRAW_TRIANGLE_PATTERN" => {
                self.draw_mode = DrawMode::PlacingTrianglePattern;
                self.multi_click_points.clear();
            }
            "DRAW_THREE_DRIVES" => {
                self.draw_mode = DrawMode::PlacingThreeDrives;
                self.multi_click_points.clear();
            }
            "DRAW_ELLIOTT_DOUBLE" => {
                self.draw_mode = DrawMode::PlacingElliottDouble;
                self.multi_click_points.clear();
            }
            "DRAW_ABCD" => {
                self.draw_mode = DrawMode::PlacingAbcdPattern;
                self.multi_click_points.clear();
            }
            "DRAW_CYPHER" => {
                self.draw_mode = DrawMode::PlacingCypherPattern;
                self.multi_click_points.clear();
            }
            "DRAW_ELLIOTT_TRIANGLE" => {
                self.draw_mode = DrawMode::PlacingElliottTriangle;
                self.multi_click_points.clear();
            }
            "DRAW_ELLIOTT_TRIPLE" => {
                self.draw_mode = DrawMode::PlacingElliottTripleCombo;
                self.multi_click_points.clear();
            }
            "DRAW_ERASER" => {
                self.draw_mode = DrawMode::Eraser;
            }
            "CLEAR_DRAWINGS" => {
                if let Some(c) = self.charts.get_mut(self.active_tab) {
                    c.drawings.clear();
                    c.drawing_styles.clear();
                }
            }
            "SESSIONS" => {
                self.show_sessions = !self.show_sessions;
                self.log.push_back(LogEntry::info(format!(
                    "Sessions: {}",
                    if self.show_sessions { "ON" } else { "OFF" }
                )));
            }
            "VOL_HEATMAP" => {
                self.show_vol_heatmap = !self.show_vol_heatmap;
                self.log.push_back(LogEntry::info(format!(
                    "Volume heatmap: {}",
                    if self.show_vol_heatmap { "ON" } else { "OFF" }
                )));
            }
            "VWAP" => {
                self.show_vwap = !self.show_vwap;
                self.log.push_back(LogEntry::info(format!(
                    "VWAP: {}",
                    if self.show_vwap { "ON" } else { "OFF" }
                )));
            }
            "PRICE_HIST" => {
                self.show_price_histogram = !self.show_price_histogram;
                self.log.push_back(LogEntry::info(format!(
                    "Price histogram: {}",
                    if self.show_price_histogram {
                        "ON"
                    } else {
                        "OFF"
                    }
                )));
            }
            "SUPERTREND" => {
                self.show_supertrend = !self.show_supertrend;
                self.log.push_back(LogEntry::info(format!(
                    "Supertrend: {}",
                    if self.show_supertrend { "ON" } else { "OFF" }
                )));
            }
            "DONCHIAN" => {
                self.show_donchian = !self.show_donchian;
                self.log.push_back(LogEntry::info(format!(
                    "Donchian: {}",
                    if self.show_donchian { "ON" } else { "OFF" }
                )));
            }
            "KELTNER" => {
                self.show_keltner = !self.show_keltner;
                self.log.push_back(LogEntry::info(format!(
                    "Keltner: {}",
                    if self.show_keltner { "ON" } else { "OFF" }
                )));
            }
            "REGRESSION" => {
                self.show_regression = !self.show_regression;
                self.log.push_back(LogEntry::info(format!(
                    "Regression: {}",
                    if self.show_regression { "ON" } else { "OFF" }
                )));
            }
            "SQUEEZE" => {
                self.show_squeeze = !self.show_squeeze;
                self.log.push_back(LogEntry::info(format!(
                    "Squeeze: {}",
                    if self.show_squeeze { "ON" } else { "OFF" }
                )));
            }
            "VAROSC" | "VAR_OSC" | "VAR_OSCILLATOR" => {
                self.show_var_oscillator = !self.show_var_oscillator;
                self.log.push_back(LogEntry::info(format!(
                    "VaR Oscillator: {}",
                    if self.show_var_oscillator {
                        "ON"
                    } else {
                        "OFF"
                    }
                )));
            }
            "CMO_CHART" | "SHOW_CMO" => {
                self.show_cmo = !self.show_cmo;
                self.log.push_back(LogEntry::info(format!(
                    "CMO chart pane: {}",
                    if self.show_cmo { "ON" } else { "OFF" }
                )));
            }
            "QSTICK_CHART" | "SHOW_QSTICK" => {
                self.show_qstick = !self.show_qstick;
                self.log.push_back(LogEntry::info(format!(
                    "QStick chart pane: {}",
                    if self.show_qstick { "ON" } else { "OFF" }
                )));
            }
            "DISPARITY_CHART" | "SHOW_DISPARITY" => {
                self.show_disparity = !self.show_disparity;
                self.log.push_back(LogEntry::info(format!(
                    "Disparity chart pane: {}",
                    if self.show_disparity { "ON" } else { "OFF" }
                )));
            }
            "BOP_CHART" | "SHOW_BOP" => {
                self.show_bop = !self.show_bop;
                self.log.push_back(LogEntry::info(format!(
                    "BOP chart pane: {}",
                    if self.show_bop { "ON" } else { "OFF" }
                )));
            }
            "STDDEV_CHART" | "SHOW_STDDEV" => {
                self.show_stddev = !self.show_stddev;
                self.log.push_back(LogEntry::info(format!(
                    "StdDev chart pane: {}",
                    if self.show_stddev { "ON" } else { "OFF" }
                )));
            }
            "MFI_CHART" | "SHOW_MFI" => {
                self.show_mfi = !self.show_mfi;
                self.log.push_back(LogEntry::info(format!(
                    "MFI chart pane: {}",
                    if self.show_mfi { "ON" } else { "OFF" }
                )));
            }
            "TRIX_CHART" | "SHOW_TRIX" => {
                self.show_trix = !self.show_trix;
                self.log.push_back(LogEntry::info(format!(
                    "TRIX chart pane: {}",
                    if self.show_trix { "ON" } else { "OFF" }
                )));
            }
            "PPO_CHART" | "SHOW_PPO" => {
                self.show_ppo = !self.show_ppo;
                self.log.push_back(LogEntry::info(format!(
                    "PPO chart pane: {}",
                    if self.show_ppo { "ON" } else { "OFF" }
                )));
            }
            "ULTOSC_CHART" | "SHOW_ULTOSC" => {
                self.show_ultosc = !self.show_ultosc;
                self.log.push_back(LogEntry::info(format!(
                    "ULTOSC chart pane: {}",
                    if self.show_ultosc { "ON" } else { "OFF" }
                )));
            }
            "STOCHRSI_CHART" | "SHOW_STOCHRSI" => {
                self.show_stochrsi = !self.show_stochrsi;
                self.log.push_back(LogEntry::info(format!(
                    "StochRSI chart pane: {}",
                    if self.show_stochrsi { "ON" } else { "OFF" }
                )));
            }
            "FVG" | "FAIR_VALUE_GAP" => {
                self.show_fvg = !self.show_fvg;
                self.log.push_back(LogEntry::info(format!(
                    "FVG: {}",
                    if self.show_fvg { "ON" } else { "OFF" }
                )));
            }
            "ORDER_BLOCKS" | "OB" => {
                self.show_order_blocks = !self.show_order_blocks;
                self.log.push_back(LogEntry::info(format!(
                    "Order Blocks: {}",
                    if self.show_order_blocks { "ON" } else { "OFF" }
                )));
            }
            "COPY_CHART" => {
                if let Some(chart) = self.charts.get(self.active_tab) {
                    let (vs, ve) = chart.visible_range();
                    let visible = &chart.bars[vs..ve];
                    if visible.is_empty() {
                        self.log
                            .push_back(LogEntry::warn("No visible bars to copy"));
                    } else {
                        let mut csv = String::from("Date,Open,High,Low,Close,Volume\n");
                        for bar in visible {
                            let dt = chrono::DateTime::from_timestamp_millis(bar.ts_ms)
                                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                                .unwrap_or_else(|| bar.ts_ms.to_string());
                            csv.push_str(&format!(
                                "{},{},{},{},{},{}\n",
                                dt, bar.open, bar.high, bar.low, bar.close, bar.volume
                            ));
                        }
                        ctx.copy_text(csv);
                        self.log.push_back(LogEntry::info(format!(
                            "Copied {} bars to clipboard as CSV",
                            visible.len()
                        )));
                    }
                }
            }
            "OBJECTS" | "OBJECT_LIST" => {
                self.show_object_list = !self.show_object_list;
            }
            _ => return false,
        }
        true
    }
}
