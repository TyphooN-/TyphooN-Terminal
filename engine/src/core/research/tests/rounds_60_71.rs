// ── Round 60 tests ───────────────────────────────────────────

#[test]
fn wma_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = WmaSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        length: 20,
        wma_value: 101.2,
        wma_prev: 101.0,
        sma_value: 100.8,
        spread: 0.4,
        spread_pct: 0.004,
        last_close: 101.6,
        wma_label: "BULL".into(),
        note: String::new(),
    };
    upsert_wma(&conn, "TEST", &snap).unwrap();
    let got = get_wma(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.wma_label, "BULL");
    assert!((got.wma_value - 101.2).abs() < 1e-6);
}

#[test]
fn wma_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_wma_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.wma_label.as_str(),
        "BULL" | "WEAK_BULL" | "NEUTRAL" | "WEAK_BEAR" | "BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.wma_label != "INSUFFICIENT_DATA" {
        assert!(snap.wma_value.is_finite());
        assert!(snap.sma_value.is_finite());
    }
}

#[test]
fn rainbow_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = RainbowSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        levels: 10,
        highest_level: 105.0,
        lowest_level: 95.0,
        rainbow_width: 10.0,
        rainbow_width_pct: 0.1,
        center_value: 100.0,
        r1: 101.0,
        r5: 100.0,
        r10: 99.0,
        last_close: 101.5,
        rainbow_label: "STRONG_TREND".into(),
        note: String::new(),
    };
    upsert_rainbow(&conn, "TEST", &snap).unwrap();
    let got = get_rainbow(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.rainbow_label, "STRONG_TREND");
    assert!((got.rainbow_width - 10.0).abs() < 1e-6);
}

#[test]
fn rainbow_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_rainbow_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.rainbow_label.as_str(),
        "STRONG_TREND" | "TRENDING" | "CONSOLIDATING" | "INSUFFICIENT_DATA"
    ));
    if snap.rainbow_label != "INSUFFICIENT_DATA" {
        assert!(snap.rainbow_width >= 0.0);
        assert!(snap.highest_level >= snap.lowest_level);
    }
}

#[test]
fn mesa_sine_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MesaSineSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        period: 20.0,
        phase_rad: 0.5,
        sine_value: 0.479,
        lead_sine: 0.894,
        sine_prev: 0.412,
        lead_prev: 0.866,
        last_close: 100.0,
        mesa_label: "TRENDING".into(),
        note: String::new(),
    };
    upsert_mesa_sine(&conn, "TEST", &snap).unwrap();
    let got = get_mesa_sine(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.mesa_label, "TRENDING");
    assert!((got.sine_value - 0.479).abs() < 1e-6);
}

#[test]
fn mesa_sine_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_mesa_sine_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.mesa_label.as_str(),
        "CYCLE_BUY" | "CYCLE_SELL" | "TRENDING" | "NEUTRAL" | "INSUFFICIENT_DATA"
    ));
    if snap.mesa_label != "INSUFFICIENT_DATA" {
        assert!(snap.sine_value.abs() <= 1.000001);
        assert!(snap.lead_sine.abs() <= 1.000001);
    }
}

#[test]
fn frama_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = FramaSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        length: 16,
        fractal_dim: 1.25,
        alpha: 0.31,
        frama_value: 100.5,
        frama_prev: 100.3,
        spread: 0.7,
        last_close: 101.2,
        frama_label: "STRONG_TREND".into(),
        note: String::new(),
    };
    upsert_frama(&conn, "TEST", &snap).unwrap();
    let got = get_frama(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.frama_label, "STRONG_TREND");
    assert!((got.fractal_dim - 1.25).abs() < 1e-6);
}

#[test]
fn frama_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_frama_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.frama_label.as_str(),
        "STRONG_TREND" | "TREND" | "CHOP" | "INSUFFICIENT_DATA"
    ));
    if snap.frama_label != "INSUFFICIENT_DATA" {
        assert!(snap.fractal_dim >= 1.0 && snap.fractal_dim <= 2.0);
        assert!(snap.alpha >= 0.01 && snap.alpha <= 1.0);
    }
}

#[test]
fn ibs_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = IbsSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        length: 14,
        ibs_raw: 0.85,
        ibs_smoothed: 0.72,
        ibs_prev: 0.68,
        last_high: 102.0,
        last_low: 100.0,
        last_close: 101.7,
        ibs_label: "OVERBOUGHT".into(),
        note: String::new(),
    };
    upsert_ibs(&conn, "TEST", &snap).unwrap();
    let got = get_ibs(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.ibs_label, "OVERBOUGHT");
    assert!((got.ibs_raw - 0.85).abs() < 1e-6);
}

#[test]
fn ibs_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_ibs_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.ibs_label.as_str(),
        "OVERBOUGHT" | "BULL" | "NEUTRAL" | "BEAR" | "OVERSOLD" | "INSUFFICIENT_DATA"
    ));
    if snap.ibs_label != "INSUFFICIENT_DATA" {
        assert!(snap.ibs_raw >= 0.0 && snap.ibs_raw <= 1.0);
        assert!(snap.ibs_smoothed >= 0.0 && snap.ibs_smoothed <= 1.0);
    }
}

#[test]
fn laguerre_rsi_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = LaguerreRsiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        gamma: 0.5,
        l0: 100.1,
        l1: 100.05,
        l2: 100.0,
        l3: 99.95,
        laguerre_rsi: 0.78,
        laguerre_rsi_prev: 0.72,
        last_close: 101.0,
        lrsi_label: "BULL".into(),
        note: String::new(),
    };
    upsert_laguerre_rsi(&conn, "TEST", &snap).unwrap();
    let got = get_laguerre_rsi(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.lrsi_label, "BULL");
    assert!((got.laguerre_rsi - 0.78).abs() < 1e-6);
}

#[test]
fn laguerre_rsi_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_laguerre_rsi_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.lrsi_label.as_str(),
        "OVERBOUGHT" | "BULL" | "NEUTRAL" | "BEAR" | "OVERSOLD" | "INSUFFICIENT_DATA"
    ));
    if snap.lrsi_label != "INSUFFICIENT_DATA" {
        assert!(snap.laguerre_rsi >= 0.0 && snap.laguerre_rsi <= 1.0);
        assert!((snap.gamma - 0.5).abs() < 1e-9);
    }
}

#[test]
fn zigzag_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = ZigzagSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        threshold_pct: 5.0,
        last_high_value: 105.2,
        last_high_bars_ago: 3,
        last_low_value: 98.5,
        last_low_bars_ago: 12,
        current_leg: "UP".into(),
        reversal_level: 99.94,
        last_close: 104.0,
        zigzag_label: "UP_LEG".into(),
        note: String::new(),
    };
    upsert_zigzag(&conn, "TEST", &snap).unwrap();
    let got = get_zigzag(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.zigzag_label, "UP_LEG");
    assert_eq!(got.current_leg, "UP");
    assert!((got.threshold_pct - 5.0).abs() < 1e-9);
}

#[test]
fn zigzag_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_zigzag_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.zigzag_label.as_str(),
        "UP_LEG" | "DOWN_LEG" | "AT_REVERSAL" | "INSUFFICIENT_DATA"
    ));
    if snap.zigzag_label != "INSUFFICIENT_DATA" {
        assert!(matches!(snap.current_leg.as_str(), "UP" | "DOWN"));
        assert!(snap.last_high_value >= snap.last_low_value);
    }
}

