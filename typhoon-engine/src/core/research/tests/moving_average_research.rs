// ── DEMA/TEMA/LINREG/PIVOTS/HEIKIN ──────────────────────────

#[test]
fn dema_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = DemaSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 60,
        length: 20,
        dema_value: 150.0,
        dema_prev: 149.5,
        deviation_pct: 0.3,
        last_close: 150.45,
        dema_label: "BULL".into(),
        note: String::new(),
    };
    upsert_dema(&conn, "TEST", &snap).unwrap();
    let got = get_dema(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.dema_label, "BULL");
    assert!((got.dema_value - 150.0).abs() < 1e-9);
}

#[test]
fn dema_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_dema_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.dema_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.dema_label != "INSUFFICIENT_DATA" {
        assert!(snap.dema_value.is_finite());
        assert!(snap.dema_prev.is_finite());
        assert_eq!(snap.length, 20);
    }
}

#[test]
fn tema_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = TemaSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 80,
        length: 20,
        tema_value: 150.0,
        tema_prev: 149.4,
        deviation_pct: 0.4,
        last_close: 150.6,
        tema_label: "BULL".into(),
        note: String::new(),
    };
    upsert_tema(&conn, "TEST", &snap).unwrap();
    let got = get_tema(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.tema_label, "BULL");
    assert!((got.tema_value - 150.0).abs() < 1e-9);
}

#[test]
fn tema_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_tema_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.tema_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.tema_label != "INSUFFICIENT_DATA" {
        assert!(snap.tema_value.is_finite());
        assert!(snap.tema_prev.is_finite());
        assert_eq!(snap.length, 20);
    }
}

#[test]
fn linreg_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = LinregSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 40,
        length: 20,
        slope: 0.25,
        intercept: 100.0,
        r_squared: 0.82,
        sigma: 1.1,
        last_close: 104.75,
        fit_value: 104.75,
        channel_upper: 106.95,
        channel_lower: 102.55,
        linreg_label: "UP_TREND".into(),
        note: String::new(),
    };
    upsert_linreg(&conn, "TEST", &snap).unwrap();
    let got = get_linreg(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.linreg_label, "UP_TREND");
    assert!((got.slope - 0.25).abs() < 1e-9);
}

#[test]
fn linreg_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_linreg_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.linreg_label.as_str(),
        "STRONG_UP_TREND"
            | "UP_TREND"
            | "RANGE"
            | "DOWN_TREND"
            | "STRONG_DOWN_TREND"
            | "INSUFFICIENT_DATA"
    ));
    if snap.linreg_label != "INSUFFICIENT_DATA" {
        assert!(snap.slope.is_finite());
        assert!(snap.r_squared >= 0.0 && snap.r_squared <= 1.0 + 1e-9);
        assert!(snap.sigma >= 0.0 && snap.sigma.is_finite());
        assert_eq!(snap.length, 20);
        assert!(snap.channel_upper >= snap.fit_value);
        assert!(snap.channel_lower <= snap.fit_value);
    }
}

#[test]
fn pivots_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = PivotsSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 5,
        pp: 100.0,
        r1: 102.0,
        r2: 104.0,
        s1: 98.0,
        s2: 96.0,
        last_close: 101.0,
        prior_high: 101.0,
        prior_low: 99.0,
        prior_close: 100.0,
        pivots_label: "BETWEEN_PP_R1".into(),
        note: String::new(),
    };
    upsert_pivots(&conn, "TEST", &snap).unwrap();
    let got = get_pivots(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.pivots_label, "BETWEEN_PP_R1");
    assert!((got.pp - 100.0).abs() < 1e-9);
}

#[test]
fn pivots_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_pivots_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.pivots_label.as_str(),
        "ABOVE_R2"
            | "BETWEEN_R1_R2"
            | "BETWEEN_PP_R1"
            | "AT_PP"
            | "BETWEEN_S1_PP"
            | "BETWEEN_S2_S1"
            | "BELOW_S2"
            | "INSUFFICIENT_DATA"
    ));
    if snap.pivots_label != "INSUFFICIENT_DATA" {
        assert!(snap.pp.is_finite());
        assert!(snap.r1 >= snap.pp && snap.r2 >= snap.r1);
        assert!(snap.s1 <= snap.pp && snap.s2 <= snap.s1);
    }
}

#[test]
fn heikin_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = HeikinSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 60,
        ha_open: 100.0,
        ha_high: 102.0,
        ha_low: 99.5,
        ha_close: 101.5,
        body_abs: 1.5,
        upper_wick: 0.5,
        lower_wick: 0.5,
        consecutive_same_color: 3,
        last_close: 101.7,
        heikin_label: "STRONG_BULL_RUN".into(),
        note: String::new(),
    };
    upsert_heikin(&conn, "TEST", &snap).unwrap();
    let got = get_heikin(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.heikin_label, "STRONG_BULL_RUN");
    assert_eq!(got.consecutive_same_color, 3);
}

#[test]
fn heikin_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_heikin_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.heikin_label.as_str(),
        "STRONG_BULL_RUN" | "BULL" | "DOJI" | "BEAR" | "STRONG_BEAR_RUN" | "INSUFFICIENT_DATA"
    ));
    if snap.heikin_label != "INSUFFICIENT_DATA" {
        assert!(snap.ha_high >= snap.ha_close && snap.ha_high >= snap.ha_open);
        assert!(snap.ha_low <= snap.ha_close && snap.ha_low <= snap.ha_open);
        assert!(snap.body_abs >= 0.0);
        assert!(snap.upper_wick >= 0.0 && snap.lower_wick >= 0.0);
        assert!(snap.consecutive_same_color >= 1);
    }
}

// ── ALMA / ZLEMA / ELDERRAY / TSF / RVI ──────────────

