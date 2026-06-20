use super::*;

mod controls;
mod persistence_io;
mod snapshot_build;
mod sync_preferences;

impl TyphooNApp {
    pub(super) fn load_session(&mut self) {
        let path = Self::session_json_path();
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                self.workspaces.clear();
                self.chart_templates.clear();
                self.journal_entries.clear();
                self.alerts.clear();
                self.alerts_set.clear();
                if let Some(sym) = v["symbol"].as_str() {
                    self.symbol_input = sym.to_string();
                }
                if let Some(mtf) = v["mtf_enabled"].as_bool() {
                    self.mtf_enabled = mtf;
                }
                if let Some(b) = v["command_open"].as_bool() {
                    self.command_open = b;
                }
                if let Some(b) = v["compact_mode"].as_bool() {
                    self.compact_mode = b;
                }
                self.apply_sync_preferences_value(&v);
                if let Some(tab) = v["active_tab"].as_u64() {
                    self.active_tab = tab as usize;
                }
                if let Some(arr) = v["mtf_visible"].as_array() {
                    self.mtf_visible = arr.iter().map(|v| v.as_bool().unwrap_or(true)).collect();
                }
                // Broker scope + econ filters (added 2026-04-09)
                self.broker_scope = match v["broker_scope"].as_str() {
                    Some("alpaca") => EventSource::Alpaca,
                    Some("kraken") => EventSource::Kraken,
                    Some("positions") => EventSource::Positions,
                    _ => EventSource::All,
                };
                if let Some(b) = v["econ_filter_high"].as_bool() {
                    self.econ_filter_high = b;
                }
                if let Some(b) = v["econ_filter_medium"].as_bool() {
                    self.econ_filter_medium = b;
                }
                if let Some(b) = v["econ_filter_low"].as_bool() {
                    self.econ_filter_low = b;
                }
                if let Some(b) = v["econ_filter_holiday"].as_bool() {
                    self.econ_filter_holiday = b;
                }
                if let Some(s) = v["econ_filter_currencies"].as_str() {
                    self.econ_filter_currencies = s.to_string();
                }
                // Restore tabs: symbol, timeframe, chart type — rebuild charts from session
                if let Some(tabs) = v["tabs"].as_array() {
                    if !tabs.is_empty() {
                        // Rebuild chart set from session data
                        self.charts.clear();
                        for tab in tabs {
                            // Canonicalise legacy sessions: before `bare_symbol_from_key`
                            // was introduced, Screener/watchlist load paths saved full
                            // cache keys (`kraken-equities:SLV:1Hour`) into chart.symbol. Normalise
                            // to bare here so try_load doesn't double-prefix.
                            let raw_sym = tab["symbol"].as_str().unwrap_or("CC");
                            let sym = bare_symbol_from_key(raw_sym);
                            let tf = tab["timeframe"]
                                .as_str()
                                .and_then(Timeframe::from_label)
                                .unwrap_or(Timeframe::H4);
                            let ct = match tab["chart_type"].as_str() {
                                Some("Heikin-Ashi") => ChartType::HeikinAshi,
                                Some("Line") => ChartType::Line,
                                Some("OHLC Bars") => ChartType::OhlcBars,
                                Some("Renko") => ChartType::Renko,
                                _ => ChartType::Candle,
                            };
                            let mut chart = ChartState::new(&sym, tf);
                            chart.chart_type = ct;
                            chart.log_scale = tab["log_scale"].as_bool().unwrap_or(false);
                            if let Some(visible_bars) = tab["visible_bars"].as_u64() {
                                chart.visible_bars = visible_bars as usize;
                            }
                            if let Some(view_offset) = tab["view_offset"].as_u64() {
                                chart.view_offset = view_offset as usize;
                            }
                            self.charts.push(chart);
                        }
                        self.active_tab = self.active_tab.min(self.charts.len().saturating_sub(1));
                        while self.mtf_visible.len() < self.charts.len() {
                            self.mtf_visible.push(true);
                        }
                        self.hydrate_loaded_charts();
                    }
                }
                if let Some(ind) = v.get("indicators") {
                    for (key, field) in [
                        ("sma200", &mut self.show_sma200),
                        ("sma100", &mut self.show_sma100),
                        ("kama", &mut self.show_kama),
                        ("ema21", &mut self.show_ema21),
                        ("bollinger", &mut self.show_bollinger),
                        ("ichimoku", &mut self.show_ichimoku),
                        ("wma", &mut self.show_wma),
                        ("hma", &mut self.show_hma),
                        ("psar", &mut self.show_psar),
                        ("atr_proj", &mut self.show_atr_proj),
                        ("prev_levels", &mut self.show_prev_levels),
                        ("pivots", &mut self.show_pivots),
                        ("fractals", &mut self.show_fractals),
                        ("harmonics", &mut self.show_harmonics),
                        ("supply_demand", &mut self.show_supply_demand),
                        ("ehlers_ss", &mut self.show_ehlers_ss),
                        ("ehlers_decycler", &mut self.show_ehlers_decycler),
                        ("ehlers_itl", &mut self.show_ehlers_itl),
                        ("ehlers_mama", &mut self.show_ehlers_mama),
                        ("ehlers_ebsw", &mut self.show_ehlers_ebsw),
                        ("ehlers_cyber", &mut self.show_ehlers_cyber),
                        ("ehlers_cg", &mut self.show_ehlers_cg),
                        ("ehlers_roof", &mut self.show_ehlers_roof),
                        ("rsi", &mut self.show_rsi),
                        ("fisher", &mut self.show_fisher),
                        ("macd", &mut self.show_macd),
                        ("stochastic", &mut self.show_stochastic),
                        ("adx", &mut self.show_adx),
                        ("cci", &mut self.show_cci),
                        ("williams_r", &mut self.show_williams_r),
                        ("obv", &mut self.show_obv),
                        ("momentum", &mut self.show_momentum),
                        ("cmo", &mut self.show_cmo),
                        ("qstick", &mut self.show_qstick),
                        ("disparity", &mut self.show_disparity),
                        ("bop", &mut self.show_bop),
                        ("stddev", &mut self.show_stddev),
                        ("mfi", &mut self.show_mfi),
                        ("trix", &mut self.show_trix),
                        ("ppo", &mut self.show_ppo),
                        ("ultosc", &mut self.show_ultosc),
                        ("stochrsi", &mut self.show_stochrsi),
                        ("var_oscillator", &mut self.show_var_oscillator),
                        ("better_volume", &mut self.show_better_volume),
                        ("volume_pane", &mut self.show_volume_pane),
                        ("sessions", &mut self.show_sessions),
                        ("vol_heatmap", &mut self.show_vol_heatmap),
                        ("vwap", &mut self.show_vwap),
                        ("price_histogram", &mut self.show_price_histogram),
                        ("supertrend", &mut self.show_supertrend),
                        ("donchian", &mut self.show_donchian),
                        ("keltner", &mut self.show_keltner),
                        ("regression", &mut self.show_regression),
                        ("squeeze", &mut self.show_squeeze),
                        ("fvg", &mut self.show_fvg),
                        ("order_blocks", &mut self.show_order_blocks),
                    ] {
                        if let Some(b) = ind[key].as_bool() {
                            *field = b;
                        }
                    }
                }
                // Restore drawings (all types)
                if let Some(drawings) = v["drawings"].as_array() {
                    if let Some(chart) = self.charts.get_mut(0) {
                        let parse_col = |d: &serde_json::Value| -> egui::Color32 {
                            let c = &d["color"];
                            egui::Color32::from_rgb(
                                c[0].as_u64().unwrap_or(200) as u8,
                                c[1].as_u64().unwrap_or(200) as u8,
                                c[2].as_u64().unwrap_or(200) as u8,
                            )
                        };
                        let parse_pt = |d: &serde_json::Value, key: &str| -> Option<(usize, f64)> {
                            let a = &d[key];
                            Some((a[0].as_u64()? as usize, a[1].as_f64()?))
                        };
                        for d in drawings {
                            match d["type"].as_str() {
                                Some("hline") => {
                                    if let Some(price) = d["price"].as_f64() {
                                        let key = format!("{:.10}", price);
                                        if chart.hline_set.insert(key) {
                                            chart.drawings.push(Drawing::HLine {
                                                price,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("vline") => {
                                    if let Some(idx) = d["bar_idx"].as_u64() {
                                        chart.drawings.push(Drawing::VLine {
                                            bar_idx: idx as usize,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("trendline") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::TrendLine {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("fibo") => {
                                    if let (Some(h), Some(l), Some(bs), Some(be)) = (
                                        d["high"].as_f64(),
                                        d["low"].as_f64(),
                                        d["bar_start"].as_u64(),
                                        d["bar_end"].as_u64(),
                                    ) {
                                        chart.drawings.push(Drawing::FiboRetrace {
                                            high: h,
                                            low: l,
                                            bar_start: bs as usize,
                                            bar_end: be as usize,
                                        });
                                    }
                                }
                                Some("rect") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::Rectangle {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("ray") => {
                                    if let (Some(o), Some(s)) =
                                        (parse_pt(d, "origin"), d["slope"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::Ray {
                                            origin: o,
                                            slope: s,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("channel") => {
                                    if let (Some(p1), Some(p2), Some(w)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"), d["width"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::Channel {
                                            p1,
                                            p2,
                                            width: w,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("extline") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::ExtendedLine {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("hray") => {
                                    if let (Some(idx), Some(p)) =
                                        (d["bar_idx"].as_u64(), d["price"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::HRay {
                                            bar_idx: idx as usize,
                                            price: p,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("crossline") => {
                                    if let (Some(idx), Some(p)) =
                                        (d["bar_idx"].as_u64(), d["price"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::CrossLine {
                                            bar_idx: idx as usize,
                                            price: p,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("arrowline") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::ArrowLine {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("infoline") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::InfoLine {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("pitchfork") => {
                                    if let (Some(pv), Some(p2), Some(p3)) =
                                        (parse_pt(d, "pivot"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::Pitchfork {
                                            pivot: pv,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("fiboext") => {
                                    if let (Some(p1), Some(p2), Some(p3)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::FiboExtension {
                                            p1,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("gannfan") => {
                                    if let (Some(o), Some(s)) =
                                        (parse_pt(d, "origin"), d["scale"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::GannFan {
                                            origin: o,
                                            scale: s,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("longpos") => {
                                    if let (Some(e), Some(s), Some(t)) = (
                                        parse_pt(d, "entry"),
                                        d["stop"].as_f64(),
                                        d["target"].as_f64(),
                                    ) {
                                        chart.drawings.push(Drawing::LongPosition {
                                            entry: e,
                                            stop: s,
                                            target: t,
                                        });
                                    }
                                }
                                Some("shortpos") => {
                                    if let (Some(e), Some(s), Some(t)) = (
                                        parse_pt(d, "entry"),
                                        d["stop"].as_f64(),
                                        d["target"].as_f64(),
                                    ) {
                                        chart.drawings.push(Drawing::ShortPosition {
                                            entry: e,
                                            stop: s,
                                            target: t,
                                        });
                                    }
                                }
                                Some("pricerange") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::PriceRange { p1, p2 });
                                    }
                                }
                                Some("text") => {
                                    if let (Some(idx), Some(p), Some(t)) = (
                                        d["bar_idx"].as_u64(),
                                        d["price"].as_f64(),
                                        d["text"].as_str(),
                                    ) {
                                        chart.drawings.push(Drawing::TextLabel {
                                            bar_idx: idx as usize,
                                            price: p,
                                            text: t.to_string(),
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("arrowmarker") => {
                                    if let (Some(idx), Some(p), Some(up)) = (
                                        d["bar_idx"].as_u64(),
                                        d["price"].as_f64(),
                                        d["is_up"].as_bool(),
                                    ) {
                                        chart.drawings.push(Drawing::ArrowMarker {
                                            bar_idx: idx as usize,
                                            price: p,
                                            is_up: up,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("ellipse") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::Ellipse {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("triangle") => {
                                    if let (Some(p1), Some(p2), Some(p3)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::Triangle {
                                            p1,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("trendangle") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::TrendAngle {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("parallelch") => {
                                    if let (Some(p1), Some(p2), Some(off)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"), d["offset"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::ParallelChannel {
                                            p1,
                                            p2,
                                            offset: off,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("fibchannel") => {
                                    if let (Some(p1), Some(p2), Some(p3)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::FibChannel {
                                            p1,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("fibtimezones") => {
                                    if let Some(idx) = d["bar_idx"].as_u64() {
                                        chart.drawings.push(Drawing::FibTimeZones {
                                            bar_idx: idx as usize,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("pricelabel") => {
                                    if let (Some(idx), Some(p)) =
                                        (d["bar_idx"].as_u64(), d["price"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::PriceLabel {
                                            bar_idx: idx as usize,
                                            price: p,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("callout") => {
                                    if let (Some(a), Some(lp), Some(t)) = (
                                        parse_pt(d, "anchor"),
                                        parse_pt(d, "label_pos"),
                                        d["text"].as_str(),
                                    ) {
                                        chart.drawings.push(Drawing::Callout {
                                            anchor: a,
                                            label_pos: lp,
                                            text: t.to_string(),
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("highlighter") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::Highlighter {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("crossmarker") => {
                                    if let (Some(idx), Some(p)) =
                                        (d["bar_idx"].as_u64(), d["price"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::CrossMarker {
                                            bar_idx: idx as usize,
                                            price: p,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("polyline") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::Polyline {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("anchornote") => {
                                    if let (Some(idx), Some(p), Some(t)) = (
                                        d["bar_idx"].as_u64(),
                                        d["price"].as_f64(),
                                        d["text"].as_str(),
                                    ) {
                                        chart.drawings.push(Drawing::AnchorNote {
                                            bar_idx: idx as usize,
                                            price: p,
                                            text: t.to_string(),
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("regressionch") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::RegressionChannel {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("gannbox") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::GannBox {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("elliott") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::ElliottWave {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("abc") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::AbcCorrection {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("daterange") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::DateRange { p1, p2 });
                                    }
                                }
                                Some("datepricerange") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::DatePriceRange { p1, p2 });
                                    }
                                }
                                Some("headshoulders") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::HeadShoulders {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("xabcd") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::XabcdPattern {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("brush") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::Brush {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("schiffpitchfork") => {
                                    if let (Some(pv), Some(p2), Some(p3)) =
                                        (parse_pt(d, "pivot"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::SchiffPitchfork {
                                            pivot: pv,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("modschiffpitchfork") => {
                                    if let (Some(pv), Some(p2), Some(p3)) =
                                        (parse_pt(d, "pivot"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::ModSchiffPitchfork {
                                            pivot: pv,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("cycliclines") => {
                                    if let (Some(bs), Some(be)) =
                                        (d["bar_start"].as_u64(), d["bar_end"].as_u64())
                                    {
                                        chart.drawings.push(Drawing::CyclicLines {
                                            bar_start: bs as usize,
                                            bar_end: be as usize,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("sinewave") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::SineWave {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("emoji") => {
                                    if let (Some(idx), Some(p)) =
                                        (d["bar_idx"].as_u64(), d["price"].as_f64())
                                    {
                                        let emoji =
                                            d["emoji"].as_str().unwrap_or("\u{1F3AF}").to_string();
                                        chart.drawings.push(Drawing::Emoji {
                                            bar_idx: idx as usize,
                                            price: p,
                                            emoji,
                                        });
                                    }
                                }
                                Some("flag") => {
                                    if let (Some(idx), Some(p)) =
                                        (d["bar_idx"].as_u64(), d["price"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::Flag {
                                            bar_idx: idx as usize,
                                            price: p,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("balloon") => {
                                    if let (Some(a), Some(lp), Some(t)) = (
                                        parse_pt(d, "anchor"),
                                        parse_pt(d, "label_pos"),
                                        d["text"].as_str(),
                                    ) {
                                        chart.drawings.push(Drawing::Balloon {
                                            anchor: a,
                                            label_pos: lp,
                                            text: t.to_string(),
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("sessionbreak") => {
                                    if let Some(idx) = d["bar_idx"].as_u64() {
                                        chart.drawings.push(Drawing::SessionBreak {
                                            bar_idx: idx as usize,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("magnetlevel") => {
                                    if let Some(price) = d["price"].as_f64() {
                                        chart.drawings.push(Drawing::MagnetLevel {
                                            price,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("riskreward") => {
                                    if let (Some(e), Some(s), Some(t)) = (
                                        parse_pt(d, "entry"),
                                        d["stop"].as_f64(),
                                        d["target"].as_f64(),
                                    ) {
                                        chart.drawings.push(Drawing::RiskRewardBox {
                                            entry: e,
                                            stop: s,
                                            target: t,
                                        });
                                    }
                                }
                                Some("fibcircle") => {
                                    if let (Some(c), Some(r)) =
                                        (parse_pt(d, "center"), parse_pt(d, "radius_pt"))
                                    {
                                        chart.drawings.push(Drawing::FibCircle {
                                            center: c,
                                            radius_pt: r,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("arcdraw") => {
                                    if let (Some(p1), Some(p2), Some(p3)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::ArcDraw {
                                            p1,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("curvedraw") => {
                                    if let (Some(p1), Some(c1), Some(c2), Some(p2)) = (
                                        parse_pt(d, "p1"),
                                        parse_pt(d, "ctrl1"),
                                        parse_pt(d, "ctrl2"),
                                        parse_pt(d, "p2"),
                                    ) {
                                        chart.drawings.push(Drawing::CurveDraw {
                                            p1,
                                            ctrl1: c1,
                                            ctrl2: c2,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("pathdraw") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::PathDraw {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("forecast") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::Forecast {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("ghostfeed") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::GhostFeed {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("signpost") => {
                                    if let (Some(idx), Some(p)) =
                                        (d["bar_idx"].as_u64(), d["price"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::Signpost {
                                            bar_idx: idx as usize,
                                            price: p,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("ruler") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::Ruler {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("timecycle") => {
                                    if let (Some(bs), Some(be)) =
                                        (d["bar_start"].as_u64(), d["bar_end"].as_u64())
                                    {
                                        chart.drawings.push(Drawing::TimeCycle {
                                            bar_start: bs as usize,
                                            bar_end: be as usize,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("speedfan") => {
                                    if let (Some(p1), Some(p2), Some(p3)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::SpeedResistanceFan {
                                            p1,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("speedarc") => {
                                    if let (Some(p1), Some(p2), Some(p3)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::SpeedResistanceArc {
                                            p1,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("fibspiral") => {
                                    if let (Some(c), Some(r)) =
                                        (parse_pt(d, "center"), parse_pt(d, "radius_pt"))
                                    {
                                        chart.drawings.push(Drawing::FibSpiral {
                                            center: c,
                                            radius_pt: r,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("rotatedrect") => {
                                    if let (Some(p1), Some(p2), Some(p3)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::RotatedRectangle {
                                            p1,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("anchoredvwap") => {
                                    if let Some(idx) = d["bar_idx"].as_u64() {
                                        chart.drawings.push(Drawing::AnchoredVwapLine {
                                            bar_idx: idx as usize,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("trendchannel") => {
                                    if let (Some(p1), Some(p2), Some(p3)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::TrendChannel {
                                            p1,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("insidepitchfork") => {
                                    if let (Some(pv), Some(p2), Some(p3)) =
                                        (parse_pt(d, "pivot"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::InsidePitchfork {
                                            pivot: pv,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("fibwedge") => {
                                    if let (Some(p1), Some(p2), Some(p3)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"), parse_pt(d, "p3"))
                                    {
                                        chart.drawings.push(Drawing::FibWedge {
                                            p1,
                                            p2,
                                            p3,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("pricenote") => {
                                    if let (Some(p), Some(t)) =
                                        (d["price"].as_f64(), d["text"].as_str())
                                    {
                                        chart.drawings.push(Drawing::PriceNote {
                                            price: p,
                                            text: t.to_string(),
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("measuretool") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::MeasureTool {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("anchoredtext") => {
                                    if let (Some(idx), Some(p), Some(t)) = (
                                        d["bar_idx"].as_u64(),
                                        d["price"].as_f64(),
                                        d["text"].as_str(),
                                    ) {
                                        chart.drawings.push(Drawing::AnchoredText {
                                            bar_idx: idx as usize,
                                            price: p,
                                            text: t.to_string(),
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("comment") => {
                                    if let (Some(idx), Some(p), Some(t)) = (
                                        d["bar_idx"].as_u64(),
                                        d["price"].as_f64(),
                                        d["text"].as_str(),
                                    ) {
                                        chart.drawings.push(Drawing::Comment {
                                            bar_idx: idx as usize,
                                            price: p,
                                            text: t.to_string(),
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("arrowleft") => {
                                    if let (Some(idx), Some(p)) =
                                        (d["bar_idx"].as_u64(), d["price"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::ArrowMarkerLeft {
                                            bar_idx: idx as usize,
                                            price: p,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("arrowright") => {
                                    if let (Some(idx), Some(p)) =
                                        (d["bar_idx"].as_u64(), d["price"].as_f64())
                                    {
                                        chart.drawings.push(Drawing::ArrowMarkerRight {
                                            bar_idx: idx as usize,
                                            price: p,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("circle") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::Circle {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("pitchfan") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::PitchFan {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("trendfibtime") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::TrendFibTime {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("gannsquare") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::GannSquare {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("gannsquarefixed") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::GannSquareFixed {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("barspattern") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::BarsPattern {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("projection") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::Projection {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("doublecurve") => {
                                    if let (Some(p1), Some(p2)) =
                                        (parse_pt(d, "p1"), parse_pt(d, "p2"))
                                    {
                                        chart.drawings.push(Drawing::DoubleCurve {
                                            p1,
                                            p2,
                                            color: parse_col(d),
                                        });
                                    }
                                }
                                Some("trianglepattern") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::TrianglePattern {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("threedrives") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::ThreeDrives {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("elliottdouble") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::ElliottDouble {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("abcd") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::AbcdPattern {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("cypher") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::CypherPattern {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("elliotttriangle") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::ElliottTriangle {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                Some("elliotttriple") => {
                                    if let Some(pts) = d["points"].as_array() {
                                        let points: Vec<(usize, f64)> = pts
                                            .iter()
                                            .filter_map(|p| {
                                                let a = p.as_array()?;
                                                Some((
                                                    a.first()?.as_u64()? as usize,
                                                    a.get(1)?.as_f64()?,
                                                ))
                                            })
                                            .collect();
                                        if !points.is_empty() {
                                            chart.drawings.push(Drawing::ElliottTripleCombo {
                                                points,
                                                color: parse_col(d),
                                            });
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                // Restore alerts
                if let Some(alerts) = v["alerts"].as_array() {
                    for a in alerts {
                        if let (Some(p), Some(l)) = (a["price"].as_f64(), a["label"].as_str()) {
                            let pair = (p, l.to_string());
                            let key = format!("{:.8}|{}", p, l);
                            if self.alerts_set.insert(key) {
                                self.alerts.push(pair);
                            }
                        }
                    }
                }
                // Restore chart templates
                if let Some(templates) = v["chart_templates"].as_object() {
                    for (name, snap) in templates {
                        self.chart_templates.insert(name.clone(), snap.clone());
                    }
                }
                // Restore MTF cols
                if let Some(cols) = v["mtf_cols"].as_u64() {
                    self.mtf_cols = cols as usize;
                }
                if let Some(b) = v["fund_source_alpaca"].as_bool() {
                    self.fund_source_alpaca = b;
                }
                if let Some(b) = v["fund_source_kraken"].as_bool() {
                    self.fund_source_kraken = b;
                }
                // Restore right panel tab
                self.right_tab = match v["right_tab"].as_str() {
                    Some("positions") => RightTab::Positions,
                    Some("orders") => RightTab::Orders,
                    Some("watchlist") => RightTab::Watchlist,
                    Some("risk") => RightTab::Risk,
                    _ => RightTab::Trading,
                };
                if let Some(b) = v["right_trading_open"].as_bool() {
                    self.right_trading_open = b;
                }
                if let Some(b) = v["right_positions_open"].as_bool() {
                    self.right_positions_open = b;
                }
                if let Some(b) = v["right_orders_open"].as_bool() {
                    self.right_orders_open = b;
                }
                if let Some(b) = v["right_watchlist_open"].as_bool() {
                    self.right_watchlist_open = b;
                }
                if let Some(b) = v["right_risk_open"].as_bool() {
                    self.right_risk_open = b;
                }
                if let Some(b) = v["right_recent_fills_open"].as_bool() {
                    self.right_recent_fills_open = b;
                }
                if let Some(b) = v["right_news_open"].as_bool() {
                    self.right_news_open = b;
                }
                if let Some(s) = v["news_search_query"].as_str() {
                    self.news_search_query = s.to_string();
                }
                if let Some(s) = v["news_selected_url_hash"].as_str() {
                    self.news_selected_url_hash = s.to_string();
                }
                if let Some(b) = v["right_mtf_grid_open"].as_bool() {
                    self.right_mtf_grid_open = b;
                }
                if let Some(order) = v["right_panel_order"].as_array() {
                    self.right_panel_order = order
                        .iter()
                        .filter_map(|value| value.as_str())
                        .filter_map(RightPanelSectionId::from_str)
                        .collect();
                    self.normalized_right_panel_order();
                }
                if let Some(model) = v["codex_model"].as_str() {
                    self.codex_model = model.to_string();
                }
                if let Some(effort) = v["codex_reasoning_effort"].as_str() {
                    self.codex_reasoning_effort =
                        Self::normalize_codex_reasoning_effort(effort).to_string();
                }
                if let Some(model) = v["hermes_model"].as_str() {
                    self.hermes_model = model.to_string();
                }
                if let Some(provider) = v["hermes_provider"].as_str() {
                    self.hermes_provider = provider.to_string();
                }
                if let Some(model) = v["grok_model"].as_str() {
                    self.grok_model = model.to_string();
                }
                if let Some(effort) = v["grok_effort"].as_str() {
                    self.grok_effort = Self::normalize_grok_effort(effort).to_string();
                }
                // Migration fallback: load credentials from old session.json if keyring is empty.
                // Secrets are no longer written to session.json (see save_session).
                // Once a session has been saved under the new code these keys will be absent.
                if self.finnhub_key.is_empty() {
                    if let Some(fk) = v["finnhub_key"].as_str() {
                        self.finnhub_key = fk.to_string();
                    }
                }
                if self.fred_key.is_empty() {
                    if let Some(fk) = v["fred_key"].as_str() {
                        self.fred_key = fk.to_string();
                    }
                }
                if self.broker_api_key.is_empty() {
                    if let Some(ak) = v["broker_api_key"].as_str() {
                        self.broker_api_key = ak.to_string();
                    }
                }
                if self.broker_secret.is_empty() {
                    if let Some(bs) = v["broker_secret"].as_str() {
                        self.broker_secret = bs.to_string();
                    }
                }
                if let Some(enabled) = v["alpaca_enabled"].as_bool() {
                    self.alpaca_enabled = enabled;
                }
                if let Some(enabled) = v["alpaca_full_bar_sync_enabled"].as_bool() {
                    self.alpaca_full_bar_sync_enabled = enabled;
                }
                if let Some(enabled) = v["kraken_full_bar_sync_enabled"].as_bool() {
                    self.kraken_full_bar_sync_enabled = enabled;
                }
                if let Some(enabled) = v["kraken_enabled"].as_bool() {
                    self.kraken_enabled = enabled;
                }
                if let Some(enabled) = v["backfill_alpaca_kraken_equities_enabled"].as_bool() {
                    self.backfill_alpaca_kraken_equities_enabled = enabled;
                }
                if let Some(enabled) = v["backfill_yahoo_chart_enabled"].as_bool() {
                    self.backfill_yahoo_chart_enabled = enabled;
                }

                if let Some(bp) = v["broker_paper"].as_bool() {
                    self.broker_paper = bp;
                }
                // Restore user watchlist
                if let Some(wl) = v["user_watchlist"].as_array() {
                    self.user_watchlist = wl
                        .iter()
                        .filter_map(|s| s.as_str().map(String::from))
                        .collect();
                    self.user_watchlist_set = self.user_watchlist.iter().cloned().collect();
                }
                if let Some(obj) = v["workspaces"].as_object() {
                    self.workspaces = obj
                        .iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect();
                }
                if let Some(b) = v["show_alpaca_positions"].as_bool() {
                    self.show_alpaca_positions = b;
                }
                if let Some(b) = v["show_kr_positions"].as_bool() {
                    self.show_kr_positions = b;
                }
                if let Some(b) = v["snap_enabled"].as_bool() {
                    self.snap_enabled = b;
                }
                if let Some(b) = v["cross_tf_drawings"].as_bool() {
                    self.cross_tf_drawings = b;
                }
                if let Some(b) = v["follow_latest"].as_bool() {
                    self.follow_latest = b;
                }
                if let Some(w) = v["draw_width"].as_f64() {
                    self.draw_width = w as f32;
                }
                if let Some(arr) = v["draw_color"].as_array() {
                    if arr.len() == 3 {
                        let r = arr[0].as_u64().unwrap_or(0) as u8;
                        let g = arr[1].as_u64().unwrap_or(188) as u8;
                        let b = arr[2].as_u64().unwrap_or(212) as u8;
                        self.draw_color = egui::Color32::from_rgb(r, g, b);
                    }
                }
                if let Some(s) = v["draw_line_style"].as_str() {
                    self.draw_line_style = match s {
                        "dashed" => LineStyle::Dashed,
                        "dotted" => LineStyle::Dotted,
                        _ => LineStyle::Solid,
                    };
                }
                // Restore SL/TP state
                if let Some(sl) = v["sl_enabled"].as_bool() {
                    self.sl_enabled = sl;
                }
                if let Some(tp) = v["tp_enabled"].as_bool() {
                    self.tp_enabled = tp;
                }
                // Restore window visibility
                if let Some(w) = v.get("windows") {
                    if let Some(b) = w["settings"].as_bool() {
                        self.show_settings = b;
                    }
                    if let Some(b) = w["risk_calc"].as_bool() {
                        self.show_risk_calc = b;
                    }
                    if let Some(b) = w["compound_calc"].as_bool() {
                        self.show_compound_calc = b;
                    }
                    if let Some(b) = w["calendar"].as_bool() {
                        self.show_calendar = b;
                    }
                    if let Some(b) = w["backtest"].as_bool() {
                        self.show_backtest = b;
                    }
                    if let Some(b) = w["news"].as_bool() {
                        self.show_news = b;
                    }
                    if let Some(b) = w["indicators_panel"].as_bool() {
                        self.show_indicators_panel = b;
                    }
                    if let Some(b) = w["screener"].as_bool() {
                        self.show_screener = b;
                    }
                    if let Some(b) = w["symbols"].as_bool() {
                        self.show_symbols = b;
                    }
                    if let Some(b) = w["optimizer"].as_bool() {
                        self.show_optimizer = b;
                    }
                    if let Some(b) = w["ai_chat"].as_bool() {
                        self.show_ai_chat = b;
                    }
                    if let Some(b) = w["claude_code"].as_bool() {
                        self.show_claude_code = b;
                    }
                    if let Some(b) = w["gemini_cli"].as_bool() {
                        self.show_gemini_cli = b;
                    }
                    if let Some(b) = w["codex_cli"].as_bool() {
                        self.show_codex_cli = b;
                    }
                    if let Some(b) = w["hermes_cli"].as_bool() {
                        self.show_hermes_cli = b;
                    }
                    if let Some(b) = w["grok_cli"].as_bool() {
                        self.show_grok_cli = b;
                    }
                    if let Some(b) = w["matrix_chat"].as_bool() {
                        self.show_matrix_chat = b;
                    }
                    if let Some(b) = w["sec"].as_bool() {
                        self.show_sec = b;
                    }
                    if let Some(b) = w["insider"].as_bool() {
                        self.show_insider = b;
                    }
                    if let Some(b) = w["fundamentals"].as_bool() {
                        self.show_fundamentals = b;
                    }
                    if let Some(b) = w["order_flow"].as_bool() {
                        self.show_order_flow = b;
                    }
                    if let Some(b) = w["bookmap"].as_bool() {
                        self.show_bookmap = b;
                    }
                    if let Some(b) = w["journal"].as_bool() {
                        self.show_journal = b;
                    }
                    if let Some(b) = w["var_mult"].as_bool() {
                        self.show_var_mult = b;
                    }
                    if let Some(b) = w["montecarlo"].as_bool() {
                        self.show_montecarlo = b;
                    }
                    if let Some(b) = w["earnings_calendar"].as_bool() {
                        self.show_earnings_calendar = b;
                    }
                    if let Some(b) = w["dividend_calendar"].as_bool() {
                        self.show_dividend_calendar = b;
                    }
                    if let Some(b) = w["event_calendar"].as_bool() {
                        self.show_event_calendar = b;
                    }
                    if let Some(b) = w["ev_scanner"].as_bool() {
                        self.show_ev_scanner = b;
                    }
                    if let Some(b) = w["stress_test"].as_bool() {
                        self.show_stress_test = b;
                    }
                    if let Some(b) = w["volume_profile"].as_bool() {
                        self.show_volume_profile = b;
                    }
                    if let Some(b) = w["hv_cone"].as_bool() {
                        self.show_hv_cone = b;
                    }
                    if let Some(b) = w["sector_heatmap"].as_bool() {
                        self.show_sector_heatmap = b;
                    }
                    if let Some(b) = w["dividends_screen"].as_bool() {
                        self.show_dividends = b;
                    }
                    if let Some(b) = w["company_info"].as_bool() {
                        self.show_company_info_window = b;
                    }
                    if let Some(b) = w["alert_builder"].as_bool() {
                        self.show_alert_builder = b;
                    }
                    if let Some(b) = w["storage"].as_bool() {
                        self.show_storage = b;
                    }
                    if let Some(b) = w["sync_status"].as_bool() {
                        self.show_sync_status = b;
                    }
                    if let Some(b) = w["unusual_volume"].as_bool() {
                        self.show_unusual_volume = b;
                    }
                    if let Some(b) = w["sector_rotation"].as_bool() {
                        self.show_sector_rotation = b;
                    }
                    if let Some(b) = w["fred"].as_bool() {
                        self.show_fred = b;
                    }
                    if let Some(b) = w["econ_calendar"].as_bool() {
                        self.show_econ_calendar = b;
                    }
                    if let Some(b) = w["congress"].as_bool() {
                        self.show_congress = b;
                    }
                    if let Some(b) = w["world_indices"].as_bool() {
                        self.show_world_indices = b;
                    }
                    if let Some(b) = w["crypto_top50"].as_bool() {
                        self.show_crypto_top50 = b;
                    }
                    if let Some(b) = w["forex_matrix"].as_bool() {
                        self.show_forex_matrix = b;
                    }
                    if let Some(b) = w["help"].as_bool() {
                        self.show_help = b;
                    }
                    if let Some(b) = w["connect"].as_bool() {
                        self.show_connect = b;
                    }
                    if let Some(b) = w["data_window"].as_bool() {
                        self.show_data_window = b;
                    }
                    if let Some(b) = w["alerts"].as_bool() {
                        self.show_alerts = b;
                    }
                    if let Some(b) = w["scope_window"].as_bool() {
                        self.show_scope_window = b;
                    }
                    if let Some(b) = w["scrape_status"].as_bool() {
                        self.show_scrape_status = b;
                    }
                    if let Some(b) = w["fear_greed"].as_bool() {
                        self.show_fear_greed = b;
                    }
                }
                // Restore journal entries
                if let Some(journal) = v["journal"].as_array() {
                    for entry in journal {
                        self.journal_entries.push(JournalEntry {
                            timestamp: entry["timestamp"].as_str().unwrap_or("").to_string(),
                            symbol: entry["symbol"].as_str().unwrap_or("").to_string(),
                            side: entry["side"].as_str().unwrap_or("BUY").to_string(),
                            qty: entry["qty"].as_f64().unwrap_or(1.0),
                            entry_price: entry["entry_price"].as_f64().unwrap_or(0.0),
                            exit_price: entry["exit_price"].as_f64(),
                            pnl: entry["pnl"].as_f64(),
                            strategy: entry["strategy"].as_str().unwrap_or("").to_string(),
                            notes: entry["notes"].as_str().unwrap_or("").to_string(),
                        });
                    }
                }
                self.log.push_back(LogEntry::info("Session restored"));
            }
        }
        self.sync_preferences_load();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RightPanelSectionId, controls::reordered_right_panel_sections,
        sync_preferences::persisted_bar_zstd_level,
    };

    #[test]
    fn persisted_bar_zstd_level_uses_saved_value() {
        let value = serde_json::json!({ "bar_zstd_level": 9 });
        assert_eq!(persisted_bar_zstd_level(&value, 3), 9);
    }

    #[test]
    fn persisted_bar_zstd_level_clamps_saved_value() {
        let high = serde_json::json!({ "bar_zstd_level": 999 });
        let low = serde_json::json!({ "bar_zstd_level": -99 });
        assert_eq!(
            persisted_bar_zstd_level(&high, 3),
            typhoon_engine::core::cache::MAX_ZSTD_LEVEL
        );
        assert_eq!(
            persisted_bar_zstd_level(&low, 3),
            typhoon_engine::core::cache::MIN_ZSTD_LEVEL
        );
    }

    #[test]
    fn persisted_bar_zstd_level_keeps_current_when_missing() {
        let value = serde_json::json!({});
        assert_eq!(persisted_bar_zstd_level(&value, 11), 11);
    }

    #[test]
    fn right_panel_reorder_moves_dragged_section_before_or_after_target() {
        use RightPanelSectionId::{
            MtfGrid, News, Orders, Positions, RecentFills, Risk, Trading, Watchlist,
        };

        let order = vec![
            Trading,
            Positions,
            RecentFills,
            Orders,
            Watchlist,
            Risk,
            News,
            MtfGrid,
        ];

        assert_eq!(
            reordered_right_panel_sections(&order, Watchlist, Positions, false).unwrap(),
            vec![
                Trading,
                Watchlist,
                Positions,
                RecentFills,
                Orders,
                Risk,
                News,
                MtfGrid
            ]
        );
        assert_eq!(
            reordered_right_panel_sections(&order, Watchlist, Positions, true).unwrap(),
            vec![
                Trading,
                Positions,
                Watchlist,
                RecentFills,
                Orders,
                Risk,
                News,
                MtfGrid
            ]
        );
        assert_eq!(
            reordered_right_panel_sections(&order, Positions, Watchlist, true).unwrap(),
            vec![
                Trading,
                RecentFills,
                Orders,
                Watchlist,
                Positions,
                Risk,
                News,
                MtfGrid
            ]
        );
    }

    #[test]
    fn right_panel_reorder_ignores_noop_and_missing_targets() {
        use RightPanelSectionId::{Positions, Trading, Watchlist};

        let order = vec![Trading, Positions];
        assert!(reordered_right_panel_sections(&order, Trading, Trading, false).is_none());
        assert!(reordered_right_panel_sections(&order, Watchlist, Trading, false).is_none());
        assert!(reordered_right_panel_sections(&order, Trading, Watchlist, false).is_none());
    }
}
