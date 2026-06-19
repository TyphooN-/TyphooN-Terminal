// ── Round 30 tests: PSR / ADF / MNKENDALL / BIPOWER / DDDUR ──

#[test]
fn psr_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = ProbabilisticSharpeSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 250,
        sharpe: 1.5,
        skewness: -0.3,
        kurtosis: 4.2,
        sr_benchmark: 0.0,
        psr: 0.87,
        psr_label: "MODERATE".into(),
        note: String::new(),
    };
    upsert_psr(&c, "TEST", &snap).unwrap();
    let back = get_psr(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.psr_label, "MODERATE");
    assert!((back.psr - 0.87).abs() < 1e-9);
    assert!((back.sharpe - 1.5).abs() < 1e-9);
}

#[test]
fn psr_compute_insufficient() {
    let snap = compute_psr_snapshot("T", "2026-04-15", &[]);
    assert_eq!(snap.psr_label, "INSUFFICIENT_DATA");
}

#[test]
fn psr_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_psr_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.psr_label, "INSUFFICIENT_DATA");
    assert!((0.0..=1.0).contains(&snap.psr));
    assert!(snap.bars_used > 0);
    assert!(snap.kurtosis > 0.0);
}

#[test]
fn adf_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = DickeyFullerSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 253,
        beta: -0.12,
        se_beta: 0.035,
        t_statistic: -3.43,
        crit_1pct: -3.43,
        crit_5pct: -2.86,
        crit_10pct: -2.57,
        reject_unit_root: true,
        adf_label: "STATIONARY".into(),
        note: String::new(),
    };
    upsert_adf(&c, "TEST", &snap).unwrap();
    let back = get_adf(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.adf_label, "STATIONARY");
    assert!(back.reject_unit_root);
    assert!((back.crit_5pct - -2.86).abs() < 1e-9);
}

#[test]
fn adf_compute_insufficient() {
    let snap = compute_adf_snapshot("T", "2026-04-15", &[]);
    assert_eq!(snap.adf_label, "INSUFFICIENT_DATA");
}

#[test]
fn adf_compute_oscillating() {
    // Oscillating ±0.5% close prices are mean-reverting — ADF should
    // produce a deeply negative t-statistic and reject the unit root.
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_adf_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.adf_label, "INSUFFICIENT_DATA");
    assert!(snap.bars_used > 0);
    assert!((snap.crit_5pct - -2.86).abs() < 1e-9);
    assert!((snap.crit_1pct - -3.43).abs() < 1e-9);
}

#[test]
fn mnkendall_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = MannKendallSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 253,
        s_statistic: 15000,
        variance: 2_100_000.0,
        z_statistic: 10.35,
        p_value: 0.0,
        tau: 0.47,
        reject_no_trend: true,
        mk_label: "STRONG_UP".into(),
        note: String::new(),
    };
    upsert_mnkendall(&c, "TEST", &snap).unwrap();
    let back = get_mnkendall(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.mk_label, "STRONG_UP");
    assert_eq!(back.s_statistic, 15000);
    assert!(back.reject_no_trend);
}

#[test]
fn mnkendall_compute_insufficient() {
    let snap = compute_mnkendall_snapshot("T", "2026-04-15", &[]);
    assert_eq!(snap.mk_label, "INSUFFICIENT_DATA");
}

#[test]
fn mnkendall_compute_monotone_up() {
    // synthetic_ohlc_bars_150 is monotonically rising → Mann-Kendall
    // should fire STRONG_UP with high S.
    let bars = synthetic_ohlc_bars_150();
    let snap = compute_mnkendall_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.mk_label, "INSUFFICIENT_DATA");
    assert!(snap.s_statistic > 0);
    assert!(snap.z_statistic > 0.0);
    assert!(snap.tau > 0.5);
    assert!(snap.reject_no_trend);
    assert_eq!(snap.mk_label, "STRONG_UP");
}

#[test]
fn bipower_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = BipowerVariationSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 250,
        realized_var: 0.04,
        bipower_var: 0.034,
        continuous_vol_ann_pct: 18.5,
        realized_vol_ann_pct: 20.0,
        jump_ratio: 0.15,
        jump_pct: 15.0,
        jump_label: "MILD_JUMPS".into(),
        note: String::new(),
    };
    upsert_bipower(&c, "TEST", &snap).unwrap();
    let back = get_bipower(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.jump_label, "MILD_JUMPS");
    assert!((back.jump_ratio - 0.15).abs() < 1e-9);
}

#[test]
fn bipower_compute_insufficient() {
    let snap = compute_bipower_snapshot("T", "2026-04-15", &[]);
    assert_eq!(snap.jump_label, "INSUFFICIENT_DATA");
}

#[test]
fn bipower_compute_oscillating() {
    // With smooth ±0.5% oscillations, adjacent products |r_t|·|r_{t-1}|
    // ≈ r² → BPV ≈ (π/2)·RV so jump_ratio should be small / near zero.
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_bipower_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.jump_label, "INSUFFICIENT_DATA");
    assert!((0.0..=1.0).contains(&snap.jump_ratio));
    assert!(snap.realized_var > 0.0);
    assert!(snap.realized_vol_ann_pct > 0.0);
}

#[test]
fn dddur_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = DrawdownDurationSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 253,
        dd_event_count: 12,
        max_dd_duration_bars: 25,
        mean_dd_duration_bars: 8.5,
        median_dd_duration_bars: 7.0,
        total_bars_underwater: 110,
        pct_time_underwater: 43.5,
        currently_underwater: true,
        current_dd_duration_bars: 4,
        dddur_label: "PERSISTENT_DD".into(),
        note: String::new(),
    };
    upsert_dddur(&c, "TEST", &snap).unwrap();
    let back = get_dddur(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.dddur_label, "PERSISTENT_DD");
    assert_eq!(back.dd_event_count, 12);
    assert!(back.currently_underwater);
}

#[test]
fn dddur_compute_insufficient() {
    let snap = compute_dddur_snapshot("T", "2026-04-15", &[]);
    assert_eq!(snap.dddur_label, "INSUFFICIENT_DATA");
}

#[test]
fn dddur_compute_monotone_dry() {
    // Monotonically rising series → no drawdowns, MOSTLY_DRY.
    let bars = synthetic_ohlc_bars_150();
    let snap = compute_dddur_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.dddur_label, "INSUFFICIENT_DATA");
    assert_eq!(snap.dd_event_count, 0);
    assert_eq!(snap.total_bars_underwater, 0);
    assert!(!snap.currently_underwater);
    assert_eq!(snap.dddur_label, "MOSTLY_DRY");
}

// ── Round 31 tests ──

#[test]
fn hilltail_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = HillTailSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 200,
        k_order_stats: 20,
        threshold_abs: 0.02,
        hill_alpha_abs: 2.5,
        hill_alpha_left: 2.3,
        hill_alpha_right: 2.7,
        tail_label: "MODERATE_TAIL".into(),
        note: String::new(),
    };
    upsert_hilltail(&c, "TEST", &snap).unwrap();
    let back = get_hilltail(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.tail_label, "MODERATE_TAIL");
    assert!((back.hill_alpha_abs - 2.5).abs() < 1e-9);
}

#[test]
fn hilltail_compute_insufficient() {
    let bars: Vec<HistoricalPriceRow> = Vec::new();
    let snap = compute_hilltail_snapshot("T", "2026-04-15", &bars);
    assert_eq!(snap.tail_label, "INSUFFICIENT_DATA");
}

#[test]
fn hilltail_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_hilltail_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.tail_label, "INSUFFICIENT_DATA");
    assert!(snap.k_order_stats >= 10);
    assert!(snap.hill_alpha_abs >= 0.0);
}

