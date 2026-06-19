// ── Round 22 tests ──

#[test]
fn retskew_snapshot_roundtrip() {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    create_research_tables_v22(&c).unwrap();
    let snap = ReturnSkewnessSnapshot {
        symbol: "AAA".into(),
        as_of: "2026-04-15".into(),
        bars_used: 253,
        mean_log_return: 0.0005,
        stdev_log_return: 0.012,
        skewness: -0.45,
        positive_return_pct: 52.0,
        largest_up_pct: 4.5,
        largest_down_pct: -6.0,
        skew_label: "LEFT".into(),
        ..Default::default()
    };
    upsert_retskew(&c, "AAA", &snap).unwrap();
    let got = get_retskew(&c, "AAA").unwrap().unwrap();
    assert_eq!(got.skew_label, "LEFT");
    assert!((got.skewness + 0.45).abs() < 1e-9);
}

#[test]
fn retkurt_snapshot_roundtrip() {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    create_research_tables_v22(&c).unwrap();
    let snap = ReturnKurtosisSnapshot {
        symbol: "BBB".into(),
        as_of: "2026-04-15".into(),
        bars_used: 253,
        excess_kurtosis: 3.5,
        outlier_2sigma_count: 12,
        outlier_3sigma_count: 2,
        outlier_2sigma_pct: 4.74,
        kurt_label: "FAT".into(),
        ..Default::default()
    };
    upsert_retkurt(&c, "BBB", &snap).unwrap();
    let got = get_retkurt(&c, "BBB").unwrap().unwrap();
    assert_eq!(got.kurt_label, "FAT");
}

#[test]
fn tailr_snapshot_roundtrip() {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    create_research_tables_v22(&c).unwrap();
    let snap = TailRatioSnapshot {
        symbol: "CCC".into(),
        as_of: "2026-04-15".into(),
        bars_used: 253,
        pct_95_return: 2.0,
        pct_05_return: -2.5,
        tail_ratio: 0.8,
        bias_label: "SLIGHT_DOWNSIDE".into(),
        ..Default::default()
    };
    upsert_tailr(&c, "CCC", &snap).unwrap();
    let got = get_tailr(&c, "CCC").unwrap().unwrap();
    assert_eq!(got.bias_label, "SLIGHT_DOWNSIDE");
}

#[test]
fn runlen_snapshot_roundtrip() {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    create_research_tables_v22(&c).unwrap();
    let snap = RunLengthSnapshot {
        symbol: "DDD".into(),
        as_of: "2026-04-15".into(),
        bars_used: 253,
        avg_up_run: 1.8,
        avg_down_run: 1.6,
        longest_up_run: 5,
        longest_down_run: 4,
        trend_label: "MIXED".into(),
        ..Default::default()
    };
    upsert_runlen(&c, "DDD", &snap).unwrap();
    let got = get_runlen(&c, "DDD").unwrap().unwrap();
    assert_eq!(got.trend_label, "MIXED");
}

#[test]
fn dayrange_snapshot_roundtrip() {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    create_research_tables_v22(&c).unwrap();
    let snap = DailyRangeSnapshot {
        symbol: "EEE".into(),
        as_of: "2026-04-15".into(),
        bars_used: 253,
        avg_range_60_pct: 1.2,
        avg_range_252_pct: 1.5,
        compression_ratio: 0.8,
        range_label: "COMPRESSED".into(),
        ..Default::default()
    };
    upsert_dayrange(&c, "EEE", &snap).unwrap();
    let got = get_dayrange(&c, "EEE").unwrap().unwrap();
    assert_eq!(got.range_label, "COMPRESSED");
}

#[test]
fn compute_retskew_insufficient() {
    let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
    let snap = compute_retskew_snapshot("AAA", "2026-04-15", &bars);
    assert_eq!(snap.skew_label, "INSUFFICIENT_DATA");
}

#[test]
fn compute_retskew_left_tail() {
    // Series with mostly small up-days and a few large down-days → negative skew.
    let mut bars = Vec::new();
    let mut c = 100.0;
    bars.push(mk_hp("2025-01-01", c, c + 0.1, c - 0.1, c));
    for i in 0..200 {
        let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
        let change = if i % 25 == 0 { -8.0 } else { 0.3 }; // occasional large down
        c *= 1.0 + change / 100.0;
        bars.push(mk_hp(&date, c, c + 0.1, c - 0.1, c));
    }
    let snap = compute_retskew_snapshot("AAA", "2026-04-15", &bars);
    assert!(snap.skew_label != "INSUFFICIENT_DATA");
    assert!(
        snap.skewness < -0.2,
        "expected negative skew, got {}",
        snap.skewness
    );
}

#[test]
fn compute_retkurt_fat_tails() {
    // Mostly quiet with rare 5-sigma events → fat-tailed.
    let mut bars = Vec::new();
    let mut c = 100.0;
    bars.push(mk_hp("2025-01-01", c, c + 0.1, c - 0.1, c));
    for i in 0..200 {
        let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
        let change = if i % 40 == 0 { 10.0 } else { 0.05 };
        c *= 1.0 + change / 100.0;
        bars.push(mk_hp(&date, c, c + 0.1, c - 0.1, c));
    }
    let snap = compute_retkurt_snapshot("AAA", "2026-04-15", &bars);
    assert!(snap.kurt_label != "INSUFFICIENT_DATA");
    assert!(
        snap.excess_kurtosis > 1.0,
        "expected fat-tailed, got {}",
        snap.excess_kurtosis
    );
}

#[test]
fn compute_retkurt_insufficient() {
    let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
    let snap = compute_retkurt_snapshot("AAA", "2026-04-15", &bars);
    assert_eq!(snap.kurt_label, "INSUFFICIENT_DATA");
}

#[test]
fn compute_tailr_balanced() {
    // Symmetric small random moves → balanced tail ratio.
    let mut bars = Vec::new();
    let mut c = 100.0;
    bars.push(mk_hp("2025-01-01", c, c + 0.1, c - 0.1, c));
    for i in 0..200 {
        let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
        let change = if i % 2 == 0 { 1.0 } else { -1.0 };
        c *= 1.0 + change / 100.0;
        bars.push(mk_hp(&date, c, c + 0.1, c - 0.1, c));
    }
    let snap = compute_tailr_snapshot("AAA", "2026-04-15", &bars);
    assert!(snap.bias_label != "INSUFFICIENT_DATA");
    assert!(snap.tail_ratio > 0.5 && snap.tail_ratio < 2.0);
}

#[test]
fn compute_tailr_insufficient() {
    let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
    let snap = compute_tailr_snapshot("AAA", "2026-04-15", &bars);
    assert_eq!(snap.bias_label, "INSUFFICIENT_DATA");
}

#[test]
fn compute_runlen_trending() {
    // Monotone up → one long run.
    let mut bars = Vec::new();
    for i in 0..100 {
        let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
        let c = 100.0 + i as f64 * 0.5;
        bars.push(mk_hp(&date, c, c + 0.1, c - 0.1, c));
    }
    let snap = compute_runlen_snapshot("AAA", "2026-04-15", &bars);
    assert!(snap.trend_label != "INSUFFICIENT_DATA");
    assert!(
        snap.longest_up_run >= 30,
        "expected long up-run, got {}",
        snap.longest_up_run
    );
    assert_eq!(snap.longest_down_run, 0);
    assert!(snap.current_run_length > 0);
}

#[test]
fn compute_runlen_choppy() {
    // Alternating up/down → average run length 1.
    let mut bars = Vec::new();
    let mut c = 100.0;
    bars.push(mk_hp("2025-01-01", c, c + 0.1, c - 0.1, c));
    for i in 0..100 {
        let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
        c += if i % 2 == 0 { 1.0 } else { -1.0 };
        bars.push(mk_hp(&date, c, c + 0.1, c - 0.1, c));
    }
    let snap = compute_runlen_snapshot("AAA", "2026-04-15", &bars);
    assert_eq!(snap.trend_label, "CHOPPY");
    assert!(snap.avg_up_run < 1.5);
    assert!(snap.avg_down_run < 1.5);
}

#[test]
fn compute_runlen_insufficient() {
    let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
    let snap = compute_runlen_snapshot("AAA", "2026-04-15", &bars);
    assert_eq!(snap.trend_label, "INSUFFICIENT_DATA");
}

#[test]
fn compute_dayrange_compressed() {
    // Wide ranges historically, tighter recently → compressed.
    let mut bars = Vec::new();
    for i in 0..253 {
        let date = format!("2025-{:02}-{:02}", (i / 21) + 1, (i % 21) + 1);
        let c = 100.0;
        let (h, l) = if i < 200 {
            (c + 2.0, c - 2.0) // wide historical
        } else {
            (c + 0.3, c - 0.3) // tight recent
        };
        bars.push(mk_hp(&date, c, h, l, c));
    }
    let snap = compute_dayrange_snapshot("AAA", "2026-04-15", &bars);
    assert!(
        snap.range_label == "TIGHT" || snap.range_label == "COMPRESSED",
        "expected compressed, got {}",
        snap.range_label
    );
    assert!(snap.compression_ratio < 1.0);
}