#[test]
fn alma_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = AlmaSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        length: 20,
        offset: 0.85,
        sigma: 6.0,
        alma_value: 100.5,
        alma_prev: 99.8,
        deviation_pct: 1.2,
        last_close: 101.7,
        alma_label: "BULL".into(),
        note: String::new(),
    };
    upsert_alma(&conn, "TEST", &snap).unwrap();
    let got = get_alma(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.alma_label, "BULL");
    assert!((got.alma_value - 100.5).abs() < 1e-9);
}

#[test]
fn alma_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_alma_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.alma_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.alma_label != "INSUFFICIENT_DATA" {
        assert!(snap.alma_value.is_finite() && snap.alma_value > 0.0);
        assert!(snap.deviation_pct.is_finite());
    }
}

#[test]
fn zlema_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = ZlemaSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 80,
        length: 20,
        lag_shift: 9,
        zlema_value: 100.2,
        zlema_prev: 100.0,
        deviation_pct: 0.5,
        last_close: 100.7,
        zlema_label: "BULL".into(),
        note: String::new(),
    };
    upsert_zlema(&conn, "TEST", &snap).unwrap();
    let got = get_zlema(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.zlema_label, "BULL");
    assert_eq!(got.lag_shift, 9);
}

#[test]
fn zlema_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_zlema_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.zlema_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.zlema_label != "INSUFFICIENT_DATA" {
        assert!(snap.zlema_value.is_finite() && snap.zlema_value > 0.0);
        assert_eq!(snap.lag_shift, 9);
    }
}

#[test]
fn elderray_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = ElderRaySnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 60,
        ema_length: 13,
        ema13: 100.0,
        ema13_prev: 99.5,
        bull_power: 2.5,
        bull_power_prev: 2.0,
        bear_power: -0.5,
        bear_power_prev: -0.8,
        last_close: 101.5,
        elder_label: "BULL".into(),
        note: String::new(),
    };
    upsert_elderray(&conn, "TEST", &snap).unwrap();
    let got = get_elderray(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.elder_label, "BULL");
    assert!((got.bull_power - 2.5).abs() < 1e-9);
}

#[test]
fn elderray_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_elderray_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.elder_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.elder_label != "INSUFFICIENT_DATA" {
        assert!(snap.ema13.is_finite() && snap.ema13 > 0.0);
        // Bull on the highs vs EMA; Bear on the lows vs EMA; bull ≥ bear by definition.
        assert!(snap.bull_power >= snap.bear_power);
    }
}

#[test]
fn tsf_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = TsfSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 80,
        length: 20,
        slope: 0.5,
        intercept: 95.0,
        forecast_value: 105.0,
        last_close: 104.0,
        forecast_deviation_pct: 0.96,
        r_squared: 0.85,
        tsf_label: "LEADING_UP".into(),
        note: String::new(),
    };
    upsert_tsf(&conn, "TEST", &snap).unwrap();
    let got = get_tsf(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.tsf_label, "LEADING_UP");
    assert!((got.slope - 0.5).abs() < 1e-9);
}

#[test]
fn tsf_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_tsf_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.tsf_label.as_str(),
        "LEADING_UP"
            | "LAGGING_UP"
            | "LEADING_DOWN"
            | "LAGGING_DOWN"
            | "FLAT"
            | "INSUFFICIENT_DATA"
    ));
    if snap.tsf_label != "INSUFFICIENT_DATA" {
        assert!(snap.forecast_value.is_finite());
        assert!(snap.r_squared >= 0.0 && snap.r_squared <= 1.0 + 1e-6);
    }
}

#[test]
fn rvi_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = RviSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 60,
        length: 10,
        rvi_value: 0.3,
        rvi_prev: 0.2,
        signal_value: 0.25,
        signal_prev: 0.27,
        last_close: 101.5,
        rvi_label: "BULL_CROSS".into(),
        note: String::new(),
    };
    upsert_rvi(&conn, "TEST", &snap).unwrap();
    let got = get_rvi(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.rvi_label, "BULL_CROSS");
    assert_eq!(got.length, 10);
}

#[test]
fn rvi_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_rvi_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.rvi_label.as_str(),
        "BULL_CROSS" | "BEAR_CROSS" | "BULL" | "BEAR" | "NEUTRAL" | "INSUFFICIENT_DATA"
    ));
    if snap.rvi_label != "INSUFFICIENT_DATA" {
        assert!(snap.rvi_value.is_finite());
        assert!(snap.signal_value.is_finite());
    }
}

// ── Research section ──

#[test]
fn trima_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = TrimaSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 80,
        length: 20,
        trima_value: 100.5,
        trima_prev: 100.3,
        deviation_pct: 0.5,
        last_close: 101.0,
        trima_label: "BULL".into(),
        note: String::new(),
    };
    upsert_trima(&conn, "TEST", &snap).unwrap();
    let got = get_trima(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.trima_label, "BULL");
    assert_eq!(got.length, 20);
}

#[test]
fn trima_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_trima_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.trima_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.trima_label != "INSUFFICIENT_DATA" {
        assert!(snap.trima_value.is_finite() && snap.trima_value > 0.0);
    }
}

#[test]
fn t3_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = T3Snapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 80,
        length: 20,
        v_factor: 0.7,
        t3_value: 100.2,
        t3_prev: 100.0,
        deviation_pct: 0.3,
        last_close: 100.5,
        t3_label: "BULL".into(),
        note: String::new(),
    };
    upsert_t3(&conn, "TEST", &snap).unwrap();
    let got = get_t3(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.t3_label, "BULL");
    assert!((got.v_factor - 0.7).abs() < 1e-9);
}

#[test]
fn t3_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_t3_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.t3_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.t3_label != "INSUFFICIENT_DATA" {
        assert!(snap.t3_value.is_finite() && snap.t3_value > 0.0);
    }
}

