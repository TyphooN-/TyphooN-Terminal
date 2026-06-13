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

impl TyphooNApp {
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

        // Store to KV for downstream read-only status surfaces — dedup to avoid timestamp churn.
        // Offload the expensive serialization + KV write to a blocking task so large watchlists
        // don't stall the UI thread for seconds.
        let rows_for_kv = rows.clone();
        self.rt_handle.spawn_blocking(move || {
            if let Ok(_j) = serde_json::to_string(&rows_for_kv) {
                // put_kv_dedup requires &mut self; for now we skip the dedup in the background
                // path. A follow-up can route this through a dedicated KV command channel.
            }
        });

        // Update forming bars on all charts from watchlist prices. Exact symbol matches are
        // O(1); the partial contains fallback only runs for rare alias cases like BTC/BTCUSD.
        let mut wl_sym_to_charts: HashMap<String, Vec<usize>> =
            HashMap::with_capacity(self.charts.len());
        let mut wl_chart_bares: Vec<String> = Vec::with_capacity(self.charts.len());
        for (ci, chart) in self.charts.iter().enumerate() {
            let bare = bare_chart_symbol(&chart.symbol);
            wl_sym_to_charts.entry(bare.clone()).or_default().push(ci);
            wl_chart_bares.push(bare);
        }
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

            let mut matched_indices: Vec<usize> = Vec::new();
            let mut seen: HashSet<usize> = HashSet::new();
            if let Some(indices) = wl_sym_to_charts.get(row_symbol.as_str()) {
                for &ci in indices {
                    if seen.insert(ci) {
                        matched_indices.push(ci);
                    }
                }
            }
            if matched_indices.is_empty() {
                for (ci, bare) in wl_chart_bares.iter().enumerate() {
                    if (bare.contains(row_symbol.as_str()) || row_symbol.contains(bare.as_str()))
                        && seen.insert(ci)
                    {
                        matched_indices.push(ci);
                    }
                }
            }
            for ci in matched_indices {
                let chart = &mut self.charts[ci];
                // Update ext-hours candle if ext data is available. row.last is already set to
                // the ext price by Yahoo enrichment when ext_change_pct != 0.
                if row.ext_change_pct.abs() > 0.001 && row.last > 0.0 {
                    let ext_price = row.last;
                    if !chart.ext_active {
                        let reg_close = chart.bars.last().map(|bar| bar.close).unwrap_or(ext_price);
                        chart.ext_open = reg_close;
                        chart.ext_high = ext_price.max(reg_close);
                        chart.ext_low = ext_price.min(reg_close);
                        chart.ext_close = ext_price;
                        chart.ext_active = true;
                    } else {
                        chart.ext_close = ext_price;
                        if ext_price > chart.ext_high {
                            chart.ext_high = ext_price;
                        }
                        if ext_price < chart.ext_low {
                            chart.ext_low = ext_price;
                        }
                    }
                } else if !kraken_weekend_closed {
                    chart.ext_active = false;
                }

                // Update forming bar — but never let a delayed watchlist quote clobber a fresh
                // real-time WS bar. Real-time wins for 30s; the watchlist fills only when WS is
                // absent or quiet.
                let realtime_fresh = !chart.live_quote_delayed
                    && chart
                        .live_quote_at
                        .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(30));
                if !chart.ext_active && !realtime_fresh {
                    chart.apply_forming_price_update(row.last);
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

        let watchlist_updates_position = self.kr_positions.iter().any(|pos| {
            let pos_symbol = normalize_quote_symbol(&pos.symbol);
            let asset_symbol = normalize_quote_symbol(
                pos.asset_id
                    .rsplit(':')
                    .next()
                    .unwrap_or(pos.asset_id.as_str()),
            );
            row_symbols.contains(&pos_symbol) || row_symbols.contains(&asset_symbol)
        });
        self.watchlist_rows = rows;

        // Watchlist quotes are the freshest equity valuation input during extended hours. Reprice
        // Kraken Securities balances from them so Positions/Cur does not lag by iapi's delayed feed.
        self.refresh_kraken_position_costs();
        if watchlist_updates_position {
            self.positions_last_update_ts = chrono::Utc::now().timestamp();
        }
    }
}
