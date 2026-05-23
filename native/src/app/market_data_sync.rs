use super::*;

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
        super::auto_compact::on_ac_power()
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
        if self.user_interacting {
            // Avoid cache misses doing SQLite reads while the user is dragging/zooming;
            // cached sparklines still render above, misses populate after interaction ends.
            return std::sync::Arc::new(Vec::new());
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
                format!("mt5:{}:1Day", symbol),
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
                "earnings": false, "dividends": false, "darwinex_outliers": false,
                "darwinex_radar": false, "swap_harvest": false, "darwin_browser": false,
                "stress_test": false, "volume_profile": true, "hv_cone": false,
                "sector_heatmap": false, "dividends_screen": false, "event_calendar": false,
                "lan_sync": false, "alerts": true, "journal": false, "compact_mode": false,
            }),
            "RESEARCH" => serde_json::json!({
                "sec": true, "insider": true, "fundamentals": true, "ev": true,
                "earnings": true, "dividends": true, "darwinex_outliers": true,
                "darwinex_radar": false, "swap_harvest": false, "darwin_browser": false,
                "stress_test": false, "volume_profile": false, "hv_cone": false,
                "sector_heatmap": true, "dividends_screen": true, "event_calendar": true,
                "lan_sync": false, "alerts": false, "journal": false, "compact_mode": false,
            }),
            "DARWIN" => serde_json::json!({
                "sec": false, "insider": false, "fundamentals": false, "ev": false,
                "earnings": false, "dividends": false, "darwinex_outliers": true,
                "darwinex_radar": true, "swap_harvest": true, "darwin_browser": true,
                "stress_test": true, "volume_profile": false, "hv_cone": false,
                "sector_heatmap": false, "dividends_screen": false, "event_calendar": false,
                "lan_sync": false, "alerts": false, "journal": true, "compact_mode": false,
            }),
            "COMPACT" => serde_json::json!({
                "sec": false, "insider": false, "fundamentals": false, "ev": false,
                "earnings": false, "dividends": false, "darwinex_outliers": false,
                "darwinex_radar": false, "swap_harvest": false, "darwin_browser": false,
                "stress_test": false, "volume_profile": false, "hv_cone": false,
                "sector_heatmap": false, "dividends_screen": false, "event_calendar": false,
                "lan_sync": false, "alerts": false, "journal": false, "compact_mode": true,
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
            "darwinex_outliers": self.show_darwinex_outliers,
            "darwinex_radar": self.show_darwinex_radar,
            "swap_harvest": self.show_swap_harvest,
            "darwin_browser": self.show_darwin_browser,
            "stress_test": self.show_stress_test,
            "volume_profile": self.show_volume_profile,
            "hv_cone": self.show_hv_cone,
            "sector_heatmap": self.show_sector_heatmap,
            "dividends_screen": self.show_dividends,
            "event_calendar": self.show_event_calendar,
            "lan_sync": self.show_lan_sync,
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
        set_bool!("darwinex_outliers", show_darwinex_outliers);
        set_bool!("darwinex_radar", show_darwinex_radar);
        set_bool!("swap_harvest", show_swap_harvest);
        set_bool!("darwin_browser", show_darwin_browser);
        set_bool!("stress_test", show_stress_test);
        set_bool!("volume_profile", show_volume_profile);
        set_bool!("hv_cone", show_hv_cone);
        set_bool!("sector_heatmap", show_sector_heatmap);
        set_bool!("dividends_screen", show_dividends);
        set_bool!("event_calendar", show_event_calendar);
        set_bool!("lan_sync", show_lan_sync);
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
            "tastytrade" => &mut self.pending_tastytrade_fetches,
            _ => &mut self.pending_alpaca_fetches,
        }
    }

    pub(super) fn total_pending_market_data_fetches(&self) -> usize {
        self.pending_alpaca_fetches.len()
            + self.pending_kraken_fetches.len()
            + self.pending_kraken_futures_fetches.len()
            + self.pending_tastytrade_fetches.len()
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
            "tastytrade" => {
                self.cached_tastytrade_sync_state
                    .insert((symbol.clone(), tf.to_string()), state);
                self.cached_tastytrade_sync_state_rev = Some(self.bg_rev);
                self.cached_tastytrade_symbols.insert(symbol);
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

    pub(super) fn kraken_equity_sync_symbols(&self) -> Vec<String> {
        if !self.kraken_enabled || !self.kraken_scrape_xstocks {
            return Vec::new();
        }
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        let mut push_symbol = |source: &str| {
            let symbol = normalize_market_data_symbol(source)
                .replace('/', "")
                .trim_end_matches(".EQ")
                .to_ascii_uppercase();
            if !symbol.is_empty() && seen.insert(symbol.clone()) {
                out.push(symbol);
            }
        };

        for symbol in self.kraken_equity_universe_symbols.clone() {
            push_symbol(&symbol);
        }
        // Fallback/augmentation while the full catalog is loading: include owned,
        // charted, watched, and any symbols Kraken Spot exposes as xStock-looking pairs.
        for (pair_name, display_name) in &self.kraken_pairs {
            if let Some(symbol) = kraken_xstock_fundamental_symbol(pair_name, display_name) {
                push_symbol(&symbol);
            }
        }
        for (asset, qty) in &self.kraken_balances {
            if qty.is_finite() && *qty > 0.0 && Self::kraken_display_asset(asset).ends_with(".EQ") {
                push_symbol(&Self::kraken_display_asset(asset));
            }
        }
        for chart in &self.charts {
            let source = cache_source_from_key(&chart.symbol);
            let bare = bare_symbol_from_key(&chart.symbol);
            if source == "kraken-equities" || bare.to_ascii_uppercase().ends_with(".EQ") {
                push_symbol(&bare);
            }
        }
        for symbol in &self.user_watchlist {
            if symbol.to_ascii_uppercase().ends_with(".EQ") {
                push_symbol(symbol);
            }
        }
        out.sort();
        out
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
        if !self.kraken_enabled || !self.kraken_any_spot_scrape_enabled() {
            return 0;
        }
        let sectors = self.kraken_sync_symbol_sectors();
        let budgets = [12usize, 16, 12, 12];
        let mut dispatched = 0usize;
        for (idx, (sector, budget)) in sectors.iter().zip(budgets).enumerate() {
            if !self.kraken_spot_sector_scrape_enabled(idx) {
                continue;
            }
            dispatched += self.schedule_kraken_pairs_with_budget(idx, sector, budget, 4);
        }
        dispatched
    }

    pub(super) fn schedule_kraken_equities_universe(&mut self) -> usize {
        let symbols = self.kraken_equity_sync_symbols();
        if !self.kraken_enabled || symbols.is_empty() {
            return 0;
        }
        if self.kraken_equities_sync_pause_until_ts > chrono::Utc::now().timestamp() {
            return 0;
        }
        let timeframes = self.enabled_standard_sync_timeframes();
        if timeframes.is_empty() {
            return 0;
        }
        let full_tilt = self.full_tilt_sync_enabled();
        let queue_window: usize = if self.user_interacting && !full_tilt {
            3
        } else if full_tilt {
            KRAKEN_EQUITIES_FULL_TILT_QUEUE_WINDOW
        } else {
            8
        };
        let batch_limit: usize = if self.user_interacting && !full_tilt {
            1
        } else if full_tilt {
            KRAKEN_EQUITIES_FULL_TILT_BATCH_SIZE
        } else {
            4
        };
        let foreground_slots = if self.user_interacting && !full_tilt {
            1
        } else if full_tilt {
            KRAKEN_EQUITIES_FULL_TILT_BATCH_SIZE
        } else {
            4
        };
        let available_slots = queue_window
            .saturating_sub(
                self.pending_kraken_fetches
                    .iter()
                    .filter(|key| key.starts_with("equity:"))
                    .count(),
            )
            .min(batch_limit);
        if available_slots == 0 {
            return 0;
        }
        if self.cached_kraken_equities_sync_state_rev != Some(self.bg_rev) {
            let previous = self.cached_kraken_equities_sync_state.clone();
            let mut rebuilt = self.build_source_cache_state_map("kraken-equities:");
            merge_recent_sync_overrides(&mut rebuilt, &previous, chrono::Utc::now().timestamp());
            self.cached_kraken_equities_sync_state = rebuilt;
            self.cached_kraken_equities_sync_state_rev = Some(self.bg_rev);
        }
        self.ensure_unresolvable_fetch_key_index();
        let focus_symbols = self.market_data_focus_symbols();
        let empty_no_data_keys = std::collections::HashSet::new();
        let no_data_keys = self
            .unresolvable_fetch_keys_by_broker
            .get("kraken-equities")
            .unwrap_or(&empty_no_data_keys);
        let empty_backfill = std::collections::HashMap::new();
        let pending_equities: std::collections::HashSet<String> = self
            .pending_kraken_fetches
            .iter()
            .filter_map(|key| key.strip_prefix("equity:").map(str::to_string))
            .collect();
        let mut cursor = self.kraken_equities_sync_cursor;
        let candidates = select_alpaca_sync_workset_rotating(
            &symbols,
            &timeframes,
            &self.cached_kraken_equities_sync_state,
            &focus_symbols,
            no_data_keys,
            &empty_backfill,
            &pending_equities,
            available_slots,
            foreground_slots,
            if self.user_interacting && !full_tilt {
                24
            } else if full_tilt {
                KRAKEN_EQUITIES_FULL_TILT_BACKGROUND_SCAN_LIMIT
            } else {
                96
            },
            &mut cursor,
            chrono::Utc::now().timestamp(),
            kraken_equities_sync_target_bars,
        );
        self.kraken_equities_sync_cursor = cursor;
        let mut dispatched = 0usize;
        for candidate in candidates {
            if self.queue_kraken_equity_fetch(&candidate.symbol, &candidate.timeframe) {
                dispatched += 1;
            }
        }
        dispatched
    }

    pub(super) fn schedule_kraken_futures_universe_sectors(&mut self) -> usize {
        if !self.kraken_enabled || !self.kraken_scrape_futures {
            return 0;
        }
        let sectors = self.kraken_futures_sync_symbol_sectors();
        let budgets = [10usize, 8, 8, 4];
        let mut dispatched = 0usize;
        for (idx, (sector, budget)) in sectors.iter().zip(budgets).enumerate() {
            dispatched += self.schedule_kraken_futures_pairs_with_budget(idx, sector, budget, 3);
        }
        dispatched
    }

    pub(super) fn tastytrade_sync_symbols(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        let mut tasty_available: std::collections::HashSet<String> =
            self.cached_tastytrade_symbols.clone();
        tasty_available.extend(
            self.tastytrade_universe_symbols
                .iter()
                .map(|symbol| normalize_market_data_symbol(symbol).replace('/', ""))
                .filter(|symbol| !symbol.is_empty()),
        );
        tasty_available.extend(
            self.tt_positions
                .iter()
                .map(|pos| normalize_market_data_symbol(&pos.symbol).replace('/', ""))
                .filter(|symbol| !symbol.is_empty()),
        );
        for symbol in &self.tastytrade_universe_symbols {
            let symbol = normalize_market_data_symbol(symbol);
            if !symbol.is_empty() && seen.insert(symbol.clone()) {
                out.push(symbol);
            }
        }
        for pos in &self.tt_positions {
            let symbol = normalize_market_data_symbol(&pos.symbol);
            if !symbol.is_empty() && seen.insert(symbol.clone()) {
                out.push(symbol);
            }
        }
        for chart in &self.charts {
            let symbol = normalize_market_data_symbol(&chart.symbol);
            let bare = symbol.replace('/', "");
            if !symbol.is_empty() && tasty_available.contains(&bare) && seen.insert(symbol.clone())
            {
                out.push(symbol);
            }
        }
        for symbol in &self.user_watchlist {
            let symbol = normalize_market_data_symbol(symbol);
            let bare = symbol.replace('/', "");
            if !symbol.is_empty() && tasty_available.contains(&bare) && seen.insert(symbol.clone())
            {
                out.push(symbol);
            }
        }
        out.sort();
        out
    }

    pub(super) fn tastytrade_has_symbol(&self, symbol: &str) -> bool {
        let target = normalize_market_data_symbol(symbol)
            .replace('/', "")
            .to_ascii_uppercase();
        if target.is_empty() {
            return false;
        }
        self.cached_tastytrade_symbols.contains(&target)
            || self.tastytrade_universe_symbols.iter().any(|candidate| {
                normalize_market_data_symbol(candidate)
                    .replace('/', "")
                    .eq_ignore_ascii_case(&target)
            })
            || self.tt_positions.iter().any(|pos| {
                normalize_market_data_symbol(&pos.symbol)
                    .replace('/', "")
                    .eq_ignore_ascii_case(&target)
            })
    }

    pub(super) fn alpaca_focus_symbols(&self) -> std::collections::HashSet<String> {
        self.cached_active_symbols
            .iter()
            .map(|sym| normalize_market_data_symbol(sym).replace('/', ""))
            .filter(|sym| !sym.is_empty())
            .filter(|sym| !self.cached_mt5_symbols.contains(sym))
            .filter(|sym| !self.cached_tastytrade_symbols.contains(sym))
            .collect()
    }

    pub(super) fn alpaca_bar_backlog_active(&self) -> bool {
        !self.pending_alpaca_fetches.is_empty() || !self.alpaca_retry_queue.is_empty()
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
        if self.user_interacting && !self.full_tilt_sync_enabled() {
            // Keep chart pan/zoom responsive during large historical syncs.  Do not
            // stop syncing entirely; just narrow new queue pressure while existing
            // in-flight requests drain naturally. Full-tilt AC mode intentionally
            // keeps pressure up when the user asked to spend the hardware.
            capacity.fetch_permits = capacity.fetch_permits.min(2);
            capacity.queue_window = capacity.queue_window.min(4);
            capacity.batch_size = capacity.batch_size.min(2);
            capacity.foreground_reserve = capacity.foreground_reserve.min(1);
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

    pub(super) fn queue_alpaca_fetch(&mut self, symbol: &str, timeframe: &str) -> bool {
        let Some(tf) = normalize_sync_timeframe_key(timeframe) else {
            return false;
        };
        if !self.alpaca_enabled || !self.sync_timeframe_enabled(tf) {
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

    pub(super) fn queue_kraken_fetch(&mut self, symbol: &str, timeframe: &str) -> bool {
        let Some(tf) = normalize_sync_timeframe_key(timeframe) else {
            return false;
        };
        if !self.kraken_enabled || !self.sync_timeframe_enabled(tf) {
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

    pub(super) fn queue_kraken_equity_fetch(&mut self, symbol: &str, timeframe: &str) -> bool {
        let Some(tf) = normalize_sync_timeframe_key(timeframe) else {
            return false;
        };
        if !self.kraken_enabled || !self.sync_timeframe_enabled(tf) {
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
            self.log.push_back(LogEntry::info(format!(
                "Kraken equities sync queued {} {}",
                symbol, tf
            )));
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
        if !self.sync_timeframe_enabled(tf) {
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

    pub(super) fn resolve_tastytrade_symbol(&self, symbol: &str) -> String {
        let target = normalize_market_data_symbol(symbol).replace('/', "");
        for candidate in self
            .tastytrade_universe_symbols
            .iter()
            .map(String::as_str)
            .chain(self.user_watchlist.iter().map(String::as_str))
            .chain(self.charts.iter().map(|chart| chart.symbol.as_str()))
            .chain(self.tt_positions.iter().map(|pos| pos.symbol.as_str()))
        {
            let normalized = normalize_market_data_symbol(candidate).replace('/', "");
            if !normalized.is_empty() && normalized.eq_ignore_ascii_case(&target) {
                return candidate.to_string();
            }
        }
        symbol.to_string()
    }

    pub(super) fn queue_tastytrade_fetch(&mut self, symbol: &str, timeframe: &str) -> bool {
        let Some(tf) = normalize_sync_timeframe_key(timeframe) else {
            return false;
        };
        if !self.tastytrade_enabled || !self.sync_timeframe_enabled(tf) {
            return false;
        }
        let symbol = normalize_market_data_symbol(symbol);
        if symbol.is_empty() {
            return false;
        }
        if self.is_unresolvable_fetch_key("tastytrade", &symbol, tf) {
            return false;
        }
        if self.is_fetch_on_cooldown("tastytrade", &symbol, tf) {
            return false;
        }
        if !self.tastytrade_backfill_complete_loaded {
            self.tastytrade_backfill_complete_load();
        }
        let fetch_key = alpaca_fetch_key(&symbol.replace('/', ""), tf);
        let backfill_complete = self
            .tastytrade_backfill_complete_pairs
            .contains_key(&fetch_key);
        if !self.pending_tastytrade_fetches.insert(fetch_key) {
            return false;
        }
        self.mark_fetch_queued("tastytrade", &symbol, tf);
        let resolved_symbol = self.resolve_tastytrade_symbol(&symbol);
        let _ = self.broker_tx.send(BrokerCmd::TastyTradeFetchBars {
            symbol: resolved_symbol,
            timeframe: tf.to_string(),
            backfill_complete,
        });
        true
    }

    pub(super) fn settle_market_data_fetch(&mut self, source: &str, symbol: &str, timeframe: &str) {
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
    pub(super) fn is_fetch_on_cooldown(
        &self,
        source: &str,
        symbol: &str,
        timeframe: &str,
    ) -> bool {
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
        if !self.alpaca_enabled || !self.broker_connected || symbols.is_empty() {
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

        if self.cached_alpaca_sync_state_rev != Some(self.bg_rev) {
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
        for candidate in candidates {
            if self.queue_alpaca_fetch(&candidate.symbol, &candidate.timeframe) {
                dispatched += 1;
            }
        }
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
        let queue_window = if self.user_interacting && !full_tilt {
            4
        } else if full_tilt {
            KRAKEN_SPOT_FULL_TILT_QUEUE_WINDOW
        } else {
            KRAKEN_SPOT_QUEUE_WINDOW
        };
        let batch_limit = if self.user_interacting && !full_tilt {
            batch_limit.min(2)
        } else if full_tilt {
            batch_limit.max(32)
        } else {
            batch_limit
        };
        let foreground_slots = if self.user_interacting && !full_tilt {
            foreground_slots.min(1)
        } else if full_tilt {
            foreground_slots.max(8)
        } else {
            foreground_slots
        };
        let scan_limit = if self.user_interacting && !full_tilt {
            48
        } else if full_tilt {
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
        if self.cached_kraken_sync_state_rev != Some(self.bg_rev) {
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
        let queue_window = if self.user_interacting && !full_tilt {
            3
        } else if full_tilt {
            KRAKEN_FUTURES_FULL_TILT_QUEUE_WINDOW
        } else {
            KRAKEN_FUTURES_QUEUE_WINDOW
        };
        let batch_limit = if self.user_interacting && !full_tilt {
            batch_limit.min(1)
        } else if full_tilt {
            batch_limit.max(24)
        } else {
            batch_limit
        };
        let foreground_slots = if self.user_interacting && !full_tilt {
            foreground_slots.min(1)
        } else if full_tilt {
            foreground_slots.max(6)
        } else {
            foreground_slots
        };
        let scan_limit = if self.user_interacting && !full_tilt {
            32
        } else if full_tilt {
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
        if self.cached_kraken_futures_sync_state_rev != Some(self.bg_rev) {
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

    pub(super) fn schedule_tastytrade_symbols(&mut self, symbols: &[String]) -> usize {
        if !self.tastytrade_enabled || !self.tt_connected || symbols.is_empty() {
            return 0;
        }
        if chrono::Utc::now().timestamp() < self.tastytrade_sync_pause_until_ts {
            return 0;
        }
        if self.tastytrade_universe_symbols.is_empty()
            && self.tt_positions.is_empty()
            && self.cached_tastytrade_symbols.is_empty()
        {
            return 0;
        }
        let timeframes = self.enabled_standard_sync_timeframes();
        if timeframes.is_empty() {
            return 0;
        }
        let full_tilt = self.full_tilt_sync_enabled();
        let queue_window = if self.user_interacting && !full_tilt {
            3usize
        } else if full_tilt {
            TASTYTRADE_FULL_TILT_QUEUE_WINDOW
        } else {
            8usize
        };
        let batch_limit = if self.user_interacting && !full_tilt {
            1usize
        } else if full_tilt {
            TASTYTRADE_FULL_TILT_BATCH_SIZE
        } else {
            3usize
        };
        let foreground_slots = if full_tilt { 4usize } else { 1usize };
        let available_slots = queue_window
            .saturating_sub(self.pending_tastytrade_fetches.len())
            .min(batch_limit);
        if available_slots == 0 {
            return 0;
        }
        if self.cached_tastytrade_sync_state_rev != Some(self.bg_rev) {
            let previous = self.cached_tastytrade_sync_state.clone();
            let mut rebuilt = self.build_source_cache_state_map("tastytrade:");
            merge_recent_sync_overrides(&mut rebuilt, &previous, chrono::Utc::now().timestamp());
            self.cached_tastytrade_sync_state = rebuilt;
            self.cached_tastytrade_sync_state_rev = Some(self.bg_rev);
        }
        if !self.tastytrade_backfill_complete_loaded {
            self.tastytrade_backfill_complete_load();
        }
        self.ensure_unresolvable_fetch_key_index();
        let focus_symbols = self.market_data_focus_symbols();
        let empty_no_data_keys = std::collections::HashSet::new();
        let no_data_keys = self
            .unresolvable_fetch_keys_by_broker
            .get("tastytrade")
            .unwrap_or(&empty_no_data_keys);
        let now_s = chrono::Utc::now().timestamp();
        let scan_limit = if self.user_interacting && !full_tilt {
            24
        } else if full_tilt {
            TASTYTRADE_FULL_TILT_BACKGROUND_SCAN_LIMIT
        } else {
            TASTYTRADE_BACKGROUND_SCAN_LIMIT
        };
        let mut cursor = self.tastytrade_sync_cursor;
        let candidates = select_alpaca_sync_workset_rotating(
            symbols,
            &timeframes,
            &self.cached_tastytrade_sync_state,
            &focus_symbols,
            no_data_keys,
            &self.tastytrade_backfill_complete_pairs,
            &self.pending_tastytrade_fetches,
            available_slots,
            foreground_slots,
            scan_limit,
            &mut cursor,
            now_s,
            tastytrade_sync_target_bars,
        );
        self.tastytrade_sync_cursor = cursor;
        let mut dispatched = 0usize;
        for candidate in candidates {
            if self.queue_tastytrade_fetch(&candidate.symbol, &candidate.timeframe) {
                dispatched += 1;
            }
        }
        dispatched
    }

    pub(super) fn maybe_request_alpaca_asset_universe(&mut self) {
        if self.alpaca_enabled && !self.all_broker_assets_fetched && self.broker_connected {
            let _ = self.broker_tx.send(BrokerCmd::GetAllAssets);
            self.all_broker_assets_fetched = true;
        }
    }

    pub(super) fn alpaca_equity_rotation_symbols(&self) -> Vec<String> {
        let mt5_covered = &self.cached_mt5_symbols;
        let tasty_covered = &self.cached_tastytrade_symbols;
        let mut equity_set: std::collections::HashSet<String> =
            std::collections::HashSet::with_capacity(self.all_broker_assets.len() + 64);
        let mut equity_syms: Vec<String> = Vec::with_capacity(self.all_broker_assets.len() + 64);

        for (sym, _name, class) in &self.all_broker_assets {
            if class != "us_equity" {
                continue;
            }
            let su = sym.to_uppercase();
            if Self::demand_is_crypto(&su)
                || mt5_covered.contains(&su)
                || tasty_covered.contains(&su)
            {
                continue;
            }
            if equity_set.insert(su.clone()) {
                equity_syms.push(su);
            }
        }
        for chart in &self.charts {
            let bare = bare_symbol_from_key(&chart.symbol).to_uppercase();
            if Self::demand_is_crypto(&bare)
                || mt5_covered.contains(&bare)
                || tasty_covered.contains(&bare)
            {
                continue;
            }
            if equity_set.insert(bare.clone()) {
                equity_syms.push(bare);
            }
        }
        for wl in &self.user_watchlist {
            let wlu = wl.to_uppercase();
            if Self::demand_is_crypto(&wlu)
                || mt5_covered.contains(&wlu)
                || tasty_covered.contains(&wlu)
            {
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