#[test]
fn compute_dayrange_expanded() {
    // Tight historical, wide recent → expanded.
    let mut bars = Vec::new();
    for i in 0..253 {
        let date = format!("2025-{:02}-{:02}", (i / 21) + 1, (i % 21) + 1);
        let c = 100.0;
        let (h, l) = if i < 200 {
            (c + 0.3, c - 0.3)
        } else {
            (c + 3.0, c - 3.0)
        };
        bars.push(mk_hp(&date, c, h, l, c));
    }
    let snap = compute_dayrange_snapshot("AAA", "2026-04-15", &bars);
    assert!(
        snap.range_label == "EXPANDED" || snap.range_label == "VERY_EXPANDED",
        "expected expanded, got {}",
        snap.range_label
    );
    assert!(snap.compression_ratio > 1.0);
}

#[test]
fn compute_dayrange_insufficient() {
    let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
    let snap = compute_dayrange_snapshot("AAA", "2026-04-15", &bars);
    assert_eq!(snap.range_label, "INSUFFICIENT_DATA");
}

// ── Web article ingestion tests ──

#[test]
fn ingested_articles_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v23(&c).unwrap();
    let snap = IngestedArticlesSnapshot {
        symbol: "AAPL".into(),
        articles: vec![WebArticle {
            title: "iPhone sales beat".into(),
            url: "https://example.com/a".into(),
            source: "Reuters".into(),
            published_at: "2026-04-10".into(),
            summary: "Strong quarter.".into(),
            agent_used: "claude".into(),
            ingested_at: 1_700_000_000,
            body: String::new(),
        }],
    };
    upsert_ingested_articles(&c, "AAPL", &snap).unwrap();
    let got = get_ingested_articles(&c, "AAPL").unwrap().unwrap();
    assert_eq!(got.articles.len(), 1);
    assert_eq!(got.articles[0].url, "https://example.com/a");
}

#[test]
fn ingested_articles_append_dedupe_and_cap() {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v23(&c).unwrap();
    let mk = |url: &str, ts: i64| WebArticle {
        url: url.into(),
        ingested_at: ts,
        ..Default::default()
    };
    let batch1 = vec![mk("u1", 100), mk("u2", 110)];
    let (added1, total1) = append_ingested_articles(&c, "AAA", batch1).unwrap();
    assert_eq!(added1, 2);
    assert_eq!(total1, 2);

    // Same URL newer timestamp should replace, not add.
    let batch2 = vec![mk("u1", 200), mk("u3", 210)];
    let (added2, total2) = append_ingested_articles(&c, "AAA", batch2).unwrap();
    assert_eq!(added2, 1);
    assert_eq!(total2, 3);

    // Cap at INGESTED_ARTICLES_MAX: inject 60 more unique URLs.
    let big: Vec<WebArticle> = (0..60).map(|i| mk(&format!("big{}", i), 300 + i)).collect();
    let (_, total3) = append_ingested_articles(&c, "AAA", big).unwrap();
    assert_eq!(total3, INGESTED_ARTICLES_MAX);

    let got = get_ingested_articles(&c, "AAA").unwrap().unwrap();
    // Most recent first: big59 should be at the top.
    assert_eq!(got.articles[0].url, "big59");
}

#[test]
fn parse_ingest_block_extracts_articles() {
    let text = r#"
Some preamble from the agent.

===TYPHOON_INGEST===
[
  {"symbol": "AAPL", "title": "iPhone sales beat", "url": "https://r.com/a",
   "source": "Reuters", "published_at": "2026-04-10", "summary": "Strong.",
   "agent": "claude"},
  {"symbol": "aapl", "title": "Services growth", "url": "https://b.com/b",
   "source": "Bloomberg", "published": "2026-04-11", "summary": "Good.",
   "agent": "claude"},
  {"symbol": "MSFT", "title": "Azure outage", "url": "https://c.com/c",
   "source": "TheVerge", "date": "2026-04-09", "summary": "Brief.",
   "agent": "claude"}
]
===END_INGEST===

Trailing text.
"#;
    let parsed = parse_ingest_block(text);
    let by_sym: std::collections::HashMap<_, _> = parsed.into_iter().collect();
    assert_eq!(by_sym.get("AAPL").map(|v| v.len()), Some(2));
    assert_eq!(by_sym.get("MSFT").map(|v| v.len()), Some(1));
    let msft = &by_sym["MSFT"][0];
    assert_eq!(msft.published_at, "2026-04-09");
    assert_eq!(msft.agent_used, "claude");
}

#[test]
fn parse_ingest_block_with_json_fence() {
    let text = r#"
===TYPHOON_INGEST===
```json
[
  {"symbol": "NVDA", "title": "Blackwell", "url": "https://x.com/n",
   "source": "CNBC", "published_at": "2026-04-12", "summary": "Demand.",
   "agent": "gemini"}
]
```
===END_INGEST===
"#;
    let parsed = parse_ingest_block(text);
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].0, "NVDA");
    assert_eq!(parsed[0].1.len(), 1);
}

#[test]
fn parse_ingest_block_skips_malformed_entries() {
    let text = r#"
===TYPHOON_INGEST===
[
  {"symbol": "AAPL"},
  {"url": "https://no-symbol.com/x"},
  {"symbol": "TSLA", "url": "https://good.com/g"}
]
===END_INGEST===
"#;
    let parsed = parse_ingest_block(text);
    let by_sym: std::collections::HashMap<_, _> = parsed.into_iter().collect();
    assert_eq!(by_sym.get("TSLA").map(|v| v.len()), Some(1));
    assert!(!by_sym.contains_key("AAPL"));
}

#[test]
fn parse_ingest_block_returns_empty_when_missing() {
    let text = "No ingest block here.";
    let parsed = parse_ingest_block(text);
    assert!(parsed.is_empty());
}

#[test]
fn parse_ingest_block_reads_optional_body_field() {
    let text = r#"
===TYPHOON_INGEST===
[
  {"symbol": "AAPL", "url": "https://r.com/a", "title": "iPhone sales",
   "summary": "Strong quarter.",
   "body": "Apple reported Q3 iPhone sales of 50M units, up 12% YoY..."},
  {"symbol": "MSFT", "url": "https://b.com/b", "title": "Azure growth",
   "summary": "Cloud accelerating.",
   "text": "Microsoft Azure revenue grew 28% on AI workload demand..."}
]
===END_INGEST===
"#;
    let parsed = parse_ingest_block(text);
    let by_sym: std::collections::HashMap<_, _> = parsed.into_iter().collect();
    let aapl = &by_sym["AAPL"][0];
    assert!(aapl.body.starts_with("Apple reported Q3"));
    // `text` is accepted as an alias for `body` (different LLMs prefer
    // different keys; both should round-trip into the same field).
    let msft = &by_sym["MSFT"][0];
    assert!(msft.body.starts_with("Microsoft Azure"));
}

#[test]
fn parse_ingest_block_defaults_body_to_empty_when_absent() {
    let text = r#"
===TYPHOON_INGEST===
[
  {"symbol": "NVDA", "url": "https://x.com/n", "title": "Blackwell",
   "summary": "Strong demand."}
]
===END_INGEST===
"#;
    let parsed = parse_ingest_block(text);
    let by_sym: std::collections::HashMap<_, _> = parsed.into_iter().collect();
    assert_eq!(by_sym["NVDA"][0].body, "");
}

#[test]
fn web_article_body_roundtrips_through_snapshot() {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v23(&c).unwrap();
    let snap = IngestedArticlesSnapshot {
        symbol: "AAPL".into(),
        articles: vec![WebArticle {
            title: "Paywalled scoop".into(),
            url: "https://wsj.com/article".into(),
            source: "WSJ".into(),
            summary: "Big news.".into(),
            body: "Full paywalled text the agent fetched and returned.".into(),
            ingested_at: 1_700_000_000,
            ..Default::default()
        }],
    };
    upsert_ingested_articles(&c, "AAPL", &snap).unwrap();
    let got = get_ingested_articles(&c, "AAPL").unwrap().unwrap();
    assert_eq!(
        got.articles[0].body,
        "Full paywalled text the agent fetched and returned."
    );
}

