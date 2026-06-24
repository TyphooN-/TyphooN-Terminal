use super::*;

// ─── Shared MTF higher-timeframe bar cache ───────────────────────────────────
// MTF_MA / MultiKAMA overlays and the right-panel MTF Grid both need a symbol's
// higher-timeframe (H1/H4/D1/W1/MN1) bars. Rather than each re-reading + parsing
// from SQLite, the MTF Grid loader (a canonical DataSourceManager load, off the
// render thread) publishes the bars it loads to this process-global memo keyed by
// `(mtf_grid_symbol_key(symbol), Timeframe::cache_suffix)`; the overlay loaders
// read it first and fall back to their own source-pinned load on a miss. A short
// TTL bounds staleness without explicit invalidation — HTF bars change at most
// once per (long) period, so a freshly-fetched bar is reflected within the TTL.
struct MtfHtfCacheEntry {
    bars: std::sync::Arc<Vec<Bar>>,
    written_ms: i64,
}

#[allow(clippy::type_complexity)]
fn mtf_htf_cache()
-> &'static std::sync::RwLock<std::collections::HashMap<(String, String), MtfHtfCacheEntry>> {
    static CACHE: std::sync::OnceLock<
        std::sync::RwLock<std::collections::HashMap<(String, String), MtfHtfCacheEntry>>,
    > = std::sync::OnceLock::new();
    CACHE.get_or_init(|| std::sync::RwLock::new(std::collections::HashMap::new()))
}

const MTF_HTF_CACHE_TTL_MS: i64 = 90_000;

/// Read a symbol's higher-timeframe bars from the shared MTF cache when a fresh
/// (within-TTL) entry exists. Key: `(mtf_grid_symbol_key(symbol), tf_suffix)`.
pub(crate) fn mtf_htf_cache_get(
    symbol_key: &str,
    tf_suffix: &str,
    now_ms: i64,
) -> Option<std::sync::Arc<Vec<Bar>>> {
    let guard = mtf_htf_cache().read().ok()?;
    let entry = guard.get(&(symbol_key.to_string(), tf_suffix.to_string()))?;
    (now_ms.saturating_sub(entry.written_ms) < MTF_HTF_CACHE_TTL_MS)
        .then(|| std::sync::Arc::clone(&entry.bars))
}

/// Publish a symbol's higher-timeframe bars to the shared MTF cache (called by the
/// MTF Grid background loader). Opportunistically prunes stale entries to bound
/// growth; the map only ever holds the active symbol's recently-loaded timeframes.
pub(crate) fn mtf_htf_cache_put(
    symbol_key: &str,
    tf_suffix: &str,
    bars: std::sync::Arc<Vec<Bar>>,
    now_ms: i64,
) {
    if let Ok(mut guard) = mtf_htf_cache().write() {
        guard.retain(|_, e| now_ms.saturating_sub(e.written_ms) < MTF_HTF_CACHE_TTL_MS);
        guard.insert(
            (symbol_key.to_string(), tf_suffix.to_string()),
            MtfHtfCacheEntry {
                bars,
                written_ms: now_ms,
            },
        );
    }
}

#[cfg(test)]
mod mtf_htf_cache_tests {
    use super::*;

    #[test]
    fn shared_htf_cache_round_trips_within_ttl_then_expires() {
        let bars = std::sync::Arc::new(vec![Bar {
            ts_ms: 1,
            open: 1.0,
            high: 1.0,
            low: 1.0,
            close: 1.0,
            volume: 1.0,
        }]);
        // Unique key so the process-global cache can't collide with other tests.
        let now = 1_000_000_000_000i64;
        mtf_htf_cache_put("ZZSHAREDTEST", "1Week", std::sync::Arc::clone(&bars), now);
        assert!(
            mtf_htf_cache_get("ZZSHAREDTEST", "1Week", now).is_some(),
            "fresh entry returned"
        );
        assert!(
            mtf_htf_cache_get("ZZSHAREDTEST", "1Week", now + MTF_HTF_CACHE_TTL_MS).is_none(),
            "entry expires at/after the TTL"
        );
        assert!(
            mtf_htf_cache_get("ZZSHAREDTEST", "1Day", now).is_none(),
            "a different timeframe suffix is a miss"
        );
    }
}

