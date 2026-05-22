use super::*;

impl TyphooNApp {
    pub(super) fn render_connect_window(
        &mut self,
        ctx: &egui::Context,
        mut settings_save_after: bool,
    ) {
        if !self.show_connect {
            return;
        }
        let mut show_connect = self.show_connect;
        egui::Window::new("Connect to Broker")
            .open(&mut show_connect)
            .resizable(true)
            .default_size([450.0, 300.0])
            .show(ctx, |ui| {
                ui.heading("Alpaca Markets");
                ui.separator();
                egui::Grid::new("broker_grid")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("API Key:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.broker_api_key)
                                .desired_width(250.0)
                                .password(true),
                        );
                        ui.end_row();
                        ui.label("Secret:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.broker_secret)
                                .desired_width(250.0)
                                .password(true),
                        );
                        ui.end_row();
                        ui.label("Mode:");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.broker_paper, true, "Paper");
                            ui.radio_value(&mut self.broker_paper, false, "Live");
                        });
                        ui.end_row();
                    });
                ui.add_space(5.0);
                ui.horizontal_wrapped(|ui| {
                    if ui.checkbox(&mut self.alpaca_enabled, "Enable Alpaca").changed() {
                        settings_save_after = true;
                        if !self.alpaca_enabled {
                            self.broker_connected = false;
                            self.live_account = None;
                            self.live_positions.clear();
                            self.live_orders.clear();
                            self.pending_alpaca_fetches.clear();
                            self.log.push_back(LogEntry::info("Alpaca disabled — no login/sync/position/order activity. Cache data preserved."));
                        }
                    }
                });
                let connect_label = if self.broker_connected {
                    egui::RichText::new("Alpaca Connected").color(UP)
                } else {
                    egui::RichText::new("Connect")
                };
                if ui.add_enabled(self.alpaca_enabled, egui::Button::new(connect_label)).clicked() && !self.broker_connected {
                    if self.broker_api_key.is_empty() || self.broker_secret.is_empty() {
                        self.log
                            .push_back(LogEntry::warn("Enter API key and secret"));
                    } else {
                        let capacity = self.alpaca_sync_capacity();
                        self.log.push_back(LogEntry::info(format!(
                            "Connecting to Alpaca {}...",
                            if self.broker_paper { "Paper" } else { "Live" }
                        )));
                        let _ = self.broker_tx.send(BrokerCmd::Connect {
                            api_key: self.broker_api_key.clone(),
                            secret: self.broker_secret.clone(),
                            paper: self.broker_paper,
                            bar_requests_per_minute: self.alpaca_effective_historical_rpm(),
                            fetch_permits: capacity.fetch_permits,
                        });
                    }
                }
                ui.add_space(10.0);
                ui.heading("tastytrade");
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Username:");
                    ui.add(egui::TextEdit::singleline(&mut self.tt_username).desired_width(200.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Password:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.tt_password)
                            .desired_width(200.0)
                            .password(true),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Mode:");
                    ui.radio_value(&mut self.tt_sandbox, true, "Sandbox");
                    ui.radio_value(&mut self.tt_sandbox, false, "Production");
                });
                if ui.checkbox(&mut self.tastytrade_enabled, "Enable tastytrade").changed() {
                    settings_save_after = true;
                    if !self.tastytrade_enabled {
                        self.tt_connected = false;
                        self.tt_positions.clear();
                        self.tt_balances = None;
                        self.pending_tastytrade_fetches.clear();
                        self.log.push_back(LogEntry::info("tastytrade disabled — no broker activity. Cache data preserved."));
                    }
                }
                if ui.add_enabled(self.tastytrade_enabled, egui::Button::new("Connect tastytrade")).clicked() {
                    if self.tt_username.is_empty() || self.tt_password.is_empty() {
                        self.log
                            .push_back(LogEntry::warn("Enter tastytrade username and password"));
                    } else {
                        if let Err(e) =
                            keyring::store(keyring::keys::TT_USERNAME, &self.tt_username)
                        {
                            self.log.push_back(LogEntry::warn(format!(
                                "Keyring store tt_username failed: {}",
                                e
                            )));
                        }
                        if let Err(e) =
                            keyring::store(keyring::keys::TT_PASSWORD, &self.tt_password)
                        {
                            self.log.push_back(LogEntry::warn(format!(
                                "Keyring store tt_password failed: {}",
                                e
                            )));
                        }
                        let _ = self.broker_tx.send(BrokerCmd::TastytradeConnect {
                            username: self.tt_username.clone(),
                            password: self.tt_password.clone(),
                            sandbox: self.tt_sandbox,
                        });
                        self.log.push_back(LogEntry::info(format!(
                            "tastytrade {} — connecting...",
                            if self.tt_sandbox {
                                "Sandbox"
                            } else {
                                "Production"
                            }
                        )));
                    }
                }
                ui.add_space(10.0);
                ui.heading("Data APIs");
                ui.separator();
                egui::Grid::new("api_keys_grid")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("Finnhub API Key:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.finnhub_key)
                                .desired_width(200.0)
                                .password(true),
                        );
                        ui.end_row();
                    });
                ui.label(
                    egui::RichText::new(
                        "Used for: News, Analyst Ratings, Insider Sentiment, Short Interest",
                    )
                    .color(AXIS_TEXT)
                    .small(),
                );
                ui.add_space(10.0);
                ui.heading("MT5 (view-only data source)");
                ui.separator();
                ui.label("MT5 bar data imported via BarCacheWriter EA → SQLite cache.");
                ui.label("Trade management stays in MT5. DARWIN analytics via XLSX import.");
            });
        self.show_connect = show_connect;
        if settings_save_after {
            self.save_session();
        }
    }

    pub(super) fn render_indicators_window(&mut self, ctx: &egui::Context) {
        if !self.show_indicators_panel {
            return;
        }
        let mut show_indicators_panel = self.show_indicators_panel;
        egui::Window::new("Indicators")
            .open(&mut show_indicators_panel)
            .resizable(true)
            .default_size([450.0, 400.0])
            .show(ctx, |ui| {
                ui.heading("Presets");
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("TyphooN (NNFX)").clicked() {
                        self.show_sma100 = false;
                        self.show_ema21 = false;
                        self.show_bollinger = false;
                        self.show_ichimoku = false;
                        self.show_wma = false;
                        self.show_hma = false;
                        self.show_psar = false;
                        self.show_rsi = false;
                        self.show_macd = false;
                        self.show_stochastic = false;
                        self.show_adx = false;
                        self.show_cci = false;
                        self.show_williams_r = false;
                        self.show_obv = false;
                        self.show_momentum = false;
                        self.show_cmo = false;
                        self.show_qstick = false;
                        self.show_disparity = false;
                        self.show_bop = false;
                        self.show_stddev = false;
                        self.show_mfi = false;
                        self.show_trix = false;
                        self.show_ppo = false;
                        self.show_ultosc = false;
                        self.show_stochrsi = false;
                        self.show_var_oscillator = false;
                        self.show_volume_pane = false;
                        self.show_fractals = false;
                        self.show_harmonics = false;
                        self.show_ehlers_ss = false;
                        self.show_ehlers_decycler = false;
                        self.show_ehlers_itl = false;
                        self.show_ehlers_mama = false;
                        self.show_ehlers_ebsw = false;
                        self.show_ehlers_cyber = false;
                        self.show_ehlers_cg = false;
                        self.show_ehlers_roof = false;
                        self.show_pivots = false;
                        self.show_vol_heatmap = false;
                        self.show_vwap = false;
                        self.show_price_histogram = false;
                        self.show_sessions = false;
                        self.show_supertrend = false;
                        self.show_donchian = false;
                        self.show_keltner = false;
                        self.show_regression = false;
                        self.show_fvg = false;
                        self.show_order_blocks = false;
                        self.show_squeeze = false;
                        self.show_atr_proj = true;
                        self.show_prev_levels = true;
                        self.show_kama = true;
                        self.show_sma200 = true;
                        self.show_supply_demand = true;
                        self.show_auto_fib = false;
                        self.show_fisher = true;
                        self.show_better_volume = true;
                        self.log.push_back(LogEntry::info(
                            "Preset: TyphooN (NNFX) — ATR_Proj + PrevLevels + MultiKAMA + MTF_MA + S/D + Fisher + BVol",
                        ));
                    }
                    if ui.button("Carney").clicked() {
                        self.show_sma200 = false;
                        self.show_sma100 = false;
                        self.show_kama = false;
                        self.show_ema21 = false;
                        self.show_bollinger = false;
                        self.show_ichimoku = false;
                        self.show_wma = false;
                        self.show_hma = false;
                        self.show_psar = false;
                        self.show_atr_proj = false;
                        self.show_prev_levels = false;
                        self.show_pivots = false;
                        self.show_supply_demand = false;
                        self.show_auto_fib = false;
                        self.show_rsi = false;
                        self.show_macd = false;
                        self.show_stochastic = false;
                        self.show_adx = false;
                        self.show_cci = false;
                        self.show_williams_r = false;
                        self.show_obv = false;
                        self.show_momentum = false;
                        self.show_cmo = false;
                        self.show_qstick = false;
                        self.show_disparity = false;
                        self.show_bop = false;
                        self.show_stddev = false;
                        self.show_mfi = false;
                        self.show_trix = false;
                        self.show_ppo = false;
                        self.show_ultosc = false;
                        self.show_stochrsi = false;
                        self.show_var_oscillator = false;
                        self.show_fisher = true;
                        self.show_harmonics = true;
                        self.show_better_volume = true;
                        self.show_fractals = true;
                        self.log.push_back(LogEntry::info(
                            "Preset: Carney — Ehlers Fisher + Harmonics (10 XABCD) + BetterVolume + Fractals",
                        ));
                    }
                    if ui.button("Clean").clicked() {
                        self.show_sma200 = false;
                        self.show_sma100 = false;
                        self.show_kama = false;
                        self.show_ema21 = false;
                        self.show_bollinger = false;
                        self.show_ichimoku = false;
                        self.show_wma = false;
                        self.show_hma = false;
                        self.show_psar = false;
                        self.show_atr_proj = false;
                        self.show_prev_levels = false;
                        self.show_pivots = false;
                        self.show_supply_demand = false;
                        self.show_auto_fib = false;
                        self.show_rsi = false;
                        self.show_fisher = false;
                        self.show_macd = false;
                        self.show_stochastic = false;
                        self.show_adx = false;
                        self.show_cci = false;
                        self.show_williams_r = false;
                        self.show_obv = false;
                        self.show_momentum = false;
                        self.show_cmo = false;
                        self.show_qstick = false;
                        self.show_disparity = false;
                        self.show_bop = false;
                        self.show_stddev = false;
                        self.show_mfi = false;
                        self.show_trix = false;
                        self.show_ppo = false;
                        self.show_ultosc = false;
                        self.show_stochrsi = false;
                        self.show_var_oscillator = false;
                        self.show_better_volume = false;
                        self.show_volume_pane = false;
                        self.show_fractals = false;
                        self.show_harmonics = false;
                        self.show_ehlers_ss = false;
                        self.show_ehlers_decycler = false;
                        self.show_ehlers_itl = false;
                        self.show_ehlers_mama = false;
                        self.show_ehlers_ebsw = false;
                        self.show_ehlers_cyber = false;
                        self.show_ehlers_cg = false;
                        self.show_ehlers_roof = false;
                        self.log
                            .push_back(LogEntry::info("All indicators disabled"));
                    }
                });

                ui.add_space(4.0);
                ui.heading("Moving Averages");
                ui.separator();
                ui.checkbox(
                    &mut self.show_sma200,
                    "MTF_MA — SMA(200/100) H1/H4/D1/W1/MN1",
                );
                ui.checkbox(&mut self.show_sma100, "SMA(100)");
                ui.checkbox(&mut self.show_kama, "MultiKAMA(10,2,30) — H1/H4/D1/W1/MN1");
                ui.checkbox(&mut self.show_ema21, "EMA(21)");
                ui.checkbox(&mut self.show_wma, "WMA(20)");
                ui.checkbox(&mut self.show_hma, "HMA(20)");

                ui.add_space(4.0);
                ui.heading("Bands, Cloud & Levels");
                ui.separator();
                ui.checkbox(&mut self.show_bollinger, "Bollinger Bands(20,2)");
                ui.checkbox(&mut self.show_supertrend, "Supertrend(10,3)");
                ui.checkbox(&mut self.show_donchian, "Donchian Channels(20)");
                ui.checkbox(&mut self.show_keltner, "Keltner Channels(20,1.5)");
                ui.checkbox(&mut self.show_regression, "Regression Channel(20,2σ)");
                ui.checkbox(&mut self.show_ichimoku, "Ichimoku Cloud(9,26,52)");
                ui.checkbox(&mut self.show_psar, "Parabolic SAR(0.02,0.2)");
                ui.checkbox(
                    &mut self.show_atr_proj,
                    "ATR Projection MTF (M15/H1/H4/D1/W1/MN1)",
                );
                ui.checkbox(
                    &mut self.show_prev_levels,
                    "Previous Candle Levels (H1/H4/D1/W1/MN1)",
                );
                ui.checkbox(&mut self.show_pivots, "Pivot Points (Classic)");
                ui.checkbox(&mut self.show_supply_demand, "Supply/Demand Zones");
                ui.checkbox(&mut self.show_fvg, "Fair Value Gaps (3-bar imbalance)");
                ui.checkbox(&mut self.show_order_blocks, "Order Blocks (ICT/Smart Money)");

                ui.add_space(4.0);
                ui.heading("Chart Overlays");
                ui.separator();
                ui.checkbox(&mut self.show_sessions, "Trading Sessions (Asian/London/NY)");
                ui.checkbox(&mut self.show_vol_heatmap, "Volume Heatmap Candles");
                ui.checkbox(&mut self.show_vwap, "VWAP (daily anchor, 1σ/2σ/3σ bands)");
                ui.checkbox(
                    &mut self.show_price_histogram,
                    "Price Distribution (time-at-level)",
                );

                ui.add_space(4.0);
                ui.heading("Pattern Recognition");
                ui.separator();
                ui.checkbox(&mut self.show_fractals, "Fractals (Bill Williams)");
                ui.checkbox(
                    &mut self.show_harmonics,
                    "Harmonic Patterns (Scott Carney — 10 XABCD)",
                );
                ui.checkbox(&mut self.show_auto_fib, "Auto Fibonacci (fractal swing)");

                ui.add_space(4.0);
                ui.heading("Oscillators (Sub-Pane)");
                ui.separator();
                ui.checkbox(&mut self.show_rsi, "RSI(14)");
                ui.checkbox(&mut self.show_macd, "MACD(12,26,9)");
                ui.checkbox(&mut self.show_stochastic, "Stochastic(14,3,3)");
                ui.checkbox(&mut self.show_adx, "ADX(14)");
                ui.checkbox(&mut self.show_cci, "CCI(20)");
                ui.checkbox(&mut self.show_williams_r, "Williams %R(14)");
                ui.checkbox(&mut self.show_obv, "OBV");
                ui.checkbox(&mut self.show_momentum, "Momentum(10)");
                ui.checkbox(&mut self.show_cmo, "CMO(9)");
                ui.checkbox(&mut self.show_qstick, "QStick(14)");
                ui.checkbox(&mut self.show_disparity, "Disparity(14)");
                ui.checkbox(&mut self.show_bop, "BOP(14)");
                ui.checkbox(&mut self.show_stddev, "StdDev(20)");
                ui.checkbox(&mut self.show_mfi, "MFI(14)");
                ui.checkbox(&mut self.show_trix, "TRIX(15,9)");
                ui.checkbox(&mut self.show_ppo, "PPO(12,26,9)");
                ui.checkbox(&mut self.show_ultosc, "ULTOSC(7,14,28)");
                ui.checkbox(&mut self.show_stochrsi, "StochRSI(14,14,3,3)");
                ui.checkbox(&mut self.show_var_oscillator, "VaR Oscillator(20,95%)");
                ui.checkbox(&mut self.show_better_volume, "Better Volume");
                ui.checkbox(&mut self.show_volume_pane, "Volume");
                ui.checkbox(&mut self.show_squeeze, "Squeeze Momentum (BB inside KC)");
                ui.checkbox(&mut self.show_fvg, "Fair Value Gaps (3-bar imbalance)");
                ui.checkbox(&mut self.show_order_blocks, "Order Blocks (ICT/Smart Money)");

                ui.add_space(4.0);
                ui.heading("Ehlers DSP");
                ui.separator();
                ui.label(egui::RichText::new("Overlay").color(AXIS_TEXT).small());
                ui.checkbox(&mut self.show_ehlers_ss, "Super Smoother(10)");
                ui.checkbox(&mut self.show_ehlers_decycler, "Decycler(20)");
                ui.checkbox(&mut self.show_ehlers_itl, "Instantaneous Trendline");
                ui.checkbox(&mut self.show_ehlers_mama, "MAMA / FAMA");
                ui.label(egui::RichText::new("Sub-Pane").color(AXIS_TEXT).small());
                ui.checkbox(&mut self.show_fisher, "Ehlers Fisher Transform(32)");
                ui.checkbox(&mut self.show_ehlers_ebsw, "Even Better Sinewave");
                ui.checkbox(&mut self.show_ehlers_cyber, "Cyber Cycle");
                ui.checkbox(&mut self.show_ehlers_cg, "CG Oscillator(10)");
                ui.checkbox(&mut self.show_ehlers_roof, "Roofing Filter(10,48)");

                ui.add_space(4.0);
                ui.heading("Indicator Periods");
                ui.separator();
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    let mut changed = false;
                    egui::Grid::new("ind_periods")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("SMA Slow:");
                            if ui
                                .add(
                                    egui::DragValue::new(&mut chart.sma_slow_period)
                                        .range(5..=500),
                                )
                                .changed()
                            {
                                changed = true;
                            }
                            ui.end_row();
                            ui.label("SMA Fast:");
                            if ui
                                .add(
                                    egui::DragValue::new(&mut chart.sma_fast_period)
                                        .range(5..=500),
                                )
                                .changed()
                            {
                                changed = true;
                            }
                            ui.end_row();
                            ui.label("EMA:");
                            if ui
                                .add(egui::DragValue::new(&mut chart.ema_period).range(2..=200))
                                .changed()
                            {
                                changed = true;
                            }
                            ui.end_row();
                            ui.label("RSI:");
                            if ui
                                .add(egui::DragValue::new(&mut chart.rsi_period).range(2..=100))
                                .changed()
                            {
                                changed = true;
                            }
                            ui.end_row();
                            ui.label("ATR:");
                            if ui
                                .add(egui::DragValue::new(&mut chart.atr_period).range(2..=100))
                                .changed()
                            {
                                changed = true;
                            }
                            ui.end_row();
                            ui.label("Bollinger:");
                            if ui
                                .add(egui::DragValue::new(&mut chart.bb_period).range(5..=100))
                                .changed()
                            {
                                changed = true;
                            }
                            ui.end_row();
                            ui.label("Stochastic:");
                            if ui
                                .add(
                                    egui::DragValue::new(&mut chart.stoch_period).range(2..=100),
                                )
                                .changed()
                            {
                                changed = true;
                            }
                            ui.end_row();
                            ui.label("ADX:");
                            if ui
                                .add(egui::DragValue::new(&mut chart.adx_period).range(2..=100))
                                .changed()
                            {
                                changed = true;
                            }
                            ui.end_row();
                            ui.label("Fisher:");
                            if ui
                                .add(
                                    egui::DragValue::new(&mut chart.fisher_period)
                                        .range(5..=100),
                                )
                                .changed()
                            {
                                changed = true;
                            }
                            ui.end_row();
                            ui.label("Momentum:");
                            if ui
                                .add(
                                    egui::DragValue::new(&mut chart.momentum_period)
                                        .range(2..=100),
                                )
                                .changed()
                            {
                                changed = true;
                            }
                            ui.end_row();
                            ui.label("MACD Fast:");
                            if ui
                                .add(egui::DragValue::new(&mut chart.macd_fast).range(2..=50))
                                .changed()
                            {
                                changed = true;
                            }
                            ui.end_row();
                            ui.label("MACD Slow:");
                            if ui
                                .add(egui::DragValue::new(&mut chart.macd_slow).range(5..=100))
                                .changed()
                            {
                                changed = true;
                            }
                            ui.end_row();
                            ui.label("MACD Signal:");
                            if ui
                                .add(
                                    egui::DragValue::new(&mut chart.macd_signal_p)
                                        .range(2..=50),
                                )
                                .changed()
                            {
                                changed = true;
                            }
                            ui.end_row();
                        });
                    if changed {
                        self.indicators_dirty = true;
                    }
                }
            });
        self.show_indicators_panel = show_indicators_panel;
    }

    pub(super) fn render_risk_calc_window(&mut self, ctx: &egui::Context) {
        if !self.show_risk_calc {
            return;
        }
        let mut show_risk_calc = self.show_risk_calc;
        egui::Window::new("Risk Calculator")
            .open(&mut show_risk_calc)
            .resizable(true)
            .default_size([400.0, 400.0])
            .show(ctx, |ui| {
                ui.heading("Position Sizing");
                ui.separator();
                egui::Grid::new("risk_calc_grid")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("Account Equity:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.rc_equity).desired_width(120.0),
                        );
                        ui.end_row();
                        ui.label("Risk %:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.rc_risk_pct).desired_width(120.0),
                        );
                        ui.end_row();
                        ui.label("Entry Price:");
                        ui.add(egui::TextEdit::singleline(&mut self.rc_entry).desired_width(120.0));
                        ui.end_row();
                        ui.label("Stop Loss:");
                        ui.add(egui::TextEdit::singleline(&mut self.rc_sl).desired_width(120.0));
                        ui.end_row();
                        ui.label("Take Profit:");
                        ui.add(egui::TextEdit::singleline(&mut self.rc_tp).desired_width(120.0));
                        ui.end_row();
                        ui.label("Tick Value:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.rc_tick_value)
                                .desired_width(120.0),
                        );
                        ui.end_row();
                        ui.label("Tick Size:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.rc_tick_size).desired_width(120.0),
                        );
                        ui.end_row();
                    });
                ui.add_space(10.0);
                if ui.button("Calculate").clicked() {
                    let equity: f64 = self
                        .rc_equity
                        .replace(['$', ','], "")
                        .parse()
                        .unwrap_or(0.0);
                    let risk_pct: f64 = self.rc_risk_pct.parse().unwrap_or(1.0);
                    let entry: f64 = self.rc_entry.parse().unwrap_or(0.0);
                    let sl: f64 = self.rc_sl.parse().unwrap_or(0.0);
                    let tp: f64 = self.rc_tp.parse().unwrap_or(0.0);
                    let tick_val: f64 = self.rc_tick_value.parse().unwrap_or(1.0);
                    let tick_sz: f64 = self.rc_tick_size.parse().unwrap_or(0.01);

                    if equity > 0.0 && entry > 0.0 && sl > 0.0 {
                        let sl_distance = (entry - sl).abs();
                        let risk_amount = equity * risk_pct / 100.0;
                        let spec = risk::SymbolSpec {
                            symbol: self.symbol_input.clone(),
                            tick_size: tick_sz,
                            tick_value: tick_val,
                            volume_min: 0.01,
                            volume_max: 100.0,
                            volume_step: 0.01,
                            contract_size: 1.0,
                            margin_rate: 0.0,
                        };
                        let lots = risk::risk_lots(&spec, risk_amount, sl_distance);
                        let rr = if tp > 0.0 && sl_distance > 0.0 {
                            (tp - entry).abs() / sl_distance
                        } else {
                            0.0
                        };
                        self.rc_result = format!(
                            "Lots: {:.2}\nRisk: ${:.2} ({:.1}%)\nSL Distance: {}\nR:R = {:.2}",
                            lots,
                            risk_amount,
                            risk_pct,
                            format_price(sl_distance),
                            rr
                        );

                        let usable = margin::usable_margin(equity, 0.0, 10.0);
                        self.rc_result
                            .push_str(&format!("\nUsable margin: ${:.2}", usable));
                    } else {
                        self.rc_result = "Enter equity, entry, and SL".to_string();
                    }
                }
                ui.separator();
                if !self.rc_result.is_empty() {
                    ui.label(
                        egui::RichText::new(&self.rc_result)
                            .monospace()
                            .color(egui::Color32::from_rgb(200, 220, 255)),
                    );

                    let entry: f64 = self.rc_entry.parse().unwrap_or(0.0);
                    let sl: f64 = self.rc_sl.parse().unwrap_or(0.0);
                    let tp: f64 = self.rc_tp.parse().unwrap_or(0.0);
                    if entry > 0.0 && sl > 0.0 && tp > 0.0 {
                        let sl_dist = (entry - sl).abs();
                        let tp_dist = (tp - entry).abs();
                        let total = sl_dist + tp_dist;
                        if total > 0.0 {
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new("Risk / Reward").strong());
                            let bar_w = 340.0_f32;
                            let bar_h = 22.0_f32;
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(bar_w, bar_h),
                                egui::Sense::hover(),
                            );
                            let painter = ui.painter_at(rect);
                            let risk_w = (sl_dist / total) as f32 * bar_w;
                            let reward_w = (tp_dist / total) as f32 * bar_w;
                            painter.rect_filled(
                                egui::Rect::from_min_size(rect.min, egui::vec2(risk_w, bar_h)),
                                2.0,
                                egui::Color32::from_rgb(220, 50, 50),
                            );
                            painter.rect_filled(
                                egui::Rect::from_min_size(
                                    egui::pos2(rect.left() + risk_w, rect.top()),
                                    egui::vec2(reward_w, bar_h),
                                ),
                                2.0,
                                egui::Color32::from_rgb(50, 200, 70),
                            );
                            painter.text(
                                egui::pos2(rect.left() + risk_w * 0.5, rect.center().y),
                                egui::Align2::CENTER_CENTER,
                                format!("Risk {}", format_price(sl_dist)),
                                egui::FontId::proportional(10.0),
                                egui::Color32::WHITE,
                            );
                            painter.text(
                                egui::pos2(rect.left() + risk_w + reward_w * 0.5, rect.center().y),
                                egui::Align2::CENTER_CENTER,
                                format!("Reward {}", format_price(tp_dist)),
                                egui::FontId::proportional(10.0),
                                egui::Color32::WHITE,
                            );
                        }
                    }
                }
            });
        self.show_risk_calc = show_risk_calc;
    }

    pub(super) fn render_compound_calc_window(&mut self, ctx: &egui::Context) {
        if !self.show_compound_calc {
            return;
        }
        let mut show_compound_calc = self.show_compound_calc;
        egui::Window::new("Compound Interest Calculator")
            .open(&mut show_compound_calc)
            .resizable(true)
            .default_size([500.0, 450.0])
            .show(ctx, |ui| {
                egui::Grid::new("ci_grid").num_columns(2).show(ui, |ui| {
                    ui.label("Principal ($):");
                    ui.add(egui::TextEdit::singleline(&mut self.ci_principal).desired_width(120.0));
                    ui.end_row();
                    ui.label("Annual Return (%):");
                    ui.add(egui::TextEdit::singleline(&mut self.ci_rate).desired_width(80.0));
                    ui.end_row();
                    ui.label("Years:");
                    ui.add(egui::TextEdit::singleline(&mut self.ci_years).desired_width(60.0));
                    ui.end_row();
                    ui.label("Compounds/Year:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.ci_compounds).desired_width(60.0),
                    );
                    ui.end_row();
                    ui.label("Monthly Add ($):");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.ci_contribution).desired_width(120.0),
                    );
                    ui.end_row();
                });
                ui.horizontal(|ui| {
                    if ui.button("Calculate").clicked() {
                        let p = self
                            .ci_principal
                            .replace(['$', ','], "")
                            .parse::<f64>()
                            .unwrap_or(10000.0);
                        let r = self.ci_rate.parse::<f64>().unwrap_or(10.0) / 100.0;
                        let y = self.ci_years.parse::<u32>().unwrap_or(10);
                        let n = self.ci_compounds.parse::<f64>().unwrap_or(12.0);
                        let monthly = self
                            .ci_contribution
                            .replace(['$', ','], "")
                            .parse::<f64>()
                            .unwrap_or(0.0);
                        self.ci_result.clear();
                        let mut balance = p;
                        let mut total_contrib = p;
                        for year in 0..=y {
                            self.ci_result.push((year as f64, balance, total_contrib));
                            for _ in 0..n as u32 {
                                balance *= 1.0 + r / n;
                                balance += monthly * 12.0 / n;
                            }
                            total_contrib += monthly * 12.0;
                        }
                    }
                    let has_curve = self.bg.equity_curve.len() >= 30;
                    if ui
                        .add_enabled(has_curve, egui::Button::new("Use My Equity Curve"))
                        .on_hover_text(
                            "Pre-fill principal and annual return from your actual DARWIN portfolio history",
                        )
                        .clicked()
                    {
                        let curve = &self.bg.equity_curve;
                        if let (Some(first), Some(last)) = (curve.first(), curve.last()) {
                            let first_val = first.1.max(1.0);
                            let last_val = last.1;
                            let n_days = curve.len() as f64;
                            let n_years = (n_days / 252.0).max(1.0 / 252.0);
                            let total_return = last_val / first_val;
                            let cagr = total_return.powf(1.0 / n_years) - 1.0;
                            self.ci_principal = format!("{:.2}", last_val);
                            self.ci_rate = format!("{:.2}", cagr * 100.0);
                            self.log.push_back(LogEntry::info(format!(
                                "Prefilled from equity curve: ${:.0} @ {:.2}% CAGR over {:.1}y",
                                last_val,
                                cagr * 100.0,
                                n_years
                            )));
                        }
                    }
                });
                if !self.ci_result.is_empty() {
                    ui.separator();
                    let final_bal = self.ci_result.last().map(|r| r.1).unwrap_or(0.0);
                    let total_cont = self.ci_result.last().map(|r| r.2).unwrap_or(0.0);
                    let interest_earned = final_bal - total_cont;
                    ui.label(
                        egui::RichText::new(format!("Final Balance: ${:.2}", final_bal))
                            .strong()
                            .color(UP),
                    );
                    ui.label(format!("Total Contributed: ${:.2}", total_cont));
                    ui.label(
                        egui::RichText::new(format!("Interest Earned: ${:.2}", interest_earned))
                            .color(ACCENT),
                    );
                    ui.separator();
                    let bal_pts: PlotPoints = PlotPoints::new(
                        self.ci_result.iter().map(|(y, b, _)| [*y, *b]).collect(),
                    );
                    let cont_pts: PlotPoints = PlotPoints::new(
                        self.ci_result.iter().map(|(y, _, c)| [*y, *c]).collect(),
                    );
                    let bal_line = Line::new("Balance", bal_pts).color(UP).width(2.0);
                    let cont_line = Line::new("Contributions", cont_pts).color(ACCENT).width(1.0);
                    Plot::new("ci_plot")
                        .height(200.0)
                        .allow_drag(false)
                        .allow_zoom(false)
                        .legend(egui_plot::Legend::default())
                        .show(ui, |plot_ui| {
                            plot_ui.line(bal_line);
                            plot_ui.line(cont_line);
                        });
                }
            });
        self.show_compound_calc = show_compound_calc;
    }
}