#[test]
fn web_article_backward_compatible_json_without_body() {
    // Simulate a snapshot blob written by an older binary that didn't
    // know about the `body` field — deserialise should not fail, and
    // body should default to empty.
    let legacy_json = r#"{
            "symbol": "AAPL",
            "articles": [
                {
                    "title": "Old article",
                    "url": "https://example.com/x",
                    "source": "Reuters",
                    "published_at": "2026-01-01",
                    "summary": "From before body shipped.",
                    "agent_used": "claude",
                    "ingested_at": 1700000000
                }
            ]
        }"#;
    let snap: IngestedArticlesSnapshot =
        serde_json::from_str(legacy_json).expect("legacy JSON must deserialize");
    assert_eq!(snap.articles[0].body, "");
    assert_eq!(snap.articles[0].title, "Old article");
}

// ── Round 23 tests ──

fn synthetic_up_trend_bars() -> Vec<HistoricalPriceRow> {
    // 60 bars, each slightly higher than the previous (deterministic
    // drift). Simulates a persistent uptrend: Hurst should be > 0.5,
    // hit rate should be high, GLASYM ratio should be ≥ 1, AUTOCOR
    // lag 1 ~ 0.
    (0..60)
        .map(|i| {
            let close = 100.0 + (i as f64) * 0.5;
            HistoricalPriceRow {
                date: format!("2025-{:02}-{:02}", 1 + (i / 20) as u32, 1 + (i % 20) as u32),
                open: close - 0.25,
                high: close + 0.5,
                low: close - 0.75,
                close,
                adj_close: close,
                volume: 1_000_000.0 + (i as f64) * 50_000.0,
                change: 0.5,
                change_pct: 0.5,
            }
        })
        .collect()
}

fn synthetic_mixed_bars() -> Vec<HistoricalPriceRow> {
    // 60 bars, alternating up/down ~equally — tests the BALANCED /
    // NEUTRAL / RANDOM_WALK paths.
    (0..60)
        .map(|i| {
            let base = 100.0;
            let close = if i % 2 == 0 { base + 1.0 } else { base - 1.0 };
            HistoricalPriceRow {
                date: format!("2025-{:02}-{:02}", 1 + (i / 20) as u32, 1 + (i % 20) as u32),
                open: base,
                high: base + 1.5,
                low: base - 1.5,
                close,
                adj_close: close,
                volume: if i % 2 == 0 { 2_000_000.0 } else { 1_000_000.0 },
                change: 0.0,
                change_pct: 0.0,
            }
        })
        .collect()
}

#[test]
fn autocor_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = AutocorrelationSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 200,
        lag1_acf: -0.12,
        lag5_acf: 0.02,
        lag10_acf: -0.01,
        lag20_acf: 0.03,
        mean_log_return: 0.0008,
        regime_label: "MEAN_REVERT".into(),
        note: String::new(),
    };
    upsert_autocor(&c, "TEST", &snap).unwrap();
    let got = get_autocor(&c, "TEST").unwrap().unwrap();
    assert_eq!(got.symbol, "TEST");
    assert!((got.lag1_acf - -0.12).abs() < 1e-9);
    assert_eq!(got.regime_label, "MEAN_REVERT");
}

#[test]
fn autocor_compute_insufficient_data() {
    let snap = compute_autocor_snapshot("X", "2026-04-15", &[]);
    assert_eq!(snap.regime_label, "INSUFFICIENT_DATA");
}

#[test]
fn autocor_compute_uptrend_labels() {
    let bars = synthetic_up_trend_bars();
    let snap = compute_autocor_snapshot("X", "2026-04-15", &bars);
    assert_ne!(snap.regime_label, "INSUFFICIENT_DATA");
    assert!(snap.bars_used >= 30);
}

#[test]
fn hurst_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = HurstSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 253,
        hurst_exponent: 0.58,
        scales_used: 4,
        min_scale: 8,
        max_scale: 64,
        memory_label: "PERSISTENT".into(),
        note: String::new(),
    };
    upsert_hurst(&c, "TEST", &snap).unwrap();
    let got = get_hurst(&c, "TEST").unwrap().unwrap();
    assert!((got.hurst_exponent - 0.58).abs() < 1e-9);
    assert_eq!(got.memory_label, "PERSISTENT");
}

#[test]
fn hurst_compute_insufficient_data() {
    let snap = compute_hurst_snapshot("X", "2026-04-15", &[]);
    assert_eq!(snap.memory_label, "INSUFFICIENT_DATA");
}

#[test]
fn hurst_compute_picks_label() {
    let bars = synthetic_mixed_bars();
    let snap = compute_hurst_snapshot("X", "2026-04-15", &bars);
    assert_ne!(snap.memory_label, "INSUFFICIENT_DATA");
    assert!(snap.scales_used >= 2);
}

#[test]
fn hitrate_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = HitRateSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 253,
        hitrate_5d: 60.0,
        hitrate_20d: 55.0,
        hitrate_60d: 52.0,
        hitrate_252d: 51.0,
        up_days: 130,
        down_days: 120,
        flat_days: 3,
        hit_label: "WEAK_BULLISH".into(),
        note: String::new(),
    };
    upsert_hitrate(&c, "TEST", &snap).unwrap();
    let got = get_hitrate(&c, "TEST").unwrap().unwrap();
    assert_eq!(got.up_days, 130);
    assert_eq!(got.hit_label, "WEAK_BULLISH");
}

#[test]
fn hitrate_compute_uptrend_is_bullish() {
    let bars = synthetic_up_trend_bars();
    let snap = compute_hitrate_snapshot("X", "2026-04-15", &bars);
    assert_ne!(snap.hit_label, "INSUFFICIENT_DATA");
    assert!(
        snap.up_days > snap.down_days,
        "uptrend should have more up days"
    );
}

#[test]
fn glasym_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = GainLossAsymmetrySnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 253,
        avg_up_pct: 1.2,
        avg_down_pct: 1.1,
        median_up_pct: 0.9,
        median_down_pct: 0.8,
        magnitude_ratio: 1.09,
        up_days: 130,
        down_days: 120,
        asymmetry_label: "BALANCED".into(),
        note: String::new(),
    };
    upsert_glasym(&c, "TEST", &snap).unwrap();
    let got = get_glasym(&c, "TEST").unwrap().unwrap();
    assert!((got.magnitude_ratio - 1.09).abs() < 1e-9);
    assert_eq!(got.asymmetry_label, "BALANCED");
}

#[test]
fn glasym_compute_insufficient_when_empty() {
    let snap = compute_glasym_snapshot("X", "2026-04-15", &[]);
    assert_eq!(snap.asymmetry_label, "INSUFFICIENT_DATA");
}

#[test]
fn glasym_compute_mixed_is_balanced() {
    let bars = synthetic_mixed_bars();
    let snap = compute_glasym_snapshot("X", "2026-04-15", &bars);
    assert_ne!(snap.asymmetry_label, "INSUFFICIENT_DATA");
    assert!(snap.up_days > 0 && snap.down_days > 0);
}

#[test]
fn volratio_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = VolumeRatioSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 253,
        avg_up_volume: 2_500_000.0,
        avg_down_volume: 2_000_000.0,
        median_up_volume: 2_400_000.0,
        median_down_volume: 1_900_000.0,
        up_down_volume_ratio: 1.25,
        max_up_volume: 8_000_000.0,
        max_down_volume: 5_500_000.0,
        up_days: 130,
        down_days: 120,
        flow_label: "SLIGHT_ACCUMULATION".into(),
        note: String::new(),
    };
    upsert_volratio(&c, "TEST", &snap).unwrap();
    let got = get_volratio(&c, "TEST").unwrap().unwrap();
    assert!((got.up_down_volume_ratio - 1.25).abs() < 1e-9);
    assert_eq!(got.flow_label, "SLIGHT_ACCUMULATION");
}

#[test]
fn volratio_compute_no_volume_returns_insufficient() {
    let bars: Vec<HistoricalPriceRow> = (0..30)
        .map(|i| HistoricalPriceRow {
            date: format!("2025-01-{:02}", i + 1),
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.0 + i as f64,
            adj_close: 100.0,
            volume: 0.0,
            change: 0.0,
            change_pct: 0.0,
        })
        .collect();
    let snap = compute_volratio_snapshot("X", "2026-04-15", &bars);
    assert_eq!(snap.flow_label, "INSUFFICIENT_DATA");
}

#[test]
fn volratio_compute_with_volume() {
    let bars = synthetic_mixed_bars();
    let snap = compute_volratio_snapshot("X", "2026-04-15", &bars);
    assert_ne!(snap.flow_label, "INSUFFICIENT_DATA");
    assert!(snap.up_days > 0 && snap.down_days > 0);
}

// ── Round 24 tests ──

