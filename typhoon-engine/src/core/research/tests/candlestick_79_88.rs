// ── Round 79/80 tests — CDL2CROWS / CDL3LINESTRIKE /
//    CDL3OUTSIDE / CDLMATCHINGLOW / CDLSEPARATINGLINES /
//    CDLSTICKSANDWICH / CDLRICKSHAWMAN / CDLTAKURI ──

#[test]
fn cdl_two_crows_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlTwoCrowsSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 40,
        pattern_value: -100,
        pattern_value_prev: 0,
        first_body_pct_range: 83.3,
        second_gap_pct: 1.8,
        third_penetration_pct: 45.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 105.0,
        cdl_two_crows_label: "BEARISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_two_crows(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_two_crows(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_two_crows_label, "BEARISH_PATTERN");
    assert_eq!(got.pattern_value, -100);
    assert!(got.second_gap_pct > 0.0);
}

#[test]
fn cdl_two_crows_compute_detects() {
    let mut bars = boring_green_bars(5, 9);
    bars.push(HistoricalPriceRow {
        date: "2024-09-06".into(),
        open: 100.0,
        high: 111.0,
        low: 99.0,
        close: 110.0,
        adj_close: 110.0,
        volume: 1_000_000.0,
        change: 10.0,
        change_pct: 10.0,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-09-07".into(),
        open: 115.0,
        high: 116.0,
        low: 111.5,
        close: 112.0,
        adj_close: 112.0,
        volume: 1_000_000.0,
        change: 2.0,
        change_pct: 1.82,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-09-08".into(),
        open: 113.5,
        high: 114.0,
        low: 104.5,
        close: 105.0,
        adj_close: 105.0,
        volume: 1_100_000.0,
        change: -7.0,
        change_pct: -6.25,
    });
    let snap = compute_cdl_two_crows_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_two_crows_label, "BEARISH_PATTERN");
    assert_eq!(snap.pattern_value, -100);
    assert!(snap.second_gap_pct > 0.0);
    assert!(snap.third_penetration_pct > 0.0);
}

#[test]
fn cdl_three_line_strike_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlThreeLineStrikeSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 45,
        pattern_value: 100,
        pattern_value_prev: 0,
        avg_first_three_body_pct_range: 62.0,
        strike_body_pct_range: 80.0,
        strike_close_vs_first_open_pct: 0.9,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 111.0,
        cdl_three_line_strike_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_three_line_strike(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_three_line_strike(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_three_line_strike_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_three_line_strike_compute_detects() {
    let mut bars = boring_green_bars(5, 9);
    bars.push(HistoricalPriceRow {
        date: "2024-09-06".into(),
        open: 110.0,
        high: 111.0,
        low: 105.0,
        close: 106.0,
        adj_close: 106.0,
        volume: 1_000_000.0,
        change: -4.0,
        change_pct: -3.64,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-09-07".into(),
        open: 106.0,
        high: 106.5,
        low: 102.5,
        close: 103.0,
        adj_close: 103.0,
        volume: 1_050_000.0,
        change: -3.0,
        change_pct: -2.83,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-09-08".into(),
        open: 103.0,
        high: 103.5,
        low: 99.5,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_100_000.0,
        change: -3.0,
        change_pct: -2.91,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-09-09".into(),
        open: 99.0,
        high: 111.5,
        low: 98.5,
        close: 111.0,
        adj_close: 111.0,
        volume: 1_200_000.0,
        change: 11.0,
        change_pct: 11.0,
    });
    let snap = compute_cdl_three_line_strike_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_three_line_strike_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.avg_first_three_body_pct_range >= 30.0);
    assert!(snap.strike_body_pct_range >= 40.0);
}

#[test]
fn cdl_three_outside_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlThreeOutsideSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 35,
        pattern_value: 100,
        pattern_value_prev: 0,
        first_body_pct_range: 83.3,
        engulf_body_ratio: 1.3,
        confirmation_pct_body2: 18.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 114.0,
        cdl_three_outside_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_three_outside(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_three_outside(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_three_outside_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
    assert!(got.engulf_body_ratio > 1.0);
}

#[test]
fn cdl_three_outside_compute_detects() {
    let mut bars = boring_green_bars(5, 9);
    bars.push(HistoricalPriceRow {
        date: "2024-09-06".into(),
        open: 110.0,
        high: 111.0,
        low: 99.0,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: -10.0,
        change_pct: -9.09,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-09-07".into(),
        open: 99.0,
        high: 112.5,
        low: 98.5,
        close: 112.0,
        adj_close: 112.0,
        volume: 1_100_000.0,
        change: 12.0,
        change_pct: 12.0,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-09-08".into(),
        open: 110.0,
        high: 114.5,
        low: 109.5,
        close: 114.0,
        adj_close: 114.0,
        volume: 1_050_000.0,
        change: 2.0,
        change_pct: 1.79,
    });
    let snap = compute_cdl_three_outside_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_three_outside_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.engulf_body_ratio > 1.0);
}

#[test]
fn cdl_matching_low_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlMatchingLowSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 30,
        pattern_value: 100,
        pattern_value_prev: 0,
        prior_body_pct_range: 83.3,
        current_body_pct_range: 53.0,
        close_match_pct_body: 3.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 100.3,
        cdl_matching_low_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_matching_low(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_matching_low(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_matching_low_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
    assert!(got.close_match_pct_body <= 5.0);
}

#[test]
fn cdl_matching_low_compute_detects() {
    let mut bars = boring_green_bars(5, 9);
    bars.push(HistoricalPriceRow {
        date: "2024-09-06".into(),
        open: 110.0,
        high: 111.0,
        low: 99.0,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: -10.0,
        change_pct: -9.09,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-09-07".into(),
        open: 104.0,
        high: 104.5,
        low: 99.5,
        close: 100.3,
        adj_close: 100.3,
        volume: 1_050_000.0,
        change: 0.3,
        change_pct: 0.3,
    });
    let snap = compute_cdl_matching_low_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_matching_low_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.close_match_pct_body <= 5.0);
}

#[test]
fn cdl_separating_lines_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlSeparatingLinesSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 30,
        pattern_value: 100,
        pattern_value_prev: 0,
        prior_body_pct_range: 83.3,
        current_body_pct_range: 58.0,
        open_match_pct_body: 5.0,
        continuation_pct_body: 70.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 117.0,
        cdl_separating_lines_label: "BULLISH_CONTINUATION".into(),
        note: String::new(),
    };
    upsert_cdl_separating_lines(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_separating_lines(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_separating_lines_label, "BULLISH_CONTINUATION");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_separating_lines_compute_detects() {
    let mut bars = boring_green_bars(5, 9);
    bars.push(HistoricalPriceRow {
        date: "2024-09-06".into(),
        open: 110.0,
        high: 111.0,
        low: 99.0,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: -10.0,
        change_pct: -9.09,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-09-07".into(),
        open: 110.5,
        high: 117.0,
        low: 109.5,
        close: 117.0,
        adj_close: 117.0,
        volume: 1_100_000.0,
        change: 17.0,
        change_pct: 17.0,
    });
    let snap = compute_cdl_separating_lines_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_separating_lines_label, "BULLISH_CONTINUATION");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.open_match_pct_body <= 10.0);
}