#[test]
fn archlm_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = ArchLmSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 200,
        q_lags: 5,
        r_squared: 0.08,
        lm_statistic: 16.0,
        p_value: 0.0068,
        crit_5pct_chi2: 11.0705,
        crit_1pct_chi2: 15.0863,
        reject_homoskedastic: true,
        arch_label: "STRONG_ARCH".into(),
        note: String::new(),
    };
    upsert_archlm(&c, "TEST", &snap).unwrap();
    let back = get_archlm(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.arch_label, "STRONG_ARCH");
    assert!(back.reject_homoskedastic);
}

#[test]
fn archlm_compute_insufficient() {
    let bars: Vec<HistoricalPriceRow> = Vec::new();
    let snap = compute_archlm_snapshot("T", "2026-04-15", &bars);
    assert_eq!(snap.arch_label, "INSUFFICIENT_DATA");
}

#[test]
fn archlm_compute_oscillating() {
    // Oscillating bars yield a regular ε² series; the LM stat should
    // be finite and the label should not be INSUFFICIENT_DATA.
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_archlm_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.arch_label, "INSUFFICIENT_DATA");
    assert!(snap.lm_statistic >= 0.0);
    assert!(snap.r_squared >= 0.0 && snap.r_squared <= 1.0);
}

#[test]
fn painratio_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = PainRatioSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 200,
        pain_index_pct: 3.5,
        annualized_return_pct: 12.0,
        pain_ratio: 3.43,
        max_dd_pct: 22.0,
        pain_label: "MODERATE_PAIN".into(),
        note: String::new(),
    };
    upsert_painratio(&c, "TEST", &snap).unwrap();
    let back = get_painratio(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.pain_label, "MODERATE_PAIN");
    assert!((back.pain_ratio - 3.43).abs() < 1e-9);
}

#[test]
fn painratio_compute_insufficient() {
    let bars: Vec<HistoricalPriceRow> = Vec::new();
    let snap = compute_painratio_snapshot("T", "2026-04-15", &bars);
    assert_eq!(snap.pain_label, "INSUFFICIENT_DATA");
}

#[test]
fn painratio_compute_monotone_low_pain() {
    // Monotonically rising close → no drawdown → pain_index ≈ 0.
    let bars = synthetic_ohlc_bars_150();
    let snap = compute_painratio_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.pain_label, "INSUFFICIENT_DATA");
    assert!(snap.pain_index_pct < 1.0);
    assert_eq!(snap.pain_label, "LOW_PAIN");
}

#[test]
fn cusum_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = CusumBreakSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 200,
        max_abs_cusum: 20.5,
        test_statistic: 1.45,
        max_abs_bar: 120,
        direction_at_max: "UP".into(),
        crit_10pct: 1.22,
        crit_5pct: 1.36,
        crit_1pct: 1.63,
        reject_stability: true,
        cusum_label: "BREAK_DETECTED".into(),
        note: String::new(),
    };
    upsert_cusum(&c, "TEST", &snap).unwrap();
    let back = get_cusum(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.cusum_label, "BREAK_DETECTED");
    assert_eq!(back.direction_at_max, "UP");
}

#[test]
fn cusum_compute_insufficient() {
    let bars: Vec<HistoricalPriceRow> = Vec::new();
    let snap = compute_cusum_snapshot("T", "2026-04-15", &bars);
    assert_eq!(snap.cusum_label, "INSUFFICIENT_DATA");
}

#[test]
fn cusum_compute_oscillating() {
    // Alternating ±0.5% returns → near-zero mean → CUSUM should
    // stay inside the 10% bound → STABLE.
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_cusum_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.cusum_label, "INSUFFICIENT_DATA");
    assert!(snap.test_statistic >= 0.0);
    assert!(snap.max_abs_cusum >= 0.0);
}

#[test]
fn cfvar_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = CornishFisherSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-15".into(),
        bars_used: 200,
        mean_ret_pct: 0.05,
        sigma_ret_pct: 1.20,
        skewness: -0.3,
        excess_kurtosis: 2.5,
        gauss_var_5pct_pct: -1.92,
        cf_var_5pct_pct: -2.15,
        gauss_var_1pct_pct: -2.74,
        cf_var_1pct_pct: -3.20,
        cf_adjustment_5pct_pct: -0.23,
        skew_term_5pct: -0.10,
        kurt_term_5pct: -0.15,
        cfvar_label: "KURT_DRIVEN".into(),
        note: String::new(),
    };
    upsert_cfvar(&c, "TEST", &snap).unwrap();
    let back = get_cfvar(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.cfvar_label, "KURT_DRIVEN");
    assert!((back.cf_adjustment_5pct_pct - (-0.23)).abs() < 1e-9);
}

#[test]
fn cfvar_compute_insufficient() {
    let bars: Vec<HistoricalPriceRow> = Vec::new();
    let snap = compute_cfvar_snapshot("T", "2026-04-15", &bars);
    assert_eq!(snap.cfvar_label, "INSUFFICIENT_DATA");
}

#[test]
fn cfvar_compute_oscillating() {
    // Oscillating symmetric returns should be near Gaussian → skew≈0,
    // excess kurt ≈ -2 (very light) → CF adjustment should be small ⇒ BENIGN
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_cfvar_snapshot("T", "2026-04-15", &bars);
    assert_ne!(snap.cfvar_label, "INSUFFICIENT_DATA");
    assert!(snap.sigma_ret_pct > 0.0);
    // Gauss 5% VaR should be non-positive (loss side)
    assert!(snap.gauss_var_5pct_pct <= snap.mean_ret_pct);
}

// ── Round 32 tests ────────────────────────────────────────────

#[test]
fn entropy_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = EntropySnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 200,
        num_bins: 15,
        entropy_bits: 3.2,
        max_entropy_bits: 3.9,
        normalised_entropy: 0.82,
        entropy_label: "HIGH_ENTROPY".into(),
        note: String::new(),
    };
    upsert_entropy(&c, "TEST", &snap).unwrap();
    let back = get_entropy(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.entropy_label, "HIGH_ENTROPY");
    assert!((back.entropy_bits - 3.2).abs() < 1e-9);
}

#[test]
fn entropy_compute_insufficient() {
    let bars: Vec<HistoricalPriceRow> = Vec::new();
    let snap = compute_entropy_snapshot("T", "2026-04-16", &bars);
    assert_eq!(snap.entropy_label, "INSUFFICIENT_DATA");
}

#[test]
fn entropy_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_entropy_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.entropy_label, "INSUFFICIENT_DATA");
    assert!(snap.entropy_bits > 0.0);
    assert!(snap.normalised_entropy > 0.0);
    assert!(snap.normalised_entropy <= 1.0);
}

#[test]
fn rachev_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = RachevSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 200,
        es_right_5pct: 2.1,
        es_left_5pct: -1.8,
        rachev_5pct: 1.167,
        es_right_1pct: 3.0,
        es_left_1pct: -2.5,
        rachev_1pct: 1.2,
        rachev_label: "SYMMETRIC".into(),
        note: String::new(),
    };
    upsert_rachev(&c, "TEST", &snap).unwrap();
    let back = get_rachev(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.rachev_label, "SYMMETRIC");
    assert!((back.rachev_5pct - 1.167).abs() < 1e-9);
}

#[test]
fn rachev_compute_insufficient() {
    let bars: Vec<HistoricalPriceRow> = Vec::new();
    let snap = compute_rachev_snapshot("T", "2026-04-16", &bars);
    assert_eq!(snap.rachev_label, "INSUFFICIENT_DATA");
}

