use super::*;
use std::collections::{HashMap, HashSet};

fn normalize_quote_symbol(symbol: &str) -> String {
    let mut out = String::with_capacity(symbol.len());
    for b in symbol.bytes() {
        if b != b'/' {
            out.push(b.to_ascii_uppercase() as char);
        }
    }
    out.trim_end_matches(".EQ").to_string()
}

fn bare_chart_symbol(symbol: &str) -> String {
    let mut s = symbol.replace('/', "");
    s.make_ascii_uppercase();
    let mut it = s.rsplit(':');
    let last = it.next().unwrap_or("");
    let is_tf = matches!(
        last,
        "1MIN" | "5MIN" | "15MIN" | "30MIN" | "1HOUR" | "4HOUR" | "1DAY" | "1WEEK" | "1MONTH"
    );
    if is_tf {
        it.next()
            .unwrap_or(last)
            .trim_end_matches(".EQ")
            .to_string()
    } else {
        s.trim_end_matches(".EQ").to_string()
    }
}

fn apply_watchlist_row_to_chart(chart: &mut ChartState, row: &WatchlistRow, weekend_closed: bool) {
    if row.prev_close > 0.0 {
        chart.prev_daily_close = row.prev_close;
    }
    if row.ext_change_pct.abs() > 0.001 && row.last > 0.0 {
        let ext_price = row.last;
        if !chart.ext_active {
            let regular_close = if row.regular_close > 0.0 {
                row.regular_close
            } else {
                chart.bars.last().map(|bar| bar.close).unwrap_or(ext_price)
            };
            chart.ext_open = regular_close;
            chart.ext_high = ext_price.max(regular_close);
            chart.ext_low = ext_price.min(regular_close);
            chart.ext_close = ext_price;
            chart.ext_active = true;
        } else {
            chart.ext_close = ext_price;
            chart.ext_high = chart.ext_high.max(ext_price);
            chart.ext_low = chart.ext_low.min(ext_price);
        }
    } else if !weekend_closed {
        chart.ext_active = false;
    }

    let realtime_fresh = !chart.live_quote_delayed
        && chart
            .live_quote_at
            .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(30));
    if !chart.ext_active && !realtime_fresh {
        chart.apply_forming_price_update(row.last);
    }
}

impl TyphooNApp {
    pub(super) fn tick_watchlist_quote_refresh(&mut self, now_instant: std::time::Instant) {
        // Watchlist quotes used to be fetched only when the user manually added a
        // symbol, so a session-restored watchlist sat empty ("No cached data …
        // never") until poked. Refresh once on startup (auto_refresh_at == None)
        // and every 30s after. The GetWatchlistQuotes handler enriches from Yahoo
        // even with no broker connected, so this also works offline / on weekends.
        if self.cache_loaded && !self.user_watchlist.is_empty() {
            // Intraday: refresh every 30s. While the xStocks market is closed for the
            // weekend, watchlist quotes are static (no new prints), so stop re-polling
            // Yahoo every 30s — refresh only on a slow safety heartbeat or when the
            // watchlist set itself changes (symbol added/removed). Friday's last
            // after-hours snapshot is retained for display in the meantime.
            let interval = if super::app_runtime_support::kraken_xstocks_weekend_closed_now() {
                std::time::Duration::from_secs(300)
            } else {
                std::time::Duration::from_secs(30)
            };
            let watchlist_changed =
                self.watchlist_quotes_fetched_count != self.user_watchlist.len();
            let due = watchlist_changed
                || self
                    .watchlist_auto_refresh_at
                    .map(|t| now_instant.duration_since(t) >= interval)
                    .unwrap_or(true);
            if due {
                self.watchlist_auto_refresh_at = Some(now_instant);
                self.watchlist_quotes_fetched_count = self.user_watchlist.len();
                let _ = self.broker_tx.send(BrokerCmd::GetWatchlistQuotes {
                    symbols: self.user_watchlist.clone(),
                });
            }
        }
    }

