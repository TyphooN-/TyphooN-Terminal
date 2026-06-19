// ── Round 76 (Quant Stats) tests ──

#[test]
fn modsharpe_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = ModSharpeSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-19".into(),
        bars_used: 252,
        annualization_factor: 252.0,
        mean_return_bar: 0.0008,
        stdev_return_bar: 0.012,
        skewness: -0.35,
        excess_kurtosis: 3.2,
        sharpe_ratio: 1.06,
        adjusted_sharpe: 0.78,
        adjustment_factor: 0.736,
        modsharpe_label: "MODERATE_POS".into(),
        note: String::new(),
    };
    upsert_modsharpe(&conn, "TEST", &snap).unwrap();
    let got = get_modsharpe(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.modsharpe_label, "MODERATE_POS");
    assert!((got.sharpe_ratio - 1.06).abs() < 1e-9);
    assert!((got.adjusted_sharpe - 0.78).abs() < 1e-9);
}

#[test]
fn modsharpe_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_modsharpe_snapshot("T", "2026-04-19", &bars);
    assert!(matches!(
        snap.modsharpe_label.as_str(),
        "STRONG_POS"
            | "MODERATE_POS"
            | "WEAK"
            | "MODERATE_NEG"
            | "STRONG_NEG"
            | "INSUFFICIENT_DATA"
    ));
    if snap.modsharpe_label != "INSUFFICIENT_DATA" {
        assert!(snap.annualization_factor > 0.0);
        assert!(snap.stdev_return_bar > 0.0);
        assert!(snap.sharpe_ratio.is_finite());
        assert!(snap.adjusted_sharpe.is_finite());
        assert!(snap.skewness.is_finite());
        assert!(snap.excess_kurtosis.is_finite());
    }
}

#[test]
fn hsiehtest_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = HsiehTestSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-19".into(),
        bars_used: 252,
        ar_order: 1,
        t_11: 0.08,
        t_22: -0.03,
        z_11: 1.62,
        z_22: -0.6,
        max_abs_z: 1.62,
        critical_95: 1.96,
        reject_null: false,
        hsieh_label: "LINEAR".into(),
        note: String::new(),
    };
    upsert_hsiehtest(&conn, "TEST", &snap).unwrap();
    let got = get_hsiehtest(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.hsieh_label, "LINEAR");
    assert!((got.max_abs_z - 1.62).abs() < 1e-9);
    assert_eq!(got.ar_order, 1);
}

#[test]
fn hsiehtest_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_hsieh_snapshot("T", "2026-04-19", &bars);
    assert!(matches!(
        snap.hsieh_label.as_str(),
        "LINEAR" | "MILD_NONLIN" | "STRONG_NONLIN" | "INSUFFICIENT_DATA"
    ));
    if snap.hsieh_label != "INSUFFICIENT_DATA" {
        assert!(snap.critical_95 > 0.0);
        assert!(snap.t_11.is_finite());
        assert!(snap.t_22.is_finite());
        assert!(snap.z_11.is_finite());
        assert!(snap.z_22.is_finite());
        assert!(snap.max_abs_z >= 0.0);
        assert_eq!(snap.ar_order, 1);
    }
}

#[test]
fn chowbreak_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = ChowBreakSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-19".into(),
        bars_used: 200,
        break_point_idx: 100,
        rss_pooled: 0.028,
        rss_unrestricted: 0.024,
        mean_pre: 0.001,
        mean_post: -0.0005,
        k_regressors: 1,
        f_stat: 8.3,
        df_num: 1,
        df_den: 198,
        critical_95: 3.84,
        reject_null: true,
        chowbreak_label: "MILD_BREAK".into(),
        note: String::new(),
    };
    upsert_chowbreak(&conn, "TEST", &snap).unwrap();
    let got = get_chowbreak(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.chowbreak_label, "MILD_BREAK");
    assert!((got.f_stat - 8.3).abs() < 1e-9);
    assert_eq!(got.break_point_idx, 100);
}

#[test]
fn chowbreak_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_chowbreak_snapshot("T", "2026-04-19", &bars);
    assert!(matches!(
        snap.chowbreak_label.as_str(),
        "NO_BREAK" | "MILD_BREAK" | "STRONG_BREAK" | "INSUFFICIENT_DATA"
    ));
    if snap.chowbreak_label != "INSUFFICIENT_DATA" {
        assert!(snap.f_stat.is_finite() && snap.f_stat >= 0.0);
        assert!(snap.rss_pooled >= snap.rss_unrestricted - 1e-12);
        assert!(snap.critical_95 > 0.0);
        assert_eq!(snap.break_point_idx, snap.bars_used / 2);
        assert_eq!(snap.k_regressors, 1);
        assert!(snap.df_den > 0);
    }
}

#[test]
fn driftburst_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = DriftBurstSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-19".into(),
        bars_used: 252,
        kernel_bandwidth_bars: 10.0,
        max_abs_statistic: 4.2,
        max_stat_signed: 4.2,
        max_at_offset: 15,
        excursions_gt_3: 3,
        critical_99_approx: 3.0,
        driftburst_label: "MILD_BURST".into(),
        note: String::new(),
    };
    upsert_driftburst(&conn, "TEST", &snap).unwrap();
    let got = get_driftburst(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.driftburst_label, "MILD_BURST");
    assert!((got.max_abs_statistic - 4.2).abs() < 1e-9);
    assert_eq!(got.excursions_gt_3, 3);
}

#[test]
fn driftburst_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_driftburst_snapshot("T", "2026-04-19", &bars);
    assert!(matches!(
        snap.driftburst_label.as_str(),
        "NO_BURST" | "MILD_BURST" | "STRONG_BURST" | "INSUFFICIENT_DATA"
    ));
    if snap.driftburst_label != "INSUFFICIENT_DATA" {
        assert!(snap.max_abs_statistic >= 0.0);
        assert!(snap.max_abs_statistic.is_finite());
        assert!(snap.max_stat_signed.is_finite());
        assert!(snap.critical_99_approx > 0.0);
        assert!(snap.kernel_bandwidth_bars > 0.0);
        assert!(snap.max_at_offset < snap.bars_used);
    }
}