#[test]
fn cdl_stick_sandwich_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlStickSandwichSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 40,
        pattern_value: 100,
        pattern_value_prev: 0,
        first_body_pct_range: 83.3,
        third_body_pct_range: 62.0,
        close_match_pct_body: 4.0,
        middle_rebound_pct: 12.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 100.4,
        cdl_stick_sandwich_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_stick_sandwich(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_stick_sandwich(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_stick_sandwich_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_stick_sandwich_compute_detects() {
    let mut bars = boring_green_bars(5, 9);
    bars.push(HistoricalPriceRow {
        date: "2024-09-06".into(),
        open: 110.0,
        high: 111.0,
        low: 99.0,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: -10.0,
        change_pct: -9.09,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-09-07".into(),
        open: 101.0,
        high: 112.5,
        low: 100.5,
        close: 112.0,
        adj_close: 112.0,
        volume: 1_100_000.0,
        change: 12.0,
        change_pct: 12.0,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-09-08".into(),
        open: 113.0,
        high: 113.5,
        low: 99.8,
        close: 100.4,
        adj_close: 100.4,
        volume: 1_150_000.0,
        change: -11.6,
        change_pct: -10.36,
    });
    let snap = compute_cdl_stick_sandwich_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_stick_sandwich_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.close_match_pct_body <= 5.0);
}

#[test]
fn cdl_rickshaw_man_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlRickshawManSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 20,
        pattern_value: 100,
        pattern_value_prev: 0,
        body_pct_range: 1.7,
        upper_shadow_pct: 49.0,
        lower_shadow_pct: 49.0,
        body_midpoint_offset_pct: 1.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 99.9,
        cdl_rickshaw_man_label: "RICKSHAW_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_rickshaw_man(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_rickshaw_man(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_rickshaw_man_label, "RICKSHAW_PATTERN");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_rickshaw_man_compute_detects() {
    let mut bars = boring_green_bars(5, 9);
    bars.push(HistoricalPriceRow {
        date: "2024-09-06".into(),
        open: 100.1,
        high: 106.0,
        low: 94.0,
        close: 99.9,
        adj_close: 99.9,
        volume: 1_000_000.0,
        change: -0.1,
        change_pct: -0.1,
    });
    let snap = compute_cdl_rickshaw_man_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_rickshaw_man_label, "RICKSHAW_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.body_pct_range <= 5.0);
    assert!(snap.upper_shadow_pct >= 30.0 && snap.lower_shadow_pct >= 30.0);
}

#[test]
fn cdl_takuri_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlTakuriSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 20,
        pattern_value: 100,
        pattern_value_prev: 0,
        body_pct_range: 0.5,
        upper_shadow_pct: 2.0,
        lower_shadow_pct: 97.0,
        lower_to_upper_ratio: 40.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 99.98,
        cdl_takuri_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_takuri(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_takuri(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_takuri_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
}

#[test]
fn cdl_takuri_compute_detects() {
    let mut bars = boring_green_bars(5, 9);
    bars.push(HistoricalPriceRow {
        date: "2024-09-06".into(),
        open: 100.02,
        high: 100.2,
        low: 92.0,
        close: 99.98,
        adj_close: 99.98,
        volume: 1_000_000.0,
        change: -0.02,
        change_pct: -0.02,
    });
    let snap = compute_cdl_takuri_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_takuri_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.upper_shadow_pct <= 10.0);
    assert!(snap.lower_shadow_pct >= 70.0);
}

// ── Round 81/82 tests — CDL3STARSINSOUTH /
//    CDLIDENTICAL3CROWS / CDLKICKING / CDLKICKINGBYLENGTH /
//    CDLLADDERBOTTOM / CDLUNIQUE3RIVER ──