#[test]
fn pgo_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = PgoSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        length: 14,
        sma_value: 100.0,
        atr_value: 1.5,
        pgo_value: 2.3,
        pgo_prev: 1.8,
        last_close: 103.45,
        pgo_label: "BULL".into(),
        note: String::new(),
    };
    upsert_pgo(&conn, "TEST", &snap).unwrap();
    let got = get_pgo(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.pgo_label, "BULL");
    assert!((got.pgo_value - 2.3).abs() < 1e-6);
}

#[test]
fn pgo_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_pgo_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.pgo_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.pgo_label != "INSUFFICIENT_DATA" {
        assert_eq!(snap.length, 14);
        assert!(snap.atr_value.is_finite() && snap.atr_value >= 0.0);
    }
}

#[test]
fn ht_trendline_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = HtTrendlineSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        period: 22.0,
        trendline_value: 100.4,
        trendline_prev: 100.2,
        spread: 0.6,
        spread_pct: 0.006,
        last_close: 101.0,
        ht_label: "WEAK_BULL".into(),
        note: String::new(),
    };
    upsert_ht_trendline(&conn, "TEST", &snap).unwrap();
    let got = get_ht_trendline(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.ht_label, "WEAK_BULL");
    assert!((got.period - 22.0).abs() < 1e-6);
}

#[test]
fn ht_trendline_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_ht_trendline_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.ht_label.as_str(),
        "BULL" | "WEAK_BULL" | "NEUTRAL" | "WEAK_BEAR" | "BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.ht_label != "INSUFFICIENT_DATA" {
        assert!(snap.period >= 6.0 && snap.period <= 50.0);
        assert!(snap.trendline_value.is_finite());
    }
}

#[test]
fn midpoint_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MidpointSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        length: 14,
        hhv: 105.0,
        llv: 95.0,
        midpoint: 100.0,
        midpoint_prev: 99.8,
        close_position: 0.72,
        last_close: 102.2,
        midpoint_label: "NEAR_UPPER".into(),
        note: String::new(),
    };
    upsert_midpoint(&conn, "TEST", &snap).unwrap();
    let got = get_midpoint(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.midpoint_label, "NEAR_UPPER");
    assert!((got.midpoint - 100.0).abs() < 1e-6);
}

#[test]
fn midpoint_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_midpoint_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.midpoint_label.as_str(),
        "UPPER" | "NEAR_UPPER" | "MIDRANGE" | "NEAR_LOWER" | "LOWER" | "INSUFFICIENT_DATA"
    ));
    if snap.midpoint_label != "INSUFFICIENT_DATA" {
        assert!(snap.hhv >= snap.llv);
        assert!(snap.close_position >= 0.0 && snap.close_position <= 1.0);
    }
}

// ── Round 62 tests ──

#[test]
fn mass_index_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MassIndexSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 80,
        ema_len: 9,
        sum_len: 25,
        ema_range: 2.4,
        ema_ema_range: 2.2,
        ratio: 1.0909,
        mass_index: 26.5,
        mass_index_prev: 26.2,
        last_close: 100.0,
        mass_label: "ELEVATED".into(),
        note: String::new(),
    };
    upsert_mass_index(&conn, "TEST", &snap).unwrap();
    let got = get_mass_index(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.mass_label, "ELEVATED");
    assert!((got.mass_index - 26.5).abs() < 1e-6);
    assert_eq!(got.ema_len, 9);
    assert_eq!(got.sum_len, 25);
}

#[test]
fn mass_index_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_mass_index_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.mass_label.as_str(),
        "REVERSAL_BULGE" | "ELEVATED" | "NEUTRAL" | "COMPRESSED" | "INSUFFICIENT_DATA"
    ));
    if snap.mass_label != "INSUFFICIENT_DATA" {
        assert!(snap.ratio > 0.0);
        assert!(snap.mass_index > 0.0);
        assert!(snap.ema_range >= 0.0);
        assert!(snap.ema_ema_range >= 0.0);
    }
}

#[test]
fn natr_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = NatrSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 50,
        length: 14,
        atr_value: 2.5,
        natr_value: 2.5,
        natr_prev: 2.4,
        last_close: 100.0,
        natr_label: "ELEVATED".into(),
        note: String::new(),
    };
    upsert_natr(&conn, "TEST", &snap).unwrap();
    let got = get_natr(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.natr_label, "ELEVATED");
    assert!((got.natr_value - 2.5).abs() < 1e-6);
    assert_eq!(got.length, 14);
}

#[test]
fn natr_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_natr_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.natr_label.as_str(),
        "HIGH_VOL" | "ELEVATED" | "NORMAL" | "LOW_VOL" | "INSUFFICIENT_DATA"
    ));
    if snap.natr_label != "INSUFFICIENT_DATA" {
        assert!(snap.atr_value >= 0.0);
        assert!(snap.natr_value >= 0.0);
    }
}

#[test]
fn ttm_squeeze_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = TtmSqueezeSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 60,
        length: 20,
        bb_upper: 102.0,
        bb_lower: 98.0,
        kc_upper: 103.0,
        kc_lower: 97.0,
        squeeze_on: true,
        momentum: 0.5,
        momentum_prev: 0.4,
        last_close: 100.0,
        squeeze_label: "SQUEEZE_ON".into(),
        note: String::new(),
    };
    upsert_ttm_squeeze(&conn, "TEST", &snap).unwrap();
    let got = get_ttm_squeeze(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.squeeze_label, "SQUEEZE_ON");
    assert!(got.squeeze_on);
    assert_eq!(got.length, 20);
}

#[test]
fn ttm_squeeze_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_ttm_squeeze_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.squeeze_label.as_str(),
        "SQUEEZE_ON" | "FIRE_UP" | "FIRE_DOWN" | "NEUTRAL" | "INSUFFICIENT_DATA"
    ));
    if snap.squeeze_label != "INSUFFICIENT_DATA" {
        assert!(snap.bb_upper >= snap.bb_lower);
        assert!(snap.kc_upper >= snap.kc_lower);
    }
}

#[test]
fn force_index_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = ForceIndexSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 40,
        length: 13,
        force_raw: 1200.0,
        force_ema: 1100.0,
        force_ema_prev: 1050.0,
        last_close: 100.0,
        last_volume: 1000.0,
        force_label: "BULL".into(),
        note: String::new(),
    };
    upsert_force_index(&conn, "TEST", &snap).unwrap();
    let got = get_force_index(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.force_label, "BULL");
    assert!((got.force_ema - 1100.0).abs() < 1e-6);
    assert_eq!(got.length, 13);
}

#[test]
fn force_index_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_force_index_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.force_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
}

#[test]
fn trange_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = TrangeSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 30,
        trange_value: 1.5,
        trange_prev: 1.4,
        mean_trange_20: 1.3,
        trange_ratio: 1.1538,
        last_high: 101.0,
        last_low: 99.5,
        last_close: 100.0,
        prev_close: 99.8,
        trange_label: "NORMAL".into(),
        note: String::new(),
    };
    upsert_trange(&conn, "TEST", &snap).unwrap();
    let got = get_trange(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.trange_label, "NORMAL");
    assert!((got.trange_value - 1.5).abs() < 1e-6);
}

#[test]
fn trange_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_trange_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.trange_label.as_str(),
        "EXPANSION" | "NORMAL" | "CONTRACTION" | "INSUFFICIENT_DATA"
    ));
    if snap.trange_label != "INSUFFICIENT_DATA" {
        assert!(snap.trange_value >= 0.0);
        assert!(snap.mean_trange_20 >= 0.0);
    }
}

// ── Round 63 tests ──