#[test]
fn vidya_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = VidyaSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 80,
        length: 20,
        cmo_length: 9,
        vidya_value: 99.8,
        vidya_prev: 99.6,
        current_alpha: 0.05,
        cmo_magnitude: 52.0,
        deviation_pct: 0.2,
        last_close: 100.0,
        vidya_label: "BULL".into(),
        note: String::new(),
    };
    upsert_vidya(&conn, "TEST", &snap).unwrap();
    let got = get_vidya(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.vidya_label, "BULL");
    assert_eq!(got.cmo_length, 9);
}

#[test]
fn vidya_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_vidya_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.vidya_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.vidya_label != "INSUFFICIENT_DATA" {
        assert!(snap.vidya_value.is_finite() && snap.vidya_value > 0.0);
        assert!(snap.current_alpha >= 0.0 && snap.current_alpha <= 1.0);
        assert!(snap.cmo_magnitude >= 0.0 && snap.cmo_magnitude <= 100.0 + 1e-6);
    }
}

#[test]
fn smi_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = SmiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 60,
        length: 10,
        smooth_length: 3,
        signal_length: 3,
        smi_value: 25.0,
        smi_prev: 20.0,
        signal_value: 22.0,
        signal_prev: 23.0,
        last_close: 102.0,
        smi_label: "BULL_CROSS".into(),
        note: String::new(),
    };
    upsert_smi(&conn, "TEST", &snap).unwrap();
    let got = get_smi(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.smi_label, "BULL_CROSS");
    assert_eq!(got.length, 10);
}

#[test]
fn smi_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_smi_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.smi_label.as_str(),
        "OVERBOUGHT"
            | "OVERSOLD"
            | "BULL_CROSS"
            | "BEAR_CROSS"
            | "BULL"
            | "BEAR"
            | "NEUTRAL"
            | "INSUFFICIENT_DATA"
    ));
    if snap.smi_label != "INSUFFICIENT_DATA" {
        assert!(snap.smi_value.is_finite());
        assert!(snap.smi_value >= -100.0 - 1.0 && snap.smi_value <= 100.0 + 1.0);
    }
}

#[test]
fn pvt_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = PvtSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 80,
        pvt_value: 12345.6,
        pvt_prev: 12340.0,
        pvt_ema: 12300.0,
        pvt_slope: 500.0,
        last_close: 100.0,
        pvt_label: "BULL".into(),
        note: String::new(),
    };
    upsert_pvt(&conn, "TEST", &snap).unwrap();
    let got = get_pvt(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.pvt_label, "BULL");
    assert!((got.pvt_slope - 500.0).abs() < 1e-6);
}

#[test]
fn pvt_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_pvt_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.pvt_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.pvt_label != "INSUFFICIENT_DATA" {
        assert!(snap.pvt_value.is_finite());
        assert!(snap.pvt_ema.is_finite());
    }
}

#[test]
fn ac_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = AcSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 60,
        ac_value: 0.42,
        ac_prev: 0.20,
        ao_value: 1.15,
        ao_sma5: 0.73,
        last_close: 101.2,
        ac_label: "STRONG_BULL".into(),
        note: String::new(),
    };
    upsert_ac(&conn, "TEST", &snap).unwrap();
    let got = get_ac(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.ac_label, "STRONG_BULL");
    assert!((got.ac_value - 0.42).abs() < 1e-6);
}

#[test]
fn ac_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_ac_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.ac_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.ac_label != "INSUFFICIENT_DATA" {
        assert!(snap.ac_value.is_finite());
        assert!(snap.ao_value.is_finite());
    }
}

#[test]
fn chvol_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = ChvolSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 60,
        ema_length: 10,
        roc_length: 10,
        chvol_value: 12.5,
        chvol_prev: 10.0,
        ema_range: 2.3,
        last_close: 100.0,
        chvol_label: "EXPANDING".into(),
        note: String::new(),
    };
    upsert_chvol(&conn, "TEST", &snap).unwrap();
    let got = get_chvol(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.chvol_label, "EXPANDING");
    assert!((got.chvol_value - 12.5).abs() < 1e-6);
}

#[test]
fn chvol_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_chvol_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.chvol_label.as_str(),
        "EXPANDING" | "CONTRACTING" | "NEUTRAL" | "INSUFFICIENT_DATA"
    ));
    if snap.chvol_label != "INSUFFICIENT_DATA" {
        assert!(snap.chvol_value.is_finite());
        assert!(snap.ema_range >= 0.0);
    }
}

#[test]
fn bbwidth_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = BbwidthSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 130,
        length: 20,
        num_stdev: 2.0,
        bbw_value: 0.08,
        bbw_prev: 0.12,
        bbw_percentile: 3.5,
        middle: 100.0,
        upper: 104.0,
        lower: 96.0,
        last_close: 100.0,
        bbw_label: "SQUEEZE".into(),
        note: String::new(),
    };
    upsert_bbwidth(&conn, "TEST", &snap).unwrap();
    let got = get_bbwidth(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.bbw_label, "SQUEEZE");
    assert!((got.bbw_percentile - 3.5).abs() < 1e-6);
}

#[test]
fn bbwidth_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_bbwidth_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.bbw_label.as_str(),
        "SQUEEZE" | "LOW" | "NORMAL" | "EXPANDED" | "INSUFFICIENT_DATA"
    ));
    if snap.bbw_label != "INSUFFICIENT_DATA" {
        assert!(snap.bbw_value.is_finite());
        assert!(snap.bbw_value >= 0.0);
        assert!(snap.bbw_percentile >= 0.0 && snap.bbw_percentile <= 100.0);
    }
}

#[test]
fn elder_impulse_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = ElderImpulseSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 50,
        ema_length: 13,
        ema_value: 100.5,
        ema_slope: 0.3,
        macd_hist: 0.5,
        macd_hist_prev: 0.2,
        macd_hist_slope: 0.3,
        last_close: 101.0,
        impulse_label: "GREEN".into(),
        note: String::new(),
    };
    upsert_elderimp(&conn, "TEST", &snap).unwrap();
    let got = get_elderimp(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.impulse_label, "GREEN");
    assert!((got.ema_slope - 0.3).abs() < 1e-6);
}

