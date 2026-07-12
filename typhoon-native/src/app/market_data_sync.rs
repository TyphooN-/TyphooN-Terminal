use super::*;
use typhoon_engine::core::fallback_bars::yahoo_chart_supports_timeframe;

const ALPACA_BATCH_FETCH_MAX_SYMBOLS: usize = 50;
const ALPACA_BATCH_FETCH_INTRADAY_SYMBOLS: usize = 16;
const ALPACA_BATCH_FETCH_LOW_TF_SYMBOLS: usize = 8;
pub(super) const BACKGROUND_RETRY_PENDING_FETCH_CAP: usize = 256;

fn full_tilt_low_tf_reserve_slots(
    full_tilt: bool,
    available_slots: usize,
    max_reserve: usize,
) -> usize {
    if !full_tilt || available_slots < 8 || max_reserve == 0 {
        return 0;
    }
    (available_slots / 8)
        .max(1)
        .min(max_reserve)
        .min(available_slots / 2)
}

/// Memory pressure levels for broad background market-data sync.
///
/// The old fixed 18 GB pause point was too late on 32 GB machines: a cold-start
/// sweep can already have dozens of full-history response bodies + parsed bar
/// vectors + cache-write buffers in flight, so RSS can overshoot from ~18 GB to
/// OOM-kill territory before the next scheduler tick. Use system-RAM percentages
/// instead: keep full pressure while there is headroom, shrink newly queued broad
/// work once RSS crosses ~45%, and pause non-focus broad work at ~55%. Focused
/// chart/MTF/user-demand work still goes through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MarketDataMemoryPressure {
    Normal,
    Reduced,
    PauseBackground,
}

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

pub(super) fn current_system_memory_mb() -> (u64, u64) {
    let mut total_mb = 0;
    let mut available_mb = 0;
    if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                if let Some(kb_str) = line.split_whitespace().nth(1) {
                    if let Ok(kb) = kb_str.parse::<u64>() {
                        total_mb = kb / 1024;
                    }
                }
            } else if line.starts_with("MemAvailable:") {
                if let Some(kb_str) = line.split_whitespace().nth(1) {
                    if let Ok(kb) = kb_str.parse::<u64>() {
                        available_mb = kb / 1024;
                    }
                }
            }
        }
    }
    (total_mb, available_mb)
}

fn low_memory_sync_budget_percent(total_mb: u64) -> usize {
    match total_mb {
        0 => 100,
        // Future users may have 16–32 GB machines. Full-tilt queue windows were
        // tuned on a workstation with enough headroom for many concurrent HTTP
        // response bodies + JSON vectors + zstd/SQLite write buffers. On smaller
        // boxes the same pending windows can overshoot the pressure gate between
        // scheduler ticks and get OOM-killed, so scale newly queued broad work by
        // installed RAM instead of waiting for RSS to spike.
        mb if mb <= 24_576 => 35,
        mb if mb <= 40_960 => 50,
        mb if mb <= 65_536 => 75,
        _ => 100,
    }
}

fn memory_scaled_sync_budget(value: usize, total_mb: u64, floor: usize) -> usize {
    let pct = low_memory_sync_budget_percent(total_mb);
    if pct >= 100 || value <= floor {
        return value.max(floor);
    }
    value
        .saturating_mul(pct)
        .div_ceil(100)
        .max(floor)
        .min(value)
}

fn market_data_memory_pressure_at(
    rss_mb: u64,
    total_mb: u64,
    available_mb: u64,
) -> MarketDataMemoryPressure {
    if rss_mb == 0 {
        return MarketDataMemoryPressure::Normal;
    }
    // Fallback keeps older/no-/proc environments bounded while avoiding
    // accidental throttling on tiny test values.
    if total_mb == 0 {
        return if rss_mb >= 16_000 {
            MarketDataMemoryPressure::PauseBackground
        } else if rss_mb >= 12_000 {
            MarketDataMemoryPressure::Reduced
        } else {
            MarketDataMemoryPressure::Normal
        };
    }
    // Start backing off before the old 14-18GB band. The screenshots show
    // ~12.7GB RSS with 300+ pending fetches shortly before a jump to 23GB anon
    // RSS; waiting until 55% RAM leaves no room for response/parse/write bursts.
    let reduced_rss = (total_mb.saturating_mul(38) / 100).max(8_000);
    let pause_rss = (total_mb.saturating_mul(48) / 100).max(reduced_rss + 1);
    let reduced_available = total_mb.saturating_mul(45) / 100;
    let pause_available = total_mb.saturating_mul(33) / 100;
    if rss_mb >= pause_rss || (available_mb > 0 && available_mb <= pause_available) {
        MarketDataMemoryPressure::PauseBackground
    } else if rss_mb >= reduced_rss || (available_mb > 0 && available_mb <= reduced_available) {
        MarketDataMemoryPressure::Reduced
    } else {
        MarketDataMemoryPressure::Normal
    }
}

fn current_market_data_memory_pressure() -> MarketDataMemoryPressure {
    let (total_mb, available_mb) = current_system_memory_mb();
    market_data_memory_pressure_at(current_process_rss_mb(), total_mb, available_mb)
}

pub(super) fn background_retry_dispatch_allowed(pending_fetches: usize) -> bool {
    pending_fetches < BACKGROUND_RETRY_PENDING_FETCH_CAP
        && current_market_data_memory_pressure() != MarketDataMemoryPressure::PauseBackground
}

fn background_market_data_fetch_allowed(focus: bool, pending_fetches: usize) -> bool {
    if focus {
        return true;
    }
    if pending_fetches >= BACKGROUND_RETRY_PENDING_FETCH_CAP {
        return false;
    }
    current_market_data_memory_pressure() != MarketDataMemoryPressure::PauseBackground
}

fn alpaca_background_sync_paused_until(pause_until_ts: i64, now_s: i64) -> bool {
    pause_until_ts > now_s
}

