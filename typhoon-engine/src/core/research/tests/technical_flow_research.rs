// ── Research section ──

#[test]
fn mcleodli_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = McLeodLiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        lag_h: 10,
        q_stat: 22.5,
        df: 10,
        critical_95: 18.307,
        p_value: 0.013,
        reject_null: true,
        mcleodli_label: "MILD_ARCH".into(),
        note: String::new(),
    };
    upsert_mcleodli(&conn, "TEST", &snap).unwrap();
    let got = get_mcleodli(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.mcleodli_label, "MILD_ARCH");
    assert!((got.q_stat - 22.5).abs() < 1e-9);
    assert_eq!(got.lag_h, 10);
}

#[test]
fn mcleodli_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_mcleodli_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.mcleodli_label.as_str(),
        "NO_ARCH" | "MILD_ARCH" | "STRONG_ARCH" | "INSUFFICIENT_DATA"
    ));
    if snap.mcleodli_label != "INSUFFICIENT_DATA" {
        assert!(snap.q_stat.is_finite() && snap.q_stat >= 0.0);
        assert!(snap.lag_h >= 5);
        assert!(snap.critical_95 > 0.0);
        assert!(snap.p_value >= 0.0 && snap.p_value <= 1.0);
        assert_eq!(snap.df, snap.lag_h);
    }
}

#[test]
fn oufit_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = OuFitSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        theta: 0.045,
        mu: 4.2,
        sigma: 0.018,
        half_life_bars: 15.4,
        residual_sd: 0.018,
        r_squared: 0.91,
        oufit_label: "MODERATE_REVERT".into(),
        note: String::new(),
    };
    upsert_oufit(&conn, "TEST", &snap).unwrap();
    let got = get_oufit(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.oufit_label, "MODERATE_REVERT");
    assert!((got.theta - 0.045).abs() < 1e-9);
    assert!((got.half_life_bars - 15.4).abs() < 1e-9);
}

#[test]
fn oufit_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_oufit_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.oufit_label.as_str(),
        "TRENDING" | "SLOW_REVERT" | "MODERATE_REVERT" | "FAST_REVERT" | "INSUFFICIENT_DATA"
    ));
    if snap.oufit_label != "INSUFFICIENT_DATA" {
        assert!(snap.theta.is_finite());
        assert!(snap.mu.is_finite());
        assert!(snap.sigma.is_finite() && snap.sigma >= 0.0);
        assert!(snap.residual_sd >= 0.0);
        assert!(snap.r_squared >= 0.0 && snap.r_squared <= 1.0);
    }
}

#[test]
fn gph_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = GphSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        m_freqs: 16,
        d_estimate: 0.12,
        d_stderr: 0.09,
        t_stat: 1.33,
        p_value_two_sided: 0.18,
        gph_label: "LONG_MEMORY".into(),
        note: String::new(),
    };
    upsert_gph(&conn, "TEST", &snap).unwrap();
    let got = get_gph(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.gph_label, "LONG_MEMORY");
    assert!((got.d_estimate - 0.12).abs() < 1e-9);
    assert_eq!(got.m_freqs, 16);
}

#[test]
fn gph_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_gph_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.gph_label.as_str(),
        "ANTIPERSISTENT" | "SHORT_MEMORY" | "LONG_MEMORY" | "NONSTATIONARY" | "INSUFFICIENT_DATA"
    ));
    if snap.gph_label != "INSUFFICIENT_DATA" {
        assert!(snap.d_estimate.is_finite());
        assert!(snap.d_stderr > 0.0);
        assert!(snap.t_stat.is_finite());
        assert!(snap.p_value_two_sided >= 0.0 && snap.p_value_two_sided <= 1.0);
        assert!(snap.m_freqs >= 4);
    }
}

#[test]
fn burgspec_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = BurgSpecSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        ar_order: 12,
        dominant_freq: 0.04,
        dominant_period_bars: 25.0,
        peak_power: 4.2e-3,
        mean_power: 1.1e-3,
        peak_to_mean_ratio: 3.82,
        burgspec_label: "WEAK_AR_CYCLE".into(),
        note: String::new(),
    };
    upsert_burgspec(&conn, "TEST", &snap).unwrap();
    let got = get_burgspec(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.burgspec_label, "WEAK_AR_CYCLE");
    assert_eq!(got.ar_order, 12);
    assert!((got.dominant_period_bars - 25.0).abs() < 1e-9);
}

#[test]
fn burgspec_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_burgspec_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.burgspec_label.as_str(),
        "NO_AR_CYCLE"
            | "WEAK_AR_CYCLE"
            | "MODERATE_AR_CYCLE"
            | "STRONG_AR_CYCLE"
            | "INSUFFICIENT_DATA"
    ));
    if snap.burgspec_label != "INSUFFICIENT_DATA" {
        assert!(snap.ar_order >= 2);
        assert!(snap.dominant_freq > 0.0);
        assert!(snap.dominant_period_bars > 0.0);
        assert!(snap.peak_power >= 0.0);
        assert!(snap.mean_power > 0.0);
        assert!(snap.peak_to_mean_ratio >= 0.0);
    }
}

#[test]
fn kendalltau_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = KendallTauSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        pair_count: 31626,
        concordant: 15800,
        discordant: 15826,
        tau: -0.0008,
        z_stat: -0.012,
        p_value_two_sided: 0.99,
        kendalltau_label: "NO_RANK_AUTO".into(),
        note: String::new(),
    };
    upsert_kendalltau(&conn, "TEST", &snap).unwrap();
    let got = get_kendalltau(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.kendalltau_label, "NO_RANK_AUTO");
    assert_eq!(got.concordant, 15800);
    assert!((got.tau - -0.0008).abs() < 1e-9);
}

#[test]
fn kendalltau_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_kendalltau_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.kendalltau_label.as_str(),
        "STRONG_POS"
            | "WEAK_POS"
            | "NO_RANK_AUTO"
            | "WEAK_NEG"
            | "STRONG_NEG"
            | "INSUFFICIENT_DATA"
    ));
    if snap.kendalltau_label != "INSUFFICIENT_DATA" {
        assert!(snap.tau >= -1.0 && snap.tau <= 1.0);
        let m = snap.bars_used - 1;
        assert_eq!(snap.pair_count, m * (m - 1) / 2);
        // ties contribute 0 to C and 0 to D; oscillating fixture has many ties
        assert!(snap.concordant + snap.discordant <= snap.pair_count);
        assert!(snap.z_stat.is_finite());
        assert!(snap.p_value_two_sided >= 0.0 && snap.p_value_two_sided <= 1.0);
    }
}

// ── Research section ──

#[test]
fn squeeze_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = SqueezeSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        short_percent_of_float: 28.0,
        days_to_cover: 6.2,
        momentum_20d_pct: 18.5,
        relvol_20d: 2.4,
        iv_rank: 78.0,
        short_float_score: 70.0,
        days_to_cover_score: 62.0,
        momentum_score: 61.6,
        relvol_score: 80.0,
        iv_rank_score: 78.0,
        composite_score: 71.2,
        inputs_present: 5,
        squeeze_label: "STRONG".into(),
        note: String::new(),
    };
    upsert_squeeze(&conn, "TEST", &snap).unwrap();
    let got = get_squeeze(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.squeeze_label, "STRONG");
    assert!((got.composite_score - 71.2).abs() < 1e-9);
    assert_eq!(got.inputs_present, 5);
}