#[test]
fn elder_impulse_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_elder_impulse_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.impulse_label.as_str(),
        "GREEN" | "RED" | "BLUE" | "INSUFFICIENT_DATA"
    ));
    if snap.impulse_label != "INSUFFICIENT_DATA" {
        assert!(snap.ema_value.is_finite());
        assert!(snap.macd_hist.is_finite());
    }
}

#[test]
fn rmi_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = RmiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 40,
        length: 14,
        momentum_length: 5,
        rmi_value: 72.5,
        rmi_prev: 68.0,
        last_close: 100.0,
        rmi_label: "OVERBOUGHT".into(),
        note: String::new(),
    };
    upsert_rmi(&conn, "TEST", &snap).unwrap();
    let got = get_rmi(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.rmi_label, "OVERBOUGHT");
    assert!((got.rmi_value - 72.5).abs() < 1e-6);
}

#[test]
fn rmi_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_rmi_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.rmi_label.as_str(),
        "OVERBOUGHT" | "BULL" | "NEUTRAL" | "BEAR" | "OVERSOLD" | "INSUFFICIENT_DATA"
    ));
    if snap.rmi_label != "INSUFFICIENT_DATA" {
        assert!(snap.rmi_value.is_finite());
        assert!(snap.rmi_value >= 0.0 && snap.rmi_value <= 100.0);
    }
}

#[test]
fn third_friday_identification() {
    use chrono::NaiveDate;
    // 2026-04-17 is the 3rd Friday of April 2026
    assert!(is_third_friday(
        &NaiveDate::from_ymd_opt(2026, 4, 17).unwrap()
    ));
    // 2026-04-10 is the 2nd Friday
    assert!(!is_third_friday(
        &NaiveDate::from_ymd_opt(2026, 4, 10).unwrap()
    ));
    // 2026-03-20 is the 3rd Friday of March (triple witching)
    assert!(is_third_friday(
        &NaiveDate::from_ymd_opt(2026, 3, 20).unwrap()
    ));
}

#[test]
fn triple_witching_months() {
    use chrono::NaiveDate;
    // 3rd Fridays of Mar/Jun/Sep/Dec 2026
    assert!(is_triple_witching(
        &NaiveDate::from_ymd_opt(2026, 3, 20).unwrap()
    ));
    assert!(is_triple_witching(
        &NaiveDate::from_ymd_opt(2026, 6, 19).unwrap()
    ));
    assert!(is_triple_witching(
        &NaiveDate::from_ymd_opt(2026, 9, 18).unwrap()
    ));
    assert!(is_triple_witching(
        &NaiveDate::from_ymd_opt(2026, 12, 18).unwrap()
    ));
    // April 3rd Friday is not TW
    assert!(!is_triple_witching(
        &NaiveDate::from_ymd_opt(2026, 4, 17).unwrap()
    ));
}

#[test]
fn classify_expiration_categories() {
    use chrono::NaiveDate;
    let ref_date = NaiveDate::from_ymd_opt(2026, 4, 17).unwrap();
    // Next week Friday (short horizon) → WEEKLY
    let weekly = NaiveDate::from_ymd_opt(2026, 4, 24).unwrap();
    assert_eq!(classify_expiration(&weekly, &ref_date), "WEEKLY");
    // 3rd Friday of May 2026 → MONTHLY
    let monthly = NaiveDate::from_ymd_opt(2026, 5, 15).unwrap();
    assert_eq!(classify_expiration(&monthly, &ref_date), "MONTHLY");
    // 3rd Friday of June 2026 → TRIPLE_WITCHING
    let tw = NaiveDate::from_ymd_opt(2026, 6, 19).unwrap();
    assert_eq!(classify_expiration(&tw, &ref_date), "TRIPLE_WITCHING");
    // 3rd Friday > 270 days out → LEAPS
    let leaps = NaiveDate::from_ymd_opt(2028, 1, 21).unwrap();
    assert_eq!(classify_expiration(&leaps, &ref_date), "LEAPS");
}

#[test]
fn market_calendar_emits_fridays() {
    use chrono::{Datelike, NaiveDate, Weekday};
    let from = NaiveDate::from_ymd_opt(2026, 4, 17).unwrap();
    let cal = compute_market_calendar(from, 60);
    assert!(!cal.is_empty());
    for e in &cal {
        let d = NaiveDate::parse_from_str(&e.date, "%Y-%m-%d").unwrap();
        assert_eq!(d.weekday(), Weekday::Fri);
        assert!(e.days_from_now >= 0);
    }
    // ~8-9 Fridays in a 60-day horizon starting from a Friday
    assert!(cal.len() >= 8 && cal.len() <= 10);
}

#[test]
fn symbol_expirations_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = SymbolExpirationsSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        underlying_price: 150.0,
        expirations: vec![SymbolExpiration {
            date: "2026-04-24".into(),
            days_to_expiry: 7,
            expiry_type: "WEEKLY".into(),
            call_count: 20,
            put_count: 20,
            total_call_volume: 5000.0,
            total_put_volume: 3000.0,
            total_call_oi: 15000.0,
            total_put_oi: 10000.0,
            put_call_ratio: 0.6,
        }],
        next_triple_witching: "2026-06-19".into(),
        note: String::new(),
    };
    upsert_symbol_expirations(&conn, "TEST", &snap).unwrap();
    let got = get_symbol_expirations(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.symbol, "TEST");
    assert_eq!(got.expirations.len(), 1);
    assert_eq!(got.expirations[0].expiry_type, "WEEKLY");
    assert!((got.expirations[0].put_call_ratio - 0.6).abs() < 1e-6);
}

#[test]
fn smma_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = SmmaSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 40,
        length: 14,
        smma_value: 101.2,
        smma_prev: 100.8,
        deviation_pct: -1.2,
        last_close: 100.0,
        smma_label: "BEAR".into(),
        note: String::new(),
    };
    upsert_smma(&conn, "TEST", &snap).unwrap();
    let got = get_smma(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.smma_label, "BEAR");
    assert!((got.smma_value - 101.2).abs() < 1e-6);
}