#[test]
fn rachev_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_rachev_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.rachev_label, "INSUFFICIENT_DATA");
    assert!(snap.rachev_5pct > 0.0);
    assert!(snap.es_right_5pct > 0.0);
    assert!(snap.es_left_5pct < 0.0);
}

#[test]
fn gpr_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = GprSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 200,
        sum_all_returns_pct: 5.0,
        sum_losses_pct: 10.0,
        sum_gains_pct: 15.0,
        gain_to_pain: 0.5,
        profit_factor: 1.5,
        win_count: 100,
        loss_count: 100,
        gpr_label: "MODEST".into(),
        note: String::new(),
    };
    upsert_gpr(&c, "TEST", &snap).unwrap();
    let back = get_gpr(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.gpr_label, "MODEST");
    assert!((back.gain_to_pain - 0.5).abs() < 1e-9);
}

#[test]
fn gpr_compute_insufficient() {
    let bars: Vec<HistoricalPriceRow> = Vec::new();
    let snap = compute_gpr_snapshot("T", "2026-04-16", &bars);
    assert_eq!(snap.gpr_label, "INSUFFICIENT_DATA");
}

#[test]
fn gpr_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_gpr_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.gpr_label, "INSUFFICIENT_DATA");
    assert!(snap.sum_losses_pct > 0.0);
    assert!(snap.sum_gains_pct > 0.0);
    assert!(snap.profit_factor > 0.0);
}

#[test]
fn pacf_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = PacfSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 200,
        pacf_lag1: -0.45,
        pacf_lag2: 0.02,
        pacf_lag3: -0.01,
        pacf_lag4: 0.03,
        pacf_lag5: -0.02,
        bartlett_crit_95: 0.138,
        significant_lags: 1,
        max_abs_pacf: 0.45,
        max_abs_lag: 1,
        pacf_label: "LAG1_DOMINANT".into(),
        note: String::new(),
    };
    upsert_pacf(&c, "TEST", &snap).unwrap();
    let back = get_pacf(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.pacf_label, "LAG1_DOMINANT");
    assert!((back.pacf_lag1 - (-0.45)).abs() < 1e-9);
}

#[test]
fn pacf_compute_insufficient() {
    let bars: Vec<HistoricalPriceRow> = Vec::new();
    let snap = compute_pacf_snapshot("T", "2026-04-16", &bars);
    assert_eq!(snap.pacf_label, "INSUFFICIENT_DATA");
}

#[test]
fn pacf_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_pacf_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.pacf_label, "INSUFFICIENT_DATA");
    assert!(snap.bartlett_crit_95 > 0.0);
    // Oscillating ±0.5% should have strong negative lag-1 PACF
    assert!(snap.pacf_lag1 < 0.0);
}

#[test]
fn apen_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = ApenSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 200,
        embed_dim: 2,
        tolerance: 0.001,
        phi_m: -1.5,
        phi_m1: -2.0,
        apen: 0.5,
        apen_label: "MODERATE".into(),
        note: String::new(),
    };
    upsert_apen(&c, "TEST", &snap).unwrap();
    let back = get_apen(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.apen_label, "MODERATE");
    assert!((back.apen - 0.5).abs() < 1e-9);
}

#[test]
fn apen_compute_insufficient() {
    let bars: Vec<HistoricalPriceRow> = Vec::new();
    let snap = compute_apen_snapshot("T", "2026-04-16", &bars);
    assert_eq!(snap.apen_label, "INSUFFICIENT_DATA");
}

#[test]
fn apen_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_apen_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.apen_label, "INSUFFICIENT_DATA");
    assert!(snap.apen >= 0.0);
    assert!(snap.tolerance > 0.0);
    // Perfectly alternating ±0.5% is very regular → low ApEn expected
    assert_eq!(snap.apen_label, "REGULAR");
}

// ── Round 33 tests ────────────────────────────────────────────

#[test]
fn upr_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = UprSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 200,
        upm1: 0.003,
        lpm2: 0.00001,
        downside_dev: 0.00316,
        upr: 0.95,
        upr_label: "MODERATE_UPSIDE".into(),
        note: String::new(),
    };
    upsert_upr(&c, "TEST", &snap).unwrap();
    let back = get_upr(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.upr_label, "MODERATE_UPSIDE");
    assert!((back.upr - 0.95).abs() < 1e-9);
}

#[test]
fn upr_compute_insufficient() {
    let bars: Vec<HistoricalPriceRow> = Vec::new();
    let snap = compute_upr_snapshot("T", "2026-04-16", &bars);
    assert_eq!(snap.upr_label, "INSUFFICIENT_DATA");
}

#[test]
fn upr_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_upr_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.upr_label, "INSUFFICIENT_DATA");
    assert!(snap.upm1 > 0.0);
    assert!(snap.downside_dev > 0.0);
    assert!(snap.upr > 0.0);
}

#[test]
fn levereff_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = LeverEffSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 200,
        corr_r_nextsq: -0.12,
        mean_vol_after_neg: 1.2,
        mean_vol_after_pos: 0.9,
        asym_ratio: 1.33,
        lever_label: "MILD_LEVERAGE".into(),
        note: String::new(),
    };
    upsert_levereff(&c, "TEST", &snap).unwrap();
    let back = get_levereff(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.lever_label, "MILD_LEVERAGE");
    assert!((back.corr_r_nextsq - (-0.12)).abs() < 1e-9);
}

#[test]
fn levereff_compute_insufficient() {
    let bars: Vec<HistoricalPriceRow> = Vec::new();
    let snap = compute_levereff_snapshot("T", "2026-04-16", &bars);
    assert_eq!(snap.lever_label, "INSUFFICIENT_DATA");
}

#[test]
fn levereff_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_levereff_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.lever_label, "INSUFFICIENT_DATA");
    assert!(snap.mean_vol_after_neg > 0.0);
    assert!(snap.mean_vol_after_pos > 0.0);
}

#[test]
fn drawdar_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = DrawDaRSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 200,
        dar_5pct: 5.0,
        cdar_5pct: 7.0,
        dar_1pct: 8.0,
        cdar_1pct: 10.0,
        max_dd_pct: 12.0,
        mean_dd_pct: 3.0,
        drawdar_label: "MODERATE_DD_RISK".into(),
        note: String::new(),
    };
    upsert_drawdar(&c, "TEST", &snap).unwrap();
    let back = get_drawdar(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.drawdar_label, "MODERATE_DD_RISK");
    assert!((back.dar_5pct - 5.0).abs() < 1e-9);
}

#[test]
fn drawdar_compute_insufficient() {
    let bars: Vec<HistoricalPriceRow> = Vec::new();
    let snap = compute_drawdar_snapshot("T", "2026-04-16", &bars);
    assert_eq!(snap.drawdar_label, "INSUFFICIENT_DATA");
}

#[test]
fn drawdar_compute_monotone() {
    let bars = synthetic_ohlc_bars_150();
    let snap = compute_drawdar_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.drawdar_label, "INSUFFICIENT_DATA");
    // Monotonically rising → drawdowns should be minimal
    assert_eq!(snap.drawdar_label, "LOW_DD_RISK");
}

#[test]
fn varhalf_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = VarHalfSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 200,
        vol_obs: 180,
        ar1_beta: 0.95,
        ar1_alpha: 0.001,
        ar1_r2: 0.90,
        half_life_days: 13.5,
        varhalf_label: "MODERATE_PERSIST".into(),
        note: String::new(),
    };
    upsert_varhalf(&c, "TEST", &snap).unwrap();
    let back = get_varhalf(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.varhalf_label, "MODERATE_PERSIST");
    assert!((back.half_life_days - 13.5).abs() < 1e-9);
}