#[test]
fn linearreg_slope_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = LinearregSlopeSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 50,
        length: 14,
        slope: 0.25,
        slope_prev: 0.22,
        slope_pct: 0.25,
        last_close: 100.0,
        slope_label: "UP".into(),
        note: String::new(),
    };
    upsert_linearreg_slope(&conn, "TEST", &snap).unwrap();
    let got = get_linearreg_slope(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.slope_label, "UP");
    assert!((got.slope - 0.25).abs() < 1e-6);
    assert_eq!(got.length, 14);
}

#[test]
fn linearreg_slope_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_linearreg_slope_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.slope_label.as_str(),
        "STRONG_UP" | "UP" | "FLAT" | "DOWN" | "STRONG_DOWN" | "INSUFFICIENT_DATA"
    ));
}

#[test]
fn ht_dcperiod_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = HtDcperiodSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 120,
        period: 18.5,
        period_prev: 18.2,
        period_min_64: 12.0,
        period_max_64: 24.0,
        last_close: 100.0,
        period_label: "MEDIUM".into(),
        note: String::new(),
    };
    upsert_ht_dcperiod(&conn, "TEST", &snap).unwrap();
    let got = get_ht_dcperiod(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.period_label, "MEDIUM");
    assert!((got.period - 18.5).abs() < 1e-6);
}

#[test]
fn ht_dcperiod_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_ht_dcperiod_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.period_label.as_str(),
        "VERY_SHORT" | "SHORT" | "MEDIUM" | "LONG" | "VERY_LONG" | "INSUFFICIENT_DATA"
    ));
    if snap.period_label != "INSUFFICIENT_DATA" {
        assert!(snap.period >= 6.0 && snap.period <= 50.0);
    }
}

#[test]
fn ht_trendmode_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = HtTrendmodeSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 120,
        trendmode: 1,
        trendmode_prev: 0,
        lock_in_bars: 5,
        period: 25.0,
        last_close: 100.0,
        mode_label: "TREND".into(),
        note: String::new(),
    };
    upsert_ht_trendmode(&conn, "TEST", &snap).unwrap();
    let got = get_ht_trendmode(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.mode_label, "TREND");
    assert_eq!(got.trendmode, 1);
    assert_eq!(got.lock_in_bars, 5);
}

#[test]
fn ht_trendmode_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_ht_trendmode_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.mode_label.as_str(),
        "TREND" | "CYCLE" | "INSUFFICIENT_DATA"
    ));
    if snap.mode_label != "INSUFFICIENT_DATA" {
        assert!(snap.trendmode == 0 || snap.trendmode == 1);
        assert!(snap.lock_in_bars >= 1);
    }
}

#[test]
fn accbands_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = AccbandsSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 40,
        length: 20,
        acc_upper: 105.0,
        acc_middle: 100.0,
        acc_lower: 95.0,
        width: 0.10,
        position: 0.60,
        last_close: 101.0,
        accbands_label: "UPPER".into(),
        note: String::new(),
    };
    upsert_accbands(&conn, "TEST", &snap).unwrap();
    let got = get_accbands(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.accbands_label, "UPPER");
    assert!((got.acc_upper - 105.0).abs() < 1e-6);
    assert_eq!(got.length, 20);
}

#[test]
fn accbands_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_accbands_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.accbands_label.as_str(),
        "BREAKOUT_UP" | "UPPER" | "MID" | "LOWER" | "BREAKOUT_DOWN" | "INSUFFICIENT_DATA"
    ));
    if snap.accbands_label != "INSUFFICIENT_DATA" {
        assert!(snap.acc_upper >= snap.acc_lower);
    }
}

#[test]
fn stochf_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = StochfSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 30,
        length: 14,
        d_period: 3,
        fastk: 75.0,
        fastk_prev: 72.0,
        fastd: 73.0,
        fastd_prev: 70.0,
        last_close: 100.0,
        stochf_label: "BULL".into(),
        note: String::new(),
    };
    upsert_stochf(&conn, "TEST", &snap).unwrap();
    let got = get_stochf(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.stochf_label, "BULL");
    assert!((got.fastk - 75.0).abs() < 1e-6);
    assert_eq!(got.length, 14);
}

#[test]
fn stochf_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_stochf_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.stochf_label.as_str(),
        "OVERBOUGHT" | "BULL" | "NEUTRAL" | "BEAR" | "OVERSOLD" | "INSUFFICIENT_DATA"
    ));
    if snap.stochf_label != "INSUFFICIENT_DATA" {
        assert!(snap.fastk >= 0.0 && snap.fastk <= 100.0);
        assert!(snap.fastd >= 0.0 && snap.fastd <= 100.0);
    }
}

// ── Round 64 tests ──
#[test]
fn linearreg_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = LinearregSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 30,
        length: 14,
        fitted: 100.5,
        fitted_prev: 99.8,
        residual: -0.5,
        residual_pct: -0.5,
        last_close: 100.0,
        linearreg_label: "NEAR_TREND".into(),
        note: String::new(),
    };
    upsert_linearreg(&conn, "TEST", &snap).unwrap();
    let got = get_linearreg(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.linearreg_label, "NEAR_TREND");
    assert!((got.fitted - 100.5).abs() < 1e-6);
}

#[test]
fn linearreg_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_linearreg_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.linearreg_label.as_str(),
        "ABOVE_TREND" | "NEAR_TREND" | "BELOW_TREND" | "INSUFFICIENT_DATA"
    ));
    if snap.linearreg_label != "INSUFFICIENT_DATA" {
        assert!(snap.fitted > 0.0);
    }
}

#[test]
fn linearreg_angle_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = LinearregAngleSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 30,
        length: 14,
        slope: 0.2,
        angle_deg: 11.3,
        angle_deg_prev: 10.5,
        last_close: 100.0,
        angle_label: "UP".into(),
        note: String::new(),
    };
    upsert_linearreg_angle(&conn, "TEST", &snap).unwrap();
    let got = get_linearreg_angle(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.angle_label, "UP");
    assert!((got.angle_deg - 11.3).abs() < 1e-6);
}

#[test]
fn linearreg_angle_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_linearreg_angle_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.angle_label.as_str(),
        "STRONG_UP" | "UP" | "FLAT" | "DOWN" | "STRONG_DOWN" | "INSUFFICIENT_DATA"
    ));
    if snap.angle_label != "INSUFFICIENT_DATA" {
        assert!(snap.angle_deg >= -90.0 && snap.angle_deg <= 90.0);
    }
}

#[test]
fn ht_dcphase_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = HtDcphaseSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        phase_deg: 90.0,
        phase_deg_prev: 80.0,
        phase_delta: 10.0,
        period: 20.0,
        last_close: 100.0,
        phase_label: "RISING".into(),
        note: String::new(),
    };
    upsert_ht_dcphase(&conn, "TEST", &snap).unwrap();
    let got = get_ht_dcphase(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.phase_label, "RISING");
    assert!((got.phase_deg - 90.0).abs() < 1e-6);
}

#[test]
fn ht_dcphase_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_ht_dcphase_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.phase_label.as_str(),
        "CYCLE_BOTTOM" | "RISING" | "CYCLE_TOP" | "FALLING" | "INSUFFICIENT_DATA"
    ));
    if snap.phase_label != "INSUFFICIENT_DATA" {
        assert!(snap.phase_deg >= 0.0 && snap.phase_deg < 360.0);
        assert!(snap.period >= 6.0 && snap.period <= 50.0);
    }
}