#[test]
fn cdl_three_stars_in_south_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlThreeStarsInSouthSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 36,
        pattern_value: 100,
        pattern_value_prev: 0,
        first_body_pct_range: 47.6,
        first_lower_shadow_pct: 47.6,
        second_body_pct_range: 47.1,
        third_body_pct_range: 18.9,
        third_inside_pct_range: 25.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 102.8,
        cdl_three_stars_in_south_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_three_stars_in_south(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_three_stars_in_south(&conn, "TEST")
        .unwrap()
        .unwrap();
    assert_eq!(got.cdl_three_stars_in_south_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
    assert!(got.third_body_pct_range <= 20.0);
}

#[test]
fn cdl_three_stars_in_south_compute_detects() {
    let mut bars = boring_green_bars(5, 10);
    bars.push(HistoricalPriceRow {
        date: "2024-10-06".into(),
        open: 110.0,
        high: 111.0,
        low: 90.0,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: -10.0,
        change_pct: -9.09,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-10-07".into(),
        open: 106.0,
        high: 106.5,
        low: 98.0,
        close: 102.0,
        adj_close: 102.0,
        volume: 1_050_000.0,
        change: 2.0,
        change_pct: 2.0,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-10-08".into(),
        open: 103.5,
        high: 104.2,
        low: 100.5,
        close: 102.8,
        adj_close: 102.8,
        volume: 1_100_000.0,
        change: 0.8,
        change_pct: 0.78,
    });
    let snap = compute_cdl_three_stars_in_south_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_three_stars_in_south_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.third_body_pct_range <= 20.0);
    assert!(snap.first_lower_shadow_pct >= 40.0);
}

#[test]
fn cdl_identical_three_crows_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlIdenticalThreeCrowsSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 40,
        pattern_value: -100,
        pattern_value_prev: 0,
        avg_body_pct_range: 87.0,
        open1_vs_close0_pct_body: 5.0,
        open2_vs_close1_pct_body: 3.8,
        total_close_decline_pct: -27.3,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 80.0,
        cdl_identical_three_crows_label: "BEARISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_identical_three_crows(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_identical_three_crows(&conn, "TEST")
        .unwrap()
        .unwrap();
    assert_eq!(got.cdl_identical_three_crows_label, "BEARISH_PATTERN");
    assert_eq!(got.pattern_value, -100);
    assert!(got.open1_vs_close0_pct_body <= 10.0);
}

#[test]
fn cdl_identical_three_crows_compute_detects() {
    let mut bars = boring_green_bars(5, 10);
    bars.push(HistoricalPriceRow {
        date: "2024-10-06".into(),
        open: 110.0,
        high: 110.5,
        low: 99.0,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: -10.0,
        change_pct: -9.09,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-10-07".into(),
        open: 100.5,
        high: 101.0,
        low: 89.0,
        close: 90.0,
        adj_close: 90.0,
        volume: 1_050_000.0,
        change: -10.0,
        change_pct: -10.0,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-10-08".into(),
        open: 90.4,
        high: 90.9,
        low: 79.0,
        close: 80.0,
        adj_close: 80.0,
        volume: 1_100_000.0,
        change: -10.0,
        change_pct: -11.11,
    });
    let snap = compute_cdl_identical_three_crows_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_identical_three_crows_label, "BEARISH_PATTERN");
    assert_eq!(snap.pattern_value, -100);
    assert!(snap.open1_vs_close0_pct_body <= 10.0);
    assert!(snap.open2_vs_close1_pct_body <= 10.0);
}

#[test]
fn cdl_kicking_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlKickingSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 24,
        pattern_value: 100,
        pattern_value_prev: 0,
        first_body_pct_range: 93.8,
        second_body_pct_range: 95.2,
        gap_pct_range: 25.0,
        second_to_first_body_ratio: 1.33,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 120.0,
        cdl_kicking_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_kicking(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_kicking(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_kicking_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
    assert!(got.gap_pct_range > 0.0);
}

#[test]
fn cdl_kicking_compute_detects() {
    let mut bars = boring_green_bars(5, 11);
    bars.push(HistoricalPriceRow {
        date: "2024-11-06".into(),
        open: 110.0,
        high: 110.2,
        low: 103.8,
        close: 104.0,
        adj_close: 104.0,
        volume: 1_000_000.0,
        change: -6.0,
        change_pct: -5.45,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-11-07".into(),
        open: 112.0,
        high: 120.2,
        low: 111.8,
        close: 120.0,
        adj_close: 120.0,
        volume: 1_100_000.0,
        change: 16.0,
        change_pct: 15.38,
    });
    let snap = compute_cdl_kicking_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_kicking_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.gap_pct_range > 0.0);
    assert!(snap.second_to_first_body_ratio > 1.0);
}