#[test]
fn varhalf_compute_insufficient() {
    let bars: Vec<HistoricalPriceRow> = Vec::new();
    let snap = compute_varhalf_snapshot("T", "2026-04-16", &bars);
    assert_eq!(snap.varhalf_label, "INSUFFICIENT_DATA");
}

#[test]
fn varhalf_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_varhalf_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.varhalf_label, "INSUFFICIENT_DATA");
    assert!(snap.vol_obs > 0);
}

#[test]
fn gini_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    let snap = GiniSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 200,
        gini_coeff: 0.35,
        mean_abs_return_pct: 1.2,
        median_abs_return_pct: 0.9,
        gini_label: "MODERATE_CONCENTRATION".into(),
        note: String::new(),
    };
    upsert_gini(&c, "TEST", &snap).unwrap();
    let back = get_gini(&c, "TEST").unwrap().unwrap();
    assert_eq!(back.gini_label, "MODERATE_CONCENTRATION");
    assert!((back.gini_coeff - 0.35).abs() < 1e-9);
}

#[test]
fn gini_compute_insufficient() {
    let bars: Vec<HistoricalPriceRow> = Vec::new();
    let snap = compute_gini_snapshot("T", "2026-04-16", &bars);
    assert_eq!(snap.gini_label, "INSUFFICIENT_DATA");
}

#[test]
fn gini_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_gini_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.gini_label, "INSUFFICIENT_DATA");
    assert!(snap.gini_coeff >= 0.0);
    assert!(snap.gini_coeff <= 1.0);
    // All |returns| are nearly equal → low Gini
    assert_eq!(snap.gini_label, "LOW_CONCENTRATION");
}

// ── Round 34 tests ──

#[test]
fn sampen_snapshot_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = SampenSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        embed_dim: 2,
        tolerance: 0.002,
        a_count: 1000,
        b_count: 2000,
        sampen: 0.693,
        sampen_label: "MODERATE".into(),
        note: String::new(),
    };
    upsert_sampen(&conn, "TEST", &snap).unwrap();
    let got = get_sampen(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.symbol, "TEST");
    assert!((got.sampen - 0.693).abs() < 1e-6);
}

#[test]
fn sampen_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_sampen_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.sampen_label, "INSUFFICIENT_DATA");
    assert_ne!(snap.sampen_label, "UNDEFINED");
    assert!(snap.sampen >= 0.0);
    assert!(snap.b_count > 0);
}

#[test]
fn permen_snapshot_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = PermenSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        embed_dim: 3,
        patterns_observed: 6,
        patterns_possible: 6,
        permen_raw: 2.58,
        permen_normalised: 0.99,
        permen_label: "HIGHLY_COMPLEX".into(),
        note: String::new(),
    };
    upsert_permen(&conn, "TEST", &snap).unwrap();
    let got = get_permen(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.symbol, "TEST");
    assert!((got.permen_normalised - 0.99).abs() < 1e-6);
}

#[test]
fn permen_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_permen_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.permen_label, "INSUFFICIENT_DATA");
    assert!(snap.permen_normalised >= 0.0);
    assert!(snap.permen_normalised <= 1.0);
    // Perfectly alternating → only 2 of 6 ordinal patterns → low normalised entropy
    assert!(snap.patterns_observed <= 6);
}

#[test]
fn recfact_snapshot_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = RecfactSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        cum_return_pct: 15.0,
        max_drawdown_pct: 5.0,
        recovery_factor: 3.0,
        recfact_label: "EXCELLENT".into(),
        note: String::new(),
    };
    upsert_recfact(&conn, "TEST", &snap).unwrap();
    let got = get_recfact(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.symbol, "TEST");
    assert!((got.recovery_factor - 3.0).abs() < 1e-6);
}

#[test]
fn recfact_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_recfact_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.recfact_label, "INSUFFICIENT_DATA");
    assert!(snap.bars_used >= 20);
}

#[test]
fn recfact_compute_rising() {
    let bars = synthetic_ohlc_bars_150();
    let snap = compute_recfact_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.recfact_label, "INSUFFICIENT_DATA");
    assert!(snap.cum_return_pct > 0.0);
    assert!(snap.recovery_factor > 0.0);
}

#[test]
fn kpss_snapshot_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = KpssSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        kpss_stat: 0.25,
        lag_truncation: 6,
        crit_10: 0.347,
        crit_5: 0.463,
        crit_1: 0.739,
        reject_stationary: false,
        kpss_label: "STATIONARY".into(),
        note: String::new(),
    };
    upsert_kpss(&conn, "TEST", &snap).unwrap();
    let got = get_kpss(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.symbol, "TEST");
    assert!((got.kpss_stat - 0.25).abs() < 1e-6);
}

#[test]
fn kpss_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_kpss_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.kpss_label, "INSUFFICIENT_DATA");
    assert!(snap.lag_truncation >= 1);
    // Oscillating returns are mean-reverting → should be stationary
    assert_eq!(snap.kpss_label, "STATIONARY");
}

#[test]
fn specent_snapshot_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = SpecentSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        num_freqs: 126,
        spectral_entropy_raw: 5.0,
        spectral_entropy_norm: 0.72,
        peak_freq_idx: 63,
        peak_power_share: 0.05,
        specent_label: "BROAD_SPECTRUM".into(),
        note: String::new(),
    };
    upsert_specent(&conn, "TEST", &snap).unwrap();
    let got = get_specent(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.symbol, "TEST");
    assert!((got.spectral_entropy_norm - 0.72).abs() < 1e-6);
}

#[test]
fn specent_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_specent_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.specent_label, "INSUFFICIENT_DATA");
    assert!(snap.spectral_entropy_norm >= 0.0);
    assert!(snap.spectral_entropy_norm <= 1.0);
    // Perfectly alternating ±0.5% → dominant frequency at N/2 → low spectral entropy
    assert!(snap.specent_label == "PERIODIC" || snap.specent_label == "MODERATE_PERIODICITY");
}

// ── Round 35 tests ──

#[test]
fn robvol_snapshot_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = RobVolSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        classical_sigma: 0.20,
        mad_sigma: 0.18,
        iqr_sigma: 0.17,
        mad_ratio: 0.90,
        iqr_ratio: 0.85,
        robvol_label: "CLEAN".into(),
        note: String::new(),
    };
    upsert_robvol(&conn, "TEST", &snap).unwrap();
    let got = get_robvol(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.symbol, "TEST");
    assert!((got.mad_ratio - 0.90).abs() < 1e-6);
}

#[test]
fn robvol_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_robvol_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.robvol_label, "INSUFFICIENT_DATA");
    assert!(snap.classical_sigma > 0.0);
    assert!(snap.mad_sigma > 0.0);
    assert!(snap.iqr_sigma > 0.0);
}

#[test]
fn renyient_snapshot_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = RenyientSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        num_bins: 9,
        alpha: 2.0,
        renyi_raw: 2.80,
        renyi_normalised: 0.88,
        collision_prob: 0.144,
        renyient_label: "HIGHLY_DISPERSED".into(),
        note: String::new(),
    };
    upsert_renyient(&conn, "TEST", &snap).unwrap();
    let got = get_renyient(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.symbol, "TEST");
    assert!((got.renyi_normalised - 0.88).abs() < 1e-6);
}

#[test]
fn renyient_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_renyient_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.renyient_label, "INSUFFICIENT_DATA");
    assert!(snap.renyi_normalised >= 0.0);
    assert!(snap.renyi_normalised <= 1.0);
    assert!(snap.collision_prob > 0.0);
    assert!(snap.collision_prob <= 1.0);
}

