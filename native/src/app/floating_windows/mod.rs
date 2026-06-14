use super::*;
mod alert_market_data_windows;
mod bookmap;
mod broker_kraken_windows;
mod company_info;
use bookmap::*;
mod news_filter;
use news_filter::*;
mod bardata;
mod macro_windows;
mod market_analytics_windows;
mod matrix_chat;
mod news;
mod reddit;
mod research_adr107;
mod research_advanced_moving_averages;
mod research_aroon_macd_variable_ma;
mod research_autocorrelation_hurst_volume;
mod research_behavior_distribution_stats;
mod research_calmar_ulcer_liquidity_normality;
mod research_candlestick_core_patterns;
mod research_candlestick_marubozu_line_patterns;
mod research_candlestick_reversal_continuation;
mod research_corporate_actions_analyst_esg;
mod research_directional_moneyflow_sar;
mod research_directional_movement_family;
mod research_dividends_earnings_upgrades;
mod research_downside_efficiency_wick_volatility;
mod research_ehlers_adaptive_ma_oscillators;
mod research_entropy_stationarity_recovery;
mod research_entropy_tail_autocorrelation;
mod research_factor_quality_credit_models;
mod research_factor_ranking_extensions;
mod research_financials_management_cot;
mod research_fractal_tail_nonlinear_rank;
mod research_fx_beta_valuation_identifiers;
mod research_gap_volatility_mean_reversion;
mod research_garch_bubble_dimension_information;
mod research_global_market_cost_capital;
mod research_ichimoku_supertrend_channels;
mod research_ingest;
mod research_insider_dividend_earnings_momentum;
mod research_jump_unitroot_multifractal_tsi;
mod research_laguerre_pivot_midpoint_models;
mod research_leverage_quality_volatility_shorts;
mod research_linearreg_hilbert_phase;
mod research_linearreg_hilbert_stochastic;
mod research_massindex_atr_squeeze_force;
mod research_momentum_gap_atr_drawdown;
mod research_moving_average_regression_pivots;
mod research_normality_lmoments_price_impact;
mod research_ohlc_price_transforms;
mod research_ohlc_volatility_cvar_calendar;
mod research_omega_fractal_burke_seasonality;
mod research_oscillator_price_momentum;
mod research_ownership_float_price_earnings;
mod research_portmanteau_ou_long_memory_spectrum;
mod research_quant_risk_nonlinearity;
mod research_rate_of_change_correlation;
mod research_residual_iid_heteroskedastic_cycles;
mod research_return_risk_dcf_options;
mod research_robust_entropy_quantile_volatility;
mod research_seasonality_correlation_technicals;
mod research_sector_factor_drift_ranks;
mod research_sharpe_stationarity_jump_drawdown;
mod research_solvency_scores_volatility_targets;
mod research_squeeze_breakout_channels;
mod research_sterling_kelly_stat_tests;
mod research_tail_arch_pain_structural_var;
mod research_upside_leverage_drawdown_var;
mod research_volume_flow_trend_oscillators;
mod research_volume_momentum_oscillators;
mod research_zero_lag_elder_forecast_balance;
mod risk_journal_windows;
mod scope;
mod scrape_status_windows;
mod screenshots;
mod sec_calendar_windows;
mod storage_sync_windows;
mod symbol_explorer;
mod symbol_screener;
mod trading_tools_windows;
mod workspace_reference_windows;

fn sortable_header(
    ui: &mut egui::Ui,
    label: &str,
    col: usize,
    sort_col: &mut usize,
    sort_asc: &mut bool,
) {
    let arrow = if *sort_col == col {
        if *sort_asc { " ↑" } else { " ↓" }
    } else {
        ""
    };
    if ui
        .add(egui::Button::new(
            egui::RichText::new(format!("{label}{arrow}"))
                .small()
                .strong(),
        ))
        .on_hover_text("Sort by this column")
        .clicked()
    {
        if *sort_col == col {
            *sort_asc = !*sort_asc;
        } else {
            *sort_col = col;
            *sort_asc = true;
        }
    }
}