#[test]
fn cdl_kicking_by_length_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlKickingByLengthSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 24,
        pattern_value: 100,
        pattern_value_prev: 0,
        first_body_pct_range: 92.6,
        second_body_pct_range: 95.7,
        gap_pct_range: 29.6,
        dominant_body_ratio: 1.8,
        dominant_side: "SECOND_BAR".into(),
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 121.0,
        cdl_kicking_by_length_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_kicking_by_length(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_kicking_by_length(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_kicking_by_length_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
    assert_eq!(got.dominant_side, "SECOND_BAR");
}

#[test]
fn cdl_kicking_by_length_compute_detects() {
    let mut bars = boring_green_bars(5, 11);
    bars.push(HistoricalPriceRow {
        date: "2024-11-06".into(),
        open: 110.0,
        high: 110.2,
        low: 104.8,
        close: 105.0,
        adj_close: 105.0,
        volume: 1_000_000.0,
        change: -5.0,
        change_pct: -4.55,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-11-07".into(),
        open: 112.0,
        high: 121.2,
        low: 111.8,
        close: 121.0,
        adj_close: 121.0,
        volume: 1_100_000.0,
        change: 16.0,
        change_pct: 15.24,
    });
    let snap = compute_cdl_kicking_by_length_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_kicking_by_length_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert_eq!(snap.dominant_side, "SECOND_BAR");
    assert!(snap.dominant_body_ratio > 1.0);
}

#[test]
fn cdl_ladder_bottom_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlLadderBottomSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 50,
        pattern_value: 100,
        pattern_value_prev: 0,
        avg_first_three_body_pct_range: 90.6,
        fourth_body_pct_range: 20.0,
        fourth_upper_shadow_pct: 60.0,
        fifth_body_pct_range: 92.3,
        breakout_pct_vs_fourth_high: 7.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 107.0,
        cdl_ladder_bottom_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_ladder_bottom(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_ladder_bottom(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_ladder_bottom_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
    assert!(got.fourth_upper_shadow_pct > 0.0);
}

#[test]
fn cdl_ladder_bottom_compute_detects() {
    let mut bars = boring_green_bars(5, 12);
    bars.push(HistoricalPriceRow {
        date: "2024-12-06".into(),
        open: 120.0,
        high: 120.5,
        low: 109.5,
        close: 110.0,
        adj_close: 110.0,
        volume: 1_000_000.0,
        change: -10.0,
        change_pct: -8.33,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-12-07".into(),
        open: 110.0,
        high: 110.4,
        low: 101.6,
        close: 102.0,
        adj_close: 102.0,
        volume: 1_050_000.0,
        change: -8.0,
        change_pct: -7.27,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-12-08".into(),
        open: 102.0,
        high: 102.3,
        low: 95.7,
        close: 96.0,
        adj_close: 96.0,
        volume: 1_100_000.0,
        change: -6.0,
        change_pct: -5.88,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-12-09".into(),
        open: 97.0,
        high: 100.0,
        low: 95.0,
        close: 96.0,
        adj_close: 96.0,
        volume: 1_080_000.0,
        change: 0.0,
        change_pct: 0.0,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-12-10".into(),
        open: 96.2,
        high: 107.5,
        low: 95.8,
        close: 107.0,
        adj_close: 107.0,
        volume: 1_150_000.0,
        change: 11.0,
        change_pct: 11.43,
    });
    let snap = compute_cdl_ladder_bottom_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_ladder_bottom_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.fourth_upper_shadow_pct > 0.0);
    assert!(snap.breakout_pct_vs_fourth_high > 0.0);
}

#[test]
fn cdl_unique_three_river_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlUniqueThreeRiverSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 32,
        pattern_value: 100,
        pattern_value_prev: 0,
        first_body_pct_range: 90.9,
        second_body_pct_range: 15.0,
        second_lower_shadow_pct: 75.0,
        third_body_pct_range: 20.0,
        third_close_vs_second_close_pct: -33.3,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 102.0,
        cdl_unique_three_river_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_unique_three_river(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_unique_three_river(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_unique_three_river_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
    assert!(got.second_lower_shadow_pct >= 50.0);
}

#[test]
fn cdl_unique_three_river_compute_detects() {
    let mut bars = boring_green_bars(5, 12);
    bars.push(HistoricalPriceRow {
        date: "2024-12-06".into(),
        open: 110.0,
        high: 110.5,
        low: 99.5,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: -10.0,
        change_pct: -9.09,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-12-07".into(),
        open: 104.0,
        high: 105.0,
        low: 95.0,
        close: 102.5,
        adj_close: 102.5,
        volume: 1_050_000.0,
        change: 2.5,
        change_pct: 2.5,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-12-08".into(),
        open: 101.5,
        high: 103.0,
        low: 100.5,
        close: 102.0,
        adj_close: 102.0,
        volume: 1_100_000.0,
        change: -0.5,
        change_pct: -0.49,
    });
    let snap = compute_cdl_unique_three_river_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_unique_three_river_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.second_lower_shadow_pct >= 50.0);
    assert!(snap.third_close_vs_second_close_pct < 0.0);
}

// ── Round 83/84 tests — CDLADVANCEBLOCK / CDLBREAKAWAY /
//    CDLGAPSIDESIDEWHITE / CDLUPSIDEGAP2CROWS /
//    CDLXSIDEGAP3METHODS / CDLCONCEALBABYSWALL ──

#[test]
fn cdl_advance_block_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlAdvanceBlockSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 36,
        pattern_value: -100,
        pattern_value_prev: 0,
        first_body_pct_range: 81.8,
        second_body_pct_range: 77.8,
        third_body_pct_range: 61.5,
        third_upper_shadow_pct: 30.8,
        total_close_gain_pct: 6.4,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 116.0,
        cdl_advance_block_label: "BEARISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_advance_block(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_advance_block(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_advance_block_label, "BEARISH_PATTERN");
    assert_eq!(got.pattern_value, -100);
    assert!(got.third_upper_shadow_pct > 15.0);
}

#[test]
fn cdl_advance_block_compute_detects() {
    let mut bars = boring_green_bars(5, 10);
    bars.push(HistoricalPriceRow {
        date: "2024-10-06".into(),
        open: 100.0,
        high: 110.0,
        low: 99.0,
        close: 109.0,
        adj_close: 109.0,
        volume: 1_000_000.0,
        change: 9.0,
        change_pct: 9.0,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-10-07".into(),
        open: 107.0,
        high: 115.0,
        low: 106.0,
        close: 114.0,
        adj_close: 114.0,
        volume: 1_050_000.0,
        change: 5.0,
        change_pct: 4.59,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-10-08".into(),
        open: 112.0,
        high: 118.0,
        low: 111.5,
        close: 116.0,
        adj_close: 116.0,
        volume: 1_100_000.0,
        change: 2.0,
        change_pct: 1.75,
    });
    let snap = compute_cdl_advance_block_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_advance_block_label, "BEARISH_PATTERN");
    assert_eq!(snap.pattern_value, -100);
    assert!(snap.third_upper_shadow_pct > 15.0);
}

