use super::*;

impl ChartState {
    pub(crate) fn should_reload_for_bar_fetch(
        &self,
        symbol: &str,
        timeframe: &str,
        source: &str,
    ) -> bool {
        if !self.symbol_matches(symbol)
            || !self
                .timeframe
                .cache_suffix()
                .eq_ignore_ascii_case(timeframe)
        {
            return false;
        }
        if matches!(source, "alpaca" | "yahoo-chart")
            && self.primary_source.eq_ignore_ascii_case("kraken-equities")
        {
            return true;
        }
        self.bars.is_empty()
            || self.primary_source.is_empty()
            || self.primary_source.eq_ignore_ascii_case(source)
    }

    pub(crate) fn latest_quote_bar_from_cache(cache: &SqliteCache, symbol: &str) -> Option<Bar> {
        chart_source_cache_keys("kraken-equities", symbol, "quote")
            .into_iter()
            .filter_map(|key| cache.get_bars_raw(&key).ok().flatten())
            .flat_map(|raw| raw.into_iter())
            .filter(|(ts, o, h, l, c, _v)| {
                *ts > 0
                    && *o > 0.0
                    && *h > 0.0
                    && *l > 0.0
                    && *c > 0.0
                    && o.is_finite()
                    && h.is_finite()
                    && l.is_finite()
                    && c.is_finite()
                    && *h >= *l
            })
            .max_by_key(|(ts, _, _, _, _, _)| *ts)
            .map(|(ts_ms, open, high, low, close, volume)| Bar {
                ts_ms,
                open,
                high,
                low,
                close,
                volume,
            })
    }

    pub(crate) fn chart_timeframe_ms(&self) -> i64 {
        (self.timeframe.minutes().max(1) as i64) * 60_000
    }

    pub(crate) fn aggregate_daily_raw_to_monthly(
        raw: Vec<(i64, f64, f64, f64, f64, f64)>,
    ) -> Vec<Bar> {
        use chrono::{Datelike, TimeZone};
        let mut monthly: std::collections::BTreeMap<(i32, u32), Bar> =
            std::collections::BTreeMap::new();
        for (ts, o, h, l, c, v) in raw.into_iter().filter(|(ts, o, h, l, c, _v)| {
            *ts > 0
                && *o > 0.0
                && *h > 0.0
                && *l > 0.0
                && *c > 0.0
                && o.is_finite()
                && h.is_finite()
                && l.is_finite()
                && c.is_finite()
                && *h >= *l
        }) {
            let Some(dt) = chrono::Utc.timestamp_millis_opt(ts).single() else {
                continue;
            };
            let Some(bucket_dt) = chrono::Utc
                .with_ymd_and_hms(dt.year(), dt.month(), 1, 0, 0, 0)
                .single()
            else {
                continue;
            };
            let bucket_key = (dt.year(), dt.month());
            let bucket_ts = bucket_dt.timestamp_millis();
            monthly
                .entry(bucket_key)
                .and_modify(|bar| {
                    bar.high = bar.high.max(h).max(c);
                    bar.low = bar.low.min(l).min(c);
                    bar.close = c;
                    bar.volume += v;
                })
                .or_insert(Bar {
                    ts_ms: bucket_ts,
                    open: o,
                    high: h,
                    low: l,
                    close: c,
                    volume: v,
                });
        }
        monthly.into_values().collect()
    }

    pub(crate) fn aggregate_bars_to_timeframe(
        raw: Vec<(i64, f64, f64, f64, f64, f64)>,
        tf_ms: i64,
    ) -> Vec<Bar> {
        let mut aggregated: Vec<Bar> = Vec::new();
        let mut current_bucket: Option<i64> = None;
        for (ts, o, h, l, c, v) in raw.into_iter().filter(|(ts, o, h, l, c, _v)| {
            *ts > 0
                && *o > 0.0
                && *h > 0.0
                && *l > 0.0
                && *c > 0.0
                && o.is_finite()
                && h.is_finite()
                && l.is_finite()
                && c.is_finite()
                && *h >= *l
        }) {
            let bucket = ts / tf_ms * tf_ms;
            if current_bucket != Some(bucket) {
                aggregated.push(Bar {
                    ts_ms: bucket,
                    open: o,
                    high: h,
                    low: l,
                    close: c,
                    volume: v,
                });
                current_bucket = Some(bucket);
            } else if let Some(bar) = aggregated.last_mut() {
                bar.high = bar.high.max(h).max(c);
                bar.low = bar.low.min(l).min(c);
                bar.close = c;
                bar.volume += v;
            }
        }
        aggregated
    }