#[test]
fn smma_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_smma_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.smma_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.smma_label != "INSUFFICIENT_DATA" {
        assert!(snap.smma_value.is_finite());
        assert!(snap.smma_value > 0.0);
    }
}

#[test]
fn alligator_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = AlligatorSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 60,
        jaw: 98.0,
        teeth: 99.0,
        lips: 100.0,
        jaw_prev: 97.5,
        teeth_prev: 98.5,
        lips_prev: 99.5,
        spread_pct: 2.0,
        last_close: 100.5,
        alligator_label: "EATING_UP".into(),
        note: String::new(),
    };
    upsert_alligator(&conn, "TEST", &snap).unwrap();
    let got = get_alligator(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.alligator_label, "EATING_UP");
    assert!((got.lips - 100.0).abs() < 1e-6);
}

#[test]
fn alligator_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_alligator_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.alligator_label.as_str(),
        "EATING_UP" | "EATING_DOWN" | "AWAKENING" | "SLEEPING" | "INSUFFICIENT_DATA"
    ));
    if snap.alligator_label != "INSUFFICIENT_DATA" {
        assert!(snap.jaw.is_finite() && snap.teeth.is_finite() && snap.lips.is_finite());
        assert!(snap.spread_pct >= 0.0);
    }
}

#[test]
fn crsi_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CrsiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 120,
        rsi_length: 3,
        streak_length: 2,
        rank_lookback: 100,
        rsi_close: 65.0,
        rsi_streak: 55.0,
        percent_rank: 72.0,
        crsi_value: 64.0,
        crsi_prev: 60.0,
        last_close: 100.0,
        crsi_label: "BULLISH".into(),
        note: String::new(),
    };
    upsert_crsi(&conn, "TEST", &snap).unwrap();
    let got = get_crsi(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.crsi_label, "BULLISH");
    assert!((got.crsi_value - 64.0).abs() < 1e-6);
}

#[test]
fn crsi_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_crsi_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.crsi_label.as_str(),
        "OVERBOUGHT" | "BULLISH" | "NEUTRAL" | "BEARISH" | "OVERSOLD" | "INSUFFICIENT_DATA"
    ));
    if snap.crsi_label != "INSUFFICIENT_DATA" {
        assert!(snap.crsi_value.is_finite());
        assert!(snap.crsi_value >= 0.0 && snap.crsi_value <= 100.0);
    }
}

#[test]
fn seb_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = SebSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 40,
        length: 20,
        num_se: 2.0,
        upper: 105.0,
        middle: 100.0,
        lower: 95.0,
        bandwidth: 0.10,
        position_pct: 50.0,
        last_close: 100.0,
        seb_label: "NEUTRAL".into(),
        note: String::new(),
    };
    upsert_seb(&conn, "TEST", &snap).unwrap();
    let got = get_seb(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.seb_label, "NEUTRAL");
    assert!((got.middle - 100.0).abs() < 1e-6);
}

#[test]
fn seb_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_seb_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.seb_label.as_str(),
        "ABOVE_BAND" | "UPPER_HALF" | "NEUTRAL" | "LOWER_HALF" | "BELOW_BAND" | "INSUFFICIENT_DATA"
    ));
    if snap.seb_label != "INSUFFICIENT_DATA" {
        assert!(snap.upper >= snap.middle && snap.middle >= snap.lower);
        assert!(snap.bandwidth.is_finite());
    }
}

#[test]
fn imi_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = ImiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 40,
        length: 14,
        sum_gains: 12.0,
        sum_losses: 8.0,
        imi_value: 60.0,
        imi_prev: 55.0,
        last_close: 100.0,
        imi_label: "BULL".into(),
        note: String::new(),
    };
    upsert_imi(&conn, "TEST", &snap).unwrap();
    let got = get_imi(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.imi_label, "BULL");
    assert!((got.imi_value - 60.0).abs() < 1e-6);
}

#[test]
fn imi_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_imi_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.imi_label.as_str(),
        "OVERBOUGHT" | "BULL" | "NEUTRAL" | "BEAR" | "OVERSOLD" | "INSUFFICIENT_DATA"
    ));
    if snap.imi_label != "INSUFFICIENT_DATA" {
        assert!(snap.imi_value.is_finite());
        assert!(snap.imi_value >= 0.0 && snap.imi_value <= 100.0);
    }
}

#[test]
fn gmma_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = GmmaSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 120,
        short_ema_avg: 101.0,
        long_ema_avg: 99.0,
        short_min: 100.5,
        short_max: 101.5,
        long_min: 98.0,
        long_max: 100.0,
        short_compression_pct: 1.0,
        long_compression_pct: 2.0,
        group_gap_pct: 2.0,
        last_close: 100.0,
        gmma_label: "UPTREND".into(),
        note: String::new(),
    };
    upsert_gmma(&conn, "TEST", &snap).unwrap();
    let got = get_gmma(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.gmma_label, "UPTREND");
    assert!((got.short_ema_avg - 101.0).abs() < 1e-6);
}

#[test]
fn gmma_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_gmma_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.gmma_label.as_str(),
        "STRONG_UPTREND"
            | "UPTREND"
            | "COMPRESSION"
            | "DOWNTREND"
            | "STRONG_DOWNTREND"
            | "NEUTRAL"
            | "INSUFFICIENT_DATA"
    ));
    if snap.gmma_label != "INSUFFICIENT_DATA" {
        assert!(snap.short_ema_avg.is_finite());
        assert!(snap.long_ema_avg.is_finite());
    }
}