#[test]
fn cdl_breakaway_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlBreakawaySnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 40,
        pattern_value: 100,
        pattern_value_prev: 0,
        first_body_pct_range: 83.3,
        initial_gap_pct_range: 25.0,
        fifth_body_pct_range: 88.0,
        gap_retracement_pct: 33.3,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 97.0,
        cdl_breakaway_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_breakaway(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_breakaway(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_breakaway_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
    assert!(got.initial_gap_pct_range > 0.0);
}

#[test]
fn cdl_breakaway_compute_detects() {
    let mut bars = boring_green_bars(5, 10);
    bars.push(HistoricalPriceRow {
        date: "2024-10-06".into(),
        open: 110.0,
        high: 111.0,
        low: 99.0,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: -10.0,
        change_pct: -9.09,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-10-07".into(),
        open: 95.0,
        high: 96.0,
        low: 90.0,
        close: 91.0,
        adj_close: 91.0,
        volume: 1_050_000.0,
        change: -9.0,
        change_pct: -9.0,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-10-08".into(),
        open: 91.0,
        high: 92.0,
        low: 87.0,
        close: 88.0,
        adj_close: 88.0,
        volume: 1_060_000.0,
        change: -3.0,
        change_pct: -3.3,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-10-09".into(),
        open: 88.0,
        high: 89.0,
        low: 85.0,
        close: 86.0,
        adj_close: 86.0,
        volume: 1_070_000.0,
        change: -2.0,
        change_pct: -2.27,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-10-10".into(),
        open: 86.0,
        high: 98.0,
        low: 85.5,
        close: 97.0,
        adj_close: 97.0,
        volume: 1_100_000.0,
        change: 11.0,
        change_pct: 12.79,
    });
    let snap = compute_cdl_breakaway_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_breakaway_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.gap_retracement_pct > 0.0);
}

#[test]
fn cdl_gap_side_side_white_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlGapSideSideWhiteSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 30,
        pattern_value: 100,
        pattern_value_prev: 0,
        gap_pct_range: 38.5,
        second_body_pct_range: 72.7,
        third_body_pct_range: 68.3,
        open_similarity_pct_body: 11.1,
        close_similarity_pct_body: 2.2,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 108.8,
        cdl_gap_side_side_white_label: "BULLISH_CONTINUATION".into(),
        note: String::new(),
    };
    upsert_cdl_gap_side_side_white(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_gap_side_side_white(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_gap_side_side_white_label, "BULLISH_CONTINUATION");
    assert_eq!(got.pattern_value, 100);
    assert!(got.open_similarity_pct_body <= 25.0);
}

#[test]
fn cdl_gap_side_side_white_compute_detects() {
    let mut bars = boring_green_bars(5, 11);
    bars.push(HistoricalPriceRow {
        date: "2024-11-06".into(),
        open: 100.0,
        high: 101.0,
        low: 99.0,
        close: 100.5,
        adj_close: 100.5,
        volume: 1_000_000.0,
        change: 0.5,
        change_pct: 0.5,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-11-07".into(),
        open: 105.0,
        high: 110.0,
        low: 104.5,
        close: 109.0,
        adj_close: 109.0,
        volume: 1_050_000.0,
        change: 8.5,
        change_pct: 8.46,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-11-08".into(),
        open: 105.5,
        high: 110.5,
        low: 104.8,
        close: 108.8,
        adj_close: 108.8,
        volume: 1_060_000.0,
        change: -0.2,
        change_pct: -0.18,
    });
    let snap = compute_cdl_gap_side_side_white_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_gap_side_side_white_label, "BULLISH_CONTINUATION");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.close_similarity_pct_body <= 35.0);
}

#[test]
fn cdl_upside_gap_two_crows_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlUpsideGapTwoCrowsSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 28,
        pattern_value: -100,
        pattern_value_prev: 0,
        first_body_pct_range: 81.8,
        upside_gap_pct_range: 9.1,
        third_open_above_second_pct_body: 50.0,
        third_close_into_gap_pct: 16.7,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 110.5,
        cdl_upside_gap_two_crows_label: "BEARISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_upside_gap_two_crows(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_upside_gap_two_crows(&conn, "TEST")
        .unwrap()
        .unwrap();
    assert_eq!(got.cdl_upside_gap_two_crows_label, "BEARISH_PATTERN");
    assert_eq!(got.pattern_value, -100);
    assert!(got.third_close_into_gap_pct > 0.0);
}

#[test]
fn cdl_upside_gap_two_crows_compute_detects() {
    let mut bars = boring_green_bars(5, 11);
    bars.push(HistoricalPriceRow {
        date: "2024-11-06".into(),
        open: 100.0,
        high: 110.0,
        low: 99.0,
        close: 109.0,
        adj_close: 109.0,
        volume: 1_000_000.0,
        change: 9.0,
        change_pct: 9.0,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-11-07".into(),
        open: 114.0,
        high: 115.0,
        low: 111.0,
        close: 112.0,
        adj_close: 112.0,
        volume: 1_050_000.0,
        change: 3.0,
        change_pct: 2.75,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-11-08".into(),
        open: 116.0,
        high: 116.5,
        low: 107.5,
        close: 110.5,
        adj_close: 110.5,
        volume: 1_060_000.0,
        change: -1.5,
        change_pct: -1.34,
    });
    let snap = compute_cdl_upside_gap_two_crows_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_upside_gap_two_crows_label, "BEARISH_PATTERN");
    assert_eq!(snap.pattern_value, -100);
    assert!(snap.upside_gap_pct_range > 0.0);
}

#[test]
fn cdl_xside_gap_three_methods_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlXSideGapThreeMethodsSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 28,
        pattern_value: 100,
        pattern_value_prev: 0,
        gap_pct_range: 22.2,
        second_body_pct_range: 71.4,
        third_body_pct_range: 68.8,
        gap_fill_pct: 33.3,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 110.5,
        cdl_xside_gap_three_methods_label: "BULLISH_CONTINUATION".into(),
        note: String::new(),
    };
    upsert_cdl_xside_gap_three_methods(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_xside_gap_three_methods(&conn, "TEST")
        .unwrap()
        .unwrap();
    assert_eq!(
        got.cdl_xside_gap_three_methods_label,
        "BULLISH_CONTINUATION"
    );
    assert_eq!(got.pattern_value, 100);
    assert!(got.gap_fill_pct > 0.0);
}

