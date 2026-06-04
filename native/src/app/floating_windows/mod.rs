use super::*;
mod bookmap;
mod broker_darwin_windows;
use bookmap::*;
mod news_filter;
use news_filter::*;
mod bardata;
mod macro_windows;
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
mod scope;
mod scrape_darwinia_windows;
mod screenshots;
mod sec_calendar_windows;
mod symbol_explorer;
mod symbol_screener;

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

        // Dividend Calendar
        if self.show_dividend_calendar {
            let div_active = if self.dividends_active_only {
                self.cached_active_symbols.clone()
            } else {
                Vec::new()
            };
            let mut dc_pending_action = SymbolAction::None;
            egui::Window::new("Dividend Calendar")
                .open(&mut self.show_dividend_calendar)
                .resizable(true)
                .default_size([500.0, 400.0])
                .max_size([500.0, 560.0])
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
                                        if !div_active.is_empty()
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
                .max_size([480.0, 560.0])
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

        // Option Chain — tastytrade option expirations from KV cache
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
                        if ui.button("Refresh").clicked() && !sym.is_empty() {
                            let _ = self.broker_tx.send(BrokerCmd::TastytradeOptionChain { symbol: sym.clone() });
                            self.log.push_back(LogEntry::info(format!("Refreshing option chain for {}...", sym)));
                        }
                        ui.label(egui::RichText::new("via tastytrade").color(oc_dim).small());
                    });
                    ui.separator();

                    // Load from KV cache
                    let chain_json = self.cache.as_ref()
                        .and_then(|c| c.get_kv(&format!("tt:options:{}", sym)).ok().flatten());

                    if let Some(json) = chain_json {
                        if let Ok(expirations) = serde_json::from_str::<serde_json::Value>(&json) {
                            if let Some(arr) = expirations.as_array() {
                                ui.label(egui::RichText::new(format!("{} expirations", arr.len())).small());
                                egui::ScrollArea::vertical().auto_shrink(false).max_height(360.0).show(ui, |ui| {
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

        // Symbol Overlap — reads from bg cache
        if self.show_symbol_overlap {
            egui::Window::new("Symbol Overlap")
                .open(&mut self.show_symbol_overlap)
                .resizable(true)
                .default_size([600.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("Cross-DARWIN Symbol Overlap");
                    ui.separator();
                    let overlaps = &self.bg.symbol_overlaps;
                    if overlaps.is_empty() {
                        ui.label("No overlapping symbols across DARWINs.");
                    } else {
                        ui.label(
                            egui::RichText::new(format!("{} overlapping symbols", overlaps.len()))
                                .strong(),
                        );
                        egui::ScrollArea::vertical()
                            .auto_shrink(false)
                            .max_height(300.0)
                            .show(ui, |ui| {
                                egui::Grid::new("overlap_grid")
                                    .striped(true)
                                    .num_columns(6)
                                    .show(ui, |ui| {
                                        ui.strong("Symbol");
                                        ui.strong("Side");
                                        ui.strong("Volume");
                                        ui.strong("Notional");
                                        ui.strong("Risk");
                                        ui.strong("DARWINs");
                                        ui.end_row();
                                        for o in overlaps.iter() {
                                            ui.label(&o.symbol);
                                            let side_c = if o.side == "buy" { UP } else { DOWN };
                                            ui.label(egui::RichText::new(&o.side).color(side_c));
                                            ui.label(format!("{:.2}", o.total_volume));
                                            ui.label(format!("${:.0}", o.total_notional));
                                            let risk_c = match o.correlation_risk.as_str() {
                                                "HIGH" => DOWN,
                                                "MEDIUM" => egui::Color32::from_rgb(255, 200, 50),
                                                _ => UP,
                                            };
                                            ui.label(
                                                egui::RichText::new(&o.correlation_risk)
                                                    .color(risk_c),
                                            );
                                            ui.label(o.darwins.join(", "));
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                });
        }

        // Correlation Matrix — reads from bg cache
        if self.show_correlation {
            egui::Window::new("Correlation Matrix")
                .open(&mut self.show_correlation)
                .resizable(true)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("DARWIN Correlation Matrix");
                    ui.separator();
                    let corrs = &self.bg.correlations;
                    if corrs.is_empty() {
                        ui.label(
                            egui::RichText::new("Need 2+ DARWINs imported for correlation.")
                                .color(AXIS_TEXT),
                        );
                    } else {
                        let high_corr: Vec<_> =
                            corrs.iter().filter(|c| c.correlation.abs() > 0.7).collect();
                        if !high_corr.is_empty() {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} high-correlation pairs (>0.7)",
                                    high_corr.len()
                                ))
                                .color(egui::Color32::from_rgb(255, 200, 50)),
                            );
                        }
                        egui::Grid::new("corr_matrix")
                            .striped(true)
                            .num_columns(3)
                            .show(ui, |ui| {
                                ui.strong("DARWIN A");
                                ui.strong("DARWIN B");
                                ui.strong("Correlation");
                                ui.end_row();
                                for c in corrs.iter() {
                                    ui.label(&c.darwin_a);
                                    ui.label(&c.darwin_b);
                                    let color = if c.correlation.abs() > 0.95 {
                                        egui::Color32::from_rgb(255, 40, 40)
                                    } else if c.correlation.abs() > 0.7 {
                                        egui::Color32::from_rgb(255, 200, 50)
                                    } else {
                                        UP
                                    };
                                    ui.label(
                                        egui::RichText::new(format!("{:.4}", c.correlation))
                                            .color(color),
                                    );
                                    ui.end_row();
                                }
                            });
                        ui.add_space(5.0);
                        ui.label(
                            egui::RichText::new("Darwinex limit: 0.95 correlation / 45d")
                                .color(AXIS_TEXT),
                        );
                        // Visual heatmap
                        if !corrs.is_empty() {
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new("Heatmap").strong());
                            let cell = 28.0_f32;
                            let n = corrs.len();
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(cell * n as f32, cell),
                                egui::Sense::hover(),
                            );
                            let painter = ui.painter_at(rect);
                            for (i, c) in corrs.iter().enumerate() {
                                let abs_c = c.correlation.abs();
                                let color = if abs_c > 0.95 {
                                    egui::Color32::from_rgb(255, 40, 40)
                                } else if abs_c > 0.7 {
                                    egui::Color32::from_rgb(255, 165, 0)
                                } else if abs_c > 0.5 {
                                    egui::Color32::from_rgb(255, 220, 50)
                                } else {
                                    egui::Color32::from_rgb(46, 204, 113)
                                };
                                let r = egui::Rect::from_min_size(
                                    rect.min + egui::vec2(i as f32 * cell, 0.0),
                                    egui::vec2(cell - 2.0, cell - 2.0),
                                );
                                painter.rect_filled(r, 3.0, color);
                                painter.text(
                                    r.center(),
                                    egui::Align2::CENTER_CENTER,
                                    format!("{:.2}", c.correlation),
                                    egui::FontId::proportional(9.0),
                                    egui::Color32::BLACK,
                                );
                            }
                        }
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

        // Monte Carlo — reads from bg cache
        if self.show_montecarlo {
            egui::Window::new("Monte Carlo VaR")
                .open(&mut self.show_montecarlo)
                .resizable(true)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("Monte Carlo Simulation");
                    ui.separator();
                    if let Some(ref var_stats) = self.bg.var_stats {
                        egui::Grid::new("mc_grid")
                            .striped(true)
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.label("Trading Days:");
                                ui.label(format!("{}", var_stats.trading_days));
                                ui.end_row();
                                ui.label("VaR 95% (daily):");
                                ui.label(format!("${:.2}", var_stats.var_95));
                                ui.end_row();
                                ui.label("VaR 99% (daily):");
                                ui.label(format!("${:.2}", var_stats.var_99));
                                ui.end_row();
                                ui.label("CVaR 95%:");
                                ui.label(format!("${:.2}", var_stats.cvar_95));
                                ui.end_row();
                                ui.label("CVaR 99%:");
                                ui.label(format!("${:.2}", var_stats.cvar_99));
                                ui.end_row();
                                ui.label("Daily Volatility:");
                                ui.label(format!("{:.4}", var_stats.daily_vol));
                                ui.end_row();
                                ui.label("Annualized Vol:");
                                ui.label(format!("{:.4}", var_stats.annualized_vol));
                                ui.end_row();
                                ui.label("Sharpe Ratio:");
                                ui.label(format!("{:.3}", var_stats.sharpe));
                                ui.end_row();
                                ui.label("Sortino Ratio:");
                                ui.label(format!("{:.3}", var_stats.sortino));
                                ui.end_row();
                                ui.label("Calmar Ratio:");
                                ui.label(format!("{:.3}", var_stats.calmar));
                                ui.end_row();
                                ui.label("Max Drawdown:");
                                ui.label(format!("{:.2}%", var_stats.max_drawdown_pct));
                                ui.end_row();
                                ui.label("Avg Daily P&L:");
                                ui.label(format!("${:.2}", var_stats.avg_daily_pnl));
                                ui.end_row();
                                ui.label("Best Day:");
                                ui.label(
                                    egui::RichText::new(format!("${:.2}", var_stats.best_day))
                                        .color(UP),
                                );
                                ui.end_row();
                                ui.label("Worst Day:");
                                ui.label(
                                    egui::RichText::new(format!("${:.2}", var_stats.worst_day))
                                        .color(DOWN),
                                );
                                ui.end_row();
                            });
                        // VaR levels bar chart
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("VaR / CVaR Levels").strong());
                        let bars = vec![
                            PlotBar::new(0.0, var_stats.var_95.abs()).name("VaR 95%"),
                            PlotBar::new(1.0, var_stats.var_99.abs()).name("VaR 99%"),
                            PlotBar::new(2.0, var_stats.cvar_95.abs()).name("CVaR 95%"),
                            PlotBar::new(3.0, var_stats.cvar_99.abs()).name("CVaR 99%"),
                        ];
                        let chart = BarChart::new("VaR Levels", bars)
                            .width(0.6)
                            .color(egui::Color32::from_rgb(255, 100, 80));
                        Plot::new("mc_var_bars")
                            .height(120.0)
                            .allow_drag(false)
                            .allow_zoom(false)
                            .show(ui, |plot_ui| {
                                plot_ui.bar_chart(chart);
                            });
                    } else {
                        ui.label(
                            egui::RichText::new(
                                "Need 30+ daily returns for Monte Carlo. Import DARWIN data.",
                            )
                            .color(AXIS_TEXT),
                        );
                    }
                });
        }

        // Stress Test — apply drawdown scenarios to portfolio
        if self.show_stress_test {
            egui::Window::new("Stress Test")
                .open(&mut self.show_stress_test)
                .resizable(true)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("Portfolio Stress Test");
                    ui.separator();
                    if let Some(ref portfolio) = self.bg.portfolio {
                        if !portfolio.accounts.is_empty() {
                            let equity = portfolio.total_final_balance;
                            ui.label(format!("Current portfolio equity: ${:.2}", equity));
                            ui.add_space(10.0);
                            let scenarios = [
                                ("2008 GFC", -56.8),
                                ("COVID Mar 2020", -33.9),
                                ("2022 Bear Market", -25.4),
                                ("Flash Crash 2010", -9.0),
                                ("Brexit Vote 2016", -5.3),
                                ("10% Correction", -10.0),
                                ("20% Bear", -20.0),
                                ("50% Crash", -50.0),
                            ];
                            egui::Grid::new("stress_grid")
                                .striped(true)
                                .num_columns(3)
                                .show(ui, |ui| {
                                    ui.strong("Scenario");
                                    ui.strong("Drawdown");
                                    ui.strong("Equity After");
                                    ui.end_row();
                                    for (name, dd_pct) in &scenarios {
                                        let after = equity * (1.0 + dd_pct / 100.0);
                                        let loss = equity - after;
                                        ui.label(*name);
                                        ui.label(
                                            egui::RichText::new(format!("{:.1}%", dd_pct))
                                                .color(DOWN),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "${:.2} (−${:.2})",
                                                after, loss
                                            ))
                                            .color(DOWN),
                                        );
                                        ui.end_row();
                                    }
                                });
                            ui.add_space(5.0);
                            ui.label(format!(
                                "Max historical DD: {:.2}%",
                                portfolio.combined_max_drawdown_pct
                            ));

                            // Horizontal bar chart of portfolio impact
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new("Portfolio Impact").strong());
                            let impact_scenarios: Vec<(&str, f64)> = scenarios
                                .iter()
                                .take(6)
                                .map(|&(name, dd_pct)| {
                                    let loss = equity * dd_pct / 100.0;
                                    (name, loss)
                                })
                                .collect();
                            let bars: Vec<PlotBar> = impact_scenarios
                                .iter()
                                .enumerate()
                                .map(|(i, &(name, loss))| {
                                    PlotBar::new(i as f64, loss)
                                        .width(0.7)
                                        .fill(DOWN)
                                        .name(name)
                                })
                                .collect();
                            let chart = BarChart::new("Impact", bars);
                            Plot::new("stress_impact_bars")
                                .height(140.0)
                                .allow_drag(false)
                                .allow_zoom(false)
                                .show(ui, |plot_ui| {
                                    plot_ui.bar_chart(chart);
                                });
                        } else {
                            ui.label(
                                egui::RichText::new("Import DARWIN data for stress testing.")
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
            // PERF: read from per-frame cache
            let scoped = self.cached_scoped_fundamentals.clone();
            egui::Window::new("Sector Heatmap")
                .open(&mut self.show_sector_heatmap)
                .resizable(true)
                .default_size([500.0, 400.0])
                .max_size([500.0, 560.0])
                .show(ctx, |ui| {
                    let sectors = typhoon_engine::core::screener::compute_sector_heatmap(&scoped);
                    ui.label(
                        egui::RichText::new(format!(
                            "{} sectors • scope: {} ({} symbols)",
                            sectors.len(),
                            scope_label,
                            scoped.len()
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
                                    for s in &sectors {
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
            // PERF: read from per-frame cache
            let scoped = self.cached_scoped_fundamentals.clone();
            let mut div_pending_action = SymbolAction::None;
            // UX7: Pre-fetch sparklines for dividend stocks
            let divs_for_sl = typhoon_engine::core::screener::screen_dividend_stocks(&scoped);
            let mut div_sparklines: std::collections::HashMap<String, std::sync::Arc<Vec<f64>>> =
                std::collections::HashMap::new();
            for d in divs_for_sl.iter().take(100) {
                let closes = self.get_sparkline(&d.symbol);
                if !closes.is_empty() {
                    div_sparklines.insert(d.symbol.to_uppercase(), closes);
                }
            }
            egui::Window::new("Dividend Yield Screener")
                .open(&mut self.show_dividends)
                .resizable(true)
                .default_size([700.0, 400.0])
                .max_size([700.0, 560.0])
                .show(ctx, |ui| {
                    let divs = typhoon_engine::core::screener::screen_dividend_stocks(&scoped);
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
                            EventSource::Darwinex,
                            "Darwinex",
                        );
                        ui.radio_value(&mut self.event_filter_source, EventSource::Tasty, "Tasty");
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
                                EventSource::All => {
                                    r.in_alpaca || r.in_darwinex || r.in_tasty || r.in_kraken
                                }
                                EventSource::Alpaca => r.in_alpaca,
                                EventSource::Darwinex => r.in_darwinex,
                                EventSource::Tasty => r.in_tasty,
                                EventSource::Kraken => r.in_kraken,
                                EventSource::Positions => {
                                    r.in_alpaca || r.in_darwinex || r.in_tasty || r.in_kraken
                                }
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
                                        if r.in_darwinex {
                                            tags.push("D");
                                        }
                                        if r.in_tasty {
                                            tags.push("T");
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
.max_size([600.0, 560.0])
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

        // ── Risk Budget ───────────────────────────────────────────────
        if self.show_risk_budget {
            egui::Window::new("Risk Budget")
                .open(&mut self.show_risk_budget)
                .resizable(true)
                .default_size([500.0, 350.0])
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new("Portfolio Risk Contribution (VaR Decomposition)")
                            .strong(),
                    );
                    ui.separator();
                    // Build from DARWIN data
                    if self.bg.var_stats.is_some() {
                        let names: Vec<String> = self
                            .bg
                            .account_details
                            .iter()
                            .map(|d| d.ticker.clone())
                            .collect();
                        let n = names.len();
                        if n > 0 {
                            let weights: Vec<f64> = vec![1.0 / n as f64; n]; // equal weight
                            let individual_vars: Vec<f64> = self
                                .bg
                                .account_details
                                .iter()
                                .map(|d| d.var_stats.as_ref().map(|v| v.var_95).unwrap_or(0.0))
                                .collect();
                            // Build correlation matrix from bg data
                            let mut corr = vec![vec![0.0; n]; n];
                            for i in 0..n {
                                corr[i][i] = 1.0;
                            }
                            let name_idx: std::collections::HashMap<&str, usize> = names
                                .iter()
                                .enumerate()
                                .map(|(i, n)| (n.as_str(), i))
                                .collect();
                            for c in &self.bg.correlations {
                                if let (Some(&i), Some(&j)) = (
                                    name_idx.get(c.darwin_a.as_str()),
                                    name_idx.get(c.darwin_b.as_str()),
                                ) {
                                    corr[i][j] = c.correlation;
                                    corr[j][i] = c.correlation;
                                }
                            }
                            let budget = typhoon_engine::core::screener::compute_risk_budget(
                                &names,
                                &weights,
                                &individual_vars,
                                &corr,
                            );
                            egui::Grid::new("risk_budget_grid")
                                .striped(true)
                                .num_columns(5)
                                .show(ui, |ui| {
                                    ui.strong("DARWIN");
                                    ui.strong("Weight%");
                                    ui.strong("VaR95");
                                    ui.strong("Risk%");
                                    ui.strong("Marginal VaR");
                                    ui.end_row();
                                    for b in &budget {
                                        ui.label(egui::RichText::new(&b.name).strong());
                                        ui.label(format!("{:.1}%", b.weight_pct));
                                        ui.label(format!("${:.0}", b.var_95));
                                        let rc = if b.risk_contribution_pct > 20.0 {
                                            DOWN
                                        } else {
                                            AXIS_TEXT
                                        };
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:.1}%",
                                                b.risk_contribution_pct
                                            ))
                                            .color(rc),
                                        );
                                        ui.label(format!("{:.2}", b.marginal_var));
                                        ui.end_row();
                                    }
                                });
                        } else {
                            ui.label("No DARWIN accounts loaded.");
                        }
                    } else {
                        ui.label("VaR data not computed yet.");
                    }
                });
        }

        // Order Flow
        if self.show_order_flow {
            egui::Window::new("Order Flow")
                .open(&mut self.show_order_flow)
                .resizable(true)
                .default_size([500.0, 450.0])
                .show(ctx, |ui| {
                    let of_green = egui::Color32::from_rgb(0, 200, 80);
                    let of_red = egui::Color32::from_rgb(220, 50, 50);
                    let of_dim = egui::Color32::from_rgb(80, 80, 100);

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
                        ui.label(egui::RichText::new(format!("Order Flow: {}", sym)).strong());
                        if ui.button("Fetch L2").clicked() && !sym.is_empty() {
                            let _ = self.broker_tx.send(BrokerCmd::GetOrderbook {
                                symbol: sym.clone(),
                            });
                        }
                        let stream_supported =
                            kraken_bookmap_stream_supported(&sym, &self.kraken_pairs);
                        let stream_button =
                            ui.add_enabled(stream_supported, egui::Button::new("Stream L2"));
                        if stream_button.clicked() && !sym.is_empty() {
                            let _ = self.broker_tx.send(BrokerCmd::KrakenStartOrderbookWs {
                                symbol: sym.clone(),
                                depth: 100,
                            });
                        }
                        if !stream_supported && !sym.is_empty() {
                            stream_button.on_hover_text(
                                "Live Kraken depth is only available for Kraken spot pairs.",
                            );
                        }
                    });
                    ui.separator();

                    if let Some(chart) = self.charts.get(self.active_tab) {
                        let bars = &chart.bars;
                        let n = bars.len();
                        if n > 10 {
                            let recent = &bars[n.saturating_sub(60)..];

                            // Cumulative Delta (buying vs selling pressure proxy)
                            ui.label(
                                egui::RichText::new("Cumulative Delta (volume × direction)")
                                    .small()
                                    .strong(),
                            );
                            let mut cum_delta = Vec::with_capacity(recent.len());
                            let mut running = 0.0_f64;
                            for b in recent {
                                let delta = if b.close >= b.open {
                                    b.volume
                                } else {
                                    -b.volume
                                };
                                running += delta;
                                cum_delta.push(running);
                            }
                            {
                                let pts: PlotPoints = PlotPoints::new(
                                    cum_delta
                                        .iter()
                                        .enumerate()
                                        .map(|(i, &d)| [i as f64, d])
                                        .collect(),
                                );
                                let c = if *cum_delta.last().unwrap_or(&0.0) >= 0.0 {
                                    of_green
                                } else {
                                    of_red
                                };
                                let line = Line::new("Cum Delta", pts).color(c).width(1.5);
                                Plot::new("cum_delta_plot")
                                    .height(100.0)
                                    .allow_drag(false)
                                    .allow_zoom(false)
                                    .allow_scroll(false)
                                    .show_axes([false, true])
                                    .show(ui, |plot_ui| {
                                        plot_ui.line(line);
                                    });
                            }

                            // Per-bar Delta bars
                            ui.label(egui::RichText::new("Per-Bar Delta").small().strong());
                            {
                                let bars_plot: Vec<PlotBar> = recent
                                    .iter()
                                    .enumerate()
                                    .map(|(i, b)| {
                                        let delta = if b.close >= b.open {
                                            b.volume
                                        } else {
                                            -b.volume
                                        };
                                        let c = if delta >= 0.0 { of_green } else { of_red };
                                        PlotBar::new(i as f64, delta).width(0.8).fill(c)
                                    })
                                    .collect();
                                let chart = BarChart::new("Delta", bars_plot);
                                Plot::new("delta_bars")
                                    .height(80.0)
                                    .allow_drag(false)
                                    .allow_zoom(false)
                                    .allow_scroll(false)
                                    .show_axes([false, true])
                                    .show(ui, |plot_ui| {
                                        plot_ui.bar_chart(chart);
                                    });
                            }

                            // Footprint-style summary (price levels with buy/sell volume)
                            ui.label(
                                egui::RichText::new("Footprint Summary (last 20 bars)")
                                    .small()
                                    .strong(),
                            );
                            let last20 = &recent[recent.len().saturating_sub(20)..];
                            let min_p = last20.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                            let max_p = last20.iter().map(|b| b.high).fold(f64::MIN, f64::max);
                            let range = max_p - min_p;
                            if range > 0.0 {
                                let levels = 15_usize;
                                let step = range / levels as f64;
                                let mut buy_vol = vec![0.0_f64; levels];
                                let mut sell_vol = vec![0.0_f64; levels];
                                for b in last20 {
                                    let mid_level =
                                        ((((b.high + b.low) / 2.0) - min_p) / step) as usize;
                                    let idx = mid_level.min(levels - 1);
                                    if b.close >= b.open {
                                        buy_vol[idx] += b.volume;
                                    } else {
                                        sell_vol[idx] += b.volume;
                                    }
                                }

                                let max_vol = buy_vol
                                    .iter()
                                    .chain(sell_vol.iter())
                                    .cloned()
                                    .fold(0.0_f64, f64::max);
                                let avail_w = ui.available_width();
                                for i in (0..levels).rev() {
                                    let price = min_p + (i as f64 + 0.5) * step;
                                    let bv = buy_vol[i];
                                    let sv = sell_vol[i];
                                    let b_frac = if max_vol > 0.0 {
                                        (bv / max_vol) as f32
                                    } else {
                                        0.0
                                    };
                                    let s_frac = if max_vol > 0.0 {
                                        (sv / max_vol) as f32
                                    } else {
                                        0.0
                                    };

                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(format_price(price))
                                                .monospace()
                                                .small()
                                                .color(of_dim),
                                        );
                                        let (rect, _) = ui.allocate_exact_size(
                                            egui::vec2(avail_w - 80.0, 12.0),
                                            egui::Sense::hover(),
                                        );
                                        let painter = ui.painter_at(rect);
                                        let mid_x = rect.left() + rect.width() / 2.0;
                                        // Buy bar (extends right from center)
                                        painter.rect_filled(
                                            egui::Rect::from_min_size(
                                                egui::pos2(mid_x, rect.top()),
                                                egui::vec2(b_frac * rect.width() / 2.0, 12.0),
                                            ),
                                            0.0,
                                            of_green,
                                        );
                                        // Sell bar (extends left from center)
                                        painter.rect_filled(
                                            egui::Rect::from_min_size(
                                                egui::pos2(
                                                    mid_x - s_frac * rect.width() / 2.0,
                                                    rect.top(),
                                                ),
                                                egui::vec2(s_frac * rect.width() / 2.0, 12.0),
                                            ),
                                            0.0,
                                            of_red,
                                        );
                                    });
                                }
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Sells ←").color(of_red).small());
                                    ui.label(egui::RichText::new("→ Buys").color(of_green).small());
                                });
                            }
                        } else {
                            ui.label(egui::RichText::new("Load chart data first.").color(of_dim));
                        }
                    }
                });
        }

        // Bookmap — one floating heatmap per requested symbol.
        if self.show_bookmap {
            self.open_bookmap_window(None);
            self.show_bookmap = false;
        }
        let mut open_bookmaps = Vec::with_capacity(self.bookmap_windows.len());
        for window in std::mem::take(&mut self.bookmap_windows) {
            let sym = window.symbol;
            let mut open = window.open;
            let title = format!("Bookmap Heatmap — {sym}");
            egui::Window::new(title)
                .id(egui::Id::new(("bookmap_heatmap", sym.as_str())))
                .open(&mut open)
                .resizable(true)
                .default_size([600.0, 450.0])
                .show(ctx, |ui| {
                    let bm_green = egui::Color32::from_rgb(0, 180, 80);
                    let bm_red = egui::Color32::from_rgb(200, 50, 50);
                    let bm_dim = egui::Color32::from_rgb(80, 80, 100);

                    let stream_supported = kraken_bookmap_stream_supported(&sym, &self.kraken_pairs);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("Depth: {sym}")).strong());
                        if ui.button("Fetch Depth").clicked() && !sym.is_empty() {
                            let _ = self.broker_tx.send(BrokerCmd::GetOrderbook {
                                symbol: sym.clone(),
                            });
                        }
                        let stream_button = ui.add_enabled(
                            stream_supported,
                            egui::Button::new("Stream Depth"),
                        );
                        if stream_button.clicked() && !sym.is_empty() {
                            let _ = self.broker_tx.send(BrokerCmd::KrakenStartOrderbookWs {
                                symbol: sym.clone(),
                                depth: 100,
                            });
                        }
                        if !stream_supported && !sym.is_empty() {
                            stream_button.on_hover_text("Live Kraken depth is only available for Kraken spot pairs, not equity symbols.");
                        }
                        ui.label(egui::RichText::new("L2 depth").color(bm_dim).small());
                    });
                    ui.separator();

                    if orderbook_json_matches_symbol(&self.orderbook_result, &sym)
                        && render_live_orderbook_heatmap(
                            ui,
                            &self.orderbook_result,
                            bm_green,
                            bm_red,
                            bm_dim,
                        )
                    {
                        ui.separator();
                    }

                    // Render depth heatmap from the requested symbol's chart data.
                    let chart = self.charts.iter().find(|chart| {
                        normalize_market_data_symbol(&chart.symbol).eq_ignore_ascii_case(&sym)
                    });
                    if let Some(chart) = chart {
                        let bars = &chart.bars;
                        let n = bars.len();
                        if n > 20 {
                            // Build a price × time volume heatmap from recent bars
                            let recent = &bars[n.saturating_sub(100)..];
                            let min_p = recent.iter().map(|b| b.low).fold(f64::MAX, f64::min);
                            let max_p = recent.iter().map(|b| b.high).fold(f64::MIN, f64::max);
                            let price_range = max_p - min_p;
                            if price_range > 0.0 {
                                let rows = 40_usize; // price levels
                                let cols = recent.len();

                                // Allocate and paint heatmap
                                let avail = ui.available_size();
                                let w = avail.x.min(580.0);
                                let h = 300.0_f32;
                                let (rect, _) =
                                    ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
                                let painter = ui.painter_at(rect);
                                painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(5, 5, 15));

                                let cell_w = w / cols as f32;
                                let cell_h = h / rows as f32;

                                for (col, bar) in recent.iter().enumerate() {
                                    let x = rect.left() + col as f32 * cell_w;
                                    // Map bar's high-low range to row indices
                                    let row_lo =
                                        ((bar.low - min_p) / price_range * rows as f64) as usize;
                                    let row_hi =
                                        ((bar.high - min_p) / price_range * rows as f64) as usize;
                                    let vol_norm =
                                        (bar.volume.ln().max(0.0) / 15.0).min(1.0) as f32;

                                    for row in row_lo..=row_hi.min(rows - 1) {
                                        let y = rect.bottom() - (row as f32 + 1.0) * cell_h;
                                        let intensity = vol_norm * 0.8;
                                        let color = if bar.close >= bar.open {
                                            egui::Color32::from_rgba_premultiplied(
                                                0,
                                                (intensity * 200.0) as u8,
                                                (intensity * 80.0) as u8,
                                                (intensity * 255.0) as u8,
                                            )
                                        } else {
                                            egui::Color32::from_rgba_premultiplied(
                                                (intensity * 200.0) as u8,
                                                (intensity * 50.0) as u8,
                                                0,
                                                (intensity * 255.0) as u8,
                                            )
                                        };
                                        painter.rect_filled(
                                            egui::Rect::from_min_size(
                                                egui::pos2(x, y),
                                                egui::vec2(cell_w, cell_h),
                                            ),
                                            0.0,
                                            color,
                                        );
                                    }
                                }

                                // Price axis labels
                                for i in 0..=4 {
                                    let frac = i as f64 / 4.0;
                                    let price = min_p + frac * price_range;
                                    let y = rect.bottom() - frac as f32 * h;
                                    painter.text(
                                        egui::pos2(rect.right() - 2.0, y),
                                        egui::Align2::RIGHT_CENTER,
                                        format_price(price),
                                        egui::FontId::monospace(9.0),
                                        bm_dim,
                                    );
                                }

                                // Legend
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new("Bid Volume").color(bm_green).small(),
                                    );
                                    ui.label(
                                        egui::RichText::new("Ask Volume").color(bm_red).small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} bars × {} levels",
                                            cols, rows
                                        ))
                                        .color(bm_dim)
                                        .small(),
                                    );
                                });
                            }
                        } else {
                            ui.label(egui::RichText::new("Load chart data first.").color(bm_dim));
                        }
                    } else {
                        ui.label(
                            egui::RichText::new(format!(
                                "No open chart data for {sym}. Open/load the symbol chart first."
                            ))
                            .color(bm_dim),
                        );
                    }
                });
            if open {
                open_bookmaps.push(BookmapWindowState { symbol: sym, open });
            }
        }
        self.bookmap_windows = open_bookmaps;

        // Orderbook DOM — shows real L2 data from Fetch Depth/Fetch L2
        if self.show_orderbook_window {
            egui::Window::new("Orderbook DOM")
                .open(&mut self.show_orderbook_window)
                .resizable(true).default_size([360.0, 420.0])
                .show(ctx, |ui| {
                    let ob_bid = egui::Color32::from_rgb(0, 200, 80);
                    let ob_ask = egui::Color32::from_rgb(220, 50, 50);
                    let ob_dim = egui::Color32::from_rgb(80, 80, 100);
                    if self.orderbook_result.is_empty() {
                        ui.label(egui::RichText::new("No L2 data — click Fetch Depth in Bookmap or Fetch L2 in Order Flow.").color(ob_dim).small());
                    } else if let Ok(v) = serde_json::from_str::<serde_json::Value>(&self.orderbook_result) {
                        let sym = v["symbol"].as_str().unwrap_or("?");
                        let ts  = v["timestamp"].as_str().unwrap_or("");
                        ui.label(egui::RichText::new(format!("{} — {}", sym, ts)).strong().small());
                        ui.separator();
                        let bids = v["bids"].as_array().map(|a| a.as_slice()).unwrap_or(&[]);
                        let asks = v["asks"].as_array().map(|a| a.as_slice()).unwrap_or(&[]);
                        // max size for bar scaling
                        let max_sz = bids.iter().chain(asks.iter())
                            .filter_map(|e| e["size"].as_f64())
                            .fold(0.0_f64, f64::max).max(1.0);
                        let avail_w = ui.available_width().min(320.0);
                        egui::ScrollArea::vertical().auto_shrink(false).max_height(340.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("Asks (sell side)").color(ob_ask).small().strong());
                            for ask in asks.iter().rev().take(15) {
                                let price = ask["price"].as_f64().unwrap_or(0.0);
                                let size  = ask["size"].as_f64().unwrap_or(0.0);
                                let frac  = (size / max_sz) as f32;
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(format_price(price)).monospace().small().color(ob_ask));
                                    let (rect, _) = ui.allocate_exact_size(egui::vec2(avail_w - 90.0, 10.0), egui::Sense::hover());
                                    ui.painter_at(rect).rect_filled(
                                        egui::Rect::from_min_size(rect.min, egui::vec2(frac * rect.width(), 10.0)),
                                        0.0, egui::Color32::from_rgba_premultiplied(200, 40, 40, 120));
                                    ui.label(egui::RichText::new(format!("{:.4}", size)).monospace().small().color(ob_dim));
                                });
                            }
                            ui.separator();
                            ui.label(egui::RichText::new("Bids (buy side)").color(ob_bid).small().strong());
                            for bid in bids.iter().take(15) {
                                let price = bid["price"].as_f64().unwrap_or(0.0);
                                let size  = bid["size"].as_f64().unwrap_or(0.0);
                                let frac  = (size / max_sz) as f32;
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(format_price(price)).monospace().small().color(ob_bid));
                                    let (rect, _) = ui.allocate_exact_size(egui::vec2(avail_w - 90.0, 10.0), egui::Sense::hover());
                                    ui.painter_at(rect).rect_filled(
                                        egui::Rect::from_min_size(rect.min, egui::vec2(frac * rect.width(), 10.0)),
                                        0.0, egui::Color32::from_rgba_premultiplied(0, 180, 60, 120));
                                    ui.label(egui::RichText::new(format!("{:.4}", size)).monospace().small().color(ob_dim));
                                });
                            }
                        });
                    } else {
                        ui.label(egui::RichText::new("Failed to parse orderbook data.").color(ob_ask).small());
                    }
                });
        }

        // MQL5/PineScript Indicator Compiler
        if self.show_indicator_compiler {
            egui::Window::new("Indicator Compiler")
                .open(&mut self.show_indicator_compiler)
                .resizable(true)
                .default_size([650.0, 550.0])
                .max_size([650.0, 560.0])
                .show(ctx, |ui| {
                    let cc_green = egui::Color32::from_rgb(46, 204, 113);
                    let cc_red = egui::Color32::from_rgb(231, 76, 60);
                    let cc_dim = egui::Color32::from_rgb(100, 100, 120);
                    // Language table — kept adjacent to the match arms below so
                    // they stay in sync if we add another frontend.
                    const LANG_LABELS: &[&str] = &[
                        "MQL5",
                        "MQL4",
                        "PineScript",
                        "EasyLanguage",
                        "thinkScript",
                        "AFL (AmiBroker)",
                        "ProBuilder",
                        "NinjaScript",
                        "cAlgo (cTrader)",
                        "ACSIL (Sierra Chart)",
                    ];
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Language:").small());
                        egui::ComboBox::from_id_salt("compiler_lang")
                            .selected_text(
                                LANG_LABELS
                                    .get(self.compiler_language)
                                    .copied()
                                    .unwrap_or("MQL5"),
                            )
                            .width(180.0)
                            .show_ui(ui, |ui| {
                                for (i, label) in LANG_LABELS.iter().enumerate() {
                                    ui.selectable_value(&mut self.compiler_language, i, *label);
                                }
                            });
                        if ui.button("Load File...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter(
                                    "Indicator",
                                    &[
                                        "mq5", "mqh", // MQL5
                                        "mq4", "mqh",  // MQL4
                                        "pine", // PineScript
                                        "el", "els", // EasyLanguage
                                        "ts", "tos", // thinkScript
                                        "afl", // AFL
                                        "itf", // ProBuilder
                                        "cs",  // NinjaScript + cAlgo (C#)
                                        "cpp", "h", // ACSIL (Sierra Chart)
                                        "txt",
                                    ],
                                )
                                .pick_file()
                            {
                                if let Ok(contents) = std::fs::read_to_string(&path) {
                                    self.compiler_source = contents;
                                    // Auto-detect language by extension / content
                                    self.compiler_language = match path
                                        .extension()
                                        .and_then(|e| e.to_str())
                                    {
                                        Some("mq4") => 1,
                                        Some("pine") => 2,
                                        Some("el") | Some("els") => 3,
                                        Some("ts") | Some("tos") => 4,
                                        Some("afl") => 5,
                                        Some("itf") => 6,
                                        Some("cs") => {
                                            // Disambiguate NinjaScript vs cAlgo by content
                                            if self.compiler_source.contains("NinjaScriptProperty")
                                                || self.compiler_source.contains("NinjaTrader")
                                            {
                                                7
                                            } else {
                                                8
                                            }
                                        }
                                        Some("cpp") | Some("h") => {
                                            // Sierra Chart ACSIL if it contains SierraChart.h or SCSF
                                            if self.compiler_source.contains("SierraChart.h")
                                                || self.compiler_source.contains("SCSFExport")
                                                || self
                                                    .compiler_source
                                                    .contains("SCStudyInterfaceRef")
                                            {
                                                9
                                            } else {
                                                0
                                            }
                                        }
                                        _ => 0,
                                    };
                                    self.log.push_back(LogEntry::info(format!(
                                        "Loaded: {}",
                                        path.display()
                                    )));
                                }
                            }
                        }
                        let compile_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("Compile").color(egui::Color32::WHITE),
                            )
                            .fill(BTN_BLUE),
                        );
                        if compile_btn.clicked() && !self.compiler_source.is_empty() {
                            let result = match self.compiler_language {
                                0 => mql5_compiler::compile_mql5(&self.compiler_source),
                                1 => mql5_compiler::compile_mql4(&self.compiler_source),
                                2 => mql5_compiler::compile_pine(&self.compiler_source),
                                3 => mql5_compiler::compile_easylang(&self.compiler_source),
                                4 => mql5_compiler::compile_thinkscript(&self.compiler_source),
                                5 => mql5_compiler::compile_afl(&self.compiler_source),
                                6 => mql5_compiler::compile_probuilder(&self.compiler_source),
                                7 => mql5_compiler::compile_ninjascript(&self.compiler_source),
                                8 => mql5_compiler::compile_calgo(&self.compiler_source),
                                9 => mql5_compiler::compile_acsil(&self.compiler_source),
                                _ => mql5_compiler::compile_mql5(&self.compiler_source),
                            };
                            self.compiler_diagnostics.clear();
                            for d in &result.diagnostics {
                                self.compiler_diagnostics.push_back(format!(
                                    "{}:{}: {} — {}",
                                    d.line,
                                    d.col,
                                    match d.level {
                                        mql5_compiler::DiagLevel::Error => "ERROR",
                                        mql5_compiler::DiagLevel::Warning => "WARN",
                                        _ => "INFO",
                                    },
                                    d.message
                                ));
                            }
                            if result.wasm.is_some() {
                                let wasm_size = result.wasm.as_ref().map(|w| w.len()).unwrap_or(0);
                                let buffers =
                                    result.metadata.as_ref().map(|m| m.buffers).unwrap_or(0);
                                let inputs = result
                                    .metadata
                                    .as_ref()
                                    .map(|m| m.inputs.len())
                                    .unwrap_or(0);
                                self.compiler_diagnostics.push_front(format!(
                                    "OK: compiled to {} bytes WASM — {} buffers, {} inputs",
                                    wasm_size, buffers, inputs
                                ));
                                self.log.push_back(LogEntry::info(format!(
                                    "Compiled: {} bytes WASM, {} buffers",
                                    wasm_size, buffers
                                )));
                            }
                            self.compiler_metadata = Some(result);
                        }
                    });

                    // ── Cross-language transpile row (ADR-090) ────────
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Transpile to:").small());
                        const TRANSPILE_TARGETS: &[(
                            &str,
                            mql5_compiler::transpile::TargetLanguage,
                        )] = &[
                            ("MQL5", mql5_compiler::transpile::TargetLanguage::Mql5),
                            ("MQL4", mql5_compiler::transpile::TargetLanguage::Mql4),
                            (
                                "PineScript v5",
                                mql5_compiler::transpile::TargetLanguage::PineScript,
                            ),
                            (
                                "EasyLanguage",
                                mql5_compiler::transpile::TargetLanguage::EasyLanguage,
                            ),
                            (
                                "thinkScript",
                                mql5_compiler::transpile::TargetLanguage::ThinkScript,
                            ),
                            (
                                "AFL (AmiBroker)",
                                mql5_compiler::transpile::TargetLanguage::Afl,
                            ),
                            (
                                "ProBuilder",
                                mql5_compiler::transpile::TargetLanguage::ProBuilder,
                            ),
                            (
                                "NinjaScript",
                                mql5_compiler::transpile::TargetLanguage::NinjaScript,
                            ),
                            (
                                "cAlgo (cTrader)",
                                mql5_compiler::transpile::TargetLanguage::Calgo,
                            ),
                            (
                                "ACSIL (Sierra Chart)",
                                mql5_compiler::transpile::TargetLanguage::Acsil,
                            ),
                        ];
                        egui::ComboBox::from_id_salt("compiler_transpile_target")
                            .selected_text(
                                TRANSPILE_TARGETS
                                    .get(self.compiler_transpile_target)
                                    .map(|(l, _)| *l)
                                    .unwrap_or("MQL5"),
                            )
                            .width(180.0)
                            .show_ui(ui, |ui| {
                                for (i, (label, _)) in TRANSPILE_TARGETS.iter().enumerate() {
                                    ui.selectable_value(
                                        &mut self.compiler_transpile_target,
                                        i,
                                        *label,
                                    );
                                }
                            });
                        if ui.button("Transpile").clicked() && !self.compiler_source.is_empty() {
                            use mql5_compiler::transpile::{SourceLanguage, transpile};
                            let from = match self.compiler_language {
                                0 => SourceLanguage::Mql5,
                                1 => SourceLanguage::Mql4,
                                2 => SourceLanguage::PineScript,
                                3 => SourceLanguage::EasyLanguage,
                                4 => SourceLanguage::ThinkScript,
                                5 => SourceLanguage::Afl,
                                6 => SourceLanguage::ProBuilder,
                                7 => SourceLanguage::NinjaScript,
                                8 => SourceLanguage::Calgo,
                                9 => SourceLanguage::Acsil,
                                _ => SourceLanguage::Mql5,
                            };
                            let to = TRANSPILE_TARGETS
                                .get(self.compiler_transpile_target)
                                .map(|(_, t)| *t)
                                .unwrap_or(mql5_compiler::transpile::TargetLanguage::Mql5);
                            match transpile(&self.compiler_source, from, to) {
                                Ok(out) => {
                                    let line_count = out.lines().count();
                                    self.compiler_transpiled = Some(out);
                                    self.log.push_back(LogEntry::info(format!(
                                        "Transpiled {:?} → {:?}: {} lines",
                                        from, to, line_count
                                    )));
                                }
                                Err(e) => {
                                    self.compiler_transpiled = None;
                                    self.log
                                        .push_back(LogEntry::err(format!("Transpile failed: {e}")));
                                    self.compiler_diagnostics
                                        .push_front(format!("TRANSPILE ERROR: {e}"));
                                }
                            }
                        }
                        if self.compiler_transpiled.is_some()
                            && ui.button("Use as Source").clicked()
                        {
                            if let Some(ref out) = self.compiler_transpiled {
                                self.compiler_source = out.clone();
                                // Map transpile-target index → language dropdown index.
                                // Transpile targets: 0=MQL5 1=MQL4 2=Pine 3=EL 4=TS 5=AFL 6=PB 7=Ninja 8=cAlgo
                                // Language dropdown: 0=MQL5 1=MQL4 2=Pine 3=EL 4=TS 5=AFL 6=PB 7=Ninja 8=cAlgo
                                // They happen to line up 1:1 after Phase 2.
                                self.compiler_language = self.compiler_transpile_target;
                                self.compiler_transpiled = None;
                            }
                        }
                        if self.compiler_transpiled.is_some() && ui.button("Copy").clicked() {
                            if let Some(ref out) = self.compiler_transpiled {
                                ui.ctx().copy_text(out.clone());
                            }
                        }
                    });
                    ui.separator();

                    // Source code editor
                    ui.label(egui::RichText::new("Source Code").small().strong());
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .max_height(280.0)
                        .id_salt("compiler_src")
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.compiler_source)
                                    .code_editor()
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(16)
                                    .font(egui::TextStyle::Monospace),
                            );
                        });
                    ui.separator();

                    // Diagnostics
                    if !self.compiler_diagnostics.is_empty() {
                        ui.label(egui::RichText::new("Diagnostics").small().strong());
                        egui::ScrollArea::vertical()
                            .auto_shrink(false)
                            .max_height(120.0)
                            .id_salt("compiler_diag")
                            .show(ui, |ui| {
                                for d in &self.compiler_diagnostics {
                                    let c = if d.starts_with("OK:") {
                                        cc_green
                                    } else if d.contains("ERROR") {
                                        cc_red
                                    } else {
                                        cc_dim
                                    };
                                    ui.label(egui::RichText::new(d).monospace().small().color(c));
                                }
                            });
                    }

                    // Metadata summary
                    if let Some(ref result) = self.compiler_metadata {
                        if let Some(ref meta) = result.metadata {
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("Name: {}", meta.short_name))
                                        .small()
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("Buffers: {}", meta.buffers))
                                        .color(cc_dim)
                                        .small(),
                                );
                                ui.label(
                                    egui::RichText::new(if meta.separate_window {
                                        "Separate Window"
                                    } else {
                                        "Chart Overlay"
                                    })
                                    .color(cc_dim)
                                    .small(),
                                );
                            });
                            if !meta.inputs.is_empty() {
                                ui.label(egui::RichText::new("Inputs:").small());
                                for inp in &meta.inputs {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "  {} ({}) = {}",
                                            inp.name, inp.param_type, inp.default_value
                                        ))
                                        .monospace()
                                        .small()
                                        .color(cc_dim),
                                    );
                                }
                            }
                            if !meta.plots.is_empty() {
                                ui.label(egui::RichText::new("Plots:").small());
                                for p in &meta.plots {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "  [{}] {} — {:?} color={}",
                                            p.index, p.label, p.draw_type, p.color
                                        ))
                                        .monospace()
                                        .small()
                                        .color(cc_dim),
                                    );
                                }
                            }
                        }
                    }

                    // Transpiled output panel
                    if let Some(ref transpiled) = self.compiler_transpiled {
                        ui.separator();
                        ui.label(
                            egui::RichText::new("Transpiled Output")
                                .small()
                                .strong()
                                .color(cc_green),
                        );
                        egui::ScrollArea::vertical()
                            .auto_shrink(false)
                            .max_height(200.0)
                            .id_salt("compiler_transpile_out")
                            .show(ui, |ui| {
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(transpiled).monospace().small(),
                                    )
                                    .wrap_mode(egui::TextWrapMode::Extend),
                                );
                            });
                    }
                });
        }

        // Risk-of-Ruin Calculator
        if self.show_risk_ruin {
            egui::Window::new("Risk-of-Ruin Calculator")
                .open(&mut self.show_risk_ruin)
                .resizable(true)
                .default_size([600.0, 450.0])
                .show(ctx, |ui| {
                    let rr_green = egui::Color32::from_rgb(46, 204, 113);
                    let rr_red = egui::Color32::from_rgb(231, 76, 60);
                    let rr_gold = egui::Color32::from_rgb(241, 196, 15);
                    let rr_dim = egui::Color32::from_rgb(100, 100, 120);

                    ui.label(egui::RichText::new("Monte Carlo Equity Path Simulation").strong());
                    ui.label(
                        egui::RichText::new(
                            "Simulate 10,000 trade sequences to estimate probability of ruin",
                        )
                        .color(rr_dim)
                        .small(),
                    );

                    // Input parameters
                    egui::Grid::new("ruin_params")
                        .num_columns(4)
                        .show(ui, |ui| {
                            ui.label("Win Rate %:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.ruin_win_rate)
                                    .desired_width(60.0),
                            );
                            ui.label("Avg Win $:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.ruin_avg_win)
                                    .desired_width(60.0),
                            );
                            ui.end_row();
                            ui.label("Avg Loss $:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.ruin_avg_loss)
                                    .desired_width(60.0),
                            );
                            ui.label("Risk %:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.ruin_risk_pct)
                                    .desired_width(60.0),
                            );
                            ui.end_row();
                        });

                    ui.horizontal(|ui| {
                        if ui
                            .button(
                                egui::RichText::new("Run 10,000 Simulations")
                                    .color(rr_green)
                                    .strong(),
                            )
                            .clicked()
                        {
                            let wr = self.ruin_win_rate.parse::<f64>().unwrap_or(55.0) / 100.0;
                            let avg_win = self.ruin_avg_win.parse::<f64>().unwrap_or(200.0);
                            let avg_loss = self.ruin_avg_loss.parse::<f64>().unwrap_or(150.0);
                            let _risk_pct = self.ruin_risk_pct.parse::<f64>().unwrap_or(2.0);

                            // CPU Monte Carlo (fast enough for 10K × 500 trades)
                            use std::collections::hash_map::DefaultHasher;
                            use std::hash::{Hash, Hasher};
                            let mut results = Vec::with_capacity(10000);
                            let starting_equity = 100000.0_f64;
                            for sim in 0..10000_u64 {
                                let mut equity = starting_equity;
                                let mut h = DefaultHasher::new();
                                sim.hash(&mut h);
                                let mut seed = h.finish();
                                for _ in 0..500 {
                                    // LCG random
                                    seed = seed
                                        .wrapping_mul(6364136223846793005)
                                        .wrapping_add(1442695040888963407);
                                    let r = (seed >> 33) as f64 / (u32::MAX as f64);
                                    if r < wr {
                                        equity += avg_win;
                                    } else {
                                        equity -= avg_loss;
                                    }
                                    if equity <= 0.0 {
                                        equity = 0.0;
                                        break;
                                    }
                                }
                                results.push(equity as f32);
                            }
                            results.sort_by(|a, b| {
                                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                            });
                            self.ruin_results = results;
                        }
                        // Auto-fill from journal
                        if !self.journal_entries.is_empty() {
                            if ui.small_button("Fill from Journal").clicked() {
                                let closed: Vec<_> = self
                                    .journal_entries
                                    .iter()
                                    .filter(|e| e.pnl.is_some())
                                    .collect();
                                if !closed.is_empty() {
                                    let wins: Vec<f64> = closed
                                        .iter()
                                        .filter_map(|e| e.pnl.filter(|&p| p > 0.0))
                                        .collect();
                                    let losses: Vec<f64> = closed
                                        .iter()
                                        .filter_map(|e| e.pnl.filter(|&p| p < 0.0).map(|p| p.abs()))
                                        .collect();
                                    let wr = wins.len() as f64 / closed.len() as f64 * 100.0;
                                    let avg_w = if wins.is_empty() {
                                        0.0
                                    } else {
                                        wins.iter().sum::<f64>() / wins.len() as f64
                                    };
                                    let avg_l = if losses.is_empty() {
                                        0.0
                                    } else {
                                        losses.iter().sum::<f64>() / losses.len() as f64
                                    };
                                    self.ruin_win_rate = format!("{:.1}", wr);
                                    self.ruin_avg_win = format!("{:.0}", avg_w);
                                    self.ruin_avg_loss = format!("{:.0}", avg_l);
                                }
                            }
                        }
                    });

                    if !self.ruin_results.is_empty() {
                        ui.separator();
                        let n = self.ruin_results.len();
                        let ruined = self.ruin_results.iter().filter(|&&e| e <= 0.0).count();
                        let ruin_pct = ruined as f64 / n as f64 * 100.0;
                        let median = self.ruin_results[n / 2];
                        let p5 = self.ruin_results[n * 5 / 100];
                        let p95 = self.ruin_results[n * 95 / 100];
                        let best = self.ruin_results.last().copied().unwrap_or(0.0);
                        let worst = self.ruin_results.first().copied().unwrap_or(0.0);

                        // Summary metrics
                        egui::Grid::new("ruin_metrics")
                            .num_columns(4)
                            .show(ui, |ui| {
                                let rc = if ruin_pct > 10.0 {
                                    rr_red
                                } else if ruin_pct > 1.0 {
                                    rr_gold
                                } else {
                                    rr_green
                                };
                                ui.label(egui::RichText::new("Prob of Ruin:").color(rr_dim));
                                ui.label(
                                    egui::RichText::new(format!("{:.2}%", ruin_pct))
                                        .color(rc)
                                        .strong(),
                                );
                                ui.label(egui::RichText::new("Median:").color(rr_dim));
                                ui.label(
                                    egui::RichText::new(format!("${:.0}", median)).color(rr_green),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("5th %ile:").color(rr_dim));
                                let p5c = if p5 < 100000.0 { rr_red } else { rr_dim };
                                ui.label(egui::RichText::new(format!("${:.0}", p5)).color(p5c));
                                ui.label(egui::RichText::new("95th %ile:").color(rr_dim));
                                ui.label(
                                    egui::RichText::new(format!("${:.0}", p95)).color(rr_green),
                                );
                                ui.end_row();
                                ui.label(egui::RichText::new("Worst:").color(rr_dim));
                                ui.label(
                                    egui::RichText::new(format!("${:.0}", worst)).color(rr_red),
                                );
                                ui.label(egui::RichText::new("Best:").color(rr_dim));
                                ui.label(
                                    egui::RichText::new(format!("${:.0}", best)).color(rr_green),
                                );
                                ui.end_row();
                            });

                        // Distribution chart (percentile buckets as line)
                        {
                            let pts: PlotPoints = PlotPoints::new(
                                (0..100)
                                    .map(|i| {
                                        let idx = i * n / 100;
                                        [i as f64, self.ruin_results[idx] as f64]
                                    })
                                    .collect(),
                            );
                            let c = if ruin_pct > 10.0 { rr_red } else { rr_green };
                            let line = Line::new("Equity Distribution", pts).color(c).width(1.5);
                            Plot::new("ruin_dist")
                                .height(200.0)
                                .allow_drag(false)
                                .allow_zoom(false)
                                .allow_scroll(false)
                                .show_axes([true, true])
                                .x_axis_label("Percentile")
                                .y_axis_label("Final Equity")
                                .show(ui, |plot_ui| {
                                    plot_ui.line(line);
                                    // Starting equity reference line
                                    let ref_pts =
                                        PlotPoints::new(vec![[0.0, 100000.0], [100.0, 100000.0]]);
                                    plot_ui.line(
                                        Line::new("Starting", ref_pts).color(rr_gold).width(1.0),
                                    );
                                });
                        }
                    }
                });
        }

        // Alert Builder + Alert Checker
        if self.show_alert_builder {
            egui::Window::new("Alert Builder")
                .open(&mut self.show_alert_builder)
                .resizable(true)
                .default_size([600.0, 400.0])
                .show(ctx, |ui| {
                    let al_green = egui::Color32::from_rgb(46, 204, 113);
                    let al_red = egui::Color32::from_rgb(231, 76, 60);
                    let al_gold = egui::Color32::from_rgb(241, 196, 15);
                    let al_cyan = egui::Color32::from_rgb(26, 188, 156);
                    let al_dim = egui::Color32::from_rgb(100, 100, 120);

                    // New alert form
                    ui.label(egui::RichText::new("Create Alert").strong());
                    ui.horizontal(|ui| {
                        ui.label("Symbol:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.alert_symbol).desired_width(80.0),
                        );
                        ui.label("Indicator:");
                        egui::ComboBox::from_id_salt("alert_ind")
                            .selected_text(ALERT_INDICATORS[self.alert_indicator])
                            .width(100.0)
                            .show_ui(ui, |ui| {
                                for (i, name) in ALERT_INDICATORS.iter().enumerate() {
                                    ui.selectable_value(&mut self.alert_indicator, i, *name);
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.label("Condition:");
                        egui::ComboBox::from_id_salt("alert_cond")
                            .selected_text(ALERT_CONDITIONS[self.alert_condition])
                            .width(130.0)
                            .show_ui(ui, |ui| {
                                for (i, name) in ALERT_CONDITIONS.iter().enumerate() {
                                    ui.selectable_value(&mut self.alert_condition, i, *name);
                                }
                            });
                        ui.label("Threshold:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.alert_threshold)
                                .desired_width(80.0),
                        );
                        if ui
                            .button(egui::RichText::new("+ Add").color(al_green))
                            .clicked()
                        {
                            if let Ok(thresh) = self.alert_threshold.parse::<f64>() {
                                let tf = self
                                    .charts
                                    .get(self.active_tab)
                                    .map(|c| c.timeframe.label().to_string())
                                    .unwrap_or("H4".into());
                                self.indicator_alerts.push(IndicatorAlert {
                                    symbol: self.alert_symbol.clone(),
                                    timeframe: tf,
                                    indicator: ALERT_INDICATORS[self.alert_indicator].to_string(),
                                    condition: ALERT_CONDITIONS[self.alert_condition].to_string(),
                                    threshold: thresh,
                                    active: true,
                                    triggered: false,
                                    last_value: None,
                                });
                                self.log.push_back(LogEntry::info(format!(
                                    "Alert: {} {} {} {}",
                                    self.alert_symbol,
                                    ALERT_INDICATORS[self.alert_indicator],
                                    ALERT_CONDITIONS[self.alert_condition],
                                    thresh
                                )));
                            }
                        }
                    });
                    ui.separator();

                    // Active alerts list
                    ui.label(
                        egui::RichText::new(format!(
                            "Active Alerts ({})",
                            self.indicator_alerts.len()
                        ))
                        .strong(),
                    );
                    let mut remove_idx: Option<usize> = None;
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            for (idx, alert) in self.indicator_alerts.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    let status_c = if alert.triggered {
                                        al_red
                                    } else if alert.active {
                                        al_green
                                    } else {
                                        al_dim
                                    };
                                    let status_t = if alert.triggered {
                                        "TRIGGERED"
                                    } else if alert.active {
                                        "ACTIVE"
                                    } else {
                                        "OFF"
                                    };
                                    ui.label(
                                        egui::RichText::new(status_t)
                                            .color(status_c)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(&alert.symbol)
                                            .color(al_cyan)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} {} {:.2}",
                                            alert.indicator, alert.condition, alert.threshold
                                        ))
                                        .small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(&alert.timeframe).color(al_dim).small(),
                                    );
                                    if let Some(v) = alert.last_value {
                                        ui.label(
                                            egui::RichText::new(format!("= {:.2}", v))
                                                .color(al_gold)
                                                .small(),
                                        );
                                    }
                                    ui.checkbox(&mut alert.active, "");
                                    if alert.triggered {
                                        if ui.small_button("Reset").clicked() {
                                            alert.triggered = false;
                                        }
                                    }
                                    if ui.small_button("x").clicked() {
                                        remove_idx = Some(idx);
                                    }
                                });
                            }
                        });
                    if let Some(idx) = remove_idx {
                        self.indicator_alerts.remove(idx);
                    }
                });
        }

        // Check indicator alerts against current chart data (every frame, cheap)
        {
            let chart_data: Option<(&str, &str, f64, f64, f64, f64, f64)> =
                self.charts.get(self.active_tab).and_then(|c| {
                    let n = c.bars.len();
                    if n < 2 {
                        return None;
                    }
                    let close = c.bars[n - 1].close;
                    let rsi = c.rsi.get(n - 1).and_then(|v| *v).unwrap_or(50.0);
                    let fisher = c.fisher.get(n - 1).and_then(|v| *v).unwrap_or(0.0);
                    let adx = c.adx.get(n - 1).and_then(|v| *v).unwrap_or(0.0);
                    let atr = c.atr.get(n - 1).and_then(|v| *v).unwrap_or(0.0);
                    Some((
                        &*c.symbol,
                        c.timeframe.label(),
                        close,
                        rsi,
                        fisher,
                        adx,
                        atr,
                    ))
                });

            if let Some((sym, _tf, close, rsi, fisher, adx, atr)) = chart_data {
                for alert in self.indicator_alerts.iter_mut() {
                    if !alert.active || alert.triggered {
                        continue;
                    }
                    // Only check if symbol matches current chart
                    if !sym.contains(&alert.symbol) {
                        continue;
                    }

                    let current_val = match alert.indicator.as_str() {
                        "Price" => close,
                        "RSI" => rsi,
                        "Fisher" => fisher,
                        "ADX" => adx,
                        "ATR" => atr,
                        _ => continue,
                    };

                    let prev_val = alert.last_value.unwrap_or(current_val);
                    let triggered = match alert.condition.as_str() {
                        "crosses above" => {
                            prev_val <= alert.threshold && current_val > alert.threshold
                        }
                        "crosses below" => {
                            prev_val >= alert.threshold && current_val < alert.threshold
                        }
                        "greater than" => current_val > alert.threshold,
                        "less than" => current_val < alert.threshold,
                        _ => false,
                    };

                    alert.last_value = Some(current_val);
                    if triggered {
                        alert.triggered = true;
                        let msg = format!(
                            "ALERT: {} {} {} {} (value: {:.2})",
                            alert.symbol,
                            alert.indicator,
                            alert.condition,
                            alert.threshold,
                            current_val
                        );
                        // Surface to the top-bar breach badge — trader cannot miss this.
                        self.alert_breach_count = self.alert_breach_count.saturating_add(1);
                        self.alert_last_breach_ts = chrono::Utc::now().timestamp();
                        self.alert_last_breach_msg = msg.clone();
                        // OS-level attention request: taskbar icon flashes, dock bounces on macOS,
                        // title bar flashes on Windows. No new crate dep — egui 0.34 supports this.
                        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                            egui::UserAttentionType::Critical,
                        ));
                        self.log.push_back(LogEntry::alert(&msg));
                        // ADR-094: Toast for triggered alerts
                        self.toasts.push(Toast {
                            message: msg.clone(),
                            color: egui::Color32::from_rgb(255, 165, 0),
                            created: std::time::Instant::now(),
                            duration: std::time::Duration::from_secs(10),
                            dismissable: true,
                            dismissed: false,
                        });
                        // Send notification if any provider configured
                        if !self.discord_webhook.is_empty()
                            || !self.ntfy_topic.is_empty()
                            || (!self.pushover_token.is_empty() && !self.pushover_user.is_empty())
                        {
                            let _ = self.broker_tx.send(BrokerCmd::SendNotification {
                                discord_webhook: self.discord_webhook.clone(),
                                pushover_token: self.pushover_token.clone(),
                                pushover_user: self.pushover_user.clone(),
                                ntfy_topic: self.ntfy_topic.clone(),
                                message: msg,
                            });
                        }
                    }
                }
            }
        }

        // Multi-Dimensional Outlier Scanner
        if self.show_darwinex_outliers {
            let outlier_scope_label = self.broker_scope_label().to_string();
            // PERF: read from per-frame cache
            let outlier_scoped_fund = self.cached_scoped_fundamentals.clone();
            let mut pending_action = SymbolAction::None;
            // UX7: pre-fetch sparklines for top outlier symbols
            let mut outlier_syms: Vec<String> = self
                .darwinex_outliers
                .iter()
                .take(200)
                .map(|o| o.symbol.clone())
                .collect();
            outlier_syms.extend(
                self.darwinex_multi_outliers
                    .iter()
                    .take(200)
                    .map(|o| o.symbol.clone()),
            );
            let mut outlier_sparklines: std::collections::HashMap<
                String,
                std::sync::Arc<Vec<f64>>,
            > = std::collections::HashMap::new();
            for sym in &outlier_syms {
                let closes = self.get_sparkline(sym);
                if !closes.is_empty() {
                    outlier_sparklines.insert(sym.to_uppercase(), closes);
                }
            }
            egui::Window::new("Outlier Scanner")
                .open(&mut self.show_darwinex_outliers)
                .resizable(true)
                .default_size([900.0, 600.0])
                .show(ctx, |ui| {
                    let ol_high = egui::Color32::from_rgb(231, 76, 60);
                    let ol_med = egui::Color32::from_rgb(241, 196, 15);
                    let ol_green = egui::Color32::from_rgb(46, 204, 113);
                    let ol_cyan = egui::Color32::from_rgb(26, 188, 156);
                    let ol_dim = egui::Color32::from_rgb(100, 100, 120);

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("Outlier Analysis [{}] — {} outliers, {} sectors", outlier_scope_label, self.darwinex_outliers.len(), self.darwinex_sector_stats.len())).strong());
                        if ui.small_button("Refresh").clicked() {
                            // Re-run with scope filter (respects SCOPE command)
                            if let Some(ref cache) = self.cache {
                                if let Some(_conn) = cache.try_connection() {
                                    let mut data: Vec<(String, String, String, f64)> = Vec::new();
                                    for f in &outlier_scoped_fund {
                                        let sector = if f.sector.is_empty() { "Unknown".to_string() } else { f.sector.clone() };
                                        let industry = if f.industry.is_empty() { sector.clone() } else { f.industry.clone() };
                                        if let Some(mc) = f.market_cap { if mc > 0.0 { data.push((f.symbol.clone(), sector, industry, mc)); } }
                                    }
                                    if data.len() >= 10 {
                                        let (o, s) = typhoon_engine::core::var::detect_outliers(&data, 1.5);
                                        self.darwinex_outliers = o; self.darwinex_sector_stats = s;
                                    }
                                }
                            }
                        }
                    });
                    ui.separator();

                    egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                        // Multi-dimensional anomaly table (VaR + EV + ATR + SEC)
                        if !self.darwinex_multi_outliers.is_empty() {
                            let extreme_count = self.darwinex_multi_outliers.iter().filter(|o| o.dimensions_flagged >= 3).count();
                            let high_count = self.darwinex_multi_outliers.iter().filter(|o| o.dimensions_flagged == 2).count();
                            ui.label(egui::RichText::new(format!("Multi-Signal Anomaly Scanner — {} EXTREME, {} HIGH, {} total",
                                extreme_count, high_count, self.darwinex_multi_outliers.len())).strong());
                            ui.label(egui::RichText::new("Score = sum of |z-scores| across flagged dimensions. Higher = more anomalous.").color(ol_dim).small());
                            ui.label(egui::RichText::new("Dims: P/E (risk) + MCap/EV (valuation) + Short Ratio (volatility) + SEC filings+insider trades (activity)").color(ol_dim).small());
                            ui.add_space(4.0);
                            // Sort outliers. Column indices:
                            //   0 Symbol, 1 Sector, 2 Industry, 3 Score, 4 Dims, 5 Tier,
                            //   6 P/E z, 7 EV z, 8 Short z, 9 SEC z
                            let mut sorted_outliers: Vec<&_> = self.darwinex_multi_outliers.iter().collect();
                            match self.outlier_sort.column {
                                0 => sorted_outliers.sort_by(|a, b| a.symbol.cmp(&b.symbol)),
                                1 => sorted_outliers.sort_by(|a, b| a.sector.cmp(&b.sector).then_with(|| a.industry.cmp(&b.industry))),
                                2 => sorted_outliers.sort_by(|a, b| a.industry.cmp(&b.industry).then_with(|| a.symbol.cmp(&b.symbol))),
                                3 => sorted_outliers.sort_by(|a, b| a.composite_score.partial_cmp(&b.composite_score).unwrap_or(std::cmp::Ordering::Equal)),
                                4 => sorted_outliers.sort_by(|a, b| a.dimensions_flagged.cmp(&b.dimensions_flagged)),
                                5 => sorted_outliers.sort_by(|a, b| a.tier.cmp(&b.tier)),
                                6 => sorted_outliers.sort_by(|a, b| a.var_z.abs().partial_cmp(&b.var_z.abs()).unwrap_or(std::cmp::Ordering::Equal)),
                                7 => sorted_outliers.sort_by(|a, b| a.ev_z.abs().partial_cmp(&b.ev_z.abs()).unwrap_or(std::cmp::Ordering::Equal)),
                                8 => sorted_outliers.sort_by(|a, b| a.atr_z.abs().partial_cmp(&b.atr_z.abs()).unwrap_or(std::cmp::Ordering::Equal)),
                                9 => sorted_outliers.sort_by(|a, b| a.sec_z.abs().partial_cmp(&b.sec_z.abs()).unwrap_or(std::cmp::Ordering::Equal)),
                                _ => {}
                            }
                            if !self.outlier_sort.ascending { sorted_outliers.reverse(); }

                            egui::Grid::new("multi_outlier_grid").striped(true).num_columns(11).min_col_width(50.0).show(ui, |ui| {
                                if SortState::header(ui, "Symbol", 0, &self.outlier_sort) { self.outlier_sort.toggle(0); }
                                ui.label(egui::RichText::new("30d").color(ol_dim).small());
                                if SortState::header(ui, "Sector", 1, &self.outlier_sort) { self.outlier_sort.toggle(1); }
                                if SortState::header(ui, "Industry", 2, &self.outlier_sort) { self.outlier_sort.toggle(2); }
                                if SortState::header(ui, "Score", 3, &self.outlier_sort) { self.outlier_sort.toggle(3); }
                                if SortState::header(ui, "Dims", 4, &self.outlier_sort) { self.outlier_sort.toggle(4); }
                                if SortState::header(ui, "Tier", 5, &self.outlier_sort) { self.outlier_sort.toggle(5); }
                                if SortState::header(ui, "P/E z", 6, &self.outlier_sort) { self.outlier_sort.toggle(6); }
                                if SortState::header(ui, "EV z", 7, &self.outlier_sort) { self.outlier_sort.toggle(7); }
                                if SortState::header(ui, "Short z", 8, &self.outlier_sort) { self.outlier_sort.toggle(8); }
                                if SortState::header(ui, "SEC z", 9, &self.outlier_sort) { self.outlier_sort.toggle(9); }
                                ui.end_row();
                                // PERF: tradability set is cached on self.cached_darwin_symbols —
                                // rebuilt only on bg_rev change, not per frame.
                                let darwin_symbols = &self.cached_darwin_symbols;

                                for o in sorted_outliers.iter().take(200) {
                                    let tier_c = match o.tier.as_str() {
                                        "EXTREME" => ol_high, "HIGH" => ol_med, _ => ol_green
                                    };
                                    let z_color = |z: f64| -> egui::Color32 {
                                        if z.abs() > 2.0 { ol_high } else if z.abs() > 1.5 { ol_med } else { ol_dim }
                                    };
                                    // Tradability: green dot = in MT5 (tradable), dim = close-only.
                                    // o.symbol is guaranteed uppercase (built from Fundamentals).
                                    let tradable = darwin_symbols.contains(o.symbol.as_str());
                                    let sym_color = if tradable { egui::Color32::WHITE } else { egui::Color32::from_rgb(80, 80, 90) };
                                    let trade_icon = if tradable { "\u{25CF} " } else { "\u{25CB} " };
                                    ui.horizontal(|ui| {
                                        let (_, action) = symbol_label_with_menu(ui, &o.symbol,
                                            egui::RichText::new(format!("{}{}", trade_icon, o.symbol)).small().strong().color(sym_color));
                                        if !matches!(action, SymbolAction::None) { pending_action = action; }
                                        if ui.small_button(egui::RichText::new("+").small()).on_hover_text("Open new chart").clicked() {
                                            pending_action = SymbolAction::OpenChart(o.symbol.clone());
                                        }
                                    });
                                    // UX7: Sparkline column (o.symbol is already uppercase).
                                    if let Some(closes) = outlier_sparklines.get(o.symbol.as_str()) {
                                        draw_inline_sparkline(ui, closes, 50.0, 12.0);
                                    } else {
                                        ui.label(egui::RichText::new("—").color(ol_dim).small());
                                    }
                                    ui.label(egui::RichText::new(&o.sector).small().color(ol_cyan));
                                    ui.label(egui::RichText::new(&o.industry).small());
                                    ui.label(egui::RichText::new(format!("{:.1}", o.composite_score)).small().color(tier_c).strong());
                                    ui.label(egui::RichText::new(format!("{}/4", o.dimensions_flagged)).small().color(tier_c));
                                    ui.label(egui::RichText::new(&o.tier).small().color(tier_c));
                                    ui.label(egui::RichText::new(format!("{:+.1}", o.var_z)).small().color(z_color(o.var_z)));
                                    ui.label(egui::RichText::new(format!("{:+.1}", o.ev_z)).small().color(z_color(o.ev_z)));
                                    ui.label(egui::RichText::new(format!("{:+.1}", o.atr_z)).small().color(z_color(o.atr_z)));
                                    ui.label(egui::RichText::new(format!("{:+.1}", o.sec_z)).small().color(z_color(o.sec_z)));
                                    ui.end_row();
                                }
                            });
                            ui.add_space(8.0);
                            ui.separator();
                        }

                        // Sector summary (single-dimension)
                        if !self.darwinex_sector_stats.is_empty() {
                            ui.label(egui::RichText::new("Sector Statistics").small().strong());
                            egui::Grid::new("sector_stats_grid").striped(true).num_columns(6).min_col_width(60.0).show(ui, |ui| {
                                ui.label(egui::RichText::new("Sector").color(ol_dim).small());
                                ui.label(egui::RichText::new("Count").color(ol_dim).small());
                                ui.label(egui::RichText::new("Median").color(ol_dim).small());
                                ui.label(egui::RichText::new("IQR").color(ol_dim).small());
                                ui.label(egui::RichText::new("Bounds").color(ol_dim).small());
                                ui.label(egui::RichText::new("Outliers").color(ol_dim).small());
                                ui.end_row();
                                for s in &self.darwinex_sector_stats {
                                    ui.label(egui::RichText::new(&s.sector).small().color(ol_cyan));
                                    ui.label(format!("{}", s.count));
                                    ui.label(typhoon_engine::core::fundamentals::format_large_number(s.median));
                                    ui.label(typhoon_engine::core::fundamentals::format_large_number(s.iqr));
                                    ui.label(egui::RichText::new(format!("{} – {}",
                                        typhoon_engine::core::fundamentals::format_large_number(s.lower_bound),
                                        typhoon_engine::core::fundamentals::format_large_number(s.upper_bound)
                                    )).color(ol_dim).small());
                                    let oc = if s.outlier_count > 3 { ol_high } else if s.outlier_count > 0 { ol_med } else { ol_green };
                                    ui.label(egui::RichText::new(format!("{}", s.outlier_count)).color(oc));
                                    ui.end_row();
                                }
                            });
                            ui.add_space(8.0);
                        }

                        // Outlier table (single-metric) — click headers to sort
                        if !self.darwinex_outliers.is_empty() {
                            ui.label(egui::RichText::new("Outliers (click headers to sort)").small().strong());
                            // Sort outliers per header state. Columns:
                            //   0 Symbol, 1 Sector, 2 Industry, 3 Value, 4 Median,
                            //   5 Tier, 6 Z-Score (|z|), 7 Direction
                            // ("30d" sparkline is display-only between Symbol and Sector — no sort.)
                            let mut sorted_single: Vec<&_> = self.darwinex_outliers.iter().collect();
                            match self.outlier_single_sort.column {
                                0 => sorted_single.sort_by(|a, b| a.symbol.cmp(&b.symbol)),
                                1 => sorted_single.sort_by(|a, b| a.sector.cmp(&b.sector).then_with(|| a.industry.cmp(&b.industry))),
                                2 => sorted_single.sort_by(|a, b| a.industry.cmp(&b.industry).then_with(|| a.symbol.cmp(&b.symbol))),
                                3 => sorted_single.sort_by(|a, b| a.metric.partial_cmp(&b.metric).unwrap_or(std::cmp::Ordering::Equal)),
                                4 => sorted_single.sort_by(|a, b| a.sector_median.partial_cmp(&b.sector_median).unwrap_or(std::cmp::Ordering::Equal)),
                                5 => sorted_single.sort_by(|a, b| a.tier.cmp(&b.tier)),
                                6 => sorted_single.sort_by(|a, b| a.z_score.abs().partial_cmp(&b.z_score.abs()).unwrap_or(std::cmp::Ordering::Equal)),
                                7 => sorted_single.sort_by(|a, b| a.direction.cmp(&b.direction)),
                                _ => {}
                            }
                            if !self.outlier_single_sort.ascending { sorted_single.reverse(); }

                            egui::Grid::new("outliers_grid").striped(true).num_columns(9).min_col_width(50.0).show(ui, |ui| {
                                if SortState::header(ui, "Symbol", 0, &self.outlier_single_sort) { self.outlier_single_sort.toggle(0); }
                                ui.label(egui::RichText::new("30d").color(ol_dim).small());
                                if SortState::header(ui, "Sector", 1, &self.outlier_single_sort) { self.outlier_single_sort.toggle(1); }
                                if SortState::header(ui, "Industry", 2, &self.outlier_single_sort) { self.outlier_single_sort.toggle(2); }
                                if SortState::header(ui, "Value", 3, &self.outlier_single_sort) { self.outlier_single_sort.toggle(3); }
                                if SortState::header(ui, "Median", 4, &self.outlier_single_sort) { self.outlier_single_sort.toggle(4); }
                                if SortState::header(ui, "Tier", 5, &self.outlier_single_sort) { self.outlier_single_sort.toggle(5); }
                                if SortState::header(ui, "Z-Score", 6, &self.outlier_single_sort) { self.outlier_single_sort.toggle(6); }
                                if SortState::header(ui, "Dir", 7, &self.outlier_single_sort) { self.outlier_single_sort.toggle(7); }
                                ui.end_row();
                                let mut scrolled = false;
                                for o in &sorted_single {
                                    let mut sym_resp_opt: Option<egui::Response> = None;
                                    ui.horizontal(|ui| {
                                        let (sym_resp, action) = symbol_label_with_menu(ui, &o.symbol,
                                            egui::RichText::new(&o.symbol).strong().color(ol_cyan));
                                        if !matches!(action, SymbolAction::None) {
                                            pending_action = action;
                                        }
                                        if ui.small_button(egui::RichText::new("+").small()).on_hover_text("Open new chart").clicked() {
                                            pending_action = SymbolAction::OpenChart(o.symbol.clone());
                                        }
                                        sym_resp_opt = Some(sym_resp);
                                    });
                                    let sym_resp = sym_resp_opt.unwrap();
                                    // UX6: Auto-scroll to first EXTREME tier outlier on pending flag
                                    if self.outlier_scroll_pending && !scrolled && o.tier == "EXTREME" {
                                        sym_resp.scroll_to_me(Some(egui::Align::Center));
                                        scrolled = true;
                                    }
                                    // UX7: Sparkline column (o.symbol is already uppercase).
                                    if let Some(closes) = outlier_sparklines.get(o.symbol.as_str()) {
                                        draw_inline_sparkline(ui, closes, 50.0, 12.0);
                                    } else {
                                        ui.label(egui::RichText::new("—").color(ol_dim).small());
                                    }
                                    ui.label(egui::RichText::new(&o.sector).small().color(ol_cyan));
                                    ui.label(egui::RichText::new(&o.industry).small());
                                    ui.label(typhoon_engine::core::fundamentals::format_large_number(o.metric));
                                    ui.label(typhoon_engine::core::fundamentals::format_large_number(o.sector_median));
                                    let tc = match o.tier.as_str() { "EXTREME" => ol_high, "HIGH" => ol_med, _ => ol_dim };
                                    ui.label(egui::RichText::new(&o.tier).color(tc).small());
                                    let zc = if o.z_score.abs() > 3.0 { ol_high } else if o.z_score.abs() > 2.0 { ol_med } else { ol_dim };
                                    ui.label(egui::RichText::new(format!("{:.2}", o.z_score)).color(zc));
                                    let dc = if o.direction == "high" { ol_green } else { ol_high };
                                    ui.label(egui::RichText::new(&o.direction).color(dc).small());
                                    ui.end_row();
                                }
                                if scrolled || self.outlier_scroll_pending {
                                    self.outlier_scroll_pending = false;
                                }
                            });
                        } else {
                            ui.label(egui::RichText::new("No outliers detected. Run EVSCRAPE first, then OUTLIERS.").color(ol_dim));
                        }
                    });
                });
            // Apply deferred symbol context menu action (after window borrow released)
            self.apply_symbol_action(pending_action);
        }

        // Trade Journal
        if self.show_journal {
            egui::Window::new("Trade Journal")
                .open(&mut self.show_journal)
                .resizable(true)
                .default_size([700.0, 500.0])
                .show(ctx, |ui| {
                    let j_green = egui::Color32::from_rgb(46, 204, 113);
                    let j_red = egui::Color32::from_rgb(231, 76, 60);
                    let j_dim = egui::Color32::from_rgb(100, 100, 120);
                    let j_cyan = egui::Color32::from_rgb(26, 188, 156);

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Trade Journal").strong());
                        ui.label(
                            egui::RichText::new(format!("{} entries", self.journal_entries.len()))
                                .color(j_dim)
                                .small(),
                        );
                        // Quick add from current chart
                        if ui
                            .small_button(egui::RichText::new("+ Add Trade").color(j_green))
                            .clicked()
                        {
                            let sym = self
                                .charts
                                .get(self.active_tab)
                                .map(|c| c.symbol.clone())
                                .unwrap_or_default();
                            let price = self
                                .charts
                                .get(self.active_tab)
                                .and_then(|c| c.bars.last().map(|b| b.close))
                                .unwrap_or(0.0);
                            self.journal_entries.push(JournalEntry {
                                timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string(),
                                symbol: sym,
                                side: "BUY".to_string(),
                                qty: 1.0,
                                entry_price: price,
                                exit_price: None,
                                pnl: None,
                                strategy: "NNFX".to_string(),
                                notes: String::new(),
                            });
                        }
                    });
                    ui.separator();

                    // Summary stats
                    if !self.journal_entries.is_empty() {
                        let total_pnl: f64 =
                            self.journal_entries.iter().filter_map(|e| e.pnl).sum();
                        let closed = self
                            .journal_entries
                            .iter()
                            .filter(|e| e.pnl.is_some())
                            .count();
                        let wins = self
                            .journal_entries
                            .iter()
                            .filter(|e| e.pnl.map(|p| p > 0.0).unwrap_or(false))
                            .count();
                        let wr = if closed > 0 {
                            wins as f64 / closed as f64 * 100.0
                        } else {
                            0.0
                        };
                        ui.horizontal(|ui| {
                            let pc = if total_pnl >= 0.0 { j_green } else { j_red };
                            ui.label(
                                egui::RichText::new(format!("P&L: ${:.0}", total_pnl))
                                    .color(pc)
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new(format!("Closed: {}", closed))
                                    .color(j_dim)
                                    .small(),
                            );
                            let wc = if wr >= 50.0 { j_green } else { j_red };
                            ui.label(
                                egui::RichText::new(format!("Win: {:.0}%", wr))
                                    .color(wc)
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "Open: {}",
                                    self.journal_entries.len() - closed
                                ))
                                .color(j_cyan)
                                .small(),
                            );
                        });
                        ui.separator();
                    }

                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            let mut delete_idx: Option<usize> = None;
                            for (idx, entry) in self.journal_entries.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(&entry.timestamp).color(j_dim).small(),
                                    );
                                    let side_c = if entry.side == "BUY" { j_green } else { j_red };
                                    ui.label(
                                        egui::RichText::new(&entry.side)
                                            .color(side_c)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(&entry.symbol)
                                            .color(j_cyan)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{:.2} @ {}",
                                            entry.qty,
                                            format_price(entry.entry_price)
                                        ))
                                        .small(),
                                    );
                                    if let Some(pnl) = entry.pnl {
                                        let pc = if pnl >= 0.0 { j_green } else { j_red };
                                        ui.label(
                                            egui::RichText::new(format!("${:.0}", pnl))
                                                .color(pc)
                                                .small()
                                                .strong(),
                                        );
                                    } else {
                                        ui.label(egui::RichText::new("open").color(j_cyan).small());
                                    }
                                    ui.label(
                                        egui::RichText::new(&entry.strategy).color(j_dim).small(),
                                    );
                                    if ui.small_button("x").clicked() {
                                        delete_idx = Some(idx);
                                    }
                                });
                                // Editable notes
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("  Notes:").color(j_dim).small());
                                    ui.add(
                                        egui::TextEdit::singleline(&mut entry.notes)
                                            .desired_width(400.0)
                                            .hint_text("Trade notes..."),
                                    );
                                });
                                // Close trade button
                                if entry.exit_price.is_none() {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new("  Exit:").color(j_dim).small(),
                                        );
                                        let mut exit_str = entry
                                            .exit_price
                                            .map(|p| format!("{:.4}", p))
                                            .unwrap_or_default();
                                        ui.add(
                                            egui::TextEdit::singleline(&mut exit_str)
                                                .desired_width(80.0)
                                                .hint_text("exit price"),
                                        );
                                        if let Ok(ep) = exit_str.parse::<f64>() {
                                            entry.exit_price = Some(ep);
                                            let pnl = if entry.side == "BUY" {
                                                (ep - entry.entry_price) * entry.qty
                                            } else {
                                                (entry.entry_price - ep) * entry.qty
                                            };
                                            entry.pnl = Some(pnl);
                                        }
                                    });
                                }
                                ui.add_space(2.0);
                            }
                            if let Some(idx) = delete_idx {
                                self.journal_entries.remove(idx);
                            }
                        });
                    // Cumulative P&L line chart
                    let closed_pnls: Vec<f64> =
                        self.journal_entries.iter().filter_map(|e| e.pnl).collect();
                    if closed_pnls.len() >= 2 {
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("Cumulative P&L").strong());
                        let mut cum = 0.0_f64;
                        let pts: PlotPoints = PlotPoints::new(
                            closed_pnls
                                .iter()
                                .enumerate()
                                .map(|(i, &p)| {
                                    cum += p;
                                    [i as f64, cum]
                                })
                                .collect(),
                        );
                        let color = if cum >= 0.0 { UP } else { DOWN };
                        let line = Line::new("Cumulative P&L", pts).color(color).width(1.5);
                        Plot::new("journal_cum_pnl")
                            .height(100.0)
                            .allow_drag(false)
                            .allow_zoom(false)
                            .show(ui, |plot_ui| {
                                plot_ui.line(line);
                            });
                    }
                });
        }

        // VaR Multiplier — reads from bg cache
        if self.show_var_mult {
            egui::Window::new("VaR Multiplier")
                .open(&mut self.show_var_mult)
                .resizable(true)
                .default_size([450.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("Darwinex VaR Corridor");
                    ui.separator();
                    if !self.bg.per_darwin_var.is_empty() {
                        egui::Grid::new("var_per_darwin")
                            .striped(true)
                            .num_columns(5)
                            .show(ui, |ui| {
                                ui.strong("DARWIN");
                                ui.strong("VaR 95%");
                                ui.strong("Vol");
                                ui.strong("Sharpe");
                                ui.strong("Status");
                                ui.end_row();
                                for (ticker, vs) in &self.bg.per_darwin_var {
                                    ui.label(ticker);
                                    ui.label(format!("${:.2}", vs.var_95));
                                    ui.label(format!("{:.4}", vs.annualized_vol));
                                    ui.label(format!("{:.3}", vs.sharpe));
                                    let var_pct = vs.annualized_vol * 100.0;
                                    let status = if var_pct >= 3.25 && var_pct <= 6.5 {
                                        ("IN", UP)
                                    } else if var_pct < 3.25 {
                                        ("LOW", egui::Color32::from_rgb(255, 200, 50))
                                    } else {
                                        ("HIGH", DOWN)
                                    };
                                    ui.label(
                                        egui::RichText::new(status.0).color(status.1).strong(),
                                    );
                                    ui.end_row();
                                }
                            });
                        // VaR Multipliers (from bg cache)
                        ui.add_space(10.0);
                        ui.heading("VaR Multipliers");
                        ui.separator();
                        if !self.bg.var_multipliers.is_empty() {
                            egui::Grid::new("var_mult_grid")
                                .striped(true)
                                .num_columns(6)
                                .show(ui, |ui| {
                                    ui.strong("DARWIN");
                                    ui.strong("Monthly VaR");
                                    ui.strong("Multiplier");
                                    ui.strong("Inv. Return");
                                    ui.strong("Corridor");
                                    ui.strong("45d VaR");
                                    ui.end_row();
                                    for m in &self.bg.var_multipliers {
                                        ui.label(&m.darwin_ticker);
                                        ui.label(format!("{:.2}%", m.monthly_var));
                                        let mc = if m.multiplier >= 1.5 {
                                            UP
                                        } else if m.multiplier >= 0.8 {
                                            egui::Color32::from_rgb(255, 200, 50)
                                        } else {
                                            DOWN
                                        };
                                        ui.label(
                                            egui::RichText::new(format!("{:.2}x", m.multiplier))
                                                .color(mc),
                                        );
                                        let irf_c = if m.investor_return_factor >= 1.0 {
                                            UP
                                        } else {
                                            DOWN
                                        };
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:.2}x",
                                                m.investor_return_factor
                                            ))
                                            .color(irf_c),
                                        );
                                        let cc = if m.in_corridor { UP } else { DOWN };
                                        ui.label(
                                            egui::RichText::new(&m.corridor_position).color(cc),
                                        );
                                        {
                                            let v45_c = if m.var_45d >= 3.25 && m.var_45d <= 6.5 {
                                                UP
                                            } else {
                                                DOWN
                                            };
                                            ui.label(
                                                egui::RichText::new(format!("{:.2}%", m.var_45d))
                                                    .color(v45_c),
                                            );
                                        }
                                        ui.end_row();
                                    }
                                });
                            // Per-DARWIN recommendations
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new("Recommendations").strong());
                            for m in &self.bg.var_multipliers {
                                if !m.recommendation.is_empty() {
                                    let rc = if m.in_corridor {
                                        AXIS_TEXT
                                    } else {
                                        egui::Color32::from_rgb(255, 200, 50)
                                    };
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{}: {}",
                                            m.darwin_ticker, m.recommendation
                                        ))
                                        .color(rc)
                                        .small(),
                                    );
                                }
                            }
                        }
                    } else {
                        ui.label(egui::RichText::new("Import DARWIN data first.").color(AXIS_TEXT));
                    }
                    ui.add_space(10.0);
                    ui.separator();
                    egui::Grid::new("var_rules").num_columns(2).show(ui, |ui| {
                        ui.label("Target corridor:");
                        ui.label(egui::RichText::new("3.25% – 6.5%").strong());
                        ui.end_row();
                        ui.label("Correlation limit:");
                        ui.label(egui::RichText::new("0.95 / 45d").strong());
                        ui.end_row();
                        ui.label("Margin accounts:");
                        ui.label(egui::RichText::new("100%").strong());
                        ui.end_row();
                    });

                    // VaR Corridor Gauge
                    if !self.bg.per_darwin_var.is_empty() {
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("VaR Corridor Gauge").strong());
                        let bar_w = 400.0_f32;
                        let bar_h = 24.0_f32;
                        let max_pct = 10.0_f32; // 0-10% range
                        for (ticker, vs) in &self.bg.per_darwin_var {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(ticker).monospace().small());
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(bar_w, bar_h),
                                    egui::Sense::hover(),
                                );
                                let painter = ui.painter_at(rect);
                                // Background (dark)
                                painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(30, 30, 50));
                                // Green corridor zone: 3.25% - 6.5%
                                let lo_frac = 3.25 / max_pct as f64;
                                let hi_frac = 6.5 / max_pct as f64;
                                let green_left = rect.left() + lo_frac as f32 * bar_w;
                                let green_right = rect.left() + hi_frac as f32 * bar_w;
                                painter.rect_filled(
                                    egui::Rect::from_min_max(
                                        egui::pos2(green_left, rect.top()),
                                        egui::pos2(green_right, rect.bottom()),
                                    ),
                                    0.0,
                                    egui::Color32::from_rgba_premultiplied(76, 175, 80, 60),
                                );
                                // Current VaR position marker
                                let var_pct = vs.annualized_vol * 100.0;
                                let frac = (var_pct / max_pct as f64).clamp(0.0, 1.0) as f32;
                                let mx = rect.left() + frac * bar_w;
                                painter.rect_filled(
                                    egui::Rect::from_center_size(
                                        egui::pos2(mx, rect.center().y),
                                        egui::vec2(3.0, bar_h),
                                    ),
                                    0.0,
                                    egui::Color32::WHITE,
                                );
                                painter.text(
                                    egui::pos2(mx, rect.top() - 2.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    format!("{:.1}%", var_pct),
                                    egui::FontId::proportional(9.0),
                                    egui::Color32::WHITE,
                                );
                            });
                        }
                        // Legend
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("0%").color(AXIS_TEXT).small());
                            ui.add_space(bar_w * 0.28);
                            ui.label(egui::RichText::new("3.25%").color(UP).small());
                            ui.add_space(bar_w * 0.22);
                            ui.label(egui::RichText::new("6.5%").color(UP).small());
                            ui.add_space(bar_w * 0.15);
                            ui.label(egui::RichText::new("10%").color(AXIS_TEXT).small());
                        });
                    }
                });
        }

        // Margin Monitor — wired to margin.rs functions
        if self.show_margin_monitor {
            egui::Window::new("Margin Monitor")
                .open(&mut self.show_margin_monitor)
                .resizable(true).default_size([450.0, 350.0])
                .show(ctx, |ui| {
                    ui.heading("Margin Calculator");
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Equity:"); ui.add(egui::TextEdit::singleline(&mut self.mm_equity).desired_width(100.0));
                        ui.label("Margin Used:"); ui.add(egui::TextEdit::singleline(&mut self.mm_margin).desired_width(100.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Margin/Lot:"); ui.add(egui::TextEdit::singleline(&mut self.mm_margin_per_lot).desired_width(100.0));
                        ui.label("TRIM %:"); ui.add(egui::TextEdit::singleline(&mut self.mm_trim_pct).desired_width(60.0));
                    });
                    if ui.button("Calculate").clicked() {
                        let equity: f64 = self.mm_equity.replace(['$', ','], "").parse().unwrap_or(0.0);
                        let margin_used: f64 = self.mm_margin.replace(['$', ','], "").parse().unwrap_or(0.0);
                        let margin_per_lot: f64 = self.mm_margin_per_lot.replace(['$', ','], "").parse().unwrap_or(1000.0);
                        let trim_pct: f64 = self.mm_trim_pct.parse().unwrap_or(150.0);
                        if equity > 0.0 {
                            let ml = margin::margin_level_pct(equity, margin_used);
                            let free = margin::usable_margin(equity, margin_used, 10.0);
                            let max_lots = margin::max_safe_lots(equity, margin_used, margin_per_lot, trim_pct);
                            let urgency = margin::protect_urgency(ml, trim_pct);
                            self.mm_result = format!(
                                "Margin Level: {:.2}%\nFree Margin: ${:.2}\nMax Safe Lots: {}\nProtect Urgency: {:.2}",
                                ml, free, max_lots, urgency
                            );
                        }
                    }
                    if !self.mm_result.is_empty() {
                        ui.separator();
                        ui.label(egui::RichText::new(&self.mm_result).monospace().color(egui::Color32::from_rgb(200, 220, 255)));
                        // Visual margin level gauge
                        let equity: f64 = self.mm_equity.replace(['$', ','], "").parse().unwrap_or(0.0);
                        let margin_used: f64 = self.mm_margin.replace(['$', ','], "").parse().unwrap_or(0.0);
                        if margin_used > 0.0 && equity > 0.0 {
                            let ml = equity / margin_used * 100.0;
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new("Margin Level Gauge").strong());
                            let bar_w = 360.0_f32;
                            let bar_h = 22.0_f32;
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(bar_w, bar_h), egui::Sense::hover());
                            let painter = ui.painter_at(rect);
                            // Draw zones: red 0-100%, yellow 100-200%, green 200-400%
                            let r_end = (bar_w * 0.25).min(bar_w);
                            let y_end = (bar_w * 0.50).min(bar_w);
                            painter.rect_filled(egui::Rect::from_min_max(rect.min, egui::pos2(rect.min.x + r_end, rect.max.y)), 0.0, egui::Color32::from_rgb(180, 40, 40));
                            painter.rect_filled(egui::Rect::from_min_max(egui::pos2(rect.min.x + r_end, rect.min.y), egui::pos2(rect.min.x + y_end, rect.max.y)), 0.0, egui::Color32::from_rgb(200, 180, 40));
                            painter.rect_filled(egui::Rect::from_min_max(egui::pos2(rect.min.x + y_end, rect.min.y), rect.max), 0.0, egui::Color32::from_rgb(40, 160, 60));
                            // Marker for current level (clamped to 0-400%)
                            let frac = (ml / 400.0).clamp(0.0, 1.0) as f32;
                            let mx = rect.min.x + frac * bar_w;
                            painter.rect_filled(egui::Rect::from_center_size(egui::pos2(mx, rect.center().y), egui::vec2(3.0, bar_h)), 0.0, egui::Color32::WHITE);
                            painter.text(egui::pos2(mx, rect.min.y - 2.0), egui::Align2::CENTER_BOTTOM, format!("{:.0}%", ml), egui::FontId::proportional(10.0), egui::Color32::WHITE);
                        }
                    }
                });
        }

        self.render_cache_stats_window(ctx);

        // Storage Manager
        if self.show_storage {
            let mut storage_save_after = false;
            let mut show_storage = self.show_storage;
            egui::Window::new("Storage Manager")
                .open(&mut show_storage)
                .resizable(true).default_size([650.0, 500.0])
                .scroll([false, true])
                .show(ctx, |ui| {
                    // Summary stats at top
                    if let Some((rows, kv, size)) = self.bg.cache_stats {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("Bar entries: {} | KV entries: {} | DB size on disk: {:.1} MB", rows, kv, size as f64 / 1024.0 / 1024.0)).small());
                        });
                        // One-line bar-sync banner — per-broker % healthy with a
                        // `[Details]` button opening the full Sync Status window.
                        let stats_rows = self.compute_bar_sync_rows();
                        let totals = compute_bar_sync_broker_totals(&stats_rows);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Sync:").color(AXIS_TEXT).small().strong());
                            for (broker, total, _healthy, pct) in &totals {
                                let color = if *total == 0 {
                                    egui::Color32::from_rgb(150, 150, 150)
                                } else if *pct >= 90.0 {
                                    egui::Color32::from_rgb(26, 188, 156)
                                } else if *pct >= 50.0 {
                                    egui::Color32::from_rgb(241, 196, 15)
                                } else {
                                    egui::Color32::from_rgb(231, 76, 60)
                                };
                                ui.label(egui::RichText::new(format!("{} {:.1}%", broker, pct)).color(color).small().monospace());
                                ui.label(egui::RichText::new("|").color(AXIS_TEXT).small());
                            }
                            if ui.small_button(egui::RichText::new("Details").small()).clicked() {
                                self.show_sync_status = true;
                            }
                        });
                        if self.alpaca_enabled {
                            self.render_alpaca_sync_profile_controls(
                                ui,
                                &mut storage_save_after,
                                "storage_manager",
                            );
                        }
                        self.render_sync_timeframe_controls(ui, &mut storage_save_after);
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Base bar zstd").color(AXIS_TEXT).small());
                            let mut level = self.bar_zstd_level;
                            if ui
                                .add(
                                    egui::Slider::new(
                                        &mut level,
                                        typhoon_engine::core::cache::MIN_ZSTD_LEVEL
                                            ..=typhoon_engine::core::cache::MAX_ZSTD_LEVEL,
                                    )
                                    .integer()
                                    .show_value(true),
                                )
                                .on_hover_text(
                                    "Compression level for normal foreground bar-cache writes. Lower = faster sync/import writes; higher = smaller disk. Kraken WS hot writes remain fixed at zstd-3; Compact promotes rows to zstd-22.",
                                )
                                .changed()
                            {
                                self.bar_zstd_level = typhoon_engine::core::cache::set_bar_zstd_level(level);
                                storage_save_after = true;
                                self.log.push_back(LogEntry::info(format!(
                                    "Base bar-cache zstd level set to {}",
                                    self.bar_zstd_level
                                )));
                            }
                            if ui.small_button("Fast 3").on_hover_text("Low CPU, larger blobs; good during broad sync.").clicked() {
                                self.bar_zstd_level = typhoon_engine::core::cache::set_bar_zstd_level(3);
                                storage_save_after = true;
                            }
                            if ui.small_button("Balanced 9").on_hover_text("Middle ground between CPU and disk size.").clicked() {
                                self.bar_zstd_level = typhoon_engine::core::cache::set_bar_zstd_level(9);
                                storage_save_after = true;
                            }
                            if ui.small_button("Max 22").on_hover_text("Smallest blobs, highest write CPU. Use with care during broad sync.").clicked() {
                                self.bar_zstd_level = typhoon_engine::core::cache::set_bar_zstd_level(22);
                                storage_save_after = true;
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.button(egui::RichText::new(format!("Compact (zstd-{})", auto_compact::TARGET_LEVEL)).small()).clicked() {
                                let db_path = cache_db_path();
                                let log_tx = self.broker_tx.clone();
                                let size_before = size;
                                let _ = log_tx.send(BrokerCmd::CompactStorage { db_path: db_path.clone(), level: auto_compact::TARGET_LEVEL });
                                self.auto_compact_in_progress = true;
                                self.auto_compact_started_ms = chrono::Utc::now().timestamp_millis();
                                self.log.push_back(LogEntry::info(format!(
                                    "Compacting cache at zstd-{} (current: {:.1} MB)... this may take several minutes",
                                    auto_compact::TARGET_LEVEL,
                                    size_before as f64 / 1024.0 / 1024.0
                                )));
                            }
                            ui.label(egui::RichText::new("Recompress all data at max level. No impact on load speed.").color(AXIS_TEXT).small());
                        });
                        // Auto-compact controls + readout (ADR-089). Manual button above always works
                        // regardless of this setting.
                        ui.horizontal(|ui| {
                            let auto_label = format!(
                                "Auto-compact ({})",
                                auto_compact::schedule_summary(self.auto_compact_schedule)
                            );
                            if ui
                                .checkbox(
                                    &mut self.auto_compact_enabled,
                                    egui::RichText::new(auto_label).small(),
                                )
                                .on_hover_text(
                                    "Promote below-target bar-cache entries to zstd-22 during the configured AC + idle window.",
                                )
                                .changed()
                            {
                                storage_save_after = true;
                            }
                        });
                        ui.horizontal(|ui| {
                            let mut schedule = self.auto_compact_schedule.sanitized();
                            let mut changed = false;
                            ui.label(egui::RichText::new("Cadence").color(AXIS_TEXT).small());
                            let mut preset =
                                auto_compact::CadencePreset::from_days(schedule.cadence_days);
                            let preset_before = preset;
                            egui::ComboBox::from_id_salt("auto_compact_cadence_preset")
                                .selected_text(preset.label())
                                .show_ui(ui, |ui| {
                                    for option in [
                                        auto_compact::CadencePreset::Daily,
                                        auto_compact::CadencePreset::Weekly,
                                        auto_compact::CadencePreset::Monthly,
                                        auto_compact::CadencePreset::Yearly,
                                        auto_compact::CadencePreset::Custom,
                                    ] {
                                        ui.selectable_value(&mut preset, option, option.label());
                                    }
                                });
                            if preset != preset_before {
                                let new_days = preset.to_days(schedule.cadence_days);
                                if new_days != schedule.cadence_days {
                                    schedule.cadence_days = new_days;
                                    changed = true;
                                }
                            }
                            ui.label(egui::RichText::new("Every").color(AXIS_TEXT).small());
                            changed |= ui
                                .add(egui::DragValue::new(&mut schedule.cadence_days).range(1..=365).suffix("d"))
                                .changed();
                            // Sub-weekly cadences ignore the weekday gate — hide the picker
                            // so the UI matches what evaluate_gate actually checks.
                            if schedule.cadence_days >= 7 {
                                egui::ComboBox::from_id_salt("auto_compact_weekday")
                                    .selected_text(auto_compact::weekday_label(schedule.window_weekday))
                                    .show_ui(ui, |ui| {
                                        for day in 0..=6 {
                                            changed |= ui
                                                .selectable_value(
                                                    &mut schedule.window_weekday,
                                                    day,
                                                    auto_compact::weekday_label(day),
                                                )
                                                .changed();
                                        }
                                    });
                            }
                            ui.label(egui::RichText::new("Start").color(AXIS_TEXT).small());
                            changed |= ui
                                .add(egui::DragValue::new(&mut schedule.window_hour_start).range(0..=23).suffix(":00"))
                                .changed();
                            ui.label(egui::RichText::new("End").color(AXIS_TEXT).small());
                            changed |= ui
                                .add(egui::DragValue::new(&mut schedule.window_hour_end).range(1..=24).suffix(":00"))
                                .changed();
                            ui.label(egui::RichText::new("Min rows").color(AXIS_TEXT).small());
                            changed |= ui
                                .add(egui::DragValue::new(&mut schedule.uncompacted_threshold).range(1..=1_000_000))
                                .changed();
                            if changed {
                                self.auto_compact_schedule = schedule.sanitized();
                                storage_save_after = true;
                            }
                        });
                        ui.horizontal(|ui| {
                            let now_ms = chrono::Utc::now().timestamp_millis();
                            let last_label = if self.auto_compact_last_run_ms <= 0 {
                                "never".to_string()
                            } else {
                                let secs = ((now_ms - self.auto_compact_last_run_ms) / 1000).max(0);
                                if secs < 3600 {
                                    format!("{}m ago", secs / 60)
                                } else if secs < 86_400 {
                                    format!("{}h ago", secs / 3600)
                                } else {
                                    format!("{}d ago", secs / 86_400)
                                }
                            };
                            ui.label(
                                egui::RichText::new(format!("last: {}", last_label))
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                            let next_ms = auto_compact::next_eligible_time_ms(
                                self.auto_compact_schedule,
                                self.auto_compact_last_run_ms,
                            );
                            let next_label = if next_ms <= now_ms + 60_000 {
                                "now".to_string()
                            } else {
                                chrono::DateTime::<chrono::Utc>::from_timestamp_millis(next_ms)
                                    .map(|dt| {
                                        dt.with_timezone(&chrono::Local)
                                            .format("%a %H:%M")
                                            .to_string()
                                    })
                                    .unwrap_or_else(|| "unknown".to_string())
                            };
                            ui.label(
                                egui::RichText::new(format!("next: {}", next_label))
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                            if let Some(reason) = self.auto_compact_last_skip.as_deref() {
                                ui.label(
                                    egui::RichText::new(format!("(skip: {})", reason))
                                        .color(AXIS_TEXT)
                                        .small(),
                                );
                            }
                            if self.auto_compact_in_progress {
                                ui.label(
                                    egui::RichText::new("running…")
                                        .color(egui::Color32::from_rgb(241, 196, 15))
                                        .small()
                                        .strong(),
                                );
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.button(egui::RichText::new("Reclaim Free Space").small()).clicked() {
                                if let Some(cache) = self.cache.clone() {
                                    let result = cache.reclaim_space();
                                    match result {
                                        Ok((before, after)) => self.log.push_back(LogEntry::info(format!(
                                            "Reclaimed SQLite free pages: {} -> {}",
                                            format_bytes_human(before),
                                            format_bytes_human(after)
                                        ))),
                                        Err(e) => self.log.push_back(LogEntry::err(format!(
                                            "Reclaim storage failed: {}",
                                            e
                                        ))),
                                    }
                                    self.refresh_storage_snapshot_after_action("reclaim");
                                }
                            }
                            ui.label(
                                egui::RichText::new(
                                    "Run WAL checkpoint + VACUUM after prior deletes to physically shrink the DB file.",
                                )
                                .color(AXIS_TEXT)
                                .small(),
                            );
                        });
                        // Purge All Bar Data
                        ui.horizontal(|ui| {
                            if self.storage_purge_bars_confirm {
                                ui.label(egui::RichText::new("This will delete ALL cached bar data. This is NOT reversible!").color(egui::Color32::from_rgb(231, 76, 60)).small());
                                if ui.button(egui::RichText::new("Yes, Delete All Bars").color(egui::Color32::from_rgb(231, 76, 60)).small()).clicked() {
                                    self.storage_purge_bars_confirm = false;
                                    if let Some(cache) = self.cache.clone() {
                                        let result = cache.delete_all_bars();
                                        match result {
                                            Ok(n) => {
                                                let size_now = cache
                                                    .stats()
                                                    .ok()
                                                    .map(|(_, _, bytes)| format_bytes_human(bytes))
                                                    .unwrap_or_else(|| "?".to_string());
                                                self.log.push_back(LogEntry::info(format!(
                                                    "Purged all bar data: {} entries deleted, DB now {}",
                                                    n, size_now
                                                )));
                                            }
                                            Err(e) => self.log.push_back(LogEntry::err(format!("Purge bars failed: {}", e))),
                                        }
                                        self.refresh_storage_snapshot_after_action("purge all bars");
                                    }
                                }
                                if ui.small_button(egui::RichText::new("Cancel").small()).clicked() {
                                    self.storage_purge_bars_confirm = false;
                                }
                            } else {
                                if ui.button(egui::RichText::new("Purge All Bar Data").color(egui::Color32::from_rgb(231, 76, 60)).small()).clicked() {
                                    self.storage_purge_bars_confirm = true;
                                    self.storage_purge_darwin_confirm = false;
                                    self.storage_purge_broker_confirm = None;
                                    self.storage_purge_timeframe_confirm = false;
                                    self.storage_purge_news_confirm = false;
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            let broker_label = |prefix: &str| match prefix {
                                "alpaca" => "Alpaca",
                                "tastytrade" => "Tastytrade",
                                "mt5" => "MT5",
                                _ => "Broker",
                            };
                            ui.label(
                                egui::RichText::new("Nuclear broker purge:")
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                            if let Some(prefix) = self.storage_purge_broker_confirm.clone() {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Delete all {} cache rows from storage?",
                                        broker_label(&prefix)
                                    ))
                                    .color(egui::Color32::from_rgb(231, 76, 60))
                                    .small(),
                                );
                                if ui
                                    .button(
                                        egui::RichText::new("Yes, Delete Broker")
                                            .color(egui::Color32::from_rgb(231, 76, 60))
                                            .small(),
                                    )
                                    .clicked()
                                {
                                    self.storage_purge_broker_confirm = None;
                                    if let Some(cache) = self.cache.clone() {
                                        let result = cache.delete_broker_data(&prefix);
                                        match result {
                                            Ok(n) => {
                                                let size_now = cache
                                                    .stats()
                                                    .ok()
                                                    .map(|(_, _, bytes)| format_bytes_human(bytes))
                                                    .unwrap_or_else(|| "?".to_string());
                                                self.log.push_back(LogEntry::info(format!(
                                                    "Purged {} cache data: {} rows deleted, DB now {}",
                                                    broker_label(&prefix),
                                                    n,
                                                    size_now
                                                )));
                                            }
                                            Err(e) => self.log.push_back(LogEntry::err(format!(
                                                "Purge {} failed: {}",
                                                broker_label(&prefix),
                                                e
                                            ))),
                                        }
                                        self.refresh_storage_snapshot_after_action("broker purge");
                                    }
                                }
                                if ui.small_button(egui::RichText::new("Cancel").small()).clicked() {
                                    self.storage_purge_broker_confirm = None;
                                }
                            } else {
                                for prefix in ["alpaca", "tastytrade", "mt5"] {
                                    if ui
                                        .button(
                                            egui::RichText::new(broker_label(prefix))
                                                .color(egui::Color32::from_rgb(231, 76, 60))
                                                .small(),
                                        )
                                        .clicked()
                                    {
                                        self.storage_purge_broker_confirm = Some(prefix.to_string());
                                        self.storage_purge_bars_confirm = false;
                                        self.storage_purge_darwin_confirm = false;
                                        self.storage_purge_timeframe_confirm = false;
                                        self.storage_purge_news_confirm = false;
                                    }
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Delete TF across all brokers:")
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                            egui::ComboBox::from_id_salt("storage_delete_timeframe")
                                .selected_text(sync_timeframe_short_label(&self.storage_delete_timeframe))
                                .show_ui(ui, |ui| {
                                    for (short, cache) in STANDARD_SYNC_TIMEFRAMES {
                                        ui.selectable_value(
                                            &mut self.storage_delete_timeframe,
                                            cache.to_string(),
                                            format!("{} ({})", short, cache),
                                        );
                                    }
                                });
                            if self.storage_purge_timeframe_confirm {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Delete every {} blob from storage?",
                                        sync_timeframe_short_label(&self.storage_delete_timeframe)
                                    ))
                                    .color(egui::Color32::from_rgb(231, 76, 60))
                                    .small(),
                                );
                                if ui
                                    .button(
                                        egui::RichText::new("Yes, Delete TF")
                                            .color(egui::Color32::from_rgb(231, 76, 60))
                                            .small(),
                                    )
                                    .clicked()
                                {
                                    self.storage_purge_timeframe_confirm = false;
                                    if let Some(cache) = self.cache.clone() {
                                        let result = cache.delete_timeframe(&self.storage_delete_timeframe);
                                        match result {
                                            Ok(n) => {
                                                let size_now = cache
                                                    .stats()
                                                    .ok()
                                                    .map(|(_, _, bytes)| format_bytes_human(bytes))
                                                    .unwrap_or_else(|| "?".to_string());
                                                self.log.push_back(LogEntry::info(format!(
                                                    "Purged {} bars across all brokers: {} entries deleted, DB now {}",
                                                    self.storage_delete_timeframe, n, size_now
                                                )));
                                            }
                                            Err(e) => self.log.push_back(LogEntry::err(format!(
                                                "Purge {} failed: {}",
                                                self.storage_delete_timeframe, e
                                            ))),
                                        }
                                        self.refresh_storage_snapshot_after_action("timeframe purge");
                                    }
                                }
                                if ui.small_button(egui::RichText::new("Cancel").small()).clicked() {
                                    self.storage_purge_timeframe_confirm = false;
                                }
                            } else if ui
                                .button(
                                    egui::RichText::new("Delete TF")
                                        .color(egui::Color32::from_rgb(231, 76, 60))
                                        .small(),
                                )
                                .clicked()
                            {
                                self.storage_purge_timeframe_confirm = true;
                                self.storage_purge_bars_confirm = false;
                                self.storage_purge_darwin_confirm = false;
                                self.storage_purge_broker_confirm = None;
                                self.storage_purge_news_confirm = false;
                            }
                        });
                        // ── News purge by age (slider with date notches) ──
                        // Manual tool only — there is no automatic news TTL
                        // (see ADR-107 + ADR-215). Articles persist
                        // indefinitely; this gives the user a way to
                        // reclaim space without writing SQL.
                        ui.horizontal(|ui| {
                            // Notches: 1w / 1m / 3m / 6m / 1y / 2y / 5y.
                            // Days, not seconds, so the cutoff is timezone
                            // independent and the labels read naturally.
                            const NEWS_PURGE_NOTCHES_DAYS: &[(i64, &str)] = &[
                                (7,    "7 days"),
                                (30,   "30 days"),
                                (90,   "90 days"),
                                (180,  "6 months"),
                                (365,  "1 year"),
                                (730,  "2 years"),
                                (1825, "5 years"),
                            ];
                            let idx = self
                                .storage_purge_news_age_idx
                                .min(NEWS_PURGE_NOTCHES_DAYS.len() - 1);
                            let (days, label) = NEWS_PURGE_NOTCHES_DAYS[idx];
                            let cutoff_ts =
                                chrono::Utc::now().timestamp() - days * 86_400;
                            let count = self
                                .cache
                                .as_ref()
                                .and_then(|c| c.connection().ok())
                                .and_then(|conn| {
                                    typhoon_engine::core::news::count_articles_older_than(
                                        &conn, cutoff_ts,
                                    )
                                    .ok()
                                })
                                .unwrap_or(0);
                            ui.label(
                                egui::RichText::new("Purge news older than:")
                                    .color(AXIS_TEXT)
                                    .small(),
                            );
                            let mut slider_idx = idx;
                            let slider = egui::Slider::new(
                                &mut slider_idx,
                                0..=(NEWS_PURGE_NOTCHES_DAYS.len() - 1),
                            )
                            .integer()
                            .show_value(false)
                            .custom_formatter(|n, _| {
                                let i = (n as usize)
                                    .min(NEWS_PURGE_NOTCHES_DAYS.len() - 1);
                                NEWS_PURGE_NOTCHES_DAYS[i].1.to_string()
                            });
                            if ui.add(slider).changed() {
                                self.storage_purge_news_age_idx = slider_idx;
                                // Cancel any pending confirm if the user is
                                // re-aiming the slider — they should
                                // explicitly re-confirm at the new cutoff.
                                self.storage_purge_news_confirm = false;
                            }
                            ui.label(
                                egui::RichText::new(format!(
                                    "({}) — {} articles affected",
                                    label, count
                                ))
                                .color(AXIS_TEXT)
                                .small(),
                            );
                        });
                        ui.horizontal(|ui| {
                            // Re-resolve count for the confirm line so the
                            // displayed N matches the in-flight slider
                            // value even on the confirmation frame.
                            const NEWS_PURGE_NOTCHES_DAYS: &[(i64, &str)] = &[
                                (7,    "7 days"),
                                (30,   "30 days"),
                                (90,   "90 days"),
                                (180,  "6 months"),
                                (365,  "1 year"),
                                (730,  "2 years"),
                                (1825, "5 years"),
                            ];
                            let idx = self
                                .storage_purge_news_age_idx
                                .min(NEWS_PURGE_NOTCHES_DAYS.len() - 1);
                            let (days, label) = NEWS_PURGE_NOTCHES_DAYS[idx];
                            let cutoff_ts =
                                chrono::Utc::now().timestamp() - days * 86_400;
                            if self.storage_purge_news_confirm {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Delete every news article older than {}? (irreversible)",
                                        label
                                    ))
                                    .color(egui::Color32::from_rgb(231, 76, 60))
                                    .small(),
                                );
                                if ui
                                    .button(
                                        egui::RichText::new("Yes, Purge News")
                                            .color(egui::Color32::from_rgb(231, 76, 60))
                                            .small(),
                                    )
                                    .clicked()
                                {
                                    self.storage_purge_news_confirm = false;
                                    if let Some(cache) = self.cache.clone() {
                                        if let Ok(conn) = cache.connection() {
                                            match typhoon_engine::core::news::purge_older_than(
                                                &conn, cutoff_ts,
                                            ) {
                                                Ok(n) => {
                                                    let size_now = cache
                                                        .stats()
                                                        .ok()
                                                        .map(|(_, _, bytes)| {
                                                            format_bytes_human(bytes)
                                                        })
                                                        .unwrap_or_else(|| "?".to_string());
                                                    self.log.push_back(LogEntry::info(format!(
                                                        "News purge: removed {} articles older than {}, DB now {}",
                                                        n, label, size_now
                                                    )));
                                                }
                                                Err(e) => self.log.push_back(LogEntry::err(
                                                    format!("News purge failed: {}", e),
                                                )),
                                            }
                                        }
                                        self.refresh_storage_snapshot_after_action(
                                            "news age purge",
                                        );
                                    }
                                }
                                if ui
                                    .small_button(egui::RichText::new("Cancel").small())
                                    .clicked()
                                {
                                    self.storage_purge_news_confirm = false;
                                }
                            } else if ui
                                .button(
                                    egui::RichText::new("Purge News")
                                        .color(egui::Color32::from_rgb(231, 76, 60))
                                        .small(),
                                )
                                .clicked()
                            {
                                self.storage_purge_news_confirm = true;
                                self.storage_purge_bars_confirm = false;
                                self.storage_purge_darwin_confirm = false;
                                self.storage_purge_broker_confirm = None;
                                self.storage_purge_timeframe_confirm = false;
                            }
                        });
                        // Purge All DARWIN Data
                        ui.horizontal(|ui| {
                            if self.storage_purge_darwin_confirm {
                                ui.label(egui::RichText::new("This will delete ALL DARWIN accounts, deals, positions & equity. This is NOT reversible!").color(egui::Color32::from_rgb(231, 76, 60)).small());
                                if ui.button(egui::RichText::new("Yes, Delete All DARWIN Data").color(egui::Color32::from_rgb(231, 76, 60)).small()).clicked() {
                                    self.storage_purge_darwin_confirm = false;
                                    if let Some(cache) = self.cache.clone() {
                                        let result = cache.delete_all_darwin();
                                        match result {
                                            Ok(n) => {
                                                let size_now = cache
                                                    .stats()
                                                    .ok()
                                                    .map(|(_, _, bytes)| format_bytes_human(bytes))
                                                    .unwrap_or_else(|| "?".to_string());
                                                self.log.push_back(LogEntry::info(format!(
                                                    "Purged all DARWIN data: {} rows deleted, DB now {}",
                                                    n, size_now
                                                )));
                                            }
                                            Err(e) => self.log.push_back(LogEntry::err(format!("Purge DARWIN failed: {}", e))),
                                        }
                                        self.refresh_storage_snapshot_after_action("DARWIN purge");
                                    }
                                }
                                if ui.small_button(egui::RichText::new("Cancel").small()).clicked() {
                                    self.storage_purge_darwin_confirm = false;
                                }
                            } else {
                                if ui.button(egui::RichText::new("Purge All DARWIN Data").color(egui::Color32::from_rgb(231, 76, 60)).small()).clicked() {
                                    self.storage_purge_darwin_confirm = true;
                                    self.storage_purge_bars_confirm = false;
                                    self.storage_purge_broker_confirm = None;
                                    self.storage_purge_timeframe_confirm = false;
                                    self.storage_purge_news_confirm = false;
                                }
                            }
                        });
                    }
                    ui.separator();

                    // ─── Cache Location (NAS support) ──────────────────────
                    // Drain any in-flight VACUUM INTO result from the worker thread.
                    if let Some(rx) = &self.storage_cache_move_rx {
                        if let Ok(msg) = rx.try_recv() {
                            match msg {
                                Ok(s) => { self.storage_cache_move_result = Some((true, s.clone())); self.log.push_back(LogEntry::info(s)); }
                                Err(e) => { self.storage_cache_move_result = Some((false, e.clone())); self.log.push_back(LogEntry::err(e)); }
                            }
                            self.storage_cache_move_rx = None;
                        }
                    }
                    ui.label(egui::RichText::new("CACHE LOCATION").color(AXIS_TEXT).small().strong());
                    {
                        let default_dir = dirs_home().join("cache");
                        let active_dir = cache_dir();
                        let configured = read_custom_cache_dir();
                        let is_custom_missing = configured.as_ref().map(|p| !p.is_dir()).unwrap_or(false);
                        let is_custom_active = active_dir != default_dir;

                        if is_custom_missing {
                            let miss = configured.as_ref().unwrap();
                            ui.colored_label(egui::Color32::from_rgb(231, 76, 60),
                                egui::RichText::new(format!("⚠ Custom cache UNAVAILABLE: {}", miss.display())).small());
                            ui.label(egui::RichText::new(format!("Falling back to default: {}", active_dir.display())).small().color(AXIS_TEXT));
                            ui.label(egui::RichText::new("Mount the drive / restart the NAS, then restart the terminal.").small().color(AXIS_TEXT));
                        } else if is_custom_active {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Custom:").small().color(AXIS_TEXT));
                                ui.label(egui::RichText::new(active_dir.display().to_string()).small().monospace());
                            });
                        } else {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Default:").small().color(AXIS_TEXT));
                                ui.label(egui::RichText::new(active_dir.display().to_string()).small().monospace());
                            });
                        }

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("New path:").small());
                            ui.add(egui::TextEdit::singleline(&mut self.storage_cache_path_input)
                                .desired_width(420.0)
                                .hint_text("/mnt/nas/typhoon-cache"));
                        });

                        let in_flight = self.storage_cache_move_rx.is_some();
                        ui.horizontal(|ui| {
                            let trimmed = self.storage_cache_path_input.trim().to_string();
                            let enabled = !trimmed.is_empty() && !in_flight;

                            if ui.add_enabled(enabled, egui::Button::new(egui::RichText::new("Save location (restart required)").small()))
                                .on_hover_text("Write setting only. Next startup opens/creates cache at this location; old data stays put.")
                                .clicked()
                            {
                                let target = PathBuf::from(&trimmed);
                                match std::fs::create_dir_all(&target) {
                                    Ok(_) => match write_custom_cache_dir(Some(&target)) {
                                        Ok(_) => {
                                            self.storage_cache_move_result = Some((true, format!("Saved. Restart terminal to open cache at {}", target.display())));
                                            self.log.push_back(LogEntry::info(format!("Cache location saved: {} (restart required)", target.display())));
                                        }
                                        Err(e) => { self.storage_cache_move_result = Some((false, format!("Save failed: {}", e))); }
                                    },
                                    Err(e) => { self.storage_cache_move_result = Some((false, format!("mkdir {} failed: {}", target.display(), e))); }
                                }
                            }

                            if ui.add_enabled(enabled && self.cache.is_some(), egui::Button::new(egui::RichText::new("Copy cache here & save").small()))
                                .on_hover_text("Safely clone the open SQLite DB via VACUUM INTO, then save the setting. Restart required to start using the copy.")
                                .clicked()
                            {
                                let target = PathBuf::from(&trimmed);
                                let target_db = target.join("typhoon_cache.db");
                                let (tx, rx) = std::sync::mpsc::channel();
                                self.storage_cache_move_rx = Some(rx);
                                self.storage_cache_move_result = Some((true, format!("Copying cache to {} ... this may take several minutes for large caches", target.display())));
                                if let Some(cache) = self.cache.clone() {
                                    let tx_on_spawn_err = tx.clone();
                                    if let Err(e) = std::thread::Builder::new()
                                        .name("typhoon-cache-vacuum-copy".into())
                                        .spawn(move || {
                                            if let Err(e) = std::fs::create_dir_all(&target) {
                                                let _ = tx.send(Err(format!("mkdir {} failed: {}", target.display(), e)));
                                                return;
                                            }
                                            if target_db.exists() {
                                                let _ = tx.send(Err(format!("{} already exists — delete or pick a different dir", target_db.display())));
                                                return;
                                            }
                                            // VACUUM INTO is the SQLite-blessed way to snapshot a live DB.
                                            let dest = target_db.display().to_string().replace('\'', "''");
                                            let sql = format!("VACUUM INTO '{}'", dest);
                                            match cache.connection() {
                                                Ok(conn) => match conn.execute(&sql, []) {
                                                    Ok(_) => match write_custom_cache_dir(Some(&target)) {
                                                        Ok(_) => { let _ = tx.send(Ok(format!("Cache copied to {}. Restart terminal to use it.", target_db.display()))); }
                                                        Err(e) => { let _ = tx.send(Err(format!("Copy OK but save-setting failed: {}", e))); }
                                                    },
                                                    Err(e) => { let _ = tx.send(Err(format!("VACUUM INTO failed: {}", e))); }
                                                },
                                                Err(e) => { let _ = tx.send(Err(format!("Could not open cache connection: {}", e))); }
                                            }
                                        })
                                    {
                                        let _ = tx_on_spawn_err.send(Err(format!("Cache copy worker failed to start: {}", e)));
                                    }
                                }
                            }

                            if ui.add_enabled(!in_flight && read_custom_cache_dir().is_some(), egui::Button::new(egui::RichText::new("Reset to default").small()))
                                .on_hover_text("Clear the override. Next startup uses ~/.config/typhoon-terminal/cache/. Data at the custom location is NOT deleted.")
                                .clicked()
                            {
                                match write_custom_cache_dir(None) {
                                    Ok(_) => {
                                        self.storage_cache_move_result = Some((true, "Reset to default. Restart terminal to apply.".to_string()));
                                        self.log.push_back(LogEntry::info("Cache location reset to default (restart required)"));
                                    }
                                    Err(e) => { self.storage_cache_move_result = Some((false, format!("Reset failed: {}", e))); }
                                }
                            }
                        });

                        if in_flight {
                            ui.label(egui::RichText::new("Copy in progress... VACUUM INTO is running in background.").small().color(AXIS_TEXT));
                        }
                        if let Some((ok, msg)) = &self.storage_cache_move_result {
                            let color = if *ok { egui::Color32::from_rgb(26, 188, 156) } else { egui::Color32::from_rgb(231, 76, 60) };
                            ui.colored_label(color, egui::RichText::new(msg).small());
                        }
                    }
                    ui.separator();

                    self.render_storage_table(ui);
                });
            self.show_storage = show_storage;
            if storage_save_after {
                self.save_session();
            }
        }

        // Sync Status — per-(broker,TF) bar-sync health table, computed
        // from the BG bar_ts_cache on render (cheap: a few thousand keys
        // bucketed into ≤45 rows). Universe is every (symbol, TF) pair
        // the cache has ever seen for MT5 / Alpaca / Tastytrade /
        // Kraken; the three trader-facing brokers always
        // get a row even when their cache slice is empty, so "0%
        // Tastytrade" is visible before the first bar sync lands.
        self.render_sync_status_window(ctx);

        // LAN Sync
        if self.show_lan_sync {
            egui::Window::new("LAN Sync")
                .open(&mut self.show_lan_sync)
                .resizable(true).default_size([400.0, 250.0])
                .show(ctx, |ui| {
                    let is_idle = self.lan_sync_mode == "idle";

                    // Status indicator
                    let (status_text, status_color) = match self.lan_sync_mode.as_str() {
                        "server" => ("Server Running", UP),
                        "client" => ("Connected to Server", UP),
                        _ => ("Idle", AXIS_TEXT),
                    };
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("\u{25CF}").color(status_color));
                        ui.label(egui::RichText::new(status_text).color(status_color).strong());
                    });
                    ui.separator();

                    // Shared settings
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Port:").color(AXIS_TEXT).small());
                        ui.add(egui::TextEdit::singleline(&mut self.lan_sync_port).desired_width(60.0).font(egui::TextStyle::Monospace));
                        ui.label(egui::RichText::new("Passphrase:").color(AXIS_TEXT).small());
                        ui.add(egui::TextEdit::singleline(&mut self.lan_sync_passphrase).desired_width(120.0).password(true).hint_text("shared secret"));
                    });
                    ui.add_space(4.0);

                    if is_idle {
                        ui.horizontal(|ui| {
                            // ── Start Server ──
                            if ui.add(egui::Button::new(egui::RichText::new("Start Server").strong()).fill(BTN_GREEN).min_size(egui::vec2(120.0, 28.0))).clicked() {
                                let port: u16 = self.lan_sync_port.parse().unwrap_or(9847);
                                if self.lan_sync_passphrase.is_empty() {
                                    self.log.push_back(LogEntry::warn("Set a passphrase for LAN sync"));
                                } else {
                                    self.lan_sync_mode = "server".into();
                                    self.lan_server_enabled = true; // auto-start on next startup
                                    // Persist passphrase + server flag to keyring + KV cache
                                    let pass_clone = self.lan_sync_passphrase.clone();
                                    let cache_clone = self.cache.clone();
                                    self.rt_handle.spawn_blocking(move || {
                                        let _ = keyring::store(keyring::keys::LAN_SYNC_PASS, &pass_clone);
                                        if let Some(ref cache) = cache_clone {
                                            let _ = cache.put_kv(&format!("cred:{}", keyring::keys::LAN_SYNC_PASS), &pass_clone);
                                            let _ = cache.put_kv("lan:server_enabled", "true");
                                        }
                                    });
                                    let db_path = cache_db_path();
                                    let _ = self.broker_tx.send(BrokerCmd::LanSyncStart { port, passphrase: self.lan_sync_passphrase.clone(), db_path });
                                    self.log.push_back(LogEntry::info(format!("LAN sync server starting on wss://0.0.0.0:{} (TLS encrypted)", port)));
                                }
                            }
                        });
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);

                        // ── Connect to Server ──
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Server IP:").color(AXIS_TEXT).small());
                            ui.add(egui::TextEdit::singleline(&mut self.lan_sync_host).desired_width(140.0).hint_text("192.168.1.100").font(egui::TextStyle::Monospace));
                            if ui.add(egui::Button::new(egui::RichText::new("Connect").strong()).fill(BTN_BLUE).min_size(egui::vec2(90.0, 28.0))).clicked() {
                                if self.lan_sync_host.is_empty() || self.lan_sync_passphrase.is_empty() {
                                    self.log.push_back(LogEntry::warn("Enter server IP and passphrase"));
                                } else {
                                    let port: u16 = self.lan_sync_port.parse().unwrap_or(9847);
                                    self.lan_sync_mode = "client".into();
                                    // Save for auto-reconnect on next startup
                                    self.lan_client_enabled = true;
                                    self.lan_server_ip = self.lan_sync_host.clone();
                                    // Persist passphrase + server IP to keyring AND KV cache
                                    // (survives crashes where session.json doesn't get written)
                                    let pass_clone = self.lan_sync_passphrase.clone();
                                    let ip_clone = self.lan_sync_host.clone();
                                    let port_clone = self.lan_sync_port.clone();
                                    let cache_clone = self.cache.clone();
                                    self.rt_handle.spawn_blocking(move || {
                                        let _ = keyring::store(keyring::keys::LAN_SYNC_PASS, &pass_clone);
                                        if let Some(ref cache) = cache_clone {
                                            let _ = cache.put_kv(&format!("cred:{}", keyring::keys::LAN_SYNC_PASS), &pass_clone);
                                            let _ = cache.put_kv("lan:server_ip", &ip_clone);
                                            let _ = cache.put_kv("lan:sync_port", &port_clone);
                                            let _ = cache.put_kv("lan:client_enabled", "true");
                                        }
                                    });
                                    let db_path = cache_db_path();
                                    let _ = self.broker_tx.send(BrokerCmd::LanSyncConnect { host: self.lan_sync_host.clone(), port, passphrase: self.lan_sync_passphrase.clone(), db_path });
                                    self.log.push_back(LogEntry::info(format!("LAN client mode enabled — auto-connect to {}:{} on startup", self.lan_sync_host, port)));
                                }
                            }
                        });
                    } else {
                        // ── Active connection — show stats + stop button ──
                        ui.add_space(4.0);
                        if self.lan_sync_mode == "server" {
                            ui.label(egui::RichText::new("Serving to LAN clients: MT5 bars, Alpaca positions/orders, DARWIN analytics, crypto backfill, fundamentals, SEC filings, news, FRED data.").color(AXIS_TEXT).small());
                            ui.label(egui::RichText::new("Clients connect using this machine's IP address.").color(AXIS_TEXT).small());
                            // Connected clients list
                            if let Some(ref cache) = self.cache {
                                if let Ok(Some(json)) = cache.get_kv("lan:server:clients") {
                                    if let Ok(ips) = serde_json::from_str::<Vec<String>>(&json) {
                                        if ips.is_empty() {
                                            ui.label(egui::RichText::new("No clients connected").color(AXIS_TEXT).small());
                                        } else {
                                            ui.add_space(4.0);
                                            ui.label(egui::RichText::new(format!("Connected clients ({})", ips.len())).small().strong());
                                            for ip in &ips {
                                                ui.horizontal(|ui| {
                                                    ui.label(egui::RichText::new("\u{25CF}").color(UP).small());
                                                    ui.label(egui::RichText::new(ip).color(egui::Color32::from_rgb(26, 188, 156)).small().monospace());
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            ui.label(egui::RichText::new(format!("Syncing from {} — read-only view of server data", self.lan_sync_host)).color(AXIS_TEXT).small());
                            ui.label(egui::RichText::new("Receiving: MT5 bars, Alpaca positions/orders, DARWIN analytics, crypto, fundamentals, SEC, news, FRED").color(AXIS_TEXT).small());
                            // Sync status: local vs remote
                            if let Some((bar_count, kv_count, file_size)) = self.bg.cache_stats {
                                ui.label(egui::RichText::new(format!(
                                    "Local: {} bars | {} KV | {:.1} MB",
                                    bar_count, kv_count, file_size as f64 / 1024.0 / 1024.0
                                )).color(AXIS_TEXT).small());
                            }
                            ui.add_space(4.0);
                            // Resync buttons
                            ui.horizontal(|ui| {
                                if ui.button(egui::RichText::new("Resync Bars").small()).clicked() {
                                    let _ = self.broker_tx.send(BrokerCmd::LanResyncBars);
                                    self.log.push_back(LogEntry::info("Requesting bar resync from LAN server..."));
                                }
                                if ui.button(egui::RichText::new("Resync DARWIN Analytics").small()).clicked() {
                                    let _ = self.broker_tx.send(BrokerCmd::LanResyncDarwin);
                                    self.log.push_back(LogEntry::info("Requesting DARWIN analytics resync from LAN server..."));
                                }
                                if ui.button(egui::RichText::new("Resync Positions").small()).clicked() {
                                    // Force reload of positions from KV cache immediately
                                    if let Some(ref cache) = self.cache {
                                        if let Ok(Some(json)) = cache.get_kv("broker:positions") {
                                            if let Ok(pos) = serde_json::from_str::<Vec<PositionInfo>>(&json) {
                                                self.live_positions = pos;
                                            }
                                        }
                                        if let Ok(Some(json)) = cache.get_kv("darwin:open_positions") {
                                            if let Ok(pos) = serde_json::from_str::<Vec<darwin::PortfolioOpenPosition>>(&json) {
                                                self.bg.open_positions = pos;
                                            }
                                        }
                                    }
                                    self.log.push_back(LogEntry::info("Positions reloaded from LAN server cache"));
                                }
                            });
                        }
                        ui.add_space(8.0);
                        if ui.add(egui::Button::new(egui::RichText::new("Stop").strong()).fill(egui::Color32::from_rgb(180, 40, 40)).min_size(egui::vec2(80.0, 28.0))).clicked() {
                            self.lan_sync_mode = "idle".into();
                            self.lan_client_enabled = false;
                            self.lan_server_enabled = false;
                            let _ = self.broker_tx.send(BrokerCmd::LanSyncStop);
                            // Clear KV persistence
                            if let Some(ref cache) = self.cache {
                                let _ = cache.put_kv("lan:server_enabled", "false");
                                let _ = cache.put_kv("lan:client_enabled", "false");
                            }
                            self.log.push_back(LogEntry::info("LAN sync stopped"));
                        }
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.label(egui::RichText::new("Transport: TLS encrypted (wss://) with ephemeral self-signed certificate.").color(egui::Color32::from_rgb(80, 80, 100)).small());
                    ui.label(egui::RichText::new("Auth: PBKDF2-HMAC-SHA256 challenge-response (100K iterations).").color(egui::Color32::from_rgb(80, 80, 100)).small());
                });
        }

        // Object List (drawing management, like MT5 Object List)
        if self.show_object_list {
            let mut delete_idx: Option<usize> = None;
            egui::Window::new("Object List")
                .open(&mut self.show_object_list)
                .resizable(true)
                .default_size([400.0, 300.0])
                .show(ctx, |ui| {
                    if let Some(chart) = self.charts.get(self.active_tab) {
                        if chart.drawings.is_empty() {
                            ui.label("No drawings on this chart.");
                        } else {
                            ui.label(
                                egui::RichText::new(format!("{} drawings", chart.drawings.len()))
                                    .small()
                                    .color(AXIS_TEXT),
                            );
                            ui.separator();
                            egui::ScrollArea::vertical()
                                .auto_shrink(false)
                                .max_height(250.0)
                                .show(ui, |ui| {
                                    egui::Grid::new("object_list_grid").striped(true).show(
                                        ui,
                                        |ui| {
                                            ui.label(egui::RichText::new("#").small().strong());
                                            ui.label(egui::RichText::new("Type").small().strong());
                                            ui.label(
                                                egui::RichText::new("Details").small().strong(),
                                            );
                                            ui.label(egui::RichText::new("").small());
                                            ui.end_row();
                                            for (idx, drawing) in chart.drawings.iter().enumerate()
                                            {
                                                ui.label(
                                                    egui::RichText::new(format!("{}", idx + 1))
                                                        .small(),
                                                );
                                                let (type_name, details) = match drawing {
                                                    Drawing::HLine { price, .. } => {
                                                        ("H-Line", format!("{:.5}", price))
                                                    }
                                                    Drawing::VLine { bar_idx, .. } => {
                                                        ("V-Line", format!("bar {}", bar_idx))
                                                    }
                                                    Drawing::TrendLine { p1, p2, .. } => (
                                                        "Trendline",
                                                        format!("{:.4}→{:.4}", p1.1, p2.1),
                                                    ),
                                                    Drawing::FiboRetrace { high, low, .. } => (
                                                        "Fib Retrace",
                                                        format!("{:.4}–{:.4}", high, low),
                                                    ),
                                                    Drawing::Rectangle { .. } => {
                                                        ("Rectangle", String::new())
                                                    }
                                                    Drawing::Ray { origin, .. } => {
                                                        ("Ray", format!("{:.4}", origin.1))
                                                    }
                                                    Drawing::Channel { .. } => {
                                                        ("Channel", String::new())
                                                    }
                                                    Drawing::ExtendedLine { .. } => {
                                                        ("Ext Line", String::new())
                                                    }
                                                    Drawing::HRay { price, .. } => {
                                                        ("H-Ray", format!("{:.5}", price))
                                                    }
                                                    Drawing::CrossLine { price, .. } => {
                                                        ("Cross", format!("{:.5}", price))
                                                    }
                                                    Drawing::ArrowLine { .. } => {
                                                        ("Arrow", String::new())
                                                    }
                                                    Drawing::InfoLine { p1, p2, .. } => (
                                                        "Info Line",
                                                        format!("{:.4}→{:.4}", p1.1, p2.1),
                                                    ),
                                                    Drawing::Pitchfork { .. } => {
                                                        ("Pitchfork", String::new())
                                                    }
                                                    Drawing::FiboExtension { .. } => {
                                                        ("Fib Extension", String::new())
                                                    }
                                                    Drawing::GannFan { .. } => {
                                                        ("Gann Fan", String::new())
                                                    }
                                                    Drawing::LongPosition {
                                                        entry,
                                                        stop,
                                                        target,
                                                    } => (
                                                        "Long Pos",
                                                        format!(
                                                            "E:{:.4} S:{:.4} T:{:.4}",
                                                            entry.1, stop, target
                                                        ),
                                                    ),
                                                    Drawing::ShortPosition {
                                                        entry,
                                                        stop,
                                                        target,
                                                    } => (
                                                        "Short Pos",
                                                        format!(
                                                            "E:{:.4} S:{:.4} T:{:.4}",
                                                            entry.1, stop, target
                                                        ),
                                                    ),
                                                    Drawing::PriceRange { .. } => {
                                                        ("Price Range", String::new())
                                                    }
                                                    Drawing::TextLabel { text, .. } => {
                                                        ("Text", text.clone())
                                                    }
                                                    Drawing::ArrowMarker { is_up, .. } => (
                                                        if *is_up {
                                                            "Arrow Up"
                                                        } else {
                                                            "Arrow Down"
                                                        },
                                                        String::new(),
                                                    ),
                                                    Drawing::Ellipse { .. } => {
                                                        ("Ellipse", String::new())
                                                    }
                                                    Drawing::Triangle { .. } => {
                                                        ("Triangle", String::new())
                                                    }
                                                    Drawing::TrendAngle { .. } => {
                                                        ("Trend Angle", String::new())
                                                    }
                                                    Drawing::ParallelChannel { .. } => {
                                                        ("Parallel Ch", String::new())
                                                    }
                                                    Drawing::FibChannel { .. } => {
                                                        ("Fib Channel", String::new())
                                                    }
                                                    Drawing::FibTimeZones { bar_idx, .. } => {
                                                        ("Fib Time", format!("bar {}", bar_idx))
                                                    }
                                                    Drawing::PriceLabel { price, .. } => {
                                                        ("Price Label", format!("{:.5}", price))
                                                    }
                                                    Drawing::Callout { text, .. } => {
                                                        ("Callout", text.clone())
                                                    }
                                                    Drawing::Highlighter { .. } => {
                                                        ("Highlighter", String::new())
                                                    }
                                                    Drawing::CrossMarker { price, .. } => {
                                                        ("Cross", format!("{:.5}", price))
                                                    }
                                                    Drawing::Polyline { points, .. } => (
                                                        "Polyline",
                                                        format!("{} pts", points.len()),
                                                    ),
                                                    Drawing::AnchorNote { text, .. } => {
                                                        ("Note", text.clone())
                                                    }
                                                    Drawing::RegressionChannel { .. } => {
                                                        ("Regression", String::new())
                                                    }
                                                    Drawing::GannBox { .. } => {
                                                        ("Gann Box", String::new())
                                                    }
                                                    Drawing::ElliottWave { points, .. } => (
                                                        "Elliott Wave",
                                                        format!("{} pts", points.len()),
                                                    ),
                                                    Drawing::AbcCorrection { .. } => {
                                                        ("ABC Correction", String::new())
                                                    }
                                                    Drawing::DateRange { p1, p2, .. } => (
                                                        "Date Range",
                                                        format!(
                                                            "{} bars",
                                                            if p2.0 > p1.0 {
                                                                p2.0 - p1.0
                                                            } else {
                                                                p1.0 - p2.0
                                                            }
                                                        ),
                                                    ),
                                                    Drawing::DatePriceRange { p1, p2, .. } => (
                                                        "Date+Price",
                                                        format!(
                                                            "{} bars",
                                                            if p2.0 > p1.0 {
                                                                p2.0 - p1.0
                                                            } else {
                                                                p1.0 - p2.0
                                                            }
                                                        ),
                                                    ),
                                                    Drawing::HeadShoulders { .. } => {
                                                        ("H&S Pattern", String::new())
                                                    }
                                                    Drawing::XabcdPattern { .. } => {
                                                        ("XABCD", String::new())
                                                    }
                                                    Drawing::Brush { points, .. } => {
                                                        ("Brush", format!("{} pts", points.len()))
                                                    }
                                                    Drawing::SchiffPitchfork { .. } => {
                                                        ("Schiff Fork", String::new())
                                                    }
                                                    Drawing::ModSchiffPitchfork { .. } => {
                                                        ("Mod Schiff", String::new())
                                                    }
                                                    Drawing::CyclicLines {
                                                        bar_start,
                                                        bar_end,
                                                        ..
                                                    } => (
                                                        "Cyclic Lines",
                                                        format!(
                                                            "{} interval",
                                                            if *bar_end > *bar_start {
                                                                bar_end - bar_start
                                                            } else {
                                                                1
                                                            }
                                                        ),
                                                    ),
                                                    Drawing::SineWave { .. } => {
                                                        ("Sine Wave", String::new())
                                                    }
                                                    Drawing::Emoji { emoji, .. } => {
                                                        ("Emoji", emoji.clone())
                                                    }
                                                    Drawing::Flag { .. } => ("Flag", String::new()),
                                                    Drawing::Balloon { text, .. } => {
                                                        ("Balloon", text.clone())
                                                    }
                                                    Drawing::SessionBreak { bar_idx, .. } => (
                                                        "Session Break",
                                                        format!("bar {}", bar_idx),
                                                    ),
                                                    Drawing::MagnetLevel { price, .. } => {
                                                        ("Magnet Level", format!("{:.5}", price))
                                                    }
                                                    Drawing::RiskRewardBox {
                                                        entry,
                                                        stop,
                                                        target,
                                                    } => (
                                                        "R:R Box",
                                                        format!(
                                                            "E:{:.4} S:{:.4} T:{:.4}",
                                                            entry.1, stop, target
                                                        ),
                                                    ),
                                                    Drawing::FibCircle { .. } => {
                                                        ("Fib Circle", String::new())
                                                    }
                                                    Drawing::ArcDraw { .. } => {
                                                        ("Arc", String::new())
                                                    }
                                                    Drawing::CurveDraw { .. } => {
                                                        ("Curve", String::new())
                                                    }
                                                    Drawing::PathDraw { points, .. } => {
                                                        ("Path", format!("{} pts", points.len()))
                                                    }
                                                    Drawing::Forecast { .. } => {
                                                        ("Forecast", String::new())
                                                    }
                                                    Drawing::GhostFeed { p1, p2, .. } => (
                                                        "Ghost Feed",
                                                        format!(
                                                            "{} bars",
                                                            if p2.0 > p1.0 {
                                                                p2.0 - p1.0
                                                            } else {
                                                                p1.0 - p2.0
                                                            }
                                                        ),
                                                    ),
                                                    Drawing::Signpost { .. } => {
                                                        ("Signpost", String::new())
                                                    }
                                                    Drawing::Ruler { p1, p2, .. } => {
                                                        ("Ruler", format!("{:.4}", p2.1 - p1.1))
                                                    }
                                                    Drawing::TimeCycle {
                                                        bar_start,
                                                        bar_end,
                                                        ..
                                                    } => (
                                                        "Time Cycle",
                                                        format!(
                                                            "{} interval",
                                                            if *bar_end > *bar_start {
                                                                bar_end - bar_start
                                                            } else {
                                                                1
                                                            }
                                                        ),
                                                    ),
                                                    Drawing::SpeedResistanceFan { .. } => {
                                                        ("Speed Fan", String::new())
                                                    }
                                                    Drawing::SpeedResistanceArc { .. } => {
                                                        ("Speed Arc", String::new())
                                                    }
                                                    Drawing::FibSpiral { .. } => {
                                                        ("Fib Spiral", String::new())
                                                    }
                                                    Drawing::RotatedRectangle { .. } => {
                                                        ("Rotated Rect", String::new())
                                                    }
                                                    Drawing::AnchoredVwapLine {
                                                        bar_idx, ..
                                                    } => ("aVWAP", format!("bar {}", bar_idx)),
                                                    Drawing::TrendChannel { .. } => {
                                                        ("Trend Channel", String::new())
                                                    }
                                                    Drawing::InsidePitchfork { .. } => {
                                                        ("Inside Pitchfork", String::new())
                                                    }
                                                    Drawing::FibWedge { .. } => {
                                                        ("Fib Wedge", String::new())
                                                    }
                                                    Drawing::PriceNote { price, text, .. } => (
                                                        "Price Note",
                                                        format!("{:.4} {}", price, text),
                                                    ),
                                                    Drawing::MeasureTool { p1, p2, .. } => {
                                                        ("Measure", format!("{:.4}", p2.1 - p1.1))
                                                    }
                                                    Drawing::AnchoredText { text, .. } => {
                                                        ("Anchored Text", text.clone())
                                                    }
                                                    Drawing::Comment { text, .. } => {
                                                        ("Comment", text.clone())
                                                    }
                                                    Drawing::ArrowMarkerLeft { .. } => {
                                                        ("Arrow Left", String::new())
                                                    }
                                                    Drawing::ArrowMarkerRight { .. } => {
                                                        ("Arrow Right", String::new())
                                                    }
                                                    Drawing::Circle { .. } => {
                                                        ("Circle", String::new())
                                                    }
                                                    Drawing::PitchFan { .. } => {
                                                        ("Pitch Fan", String::new())
                                                    }
                                                    Drawing::TrendFibTime { .. } => {
                                                        ("Trend Fib Time", String::new())
                                                    }
                                                    Drawing::GannSquare { .. } => {
                                                        ("Gann Square", String::new())
                                                    }
                                                    Drawing::GannSquareFixed { .. } => {
                                                        ("Gann Square Fixed", String::new())
                                                    }
                                                    Drawing::BarsPattern { .. } => {
                                                        ("Bars Pattern", String::new())
                                                    }
                                                    Drawing::Projection { .. } => {
                                                        ("Projection", String::new())
                                                    }
                                                    Drawing::DoubleCurve { .. } => {
                                                        ("Double Curve", String::new())
                                                    }
                                                    Drawing::TrianglePattern { .. } => {
                                                        ("Triangle Pattern", String::new())
                                                    }
                                                    Drawing::ThreeDrives { .. } => {
                                                        ("Three Drives", String::new())
                                                    }
                                                    Drawing::ElliottDouble { .. } => {
                                                        ("Elliott WXY", String::new())
                                                    }
                                                    Drawing::AbcdPattern { .. } => {
                                                        ("ABCD", String::new())
                                                    }
                                                    Drawing::CypherPattern { .. } => {
                                                        ("Cypher", String::new())
                                                    }
                                                    Drawing::ElliottTriangle { .. } => {
                                                        ("Elliott ABCDE", String::new())
                                                    }
                                                    Drawing::ElliottTripleCombo { .. } => {
                                                        ("Elliott WXYXZ", String::new())
                                                    }
                                                };
                                                ui.label(egui::RichText::new(type_name).small());
                                                ui.label(
                                                    egui::RichText::new(details)
                                                        .small()
                                                        .color(AXIS_TEXT),
                                                );
                                                if ui.small_button("Del").clicked() {
                                                    delete_idx = Some(idx);
                                                }
                                                ui.end_row();
                                            }
                                        },
                                    );
                                });
                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.button("Clear All").clicked() {
                                    delete_idx = Some(usize::MAX); // sentinel for clear all
                                }
                            });
                        }
                    }
                });
            if let Some(idx) = delete_idx {
                if let Some(chart) = self.charts.get_mut(self.active_tab) {
                    if idx == usize::MAX {
                        chart.drawings.clear();
                    } else if idx < chart.drawings.len() {
                        chart.drawings.remove(idx);
                    }
                }
            }
        }

        // Help — keyboard shortcuts + quick command reference.
        // Searchable filter covers both sections.
        if self.show_help {
            egui::Window::new("Keyboard Shortcuts & Command Reference")
                .open(&mut self.show_help)
                .resizable(true)
                .default_size([720.0, 560.0])
                .max_size([720.0, 560.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Help");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.help_filter)
                                .hint_text("filter keys/commands…")
                                .desired_width(260.0),
                        );
                        if ui.small_button("Clear").clicked() {
                            self.help_filter.clear();
                        }
                    });
                    ui.separator();

                    let filter_lower = self.help_filter.to_lowercase();
                    let matches = |key: &str, desc: &str| -> bool {
                        filter_lower.is_empty()
                            || key.to_lowercase().contains(&filter_lower)
                            || desc.to_lowercase().contains(&filter_lower)
                    };

                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            // ── Chart navigation ──
                            ui.label(
                                egui::RichText::new("Chart navigation")
                                    .color(ACCENT)
                                    .strong(),
                            );
                            egui::Grid::new("help_nav")
                                .striped(true)
                                .num_columns(2)
                                .show(ui, |ui| {
                                    let nav: &[(&str, &str)] = &[
                                        ("Scroll wheel", "Zoom chart (horizontal)"),
                                        ("Ctrl + scroll", "Zoom chart (vertical / price)"),
                                        ("Double-click", "Reset zoom & pan"),
                                        ("Click + drag", "Pan chart"),
                                        ("← →", "Bar-by-bar scroll"),
                                        ("Home / End", "Jump to start / end"),
                                        ("PgUp / PgDn", "Half-screen scroll"),
                                        ("+ / -", "Zoom in / out"),
                                        ("Delete / Backspace", "Remove last drawing"),
                                        ("Right-click", "Context menu (drawings, chart type)"),
                                    ];
                                    for (k, d) in nav {
                                        if !matches(k, d) {
                                            continue;
                                        }
                                        ui.label(egui::RichText::new(*k).monospace());
                                        ui.label(*d);
                                        ui.end_row();
                                    }
                                });
                            ui.add_space(8.0);

                            // ── App / window management ──
                            ui.label(egui::RichText::new("App & window").color(ACCENT).strong());
                            egui::Grid::new("help_app")
                                .striped(true)
                                .num_columns(2)
                                .show(ui, |ui| {
                                    let app: &[(&str, &str)] = &[
                                        (
                                            "~ (tilde/backtick)",
                                            "Open command palette (Quake-style)",
                                        ),
                                        (
                                            "Esc",
                                            "Close palette / cancel drawing / close top window",
                                        ),
                                        ("Ctrl+N", "New chart tab"),
                                        ("Ctrl+W", "Close current tab"),
                                        ("Ctrl+Tab", "Next tab"),
                                        ("Ctrl+Shift+Tab", "Previous tab"),
                                        ("Alt+1..9", "Jump to timeframe 1..9"),
                                        ("F5", "Reload bars from cache"),
                                        ("F11", "Toggle fullscreen"),
                                        ("Alt+F4", "Quit"),
                                    ];
                                    for (k, d) in app {
                                        if !matches(k, d) {
                                            continue;
                                        }
                                        ui.label(egui::RichText::new(*k).monospace());
                                        ui.label(*d);
                                        ui.end_row();
                                    }
                                });
                            ui.add_space(8.0);

                            // ── Commands reference (auto-generated from COMMANDS registry) ──
                            // Skips the DRAW_* cluster — they're listed in their own section below.
                            ui.label(
                                egui::RichText::new(format!(
                                    "Command palette ({} commands)",
                                    COMMANDS
                                        .iter()
                                        .filter(|c| !c.name.starts_with("DRAW_"))
                                        .count()
                                ))
                                .color(ACCENT)
                                .strong(),
                            );
                            ui.label(
                                egui::RichText::new(
                                    "Press ~ then type. All commands are case-insensitive.",
                                )
                                .small()
                                .color(AXIS_TEXT),
                            );
                            egui::Grid::new("help_cmds")
                                .striped(true)
                                .num_columns(2)
                                .show(ui, |ui| {
                                    for cmd in COMMANDS {
                                        if cmd.name.starts_with("DRAW_") {
                                            continue;
                                        }
                                        if !matches(cmd.name, cmd.desc) {
                                            continue;
                                        }
                                        ui.label(
                                            egui::RichText::new(cmd.name)
                                                .monospace()
                                                .color(egui::Color32::from_rgb(150, 200, 255)),
                                        );
                                        ui.label(cmd.desc);
                                        ui.end_row();
                                    }
                                });
                            ui.add_space(8.0);

                            // ── Drawing tools (separate section) ──
                            ui.collapsing(
                                egui::RichText::new(format!(
                                    "Drawing tools ({} types)",
                                    COMMANDS
                                        .iter()
                                        .filter(|c| c.name.starts_with("DRAW_"))
                                        .count()
                                ))
                                .color(ACCENT)
                                .strong(),
                                |ui| {
                                    egui::Grid::new("help_draw")
                                        .striped(true)
                                        .num_columns(2)
                                        .show(ui, |ui| {
                                            for cmd in COMMANDS {
                                                if !cmd.name.starts_with("DRAW_") {
                                                    continue;
                                                }
                                                if !matches(cmd.name, cmd.desc) {
                                                    continue;
                                                }
                                                ui.label(
                                                    egui::RichText::new(cmd.name)
                                                        .monospace()
                                                        .color(egui::Color32::from_rgb(
                                                            150, 200, 255,
                                                        )),
                                                );
                                                ui.label(cmd.desc);
                                                ui.end_row();
                                            }
                                        });
                                },
                            );
                            ui.add_space(10.0);

                            // ── Status footer ──
                            ui.separator();
                            ui.label(egui::RichText::new("TyphooN Terminal").color(ACCENT));
                            let gpu_ind = if self.gpu_indicators.is_some() {
                                "GPU Indicators: Active"
                            } else {
                                "GPU Indicators: CPU fallback"
                            };
                            let gpu_dar = if self.gpu_darwin.is_some() {
                                "GPU DARWIN Analytics: Active"
                            } else {
                                "GPU DARWIN: CPU fallback"
                            };
                            ui.label(
                                egui::RichText::new(gpu_ind)
                                    .color(if self.gpu_indicators.is_some() {
                                        UP
                                    } else {
                                        DOWN
                                    })
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(gpu_dar)
                                    .color(if self.gpu_darwin.is_some() { UP } else { DOWN })
                                    .small(),
                            );
                        });
                });
        }

        // Data Window — all indicator values at crosshair position
        if self.show_data_window {
            egui::Window::new("Data Window")
                .open(&mut self.show_data_window)
                .resizable(true)
                .default_size([400.0, 500.0])
                .show(ctx, |ui| {
                    if let Some(chart) = self.charts.get(self.active_tab) {
                        let (si, ei) = chart.visible_range();
                        let bars = &chart.bars[si..ei];
                        if let Some(_pos) = self.crosshair {
                            // Find bar index from crosshair
                            if !bars.is_empty() {
                                let price_axis_w = 70.0_f32;
                                let _bar_w =
                                    (ui.available_width() + price_axis_w) / bars.len() as f32; // approximate
                                let _rel_idx = 0.max(bars.len() / 2); // fallback to middle if we can't calculate
                                // Use most recent bar as fallback
                                let abs_idx = ei.saturating_sub(1);
                                let b = &chart.bars[abs_idx];
                                ui.heading(format!(
                                    "{} [{}]",
                                    chart.symbol,
                                    chart.timeframe.label()
                                ));
                                ui.separator();
                                egui::Grid::new("data_grid")
                                    .striped(true)
                                    .num_columns(2)
                                    .show(ui, |ui| {
                                        ui.label("Open");
                                        ui.label(format_price(b.open));
                                        ui.end_row();
                                        ui.label("High");
                                        ui.label(format_price(b.high));
                                        ui.end_row();
                                        ui.label("Low");
                                        ui.label(format_price(b.low));
                                        ui.end_row();
                                        ui.label("Close");
                                        ui.label(format_price(b.close));
                                        ui.end_row();
                                        ui.label("Volume");
                                        ui.label(format!("{:.0}", b.volume));
                                        ui.end_row();
                                        ui.end_row();
                                        if let Some(Some(v)) = chart.sma200.get(abs_idx) {
                                            ui.label(
                                                egui::RichText::new("SMA200").color(SMA200_COL),
                                            );
                                            ui.label(format_price(*v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.sma100.get(abs_idx) {
                                            ui.label(
                                                egui::RichText::new("SMA100").color(SMA100_COL),
                                            );
                                            ui.label(format_price(*v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.ema21.get(abs_idx) {
                                            ui.label(egui::RichText::new("EMA21").color(EMA_COL));
                                            ui.label(format_price(*v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.kama.get(abs_idx) {
                                            ui.label(egui::RichText::new("KAMA").color(KAMA_COL));
                                            ui.label(format_price(*v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.wma.get(abs_idx) {
                                            ui.label(egui::RichText::new("WMA20").color(WMA_COL));
                                            ui.label(format_price(*v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.hma.get(abs_idx) {
                                            ui.label(egui::RichText::new("HMA20").color(HMA_COL));
                                            ui.label(format_price(*v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.bb_upper.get(abs_idx) {
                                            ui.label(egui::RichText::new("BB Upper").color(BB_COL));
                                            ui.label(format_price(*v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.bb_lower.get(abs_idx) {
                                            ui.label(egui::RichText::new("BB Lower").color(BB_COL));
                                            ui.label(format_price(*v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.rsi.get(abs_idx) {
                                            let rsi_col = if *v > 70.0 {
                                                DOWN
                                            } else if *v < 30.0 {
                                                UP
                                            } else {
                                                RSI_LINE
                                            };
                                            ui.label(egui::RichText::new("RSI").color(rsi_col));
                                            ui.label(
                                                egui::RichText::new(format!("{:.1}", v))
                                                    .color(rsi_col),
                                            );
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.fisher.get(abs_idx) {
                                            let f_col =
                                                if *v > 0.0 { FISHER_POS } else { FISHER_NEG };
                                            ui.label(egui::RichText::new("Fisher").color(f_col));
                                            ui.label(
                                                egui::RichText::new(format!("{:.3}", v))
                                                    .color(f_col),
                                            );
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.atr.get(abs_idx) {
                                            ui.label(egui::RichText::new("ATR").color(AXIS_TEXT));
                                            ui.label(
                                                egui::RichText::new(format_price(*v))
                                                    .color(AXIS_TEXT),
                                            );
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.macd_line.get(abs_idx) {
                                            ui.label(
                                                egui::RichText::new("MACD").color(MACD_LINE_COL),
                                            );
                                            ui.label(format!("{:.4}", v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.stoch_k.get(abs_idx) {
                                            ui.label(
                                                egui::RichText::new("Stoch %K").color(STOCH_K_COL),
                                            );
                                            ui.label(format!("{:.1}", v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.adx.get(abs_idx) {
                                            ui.label(egui::RichText::new("ADX").color(ADX_COL));
                                            ui.label(format!("{:.1}", v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.cci.get(abs_idx) {
                                            ui.label(egui::RichText::new("CCI").color(CCI_COL));
                                            ui.label(format!("{:.1}", v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.williams_r.get(abs_idx) {
                                            ui.label(egui::RichText::new("W%R").color(WILLR_COL));
                                            ui.label(format!("{:.1}", v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.momentum.get(abs_idx) {
                                            ui.label("Momentum");
                                            ui.label(format_price(*v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.mfi.get(abs_idx) {
                                            let col = if *v > 80.0 {
                                                DOWN
                                            } else if *v < 20.0 {
                                                UP
                                            } else {
                                                MFI_COL
                                            };
                                            ui.label(egui::RichText::new("MFI").color(col));
                                            ui.label(
                                                egui::RichText::new(format!("{:.1}", v)).color(col),
                                            );
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.trix_line.get(abs_idx) {
                                            ui.label(
                                                egui::RichText::new("TRIX").color(TRIX_LINE_COL),
                                            );
                                            ui.label(format!("{:+.4}", v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.ppo_line.get(abs_idx) {
                                            ui.label(
                                                egui::RichText::new("PPO").color(PPO_LINE_COL),
                                            );
                                            ui.label(format!("{:+.3}", v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.ultosc.get(abs_idx) {
                                            let col = if *v > 70.0 {
                                                DOWN
                                            } else if *v < 30.0 {
                                                UP
                                            } else {
                                                ULTOSC_COL
                                            };
                                            ui.label(egui::RichText::new("ULTOSC").color(col));
                                            ui.label(
                                                egui::RichText::new(format!("{:.1}", v)).color(col),
                                            );
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.stochrsi_k.get(abs_idx) {
                                            ui.label(
                                                egui::RichText::new("StochRSI %K")
                                                    .color(STOCH_K_COL),
                                            );
                                            ui.label(format!("{:.1}", v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.var_oscillator.get(abs_idx) {
                                            ui.label("VaR Osc");
                                            ui.label(format!("{:.1}", v));
                                            ui.end_row();
                                        }
                                        if let Some(Some(v)) = chart.psar.get(abs_idx) {
                                            ui.label(egui::RichText::new("P.SAR").color(SAR_COL));
                                            ui.label(format_price(*v));
                                            ui.end_row();
                                        }
                                    });
                            }
                        } else {
                            ui.label(
                                egui::RichText::new("Move cursor over chart").color(AXIS_TEXT),
                            );
                        }
                    }
                });
        }

        // Price Alerts
        if self.show_alerts {
            egui::Window::new("Price Alerts")
                .open(&mut self.show_alerts)
                .resizable(true)
                .default_size([500.0, 350.0])
                .show(ctx, |ui| {
                    ui.heading("Alerts");
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Price:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.alert_price_input)
                                .desired_width(100.0),
                        );
                        ui.label("Label:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.alert_label_input)
                                .desired_width(100.0),
                        );
                    });
                    if ui.button("Add Alert").clicked() {
                        if let Ok(price) = self.alert_price_input.parse::<f64>() {
                            let label = if self.alert_label_input.is_empty() {
                                format_price(price)
                            } else {
                                self.alert_label_input.clone()
                            };
                            self.alerts.push((price, label));
                            self.alert_price_input.clear();
                            self.alert_label_input.clear();
                            self.log.push_back(LogEntry::info(format!(
                                "Alert set at {}",
                                format_price(price)
                            )));
                        }
                    }
                    ui.separator();
                    if self.alerts.is_empty() {
                        ui.label(egui::RichText::new("No alerts set.").color(AXIS_TEXT));
                    } else {
                        let mut remove_idx: Option<usize> = None;
                        for (i, (price, label)) in self.alerts.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format_price(*price))
                                        .strong()
                                        .monospace(),
                                );
                                ui.label(label);
                                if ui.small_button("X").clicked() {
                                    remove_idx = Some(i);
                                }
                            });
                        }
                        if let Some(idx) = remove_idx {
                            self.alerts.remove(idx);
                        }
                        if ui.button("Clear All Alerts").clicked() {
                            self.alerts.clear();
                        }
                    }

                    // Check alerts against current price
                    if let Some(chart) = self.charts.get(self.active_tab) {
                        if let Some(last) = chart.bars.last() {
                            for (price, label) in &self.alerts {
                                let dist = (last.close - price).abs();
                                let pct = dist / last.close * 100.0;
                                if pct < 0.1 {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "ALERT TRIGGERED: {} at {}",
                                            label,
                                            format_price(*price)
                                        ))
                                        .color(egui::Color32::from_rgb(255, 80, 80))
                                        .strong(),
                                    );
                                }
                            }
                        }
                    }
                    // ── DARWIN Risk Alerts ──────────────────────
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("DARWIN Risk Alerts").strong());
                    ui.separator();
                    {
                        let alerts = &self.bg.darwin_alerts;
                        if alerts.is_empty() {
                            ui.label(egui::RichText::new("No risk alerts — all clear.").color(UP));
                        } else {
                            for alert in alerts {
                                let color = match alert.severity.as_str() {
                                    "CRITICAL" => DOWN,
                                    "WARNING" => egui::Color32::from_rgb(255, 200, 50),
                                    _ => AXIS_TEXT,
                                };
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("\u{2588}").color(color));
                                    ui.label(
                                        egui::RichText::new(&alert.severity)
                                            .color(color)
                                            .small()
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(&alert.alert_type).small().strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(&alert.message)
                                            .color(AXIS_TEXT)
                                            .small(),
                                    );
                                });
                            }
                        }
                    }
                });
        }

        // Fear & Greed Index window
        if self.show_fear_greed {
            egui::Window::new("Fear & Greed Index")
                .open(&mut self.show_fear_greed)
                .resizable(true)
                .default_size([340.0, 220.0])
                .show(ctx, |ui| {
                    ui.heading("Crypto Fear & Greed Index");
                    ui.separator();
                    if ui.button("Refresh").clicked() {
                        let _ = self.broker_tx.send(BrokerCmd::FetchFearGreed);
                    }
                    ui.add_space(8.0);

                    let val = self.fear_greed_value;
                    // Color based on value zone
                    let gauge_color = if val <= 25 {
                        egui::Color32::from_rgb(255, 50, 50) // Extreme Fear — red
                    } else if val <= 45 {
                        egui::Color32::from_rgb(255, 165, 0) // Fear — orange
                    } else if val <= 55 {
                        egui::Color32::from_rgb(255, 255, 80) // Neutral — yellow
                    } else if val <= 75 {
                        egui::Color32::from_rgb(144, 238, 100) // Greed — light green
                    } else {
                        egui::Color32::from_rgb(0, 200, 0) // Extreme Greed — green
                    };

                    // Large value display
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("{}", val))
                                .color(gauge_color)
                                .size(48.0)
                                .strong(),
                        );
                        ui.vertical(|ui| {
                            ui.add_space(12.0);
                            ui.label(
                                egui::RichText::new(&self.fear_greed_label)
                                    .color(gauge_color)
                                    .size(18.0),
                            );
                            ui.label(egui::RichText::new("/ 100").color(AXIS_TEXT).size(14.0));
                        });
                    });

                    ui.add_space(8.0);

                    // Gauge bar
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 24.0),
                        egui::Sense::hover(),
                    );
                    let painter = ui.painter_at(rect);
                    // Background
                    painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(40, 40, 40));
                    // Gradient zones
                    let w = rect.width();
                    let zone_colors = [
                        (0.0, 0.25, egui::Color32::from_rgb(255, 50, 50)),
                        (0.25, 0.45, egui::Color32::from_rgb(255, 165, 0)),
                        (0.45, 0.55, egui::Color32::from_rgb(255, 255, 80)),
                        (0.55, 0.75, egui::Color32::from_rgb(144, 238, 100)),
                        (0.75, 1.0, egui::Color32::from_rgb(0, 200, 0)),
                    ];
                    for (start, end, color) in &zone_colors {
                        let zone_rect = egui::Rect::from_min_max(
                            egui::pos2(rect.min.x + w * *start as f32, rect.min.y),
                            egui::pos2(rect.min.x + w * *end as f32, rect.max.y),
                        );
                        painter.rect_filled(zone_rect, 0.0, *color);
                    }
                    // Indicator needle
                    let needle_x = rect.min.x + w * (val as f32 / 100.0);
                    painter.line_segment(
                        [
                            egui::pos2(needle_x, rect.min.y - 2.0),
                            egui::pos2(needle_x, rect.max.y + 2.0),
                        ],
                        egui::Stroke::new(3.0, egui::Color32::WHITE),
                    );

                    ui.add_space(4.0);
                    // Zone labels
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Extreme Fear")
                                .color(egui::Color32::from_rgb(255, 50, 50))
                                .small(),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new("Fear")
                                .color(egui::Color32::from_rgb(255, 165, 0))
                                .small(),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new("Neutral")
                                .color(egui::Color32::from_rgb(255, 255, 80))
                                .small(),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new("Greed")
                                .color(egui::Color32::from_rgb(144, 238, 100))
                                .small(),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new("Extreme Greed")
                                .color(egui::Color32::from_rgb(0, 200, 0))
                                .small(),
                        );
                    });
                });
        }

        // World Indices Dashboard
        if self.show_world_indices {
            egui::Window::new("World Indices")
                .open(&mut self.show_world_indices)
                .resizable(true)
                .default_size([620.0, 480.0])
                .max_size([620.0, 560.0])
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("World Stock Indices & ETFs").strong());
                    if ui.small_button("Refresh").clicked() {
                        let symbols = vec![
                            "DIA", "SPY", "QQQ", "IWM", "EFA", "EEM", "VGK", "EWJ", "FXI", "EWZ",
                            "GLD", "SLV", "USO", "TLT", "UUP", "BTCUSD",
                        ]
                        .into_iter()
                        .map(String::from)
                        .collect();
                        let _ = self
                            .broker_tx
                            .send(BrokerCmd::GetWatchlistQuotes { symbols });
                    }
                    ui.separator();
                    if self.world_indices_data.is_empty() {
                        ui.label(
                            egui::RichText::new("Loading... (requires broker connection)")
                                .color(AXIS_TEXT),
                        );
                    } else {
                        let descs: std::collections::HashMap<&str, &str> = [
                            ("DIA", "DJIA"),
                            ("SPY", "S&P 500"),
                            ("QQQ", "NASDAQ-100"),
                            ("IWM", "Russell 2000"),
                            ("EFA", "EAFE Intl"),
                            ("EEM", "Emerging Mkts"),
                            ("VGK", "Europe"),
                            ("EWJ", "Japan"),
                            ("FXI", "China"),
                            ("EWZ", "Brazil"),
                            ("GLD", "Gold"),
                            ("SLV", "Silver"),
                            ("USO", "Oil"),
                            ("TLT", "20Y Bonds"),
                            ("UUP", "US Dollar"),
                            ("BTCUSD", "Bitcoin"),
                        ]
                        .iter()
                        .cloned()
                        .collect();
                        egui::ScrollArea::vertical()
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                egui::Grid::new("indices_grid")
                                    .striped(true)
                                    .num_columns(5)
                                    .min_col_width(80.0)
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new("Symbol")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Name")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Last")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Change")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Change%")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.end_row();
                                        for row in &self.world_indices_data {
                                            let desc = descs
                                                .get(row.symbol.to_uppercase().as_str())
                                                .unwrap_or(&"");
                                            let color = if row.change_pct > 0.0 {
                                                UP
                                            } else if row.change_pct < 0.0 {
                                                DOWN
                                            } else {
                                                AXIS_TEXT
                                            };
                                            ui.label(
                                                egui::RichText::new(&row.symbol)
                                                    .small()
                                                    .strong()
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(*desc).small().color(AXIS_TEXT),
                                            );
                                            ui.label(
                                                egui::RichText::new(format_price(row.last))
                                                    .small()
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{:+.2}", row.change))
                                                    .color(color)
                                                    .small()
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:+.2}%",
                                                    row.change_pct
                                                ))
                                                .color(color)
                                                .small()
                                                .strong()
                                                .monospace(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                });
        }

        // Crypto Top 50 (CoinGecko)
        if self.show_crypto_top50 {
            egui::Window::new("Crypto Top 50")
                .open(&mut self.show_crypto_top50)
                .resizable(true)
                .default_size([700.0, 550.0])
                .max_size([700.0, 560.0])
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("Top 50 Cryptocurrencies by Market Cap").strong());
                    if ui.small_button("Refresh").clicked() {
                        let _ = self.broker_tx.send(BrokerCmd::FetchCryptoTop50);
                    }
                    ui.separator();
                    if self.crypto_top50.is_empty() {
                        ui.label(egui::RichText::new("Loading from CoinGecko...").color(AXIS_TEXT));
                    } else {
                        egui::ScrollArea::vertical()
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                egui::Grid::new("crypto50_grid")
                                    .striped(true)
                                    .num_columns(5)
                                    .min_col_width(80.0)
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new("#")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Name")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Price")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("24h%")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Market Cap")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.end_row();
                                        for (i, (name, price, change, mcap)) in
                                            self.crypto_top50.iter().enumerate()
                                        {
                                            let color = if *change > 0.0 {
                                                UP
                                            } else if *change < 0.0 {
                                                DOWN
                                            } else {
                                                AXIS_TEXT
                                            };
                                            ui.label(
                                                egui::RichText::new(format!("{}", i + 1))
                                                    .small()
                                                    .monospace(),
                                            );
                                            ui.label(egui::RichText::new(name).small());
                                            let price_str = if *price >= 1.0 {
                                                format!("${:.2}", price)
                                            } else {
                                                format!("${:.6}", price)
                                            };
                                            ui.label(
                                                egui::RichText::new(price_str).small().monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("{:+.2}%", change))
                                                    .color(color)
                                                    .small()
                                                    .strong()
                                                    .monospace(),
                                            );
                                            let mcap_str = if *mcap >= 1e12 {
                                                format!("${:.1}T", mcap / 1e12)
                                            } else if *mcap >= 1e9 {
                                                format!("${:.1}B", mcap / 1e9)
                                            } else if *mcap >= 1e6 {
                                                format!("${:.1}M", mcap / 1e6)
                                            } else {
                                                format!("${:.0}", mcap)
                                            };
                                            ui.label(
                                                egui::RichText::new(mcap_str).small().monospace(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                });
        }

        // Forex Major Pairs Dashboard
        if self.show_forex_matrix {
            egui::Window::new("Forex Pairs")
                .open(&mut self.show_forex_matrix)
                .resizable(true)
                .default_size([550.0, 380.0])
                .max_size([550.0, 560.0])
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("Major Forex Pairs").strong());
                    if ui.small_button("Refresh").clicked() {
                        let symbols = vec![
                            "EURUSD", "GBPUSD", "USDJPY", "USDCHF", "AUDUSD", "NZDUSD", "USDCAD",
                            "EURGBP", "EURJPY", "GBPJPY",
                        ]
                        .into_iter()
                        .map(String::from)
                        .collect();
                        let _ = self
                            .broker_tx
                            .send(BrokerCmd::GetWatchlistQuotes { symbols });
                    }
                    ui.separator();
                    if self.forex_pairs_data.is_empty() {
                        ui.label(
                            egui::RichText::new("Loading... (requires broker connection)")
                                .color(AXIS_TEXT),
                        );
                    } else {
                        egui::ScrollArea::vertical()
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                egui::Grid::new("forex_grid")
                                    .striped(true)
                                    .num_columns(4)
                                    .min_col_width(90.0)
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new("Pair")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Last")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Change")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Change%")
                                                .color(AXIS_TEXT)
                                                .small()
                                                .strong(),
                                        );
                                        ui.end_row();
                                        for row in &self.forex_pairs_data {
                                            let color = if row.change_pct > 0.0 {
                                                UP
                                            } else if row.change_pct < 0.0 {
                                                DOWN
                                            } else {
                                                AXIS_TEXT
                                            };
                                            // Forex uses 5 decimal places for most, 3 for JPY pairs
                                            let is_jpy = row.symbol.to_uppercase().contains("JPY");
                                            let price_str = if is_jpy {
                                                format!("{:.3}", row.last)
                                            } else {
                                                format!("{:.5}", row.last)
                                            };
                                            ui.label(
                                                egui::RichText::new(&row.symbol)
                                                    .small()
                                                    .strong()
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(price_str).small().monospace(),
                                            );
                                            let chg_str = if is_jpy {
                                                format!("{:+.3}", row.change)
                                            } else {
                                                format!("{:+.5}", row.change)
                                            };
                                            ui.label(
                                                egui::RichText::new(chg_str)
                                                    .color(color)
                                                    .small()
                                                    .monospace(),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:+.2}%",
                                                    row.change_pct
                                                ))
                                                .color(color)
                                                .small()
                                                .strong()
                                                .monospace(),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                });
        }

        // DARWIN FTP Browser
        if self.show_darwin_browser {
            egui::Window::new("DarwinIA Browser")
                .open(&mut self.show_darwin_browser)
                .resizable(true).default_size([950.0, 600.0])
.max_size([950.0, 640.0])
                .show(ctx, |ui| {
                    // Top bar: scan button + stats
                    ui.horizontal(|ui| {
                        let has_gpu = self.gpu_darwin.is_some();
                        let label = if has_gpu { "DarwinIA Scan (GPU)" } else { "DarwinIA Scan (CPU)" };
                        if ui.add_enabled(!self.darwin_ftp_dir.is_empty(), egui::Button::new(label)).clicked() {
                            if has_gpu {
                                let _ = self.broker_tx.send(BrokerCmd::DarwinGpuScan { ftp_dir: self.darwin_ftp_dir.clone(), min_days: 90 });
                                self.log.push_back(LogEntry::info("DarwinIA scan started (GPU)..."));
                            } else {
                                let _ = self.broker_tx.send(BrokerCmd::DarwinFtpScan { ftp_dir: self.darwin_ftp_dir.clone(), min_days: 90 });
                                self.log.push_back(LogEntry::info("DarwinIA scan started (CPU)..."));
                            }
                        }
                        ui.label(format!("{} DARWINs loaded", self.ftp_scan_results.len()));
                        ui.separator();
                        // Ticker lookup
                        ui.label("Lookup:");
                        ui.add(egui::TextEdit::singleline(&mut self.ftp_detail_ticker).desired_width(60.0).hint_text("HAKR"));
                        if ui.button("View").clicked() && !self.ftp_detail_ticker.is_empty() && !self.darwin_ftp_dir.is_empty() {
                            let ticker = self.ftp_detail_ticker.trim().to_uppercase();
                            self.ftp_detail_ticker = ticker.clone();
                            let ftp = std::path::Path::new(&self.darwin_ftp_dir);
                            self.ftp_detail_avail = Some(darwin_ftp::check_availability(ftp, &ticker));
                            if let Ok(returns) = darwin_ftp::read_return_file(ftp, &ticker) {
                                self.ftp_detail_summary = Some(darwin_ftp::compute_return_summary(&ticker, &returns));
                                self.ftp_detail_returns = returns;
                            } else {
                                self.ftp_detail_summary = None;
                                self.ftp_detail_returns.clear();
                            }
                        }
                    });
                    ui.separator();

                    // Two-panel layout: left = table, right = detail
                    let avail_width = ui.available_width();
                    ui.horizontal(|ui| {
                        // Left panel: scan results table
                        ui.vertical(|ui| {
                            ui.set_width(avail_width * 0.55);
                            ui.heading("Universe");
                            if self.ftp_scan_results.is_empty() {
                                ui.label(egui::RichText::new("Click 'Scan Universe' to load DARWINs from FTP.").color(AXIS_TEXT));
                                if self.darwin_ftp_dir.is_empty() {
                                    ui.label(egui::RichText::new("Set FTP Dir in Settings first.").color(DOWN));
                                }
                            } else {
                                let mut darwin_sorted: Vec<&_> = self.ftp_scan_results.iter().collect();
                                match self.darwin_browser_sort.column {
                                    0 => darwin_sorted.sort_by(|a, b| a.ticker.cmp(&b.ticker)),
                                    1 => darwin_sorted.sort_by(|a, b| a.trading_days.cmp(&b.trading_days)),
                                    2 => darwin_sorted.sort_by(|a, b| a.total_return_pct.partial_cmp(&b.total_return_pct).unwrap_or(std::cmp::Ordering::Equal)),
                                    3 => darwin_sorted.sort_by(|a, b| a.max_drawdown_pct.partial_cmp(&b.max_drawdown_pct).unwrap_or(std::cmp::Ordering::Equal)),
                                    4 => darwin_sorted.sort_by(|a, b| a.sharpe.partial_cmp(&b.sharpe).unwrap_or(std::cmp::Ordering::Equal)),
                                    5 => darwin_sorted.sort_by(|a, b| a.sortino.partial_cmp(&b.sortino).unwrap_or(std::cmp::Ordering::Equal)),
                                    6 => darwin_sorted.sort_by(|a, b| a.last_quote.partial_cmp(&b.last_quote).unwrap_or(std::cmp::Ordering::Equal)),
                                    _ => {}
                                }
                                if !self.darwin_browser_sort.ascending { darwin_sorted.reverse(); }
                                egui::ScrollArea::vertical().auto_shrink(false).max_height(500.0).show(ui, |ui| {
                                    egui::Grid::new("ftp_universe").striped(true).num_columns(7).show(ui, |ui| {
                                        if SortState::header(ui, "DARWIN", 0, &self.darwin_browser_sort) { self.darwin_browser_sort.toggle(0); }
                                        if SortState::header(ui, "Days", 1, &self.darwin_browser_sort) { self.darwin_browser_sort.toggle(1); }
                                        if SortState::header(ui, "Return%", 2, &self.darwin_browser_sort) { self.darwin_browser_sort.toggle(2); }
                                        if SortState::header(ui, "MaxDD%", 3, &self.darwin_browser_sort) { self.darwin_browser_sort.toggle(3); }
                                        if SortState::header(ui, "Sharpe", 4, &self.darwin_browser_sort) { self.darwin_browser_sort.toggle(4); }
                                        if SortState::header(ui, "Sortino", 5, &self.darwin_browser_sort) { self.darwin_browser_sort.toggle(5); }
                                        if SortState::header(ui, "Price", 6, &self.darwin_browser_sort) { self.darwin_browser_sort.toggle(6); }
                                        ui.end_row();
                                        for s in darwin_sorted.iter().take(500) {
                                            let ret_c = if s.total_return_pct >= 0.0 { UP } else { DOWN };
                                            // Clickable ticker
                                            if ui.add(egui::Label::new(egui::RichText::new(&s.ticker).strong().color(ACCENT)).sense(egui::Sense::click())).clicked() {
                                                self.ftp_detail_ticker = s.ticker.clone();
                                                let ftp = std::path::Path::new(&self.darwin_ftp_dir);
                                                self.ftp_detail_avail = Some(darwin_ftp::check_availability(ftp, &s.ticker));
                                                if let Ok(returns) = darwin_ftp::read_return_file(ftp, &s.ticker) {
                                                    self.ftp_detail_summary = Some(darwin_ftp::compute_return_summary(&s.ticker, &returns));
                                                    self.ftp_detail_returns = returns;
                                                }
                                            }
                                            ui.label(format!("{}", s.trading_days));
                                            ui.label(egui::RichText::new(format!("{:.1}%", s.total_return_pct)).color(ret_c));
                                            ui.label(egui::RichText::new(format!("{:.1}%", s.max_drawdown_pct)).color(DOWN));
                                            let sharpe_c = if s.sharpe >= 1.0 { UP } else if s.sharpe >= 0.0 { AXIS_TEXT } else { DOWN };
                                            ui.label(egui::RichText::new(format!("{:.2}", s.sharpe)).color(sharpe_c));
                                            ui.label(format!("{:.2}", s.sortino));
                                            ui.label(format!("{:.1}", s.last_quote));
                                            ui.end_row();
                                        }
                                    });
                                });
                            }
                        });

                        ui.separator();

                        // Right panel: detail view
                        ui.vertical(|ui| {
                            ui.set_width(avail_width * 0.42);
                            if let Some(ref summary) = self.ftp_detail_summary {
                                ui.heading(format!("DARWIN {}", summary.ticker));
                                ui.separator();
                                egui::Grid::new("ftp_detail").striped(true).num_columns(2).show(ui, |ui| {
                                    ui.label("Trading Days:"); ui.label(format!("{}", summary.trading_days)); ui.end_row();
                                    let ret_c = if summary.total_return_pct >= 0.0 { UP } else { DOWN };
                                    ui.label("Total Return:"); ui.label(egui::RichText::new(format!("{:.2}%", summary.total_return_pct)).color(ret_c)); ui.end_row();
                                    ui.label("Max Drawdown:"); ui.label(egui::RichText::new(format!("{:.2}%", summary.max_drawdown_pct)).color(DOWN)); ui.end_row();
                                    ui.label("Sharpe Ratio:"); ui.label(format!("{:.3}", summary.sharpe)); ui.end_row();
                                    ui.label("Sortino Ratio:"); ui.label(format!("{:.3}", summary.sortino)); ui.end_row();
                                    ui.label("Daily Vol:"); ui.label(format!("{:.4}", summary.daily_vol)); ui.end_row();
                                    ui.label("Best Day:"); ui.label(egui::RichText::new(format!("{:.2}%", summary.best_day_pct)).color(UP)); ui.end_row();
                                    ui.label("Worst Day:"); ui.label(egui::RichText::new(format!("{:.2}%", summary.worst_day_pct)).color(DOWN)); ui.end_row();
                                    ui.label("DARWIN Price:"); ui.label(format!("{:.2}", summary.last_quote)); ui.end_row();
                                    ui.label("Experience:"); ui.label(format!("{:.1}", summary.experience_score)); ui.end_row();
                                    ui.label("Risk Stability:"); ui.label(format!("{:.1}", summary.risk_stability_score)); ui.end_row();
                                    ui.label("Performance:"); ui.label(format!("{:.1}", summary.performance_score)); ui.end_row();
                                });

                                // Equity curve plot
                                if self.ftp_detail_returns.len() > 5 {
                                    ui.add_space(10.0);
                                    ui.label(egui::RichText::new("Equity Curve").strong());
                                    let points: PlotPoints = PlotPoints::new(
                                        self.ftp_detail_returns.iter().enumerate()
                                            .filter_map(|(i, r)| r.cumulative_returns.last().map(|v| [i as f64, *v * 100.0]))
                                            .collect()
                                    );
                                    let line = Line::new("Equity", points).color(ACCENT);
                                    Plot::new("ftp_equity_plot")
                                        .height(180.0)
                                        .allow_drag(false)
                                        .allow_zoom(false)
                                        .show(ui, |plot_ui| { plot_ui.line(line); });
                                }

                                // Data availability
                                if let Some(ref avail) = self.ftp_detail_avail {
                                    ui.add_space(10.0);
                                    ui.label(egui::RichText::new("Data Available").strong());
                                    ui.horizontal_wrapped(|ui| {
                                        let show = |ui: &mut egui::Ui, name: &str, has: bool| {
                                            let c = if has { UP } else { egui::Color32::from_rgb(60, 60, 60) };
                                            ui.label(egui::RichText::new(name).color(c).small());
                                        };
                                        show(ui, "RETURN", avail.has_return);
                                        show(ui, "TRADES", avail.has_trades);
                                        show(ui, "POSITIONS", avail.has_positions);
                                        show(ui, "EXPERIENCE", avail.has_experience);
                                        show(ui, "RISK", avail.has_risk_stability);
                                        show(ui, "PERF", avail.has_performance);
                                        show(ui, "SCALE", avail.has_scalability);
                                        show(ui, "CORR", avail.has_market_correlation);
                                        show(ui, "BADGES", avail.has_badges);
                                        show(ui, "QUOTES", avail.has_quotes);
                                        show(ui, "VAR10", avail.has_former_var10);
                                    });
                                    if !avail.quote_months.is_empty() {
                                        ui.label(egui::RichText::new(format!("Quotes: {} months ({} → {})",
                                            avail.quote_months.len(),
                                            avail.quote_months.first().unwrap_or(&String::new()),
                                            avail.quote_months.last().unwrap_or(&String::new())
                                        )).color(AXIS_TEXT).small());
                                    }
                                    ui.label(egui::RichText::new(format!("D-Score: {} days", avail.dscore_days)).color(AXIS_TEXT).small());
                                }

                                // Correlation with our DARWINs
                                if !self.bg.accounts.is_empty() && !self.darwin_ftp_dir.is_empty() {
                                    ui.add_space(10.0);
                                    ui.label(egui::RichText::new("Correlation with Portfolio").strong());
                                    let ftp = std::path::Path::new(&self.darwin_ftp_dir);
                                    for acct in &self.bg.accounts {
                                        match darwin_ftp::compute_correlation(ftp, &summary.ticker, &acct.darwin_ticker) {
                                            Ok(corr) => {
                                                let c = if corr.abs() > 0.7 { DOWN } else if corr.abs() > 0.4 { egui::Color32::from_rgb(255, 200, 50) } else { UP };
                                                ui.label(egui::RichText::new(format!("vs {}: {:.4}", acct.darwin_ticker, corr)).color(c).small());
                                            }
                                            Err(_) => {
                                                ui.label(egui::RichText::new(format!("vs {}: N/A", acct.darwin_ticker)).color(AXIS_TEXT).small());
                                            }
                                        }
                                    }
                                }
                            } else {
                                ui.heading("DARWIN Detail");
                                ui.label(egui::RichText::new("Enter a ticker and click View, or click a ticker in the table.").color(AXIS_TEXT));
                            }
                        });
                    });
                });
        }
    }
}