#[test]
fn squeeze_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    // With no short/ivol/relvol inputs the oscillating fixture should
    // yield INSUFFICIENT_DATA (only momentum axis is available).
    let snap = compute_squeeze_snapshot("T", "2026-04-17", &bars, None, None, None);
    assert!(matches!(
        snap.squeeze_label.as_str(),
        "NO_SQUEEZE" | "WATCH" | "ELEVATED" | "STRONG" | "EXTREME" | "INSUFFICIENT_DATA"
    ));
    // With 3 synthetic inputs we should get a real label.
    let si = ShortInterestSnapshot {
        symbol: "T".into(),
        short_percent_of_float: 25.0,
        days_to_cover: 5.0,
        ..Default::default()
    };
    let iv = IvolSnapshot {
        symbol: "T".into(),
        iv_rank: 70.0,
        ..Default::default()
    };
    let rv = RelVolSnapshot {
        symbol: "T".into(),
        rel_volume_20d: 2.0,
        ..Default::default()
    };
    let s2 = compute_squeeze_snapshot("T", "2026-04-17", &bars, Some(&si), Some(&iv), Some(&rv));
    assert!(s2.squeeze_label != "INSUFFICIENT_DATA");
    assert!(s2.composite_score.is_finite());
    assert!(s2.composite_score >= 0.0 && s2.composite_score <= 100.0);
    assert!(s2.inputs_present >= 3);
    assert!(s2.short_float_score >= 0.0 && s2.short_float_score <= 100.0);
    assert!(s2.iv_rank_score >= 0.0 && s2.iv_rank_score <= 100.0);
}

#[test]
fn squeezerank_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = SqueezeRankSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        composite_score: 85.0,
        peer_count: 120,
        rank: 3,
        percentile: 97.5,
        squeezerank_label: "TOP_5PCT".into(),
        note: String::new(),
    };
    upsert_squeezerank(&conn, "TEST", &snap).unwrap();
    let got = get_squeezerank(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.squeezerank_label, "TOP_5PCT");
    assert_eq!(got.rank, 3);
    assert!((got.percentile - 97.5).abs() < 1e-9);
}

#[test]
fn squeezerank_compute_oscillating() {
    let _ = synthetic_oscillating_bars_150();
    // Build a synthetic peer set with varying composite scores.
    let mut peers: Vec<SqueezeSnapshot> = Vec::new();
    for i in 0..10 {
        peers.push(SqueezeSnapshot {
            symbol: format!("SYM{}", i),
            composite_score: (i as f64) * 10.0,
            inputs_present: 5,
            squeeze_label: "ELEVATED".into(),
            ..Default::default()
        });
    }
    let top = peers.iter().find(|s| s.symbol == "SYM9").cloned().unwrap();
    let snap = compute_squeezerank_snapshot("SYM9", "2026-04-17", Some(&top), &peers);
    assert!(matches!(
        snap.squeezerank_label.as_str(),
        "TOP_1PCT"
            | "TOP_5PCT"
            | "TOP_10PCT"
            | "ABOVE_MEDIAN"
            | "BELOW_MEDIAN"
            | "INSUFFICIENT_DATA"
    ));
    if snap.squeezerank_label != "INSUFFICIENT_DATA" {
        assert_eq!(snap.peer_count, 10);
        assert_eq!(snap.rank, 1);
        assert!(snap.percentile >= 0.0 && snap.percentile <= 100.0);
    }
    // Small-peer-set path: fewer than 5 peers is INSUFFICIENT_DATA.
    let tiny: Vec<SqueezeSnapshot> = peers.iter().take(3).cloned().collect();
    let s2 = compute_squeezerank_snapshot("SYM0", "2026-04-17", tiny.first(), &tiny);
    assert_eq!(s2.squeezerank_label, "INSUFFICIENT_DATA");
}

#[test]
fn bbsqueeze_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = BbsqueezeSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        period: 20,
        bb_width_current: 0.015,
        bb_width_min_120: 0.010,
        bb_width_max_120: 0.080,
        bb_width_percentile: 8.3,
        upper_band: 155.2,
        lower_band: 150.1,
        mid_band: 152.65,
        last_close: 153.0,
        bbsqueeze_label: "TIGHT_SQUEEZE".into(),
        note: String::new(),
    };
    upsert_bbsqueeze(&conn, "TEST", &snap).unwrap();
    let got = get_bbsqueeze(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.bbsqueeze_label, "TIGHT_SQUEEZE");
    assert!((got.bb_width_current - 0.015).abs() < 1e-9);
    assert_eq!(got.period, 20);
}

#[test]
fn bbsqueeze_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_bbsqueeze_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.bbsqueeze_label.as_str(),
        "TIGHT_SQUEEZE" | "MODERATE_SQUEEZE" | "NORMAL" | "EXPANSION" | "INSUFFICIENT_DATA"
    ));
    if snap.bbsqueeze_label != "INSUFFICIENT_DATA" {
        assert!(snap.bb_width_current.is_finite() && snap.bb_width_current >= 0.0);
        assert!(snap.bb_width_min_120 <= snap.bb_width_max_120);
        assert!(snap.bb_width_percentile >= 0.0 && snap.bb_width_percentile <= 100.0);
        assert!(snap.upper_band >= snap.mid_band);
        assert!(snap.mid_band >= snap.lower_band);
        assert_eq!(snap.period, 20);
    }
}

#[test]
fn donchian_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = DonchianSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        period: 20,
        upper_channel: 160.0,
        lower_channel: 140.0,
        mid_channel: 150.0,
        last_close: 162.5,
        channel_position_pct: 100.0,
        breakout_upper: true,
        breakout_lower: false,
        donchian_label: "BREAKOUT_UP".into(),
        note: String::new(),
    };
    upsert_donchian(&conn, "TEST", &snap).unwrap();
    let got = get_donchian(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.donchian_label, "BREAKOUT_UP");
    assert!(got.breakout_upper);
    assert!((got.upper_channel - 160.0).abs() < 1e-9);
}

#[test]
fn donchian_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_donchian_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.donchian_label.as_str(),
        "BREAKOUT_UP"
            | "APPROACH_UP"
            | "NEUTRAL"
            | "APPROACH_DOWN"
            | "BREAKOUT_DOWN"
            | "INSUFFICIENT_DATA"
    ));
    if snap.donchian_label != "INSUFFICIENT_DATA" {
        assert!(snap.upper_channel >= snap.lower_channel);
        assert!(snap.channel_position_pct >= 0.0 && snap.channel_position_pct <= 100.0);
        assert!(snap.last_close.is_finite());
        assert_eq!(snap.period, 20);
    }
}