#[test]
fn cdl_xside_gap_three_methods_compute_detects() {
    let mut bars = boring_green_bars(5, 12);
    bars.push(HistoricalPriceRow {
        date: "2024-12-06".into(),
        open: 100.0,
        high: 109.0,
        low: 99.0,
        close: 108.5,
        adj_close: 108.5,
        volume: 1_000_000.0,
        change: 8.5,
        change_pct: 8.5,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-12-07".into(),
        open: 112.0,
        high: 118.0,
        low: 111.5,
        close: 117.0,
        adj_close: 117.0,
        volume: 1_050_000.0,
        change: 8.5,
        change_pct: 7.83,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-12-08".into(),
        open: 116.0,
        high: 116.5,
        low: 109.5,
        close: 110.5,
        adj_close: 110.5,
        volume: 1_060_000.0,
        change: -6.5,
        change_pct: -5.56,
    });
    let snap = compute_cdl_xside_gap_three_methods_snapshot("T", "2026-04-20", &bars);
    assert_eq!(
        snap.cdl_xside_gap_three_methods_label,
        "BULLISH_CONTINUATION"
    );
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.gap_fill_pct > 0.0);
}

#[test]
fn cdl_conceal_baby_swallow_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlConcealBabySwallowSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-20".into(),
        bars_used: 26,
        pattern_value: 100,
        pattern_value_prev: 0,
        first_body_pct_range: 96.2,
        second_body_pct_range: 98.0,
        third_upper_shadow_pct: 23.1,
        fourth_range_engulf_pct: 34.6,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 84.5,
        cdl_conceal_baby_swallow_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_conceal_baby_swallow(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_conceal_baby_swallow(&conn, "TEST")
        .unwrap()
        .unwrap();
    assert_eq!(got.cdl_conceal_baby_swallow_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
    assert!(got.third_upper_shadow_pct >= 20.0);
}

#[test]
fn cdl_conceal_baby_swallow_compute_detects() {
    let mut bars = boring_green_bars(5, 12);
    bars.push(HistoricalPriceRow {
        date: "2024-12-06".into(),
        open: 110.0,
        high: 110.2,
        low: 99.8,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: -10.0,
        change_pct: -9.09,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-12-07".into(),
        open: 100.0,
        high: 100.1,
        low: 89.9,
        close: 90.0,
        adj_close: 90.0,
        volume: 1_050_000.0,
        change: -10.0,
        change_pct: -10.0,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-12-08".into(),
        open: 88.0,
        high: 89.2,
        low: 84.0,
        close: 86.0,
        adj_close: 86.0,
        volume: 1_060_000.0,
        change: -4.0,
        change_pct: -4.44,
    });
    bars.push(HistoricalPriceRow {
        date: "2024-12-09".into(),
        open: 89.5,
        high: 90.0,
        low: 83.0,
        close: 84.5,
        adj_close: 84.5,
        volume: 1_070_000.0,
        change: -1.5,
        change_pct: -1.74,
    });
    let snap = compute_cdl_conceal_baby_swallow_snapshot("T", "2026-04-20", &bars);
    assert_eq!(snap.cdl_conceal_baby_swallow_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.fourth_range_engulf_pct > 0.0);
}

// ── Round 85/86 tests — CDLHIKKAKE / CDLHIKKAKEMOD /
//    CDLMATHOLD / CDLRISEFALL3METHODS ──

#[test]
fn cdl_hikkake_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlHikkakeSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-21".into(),
        bars_used: 24,
        pattern_value: 100,
        pattern_value_prev: 0,
        inside_width_pct_mother: 45.0,
        false_break_extension_pct: 18.2,
        trigger_body_pct_range: 54.5,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 104.5,
        cdl_hikkake_label: "BULLISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_hikkake(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_hikkake(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_hikkake_label, "BULLISH_PATTERN");
    assert_eq!(got.pattern_value, 100);
    assert!(got.false_break_extension_pct > 0.0);
}

#[test]
fn cdl_hikkake_compute_detects() {
    let mut bars = boring_green_bars(5, 13);
    bars.push(HistoricalPriceRow {
        date: "2025-01-06".into(),
        open: 100.0,
        high: 110.0,
        low: 95.0,
        close: 108.0,
        adj_close: 108.0,
        volume: 1_000_000.0,
        change: 8.0,
        change_pct: 8.0,
    });
    bars.push(HistoricalPriceRow {
        date: "2025-01-07".into(),
        open: 103.0,
        high: 107.0,
        low: 99.0,
        close: 104.0,
        adj_close: 104.0,
        volume: 1_050_000.0,
        change: -4.0,
        change_pct: -3.7,
    });
    bars.push(HistoricalPriceRow {
        date: "2025-01-08".into(),
        open: 98.0,
        high: 106.0,
        low: 96.0,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_060_000.0,
        change: -4.0,
        change_pct: -3.85,
    });
    let snap = compute_cdl_hikkake_snapshot("T", "2026-04-21", &bars);
    assert_eq!(snap.cdl_hikkake_label, "BULLISH_PATTERN");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.false_break_extension_pct > 0.0);
}

#[test]
fn cdl_hikkake_mod_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlHikkakeModSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-21".into(),
        bars_used: 25,
        pattern_value: -100,
        pattern_value_prev: 0,
        inside_width_pct_mother: 40.0,
        false_break_extension_pct: 22.0,
        confirmation_extension_pct: 18.5,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 96.5,
        cdl_hikkake_mod_label: "BEARISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_hikkake_mod(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_hikkake_mod(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_hikkake_mod_label, "BEARISH_PATTERN");
    assert_eq!(got.pattern_value, -100);
    assert!(got.confirmation_extension_pct > 0.0);
}

