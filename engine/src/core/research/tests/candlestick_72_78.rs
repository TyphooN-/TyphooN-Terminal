// ── Round 72 CDL* tests ────────────────────────────────────────

fn synthetic_cdl_doji_bar() -> HistoricalPriceRow {
    // Final bar has open ≈ close (tiny body) with long shadows — a doji.
    HistoricalPriceRow {
        date: "2024-06-15".into(),
        open: 100.0,
        high: 102.0,
        low: 98.0,
        close: 100.05,
        adj_close: 100.05,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    }
}

fn synthetic_cdl_hammer_bar() -> HistoricalPriceRow {
    // Small body upper, long lower shadow — bullish hammer.
    // body = 0.5, range = 4.5, lower = 3.5 (≥ 2*body), upper = 0.5.
    HistoricalPriceRow {
        date: "2024-06-16".into(),
        open: 100.0,
        high: 100.5,
        low: 96.5,
        close: 100.3,
        adj_close: 100.3,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    }
}

fn synthetic_cdl_shooting_star_bar() -> HistoricalPriceRow {
    // Small body lower, long upper shadow — bearish shooting star.
    // body = 0.5, range = 4.5, upper = 3.5 (≥ 2*body), lower = 0.5.
    HistoricalPriceRow {
        date: "2024-06-17".into(),
        open: 100.3,
        high: 103.8,
        low: 99.8,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    }
}

#[test]
fn cdl_doji_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlDojiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 100,
        pattern_value: 100,
        pattern_value_prev: 0,
        body_pct_range: 1.1,
        upper_shadow_pct: 45.0,
        lower_shadow_pct: 45.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 100.05,
        cdl_doji_label: "DOJI_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_doji(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_doji(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_doji_label, "DOJI_PATTERN");
    assert_eq!(got.pattern_value, 100);
    assert!(got.last_bar_match);
}

