use super::*;
use crate::app::app_runtime_support::should_start_manual_background_scope_scrape;

impl TyphooNApp {
    pub(super) fn render_news_window(&mut self, ctx: &egui::Context) {
        // News
        if self.show_news {
            // Resolve chart symbol once up front (avoid borrow conflicts inside the window closure).
            let chart_symbol = self
                .charts
                .get(self.active_tab)
                .map(|c| {
                    c.symbol
                        .split(':')
                        .rev()
                        .nth(1)
                        .or_else(|| c.symbol.split(':').last())
                        .unwrap_or("AAPL")
                        .to_string()
                })
                .unwrap_or_else(|| "AAPL".to_string());
            // `news_symbol_filter` was removed from the UI (the Search
            // field now owns filtering). The field is retained on the
            // app struct for backward-compat with sessions saved by
            // older builds, but the auto-fill from active-chart no
            // longer happens — the Fetch button uses `chart_symbol`
            // directly, and the Search field starts empty so all cached
            // articles are visible.
            let _ = &self.news_symbol_filter; // keep the borrow live for any future use

            // The header's "· N in DB" count is computed broker-side and pushed
            // via BrokerMsg::NewsDbTotal (see the count emits in
            // typhoon_broker_runtime::news on cached-load / fresh-fetch / scrape,
            // handled in app_runtime_news_results). The render thread does ZERO
            // SQLite work for it: the old poll here called count_all_articles on
            // the write connection, so it blocked behind the bulk bar-sync
            // writers and produced the 10–17s News-window frame stalls. The
            // auto-load below fires a LoadCachedNews on first open, whose
            // response carries the count, so the header still populates promptly.
            // See ADR-121.

            // Auto-load cached articles into memory on first open this
            // session. Without this, fresh launches show "0 cached"
            // until the user clicks Load Cached even though the SQLite
            // table holds the corpus. Gated on:
            //   * cache is open (otherwise the load is a no-op)
            //   * no load already in flight (avoid duplicate dispatch)
            //   * not already done this session (one-shot)
            //   * DB has rows worth loading (skip empty cache)
            if !self.news_initial_load_done
                && !self.news_loading
                && self.cache.is_some()
                && self.news_db_total.map(|n| n > 0).unwrap_or(true)
            {
                self.news_initial_load_done = true;
                self.news_loading = true;
                let load_symbol = match SearchFilterMode::parse(&self.news_search_query) {
                    SearchFilterMode::Symbols(syms) if !syms.is_empty() => syms.join(","),
                    _ => String::new(),
                };
                let _ = self.broker_tx.send(BrokerCmd::LoadCachedNews {
                    symbol: load_symbol,
                    limit: 500,
                });
            }
            let news_scope_is_all = self.broker_scope == EventSource::All;
            let news_scope_label = if news_scope_is_all { "All" } else { "Active" }.to_string();
            // Build only the small Active set per frame. Full ALL expansion can be
            // 10k+ symbols, so do it only after the user clicks Fetch (All).
            let mut active_news_symbols: Vec<String> =
                self.active_news_scrape_symbols().into_iter().collect();
            active_news_symbols.sort();
            active_news_symbols.dedup();
            if active_news_symbols.is_empty() && !chart_symbol.trim().is_empty() {
                active_news_symbols.push(chart_symbol.clone());
            }

            let content_h = ctx.content_rect().height();
            let news_default_h = (content_h * 0.58).clamp(420.0, 560.0);
            let mut open = self.show_news;
            let mut open_url: Option<String> = None;
            let mut open_chart_symbol: Option<String> = None;
            // Optimistic per-article removal: the click handlers below record the
            // hash to drop, applied after the window closure so we never mutate
            // news_full_articles while it is being iterated for rendering.
            let mut news_remove_hash: Option<String> = None;
            // "Purge spam": records the active ticker to bulk-remove cached
            // articles that fail the relevance gate, applied after the closure.
            let mut news_purge_ticker: Option<String> = None;
            egui::Window::new("News & Research")
                .open(&mut open)
                .resizable(true)
                .default_size([920.0, news_default_h])
                .min_size([300.0, 260.0])
                // Keep the reader inside the viewport. Without a vertical cap,
                // long article bodies can make the floating window auto-size past
                // the screen bottom before the child scroll areas get a chance to
                // take over.
                .max_height((content_h - 24.0).max(260.0))
                // Window resizing in egui is still content-driven: when the
                // content reports a larger minimum than the user's dragged size,
                // the window grows again on the next frame. Keep a thin outer
                // vertical scroll shell as the last-resort overflow path so the
                // header/detail content cannot force the floating window back to
                // max height after a vertical shrink. The list/body panes still
                // own normal scrolling at usable sizes.
                .vscroll(true)
                .constrain(true)
                .show(ctx, |ui| {
                    // ── Top bar: Search-driven filter + fetch controls ────────────
                    // The Search field is now the single point of control for
                    // the news list. Three modes are auto-detected from the
                    // text content:
                    //   * Empty               → show every cached article
                    //   * "TNDM, GDC, CC"     → comma-separated symbol filter
                    //                           (matches article.symbol OR any
                    //                           ticker in article.tickers)
                    //   * "/dads.*club/i"     → regex headline match
                    //   * anything else       → FTS5 broker keyword search
                    // The previous separate "Symbol:" input + "Use Chart" pair
                    // was redundant once the search field took over filtering.
                    ui.horizontal(|ui| {
                        if ui.add_enabled(!self.news_loading, egui::Button::new("Load Cached").fill(BTN_GREEN))
                            .on_hover_text("Read cached articles from SQLite (no fetch). If Search contains symbol CSV, load those symbols directly instead of only the latest global rows.").clicked() {
                            self.news_loading = true;
                            let load_symbol = match SearchFilterMode::parse(&self.news_search_query) {
                                SearchFilterMode::Symbols(syms) if !syms.is_empty() => syms.join(","),
                                _ => String::new(),
                            };
                            let _ = self.broker_tx.send(BrokerCmd::LoadCachedNews { symbol: load_symbol, limit: 500 });
                        }
                        if ui.add_enabled(!self.news_loading, egui::Button::new("Fetch All Sources").fill(BTN_BLUE))
                            .on_hover_text("Fetch fresh news for the active chart symbol from every configured provider (GDELT, Yahoo, Marketaux, Alpha Vantage, FMP, Finnhub, CryptoPanic, CoinDesk).").clicked() {
                            // Falls back to the active chart's symbol so the
                            // user doesn't need a separate symbol input. To
                            // fetch for a different symbol, open that symbol
                            // on a chart first (CHARTSYMBOL command).
                            let sym = chart_symbol.trim().to_uppercase();
                            if sym.is_empty() {
                                self.log.push_back(LogEntry::warn("News: open a symbol on a chart first"));
                            } else {
                                self.news_loading = true;
                                let _ = self.broker_tx.send(BrokerCmd::FetchNewsMulti {
                                    symbol: sym.clone(),
                                    marketaux_key: self.marketaux_key.clone(),
                                    alpha_vantage_key: self.alpha_vantage_key.clone(),
                                    fmp_key: self.fmp_key.clone(),
                                    finnhub_key: self.finnhub_key.clone(),
                                    cryptopanic_key: self.cryptopanic_key.clone(),
                                });
                                self.log.push_back(LogEntry::info(format!("News: fetching multi-source for {}...", sym)));
                            }
                        }
                        let scrape_label = format!("Fetch ({})", news_scope_label);
                        let scrape_hover = if news_scope_is_all {
                            "Full news scrape for the whole available source universe. Symbol expansion is deferred until click so the UI does not rebuild 10k+ symbols every frame.".to_string()
                        } else {
                            format!(
                                "Bulk news scrape for Active symbols: watchlist + positions + MTF Grid ({} symbol{})",
                                active_news_symbols.len(),
                                if active_news_symbols.len() == 1 { "" } else { "s" }
                            )
                        };
                        if ui
                            .add_enabled(
                                !self.news_loading
                                    && (news_scope_is_all || !active_news_symbols.is_empty()),
                                egui::Button::new(scrape_label).fill(BTN_MG),
                            )
                            .on_hover_text(scrape_hover)
                            .clicked()
                        {
                            let symbols = if news_scope_is_all {
                                self.news_scrape_scope_symbols()
                            } else {
                                active_news_symbols.clone()
                            };
                            if symbols.is_empty() {
                                self.log.push_back(LogEntry::warn(
                                    "News: full universe is not ready yet; wait for source universes to load",
                                ));
                            } else {
                                let symbol_count = symbols.len();
                                if !should_start_manual_background_scope_scrape(
                                    self.broker_scope,
                                    symbol_count,
                                    self.heavy_sync_in_progress,
                                ) {
                                    self.log.push_back(LogEntry::warn(format!(
                                        "News: Scope {} scrape deferred during market-data catch-up ({} symbols); use Active scope or retry after sync settles",
                                        news_scope_label, symbol_count
                                    )));
                                } else {
                                    self.news_loading = true;
                                    let _ = self.broker_tx.send(BrokerCmd::NewsScrapeSymbols {
                                        symbols,
                                        marketaux_key: self.marketaux_key.clone(),
                                        alpha_vantage_key: self.alpha_vantage_key.clone(),
                                        fmp_key: self.fmp_key.clone(),
                                        finnhub_key: self.finnhub_key.clone(),
                                        cryptopanic_key: self.cryptopanic_key.clone(),
                                    });
                                    self.log.push_back(LogEntry::info(format!(
                                        "News: scope scrape started for {} ({} symbols)",
                                        news_scope_label, symbol_count
                                    )));
                                }
                            }
                        }
                        if ui
                            .add_enabled(
                                !self.news_loading && !self.news_full_articles.is_empty(),
                                egui::Button::new("Purge spam"),
                            )
                            .on_hover_text(
                                "Remove cached articles that fail the relevance gate for the active ticker (GDELT false-positives like cooking / real-estate spam on short tickers). Deleted + ignored so they do not return.",
                            )
                            .clicked()
                        {
                            news_purge_ticker = Some(chart_symbol.clone());
                        }
                        if self.news_loading {
                            ui.spinner();
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Search:").color(AXIS_TEXT));
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.news_search_query)
                                .desired_width(360.0)
                                .hint_text("symbol(s) e.g. TNDM, GDC  ·  /regex/  ·  keyword… (FTS5)"),
                        );
                        let enter_pressed = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        let fts_clicked = ui.button("FTS Search").clicked();
                        let do_search = enter_pressed || fts_clicked;
                        // FTS button explicitly forces a broker keyword search
                        // even when the field LOOKS like a symbol or regex —
                        // useful when you actually want to search article
                        // bodies for the literal text "TNDM". Pressing Enter
                        // on a symbol CSV loads those symbols directly from
                        // SQLite, so older WOK/TNDM articles don't disappear
                        // just because they fell outside the latest global 500.
                        if do_search {
                            let q = self.news_search_query.trim().to_string();
                            if !q.is_empty() {
                                self.news_loading = true;
                                match SearchFilterMode::parse(&q) {
                                    SearchFilterMode::Symbols(syms) if !fts_clicked && !syms.is_empty() => {
                                        let _ = self.broker_tx.send(BrokerCmd::LoadCachedNews {
                                            symbol: syms.join(","),
                                            limit: 500,
                                        });
                                    }
                                    _ => {
                                        let _ = self.broker_tx.send(BrokerCmd::SearchNews { query: q, limit: 200 });
                                    }
                                }
                            } else {
                                // Empty query → reload everything cached.
                                self.news_loading = true;
                                let _ = self.broker_tx.send(BrokerCmd::LoadCachedNews { symbol: String::new(), limit: 500 });
                            }
                        }
                        ui.separator();
                        // Show in-memory count AND DB total so the user
                        // can tell at a glance whether Load Cached has
                        // pulled the data into memory yet, vs whether
                        // the cache itself is empty. "47 loaded · 12,803
                        // in DB" reads better than just "47 cached".
                        let mem_n = self.news_full_articles.len();
                        let label = match self.news_db_total {
                            Some(db) if db as usize > mem_n => format!(
                                "{} loaded · {} in DB",
                                mem_n, db
                            ),
                            Some(_) => format!("{} cached", mem_n),
                            None => format!("{} cached", mem_n),
                        };
                        ui.label(egui::RichText::new(label).color(AXIS_TEXT).small());
                    });
                    ui.collapsing("API Keys (free tier)", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Marketaux:");
                            ui.add(egui::TextEdit::singleline(&mut self.marketaux_key).desired_width(180.0).password(true));
                            ui.label(egui::RichText::new("100/day").color(AXIS_TEXT).small());
                        });
                        ui.horizontal(|ui| {
                            ui.label("Alpha Vantage:");
                            ui.add(egui::TextEdit::singleline(&mut self.alpha_vantage_key).desired_width(180.0).password(true));
                            ui.label(egui::RichText::new("25/day").color(AXIS_TEXT).small());
                        });
                        ui.horizontal(|ui| {
                            ui.label("FMP:");
                            ui.add(egui::TextEdit::singleline(&mut self.fmp_key).desired_width(180.0).password(true));
                            ui.label(egui::RichText::new("250/day — shared w/ transcripts").color(AXIS_TEXT).small());
                        });
                        ui.label(egui::RichText::new("GDELT and Yahoo RSS require no key.").color(AXIS_TEXT).small());
                    });
                    ui.separator();