#[test]
fn cdl_hikkake_mod_compute_detects() {
    let mut bars = boring_green_bars(5, 14);
    bars.push(HistoricalPriceRow {
        date: "2025-01-06".into(),
        open: 100.0,
        high: 110.0,
        low: 96.0,
        close: 108.0,
        adj_close: 108.0,
        volume: 1_000_000.0,
        change: 8.0,
        change_pct: 8.0,
    });
    bars.push(HistoricalPriceRow {
        date: "2025-01-07".into(),
        open: 104.0,
        high: 108.0,
        low: 100.0,
        close: 103.0,
        adj_close: 103.0,
        volume: 1_050_000.0,
        change: -5.0,
        change_pct: -4.63,
    });
    bars.push(HistoricalPriceRow {
        date: "2025-01-08".into(),
        open: 109.0,
        high: 112.0,
        low: 101.0,
        close: 102.0,
        adj_close: 102.0,
        volume: 1_060_000.0,
        change: -1.0,
        change_pct: -0.97,
    });
    bars.push(HistoricalPriceRow {
        date: "2025-01-09".into(),
        open: 101.0,
        high: 102.0,
        low: 94.0,
        close: 98.0,
        adj_close: 98.0,
        volume: 1_080_000.0,
        change: -4.0,
        change_pct: -3.92,
    });
    let snap = compute_cdl_hikkake_mod_snapshot("T", "2026-04-21", &bars);
    assert_eq!(snap.cdl_hikkake_mod_label, "BEARISH_PATTERN");
    assert_eq!(snap.pattern_value, -100);
    assert!(snap.confirmation_extension_pct > 0.0);
}

#[test]
fn cdl_mat_hold_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlMatHoldSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-21".into(),
        bars_used: 32,
        pattern_value: 100,
        pattern_value_prev: 0,
        first_body_pct_range: 81.8,
        middle_avg_body_pct_range: 19.4,
        initial_gap_pct_range: 13.6,
        hold_depth_pct_body: 26.3,
        final_body_pct_range: 83.3,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 122.0,
        cdl_mat_hold_label: "BULLISH_CONTINUATION".into(),
        note: String::new(),
    };
    upsert_cdl_mat_hold(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_mat_hold(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_mat_hold_label, "BULLISH_CONTINUATION");
    assert_eq!(got.pattern_value, 100);
    assert!(got.initial_gap_pct_range > 0.0);
}

#[test]
fn cdl_mat_hold_compute_detects() {
    let mut bars = boring_green_bars(5, 14);
    bars.push(HistoricalPriceRow {
        date: "2025-02-03".into(),
        open: 100.0,
        high: 111.0,
        low: 99.0,
        close: 110.0,
        adj_close: 110.0,
        volume: 1_000_000.0,
        change: 10.0,
        change_pct: 10.0,
    });
    bars.push(HistoricalPriceRow {
        date: "2025-02-04".into(),
        open: 113.0,
        high: 116.0,
        low: 112.5,
        close: 114.0,
        adj_close: 114.0,
        volume: 1_050_000.0,
        change: 4.0,
        change_pct: 3.64,
    });
    bars.push(HistoricalPriceRow {
        date: "2025-02-05".into(),
        open: 113.2,
        high: 114.5,
        low: 108.5,
        close: 112.2,
        adj_close: 112.2,
        volume: 1_060_000.0,
        change: -1.8,
        change_pct: -1.58,
    });
    bars.push(HistoricalPriceRow {
        date: "2025-02-06".into(),
        open: 112.0,
        high: 114.2,
        low: 109.0,
        close: 111.2,
        adj_close: 111.2,
        volume: 1_070_000.0,
        change: -1.0,
        change_pct: -0.89,
    });
    bars.push(HistoricalPriceRow {
        date: "2025-02-07".into(),
        open: 113.5,
        high: 123.0,
        low: 113.2,
        close: 122.0,
        adj_close: 122.0,
        volume: 1_100_000.0,
        change: 10.8,
        change_pct: 9.71,
    });
    let snap = compute_cdl_mat_hold_snapshot("T", "2026-04-21", &bars);
    assert_eq!(snap.cdl_mat_hold_label, "BULLISH_CONTINUATION");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.middle_avg_body_pct_range <= 35.0);
}

#[test]
fn cdl_rise_fall_three_methods_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlRiseFallThreeMethodsSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-21".into(),
        bars_used: 34,
        pattern_value: -100,
        pattern_value_prev: 0,
        first_body_pct_range: 83.3,
        middle_avg_body_pct_range: 21.0,
        containment_pct_body: 28.0,
        final_body_pct_range: 78.6,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 96.0,
        cdl_rise_fall_three_methods_label: "BEARISH_CONTINUATION".into(),
        note: String::new(),
    };
    upsert_cdl_rise_fall_three_methods(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_rise_fall_three_methods(&conn, "TEST")
        .unwrap()
        .unwrap();
    assert_eq!(
        got.cdl_rise_fall_three_methods_label,
        "BEARISH_CONTINUATION"
    );
    assert_eq!(got.pattern_value, -100);
    assert!(got.containment_pct_body > 0.0);
}