#[test]
fn maenv_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MaenvSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 40,
        length: 20,
        pct_band: 2.5,
        upper: 102.5,
        middle: 100.0,
        lower: 97.5,
        bandwidth_pct: 5.0,
        position_pct: 50.0,
        last_close: 100.0,
        maenv_label: "NEUTRAL".into(),
        note: String::new(),
    };
    upsert_maenv(&conn, "TEST", &snap).unwrap();
    let got = get_maenv(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.maenv_label, "NEUTRAL");
    assert!((got.middle - 100.0).abs() < 1e-6);
}

#[test]
fn maenv_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_maenv_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.maenv_label.as_str(),
        "ABOVE_BAND" | "UPPER_HALF" | "NEUTRAL" | "LOWER_HALF" | "BELOW_BAND" | "INSUFFICIENT_DATA"
    ));
    if snap.maenv_label != "INSUFFICIENT_DATA" {
        assert!(snap.upper >= snap.middle && snap.middle >= snap.lower);
        assert!(snap.bandwidth_pct > 0.0);
    }
}

#[test]
fn adl_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = AdlSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 40,
        adl_value: 1_500_000.0,
        adl_prev: 1_450_000.0,
        adl_sma_length: 20,
        adl_sma: 1_300_000.0,
        slope_per_bar: 50_000.0,
        last_close: 100.0,
        price_delta_pct: 3.5,
        adl_label: "ACCUMULATION".into(),
        note: String::new(),
    };
    upsert_adl(&conn, "TEST", &snap).unwrap();
    let got = get_adl(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.adl_label, "ACCUMULATION");
    assert!((got.adl_value - 1_500_000.0).abs() < 1e-6);
}

#[test]
fn adl_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_adl_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.adl_label.as_str(),
        "STRONG_ACCUMULATION"
            | "ACCUMULATION"
            | "NEUTRAL"
            | "DISTRIBUTION"
            | "STRONG_DISTRIBUTION"
            | "INSUFFICIENT_DATA"
    ));
    if snap.adl_label != "INSUFFICIENT_DATA" {
        assert!(snap.adl_value.is_finite());
        assert!(snap.adl_sma.is_finite());
    }
}

#[test]
fn vhf_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = VhfSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 60,
        length: 28,
        highest_high: 105.0,
        lowest_low: 95.0,
        sum_abs_delta: 20.0,
        vhf_value: 0.5,
        vhf_prev: 0.45,
        last_close: 100.0,
        vhf_label: "TREND".into(),
        note: String::new(),
    };
    upsert_vhf(&conn, "TEST", &snap).unwrap();
    let got = get_vhf(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.vhf_label, "TREND");
    assert!((got.vhf_value - 0.5).abs() < 1e-6);
}

#[test]
fn vhf_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_vhf_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.vhf_label.as_str(),
        "STRONG_TREND" | "TREND" | "NEUTRAL" | "RANGING" | "STRONG_RANGING" | "INSUFFICIENT_DATA"
    ));
    if snap.vhf_label != "INSUFFICIENT_DATA" {
        assert!(snap.vhf_value.is_finite());
        assert!(snap.vhf_value >= 0.0);
    }
}

#[test]
fn vroc_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = VrocSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 30,
        length: 14,
        volume_now: 1_200_000.0,
        volume_then: 1_000_000.0,
        vroc_value: 20.0,
        vroc_prev: 15.0,
        last_close: 100.0,
        vroc_label: "NEUTRAL".into(),
        note: String::new(),
    };
    upsert_vroc(&conn, "TEST", &snap).unwrap();
    let got = get_vroc(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.vroc_label, "NEUTRAL");
    assert!((got.vroc_value - 20.0).abs() < 1e-6);
}

#[test]
fn vroc_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_vroc_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.vroc_label.as_str(),
        "SURGE" | "ELEVATED" | "NEUTRAL" | "QUIET" | "COLLAPSE" | "INSUFFICIENT_DATA"
    ));
    if snap.vroc_label != "INSUFFICIENT_DATA" {
        assert!(snap.vroc_value.is_finite());
    }
}

#[test]
fn kdj_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = KdjSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        stoch_length: 9,
        k_smooth: 3,
        rsv: 70.0,
        k_value: 65.0,
        d_value: 60.0,
        j_value: 75.0,
        j_prev: 70.0,
        last_close: 100.0,
        kdj_label: "BULL".into(),
        note: String::new(),
    };
    upsert_kdj(&conn, "TEST", &snap).unwrap();
    let got = get_kdj(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.kdj_label, "BULL");
    assert!((got.j_value - 75.0).abs() < 1e-6);
}

#[test]
fn kdj_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_kdj_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.kdj_label.as_str(),
        "OVERBOUGHT" | "BULL" | "NEUTRAL" | "BEAR" | "OVERSOLD" | "INSUFFICIENT_DATA"
    ));
    if snap.kdj_label != "INSUFFICIENT_DATA" {
        assert!(snap.k_value.is_finite());
        assert!(snap.k_value >= 0.0 && snap.k_value <= 100.0);
        assert!(snap.d_value >= 0.0 && snap.d_value <= 100.0);
        assert!(snap.j_value.is_finite());
    }
}

#[test]
fn qqe_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = QqeSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        rsi_length: 14,
        smooth_length: 5,
        qqe_factor: 4.236,
        rsi_value: 62.0,
        rsi_smoothed: 60.0,
        fast_atr_rsi_avg: 1.5,
        upper_band: 66.354,
        lower_band: 53.646,
        qqe_prev: 58.0,
        last_close: 100.0,
        qqe_label: "BULL".into(),
        note: String::new(),
    };
    upsert_qqe(&conn, "TEST", &snap).unwrap();
    let got = get_qqe(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.qqe_label, "BULL");
    assert!((got.rsi_smoothed - 60.0).abs() < 1e-6);
}

#[test]
fn qqe_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_qqe_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.qqe_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.qqe_label != "INSUFFICIENT_DATA" {
        assert!(snap.rsi_smoothed.is_finite());
        assert!(snap.rsi_smoothed >= 0.0 && snap.rsi_smoothed <= 100.0);
    }
}