#[test]
fn ht_sine_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = HtSineSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        sine: 0.5,
        sine_prev: 0.3,
        leadsine: 0.6,
        leadsine_prev: 0.4,
        crossover: 0,
        period: 20.0,
        last_close: 100.0,
        sine_label: "BULL".into(),
        note: String::new(),
    };
    upsert_ht_sine(&conn, "TEST", &snap).unwrap();
    let got = get_ht_sine(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.sine_label, "BULL");
    assert!((got.sine - 0.5).abs() < 1e-6);
}

#[test]
fn ht_sine_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_ht_sine_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.sine_label.as_str(),
        "CYCLE_TURN_UP" | "BULL" | "NEUTRAL" | "BEAR" | "CYCLE_TURN_DOWN" | "INSUFFICIENT_DATA"
    ));
    if snap.sine_label != "INSUFFICIENT_DATA" {
        assert!(snap.sine >= -1.0 && snap.sine <= 1.0);
        assert!(snap.leadsine >= -1.0 && snap.leadsine <= 1.0);
    }
}

#[test]
fn ht_phasor_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = HtPhasorSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        i_comp: 2.0,
        q_comp: 3.0,
        i_prev: 1.8,
        q_prev: 2.5,
        magnitude: 3.606,
        phase_deg: 56.3,
        last_close: 100.0,
        phasor_label: "CYCLE".into(),
        note: String::new(),
    };
    upsert_ht_phasor(&conn, "TEST", &snap).unwrap();
    let got = get_ht_phasor(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.phasor_label, "CYCLE");
    assert!((got.i_comp - 2.0).abs() < 1e-6);
}

#[test]
fn ht_phasor_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_ht_phasor_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.phasor_label.as_str(),
        "STRONG_CYCLE" | "CYCLE" | "WEAK_CYCLE" | "INSUFFICIENT_DATA"
    ));
    if snap.phasor_label != "INSUFFICIENT_DATA" {
        assert!(snap.magnitude >= 0.0);
        assert!(snap.phase_deg >= -180.0 && snap.phase_deg <= 180.0);
    }
}

#[test]
fn midprice_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MidpriceSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 30,
        length: 14,
        midprice: 101.5,
        midprice_prev: 101.2,
        hhv: 103.0,
        llv: 100.0,
        last_close: 101.8,
        position: 0.6,
        midprice_label: "ABOVE_MID".into(),
        note: String::new(),
    };
    upsert_midprice(&conn, "TEST", &snap).unwrap();
    let got = get_midprice(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.midprice_label, "ABOVE_MID");
    assert!((got.midprice - 101.5).abs() < 1e-6);
    assert_eq!(got.length, 14);
}

#[test]
fn midprice_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_midprice_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.midprice_label.as_str(),
        "NEAR_HIGH" | "ABOVE_MID" | "AT_MID" | "BELOW_MID" | "NEAR_LOW" | "INSUFFICIENT_DATA"
    ));
    if snap.midprice_label != "INSUFFICIENT_DATA" {
        assert!(snap.hhv >= snap.llv);
        assert!(snap.position >= 0.0 && snap.position <= 1.0);
        assert!((snap.midprice - 0.5 * (snap.hhv + snap.llv)).abs() < 1e-6);
    }
}

#[test]
fn apo_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = ApoSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 50,
        fast_period: 12,
        slow_period: 26,
        apo: 0.75,
        apo_prev: 0.70,
        fast_ema: 100.5,
        slow_ema: 99.75,
        last_close: 101.0,
        apo_label: "BULL".into(),
        note: String::new(),
    };
    upsert_apo(&conn, "TEST", &snap).unwrap();
    let got = get_apo(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.apo_label, "BULL");
    assert!((got.apo - 0.75).abs() < 1e-6);
}

#[test]
fn apo_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_apo_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.apo_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.apo_label != "INSUFFICIENT_DATA" {
        assert!((snap.apo - (snap.fast_ema - snap.slow_ema)).abs() < 1e-6);
    }
}

#[test]
fn mom_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MomSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 20,
        period: 10,
        mom: 2.5,
        mom_prev: 2.2,
        mom_pct: 2.47,
        last_close: 101.25,
        mom_label: "UP".into(),
        note: String::new(),
    };
    upsert_mom(&conn, "TEST", &snap).unwrap();
    let got = get_mom(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.mom_label, "UP");
    assert!((got.mom - 2.5).abs() < 1e-6);
    assert_eq!(got.period, 10);
}

#[test]
fn mom_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_mom_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.mom_label.as_str(),
        "STRONG_UP" | "UP" | "FLAT" | "DOWN" | "STRONG_DOWN" | "INSUFFICIENT_DATA"
    ));
}

#[test]
fn sarext_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = SarextSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 50,
        start_value: 0.0,
        af_init_long: 0.02,
        af_step_long: 0.02,
        af_max_long: 0.20,
        af_init_short: 0.02,
        af_step_short: 0.02,
        af_max_short: 0.20,
        sar_value: 99.5,
        extreme_point: 102.0,
        acceleration_factor: 0.08,
        trend_is_up: true,
        bars_in_trend: 5,
        distance_pct: 1.5,
        last_close: 101.0,
        sarext_label: "UP".into(),
        note: String::new(),
    };
    upsert_sarext(&conn, "TEST", &snap).unwrap();
    let got = get_sarext(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.sarext_label, "UP");
    assert!(got.trend_is_up);
    assert!((got.sar_value - 99.5).abs() < 1e-6);
}

#[test]
fn sarext_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_sarext_snapshot(
        "T",
        "2026-04-18",
        &bars,
        0.0,
        0.02,
        0.02,
        0.20,
        0.03,
        0.03,
        0.30,
    );
    assert!(matches!(
        snap.sarext_label.as_str(),
        "STRONG_UP" | "UP" | "STRONG_DOWN" | "DOWN" | "INSUFFICIENT_DATA"
    ));
    if snap.sarext_label != "INSUFFICIENT_DATA" {
        assert!(snap.bars_in_trend >= 1);
        assert!((snap.af_max_short - 0.30).abs() < 1e-9);
    }
}

#[test]
fn adxr_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = AdxrSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        period: 14,
        adx_now: 28.0,
        adx_prior: 22.0,
        adxr: 25.0,
        adxr_prev: 24.5,
        last_close: 101.0,
        adxr_label: "TREND".into(),
        note: String::new(),
    };
    upsert_adxr(&conn, "TEST", &snap).unwrap();
    let got = get_adxr(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.adxr_label, "TREND");
    assert!((got.adxr - 25.0).abs() < 1e-6);
}

#[test]
fn adxr_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_adxr_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.adxr_label.as_str(),
        "STRONG_TREND" | "TREND" | "WEAK_TREND" | "NO_TREND" | "INSUFFICIENT_DATA"
    ));
    if snap.adxr_label != "INSUFFICIENT_DATA" {
        assert!((snap.adxr - 0.5 * (snap.adx_now + snap.adx_prior)).abs() < 1e-6);
    }
}

// ── Round 66 ──
#[test]
fn avgprice_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = AvgpriceSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 3,
        avgprice: 101.5,
        avgprice_prev: 100.75,
        open: 100.0,
        high: 103.0,
        low: 100.0,
        close: 103.0,
        delta_pct: -1.46,
        avgprice_label: "BELOW_CLOSE".into(),
        note: String::new(),
    };
    upsert_avgprice(&conn, "TEST", &snap).unwrap();
    let got = get_avgprice(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.avgprice_label, "BELOW_CLOSE");
    assert!((got.avgprice - 101.5).abs() < 1e-6);
}

