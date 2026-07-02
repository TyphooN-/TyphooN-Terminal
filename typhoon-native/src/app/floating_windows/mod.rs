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
mod research;
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
        // TradeCopy (multi-account position copy + live order mirroring, ADR-130)
        self.render_tradecopy_window(ctx);
        // Broker connect + Kraken trade-history / open-orders windows
        self.render_broker_kraken_windows(ctx);
        // Alpaca position-close ticket (Sell to close a long / Buy to close a short)
        self.render_alpaca_close_dialog(ctx);

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

        // ── Research floating windows ──
        // Single dispatch boundary (ADR-125 Phase 1): every per-indicator /
        // per-fundamental research window renderer now lives under `research/`
        // and is invoked through one aggregator. Each inner renderer
        // early-returns on its own show_* flag; call order is preserved.
        self.render_research_ui_windows(ctx);

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