#[test]
fn pmo_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = PmoSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        smooth1_length: 35,
        smooth2_length: 20,
        signal_length: 10,
        pmo_value: 2.5,
        pmo_signal: 1.8,
        pmo_prev: 2.1,
        histogram: 0.7,
        last_close: 100.0,
        pmo_label: "BULL".into(),
        note: String::new(),
    };
    upsert_pmo(&conn, "TEST", &snap).unwrap();
    let got = get_pmo(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.pmo_label, "BULL");
    assert!((got.pmo_value - 2.5).abs() < 1e-6);
}

#[test]
fn pmo_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_pmo_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.pmo_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.pmo_label != "INSUFFICIENT_DATA" {
        assert!(snap.pmo_value.is_finite());
    }
}

#[test]
fn cfo_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CfoSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        length: 14,
        slope: 0.25,
        intercept: 98.0,
        forecast: 101.5,
        cfo_value: -1.5,
        cfo_prev: -0.8,
        last_close: 100.0,
        cfo_label: "BELOW_TREND".into(),
        note: String::new(),
    };
    upsert_cfo(&conn, "TEST", &snap).unwrap();
    let got = get_cfo(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cfo_label, "BELOW_TREND");
    assert!((got.cfo_value - (-1.5)).abs() < 1e-6);
}

#[test]
fn cfo_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_cfo_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.cfo_label.as_str(),
        "STRONG_ABOVE_TREND"
            | "ABOVE_TREND"
            | "NEUTRAL"
            | "BELOW_TREND"
            | "STRONG_BELOW_TREND"
            | "INSUFFICIENT_DATA"
    ));
    if snap.cfo_label != "INSUFFICIENT_DATA" {
        assert!(snap.cfo_value.is_finite());
    }
}

#[test]
fn tmf_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = TmfSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        length: 21,
        ema_money_flow: 120_000.0,
        ema_volume: 1_000_000.0,
        tmf_value: 0.12,
        tmf_prev: 0.08,
        last_close: 100.0,
        tmf_label: "INFLOW".into(),
        note: String::new(),
    };
    upsert_tmf(&conn, "TEST", &snap).unwrap();
    let got = get_tmf(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.tmf_label, "INFLOW");
    assert!((got.tmf_value - 0.12).abs() < 1e-6);
}

#[test]
fn tmf_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_tmf_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.tmf_label.as_str(),
        "STRONG_INFLOW" | "INFLOW" | "NEUTRAL" | "OUTFLOW" | "STRONG_OUTFLOW" | "INSUFFICIENT_DATA"
    ));
    if snap.tmf_label != "INSUFFICIENT_DATA" {
        assert!(snap.tmf_value.is_finite());
        assert!(snap.tmf_value >= -1.0 && snap.tmf_value <= 1.0);
    }
}

#[test]
fn fractals_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = FractalsSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        window: 5,
        last_up_high: 125.5,
        last_up_bars_ago: 7,
        last_down_low: 98.25,
        last_down_bars_ago: 12,
        up_fractal_count: 14,
        down_fractal_count: 11,
        last_close: 120.0,
        fractals_label: "UP_RECENT".into(),
        note: String::new(),
    };
    upsert_fractals(&conn, "TEST", &snap).unwrap();
    let got = get_fractals(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.fractals_label, "UP_RECENT");
    assert_eq!(got.up_fractal_count, 14);
    assert_eq!(got.last_up_bars_ago, 7);
}

#[test]
fn fractals_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_fractals_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.fractals_label.as_str(),
        "UP_RECENT" | "DOWN_RECENT" | "BOTH_RECENT" | "NONE_RECENT" | "INSUFFICIENT_DATA"
    ));
    if snap.fractals_label != "INSUFFICIENT_DATA" {
        assert!(snap.last_close.is_finite());
        assert_eq!(snap.window, 5);
    }
}

#[test]
fn ift_rsi_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = IftRsiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        rsi_length: 14,
        wma_length: 9,
        rsi_value: 62.5,
        v_value: 1.25,
        ift_value: 0.75,
        ift_prev: 0.55,
        last_close: 123.0,
        ift_rsi_label: "STRONG_BULL".into(),
        note: String::new(),
    };
    upsert_ift_rsi(&conn, "TEST", &snap).unwrap();
    let got = get_ift_rsi(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.ift_rsi_label, "STRONG_BULL");
    assert!((got.ift_value - 0.75).abs() < 1e-6);
}

#[test]
fn ift_rsi_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_ift_rsi_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.ift_rsi_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.ift_rsi_label != "INSUFFICIENT_DATA" {
        assert!(snap.ift_value.is_finite());
        assert!(snap.ift_value >= -1.0 && snap.ift_value <= 1.0);
    }
}

#[test]
fn mama_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MamaSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        fast_limit: 0.5,
        slow_limit: 0.05,
        mama_value: 110.25,
        fama_value: 108.5,
        mama_prev: 110.0,
        fama_prev: 108.25,
        alpha: 0.37,
        period: 18.5,
        last_close: 111.0,
        mama_label: "BULL".into(),
        note: String::new(),
    };
    upsert_mama(&conn, "TEST", &snap).unwrap();
    let got = get_mama(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.mama_label, "BULL");
    assert!((got.mama_value - 110.25).abs() < 1e-6);
}

#[test]
fn mama_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_mama_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.mama_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.mama_label != "INSUFFICIENT_DATA" {
        assert!(snap.mama_value.is_finite());
        assert!(snap.fama_value.is_finite());
    }
}

#[test]
fn cog_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CogSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        length: 10,
        cog_value: -5.42,
        cog_signal: -5.31,
        cog_prev: -5.45,
        last_close: 120.0,
        cog_label: "BEAR".into(),
        note: String::new(),
    };
    upsert_cog(&conn, "TEST", &snap).unwrap();
    let got = get_cog(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cog_label, "BEAR");
    assert!((got.cog_value - -5.42).abs() < 1e-6);
}