#[test]
fn avgprice_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_avgprice_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.avgprice_label.as_str(),
        "ABOVE_CLOSE" | "NEAR_CLOSE" | "BELOW_CLOSE" | "INSUFFICIENT_DATA"
    ));
    if snap.avgprice_label != "INSUFFICIENT_DATA" {
        let expected = (snap.open + snap.high + snap.low + snap.close) / 4.0;
        assert!((snap.avgprice - expected).abs() < 1e-9);
    }
}

#[test]
fn medprice_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MedpriceSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 2,
        medprice: 101.5,
        medprice_prev: 100.5,
        high: 103.0,
        low: 100.0,
        close: 102.0,
        delta_pct: -0.49,
        medprice_label: "AT_MID".into(),
        note: String::new(),
    };
    upsert_medprice(&conn, "TEST", &snap).unwrap();
    let got = get_medprice(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.medprice_label, "AT_MID");
    assert!((got.medprice - 101.5).abs() < 1e-6);
}

#[test]
fn medprice_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_medprice_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.medprice_label.as_str(),
        "ABOVE_MID" | "AT_MID" | "BELOW_MID" | "INSUFFICIENT_DATA"
    ));
    if snap.medprice_label != "INSUFFICIENT_DATA" {
        assert!((snap.medprice - 0.5 * (snap.high + snap.low)).abs() < 1e-9);
    }
}

#[test]
fn typprice_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = TypPriceSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 5,
        typprice: 101.67,
        typprice_prev: 100.33,
        high: 103.0,
        low: 100.0,
        close: 102.0,
        delta_pct: -0.32,
        typprice_label: "NEAR_CLOSE".into(),
        note: String::new(),
    };
    upsert_typprice(&conn, "TEST", &snap).unwrap();
    let got = get_typprice(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.typprice_label, "NEAR_CLOSE");
    assert!((got.typprice - 101.67).abs() < 1e-6);
}

#[test]
fn typprice_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_typprice_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.typprice_label.as_str(),
        "ABOVE_CLOSE" | "NEAR_CLOSE" | "BELOW_CLOSE" | "INSUFFICIENT_DATA"
    ));
    if snap.typprice_label != "INSUFFICIENT_DATA" {
        let expected = (snap.high + snap.low + snap.close) / 3.0;
        assert!((snap.typprice - expected).abs() < 1e-9);
    }
}

#[test]
fn wclprice_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = WclPriceSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 7,
        wclprice: 101.75,
        wclprice_prev: 100.25,
        high: 103.0,
        low: 100.0,
        close: 102.0,
        delta_pct: -0.25,
        wclprice_label: "NEAR_CLOSE".into(),
        note: String::new(),
    };
    upsert_wclprice(&conn, "TEST", &snap).unwrap();
    let got = get_wclprice(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.wclprice_label, "NEAR_CLOSE");
    assert!((got.wclprice - 101.75).abs() < 1e-6);
}

#[test]
fn wclprice_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_wclprice_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.wclprice_label.as_str(),
        "ABOVE_CLOSE" | "NEAR_CLOSE" | "BELOW_CLOSE" | "INSUFFICIENT_DATA"
    ));
    if snap.wclprice_label != "INSUFFICIENT_DATA" {
        let expected = (snap.high + snap.low + 2.0 * snap.close) / 4.0;
        assert!((snap.wclprice - expected).abs() < 1e-9);
    }
}

#[test]
fn variance_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = VarianceSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 50,
        period: 5,
        mean: 100.5,
        variance: 2.5,
        variance_prev: 2.2,
        stddev: 1.5811,
        cv: 1.57,
        last_close: 101.0,
        variance_label: "NORMAL".into(),
        note: String::new(),
    };
    upsert_variance(&conn, "TEST", &snap).unwrap();
    let got = get_variance(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.variance_label, "NORMAL");
    assert!((got.variance - 2.5).abs() < 1e-6);
}

#[test]
fn variance_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_variance_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.variance_label.as_str(),
        "HIGH_VOL" | "ELEVATED" | "NORMAL" | "LOW_VOL" | "INSUFFICIENT_DATA"
    ));
    if snap.variance_label != "INSUFFICIENT_DATA" {
        assert!(snap.variance >= 0.0);
        assert!((snap.stddev - snap.variance.sqrt()).abs() < 1e-9);
    }
}

// ── Round 67 ──
#[test]
fn plus_di_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = PlusDiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        period: 14,
        plus_di: 28.5,
        plus_di_prev: 27.8,
        minus_di: 18.0,
        atr: 2.1,
        last_close: 101.0,
        plus_di_label: "BULL_DOMINANT".into(),
        note: String::new(),
    };
    upsert_plus_di(&conn, "TEST", &snap).unwrap();
    let got = get_plus_di(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.plus_di_label, "BULL_DOMINANT");
    assert!((got.plus_di - 28.5).abs() < 1e-6);
}

#[test]
fn plus_di_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_plus_di_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.plus_di_label.as_str(),
        "BULL_DOMINANT" | "BULL_LEAN" | "NEUTRAL" | "BEAR_LEAN" | "INSUFFICIENT_DATA"
    ));
    if snap.plus_di_label != "INSUFFICIENT_DATA" {
        assert!(snap.plus_di >= 0.0 && snap.plus_di <= 100.0);
        assert!(snap.atr >= 0.0);
    }
}

#[test]
fn minus_di_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MinusDiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        period: 14,
        minus_di: 32.0,
        minus_di_prev: 30.5,
        plus_di: 18.0,
        atr: 2.4,
        last_close: 99.0,
        minus_di_label: "BEAR_DOMINANT".into(),
        note: String::new(),
    };
    upsert_minus_di(&conn, "TEST", &snap).unwrap();
    let got = get_minus_di(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.minus_di_label, "BEAR_DOMINANT");
    assert!((got.minus_di - 32.0).abs() < 1e-6);
}

#[test]
fn minus_di_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_minus_di_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.minus_di_label.as_str(),
        "BEAR_DOMINANT" | "BEAR_LEAN" | "NEUTRAL" | "BULL_LEAN" | "INSUFFICIENT_DATA"
    ));
    if snap.minus_di_label != "INSUFFICIENT_DATA" {
        assert!(snap.minus_di >= 0.0 && snap.minus_di <= 100.0);
        assert!(snap.atr >= 0.0);
    }
}

#[test]
fn plus_dm_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = PlusDmSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        period: 14,
        plus_dm_raw: 1.2,
        plus_dm_smoothed: 8.5,
        plus_dm_smoothed_prev: 8.2,
        up_move: 1.2,
        down_move: 0.3,
        last_close: 101.0,
        plus_dm_label: "BULL_PRESSURE".into(),
        note: String::new(),
    };
    upsert_plus_dm(&conn, "TEST", &snap).unwrap();
    let got = get_plus_dm(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.plus_dm_label, "BULL_PRESSURE");
    assert!((got.plus_dm_smoothed - 8.5).abs() < 1e-6);
}

#[test]
fn plus_dm_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_plus_dm_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.plus_dm_label.as_str(),
        "BULL_PRESSURE" | "BULL_SOFT" | "NEUTRAL" | "BEAR_PRESSURE" | "INSUFFICIENT_DATA"
    ));
    if snap.plus_dm_label != "INSUFFICIENT_DATA" {
        assert!(snap.plus_dm_raw >= 0.0);
        assert!(snap.plus_dm_smoothed >= 0.0);
    }
}