fn memory_bounded_available_slots(
    queue_window: usize,
    pending: usize,
    batch_limit: usize,
    foreground_reserve: usize,
) -> usize {
    let (total_mb, _) = current_system_memory_mb();
    let queue_floor = foreground_reserve.max(8).min(queue_window.max(1));
    let batch_floor = foreground_reserve.max(4).min(batch_limit.max(1));
    let queue_window = memory_scaled_sync_budget(queue_window, total_mb, queue_floor);
    let batch_limit = memory_scaled_sync_budget(batch_limit, total_mb, batch_floor);
    let available = queue_window.saturating_sub(pending).min(batch_limit);
    match current_market_data_memory_pressure() {
        MarketDataMemoryPressure::Normal => available,
        // Let broad sync keep moving, but stop feeding hundreds of new full-history
        // tuples into memory while a previous burst is still being decompressed,
        // parsed, serialized, and compressed.
        MarketDataMemoryPressure::Reduced => available.min(foreground_reserve.max(8)),
        // Selector gets just the foreground reserve; candidates are also filtered
        // to focus symbols before dispatch below.
        MarketDataMemoryPressure::PauseBackground => available.min(foreground_reserve.max(1)),
    }
}

fn drop_background_candidates_when_paused(candidates: &mut Vec<AlpacaSyncCandidate>) {
    if current_market_data_memory_pressure() == MarketDataMemoryPressure::PauseBackground {
        candidates.retain(|candidate| candidate.focus);
    }
}

/// Canonical `"<source>:"` cache-key prefix for a source segment, or `None` for
/// keys the sync scheduler doesn't track (merged/default/legacy variants).
fn sync_state_source_prefix_for_segment(seg: &str) -> Option<&'static str> {
    match seg {
        "alpaca" => Some("alpaca:"),
        "kraken" => Some("kraken:"),
        "kraken-futures" => Some("kraken-futures:"),
        "kraken-equities" => Some("kraken-equities:"),
        "yahoo-chart" => Some("yahoo-chart:"),
        _ => None,
    }
}

/// True for the sub-daily equity timeframes that only print during a live
/// session. Daily and higher settle once at the close, so they remain worth
/// pulling while the market is shut; 1Min is excluded because the Alpaca low-TF
/// assist never queues it. Used by the Lever 1 market-closed fetch gate.
pub(super) fn is_intraday_equity_sync_tf(tf: &str) -> bool {
    matches!(tf, "5Min" | "15Min" | "30Min" | "1Hour" | "4Hour")
}

/// The regular US-equities market is idle — fully CLOSED (weekends, holidays, the
/// overnight gap before a non-trading day) or in the overnight (Blue Ocean)
/// session — so no regular session is printing bars for the broad universe. The
/// adaptive re-probe backoff only applies while idle; during a live regular
/// session (OPEN / PRE-MARKET / AFTER-HOURS) every cell re-probes at the fast
/// base cadence so it resyncs immediately at the open. An empty/unknown clock
/// reads not-idle (fail open). State keywords are mutually exclusive on the phrase
/// before `·` ("CLOSED" vs "OVERNIGHT"), so a substring test is unambiguous.
fn market_status_is_idle(status: &str) -> bool {
    status.contains("CLOSED") || status.contains("OVERNIGHT")
}

/// Longest a caught-up cell waits between idle re-probes, in whole timeframe
/// periods. 1 = the bar-formation rate: a completed bar for a timeframe closes
/// exactly once per period, so re-probing faster provably can't surface a new
/// bar — this is the most aggressive re-check cadence that isn't pure waste. The
/// gate is scoped to the bounded demand/watchlist/chart set (not the ~11k
/// rotation universe), so probing it every period during idle is cheap. Raise it
/// to trade freshness for idle RPM (2 ⇒ at most every two periods, …).
const REFETCH_BACKOFF_MAX_PERIODS: i64 = 1;
/// Shift-overflow guard for the doubling below. The per-period ceiling already
/// binds long before this, so it only keeps `1 << streak` well-defined once a
/// cell has settled empty many times over a long idle stretch.
const REFETCH_BACKOFF_SHIFT_CAP: u32 = 20;

/// Adaptive re-probe backoff window (seconds) for a background intraday cell that
/// has settled with no new bars `streak` times in a row. `streak` 0 is the base
/// ~half-period cadence (identical to [`Self::is_fetch_on_cooldown`]) so an
/// out-of-sync cell that keeps landing bars stays fast; each further empty settle
/// doubles the window, but it never exceeds [`REFETCH_BACKOFF_MAX_PERIODS`] full
/// timeframe periods. A caught-up cell can't gain a bar faster than one per
/// period, so that cap is the theoretical limit of useful re-probing — and it
/// scales with the timeframe (5Min caps at ~5min, 4Hour at ~4h) rather than a
/// fixed wall-clock ceiling.
fn refetch_backoff_secs(period_s: i64, streak: u32) -> i64 {
    let base = (period_s / 2).max(30);
    let ceil = period_s
        .saturating_mul(REFETCH_BACKOFF_MAX_PERIODS)
        .max(base);
    base.saturating_mul(1i64 << streak.min(REFETCH_BACKOFF_SHIFT_CAP))
        .min(ceil)
}

/// Build the per-source `(symbol, timeframe) -> SyncCacheState` maps the sync
/// scheduler consumes, in a SINGLE pass over `detailed_stats`. Each lane used to
/// rescan the whole catalog on the render thread (`build_source_cache_state_map`,
/// up to five full scans per sync tick — the recurring ~130ms `pre_broker`
/// hitch); the BG worker now does the one scan off the render thread and ships
/// the small result maps in `BgData::source_sync_state`. Parsing matches the old
/// per-prefix scan exactly (skip `__`-meta keys, `SYM:TF` only, newest write
/// wins per pair).
pub(super) fn build_source_sync_state_maps(
    detailed_stats: &[(String, i64, i64)],
    bar_ts_cache: &std::collections::HashMap<String, (i64, i64, i64)>,
) -> std::collections::HashMap<
    &'static str,
    std::collections::HashMap<(String, String), SyncCacheState>,