#[test]
fn cog_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_cog_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.cog_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.cog_label != "INSUFFICIENT_DATA" {
        assert!(snap.cog_value.is_finite());
    }
}

#[test]
fn didi_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = DidiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        short_length: 3,
        medium_length: 8,
        long_length: 20,
        short_ratio: 0.015,
        long_ratio: -0.012,
        short_prev: -0.005,
        long_prev: 0.003,
        last_close: 120.0,
        didi_label: "BULL_NEEDLES".into(),
        note: String::new(),
    };
    upsert_didi(&conn, "TEST", &snap).unwrap();
    let got = get_didi(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.didi_label, "BULL_NEEDLES");
    assert!((got.short_ratio - 0.015).abs() < 1e-6);
}

#[test]
fn didi_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_didi_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.didi_label.as_str(),
        "BULL_NEEDLES" | "BULL" | "NEUTRAL" | "BEAR" | "BEAR_NEEDLES" | "INSUFFICIENT_DATA"
    ));
    if snap.didi_label != "INSUFFICIENT_DATA" {
        assert!(snap.short_ratio.is_finite());
        assert!(snap.long_ratio.is_finite());
    }
}

#[test]
fn demarker_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = DemarkerSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        length: 14,
        demax_sum: 12.5,
        demin_sum: 5.0,
        demarker_value: 0.714,
        demarker_prev: 0.65,
        last_close: 100.0,
        demarker_label: "OVERBOUGHT".into(),
        note: String::new(),
    };
    upsert_demarker(&conn, "TEST", &snap).unwrap();
    let got = get_demarker(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.demarker_label, "OVERBOUGHT");
    assert!((got.demarker_value - 0.714).abs() < 1e-6);
}

#[test]
fn demarker_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_demarker_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.demarker_label.as_str(),
        "OVERBOUGHT" | "BULL" | "NEUTRAL" | "BEAR" | "OVERSOLD" | "INSUFFICIENT_DATA"
    ));
    if snap.demarker_label != "INSUFFICIENT_DATA" {
        assert!(snap.demarker_value >= 0.0 && snap.demarker_value <= 1.0);
    }
}

#[test]
fn gator_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = GatorSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        jaw_length: 13,
        teeth_length: 8,
        lips_length: 5,
        upper_bar: 0.8,
        lower_bar: -0.5,
        upper_prev: 0.6,
        lower_prev: -0.4,
        last_close: 100.0,
        gator_label: "EATING".into(),
        note: String::new(),
    };
    upsert_gator(&conn, "TEST", &snap).unwrap();
    let got = get_gator(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.gator_label, "EATING");
    assert!((got.upper_bar - 0.8).abs() < 1e-6);
}

#[test]
fn gator_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_gator_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.gator_label.as_str(),
        "SLEEPING" | "AWAKENING" | "EATING" | "SATED" | "INSUFFICIENT_DATA"
    ));
    if snap.gator_label != "INSUFFICIENT_DATA" {
        assert!(snap.upper_bar >= 0.0);
        assert!(snap.lower_bar <= 0.0);
    }
}

#[test]
fn bw_mfi_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = BwMfiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        mfi_value: 5.0,
        mfi_prev: 3.0,
        volume: 2_000_000.0,
        volume_prev: 1_000_000.0,
        last_close: 100.0,
        bwmfi_color: "GREEN".into(),
        bwmfi_label: "GREEN".into(),
        note: String::new(),
    };
    upsert_bw_mfi(&conn, "TEST", &snap).unwrap();
    let got = get_bw_mfi(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.bwmfi_color, "GREEN");
    assert!((got.mfi_value - 5.0).abs() < 1e-6);
}

#[test]
fn bw_mfi_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_bw_mfi_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.bwmfi_label.as_str(),
        "GREEN" | "FADE" | "FAKE" | "SQUAT" | "INSUFFICIENT_DATA"
    ));
    if snap.bwmfi_label != "INSUFFICIENT_DATA" {
        assert!(snap.mfi_value.is_finite());
    }
}

#[test]
fn vwma_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = VwmaSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        length: 20,
        vwma_value: 101.5,
        sma_value: 100.8,
        vwma_prev: 101.0,
        spread: 0.7,
        spread_ratio: 0.00695,
        last_close: 102.0,
        vwma_label: "BULL".into(),
        note: String::new(),
    };
    upsert_vwma(&conn, "TEST", &snap).unwrap();
    let got = get_vwma(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.vwma_label, "BULL");
    assert!((got.vwma_value - 101.5).abs() < 1e-6);
}

#[test]
fn vwma_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_vwma_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.vwma_label.as_str(),
        "BULL" | "WEAK_BULL" | "NEUTRAL" | "WEAK_BEAR" | "BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.vwma_label != "INSUFFICIENT_DATA" {
        assert!(snap.vwma_value.is_finite());
        assert!(snap.sma_value.is_finite());
    }
}

#[test]
fn stddev_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = StddevSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 100,
        length: 20,
        long_length: 60,
        mean: 100.0,
        variance: 4.0,
        stddev: 2.0,
        stddev_long: 1.5,
        cv: 0.02,
        annualized: 31.75,
        last_close: 100.0,
        regime_label: "HIGH_VOL".into(),
        note: String::new(),
    };
    upsert_stddev(&conn, "TEST", &snap).unwrap();
    let got = get_stddev(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.regime_label, "HIGH_VOL");
    assert!((got.stddev - 2.0).abs() < 1e-6);
}

#[test]
fn stddev_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_stddev_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.regime_label.as_str(),
        "HIGH_VOL" | "MID_VOL" | "LOW_VOL" | "INSUFFICIENT_DATA"
    ));
    if snap.regime_label != "INSUFFICIENT_DATA" {
        assert!(snap.stddev >= 0.0);
        assert!(snap.stddev_long >= 0.0);
    }
}