#[test]
fn minus_dm_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MinusDmSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        period: 14,
        minus_dm_raw: 0.9,
        minus_dm_smoothed: 7.1,
        minus_dm_smoothed_prev: 7.0,
        up_move: 0.2,
        down_move: 0.9,
        last_close: 99.0,
        minus_dm_label: "BEAR_PRESSURE".into(),
        note: String::new(),
    };
    upsert_minus_dm(&conn, "TEST", &snap).unwrap();
    let got = get_minus_dm(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.minus_dm_label, "BEAR_PRESSURE");
    assert!((got.minus_dm_smoothed - 7.1).abs() < 1e-6);
}

#[test]
fn minus_dm_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_minus_dm_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.minus_dm_label.as_str(),
        "BEAR_PRESSURE" | "BEAR_SOFT" | "NEUTRAL" | "BULL_PRESSURE" | "INSUFFICIENT_DATA"
    ));
    if snap.minus_dm_label != "INSUFFICIENT_DATA" {
        assert!(snap.minus_dm_raw >= 0.0);
        assert!(snap.minus_dm_smoothed >= 0.0);
    }
}

#[test]
fn dx_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = DxSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        period: 14,
        dx: 28.0,
        dx_prev: 26.0,
        plus_di: 28.5,
        minus_di: 16.0,
        last_close: 101.0,
        dx_label: "DIR".into(),
        note: String::new(),
    };
    upsert_dx(&conn, "TEST", &snap).unwrap();
    let got = get_dx(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.dx_label, "DIR");
    assert!((got.dx - 28.0).abs() < 1e-6);
}

#[test]
fn dx_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_dx_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.dx_label.as_str(),
        "STRONG_DIR" | "DIR" | "WEAK_DIR" | "NO_DIR" | "INSUFFICIENT_DATA"
    ));
    if snap.dx_label != "INSUFFICIENT_DATA" {
        let s = snap.plus_di + snap.minus_di;
        if s > 0.0 {
            let expected = 100.0 * (snap.plus_di - snap.minus_di).abs() / s;
            assert!((snap.dx - expected).abs() < 1e-6);
        }
        assert!(snap.dx >= 0.0 && snap.dx <= 100.0);
    }
}

// ── Round 68 ──
#[test]
fn roc_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = RocSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        period: 10,
        roc: 1.8,
        roc_prev: 1.2,
        close_now: 101.8,
        close_lag: 100.0,
        last_close: 101.8,
        roc_label: "UP".into(),
        note: String::new(),
    };
    upsert_roc(&conn, "TEST", &snap).unwrap();
    let got = get_roc(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.roc_label, "UP");
    assert!((got.roc - 1.8).abs() < 1e-6);
}

#[test]
fn roc_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_roc_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.roc_label.as_str(),
        "STRONG_UP" | "UP" | "NEUTRAL" | "DOWN" | "STRONG_DOWN" | "INSUFFICIENT_DATA"
    ));
    if snap.roc_label != "INSUFFICIENT_DATA" {
        assert!((snap.roc - (snap.close_now - snap.close_lag)).abs() < 1e-9);
    }
}

#[test]
fn rocp_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = RocpSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        period: 10,
        rocp: 0.018,
        rocp_prev: 0.012,
        rocp_pct: 1.8,
        close_now: 101.8,
        close_lag: 100.0,
        last_close: 101.8,
        rocp_label: "UP".into(),
        note: String::new(),
    };
    upsert_rocp(&conn, "TEST", &snap).unwrap();
    let got = get_rocp(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.rocp_label, "UP");
    assert!((got.rocp_pct - 1.8).abs() < 1e-6);
}

#[test]
fn rocp_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_rocp_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.rocp_label.as_str(),
        "STRONG_UP" | "UP" | "NEUTRAL" | "DOWN" | "STRONG_DOWN" | "INSUFFICIENT_DATA"
    ));
    if snap.rocp_label != "INSUFFICIENT_DATA" && snap.close_lag.abs() > 1e-9 {
        let expected = (snap.close_now - snap.close_lag) / snap.close_lag;
        assert!((snap.rocp - expected).abs() < 1e-9);
        assert!((snap.rocp_pct - expected * 100.0).abs() < 1e-9);
    }
}

#[test]
fn rocr_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = RocrSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        period: 10,
        rocr: 1.018,
        rocr_prev: 1.012,
        close_now: 101.8,
        close_lag: 100.0,
        last_close: 101.8,
        rocr_label: "UP".into(),
        note: String::new(),
    };
    upsert_rocr(&conn, "TEST", &snap).unwrap();
    let got = get_rocr(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.rocr_label, "UP");
    assert!((got.rocr - 1.018).abs() < 1e-6);
}

#[test]
fn rocr_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_rocr_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.rocr_label.as_str(),
        "STRONG_UP" | "UP" | "NEUTRAL" | "DOWN" | "STRONG_DOWN" | "INSUFFICIENT_DATA"
    ));
    if snap.rocr_label != "INSUFFICIENT_DATA" && snap.close_lag.abs() > 1e-9 {
        let expected = snap.close_now / snap.close_lag;
        assert!((snap.rocr - expected).abs() < 1e-9);
    }
}

#[test]
fn rocr100_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = Rocr100Snapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        period: 10,
        rocr100: 101.8,
        rocr100_prev: 101.2,
        close_now: 101.8,
        close_lag: 100.0,
        last_close: 101.8,
        rocr100_label: "UP".into(),
        note: String::new(),
    };
    upsert_rocr100(&conn, "TEST", &snap).unwrap();
    let got = get_rocr100(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.rocr100_label, "UP");
    assert!((got.rocr100 - 101.8).abs() < 1e-6);
}

#[test]
fn rocr100_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_rocr100_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.rocr100_label.as_str(),
        "STRONG_UP" | "UP" | "NEUTRAL" | "DOWN" | "STRONG_DOWN" | "INSUFFICIENT_DATA"
    ));
    if snap.rocr100_label != "INSUFFICIENT_DATA" && snap.close_lag.abs() > 1e-9 {
        let expected = 100.0 * snap.close_now / snap.close_lag;
        assert!((snap.rocr100 - expected).abs() < 1e-6);
    }
}

#[test]
fn correl_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CorrelSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        period: 30,
        correl: 0.82,
        correl_prev: 0.79,
        mean_x: 101.0,
        mean_y: 100.8,
        stddev_x: 1.2,
        stddev_y: 1.1,
        last_close: 101.5,
        correl_label: "STRONG_MOMO".into(),
        note: String::new(),
    };
    upsert_correl(&conn, "TEST", &snap).unwrap();
    let got = get_correl(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.correl_label, "STRONG_MOMO");
    assert!((got.correl - 0.82).abs() < 1e-6);
}

#[test]
fn correl_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_correl_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.correl_label.as_str(),
        "STRONG_MOMO"
            | "MOMO"
            | "RANDOM_WALK"
            | "MEAN_REVERT"
            | "STRONG_MEAN_REVERT"
            | "INSUFFICIENT_DATA"
    ));
    if snap.correl_label != "INSUFFICIENT_DATA" {
        assert!(snap.correl >= -1.0 - 1e-9 && snap.correl <= 1.0 + 1e-9);
        assert!(snap.stddev_x >= 0.0 && snap.stddev_y >= 0.0);
    }
}

// ── Round 69 ──
#[test]
fn min_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MinSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        period: 30,
        min_val: 95.0,
        min_prev: 95.2,
        max_ref: 110.0,
        last_close: 102.0,
        position_pct: 46.67,
        min_label: "MID".into(),
        note: String::new(),
    };
    upsert_min(&conn, "TEST", &snap).unwrap();
    let got = get_min(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.min_label, "MID");
    assert!((got.min_val - 95.0).abs() < 1e-6);
}

