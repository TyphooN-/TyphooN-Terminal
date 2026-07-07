use super::*;

const EMPTY_CHART_RELOAD_RETRY_AFTER: std::time::Duration = std::time::Duration::from_secs(30);

/// Max concurrent background deferred-chart loaders. Each holds a full bar set
/// while it merges, so this bounds cold-start RSS and read-connection lock churn.
const DEFERRED_CHART_MAX_INFLIGHT: usize = 3;
/// Cheap in-memory restores finalized per frame. Each restore clones bars + runs
/// GPU indicators (a few ms); capping keeps a burst of simultaneously-ready cells
/// from spiking one frame.
const DEFERRED_CHART_FINALIZE_PER_FRAME: usize = 4;
/// Queue entries examined per frame. Bounds the scheduler's own per-frame cost.
const DEFERRED_CHART_SCAN_WINDOW: usize = 16;
/// In-flight marker is evicted after this long so a hung/deadlocked worker (whose
/// completion never arrives) can't strand a cell as permanently "loading". Far
/// longer than any plausible single load, so it never races a healthy worker.
const DEFERRED_CHART_INFLIGHT_STALE_AFTER: std::time::Duration = std::time::Duration::from_secs(45);

fn deferred_chart_load_key(chart: &ChartState) -> String {
    format!(
        "{}:{}:{}",
        chart.symbol,
        chart.timeframe.cache_suffix(),
        chart.source_override
    )
}

fn empty_chart_load_retry_due(
    last_attempt: Option<std::time::Instant>,
    now: std::time::Instant,
) -> bool {
    last_attempt
        .map(|last| now.duration_since(last) >= EMPTY_CHART_RELOAD_RETRY_AFTER)
        .unwrap_or(true)
}

fn parse_order_qty(value: &str) -> f64 {
    value.trim().parse::<f64>().unwrap_or(0.0).max(0.0)
}

fn alpaca_order_is_working(status: &str) -> bool {
    !matches!(
        status.to_ascii_lowercase().as_str(),
        "filled" | "canceled" | "cancelled" | "expired" | "rejected"
    )
}

fn push_or_merge_order_line(out: &mut Vec<OrderLine>, line: OrderLine) {
    // O(1) dedup/merge using temp map keyed by (is_buy, source, price rounded)
    use std::collections::HashMap;
    let key = (
        line.is_buy,
        line.source.clone(),
        (line.price * 1_000_000_000.0).round() as i64,
    );
    let map: HashMap<_, usize> = out
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let k = (
                e.is_buy,
                e.source.clone(),
                (e.price * 1_000_000_000.0).round() as i64,
            );
            (k, i)
        })
        .collect();
    if let Some(&idx) = map.get(&key) {
        let existing = &mut out[idx];
        existing.qty += line.qty;
        existing.notional_delta += line.notional_delta;
        existing.account_pct_delta = match (existing.account_pct_delta, line.account_pct_delta) {
            (Some(a), Some(b)) => Some(a + b),
            _ => None,
        };
        return;
    }
    out.push(line);
}

fn collect_alpaca_order_lines_for_symbol(
    orders: &[OrderInfo],
    bare_upper: &str,
    current_price: f64,
    tick_size: f64,
    account_balance: Option<f64>,
    out: &mut Vec<OrderLine>,
) {
    fn walk(
        order: &OrderInfo,
        bare_upper: &str,
        current_price: f64,
        tick_size: f64,
        account_balance: Option<f64>,
        out: &mut Vec<OrderLine>,
    ) {
        let order_sym = order.symbol.replace('/', "").to_ascii_uppercase();
        if !order_sym.is_empty()
            && (symbol_matches_no_alloc(&order_sym, bare_upper)
                || order_sym.contains(bare_upper)
                || bare_upper.contains(&order_sym))
            && alpaca_order_is_working(&order.status)
        {
            let price = order
                .limit_price
                .as_deref()
                .or(order.stop_price.as_deref())
                .and_then(|price| price.parse::<f64>().ok())
                .filter(|price| price.is_finite() && *price > 0.0);
            if let Some(price) = price {
                let qty =
                    (parse_order_qty(&order.qty) - parse_order_qty(&order.filled_qty)).max(0.0);
                if qty > 0.0 && qty.is_finite() {
                    let is_buy = order.side.eq_ignore_ascii_case("buy");
                    let notional = qty * price;
                    let signed_notional = if is_buy { -notional } else { notional };
                    push_or_merge_order_line(
                        out,
                        OrderLine {
                            price,
                            qty,
                            is_buy,
                            source: "Alpaca".to_string(),
                            notional_delta: signed_notional,
                            account_pct_delta: account_balance
                                .filter(|balance| *balance > f64::EPSILON)
                                .map(|balance| signed_notional / balance * 100.0),
                            pips_from_current: (tick_size > f64::EPSILON
                                && current_price.is_finite()
                                && current_price > 0.0)
                                .then_some((price - current_price) / tick_size),
                        },
                    );
                }
            }
        }
        if let Some(legs) = &order.legs {
            for leg in legs {
                walk(
                    leg,
                    bare_upper,
                    current_price,
                    tick_size,
                    account_balance,
                    out,
                );
            }
        }
    }

    for order in orders {
        walk(
            order,
            bare_upper,
            current_price,
            tick_size,
            account_balance,
            out,
        );
    }
}

fn collect_kraken_order_lines_for_symbol(
    orders: &[typhoon_engine::broker::kraken::KrakenOrder],
    bare_upper: &str,
    current_price: f64,
    tick_size: f64,
    account_balance: Option<f64>,
    out: &mut Vec<OrderLine>,
) {
    for order in orders {
        if !alpaca_order_is_working(&order.status) {
            continue;
        }
        let pair_norm = typhoon_engine::core::kraken::normalize_pair_symbol(&order.pair)
            .replace('/', "")
            .to_ascii_uppercase();
        let base = TyphooNApp::kraken_pair_base_ticker(&order.pair);
        if !(symbol_matches_no_alloc(&pair_norm, bare_upper)
            || symbol_matches_no_alloc(&base, bare_upper)
            || pair_norm.contains(bare_upper)
            || bare_upper.contains(&pair_norm))
        {
            continue;
        }
        let price = if order.price > 0.0 {
            order.price
        } else if let Some(limit_price) = order.limitprice.filter(|price| *price > 0.0) {
            limit_price
        } else if let Some(stop_price) = order.stopprice.filter(|price| *price > 0.0) {
            stop_price
        } else {
            continue;
        };
        let qty = (order.vol - order.vol_exec).max(0.0);
        if !(qty > 0.0 && qty.is_finite()) {
            continue;
        }
        let is_buy = order.r#type.eq_ignore_ascii_case("buy");
        let notional = qty * price;
        let signed_notional = if is_buy { -notional } else { notional };
        push_or_merge_order_line(
            out,
            OrderLine {
                price,
                qty,
                is_buy,
                source: "Kraken".to_string(),
                notional_delta: signed_notional,
                account_pct_delta: account_balance
                    .filter(|balance| *balance > f64::EPSILON)
                    .map(|balance| signed_notional / balance * 100.0),
                pips_from_current: (tick_size > f64::EPSILON
                    && current_price.is_finite()
                    && current_price > 0.0)
                    .then_some((price - current_price) / tick_size),
            },
        );
    }
}

pub(super) const MTF_GRID_TIMEFRAMES: [(&str, Timeframe); 9] = [
    ("M1", Timeframe::M1),
    ("M5", Timeframe::M5),
    ("M15", Timeframe::M15),
    ("M30", Timeframe::M30),
    ("H1", Timeframe::H1),
    ("H4", Timeframe::H4),
    ("D1", Timeframe::D1),
    ("W1", Timeframe::W1),
    ("MN1", Timeframe::MN1),
];

/// Max (symbol, timeframe) cells the MTF Grid background fill loads per pass. The
/// right-panel MTF Grid is foreground trading UI, so one pass should cover a normal
/// open grid instead of visibly dribbling rows over many throttle windows. The work
/// still runs on one blocking worker, which bounds cache/decompress pressure without
/// leaving the navbar stagnant.
const MTF_GRID_FILL_PER_BATCH: usize = 256;