    pub(super) fn handle_watchlist_quotes(&mut self, mut rows: Vec<WatchlistRow>) {
        // Weekend/off-hours quote providers can return empty/zero rows. Don't let a
        // failed refresh wipe useful cached rows already displayed in the watchlist.
        // Build an O(1) lookup once instead of scanning existing rows for every zero row.
        let existing_good: HashMap<String, WatchlistRow> = self
            .watchlist_rows
            .iter()
            .filter(|row| row.last > 0.0)
            .map(|row| (normalize_quote_symbol(&row.symbol), row.clone()))
            .collect();
        for row in &mut rows {
            if row.last <= 0.0 {
                if let Some(existing) = existing_good.get(&normalize_quote_symbol(&row.symbol)) {
                    *row = existing.clone();
                }
            }
        }

        self.watchlist_last_update_ts = chrono::Utc::now().timestamp();

        // Note: previous KV persist attempt here cloned rows + spawned blocking to_string,
        // but the actual put_kv_dedup was never called (commented). Removed to eliminate
        // unnecessary clone/alloc on every live watchlist quote update. Re-add via cmd
        // channel if real persistence needed.

        // Exact chart dispatch reuses the maintained O(1) index. The partial
        // contains fallback scans only for rare aliases such as BTC/BTCUSD.
        // During the xStocks weekend close, retain Friday's last extended-hours
        // snapshot instead of clearing it (Yahoo returns no extended change over the
        // weekend, which would otherwise flip ext_active off and drop the Ext% badge).
        let kraken_weekend_closed = super::app_runtime_support::kraken_xstocks_weekend_closed_now();

        if kraken_weekend_closed {
            for row in &mut rows {
                if row.ext_change_pct.abs() > 0.001 || row.last <= 0.0 {
                    continue;
                }
                if let Some(existing) = existing_good.get(&normalize_quote_symbol(&row.symbol)) {
                    if existing.ext_change_pct.abs() > 0.001 && existing.last > 0.0 {
                        *row = existing.clone();
                    }
                }
            }
        }

        let mut row_symbols: HashSet<String> = HashSet::with_capacity(rows.len());
        for row in &rows {
            if row.last <= 0.0 {
                continue;
            }
            let row_symbol = normalize_quote_symbol(&row.symbol);
            row_symbols.insert(row_symbol.clone());

            if let Some(indices) = self.chart_by_bare.get(&row_symbol) {
                for &chart_index in indices {
                    if let Some(chart) = self.charts.get_mut(chart_index) {
                        apply_watchlist_row_to_chart(chart, row, kraken_weekend_closed);
                    }
                }
            } else {
                for chart in &mut self.charts {
                    let bare = bare_chart_symbol(&chart.symbol);
                    if !bare.is_empty()
                        && (bare.contains(row_symbol.as_str())
                            || row_symbol.contains(bare.as_str()))
                    {
                        apply_watchlist_row_to_chart(chart, row, kraken_weekend_closed);
                    }
                }
            }
        }

        // Route to world indices / forex windows if open. The old disabled block rebuilt vectors
        // with per-row uppercase allocation; keep this path cheap and single-pass when visible.
        if self.show_world_indices || self.show_forex_matrix {
            static INDICES: std::sync::LazyLock<HashSet<&'static str>> =
                std::sync::LazyLock::new(|| {
                    [
                        "DIA", "SPY", "QQQ", "IWM", "EFA", "EEM", "VGK", "EWJ", "FXI", "EWZ",
                        "GLD", "SLV", "USO", "TLT", "UUP", "BTCUSD",
                    ]
                    .into_iter()
                    .collect()
                });
            static FOREX: std::sync::LazyLock<HashSet<&'static str>> =
                std::sync::LazyLock::new(|| {
                    [
                        "EURUSD", "GBPUSD", "USDJPY", "USDCHF", "AUDUSD", "NZDUSD", "USDCAD",
                        "EURGBP", "EURJPY", "GBPJPY",
                    ]
                    .into_iter()
                    .collect()
                });
            let mut idx_rows: Vec<WatchlistRow> = Vec::new();
            let mut fx_rows: Vec<WatchlistRow> = Vec::new();
            for row in &rows {
                let sym_upper = normalize_quote_symbol(&row.symbol);
                if self.show_world_indices && INDICES.contains(sym_upper.as_str()) {
                    idx_rows.push(row.clone());
                }
                if self.show_forex_matrix && FOREX.contains(sym_upper.as_str()) {
                    fx_rows.push(row.clone());
                }
            }
            if !idx_rows.is_empty() {
                self.world_indices_data = idx_rows;
            }
            if !fx_rows.is_empty() {
                self.forex_pairs_data = fx_rows;
            }
        }

        // Reuse the maintained O(1) position indices instead of cloning all map
        // keys and rebuilding normalized asset tails on every quote snapshot.
        let watchlist_updates_position = row_symbols.iter().any(|symbol| {
            self.kr_positions_by_symbol.contains_key(symbol)
                || self.kr_position_asset_tails.contains(symbol)
        });
        self.watchlist_rows = rows;
        self.rebuild_watchlist_live_index();

        // Watchlist quotes are the freshest equity valuation input during extended hours. Reprice
        // Kraken Securities balances from them so Positions/Cur does not lag by iapi's delayed feed.
        self.refresh_kraken_position_costs();
        if watchlist_updates_position {
            self.positions_last_update_ts = chrono::Utc::now().timestamp();
        }
    }
}
