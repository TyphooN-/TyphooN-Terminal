use super::*;

impl TyphooNApp {
    pub(super) fn tick_news_body_hydrator(&mut self, now_instant: std::time::Instant) {
        // News body hydrator: fetch the full article text for rows that
        // still only have the provider summary. Throttled by
        // HYDRATE_INTERVAL_SECS and gated on `in_flight` so we never have
        // two tokio tasks racing on the same cache rows.
        if self.cache_loaded
            && !self.news_body_hydrate_in_flight
            && now_instant.duration_since(self.news_body_last_hydrate)
                >= std::time::Duration::from_secs(
                    typhoon_broker_runtime::news_ingest::HYDRATE_INTERVAL_SECS,
                )
        {
            if let Some(cache) = self.cache.clone() {
                self.news_body_last_hydrate = now_instant;
                self.news_body_hydrate_in_flight = true;
                let symbol_hint = self
                    .charts
                    .first()
                    .map(|c| c.symbol.clone())
                    .filter(|s| !s.is_empty());
                let rt = self.rt_handle.clone();
                rt.spawn(async move {
                    let _ = typhoon_broker_runtime::news_ingest::hydrate_missing_bodies(
                        cache,
                        symbol_hint,
                    )
                    .await;
                    // No callback channel: the next tick simply observes the
                    // `in_flight` flag being reset after the task completes.
                    // We can't poke `self` from here, so the gate is released
                    // on the next `update()` by a separate fast path below.
                });
            }
        }
        // Release the in-flight flag after a generous timeout — covers the
        // case where the spawned task is still running but a new tick wants
        // to re-arm. We don't need exact synchronisation: the new task will
        // pick up whatever rows are still empty.
        if self.news_body_hydrate_in_flight
            && now_instant.duration_since(self.news_body_last_hydrate)
                >= std::time::Duration::from_secs(
                    typhoon_broker_runtime::news_ingest::HYDRATE_INTERVAL_SECS * 2,
                )
        {
            self.news_body_hydrate_in_flight = false;
        }
    }

    pub(super) fn handle_news_sec_result_msg(&mut self, msg: BrokerMsg) {
        match msg {
            BrokerMsg::SecScrapeResult(msg) => {
                self.scrape_sec_running = false;
                self.scrape_sec_last_msg = msg.clone();
                self.log.push_back(LogEntry::info(msg));
            }
            BrokerMsg::FilingContent(text) => {
                self.sec_filing_content = text;
                self.sec_filing_loading = false;
                // Invalidate cached summary so it re-computes for the new content.
                self.sec_filing_summary = None;
                self.sec_filing_summary_for.clear();
                self.log
                    .push_back(LogEntry::info("SEC filing document loaded"));
            }
            BrokerMsg::FinnhubNewsResult(articles) => {
                self.news_loading = false;
                self.log.push_back(LogEntry::info(format!(
                    "Finnhub: {} articles loaded",
                    articles.len()
                )));
                self.news_articles = articles;
            }
            _ => {}
        }
    }

    pub(super) fn handle_news_ingest_msg(&mut self, msg: BrokerMsg) {
        match msg {
            BrokerMsg::IngestResearchResult {
                per_symbol_added,
                errors,
            } => self.handle_ingest_research_result(per_symbol_added, errors),
            BrokerMsg::NewsArticlesLoaded { symbol, articles } => {
                self.handle_news_articles_loaded(symbol, articles);
            }
            BrokerMsg::NewsDbTotal(n) => {
                self.news_db_total = Some(n);
            }
            _ => {}
        }
    }

    fn handle_ingest_research_result(
        &mut self,
        per_symbol_added: Vec<(String, usize, usize)>,
        errors: Vec<String>,
    ) {
        self.ingest_research_busy = false;
        if per_symbol_added.is_empty() && errors.is_empty() {
            self.ingest_research_status = "No articles parsed.".into();
            return;
        }

        let summary: Vec<String> = per_symbol_added
            .iter()
            .map(|(s, added, total)| format!("{}: +{} (now {})", s, added, total))
            .collect();
        let total_added: usize = per_symbol_added.iter().map(|(_, a, _)| *a).sum();
        self.ingest_research_status = if errors.is_empty() {
            format!(
                "Ingested {} new articles across {} symbol(s): {}",
                total_added,
                per_symbol_added.len(),
                summary.join(" · ")
            )
        } else {
            format!(
                "Ingested {} new articles · {} error(s): {}",
                total_added,
                errors.len(),
                errors.join("; ")
            )
        };
        self.log
            .push_back(LogEntry::info(self.ingest_research_status.clone()));

        // Auto-refresh the News panel so the pasting user sees the new articles
        // without having to click "Load Cached". Prefer the symbol the user is
        // currently filtering; otherwise fall back to the first ingested symbol.
        if total_added > 0 {
            let refresh_sym = if !self.news_symbol_filter.trim().is_empty() {
                self.news_symbol_filter.trim().to_uppercase()
            } else {
                per_symbol_added
                    .first()
                    .map(|(s, _, _)| s.to_uppercase())
                    .unwrap_or_default()
            };
            if !refresh_sym.is_empty() {
                self.news_loading = true;
                let _ = self.broker_tx.send(BrokerCmd::LoadCachedNews {
                    symbol: refresh_sym,
                    limit: 200,
                });
            }
        }
    }

    fn handle_news_articles_loaded(
        &mut self,
        symbol: String,
        articles: Vec<typhoon_engine::core::news::NewsArticle>,
    ) {
        self.news_loading = false;
        let count = articles.len();
        self.news_full_articles = articles;

        // Update content hash for news cache guard.
        let mut h = self.news_full_articles.len() as u64;
        if let Some(first) = self.news_full_articles.first() {
            for b in first.headline.as_bytes() {
                h = h.wrapping_mul(31).wrapping_add(*b as u64);
            }
        }
        self.news_input_hash = h;

        // Build headline rows and restore selection in one pass. Previous code
        // built the rows, then did a second linear .position() scan by URL hash.
        let selected_hash = self.news_selected_url_hash.clone();
        let restore_selected = !selected_hash.is_empty();
        let mut restored_idx = None;
        self.news_articles = self
            .news_full_articles
            .iter()
            .enumerate()
            .map(|(idx, a)| {
                if restore_selected && restored_idx.is_none() && a.url_hash == selected_hash {
                    restored_idx = Some(idx);
                }
                let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(a.published_at, 0)
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "—".to_string());
                let source = if a.provider.is_empty() {
                    a.source.clone()
                } else {
                    a.provider.clone()
                };
                (a.headline.clone(), source, dt)
            })
            .collect();

        if restore_selected {
            self.news_selected = restored_idx;
        }
        // Clear selection if the selected index is now out of range.
        if let Some(idx) = self.news_selected {
            if idx >= self.news_full_articles.len() {
                self.news_selected = None;
            }
        }
        if self.news_selected.is_none() && !self.news_full_articles.is_empty() {
            self.news_selected = Some(0);
        }
        if let Some(idx) = self.news_selected {
            if let Some(article) = self.news_full_articles.get(idx) {
                self.news_selected_url_hash = article.url_hash.clone();
            }
        }

        let label = if symbol.is_empty() {
            "all".to_string()
        } else {
            symbol
        };
        self.log.push_back(LogEntry::info(format!(
            "News {}: {} articles loaded",
            label, count
        )));
    }
}
