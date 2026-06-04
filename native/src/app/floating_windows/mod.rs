use super::*;
mod alert_market_data_windows;
mod bookmap;
mod broker_darwin_windows;
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
mod research_ingest;
mod research_round02;
mod research_round03;
mod research_round04;
mod research_round05;
mod research_round06;
mod research_round07;
mod research_round08;
mod research_round09;
mod research_round10;
mod research_round11;
mod research_round12;
mod research_round13_to15;
mod research_round16;
mod research_round17;
mod research_round18_to20;
mod research_round21_to22;
mod research_round23;
mod research_round24;
mod research_round25;
mod research_round26;
mod research_round27;
mod research_round28;
mod research_round29;
mod research_round30;
mod research_round31;
mod research_round32;
mod research_round33;
mod research_round34;
mod research_round35;
mod research_round36;
mod research_round37;
mod research_round38;
mod research_round39;
mod research_round40;
mod research_round41;
mod research_round42;
mod research_round43;
mod research_round44;
mod research_round46;
mod research_round47;
mod research_round48;
mod research_round51;
mod research_round52;
mod research_round55;
mod research_round60;
mod research_round61;
mod research_round62;
mod research_round63;
mod research_round64;
mod research_round66;
mod research_round67;
mod research_round68;
mod research_round71;
mod research_round72;
mod research_round76;
mod research_round77;
mod research_round78;
mod risk_journal_windows;
mod scope;
mod scrape_darwinia_windows;
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
        // Settings
        // Save credentials to keyring + SQLite fallback when Settings window closes
        if self.was_settings_open && !self.show_settings {
            let creds = [
                (keyring::keys::ALPACA_API_KEY, self.broker_api_key.as_str()),
                (keyring::keys::ALPACA_SECRET, self.broker_secret.as_str()),
                (keyring::keys::FINNHUB_KEY, self.finnhub_key.as_str()),
                (keyring::keys::FRED_KEY, self.fred_key.as_str()),
                (keyring::keys::TT_USERNAME, self.tt_username.as_str()),
                (keyring::keys::TT_PASSWORD, self.tt_password.as_str()),
                (
                    keyring::keys::LAN_SYNC_PASS,
                    self.lan_sync_passphrase.as_str(),
                ),
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
        // Broker, Kraken, and Darwinex windows
        self.render_broker_darwin_windows(ctx);

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

        self.render_risk_calc_window(ctx);
        self.render_compound_calc_window(ctx);

        self.render_backtest_window(ctx);

        // Screener — uses cached symbol data
        self.render_symbol_screener_window(ctx);

        // Symbols Explorer — all-encompassing symbol browser with broker hierarchy
        self.render_symbol_explorer_window(ctx);

        self.render_optimizer_window(ctx);

        // News
        self.render_news_window(ctx);

        // ── Godel parity research windows (ADR-107) ───────────────────────
        self.render_research_adr107_windows(ctx);

        // ── Research Godel Parity Round 2 windows ─────────────────────
        self.render_research_round02_windows(ctx);

        // ── Research Godel Parity Round 3 windows ─────────────────────
        self.render_research_round03_windows(ctx);

        // ── Research Round 4 windows ──────────────────────────────────
        self.render_research_round04_windows(ctx);

        // ── Research Round 5 windows ──────────────────────────────────
        self.render_research_round05_windows(ctx);

        // ── Research Round 6 windows ──────────────────────────────────
        self.render_research_round06_windows(ctx);

        // ── Research Godel Parity Round 7 ──
        self.render_research_round07_windows(ctx);

        // ── Research Round 8 windows ──
        self.render_research_round08_windows(ctx);

        // ── Research Round 9 windows ──
        self.render_research_round09_windows(ctx);

        // ── Research Godel Parity Round 10 ──
        self.render_research_round10_windows(ctx);

        // ── Research Godel Parity Round 11 windows ─────────────────────────────
        self.render_research_round11_windows(ctx);

        // Research Round 12 windows
        self.render_research_round12_windows(ctx);

        // Research Rounds 13-15 windows
        self.render_research_round13_to15_windows(ctx);

        // ── Research Round 16 ────────────────────────────────────────────────
        self.render_research_round16_windows(ctx);

        // ── Research Round 17 ──
        self.render_research_round17_windows(ctx);

        // Research Rounds 18-20 windows
        self.render_research_round18_to20_windows(ctx);

        // Research Rounds 21-22 windows
        self.render_research_round21_to22_windows(ctx);

        // ── Research Round 23 windows ──
        self.render_research_round23_windows(ctx);

        // ── Research Round 24 windows ──
        self.render_research_round24_windows(ctx);

        // ── Research Round 25 windows ──
        self.render_research_round25_windows(ctx);

        // ── Research Round 26 windows ──
        self.render_research_round26_windows(ctx);

        // ── Research Round 27 windows ──
        self.render_research_round27_windows(ctx);

        // ── Research Round 28 windows ──
        self.render_research_round28_windows(ctx);

        // ── Research Round 29 windows ──
        self.render_research_round29_windows(ctx);

        // ── Research Round 30 windows ──
        self.render_research_round30_windows(ctx);

        // ── Research Round 31 windows ──
        self.render_research_round31_windows(ctx);

        // ── Research Round 32 windows ──
        self.render_research_round32_windows(ctx);

        // ── Research Round 33 windows ──
        self.render_research_round33_windows(ctx);

        // ── Research Round 34 windows ──
        self.render_research_round34_windows(ctx);

        // ── Research Round 35 windows ──
        self.render_research_round35_windows(ctx);

        // ── Research Round 36 windows ──
        self.render_research_round36_windows(ctx);

        // ── Research Round 37 windows ──
        self.render_research_round37_windows(ctx);

        // ── Research Round 38 windows ──
        self.render_research_round38_windows(ctx);

        // ── Research Round 39 windows ──
        self.render_research_round39_windows(ctx);

        // ── Research Round 40 windows ──
        self.render_research_round40_windows(ctx);

        // ── Research Round 41 windows ──
        self.render_research_round41_windows(ctx);

        // ── Research Round 42 windows ──
        self.render_research_round42_windows(ctx);

        // ── Research Round 43 windows ──
        self.render_research_round43_windows(ctx);

        // ── Research Round 44 windows ──
        self.render_research_round44_windows(ctx);

        // ── Research Round 46 windows ──
        self.render_research_round46_windows(ctx);

        // ── Research Round 47 windows ──
        self.render_research_round47_windows(ctx);

        // ── Research Round 48 windows ──
        self.render_research_round48_windows(ctx);

        // ── Research Round 51 windows ──
        self.render_research_round51_windows(ctx);

        // ── Research Round 52 windows ──
        self.render_research_round52_windows(ctx);

        // ── Research Round 55: SMMA / ALLIGATOR / CRSI / SEB / IMI ──
        self.render_research_round55_windows(ctx);

        // ── Research Round 60: WMA / RAINBOW / MESA_SINE / FRAMA / IBS windows ──
        self.render_research_round60_windows(ctx);

        // ── Research Round 61: LAGUERRE_RSI / ZIGZAG / PGO / HT_TRENDLINE / MIDPOINT windows ──
        self.render_research_round61_windows(ctx);

        // ── Research Round 62 windows ──
        self.render_research_round62_windows(ctx);

        // ── Research Round 63 egui windows ──
        self.render_research_round63_windows(ctx);

        // ── Research Round 64 egui windows ──
        self.render_research_round64_windows(ctx);

        // ── Research Round 66 windows: AVGPRICE / MEDPRICE / TYPPRICE / WCLPRICE / VARIANCE ──
        self.render_research_round66_windows(ctx);

        // ── Research Round 67: PLUS_DI / MINUS_DI / PLUS_DM / MINUS_DM / DX ──
        self.render_research_round67_windows(ctx);

        // ── Research Round 68 windows ──
        self.render_research_round68_windows(ctx);

        // ── Research Round 71 windows ──
        self.render_research_round71_windows(ctx);

        // ── Research Round 72 CDL* windows ─────────────────────────────────
        self.render_research_round72_windows(ctx);

        // ── Research Round 77 popup windows ──
        self.render_research_round77_windows(ctx);

        // ── Research Round 78 popup windows ──
        self.render_research_round78_windows(ctx);

        // ── Research Round 76 (Quant Stats) popup windows ──
        self.render_research_round76_windows(ctx);

        // Research ingest and packet viewer
        self.render_research_ingest_windows(ctx);

        // GY — Treasury Yield Curve
        // Macro data windows
        self.render_macro_windows(ctx);
        // SEC, macro calendar, earnings, and congressional-trade windows
        self.render_sec_calendar_windows(ctx);

        // SwapHarvest, Darwinex Radar, scrape status, and earnings windows
        self.render_scrape_darwinia_windows(ctx);

        // Market analytics, calendars, screeners, and portfolio risk windows
        self.render_market_analytics_windows(ctx);

        // Order flow, bookmap, orderbook DOM, and indicator compiler windows
        self.render_trading_tools_windows(ctx);

        // Risk tools, alerts, outlier scanner, trade journal, and margin windows
        self.render_risk_journal_windows(ctx);

        // Cache stats, storage manager, and LAN sync windows
        self.render_storage_sync_windows(ctx);

        // Object list, command reference, and data-window overlays
        self.render_workspace_reference_windows(ctx);

        // Alerts, market data dashboards, and DarwinIA browser
        self.render_alert_market_data_windows(ctx);
    }
}