fn synthetic_gappy_bars() -> Vec<HistoricalPriceRow> {
    // 40 bars with intentional overnight gaps (open ≠ prior close).
    (0..40)
        .map(|i| {
            let prior_close = 100.0 + (i as f64);
            let gap = if i % 5 == 0 {
                1.5
            } else if i % 7 == 0 {
                -1.2
            } else {
                0.2
            };
            let open = prior_close + gap;
            let close = open + 0.3;
            HistoricalPriceRow {
                date: format!("2025-{:02}-{:02}", 1 + (i / 20) as u32, 1 + (i % 20) as u32),
                open,
                high: open + 0.8,
                low: open - 0.6,
                close,
                adj_close: close,
                volume: 1_000_000.0,
                change: 0.3,
                change_pct: 0.3,
            }
        })
        .collect()
}

#[test]
fn drawup_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = DrawupHistorySnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 200,
        max_drawup_pct: 22.5,
        max_drawup_trough_date: "2025-06-01".into(),
        max_drawup_peak_date: "2025-09-15".into(),
        longest_drawup_days: 45,
        rallies_5pct: 4,
        rallies_10pct: 2,
        current_drawup_pct: 3.2,
        rally_label: "STRONG".into(),
        note: String::new(),
    };
    upsert_drawup(&c, "TEST", &snap).unwrap();
    let got = get_drawup(&c, "TEST").unwrap().unwrap();
    assert!((got.max_drawup_pct - 22.5).abs() < 1e-9);
    assert_eq!(got.rally_label, "STRONG");
}

#[test]
fn drawup_compute_up_trend_is_explosive() {
    let bars = synthetic_up_trend_bars();
    let snap = compute_drawup_snapshot("X", "2026-04-15", &bars);
    assert_ne!(snap.rally_label, "INSUFFICIENT_DATA");
    assert!(snap.max_drawup_pct > 0.0);
}

#[test]
fn drawup_compute_insufficient() {
    let snap = compute_drawup_snapshot("X", "2026-04-15", &[]);
    assert_eq!(snap.rally_label, "INSUFFICIENT_DATA");
}

#[test]
fn gapstats_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = GapStatsSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 252,
        gap_up_count: 30,
        gap_down_count: 25,
        avg_gap_pct: 0.08,
        avg_gap_up_pct: 1.5,
        avg_gap_down_pct: -1.3,
        largest_gap_up_pct: 6.2,
        largest_gap_down_pct: -4.8,
        gap_frequency_pct: 21.83,
        bias_label: "SLIGHT_UP".into(),
        note: String::new(),
    };
    upsert_gapstats(&c, "TEST", &snap).unwrap();
    let got = get_gapstats(&c, "TEST").unwrap().unwrap();
    assert_eq!(got.gap_up_count, 30);
    assert_eq!(got.bias_label, "SLIGHT_UP");
}

#[test]
fn gapstats_compute_with_gaps() {
    let bars = synthetic_gappy_bars();
    let snap = compute_gapstats_snapshot("X", "2026-04-15", &bars);
    assert_ne!(snap.bias_label, "INSUFFICIENT_DATA");
    assert!(snap.gap_up_count > 0 || snap.gap_down_count > 0);
}

#[test]
fn gapstats_compute_insufficient() {
    let snap = compute_gapstats_snapshot("X", "2026-04-15", &[]);
    assert_eq!(snap.bias_label, "INSUFFICIENT_DATA");
}

#[test]
fn volcluster_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = VolClusterSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 252,
        sq_acf_lag1: 0.18,
        sq_acf_lag5: 0.09,
        sq_acf_lag20: 0.05,
        abs_acf_lag1: 0.22,
        abs_acf_lag5: 0.12,
        abs_acf_lag20: 0.07,
        cluster_label: "MODERATE".into(),
        note: String::new(),
    };
    upsert_volcluster(&c, "TEST", &snap).unwrap();
    let got = get_volcluster(&c, "TEST").unwrap().unwrap();
    assert!((got.abs_acf_lag1 - 0.22).abs() < 1e-9);
    assert_eq!(got.cluster_label, "MODERATE");
}

#[test]
fn volcluster_compute_insufficient() {
    let snap = compute_volcluster_snapshot("X", "2026-04-15", &[]);
    assert_eq!(snap.cluster_label, "INSUFFICIENT_DATA");
}

#[test]
fn closeplc_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = ClosePlacementSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 200,
        avg_placement: 0.65,
        median_placement: 0.70,
        latest_placement: 0.82,
        pct_near_high: 35.0,
        pct_near_low: 12.0,
        placement_label: "BULL".into(),
        note: String::new(),
    };
    upsert_closeplc(&c, "TEST", &snap).unwrap();
    let got = get_closeplc(&c, "TEST").unwrap().unwrap();
    assert!((got.avg_placement - 0.65).abs() < 1e-9);
    assert_eq!(got.placement_label, "BULL");
}

#[test]
fn closeplc_compute_with_bars() {
    let bars = synthetic_up_trend_bars();
    let snap = compute_closeplc_snapshot("X", "2026-04-15", &bars);
    assert_ne!(snap.placement_label, "INSUFFICIENT_DATA");
    assert!(snap.avg_placement >= 0.0 && snap.avg_placement <= 1.0);
}

#[test]
fn closeplc_compute_insufficient() {
    let snap = compute_closeplc_snapshot("X", "2026-04-15", &[]);
    assert_eq!(snap.placement_label, "INSUFFICIENT_DATA");
}

#[test]
fn mrhl_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = MeanReversionHalfLifeSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 252,
        beta: 0.25,
        alpha: 0.0001,
        half_life_days: 0.5,
        r_squared: 0.06,
        regime_label: "NEUTRAL".into(),
        note: String::new(),
    };
    upsert_mrhl(&c, "TEST", &snap).unwrap();
    let got = get_mrhl(&c, "TEST").unwrap().unwrap();
    assert!((got.beta - 0.25).abs() < 1e-9);
    assert_eq!(got.regime_label, "NEUTRAL");
}

#[test]
fn mrhl_compute_insufficient() {
    let snap = compute_mrhl_snapshot("X", "2026-04-15", &[]);
    assert_eq!(snap.regime_label, "INSUFFICIENT_DATA");
}

#[test]
fn mrhl_compute_with_bars() {
    let bars = synthetic_mixed_bars();
    let snap = compute_mrhl_snapshot("X", "2026-04-15", &bars);
    assert_ne!(snap.regime_label, "INSUFFICIENT_DATA");
    assert!(snap.beta.is_finite());
}

// ── Round 25 tests ──

#[test]
fn downvol_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = DownsideVolSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 252,
        mean_log_return: 0.0008,
        downside_dev: 0.012,
        downside_dev_ann: 0.19,
        upside_dev: 0.011,
        sortino_ratio: 0.067,
        sortino_ratio_ann: 1.06,
        downside_pct_of_total: 50.5,
        sortino_label: "GOOD".into(),
        note: String::new(),
    };
    upsert_downvol(&c, "TEST", &snap).unwrap();
    let got = get_downvol(&c, "TEST").unwrap().unwrap();
    assert!((got.sortino_ratio_ann - 1.06).abs() < 1e-9);
    assert_eq!(got.sortino_label, "GOOD");
}

#[test]
fn downvol_compute_insufficient() {
    let snap = compute_downvol_snapshot("X", "2026-04-15", &[]);
    assert_eq!(snap.sortino_label, "INSUFFICIENT_DATA");
}

#[test]
fn downvol_compute_uptrend_is_good() {
    let bars = synthetic_up_trend_bars();
    let snap = compute_downvol_snapshot("X", "2026-04-15", &bars);
    assert_ne!(snap.sortino_label, "INSUFFICIENT_DATA");
    assert!(snap.mean_log_return > 0.0);
}

#[test]
fn sharpr_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = SharpeRatioSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 252,
        mean_log_return: 0.0008,
        stdev_log_return: 0.012,
        sharpe_ratio: 0.067,
        sharpe_ratio_ann: 1.06,
        mean_return_ann: 0.2016,
        stdev_return_ann: 0.19,
        sharpe_label: "GOOD".into(),
        note: String::new(),
    };
    upsert_sharpr(&c, "TEST", &snap).unwrap();
    let got = get_sharpr(&c, "TEST").unwrap().unwrap();
    assert!((got.sharpe_ratio_ann - 1.06).abs() < 1e-9);
    assert_eq!(got.sharpe_label, "GOOD");
}

#[test]
fn sharpr_compute_insufficient() {
    let snap = compute_sharpr_snapshot("X", "2026-04-15", &[]);
    assert_eq!(snap.sharpe_label, "INSUFFICIENT_DATA");
}

#[test]
fn sharpr_compute_uptrend_is_positive() {
    let bars = synthetic_up_trend_bars();
    let snap = compute_sharpr_snapshot("X", "2026-04-15", &bars);
    assert_ne!(snap.sharpe_label, "INSUFFICIENT_DATA");
    assert!(snap.sharpe_ratio > 0.0);
}

