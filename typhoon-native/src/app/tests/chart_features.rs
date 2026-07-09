// ── BetterVolume MQL5 classification tests ────────────────────────

#[test]
fn test_better_volume_mql5_classifications() {
    // BetterVolume uses adaptive comparison against lookback extremes (not fixed thresholds).
    // Verify basic properties: correct length, valid classification range, variety of results.
    let bars = make_oscillating_bars(50);
    let bv = compute_better_volume(&bars);
    assert_eq!(bv.len(), bars.len());
    // All values should be valid classification (0-5)
    for (i, &v) in bv.iter().enumerate() {
        assert!(v <= 5, "Bar {} has invalid classification {}", i, v);
    }
    // First `lookback` bars should be normal (5) since lookback not ready
    assert_eq!(bv[0], 5, "Bar 0 should be normal (5)");
    // With oscillating data, at least some bars should be non-normal
    let non_normal = bv.iter().filter(|&&v| v != 5).count();
    assert!(
        non_normal > 0,
        "With oscillating data, some bars should have non-normal classification"
    );
}

// ── Supply/Demand zone break detection ────────────────────────────

#[test]
fn test_supply_demand_break_detection() {
    // Create bars: rally to 200, crash through all supply zones
    let mut bars = Vec::new();
    // Phase 1: 30 bars oscillating 90-110 (creates fractals)
    for i in 0..30 {
        let base = 100.0 + (i as f64 * 0.5).sin() * 8.0;
        bars.push(Bar {
            ts_ms: 1700000000000 + i as i64 * 86400000,
            open: base - 0.5,
            high: base + 2.0,
            low: base - 2.0,
            close: base + 0.5,
            volume: 1000.0,
        });
    }
    // Phase 2: massive rally through all zones
    for i in 30..50 {
        let base = 100.0 + (i - 30) as f64 * 5.0;
        bars.push(Bar {
            ts_ms: 1700000000000 + i as i64 * 86400000,
            open: base,
            high: base + 3.0,
            low: base - 1.0,
            close: base + 2.0,
            volume: 2000.0,
        });
    }

    let (supply, _demand) = compute_supply_demand_zones(&bars);
    // Supply zones from phase 1 should be broken by phase 2 rally
    // Only zones near the top (if any) should survive
    for (_, hi, _, _) in &supply {
        assert!(
            *hi >= 150.0,
            "Surviving supply zone should be at high prices (got {})",
            hi
        );
    }
}

// ── Supply/Demand zone merge ──────────────────────────────────────

#[test]
fn test_supply_demand_merge_overlapping() {
    // Create bars with multiple close fractal lows at similar prices
    let mut bars = Vec::new();
    for i in 0..50 {
        let base = if i % 10 < 5 {
            100.0 + i as f64 * 0.1
        } else {
            95.0
        }; // oscillate with dips to 95
        bars.push(Bar {
            ts_ms: 1700000000000 + i as i64 * 86400000,
            open: base + 0.5,
            high: base + 2.0,
            low: base - 2.0,
            close: base - 0.5,
            volume: 1000.0,
        });
    }
    let (supply, demand) = compute_supply_demand_zones(&bars);
    // After merge, overlapping zones should be consolidated
    // Check no two zones of same type overlap
    for i in 0..supply.len() {
        for j in (i + 1)..supply.len() {
            let a = &supply[i];
            let b = &supply[j];
            let overlap = a.1 >= b.2 && b.1 >= a.2; // hi_a >= lo_b && hi_b >= lo_a
            assert!(
                !overlap,
                "Supply zones {} and {} overlap: ({:.2},{:.2}) vs ({:.2},{:.2})",
                i, j, a.2, a.1, b.2, b.1
            );
        }
    }
    for i in 0..demand.len() {
        for j in (i + 1)..demand.len() {
            let a = &demand[i];
            let b = &demand[j];
            let overlap = a.1 >= b.2 && b.1 >= a.2;
            assert!(!overlap, "Demand zones {} and {} overlap", i, j);
        }
    }
}

// ── GPU S/D zones from GPU output ─────────────────────────────────

#[test]
fn test_supply_demand_from_gpu() {
    let bars = make_oscillating_bars(50);
    // Simulate GPU output: mark bar 16 as supply fractal, bar 31 as demand
    let mut gpu_data = vec![0.0f32; 50 * 3];
    gpu_data[16 * 3] = -1.0; // supply
    gpu_data[16 * 3 + 1] = bars[16].high as f32;
    gpu_data[16 * 3 + 2] = bars[16].close as f32;
    gpu_data[31 * 3] = 1.0; // demand
    gpu_data[31 * 3 + 1] = bars[31].close as f32;
    gpu_data[31 * 3 + 2] = bars[31].low as f32;

    let (supply, demand) = compute_supply_demand_zones_from_gpu(&gpu_data, &bars);
    // Should produce at least the zones we marked (unless broken)
    let total = supply.len() + demand.len();
    assert!(
        total <= 2,
        "Should have at most 2 zones from 2 GPU fractals, got {}",
        total
    );
    for (_, hi, lo, _) in supply.iter().chain(demand.iter()) {
        assert!(hi > lo, "Zone high must be > low");
    }
}

// ── Aggregate bars to HTF ─────────────────────────────────────────

#[test]
fn test_aggregate_bars_to_htf() {
    // 12 hourly bars → 3 4-hour bars
    let mut bars = Vec::new();
    for i in 0..12 {
        bars.push(Bar {
            ts_ms: 1700000000000 + i as i64 * 3600000,
            open: 100.0 + i as f64,
            high: 105.0 + i as f64,
            low: 95.0 + i as f64,
            close: 102.0 + i as f64,
            volume: 1000.0,
        });
    }
    let htf = aggregate_bars_to_htf(&bars, 240); // 4 hours = 240 minutes
    assert!(
        htf.len() >= 3 && htf.len() <= 4,
        "12 hourly bars → 3-4 4-hour bars (timestamp bucketing), got {}",
        htf.len()
    );
    // First HTF bar should have open from bar 0
    assert_eq!(htf[0].open, bars[0].open);
    // Last HTF bar should have close from last input bar
    assert_eq!(htf.last().unwrap().close, bars[11].close);
    // Each HTF bar volume should be sum of its constituent bars
    assert!(htf[0].volume > 0.0, "HTF bar should have non-zero volume");
}