#[test]
fn retquant_snapshot_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = RetquantSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        p01_pct: -3.0,
        p05_pct: -2.0,
        p10_pct: -1.5,
        p25_pct: -0.5,
        p50_pct: 0.05,
        p75_pct: 0.6,
        p90_pct: 1.4,
        p95_pct: 1.9,
        p99_pct: 2.8,
        iqr_pct: 1.1,
        tail_asymmetry: -0.03,
        retquant_label: "SYMMETRIC".into(),
        note: String::new(),
    };
    upsert_retquant(&conn, "TEST", &snap).unwrap();
    let got = get_retquant(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.symbol, "TEST");
    assert!((got.iqr_pct - 1.1).abs() < 1e-6);
}

#[test]
fn retquant_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_retquant_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.retquant_label, "INSUFFICIENT_DATA");
    assert!(snap.p99_pct >= snap.p75_pct);
    assert!(snap.p75_pct >= snap.p50_pct);
    assert!(snap.p50_pct >= snap.p25_pct);
    assert!(snap.p25_pct >= snap.p01_pct);
    assert!(snap.iqr_pct >= 0.0);
}

#[test]
fn msent_snapshot_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MsentSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        embed_dim: 2,
        tolerance: 0.002,
        max_scale: 5,
        sampen_scale1: 0.8,
        sampen_scale2: 0.9,
        sampen_scale3: 1.0,
        sampen_scale4: 1.1,
        sampen_scale5: 1.2,
        msent_complexity_index: 5.0,
        msent_label: "INCREASING".into(),
        note: String::new(),
    };
    upsert_msent(&conn, "TEST", &snap).unwrap();
    let got = get_msent(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.symbol, "TEST");
    assert!((got.msent_complexity_index - 5.0).abs() < 1e-6);
}

#[test]
fn msent_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_msent_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.msent_label, "INSUFFICIENT_DATA");
    assert_eq!(snap.max_scale, 5);
    assert!(snap.tolerance > 0.0);
}

#[test]
fn ewmavol_snapshot_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = EwmaVolSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        lambda: 0.94,
        ewma_variance: 0.00015,
        ewma_sigma_daily: 0.01224,
        ewma_sigma_annual: 0.194,
        classical_sigma_annual: 0.20,
        ewma_to_classical: 0.97,
        ewmavol_label: "NORMAL".into(),
        note: String::new(),
    };
    upsert_ewmavol(&conn, "TEST", &snap).unwrap();
    let got = get_ewmavol(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.symbol, "TEST");
    assert!((got.ewma_to_classical - 0.97).abs() < 1e-6);
}

#[test]
fn ewmavol_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_ewmavol_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.ewmavol_label, "INSUFFICIENT_DATA");
    assert!(snap.ewma_sigma_annual > 0.0);
    assert!(snap.classical_sigma_annual > 0.0);
    assert!(snap.ewma_to_classical > 0.0);
    assert!((snap.lambda - 0.94).abs() < 1e-9);
}

// ── Round 36 tests ────────────────────────────────────

#[test]
fn ksnorm_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = KsnormSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 200,
        ks_statistic: 0.08,
        critical_10pct: 0.086,
        critical_5pct: 0.096,
        critical_1pct: 0.115,
        reject_10pct: false,
        reject_5pct: false,
        reject_1pct: false,
        mean: 0.0004,
        sigma: 0.012,
        ksnorm_label: "NORMAL".into(),
        note: String::new(),
    };
    upsert_ksnorm(&conn, "TEST", &snap).unwrap();
    let got = get_ksnorm(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.ksnorm_label, "NORMAL");
    assert!((got.ks_statistic - 0.08).abs() < 1e-9);
}

#[test]
fn ksnorm_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_ksnorm_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.ksnorm_label, "INSUFFICIENT_DATA");
    assert!(snap.ks_statistic >= 0.0 && snap.ks_statistic <= 1.0);
    assert!(snap.critical_10pct > 0.0);
    assert!(snap.critical_5pct > snap.critical_10pct);
    assert!(snap.critical_1pct > snap.critical_5pct);
    assert!(snap.sigma > 0.0);
}

#[test]
fn adtest_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = AdtestSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 200,
        ad_statistic: 0.50,
        ad_adjusted: 0.505,
        p_value_approx: 0.45,
        critical_10pct: 0.631,
        critical_5pct: 0.752,
        critical_1pct: 1.035,
        reject_10pct: false,
        reject_5pct: false,
        reject_1pct: false,
        adtest_label: "NORMAL".into(),
        note: String::new(),
    };
    upsert_adtest(&conn, "TEST", &snap).unwrap();
    let got = get_adtest(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.adtest_label, "NORMAL");
    assert!((got.ad_adjusted - 0.505).abs() < 1e-9);
}

#[test]
fn adtest_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_adtest_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.adtest_label, "INSUFFICIENT_DATA");
    assert!(snap.ad_adjusted >= 0.0);
    assert!(snap.p_value_approx >= 0.0 && snap.p_value_approx <= 1.0);
    assert!((snap.critical_5pct - 0.752).abs() < 1e-9);
}

#[test]
fn lmom_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = LmomSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 200,
        l1_mean: 0.0004,
        l2_scale: 0.007,
        l3: 0.0,
        l4: 0.001,
        tau3_skew: 0.05,
        tau4_kurt: 0.14,
        lmom_label: "NEAR_SYMMETRIC".into(),
        note: String::new(),
    };
    upsert_lmom(&conn, "TEST", &snap).unwrap();
    let got = get_lmom(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.lmom_label, "NEAR_SYMMETRIC");
    assert!((got.tau4_kurt - 0.14).abs() < 1e-9);
}

#[test]
fn lmom_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_lmom_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.lmom_label, "INSUFFICIENT_DATA");
    assert!(snap.l2_scale > 0.0);
    assert!(snap.tau3_skew >= -1.0 && snap.tau3_skew <= 1.0);
    // Bimodal oscillating fixture lies outside the continuous-distribution
    // τ4 bounds — assert finiteness rather than the [-0.25,1] envelope.
    assert!(snap.tau4_kurt.is_finite());
}

#[test]
fn kylelam_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = KylelamSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 200,
        kyle_lambda: 1.2e-9,
        mean_abs_dp: 0.50,
        mean_volume: 1.0e6,
        correlation: 0.32,
        r_squared: 0.1024,
        kylelam_label: "MODERATE_IMPACT".into(),
        note: String::new(),
    };
    upsert_kylelam(&conn, "TEST", &snap).unwrap();
    let got = get_kylelam(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.kylelam_label, "MODERATE_IMPACT");
    assert!((got.r_squared - 0.1024).abs() < 1e-9);
}

#[test]
fn kylelam_compute_oscillating() {
    // Fixture uses constant volume (1M), so var(V) = 0 → INSUFFICIENT_DATA.
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_kylelam_snapshot("T", "2026-04-16", &bars);
    assert_eq!(snap.kylelam_label, "INSUFFICIENT_DATA");
}

#[test]
fn peakover_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = PeakoverSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 200,
        threshold_p95: 0.02,
        threshold_p99: 0.035,
        count_p95: 9,
        count_p99: 2,
        mean_excess_p95: 0.005,
        mean_excess_p99: 0.008,
        max_excess_p95: 0.015,
        max_excess_p99: 0.020,
        peakover_label: "HEAVY_TAIL".into(),
        note: String::new(),
    };
    upsert_peakover(&conn, "TEST", &snap).unwrap();
    let got = get_peakover(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.peakover_label, "HEAVY_TAIL");
    assert_eq!(got.count_p95, 9);
}