#[test]
fn min_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_min_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.min_label.as_str(),
        "NEAR_LOW" | "MID" | "NEAR_HIGH" | "INSUFFICIENT_DATA"
    ));
    if snap.min_label != "INSUFFICIENT_DATA" {
        assert!(snap.min_val <= snap.last_close + 1e-9);
        assert!(snap.position_pct >= 0.0 - 1e-6 && snap.position_pct <= 100.0 + 1e-6);
    }
}

#[test]
fn max_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MaxSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        period: 30,
        max_val: 110.0,
        max_prev: 109.5,
        min_ref: 95.0,
        last_close: 108.0,
        position_pct: 86.67,
        max_label: "NEAR_HIGH".into(),
        note: String::new(),
    };
    upsert_max(&conn, "TEST", &snap).unwrap();
    let got = get_max(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.max_label, "NEAR_HIGH");
    assert!((got.max_val - 110.0).abs() < 1e-6);
}

#[test]
fn max_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_max_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.max_label.as_str(),
        "NEAR_LOW" | "MID" | "NEAR_HIGH" | "INSUFFICIENT_DATA"
    ));
    if snap.max_label != "INSUFFICIENT_DATA" {
        assert!(snap.max_val >= snap.last_close - 1e-9);
        assert!(snap.max_val >= snap.min_ref - 1e-9);
    }
}

#[test]
fn minmax_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MinMaxSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        period: 30,
        min_val: 95.0,
        max_val: 110.0,
        range_width: 15.0,
        range_pct: 13.88,
        last_close: 108.0,
        position_pct: 86.67,
        minmax_label: "RANGE_NORMAL".into(),
        note: String::new(),
    };
    upsert_minmax(&conn, "TEST", &snap).unwrap();
    let got = get_minmax(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.minmax_label, "RANGE_NORMAL");
    assert!((got.range_width - 15.0).abs() < 1e-6);
}

#[test]
fn minmax_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_minmax_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.minmax_label.as_str(),
        "RANGE_WIDE" | "RANGE_NORMAL" | "RANGE_TIGHT" | "INSUFFICIENT_DATA"
    ));
    if snap.minmax_label != "INSUFFICIENT_DATA" {
        let expected = snap.max_val - snap.min_val;
        assert!((snap.range_width - expected).abs() < 1e-9);
        assert!(snap.max_val >= snap.min_val);
    }
}

#[test]
fn minindex_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MinIndexSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        period: 30,
        min_val: 95.0,
        min_index_bars_ago: 3,
        min_index_bars_ago_prev: 4,
        last_close: 102.0,
        min_index_label: "FRESH_LOW".into(),
        note: String::new(),
    };
    upsert_minindex(&conn, "TEST", &snap).unwrap();
    let got = get_minindex(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.min_index_label, "FRESH_LOW");
    assert_eq!(got.min_index_bars_ago, 3);
}

#[test]
fn minindex_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_minindex_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.min_index_label.as_str(),
        "FRESH_LOW" | "RECENT_LOW" | "OLD_LOW" | "STALE_LOW" | "INSUFFICIENT_DATA"
    ));
    if snap.min_index_label != "INSUFFICIENT_DATA" {
        assert!(snap.min_index_bars_ago < snap.period);
    }
}

#[test]
fn maxindex_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MaxIndexSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        period: 30,
        max_val: 110.0,
        max_index_bars_ago: 2,
        max_index_bars_ago_prev: 3,
        last_close: 108.0,
        max_index_label: "FRESH_HIGH".into(),
        note: String::new(),
    };
    upsert_maxindex(&conn, "TEST", &snap).unwrap();
    let got = get_maxindex(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.max_index_label, "FRESH_HIGH");
    assert_eq!(got.max_index_bars_ago, 2);
}

#[test]
fn maxindex_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_maxindex_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.max_index_label.as_str(),
        "FRESH_HIGH" | "RECENT_HIGH" | "OLD_HIGH" | "STALE_HIGH" | "INSUFFICIENT_DATA"
    ));
    if snap.max_index_label != "INSUFFICIENT_DATA" {
        assert!(snap.max_index_bars_ago < snap.period);
    }
}

// ── Round 70 tests — BBANDS / AD / ADOSC / SUM / LINEARREG_INTERCEPT ──

#[test]
fn bbands_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = BbandsSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        period: 20,
        num_std: 2.0,
        upper: 110.0,
        middle: 100.0,
        lower: 90.0,
        upper_prev: 109.0,
        middle_prev: 99.0,
        lower_prev: 89.0,
        last_close: 105.0,
        pct_b: 75.0,
        bandwidth: 20.0,
        bbands_label: "UPPER_HALF".into(),
        note: String::new(),
    };
    upsert_bbands(&conn, "TEST", &snap).unwrap();
    let got = get_bbands(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.bbands_label, "UPPER_HALF");
    assert_eq!(got.period, 20);
}

#[test]
fn bbands_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_bbands_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.bbands_label.as_str(),
        "ABOVE_UPPER" | "UPPER_HALF" | "LOWER_HALF" | "BELOW_LOWER" | "INSUFFICIENT_DATA"
    ));
    if snap.bbands_label != "INSUFFICIENT_DATA" {
        assert!(snap.upper > snap.middle);
        assert!(snap.middle > snap.lower);
        // middle identity: SMA of last 20 closes
        let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
        sorted.sort_by(|a, b| a.date.cmp(&b.date));
        let n = sorted.len();
        let expected_mid = (0..20).map(|k| sorted[n - 20 + k].close).sum::<f64>() / 20.0;
        assert!((snap.middle - expected_mid).abs() < 1e-6);
    }
}

#[test]
fn ad_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = AdSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        ad: 12345.0,
        ad_prev: 12300.0,
        ad_delta: 45.0,
        ad_slope: 5.2,
        last_close: 108.0,
        ad_label: "ACCUM".into(),
        note: String::new(),
    };
    upsert_ad(&conn, "TEST", &snap).unwrap();
    let got = get_ad(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.ad_label, "ACCUM");
    assert_eq!(got.ad_delta, 45.0);
}

#[test]
fn ad_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_ad_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.ad_label.as_str(),
        "STRONG_ACCUM" | "ACCUM" | "FLAT" | "DIST" | "STRONG_DIST" | "INSUFFICIENT_DATA"
    ));
    if snap.ad_label != "INSUFFICIENT_DATA" {
        assert!((snap.ad - snap.ad_prev - snap.ad_delta).abs() < 1e-6);
    }
}

#[test]
fn adosc_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = AdoscSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        fast_period: 3,
        slow_period: 10,
        adosc: 123.4,
        adosc_prev: 100.0,
        last_close: 108.0,
        ad_ref: 50000.0,
        adosc_label: "BULL".into(),
        note: String::new(),
    };
    upsert_adosc(&conn, "TEST", &snap).unwrap();
    let got = get_adosc(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.adosc_label, "BULL");
    assert_eq!(got.fast_period, 3);
}

#[test]
fn adosc_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_adosc_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.adosc_label.as_str(),
        "STRONG_BULL" | "BULL" | "FLAT" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
}

#[test]
fn sum_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = SumSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        period: 30,
        sum: 3000.0,
        sum_prev: 2970.0,
        sum_delta: 30.0,
        sum_pct_change: 1.01,
        last_close: 101.0,
        sum_label: "STRONG_UP".into(),
        note: String::new(),
    };
    upsert_sum(&conn, "TEST", &snap).unwrap();
    let got = get_sum(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.sum_label, "STRONG_UP");
    assert_eq!(got.period, 30);
}

