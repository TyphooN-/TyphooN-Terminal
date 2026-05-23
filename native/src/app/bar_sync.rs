use super::BgDarwinData;
use super::sync_workset::normalize_sync_timeframe_key;

/// Per-(broker,TF) bar-sync health snapshot for the Sync Status window + the
/// compact Storage Manager banner. When Research / Backtest sync tallies land
/// later, a `category` discriminant plus a dedicated row type can be grafted
/// on — for now this struct is the whole surface so we don't leave any
/// unused scaffolding in the tree.
#[derive(Clone, Debug, Default)]
pub(super) struct SyncStatsRow {
    pub(super) broker: String,   // "MT5" | "Alpaca" | "Tastytrade" | "Kraken"
    pub(super) tf: String,       // "1Min" | "1Hour" | "1Day" | …
    pub(super) total: u64,       // (sym,tf) pairs seen for this (broker,tf) bucket
    pub(super) healthy: u64,     // last bar lag < TF_period × 24
    pub(super) stale: u64,       // last bar lag ≥ threshold
    pub(super) empty: u64,       // cached blob has no bars (last_ms <= 0)
    pub(super) pct_healthy: f32, // 0..100
}

/// Aggregate `bar_ts_cache` into per-(broker,TF) rows. Always emits rows for
/// actively managed live bar sources (Kraken / Alpaca / Tastytrade). MT5 appears
/// only after it has actually written bars, so unconfigured MT5 sync no longer
/// consumes Sync Status space.
pub(super) fn compute_bar_sync_stats(
    detailed_stats: &[(String, i64, i64)],
    bar_ts_cache: &std::collections::HashMap<String, (i64, i64, i64)>,
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
            "mt5" => Some("MT5"),
            "alpaca" => Some("Alpaca"),
            "tastytrade" => Some("Tastytrade"),
            "kraken" | "kraken-futures" | "kraken-equities" => Some("Kraken"),
            _ => None,
        }
    };
    let required_brokers = ["Kraken"];

    let mut groups: BTreeMap<(String, String), (u64, u64, u64)> = BTreeMap::new();
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
            if now_ms - last_ms > period * 24 && !recently_checked {
                entry.1 += 1;
            } else {
                entry.0 += 1;
            }
        } else {
            entry.1 += 1;
        }
    }

    let mut rows: Vec<SyncStatsRow> = groups
        .into_iter()
        .map(|((broker, tf), (healthy, stale, empty))| {
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
    rows.sort_by(|a, b| {
        let ai = tf_order
            .iter()
            .position(|tf| *tf == a.tf)
            .unwrap_or(usize::MAX);
        let bi = tf_order
            .iter()
            .position(|tf| *tf == b.tf)
            .unwrap_or(usize::MAX);
        a.broker.cmp(&b.broker).then(ai.cmp(&bi))
    });
}

/// Aggregate per-broker totals from a Vec<SyncStatsRow> for the compact
/// banner / one-liner display. Returns `(broker, total, healthy, pct)`
/// tuples in display order (Kraken, Alpaca, Tastytrade, MT5, then any others).
pub(super) fn compute_bar_sync_broker_totals(
    rows: &[SyncStatsRow],
) -> Vec<(String, u64, u64, f32)> {
    use std::collections::BTreeMap;

    let mut totals: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for row in rows {
        let entry = totals.entry(row.broker.clone()).or_default();
        entry.0 += row.total;
        entry.1 += row.healthy;
    }

    let order = ["Kraken", "Alpaca", "Tastytrade", "MT5"];
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
    bg: &mut BgDarwinData,
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
}