#[test]
fn effratio_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = EfficiencyRatioSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 252,
        start_close: 100.0,
        end_close: 130.0,
        net_change: 30.0,
        net_change_pct: 30.0,
        sum_abs_changes: 50.0,
        efficiency_ratio: 0.6,
        signed_efficiency: 0.6,
        efficiency_label: "STRONG_TREND".into(),
        note: String::new(),
    };
    upsert_effratio(&c, "TEST", &snap).unwrap();
    let got = get_effratio(&c, "TEST").unwrap().unwrap();
    assert!((got.efficiency_ratio - 0.6).abs() < 1e-9);
    assert_eq!(got.efficiency_label, "STRONG_TREND");
}

#[test]
fn effratio_compute_uptrend_is_trending() {
    let bars = synthetic_up_trend_bars();
    let snap = compute_effratio_snapshot("X", "2026-04-15", &bars);
    assert_ne!(snap.efficiency_label, "INSUFFICIENT_DATA");
    assert!(
        snap.efficiency_ratio > 0.5,
        "strictly monotone bars should be highly efficient, got {}",
        snap.efficiency_ratio
    );
}

#[test]
fn effratio_compute_chop_is_low() {
    let bars = synthetic_mixed_bars();
    let snap = compute_effratio_snapshot("X", "2026-04-15", &bars);
    assert_ne!(snap.efficiency_label, "INSUFFICIENT_DATA");
    assert!(
        snap.efficiency_ratio < 0.5,
        "alternating bars should be choppy, got {}",
        snap.efficiency_ratio
    );
}

#[test]
fn wickbias_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = WickBiasSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 252,
        avg_upper_wick: 0.18,
        avg_lower_wick: 0.22,
        median_upper_wick: 0.15,
        median_lower_wick: 0.2,
        avg_body_share: 0.6,
        wick_bias_score: 0.04,
        bias_label: "BUYER_LEAN".into(),
        note: String::new(),
    };
    upsert_wickbias(&c, "TEST", &snap).unwrap();
    let got = get_wickbias(&c, "TEST").unwrap().unwrap();
    assert!((got.wick_bias_score - 0.04).abs() < 1e-9);
    assert_eq!(got.bias_label, "BUYER_LEAN");
}

#[test]
fn wickbias_compute_insufficient() {
    let snap = compute_wickbias_snapshot("X", "2026-04-15", &[]);
    assert_eq!(snap.bias_label, "INSUFFICIENT_DATA");
}

#[test]
fn wickbias_compute_with_bars() {
    let bars = synthetic_up_trend_bars();
    let snap = compute_wickbias_snapshot("X", "2026-04-15", &bars);
    assert_ne!(snap.bias_label, "INSUFFICIENT_DATA");
    let total = snap.avg_upper_wick + snap.avg_lower_wick + snap.avg_body_share;
    assert!(
        (total - 1.0).abs() < 1e-6,
        "wick + body should sum to 1, got {}",
        total
    );
}

#[test]
fn volofvol_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = VolOfVolSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 233,
        mean_rv20: 0.012,
        stdev_rv20: 0.003,
        min_rv20: 0.008,
        max_rv20: 0.018,
        latest_rv20: 0.013,
        cv_rv20: 0.25,
        cv_label: "MODERATE".into(),
        note: String::new(),
    };
    upsert_volofvol(&c, "TEST", &snap).unwrap();
    let got = get_volofvol(&c, "TEST").unwrap().unwrap();
    assert!((got.cv_rv20 - 0.25).abs() < 1e-9);
    assert_eq!(got.cv_label, "MODERATE");
}

#[test]
fn volofvol_compute_insufficient() {
    let snap = compute_volofvol_snapshot("X", "2026-04-15", &[]);
    assert_eq!(snap.cv_label, "INSUFFICIENT_DATA");
}

// ── Round 26 tests ──

#[test]
fn calmar_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = CalmarRatioSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 252,
        total_return_pct: 25.0,
        annualized_return_pct: 25.0,
        max_drawdown_pct: 10.0,
        calmar_ratio: 2.5,
        calmar_label: "GOOD".into(),
        note: String::new(),
    };
    upsert_calmar(&c, "TEST", &snap).unwrap();
    let got = get_calmar(&c, "TEST").unwrap().unwrap();
    assert!((got.calmar_ratio - 2.5).abs() < 1e-9);
    assert_eq!(got.calmar_label, "GOOD");
}

#[test]
fn calmar_compute_insufficient() {
    let snap = compute_calmar_snapshot("X", "2026-04-15", &[]);
    assert_eq!(snap.calmar_label, "INSUFFICIENT_DATA");
}

#[test]
fn calmar_compute_positive() {
    let bars = synthetic_up_trend_bars();
    let snap = compute_calmar_snapshot("T", "2026-04-15", &bars);
    assert!(
        snap.annualized_return_pct > 0.0,
        "rising series should have positive return"
    );
    assert_eq!(snap.calmar_label, "EXCELLENT");
}

#[test]
fn ulcer_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = UlcerIndexSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 252,
        ulcer_index: 4.5,
        mean_drawdown_pct: -2.1,
        max_drawdown_pct: -8.0,
        pct_in_drawdown: 70.0,
        annualized_return_pct: 15.0,
        martin_ratio: 3.33,
        ulcer_label: "MILD".into(),
        note: String::new(),
    };
    upsert_ulcer(&c, "TEST", &snap).unwrap();
    let got = get_ulcer(&c, "TEST").unwrap().unwrap();
    assert!((got.ulcer_index - 4.5).abs() < 1e-9);
    assert_eq!(got.ulcer_label, "MILD");
}

#[test]
fn ulcer_compute_insufficient() {
    let snap = compute_ulcer_snapshot("X", "2026-04-15", &[]);
    assert_eq!(snap.ulcer_label, "INSUFFICIENT_DATA");
}

#[test]
fn ulcer_compute_rising() {
    let bars = synthetic_up_trend_bars();
    let snap = compute_ulcer_snapshot("T", "2026-04-15", &bars);
    assert!(
        snap.ulcer_index < 1.0,
        "steadily rising series should have very low ulcer"
    );
    assert_eq!(snap.ulcer_label, "LOW_PAIN");
}

#[test]
fn varratio_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = VarianceRatioSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 252,
        vr_2: 1.05,
        vr_5: 1.02,
        vr_10: 0.98,
        vr_20: 0.95,
        z_stat_2: 0.5,
        z_stat_5: 0.2,
        rw_label: "RANDOM_WALK".into(),
        note: String::new(),
    };
    upsert_varratio(&c, "TEST", &snap).unwrap();
    let got = get_varratio(&c, "TEST").unwrap().unwrap();
    assert!((got.vr_5 - 1.02).abs() < 1e-9);
    assert_eq!(got.rw_label, "RANDOM_WALK");
}

#[test]
fn varratio_compute_insufficient() {
    let snap = compute_varratio_snapshot("X", "2026-04-15", &[]);
    assert_eq!(snap.rw_label, "INSUFFICIENT_DATA");
}

#[test]
fn varratio_compute_random() {
    let bars = synthetic_up_trend_bars();
    let snap = compute_varratio_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.rw_label, "INSUFFICIENT_DATA");
}

#[test]
fn amihud_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = AmihudIlliqSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 252,
        mean_illiq: 0.05,
        median_illiq: 0.04,
        illiq_90th: 0.12,
        avg_dollar_volume: 5e7,
        illiq_label: "LIQUID".into(),
        note: String::new(),
    };
    upsert_amihud(&c, "TEST", &snap).unwrap();
    let got = get_amihud(&c, "TEST").unwrap().unwrap();
    assert!((got.mean_illiq - 0.05).abs() < 1e-9);
    assert_eq!(got.illiq_label, "LIQUID");
}

#[test]
fn amihud_compute_insufficient() {
    let snap = compute_amihud_snapshot("X", "2026-04-15", &[]);
    assert_eq!(snap.illiq_label, "INSUFFICIENT_DATA");
}

#[test]
fn amihud_compute_liquid() {
    let bars = synthetic_up_trend_bars();
    let snap = compute_amihud_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.illiq_label, "INSUFFICIENT_DATA");
    assert!(snap.avg_dollar_volume > 0.0);
}

#[test]
fn jbnorm_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = JarqueBeraSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 252,
        skewness: 0.1,
        excess_kurtosis: 0.5,
        jb_statistic: 3.0,
        jb_pvalue: (-1.5_f64).exp(),
        normal_label: "NORMAL".into(),
        note: String::new(),
    };
    upsert_jbnorm(&c, "TEST", &snap).unwrap();
    let got = get_jbnorm(&c, "TEST").unwrap().unwrap();
    assert!((got.jb_statistic - 3.0).abs() < 1e-9);
    assert_eq!(got.normal_label, "NORMAL");
}

#[test]
fn jbnorm_compute_insufficient() {
    let snap = compute_jbnorm_snapshot("X", "2026-04-15", &[]);
    assert_eq!(snap.normal_label, "INSUFFICIENT_DATA");
}