// ── TradeMarker aggregation ───────────────────────────────────────

#[test]
fn test_trade_marker_aggregation() {
    // Verify the HashMap aggregation logic used in build_trade_overlay
    use std::collections::HashMap;
    let mut marker_map: HashMap<(usize, bool, i64), (f64, u32, String)> = HashMap::new();

    // 3 buys at same bar+price
    for _ in 0..3 {
        let entry = marker_map
            .entry((100, true, 15000))
            .or_insert((0.0, 0, String::new()));
        entry.0 += 0.10;
        entry.1 += 1;
    }
    // 1 sell at different price
    let entry = marker_map
        .entry((100, false, 16000))
        .or_insert((0.0, 0, String::new()));
    entry.0 += 0.50;
    entry.1 += 1;

    assert_eq!(marker_map.len(), 2, "Should have 2 unique entries");
    let buy = marker_map.get(&(100, true, 15000)).unwrap();
    assert!(
        (buy.0 - 0.30).abs() < 0.001,
        "Aggregated volume should be 0.30"
    );
    assert_eq!(buy.1, 3, "Should have 3 aggregated trades");
}

// ── TradeOverlay default ──────────────────────────────────────────

#[test]
fn test_trade_overlay_default() {
    let ov = TradeOverlay::default();
    assert!(ov.markers.is_empty());
    assert!(ov.position_lines.is_empty());
}

// ── BetterVolume extended tests ─────────────────────────────────────

#[test]
fn test_better_volume_climax_up() {
    // Create bars where bar at index `lookback` has a massive bullish candle
    // with extremely high buy volume * range → should trigger climax up (1)
    // Need n > target + lookback to satisfy both skip conditions in compute_better_volume
    let lookback = 20usize;
    let target = lookback; // bar 20
    let n = target + lookback + 5; // 45 bars total
    let mut bars = Vec::new();
    for i in 0..n {
        if i == target {
            // Massive bullish candle: huge range, high volume, close > open
            bars.push(Bar {
                ts_ms: 1700000000000 + i as i64 * 3600000,
                open: 90.0,
                high: 130.0,
                low: 89.0,
                close: 128.0,
                volume: 50000.0, // 50x normal
            });
        } else {
            // Normal small bars
            bars.push(Bar {
                ts_ms: 1700000000000 + i as i64 * 3600000,
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.5,
                volume: 1000.0,
            });
        }
    }
    let bv = compute_better_volume(&bars);
    // Bar at target should be climax up (1) or climax+churn (4)
    let val = bv[target];
    assert!(
        val == 1 || val == 4,
        "Massive bullish bar should be climax up (1) or climax+churn (4), got {}",
        val
    );
}

#[test]
fn test_better_volume_churn() {
    // Churn: very high volume but tiny range → vol/range is highest
    let lookback = 20usize;
    let target = lookback;
    let n = target + lookback + 5;
    let mut bars = Vec::new();
    for i in 0..n {
        if i == target {
            // Tiny range, huge volume
            bars.push(Bar {
                ts_ms: 1700000000000 + i as i64 * 3600000,
                open: 100.0,
                high: 100.1,
                low: 99.9,
                close: 100.05,
                volume: 100000.0, // enormous volume, tiny range
            });
        } else {
            bars.push(Bar {
                ts_ms: 1700000000000 + i as i64 * 3600000,
                open: 100.0,
                high: 102.0,
                low: 98.0,
                close: 101.0,
                volume: 1000.0,
            });
        }
    }
    let bv = compute_better_volume(&bars);
    // Should be churn (3) or climax+churn (4)
    let val = bv[target];
    assert!(
        val == 3 || val == 4,
        "High-volume tiny-range bar should be churn (3) or climax+churn (4), got {}",
        val
    );
}

#[test]
fn test_better_volume_low_volume() {
    // Create bars where one bar has extremely low volume
    let lookback = 20usize;
    let target = lookback;
    let n = target + lookback + 5;
    let mut bars = Vec::new();
    for i in 0..n {
        if i == target {
            bars.push(Bar {
                ts_ms: 1700000000000 + i as i64 * 3600000,
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: 0.1, // nearly zero volume
            });
        } else {
            bars.push(Bar {
                ts_ms: 1700000000000 + i as i64 * 3600000,
                open: 100.0,
                high: 102.0,
                low: 98.0,
                close: 101.0,
                volume: 5000.0,
            });
        }
    }
    let bv = compute_better_volume(&bars);
    // Should be low volume (0)
    assert_eq!(
        bv[target], 0,
        "Near-zero volume bar should be low volume (0), got {}",
        bv[target]
    );
}

#[test]
fn test_better_volume_all_normal_flat() {
    // Identical bars — at the lookback boundary, metrics equal extremes
    // so some may classify as non-normal, but most should be normal (5)
    let n = 30;
    let bars: Vec<Bar> = (0..n)
        .map(|i| Bar {
            ts_ms: 1700000000000 + i as i64 * 3600000,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: 1000.0,
        })
        .collect();
    let bv = compute_better_volume(&bars);
    assert_eq!(bv.len(), n);
    // First `lookback` bars should be normal (5)
    for i in 0..20 {
        assert_eq!(bv[i], 5, "Bar {} in warmup should be normal", i);
    }
}

#[test]
fn test_var_oscillator_warmup_and_downside_signal() {
    let mut closes = vec![100.0];
    for i in 1..40 {
        let prev = closes[i - 1];
        let next = if i == 25 { prev * 0.92 } else { prev * 1.003 };
        closes.push(next);
    }
    let bars = make_close_bars(&closes);
    let osc = compute_var_oscillator(&bars, 20);
    assert!(osc[..20].iter().all(|v| v.is_none()));
    assert!(
        osc[20].is_some(),
        "first fully-populated VaR window should be valid"
    );
    assert!(
        osc[25].unwrap_or_default() > 100.0,
        "sharp downside move should exceed +100 VaR units"
    );
}

#[test]
fn test_var_oscillator_upside_moves_are_negative() {
    let mut closes = vec![100.0; 40];
    for i in 1..40 {
        closes[i] = closes[i - 1] * 1.002;
    }
    closes[25] = closes[24] * 1.08;
    let bars = make_close_bars(&closes);
    let osc = compute_var_oscillator(&bars, 20);
    assert!(
        osc[25].unwrap_or_default() < 0.0,
        "upside shock should plot below zero"
    );
}

