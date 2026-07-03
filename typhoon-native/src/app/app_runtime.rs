use super::*;

use super::app_runtime_support::*;

/// Log the slowest `pre_broker` tick when it crosses the threshold, with a sorted
/// breakdown of the others. Silent in steady state; a cold-start hang prints e.g.
/// `Slow pre_broker tick: deferred_chart_loads took 13800.0ms — breakdown: ...`,
/// which the aggregate `pre_broker_ms` alone never isolated.
fn report_slow_pre_broker_ticks(ticks: &[(&'static str, f32)]) {
    const SLOW_TICK_MS: f32 = 150.0;
    let Some(&(name, ms)) = ticks
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    else {
        return;
    };
    if ms < SLOW_TICK_MS {
        return;
    }
    let mut sorted: Vec<(&'static str, f32)> =
        ticks.iter().copied().filter(|(_, t)| *t >= 1.0).collect();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let breakdown = sorted
        .iter()
        .take(6)
        .map(|(n, t)| format!("{n}={t:.1}ms"))
        .collect::<Vec<_>>()
        .join(" ");
    tracing::warn!("Slow pre_broker tick: {name} took {ms:.1}ms — breakdown: {breakdown}");
}
impl eframe::App for TyphooNApp {
    fn on_exit(&mut self) {
        self.save_session();
        // Explicit WAL checkpoint on exit — keeps WAL file small for next startup.
        if let Some(ref cache) = self.cache {
            if let Ok(conn) = cache.connection() {
                let _ = conn.execute("PRAGMA wal_checkpoint(TRUNCATE)", []);
            }
        }
    }

    // eframe 0.35 removed App::update; the whole frame body lives in ui().
    // Deliberately NOT split into logic()+ui(): eframe 0.34 already gated
    // update() behind is_visible, so a hidden window pausing the data pump is
    // the long-standing shipped behavior, and keeping one body preserves it
    // exactly. Chrome panels render through the root `ui`; floating
    // egui::Window/Area code keeps using `ctx`.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &ui.ctx().clone();
        self.frame_count += 1;
        let now_instant = std::time::Instant::now();
        let perf_pre_broker_ms;
        let perf_broker_drain_ms;
        let perf_after_broker_started;
        let msgs_drained;
        let perf_post_broker_setup_ms;
        let perf_chrome_panels_ms;
        let perf_floating_windows_ms;
        // Track user activity for the auto-compact idle gate. Any input event in
        // the frame counts as activity. Cheap — `events` is always queried below.
        if ctx.input(|i| !i.events.is_empty()) {
            self.auto_compact_last_input_at = std::time::Instant::now();
        }
        // PERF DIAG: time each pre_broker tick so a cold-start stall names the exact
        // sub-operation. `pre_broker_ms` is only the aggregate; a one-off multi-second
        // hang (render-thread cache-lock contention during the startup burst) was
        // untraceable from it. Silent in steady state — report_slow_pre_broker_ticks
        // logs a breakdown only when a tick crosses the threshold.
        let mut pre_broker_ticks: Vec<(&'static str, f32)> = Vec::with_capacity(24);
        macro_rules! timed_tick {
            ($name:literal, $body:expr) => {{
                let _tt = std::time::Instant::now();
                $body;
                pre_broker_ticks.push(($name, _tt.elapsed().as_secs_f32() * 1000.0));
            }};
        }
        timed_tick!("auto_compact", self.tick_auto_compact());
        timed_tick!(
            "clear_stale_ui_busy_flags",
            self.clear_stale_ui_busy_flags(now_instant)
        );
        // Alpaca retry queue: internally throttled to 10s between ticks.
        // Loads persisted state on first call, re-dispatches due entries.
        timed_tick!("poll_alpaca_retry_queue", self.poll_alpaca_retry_queue());
        let _tt_state_caches = std::time::Instant::now();
        // PERF: Broad sync/scrape work must not leave egui in continuous full-rate
        // repaint mode. The flag was previously initialized but never driven, so
        // a 12k-symbol universe sync + news/SEC/fundamentals passes still rendered
        // every idle frame. Input frames still request immediate repaint below.
        let pending_market_data_fetches = self.total_pending_market_data_fetches();
        self.heavy_sync_in_progress = ui_heavy_sync_active(
            pending_market_data_fetches,
            self.deferred_chart_loads.len(),
            self.news_loading,
            self.scrape_fund_running,
            self.scrape_sec_running,
            self.auto_compact_in_progress,
        );
        // PERF: rebuild scope HashSet only when bg data loaded or scope changed,
        // not every frame. Steady state = zero work.
        let scope_key = (self.bg_rev, self.broker_scope);
        if self.cached_scope_key != Some(scope_key) {
            self.cached_scope_syms = self.broker_scope_symbols();
            self.cached_scope_key = Some(scope_key);
        }
        // PERF: Cache active_symbols() + HashSet until its chart/position/watchlist inputs change
        // (used by 5+ windows for "Active Only" filters).
        let active_symbols_key = self.active_symbols_cache_key();
        if self.cached_active_symbols_key != Some(active_symbols_key) {
            self.cached_active_symbols = self.active_symbols();
            self.cached_active_symbols_set = self.cached_active_symbols.iter().cloned().collect();
            self.cached_active_symbols_key = Some(active_symbols_key);
        }
        // PERF: Cache scoped_fundamentals_owned() only when bg/scope changes — not per frame.
        // Was cloning ~500 Fundamentals structs (≈1 MB) every frame for no reason.
        if self.cached_scoped_fundamentals_key != Some(scope_key) {
            self.cached_scoped_fundamentals = self.scoped_fundamentals_owned();
            self.cached_scoped_fundamentals_key = Some(scope_key);
        }
        if self.cached_alpaca_sync_state_rev != Some(self.bg_rev)
            && (!self.heavy_sync_in_progress || self.cached_alpaca_sync_state.is_empty())
        {
            let previous = self.cached_alpaca_sync_state.clone();
            let mut rebuilt = self.build_alpaca_cache_state_map();
            merge_recent_sync_overrides(&mut rebuilt, &previous, chrono::Utc::now().timestamp());
            self.cached_alpaca_sync_state = rebuilt;
            self.cached_alpaca_sync_state_rev = Some(self.bg_rev);
        }
        pre_broker_ticks.push((
            "state_caches",
            _tt_state_caches.elapsed().as_secs_f32() * 1000.0,
        ));
        // Bound log size to prevent unbounded memory growth.
        // 200 is a steady-state cap — small enough that pop_front is amortized O(1)
        // even during bulk imports that push dozens of lines per frame.
        while self.log.len() > 200 {
            self.log.pop_front();
        }

        timed_tick!(
            "request_missing_kraken_catalogs",
            self.request_missing_kraken_catalogs()
        );
        timed_tick!(
            "refresh_active_crypto_chart_if_due",
            self.refresh_active_crypto_chart_if_due(now_instant)
        );
        timed_tick!(
            "watchlist_quote_refresh",
            self.tick_watchlist_quote_refresh(now_instant)
        );
        timed_tick!(
            "positions_orders_refresh",
            self.tick_positions_orders_refresh(now_instant)
        );
        timed_tick!("bar_sync_status_refresh", self.tick_bar_sync_status_refresh());
        timed_tick!(
            "kraken_universe_schedulers",
            self.tick_kraken_universe_schedulers(now_instant)
        );
        timed_tick!(
            "kraken_ws_scheduling",
            self.tick_kraken_ws_scheduling(now_instant)
        );
        timed_tick!(
            "news_body_hydrator",
            self.tick_news_body_hydrator(now_instant)
        );
        timed_tick!("screenshot_capture", self.tick_screenshot_capture(ctx));
        timed_tick!("cache_startup", self.tick_cache_startup());

        // ── Global font/spacing to match old WebKit (Consolas 11px) ──────
        if self.frame_count == 1 {
            // Apply the dark visuals once, then layer the widget overrides below
            // on top. This used to run set_visuals(dark_visuals()) every frame,
            // which both rebuilt the global Style Arc per frame and wholesale
            // replaced style.visuals — silently clobbering the corner-radius and
            // bg_stroke overrides in this block from frame 2 onward (rounded
            // widgets despite the ALL SQUARE spec).
            ctx.set_visuals(Self::dark_visuals());
            let mut style = (*ctx.global_style()).clone();
            // ── AESTHETIC: Godel Terminal + old WebKit ──
            // Monospace everything, compact, square, green accents
            style.text_styles.insert(
                egui::TextStyle::Small,
                egui::FontId::new(10.0, egui::FontFamily::Monospace),
            );
            style.text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(11.0, egui::FontFamily::Monospace),
            );
            style.text_styles.insert(
                egui::TextStyle::Monospace,
                egui::FontId::new(11.0, egui::FontFamily::Monospace),
            );
            style.text_styles.insert(
                egui::TextStyle::Button,
                egui::FontId::new(10.0, egui::FontFamily::Monospace),
            );
            style.text_styles.insert(
                egui::TextStyle::Heading,
                egui::FontId::new(12.0, egui::FontFamily::Monospace),
            );
            // Compact but readable spacing
            style.spacing.item_spacing = egui::vec2(6.0, 2.0);
            style.spacing.button_padding = egui::vec2(4.0, 1.0);
            style.spacing.interact_size = egui::vec2(16.0, 14.0);
            style.spacing.indent = 8.0;
            style.spacing.scroll = egui::style::ScrollStyle {
                bar_width: 4.0,
                ..style.spacing.scroll
            };
            // ALL SQUARE — zero corner radius
            style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(0);
            style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(0);
            style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(0);
            style.visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(0);
            // Thin widget borders
            style.visuals.widgets.inactive.bg_stroke =
                egui::Stroke::new(0.5, egui::Color32::from_rgb(35, 40, 55));
            style.visuals.widgets.hovered.bg_stroke =
                egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 65, 90));
            style.visuals.widgets.noninteractive.bg_stroke =
                egui::Stroke::new(0.0, egui::Color32::TRANSPARENT);
            ctx.set_global_style(style);
        }

        timed_tick!(
            "background_snapshot_drain",
            self.tick_background_snapshot_drain()
        );
        // Apply a completed off-thread Kraken universe digest here — AFTER the bg
        // snapshot drain — so its `self.bg.regulatory_alerts_by_symbol` write isn't
        // clobbered by a same-frame snapshot replace (the old synchronous handler
        // ran post-drain for the same reason).
        timed_tick!(
            "kraken_universe_digest",
            self.tick_kraken_universe_digest()
        );
        timed_tick!(
            "deferred_chart_loads",
            self.tick_deferred_chart_loads(ctx, now_instant)
        );
        timed_tick!(
            "dirty_indicator_recompute",
            self.tick_dirty_indicator_recompute()
        );
        timed_tick!(
            "chart_background_results",
            self.tick_chart_background_results()
        );
        report_slow_pre_broker_ticks(&pre_broker_ticks);

        (
            perf_pre_broker_ms,
            perf_broker_drain_ms,
            perf_after_broker_started,
            msgs_drained,
        ) = self.tick_broker_messages(ctx, now_instant);

        let post_broker_setup_started = std::time::Instant::now();
        self.sync_cross_timeframe_drawings();
        perf_post_broker_setup_ms = post_broker_setup_started.elapsed().as_secs_f64() * 1000.0;

        let chrome_panels_started = std::time::Instant::now();
        self.render_menu_bar(ui);
        self.render_symbol_timeframe_toolbar(ui);
        self.render_symbol_autocomplete_dropdown(ctx);

        self.render_tab_bar(ui);
        self.render_bottom_panels(ui);

        self.render_right_panel(ui);
        perf_chrome_panels_ms = chrome_panels_started.elapsed().as_secs_f64() * 1000.0;

        // ── floating windows ─────────────────────────────────────────────────
        // Always call draw_floating_windows so close buttons work.
        // Performance: all background data reads from self.bg (background-computed).
        let floating_windows_started = std::time::Instant::now();
        self.draw_floating_windows(ctx);
        perf_floating_windows_ms = floating_windows_started.elapsed().as_secs_f64() * 1000.0;

        self.render_regulatory_alert_windows(ctx);

        // ── central panel (chart area) ────────────────────────────────────────
        self.render_drawing_toolbar(ui);

        egui::CentralPanel::default().show(ui, |ui| {
            let pointer_over_floating = self.handle_runtime_input(ctx);
            self.render_central_panel(ctx, ui, pointer_over_floating);
        });

        // ── Console (egui::Window for proper focus/interaction on Wayland) ────
        if self.command_open {
            // ADR-092: When filter is empty, show recent commands first
            let filter_empty = self.command_input.trim().is_empty();
            // ADR-094: Context-aware command filtering
            let context_filter: Option<&[&str]> = match self.palette_context {
                PaletteContext::Global => None,
                PaletteContext::Chart => Some(&[
                    "DRAW_HLINE",
                    "DRAW_TRENDLINE",
                    "DRAW_FIBO",
                    "DRAW_VLINE",
                    "DRAW_RECT",
                    "DRAW_RAY",
                    "DRAW_CHANNEL",
                    "DRAW_PARALLEL_CH",
                    "DRAW_FIB_CHANNEL",
                    "DRAW_REGRESSION",
                    "NNFX",
                    "RESET_IND",
                    "SESSIONS",
                    "SUPERTREND",
                    "DONCHIAN",
                    "KELTNER",
                    "BOLLINGER",
                    "ICHIMOKU",
                    "SQUEEZE",
                    "REGRESSION",
                    "FVG",
                    "ORDER_BLOCKS",
                    "CANDLE",
                    "HEIKINASHI",
                    "LINE",
                    "OHLC",
                    "RENKO",
                    "M1",
                    "M5",
                    "M15",
                    "M30",
                    "H1",
                    "H4",
                    "D1",
                    "W1",
                    "MN1",
                    "SCREENSHOT",
                    "COPY_CHART",
                    "REPLAY",
                    "VOLUME_PROFILE",
                    "VWAP",
                ]),
                PaletteContext::Watchlist => Some(&[
                    "SEARCH",
                    "QUOTE",
                    "FUNDAMENTALS",
                    "SEC",
                    "INSIDER",
                    "EV",
                    "EARNINGS",
                    "DIVIDENDS",
                    "ANALYST",
                    "SHORT_INTEREST",
                    "ALERTS",
                    "NEWS",
                    "OPTIONS",
                ]),
            };
            let palette_commands: Vec<&Command> =
                if filter_empty && !self.recent_commands.is_empty() {
                    // Show recent commands first, then all commands. Track names in a set so
                    // dedupe stays O(1) instead of rescanning the growing command list.
                    let mut cmds: Vec<&Command> = Vec::with_capacity(COMMANDS.len());
                    let mut seen_names: std::collections::HashSet<&'static str> =
                        std::collections::HashSet::with_capacity(COMMANDS.len());
                    for name in &self.recent_commands {
                        if let Some(c) = COMMANDS.iter().find(|c| c.name == name.as_str()) {
                            if seen_names.insert(c.name) {
                                cmds.push(c);
                            }
                        }
                    }
                    for c in COMMANDS.iter() {
                        if seen_names.insert(c.name) {
                            cmds.push(c);
                        }
                    }
                    cmds
                } else {
                    // PERF: lowercase the query ONCE, read pre-lowercased name/desc from COMMANDS_LOWER.
                    let query_lower = self.command_input.to_lowercase();
                    let mut scored: Vec<(i32, &Command)> = COMMANDS
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, c)| {
                            let ctx_match =
                                context_filter.map_or(true, |allowed| allowed.contains(&c.name));
                            if !ctx_match {
                                return None;
                            }
                            let (ref name_lc, ref desc_lc) = COMMANDS_LOWER[idx];
                            let name_score = fuzzy_score(&query_lower, name_lc);
                            let desc_score = fuzzy_score(&query_lower, desc_lc).map(|s| s + 500);
                            match (name_score, desc_score) {
                                (Some(n), Some(d)) => Some((n.min(d), c)),
                                (Some(n), None) => Some((n, c)),
                                (None, Some(d)) => Some((d, c)),
                                (None, None) => None,
                            }
                        })
                        .collect();
                    scored.sort_by_key(|(s, _)| *s);
                    scored.into_iter().map(|(_, c)| c).collect()
                };
            // Reset context to Global after opening (one-shot filtering)
            if filter_empty && self.palette_context != PaletteContext::Global {
                // Keep context while palette is open — reset on close
            }

            let num_visible = palette_commands.len().clamp(1, 15);
            let console_height = (num_visible as f32) * 24.0 + 52.0;

            let screen_width = ctx.input(|i| i.viewport_rect()).width();
            egui::Window::new("__console__")
                .title_bar(false)
                .anchor(egui::Align2::LEFT_TOP, [0.0, 0.0])
                .fixed_size([screen_width, console_height])
                .frame(
                    egui::Frame::window(&ctx.global_style())
                        .fill(egui::Color32::from_rgba_premultiplied(8, 8, 24, 247))
                        .inner_margin(8.0)
                        .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(76, 175, 80))),
                )
                .show(ctx, |ui| {
                    let input_resp = ui.add(
                        egui::TextEdit::singleline(&mut self.command_input)
                            .desired_width(screen_width - 24.0)
                            .hint_text("type a command… (Esc to close)")
                            .font(egui::FontId::monospace(14.0))
                            .text_color(egui::Color32::from_rgb(76, 175, 80)),
                    );
                    input_resp.request_focus();

                    // Arrow key navigation
                    let cmd_count = palette_commands.len();
                    let arrow_down = ctx.input(|i| i.key_pressed(egui::Key::ArrowDown));
                    let arrow_up = ctx.input(|i| i.key_pressed(egui::Key::ArrowUp));
                    if arrow_down && cmd_count > 0 {
                        self.console_selected =
                            (self.console_selected + 1).min(cmd_count.saturating_sub(1));
                    }
                    if arrow_up && cmd_count > 0 {
                        self.console_selected = self.console_selected.saturating_sub(1);
                    }
                    // Reset selection only when user actually types (not arrow-key driven changes)
                    if input_resp.changed() && !arrow_down && !arrow_up {
                        self.console_selected = 0;
                    }

                    ui.separator();

                    let mut execute: Option<String> = None;
                    // Build the MRU set once — was running iter().take(10).any() per row × N commands.
                    let recent_set: std::collections::HashSet<&str> = if filter_empty {
                        self.recent_commands
                            .iter()
                            .take(10)
                            .map(|s| s.as_str())
                            .collect()
                    } else {
                        std::collections::HashSet::new()
                    };
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .max_height(console_height - 52.0)
                        .show(ui, |ui| {
                            for (i, cmd) in palette_commands.iter().enumerate() {
                                let is_selected = i == self.console_selected;
                                let row_bg = if is_selected {
                                    egui::Color32::from_rgb(15, 52, 96)
                                } else {
                                    egui::Color32::TRANSPARENT
                                };
                                let name_col = if is_selected {
                                    egui::Color32::WHITE
                                } else {
                                    egui::Color32::from_rgb(136, 255, 255)
                                };

                                let row = ui.horizontal(|ui| {
                                    // Selected row background
                                    let rect = ui.available_rect_before_wrap();
                                    let row_rect = egui::Rect::from_min_size(
                                        rect.min,
                                        egui::vec2(rect.width(), 20.0),
                                    );
                                    ui.painter().rect_filled(row_rect, 0.0, row_bg);

                                    ui.label(
                                        egui::RichText::new(cmd.name)
                                            .color(name_col)
                                            .monospace()
                                            .strong()
                                            .size(13.0),
                                    );
                                    // ADR-092: show RECENT badge for MRU commands (O(1) HashSet lookup).
                                    if recent_set.contains(cmd.name) {
                                        ui.label(
                                            egui::RichText::new("RECENT")
                                                .color(egui::Color32::from_rgb(76, 175, 80))
                                                .size(9.0),
                                        );
                                    }
                                    ui.add_space(12.0);
                                    ui.label(
                                        egui::RichText::new(cmd.desc)
                                            .color(egui::Color32::from_rgb(136, 136, 136))
                                            .size(11.0),
                                    );
                                });
                                // Click: execute the selected palette row verbatim (no arguments).
                                if row.response.interact(egui::Sense::click()).clicked() {
                                    execute = Some(cmd.name.to_string());
                                }
                            }
                        });

                    // Enter key: if the user typed arguments (whitespace present after the
                    // command name), pass the raw input through so commands like
                    // `ASKGEMINI CC,NCLH what's their debt?` keep their arguments. Otherwise
                    // use the currently-selected palette entry so fuzzy-match still works.
                    if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let raw = self.command_input.trim().to_string();
                        if raw.contains(char::is_whitespace) {
                            // User typed command + args — honour them verbatim.
                            execute = Some(raw);
                        } else {
                            execute = palette_commands
                                .get(self.console_selected)
                                .map(|c| c.name.to_string());
                        }
                    }
                    if let Some(cmd_name) = execute {
                        self.command_open = false;
                        // ADR-092: track recent commands (MRU, max 10). For commands with
                        // arguments we only remember the leading token so the MRU list stays
                        // clean and repeat-able.
                        let mru_key = cmd_name
                            .split_whitespace()
                            .next()
                            .unwrap_or(&cmd_name)
                            .to_uppercase();
                        self.recent_commands.retain(|n| n != &mru_key);
                        self.recent_commands.push_front(mru_key);
                        self.recent_commands.truncate(10);
                        self.handle_command(&cmd_name, ctx);
                    }
                });
        }

        // Auto-save session + keyring sync every 60 seconds — runs off UI thread
        if now_instant.duration_since(self.session_last_autosave)
            >= std::time::Duration::from_secs(60)
        {
            self.session_last_autosave = now_instant;
            // Collect all state needed for save (cheap copies of strings + JSON)
            let session_json = self.build_session_json();
            self.sync_preferences_save();
            let creds: Vec<(String, String)> = [
                (keyring::keys::ALPACA_API_KEY, &self.broker_api_key),
                (keyring::keys::ALPACA_SECRET, &self.broker_secret),
                (keyring::keys::FINNHUB_KEY, &self.finnhub_key),
                (keyring::keys::FRED_KEY, &self.fred_key),
                (keyring::keys::CRYPTOPANIC_KEY, &self.cryptopanic_key),
            ]
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
            let cache_clone = self.cache.clone();
            let rt_handle = self.rt_handle.clone();
            rt_handle.spawn_blocking(move || {
                // Write session JSON to disk
                let mut path = dirs_home();
                path.push("session.json");
                let _ = std::fs::write(&path, session_json);
                // Sync credentials to keyring (only if changed)
                for (key, val) in &creds {
                    if let Ok(Some(existing)) = keyring::load(key) {
                        if &existing == val {
                            continue;
                        }
                    }
                    let _ = keyring::store(key, val);
                    // Also write to cache fallback
                    if let Some(ref cache) = cache_clone {
                        let _ = cache.put_kv(&format!("cred:{}", key), val);
                    }
                }
            });
        }

        // Update Prometheus metrics every ~5 seconds. Keep this wall-clock gated;
        // frame_count-based throttles become pathological under 144/240Hz repaint.
        if now_instant.duration_since(self.metrics_last_update) >= std::time::Duration::from_secs(5)
        {
            self.metrics_last_update = now_instant;
            if let Some(ref reg) = self.metrics_registry {
                let mut snap = crate::metrics::MetricsSnapshot::default();

                // Uptime
                snap.uptime_seconds = self.metrics_start.elapsed().as_secs_f64();

                // Broker connection
                snap.broker_connected.push((
                    "alpaca".to_string(),
                    if self.broker_connected { 1.0 } else { 0.0 },
                ));

                // Account equity from live account
                if let Some(ref acct) = self.live_account {
                    snap.account_equity
                        .push(("alpaca".to_string(), acct.equity));
                }

                // Open positions count
                snap.positions_open
                    .push(("alpaca".to_string(), self.live_positions.len() as f64));

                // Price alerts
                snap.alerts_active = self.alerts.len() as f64 + self.indicator_alerts.len() as f64;

                // Cache stats: (rows, kv_entries, size_bytes)
                if let Some((rows, _kv, size)) = self.bg.cache_stats {
                    snap.cache_size_bytes = size as f64;
                    snap.cache_symbols_total = rows as f64;
                }

                // Detailed stats: bar counts per symbol/TF (skip metadata keys)
                for (key, count, _size) in &self.bg.detailed_stats {
                    // Cache metadata rows all follow `<prefix>:__<NAME>__[…]`
                    // (SYMBOLS, SPECS, SERVER, HEARTBEAT, …). Matching the `:__` segment
                    // covers any new metadata name without a hardcoded allow-list.
                    if key.contains(":__") {
                        continue;
                    }
                    // Skip 0-count entries to reduce cardinality
                    if *count == 0 {
                        continue;
                    }
                    // key format: "source:SYMBOL:TF" or "SYMBOL:TF"
                    let parts: Vec<&str> = key.rsplitn(2, ':').collect();
                    if parts.len() == 2 {
                        snap.bars
                            .push((parts[1].to_string(), parts[0].to_string(), *count as f64));
                    }
                }

                reg.update(&snap);
            }
        }

        // ── Data sync ───────────────────────────────────────────────────────
        // No API calls or data operations before cache is loaded.
        if self.cache_loaded {
            // Weekend crypto sync via Kraken. Runs every ~60s, one symbol per cycle.
            // Symbols come from a hardcoded floor plus any crypto in chart tabs
            // or the user watchlist so user-added coins (incl. XMR/ZEC/DASH
            // which Alpaca doesn't list) still get weekend refresh coverage.
            if now_instant.duration_since(self.weekend_crypto_last_sync)
                >= std::time::Duration::from_secs(60)
            {
                self.weekend_crypto_last_sync = now_instant;
                let now_utc = chrono::Utc::now();
                let eastern = now_utc.with_timezone(
                    &chrono::FixedOffset::west_opt(5 * 3600)
                        .unwrap_or_else(|| chrono::FixedOffset::east_opt(0).unwrap()),
                );
                use chrono::Datelike;
                let is_weekend = matches!(
                    eastern.weekday(),
                    chrono::Weekday::Sat | chrono::Weekday::Sun
                );
                if is_weekend {
                    let mut crypto_syms: Vec<String> = [
                        "BTCUSD", "ETHUSD", "SOLUSD", "DOGEUSD", "XRPUSD", "ADAUSD", "LTCUSD",
                        "LINKUSD", "AVAXUSD", "DOTUSD",
                    ]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                    let mut crypto_set: std::collections::HashSet<String> =
                        crypto_syms.iter().cloned().collect();
                    for chart in &self.charts {
                        let bare = bare_symbol_from_key(&chart.symbol).to_uppercase();
                        if Self::demand_is_crypto(&bare) && !crypto_set.contains(&bare) {
                            crypto_syms.push(bare.clone());
                            crypto_set.insert(bare);
                        }
                    }
                    for wl in &self.user_watchlist {
                        let wlu = wl.to_uppercase();
                        if Self::demand_is_crypto(&wlu) && !crypto_set.contains(&wlu) {
                            crypto_syms.push(wlu.clone());
                            crypto_set.insert(wlu);
                        }
                    }
                    if !crypto_syms.is_empty() {
                        let sym_idx = ((self.frame_count / 240) as usize) % crypto_syms.len();
                        let sym = crypto_syms[sym_idx].clone();
                        let db_path = cache_db_path();
                        let kraken_tfs = self.filtered_sync_timeframes([
                            "1Day", "1Hour", "4Hour", "15Min", "30Min", "5Min",
                        ]);
                        if !kraken_tfs.is_empty() && self.kraken_spot_symbol_scrape_enabled(&sym) {
                            let _ = self.broker_tx.send(BrokerCmd::KrakenBackfill {
                                symbol: sym,
                                timeframes: kraken_tfs,
                                db_path,
                                backfill_complete: false,
                            });
                        }
                    }
                }
            }

            // Alpaca equity rotation — iterate Alpaca's full us_equity tradable
            // universe (~11000 symbols), plus a chart/watchlist floor that holds
            // even before the asset-list fetch completes. Runs 7 days/week —
            // stocks don't trade on weekends but the historical backfill can
            // still progress.
            if now_instant.duration_since(self.alpaca_rotation_last_sync)
                >= self.market_data_sync_interval()
            {
                self.alpaca_rotation_last_sync = now_instant;
                if self.alpaca_enabled {
                    self.maybe_request_alpaca_asset_universe();
                    self.push_alpaca_sync_runtime_config();
                    let equity_syms = self.alpaca_equity_rotation_symbols_cached();
                    self.schedule_alpaca_pairs(&equity_syms);
                }
            }
        }

        // Repaint strategy:
        // - Trading terminals should not idle at 4-10 FPS while prices, cursor
        //   overlays, live bars, and background sync state are moving.
        // - Request the next frame every update and let wgpu/eframe vsync cap it
        //   at the monitor's native refresh rate. This keeps UI latency low while
        //   still avoiding runaway uncapped presentation.
        // - TYPHOON_IDLE_FPS can force an explicit refresh-rate cap for
        //   profiling/problem displays; unset/0 means native-refresh continuous
        //   repaint through vsync/GSYNC/FreeSync.
        let session_save_started = std::time::Instant::now();
        let render_after_broker_ms = session_save_started
            .saturating_duration_since(perf_after_broker_started)
            .as_secs_f64()
            * 1000.0;
        self.maybe_incremental_session_save(ctx);
        let session_save_ms = session_save_started.elapsed().as_secs_f64() * 1000.0;

        let update_ms = now_instant.elapsed().as_secs_f64() * 1000.0;
        // Sampled once per frame and shared by both perf-stall logs below (the
        // per-frame detail warn and the 5s summary). Reading /proc VmRSS is a
        // few microseconds — negligible against the frame budget.
        let rss_mb = crate::app::market_data_sync::current_process_rss_mb();

        if update_ms >= 250.0 {
            let render_residual_ms = (render_after_broker_ms
                - perf_post_broker_setup_ms
                - perf_chrome_panels_ms
                - perf_floating_windows_ms)
                .max(0.0);
            tracing::warn!(
                "UI frame stall detail: update_ms={:.2} pre_broker_ms={:.2} broker_drain_ms={:.2} render_after_broker_ms={:.2} post_broker_setup_ms={:.2} chrome_panels_ms={:.2} floating_windows_ms={:.2} render_residual_ms={:.2} session_save_ms={:.2} msgs_drained={} pending_fetches={} heavy_sync={} news_loading={} fund_scrape={} sec_scrape={} compact={} rss_mb={}",
                update_ms,
                perf_pre_broker_ms,
                perf_broker_drain_ms,
                render_after_broker_ms,
                perf_post_broker_setup_ms,
                perf_chrome_panels_ms,
                perf_floating_windows_ms,
                render_residual_ms,
                session_save_ms,
                msgs_drained,
                self.total_pending_market_data_fetches(),
                self.heavy_sync_in_progress,
                self.news_loading,
                self.scrape_fund_running,
                self.scrape_sec_running,
                self.auto_compact_in_progress,
                rss_mb,
            );
        }
        if update_ms > 16.7 {
            self.perf_slow_frame_count = self.perf_slow_frame_count.saturating_add(1);
        }
        self.perf_max_update_ms = self.perf_max_update_ms.max(update_ms);
        self.perf_broker_msgs_drained = self
            .perf_broker_msgs_drained
            .saturating_add(msgs_drained as u32);
        if now_instant.duration_since(self.perf_last_report) >= std::time::Duration::from_secs(5) {
            if self.perf_slow_frame_count > 0 || self.perf_broker_msgs_drained > 0 {
                let pending_fetches = self.total_pending_market_data_fetches();
                if self.perf_max_update_ms >= 250.0 {
                    tracing::warn!(
                        "UI frame stall: max_update_ms={:.2} slow_frames={} broker_msgs={} pending_fetches={} deferred_chart_loads={} rss_mb={} heavy_sync={} news_loading={} fund_scrape={} sec_scrape={} compact={} log_entries={}",
                        self.perf_max_update_ms,
                        self.perf_slow_frame_count,
                        self.perf_broker_msgs_drained,
                        pending_fetches,
                        self.deferred_chart_loads.len(),
                        rss_mb,
                        self.heavy_sync_in_progress,
                        self.news_loading,
                        self.scrape_fund_running,
                        self.scrape_sec_running,
                        self.auto_compact_in_progress,
                        self.log.len()
                    );
                } else {
                    tracing::debug!(
                        "frame perf: max_update_ms={:.2} slow_frames={} broker_msgs={} pending_fetches={} deferred_chart_loads={} log_entries={}",
                        self.perf_max_update_ms,
                        self.perf_slow_frame_count,
                        self.perf_broker_msgs_drained,
                        pending_fetches,
                        self.deferred_chart_loads.len(),
                        self.log.len()
                    );
                }
            }
            self.perf_last_report = now_instant;
            self.perf_slow_frame_count = 0;
            self.perf_max_update_ms = 0.0;
            self.perf_broker_msgs_drained = 0;
        }

        static IDLE_FPS_CAP: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
        let idle_fps_cap = *IDLE_FPS_CAP.get_or_init(|| {
            std::env::var("TYPHOON_IDLE_FPS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0)
        });
        if self.heavy_sync_in_progress {
            let frame_ms = if idle_fps_cap > 0 {
                (1000 / idle_fps_cap.max(1)).max(1)
            } else {
                // Keep visible progress/animations fluid under sync pressure while
                // avoiding unconstrained native-refresh repaint competing with the
                // background sync workers and the compositor.
                16
            };
            ctx.request_repaint_after(std::time::Duration::from_millis(frame_ms));
        } else if idle_fps_cap > 0 {
            let frame_ms = (1000 / idle_fps_cap.max(1)).max(1);
            ctx.request_repaint_after(std::time::Duration::from_millis(frame_ms));
        } else {
            ctx.request_repaint();
        }

        // UX3: Apply any deferred symbol context-menu action from right-panel renders
        if !matches!(self.deferred_symbol_action, SymbolAction::None) {
            let action = std::mem::replace(&mut self.deferred_symbol_action, SymbolAction::None);
            self.apply_symbol_action(action);
        }
    }
}