#[test]
fn kama_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = KamaSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        period: 10,
        efficiency_ratio: 0.65,
        kama_value: 152.3,
        last_close: 155.0,
        kama_slope_pct: 1.4,
        kama_label: "MODERATE_TREND".into(),
        note: String::new(),
    };
    upsert_kama(&conn, "TEST", &snap).unwrap();
    let got = get_kama(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.kama_label, "MODERATE_TREND");
    assert!((got.efficiency_ratio - 0.65).abs() < 1e-9);
    assert_eq!(got.period, 10);
}

#[test]
fn kama_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_kama_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.kama_label.as_str(),
        "STRONG_TREND" | "MODERATE_TREND" | "WEAK_TREND" | "CHOPPY" | "INSUFFICIENT_DATA"
    ));
    if snap.kama_label != "INSUFFICIENT_DATA" {
        assert!(snap.efficiency_ratio >= 0.0 && snap.efficiency_ratio <= 1.0);
        assert!(snap.kama_value.is_finite());
        assert!(snap.last_close.is_finite());
        assert!(snap.kama_slope_pct.is_finite());
        assert_eq!(snap.period, 10);
    }
}

// ── Research section ──
#[test]
fn ichimoku_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = IchimokuSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        tenkan_sen: 155.0,
        kijun_sen: 150.0,
        senkou_span_a: 152.5,
        senkou_span_b: 148.0,
        chikou_span: 160.0,
        cloud_top: 152.5,
        cloud_bottom: 148.0,
        last_close: 162.0,
        close_vs_cloud_pct: 7.8,
        ichimoku_label: "STRONG_BULL".into(),
        note: String::new(),
    };
    upsert_ichimoku(&conn, "TEST", &snap).unwrap();
    let got = get_ichimoku(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.ichimoku_label, "STRONG_BULL");
    assert!((got.tenkan_sen - 155.0).abs() < 1e-9);
}

#[test]
fn ichimoku_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_ichimoku_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.ichimoku_label.as_str(),
        "STRONG_BULL" | "BULL" | "IN_CLOUD" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.ichimoku_label != "INSUFFICIENT_DATA" {
        assert!(snap.tenkan_sen.is_finite());
        assert!(snap.kijun_sen.is_finite());
        assert!(snap.cloud_top >= snap.cloud_bottom);
        assert!(snap.last_close.is_finite());
    }
}

#[test]
fn supertrend_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = SupertrendSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        period: 10,
        multiplier: 3.0,
        atr: 1.8,
        upper_band: 162.5,
        lower_band: 157.1,
        supertrend_value: 157.1,
        trend_is_up: true,
        last_close: 160.0,
        distance_pct: 1.8,
        bars_in_trend: 12,
        supertrend_label: "UP".into(),
        note: String::new(),
    };
    upsert_supertrend(&conn, "TEST", &snap).unwrap();
    let got = get_supertrend(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.supertrend_label, "UP");
    assert!(got.trend_is_up);
    assert_eq!(got.period, 10);
}

#[test]
fn supertrend_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_supertrend_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.supertrend_label.as_str(),
        "STRONG_UP" | "UP" | "FLAT" | "STRONG_DOWN" | "DOWN" | "INSUFFICIENT_DATA"
    ));
    if snap.supertrend_label != "INSUFFICIENT_DATA" {
        assert!(snap.atr >= 0.0);
        assert!(snap.upper_band >= snap.lower_band);
        assert_eq!(snap.period, 10);
        assert!((snap.multiplier - 3.0).abs() < 1e-9);
    }
}

#[test]
fn keltner_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = KeltnerSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        ema_period: 20,
        atr_period: 10,
        multiplier: 2.0,
        ema_value: 150.0,
        atr: 1.8,
        upper_channel: 153.6,
        lower_channel: 146.4,
        last_close: 152.0,
        channel_width: 7.2,
        width_pct_of_mid: 4.8,
        channel_position_pct: 77.8,
        ttm_squeeze_on: false,
        keltner_label: "IN_CHANNEL".into(),
        note: String::new(),
    };
    upsert_keltner(&conn, "TEST", &snap).unwrap();
    let got = get_keltner(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.keltner_label, "IN_CHANNEL");
    assert_eq!(got.ema_period, 20);
    assert_eq!(got.atr_period, 10);
}

#[test]
fn keltner_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_keltner_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.keltner_label.as_str(),
        "BREAKOUT_UP"
            | "NEAR_UPPER"
            | "IN_CHANNEL"
            | "NEAR_LOWER"
            | "BREAKOUT_DOWN"
            | "INSUFFICIENT_DATA"
    ));
    if snap.keltner_label != "INSUFFICIENT_DATA" {
        assert!(snap.atr >= 0.0);
        assert!(snap.upper_channel > snap.lower_channel);
        assert!(snap.channel_width > 0.0);
        assert!(snap.channel_position_pct >= 0.0 && snap.channel_position_pct <= 100.0);
    }
}

#[test]
fn fisher_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = FisherSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        period: 10,
        fisher_value: 1.25,
        fisher_signal: 0.75,
        extreme_2_cross: false,
        peak_abs_10: 1.8,
        last_close: 160.0,
        fisher_label: "POS".into(),
        note: String::new(),
    };
    upsert_fisher(&conn, "TEST", &snap).unwrap();
    let got = get_fisher(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.fisher_label, "POS");
    assert!((got.fisher_value - 1.25).abs() < 1e-9);
}

#[test]
fn fisher_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_fisher_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.fisher_label.as_str(),
        "STRONG_POS" | "POS" | "NEUTRAL" | "NEG" | "STRONG_NEG" | "INSUFFICIENT_DATA"
    ));
    if snap.fisher_label != "INSUFFICIENT_DATA" {
        assert!(snap.fisher_value.is_finite());
        assert!(snap.fisher_signal.is_finite());
        assert!(snap.peak_abs_10 >= 0.0);
        assert_eq!(snap.period, 10);
    }
}

#[test]
fn aroon_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = AroonSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        period: 25,
        aroon_up: 80.0,
        aroon_down: 20.0,
        aroon_oscillator: 60.0,
        bars_since_high: 5,
        bars_since_low: 20,
        last_close: 160.0,
        aroon_label: "STRONG_UP".into(),
        note: String::new(),
    };
    upsert_aroon(&conn, "TEST", &snap).unwrap();
    let got = get_aroon(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.aroon_label, "STRONG_UP");
    assert_eq!(got.period, 25);
}

#[test]
fn aroon_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_aroon_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.aroon_label.as_str(),
        "STRONG_UP"
            | "WEAK_UP"
            | "CONSOLIDATION"
            | "WEAK_DOWN"
            | "STRONG_DOWN"
            | "INSUFFICIENT_DATA"
    ));
    if snap.aroon_label != "INSUFFICIENT_DATA" {
        assert!(snap.aroon_up >= 0.0 && snap.aroon_up <= 100.0);
        assert!(snap.aroon_down >= 0.0 && snap.aroon_down <= 100.0);
        assert!(snap.aroon_oscillator >= -100.0 && snap.aroon_oscillator <= 100.0);
        assert_eq!(snap.period, 25);
    }
}