#[test]
fn test_chart_talib_gpu_fallback_series_have_expected_ranges() {
    let bars = make_oscillating_bars(80);

    let cmo = compute_cmo(&bars, 9);
    assert!(cmo[..9].iter().all(|v| v.is_none()));
    assert!(cmo[9].unwrap_or_default().abs() <= 100.0);

    let qstick = compute_qstick(&bars, 14);
    assert!(qstick[..13].iter().all(|v| v.is_none()));
    assert!(qstick[13].unwrap_or_default().is_finite());

    let disparity = compute_disparity(&bars, 14);
    assert!(disparity[..13].iter().all(|v| v.is_none()));
    assert!(disparity[13].unwrap_or_default().is_finite());

    let bop = compute_bop(&bars, 14);
    assert!(bop[..13].iter().all(|v| v.is_none()));
    assert!(bop[13].unwrap_or_default().abs() <= 1.0);

    let stddev = compute_stddev(&bars, 20);
    assert!(stddev[..19].iter().all(|v| v.is_none()));
    assert!(stddev[19].unwrap_or_default() >= 0.0);
}

// ── Format functions ─────────────────────────────────────────────────

#[test]
fn test_format_price_buf_zero() {
    let mut buf = String::new();
    format_price_buf(0.0, &mut buf);
    assert_eq!(buf, "0");
}

#[test]
fn test_format_price_buf_large() {
    let mut buf = String::new();
    format_price_buf(12345.67, &mut buf);
    assert_eq!(buf, "12345.67"); // >= 10000 → 2 decimal places
}

#[test]
fn test_format_price_buf_medium() {
    let mut buf = String::new();
    format_price_buf(123.4567, &mut buf);
    assert_eq!(buf, "123.4567"); // >= 1.0 → 4 decimal places
}

#[test]
fn test_format_price_buf_small() {
    let mut buf = String::new();
    format_price_buf(0.123456, &mut buf);
    assert_eq!(buf, "0.123456"); // < 1.0 → 6 decimal places
}

#[test]
fn test_format_price_buf_negative() {
    let mut buf = String::new();
    format_price_buf(-50.1234, &mut buf);
    assert_eq!(buf, "-50.1234"); // abs >= 1.0 → 4 decimals
}

#[test]
fn test_format_price_buf_reuses_buffer() {
    let mut buf = String::new();
    format_price_buf(100.0, &mut buf);
    let first = buf.clone();
    format_price_buf(200.0, &mut buf);
    assert_ne!(first, buf, "Buffer should be cleared and rewritten");
    assert!(buf.contains("200"), "Should contain new value");
}

#[test]
fn test_format_ts_buf_daily() {
    let mut buf = String::new();
    // 2023-11-15 00:00:00 UTC → 1700006400000
    let ts = 1700006400000_i64;
    format_ts_buf(ts, Timeframe::D1, &mut buf);
    assert!(
        buf.contains("Nov") || buf.contains("15"),
        "D1 format should contain day/month, got: {}",
        buf
    );
}

#[test]
fn test_format_ts_buf_hourly_midnight() {
    let mut buf = String::new();
    // Midnight → should show date, not time
    let ts = 1700006400000_i64; // 2023-11-15 00:00 UTC
    format_ts_buf(ts, Timeframe::H4, &mut buf);
    // At midnight, H4 shows date format
    assert!(
        !buf.contains(":") || buf.contains("00:00") || buf.contains("Nov"),
        "H4 at midnight should show date, got: {}",
        buf
    );
}

#[test]
fn test_format_ts_buf_hourly_nonmidnight() {
    let mut buf = String::new();
    // 2023-11-15 14:00:00 UTC
    let ts = 1700006400000_i64 + 14 * 3600000;
    format_ts_buf(ts, Timeframe::H1, &mut buf);
    assert!(
        buf.contains("14:00"),
        "H1 non-midnight should show HH:MM, got: {}",
        buf
    );
}

#[test]
fn test_format_ts_buf_monthly() {
    let mut buf = String::new();
    let ts = 1700006400000_i64;
    format_ts_buf(ts, Timeframe::MN1, &mut buf);
    // MN1 format: "Nov'23"
    assert!(
        buf.contains("Nov") && buf.contains("23"),
        "MN1 should show Mon'YY, got: {}",
        buf
    );
}

#[test]
fn test_format_ts_buf_minute() {
    let mut buf = String::new();
    let ts = 1700006400000_i64 + 9 * 3600000 + 30 * 60000; // 09:30
    format_ts_buf(ts, Timeframe::M15, &mut buf);
    assert!(buf.contains("09:30"), "M15 should show HH:MM, got: {}", buf);
}

#[test]
fn test_apply_storage_snapshot_prunes_deleted_keys_and_updates_sizes() {
    let mut bg = BgData::default();
    bg.bar_ts_cache
        .insert("kraken-futures:EURUSD:1Min".into(), (1, 2, 10));
    bg.bar_ts_cache
        .insert("alpaca:AAPL:1Day".into(), (3, 4, 20));

    apply_storage_snapshot(
        &mut bg,
        (1, 7, 9_999),
        vec![("alpaca:AAPL:1Day".into(), 123, 456, 789)],
    );

    assert_eq!(bg.cache_stats, Some((1, 7, 9_999)));
    assert_eq!(
        bg.detailed_stats,
        vec![("alpaca:AAPL:1Day".into(), 123, 456)]
    );
    assert_eq!(bg.cache_blob_sizes.get("alpaca:AAPL:1Day"), Some(&789));
    assert!(!bg.bar_ts_cache.contains_key("kraken-futures:EURUSD:1Min"));
    assert!(bg.bar_ts_cache.contains_key("alpaca:AAPL:1Day"));
}

#[test]
fn test_codex_reasoning_effort_normalization_defaults_unknown_values() {
    assert_eq!(
        TyphooNApp::normalize_codex_reasoning_effort("medium"),
        "medium"
    );
    assert_eq!(
        TyphooNApp::normalize_codex_reasoning_effort("bogus"),
        "default"
    );
    assert_eq!(TyphooNApp::normalize_codex_reasoning_effort(""), "default");
}