#[test]
fn hlvclust_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = HlvClustSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-19".into(),
        bars_used: 252,
        lag_h: 10,
        parkinson_vol_bar: 0.014,
        parkinson_vol_annualised: 0.22,
        ac_lag1: 0.18,
        ac_lag5: 0.07,
        lb_q_stat: 35.2,
        critical_95: 18.307,
        p_value: 0.00011,
        reject_null: true,
        hlvclust_label: "STRONG_CLUST".into(),
        note: String::new(),
    };
    upsert_hlvclust(&conn, "TEST", &snap).unwrap();
    let got = get_hlvclust(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.hlvclust_label, "STRONG_CLUST");
    assert!((got.lb_q_stat - 35.2).abs() < 1e-9);
    assert_eq!(got.lag_h, 10);
}

#[test]
fn hlvclust_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_hlvclust_snapshot("T", "2026-04-19", &bars);
    assert!(matches!(
        snap.hlvclust_label.as_str(),
        "NO_CLUST" | "MILD_CLUST" | "STRONG_CLUST" | "INSUFFICIENT_DATA"
    ));
    if snap.hlvclust_label != "INSUFFICIENT_DATA" {
        assert!(snap.lb_q_stat >= 0.0 && snap.lb_q_stat.is_finite());
        assert!(snap.critical_95 > 0.0);
        assert!(snap.parkinson_vol_bar >= 0.0);
        assert!(snap.parkinson_vol_annualised >= 0.0);
        assert!(snap.ac_lag1.is_finite());
        assert!(snap.ac_lag5.is_finite());
        assert!(snap.p_value >= 0.0 && snap.p_value <= 1.0);
        assert_eq!(snap.lag_h, 10);
    }
}

// ── Round 77 (Quant Stats) tests ──

#[test]
fn yangzhang_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = YangZhangVolSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-19".into(),
        bars_used: 200,
        overnight_var: 0.00008,
        open_to_close_var: 0.00015,
        rs_component: 0.00012,
        k_weight: 0.1618,
        yz_vol_bar: 0.0152,
        yz_vol_annualised_pct: 24.2,
        cc_vol_annualised_pct: 22.1,
        efficiency_vs_close: 0.91,
        yangzhang_label: "MODERATE".into(),
        note: String::new(),
    };
    upsert_yangzhang(&conn, "TEST", &snap).unwrap();
    let got = get_yangzhang(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.yangzhang_label, "MODERATE");
    assert!((got.yz_vol_annualised_pct - 24.2).abs() < 1e-9);
    assert!((got.k_weight - 0.1618).abs() < 1e-9);
}

#[test]
fn yangzhang_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_yangzhang_snapshot("T", "2026-04-19", &bars);
    assert!(matches!(
        snap.yangzhang_label.as_str(),
        "VERY_LOW" | "LOW" | "MODERATE" | "HIGH" | "VERY_HIGH" | "INSUFFICIENT_DATA"
    ));
    if snap.yangzhang_label != "INSUFFICIENT_DATA" {
        assert!(snap.yz_vol_bar >= 0.0);
        assert!(snap.yz_vol_annualised_pct >= 0.0);
        assert!(snap.k_weight > 0.0 && snap.k_weight < 1.0);
        assert!(snap.overnight_var.is_finite());
        assert!(snap.open_to_close_var >= 0.0);
        assert!(snap.cc_vol_annualised_pct >= 0.0);
    }
}

#[test]
fn kuiper_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = KuiperSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-19".into(),
        bars_used: 252,
        mean: 0.0005,
        stdev: 0.013,
        d_plus: 0.056,
        d_minus: 0.042,
        v_stat: 0.098,
        v_stat_adj: 1.62,
        critical_95: 1.747,
        p_value_approx: 0.118,
        reject_null: false,
        kuiper_label: "NORMAL".into(),
        note: String::new(),
    };
    upsert_kuiper(&conn, "TEST", &snap).unwrap();
    let got = get_kuiper(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.kuiper_label, "NORMAL");
    assert!((got.v_stat_adj - 1.62).abs() < 1e-9);
    assert!((got.critical_95 - 1.747).abs() < 1e-9);
}

#[test]
fn kuiper_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_kuiper_snapshot("T", "2026-04-19", &bars);
    assert!(matches!(
        snap.kuiper_label.as_str(),
        "NORMAL" | "MILD_DEPART" | "STRONG_DEPART" | "INSUFFICIENT_DATA"
    ));
    if snap.kuiper_label != "INSUFFICIENT_DATA" {
        assert!(snap.d_plus >= 0.0);
        assert!(snap.d_minus >= 0.0);
        assert!(snap.v_stat >= 0.0);
        assert!(snap.v_stat_adj >= snap.v_stat);
        assert!(snap.p_value_approx >= 0.0 && snap.p_value_approx <= 1.0);
        assert!((snap.critical_95 - 1.747).abs() < 1e-9);
        assert!(snap.stdev > 0.0);
    }
}

#[test]
fn dagostino_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = DagostinoSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-19".into(),
        bars_used: 252,
        skewness: -0.4,
        excess_kurtosis: 2.8,
        z_skew: -2.1,
        z_kurt: 3.3,
        k2_stat: 15.3,
        critical_95: 5.991,
        p_value: 0.00047,
        reject_null: true,
        dagostino_label: "BOTH_DEPART".into(),
        note: String::new(),
    };
    upsert_dagostino(&conn, "TEST", &snap).unwrap();
    let got = get_dagostino(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.dagostino_label, "BOTH_DEPART");
    assert!((got.k2_stat - 15.3).abs() < 1e-9);
    assert!((got.critical_95 - 5.991).abs() < 1e-9);
}

#[test]
fn dagostino_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_dagostino_snapshot("T", "2026-04-19", &bars);
    assert!(matches!(
        snap.dagostino_label.as_str(),
        "NORMAL" | "SKEW_DOMINANT" | "KURT_DOMINANT" | "BOTH_DEPART" | "INSUFFICIENT_DATA"
    ));
    if snap.dagostino_label != "INSUFFICIENT_DATA" {
        assert!(snap.skewness.is_finite());
        assert!(snap.excess_kurtosis.is_finite());
        assert!(snap.z_skew.is_finite());
        assert!(snap.z_kurt.is_finite());
        assert!(snap.k2_stat >= 0.0 && snap.k2_stat.is_finite());
        assert!(snap.p_value >= 0.0 && snap.p_value <= 1.0);
        assert!((snap.critical_95 - 5.991).abs() < 1e-9);
    }
}

