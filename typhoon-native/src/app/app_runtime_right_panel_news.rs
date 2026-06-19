use super::*;

#[allow(deprecated)]
impl TyphooNApp {
    pub(super) fn render_right_panel_news_section(&mut self, ui: &mut egui::Ui) {
        // ── News Section ──────────────────────────────────
        {
            // Use the focus-filtered count in the header so the navbar
            // tells the user how many articles their open charts /
            // watchlist / positions / orders / holdings actually match.
            // Falls back to total when news_full_articles is empty
            // (legacy 3-tuple news has no ticker tags to filter by).
            let news_count = if !self.news_full_articles.is_empty() {
                let focus = self.news_focus_symbols();
                self.news_full_articles
                    .iter()
                    .filter(|a| Self::news_article_in_focus(&focus, &a.symbol, &a.tickers))
                    .count()
            } else {
                self.news_articles.len()
            };
            let news_section = egui::CollapsingHeader::new(
                egui::RichText::new(format!("☰ News ({})", news_count))
                    .strong()
                    .small(),
            )
            .id_salt("news_section")
            .default_open(self.right_news_open || news_count > 0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // One Fetch News button for the right panel. When MTF Grid is
                    // active with multiple symbols, it kicks the multi-source
                    // dedup scrape (NewsScrapeSymbols); otherwise it falls back
                    // to the single-symbol Finnhub fetch for the active chart.
                    // Avoids the previous duplicate button in the MTF Grid section.
                    let mtf_symbols = if self.mtf_enabled {
                        self.mtf_grid_news_symbols()
                    } else {
                        Vec::new()
                    };
                    let use_mtf = mtf_symbols.len() > 1;
                    let button_label = if use_mtf {
                        format!("Fetch News ({} MTF)", mtf_symbols.len())
                    } else {
                        "Fetch News".to_string()
                    };
                    let have_finnhub = !self.finnhub_key.is_empty();
                    let can_fetch = use_mtf || have_finnhub;
                    if can_fetch {
                        if ui
                            .add_enabled(
                                !self.news_loading,
                                egui::Button::new(
                                    egui::RichText::new(button_label).small(),
                                )
                                .fill(BTN_BLUE)
                                .min_size(egui::vec2(80.0, 18.0)),
                            )
                            .on_hover_text(if use_mtf {
                                "Fetch/cache multi-source news once per unique MTF Grid ticker"
                            } else {
                                "Fetch Finnhub news for the active symbol"
                            })
                            .clicked()
                        {
                            if use_mtf {
                                let count = mtf_symbols.len();
                                let label = mtf_symbols.join(", ");
                                let _ = self.broker_tx.send(BrokerCmd::NewsScrapeSymbols {
                                    symbols: mtf_symbols,
                                    marketaux_key: self.marketaux_key.clone(),
                                    alpha_vantage_key: self.alpha_vantage_key.clone(),
                                    fmp_key: self.fmp_key.clone(),
                                    finnhub_key: self.finnhub_key.clone(),
                                    cryptopanic_key: self.cryptopanic_key.clone(),
                                });
                                self.news_loading = true;
                                self.show_news = true;
                                self.log.push_back(LogEntry::info(format!(
                                    "News: fetching {} deduped MTF Grid symbol(s): {}",
                                    count, label
                                )));
                            } else {
                                let sym = self
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
                                self.news_loading = true;
                                if typhoon_engine::core::news::is_crypto_symbol(&sym) {
                                    let _ = self.broker_tx.send(BrokerCmd::FetchNewsMulti {
                                        symbol: sym.clone(),
                                        marketaux_key: self.marketaux_key.clone(),
                                        alpha_vantage_key: self.alpha_vantage_key.clone(),
                                        fmp_key: self.fmp_key.clone(),
                                        finnhub_key: self.finnhub_key.clone(),
                                        cryptopanic_key: self.cryptopanic_key.clone(),
                                    });
                                    self.show_news = true;
                                    self.log.push_back(LogEntry::info(format!(
                                        "News: fetching crypto multi-source for {sym}"
                                    )));
                                } else {
                                    let _ = self.broker_tx.send(BrokerCmd::FinnhubNews {
                                        symbol: sym.clone(),
                                        api_key: self.finnhub_key.clone(),
                                    });
                                    self.log.push_back(LogEntry::info(format!(
                                        "Finnhub: fetching news for {sym}"
                                    )));
                                }
                            }
                        }
                        if self.news_loading {
                            ui.spinner();
                        }
                    } else {
                        ui.label(
                            egui::RichText::new("Set Finnhub key in Settings")
                                .color(AXIS_TEXT)
                                .small(),
                        );
                    }
                });
                // Build the O(n)-construct, O(1)-lookup focus set once so the
                // per-article filter below runs in constant time per row.
                let news_focus = self.news_focus_symbols();
                let right_news_rows: Vec<(String, String, String, String)> = if !self.news_full_articles.is_empty() {
                    self.news_full_articles
                        .iter()
                        .filter(|a| {
                            Self::news_article_in_focus(&news_focus, &a.symbol, &a.tickers)
                        })
                        .map(|a| {
                            let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(
                                a.published_at,
                                0,
                            )
                            .map(|d| d.format("%Y-%m-%d").to_string())
                            .unwrap_or_else(|| "—".to_string());
                            let source = if a.provider.is_empty() {
                                a.source.clone()
                            } else {
                                a.provider.clone()
                            };
                            let mut tickers = Vec::new();
                            let primary = a.symbol.trim().to_uppercase();
                            if !primary.is_empty() {
                                tickers.push(primary);
                            }
                            for ticker in &a.tickers {
                                let ticker = ticker.trim().to_uppercase();
                                if !ticker.is_empty()
                                    && !tickers.iter().any(|t| t == &ticker)
                                {
                                    tickers.push(ticker);
                                }
                            }
                            (a.headline.clone(), source, dt, tickers.join(", "))
                        })
                        .collect()
                } else {
                    // Legacy 3-tuple news has no ticker metadata, so the focus
                    // filter can't bite — show everything.
                    self.news_articles
                        .iter()
                        .map(|(headline, source, dt)| {
                            (headline.clone(), source.clone(), dt.clone(), String::new())
                        })
                        .collect()
                };
                if news_count == 0 {
                    // Distinguish "no fetch yet" from "filter excluded
                    // everything" — the latter is easy to hit when the
                    // user has narrow focus and the news cache spans a
                    // wider universe.
                    let total_cached = self
                        .news_full_articles
                        .len()
                        .max(self.news_articles.len());
                    let message = if total_cached > 0 {
                        format!(
                            "{} cached articles, none match your open charts / watchlist / positions / orders / holdings.",
                            total_cached
                        )
                    } else {
                        "No news loaded for the active symbol.".to_string()
                    };
                    ui.label(
                        egui::RichText::new(message).color(AXIS_TEXT).small(),
                    );
                } else {
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .max_height(180.0)
                        .id_salt("news_scroll_r")
                        .show(ui, |ui| {
                            let have_full = !self.news_full_articles.is_empty();
                            let mut open_idx: Option<usize> = None;
                            for (i, (headline, source, dt, tickers)) in
                                right_news_rows.iter().enumerate()
                            {
                                let hl = headline.to_lowercase();
                                let bullish = [
                                    "surge",
                                    "rally",
                                    "beat",
                                    "up ",
                                    "soar",
                                    "gain",
                                    "rise",
                                    "jump",
                                    "bull",
                                    "record high",
                                ];
                                let bearish = [
                                    "crash", "fall", "miss", "down ", "plunge",
                                    "drop", "sink", "bear", "sell-off", "selloff",
                                    "decline",
                                ];
                                let is_bull =
                                    bullish.iter().any(|w| hl.contains(w));
                                let is_bear =
                                    bearish.iter().any(|w| hl.contains(w));
                                let (hl_color, hl_prefix) = if is_bull {
                                    (UP, "[BULL] ")
                                } else if is_bear {
                                    (DOWN, "[BEAR] ")
                                } else {
                                    (egui::Color32::from_rgb(190, 190, 200), "")
                                };
                                let hl_text = if hl_prefix.is_empty() {
                                    headline.clone()
                                } else {
                                    format!("{hl_prefix}{headline}")
                                };
                                // Wrap the row (meta line + headline) in a single
                                // frameless button so the whole article area is the
                                // click target. Opens the News floating window and
                                // focuses this article instead of letting drag turn
                                // into a text selection.
                                let resp = ui
                                    .scope(|ui| {
                                        ui.spacing_mut().button_padding =
                                            egui::vec2(2.0, 2.0);
                                        ui.add(
                                            egui::Button::new(
                                                egui::RichText::new(&hl_text)
                                                    .color(hl_color)
                                                    .small(),
                                            )
                                            .frame(false)
                                            .fill(egui::Color32::TRANSPARENT)
                                            .wrap(),
                                        )
                                    })
                                    .inner
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .on_hover_text(
                                        "Open in News window",
                                    );
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    ui.label(
                                        egui::RichText::new(dt)
                                            .color(egui::Color32::from_rgb(
                                                80, 80, 95,
                                            ))
                                            .small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(source)
                                            .color(egui::Color32::from_rgb(
                                                100, 100, 120,
                                            ))
                                            .small(),
                                    );
                                    if !tickers.is_empty() {
                                        ui.label(
                                            egui::RichText::new(tickers)
                                                .color(egui::Color32::from_rgb(
                                                    180, 200, 140,
                                                ))
                                                .small(),
                                        );
                                    }
                                });
                                if resp.clicked() {
                                    open_idx = Some(i);
                                }
                                ui.add_space(2.0);
                            }
                            if let Some(i) = open_idx {
                                if have_full
                                    && i < self.news_full_articles.len()
                                {
                                    self.news_selected = Some(i);
                                }
                                self.show_news = true;
                            }
                    });
                }
            });
            self.right_news_open = news_section.fully_open();
            self.handle_right_panel_section_drag(
                ui,
                RightPanelSectionId::News,
                &news_section.header_response,
            );
        }
    }
}