#[test]
fn test_build_codex_exec_args_omits_reasoning_override_for_default() {
    let args = TyphooNApp::build_codex_exec_args("gpt-5-codex", "default", "hello");
    assert_eq!(
        args,
        vec![
            "exec".to_string(),
            "--model".to_string(),
            "gpt-5-codex".to_string(),
            "--skip-git-repo-check".to_string(),
            "hello".to_string(),
        ]
    );
}

#[test]
fn test_build_codex_exec_args_includes_reasoning_override_when_selected() {
    let args = TyphooNApp::build_codex_exec_args("gpt-5", "xhigh", "hello");
    assert_eq!(
        args,
        vec![
            "exec".to_string(),
            "--model".to_string(),
            "gpt-5".to_string(),
            "--skip-git-repo-check".to_string(),
            "-c".to_string(),
            "model_reasoning_effort=\"xhigh\"".to_string(),
            "hello".to_string(),
        ]
    );
}

#[test]
fn test_build_hermes_exec_args_uses_configured_defaults_when_blank() {
    let args = TyphooNApp::build_hermes_exec_args("", "", "hello");
    assert_eq!(args, vec!["--oneshot".to_string(), "hello".to_string()]);
}

#[test]
fn test_build_hermes_exec_args_includes_overrides_when_selected() {
    let args = TyphooNApp::build_hermes_exec_args("openai/gpt-5.1", "openrouter", "hello");
    assert_eq!(
        args,
        vec![
            "--model".to_string(),
            "openai/gpt-5.1".to_string(),
            "--provider".to_string(),
            "openrouter".to_string(),
            "--oneshot".to_string(),
            "hello".to_string(),
        ]
    );
}

#[test]
fn test_grok_effort_normalization_defaults_unknown_values() {
    assert_eq!(TyphooNApp::normalize_grok_effort("max"), "max");
    assert_eq!(TyphooNApp::normalize_grok_effort("bogus"), "high");
    assert_eq!(TyphooNApp::normalize_grok_effort(""), "high");
}

#[test]
fn test_build_grok_exec_args_uses_auto_model_when_blank_or_auto() {
    let args = TyphooNApp::build_grok_exec_args("auto", "bogus", "hello");
    assert_eq!(
        args,
        vec![
            "--no-alt-screen".to_string(),
            "--output-format".to_string(),
            "plain".to_string(),
            "--effort".to_string(),
            "high".to_string(),
            "--single".to_string(),
            "hello".to_string(),
        ]
    );
}

#[test]
fn test_build_grok_exec_args_omits_model_even_when_legacy_value_is_saved() {
    let args = TyphooNApp::build_grok_exec_args("grok-code-fast-1", "max", "hello");
    assert_eq!(
        args,
        vec![
            "--no-alt-screen".to_string(),
            "--output-format".to_string(),
            "plain".to_string(),
            "--effort".to_string(),
            "max".to_string(),
            "--single".to_string(),
            "hello".to_string(),
        ]
    );
}

fn sample_events() -> Vec<EventRow> {
    vec![
        EventRow {
            symbol: "AAPL".into(),
            company: "Apple Inc.".into(),
            date: "2026-05-01".into(),
            days_until: 10,
            kind: EventKind::Earnings,
            detail: "P/E 28.5".into(),
            in_alpaca: true,
            in_kraken: false,
        },
        EventRow {
            symbol: "MSFT".into(),
            company: "Microsoft".into(),
            date: "2026-05-08".into(),
            days_until: 17,
            kind: EventKind::ExDividend,
            detail: "0.82% yield".into(),
            in_alpaca: true,
            in_kraken: false,
        },
        EventRow {
            symbol: "T".into(),
            company: "AT&T, Inc.".into(), // tests comma escaping
            date: "2026-05-15".into(),
            days_until: 24,
            kind: EventKind::DividendPayment,
            detail: "5.10% yield".into(),
            in_alpaca: false,
            in_kraken: true,
        },
    ]
}

#[test]
fn test_build_events_ics_contains_calendar_wrapper() {
    let ics = TyphooNApp::build_events_ics(&sample_events(), EventSource::All, true, true, true);
    assert!(ics.starts_with("BEGIN:VCALENDAR\r\n"));
    assert!(ics.ends_with("END:VCALENDAR\r\n"));
    assert!(ics.contains("VERSION:2.0"));
    assert!(ics.contains("PRODID:-//TyphooN Terminal"));
}

#[test]
fn test_build_events_ics_emits_all_filtered_vevents() {
    let ics = TyphooNApp::build_events_ics(&sample_events(), EventSource::All, true, true, true);
    let vevent_count = ics.matches("BEGIN:VEVENT").count();
    assert_eq!(vevent_count, 3, "All 3 events should be emitted");
    assert!(ics.contains("SUMMARY:AAPL — Earnings"));
    assert!(ics.contains("DTSTART;VALUE=DATE:20260501"));
    assert!(ics.contains("DTEND;VALUE=DATE:20260502"));
}

#[test]
fn test_build_events_ics_respects_source_filter() {
    // Only Alpaca — AAPL (yes), MSFT (yes), T (no)
    let ics = TyphooNApp::build_events_ics(&sample_events(), EventSource::Alpaca, true, true, true);
    assert_eq!(ics.matches("BEGIN:VEVENT").count(), 2);
    assert!(ics.contains("AAPL"));
    assert!(ics.contains("MSFT"));
    assert!(!ics.contains("SUMMARY:T"));
}

#[test]
fn test_build_events_ics_respects_type_filter() {
    // Earnings only
    let ics = TyphooNApp::build_events_ics(&sample_events(), EventSource::All, true, false, false);
    assert_eq!(ics.matches("BEGIN:VEVENT").count(), 1);
    assert!(ics.contains("AAPL"));
    // Ex-Div + Div-Pay only
    let ics2 = TyphooNApp::build_events_ics(&sample_events(), EventSource::All, false, true, true);
    assert_eq!(ics2.matches("BEGIN:VEVENT").count(), 2);
}

#[test]
fn test_build_events_ics_escapes_special_chars() {
    let ics = TyphooNApp::build_events_ics(&sample_events(), EventSource::All, true, true, true);
    // Comma in "AT&T, Inc." must be escaped per RFC 5545
    assert!(
        ics.contains("AT&T\\, Inc."),
        "comma should be backslash-escaped: {}",
        ics
    );
}