> {
    let mut maps: std::collections::HashMap<
        &'static str,
        std::collections::HashMap<(String, String), SyncCacheState>,
    > = std::collections::HashMap::new();
    for (key, bars, ts) in detailed_stats {
        let Some((seg, rest)) = key.split_once(':') else {
            continue;
        };
        let Some(prefix) = sync_state_source_prefix_for_segment(seg) else {
            continue;
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
        let last_bar_ts_s = bar_ts_cache
            .get(key)
            .map(|(_, last_ms, _)| last_ms.div_euclid(1000))
            .unwrap_or(0);
        let entry = maps
            .entry(prefix)
            .or_default()
            .entry((sym, tf))
            .or_default();
        if *ts > entry.write_ts_s {
            *entry = SyncCacheState {
                last_bar_ts_s,
                write_ts_s: *ts,
                bar_count: *bars,
            };
        }
    }
    maps
}

pub(super) fn normalize_kraken_equity_symbol_list<'a, I>(symbols: I) -> Vec<String>
where
    I: IntoIterator<Item = &'a String>,
{
    typhoon_chart_ui::cache_keys::normalize_kraken_equity_symbol_list(symbols)
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
        // Read the BG-worker-precomputed map (built once, off the render thread,
        // in `build_source_sync_state_maps`) instead of rescanning the whole
        // catalog here. Empty until the first BG snapshot arrives — cold-cache
        // semantics, i.e. everything reads as Missing → coverage-first sync.
        self.bg
            .source_sync_state
            .get(prefix)
            .cloned()
            .unwrap_or_default()
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
        // The cached_active_symbols_set is precisely the normalized bare upper no-/ form
        // (same logic as normalize_market_data_symbol + replace /). Use it directly for O(1)
        // path when populated (avoids per-call map/collect in schedulers and workset builders).
        if !self.cached_active_symbols_set.is_empty() {
            return self.cached_active_symbols_set.clone();
        }
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

    /// True when a Kraken spot pair is excluded by the global crypto/fiat quote
    /// filters: its quote is a fiat/stable quote that is currently disabled and its
    /// base is not itself a fiat currency (pure fiat-FX pairs like EUR/USD are always
    /// kept). The single source of truth shared by the WS OHLC firehose and the
    /// disabled-quote cache prune so the two can't drift.
    pub(super) fn kraken_pair_quote_disabled(&self, symbol: &str) -> bool {
        const FIAT_QUOTES: [&str; 10] = [
            "USD", "USDT", "USDC", "USDG", "EUR", "GBP", "CAD", "AUD", "JPY", "CHF",
        ];
        const FIAT_BASES: [&str; 7] = ["USD", "EUR", "GBP", "CAD", "AUD", "JPY", "CHF"];
        let symbol = typhoon_engine::core::kraken::normalize_pair_symbol(symbol);
        let Some(quote) = Self::kraken_symbol_quote(&symbol) else {
            return false;
        };
        if !FIAT_QUOTES.contains(&quote) || self.crypto_fiat_quote_scrape_enabled(quote) {
            return false;
        }
        let base = symbol
            .strip_suffix(quote)
            .unwrap_or(symbol.as_str())
            .trim_end_matches('/');
        !FIAT_BASES.contains(&base)
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
        // Use cached value if populated and key matches current active inputs.
        // Cache is populated centrally in pre-broker cache block (O(1) hit after change).
        let active_key = self.active_symbols_cache_key();
        if self.cached_kraken_sync_sectors_key == Some(active_key) && !self.cached_kraken_sync_sectors.is_empty() {
            return self.cached_kraken_sync_sectors.clone();
        }
        // Fallback / first build (rare after init).
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

    /// Memoized [`Self::kraken_equity_catalog_symbols`]. Normalizing+sorting the
    /// ~12k xStock universe is pure and was re-run on the render thread every 60s
    /// scheduler tick; cache it and rebuild only when the universe list length
    /// changes (it is replaced wholesale on reload). Used by the hot scheduler path.
    pub(super) fn kraken_equity_catalog_symbols_cached(&mut self) -> Vec<String> {
        let sig = self.kraken_equity_universe_symbols.len();
        if self.cached_kraken_equity_catalog_sig != Some(sig) {
            let rebuilt = self.kraken_equity_catalog_symbols();
            self.cached_kraken_equity_catalog = rebuilt;
            self.cached_kraken_equity_catalog_sig = Some(sig);
        }
        self.cached_kraken_equity_catalog.clone()
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
        if !self.broad_sync_state_ready() {
            return 0;
        }
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

    /// True once the BG worker's first detailed-stats pass has landed in
    /// `self.bg`. Before that, every (symbol, timeframe) reads as Missing, so a
    /// broad-universe scheduler tick would re-dispatch full-history fetches for
    /// the ENTIRE catalog — the observed session-start storm (6.8k Alpaca +
    /// 7.8k Yahoo full 1Month pulls inside one hour). Broad lanes wait for the
    /// snapshot; focus/chart-demand paths are deliberately not gated.
    pub(super) fn broad_sync_state_ready(&self) -> bool {
        self.bg.sync_state_ready
    }

    pub(super) fn schedule_kraken_equities_universe(&mut self) -> usize {
        if !self.broad_sync_state_ready() {
            return 0;
        }
        let catalog_symbols = self.kraken_equity_catalog_symbols_cached();
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
        if self.cached_kraken_equities_sync_state_rev != Some(self.bg_rev) {
            let previous = self.cached_kraken_equities_sync_state.clone();
            let mut rebuilt = self.build_source_cache_state_map("kraken-equities:");
            merge_recent_sync_overrides(&mut rebuilt, &previous, chrono::Utc::now().timestamp());
            self.cached_kraken_equities_sync_state = rebuilt;
            self.cached_kraken_equities_sync_state_rev = Some(self.bg_rev);
        }
        self.ensure_unresolvable_fetch_key_index();
        let focus_symbols = self.market_data_focus_symbols();

        // Kraken high-TF (1Day/1Week/1Month) backfill aggressiveness fix:
        // Always treat these rows as high-priority when the symbol is focused
        // (open chart or MTF grid). This prevents the "stale forever" state
        // the user reported even for actively used Kraken symbols.

        let empty_no_data_keys = std::collections::HashSet::new();
        let empty_backfill = std::collections::HashMap::new();
        let mut dispatched = 0usize;
        let now_s = chrono::Utc::now().timestamp();

        let native_available_slots = memory_bounded_available_slots(
            queue_window,
            self.pending_kraken_fetches
                .iter()
                .filter(|key| key.starts_with("equity:"))
                .count(),
            batch_limit,
            foreground_slots,
        );
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
            // Mirrors queue_kraken_equity_fetch's symbol normalization so the
            // cooldown probe hits the same key mark_fetch_queued recorded.
            let equities_dispatch_blocked = |symbol: &str, tf: &str| {
                let symbol = normalize_market_data_symbol(symbol)
                    .replace('/', "")
                    .trim_end_matches(".EQ")
                    .to_ascii_uppercase();
                self.is_fetch_on_cooldown("kraken-equities", &symbol, tf)
            };
            // Tier priority (MTF Grid > Active > Background) + high-TF-first is applied inside the workset selector
            let mut candidates = select_alpaca_sync_workset_rotating_with_stale_multiplier(
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
                &equities_dispatch_blocked,
            );
            drop_background_candidates_when_paused(&mut candidates);
            self.kraken_equities_sync_cursor = cursor;
            for candidate in candidates {
                if self.queue_kraken_equity_fetch(&candidate.symbol, &candidate.timeframe) {
                    dispatched += 1;
                }
            }
        }

        if self.backfill_alpaca_kraken_equities_enabled
            && !fallback_timeframes.is_empty()
            && !alpaca_background_sync_paused_until(self.alpaca_sync_pause_until_ts, now_s)
        {
            let alpaca_timeframes: Vec<String> = fallback_timeframes
                .iter()
                .filter(|tf| alpaca_sync_target_bars(tf).is_some())
                .cloned()
                .collect();
            // M1/M5 broad equity sync is Kraken-native only. Alpaca assist
            // remains disabled there so it does not burn historical RPM on
            // rows the merged/chart path deliberately ignores.
            if !alpaca_timeframes.is_empty() {
                if self.cached_alpaca_sync_state_rev != Some(self.bg_rev) {
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
                let available_slots = memory_bounded_available_slots(
                    capacity.queue_window,
                    self.pending_alpaca_fetches.len(),
                    capacity.batch_size,
                    capacity.foreground_reserve,
                );
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
                    // Mirrors queue_alpaca_fetch's symbol normalization.
                    let alpaca_dispatch_blocked = |symbol: &str, tf: &str| {
                        let symbol = normalize_market_data_symbol(symbol).replace('/', "");
                        self.is_fetch_on_cooldown("alpaca", &symbol, tf)
                    };
                    let mut candidates = select_alpaca_sync_workset_rotating(
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
                        &alpaca_dispatch_blocked,
                    );
                    drop_background_candidates_when_paused(&mut candidates);
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
                let available_slots = memory_bounded_available_slots(
                    yahoo_queue_window,
                    self.pending_yahoo_chart_fetches.len(),
                    yahoo_batch_limit,
                    yahoo_foreground_slots,
                );
                if available_slots > 0 {
                    if self.cached_yahoo_chart_sync_state_rev != Some(self.bg_rev) {
                        let previous = self.cached_yahoo_chart_sync_state.clone();
                        let mut rebuilt = self.build_source_cache_state_map("yahoo-chart:");
                        merge_recent_sync_overrides(&mut rebuilt, &previous, now_s);
                        self.cached_yahoo_chart_sync_state = rebuilt;
                        self.cached_yahoo_chart_sync_state_rev = Some(self.bg_rev);
                    }
                    if !self.yahoo_chart_backfill_complete_loaded {
                        self.yahoo_chart_backfill_complete_load();
                    }
                    let no_data = self
                        .unresolvable_fetch_keys_by_broker
                        .get("yahoo-chart")
                        .cloned()
                        .unwrap_or_default();
                    let mut cursor = self.yahoo_chart_sync_cursor;
                    // Mirrors queue_yahoo_chart_fetch's symbol normalization.
                    let yahoo_dispatch_blocked = |symbol: &str, tf: &str| {
                        let symbol = normalize_market_data_symbol(symbol)
                            .replace('/', "")
                            .trim_end_matches(".EQ")
                            .to_ascii_uppercase();
                        self.is_fetch_on_cooldown("yahoo-chart", &symbol, tf)
                    };
                    // Every Yahoo chart fetch pulls full period1=0 history, so a
                    // pair marked complete never needs Backfill re-selection —
                    // without this map the u32::MAX target made every (symbol,
                    // 1Month) an eternal Backfill candidate and Yahoo re-pulled
                    // the whole catalog's monthly history forever, starving
                    // 1Week/1Day (observed 8.3k 1Month rewrites in one night).
                    let mut candidates = select_alpaca_sync_workset_rotating(
                        &fallback_symbols,
                        &yahoo_timeframes,
                        &self.cached_yahoo_chart_sync_state,
                        &focus_symbols,
                        &no_data,
                        &self.yahoo_chart_backfill_complete_pairs,
                        &self.pending_yahoo_chart_fetches,
                        available_slots,
                        yahoo_foreground_slots,
                        yahoo_scan_limit,
                        &mut cursor,
                        now_s,
                        alpaca_sync_target_bars,
                        &yahoo_dispatch_blocked,
                    );
                    drop_background_candidates_when_paused(&mut candidates);
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
        if !self.broad_sync_state_ready() {
            return 0;
        }
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

    /// Aggregate Alpaca historical budget across the account pool (ADR-130):
    /// the per-account RPM (hint/observed) times the number of accounts in the
    /// data-sync rotation, since each account has an independent rate limiter.
    pub(super) fn alpaca_aggregate_historical_rpm(&self) -> u32 {
        self.alpaca_effective_historical_rpm()
            .saturating_mul(self.alpaca_data_account_count() as u32)
    }

    pub(super) fn alpaca_sync_capacity(&self) -> AlpacaSyncCapacity {
        // Tier by the AGGREGATE pool budget so the scheduler feeds enough
        // symbols to keep every account's limiter busy, then scale worker
        // permits with the pool size (bounded — each in-flight worker is still
        // paced by its own account's limiter).
        let accounts = self.alpaca_data_account_count();
        let mut capacity = alpaca_sync_capacity_for_rpm(self.alpaca_aggregate_historical_rpm());
        capacity.fetch_permits = capacity.fetch_permits.saturating_mul(accounts).min(48);
        if self.full_tilt_sync_enabled() {
            capacity.fetch_permits = capacity.fetch_permits.max(ALPACA_FULL_TILT_FETCH_PERMITS);
            capacity.queue_window = capacity.queue_window.max(ALPACA_FULL_TILT_QUEUE_WINDOW);
            capacity.batch_size = capacity.batch_size.max(ALPACA_FULL_TILT_BATCH_SIZE);
            capacity.foreground_reserve = capacity.foreground_reserve.max(8);
        }
        let (total_mb, _) = current_system_memory_mb();
        capacity.fetch_permits = memory_scaled_sync_budget(capacity.fetch_permits, total_mb, 2);
        capacity.queue_window = memory_scaled_sync_budget(capacity.queue_window, total_mb, 32);
        capacity.batch_size = memory_scaled_sync_budget(capacity.batch_size, total_mb, 16);
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

    /// Depth of a full provider-history batch pull, in bars. Chunks at this
    /// depth are the coverage path (Missing/Backfill buckets); the engine also
    /// treats a Complete-outcome symbol omission at this depth as authoritative
    /// "provider has nothing" and tombstones it.
    pub(super) const ALPACA_BATCH_DEEP_HISTORY_BARS: u32 = 10_000;

    /// Bars of lookback a stale top-up chunk actually needs: the widest gap in
    /// the chunk (candidate score = seconds since that symbol's last bar) plus
    /// 50% headroom. Before this, every batch re-pulled the full 10k-bar server
    /// history per symbol, so a routine post-close 1Day refresh of the 12.4k
    /// catalog burned the entire aggregate RPM budget on already-cached bars —
    /// the reason 3 Alpaca lanes performed like one.
    fn alpaca_batch_topup_limit_bars(timeframe: &str, max_age_s: i64) -> u32 {
        let period_s = sync_timeframe_period_secs(timeframe).unwrap_or(60).max(1);
        let gap_bars = (max_age_s.max(0) / period_s).max(1) as u32;
        gap_bars
            .saturating_add((gap_bars / 2).max(8))
            .clamp(64, Self::ALPACA_BATCH_DEEP_HISTORY_BARS)
    }

    fn queue_alpaca_batch_fetches_from_candidates(
        &mut self,
        candidates: Vec<AlpacaSyncCandidate>,
    ) -> usize {
        // Split per (timeframe, deep): Missing/Backfill chunks need full
        // provider history; Stale chunks only need the gap since their oldest
        // cached tail. Value = (symbols, max stale age seen in the group).
        let mut by_tf: std::collections::BTreeMap<(String, bool), (Vec<String>, i64)> =
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
            if Self::alpaca_low_tf_assist_unsupported_for_symbol(
                &self.kraken_equity_universe_set,
                &symbol,
                tf,
            ) {
                continue;
            }
            let fetch_key = alpaca_fetch_key(&symbol, tf);
            if self.is_fetch_on_cooldown("alpaca", &symbol, tf) {
                continue;
            }
            if !self.pending_alpaca_fetches.insert(fetch_key) {
                continue;
            }
            self.mark_fetch_queued("alpaca", &symbol, tf);
            let deep = candidate.bucket != AlpacaSyncBucket::Stale;
            let entry = by_tf.entry((tf.to_string(), deep)).or_default();
            entry.0.push(symbol);
            if !deep {
                // Stale candidates carry score = age of the last cached bar.
                entry.1 = entry.1.max(candidate.score);
            }
            dispatched += 1;
        }
        for ((timeframe, deep), (symbols, max_age_s)) in by_tf {
            let limit = if deep {
                Self::ALPACA_BATCH_DEEP_HISTORY_BARS
            } else {
                Self::alpaca_batch_topup_limit_bars(&timeframe, max_age_s)
            };
            let chunk_symbols = Self::alpaca_batch_fetch_chunk_symbols(&timeframe);
            for chunk in symbols.chunks(chunk_symbols) {
                let _ = self.broker_tx.send(BrokerCmd::AlpacaFetchBarsBatch {
                    symbols: chunk.to_vec(),
                    timeframe: timeframe.clone(),
                    limit,
                });
            }
        }
        dispatched
    }

    /// Adaptive re-probe backoff gate for a background intraday cell. `true` when
    /// the cell is still inside its per-period backoff window — it has repeatedly
    /// settled with no new bars (caught up, nothing printing), so this re-probe is
    /// skipped to save Alpaca RPM. A streak-0 cell reduces to the base cadence, so
    /// out-of-sync cells (which drop their streak the instant a fetch lands bars)
    /// keep fetching whenever possible until they catch up. `symbol`/`timeframe`
    /// are the normalized keys queued into `fetch_last_queued_ts`.
    pub(super) fn bg_intraday_refetch_backed_off(&self, symbol: &str, timeframe: &str) -> bool {
        let cell_key = alpaca_fetch_key(symbol, timeframe);
        let streak = self
            .bg_refetch_empty_streak
            .get(&cell_key)
            .copied()
            .unwrap_or(0);
        if streak == 0 {
            return false;
        }
        let Some(period_s) = sync_timeframe_period_secs(timeframe) else {
            return false;
        };
        let window = refetch_backoff_secs(period_s, streak);
        let Some(last) = self
            .fetch_last_queued_ts
            .get(&format!("alpaca:{cell_key}"))
            .copied()
        else {
            return false;
        };
        chrono::Utc::now().timestamp().saturating_sub(last) < window
    }

    /// Adaptive-backoff bookkeeping, run when an Alpaca background fetch settles
    /// successfully. A fetch that wrote new bars advanced the cell's `write_ts_s`
    /// past its queue time (`note_cached_sync_success` runs on the preceding
    /// `BarsFetched`, which the broker emits only for count > 0), so the streak is
    /// cleared and the cell keeps the fast base cadence. A fetch that wrote nothing
    /// (caught up / idle market) grows the streak so the cell re-probes at most
    /// once per timeframe period instead of on the faster base cadence.
    pub(super) fn note_alpaca_refetch_outcome(&mut self, symbol: &str, timeframe: &str) {
        let Some(tf) = normalize_sync_timeframe_key(timeframe) else {
            return;
        };
        let sym = normalize_market_data_symbol(symbol).replace('/', "");
        if sym.is_empty() {
            return;
        }
        let cell_key = alpaca_fetch_key(&sym, tf);
        let queued = self
            .fetch_last_queued_ts
            .get(&format!("alpaca:{cell_key}"))
            .copied()
            .unwrap_or(0);
        let wrote = self
            .cached_alpaca_sync_state
            .get(&(sym, tf.to_string()))
            .map(|s| s.write_ts_s)
            .unwrap_or(0);
        if queued > 0 && wrote >= queued {
            self.bg_refetch_empty_streak.remove(&cell_key);
        } else {
            let entry = self.bg_refetch_empty_streak.entry(cell_key).or_insert(0);
            *entry = entry.saturating_add(1);
        }
    }

    pub(super) fn queue_alpaca_fetch(&mut self, symbol: &str, timeframe: &str) -> bool {
        let Some(tf) = normalize_sync_timeframe_key(timeframe) else {
            return false;
        };
        if !self.alpaca_enabled || !self.broker_connected || !self.sync_timeframe_enabled(tf) {
            return false;
        }
        let symbol = normalize_market_data_symbol(symbol).replace('/', "");
        if Self::alpaca_low_tf_assist_unsupported_for_symbol(
            &self.kraken_equity_universe_set,
            &symbol,
            tf,
        ) {
            return false;
        }
        if !self.alpaca_no_data_loaded {
            self.alpaca_no_data_load();
        }
        if self
            .alpaca_no_data_pairs
            .contains_key(&alpaca_fetch_key(&symbol, tf))
            || self.is_unresolvable_fetch_key("alpaca", &symbol, tf)
        {
            tracing::debug!(
                "Alpaca {} {}: known no-data symbol — skipping fetch",
                symbol,
                tf
            );
            return false;
        }
        if !self.alpaca_backfill_complete_loaded {
            self.alpaca_backfill_complete_load();
        }
        if self.is_fetch_on_cooldown("alpaca", &symbol, tf) {
            return false;
        }
        if self.cached_alpaca_sync_state_rev != Some(self.bg_rev) {
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
        let norm_sym = normalize_market_data_symbol(&symbol).replace("/", "");
        let focus = self.cached_active_symbols_set.contains(&norm_sym);
        if !focus
            && alpaca_background_sync_paused_until(
                self.alpaca_sync_pause_until_ts,
                chrono::Utc::now().timestamp(),
            )
        {
            return false;
        }
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
        // Adaptive re-probe backoff (replaces the old market-closed hard skip):
        // out-of-sync cells (Missing/behind) fetch whenever possible — the goal is
        // full bar data across every enabled timeframe for research/charting. Only
        // a cell that keeps settling empty in an idle market (caught up, nothing
        // printing) is throttled, and only down to one re-probe per bar period —
        // the fastest rate a new bar could appear. Overnight-active symbols
        // self-exempt by producing bars (their streak resets). Bypassed during
        // live regular sessions for a fast resync at the open; the focused chart,
        // daily+ bars, and cells still backfilling history are never gated here.
        if !focus
            && is_intraday_equity_sync_tf(tf)
            && market_status_is_idle(&self.market_clock_status)
            && self.bg_intraday_refetch_backed_off(&symbol, tf)
        {
            return false;
        }
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

    pub(super) fn alpaca_low_tf_assist_unsupported_for_symbol(
        kraken_equity_universe_set: &std::collections::HashSet<String>,
        symbol: &str,
        tf: &str,
    ) -> bool {
        matches!(tf, "1Min" | "5Min") && kraken_equity_universe_set.contains(symbol)
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
        let chart_or_owned = self.chart_by_bare.contains_key(&symbol)
            || self.kraken_balance_assets_by_display.contains(&symbol);
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
        if !self.broad_sync_state_ready() {
            return 0;
        }
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

        let full_tilt = self.full_tilt_sync_enabled();
        let now_s = chrono::Utc::now().timestamp();
        if alpaca_background_sync_paused_until(self.alpaca_sync_pause_until_ts, now_s) {
            return 0;
        }
        let capacity = self.alpaca_sync_capacity();
        let available_slots = memory_bounded_available_slots(
            capacity.queue_window,
            self.pending_alpaca_fetches.len(),
            capacity.batch_size,
            capacity.foreground_reserve,
        );
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
        let scan_limit = if full_tilt {
            ALPACA_FULL_TILT_BACKGROUND_SCAN_LIMIT
        } else {
            ALPACA_BACKGROUND_SCAN_LIMIT
        };
        // Mirrors queue_alpaca_fetch's symbol normalization.
        let alpaca_dispatch_blocked = |symbol: &str, tf: &str| {
            let symbol = normalize_market_data_symbol(symbol).replace('/', "");
            self.is_fetch_on_cooldown("alpaca", &symbol, tf)
        };
        let low_tf_reserve = full_tilt_low_tf_reserve_slots(
            full_tilt,
            available_slots,
            ALPACA_FULL_TILT_LOW_TF_RESERVE_BATCH,
        );
        let main_slots = available_slots.saturating_sub(low_tf_reserve);
        let mut candidates = select_alpaca_sync_workset_rotating(
            symbols,
            &timeframes,
            &self.cached_alpaca_sync_state,
            &focus_symbols,
            &no_data_keys,
            &self.alpaca_backfill_complete_pairs,
            &self.pending_alpaca_fetches,
            main_slots,
            capacity.foreground_reserve,
            scan_limit,
            &mut cursor,
            now_s,
            alpaca_sync_target_bars,
            &alpaca_dispatch_blocked,
        );
        if low_tf_reserve > 0 {
            let mut staged_pending = self.pending_alpaca_fetches.clone();
            staged_pending.extend(
                candidates
                    .iter()
                    .map(|candidate| alpaca_fetch_key(&candidate.symbol, &candidate.timeframe)),
            );
            candidates.extend(select_low_timeframe_sync_reserve_rotating(
                symbols,
                &timeframes,
                &self.cached_alpaca_sync_state,
                &focus_symbols,
                &no_data_keys,
                &self.alpaca_backfill_complete_pairs,
                &staged_pending,
                low_tf_reserve,
                scan_limit,
                &mut cursor,
                now_s,
                24,
                alpaca_sync_target_bars,
                &alpaca_dispatch_blocked,
            ));
        }
        drop_background_candidates_when_paused(&mut candidates);
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
        let available_slots = memory_bounded_available_slots(
            queue_window,
            self.pending_kraken_fetches.len(),
            batch_limit,
            foreground_slots,
        );
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
        // Mirrors queue_kraken_fetch's symbol normalization.
        let kraken_dispatch_blocked = |symbol: &str, tf: &str| {
            let symbol = typhoon_engine::core::kraken::normalize_pair_symbol(symbol);
            self.is_fetch_on_cooldown("kraken", &symbol, tf)
        };
        let low_tf_reserve = full_tilt_low_tf_reserve_slots(
            full_tilt,
            available_slots,
            KRAKEN_SPOT_FULL_TILT_LOW_TF_RESERVE_BATCH,
        );
        let main_slots = available_slots.saturating_sub(low_tf_reserve);
        let mut candidates = select_alpaca_sync_workset_rotating(
            symbols,
            &timeframes,
            &self.cached_kraken_sync_state,
            &focus_symbols,
            no_data_keys,
            &self.kraken_backfill_complete_pairs,
            &self.pending_kraken_fetches,
            main_slots,
            foreground_slots,
            scan_limit,
            &mut cursor,
            now_s,
            kraken_sync_target_bars,
            &kraken_dispatch_blocked,
        );
        if low_tf_reserve > 0 {
            let mut staged_pending = self.pending_kraken_fetches.clone();
            staged_pending.extend(
                candidates
                    .iter()
                    .map(|candidate| alpaca_fetch_key(&candidate.symbol, &candidate.timeframe)),
            );
            candidates.extend(select_low_timeframe_sync_reserve_rotating(
                symbols,
                &timeframes,
                &self.cached_kraken_sync_state,
                &focus_symbols,
                no_data_keys,
                &self.kraken_backfill_complete_pairs,
                &staged_pending,
                low_tf_reserve,
                scan_limit,
                &mut cursor,
                now_s,
                24,
                kraken_sync_target_bars,
                &kraken_dispatch_blocked,
            ));
        }
        drop_background_candidates_when_paused(&mut candidates);
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
        let available_slots = memory_bounded_available_slots(
            queue_window,
            self.pending_kraken_futures_fetches.len(),
            batch_limit,
            foreground_slots,
        );
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
        // Mirrors queue_kraken_futures_fetch's symbol normalization.
        let futures_dispatch_blocked = |symbol: &str, tf: &str| {
            let symbol = typhoon_engine::core::kraken_futures::normalize_futures_symbol(symbol);
            self.is_fetch_on_cooldown("kraken-futures", &symbol, tf)
        };
        let mut candidates = select_alpaca_sync_workset_rotating(
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
            &futures_dispatch_blocked,
        );
        drop_background_candidates_when_paused(&mut candidates);
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

    /// Memoized [`Self::alpaca_equity_rotation_symbols`]. Uppercasing+dedup+sort over
    /// Alpaca's ~11k us_equity universe is pure and was re-run on the render thread
    /// every 60s rotation tick; cache it and rebuild only when the input lengths
    /// change. The chart/watchlist floor is a backup (those symbols also sync via
    /// their demand paths), so a same-length swap costing one cycle is harmless.
    pub(super) fn alpaca_equity_rotation_symbols_cached(&mut self) -> Vec<String> {
        let sig = (
            self.all_broker_assets.len(),
            self.charts.len(),
            self.user_watchlist.len(),
        );
        if self.cached_alpaca_equity_rotation_sig != Some(sig) {
            let rebuilt = self.alpaca_equity_rotation_symbols();
            self.cached_alpaca_equity_rotation = rebuilt;
            self.cached_alpaca_equity_rotation_sig = Some(sig);
        }
        self.cached_alpaca_equity_rotation.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn market_status_is_idle_only_for_closed_and_overnight() {
        // Idle = no live regular session; adaptive backoff engages only here.
        assert!(market_status_is_idle("US equities CLOSED · opens in 6h"));
        assert!(market_status_is_idle(
            "US equities OVERNIGHT · next pre-market in 5h 44m"
        ));
        // Live regular sessions and an unfetched clock read not-idle (backoff
        // bypassed → fast resync).
        assert!(!market_status_is_idle("US equities OPEN · closes in 5h 0m"));
        assert!(!market_status_is_idle(
            "US equities PRE-MARKET · Core in 55m"
        ));
        assert!(!market_status_is_idle(
            "US equities AFTER-HOURS · closes in 3h 0m"
        ));
        assert!(!market_status_is_idle(""));
    }

    #[test]
    fn refetch_backoff_secs_caps_at_one_timeframe_period() {
        // 15Min: base = 900/2 = 450s. streak 0 keeps the fast catch-up cadence.
        assert_eq!(refetch_backoff_secs(900, 0), 450);
        // Any empty streak caps at exactly one period — the bar-formation rate,
        // the most aggressive re-check that can still surface a new bar.
        assert_eq!(refetch_backoff_secs(900, 1), 900);
        assert_eq!(refetch_backoff_secs(900, 2), 900);
        assert_eq!(refetch_backoff_secs(900, 40), 900);
        // Scales per-timeframe, not a fixed ceiling: 5Min → 5min, 4Hour → 4h.
        assert_eq!(refetch_backoff_secs(300, 5), 300);
        assert_eq!(refetch_backoff_secs(14_400, 5), 14_400);
        // Base floor: even a tiny period never re-probes faster than 30s.
        assert_eq!(refetch_backoff_secs(40, 0), 30);
    }

    #[test]
    fn intraday_equity_sync_tf_excludes_daily_and_higher() {
        for tf in ["5Min", "15Min", "30Min", "1Hour", "4Hour"] {
            assert!(is_intraday_equity_sync_tf(tf), "{tf} should be intraday");
        }
        for tf in ["1Min", "1Day", "1Week", "1Month"] {
            assert!(!is_intraday_equity_sync_tf(tf), "{tf} should not be gated");
        }
    }

    #[test]
    fn build_source_sync_state_maps_buckets_by_source_and_keeps_newest() {
        let detailed = vec![
            ("alpaca:AAPL:1Day".to_string(), 100i64, 1_000i64),
            ("alpaca:AAPL:1Day".to_string(), 250, 2_000), // newer write wins
            ("kraken:ETHUSD:1Hour".to_string(), 50, 1_500),
            ("yahoo-chart:msft:1Week".to_string(), 7, 1_200), // lowercase symbol
            ("kraken-equities:TNDM.EQ:1Day".to_string(), 9, 1_100), // .EQ stripped
            ("kraken-futures:XBTUSD:4Hour".to_string(), 3, 1_050),
            ("merged:AAPL:1Day".to_string(), 999, 9_999), // untracked source
            ("default:AAPL:1Day".to_string(), 999, 9_999), // untracked source
            ("alpaca:__META__:1Day".to_string(), 1, 9_999), // meta key skipped
            ("alpaca:BADKEY".to_string(), 1, 9_999),      // no timeframe → skipped
            ("alpaca:AAPL:1Day:extra".to_string(), 1, 9_999), // extra segment → skipped
        ];
        let bar_ts: std::collections::HashMap<String, (i64, i64, i64)> =
            std::collections::HashMap::from([("alpaca:AAPL:1Day".to_string(), (0, 5_000_000, 0))]);
        let maps = build_source_sync_state_maps(&detailed, &bar_ts);

        let alpaca = &maps["alpaca:"];
        assert_eq!(
            alpaca.len(),
            1,
            "only the valid AAPL:1Day pair; meta/malformed keys skipped"
        );
        let aapl = alpaca[&("AAPL".to_string(), "1Day".to_string())];
        assert_eq!(aapl.bar_count, 250, "newest write_ts wins");
        assert_eq!(aapl.write_ts_s, 2_000);
        assert_eq!(
            aapl.last_bar_ts_s, 5_000,
            "last_bar_ts from bar_ts_cache, ms→s"
        );

        // Every tracked lane buckets under its own prefix; untracked sources don't.
        assert_eq!(maps["kraken:"].len(), 1);
        assert_eq!(maps["yahoo-chart:"].len(), 1);
        assert_eq!(maps["kraken-equities:"].len(), 1);
        assert_eq!(maps["kraken-futures:"].len(), 1);
        assert!(!maps.contains_key("merged:"), "merged is not a sync lane");
        assert!(!maps.contains_key("default:"), "default is not a sync lane");
    }

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
    fn batch_topup_limit_covers_gap_with_headroom_and_stays_bounded() {
        // 3-day gap on 1Day bars: gap 3 + headroom 8 = 11, floored at 64 so a
        // batch is never pointlessly narrow.
        assert_eq!(
            TyphooNApp::alpaca_batch_topup_limit_bars("1Day", 3 * 86_400),
            64
        );
        // 200-day gap: 200 + 100 headroom = 300 — a bounded window instead of
        // the old full 10k-bar history re-pull.
        assert_eq!(
            TyphooNApp::alpaca_batch_topup_limit_bars("1Day", 200 * 86_400),
            300
        );
        // Pathological ages clamp at the deep-history ceiling.
        assert_eq!(
            TyphooNApp::alpaca_batch_topup_limit_bars("15Min", 400 * 86_400),
            TyphooNApp::ALPACA_BATCH_DEEP_HISTORY_BARS
        );
    }

    #[test]
    fn memory_pressure_uses_system_relative_rss_and_available_thresholds() {
        assert_eq!(
            market_data_memory_pressure_at(11_500, 32_000, 20_000),
            MarketDataMemoryPressure::Normal
        );
        assert_eq!(
            market_data_memory_pressure_at(12_500, 32_000, 20_000),
            MarketDataMemoryPressure::Reduced
        );
        assert_eq!(
            market_data_memory_pressure_at(15_500, 32_000, 20_000),
            MarketDataMemoryPressure::PauseBackground
        );
        assert_eq!(
            market_data_memory_pressure_at(8_000, 32_000, 10_000),
            MarketDataMemoryPressure::PauseBackground
        );
        assert_eq!(
            market_data_memory_pressure_at(0, 32_000, 20_000),
            MarketDataMemoryPressure::Normal
        );
    }

    #[test]
    fn memory_pressure_fallback_without_meminfo_is_still_bounded() {
        assert_eq!(
            market_data_memory_pressure_at(11_999, 0, 0),
            MarketDataMemoryPressure::Normal
        );
        assert_eq!(
            market_data_memory_pressure_at(12_000, 0, 0),
            MarketDataMemoryPressure::Reduced
        );
        assert_eq!(
            market_data_memory_pressure_at(16_000, 0, 0),
            MarketDataMemoryPressure::PauseBackground
        );
    }

    #[test]
    fn low_memory_sync_budget_scales_full_tilt_work_before_pressure_spikes() {
        assert_eq!(low_memory_sync_budget_percent(16_384), 35);
        assert_eq!(low_memory_sync_budget_percent(32_000), 50);
        assert_eq!(low_memory_sync_budget_percent(49_152), 75);
        assert_eq!(low_memory_sync_budget_percent(98_304), 100);

        assert_eq!(memory_scaled_sync_budget(256, 32_000, 32), 128);
        assert_eq!(memory_scaled_sync_budget(24, 32_000, 6), 12);
        assert_eq!(memory_scaled_sync_budget(6, 16_384, 6), 6);
        assert_eq!(memory_scaled_sync_budget(256, 98_304, 32), 256);
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
    fn alpaca_background_sync_pause_is_time_bounded() {
        assert!(!alpaca_background_sync_paused_until(999, 1000));
        assert!(!alpaca_background_sync_paused_until(1000, 1000));
        assert!(alpaca_background_sync_paused_until(1001, 1000));
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
