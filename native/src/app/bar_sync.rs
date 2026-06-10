use super::BgData;
use super::sync_workset::normalize_sync_timeframe_key;

/// Per-(broker,TF) bar-sync health snapshot for the Sync Status window + the
/// compact Storage Manager banner. When Research / Backtest sync tallies land
/// later, a `category` discriminant plus a dedicated row type can be grafted
/// on — for now this struct is the whole surface so we don't leave any
/// unused scaffolding in the tree.
#[derive(Clone, Debug, Default)]
pub(super) struct SyncStatsRow {
    pub(super) broker: String, // "Kraken Spot" | "Kraken Equities" | "Alpaca" | …
    pub(super) tf: String,     // "1Min" | "1Hour" | "1Day" | …
    pub(super) total: u64,     // (sym,tf) pairs seen for this (broker,tf) bucket
    pub(super) healthy: u64,   // last bar lag < TF_period × 24
    pub(super) stale: u64,     // last bar lag ≥ threshold
    pub(super) empty: u64,     // cached blob has no bars (last_ms <= 0)
    pub(super) settled: u64,   // checked/exhausted provider window, counted healthy
    pub(super) note: Option<String>,
    pub(super) pct_healthy: f32, // 0..100
}

/// Aggregate `bar_ts_cache` into per-(broker,TF) rows. Always emits rows for
/// actively managed live bar sources (Kraken / Alpaca).
///
/// `is_backfill_complete(key)` distinguishes "stale because we haven't checked
/// lately" from "stale because the provider has no newer bar to give us." A
/// pair flagged backfill-complete (provider window saturated for Kraken,
/// historical depth exhausted for the others) stays in the Healthy bucket once
/// its cached bar ages past 24×TF — fetching it again can't move that anchor
/// forward, so counting it as Stale is a UI lie that paints the long tail of
/// illiquid pairs red on every weekend.
pub(super) fn compute_bar_sync_stats(
    detailed_stats: &[(String, i64, i64)],
    bar_ts_cache: &std::collections::HashMap<String, (i64, i64, i64)>,
    is_backfill_complete: &dyn Fn(&str) -> bool,
) -> Vec<SyncStatsRow> {
    use std::collections::BTreeMap;

    let period_ms: BTreeMap<&'static str, i64> = [
        ("1Min", 60_000i64),
        ("5Min", 300_000),
        ("15Min", 900_000),
        ("30Min", 1_800_000),
        ("1Hour", 3_600_000),
        ("4Hour", 14_400_000),
        ("1Day", 86_400_000),
        ("1Week", 604_800_000),
        ("1Month", 2_592_000_000),
    ]
    .into_iter()
    .collect();
    let tf_order = [
        "1Min", "5Min", "15Min", "30Min", "1Hour", "4Hour", "1Day", "1Week", "1Month",
    ];
    let broker_for_prefix = |p: &str| -> Option<&'static str> {
        match p {
            "alpaca" => Some("Alpaca"),
            "kraken" => Some("Kraken Spot"),
            "kraken-futures" => Some("Kraken Futures"),
            "kraken-equities" => Some("Kraken Equities"),
            "yahoo-chart" => Some("Yahoo"),
            _ => None,
        }
    };
    let required_brokers = ["Kraken Spot"];

    let mut groups: BTreeMap<(String, String), (u64, u64, u64, u64)> = BTreeMap::new();
    for broker in &required_brokers {
        for tf in &tf_order {
            groups
                .entry(((*broker).to_string(), (*tf).to_string()))
                .or_default();
        }
    }

    let now_ms = chrono::Utc::now().timestamp_millis();
    for (key, bar_count, write_ts_s) in detailed_stats {
        let mut parts = key.splitn(3, ':');
        let Some(prefix) = parts.next() else {
            continue;
        };
        let Some(symbol) = parts.next() else {
            continue;
        };
        let Some(raw_tf) = parts.next() else {
            continue;
        };
        let Some(broker) = broker_for_prefix(prefix) else {
            continue;
        };
        if symbol.is_empty() || symbol.starts_with("__") {
            continue;
        }
        let Some(tf) = normalize_sync_timeframe_key(raw_tf) else {
            continue;
        };
        if matches!(prefix, "alpaca" | "yahoo-chart") && matches!(tf, "1Min" | "5Min") {
            continue;
        }
        // Kraken-equities is WS-first for native bars through W1. Monthly is
        // constructed from D1 on the merged/chart path, so hide stale
        // `kraken-equities:*:1Month` legacy rows instead of treating them as
        // provider-native coverage.
        if prefix == "kraken-equities" && !super::kraken_equity_full_universe_timeframe(tf) {
            continue;
        }
        if matches!(prefix, "kraken" | "kraken-futures") && tf == "1Month" {
            continue;
        }
        let entry = groups
            .entry((broker.to_string(), tf.to_string()))
            .or_default();
        if *bar_count <= 0 {
            entry.2 += 1;
            continue;
        }
        let last_ms = bar_ts_cache
            .get(key)
            .map(|(_, last_ms, _)| *last_ms)
            .filter(|last_ms| *last_ms > 0)
            .unwrap_or_else(|| write_ts_s.saturating_mul(1000));
        if last_ms <= 0 {
            entry.2 += 1;
        } else if let Some(period) = period_ms.get(tf) {
            let write_ms = write_ts_s.saturating_mul(1000);
            let recently_checked = write_ms > 0 && now_ms - write_ms <= period * 24;
            let bar_aged_out = now_ms - last_ms > period * 24;
            if bar_aged_out && !recently_checked && !is_backfill_complete(key) {
                entry.1 += 1;
            } else if bar_aged_out && !recently_checked && is_backfill_complete(key) {
                entry.0 += 1;
                entry.3 += 1;
            } else {
                entry.0 += 1;
            }
        } else {
            entry.1 += 1;
        }
    }

    let mut rows: Vec<SyncStatsRow> = groups
        .into_iter()
        .map(|((broker, tf), (healthy, stale, empty, settled))| {
            let total = healthy + stale + empty;
            let pct_healthy = if total == 0 {
                0.0
            } else {
                (healthy as f32 / total as f32) * 100.0
            };
            SyncStatsRow {
                broker,
                tf,
                total,
                healthy,
                stale,
                empty,
                settled,
                note: None,
                pct_healthy,
            }
        })
        .collect();

    sort_sync_stats_rows(&mut rows);
    rows
}