// ── Research section ──

#[test]
fn adx_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = AdxSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        period: 14,
        plus_di: 28.0,
        minus_di: 14.0,
        adx: 32.0,
        dx: 33.0,
        atr: 1.9,
        last_close: 150.0,
        adx_label: "TREND".into(),
        note: String::new(),
    };
    upsert_adx(&conn, "TEST", &snap).unwrap();
    let got = get_adx(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.adx_label, "TREND");
    assert_eq!(got.period, 14);
    assert!((got.adx - 32.0).abs() < 1e-9);
}

#[test]
fn adx_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_adx_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.adx_label.as_str(),
        "STRONG_TREND" | "TREND" | "WEAK_TREND" | "NO_TREND" | "INSUFFICIENT_DATA"
    ));
    if snap.adx_label != "INSUFFICIENT_DATA" {
        assert!(snap.plus_di >= 0.0);
        assert!(snap.minus_di >= 0.0);
        assert!(snap.adx >= 0.0 && snap.adx <= 100.0);
        assert!(snap.atr >= 0.0);
        assert_eq!(snap.period, 14);
    }
}

#[test]
fn cci_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CciSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        period: 20,
        typical_price: 150.0,
        tp_sma: 148.5,
        mean_abs_dev: 1.2,
        cci_value: 83.3,
        last_close: 150.5,
        cci_label: "BULL".into(),
        note: String::new(),
    };
    upsert_cci(&conn, "TEST", &snap).unwrap();
    let got = get_cci(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cci_label, "BULL");
    assert_eq!(got.period, 20);
}

#[test]
fn cci_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_cci_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.cci_label.as_str(),
        "OVERBOUGHT" | "BULL" | "NEUTRAL" | "BEAR" | "OVERSOLD" | "INSUFFICIENT_DATA"
    ));
    if snap.cci_label != "INSUFFICIENT_DATA" {
        assert!(snap.typical_price.is_finite());
        assert!(snap.tp_sma.is_finite());
        assert!(snap.mean_abs_dev >= 0.0);
        assert!(snap.cci_value.is_finite());
        assert_eq!(snap.period, 20);
    }
}

#[test]
fn cmf_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CmfSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        period: 20,
        cmf_value: 0.18,
        money_flow_volume_sum: 1.8e8,
        volume_sum: 1.0e9,
        last_close: 150.0,
        cmf_label: "ACCUM".into(),
        note: String::new(),
    };
    upsert_cmf(&conn, "TEST", &snap).unwrap();
    let got = get_cmf(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cmf_label, "ACCUM");
    assert!((got.cmf_value - 0.18).abs() < 1e-9);
}

#[test]
fn cmf_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_cmf_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.cmf_label.as_str(),
        "STRONG_ACCUM" | "ACCUM" | "NEUTRAL" | "DIST" | "STRONG_DIST" | "INSUFFICIENT_DATA"
    ));
    if snap.cmf_label != "INSUFFICIENT_DATA" {
        assert!(snap.cmf_value >= -1.0 && snap.cmf_value <= 1.0);
        assert!(snap.volume_sum >= 0.0);
        assert_eq!(snap.period, 20);
    }
}

#[test]
fn mfi_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MfiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        period: 14,
        mfi_value: 62.0,
        positive_mf_sum: 2.1e8,
        negative_mf_sum: 1.3e8,
        money_flow_ratio: 1.615,
        last_close: 150.0,
        mfi_label: "BULL".into(),
        note: String::new(),
    };
    upsert_mfi(&conn, "TEST", &snap).unwrap();
    let got = get_mfi(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.mfi_label, "BULL");
    assert_eq!(got.period, 14);
}

#[test]
fn mfi_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_mfi_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.mfi_label.as_str(),
        "OVERBOUGHT" | "BULL" | "NEUTRAL" | "BEAR" | "OVERSOLD" | "INSUFFICIENT_DATA"
    ));
    if snap.mfi_label != "INSUFFICIENT_DATA" {
        assert!(snap.mfi_value >= 0.0 && snap.mfi_value <= 100.0);
        assert!(snap.positive_mf_sum >= 0.0);
        assert!(snap.negative_mf_sum >= 0.0);
        assert_eq!(snap.period, 14);
    }
}

#[test]
fn psar_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = PsarSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        af_start: 0.02,
        af_step: 0.02,
        af_max: 0.20,
        sar_value: 148.0,
        extreme_point: 152.0,
        acceleration_factor: 0.06,
        trend_is_up: true,
        bars_in_trend: 8,
        distance_pct: 1.35,
        last_close: 150.0,
        psar_label: "UP".into(),
        note: String::new(),
    };
    upsert_psar(&conn, "TEST", &snap).unwrap();
    let got = get_psar(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.psar_label, "UP");
    assert!(got.trend_is_up);
    assert!((got.sar_value - 148.0).abs() < 1e-9);
}

#[test]
fn psar_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_psar_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.psar_label.as_str(),
        "STRONG_UP" | "UP" | "FLAT" | "DOWN" | "STRONG_DOWN" | "INSUFFICIENT_DATA"
    ));
    if snap.psar_label != "INSUFFICIENT_DATA" {
        assert!(snap.sar_value.is_finite());
        assert!(snap.extreme_point.is_finite());
        assert!(snap.acceleration_factor >= snap.af_start);
        assert!(snap.acceleration_factor <= snap.af_max);
        assert!(snap.bars_in_trend >= 1);
    }
}

// ── Research section ──

#[test]
fn vortex_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = VortexSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        period: 14,
        vi_plus: 1.12,
        vi_minus: 0.88,
        vi_diff: 0.24,
        sum_tr: 42.5,
        sum_vm_plus: 47.6,
        sum_vm_minus: 37.4,
        last_close: 150.0,
        vortex_label: "BULL".into(),
        note: String::new(),
    };
    upsert_vortex(&conn, "TEST", &snap).unwrap();
    let got = get_vortex(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.vortex_label, "BULL");
    assert!((got.vi_plus - 1.12).abs() < 1e-9);
}

#[test]
fn vortex_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_vortex_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.vortex_label.as_str(),
        "BULL_CROSS" | "BULL" | "NEUTRAL" | "BEAR" | "BEAR_CROSS" | "INSUFFICIENT_DATA"
    ));
    if snap.vortex_label != "INSUFFICIENT_DATA" {
        assert!(snap.vi_plus.is_finite() && snap.vi_plus >= 0.0);
        assert!(snap.vi_minus.is_finite() && snap.vi_minus >= 0.0);
        assert!(snap.sum_tr > 0.0);
        assert_eq!(snap.period, 14);
    }
}

#[test]
fn chop_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = ChopSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        period: 14,
        chop_value: 55.4,
        sum_tr: 28.3,
        range_high: 152.0,
        range_low: 145.0,
        range_span: 7.0,
        last_close: 150.0,
        chop_label: "RANGING".into(),
        note: String::new(),
    };
    upsert_chop(&conn, "TEST", &snap).unwrap();
    let got = get_chop(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.chop_label, "RANGING");
    assert!((got.chop_value - 55.4).abs() < 1e-9);
}