#[test]
fn cdl_rise_fall_three_methods_compute_detects() {
    let mut bars = boring_green_bars(5, 15);
    bars.push(HistoricalPriceRow {
        date: "2025-03-03".into(),
        open: 110.0,
        high: 111.0,
        low: 99.0,
        close: 100.0,
        adj_close: 100.0,
        volume: 1_000_000.0,
        change: -10.0,
        change_pct: -9.09,
    });
    bars.push(HistoricalPriceRow {
        date: "2025-03-04".into(),
        open: 101.2,
        high: 104.0,
        low: 100.8,
        close: 102.0,
        adj_close: 102.0,
        volume: 1_050_000.0,
        change: 2.0,
        change_pct: 2.0,
    });
    bars.push(HistoricalPriceRow {
        date: "2025-03-05".into(),
        open: 102.2,
        high: 104.8,
        low: 101.8,
        close: 103.0,
        adj_close: 103.0,
        volume: 1_060_000.0,
        change: 1.0,
        change_pct: 0.98,
    });
    bars.push(HistoricalPriceRow {
        date: "2025-03-06".into(),
        open: 101.8,
        high: 103.8,
        low: 101.4,
        close: 102.4,
        adj_close: 102.4,
        volume: 1_070_000.0,
        change: -0.6,
        change_pct: -0.58,
    });
    bars.push(HistoricalPriceRow {
        date: "2025-03-07".into(),
        open: 101.0,
        high: 101.5,
        low: 95.0,
        close: 96.0,
        adj_close: 96.0,
        volume: 1_100_000.0,
        change: -6.5,
        change_pct: -6.34,
    });
    let snap = compute_cdl_rise_fall_three_methods_snapshot("T", "2026-04-21", &bars);
    assert_eq!(
        snap.cdl_rise_fall_three_methods_label,
        "BEARISH_CONTINUATION"
    );
    assert_eq!(snap.pattern_value, -100);
    assert!(snap.middle_avg_body_pct_range <= 35.0);
}

// ── Round 87/88 tests — CDLSTALLEDPATTERN /
//    CDLTASUKIGAP ──

#[test]
fn cdl_stalled_pattern_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlStalledPatternSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-21".into(),
        bars_used: 22,
        pattern_value: -100,
        pattern_value_prev: 0,
        first_body_pct_range: 81.8,
        second_body_pct_range: 77.8,
        third_body_pct_range: 22.5,
        third_open_gap_pct_range: 11.1,
        third_upper_shadow_pct: 48.6,
        close_progress_pct_prev_leg: 28.6,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 118.0,
        cdl_stalled_pattern_label: "BEARISH_PATTERN".into(),
        note: String::new(),
    };
    upsert_cdl_stalled_pattern(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_stalled_pattern(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_stalled_pattern_label, "BEARISH_PATTERN");
    assert_eq!(got.pattern_value, -100);
    assert!(got.third_upper_shadow_pct > 15.0);
}

#[test]
fn cdl_stalled_pattern_compute_detects() {
    let mut bars = boring_green_bars(5, 16);
    bars.push(HistoricalPriceRow {
        date: "2025-04-01".into(),
        open: 100.0,
        high: 110.0,
        low: 99.0,
        close: 109.0,
        adj_close: 109.0,
        volume: 1_000_000.0,
        change: 9.0,
        change_pct: 9.0,
    });
    bars.push(HistoricalPriceRow {
        date: "2025-04-02".into(),
        open: 108.0,
        high: 117.0,
        low: 107.5,
        close: 116.0,
        adj_close: 116.0,
        volume: 1_050_000.0,
        change: 7.0,
        change_pct: 6.42,
    });
    bars.push(HistoricalPriceRow {
        date: "2025-04-03".into(),
        open: 117.0,
        high: 120.5,
        low: 116.8,
        close: 118.0,
        adj_close: 118.0,
        volume: 1_060_000.0,
        change: 2.0,
        change_pct: 1.72,
    });
    let snap = compute_cdl_stalled_pattern_snapshot("T", "2026-04-21", &bars);
    assert_eq!(snap.cdl_stalled_pattern_label, "BEARISH_PATTERN");
    assert_eq!(snap.pattern_value, -100);
    assert!(snap.close_progress_pct_prev_leg < 60.0);
}

#[test]
fn cdl_tasuki_gap_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CdlTasukiGapSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-21".into(),
        bars_used: 24,
        pattern_value: 100,
        pattern_value_prev: 0,
        first_body_pct_range: 81.8,
        second_body_pct_range: 66.7,
        third_body_pct_range: 66.7,
        gap_pct_range: 27.3,
        gap_fill_pct: 66.7,
        third_open_pct_second_body: 75.0,
        last_bar_match: true,
        days_since_pattern: 0,
        last_close: 111.0,
        cdl_tasuki_gap_label: "BULLISH_CONTINUATION".into(),
        note: String::new(),
    };
    upsert_cdl_tasuki_gap(&conn, "TEST", &snap).unwrap();
    let got = get_cdl_tasuki_gap(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cdl_tasuki_gap_label, "BULLISH_CONTINUATION");
    assert_eq!(got.pattern_value, 100);
    assert!(got.gap_fill_pct > 0.0);
}

#[test]
fn cdl_tasuki_gap_compute_detects() {
    let mut bars = boring_green_bars(5, 16);
    bars.push(HistoricalPriceRow {
        date: "2025-04-01".into(),
        open: 100.0,
        high: 110.0,
        low: 99.0,
        close: 109.0,
        adj_close: 109.0,
        volume: 1_000_000.0,
        change: 9.0,
        change_pct: 9.0,
    });
    bars.push(HistoricalPriceRow {
        date: "2025-04-02".into(),
        open: 114.0,
        high: 119.0,
        low: 113.0,
        close: 118.0,
        adj_close: 118.0,
        volume: 1_050_000.0,
        change: 9.0,
        change_pct: 8.26,
    });
    bars.push(HistoricalPriceRow {
        date: "2025-04-03".into(),
        open: 117.0,
        high: 117.5,
        low: 110.5,
        close: 111.0,
        adj_close: 111.0,
        volume: 1_060_000.0,
        change: -7.0,
        change_pct: -5.93,
    });
    let snap = compute_cdl_tasuki_gap_snapshot("T", "2026-04-21", &bars);
    assert_eq!(snap.cdl_tasuki_gap_label, "BULLISH_CONTINUATION");
    assert_eq!(snap.pattern_value, 100);
    assert!(snap.gap_fill_pct > 0.0 && snap.gap_fill_pct < 100.0);
}

