use super::*;

const ALPACA_BATCH_FETCH_MAX_SYMBOLS: usize = 50;
const ALPACA_BATCH_FETCH_INTRADAY_SYMBOLS: usize = 16;
const ALPACA_BATCH_FETCH_LOW_TF_SYMBOLS: usize = 8;
pub(super) const BACKGROUND_RETRY_PENDING_FETCH_CAP: usize = 256;

/// When process RSS exceeds this threshold we pause new background (non-focus)
/// market data fetches. Focus charts and explicit user actions are still allowed.
/// 18 GB on a 32 GB system leaves headroom for the rest of the desktop.
const HEAVY_SYNC_RSS_PAUSE_MB: u64 = 18_000;
pub(super) fn current_process_rss_mb() -> u64 {
    // Lightweight /proc/self/status read — no extra dependencies.
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                if let Some(kb_str) = line.split_whitespace().nth(1) {
                    if let Ok(kb) = kb_str.parse::<u64>() {
                        return kb / 1024;
                    }
                }
            }
        }
    }
    0
}

pub(super) fn background_retry_dispatch_allowed(pending_fetches: usize) -> bool {
    pending_fetches < BACKGROUND_RETRY_PENDING_FETCH_CAP
}

fn background_market_data_fetch_allowed(focus: bool, pending_fetches: usize) -> bool {
    if focus {
        return true;
    }
    if pending_fetches >= BACKGROUND_RETRY_PENDING_FETCH_CAP {
        return false;
    }
    // Memory backpressure: pause broad background work when RSS is already high.
    // Focus charts and explicit user actions bypass this check.
    let rss_mb = current_process_rss_mb();
    if rss_mb > 0 && rss_mb >= HEAVY_SYNC_RSS_PAUSE_MB {
        return false;
    }
    true
}

pub(super) fn normalize_kraken_equity_symbol_list<'a, I>(symbols: I) -> Vec<String>
where
    I: IntoIterator<Item = &'a String>,
{
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for source in symbols {
        let symbol = normalize_market_data_symbol(source)
            .replace('/', "")
            .trim_end_matches(".EQ")
            .to_ascii_uppercase();
        if !symbol.is_empty() && seen.insert(symbol.clone()) {
            out.push(symbol);
        }
    }
    out.sort();
    out
}

pub(super) fn kraken_equity_native_symbols_for_timeframe(
    catalog_symbols: &[String],
    demand_symbols: &[String],
    timeframe: &str,
) -> Vec<String> {
    // Native Kraken-Equities (iapi/WS) is the demand-scoped DEPTH lane, so the
    // Sync Status "Kraken Equities" row counts only held/charted/watchlisted
    // symbols and can reach ~100% as they fill. The ~12k catalog's breadth lives
    // in the Merged row (Alpaca/Yahoo history + merge), not the native provider
    // row — counting 12k here would peg it near 0% forever.
    let _ = catalog_symbols;
    if kraken_equity_full_universe_timeframe(timeframe) {
        normalize_kraken_equity_symbol_list(demand_symbols.iter())
    } else {
        Vec::new()
    }
}

pub(super) fn kraken_equity_native_history_symbols(
    catalog_symbols: &[String],
    demand_symbols: &[String],
) -> Vec<String> {
    // iapi history is the rate-limited DEPTH lane: ~6 req/s Cloudflare ceiling,
    // one (symbol, tf) per call, 1015-banned on overshoot. It owns ONLY the
    // demand set — symbols you hold, chart, or watchlist — which reaches full
    // depth across every timeframe in ~a minute. The ~12k reference catalog's
    // breadth is carried by the batched Alpaca/Yahoo history lanes plus the
    // merge, never by iapi: a catalog × timeframe sweep at 6 req/s is multi-hour
    // and trips escalating 1015 bans, which is exactly why overnight
    // Kraken-Equities coverage flatlined near zero.
    let _ = catalog_symbols;
    normalize_kraken_equity_symbol_list(demand_symbols.iter())
}

pub(super) fn kraken_equity_symbols_for_timeframe(
    catalog_symbols: &[String],
    demand_symbols: &[String],
    timeframe: &str,
) -> Vec<String> {
    if (kraken_equity_full_universe_timeframe(timeframe)
        || kraken_equity_broad_fallback_timeframe(timeframe))
        && !catalog_symbols.is_empty()
    {
        normalize_kraken_equity_symbol_list(catalog_symbols.iter())
    } else {
        normalize_kraken_equity_symbol_list(demand_symbols.iter())
    }
}

impl TyphooNApp {
    pub(super) fn full_tilt_sync_enabled(&self) -> bool {
        match std::env::var("TYPHOON_SYNC_FULL_TILT") {
            Ok(value) => {
                let value = value.trim().to_ascii_lowercase();
                if matches!(value.as_str(), "0" | "false" | "off" | "no" | "battery") {
                    return false;
                }
                if matches!(value.as_str(), "1" | "true" | "on" | "yes" | "ac" | "full") {
                    return true;
                }
            }
            Err(_) => {}
        }
        // Stay in full-tilt while we are behind on coverage regardless of
        // power source. A weekend that lands us at 55% would otherwise idle
        // overnight at the balanced cadence and never catch up before Monday
        // open. Threshold/hysteresis live in `auto_full_tilt_until_caught_up`.
        if self.auto_full_tilt_until_caught_up() {
            return true;
        }
        super::auto_compact::on_ac_power()
    }

    /// True while live-broker bar coverage sits below the "we're caught up"
    /// threshold. Reads the cached Sync Status snapshot so it is O(1) on the
    /// hot path; the snapshot itself is refreshed on a paced cadence by
    /// the main update loop. The Sync Status window gets 1 Hz updates when
    /// visible, but the hidden/background path is deliberately slower so a
    /// 12k-symbol universe does not rebuild coverage rows every frame/second.
    /// Hysteresis lives in `compute_bar_sync_rows` so the flip
    /// happens exactly when the snapshot is computed, not when the predicate
    /// is read.
    pub(super) fn auto_full_tilt_until_caught_up(&self) -> bool {
        self.auto_full_tilt_active
    }

    pub(super) fn market_data_sync_interval(&self) -> std::time::Duration {
        let secs = if self.full_tilt_sync_enabled() {
            FULL_TILT_SYNC_INTERVAL_SECS
        } else {
            BALANCED_SYNC_INTERVAL_SECS
        };
        std::time::Duration::from_secs(secs)
    }

    /// UX7: Lazily fetch 30 daily closes for a symbol from bar cache.
    /// Returns Arc for O(1) clones — called per-row per-frame in open scanners.
    /// MEM: Soft-capped at 2000 entries (≈2000 × 30 × 8 bytes = ~480KB).
    pub(super) fn get_sparkline(&mut self, symbol: &str) -> std::sync::Arc<Vec<f64>> {
        let key = symbol.to_uppercase();
        if let Some(closes) = self.sparkline_cache.get(&key) {
            return std::sync::Arc::clone(closes);
        }
        // Soft cap: drop random 25% if exceeded (no LRU bookkeeping cost)
        if self.sparkline_cache.len() > 2000 {
            let to_drop: Vec<String> = self.sparkline_cache.keys().take(500).cloned().collect();
            for k in to_drop {
                self.sparkline_cache.remove(&k);
            }
        }
        // Lazy load
        if let Some(ref cache) = self.cache {
            let candidates = [
                format!("kraken:{}:1Day", symbol),
                format!("alpaca:{}:1Day", symbol),
            ];
            for k in &candidates {
                if let Ok(Some(bars)) = cache.get_bars_raw(k) {
                    if bars.len() >= 5 {
                        let take = bars.len().min(30);
                        let closes: Vec<f64> = bars
                            .iter()
                            .rev()
                            .take(take)
                            .rev()
                            .map(|(_, _, _, _, c, _)| *c)
                            .collect();
                        let arc = std::sync::Arc::new(closes);
                        self.sparkline_cache
                            .insert(key.clone(), std::sync::Arc::clone(&arc));
                        return arc;
                    }
                }
            }
        }
        // Cache empty result to avoid retrying every frame
        let empty = std::sync::Arc::new(Vec::new());
        self.sparkline_cache
            .insert(key, std::sync::Arc::clone(&empty));
        empty
    }

    /// UX4: Built-in workspace presets — return JSON for known workspace names.
    /// User-saved workspaces in self.workspaces take precedence.
    pub(super) fn builtin_workspace(name: &str) -> Option<serde_json::Value> {
        let json = match name.to_uppercase().as_str() {
            "TRADING" => serde_json::json!({
                "sec": false, "insider": false, "fundamentals": false, "ev": false,
                "earnings": false, "dividends": false, "outliers": false,
                "stress_test": false, "volume_profile": true, "hv_cone": false,
                "sector_heatmap": false, "dividends_screen": false, "event_calendar": false,
            "alerts": true, "journal": false, "compact_mode": false,
            }),
            "RESEARCH" => serde_json::json!({
                "sec": true, "insider": true, "fundamentals": true, "ev": true,
                "earnings": true, "dividends": true, "outliers": true,
                "stress_test": false, "volume_profile": false, "hv_cone": false,
                "sector_heatmap": true, "dividends_screen": true, "event_calendar": true,
            "alerts": false, "journal": false, "compact_mode": false,
            }),
            "COMPACT" => serde_json::json!({
                "sec": false, "insider": false, "fundamentals": false, "ev": false,
                "earnings": false, "dividends": false, "outliers": false,
                "stress_test": false, "volume_profile": false, "hv_cone": false,
                "sector_heatmap": false, "dividends_screen": false, "event_calendar": false,
            "alerts": false, "journal": false, "compact_mode": true,
            }),
            _ => return None,
        };
        Some(json)
    }

    /// UX4: Capture current window layout (all show_* flags) as JSON for workspace presets.
    pub(super) fn capture_workspace_snapshot(&self) -> serde_json::Value {
        serde_json::json!({
            "sec": self.show_sec,
            "insider": self.show_insider,
            "fundamentals": self.show_fundamentals,
            "ev": self.show_ev_scanner,
            "earnings": self.show_earnings_calendar,
            "dividends": self.show_dividend_calendar,
            "outliers": self.show_outliers,
            "stress_test": self.show_stress_test,
            "volume_profile": self.show_volume_profile,
            "hv_cone": self.show_hv_cone,
            "sector_heatmap": self.show_sector_heatmap,
            "dividends_screen": self.show_dividends,
            "company_info": self.show_company_info_window,
            "event_calendar": self.show_event_calendar,
            "alerts": self.show_alert_builder,
            "journal": self.show_journal,
            "compact_mode": self.compact_mode,
            "broker_scope": self.broker_scope_label(),
        })
    }