#[test]
fn chop_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_chop_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.chop_label.as_str(),
        "CHOP" | "RANGING" | "NEUTRAL" | "TRANSITIONAL" | "TRENDING" | "INSUFFICIENT_DATA"
    ));
    if snap.chop_label != "INSUFFICIENT_DATA" {
        assert!(snap.chop_value.is_finite());
        assert!(snap.chop_value >= 0.0 && snap.chop_value <= 110.0);
        assert!(snap.range_high >= snap.range_low);
        assert_eq!(snap.period, 14);
    }
}

#[test]
fn obv_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = ObvSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        slope_window: 20,
        obv_value: 1_500_000.0,
        obv_slope: 2_500.0,
        obv_change_pct: 3.4,
        obv_min_20: 1_400_000.0,
        obv_max_20: 1_600_000.0,
        last_close: 150.0,
        obv_label: "UP".into(),
        note: String::new(),
    };
    upsert_obv(&conn, "TEST", &snap).unwrap();
    let got = get_obv(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.obv_label, "UP");
    assert!((got.obv_value - 1_500_000.0).abs() < 1e-3);
}

#[test]
fn obv_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_obv_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.obv_label.as_str(),
        "STRONG_UP" | "UP" | "NEUTRAL" | "DOWN" | "STRONG_DOWN" | "INSUFFICIENT_DATA"
    ));
    if snap.obv_label != "INSUFFICIENT_DATA" {
        assert!(snap.obv_value.is_finite());
        assert!(snap.obv_slope.is_finite());
        assert!(snap.obv_max_20 >= snap.obv_min_20);
        assert_eq!(snap.slope_window, 20);
    }
}

#[test]
fn trix_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = TrixSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        period: 15,
        signal_period: 9,
        trix_value: 0.042,
        signal_value: 0.031,
        histogram: 0.011,
        ema3_value: 149.75,
        last_close: 150.0,
        trix_label: "BULL".into(),
        note: String::new(),
    };
    upsert_trix(&conn, "TEST", &snap).unwrap();
    let got = get_trix(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.trix_label, "BULL");
    assert!((got.trix_value - 0.042).abs() < 1e-9);
}

#[test]
fn trix_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_trix_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.trix_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.trix_label != "INSUFFICIENT_DATA" {
        assert!(snap.trix_value.is_finite());
        assert!(snap.signal_value.is_finite());
        assert!(snap.ema3_value.is_finite() && snap.ema3_value > 0.0);
        assert_eq!(snap.period, 15);
        assert_eq!(snap.signal_period, 9);
    }
}

#[test]
fn hma_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = HmaSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        period: 20,
        half_period: 10,
        sqrt_period: 4,
        hma_value: 149.8,
        hma_slope_pct: 0.45,
        hma_vs_close_pct: 0.13,
        last_close: 150.0,
        hma_label: "UP".into(),
        note: String::new(),
    };
    upsert_hma(&conn, "TEST", &snap).unwrap();
    let got = get_hma(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.hma_label, "UP");
    assert!((got.hma_value - 149.8).abs() < 1e-9);
}

#[test]
fn hma_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_hma_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.hma_label.as_str(),
        "STRONG_UP" | "UP" | "NEUTRAL" | "DOWN" | "STRONG_DOWN" | "INSUFFICIENT_DATA"
    ));
    if snap.hma_label != "INSUFFICIENT_DATA" {
        assert!(snap.hma_value.is_finite() && snap.hma_value > 0.0);
        assert!(snap.hma_slope_pct.is_finite());
        assert_eq!(snap.period, 20);
        assert_eq!(snap.half_period, 10);
        assert!(snap.sqrt_period >= 4);
    }
}

// ── Research section ──

#[test]
fn ppo_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = PpoSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        fast_period: 12,
        slow_period: 26,
        signal_period: 9,
        ema_fast: 151.0,
        ema_slow: 148.0,
        ppo_value: 2.027,
        signal_value: 1.8,
        histogram: 0.227,
        last_close: 150.0,
        ppo_label: "BULL".into(),
        note: String::new(),
    };
    upsert_ppo(&conn, "TEST", &snap).unwrap();
    let got = get_ppo(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.ppo_label, "BULL");
    assert!((got.ppo_value - 2.027).abs() < 1e-9);
}

#[test]
fn ppo_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_ppo_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.ppo_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.ppo_label != "INSUFFICIENT_DATA" {
        assert!(snap.ppo_value.is_finite());
        assert!(snap.signal_value.is_finite());
        assert!(snap.ema_fast > 0.0);
        assert!(snap.ema_slow > 0.0);
        assert_eq!(snap.fast_period, 12);
        assert_eq!(snap.slow_period, 26);
        assert_eq!(snap.signal_period, 9);
    }
}

#[test]
fn dpo_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = DpoSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        period: 20,
        shift: 11,
        sma_value: 148.0,
        dpo_value: 2.5,
        dpo_pct: 1.69,
        last_close: 150.0,
        dpo_label: "BULL".into(),
        note: String::new(),
    };
    upsert_dpo(&conn, "TEST", &snap).unwrap();
    let got = get_dpo(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.dpo_label, "BULL");
    assert!((got.dpo_value - 2.5).abs() < 1e-9);
}

#[test]
fn dpo_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_dpo_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.dpo_label.as_str(),
        "PEAK_HIGH" | "BULL" | "NEUTRAL" | "BEAR" | "PEAK_LOW" | "INSUFFICIENT_DATA"
    ));
    if snap.dpo_label != "INSUFFICIENT_DATA" {
        assert!(snap.dpo_value.is_finite());
        assert!(snap.sma_value > 0.0);
        assert_eq!(snap.period, 20);
        assert_eq!(snap.shift, 11);
    }
}

#[test]
fn kst_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = KstSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        rcma1: 0.8,
        rcma2: 1.2,
        rcma3: 1.5,
        rcma4: 2.0,
        kst_value: 15.7,
        signal_value: 14.2,
        histogram: 1.5,
        last_close: 150.0,
        kst_label: "STRONG_BULL".into(),
        note: String::new(),
    };
    upsert_kst(&conn, "TEST", &snap).unwrap();
    let got = get_kst(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.kst_label, "STRONG_BULL");
    assert!((got.kst_value - 15.7).abs() < 1e-9);
}

#[test]
fn kst_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_kst_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.kst_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.kst_label != "INSUFFICIENT_DATA" {
        assert!(snap.kst_value.is_finite());
        assert!(snap.signal_value.is_finite());
        assert!(snap.rcma1.is_finite());
        assert!(snap.rcma4.is_finite());
    }
}

#[test]
fn ultosc_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = UltoscSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        period_short: 7,
        period_mid: 14,
        period_long: 28,
        avg_short: 0.55,
        avg_mid: 0.52,
        avg_long: 0.50,
        ultosc_value: 53.1,
        last_close: 150.0,
        ultosc_label: "BULL".into(),
        note: String::new(),
    };
    upsert_ultosc(&conn, "TEST", &snap).unwrap();
    let got = get_ultosc(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.ultosc_label, "BULL");
    assert!((got.ultosc_value - 53.1).abs() < 1e-9);
}