#[test]
fn cdl_doji_compute_detects_doji() {
    // Build 10 boring bars + 1 doji bar to verify last-bar detection.
    let mut bars: Vec<HistoricalPriceRow> = (0..10)
        .map(|i| HistoricalPriceRow {
            date: format!("2024-06-{:02}", i + 1),
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.8,
            adj_close: 100.8,
            volume: 1_000_000.0,
            change: 0.0,
            change_pct: 0.0,
        })
        .collect();
    bars.push(synthetic_cdl_doji_bar());
    let snap = compute_cdl_doji_snapshot("T", "2026-04-18", &bars);
    assert_eq!(snap.cdl_doji_label, "DOJI_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.last_bar_match);
    assert_eq!(snap.days_since_pattern, 0);
    // Identity: body_pct_range is small (≤ 5%) when doji detected
    assert!(snap.body_pct_range <= 5.0);
}

#[test]
fn cdl_hammer_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlHammerSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 100,
        pattern_value: 100,
        pattern_value_prev: 0,
        body_pct_range: 11.0,
        upper_shadow_pct: 11.0,
        lower_shadow_pct: 78.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 100.3,
        cdl_hammer_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_hammer(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_hammer(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_hammer_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_hammer_compute_detects_hammer() {
    let mut bars: Vec<HistoricalPriceRow> = (0..10)
        .map(|i| HistoricalPriceRow {
            date: format!("2024-06-{:02}", i + 1),
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.8,
            adj_close: 100.8,
            volume: 1_000_000.0,
            change: 0.0,
            change_pct: 0.0,
        })
        .collect();
    bars.push(synthetic_cdl_hammer_bar());
    let snap = compute_cdl_hammer_snapshot("T", "2026-04-18", &bars);
    assert_eq!(snap.cdl_hammer_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.last_bar_match);
    // Identity: hammer has lower_shadow ≥ 2× body (so lower_pct > upper_pct)
    assert!(snap.lower_shadow_pct > snap.upper_shadow_pct);
}

#[test]
fn cdl_shooting_star_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlShootingStarSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 100,
        pattern_value: -100,
        pattern_value_prev: 0,
        body_pct_range: 11.0,
        upper_shadow_pct: 78.0,
        lower_shadow_pct: 11.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 100.0,
        cdl_shooting_star_label: "BEARISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_shooting_star(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_shooting_star(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_shooting_star_label, "BEARISH_PATTERN");
    assert_eq!(got.pattern_value, -100);
}

#[test]
fn cdl_shooting_star_compute_detects() {
    let mut bars: Vec<HistoricalPriceRow> = (0..10)
        .map(|i| HistoricalPriceRow {
            date: format!("2024-06-{:02}", i + 1),
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.8,
            adj_close: 100.8,
            volume: 1_000_000.0,
            change: 0.0,
            change_pct: 0.0,
        })
        .collect();
    bars.push(synthetic_cdl_shooting_star_bar());
    let snap = compute_cdl_shooting_star_snapshot("T", "2026-04-18", &bars);
    assert_eq!(snap.cdl_shooting_star_label, "BEARISH_PATTERN");
    assert_eq!(snap.pattern_value, -100);
    // Identity: shooting star has upper_shadow ≥ 2× body (upper_pct > lower_pct)
    assert!(snap.upper_shadow_pct > snap.lower_shadow_pct);
}

#[test]
fn cdl_engulfing_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlEngulfingSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 100,
        pattern_value: 100,
        pattern_value_prev: 0,
        body_size_ratio: 2.5,
        prior_body_pct_range: 50.0,
        current_body_pct_range: 85.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 102.0,
        cdl_engulfing_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_engulfing(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_engulfing(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_engulfing_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_engulfing_compute_detects_bullish() {
    // Build 10 boring bars + prior red + current bullish engulfing.
    let mut bars: Vec<HistoricalPriceRow> = (0..10)
        .map(|i| HistoricalPriceRow {
            date: format!("2024-06-{:02}", i + 1),
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.8,
            adj_close: 100.8,
            volume: 1_000_000.0,
            change: 0.0,
            change_pct: 0.0,
        })
        .collect();
    // Prior bar: red (open 101, close 99, body = 2)
    bars.push(HistoricalPriceRow {
        date: "2024-06-11".into(),
        open: 101.0,
        high: 101.5,
        low: 98.5,
        close: 99.0,
        adj_close: 99.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    // Current bar: green, engulfs prior body (open 98.5, close 102, body = 3.5 > 2)
    bars.push(HistoricalPriceRow {
        date: "2024-06-12".into(),
        open: 98.5,
        high: 102.5,
        low: 98.0,
        close: 102.0,
        adj_close: 102.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    let snap = compute_cdl_engulfing_snapshot("T", "2026-04-18", &bars);
    assert_eq!(snap.cdl_engulfing_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    // Identity: body_size_ratio > 1.0 when current engulfs prior
    assert!(snap.body_size_ratio > 1.0);
}

#[test]
fn cdl_harami_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlHaramiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 100,
        pattern_value: 100,
        pattern_value_prev: 0,
        body_size_ratio: 0.3,
        prior_body_pct_range: 85.0,
        current_body_pct_range: 40.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 100.5,
        cdl_harami_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_harami(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_harami(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_harami_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_harami_compute_detects_bullish() {
    // Build 10 boring bars + prior red + current green contained in prior body.
    let mut bars: Vec<HistoricalPriceRow> = (0..10)
        .map(|i| HistoricalPriceRow {
            date: format!("2024-06-{:02}", i + 1),
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.8,
            adj_close: 100.8,
            volume: 1_000_000.0,
            change: 0.0,
            change_pct: 0.0,
        })
        .collect();
    // Prior bar: red, big body (open 104, close 98, body = 6)
    bars.push(HistoricalPriceRow {
        date: "2024-06-11".into(),
        open: 104.0,
        high: 104.5,
        low: 97.5,
        close: 98.0,
        adj_close: 98.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    // Current bar: green, body inside prior (open 100, close 102, body = 2 < 6)
    // open 100 ≥ prior_close 98, close 102 ≤ prior_open 104
    bars.push(HistoricalPriceRow {
        date: "2024-06-12".into(),
        open: 100.0,
        high: 102.3,
        low: 99.8,
        close: 102.0,
        adj_close: 102.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    let snap = compute_cdl_harami_snapshot("T", "2026-04-18", &bars);
    assert_eq!(snap.cdl_harami_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    // Identity: body_size_ratio < 1.0 when current contained in prior
    assert!(snap.body_size_ratio < 1.0);
}

// ── Round 73 CDL* tests ────────────────────────────────────────

fn boring_green_bars(count: usize, start_date_month: u32) -> Vec<HistoricalPriceRow> {
    (0..count)
        .map(|i| HistoricalPriceRow {
            date: format!("2024-{:02}-{:02}", start_date_month, i + 1),
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.8,
            adj_close: 100.8,
            volume: 1_000_000.0,
            change: 0.0,
            change_pct: 0.0,
        })
        .collect()
}

#[test]
fn cdl_morning_star_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlMorningStarSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 100,
        pattern_value: 100,
        pattern_value_prev: 0,
        penetration_pct: 45.0,
        star_body_pct_range: 15.0,
        first_body_pct_range: 60.0,
        last_body_pct_range: 65.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 103.0,
        cdl_morning_star_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_morning_star(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_morning_star(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_morning_star_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_morning_star_compute_detects() {
    let mut bars = boring_green_bars(10, 7);
    // bar 0: large red body (open 104, close 100, body 4, range 5)
    bars.push(HistoricalPriceRow {
        date: "2024-07-11".into(),
        open: 104.0,
        high: 104.5,
        low: 99.5,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    // bar 1: small star (body 0.1, range 1)
    bars.push(HistoricalPriceRow {
        date: "2024-07-12".into(),
        open: 99.5,
        high: 100.1,
        low: 99.1,
        close: 99.6,
        adj_close: 99.6,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    // bar 2: large green body (open 100, close 104, body 4, range 5), above bar0_mid = 102
    bars.push(HistoricalPriceRow {
        date: "2024-07-13".into(),
        open: 100.0,
        high: 104.5,
        low: 99.8,
        close: 104.0,
        adj_close: 104.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    let snap = compute_cdl_morning_star_snapshot("T", "2026-04-18", &bars);
    assert_eq!(snap.cdl_morning_star_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    // Identity: penetration > 0 when bullish (last close above first midpoint)
    assert!(snap.penetration_pct > 0.0);
}

#[test]
fn cdl_evening_star_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlEveningStarSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 100,
        pattern_value: -100,
        pattern_value_prev: 0,
        penetration_pct: 45.0,
        star_body_pct_range: 15.0,
        first_body_pct_range: 60.0,
        last_body_pct_range: 65.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 97.0,
        cdl_evening_star_label: "BEARISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_evening_star(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_evening_star(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_evening_star_label, "BEARISH_PATTERN");
    assert_eq!(got.pattern_value, -100);
}

#[test]
fn cdl_evening_star_compute_detects() {
    let mut bars = boring_green_bars(10, 8);
    // bar 0: large green body (open 100, close 104)
    bars.push(HistoricalPriceRow {
        date: "2024-08-11".into(),
        open: 100.0,
        high: 104.5,
        low: 99.8,
        close: 104.0,
        adj_close: 104.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    // bar 1: small star
    bars.push(HistoricalPriceRow {
        date: "2024-08-12".into(),
        open: 104.5,
        high: 105.0,
        low: 104.0,
        close: 104.4,
        adj_close: 104.4,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    // bar 2: large red body (open 104, close 100), below bar0_mid = 102
    bars.push(HistoricalPriceRow {
        date: "2024-08-13".into(),
        open: 104.0,
        high: 104.5,
        low: 99.5,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    let snap = compute_cdl_evening_star_snapshot("T", "2026-04-18", &bars);
    assert_eq!(snap.cdl_evening_star_label, "BEARISH_PATTERN");
    assert_eq!(snap.pattern_value, -100);
    // Identity: penetration > 0 when bearish (last close below first midpoint)
    assert!(snap.penetration_pct > 0.0);
}

#[test]
fn cdl_three_black_crows_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlThreeBlackCrowsSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 100,
        pattern_value: -100,
        pattern_value_prev: 0,
        avg_body_pct_range: 80.0,
        total_close_decline_pct: -6.5,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 94.0,
        cdl_three_black_crows_label: "BEARISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_three_black_crows(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_three_black_crows(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_three_black_crows_label, "BEARISH_PATTERN");
    assert_eq!(got.pattern_value, -100);
}

#[test]
fn cdl_three_black_crows_compute_detects() {
    let mut bars = boring_green_bars(10, 9);
    // Three consecutive red bars, each opens within prior body, closes below prior close.
    // Bar 0: open 104, close 101 (red, body 3, range 4, body_pct 75%)
    bars.push(HistoricalPriceRow {
        date: "2024-09-11".into(),
        open: 104.0,
        high: 104.5,
        low: 100.5,
        close: 101.0,
        adj_close: 101.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    // Bar 1: open 103 (between 101 and 104), close 99 (< 101, body 4, range 4.5)
    bars.push(HistoricalPriceRow {
        date: "2024-09-12".into(),
        open: 103.0,
        high: 103.2,
        low: 98.7,
        close: 99.0,
        adj_close: 99.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    // Bar 2: open 101 (between 99 and 103), close 96 (< 99, body 5, range 5.5)
    bars.push(HistoricalPriceRow {
        date: "2024-09-13".into(),
        open: 101.0,
        high: 101.2,
        low: 95.7,
        close: 96.0,
        adj_close: 96.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    let snap = compute_cdl_three_black_crows_snapshot("T", "2026-04-18", &bars);
    assert_eq!(snap.cdl_three_black_crows_label, "BEARISH_PATTERN");
    assert_eq!(snap.pattern_value, -100);
    // Identity: total_close_decline_pct < 0 when bearish
    assert!(snap.total_close_decline_pct < 0.0);
}

#[test]
fn cdl_three_white_soldiers_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlThreeWhiteSoldiersSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 100,
        pattern_value: 100,
        pattern_value_prev: 0,
        avg_body_pct_range: 80.0,
        total_close_advance_pct: 6.5,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 106.0,
        cdl_three_white_soldiers_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_three_white_soldiers(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_three_white_soldiers(&conn, "TEST")
        .unwrap()
        .unwrap();
    assert_eq!(got.cdl_three_white_soldiers_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_three_white_soldiers_compute_detects() {
    let mut bars = boring_green_bars(10, 10);
    // Bar 0: open 100, close 103 (green, body 3, range 4)
    bars.push(HistoricalPriceRow {
        date: "2024-10-11".into(),
        open: 100.0,
        high: 103.5,
        low: 99.5,
        close: 103.0,
        adj_close: 103.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    // Bar 1: open 101 (between 100 and 103), close 105 (> 103, body 4, range 4.5)
    bars.push(HistoricalPriceRow {
        date: "2024-10-12".into(),
        open: 101.0,
        high: 105.3,
        low: 100.8,
        close: 105.0,
        adj_close: 105.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    // Bar 2: open 103 (between 101 and 105), close 108 (> 105, body 5, range 5.5)
    bars.push(HistoricalPriceRow {
        date: "2024-10-13".into(),
        open: 103.0,
        high: 108.3,
        low: 102.8,
        close: 108.0,
        adj_close: 108.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    let snap = compute_cdl_three_white_soldiers_snapshot("T", "2026-04-18", &bars);
    assert_eq!(snap.cdl_three_white_soldiers_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.total_close_advance_pct > 0.0);
}

#[test]
fn cdl_dark_cloud_cover_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlDarkCloudCoverSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 100,
        pattern_value: -100,
        pattern_value_prev: 0,
        penetration_pct: 60.0,
        prior_body_pct_range: 80.0,
        current_body_pct_range: 70.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 100.5,
        cdl_dark_cloud_cover_label: "BEARISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_dark_cloud_cover(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_dark_cloud_cover(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_dark_cloud_cover_label, "BEARISH_PATTERN");
    assert_eq!(got.pattern_value, -100);
}

#[test]
fn cdl_dark_cloud_cover_compute_detects() {
    let mut bars = boring_green_bars(10, 11);
    // Prior bar: green, body ≥ 30% of range. open 100, close 104, body 4, range 4.5
    bars.push(HistoricalPriceRow {
        date: "2024-11-11".into(),
        open: 100.0,
        high: 104.3,
        low: 99.8,
        close: 104.0,
        adj_close: 104.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    // Current bar: red. open 104.5 (> prior_high 104.3), close 101 (< prior_midpoint 102),
    //   but close 101 > prior_open 100 (not engulfing). body 3.5, range 4.
    bars.push(HistoricalPriceRow {
        date: "2024-11-12".into(),
        open: 104.5,
        high: 104.7,
        low: 100.7,
        close: 101.0,
        adj_close: 101.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    let snap = compute_cdl_dark_cloud_cover_snapshot("T", "2026-04-18", &bars);
    assert_eq!(snap.cdl_dark_cloud_cover_label, "BEARISH_PATTERN");
    assert_eq!(snap.pattern_value, -100);
    // Identity: penetration_pct > 0 when bearish (prior_close above current_close)
    assert!(snap.penetration_pct > 0.0);
}

// ── Round 74 tests — CDLPIERCING / CDLDRAGONFLYDOJI /
//    CDLGRAVESTONEDOJI / CDLHANGINGMAN / CDLINVERTEDHAMMER ──

#[test]
fn cdl_piercing_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlPiercingSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 100,
        pattern_value: 100,
        pattern_value_prev: 0,
        penetration_pct: 65.0,
        prior_body_pct_range: 80.0,
        current_body_pct_range: 70.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 103.0,
        cdl_piercing_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_piercing(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_piercing(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_piercing_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_piercing_compute_detects() {
    let mut bars = boring_green_bars(10, 12);
    // Prior bar: red, body ≥ 30% of range. open 104, close 100, body 4, range 4.5
    bars.push(HistoricalPriceRow {
        date: "2024-12-11".into(),
        open: 104.0,
        high: 104.2,
        low: 99.7,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    // Current bar: green. open 99.5 (< prior_low 99.7), close 103 (> prior_midpoint 102),
    //   but close 103 < prior_open 104 (not engulfing). body 3.5, range 4.
    bars.push(HistoricalPriceRow {
        date: "2024-12-12".into(),
        open: 99.5,
        high: 103.3,
        low: 99.3,
        close: 103.0,
        adj_close: 103.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    let snap = compute_cdl_piercing_snapshot("T", "2026-04-18", &bars);
    assert_eq!(snap.cdl_piercing_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.penetration_pct > 0.0);
}

#[test]
fn cdl_dragonfly_doji_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlDragonflyDojiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 50,
        pattern_value: 100,
        pattern_value_prev: 0,
        body_pct_range: 2.0,
        upper_shadow_pct: 3.0,
        lower_shadow_pct: 80.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 50.1,
        cdl_dragonfly_doji_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_dragonfly_doji(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_dragonfly_doji(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_dragonfly_doji_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_dragonfly_doji_compute_detects() {
    let mut bars = boring_green_bars(5, 3);
    // Dragonfly: open 100, close 100.1 (tiny body 0.1), high 100.2, low 95 (long lower shadow 5)
    bars.push(HistoricalPriceRow {
        date: "2024-03-06".into(),
        open: 100.0,
        high: 100.2,
        low: 95.0,
        close: 100.1,
        adj_close: 100.1,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    let snap = compute_cdl_dragonfly_doji_snapshot("T", "2026-04-18", &bars);
    assert_eq!(snap.cdl_dragonfly_doji_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.lower_shadow_pct > snap.upper_shadow_pct);
}

#[test]
fn cdl_gravestone_doji_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlGravestoneDojiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 50,
        pattern_value: -100,
        pattern_value_prev: 0,
        body_pct_range: 2.0,
        upper_shadow_pct: 80.0,
        lower_shadow_pct: 3.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 50.0,
        cdl_gravestone_doji_label: "BEARISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_gravestone_doji(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_gravestone_doji(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_gravestone_doji_label, "BEARISH_PATTERN");
    assert_eq!(got.pattern_value, -100);
}

#[test]
fn cdl_gravestone_doji_compute_detects() {
    let mut bars = boring_green_bars(5, 4);
    // Gravestone: open 100.1, close 100 (tiny body 0.1), high 105 (long upper shadow 5), low 99.95
    bars.push(HistoricalPriceRow {
        date: "2024-04-06".into(),
        open: 100.1,
        high: 105.0,
        low: 99.95,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    let snap = compute_cdl_gravestone_doji_snapshot("T", "2026-04-18", &bars);
    assert_eq!(snap.cdl_gravestone_doji_label, "BEARISH_PATTERN");
    assert_eq!(snap.pattern_value, -100);
    assert!(snap.upper_shadow_pct > snap.lower_shadow_pct);
}

#[test]
fn cdl_hanging_man_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlHangingManSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 50,
        pattern_value: -100,
        pattern_value_prev: 0,
        body_pct_range: 20.0,
        upper_shadow_pct: 5.0,
        lower_shadow_pct: 70.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 100.5,
        cdl_hanging_man_label: "BEARISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_hanging_man(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_hanging_man(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_hanging_man_label, "BEARISH_PATTERN");
    assert_eq!(got.pattern_value, -100);
}

#[test]
fn cdl_hanging_man_compute_detects() {
    let mut bars = boring_green_bars(5, 5);
    // Hanging Man / Hammer geometry: body 0.5 (open 100, close 100.5), range ~4.
    // High 100.7 (upper shadow 0.2 ≤ body), low 96.5 (lower shadow 3.5 ≥ 2 × body).
    bars.push(HistoricalPriceRow {
        date: "2024-05-06".into(),
        open: 100.0,
        high: 100.7,
        low: 96.5,
        close: 100.5,
        adj_close: 100.5,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    let snap = compute_cdl_hanging_man_snapshot("T", "2026-04-18", &bars);
    assert_eq!(snap.cdl_hanging_man_label, "BEARISH_PATTERN");
    assert_eq!(snap.pattern_value, -100);
    assert!(snap.lower_shadow_pct > snap.upper_shadow_pct);
}

#[test]
fn cdl_inverted_hammer_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlInvertedHammerSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 50,
        pattern_value: 100,
        pattern_value_prev: 0,
        body_pct_range: 20.0,
        upper_shadow_pct: 70.0,
        lower_shadow_pct: 5.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 100.5,
        cdl_inverted_hammer_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_inverted_hammer(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_inverted_hammer(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_inverted_hammer_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_inverted_hammer_compute_detects() {
    let mut bars = boring_green_bars(5, 6);
    // Inverted hammer / shooting star geometry: body 0.5 (open 100, close 100.5),
    // range ~4. Upper shadow 3.5 (≥ 2 × body), lower shadow 0.2 (≤ body).
    bars.push(HistoricalPriceRow {
        date: "2024-06-06".into(),
        open: 100.0,
        high: 104.0,
        low: 99.8,
        close: 100.5,
        adj_close: 100.5,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    let snap = compute_cdl_inverted_hammer_snapshot("T", "2026-04-18", &bars);
    assert_eq!(snap.cdl_inverted_hammer_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.upper_shadow_pct > snap.lower_shadow_pct);
}

// ── Round 75 tests — CDLHARAMICROSS / CDLLONGLEGGEDDOJI / CDLMARUBOZU /
//    CDLSPINNINGTOP / CDLTRISTAR ──

#[test]
fn cdl_harami_cross_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlHaramiCrossSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-19".into(),
        bars_used: 60,
        pattern_value: 100,
        pattern_value_prev: 0,
        prior_body_pct_range: 80.0,
        current_body_pct_range: 3.0,
        body_size_ratio: 0.05,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 100.5,
        cdl_harami_cross_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_harami_cross(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_harami_cross(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_harami_cross_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
    assert!(got.body_size_ratio < 1.0);
}

#[test]
fn cdl_harami_cross_compute_detects() {
    let mut bars = boring_green_bars(3, 5);
    // Prior: big red body (open 110, close 100, range 110 → 99, body 10 ≥ 30% of range 11).
    bars.push(HistoricalPriceRow {
        date: "2024-06-05".into(),
        open: 110.0,
        high: 110.0,
        low: 99.0,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: -10.0,
        change_pct: -9.0,
    });
    // Current: doji contained in prior body. Range 104..106, body at ~105 with body_pct ≤ 5%.
    bars.push(HistoricalPriceRow {
        date: "2024-06-06".into(),
        open: 105.00,
        high: 106.0,
        low: 104.0,
        close: 105.03,
        adj_close: 105.03,
        volume: 1_000_000.0,
        change: 5.03,
        change_pct: 5.03,
    });
    let snap = compute_cdl_harami_cross_snapshot("T", "2026-04-19", &bars);
    assert_eq!(snap.cdl_harami_cross_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.current_body_pct_range <= 5.0);
    assert!(snap.body_size_ratio < 1.0);
}

#[test]
fn cdl_long_legged_doji_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlLongLeggedDojiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-19".into(),
        bars_used: 40,
        pattern_value: 100,
        pattern_value_prev: 0,
        body_pct_range: 3.0,
        upper_shadow_pct: 45.0,
        lower_shadow_pct: 52.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 100.03,
        cdl_long_legged_doji_label: "DOJI_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_long_legged_doji(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_long_legged_doji(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_long_legged_doji_label, "DOJI_PATTERN");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_long_legged_doji_compute_detects() {
    let mut bars = boring_green_bars(5, 6);
    // Long-legged doji: open ≈ close at 100, range 10, both shadows ≥ 30% of range.
    bars.push(HistoricalPriceRow {
        date: "2024-06-06".into(),
        open: 100.0,
        high: 105.0,
        low: 95.0,
        close: 100.1,
        adj_close: 100.1,
        volume: 1_000_000.0,
        change: 0.1,
        change_pct: 0.1,
    });
    let snap = compute_cdl_long_legged_doji_snapshot("T", "2026-04-19", &bars);
    assert_eq!(snap.cdl_long_legged_doji_label, "DOJI_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.body_pct_range <= 5.0);
    assert!(snap.upper_shadow_pct >= 30.0 && snap.lower_shadow_pct >= 30.0);
}

#[test]
fn cdl_marubozu_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlMarubozuSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-19".into(),
        bars_used: 30,
        pattern_value: 100,
        pattern_value_prev: 0,
        body_pct_range: 95.0,
        upper_shadow_pct: 2.0,
        lower_shadow_pct: 3.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 110.0,
        cdl_marubozu_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_marubozu(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_marubozu(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_marubozu_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_marubozu_compute_detects() {
    let mut bars = boring_green_bars(5, 6);
    // Bullish marubozu: open 100.02, close 109.98, high 110.0, low 100.0 — body ≥ 90%, shadows ≤ 5%.
    bars.push(HistoricalPriceRow {
        date: "2024-06-06".into(),
        open: 100.02,
        high: 110.0,
        low: 100.0,
        close: 109.98,
        adj_close: 109.98,
        volume: 1_000_000.0,
        change: 9.96,
        change_pct: 9.96,
    });
    let snap = compute_cdl_marubozu_snapshot("T", "2026-04-19", &bars);
    assert_eq!(snap.cdl_marubozu_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.body_pct_range >= 90.0);
    assert!(snap.upper_shadow_pct <= 5.0 && snap.lower_shadow_pct <= 5.0);
}

#[test]
fn cdl_spinning_top_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlSpinningTopSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-19".into(),
        bars_used: 35,
        pattern_value: 100,
        pattern_value_prev: 0,
        body_pct_range: 20.0,
        upper_shadow_pct: 40.0,
        lower_shadow_pct: 40.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 100.5,
        cdl_spinning_top_label: "GREEN_BODY_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_spinning_top(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_spinning_top(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_spinning_top_label, "GREEN_BODY_PATTERN");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_spinning_top_compute_detects() {
    let mut bars = boring_green_bars(5, 6);
    // Spinning top: open 100, close 101 (body 1), high 105, low 97 → range 8, body 1 (12.5%)
    // upper shadow 4 > body, lower shadow 3 > body.
    bars.push(HistoricalPriceRow {
        date: "2024-06-06".into(),
        open: 100.0,
        high: 105.0,
        low: 97.0,
        close: 101.0,
        adj_close: 101.0,
        volume: 1_000_000.0,
        change: 1.0,
        change_pct: 1.0,
    });
    let snap = compute_cdl_spinning_top_snapshot("T", "2026-04-19", &bars);
    assert_eq!(snap.cdl_spinning_top_label, "GREEN_BODY_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.body_pct_range <= 30.0);
}

#[test]
fn cdl_tristar_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlTristarSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-19".into(),
        bars_used: 50,
        pattern_value: 100,
        pattern_value_prev: 0,
        avg_body_pct_range: 3.0,
        middle_gap_pct: -1.5,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 100.1,
        cdl_tristar_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_tristar(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_tristar(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_tristar_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_tristar_compute_detects() {
    let mut bars = boring_green_bars(3, 5);
    // Bullish tristar: three dojis with middle gapping below.
    // Doji 1: around 100, tight body, wider range
    bars.push(HistoricalPriceRow {
        date: "2024-06-04".into(),
        open: 100.0,
        high: 101.0,
        low: 99.5,
        close: 100.02,
        adj_close: 100.02,
        volume: 1_000_000.0,
        change: 0.02,
        change_pct: 0.02,
    });
    // Doji 2: gaps below — body around 97, high 97.5 < 99.5 (doji1 low) and < doji3 low
    bars.push(HistoricalPriceRow {
        date: "2024-06-05".into(),
        open: 97.0,
        high: 97.3,
        low: 96.0,
        close: 96.98,
        adj_close: 96.98,
        volume: 1_000_000.0,
        change: -3.04,
        change_pct: -3.04,
    });
    // Doji 3: back up — body around 100, low 99.6 > 97.3 (doji2 high)
    bars.push(HistoricalPriceRow {
        date: "2024-06-06".into(),
        open: 100.2,
        high: 101.1,
        low: 99.6,
        close: 100.22,
        adj_close: 100.22,
        volume: 1_000_000.0,
        change: 3.24,
        change_pct: 3.34,
    });
    let snap = compute_cdl_tristar_snapshot("T", "2026-04-19", &bars);
    assert_eq!(snap.cdl_tristar_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.avg_body_pct_range <= 5.0);
}

// ── Round 76 tests — CDLDOJISTAR / CDLMORNINGDOJISTAR /
//    CDLEVENINGDOJISTAR / CDLABANDONEDBABY / CDL3INSIDE ──

#[test]
fn cdl_doji_star_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlDojiStarSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-19".into(),
        bars_used: 50,
        pattern_value: 100,
        pattern_value_prev: 0,
        prior_body_pct_range: 70.0,
        current_body_pct_range: 3.0,
        gap_pct: -1.5,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 99.0,
        cdl_doji_star_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_doji_star(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_doji_star(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_doji_star_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_doji_star_compute_detects() {
    let mut bars = boring_green_bars(3, 5);
    // Prior: big red body (open 110, close 100, range ~11, body 10 ≥ 30%).
    bars.push(HistoricalPriceRow {
        date: "2024-06-05".into(),
        open: 110.0,
        high: 110.0,
        low: 99.0,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: -10.0,
        change_pct: -9.0,
    });
    // Current: doji gapping BELOW prior close (bullish bottom star).
    // Body at 95, range 94..96 → body_pct ≈ 5% or less; body entirely below 100.
    bars.push(HistoricalPriceRow {
        date: "2024-06-06".into(),
        open: 95.02,
        high: 96.0,
        low: 94.0,
        close: 94.98,
        adj_close: 94.98,
        volume: 1_000_000.0,
        change: -5.02,
        change_pct: -5.02,
    });
    let snap = compute_cdl_doji_star_snapshot("T", "2026-04-19", &bars);
    assert_eq!(snap.cdl_doji_star_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.current_body_pct_range <= 5.0);
    assert!(snap.gap_pct < 0.0); // gap down signed negative
}

#[test]
fn cdl_morning_doji_star_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlMorningDojiStarSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-19".into(),
        bars_used: 80,
        pattern_value: 100,
        pattern_value_prev: 0,
        bar1_body_pct_range: 75.0,
        bar2_body_pct_range: 3.0,
        bar3_close_vs_bar1_mid_pct: 2.5,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 103.5,
        cdl_morning_doji_star_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_morning_doji_star(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_morning_doji_star(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_morning_doji_star_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_morning_doji_star_compute_detects() {
    let mut bars = boring_green_bars(10, 7);
    // Bar 1: long red body (open 104, close 100)
    bars.push(HistoricalPriceRow {
        date: "2024-07-11".into(),
        open: 104.0,
        high: 104.5,
        low: 99.5,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    // Bar 2: doji gapping below bar 1 close (100). Body at 97, range 96.5..97.8.
    bars.push(HistoricalPriceRow {
        date: "2024-07-12".into(),
        open: 97.0,
        high: 97.8,
        low: 96.5,
        close: 97.02,
        adj_close: 97.02,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    // Bar 3: green, closes above bar 1 midpoint (102)
    bars.push(HistoricalPriceRow {
        date: "2024-07-13".into(),
        open: 98.0,
        high: 104.5,
        low: 97.9,
        close: 104.0,
        adj_close: 104.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    let snap = compute_cdl_morning_doji_star_snapshot("T", "2026-04-19", &bars);
    assert_eq!(snap.cdl_morning_doji_star_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.bar2_body_pct_range <= 5.0);
    assert!(snap.bar3_close_vs_bar1_mid_pct > 0.0);
}

#[test]
fn cdl_evening_doji_star_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlEveningDojiStarSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-19".into(),
        bars_used: 80,
        pattern_value: -100,
        pattern_value_prev: 0,
        bar1_body_pct_range: 75.0,
        bar2_body_pct_range: 3.0,
        bar3_close_vs_bar1_mid_pct: -2.5,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 98.5,
        cdl_evening_doji_star_label: "BEARISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_evening_doji_star(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_evening_doji_star(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_evening_doji_star_label, "BEARISH_PATTERN");
    assert_eq!(got.pattern_value, -100);
}

#[test]
fn cdl_evening_doji_star_compute_detects() {
    let mut bars = boring_green_bars(10, 7);
    // Bar 1: long green body (open 100, close 104)
    bars.push(HistoricalPriceRow {
        date: "2024-07-11".into(),
        open: 100.0,
        high: 104.5,
        low: 99.5,
        close: 104.0,
        adj_close: 104.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    // Bar 2: doji gapping above bar 1 close (104). Body at 107, range 106.5..107.8.
    bars.push(HistoricalPriceRow {
        date: "2024-07-12".into(),
        open: 107.0,
        high: 107.8,
        low: 106.5,
        close: 107.02,
        adj_close: 107.02,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    // Bar 3: red, closes below bar 1 midpoint (102)
    bars.push(HistoricalPriceRow {
        date: "2024-07-13".into(),
        open: 106.0,
        high: 106.1,
        low: 99.5,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    let snap = compute_cdl_evening_doji_star_snapshot("T", "2026-04-19", &bars);
    assert_eq!(snap.cdl_evening_doji_star_label, "BEARISH_PATTERN");
    assert_eq!(snap.pattern_value, -100);
    assert!(snap.bar2_body_pct_range <= 5.0);
    assert!(snap.bar3_close_vs_bar1_mid_pct < 0.0);
}

#[test]
fn cdl_abandoned_baby_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlAbandonedBabySnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-19".into(),
        bars_used: 60,
        pattern_value: 100,
        pattern_value_prev: 0,
        bar1_body_pct_range: 85.0,
        bar2_body_pct_range: 3.0,
        gap_down_pct: 1.5,
        gap_up_pct: 1.8,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 103.0,
        cdl_abandoned_baby_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_abandoned_baby(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_abandoned_baby(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_abandoned_baby_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_abandoned_baby_compute_detects() {
    let mut bars = boring_green_bars(10, 7);
    // Bar 1: long red body, low = 99. (open 110, close 100, high 111, low 99)
    bars.push(HistoricalPriceRow {
        date: "2024-07-11".into(),
        open: 110.0,
        high: 111.0,
        low: 99.0,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    // Bar 2: doji with high = 95 (< bar1.low 99). body around 94, range 93..95.
    bars.push(HistoricalPriceRow {
        date: "2024-07-12".into(),
        open: 94.0,
        high: 95.0,
        low: 93.0,
        close: 94.05,
        adj_close: 94.05,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    // Bar 3: green with low = 96 (> bar2.high 95). body 100→108, range 96..108.
    bars.push(HistoricalPriceRow {
        date: "2024-07-13".into(),
        open: 100.0,
        high: 108.0,
        low: 96.0,
        close: 108.0,
        adj_close: 108.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    let snap = compute_cdl_abandoned_baby_snapshot("T", "2026-04-19", &bars);
    assert_eq!(snap.cdl_abandoned_baby_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.bar2_body_pct_range <= 5.0);
}

#[test]
fn cdl_three_inside_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlThreeInsideSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-19".into(),
        bars_used: 70,
        pattern_value: 100,
        pattern_value_prev: 0,
        bar1_body_pct_range: 75.0,
        body_size_ratio: 0.25,
        bar3_close_vs_bar1_open_pct: 1.5,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 112.0,
        cdl_three_inside_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_three_inside(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_three_inside(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_three_inside_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
    assert!(got.body_size_ratio < 1.0);
}

#[test]
fn cdl_three_inside_compute_detects() {
    let mut bars = boring_green_bars(10, 7);
    // Bar 1: long red body (open 110, close 100, range 111..99, body_pct ≈ 83%)
    bars.push(HistoricalPriceRow {
        date: "2024-07-11".into(),
        open: 110.0,
        high: 111.0,
        low: 99.0,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    // Bar 2: small green body inside bar 1 body (100..110) — open 103, close 107.
    bars.push(HistoricalPriceRow {
        date: "2024-07-12".into(),
        open: 103.0,
        high: 107.5,
        low: 102.5,
        close: 107.0,
        adj_close: 107.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    // Bar 3: closes above bar 1 open (110), i.e., 112.
    bars.push(HistoricalPriceRow {
        date: "2024-07-13".into(),
        open: 108.0,
        high: 112.5,
        low: 107.5,
        close: 112.0,
        adj_close: 112.0,
        volume: 1_000_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    let snap = compute_cdl_three_inside_snapshot("T", "2026-04-19", &bars);
    assert_eq!(snap.cdl_three_inside_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.body_size_ratio < 1.0);
    assert!(snap.bar3_close_vs_bar1_open_pct > 0.0);
}

// ── Round 77 tests — CDLBELTHOLD / CDLCLOSINGMARUBOZU / CDLHIGHWAVE /
//    CDLLONGLINE / CDLSHORTLINE ──

#[test]
fn cdl_belt_hold_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlBeltHoldSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 30,
        pattern_value: 100,
        pattern_value_prev: 0,
        body_pct_range: 80.0,
        opening_shadow_pct: 0.0,
        closing_shadow_pct: 20.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 108.0,
        cdl_belt_hold_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_belt_hold(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_belt_hold(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_belt_hold_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
    assert!(got.opening_shadow_pct <= 5.0);
}

#[test]
fn cdl_belt_hold_compute_detects() {
    let mut bars = boring_green_bars(5, 6);
    bars.push(HistoricalPriceRow {
        date: "2024-08-06".into(),
        open: 100.0,
        high: 110.0,
        low: 100.0,
        close: 108.0,
        adj_close: 108.0,
        volume: 1_000_000.0,
        change: 8.0,
        change_pct: 8.0,
    });
    let snap = compute_cdl_belt_hold_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_belt_hold_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.body_pct_range >= 60.0);
    assert!(snap.opening_shadow_pct <= 5.0);
}

#[test]
fn cdl_closing_marubozu_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlClosingMarubozuSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 30,
        pattern_value: 100,
        pattern_value_prev: 0,
        body_pct_range: 80.0,
        opening_shadow_pct: 20.0,
        closing_shadow_pct: 0.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 108.0,
        cdl_closing_marubozu_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_closing_marubozu(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_closing_marubozu(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_closing_marubozu_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
    assert!(got.closing_shadow_pct <= 5.0);
}

#[test]
fn cdl_closing_marubozu_compute_detects() {
    let mut bars = boring_green_bars(5, 6);
    bars.push(HistoricalPriceRow {
        date: "2024-08-06".into(),
        open: 100.0,
        high: 108.0,
        low: 98.0,
        close: 108.0,
        adj_close: 108.0,
        volume: 1_000_000.0,
        change: 8.0,
        change_pct: 8.0,
    });
    let snap = compute_cdl_closing_marubozu_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_closing_marubozu_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.body_pct_range >= 60.0);
    assert!(snap.closing_shadow_pct <= 5.0);
}

#[test]
fn cdl_high_wave_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlHighWaveSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 30,
        pattern_value: 100,
        pattern_value_prev: 0,
        body_pct_range: 11.1,
        upper_shadow_pct: 44.4,
        lower_shadow_pct: 44.4,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 101.0,
        cdl_high_wave_label: "GREEN_BODY_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_high_wave(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_high_wave(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_high_wave_label, "GREEN_BODY_PATTERN");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_high_wave_compute_detects() {
    let mut bars = boring_green_bars(5, 6);
    bars.push(HistoricalPriceRow {
        date: "2024-08-06".into(),
        open: 100.0,
        high: 105.0,
        low: 96.0,
        close: 101.0,
        adj_close: 101.0,
        volume: 1_000_000.0,
        change: 1.0,
        change_pct: 1.0,
    });
    let snap = compute_cdl_high_wave_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_high_wave_label, "GREEN_BODY_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.body_pct_range <= 20.0);
    assert!(snap.upper_shadow_pct >= 30.0 && snap.lower_shadow_pct >= 30.0);
}

#[test]
fn cdl_long_line_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlLongLineSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 30,
        pattern_value: 100,
        pattern_value_prev: 0,
        body_pct_range: 77.8,
        upper_shadow_pct: 11.1,
        lower_shadow_pct: 11.1,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 107.0,
        cdl_long_line_label: "GREEN_BODY_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_long_line(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_long_line(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_long_line_label, "GREEN_BODY_PATTERN");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_long_line_compute_detects() {
    let mut bars = boring_green_bars(5, 6);
    bars.push(HistoricalPriceRow {
        date: "2024-08-06".into(),
        open: 100.0,
        high: 108.0,
        low: 99.0,
        close: 107.0,
        adj_close: 107.0,
        volume: 1_000_000.0,
        change: 7.0,
        change_pct: 7.0,
    });
    let snap = compute_cdl_long_line_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_long_line_label, "GREEN_BODY_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.body_pct_range >= 60.0);
    assert!(snap.upper_shadow_pct <= 20.0 && snap.lower_shadow_pct <= 20.0);
}

#[test]
fn cdl_short_line_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlShortLineSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 30,
        pattern_value: 100,
        pattern_value_prev: 0,
        body_pct_range: 20.0,
        upper_shadow_pct: 20.0,
        lower_shadow_pct: 20.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 100.6,
        cdl_short_line_label: "GREEN_BODY_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_short_line(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_short_line(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_short_line_label, "GREEN_BODY_PATTERN");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_short_line_compute_detects() {
    let mut bars = boring_green_bars(5, 6);
    bars.push(HistoricalPriceRow {
        date: "2024-08-06".into(),
        open: 100.0,
        high: 100.5,
        low: 99.7,
        close: 100.2,
        adj_close: 100.2,
        volume: 1_000_000.0,
        change: 0.2,
        change_pct: 0.2,
    });
    let snap = compute_cdl_short_line_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_short_line_label, "GREEN_BODY_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.body_pct_range >= 5.0 && snap.body_pct_range <= 30.0);
    assert!(snap.upper_shadow_pct <= 40.0 && snap.lower_shadow_pct <= 40.0);
}

// ── Round 78 tests — CDLCOUNTERATTACK / CDLHOMINGPIGEON / CDLINNECK /
//    CDLONNECK / CDLTHRUSTING ──

#[test]
fn cdl_counterattack_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlCounterattackSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 30,
        pattern_value: 100,
        pattern_value_prev: 0,
        prior_body_pct_range: 83.3,
        current_body_pct_range: 90.0,
        gap_open_pct: 1.0,
        close_diff_pct_body: 2.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 100.2,
        cdl_counterattack_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_counterattack(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_counterattack(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_counterattack_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
    assert!(got.close_diff_pct_body <= 10.0);
}

#[test]
fn cdl_counterattack_compute_detects() {
    let mut bars = boring_green_bars(5, 6);
    bars.push(HistoricalPriceRow {
        date: "2024-08-06".into(),
        open: 110.0,
        high: 111.0,
        low: 99.0,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: -10.0,
        change_pct: -9.1,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-08-07".into(),
        open: 98.0,
        high: 100.4,
        low: 98.0,
        close: 100.2,
        adj_close: 100.2,
        volume: 1_200_000.0,
        change: 2.2,
        change_pct: 2.24,
    });
    let snap = compute_cdl_counterattack_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_counterattack_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.gap_open_pct > 0.0);
    assert!(snap.close_diff_pct_body <= 10.0);
}

#[test]
fn cdl_homing_pigeon_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlHomingPigeonSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 30,
        pattern_value: 100,
        pattern_value_prev: 0,
        prior_body_pct_range: 83.3,
        current_body_pct_range: 50.0,
        body_size_ratio: 0.3,
        inner_body_margin_pct: 20.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 103.0,
        cdl_homing_pigeon_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_homing_pigeon(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_homing_pigeon(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_homing_pigeon_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
    assert!(got.body_size_ratio < 1.0);
}

#[test]
fn cdl_homing_pigeon_compute_detects() {
    let mut bars = boring_green_bars(5, 6);
    bars.push(HistoricalPriceRow {
        date: "2024-08-06".into(),
        open: 110.0,
        high: 112.0,
        low: 99.0,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: -10.0,
        change_pct: -9.1,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-08-07".into(),
        open: 106.0,
        high: 107.0,
        low: 101.0,
        close: 103.0,
        adj_close: 103.0,
        volume: 1_100_000.0,
        change: -3.0,
        change_pct: -2.8,
    });
    let snap = compute_cdl_homing_pigeon_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_homing_pigeon_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.body_size_ratio < 1.0);
    assert!(snap.inner_body_margin_pct >= 0.0);
}

#[test]
fn cdl_in_neck_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlInNeckSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 30,
        pattern_value: -100,
        pattern_value_prev: 0,
        prior_body_pct_range: 83.3,
        current_body_pct_range: 53.0,
        gap_open_pct: 1.0,
        penetration_pct: 12.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 101.2,
        cdl_in_neck_label: "BEARISH_CONTINUATION".into(),
        note: String::new(),
    };
    upsert_cdl_in_neck(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_in_neck(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_in_neck_label, "BEARISH_CONTINUATION");
    assert_eq!(got.pattern_value, -100);
    assert!(got.penetration_pct > 5.0 && got.penetration_pct <= 25.0);
}

#[test]
fn cdl_in_neck_compute_detects() {
    let mut bars = boring_green_bars(5, 6);
    bars.push(HistoricalPriceRow {
        date: "2024-08-06".into(),
        open: 110.0,
        high: 111.0,
        low: 99.0,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: -10.0,
        change_pct: -9.1,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-08-07".into(),
        open: 98.0,
        high: 103.0,
        low: 97.0,
        close: 101.2,
        adj_close: 101.2,
        volume: 1_100_000.0,
        change: 3.2,
        change_pct: 3.27,
    });
    let snap = compute_cdl_in_neck_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_in_neck_label, "BEARISH_CONTINUATION");
    assert_eq!(snap.pattern_value, -100);
    assert!(snap.gap_open_pct > 0.0);
    assert!(snap.penetration_pct > 5.0 && snap.penetration_pct <= 25.0);
}

#[test]
fn cdl_on_neck_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlOnNeckSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 30,
        pattern_value: -100,
        pattern_value_prev: 0,
        prior_body_pct_range: 83.3,
        current_body_pct_range: 65.0,
        gap_open_pct: 1.0,
        close_match_pct: 3.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 100.3,
        cdl_on_neck_label: "BEARISH_CONTINUATION".into(),
        note: String::new(),
    };
    upsert_cdl_on_neck(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_on_neck(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_on_neck_label, "BEARISH_CONTINUATION");
    assert_eq!(got.pattern_value, -100);
    assert!(got.close_match_pct <= 5.0);
}

#[test]
fn cdl_on_neck_compute_detects() {
    let mut bars = boring_green_bars(5, 6);
    bars.push(HistoricalPriceRow {
        date: "2024-08-06".into(),
        open: 110.0,
        high: 111.0,
        low: 99.0,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: -10.0,
        change_pct: -9.1,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-08-07".into(),
        open: 98.0,
        high: 100.5,
        low: 97.0,
        close: 100.3,
        adj_close: 100.3,
        volume: 1_100_000.0,
        change: 2.3,
        change_pct: 2.35,
    });
    let snap = compute_cdl_on_neck_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_on_neck_label, "BEARISH_CONTINUATION");
    assert_eq!(snap.pattern_value, -100);
    assert!(snap.gap_open_pct > 0.0);
    assert!(snap.close_match_pct <= 5.0);
}

#[test]
fn cdl_thrusting_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlThrustingSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 30,
        pattern_value: -100,
        pattern_value_prev: 0,
        prior_body_pct_range: 83.3,
        current_body_pct_range: 78.0,
        gap_open_pct: 1.0,
        penetration_pct: 35.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 103.5,
        cdl_thrusting_label: "BEARISH_CONTINUATION".into(),
        note: String::new(),
    };
    upsert_cdl_thrusting(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_thrusting(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_thrusting_label, "BEARISH_CONTINUATION");
    assert_eq!(got.pattern_value, -100);
    assert!(got.penetration_pct > 25.0 && got.penetration_pct < 50.0);
}

#[test]
fn cdl_thrusting_compute_detects() {
    let mut bars = boring_green_bars(5, 6);
    bars.push(HistoricalPriceRow {
        date: "2024-08-06".into(),
        open: 110.0,
        high: 111.0,
        low: 99.0,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: -10.0,
        change_pct: -9.1,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-08-07".into(),
        open: 98.0,
        high: 104.0,
        low: 97.0,
        close: 103.5,
        adj_close: 103.5,
        volume: 1_100_000.0,
        change: 5.5,
        change_pct: 5.61,
    });
    let snap = compute_cdl_thrusting_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_thrusting_label, "BEARISH_CONTINUATION");
    assert_eq!(snap.pattern_value, -100);
    assert!(snap.gap_open_pct > 0.0);
    assert!(snap.penetration_pct > 25.0 && snap.penetration_pct < 50.0);
}

