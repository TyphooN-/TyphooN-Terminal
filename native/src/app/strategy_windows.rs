use super::*;

impl TyphooNApp {
    pub(super) fn render_backtest_window(&mut self, ctx: &egui::Context) {
        if !self.show_backtest {
            return;
        }
        let mut show_backtest = self.show_backtest;
        egui::Window::new("Backtest Engine")
            .open(&mut show_backtest)
            .resizable(true)
            .default_size([600.0, 500.0])
            .show(ctx, |ui| {
                ui.heading("Strategy Backtest");
                ui.separator();
                let chart = self.charts.get(self.active_tab);
                let n_bars = chart.map(|c| c.bars.len()).unwrap_or(0);
                let tf = chart.map(|c| c.timeframe.label()).unwrap_or("—");
                ui.horizontal(|ui| {
                    ui.label("Symbol:");
                    ui.label(egui::RichText::new(&self.symbol_input).strong());
                    ui.label("TF:");
                    ui.label(egui::RichText::new(tf).strong());
                    ui.label("Bars:");
                    ui.label(egui::RichText::new(format!("{}", n_bars)).strong());
                });
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.label("Strategy:");
                    ui.radio_value(&mut self.bt_strategy, 0, "SMA Cross");
                    ui.radio_value(&mut self.bt_strategy, 1, "NNFX");
                    ui.radio_value(&mut self.bt_strategy, 2, "KAMA Cross");
                    ui.radio_value(&mut self.bt_strategy, 3, "Fisher Cross");
                    ui.radio_value(&mut self.bt_strategy, 4, "RSI Mean-Rev");
                });
                ui.horizontal(|ui| {
                    ui.label("Fast Period:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.bt_fast_period).desired_width(50.0),
                    );
                    ui.label("Slow Period:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.bt_slow_period).desired_width(50.0),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Initial Equity:");
                    ui.add(egui::TextEdit::singleline(&mut self.bt_equity).desired_width(80.0));
                });

                ui.add_space(5.0);
                if ui.button("Run Backtest").clicked() && n_bars > 0 {
                    if let Some(chart) = self.charts.get(self.active_tab) {
                        let engine_bars: Vec<EngineBar> = chart
                            .bars
                            .iter()
                            .map(|b| EngineBar {
                                timestamp: format_ts(b.ts_ms, chart.timeframe),
                                open: b.open,
                                high: b.high,
                                low: b.low,
                                close: b.close,
                                volume: b.volume,
                            })
                            .collect();
                        let fast: usize = self.bt_fast_period.parse().unwrap_or(10);
                        let slow: usize = self.bt_slow_period.parse().unwrap_or(50);
                        let equity: f64 = self
                            .bt_equity
                            .replace(['$', ','], "")
                            .parse()
                            .unwrap_or(10000.0);

                        let result = match self.bt_strategy {
                            0 => {
                                let mut strat = backtest::SMACrossStrategy::new(fast, slow);
                                backtest::run_backtest(&engine_bars, &mut strat, equity)
                            }
                            1 => {
                                let mut strat = backtest::NNFXStrategy::new(fast, slow);
                                backtest::run_backtest(&engine_bars, &mut strat, equity)
                            }
                            2 => {
                                let mut strat = backtest::KAMACrossStrategy::new(fast, 2, 30);
                                backtest::run_backtest(&engine_bars, &mut strat, equity)
                            }
                            3 => {
                                let mut strat = backtest::FisherCrossStrategy::new(fast.max(10));
                                backtest::run_backtest(&engine_bars, &mut strat, equity)
                            }
                            4 => {
                                let mut strat =
                                    backtest::RSIMeanRevStrategy::new(fast.max(5), 30.0, 70.0);
                                backtest::run_backtest(&engine_bars, &mut strat, equity)
                            }
                            _ => {
                                let mut strat = backtest::SMACrossStrategy::new(fast, slow);
                                backtest::run_backtest(&engine_bars, &mut strat, equity)
                            }
                        };
                        self.bt_result = Some(result.report);
                        self.bt_trades = result.trades;
                        self.bt_equity_curve = result.equity_curve;
                        self.log.push_back(LogEntry::info(format!(
                            "Backtest complete: {} trades, PF={:.2}, WR={:.1}%",
                            self.bt_trades.len(),
                            self.bt_result
                                .as_ref()
                                .map(|r| r.profit_factor)
                                .unwrap_or(0.0),
                            self.bt_result.as_ref().map(|r| r.win_rate).unwrap_or(0.0),
                        )));
                    }
                }

                if let Some(ref report) = self.bt_result {
                    ui.add_space(10.0);
                    ui.heading("Results");
                    ui.separator();
                    egui::Grid::new("bt_report")
                        .striped(true)
                        .num_columns(4)
                        .show(ui, |ui| {
                            ui.label("Trades:");
                            ui.label(format!("{}", report.total_trades));
                            ui.label("Win Rate:");
                            {
                                let wr_c = if report.win_rate >= 50.0 {
                                    UP
                                } else if report.win_rate >= 40.0 {
                                    egui::Color32::from_rgb(255, 200, 50)
                                } else {
                                    DOWN
                                };
                                ui.label(
                                    egui::RichText::new(format!("{:.1}%", report.win_rate))
                                        .color(wr_c),
                                );
                            }
                            ui.end_row();
                            ui.label("Profit Factor:");
                            ui.label(format!("{:.2}", report.profit_factor));
                            ui.label("Sharpe:");
                            ui.label(format!("{:.3}", report.sharpe_ratio));
                            ui.end_row();
                            let pnl_c = if report.total_pnl >= 0.0 { UP } else { DOWN };
                            ui.label("Total P&L:");
                            ui.label(
                                egui::RichText::new(format!("${:.2}", report.total_pnl))
                                    .color(pnl_c),
                            );
                            ui.label("Max DD:");
                            ui.label(
                                egui::RichText::new(format!("{:.2}%", report.max_drawdown_pct))
                                    .color(DOWN),
                            );
                            ui.end_row();
                            ui.label("Avg Win:");
                            ui.label(format!("${:.2}", report.avg_win));
                            ui.label("Avg Loss:");
                            ui.label(format!("${:.2}", report.avg_loss));
                            ui.end_row();
                            ui.label("Max Win Streak:");
                            ui.label(format!("{}", report.max_consecutive_wins));
                            ui.label("Max Loss Streak:");
                            ui.label(format!("{}", report.max_consecutive_losses));
                            ui.end_row();
                        });

                    if self.bt_equity_curve.len() > 2 {
                        ui.add_space(10.0);
                        ui.heading("Equity Curve");
                        let points: PlotPoints = PlotPoints::new(
                            self.bt_equity_curve
                                .iter()
                                .enumerate()
                                .map(|(i, &v)| [i as f64, v])
                                .collect(),
                        );
                        let line = Line::new("Equity", points).color(ACCENT);
                        Plot::new("bt_equity_plot")
                            .height(150.0)
                            .allow_drag(false)
                            .allow_zoom(false)
                            .show(ui, |plot_ui| {
                                plot_ui.line(line);
                            });
                    }

                    if !self.bt_trades.is_empty() {
                        ui.add_space(10.0);
                        ui.collapsing(format!("Trade List ({})", self.bt_trades.len()), |ui| {
                            egui::ScrollArea::vertical()
                                .auto_shrink(false)
                                .max_height(200.0)
                                .show(ui, |ui| {
                                    egui::Grid::new("bt_trades")
                                        .striped(true)
                                        .num_columns(5)
                                        .show(ui, |ui| {
                                            ui.strong("#");
                                            ui.strong("Side");
                                            ui.strong("Entry");
                                            ui.strong("Exit");
                                            ui.strong("P&L");
                                            ui.end_row();
                                            for (i, t) in self.bt_trades.iter().enumerate() {
                                                ui.label(format!("{}", i + 1));
                                                ui.label(&t.side);
                                                ui.label(format_price(t.entry_price));
                                                ui.label(format_price(t.exit_price));
                                                let c = if t.pnl >= 0.0 { UP } else { DOWN };
                                                ui.label(
                                                    egui::RichText::new(format!("{:.2}", t.pnl))
                                                        .color(c),
                                                );
                                                ui.end_row();
                                            }
                                        });
                                });
                        });
                    }
                }
            });
        self.show_backtest = show_backtest;
    }

    pub(super) fn render_optimizer_window(&mut self, ctx: &egui::Context) {
        if !self.show_optimizer {
            return;
        }
        let mut show_optimizer = self.show_optimizer;
        egui::Window::new("Optimizer")
            .open(&mut show_optimizer)
            .resizable(true)
            .default_size([750.0, 600.0])
            .show(ctx, |ui| {
                let opt_green = egui::Color32::from_rgb(46, 204, 113);
                let opt_red = egui::Color32::from_rgb(231, 76, 60);
                let opt_gold = egui::Color32::from_rgb(241, 196, 15);
                let opt_cyan = egui::Color32::from_rgb(26, 188, 156);
                let opt_dim = egui::Color32::from_rgb(100, 100, 120);

                let gpu_available = self.gpu_backtester.is_some();
                ui.horizontal(|ui| {
                    ui.heading("Strategy Optimizer");
                    if gpu_available {
                        ui.label(egui::RichText::new("GPU").color(opt_green).strong());
                    } else {
                        ui.label(egui::RichText::new("CPU").color(opt_gold));
                    }
                });
                ui.separator();

                let chart = self.charts.get(self.active_tab);
                let n_bars = chart.map(|c| c.bars.len()).unwrap_or(0);
                ui.label(
                    egui::RichText::new(format!(
                        "Symbol: {}  |  Bars: {}  |  {}",
                        self.symbol_input,
                        n_bars,
                        chart.map(|c| c.timeframe.label()).unwrap_or("?")
                    ))
                    .color(opt_cyan),
                );

                ui.add_space(4.0);
                ui.label(egui::RichText::new("Parameter Ranges").strong());
                egui::Grid::new("opt_params")
                    .num_columns(4)
                    .show(ui, |ui| {
                        ui.label("SMA Fast:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.opt_fast_range)
                                .desired_width(60.0),
                        );
                        ui.label("SMA Slow:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.opt_slow_range)
                                .desired_width(60.0),
                        );
                        ui.end_row();
                        ui.label("RSI Period:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.opt_rsi_range).desired_width(60.0),
                        );
                        ui.label("ATR SL Mult:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.opt_atr_sl_range)
                                .desired_width(60.0),
                        );
                        ui.end_row();
                        ui.label("ATR TP Mult:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.opt_atr_tp_range)
                                .desired_width(60.0),
                        );
                        ui.label("");
                        ui.label("");
                        ui.end_row();
                    });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if gpu_available
                        && ui
                            .button(
                                egui::RichText::new("Run GPU Optimization")
                                    .color(opt_green)
                                    .strong(),
                            )
                            .clicked()
                        && n_bars > 50
                    {
                        if let Some(chart) = self.charts.get(self.active_tab) {
                            let closes: Vec<f32> =
                                chart.bars.iter().map(|b| b.close as f32).collect();
                            let highs: Vec<f32> =
                                chart.bars.iter().map(|b| b.high as f32).collect();
                            let lows: Vec<f32> =
                                chart.bars.iter().map(|b| b.low as f32).collect();

                            let fast = parse_range(&self.opt_fast_range, 5, 50);
                            let slow = parse_range(&self.opt_slow_range, 20, 200);
                            let rsi_r = parse_range(&self.opt_rsi_range, 10, 20);
                            let atr_sl = parse_range_f32(&self.opt_atr_sl_range, 1.0, 3.0);
                            let atr_tp = parse_range_f32(&self.opt_atr_tp_range, 2.0, 5.0);

                            let mut combos = Vec::new();
                            let fast_step = ((fast.1 - fast.0) / 10).max(1);
                            let slow_step = ((slow.1 - slow.0) / 10).max(1);
                            let rsi_step = ((rsi_r.1 - rsi_r.0) / 5).max(1);
                            let atr_sl_step = (atr_sl.1 - atr_sl.0) / 5.0;
                            let atr_tp_step = (atr_tp.1 - atr_tp.0) / 5.0;

                            let mut f = fast.0;
                            while f <= fast.1 {
                                let mut s = slow.0;
                                while s <= slow.1 {
                                    if s > f {
                                        let mut r = rsi_r.0;
                                        while r <= rsi_r.1 {
                                            let mut sl_m = atr_sl.0;
                                            while sl_m <= atr_sl.1 + 0.001 {
                                                let mut tp_m = atr_tp.0;
                                                while tp_m <= atr_tp.1 + 0.001 {
                                                    combos.push(gpu_compute::ParamCombo {
                                                        sma_fast: f as u32,
                                                        sma_slow: s as u32,
                                                        rsi_period: r as u32,
                                                        rsi_overbought: 70.0,
                                                        rsi_oversold: 30.0,
                                                        atr_period: 14,
                                                        atr_sl_mult: sl_m as f32,
                                                        atr_tp_mult: tp_m as f32,
                                                    });
                                                    tp_m += atr_tp_step;
                                                }
                                                sl_m += atr_sl_step;
                                            }
                                            r += rsi_step;
                                        }
                                    }
                                    s += slow_step;
                                }
                                f += fast_step;
                            }

                            let combo_count = combos.len();
                            self.gpu_opt_combos = combos.clone();

                            if let Some(ref mut bt) = self.gpu_backtester {
                                let t = std::time::Instant::now();
                                bt.upload(&closes, &highs, &lows, &combos);
                                if let Some(results) = bt.evaluate() {
                                    let elapsed = t.elapsed();
                                    self.gpu_opt_results = results;
                                    let mut indexed: Vec<(usize, &gpu_compute::BacktestResult)> =
                                        self.gpu_opt_results.iter().enumerate().collect();
                                    indexed.sort_by(|a, b| {
                                        b.1.sharpe
                                            .partial_cmp(&a.1.sharpe)
                                            .unwrap_or(std::cmp::Ordering::Equal)
                                    });
                                    let sorted_results: Vec<gpu_compute::BacktestResult> = indexed
                                        .iter()
                                        .map(|(i, _)| self.gpu_opt_results[*i].clone())
                                        .collect();
                                    let sorted_combos: Vec<gpu_compute::ParamCombo> = indexed
                                        .iter()
                                        .map(|(i, _)| self.gpu_opt_combos[*i].clone())
                                        .collect();
                                    self.gpu_opt_results = sorted_results;
                                    self.gpu_opt_combos = sorted_combos;
                                    self.log.push_back(LogEntry::info(format!(
                                        "GPU Optimizer: {} combos tested in {:.1}ms ({:.0} combos/sec)",
                                        combo_count,
                                        elapsed.as_secs_f64() * 1000.0,
                                        combo_count as f64 / elapsed.as_secs_f64()
                                    )));
                                }
                            }
                        }
                    }
                    if gpu_available
                        && ui
                            .button(
                                egui::RichText::new("Run NNFX Optimizer")
                                    .color(egui::Color32::from_rgb(155, 89, 182))
                                    .strong(),
                            )
                            .clicked()
                        && n_bars > 50
                    {
                        if let Some(chart) = self.charts.get(self.active_tab) {
                            let closes: Vec<f32> =
                                chart.bars.iter().map(|b| b.close as f32).collect();
                            let highs: Vec<f32> =
                                chart.bars.iter().map(|b| b.high as f32).collect();
                            let lows: Vec<f32> =
                                chart.bars.iter().map(|b| b.low as f32).collect();

                            let mut nnfx_combos = Vec::new();
                            for kama_p in (5..=20).step_by(3) {
                                for fisher_p in (10..=40).step_by(5) {
                                    for adx_thresh in [20.0_f32, 25.0, 30.0] {
                                        for sl_mult in [1.0_f32, 1.5, 2.0, 2.5] {
                                            for tp_mult in [1.5_f32, 2.0, 3.0, 4.0] {
                                                nnfx_combos.push(gpu_compute::NnfxParamCombo {
                                                    kama_period: kama_p,
                                                    fisher_period: fisher_p,
                                                    atr_period: 14,
                                                    adx_period: 14,
                                                    adx_threshold: adx_thresh,
                                                    atr_sl_mult: sl_mult,
                                                    atr_tp_mult: tp_mult,
                                                });
                                            }
                                        }
                                    }
                                }
                            }

                            let combo_count = nnfx_combos.len();
                            if let Some(ref mut bt) = self.gpu_backtester {
                                let t = std::time::Instant::now();
                                if let Some(results) =
                                    bt.evaluate_nnfx(&closes, &highs, &lows, &nnfx_combos)
                                {
                                    let elapsed = t.elapsed();
                                    let mut indexed: Vec<(usize, f32)> = results
                                        .iter()
                                        .enumerate()
                                        .map(|(i, r)| (i, r.sharpe))
                                        .collect();
                                    indexed.sort_by(|a, b| {
                                        b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
                                    });

                                    self.gpu_opt_results = indexed
                                        .iter()
                                        .map(|(i, _)| results[*i].clone())
                                        .collect();
                                    self.gpu_opt_combos = indexed
                                        .iter()
                                        .map(|(i, _)| {
                                            let nc = &nnfx_combos[*i];
                                            gpu_compute::ParamCombo {
                                                sma_fast: nc.kama_period,
                                                sma_slow: nc.fisher_period,
                                                rsi_period: nc.adx_period,
                                                rsi_overbought: nc.adx_threshold,
                                                rsi_oversold: 0.0,
                                                atr_period: nc.atr_period,
                                                atr_sl_mult: nc.atr_sl_mult,
                                                atr_tp_mult: nc.atr_tp_mult,
                                            }
                                        })
                                        .collect();

                                    self.log.push_back(LogEntry::info(format!(
                                        "NNFX Optimizer: {} combos tested in {:.1}ms ({:.0}/sec) — Fisher+KAMA+ATR+ADX",
                                        combo_count,
                                        elapsed.as_secs_f64() * 1000.0,
                                        combo_count as f64 / elapsed.as_secs_f64()
                                    )));
                                }
                            }
                        }
                    }
                    if ui.button("Run CPU Optimization").clicked() && n_bars > 50 {
                        if let Some(chart) = self.charts.get(self.active_tab) {
                            let engine_bars: Vec<EngineBar> = chart
                                .bars
                                .iter()
                                .map(|b| EngineBar {
                                    timestamp: format_ts(b.ts_ms, chart.timeframe),
                                    open: b.open,
                                    high: b.high,
                                    low: b.low,
                                    close: b.close,
                                    volume: b.volume,
                                })
                                .collect();
                            let fast: (usize, usize) = parse_range(&self.opt_fast_range, 5, 50);
                            let slow: (usize, usize) = parse_range(&self.opt_slow_range, 20, 200);
                            let report =
                                backtest::optimize_sma_cross(&engine_bars, fast, slow, 10000.0, 20);
                            self.opt_results = report.results;
                            self.log.push_back(LogEntry::info(format!(
                                "CPU Optimizer: {} combinations tested",
                                report.total_combinations
                            )));
                        }
                    }
                });

                if !self.gpu_opt_results.is_empty() {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "GPU Results — Top {} of {}",
                                self.gpu_opt_results.len().min(50),
                                self.gpu_opt_results.len()
                            ))
                            .strong()
                            .color(opt_green),
                        );
                    });

                    let bars: Vec<PlotBar> = self
                        .gpu_opt_results
                        .iter()
                        .take(50)
                        .enumerate()
                        .map(|(i, r)| {
                            let c = if r.net_pnl >= 0.0 {
                                opt_green
                            } else {
                                opt_red
                            };
                            PlotBar::new(i as f64, r.net_pnl as f64).width(0.8).fill(c)
                        })
                        .collect();
                    if !bars.is_empty() {
                        let chart = BarChart::new("P&L by Combo", bars);
                        Plot::new("gpu_opt_pnl")
                            .height(100.0)
                            .allow_drag(false)
                            .allow_zoom(false)
                            .allow_scroll(false)
                            .show_axes([false, true])
                            .show(ui, |plot_ui| {
                                plot_ui.bar_chart(chart);
                            });
                    }

                    if self.gpu_opt_results.len() > 4
                        && self.gpu_opt_combos.len() == self.gpu_opt_results.len()
                    {
                        ui.label(
                            egui::RichText::new("Parameter Heatmap (Fast × Slow → Sharpe)")
                                .small()
                                .strong(),
                        );
                        let mut fast_set: Vec<u32> =
                            self.gpu_opt_combos.iter().map(|c| c.sma_fast).collect();
                        fast_set.sort();
                        fast_set.dedup();
                        let mut slow_set: Vec<u32> =
                            self.gpu_opt_combos.iter().map(|c| c.sma_slow).collect();
                        slow_set.sort();
                        slow_set.dedup();

                        if fast_set.len() > 1 && slow_set.len() > 1 {
                            let cols = fast_set.len();
                            let rows = slow_set.len();
                            let avail_w = ui.available_width().min(500.0);
                            let h = (rows as f32 * 14.0).min(200.0);
                            let cell_w = avail_w / cols as f32;
                            let cell_h = h / rows as f32;
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(avail_w, h),
                                egui::Sense::hover(),
                            );
                            let painter = ui.painter_at(rect);
                            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(10, 10, 20));

                            let max_sharpe = self
                                .gpu_opt_results
                                .iter()
                                .map(|r| r.sharpe)
                                .fold(0.0_f32, f32::max)
                                .max(0.01);
                            let min_sharpe = self
                                .gpu_opt_results
                                .iter()
                                .map(|r| r.sharpe)
                                .fold(f32::MAX, f32::min);

                            let fast_idx: std::collections::HashMap<u32, usize> = fast_set
                                .iter()
                                .enumerate()
                                .map(|(i, &f)| (f, i))
                                .collect();
                            let slow_idx: std::collections::HashMap<u32, usize> = slow_set
                                .iter()
                                .enumerate()
                                .map(|(i, &s)| (s, i))
                                .collect();
                            for (combo, result) in
                                self.gpu_opt_combos.iter().zip(self.gpu_opt_results.iter())
                            {
                                let col = fast_idx.get(&combo.sma_fast).copied().unwrap_or(0);
                                let row = slow_idx.get(&combo.sma_slow).copied().unwrap_or(0);
                                let x = rect.left() + col as f32 * cell_w;
                                let y = rect.top() + row as f32 * cell_h;

                                let norm = if max_sharpe > min_sharpe {
                                    (result.sharpe - min_sharpe) / (max_sharpe - min_sharpe)
                                } else {
                                    0.5
                                };
                                let color = if result.sharpe > 0.0 {
                                    egui::Color32::from_rgb(
                                        0,
                                        (norm * 200.0) as u8,
                                        (norm * 60.0) as u8,
                                    )
                                } else {
                                    egui::Color32::from_rgb(
                                        ((1.0 - norm) * 200.0) as u8,
                                        0,
                                        0,
                                    )
                                };
                                painter.rect_filled(
                                    egui::Rect::from_min_size(
                                        egui::pos2(x, y),
                                        egui::vec2(cell_w - 1.0, cell_h - 1.0),
                                    ),
                                    0.0,
                                    color,
                                );
                            }

                            for (i, &f) in fast_set.iter().enumerate() {
                                let x = rect.left() + i as f32 * cell_w + cell_w / 2.0;
                                painter.text(
                                    egui::pos2(x, rect.bottom() + 2.0),
                                    egui::Align2::CENTER_TOP,
                                    format!("{}", f),
                                    egui::FontId::monospace(8.0),
                                    opt_dim,
                                );
                            }
                            for (i, &s) in slow_set.iter().enumerate() {
                                let y = rect.top() + i as f32 * cell_h + cell_h / 2.0;
                                painter.text(
                                    egui::pos2(rect.left() - 2.0, y),
                                    egui::Align2::RIGHT_CENTER,
                                    format!("{}", s),
                                    egui::FontId::monospace(8.0),
                                    opt_dim,
                                );
                            }
                            ui.add_space(14.0);
                        }
                    }

                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .max_height(350.0)
                        .show(ui, |ui| {
                            egui::Grid::new("gpu_opt_grid")
                                .striped(true)
                                .num_columns(11)
                                .min_col_width(45.0)
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new("Fast").color(opt_dim).small());
                                    ui.label(egui::RichText::new("Slow").color(opt_dim).small());
                                    ui.label(egui::RichText::new("RSI").color(opt_dim).small());
                                    ui.label(egui::RichText::new("SL×").color(opt_dim).small());
                                    ui.label(egui::RichText::new("TP×").color(opt_dim).small());
                                    ui.label(egui::RichText::new("P&L").color(opt_dim).small());
                                    ui.label(egui::RichText::new("DD%").color(opt_dim).small());
                                    ui.label(
                                        egui::RichText::new("Sharpe").color(opt_dim).small(),
                                    );
                                    ui.label(egui::RichText::new("Win%").color(opt_dim).small());
                                    ui.label(
                                        egui::RichText::new("Trades").color(opt_dim).small(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Robust").color(opt_dim).small(),
                                    );
                                    ui.end_row();

                                    for (i, r) in self.gpu_opt_results.iter().take(50).enumerate()
                                    {
                                        let combo = &self.gpu_opt_combos[i.min(
                                            self.gpu_opt_combos.len().saturating_sub(1),
                                        )];
                                        ui.label(format!("{}", combo.sma_fast));
                                        ui.label(format!("{}", combo.sma_slow));
                                        ui.label(format!("{}", combo.rsi_period));
                                        ui.label(format!("{:.1}", combo.atr_sl_mult));
                                        ui.label(format!("{:.1}", combo.atr_tp_mult));
                                        let pc = if r.net_pnl >= 0.0 {
                                            opt_green
                                        } else {
                                            opt_red
                                        };
                                        ui.label(
                                            egui::RichText::new(format!("${:.0}", r.net_pnl))
                                                .color(pc),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:.1}%",
                                                r.max_drawdown * 100.0
                                            ))
                                            .color(opt_red),
                                        );
                                        let sc = if r.sharpe > 1.0 {
                                            opt_green
                                        } else if r.sharpe > 0.0 {
                                            opt_gold
                                        } else {
                                            opt_red
                                        };
                                        ui.label(
                                            egui::RichText::new(format!("{:.2}", r.sharpe))
                                                .color(sc),
                                        );
                                        let wc = if r.win_rate > 50.0 {
                                            opt_green
                                        } else {
                                            opt_red
                                        };
                                        ui.label(
                                            egui::RichText::new(format!("{:.0}%", r.win_rate))
                                                .color(wc),
                                        );
                                        ui.label(format!("{}", r.trade_count));
                                        let rc = if r.robustness_score > 0.7 {
                                            opt_green
                                        } else if r.robustness_score > 0.3 {
                                            opt_gold
                                        } else {
                                            opt_red
                                        };
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:.2}",
                                                r.robustness_score
                                            ))
                                            .color(rc),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                }

                if !self.opt_results.is_empty() && self.gpu_opt_results.is_empty() {
                    ui.add_space(10.0);
                    ui.heading(format!("CPU Results — Top {}", self.opt_results.len()));
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .max_height(300.0)
                        .show(ui, |ui| {
                            egui::Grid::new("opt_grid")
                                .striped(true)
                                .num_columns(6)
                                .show(ui, |ui| {
                                    ui.strong("Fast");
                                    ui.strong("Slow");
                                    ui.strong("Trades");
                                    ui.strong("PF");
                                    ui.strong("Sharpe");
                                    ui.strong("P&L");
                                    ui.end_row();
                                    for r in &self.opt_results {
                                        ui.label(format!("{}", r.fast_period));
                                        ui.label(format!("{}", r.slow_period));
                                        ui.label(format!("{}", r.total_trades));
                                        ui.label(format!("{:.2}", r.profit_factor));
                                        ui.label(format!("{:.3}", r.sharpe_ratio));
                                        let c = if r.total_pnl >= 0.0 { UP } else { DOWN };
                                        ui.label(
                                            egui::RichText::new(format!("${:.0}", r.total_pnl))
                                                .color(c),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                }

                ui.add_space(8.0);
                ui.heading("Walk-Forward Analysis");
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Windows:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.wf_windows_count).desired_width(40.0),
                    );
                    if ui.button("Run Walk-Forward").clicked() && n_bars > 200 {
                        if let Some(chart) = self.charts.get(self.active_tab) {
                            let engine_bars: Vec<EngineBar> = chart
                                .bars
                                .iter()
                                .map(|b| EngineBar {
                                    timestamp: format_ts(b.ts_ms, chart.timeframe),
                                    open: b.open,
                                    high: b.high,
                                    low: b.low,
                                    close: b.close,
                                    volume: b.volume,
                                })
                                .collect();
                            let fast_r: (usize, usize) = parse_range(&self.opt_fast_range, 5, 50);
                            let slow_r: (usize, usize) =
                                parse_range(&self.opt_slow_range, 20, 200);
                            let equity: f64 = self
                                .bt_equity
                                .replace(['$', ','], "")
                                .parse()
                                .unwrap_or(10000.0);
                            let windows: usize = self.wf_windows_count.parse().unwrap_or(5);
                            self.wf_result = Some(backtest::walk_forward(
                                &engine_bars,
                                fast_r.0..fast_r.1,
                                slow_r.0..slow_r.1,
                                windows,
                                equity,
                            ));
                            self.log.push_back(LogEntry::info(format!(
                                "Walk-forward complete: {} windows",
                                windows
                            )));
                        }
                    }
                });

                if let Some(ref wf) = self.wf_result {
                    ui.add_space(4.0);
                    let rob_c = if wf.robustness_score > 0.5 {
                        UP
                    } else if wf.robustness_score > 0.25 {
                        egui::Color32::from_rgb(241, 196, 15)
                    } else {
                        DOWN
                    };
                    egui::Grid::new("wf_summary")
                        .striped(true)
                        .num_columns(4)
                        .show(ui, |ui| {
                            ui.label("OOS Sharpe:");
                            ui.label(format!("{:.3}", wf.oos_sharpe));
                            ui.label("OOS PF:");
                            ui.label(format!("{:.2}", wf.oos_profit_factor));
                            ui.end_row();
                            ui.label("OOS Win%:");
                            ui.label(format!("{:.1}%", wf.oos_win_rate * 100.0));
                            ui.label("Robustness:");
                            ui.label(
                                egui::RichText::new(format!("{:.2}", wf.robustness_score))
                                    .color(rob_c),
                            );
                            ui.end_row();
                            ui.label("Best Params:");
                            ui.label(format!("Fast={} Slow={}", wf.best_params.0, wf.best_params.1));
                            ui.label("");
                            ui.label("");
                            ui.end_row();
                        });
                    if !wf.windows.is_empty() {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Per-Window Results").small().strong());
                        egui::Grid::new("wf_windows")
                            .striped(true)
                            .num_columns(6)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("#").color(AXIS_TEXT).small());
                                ui.label(
                                    egui::RichText::new("Fast/Slow").color(AXIS_TEXT).small(),
                                );
                                ui.label(
                                    egui::RichText::new("IS Sharpe").color(AXIS_TEXT).small(),
                                );
                                ui.label(
                                    egui::RichText::new("OOS Sharpe").color(AXIS_TEXT).small(),
                                );
                                ui.label(
                                    egui::RichText::new("OOS P&L").color(AXIS_TEXT).small(),
                                );
                                ui.label(egui::RichText::new("Trades").color(AXIS_TEXT).small());
                                ui.end_row();
                                for w in &wf.windows {
                                    ui.label(format!("{}", w.window_idx + 1));
                                    ui.label(format!("{}/{}", w.best_fast, w.best_slow));
                                    ui.label(format!("{:.3}", w.is_sharpe));
                                    let oos_c = if w.oos_sharpe > 0.0 { UP } else { DOWN };
                                    ui.label(
                                        egui::RichText::new(format!("{:.3}", w.oos_sharpe))
                                            .color(oos_c),
                                    );
                                    let pnl_c = if w.oos_pnl >= 0.0 { UP } else { DOWN };
                                    ui.label(
                                        egui::RichText::new(format!("${:.0}", w.oos_pnl))
                                            .color(pnl_c),
                                    );
                                    ui.label(format!("{}", w.oos_trades));
                                    ui.end_row();
                                }
                            });
                    }
                }
            });
        self.show_optimizer = show_optimizer;
    }
}