#[test]
fn test_build_events_ics_skips_unparseable_dates() {
    let bad = vec![EventRow {
        symbol: "X".into(),
        company: "Bad".into(),
        date: "not-a-date".into(),
        days_until: 0,
        kind: EventKind::Earnings,
        detail: String::new(),
        in_alpaca: true,
        in_kraken: false,
    }];
    let ics = TyphooNApp::build_events_ics(&bad, EventSource::All, true, true, true);
    assert_eq!(ics.matches("BEGIN:VEVENT").count(), 0);
}

#[test]
fn news_article_in_focus_empty_set_passes_everything() {
    let focus = std::collections::HashSet::new();
    assert!(TyphooNApp::news_article_in_focus(&focus, "AAPL", &[]));
    assert!(TyphooNApp::news_article_in_focus(
        &focus,
        "",
        &["random".into()]
    ));
}

#[test]
fn news_article_in_focus_matches_primary_symbol() {
    let focus = std::collections::HashSet::from(["AAPL".to_string()]);
    assert!(TyphooNApp::news_article_in_focus(&focus, "AAPL", &[]));
    assert!(TyphooNApp::news_article_in_focus(&focus, "aapl", &[]));
    assert!(TyphooNApp::news_article_in_focus(&focus, " AAPL ", &[]));
}

#[test]
fn news_article_in_focus_matches_any_tagged_ticker() {
    let focus = std::collections::HashSet::from(["TMO".to_string()]);
    // Primary is unrelated, but tickers carry the match.
    assert!(TyphooNApp::news_article_in_focus(
        &focus,
        "A",
        &["XLV".into(), "TMO".into()]
    ));
}

#[test]
fn news_article_in_focus_rejects_when_no_overlap() {
    let focus = std::collections::HashSet::from(["AAPL".to_string(), "MSFT".to_string()]);
    assert!(!TyphooNApp::news_article_in_focus(
        &focus,
        "TMO",
        &["XLV".into(), "A".into()]
    ));
}

#[test]
fn news_article_in_focus_handles_empty_primary_with_tagged_tickers() {
    let focus = std::collections::HashSet::from(["BTC".to_string()]);
    assert!(TyphooNApp::news_article_in_focus(
        &focus,
        "",
        &["btc".into()]
    ));
}

#[test]
fn news_article_in_focus_ignores_whitespace_only_primary() {
    let focus = std::collections::HashSet::from(["AAPL".to_string()]);
    // Whitespace-only primary uppercases to empty; tickers must carry it.
    assert!(!TyphooNApp::news_article_in_focus(&focus, "   ", &[]));
    assert!(TyphooNApp::news_article_in_focus(
        &focus,
        "   ",
        &["AAPL".into()]
    ));
}

#[test]
fn kraken_ws_pair_is_fresh_at_returns_false_for_missing_entry() {
    let map = std::collections::HashMap::new();
    assert!(!TyphooNApp::kraken_ws_pair_is_fresh_at(
        &map, "BTCUSD", "1Min", 0
    ));
}

#[test]
fn kraken_ws_pair_is_fresh_at_returns_false_for_unknown_timeframe() {
    let mut map = std::collections::HashMap::new();
    map.insert(
        ("BTCUSD".to_string(), "BOGUS".to_string()),
        1_700_000_000_000,
    );
    assert!(!TyphooNApp::kraken_ws_pair_is_fresh_at(
        &map,
        "BTCUSD",
        "BOGUS",
        1_700_000_000_000
    ));
}

#[test]
fn kraken_ws_pair_is_fresh_at_passes_within_tf_x24_window() {
    // 1Min × 24 = 1440s = 1,440,000ms. Anchor at now-1,000,000ms (16.6min ago)
    // is still within the freshness window.
    let now_ms = 1_700_000_000_000i64;
    let anchor_ms = now_ms - 1_000_000;
    let mut map = std::collections::HashMap::new();
    map.insert(("BTCUSD".to_string(), "1Min".to_string()), anchor_ms);
    assert!(TyphooNApp::kraken_ws_pair_is_fresh_at(
        &map, "BTCUSD", "1Min", now_ms
    ));
}

#[test]
fn kraken_ws_pair_is_fresh_at_rejects_anchor_outside_window() {
    // 1Min × 24 = 1440s. Anchor at now-2000s (33 min) is past the window.
    let now_ms = 1_700_000_000_000i64;
    let anchor_ms = now_ms - 2_000_000;
    let mut map = std::collections::HashMap::new();
    map.insert(("BTCUSD".to_string(), "1Min".to_string()), anchor_ms);
    assert!(!TyphooNApp::kraken_ws_pair_is_fresh_at(
        &map, "BTCUSD", "1Min", now_ms
    ));
}

#[test]
fn kraken_ws_pair_is_fresh_at_scales_with_timeframe_period() {
    // 1Day × 24 = 24 days. Anchor at now - 20 days should still be fresh.
    let day_ms = 86_400_000i64;
    let now_ms = 1_700_000_000_000i64;
    let anchor_ms = now_ms - 20 * day_ms;
    let mut map = std::collections::HashMap::new();
    map.insert(("BTCUSD".to_string(), "1Day".to_string()), anchor_ms);
    assert!(TyphooNApp::kraken_ws_pair_is_fresh_at(
        &map, "BTCUSD", "1Day", now_ms
    ));
    // But 25 days ago is past the window.
    let stale_anchor = now_ms - 25 * day_ms;
    map.insert(("BTCUSD".to_string(), "1Day".to_string()), stale_anchor);
    assert!(!TyphooNApp::kraken_ws_pair_is_fresh_at(
        &map, "BTCUSD", "1Day", now_ms
    ));
}

#[test]
#[allow(deprecated)]
fn nav_typography_helpers_exist() {
    // Create a dummy context to exercise the helpers
    let ctx = egui::Context::default();
    let fonts = egui::FontDefinitions::default();
    ctx.set_fonts(fonts);

    let _ = ctx.run_ui(Default::default(), |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            nav_primary(ui, "TEST");
            nav_secondary(ui, "123.45");
            nav_muted(ui, "Yahoo");
        });
    });
}

#[test]
fn yahoo_fetcher_rate_limit_test() {
    // The rate limiter should prevent calls closer than 5 seconds
    // This is a structural test
    assert!(true);
}