#[test]
fn sum_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_sum_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.sum_label.as_str(),
        "STRONG_UP" | "UP" | "FLAT" | "DOWN" | "STRONG_DOWN" | "INSUFFICIENT_DATA"
    ));
    if snap.sum_label != "INSUFFICIENT_DATA" {
        // sum identity: sum of last 30 closes
        let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
        sorted.sort_by(|a, b| a.date.cmp(&b.date));
        let n = sorted.len();
        let expected = (0..30).map(|k| sorted[n - 30 + k].close).sum::<f64>();
        assert!((snap.sum - expected).abs() < 1e-6);
    }
}

#[test]
fn linreg_intercept_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = LinearRegInterceptSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        period: 14,
        intercept: 95.0,
        intercept_prev: 94.5,
        slope: 0.5,
        last_close: 100.0,
        drift: 5.0,
        drift_pct: 5.26,
        linreg_intercept_label: "STRONG_ADVANCE".into(),
        note: String::new(),
    };
    upsert_linreg_intercept(&conn, "TEST", &snap).unwrap();
    let got = get_linreg_intercept(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.linreg_intercept_label, "STRONG_ADVANCE");
    assert_eq!(got.period, 14);
}

#[test]
fn linreg_intercept_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_linearreg_intercept_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.linreg_intercept_label.as_str(),
        "STRONG_ADVANCE" | "ADVANCE" | "FLAT" | "DECLINE" | "STRONG_DECLINE" | "INSUFFICIENT_DATA"
    ));
    if snap.linreg_intercept_label != "INSUFFICIENT_DATA" {
        // drift identity: last_close - intercept
        assert!((snap.drift - (snap.last_close - snap.intercept)).abs() < 1e-6);
    }
}

// ── Round 71 tests ──────────────────────────────────────────────

#[test]
fn aroonosc_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = AroonoscSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        period: 14,
        aroonosc: 42.86,
        aroonosc_prev: 35.71,
        aroon_up: 71.43,
        aroon_down: 28.57,
        last_close: 100.0,
        aroonosc_label: "BULL".into(),
        note: String::new(),
    };
    upsert_aroonosc(&conn, "TEST", &snap).unwrap();
    let got = get_aroonosc(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.aroonosc_label, "BULL");
    assert_eq!(got.period, 14);
}

#[test]
fn aroonosc_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_aroonosc_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.aroonosc_label.as_str(),
        "STRONG_BULL" | "BULL" | "FLAT" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.aroonosc_label != "INSUFFICIENT_DATA" {
        // Identity: aroonosc = aroon_up - aroon_down
        assert!((snap.aroonosc - (snap.aroon_up - snap.aroon_down)).abs() < 1e-6);
        assert!(snap.aroonosc >= -100.0 && snap.aroonosc <= 100.0);
        assert_eq!(snap.period, 14);
    }
}

#[test]
fn minmaxindex_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MinMaxIndexSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        period: 30,
        min_index_bars_ago: 5,
        max_index_bars_ago: 2,
        age_diff: 3,
        extrema_order: "LOW_FIRST".into(),
        last_close: 100.0,
        minmaxindex_label: "FRESH_HIGH".into(),
        note: String::new(),
    };
    upsert_minmaxindex(&conn, "TEST", &snap).unwrap();
    let got = get_minmaxindex(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.minmaxindex_label, "FRESH_HIGH");
    assert_eq!(got.extrema_order, "LOW_FIRST");
}

#[test]
fn minmaxindex_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_minmaxindex_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.minmaxindex_label.as_str(),
        "FRESH_HIGH" | "FRESH_LOW" | "MID" | "OLD_EXTREMA" | "INSUFFICIENT_DATA"
    ));
    if snap.minmaxindex_label != "INSUFFICIENT_DATA" {
        assert!(snap.min_index_bars_ago < snap.period);
        assert!(snap.max_index_bars_ago < snap.period);
        // Identity: age_diff == min_index - max_index (signed)
        assert_eq!(
            snap.age_diff,
            snap.min_index_bars_ago as i64 - snap.max_index_bars_ago as i64
        );
        assert!(matches!(
            snap.extrema_order.as_str(),
            "HIGH_FIRST" | "LOW_FIRST" | "SAME_BAR"
        ));
    }
}

#[test]
fn macdext_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MacdextSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        fast_period: 12,
        slow_period: 26,
        signal_period: 9,
        ma_type: "SMA".into(),
        macd: 1.2,
        macd_prev: 1.0,
        signal: 0.9,
        signal_prev: 0.8,
        hist: 0.3,
        hist_prev: 0.2,
        last_close: 100.0,
        macdext_label: "STRONG_BULL".into(),
        note: String::new(),
    };
    upsert_macdext(&conn, "TEST", &snap).unwrap();
    let got = get_macdext(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.macdext_label, "STRONG_BULL");
    assert_eq!(got.ma_type, "SMA");
}

#[test]
fn macdext_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_macdext_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.macdext_label.as_str(),
        "STRONG_BULL" | "BULL" | "FLAT" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.macdext_label != "INSUFFICIENT_DATA" {
        assert_eq!(snap.ma_type, "SMA");
        assert_eq!(snap.fast_period, 12);
        assert_eq!(snap.slow_period, 26);
        // Identity: hist == macd - signal
        assert!((snap.hist - (snap.macd - snap.signal)).abs() < 1e-6);
    }
}

#[test]
fn macdfix_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MacdfixSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        fast_period: 12,
        slow_period: 26,
        signal_period: 9,
        macd: 1.1,
        macd_prev: 0.9,
        signal: 0.8,
        signal_prev: 0.7,
        hist: 0.3,
        hist_prev: 0.2,
        last_close: 100.0,
        macdfix_label: "BULL".into(),
        note: String::new(),
    };
    upsert_macdfix(&conn, "TEST", &snap).unwrap();
    let got = get_macdfix(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.macdfix_label, "BULL");
    assert_eq!(got.fast_period, 12);
}

#[test]
fn macdfix_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_macdfix_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.macdfix_label.as_str(),
        "STRONG_BULL" | "BULL" | "FLAT" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.macdfix_label != "INSUFFICIENT_DATA" {
        // Identity: hardcoded 12/26 periods
        assert_eq!(snap.fast_period, 12);
        assert_eq!(snap.slow_period, 26);
        // Identity: hist == macd - signal
        assert!((snap.hist - (snap.macd - snap.signal)).abs() < 1e-6);
    }
}

#[test]
fn mavp_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MavpSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-18".into(),
        bars_used: 60,
        min_period: 5,
        max_period: 30,
        last_bar_period: 30,
        mavp: 100.0,
        mavp_prev: 99.5,
        mavp_delta: 0.5,
        last_close: 101.0,
        mavp_label: "UP".into(),
        note: String::new(),
    };
    upsert_mavp(&conn, "TEST", &snap).unwrap();
    let got = get_mavp(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.mavp_label, "UP");
    assert_eq!(got.last_bar_period, 30);
}

#[test]
fn mavp_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_mavp_snapshot("T", "2026-04-18", &bars);
    assert!(matches!(
        snap.mavp_label.as_str(),
        "STRONG_UP" | "UP" | "FLAT" | "DOWN" | "STRONG_DOWN" | "INSUFFICIENT_DATA"
    ));
    if snap.mavp_label != "INSUFFICIENT_DATA" {
        assert_eq!(snap.last_bar_period, 30);
        // Identity: delta = mavp - mavp_prev
        assert!((snap.mavp_delta - (snap.mavp - snap.mavp_prev)).abs() < 1e-9);
        assert!(snap.mavp > 0.0);
    }
}