/// Multi-timeframe overlay computation (HTF MAs/KAMA, previous-candle levels) for a chart
/// viewport (ADR-125 Target 2). A native extension trait because it calls the native
/// `ChartIndicatorCompute` GPU path and the module-local HTF bar cache; `ChartState` lives
/// in `typhoon-chart-ui`. Re-exported from `chart` so call sites keep method syntax.
pub(crate) trait ChartMtfOverlays {
    fn load_mtf_htf_bars(
        &self,
        cache: &SqliteCache,
        bare_sym: &str,
        base_sym: &str,
        tf_suffix: &str,
    ) -> Option<Vec<Bar>>;
    fn mtf_line_scale_ok(bars: &[Bar], projected: &[(usize, f64)]) -> bool;
    fn htf_source_matches_host_scale(host: &[Bar], htf: &[Bar]) -> bool;
    fn compute_mtf_sma(&mut self, cache: &SqliteCache);
    fn ensure_mql_mtf_overlays_for_render(
        &mut self,
        cache: &SqliteCache,
        show_mtf_ma: bool,
        show_multi_kama: bool,
    );
    fn should_ensure_mql_mtf_overlays_for_render(
        heavy_sync_in_progress: bool,
        mtf_enabled: bool,
        is_focused: bool,
    ) -> bool;
    fn compute_multi_kama(&mut self, cache: &SqliteCache);
    fn mtf_base_and_bare_sym(&self) -> (String, String);
    fn compute_prev_candle_levels_native(&mut self, cache: &SqliteCache);
}
impl ChartMtfOverlays for ChartState {
    /// Load higher-timeframe bars for an MTF overlay (MTF_MA / MultiKAMA),
    /// preferring the SAME cache source the chart's candles loaded from so the
    /// overlay never mixes price scales / adjustments with the displayed bars
    /// (ADR-123, source consistency). When the chart's source is known we
    /// restrict to it and return `None` — dropping the line — if that timeframe
    /// is absent, rather than crossing to a differently-adjusted source. Only
    /// when the source is unknown (`""`) do we fall back to the legacy
    /// broad-prefix search (still scale-guarded by `mtf_line_scale_ok`).
    fn load_mtf_htf_bars(
        &self,
        cache: &SqliteCache,
        bare_sym: &str,
        base_sym: &str,
        tf_suffix: &str,
    ) -> Option<Vec<Bar>> {
        let try_key = |key: &str| -> Option<Vec<Bar>> {
            let raw = cache.get_bars_raw(key).ok().flatten()?;
            if !chart_source_bars_match_timeframe(cache_source_from_key(key), tf_suffix, &raw) {
                return None;
            }
            let bars: Vec<Bar> = raw
                .into_iter()
                .map(|(ts, o, h, l, c, v)| Bar {
                    ts_ms: ts,
                    open: o,
                    high: h,
                    low: l,
                    close: c,
                    volume: v,
                })
                .collect();
            // ADR-123 #3: reject an HTF source carrying a mis-scaled era vs the host
            // candles (unadjusted intraday across a split — YI's 1Hour/4Hour sat ~10×
            // high in the pre-split window while its 1Day/1Week were adjusted). Checked
            // at the BAR level, so a clean higher-TF source whose *lagging average*
            // legitimately rides above a crashed price (e.g. W1/200 over a −90% move)
            // is kept — its bars match scale; only the projected SMA lags.
            if !Self::htf_source_matches_host_scale(&self.bars, &bars) {
                return None;
            }
            Some(bars)
        };

        // Shared MTF cache (the chosen MTF_MA / MTF_Grid unification): prefer the
        // Grid loader's canonical bars so the two always agree and SQLite is read
        // once. Keep the host-scale guard so a mis-scaled era is still rejected; on
        // a miss or scale-reject, fall through to the source-pinned load below.
        let now_ms = chrono::Utc::now().timestamp_millis();
        let sym_key = super::chart_ops::mtf_grid_symbol_key(&self.symbol);
        if let Some(shared) = mtf_htf_cache_get(&sym_key, tf_suffix, now_ms) {
            if Self::htf_source_matches_host_scale(&self.bars, &shared) {
                return Some((*shared).clone());
            }
        }

        // ADR-123 #2: restrict to the candles' own source — canonical
        // "{source}:{sym}:{tf}" — and drop the line if that TF is absent there.
        if !self.primary_source.is_empty() {
            return try_key(&format!(
                "{}:{}:{}",
                self.primary_source, bare_sym, tf_suffix
            ));
        }

        // Unknown source: legacy broad search.
        let prefixes = [
            "merged:",
            "default:",
            "kraken-equities:",
            "kraken:",
            "kraken-futures:",
            "alpaca:",
            "yahoo-chart:",
            "paper_TyphooN:",
            "alpaca_paper_TyphooN:",
            "",
        ];
        for prefix in &prefixes {
            if let Some(bars) = try_key(&format!("{}{}:{}", prefix, bare_sym, tf_suffix)) {
                return Some(bars);
            }
        }
        if let Some(bars) = try_key(&format!("{}:{}", base_sym, tf_suffix)) {
            return Some(bars);
        }
        if let Ok(keys) = cache.search_keys(bare_sym, 32) {
            let tf_lower = tf_suffix.to_lowercase();
            for k in &keys {
                if k.to_lowercase().ends_with(&tf_lower) {
                    if let Some(bars) = try_key(k) {
                        return Some(bars);
                    }
                }
            }
        }
        None
    }