    pub(crate) fn rebuild_from_lower_timeframe_if_dislocated(
        &mut self,
        cache: &SqliteCache,
        symbol: &str,
    ) -> bool {
        let Some(quote) = Self::latest_quote_bar_from_cache(cache, symbol) else {
            return false;
        };
        if self.bars.is_empty() || quote.close <= 0.0 || !quote.close.is_finite() {
            return false;
        }
        let Some(last_close) = self
            .bars
            .last()
            .map(|bar| bar.close)
            .filter(|p| *p > 0.0 && p.is_finite())
        else {
            return false;
        };
        let ratio = if last_close >= quote.close {
            last_close / quote.close
        } else {
            quote.close / last_close
        };
        if ratio < 20.0 {
            return false;
        }

        let target_tf_ms = self.chart_timeframe_ms();
        let lower_tfs = [
            ("1Min", 60_000_i64),
            ("5Min", 5 * 60_000_i64),
            ("15Min", 15 * 60_000_i64),
            ("30Min", 30 * 60_000_i64),
            ("1Hour", 60 * 60_000_i64),
            ("4Hour", 4 * 60 * 60_000_i64),
        ];
        let source = if self.primary_source.is_empty() {
            "kraken-equities"
        } else {
            self.primary_source
        };
        for (lower_tf, lower_ms) in lower_tfs {
            if lower_ms >= target_tf_ms {
                continue;
            }
            for key in chart_source_cache_keys(source, symbol, lower_tf) {
                let Ok(Some(raw)) = cache.get_bars_raw(&key) else {
                    continue;
                };
                let rebuilt = Self::aggregate_bars_to_timeframe(raw, target_tf_ms);
                if rebuilt.len() < 2 {
                    continue;
                }
                let Some(rebuilt_close) = rebuilt
                    .last()
                    .map(|bar| bar.close)
                    .filter(|p| *p > 0.0 && p.is_finite())
                else {
                    continue;
                };
                let rebuilt_ratio = if rebuilt_close >= quote.close {
                    rebuilt_close / quote.close
                } else {
                    quote.close / rebuilt_close
                };
                if rebuilt_ratio < 20.0 {
                    self.bars = rebuilt;
                    self.primary_source = source;
                    self.primary_first_ts = self.bars.first().map(|bar| bar.ts_ms).unwrap_or(0);
                    self.gap_fill_timestamps.clear();
                    return true;
                }
            }
        }
        false
    }

    pub(crate) fn apply_quote_cache_overlay(&mut self, cache: &SqliteCache, symbol: &str) -> bool {
        let Some(quote) = Self::latest_quote_bar_from_cache(cache, symbol) else {
            return false;
        };
        if self.bars.is_empty() {
            self.bars.push(quote);
            self.primary_source = "kraken-equities";
            return true;
        }
        let tf_ms = self.chart_timeframe_ms();
        let Some(last) = self.bars.last_mut() else {
            return false;
        };
        // Always allow live quotes. The 30-second freshness guard in technical_analysis.rs
        // prevents stale bid/ask from being shown. This fixes decoupling during extended hours.
        if quote.ts_ms < last.ts_ms.saturating_add(tf_ms) {
            last.close = quote.close;
            last.high = last.high.max(quote.high).max(quote.close);
            last.low = if last.low > 0.0 {
                last.low.min(quote.low).min(quote.close)
            } else {
                quote.low.min(quote.close)
            };
            last.volume = last.volume.max(quote.volume);
        } else {
            self.bars.push(quote);
        }
        true
    }

    /// Cache key for this symbol + timeframe.
    /// Try multiple prefix variants to find data in cache.
    pub(crate) fn find_cache_key(
        &self,
        cache: &SqliteCache,
        dsm: &typhoon_engine::core::data_source::DataSourceManager,
    ) -> String {
        let tf = self.timeframe.cache_suffix();
        let sym = {
            let parts: Vec<&str> = self.symbol.split(':').collect();
            let is_tf = matches!(
                parts.last().copied(),
                Some(
                    "1Min"
                        | "5Min"
                        | "15Min"
                        | "30Min"
                        | "1Hour"
                        | "4Hour"
                        | "1Day"
                        | "1Week"
                        | "1Month"
                )
            );
            if is_tf && parts.len() > 1 {
                parts[..parts.len() - 1].join(":")
            } else {
                self.symbol.clone()
            }
        };
        let sym_norm = normalize_market_data_symbol(&sym);

        // Normalize crypto: try both with and without slash
        let sym_alt = if sym_norm.contains('/') {
            sym_norm.replace('/', "")
        } else {
            let crypto_bases = [
                "BTC", "ETH", "SOL", "DOGE", "XRP", "ADA", "LTC", "LINK", "AVAX", "DOT", "XMR",
                "ZEC", "DASH", "UNI", "AAVE", "MATIC", "SHIB", "ATOM", "ALGO", "FTM", "NEAR",
                "APE", "ARB", "OP", "MKR", "COMP", "SNX", "CRV", "SUSHI", "YFI", "BAT", "MANA",
                "SAND", "AXS", "BCH", "ETC", "XLM", "FIL", "HBAR", "ICP", "VET", "THETA",
            ];
            let su = sym_norm.to_uppercase();
            crypto_bases
                .iter()
                .find_map(|base| {
                    if su.starts_with(base) && su.ends_with("USD") && su.len() == base.len() + 3 {
                        Some(format!("{}/USD", base))
                    } else {
                        None
                    }
                })
                .unwrap_or_default()
        };

        // ADR-038 Phase 2: Use DataSourceManager for priority-ordered candidates
        let mut candidates = dsm.resolve_candidates(&sym, tf);
        if sym_norm != sym {
            candidates.extend(dsm.resolve_candidates(&sym_norm, tf));
        }
        // Also add legacy key variants for backward compatibility
        candidates.push(format!("paper_TyphooN:{}:{}", sym.to_uppercase(), tf));
        candidates.push(format!(
            "alpaca_paper_TyphooN:{}:{}",
            sym.to_uppercase(),
            tf
        ));
        candidates.push(format!("default:{}:{}", sym.to_uppercase(), tf));
        if sym_norm != sym {
            candidates.push(format!("paper_TyphooN:{}:{}", sym_norm.to_uppercase(), tf));
            candidates.push(format!(
                "alpaca_paper_TyphooN:{}:{}",
                sym_norm.to_uppercase(),
                tf
            ));
            candidates.push(format!("default:{}:{}", sym_norm.to_uppercase(), tf));
        }
        // Crypto slash/no-slash variants
        if !sym_alt.is_empty() {
            let alt_candidates = dsm.resolve_candidates(&sym_alt, tf);
            candidates.extend(alt_candidates);
        }

        let prefer_fresh_equity = chart_prefers_fresh_equity_source(&sym_norm);
        let mut best_equity: Option<(String, i64, u8)> = None;
        for key in &candidates {
            if let Ok(Some(raw)) = cache.get_bars_raw(key) {
                if !raw.is_empty()
                    && chart_source_bars_match_timeframe(cache_source_from_key(key), tf, &raw)
                {
                    let source = cache_source_from_key(key);
                    if prefer_fresh_equity {
                        if let Some(rank) = chart_equity_source_rank(source) {
                            let last_ts = chart_bar_last_valid_ts(&raw);
                            let replace = best_equity
                                .as_ref()
                                .map(|(_, best_ts, best_rank)| {
                                    last_ts > *best_ts || (last_ts == *best_ts && rank < *best_rank)
                                })
                                .unwrap_or(true);
                            if replace {
                                best_equity = Some((key.clone(), last_ts, rank));
                            }
                            continue;
                        }
                    }
                    return key.clone();
                }
            }
        }

        // Fallback: partial-match search via SQL LIKE
        if let Ok(keys) = cache.search_keys(&sym, 32) {
            let tf_lower = tf.to_lowercase();
            for key in &keys {
                if key.to_lowercase().ends_with(&tf_lower) {
                    if let Ok(Some(raw)) = cache.get_bars_raw(key) {
                        if !raw.is_empty()
                            && chart_source_bars_match_timeframe(
                                cache_source_from_key(key),
                                tf,
                                &raw,
                            )
                        {
                            let source = cache_source_from_key(key);
                            if prefer_fresh_equity {
                                if let Some(rank) = chart_equity_source_rank(source) {
                                    let last_ts = chart_bar_last_valid_ts(&raw);
                                    let replace = best_equity
                                        .as_ref()
                                        .map(|(_, best_ts, best_rank)| {
                                            last_ts > *best_ts
                                                || (last_ts == *best_ts && rank < *best_rank)
                                        })
                                        .unwrap_or(true);
                                    if replace {
                                        best_equity = Some((key.clone(), last_ts, rank));
                                    }
                                    continue;
                                }
                            }
                            return key.clone();
                        }
                    }
                }
            }
        }

        if let Some((key, _, _)) = best_equity {
            return key;
        }

        // Default fallback: first source in priority order
        format!("kraken:{}:{}", sym, tf)
    }