#[test]
fn jbnorm_pvalue_chi2() {
    let bars = synthetic_up_trend_bars();
    let snap = compute_jbnorm_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.normal_label, "INSUFFICIENT_DATA");
    assert!(
        snap.jb_pvalue >= 0.0 && snap.jb_pvalue <= 1.0,
        "p-value must be in [0,1]"
    );
    let expected_p = (-snap.jb_statistic / 2.0).exp();
    assert!(
        (snap.jb_pvalue - expected_p).abs() < 1e-12,
        "p = exp(-JB/2) for chi²(2)"
    );
}

// ── Round 27 tests ──

#[test]
fn omega_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = OmegaRatioSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 252,
        gains_sum: 0.35,
        losses_sum: 0.30,
        gain_days: 135,
        loss_days: 117,
        omega_ratio: 1.1667,
        win_rate_pct: 53.57,
        omega_label: "GOOD".into(),
        note: String::new(),
    };
    upsert_omega(&c, "TEST", &snap).unwrap();
    let got = get_omega(&c, "TEST").unwrap().unwrap();
    assert!((got.omega_ratio - 1.1667).abs() < 1e-9);
    assert_eq!(got.omega_label, "GOOD");
}

#[test]
fn omega_compute_insufficient() {
    let snap = compute_omega_snapshot("X", "2026-04-15", &[]);
    assert_eq!(snap.omega_label, "INSUFFICIENT_DATA");
}

#[test]
fn omega_compute_rising() {
    let bars = synthetic_up_trend_bars();
    let snap = compute_omega_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.omega_label, "INSUFFICIENT_DATA");
    assert!(snap.gain_days > 0);
    // monotone rising → losses_sum is ~0 → omega is very large or infinite
    assert!(snap.omega_ratio > 10.0 || !snap.omega_ratio.is_finite());
    let total = (snap.gain_days + snap.loss_days) as f64;
    if total > 0.0 {
        let expected_win = snap.gain_days as f64 / total * 100.0;
        assert!((snap.win_rate_pct - expected_win).abs() < 1e-9);
    }
}

#[test]
fn dfa_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = DetrendedFluctuationSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 252,
        alpha: 0.52,
        num_scales: 9,
        r_squared: 0.94,
        dfa_label: "RANDOM_WALK".into(),
        note: String::new(),
    };
    upsert_dfa(&c, "TEST", &snap).unwrap();
    let got = get_dfa(&c, "TEST").unwrap().unwrap();
    assert!((got.alpha - 0.52).abs() < 1e-9);
    assert_eq!(got.dfa_label, "RANDOM_WALK");
}

#[test]
fn dfa_compute_insufficient() {
    let snap = compute_dfa_snapshot("X", "2026-04-15", &[]);
    assert_eq!(snap.dfa_label, "INSUFFICIENT_DATA");
}

#[test]
fn dfa_compute_produces_alpha() {
    // Synthetic bars (60) are too few for DFA (needs ≥100 returns).
    // Build a longer deterministic series so DFA runs and returns a usable alpha.
    let mut bars: Vec<HistoricalPriceRow> = Vec::with_capacity(300);
    let mut p: f64 = 100.0;
    for i in 0..300 {
        // Tiny alternating step so returns are non-zero but not trending
        let step = if i % 2 == 0 { 0.3 } else { -0.2 };
        p += step;
        bars.push(HistoricalPriceRow {
            date: format!("2024-{:02}-{:02}", (i / 30) + 1, (i % 30) + 1),
            open: p,
            high: p + 0.1,
            low: p - 0.1,
            close: p,
            adj_close: p,
            volume: 1_000_000.0,
            change: step,
            change_pct: step / (p - step) * 100.0,
        });
    }
    let snap = compute_dfa_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.dfa_label, "INSUFFICIENT_DATA");
    assert!(snap.alpha.is_finite());
    assert!(snap.num_scales >= 4);
}

#[test]
fn burke_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = BurkeRatioSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 252,
        annualized_return_pct: 18.5,
        dd_event_count: 4,
        sum_sq_drawdowns: 200.0,
        worst_event_dd_pct: 10.0,
        burke_ratio: 1.308,
        burke_label: "GOOD".into(),
        note: String::new(),
    };
    upsert_burke(&c, "TEST", &snap).unwrap();
    let got = get_burke(&c, "TEST").unwrap().unwrap();
    assert_eq!(got.dd_event_count, 4);
    assert_eq!(got.burke_label, "GOOD");
}

#[test]
fn burke_compute_insufficient() {
    let snap = compute_burke_snapshot("X", "2026-04-15", &[]);
    assert_eq!(snap.burke_label, "INSUFFICIENT_DATA");
}

#[test]
fn burke_compute_no_drawdowns() {
    let bars = synthetic_up_trend_bars();
    let snap = compute_burke_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.burke_label, "INSUFFICIENT_DATA");
    assert_eq!(
        snap.dd_event_count, 0,
        "monotone-rising series has no drawdown events"
    );
    assert!(snap.annualized_return_pct > 0.0);
    assert_eq!(snap.burke_label, "EXCELLENT");
}

#[test]
fn monthseas_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let mut hit = [50.0_f64; 12];
    hit[0] = 80.0;
    hit[8] = 25.0;
    let snap = MonthlySeasonalitySnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 2520,
        years_covered: 10,
        month_hit_pct: hit,
        month_mean_ret_pct: [0.0; 12],
        best_month_idx: 0,
        worst_month_idx: 8,
        best_month_hit_pct: 80.0,
        worst_month_hit_pct: 25.0,
        season_label: "STRONG_SEASONAL".into(),
        note: String::new(),
    };
    upsert_monthseas(&c, "TEST", &snap).unwrap();
    let got = get_monthseas(&c, "TEST").unwrap().unwrap();
    assert_eq!(got.best_month_idx, 0);
    assert_eq!(got.worst_month_idx, 8);
    assert_eq!(got.season_label, "STRONG_SEASONAL");
}

#[test]
fn monthseas_compute_insufficient() {
    let snap = compute_monthseas_snapshot("X", "2026-04-15", &[]);
    assert_eq!(snap.season_label, "INSUFFICIENT_DATA");
}

#[test]
fn monthseas_compute_dated_series() {
    // 4 years × 12 months × 21 days = 1008 bars. Deterministic monthly trajectory.
    let mut bars: Vec<HistoricalPriceRow> = Vec::with_capacity(1008);
    let mut p: f64 = 100.0;
    for year in 2022..=2025 {
        for month in 1..=12u32 {
            let step_per_day = if (month % 2) == 1 { 0.05 } else { -0.02 };
            for day in 1..=21u32 {
                p += step_per_day;
                bars.push(HistoricalPriceRow {
                    date: format!("{year:04}-{month:02}-{day:02}"),
                    open: p,
                    high: p + 0.1,
                    low: p - 0.1,
                    close: p,
                    adj_close: p,
                    volume: 1_000_000.0,
                    change: step_per_day,
                    change_pct: step_per_day / (p - step_per_day) * 100.0,
                });
            }
        }
    }
    let snap = compute_monthseas_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.season_label, "INSUFFICIENT_DATA");
    assert!(snap.years_covered >= 3);
    // Odd-index months should have >50% hit rate; even months <50%
    assert!(
        snap.month_hit_pct[0] > 50.0,
        "Jan should trend up in synthetic data"
    );
    assert!(
        snap.month_hit_pct[1] < 50.0,
        "Feb should trend down in synthetic data"
    );
}

#[test]
fn rollsprd_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = RollSpreadSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 252,
        first_lag_cov: -0.015,
        mean_price: 150.0,
        implicit_spread: 0.245,
        implicit_spread_bps: 16.33,
        roll_label: "NORMAL".into(),
        note: String::new(),
    };
    upsert_rollsprd(&c, "TEST", &snap).unwrap();
    let got = get_rollsprd(&c, "TEST").unwrap().unwrap();
    assert!((got.implicit_spread_bps - 16.33).abs() < 1e-9);
    assert_eq!(got.roll_label, "NORMAL");
}

#[test]
fn rollsprd_compute_insufficient() {
    let snap = compute_rollsprd_snapshot("X", "2026-04-15", &[]);
    assert_eq!(snap.roll_label, "INSUFFICIENT_DATA");
}

#[test]
fn rollsprd_compute_trending_rejects() {
    // Monotone rising → Δp has positive autocorrelation → first-lag cov > 0 → invalid
    let bars = synthetic_up_trend_bars();
    let snap = compute_rollsprd_snapshot("T", "2026-04-15", &bars);
    assert_eq!(snap.roll_label, "INVALID_POSITIVE_COV");
    assert!(snap.first_lag_cov >= 0.0);
}