#[test]
fn baiperron_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = BaiPerronSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-19".into(),
        bars_used: 200,
        trim_fraction: 0.15,
        search_lo: 30,
        search_hi: 170,
        best_break_idx: 95,
        sup_f_stat: 12.4,
        mean_pre: 0.0012,
        mean_post: -0.0005,
        rss_no_break: 0.028,
        rss_at_best: 0.024,
        critical_95: 8.58,
        p_value_approx: 0.00043,
        reject_null: true,
        baiperron_label: "MILD_BREAK".into(),
        note: String::new(),
    };
    upsert_baiperron(&conn, "TEST", &snap).unwrap();
    let got = get_baiperron(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.baiperron_label, "MILD_BREAK");
    assert!((got.sup_f_stat - 12.4).abs() < 1e-9);
    assert_eq!(got.best_break_idx, 95);
}

#[test]
fn baiperron_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_baiperron_snapshot("T", "2026-04-19", &bars);
    assert!(matches!(
        snap.baiperron_label.as_str(),
        "NO_BREAK" | "MILD_BREAK" | "STRONG_BREAK" | "INSUFFICIENT_DATA"
    ));
    if snap.baiperron_label != "INSUFFICIENT_DATA" {
        assert!(snap.sup_f_stat >= 0.0 && snap.sup_f_stat.is_finite());
        assert!(snap.rss_no_break >= snap.rss_at_best - 1e-12);
        assert!(snap.search_lo < snap.search_hi);
        assert!(snap.best_break_idx >= snap.search_lo);
        assert!(snap.best_break_idx <= snap.search_hi);
        assert!((snap.trim_fraction - 0.15).abs() < 1e-9);
        assert!((snap.critical_95 - 8.58).abs() < 1e-9);
    }
}

#[test]
fn kupiecpof_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = KupiecPofSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-19".into(),
        bars_used: 252,
        confidence_level: 0.95,
        nominal_exceedance_rate: 0.05,
        rolling_window: 60,
        test_window: 192,
        var_latest_bar: 0.021,
        n_exceedances: 14,
        expected_exceedances: 9.6,
        realised_exceedance_rate: 0.0729,
        lr_pof_stat: 1.8,
        critical_95: 3.841,
        p_value: 0.18,
        reject_null: false,
        kupiec_label: "GOOD_FIT".into(),
        note: String::new(),
    };
    upsert_kupiecpof(&conn, "TEST", &snap).unwrap();
    let got = get_kupiecpof(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.kupiec_label, "GOOD_FIT");
    assert!((got.lr_pof_stat - 1.8).abs() < 1e-9);
    assert_eq!(got.n_exceedances, 14);
}

#[test]
fn kupiecpof_compute_oscillating() {
    let bars = synthetic_oscillating_bars_150();
    let snap = compute_kupiecpof_snapshot("T", "2026-04-19", &bars);
    assert!(matches!(
        snap.kupiec_label.as_str(),
        "GOOD_FIT" | "OVER_ESTIMATED" | "UNDER_ESTIMATED" | "INSUFFICIENT_DATA"
    ));
    if snap.kupiec_label != "INSUFFICIENT_DATA" {
        assert!((snap.confidence_level - 0.95).abs() < 1e-9);
        assert!((snap.nominal_exceedance_rate - 0.05).abs() < 1e-9);
        assert_eq!(snap.rolling_window, 60);
        assert_eq!(snap.test_window, snap.bars_used - snap.rolling_window);
        assert!(snap.lr_pof_stat >= 0.0 && snap.lr_pof_stat.is_finite());
        assert!(snap.p_value >= 0.0 && snap.p_value <= 1.0);
        assert!((snap.critical_95 - 3.841).abs() < 1e-9);
        assert!(snap.n_exceedances <= snap.test_window);
    }
}

#[test]
fn momrank_multi_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = MomentumRankMultiSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-21".into(),
        sector: "Technology".into(),
        peers_considered: 8,
        peers_with_data: 6,
        ret_1m_pct: 4.2,
        ret_3m_pct: 9.1,
        ret_6m_pct: 18.3,
        ret_ytd_pct: 11.2,
        ret_1y_pct: 27.5,
        pct_1m: 75.0,
        pct_3m: 82.5,
        pct_6m: 91.0,
        pct_ytd: 79.0,
        pct_1y: 88.0,
        composite_percentile: 84.7,
        horizons_above_median: 5,
        rank_position: 2,
        rank_label: "TOP_QUARTILE".into(),
        note: String::new(),
    };
    upsert_momrank_multi(&conn, "TEST", &snap).unwrap();
    let got = get_momrank_multi(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.rank_label, "TOP_QUARTILE");
    assert_eq!(got.rank_position, 2);
    assert!((got.composite_percentile - 84.7).abs() < 1e-9);
}

#[test]
fn compute_momrank_multi_top_decile() {
    let subject = PricePerformanceSnapshot {
        symbol: "AAA".into(),
        as_of: "2026-04-21".into(),
        bars_used: 260,
        latest_close: 120.0,
        ret_1m_pct: 8.0,
        ret_3m_pct: 18.0,
        ret_6m_pct: 32.0,
        ret_ytd_pct: 20.0,
        ret_1y_pct: 41.0,
        trend_label: "STRONG_BULL".into(),
        note: String::new(),
    };
    let peers = vec![
        (
            "BBB".into(),
            Some(PricePerformanceSnapshot {
                symbol: "BBB".into(),
                as_of: "2026-04-21".into(),
                bars_used: 260,
                latest_close: 90.0,
                ret_1m_pct: 3.0,
                ret_3m_pct: 7.0,
                ret_6m_pct: 12.0,
                ret_ytd_pct: 9.0,
                ret_1y_pct: 18.0,
                trend_label: "BULL".into(),
                note: String::new(),
            }),
        ),
        (
            "CCC".into(),
            Some(PricePerformanceSnapshot {
                symbol: "CCC".into(),
                as_of: "2026-04-21".into(),
                bars_used: 260,
                latest_close: 80.0,
                ret_1m_pct: -1.0,
                ret_3m_pct: 2.0,
                ret_6m_pct: 6.0,
                ret_ytd_pct: 4.0,
                ret_1y_pct: 9.0,
                trend_label: "NEUTRAL".into(),
                note: String::new(),
            }),
        ),
        (
            "DDD".into(),
            Some(PricePerformanceSnapshot {
                symbol: "DDD".into(),
                as_of: "2026-04-21".into(),
                bars_used: 260,
                latest_close: 75.0,
                ret_1m_pct: 2.5,
                ret_3m_pct: 6.0,
                ret_6m_pct: 9.5,
                ret_ytd_pct: 8.0,
                ret_1y_pct: 14.0,
                trend_label: "BULL".into(),
                note: String::new(),
            }),
        ),
        (
            "EEE".into(),
            Some(PricePerformanceSnapshot {
                symbol: "EEE".into(),
                as_of: "2026-04-21".into(),
                bars_used: 260,
                latest_close: 140.0,
                ret_1m_pct: 1.0,
                ret_3m_pct: 5.0,
                ret_6m_pct: 8.0,
                ret_ytd_pct: 3.5,
                ret_1y_pct: 12.0,
                trend_label: "NEUTRAL".into(),
                note: String::new(),
            }),
        ),
    ];
    let snap =
        compute_momrank_multi_snapshot("AAA", "2026-04-21", "Technology", Some(&subject), &peers);
    assert_eq!(snap.rank_label, "TOP_DECILE");
    assert_eq!(snap.rank_position, 1);
    assert_eq!(snap.horizons_above_median, 5);
    assert!(snap.composite_percentile >= 90.0);
}