#[test]
fn ultosc_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_ultosc_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.ultosc_label.as_str(),
        "OVERBOUGHT" | "BULL" | "NEUTRAL" | "BEAR" | "OVERSOLD" | "INSUFFICIENT_DATA"
    ));
    if snap.ultosc_label != "INSUFFICIENT_DATA" {
        assert!(snap.ultosc_value.is_finite());
        assert!(snap.ultosc_value >= 0.0 && snap.ultosc_value <= 100.0);
        assert_eq!(snap.period_short, 7);
        assert_eq!(snap.period_mid, 14);
        assert_eq!(snap.period_long, 28);
    }
}

#[test]
fn willr_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = WillrSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        period: 14,
        highest_high: 155.0,
        lowest_low: 145.0,
        willr_value: -30.0,
        last_close: 152.0,
        willr_label: "BULL".into(),
        note: String::new(),
    };
    upsert_willr(&conn, "TEST", &snap).unwrap();
    let got = get_willr(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.willr_label, "BULL");
    assert!((got.willr_value - -30.0).abs() < 1e-9);
}

#[test]
fn willr_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_willr_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.willr_label.as_str(),
        "OVERBOUGHT" | "BULL" | "NEUTRAL" | "BEAR" | "OVERSOLD" | "INSUFFICIENT_DATA"
    ));
    if snap.willr_label != "INSUFFICIENT_DATA" {
        assert!(snap.willr_value.is_finite());
        assert!(snap.willr_value >= -100.0 && snap.willr_value <= 0.0);
        assert!(snap.highest_high >= snap.lowest_low);
        assert_eq!(snap.period, 14);
    }
}

// ── Research section ──

#[test]
fn mass_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MassSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        ema_period: 9,
        sum_period: 25,
        mass_value: 25.7,
        single_ratio: 1.03,
        last_close: 150.0,
        mass_label: "NEUTRAL".into(),
        note: String::new(),
    };
    upsert_mass(&conn, "TEST", &snap).unwrap();
    let got = get_mass(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.mass_label, "NEUTRAL");
    assert!((got.mass_value - 25.7).abs() < 1e-9);
}

#[test]
fn mass_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_mass_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.mass_label.as_str(),
        "REVERSAL_BULGE" | "WATCH" | "NEUTRAL" | "INSUFFICIENT_DATA"
    ));
    if snap.mass_label != "INSUFFICIENT_DATA" {
        assert!(snap.mass_value.is_finite() && snap.mass_value >= 0.0);
        assert!(snap.single_ratio.is_finite() && snap.single_ratio >= 0.0);
        assert_eq!(snap.ema_period, 9);
        assert_eq!(snap.sum_period, 25);
    }
}

#[test]
fn chaikosc_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = ChaikoscSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        fast_period: 3,
        slow_period: 10,
        ad_last: 1_250_000.0,
        ema_fast_ad: 1_260_000.0,
        ema_slow_ad: 1_230_000.0,
        chaikosc_value: 30_000.0,
        last_close: 150.0,
        chaikosc_label: "ACCUM".into(),
        note: String::new(),
    };
    upsert_chaikosc(&conn, "TEST", &snap).unwrap();
    let got = get_chaikosc(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.chaikosc_label, "ACCUM");
    assert!((got.chaikosc_value - 30_000.0).abs() < 1e-6);
}

#[test]
fn chaikosc_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_chaikosc_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.chaikosc_label.as_str(),
        "STRONG_ACCUM" | "ACCUM" | "NEUTRAL" | "DIST" | "STRONG_DIST" | "INSUFFICIENT_DATA"
    ));
    if snap.chaikosc_label != "INSUFFICIENT_DATA" {
        assert!(snap.chaikosc_value.is_finite());
        assert!(snap.ema_fast_ad.is_finite());
        assert!(snap.ema_slow_ad.is_finite());
        assert_eq!(snap.fast_period, 3);
        assert_eq!(snap.slow_period, 10);
    }
}

#[test]
fn klinger_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = KlingerSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        fast_period: 34,
        slow_period: 55,
        signal_period: 13,
        ema_fast_vf: 12_000.0,
        ema_slow_vf: 10_500.0,
        kvo_value: 1_500.0,
        signal_value: 1_200.0,
        histogram: 300.0,
        last_close: 150.0,
        klinger_label: "BULL".into(),
        note: String::new(),
    };
    upsert_klinger(&conn, "TEST", &snap).unwrap();
    let got = get_klinger(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.klinger_label, "BULL");
    assert!((got.kvo_value - 1_500.0).abs() < 1e-6);
}

#[test]
fn klinger_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_klinger_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.klinger_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.klinger_label != "INSUFFICIENT_DATA" {
        assert!(snap.kvo_value.is_finite());
        assert!(snap.signal_value.is_finite());
        assert_eq!(snap.fast_period, 34);
        assert_eq!(snap.slow_period, 55);
        assert_eq!(snap.signal_period, 13);
    }
}

#[test]
fn stochrsi_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = StochRsiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        rsi_period: 14,
        stoch_period: 14,
        k_period: 3,
        d_period: 3,
        rsi_value: 55.0,
        rsi_min: 40.0,
        rsi_max: 70.0,
        stoch_rsi_raw: 0.5,
        k_value: 55.0,
        d_value: 50.0,
        last_close: 150.0,
        stochrsi_label: "BULL".into(),
        note: String::new(),
    };
    upsert_stochrsi(&conn, "TEST", &snap).unwrap();
    let got = get_stochrsi(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.stochrsi_label, "BULL");
    assert!((got.k_value - 55.0).abs() < 1e-9);
}

#[test]
fn stochrsi_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_stochrsi_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.stochrsi_label.as_str(),
        "OVERBOUGHT" | "BULL" | "NEUTRAL" | "BEAR" | "OVERSOLD" | "INSUFFICIENT_DATA"
    ));
    if snap.stochrsi_label != "INSUFFICIENT_DATA" {
        assert!(snap.rsi_value.is_finite());
        assert!(snap.rsi_value >= 0.0 && snap.rsi_value <= 100.0);
        assert!(snap.k_value >= 0.0 && snap.k_value <= 100.0);
        assert!(snap.d_value >= 0.0 && snap.d_value <= 100.0);
        assert!(snap.rsi_max >= snap.rsi_min);
        assert_eq!(snap.rsi_period, 14);
        assert_eq!(snap.stoch_period, 14);
    }
}

#[test]
fn awesome_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = AwesomeSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        fast_period: 5,
        slow_period: 34,
        sma_fast: 150.5,
        sma_slow: 149.0,
        ao_value: 1.5,
        ao_prev: 1.2,
        ao_color_up: true,
        last_close: 150.0,
        awesome_label: "BULL".into(),
        note: String::new(),
    };
    upsert_awesome(&conn, "TEST", &snap).unwrap();
    let got = get_awesome(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.awesome_label, "BULL");
    assert!((got.ao_value - 1.5).abs() < 1e-9);
}