pub(super) fn sort_sync_stats_rows(rows: &mut [SyncStatsRow]) {
    let tf_order = [
        "1Min", "5Min", "15Min", "30Min", "1Hour", "4Hour", "1Day", "1Week", "1Month",
    ];
    let broker_order = [
        "Kraken Spot",
        "Kraken Equities",
        "Kraken Futures",
        "Merged",
        "Alpaca",
        "Yahoo",
    ];
    rows.sort_by(|a, b| {
        let ab = broker_order
            .iter()
            .position(|broker| *broker == a.broker)
            .unwrap_or(usize::MAX);
        let bb = broker_order
            .iter()
            .position(|broker| *broker == b.broker)
            .unwrap_or(usize::MAX);
        let ai = tf_order
            .iter()
            .position(|tf| *tf == a.tf)
            .unwrap_or(usize::MAX);
        let bi = tf_order
            .iter()
            .position(|tf| *tf == b.tf)
            .unwrap_or(usize::MAX);
        ab.cmp(&bb).then(ai.cmp(&bi)).then(a.broker.cmp(&b.broker))
    });
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SyncStatsRowStatusCells {
    pub(super) symbols: String,
    pub(super) healthy: String,
    pub(super) stale_or_empty: String,
    pub(super) settled: String,
    pub(super) note: String,
}

pub(super) fn sync_stats_row_status_cells(row: &SyncStatsRow) -> SyncStatsRowStatusCells {
    SyncStatsRowStatusCells {
        symbols: row.total.to_string(),
        healthy: row.healthy.to_string(),
        stale_or_empty: (row.stale + row.empty).to_string(),
        settled: row.settled.to_string(),
        note: row.note.clone().unwrap_or_default(),
    }
}

/// Aggregate per-broker totals from a Vec<SyncStatsRow> for the compact
/// banner / one-liner display. Returns `(broker, total, healthy, pct)`
/// tuples in display order (Kraken, Alpaca, then any others).
pub(super) fn compute_bar_sync_broker_totals(
    rows: &[SyncStatsRow],
) -> Vec<(String, u64, u64, f32)> {
    use std::collections::BTreeMap;

    let mut totals: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for row in rows {
        if row.total == 0 {
            continue;
        }
        let entry = totals.entry(row.broker.clone()).or_default();
        entry.0 += row.total;
        entry.1 += row.healthy;
    }

    let order = [
        "Kraken Spot",
        "Kraken Equities",
        "Kraken Futures",
        "Merged",
        "Alpaca",
        "Yahoo",
    ];
    let mut out = Vec::new();
    for name in order {
        if let Some((total, healthy)) = totals.remove(name) {
            let pct = if total == 0 {
                0.0
            } else {
                (healthy as f32 / total as f32) * 100.0
            };
            out.push((name.to_string(), total, healthy, pct));
        }
    }
    for (broker, (total, healthy)) in totals {
        let pct = if total == 0 {
            0.0
        } else {
            (healthy as f32 / total as f32) * 100.0
        };
        out.push((broker, total, healthy, pct));
    }
    out
}

pub(super) fn apply_storage_snapshot(
    bg: &mut BgData,
    cache_stats: (i64, i64, i64),
    detailed_rows: Vec<(String, i64, i64, i64)>,
) {
    bg.cache_stats = Some(cache_stats);

    let mut detailed_stats = Vec::with_capacity(detailed_rows.len());
    let mut cache_blob_sizes = std::collections::HashMap::with_capacity(detailed_rows.len());
    for (key, bar_count, timestamp, blob_bytes) in detailed_rows {
        cache_blob_sizes.insert(key.clone(), blob_bytes);
        detailed_stats.push((key, bar_count, timestamp));
    }

    bg.detailed_stats = detailed_stats;
    bg.cache_blob_sizes = cache_blob_sizes;

    let current_keys: std::collections::HashSet<&str> = bg
        .detailed_stats
        .iter()
        .map(|(key, _, _)| key.as_str())
        .collect();
    bg.bar_ts_cache
        .retain(|key, _| current_keys.contains(key.as_str()));
}

pub(super) fn format_bytes_human(bytes: i64) -> String {
    if bytes < 0 {
        return "\u{2014}".to_string();
    }
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let units = ["KB", "MB", "GB", "TB"];
    let mut value = bytes as f64 / 1024.0;
    let mut unit_idx = 0usize;
    while value >= 1024.0 && unit_idx + 1 < units.len() {
        value /= 1024.0;
        unit_idx += 1;
    }
    format!("{value:.1} {}", units[unit_idx])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_bar_sync_stats_counts_current_detailed_rows() {
        let now_s = chrono::Utc::now().timestamp();
        let old_bar_ms = (now_s - 90 * 86_400).saturating_mul(1000);
        let rows = compute_bar_sync_stats(
            &[
                ("alpaca:AAPL:1Day".into(), 42, now_s),
                ("alpaca:MSFT:1Day".into(), 42, 1_700_000_000),
                ("alpaca:__META__:1Day".into(), 42, now_s),
                ("alpaca:QQQ:W1".into(), 10, now_s),
            ],
            &std::collections::HashMap::from([("alpaca:AAPL:1Day".into(), (1, old_bar_ms, 1))]),
            &|_| false,
        );

        let day = rows
            .iter()
            .find(|row| row.broker == "Alpaca" && row.tf == "1Day")
            .expect("missing 1Day row");
        assert_eq!(day.total, 2);

        let week = rows
            .iter()
            .find(|row| row.broker == "Alpaca" && row.tf == "1Week")
            .expect("missing 1Week row");
        assert_eq!(week.total, 1);
    }

    #[test]
    fn backfill_complete_pair_with_aged_bar_counts_healthy() {
        // 1Hour staleness window is 24h; this last bar is 48h old AND the
        // last cache write was also 48h ago (so `recently_checked` is false).
        // Without the backfill_complete escape hatch this row would land in
        // Stale; with it, it stays Healthy because Kraken has no newer bar.
        let now_s = chrono::Utc::now().timestamp();
        let aged_bar_ms = (now_s - 48 * 3600).saturating_mul(1000);
        let aged_write_s = now_s - 48 * 3600;
        let key = "kraken:BTC/USD:1Hour".to_string();

        let stale_rows = compute_bar_sync_stats(
            &[(key.clone(), 720, aged_write_s)],
            &std::collections::HashMap::from([(key.clone(), (1, aged_bar_ms, 1))]),
            &|_| false,
        );
        let stale_row = stale_rows
            .iter()
            .find(|row| row.broker == "Kraken Spot" && row.tf == "1Hour")
            .expect("missing 1Hour row");
        assert_eq!(stale_row.stale, 1);
        assert_eq!(stale_row.healthy, 0);

        let healthy_rows = compute_bar_sync_stats(
            &[(key.clone(), 720, aged_write_s)],
            &std::collections::HashMap::from([(key.clone(), (1, aged_bar_ms, 1))]),
            &|k| k == "kraken:BTC/USD:1Hour",
        );
        let healthy_row = healthy_rows
            .iter()
            .find(|row| row.broker == "Kraken Spot" && row.tf == "1Hour")
            .expect("missing 1Hour row");
        assert_eq!(healthy_row.healthy, 1);
        assert_eq!(healthy_row.stale, 0);
    }

    #[test]
    fn compute_bar_sync_stats_exposes_fallback_provider_rows() {
        let now_s = chrono::Utc::now().timestamp();
        let rows = compute_bar_sync_stats(
            &[("yahoo-chart:TNDM:1Day".into(), 10, now_s)],
            &std::collections::HashMap::new(),
            &|_| false,
        );

        let yahoo = rows
            .iter()
            .find(|row| row.broker == "Yahoo" && row.tf == "1Day")
            .expect("missing Yahoo fallback row");
        assert_eq!(yahoo.total, 1);
        assert_eq!(yahoo.healthy, 1);
    }

    #[test]
    fn compute_bar_sync_stats_hides_obsolete_assist_low_timeframe_rows_but_keeps_kraken_equities() {
        let now_s = chrono::Utc::now().timestamp();
        let rows = compute_bar_sync_stats(
            &[
                ("alpaca:AAPL:1Min".into(), 10, now_s),
                ("alpaca:AAPL:5Min".into(), 10, now_s),
                ("yahoo-chart:AAPL:1Min".into(), 10, now_s),
                ("yahoo-chart:AAPL:5Min".into(), 10, now_s),
                ("kraken-equities:AAPL:1Min".into(), 10, now_s),
                ("kraken-equities:AAPL:5Min".into(), 10, now_s),
                ("kraken:BTC/USD:1Min".into(), 10, now_s),
            ],
            &std::collections::HashMap::new(),
            &|_| false,
        );

        assert!(!rows.iter().any(|row| matches!(
            (row.broker.as_str(), row.tf.as_str()),
            ("Alpaca" | "Yahoo", "1Min" | "5Min")
        )));
        let equities = rows
            .iter()
            .find(|row| row.broker == "Kraken Equities" && row.tf == "1Min")
            .expect("valid Kraken Equities low timeframe row should remain visible");
        assert_eq!(equities.total, 1);
        let spot = rows
            .iter()
            .find(|row| row.broker == "Kraken Spot" && row.tf == "1Min")
            .expect("valid Kraken Spot low timeframe row should remain visible");
        assert_eq!(spot.total, 1);
    }

    #[test]
    fn broker_totals_orders_merged_before_fallback_sources() {
        let totals = compute_bar_sync_broker_totals(&[
            SyncStatsRow {
                broker: "Yahoo".into(),
                tf: "1Day".into(),
                total: 1,
                healthy: 1,
                stale: 0,
                empty: 0,
                settled: 0,
                note: None,
                pct_healthy: 100.0,
            },
            SyncStatsRow {
                broker: "Merged".into(),
                tf: "1Day".into(),
                total: 2,
                healthy: 2,
                stale: 0,
                empty: 0,
                settled: 0,
                note: None,
                pct_healthy: 100.0,
            },
            SyncStatsRow {
                broker: "Kraken Spot".into(),
                tf: "1Day".into(),
                total: 2,
                healthy: 1,
                stale: 1,
                empty: 0,
                settled: 0,
                note: None,
                pct_healthy: 50.0,
            },
        ]);

        let names: Vec<&str> = totals
            .iter()
            .map(|(broker, _, _, _)| broker.as_str())
            .collect();
        assert_eq!(names, vec!["Kraken Spot", "Merged", "Yahoo"]);
    }

    #[test]
    fn compute_bar_sync_stats_surfaces_kraken_equity_intraday_universe_rows() {
        let now_s = chrono::Utc::now().timestamp();
        let rows = compute_bar_sync_stats(
            &[
                ("kraken-equities:AAPL:15Min".into(), 10, now_s),
                ("kraken-equities:AAPL:1Day".into(), 10, now_s),
            ],
            &std::collections::HashMap::new(),
            &|_| false,
        );

        let fifteen = rows
            .iter()
            .find(|row| row.broker == "Kraken Equities" && row.tf == "15Min")
            .expect("Kraken Equities 15Min row should be visible");
        assert_eq!(fifteen.total, 1);

        let day = rows
            .iter()
            .find(|row| row.broker == "Kraken Equities" && row.tf == "1Day")
            .expect("missing Kraken 1Day row");
        assert_eq!(day.total, 1);
    }

    #[test]
    fn kraken_native_sources_are_reported_as_separate_lanes() {
        let now_s = chrono::Utc::now().timestamp();
        let rows = compute_bar_sync_stats(
            &[
                ("kraken:BTC/USD:1Day".into(), 10, now_s),
                ("kraken-equities:AAPL:1Day".into(), 10, now_s),
            ],
            &std::collections::HashMap::new(),
            &|_| false,
        );

        let spot = rows
            .iter()
            .find(|row| row.broker == "Kraken Spot" && row.tf == "1Day")
            .expect("missing Kraken Spot 1Day row");
        assert_eq!(spot.total, 1);

        let equities = rows
            .iter()
            .find(|row| row.broker == "Kraken Equities" && row.tf == "1Day")
            .expect("missing Kraken Equities 1Day row");
        assert_eq!(equities.total, 1);
    }

    #[test]
    fn backfill_complete_pair_reports_settled_bucket_without_counting_stale() {
        let now_s = chrono::Utc::now().timestamp();
        let aged_bar_ms = (now_s - 48 * 3600).saturating_mul(1000);
        let aged_write_s = now_s - 48 * 3600;
        let key = "kraken:ETH/USD:1Hour".to_string();

        let rows = compute_bar_sync_stats(
            &[(key.clone(), 720, aged_write_s)],
            &std::collections::HashMap::from([(key.clone(), (1, aged_bar_ms, 1))]),
            &|k| k == key,
        );

        let hour = rows
            .iter()
            .find(|row| row.broker == "Kraken Spot" && row.tf == "1Hour")
            .expect("missing Kraken Spot 1Hour row");
        assert_eq!(hour.healthy, 1);
        assert_eq!(hour.stale, 0);
        assert_eq!(hour.settled, 1);
    }

    #[test]
    fn sync_status_row_cells_surface_settled_and_notes() {
        let row = SyncStatsRow {
            broker: "Merged".into(),
            tf: "5Min".into(),
            total: 3,
            healthy: 1,
            stale: 1,
            empty: 1,
            settled: 7,
            note: Some("provider lane note".to_string()),
            pct_healthy: 33.3,
        };
        let cells = sync_stats_row_status_cells(&row);
        assert_eq!(cells.symbols, "3");
        assert_eq!(cells.healthy, "1");
        assert_eq!(cells.stale_or_empty, "2");
        assert_eq!(cells.settled, "7");
        assert_eq!(cells.note, "provider lane note");
    }
}