#[test]
fn corrstk_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CorrStkSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-21".into(),
        symbol_sector: "Technology".into(),
        market_benchmark: "SPY".into(),
        sector_benchmark: "XLK".into(),
        overlaps_spy_20d: 20,
        overlaps_spy_60d: 60,
        overlaps_spy_252d: 252,
        overlaps_sector_20d: 20,
        overlaps_sector_60d: 60,
        overlaps_sector_252d: 252,
        corr_spy_20d: 0.84,
        corr_spy_60d: 0.79,
        corr_spy_252d: 0.73,
        beta_spy_252d: 1.12,
        r_squared_spy_252d: 0.53,
        corr_sector_20d: 0.91,
        corr_sector_60d: 0.86,
        corr_sector_252d: 0.82,
        beta_sector_252d: 1.05,
        r_squared_sector_252d: 0.67,
        dominant_benchmark: "XLK".into(),
        correlation_label: "SECTOR_LOCKSTEP".into(),
        note: String::new(),
    };
    upsert_corrstk(&conn, "TEST", &snap).unwrap();
    let got = get_corrstk(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.correlation_label, "SECTOR_LOCKSTEP");
    assert_eq!(got.dominant_benchmark, "XLK");
    assert!((got.corr_sector_252d - 0.82).abs() < 1e-9);
}

#[test]
fn compute_corrstk_index_lockstep() {
    let mut subject = Vec::new();
    let mut spy = Vec::new();
    let mut xlk = Vec::new();
    let base = chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let mut s = 100.0;
    let mut m = 400.0;
    let mut k = 200.0;
    for i in 0..320 {
        let d = base + chrono::Days::new(i as u64);
        let ret = 0.001 + ((i % 9) as f64 - 4.0) * 0.0015;
        let ret_sector = 0.0008 + ((i % 11) as f64 - 5.0) * 0.0010;
        s *= 1.0 + ret * 1.1;
        m *= 1.0 + ret;
        k *= 1.0 + ret_sector;
        let row = |close: f64, date: chrono::NaiveDate| HistoricalPriceRow {
            date: date.format("%Y-%m-%d").to_string(),
            open: close * 0.99,
            high: close * 1.01,
            low: close * 0.98,
            close,
            adj_close: close,
            volume: 1_000_000.0,
            change: 0.0,
            change_pct: 0.0,
        };
        subject.push(row(s, d));
        spy.push(row(m, d));
        xlk.push(row(k, d));
    }
    let snap = compute_corrstk_snapshot(
        "AAA",
        "2026-04-21",
        "Technology",
        "SPY",
        &subject,
        &spy,
        Some("XLK"),
        &xlk,
    );
    assert!(matches!(
        snap.correlation_label.as_str(),
        "INDEX_LOCKSTEP" | "MIXED"
    ));
    assert!(snap.overlaps_spy_252d >= 20);
    assert!(snap.corr_spy_252d > 0.65);
    assert!(snap.beta_spy_252d.is_finite());
}

#[test]
fn tlrank_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = ThirtyDayLiquidityRankSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-21".into(),
        sector: "Technology".into(),
        window_days: 30,
        bars_used: 30,
        avg_30d_dollar_volume: 245_000_000.0,
        tier_label: "LIQUID".into(),
        peers_considered: 7,
        peers_with_data: 5,
        sector_median_dollar_volume: 180_000_000.0,
        sector_p25_dollar_volume: 90_000_000.0,
        sector_p75_dollar_volume: 260_000_000.0,
        percentile_rank: 82.5,
        rank_position: 2,
        rank_label: "TOP_QUARTILE".into(),
        note: String::new(),
    };
    upsert_tlrank(&conn, "TEST", &snap).unwrap();
    let got = get_tlrank(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.rank_label, "TOP_QUARTILE");
    assert_eq!(got.rank_position, 2);
    assert!((got.avg_30d_dollar_volume - 245_000_000.0).abs() < 1e-9);
}

#[test]
fn compute_tlrank_top_decile() {
    let mk_bars = |start: chrono::NaiveDate, close: f64, volume: f64| -> Vec<HistoricalPriceRow> {
        (0..30)
            .map(|i| {
                let d = start + chrono::Days::new(i as u64);
                let px = close + i as f64 * 0.25;
                HistoricalPriceRow {
                    date: d.format("%Y-%m-%d").to_string(),
                    open: px * 0.99,
                    high: px * 1.01,
                    low: px * 0.98,
                    close: px,
                    adj_close: px,
                    volume,
                    change: 0.0,
                    change_pct: 0.0,
                }
            })
            .collect()
    };
    let base = chrono::NaiveDate::from_ymd_opt(2026, 1, 2).unwrap();
    let subject = mk_bars(base, 120.0, 3_000_000.0);
    let peers = vec![
        ("BBB".into(), mk_bars(base, 45.0, 900_000.0)),
        ("CCC".into(), mk_bars(base, 75.0, 1_100_000.0)),
        ("DDD".into(), mk_bars(base, 32.0, 700_000.0)),
        ("EEE".into(), mk_bars(base, 55.0, 850_000.0)),
    ];
    let snap = compute_tlrank_snapshot("AAA", "2026-04-21", "Technology", &subject, &peers);
    assert_eq!(snap.rank_label, "TOP_DECILE");
    assert_eq!(snap.rank_position, 1);
    assert_eq!(snap.bars_used, 30);
    assert!(snap.avg_30d_dollar_volume > snap.sector_p75_dollar_volume);
}