#[test]
fn awesome_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_awesome_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.awesome_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.awesome_label != "INSUFFICIENT_DATA" {
        assert!(snap.ao_value.is_finite());
        assert!(snap.sma_fast > 0.0);
        assert!(snap.sma_slow > 0.0);
        assert_eq!(snap.fast_period, 5);
        assert_eq!(snap.slow_period, 34);
    }
}

// ── Research section ──

#[test]
fn efi_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = EfiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        ema_period: 13,
        raw_efi: 25_000.0,
        efi_value: 18_500.0,
        efi_prev: 17_200.0,
        last_close: 150.0,
        efi_label: "BULL".into(),
        note: String::new(),
    };
    upsert_efi(&conn, "TEST", &snap).unwrap();
    let got = get_efi(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.efi_label, "BULL");
    assert!((got.efi_value - 18_500.0).abs() < 1e-6);
}

#[test]
fn efi_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_efi_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.efi_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.efi_label != "INSUFFICIENT_DATA" {
        assert!(snap.efi_value.is_finite());
        assert!(snap.raw_efi.is_finite());
        assert_eq!(snap.ema_period, 13);
    }
}

#[test]
fn emv_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = EmvSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        sma_period: 14,
        volume_scale: 100_000_000.0,
        raw_emv: 1.25,
        emv_value: 0.75,
        last_close: 150.0,
        emv_label: "BULL".into(),
        note: String::new(),
    };
    upsert_emv(&conn, "TEST", &snap).unwrap();
    let got = get_emv(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.emv_label, "BULL");
    assert!((got.emv_value - 0.75).abs() < 1e-9);
}

#[test]
fn emv_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_emv_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.emv_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.emv_label != "INSUFFICIENT_DATA" {
        assert!(snap.emv_value.is_finite());
        assert!(snap.raw_emv.is_finite());
        assert_eq!(snap.sma_period, 14);
        assert!(snap.volume_scale > 0.0);
    }
}

#[test]
fn nvi_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = NviSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        signal_period: 255,
        nvi_value: 1050.0,
        signal_value: 1020.0,
        last_close: 150.0,
        nvi_label: "BULL".into(),
        note: String::new(),
    };
    upsert_nvi(&conn, "TEST", &snap).unwrap();
    let got = get_nvi(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.nvi_label, "BULL");
    assert!((got.nvi_value - 1050.0).abs() < 1e-9);
}

#[test]
fn nvi_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_nvi_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.nvi_label.as_str(),
        "BULL" | "NEUTRAL" | "BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.nvi_label != "INSUFFICIENT_DATA" {
        assert!(snap.nvi_value.is_finite() && snap.nvi_value > 0.0);
        assert!(snap.signal_value.is_finite() && snap.signal_value > 0.0);
        assert!(snap.signal_period >= 3);
    }
}

#[test]
fn pvi_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = PviSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        signal_period: 255,
        pvi_value: 1120.0,
        signal_value: 1080.0,
        last_close: 150.0,
        pvi_label: "BULL".into(),
        note: String::new(),
    };
    upsert_pvi(&conn, "TEST", &snap).unwrap();
    let got = get_pvi(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.pvi_label, "BULL");
    assert!((got.pvi_value - 1120.0).abs() < 1e-9);
}

#[test]
fn pvi_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_pvi_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.pvi_label.as_str(),
        "BULL" | "NEUTRAL" | "BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.pvi_label != "INSUFFICIENT_DATA" {
        assert!(snap.pvi_value.is_finite() && snap.pvi_value > 0.0);
        assert!(snap.signal_value.is_finite() && snap.signal_value > 0.0);
        assert!(snap.signal_period >= 3);
    }
}

#[test]
fn coppock_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CoppockSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        roc_fast: 11,
        roc_slow: 14,
        wma_period: 10,
        coppock_value: 2.5,
        coppock_prev: 1.8,
        last_close: 150.0,
        coppock_label: "BULL".into(),
        note: String::new(),
    };
    upsert_coppock(&conn, "TEST", &snap).unwrap();
    let got = get_coppock(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.coppock_label, "BULL");
    assert!((got.coppock_value - 2.5).abs() < 1e-9);
}

#[test]
fn coppock_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_coppock_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.coppock_label.as_str(),
        "BUY_CROSS" | "SELL_CROSS" | "BULL" | "NEUTRAL" | "BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.coppock_label != "INSUFFICIENT_DATA" {
        assert!(snap.coppock_value.is_finite());
        assert!(snap.coppock_prev.is_finite());
        assert_eq!(snap.roc_fast, 11);
        assert_eq!(snap.roc_slow, 14);
        assert_eq!(snap.wma_period, 10);
    }
}

#[test]
fn cmo_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CmoSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        period: 9,
        sum_up: 12.5,
        sum_dn: 7.5,
        cmo_value: 25.0,
        last_close: 150.0,
        cmo_label: "BULL".into(),
        note: String::new(),
    };
    upsert_cmo(&conn, "TEST", &snap).unwrap();
    let got = get_cmo(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cmo_label, "BULL");
    assert!((got.cmo_value - 25.0).abs() < 1e-9);
}

#[test]
fn cmo_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_cmo_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.cmo_label.as_str(),
        "OVERBOUGHT" | "BULL" | "NEUTRAL" | "BEAR" | "OVERSOLD" | "INSUFFICIENT_DATA"
    ));
    if snap.cmo_label != "INSUFFICIENT_DATA" {
        assert!(snap.cmo_value.is_finite());
        assert!(snap.cmo_value >= -100.0 && snap.cmo_value <= 100.0);
        assert_eq!(snap.period, 9);
    }
}

#[test]
fn qstick_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = QstickSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        period: 14,
        qstick_value: 0.8,
        qstick_prev: 0.6,
        last_close: 150.0,
        qstick_label: "BULL".into(),
        note: String::new(),
    };
    upsert_qstick(&conn, "TEST", &snap).unwrap();
    let got = get_qstick(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.qstick_label, "BULL");
    assert!((got.qstick_value - 0.8).abs() < 1e-9);
}

#[test]
fn qstick_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_qstick_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.qstick_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.qstick_label != "INSUFFICIENT_DATA" {
        assert!(snap.qstick_value.is_finite());
        assert!(snap.qstick_prev.is_finite());
        assert_eq!(snap.period, 14);
    }
}

#[test]
fn disparity_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = DisparitySnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        period: 14,
        sma_value: 148.0,
        disparity_value: 1.35,
        last_close: 150.0,
        disparity_label: "BULL".into(),
        note: String::new(),
    };
    upsert_disparity(&conn, "TEST", &snap).unwrap();
    let got = get_disparity(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.disparity_label, "BULL");
    assert!((got.disparity_value - 1.35).abs() < 1e-9);
}