impl TyphooNApp {
    pub(super) fn draw_floating_windows(&mut self, ctx: &egui::Context) {
        // Per-window render timing: names the culprit when a single floating
        // window blows the frame budget (a synchronous cache scan, an
        // un-virtualized list, etc.). Each render early-returns on its show_*
        // flag, so this only fires for windows that are actually open —
        // diagnostic for the rare multi-second `floating_windows_ms` spikes.
        macro_rules! timed_window {
            ($name:literal, $call:expr) => {{
                let _t = std::time::Instant::now();
                $call;
                let _ms = _t.elapsed().as_secs_f64() * 1000.0;
                if _ms > 150.0 {
                    tracing::warn!("slow floating window: {} took {:.0}ms", $name, _ms);
                }
            }};
        }
        // Settings
        // Save credentials to keyring + SQLite fallback when Settings window closes
        if self.was_settings_open && !self.show_settings {
            let creds = [
                (keyring::keys::ALPACA_API_KEY, self.broker_api_key.as_str()),
                (keyring::keys::ALPACA_SECRET, self.broker_secret.as_str()),
                (keyring::keys::FINNHUB_KEY, self.finnhub_key.as_str()),
                (keyring::keys::FRED_KEY, self.fred_key.as_str()),
                (
                    keyring::keys::DISCORD_WEBHOOK,
                    self.discord_webhook.as_str(),
                ),
                (keyring::keys::PUSHOVER_TOKEN, self.pushover_token.as_str()),
                (keyring::keys::PUSHOVER_USER, self.pushover_user.as_str()),
                (keyring::keys::NTFY_TOPIC, self.ntfy_topic.as_str()),
                (keyring::keys::ANTHROPIC_KEY, self.anthropic_key.as_str()),
                (keyring::keys::OPENAI_KEY, self.openai_key.as_str()),
                (keyring::keys::KRAKEN_API_KEY, self.kraken_api_key.as_str()),
                (
                    keyring::keys::KRAKEN_API_SECRET,
                    self.kraken_api_secret.as_str(),
                ),
                (
                    keyring::keys::KRAKEN_WS_API_KEY,
                    self.kraken_ws_api_key.as_str(),
                ),
                (
                    keyring::keys::KRAKEN_WS_API_SECRET,
                    self.kraken_ws_api_secret.as_str(),
                ),
            ];
            let mut kr_ok = true;
            let mut saved_credentials: Vec<&'static str> = Vec::new();
            for (key, val) in &creds {
                if let Err(e) = keyring::store(key, val) {
                    kr_ok = false;
                    self.log.push_back(LogEntry::warn(format!(
                        "Keyring store '{}' failed: {}",
                        key, e
                    )));
                } else {
                    saved_credentials.push(*key);
                }
                // Always write SQLite fallback
                if let Some(ref cache) = self.cache {
                    let _ = cache.put_kv(&format!("cred:{}", key), val);
                }
            }
            let dest = if kr_ok {
                "system keyring + SQLite"
            } else {
                "SQLite fallback (keyring unavailable)"
            };
            if !saved_credentials.is_empty() {
                self.log.push_back(LogEntry::info(format!(
                    "Credentials saved to {}: {}",
                    dest,
                    saved_credentials.join(", ")
                )));
            }
            // Also save session to persist non-credential settings (tt_sandbox, broker_paper, etc.)
            self.save_session();
        }
        self.was_settings_open = self.show_settings;

        let _settings_save_after = self.render_settings_window(ctx);
        // Broker connect + Kraken trade-history / open-orders windows
        self.render_broker_kraken_windows(ctx);

        // AI Chat (Anthropic Claude / OpenAI GPT / …)
        self.render_ai_chat_window(ctx);

        // ── Claude Code CLI chat ──
        self.render_claude_code_window(ctx);

        // ── Gemini CLI chat ──
        self.render_gemini_cli_window(ctx);

        // ── Codex CLI chat ──
        self.render_codex_cli_window(ctx);

        // ── Hermes Agent CLI chat ──
        self.render_hermes_cli_window(ctx);

        // ── Grok Build CLI chat ──
        self.render_grok_cli_window(ctx);

        // ── AI Sessions history browser ──
        self.render_ai_sessions_window(ctx);

        // ── Screenshots Gallery (palette: SCREENSHOTS / GALLERY) ──
        self.render_screenshots_gallery_window(ctx);

        // ── AI Response Cache stats window ──
        self.render_ai_cache_window(ctx);

        // Matrix Chat (public room viewer)
        self.render_matrix_chat_window(ctx);

        // BARDATA Progress Window
        self.render_bardata_progress_window(ctx);

        // Reddit WallStreetBets
        self.render_reddit_window(ctx);

        // Risk Calculator — wired to engine risk.rs
        // ── SCOPE popup window with source checkboxes ──
        self.render_scope_window(ctx);
        self.render_company_info_window(ctx);

        self.render_risk_calc_window(ctx);
        self.render_compound_calc_window(ctx);

        self.render_backtest_window(ctx);

        // Screener — uses cached symbol data
        timed_window!("symbol_screener", self.render_symbol_screener_window(ctx));

        // Symbols Explorer — all-encompassing symbol browser with broker hierarchy
        timed_window!("symbol_explorer", self.render_symbol_explorer_window(ctx));

        self.render_optimizer_window(ctx);

        // News
        timed_window!("news", self.render_news_window(ctx));

        // ── Godel parity research windows (ADR-107) ───────────────────────
        self.render_research_adr107_windows(ctx);

        // ── Research Godel Parity Round 2 windows ─────────────────────
        self.render_research_dividends_earnings_upgrades_windows(ctx);

        // ── Research Godel Parity Round 3 windows ─────────────────────
        self.render_research_financials_management_cot_windows(ctx);

        // ── Research Round 4 windows ──────────────────────────────────
        self.render_research_corporate_actions_analyst_esg_windows(ctx);

        // ── Research Round 5 windows ──────────────────────────────────
        self.render_research_ownership_float_price_earnings_windows(ctx);

        // ── Research Round 6 windows ──────────────────────────────────
        self.render_research_global_market_cost_capital_windows(ctx);

        // ── Research Godel Parity Round 7 ──
        self.render_research_fx_beta_valuation_identifiers_windows(ctx);

        // ── Research Round 8 windows ──
        self.render_research_return_risk_dcf_options_windows(ctx);

        // ── Research Round 9 windows ──
        self.render_research_seasonality_correlation_technicals_windows(ctx);

        // ── Research Godel Parity Round 10 ──
        self.render_research_leverage_quality_volatility_shorts_windows(ctx);

        // ── Research Godel Parity Round 11 windows ─────────────────────────────
        self.render_research_solvency_scores_volatility_targets_windows(ctx);

        // Research Round 12 windows
        self.render_research_insider_dividend_earnings_momentum_windows(ctx);

        // Research Rounds 13-15 windows
        self.render_research_factor_quality_credit_models_windows(ctx);

        // ── Research Round 16 ────────────────────────────────────────────────
        self.render_research_sector_factor_drift_ranks_windows(ctx);

        // ── Research Round 17 ──
        self.render_research_factor_ranking_extensions_windows(ctx);

        // Research Rounds 18-20 windows
        self.render_research_momentum_gap_atr_drawdown_windows(ctx);

        // Research Rounds 21-22 windows
        self.render_research_behavior_distribution_stats_windows(ctx);

        // ── Research Round 23 windows ──
        self.render_research_autocorrelation_hurst_volume_windows(ctx);

        // ── Research Round 24 windows ──
        self.render_research_gap_volatility_mean_reversion_windows(ctx);

        // ── Research Round 25 windows ──
        self.render_research_downside_efficiency_wick_volatility_windows(ctx);

        // ── Research Round 26 windows ──
        self.render_research_calmar_ulcer_liquidity_normality_windows(ctx);

        // ── Research Round 27 windows ──
        self.render_research_omega_fractal_burke_seasonality_windows(ctx);

        // ── Research Round 28 windows ──
        self.render_research_ohlc_volatility_cvar_calendar_windows(ctx);

        // ── Research Round 29 windows ──
        self.render_research_sterling_kelly_stat_tests_windows(ctx);

        // ── Research Round 30 windows ──
        self.render_research_sharpe_stationarity_jump_drawdown_windows(ctx);

        // ── Research Round 31 windows ──
        self.render_research_tail_arch_pain_structural_var_windows(ctx);

        // ── Research Round 32 windows ──
        self.render_research_entropy_tail_autocorrelation_windows(ctx);

        // ── Research Round 33 windows ──
        self.render_research_upside_leverage_drawdown_var_windows(ctx);

        // ── Research Round 34 windows ──
        self.render_research_entropy_stationarity_recovery_windows(ctx);

        // ── Research Round 35 windows ──
        self.render_research_robust_entropy_quantile_volatility_windows(ctx);

        // ── Research Round 36 windows ──
        self.render_research_normality_lmoments_price_impact_windows(ctx);

        // ── Research Round 37 windows ──
        self.render_research_fractal_tail_nonlinear_rank_windows(ctx);

        // ── Research Round 38 windows ──
        self.render_research_jump_unitroot_multifractal_tsi_windows(ctx);

        // ── Research Round 39 windows ──
        self.render_research_garch_bubble_dimension_information_windows(ctx);

        // ── Research Round 40 windows ──
        self.render_research_residual_iid_heteroskedastic_cycles_windows(ctx);

        // ── Research Round 41 windows ──
        self.render_research_portmanteau_ou_long_memory_spectrum_windows(ctx);

        // ── Research Round 42 windows ──
        self.render_research_squeeze_breakout_channels_windows(ctx);

        // ── Research Round 43 windows ──
        self.render_research_ichimoku_supertrend_channels_windows(ctx);

        // ── Research Round 44 windows ──
        self.render_research_directional_moneyflow_sar_windows(ctx);

        // ── Research Round 46 windows ──
        self.render_research_oscillator_price_momentum_windows(ctx);

        // ── Research Round 47 windows ──
        self.render_research_volume_momentum_oscillators_windows(ctx);

        // ── Research Round 48 windows ──
        self.render_research_volume_flow_trend_oscillators_windows(ctx);

        // ── Research Round 51 windows ──
        self.render_research_moving_average_regression_pivots_windows(ctx);

        // ── Research Round 52 windows ──
        self.render_research_zero_lag_elder_forecast_balance_windows(ctx);

        // ── Research Round 55: SMMA / ALLIGATOR / CRSI / SEB / IMI ──
        self.render_research_advanced_moving_averages_windows(ctx);

        // ── Research Round 60: WMA / RAINBOW / MESA_SINE / FRAMA / IBS windows ──
        self.render_research_ehlers_adaptive_ma_oscillators_windows(ctx);

        // ── Research Round 61: LAGUERRE_RSI / ZIGZAG / PGO / HT_TRENDLINE / MIDPOINT windows ──
        self.render_research_laguerre_pivot_midpoint_models_windows(ctx);

        // ── Research Round 62 windows ──
        self.render_research_massindex_atr_squeeze_force_windows(ctx);

        // ── Research Round 63 egui windows ──
        self.render_research_linearreg_hilbert_stochastic_windows(ctx);

        // ── Research Round 64 egui windows ──
        self.render_research_linearreg_hilbert_phase_windows(ctx);

        // ── Research Round 66 windows: AVGPRICE / MEDPRICE / TYPPRICE / WCLPRICE / VARIANCE ──
        self.render_research_ohlc_price_transforms_windows(ctx);

        // ── Research Round 67: PLUS_DI / MINUS_DI / PLUS_DM / MINUS_DM / DX ──
        self.render_research_directional_movement_family_windows(ctx);

        // ── Research Round 68 windows ──
        self.render_research_rate_of_change_correlation_windows(ctx);

        // ── Research Round 71 windows ──
        self.render_research_aroon_macd_variable_ma_windows(ctx);

        // ── Research Round 72 CDL* windows ─────────────────────────────────
        self.render_research_candlestick_core_patterns_windows(ctx);

        // ── Research Round 77 popup windows ──
        self.render_research_candlestick_marubozu_line_patterns_windows(ctx);

        // ── Research Round 78 popup windows ──
        self.render_research_candlestick_reversal_continuation_windows(ctx);

        // ── Research Round 76 (Quant Stats) popup windows ──
        self.render_research_quant_risk_nonlinearity_windows(ctx);

        // Research ingest and packet viewer
        self.render_research_ingest_windows(ctx);

        // GY — Treasury Yield Curve
        // Macro data windows
        self.render_macro_windows(ctx);
        // SEC, macro calendar, earnings, and congressional-trade windows
        timed_window!("sec_calendar", self.render_sec_calendar_windows(ctx));

        // Scrape status, fundamentals, EV scanner, and earnings windows
        self.render_scrape_status_windows(ctx);

        // Market analytics, calendars, screeners, and portfolio risk windows
        self.render_market_analytics_windows(ctx);

        // Order flow, bookmap, orderbook DOM, and indicator compiler windows
        self.render_trading_tools_windows(ctx);

        // Risk tools, alerts, outlier scanner, trade journal, and margin windows
        self.render_risk_journal_windows(ctx);

        // Cache stats and storage manager windows
        timed_window!("storage_sync", self.render_storage_sync_windows(ctx));

        // Object list, command reference, and data-window overlays
        self.render_workspace_reference_windows(ctx);

        // Alerts and market data dashboards
        self.render_alert_market_data_windows(ctx);
    }
}