#[test]
fn corrrank_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = CorrelationRankSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-21".into(),
        sector: "Technology".into(),
        benchmark_name: "XLK".into(),
        benchmark_kind: "SECTOR_ETF".into(),
        subject_corr_252d: 0.88,
        subject_abs_corr_252d: 0.88,
        subject_beta_252d: 1.08,
        subject_r_squared_252d: 0.77,
        subject_correlation_label: "SECTOR_LOCKSTEP".into(),
        peers_considered: 6,
        peers_with_data: 4,
        sector_median_abs_corr_252d: 0.74,
        sector_p25_abs_corr_252d: 0.61,
        sector_p75_abs_corr_252d: 0.86,
        percentile_rank: 90.0,
        rank_position: 1,
        rank_label: "TOP_DECILE".into(),
        note: String::new(),
    };
    upsert_corrrank(&conn, "TEST", &snap).unwrap();
    let got = get_corrrank(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.rank_label, "TOP_DECILE");
    assert_eq!(got.benchmark_name, "XLK");
    assert!((got.subject_abs_corr_252d - 0.88).abs() < 1e-9);
}

#[test]
fn compute_corrrank_top_decile() {
    let subject = CorrStkSnapshot {
        symbol: "AAA".into(),
        as_of: "2026-04-21".into(),
        symbol_sector: "Technology".into(),
        market_benchmark: "SPY".into(),
        sector_benchmark: "XLK".into(),
        overlaps_spy_20d: 20,
        overlaps_spy_60d: 60,
        overlaps_spy_252d: 252,
        overlaps_sector_20d: 20,
        overlaps_sector_60d: 60,
        overlaps_sector_252d: 252,
        corr_spy_20d: 0.71,
        corr_spy_60d: 0.76,
        corr_spy_252d: 0.79,
        beta_spy_252d: 1.10,
        r_squared_spy_252d: 0.62,
        corr_sector_20d: 0.88,
        corr_sector_60d: 0.90,
        corr_sector_252d: 0.92,
        beta_sector_252d: 1.05,
        r_squared_sector_252d: 0.81,
        dominant_benchmark: "XLK".into(),
        correlation_label: "SECTOR_LOCKSTEP".into(),
        note: String::new(),
    };
    let mk_peer = |sym: &str, corr: f64| CorrStkSnapshot {
        symbol: sym.into(),
        as_of: "2026-04-21".into(),
        symbol_sector: "Technology".into(),
        market_benchmark: "SPY".into(),
        sector_benchmark: "XLK".into(),
        overlaps_spy_20d: 20,
        overlaps_spy_60d: 60,
        overlaps_spy_252d: 252,
        overlaps_sector_20d: 20,
        overlaps_sector_60d: 60,
        overlaps_sector_252d: 252,
        corr_spy_20d: corr - 0.10,
        corr_spy_60d: corr - 0.08,
        corr_spy_252d: corr - 0.06,
        beta_spy_252d: 1.0,
        r_squared_spy_252d: 0.5,
        corr_sector_20d: corr - 0.03,
        corr_sector_60d: corr - 0.02,
        corr_sector_252d: corr,
        beta_sector_252d: 1.0,
        r_squared_sector_252d: corr * corr,
        dominant_benchmark: "XLK".into(),
        correlation_label: "MIXED".into(),
        note: String::new(),
    };
    let peers_owned = [
        mk_peer("BBB", 0.84),
        mk_peer("CCC", 0.76),
        mk_peer("DDD", 0.63),
        mk_peer("EEE", 0.55),
    ];
    let peers: Vec<&CorrStkSnapshot> = peers_owned.iter().collect();
    let snap = compute_corrrank_snapshot("AAA", "2026-04-21", "Technology", Some(&subject), &peers);
    assert_eq!(snap.benchmark_kind, "SECTOR_ETF");
    assert_eq!(snap.benchmark_name, "XLK");
    assert_eq!(snap.rank_label, "TOP_DECILE");
    assert_eq!(snap.rank_position, 1);
    assert!(snap.subject_abs_corr_252d > snap.sector_p75_abs_corr_252d);
}

#[test]
fn operank_delta_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = OperatingMarginDeltaRankSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-21".into(),
        sector: "Technology".into(),
        basis: "quarterly".into(),
        latest_period: "2025-12-31".into(),
        operating_margin_pct: 24.0,
        operating_margin_change_pct: 5.5,
        operating_trend_label: "EXPANDING".into(),
        peers_considered: 7,
        peers_with_data: 5,
        sector_median_change_pct: 1.4,
        sector_p25_change_pct: -0.5,
        sector_p75_change_pct: 3.2,
        percentile_rank: 92.0,
        rank_position: 1,
        rank_label: "TOP_DECILE".into(),
        note: String::new(),
    };
    upsert_operank_delta(&conn, "TEST", &snap).unwrap();
    let got = get_operank_delta(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.rank_label, "TOP_DECILE");
    assert_eq!(got.latest_period, "2025-12-31");
    assert!((got.operating_margin_change_pct - 5.5).abs() < 1e-9);
}

#[test]
fn compute_operank_delta_top_decile() {
    let subject = MarginsSnapshot {
        symbol: "AAA".into(),
        as_of: "2026-04-21".into(),
        basis: "quarterly".into(),
        latest_period: "2025-12-31".into(),
        latest_operating_margin_pct: 22.0,
        prior_operating_margin_pct: 15.0,
        operating_margin_change_pct: 7.0,
        periods_used: 4,
        operating_trend_label: "EXPANDING".into(),
        ..Default::default()
    };
    let peers_owned = [
        MarginsSnapshot {
            symbol: "BBB".into(),
            operating_margin_change_pct: 2.0,
            periods_used: 4,
            ..Default::default()
        },
        MarginsSnapshot {
            symbol: "CCC".into(),
            operating_margin_change_pct: 1.0,
            periods_used: 4,
            ..Default::default()
        },
        MarginsSnapshot {
            symbol: "DDD".into(),
            operating_margin_change_pct: -1.0,
            periods_used: 4,
            ..Default::default()
        },
        MarginsSnapshot {
            symbol: "EEE".into(),
            operating_margin_change_pct: 3.5,
            periods_used: 4,
            ..Default::default()
        },
    ];
    let peers: Vec<&MarginsSnapshot> = peers_owned.iter().collect();
    let snap =
        compute_operank_delta_snapshot("AAA", "2026-04-21", "Technology", Some(&subject), &peers);
    assert_eq!(snap.rank_label, "TOP_DECILE");
    assert_eq!(snap.rank_position, 1);
    assert!(snap.operating_margin_change_pct > snap.sector_p75_change_pct);
}