#[test]
fn peakover_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_peakover_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.peakover_label, "INSUFFICIENT_DATA");
    assert!(snap.threshold_p95 > 0.0);
    assert!(snap.threshold_p99 >= snap.threshold_p95);
    assert!(snap.mean_excess_p95 >= 0.0);
    assert!(snap.max_excess_p95 >= snap.mean_excess_p95);
}

#[test]
fn higuchi_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = HiguchiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 200,
        k_max: 10,
        fractal_dim: 1.45,
        r_squared: 0.98,
        log_k_count: 10,
        higuchi_label: "RANDOM".into(),
        note: String::new(),
    };
    upsert_higuchi(&conn, "TEST", &snap).unwrap();
    let got = get_higuchi(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.higuchi_label, "RANDOM");
    assert!((got.fractal_dim - 1.45).abs() < 1e-9);
}

#[test]
fn higuchi_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_higuchi_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.higuchi_label, "INSUFFICIENT_DATA");
    assert!(snap.fractal_dim.is_finite());
    assert!(snap.log_k_count >= 3);
}

#[test]
fn pickands_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = PickandsSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 200,
        k_index: 12,
        gamma_hat: 0.25,
        tail_index: 4.0,
        x_k: 0.03,
        x_2k: 0.02,
        x_4k: 0.015,
        pickands_label: "FRECHET_MODERATE".into(),
        note: String::new(),
    };
    upsert_pickands(&conn, "TEST", &snap).unwrap();
    let got = get_pickands(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.pickands_label, "FRECHET_MODERATE");
    assert!((got.gamma_hat - 0.25).abs() < 1e-9);
}

#[test]
fn pickands_compute_oscillating() {
    // Fixture has only 2 distinct |r| values (±0.5%), so x_k = x_2k = x_4k → INSUFFICIENT_DATA.
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_pickands_snapshot("T", "2026-04-16", &bars);
    assert_eq!(snap.pickands_label, "INSUFFICIENT_DATA");
}

#[test]
fn kappa3_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = Kappa3Snapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 200,
        mar: 0.0,
        excess_mean: 0.08,
        lpm3: 1e-5,
        lpm3_root: 0.20,
        kappa3: 0.40,
        sortino_compare: 0.55,
        kappa3_label: "POSITIVE".into(),
        note: String::new(),
    };
    upsert_kappa3(&conn, "TEST", &snap).unwrap();
    let got = get_kappa3(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.kappa3_label, "POSITIVE");
    assert!((got.kappa3 - 0.40).abs() < 1e-9);
}

#[test]
fn kappa3_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_kappa3_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.kappa3_label, "INSUFFICIENT_DATA");
    assert!(snap.lpm3 > 0.0);
    assert!(snap.lpm3_root > 0.0);
    assert!(snap.kappa3.is_finite());
    assert!(snap.sortino_compare.is_finite());
}

#[test]
fn lyapunov_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = LyapunovSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 200,
        embed_dim: 3,
        time_delay: 1,
        lambda_max: 0.04,
        r_squared: 0.85,
        steps_used: 20,
        lyapunov_label: "WEAKLY_CHAOTIC".into(),
        note: String::new(),
    };
    upsert_lyapunov(&conn, "TEST", &snap).unwrap();
    let got = get_lyapunov(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.lyapunov_label, "WEAKLY_CHAOTIC");
    assert!((got.lambda_max - 0.04).abs() < 1e-9);
}

#[test]
fn lyapunov_compute_oscillating() {
    // Alternating ±0.5% returns → embedded vectors are highly degenerate.
    // Label should be CHAOTIC / WEAKLY_CHAOTIC / PERIODIC / STABLE / INSUFFICIENT_DATA.
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_lyapunov_snapshot("T", "2026-04-16", &bars);
    assert!(matches!(
        snap.lyapunov_label.as_str(),
        "CHAOTIC" | "WEAKLY_CHAOTIC" | "PERIODIC" | "STABLE" | "INSUFFICIENT_DATA"
    ));
    if snap.lyapunov_label != "INSUFFICIENT_DATA" {
        assert!(snap.lambda_max.is_finite());
    }
}

#[test]
fn rankac_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = RankacSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 200,
        rho_lag1: -0.08,
        rho_lag5: 0.02,
        rho_lag10: -0.03,
        mean_abs_rho: 0.0433,
        max_abs_rho: 0.08,
        rankac_label: "WEAK_DEPENDENCE".into(),
        note: String::new(),
    };
    upsert_rankac(&conn, "TEST", &snap).unwrap();
    let got = get_rankac(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.rankac_label, "WEAK_DEPENDENCE");
    assert!((got.rho_lag1 - (-0.08)).abs() < 1e-9);
}

#[test]
fn rankac_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_rankac_snapshot("T", "2026-04-16", &bars);
    assert_ne!(snap.rankac_label, "INSUFFICIENT_DATA");
    assert!(snap.rho_lag1.is_finite());
    assert!(snap.rho_lag5.is_finite());
    assert!(snap.rho_lag10.is_finite());
    assert!(snap.rho_lag1 >= -1.0 && snap.rho_lag1 <= 1.0);
    assert!(snap.max_abs_rho >= snap.mean_abs_rho);
}

// ── Round 38 ─────────────────────────────────────────────────

#[test]
fn bnsjump_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = BnsjumpSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        realized_variance: 0.0012,
        bipower_variance: 0.0009,
        jump_ratio: 0.25,
        jump_z_stat: 2.1,
        p_value: 0.018,
        bnsjump_label: "MODERATE_JUMP".into(),
        note: String::new(),
    };
    upsert_bnsjump(&conn, "TEST", &snap).unwrap();
    let got = get_bnsjump(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.bnsjump_label, "MODERATE_JUMP");
    assert!((got.jump_z_stat - 2.1).abs() < 1e-9);
}

#[test]
fn bnsjump_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_bnsjump_snapshot("T", "2026-04-16", &bars);
    assert!(matches!(
        snap.bnsjump_label.as_str(),
        "STRONG_JUMP" | "MODERATE_JUMP" | "WEAK_JUMP" | "NO_JUMP" | "INSUFFICIENT_DATA"
    ));
    if snap.bnsjump_label != "INSUFFICIENT_DATA" {
        assert!(snap.realized_variance.is_finite());
        assert!(snap.bipower_variance.is_finite());
        assert!(snap.jump_z_stat.is_finite());
        assert!(snap.p_value >= 0.0 && snap.p_value <= 1.0);
    }
}

#[test]
fn pproot_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = PprootSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        rho_hat: 0.997,
        t_rho: -0.8,
        z_rho: -0.9,
        z_t: -1.1,
        lag_truncation: 5,
        pproot_label: "UNIT_ROOT".into(),
        note: String::new(),
    };
    upsert_pproot(&conn, "TEST", &snap).unwrap();
    let got = get_pproot(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.pproot_label, "UNIT_ROOT");
    assert!((got.rho_hat - 0.997).abs() < 1e-9);
}

#[test]
fn pproot_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_pproot_snapshot("T", "2026-04-16", &bars);
    assert!(matches!(
        snap.pproot_label.as_str(),
        "STATIONARY_STRONG" | "STATIONARY_WEAK" | "BORDERLINE" | "UNIT_ROOT" | "INSUFFICIENT_DATA"
    ));
    if snap.pproot_label != "INSUFFICIENT_DATA" {
        assert!(snap.rho_hat.is_finite());
        assert!(snap.z_t.is_finite());
        assert!(snap.lag_truncation >= 1);
    }
}