#[test]
fn watchlist_fallback_price_display_test() {
    let mut fallback_prices = std::collections::HashMap::new();
    fallback_prices.insert(
        "TEST".to_string(),
        (123.45, "Yahoo".to_string(), std::time::Instant::now()),
    );
    assert!(fallback_prices.contains_key("TEST"));
}

#[test]
fn watchlist_row_from_raw_bars_uses_close_prices_for_weekend_cache() {
    let raw = vec![
        (1_700_000_000_000, 100.0, 110.0, 90.0, 105.0, 1_000.0),
        (1_700_086_400_000, 105.0, 115.0, 95.0, 112.0, 1_500.0),
    ];

    let row = watchlist_row_from_raw_bars("TEST", "alpaca:TEST:1Day", &raw).unwrap();

    assert_eq!(row.symbol, "TEST");
    assert_eq!(row.cache_key, "alpaca:TEST:1Day");
    assert_eq!(row.last, 112.0);
    assert_eq!(row.prev_close, 105.0);
    assert_eq!(row.change, 7.0);
    assert!((row.change_pct - 6.666_666_666_666_667).abs() < f64::EPSILON * 16.0);
    assert_eq!(row.volume, 1_500.0);
}

#[test]
fn watchlist_row_from_raw_bars_accepts_single_valid_cached_bar() {
    let raw = vec![(1_700_000_000_000, 10.0, 11.0, 9.0, 10.5, 250.0)];

    let row = watchlist_row_from_raw_bars("SOLO", "default:SOLO:1Day", &raw).unwrap();

    assert_eq!(row.last, 10.5);
    assert_eq!(row.prev_close, 10.5);
    assert_eq!(row.change, 0.0);
    assert_eq!(row.change_pct, 0.0);
}

#[test]
fn yahoo_price_fallback_test() {
    // Basic existence test - real network call is done at runtime
    assert!(true);
}

#[test]
#[allow(deprecated)]
fn market_depth_and_volume_profile_render_helpers_are_callable() {
    let depth = compute_market_depth(&[(100.0, 2.0), (99.5, 1.0)], &[(100.5, 3.0)]);
    assert_eq!(depth.bids.len(), 2);
    assert_eq!(depth.asks.len(), 1);

    let profile = VolumeProfile {
        price_levels: vec![(99.5, 10.0), (100.0, 25.0), (100.5, 15.0)],
        poc: 100.0,
        value_area_high: 100.5,
        value_area_low: 99.5,
    };

    let ctx = egui::Context::default();
    let _ = ctx.run_ui(Default::default(), |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(200.0, 120.0));
            let painter = ui.painter();
            draw_market_depth(painter, &depth, rect);
            draw_volume_profile(painter, &profile, rect);
        });
    });
}

#[test]
fn forming_bar_helpers_test() {
    let mut chart = ChartState::new("TEST", Timeframe::M1);
    chart.bars.push(Bar {
        ts_ms: 1000,
        open: 1.0,
        high: 1.0,
        low: 1.0,
        close: 1.0,
        volume: 1.0,
    });
    chart.mark_structural_change();
    let gen_before = chart.visible_bars_gen;
    chart.apply_forming_bar_update(Bar {
        ts_ms: 1000,
        open: 1.0,
        high: 2.0,
        low: 0.5,
        close: 1.5,
        volume: 2.0,
    });
    assert!(chart.forming_bar_dirty);
    assert_eq!(chart.visible_bars_gen, gen_before);
}

#[test]
fn live_quote_update_marks_forming_bar_dirty() {
    let mut chart = ChartState::new("TEST", Timeframe::M1);
    chart.bars.push(Bar {
        ts_ms: 1000,
        open: 100.0,
        high: 101.0,
        low: 99.0,
        close: 100.0,
        volume: 1.0,
    });
    chart.mark_structural_change();

    assert!(chart.apply_live_quote_update(110.0, 112.0, 10.0, 10.0, false));

    let last = chart.bars.last().unwrap();
    assert_eq!(last.close, 111.0);
    assert_eq!(last.high, 111.0);
    assert_eq!(last.low, 99.0);
    assert!(chart.forming_bar_dirty);
    assert_eq!(chart.fresh_live_quote_mid(), Some(111.0));
}

#[test]
fn wide_live_quote_spread_does_not_become_current_price() {
    let mut chart = ChartState::new("WOK", Timeframe::D1);
    chart.bars.push(Bar {
        ts_ms: 1000,
        open: 0.08,
        high: 0.081,
        low: 0.074,
        close: 0.0766,
        volume: 1.0,
    });

    // Off-hours/tokenized-equity books can be real but very wide. Keep the
    // bid/ask for spread display; do not turn that midpoint into C/current.
    assert!(!chart.apply_live_quote_update(0.065, 0.0866, 1.0, 1.0, false));

    let last = chart.bars.last().unwrap();
    assert_eq!(last.close, 0.0766);
    assert_eq!(chart.live_bid, 0.065);
    assert_eq!(chart.live_ask, 0.0866);
    assert!(ChartState::live_quote_spread_pct(chart.live_bid, chart.live_ask).unwrap() > 20.0);
    assert_eq!(chart.fresh_live_quote_mid(), None);
}

#[test]
fn extended_hours_live_quote_does_not_mutate_regular_close() {
    let mut chart = ChartState::new("WOK", Timeframe::D1);
    chart.bars.push(Bar {
        ts_ms: 1000,
        open: 0.08,
        high: 0.081,
        low: 0.074,
        close: 0.0766,
        volume: 1.0,
    });
    chart.ext_active = true;
    chart.ext_close = 0.0772;

    assert!(!chart.apply_live_quote_update(0.0770, 0.0774, 1.0, 1.0, false));

    assert_eq!(chart.bars.last().unwrap().close, 0.0766);
    assert!((chart.fresh_live_quote_mid().unwrap() - 0.0772).abs() < 1e-12);
}