    /// UX4: Apply a workspace snapshot — toggle window visibility from JSON.
    pub(super) fn apply_workspace_snapshot(&mut self, snap: &serde_json::Value) {
        macro_rules! set_bool {
            ($key:expr, $field:ident) => {
                if let Some(b) = snap.get($key).and_then(|v| v.as_bool()) {
                    self.$field = b;
                }
            };
        }
        set_bool!("sec", show_sec);
        set_bool!("insider", show_insider);
        set_bool!("fundamentals", show_fundamentals);
        set_bool!("ev", show_ev_scanner);
        set_bool!("earnings", show_earnings_calendar);
        set_bool!("dividends", show_dividend_calendar);
        set_bool!("outliers", show_outliers);
        set_bool!("stress_test", show_stress_test);
        set_bool!("volume_profile", show_volume_profile);
        set_bool!("hv_cone", show_hv_cone);
        set_bool!("sector_heatmap", show_sector_heatmap);
        set_bool!("dividends_screen", show_dividends);
        set_bool!("company_info", show_company_info_window);
        set_bool!("event_calendar", show_event_calendar);
        set_bool!("alerts", show_alert_builder);
        set_bool!("journal", show_journal);
        set_bool!("compact_mode", compact_mode);
    }

    /// Capture current indicator show_* flags as a JSON value (same schema as session "indicators").
    pub(super) fn capture_indicator_snapshot(&self) -> serde_json::Value {
        serde_json::json!({
            "sma200": self.show_sma200, "sma100": self.show_sma100,
            "kama": self.show_kama, "ema21": self.show_ema21,
            "bollinger": self.show_bollinger, "ichimoku": self.show_ichimoku,
            "wma": self.show_wma, "hma": self.show_hma,
            "psar": self.show_psar, "atr_proj": self.show_atr_proj,
            "prev_levels": self.show_prev_levels, "pivots": self.show_pivots,
            "fractals": self.show_fractals, "harmonics": self.show_harmonics,
            "auto_fib": self.show_auto_fib, "supply_demand": self.show_supply_demand,
            "ehlers_ss": self.show_ehlers_ss, "ehlers_decycler": self.show_ehlers_decycler,
            "ehlers_itl": self.show_ehlers_itl, "ehlers_mama": self.show_ehlers_mama,
            "ehlers_ebsw": self.show_ehlers_ebsw, "ehlers_cyber": self.show_ehlers_cyber,
            "ehlers_cg": self.show_ehlers_cg, "ehlers_roof": self.show_ehlers_roof,
            "rsi": self.show_rsi, "fisher": self.show_fisher,
            "macd": self.show_macd, "stochastic": self.show_stochastic,
            "adx": self.show_adx, "cci": self.show_cci,
            "williams_r": self.show_williams_r, "obv": self.show_obv,
            "momentum": self.show_momentum, "cmo": self.show_cmo,
            "qstick": self.show_qstick, "disparity": self.show_disparity,
            "bop": self.show_bop, "stddev": self.show_stddev,
            "mfi": self.show_mfi, "trix": self.show_trix,
            "ppo": self.show_ppo, "ultosc": self.show_ultosc,
            "stochrsi": self.show_stochrsi,
            "var_oscillator": self.show_var_oscillator,
            "better_volume": self.show_better_volume,
            "volume_pane": self.show_volume_pane, "sessions": self.show_sessions,
            "vol_heatmap": self.show_vol_heatmap, "vwap": self.show_vwap,
            "price_histogram": self.show_price_histogram,
            "supertrend": self.show_supertrend, "donchian": self.show_donchian,
            "keltner": self.show_keltner, "regression": self.show_regression,
            "squeeze": self.show_squeeze, "fvg": self.show_fvg,
        })
    }

    /// Apply a JSON indicator snapshot to all show_* flags.
    pub(super) fn apply_indicator_snapshot(&mut self, snap: &serde_json::Value) {
        for (key, field) in [
            ("sma200", &mut self.show_sma200),
            ("sma100", &mut self.show_sma100),
            ("kama", &mut self.show_kama),
            ("ema21", &mut self.show_ema21),
            ("bollinger", &mut self.show_bollinger),
            ("ichimoku", &mut self.show_ichimoku),
            ("wma", &mut self.show_wma),
            ("hma", &mut self.show_hma),
            ("psar", &mut self.show_psar),
            ("atr_proj", &mut self.show_atr_proj),
            ("prev_levels", &mut self.show_prev_levels),
            ("pivots", &mut self.show_pivots),
            ("fractals", &mut self.show_fractals),
            ("harmonics", &mut self.show_harmonics),
            ("auto_fib", &mut self.show_auto_fib),
            ("supply_demand", &mut self.show_supply_demand),
            ("ehlers_ss", &mut self.show_ehlers_ss),
            ("ehlers_decycler", &mut self.show_ehlers_decycler),
            ("ehlers_itl", &mut self.show_ehlers_itl),
            ("ehlers_mama", &mut self.show_ehlers_mama),
            ("ehlers_ebsw", &mut self.show_ehlers_ebsw),
            ("ehlers_cyber", &mut self.show_ehlers_cyber),
            ("ehlers_cg", &mut self.show_ehlers_cg),
            ("ehlers_roof", &mut self.show_ehlers_roof),
            ("rsi", &mut self.show_rsi),
            ("fisher", &mut self.show_fisher),
            ("macd", &mut self.show_macd),
            ("stochastic", &mut self.show_stochastic),
            ("adx", &mut self.show_adx),
            ("cci", &mut self.show_cci),
            ("williams_r", &mut self.show_williams_r),
            ("obv", &mut self.show_obv),
            ("momentum", &mut self.show_momentum),
            ("cmo", &mut self.show_cmo),
            ("qstick", &mut self.show_qstick),
            ("disparity", &mut self.show_disparity),
            ("bop", &mut self.show_bop),
            ("stddev", &mut self.show_stddev),
            ("mfi", &mut self.show_mfi),
            ("trix", &mut self.show_trix),
            ("ppo", &mut self.show_ppo),
            ("ultosc", &mut self.show_ultosc),
            ("stochrsi", &mut self.show_stochrsi),
            ("var_oscillator", &mut self.show_var_oscillator),
            ("better_volume", &mut self.show_better_volume),
            ("volume_pane", &mut self.show_volume_pane),
            ("sessions", &mut self.show_sessions),
            ("vol_heatmap", &mut self.show_vol_heatmap),
            ("vwap", &mut self.show_vwap),
            ("price_histogram", &mut self.show_price_histogram),
            ("supertrend", &mut self.show_supertrend),
            ("donchian", &mut self.show_donchian),
            ("keltner", &mut self.show_keltner),
            ("regression", &mut self.show_regression),
            ("squeeze", &mut self.show_squeeze),
            ("fvg", &mut self.show_fvg),
        ] {
            if let Some(b) = snap[key].as_bool() {
                *field = b;
            }
        }
    }

    /// Built-in template: NNFX (KAMA+Fisher+ATR+BVol+S/D+PrevLevels+SMA200).
    pub(super) fn builtin_template_nnfx() -> serde_json::Value {
        serde_json::json!({
            "sma200": true, "sma100": false, "kama": true, "ema21": false,
            "bollinger": false, "ichimoku": false, "wma": false, "hma": false,
            "psar": false, "atr_proj": true, "prev_levels": true, "pivots": false,
            "fractals": false, "harmonics": false, "auto_fib": false, "supply_demand": true,
            "ehlers_ss": false, "ehlers_decycler": false, "ehlers_itl": false, "ehlers_mama": false,
            "ehlers_ebsw": false, "ehlers_cyber": false, "ehlers_cg": false, "ehlers_roof": false,
            "rsi": false, "fisher": true, "macd": false, "stochastic": false,
            "adx": false, "cci": false, "williams_r": false, "obv": false,
            "momentum": false, "cmo": false, "qstick": false, "disparity": false,
            "bop": false, "stddev": false, "mfi": false, "trix": false,
            "ppo": false, "ultosc": false, "stochrsi": false,
            "var_oscillator": false, "better_volume": true,
            "volume_pane": false, "sessions": true,
            "vol_heatmap": false, "vwap": false, "price_histogram": false,
            "supertrend": false, "donchian": false, "keltner": false,
            "regression": false, "squeeze": false, "fvg": false,
        })
    }

    /// Built-in template: CLEAN (everything off except volume_pane).
    pub(super) fn builtin_template_clean() -> serde_json::Value {
        serde_json::json!({
            "sma200": false, "sma100": false, "kama": false, "ema21": false,
            "bollinger": false, "ichimoku": false, "wma": false, "hma": false,
            "psar": false, "atr_proj": false, "prev_levels": false, "pivots": false,
            "fractals": false, "harmonics": false, "auto_fib": false, "supply_demand": false,
            "ehlers_ss": false, "ehlers_decycler": false, "ehlers_itl": false, "ehlers_mama": false,
            "ehlers_ebsw": false, "ehlers_cyber": false, "ehlers_cg": false, "ehlers_roof": false,
            "rsi": false, "fisher": false, "macd": false, "stochastic": false,
            "adx": false, "cci": false, "williams_r": false, "obv": false,
            "momentum": false, "cmo": false, "qstick": false, "disparity": false,
            "bop": false, "stddev": false, "mfi": false, "trix": false,
            "ppo": false, "ultosc": false, "stochrsi": false,
            "var_oscillator": false, "better_volume": false,
            "volume_pane": true, "sessions": false,
            "vol_heatmap": false, "vwap": false, "price_histogram": false,
            "supertrend": false, "donchian": false, "keltner": false,
            "regression": false, "squeeze": false, "fvg": false,
        })
    }

    /// Built-in template: FULL (everything on).
    pub(super) fn builtin_template_full() -> serde_json::Value {
        serde_json::json!({
            "sma200": true, "sma100": true, "kama": true, "ema21": true,
            "bollinger": true, "ichimoku": true, "wma": true, "hma": true,
            "psar": true, "atr_proj": true, "prev_levels": true, "pivots": true,
            "fractals": true, "harmonics": true, "auto_fib": true, "supply_demand": true,
            "ehlers_ss": true, "ehlers_decycler": true, "ehlers_itl": true, "ehlers_mama": true,
            "ehlers_ebsw": true, "ehlers_cyber": true, "ehlers_cg": true, "ehlers_roof": true,
            "rsi": true, "fisher": true, "macd": true, "stochastic": true,
            "adx": true, "cci": true, "williams_r": true, "obv": true,
            "momentum": true, "cmo": true, "qstick": true, "disparity": true,
            "bop": true, "stddev": true, "mfi": true, "trix": true,
            "ppo": true, "ultosc": true, "stochrsi": true,
            "var_oscillator": true, "better_volume": true,
            "volume_pane": true, "sessions": true,
            "vol_heatmap": true, "vwap": true, "price_histogram": true,
            "supertrend": true, "donchian": true, "keltner": true,
            "regression": true, "squeeze": true, "fvg": true,
        })
    }

    pub(super) fn sync_timeframe_enabled(&self, tf: &str) -> bool {
        normalize_sync_timeframe_key(tf)
            .map(|tf| self.enabled_sync_timeframes.contains(tf))
            .unwrap_or(false)
    }