/// The MTF Grid's per-cell indicator values: `(close, sma200, kama, fisher,
/// fisher_signal)`. `None` means "no value" (not loaded / insufficient history).
pub(super) type MtfCellValues = (
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MtfChartGroup {
    pub(super) symbol: String,
    pub(super) indices: Vec<usize>,
}

fn mtf_timeframe_rank(tf: Timeframe) -> Option<usize> {
    match tf {
        Timeframe::M1 => Some(0),
        Timeframe::M5 => Some(1),
        Timeframe::M15 => Some(2),
        Timeframe::M30 => Some(3),
        Timeframe::H1 => Some(4),
        Timeframe::H4 => Some(5),
        Timeframe::D1 => Some(6),
        Timeframe::W1 => Some(7),
        Timeframe::MN1 => Some(8),
    }
}

pub(super) fn mtf_grid_symbol_key(symbol: &str) -> String {
    let mut candidate = bare_symbol_from_key(symbol);
    if let Some(stripped) = candidate.strip_suffix(".EQ") {
        candidate = stripped.to_string();
    }
    candidate
}

pub(super) fn chart_company_name_catalog(
    alpaca_assets: &[(String, String, String)],
    kraken_equity_names: &std::collections::HashMap<String, String>,
    primary_broker: OrderBroker,
) -> std::collections::HashMap<String, String> {
    let mut names = std::collections::HashMap::new();
    let insert_alpaca = |names: &mut std::collections::HashMap<String, String>| {
        for (symbol, name, class) in alpaca_assets {
            let symbol = symbol.trim().to_ascii_uppercase();
            let name = name.trim();
            if symbol.is_empty() || name.is_empty() {
                continue;
            }
            if class.eq_ignore_ascii_case("us_equity")
                || class.eq_ignore_ascii_case("stock")
                || class.eq_ignore_ascii_case("equity")
            {
                names.insert(symbol, name.to_string());
            }
        }
    };
    let insert_kraken = |names: &mut std::collections::HashMap<String, String>| {
        for (symbol, name) in kraken_equity_names {
            let symbol = mtf_grid_symbol_key(symbol).to_ascii_uppercase();
            let name = name.trim();
            if !symbol.is_empty() && !name.is_empty() {
                names.insert(symbol, name.to_string());
            }
        }
    };

    match primary_broker {
        OrderBroker::Alpaca => {
            insert_kraken(&mut names);
            insert_alpaca(&mut names);
        }
        OrderBroker::Kraken => {
            insert_alpaca(&mut names);
            insert_kraken(&mut names);
        }
    }
    names
}

fn kraken_position_covers_balance_asset(positions: &[PositionInfo], asset: &str) -> bool {
    let display = TyphooNApp::kraken_display_asset(asset);
    let bare_display = display.strip_suffix(".EQ").unwrap_or(display.as_str());
    // O(1) via temp map (small N, but consistent with by_symbol maps elsewhere)
    let pos_by_sym: std::collections::HashMap<String, &PositionInfo> = positions
        .iter()
        .map(|p| (p.symbol.to_ascii_uppercase(), p))
        .collect();
    pos_by_sym.values().any(|pos| {
        if !pos.qty.is_finite() || pos.qty <= 0.0 || !pos.side.eq_ignore_ascii_case("long") {
            return false;
        }
        let pos_symbol = typhoon_engine::core::kraken::normalize_pair_symbol(&pos.symbol)
            .replace('/', "")
            .to_ascii_uppercase();
        let pos_base = TyphooNApp::kraken_base_asset_for_pair(&pos_symbol);
        TyphooNApp::kraken_asset_keys_match(&display, &pos_symbol)
            || TyphooNApp::kraken_asset_keys_match(bare_display, &pos_symbol)
            || TyphooNApp::kraken_asset_keys_match(&display, &pos_base)
            || TyphooNApp::kraken_asset_keys_match(bare_display, &pos_base)
    })
}

fn mtf_low_timeframe(tf: Timeframe) -> bool {
    matches!(tf, Timeframe::M1 | Timeframe::M5)
}

fn mtf_empty_low_timeframe_backing_chart(chart: &ChartState) -> bool {
    !chart.show_in_tab_bar && chart.bars.is_empty() && mtf_low_timeframe(chart.timeframe)
}

fn low_timeframe_no_data_reason(reason: &str) -> bool {
    let reason = reason.to_ascii_lowercase();
    reason.contains("no data") || reason.contains("no bars")
}

pub(super) fn low_timeframe_no_data_symbols(
    pairs: &std::collections::HashMap<String, UnresolvablePair>,
) -> std::collections::HashSet<String> {
    let mut seen: std::collections::HashMap<(String, String), (bool, bool)> =
        std::collections::HashMap::new();
    for entry in pairs.values() {
        if !low_timeframe_no_data_reason(&entry.reason) {
            continue;
        }
        let Some(tf) = normalize_sync_timeframe_key(&entry.timeframe) else {
            continue;
        };
        if !matches!(tf, "1Min" | "5Min") {
            continue;
        }
        let symbol = mtf_grid_symbol_key(&entry.symbol).to_ascii_uppercase();
        if symbol.is_empty() {
            continue;
        }
        let flags = seen
            .entry((entry.broker.to_ascii_lowercase(), symbol))
            .or_insert((false, false));
        match tf {
            "1Min" => flags.0 = true,
            "5Min" => flags.1 = true,
            _ => {}
        }
    }
    seen.into_iter()
        .filter_map(|((_broker, symbol), (has_m1, has_m5))| (has_m1 && has_m5).then_some(symbol))
        .collect()
}

pub(super) fn open_chart_preload_indices(charts: &[ChartState]) -> Vec<usize> {
    charts
        .iter()
        .enumerate()
        .filter_map(|(idx, chart)| {
            (chart.bars.is_empty() && !mtf_empty_low_timeframe_backing_chart(chart)).then_some(idx)
        })
        .collect()
}

#[cfg(test)]
pub(super) fn mtf_visible_chart_groups(
    charts: &[ChartState],
    visible: &[bool],
) -> Vec<MtfChartGroup> {
    mtf_visible_chart_groups_filtered(charts, visible, &std::collections::HashSet::new())
}

pub(super) fn mtf_visible_chart_groups_filtered(
    charts: &[ChartState],
    visible: &[bool],
    suppressed_symbols: &std::collections::HashSet<String>,
) -> Vec<MtfChartGroup> {
    let mut groups: Vec<MtfChartGroup> = Vec::new();
    for (idx, chart) in charts.iter().enumerate() {
        if !visible.get(idx).copied().unwrap_or(true)
            || mtf_timeframe_rank(chart.timeframe).is_none()
            || mtf_empty_low_timeframe_backing_chart(chart)
        {
            continue;
        }
        let symbol = mtf_grid_symbol_key(&chart.symbol);
        if symbol.is_empty() || suppressed_symbols.contains(&symbol.to_ascii_uppercase()) {
            continue;
        }
        if let Some(group) = groups.iter_mut().find(|group| group.symbol == symbol) {
            // small N, or could map but groups mutable
            group.indices.push(idx);
        } else {
            groups.push(MtfChartGroup {
                symbol,
                indices: vec![idx],
            });
        }
    }
    groups.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    for group in &mut groups {
        group.indices.sort_by(|&a, &b| {
            let rank_a = mtf_timeframe_rank(charts[a].timeframe).unwrap_or(usize::MAX);
            let rank_b = mtf_timeframe_rank(charts[b].timeframe).unwrap_or(usize::MAX);
            rank_a.cmp(&rank_b).then_with(|| a.cmp(&b))
        });
    }
    groups
}

pub(super) fn mtf_flat_chart_indices(groups: &[MtfChartGroup]) -> Vec<usize> {
    groups
        .iter()
        .flat_map(|group| group.indices.iter().copied())
        .collect()
}

pub(super) fn mtf_canvas_grid_cols(_cell_count: usize) -> usize {
    2
}

pub(super) fn mtf_canvas_grid_rows(cell_count: usize) -> usize {
    let cols = mtf_canvas_grid_cols(cell_count);
    (cell_count + cols - 1) / cols
}

/// True iff `raw` becomes `target_upper` after stripping `'/'` and uppercasing
/// (ASCII). Avoids the per-call `raw.replace('/', "").to_uppercase()` allocation
/// that build_trade_overlay used to do once per scanned position.
fn symbol_matches_no_alloc(raw: &str, target_upper: &str) -> bool {
    let mut t = target_upper.bytes();
    for byte in raw.bytes() {
        if byte == b'/' {
            continue;
        }
        let upper = byte.to_ascii_uppercase();
        match t.next() {
            Some(tb) if tb == upper => {}
            _ => return false,
        }
    }
    t.next().is_none()
}

impl TyphooNApp {
    pub(crate) fn tick_chart_background_results(&mut self) {
        // ── MTF Grid background fill: clear the in-flight guard when the worker
        // finishes (it writes the unified result cache directly, so there are no
        // results to marshal here). Disconnected = worker dropped/panicked; clear
        // too so the throttled refresh can spawn the next pass.
        if let Some(rx) = self.mtf_grid_rx.as_ref() {
            match rx.try_recv() {
                Ok(()) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.mtf_grid_rx = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }

        // ── receive Reg SHO cached prices from background thread (non-blocking) ──
        if let Some(ref rx) = self.regulatory_prices_rx {
            if let Ok(results) = rx.try_recv() {
                for (sym, row) in results {
                    self.regulatory_prices.insert(sym, row);
                }
                self.regulatory_prices_rx = None; // done
            }
        }
    }

    pub(crate) fn tick_dirty_indicator_recompute(&mut self) {
        // ── recompute indicators when periods changed in UI ──────────────
        if self.indicators_dirty {
            self.indicators_dirty = false;
            let mut gpu = self.gpu_indicators.take();
            // MAX PERFORMANCE: During heavy sync, completely skip indicator computation
            // for everything except the single active chart, and even then only if
            // we are not in a forming bar update (which has its own O(1) path).
            if let Some(chart) = self.charts.get_mut(self.active_tab) {
                if chart.bars.is_empty() {
                    // O(1) skip
                } else if !self.heavy_sync_in_progress {
                    chart.compute_indicators_gpu(gpu.as_mut());
                } else if chart.forming_bar_dirty {
                    chart.compute_indicators_gpu(gpu.as_mut());
                }
            }
            self.gpu_indicators = gpu;
        }
    }

    pub(crate) fn tick_deferred_chart_loads(
        &mut self,
        ctx: &egui::Context,
        now_instant: std::time::Instant,
    ) {
        // Keep every open chart tab warm, not just the active tab or currently
        // visible MTF cells. Users should be able to switch tabs without that
        // click being the first time bars/indicators are loaded.
        for idx in open_chart_preload_indices(&self.charts) {
            if self.should_queue_empty_chart_reload(idx, now_instant) {
                self.queue_chart_reload(idx);
            }
        }

        // ── deferred chart loading: off the render thread ──
        // The cold load (SQLite read + zstd decompress + equity merge + HTF overlay
        // reads) is expensive and previously ran here, synchronously, one chart per
        // frame — every frame the render thread stalled 150–900ms before drawing a
        // single panel. Now the heavy work runs on the worker pool
        // (`spawn_deferred_chart_load`), which publishes into the shared result/HTF/
        // value caches. The render thread only drains completions and does the cheap
        // in-memory restore (bars clone + GPU indicators) for cells a worker prepared.

        // Drain worker completions: clear the in-flight marker; for empty results,
        // drop the matching queued chart(s) and throttle the retry (mirrors the old
        // synchronous empty-load handling so MTF render loops don't respin them).
        let mut completions: Vec<(String, &'static str, bool)> = Vec::new();
        if let Some(rx) = self.deferred_chart_load_rx.as_ref() {
            while let Ok(msg) = rx.try_recv() {
                completions.push(msg);
            }
        }
        // Evict stale in-flight markers: a worker that hung/deadlocked never sends a
        // completion, which would otherwise strand its cell forever (the spawn-dedup
        // check would keep skipping it). Eviction makes it re-spawnable.
        self.deferred_chart_inflight.retain(|key, spawned_at| {
            if now_instant.duration_since(*spawned_at) < DEFERRED_CHART_INFLIGHT_STALE_AFTER {
                return true;
            }
            tracing::warn!(
                "Deferred chart loader stale (no completion in {}s): {} [{}] — will retry",
                DEFERRED_CHART_INFLIGHT_STALE_AFTER.as_secs(),
                key.0,
                key.1
            );
            false
        });
        for (sym_key, tf, had_bars) in completions {
            self.deferred_chart_inflight.remove(&(sym_key.clone(), tf));
            if had_bars {
                continue;
            }
            let mut j = 0;
            while j < self.deferred_chart_loads.len() {
                let idx = self.deferred_chart_loads[j];
                let matches = self
                    .charts
                    .get(idx)
                    .map(|c| {
                        c.source_override.is_empty()
                            && c.timeframe.cache_suffix() == tf
                            && mtf_grid_symbol_key(&c.symbol) == sym_key
                    })
                    .unwrap_or(false);
                if matches {
                    if let Some(key) = self.charts.get(idx).map(deferred_chart_load_key) {
                        self.deferred_chart_empty_load_at.insert(key, now_instant);
                    }
                    self.deferred_chart_loads.remove(j);
                    self.deferred_chart_load_set.remove(&idx);
                } else {
                    j += 1;
                }
            }
        }

        if self.deferred_chart_loads.is_empty() {
            return;
        }
        let Some(cache) = self.cache.clone() else {
            return;
        };

        // Walk the queue front-to-back. Finalize charts whose background load has
        // landed (cheap restore, budget-capped), and spawn workers for upcoming
        // not-yet-loading charts up to the concurrency cap.
        let mut gpu = self.gpu_indicators.take();
        let now_ms = chrono::Utc::now().timestamp_millis();
        let mut finalized = 0usize;
        let mut scanned = 0usize;
        let mut i = 0usize;
        while i < self.deferred_chart_loads.len() && scanned < DEFERRED_CHART_SCAN_WINDOW {
            scanned += 1;
            let idx = self.deferred_chart_loads[i];
            let Some((load_key, sym_key, tf, source_override, bars_empty, symbol, timeframe)) =
                self.charts.get(idx).map(|c| {
                    (
                        deferred_chart_load_key(c),
                        mtf_grid_symbol_key(&c.symbol),
                        c.timeframe.cache_suffix(),
                        c.source_override,
                        c.bars.is_empty(),
                        c.symbol.clone(),
                        c.timeframe,
                    )
                })
            else {
                // Stale index — drop it.
                self.deferred_chart_loads.remove(i);
                self.deferred_chart_load_set.remove(&idx);
                continue;
            };

            // Synchronous in-place load for (a) charts pinned to an explicit source
            // and (b) refreshes of an already-populated chart (a fetch result, style
            // change, etc.). `try_load` refreshes in place and preserves the camera,
            // which the async restore path (empty→loaded only) cannot. The async
            // worker path below is reserved for the cold-start case — an empty,
            // auto-source chart — which is the startup-freeze hot path. Populated
            // reloads arrive one at a time, so running them here can't re-freeze.
            if !source_override.is_empty() || !bars_empty {
                if finalized >= DEFERRED_CHART_FINALIZE_PER_FRAME {
                    i += 1;
                    continue;
                }
                let loaded = self
                    .charts
                    .get_mut(idx)
                    .map(|c| c.try_load(&cache, &mut self.log, gpu.as_mut()))
                    .unwrap_or(true);
                if loaded {
                    let empty = self
                        .charts
                        .get(idx)
                        .map(|c| c.bars.is_empty())
                        .unwrap_or(false);
                    if empty {
                        self.deferred_chart_empty_load_at
                            .insert(load_key, now_instant);
                    } else {
                        self.deferred_chart_empty_load_at.remove(&load_key);
                        if let Some(c) = self.charts.get(idx) {
                            c.publish_result_to_cache();
                        }
                    }
                    self.deferred_chart_loads.remove(i);
                    self.deferred_chart_load_set.remove(&idx);
                    finalized += 1;
                    continue;
                }
                i += 1;
                continue;
            }

            // Ready in the shared result cache (a worker prepared it)? Probe cheaply
            // before mutating, then do the budget-capped in-memory restore.
            let ready = super::chart::chart_result_cache_get(&sym_key, tf, now_ms)
                .map(|e| !e.bars.is_empty())
                .unwrap_or(false);
            if ready {
                if finalized >= DEFERRED_CHART_FINALIZE_PER_FRAME {
                    i += 1;
                    continue;
                }
                let restored = self
                    .charts
                    .get_mut(idx)
                    .map(|c| c.restore_from_result_cache(&cache, gpu.as_mut()))
                    .unwrap_or(false);
                if restored {
                    self.deferred_chart_empty_load_at.remove(&load_key);
                    if let Some(c) = self.charts.get(idx) {
                        c.publish_result_to_cache();
                    }
                }
                // Drop it either way: a non-empty entry restores; a microscopic
                // TTL-race miss leaves bars empty and re-queues via the preload pass.
                self.deferred_chart_loads.remove(i);
                self.deferred_chart_load_set.remove(&idx);
                finalized += 1;
                continue;
            }

            // Not loaded yet — ensure a worker is on it, then advance so other queued
            // cells get scheduled in the same frame.
            let key = (sym_key, tf);
            if !self.deferred_chart_inflight.contains_key(&key)
                && self.deferred_chart_inflight.len() < DEFERRED_CHART_MAX_INFLIGHT
            {
                self.deferred_chart_inflight
                    .insert(key.clone(), now_instant);
                self.spawn_deferred_chart_load(&cache, symbol, timeframe, key.0, tf);
            }
            i += 1;
        }
        self.gpu_indicators = gpu;

        // Keep frames coming while loads are outstanding (covers TYPHOON_IDLE_FPS
        // caps where the app would otherwise not continuously repaint).
        if !self.deferred_chart_loads.is_empty() {
            ctx.request_repaint();
        }
    }

    /// Load one (symbol, timeframe) on the worker pool and publish the result into
    /// the shared result/HTF/value caches so the render thread can restore it
    /// cheaply. Reports `(symbol_key, tf_suffix, had_bars)` back so the scheduler
    /// can clear the in-flight marker and retire empty results. Mirrors the MTF Grid
    /// navbar fill (`compute_mtf_grid_status`) — same off-thread load + cache publish.
    fn spawn_deferred_chart_load(
        &mut self,
        cache: &Arc<SqliteCache>,
        symbol: String,
        timeframe: Timeframe,
        sym_key: String,
        tf_suffix: &'static str,
    ) {
        if self.deferred_chart_load_tx.is_none() {
            let (tx, rx) = std::sync::mpsc::channel();
            self.deferred_chart_load_tx = Some(tx);
            self.deferred_chart_load_rx = Some(rx);
        }
        let Some(tx) = self.deferred_chart_load_tx.as_ref().map(Clone::clone) else {
            return;
        };
        let cache = Arc::clone(cache);
        let rt_handle = self.rt_handle.clone();
        rt_handle.spawn_blocking(move || {
            let mut temp = ChartState::new(&symbol, timeframe);
            let dsm = typhoon_engine::core::data_source::DataSourceManager::default();
            temp.load(&cache, &mut std::collections::VecDeque::new(), None, &dsm);
            let had_bars = !temp.bars.is_empty();
            if had_bars {
                let now_ms = chrono::Utc::now().timestamp_millis();
                let close = temp.bars.last().map(|b| b.close);
                let sma = temp.sma200.last().and_then(|v| *v);
                let kama = temp.kama.last().and_then(|v| *v);
                let fisher = temp.fisher.last().and_then(|v| *v);
                let fsig = temp.fisher_signal.last().and_then(|v| *v);
                let source = temp.primary_source;
                let bars = std::sync::Arc::new(std::mem::take(&mut temp.bars));
                // Bars → shared HTF cache (overlays reuse them) and the reopen cache;
                // values → the sticky grid store the navbar reads.
                super::chart::mtf_htf_cache_put(
                    &sym_key,
                    tf_suffix,
                    std::sync::Arc::clone(&bars),
                    now_ms,
                );
                super::chart::chart_result_cache_put(&sym_key, tf_suffix, bars, source, now_ms);
                super::chart::mtf_grid_value_put(
                    &sym_key,
                    tf_suffix,
                    (close, sma, kama, fisher, fsig),
                    now_ms,
                );
            }
            let _ = tx.send((sym_key, tf_suffix, had_bars));
        });
    }
}

impl TyphooNApp {
    pub(super) fn close_partial_active_symbol(&mut self) {
        let Some((symbol, _)) = self.active_trade_symbol_and_price() else {
            self.log.push_back(LogEntry::warn(
                "Close Partial: active chart symbol unavailable",
            ));
            return;
        };
        let (send_alpaca, send_kraken) = self.selected_live_broker_targets();
        if !send_alpaca && !send_kraken {
            self.log.push_back(LogEntry::warn(
                "Close Partial: no broker connected for selected target",
            ));
            return;
        }
        let sl = self.sl_enabled.then_some(self.sl_price).flatten();
        let tp = self.tp_enabled.then_some(self.tp_price).flatten();
        let mut any = false;

        if send_alpaca {
            let bare = bare_symbol_from_key(&symbol);
            if let Some(pos) = self.live_positions_by_symbol.get(&bare) {
                let half_qty = pos.qty.abs() / 2.0;
                if half_qty > 0.0 {
                    let remaining_qty = (pos.qty.abs() - half_qty).max(0.0);
                    let _ = self.broker_tx.send(BrokerCmd::AlpacaClosePositionPercent {
                        symbol: symbol.clone(),
                        percentage: 50.0,
                    });
                    if remaining_qty > 0.0 && (sl.is_some() || tp.is_some()) {
                        let _ = self.broker_tx.send(BrokerCmd::AlpacaSyncExits {
                            symbol: symbol.clone(),
                            sl_price: sl,
                            tp_price: tp,
                            wait_for_qty_at_most: Some(remaining_qty),
                        });
                    }
                    any = true;
                    self.log.push_back(LogEntry::info(format!(
                        "Close Partial: Alpaca {} {:.4}",
                        symbol, half_qty
                    )));
                }
            } else {
                self.log.push_back(LogEntry::warn(format!(
                    "Close Partial: no Alpaca position found for {}",
                    symbol
                )));
            }
        }
        if send_kraken {
            let bare = bare_symbol_from_key(&symbol);
            if let Some(pos) = self.kr_positions_by_symbol.get(&bare) {
                let half_qty = pos.qty.abs() / 2.0;
                if half_qty > 0.0 {
                    let remaining_qty = (pos.qty.abs() - half_qty).max(0.0);
                    let _ = self.broker_tx.send(BrokerCmd::KrakenClosePosition {
                        pair: symbol.clone(),
                        volume: Some(half_qty),
                    });
                    if remaining_qty > 0.0 && (sl.is_some() || tp.is_some()) {
                        let _ = self.broker_tx.send(BrokerCmd::KrakenSyncExits {
                            pair: symbol.clone(),
                            sl_price: sl,
                            tp_price: tp,
                            wait_for_position: true,
                            wait_for_qty_at_most: Some(remaining_qty),
                        });
                    }
                    any = true;
                    self.log.push_back(LogEntry::info(format!(
                        "Close Partial: Kraken {} {:.6}",
                        symbol, half_qty
                    )));
                }
            } else {
                self.log.push_back(LogEntry::warn(format!(
                    "Close Partial: no Kraken position found for {}",
                    symbol
                )));
            }
        }
        if !any {
            self.log.push_back(LogEntry::warn(
                "Close Partial: no position size available to reduce",
            ));
        }
    }

    /// Load cached daily-bar prices for every regulatory-alert symbol (Reg SHO
    /// threshold OR trading halt) not already in the watchlist, off the render
    /// thread (the same SQLite-read stall pitfall as the MTF grid: a bulk
    /// bar-sync writer can hold the conn mutex). Results are merged into
    /// `regulatory_prices` so the Reg SHO and Halts windows fill their Last /
    /// Daily-close / Chg% columns for the whole list; live bid/ask still come
    /// from watchlisted symbols only (the windows are cache-based).
    pub(super) fn spawn_regulatory_price_load(&mut self) {
        let cache = match &self.cache {
            Some(c) => Arc::clone(c),
            None => return,
        };
        let in_watchlist: std::collections::HashSet<String> = self
            .watchlist_rows
            .iter()
            .map(|r| r.symbol.to_ascii_uppercase())
            .collect();
        let symbols: Vec<String> = self
            .bg
            .regulatory_alerts_by_symbol
            .keys()
            .filter(|s| !in_watchlist.contains(&s.to_ascii_uppercase()))
            .cloned()
            .collect();
        if symbols.is_empty() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let rt_handle = self.rt_handle.clone();
        rt_handle.spawn_blocking(move || {
            // Daily first — the window's columns are daily close / daily change.
            let tfs = ["1Day", "4Hour", "1Hour"];
            let mut out: Vec<(String, WatchlistRow)> = Vec::new();
            for sym in symbols {
                'search: for tf in tfs {
                    for source in ["alpaca", "kraken", "kraken-equities", "default"] {
                        for key in chart_source_cache_keys(source, &sym, tf) {
                            if let Ok(Some(raw)) = cache.get_bars_raw(&key) {
                                if let Some(mut row) = watchlist_row_from_raw_bars(&sym, &key, &raw)
                                {
                                    // For a daily bar the last close IS the daily
                                    // close — surface it so Dly Close fills too.
                                    if tf.eq_ignore_ascii_case("1Day") {
                                        row.regular_close = row.last;
                                    }
                                    out.push((sym.clone(), row));
                                    break 'search;
                                }
                            }
                        }
                    }
                }
            }
            let _ = tx.send(out);
        });
        self.regulatory_prices_rx = Some(rx);
    }

    /// Force a market-data refresh for every symbol shown in the regulatory
    /// windows (Reg SHO threshold + trading halts) by queueing a daily-bar fetch
    /// per symbol — least-fresh, or no-data, symbols first so the emptiest rows
    /// fill soonest. One `1Day` fetch per symbol (the windows' columns are daily
    /// close / daily change); the broker queue's pending cap, per-symbol cooldown
    /// and freshness classifier throttle or skip the rest. Freshly fetched bars
    /// surface through the window's throttled `spawn_regulatory_price_load` read.
    pub(super) fn refresh_regulatory_prices(&mut self) {
        if self.bg.regulatory_alerts_by_symbol.is_empty() {
            return;
        }
        // Rank by the newest cache write-ts across the same source/timeframe keys
        // the price load reads; symbols with no cached bar sort first (i64::MIN).
        // Compute the order in a block so all immutable `self.bg` borrows end
        // before the `&mut self` fetch loop.
        let symbols: Vec<String> = {
            let ts_by_key: std::collections::HashMap<&str, i64> = self
                .bg
                .detailed_stats
                .iter()
                .map(|(key, _bars, ts)| (key.as_str(), *ts))
                .collect();
            let freshness = |sym: &str| -> i64 {
                let mut newest = i64::MIN;
                for tf in ["1Day", "4Hour", "1Hour"] {
                    for source in ["alpaca", "kraken", "kraken-equities", "default"] {
                        for key in chart_source_cache_keys(source, sym, tf) {
                            if let Some(&ts) = ts_by_key.get(key.as_str()) {
                                newest = newest.max(ts);
                            }
                        }
                    }
                }
                newest
            };
            let mut ranked: Vec<(i64, String)> = self
                .bg
                .regulatory_alerts_by_symbol
                .keys()
                .map(|sym| (freshness(sym), sym.clone()))
                .collect();
            ranked.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
            ranked.into_iter().map(|(_, sym)| sym).collect()
        };
        for sym in &symbols {
            self.queue_symbol_fetch_for_source(sym, "1Day");
        }
        // Re-read the cache promptly so the table reflects fetched bars as they land.
        self.regulatory_price_read_at = None;
    }

    pub(super) fn reload_symbol(&mut self, symbol: &str, tf: Timeframe) {
        // NOTE: For live Kraken WS forming-bar updates, prefer
        // chart.apply_forming_bar_update() + chart.mark_structural_change()
        // over a full reload to hit the draw_chart early-out.
        // Full reloads should only happen on closed bars or user-initiated symbol change.
        self.reload_symbol_auto(symbol, tf);
        self.queue_open_symbol_sync_all_timeframes(symbol);
    }

    pub(super) fn queue_open_symbol_sync_all_timeframes(&mut self, symbol: &str) -> usize {
        let symbol = symbol.trim();
        if symbol.is_empty() {
            return 0;
        }
        let timeframes = self.enabled_standard_sync_timeframes();
        let mut queued = 0usize;
        for tf in timeframes {
            if self.queue_symbol_fetch_for_source(symbol, &tf) {
                queued += 1;
            }
        }
        queued
    }

    fn queue_symbol_fetch_for_source(&mut self, symbol: &str, tf_key: &str) -> bool {
        if !self.sync_timeframe_enabled(tf_key) {
            return false;
        }
        let bare = normalize_market_data_symbol(symbol)
            .replace('/', "")
            .trim_end_matches(".EQ")
            .to_ascii_uppercase();
        if self.kraken_enabled
            && self.kraken_scrape_xstocks
            && self
                .kraken_equity_universe_symbols
                .binary_search_by(|candidate| candidate.as_str().cmp(bare.as_str()))
                .is_ok()
        {
            self.dispatch_kraken_equity_ticker(&bare);
            if self.queue_kraken_equity_fetch(&bare, tf_key) {
                return true;
            }
        }
        let kraken_symbol = typhoon_engine::core::kraken::normalize_pair_symbol(symbol);
        if !kraken_symbol.is_empty()
            && typhoon_engine::core::kraken::to_kraken_pair_lossy(&kraken_symbol).is_some()
            && self.queue_kraken_fetch(&kraken_symbol, tf_key)
        {
            return true;
        }
        self.queue_alpaca_fetch(symbol, tf_key)
    }

    pub(super) fn reload_symbol_auto(&mut self, symbol: &str, tf: Timeframe) {
        if let Some(cache) = self.cache.clone() {
            let (chart_type, source_override) = self
                .charts
                .get(self.active_tab)
                .map(|c| (c.chart_type, c.source_override))
                .unwrap_or((ChartType::Candle, ""));
            let mut chart = ChartState::new(symbol, tf);
            chart.chart_type = chart_type;
            chart.source_override = source_override;

            // Preserve manual camera on reload / MTF restore for live symbols.
            // User free-look (drag/zoom) should survive sync or tab restore.
            let prior_manual = self
                .charts
                .iter()
                .find(|c| c.symbol.eq_ignore_ascii_case(symbol) && c.timeframe == tf)
                .map(|c| {
                    (
                        c.manual_view_override,
                        c.camera.clone(),
                        c.view_offset,
                        c.visible_bars,
                    )
                });
            if let Some((was_manual, cam, vo, vb)) = prior_manual {
                if was_manual {
                    chart.manual_view_override = true;
                    chart.camera = cam;
                    chart.view_offset = vo;
                    chart.visible_bars = vb;
                }
            }
            let cache_ref = Arc::as_ref(&cache);
            let mut gpu = self.gpu_indicators.take();
            let load_succeeded = chart.try_load(cache_ref, &mut self.log, gpu.as_mut());
            self.gpu_indicators = gpu;
            if !load_succeeded {
                // Read error (not contention — read_conn is UI-exclusive)
                self.log
                    .push_back(LogEntry::err("Cache read error — check logs"));
            } else if chart.bars.is_empty() {
                let tf_key = tf.cache_suffix();
                let kraken_symbol = typhoon_engine::core::kraken::normalize_pair_symbol(symbol);
                let kraken_supported =
                    typhoon_engine::core::kraken::to_kraken_pair_lossy(&kraken_symbol).is_some();
                if !self.sync_timeframe_enabled(tf_key) {
                    self.log.push_back(LogEntry::warn(format!(
                        "No cached data for {} {} — sync for {} is disabled",
                        symbol,
                        tf.label(),
                        sync_timeframe_short_label(tf_key)
                    )));
                } else if kraken_supported {
                    let queued = self.queue_kraken_fetch(&kraken_symbol, tf_key);
                    if queued {
                        self.log.push_back(LogEntry::info(format!(
                            "No cached data for {} {} — fetching from Kraken...",
                            symbol,
                            tf.label()
                        )));
                    }
                } else if self.queue_alpaca_fetch(symbol, tf_key) {
                    self.log.push_back(LogEntry::info(format!(
                        "No cached data for {} {} — fetching from Alpaca...",
                        symbol,
                        tf.label()
                    )));
                }
            }
            let split_probe_symbol = normalize_market_data_symbol(symbol)
                .replace('/', "")
                .trim_end_matches(".EQ")
                .to_ascii_uppercase();
            let split_probe_is_equity = !split_probe_symbol.is_empty()
                && split_probe_symbol.len() <= 6
                && split_probe_symbol.chars().all(|c| c.is_ascii_alphabetic());
            if split_probe_is_equity
                && !(self.splits_loading
                    && self.splits_symbol.eq_ignore_ascii_case(&split_probe_symbol))
            {
                let splits_cached = cache_ref
                    .read_connection()
                    .ok()
                    .and_then(|conn| {
                        typhoon_engine::core::research::get_stock_splits(&conn, &split_probe_symbol)
                            .ok()
                    })
                    .is_some();
                if !splits_cached {
                    self.splits_loading = true;
                    self.splits_symbol = split_probe_symbol.clone();
                    let _ = self.broker_tx.send(BrokerCmd::FetchStockSplits {
                        symbol: split_probe_symbol,
                        fmp_key: self.fmp_key.clone(),
                    });
                }
            }
            if let Some(target) = self.charts.get_mut(self.active_tab) {
                *target = chart;
            }
            // Refresh MTF Grid status for all timeframes
            self.compute_mtf_grid_status();
        } else {
            self.log.push_back(LogEntry::warn("Cache not available"));
        }
    }

    pub(super) fn queue_chart_reload(&mut self, idx: usize) {
        if idx < self.charts.len() && self.deferred_chart_load_set.insert(idx) {
            self.deferred_chart_loads.push_back(idx);
        }
    }

    /// Queue every open chart that has empty bars for the off-thread deferred
    /// loader. O(1) per chart (HashSet insert + push, no cache I/O) — replaces the
    /// "synchronously `try_load` every empty chart in a loop on the render thread"
    /// idiom that froze the UI for seconds when enabling/restoring an MTF grid.
    pub(super) fn queue_empty_charts_for_load(&mut self) {
        for idx in 0..self.charts.len() {
            if self
                .charts
                .get(idx)
                .map(|c| c.bars.is_empty())
                .unwrap_or(false)
            {
                self.queue_chart_reload(idx);
            }
        }
    }

    /// Queue every open chart for reload via the deferred loader (the RELOAD
    /// command). Empty charts load off-thread; already-populated charts route to
    /// the budget-gated synchronous refresh inside the scheduler, so even a full
    /// reload spreads across frames instead of blocking one.
    pub(super) fn queue_all_charts_for_reload(&mut self) {
        for idx in 0..self.charts.len() {
            self.queue_chart_reload(idx);
        }
    }

    pub(super) fn should_queue_empty_chart_reload(
        &self,
        idx: usize,
        now: std::time::Instant,
    ) -> bool {
        let Some(chart) = self.charts.get(idx) else {
            return false;
        };
        if !chart.bars.is_empty() {
            return false;
        }
        let key = deferred_chart_load_key(chart);
        empty_chart_load_retry_due(self.deferred_chart_empty_load_at.get(&key).copied(), now)
    }

    pub(super) fn normalize_news_ticker_for_chart(raw: &str) -> Option<String> {
        let symbol = normalize_market_data_symbol(raw)
            .trim()
            .trim_matches(|ch: char| {
                !ch.is_ascii_alphanumeric() && ch != '.' && ch != '-' && ch != '/'
            })
            .to_ascii_uppercase();
        let valid_chars = symbol
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '/');
        let well_formed_parts = symbol
            .split(['.', '-', '/'])
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_alphanumeric()));
        if symbol.is_empty() || symbol.len() > 16 || !valid_chars || !well_formed_parts {
            None
        } else {
            Some(symbol)
        }
    }

    pub(super) fn news_article_tickers(primary_symbol: &str, tickers: &[String]) -> Vec<String> {
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        if let Some(primary) = Self::normalize_news_ticker_for_chart(primary_symbol) {
            if seen.insert(primary.clone()) {
                out.push(primary);
            }
        }
        for ticker in tickers {
            let Some(ticker) = Self::normalize_news_ticker_for_chart(ticker) else {
                continue;
            };
            if seen.insert(ticker.clone()) {
                out.push(ticker);
            }
        }
        out
    }

    pub(super) fn open_news_ticker_chart(&mut self, raw_ticker: &str) -> bool {
        let Some(symbol) = Self::normalize_news_ticker_for_chart(raw_ticker) else {
            return false;
        };

        // `false`: the explicit focus below selects the D1 tab itself (and works
        // for hidden backing charts too), so ensure must not also move active_tab.
        self.ensure_mtf_grid_for_symbol(&symbol, false);
        let symbol_key = symbol
            .replace('/', "")
            .trim_end_matches(".EQ")
            .to_ascii_uppercase();
        if let Some(existing_idx) = self.chart_by_bare.get(&symbol_key).and_then(|indices| {
            indices.iter().copied().find(|&idx| {
                self.charts
                    .get(idx)
                    .is_some_and(|chart| chart.timeframe == Timeframe::D1)
            })
        }) {
            self.active_tab = existing_idx;
        }
        self.symbol_input = symbol.clone();
        self.mtf_enabled = true;
        self.compute_mtf_grid_status();
        self.log.push_back(LogEntry::info(format!(
            "News: opened/focused {} MTF grid",
            symbol
        )));
        true
    }

    /// Compute MTF Grid indicator status for all timeframes from cache.
    /// Parallel: spawns threads for TFs not already loaded in chart tabs.
    /// Order-independent hash of the open-chart `(symbol-key, timeframe)` set. It
    /// changes whenever a chart is opened, closed, or retimeframed, so the grid's
    /// cache fallback (`mtf_grid_status`) can be recomputed for the new layout —
    /// otherwise a just-closed timeframe drops to a stale/empty cell.
    pub(super) fn mtf_open_chart_signature(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        // XOR per-chart hashes → independent of tab order; fold in the count so
        // adding then removing different charts can't alias to the same value.
        let mut acc: u64 = self.charts.len() as u64;
        for c in &self.charts {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            mtf_grid_symbol_key(&c.symbol).hash(&mut h);
            c.timeframe.label().hash(&mut h);
            acc ^= h.finish();
        }
        acc
    }

    /// The symbols shown in the MTF Grid navbar: one per distinct open **tab**
    /// (`show_in_tab_bar`), sorted, with the supported-timeframe and low-TF-no-data
    /// filters applied. Drives the grid off the user's actual open tabs — no hidden
    /// backing charts — so the dot rows match the tab strip.
    pub(super) fn mtf_grid_navbar_symbols(&self) -> Vec<String> {
        let suppressed = low_timeframe_no_data_symbols(&self.unresolvable_pairs);
        let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for chart in &self.charts {
            if !chart.show_in_tab_bar || mtf_timeframe_rank(chart.timeframe).is_none() {
                continue;
            }
            let key = mtf_grid_symbol_key(&chart.symbol);
            if key.is_empty() || suppressed.contains(&key.to_ascii_uppercase()) {
                continue;
            }
            seen.insert(key);
        }
        seen.into_iter().collect()
    }

    /// Per-timeframe MTF Grid values for one symbol, in timeframe order, for the
    /// timeframes that have data. Each cell prefers a live open tab (always current)
    /// and otherwise reads the unified result cache, which the background fill
    /// (`compute_mtf_grid_status`) keeps warm for cells with no open tab. A timeframe
    /// with neither source is omitted — so M1/M5 only appear when a provider actually
    /// has them, and an as-yet-unfilled cell is simply absent rather than grey-forced.
    pub(super) fn mtf_grid_symbol_values(
        &self,
        symbol_key: &str,
    ) -> Vec<(&'static str, MtfCellValues)> {
        let now_ms = chrono::Utc::now().timestamp_millis();
        // One pass over the charts: collect this symbol's live open-tab values by
        // timeframe (avoids re-scanning per timeframe).
        let mut live: std::collections::HashMap<Timeframe, MtfCellValues> =
            std::collections::HashMap::new();
        for c in &self.charts {
            if c.show_in_tab_bar
                && !c.bars.is_empty()
                && mtf_grid_symbol_key(&c.symbol).eq_ignore_ascii_case(symbol_key)
            {
                live.entry(c.timeframe).or_insert((
                    c.fresh_live_quote_mid()
                        .or_else(|| c.bars.last().map(|b| b.close)),
                    c.sma200.last().and_then(|v| *v),
                    c.kama.last().and_then(|v| *v),
                    c.fisher.last().and_then(|v| *v),
                    c.fisher_signal.last().and_then(|v| *v),
                ));
            }
        }
        let mut out: Vec<(&'static str, MtfCellValues)> = Vec::new();
        for &(label, tf) in &MTF_GRID_TIMEFRAMES {
            // Only timeframes enabled in Sync are shown — a disabled TF (e.g. M1/M5)
            // is never loaded, so it never appears as a column.
            if !self.enabled_sync_timeframes.contains(tf.cache_suffix()) {
                continue;
            }
            // Prefer a live open tab (always current); otherwise the sticky value
            // store, which the fill keeps warm and which outlives the bars TTL so the
            // dot doesn't blink to grey while a slow refill is in flight.
            let vals = live.get(&tf).copied().or_else(|| {
                super::chart::mtf_grid_value_get(symbol_key, tf.cache_suffix(), now_ms)
            });
            if let Some(v) = vals {
                out.push((label, v));
            }
        }
        out
    }

    /// Background fill for the MTF Grid's unified result cache. For every navbar
    /// symbol's timeframe with neither an open tab nor a fresh cache entry, this
    /// loads the bars off the render thread (a throttled, capped batch) and writes
    /// the last indicator values + bars into the result cache, where the grid render
    /// and chart reopens read them. Replaces the old hidden backing charts with the
    /// same data, cached + TTL-pruned instead of held in persistent ChartStates the
    /// sync loop had to maintain. This is foreground MTF Grid work: do not defer it
    /// behind heavy full-universe sync, or visible rows sit half-empty for minutes.
    /// (Name kept for its call sites; `mtf_grid_status_*` are now throttle bookkeeping
    /// read by the navbar pre-block.)
    pub(super) fn compute_mtf_grid_status(&mut self) {
        let cache = match &self.cache {
            Some(c) => Arc::clone(c),
            None => return,
        };
        self.mtf_grid_status_symbol = self.symbol_input.trim().to_string();
        self.mtf_grid_status_open_sig = self.mtf_open_chart_signature();
        self.mtf_grid_status_at = Some(std::time::Instant::now());
        let now_ms = chrono::Utc::now().timestamp_millis();
        let active_key = mtf_grid_symbol_key(&self.symbol_input).to_ascii_uppercase();
        // (symbol, tf) cells with no open tab and no fresh cache entry.
        let mut cells: Vec<(String, Timeframe)> = Vec::new();
        for symbol in self.mtf_grid_navbar_symbols() {
            let key = mtf_grid_symbol_key(&symbol);
            for &(_label, tf) in &MTF_GRID_TIMEFRAMES {
                // Never load a timeframe that's disabled in Sync (e.g. M1/M5) — that
                // was the source of the "No chart data found for …:1Min" spam and the
                // wasted probes for data that does not exist.
                if !self.enabled_sync_timeframes.contains(tf.cache_suffix()) {
                    continue;
                }
                let has_tab = self.charts.iter().any(|c| {
                    c.show_in_tab_bar
                        && !c.bars.is_empty()
                        && c.timeframe == tf
                        && mtf_grid_symbol_key(&c.symbol).eq_ignore_ascii_case(&key)
                });
                // Skip cells with a live tab (read live in the render) and cells whose
                // dot value is still fresh in the sticky grid store. This gates on the
                // 1h value store — what the navbar dots actually read — NOT the 90s
                // bars/result cache. Gating on the 90s cache made the fill re-load
                // EVERY grid cell's full bar set every 90s: a continuous decompress +
                // merge + indicator treadmill on the worker pool that churned gigabytes
                // of RSS and held the cache lock against the render thread (the
                // recurring ~150ms render stalls). The dots are slow higher-timeframe
                // indicators, so an hourly refresh (or an on-demand reload when the user
                // opens that chart) is plenty.
                if has_tab
                    || super::chart::mtf_grid_value_get(&key, tf.cache_suffix(), now_ms).is_some()
                {
                    continue;
                }
                cells.push((symbol.clone(), tf));
            }
        }
        if cells.is_empty() {
            return;
        }
        // Active symbol's cells first so the focused row fills immediately; cap only
        // pathological cases. A normal open grid (e.g. 15 symbols × 7 TFs) should
        // finish in one pass, not over repeated six-second windows.
        cells.sort_by_key(|(s, _)| mtf_grid_symbol_key(s).to_ascii_uppercase() != active_key);
        cells.truncate(MTF_GRID_FILL_PER_BATCH);
        let (tx, rx) = std::sync::mpsc::channel();
        let rt_handle = self.rt_handle.clone();
        rt_handle.spawn_blocking(move || {
            for (symbol, tf) in cells {
                let mut temp = ChartState::new(&symbol, tf);
                let dsm = typhoon_engine::core::data_source::DataSourceManager::default();
                temp.load(&cache, &mut std::collections::VecDeque::new(), None, &dsm);
                if temp.bars.is_empty() {
                    continue;
                }
                let now_ms = chrono::Utc::now().timestamp_millis();
                let key = mtf_grid_symbol_key(&symbol);
                let close = temp.bars.last().map(|b| b.close);
                let sma = temp.sma200.last().and_then(|v| *v);
                let kama = temp.kama.last().and_then(|v| *v);
                let fisher = temp.fisher.last().and_then(|v| *v);
                let fsig = temp.fisher_signal.last().and_then(|v| *v);
                let source = temp.primary_source;
                let bars = std::sync::Arc::new(std::mem::take(&mut temp.bars));
                // Bars → shared HTF cache (overlays reuse them) and the reopen cache;
                // values → the sticky grid store the navbar reads.
                super::chart::mtf_htf_cache_put(
                    &key,
                    tf.cache_suffix(),
                    std::sync::Arc::clone(&bars),
                    now_ms,
                );
                super::chart::chart_result_cache_put(&key, tf.cache_suffix(), bars, source, now_ms);
                super::chart::mtf_grid_value_put(
                    &key,
                    tf.cache_suffix(),
                    (close, sma, kama, fisher, fsig),
                    now_ms,
                );
            }
            let _ = tx.send(());
        });
        self.mtf_grid_rx = Some(rx);
    }

    /// Return the unique news tickers represented by the current MTF grid charts.
    /// The grid may contain the same ticker across many timeframes/source-prefixed
    /// cache keys; news fetches should happen once per underlying ticker.
    pub(super) fn mtf_grid_news_symbols(&self) -> Vec<String> {
        fn is_timeframe_token(token: &str) -> bool {
            matches!(
                token.to_ascii_uppercase().as_str(),
                "M1" | "M5"
                    | "M15"
                    | "M30"
                    | "H1"
                    | "H4"
                    | "D1"
                    | "W1"
                    | "MN1"
                    | "1MIN"
                    | "5MIN"
                    | "15MIN"
                    | "30MIN"
                    | "1HOUR"
                    | "4HOUR"
                    | "1DAY"
                    | "1WEEK"
                    | "1MONTH"
            )
        }

        let suppressed_symbols = low_timeframe_no_data_symbols(&self.unresolvable_pairs);
        let mut symbols = std::collections::BTreeSet::new();
        for (i, chart) in self.charts.iter().enumerate() {
            if self.mtf_enabled && !self.mtf_visible.get(i).copied().unwrap_or(true) {
                continue;
            }
            let parts: Vec<&str> = chart.symbol.split(':').collect();
            let candidate = match parts.as_slice() {
                [source, sym] if !is_timeframe_token(sym) && !source.eq_ignore_ascii_case(sym) => {
                    *sym
                }
                [sym, tf] if is_timeframe_token(tf) => *sym,
                [_, sym, _tf] => *sym,
                _ => chart.symbol.as_str(),
            };
            let mut symbol = normalize_market_data_symbol(candidate).replace('/', "");
            if let Some(stripped) = symbol.strip_suffix(".EQ") {
                symbol = stripped.to_string();
            }
            if !symbol.is_empty()
                && !is_timeframe_token(&symbol)
                && !suppressed_symbols.contains(&symbol.to_ascii_uppercase())
            {
                symbols.insert(symbol);
            }
        }
        symbols.into_iter().collect()
    }

    /// Ensure this symbol has one MTF chart per supported MTF Grid timeframe.
    /// M1/M5 stay visible for native Kraken Spot and Kraken Equities; unsupported/missing assist providers render as empty/grey panes.
    ///
    /// `focus_active_d1` moves the active tab to this symbol's D1 chart (used when
    /// the user explicitly enters the MTF grid). Passive callers — the navbar
    /// pre-population sweep that runs in every charting mode — pass `false` so
    /// back-filling a symbol's hidden timeframes never hijacks the focused tab.
    pub(super) fn ensure_mtf_grid_for_symbol(&mut self, symbol: &str, focus_active_d1: bool) {
        let symbol = symbol.trim();
        if symbol.is_empty() {
            return;
        }
        let symbol_key = normalize_market_data_symbol(symbol)
            .replace('/', "")
            .trim_end_matches(".EQ")
            .to_ascii_uppercase();
        if low_timeframe_no_data_symbols(&self.unresolvable_pairs).contains(&symbol_key) {
            return;
        }
        let mut existing_chart_by_tf: std::collections::HashMap<Timeframe, usize> =
            std::collections::HashMap::new();
        let mut empty_low_timeframes: std::collections::HashSet<Timeframe> =
            std::collections::HashSet::new();
        for (idx, chart) in self.charts.iter().enumerate() {
            if !mtf_grid_symbol_key(&chart.symbol).eq_ignore_ascii_case(&symbol_key) {
                continue;
            }
            existing_chart_by_tf.entry(chart.timeframe).or_insert(idx);
            if mtf_empty_low_timeframe_backing_chart(chart) {
                empty_low_timeframes.insert(chart.timeframe);
            }
        }
        for &(label, tf) in &MTF_GRID_TIMEFRAMES {
            if empty_low_timeframes.contains(&tf) {
                continue;
            }
            let existing_idx = existing_chart_by_tf.get(&tf).copied();
            let idx = if let Some(idx) = existing_idx {
                idx
            } else {
                // Push the cell empty and load it off the render thread via the
                // deferred loader (+ a fetch in case the cache has no rows yet).
                // Synchronously try_load-ing every timeframe here froze the UI when
                // opening a symbol's full MTF grid (~7 cold loads on the render thread).
                let mut chart = ChartState::new(symbol, tf);
                chart.show_in_tab_bar = false;
                self.charts.push(chart);
                let idx = self.charts.len().saturating_sub(1);
                self.rebuild_live_indices();
                self.queue_chart_reload(idx);
                let _ = self.queue_symbol_fetch_for_source(symbol, tf.cache_suffix());
                idx
            };
            while self.mtf_visible.len() < self.charts.len() {
                self.mtf_visible.push(true);
            }
            if let Some(visible) = self.mtf_visible.get_mut(idx) {
                *visible = true;
            }
            if focus_active_d1
                && label == "D1"
                && self
                    .charts
                    .get(idx)
                    .is_some_and(|chart| chart.show_in_tab_bar)
            {
                self.active_tab = idx;
            }
        }
    }

    /// Build trade overlay for a chart: broker fills as arrows + open position lines.
    /// Aggregates same-price entries at same bar into single markers.
    pub(super) fn build_trade_overlay(&self, chart: &ChartState) -> TradeOverlay {
        let mut overlay = TradeOverlay::default();
        if chart.bars.is_empty() {
            return overlay;
        }

        // Extract bare symbol from chart symbol for matching
        // Normalize: strip source prefix, TF suffix, and slashes (SOL/USD → SOLUSD)
        let bare_sym = {
            let s = &chart.symbol;
            let parts: Vec<&str> = s.split(':').collect();
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
            sym_parts
                .last()
                .copied()
                .unwrap_or(s.as_str())
                .replace('/', "")
        };
        if bare_sym.is_empty() {
            return overlay;
        }

        let first_ts = chart.bars.first().map(|b| b.ts_ms).unwrap_or(0);
        let last_ts = chart.bars.last().map(|b| b.ts_ms).unwrap_or(0);

        // Tolerance based on timeframe (a deal can be slightly after the last bar)
        let tf_tolerance_ms: i64 = match chart.timeframe {
            Timeframe::MN1 => 35 * 86_400_000, // 35 days
            Timeframe::W1 => 8 * 86_400_000,   // 8 days
            Timeframe::D1 => 2 * 86_400_000,   // 2 days
            _ => 86_400_000,                   // 1 day
        };
        // Find bar index for a timestamp (binary search on sorted bars)
        let find_bar = |ts_ms: i64| -> Option<usize> {
            if ts_ms < first_ts || ts_ms > last_ts + tf_tolerance_ms {
                return None;
            }
            match chart.bars.binary_search_by_key(&ts_ms, |b| b.ts_ms) {
                Ok(idx) => Some(idx),
                Err(idx) => {
                    if idx > 0 {
                        Some(idx - 1)
                    } else {
                        Some(0)
                    }
                }
            }
        };

        // Trade-marker accumulator shared by broker fills below.
        use std::collections::HashMap;
        let bare_upper = bare_sym.to_uppercase();
        let mut marker_map: HashMap<(usize, bool, i64), (f64, u32, String)> = HashMap::new(); // (bar_idx, is_buy, price_cents) → (total_vol, count, ticker)

        // Broker fills — add to marker map before conversion. Alpaca currently
        // enters through `recent_fills`; Kraken keeps a full REST + private-WS
        // trade deque. Both paths feed the same chart arrows.
        if self.show_alpaca_positions {
            for (sym, side, qty, price, time) in &self.recent_fills {
                let fill_sym = sym.replace('/', "").to_uppercase();
                if !fill_sym.contains(&bare_upper) && !bare_upper.contains(&fill_sym) {
                    continue;
                }
                // Alpaca `transaction_time` is RFC3339 with either a `Z` or a
                // numeric offset (e.g. `-04:00`); the old fixed `…Z` pattern
                // silently dropped offset-form fills → ts=0 → no chart arrow.
                let ts = chrono::DateTime::parse_from_rfc3339(time)
                    .map(|dt| dt.timestamp_millis())
                    .or_else(|_| {
                        chrono::NaiveDateTime::parse_from_str(time, "%Y-%m-%dT%H:%M:%S%.fZ")
                            .or_else(|_| {
                                chrono::NaiveDateTime::parse_from_str(time, "%Y-%m-%d %H:%M:%S")
                            })
                            .or_else(|_| {
                                chrono::NaiveDate::parse_from_str(time, "%Y-%m-%d")
                                    .map(|d| d.and_hms_opt(0, 0, 0).unwrap_or_default())
                            })
                            .map(|dt| dt.and_utc().timestamp_millis())
                    })
                    .unwrap_or(0);
                if let Some(bar_idx) = find_bar(ts) {
                    let is_buy = side == "buy";
                    let price_key = (*price * 100000.0) as i64;
                    let entry = marker_map.entry((bar_idx, is_buy, price_key)).or_insert((
                        0.0,
                        0,
                        String::new(),
                    ));
                    entry.0 += qty;
                    entry.1 += 1;
                    if !entry.2.contains("Alpaca") {
                        if !entry.2.is_empty() {
                            entry.2.push_str(", ");
                        }
                        entry.2.push_str("Alpaca");
                    }
                }
            }
        }

        if self.show_kr_positions {
            for trade in &self.kraken_trades {
                let pair_norm = typhoon_engine::core::kraken::normalize_pair_symbol(&trade.pair)
                    .replace('/', "")
                    .to_ascii_uppercase();
                let base = Self::kraken_base_asset_for_pair(&pair_norm);
                let matches_chart = symbol_matches_no_alloc(&pair_norm, &bare_upper)
                    || Self::kraken_asset_keys_match(&base, &bare_upper)
                    || pair_norm.contains(&bare_upper)
                    || bare_upper.contains(&pair_norm);
                if !matches_chart || trade.price <= 0.0 || trade.vol <= 0.0 {
                    continue;
                }
                let ts = (trade.time * 1000.0) as i64;
                if let Some(bar_idx) = find_bar(ts) {
                    let is_buy = trade.side.eq_ignore_ascii_case("buy");
                    let price_key = (trade.price * 100000.0) as i64;
                    let entry = marker_map.entry((bar_idx, is_buy, price_key)).or_insert((
                        0.0,
                        0,
                        String::new(),
                    ));
                    entry.0 += trade.vol;
                    entry.1 += 1;
                    if !entry.2.contains("Kraken") {
                        if !entry.2.is_empty() {
                            entry.2.push_str(", ");
                        }
                        entry.2.push_str("Kraken");
                    }
                }
            }
        }

        // SEC insider trades (Form 4) — show buy/sell markers on chart
        if let Some(trades) = self.bg.insider_trades.get(&bare_upper) {
            for trade in trades {
                // Parse "YYYY-MM-DD" to timestamp
                let ts = chrono::NaiveDate::parse_from_str(&trade.transaction_date, "%Y-%m-%d")
                    .map(|d| {
                        d.and_hms_opt(12, 0, 0)
                            .unwrap_or_default()
                            .and_utc()
                            .timestamp_millis()
                    })
                    .unwrap_or(0);
                if let Some(bar_idx) = find_bar(ts) {
                    let is_buy =
                        !matches!(trade.transaction_type.chars().next(), Some('S') | Some('D'));
                    // Use the bar's close price as the marker price (insider trade price may not match chart scale)
                    let price = if let Some(bar) = chart.bars.get(bar_idx) {
                        bar.close
                    } else {
                        trade.price
                    };
                    let price_key = (price * 100000.0) as i64;
                    let label = format!(
                        "SEC:{}",
                        trade.insider_name.split_whitespace().next().unwrap_or("")
                    );
                    let entry = marker_map.entry((bar_idx, is_buy, price_key)).or_insert((
                        0.0,
                        0,
                        String::new(),
                    ));
                    entry.0 += trade.shares;
                    entry.1 += 1;
                    if !entry.2.is_empty() {
                        entry.2.push_str(", ");
                    }
                    entry.2.push_str(&label);
                }
            }
        }

        // Convert marker map to sorted markers
        for ((bar_idx, is_buy, price_key), (volume, count, ticker)) in marker_map {
            overlay.markers.push(TradeMarker {
                bar_idx,
                price: price_key as f64 / 100000.0,
                volume,
                is_buy,
                count,
                ticker,
            });
        }
        overlay.markers.sort_by_key(|m| m.bar_idx);

        // Live Alpaca working order lines. Alpaca `nested=true` includes
        // bracket/OCO child legs; flatten them so fixed-price exits show too.
        // Market/trailing orders have no fixed chart price and are skipped.
        if self.show_alpaca_positions {
            let current_price = chart
                .fresh_live_quote_mid()
                .or_else(|| chart.bars.last().map(|bar| bar.close))
                .unwrap_or(0.0);
            let tick_size = self.trade_symbol_spec(&bare_upper, current_price).tick_size;
            let account_balance = self
                .live_account
                .as_ref()
                .map(Self::alpaca_current_risk_balance)
                .filter(|balance| balance.is_finite() && *balance > 0.0);
            collect_alpaca_order_lines_for_symbol(
                &self.live_orders,
                &bare_upper,
                current_price,
                tick_size,
                account_balance,
                &mut overlay.order_lines,
            );
        }

        if self.show_kr_positions {
            let current_price = chart
                .fresh_live_quote_mid()
                .or_else(|| chart.bars.last().map(|bar| bar.close))
                .unwrap_or(0.0);
            let tick_size = self.trade_symbol_spec(&bare_upper, current_price).tick_size;
            let account_balance = self
                .kraken_trade_account_snapshot()
                .map(|snap| snap.balance)
                .filter(|balance| balance.is_finite() && *balance > 0.0);
            collect_kraken_order_lines_for_symbol(
                &self.kraken_open_orders,
                &bare_upper,
                current_price,
                tick_size,
                account_balance,
                &mut overlay.order_lines,
            );
        }

        // Live broker position lines (Alpaca + Kraken).
        // Kraken spot crypto balances are inventory rather than broker
        // `PositionInfo` rows, but the chart still needs a visible holding
        // entry line when cost basis is known.
        let alpaca_iter: Box<dyn Iterator<Item = &PositionInfo>> = if self.show_alpaca_positions {
            Box::new(self.live_positions.iter())
        } else {
            Box::new(std::iter::empty())
        };
        let kr_iter: Box<dyn Iterator<Item = &PositionInfo>> = if self.show_kr_positions {
            Box::new(self.kr_positions.iter())
        } else {
            Box::new(std::iter::empty())
        };
        let all_broker_positions = alpaca_iter.chain(kr_iter);
        // `bare_upper` is already computed once at the top of this function;
        // recomputing it inside the loop allocated a new String per broker position.
        // Short-circuit on a no-alloc equality check before paying for the substring
        // form — most positions match exactly, and only the rare crypto-style
        // `BTCUSD` vs `BTC` case actually needs the normalized String.
        for pos in all_broker_positions {
            let keep = if symbol_matches_no_alloc(&pos.symbol, &bare_upper) {
                true
            } else {
                let pos_sym = pos.symbol.replace('/', "").to_uppercase();
                pos_sym.contains(&bare_upper) || bare_upper.contains(&pos_sym)
            };
            if !keep {
                continue;
            }
            let is_buy = pos.side == "long";
            // Kraken xStock positions are derived from cash-account balances and
            // arrive with avg_entry_price = 0.0 (the balance snapshot carries no
            // cost basis). Resolve the real entry from trade-history cost basis —
            // the same source the positions panel uses (kraken_balance_avg_price)
            // — so the line sits at the entry instead of being pinned to price 0
            // (the chart bottom), which is the BUY 0.0000 regression.
            let entry_price = if pos.avg_entry_price > 0.0 {
                pos.avg_entry_price
            } else if let Some(asset) = pos.asset_id.strip_prefix("equity_balance:") {
                self.kraken_balance_avg_price(asset).unwrap_or(0.0)
            } else {
                pos.avg_entry_price
            };
            if !(entry_price > 0.0 && entry_price.is_finite()) {
                // No usable entry price (cost basis not loaded yet) — skip rather
                // than draw a meaningless dashed line pinned to price 0.
                continue;
            }
            overlay.position_lines.push(PositionLine {
                price: entry_price,
                volume: pos.qty,
                is_buy,
                line_type: 0, // entry
            });
        }

        if self.show_kr_positions {
            for (asset, qty) in &self.kraken_balances {
                if !qty.is_finite() || *qty <= 0.0 || Self::kraken_is_cash_balance_asset(asset) {
                    continue;
                }
                if kraken_position_covers_balance_asset(&self.kr_positions, asset) {
                    continue;
                }
                let display = Self::kraken_display_asset(asset);
                let pair = Self::kraken_spot_pair_for_balance_asset(asset);
                let pair_norm = typhoon_engine::core::kraken::normalize_pair_symbol(&pair)
                    .replace('/', "")
                    .to_ascii_uppercase();
                let base = Self::kraken_base_asset_for_pair(&pair_norm);
                let matches_chart = Self::kraken_asset_keys_match(&display, &bare_upper)
                    || Self::kraken_asset_keys_match(&base, &bare_upper)
                    || symbol_matches_no_alloc(&pair_norm, &bare_upper)
                    || pair_norm.contains(&bare_upper)
                    || bare_upper.contains(&pair_norm);
                if !matches_chart {
                    continue;
                }
                let Some(avg_price) = self.kraken_balance_avg_price(asset) else {
                    continue;
                };
                if avg_price <= 0.0 || !avg_price.is_finite() {
                    continue;
                }
                overlay.position_lines.push(PositionLine {
                    price: avg_price,
                    volume: *qty,
                    is_buy: true,
                    line_type: 0,
                });
            }
        }

        // Deduplicate position lines (aggregate same price+type, sum volume).
        // Bucket by a rounded 5-decimal price key, but keep the EXACT first
        // price for display. The positions panel formats the raw avg, so a
        // truncated bucket value (pk/100000) could round differently at the
        // 4th decimal and make the on-chart label disagree with the navbar
        // (e.g. 0.1057 vs 0.1058). Preserving the real price keeps them in sync.
        {
            let mut agg: HashMap<(i64, u8), (f64, f64, bool)> = HashMap::new();
            for pl in &overlay.position_lines {
                let key = ((pl.price * 100000.0).round() as i64, pl.line_type);
                let entry = agg.entry(key).or_insert((pl.price, 0.0, pl.is_buy));
                entry.1 += pl.volume;
            }
            overlay.position_lines = agg
                .into_iter()
                .map(|((_, lt), (price, vol, is_buy))| PositionLine {
                    price,
                    volume: vol,
                    is_buy,
                    line_type: lt,
                })
                .collect();
        }

        overlay
    }

    pub(super) fn indicator_flags(&self) -> IndicatorFlags {
        IndicatorFlags {
            sma200: self.show_sma200,
            sma100: self.show_sma100,
            kama: self.show_kama,
            ema21: self.show_ema21,
            bollinger: self.show_bollinger,
            ichimoku: self.show_ichimoku,
            wma: self.show_wma,
            hma: self.show_hma,
            psar: self.show_psar,
            atr_proj: self.show_atr_proj,
            prev_levels: self.show_prev_levels,
            pivots: self.show_pivots,
            fractals: self.show_fractals,
            harmonics: self.show_harmonics,
            auto_fib: self.show_auto_fib,
            supply_demand: self.show_supply_demand,
            ehlers_ss: self.show_ehlers_ss,
            ehlers_decycler: self.show_ehlers_decycler,
            ehlers_itl: self.show_ehlers_itl,
            ehlers_mama: self.show_ehlers_mama,
            sessions: self.show_sessions,
            vol_heatmap: self.show_vol_heatmap,
            vwap: self.show_vwap,
            price_histogram: self.show_price_histogram,
            supertrend: self.show_supertrend,
            donchian: self.show_donchian,
            keltner: self.show_keltner,
            regression: self.show_regression,
            fvg: self.show_fvg,
            order_blocks: self.show_order_blocks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_order(
        symbol: &str,
        side: &str,
        qty: &str,
        filled: &str,
        limit: Option<&str>,
        status: &str,
    ) -> OrderInfo {
        OrderInfo {
            id: format!("{symbol}-{side}"),
            symbol: symbol.to_string(),
            qty: qty.to_string(),
            filled_qty: filled.to_string(),
            side: side.to_string(),
            order_type: "limit".to_string(),
            order_class: None,
            status: status.to_string(),
            limit_price: limit.map(str::to_string),
            stop_price: None,
            trail_price: None,
            trail_percent: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            filled_at: None,
            filled_avg_price: None,
            legs: None,
        }
    }

    #[test]
    fn alpaca_order_lines_use_open_qty_signed_notional_pct_and_pips() {
        let orders = vec![test_order("AAPL", "buy", "10", "2", Some("99.50"), "new")];
        let mut lines = Vec::new();

        collect_alpaca_order_lines_for_symbol(
            &orders,
            "AAPL",
            100.0,
            0.01,
            Some(10_000.0),
            &mut lines,
        );

        assert_eq!(lines.len(), 1);
        let line = &lines[0];
        assert!(line.is_buy);
        assert_eq!(line.qty, 8.0);
        assert_eq!(line.price, 99.50);
        assert!((line.notional_delta + 796.0).abs() < 1e-9);
        assert!((line.account_pct_delta.unwrap() + 7.96).abs() < 1e-9);
        assert!((line.pips_from_current.unwrap() + 50.0).abs() < 1e-9);
    }

    #[test]
    fn alpaca_order_lines_merge_same_side_source_and_price() {
        let orders = vec![
            test_order("AAPL", "sell", "10", "0", Some("105.00"), "new"),
            test_order("AAPL", "sell", "15", "5", Some("105.00"), "new"),
        ];
        let mut lines = Vec::new();

        collect_alpaca_order_lines_for_symbol(
            &orders,
            "AAPL",
            100.0,
            0.01,
            Some(10_000.0),
            &mut lines,
        );

        assert_eq!(lines.len(), 1);
        let line = &lines[0];
        assert!(!line.is_buy);
        assert_eq!(line.qty, 20.0);
        assert_eq!(line.price, 105.0);
        assert!((line.notional_delta - 2100.0).abs() < 1e-9);
        assert!((line.account_pct_delta.unwrap() - 21.0).abs() < 1e-9);
    }

    #[test]
    fn alpaca_order_lines_flatten_nested_working_legs_and_skip_filled_parent() {
        let mut parent = test_order("SPY", "buy", "1", "1", Some("470"), "filled");
        parent.legs = Some(vec![test_order(
            "SPY",
            "sell",
            "1",
            "0",
            Some("480"),
            "new",
        )]);
        let mut lines = Vec::new();

        collect_alpaca_order_lines_for_symbol(
            &[parent],
            "SPY",
            475.0,
            0.01,
            Some(20_000.0),
            &mut lines,
        );

        assert_eq!(lines.len(), 1);
        assert!(!lines[0].is_buy);
        assert_eq!(lines[0].price, 480.0);
        assert!((lines[0].notional_delta - 480.0).abs() < 1e-9);
        assert!((lines[0].pips_from_current.unwrap() - 500.0).abs() < 1e-9);
    }

    #[test]
    fn news_article_tickers_normalizes_and_deduplicates_symbols() {
        let tickers = vec![
            " aapl ".to_string(),
            "MSFT".to_string(),
            "msft".to_string(),
            "../bad".to_string(),
            "THIS-SYMBOL-IS-TOO-LONG".to_string(),
        ];

        assert_eq!(
            TyphooNApp::news_article_tickers("AAPL", &tickers),
            vec!["AAPL".to_string(), "MSFT".to_string()]
        );
    }

    #[test]
    fn news_ticker_normalization_accepts_common_market_symbols() {
        assert_eq!(
            TyphooNApp::normalize_news_ticker_for_chart(" brk.b "),
            Some("BRK.B".to_string())
        );
        assert_eq!(
            TyphooNApp::normalize_news_ticker_for_chart("BTC/USD"),
            Some("BTC/USD".to_string())
        );
        assert_eq!(TyphooNApp::normalize_news_ticker_for_chart(""), None);
        assert_eq!(
            TyphooNApp::normalize_news_ticker_for_chart("THIS-SYMBOL-IS-TOO-LONG"),
            None
        );
    }

    #[test]
    fn mtf_grid_timeframes_include_low_timeframes_for_native_kraken_pairs() {
        let labels: Vec<&str> = MTF_GRID_TIMEFRAMES
            .iter()
            .map(|(label, _)| *label)
            .collect();

        assert_eq!(
            labels,
            vec!["M1", "M5", "M15", "M30", "H1", "H4", "D1", "W1", "MN1"]
        );
    }

    #[test]
    fn mtf_grid_groups_visible_charts_by_symbol_and_sorts_each_symbol_by_timeframe() {
        let charts = vec![
            ChartState::new("kraken:WOK.EQ:1Day", Timeframe::D1),
            ChartState::new("kraken:BABYUSD:4Hour", Timeframe::H4),
            ChartState::new("kraken:WOK.EQ:15Min", Timeframe::M15),
            ChartState::new("kraken:BABYUSD:1Hour", Timeframe::H1),
            ChartState::new("kraken:WOK.EQ:1Min", Timeframe::M1),
            ChartState::new("kraken:WOK.EQ:1Week", Timeframe::W1),
            ChartState::new("kraken:BABYUSD:5Min", Timeframe::M5),
        ];
        let visible = vec![true; charts.len()];

        let groups = mtf_visible_chart_groups(&charts, &visible);

        // Groups are sorted alphabetically by symbol (BABYUSD before WOK), and each
        // group's indices are ordered by ascending timeframe rank.
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].symbol, "BABYUSD");
        assert_eq!(groups[0].indices, vec![6, 3, 1]);
        assert_eq!(groups[1].symbol, "WOK");
        assert_eq!(groups[1].indices, vec![4, 2, 0, 5]);
    }

    #[test]
    fn mtf_grid_omits_empty_low_timeframe_cells() {
        let mut m15 = ChartState::new("AMC", Timeframe::M15);
        m15.bars.push(Bar {
            ts_ms: 1,
            open: 1.0,
            high: 1.0,
            low: 1.0,
            close: 1.0,
            volume: 1.0,
        });
        let mut h1 = ChartState::new("AMC", Timeframe::H1);
        h1.bars.push(Bar {
            ts_ms: 1,
            open: 1.0,
            high: 1.0,
            low: 1.0,
            close: 1.0,
            volume: 1.0,
        });
        let mut m1 = ChartState::new("AMC", Timeframe::M1);
        m1.show_in_tab_bar = false;
        let mut m5 = ChartState::new("AMC", Timeframe::M5);
        m5.show_in_tab_bar = false;
        let charts = vec![m1, m5, m15, h1];
        let visible = vec![true; charts.len()];

        let groups = mtf_visible_chart_groups(&charts, &visible);

        assert_eq!(groups.len(), 1);
        // Empty M1/M5 backing charts (indices 0,1) are excluded; only the loaded
        // M15/H1 tabs (indices 2,3) remain.
        assert_eq!(groups[0].indices, vec![2, 3]);
    }

    #[test]
    fn mtf_chart_canvas_uses_flat_two_column_flow() {
        let charts = vec![
            ChartState::new("AMC", Timeframe::M15),
            ChartState::new("AMC", Timeframe::M30),
            ChartState::new("AMC", Timeframe::H1),
            ChartState::new("AVAT", Timeframe::M15),
            ChartState::new("AVAT", Timeframe::M30),
        ];
        let visible = vec![true; charts.len()];
        let groups = mtf_visible_chart_groups(&charts, &visible);

        assert_eq!(mtf_flat_chart_indices(&groups), vec![0, 1, 2, 3, 4]);
        assert_eq!(mtf_canvas_grid_cols(1), 2);
        assert_eq!(mtf_canvas_grid_cols(5), 2);
        assert_eq!(mtf_canvas_grid_rows(5), 3);
    }

    #[test]
    fn mtf_grid_suppresses_symbol_when_broker_has_no_m1_or_m5_bars() {
        let charts = vec![
            ChartState::new("CC", Timeframe::D1),
            ChartState::new("CC", Timeframe::H4),
            ChartState::new("WEN", Timeframe::D1),
        ];
        let visible = vec![true; charts.len()];
        let no_low_tf_symbols = ["CC".to_string()].into_iter().collect();

        let groups = mtf_visible_chart_groups_filtered(&charts, &visible, &no_low_tf_symbols);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].symbol, "WEN");
    }

    #[test]
    fn low_timeframe_no_data_symbols_require_m1_and_m5_from_same_broker() {
        let mut pairs = std::collections::HashMap::new();
        pairs.insert(
            "m1".to_string(),
            UnresolvablePair {
                broker: "kraken-equities".to_string(),
                symbol: "CC".to_string(),
                timeframe: "1Min".to_string(),
                reason: "provider returned no bars".to_string(),
                ts: 1,
            },
        );
        pairs.insert(
            "m5".to_string(),
            UnresolvablePair {
                broker: "kraken-equities".to_string(),
                symbol: "CC.EQ".to_string(),
                timeframe: "5Min".to_string(),
                reason: "provider returned no data".to_string(),
                ts: 2,
            },
        );
        pairs.insert(
            "wen_m1".to_string(),
            UnresolvablePair {
                broker: "kraken-equities".to_string(),
                symbol: "WEN".to_string(),
                timeframe: "1Min".to_string(),
                reason: "provider returned no bars".to_string(),
                ts: 3,
            },
        );

        let suppressed = low_timeframe_no_data_symbols(&pairs);

        assert!(suppressed.contains("CC"));
        assert!(!suppressed.contains("WEN"));
    }

    #[test]
    fn open_chart_preload_indices_include_inactive_empty_tabs() {
        let mut loaded = ChartState::new("CC", Timeframe::D1);
        loaded.bars.push(Bar {
            ts_ms: 1,
            open: 10.0,
            high: 10.0,
            low: 10.0,
            close: 10.0,
            volume: 1.0,
        });
        let mut hidden_low_tf_backing = ChartState::new("CC", Timeframe::M1);
        hidden_low_tf_backing.show_in_tab_bar = false;
        let charts = vec![
            loaded,
            ChartState::new("WEN", Timeframe::D1),
            ChartState::new("CC", Timeframe::H4),
            hidden_low_tf_backing,
        ];

        assert_eq!(open_chart_preload_indices(&charts), vec![1, 2]);
    }

    #[test]
    fn company_name_catalog_prefers_primary_broker_names() {
        let alpaca_assets = vec![
            (
                "CC".to_string(),
                "The Chemours Company".to_string(),
                "us_equity".to_string(),
            ),
            (
                "BTCUSD".to_string(),
                "Bitcoin".to_string(),
                "crypto".to_string(),
            ),
        ];
        let mut kraken_names = std::collections::HashMap::new();
        kraken_names.insert("CC".to_string(), "Kraken CC Placeholder".to_string());

        let alpaca_primary =
            chart_company_name_catalog(&alpaca_assets, &kraken_names, OrderBroker::Alpaca);
        assert_eq!(
            alpaca_primary.get("CC").map(String::as_str),
            Some("The Chemours Company")
        );
        assert!(!alpaca_primary.contains_key("BTCUSD"));

        let kraken_primary =
            chart_company_name_catalog(&alpaca_assets, &kraken_names, OrderBroker::Kraken);
        assert_eq!(
            kraken_primary.get("CC").map(String::as_str),
            Some("Kraken CC Placeholder")
        );
    }

    #[test]
    fn empty_chart_load_retry_is_backed_off_after_no_data_attempt() {
        let now = std::time::Instant::now();

        assert!(empty_chart_load_retry_due(None, now));
        assert!(!empty_chart_load_retry_due(
            Some(now - EMPTY_CHART_RELOAD_RETRY_AFTER / 2),
            now
        ));
        assert!(empty_chart_load_retry_due(
            Some(now - EMPTY_CHART_RELOAD_RETRY_AFTER),
            now
        ));
    }

    fn test_position(symbol: &str, qty: f64, side: &str) -> PositionInfo {
        PositionInfo {
            symbol: symbol.to_string(),
            qty,
            qty_available: qty,
            side: side.to_string(),
            avg_entry_price: 1.0,
            market_value: qty,
            unrealized_pl: 0.0,
            asset_class: "stock".to_string(),
            asset_id: "equity_balance:test".to_string(),
        }
    }

    #[test]
    fn kraken_balance_overlay_skips_assets_already_reported_as_positions() {
        let positions = vec![test_position("WOK", 7142.0, "long")];

        assert!(kraken_position_covers_balance_asset(&positions, "WOK.EQ"));
        assert!(kraken_position_covers_balance_asset(&positions, "WOK"));
    }

    #[test]
    fn kraken_balance_overlay_still_allows_inventory_without_position_row() {
        let positions = vec![test_position("GDC", 100.0, "long")];

        assert!(!kraken_position_covers_balance_asset(&positions, "WOK.EQ"));
        assert!(!kraken_position_covers_balance_asset(&positions, "WOK"));
    }
}