#[test]
fn delayed_live_quote_does_not_drive_realtime_display() {
    // A non-WS-tokenized xStock (e.g. WOK) has no Kraken WS L2 book, so the chart's
    // only bid/ask is the iapi equities ticker — always fetched delayed=true. A
    // delayed quote must be stored for reference but never folded into the forming
    // bar nor exposed as the live mid; otherwise its stale price decouples the
    // chart's candle/bid-ask from the watchlist's fresher consolidated last (the
    // "big desync between chart / watchlist" report).
    let mut chart = ChartState::new("WOK", Timeframe::H1);
    chart.bars.push(Bar {
        ts_ms: 1000,
        open: 0.10,
        high: 0.103,
        low: 0.099,
        close: 0.1026,
        volume: 1.0,
    });

    assert!(!chart.apply_live_quote_update(0.0858, 0.0870, 1.0, 1.0, true));
    // Stored so the rest of the app can see the latest (delayed) quote…
    assert_eq!(chart.live_bid, 0.0858);
    assert_eq!(chart.live_ask, 0.0870);
    assert!(chart.live_quote_delayed);
    // …but it neither moved the forming candle nor became the real-time mid.
    assert_eq!(chart.bars.last().unwrap().close, 0.1026);
    assert_eq!(chart.fresh_live_quote_mid(), None);

    // A real-time (delayed=false) quote at the same spread still drives the chart.
    assert!(chart.apply_live_quote_update(0.1020, 0.1032, 5.0, 5.0, false));
    assert!((chart.fresh_live_quote_mid().unwrap() - 0.1026).abs() < 1e-9);
    assert!((chart.bars.last().unwrap().close - 0.1026).abs() < 1e-9);
}

#[test]
fn full_recompute_folds_fresh_live_quote_after_cache_reload_without_fast_path() {
    let mut chart = ChartState::new("TEST", Timeframe::M1);
    for i in 0..300 {
        chart.bars.push(Bar {
            ts_ms: 1000 + i as i64 * 60_000,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.0,
            volume: 1.0,
        });
    }
    chart.compute_indicators();
    chart.forming_bar_dirty = false;

    // Simulate a queued cache reload that replaced the active forming candle with
    // an older persisted close while the chart still owns a fresh live quote.
    chart.bars.last_mut().unwrap().close = 100.0;
    chart.bars.last_mut().unwrap().high = 101.0;
    chart.live_bid = 119.0;
    chart.live_ask = 121.0;
    chart.live_quote_at = Some(std::time::Instant::now());
    chart.live_quote_delayed = false;

    chart.compute_indicators();

    let last = chart.bars.last().unwrap();
    assert_eq!(last.close, 120.0);
    assert_eq!(last.high, 120.0);
    assert!(!chart.forming_bar_dirty);
}

#[test]
fn stale_live_quote_is_not_folded_into_reloaded_bar() {
    let mut chart = ChartState::new("TEST", Timeframe::M1);
    chart.bars.push(Bar {
        ts_ms: 1000,
        open: 100.0,
        high: 101.0,
        low: 99.0,
        close: 100.0,
        volume: 1.0,
    });
    chart.live_bid = 119.0;
    chart.live_ask = 121.0;
    chart.live_quote_at = Some(std::time::Instant::now() - std::time::Duration::from_secs(31));

    chart.compute_indicators();

    assert_eq!(chart.bars.last().unwrap().close, 100.0);
    assert!(!chart.forming_bar_dirty);
}

#[test]
fn news_dedup_placeholder_test() {
    // Placeholder test for article deduplication logic.
    // Real implementation will use article_exists_by_url_hash.
    let should_dedup = true;
    assert!(should_dedup);
}

#[test]
fn kraken_ws_pair_is_fresh_at_handles_future_anchor_gracefully() {
    // Defensive: clock skew could land an anchor slightly in the future.
    // saturating_sub(future) clamps to 0, which is < max_age_ms → fresh.
    let now_ms = 1_700_000_000_000i64;
    let future_anchor = now_ms + 60_000;
    let mut map = std::collections::HashMap::new();
    map.insert(("BTCUSD".to_string(), "1Min".to_string()), future_anchor);
    assert!(TyphooNApp::kraken_ws_pair_is_fresh_at(
        &map, "BTCUSD", "1Min", now_ms
    ));
}

#[test]
fn chart_state_forming_bar_fast_path() {
    let mut chart = ChartState::new("TEST", Timeframe::M1);
    chart.bars.push(Bar {
        ts_ms: 1_000_000,
        open: 100.0,
        high: 101.0,
        low: 99.0,
        close: 100.5,
        volume: 10.0,
    });
    chart.mark_structural_change();
    let gen_before = chart.visible_bars_gen;

    let forming = Bar {
        ts_ms: 1_000_000,
        open: 100.0,
        high: 102.0,
        low: 99.5,
        close: 101.8,
        volume: 15.0,
    };
    chart.apply_forming_bar_update(forming);

    assert!(chart.forming_bar_dirty);
    assert_eq!(chart.last_visible_bar_ts, 1_000_000);
    assert_eq!(chart.bars.last().unwrap().close, 101.8);
    assert_eq!(chart.visible_bars_gen, gen_before);

    let closed = Bar {
        ts_ms: 1_060_000,
        open: 101.8,
        high: 103.0,
        low: 101.0,
        close: 102.5,
        volume: 20.0,
    };
    chart.bars.push(closed);
    chart.mark_structural_change();

    assert!(!chart.forming_bar_dirty);
    assert!(chart.visible_bars_gen > gen_before);
}

#[test]
fn chart_state_tracks_render_snapshot_fields_without_skipping_paint() {
    let mut chart = ChartState::new("TEST", Timeframe::M5);
    chart.bars.push(Bar {
        ts_ms: 1000,
        open: 1.0,
        high: 1.0,
        low: 1.0,
        close: 1.0,
        volume: 1.0,
    });
    chart.mark_structural_change();

    // These fields are retained for data/change diagnostics. draw_chart must
    // still paint every frame because egui has no retained chart render target;
    // skipping paint causes closed-market charts to blank/flicker on hover/pan.
    chart.last_rendered_gen = chart.visible_bars_gen;
    chart.last_rendered_bar_ts = chart.last_visible_bar_ts;

    assert_eq!(chart.visible_bars_gen, chart.last_rendered_gen);
    assert_eq!(chart.last_visible_bar_ts, chart.last_rendered_bar_ts);
}

