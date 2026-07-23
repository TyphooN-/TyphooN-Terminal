use super::*;

impl TyphooNApp {
    pub(super) fn render_market_analytics_windows(&mut self, ctx: &egui::Context) {
        // Dividend Calendar
        if self.show_dividend_calendar {
            let filter_active = research_sort_indices::active_only_filter_enabled(
                self.dividends_active_only,
                self.cached_active_symbols.len(),
            );
            let mut dc_pending_action = SymbolAction::None;
            egui::Window::new("Dividend Calendar")
                .open(&mut self.show_dividend_calendar)
                .resizable(true)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} upcoming dividends",
                                self.bg.upcoming_dividends.len()
                            ))
                            .color(AXIS_TEXT)
                            .small(),
                        );
                        ui.checkbox(
                            &mut self.dividends_active_only,
                            egui::RichText::new("Active Only").small(),
                        );
                    });
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            egui::Grid::new("div_cal_grid")
                                .striped(true)
                                .num_columns(4)
                                .show(ui, |ui| {
                                    ui.strong("Ex-Div Date");
                                    ui.strong("Symbol");
                                    ui.strong("Company");
                                    ui.strong("Yield%");
                                    ui.end_row();
                                    for (sym, company, date, yld) in &self.bg.upcoming_dividends {
                                        // PERF: fundamentals.symbol is always uppercase.
                                        if filter_active
                                            && !self
                                                .cached_active_symbols_set
                                                .contains(sym.as_str())
                                        {
                                            continue;
                                        }
                                        ui.label(
                                            egui::RichText::new(date).color(AXIS_TEXT).small(),
                                        );
                                        let (_, dc_action) = symbol_label_with_menu(
                                            ui,
                                            sym,
                                            egui::RichText::new(sym).small().strong().monospace(),
                                        );
                                        if !matches!(dc_action, SymbolAction::None) {
                                            dc_pending_action = dc_action;
                                        }
                                        ui.label(egui::RichText::new(company).small());
                                        let y = yld.unwrap_or(0.0);
                                        let y_col = if y > 4.0 { UP } else { AXIS_TEXT };
                                        ui.label(
                                            egui::RichText::new(format!("{:.2}%", y))
                                                .color(y_col)
                                                .small(),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                });
            self.apply_symbol_action(dc_pending_action);
        }

        // Analyst — wired to Finnhub recommendations
        if self.show_analyst {
            egui::Window::new("Analyst Ratings")
                .open(&mut self.show_analyst)
                .resizable(true)
                .default_size([480.0, 340.0])
                .show(ctx, |ui| {
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
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("Analyst: {}", sym)).strong());
                        if ui.button("Fetch Ratings").clicked()
                            && !sym.is_empty()
                            && !self.finnhub_key.is_empty()
                        {
                            let _ = self.broker_tx.send(BrokerCmd::GetAnalyst {
                                symbol: sym.clone(),
                                finnhub_key: self.finnhub_key.clone(),
                            });
                            self.log.push_back(LogEntry::info(format!(
                                "Fetching analyst ratings for {}...",
                                sym
                            )));
                        }
                        if self.finnhub_key.is_empty() {
                            ui.label(
                                egui::RichText::new("(add Finnhub key in Settings)")
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                        }
                    });
                    ui.separator();
                    if self.analyst_result.is_empty() {
                        ui.label(
                            egui::RichText::new("No data — click Fetch Ratings.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else if let Ok(arr) =
                        serde_json::from_str::<serde_json::Value>(&self.analyst_result)
                    {
                        if let Some(recs) = arr.as_array() {
                            egui::ScrollArea::vertical()
                                .auto_shrink(false)
                                .max_height(260.0)
                                .show(ui, |ui| {
                                    egui::Grid::new("analyst_grid")
                                        .striped(true)
                                        .num_columns(7)
                                        .show(ui, |ui| {
                                            ui.strong("Period");
                                            ui.strong("StrongBuy");
                                            ui.strong("Buy");
                                            ui.strong("Hold");
                                            ui.strong("Sell");
                                            ui.strong("StrongSell");
                                            ui.strong("Consensus");
                                            ui.end_row();
                                            for rec in recs.iter().take(12) {
                                                let period = rec["period"].as_str().unwrap_or("—");
                                                let sb = rec["strongBuy"].as_i64().unwrap_or(0);
                                                let b = rec["buy"].as_i64().unwrap_or(0);
                                                let h = rec["hold"].as_i64().unwrap_or(0);
                                                let s = rec["sell"].as_i64().unwrap_or(0);
                                                let ss = rec["strongSell"].as_i64().unwrap_or(0);
                                                let buy_total = sb + b;
                                                let sell_total = s + ss;
                                                let consensus = if buy_total > sell_total + h {
                                                    "BUY"
                                                } else if sell_total > buy_total + h {
                                                    "SELL"
                                                } else {
                                                    "HOLD"
                                                };
                                                let con_color = match consensus {
                                                    "BUY" => UP,
                                                    "SELL" => DOWN,
                                                    _ => egui::Color32::from_rgb(200, 180, 50),
                                                };
                                                ui.label(period);
                                                ui.label(
                                                    egui::RichText::new(sb.to_string()).color(UP),
                                                );
                                                ui.label(
                                                    egui::RichText::new(b.to_string()).color(UP),
                                                );
                                                ui.label(h.to_string());
                                                ui.label(
                                                    egui::RichText::new(s.to_string()).color(DOWN),
                                                );
                                                ui.label(
                                                    egui::RichText::new(ss.to_string()).color(DOWN),
                                                );
                                                ui.label(
                                                    egui::RichText::new(consensus)
                                                        .color(con_color)
                                                        .strong(),
                                                );
                                                ui.end_row();
                                            }
                                        });
                                });
                        }
                    } else {
                        ui.label(
                            egui::RichText::new("Failed to parse analyst data.")
                                .color(DOWN)
                                .small(),
                        );
                    }
                    // Price target section (appended via PriceTarget: routing)
                    if let Some(pt_start) = self.analyst_result.find("---PRICE_TARGET---") {
                        let pt_json = &self.analyst_result[pt_start + 18..];
                        if let Ok(pt) = serde_json::from_str::<serde_json::Value>(pt_json.trim()) {
                            ui.separator();
                            ui.label(egui::RichText::new("Price Target").small().strong());
                            egui::Grid::new("pt_grid").num_columns(2).show(ui, |ui| {
                                if let Some(h) = pt["targetHigh"].as_f64() {
                                    ui.label("High:");
                                    ui.label(egui::RichText::new(format!("${:.2}", h)).color(UP));
                                    ui.end_row();
                                }
                                if let Some(m) = pt["targetMedian"].as_f64() {
                                    ui.label("Median:");
                                    ui.label(format!("${:.2}", m));
                                    ui.end_row();
                                }
                                if let Some(l) = pt["targetLow"].as_f64() {
                                    ui.label("Low:");
                                    ui.label(egui::RichText::new(format!("${:.2}", l)).color(DOWN));
                                    ui.end_row();
                                }
                                if let Some(m) = pt["targetMean"].as_f64() {
                                    ui.label("Mean:");
                                    ui.label(format!("${:.2}", m));
                                    ui.end_row();
                                }
                                if let Some(n) = pt["numberOfAnalysts"].as_i64() {
                                    ui.label("Analysts:");
                                    ui.label(n.to_string());
                                    ui.end_row();
                                }
                            });
                        }
                    }
                });
        }

        // Holders — wired to SEC EDGAR 13F
        if self.show_holders {
            egui::Window::new("Institutional Holders")
                .open(&mut self.show_holders)
                .resizable(true)
                .default_size([560.0, 400.0])
                .show(ctx, |ui| {
                    let ticker = self
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
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("Holders: {}", ticker)).strong());
                        if !ticker.is_empty()
                            && ui
                                .small_button(egui::RichText::new("+").small())
                                .on_hover_text("Open new chart")
                                .clicked()
                        {
                            self.deferred_symbol_action = SymbolAction::OpenChart(ticker.clone());
                        }
                        if ui.button("Fetch 13F").clicked() && !ticker.is_empty() {
                            let _ = self.broker_tx.send(BrokerCmd::GetHolders {
                                ticker: ticker.clone(),
                            });
                            self.log.push_back(LogEntry::info(format!(
                                "Fetching 13F holders for {}...",
                                ticker
                            )));
                        }
                    });
                    ui.separator();
                    if self.holders_result.is_empty() {
                        ui.label(
                            egui::RichText::new("No data — click Fetch 13F.")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    } else if let Ok(v) =
                        serde_json::from_str::<serde_json::Value>(&self.holders_result)
                    {
                        // Entity summary
                        egui::Grid::new("holders_meta")
                            .num_columns(2)
                            .show(ui, |ui| {
                                if let Some(s) = v["entity_name"].as_str() {
                                    ui.strong("Entity:");
                                    ui.label(s);
                                    ui.end_row();
                                }
                                if let Some(s) =
                                    v["sic_description"].as_str().filter(|s| !s.is_empty())
                                {
                                    ui.strong("SIC:");
                                    ui.label(s);
                                    ui.end_row();
                                }
                                if let Some(s) = v["state_of_incorporation"]
                                    .as_str()
                                    .filter(|s| !s.is_empty())
                                {
                                    ui.strong("State:");
                                    ui.label(s);
                                    ui.end_row();
                                }
                                if let Some(s) =
                                    v["fiscal_year_end"].as_str().filter(|s| !s.is_empty())
                                {
                                    ui.strong("FY End:");
                                    ui.label(s);
                                    ui.end_row();
                                }
                            });
                        ui.separator();
                        let count = v["total_13f_found"].as_u64().unwrap_or(0);
                        ui.label(
                            egui::RichText::new(format!("{} 13F filings found (SEC EDGAR)", count))
                                .small(),
                        );
                        if let Some(filings) = v["filings_13f"].as_array() {
                            egui::ScrollArea::vertical()
                                .auto_shrink(false)
                                .max_height(240.0)
                                .show(ui, |ui| {
                                    egui::Grid::new("holders_grid")
                                        .striped(true)
                                        .num_columns(3)
                                        .show(ui, |ui| {
                                            ui.strong("Form");
                                            ui.strong("Filed");
                                            ui.strong("Accession");
                                            ui.end_row();
                                            for f in filings.iter() {
                                                let form = f["form"].as_str().unwrap_or("—");
                                                let date = f["filing_date"].as_str().unwrap_or("—");
                                                let acc =
                                                    f["accession_number"].as_str().unwrap_or("—");
                                                ui.label(form);
                                                ui.label(date);
                                                ui.label(
                                                    egui::RichText::new(acc).monospace().small(),
                                                );
                                                ui.end_row();
                                            }
                                        });
                                });
                        }
                    } else {
                        ui.label(
                            egui::RichText::new("Failed to parse holders data.")
                                .color(DOWN)
                                .small(),
                        );
                    }
                });
        }

        // Option Chain — option expirations from KV cache
        if self.show_option_chain {
            egui::Window::new("Option Chain")
                        .open(&mut self.show_option_chain)
                        .resizable(true).default_size([560.0, 440.0])
                        .show(ctx, |ui| {
                            let sym = &self.option_chain_sym;
                            let oc_green = egui::Color32::from_rgb(0, 200, 80);
                            let oc_red   = egui::Color32::from_rgb(220, 50, 50);
                            let oc_dim   = egui::Color32::from_rgb(80, 80, 100);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(format!("Option Chain: {}", sym)).strong());
                                let _ = oc_dim;
                            });
                            ui.separator();

                            // Load from KV cache
                            let chain_json = self.cache.as_ref()
                                .and_then(|c| c.get_kv(&format!("tt:options:{}", sym)).ok().flatten());

                            if let Some(json) = chain_json {
                                if let Ok(expirations) = serde_json::from_str::<serde_json::Value>(&json) {
                                    if let Some(arr) = expirations.as_array() {
                                        ui.label(egui::RichText::new(format!("{} expirations", arr.len())).small());
                                        // Fill remaining height so enlarging the window shows more expirations.
                                        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                                            for exp in arr.iter() {
                                                let date = exp["expiration_date"].as_str().unwrap_or("?");
                                                let strikes = exp["strikes"].as_array();
                                                let strike_count = strikes.map(|s| s.len()).unwrap_or(0);
                                                let header = format!("{} ({} strikes)", date, strike_count);
                                                egui::CollapsingHeader::new(
                                                    egui::RichText::new(header).small()
                                                )
                                                .id_salt(date)
                                                .show(ui, |ui| {
                                                    if let Some(strikes) = strikes {
                                                        egui::Grid::new(format!("oc_{}", date))
                                                            .striped(true).num_columns(7)
                                                            .show(ui, |ui| {
                                                            ui.strong("Strike");
                                                            ui.strong("Call");
                                                            ui.strong("Put");
                                                            ui.strong("Delta");
                                                            ui.strong("Gamma");
                                                            ui.strong("Theta");
                                                            ui.strong("Vega");
                                                            ui.end_row();
                                                            // Compute Greeks for each strike
                                                            let spot = self.live_account.as_ref().map(|_| {
                                                                // Use last price from watchlist or chart
                                                                self.watchlist_rows.iter()
                                                                    .find(|r| r.symbol.eq_ignore_ascii_case(sym))
                                                                    .map(|r| r.last)
                                                                    .unwrap_or(0.0)
                                                            }).unwrap_or(0.0);
                                                            let days_to_exp = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()
                                                                .map(|exp| {
                                                                    let today = chrono::Utc::now().date_naive();
                                                                    (exp - today).num_days().max(1) as f64
                                                                }).unwrap_or(30.0);
                                                            let t_years = days_to_exp / 365.0;
                                                            let r = 0.05; // risk-free rate estimate
                                                            let sigma = 0.30; // default IV (30%)

                                                            for s in strikes.iter() {
                                                                let strike = s["strike_price"].as_f64().unwrap_or(0.0);
                                                                let call_sym = s["call_symbol"].as_str().unwrap_or("—");
                                                                let put_sym  = s["put_symbol"].as_str().unwrap_or("—");
                                                                ui.label(format!("{:.2}", strike));
                                                                ui.label(egui::RichText::new(call_sym).color(oc_green).monospace().small());
                                                                ui.label(egui::RichText::new(put_sym).color(oc_red).monospace().small());
                                                                // Greeks columns
                                                                if spot > 0.0 {
                                                                    let cg = typhoon_engine::core::options::greeks(spot, strike, t_years, r, sigma, true);
                                                                    let _pg = typhoon_engine::core::options::greeks(spot, strike, t_years, r, sigma, false);
                                                                    ui.label(egui::RichText::new(format!("{:.3}", cg.delta)).small());
                                                                    ui.label(egui::RichText::new(format!("{:.4}", cg.gamma)).small());
                                                                    ui.label(egui::RichText::new(format!("{:.2}", cg.theta)).small());
                                                                    ui.label(egui::RichText::new(format!("{:.3}", cg.vega)).small());
                                                                } else {
                                                                    ui.label("—"); ui.label("—"); ui.label("—"); ui.label("—");
                                                                }
                                                                ui.end_row();
                                                            }
                                                        });
                                                    }
                                                });
                                            }
                                        });
                                    }
                                } else {
                                    ui.label(egui::RichText::new("Failed to parse option chain data.").color(oc_red).small());
                                }
                            } else {
                                ui.label(egui::RichText::new(format!("No data cached for {} — click Refresh or run OPTIONS command.", sym)).color(oc_dim).small());
                            }
                        });
        }

        // Seasonals — computed from loaded chart bar data
        if self.show_seasonals {
            egui::Window::new("Seasonal Patterns")
                .open(&mut self.show_seasonals)
                .resizable(true)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("Seasonality Analysis");
                    ui.separator();
                    if let Some(chart) = self.charts.get(self.active_tab) {
                        if chart.bars.len() > 30 {
                            // Compute monthly returns from bar data
                            let mut monthly: std::collections::HashMap<u32, Vec<f64>> =
                                std::collections::HashMap::new();
                            for w in chart.bars.windows(2) {
                                let dt = chrono::DateTime::from_timestamp_millis(w[1].ts_ms);
                                if let Some(dt) = dt {
                                    let month =
                                        dt.format("%m").to_string().parse::<u32>().unwrap_or(0);
                                    let ret = (w[1].close - w[0].close) / w[0].close * 100.0;
                                    monthly.entry(month).or_default().push(ret);
                                }
                            }
                            let months = [
                                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep",
                                "Oct", "Nov", "Dec",
                            ];
                            let mut monthly_avgs: Vec<(usize, f64)> = Vec::new();
                            egui::Grid::new("seasonal_grid")
                                .striped(true)
                                .num_columns(4)
                                .show(ui, |ui| {
                                    ui.strong("Month");
                                    ui.strong("Avg Return %");
                                    ui.strong("Win Rate %");
                                    ui.strong("Samples");
                                    ui.end_row();
                                    for (i, name) in months.iter().enumerate() {
                                        let m = (i + 1) as u32;
                                        if let Some(rets) = monthly.get(&m) {
                                            if !rets.is_empty() {
                                                let avg: f64 =
                                                    rets.iter().sum::<f64>() / rets.len() as f64;
                                                let wins =
                                                    rets.iter().filter(|&&r| r > 0.0).count();
                                                let wr = wins as f64 / rets.len() as f64 * 100.0;
                                                let c = if avg >= 0.0 { UP } else { DOWN };
                                                ui.label(*name);
                                                ui.label(
                                                    egui::RichText::new(format!("{:.3}", avg))
                                                        .color(c),
                                                );
                                                ui.label(format!("{:.1}", wr));
                                                ui.label(format!("{}", rets.len()));
                                                ui.end_row();
                                                monthly_avgs.push((i, avg));
                                            }
                                        }
                                    }
                                });
                            // Monthly returns bar chart
                            if !monthly_avgs.is_empty() {
                                ui.add_space(8.0);
                                ui.label(egui::RichText::new("Monthly Average Returns").strong());
                                let pos_bars: Vec<PlotBar> = monthly_avgs
                                    .iter()
                                    .filter(|&&(_, avg)| avg >= 0.0)
                                    .map(|&(i, avg)| PlotBar::new(i as f64, avg).name(months[i]))
                                    .collect();
                                let neg_bars: Vec<PlotBar> = monthly_avgs
                                    .iter()
                                    .filter(|&&(_, avg)| avg < 0.0)
                                    .map(|&(i, avg)| PlotBar::new(i as f64, avg).name(months[i]))
                                    .collect();
                                let pos_chart =
                                    BarChart::new("Positive", pos_bars).width(0.6).color(UP);
                                let neg_chart =
                                    BarChart::new("Negative", neg_bars).width(0.6).color(DOWN);
                                Plot::new("seasonal_bars")
                                    .height(120.0)
                                    .allow_drag(false)
                                    .allow_zoom(false)
                                    .show(ui, |plot_ui| {
                                        plot_ui.bar_chart(pos_chart);
                                        plot_ui.bar_chart(neg_chart);
                                    });
                            }
                        } else {
                            ui.label(
                                egui::RichText::new("Need more bar data for seasonal analysis.")
                                    .color(AXIS_TEXT),
                            );
                        }
                    }
                });
        }

        // Volume Profile — computed from loaded chart bars
        if self.show_volume_profile {
            egui::Window::new("Volume Profile")
                .open(&mut self.show_volume_profile)
                .resizable(true)
                .default_size([400.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("Volume Profile");
                    ui.separator();
                    if let Some(chart) = self.charts.get(self.active_tab) {
                        let (si, ei) = chart.visible_range();
                        let bars = &chart.bars[si..ei];
                        if bars.len() > 10 {
                            // Build volume-at-price histogram
                            let price_min = bars.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                            let price_max = bars.iter().map(|b| b.high).fold(f64::MIN, f64::max);
                            let num_bins = 30;
                            let bin_size = (price_max - price_min) / num_bins as f64;
                            if bin_size > 0.0 {
                                let mut bins = vec![0.0_f64; num_bins];
                                for b in bars {
                                    let mid = (b.high + b.low) / 2.0;
                                    let idx = ((mid - price_min) / bin_size).floor() as usize;
                                    let idx = idx.min(num_bins - 1);
                                    bins[idx] += b.volume;
                                }
                                let max_vol = bins.iter().fold(0.0_f64, |a, &b| a.max(b));
                                let poc_idx = bins
                                    .iter()
                                    .enumerate()
                                    .max_by(|a, b| {
                                        a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)
                                    })
                                    .map(|(i, _)| i)
                                    .unwrap_or(0);
                                let poc_price = price_min + (poc_idx as f64 + 0.5) * bin_size;
                                ui.label(
                                    egui::RichText::new(format!(
                                        "POC: {}",
                                        format_price(poc_price)
                                    ))
                                    .strong()
                                    .color(ACCENT),
                                );

                                // Value Area (70% of volume)
                                let total_vol: f64 = bins.iter().sum();
                                let va_target = total_vol * 0.7;
                                let mut va_vol = bins[poc_idx];
                                let mut va_lo = poc_idx;
                                let mut va_hi = poc_idx;
                                while va_vol < va_target && (va_lo > 0 || va_hi < num_bins - 1) {
                                    let expand_lo = if va_lo > 0 { bins[va_lo - 1] } else { 0.0 };
                                    let expand_hi = if va_hi < num_bins - 1 {
                                        bins[va_hi + 1]
                                    } else {
                                        0.0
                                    };
                                    if expand_lo >= expand_hi && va_lo > 0 {
                                        va_lo -= 1;
                                        va_vol += bins[va_lo];
                                    } else if va_hi < num_bins - 1 {
                                        va_hi += 1;
                                        va_vol += bins[va_hi];
                                    } else {
                                        break;
                                    }
                                }
                                let vah = price_min + (va_hi as f64 + 1.0) * bin_size;
                                let val = price_min + va_lo as f64 * bin_size;
                                ui.label(format!(
                                    "VAH: {}  |  VAL: {}",
                                    format_price(vah),
                                    format_price(val)
                                ));

                                // Initial Balance (IB) — first hour of the session
                                // Detect session start: first bar of the last trading day
                                let last_day =
                                    bars.last().map(|b| b.ts_ms / 1000 / 86400).unwrap_or(0);
                                let session_bars: Vec<&Bar> = bars
                                    .iter()
                                    .filter(|b| b.ts_ms / 1000 / 86400 == last_day)
                                    .collect();
                                if session_bars.len() > 2 {
                                    // IB = first ~60 minutes of bars
                                    let session_start_ts =
                                        session_bars.first().map(|b| b.ts_ms).unwrap_or(0);
                                    let ib_end_ts = session_start_ts + 60 * 60 * 1000; // +1 hour in ms
                                    let ib_bars: Vec<&&Bar> = session_bars
                                        .iter()
                                        .filter(|b| b.ts_ms <= ib_end_ts)
                                        .collect();
                                    if ib_bars.len() > 1 {
                                        let ib_high =
                                            ib_bars.iter().map(|b| b.high).fold(f64::MIN, f64::max);
                                        let ib_low =
                                            ib_bars.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "IB High: {}  |  IB Low: {}  |  IB Range: {}",
                                                format_price(ib_high),
                                                format_price(ib_low),
                                                format_price(ib_high - ib_low)
                                            ))
                                            .color(SMA200_COL)
                                            .small(),
                                        );
                                    }
                                }
                                ui.add_space(5.0);

                                // Horizontal bar chart
                                let avail = ui.available_size();
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(avail.x, 250.0),
                                    egui::Sense::hover(),
                                );
                                let painter = ui.painter_at(rect);
                                painter.rect_filled(rect, 0.0, BG);
                                let row_h = rect.height() / num_bins as f32;
                                for (i, &vol) in bins.iter().enumerate().rev() {
                                    let frac = if max_vol > 0.0 { vol / max_vol } else { 0.0 };
                                    let y = rect.top() + (num_bins - 1 - i) as f32 * row_h;
                                    let w = frac as f32 * rect.width() * 0.85;
                                    let color = if i == poc_idx {
                                        ACCENT
                                    } else if i >= va_lo && i <= va_hi {
                                        egui::Color32::from_rgba_premultiplied(76, 175, 80, 100)
                                    } else {
                                        egui::Color32::from_rgba_premultiplied(100, 100, 140, 80)
                                    };
                                    painter.rect_filled(
                                        egui::Rect::from_min_size(
                                            egui::pos2(rect.left(), y),
                                            egui::vec2(w, row_h - 1.0),
                                        ),
                                        0.0,
                                        color,
                                    );
                                    // Price label
                                    let price = price_min + (i as f64 + 0.5) * bin_size;
                                    painter.text(
                                        egui::pos2(rect.right() - 2.0, y + row_h * 0.5),
                                        egui::Align2::RIGHT_CENTER,
                                        format_price(price),
                                        egui::FontId::monospace(8.0),
                                        AXIS_TEXT,
                                    );
                                }
                            }
                        } else {
                            ui.label(
                                egui::RichText::new("Need visible bar data for volume profile.")
                                    .color(AXIS_TEXT),
                            );
                        }
                    }
                });
        }

        // ── HV Cone ────────────────────────────────────────────────────
        if self.show_hv_cone {
            egui::Window::new("Historical Volatility Cone")
                .open(&mut self.show_hv_cone)
                .resizable(true)
                .default_size([450.0, 300.0])
                .show(ctx, |ui| {
                    if let Some(chart) = self.charts.get(self.active_tab) {
                        let closes: Vec<f64> = chart.bars.iter().map(|b| b.close).collect();
                        let cone = typhoon_engine::core::screener::compute_hv_cone(
                            &closes,
                            &[10, 20, 60, 252],
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "HV Cone: {} ({} bars)",
                                chart.symbol,
                                closes.len()
                            ))
                            .strong(),
                        );
                        ui.separator();
                        egui::Grid::new("hv_cone_grid")
                            .striped(true)
                            .num_columns(6)
                            .show(ui, |ui| {
                                ui.strong("Lookback");
                                ui.strong("Current HV");
                                ui.strong("Percentile");
                                ui.strong("Min");
                                ui.strong("Median");
                                ui.strong("Max");
                                ui.end_row();
                                for pt in &cone {
                                    ui.label(format!("{}d", pt.lookback));
                                    let hv_col = if pt.percentile > 80.0 {
                                        DOWN
                                    } else if pt.percentile > 50.0 {
                                        SMA200_COL
                                    } else {
                                        UP
                                    };
                                    ui.label(
                                        egui::RichText::new(format!("{:.1}%", pt.current_hv))
                                            .color(hv_col),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("{:.0}%ile", pt.percentile))
                                            .color(hv_col),
                                    );
                                    ui.label(format!("{:.1}%", pt.min_hv));
                                    ui.label(format!("{:.1}%", pt.median_hv));
                                    ui.label(format!("{:.1}%", pt.max_hv));
                                    ui.end_row();
                                }
                            });
                    } else {
                        ui.label("Open a chart first.");
                    }
                });
        }

        // ── Sector Heatmap ────────────────────────────────────────────
        if self.show_sector_heatmap {
            let scope_label = self.broker_scope_label();
            let scoped_count = self.cached_scoped_fundamentals.len();
            if let Some(scope_key) = self.cached_scoped_fundamentals_key {
                let scoped = std::sync::Arc::clone(&self.cached_scoped_fundamentals);
                style_scope::refresh_arc_slice_cache(
                    &mut self.cached_sector_heatmap,
                    &mut self.cached_sector_heatmap_key,
                    scope_key,
                    || typhoon_engine::core::screener::compute_sector_heatmap(&scoped),
                );
            }
            // PERF: grouping and sorting happen only when the scoped snapshot changes.
            let sectors = std::sync::Arc::clone(&self.cached_sector_heatmap);
            egui::Window::new("Sector Heatmap")
                .open(&mut self.show_sector_heatmap)
                .resizable(true)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} sectors • scope: {} ({} symbols)",
                            sectors.len(),
                            scope_label,
                            scoped_count
                        ))
                        .strong(),
                    );
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            egui::Grid::new("sector_heat_grid")
                                .striped(true)
                                .num_columns(4)
                                .show(ui, |ui| {
                                    ui.strong("Sector");
                                    ui.strong("Symbols");
                                    ui.strong("Total MCap");
                                    ui.strong("Avg P/E");
                                    ui.end_row();
                                    for s in sectors.iter() {
                                        ui.label(&s.sector);
                                        ui.label(format!("{}", s.symbol_count));
                                        ui.label(fundamentals::format_large_number(
                                            s.total_market_cap,
                                        ));
                                        ui.label(format!("{:.1}", s.avg_pe));
                                        ui.end_row();
                                    }
                                });
                        });
                });
        }

        // ── Dividend Yield Screener ───────────────────────────────────
        if self.show_dividends {
            let scope_label = self.broker_scope_label();
            if let Some(scope_key) = self.cached_scoped_fundamentals_key {
                let scoped = std::sync::Arc::clone(&self.cached_scoped_fundamentals);
                style_scope::refresh_arc_slice_cache(
                    &mut self.cached_dividend_screen,
                    &mut self.cached_dividend_screen_key,
                    scope_key,
                    || typhoon_engine::core::screener::screen_dividend_stocks(&scoped),
                );
            }
            // PERF: screening and sorting happen only when the scoped snapshot changes.
            let divs = std::sync::Arc::clone(&self.cached_dividend_screen);
            let mut div_pending_action = SymbolAction::None;
            // UX7: Pre-fetch sparklines for dividend stocks
            let mut div_sparklines: std::collections::HashMap<String, std::sync::Arc<Vec<f64>>> =
                std::collections::HashMap::new();
            for d in divs.iter().take(100) {
                let closes = self.get_sparkline(&d.symbol);
                if !closes.is_empty() {
                    div_sparklines.insert(d.symbol.to_uppercase(), closes);
                }
            }
            egui::Window::new("Dividend Yield Screener")
                .open(&mut self.show_dividends)
                .resizable(true)
                .default_size([700.0, 400.0])
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} dividend stocks • scope: {}",
                            divs.len(),
                            scope_label
                        ))
                        .strong(),
                    );
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            egui::Grid::new("div_screen_grid")
                                .striped(true)
                                .num_columns(6)
                                .show(ui, |ui| {
                                    ui.strong("Symbol");
                                    ui.strong("30d");
                                    ui.strong("Company");
                                    ui.strong("Yield%");
                                    ui.strong("Ex-Div");
                                    ui.strong("P/E");
                                    ui.end_row();
                                    for d in divs.iter().take(100) {
                                        ui.horizontal(|ui| {
                                            let (_, dv_action) = symbol_label_with_menu(
                                                ui,
                                                &d.symbol,
                                                egui::RichText::new(&d.symbol).strong(),
                                            );
                                            if !matches!(dv_action, SymbolAction::None) {
                                                div_pending_action = dv_action;
                                            }
                                            if ui
                                                .small_button(egui::RichText::new("+").small())
                                                .on_hover_text("Open new chart")
                                                .clicked()
                                            {
                                                div_pending_action =
                                                    SymbolAction::OpenChart(d.symbol.clone());
                                            }
                                        });
                                        if let Some(closes) =
                                            div_sparklines.get(&d.symbol.to_uppercase())
                                        {
                                            draw_inline_sparkline(ui, closes, 50.0, 12.0);
                                        } else {
                                            ui.label(
                                                egui::RichText::new("—").color(AXIS_TEXT).small(),
                                            );
                                        }
                                        ui.label(egui::RichText::new(&d.company).small());
                                        let yc = if d.dividend_yield > 4.0 {
                                            UP
                                        } else {
                                            AXIS_TEXT
                                        };
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:.2}%",
                                                d.dividend_yield
                                            ))
                                            .color(yc),
                                        );
                                        ui.label(egui::RichText::new(&d.ex_div_date).small());
                                        ui.label(format!("{:.1}", d.pe_ratio));
                                        ui.end_row();
                                    }
                                });
                        });
                });
            self.apply_symbol_action(div_pending_action);
        }

        // ── Event Calendar (upcoming earnings / ex-div / div-pay) ─────
        if self.show_event_calendar {
            let mut event_pending_action = SymbolAction::None;
            egui::Window::new("Event Calendar — Upcoming Important Dates")
                .open(&mut self.show_event_calendar)
                .resizable(true)
                .default_size([780.0, 520.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Source:").strong());
                        ui.radio_value(&mut self.event_filter_source, EventSource::All, "All");
                        ui.radio_value(
                            &mut self.event_filter_source,
                            EventSource::Alpaca,
                            "Alpaca",
                        );
                        ui.radio_value(
                            &mut self.event_filter_source,
                            EventSource::Kraken,
                            "Kraken",
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Type:").strong());
                        ui.checkbox(&mut self.event_filter_earnings, "Earnings");
                        ui.checkbox(&mut self.event_filter_exdiv, "Ex-Div");
                        ui.checkbox(&mut self.event_filter_divpay, "Div Pay");
                        ui.separator();
                        if ui.small_button("Export .ics").clicked() {
                            // Export currently-filtered events to an iCalendar file
                            // that can be imported into Google / Apple / Outlook calendars.
                            let mut path = dirs_home();
                            path.push("typhoon_events.ics");
                            let ics = Self::build_events_ics(
                                &self.event_calendar_rows,
                                self.event_filter_source,
                                self.event_filter_earnings,
                                self.event_filter_exdiv,
                                self.event_filter_divpay,
                            );
                            match std::fs::write(&path, ics) {
                                Ok(_) => self.log.push_back(LogEntry::info(format!(
                                    "Event calendar exported to {}",
                                    path.display()
                                ))),
                                Err(e) => self
                                    .log
                                    .push_back(LogEntry::err(format!("ICS export failed: {e}"))),
                            }
                        }
                    });
                    ui.separator();

                    // Apply filters.
                    let filtered: Vec<&EventRow> = self
                        .event_calendar_rows
                        .iter()
                        .filter(|r| {
                            let src_ok = match self.event_filter_source {
                                EventSource::All => r.in_alpaca || r.in_kraken,
                                EventSource::Alpaca => r.in_alpaca,
                                EventSource::Kraken => r.in_kraken,
                                EventSource::Positions => r.in_alpaca || r.in_kraken,
                            };
                            let kind_ok = match r.kind {
                                EventKind::Earnings => self.event_filter_earnings,
                                EventKind::ExDividend => self.event_filter_exdiv,
                                EventKind::DividendPayment => self.event_filter_divpay,
                            };
                            src_ok && kind_ok
                        })
                        .collect();

                    ui.label(
                        egui::RichText::new(format!(
                            "{} events shown ({} total)",
                            filtered.len(),
                            self.event_calendar_rows.len()
                        ))
                        .strong(),
                    );
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            egui::Grid::new("event_cal_grid")
                                .striped(true)
                                .num_columns(7)
                                .show(ui, |ui| {
                                    ui.strong("Date");
                                    ui.strong("Days");
                                    ui.strong("Type");
                                    ui.strong("Symbol");
                                    ui.strong("Company");
                                    ui.strong("Detail");
                                    ui.strong("Brokers");
                                    ui.end_row();
                                    for r in filtered.iter().take(500) {
                                        let date_col = if r.days_until <= 3 {
                                            DOWN
                                        } else if r.days_until <= 7 {
                                            egui::Color32::from_rgb(220, 180, 60)
                                        } else {
                                            AXIS_TEXT
                                        };
                                        ui.label(egui::RichText::new(&r.date).color(date_col));
                                        ui.label(
                                            egui::RichText::new(format!("{}", r.days_until))
                                                .color(date_col),
                                        );
                                        let kind_col = match r.kind {
                                            EventKind::Earnings => {
                                                egui::Color32::from_rgb(100, 180, 255)
                                            }
                                            EventKind::ExDividend => {
                                                egui::Color32::from_rgb(120, 220, 120)
                                            }
                                            EventKind::DividendPayment => {
                                                egui::Color32::from_rgb(220, 200, 80)
                                            }
                                        };
                                        ui.label(
                                            egui::RichText::new(r.kind.label())
                                                .color(kind_col)
                                                .strong(),
                                        );
                                        let (_, ev_action) = symbol_label_with_menu(
                                            ui,
                                            &r.symbol,
                                            egui::RichText::new(&r.symbol).strong().monospace(),
                                        );
                                        if !matches!(ev_action, SymbolAction::None) {
                                            event_pending_action = ev_action;
                                        }
                                        ui.label(egui::RichText::new(&r.company).small());
                                        ui.label(egui::RichText::new(&r.detail).small());
                                        let mut tags = Vec::new();
                                        if r.in_alpaca {
                                            tags.push("A");
                                        }
                                        if r.in_kraken {
                                            tags.push("K");
                                        }
                                        ui.label(
                                            egui::RichText::new(tags.join("")).small().monospace(),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                });
            self.apply_symbol_action(event_pending_action);
        }

        // ── MTF Confluence ────────────────────────────────────────────
        if self.show_confluence {
            egui::Window::new("MTF Confluence")
                .open(&mut self.show_confluence)
                .resizable(true)
                .default_size([400.0, 300.0])
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("Multi-Timeframe Confluence Score").strong());
                    ui.separator();
                    // Compute confluence for each chart symbol
                    for chart in &self.charts {
                        let sym = chart.symbol.split(':').next().unwrap_or(&chart.symbol);
                        // Gather signals from indicator data
                        let mut signals: Vec<(String, Option<bool>)> = Vec::new();
                        // RSI: >50 = bullish, <50 = bearish
                        if let Some(Some(rsi)) = chart.rsi.last() {
                            signals.push(("RSI".into(), Some(*rsi > 50.0)));
                        }
                        // MACD: line > signal = bullish
                        if let (Some(Some(ml)), Some(Some(ms))) =
                            (chart.macd_line.last(), chart.macd_signal.last())
                        {
                            signals.push(("MACD".into(), Some(*ml > *ms)));
                        }
                        // Price vs SMA200: above = bullish
                        if let (Some(bar), Some(Some(sma))) =
                            (chart.bars.last(), chart.sma200.last())
                        {
                            signals.push(("SMA200".into(), Some(bar.close > *sma)));
                        }
                        // Price vs KAMA: above = bullish
                        if let (Some(bar), Some(Some(kama))) =
                            (chart.bars.last(), chart.kama.last())
                        {
                            signals.push(("KAMA".into(), Some(bar.close > *kama)));
                        }
                        let conf =
                            typhoon_engine::core::screener::compute_mtf_confluence(sym, &signals);
                        let score_col = if conf.confluence_score > 0.3 {
                            UP
                        } else if conf.confluence_score < -0.3 {
                            DOWN
                        } else {
                            AXIS_TEXT
                        };
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(sym).strong().monospace());
                            ui.label(
                                egui::RichText::new(format!("{:+.2}", conf.confluence_score))
                                    .color(score_col),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "({} bull / {} bear / {} total)",
                                    conf.bullish_tfs, conf.bearish_tfs, conf.total_tfs
                                ))
                                .small()
                                .color(AXIS_TEXT),
                            );
                        });
                    }
                });
        }

        // ── Stat Arb Pairs ────────────────────────────────────────────
        if self.show_stat_arb {
            let mut sa_pending_action = SymbolAction::None;
            egui::Window::new("Statistical Arbitrage Pairs")
                        .open(&mut self.show_stat_arb)
                        .resizable(true).default_size([600.0, 400.0])
                        .show(ctx, |ui| {
                            ui.label(egui::RichText::new("Correlated Pairs — Spread Z-Score").strong());
                            ui.separator();
                            // Build close map from all chart symbols
                            let mut close_map: std::collections::HashMap<String, Vec<f64>> = std::collections::HashMap::new();
                            for chart in &self.charts {
                                let sym = chart.symbol.split(':').next().unwrap_or(&chart.symbol).to_uppercase();
                                if !sym.is_empty() && chart.bars.len() > 50 {
                                    close_map.insert(sym, chart.bars.iter().map(|b| b.close).collect());
                                }
                            }
                            let pairs = typhoon_engine::core::screener::find_stat_arb_pairs(&close_map, 0.7, 50);
                            if pairs.is_empty() {
                                ui.label(egui::RichText::new("No correlated pairs found (need >2 charts with >50 bars, correlation >0.7).").color(AXIS_TEXT));
                            } else {
                                egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                                    egui::Grid::new("stat_arb_grid").striped(true).num_columns(5).show(ui, |ui| {
                                        ui.strong("Pair"); ui.strong("Corr"); ui.strong("Z-Score"); ui.strong("Half-Life"); ui.strong("Signal");
                                        ui.end_row();
                                        for p in pairs.iter().take(20) {
                                            ui.horizontal(|ui| {
                                                let (_, sa_act_a) = symbol_label_with_menu(ui, &p.symbol_a,
                                                    egui::RichText::new(&p.symbol_a).strong());
                                                if !matches!(sa_act_a, SymbolAction::None) { sa_pending_action = sa_act_a; }
                                                ui.label("/");
                                                let (_, sa_act_b) = symbol_label_with_menu(ui, &p.symbol_b,
                                                    egui::RichText::new(&p.symbol_b).strong());
                                                if !matches!(sa_act_b, SymbolAction::None) { sa_pending_action = sa_act_b; }
                                            });
                                            ui.label(format!("{:.3}", p.correlation));
                                            let zc = if p.current_zscore.abs() > 2.0 { DOWN } else if p.current_zscore.abs() > 1.5 { SMA200_COL } else { AXIS_TEXT };
                                            ui.label(egui::RichText::new(format!("{:+.2}", p.current_zscore)).color(zc));
                                            let hl = if p.half_life < 1000.0 { format!("{:.1} bars", p.half_life) } else { "N/A".into() };
                                            ui.label(hl);
                                            let signal = if p.current_zscore > 2.0 { "SHORT spread" }
                                                else if p.current_zscore < -2.0 { "LONG spread" }
                                                else { "—" };
                                            let sc = if signal.contains("SHORT") { DOWN } else if signal.contains("LONG") { UP } else { AXIS_TEXT };
                                            ui.label(egui::RichText::new(signal).color(sc));
                                            ui.end_row();
                                        }
                                    });
                                });
                            }
                        });
            self.apply_symbol_action(sa_pending_action);
        }
    }
}