#[test]
fn rollsprd_compute_bouncing_valid() {
    // Construct alternating up/down tick series: Δp alternates sign →
    // first-lag cov should be negative → spread computable.
    let mut bars: Vec<HistoricalPriceRow> = Vec::with_capacity(60);
    let base: f64 = 100.0;
    for i in 0..60 {
        let offset = if i % 2 == 0 { 0.0 } else { 0.10 };
        let p = base + offset;
        bars.push(HistoricalPriceRow {
            date: format!("2024-{:02}-{:02}", (i / 30) + 1, (i % 30) + 1),
            open: p,
            high: p + 0.01,
            low: p - 0.01,
            close: p,
            adj_close: p,
            volume: 1_000_000.0,
            change: 0.0,
            change_pct: 0.0,
        });
    }
    let snap = compute_rollsprd_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.roll_label, "INSUFFICIENT_DATA");
    assert_ne!(snap.roll_label, "INVALID_POSITIVE_COV");
    assert!(snap.implicit_spread > 0.0);
    assert!(snap.implicit_spread_bps > 0.0);
}

// ── Round 28 tests ──

fn synthetic_ohlc_bars_150() -> Vec<HistoricalPriceRow> {
    // 150 dated bars with non-trivial intraday ranges and a mild
    // drift — enough for all Round 28 vol estimators (>=30) and
    // CVAR (>=100).
    (0..150)
        .map(|i| {
            let close = 100.0 + (i as f64) * 0.3;
            let open = close - 0.2;
            let range = 1.0 + ((i % 7) as f64) * 0.1;
            let high = close + range * 0.5;
            let low = close - range * 0.5;
            let month = 1 + (i / 25) as u32;
            let day = 1 + (i % 25) as u32;
            HistoricalPriceRow {
                date: format!("2024-{:02}-{:02}", month, day),
                open,
                high,
                low,
                close,
                adj_close: close,
                volume: 1_000_000.0,
                change: 0.0,
                change_pct: 0.0,
            }
        })
        .collect()
}

#[test]
fn parkinson_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = ParkinsonVolSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 200,
        daily_vol_pct: 1.2,
        annualized_vol_pct: 19.0,
        mean_hl_log_ratio: 0.01,
        vol_label: "LOW".into(),
        note: String::new(),
    };
    upsert_parkinson(&c, "TEST", &snap).unwrap();
    let back = get_parkinson(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.vol_label, "LOW");
    assert!((back.annualized_vol_pct - 19.0).abs() < 1e-9);
}

#[test]
fn parkinson_compute_insufficient() {
    let snap = compute_parkinson_snapshot("T", "2026-04-15", &[]);
    assert_eq!(snap.vol_label, "INSUFFICIENT_DATA");
}

#[test]
fn parkinson_compute_rising() {
    let bars = synthetic_ohlc_bars_150();
    let snap = compute_parkinson_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.vol_label, "INSUFFICIENT_DATA");
    assert!(snap.annualized_vol_pct > 0.0);
    assert!(snap.daily_vol_pct > 0.0);
    assert!(snap.bars_used >= 30);
}

#[test]
fn gkvol_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = GarmanKlassVolSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 200,
        daily_vol_pct: 1.0,
        annualized_vol_pct: 15.9,
        range_component: 0.0002,
        co_component: 0.00005,
        vol_label: "LOW".into(),
        note: String::new(),
    };
    upsert_gkvol(&c, "TEST", &snap).unwrap();
    let back = get_gkvol(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.vol_label, "LOW");
    assert!((back.annualized_vol_pct - 15.9).abs() < 1e-9);
}

#[test]
fn gkvol_compute_insufficient() {
    let snap = compute_gkvol_snapshot("T", "2026-04-15", &[]);
    assert_eq!(snap.vol_label, "INSUFFICIENT_DATA");
}

#[test]
fn gkvol_compute_rising() {
    let bars = synthetic_ohlc_bars_150();
    let snap = compute_gkvol_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.vol_label, "INSUFFICIENT_DATA");
    assert!(snap.annualized_vol_pct > 0.0);
    assert!(snap.range_component > 0.0);
    assert!(snap.bars_used >= 30);
}

#[test]
fn rsvol_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = RogersSatchellVolSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 150,
        daily_vol_pct: 1.1,
        annualized_vol_pct: 17.5,
        vol_label: "LOW".into(),
        note: String::new(),
    };
    upsert_rsvol(&c, "TEST", &snap).unwrap();
    let back = get_rsvol(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.vol_label, "LOW");
    assert!((back.annualized_vol_pct - 17.5).abs() < 1e-9);
}

#[test]
fn rsvol_compute_insufficient() {
    let snap = compute_rsvol_snapshot("T", "2026-04-15", &[]);
    assert_eq!(snap.vol_label, "INSUFFICIENT_DATA");
}

#[test]
fn rsvol_compute_rising() {
    let bars = synthetic_ohlc_bars_150();
    let snap = compute_rsvol_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.vol_label, "INSUFFICIENT_DATA");
    assert!(snap.annualized_vol_pct >= 0.0);
    assert!(snap.bars_used >= 30);
}

#[test]
fn cvar_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = CVaRSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 200,
        var_5pct_ret_pct: -2.1,
        cvar_5pct_ret_pct: -3.0,
        var_1pct_ret_pct: -4.1,
        cvar_1pct_ret_pct: -5.5,
        tail_days_5pct: 10,
        tail_days_1pct: 2,
        cvar_label: "MODERATE".into(),
        note: String::new(),
    };
    upsert_cvar(&c, "TEST", &snap).unwrap();
    let back = get_cvar(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.cvar_label, "MODERATE");
    assert!((back.cvar_5pct_ret_pct - (-3.0)).abs() < 1e-9);
}

#[test]
fn cvar_compute_insufficient() {
    let snap = compute_cvar_snapshot("T", "2026-04-15", &[]);
    assert_eq!(snap.cvar_label, "INSUFFICIENT_DATA");
}

#[test]
fn cvar_compute_tailed() {
    // Construct 150 bars with a deterministic fat left tail: most
    // moves are +0.1%, but every 10th bar drops 5%.
    let mut bars: Vec<HistoricalPriceRow> = Vec::with_capacity(150);
    let mut price: f64 = 100.0;
    for i in 0..150 {
        let new_price = if i % 10 == 9 {
            price * 0.95
        } else {
            price * 1.001
        };
        let month = 1 + (i / 25) as u32;
        let day = 1 + (i % 25) as u32;
        bars.push(HistoricalPriceRow {
            date: format!("2024-{:02}-{:02}", month, day),
            open: price,
            high: new_price.max(price) + 0.1,
            low: new_price.min(price) - 0.1,
            close: new_price,
            adj_close: new_price,
            volume: 1_000_000.0,
            change: 0.0,
            change_pct: 0.0,
        });
        price = new_price;
    }
    let snap = compute_cvar_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.cvar_label, "INSUFFICIENT_DATA");
    assert!(snap.var_5pct_ret_pct < 0.0);
    assert!(snap.cvar_5pct_ret_pct <= snap.var_5pct_ret_pct);
    assert!(snap.tail_days_5pct >= 1);
}

#[test]
fn doweffect_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = DayOfWeekEffectSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 250,
        weeks_covered: 52,
        dow_hit_pct: [55.0, 48.0, 52.0, 60.0, 57.0],
        dow_mean_ret_pct: [0.1, -0.05, 0.02, 0.15, 0.1],
        dow_sample_count: [52, 50, 51, 50, 50],
        best_dow_idx: 3,
        worst_dow_idx: 1,
        best_dow_hit_pct: 60.0,
        worst_dow_hit_pct: 48.0,
        dow_label: "MILD_EFFECT".into(),
        note: String::new(),
    };
    upsert_doweffect(&c, "TEST", &snap).unwrap();
    let back = get_doweffect(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.dow_label, "MILD_EFFECT");
    assert_eq!(back.best_dow_idx, 3);
}

#[test]
fn doweffect_compute_insufficient() {
    let snap = compute_doweffect_snapshot("T", "2026-04-15", &[]);
    assert_eq!(snap.dow_label, "INSUFFICIENT_DATA");
}