    pub(super) fn enabled_standard_sync_timeframes(&self) -> Vec<String> {
        STANDARD_SYNC_TIMEFRAMES
            .iter()
            .filter_map(|(_, tf)| self.sync_timeframe_enabled(tf).then(|| (*tf).to_string()))
            .collect()
    }

    pub(super) fn filtered_sync_timeframes<'a, I>(&self, tfs: I) -> Vec<String>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut seen = std::collections::HashSet::with_capacity(STANDARD_SYNC_TIMEFRAMES.len());
        let mut out: Vec<String> = Vec::with_capacity(STANDARD_SYNC_TIMEFRAMES.len());
        for tf in tfs {
            let Some(norm) = normalize_sync_timeframe_key(tf) else {
                continue;
            };
            if !self.sync_timeframe_enabled(norm) || !seen.insert(norm) {
                continue;
            }
            out.push(norm.to_string());
        }
        out
    }

    pub(super) fn build_source_cache_state_map(
        &self,
        prefix: &str,
    ) -> std::collections::HashMap<(String, String), SyncCacheState> {
        let mut map: std::collections::HashMap<(String, String), SyncCacheState> =
            std::collections::HashMap::with_capacity(self.bg.detailed_stats.len());
        for (key, bars, ts) in &self.bg.detailed_stats {
            let rest = match key.strip_prefix(prefix) {
                Some(r) => r,
                None => continue,
            };
            if rest.starts_with("__") {
                continue;
            }
            let mut it = rest.split(':');
            let sym = match it.next() {
                Some(s) if !s.is_empty() => normalize_market_data_symbol(s).replace('/', ""),
                _ => continue,
            };
            let tf = match it.next() {
                Some(s) if !s.is_empty() => match normalize_sync_timeframe_key(s) {
                    Some(tf) => tf.to_string(),
                    None => continue,
                },
                _ => continue,
            };
            if it.next().is_some() {
                continue;
            }
            let last_bar_ts_s = self
                .bg
                .bar_ts_cache
                .get(key)
                .map(|(_, last_ms, _)| last_ms.div_euclid(1000))
                .unwrap_or(0);
            let entry = map.entry((sym, tf)).or_default();
            if *ts > entry.write_ts_s {
                *entry = SyncCacheState {
                    last_bar_ts_s,
                    write_ts_s: *ts,
                    bar_count: *bars,
                };
            }
        }
        map
    }

    pub(super) fn build_alpaca_cache_state_map(
        &self,
    ) -> std::collections::HashMap<(String, String), SyncCacheState> {
        self.build_source_cache_state_map("alpaca:")
    }

    pub(super) fn pending_fetches_for_source_mut(
        &mut self,
        source: &str,
    ) -> &mut std::collections::HashSet<String> {
        match source {
            "alpaca" => &mut self.pending_alpaca_fetches,
            "kraken" => &mut self.pending_kraken_fetches,
            "kraken-futures" => &mut self.pending_kraken_futures_fetches,
            "yahoo-chart" => &mut self.pending_yahoo_chart_fetches,
            _ => &mut self.pending_alpaca_fetches,
        }
    }

    pub(super) fn total_pending_market_data_fetches(&self) -> usize {
        self.pending_alpaca_fetches.len()
            + self.pending_kraken_fetches.len()
            + self.pending_kraken_futures_fetches.len()
            + self.pending_yahoo_chart_fetches.len()
    }

    pub(super) fn note_cached_sync_success(
        &mut self,
        source: &str,
        symbol: &str,
        timeframe: &str,
        bar_count: usize,
    ) {
        let Some(tf) = normalize_sync_timeframe_key(timeframe) else {
            return;
        };
        let symbol = normalize_market_data_symbol(symbol).replace('/', "");
        if symbol.is_empty() {
            return;
        }
        let now_s = chrono::Utc::now().timestamp();
        let state = SyncCacheState {
            last_bar_ts_s: now_s,
            write_ts_s: now_s,
            bar_count: bar_count as i64,
        };
        match source {
            "alpaca" => {
                self.cached_alpaca_sync_state
                    .insert((symbol.clone(), tf.to_string()), state);
                self.cached_alpaca_sync_state_rev = Some(self.bg_rev);
            }
            "kraken" => {
                self.cached_kraken_sync_state
                    .insert((symbol.clone(), tf.to_string()), state);
                self.cached_kraken_sync_state_rev = Some(self.bg_rev);
            }
            "kraken-futures" => {
                self.cached_kraken_futures_sync_state
                    .insert((symbol.clone(), tf.to_string()), state);
                self.cached_kraken_futures_sync_state_rev = Some(self.bg_rev);
            }
            "kraken-equities" => {
                self.cached_kraken_equities_sync_state
                    .insert((symbol.clone(), tf.to_string()), state);
                self.cached_kraken_equities_sync_state_rev = Some(self.bg_rev);
            }
            "yahoo-chart" => {
                self.cached_yahoo_chart_sync_state
                    .insert((symbol.clone(), tf.to_string()), state);
                self.cached_yahoo_chart_sync_state_rev = Some(self.bg_rev);
            }
            _ => {}
        }
    }

    pub(super) fn market_data_focus_symbols(&self) -> std::collections::HashSet<String> {
        self.cached_active_symbols
            .iter()
            .map(|symbol| normalize_market_data_symbol(symbol).replace('/', ""))
            .filter(|symbol| !symbol.is_empty())
            .collect()
    }

    pub(super) fn schedule_light_market_data_targets(&mut self) -> usize {
        let symbols: Vec<String> = self.market_data_focus_symbols().into_iter().collect();
        if symbols.is_empty() {
            return 0;
        }
        let timeframes = self.enabled_standard_sync_timeframes();
        if timeframes.is_empty() {
            return 0;
        }
        let max_dispatch = if self.full_tilt_sync_enabled() {
            96
        } else {
            32
        };
        let mut dispatched = 0usize;
        for symbol in symbols {
            for tf in &timeframes {
                let mut queued = false;
                if self.kraken_enabled {
                    queued |= self.queue_kraken_fetch(&symbol, tf);
                    queued |= self.queue_kraken_equity_fetch(&symbol, tf);
                }
                if self.broker_connected {
                    queued |= self.queue_alpaca_fetch(&symbol, tf);
                }
                if queued {
                    dispatched += 1;
                    if dispatched >= max_dispatch {
                        return dispatched;
                    }
                }
            }
        }
        dispatched
    }

    pub(super) fn kraken_symbol_quote(symbol: &str) -> Option<&'static str> {
        let symbol = typhoon_engine::core::kraken::normalize_pair_symbol(symbol);
        const QUOTES: [&str; 12] = [
            "USDG", "USDT", "USDC", "USD", "EUR", "GBP", "CAD", "AUD", "JPY", "CHF", "XBT", "BTC",
        ];
        QUOTES
            .iter()
            .find(|quote| symbol.ends_with(**quote))
            .copied()
    }

    pub(super) fn kraken_symbol_sector(symbol: &str) -> usize {
        let symbol = typhoon_engine::core::kraken::normalize_pair_symbol(symbol);
        let quote = Self::kraken_symbol_quote(&symbol).unwrap_or("");
        let base = if quote.is_empty() || quote.len() >= symbol.len() {
            symbol.as_str()
        } else {
            &symbol[..symbol.len() - quote.len()]
        };
        let is_spot_fx = matches!(base, "USD" | "EUR" | "GBP" | "CAD" | "AUD" | "JPY" | "CHF")
            && matches!(quote, "USD" | "EUR" | "GBP" | "CAD" | "AUD" | "JPY" | "CHF");
        let is_xstock = base.ends_with(".EQ");
        if is_xstock {
            0 // xStocks / tokenized ETFs
        } else if is_spot_fx {
            2 // fiat FX pairs
        } else if matches!(quote, "USD" | "USDG" | "USDC" | "USDT") {
            1 // USD + stablecoin quoted spot
        } else if matches!(quote, "EUR" | "GBP" | "CAD" | "AUD" | "JPY" | "CHF") {
            2 // fiat-quoted spot
        } else {
            3 // crypto crosses and everything else
        }
    }

    pub(super) fn kraken_any_spot_scrape_enabled(&self) -> bool {
        self.kraken_scrape_xstocks
            || self.crypto_fiat_quote_usd
            || self.crypto_fiat_quote_usdt
            || self.crypto_fiat_quote_usdc
            || self.crypto_fiat_quote_usdg
            || self.crypto_fiat_quote_eur
            || self.crypto_fiat_quote_gbp
            || self.crypto_fiat_quote_cad
            || self.crypto_fiat_quote_aud
            || self.crypto_fiat_quote_jpy
            || self.crypto_fiat_quote_chf
            || self.kraken_scrape_crypto_crosses
    }

    pub(super) fn crypto_fiat_quote_scrape_enabled(&self, quote: &str) -> bool {
        match quote {
            "USD" => self.crypto_fiat_quote_usd,
            "USDT" => self.crypto_fiat_quote_usdt,
            "USDC" => self.crypto_fiat_quote_usdc,
            "USDG" => self.crypto_fiat_quote_usdg,
            "EUR" => self.crypto_fiat_quote_eur,
            "GBP" => self.crypto_fiat_quote_gbp,
            "CAD" => self.crypto_fiat_quote_cad,
            "AUD" => self.crypto_fiat_quote_aud,
            "JPY" => self.crypto_fiat_quote_jpy,
            "CHF" => self.crypto_fiat_quote_chf,
            _ => false,
        }
    }

    pub(super) fn kraken_spot_sector_scrape_enabled(&self, sector: usize) -> bool {
        Self::kraken_spot_sector_scrape_enabled_from_flags(
            sector,
            self.kraken_scrape_xstocks,
            self.crypto_fiat_quote_usd,
            self.crypto_fiat_quote_usdt,
            self.crypto_fiat_quote_usdc,
            self.crypto_fiat_quote_usdg,
            self.crypto_fiat_quote_eur,
            self.crypto_fiat_quote_gbp,
            self.crypto_fiat_quote_cad,
            self.crypto_fiat_quote_aud,
            self.crypto_fiat_quote_jpy,
            self.crypto_fiat_quote_chf,
            self.kraken_scrape_crypto_crosses,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn kraken_spot_sector_scrape_enabled_from_flags(
        sector: usize,
        _xstocks: bool,
        quote_usd: bool,
        quote_usdt: bool,
        quote_usdc: bool,
        quote_usdg: bool,
        quote_eur: bool,
        quote_gbp: bool,
        quote_cad: bool,
        quote_aud: bool,
        quote_jpy: bool,
        quote_chf: bool,
        crypto_crosses: bool,
    ) -> bool {
        match sector {
            0 => false, // xStocks use Kraken's internal equities API, not public Spot OHLC.
            1 => quote_usd || quote_usdt || quote_usdc || quote_usdg,
            2 => {
                quote_usd
                    || quote_eur
                    || quote_gbp
                    || quote_cad
                    || quote_aud
                    || quote_jpy
                    || quote_chf
            }
            3 => crypto_crosses,
            _ => false,
        }
    }

    pub(super) fn kraken_spot_symbol_scrape_enabled(&self, symbol: &str) -> bool {
        let symbol = typhoon_engine::core::kraken::normalize_pair_symbol(symbol);
        if symbol.is_empty()
            || typhoon_engine::core::kraken::to_kraken_pair_lossy(&symbol).is_none()
            || !self.kraken_spot_symbol_in_loaded_pairs(&symbol)
        {
            return false;
        }
        let sector = Self::kraken_symbol_sector(&symbol);
        if !self.kraken_spot_sector_scrape_enabled(sector) {
            return false;
        }
        match sector {
            1 | 2 => Self::kraken_symbol_quote(&symbol)
                .is_some_and(|quote| self.crypto_fiat_quote_scrape_enabled(quote)),
            _ => true,
        }
    }

    fn kraken_spot_symbol_in_loaded_pairs(&self, symbol: &str) -> bool {
        if self.kraken_pairs.is_empty() {
            return true;
        }
        // O(1) lookup against the pre-normalized set built when KrakenPairs arrives.
        // The previous implementation called `normalize_pair_symbol` twice per element
        // for every check, blowing up the sync-symbol audit on each tick.
        let symbol = typhoon_engine::core::kraken::normalize_pair_symbol(symbol);
        let key = symbol.to_ascii_uppercase();
        self.kraken_pairs_normalized.contains(&key)
    }

    pub(super) fn kraken_sync_symbol_sectors(&self) -> Vec<Vec<String>> {
        let mut sectors = vec![Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        let mut seen = std::collections::HashSet::new();
        let mut push_symbol = |source: &str| {
            let symbol = typhoon_engine::core::kraken::normalize_pair_symbol(source);
            if symbol.is_empty()
                || !self.kraken_spot_symbol_scrape_enabled(&symbol)
                || !seen.insert(symbol.clone())
            {
                return;
            }
            let sector = Self::kraken_symbol_sector(&symbol);
            sectors[sector].push(symbol);
        };

        for (pair_name, display_name) in &self.kraken_pairs {
            let source = if display_name.trim().is_empty() {
                pair_name.as_str()
            } else {
                display_name.as_str()
            };
            push_symbol(source);
        }
        for chart in &self.charts {
            push_symbol(&chart.symbol);
        }
        for symbol in &self.user_watchlist {
            push_symbol(symbol);
        }
        for sector in &mut sectors {
            sector.sort();
        }
        sectors
    }

    pub(super) fn kraken_futures_sync_symbols(&self) -> Vec<String> {
        if !self.kraken_scrape_futures {
            return Vec::new();
        }
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        let mut push_symbol = |source: &str| {
            let symbol = typhoon_engine::core::kraken_futures::normalize_futures_symbol(source);
            if !symbol.is_empty()
                && typhoon_engine::core::kraken_futures::is_futures_symbol(&symbol)
                && seen.insert(symbol.clone())
            {
                out.push(symbol);
            }
        };

        for symbol in &self.kraken_futures_symbols {
            push_symbol(symbol);
        }
        for chart in &self.charts {
            push_symbol(&chart.symbol);
        }
        for symbol in &self.user_watchlist {
            push_symbol(symbol);
        }
        out.sort();
        out
    }

    pub(super) fn kraken_equity_catalog_symbol_count(&self) -> usize {
        if !self.kraken_enabled || !self.kraken_scrape_xstocks {
            0
        } else {
            self.kraken_equity_universe_symbols.len()
        }
    }

    pub(super) fn kraken_equity_catalog_symbols(&self) -> Vec<String> {
        if !self.kraken_enabled || !self.kraken_scrape_xstocks {
            return Vec::new();
        }
        normalize_kraken_equity_symbol_list(self.kraken_equity_universe_symbols.iter())
    }

    /// The WS-subscribable subset (tokenized `{SYM}x/USD` xStocks). Used to scope
    /// the WS OHLC snapshot sweep to pairs Kraken actually serves on WS v2, rather
    /// than the full ~12k iapi catalog (which is ~99% non-WS Securities). Returns
    /// empty if the WS tokenized snapshot was unavailable at universe load — in
    /// which case there is nothing to sweep on WS and breadth rides Alpaca/Yahoo.
    pub(super) fn kraken_equity_ws_sweep_symbols(&self) -> Vec<String> {
        if !self.kraken_enabled || !self.kraken_scrape_xstocks {
            return Vec::new();
        }
        normalize_kraken_equity_symbol_list(self.kraken_equity_tokenized_symbols.iter())
    }

    pub(super) fn kraken_equity_demand_symbols(&self) -> Vec<String> {
        if !self.kraken_enabled || !self.kraken_scrape_xstocks {
            return Vec::new();
        }
        let mut raw = Vec::new();
        for (pair_name, display_name) in &self.kraken_pairs {
            if let Some(symbol) = kraken_xstock_fundamental_symbol(pair_name, display_name) {
                raw.push(symbol);
            }
        }
        for (asset, qty) in &self.kraken_balances {
            if qty.is_finite() && *qty > 0.0 && Self::kraken_display_asset(asset).ends_with(".EQ") {
                raw.push(Self::kraken_display_asset(asset));
            }
        }
        for chart in &self.charts {
            let source = cache_source_from_key(&chart.symbol);
            let bare = bare_symbol_from_key(&chart.symbol);
            if source == "kraken-equities" || bare.to_ascii_uppercase().ends_with(".EQ") {
                raw.push(bare);
            }
        }
        for symbol in &self.user_watchlist {
            if symbol.to_ascii_uppercase().ends_with(".EQ") {
                raw.push(symbol.clone());
            }
        }
        normalize_kraken_equity_symbol_list(raw.iter())
    }

    pub(super) fn kraken_futures_symbol_sector(symbol: &str) -> usize {
        let symbol = typhoon_engine::core::kraken_futures::normalize_futures_symbol(symbol);
        if symbol.starts_with("PF_") {
            0 // flexible/perpetual futures
        } else if symbol.starts_with("PI_") {
            1 // inverse perpetual futures
        } else if symbol.starts_with("FF_") || symbol.starts_with("FI_") {
            2 // dated futures
        } else {
            3
        }
    }

    pub(super) fn kraken_futures_sync_symbol_sectors(&self) -> Vec<Vec<String>> {
        let mut sectors = vec![Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        for symbol in self.kraken_futures_sync_symbols() {
            let sector = Self::kraken_futures_symbol_sector(&symbol);
            sectors[sector].push(symbol);
        }
        for sector in &mut sectors {
            sector.sort();
        }
        sectors
    }

    pub(super) fn schedule_kraken_universe_sectors(&mut self) -> usize {
        if !self.kraken_enabled
            || !self.kraken_full_bar_sync_enabled
            || !self.kraken_any_spot_scrape_enabled()
        {
            return 0;
        }
        let sectors = self.kraken_sync_symbol_sectors();
        let full_tilt = self.full_tilt_sync_enabled();
        let budgets = if full_tilt {
            [96usize, 128, 96, 96]
        } else {
            [12usize, 16, 12, 12]
        };
        let foreground_slots = if full_tilt { 16 } else { 4 };
        let mut dispatched = 0usize;
        for (idx, (sector, budget)) in sectors.iter().zip(budgets).enumerate() {
            if !self.kraken_spot_sector_scrape_enabled(idx) {
                continue;
            }
            dispatched +=
                self.schedule_kraken_pairs_with_budget(idx, sector, budget, foreground_slots);
        }
        dispatched
    }

    pub(super) fn schedule_kraken_equities_universe(&mut self) -> usize {
        let catalog_symbols = self.kraken_equity_catalog_symbols();
        let demand_symbols = self.kraken_equity_demand_symbols();
        let native_symbols =
            kraken_equity_native_history_symbols(&catalog_symbols, &demand_symbols);
        let fallback_symbols = if catalog_symbols.is_empty() {
            demand_symbols.clone()
        } else {
            catalog_symbols.clone()
        };
        if !self.kraken_enabled
            || !self.kraken_full_bar_sync_enabled
            || (native_symbols.is_empty() && fallback_symbols.is_empty())
        {
            return 0;
        }
        if self.kraken_equities_sync_pause_until_ts > chrono::Utc::now().timestamp() {
            return 0;
        }
        let enabled_timeframes = self.enabled_standard_sync_timeframes();
        // Kraken Equities is WS-first for live/current OHLC and REST/iapi remains
        // the repair/backfill lane. Treat all enabled standard timeframes as native
        // Kraken Equities coverage; assist providers keep their own narrower gates.
        let native_timeframes: Vec<String> = enabled_timeframes
            .iter()
            .filter(|tf| kraken_equity_full_universe_timeframe(tf))
            .cloned()
            .collect();
        let fallback_timeframes: Vec<String> = enabled_timeframes
            .iter()
            .filter(|tf| kraken_equity_broad_fallback_timeframe(tf))
            .cloned()
            .collect();
        if native_timeframes.is_empty() && fallback_timeframes.is_empty() {
            return 0;
        }
        let full_tilt = self.full_tilt_sync_enabled();
        let queue_window: usize = if full_tilt {
            KRAKEN_EQUITIES_FULL_TILT_QUEUE_WINDOW
        } else {
            8
        };
        let batch_limit: usize = if full_tilt {
            KRAKEN_EQUITIES_FULL_TILT_BATCH_SIZE
        } else {
            4
        };
        let foreground_slots = if full_tilt {
            KRAKEN_EQUITIES_FULL_TILT_BATCH_SIZE
        } else {
            4
        };
        let scan_limit = if full_tilt {
            KRAKEN_EQUITIES_FULL_TILT_BACKGROUND_SCAN_LIMIT
        } else {
            96
        };
        if self.cached_kraken_equities_sync_state_rev != Some(self.bg_rev)
            && (!self.heavy_sync_in_progress || self.cached_kraken_equities_sync_state.is_empty())
        {
            let previous = self.cached_kraken_equities_sync_state.clone();
            let mut rebuilt = self.build_source_cache_state_map("kraken-equities:");
            merge_recent_sync_overrides(&mut rebuilt, &previous, chrono::Utc::now().timestamp());
            self.cached_kraken_equities_sync_state = rebuilt;
            self.cached_kraken_equities_sync_state_rev = Some(self.bg_rev);
        }
        self.ensure_unresolvable_fetch_key_index();
        let _focus_symbols = self.market_data_focus_symbols();
        let focus_symbols = self.market_data_focus_symbols();

        // Kraken high-TF (1Day/1Week/1Month) backfill aggressiveness fix:
        // Always treat these rows as high-priority when the symbol is focused
        // (open chart or MTF grid). This prevents the "stale forever" state
        // the user reported even for actively used Kraken symbols.

        let empty_no_data_keys = std::collections::HashSet::new();
        let empty_backfill = std::collections::HashMap::new();
        let mut dispatched = 0usize;
        let now_s = chrono::Utc::now().timestamp();

        let native_available_slots = queue_window
            .saturating_sub(
                self.pending_kraken_fetches
                    .iter()
                    .filter(|key| key.starts_with("equity:"))
                    .count(),
            )
            .min(batch_limit);
        if native_available_slots > 0 && !native_timeframes.is_empty() {
            let no_data_keys = self
                .unresolvable_fetch_keys_by_broker
                .get("kraken-equities")
                .unwrap_or(&empty_no_data_keys);
            let pending_equities: std::collections::HashSet<String> = self
                .pending_kraken_fetches
                .iter()
                .filter_map(|key| key.strip_prefix("equity:").map(str::to_string))
                .collect();
            let mut cursor = self.kraken_equities_sync_cursor;
            // Tier priority (MTF Grid > Active > Background) + high-TF-first is applied inside the workset selector
            let candidates = select_alpaca_sync_workset_rotating_with_stale_multiplier(
                &native_symbols,
                &native_timeframes,
                &self.cached_kraken_equities_sync_state,
                &focus_symbols,
                no_data_keys,
                &empty_backfill,
                &pending_equities,
                native_available_slots,
                foreground_slots,
                scan_limit,
                &mut cursor,
                now_s,
                1,
                kraken_equities_sync_target_bars,
            );
            self.kraken_equities_sync_cursor = cursor;
            for candidate in candidates {
                if self.queue_kraken_equity_fetch(&candidate.symbol, &candidate.timeframe) {
                    dispatched += 1;
                }
            }
        }

        if self.backfill_alpaca_kraken_equities_enabled && !fallback_timeframes.is_empty() {
            let alpaca_timeframes: Vec<String> = fallback_timeframes
                .iter()
                .filter(|tf| alpaca_sync_target_bars(tf).is_some())
                .cloned()
                .collect();
            // M1/M5 broad equity sync is Kraken-native only. Alpaca assist
            // remains disabled there so it does not burn historical RPM on
            // rows the merged/chart path deliberately ignores.
            if !alpaca_timeframes.is_empty() {
                if self.cached_alpaca_sync_state_rev != Some(self.bg_rev)
                    && (!self.heavy_sync_in_progress || self.cached_alpaca_sync_state.is_empty())
                {
                    let previous = self.cached_alpaca_sync_state.clone();
                    let mut rebuilt = self.build_alpaca_cache_state_map();
                    merge_recent_sync_overrides(&mut rebuilt, &previous, now_s);
                    self.cached_alpaca_sync_state = rebuilt;
                    self.cached_alpaca_sync_state_rev = Some(self.bg_rev);
                }
                if !self.alpaca_no_data_loaded {
                    self.alpaca_no_data_load();
                }
                if !self.alpaca_backfill_complete_loaded {
                    self.alpaca_backfill_complete_load();
                }
                let capacity = self.alpaca_sync_capacity();
                let available_slots = capacity
                    .queue_window
                    .saturating_sub(self.pending_alpaca_fetches.len())
                    .min(capacity.batch_size);
                if available_slots > 0 {
                    let mut no_data: std::collections::HashSet<String> =
                        self.alpaca_no_data_pairs.keys().cloned().collect();
                    if let Some(unresolvable) = self.unresolvable_fetch_keys_by_broker.get("alpaca")
                    {
                        no_data.extend(unresolvable.iter().cloned());
                    }
                    no_data.extend(
                        self.alpaca_retry_queue
                            .iter()
                            .map(|retry| alpaca_fetch_key(&retry.symbol, &retry.timeframe)),
                    );
                    let mut cursor = self.kraken_equities_alpaca_sync_cursor;
                    let candidates = select_alpaca_sync_workset_rotating(
                        &fallback_symbols,
                        &alpaca_timeframes,
                        &self.cached_alpaca_sync_state,
                        &focus_symbols,
                        &no_data,
                        &self.alpaca_backfill_complete_pairs,
                        &self.pending_alpaca_fetches,
                        available_slots,
                        capacity.foreground_reserve,
                        if full_tilt {
                            ALPACA_FULL_TILT_BACKGROUND_SCAN_LIMIT
                        } else {
                            ALPACA_BACKGROUND_SCAN_LIMIT
                        },
                        &mut cursor,
                        now_s,
                        alpaca_sync_target_bars,
                    );
                    self.kraken_equities_alpaca_sync_cursor = cursor;
                    dispatched += self.queue_alpaca_batch_fetches_from_candidates(candidates);
                }
            }
        }

        if self.backfill_yahoo_chart_enabled && !fallback_timeframes.is_empty() {
            if self.yahoo_chart_sync_pause_until_ts > chrono::Utc::now().timestamp() {
                // Yahoo sync lane is on cooldown
            } else {
                let yahoo_timeframes: Vec<String> = fallback_timeframes
                    .iter()
                    .filter(|tf| yahoo_chart_supports_timeframe(tf))
                    .cloned()
                    .collect();
                if yahoo_timeframes.is_empty() {
                    return dispatched;
                }
                let yahoo_queue_window = if full_tilt {
                    YAHOO_CHART_FULL_TILT_QUEUE_WINDOW
                } else {
                    YAHOO_CHART_QUEUE_WINDOW
                };
                let yahoo_batch_limit = if full_tilt {
                    YAHOO_CHART_FULL_TILT_BATCH_SIZE
                } else {
                    YAHOO_CHART_BATCH_SIZE
                };
                let yahoo_foreground_slots = if full_tilt { 2 } else { 1 };
                let yahoo_scan_limit = if full_tilt {
                    YAHOO_CHART_FULL_TILT_BACKGROUND_SCAN_LIMIT
                } else {
                    128
                };
                let available_slots = yahoo_queue_window
                    .saturating_sub(self.pending_yahoo_chart_fetches.len())
                    .min(yahoo_batch_limit);
                if available_slots > 0 {
                    if self.cached_yahoo_chart_sync_state_rev != Some(self.bg_rev)
                        && (!self.heavy_sync_in_progress
                            || self.cached_yahoo_chart_sync_state.is_empty())
                    {
                        let previous = self.cached_yahoo_chart_sync_state.clone();
                        let mut rebuilt = self.build_source_cache_state_map("yahoo-chart:");
                        merge_recent_sync_overrides(&mut rebuilt, &previous, now_s);
                        self.cached_yahoo_chart_sync_state = rebuilt;
                        self.cached_yahoo_chart_sync_state_rev = Some(self.bg_rev);
                    }
                    let no_data = self
                        .unresolvable_fetch_keys_by_broker
                        .get("yahoo-chart")
                        .cloned()
                        .unwrap_or_default();
                    let mut cursor = self.yahoo_chart_sync_cursor;
                    let candidates = select_alpaca_sync_workset_rotating(
                        &fallback_symbols,
                        &yahoo_timeframes,
                        &self.cached_yahoo_chart_sync_state,
                        &focus_symbols,
                        &no_data,
                        &empty_backfill,
                        &self.pending_yahoo_chart_fetches,
                        available_slots,
                        yahoo_foreground_slots,
                        yahoo_scan_limit,
                        &mut cursor,
                        now_s,
                        alpaca_sync_target_bars,
                    );
                    self.yahoo_chart_sync_cursor = cursor;
                    for candidate in candidates {
                        if self.queue_yahoo_chart_fetch(&candidate.symbol, &candidate.timeframe) {
                            dispatched += 1;
                        }
                    }
                }
            }
        }

        dispatched
    }

    pub(super) fn schedule_kraken_futures_universe_sectors(&mut self) -> usize {
        if !self.kraken_enabled || !self.kraken_full_bar_sync_enabled || !self.kraken_scrape_futures
        {
            return 0;
        }
        let sectors = self.kraken_futures_sync_symbol_sectors();
        let full_tilt = self.full_tilt_sync_enabled();
        let budgets = if full_tilt {
            [96usize, 64, 64, 32]
        } else {
            [10usize, 8, 8, 4]
        };
        let foreground_slots = if full_tilt { 12 } else { 3 };
        let mut dispatched = 0usize;
        for (idx, (sector, budget)) in sectors.iter().zip(budgets).enumerate() {
            dispatched += self.schedule_kraken_futures_pairs_with_budget(
                idx,
                sector,
                budget,
                foreground_slots,
            );
        }
        dispatched
    }

    pub(super) fn alpaca_focus_symbols(&self) -> std::collections::HashSet<String> {
        self.cached_active_symbols
            .iter()
            .map(|sym| normalize_market_data_symbol(sym).replace('/', ""))
            .filter(|sym| !sym.is_empty())
            .collect()
    }

    pub(super) fn alpaca_effective_historical_rpm(&self) -> u32 {
        alpaca_effective_historical_rpm(
            self.alpaca_historical_rpm_hint,
            self.alpaca_historical_rpm_observed,
        )
    }

    pub(super) fn alpaca_sync_capacity(&self) -> AlpacaSyncCapacity {
        let mut capacity = alpaca_sync_capacity_for_rpm(self.alpaca_effective_historical_rpm());
        if self.full_tilt_sync_enabled() {
            capacity.fetch_permits = capacity.fetch_permits.max(ALPACA_FULL_TILT_FETCH_PERMITS);
            capacity.queue_window = capacity.queue_window.max(ALPACA_FULL_TILT_QUEUE_WINDOW);
            capacity.batch_size = capacity.batch_size.max(ALPACA_FULL_TILT_BATCH_SIZE);
            capacity.foreground_reserve = capacity.foreground_reserve.max(8);
        }
        capacity
    }

    pub(super) fn push_alpaca_sync_runtime_config(&self) {
        if !self.alpaca_enabled || !self.broker_connected {
            return;
        }
        let capacity = self.alpaca_sync_capacity();
        let _ = self.broker_tx.send(BrokerCmd::ConfigureAlpacaSync {
            bar_requests_per_minute: self.alpaca_effective_historical_rpm(),
            fetch_permits: capacity.fetch_permits,
        });
    }

    pub(super) fn alpaca_batch_fetch_supported(timeframe: &str) -> bool {
        matches!(
            normalize_sync_timeframe_key(timeframe),
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
        )
    }

    pub(super) fn alpaca_batch_fetch_chunk_symbols(timeframe: &str) -> usize {
        match normalize_sync_timeframe_key(timeframe) {
            // Dense payloads can otherwise put several workers into 100k+ bar
            // JSON/decompression/merge paths at once. Keep the multi-symbol
            // endpoint, but bound per-request RSS by timeframe.
            Some("1Min" | "5Min") => ALPACA_BATCH_FETCH_LOW_TF_SYMBOLS,
            Some("15Min" | "30Min" | "1Hour") => ALPACA_BATCH_FETCH_INTRADAY_SYMBOLS,
            _ => ALPACA_BATCH_FETCH_MAX_SYMBOLS,
        }
    }

    fn queue_alpaca_batch_fetches_from_candidates(
        &mut self,
        candidates: Vec<AlpacaSyncCandidate>,
    ) -> usize {
        let mut by_tf: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();
        let mut dispatched = 0usize;
        for candidate in candidates {
            if candidate.focus || !Self::alpaca_batch_fetch_supported(&candidate.timeframe) {
                if self.queue_alpaca_fetch(&candidate.symbol, &candidate.timeframe) {
                    dispatched += 1;
                }
                continue;
            }
            let Some(tf) = normalize_sync_timeframe_key(&candidate.timeframe) else {
                continue;
            };
            let symbol = normalize_market_data_symbol(&candidate.symbol).replace('/', "");
            let fetch_key = alpaca_fetch_key(&symbol, tf);
            if self.is_fetch_on_cooldown("alpaca", &symbol, tf) {
                continue;
            }
            if !self.pending_alpaca_fetches.insert(fetch_key) {
                continue;
            }
            self.mark_fetch_queued("alpaca", &symbol, tf);
            by_tf.entry(tf.to_string()).or_default().push(symbol);
            dispatched += 1;
        }
        for (timeframe, symbols) in by_tf {
            let chunk_symbols = Self::alpaca_batch_fetch_chunk_symbols(&timeframe);
            for chunk in symbols.chunks(chunk_symbols) {
                let _ = self.broker_tx.send(BrokerCmd::AlpacaFetchBarsBatch {
                    symbols: chunk.to_vec(),
                    timeframe: timeframe.clone(),
                });
            }
        }
        dispatched
    }

    pub(super) fn queue_alpaca_fetch(&mut self, symbol: &str, timeframe: &str) -> bool {
        let Some(tf) = normalize_sync_timeframe_key(timeframe) else {
            return false;
        };
        if !self.alpaca_enabled || !self.broker_connected || !self.sync_timeframe_enabled(tf) {
            return false;
        }
        let symbol = normalize_market_data_symbol(symbol).replace('/', "");
        if !self.alpaca_no_data_loaded {
            self.alpaca_no_data_load();
        }
        if self
            .alpaca_no_data_pairs
            .contains_key(&alpaca_fetch_key(&symbol, tf))
            || self.is_unresolvable_fetch_key("alpaca", &symbol, tf)
        {
            self.log.push_back(LogEntry::warn(format!(
                "Alpaca {} {}: known no-data symbol — skipping fetch",
                symbol, tf
            )));
            return false;
        }
        if !self.alpaca_backfill_complete_loaded {
            self.alpaca_backfill_complete_load();
        }
        if self.is_fetch_on_cooldown("alpaca", &symbol, tf) {
            return false;
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
        let now_s = chrono::Utc::now().timestamp();
        let state = self
            .cached_alpaca_sync_state
            .get(&(symbol.clone(), tf.to_string()))
            .copied();
        let focus = self.cached_active_symbols.iter().any(|candidate| {
            normalize_market_data_symbol(candidate)
                .replace('/', "")
                .eq_ignore_ascii_case(&symbol)
        });
        if !background_market_data_fetch_allowed(focus, self.total_pending_market_data_fetches()) {
            return false;
        }
        let Some(_) = classify_alpaca_sync_candidate(
            now_s,
            &symbol,
            tf,
            state,
            focus,
            alpaca_sync_target_bars,
        ) else {
            return false;
        };
        let fetch_key = alpaca_fetch_key(&symbol, tf);
        let backfill_complete = self.alpaca_backfill_complete_pairs.contains_key(&fetch_key);
        if !self.pending_alpaca_fetches.insert(fetch_key) {
            return false;
        }
        self.mark_fetch_queued("alpaca", &symbol, tf);
        let _ = self.broker_tx.send(BrokerCmd::AlpacaFetchBars {
            symbol,
            timeframe: tf.to_string(),
            db_path: cache_db_path(),
            backfill_complete,
        });
        true
    }

    /// `true` if the Kraken WS OHLC pipeline pushed a bar for this
    /// (symbol, tf) within the last `TF_period × 24` — the same staleness
    /// rule the Sync Status window uses. O(1): one HashMap lookup + a
    /// constant-time arithmetic check.
    ///
    /// Note we don't actively prune `kraken_ws_fresh_until`; entries age
    /// out naturally because the inner predicate compares against `now_ms`
    /// every call. A long-running session with churning WS subscriptions
    /// will accumulate stale entries proportional to the number of
    /// (symbol, tf) tuples Kraken has ever served us — bounded by the
    /// universe size (≈ 100k), and a single i64 per entry.
    pub(super) fn kraken_ws_pair_is_fresh(&self, symbol: &str, tf: &str) -> bool {
        let now_ms = chrono::Utc::now().timestamp_millis();
        Self::kraken_ws_pair_is_fresh_at(&self.kraken_ws_fresh_until, symbol, tf, now_ms)
    }

    pub(super) fn kraken_ws_pair_is_fresh_at(
        fresh_map: &std::collections::HashMap<(String, String), i64>,
        symbol: &str,
        tf: &str,
        now_ms: i64,
    ) -> bool {
        let Some(period_s) = sync_timeframe_period_secs(tf) else {
            return false;
        };
        let key = (symbol.to_string(), tf.to_string());
        let Some(&fresh_anchor_ms) = fresh_map.get(&key) else {
            return false;
        };
        let max_age_ms = period_s.saturating_mul(1000).saturating_mul(24);
        now_ms.saturating_sub(fresh_anchor_ms) < max_age_ms
    }

    pub(super) fn queue_kraken_fetch(&mut self, symbol: &str, timeframe: &str) -> bool {
        let Some(tf) = normalize_sync_timeframe_key(timeframe) else {
            return false;
        };
        if !self.kraken_enabled
            || !self.sync_timeframe_enabled(tf)
            || !kraken_spot_native_timeframe(tf)
        {
            return false;
        }
        let symbol = typhoon_engine::core::kraken::normalize_pair_symbol(symbol);
        if symbol.is_empty()
            || typhoon_engine::core::kraken::to_kraken_pair_lossy(&symbol).is_none()
            || !self.kraken_spot_symbol_scrape_enabled(&symbol)
        {
            return false;
        }
        if self.is_unresolvable_fetch_key("kraken", &symbol, tf) {
            return false;
        }
        if self.is_fetch_on_cooldown("kraken", &symbol, tf) {
            return false;
        }
        // O(1) WS-fresh skip: if the Kraken WS OHLC pipeline pushed a bar
        // for this (symbol, tf) recently enough that REST refetch can't
        // produce anything newer, drop the REST request entirely. This is
        // the whole point of the WS feed for low timeframes — keep REST's
        // ~55 req/min budget on the high-TF backfill where it still wins.
        if self.kraken_ws_pair_is_fresh(&symbol, tf) {
            return false;
        }
        if !self.kraken_backfill_complete_loaded {
            self.kraken_backfill_complete_load();
        }
        let fetch_key = alpaca_fetch_key(&symbol, tf);
        let backfill_complete = self.kraken_backfill_complete_pairs.contains_key(&fetch_key);
        if !self.pending_kraken_fetches.insert(fetch_key) {
            return false;
        }
        self.mark_fetch_queued("kraken", &symbol, tf);
        let _ = self.broker_tx.send(BrokerCmd::KrakenBackfill {
            symbol: symbol.clone(),
            timeframes: vec![tf.to_string()],
            db_path: cache_db_path(),
            backfill_complete,
        });
        true
    }

    /// Dispatch a Kraken equity ticker fetch unless the iapi host is currently
    /// rate-limited. Cloudflare 1015 / HTTP 429 from any iapi endpoint arms a
    /// shared back-off in the engine; checking here means we suppress the
    /// dispatch (and the noisy error round-trip) until the window clears.
    /// Returns whether the command was actually sent.
    pub(super) fn dispatch_kraken_equity_ticker(&self, symbol: &str) -> bool {
        if typhoon_engine::broker::kraken::iapi_rate_limited_for_secs().is_some() {
            return false;
        }
        let _ = self.broker_tx.send(BrokerCmd::KrakenFetchEquityTicker {
            symbol: symbol.to_string(),
        });
        true
    }

    pub(super) fn queue_yahoo_chart_fetch(&mut self, symbol: &str, timeframe: &str) -> bool {
        let Some(tf) = normalize_sync_timeframe_key(timeframe) else {
            return false;
        };
        if !self.backfill_yahoo_chart_enabled || !self.sync_timeframe_enabled(tf) {
            return false;
        }
        if self.yahoo_chart_sync_pause_until_ts > chrono::Utc::now().timestamp() {
            return false;
        }
        if !yahoo_chart_supports_timeframe(tf) {
            return false;
        }
        let symbol = normalize_market_data_symbol(symbol)
            .replace('/', "")
            .trim_end_matches(".EQ")
            .to_ascii_uppercase();
        if symbol.is_empty()
            || self.is_unresolvable_fetch_key("yahoo-chart", &symbol, tf)
            || self.is_fetch_on_cooldown("yahoo-chart", &symbol, tf)
        {
            return false;
        }
        let fetch_key = alpaca_fetch_key(&symbol, tf);
        if !self.pending_yahoo_chart_fetches.insert(fetch_key) {
            return false;
        }
        self.mark_fetch_queued("yahoo-chart", &symbol, tf);
        let _ = self.broker_tx.send(BrokerCmd::YahooChartFetchBars {
            symbol,
            timeframe: tf.to_string(),
        });
        true
    }

    pub(super) fn queue_kraken_equity_fetch(&mut self, symbol: &str, timeframe: &str) -> bool {
        let Some(tf) = normalize_sync_timeframe_key(timeframe) else {
            return false;
        };
        if !self.kraken_enabled
            || !self.sync_timeframe_enabled(tf)
            || !kraken_equity_full_universe_timeframe(tf)
        {
            return false;
        }
        if self.kraken_equities_sync_pause_until_ts > chrono::Utc::now().timestamp() {
            return false;
        }
        let symbol = normalize_market_data_symbol(symbol)
            .replace('/', "")
            .trim_end_matches(".EQ")
            .to_ascii_uppercase();
        if symbol.is_empty() {
            return false;
        }
        // WS v2 OHLC is now the primary xStocks current-bar path. Once it has
        // delivered a fresh closed bar for this symbol/timeframe, suppress the
        // REST/iapi history pull; keep REST for initial cold-start/gap repair
        // and for cases where WS has not produced this tuple yet.
        if self.kraken_ws_pair_is_fresh(&symbol, tf) {
            return false;
        }
        if self.is_unresolvable_fetch_key("kraken-equities", &symbol, tf) {
            return false;
        }
        // Cooldown gate: after a completed fetch, don't re-queue the same
        // SYMBOL:TF more often than half the TF period. Without this, the
        // KrakenBalances tick (~every minute) re-queues WOK 30Min every
        // minute even when the market is closed and the cached bars are
        // already current.
        if self.is_fetch_on_cooldown("kraken-equities", &symbol, tf) {
            return false;
        }
        let fetch_key = alpaca_fetch_key(&symbol, tf);
        if !self
            .pending_kraken_fetches
            .insert(format!("equity:{fetch_key}"))
        {
            return false;
        }
        self.mark_fetch_queued("kraken-equities", &symbol, tf);
        let _ = self.broker_tx.send(BrokerCmd::KrakenFetchEquityHistory {
            symbol: symbol.clone(),
            timeframe: tf.to_string(),
        });
        let alpaca_assist_queued = if self.backfill_alpaca_kraken_equities_enabled {
            // Assist-only path: queue the same Kraken-equity symbol/timeframe into
            // Alpaca as provenance-tagged fallback data. This deliberately does
            // not enumerate the Alpaca universe; it only follows Kraken equity
            // candidates that the Kraken scheduler already selected.
            self.queue_alpaca_fetch(&symbol, tf)
        } else {
            false
        };
        let yahoo_assist_queued = self.queue_yahoo_chart_fetch(&symbol, tf);
        let chart_or_owned = self.charts.iter().any(|chart| {
            normalize_market_data_symbol(&chart.symbol)
                .replace('/', "")
                .trim_end_matches(".EQ")
                .eq_ignore_ascii_case(&symbol)
        }) || self.kraken_balances.iter().any(|(asset, _)| {
            Self::kraken_display_asset(asset)
                .trim_end_matches(".EQ")
                .eq_ignore_ascii_case(&symbol)
        });
        if chart_or_owned {
            // Multi-TF refills push one line per TF in quick succession
            // (1Hour + 30Min + 15Min + 5Min, etc.). Visible in tracing for
            // diagnostics but no longer stacking four user-log entries per
            // symbol refresh.
            tracing::debug!(
                "Kraken equities sync queued {} {} (alpaca_assist={} yahoo_assist={})",
                symbol,
                tf,
                alpaca_assist_queued,
                yahoo_assist_queued
            );
        }
        true
    }

    pub(super) fn queue_kraken_futures_fetch(&mut self, symbol: &str, timeframe: &str) -> bool {
        if !self.kraken_enabled || !self.kraken_scrape_futures {
            return false;
        }
        let Some(tf) = normalize_sync_timeframe_key(timeframe) else {
            return false;
        };
        if !self.sync_timeframe_enabled(tf) || tf == "1Month" {
            return false;
        }
        let symbol = typhoon_engine::core::kraken_futures::normalize_futures_symbol(symbol);
        if symbol.is_empty() || !typhoon_engine::core::kraken_futures::is_futures_symbol(&symbol) {
            return false;
        }
        if self.is_unresolvable_fetch_key("kraken-futures", &symbol, tf) {
            return false;
        }
        if self.is_fetch_on_cooldown("kraken-futures", &symbol, tf) {
            return false;
        }
        if !self.kraken_futures_backfill_complete_loaded {
            self.kraken_futures_backfill_complete_load();
        }
        let fetch_key = alpaca_fetch_key(&symbol, tf);
        let backfill_complete = self
            .kraken_futures_backfill_complete_pairs
            .contains_key(&fetch_key);
        if !self.pending_kraken_futures_fetches.insert(fetch_key) {
            return false;
        }
        self.mark_fetch_queued("kraken-futures", &symbol, tf);
        let _ = self.broker_tx.send(BrokerCmd::KrakenFuturesBackfill {
            symbol: symbol.clone(),
            timeframes: vec![tf.to_string()],
            db_path: cache_db_path(),
            backfill_complete,
        });
        self.log.push_back(LogEntry::info(format!(
            "Kraken Futures sync queued {} {} ({} pending)",
            symbol,
            tf,
            self.pending_kraken_futures_fetches.len()
        )));
        true
    }

    pub(super) fn settle_market_data_fetch(&mut self, source: &str, symbol: &str, timeframe: &str) {
        if source.eq_ignore_ascii_case("kraken-equities") {
            self.pending_kraken_fetches
                .remove(&format!("equity:{}", alpaca_fetch_key(symbol, timeframe)));
            return;
        }
        self.pending_fetches_for_source_mut(source)
            .remove(&alpaca_fetch_key(symbol, timeframe));
    }

    fn ensure_unresolvable_fetch_key_index(&mut self) {
        if self.unresolvable_fetch_keys_by_broker.is_empty() && !self.unresolvable_pairs.is_empty()
        {
            self.rebuild_unresolvable_fetch_key_index();
        }
    }

    fn is_unresolvable_fetch_key(&self, broker: &str, symbol: &str, timeframe: &str) -> bool {
        let fetch_key = alpaca_fetch_key(symbol, timeframe);
        self.unresolvable_fetch_keys_by_broker
            .get(&broker.to_ascii_lowercase())
            .is_some_and(|keys| keys.contains(&fetch_key))
            || self
                .unresolvable_pairs
                .contains_key(&unresolvable_pair_key(broker, symbol, timeframe))
    }

    /// True if this source/symbol/tf was queued recently enough that re-queueing
    /// now would just hit the same cache slot before a new bar could exist.
    /// Uses ~half the TF period as the cooldown so we still refresh well within
    /// one bar's worth of time during market hours.
    pub(super) fn is_fetch_on_cooldown(&self, source: &str, symbol: &str, timeframe: &str) -> bool {
        let Some(period_s) = sync_timeframe_period_secs(timeframe) else {
            return false;
        };
        let key = format!("{}:{}:{}", source, symbol, timeframe);
        let Some(last) = self.fetch_last_queued_ts.get(&key).copied() else {
            return false;
        };
        let now_s = chrono::Utc::now().timestamp();
        let cooldown = (period_s / 2).max(30);
        now_s.saturating_sub(last) < cooldown
    }

    pub(super) fn mark_fetch_queued(&mut self, source: &str, symbol: &str, timeframe: &str) {
        let key = format!("{}:{}:{}", source, symbol, timeframe);
        self.fetch_last_queued_ts
            .insert(key, chrono::Utc::now().timestamp());
    }

    pub(super) fn schedule_alpaca_pairs(&mut self, symbols: &[String]) -> usize {
        if !self.alpaca_enabled
            || !self.broker_connected
            || !self.alpaca_full_bar_sync_enabled
            || symbols.is_empty()
        {
            return 0;
        }

        let timeframes = self.enabled_standard_sync_timeframes();
        if timeframes.is_empty() {
            return 0;
        }

        let capacity = self.alpaca_sync_capacity();
        let available_slots = capacity
            .queue_window
            .saturating_sub(self.pending_alpaca_fetches.len())
            .min(capacity.batch_size);
        if available_slots == 0 {
            return 0;
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
        let focus_symbols = self.alpaca_focus_symbols();
        if !self.alpaca_no_data_loaded {
            self.alpaca_no_data_load();
        }
        if !self.alpaca_backfill_complete_loaded {
            self.alpaca_backfill_complete_load();
        }
        self.ensure_unresolvable_fetch_key_index();
        let now_s = chrono::Utc::now().timestamp();
        let mut no_data_keys: std::collections::HashSet<String> =
            self.alpaca_no_data_pairs.keys().cloned().collect();
        if let Some(unresolvable) = self.unresolvable_fetch_keys_by_broker.get("alpaca") {
            no_data_keys.extend(unresolvable.iter().cloned());
        }
        no_data_keys.extend(
            self.alpaca_retry_queue
                .iter()
                .map(|retry| alpaca_fetch_key(&retry.symbol, &retry.timeframe)),
        );
        let mut cursor = self.alpaca_sync_cursor;
        let candidates = select_alpaca_sync_workset_rotating(
            symbols,
            &timeframes,
            &self.cached_alpaca_sync_state,
            &focus_symbols,
            &no_data_keys,
            &self.alpaca_backfill_complete_pairs,
            &self.pending_alpaca_fetches,
            available_slots,
            capacity.foreground_reserve,
            if self.full_tilt_sync_enabled() {
                ALPACA_FULL_TILT_BACKGROUND_SCAN_LIMIT
            } else {
                ALPACA_BACKGROUND_SCAN_LIMIT
            },
            &mut cursor,
            now_s,
            alpaca_sync_target_bars,
        );
        self.alpaca_sync_cursor = cursor;

        let mut dispatched = 0usize;
        dispatched += self.queue_alpaca_batch_fetches_from_candidates(candidates);
        dispatched
    }

    pub(super) fn schedule_kraken_pairs_with_budget(
        &mut self,
        sector_idx: usize,
        symbols: &[String],
        batch_limit: usize,
        foreground_slots: usize,
    ) -> usize {
        if !self.kraken_enabled || symbols.is_empty() {
            return 0;
        }
        let timeframes = self.enabled_standard_sync_timeframes();
        if timeframes.is_empty() {
            return 0;
        }
        let full_tilt = self.full_tilt_sync_enabled();
        let queue_window = if full_tilt {
            KRAKEN_SPOT_FULL_TILT_QUEUE_WINDOW
        } else {
            KRAKEN_SPOT_QUEUE_WINDOW
        };
        let batch_limit = if full_tilt {
            batch_limit.max(128)
        } else {
            batch_limit
        };
        let foreground_slots = if full_tilt {
            foreground_slots.max(16)
        } else {
            foreground_slots
        };
        let scan_limit = if full_tilt {
            KRAKEN_SPOT_FULL_TILT_BACKGROUND_SCAN_LIMIT
        } else {
            KRAKEN_SPOT_BACKGROUND_SCAN_LIMIT
        };
        let available_slots = queue_window
            .saturating_sub(self.pending_kraken_fetches.len())
            .min(batch_limit);
        if available_slots == 0 {
            return 0;
        }
        if self.cached_kraken_sync_state_rev != Some(self.bg_rev)
            && (!self.heavy_sync_in_progress || self.cached_kraken_sync_state.is_empty())
        {
            let previous = self.cached_kraken_sync_state.clone();
            let mut rebuilt = self.build_source_cache_state_map("kraken:");
            merge_recent_sync_overrides(&mut rebuilt, &previous, chrono::Utc::now().timestamp());
            self.cached_kraken_sync_state = rebuilt;
            self.cached_kraken_sync_state_rev = Some(self.bg_rev);
        }
        if !self.kraken_backfill_complete_loaded {
            self.kraken_backfill_complete_load();
        }
        self.ensure_unresolvable_fetch_key_index();
        let focus_symbols = self.market_data_focus_symbols();
        let empty_no_data_keys = std::collections::HashSet::new();
        let no_data_keys = self
            .unresolvable_fetch_keys_by_broker
            .get("kraken")
            .unwrap_or(&empty_no_data_keys);
        let now_s = chrono::Utc::now().timestamp();
        let cursor_idx = sector_idx.min(self.kraken_spot_sync_cursors.len().saturating_sub(1));
        let mut cursor = self.kraken_spot_sync_cursors[cursor_idx];
        let candidates = select_alpaca_sync_workset_rotating(
            symbols,
            &timeframes,
            &self.cached_kraken_sync_state,
            &focus_symbols,
            no_data_keys,
            &self.kraken_backfill_complete_pairs,
            &self.pending_kraken_fetches,
            available_slots,
            foreground_slots,
            scan_limit,
            &mut cursor,
            now_s,
            kraken_sync_target_bars,
        );
        self.kraken_spot_sync_cursors[cursor_idx] = cursor;
        let mut dispatched = 0usize;
        for candidate in candidates {
            if self.queue_kraken_fetch(&candidate.symbol, &candidate.timeframe) {
                dispatched += 1;
            }
        }
        dispatched
    }

    pub(super) fn schedule_kraken_futures_pairs_with_budget(
        &mut self,
        sector_idx: usize,
        symbols: &[String],
        batch_limit: usize,
        foreground_slots: usize,
    ) -> usize {
        if !self.kraken_enabled || symbols.is_empty() {
            return 0;
        }
        let timeframes = self.enabled_standard_sync_timeframes();
        if timeframes.is_empty() {
            return 0;
        }
        let full_tilt = self.full_tilt_sync_enabled();
        let queue_window = if full_tilt {
            KRAKEN_FUTURES_FULL_TILT_QUEUE_WINDOW
        } else {
            KRAKEN_FUTURES_QUEUE_WINDOW
        };
        let batch_limit = if full_tilt {
            batch_limit.max(96)
        } else {
            batch_limit
        };
        let foreground_slots = if full_tilt {
            foreground_slots.max(12)
        } else {
            foreground_slots
        };
        let scan_limit = if full_tilt {
            KRAKEN_FUTURES_FULL_TILT_BACKGROUND_SCAN_LIMIT
        } else {
            KRAKEN_FUTURES_BACKGROUND_SCAN_LIMIT
        };
        let available_slots = queue_window
            .saturating_sub(self.pending_kraken_futures_fetches.len())
            .min(batch_limit);
        if available_slots == 0 {
            return 0;
        }
        if self.cached_kraken_futures_sync_state_rev != Some(self.bg_rev)
            && (!self.heavy_sync_in_progress || self.cached_kraken_futures_sync_state.is_empty())
        {
            let previous = self.cached_kraken_futures_sync_state.clone();
            let mut rebuilt = self.build_source_cache_state_map("kraken-futures:");
            merge_recent_sync_overrides(&mut rebuilt, &previous, chrono::Utc::now().timestamp());
            self.cached_kraken_futures_sync_state = rebuilt;
            self.cached_kraken_futures_sync_state_rev = Some(self.bg_rev);
        }
        if !self.kraken_futures_backfill_complete_loaded {
            self.kraken_futures_backfill_complete_load();
        }
        self.ensure_unresolvable_fetch_key_index();
        let focus_symbols = self.market_data_focus_symbols();
        let empty_no_data_keys = std::collections::HashSet::new();
        let no_data_keys = self
            .unresolvable_fetch_keys_by_broker
            .get("kraken-futures")
            .unwrap_or(&empty_no_data_keys);
        let now_s = chrono::Utc::now().timestamp();
        let cursor_idx = sector_idx.min(self.kraken_futures_sync_cursors.len().saturating_sub(1));
        let mut cursor = self.kraken_futures_sync_cursors[cursor_idx];
        let candidates = select_alpaca_sync_workset_rotating(
            symbols,
            &timeframes,
            &self.cached_kraken_futures_sync_state,
            &focus_symbols,
            no_data_keys,
            &self.kraken_futures_backfill_complete_pairs,
            &self.pending_kraken_futures_fetches,
            available_slots,
            foreground_slots,
            scan_limit,
            &mut cursor,
            now_s,
            kraken_futures_sync_target_bars,
        );
        self.kraken_futures_sync_cursors[cursor_idx] = cursor;
        let mut dispatched = 0usize;
        for candidate in candidates {
            if self.queue_kraken_futures_fetch(&candidate.symbol, &candidate.timeframe) {
                dispatched += 1;
            }
        }
        dispatched
    }

    pub(super) fn maybe_request_alpaca_asset_universe(&mut self) {
        if self.alpaca_enabled
            && self.alpaca_full_bar_sync_enabled
            && !self.all_broker_assets_fetched
            && self.broker_connected
        {
            let _ = self.broker_tx.send(BrokerCmd::GetAllAssets);
            self.all_broker_assets_fetched = true;
        }
    }

    pub(super) fn alpaca_equity_rotation_symbols(&self) -> Vec<String> {
        let mut equity_set: std::collections::HashSet<String> =
            std::collections::HashSet::with_capacity(self.all_broker_assets.len() + 64);
        let mut equity_syms: Vec<String> = Vec::with_capacity(self.all_broker_assets.len() + 64);

        for (sym, _name, class) in &self.all_broker_assets {
            if class != "us_equity" {
                continue;
            }
            let su = sym.to_uppercase();
            if Self::demand_is_crypto(&su) {
                continue;
            }
            if equity_set.insert(su.clone()) {
                equity_syms.push(su);
            }
        }
        for chart in &self.charts {
            let bare = bare_symbol_from_key(&chart.symbol).to_uppercase();
            if Self::demand_is_crypto(&bare) {
                continue;
            }
            if equity_set.insert(bare.clone()) {
                equity_syms.push(bare);
            }
        }
        for wl in &self.user_watchlist {
            let wlu = wl.to_uppercase();
            if Self::demand_is_crypto(&wlu) {
                continue;
            }
            if equity_set.insert(wlu.clone()) {
                equity_syms.push(wlu);
            }
        }
        equity_syms.sort();
        equity_syms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kraken_equity_native_symbols_for_timeframe_is_demand_scoped() {
        let catalog = vec!["TNDM.EQ".to_string(), "wok".to_string(), "TNDM".to_string()];
        let demand = vec!["POM.EQ".to_string(), "array".to_string()];

        // Native rows count only the demand set, regardless of catalog size, so
        // the "Kraken Equities" status row can converge to ~100%.
        for tf in ["15Min", "1Day", "1Week"] {
            assert_eq!(
                kraken_equity_native_symbols_for_timeframe(&catalog, &demand, tf),
                vec!["ARRAY".to_string(), "POM".to_string()],
                "{tf} native row should be demand-scoped"
            );
        }
        assert!(kraken_equity_native_symbols_for_timeframe(&catalog, &demand, "1Month").is_empty());
        assert_eq!(
            kraken_equity_native_symbols_for_timeframe(&[], &demand, "1Day"),
            vec!["ARRAY".to_string(), "POM".to_string()]
        );
    }

    #[test]
    fn kraken_equity_symbols_for_timeframe_uses_catalog_for_all_supported_merged_timeframes() {
        let catalog = vec!["TNDM.EQ".to_string(), "wok".to_string(), "TNDM".to_string()];
        let demand = vec!["POM.EQ".to_string(), "array".to_string()];

        for tf in ["1Min", "5Min", "15Min", "1Day", "1Week", "1Month"] {
            assert_eq!(
                kraken_equity_symbols_for_timeframe(&catalog, &demand, tf),
                vec!["TNDM".to_string(), "WOK".to_string()],
                "{tf} should be catalog-scoped when the Kraken Equities universe is loaded"
            );
        }
        assert_eq!(
            kraken_equity_symbols_for_timeframe(&[], &demand, "1Day"),
            vec!["ARRAY".to_string(), "POM".to_string()]
        );
        assert_eq!(
            kraken_equity_symbols_for_timeframe(&[], &demand, "15Min"),
            vec!["ARRAY".to_string(), "POM".to_string()]
        );
    }

    #[test]
    fn normalize_kraken_equity_symbol_list_strips_wrappers_and_dedupes() {
        let raw = vec![
            "tndm.eq".to_string(),
            "TNDM".to_string(),
            "".to_string(),
            "w/ok.EQ".to_string(),
        ];
        assert_eq!(
            normalize_kraken_equity_symbol_list(raw.iter()),
            vec!["TNDM".to_string(), "WOK".to_string()]
        );
    }

    fn symbols(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn kraken_equity_native_history_is_demand_scoped_not_full_catalog() {
        let catalog = symbols(&["AAPL", "MSFT", "NVDA", "TSLA"]);
        let demand = symbols(&["WOK.EQ", "AAPLx/USD"]);

        let selected = kraken_equity_native_history_symbols(&catalog, &demand);

        // iapi (~6 req/s, 1015-banned on overshoot) owns only the demand depth
        // lane; the catalog's breadth is carried by Alpaca/Yahoo + the merge.
        assert_eq!(selected, symbols(&["AAPLXUSD", "WOK"]));
    }

    #[test]
    fn kraken_equity_native_history_falls_back_to_demand_before_catalog_loads() {
        let catalog: Vec<String> = Vec::new();
        let demand = symbols(&["WOK.EQ", "AAPLx/USD"]);

        let selected = kraken_equity_native_history_symbols(&catalog, &demand);

        // Until the catalog has loaded, repair the active/held/watchlist set so
        // open charts still backfill instead of waiting on the universe fetch.
        assert_eq!(selected, symbols(&["AAPLXUSD", "WOK"]));
    }

    #[test]
    fn kraken_equity_native_is_demand_scoped_while_assist_lanes_stay_catalog_broad() {
        let catalog = symbols(&["MSFT", "AAPL"]);
        let demand = symbols(&["WOK.EQ"]);

        assert_eq!(
            kraken_equity_native_symbols_for_timeframe(&catalog, &demand, "1Day"),
            symbols(&["WOK"]),
            "native iapi/WS rows are demand-scoped depth, not catalog breadth"
        );
        assert_eq!(
            kraken_equity_symbols_for_timeframe(&catalog, &demand, "1Month"),
            symbols(&["AAPL", "MSFT"]),
            "assist/merged broad lanes (Alpaca/Yahoo) still rotate over the catalog"
        );
    }

    #[test]
    fn background_retry_dispatch_stops_when_pending_pressure_is_high() {
        assert!(background_retry_dispatch_allowed(0));
        assert!(background_retry_dispatch_allowed(
            BACKGROUND_RETRY_PENDING_FETCH_CAP - 1
        ));
        assert!(!background_retry_dispatch_allowed(
            BACKGROUND_RETRY_PENDING_FETCH_CAP
        ));
    }

    #[test]
    fn background_fetch_backpressure_preserves_focus_symbols() {
        assert!(!background_market_data_fetch_allowed(
            false,
            BACKGROUND_RETRY_PENDING_FETCH_CAP
        ));
        // RSS guard is environment-dependent; we only assert the pending_fetches path here.

        assert!(background_market_data_fetch_allowed(true, 0));
        assert!(background_market_data_fetch_allowed(
            true,
            BACKGROUND_RETRY_PENDING_FETCH_CAP * 10
        ));
    }
}