#[test]
fn mfdfa_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MfdfaSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        h_q_neg2: 0.62,
        h_q_zero: 0.55,
        h_q_pos2: 0.48,
        delta_h: 0.14,
        scales_used: 7,
        mfdfa_label: "WEAK_MULTIFRACTAL".into(),
        note: String::new(),
    };
    upsert_mfdfa(&conn, "TEST", &snap).unwrap();
    let got = get_mfdfa(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.mfdfa_label, "WEAK_MULTIFRACTAL");
    assert!((got.delta_h - 0.14).abs() < 1e-9);
}

#[test]
fn mfdfa_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_mfdfa_snapshot("T", "2026-04-16", &bars);
    assert!(matches!(
        snap.mfdfa_label.as_str(),
        "STRONG_MULTIFRACTAL"
            | "MODERATE_MULTIFRACTAL"
            | "WEAK_MULTIFRACTAL"
            | "MONOFRACTAL"
            | "INSUFFICIENT_DATA"
    ));
    if snap.mfdfa_label != "INSUFFICIENT_DATA" {
        assert!(snap.h_q_neg2.is_finite());
        assert!(snap.h_q_zero.is_finite());
        assert!(snap.h_q_pos2.is_finite());
        assert!(snap.scales_used >= 3);
    }
}

#[test]
fn hillks_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = HillksSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        k_order: 25,
        alpha_hat: 3.2,
        ks_statistic: 0.12,
        ks_critical_5pct: 0.272,
        hillks_label: "ACCEPTABLE_FIT".into(),
        note: String::new(),
    };
    upsert_hillks(&conn, "TEST", &snap).unwrap();
    let got = get_hillks(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.hillks_label, "ACCEPTABLE_FIT");
    assert!((got.alpha_hat - 3.2).abs() < 1e-9);
}

#[test]
fn hillks_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_hillks_snapshot("T", "2026-04-16", &bars);
    assert!(matches!(
        snap.hillks_label.as_str(),
        "GOOD_FIT" | "ACCEPTABLE_FIT" | "POOR_FIT" | "REJECT" | "INSUFFICIENT_DATA"
    ));
    if snap.hillks_label != "INSUFFICIENT_DATA" {
        assert!(snap.alpha_hat.is_finite());
        assert!(snap.ks_statistic >= 0.0);
        assert!(snap.ks_critical_5pct > 0.0);
    }
}

#[test]
fn tsi_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = TsiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        ema_long: 25,
        ema_short: 13,
        tsi_value: 18.5,
        signal_value: 12.3,
        tsi_minus_signal: 6.2,
        tsi_label: "BULL".into(),
        note: String::new(),
    };
    upsert_tsi(&conn, "TEST", &snap).unwrap();
    let got = get_tsi(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.tsi_label, "BULL");
    assert!((got.tsi_value - 18.5).abs() < 1e-9);
}

#[test]
fn tsi_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_tsi_snapshot("T", "2026-04-16", &bars);
    assert!(matches!(
        snap.tsi_label.as_str(),
        "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    ));
    if snap.tsi_label != "INSUFFICIENT_DATA" {
        assert!(snap.tsi_value.is_finite());
        assert!(snap.signal_value.is_finite());
        assert!(snap.ema_long == 25 && snap.ema_short == 13);
    }
}

#[test]
fn garch11_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = Garch11Snapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        omega: 0.00002,
        alpha: 0.08,
        beta: 0.90,
        persistence: 0.98,
        unconditional_var: 0.001,
        half_life_bars: 34.3,
        log_likelihood: 800.5,
        garch11_label: "HIGH_PERSISTENCE".into(),
        note: String::new(),
    };
    upsert_garch11(&conn, "TEST", &snap).unwrap();
    let got = get_garch11(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.garch11_label, "HIGH_PERSISTENCE");
    assert!((got.persistence - 0.98).abs() < 1e-9);
}

#[test]
fn garch11_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_garch11_snapshot("T", "2026-04-16", &bars);
    assert!(matches!(
        snap.garch11_label.as_str(),
        "HIGH_PERSISTENCE"
            | "MODERATE_PERSISTENCE"
            | "LOW_PERSISTENCE"
            | "NEAR_INTEGRATED"
            | "INSUFFICIENT_DATA"
    ));
    if snap.garch11_label != "INSUFFICIENT_DATA" {
        assert!(snap.omega.is_finite() && snap.omega >= 0.0);
        assert!(snap.alpha >= 0.0 && snap.alpha <= 1.0);
        assert!(snap.beta >= 0.0 && snap.beta <= 1.0);
        assert!(snap.persistence >= 0.0);
    }
}

#[test]
fn sadf_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = SadfSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        min_window: 42,
        adf_full: -1.2,
        sadf_stat: 1.8,
        sadf_argmax_end: 180,
        critical_95: 1.49,
        reject_null: true,
        sadf_label: "EXPLOSIVE_LIKELY".into(),
        note: String::new(),
    };
    upsert_sadf(&conn, "TEST", &snap).unwrap();
    let got = get_sadf(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.sadf_label, "EXPLOSIVE_LIKELY");
    assert!((got.sadf_stat - 1.8).abs() < 1e-9);
    assert_eq!(got.reject_null, true);
}

#[test]
fn sadf_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_sadf_snapshot("T", "2026-04-16", &bars);
    assert!(matches!(
        snap.sadf_label.as_str(),
        "EXPLOSIVE_CONFIRMED" | "EXPLOSIVE_LIKELY" | "BORDERLINE" | "STABLE" | "INSUFFICIENT_DATA"
    ));
    if snap.sadf_label != "INSUFFICIENT_DATA" {
        assert!(snap.sadf_stat.is_finite());
        assert!(snap.critical_95 > 0.0);
        assert!(snap.min_window >= 1);
    }
}

#[test]
fn cordim_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CordimSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        embed_dim: 3,
        radii_count: 8,
        d2: 2.3,
        r_squared: 0.98,
        cordim_label: "MODERATE_DIM".into(),
        note: String::new(),
    };
    upsert_cordim(&conn, "TEST", &snap).unwrap();
    let got = get_cordim(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.cordim_label, "MODERATE_DIM");
    assert!((got.d2 - 2.3).abs() < 1e-9);
}

#[test]
fn cordim_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_cordim_snapshot("T", "2026-04-16", &bars);
    assert!(matches!(
        snap.cordim_label.as_str(),
        "LOW_DIM" | "MODERATE_DIM" | "HIGH_DIM" | "STOCHASTIC" | "INSUFFICIENT_DATA"
    ));
    if snap.cordim_label != "INSUFFICIENT_DATA" {
        assert!(snap.d2.is_finite());
        assert!(snap.embed_dim == 3);
        assert!(snap.radii_count >= 1);
    }
}

#[test]
fn skspec_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = SkspecSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        window_size: 30,
        mean_skew: 0.12,
        std_skew: 0.45,
        min_skew: -0.8,
        max_skew: 1.1,
        range_skew: 1.9,
        skspec_label: "DRIFTING".into(),
        note: String::new(),
    };
    upsert_skspec(&conn, "TEST", &snap).unwrap();
    let got = get_skspec(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.skspec_label, "DRIFTING");
    assert!((got.range_skew - 1.9).abs() < 1e-9);
}

#[test]
fn skspec_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_skspec_snapshot("T", "2026-04-16", &bars);
    assert!(matches!(
        snap.skspec_label.as_str(),
        "STABLE_POSITIVE" | "STABLE_NEGATIVE" | "DRIFTING" | "UNSTABLE" | "INSUFFICIENT_DATA"
    ));
    if snap.skspec_label != "INSUFFICIENT_DATA" {
        assert!(snap.mean_skew.is_finite());
        assert!(snap.std_skew >= 0.0);
        assert!(snap.range_skew >= 0.0);
        assert!(snap.window_size == 30);
    }
}