                    // ── Two-pane reader: list (left) + body (right) ──────────────
                    if self.news_full_articles.is_empty() {
                        // Empty-state message differentiates "the DB is
                        // empty" vs "the in-memory list is empty but
                        // the DB has rows" so the user knows whether
                        // Load Cached will actually surface anything.
                        let msg = match self.news_db_total {
                            Some(0) | None => {
                                "No cached news. Click Fetch All Sources for the active chart symbol."
                                    .to_string()
                            }
                            Some(n) if self.news_loading => format!(
                                "Loading {} cached articles from SQLite…",
                                n
                            ),
                            Some(n) => format!(
                                "{} articles in cache. Click Load Cached to pull them into memory.",
                                n
                            ),
                        };
                        ui.label(egui::RichText::new(msg).color(AXIS_TEXT));
                    } else {
                        // Parse the Search field into a client-side filter
                        // before grouping so dedup only spans the filtered set.
                        // Three modes: empty, /regex/, symbol-CSV.
                        let raw_filter = self.news_search_query.trim();
                        let filter_mode = SearchFilterMode::parse(raw_filter);
                        // Apply filter to produce visible_indices in input
                        // order. Indices stay relative to news_full_articles
                        // so click handlers can still index back cleanly.
                        let visible_indices: Vec<usize> = self
                            .news_full_articles
                            .iter()
                            .enumerate()
                            .filter(|(_, a)| filter_mode.matches(a))
                            .map(|(i, _)| i)
                            .collect();
                        // Group the FILTERED articles so multiple sources of
                        // the same story collapse into one row. Each group is
                        // (primary_idx, alternate_indices) — both index into
                        // news_full_articles, not into visible_indices.
                        let filtered_articles: Vec<typhoon_engine::core::news::NewsArticle> = visible_indices
                            .iter()
                            .map(|&i| self.news_full_articles[i].clone())
                            .collect();
                        let local_groups = typhoon_engine::core::news::group_articles_by_headline(
                            &filtered_articles,
                        );
                        // Remap local indices (into filtered_articles) back to
                        // full-list indices so click handlers still work.
                        let groups: Vec<(usize, Vec<usize>)> = local_groups
                            .into_iter()
                            .map(|(primary, alternates)| {
                                let p = visible_indices[primary];
                                let alts = alternates
                                    .into_iter()
                                    .map(|i| visible_indices[i])
                                    .collect();
                                (p, alts)
                            })
                            .collect();
                        let avail_w = ui.available_width();
                        let list_w = (avail_w * 0.38).clamp(240.0, 420.0);
                        // Filter status line above the panes so the user sees
                        // "12 stories from 47 articles · TNDM, GDC" etc.
                        let status = match &filter_mode {
                            SearchFilterMode::All => format!(
                                "{} stories from {} articles",
                                groups.len(),
                                self.news_full_articles.len()
                            ),
                            SearchFilterMode::Symbols(syms) => format!(
                                "{} stories from {} articles · filter: {}",
                                groups.len(),
                                visible_indices.len(),
                                syms.join(", ")
                            ),
                            SearchFilterMode::Regex { pattern, .. } => format!(
                                "{} stories from {} articles · /{}/i",
                                groups.len(),
                                visible_indices.len(),
                                pattern
                            ),
                            SearchFilterMode::InvalidRegex(err) => format!(
                                "Regex error: {} (showing all)",
                                err
                            ),
                        };
                        ui.label(egui::RichText::new(status).color(AXIS_TEXT).small());
                        // Bind pane height to the *post-header* remaining area. This
                        // matters for both directions: when the user shrinks, the
                        // scroll areas get a small bounded height instead of forcing
                        // the window back open; when the user expands, the scroll
                        // areas advertise that larger height so egui preserves the
                        // new resize state instead of snapping back to content size.
                        // Reserve a small slack below the two-pane row. Allocating
                        // exactly `available_height` made the row + its trailing
                        // item-spacing overflow the window content rect by a few px
                        // every frame, so egui's resize kept growing the window back
                        // to `max_height` — the "can't shrink vertically" bug. Keeping
                        // the row strictly inside the available area lets the user
                        // shrink the window and have it stay shrunk.
                        let pane_h = (ui.available_height()
                            - ui.spacing().item_spacing.y
                            - 4.0)
                            .min((content_h * 0.82).max(96.0))
                            .max(96.0);
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), pane_h),
                            egui::Layout::left_to_right(egui::Align::Min),
                            |ui| {
                            // ── Left: article list ──
                            ui.vertical(|ui| {
                                ui.set_width(list_w);
                                ui.set_min_height(pane_h);
                                egui::ScrollArea::vertical()
                                    .id_salt("news_list_scroll")
                                    .max_height(pane_h)
                                    .min_scrolled_height(pane_h)
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        for (i, alternates) in &groups {
                                            let i = *i;
                                            let a = &self.news_full_articles[i];
                                            let selected = self.news_selected == Some(i)
                                                || alternates.iter().any(|&j| self.news_selected == Some(j));
                                            let source_count = 1 + alternates.len();
                                            let ts = if a.published_at > 0 {
                                                chrono::DateTime::from_timestamp(a.published_at, 0)
                                                    .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                                                    .unwrap_or_default()
                                            } else { String::new() };
                                            let sent_color = match a.sentiment.as_str() {
                                                "bullish" => UP,
                                                "bearish" => DOWN,
                                                _ => egui::Color32::from_rgb(160, 160, 160),
                                            };
                                            let associated_tickers = Self::news_article_tickers(
                                                &a.symbol,
                                                &a.tickers,
                                            );
                                            let row = ui.group(|ui| {
                                                ui.set_width(list_w - 20.0);
                                                ui.vertical(|ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(egui::RichText::new(&a.source).color(egui::Color32::from_rgb(130, 170, 220)).small());
                                                        if !a.provider.is_empty() {
                                                            ui.label(egui::RichText::new(format!("· {}", &a.provider)).color(AXIS_TEXT).small());
                                                        }
                                                        if !a.sentiment.is_empty() {
                                                            ui.label(egui::RichText::new(format!("· {}", &a.sentiment)).color(sent_color).small());
                                                        }
                                                        if source_count > 1 {
                                                            // Group badge: tells the user this
                                                            // story was published by N outlets.
                                                            // The right pane lets them switch
                                                            // between them via the Sources
                                                            // dropdown.
                                                            ui.label(
                                                                egui::RichText::new(format!(
                                                                    "· +{} sources",
                                                                    source_count - 1
                                                                ))
                                                                .color(egui::Color32::from_rgb(
                                                                    200, 160, 100,
                                                                ))
                                                                .small(),
                                                            );
                                                        }
                                                    });
                                                    let color = if selected {
                                                        egui::Color32::from_rgb(255, 255, 255)
                                                    } else {
                                                        egui::Color32::from_rgb(220, 220, 220)
                                                    };
                                                    ui.horizontal_top(|ui| {
                                                        // Reserve the dismiss button's width up front so the
                                                        // headline wraps within the remaining column width
                                                        // instead of overflowing and pushing "×" outside the
                                                        // clipped list column (which made it unclickable).
                                                        let btn_w = 24.0;
                                                        let text_w = (ui.available_width() - btn_w).max(40.0);
                                                        ui.allocate_ui_with_layout(
                                                            egui::vec2(text_w, 0.0),
                                                            egui::Layout::top_down(egui::Align::Min),
                                                            |ui| {
                                                                ui.add(
                                                                    egui::Label::new(
                                                                        egui::RichText::new(&a.headline).color(color).strong(),
                                                                    )
                                                                    .wrap_mode(egui::TextWrapMode::Wrap),
                                                                );
                                                            },
                                                        );
                                                        if ui.small_button("×").clicked() {
                                                            if let Some(article) = self.news_full_articles.get(i) {
                                                                let _ = self.broker_tx.send(BrokerCmd::IgnoreNewsArticle {
                                                                    symbol: chart_symbol.clone(),
                                                                    url_hash: article.url_hash.clone(),
                                                                });
                                                                news_remove_hash = Some(article.url_hash.clone());
                                                                self.log.push_back(LogEntry::info(format!("News: ignored article for {}", chart_symbol)));
                                                            }
                                                        }
                                                    });
                                                    if !ts.is_empty() {
                                                        ui.label(egui::RichText::new(ts).color(AXIS_TEXT).small());
                                                    }
                                                    if !associated_tickers.is_empty() {
                                                        ui.horizontal_wrapped(|ui| {
                                                            for ticker in &associated_tickers {
                                                                let button = egui::Button::new(
                                                                    egui::RichText::new(ticker)
                                                                        .color(egui::Color32::from_rgb(
                                                                            180, 220, 160,
                                                                        ))
                                                                        .small(),
                                                                )
                                                                .small();
                                                                if ui
                                                                    .add(button)
                                                                    .on_hover_text(format!(
                                                                        "Open {} D1 chart in MTF_Grid",
                                                                        ticker
                                                                    ))
                                                                    .clicked()
                                                                {
                                                                    open_chart_symbol = Some(ticker.clone());
                                                                }
                                                            }
                                                        });
                                                    }
                                                });
                                            });
                            row.response.context_menu(|ui| {
                                if ui.button("Remove / Ignore this article").clicked() {
                                    if let Some(article) = self.news_full_articles.get(i) {
                                        let _ = self.broker_tx.send(BrokerCmd::IgnoreNewsArticle {
                                            symbol: chart_symbol.clone(),
                                            url_hash: article.url_hash.clone(),
                                        });
                                        news_remove_hash = Some(article.url_hash.clone());
                                        self.log.push_back(LogEntry::info(format!("News: ignored article for {}", chart_symbol)));
                                    }
                                    ui.close();
                                }
                            });

                            if row.response.interact(egui::Sense::click()).clicked() {
                                                self.news_selected = Some(i);
                                                // On-click hydrate: if the selected article
                                                // still has no body, ask the broker thread to
                                                // fetch the URL and refresh the symbol's
                                                // article list when done. Idempotent at the
                                                // cache layer, so the rare double-click is
                                                // harmless. Articles past the per-URL retry
                                                // ceiling (body_fetch_attempts >= MAX) skip
                                                // the dispatch — the placeholder stays as
                                                // "body unavailable" and the user has the
                                                // Open Source button.
                                                if let Some(article) = self.news_full_articles.get(i) {
                                                    self.news_selected_url_hash = article.url_hash.clone();
                                                    if article.body.is_empty()
                                                        && !article.url.is_empty()
                                                        && article.body_fetch_attempts
                                                            < typhoon_engine::core::news::MAX_BODY_FETCH_ATTEMPTS
                                                    {
                                                        let _ = self.broker_tx.send(
                                                            BrokerCmd::HydrateNewsArticle {
                                                                symbol: article.symbol.clone(),
                                                                url_hash: article.url_hash.clone(),
                                                                url: article.url.clone(),
                                                            },
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    });
                            });
                            ui.separator();
                            // ── Right: article body ──
                            ui.vertical(|ui| {
                                ui.set_min_height(pane_h);
                                if let Some(idx) = self.news_selected {
                                    // Find which group (if any) this selected
                                    // article belongs to so we can show the
                                    // Sources switcher when there are siblings.
                                    let selected_group: Option<&(usize, Vec<usize>)> = groups
                                        .iter()
                                        .find(|(p, alts)| *p == idx || alts.iter().any(|&j| j == idx));
                                    // Sources switcher: one button per article
                                    // in this group, including the currently-
                                    // selected one. Clicking switches which
                                    // article is rendered without affecting the
                                    // left-list selection-state styling.
                                    if let Some((primary, alts)) = selected_group {
                                        if !alts.is_empty() {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "Sources ({}):",
                                                        1 + alts.len()
                                                    ))
                                                    .color(AXIS_TEXT)
                                                    .small(),
                                                );
                                                let mut all = vec![*primary];
                                                all.extend(alts.iter().copied());
                                                for src_idx in all {
                                                    let src_a = match self
                                                        .news_full_articles
                                                        .get(src_idx)
                                                    {
                                                        Some(x) => x,
                                                        None => continue,
                                                    };
                                                    let label = if !src_a.provider.is_empty() {
                                                        src_a.provider.clone()
                                                    } else {
                                                        src_a.source.clone()
                                                    };
                                                    let is_selected = idx == src_idx;
                                                    let btn = egui::Button::new(
                                                        egui::RichText::new(&label).small(),
                                                    )
                                                    .selected(is_selected);
                                                    if ui.add(btn).clicked() {
                                                        self.news_selected = Some(src_idx);
                                                        self.news_selected_url_hash = src_a.url_hash.clone();
                                                    }
                                                }
                                            });
                                            ui.add_space(2.0);
                                        }
                                    }
                                    if let Some(a) = self.news_full_articles.get(idx) {
                                        let associated_tickers = Self::news_article_tickers(
                                            &a.symbol,
                                            &a.tickers,
                                        );
                                        ui.label(egui::RichText::new(&a.headline).strong().size(16.0));
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(&a.source).color(egui::Color32::from_rgb(130, 170, 220)));
                                            if !a.provider.is_empty() {
                                                ui.label(egui::RichText::new(format!("· {}", &a.provider)).color(AXIS_TEXT));
                                            }
                                            if a.published_at > 0 {
                                                let ts = chrono::DateTime::from_timestamp(a.published_at, 0)
                                                    .map(|d| d.format("%Y-%m-%d %H:%M UTC").to_string())
                                                    .unwrap_or_default();
                                                ui.label(egui::RichText::new(format!("· {}", ts)).color(AXIS_TEXT));
                                            }
                                        });
                                        if !a.sentiment.is_empty() {
                                            let sent_color = match a.sentiment.as_str() {
                                                "bullish" => UP,
                                                "bearish" => DOWN,
                                                _ => egui::Color32::from_rgb(160, 160, 160),
                                            };
                                            let score_text = if a.sentiment_score != 0.0 {
                                                format!("Sentiment: {} ({:+.2})", a.sentiment, a.sentiment_score)
                                            } else {
                                                format!("Sentiment: {}", a.sentiment)
                                            };
                                            ui.label(egui::RichText::new(score_text).color(sent_color));
                                        }
                                        if !associated_tickers.is_empty() {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.label(egui::RichText::new("Tickers:").color(AXIS_TEXT).small());
                                                for t in &associated_tickers {
                                                    let button = egui::Button::new(
                                                        egui::RichText::new(t)
                                                            .color(egui::Color32::from_rgb(
                                                                180, 220, 160,
                                                            ))
                                                            .small(),
                                                    )
                                                    .small();
                                                    if ui
                                                        .add(button)
                                                        .on_hover_text(format!(
                                                            "Open {} D1 chart in MTF_Grid",
                                                            t
                                                        ))
                                                        .clicked()
                                                    {
                                                        open_chart_symbol = Some(t.clone());
                                                    }
                                                }
                                            });
                                        }
                                        if !a.categories.is_empty() {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.label(egui::RichText::new("Topics:").color(AXIS_TEXT).small());
                                                for c in &a.categories {
                                                    ui.label(egui::RichText::new(c).color(AXIS_TEXT).small());
                                                }
                                            });
                                        }
                                        ui.separator();
                                        let body_scroll_h = (ui.available_height() - 24.0).max(96.0);
                                        egui::ScrollArea::vertical()
                                            .id_salt("news_body_scroll")
                                            .auto_shrink([false, false])
                                            .min_scrolled_height(body_scroll_h)
                                            .max_height(body_scroll_h)
                                            .show(ui, |ui| {
                                                // Hero image (og:image / provider banner).
                                                // Constrained to pane width to keep the
                                                // layout stable when the source is a
                                                // 2000×1500 publisher hero. Errors load
                                                // silently — egui's loader retries the URL
                                                // on its own schedule and the rest of the
                                                // article renders regardless.
                                                if !a.image_url.is_empty() {
                                                    let max_w = ui.available_width().min(560.0);
                                                    ui.add(
                                                        egui::Image::new(&a.image_url)
                                                            .max_width(max_w)
                                                            .corner_radius(4.0),
                                                    );
                                                    ui.add_space(8.0);
                                                }
                                                // Prefer the cached full body when the
                                                // hydrator has fetched it (see ADR-214 +
                                                // `news_ingest`); fall back to the
                                                // provider summary; finally show a
                                                // placeholder so the user knows the
                                                // body fetch is still pending. The body is
                                                // rendered via the CommonMark viewer so
                                                // paragraph breaks, inline links, and any
                                                // markdown the extractor or AI return path
                                                // preserves all format properly — see
                                                // ADR-215 for the threat-model decision
                                                // against a full HTML/JS renderer.
                                                if !a.body.is_empty() {
                                                    // Readability pass over the raw scraped body:
                                                    // strips Loading.../ad cruft, delineates the
                                                    // reader-comments blob, and forces real
                                                    // CommonMark paragraph breaks. Free fn returning
                                                    // an owned String so it doesn't borrow self
                                                    // (news_md_cache is a disjoint field borrow).
                                                    let cleaned =
                                                        typhoon_engine::core::news::clean_article_body(
                                                            &a.body,
                                                        );
                                                    egui_commonmark::CommonMarkViewer::new()
                                                        .show(ui, &mut self.news_md_cache, &cleaned);
                                                } else {
                                                    let hydration_exhausted = a.body_fetch_attempts
                                                        >= typhoon_engine::core::news::MAX_BODY_FETCH_ATTEMPTS;
                                                    if !a.summary.is_empty() {
                                                        egui_commonmark::CommonMarkViewer::new()
                                                            .show(ui, &mut self.news_md_cache, &a.summary);
                                                        ui.add_space(6.0);
                                                    }
                                                    let placeholder = if hydration_exhausted {
                                                        "(Body unavailable for this publisher — click Open Source for the full article.)"
                                                    } else if !a.summary.is_empty() {
                                                        "(Full article still hydrating — re-open in a minute or click Open Source.)"
                                                    } else {
                                                        "(No summary — click Open Source for the full article.)"
                                                    };
                                                    ui.label(
                                                        egui::RichText::new(placeholder)
                                                            .color(AXIS_TEXT)
                                                            .italics()
                                                            .small(),
                                                    );
                                                }
                                            });
                                        ui.horizontal(|ui| {
                                            if ui.button("Open Source").clicked() {
                                                open_url = Some(a.url.clone());
                                            }
                                        });
                                        // The raw URL is a single long unbreakable token. Inside the
                                        // horizontal row above it refused to wrap, so its full pixel
                                        // width became the article pane's — and therefore the whole
                                        // window's — minimum width, a resize floor that changed with
                                        // every article (the bug the user reported). Render it on its
                                        // own line and TRUNCATE to the available width so the reader
                                        // shrinks freely on both axes; the full URL is on hover.
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(&a.url).color(AXIS_TEXT).small(),
                                            )
                                            .wrap_mode(egui::TextWrapMode::Truncate),
                                        )
                                        .on_hover_text(a.url.as_str());
                                    }
                                } else {
                                    ui.label(egui::RichText::new("Select an article from the list.").color(AXIS_TEXT));
                                }
                            });
                        });
                    }
                });
            self.show_news = open;
            // Apply the optimistic removal now that rendering (which borrows
            // news_full_articles) has finished. Selection is cleared because the
            // indices shift; the detail pane falls back to "Select an article".
            if let Some(hash) = news_remove_hash {
                self.news_full_articles.retain(|a| a.url_hash != hash);
                self.news_selected = None;
                self.news_selected_url_hash.clear();
            }
            // Bulk "Purge spam": drop every loaded article that fails the
            // relevance gate for the active ticker (GDELT false-positives), and
            // ignore each so it can't re-ingest. Hashes are collected first to
            // keep the borrows disjoint.
            if let Some(ticker) = news_purge_ticker {
                let t = ticker.trim().to_uppercase();
                let purge: Vec<String> = self
                    .news_full_articles
                    .iter()
                    .filter(|a| {
                        let assoc = a.symbol.eq_ignore_ascii_case(&t)
                            || a.tickers.iter().any(|tk| tk.eq_ignore_ascii_case(&t));
                        assoc && !typhoon_engine::core::news::article_is_relevant_for_ticker(a, &t)
                    })
                    .map(|a| a.url_hash.clone())
                    .collect();
                for hash in &purge {
                    let _ = self.broker_tx.send(BrokerCmd::IgnoreNewsArticle {
                        symbol: t.clone(),
                        url_hash: hash.clone(),
                    });
                }
                if !purge.is_empty() {
                    self.news_full_articles
                        .retain(|a| !purge.contains(&a.url_hash));
                    self.news_selected = None;
                    self.news_selected_url_hash.clear();
                }
                self.log.push_back(LogEntry::info(format!(
                    "News: purged {} irrelevant article(s) for {}",
                    purge.len(),
                    t
                )));
            }
            if let Some(symbol) = open_chart_symbol {
                self.open_news_ticker_chart(&symbol);
            }
            if let Some(url) = open_url {
                ctx.open_url(egui::OpenUrl::new_tab(url));
            }
        }
    }
}