#[test]
fn divacc_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = DividendAccelerationSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-21".into(),
        total_payments: 12,
        years_covered: 4,
        latest_year: 2025,
        latest_annual_dividend: 2.4,
        latest_yoy_growth_pct: 20.0,
        prior_yoy_growth_pct: 10.0,
        acceleration_pct_pts: 10.0,
        recent_3y_avg_growth_pct: 11.0,
        prior_3y_avg_growth_pct: 0.0,
        acceleration_3y_avg_pct_pts: 0.0,
        consecutive_growth_years: 3,
        consistency_score_pct: 100.0,
        annual_rows: vec![DivgAnnualRow {
            year: 2025,
            total_amount: 2.4,
            payment_count: 4,
            growth_pct: 20.0,
        }],
        divacc_label: "ACCELERATING".into(),
        note: String::new(),
    };
    upsert_divacc(&conn, "TEST", &snap).unwrap();
    let got = get_divacc(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.divacc_label, "ACCELERATING");
    assert_eq!(got.latest_year, 2025);
    assert!((got.acceleration_pct_pts - 10.0).abs() < 1e-9);
}

#[test]
fn compute_divacc_accelerating() {
    let rows = vec![
        DividendRecord {
            ex_date: "2021-03-15".into(),
            amount: 1.00,
            ..Default::default()
        },
        DividendRecord {
            ex_date: "2022-03-15".into(),
            amount: 1.10,
            ..Default::default()
        },
        DividendRecord {
            ex_date: "2023-03-15".into(),
            amount: 1.35,
            ..Default::default()
        },
        DividendRecord {
            ex_date: "2024-03-15".into(),
            amount: 1.90,
            ..Default::default()
        },
    ];
    let snap = compute_divacc_snapshot("KO", "2026-04-21", &rows);
    assert_eq!(snap.divacc_label, "ACCELERATING");
    assert_eq!(snap.years_covered, 4);
    assert!(snap.acceleration_pct_pts > 0.0);
    assert!(snap.latest_yoy_growth_pct > snap.prior_yoy_growth_pct);
}

#[test]
fn epsacc_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = EpsAccelerationSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-21".into(),
        quarters_used: 8,
        latest_period: "2026-03-31".into(),
        latest_eps: 1.42,
        prior_year_eps: 0.88,
        latest_yoy_growth_pct: 61.4,
        prior_yoy_growth_pct: -12.0,
        acceleration_pct_pts: 73.4,
        recent_2q_avg_yoy_growth_pct: 42.0,
        prior_2q_avg_yoy_growth_pct: 8.0,
        positive_yoy_quarters: 3,
        epsacc_label: "TURNAROUND".into(),
        note: String::new(),
    };
    upsert_epsacc(&conn, "TEST", &snap).unwrap();
    let got = get_epsacc(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.epsacc_label, "TURNAROUND");
    assert_eq!(got.latest_period, "2026-03-31");
    assert!((got.latest_eps - 1.42).abs() < 1e-9);
}

#[test]
fn compute_epsacc_turnaround() {
    let statements = FinancialStatements {
        income_quarterly: vec![
            IncomeStatement {
                date: "2026-03-31".into(),
                eps: 1.20,
                ..Default::default()
            },
            IncomeStatement {
                date: "2025-12-31".into(),
                eps: 0.50,
                ..Default::default()
            },
            IncomeStatement {
                date: "2025-09-30".into(),
                eps: 0.40,
                ..Default::default()
            },
            IncomeStatement {
                date: "2025-06-30".into(),
                eps: 0.35,
                ..Default::default()
            },
            IncomeStatement {
                date: "2025-03-31".into(),
                eps: 0.60,
                ..Default::default()
            },
            IncomeStatement {
                date: "2024-12-31".into(),
                eps: 0.80,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let snap = compute_epsacc_snapshot("NVDA", "2026-04-21", &statements);
    assert_eq!(snap.epsacc_label, "TURNAROUND");
    assert!(snap.latest_yoy_growth_pct > 0.0);
    assert!(snap.prior_yoy_growth_pct < 0.0);
    assert!(snap.acceleration_pct_pts > 0.0);
}

#[test]
fn vrp_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = VolRiskPremiumSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-21".into(),
        current_atm_iv_pct: 44.0,
        iv_rank: 82.0,
        iv_percentile: 79.0,
        iv_observation_count: 52,
        rv20_pct: 27.0,
        rv60_pct: 24.0,
        rv252_pct: 22.0,
        rv20_percentile: 36.0,
        rv_cone_label: "BELOW_AVG".into(),
        iv_minus_rv20_pct: 17.0,
        iv_to_rv20_ratio: 1.63,
        iv_minus_rv252_pct: 22.0,
        iv_to_rv252_ratio: 2.0,
        premium_label: "EXTREME_RICH".into(),
        note: String::new(),
    };
    upsert_vrp(&conn, "TEST", &snap).unwrap();
    let got = get_vrp(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.premium_label, "EXTREME_RICH");
    assert_eq!(got.rv_cone_label, "BELOW_AVG");
    assert!((got.iv_to_rv20_ratio - 1.63).abs() < 1e-9);
}

#[test]
fn compute_vrp_extreme_rich() {
    let iv = IvolSnapshot {
        symbol: "TSLA".into(),
        as_of: "2026-04-21".into(),
        current_atm_iv_pct: 48.0,
        iv_rank: 85.0,
        iv_percentile: 88.0,
        observation_count: 48,
        ..Default::default()
    };
    let rv = RealizedVolConeSnapshot {
        symbol: "TSLA".into(),
        as_of: "2026-04-21".into(),
        rv20_pct: 28.0,
        rv60_pct: 25.0,
        rv252_pct: 23.0,
        rv20_percentile: 35.0,
        cone_label: "BELOW_AVG".into(),
        ..Default::default()
    };
    let snap = compute_vrp_snapshot("TSLA", "2026-04-21", Some(&iv), Some(&rv));
    assert_eq!(snap.premium_label, "EXTREME_RICH");
    assert!(snap.iv_to_rv20_ratio > 1.5);
    assert!(snap.iv_minus_rv20_pct > 15.0);
}

#[test]
fn short_interest_history_upsert_dedupes_repeated_values() {
    let conn = Connection::open_in_memory().unwrap();
    upsert_short_interest_history(
        &conn,
        "TEST",
        &[ShortInterestHistoryPoint {
            as_of: "2026-01-15".into(),
            short_percent_of_float: 12.0,
            short_ratio: 3.2,
            shares_outstanding: 100_000_000.0,
        }],
    )
    .unwrap();
    upsert_short_interest_history(
        &conn,
        "TEST",
        &[
            ShortInterestHistoryPoint {
                as_of: "2026-01-29".into(),
                short_percent_of_float: 12.0,
                short_ratio: 3.2,
                shares_outstanding: 100_000_000.0,
            },
            ShortInterestHistoryPoint {
                as_of: "2026-02-12".into(),
                short_percent_of_float: 10.5,
                short_ratio: 2.9,
                shares_outstanding: 100_000_000.0,
            },
        ],
    )
    .unwrap();
    let got = get_short_interest_history(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].as_of, "2026-01-15");
    assert_eq!(got[1].as_of, "2026-02-12");
    assert!((got[1].short_percent_of_float - 10.5).abs() < 1e-9);
}