    /// ADR-123 #1: price-scale sanity guard. Rejects an MTF/KAMA overlay line
    /// whose values sit on a wildly different scale than the candles (e.g. an
    /// un-back-adjusted or pre-split feed). Uses the median `value / close` ratio
    /// at the matched bars: a legitimately lagging average has a median near 1,
    /// whereas a mis-scaled feed is persistently many-fold off. Kept when the
    /// median ratio is within `[1/SCALE_TOL, SCALE_TOL]`.
    fn mtf_line_scale_ok(bars: &[Bar], projected: &[(usize, f64)]) -> bool {
        // Raised from 4.0 to allow legitimate SMA lag on post-crash equities (WOK H1 SMA200 can be 20-100x price).
        // Prevents MTF_MA from being silently dropped while still catching grossly mis-scaled feeds.
        const SCALE_TOL: f64 = 100.0;
        let mut ratios: Vec<f64> = projected
            .iter()
            .filter_map(|&(i, v)| {
                let close = bars.get(i).map(|b| b.close).unwrap_or(0.0);
                (close > 0.0 && v.is_finite() && v > 0.0).then_some(v / close)
            })
            .collect();
        if ratios.is_empty() {
            return false;
        }
        ratios.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = ratios[ratios.len() / 2];
        (1.0 / SCALE_TOL..=SCALE_TOL).contains(&median)
    }

    /// ADR-123 #3: bar-level scale check for an MTF/KAMA higher-timeframe source.
    /// Rejects a source whose bars sit on a different price scale than the host
    /// candles over more than `MAX_OFFSCALE_FRAC` of their overlap — the signature
    /// of an unadjusted intraday era across a corporate action (YI's `1Hour`/`4Hour`
    /// ran ~10× the adjusted `1Day`/`1Week` in the pre-split window, since the
    /// intraday feed had no corroborator there to correct it).
    ///
    /// Distinct from [`Self::mtf_line_scale_ok`], which takes the **median** ratio of
    /// the projected average and so (by design) ignores a *localized* bad era. This
    /// looks at the **source bars** instead of the lagging average, so a clean
    /// higher-TF whose SMA legitimately rides far above a crashed price (a W1/200
    /// over a −90% move — expected lag, not a scale fault) is kept, while a feed that
    /// is genuinely mis-scaled for a sustained block of bars is dropped.
    fn htf_source_matches_host_scale(host: &[Bar], htf: &[Bar]) -> bool {
        const SCALE_TOL: f64 = 4.0;
        const MAX_OFFSCALE_FRAC: f64 = 0.08; // clean sources ~0–1%; mis-scaled eras ≥12%
        if host.len() < 2 {
            return true; // no host scale to validate against
        }
        let host_ts: Vec<i64> = host.iter().map(|b| b.ts_ms).collect();
        let (mut off, mut tot) = (0usize, 0usize);
        for b in htf {
            if b.close <= 0.0 || !b.close.is_finite() {
                continue;
            }
            // Host candle whose bucket contains this HTF bar (nearest prior close).
            let j = match host_ts.binary_search(&b.ts_ms) {
                Ok(k) => k,
                Err(0) => continue, // before the host range — nothing to compare against
                Err(k) => k - 1,
            };
            let hc = host[j].close;
            if hc <= 0.0 {
                continue;
            }
            tot += 1;
            let r = b.close / hc;
            if r < 1.0 / SCALE_TOL || r > SCALE_TOL {
                off += 1;
            }
        }
        tot == 0 || (off as f64 / tot as f64) <= MAX_OFFSCALE_FRAC
    }