#[test]
fn disparity_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_disparity_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.disparity_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.disparity_label != "INSUFFICIENT_DATA" {
        assert!(snap.disparity_value.is_finite());
        assert!(snap.sma_value.is_finite() && snap.sma_value > 0.0);
        assert_eq!(snap.period, 14);
    }
}

#[test]
fn bop_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = BopSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        period: 14,
        raw_bop: 0.35,
        bop_value: 0.22,
        last_close: 150.0,
        bop_label: "BULL".into(),
        note: String::new(),
    };
    upsert_bop(&conn, "TEST", &snap).unwrap();
    let got = get_bop(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.bop_label, "BULL");
    assert!((got.bop_value - 0.22).abs() < 1e-9);
}

#[test]
fn bop_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_bop_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.bop_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.bop_label != "INSUFFICIENT_DATA" {
        assert!(snap.bop_value.is_finite());
        assert!(snap.bop_value >= -1.0 && snap.bop_value <= 1.0);
        assert!(snap.raw_bop.is_finite());
        assert_eq!(snap.period, 14);
    }
}

#[test]
fn schaff_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = SchaffSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 252,
        ema_fast: 23,
        ema_slow: 50,
        cycle: 10,
        stc_value: 62.5,
        stc_prev: 58.0,
        last_close: 150.0,
        schaff_label: "BULL".into(),
        note: String::new(),
    };
    upsert_schaff(&conn, "TEST", &snap).unwrap();
    let got = get_schaff(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.schaff_label, "BULL");
    assert!((got.stc_value - 62.5).abs() < 1e-9);
}

#[test]
fn schaff_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_schaff_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.schaff_label.as_str(),
        "OVERBOUGHT" | "BULL" | "NEUTRAL" | "BEAR" | "OVERSOLD" | "INSUFFICIENT_DATA"
    ));
    if snap.schaff_label != "INSUFFICIENT_DATA" {
        assert!(snap.stc_value.is_finite());
        assert!(snap.stc_value >= 0.0 && snap.stc_value <= 100.0);
        assert!(snap.stc_prev.is_finite());
        assert_eq!(snap.ema_fast, 23);
        assert_eq!(snap.ema_slow, 50);
        assert_eq!(snap.cycle, 10);
    }
}

// ── Research section ──

#[test]
fn stoch_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = StochSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 60,
        k_period: 14,
        d_period: 3,
        smoothing: 3,
        percent_k: 72.5,
        percent_d: 68.0,
        last_close: 150.0,
        stoch_label: "BULL".into(),
        note: String::new(),
    };
    upsert_stoch(&conn, "TEST", &snap).unwrap();
    let got = get_stoch(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.stoch_label, "BULL");
    assert!((got.percent_k - 72.5).abs() < 1e-9);
}

#[test]
fn stoch_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_stoch_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.stoch_label.as_str(),
        "OVERBOUGHT" | "BULL" | "NEUTRAL" | "BEAR" | "OVERSOLD" | "INSUFFICIENT_DATA"
    ));
    if snap.stoch_label != "INSUFFICIENT_DATA" {
        assert!(snap.percent_k >= 0.0 && snap.percent_k <= 100.0);
        assert!(snap.percent_d >= 0.0 && snap.percent_d <= 100.0);
        assert_eq!(snap.k_period, 14);
    }
}

#[test]
fn macd_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MacdSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 120,
        fast_period: 12,
        slow_period: 26,
        signal_period: 9,
        macd_value: 1.25,
        signal_value: 1.00,
        histogram: 0.25,
        histogram_prev: 0.10,
        last_close: 150.0,
        macd_label: "BULL".into(),
        note: String::new(),
    };
    upsert_macd(&conn, "TEST", &snap).unwrap();
    let got = get_macd(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.macd_label, "BULL");
    assert!((got.histogram - 0.25).abs() < 1e-9);
}

#[test]
fn macd_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_macd_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.macd_label.as_str(),
        "BULL_CROSS" | "BULL" | "NEUTRAL" | "BEAR" | "BEAR_CROSS" | "INSUFFICIENT_DATA"
    ));
    if snap.macd_label != "INSUFFICIENT_DATA" {
        assert!(snap.macd_value.is_finite());
        assert!(snap.signal_value.is_finite());
        assert!((snap.histogram - (snap.macd_value - snap.signal_value)).abs() < 1e-9);
    }
}

#[test]
fn vwap_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = VwapSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 60,
        window: 20,
        vwap_value: 148.5,
        last_close: 150.0,
        deviation_pct: 1.01,
        vwap_label: "ABOVE".into(),
        note: String::new(),
    };
    upsert_vwap(&conn, "TEST", &snap).unwrap();
    let got = get_vwap(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.vwap_label, "ABOVE");
    assert!((got.deviation_pct - 1.01).abs() < 1e-9);
}

#[test]
fn vwap_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_vwap_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.vwap_label.as_str(),
        "STRONG_ABOVE" | "ABOVE" | "AT" | "BELOW" | "STRONG_BELOW" | "INSUFFICIENT_DATA"
    ));
    if snap.vwap_label != "INSUFFICIENT_DATA" {
        assert!(snap.vwap_value > 0.0);
        assert!(snap.deviation_pct.is_finite());
        assert_eq!(snap.window, 20);
    }
}

#[test]
fn mcgd_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = McgdSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 60,
        length: 14,
        mcgd_value: 149.0,
        mcgd_prev: 148.5,
        last_close: 150.0,
        deviation_pct: 0.67,
        mcgd_label: "BULL".into(),
        note: String::new(),
    };
    upsert_mcgd(&conn, "TEST", &snap).unwrap();
    let got = get_mcgd(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.mcgd_label, "BULL");
    assert!((got.mcgd_value - 149.0).abs() < 1e-9);
}

#[test]
fn mcgd_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_mcgd_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.mcgd_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.mcgd_label != "INSUFFICIENT_DATA" {
        assert!(snap.mcgd_value.is_finite() && snap.mcgd_value > 0.0);
        assert!(snap.mcgd_prev.is_finite());
        assert_eq!(snap.length, 14);
    }
}

#[test]
fn rwi_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = RwiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-17".into(),
        bars_used: 60,
        length: 14,
        rwi_high: 1.8,
        rwi_low: 0.6,
        last_close: 150.0,
        rwi_label: "TRENDING_UP".into(),
        note: String::new(),
    };
    upsert_rwi(&conn, "TEST", &snap).unwrap();
    let got = get_rwi(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.rwi_label, "TRENDING_UP");
    assert!((got.rwi_high - 1.8).abs() < 1e-9);
}

#[test]
fn rwi_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_rwi_snapshot("T", "2026-04-17", &bars);
    assert!(matches!(
        snap.rwi_label.as_str(),
        "TRENDING_UP" | "TRENDING_DOWN" | "RANGE_BOUND" | "INSUFFICIENT_DATA"
    ));
    if snap.rwi_label != "INSUFFICIENT_DATA" {
        assert!(snap.rwi_high >= 0.0 && snap.rwi_high.is_finite());
        assert!(snap.rwi_low >= 0.0 && snap.rwi_low.is_finite());
        assert_eq!(snap.length, 14);
    }
}