#[test]
fn short_interest_history_parser_handles_finnhub_style_rows() {
    let rows = vec![
        serde_json::json!({
            "date": "2026-02-28",
            "shortPercentOfFloat": 7.25,
            "shortRatio": 2.4,
            "sharesOutstanding": 123456789.0
        }),
        serde_json::json!({
            "settlementDate": "2026-03-15T00:00:00Z",
            "shortInterest": 9000000.0,
            "shareFloat": 100000000.0,
            "daysToCover": "2.8"
        }),
    ];
    let parsed = short_interest_history_points_from_json_rows(&rows);
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].as_of, "2026-02-28");
    assert!((parsed[0].short_percent_of_float - 7.25).abs() < 1e-9);
    assert_eq!(parsed[1].as_of, "2026-03-15");
    assert!((parsed[1].short_percent_of_float - 9.0).abs() < 1e-9);
    assert!((parsed[1].short_ratio - 2.8).abs() < 1e-9);
}

#[test]
fn shortrank_delta_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = ShortInterestDeltaRankSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-21".into(),
        sector: "Technology".into(),
        lookback_days: 180,
        history_points_used: 4,
        history_start_date: "2026-01-02".into(),
        history_end_date: "2026-04-18".into(),
        latest_short_pct_of_float: 6.5,
        prior_short_pct_of_float: 10.5,
        delta_short_pct_points: -4.0,
        latest_short_ratio: 2.3,
        prior_short_ratio: 3.9,
        subject_trend_label: "COVERING".into(),
        peers_considered: 6,
        peers_with_data: 4,
        sector_median_delta_pct_pts: 1.0,
        sector_p25_delta_pct_pts: -0.5,
        sector_p75_delta_pct_pts: 2.0,
        percentile_rank: 90.0,
        rank_position: 1,
        rank_label: "SAFEST_DECILE".into(),
        note: String::new(),
    };
    upsert_shortrank_delta(&conn, "TEST", &snap).unwrap();
    let got = get_shortrank_delta(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.rank_label, "SAFEST_DECILE");
    assert_eq!(got.history_start_date, "2026-01-02");
    assert!((got.delta_short_pct_points + 4.0).abs() < 1e-9);
}

#[test]
fn compute_shortrank_delta_safest_decile() {
    let subject = vec![
        ShortInterestHistoryPoint {
            as_of: "2026-01-10".into(),
            short_percent_of_float: 12.0,
            short_ratio: 4.1,
            shares_outstanding: 1_000_000.0,
        },
        ShortInterestHistoryPoint {
            as_of: "2026-04-18".into(),
            short_percent_of_float: 7.0,
            short_ratio: 2.5,
            shares_outstanding: 1_000_000.0,
        },
    ];
    let peers = vec![
        (
            "BBB".into(),
            vec![
                ShortInterestHistoryPoint {
                    as_of: "2026-01-10".into(),
                    short_percent_of_float: 5.0,
                    short_ratio: 1.9,
                    shares_outstanding: 1_000_000.0,
                },
                ShortInterestHistoryPoint {
                    as_of: "2026-04-18".into(),
                    short_percent_of_float: 6.0,
                    short_ratio: 2.0,
                    shares_outstanding: 1_000_000.0,
                },
            ],
        ),
        (
            "CCC".into(),
            vec![
                ShortInterestHistoryPoint {
                    as_of: "2026-01-10".into(),
                    short_percent_of_float: 8.0,
                    short_ratio: 2.6,
                    shares_outstanding: 1_000_000.0,
                },
                ShortInterestHistoryPoint {
                    as_of: "2026-04-18".into(),
                    short_percent_of_float: 10.0,
                    short_ratio: 3.1,
                    shares_outstanding: 1_000_000.0,
                },
            ],
        ),
        (
            "DDD".into(),
            vec![
                ShortInterestHistoryPoint {
                    as_of: "2026-01-10".into(),
                    short_percent_of_float: 6.0,
                    short_ratio: 2.0,
                    shares_outstanding: 1_000_000.0,
                },
                ShortInterestHistoryPoint {
                    as_of: "2026-04-18".into(),
                    short_percent_of_float: 6.5,
                    short_ratio: 2.2,
                    shares_outstanding: 1_000_000.0,
                },
            ],
        ),
        (
            "EEE".into(),
            vec![
                ShortInterestHistoryPoint {
                    as_of: "2026-01-10".into(),
                    short_percent_of_float: 9.0,
                    short_ratio: 3.0,
                    shares_outstanding: 1_000_000.0,
                },
                ShortInterestHistoryPoint {
                    as_of: "2026-04-18".into(),
                    short_percent_of_float: 8.0,
                    short_ratio: 2.8,
                    shares_outstanding: 1_000_000.0,
                },
            ],
        ),
    ];
    let snap =
        compute_shortrank_delta_snapshot("AAA", "2026-04-21", "Technology", &subject, &peers);
    assert_eq!(snap.rank_label, "SAFEST_DECILE");
    assert_eq!(snap.rank_position, 1);
    assert_eq!(snap.subject_trend_label, "HEAVY_COVERING");
    assert!(snap.delta_short_pct_points < snap.sector_p25_delta_pct_pts);
}