    /// Compute MultiKAMA: load bars from higher timeframes and compute KAMA(10,2,30) on each.
    /// Projects KAMA values onto this chart's x-axis by matching timestamps.
    /// Compute MTF SMA lines matching MTF_MA.mqh behavior.
    /// Loads HTF bars from cache, computes SMA on them, projects onto current chart.
    /// Lines: H1/200, H4/200, D1/200, W1/200, W1/100, MN1/100
    fn compute_mtf_sma(&mut self, cache: &SqliteCache) {
        self.mtf_sma.clear();
        if self.bars.is_empty() {
            return;
        }

        let base_sym = {
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

        // (label, tf_suffix, sma_period, tf_minutes) — matching MTF_MA.mqh plotted lines
        let mtf_lines: &[(&str, &str, usize, u32)] = &[
            ("H1 200", "1Hour", 200, 60),
            ("H4 200", "4Hour", 200, 240),
            ("D1 200", "1Day", 200, 1440),
            ("W1 200", "1Week", 200, 10080),
            ("W1 100", "1Week", 100, 10080),
            ("MN1 100", "1Month", 100, 43200),
        ];

        // Extract bare symbol (strip ALL prefixes and timeframe)
        let bare_sym = {
            let known_prefixes = [
                "default:",
                "kraken-futures:",
                "kraken-equities:",
                "kraken:",
                "alpaca:",
                "yahoo-chart:",
                "paper_TyphooN:",
                "alpaca_paper_TyphooN:",
            ];
            let mut s = base_sym.as_str();
            for pfx in &known_prefixes {
                if s.starts_with(pfx) {
                    s = &s[pfx.len()..];
                    break;
                }
            }
            let parts: Vec<&str> = s.split(':').collect();
            parts
                .last()
                .copied()
                .unwrap_or(s)
                .replace('/', "")
                .trim_end_matches(".EQ")
                .to_string()
        };

        for &(label, tf_suffix, period, _tf_min) in mtf_lines {
            // 1:1 MT5 parity: MTF_MA.mqh declares all 6 plotted buffers as INDICATOR_DATA
            // (see MTF_MA.mqh lines 72-77) with no chart-period guard, so every line is
            // drawn on every host timeframe. We match that exactly — lower-TF lines
            // projected onto higher-TF bars are informationally thin but MT5-accurate.
            // ADR-123 #2: source-consistent load (prefers the candles' own source).
            let htf_bars = self.load_mtf_htf_bars(cache, &bare_sym, &base_sym, tf_suffix);

            if let Some(htf) = htf_bars {
                if htf.len() < period {
                    continue;
                }
                let sma_vals = compute_sma(&htf, period);

                // Project HTF SMA onto current chart bars via timestamp matching
                let mut projected: Vec<(usize, f64)> = Vec::new();
                let mut htf_idx = 0;
                for (i, bar) in self.bars.iter().enumerate() {
                    while htf_idx + 1 < htf.len() && htf[htf_idx + 1].ts_ms <= bar.ts_ms {
                        htf_idx += 1;
                    }
                    if htf_idx < sma_vals.len() {
                        if let Some(v) = sma_vals[htf_idx] {
                            projected.push((i, v));
                        }
                    }
                }

                // ADR-123 #1: drop the line if it sits on a mismatched price scale.
                if !projected.is_empty() && Self::mtf_line_scale_ok(&self.bars, &projected) {
                    self.mtf_sma.push((label.to_string(), projected));
                }
            }
        }
    }

    fn ensure_mql_mtf_overlays_for_render(
        &mut self,
        cache: &SqliteCache,
        show_mtf_ma: bool,
        show_multi_kama: bool,
    ) {
        if self.bars.is_empty() {
            return;
        }
        if show_mtf_ma && self.mtf_sma.is_empty() {
            self.compute_mtf_sma(cache);
        }
        if show_multi_kama && self.multi_kama.is_empty() {
            self.compute_multi_kama(cache);
        }
    }

    fn should_ensure_mql_mtf_overlays_for_render(
        heavy_sync_in_progress: bool,
        mtf_enabled: bool,
        is_focused: bool,
    ) -> bool {
        !heavy_sync_in_progress || !mtf_enabled || is_focused
    }

    fn compute_multi_kama(&mut self, cache: &SqliteCache) {
        self.multi_kama.clear();
        if self.bars.is_empty() {
            return;
        }

        // Extract base symbol (strip timeframe suffix from symbol)
        let base_sym = {
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

        let higher_tfs: &[(&str, &str, u32)] = &[
            ("H1", "1Hour", 60),
            ("H4", "4Hour", 240),
            ("D1", "1Day", 1440),
            ("W1", "1Week", 10080),
            ("MN1", "1Month", 43200),
        ];

        // Extract bare symbol (strip source prefixes like kraken:)
        let bare_sym = {
            let known_prefixes = [
                "default:",
                "kraken-futures:",
                "kraken-equities:",
                "kraken:",
                "alpaca:",
                "yahoo-chart:",
                "paper_TyphooN:",
                "alpaca_paper_TyphooN:",
            ];
            let mut s = base_sym.as_str();
            for pfx in &known_prefixes {
                if s.starts_with(pfx) {
                    s = &s[pfx.len()..];
                    break;
                }
            }
            let parts: Vec<&str> = s.split(':').collect();
            parts
                .last()
                .copied()
                .unwrap_or(s)
                .replace('/', "")
                .trim_end_matches(".EQ")
                .to_string()
        };

        for &(tf_label, tf_suffix, _tf_min) in higher_tfs {
            // 1:1 MT5 parity: MultiKAMA.mqh declares all 5 plotted buffers
            // (ExtAMABuffer_H1/H4/D1/W1/MN1) as INDICATOR_DATA with no chart-period
            // guard (see MultiKAMA.mqh lines 47-58), so every KAMA line is drawn on
            // every host timeframe. We match that exactly.
            // ADR-123 #2: source-consistent load (prefers the candles' own source).
            let htf_bars = self.load_mtf_htf_bars(cache, &bare_sym, &base_sym, tf_suffix);

            if let Some(htf) = htf_bars {
                if htf.len() < 12 {
                    continue;
                }
                // Compute KAMA(10,2,30) on higher TF bars
                let kama_vals = compute_kama(&htf, 10, 2, 30);

                // Map higher TF KAMA values onto this chart's bar indices by timestamp
                // For each of our bars, find the most recent HTF bar that's <= our timestamp
                let mut projected: Vec<(usize, f64)> = Vec::new();
                let mut htf_idx = 0;
                for (i, bar) in self.bars.iter().enumerate() {
                    while htf_idx + 1 < htf.len() && htf[htf_idx + 1].ts_ms <= bar.ts_ms {
                        htf_idx += 1;
                    }
                    if htf_idx < kama_vals.len() {
                        if let Some(k) = kama_vals[htf_idx] {
                            projected.push((i, k));
                        }
                    }
                }

                // ADR-123 #1: drop the line if it sits on a mismatched price scale.
                if !projected.is_empty() && Self::mtf_line_scale_ok(&self.bars, &projected) {
                    self.multi_kama.push((tf_label.to_string(), projected));
                }
            }
        }
    }

    /// `(base_sym, bare_sym)` used to locate this chart's higher-timeframe series
    /// in cache — mirrors the extraction in `compute_mtf_sma`/`compute_multi_kama`.
    fn mtf_base_and_bare_sym(&self) -> (String, String) {
        let base_sym = {
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
        let bare_sym = {
            let known_prefixes = [
                "default:",
                "kraken-futures:",
                "kraken-equities:",
                "kraken:",
                "alpaca:",
                "yahoo-chart:",
                "paper_TyphooN:",
                "alpaca_paper_TyphooN:",
            ];
            let mut s = base_sym.as_str();
            for pfx in &known_prefixes {
                if s.starts_with(pfx) {
                    s = &s[pfx.len()..];
                    break;
                }
            }
            let parts: Vec<&str> = s.split(':').collect();
            parts
                .last()
                .copied()
                .unwrap_or(s)
                .replace('/', "")
                .trim_end_matches(".EQ")
                .to_string()
        };
        (base_sym, bare_sym)
    }

    /// Refine previous/current candle levels from the **native per-timeframe**
    /// candles in cache, matching `PreviousCandleLevels.mqh`, which reads
    /// `iHigh(_Symbol, PERIOD_X, n)` from each timeframe's own series rather than
    /// re-aggregating the host chart's bars. For a 24/7 merged-source symbol (e.g.
    /// a Kraken xStock) the host H1 series need not fully cover each higher-TF
    /// period — gaps, partial sessions, or a cross-source scale era make the
    /// re-aggregated weekly/daily/H4 highs wrong. The native HTF candle is
    /// authoritative: its last bar is the current (forming) period and its
    /// second-to-last is the previous (last closed) period. Only overrides a level
    /// when its HTF series is present and passes `load_mtf_htf_bars`' scale guards;
    /// otherwise the aggregated value from `compute_indicators` is kept as a
    /// fallback. Cache-bound, so call from the load paths (not per render frame).
    fn compute_prev_candle_levels_native(&mut self, cache: &SqliteCache) {
        if self.bars.is_empty() {
            return;
        }
        let (base_sym, bare_sym) = self.mtf_base_and_bare_sym();
        // Owned loads first (each releases its &self borrow) so the field writes
        // below can take &mut self.
        let h1 = self.load_mtf_htf_bars(cache, &bare_sym, &base_sym, "1Hour");
        let h4 = self.load_mtf_htf_bars(cache, &bare_sym, &base_sym, "4Hour");
        let d1 = self.load_mtf_htf_bars(cache, &bare_sym, &base_sym, "1Day");
        let w1 = self.load_mtf_htf_bars(cache, &bare_sym, &base_sym, "1Week");
        let mn1 = self.load_mtf_htf_bars(cache, &bare_sym, &base_sym, "1Month");

        // Previous = second-to-last native bar (last *closed* HTF candle).
        let prev = |bars: &[Bar]| -> Option<(f64, f64)> {
            (bars.len() >= 2).then(|| {
                let p = &bars[bars.len() - 2];
                (p.high, p.low)
            })
        };
        // Current = last native bar (the forming HTF candle).
        let cur = |bars: &[Bar]| -> Option<(f64, f64)> { bars.last().map(|b| (b.high, b.low)) };

        if let Some(b) = h1.as_deref().filter(|b| !b.is_empty()) {
            if let Some((h, l)) = prev(b) {
                self.prev_h1_high = Some(h);
                self.prev_h1_low = Some(l);
            }
        }
        if let Some(b) = h4.as_deref().filter(|b| !b.is_empty()) {
            if let Some((h, l)) = prev(b) {
                self.prev_h4_high = Some(h);
                self.prev_h4_low = Some(l);
            }
        }
        if let Some(b) = d1.as_deref().filter(|b| !b.is_empty()) {
            if let Some((h, l)) = prev(b) {
                self.prev_daily_high = Some(h);
                self.prev_daily_low = Some(l);
            }
            if let Some((h, l)) = cur(b) {
                self.current_daily_high = Some(h);
                self.current_daily_low = Some(l);
            }
        }
        if let Some(b) = w1.as_deref().filter(|b| !b.is_empty()) {
            if let Some((h, l)) = prev(b) {
                self.prev_weekly_high = Some(h);
                self.prev_weekly_low = Some(l);
            }
            if let Some((h, l)) = cur(b) {
                self.current_weekly_high = Some(h);
                self.current_weekly_low = Some(l);
            }
        }
        if let Some(b) = mn1.as_deref().filter(|b| !b.is_empty()) {
            if let Some((h, l)) = prev(b) {
                self.prev_monthly_high = Some(h);
                self.prev_monthly_low = Some(l);
            }
            if let Some((h, l)) = cur(b) {
                self.current_monthly_high = Some(h);
                self.current_monthly_low = Some(l);
            }
        }
    }
}