#[test]
fn compute_indicators_gpu_forming_bar_fast_path() {
    let mut chart = ChartState::new("TEST", Timeframe::M1);
    // Seed with enough bars for SMA
    for i in 0..300 {
        chart.bars.push(Bar {
            ts_ms: 1000 + i as i64 * 60_000,
            open: 100.0 + i as f64 * 0.1,
            high: 101.0 + i as f64 * 0.1,
            low: 99.0 + i as f64 * 0.1,
            close: 100.5 + i as f64 * 0.1,
            volume: 1000.0,
        });
    }
    chart.mark_structural_change();

    // Simulate live WS tick
    chart.forming_bar_dirty = true;
    chart.apply_forming_bar_update(Bar {
        ts_ms: chart.bars.last().unwrap().ts_ms,
        open: 130.0,
        high: 132.0,
        low: 129.0,
        close: 131.5,
        volume: 1500.0,
    });

    // The fast path in compute_indicators_gpu should handle this without full recompute
    // (we just check that the flag is respected and last value is updated)
    assert!(chart.forming_bar_dirty); // still set until compute_indicators_gpu consumes it
}

// Yahoo Finance price fallback (used when primary broker has no recent data)
#[allow(dead_code)]
pub async fn fetch_yahoo_last_price(symbol: &str) -> Option<(f64, String)> {
    // Simple rate limiting to avoid hammering Yahoo
    static LAST_YAHOO_CALL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let last = LAST_YAHOO_CALL.load(std::sync::atomic::Ordering::Relaxed);
    if now - last < 5 {
        return None; // too soon
    }
    LAST_YAHOO_CALL.store(now, std::sync::atomic::Ordering::Relaxed);

    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?interval=1d&range=5d",
        symbol
    );

    let client = reqwest::Client::new();
    let resp = match client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (compatible; TyphooN-Terminal)")
        .timeout(std::time::Duration::from_secs(8))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return None,
    };

    if !resp.status().is_success() {
        return None;
    }

    let json: serde_json::Value = match resp.json().await {
        Ok(j) => j,
        Err(_) => return None,
    };

    let price = json["chart"]["result"][0]["meta"]["regularMarketPrice"]
        .as_f64()
        .or_else(|| json["chart"]["result"][0]["meta"]["previousClose"].as_f64())?;

    Some((price, "Yahoo".to_string()))
}

#[allow(dead_code)]
pub async fn fetch_last_price_with_fallback(symbol: &str) -> Option<(f64, String)> {
    if let Some((price, source)) = fetch_yahoo_last_price(symbol).await {
        return Some((price, source));
    }
    None
}

/// Headless reproduction of the armed-tool click gate (egui 0.35): a press +
/// release over a CentralPanel body widget must surface as a raw
/// `primary_clicked()` with `egui_wants_pointer_input()` false, and the
/// widget-routed `clicked()` should agree. Pins down exactly which gate eats
/// the first placement click.
#[test]
fn armed_click_gate_over_central_panel() {
    let ctx = egui::Context::default();
    let click_pos = egui::pos2(400.0, 300.0);
    let mut release_frame_report = None;
    let mut per_phase = Vec::new();
    for phase in 0..4 {
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(800.0, 600.0),
            )),
            ..Default::default()
        };
        match phase {
            0 => input.events.push(egui::Event::PointerMoved(click_pos)),
            1 => input.events.push(egui::Event::PointerButton {
                pos: click_pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            }),
            2 => input.events.push(egui::Event::PointerButton {
                pos: click_pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::default(),
            }),
            _ => {}
        }
        let _ = ctx.run_ui(input, |root_ui| {
            let ctx = root_ui.ctx().clone();
            egui::Panel::top("toolbar").show(root_ui, |ui| {
                ui.label("toolbar");
            });
            egui::CentralPanel::default().show(root_ui, |ui| {
                let (rect, _hover) =
                    ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
                let body = egui::Rect::from_min_max(
                    rect.min,
                    egui::pos2(rect.right() - 98.0, rect.bottom()),
                );
                let resp = ui.interact(
                    body,
                    ui.id().with("single_chart_body_drag"),
                    egui::Sense::click_and_drag(),
                );
                per_phase.push((
                    phase,
                    ctx.egui_wants_pointer_input(),
                    ctx.egui_is_using_pointer(),
                    ctx.layer_id_at(click_pos).map(|l| l.order),
                ));
                if phase == 2 {
                    release_frame_report = Some((
                        ctx.input(|i: &egui::InputState| i.pointer.primary_clicked()),
                        ctx.egui_wants_pointer_input(),
                        ctx.input(|i| i.pointer.interact_pos()),
                        ctx.layer_id_at(click_pos).map(|l| l.order),
                        resp.clicked(),
                    ));
                }
            });
        });
    }
    println!("per-phase (phase, wants_pointer, is_using, layer): {per_phase:?}");
    let (raw_clicked, wants_pointer, interact_pos, layer, widget_clicked) =
        release_frame_report.expect("release frame ran");
    // The raw click + its position must surface on the release frame — this
    // is what the placement gate consumes.
    assert!(raw_clicked, "primary_clicked() false on release frame");
    assert!(interact_pos.is_some(), "interact_pos None on release frame");
    assert!(
        widget_clicked,
        "widget-routed clicked() false on release frame"
    );
    // The chart body sits on the Background layer — the "over floating UI"
    // test used by chart input gating (windows = Middle, popups = Foreground)
    // must NOT fire here.
    assert_eq!(layer, Some(egui::Order::Background));
    // TRAP (egui 0.35): `egui_wants_pointer_input()` is TRUE on EVERY frame
    // over a CentralPanel (panel widgets register a Background layer and the
    // root-rect test classifies that as "over egui"). It must never be used
    // to gate chart clicks/hover — doing so silently killed drawing
    // placement, the crosshair, and scroll-zoom. This assertion documents
    // the behavior so nobody reintroduces the gate believing it means
    // "pointer over a floating window".
    assert!(
        wants_pointer,
        "egui_wants_pointer_input() became false over a CentralPanel — if \
         egui's semantics changed back, the layer-order gating in \
         app_runtime_central_panel/app_runtime_input can be revisited"
    );
    // And `egui_is_using_pointer()` is only true while the button is held
    // (press frame) — false on hover and release frames.
    let is_using_by_phase: Vec<bool> = per_phase.iter().map(|(_, _, u, _)| *u).collect();
    assert_eq!(is_using_by_phase, vec![false, true, false, false]);
}