#[test]
fn insiderconc_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    let snap = InsiderConcentrationSnapshot {
        symbol: "TEST".into(),
        as_of: "2026-04-22".into(),
        sector: "Technology".into(),
        latest_holdings_date: "2026-04-18".into(),
        trade_rows_used: 6,
        reporters_covered: 3,
        reporters_holding_shares: 2,
        shares_outstanding: 100_000_000.0,
        total_estimated_insider_shares: 12_500_000.0,
        estimated_insider_pct_held: 12.5,
        largest_reporter: "Alice CEO".into(),
        largest_reporter_shares: 8_000_000.0,
        largest_reporter_pct_of_outstanding: 8.0,
        largest_reporter_weight_pct: 64.0,
        peers_considered: 7,
        peers_with_data: 4,
        sector_median_pct_held: 6.5,
        sector_p25_pct_held: 4.0,
        sector_p75_pct_held: 9.5,
        percentile_rank: 92.0,
        rank_position: 1,
        rank_label: "TOP_DECILE".into(),
        note: String::new(),
    };
    upsert_insiderconc(&conn, "TEST", &snap).unwrap();
    let got = get_insiderconc(&conn, "TEST").unwrap().unwrap();
    assert_eq!(got.rank_label, "TOP_DECILE");
    assert_eq!(got.largest_reporter, "Alice CEO");
    assert!((got.estimated_insider_pct_held - 12.5).abs() < 1e-9);
}

#[test]
fn compute_insiderconc_uses_latest_holding_per_reporter() {
    let subject = vec![
        InsiderTrade {
            transaction_date: "2026-01-10".into(),
            filing_date: "2026-01-11".into(),
            reporting_name: "Alice CEO".into(),
            shares_owned_after: 1_000_000.0,
            ..Default::default()
        },
        InsiderTrade {
            transaction_date: "2026-04-10".into(),
            filing_date: "2026-04-11".into(),
            reporting_name: "Alice CEO".into(),
            shares_owned_after: 3_000_000.0,
            ..Default::default()
        },
        InsiderTrade {
            transaction_date: "2026-04-12".into(),
            filing_date: "2026-04-13".into(),
            reporting_name: "Bob CFO".into(),
            shares_owned_after: 1_500_000.0,
            ..Default::default()
        },
        InsiderTrade {
            transaction_date: "2026-04-12".into(),
            filing_date: "2026-04-13".into(),
            reporting_name: "Carol Director".into(),
            shares_owned_after: 0.0,
            ..Default::default()
        },
    ];
    let peers = vec![
        (
            "BBB".into(),
            Some(100_000_000.0),
            vec![InsiderTrade {
                transaction_date: "2026-04-10".into(),
                filing_date: "2026-04-11".into(),
                reporting_name: "Peer One".into(),
                shares_owned_after: 2_000_000.0,
                ..Default::default()
            }],
        ),
        (
            "CCC".into(),
            Some(100_000_000.0),
            vec![InsiderTrade {
                transaction_date: "2026-04-10".into(),
                filing_date: "2026-04-11".into(),
                reporting_name: "Peer Two".into(),
                shares_owned_after: 5_000_000.0,
                ..Default::default()
            }],
        ),
        (
            "DDD".into(),
            Some(100_000_000.0),
            vec![InsiderTrade {
                transaction_date: "2026-04-10".into(),
                filing_date: "2026-04-11".into(),
                reporting_name: "Peer Three".into(),
                shares_owned_after: 7_000_000.0,
                ..Default::default()
            }],
        ),
        (
            "EEE".into(),
            Some(100_000_000.0),
            vec![InsiderTrade {
                transaction_date: "2026-04-10".into(),
                filing_date: "2026-04-11".into(),
                reporting_name: "Peer Four".into(),
                shares_owned_after: 9_000_000.0,
                ..Default::default()
            }],
        ),
    ];
    let snap = compute_insiderconc_snapshot(
        "AAA",
        "2026-04-22",
        "Technology",
        Some(20_000_000.0),
        &subject,
        &peers,
    );
    assert_eq!(snap.rank_label, "TOP_DECILE");
    assert_eq!(snap.rank_position, 1);
    assert_eq!(snap.reporters_covered, 3);
    assert_eq!(snap.reporters_holding_shares, 2);
    assert_eq!(snap.latest_holdings_date, "2026-04-12");
    assert!((snap.total_estimated_insider_shares - 4_500_000.0).abs() < 1e-9);
    assert!((snap.estimated_insider_pct_held - 22.5).abs() < 1e-9);
    assert_eq!(snap.largest_reporter, "Alice CEO");
    assert!((snap.largest_reporter_pct_of_outstanding - 15.0).abs() < 1e-9);
}

#[test]
fn compute_insiderconc_no_data_without_subject_rows() {
    let peers = vec![
        (
            "BBB".into(),
            Some(100_000_000.0),
            vec![InsiderTrade {
                transaction_date: "2026-04-10".into(),
                filing_date: "2026-04-11".into(),
                reporting_name: "Peer One".into(),
                shares_owned_after: 2_000_000.0,
                ..Default::default()
            }],
        ),
        (
            "CCC".into(),
            Some(100_000_000.0),
            vec![InsiderTrade {
                transaction_date: "2026-04-10".into(),
                filing_date: "2026-04-11".into(),
                reporting_name: "Peer Two".into(),
                shares_owned_after: 3_000_000.0,
                ..Default::default()
            }],
        ),
        (
            "DDD".into(),
            Some(100_000_000.0),
            vec![InsiderTrade {
                transaction_date: "2026-04-10".into(),
                filing_date: "2026-04-11".into(),
                reporting_name: "Peer Three".into(),
                shares_owned_after: 4_000_000.0,
                ..Default::default()
            }],
        ),
    ];
    let snap = compute_insiderconc_snapshot(
        "AAA",
        "2026-04-22",
        "Technology",
        Some(50_000_000.0),
        &[],
        &peers,
    );
    assert_eq!(snap.rank_label, "NO_DATA");
    assert!(snap.note.contains("shares_owned_after"));
}