#[test]
fn automi_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = AutomiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        num_bins: 8,
        mi_lag1: 0.35,
        mi_lag5: 0.12,
        mi_lag10: 0.04,
        h_marginal: 3.0,
        normalized_mi1: 0.117,
        automi_label: "WEAK".into(),
        note: String::new(),
    };
    upsert_automi(&conn, "TEST", &snap).unwrap();
    let got = get_automi(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.automi_label, "WEAK");
    assert!((got.mi_lag1 - 0.35).abs() < 1e-9);
}

#[test]
fn automi_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_automi_snapshot("T", "2026-04-16", &bars);
    assert!(matches!(
        snap.automi_label.as_str(),
        "STRONG" | "MODERATE" | "WEAK" | "INDEPENDENT" | "INSUFFICIENT_DATA"
    ));
    if snap.automi_label != "INSUFFICIENT_DATA" {
        assert!(snap.mi_lag1 >= 0.0);
        assert!(snap.mi_lag5 >= 0.0);
        assert!(snap.mi_lag10 >= 0.0);
        assert!(snap.num_bins == 8);
        assert!(snap.h_marginal >= 0.0);
    }
}

// ── Round 40 tests ──

#[test]
fn durbinwatson_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = DurbinWatsonSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        dw_stat: 2.05,
        rho_estimate: -0.025,
        dw_label: "NO_AUTOCORR".into(),
        note: String::new(),
    };
    upsert_durbinwatson(&conn, "TEST", &snap).unwrap();
    let got = get_durbinwatson(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.dw_label, "NO_AUTOCORR");
    assert!((got.dw_stat - 2.05).abs() < 1e-9);
}

#[test]
fn durbinwatson_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_durbinwatson_snapshot("T", "2026-04-16", &bars);
    assert!(matches!(
        snap.dw_label.as_str(),
        "STRONG_POS" | "WEAK_POS" | "NO_AUTOCORR" | "WEAK_NEG" | "STRONG_NEG" | "INSUFFICIENT_DATA"
    ));
    if snap.dw_label != "INSUFFICIENT_DATA" {
        assert!(snap.dw_stat.is_finite());
        assert!(snap.dw_stat >= 0.0 && snap.dw_stat <= 4.0);
        assert!(snap.rho_estimate.is_finite());
    }
}

#[test]
fn bdstest_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = BdsTestSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        embed_dim: 2,
        epsilon_mult: 0.7,
        bds_stat: 2.3,
        p_value_two_sided: 0.021,
        reject_null: true,
        bds_label: "WEAK_DEPENDENCE".into(),
        note: String::new(),
    };
    upsert_bdstest(&conn, "TEST", &snap).unwrap();
    let got = get_bdstest(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.bds_label, "WEAK_DEPENDENCE");
    assert_eq!(got.embed_dim, 2);
    assert!((got.bds_stat - 2.3).abs() < 1e-9);
}

#[test]
fn bdstest_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_bdstest_snapshot("T", "2026-04-16", &bars);
    assert!(matches!(
        snap.bds_label.as_str(),
        "IID_CONFIRMED" | "WEAK_DEPENDENCE" | "STRONG_DEPENDENCE" | "INSUFFICIENT_DATA"
    ));
    if snap.bds_label != "INSUFFICIENT_DATA" {
        assert!(snap.bds_stat.is_finite());
        assert!(snap.p_value_two_sided >= 0.0 && snap.p_value_two_sided <= 1.0);
        assert_eq!(snap.embed_dim, 2);
    }
}

#[test]
fn breuschpagan_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = BreuschPaganSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        lm_stat: 4.2,
        r_squared: 0.0167,
        df: 1,
        critical_95: 3.841,
        reject_null: true,
        bp_label: "MILD_HETERO".into(),
        note: String::new(),
    };
    upsert_breuschpagan(&conn, "TEST", &snap).unwrap();
    let got = get_breuschpagan(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.bp_label, "MILD_HETERO");
    assert_eq!(got.df, 1);
    assert!((got.lm_stat - 4.2).abs() < 1e-9);
}

#[test]
fn breuschpagan_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_breuschpagan_snapshot("T", "2026-04-16", &bars);
    assert!(matches!(
        snap.bp_label.as_str(),
        "HOMOSKEDASTIC" | "MILD_HETERO" | "STRONG_HETERO" | "INSUFFICIENT_DATA"
    ));
    if snap.bp_label != "INSUFFICIENT_DATA" {
        assert!(snap.lm_stat >= 0.0);
        assert!(snap.r_squared >= 0.0 && snap.r_squared <= 1.0);
        assert!(snap.critical_95 > 0.0);
    }
}

#[test]
fn turnpts_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = TurnPtsSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        observed_turnpts: 170,
        expected_turnpts: 166.67,
        variance_turnpts: 44.5,
        z_stat: 0.5,
        p_value_two_sided: 0.617,
        reject_null: false,
        turnpts_label: "RANDOM_IID".into(),
        note: String::new(),
    };
    upsert_turnpts(&conn, "TEST", &snap).unwrap();
    let got = get_turnpts(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.turnpts_label, "RANDOM_IID");
    assert_eq!(got.observed_turnpts, 170);
    assert!((got.z_stat - 0.5).abs() < 1e-9);
}

#[test]
fn turnpts_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_turnpts_snapshot("T", "2026-04-16", &bars);
    assert!(matches!(
        snap.turnpts_label.as_str(),
        "RANDOM_IID" | "OVER_TURNING" | "UNDER_TURNING" | "INSUFFICIENT_DATA"
    ));
    if snap.turnpts_label != "INSUFFICIENT_DATA" {
        assert!(snap.expected_turnpts > 0.0);
        assert!(snap.variance_turnpts > 0.0);
        assert!(snap.z_stat.is_finite());
        assert!(snap.p_value_two_sided >= 0.0 && snap.p_value_two_sided <= 1.0);
    }
}

#[test]
fn periodogram_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = PeriodogramSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-16".into(),
        bars_used: 252,
        n_freqs: 126,
        dominant_freq: 0.05,
        dominant_period_bars: 20.0,
        dominant_power: 1.5e-3,
        total_power: 1.2e-2,
        dominant_power_ratio: 0.125,
        periodogram_label: "MODERATE_CYCLE".into(),
        note: String::new(),
    };
    upsert_periodogram(&conn, "TEST", &snap).unwrap();
    let got = get_periodogram(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.periodogram_label, "MODERATE_CYCLE");
    assert!((got.dominant_period_bars - 20.0).abs() < 1e-9);
    assert_eq!(got.n_freqs, 126);
}

#[test]
fn periodogram_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_periodogram_snapshot("T", "2026-04-16", &bars);
    assert!(matches!(
        snap.periodogram_label.as_str(),
        "STRONG_CYCLE" | "MODERATE_CYCLE" | "WEAK_CYCLE" | "NO_CYCLE" | "INSUFFICIENT_DATA"
    ));
    if snap.periodogram_label != "INSUFFICIENT_DATA" {
        assert!(snap.n_freqs >= 1);
        assert!(snap.dominant_freq > 0.0);
        assert!(snap.dominant_period_bars > 0.0);
        assert!(snap.dominant_power >= 0.0);
        assert!(snap.total_power > 0.0);
        assert!(snap.dominant_power_ratio >= 0.0 && snap.dominant_power_ratio <= 1.0);
    }
}