    /// Fast cache key without any DB probing. Used by try_load to avoid blocking.
    /// Try to load bars without blocking. Returns false if lock is contended.
    /// Use this from the UI thread render loop to avoid freezing.
    /// Load bars from cache. read_conn is exclusively owned by the UI thread,
    /// so lock() always succeeds immediately — no contention possible.
    /// Returns true if data was loaded (even if empty), false only on error.
    pub(crate) fn try_load(
        &mut self,
        cache: &SqliteCache,
        log: &mut VecDeque<LogEntry>,
        gpu: Option<&mut gpu_compute::GpuCompute>,
    ) -> bool {
        // Data priority mirrors DataSourceManager's default order:
        // Kraken spot/xStocks → Kraken Futures → Alpaca fallback.
        let sym = {
            let parts: Vec<&str> = self.symbol.split(':').collect();
            let is_tf = matches!(
                parts.last().copied(),
                Some(
                    "1Min"
                        | "5Min"
                        | "15Min"
                        | "30Min"
                        | "1Hour"
                        | "4Hour"
                        | "1Day"
                        | "1Week"
                        | "1Month"
                )
            );
            if is_tf && parts.len() > 1 {
                parts[..parts.len() - 1].join(":")
            } else {
                self.symbol.clone()
            }
        };
        let sym_norm = normalize_market_data_symbol(&sym);
        let tf = self.timeframe.cache_suffix();
        let old_bars_empty = self.bars.is_empty();
        let old_len = self.bars.len();
        let source_override = self.source_override;
        if source_override == "merged" {
            let (load_started_at, load_rss_before) = chart_log_merged_cache_load_start(
                log,
                "source_override",
                &sym,
                self.timeframe.label(),
            );
            let merged = chart_load_merged_equity_bars_from_cache(cache, &sym, tf);
            chart_log_merged_cache_load_done(
                log,
                "source_override",
                &sym,
                self.timeframe.label(),
                merged.len(),
                load_started_at,
                load_rss_before,
            );
            if !merged.is_empty() {
                self.gap_fill_timestamps.clear();
                self.bars = merged;
                self.primary_source = "merged";
                self.source_override = source_override;
                self.primary_first_ts = self.bars.first().map(|bar| bar.ts_ms).unwrap_or(0);
                if old_bars_empty {
                    self.view_offset = self.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
                    self.manual_view_override = false;
                    self.reset_camera_from_legacy();
                } else {
                    self.camera.on_data_len_changed(old_len, self.bars.len());
                    self.sync_camera_to_legacy();
                }
                self.compute_indicators_gpu(gpu);
            } else {
                self.bars.clear();
                self.primary_source = "";
                self.source_override = source_override;
                self.gap_fill_timestamps.clear();
                self.compute_indicators_gpu(gpu);
            }
            return true;
        }
        if !source_override.is_empty() {
            let mut result: Option<Vec<(i64, f64, f64, f64, f64, f64)>> = None;
            for key in chart_source_cache_keys(source_override, &sym, tf) {
                match cache.get_bars_raw(&key) {
                    Ok(Some(raw))
                        if !raw.is_empty()
                            && chart_source_bars_match_timeframe(source_override, tf, &raw) =>
                    {
                        result = Some(raw);
                        break;
                    }
                    _ => {}
                }
            }
            if let Some(raw) = result {
                self.gap_fill_timestamps.clear();
                self.bars = raw
                    .into_iter()
                    .filter(|(ts, o, h, l, c, _v)| {
                        *ts > 0
                            && *o > 0.0
                            && *h > 0.0
                            && *l > 0.0
                            && *c > 0.0
                            && o.is_finite()
                            && h.is_finite()
                            && l.is_finite()
                            && c.is_finite()
                            && *h >= *l
                    })
                    .map(|(ts, o, h, l, c, v)| Bar {
                        ts_ms: ts,
                        open: o,
                        high: h,
                        low: l,
                        close: c,
                        volume: v,
                    })
                    .collect();
                self.primary_source = if self.bars.is_empty() {
                    ""
                } else {
                    source_override
                };
                self.source_override = source_override;
                self.primary_first_ts = self.bars.first().map(|bar| bar.ts_ms).unwrap_or(0);
                if old_bars_empty {
                    self.view_offset = self.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
                    self.manual_view_override = false;
                    self.reset_camera_from_legacy();
                } else {
                    self.camera.on_data_len_changed(old_len, self.bars.len());
                    self.sync_camera_to_legacy();
                }
                self.compute_indicators_gpu(gpu);
            } else {
                self.bars.clear();
                self.primary_source = "";
                self.source_override = source_override;
                self.gap_fill_timestamps.clear();
                self.compute_indicators_gpu(gpu);
            }
            return true;
        }
        if chart_prefers_fresh_equity_source(&sym_norm) {
            let (load_started_at, load_rss_before) = chart_log_merged_cache_load_start(
                log,
                "fresh_equity_auto",
                &sym,
                self.timeframe.label(),
            );
            let merged = chart_load_merged_equity_bars_from_cache(cache, &sym, tf);
            chart_log_merged_cache_load_done(
                log,
                "fresh_equity_auto",
                &sym,
                self.timeframe.label(),
                merged.len(),
                load_started_at,
                load_rss_before,
            );
            if !merged.is_empty() {
                self.gap_fill_timestamps.clear();
                self.bars = merged;
                self.primary_source = "merged";
                self.primary_first_ts = self.bars.first().map(|bar| bar.ts_ms).unwrap_or(0);
                if old_bars_empty {
                    self.view_offset = self.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
                    self.manual_view_override = false;
                    self.reset_camera_from_legacy();
                } else {
                    self.camera.on_data_len_changed(old_len, self.bars.len());
                    self.sync_camera_to_legacy();
                }
                self.compute_indicators_gpu(gpu);
                return true;
            }
        }
        let mut keys_to_try = vec![
            format!("kraken:{}:{}", sym, tf),
            format!("kraken-equities:{}:{}", sym, tf),
            format!("kraken-futures:{}:{}", sym, tf),
            format!("alpaca:{}:{}", sym, tf),
            format!("yahoo-chart:{}:{}", sym, tf),
        ];
        if sym_norm != sym {
            keys_to_try.extend([
                format!("kraken:{}:{}", sym_norm, tf),
                format!("kraken-equities:{}:{}", sym_norm, tf),
                format!("kraken-futures:{}:{}", sym_norm, tf),
                format!("alpaca:{}:{}", sym_norm, tf),
                format!("yahoo-chart:{}:{}", sym_norm, tf),
            ]);
        }
        let prefer_fresh_equity = chart_prefers_fresh_equity_source(&sym_norm);
        let native_equity_low_tf_only =
            prefer_fresh_equity && chart_equity_low_timeframe_requires_native_source(tf);
        let mut result: Option<(Vec<(i64, f64, f64, f64, f64, f64)>, bool, &'static str)> = None;
        let mut best_equity: Option<(
            Vec<(i64, f64, f64, f64, f64, f64)>,
            bool,
            &'static str,
            i64,
            u8,
        )> = None;
        for k in &keys_to_try {
            match cache.get_bars_raw(k) {
                Ok(Some(raw))
                    if !raw.is_empty()
                        && chart_source_bars_match_timeframe(
                            cache_source_from_key(k),
                            tf,
                            &raw,
                        ) =>
                {
                    let source = cache_source_from_key(k);
                    let is_gap_fill = k.starts_with("kraken:") || k.starts_with("kraken-futures:");
                    if prefer_fresh_equity {
                        if native_equity_low_tf_only && source != "kraken-equities" {
                            continue;
                        }
                        if let Some(rank) = chart_equity_source_rank(source) {
                            let last_ts = chart_bar_last_valid_ts(&raw);
                            let replace = best_equity
                                .as_ref()
                                .map(|(_, _, _, best_ts, best_rank)| {
                                    last_ts > *best_ts || (last_ts == *best_ts && rank < *best_rank)
                                })
                                .unwrap_or(true);
                            if replace {
                                best_equity = Some((raw, is_gap_fill, source, last_ts, rank));
                            }
                            continue;
                        }
                    }
                    result = Some((raw, is_gap_fill, source));
                    break;
                }
                _ => {}
            }
        }
        if result.is_none() {
            if let Some((raw, is_gap_fill, source, _, _)) = best_equity {
                result = Some((raw, is_gap_fill, source));
            }
        }
        if result.is_none() && tf == "1Month" {
            let monthly_sources = [
                "kraken",
                "kraken-equities",
                "kraken-futures",
                "alpaca",
                "yahoo-chart",
                "default",
            ];
            for source in monthly_sources {
                for key in chart_source_cache_keys(source, &sym, "1Day") {
                    let Ok(Some(raw)) = cache.get_bars_raw(&key) else {
                        continue;
                    };
                    let monthly = Self::aggregate_daily_raw_to_monthly(raw);
                    if monthly.len() >= 2 {
                        self.bars = monthly;
                        self.primary_source = "merged";
                        self.primary_first_ts = self.bars.first().map(|bar| bar.ts_ms).unwrap_or(0);
                        self.gap_fill_timestamps.clear();
                        self.compute_indicators_gpu(gpu);
                        return true;
                    }
                }
            }
        }
        if let Some((raw, primary_is_gap_fill, primary_source)) = result {
            self.gap_fill_timestamps.clear();
            // Filter invalid bars (parity with load() at ~line 1842) — epoch-0 timestamps,
            // non-positive prices, NaN, or high<low would otherwise render as phantom
            // flat lines on the non-blocking UI hot path before load() runs.
            self.bars = raw
                .into_iter()
                .filter(|(ts, o, h, l, c, _v)| {
                    *ts > 0
                        && *o > 0.0
                        && *h > 0.0
                        && *l > 0.0
                        && *c > 0.0
                        && o.is_finite()
                        && h.is_finite()
                        && l.is_finite()
                        && c.is_finite()
                        && *h >= *l
                })
                .map(|(ts, o, h, l, c, v)| {
                    if primary_is_gap_fill {
                        self.gap_fill_timestamps.insert(ts);
                    }
                    Bar {
                        ts_ms: ts,
                        open: o,
                        high: h,
                        low: l,
                        close: c,
                        volume: v,
                    }
                })
                .collect();
            self.primary_source = if self.bars.is_empty() {
                ""
            } else {
                primary_source
            };

            // Track primary source range (bars before this are backfill)
            self.primary_first_ts = if primary_is_gap_fill {
                0
            } else {
                self.bars.first().map(|b| b.ts_ms).unwrap_or(0)
            };

            let mut gap_filled = 0usize;
            {
                // Merge provenance-tagged alternate-source bars without duplicating
                // the same D/W/M session. Providers do not agree on candle
                // timestamps: Kraken often uses 00:00 UTC, Alpaca/Yahoo US
                // equities use 04:00/05:00 UTC, and live daily candles can
                // arrive at close time. Use calendar buckets for higher
                // timeframes and offset aliases for intraday UTC+2/US
                // market-time variants.
                let tf_ms = match tf {
                    "4Hour" => 4 * 3_600_000,
                    "1Hour" => 3_600_000,
                    "30Min" => 1_800_000,
                    "15Min" => 900_000,
                    "5Min" => 300_000,
                    _ => 60_000,
                };
                let snap = |ts: i64| -> i64 {
                    match tf {
                        "1Month" => chrono::DateTime::from_timestamp_millis(ts)
                            .and_then(|dt| {
                                chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month(), 1)
                                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                            })
                            .map(|ndt| ndt.and_utc().timestamp_millis())
                            .unwrap_or(ts),
                        "1Week" => chrono::DateTime::from_timestamp_millis(ts)
                            .and_then(|dt| {
                                let days_since_mon = dt.weekday().num_days_from_monday() as i64;
                                (dt.date_naive() - chrono::Duration::days(days_since_mon))
                                    .and_hms_opt(0, 0, 0)
                            })
                            .map(|ndt| ndt.and_utc().timestamp_millis())
                            .unwrap_or(ts),
                        "1Day" => chrono::DateTime::from_timestamp_millis(ts)
                            .and_then(|dt| {
                                chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month(), dt.day())
                                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                            })
                            .map(|ndt| ndt.and_utc().timestamp_millis())
                            .unwrap_or(ts),
                        _ => ts / tf_ms * tf_ms,
                    }
                };
                let alias_offsets_ms: &[i64] = if matches!(tf, "1Day" | "1Week" | "1Month") {
                    &[0]
                } else {
                    &[
                        0,
                        2 * 3_600_000,
                        -2 * 3_600_000,
                        4 * 3_600_000,
                        -4 * 3_600_000,
                        5 * 3_600_000,
                        -5 * 3_600_000,
                    ]
                };
                let mut occupied: std::collections::HashSet<i64> = std::collections::HashSet::new();
                let mut primary_min_snapped: Option<i64> = None;
                let mut primary_max_snapped: Option<i64> = None;
                for b in self.bars.iter() {
                    let snapped = snap(b.ts_ms);
                    primary_min_snapped =
                        Some(primary_min_snapped.map_or(snapped, |min| min.min(snapped)));
                    primary_max_snapped =
                        Some(primary_max_snapped.map_or(snapped, |max| max.max(snapped)));
                    for offset in alias_offsets_ms {
                        occupied.insert(snap(b.ts_ms.saturating_add(*offset)));
                    }
                }
                self.gap_fill_timestamps.clear();
                // Try all alternate source prefixes for gap-fill (crypto slash variants too)
                let sym_slash = {
                    let s = sym.to_uppercase();
                    let crypto_bases = [
                        "BTC", "ETH", "SOL", "DOGE", "XRP", "ADA", "LTC", "LINK", "AVAX", "DOT",
                        "XMR", "ZEC", "DASH",
                    ];
                    crypto_bases
                        .iter()
                        .find_map(|base| {
                            if s.starts_with(base)
                                && s.ends_with("USD")
                                && s.len() == base.len() + 3
                            {
                                Some(format!("{}/USD", base))
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default()
                };
                let gap_prefixes = ["kraken", "kraken-futures", "alpaca", "yahoo-chart"];
                for prefix in &gap_prefixes {
                    // Try both SOLUSD and SOL/USD key forms
                    let keys_to_try: Vec<String> = if sym_slash.is_empty() {
                        vec![format!("{}:{}:{}", prefix, sym, tf)]
                    } else {
                        vec![
                            format!("{}:{}:{}", prefix, sym, tf),
                            format!("{}:{}:{}", prefix, sym_slash, tf),
                        ]
                    };
                    for gap_key in &keys_to_try {
                        if let Ok(Some(gap_raw)) = cache.get_bars_raw(gap_key) {
                            if !chart_source_bars_match_timeframe(
                                cache_source_from_key(gap_key),
                                tf,
                                &gap_raw,
                            ) {
                                continue;
                            }
                            for (ts, o, h, l, c, v) in gap_raw {
                                let snapped = snap(ts);
                                if !occupied.contains(&snapped)
                                    && chart_gap_fill_bar_allowed(
                                        primary_source,
                                        cache_source_from_key(gap_key),
                                        snapped,
                                        primary_min_snapped,
                                        primary_max_snapped,
                                    )
                                {
                                    for offset in alias_offsets_ms {
                                        occupied.insert(snap(ts.saturating_add(*offset)));
                                    }
                                    self.bars.push(Bar {
                                        ts_ms: ts,
                                        open: o,
                                        high: h,
                                        low: l,
                                        close: c,
                                        volume: v,
                                    });
                                    self.gap_fill_timestamps.insert(ts);
                                    gap_filled += 1;
                                }
                            }
                        }
                    }
                }
                if gap_filled > 0 {
                    self.bars.sort_by_key(|b| b.ts_ms);
                }
            }

            let agg_info = String::new(); // custom TFs removed

            let ltf_rebuilt = self.rebuild_from_lower_timeframe_if_dislocated(cache, &sym);
            let quote_overlaid = self.apply_quote_cache_overlay(cache, &sym);
            if old_bars_empty {
                self.view_offset = self.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
                self.manual_view_override = false;
                self.reset_camera_from_legacy();
            } else {
                self.camera.on_data_len_changed(old_len, self.bars.len());
                self.sync_camera_to_legacy();
            }
            self.compute_indicators_gpu(gpu);
            self.compute_mtf_sma(cache);
            self.compute_multi_kama(cache);
            self.compute_prev_candle_levels_native(cache);
            let mtf_info = if !self.mtf_sma.is_empty() || !self.multi_kama.is_empty() {
                format!(
                    " | MTF_MA: {} lines, MultiKAMA: {} TFs",
                    self.mtf_sma.len(),
                    self.multi_kama.len()
                )
            } else {
                String::new()
            };
            let gap_info = if gap_filled > 0 {
                format!(" +{} gap-fill", gap_filled)
            } else if ltf_rebuilt {
                " +LTF rebuild +quote".to_string()
            } else if quote_overlaid {
                " +quote".to_string()
            } else {
                String::new()
            };
            log.push_back(LogEntry::info(format!(
                "Loaded {} bars for {} [{}]{}{}{}",
                self.bars.len(),
                self.symbol,
                self.timeframe.label(),
                agg_info,
                mtf_info,
                gap_info
            )));
        }
        true
    }

    /// Load bars from the shared cache, re-compute indicators.
    pub(crate) fn load(
        &mut self,
        cache: &SqliteCache,
        log: &mut VecDeque<LogEntry>,
        gpu: Option<&mut gpu_compute::GpuCompute>,
        dsm: &typhoon_engine::core::data_source::DataSourceManager,
    ) {
        let key = self.find_cache_key(cache, dsm);
        let key_source = cache_source_from_key(&key);
        let tf = self.timeframe.cache_suffix();

        // Extract bare symbol for multi-source lookup
        let bare_sym = {
            let parts: Vec<&str> = self.symbol.split(':').collect();
            let is_tf = matches!(
                parts.last().copied(),
                Some(
                    "1Min"
                        | "5Min"
                        | "15Min"
                        | "30Min"
                        | "1Hour"
                        | "4Hour"
                        | "1Day"
                        | "1Week"
                        | "1Month"
                )
            );
            let sym_parts = if is_tf && parts.len() > 1 {
                &parts[..parts.len() - 1]
            } else {
                &parts[..]
            };
            let s = sym_parts.last().copied().unwrap_or(&self.symbol);
            // Strip known prefixes
            let known = [
                "default:",
                "kraken-futures:",
                "kraken-equities:",
                "kraken:",
                "alpaca:",
                "yahoo-chart:",
                "paper_TyphooN:",
                "alpaca_paper_TyphooN:",
            ];
            let mut r = s;
            for pfx in &known {
                if r.starts_with(pfx) {
                    r = &r[pfx.len()..];
                    break;
                }
            }
            r.split(':').last().unwrap_or(r).replace('/', "")
        };

        if chart_prefers_fresh_equity_source(&bare_sym) {
            let (load_started_at, load_rss_before) = chart_log_merged_cache_load_start(
                log,
                "restored_cache",
                &bare_sym,
                self.timeframe.label(),
            );
            let merged = chart_load_merged_equity_bars_from_cache(cache, &bare_sym, tf);
            chart_log_merged_cache_load_done(
                log,
                "restored_cache",
                &bare_sym,
                self.timeframe.label(),
                merged.len(),
                load_started_at,
                load_rss_before,
            );
            if !merged.is_empty() {
                self.gap_fill_timestamps.clear();
                self.bars = merged;
                self.primary_source = "merged";
                self.primary_first_ts = self.bars.first().map(|bar| bar.ts_ms).unwrap_or(0);
                self.compute_indicators_gpu(gpu);
                self.compute_mtf_sma(cache);
                self.compute_multi_kama(cache);
                self.compute_prev_candle_levels_native(cache);
                return;
            }
        }

        // Load primary source (filter invalid bars at read time)
        match cache.get_bars_raw(&key) {
            Ok(Some(raw)) if chart_source_bars_match_timeframe(key_source, tf, &raw) => {
                self.bars = raw
                    .into_iter()
                    .filter(|(ts, o, h, l, c, _v)| {
                        *ts > 0 && *o > 0.0 && *h > 0.0 && *l > 0.0 && *c > 0.0 && *h >= *l
                    })
                    .map(|(ts, o, h, l, c, v)| Bar {
                        ts_ms: ts,
                        open: o,
                        high: h,
                        low: l,
                        close: c,
                        volume: v,
                    })
                    .collect();
                self.primary_source = if self.bars.is_empty() { "" } else { key_source };
            }
            Ok(Some(_)) | Ok(None) => {
                self.bars.clear();
                self.primary_source = "";
                if tf == "1Month" {
                    for source in [
                        "kraken",
                        "kraken-equities",
                        "kraken-futures",
                        "alpaca",
                        "yahoo-chart",
                        "default",
                    ] {
                        for daily_key in chart_source_cache_keys(source, &bare_sym, "1Day") {
                            let Ok(Some(raw)) = cache.get_bars_raw(&daily_key) else {
                                continue;
                            };
                            let monthly = Self::aggregate_daily_raw_to_monthly(raw);
                            if monthly.len() >= 2 {
                                self.bars = monthly;
                                self.primary_source = "merged";
                                break;
                            }
                        }
                        if !self.bars.is_empty() {
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                self.bars.clear();
                self.primary_source = "";
                log.push_back(LogEntry::err(format!("Cache read error: {e}")));
            }
        }

        // Merge gap-fill sources: Kraken spot/xStocks and Futures
        // For crypto: merge ALL bars (fill gaps anywhere, not just append to end)
        let crypto_bases = [
            "BTC", "ETH", "SOL", "DOGE", "XRP", "ADA", "LTC", "LINK", "AVAX", "DOT", "XMR", "ZEC",
            "DASH", "UNI", "AAVE", "MATIC", "SHIB", "ATOM", "ALGO", "FTM", "NEAR", "APE", "ARB",
        ];
        let sym_upper = bare_sym.to_uppercase();
        let is_crypto = crypto_bases
            .iter()
            .any(|b| sym_upper.starts_with(b) && sym_upper.ends_with("USD"));

        if is_crypto {
            self.gap_fill_timestamps.clear();
            // Snap timestamps to TF boundary for dedup (handles per-source TZ offsets vs Kraken UTC)
            // Weekly: snap to Monday 00:00 UTC. Monthly: snap to 1st of month 00:00 UTC.
            let tf_ms: i64 = match tf {
                "1Day" => 86_400_000,
                "4Hour" => 4 * 3_600_000,
                "1Hour" => 3_600_000,
                "30Min" => 1_800_000,
                "15Min" => 900_000,
                "5Min" => 300_000,
                _ => 60_000,
            };
            let snap = |ts: i64| -> i64 {
                match tf {
                    "1Month" => {
                        // Snap to 1st of month 00:00 UTC
                        let dt = chrono::DateTime::from_timestamp(ts / 1000, 0).unwrap_or_default();
                        chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month(), 1)
                            .and_then(|d| d.and_hms_opt(0, 0, 0))
                            .map(|ndt| ndt.and_utc().timestamp() * 1000)
                            .unwrap_or(ts / tf_ms * tf_ms)
                    }
                    "1Week" => {
                        // Snap to Monday 00:00 UTC
                        let dt = chrono::DateTime::from_timestamp(ts / 1000, 0).unwrap_or_default();
                        let days_since_mon = dt.weekday().num_days_from_monday() as i64;
                        let mon = dt.date_naive() - chrono::Duration::days(days_since_mon);
                        mon.and_hms_opt(0, 0, 0)
                            .map(|ndt| ndt.and_utc().timestamp() * 1000)
                            .unwrap_or(ts / (7 * 86_400_000) * (7 * 86_400_000))
                    }
                    _ => ts / tf_ms * tf_ms,
                }
            };
            let mut existing_snapped: std::collections::HashSet<i64> =
                self.bars.iter().map(|b| snap(b.ts_ms)).collect();

            let kr_key = format!("kraken:{}:{}", bare_sym, tf);
            if let Ok(Some(raw)) = cache.get_bars_raw(&kr_key) {
                let mut merged = 0;
                for (ts, o, h, l, c, v) in raw {
                    if o <= 0.0 || h <= 0.0 || l <= 0.0 || c <= 0.0 || h < l {
                        continue;
                    }
                    let snapped = snap(ts);
                    if !existing_snapped.contains(&snapped) {
                        self.bars.push(Bar {
                            ts_ms: ts,
                            open: o,
                            high: h,
                            low: l,
                            close: c,
                            volume: v,
                        });
                        self.gap_fill_timestamps.insert(ts);
                        existing_snapped.insert(snapped);
                        merged += 1;
                    }
                }
                if merged > 0 {
                    log.push_back(LogEntry::info(format!(
                        "  +{} bars from Kraken weekend fill",
                        merged
                    )));
                }
            }

            let kr_fut_key = format!("kraken-futures:{}:{}", bare_sym, tf);
            if let Ok(Some(raw)) = cache.get_bars_raw(&kr_fut_key) {
                let mut merged = 0;
                for (ts, o, h, l, c, v) in raw {
                    if o <= 0.0 || h <= 0.0 || l <= 0.0 || c <= 0.0 || h < l {
                        continue;
                    }
                    let snapped = snap(ts);
                    if !existing_snapped.contains(&snapped) {
                        self.bars.push(Bar {
                            ts_ms: ts,
                            open: o,
                            high: h,
                            low: l,
                            close: c,
                            volume: v,
                        });
                        self.gap_fill_timestamps.insert(ts);
                        existing_snapped.insert(snapped);
                        merged += 1;
                    }
                }
                if merged > 0 {
                    log.push_back(LogEntry::info(format!(
                        "  +{} bars from Kraken Futures fill",
                        merged
                    )));
                }
            }

            // Sort merged bars by timestamp (sources may interleave)
            if !self.bars.is_empty() {
                self.bars.sort_by_key(|b| b.ts_ms);
            }
        }

        // Remove any bars with invalid prices (negative, zero, NaN, or obviously wrong)
        // Runs unconditionally on ALL bars from ALL sources
        {
            let pre_filter = self.bars.len();
            self.bars.retain(|b| {
                b.open > 0.0
                    && b.high > 0.0
                    && b.low > 0.0
                    && b.close > 0.0
                    && b.open.is_finite()
                    && b.high.is_finite()
                    && b.low.is_finite()
                    && b.close.is_finite()
                    && b.high >= b.low
                    && b.ts_ms > 0
            });
            if self.bars.len() < pre_filter {
                log.push_back(LogEntry::warn(format!(
                    "  Filtered {} invalid bars (negative/zero/NaN/bad prices)",
                    pre_filter - self.bars.len()
                )));
            }
        }

        // Synthesize a current forming bar from lower-timeframe data only when
        // the existing HTF series is caught up through the previous bucket. If
        // the primary source is several sessions stale, aggregating every newer
        // LTF candle into one bar creates a fake multi-day monster candle.
        if !self.bars.is_empty() && self.timeframe.minutes() > 5 {
            let last_ts = self.bars.last().map(|b| b.ts_ms).unwrap_or(0);
            let tf_ms = self.timeframe.minutes() as i64 * 60 * 1000;
            let now_ms = chrono::Utc::now().timestamp_millis();
            if chart_forming_bar_allowed(last_ts, now_ms, tf_ms) {
                let current_bucket = now_ms / tf_ms * tf_ms;
                // Try M5 first, then M1
                // Cascade through all lower timeframes from all sources for best resolution
                let src = self.symbol.split(':').next().unwrap_or("kraken");
                let ltf_suffixes = [
                    "1Min", "5Min", "15Min", "30Min", "1Hour", "4Hour", "1Day", "1Week",
                ];
                let sources = ["kraken", "kraken-futures", src, "kraken-equities", "alpaca"];
                let mut ltf_keys = Vec::new();
                for ltf in &ltf_suffixes {
                    let ltf_min: u32 = match *ltf {
                        "1Min" => 1,
                        "5Min" => 5,
                        "15Min" => 15,
                        "30Min" => 30,
                        "1Hour" => 60,
                        "4Hour" => 240,
                        "1Day" => 1440,
                        _ => 10080,
                    };
                    if ltf_min < self.timeframe.minutes() {
                        for s in &sources {
                            ltf_keys.push(format!("{}:{}:{}", s, bare_sym, ltf));
                        }
                    }
                }
                for ltf_key in &ltf_keys {
                    if let Ok(Some(ltf_raw)) = cache.get_bars_raw(ltf_key) {
                        // Find LTF bars inside the current HTF bucket only.
                        let forming_start = current_bucket;
                        let forming_end = forming_start.saturating_add(tf_ms);
                        let newer: Vec<_> = ltf_raw
                            .iter()
                            .filter(|(ts, _, _, _, _, _)| *ts >= forming_start && *ts < forming_end)
                            .collect();
                        if !newer.is_empty() {
                            let open = newer.first().map(|(_, o, _, _, _, _)| *o).unwrap_or(0.0);
                            let high = newer
                                .iter()
                                .map(|(_, _, h, _, _, _)| *h)
                                .fold(f64::NEG_INFINITY, f64::max);
                            let low = newer
                                .iter()
                                .map(|(_, _, _, l, _, _)| *l)
                                .fold(f64::INFINITY, f64::min);
                            let close = newer.last().map(|(_, _, _, _, c, _)| *c).unwrap_or(0.0);
                            let volume: f64 = newer.iter().map(|(_, _, _, _, _, v)| *v).sum();
                            self.bars.push(Bar {
                                ts_ms: forming_start,
                                open,
                                high,
                                low,
                                close,
                                volume,
                            });
                            log.push_back(LogEntry::info(format!(
                                "  +1 forming bar from {} LTF bars",
                                newer.len()
                            )));
                            break;
                        }
                    }
                }
            }
        }

        let ltf_rebuilt = self.rebuild_from_lower_timeframe_if_dislocated(cache, &bare_sym);
        let quote_overlaid = self.apply_quote_cache_overlay(cache, &bare_sym);

        if self.bars.is_empty() {
            log.push_back(LogEntry::warn(format!(
                "No chart data found for key '{}'",
                key
            )));
        } else {
            self.view_offset = self.bars.len().saturating_sub(1) + CHART_RIGHT_MARGIN;
            self.compute_indicators_gpu(gpu);
            self.compute_mtf_sma(cache);
            self.compute_multi_kama(cache);
            self.compute_prev_candle_levels_native(cache);
            let mtf_info = if !self.mtf_sma.is_empty() || !self.multi_kama.is_empty() {
                format!(
                    " | MTF_MA: {} lines, MultiKAMA: {} TFs",
                    self.mtf_sma.len(),
                    self.multi_kama.len()
                )
            } else {
                String::new()
            };
            let quote_info = if ltf_rebuilt {
                " +LTF rebuild +quote"
            } else if quote_overlaid {
                " +quote"
            } else {
                ""
            };
            log.push_back(LogEntry::info(format!(
                "Loaded {} bars for {} [{}]{}{}",
                self.bars.len(),
                self.symbol,
                self.timeframe.label(),
                mtf_info,
                quote_info
            )));
        }
        // Steady-state cap of 200 (was 500). Console log is diagnostic, not forensic —
        // keep it tight to avoid frame jank during bulk imports that push dozens of lines.
        while log.len() > 200 {
            log.pop_front();
        }
    }
}