#[test]
fn doweffect_compute_dated_series() {
    // 2 years of real-calendar weekdays starting 2022-01-03 (Monday).
    // Walk calendar, skip weekends. Inject a "Friday rally" pattern:
    // Fridays close above open; Mondays close below.
    use chrono::{Datelike, Duration, NaiveDate};
    let mut d = NaiveDate::from_ymd_opt(2022, 1, 3).unwrap();
    let mut bars: Vec<HistoricalPriceRow> = Vec::new();
    let mut i = 0_i32;
    while bars.len() < 500 {
        let w = d.weekday().num_days_from_monday();
        if w < 5 {
            let open = 100.0 + (i as f64) * 0.01;
            let close = match w {
                0 => open * 0.995, // Monday: down
                4 => open * 1.010, // Friday: up
                _ => open * (1.0 + ((i % 3) as f64 - 1.0) * 0.001),
            };
            bars.push(HistoricalPriceRow {
                date: d.format("%Y-%m-%d").to_string(),
                open,
                high: open.max(close) + 0.1,
                low: open.min(close) - 0.1,
                close,
                adj_close: close,
                volume: 1_000_000.0,
                change: 0.0,
                change_pct: 0.0,
            });
            i += 1;
        }
        d = d + Duration::days(1);
    }
    let snap = compute_doweffect_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.dow_label, "INSUFFICIENT_DATA");
    // Friday (idx 4) should dominate hit rate; Monday (idx 0) should trail.
    assert_eq!(snap.best_dow_idx, 4);
    assert_eq!(snap.worst_dow_idx, 0);
    assert!(snap.best_dow_hit_pct > snap.worst_dow_hit_pct);
    assert!(snap.dow_sample_count.iter().all(|c| *c >= 10));
}

// ── Round 29 tests ──

fn synthetic_oscillating_bars_150() -> Vec<HistoricalPriceRow> {
    // 150 bars alternating up/down ~0.5% so we have both positive and
    // negative log returns (required for KELLYF and RUNSTEST), plus
    // non-trivial variance for LJUNGB.
    let mut out = Vec::with_capacity(150);
    let mut price: f64 = 100.0;
    for i in 0..150 {
        let next = if i % 2 == 0 {
            price * 1.005
        } else {
            price * 0.995
        };
        let month = 1 + (i / 25) as u32;
        let day = 1 + (i % 25) as u32;
        out.push(HistoricalPriceRow {
            date: format!("2024-{:02}-{:02}", month, day),
            open: price,
            high: next.max(price) + 0.1,
            low: next.min(price) - 0.1,
            close: next,
            adj_close: next,
            volume: 1_000_000.0,
            change: 0.0,
            change_pct: 0.0,
        });
        price = next;
    }
    out
}

fn synthetic_drops_bars_150() -> Vec<HistoricalPriceRow> {
    // 150 bars with mostly small gains + periodic 5% drops so Sterling
    // sees real drawdown events.
    let mut out = Vec::with_capacity(150);
    let mut price: f64 = 100.0;
    for i in 0..150 {
        let next = if i % 10 == 9 {
            price * 0.95
        } else {
            price * 1.001
        };
        let month = 1 + (i / 25) as u32;
        let day = 1 + (i % 25) as u32;
        out.push(HistoricalPriceRow {
            date: format!("2024-{:02}-{:02}", month, day),
            open: price,
            high: next.max(price) + 0.1,
            low: next.min(price) - 0.1,
            close: next,
            adj_close: next,
            volume: 1_000_000.0,
            change: 0.0,
            change_pct: 0.0,
        });
        price = next;
    }
    out
}

#[test]
fn sterling_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = SterlingRatioSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 253,
        annualized_return_pct: 12.0,
        worst_n: 5,
        dd_event_count: 9,
        mean_worst_dd_pct: 8.0,
        sterling_ratio: 1.5,
        sterling_label: "EXCELLENT".into(),
        note: String::new(),
    };
    upsert_sterling(&c, "TEST", &snap).unwrap();
    let back = get_sterling(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.sterling_label, "EXCELLENT");
    assert!((back.sterling_ratio - 1.5).abs() < 1e-9);
}

#[test]
fn sterling_compute_insufficient() {
    let snap = compute_sterling_snapshot("T", "2026-04-15", &[]);
    assert_eq!(snap.sterling_label, "INSUFFICIENT_DATA");
}

#[test]
fn sterling_compute_with_drawdowns() {
    let bars = synthetic_drops_bars_150();
    let snap = compute_sterling_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.sterling_label, "INSUFFICIENT_DATA");
    assert!(snap.dd_event_count >= 1);
    assert!(snap.mean_worst_dd_pct > 0.0);
    assert!(snap.worst_n <= 5);
}

#[test]
fn kellyf_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = KellyFractionSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 200,
        win_rate: 0.55,
        loss_rate: 0.45,
        avg_win_pct: 1.2,
        avg_loss_pct: 0.9,
        win_loss_ratio: 1.333,
        kelly_fraction: 0.213,
        half_kelly: 0.1065,
        kelly_label: "MODERATE".into(),
        note: String::new(),
    };
    upsert_kellyf(&c, "TEST", &snap).unwrap();
    let back = get_kellyf(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.kelly_label, "MODERATE");
    assert!((back.kelly_fraction - 0.213).abs() < 1e-9);
}

#[test]
fn kellyf_compute_insufficient() {
    let snap = compute_kellyf_snapshot("T", "2026-04-15", &[]);
    assert_eq!(snap.kelly_label, "INSUFFICIENT_DATA");
}

#[test]
fn kellyf_compute_mixed() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_kellyf_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.kelly_label, "INSUFFICIENT_DATA");
    assert!(snap.win_rate > 0.0);
    assert!(snap.loss_rate > 0.0);
    assert!(snap.avg_win_pct > 0.0);
    assert!(snap.avg_loss_pct > 0.0);
}

#[test]
fn ljungb_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = LjungBoxSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 252,
        lag_h: 10,
        q_statistic: 22.5,
        p_value: 0.013,
        reject_white_noise: true,
        ljungb_label: "MODERATE_DEP".into(),
        note: String::new(),
    };
    upsert_ljungb(&c, "TEST", &snap).unwrap();
    let back = get_ljungb(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.ljungb_label, "MODERATE_DEP");
    assert_eq!(back.lag_h, 10);
}

#[test]
fn ljungb_compute_insufficient() {
    let snap = compute_ljungb_snapshot("T", "2026-04-15", &[]);
    assert_eq!(snap.ljungb_label, "INSUFFICIENT_DATA");
}

#[test]
fn ljungb_compute_oscillating() {
    // Perfect +0.5%/-0.5% alternation should exhibit strong negative
    // lag-1 autocorrelation → reject white noise at h=10.
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_ljungb_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.ljungb_label, "INSUFFICIENT_DATA");
    assert!(snap.q_statistic > 0.0);
    assert_eq!(snap.lag_h, 10);
    assert!((0.0..=1.0).contains(&snap.p_value));
}

#[test]
fn runstest_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = RunsTestSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 200,
        positive_days: 110,
        negative_days: 90,
        runs_observed: 95,
        runs_expected: 100.0,
        runs_std: 7.0,
        z_statistic: -0.71,
        p_value: 0.48,
        reject_randomness: false,
        runs_label: "RANDOM".into(),
        note: String::new(),
    };
    upsert_runstest(&c, "TEST", &snap).unwrap();
    let back = get_runstest(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.runs_label, "RANDOM");
    assert_eq!(back.positive_days, 110);
}

#[test]
fn runstest_compute_insufficient() {
    let snap = compute_runstest_snapshot("T", "2026-04-15", &[]);
    assert_eq!(snap.runs_label, "INSUFFICIENT_DATA");
}

#[test]
fn runstest_compute_oscillating() {
    // Alternating +/- returns produce maximal runs → large positive
    // z-statistic → ANTI_CLUST (anti-clustering) label.
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_runstest_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.runs_label, "INSUFFICIENT_DATA");
    assert!(snap.positive_days > 0);
    assert!(snap.negative_days > 0);
    assert!(snap.runs_observed >= 2);
    assert!(snap.runs_std >= 0.0);
}

#[test]
fn zeroret_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = ZeroReturnSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 250,
        zero_day_count: 3,
        zero_day_pct: 1.2,
        longest_zero_streak: 2,
        epsilon: 1e-6,
        zero_label: "LIQUID".into(),
        note: String::new(),
    };
    upsert_zeroret(&c, "TEST", &snap).unwrap();
    let back = get_zeroret(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.zero_label, "LIQUID");
    assert!((back.epsilon - 1e-6).abs() < 1e-15);
}

#[test]
fn zeroret_compute_insufficient() {
    let snap = compute_zeroret_snapshot("T", "2026-04-15", &[]);
    assert_eq!(snap.zero_label, "INSUFFICIENT_DATA");
}

#[test]
fn zeroret_compute_liquid_series() {
    // synthetic_ohlc_bars_150 has monotonically increasing close so
    // every log return is ≫ 1e-6 — expect HIGHLY_LIQUID.
    let bars = synthetic_ohlc_bars_150();
    let snap = compute_zeroret_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.zero_label, "INSUFFICIENT_DATA");
    assert_eq!(snap.zero_day_count, 0);
    assert_eq!(snap.longest_zero_streak, 0);
    assert_eq!(snap.zero_label, "HIGHLY_LIQUID");
    assert!((snap.epsilon - 1e-6).abs() < 1e-15);
}

